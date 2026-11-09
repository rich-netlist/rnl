//! `rnl replace-records`: the records of one level replaced by the records it is given, in the file
//! that holds the level (standard 2.2, 2.8, 17).
//!
//! The netlist is read from its entry, as `rnl solve` reads it, and `rnl_edit::replace::replace_records`
//! gives the new text of the one file that holds the level. That file is held as a run of
//! `rnl solve` holds its files ([`SolveRun::files`]), with the other files the netlist read, so that
//! [`solve::write_with_encoding`] writes it where `-o DIR` or `--in-place` says, in the encoding it
//! was read in, and copies the rest under `DIR`: the one way the command writes a file.

use std::collections::BTreeMap;
use std::path::Path;

use rnl_edit::replace::{Failure, Reading};
use rnl_runtime::source::{normalize, Provider};
use rnl_runtime::Options;

use crate::provider::path_str;
use crate::solve::{Noting, SolveRun, SolvedFile};

/// What `rnl replace-records` takes besides the file, the level and the records.
#[derive(Clone, Debug, Default)]
pub struct ReplaceOptions {
    /// "spice" or "spectre", instead of guessing from the file.
    pub dialect: Option<String>,
    /// The formats set for files by path (`rnl_view::BuildOptions::formats`).
    pub formats: BTreeMap<String, String>,
    /// The runtime's options the netlist is read with.
    pub runtime: Options,
}

/// The file with the records of a level replaced.
#[derive(Clone, Debug)]
pub struct ReplaceRun {
    /// The file, as a run of `rnl solve` holds its files, with no level solved: [`SolveRun::files`]
    /// is the one file with its new text, and [`SolveRun::read`] the files the netlist reads, for
    /// [`solve::write_with_encoding`].
    pub run: SolveRun,
    /// How many records of the level were taken out.
    pub removed: usize,
    /// How many records were put in.
    pub added: usize,
}

/// Replace the records of the level titled `level` by `records`, RNL source without comment
/// markers, in `file`, the main file of the netlist, or in the file of the netlist that holds the
/// level. Nothing is written; refuses, with the reason and its kind, as
/// `rnl_edit::replace::replace_records` does.
pub fn replace_records(
    file: &Path,
    files: &dyn Provider,
    options: &ReplaceOptions,
    level: &str,
    records: &str,
) -> Result<ReplaceRun, Failure> {
    let entry = path_str(file);
    let noting = Noting::new(files);
    let reading = Reading {
        entry: &entry,
        dialect: options.dialect.as_deref(),
        formats: &options.formats,
        options: &options.runtime,
    };
    let done = rnl_edit::replace::replace_records(&reading, &noting, level, records)?;
    // the files the netlist read, but for the packages of the package directories, which it reaches
    // by name
    let packages: Vec<String> = files.package_files().iter().map(|p| normalize(p)).collect();
    let read: Vec<String> = noting.read.take().into_iter().filter(|p| !packages.contains(p)).collect();
    let replaced = SolvedFile { path: done.path, text: done.text };
    let run = SolveRun {
        entry,
        levels: Vec::new(),
        not_solved: Vec::new(),
        files: vec![replaced],
        read,
        errors: 0,
        raised: Vec::new(),
    };
    Ok(ReplaceRun { run, removed: done.removed, added: done.added })
}
