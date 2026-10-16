//! Writing RNL back into host files (standard 2.2, 16.4, 17).
//!
//! A tool changes only its own records: a solver replaces the `type="generated"` records of the
//! level it solved, an editor appends its records after them. Everything else in the file, host
//! statements, ordinary comments and other records, stays byte for byte, line endings included.
//! New records go at the end of the level's own text, in the comment carrier the file already
//! uses for RNL. A file a tool saves has a header record with its version ([`with_header`],
//! 16.1).
//!
//! Every change is made at the positions of the document's reading of the text, from the end of
//! the text up, so that each leaves the places of those above it as they were: the records of a
//! file, its header among them, are found where the reading found them, however many lines the
//! other changes add or remove. A solver that writes a level back and gives its file its header
//! makes both changes in one pass ([`replace_generated_with_header`],
//! [`remove_generated_with_header`]).

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

/// One change to a text, at a place that the positions of the document's reading describe.
enum Edit {
    /// Remove the characters from one (line, column) to another inclusive, 1-based: a record.
    Remove((u32, u32), (u32, u32)),
    /// Insert lines, each without its line ending, before a line, 1-based.
    Lines(u32, Vec<String>),
    /// Insert text into a line, 1-based, before a character, 0-based.
    Text(u32, usize, String),
}

impl Edit {
    /// Where it changes the text: the line, and the column of the first character it touches,
    /// 1-based, 0 for lines inserted before the line.
    fn at(&self) -> (u32, usize) {
        match self {
            Edit::Remove(a, _) => (a.0, a.1 as usize),
            Edit::Lines(line, _) => (*line, 0),
            Edit::Text(line, col, _) => (*line, col + 1),
        }
    }
}

/// Make the changes to a text from its end up, so that each leaves the places of those above it
/// where the reading found them. Of two changes at one place, the one given first ends up first.
fn apply(text: &str, marker: &str, edits: Vec<Edit>) -> String {
    let mut edits: Vec<(usize, Edit)> = edits.into_iter().enumerate().collect();
    edits.sort_by(|(i, a), (j, b)| b.at().cmp(&a.at()).then(j.cmp(i)));
    let mut lines = Lines::new(text);
    for (_, e) in edits {
        match e {
            Edit::Remove(a, b) => lines.remove(a, b, marker),
            Edit::Lines(before, new) => lines.insert(before, &new),
            Edit::Text(line, col, s) => lines.insert_in(line, col, &s),
        }
    }
    lines.text()
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
    apply(text, carrier(doc, file, text), removals(doc, file, pred))
}

/// The removal of each record of `file` that `pred` picks.
fn removals(doc: &HostDocument, file: &str, pred: impl Fn(LevelId, &Record) -> bool) -> Vec<Edit> {
    records_of(doc, file)
        .into_iter()
        .filter(|(lv, r)| pred(*lv, r))
        .map(|(_, r)| {
            let (a, b) = span(r);
            Edit::Remove(a, b)
        })
        .collect()
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
    apply(text, carrier(doc, file, text), generated_removals(doc, level, file))
}

/// [`remove_generated_for`], and the file given its header record and its version as
/// [`with_header`] gives them, in one pass at the positions of the document's reading of `text`:
/// what a solver writes into a file that holds a record of the solver for the level it solved,
/// written there with scope= (2.6), and that it saves (16.1).
pub fn remove_generated_with_header(
    doc: &HostDocument,
    level: LevelId,
    file: &str,
    text: &str,
    version: &str,
) -> String {
    let (mut edits, _) = header_edits(doc, file, text, version);
    edits.extend(generated_removals(doc, level, file));
    apply(text, carrier(doc, file, text), edits)
}

/// The removal of each record of the solver in `file` that applies to `level`.
fn generated_removals(doc: &HostDocument, level: LevelId, file: &str) -> Vec<Edit> {
    removals(doc, file, |k, r| record_type(r) == "generated" && applies_to(doc, k, r, level))
}

/// Replace the solver's records of one level by a new one at the end of its text (14.8, 17): what a
/// solver does after solving the level. `record` is the whole `@RNL(type="generated", …) {…}`;
/// `text` is the text of the level's file. The records of the solver removed are those that apply
/// to the level ([`generated_for`]) and are written in its file; one written with scope= in
/// another file is removed from that file's text by [`remove_generated_for`]. A record with scope=
/// written in the level's text for a subcircuit below it stays: it is that subcircuit's.
pub fn replace_generated(doc: &HostDocument, level: LevelId, text: &str, record: &str) -> String {
    let file = &doc.levels[level].file;
    let marker = carrier(doc, file, text);
    apply(text, marker, replacement(doc, level, text, record, marker))
}

/// [`replace_generated`], and the level's file given its header record and its version as
/// [`with_header`] gives them, in one pass at the positions of the document's reading of `text`:
/// what a solver writes into the file of a level it solved, which it saves (16.1). The header
/// record stays the first record of its file (2.8): one written after a subcircuit solved, which
/// the record put at the end of the subcircuit would come before, moves to the line above the
/// subcircuit, the outermost one in the file that holds the level, with its version, and is
/// removed where the reading found it; no record comes before it there, as none came before it
/// where it was. The record follows the carrier of a header written in the same pass (2.2), so
/// that the next solve, which finds that header, writes it the same.
pub fn replace_generated_with_header(
    doc: &HostDocument,
    level: LevelId,
    text: &str,
    record: &str,
    version: &str,
) -> String {
    let file = &doc.levels[level].file;
    let (mut edits, marker) = header_edits(doc, file, text, version);
    let at = end_of(doc, level, &Lines::new(text));
    if let (Some(header), Some(above)) = (first_header(doc, file), subckt_start(doc, level)) {
        if span(header).0 .0 >= at {
            // the version it gets where it is goes with it
            edits = moved_header(header, version, marker, above);
        }
    }
    edits.extend(replacement(doc, level, text, record, marker));
    apply(text, carrier(doc, file, text), edits)
}

/// The first line of the outermost subcircuit, in the file of `level`, that holds `level` or is
/// it: the record written at the end of `level` lies inside it. None for a file level.
fn subckt_start(doc: &HostDocument, level: LevelId) -> Option<u32> {
    let file = &doc.levels[level].file;
    let mut start = None;
    let mut at = Some(level);
    while let Some(k) = at {
        let lv = &doc.levels[k];
        if lv.kind != LevelKind::Subckt || lv.file != *file {
            break;
        }
        start = Some(lv.line);
        at = lv.parent;
    }
    start.filter(|&line| line > 0)
}

/// The changes that move a header record to the line `above`, carried by `marker`, with VERSION
/// when it has none: removed where the reading found it, and written whole there, each of its
/// lines after the first as it was after its marker, so that its indentation stays.
fn moved_header(header: &Record, version: &str, marker: &str, above: u32) -> Vec<Edit> {
    let mut text = header.text.clone();
    if let Some((brace, s)) = version_insert(header, version) {
        let at = text.char_indices().nth(brace + 1).map_or(text.len(), |(b, _)| b);
        text.insert_str(at, &s);
    }
    let lines = text
        .split('\n')
        .enumerate()
        .map(|(k, l)| if k == 0 { format!("{marker} {l}") } else { format!("{marker}{l}") });
    let (a, b) = span(header);
    vec![Edit::Remove(a, b), Edit::Lines(above, lines.collect())]
}

/// Where VERSION goes into a header record that has none: after the `{` of its body, given as the
/// offset of that `{` in its text, with what goes there. None when it has a VERSION, or no body.
fn version_insert(header: &Record, version: &str) -> Option<(usize, String)> {
    let ast = parse_record(&header.text);
    let body = ast.body?;
    let call = |m: &crate::syntax::Member| m.segments.first().and_then(|s| s.call.as_ref()).map(|c| c.kw.clone());
    if body.members.iter().any(|m| call(m).as_deref() == Some("VERSION")) {
        return None;
    }
    let sep = if body.members.is_empty() { "" } else { ";" };
    Some((body.start, format!(" @VERSION({}){sep}", crate::problem::written_string(version))))
}

/// The records of a file as a reading of it takes them (2.8), each with its level: those of the
/// file level that reads it, one that reads it whole before one that reads a section of it, and of
/// the subcircuits written in it; a section of `.lib` that is read counts as a file. With that file
/// level, None when no file level reads the file.
fn file_records<'a>(doc: &'a HostDocument, file: &str) -> (Option<LevelId>, Vec<(LevelId, &'a Record)>) {
    let mut levels: Vec<LevelId> = (0..doc.levels.len())
        .filter(|&l| doc.levels[l].kind == LevelKind::File && doc.levels[l].file == file)
        .collect();
    levels.sort_by_key(|&l| doc.levels[l].section.is_some());
    let Some(&top) = levels.first() else {
        return (None, records_of(doc, file));
    };
    let in_top = |mut l: LevelId| loop {
        let lv = &doc.levels[l];
        if lv.kind == LevelKind::File {
            return l == top;
        }
        match lv.parent {
            Some(p) => l = p,
            None => return false,
        }
    };
    let records = (0..doc.levels.len())
        .filter(|&l| doc.levels[l].file == file && in_top(l))
        .flat_map(|l| doc.levels[l].records.iter().filter(|r| r.file == file).map(move |r| (l, r)))
        .collect();
    (Some(top), records)
}

/// The header record of a file, when it is the first record of the file as the reading takes it,
/// written at its file level ([`file_records`], 2.8); a package file has a header by rule (16.2)
/// and gives none.
fn first_header<'a>(doc: &'a HostDocument, file: &str) -> Option<&'a Record> {
    if file.to_lowercase().ends_with(".rnl") {
        return None;
    }
    let (top, records) = file_records(doc, file);
    let first = records.into_iter().min_by_key(|(_, r)| span(r).0);
    first.filter(|(l, r)| top.is_none_or(|t| t == *l) && record_type(r) == "header").map(|(_, r)| r)
}

/// The changes of [`replace_generated`]: the removal of the level's records of the solver in its
/// file, and the new record at the end of the level (17). A record of the solver below the end of
/// the level, written with scope= in the file level after a subcircuit, is removed before the new
/// record goes in, those above after it.
fn replacement(doc: &HostDocument, level: LevelId, text: &str, record: &str, marker: &str) -> Vec<Edit> {
    let file = &doc.levels[level].file;
    let at = end_of(doc, level, &Lines::new(text));
    let mut edits = generated_removals(doc, level, file);
    edits.push(Edit::Lines(at, carried(record, marker)));
    edits
}

/// A file that a tool saves has its header record and its version (16.1): `text`, the text of
/// `file`, with `@RNL(type="header") { @VERSION("…")}` added as its first record when the file has
/// no header record, or with VERSION added to its header record when that has none. `version` is
/// the version the file was read in, so that it reads the same. A file whose header record has its
/// version, one whose header record is not the first record of the file (an error its author
/// fixes), and a package file, which has a header by rule (16.2), come back as they are. The
/// header record is the first record of the whole file, as the reading takes it (2.8): a record of
/// a subcircuit written above it comes before it, and a section of `.lib` that is read counts as a
/// file ([`file_records`]).
///
/// The first line of a host file never carries RNL (2.2): it is the title of the main file, and
/// in an included file a comment or a statement. The header goes on the line after a title or a
/// comment; before a statement, a comment line, the bare marker, is put first, so that the
/// statement keeps its meaning on the third line. Spectre reads a file that is not a `.scs` file
/// in SPICE mode until its `simulator lang=spectre`, and `//` is no comment there (2.2): a header
/// carried by `//` goes after that statement, and after the lines that continue it, when no other
/// statement and no record comes before it; otherwise it goes above the statement, written with
/// `*`, which carries in both modes, and so does the bare marker.
///
/// A file read only through library sections (`.lib file sec`, Spectre's `include "file"
/// section=sec`, [`crate::host::Level::section`]) is read inside them alone, and tools must not
/// write outside them (2.2): it gets no new header record, and takes the version of the file that
/// includes it (16.1). A header record that the reading found inside a section gets its version
/// there.
///
/// `text` is the text whose positions the document gives, as the reading found it. To give a file
/// its header together with the records a solver writes or removes, use
/// [`replace_generated_with_header`] or [`remove_generated_with_header`], which make every change
/// at those positions.
pub fn with_header(doc: &HostDocument, file: &str, text: &str, version: &str) -> String {
    apply(text, carrier(doc, file, text), header_edits(doc, file, text, version).0)
}

/// The changes that give a file its header record and its version ([`with_header`]): none when it
/// has them, or is left as it is; and the carrier that records written with them follow: that of
/// a new header, else the file's ([`carrier`]).
fn header_edits(doc: &HostDocument, file: &str, text: &str, version: &str) -> (Vec<Edit>, &'static str) {
    let marker = carrier(doc, file, text);
    if file.to_lowercase().ends_with(".rnl") {
        return (Vec::new(), marker);
    }
    let version_kw = format!("@VERSION({})", crate::problem::written_string(version));
    let all = records_of(doc, file);
    match first_header(doc, file) {
        Some(first) => {
            // right after the `{` of the body
            let edit = version_insert(first, version).map(|(brace, s)| {
                let at = first.loc(brace);
                Edit::Text(at.line, at.col as usize, s)
            });
            return (edit.into_iter().collect(), marker);
        }
        None if all.iter().any(|(_, r)| record_type(r) == "header") => return (Vec::new(), marker),
        None => {}
    }
    // read only through library sections: line 2 lies outside each of them, and is not read (2.2)
    if file_records(doc, file).0.is_some_and(|top| doc.levels[top].section.is_some()) {
        return (Vec::new(), marker);
    }
    let header = |m: &str| carried(&format!("@RNL(type=\"header\") {{ {version_kw}}}"), m);
    let lines = Lines::new(text);
    let main = doc.levels.first().is_some_and(|l| l.file == file);
    let spectre = doc.dialect.name == "spectre";
    let mut marker = marker;
    if spectre && marker == "//" && !file.to_lowercase().ends_with(".scs") {
        match after_spectre_switch(&lines, usize::from(main)) {
            // a record above the switch is read: the header goes before it, to be the first (2.8)
            Some(after) if !all.iter().any(|(_, r)| r.lines.0 < after) => {
                return (vec![Edit::Lines(after, header(marker))], marker);
            }
            // above the switch, in SPICE mode, where only `*` is a comment (2.2)
            _ => marker = "*",
        }
    }
    let comment = |l: &str| l.starts_with('*') || (spectre && l.starts_with("//"));
    let edit = match lines.lines.first().map(|l| Lines::body(l)) {
        Some(l) if main || l.trim().is_empty() || comment(l) => Edit::Lines(2, header(marker)),
        _ => Edit::Lines(1, [vec![marker.to_string()], header(marker)].concat()),
    };
    (vec![edit], marker)
}

/// The line, 1-based, after a file's `simulator lang=spectre` and the lines that continue it (a
/// line ending with `\`), when that statement is the first one from the line `from` on, 0-based,
/// past blank and comment lines; where Spectre reads a file that is not a `.scs` file in Spectre
/// mode from (2.2).
fn after_spectre_switch(lines: &Lines, from: usize) -> Option<u32> {
    let body = |k: usize| lines.lines.get(k).map(|l| Lines::body(l).trim());
    let mut k = from;
    while let Some(l) = body(k) {
        if l.is_empty() || l.starts_with('*') || l.starts_with("//") {
            k += 1;
            continue;
        }
        let low = l.to_lowercase();
        if !(low.starts_with("simulator") && low.contains("spectre")) {
            return None;
        }
        while body(k).is_some_and(|l| l.ends_with('\\')) {
            k += 1;
        }
        return u32::try_from(k + 2).ok();
    }
    None
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
                s.line = no;
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
    fn a_header_below_a_subcircuit_solved_moves_above_it() {
        // a header record after a subcircuit: the record of the subcircuit goes in before its
        // `.ends`, and would come before the header, which must be the first record of the file
        // (2.8); so the header moves above the subcircuit, with its VERSION (16.1, 17)
        let net = "* t\n.subckt s a b\nR1 a b 1k\n.ends\n* @RNL(type=\"header\") { }\nX1 a b s\n.end\n";
        let d = doc(net, "a.sp", "*");
        let record = "@RNL(type=\"generated\") {\n  #R1 @PLACE(0, 0)\n}";
        let out = replace_generated_with_header(&d, 1, net, record, "0.2");
        let moved = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n.subckt s a b\nR1 a b 1k\n\
                     * @RNL(type=\"generated\") {\n*   #R1 @PLACE(0, 0)\n* }\n.ends\nX1 a b s\n.end\n";
        assert_eq!(out, moved);
        // the next level written finds it whole, and leaves it where it is
        assert_eq!(with_header(&doc(&out, "a.sp", "*"), "a.sp", &out, "0.2"), out);
        let bare = out.replace(" @VERSION(\"0.2\")", "");
        assert_eq!(with_header(&doc(&bare, "a.sp", "*"), "a.sp", &bare, "0.2"), out);
        let again = replace_generated_with_header(&doc(&out, "a.sp", "*"), 1, &out, record, "0.2");
        assert_eq!(again, out);
        // the file level written first: its record goes at its end, below the header, which gets its
        // version where it is; the subcircuit written next moves it
        let top = replace_generated_with_header(&d, 0, net, "@RNL(type=\"generated\") {}", "0.2");
        let kept = "* t\n.subckt s a b\nR1 a b 1k\n.ends\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\nX1 a b s\n\
                    * @RNL(type=\"generated\") {}\n.end\n";
        assert_eq!(top, kept);
        let both = replace_generated_with_header(&doc(&top, "a.sp", "*"), 1, &top, record, "0.2");
        assert_eq!(both, moved.replace(".end\n", "* @RNL(type=\"generated\") {}\n.end\n"));
        // a header on several lines keeps its indentation where it goes
        let long = net.replace("{ }", "{\n*   @DEVICE(x)\n* }");
        let out = replace_generated_with_header(&doc(&long, "a.sp", "*"), 1, &long, record, "0.2");
        let header = "* @RNL(type=\"header\") { @VERSION(\"0.2\");\n*   @DEVICE(x)\n* }\n.subckt s a b\n";
        assert!(out.starts_with(&format!("* t\n{header}")) && out.matches("type=\"header\"").count() == 1, "{out}");
        // a header after a record of a subcircuit is not the first record of the file: an error
        // its author fixes, left as it is
        let late = net.replace("R1 a b 1k\n", "R1 a b 1k\n* @RNL {#R1}\n");
        assert_eq!(with_header(&doc(&late, "a.sp", "*"), "a.sp", &late, "0.2"), late);
        let out = replace_generated_with_header(&doc(&late, "a.sp", "*"), 1, &late, record, "0.2");
        assert_eq!(
            out,
            late.replace("* @RNL {#R1}\n", "* @RNL {#R1}\n* @RNL(type=\"generated\") {\n*   #R1 @PLACE(0, 0)\n* }\n")
        );
        // without a header, at the end of a level on line 2: the header first, then the record
        let net = "* t\n.end\n";
        let out = replace_generated_with_header(&doc(net, "a.sp", "*"), 0, net, "@RNL(type=\"generated\") {}", "0.2");
        assert_eq!(out, "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n* @RNL(type=\"generated\") {}\n.end\n");
        // a record with scope= removed from below a header that has no version
        let net =
            "* t\n* @RNL(type=\"header\") {}\n* @RNL(type=\"generated\", scope=\"s\") {\n*   #R1 @PLACE(0, 0)\n* }\n\
                   .subckt s a b\nR1 a b 1k\n.ends\n.end\n";
        let out = remove_generated_with_header(&doc(net, "a.sp", "*"), 1, "a.sp", net, "0.2");
        assert_eq!(out, "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.subckt s a b\nR1 a b 1k\n.ends\n.end\n");
    }

    #[test]
    fn a_header_in_slashes_goes_after_the_switch_to_spectre() {
        // a file that is not a .scs file is read in SPICE mode until `simulator lang=spectre`, where
        // `//` carries nothing (2.2): the header goes after it and the lines that continue it
        let header = "// @RNL(type=\"header\") { @VERSION(\"0.2\")}";
        let mut d = doc("// t\nX1 (a b) div\n", "a.scs", "//");
        d.add_level(Level::new(LevelKind::File, "lib.inc", "lib.inc", "lib.inc"), Some(0));
        let body = "subckt div (p n)\nR1 (p n) resistor r=1k\nends div\n";
        for (head, want) in [
            ("simulator lang=spectre\n", format!("simulator lang=spectre\n{header}\n")),
            ("* lib\n\nsimulator lang=spectre\n", format!("* lib\n\nsimulator lang=spectre\n{header}\n")),
            (
                "simulator lang=spectre \\\n  insensitive=no\n",
                format!("simulator lang=spectre \\\n  insensitive=no\n{header}\n"),
            ),
        ] {
            let lib = format!("{head}{body}");
            assert_eq!(carrier(&d, "lib.inc", &lib), "//");
            assert_eq!(with_header(&d, "lib.inc", &lib, "0.2"), format!("{want}{body}"), "{head}");
        }
        // the main file, after its title
        let net = "* t\nsimulator lang=spectre\nR1 (a b) resistor r=1k\n";
        let d = doc(net, "a.net", "//");
        assert_eq!(
            with_header(&d, "a.net", net, "0.2"),
            format!("* t\nsimulator lang=spectre\n{header}\nR1 (a b) resistor r=1k\n")
        );
        // a .scs file is in Spectre mode from its first line
        let mut d = doc("// t\n", "a.scs", "//");
        d.add_level(Level::new(LevelKind::File, "lib.scs", "lib.scs", "lib.scs"), Some(0));
        let lib = format!("simulator lang=spectre\n{body}");
        assert_eq!(with_header(&d, "lib.scs", &lib, "0.2"), format!("//\n{header}\n{lib}"));
    }

    #[test]
    fn above_the_switch_to_spectre_a_header_is_written_in_stars() {
        // above the first `simulator lang=spectre` of a file that is not a .scs file, Spectre reads
        // SPICE, where `//` is no comment (2.2): the header there, and the bare marker before a
        // statement, are written with `*`, which carries in both modes
        let star = "* @RNL(type=\"header\") { @VERSION(\"0.2\")}";
        let mut d = doc("// t\nX1 (a b) div\n", "a.scs", "//");
        d.add_level(Level::new(LevelKind::File, "lib.inc", "lib.inc", "lib.inc"), Some(0));
        let body = "simulator lang=spectre\nsubckt div (p n)\nR1 (p n) resistor r=1k\nends div\n";
        for (head, want) in [
            ("* lib\n.param rr=1k\n", format!("* lib\n{star}\n.param rr=1k\n")),
            ("simulator lang=spice\n.param rr=1k\n", format!("*\n{star}\nsimulator lang=spice\n.param rr=1k\n")),
        ] {
            let lib = format!("{head}{body}");
            assert_eq!(carrier(&d, "lib.inc", &lib), "//");
            assert_eq!(with_header(&d, "lib.inc", &lib, "0.2"), format!("{want}{body}"), "{head}");
        }
        // a main file; the record written with the header follows it, so that the next solve
        // writes the same
        let net = "* t\n.param rr=1k\nsimulator lang=spectre\nR1 (a b) resistor r=1k\n";
        let d = doc(net, "a.net", "//");
        let out = replace_generated_with_header(&d, 0, net, "@RNL(type=\"generated\") {#R1 @PLACE(0, 0)}", "0.2");
        let want = format!("* t\n{star}\n.param rr=1k\nsimulator lang=spectre\nR1 (a b) resistor r=1k\n");
        assert_eq!(out, format!("{want}* @RNL(type=\"generated\") {{#R1 @PLACE(0, 0)}}\n"));
        // a record above the switch, which the reading takes: the header goes before it, not after
        // the switch below it, where it would not be the first record (2.8)
        let net = "* t\n// @RNL { /* note */ }\nsimulator lang=spectre\nR1 (a b) resistor r=1k\n";
        let d = doc(net, "a.net", "//");
        assert_eq!(d.levels[0].records.len(), 1);
        assert_eq!(
            with_header(&d, "a.net", net, "0.2"),
            format!("* t\n{star}\n// @RNL {{ /* note */ }}\nsimulator lang=spectre\nR1 (a b) resistor r=1k\n")
        );
    }

    #[test]
    fn a_file_read_only_through_library_sections_gets_no_header() {
        // what lies outside the selected section is not read, and tools must not write there
        // (2.2): no header goes on line 2 of a file read only through its sections
        let header = "* @RNL(type=\"header\") { @VERSION(\"0.2\")}";
        let mut d = doc("* t\n.end\n", "a.sp", "*");
        let mut tt = Level::new(LevelKind::File, "models.lib (tt)", "models.lib", "models.lib");
        tt.section = Some("tt".into());
        d.add_level(tt, Some(0));
        let lib = "* models\n.lib tt\n.subckt pair a b\nR1 a b 1k\n.ends\n.endl tt\n";
        assert_eq!(with_header(&d, "models.lib", lib, "0.2"), lib);
        // read whole as well, it reads line 2
        d.add_level(Level::new(LevelKind::File, "models.lib", "models.lib", "models.lib"), Some(0));
        assert_eq!(with_header(&d, "models.lib", lib, "0.2"), format!("* models\n{header}\n{}", &lib[9..]));
    }

    #[test]
    fn spectre_carrier_and_end_of_file() {
        let net = "// t\nR1 (a b) resistor r=1k";
        let d = doc(net, "a.scs", "//");
        let out = append_record(&d, 0, net, "@RNL {#R1 @PLACE(0, 0)}");
        assert_eq!(out, "// t\nR1 (a b) resistor r=1k\n// @RNL {#R1 @PLACE(0, 0)}\n");
    }
}
