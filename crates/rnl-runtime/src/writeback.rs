//! Writing RNL back into host files (standard 2.2, 16.4, 17).
//!
//! A tool changes only its own records: a solver replaces the `type="generated"` records of the
//! level it solved, an editor appends its records after them. Everything else in the file, host
//! statements, ordinary comments and other records, stays byte for byte, line endings included.
//! New records go at the end of the level's own text, in the comment carrier the file already
//! uses for RNL, `*` where the reading is in SPICE mode ([`carrier_at`]); a level that ends in a
//! region written to be encrypted has its record before the region ([`record_line`]). A file a
//! tool saves has a header record with its version ([`with_header`], 16.1).
//!
//! Every change is made at the positions of the document's reading of the text, from the end of
//! the text up, so that each leaves the places of those above it as they were: the records of a
//! file, its header among them, are found where the reading found them, however many lines the
//! other changes add or remove. A solver that writes a level back and gives its file its header
//! makes both changes in one pass ([`replace_generated_with_header`],
//! [`remove_generated_with_header`]); one that writes several levels of a file first moves the
//! header that the record of one of them would come before ([`move_header_for`]).

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
    /// that are left holding nothing but a comment marker, `marker`, the file's carrier, or the
    /// marker that carried the record on its line.
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
        // the marker that carries the record on its line: the file's, or the other one of a file
        // in Spectre's dialect, `*` above its `simulator lang=spectre` or `//` below it (2.2)
        let marker = match head.trim() {
            own @ ("*" | "//") if !marker.is_empty() => own,
            _ => marker,
        };
        // a record removed from the middle of a carrier line leaves one marker and one space
        let merged = if !marker.is_empty() && head.trim() == marker && !tail.trim().is_empty() {
            format!("{marker} {}", tail.trim_start())
        } else {
            format!("{head}{tail}")
        };

        let ending = Self::ending(&last_line).to_string();
        let rest = merged.trim();
        let empty = rest.is_empty() || rest == marker;
        let replacement: Vec<String> = if empty { Vec::new() } else { vec![format!("{}{ending}", merged.trim_end())] };
        self.lines.splice(l0..=l1, replacement);
    }

    /// Insert `s` into a line before the character at `col`, 0-based, of its text.
    fn insert_in(&mut self, line: u32, col: usize, s: &str) {
        let Some(l) = self.lines.get_mut((line as usize).wrapping_sub(1)) else { return };
        let at = l.char_indices().nth(col).map_or(Self::body(l).len(), |(i, _)| i);
        l.insert_str(at, s);
    }

    /// Replace the characters of a line from `from` to `to`, 0-based, `to` not included, by `s`.
    fn replace_in(&mut self, line: u32, from: usize, to: usize, s: &str) {
        let Some(l) = self.lines.get_mut((line as usize).wrapping_sub(1)) else { return };
        let end = Self::body(l).len();
        let at = |col: usize| l.char_indices().nth(col).map_or(end, |(i, _)| i);
        let (a, b) = (at(from), at(to));
        l.replace_range(a..b.max(a), s);
    }

    fn insert(&mut self, before_line: u32, new: &[String]) {
        // a line not there yet, such as line 2 of an empty text, has empty lines before it
        while self.lines.len() + 1 < before_line as usize {
            if let Some(last) = self.lines.last_mut().filter(|l| !l.ends_with('\n')) {
                last.push_str(self.eol);
            }
            self.lines.push(self.eol.to_string());
        }
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
    /// Replace the characters of a line, 1-based, from one to another, 0-based, the second not
    /// included, by text.
    Replace(u32, usize, usize, String),
}

impl Edit {
    /// Where it changes the text: the line, and the column of the first character it touches,
    /// 1-based, 0 for lines inserted before the line.
    fn at(&self) -> (u32, usize) {
        match self {
            Edit::Remove(a, _) => (a.0, a.1 as usize),
            Edit::Lines(line, _) => (*line, 0),
            Edit::Text(line, col, _) | Edit::Replace(line, col, _, _) => (*line, col + 1),
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
            Edit::Replace(line, from, to, s) => lines.replace_in(line, from, to, &s),
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
/// Where the reading is in SPICE mode, `//` is no comment: [`carrier_at`] gives the marker of a
/// record written at a line.
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
    let spectre_mode = doc.rules(file).name == "spectre"
        && (file.to_lowercase().ends_with(".scs")
            || text.lines().take(5).any(|l| l.trim().to_lowercase().starts_with("simulator") && l.contains("spectre")));
    if spectre_mode {
        "//"
    } else {
        "*"
    }
}

/// The comment marker of a record written into `file` before its line `line` (2.2): the file's
/// ([`carrier`]), but `*` where the reading is in SPICE mode, where `//` is no comment, such as a
/// `simulator lang=spice` section of a Spectre file.
pub fn carrier_at(doc: &HostDocument, file: &str, text: &str, line: u32) -> &'static str {
    in_mode(doc, file, line, carrier(doc, file, text))
}

/// `marker`, the marker that RNL written into `file` before its line `line` would follow, or `*`
/// in its stead where the reading is in SPICE mode and it is `//` (2.2).
fn in_mode(doc: &HostDocument, file: &str, line: u32, marker: &'static str) -> &'static str {
    if marker == "//" && doc.in_spice_mode(file, line) {
        "*"
    } else {
        marker
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
/// the `.end` of a main file, or at the end of the file, and before a region written to be
/// encrypted that the end lies in ([`record_line`]), carried as the reading takes it there
/// ([`carrier_at`]). `text` is the text of the level's file. A level that has no place for it,
/// such as a subcircuit written in such a region, gets nothing: the text comes back as it is.
pub fn append_record(doc: &HostDocument, level: LevelId, text: &str, record: &str) -> String {
    let Ok(at) = record_line(doc, level, text) else { return text.to_string() };
    let mut lines = Lines::new(text);
    lines.insert(at, &carried(record, carrier_at(doc, &doc.levels[level].file, text, at)));
    lines.text()
}

/// The line before which text added to the end of a level goes (17).
fn end_of(doc: &HostDocument, level: LevelId, lines: &Lines) -> u32 {
    let lv = &doc.levels[level];
    if let Some((_, end)) = lv.section_lines {
        end
    } else if lv.end_line > 0 {
        lv.end_line
    } else {
        lines.lines.len() as u32 + 1
    }
}

/// The line before which a record added to the end of a level goes (17): the `.ends` of a
/// subcircuit, the `.end` of a main file, or one past the last line of a file. A record in a
/// region written to be encrypted (`.prot … .unprot`, `protect … unprotect`) is gone once the
/// region is encrypted (2.2): when the end of the level lies in one, the record goes before the
/// region instead, and before the subcircuit written in the level that holds that place, at the
/// last place of the level's own text above the end that lies in no region. A level without one,
/// such as a subcircuit whose `.subckt` lies in a region, gives that region, as the lines that open
/// and close it ([`HostDocument::unwritable_at`]). Other excluded host regions follow the same
/// rule, including unterminated scripts/foreign languages, preprocessing and job-title lines.
pub fn record_line(doc: &HostDocument, level: LevelId, text: &str) -> Result<u32, (u32, u32)> {
    let lv = &doc.levels[level];
    let file = &doc.levels[doc.file_level(level)];
    if file.section.is_some() && file.section_lines.is_none() {
        return Err((1, Lines::new(text).lines.len() as u32 + 1));
    }
    // the first place of its own text: after its `.subckt`; line 2 of a file, as line 1 is not read
    let first =
        if lv.kind == LevelKind::Subckt { lv.line + 1 } else { lv.section_lines.map_or(2, |(start, _)| start + 1) };
    free_place(doc, level, end_of(doc, level, &Lines::new(text)), first)
}

/// The last place of the own text of `level`, at the line `at` or above it, that lies in no region
/// excluded from writing (2.2) and in no subcircuit written in `level`, given as the line that
/// text added there goes before; `first` is the first place of that text. A region that begins
/// above `first` gives an error with its lines, as [`record_line`] says.
fn free_place(doc: &HostDocument, level: LevelId, mut at: u32, first: u32) -> Result<u32, (u32, u32)> {
    let lv = &doc.levels[level];
    let below: Vec<&crate::host::Level> = doc
        .descendants(level)
        .into_iter()
        .map(|k| &doc.levels[k])
        .filter(|s| s.kind == LevelKind::Subckt && s.file == lv.file)
        .collect();
    loop {
        if let Some((open, close)) = doc.unwritable_in(level, at) {
            if open < first {
                return Err((open, close));
            }
            at = open;
        } else if let Some(start) = below.iter().filter(|s| s.line < at && at <= s.end_line).map(|s| s.line).min() {
            at = start;
        } else {
            return Ok(at);
        }
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
                && doc.names_level(level, &target)
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
    let (mut edits, _) = header_edits(doc, file, None, text, version);
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
///
/// The new record goes where [`record_line`] puts it, before a region written to be encrypted that
/// the end of the level lies in, carried as the reading takes it there ([`carrier_at`]); the
/// records it replaces go wherever they are, in such a region as well, so that none of them comes
/// after it. A level that has no place for it is left as it is: the text comes back unchanged.
pub fn replace_generated(doc: &HostDocument, level: LevelId, text: &str, record: &str) -> String {
    let file = &doc.levels[level].file;
    let marker = carrier(doc, file, text);
    apply(text, marker, replacement(doc, level, text, record, marker))
}

/// [`replace_generated`], and the level's file given its header record and its version as
/// [`with_header`] gives them, in one pass at the positions of the document's reading of `text`:
/// what a solver writes into the file of a level it solved, which it saves (16.1). The header
/// record stays the first record of its file (2.8): one written after a subcircuit solved, which
/// the record put at the end of the subcircuit would come before, moves to where a new header
/// goes ([`with_header`]; the draft 0.3, 16.3 and 2.2): on line 2, after the title or the first
/// line, which never carries RNL, written with the carrier of the file; in a file that is not a
/// `.scs` file and whose first statement is `simulator lang=spectre`, after that statement with
/// `//`, and otherwise above it with `*`, in SPICE mode. It keeps its VERSION, or gets one, and
/// the indentation of its lines, and is removed where the reading found it; no record comes
/// before it there, as none came before it where it was. A file read only through library
/// sections, whose line 2 is not read (2.2), has it moved to the line above the subcircuit
/// instead, the outermost one in the file that holds the level, inside the section. The record
/// follows the carrier of a header written in the same pass (2.2), so that the next solve, which
/// finds that header, writes it the same, but for `*` where the reading is in SPICE mode.
///
/// Nothing is written in a region written to be encrypted (2.2): the record goes before it
/// ([`record_line`]), and so does a header that moves above a subcircuit whose `.subckt` lies in
/// one; a header there gets no VERSION. A level that has no place for its record is left as it
/// is, and so is its file: the text comes back unchanged.
pub fn replace_generated_with_header(
    doc: &HostDocument,
    level: LevelId,
    text: &str,
    record: &str,
    version: &str,
) -> String {
    if record_line(doc, level, text).is_err() {
        return text.to_string();
    }
    let file = &doc.levels[level].file;
    let at = Some(doc.file_level(level));
    let (mut edits, marker) =
        header_move(doc, level, text, version).unwrap_or_else(|| header_edits(doc, file, at, text, version));
    edits.extend(replacement(doc, level, text, record, marker));
    apply(text, carrier(doc, file, text), edits)
}

/// The text of the level's file, `text`, with its header record moved where
/// [`replace_generated_with_header`] moves it when the record of the solver written at the end of
/// `level` would come before it, with its VERSION; the text as it is otherwise. A solver that
/// writes several levels of a file moves the header so before it writes the first of them, and
/// reads the file again: where it goes, the header may need a carrier other than the file's, `*`
/// above the `simulator lang=spectre` of a file in `//` (2.2), and every record written into the
/// file then follows that carrier, those of the levels written before the one that comes before
/// the header as well, as the next solve, which finds the header there, writes them.
pub fn move_header_for(doc: &HostDocument, level: LevelId, text: &str, version: &str) -> String {
    match header_move(doc, level, text, version) {
        Some((edits, _)) => apply(text, carrier(doc, &doc.levels[level].file, text), edits),
        None => text.to_string(),
    }
}

/// The changes that move the header record of the level's file when the record of the solver
/// written at the end of `level` (17), or before a region written to be encrypted that the end
/// lies in ([`record_line`]), would come before it, as [`replace_generated_with_header`] says, and
/// the carrier it has where it goes, which records written with it follow (2.2); None when it
/// stays where it is, or has no place to go out of such a region. The target reading controls
/// both the header and its destination, even when another reading takes the physical file whole.
fn header_move(doc: &HostDocument, level: LevelId, text: &str, version: &str) -> Option<(Vec<Edit>, &'static str)> {
    let file = &doc.levels[level].file;
    let at = record_line(doc, level, text).ok()?;
    let file_level = Some(doc.file_level(level));
    let header = first_header(doc, file, file_level).filter(|h| span(h).0 .0 >= at)?;
    let marker = carrier(doc, file, text);
    let to = top(doc, file, file_level, text, marker)
        .or_else(|| subckt_start(doc, level).map(|line| Top { line, marker, bare: false }))
        .and_then(|to| to.kept(doc, file, file_level))?;
    // the version it gets where it is goes with it
    Some((moved_header(header, version, to), to.marker))
}

/// The first line of the outermost subcircuit, in the file of `level`, that holds `level` or is
/// it: the record written at the end of `level` lies inside it; or, when that line lies in a
/// region written to be encrypted, the last place above it in the text of the file level that
/// holds no subcircuit and lies in no region ([`free_place`]). None for a file level, or when there
/// is no such place. Where the header of a file read only through library sections moves to
/// (2.2).
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
    let (start, outer) = (start.filter(|&line| line > 0)?, at?);
    let first = doc.levels[outer].section_lines.map_or(2, |(start, _)| start + 1);
    free_place(doc, outer, start, first).ok()
}

/// The changes that move a header record to `to`, carried by its marker, with VERSION when it has
/// none: removed where the reading found it, and written whole there, each of its lines after the
/// first as it was after its marker, so that its indentation stays.
fn moved_header(header: &Record, version: &str, to: Top) -> Vec<Edit> {
    let mut text = header.text.clone();
    if let Some((brace, s)) = version_insert(header, version) {
        let at = text.char_indices().nth(brace + 1).map_or(text.len(), |(b, _)| b);
        text.insert_str(at, &s);
    }
    let marker = to.marker;
    let lines = text
        .split('\n')
        .enumerate()
        .map(|(k, l)| if k == 0 { format!("{marker} {l}") } else { format!("{marker}{l}") });
    let (a, b) = span(header);
    vec![Edit::Remove(a, b), to.edit(lines.collect())]
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
/// file level `at`, or when it is None of the file level that reads `file`, one that reads it whole
/// before one that reads a section of it, and of the subcircuits written in it; a section of
/// `.lib` that is read counts as a file, so that two sections of one file have their records
/// apart. With that file level, None when no file level reads the file.
fn file_records<'a>(
    doc: &'a HostDocument,
    file: &str,
    at: Option<LevelId>,
) -> (Option<LevelId>, Vec<(LevelId, &'a Record)>) {
    let mut levels: Vec<LevelId> = (0..doc.levels.len())
        .filter(|&l| doc.levels[l].kind == LevelKind::File && doc.levels[l].file == file)
        .collect();
    levels.sort_by_key(|&l| doc.levels[l].section.is_some());
    let Some(top) = at.or(levels.first().copied()) else {
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
/// written at its file level, `at` or the one that reads `file` ([`file_records`], 2.8); a package
/// file has a header by rule (16.2) and gives none.
fn first_header<'a>(doc: &'a HostDocument, file: &str, at: Option<LevelId>) -> Option<&'a Record> {
    if file.to_lowercase().ends_with(".rnl") {
        return None;
    }
    let (top, records) = file_records(doc, file, at);
    let first = records.into_iter().min_by_key(|(_, r)| span(r).0);
    first.filter(|(l, r)| top.is_none_or(|t| t == *l) && record_type(r) == "header").map(|(_, r)| r)
}

/// The changes of [`replace_generated`]: the removal of the level's records of the solver in its
/// file, and the new record at the end of the level (17), or before a region written to be
/// encrypted that the end lies in ([`record_line`]), carried by `marker`, the carrier of the
/// file's records, but by `*` where the reading is in SPICE mode (2.2); none when the level has no
/// place for it. A record of the solver below the place of the new record, written with scope= in
/// the file level after a subcircuit, is removed before the new record goes in, those above after
/// it.
fn replacement(doc: &HostDocument, level: LevelId, text: &str, record: &str, marker: &'static str) -> Vec<Edit> {
    let file = &doc.levels[level].file;
    let Ok(at) = record_line(doc, level, text) else { return Vec::new() };
    let mut edits = generated_removals(doc, level, file);
    edits.push(Edit::Lines(at, carried(record, in_mode(doc, file, at, marker))));
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
/// The first line of a host file never carries RNL (2.2): it is the title of the main file, and in
/// an included file a comment or a statement. The header goes on the line after a title or a
/// comment, and after the lines that continue it (`+` lines, or the line after one that ends with
/// `\` in Spectre's dialect); before a statement, a comment line, the bare marker, is put first, so
/// that the statement keeps its meaning on the third line. Spectre reads a file that is not a
/// `.scs` file in SPICE mode until its `simulator lang=spectre`, and `//` is no comment there
/// (2.2): a header carried by `//` goes after that statement, and after the lines that continue it,
/// when no other statement and no record comes before it; otherwise it goes above the statement,
/// written with `*`, which carries in both modes, and so does the bare marker.
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
    apply(text, carrier(doc, file, text), header_edits(doc, file, None, text, version).0)
}

/// The changes that give a file its header record and its version ([`with_header`]): none when it
/// has them, or is left as it is; and the carrier that records written with them follow: that of
/// a new header, else the file's ([`carrier`]). Nothing is written in a region written to be
/// encrypted (2.2): a header record there gets no VERSION. `at` is the file level whose header it
/// is, a library section read as a file (2.8), or None for the one that reads `file`.
fn header_edits(
    doc: &HostDocument,
    file: &str,
    at: Option<LevelId>,
    text: &str,
    version: &str,
) -> (Vec<Edit>, &'static str) {
    let marker = carrier(doc, file, text);
    if file.to_lowercase().ends_with(".rnl") {
        return (Vec::new(), marker);
    }
    match first_header(doc, file, at) {
        Some(first) => {
            // right after the `{` of the body
            let edit = version_insert(first, version).map(|(brace, s)| {
                let at = first.loc(brace);
                Edit::Text(at.line, at.col as usize, s)
            });
            let top = file_records(doc, file, at).0;
            let edit = edit.filter(|e| unwritable(doc, file, top, e.at().0).is_none());
            return (edit.into_iter().collect(), marker);
        }
        None if headers_of(doc, file, at) => return (Vec::new(), marker),
        // a library section, whose text is all a reading of it reads, gets no new header (2.2)
        None if at.is_some_and(|l| doc.levels[l].section.is_some()) => return (Vec::new(), marker),
        None => {}
    }
    let version_kw = format!("@VERSION({})", crate::problem::written_string(version));
    match new_header(doc, file, text, marker, |_| format!("@RNL(type=\"header\") {{ {version_kw}}}")) {
        Some((edit, marker)) => (vec![edit], marker),
        None => (Vec::new(), marker),
    }
}

/// The change that writes a new header record at the top of `file` ([`top`]), the record that
/// `record` gives for the marker that carries it there, and that marker; None where none can be
/// written, such as in a file read only through library sections.
fn new_header(
    doc: &HostDocument,
    file: &str,
    text: &str,
    marker: &'static str,
    record: impl FnOnce(&'static str) -> String,
) -> Option<(Edit, &'static str)> {
    let to = top(doc, file, None, text, marker).and_then(|to| to.kept(doc, file, None))?;
    Some((to.edit(carried(&record(to.marker), to.marker)), to.marker))
}

/// A file that has no header record, given one that is not [`with_header`]'s, such as one with
/// USE-PACKAGE (16.3): `record` gives the whole record for the comment marker that carries it, so
/// that it can fit its lines to it, and it goes where [`with_header`] puts a new header, by the
/// same rules. A file that has a header record, or where none can be written, comes back as it
/// is, and so does a package file. `text` is the text whose positions the document gives.
pub fn add_header(doc: &HostDocument, file: &str, text: &str, record: impl FnOnce(&'static str) -> String) -> String {
    if file.to_lowercase().ends_with(".rnl") || records_of(doc, file).iter().any(|(_, r)| record_type(r) == "header") {
        return text.to_string();
    }
    let marker = carrier(doc, file, text);
    match new_header(doc, file, text, marker, record) {
        Some((edit, _)) => apply(text, marker, vec![edit]),
        None => text.to_string(),
    }
}

/// The USE-PACKAGE that keeps the standard series 0.2 in a file raised to 0.3 (16.3).
pub const PIN_STD_0_2: &str = "@USE-PACKAGE(name=\"rnl-std\", version=\"0.2\")";

/// `text`, the text of one file, with the file levels `levels` that read it raised to the version
/// 0.3 (16.1), each with whether it was: the VERSION of the header record of each says 0.3, and
/// with its `pin`, [`PIN_STD_0_2`] follows it, so that it keeps the standard series 0.2 it was read
/// with (16.3); a file level without a header record gets one that says so, where a new header
/// goes ([`with_header`]). A library section is read as a file (2.8): two sections of one file are
/// raised apart, and the text outside them stays as it is. One is not raised when it cannot be so:
/// a package file, a header record that is not the first record of its file level, a VERSION on
/// several lines or in a region written to be encrypted (2.2), or a library section that has no
/// header record.
pub fn raise_version(doc: &HostDocument, text: &str, levels: &[(LevelId, bool)]) -> (String, Vec<bool>) {
    let mut edits: Vec<Edit> = Vec::new();
    let mut raised = Vec::new();
    for &(level, pin) in levels {
        let edit = raise_edit(doc, level, text, pin);
        raised.push(edit.is_some());
        // a section read twice has one header record: it is raised once
        edits.extend(edit.filter(|e| !edits.iter().any(|o| o.at() == e.at())));
    }
    if edits.is_empty() {
        return (text.to_string(), raised);
    }
    let marker = levels.first().map_or("*", |&(l, _)| carrier(doc, &doc.levels[l].file, text));
    (apply(text, marker, edits), raised)
}

/// The change that raises the file level `level` to 0.3 ([`raise_version`]), None when it cannot
/// be raised.
fn raise_edit(doc: &HostDocument, level: LevelId, text: &str, pin: bool) -> Option<Edit> {
    let file = &doc.levels[level].file;
    if file.to_lowercase().ends_with(".rnl") {
        return None;
    }
    let pinned = if pin { format!("; {PIN_STD_0_2}") } else { String::new() };
    let raised = format!("@VERSION(\"0.3\"){pinned}");
    let marker = carrier(doc, file, text);
    let edit = match first_header(doc, file, Some(level)) {
        Some(header) => {
            let ast = parse_record(&header.text);
            let body = ast.body?;
            let call = |m: &crate::syntax::Member| m.segments.first().and_then(|s| s.call.clone());
            match body.members.iter().filter_map(|m| call(m)).find(|c| c.kw == "VERSION") {
                Some(version) => {
                    let (a, b) = (header.loc(version.start), header.loc(version.end.saturating_sub(1)));
                    if a.line != b.line {
                        return None;
                    }
                    Edit::Replace(a.line, a.col as usize - 1, b.col as usize, raised)
                }
                // right after the `{` of the body, as `with_header` writes VERSION there
                None => {
                    let sep = if body.members.is_empty() { "" } else { ";" };
                    let at = header.loc(body.start);
                    Edit::Text(at.line, at.col as usize, format!(" {raised}{sep}"))
                }
            }
        }
        None if headers_of(doc, file, Some(level)) => return None,
        None if doc.levels[level].section.is_some() => return None,
        None => new_header(doc, file, text, marker, |_| format!("@RNL(type=\"header\") {{ {raised}}}"))?.0,
    };
    if doc.unwritable_in(level, edit.at().0).is_some() {
        return None;
    }
    Some(edit)
}

/// Whether a file has a header record anywhere, or the file level `at` has one: one that is not
/// the first record is an error its author fixes, and a tool writes no other.
fn headers_of(doc: &HostDocument, file: &str, at: Option<LevelId>) -> bool {
    let header = |r: &&Record| record_type(r) == "header";
    match at {
        Some(_) => file_records(doc, file, at).1.iter().map(|(_, r)| r).any(header),
        None => records_of(doc, file).iter().map(|(_, r)| r).any(header),
    }
}

/// The file levels that take their language version from the file level `level` (16.1): the files
/// it includes, and those they include in turn, whose header records write no VERSION, a library
/// section read as a file among them (2.8), in the order of the document. Raising `level` to 0.3
/// has them read in 0.3 with it. The file levels `apart`, raised on their own, are left out, and so
/// are those that take their version from one of them.
pub fn takes_version_of(doc: &HostDocument, level: LevelId, apart: &[LevelId]) -> Vec<LevelId> {
    let mut out = Vec::new();
    let mut todo: Vec<LevelId> = doc.levels[level].children.iter().rev().copied().collect();
    while let Some(l) = todo.pop() {
        let lv = &doc.levels[l];
        if lv.kind == LevelKind::File {
            if apart.contains(&l) || writes_version(doc, l) {
                continue;
            }
            out.push(l);
        }
        todo.extend(lv.children.iter().rev().copied());
    }
    out
}

/// Whether the header record of the file level `level` writes a VERSION (16.1).
fn writes_version(doc: &HostDocument, level: LevelId) -> bool {
    let Some(header) = first_header(doc, &doc.levels[level].file, Some(level)) else { return false };
    let Some(body) = parse_record(&header.text).body else { return false };
    body.members.iter().filter_map(|m| m.segments.first().and_then(|s| s.call.as_ref())).any(|c| c.kw == "VERSION")
}

/// Whether the header record of the file level `level`, a library section read as a file (2.8),
/// or that of a file level above it, which includes it, pins a version of the standard package,
/// `@USE-PACKAGE(name="rnl-std", version="…")`, which then comes before the series that the
/// version of the file implies (16.3).
pub fn pins_std(doc: &HostDocument, level: LevelId) -> bool {
    let mut files: Vec<LevelId> = Vec::new();
    for l in std::iter::once(level).chain(doc.ancestors(level)).map(|l| doc.file_level(l)) {
        if !files.contains(&l) {
            files.push(l);
        }
    }
    files.into_iter().filter_map(|l| first_header(doc, &doc.levels[l].file, Some(l))).any(|h| {
        let Some(body) = parse_record(&h.text).body else { return false };
        body.members.iter().filter_map(|m| m.segments.first().and_then(|s| s.call.as_ref())).any(|c| {
            let named = |n: &str| c.params.as_ref().and_then(|p| p.named.iter().find(|x| x.name == n));
            c.kw == "USE-PACKAGE"
                && named("name").is_some_and(|n| n.value == Value::Str("rnl-std".into()))
                && named("version").is_some()
        })
    })
}

/// The names in the text of the file level `level`, a library section read as a file (2.8), that
/// have the spelling of a standard word that the version 0.3 adds, `numbering`, a field of the
/// record head (2.8): keys of texts, role names, parameter names of definitions and of usages,
/// sub-object names, aliases, those it imports from alias packages among `imported`
/// ([`crate::Interpretation::imported_aliases`]), fields of record heads, and the parameters of
/// the host's instances, each as what it is and where. Raising the file level to 0.3 may change
/// their meaning, so a tool that raises it gives a warning for each (16.1, 16.5).
pub fn spelled_like_new_words(doc: &HostDocument, level: LevelId, imported: &[crate::ImportedAlias]) -> Vec<String> {
    let file = doc.levels[level].file.as_str();
    let words: Vec<&str> = crate::words::draft_header_fields().iter().map(|f| f.name.as_str()).collect();
    let mut found: Vec<(&'static str, String, crate::diag::Loc)> = Vec::new();
    for (_, rec) in file_records(doc, file, Some(level)).1 {
        let ast = parse_record(&rec.text);
        let mut names: Vec<(&'static str, String, usize)> = Vec::new();
        for n in ast.header.iter().flat_map(|h| &h.named) {
            names.push(("a field of a record head", n.name.clone(), n.off));
        }
        if let Some(body) = &ast.body {
            names_in(&body.members, &mut names);
        }
        found.extend(
            names.into_iter().filter(|(_, n, _)| words.contains(&n.as_str())).map(|(w, n, o)| (w, n, rec.loc(o))),
        );
    }
    for a in imported.iter().filter(|a| a.level == level && words.contains(&a.alias.as_str())) {
        found.push(("an alias from an alias package", a.alias.clone(), a.loc.clone()));
    }
    for lv in &doc.levels {
        for inst in lv.instances.iter().filter(|i| i.file == file && doc.file_level_of(i) == Some(level)) {
            for (name, _) in inst.params.iter().filter(|(n, _)| words.contains(&n.as_str())) {
                found.push(("a parameter of an instance", name.clone(), crate::diag::Loc::line(file, inst.line)));
            }
        }
    }
    found.sort_by_key(|(_, _, at)| (at.line, at.col));
    found.dedup();
    found.into_iter().map(|(what, name, at)| format!("{what}, `{name}`, line {}", at.line)).collect()
}

/// The names written in a record's members, their definition bodies included, as
/// [`spelled_like_new_words`] takes them: what each is, the name, and its offset in the record.
fn names_in(members: &[std::rc::Rc<crate::syntax::Member>], out: &mut Vec<(&'static str, String, usize)>) {
    use crate::syntax::{ObjExpr, Sep};
    // the names of `roles=` and `parameters=`: a string, or the first of a list
    let listed = |v: &Value| -> Vec<String> {
        let Value::Arr(items) = v else { return Vec::new() };
        items
            .iter()
            .filter_map(|i| match i {
                Value::Str(s) => Some(s.clone()),
                Value::Arr(xs) => match xs.first() {
                    Some(Value::Str(s)) => Some(s.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    };
    for m in members {
        for s in &m.segments {
            for a in &s.args {
                if let Some(role) = &a.role {
                    out.push(("a role name", role.clone(), a.start));
                }
                match &a.obj {
                    ObjExpr::Ref(r) => out.extend(
                        r.path
                            .iter()
                            .filter(|p| p.sep == Sep::Sub)
                            .map(|p| ("a sub-object name", p.name.clone(), r.start)),
                    ),
                    ObjExpr::Set(set) => names_in(&set.members, out),
                    ObjExpr::Point(_) => {}
                }
            }
            let Some(call) = &s.call else { continue };
            let Some(params) = &call.params else { continue };
            for n in &params.named {
                match (call.kw.as_str(), n.name.as_str(), &n.value) {
                    (_, "key", Value::Str(k)) => out.push(("the key of a text", k.clone(), n.off)),
                    ("DEFINE", "roles", v) => out.extend(listed(v).into_iter().map(|r| ("a role name", r, n.off))),
                    ("DEFINE", "parameters", v) => {
                        out.extend(listed(v).into_iter().map(|p| ("a parameter of a definition", p, n.off)))
                    }
                    _ => out.push(("a parameter name", n.name.clone(), n.off)),
                }
            }
            if call.kw == "ALIAS" {
                if let Some(Value::Str(alias)) = params.positional.first() {
                    out.push(("an alias", alias.trim_start_matches('@').to_string(), params.start));
                }
            }
        }
    }
}

/// File-wide header operations use the whole-file reading when one exists; a targeted
/// section operation uses its own file level. Both retain the physical host exclusions.
fn unwritable(doc: &HostDocument, file: &str, top: Option<LevelId>, line: u32) -> Option<(u32, u32)> {
    match top {
        Some(level) => doc.unwritable_in(level, line),
        None => doc.unwritable_at(file, line),
    }
}

/// Where a header record written at the top of a file goes: before the line `line`, 1-based,
/// carried by `marker`, after the bare marker when `bare`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Top {
    line: u32,
    marker: &'static str,
    bare: bool,
}

impl Top {
    /// Where the reading keeps a header written there (2.2): None in a region written to be
    /// encrypted, and carried by `*` in its stead where the reading is in SPICE mode. The
    /// target file level `at` also bounds a library-section move; None selects the file-wide reading.
    fn kept(self, doc: &HostDocument, file: &str, at: Option<LevelId>) -> Option<Top> {
        let marker = in_mode(doc, file, self.line, self.marker);
        let mut line = self.line;
        let mut below_first = false;
        let (top, records) = file_records(doc, file, at);
        while let Some((open, close)) = unwritable(doc, file, top, line) {
            if open < 2 || below_first {
                below_first = true;
                // An encrypted envelope may start on the first comment line. Its first safe
                // header position is after the closing pragma, if it closes before EOF and
                // no existing record would precede the new header there.
                let past = doc.source(file)?.lines().count() as u32 + 1;
                if close >= past || records.iter().any(|(_, r)| r.lines.0 <= close) {
                    return None;
                }
                line = close + 1;
            } else {
                line = open;
            }
        }
        if top.is_some_and(|level| {
            let lv = &doc.levels[level];
            lv.section.is_some() && lv.section_lines.is_none_or(|(start, end)| line <= start || line > end)
        }) {
            return None;
        }
        let marker = in_mode(doc, file, line, marker);
        Some(Top { line, marker, ..self })
    }

    /// The change that writes these lines, each behind the marker already, there.
    fn edit(self, lines: Vec<String>) -> Edit {
        if self.bare {
            Edit::Lines(self.line, [vec![self.marker.to_string()], lines].concat())
        } else {
            Edit::Lines(self.line, lines)
        }
    }
}

/// Where a file's header record goes at its top ([`with_header`]; the draft 0.3, 16.3 and 2.2),
/// carried by `marker`, the file's carrier, unless said otherwise: line 2, after a title or a
/// comment on the first line, which never carries RNL, and after the lines that continue it; the
/// bare marker first before a statement on the first line of an included file. In a file that is not a `.scs` file, carried by `//`, whose
/// first statement is `simulator lang=spectre`, after that statement and the lines that continue
/// it, when no record comes before it; otherwise above it, in SPICE mode, where only `*` is a
/// comment (2.2), carried by `*`. None for a selected library-section reading: a file-wide
/// top position need not lie inside that section, even when another reading takes the file whole (2.2).
/// `at` names the target file level, or None selects the file-wide reading.
fn top(doc: &HostDocument, file: &str, at: Option<LevelId>, text: &str, marker: &'static str) -> Option<Top> {
    if file_records(doc, file, at).0.is_some_and(|top| doc.levels[top].section.is_some()) {
        return None;
    }
    let lines = Lines::new(text);
    let main = doc.levels.first().is_some_and(|l| l.file == file);
    let spectre = doc.rules(file).name == "spectre";
    let mut marker = marker;
    if spectre && marker == "//" && !file.to_lowercase().ends_with(".scs") {
        match after_spectre_switch(&lines, usize::from(main)) {
            // a record above the switch is read: the header goes before it, to be the first (2.8)
            Some(after) if !records_of(doc, file).iter().any(|(_, r)| r.lines.0 < after) => {
                return Some(Top { line: after, marker, bare: false });
            }
            // above the switch, in SPICE mode, where only `*` is a comment (2.2)
            _ => marker = "*",
        }
    }
    let comment = |l: &str| l.starts_with('*') || (spectre && l.starts_with("//"));
    let first = lines.lines.first().map(|l| Lines::body(l));
    let bare = first.is_some_and(|l| !(main || l.trim().is_empty() || comment(l)));
    if bare {
        return Some(Top { line: 1, marker, bare });
    }
    // after the lines that continue line 1, such as a title: a `+` line, or in Spectre's dialect
    // the line after one that ends with `\`; no comment goes between a line and its continuation
    // (2.2)
    let body = |line: usize| lines.lines.get(line - 1).map(|l| Lines::body(l));
    let mut line = 2;
    while body(line).is_some_and(|l| l.trim_start().starts_with('+'))
        || (spectre && body(line - 1).is_some_and(|l| l.ends_with('\\')) && body(line).is_some())
    {
        line += 1;
    }
    Some(Top { line: line as u32, marker, bare })
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

    /// A tiny SPICE reader for these tests: `*` carriers, `.subckt`/`.ends` levels, `.prot …
    /// .unprot` regions; with `//`, a Spectre reader, whose `*` lines carry as well, with `protect …
    /// unprotect` regions, and in SPICE mode from the start of a file that is not a .scs file, or
    /// from a `simulator lang=spice`, up to a `simulator lang=spectre`.
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
        let (mut protect, mut spice) = (None, (marker == "//" && !file.ends_with(".scs")).then_some(0));
        for (k, l) in lines.iter().enumerate() {
            let no = k as u32 + 1;
            if k > 0 {
                let own = if marker == "//" && l.starts_with('*') { "*" } else { marker };
                if let Some(rest) = l.strip_prefix(own) {
                    if rest.is_empty() || rest.starts_with(' ') {
                        run.push(CarrierLine { line: no, content: rest.to_string(), col: own.len() as i64 });
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
            } else if low.starts_with(".prot") || low == "protect" {
                protect.get_or_insert(no);
            } else if low.starts_with(".unprot") || low == "unprotect" {
                d.protected.extend(protect.take().map(|a| (file.to_string(), a, no)));
            } else if low.starts_with("simulator") && marker == "//" {
                if low.contains("spectre") {
                    d.spice_mode.extend(spice.take().map(|a| (file.to_string(), a, no)));
                } else {
                    spice.get_or_insert(no);
                }
            }
        }
        let past = lines.len() as u32 + 1;
        d.protected.extend(protect.map(|a| (file.to_string(), a, past)));
        d.spice_mode.extend(spice.map(|a| (file.to_string(), a, past)));
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
    fn a_header_below_a_subcircuit_solved_moves_to_line_2() {
        // a header record after a subcircuit: the record of the subcircuit goes in before its
        // `.ends`, and would come before the header, which must be the first record of the file
        // (2.8); so the header moves where a new header goes, line 2 after the title (the draft
        // 0.3, 16.3 and 2.2), with its VERSION (16.1, 17)
        let net = "* t\nR0 a 0 1k\n.subckt s a b\nR1 a b 1k\n.ends\n* @RNL(type=\"header\") { }\nX1 a b s\n.end\n";
        let d = doc(net, "a.sp", "*");
        let record = "@RNL(type=\"generated\") {\n  #R1 @PLACE(0, 0)\n}";
        let out = replace_generated_with_header(&d, 1, net, record, "0.2");
        let moved = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\nR0 a 0 1k\n.subckt s a b\nR1 a b 1k\n\
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
        let kept = "* t\nR0 a 0 1k\n.subckt s a b\nR1 a b 1k\n.ends\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n\
                    X1 a b s\n* @RNL(type=\"generated\") {}\n.end\n";
        assert_eq!(top, kept);
        let both = replace_generated_with_header(&doc(&top, "a.sp", "*"), 1, &top, record, "0.2");
        assert_eq!(both, moved.replace(".end\n", "* @RNL(type=\"generated\") {}\n.end\n"));
        // a header on several lines keeps its indentation where it goes
        let long = net.replace("{ }", "{\n*   @DEVICE(x)\n* }");
        let out = replace_generated_with_header(&doc(&long, "a.sp", "*"), 1, &long, record, "0.2");
        let header = "* @RNL(type=\"header\") { @VERSION(\"0.2\");\n*   @DEVICE(x)\n* }\nR0 a 0 1k\n.subckt s a b\n";
        assert!(out.starts_with(&format!("* t\n{header}")) && out.matches("type=\"header\"").count() == 1, "{out}");
        assert_eq!(replace_generated_with_header(&doc(&out, "a.sp", "*"), 1, &out, record, "0.2"), out);
        // a subcircuit right after the title: line 2 is above it
        let first = net.replace("R0 a 0 1k\n", "");
        let out = replace_generated_with_header(&doc(&first, "a.sp", "*"), 1, &first, record, "0.2");
        assert_eq!(out, moved.replace("R0 a 0 1k\n", ""));
        // a file read only through a library section, whose line 2 is not read (2.2): above the
        // subcircuit instead, inside the section
        let lib =
            "* models\n.lib tt\nR0 a 0 1k\n.subckt s a b\nR1 a b 1k\n.ends\n* @RNL(type=\"header\") { }\n.endl tt\n";
        let mut d = doc(lib, "models.lib", "*");
        d.levels[0].section = Some("tt".into());
        d.levels[0].section_lines = Some((2, 8));
        let out = replace_generated_with_header(&d, 1, lib, "@RNL(type=\"generated\") {}", "0.2");
        let want =
            "* models\n.lib tt\nR0 a 0 1k\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n.subckt s a b\nR1 a b 1k\n\
                    * @RNL(type=\"generated\") {}\n.ends\n.endl tt\n";
        assert_eq!(out, want);
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
    fn a_header_below_a_subcircuit_moves_after_the_switch_to_spectre_or_in_stars_above_it() {
        // a file that is not a .scs file, whose header in `//` comes after a subcircuit solved:
        // where a new header goes (the draft 0.3, 16.3 and 2.2), after its `simulator
        // lang=spectre` when that is its first statement, still in `//`; above the switch, in
        // SPICE mode, in `*`, which the record written with it follows
        let body = "subckt s (a b)\nR1 (a b) resistor r=1k\nends s\n// @RNL(type=\"header\") { }\nX1 (a b) s\n";
        let record = "@RNL(type=\"generated\") {#R1 @PLACE(0, 0)}";
        let net = format!("* t\nsimulator lang=spectre\n{body}");
        let d = doc(&net, "a.net", "//");
        let out = replace_generated_with_header(&d, 1, &net, record, "0.2");
        let want = "* t\nsimulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.2\") }\nsubckt s (a b)\n\
                    R1 (a b) resistor r=1k\n// @RNL(type=\"generated\") {#R1 @PLACE(0, 0)}\nends s\nX1 (a b) s\n";
        assert_eq!(out, want);
        assert_eq!(replace_generated_with_header(&doc(&out, "a.net", "//"), 1, &out, record, "0.2"), out);
        let net = format!("* t\n.param rr=1k\nsimulator lang=spectre\n{body}");
        let d = doc(&net, "a.net", "//");
        let out = replace_generated_with_header(&d, 1, &net, record, "0.2");
        let head = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n.param rr=1k\nsimulator lang=spectre\n";
        let want = format!(
            "{head}subckt s (a b)\nR1 (a b) resistor r=1k\n* @RNL(type=\"generated\") {{#R1 @PLACE(0, 0)}}\nends s\n\
             X1 (a b) s\n"
        );
        assert_eq!(out, want);
        // moved before the file level, which comes first, is written: the record of the file level
        // then follows the header in `*` as well, as the next solve, which finds it there, writes it
        let moved = move_header_for(&d, 1, &net, "0.2");
        assert_eq!(moved, format!("{head}subckt s (a b)\nR1 (a b) resistor r=1k\nends s\nX1 (a b) s\n"));
        assert_eq!(move_header_for(&d, 0, &net, "0.2"), net);
        let d = doc(&moved, "a.net", "//");
        assert_eq!(carrier(&d, "a.net", &moved), "*");
        assert_eq!(move_header_for(&d, 1, &moved, "0.2"), moved);
        let top = replace_generated_with_header(&d, 0, &moved, "@RNL(type=\"generated\") {}", "0.2");
        let both = replace_generated_with_header(&doc(&top, "a.net", "//"), 1, &top, record, "0.2");
        assert_eq!(both, format!("{want}* @RNL(type=\"generated\") {{}}\n"));
    }

    #[test]
    fn a_record_removed_leaves_no_marker_of_its_own_alone() {
        // a file in Spectre's dialect whose carrier is `*`, that of its header above its
        // `simulator lang=spectre`, with records in `//` below it: each removed takes its line
        // with it, or leaves one `//` and one space before what follows it on its line
        let net = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n.param rr=1k\nsimulator lang=spectre\n\
                   // @RNL(type=\"generated\") {\n//   #R1 @PLACE(0, 0)\n// }\n\
                   // @RNL(type=\"generated\") {#R1}   /* note */\nR1 (a b) resistor r=1k\n";
        let d = doc(net, "a.net", "//");
        assert_eq!(carrier(&d, "a.net", net), "*");
        assert_eq!(
            strip_generated(&d, "a.net", net),
            "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n.param rr=1k\nsimulator lang=spectre\n\
             // /* note */\nR1 (a b) resistor r=1k\n"
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
        tt.section_lines = Some((2, 6));
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

    #[test]
    fn a_record_goes_before_a_region_written_to_be_encrypted() {
        // a record in `.prot … .unprot` is gone once the region is encrypted (2.2): a subcircuit
        // whose `.ends` lies in one has its record before the region, inside the subcircuit, and
        // the records of the solver it replaces go wherever they are, so that none comes after it
        let net = "* t\n.subckt s a b\nR1 a b 1k\n.prot\nR2 a b 1k\n* @RNL(type=\"generated\") {#R1 @PLACE(9, 9)}\n\
                   .ends\n.unprot\nX1 a b s\n.end\n";
        let d = doc(net, "a.sp", "*");
        assert_eq!(d.protected, [("a.sp".to_string(), 4, 8)]);
        assert_eq!((record_line(&d, 1, net), record_line(&d, 0, net)), (Ok(4), Ok(10)));
        let record = "@RNL(type=\"generated\") {#R1 @PLACE(0, 0)}";
        let header = "* @RNL(type=\"header\") { @VERSION(\"0.2\")}";
        assert_eq!(
            replace_generated_with_header(&d, 1, net, record, "0.2"),
            format!("* t\n{header}\n.subckt s a b\nR1 a b 1k\n* {record}\n.prot\nR2 a b 1k\n.ends\n.unprot\nX1 a b s\n.end\n")
        );
        // a main file whose `.end` lies in a region left open, which begins in a subcircuit: before
        // that subcircuit, the last place of the file level's own text above the region
        let net = "* t\nR0 a 0 1k\n.subckt s a b\nR1 a b 1k\n.prot\nR2 a b 1k\n.ends\nX1 a b s\n.end\n";
        let d = doc(net, "a.sp", "*");
        assert_eq!(d.protected, [("a.sp".to_string(), 5, 10)]);
        assert_eq!((record_line(&d, 0, net), record_line(&d, 1, net)), (Ok(3), Ok(5)));
        assert_eq!(
            append_record(&d, 0, net, "@RNL {#X1 @PLACE(0, 0)}"),
            net.replace("R0 a 0 1k\n", "R0 a 0 1k\n* @RNL {#X1 @PLACE(0, 0)}\n")
        );
    }

    #[test]
    fn a_subcircuit_written_in_a_region_written_to_be_encrypted_gets_no_record() {
        // its `.subckt` lies in the region, and so does every place of its own text (2.2): its file
        // is left as it is, the record of the solver there as well
        let net =
            "* t\n.prot\n.subckt s a b\nR1 a b 1k\n* @RNL(type=\"generated\") {#R1 @PLACE(9, 9)}\n.ends\n.unprot\n\
                   X1 a b s\n.end\n";
        let d = doc(net, "a.sp", "*");
        assert_eq!((record_line(&d, 1, net), record_line(&d, 0, net)), (Err((2, 7)), Ok(9)));
        let record = "@RNL(type=\"generated\") {#R1 @PLACE(0, 0)}";
        assert_eq!(replace_generated_with_header(&d, 1, net, record, "0.2"), net);
        assert_eq!(replace_generated(&d, 1, net, record), net);
        assert_eq!(append_record(&d, 1, net, record), net);
        assert_eq!(move_header_for(&d, 1, net, "0.2"), net);
        // a header record in such a region gets no VERSION there
        let net = "* t\n.prot\n* @RNL(type=\"header\") { }\nR1 a b 1k\n.unprot\n.end\n";
        let d = doc(net, "a.sp", "*");
        assert_eq!(with_header(&d, "a.sp", net, "0.2"), net);
        let out = replace_generated_with_header(&d, 0, net, "@RNL(type=\"generated\") {}", "0.2");
        assert_eq!(out, net.replace(".end\n", "* @RNL(type=\"generated\") {}\n.end\n"));
    }

    #[test]
    fn a_header_that_moves_above_a_subcircuit_goes_before_a_region_written_to_be_encrypted() {
        // a file read only through a library section, whose header moves above the subcircuit it
        // comes after (2.8): the `.subckt` lies in a region, so the header goes before the region,
        // still inside the section (2.2)
        let lib = "* models\n.lib tt\nR0 a 0 1k\n.prot\n.subckt s a b\n.unprot\nR1 a b 1k\n.ends\n\
                   * @RNL(type=\"header\") { }\n.endl tt\n";
        let mut d = doc(lib, "models.lib", "*");
        d.levels[0].section = Some("tt".into());
        d.levels[0].section_lines = Some((2, 10));
        let out = replace_generated_with_header(&d, 1, lib, "@RNL(type=\"generated\") {}", "0.2");
        let want =
            "* models\n.lib tt\nR0 a 0 1k\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n.prot\n.subckt s a b\n\
                    .unprot\nR1 a b 1k\n* @RNL(type=\"generated\") {}\n.ends\n.endl tt\n";
        assert_eq!(out, want);
    }

    #[test]
    fn in_spice_mode_a_record_is_carried_by_a_star() {
        // in a `simulator lang=spice` section of a Spectre file, `//` is no comment, and a record
        // carried by it would be lost: it is written with `*`, a comment in both modes (2.2); in
        // Spectre mode, with the carrier of the file
        let net =
            "// t\nsubckt s (a b)\nR1 (a b) resistor r=1k\nends s\nsimulator lang=spice\n.subckt p a b\nR2 a b 1k\n\
                   .ends\nsimulator lang=spectre\nX1 (a b) s\n";
        let d = doc(net, "a.scs", "//");
        assert_eq!(d.spice_mode, [("a.scs".to_string(), 5, 9)]);
        let markers: Vec<&str> = [4, 8, 9, 10, 11].iter().map(|&l| carrier_at(&d, "a.scs", net, l)).collect();
        assert_eq!(markers, ["//", "*", "*", "//", "//"]);
        let record = "@RNL(type=\"generated\") {#R2 @PLACE(0, 0)}";
        let out = replace_generated(&d, 2, net, record);
        assert_eq!(out, net.replace("R2 a b 1k\n", &format!("R2 a b 1k\n* {record}\n")));
        assert_eq!(append_record(&d, 2, net, record), out);
        // a file whose records are in `//` after its switch to Spectre mode, with a subcircuit in a
        // later SPICE section: its record in `*`, and so is that of the file level, at its end there
        let net = "* t\nsimulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.2\")}\nX1 (a b) p\n\
                   simulator lang=spice\n.subckt p a b\nR2 a b 1k\n.ends\n";
        let d = doc(net, "a.net", "//");
        assert_eq!(carrier(&d, "a.net", net), "//");
        let out = replace_generated_with_header(&d, 1, net, record, "0.2");
        assert_eq!(out, net.replace("R2 a b 1k\n", &format!("R2 a b 1k\n* {record}\n")));
        let out = replace_generated_with_header(&d, 0, net, "@RNL(type=\"generated\") {}", "0.2");
        assert_eq!(out, format!("{net}* @RNL(type=\"generated\") {{}}\n"));
    }

    #[test]
    fn a_header_record_of_its_own_goes_where_a_new_header_goes() {
        // `add_header` writes the record that its caller makes for the marker that carries it, by
        // the rules of `with_header`: after line 1 and the lines that continue it, a `+` line, or
        // in Spectre's dialect the line after one that ends with `\` (2.2)
        let record = |m: &'static str| format!("@RNL(type=\"header\") {{ @VERSION(\"0.3\"); @DEVICE(m={})}}", m.len());
        let net = "title\n+ more of it\nR1 a 0 1k\n";
        let d = doc(net, "a.sp", "*");
        let head = "title\n+ more of it\n* @RNL(type=\"header\") { @VERSION(\"0.3\")";
        assert_eq!(add_header(&d, "a.sp", net, record), format!("{head}; @DEVICE(m=1)}}\nR1 a 0 1k\n"));
        assert_eq!(with_header(&d, "a.sp", net, "0.3"), format!("{head}}}\nR1 a 0 1k\n"));
        let net = "t \\\nmore\nR1 (a b) resistor r=1k\n";
        let d = doc(net, "a.scs", "//");
        assert_eq!(
            add_header(&d, "a.scs", net, record),
            "t \\\nmore\n// @RNL(type=\"header\") { @VERSION(\"0.3\"); @DEVICE(m=2)}\nR1 (a b) resistor r=1k\n"
        );
        // an empty text keeps line 1, which is never read, empty
        let d = doc("", "a.sp", "*");
        assert_eq!(with_header(&d, "a.sp", "", "0.3"), "\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n");
        assert_eq!(
            add_header(&d, "a.sp", "", record),
            "\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @DEVICE(m=1)}\n"
        );
        // a file that has a header record, and a package file, come back as they are
        let with = "* t\n* @RNL(type=\"header\") {}\n.end\n";
        assert_eq!(add_header(&doc(with, "a.sp", "*"), "a.sp", with, record), with);
        assert_eq!(add_header(&d, "p.rnl", "@RNL {}\n", record), "@RNL {}\n");
    }

    #[test]
    fn a_file_raised_to_0_3_keeps_the_standard_series_it_was_read_with() {
        // the VERSION of its header says 0.3, followed by the pin of the series 0.2 (16.1, 16.3)
        let raise = |d: &HostDocument, text: &str, pin: bool| {
            let (out, done) = raise_version(d, text, &[(0, pin)]);
            done[0].then_some(out)
        };
        let net = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\"); @DEVICE(x) }\nR1 a b 1k\n.end\n";
        let d = doc(net, "a.sp", "*");
        assert!(!pins_std(&d, 0));
        let raised = format!(
            "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"); {PIN_STD_0_2}; @DEVICE(x) }}\nR1 a b 1k\n.end\n"
        );
        assert_eq!(raise(&d, net, true).as_deref(), Some(raised.as_str()));
        // a pin of the standard package stays, the only one, in the file and for a file it includes
        let pinned = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.2\")}\n.end\n";
        let mut d = doc(pinned, "a.sp", "*");
        assert!(pins_std(&d, 0));
        let out = raise(&d, pinned, false).unwrap();
        assert_eq!(out, pinned.replacen("\"0.2\"", "\"0.3\"", 1));
        let lib = d.add_level(Level::new(LevelKind::File, "lib.inc", "lib.inc", "lib.inc"), Some(0));
        assert!(pins_std(&d, lib));
        // a header without VERSION gets it after its `{`; a file without a header gets one
        let pin = format!("{{ @VERSION(\"0.3\"); {PIN_STD_0_2}}}");
        let bare = "* t\n* @RNL(type=\"header\") {}\n.end\n";
        assert_eq!(raise(&doc(bare, "a.sp", "*"), bare, true), Some(bare.replace("{}", &pin)));
        let none = "* t\nR1 a b 1k\n.end\n";
        let out = raise(&doc(none, "a.sp", "*"), none, true);
        assert_eq!(out, Some(format!("* t\n* @RNL(type=\"header\") {pin}\nR1 a b 1k\n.end\n")));
        // a package file, a header that is not the first record, and one in a region written to be
        // encrypted are not raised
        let package = d.add_level(Level::new(LevelKind::File, "p.rnl", "p.rnl", "p.rnl"), Some(0));
        assert_eq!(raise_version(&d, "@RNL {}\n", &[(package, true)]), ("@RNL {}\n".to_string(), vec![false]));
        let late = "* t\n* @RNL {#R1}\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.end\n";
        assert_eq!(raise(&doc(late, "a.sp", "*"), late, true), None);
        let hidden = "* t\n.prot\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.unprot\n.end\n";
        assert_eq!(raise(&doc(hidden, "a.sp", "*"), hidden, true), None);
    }

    #[test]
    fn names_spelled_like_the_word_that_0_3_adds() {
        // `numbering`, a field of the record head in 0.3 (2.8), would change the meaning of a name of
        // that spelling (16.1, 16.5)
        let net = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\"); @ALIAS(\"numbering\", \"axis\")}\nR1 a b 1k\n\
                   * @RNL(numbering=\"x\") { #R1 @TEXT(\"a\", key=\"numbering\"); \
                   $D = @DEFINE(roles=[\"numbering\"], parameters=[[\"numbering\", 1]]) {} }\n.end\n";
        let d = doc(net, "a.sp", "*");
        let mut found = spelled_like_new_words(&d, 0, &[]);
        found.sort();
        assert_eq!(
            found,
            [
                "a field of a record head, `numbering`, line 4",
                "a parameter of a definition, `numbering`, line 4",
                "a role name, `numbering`, line 4",
                "an alias, `numbering`, line 2",
                "the key of a text, `numbering`, line 4",
            ]
        );
        let plain = "* t\n* @RNL { #R1 @TEXT(\"numbering\", key=\"name\") }\n.end\n";
        assert!(spelled_like_new_words(&doc(plain, "a.sp", "*"), 0, &[]).is_empty());
    }
}
