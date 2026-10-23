//! SPICE host adapter (standard 2.1–2.7, 6.1, 6.3, 6.12).
//!
//! Reads a SPICE netlist and the files it includes into the runtime's dialect-neutral host model:
//! levels (the main file level, included file levels, `.lib` sections, subcircuits), instances,
//! ports, and the RNL records cut out of the carrier lines. It covers the structural subset:
//! elements, nets, subcircuits and their ports, include and lib, model names and types. It does
//! not evaluate expressions or interpret analyses.
//!
//! A carrier is a whole-line comment with `*` in column 1 followed by a space, or a bare `*`
//! (2.2). The first line of every file is a title and never a carrier.

pub mod regions;

use std::collections::{HashMap, HashSet};

use rnl_runtime::diag::{Loc, Sink};
use rnl_runtime::host::{file_name, Dialect, HostDocument, Instance, Level, LevelId, LevelKind};
use rnl_runtime::records::{scan_run, CarrierLine};
use rnl_runtime::source::{dirname, normalize, relative_to, resolve_include, Provider};

/// A record in a region written to be encrypted (2.2).
const PROTECTED: &str = "RNL between `.prot` and `.unprot` is read, but it is gone once the region is encrypted (2.2)";

/// The SPICE dialect: names ignore case; ngspice's `gnd` is ground (6.3).
pub fn dialect() -> Dialect {
    Dialect { name: "spice".into(), case_sensitive: false, ground_aliases: vec!["gnd".into()] }
}

/// Advisory only: these generic adapters do not identify a verified simulator/version.
/// Return whether this line supplied the file's compatibility hint (standard 3.1).
/// `slash_comments` selects Spectre comments; otherwise use SPICE's `;`/`$` comments.
/// Quoted strings and SPICE braced expressions are host code, not comment text.
pub fn hint_comment_compatibility(raw: &str, slash_comments: bool, file: &str, line: u32, sink: &mut Sink) -> bool {
    let chars: Vec<char> = raw.chars().collect();
    let first = chars.iter().position(|c| !c.is_whitespace()).unwrap_or(chars.len());
    let mut comment = None;
    if chars.get(first) == Some(&'*') || (slash_comments && chars.get(first..first + 2) == Some(&['/', '/'])) {
        comment = Some(first);
    } else {
        let mut quote = None;
        let mut braces = 0usize;
        let mut k = 0;
        while k < chars.len() {
            let c = chars[k];
            if let Some(q) = quote {
                if c == '\\' {
                    k += 2;
                    continue;
                }
                if c == q {
                    quote = None;
                }
            } else if c == '"' || (!slash_comments && c == '\'') {
                quote = Some(c);
            } else if !slash_comments && c == '{' {
                braces += 1;
            } else if !slash_comments && c == '}' {
                braces = braces.saturating_sub(1);
            } else if braces == 0
                && (k == 0 || chars[k - 1].is_whitespace())
                && (if slash_comments { c == '/' && chars.get(k + 1) == Some(&'/') } else { c == ';' || c == '$' })
            {
                comment = Some(k);
                break;
            }
            k += 1;
        }
    }
    let Some(column) =
        comment.and_then(|start| chars[start..].iter().position(|c| !c.is_ascii()).map(|at| start + at + 1))
    else {
        return false;
    };
    sink.hint(
        "non-ASCII characters in host comments have not been verified for the simulator/version used by this generic adapter; check compatibility before simulation (3.1)",
        Some(Loc::new(file, line, column as u32)),
    );
    true
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

/// The tokens of an element line: its words, with a quoted text (`'2 * V(in)'`, `"..."`) or a
/// braced expression (`{ 2*V(in) }`) one token whatever spaces it holds, and a `name = value` with
/// spaces about the `=` one token `name=value`. The `=` of a `TABLE {V(in)} = (0, 0) (1, 1m)` after a
/// braced expression is no assignment, and joins nothing.
pub fn element_tokens(body: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let (mut word, mut quote, mut braces) = (String::new(), None::<char>, 0usize);
    for c in body.chars() {
        match quote {
            Some(q) => {
                word.push(c);
                if c == q {
                    quote = None;
                }
                continue;
            }
            None if braces > 0 => {
                word.push(c);
                match c {
                    '{' => braces += 1,
                    '}' => braces -= 1,
                    _ => {}
                }
                continue;
            }
            None => {}
        }
        match c {
            '\'' | '"' => {
                quote = Some(c);
                word.push(c);
            }
            '{' => {
                braces = 1;
                word.push(c);
            }
            c if c.is_whitespace() => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            c => word.push(c),
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    // `name = value`, `name= value` and `name =value` are `name=value`
    let mut out: Vec<String> = Vec::new();
    let mut k = 0;
    while k < words.len() {
        let w = &words[k];
        let after_brace = out.last().is_some_and(|p| p.starts_with('{') || p.ends_with('}'));
        if w == "=" && !out.is_empty() && !after_brace && k + 1 < words.len() {
            let last = out.pop().unwrap();
            out.push(format!("{last}={}", words[k + 1]));
            k += 2;
        } else if w.starts_with('=') && w.len() > 1 && !out.is_empty() && !after_brace {
            let last = out.pop().unwrap();
            out.push(format!("{last}{w}"));
            k += 1;
        } else if w.ends_with('=') && w.len() > 1 && !w.starts_with(['\'', '"', '{']) && k + 1 < words.len() {
            out.push(format!("{w}{}", words[k + 1]));
            k += 2;
        } else {
            out.push(w.clone());
            k += 1;
        }
    }
    out
}

/// Split the tokens of an element line into plain tokens and `name=value` parameters. A token that
/// starts with a quote or a brace is a text or an expression, plain whatever it holds.
pub fn split_params<S: AsRef<str>>(tokens: &[S]) -> (Vec<String>, Vec<(String, String)>) {
    let mut plain = Vec::new();
    let mut params = Vec::new();
    for t in tokens {
        let t = t.as_ref();
        match t.find('=') {
            Some(k) if k > 0 && !t.starts_with(['\'', '"', '{']) => {
                params.push((t[..k].to_string(), t[k + 1..].to_string()))
            }
            _ => plain.push(t.to_string()),
        }
    }
    (plain, params)
}

/// The length in bytes of the number a token starts with, digits with an optional fraction and
/// exponent and a sign: `2.0`, `-1e-3u` start with one; `2n3904` and `V(in)` do not (it is `2`
/// and then letters and digits), `n3904` has none.
fn number_len(t: &str) -> Option<usize> {
    let unsigned = t.strip_prefix(['+', '-']).unwrap_or(t);
    let sign = t.len() - unsigned.len();
    let mantissa = unsigned.chars().take_while(|c| c.is_ascii_digit() || *c == '.').count();
    if !unsigned[..mantissa].chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut end = mantissa;
    if let Some(e) = unsigned[end..].strip_prefix(['e', 'E']) {
        let signed = usize::from(e.starts_with(['+', '-']));
        let digits = e[signed..].chars().take_while(|c| c.is_ascii_digit()).count();
        if digits > 0 {
            end += 1 + signed + digits;
        }
    }
    Some(sign + end)
}

/// Whether a token is a number as SPICE writes one: digits with an optional fraction and exponent,
/// then letters for a scale and a unit, such as `2.0`, `1u` or `10meg`; `2n3904` is a name.
pub fn is_number(t: &str) -> bool {
    number_len(t).is_some_and(|n| t[n..].chars().all(|c| c.is_ascii_alphabetic()))
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
    let toks = element_tokens(body);
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
            // `PARAMS:` ends the nodes and the subcircuit's name, and a lone `/` only separates the
            // nodes from the name (CDL): neither is a node, and neither is the name
            let end = plain.iter().position(|t| t.eq_ignore_ascii_case("params:")).unwrap_or(plain.len());
            let mut head: Vec<String> = plain[..end].iter().filter(|t| t.as_str() != "/").cloned().collect();
            let last = head.pop()?;
            inst.model = Some(last);
            inst.nodes = head;
            inst.is_subckt = true;
        }
        // nodes and model are told apart once every `.model` is known (resolve_nodes)
        'M' | 'Q' => {}
        'D' => {
            inst.nodes = take(2);
            inst.model = plain.get(2).cloned();
        }
        'J' | 'Z' => {
            inst.nodes = take(3);
            inst.model = plain.get(3).cloned();
        }
        'U' if plain.len() >= 5 => {
            // HSPICE's lossy transmission line, `U in1 ref1 out1 ref2 model`, not SPICE3's RC line
            let (last, nodes) = plain.split_last()?;
            inst.nodes = nodes.to_vec();
            inst.model = Some(last.clone());
            line_of_transmission(&mut inst);
        }
        'U' => {
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
        'W' if is_transmission_line(&inst.params, &plain) => {
            // HSPICE's transmission line, `W in1 ref1 out1 ref2 RLGCMODEL=name N=1 L=10m`, not SPICE3's
            // current-controlled switch, `W n+ n- Vname model`
            inst.nodes = plain.clone();
            inst.model =
                ["rlgcmodel", "tablemodel", "fsmodel", "umodel"].iter().find_map(|k| inst.param(k)).map(String::from);
            line_of_transmission(&mut inst);
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
            inst.value = rest(2).or_else(|| named_value(letter, &inst));
        }
        'E' | 'G' if behavioural(&toks[1..]).is_some() => {
            // a nonlinear source: its value is an expression or a table, and POLY(k) has the k
            // pairs of nodes it reads after its two
            let (nodes, value) = behavioural(&toks[1..])?;
            inst.nodes = nodes;
            inst.value = value;
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
    if matches!(letter, 'R' | 'C' | 'L' | 'V' | 'I') {
        if let Some(v) = &inst.value {
            note_milli(v, file, line, sink);
        }
    }
    Some(Element { inst, plain })
}

/// Whether a W line is a transmission line of HSPICE, not a current-controlled switch: it has a
/// parameter only a transmission line has, or more words than the switch's `n+ n- Vname model`,
/// not counting the `on` or `off` that ends a switch.
fn is_transmission_line(params: &[(String, String)], plain: &[String]) -> bool {
    const OF_A_LINE: [&str; 8] = ["n", "l", "rlgcmodel", "tablemodel", "fsmodel", "umodel", "delay", "td"];
    let words = plain.iter().filter(|t| !t.eq_ignore_ascii_case("on") && !t.eq_ignore_ascii_case("off")).count();
    words > 4 || params.iter().any(|(k, _)| OF_A_LINE.iter().any(|p| k.eq_ignore_ascii_case(p)))
}

/// Make the instance a transmission line (a W or U of HSPICE): its type, and the names of its
/// terminals when it has the four of a single line, as T and O have them (6.12).
fn line_of_transmission(inst: &mut Instance) {
    inst.mtype = Some("tline".to_string());
    inst.default_terminals = Some(if inst.nodes.len() == 4 {
        ["p1", "n1", "p2", "n2"].iter().map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    });
}

/// The parameters of an element line that stand for the value that other writings give by
/// position: `R1 a b r=10k`, `C1 a b c=1p`, `L1 a b l=2n`, `V1 a b dc=1.8`, `B1 a b v=2*V(in)`.
fn named_value(letter: char, inst: &Instance) -> Option<String> {
    let names: &[&str] = match letter {
        'R' => &["r", "resistance"],
        'C' => &["c", "capacitance"],
        'L' => &["l", "inductance"],
        'V' | 'I' => &["dc"],
        'B' => &["v", "i"],
        _ => return None,
    };
    names.iter().find_map(|n| inst.param(n)).map(String::from)
}

/// For an E or G whose value is an expression or a table, not the gain of four nodes: its nodes and
/// its value, from the words after its name. It is told by the word where the control nodes would
/// be: `VALUE`, `VOL`, `CUR`, `TABLE`, `LAPLACE`, `FREQ`, `CHEBYSHEV`, `VALUE={...}`, `VOL='...'`,
/// or `POLY(k)`, which reads k pairs of control nodes after the keyword. The value is the text
/// from the keyword on, but for the control nodes of a POLY.
fn behavioural(tail: &[String]) -> Option<(Vec<String>, Option<String>)> {
    let word = tail.get(2)?.to_ascii_lowercase();
    let key = word.split(['=', '(']).next().unwrap_or("");
    if key == "poly" {
        // POLY(k) or POLY (k)
        let (count, from) = match word.strip_prefix("poly(") {
            Some(inner) => (inner.trim_end_matches(')'), 3),
            None => (tail.get(3)?.trim_matches(['(', ')']), 4),
        };
        let pairs: usize = count.parse().ok()?;
        let controls = tail.get(from..from + 2 * pairs)?;
        let mut nodes = tail[..2].to_vec();
        nodes.extend(controls.iter().cloned());
        let coefficients = &tail[from + 2 * pairs..];
        let value = tail[2..from].iter().chain(coefficients).cloned().collect::<Vec<_>>().join(" ");
        return Some((nodes, Some(value)));
    }
    let keyword = word == key || word.starts_with(&format!("{key}="));
    (keyword && matches!(key, "value" | "vol" | "cur" | "table" | "laplace" | "freq" | "chebyshev"))
        .then(|| (tail[..2].to_vec(), Some(tail[2..].join(" "))))
}

/// A hint for a value that SPICE reads as milli though it may be written as mega: `10M` is
/// 10 milli, as `10m` is, and mega is `meg` (ngspice, HSPICE and LTspice alike).
fn note_milli(value: &str, file: &str, line: u32, sink: &mut Sink) {
    for t in value.split_whitespace() {
        let Some(n) = number_len(t) else { continue };
        let suffix = &t[n..];
        let lower = suffix.to_ascii_lowercase();
        if suffix.starts_with('M')
            && suffix.chars().all(|c| c.is_ascii_alphabetic())
            && !lower.starts_with("meg")
            && !lower.starts_with("mil")
        {
            sink.hint(
                format!(
                    "{t} is milli in SPICE, as {} is: M is milli in either case, and mega is meg",
                    t.to_ascii_lowercase()
                ),
                Some(Loc::line(file, line)),
            );
        }
    }
}

/// Register what a `.model` card says in `models`, by lower case name: the type, which is the word
/// after the name. A binned model such as `pch.1` is registered under its full name and under its
/// base name `pch`, since an instance may write either. The type of LTspice's `vdmos` is its
/// polarity: `pmos` with the word `pchan`, else `nmos`.
pub fn register_model(models: &mut HashMap<String, String>, card: &str) {
    let toks: Vec<&str> = card.split(|c: char| c.is_whitespace() || c == '(').filter(|t| !t.is_empty()).collect();
    if toks.len() < 3 {
        return;
    }
    let name = toks[1].to_lowercase();
    let mut kind = toks[2].to_lowercase();
    if kind == "vdmos" {
        kind = if toks[3..].iter().any(|t| t.eq_ignore_ascii_case("pchan")) { "pmos" } else { "nmos" }.to_string();
    }
    if let Some((base, bin)) = name.rsplit_once('.') {
        if !base.is_empty() && !bin.is_empty() && bin.chars().all(|c| c.is_ascii_digit()) {
            models.entry(base.to_string()).or_insert_with(|| kind.clone());
        }
    }
    models.insert(name, kind);
}

/// Whether the model of an element is among the words after its nodes: the elements that read a
/// `.model` card to tell its nodes from its model and its value, or its polarity from the card
/// (`J` and `Z`, whose nodes are known without it).
pub fn needs_models(letter: Option<char>) -> bool {
    matches!(letter, Some('M') | Some('Q') | Some('J') | Some('Z') | Some('R') | Some('C') | Some('L'))
}

/// Where the model name of an M or a Q stands in its words, once all `.model` names are known: the
/// first word after the three nodes that every such device has that names a model; else the last
/// word that is no number and no `off`, since values by position, such as an area, come after it.
fn model_position(plain: &[String], is_model: impl Fn(&str) -> bool) -> Option<usize> {
    if plain.len() < 4 {
        return None;
    }
    (3..plain.len())
        .find(|&k| is_model(&plain[k]))
        .or_else(|| (3..plain.len()).rev().find(|&k| !is_number(&plain[k]) && !plain[k].eq_ignore_ascii_case("off")))
}

/// Tell nodes from the model name for M and Q once all model names are known, and a resistor,
/// capacitor or inductor of a `.model` card, `R1 a b rpoly 10k`, from a value. A transistor whose
/// model has no card, or names none, is marked: its polarity is not known (`Instance::model_missing`).
pub fn resolve_nodes(inst: &mut Instance, plain: &[String], models: &HashMap<String, String>) {
    let is_model = |t: &str| models.contains_key(&t.to_lowercase());
    match inst.letter {
        Some('M') | Some('Q') => {
            match model_position(plain, is_model) {
                Some(k) => {
                    inst.nodes = plain[..k].to_vec();
                    inst.model = Some(plain[k].clone());
                }
                None => inst.nodes = plain.to_vec(),
            }
            inst.model_missing = !inst.model.as_deref().is_some_and(is_model);
        }
        Some('J') | Some('Z') => inst.model_missing = !inst.model.as_deref().is_some_and(is_model),
        Some(letter @ ('R' | 'C' | 'L')) => {
            if let Some(m) = plain.get(2).filter(|m| is_model(m)) {
                inst.model = Some(m.clone());
                let value: Vec<&str> = plain[3..].iter().map(String::as_str).collect();
                inst.value = if value.is_empty() { named_value(letter, inst) } else { Some(value.join(" ")) };
            }
        }
        _ => {}
    }
}

/// The ports among the tokens of a `.subckt` statement after its name, or of a `+` line that
/// continues it, and whether its parameters begin among them, `params:` or a `name=value`, after
/// which no token is a port.
pub fn subckt_ports(tokens: &[&str]) -> (Vec<String>, bool) {
    let mut ports = Vec::new();
    for t in tokens {
        if t.contains('=') || t.eq_ignore_ascii_case("params:") {
            return (ports, true);
        }
        ports.push(t.to_string());
    }
    (ports, false)
}

/// The tokens of a line of a `.subckt` statement, without its end-of-line comment.
pub fn subckt_tokens(line: &str) -> Vec<String> {
    strip_inline_comment(line).split_whitespace().map(String::from).collect()
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
    /// The [`Level::name_key`]s of each level's instances, to find a name defined twice without
    /// looking through the level's instances.
    names: HashMap<LevelId, HashSet<String>>,
    unresolved: Vec<(LevelId, usize, Vec<String>)>,
    main_dir: String,
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
        let mut regions = regions::Regions::default();
        // the line of the `.prot` of a region written to be encrypted, while in it (2.2)
        let mut protect: Option<u32> = None;
        let mut ended = false;
        let mut compatibility_hint = false;
        let mut pending: Option<Pending> = None;
        // a `.subckt` whose `+` lines add ports, until its parameters begin
        let mut subckt_more: Option<LevelId> = None;
        let mut continued = Continued::default();
        // `.lib` sections: inside the selected one when reading a section; skipping an unselected one
        let mut in_section = section.is_none();
        let mut skip_lib: Option<(String, u32)> = None;

        for (idx, raw) in lines.iter().enumerate() {
            let no = idx as u32 + 1;
            let raw = raw.trim_end_matches('\r');
            let stripped = raw.trim();
            let low = stripped.to_lowercase();
            if !compatibility_hint {
                compatibility_hint = hint_comment_compatibility(raw, false, &fname, no, self.sink);
            }
            if no == 1 {
                if raw.contains("@RNL") {
                    self.sink.warning(
                        "the first line of a file is a title and is not read as RNL (2.2)",
                        Some(Loc::line(&fname, 1)),
                    );
                }
                // Encryption pragmas are meaningful even on a file's first comment line.
                if raw.trim_start().starts_with("//") {
                    regions.skip(raw, no, false, main, &fname, self.sink);
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
                let card = strip_inline_comment(stripped);
                let toks: Vec<&str> = card.split_whitespace().collect();
                let is_lib = toks.first().is_some_and(|t| t.eq_ignore_ascii_case(".lib"));
                let is_endl = toks.first().is_some_and(|t| t.eq_ignore_ascii_case(".endl"));
                if let Some((name, start)) = &skip_lib {
                    if is_endl && toks.get(1).is_none_or(|t| t.eq_ignore_ascii_case(name)) {
                        self.doc.unselected_sections.push((level, *start, no));
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
                    self.doc.levels[level].section_lines.get_or_insert((no, 0));
                } else if raw.contains("@RNL") {
                    self.sink.warning(
                        "RNL outside the selected `.lib` section is not read (2.2)",
                        Some(Loc::line(&fname, no)),
                    );
                }
                continue;
            }
            if regions.skip(raw, no, false, main, &fname, self.sink) {
                self.flush_run(&mut run, &fname, cur);
                subckt_more = None;
                if let Some(p) = pending.take() {
                    self.statement(&p.text, p.line, &fname, path, cur, circ, stack);
                }
                continue;
            }
            if let Some(rest) = raw.strip_prefix('*') {
                if rest.trim().is_empty() || rest.starts_with(' ') {
                    if (pending.is_some() || subckt_more.is_some()) && continued.after(&lines, idx) {
                        self.sink.warning(COMMENT_IN_A_STATEMENT, Some(Loc::line(&fname, no)));
                    }
                    if protect.is_some() && rest.contains("@RNL") {
                        self.sink.warning(PROTECTED, Some(Loc::line(&fname, no)));
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
                } else if let Some(sub) = subckt_more {
                    let toks = subckt_tokens(cont);
                    let (ports, params) = subckt_ports(&toks.iter().map(String::as_str).collect::<Vec<_>>());
                    self.doc.levels[sub].ports.extend(ports);
                    if params {
                        subckt_more = None;
                    }
                }
                continue;
            }
            subckt_more = None;
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
                    self.doc.levels[level].close_section(no);
                }
                continue;
            }
            if first == ".prot" || first == ".protect" {
                protect.get_or_insert(no);
                continue;
            }
            if first == ".unprot" || first == ".unprotect" {
                if let Some(start) = protect.take() {
                    self.doc.protected.push((fname.clone(), start, no));
                }
                continue;
            }
            if first == ".lib" {
                let card = strip_inline_comment(stripped);
                let toks: Vec<&str> = card.split_whitespace().collect();
                if toks.len() == 2 {
                    // a section definition met while reading the file as a whole: not selected
                    skip_lib = Some((toks[1].to_string(), no));
                } else if toks.len() >= 3 {
                    let file = unquote(toks[1]);
                    self.include(&file, Some(toks[2]), no, &fname, path, cur, circ, stack, true);
                }
                continue;
            }
            if first == ".subckt" {
                let toks = subckt_tokens(stripped);
                let toks: Vec<&str> = toks.iter().map(String::as_str).collect();
                let name = toks.get(1).copied().unwrap_or("?");
                let sub = self.new_level(LevelKind::Subckt, name, &fname, path, cur, true, no);
                let (ports, params) = subckt_ports(&toks[2.min(toks.len())..]);
                self.doc.levels[sub].ports = ports;
                subckt_more = (!params).then_some(sub);
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
        self.doc.excluded.extend(regions.finish(&fname, lines.len() as u32 + 1, self.sink));
        if let Some((_, start)) = skip_lib {
            self.doc.unselected_sections.push((level, start, lines.len() as u32 + 1));
        }
        if let Some(start) = protect {
            self.doc.protected.push((fname.clone(), start, lines.len() as u32 + 1));
        }
        if !sub_stack.is_empty() {
            self.sink.error("a subcircuit has no `.ends`", Some(Loc::line(&fname, lines.len() as u32)));
        }
        if let Some(name) = section.filter(|_| self.doc.levels[level].section_lines.is_none()) {
            self.sink.error(
                format!("the selected library section {name} was not found; no records can be written there (2.2)"),
                Some(Loc::line(&fname, 1)),
            );
        }
        if self.doc.levels[level].end_line == 0 {
            self.doc.levels[level].end_line = lines.len() as u32 + 1;
        }
        self.doc.levels[level].close_section(lines.len() as u32 + 1);
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
        explicit: bool,
    ) {
        let full = resolve_include(self.provider, path, target, self.sink, Loc::line(fname, no));
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
        self.doc.levels[child].section = section.map(String::from);
        self.doc.levels[child].include_site = explicit.then(|| Loc::line(fname, no));
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
                self.include(&target, None, no, fname, path, cur, circ, stack, true);
            }
            return;
        }
        if first == ".model" {
            register_model(&mut self.models, &body);
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
        // the include or section the line was read through (2.8)
        let file_level = self.doc.file_level(cur);
        let lv = &mut self.doc.levels[circ];
        if !self.names.entry(circ).or_default().insert(lv.name_key(&el.inst.name)) {
            self.sink.error(format!("element {} is defined twice", el.inst.name), Some(Loc::line(fname, no)));
            return;
        }
        let mut inst = el.inst;
        inst.file_level = Some(file_level);
        inst.order = lv.instances.len();
        let needs = needs_models(inst.letter);
        lv.instances.push(inst);
        if needs {
            let k = lv.instances.len() - 1;
            self.unresolved.push((circ, k, el.plain));
        }
    }
}

fn unquote(s: &str) -> String {
    s.trim_matches(|c| c == '\'' || c == '"').to_string()
}

/// The warning for a comment line between a statement, a `.subckt` with its ports among them, and
/// a `+` line that continues it (2.2). The Spectre adapter gives it in SPICE mode too.
pub const COMMENT_IN_A_STATEMENT: &str =
    "a comment line between a statement and its continuation: read, but hosts treat it differently (2.2)";

/// The answer of [`Continued::after`] for one run of comment lines: it is the same for all the
/// lines of the run, so each run is looked through once.
#[derive(Default)]
pub struct Continued {
    /// The answer holds for the lines from `from` up to the one before `next`, the first line after
    /// `from` that is not a comment line.
    from: usize,
    next: usize,
    answer: bool,
}

impl Continued {
    /// Whether the first line after line `idx` that is not a comment line is a continuation line.
    pub fn after(&mut self, lines: &[&str], idx: usize) -> bool {
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
    read_with_includes(path, provider, sink, &[])
}

/// Read additional files as root includes, without altering source text or line numbers.
pub fn read_with_includes(
    path: &str,
    provider: &dyn Provider,
    sink: &mut Sink,
    includes: &[String],
) -> Option<HostDocument> {
    read_as(path, provider, sink, includes, true)
}

/// Index a library file as an include: its first line is a statement, not a SPICE title.
pub fn read_library(path: &str, provider: &dyn Provider, sink: &mut Sink) -> Option<HostDocument> {
    read_as(path, provider, sink, &[], false)
}

fn read_as(
    path: &str,
    provider: &dyn Provider,
    sink: &mut Sink,
    includes: &[String],
    main_file: bool,
) -> Option<HostDocument> {
    let path = normalize(path);
    provider.read(&path)?;
    let main_dir = dirname(&path);
    let fname = file_name(&path).to_string();
    let mut main = Level::new(LevelKind::File, fname.clone(), fname, path.clone());
    main.circuit = true;
    let doc = HostDocument::new(main, dialect());
    let mut r =
        Reader { provider, sink, doc, models: HashMap::new(), names: HashMap::new(), unresolved: Vec::new(), main_dir };
    r.read_file(&path, 0, 0, main_file, &[format!("{path}#")], None);
    let fname = file_name(&path);
    for extra in includes {
        r.include(extra, None, 1, fname, &path, 0, 0, &[format!("{path}#")], false);
    }
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
            // a type the adapter gave (a transmission line) stands; else the model card's
            if !inst.is_subckt && inst.mtype.is_none() {
                inst.mtype = inst.model.as_ref().and_then(|m| models.get(&m.to_lowercase()).cloned());
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
        assert_eq!(doc.levels[1].section.as_deref(), Some("tt"));
        assert_eq!(doc.levels[0].section, None);
        assert!(doc.subckt("CELL").is_some());
        assert_eq!(sink.all().len(), 1);
        assert_eq!(doc.main().instances[0].mtype.as_deref(), Some("nmos"));
    }

    #[test]
    fn protected_regions() {
        // a region written to be encrypted is read, a record in it with a warning, since the record
        // is gone once the region is encrypted (2.2); one left open closes after the last line
        let net = "* t\n.prot\n.subckt cell a b\nR1 a b 1k\n* @RNL {#R1 @PLACE(0, 0)}\n.ends\n.unprot\nX1 n1 n2 cell\n\
                   .PROTECT\nR2 n1 0 1k\n.end\n";
        let p = MemoryProvider::new().with("t.sp", net);
        let mut sink = Sink::new();
        let doc = read("t.sp", &p, &mut sink).unwrap();
        assert_eq!((doc.levels[1].records.len(), doc.levels[1].instances.len(), doc.main().instances.len()), (1, 1, 2));
        let said: Vec<(&str, Option<Loc>)> = sink.all().iter().map(|d| (d.message.as_str(), d.loc.clone())).collect();
        assert_eq!(said, [(PROTECTED, Some(Loc::line("t.sp", 5)))]);
        assert_eq!(doc.protected, [("t.sp".to_string(), 2, 7), ("t.sp".to_string(), 9, 12)]);
    }

    #[test]
    fn names_defined_twice() {
        // names ignore case in their ASCII letters only, and each subcircuit has names of its own
        let net = "* t\nR1 a 0 1k\nr1 a 0 2k\nRÄ a 0 1k\nRä a 0 1k\n.subckt cell a\nR1 a 0 1k\n.ends\n.end\n";
        let p = MemoryProvider::new().with("t.sp", net);
        let mut sink = Sink::new();
        let doc = read("t.sp", &p, &mut sink).unwrap();
        let msgs: Vec<&str> = sink.all().iter().map(|d| d.message.as_str()).collect();
        assert_eq!(msgs, vec!["element r1 is defined twice"]);
        let names: Vec<&str> = doc.main().instances.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["R1", "RÄ", "Rä"]);
        assert_eq!(doc.main().instances[2].order, 2);
        assert_eq!(doc.levels[1].instances.len(), 1);
    }

    #[test]
    fn files_are_named_relative_to_the_main_file_at_the_root_too() {
        // a file beside the main file and one below it, each with RNL on its title line
        let top = "* top\n.include \"x.inc\"\n.include \"sub/y.inc\"\n.end\n";
        let (x, y) = ("* @RNL {#R1}\n.model nch nmos\n", "* @RNL {#R2}\n.model pch pmos\n");
        for dir in ["", "/w"] {
            let p = MemoryProvider::new()
                .with(&format!("{dir}/top.sp"), top)
                .with(&format!("{dir}/x.inc"), x)
                .with(&format!("{dir}/sub/y.inc"), y);
            let mut sink = Sink::new();
            let doc = read(&format!("{dir}/top.sp"), &p, &mut sink).unwrap();
            let sources: Vec<&str> = doc.sources.iter().map(|(f, _)| f.as_str()).collect();
            assert_eq!(sources, ["top.sp", "x.inc", "sub/y.inc"], "{dir}");
            let files: Vec<&str> = doc.levels.iter().map(|l| l.file.as_str()).collect();
            assert_eq!(files, ["top.sp", "x.inc", "sub/y.inc"], "{dir}");
            let places: Vec<Option<Loc>> = sink.all().iter().map(|d| d.loc.clone()).collect();
            assert_eq!(places, [Some(Loc::line("x.inc", 1)), Some(Loc::line("sub/y.inc", 1))], "{dir}");
        }
    }

    #[test]
    fn included_files_of_one_name_in_two_directories_are_two_sources() {
        // a/x.inc and b/x.inc: each is a source, by its name relative to the main file, with its text
        let top = "* top\n.include \"a/x.inc\"\n.include \"b/x.inc\"\nX1 n1 0 ca\nX2 n2 0 cb\n.end\n";
        let (a, b) = ("* a\n.subckt ca p n\nR1 p n 1k\n.ends\n", "* b\n.subckt cb p n\nR1 p n 2k\n.ends\n");
        let p = MemoryProvider::new().with("/w/top.sp", top).with("/w/a/x.inc", a).with("/w/b/x.inc", b);
        let mut sink = Sink::new();
        let doc = read("/w/top.sp", &p, &mut sink).unwrap();
        assert!(sink.all().is_empty(), "{:?}", sink.all());
        let sources: Vec<(&str, &str)> = doc.sources.iter().map(|(f, t)| (f.as_str(), t.as_str())).collect();
        assert_eq!(sources, [("top.sp", top), ("a/x.inc", a), ("b/x.inc", b)]);
    }

    #[test]
    fn the_ports_of_a_subckt_on_its_continuation_lines_and_without_its_comment() {
        // the `+` lines of a `.subckt` add ports, up to its parameters; an end-of-line comment
        // holds none; the `+` line of an element after it is the element's
        for (text, ports) in [
            (".subckt cell a\n+ b\nR1 a b 1k\n.ends cell\n", vec!["a", "b"]),
            (".subckt cell a b ; two ports\nR1 a b 1k\n.ends cell\n", vec!["a", "b"]),
            (".subckt cell a $ the first\n+ b $ the second\n.ends cell\n", vec!["a", "b"]),
            (".subckt cell a\n+ b params: w=1\n+ c=2\n.ends cell\n", vec!["a", "b"]),
            (".subckt cell a\n\n+ b\nR1 a b\n+ 1k\n.ends cell\n", vec!["a", "b"]),
        ] {
            let p = MemoryProvider::new().with("t.sp", &format!("* t\n{text}.end\n"));
            let mut sink = Sink::new();
            let doc = read("t.sp", &p, &mut sink).unwrap();
            assert!(sink.all().is_empty(), "{text}: {:?}", sink.all());
            assert_eq!(doc.levels[1].ports, ports, "{text}");
            if let Some(r1) = doc.levels[1].instances.first() {
                assert_eq!(
                    (r1.nodes.as_slice(), r1.value.as_deref()),
                    (&["a".to_string(), "b".into()][..], Some("1k"))
                );
            }
        }
        // a comment line between it and its continuation is read, with a warning, as for an element
        let p = MemoryProvider::new().with("t.sp", "* t\n.subckt cell a\n* between\n+ b\n.ends cell\n.end\n");
        let mut sink = Sink::new();
        let doc = read("t.sp", &p, &mut sink).unwrap();
        assert_eq!(doc.levels[1].ports, ["a", "b"]);
        assert_eq!(sink.all().len(), 1, "{:?}", sink.all());
    }

    #[test]
    fn inline_comments() {
        assert_eq!(strip_inline_comment("R1 a b 1k ; x"), "R1 a b 1k");
        assert_eq!(strip_inline_comment("R1 a b 1k$x"), "R1 a b 1k$x");
        assert_eq!(strip_inline_comment("R1 a b 1k $ x"), "R1 a b 1k");
    }

    /// The instances of a netlist of one level, read, and the messages of the reading.
    fn read_all(net: &str) -> (Vec<Instance>, Vec<String>) {
        let p = MemoryProvider::new().with("t.sp", net);
        let mut sink = Sink::new();
        let doc = read("t.sp", &p, &mut sink).unwrap();
        let messages = sink.all().iter().map(|d| d.message.clone()).collect();
        (doc.main().instances.clone(), messages)
    }

    fn find<'a>(instances: &'a [Instance], name: &str) -> &'a Instance {
        instances.iter().find(|i| i.name == name).unwrap_or_else(|| panic!("no {name}"))
    }

    fn nodes(i: &Instance) -> String {
        i.nodes.join(" ")
    }

    #[test]
    fn tokens_keep_quotes_and_braces_whole_and_join_spaced_assignments() {
        let t = |s: &str| element_tokens(s);
        assert_eq!(t("M1 d g s b nch w = 1u l= 2u m =3"), ["M1", "d", "g", "s", "b", "nch", "w=1u", "l=2u", "m=3"]);
        assert_eq!(t("E1 out 0 VOL='2 * V(in)'"), ["E1", "out", "0", "VOL='2 * V(in)'"]);
        assert_eq!(t("E1 out 0 VALUE = { 2 * V(in) }"), ["E1", "out", "0", "VALUE={ 2 * V(in) }"]);
        // the = after a braced expression belongs to the table
        assert_eq!(
            t("G1 o 0 TABLE {V(in)} = (0, 0) (1, 1m)"),
            ["G1", "o", "0", "TABLE", "{V(in)}", "=", "(0,", "0)", "(1,", "1m)"]
        );
        let (plain, params) = split_params(&t("X1 a b sub w = 1u"));
        assert_eq!(plain, ["X1", "a", "b", "sub"]);
        assert_eq!(params, [("w".to_string(), "1u".to_string())]);
        for number in ["2.0", "1u", "-1e-3", "10meg", "1.5M"] {
            assert!(is_number(number), "{number}");
        }
        for name in ["2n3904", "pmod", "V(in)", ".", "dt"] {
            assert!(!is_number(name), "{name}");
        }
    }

    #[test]
    fn m_and_q_find_their_model_among_three_to_six_nodes_and_values() {
        let net = "* t\n.model pmod pmos\n.model nsoi nmos\n.model pnpm pnp\n\
                   M3 d g s pmod w=1u\nM4 d g s b nsoi\nM5 d g s e b nsoi\nM5x d g s e b ext_soi\nM4x d g s b ext\nM3x d g s ext\n\
                   M3s d g s pmod w = 1u\nMn 2 1 0 0 ext_n\nQ3 c b e pnpm\nQ4 c b e s pnpm\nQ5 c b e s dt pnpm\n\
                   Qarea c b e ext_pnp 2.0\nQoff c b e ext_pnp off\nQ4ext c b e s 2n3904\nQ3num c b e 2n3904 1.5\n.end\n";
        let (i, _) = read_all(net);
        let cases = [
            ("M3", "d g s", "pmod", "pmos"),
            ("M4", "d g s b", "nsoi", "nmos"),
            ("M5", "d g s e b", "nsoi", "nmos"),
            // models the file does not have: the last word that is no number
            ("M5x", "d g s e b", "ext_soi", ""),
            ("M4x", "d g s b", "ext", ""),
            ("M3x", "d g s", "ext", ""),
            ("M3s", "d g s", "pmod", "pmos"),
            ("Mn", "2 1 0 0", "ext_n", ""),
            ("Q3", "c b e", "pnpm", "pnp"),
            ("Q4", "c b e s", "pnpm", "pnp"),
            ("Q5", "c b e s dt", "pnpm", "pnp"),
            ("Qarea", "c b e", "ext_pnp", ""),
            ("Qoff", "c b e", "ext_pnp", ""),
            ("Q4ext", "c b e s", "2n3904", ""),
            ("Q3num", "c b e", "2n3904", ""),
        ];
        for (name, want_nodes, model, mtype) in cases {
            let inst = find(&i, name);
            assert_eq!(
                (nodes(inst), inst.model.as_deref(), inst.mtype.as_deref().unwrap_or("")),
                (want_nodes.to_string(), Some(model), mtype),
                "{name}"
            );
        }
    }

    #[test]
    fn a_binned_model_is_known_by_its_name_and_its_base_and_vdmos_by_its_polarity() {
        let net = "* t\n.model pch.1 pmos level=54\n.model pch.2 pmos\n.model nch.10 nmos\n.model qlat lpnp\n\
                   .model mp vdmos pchan Vto=-1.5\n.model mn VDMOS(Vto=3)\n.model d1 d\n\
                   M1 d g s b pch\nM2 d g s b pch.1\nM3 d g s b pch.2\nM4 d g s b nch\nMp d g s mp\nMn d g s mn\nQl c b e qlat\n.end\n";
        let (i, _) = read_all(net);
        let types: Vec<(&str, &str)> = ["M1", "M2", "M3", "M4", "Mp", "Mn", "Ql"]
            .iter()
            .map(|n| (*n, find(&i, n).mtype.as_deref().unwrap_or("-")))
            .collect();
        assert_eq!(
            types,
            [
                ("M1", "pmos"),
                ("M2", "pmos"),
                ("M3", "pmos"),
                ("M4", "nmos"),
                ("Mp", "pmos"),
                ("Mn", "nmos"),
                ("Ql", "lpnp")
            ]
        );
        assert_eq!(nodes(find(&i, "M1")), "d g s b");
        assert_eq!(nodes(find(&i, "Mp")), "d g s");
    }

    #[test]
    fn behavioural_sources_keep_their_nodes_and_show_their_expression() {
        let net = "* t\nE1 o1 0 VOL='2*V(in)'\nE2 o2 0 VALUE = { 2*V(in) }\nG1 o3 0 TABLE {V(in)} = (0, 0) (1, 1m)\n\
                   E3 o4 0 POLY(2) a 0 b 0 0 1.0 1.0\nE4 o5 0 LAPLACE {V(in)} = {1/(1+s)}\nE5 o6 0 CUR='I(v1)'\n\
                   E6 o7 0 c 0 2.5\nG2 o8 0 vol_in 0 1m\nE7 o9 0 POLY (1) c 0 0 1\n.end\n";
        let (i, _) = read_all(net);
        let got = |n: &str| (nodes(find(&i, n)), find(&i, n).value.clone().unwrap_or_default());
        assert_eq!(got("E1"), ("o1 0".into(), "VOL='2*V(in)'".into()));
        assert_eq!(got("E2"), ("o2 0".into(), "VALUE={ 2*V(in) }".into()));
        assert_eq!(got("G1"), ("o3 0".into(), "TABLE {V(in)} = (0, 0) (1, 1m)".into()));
        assert_eq!(got("E3"), ("o4 0 a 0 b 0".into(), "POLY(2) 0 1.0 1.0".into()));
        assert_eq!(got("E4"), ("o5 0".into(), "LAPLACE {V(in)} = {1/(1+s)}".into()));
        assert_eq!(got("E5"), ("o6 0".into(), "CUR='I(v1)'".into()));
        // a gain of four nodes, whatever the names of the nodes
        assert_eq!(got("E6"), ("o7 0 c 0".into(), "2.5".into()));
        assert_eq!(got("G2"), ("o8 0 vol_in 0".into(), "1m".into()));
        assert_eq!(got("E7"), ("o9 0 c 0".into(), "POLY (1) 0 1".into()));
    }

    #[test]
    fn a_subcircuit_call_ends_its_nodes_at_params_and_at_a_slash() {
        let net = "* t\nX1 a b vdd vss inv PARAMS: wp=2u wn=1u\nX2 a b vdd vss / inv wp=2u\nX3 a b inv w = 1u\nX4 a b inv\n.end\n";
        let (i, _) = read_all(net);
        for n in ["X1", "X2"] {
            assert_eq!(
                (nodes(find(&i, n)), find(&i, n).model.as_deref()),
                ("a b vdd vss".to_string(), Some("inv")),
                "{n}"
            );
        }
        for n in ["X3", "X4"] {
            assert_eq!((nodes(find(&i, n)), find(&i, n).model.as_deref()), ("a b".to_string(), Some("inv")), "{n}");
        }
    }

    #[test]
    fn w_and_u_are_transmission_lines_of_hspice_or_the_switch_and_rc_line_of_spice3() {
        let net =
            "* t\nW1 a b Vc swm\nW2 in1 ref1 out1 ref2 RLGCMODEL=wmod N=1 L=10m\nW3 i1 r1 o1 r2 wm\nWs a b Vc swm on\n\
                   U1 a b ref urc L=1u\nU2 in1 ref1 out1 ref2 umod L=10m\n.end\n";
        let (i, _) = read_all(net);
        let kind = |n: &str| {
            let x = find(&i, n);
            (nodes(x), x.model.clone().unwrap_or_default(), x.mtype.clone().unwrap_or_default())
        };
        assert_eq!(kind("W1"), ("a b".into(), "swm".into(), "".into()));
        assert_eq!(kind("Ws"), ("a b".into(), "swm".into(), "".into()));
        assert_eq!(kind("W2"), ("in1 ref1 out1 ref2".into(), "wmod".into(), "tline".into()));
        assert_eq!(kind("W3").2, "tline");
        assert_eq!(kind("U1"), ("a b ref".into(), "urc".into(), "".into()));
        assert_eq!(kind("U2"), ("in1 ref1 out1 ref2".into(), "umod".into(), "tline".into()));
        // the four nodes of a line are named as those of T and O are
        let names = |n: &str| find(&i, n).default_terminals.clone().unwrap_or_default().join(" ");
        assert_eq!(names("U2"), "p1 n1 p2 n2");
        assert_eq!(names("W1"), "p n");
    }

    #[test]
    fn values_given_by_name_and_models_of_resistors_and_capacitors() {
        let net = "* t\n.model rpoly r rsh=500\n.model cmim c\nR1 a b r=10k\nC1 a b c=1p\nL1 a b l=2n\nV1 a b dc=1.8\n\
                   B1 a b V=2*V(in)\nB2 a b I = V(in)/1k\nR2 a b rpoly 10k\nR3 a b rpoly r=10k w=2u l=20u\nC2 a b cmim 2p\n\
                   R4 a b 5k tc1 = 1e-3\n.end\n";
        let (i, _) = read_all(net);
        let got = |n: &str| {
            let x = find(&i, n);
            (
                x.value.clone().unwrap_or_default(),
                x.model.clone().unwrap_or_default(),
                x.mtype.clone().unwrap_or_default(),
            )
        };
        assert_eq!(got("R1").0, "10k");
        assert_eq!(got("C1").0, "1p");
        assert_eq!(got("L1").0, "2n");
        assert_eq!(got("V1").0, "1.8");
        assert_eq!(got("B1").0, "2*V(in)");
        assert_eq!(got("B2").0, "V(in)/1k");
        assert_eq!(got("R2"), ("10k".into(), "rpoly".into(), "r".into()));
        assert_eq!(got("R3"), ("10k".into(), "rpoly".into(), "r".into()));
        assert_eq!(got("C2"), ("2p".into(), "cmim".into(), "c".into()));
        assert_eq!(got("R4").0, "5k");
        assert_eq!(find(&i, "R4").param("tc1"), Some("1e-3"));
    }

    #[test]
    fn a_capital_m_after_a_number_is_milli_and_gets_a_hint() {
        let net = "* t\nR1 a b 10M\nR2 a b 10m\nR3 a b 10meg\nR4 a b 10Meg\nC1 a b 1.5M\nV1 a b dc 1e3M\nR5 a b {2*M}\nR6 a b 10MIL\n.end\n";
        let (_, hints) = read_all(net);
        let hinted: Vec<&str> = hints.iter().filter_map(|m| m.split_whitespace().next()).collect();
        assert_eq!(hinted, ["10M", "1.5M", "1e3M"], "{hints:?}");
        assert!(hints[0].contains("milli") && hints[0].contains("meg"), "{}", hints[0]);
    }

    #[test]
    fn a_transistor_whose_model_card_is_not_found_is_marked() {
        // M, Q, J and Z: the polarity is told by the card alone, so a model that no card names, or
        // none at all, leaves it unknown; the other elements are not marked
        let net = "* t\n.model nch nmos\n.model qn npn\n.model jn njf\n\
                   M1 d g s b nch\nM2 d g s b pch\nM3 d g s\nM4 d g s s nch w=1u\n\
                   Q1 c b e qn\nQ2 c b e 2n3904\nQ3 c b e s dt qn 2.0\n\
                   J1 d g s jn\nJ2 d g s jp\nZ1 d g s zm\n\
                   D1 a k dm\nR1 a b rm 1k\nC1 a b 1p\nX1 a b sub\n.end\n";
        let (insts, _) = read_all(net);
        let marked: Vec<&str> = insts.iter().filter(|i| i.model_missing).map(|i| i.name.as_str()).collect();
        assert_eq!(marked, ["M2", "M3", "Q2", "J2", "Z1"]);
        assert_eq!(find(&insts, "M2").model.as_deref(), Some("pch"));
        assert_eq!(find(&insts, "M3").model, None);
        // a card named in a different case, and one defined after the element, are found
        let (insts, _) = read_all("* t\nM1 d g s b NCH\nJ1 d g s Jn\n.model nch nmos\n.model JN njf\n.end\n");
        assert!(insts.iter().all(|i| !i.model_missing), "{:?}", insts.iter().map(|i| &i.name).collect::<Vec<_>>());
    }
}

#[cfg(test)]
mod comment_advice_tests {
    use super::*;

    #[test]
    fn compatibility_detection_distinguishes_comments_from_host_names_and_strings() {
        for (line, slash, expected) in [
            ("Rä a b 1k", false, false),
            ("R1 a b '中文 $ still an expression'", false, false),
            ("R1 a b {中文 ; still an expression}", false, false),
            ("include \"中文 // still a path\"", true, false),
            ("R1 a b 1k ; 中文", false, true),
            ("R1 a b 1k $ 中文", false, true),
            ("R1 (a b) resistor r=1k // 中文", true, true),
            ("  * 中文", false, true),
            ("// ASCII only", true, false),
        ] {
            let mut sink = Sink::new();
            assert_eq!(hint_comment_compatibility(line, slash, "test", 1, &mut sink), expected, "{line}");
            assert_eq!(sink.all().len(), usize::from(expected));
        }
    }
}
