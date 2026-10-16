//! Writing RNL back into host files (standard 2.2, 16.4, 17).
//!
//! A tool changes only its own records: a solver replaces the `type="generated"` records of the
//! level it solved, an editor appends its records after them. Everything else in the file, host
//! statements, ordinary comments and other records, stays byte for byte, line endings included.
//! New records go at the end of the level's own text, in the comment carrier the file already
//! uses for RNL. A file a tool saves has a header record with its version ([`with_header`],
//! 16.1).

use crate::host::{HostDocument, LevelId, LevelKind, Record};
use crate::syntax::{parse_record, Value};

/// The `type=` of a record's header: "authored" when not written.
pub fn record_type(rec: &Record) -> String {
    field(rec, "type").unwrap_or_else(|| "authored".into())
}

/// A string field of a record's header, such as `author`.
pub fn field(rec: &Record, name: &str) -> Option<String> {
    let ast = parse_record(&rec.text);
    let h = ast.header?;
    match h.get(name) {
        Some(Value::Str(s)) => Some(s.clone()),
        _ => None,
    }
}

/// The lines of a text with their line endings.
struct Lines {
    lines: Vec<String>,
    eol: &'static str,
}

impl Lines {
    fn new(text: &str) -> Lines {
        let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
        Lines { lines: text.split_inclusive('\n').map(String::from).collect(), eol }
    }

    fn body(s: &str) -> &str {
        s.trim_end_matches(['\n', '\r'])
    }

    fn ending(s: &str) -> &str {
        &s[Self::body(s).len()..]
    }

    fn text(&self) -> String {
        self.lines.concat()
    }

    /// Remove the characters from (line, col) to (line, col) inclusive, 1-based; drop the lines
    /// that are left holding nothing but a comment marker.
    fn remove(&mut self, start: (u32, u32), end: (u32, u32), marker: &str) {
        let (l0, c0) = (start.0 as usize - 1, start.1 as usize - 1);
        let (l1, c1) = (end.0 as usize - 1, end.1 as usize);
        if l1 >= self.lines.len() {
            return;
        }
        let first: Vec<char> = Self::body(&self.lines[l0]).chars().collect();
        let last_line = self.lines[l1].clone();
        let last: Vec<char> = Self::body(&last_line).chars().collect();
        let head: String = first[..c0.min(first.len())].iter().collect();
        let mut tail: String = last[c1.min(last.len())..].iter().collect();
        // tokens after the record on its line fail (2.4); without a record before them they would
        // be a host comment, so an empty record keeps them what they were
        if holds_tokens(&tail) {
            tail = format!("@RNL {{}} {}", tail.trim_start());
        }
        // a record removed from the middle of a carrier line leaves one marker and one space
        let merged = if !marker.is_empty() && head.trim() == marker && !tail.trim().is_empty() {
            format!("{marker} {}", tail.trim_start())
        } else {
            format!("{head}{tail}")
        };

        let ending = Self::ending(&last_line).to_string();
        let rest = merged.trim();
        let empty = rest.is_empty() || rest == marker || (marker.is_empty() && rest.is_empty());
        let replacement: Vec<String> = if empty { Vec::new() } else { vec![format!("{}{ending}", merged.trim_end())] };
        self.lines.splice(l0..=l1, replacement);
    }

    /// Insert `s` into a line before the character at `col`, 0-based, of its text.
    fn insert_in(&mut self, line: u32, col: usize, s: &str) {
        let Some(l) = self.lines.get_mut((line as usize).wrapping_sub(1)) else { return };
        let at = l.char_indices().nth(col).map_or(Self::body(l).len(), |(i, _)| i);
        l.insert_str(at, s);
    }

    fn insert(&mut self, before_line: u32, new: &[String]) {
        let at = (before_line as usize).saturating_sub(1).min(self.lines.len());
        if at == self.lines.len() {
            if let Some(last) = self.lines.last_mut() {
                if !last.ends_with('\n') {
                    last.push_str(self.eol);
                }
            }
        }
        let eol = self.eol;
        let added: Vec<String> = new.iter().map(|l| format!("{l}{eol}")).collect();
        self.lines.splice(at..at, added);
    }
}

/// Whether the text after a record on its line holds tokens, which fail there (2.4): anything but
/// white space, `/* … */` comments and a new record.
fn holds_tokens(tail: &str) -> bool {
    let mut t = tail.trim_start();
    while let Some(rest) = t.strip_prefix("/*") {
        match rest.find("*/") {
            Some(i) => t = rest[i + 2..].trim_start(),
            None => return false,
        }
    }
    !t.is_empty() && !t.starts_with("@RNL")
}

/// The comment marker that carries RNL in a file: what its records use, else the dialect's (2.2).
pub fn carrier(doc: &HostDocument, file: &str, text: &str) -> &'static str {
    if file.to_lowercase().ends_with(".rnl") {
        return "";
    }
    let lines: Vec<&str> = text.lines().collect();
    for lv in &doc.levels {
        for r in lv.records.iter().filter(|r| r.file == file) {
            if let Some(l) = lines.get(r.lines.0 as usize - 1) {
                if l.starts_with("//") {
                    return "//";
                }
                if l.starts_with('*') {
                    return "*";
                }
            }
        }
    }
    let spectre_mode = doc.dialect.name == "spectre"
        && (file.to_lowercase().ends_with(".scs")
            || text.lines().take(5).any(|l| l.trim().to_lowercase().starts_with("simulator") && l.contains("spectre")));
    if spectre_mode {
        "//"
    } else {
        "*"
    }
}

/// The records of a file, with the level each belongs to.
fn records_of<'a>(doc: &'a HostDocument, file: &str) -> Vec<(LevelId, &'a Record)> {
    let mut out = Vec::new();
    for (k, lv) in doc.levels.iter().enumerate() {
        for r in &lv.records {
            if r.file == file {
                out.push((k, r));
            }
        }
    }
    out
}

fn span(r: &Record) -> ((u32, u32), (u32, u32)) {
    let n = r.text.chars().count();
    let a = r.loc(0);
    let b = r.loc(n.saturating_sub(1));
    ((a.line, a.col), (b.line, b.col))
}

/// Remove the records of `file` that `pred` picks; everything else stays as it is.
pub fn remove_records(doc: &HostDocument, file: &str, text: &str, pred: impl Fn(LevelId, &Record) -> bool) -> String {
    let marker = carrier(doc, file, text);
    let mut lines = Lines::new(text);
    let mut spans: Vec<((u32, u32), (u32, u32))> =
        records_of(doc, file).into_iter().filter(|(lv, r)| pred(*lv, r)).map(|(_, r)| span(r)).collect();
    spans.sort();
    for (a, b) in spans.into_iter().rev() {
        lines.remove(a, b, marker);
    }
    lines.text()
}

/// Remove the solver's records (`type="generated"`) from a file: the drawing as its authors wrote it
/// (architecture 3.3).
pub fn strip_generated(doc: &HostDocument, file: &str, text: &str) -> String {
    remove_records(doc, file, text, |_, r| record_type(r) == "generated")
}

/// Format a record for a file: each line behind the comment marker and one space (2.2).
pub fn carried(record: &str, marker: &str) -> Vec<String> {
    record
        .lines()
        .map(|l| {
            if marker.is_empty() {
                l.to_string()
            } else if l.trim().is_empty() {
                marker.to_string()
            } else {
                format!("{marker} {l}")
            }
        })
        .collect()
}

/// Add a record at the end of a level's own text (17): before the `.ends` of a subcircuit, before
/// the `.end` of a main file, or at the end of the file. `text` is the text of the level's file.
pub fn append_record(doc: &HostDocument, level: LevelId, text: &str, record: &str) -> String {
    let lv = &doc.levels[level];
    let marker = carrier(doc, &lv.file, text);
    let mut lines = Lines::new(text);
    lines.insert(end_of(doc, level, &lines), &carried(record, marker));
    lines.text()
}

/// The line before which a record added to the end of a level goes (17).
fn end_of(doc: &HostDocument, level: LevelId, lines: &Lines) -> u32 {
    let lv = &doc.levels[level];
    if lv.end_line > 0 {
        lv.end_line
    } else {
        lines.lines.len() as u32 + 1
    }
}

/// Whether a record, written in the text of the level `written_in`, applies to `level` (2.6): one
/// without scope= to the level it is written in, one with scope= to the subcircuits of that name
/// below it. A scope= written inside a subcircuit is treated as not written (2.6, 2.8); one that
/// names no subcircuit below its level applies to none.
pub fn applies_to(doc: &HostDocument, written_in: LevelId, rec: &Record, level: LevelId) -> bool {
    let scope = field(rec, "scope").filter(|_| doc.levels[written_in].kind != LevelKind::Subckt);
    match scope {
        None => written_in == level,
        Some(target) => {
            let lv = &doc.levels[level];
            lv.kind == LevelKind::Subckt
                && doc.key(&lv.name) == doc.key(&target)
                && doc.ancestors(level).contains(&written_in)
        }
    }
}

/// The records of the solver that apply to a level, with the level each is written in: those of
/// `type="generated"` in its own text, and those written above it with scope= naming it. The
/// solver writes no scope= (2.6, 17), but such a record still applies to the level, and what
/// solving the level writes replaces it.
pub fn generated_for(doc: &HostDocument, level: LevelId) -> Vec<(LevelId, &Record)> {
    let mut out = Vec::new();
    for (k, lv) in doc.levels.iter().enumerate() {
        for r in &lv.records {
            if record_type(r) == "generated" && applies_to(doc, k, r, level) {
                out.push((k, r));
            }
        }
    }
    out
}

/// The files that hold records of the solver that apply to a level ([`generated_for`]), each
/// once, in the order of the levels.
pub fn generated_files(doc: &HostDocument, level: LevelId) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, r) in generated_for(doc, level) {
        if !out.contains(&r.file) {
            out.push(r.file.clone());
        }
    }
    out
}

/// Remove from `text`, the text of `file`, the records of the solver that apply to `level`
/// ([`generated_for`]), and nothing else.
pub fn remove_generated_for(doc: &HostDocument, level: LevelId, file: &str, text: &str) -> String {
    remove_records(doc, file, text, |k, r| record_type(r) == "generated" && applies_to(doc, k, r, level))
}

/// Replace the solver's records of one level by a new one at the end of its text (14.8, 17): what a
/// solver does after solving the level. `record` is the whole `@RNL(type="generated", …) {…}`;
/// `text` is the text of the level's file. The records of the solver removed are those that apply
/// to the level ([`generated_for`]) and are written in its file; one written with scope= in
/// another file is removed from that file's text by [`remove_generated_for`]. A record with scope=
/// written in the level's text for a subcircuit below it stays: it is that subcircuit's.
pub fn replace_generated(doc: &HostDocument, level: LevelId, text: &str, record: &str) -> String {
    let file = doc.levels[level].file.clone();
    let marker = carrier(doc, &file, text);
    let mut lines = Lines::new(text);
    let at = end_of(doc, level, &lines);
    let mut spans: Vec<((u32, u32), (u32, u32))> =
        generated_for(doc, level).into_iter().filter(|(_, r)| r.file == file).map(|(_, r)| span(r)).collect();
    spans.sort();
    // from the end of the text up, so that each change leaves the lines above it where they were:
    // the records below the end of the level (written with scope= in the file level after a
    // subcircuit), the new record, the records above
    for (a, b) in spans.iter().rev().filter(|(a, _)| a.0 >= at) {
        lines.remove(*a, *b, marker);
    }
    lines.insert(at, &carried(record, marker));
    for (a, b) in spans.iter().rev().filter(|(a, _)| a.0 < at) {
        lines.remove(*a, *b, marker);
    }
    lines.text()
}

/// A file that a tool saves has its header record and its version (16.1): `text`, the text of
/// `file`, with `@RNL(type="header") { @VERSION("…")}` added as its first record when the file has
/// no header record, or with VERSION added to its header record when that has none. `version` is
/// the version the file was read in, so that it reads the same. A file whose header record has its
/// version, one whose header record is not its first record (an error its author fixes), and a
/// package file, which has a header by rule (16.2), come back as they are.
///
/// The first line of a host file never carries RNL (2.2): it is the title of the main file, and
/// in an included file a comment or a statement. The header goes on the line after a title or a
/// comment; before a statement, a comment line, the bare marker, is put first, so that the
/// statement keeps its meaning on the third line.
pub fn with_header(doc: &HostDocument, file: &str, text: &str, version: &str) -> String {
    if file.to_lowercase().ends_with(".rnl") {
        return text.to_string();
    }
    let version_kw = format!("@VERSION({})", crate::problem::written_string(version));
    let records = records_of(doc, file);
    if let Some((_, first)) = records.iter().min_by_key(|(_, r)| span(r).0) {
        if record_type(first) != "header" {
            if records.iter().any(|(_, r)| record_type(r) == "header") {
                return text.to_string();
            }
        } else {
            let ast = parse_record(&first.text);
            let Some(body) = ast.body else { return text.to_string() };
            let call =
                |m: &crate::syntax::Member| m.segments.first().and_then(|s| s.call.as_ref()).map(|c| c.kw.clone());
            if body.members.iter().any(|m| call(m).as_deref() == Some("VERSION")) {
                return text.to_string();
            }
            // right after the `{` of the body
            let at = first.loc(body.start);
            let sep = if body.members.is_empty() { "" } else { ";" };
            let mut lines = Lines::new(text);
            lines.insert_in(at.line, at.col as usize, &format!(" {version_kw}{sep}"));
            return lines.text();
        }
    }
    let marker = carrier(doc, file, text);
    let header = carried(&format!("@RNL(type=\"header\") {{ {version_kw}}}"), marker);
    let mut lines = Lines::new(text);
    let first = lines.lines.first().map(|l| Lines::body(l).to_string());
    let main = doc.levels.first().is_some_and(|l| l.file == file);
    let comment = |l: &str| l.starts_with('*') || (doc.dialect.name == "spectre" && l.starts_with("//"));
    match first {
        Some(l) if main || l.trim().is_empty() || comment(&l) => lines.insert(2, &header),
        _ => lines.insert(1, &[vec![marker.to_string()], header].concat()),
    }
    lines.text()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{Dialect, Level, LevelKind};
    use crate::records::{scan_run, CarrierLine};

    /// A tiny SPICE reader for these tests: `*` carriers, `.subckt`/`.ends` levels.
    fn doc(text: &str, file: &str, marker: &str) -> HostDocument {
        let mut main = Level::new(LevelKind::File, file, file, file);
        main.circuit = true;
        let mut d = HostDocument::new(
            main,
            Dialect {
                name: if marker == "//" { "spectre".into() } else { "spice".into() },
                case_sensitive: false,
                ground_aliases: vec![],
            },
        );
        let mut sink = crate::diag::Sink::new();
        let mut cur = 0;
        let mut run: Vec<CarrierLine> = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        for (k, l) in lines.iter().enumerate() {
            let no = k as u32 + 1;
            if k > 0 {
                if let Some(rest) = l.strip_prefix(marker) {
                    if rest.is_empty() || rest.starts_with(' ') {
                        run.push(CarrierLine { line: no, content: rest.to_string(), col: marker.len() as i64 });
                        continue;
                    }
                }
            }
            if !run.is_empty() {
                let recs = scan_run(&run, file, &mut sink);
                d.levels[cur].records.extend(recs);
                run.clear();
            }
            let low = l.trim().to_lowercase();
            if low.starts_with(".subckt") || low.starts_with("subckt") {
                let mut s = Level::new(LevelKind::Subckt, "s", file, file);
                s.circuit = true;
                cur = d.add_level(s, Some(0));
            } else if low.starts_with(".ends") || low.starts_with("ends") {
                d.levels[cur].end_line = no;
                cur = 0;
            } else if low == ".end" {
                d.levels[0].end_line = no;
            }
        }
        if d.levels[0].end_line == 0 {
            d.levels[0].end_line = lines.len() as u32 + 1;
        }
        d
    }

    const NET: &str = "* t\r\nR1 a b 1k\r\n* note @RNL {#R1 @PLACE(0, 0)}\r\n* @RNL(type=\"generated\") {\r\n*   #R1 @PLACE(1, 2)\r\n* } @RNL {#R1}\r\n* @RNL(type=\"generated\", priority=\"high\") { #R1 @ORIENT(\"R90\")}\r\n.end\r\n";

    #[test]
    fn strip_keeps_everything_else() {
        let d = doc(NET, "a.sp", "*");
        let out = strip_generated(&d, "a.sp", NET);
        assert_eq!(out, "* t\r\nR1 a b 1k\r\n* note @RNL {#R1 @PLACE(0, 0)}\r\n* @RNL {#R1}\r\n.end\r\n");
        // nothing picked: the text comes back unchanged
        assert_eq!(remove_records(&d, "a.sp", NET, |_, _| false), NET);
    }

    #[test]
    fn replace_in_a_subcircuit() {
        let net =
            "* t\n.subckt s a b\nR1 a b 1k\n* @RNL(type=\"generated\") {#R1 @PLACE(0, 0)}\n* @RNL {#R1}\n.ends\n.end\n";
        let d = doc(net, "a.sp", "*");
        let out = replace_generated(&d, 1, net, "@RNL(type=\"generated\", priority=\"high\") {\n  #R1 @PLACE(4, 4)\n}");
        assert_eq!(out, "* t\n.subckt s a b\nR1 a b 1k\n* @RNL {#R1}\n* @RNL(type=\"generated\", priority=\"high\") {\n*   #R1 @PLACE(4, 4)\n* }\n.ends\n.end\n");
    }

    #[test]
    fn tokens_after_a_record_removed_stay_an_error() {
        // `#R1 @PLACE(0, 0)` after the record fails (2.4): an empty record keeps it so, where it
        // would otherwise be a host comment; a comment after a record stays one
        let net = "* t\nR1 a b 1k\n* @RNL(type=\"generated\") {\n*   #R1 @PLACE(1, 2)\n* } #R1 @PLACE(0, 0)\n\
                   * @RNL(type=\"generated\") {#R1} /* note */\n.end\n";
        let d = doc(net, "a.sp", "*");
        let out = strip_generated(&d, "a.sp", net);
        assert_eq!(out, "* t\nR1 a b 1k\n* @RNL {} #R1 @PLACE(0, 0)\n* /* note */\n.end\n");
        assert!(holds_tokens(" /* a */ #R1") && !holds_tokens(" /* a */ ") && !holds_tokens(" @RNL {#R1}"));
        assert!(!holds_tokens(" /* runs on") && !holds_tokens("  "));
    }

    #[test]
    fn the_records_of_the_solver_written_with_scope_go_with_their_subcircuit() {
        // inside the subcircuit, scope= is treated as not written (2.6, 2.8); at the file level, the
        // first record applies to s, the second to the file level
        let net = "* t\n.subckt s a b\nR1 a b 1k\n* @RNL(type=\"generated\", scope=\"s\") {#R1 @PLACE(9, 9)}\n.ends\n\
                   * @RNL(type=\"generated\", scope=\"S\") {#R1 @PLACE(0, 0)}\n* @RNL(type=\"generated\") {#X1 @PLACE(1, 1)}\n\
                   * @RNL(type=\"generated\", scope=\"none\") {#R1}\nX1 a b s\n.end\n";
        let d = doc(net, "a.sp", "*");
        let at = |lv: LevelId| generated_for(&d, lv).iter().map(|(_, r)| r.lines.0).collect::<Vec<_>>();
        // in the order of the levels they are written in
        assert_eq!(at(1), [6, 4]);
        assert_eq!(at(0), [7]);
        assert_eq!(generated_files(&d, 1), ["a.sp"]);
        let out = replace_generated(&d, 1, net, "@RNL(type=\"generated\") {#R1 @PLACE(2, 2)}");
        assert_eq!(
            out,
            "* t\n.subckt s a b\nR1 a b 1k\n* @RNL(type=\"generated\") {#R1 @PLACE(2, 2)}\n.ends\n\
             * @RNL(type=\"generated\") {#X1 @PLACE(1, 1)}\n* @RNL(type=\"generated\", scope=\"none\") {#R1}\nX1 a b s\n.end\n"
        );
        // the file level keeps the record written for s, and the one that applies to nothing
        let out = replace_generated(&d, 0, net, "@RNL(type=\"generated\") {#X1 @PLACE(3, 3)}");
        assert_eq!(
            out,
            "* t\n.subckt s a b\nR1 a b 1k\n* @RNL(type=\"generated\", scope=\"s\") {#R1 @PLACE(9, 9)}\n.ends\n\
             * @RNL(type=\"generated\", scope=\"S\") {#R1 @PLACE(0, 0)}\n* @RNL(type=\"generated\", scope=\"none\") {#R1}\n\
             X1 a b s\n* @RNL(type=\"generated\") {#X1 @PLACE(3, 3)}\n.end\n"
        );
        assert_eq!(remove_generated_for(&d, 0, "a.sp", net), strip_except(&d, net, &[4, 6, 8]));
    }

    /// The text without the records of the solver, but those that begin on these lines.
    fn strip_except(d: &HostDocument, net: &str, keep: &[u32]) -> String {
        remove_records(d, "a.sp", net, |_, r| record_type(r) == "generated" && !keep.contains(&r.lines.0))
    }

    #[test]
    fn a_file_saved_has_its_header_and_its_version() {
        let header = "* @RNL(type=\"header\") { @VERSION(\"0.2\")}";
        // the main file: after its title, in its line endings
        let net = "* t\r\nR1 a b 1k\r\n.end\r\n";
        let d = doc(net, "a.sp", "*");
        assert_eq!(with_header(&d, "a.sp", net, "0.2"), format!("* t\r\n{header}\r\nR1 a b 1k\r\n.end\r\n"));
        // with a header that has its version, as it is; one without a version gets it
        let with = format!("* t\n{header}\nR1 a b 1k\n.end\n");
        assert_eq!(with_header(&doc(&with, "a.sp", "*"), "a.sp", &with, "0.2"), with);
        for (body, want) in [("{}", "{ @VERSION(\"0.2\")}"), ("{ @DEVICE(x) }", "{ @VERSION(\"0.2\"); @DEVICE(x) }")] {
            let net = format!("* t\n* @RNL(type=\"header\") {body}\n.end\n");
            let out = with_header(&doc(&net, "a.sp", "*"), "a.sp", &net, "0.2");
            assert_eq!(out, format!("* t\n* @RNL(type=\"header\") {want}\n.end\n"));
        }
        // a header that is not the first record is left to its author
        let late = "* t\n* @RNL {#R1}\n* @RNL(type=\"header\") {}\n.end\n";
        assert_eq!(with_header(&doc(late, "a.sp", "*"), "a.sp", late, "0.2"), late);
        // an included file: after a comment on its first line; before a statement there, a comment
        // line first, so that the statement stays a statement
        let mut d = doc("* t\n.end\n", "a.sp", "*");
        d.add_level(Level::new(LevelKind::File, "lib.inc", "lib.inc", "lib.inc"), Some(0));
        let lib = "* lib\n.subckt div p n\nR1 p n 1k\n.ends\n";
        assert_eq!(
            with_header(&d, "lib.inc", lib, "0.2"),
            format!("* lib\n{header}\n.subckt div p n\nR1 p n 1k\n.ends\n")
        );
        let lib = ".subckt div p n\nR1 p n 1k\n.ends";
        assert_eq!(with_header(&d, "lib.inc", lib, "0.2"), format!("*\n{header}\n.subckt div p n\nR1 p n 1k\n.ends"));
        // Spectre's carrier; a package file as it is
        let net = "// t\nR1 (a b) resistor r=1k\n";
        let d = doc(net, "a.scs", "//");
        assert_eq!(
            with_header(&d, "a.scs", net, "0.2"),
            "// t\n// @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 (a b) resistor r=1k\n"
        );
        assert_eq!(with_header(&d, "p.rnl", "@RNL {}\n", "0.2"), "@RNL {}\n");
    }

    #[test]
    fn spectre_carrier_and_end_of_file() {
        let net = "// t\nR1 (a b) resistor r=1k";
        let d = doc(net, "a.scs", "//");
        let out = append_record(&d, 0, net, "@RNL {#R1 @PLACE(0, 0)}");
        assert_eq!(out, "// t\nR1 (a b) resistor r=1k\n// @RNL {#R1 @PLACE(0, 0)}\n");
    }
}
