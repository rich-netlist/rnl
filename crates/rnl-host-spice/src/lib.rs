//! SPICE host adapter (spec-draft 2.1–2.7, 6.1, 6.3, 6.12).
//!
//! Reads a SPICE netlist and the files it includes into the runtime's dialect-neutral host model:
//! levels (the main file level, included file levels, `.lib` sections, subcircuits), instances,
//! ports, and the RNL records cut out of the carrier lines. It covers the structural subset:
//! elements, nets, subcircuits and their ports, include and lib, model names and types. It does
//! not evaluate expressions or interpret analyses.
//!
//! A carrier is a whole-line comment with `*` in column 1 followed by a space, or a bare `*`
//! (2.2). The first line of every file is a title and never a carrier.

use std::collections::HashMap;

use rnl_runtime::diag::{Loc, Sink};
use rnl_runtime::host::{file_name, Dialect, HostDocument, Instance, Level, LevelId, LevelKind};
use rnl_runtime::records::{scan_run, CarrierLine};
use rnl_runtime::source::{dirname, join, normalize, Provider};

/// The SPICE dialect: names ignore case; ngspice's `gnd` is ground (6.3).
pub fn dialect() -> Dialect {
    Dialect { name: "spice".into(), case_sensitive: false, ground_aliases: vec!["gnd".into()] }
}

/// The adapter's default DEVICE table: terminal names by element letter (6.12).
pub fn default_terminals(letter: char) -> Option<Vec<String>> {
    let names: &[&str] = match letter {
        'M' => &["d", "g", "s", "b"],
        'Q' => &["c", "b", "e", "s"],
        'J' => &["d", "g", "s"],
        'D' => &["a", "k"],
        'R' | 'C' | 'L' | 'V' | 'I' | 'B' | 'F' | 'H' | 'W' => &["p", "n"],
        'E' | 'G' | 'S' => &["p", "n", "cp", "cn"],
        _ => return None,
    };
    Some(names.iter().map(|s| s.to_string()).collect())
}

/// The DEVICE statements equivalent to [`default_terminals`], as a tool may write them out (6.12).
pub fn default_device_statements() -> Vec<String> {
    "MQJDRCLVIBFHWEGS"
        .chars()
        .map(|l| {
            let t = default_terminals(l).unwrap();
            let quoted: Vec<String> = t.iter().map(|x| format!("\"{x}\"")).collect();
            format!("@DEVICE(name=\"{l}*\", terminals=[{}])", quoted.join(", "))
        })
        .collect()
}

/// Remove an inline comment: from a `;` or `$` that follows whitespace to the end of the line.
pub fn strip_inline_comment(text: &str) -> String {
    let padded = format!(" {text}");
    let chars: Vec<char> = padded.chars().collect();
    for k in 1..chars.len() {
        if (chars[k] == ';' || chars[k] == '$') && chars[k - 1].is_whitespace() {
            return chars[..k - 1].iter().collect::<String>().trim().to_string();
        }
    }
    padded.trim().to_string()
}

/// Split the tokens of an element line into plain tokens and `name=value` parameters.
pub fn split_params(tokens: &[&str]) -> (Vec<String>, Vec<(String, String)>) {
    let mut plain = Vec::new();
    let mut params = Vec::new();
    for t in tokens {
        match t.find('=') {
            Some(k) if k > 0 => params.push((t[..k].to_string(), t[k + 1..].to_string())),
            _ => plain.push(t.to_string()),
        }
    }
    (plain, params)
}

/// An element line before the model names are all known: M and Q need them to tell nodes from
/// the model name.
#[derive(Debug)]
pub struct Element {
    pub inst: Instance,
    pub plain: Vec<String>,
}

/// Parse one SPICE element statement (continuations joined, inline comment removed).
/// Returns None for lines that are not drawn elements (`K` couplings) or unknown letters.
pub fn parse_element(body: &str, file: &str, line: u32, sink: &mut Sink) -> Option<Element> {
    let toks: Vec<&str> = body.split_whitespace().collect();
    let name = toks.first()?.to_string();
    let letter = name.chars().next()?.to_ascii_uppercase();
    let (plain, params) = split_params(&toks[1..]);
    let mut inst = Instance {
        name: name.clone(),
        letter: Some(letter),
        params,
        file: file.to_string(),
        line,
        default_terminals: default_terminals(letter),
        ..Instance::default()
    };
    let take = |n: usize| -> Vec<String> { plain.iter().take(n).cloned().collect() };
    let rest = |n: usize| -> Option<String> {
        let r: Vec<&str> = plain.iter().skip(n).map(String::as_str).collect();
        if r.is_empty() {
            None
        } else {
            Some(r.join(" "))
        }
    };
    match letter {
        'X' => {
            let (last, nodes) = plain.split_last()?;
            inst.model = Some(last.clone());
            inst.nodes = nodes.to_vec();
            inst.is_subckt = true;
        }
        // nodes and model are told apart once every `.model` is known (resolve_models)
        'M' | 'Q' => {}
        'D' => {
            inst.nodes = take(2);
            inst.model = plain.get(2).cloned();
        }
        'J' | 'Z' | 'U' => {
            inst.nodes = take(3);
            inst.model = plain.get(3).cloned();
        }
        'O' => {
            inst.nodes = take(4);
            inst.model = plain.get(4).cloned();
        }
        'F' | 'H' => {
            inst.nodes = take(2);
            inst.value = rest(2);
        }
        'W' => {
            inst.nodes = take(2);
            inst.model = plain.get(3).cloned();
        }
        'S' => {
            inst.nodes = take(4);
            inst.model = plain.get(4).cloned();
        }
        'R' | 'C' | 'L' | 'V' | 'I' | 'B' => {
            inst.nodes = take(2);
            inst.value = rest(2);
        }
        'E' | 'G' | 'T' => {
            inst.nodes = take(4);
            inst.value = rest(4);
        }
        'K' => return None,
        _ => {
            sink.warning(
                format!("element {name} is not in the supported SPICE subset and is skipped"),
                Some(Loc::line(file, line)),
            );
            return None;
        }
    }
    Some(Element { inst, plain })
}

/// Tell nodes from the model name for M and Q once all model names are known: the first token
/// after the minimum node count that names a model ends the nodes.
pub fn resolve_nodes(inst: &mut Instance, plain: &[String], models: &HashMap<String, String>) {
    let is_model = |t: &str| models.contains_key(&t.to_lowercase());
    match inst.letter {
        Some('M') => {
            let k = (4..plain.len()).find(|&k| is_model(&plain[k])).or(if plain.len() > 4 { Some(4) } else { None });
            match k {
                Some(k) => {
                    inst.nodes = plain[..k].to_vec();
                    inst.model = Some(plain[k].clone());
                }
                None => inst.nodes = plain.to_vec(),
            }
        }
        Some('Q') => {
            if plain.len() >= 5 && !is_model(&plain[3]) {
                inst.nodes = plain[..4].to_vec();
                inst.model = Some(plain[4].clone());
            } else {
                inst.nodes = plain.iter().take(3).cloned().collect();
                inst.model = plain.get(3).cloned();
            }
        }
        _ => {}
    }
}

/// Ports of a `.subckt` line: the tokens after the name up to the first parameter.
pub fn subckt_ports(tokens: &[&str]) -> Vec<String> {
    let mut ports = Vec::new();
    for t in tokens {
        if t.contains('=') || t.eq_ignore_ascii_case("params:") {
            break;
        }
        ports.push(t.to_string());
    }
    ports
}

struct Pending {
    line: u32,
    text: String,
}

/// Reads a SPICE document.
pub struct Reader<'a> {
    provider: &'a dyn Provider,
    pub sink: &'a mut Sink,
    pub doc: HostDocument,
    models: HashMap<String, String>,
    unresolved: Vec<(LevelId, usize, Vec<String>)>,
    main_dir: String,
}

impl<'a> Reader<'a> {
    fn display(&self, path: &str) -> String {
        let p = normalize(path);
        if !self.main_dir.is_empty() {
            if let Some(rest) = p.strip_prefix(&format!("{}/", self.main_dir)) {
                return rest.to_string();
            }
        }
        p
    }

    #[allow(clippy::too_many_arguments)]
    fn new_level(
        &mut self,
        kind: LevelKind,
        name: &str,
        file: &str,
        path: &str,
        parent: LevelId,
        circuit: bool,
        line: u32,
    ) -> LevelId {
        let mut lv = Level::new(kind, name, file, path);
        lv.circuit = circuit;
        lv.line = line;
        self.doc.add_level(lv, Some(parent))
    }

    /// Read one host file into `level`; elements outside subcircuits go to `circ` (6.1).
    /// `section`: read only this `.lib` section of the file.
    fn read_file(
        &mut self,
        path: &str,
        level: LevelId,
        circ: LevelId,
        main: bool,
        stack: &[String],
        section: Option<&str>,
    ) {
        let Some(text) = self.provider.read(path) else {
            return;
        };
        let fname = self.display(path);
        if !self.doc.sources.iter().any(|(f, _)| *f == fname) {
            self.doc.sources.push((fname.clone(), text.clone()));
        }
        let lines: Vec<&str> = text.lines().collect();
        let mut cur = level;
        let mut circ = circ;
        let mut sub_stack: Vec<(LevelId, LevelId)> = Vec::new();
        let mut run: Vec<CarrierLine> = Vec::new();
        let mut in_control = false;
        let mut ended = false;
        let mut pending: Option<Pending> = None;
        let mut continued = Continued::default();
        // `.lib` sections: inside the selected one when reading a section; skipping an unselected one
        let mut in_section = section.is_none();
        let mut skip_lib: Option<String> = None;

        for (idx, raw) in lines.iter().enumerate() {
            let no = idx as u32 + 1;
            let raw = raw.trim_end_matches('\r');
            let stripped = raw.trim();
            let low = stripped.to_lowercase();
            if no == 1 {
                if raw.contains("@RNL") {
                    self.sink.warning(
                        "the first line of a file is a title and is not read as RNL (2.2)",
                        Some(Loc::line(&fname, 1)),
                    );
                }
                if main || raw.starts_with('*') {
                    continue;
                }
            }
            if ended {
                if raw.contains("@RNL") {
                    self.sink
                        .warning("RNL after `.end` of the main file is not read (2.2)", Some(Loc::line(&fname, no)));
                }
                continue;
            }
            // `.lib` section bookkeeping (2.2: an unselected section is not read)
            if !in_section || skip_lib.is_some() {
                let toks: Vec<&str> = stripped.split_whitespace().collect();
                let is_lib = toks.first().is_some_and(|t| t.eq_ignore_ascii_case(".lib"));
                let is_endl = toks.first().is_some_and(|t| t.eq_ignore_ascii_case(".endl"));
                if let Some(name) = &skip_lib {
                    if is_endl && toks.get(1).is_none_or(|t| t.eq_ignore_ascii_case(name)) {
                        skip_lib = None;
                    } else if raw.contains("@RNL") {
                        self.sink.warning(
                            "RNL in a `.lib` section that is not selected is not read (2.2)",
                            Some(Loc::line(&fname, no)),
                        );
                    }
                    continue;
                }
                if is_lib && toks.len() == 2 && section.is_some_and(|s| s.eq_ignore_ascii_case(toks[1])) {
                    in_section = true;
                } else if raw.contains("@RNL") {
                    self.sink.warning(
                        "RNL outside the selected `.lib` section is not read (2.2)",
                        Some(Loc::line(&fname, no)),
                    );
                }
                continue;
            }
            if in_control {
                if raw.contains("@RNL") {
                    self.sink.warning("RNL inside `.control` is not read (2.2)", Some(Loc::line(&fname, no)));
                }
                if low.starts_with(".endc") {
                    in_control = false;
                }
                continue;
            }
            if let Some(rest) = raw.strip_prefix('*') {
                if rest.trim().is_empty() || rest.starts_with(' ') {
                    if pending.is_some() && continued.after(&lines, idx) {
                        self.sink.warning(
                            "a comment line between a statement and its continuation: read, but hosts treat it differently (2.2)",
                            Some(Loc::line(&fname, no)),
                        );
                    }
                    run.push(CarrierLine { line: no, content: rest.to_string(), col: 1 });
                    continue;
                }
                self.flush_run(&mut run, &fname, cur);
                if raw.contains("@RNL") {
                    self.sink.warning(
                        "no space after the comment marker: this line is not a carrier and its @RNL is not a record entry (2.2)",
                        Some(Loc::line(&fname, no)),
                    );
                }
                continue;
            }
            // not a carrier line: the current run (and any open record) ends here
            self.flush_run(&mut run, &fname, cur);
            if stripped.is_empty() {
                continue;
            }
            if raw.contains("@RNL") && !stripped.starts_with('+') {
                self.sink.warning(
                    "@RNL in an end-of-line comment or a host statement is host text (2.2)",
                    Some(Loc::line(&fname, no)),
                );
            }
            if let Some(cont) = stripped.strip_prefix('+') {
                if let Some(p) = pending.as_mut() {
                    p.text.push(' ');
                    p.text.push_str(cont);
                }
                continue;
            }
            if let Some(p) = pending.take() {
                self.statement(&p.text, p.line, &fname, path, cur, circ, stack);
            }
            let first = low.split_whitespace().next().unwrap_or("");
            if first == ".end" {
                if main {
                    ended = true;
                    self.doc.levels[level].end_line = no;
                }
                continue;
            }
            if first == ".endl" {
                if section.is_some() {
                    in_section = false;
                }
                continue;
            }
            if first == ".control" {
                in_control = true;
                continue;
            }
            if first == ".lib" {
                let toks: Vec<&str> = stripped.split_whitespace().collect();
                if toks.len() == 2 {
                    // a section definition met while reading the file as a whole: not selected
                    skip_lib = Some(toks[1].to_string());
                } else if toks.len() >= 3 {
                    let file = unquote(toks[1]);
                    self.include(&file, Some(toks[2]), no, &fname, path, cur, circ, stack);
                }
                continue;
            }
            if first == ".subckt" {
                let toks: Vec<&str> = stripped.split_whitespace().collect();
                let name = toks.get(1).copied().unwrap_or("?");
                let sub = self.new_level(LevelKind::Subckt, name, &fname, path, cur, true, no);
                self.doc.levels[sub].ports = subckt_ports(&toks[2.min(toks.len())..]);
                let key = self.doc.key(name);
                self.doc.subckts.insert(key, sub);
                sub_stack.push((cur, circ));
                cur = sub;
                circ = sub;
                continue;
            }
            if first == ".ends" {
                self.doc.levels[cur].end_line = no;
                if let Some((c, k)) = sub_stack.pop() {
                    cur = c;
                    circ = k;
                }
                continue;
            }
            pending = Some(Pending { line: no, text: stripped.to_string() });
        }
        self.flush_run(&mut run, &fname, cur);
        if let Some(p) = pending.take() {
            self.statement(&p.text, p.line, &fname, path, cur, circ, stack);
        }
        if !sub_stack.is_empty() {
            self.sink.error("a subcircuit has no `.ends`", Some(Loc::line(&fname, lines.len() as u32)));
        }
        if self.doc.levels[level].end_line == 0 {
            self.doc.levels[level].end_line = lines.len() as u32 + 1;
        }
    }

    fn flush_run(&mut self, run: &mut Vec<CarrierLine>, fname: &str, level: LevelId) {
        if run.is_empty() {
            return;
        }
        let recs = scan_run(run, fname, self.sink);
        self.doc.levels[level].records.extend(recs);
        run.clear();
    }

    #[allow(clippy::too_many_arguments)]
    fn include(
        &mut self,
        target: &str,
        section: Option<&str>,
        no: u32,
        fname: &str,
        path: &str,
        cur: LevelId,
        circ: LevelId,
        stack: &[String],
    ) {
        let full = join(&dirname(path), target);
        let key = format!("{}#{}", full, section.unwrap_or("").to_lowercase());
        if stack.contains(&key) {
            self.sink.error(format!("the include of {target} forms a cycle"), Some(Loc::line(fname, no)));
            return;
        }
        if self.provider.read(&full).is_none() {
            self.sink.error(format!("the included file {target} cannot be found"), Some(Loc::line(fname, no)));
            return;
        }
        let display = self.display(&full);
        let name = match section {
            Some(s) => format!("{} ({s})", file_name(&display)),
            None => file_name(&display).to_string(),
        };
        let child = self.new_level(LevelKind::File, &name, &display, &full, cur, false, no);
        let mut st = stack.to_vec();
        st.push(key);
        self.read_file(&full, child, circ, false, &st, section);
    }

    #[allow(clippy::too_many_arguments)]
    fn statement(
        &mut self,
        text: &str,
        no: u32,
        fname: &str,
        path: &str,
        cur: LevelId,
        circ: LevelId,
        stack: &[String],
    ) {
        let body = strip_inline_comment(text);
        if body.is_empty() {
            return;
        }
        let low = body.to_lowercase();
        let first = low.split_whitespace().next().unwrap_or("");
        if first.starts_with(".inc") {
            if let Some(target) = body.split_whitespace().nth(1) {
                let target = unquote(target);
                self.include(&target, None, no, fname, path, cur, circ, stack);
            }
            return;
        }
        if first == ".model" {
            let toks: Vec<&str> =
                body.split(|c: char| c.is_whitespace() || c == '(').filter(|t| !t.is_empty()).collect();
            if toks.len() >= 3 {
                self.models.insert(toks[1].to_lowercase(), toks[2].to_lowercase());
            }
            return;
        }
        if first == ".global" {
            for t in body.split_whitespace().skip(1) {
                let k = self.doc.net_key(t);
                self.doc.globals.insert(k);
            }
            return;
        }
        if body.starts_with('.') {
            return;
        }
        let Some(el) = parse_element(&body, fname, no, self.sink) else {
            return;
        };
        if !self.doc.levels[circ].circuit {
            return;
        }
        let lv = &mut self.doc.levels[circ];
        if lv.inst(&el.inst.name).is_some() {
            self.sink.error(format!("element {} is defined twice", el.inst.name), Some(Loc::line(fname, no)));
            return;
        }
        let needs = matches!(el.inst.letter, Some('M') | Some('Q'));
        let k = lv.add_instance(el.inst);
        if needs {
            self.unresolved.push((circ, k, el.plain));
        }
    }
}

fn unquote(s: &str) -> String {
    s.trim_matches(|c| c == '\'' || c == '"').to_string()
}

/// The answer of [`Continued::after`] for one run of comment lines: it is the same for all the
/// lines of the run, so each run is looked through once.
#[derive(Default)]
struct Continued {
    /// The answer holds for the lines from `from` up to the one before `next`, the first line after
    /// `from` that is not a comment line.
    from: usize,
    next: usize,
    answer: bool,
}

impl Continued {
    /// Whether the first line after line `idx` that is not a comment line is a continuation line.
    fn after(&mut self, lines: &[&str], idx: usize) -> bool {
        if !(self.from..self.next).contains(&idx) {
            let next = (idx + 1..lines.len()).find(|&k| !lines[k].starts_with('*')).unwrap_or(lines.len());
            let answer = lines.get(next).is_some_and(|s| s.trim().starts_with('+'));
            *self = Continued { from: idx, next, answer };
        }
        self.answer
    }
}

/// Read a SPICE document whose main file is `path`.
pub fn read(path: &str, provider: &dyn Provider, sink: &mut Sink) -> Option<HostDocument> {
    let path = normalize(path);
    provider.read(&path)?;
    let main_dir = dirname(&path);
    let fname = file_name(&path).to_string();
    let mut main = Level::new(LevelKind::File, fname.clone(), fname, path.clone());
    main.circuit = true;
    let doc = HostDocument::new(main, dialect());
    let mut r = Reader { provider, sink, doc, models: HashMap::new(), unresolved: Vec::new(), main_dir };
    r.read_file(&path, 0, 0, true, &[format!("{path}#")], None);
    finish(&mut r.doc, &r.models, std::mem::take(&mut r.unresolved));
    Some(r.doc)
}

/// After every file is read: nodes of M and Q, model types (10.1).
fn finish(doc: &mut HostDocument, models: &HashMap<String, String>, unresolved: Vec<(LevelId, usize, Vec<String>)>) {
    for (lv, k, plain) in unresolved {
        resolve_nodes(&mut doc.levels[lv].instances[k], &plain, models);
    }
    for lv in &mut doc.levels {
        for inst in &mut lv.instances {
            if let (Some(m), false) = (&inst.model, inst.is_subckt) {
                inst.mtype = models.get(&m.to_lowercase()).cloned();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnl_runtime::source::MemoryProvider;

    #[test]
    fn levels_records_and_instances() {
        let net = "* title\n.include lib.inc\n.model nch nmos level=1\n.subckt amp in out\nM1 out in 0 0 nch w=1u\n* @RNL {#M1 @PLACE(0, 0)}\n.ends\nX1 a b amp\nQ1 c b e npn1\nQ2 c b e s npn1\n.model npn1 npn\nR1 a 0 1k ; load\n+ tc=0\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n*\n* @RNL {\n*   #R1 @PLACE(1, 2)\n* }\n.end\n* @RNL {#X1}\n";
        let lib = "* lib\n.subckt rdiv top bot\nR1 top bot 1k\n.ends\n";
        let p = MemoryProvider::new().with("w/top.sp", net).with("w/lib.inc", lib);
        let mut sink = Sink::new();
        let doc = read("w/top.sp", &p, &mut sink).unwrap();
        let titles: Vec<String> = doc.levels.iter().map(|l| l.title()).collect();
        assert_eq!(titles, vec!["top.sp", "lib.inc", "rdiv", "amp"]);
        let main = doc.main();
        assert_eq!(main.records.len(), 2);
        assert_eq!(main.records[1].loc(0), Loc::new("top.sp", 16, 3));
        let names: Vec<&str> = main.instances.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["X1", "Q1", "Q2", "R1"]);
        assert_eq!(main.instances[1].nodes, vec!["c", "b", "e"]);
        assert_eq!(main.instances[2].nodes, vec!["c", "b", "e", "s"]);
        assert_eq!(main.instances[2].mtype.as_deref(), Some("npn"));
        assert_eq!(main.instances[3].value.as_deref(), Some("1k"));
        let amp = &doc.levels[3];
        assert_eq!(amp.ports, vec!["in", "out"]);
        assert_eq!(amp.records.len(), 1);
        assert_eq!(amp.instances[0].mtype.as_deref(), Some("nmos"));
        assert_eq!(sink.all().len(), 1, "{:?}", sink.all()); // RNL after .end
    }

    #[test]
    fn lib_sections() {
        let net = "* t\n.lib 'models.lib' tt\nM1 d g 0 0 nch\n.end\n";
        let lib = "* models\n.lib ff\n* @RNL {#X}\n.model nch nmos\n.endl ff\n.lib tt\n.model nch nmos\n.subckt cell a\n.ends\n.endl tt\n";
        let p = MemoryProvider::new().with("top.sp", net).with("models.lib", lib);
        let mut sink = Sink::new();
        let doc = read("top.sp", &p, &mut sink).unwrap();
        assert_eq!(doc.levels[1].name, "models.lib (tt)");
        assert!(doc.subckt("CELL").is_some());
        assert_eq!(sink.all().len(), 1);
        assert_eq!(doc.main().instances[0].mtype.as_deref(), Some("nmos"));
    }

    #[test]
    fn inline_comments() {
        assert_eq!(strip_inline_comment("R1 a b 1k ; x"), "R1 a b 1k");
        assert_eq!(strip_inline_comment("R1 a b 1k$x"), "R1 a b 1k$x");
        assert_eq!(strip_inline_comment("R1 a b 1k $ x"), "R1 a b 1k");
    }
}
