//! An interpretation as a viewer shows it: each drawing level as SVG or primitives, the
//! diagnostics, the source files, and the links between them. The same runtime and renderer as
//! `rnl render`.
//!
//! The links are objects (the host instances, nets and drawn usages of every level) and the places that show
//! them: where the netlist names an instance or one of its nodes, where a record refers to one,
//! and what on each drawing stands for one (a symbol's box, a pin, a wire).

use std::collections::{HashMap, HashSet};

use rnl_runtime::diag::{Diag, Sink};
use rnl_runtime::geom::{Orient, Pt};
use rnl_runtime::host::{dialect_key, file_name, HostDocument, LevelKind};
use rnl_runtime::interp::obj::{Lit, WStr};
use rnl_runtime::markup;
use rnl_runtime::model::{ItemView, Layer, LevelView, RefTarget, Style, SymbolView, TextTarget, TextView};
use rnl_runtime::source::{dirname, relative_to};
use rnl_runtime::Interpretation;
use serde::Serialize;

use crate::primitive::{self, Primitive};
use crate::{BuildOptions, FormatInfo};

mod symbols;
pub use symbols::{InspectedLabel, InspectedPin, InspectedSymbol};
mod usages;
pub use usages::{Usage, UsageRole};

/// What a viewer shows of a document, as [`crate::build`] gives it.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Document {
    /// Symbols used by subcircuit instances or defined by subcircuits, once per runtime identity.
    pub symbols: Vec<InspectedSymbol>,
    /// The entry as the frontend named it, for building again.
    pub path: String,
    /// Its file name, for the title bar.
    pub name: String,
    /// The format of the main file, "spice" or "spectre"; or "text" for a file that is not a
    /// netlist, such as a Verilog-A model, which is not read.
    pub dialect: String,
    /// The formats a file can be read with, as `rnl_view::formats` lists them.
    pub formats: Vec<FormatInfo>,
    pub levels: Vec<Level>,
    pub diagnostics: Vec<Diagnostic>,
    /// Why the document could not be interpreted at all, such as an unsupported language version.
    pub failed: Option<String>,
    /// The host files: the opened file first, then the files it includes.
    pub files: Vec<SourceFile>,
    /// The instances, nets and drawn usages of every level.
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

/// A drawing level. Its SVG and its hits are in pixels with y down; its primitives are in scene
/// units with y up, `scale` pixels each.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Level {
    /// Optional inspection geometry in SVG pixels (y down), independent of display styles.
    pub currents: Vec<crate::CurrentArrow>,
    /// All usages in this drawing scope, including semantic usages without a symbol.
    pub usages: Vec<Usage>,
    /// Terminal names in subcircuit declaration order.
    pub ports: Vec<String>,
    /// The subcircuit's own block symbol, an index into `Document::symbols`.
    pub symbol: Option<usize>,
    /// The host level drawn, as `Object::level` names it.
    pub host: usize,
    pub title: String,
    /// "file" or "subckt".
    pub kind: &'static str,
    /// A subcircuit's definition: file index, the first line of its header and the line of its end
    /// (`.ends`, `ends`), or the file's last line when it has none; None for a file level.
    pub lines: Option<[u32; 3]>,
    /// The drawing as an SVG document, written from the primitives; absent unless
    /// `BuildOptions::svg` asks for it, as it does by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub svg: Option<String>,
    /// The drawing as primitives in drawing order (decision 0008); absent unless
    /// `BuildOptions::primitives` asks for them.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub primitives: Option<Vec<Primitive>>,
    /// Pixels per scene unit in the SVG and the hits: a point (x, y) of a primitive, or of the
    /// scene, is at (x·scale, −y·scale) in them.
    pub scale: f64,
    /// Nothing on the level can be drawn: no object has a position yet.
    pub empty: bool,
    /// What the level lacks without a solver; see [`crate::level_status`].
    pub status: Option<String>,
    /// Whether to solve the level (decision 0008): "fixed", nothing for a solver to decide;
    /// "unsolved", something to decide and no record of the solver; "solved", a record of the
    /// solver that is up to date; "stale", one whose input or numbering is out of date. Absent
    /// unless `BuildOptions::status` asks for it; see `rnl_runtime::previous::solve_status`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub solve: Option<&'static str>,
    /// What can be clicked on the drawing, in SVG pixels.
    pub hits: Vec<Hit>,
    /// The subcircuit instances of the level, in netlist order: the hierarchy below it.
    pub children: Vec<Child>,
    /// The objects of the level that are not drawn on it: those that have no position, each with
    /// its symbol drawn alone, in the order of the netlist (their number is the one `status`
    /// gives), then those that are not drawn at all, since no symbol can be chosen for them, each
    /// with its `reason` and no symbol.
    pub unplaced: Vec<Unplaced>,
}

/// An object of a level that has no position yet, so it is not drawn there: an instance, a bundle
/// of instances or a usage such as a ground symbol, left to a solver. A viewer lists them, each
/// with its symbol drawn alone and the nets at its terminals.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Unplaced {
    /// "instance", "bundle" or "usage".
    pub kind: &'static str,
    /// An instance's name; for a bundle or a usage, the label the diagnostics give it, such as
    /// "a usage of $GND".
    pub name: String,
    /// The name of the symbol it is drawn with; empty for an object that has none (see `reason`).
    pub symbol: String,
    /// Why it is not drawn at all, whatever its position, such as `model card not found` for a
    /// transistor whose model the netlist does not define; absent for an object that only lacks a
    /// position, which a solver can place. Such an object has no symbol and no drawing.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub reason: Option<String>,
    /// The instances it stands for, indexes into `Document::objects`: one for an instance, its
    /// members for a bundle, none for a usage. A viewer shows the first one's statement.
    pub objects: Vec<usize>,
    /// The symbol alone at the origin, as it would be drawn on the level with its orientation,
    /// and the name of the net at each of its pins beside the pin, as an SVG document; absent
    /// unless `BuildOptions::svg` asks for it, and absent from a Document built with
    /// [`crate::build_drawn`], whose [`Drawings`] give it on request.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub svg: Option<String>,
    /// The same drawing as primitives in scene units, as `Level::primitives` has them; absent
    /// unless `BuildOptions::primitives` asks for them, and from a Document of `build_drawn`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub primitives: Option<Vec<Primitive>>,
}

/// The drawing of one object of [`Level::unplaced`], as [`Drawings::unplaced`] gives it: what
/// `Unplaced::svg` and `Unplaced::primitives` hold when the Document carries them.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct UnplacedDrawing {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub svg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub primitives: Option<Vec<Primitive>>,
}

/// The drawings of the objects that a Document lists without a position, kept to give on request
/// instead of carried in the Document: a netlist with no positions has every object in the list,
/// and the page shows a few of them at a time. [`crate::build_drawn`] makes them with the Document.
/// They own only the exported model, so a background compiler may transfer them to a frontend.
pub struct Drawings {
    levels: Vec<DrawnLevel>,
    render: rnl_render::RenderOptions,
    svg: bool,
    primitives: bool,
}

mod memory;

/// The unplaced objects of a level: the items to draw, the symbols they are drawn with, and their
/// names as the Document lists them.
struct DrawnLevel {
    items: Vec<ItemView>,
    symbols: Vec<SymbolView>,
    names: Vec<String>,
}

impl Drawings {
    /// The drawings of the levels of `doc`, taken from `levels`, which are left without them.
    pub(crate) fn take(levels: &mut [LevelView], doc: &Document, options: &BuildOptions) -> Drawings {
        let levels = levels
            .iter_mut()
            .zip(&doc.levels)
            .map(|(lv, level)| DrawnLevel {
                items: std::mem::take(&mut lv.unplaced_items),
                symbols: std::mem::take(&mut lv.unplaced_symbols),
                names: level.unplaced.iter().map(|u| u.name.clone()).collect(),
            })
            .collect();
        Drawings {
            levels,
            render: rnl_render::RenderOptions::default(),
            svg: options.svg,
            primitives: options.primitives,
        }
    }

    /// The drawings of the objects at `items`, indexes into `Document::levels[level].unplaced`, in
    /// that order. `names`, when given, are the names the caller has for them, which must be the
    /// Document's: the drawings are of the Document that was built last, and a caller that
    /// asks about another one is told so instead of given the drawings of other objects.
    pub fn unplaced(&self, level: usize, items: &[usize], names: &[String]) -> Result<Vec<UnplacedDrawing>, String> {
        let lv = self.levels.get(level).ok_or_else(|| format!("the document has no level {level}"))?;
        if !names.is_empty() && names.len() != items.len() {
            return Err("the objects and their names are not as many".to_string());
        }
        let mut alone = LevelView { symbols: lv.symbols.clone(), ..LevelView::default() };
        let mut out = Vec::with_capacity(items.len());
        for (k, &i) in items.iter().enumerate() {
            let it = lv.items.get(i).ok_or_else(|| format!("level {level} lists no unplaced object {i}"))?;
            if !names.is_empty() && lv.names[i] != names[k] {
                return Err(format!(
                    "unplaced object {i} of level {level} is {}, not {}: the document was built again",
                    lv.names[i], names[k]
                ));
            }
            out.push(draw_alone(&mut alone, it, &self.render, self.svg, self.primitives));
        }
        Ok(out)
    }
}

/// An unplaced object drawn alone as the outputs ask: `alone` is a level of nothing but the symbols
/// of the objects, with `it` put on it, at the origin, and the names of its nets beside its pins.
fn draw_alone(
    alone: &mut LevelView,
    it: &ItemView,
    render: &rnl_render::RenderOptions,
    svg: bool,
    primitives: bool,
) -> UnplacedDrawing {
    let sym = &alone.symbols[it.symbol];
    let labels = net_labels(it, sym);
    alone.items = vec![ItemView { id: "item-0".into(), at: (0.0, 0.0), ..it.clone() }];
    alone.texts = labels;
    let prims = rnl_render::primitives(alone, render);
    UnplacedDrawing {
        svg: svg.then(|| rnl_render::svg_document(&prims, render)),
        primitives: primitives.then(|| primitive::primitives(&prims)),
    }
}

/// A subcircuit instance, as a branch of the hierarchy.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Child {
    /// The symbol this instance actually uses in its parent, including parent overrides.
    pub symbol: Option<usize>,
    pub instance: String,
    /// The subcircuit it instantiates, as written.
    pub master: String,
    /// The level that draws the subcircuit, an index into `Document::levels`; None when the
    /// netlist does not define it.
    pub level: Option<usize>,
}

/// Something on a drawing that stands for objects.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Hit {
    /// "box": a symbol, given by its top-left and bottom-right corners; "pin": a pin, one point;
    /// "wire": a wire, its vertices; "label": a label of an object, given by its top-left and
    /// bottom-right corners, which stands for what that object stands for: for a terminal, the nets
    /// of its pin, as the pin does; and for nothing when it is not an instance, a bundle, a usage, a
    /// wire, a net or a terminal drawn at a pin with a net.
    pub kind: &'static str,
    /// In SVG pixels, y down.
    pub points: Vec<[f64; 2]>,
    /// The objects it stands for: an instance (several for a bundle), a usage followed by its
    /// associated net, or nets. The first object is what clicking the hit selects. A usage's
    /// label keeps its net first, so clicking a net label continues to select the net.
    pub objects: Vec<usize>,
    /// The `data-id` of the SVG groups that draw it, and the owner of its primitives.
    /// Repeated text ids are disambiguated for the page, so each label has its own groups.
    pub id: Option<String>,
    /// The name a netlist gives a usage, such as a ground or a port, for the box of one that has a
    /// name (`%gnd1`: "gnd1"); a viewer names an instance or a net by `objects`. Absent for the
    /// others, the usages that are not named among them.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub name: Option<String>,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SourceFile {
    /// Explicit include directives and the files the host resolver actually loaded.
    pub includes: Vec<SourceInclude>,
    /// Physical lines containing free text in active, syntactically valid RNL members (1-based).
    pub semantics: Vec<u32>,
    /// The file as the runtime knows it: relative to the opened file's directory when it is in it
    /// or below, otherwise as the provider names it. With a rooted or absolute entry, the entry's
    /// directory joined with this path (`rnl_runtime::source::join`) is the file in the
    /// workspace. With a relative entry in a directory, a file outside that directory keeps a
    /// relative name, which can be that of a file inside it; the Document then holds only one of
    /// the two.
    pub path: String,
    /// The file as shown: relative to the opened file's directory when it is inside it.
    pub name: String,
    /// The format the file was read with, a name of `Document::formats`; none for a package file.
    /// "text" for a file that is not a netlist, such as a Verilog-A model: its text, not read.
    pub format: Option<String>,
    /// The format the file is read with when none is set for it, as a frontend shows its
    /// automatic choice; none for a package file.
    pub default_format: Option<String>,
    pub text: String,
    /// Where each record is: first line, first column, last line, last column (1-based, inclusive).
    pub records: Vec<[u32; 4]>,
    /// The statements, or parts of them, that do not take effect: first line, first column, last
    /// line, last column, and 0 when they failed or 1 when a conflict overrode them (7.5).
    pub failed: Vec<[u32; 5]>,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Object {
    /// "instance", "net" or "usage".
    pub kind: &'static str,
    /// An instance or net's name, or a usage's editor identity (`%gnd1` when named).
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
#[cfg_attr(test, derive(ts_rs::TS))]
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

#[derive(Serialize, Debug, Clone)]
#[cfg_attr(test, derive(ts_rs::TS))]
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

/// The Document of `entry` from its interpretation: draw every level as `options` ask and link the
/// objects to their places. `missing` is [`Document::missing`].
pub(crate) fn document(
    entry: &str,
    r: &Interpretation,
    missing: Vec<String>,
    options: &BuildOptions,
    lazy: bool,
) -> Document {
    let dir = dirname(entry);
    let shown_file = |file: &str| relative_to(file, &dir).unwrap_or(file).to_string();

    // A package checked on its own has no host sources. Still expose the opened package's
    // text, including its record ranges, so the Files tab can open built-ins read-only.
    let package_doc = if r.doc.sources.is_empty() && r.doc.dialect.name == "none" {
        let mut source_doc = r.doc.clone();
        if let Some((name, text)) = r.sources.iter().find(|(name, _)| name == &source_doc.main().file) {
            source_doc.sources.push((name.clone(), text.clone()));
            source_doc.levels[0].records =
                rnl_runtime::records::scan_run(&rnl_runtime::records::package_lines(text), name, &mut Sink::new());
        }
        Some(source_doc)
    } else {
        None
    };
    let mut files = source_files(package_doc.as_ref().unwrap_or(&r.doc), &shown_file);
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
    let mut levels: Vec<Level> = r.model.levels.iter().map(|lv| ix.level(lv, options, lazy)).collect();
    let drawn: HashMap<usize, usize> = r.model.levels.iter().enumerate().map(|(k, lv)| (lv.level, k)).collect();
    for (k, lv) in r.model.levels.iter().enumerate() {
        let host = &r.doc.levels[lv.level];
        if host.kind == LevelKind::Subckt && host.line > 0 {
            if let Some(&f) = file_index.get(&host.file) {
                // a subcircuit with no end has none (0), or the line after its file's last
                let last = (files[f].text.lines().count() as u32).max(host.line);
                let end = if host.end_line >= host.line { host.end_line.min(last) } else { last };
                levels[k].lines = Some([f as u32, host.line, end]);
            }
        }
        levels[k].children = r.doc.levels[lv.level]
            .instances
            .iter()
            .filter(|i| i.is_subckt)
            .map(|i| {
                let master = i.model.clone().unwrap_or_default();
                let level = r.doc.subckt_of(i).and_then(|h| drawn.get(&h).copied());
                Child { instance: i.name.clone(), master, level, symbol: None }
            })
            .collect();
    }
    for (&(lv, i), &o) in &ix.insts {
        let inst = &r.doc.levels[lv].instances[i];
        if inst.is_subckt {
            ix.objects[o].opens = r.doc.subckt_of(inst).and_then(|h| drawn.get(&h).copied());
        }
    }

    usages::inspect_all(r, &mut levels, &mut ix);
    let symbols = symbols::inspect(r, &mut levels);
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
        symbols,
        path: entry.to_string(),
        name: file_name(entry).to_string(),
        dialect: r.doc.dialect.name.clone(),
        formats: crate::formats(),
        levels,
        diagnostics,
        failed: r.model.failed.clone(),
        files,
        objects,
        spans,
        missing,
    }
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SourceInclude {
    pub line: u32,
    /// Index in Document.files.
    pub file: usize,
}

/// The host files with the places of their records; the opened file comes first.
fn source_files(doc: &HostDocument, shown: &dyn Fn(&str) -> String) -> Vec<SourceFile> {
    let mut files: Vec<SourceFile> = doc
        .sources
        .iter()
        .map(|(p, text)| SourceFile {
            includes: Vec::new(),
            semantics: Vec::new(),
            path: p.clone(),
            name: shown(p),
            format: doc.file_format(p).map(|f| f.format.clone()),
            default_format: doc.file_format(p).map(|f| f.default.clone()),
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
        if let Some(site) = &lv.include_site {
            if let Some(target) = files.iter().position(|f| f.path == lv.file) {
                if let Some(file) = files.iter_mut().find(|f| f.path == site.file) {
                    let link = SourceInclude { line: site.line, file: target };
                    if !file.includes.contains(&link) {
                        file.includes.push(link);
                    }
                }
            }
        }
        for rec in &lv.records {
            let Some(f) = files.iter_mut().find(|f| f.path == rec.file) else { continue };
            let n = rec.text.chars().count();
            let (a, b) = (rec.loc(0), rec.loc(n.saturating_sub(1)));
            f.records.push([a.line, a.col, b.line, b.col]);
            if let Some(body) = rnl_runtime::syntax::parse_record(&rec.text).body {
                semantic_lines(&body, rec, &mut f.semantics);
            }
        }
    }
    for f in &mut files {
        f.records.sort();
        f.records.dedup();
        f.semantics.sort_unstable();
        f.semantics.dedup();
    }
    files
}

fn semantic_lines(set: &rnl_runtime::syntax::SetExpr, rec: &rnl_runtime::host::Record, out: &mut Vec<u32>) {
    for member in &set.members {
        if member.error.is_some() {
            continue;
        }
        out.extend(
            member
                .free_words
                .iter()
                .filter(|(word, _)| word.chars().any(char::is_alphanumeric))
                .map(|(_, off)| rec.loc(*off).line),
        );
        for arg in member.segments.iter().flat_map(|s| &s.args) {
            if let rnl_runtime::syntax::ObjExpr::Set(body) = &arg.obj {
                semantic_lines(body, rec, out);
            }
        }
    }
}

struct Index<'a> {
    doc: &'a HostDocument,
    files: &'a HashMap<String, usize>,
    objects: Vec<Object>,
    spans: Vec<Span>,
    insts: HashMap<(usize, usize), usize>,
    nets: HashMap<(usize, String), usize>,
    usages: HashMap<(usize, String), usize>,
}

impl<'a> Index<'a> {
    fn new(doc: &'a HostDocument, files: &'a HashMap<String, usize>) -> Index<'a> {
        Index {
            doc,
            files,
            objects: Vec::new(),
            spans: Vec::new(),
            insts: HashMap::new(),
            nets: HashMap::new(),
            usages: HashMap::new(),
        }
    }

    fn usage(&mut self, level: usize, id: &str) -> usize {
        let key = (level, id.to_string());
        if let Some(&o) = self.usages.get(&key) {
            return o;
        }
        let o = self.objects.len();
        self.objects.push(Object {
            kind: "usage",
            name: id.into(),
            level,
            level_title: self.doc.levels[level].title(),
            lines: None,
            opens: None,
        });
        self.usages.insert(key, o);
        o
    }

    /// The net of a level by the name a statement of its own file writes.
    fn net(&mut self, level: usize, written: &str) -> usize {
        let file = self.doc.levels[level].file.clone();
        self.net_in(level, written, &file)
    }

    /// The net of a level by the name a statement of the file `file` writes: compared as the
    /// runtime compares it ([`HostDocument::net_key_in`]).
    fn net_in(&mut self, level: usize, written: &str, file: &str) -> usize {
        let key = self.doc.net_key_in(level, written, file);
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
                self.nodes(lv, f, &inst.file, &tokens, 1, &inst.nodes, "node");
            }
            if level.kind == LevelKind::Subckt && level.line > 0 {
                if let Some(&f) = self.files.get(&level.file) {
                    if let Some((_, tokens)) = statement(&lines[f], level.line as usize - 1) {
                        // `.subckt NAME ports…`, `subckt NAME (ports…)` or `inline subckt NAME …`
                        let skip = if tokens.first().is_some_and(|t| t.text == "inline") { 3 } else { 2 };
                        self.nodes(lv, f, &level.file, &tokens, skip, &level.ports, "port");
                    }
                }
            }
        }
    }

    /// The tokens from `from` on that name `nodes`, in order, in a statement of the file `written`,
    /// the `file`th of the Document.
    #[allow(clippy::too_many_arguments)]
    fn nodes(
        &mut self,
        lv: usize,
        file: usize,
        written: &str,
        tokens: &[Token],
        from: usize,
        nodes: &[String],
        kind: &'static str,
    ) {
        let rules = self.doc.rules(written);
        let mut j = from;
        for node in nodes {
            let key = dialect_key(rules, node);
            while j < tokens.len() && dialect_key(rules, &tokens[j].text) != key {
                j += 1;
            }
            let Some(t) = tokens.get(j) else { break };
            let object = self.net_in(lv, node, written);
            self.spans.push(Span { object, file, line: t.line, col: t.col, len: t.len, kind });
            j += 1;
        }
    }

    /// The drawing of a level, as `options` ask, and what on it can be clicked.
    fn level(&mut self, lv: &LevelView, options: &BuildOptions, lazy: bool) -> Level {
        let lv = &with_page_ids(lv);
        let render = rnl_render::RenderOptions::default();
        let prims = rnl_render::primitives(lv, &render);
        let px = |p: Pt| [p.0 * render.scale, -p.1 * render.scale];
        let mut hits = Vec::new();
        // what each item stands for, for the labels attached to it
        let mut item_objects: Vec<Vec<usize>> = Vec::with_capacity(lv.items.len());
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
            // a usage the level names, such as `%gnd1`, has that name; the others have none
            let name = (it.kind == "usage")
                .then(|| it.object_id.strip_prefix('%'))
                .flatten()
                .map(|n| n.trim_matches(['\'', '"']).to_string());
            // A virtual symbol is selectable independently of its net, including a junction
            // on a point with no pins. Keep the net on the hit for network highlighting.
            let mut labels = objects.clone();
            if it.kind == "usage" {
                let usage = self.usage(lv.level, &it.object_id);
                objects.insert(0, usage);
                // Preserve clicking a label to select its net; the usage also owns its label.
                labels.push(usage);
            }
            item_objects.push(labels);
            hits.push(Hit {
                kind: "box",
                points: vec![px((left, top)), px((right, bottom))],
                objects,
                id: Some(it.id.clone()),
                name,
            });
            for (pin, nets) in sym.pins.iter().zip(&it.pin_nets) {
                if nets.is_empty() {
                    continue;
                }
                let objects = nets.iter().map(|n| self.net(lv.level, n)).collect();
                let points = vec![px(place(it.at, it.orient, pin.pos))];
                hits.push(Hit { kind: "pin", points, objects, id: None, name: None });
            }
        }
        for w in &lv.wires {
            let objects = w.nets.iter().map(|n| self.net(lv.level, n)).collect();
            hits.push(Hit {
                kind: "wire",
                points: w.points.iter().map(|p| px(*p)).collect(),
                objects,
                id: Some(w.id.clone()),
                name: None,
            });
        }
        // a label stands for what it is attached to (a terminal's for its pin's nets), and for
        // nothing when that cannot be selected:
        // a click on it is then not a click on whatever is under it. with_page_ids gives each
        // text its own groups, including labels of repeated unnamed usages.
        let boxes = rnl_render::text_boxes(lv, &render);
        for (t, bbox) in lv.texts.iter().zip(boxes) {
            let (true, Some([x0, y0, x1, y1])) = (t.attach.is_some(), bbox) else { continue };
            let objects = match &t.target {
                Some(TextTarget::Item(i)) => item_objects.get(*i).cloned().unwrap_or_default(),
                Some(TextTarget::Wire(i)) => {
                    lv.wires.get(*i).map(|w| w.nets.iter().map(|n| self.net(lv.level, n)).collect()).unwrap_or_default()
                }
                Some(TextTarget::Net(n)) => vec![self.net(lv.level, n)],
                // a terminal's label stands for its pin's nets, as the pin does, and for nothing
                // when the pin has none
                Some(TextTarget::Pin(i, k)) => lv
                    .items
                    .get(*i)
                    .and_then(|it| it.pin_nets.get(*k))
                    .map(|nets| nets.iter().map(|n| self.net(lv.level, n)).collect())
                    .unwrap_or_default(),
                None => Vec::new(),
            };
            hits.push(Hit {
                kind: "label",
                points: vec![px((x0.min(x1), y0.max(y1))), px((x0.max(x1), y0.min(y1)))],
                objects,
                id: Some(t.id.clone()),
                name: None,
            });
        }
        Level {
            currents: crate::overlays::currents(lv, render.scale),
            usages: Vec::new(),
            ports: self.doc.levels[lv.level].ports.clone(),
            symbol: None,
            host: lv.level,
            title: lv.title.clone(),
            kind: lv.kind,
            lines: None,
            svg: options.svg.then(|| rnl_render::svg_document(&prims, &render)),
            primitives: options.primitives.then(|| primitive::primitives(&prims)),
            scale: render.scale,
            empty: prims.is_empty(),
            status: crate::level_status(lv),
            solve: None,
            hits,
            children: Vec::new(),
            unplaced: self.unplaced(lv, &render, options, lazy),
        }
    }

    /// The objects of a level without a position, each drawn alone as `options` ask, or, when
    /// `lazy`, not drawn: [`Drawings`] gives their drawings on request.
    fn unplaced(
        &self,
        lv: &LevelView,
        render: &rnl_render::RenderOptions,
        options: &BuildOptions,
        lazy: bool,
    ) -> Vec<Unplaced> {
        // a level of nothing but the symbols of the objects, with one of them in turn on it
        let draws = !lazy && (options.svg || options.primitives);
        let mut alone =
            LevelView { symbols: if draws { lv.unplaced_symbols.clone() } else { Vec::new() }, ..LevelView::default() };
        let mut out = Vec::new();
        for it in &lv.unplaced_items {
            let sym = &lv.unplaced_symbols[it.symbol];
            let drawn = draws.then(|| draw_alone(&mut alone, it, render, options.svg, options.primitives));
            let name = match it.host.as_slice() {
                [i] => self.doc.levels[lv.level].instances.get(*i).map(|inst| inst.name.clone()),
                _ => None,
            };
            out.push(Unplaced {
                kind: it.kind,
                name: name.unwrap_or_else(|| it.id.clone()),
                symbol: sym.name.clone(),
                reason: None,
                objects: it.host.iter().filter_map(|i| self.insts.get(&(lv.level, *i)).copied()).collect(),
                svg: drawn.as_ref().and_then(|d| d.svg.clone()),
                primitives: drawn.and_then(|d| d.primitives),
            });
        }
        // the devices that are not drawn at all: listed after, with the reason and no drawing
        for u in &lv.undrawn {
            let name = match u.host.as_slice() {
                [i] => self.doc.levels[lv.level].instances.get(*i).map(|inst| inst.name.clone()),
                _ => None,
            };
            out.push(Unplaced {
                kind: if u.host.len() == 1 { "instance" } else { "bundle" },
                name: name.unwrap_or_else(|| u.id.clone()),
                symbol: String::new(),
                reason: Some(u.reason.clone()),
                objects: u.host.iter().filter_map(|i| self.insts.get(&(lv.level, *i)).copied()).collect(),
                svg: None,
                primitives: None,
            });
        }
        out
    }
}

/// How far a net's name stands from its pin, in scene units, and the size of its text.
const LABEL_GAP: f64 = 0.35;
const LABEL_SIZE: f64 = 1.0;

/// The names of the nets at the pins of an unplaced object's symbol, each beside its pin on the
/// side the pin points to; a pin with no net has none.
fn net_labels(it: &ItemView, sym: &SymbolView) -> Vec<TextView> {
    let mut style = Style::new();
    style.insert("size".into(), Lit::Num(LABEL_SIZE));
    style.insert("textcolor".into(), Lit::Str(WStr::new("#1f4fd8", None)));
    let centre = ((sym.bbox[0] + sym.bbox[2]) / 2.0, (sym.bbox[1] + sym.bbox[3]) / 2.0);
    let mut out = Vec::new();
    for (k, (pin, nets)) in sym.pins.iter().zip(&it.pin_nets).enumerate() {
        if nets.is_empty() {
            continue;
        }
        // the way out of the pin: its own, else away from the middle of the symbol
        let outward = (pin.pos.0 - centre.0, pin.pos.1 - centre.1);
        let away = match pin.dir {
            Some(0) => (1.0, 0.0),
            Some(90) => (0.0, 1.0),
            Some(180) => (-1.0, 0.0),
            Some(270) => (0.0, -1.0),
            _ => outward,
        };
        let (dx, dy) = it.orient.apply(away);
        let (halign, valign, off) = if dx.abs() >= dy.abs() {
            if dx >= 0.0 {
                ("start", "middle", (LABEL_GAP, 0.0))
            } else {
                ("end", "middle", (-LABEL_GAP, 0.0))
            }
        } else if dy >= 0.0 {
            ("middle", "bottom", (0.0, LABEL_GAP))
        } else {
            ("middle", "top", (0.0, -LABEL_GAP))
        };
        let p = it.orient.apply(pin.pos);
        let text =
            nets.iter().map(|n| format!("{}{n}{}", markup::LITERAL, markup::LITERAL)).collect::<Vec<_>>().join(", ");
        out.push(TextView {
            id: format!("net-{k}"),
            object_id: format!("net-{k}"),
            attach: None,
            target: None,
            key: None,
            markup: text.clone(),
            written: false,
            anchor: (it.at.0 + p.0 + off.0, it.at.1 + p.1 + off.1),
            lines: markup::parse(&text),
            halign: halign.into(),
            valign: valign.into(),
            rotate: 0.0,
            orient: Orient::R0,
            style: style.clone(),
            own_style: Style::new(),
            layer: Layer::Top,
            order: vec![1.0, k as f64],
        });
    }
    out
}

/// A level with its items and wires named for the page, "item-K" and "wire-K" by their index. The
/// model names them for people ("a wire"); the page needs each SVG group and primitive to say
/// which one it draws. Repeated text ids also get separate display ids; unique source ids stay
/// unchanged, and the runtime's object ids and attachment targets are untouched.
pub(crate) fn with_page_ids(source: &LevelView) -> LevelView {
    let mut lv = source.clone();
    for (k, it) in lv.items.iter_mut().enumerate() {
        it.id = format!("item-{k}");
    }
    for (k, w) in lv.wires.iter_mut().enumerate() {
        w.id = format!("wire-{k}");
    }
    let mut counts = HashMap::<&str, usize>::new();
    for t in &source.texts {
        *counts.entry(&t.id).or_default() += 1;
    }
    if counts.len() == lv.texts.len() {
        return lv;
    }
    let mut used: HashSet<String> = counts.keys().map(|id| (*id).to_owned()).collect();
    for (k, t) in lv.texts.iter_mut().enumerate() {
        if counts[t.id.as_str()] > 1 {
            let mut id = format!("text-{k}");
            while !used.insert(id.clone()) {
                id.push('_');
            }
            t.id = id;
        }
    }
    lv
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
        assert!(d.levels.iter().all(|l| l.svg.as_ref().is_some_and(|s| s.starts_with("<svg")) && !l.empty));
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
    fn the_objects_without_a_position_are_listed_with_their_symbols_and_nets() {
        let text = "* two of three are left to a solver
R1 a b 1k
C1 b 0 1p
.model nmos nmos
M1 d g s 0 nmos
* @RNL { #R1 @PLACE(0, 0) }
.end
";
        let files = rnl_runtime::source::MemoryProvider::new().with("a.sp", text);
        let d = crate::build("a.sp", &files, &BuildOptions::default()).unwrap();
        let level = &d.levels[0];
        // the two instances, and the grounds of the net 0 that nothing places either
        let status = level.status.as_deref().unwrap();
        assert!(status.starts_with(&format!("{} without a position", level.unplaced.len())), "{status}");
        let instances: Vec<&str> =
            level.unplaced.iter().filter(|u| u.kind == "instance").map(|u| u.name.as_str()).collect();
        assert_eq!(instances, ["C1", "M1"]);
        assert!(level.unplaced.iter().filter(|u| u.kind == "usage").all(|u| u.objects.is_empty()));
        // each stands for its instance in the Document's objects
        let objects: Vec<(&str, usize)> = level
            .unplaced
            .iter()
            .filter(|u| u.kind == "instance")
            .map(|u| (d.objects[u.objects[0]].name.as_str(), u.objects.len()))
            .collect();
        assert_eq!(objects, [("C1", 1), ("M1", 1)]);
        assert!(level.unplaced.iter().all(|u| !u.symbol.is_empty()));
        // the SVG draws the symbol and the name of the net at each of its pins, as texts
        let svg = |k: usize| level.unplaced[k].svg.as_deref().unwrap();
        let at = |name: &str| level.unplaced.iter().position(|u| u.name == name).unwrap();
        for (k, nets) in [(at("C1"), vec!["b", "0"]), (at("M1"), vec!["d", "g", "s", "0"])] {
            assert!(svg(k).starts_with("<svg"), "{}", svg(k));
            for net in nets {
                assert!(svg(k).contains(&format!(">{net}</text>")), "{net} is not in {}", svg(k));
            }
        }
        // the drawing of the level has neither of them
        assert!(!level.svg.as_ref().unwrap().contains("fill=\"#1f4fd8\""));
        assert!(level
            .hits
            .iter()
            .all(|h| h.objects.iter().all(|o| !["C1", "M1"].contains(&d.objects[*o].name.as_str()))));
    }

    #[test]
    fn the_drawings_of_the_unplaced_can_be_asked_for_instead_of_carried() {
        let text = "* some left\n.model nmos nmos\nR1 a b 1k\nC1 b 0 1p\nM1 d g s 0 nmos\nR2 a d 2k\n* @RNL { #R1 @PLACE(0, 0) }\n.end\n";
        let files = rnl_runtime::source::MemoryProvider::new().with("a.sp", text);
        let options = BuildOptions { primitives: true, ..BuildOptions::default() };
        let carried = crate::build("a.sp", &files, &options).unwrap();
        let (kept, drawings) = crate::build_drawn("a.sp", &files, &options).unwrap();
        // the Document is the same but for what the objects carry of their drawings
        let mut v = serde_json::to_value(&carried).unwrap();
        for level in v["levels"].as_array_mut().unwrap() {
            for u in level["unplaced"].as_array_mut().unwrap() {
                let u = u.as_object_mut().unwrap();
                assert!(u.remove("svg").is_some() && u.remove("primitives").is_some());
            }
        }
        assert_eq!(serde_json::to_value(&kept).unwrap(), v);
        let listed = &carried.levels[0].unplaced;
        assert!(listed.len() >= 3);
        assert!(kept.levels[0].unplaced.iter().all(|u| u.svg.is_none() && u.primitives.is_none()));
        // asked for, in any order, each is what the Document carries
        let names: Vec<String> = listed.iter().map(|u| u.name.clone()).collect();
        let all: Vec<usize> = (0..listed.len()).collect();
        let got = drawings.unplaced(0, &all, &names).unwrap();
        for (u, d) in listed.iter().zip(&got) {
            assert_eq!((&u.svg, &u.primitives), (&d.svg, &d.primitives), "{}", u.name);
        }
        let some = drawings.unplaced(0, &[2, 0], &[names[2].clone(), names[0].clone()]).unwrap();
        assert_eq!(some, [got[2].clone(), got[0].clone()]);
        // the names are optional, and, given, must be those of the Document: a Document built again
        // is not answered with the drawings of other objects
        assert_eq!(drawings.unplaced(0, &[1], &[]).unwrap(), [got[1].clone()]);
        let e = drawings.unplaced(0, &[1], &["R9".to_string()]).unwrap_err();
        assert!(e.contains("built again"), "{e}");
        assert!(drawings.unplaced(0, &[99], &[]).is_err());
        assert!(drawings.unplaced(7, &[0], &[]).is_err());
        assert!(drawings.unplaced(0, &[0, 1], &["R2".to_string()]).is_err());
    }

    #[test]
    fn a_label_stands_for_what_it_is_attached_to() {
        let text = "* labels
R1 a b 1k
C1 b 0 1p
* @RNL(type=\"header\") { @VERSION(\"0.2\")}
* @RNL {
*   #R1 @PLACE(0, 0);
*   #C1 @PLACE(6, 0);
*   (-3, 5) @TEXT(\"free\");
*   #R1:1 @TEXT(\"pin\", dy=1)
* }
.end
";
        let files = rnl_runtime::source::MemoryProvider::new().with("a.sp", text);
        let d = crate::build("a.sp", &files, &BuildOptions::default()).unwrap();
        let level = &d.levels[0];
        let labels: Vec<&Hit> = level.hits.iter().filter(|h| h.kind == "label").collect();
        let names = |h: &Hit| h.objects.iter().map(|o| d.objects[*o].name.as_str()).collect::<Vec<_>>();
        // the name and value of each instance stand for it; the text at a point is no label
        let of = |name: &str| labels.iter().filter(|h| names(h) == [name]).count();
        assert!((of("R1"), of("C1")) == (2, 2) || (of("R1"), of("C1")) == (2, 1), "{labels:?}");
        assert!(labels.iter().all(|h| h.points.len() == 2 && h.points[0][0] < h.points[1][0]));
        assert!(labels.iter().all(|h| h.points[0][1] < h.points[1][1]));
        // a terminal's label stands for its pin's nets, as a click on the pin does
        let pin = level.hits.iter().find(|h| h.kind == "pin" && names(h) == ["a"]).expect("R1's first pin");
        let terminal: Vec<&&Hit> = labels.iter().filter(|h| names(h) == ["a"]).collect();
        assert_eq!(terminal.len(), 1, "{labels:?}");
        assert_eq!(terminal[0].objects, pin.objects);
        assert!(labels.iter().all(|h| !h.objects.is_empty()), "{labels:?}");
        // a label's group is in the SVG, and its box is around the text
        let svg = level.svg.as_deref().unwrap();
        for h in &labels {
            let id = h.id.as_deref().unwrap();
            assert!(svg.contains(&format!("data-id=\"{id}\"")), "{id}");
        }
        // none of it changes the other hits
        assert_eq!(level.hits.iter().filter(|h| h.kind == "box").count(), 2);
    }

    #[test]
    fn a_terminal_that_no_pin_draws_has_no_label_to_click() {
        // the symbol draws a pin for R1's first terminal only: the second is not drawn, nor is its
        // label, so nothing on the drawing stands for it; the first's label stands for its pin's net
        let text = "* a terminal without a pin
R1 a b 1k
* @RNL(type=\"header\") { @VERSION(\"0.2\")}
* @RNL {
*   $ONE = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); @LINE {(0, 0) (0, 2)} };
*   #R1 @SET-SYMBOL $ONE;
*   #R1 @PLACE(0, 0);
*   #R1:1 @TEXT(\"one\", dx=2);
*   #R1:2 @TEXT(\"two\", dx=-2)
* }
.end
";
        let files = rnl_runtime::source::MemoryProvider::new().with("a.sp", text);
        let d = crate::build("a.sp", &files, &BuildOptions::default()).unwrap();
        let level = &d.levels[0];
        let names = |h: &Hit| h.objects.iter().map(|o| d.objects[*o].name.as_str()).collect::<Vec<_>>();
        let pins: Vec<&Hit> = level.hits.iter().filter(|h| h.kind == "pin").collect();
        assert_eq!(pins.iter().map(|h| names(h)).collect::<Vec<_>>(), [["a"]], "{pins:?}");
        // the symbol has no labels of its own: the labels are the terminals'
        let terminals: Vec<&Hit> = level.hits.iter().filter(|h| h.kind == "label").collect();
        assert_eq!(terminals.iter().map(|h| names(h)).collect::<Vec<_>>(), [["a"]], "{terminals:?}");
        assert_eq!(terminals[0].objects, pins[0].objects);
        assert!(!level.svg.as_deref().unwrap().contains(">two<"));
    }

    #[test]
    fn labels_with_repeated_runtime_ids_get_independent_display_groups() {
        // the supply symbols of the corpus: four unnamed usages, whose labels are all `%s::name`
        let d = load(&case("03-five-transistor-ota.sp")).unwrap();
        let level = &d.levels[0];
        let is_vdd = |h: &Hit| h.objects.iter().any(|o| d.objects[*o].kind == "net" && d.objects[*o].name == "vdd");
        let on_vdd = |kind: &str| level.hits.iter().filter(|h| h.kind == kind && is_vdd(h)).collect::<Vec<&Hit>>();
        let vdd = on_vdd("label");
        assert!(vdd.len() >= 3, "{vdd:?}");
        let ids: HashSet<_> = vdd.iter().map(|h| h.id.as_deref().expect("label group")).collect();
        assert_eq!(ids.len(), vdd.len());
        let svg = level.svg.as_deref().unwrap();
        for id in &ids {
            assert!(svg.contains(&format!("data-id=\"{id}\"")), "{id}");
        }
        // A different net must not share any of the VDD labels' display groups.
        for h in level.hits.iter().filter(|h| h.kind == "label" && !is_vdd(h)) {
            assert!(!ids.contains(h.id.as_deref().expect("label group")));
        }
        // Each still stands for the net, as well as its own independently selectable usage.
        assert_eq!(vdd.len(), on_vdd("box").len());
        // the supply symbols have no name of their own, the ports of the netlist have
        let named: Vec<(&str, Vec<&str>)> = level
            .hits
            .iter()
            .filter_map(|h| {
                h.name.as_deref().map(|n| {
                    (
                        n,
                        h.objects
                            .iter()
                            .filter(|o| d.objects[**o].kind == "net")
                            .map(|o| d.objects[*o].name.as_str())
                            .collect(),
                    )
                })
            })
            .collect();
        assert!(on_vdd("box").iter().all(|h| h.name.is_none()));
        assert_eq!(
            named,
            [("pi", vec!["inp"]), ("pn", vec!["inn"]), ("po", vec!["out"])],
            "the ports of the netlist have their names"
        );
        assert!(named.iter().all(|(_, nets)| nets.len() == 1), "{named:?}");
    }

    #[test]
    fn unkeyed_wire_labels_get_separate_groups_for_each_net() {
        let files = rnl_runtime::source::MemoryProvider::new()
            .with("wire-labels.sp", include_str!("../../../tests/viewer/inputs/wire-labels.sp"));
        let d = crate::build("wire-labels.sp", &files, &BuildOptions::default()).unwrap();
        let level = &d.levels[0];
        let labels: Vec<_> = level
            .hits
            .iter()
            .filter(|h| h.kind == "label" && h.objects.len() == 1 && d.objects[h.objects[0]].kind == "net")
            .collect();
        assert_eq!(labels.len(), 2);
        let ids: HashSet<_> = labels.iter().map(|h| h.id.as_deref().expect("net label group")).collect();
        assert_eq!(ids.len(), 2);
        for id in ids {
            assert!(level.svg.as_deref().unwrap().contains(&format!("data-id=\"{id}\"")));
        }
    }

    #[test]
    fn display_text_ids_avoid_existing_ids_and_preserve_model_identity() {
        let r = crate::interpret(&case("03-five-transistor-ota.sp"), None, &Disk, &Default::default()).unwrap();
        let mut level = r.model.levels[0].clone();
        level.texts[0].id = "text-1".into();
        level.texts[1].id = "repeated".into();
        level.texts[2].id = "repeated".into();
        let page = with_page_ids(&level);
        assert_eq!(page.texts[0].id, "text-1");
        let ids: HashSet<_> = page.texts.iter().map(|t| &t.id).collect();
        assert_eq!(ids.len(), page.texts.len());
        for (original, displayed) in level.texts.iter().zip(&page.texts) {
            assert_eq!(original.object_id, displayed.object_id);
            assert_eq!(original.attach, displayed.attach);
            assert_eq!(original.anchor, displayed.anchor);
        }
        assert_eq!(level.texts[1].id, "repeated");
    }

    #[test]
    fn the_unplaced_follow_the_outputs_the_frontend_asks_for() {
        let text = "* one left
R1 a b 1k
.end
";
        let files = rnl_runtime::source::MemoryProvider::new().with("a.sp", text);
        let build = |svg: bool, primitives: bool| {
            let options = BuildOptions { svg, primitives, ..BuildOptions::default() };
            crate::build("a.sp", &files, &options).unwrap().levels.remove(0).unplaced.remove(0)
        };
        let u = build(true, false);
        assert!(u.svg.is_some() && u.primitives.is_none());
        let u = build(false, true);
        assert!(
            u.svg.is_none()
                && u.primitives.is_some_and(|p| p.iter().any(|p| matches!(p.shape, primitive::Shape::Text(_))))
        );
    }

    #[test]
    fn a_level_with_everything_placed_lists_nothing() {
        let d = load(&case("04-hierarchy.sp")).unwrap();
        assert!(d.levels.iter().all(|l| l.unplaced.is_empty()));
    }

    #[test]
    fn a_missing_file_is_an_error() {
        assert!(load(&case("no-such-case.sp")).is_err());
    }

    #[test]
    fn an_entry_at_the_root_names_its_files_as_one_in_a_directory_does() {
        // a file beside the entry and one below it, each with a record that refers to nothing, and
        // an include that is missing; no header record, so the runtime gives a hint at the main
        // file by its level's file
        let top = "* top\n.include \"x.inc\"\n.include \"sub/y.inc\"\n.include \"gone.inc\"\nR1 a 0 1k\n* @RNL {#R9 @PLACE(0, 0)}\n.end\n";
        let x = "* x\n.subckt inner p q\nR2 p q 1k\n* @RNL {#R8 @PLACE(0, 0)}\n.ends\n";
        let y = "* y\n.subckt lower p q\nR3 p q 1k\n* @RNL {#R7 @PLACE(0, 0)}\n.ends\n";
        let build = |dir: &str| {
            let files = rnl_runtime::source::MemoryProvider::new()
                .with(&format!("{dir}/top.sp"), top)
                .with(&format!("{dir}/x.inc"), x)
                .with(&format!("{dir}/sub/y.inc"), y);
            crate::build(&format!("{dir}/top.sp"), &files, &BuildOptions::default()).unwrap()
        };
        let (root, w) = (build(""), build("/w"));
        let names: Vec<(&str, &str)> = root.files.iter().map(|f| (f.path.as_str(), f.name.as_str())).collect();
        assert_eq!(names, [("top.sp", "top.sp"), ("x.inc", "x.inc"), ("sub/y.inc", "sub/y.inc")]);
        assert_eq!(root.files[0].includes, [SourceInclude { line: 2, file: 1 }, SourceInclude { line: 3, file: 2 }]);
        assert_eq!(root.missing, ["/gone.inc"]);
        // every diagnostic is at a line of one of the files, and names it as the file's tab does
        for x in &root.diagnostics {
            let file = x.file_index.map(|k| root.files[k].name.as_str());
            assert!(x.line > 0 && file == Some(x.file.as_str()) && x.full_path == x.file, "{x:?}");
        }
        let lines = |d: &Document| d.diagnostics.iter().map(|x| (x.file_index, x.line)).collect::<Vec<_>>();
        let want = [(0, 4), (0, 2), (0, 6), (0, 1), (1, 4), (1, 2), (2, 4), (2, 2)].map(|(k, line)| (Some(k), line));
        assert_eq!(lines(&root), want);
        // the same Document as in a directory, but for the paths that name the entry and what is missing
        let json = |d: &Document| {
            let mut v = serde_json::to_value(d).unwrap();
            v["path"] = serde_json::Value::Null;
            v["missing"] = serde_json::Value::Null;
            v
        };
        assert_eq!(json(&root), json(&w));
        assert_eq!((w.path.as_str(), w.missing.as_slice()), ("/w/top.sp", ["/w/gone.inc".to_string()].as_slice()));
    }

    #[test]
    fn include_links_use_resolved_paths_sections_and_exclude_auto_includes() {
        use rnl_runtime::source::MemoryProvider;
        let files = MemoryProvider::new()
            .with("/w/top.sp", "* t\n.include a/x.inc\n.include b/x.inc\n.lib lib.inc tt\n")
            .with("/w/a/x.inc", ".subckt a p n\n.ends\n")
            .with("/w/b/x.inc", ".subckt b p n\n.ends\n")
            .with("/w/lib.inc", ".lib tt\n.model n nmos\n.endl\n")
            .with("/w/auto.inc", ".subckt auto p n\n.ends\n");
        let host =
            rnl_host_spice::read_with_includes("/w/top.sp", &files, &mut Sink::default(), &["/w/auto.inc".into()])
                .unwrap();
        let shown = source_files(&host, &str::to_string);
        let targets: Vec<_> = shown[0].includes.iter().map(|i| (i.line, shown[i.file].path.as_str())).collect();
        assert_eq!(targets, [(2, "a/x.inc"), (3, "b/x.inc"), (4, "lib.inc")]);

        let files = MemoryProvider::new()
            .with("/w/top.scs", "simulator lang=spectre\ninclude \"a/x.scs\" section=tt\ninclude \"b/x.scs\"\n")
            .with("/w/a/x.scs", "section tt\nmodel n mos1\nendsection\n")
            .with("/w/b/x.scs", "subckt b p n\nends b\n");
        let host = rnl_host_spectre::read("/w/top.scs", &files, &mut Sink::default()).unwrap();
        let shown = source_files(&host, &str::to_string);
        let targets: Vec<_> = shown[0].includes.iter().map(|i| (i.line, shown[i.file].path.as_str())).collect();
        assert_eq!(targets, [(2, "a/x.scs"), (3, "b/x.scs")]);
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
        let svg = d.levels[0].svg.as_deref().unwrap();
        assert!(ids.iter().all(|id| svg.contains(&format!("data-id=\"{id}\""))));
        assert!(hits.iter().any(|h| h.kind == "pin" && h.objects.contains(&out)));
        // A ground symbol selects its own usage, while retaining its ground net for highlighting.
        let gnd = d.objects.iter().position(|o| o.kind == "net" && o.name == "0").unwrap();
        assert!(hits.iter().any(|h| h.kind == "box"
            && h.objects.len() == 2
            && d.objects[h.objects[0]].kind == "usage"
            && h.objects[1] == gnd));
    }

    #[test]
    fn usages_have_independent_objects_and_preserve_net_hits() {
        let text = r#"* virtual objects
R1 rail out 1k
* @RNL(type="header") { @VERSION("0.3") }
* @RNL {
*   #R1 @PLACE(0, 0);
*   %g = $GROUND @ATTACH &rail; %g @PLACE(-8, -4);
*   %g2 = $GROUND @ATTACH &rail; %g2 @PLACE(-2, -4);
*   %s = $SUPPLY @ATTACH &rail; %s @PLACE(8, 6);
*   %p = $IO_PORT @ATTACH &rail; %p @PLACE(12, 6);
*   %j = $JUNCTION @ATTACH (4, -5);
*   &rail @WIRE {%g:1 (-8, 3) #R1:1 (8, 3) %s:1}
* }
.end
"#;
        let files = rnl_runtime::source::MemoryProvider::new().with("virtual.sp", text);
        let d = crate::build("virtual.sp", &files, &BuildOptions::default()).unwrap();
        assert!(d.failed.is_none());
        assert!(!d.diagnostics.iter().any(|x| x.severity == "error"), "{:?}", d.diagnostics);
        let hits = &d.levels[0].hits;
        let net = d.objects.iter().position(|o| o.kind == "net" && o.name == "rail").unwrap();
        let mut usages = Vec::new();
        for name in ["g", "g2", "s", "p", "j"] {
            let hit = hits.iter().find(|h| h.kind == "box" && h.name.as_deref() == Some(name)).unwrap();
            let usage = hit.objects[0];
            assert_eq!(d.objects[usage].kind, "usage");
            assert_eq!(d.objects[usage].name, format!("%{name}"));
            assert_eq!((d.objects[usage].level, d.objects[usage].opens), (0, None));
            assert!(d.objects[usage].lines.is_some());
            assert!(!usages.contains(&usage));
            usages.push(usage);
            let nets = if name == "j" { Vec::new() } else { vec![net] };
            assert_eq!(&hit.objects[1..], nets.as_slice());
            for label in hits.iter().filter(|h| h.kind == "label" && h.objects.contains(&usage)) {
                assert_eq!(label.objects, [net, usage]);
            }
        }
        assert!(hits.iter().any(|h| h.kind == "wire" && h.objects == [net]));
        assert!(hits.iter().any(|h| h.kind == "pin" && h.objects == [net]));
        assert!(hits
            .iter()
            .filter(|h| h.kind == "wire" || h.kind == "pin")
            .all(|h| h.objects.iter().all(|o| d.objects[*o].kind == "net")));
    }

    #[test]
    fn the_hits_and_the_primitives_name_the_same_drawing() {
        let options = BuildOptions { primitives: true, ..BuildOptions::default() };
        let d = crate::build(&case("01-rc-lowpass.sp"), &Disk, &options).unwrap();
        let level = &d.levels[0];
        let (prims, scale) = (level.primitives.as_ref().unwrap(), level.scale);
        let svg = level.svg.as_deref().unwrap();
        let mut wires = 0;
        for h in &level.hits {
            let Some(id) = &h.id else { continue };
            let owned: Vec<&Primitive> = prims.iter().filter(|p| p.owner == *id).collect();
            assert!(!owned.is_empty() && svg.contains(&format!("data-id=\"{id}\"")), "{id}");
            if h.kind == "wire" {
                // a wire's vertices in SVG pixels are those of its path in scene units
                let crate::Shape::Path { segments, .. } = &owned[0].shape else { panic!("{:?}", owned[0]) };
                let points: Vec<[f64; 2]> = segments
                    .iter()
                    .map(|s| match s {
                        crate::Segment::Move { to } | crate::Segment::Line { to } => [to[0] * scale, -to[1] * scale],
                        s => panic!("{s:?}"),
                    })
                    .collect();
                assert_eq!(points, h.points);
                wires += 1;
            }
        }
        assert!(wires > 2);
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

    #[test]
    fn a_device_that_cannot_be_drawn_is_listed_after_the_unplaced_with_its_reason() {
        let text = "* a transistor without its model card is not drawn
.model nch nmos
R1 a b 1k
M1 d g s b nch
M2 d g s b pch
* @RNL { #R1 @PLACE(0, 0) }
.end
";
        let files = rnl_runtime::source::MemoryProvider::new().with("a.sp", text);
        let d = crate::build("a.sp", &files, &BuildOptions::default()).unwrap();
        let level = &d.levels[0];
        let listed: Vec<(&str, &str, Option<&str>)> =
            level.unplaced.iter().map(|u| (u.name.as_str(), u.symbol.as_str(), u.reason.as_deref())).collect();
        let names: Vec<&str> = listed.iter().map(|l| l.0).collect();
        assert_eq!(names.last(), Some(&"M2"), "{listed:?}");
        assert_eq!(listed.last().unwrap(), &("M2", "", Some("model card not found")));
        // the others, which a solver can place, have a symbol and no reason
        assert!(listed[..listed.len() - 1].iter().all(|(_, symbol, reason)| !symbol.is_empty() && reason.is_none()));
        // it has no drawing, and stands for its instance in the objects
        let m2 = level.unplaced.last().unwrap();
        assert!(m2.svg.is_none() && m2.primitives.is_none());
        assert_eq!((m2.kind, m2.objects.len()), ("instance", 1));
        assert_eq!(d.objects[m2.objects[0]].name, "M2");
        // the status counts the objects without a position, not the ones that cannot be drawn
        assert!(level.status.as_deref().unwrap().starts_with("1 without a position"), "{:?}", level.status);
        // in the JSON the reason is there for it alone
        let json = serde_json::to_value(&d).unwrap();
        let rows = json["levels"][0]["unplaced"].as_array().unwrap();
        assert_eq!(rows.iter().filter(|u| u.get("reason").is_some()).count(), 1);
        assert_eq!(rows.last().unwrap()["reason"], "model card not found");
    }
}
