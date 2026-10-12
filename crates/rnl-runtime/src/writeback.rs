//! Writing RNL back into host files (spec-draft 2.2, 16.4, 17).
//!
//! A tool changes only its own records: a solver replaces the `type="generated"` records of the
//! level it solved, an editor appends its records after them. Everything else in the file, host
//! statements, ordinary comments and other records, stays byte for byte, line endings included.
//! New records go at the end of the level's own text, in the comment carrier the file already
//! uses for RNL.

use crate::host::{HostDocument, LevelId, Record};
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
        let tail: String = last[c1.min(last.len())..].iter().collect();
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
    let at = if lv.end_line > 0 { lv.end_line } else { lines.lines.len() as u32 + 1 };
    lines.insert(at, &carried(record, marker));
    lines.text()
}

/// Replace the solver's records of one level by a new one at the end of its text (14.8, 17): what a
/// solver does after solving the level. `record` is the whole `@RNL(type="generated", …) {…}`.
pub fn replace_generated(doc: &HostDocument, level: LevelId, text: &str, record: &str) -> String {
    let lv = &doc.levels[level];
    let file = lv.file.clone();
    // insert first: the removals are all above the end of the level
    let with = append_record(doc, level, text, record);
    let marker = carrier(doc, &file, text);
    let mut lines = Lines::new(&with);
    let mut spans: Vec<((u32, u32), (u32, u32))> =
        lv.records.iter().filter(|r| r.file == file && record_type(r) == "generated").map(span).collect();
    spans.sort();
    for (a, b) in spans.into_iter().rev() {
        lines.remove(a, b, marker);
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
    fn spectre_carrier_and_end_of_file() {
        let net = "// t\nR1 (a b) resistor r=1k";
        let d = doc(net, "a.scs", "//");
        let out = append_record(&d, 0, net, "@RNL {#R1 @PLACE(0, 0)}");
        assert_eq!(out, "// t\nR1 (a b) resistor r=1k\n// @RNL {#R1 @PLACE(0, 0)}\n");
    }
}
