//! An interpretation as a viewer shows it: one SVG per drawing level, the diagnostics, the
//! source files, and the links between them. The same runtime and renderer as `rnl render`.
//!
//! The links are objects (the host instances and nets of every level) and the places that show
//! them: where the netlist names an instance or one of its nodes, where a record refers to one,
//! and what on each drawing stands for one (a symbol's box, a pin, a wire).

use std::collections::HashMap;

use rnl_runtime::diag::{Diag, Sink};
use rnl_runtime::geom::{Orient, Pt};
use rnl_runtime::host::{file_name, HostDocument, LevelKind};
use rnl_runtime::model::{LevelView, RefTarget};
use rnl_runtime::source::dirname;
use rnl_runtime::Interpretation;
use serde::Serialize;

/// What a viewer shows of a document, as [`crate::build`] gives it.
#[derive(Serialize, Debug)]
pub struct Document {
    /// The entry as the frontend named it, for building again.
    pub path: String,
    /// Its file name, for the title bar.
    pub name: String,
    /// "spice" or "spectre".
    pub dialect: String,
    pub levels: Vec<Level>,
    pub diagnostics: Vec<Diagnostic>,
    /// Why the document could not be interpreted at all, such as an unsupported language version.
    pub failed: Option<String>,
    /// The host files: the opened file first, then the files it includes.
    pub files: Vec<SourceFile>,
    /// The instances and nets of every level.
    pub objects: Vec<Object>,
    /// The places in the files that name an object.
    pub spans: Vec<Span>,
    /// The files the core asked for by path and did not get, such as an include or a package
    /// file named by `path=` that is not in the workspace: each once, as the provider was asked
    /// for it, in the order first asked. A frontend that can read them adds them to the
    /// workspace and builds again. The bundled packages are never missing, and neither is a
    /// package found by name: that comes only from the provider's package directories (16.2).
    pub missing: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct Level {
    /// The host level drawn, as `Object::level` names it.
    pub host: usize,
    pub title: String,
    /// "file" or "subckt".
    pub kind: &'static str,
    pub svg: String,
    /// Nothing on the level can be drawn: no object has a position yet.
    pub empty: bool,
    /// What the level lacks without a solver; see [`crate::level_status`].
    pub status: Option<String>,
    /// What can be clicked on the drawing, in SVG coordinates.
    pub hits: Vec<Hit>,
    /// The subcircuit instances of the level, in netlist order: the hierarchy below it.
    pub children: Vec<Child>,
}

/// A subcircuit instance, as a branch of the hierarchy.
#[derive(Serialize, Debug)]
pub struct Child {
    pub instance: String,
    /// The subcircuit it instantiates, as written.
    pub master: String,
    /// The level that draws the subcircuit, an index into `Document::levels`; None when the
    /// netlist does not define it.
    pub level: Option<usize>,
}

/// Something on a drawing that stands for objects.
#[derive(Serialize, Debug)]
pub struct Hit {
    /// "box": a symbol, given by its top-left and bottom-right corners; "pin": a pin, one point;
    /// "wire": a wire, its vertices.
    pub kind: &'static str,
    pub points: Vec<[f64; 2]>,
    /// The objects it stands for: an instance (several for a bundle), or nets.
    pub objects: Vec<usize>,
    /// The `data-id` of the SVG groups that draw it.
    pub id: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct SourceFile {
    /// The file as the runtime knows it: relative to the opened file's directory when it is in it
    /// or below, otherwise as the provider names it. With a rooted or absolute entry, the entry's
    /// directory joined with this path (`rnl_runtime::source::join`) is the file in the
    /// workspace. With a relative entry in a directory, a file outside that directory keeps a
    /// relative name, which can be that of a file inside it; the Document then holds only one of
    /// the two.
    pub path: String,
    /// The file as shown: relative to the opened file's directory when it is inside it.
    pub name: String,
    pub text: String,
    /// Where each record is: first line, first column, last line, last column (1-based, inclusive).
    pub records: Vec<[u32; 4]>,
    /// The statements, or parts of them, that do not take effect: first line, first column, last
    /// line, last column, and 0 when they failed or 1 when a conflict overrode them (7.5).
    pub failed: Vec<[u32; 5]>,
}

#[derive(Serialize, Debug)]
pub struct Object {
    /// "instance" or "net".
    pub kind: &'static str,
    pub name: String,
    /// The host level it belongs to, and that level's title.
    pub level: usize,
    pub level_title: String,
    /// An instance's statement: file index, first and last line.
    pub lines: Option<[u32; 3]>,
    /// For a subcircuit instance, the level that draws its subcircuit, an index into
    /// `Document::levels`.
    pub opens: Option<usize>,
}

#[derive(Serialize, Debug)]
pub struct Span {
    pub object: usize,
    pub file: usize,
    pub line: u32,
    /// 1-based, in characters.
    pub col: u32,
    pub len: u32,
    /// "name": an instance's name in its statement; "node": a net in an instance's statement;
    /// "port": a net in a subcircuit's header; "ref": a reference in a record.
    pub kind: &'static str,
}

#[derive(Serialize, Debug)]
pub struct Diagnostic {
    /// "error", "warning" or "hint".
    pub severity: &'static str,
    /// The file as shown: relative to the opened file's directory when it is inside it.
    pub file: String,
    /// The file as the runtime knows it.
    pub full_path: String,
    /// The index of the file in `Document::files`, when it is one of them.
    pub file_index: Option<usize>,
    pub line: u32,
    /// 0 when the diagnostic is about the whole line.
    pub col: u32,
    pub message: String,
    /// Expansion, rule or package history, such as "applying $GND_STUB (a.sp:12:3)".
    pub origin: String,
    /// The upstream cause, as "file:line:col: message".
    pub cause: Option<String>,
}

/// The Document of `entry` from its interpretation: draw every level and link the objects to
/// their places. `missing` is [`Document::missing`].
pub(crate) fn document(entry: &str, r: &Interpretation, missing: Vec<String>) -> Document {
    let dir = dirname(entry);
    let shown_file = |file: &str| match file.strip_prefix(&dir).and_then(|f| f.strip_prefix('/')) {
        Some(rel) if !dir.is_empty() => rel.to_string(),
        _ => file.to_string(),
    };

    let mut files = source_files(&r.doc, &shown_file);
    for f in &r.failed {
        if let Some(file) = files.iter_mut().find(|x| x.path == f.start.file) {
            file.failed.push([f.start.line, f.start.col, f.end.line, f.end.col, u32::from(f.conflict)]);
        }
    }
    let file_index: HashMap<String, usize> = files.iter().enumerate().map(|(k, f)| (f.path.clone(), k)).collect();
    let mut ix = Index::new(&r.doc, &file_index);
    ix.netlist(&files);
    for h in &r.refs {
        let Some(&file) = file_index.get(&h.loc.file) else { continue };
        let object = match &h.target {
            RefTarget::Instance(i) => match ix.insts.get(&(h.level, *i)) {
                Some(&o) => o,
                None => continue,
            },
            RefTarget::Net(key) => ix.net_by_key(h.level, key, key),
        };
        ix.spans.push(Span { object, file, line: h.loc.line, col: h.loc.col, len: h.len, kind: "ref" });
    }
    let mut levels: Vec<Level> = r.model.levels.iter().map(|lv| ix.level(lv)).collect();
    let drawn: HashMap<usize, usize> = r.model.levels.iter().enumerate().map(|(k, lv)| (lv.level, k)).collect();
    for (k, lv) in r.model.levels.iter().enumerate() {
        levels[k].children = r.doc.levels[lv.level]
            .instances
            .iter()
            .filter(|i| i.is_subckt)
            .map(|i| {
                let master = i.model.clone().unwrap_or_default();
                let level = r.doc.subckt(&master).and_then(|h| drawn.get(&h).copied());
                Child { instance: i.name.clone(), master, level }
            })
            .collect();
    }
    for (&(lv, i), &o) in &ix.insts {
        let inst = &r.doc.levels[lv].instances[i];
        if inst.is_subckt {
            let master = inst.model.as_deref().unwrap_or_default();
            ix.objects[o].opens = r.doc.subckt(master).and_then(|h| drawn.get(&h).copied());
        }
    }

    let diagnostic = |d: &Diag, sink: &Sink| {
        let (file, line, col) = match &d.loc {
            Some(l) => (l.file.clone(), l.line, l.col),
            None => (String::new(), 0, 0),
        };
        let cause = d.cause.map(|c| {
            let c = sink.get(c);
            match &c.loc {
                Some(l) => format!("{}:{}{}: {}", shown_file(&l.file), l.line, col_suffix(l.col), c.message),
                None => c.message.clone(),
            }
        });
        Diagnostic {
            severity: d.severity.as_str(),
            file: shown_file(&file),
            file_index: file_index.get(&file).copied(),
            full_path: file,
            line,
            col,
            message: d.message.clone(),
            origin: d.origin.clone(),
            cause,
        }
    };
    let diagnostics = r.diagnostics().into_iter().map(|d| diagnostic(d, &r.sink)).collect();
    let (objects, spans) = (ix.objects, ix.spans);
    Document {
        path: entry.to_string(),
        name: file_name(entry).to_string(),
        dialect: r.doc.dialect.name.clone(),
        levels,
        diagnostics,
        failed: r.model.failed.clone(),
        files,
        objects,
        spans,
        missing,
    }
}

/// The host files with the places of their records; the opened file comes first.
fn source_files(doc: &HostDocument, shown: &dyn Fn(&str) -> String) -> Vec<SourceFile> {
    let mut files: Vec<SourceFile> = doc
        .sources
        .iter()
        .map(|(p, text)| SourceFile {
            path: p.clone(),
            name: shown(p),
            text: text.clone(),
            records: Vec::new(),
            failed: Vec::new(),
        })
        .collect();
    let main = doc.main().file.clone();
    if let Some(k) = files.iter().position(|f| f.path == main) {
        let f = files.remove(k);
        files.insert(0, f);
    }
    for lv in &doc.levels {
        for rec in &lv.records {
            let Some(f) = files.iter_mut().find(|f| f.path == rec.file) else { continue };
            let n = rec.text.chars().count();
            let (a, b) = (rec.loc(0), rec.loc(n.saturating_sub(1)));
            f.records.push([a.line, a.col, b.line, b.col]);
        }
    }
    for f in &mut files {
        f.records.sort();
        f.records.dedup();
    }
    files
}

struct Index<'a> {
    doc: &'a HostDocument,
    files: &'a HashMap<String, usize>,
    objects: Vec<Object>,
    spans: Vec<Span>,
    insts: HashMap<(usize, usize), usize>,
    nets: HashMap<(usize, String), usize>,
}

impl<'a> Index<'a> {
    fn new(doc: &'a HostDocument, files: &'a HashMap<String, usize>) -> Index<'a> {
        Index { doc, files, objects: Vec::new(), spans: Vec::new(), insts: HashMap::new(), nets: HashMap::new() }
    }

    /// The net of a level by the name a statement writes.
    fn net(&mut self, level: usize, written: &str) -> usize {
        let key = self.doc.net_key(written);
        self.net_by_key(level, &key, written)
    }

    fn net_by_key(&mut self, level: usize, key: &str, written: &str) -> usize {
        if let Some(&o) = self.nets.get(&(level, key.to_string())) {
            return o;
        }
        let name = if key == "0" { "0".to_string() } else { written.to_string() };
        self.objects.push(Object {
            kind: "net",
            name,
            level,
            level_title: self.doc.levels[level].title(),
            lines: None,
            opens: None,
        });
        self.nets.insert((level, key.to_string()), self.objects.len() - 1);
        self.objects.len() - 1
    }

    /// Every instance of every level, and the places in the netlist that name instances and nets.
    fn netlist(&mut self, files: &[SourceFile]) {
        let lines: Vec<Vec<&str>> = files.iter().map(|f| f.text.lines().collect()).collect();
        for (lv, level) in self.doc.levels.iter().enumerate() {
            for (i, inst) in level.instances.iter().enumerate() {
                let file = self.files.get(&inst.file).copied();
                let statement = file.filter(|_| inst.line > 0).and_then(|f| {
                    let (last, tokens) = statement(&lines[f], inst.line as usize - 1)?;
                    Some((f, last, tokens))
                });
                self.objects.push(Object {
                    kind: "instance",
                    name: inst.name.clone(),
                    level: lv,
                    level_title: level.title(),
                    lines: statement.as_ref().map(|(f, last, _)| [*f as u32, inst.line, *last as u32 + 1]),
                    opens: None,
                });
                let object = self.objects.len() - 1;
                self.insts.insert((lv, i), object);
                let Some((f, _, tokens)) = statement else { continue };
                if let Some(t) = tokens.first() {
                    self.spans.push(Span { object, file: f, line: t.line, col: t.col, len: t.len, kind: "name" });
                }
                self.nodes(lv, f, &tokens, 1, &inst.nodes, "node");
            }
            if level.kind == LevelKind::Subckt && level.line > 0 {
                if let Some(&f) = self.files.get(&level.file) {
                    if let Some((_, tokens)) = statement(&lines[f], level.line as usize - 1) {
                        // `.subckt NAME ports…`, `subckt NAME (ports…)` or `inline subckt NAME …`
                        let skip = if tokens.first().is_some_and(|t| t.text == "inline") { 3 } else { 2 };
                        self.nodes(lv, f, &tokens, skip, &level.ports, "port");
                    }
                }
            }
        }
    }

    /// The tokens from `from` on that name `nodes`, in order.
    fn nodes(&mut self, lv: usize, file: usize, tokens: &[Token], from: usize, nodes: &[String], kind: &'static str) {
        let mut j = from;
        for node in nodes {
            let key = self.doc.key(node);
            while j < tokens.len() && self.doc.key(&tokens[j].text) != key {
                j += 1;
            }
            let Some(t) = tokens.get(j) else { break };
            let object = self.net(lv, node);
            self.spans.push(Span { object, file, line: t.line, col: t.col, len: t.len, kind });
            j += 1;
        }
    }

    /// The drawing of a level and what on it can be clicked.
    fn level(&mut self, lv: &LevelView) -> Level {
        // the model names items and wires for people ("a wire"); the page needs each SVG group to
        // say which one it draws
        let mut lv = lv.clone();
        for (k, it) in lv.items.iter_mut().enumerate() {
            it.id = format!("item-{k}");
        }
        for (k, w) in lv.wires.iter_mut().enumerate() {
            w.id = format!("wire-{k}");
        }
        let lv = &lv;
        let prims = rnl_render::primitives(lv);
        let options = rnl_render::RenderOptions::default();
        let px = |p: Pt| [p.0 * options.scale, -p.1 * options.scale];
        let mut hits = Vec::new();
        for it in &lv.items {
            let sym = &lv.symbols[it.symbol];
            let [x0, y0, x1, y1] = sym.bbox;
            let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|c| place(it.at, it.orient, c));
            let min = |f: fn(&Pt) -> f64| corners.iter().map(f).fold(f64::MAX, f64::min);
            let max = |f: fn(&Pt) -> f64| corners.iter().map(f).fold(f64::MIN, f64::max);
            let (left, right, bottom, top) = (min(|p| p.0), max(|p| p.0), min(|p| p.1), max(|p| p.1));
            let mut objects: Vec<usize> =
                it.host.iter().filter_map(|i| self.insts.get(&(lv.level, *i)).copied()).collect();
            if objects.is_empty() {
                // a usage, such as a ground symbol, stands for the net it is on
                let mut nets: Vec<usize> = it.pin_nets.iter().flatten().map(|n| self.net(lv.level, n)).collect();
                nets.dedup();
                if nets.len() == 1 {
                    objects = nets;
                }
            }
            hits.push(Hit {
                kind: "box",
                points: vec![px((left, top)), px((right, bottom))],
                objects,
                id: Some(it.id.clone()),
            });
            for (pin, nets) in sym.pins.iter().zip(&it.pin_nets) {
                if nets.is_empty() {
                    continue;
                }
                let objects = nets.iter().map(|n| self.net(lv.level, n)).collect();
                hits.push(Hit { kind: "pin", points: vec![px(place(it.at, it.orient, pin.pos))], objects, id: None });
            }
        }
        for w in &lv.wires {
            let objects = w.nets.iter().map(|n| self.net(lv.level, n)).collect();
            hits.push(Hit {
                kind: "wire",
                points: w.points.iter().map(|p| px(*p)).collect(),
                objects,
                id: Some(w.id.clone()),
            });
        }
        Level {
            host: lv.level,
            title: lv.title.clone(),
            kind: lv.kind,
            svg: rnl_render::svg_document(&prims, &options),
            empty: prims.is_empty(),
            status: crate::level_status(lv),
            hits,
            children: Vec::new(),
        }
    }
}

fn place(at: Pt, orient: Orient, p: Pt) -> Pt {
    let d = orient.apply(p);
    (at.0 + d.0, at.1 + d.1)
}

/// A word of a host statement and where it is.
#[derive(Debug)]
struct Token {
    text: String,
    line: u32,
    col: u32,
    len: u32,
}

/// The statement starting on line `first` (0-based): its last line, with the SPICE `+` and the
/// Spectre `\` continuations, and its words without comments. Parentheses and commas separate
/// words, so Spectre's `(d g s b)` gives the four nodes.
fn statement(lines: &[&str], first: usize) -> Option<(usize, Vec<Token>)> {
    lines.get(first)?;
    let mut last = first;
    while last + 1 < lines.len() {
        let joined = code(lines[last]).trim_end().ends_with('\\');
        let plus = lines[last + 1].trim_start().starts_with('+');
        if !(joined || plus) {
            break;
        }
        last += 1;
    }
    let mut tokens = Vec::new();
    for (n, raw) in lines.iter().enumerate().take(last + 1).skip(first) {
        let chars: Vec<char> = code(raw).chars().collect();
        let mut k = 0;
        if n > first {
            // the `+` of a continuation line
            while k < chars.len() && chars[k].is_whitespace() {
                k += 1;
            }
            if chars.get(k) == Some(&'+') {
                k += 1;
            }
        }
        while k < chars.len() {
            if chars[k].is_whitespace() || "(),\\".contains(chars[k]) {
                k += 1;
                continue;
            }
            let start = k;
            while k < chars.len() && !chars[k].is_whitespace() && !"(),\\".contains(chars[k]) {
                k += 1;
            }
            tokens.push(Token {
                text: chars[start..k].iter().collect(),
                line: n as u32 + 1,
                col: start as u32 + 1,
                len: (k - start) as u32,
            });
        }
    }
    Some((last, tokens))
}

/// A host line without its inline comment: SPICE `;` and ` $`, Spectre `//`.
fn code(line: &str) -> &str {
    let mut prev = ' ';
    for (i, c) in line.char_indices() {
        if c == ';' || (c == '$' && prev.is_whitespace()) || (c == '/' && line[i + 1..].starts_with('/')) {
            return &line[..i];
        }
        prev = c;
    }
    line
}

fn col_suffix(col: u32) -> String {
    if col > 0 {
        format!(":{col}")
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{corpus, Disk};
    use crate::BuildOptions;

    fn load(entry: &str) -> Result<Document, String> {
        crate::build(entry, &Disk, &BuildOptions::default())
    }

    fn case(name: &str) -> String {
        corpus(&format!("cases/{name}"))
    }

    fn equivalence(name: &str) -> String {
        corpus(&format!("equivalence/{name}"))
    }

    /// The text of a span.
    fn text(d: &Document, s: &Span) -> String {
        let line = d.files[s.file].text.lines().nth(s.line as usize - 1).unwrap();
        line.chars().skip(s.col as usize - 1).take(s.len as usize).collect()
    }

    #[test]
    fn draws_every_level_with_its_status() {
        let d = load(&case("04-hierarchy.sp")).unwrap();
        assert_eq!(d.name, "04-hierarchy.sp");
        assert!(d.levels.len() >= 3, "{:?}", d.levels.iter().map(|l| &l.title).collect::<Vec<_>>());
        assert!(d.levels.iter().all(|l| l.svg.starts_with("<svg") && !l.empty));
        let d = load(&case("10-constraints-and-priority.sp")).unwrap();
        assert!(d.levels[0].status.as_deref().is_some_and(|s| s.contains("needs-solver")));
    }

    #[test]
    fn diagnostics_are_relative_to_the_opened_file() {
        let d = load(&case("12-diagnostics.sp")).unwrap();
        assert!(d.diagnostics.iter().any(|x| x.severity == "error"));
        let own: Vec<_> = d.diagnostics.iter().filter(|x| x.full_path.ends_with("12-diagnostics.sp")).collect();
        assert!(!own.is_empty());
        assert!(own.iter().all(|x| x.file == "12-diagnostics.sp" && x.line > 0 && x.file_index == Some(0)));
        // its subcircuit has nothing with a position
        assert!(d
            .levels
            .iter()
            .any(|l| l.empty && l.status.as_deref().is_some_and(|s| s.contains("without a position"))));
    }

    #[test]
    fn a_missing_file_is_an_error() {
        assert!(load(&case("no-such-case.sp")).is_err());
    }

    #[test]
    fn names_nodes_and_references_point_at_their_objects() {
        let d = load(&case("01-rc-lowpass.sp")).unwrap();
        assert_eq!(d.files.len(), 1);
        assert!(!d.files[0].records.is_empty());
        let r1 = d.objects.iter().position(|o| o.kind == "instance" && o.name == "R1").unwrap();
        assert_eq!(d.objects[r1].lines, Some([0, 4, 4]));
        let of = |o: usize| d.spans.iter().filter(move |s| s.object == o);
        let kinds: Vec<(&str, String)> = of(r1).map(|s| (s.kind, text(&d, s))).collect();
        assert!(kinds.contains(&("name", "R1".into())));
        assert!(kinds.contains(&("ref", "#R1".into())));
        let out = d.objects.iter().position(|o| o.kind == "net" && o.name == "out").unwrap();
        let texts: Vec<(&str, String)> = of(out).map(|s| (s.kind, text(&d, s))).collect();
        assert!(texts.contains(&("node", "out".into())), "{texts:?}");
        assert!(texts.contains(&("ref", "&out".into())), "{texts:?}");
        // on the drawing: R1's box, and the wires and pins of `out`
        let hits = &d.levels[0].hits;
        let b = hits.iter().find(|h| h.kind == "box" && h.objects == vec![r1]).unwrap();
        assert!(b.points[0][0] < b.points[1][0] && b.points[0][1] < b.points[1][1]);
        assert!(hits.iter().any(|h| h.kind == "wire" && h.objects.contains(&out)));
        // each wire has its own group, so a net's wires can be marked alone
        let ids: Vec<&str> = hits.iter().filter(|h| h.kind == "wire").filter_map(|h| h.id.as_deref()).collect();
        assert!(ids.iter().all(|id| ids.iter().filter(|x| *x == id).count() == 1), "{ids:?}");
        assert!(ids.iter().all(|id| d.levels[0].svg.contains(&format!("data-id=\"{id}\""))));
        assert!(hits.iter().any(|h| h.kind == "pin" && h.objects.contains(&out)));
        // a ground symbol stands for the ground net
        let gnd = d.objects.iter().position(|o| o.kind == "net" && o.name == "0").unwrap();
        assert!(hits.iter().any(|h| h.kind == "box" && h.objects == vec![gnd]));
    }

    #[test]
    fn each_level_lists_the_subcircuits_below_it() {
        let d = load(&case("04-hierarchy.sp")).unwrap();
        let top: Vec<(&str, &str)> = d.levels[0]
            .children
            .iter()
            .map(|c| (c.instance.as_str(), d.levels[c.level.unwrap()].title.as_str()))
            .collect();
        assert!(top.contains(&("X1", "amp")) && top.iter().any(|(_, l)| *l == "rdiv"), "{top:?}");
        let amp = d.levels.iter().find(|l| l.title == "amp").unwrap();
        assert!(amp.children.is_empty());
        // a subcircuit instance knows the drawing it opens; others open none
        let x1 = d.objects.iter().find(|o| o.kind == "instance" && o.name == "X1").unwrap();
        assert_eq!(x1.opens.map(|l| d.levels[l].title.as_str()), Some("amp"));
        assert!(d.objects.iter().filter(|o| o.name.starts_with('R') || o.kind == "net").all(|o| o.opens.is_none()));
    }

    #[test]
    fn failed_and_overridden_statements_are_given_per_file() {
        let d = load(&case("10-constraints-and-priority.sp")).unwrap();
        assert!(!d.files[0].failed.is_empty());
        assert!(d.files[0].failed.iter().all(|f| f[4] == 1 && f[0] <= f[2]));
        let d = load(&case("12-diagnostics.sp")).unwrap();
        assert!(d.files[0].failed.iter().any(|f| f[4] == 0) && d.files[0].failed.iter().any(|f| f[4] == 1));
        assert!(load(&case("01-rc-lowpass.sp")).unwrap().files[0].failed.is_empty());
    }

    #[test]
    fn spectre_nodes_in_parentheses_and_subcircuit_ports() {
        let d = load(&equivalence("inverter-chain.scs")).unwrap();
        let spans: Vec<(&str, String)> = d.spans.iter().map(|s| (s.kind, text(&d, s))).collect();
        assert!(spans.iter().any(|(k, _)| *k == "port"), "{spans:?}");
        let mp = d.objects.iter().position(|o| o.kind == "instance").unwrap();
        let nodes = d.spans.iter().filter(|s| s.kind == "node" && d.objects[s.object].level == d.objects[mp].level);
        assert!(nodes.count() >= 2);
    }

    #[test]
    fn statements_follow_continuations_and_skip_comments() {
        let lines = ["M1 d g", "+ s b nch ; comment d", "R1 a b"];
        let (last, tokens) = statement(&lines, 0).unwrap();
        assert_eq!(last, 1);
        let words: Vec<&str> = tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(words, ["M1", "d", "g", "s", "b", "nch"]);
        assert_eq!((tokens[3].line, tokens[3].col), (2, 3));
        let lines = ["m1 (d g \\", "  s b) nch // x"];
        let (last, tokens) = statement(&lines, 0).unwrap();
        assert_eq!(last, 1);
        assert_eq!(tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["m1", "d", "g", "s", "b", "nch"]);
    }
}
