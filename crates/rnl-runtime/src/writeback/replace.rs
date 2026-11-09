//! Replacing the records of one level in a file that has records already (standard 2.2, 2.8, 17).
//!
//! The drawing of a level is the records written in its own text. [`replace_records`] takes them all
//! out, the records an author wrote and the one a solver wrote, and puts the records it is given at
//! the end of the level's own text, in the comment carrier the file uses for RNL ([`record_line`],
//! [`carrier_at`](super::carrier_at)). Everything else in the file stays byte for byte: host
//! statements, ordinary comments, the header record, the records of every other level, line endings
//! included. It writes nothing it cannot stand behind: a level that has no place for a record, one
//! that is not told from another by its title, and records that do not read all refuse, with a kind
//! of the toolkit's list ([`codes`]) and the reason, and no text is given.
//!
//! Reading the records is the caller's work, as the runtime reads no file: [`replace_records`] is
//! given a function that reads the document again with the new text of the file in place of the
//! old, and refuses unless the records read back as they were given, with no error in their lines.

use std::fmt;

use super::{
    applies_to, apply, carried, carrier, file_records, in_mode, record_line, record_type, records_of, span, Edit, Lines,
};
use crate::diag::{codes, Code, Severity, Sink};
use crate::host::{file_name, HostDocument, Level, LevelId, LevelKind, Record};
use crate::records::{scan_run, CarrierLine};
use crate::Interpretation;

/// Why a tool does not write what it was asked to: the kind of the refusal, from the toolkit's list
/// ([`codes`]), and its message. Printed as `RNL8405 writeback-replace-region-encrypted: …`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub code: Code,
    pub message: String,
}

impl Refused {
    fn new(code: Code, message: impl Into<String>) -> Refused {
        Refused { code, message: message.into() }
    }
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for Refused {}

/// What [`replace_records`] gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replaced {
    /// The new text of the level's file.
    pub text: String,
    /// How many records of the level were taken out: its authored records, its record of the solver,
    /// and a record of the solver written for it with scope= in a level above it, in this file.
    pub removed: usize,
    /// How many records were put in their place.
    pub added: usize,
    /// The lines of the new text that hold them, the first and the last, 1-based; None when none was
    /// added.
    pub lines: Option<(u32, u32)>,
}

/// The level titled `title`, as the commands name one: a subcircuit by its name, which compares by
/// the case rule of its dialect, a file level by the name of its file. Refuses when no level is, or
/// when several are, such as the subcircuits of one name in two files: it takes the same title to
/// mean one level (a file included twice is read twice, and counts once).
pub fn find_level(doc: &HostDocument, title: &str) -> Result<LevelId, Refused> {
    let levels = || doc.levels.iter().enumerate().filter(|(_, lv)| lv.kind != LevelKind::Package);
    let mut found: Vec<LevelId> = levels().filter(|(_, lv)| lv.title() == title).map(|(k, _)| k).collect();
    if found.is_empty() {
        let same = |lv: &Level| lv.kind == LevelKind::Subckt && lv.same_name(&lv.name, title);
        found = levels().filter(|(_, lv)| same(lv)).map(|(k, _)| k).collect();
    }
    // the readings of one file, and of one library section of it, are one place
    let mut places: Vec<(LevelId, &Level)> = Vec::new();
    for &k in &found {
        let lv = &doc.levels[k];
        let twin = |(_, o): &(LevelId, &Level)| {
            (o.kind, &o.file, o.line, &o.section) == (lv.kind, &lv.file, lv.line, &lv.section)
        };
        if !places.iter().any(twin) {
            places.push((k, lv));
        }
    }
    match places.as_slice() {
        [(k, _)] => Ok(*k),
        [] => {
            let mut titles: Vec<String> = Vec::new();
            for (_, lv) in levels() {
                if !titles.contains(&lv.title()) {
                    titles.push(lv.title());
                }
            }
            let more = titles.len().saturating_sub(MOST_TITLES);
            titles.truncate(MOST_TITLES);
            let mut list = titles.join(", ");
            if more > 0 {
                list.push_str(&format!(" and {more} more"));
            }
            let message = format!("no level is titled {title}; the levels are {list}");
            Err(Refused::new(codes::WRITEBACK_REPLACE_LEVEL_NOT_FOUND, message))
        }
        _ => {
            let at: Vec<String> = places
                .iter()
                .map(|(_, lv)| match lv.kind {
                    LevelKind::Subckt => format!("{} line {}", file_name(&lv.file), lv.line),
                    _ => file_name(&lv.file).to_string(),
                })
                .collect();
            let message = format!(
                "{} levels are titled {title}, in {}; the level to replace is not one",
                at.len(),
                at.join(", ")
            );
            Err(Refused::new(codes::WRITEBACK_REPLACE_LEVEL_AMBIGUOUS, message))
        }
    }
}

/// The place of a record in its file: the line and column of its first character and of its last,
/// 1-based ([`span`]).
type Span = ((u32, u32), (u32, u32));

/// How many titles a refusal lists.
const MOST_TITLES: usize = 12;

/// How many problems of the records a refusal lists.
const MOST_PROBLEMS: usize = 3;

/// Replace the records of `level` in `text`, the text of its file, by `records`, which are RNL
/// source without comment markers: one or more `@RNL(…) {…}` one after another, the level's whole
/// new drawing. Taken out are the records written in the level's own text but for a header record,
/// the authored ones and the solver's, and the records of the solver written for it with scope= in
/// a level above in the same file ([`generated_for`](super::generated_for)); a record of the solver
/// for it in another file is not in `text` and stays there. Everything else stays: the header
/// record, the records of the other levels, host statements, comments, line endings. The new
/// records go where [`record_line`] puts a record, carried as the reading takes it there
/// ([`carrier_at`](super::carrier_at)); `records` with nothing in it only take the old ones out.
/// `doc` is the reading of `text`, as the places of its records say.
///
/// It refuses, and gives no text, where the level has no place for a record (in a region written to
/// be encrypted, or excluded from writing, or a library section that is not there), where the
/// records would come before the header record of their file, which is to stay its first record
/// (2.8), and where they do not read: `reread` is given the new text of the file and gives the
/// document read again with it, and the refusal says which problems lie in the new lines, which
/// records read back otherwise than they were given, or that the file declares a language version
/// that is not interpreted (16.1), where no record can be checked. A text without a record in it is
/// no records.
pub fn replace_records(
    doc: &HostDocument,
    level: LevelId,
    text: &str,
    records: &str,
    reread: impl FnOnce(&str) -> Result<Interpretation, String>,
) -> Result<Replaced, Refused> {
    let lv = &doc.levels[level];
    let file = lv.file.as_str();
    let at = record_line(doc, level, text).map_err(|(a, b)| no_place(doc, level, a, b))?;
    let marker = carrier(doc, file, text);
    let carrier_here = in_mode(doc, file, at, marker);

    // the lines given, without the blank ones before and after them, each behind the carrier
    let given: Vec<&str> = records.lines().collect();
    let skipped = given.iter().take_while(|l| l.trim().is_empty()).count();
    let kept = given.len() - given.iter().rev().take_while(|l| l.trim().is_empty()).count();
    let given = if skipped < kept { carried(&given[skipped..kept].join("\n"), carrier_here) } else { Vec::new() };

    // the header record stays the first record of its file (2.8): records put before it would not
    // leave it so, and a tool changes no line outside the level's records
    if !given.is_empty() {
        let top = doc.file_level(level);
        let header = file_records(doc, file, Some(top))
            .1
            .into_iter()
            .filter(|(_, r)| record_type(r) == "header")
            .map(|(_, r)| span(r).0 .0)
            .filter(|&line| line >= at)
            .min();
        if let Some(line) = header {
            let message = format!(
                "the records are not replaced: they would come before the header record of {}, line {line}, \
                 which stays the first record of its file (2.8)",
                file_name(file)
            );
            return Err(Refused::new(codes::WRITEBACK_REPLACE_BEFORE_HEADER, message));
        }
    }

    // the old records go wherever they are; the new ones go in at `at`, all at the places of `doc`
    let picked = |k: LevelId, r: &Record| {
        (k == level && record_type(r) != "header") || (record_type(r) == "generated" && applies_to(doc, k, r, level))
    };
    let old: Vec<_> = records_of(doc, file).into_iter().filter(|(k, r)| picked(*k, r)).map(|(_, r)| span(r)).collect();
    let first = first_new_line(text, marker, &old, at);
    let (expected, scanned) = read_given(&given, carrier_here, first, file);
    // the problems of the new lines, as the line of the records given counts them
    let line_of = |line: u32| line.saturating_sub(first) + 1 + skipped as u32;
    if !scanned.is_empty() {
        return Err(unread(scanned, line_of));
    }
    if !given.is_empty() && expected.is_empty() {
        let message = "the records given do not read, and nothing is written: the text given holds no RNL record, \
                       which is @RNL(…) {…}, written without comment markers";
        return Err(Refused::new(codes::WRITEBACK_REPLACE_RECORDS_UNREAD, message));
    }
    let mut edits: Vec<Edit> = old.iter().map(|&(a, b)| Edit::Remove(a, b)).collect();
    let removed = edits.len();
    let last = first + given.len() as u32 - 1;
    let lines = (!given.is_empty()).then_some((first, last));
    if !given.is_empty() {
        edits.push(Edit::Lines(at, given));
    }
    let text = apply(text, marker, edits);

    let read = reread(&text).map_err(|e| {
        let message = format!("the file does not read back with the records as given: it cannot be read again: {e}");
        Refused::new(codes::WRITEBACK_REPLACE_REREAD_DIFFERS, message)
    })?;
    // what it reads as, with the records in it
    if let Some(Some((declaring, version))) = read.unsupported.get(level) {
        let declared = file_name(&read.doc.levels[*declaring].file);
        let message = format!(
            "the records are not replaced: {declared} declares the version {version}, which this reading does not \
             interpret: none of its RNL is read, and it is kept as it is (16.1)"
        );
        return Err(Refused::new(codes::WRITEBACK_REPLACE_VERSION_UNSUPPORTED, message));
    }
    let this_level = |a: &Level| (a.kind, &a.name, &a.file, a.line) == (lv.kind, &lv.name, &lv.file, lv.line);
    if !read.doc.levels.get(level).is_some_and(this_level) {
        let message = "the file does not read back with the records as given: its levels are not the ones it had";
        return Err(Refused::new(codes::WRITEBACK_REPLACE_REREAD_DIFFERS, message));
    }
    let Some((first, last)) = lines else { return Ok(Replaced { text, removed, added: 0, lines }) };
    let mut problems: Vec<(u32, String)> = read
        .diagnostics()
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .filter_map(|d| {
            d.loc.as_ref().filter(|l| l.file == file && (first..=last).contains(&l.line)).map(|l| (l.line, d))
        })
        .map(|(line, d)| (line, d.message.clone()))
        .collect();
    problems.sort();
    problems.dedup();
    if !problems.is_empty() {
        return Err(unread(problems, line_of));
    }
    // the records read back are the records given
    let back: Vec<&Record> = read.doc.levels[level]
        .records
        .iter()
        .filter(|r| r.file == file && r.lines.0 >= first && r.lines.1 <= last)
        .collect();
    let same = back.len() == expected.len()
        && back.iter().zip(&expected).all(|(a, b)| (&a.text, a.lines) == (&b.text, b.lines));
    if !same {
        let message = format!(
            "the file does not read back with the records as given: {} records given, and {} read back from the \
             lines {first} to {last} of {}, not the same",
            expected.len(),
            back.len(),
            file_name(file)
        );
        return Err(Refused::new(codes::WRITEBACK_REPLACE_REREAD_DIFFERS, message));
    }
    Ok(Replaced { text, removed, added: expected.len(), lines })
}

/// The line, in the text the replacement gives, where the records put at `at` begin: `at`, less the
/// lines that the records of `old`, the spans of those taken out, leave above it. A text shorter than
/// `at` has blank lines made up to it ([`Lines::insert`]).
fn first_new_line(text: &str, marker: &str, old: &[Span], at: u32) -> u32 {
    let lines = Lines::new(text);
    let before = (at as usize).saturating_sub(1);
    let above = before.min(lines.lines.len());
    let prefix = lines.lines[..above].concat();
    let removals = old.iter().filter(|(_, end)| end.0 < at).map(|&(a, b)| Edit::Remove(a, b)).collect();
    let kept = Lines::new(&apply(&prefix, marker, removals)).lines.len();
    (kept + (before - above)) as u32 + 1
}

/// The refusal of records that do not read, `problems` being the lines of the new text, each with its
/// message, which `line_of` gives as the line of the records given.
fn unread(problems: Vec<(u32, String)>, line_of: impl Fn(u32) -> u32) -> Refused {
    let more = problems.len().saturating_sub(MOST_PROBLEMS);
    let mut list: Vec<String> =
        problems.iter().take(MOST_PROBLEMS).map(|(line, m)| format!("line {}: {m}", line_of(*line))).collect();
    if more > 0 {
        list.push(format!("and {more} more"));
    }
    let message = format!("the records given do not read, and nothing is written: {}", list.join("; "));
    Refused::new(codes::WRITEBACK_REPLACE_RECORDS_UNREAD, message)
}

/// The records of the lines given, `lines` behind `marker`, as a host adapter cuts them out of the
/// lines of `file` from `first` on, and the errors it finds in them, each with its line.
fn read_given(lines: &[String], marker: &str, first: u32, file: &str) -> (Vec<Record>, Vec<(u32, String)>) {
    let run: Vec<CarrierLine> = lines
        .iter()
        .enumerate()
        .map(|(k, l)| CarrierLine {
            line: first + k as u32,
            content: l.strip_prefix(marker).unwrap_or(l).to_string(),
            col: marker.chars().count() as i64,
        })
        .collect();
    let mut sink = Sink::new();
    let records = scan_run(&run, file, &mut sink);
    let errors = sink.all().iter().filter(|d| d.severity == Severity::Error);
    let errors = errors.filter_map(|d| d.loc.as_ref().map(|l| (l.line, d.message.clone())));
    (records, errors.collect())
}

/// The refusal of a level that has no place for a record, `record_line` having given the lines of
/// the region `a` to `b`.
fn no_place(doc: &HostDocument, level: LevelId, a: u32, b: u32) -> Refused {
    let lv = &doc.levels[level];
    let (file_level, title, named) = (doc.file_level(level), lv.title(), file_name(&lv.file));
    let selected = &doc.levels[file_level];
    let excluded = doc.excluded.iter().any(|(f, x, y)| *f == lv.file && (*x, *y) == (a, b))
        || doc.unselected_sections.iter().any(|(l, x, y)| *l == file_level && (*x, *y) == (a, b));
    match selected.section.as_deref() {
        Some(section) if selected.section_lines.is_none() => Refused::new(
            codes::WRITEBACK_REPLACE_SECTION_NOT_FOUND,
            format!(
                "the records are not replaced: the selected library section {section} was not found in {named} (2.2)"
            ),
        ),
        _ if excluded => Refused::new(
            codes::WRITEBACK_REPLACE_REGION_EXCLUDED,
            format!(
                "the records are not replaced: {title} lies in a host region excluded from writing, lines {a} to {b} \
                 of {named} (2.2)"
            ),
        ),
        _ => Refused::new(
            codes::WRITEBACK_REPLACE_REGION_ENCRYPTED,
            format!(
                "the records are not replaced: {title} lies in a region written to be encrypted, lines {a} to {b} of \
                 {named}, where a record is gone once the region is encrypted (2.2)"
            ),
        ),
    }
}
