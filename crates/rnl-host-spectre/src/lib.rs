//! Spectre host adapter (standard 2.1–2.6, 6.3, 6.11, 6.12).
//!
//! Reads a Spectre netlist into the runtime's dialect-neutral host model. It covers the
//! structural subset: instances (with or without parentheses), nets, `subckt` and `inline subckt`
//! with their ports, `include` (with `section=`) and library sections, `model` cards and their
//! types, `global`, and the switch between the Spectre and SPICE language modes. It does not
//! evaluate parameters or interpret analyses.
//!
//! Carriers (2.2): in Spectre mode a whole-line comment `//` or `*` in column 1 followed by a
//! space, or a bare one; in SPICE mode `*` as in the SPICE adapter. The first line of the main
//! file is a title and is never read as RNL; the first line of any file never carries RNL.
//!
//! Names compare by case (3.6). A backslash escapes the next character of a name, so
//! `D\<3\>` is read as `D<3>` (6.11). Only `0` is ground.

use std::collections::{HashMap, HashSet};

use rnl_runtime::diag::{Loc, Sink};
use rnl_runtime::host::{file_name, Dialect, HostDocument, Instance, Level, LevelId, LevelKind};
use rnl_runtime::records::{scan_run, CarrierLine};
use rnl_runtime::source::{dirname, join, normalize, relative_to, Provider};

/// A record in a region written to be encrypted (2.2): Spectre's `protect … unprotect`, and
/// `.prot … .unprot` in SPICE mode.
const PROTECTED: &str =
    "RNL between `protect` and `unprotect` is read, but it is gone once the region is encrypted (2.2)";
const PROTECTED_SPICE: &str =
    "RNL between `.prot` and `.unprot` is read, but it is gone once the region is encrypted (2.2)";

/// The Spectre dialect: names compare by case; `0` is ground.
pub fn dialect() -> Dialect {
    Dialect { name: "spectre".into(), case_sensitive: true, ground_aliases: Vec::new() }
}

/// Built-in primitives: (master, default terminal names, the parameter shown as `${value}`). The
/// `switch` is not among them: its terminals depend on its nodes ([`SWITCH`]).
const PRIMITIVES: [(&str, &[&str], Option<&str>); 14] = [
    ("resistor", &["p", "n"], Some("r")),
    ("capacitor", &["p", "n"], Some("c")),
    ("inductor", &["p", "n"], Some("l")),
    ("vsource", &["p", "n"], Some("dc")),
    ("isource", &["p", "n"], Some("dc")),
    ("diode", &["a", "k"], None),
    ("vcvs", &["p", "n", "cp", "cn"], Some("gain")),
    ("vccs", &["p", "n", "cp", "cn"], Some("gm")),
    ("ccvs", &["p", "n"], Some("rm")),
    ("cccs", &["p", "n"], Some("gain")),
    ("bsource", &["p", "n"], None),
    // a switch between p and n, controlled by the voltage between cp and cn, as S of SPICE
    ("relay", &["p", "n", "cp", "cn"], None),
    // its first winding between p_plus and p_minus, its second between s_plus and s_minus
    ("transformer", &["p_plus", "p_minus", "s_plus", "s_minus"], None),
    ("port", &["p", "n"], Some("r")),
];

/// The master of Spectre's switch, `Name (t0 t1 … tn) switch`: a common terminal, t0, and its
/// throws, t1 to tn. It has as many terminals as nodes, named as Spectre numbers them, so that its
/// `position` is the number of a terminal.
const SWITCH: &str = "switch";

/// Device families of model masters (6.12): the default terminal names and the model type.
fn family(master: &str) -> Option<&'static str> {
    let m = master.to_lowercase();
    let mos = [
        "bsim1",
        "bsim2",
        "bsim3",
        "bsim3v3",
        "bsim4",
        "bsim6",
        "bsimbulk",
        "bsimsoi",
        "bsimcmg",
        "bsimimg",
        "b3soipd",
        "mos0",
        "mos1",
        "mos2",
        "mos3",
        "mos9",
        "mos11",
        "mos705",
        "mos902",
        "mos903",
        "psp102",
        "psp103",
        "psp104",
        "pspnqs102",
        "pspnqs103",
        "ekv",
        "hisim",
        "hisim2",
        "hisimhv",
        "hvmos",
        "bta_soi",
        "ucsd_hvmos",
    ];
    let bjt = [
        "bjt", "bjt500", "bjt503", "bjt504", "bjt504t", "bjt3500", "bjt301", "vbic", "hicum", "hicum0", "hicum2",
        "mextram",
    ];
    if mos.contains(&m.as_str()) {
        Some("mos")
    } else if bjt.contains(&m.as_str()) {
        Some("bjt")
    } else if m == "diode" || m == "juncap" || m == "juncap200" {
        Some("diode")
    } else if m == "jfet" {
        Some("jfet")
    } else if m == "resistor" || m == "rdiff" || m == "r3" {
        Some("resistor")
    } else if m == "capacitor" {
        Some("capacitor")
    } else if m == "inductor" {
        Some("inductor")
    } else if m == "relay" {
        Some("relay")
    } else {
        None
    }
}

fn family_terminals(f: &str) -> Option<Vec<String>> {
    let t: &[&str] = match f {
        "mos" => &["d", "g", "s", "b"],
        "bjt" => &["c", "b", "e", "s"],
        "diode" => &["a", "k"],
        "jfet" => &["d", "g", "s"],
        "resistor" | "capacitor" | "inductor" => &["p", "n"],
        "relay" => &["p", "n", "cp", "cn"],
        _ => return None,
    };
    Some(t.iter().map(|s| s.to_string()).collect())
}

/// The model type in the words SPICE uses (10.1): nmos, pmos, npn, pnp, d, r, c, l, sw.
fn family_type(f: &str, params: &[(String, String)]) -> Option<String> {
    let ty = params.iter().find(|(k, _)| k.eq_ignore_ascii_case("type")).map(|(_, v)| v.to_lowercase());
    Some(match f {
        "mos" => match ty.as_deref() {
            Some("p") | Some("pmos") => "pmos".into(),
            _ => "nmos".into(),
        },
        "bjt" => match ty.as_deref() {
            Some("pnp") => "pnp".into(),
            _ => "npn".into(),
        },
        "jfet" => match ty.as_deref() {
            Some("p") | Some("pjf") => "pjf".into(),
            _ => "njf".into(),
        },
        "diode" => "d".into(),
        "resistor" => "r".into(),
        "capacitor" => "c".into(),
        "inductor" => "l".into(),
        // a relay, a switch controlled by a voltage: S of SPICE, whose model type is sw
        "relay" => "sw".into(),
        _ => return None,
    })
}

/// Statements whose second word names an analysis or control: not instances.
const ANALYSES: [&str; 44] = [
    "ac",
    "dc",
    "tran",
    "noise",
    "pss",
    "pac",
    "pnoise",
    "pxf",
    "psp",
    "pstb",
    "qpss",
    "qpac",
    "qpnoise",
    "qpxf",
    "qpsp",
    "hb",
    "hbac",
    "hbnoise",
    "hbsp",
    "hbxf",
    "hbstb",
    "sp",
    "stb",
    "xf",
    "info",
    "options",
    "set",
    "shell",
    "alter",
    "altergroup",
    "check",
    "checklimit",
    "montecarlo",
    "sweep",
    "envlp",
    "dcmatch",
    "reliability",
    "sens",
    "fourier",
    "loadpull",
    "pz",
    "sprobe",
    "assert",
    "tdr",
];

/// Statements that are not instances, by their first word.
const DIRECTIVES: [&str; 14] = [
    "parameters",
    "statistics",
    "real",
    "ic",
    "nodeset",
    "save",
    "paramset",
    "ahdl_include",
    "hdl_include",
    "pspice_include",
    "dspf_include",
    "export",
    "function",
    "if",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Spectre,
    Spice,
}

/// Split a Spectre statement into tokens: parentheses are tokens of their own, quoted strings stay
/// whole, a backslash escapes the next character.
fn tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    let push = |cur: &mut String, out: &mut Vec<String>| {
        if !cur.is_empty() {
            out.push(std::mem::take(cur));
        }
    };
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            '"' => {
                cur.push('"');
                for n in chars.by_ref() {
                    cur.push(n);
                    if n == '"' {
                        break;
                    }
                }
            }
            '(' | ')' => {
                push(&mut cur, &mut out);
                out.push(c.to_string());
            }
            c if c.is_whitespace() => push(&mut cur, &mut out),
            _ => cur.push(c),
        }
    }
    push(&mut cur, &mut out);
    out
}

/// Remove an end-of-line comment: `//` after whitespace.
fn strip_comment(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut in_str = false;
    for k in 0..chars.len() {
        if chars[k] == '"' {
            in_str = !in_str;
        }
        if !in_str && chars[k] == '/' && chars.get(k + 1) == Some(&'/') && (k == 0 || chars[k - 1].is_whitespace()) {
            return chars[..k].iter().collect::<String>().trim().to_string();
        }
    }
    s.trim().to_string()
}

fn unquote(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '\'').to_string()
}

struct Model {
    master: String,
    params: Vec<(String, String)>,
}

struct Reader<'a> {
    provider: &'a dyn Provider,
    sink: &'a mut Sink,
    doc: HostDocument,
    models: HashMap<String, Model>,
    spice_models: HashMap<String, String>,
    /// The [`Level::name_key`]s of each level's instances, to find a name defined twice without
    /// looking through the level's instances.
    names: HashMap<LevelId, HashSet<String>>,
    pending_spice: Vec<(LevelId, usize, Vec<String>)>,
    main_dir: String,
}

struct State {
    mode: Mode,
    cur: LevelId,
    circ: LevelId,
    stack: Vec<(LevelId, LevelId, Mode)>,
}

impl<'a> Reader<'a> {
    /// A file's name in the document: relative to the main file's directory when it is in it or
    /// below, the root included, else its path.
    fn display(&self, path: &str) -> String {
        let p = normalize(path);
        match relative_to(&p, &self.main_dir) {
            Some(rest) => rest.to_string(),
            None => p,
        }
    }

    /// Read one file. `section`: read only this library section.
    #[allow(clippy::too_many_arguments)]
    fn read_file(
        &mut self,
        path: &str,
        level: LevelId,
        circ: LevelId,
        main: bool,
        mode: Mode,
        stack: &[String],
        section: Option<&str>,
    ) {
        let Some(text) = self.provider.read(path) else { return };
        let fname = self.display(path);
        if !self.doc.sources.iter().any(|(f, _)| *f == fname) {
            self.doc.sources.push((fname.clone(), text.clone()));
        }
        let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
        let mut st = State { mode, cur: level, circ, stack: Vec::new() };
        let mut run: Vec<CarrierLine> = Vec::new();
        let mut pending: Option<(u32, String)> = None;
        let mut in_section = section.is_none();
        let mut skip_section: Option<String> = None;
        // a region written to be encrypted, while in it: the line that opens it, and the warning
        // for a record there (2.2)
        let mut protect: Option<(u32, &str)> = None;
        // a run of lines read in SPICE mode, while in it: the line after which it began, 0 from
        // the start of the file (`HostDocument::spice_mode`)
        let mut spice = (mode == Mode::Spice).then_some(0);
        let n = lines.len();
        let mut idx = 0usize;
        while idx < n {
            spice_mode(&mut spice, st.mode, idx as u32, &fname, &mut self.doc.spice_mode);
            let raw = lines[idx];
            let no = idx as u32 + 1;
            idx += 1;
            let stripped = raw.trim();
            if no == 1 {
                if raw.contains("@RNL") {
                    self.sink.warning("the first line of a file is not read as RNL (2.2)", Some(Loc::line(&fname, 1)));
                }
                // the main file's first line is a title; an included file has none (S1 p51, p89)
                if main || raw.starts_with('*') || raw.starts_with("//") {
                    continue;
                }
            }
            // library sections (Spectre `section`, SPICE `.lib`)
            if section.is_some() || skip_section.is_some() {
                let low = stripped.to_lowercase();
                let words: Vec<&str> = low.split_whitespace().collect();
                let first = words.first().copied().unwrap_or("");
                if let Some(name) = &skip_section {
                    if (first == "endsection" || first == ".endl")
                        && words.get(1).is_none_or(|w| w.eq_ignore_ascii_case(name))
                    {
                        skip_section = None;
                    } else if raw.contains("@RNL") {
                        self.sink.warning(
                            "RNL in a library section that is not selected is not read (2.2)",
                            Some(Loc::line(&fname, no)),
                        );
                    }
                    continue;
                }
                if !in_section {
                    let opens = (first == "section" || first == ".lib") && words.len() == 2;
                    if opens && section.is_some_and(|s| s.eq_ignore_ascii_case(words[1])) {
                        in_section = true;
                        // each section sets its own language; it starts in SPICE mode (S1 p498)
                        st.mode = if first == "section" { Mode::Spectre } else { Mode::Spice };
                    } else if raw.contains("@RNL") {
                        self.sink.warning(
                            "RNL outside the selected library section is not read (2.2)",
                            Some(Loc::line(&fname, no)),
                        );
                    }
                    continue;
                }
            }
            // carriers
            let carrier_rest = match st.mode {
                Mode::Spectre => {
                    raw.strip_prefix("//").map(|r| (r, 2)).or_else(|| raw.strip_prefix('*').map(|r| (r, 1)))
                }
                Mode::Spice => raw.strip_prefix('*').map(|r| (r, 1)),
            };
            if let Some((rest, col)) = carrier_rest {
                if rest.trim().is_empty() || rest.starts_with(' ') {
                    if let Some((_, warning)) = protect.filter(|_| rest.contains("@RNL")) {
                        self.sink.warning(warning, Some(Loc::line(&fname, no)));
                    }
                    run.push(CarrierLine { line: no, content: rest.to_string(), col });
                    continue;
                }
                self.flush(&mut run, &fname, st.cur);
                if raw.contains("@RNL") {
                    self.sink.warning(
                        "no space after the comment marker: this line is not a carrier and its @RNL is not a record entry (2.2)",
                        Some(Loc::line(&fname, no)),
                    );
                }
                continue;
            }
            self.flush(&mut run, &fname, st.cur);
            if stripped.is_empty() {
                continue;
            }
            if raw.contains("@RNL") {
                self.sink.warning(
                    "@RNL in an end-of-line comment or a host statement is host text (2.2)",
                    Some(Loc::line(&fname, no)),
                );
            }
            if st.mode == Mode::Spice {
                if let Some(cont) = stripped.strip_prefix('+') {
                    if let Some(p) = pending.as_mut() {
                        p.1.push(' ');
                        p.1.push_str(cont);
                    }
                    continue;
                }
                if let Some((pl, pt)) = pending.take() {
                    self.spice_statement(&pt, pl, &fname, path, &mut st, stack);
                }
                let low = stripped.to_lowercase();
                let first = low.split_whitespace().next().unwrap_or("");
                match first {
                    ".end" => {
                        if main {
                            self.doc.levels[level].end_line = no;
                            break;
                        }
                        continue;
                    }
                    ".endl" => {
                        if section.is_some() {
                            in_section = false;
                        }
                        continue;
                    }
                    ".lib" => {
                        let toks: Vec<&str> = stripped.split_whitespace().collect();
                        if toks.len() == 2 {
                            skip_section = Some(toks[1].to_string());
                        } else if toks.len() >= 3 {
                            self.include(&unquote(toks[1]), Some(toks[2]), no, &fname, path, &st, stack, Mode::Spice);
                        }
                        continue;
                    }
                    ".subckt" => {
                        let toks: Vec<&str> = stripped.split_whitespace().collect();
                        let name = toks.get(1).copied().unwrap_or("?");
                        let ports = rnl_host_spice::subckt_ports(&toks[2.min(toks.len())..]);
                        self.open_subckt(name, ports, &fname, path, no, &mut st, false);
                        continue;
                    }
                    ".ends" => {
                        self.doc.levels[st.cur].end_line = no;
                        self.close_subckt(&mut st);
                        continue;
                    }
                    "simulator" => {
                        if low.contains("spectre") {
                            st.mode = Mode::Spectre;
                        }
                        continue;
                    }
                    ".prot" | ".protect" => {
                        protect.get_or_insert((no, PROTECTED_SPICE));
                        continue;
                    }
                    ".unprot" | ".unprotect" => {
                        if let Some((start, _)) = protect.take() {
                            self.doc.protected.push((fname.clone(), start, no));
                        }
                        continue;
                    }
                    _ => {}
                }
                pending = Some((no, stripped.to_string()));
                continue;
            }
            // Spectre mode: join `\` and `+` continuations
            let mut stmt = strip_comment(stripped);
            let mut last = no;
            loop {
                if stmt.ends_with('\\') {
                    stmt.pop();
                    if idx < n {
                        stmt.push(' ');
                        stmt.push_str(&strip_comment(lines[idx].trim()));
                        last = idx as u32 + 1;
                        idx += 1;
                        continue;
                    }
                }
                if idx < n && lines[idx].trim_start().starts_with('+') {
                    stmt.push(' ');
                    stmt.push_str(&strip_comment(lines[idx].trim_start()[1..].trim()));
                    last = idx as u32 + 1;
                    idx += 1;
                    continue;
                }
                break;
            }
            let _ = last;
            match stmt.split_whitespace().next() {
                Some("protect") => {
                    protect.get_or_insert((no, PROTECTED));
                    continue;
                }
                Some("unprotect") => {
                    if let Some((start, _)) = protect.take() {
                        self.doc.protected.push((fname.clone(), start, no));
                    }
                    continue;
                }
                _ => {}
            }
            if self.spectre_statement(&stmt, no, &fname, path, &mut st, stack, section) == Some(true) {
                in_section = false;
            }
        }
        self.flush(&mut run, &fname, st.cur);
        if let Some((pl, pt)) = pending.take() {
            self.spice_statement(&pt, pl, &fname, path, &mut st, stack);
        }
        if let Some((start, _)) = protect {
            self.doc.protected.push((fname.clone(), start, n as u32 + 1));
        }
        spice_mode(&mut spice, st.mode, idx as u32, &fname, &mut self.doc.spice_mode);
        if let Some(from) = spice {
            self.doc.spice_mode.push((fname.clone(), from, n as u32 + 1));
        }
        if !st.stack.is_empty() {
            self.sink.error("a subcircuit has no `ends`", Some(Loc::line(&fname, n as u32)));
        }
        if self.doc.levels[level].end_line == 0 {
            self.doc.levels[level].end_line = n as u32 + 1;
        }
    }

    fn flush(&mut self, run: &mut Vec<CarrierLine>, fname: &str, level: LevelId) {
        if run.is_empty() {
            return;
        }
        let recs = scan_run(run, fname, self.sink);
        self.doc.levels[level].records.extend(recs);
        run.clear();
    }

    #[allow(clippy::too_many_arguments)]
    fn open_subckt(
        &mut self,
        name: &str,
        ports: Vec<String>,
        fname: &str,
        path: &str,
        no: u32,
        st: &mut State,
        spectre: bool,
    ) {
        let mut lv = Level::new(LevelKind::Subckt, name, fname, path);
        lv.circuit = true;
        lv.line = no;
        lv.case_sensitive = spectre || self.doc.dialect.case_sensitive;
        lv.ports = ports;
        let id = self.doc.add_level(lv, Some(st.cur));
        let key = self.doc.key(name);
        self.doc.subckts.insert(key, id);
        st.stack.push((st.cur, st.circ, st.mode));
        st.cur = id;
        st.circ = id;
    }

    fn close_subckt(&mut self, st: &mut State) {
        if let Some((c, k, _m)) = st.stack.pop() {
            st.cur = c;
            st.circ = k;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn include(
        &mut self,
        target: &str,
        section: Option<&str>,
        no: u32,
        fname: &str,
        path: &str,
        st: &State,
        stack: &[String],
        parent_mode: Mode,
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
        let mode = start_mode(&full);
        let _ = parent_mode; // the mode comes back after the include (S1 p89)
        let display = self.display(&full);
        let name = match section {
            Some(s) => format!("{} ({s})", file_name(&display)),
            None => file_name(&display).to_string(),
        };
        let mut lv = Level::new(LevelKind::File, &name, &display, &full);
        lv.line = no;
        lv.section = section.map(String::from);
        let child = self.doc.add_level(lv, Some(st.cur));
        let mut stck = stack.to_vec();
        stck.push(key);
        self.read_file(&full, child, st.circ, false, mode, &stck, section);
    }

    /// One Spectre statement; returns Some(true) at the end of the selected section.
    #[allow(clippy::too_many_arguments)]
    fn spectre_statement(
        &mut self,
        stmt: &str,
        no: u32,
        fname: &str,
        path: &str,
        st: &mut State,
        stack: &[String],
        section: Option<&str>,
    ) -> Option<bool> {
        let toks = tokens(stmt);
        let first = toks.first()?.clone();
        let low = first.to_lowercase();
        match low.as_str() {
            "simulator" => {
                if stmt.to_lowercase().contains("spice") && !stmt.to_lowercase().contains("spectre") {
                    st.mode = Mode::Spice;
                }
                return None;
            }
            "include" => {
                let target = toks.get(1).map(|t| unquote(t))?;
                let sec = toks.iter().skip(2).find_map(|t| t.strip_prefix("section=").map(String::from));
                self.include(&target, sec.as_deref(), no, fname, path, st, stack, Mode::Spectre);
                return None;
            }
            "library" | "endlibrary" => return None,
            "section" => return None,
            "endsection" => return Some(section.is_some()),
            "global" => {
                for t in toks.iter().skip(1) {
                    let k = self.doc.net_key(t);
                    self.doc.globals.insert(k);
                }
                return None;
            }
            "model" => {
                if toks.len() >= 3 {
                    let name = toks[1].clone();
                    let master = toks[2].clone();
                    let params = toks[3..]
                        .iter()
                        .filter_map(|t| t.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
                        .collect();
                    // binned models `nch.1`, `nch.2` stand for `nch`
                    let base = match name.rsplit_once('.') {
                        Some((b, k)) if k.chars().all(|c| c.is_ascii_digit()) => b.to_string(),
                        _ => name.clone(),
                    };
                    self.models.entry(base).or_insert(Model { master, params });
                }
                return None;
            }
            "subckt" | "inline" => {
                let rest: Vec<&String> = toks.iter().skip(if low == "inline" { 2 } else { 1 }).collect();
                let name = rest.first().map(|s| s.as_str()).unwrap_or("?").to_string();
                let ports: Vec<String> = rest
                    .iter()
                    .skip(1)
                    .filter(|t| t.as_str() != "(" && t.as_str() != ")" && !t.contains('='))
                    .map(|t| t.to_string())
                    .collect();
                self.open_subckt(&name, ports, fname, path, no, st, true);
                return None;
            }
            "ends" => {
                self.doc.levels[st.cur].end_line = no;
                self.close_subckt(st);
                return None;
            }
            _ => {}
        }
        if DIRECTIVES.contains(&low.as_str()) || first.starts_with('}') || first.starts_with('{') {
            return None;
        }
        // an instance: `name (nodes) master params` or `name nodes master params`
        let name = first;
        let (nodes, master, rest): (Vec<String>, String, Vec<String>) = if toks.get(1).map(|s| s.as_str()) == Some("(")
        {
            let close = toks.iter().position(|t| t == ")")?;
            let nodes = toks[2..close].to_vec();
            let master = toks.get(close + 1)?.clone();
            (nodes, master, toks[close + 2.min(toks.len() - close - 1)..].to_vec())
        } else {
            let k = toks.iter().position(|t| t.contains('=')).unwrap_or(toks.len());
            if k < 2 {
                return None;
            }
            let master = toks[k - 1].clone();
            (toks[1..k - 1].to_vec(), master, toks[k..].to_vec())
        };
        if ANALYSES.contains(&master.to_lowercase().as_str()) {
            return None;
        }
        let params: Vec<(String, String)> =
            rest.iter().filter_map(|t| t.split_once('=').map(|(k, v)| (k.to_string(), v.to_string()))).collect();
        let inst = Instance {
            name: name.clone(),
            letter: None,
            nodes,
            model: Some(master),
            value: None,
            params,
            file: fname.to_string(),
            line: no,
            case_sensitive: true,
            ..Instance::default()
        };
        self.add_instance(inst, st.circ, fname, no, None);
        None
    }

    fn spice_statement(&mut self, text: &str, no: u32, fname: &str, path: &str, st: &mut State, stack: &[String]) {
        let body = rnl_host_spice::strip_inline_comment(text);
        if body.is_empty() {
            return;
        }
        let low = body.to_lowercase();
        let first = low.split_whitespace().next().unwrap_or("");
        if first.starts_with(".inc") {
            if let Some(t) = body.split_whitespace().nth(1) {
                self.include(&unquote(t), None, no, fname, path, st, stack, Mode::Spice);
            }
            return;
        }
        if first == ".model" {
            let toks: Vec<&str> =
                body.split(|c: char| c.is_whitespace() || c == '(').filter(|t| !t.is_empty()).collect();
            if toks.len() >= 3 {
                self.spice_models.insert(toks[1].to_lowercase(), toks[2].to_lowercase());
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
        let Some(el) = rnl_host_spice::parse_element(&body, fname, no, self.sink) else { return };
        let needs = matches!(el.inst.letter, Some('M') | Some('Q'));
        self.add_instance(el.inst, st.circ, fname, no, if needs { Some(el.plain) } else { None });
    }

    fn add_instance(&mut self, mut inst: Instance, circ: LevelId, fname: &str, no: u32, plain: Option<Vec<String>>) {
        if !self.doc.levels[circ].circuit {
            return;
        }
        let lv = &mut self.doc.levels[circ];
        if !self.names.entry(circ).or_default().insert(lv.name_key(&inst.name)) {
            self.sink.error(format!("instance {} is defined twice", inst.name), Some(Loc::line(fname, no)));
            return;
        }
        inst.order = lv.instances.len();
        lv.instances.push(inst);
        if let Some(p) = plain {
            let k = lv.instances.len() - 1;
            self.pending_spice.push((circ, k, p));
        }
    }

    /// After every file is read: what each master is (6.12, 10.1, 13.6).
    fn finish(&mut self) {
        let spice_models: HashMap<String, String> = self.spice_models.clone();
        for (lv, k, plain) in std::mem::take(&mut self.pending_spice) {
            rnl_host_spice::resolve_nodes(&mut self.doc.levels[lv].instances[k], &plain, &spice_models);
        }
        for lv in 0..self.doc.levels.len() {
            for k in 0..self.doc.levels[lv].instances.len() {
                let inst = &self.doc.levels[lv].instances[k];
                if inst.letter.is_some() {
                    // an element read in SPICE mode
                    let (model, is_x) = (inst.model.clone(), inst.is_subckt);
                    let inst = &mut self.doc.levels[lv].instances[k];
                    if let (Some(m), false) = (model, is_x) {
                        inst.mtype = spice_models.get(&m.to_lowercase()).cloned();
                    }
                    continue;
                }
                let master = inst.model.clone().unwrap_or_default();
                let (terms, value_param, mtype, is_subckt) = if master == SWITCH {
                    (Some((0..inst.nodes.len()).map(|k| format!("t{k}")).collect()), None, None, false)
                } else if let Some((_, t, v)) = PRIMITIVES.iter().find(|(m, _, _)| *m == master) {
                    (Some(t.iter().map(|s| s.to_string()).collect::<Vec<_>>()), v.map(String::from), None, false)
                } else if let Some(m) = self.models.get(&master) {
                    match family(&m.master) {
                        Some(f) => (family_terminals(f), None, family_type(f, &m.params), false),
                        None => (None, None, None, false),
                    }
                } else if self.doc.subckts.contains_key(&master) {
                    (None, None, None, true)
                } else {
                    // neither a primitive nor a model card: a subcircuit defined elsewhere, such as a PDK device (6.4)
                    (None, None, None, true)
                };
                let inst = &mut self.doc.levels[lv].instances[k];
                inst.default_terminals = terms;
                inst.mtype = mtype;
                inst.is_subckt = is_subckt;
                if let Some(vp) = value_param {
                    inst.value = inst.params.iter().find(|(p, _)| *p == vp).map(|(_, v)| v.clone());
                }
            }
        }
    }
}

/// The language mode a file starts in (2.2): Spectre for a `.scs` file, SPICE for any other, up
/// to its `simulator lang=spectre`. Each `simulator lang=` line switches the mode of the lines
/// after it, wherever it stands.
fn start_mode(path: &str) -> Mode {
    if path.to_lowercase().ends_with(".scs") {
        Mode::Spectre
    } else {
        Mode::Spice
    }
}

/// Follow the mode of a reading of `file` after its line `after` (`HostDocument::spice_mode`):
/// `spice` holds the line after which the run of lines read in SPICE mode began, while in one; a
/// run that a line ends, the switch to Spectre mode, goes into `regions` with that line.
fn spice_mode(spice: &mut Option<u32>, mode: Mode, after: u32, file: &str, regions: &mut Vec<(String, u32, u32)>) {
    match (mode, *spice) {
        (Mode::Spice, None) => *spice = Some(after),
        (Mode::Spectre, Some(from)) => {
            regions.push((file.to_string(), from, after));
            *spice = None;
        }
        _ => {}
    }
}

/// Read a Spectre document whose main file is `path`.
pub fn read(path: &str, provider: &dyn Provider, sink: &mut Sink) -> Option<HostDocument> {
    let path = normalize(path);
    provider.read(&path)?;
    let main_dir = dirname(&path);
    let fname = file_name(&path).to_string();
    let mut main = Level::new(LevelKind::File, fname.clone(), fname, path.clone());
    main.circuit = true;
    main.case_sensitive = true;
    let doc = HostDocument::new(main, dialect());
    let mode = start_mode(&path);
    let mut r = Reader {
        provider,
        sink,
        doc,
        models: HashMap::new(),
        spice_models: HashMap::new(),
        names: HashMap::new(),
        pending_spice: Vec::new(),
        main_dir,
    };
    r.read_file(&path, 0, 0, true, mode, &[format!("{path}#")], None);
    r.finish();
    Some(r.doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnl_runtime::source::MemoryProvider;

    #[test]
    fn instances_models_and_subckts() {
        let net = "// title\nsimulator lang=spectre\nglobal 0 vdd!\nmodel nch bsim4 type=n\nmodel pch bsim4 type=p\nsubckt inv (a y vdd vss)\n  MP (y a vdd vdd) pch w=2u\n  MN (y a vss vss) nch w=1u\n// @RNL {#MN @PLACE(0, 0)}\nends inv\nX1 (in out vdd! 0) inv\nR1 (out 0) resistor r=1k\nV1 vdd! 0 vsource dc=1.8\nD\\<3\\> (a b) diode\ntran1 tran stop=1n\nC1 (out 0) capacitor \\\n  c=1p\n// @RNL {\n//   #R1 @PLACE(1, 2)\n// }\n";
        let p = MemoryProvider::new().with("a.scs", net);
        let mut sink = Sink::new();
        let doc = read("a.scs", &p, &mut sink).unwrap();
        assert!(sink.all().is_empty(), "{:?}", sink.all());
        let main = doc.main();
        let names: Vec<&str> = main.instances.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["X1", "R1", "V1", "D<3>", "C1"]);
        assert!(main.instances[0].is_subckt);
        assert_eq!(main.instances[1].value.as_deref(), Some("1k"));
        assert_eq!(main.instances[1].default_terminals, Some(vec!["p".into(), "n".into()]));
        assert_eq!(main.instances[2].nodes, vec!["vdd!", "0"]);
        assert_eq!(main.instances[4].value.as_deref(), Some("1p"));
        assert_eq!(main.records.len(), 1);
        let inv = &doc.levels[1];
        assert_eq!(inv.ports, vec!["a", "y", "vdd", "vss"]);
        assert_eq!(inv.instances[0].mtype.as_deref(), Some("pmos"));
        assert_eq!(inv.instances[1].default_terminals.as_ref().unwrap()[0], "d");
        assert_eq!(inv.records.len(), 1);
        assert!(doc.globals.contains("vdd!"));
    }

    #[test]
    fn switches_relays_and_transformers() {
        // a switch has a terminal for each node, t0 its common one; a relay is S of SPICE, as a
        // primitive or a model card of type sw; a transformer has two windings
        let net = "// t\nsimulator lang=spectre\nmodel rl relay ropen=1G\nS1 (a b) switch\n\
                   S2 (a b c d) switch position=2\nK1 (a b c d) relay\nK2 (a b c d) rl\n\
                   T1 (a b c d) transformer n1=1 n2=2\n";
        let p = MemoryProvider::new().with("a.scs", net);
        let mut sink = Sink::new();
        let doc = read("a.scs", &p, &mut sink).unwrap();
        assert!(sink.all().is_empty(), "{:?}", sink.all());
        let seen: Vec<(Vec<String>, Option<&str>)> =
            doc.main().instances.iter().map(|i| (i.default_terminals.clone().unwrap(), i.mtype.as_deref())).collect();
        let t = |n: &[&str]| n.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            seen,
            [
                (t(&["t0", "t1"]), None),
                (t(&["t0", "t1", "t2", "t3"]), None),
                (t(&["p", "n", "cp", "cn"]), None),
                (t(&["p", "n", "cp", "cn"]), Some("sw")),
                (t(&["p_plus", "p_minus", "s_plus", "s_minus"]), None),
            ]
        );
        assert!(doc.main().instances.iter().all(|i| !i.is_subckt));
    }

    #[test]
    fn spice_mode_and_sections() {
        let net = "// t\nsimulator lang=spectre\ninclude \"lib.scs\" section=tt\nsimulator lang=spice\nR1 a 0 1k\n* @RNL {#R1 @PLACE(0, 0)}\nsimulator lang=spectre\nR2 (a 0) resistor r=2k\n";
        let lib = "simulator lang=spectre\nlibrary models\nsection ff\n// @RNL {#X}\nmodel nch bsim4 type=n\nendsection ff\nsection tt\nmodel nch bsim4 type=n\nendsection tt\nendlibrary\n";
        let p = MemoryProvider::new().with("a.scs", net).with("lib.scs", lib);
        let mut sink = Sink::new();
        let doc = read("a.scs", &p, &mut sink).unwrap();
        assert_eq!(sink.all().len(), 1, "{:?}", sink.all());
        let names: Vec<&str> = doc.main().instances.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["R1", "R2"]);
        assert_eq!(doc.main().instances[0].value.as_deref(), Some("1k"));
        assert_eq!(doc.main().records.len(), 1);
        assert_eq!(doc.levels[1].name, "lib.scs (tt)");
        assert_eq!(doc.levels[1].section.as_deref(), Some("tt"));
    }

    #[test]
    fn protected_regions() {
        // Spectre's protect … unprotect, and .prot … .unprot in SPICE mode: read, a record in them
        // with a warning (2.2)
        let net = "// t\nprotect\nsubckt cell (a b)\nR1 (a b) resistor r=1k\n// @RNL {#R1 @PLACE(0, 0)}\nends cell\n\
                   unprotect\nsimulator lang=spice\n.prot\nX1 n1 n2 cell\n* @RNL {#X1 @PLACE(5, 0)}\n.unprot\n";
        let p = MemoryProvider::new().with("a.scs", net);
        let mut sink = Sink::new();
        let doc = read("a.scs", &p, &mut sink).unwrap();
        assert_eq!((doc.levels[1].records.len(), doc.main().records.len(), doc.main().instances.len()), (1, 1, 1));
        let said: Vec<(&str, Option<Loc>)> = sink.all().iter().map(|d| (d.message.as_str(), d.loc.clone())).collect();
        assert_eq!(said, [(PROTECTED, Some(Loc::line("a.scs", 5))), (PROTECTED_SPICE, Some(Loc::line("a.scs", 11)))]);
        assert_eq!(doc.protected, [("a.scs".to_string(), 2, 7), ("a.scs".to_string(), 9, 12)]);
    }

    #[test]
    fn the_lines_read_in_spice_mode() {
        // where `//` is no comment (2.2): from a `simulator lang=spice` to the `simulator
        // lang=spectre` after it, or to the end of the file; from the start of a file that is not a
        // .scs file; from the opening of a `.lib` section
        let net = "// t\nsimulator lang=spice\nR1 a 0 1k\nsimulator lang=spectre\ninclude \"x.inc\"\n\
                   simulator lang=spice\n.lib \"m.scs\" tt\nR2 a 0 1k\n";
        let inc = "* x\nsimulator lang=spectre\nR3 (a 0) resistor r=1k\n";
        let lib = "* m\n.lib tt\n.model nch nmos\n.endl tt\n";
        let p = MemoryProvider::new().with("a.scs", net).with("x.inc", inc).with("m.scs", lib);
        let mut sink = Sink::new();
        let doc = read("a.scs", &p, &mut sink).unwrap();
        // the circuit of the main file holds what its included file writes
        let names: Vec<&str> = doc.main().instances.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["R1", "R3", "R2"]);
        let region = |f: &str, a, b| (f.to_string(), a, b);
        assert_eq!(
            doc.spice_mode,
            [region("a.scs", 2, 4), region("x.inc", 0, 2), region("m.scs", 2, 5), region("a.scs", 6, 9)]
        );
        // the switches themselves: each is read in the mode before it
        let spice: Vec<u32> = (1..=9).filter(|&l| doc.in_spice_mode("a.scs", l)).collect();
        assert_eq!(spice, [3, 4, 7, 8, 9]);
    }

    #[test]
    fn a_file_that_is_not_a_scs_file_is_in_spice_mode_up_to_its_switch() {
        // also within its first lines: before `simulator lang=spectre`, `//` is no comment and
        // carries no record; after it, it does (2.2). So for the main file and an included one
        let net =
            "title\n// @RNL {#R1 @PLACE(0, 0)}\nsimulator lang=spectre\ninclude \"x.inc\"\nR1 (a 0) resistor r=1k\n\
                   // @RNL {#R1 @PLACE(1, 2)}\n";
        let inc = "* x\n// @RNL {#R2 @PLACE(0, 0)}\nsimulator lang=spectre\nR2 (a 0) resistor r=1k\n// @RNL {#R2 @PLACE(3, 4)}\n";
        let p = MemoryProvider::new().with("t.net", net).with("x.inc", inc);
        let mut sink = Sink::new();
        let doc = read("t.net", &p, &mut sink).unwrap();
        let lines = |lv: usize| doc.levels[lv].records.iter().map(|r| r.lines.0).collect::<Vec<u32>>();
        assert_eq!((lines(0), lines(1)), (vec![6], vec![5]));
        // the lines with `//` in SPICE mode are statements of the host, which the adapter skips
        let said: Vec<(String, u32)> =
            sink.all().iter().filter_map(|d| d.loc.as_ref().map(|l| (l.file.clone(), l.line))).collect();
        assert!(said.contains(&("t.net".into(), 2)) && said.contains(&("x.inc".into(), 2)), "{:?}", sink.all());
    }

    #[test]
    fn names_defined_twice() {
        // names compare by case in SPICE mode too, and each subcircuit has names of its own
        let net = "// t\nsimulator lang=spectre\nR1 (a 0) resistor r=1k\nr1 (a 0) resistor r=1k\nsimulator lang=spice\nR1 a 0 2k\nsimulator lang=spectre\nsubckt cell (a)\nR1 (a 0) resistor r=1k\nends cell\n";
        let p = MemoryProvider::new().with("a.scs", net);
        let mut sink = Sink::new();
        let doc = read("a.scs", &p, &mut sink).unwrap();
        let msgs: Vec<&str> = sink.all().iter().map(|d| d.message.as_str()).collect();
        assert_eq!(msgs, vec!["instance R1 is defined twice"]);
        let names: Vec<&str> = doc.main().instances.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["R1", "r1"]);
        assert_eq!(doc.main().instances[1].order, 1);
        assert_eq!(doc.levels[1].instances.len(), 1);
    }

    #[test]
    fn files_are_named_relative_to_the_main_file_at_the_root_too() {
        // a file beside the main file and one below it, each with RNL in a library section not selected
        let top = "// top\nsimulator lang=spectre\ninclude \"x.scs\" section=tt\ninclude \"sub/y.scs\" section=tt\n";
        let lib = "library models\nsection ff\n// @RNL {#X}\nendsection ff\nsection tt\nendsection tt\nendlibrary\n";
        for dir in ["", "/w"] {
            let p = MemoryProvider::new()
                .with(&format!("{dir}/top.scs"), top)
                .with(&format!("{dir}/x.scs"), lib)
                .with(&format!("{dir}/sub/y.scs"), lib);
            let mut sink = Sink::new();
            let doc = read(&format!("{dir}/top.scs"), &p, &mut sink).unwrap();
            let sources: Vec<&str> = doc.sources.iter().map(|(f, _)| f.as_str()).collect();
            assert_eq!(sources, ["top.scs", "x.scs", "sub/y.scs"], "{dir}");
            let files: Vec<&str> = doc.levels.iter().map(|l| l.file.as_str()).collect();
            assert_eq!(files, ["top.scs", "x.scs", "sub/y.scs"], "{dir}");
            let places: Vec<Option<Loc>> = sink.all().iter().map(|d| d.loc.clone()).collect();
            assert_eq!(places, [Some(Loc::line("x.scs", 3)), Some(Loc::line("sub/y.scs", 3))], "{dir}");
        }
    }

    #[test]
    fn included_files_of_one_name_in_two_directories_are_two_sources() {
        // a/x.scs and b/x.scs: each is a source, by its name relative to the main file, with its text
        let top =
            "// top\nsimulator lang=spectre\ninclude \"a/x.scs\"\ninclude \"b/x.scs\"\nX1 (n1 0) ca\nX2 (n2 0) cb\n";
        let a = "// a\nsubckt ca (p n)\nR1 (p n) resistor r=1k\nends ca\n";
        let b = "// b\nsubckt cb (p n)\nR1 (p n) resistor r=2k\nends cb\n";
        let p = MemoryProvider::new().with("/w/top.scs", top).with("/w/a/x.scs", a).with("/w/b/x.scs", b);
        let mut sink = Sink::new();
        let doc = read("/w/top.scs", &p, &mut sink).unwrap();
        assert!(sink.all().is_empty(), "{:?}", sink.all());
        let sources: Vec<(&str, &str)> = doc.sources.iter().map(|(f, t)| (f.as_str(), t.as_str())).collect();
        assert_eq!(sources, [("top.scs", top), ("a/x.scs", a), ("b/x.scs", b)]);
    }
}
