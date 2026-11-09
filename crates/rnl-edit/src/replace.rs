//! Replacing the records of one level in a netlist that has records already.
//!
//! [`write`](crate::write) writes into a netlist without RNL records. This is the narrow other case:
//! a tool that has a level's whole new drawing, such as Analog Canvas after an engineer tidied one
//! subcircuit, puts it back into the netlist's own files. [`replace_records`] reads the netlist
//! from its entry, finds the level by its title, takes out the records of that level in the file
//! that holds it and puts the records it is given at the end of the level's own text
//! (`writeback::replace_records`), and gives the new text of that one file. Nothing else changes in
//! it, and no other file is touched: the caller writes the file where it is. It merges nothing into
//! the records that were there and keeps no revision of them.

use std::collections::BTreeMap;

use rnl_runtime::diag::Sink;
use rnl_runtime::source::{normalize, Layered, MemoryProvider, Provider};
use rnl_runtime::writeback::{find_level, Refused};
use rnl_runtime::Options;

/// How the netlist is read: the file that begins it, and how its files are read.
#[derive(Clone, Debug)]
pub struct Reading<'a> {
    /// The path of the main file, as the provider names it.
    pub entry: &'a str,
    /// "spice" or "spectre", instead of guessing from the entry.
    pub dialect: Option<&'a str>,
    /// The formats set for files by path (`rnl_view::BuildOptions::formats`).
    pub formats: &'a BTreeMap<String, String>,
    /// The runtime's options; the records are read with `generated` set, as a new record of the
    /// solver is read like any other.
    pub options: &'a Options,
}

/// The new text of the file that holds a level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replacement {
    /// The path of the file, as the reading names it, `/`-separated.
    pub path: String,
    /// Its new text.
    pub text: String,
    /// How many records of the level were taken out.
    pub removed: usize,
    /// How many records were put in.
    pub added: usize,
}

/// Why nothing is replaced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The operation refuses, with its reason and the kind of it (`rnl_runtime::diag::codes`).
    Refused(Refused),
    /// The netlist cannot be read.
    Unreadable(String),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Refused(r) => r.fmt(f),
            Failure::Unreadable(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for Failure {}

/// Replace the records of the level titled `title`, a subcircuit by its name or a file by its file
/// name as the commands take them, by `records`: RNL source without comment markers, one or more
/// `@RNL(…) {…}`, the level's whole new drawing. `files` reads the netlist and what it includes.
///
/// The records of the level in its file are taken out, its authored records and its record of the
/// solver, and the new ones go at the end of its own text, in the comment carrier of the file; the
/// header record, the other levels' records and every other line stay as they are, line endings
/// included. It refuses, giving nothing, where the level is not found or is not one, where the level
/// has no place for a record, as in a region written to be encrypted, and where the records do not
/// read: the netlist is read again with the new text, and no error may lie in the new lines.
pub fn replace_records(
    reading: &Reading,
    files: &dyn Provider,
    title: &str,
    records: &str,
) -> Result<Replacement, Failure> {
    let mut sink = Sink::new();
    let doc = rnl_view::read_host_with(reading.entry, reading.dialect, reading.formats, files, &mut sink)
        .map_err(Failure::Unreadable)?;
    let level = find_level(&doc, title).map_err(Failure::Refused)?;
    let lv = &doc.levels[level];
    let path = normalize(&lv.path);
    let text = match doc.source(&lv.file) {
        Some(text) => text.to_string(),
        None => files.read(&path).ok_or_else(|| Failure::Unreadable(format!("cannot read {path}")))?,
    };
    let options = Options { generated: true, problems: false, ..reading.options.clone() };
    // the netlist as it reads with the new text of the file in the file's place
    let reread = |new: &str| {
        let memory = MemoryProvider::new().with(&path, new);
        let both = Layered { layers: vec![&memory, files] };
        rnl_view::interpret_with(reading.entry, reading.dialect, reading.formats, &both, &options)
    };
    let done =
        rnl_runtime::writeback::replace_records(&doc, level, &text, records, reread).map_err(Failure::Refused)?;
    Ok(Replacement { path, text: done.text, removed: done.removed, added: done.added })
}
