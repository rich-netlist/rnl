//! `rnl solve`: every drawing level of a document solved, and its record of the solver written
//! back (standard 2.8, 8.5, 13.7, 14.8, 16.1, chapter 17).
//!
//! The document is read as a solver reads it, `Options::solver()`, without the records with
//! `type="generated"` (2.8). Each level is solved by `rnl-solver`; its record is written with
//! `solution::write_record`, the hash `sig2` of its problem and of the solver's canonical options in
//! the head, finished with `rnl_view::finish_record`, which adds the check of its numbers in a file
//! of the draft 0.3 (5.9), and put in place of the level's records of the solver with
//! `writeback::replace_generated_with_header`, at the end of the level's own text (17). A record of
//! the solver written for the level with scope= in another file, which a solver does not write
//! (2.6) but which applies to the level all the same, is removed from that file. Every file written
//! has its header record and its version (16.1), added when it has none, in the same pass as the
//! record, at the positions the reading gives; but a file read only through library sections,
//! where what lies outside them is not read (2.2), gets no new header. The header record stays the
//! first record of its file (2.8): one written after a subcircuit that gets its record moves where
//! a new header goes, to line 2 (the draft 0.3, 16.3 and 2.2), before any level of its file is
//! written (`writeback::move_header_for`), so that every record written into the file follows the
//! carrier it has there, `*` above the `simulator lang=spectre` of a file in `//`, as the next
//! solve, which finds it there, writes them. A file may hold several levels, and
//! writing one back moves the lines of those after it, so the document is read again before the
//! next level, the texts written so far held in memory over the files.
//!
//! A record in a region written to be encrypted (`.prot … .unprot`, `protect … unprotect`) is
//! gone once the region is encrypted (2.2): the record of a level whose end lies in one goes before
//! the region (`writeback::record_line`), and a level whose own text lies in one, such as a
//! subcircuit of an encrypted library, is solved but not written, with a note that says so. In a
//! `simulator lang=spice` section of a Spectre file, where `//` is no comment, a record is carried
//! by `*` (2.2).
//!
//! The resolution of each level is limited in time by the solver's `time_limit` (the draft 0.3,
//! 8.5), 5 seconds unless `--solver-options` says otherwise, measured on the wall clock
//! ([`WallClock`]): a fact whose check does not finish within it is discarded undecided, and the
//! report of the level has a warning for it (7.6); a fact whose minimal conflict groups are not
//! all found within it has those found, and its hint says that the list is not complete.
//! `time_limit=none` gives the resolution of 8.5 exactly, the same on every run, however long it
//! takes.
//!
//! The report of a level gives each fact discarded with every minimal group of facts it conflicts
//! with, and each statement whose facts are in such a group with the facts discarded because of it
//! (the draft 0.3, 8.5); the hints of a fact of priority low, and of the statements that only such
//! facts lose, only with `--report` ([`report`]), as 7.6 allows them to be left out. Without it the
//! solver does not seek the groups of such a fact beyond its first, which can cost far more than
//! the rest of the resolution (`SolverOptions::hints_for_low`), and a note says so.
//!
//! A fact that has no solution on its own contradicts itself and fails (the draft 0.3, 8.5; 7.6):
//! the report of its level has an error for it, on a line that begins with `error:`, and the level
//! is solved and written back without it, as if it were not written ([`errors`]). The runtime fails
//! those it sees from what is written alone, with an error of the netlist, and leaves them out of
//! the problem (8.5, 9.5); the solver gets those that only the pins of a symbol and the
//! orientations tie. `rnl solve` exits with 1 when any error was reported, of the netlist as a
//! solver reads it ([`SolveRun::errors`]) or of the solver, once the files are written.
//!
//! A level whose own file has an effective language version that the reading does not
//! interpret is not solved: none of the RNL of that file is read, and the file is kept as it is
//! (16.1), so nothing is written into it. An explicitly supported include resumes interpretation.
//!
//! A tool writes only 0.3 (16.1): a file of 0.2 that `rnl solve` writes a record into is raised to
//! 0.3, and its levels are solved again as they read in 0.3 ([`solve_with_clock`],
//! [`VersionRaise`]). A file that only loses a record of the solver written there with scope= is
//! written as it is otherwise and keeps its version, as `rnl strip-generated` only removes records.
//! Its header record then says `@VERSION("0.3")`, followed by
//! `@USE-PACKAGE(name="rnl-std", version="0.2")` unless the file, or a file that includes it, pins
//! a version of the standard package, so that it keeps the standard series 0.2 it was read with,
//! its names and its drawing (16.3). A name in it spelled like a word that 0.3 adds, `numbering`,
//! gets a warning, as the raise may change its meaning, and the file is raised all the same. A
//! file whose header record cannot say 0.3, such as a library section without one, stays at 0.2,
//! with a warning, and its record lists what it cannot hold as unsolved.
//!
//! [`solve`] does the work without writing a file; [`write`] puts the texts where `-o DIR` or
//! `--in-place` says, with the other files the document reads copied under `DIR`; [`lines`] is
//! what the command prints of the levels.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rnl_runtime::diag::Severity;
use rnl_runtime::host::{file_name, LevelKind};
use rnl_runtime::interp::obj::Priority;
use rnl_runtime::interp::unit::DEFAULT_VERSION;
use rnl_runtime::problem::{sig2, FactKind, LevelProblem, PointRef, Reference};
use rnl_runtime::solution::{write_record, Written};
use rnl_runtime::source::{dirname, normalize, relative_to, Layered, MemoryProvider, Provider};
use rnl_runtime::writeback::{
    generated_files, move_header_for, pins_std, raise_version, record_line, remove_generated_with_header,
    replace_generated_with_header, spelled_like_new_words, takes_version_of, PIN_STD_0_2,
};
use rnl_runtime::{Interpretation, Options};
use rnl_solver::{Clock, Discarded, Solved, SolverOptions, Statement, AUTHOR};

use crate::provider::path_str;

/// What `rnl solve` takes besides the file.
#[derive(Clone, Debug)]
pub struct SolveOptions {
    /// "spice" or "spectre", instead of guessing from the file.
    pub dialect: Option<String>,
    /// The formats set for files by path (`rnl_view::BuildOptions::formats`).
    pub formats: BTreeMap<String, String>,
    /// Use the standard package implicitly at the root (16.3).
    pub std: bool,
    /// Only the level with this title: a subcircuit name, or the file name.
    pub level: Option<String>,
    /// The solver's options; their canonical text is the second segment of the hash (17). Its
    /// time limit is the toolkit's default of 5 seconds unless set otherwise (the draft 0.3, 8.5).
    pub solver: SolverOptions,
}

impl Default for SolveOptions {
    fn default() -> SolveOptions {
        SolveOptions {
            dialect: None,
            formats: BTreeMap::new(),
            std: true,
            level: None,
            solver: SolverOptions::default(),
        }
    }
}

impl SolveOptions {
    /// The runtime's options the levels are read with: as a solver reads them (2.8).
    pub fn runtime(&self) -> Options {
        Options { std: self.std, ..Options::solver() }
    }
}

/// One level solved and written back.
#[derive(Clone, Debug)]
pub struct SolvedLevel {
    /// The problem solved, as the document read before this level was written back gave it.
    pub problem: LevelProblem,
    pub solved: Solved,
    /// The record as [`write_record`] gives it, with what it cannot hold (`unsolved`) and what it
    /// holds although a solver should not do it (`notes`); and, among the notes, why it is not
    /// written into the file when the level lies in a region written to be encrypted (2.2).
    pub written: Written,
    /// The record written into the file: [`Written::record`] finished by
    /// `rnl_view::finish_record`; for a level not written, the record it would have.
    pub record: String,
    /// The `hash` of the record head (17).
    pub hash: String,
    /// The path of the file the level's text is in, as it was read.
    pub path: String,
    /// The time the solver took for the level. Nothing written depends on it.
    pub time: Duration,
    /// Whether the solver sought the hints of the facts of priority low discarded, every minimal
    /// conflict group of each and its hints by statement (`SolverOptions::hints_for_low`), as
    /// `--report` asks: [`report`] prints them only then.
    pub hints_for_low: bool,
}

/// A level that is not solved, because the reading does not interpret its RNL (16.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotSolved {
    pub title: String,
    /// "file" or "subckt".
    pub kind: &'static str,
    /// The path of the file the level's text is in, as it was read.
    pub path: String,
    /// Why: the file that declares a version the reading does not interpret, and the version.
    pub reason: String,
    /// How many of [`SolveRun::levels`] come before it in the order of the document's levels.
    pub before: usize,
}

/// A file a level was written back into, with its text after every level in it was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolvedFile {
    /// The path it was read from, `/`-separated.
    pub path: String,
    pub text: String,
}

/// What [`solve`] gives.
#[derive(Clone, Debug)]
pub struct SolveRun {
    /// The file solved, `/`-separated.
    pub entry: String,
    /// The levels solved, in the order of the document's drawing levels.
    pub levels: Vec<SolvedLevel>,
    /// The levels asked for that are not solved, because their RNL is not interpreted (16.1).
    pub not_solved: Vec<NotSolved>,
    /// Every file a level was written back into, in the order it was first written.
    pub files: Vec<SolvedFile>,
    /// Every file the document was read from, normalized and in the order first read: the file
    /// solved, the files it includes, the package files named by `path=` (16.2). Not the packages
    /// of the package directories, which a file reaches by name, nor the bundled ones.
    pub read: Vec<String>,
    /// The errors of the document read as a solver reads it, before anything was written back
    /// but the files raised to 0.3 ([`SolveRun::raised`]).
    pub errors: usize,
    /// The files of 0.2 that records are written into (16.1): raised, or kept at 0.2 when they
    /// cannot be.
    pub raised: Vec<VersionRaise>,
}

/// A file of 0.2 that `rnl solve` writes a record into: a tool writes only 0.3, so it raises the
/// file to 0.3 (16.1), and solves its levels again as they read in 0.3.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionRaise {
    /// The path of the file, `/`-separated.
    pub path: String,
    /// The library section of the file that is raised, which is read as a file of its own (2.8);
    /// None for the file read whole.
    pub section: Option<String>,
    /// Whether it is raised to 0.3; when not, it stays at 0.2 and [`VersionRaise::why`] says why.
    pub raised: bool,
    /// Whether its header record got `@USE-PACKAGE(name="rnl-std", version="0.2")`, which keeps
    /// the standard series 0.2 it was read with (16.3): it does, unless it, or a file that includes
    /// it, pins a version of the standard package already.
    pub pinned: bool,
    /// Why it is not raised: that its header record cannot be changed so.
    pub why: Vec<String>,
    /// The names in it spelled like a word that 0.3 adds, whose meaning the raise may change
    /// (16.1, 16.5): a warning for each, and the file is raised all the same.
    pub clashes: Vec<String>,
    /// The files it includes that write no VERSION and take its version (16.1): raised, it has
    /// them read in 0.3 too, and each gets a warning, with one for each name in it spelled like a
    /// word that 0.3 adds. Their text is not changed.
    pub followers: Vec<Follower>,
}

/// A file level that writes no VERSION and takes the version of a file raised to 0.3, which
/// includes it (16.1): it reads in 0.3 with it ([`VersionRaise::followers`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Follower {
    /// The path of its file, `/`-separated.
    pub path: String,
    /// Its library section, which is read as a file of its own (2.8); None for the file read whole.
    pub section: Option<String>,
    /// The names in it spelled like a word that 0.3 adds, as [`VersionRaise::clashes`].
    pub clashes: Vec<String>,
}

/// Where [`write`] puts the files solved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Under this directory, each file named as it is relative to the directory of the file
    /// solved (`-o DIR`).
    Dir(PathBuf),
    /// Over the files read (`--in-place`).
    InPlace,
}

/// The clock that the time limit of the resolution is measured on (the draft 0.3, 8.5): the
/// seconds since it was made, by `std::time::Instant`, which the solver itself does not read.
#[derive(Clone, Copy, Debug)]
pub struct WallClock(Instant);

impl WallClock {
    pub fn new() -> WallClock {
        WallClock(Instant::now())
    }
}

impl Default for WallClock {
    fn default() -> WallClock {
        WallClock::new()
    }
}

impl Clock for WallClock {
    fn elapsed(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}

/// The files given by another provider, with the paths of those it gave noted.
struct Noting<'a> {
    files: &'a dyn Provider,
    read: RefCell<Vec<String>>,
}

impl Provider for Noting<'_> {
    fn auto_include(&self) -> Option<&dyn rnl_runtime::source::LibraryLookup> {
        self.files.auto_include()
    }
    fn path_key(&self, path: &str) -> String {
        self.files.path_key(path)
    }
    fn read(&self, path: &str) -> Option<String> {
        let text = self.files.read(path)?;
        let path = normalize(path);
        let mut read = self.read.borrow_mut();
        if !read.contains(&path) {
            read.push(path);
        }
        Some(text)
    }

    fn package_files(&self) -> Vec<String> {
        self.files.package_files()
    }
    fn package_candidates(&self, name: &str) -> Vec<String> {
        self.files.package_candidates(name)
    }
    fn include_candidates(&self, name: &str) -> Vec<String> {
        self.files.include_candidates(name)
    }
}

/// Reads `file` as a solver reads it, solves every drawing level, or the one `options.level`
/// names, and writes each level's record of the solver back into the text of its file: the
/// texts are in [`SolveRun::files`], and nothing is written to disk. A record of the solver that
/// applies to the level from another file, written there with scope=, is removed from that file,
/// and each file written gets the header record it lacks (16.1). A level whose RNL is not
/// interpreted, because its file or one above it declares a version the reading does not read,
/// is not solved and nothing is written into it (16.1). Fails when the file cannot be read, or
/// when no level has the title asked for. The time limit of the resolution is measured on the wall
/// clock ([`WallClock`]).
pub fn solve(file: &Path, files: &dyn Provider, options: &SolveOptions) -> Result<SolveRun, String> {
    solve_with_clock(file, files, options, &WallClock::new())
}

/// [`solve`], with the time limit of the resolution of each level measured on `clock` (the draft
/// 0.3, 8.5): the limit of a level runs from the reading when its resolution begins.
///
/// A file of 0.2 that it writes a record into is raised to 0.3, as a tool writes only 0.3 (16.1): its header
/// record says 0.3, with `@USE-PACKAGE(name="rnl-std", version="0.2")` unless it or a file that
/// includes it pins a version of the standard package, so that it keeps the series 0.2 it was read
/// with (16.3), and each name in it spelled like a word that 0.3 adds gets a warning
/// ([`spelled_like_new_words`]). The levels are then solved again from the start, as they read
/// with the files raised; a file whose header record cannot say 0.3 stays at 0.2, and its record
/// lists what it cannot hold as unsolved. A library section is read as a file (2.8): of two
/// sections of one file, only one written into is raised, and the text outside it stays as it is.
pub fn solve_with_clock(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
) -> Result<SolveRun, String> {
    solve_watched(file, files, options, clock, &|_| true)
}

/// Where a solve is: the level about to be solved, by its place among the levels solved and its
/// title, for a frontend to show while it works.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    /// The level about to be solved, counted from 0 among those the run solves.
    pub level: usize,
    /// How many levels the run solves.
    pub levels: usize,
    /// Its title: a subcircuit name, or the file name.
    pub title: String,
    /// Whether this is the second pass, after files were raised to 0.3 (16.1).
    pub again: bool,
}

/// What [`solve_watched`] fails with when its watcher stops it.
pub const CANCELLED: &str = "the solve was cancelled";

/// [`solve_with_clock`], with `watch` told of each level before it is solved: when it says false,
/// the solve stops there, nothing is given back but [`CANCELLED`], and the levels solved so far
/// are dropped. A frontend runs it on a thread of its own, shows where it is, and stops it at the
/// user's word; a level stops sooner when the clock reaches the time limit of its resolution.
pub fn solve_watched(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
    watch: &dyn Fn(&Progress) -> bool,
) -> Result<SolveRun, String> {
    let (run, wants) = pass(file, files, options, clock, &[], watch)?;
    if wants.is_empty() {
        return Ok(run);
    }
    // the files as they were read, raised
    let entry = path_str(file);
    let original =
        rnl_view::interpret_with(&entry, options.dialect.as_deref(), &options.formats, files, &options.runtime())?;
    let mut raised = Vec::new();
    let mut texts: Vec<(String, String)> = Vec::new();
    let mut paths: Vec<(&String, &String)> = Vec::new();
    for (path, file, _) in &wants {
        if !paths.contains(&(path, file)) {
            paths.push((path, file));
        }
    }
    for (path, file) in paths {
        let text = text_of(&original, files, file, path)?;
        // each file level of 0.2 of the file written into, a library section read as a file
        // (2.8), raised in one pass over the text
        let mut asked = Vec::new();
        for (_, _, section) in wants.iter().filter(|(p, f, _)| p == path && f == file) {
            let doc = &original.doc;
            let Some(level) = (0..doc.levels.len()).find(|&l| {
                let lv = &doc.levels[l];
                lv.kind == LevelKind::File && lv.file == *file && lv.section == *section
            }) else {
                continue;
            };
            let clashes = spelled_like_new_words(doc, level, &original.imported_aliases);
            asked.push((level, !pins_std(doc, level), section.clone(), clashes));
        }
        let all: Vec<(usize, bool)> = asked.iter().map(|(l, pin, _, _)| (*l, *pin)).collect();
        let (out, done) = raise_version(&original.doc, &text, &all);
        for ((level, pin, section, clashes), ok) in asked.into_iter().zip(done) {
            let why = if ok { Vec::new() } else { vec!["its header record cannot say 0.3 so".to_string()] };
            let followers = Vec::new();
            let v =
                VersionRaise { path: path.clone(), section, raised: ok, pinned: ok && pin, why, clashes, followers };
            raised.push((level, v));
        }
        if out != text {
            texts.push((path.clone(), out));
        }
    }
    // the files read are those of the first pass: the second reads the files raised from memory
    let read = run.read.clone();
    let mut run = if texts.is_empty() { run } else { pass(file, files, options, clock, &texts, watch)?.0 };
    run.read = read;
    // a file level that cannot say 0.3 itself, such as a library section without a header record,
    // takes the version of the file that includes it (16.1): when that one is raised, it reads in
    // 0.3 too, and the line of that file says so
    let reads_0_3 = |level: usize| {
        let doc = &original.doc;
        let of = |l: usize| doc.file_level(l) == level;
        run.levels.iter().any(|l| of(l.problem.level) && l.problem.version == "0.3")
    };
    // the files without a VERSION that take the version of one raised read in 0.3 with it (16.1)
    let apart: Vec<usize> = raised.iter().filter(|(_, v)| v.raised).map(|(level, _)| *level).collect();
    for (level, v) in raised.iter_mut().filter(|(_, v)| v.raised) {
        let doc = &original.doc;
        let follower = |l: usize| Follower {
            path: normalize(&doc.levels[l].path),
            section: doc.levels[l].section.clone(),
            clashes: spelled_like_new_words(doc, l, &original.imported_aliases),
        };
        v.followers = takes_version_of(doc, *level, &apart).into_iter().map(follower).collect();
    }
    run.raised = raised.into_iter().filter(|(level, v)| v.raised || !reads_0_3(*level)).map(|(_, v)| v).collect();
    Ok(run)
}

/// A file level of 0.2 that a record is written into, which is raised to 0.3 (16.1): the path and the name of its
/// file in the document, and its library section, which is read as a file (2.8).
type Want = (String, String, Option<String>);

/// One pass of [`solve_with_clock`] over the levels, with the texts of the files `raised` over
/// the files: the run, and the file levels of 0.2 whose records need 0.3, as the path and name of
/// their file in the document and their library section; those only in the first pass, when
/// nothing is raised.
fn pass(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
    raised: &[(String, String)],
    watch: &dyn Fn(&Progress) -> bool,
) -> Result<(SolveRun, Vec<Want>), String> {
    let entry = path_str(file);
    let runtime = options.runtime();
    let canonical = options.solver.canonical();
    let files = Noting { files, read: RefCell::new(Vec::new()) };
    let mut memory = MemoryProvider::new();
    for (path, text) in raised {
        memory.insert(path, text);
    }
    let mut wants: Vec<Want> = Vec::new();
    let read = |memory: &MemoryProvider| -> Result<Interpretation, String> {
        let both = Layered { layers: vec![memory, &files] };
        rnl_view::interpret_with(&entry, options.dialect.as_deref(), &options.formats, &both, &runtime)
    };
    let first = read(&memory)?;
    let errors = first.diagnostics().iter().filter(|d| d.severity == Severity::Error).count();
    let mut targets: Vec<(usize, String)> = Vec::new();
    let mut not_solved = Vec::new();
    for p in first.problems.iter().filter(|p| options.level.as_ref().is_none_or(|t| *t == p.title)) {
        match first.unsupported.get(p.level).cloned().flatten() {
            Some((at, version)) => {
                let reason = format!(
                    "{} declares the version {version}, which this reading does not interpret: none of its RNL is \
                     read, and it is kept as it is (16.1)",
                    file_name(&first.doc.levels[at].path)
                );
                let path = normalize(&first.doc.levels[p.level].path);
                not_solved.push(NotSolved {
                    title: p.title.clone(),
                    kind: p.kind,
                    path,
                    reason,
                    before: targets.len(),
                });
            }
            None => targets.push((p.level, p.title.clone())),
        }
    }
    if targets.is_empty() && not_solved.is_empty() {
        if let Some(t) = &options.level {
            return Err(format!("no drawing level is titled {t}"));
        }
    }
    // a header that the record of a level would come before moves before any level of its file is
    // written (2.8; the draft 0.3, 16.3 and 2.2), so that every record written into the file
    // follows the carrier it has where it goes, as the next solve, which finds it there, writes them
    let mut moved: Vec<String> = Vec::new();
    for (level, title) in &targets {
        let Some(p) = first.problems.iter().find(|p| p.level == *level && p.title == *title) else { continue };
        let host = &first.doc.levels[*level];
        let path = normalize(&host.path);
        if moved.contains(&path) {
            continue;
        }
        let text = text_of(&first, &Layered { layers: vec![&memory, &files] }, &host.file, &path)?;
        let out = move_header_for(&first.doc, *level, &text, &p.version);
        if out != text {
            memory.insert(&path, &out);
            moved.push(path);
        }
    }
    let mut levels = Vec::new();
    let mut solved_files: Vec<SolvedFile> = Vec::new();
    // the document read again when a header moved
    let mut current = if moved.is_empty() { Some(first) } else { None };
    let count = targets.len();
    for (k, (level, title)) in targets.into_iter().enumerate() {
        let progress = Progress { level: k, levels: count, title: title.clone(), again: !raised.is_empty() };
        if !watch(&progress) {
            return Err(CANCELLED.to_string());
        }
        // the document as written so far: the lines of this level have moved if one before it in
        // its file was written back
        let r = match current.take() {
            Some(r) => r,
            None => read(&memory)?,
        };
        let p = r
            .problems
            .iter()
            .find(|p| p.level == level && p.title == title)
            .ok_or_else(|| format!("the level {title} is not read again after the levels before it were solved"))?;
        let start = Instant::now();
        let solved = rnl_solver::solve_with_clock(p, &options.solver, clock);
        let time = start.elapsed();
        let hash = sig2(p, &canonical);
        let mut written = write_record(p, &solved.solution, AUTHOR, &hash);
        let host = &r.doc.levels[p.level];
        let path = normalize(&host.path);
        // a file of 0.2 that the record is written into is raised to 0.3 (16.1): the file level of
        // the level, a library section read as a file (2.8)
        let want: Want = (path.clone(), host.file.clone(), r.doc.levels[r.doc.file_level(p.level)].section.clone());
        let (record, texts) = {
            let both = Layered { layers: vec![&memory, &files] };
            let record = rnl_view::finish_record(&r.doc, &both, &runtime, p, &written.record);
            let text = text_of(&r, &both, &host.file, &path)?;
            let mut texts = Vec::new();
            match record_line(&r.doc, p.level, &text) {
                // a level whose own text lies in a region written to be encrypted has no place for a
                // record that is not gone once the region is encrypted (2.2): nothing is written for it
                Err(_)
                    if r.doc.levels[r.doc.file_level(p.level)].section.is_some()
                        && r.doc.levels[r.doc.file_level(p.level)].section_lines.is_none() =>
                {
                    written.notes.push(format!(
                        "the record is not written: the selected library section {} was not found in {} (2.2)",
                        r.doc.levels[r.doc.file_level(p.level)].section.as_deref().unwrap(),
                        file_name(&host.file)
                    ));
                }
                Err((a, b))
                    if r.doc.excluded.iter().any(|(f, x, y)| f == &host.file && (*x, *y) == (a, b))
                        || r.doc
                            .unselected_sections
                            .iter()
                            .any(|(l, x, y)| *l == r.doc.file_level(p.level) && (*x, *y) == (a, b)) =>
                {
                    written.notes.push(format!(
                        "the record is not written: {} lies in a host region excluded from writing, lines {a} to {b} of {} (2.2)",
                        p.title,
                        file_name(&host.file)
                    ));
                }
                Err((a, b)) => written.notes.push(format!(
                    "the record is not written: {} lies in a region written to be encrypted, lines {a} to {b} of {}, \
                     where a record is gone once the region is encrypted (2.2)",
                    p.title,
                    file_name(&host.file)
                )),
                Ok(_) => {
                    // the record and the header in one pass, at the positions of this reading: a
                    // header below the end of a subcircuit written moves to line 2, to stay the first
                    // record (2.8)
                    let out = replace_generated_with_header(&r.doc, p.level, &text, &record, &p.version);
                    texts.push((path.clone(), out));
                    if p.version == "0.2" && raised.is_empty() && !wants.contains(&want) {
                        wants.push(want);
                    }
                    // a record of the solver for this level written with scope= in another file (2.6):
                    // removed, and nothing is written there, so that file keeps its version
                    for other in generated_files(&r.doc, p.level).into_iter().filter(|f| *f != host.file) {
                        let Some(lv) = r.doc.levels.iter().find(|l| l.file == other) else { continue };
                        let at = normalize(&lv.path);
                        let text = text_of(&r, &both, &other, &at)?;
                        let out = remove_generated_with_header(&r.doc, p.level, &other, &text, &version_of(&r, &other));
                        texts.push((at, out));
                    }
                }
            }
            (record, texts)
        };
        for (at, out) in texts {
            memory.insert(&at, &out);
            match solved_files.iter_mut().find(|f| f.path == at) {
                Some(f) => f.text = out,
                None => solved_files.push(SolvedFile { path: at, text: out }),
            }
        }
        let hints_for_low = options.solver.hints_for_low;
        levels.push(SolvedLevel { problem: p.clone(), solved, written, record, hash, path, time, hints_for_low });
    }
    let packages: Vec<String> = files.files.package_files().iter().map(|p| normalize(p)).collect();
    let read: Vec<String> = files.read.take().into_iter().filter(|p| !packages.contains(p)).collect();
    Ok((SolveRun { entry, levels, not_solved, files: solved_files, read, errors, raised: Vec::new() }, wants))
}

/// The text of a file as the reading `r` found it, which the positions of its records refer to; read
/// from `files` when the reading kept none.
fn text_of(r: &Interpretation, files: &dyn Provider, file: &str, path: &str) -> Result<String, String> {
    match r.doc.source(file) {
        Some(t) => Ok(t.to_string()),
        None => files.read(path).ok_or_else(|| format!("cannot read {path}")),
    }
}

/// The version a file of the document was read in (16.1): that of a level in it, else the tool's
/// default, which a file without a header is read in.
fn version_of(r: &Interpretation, file: &str) -> String {
    r.problems
        .iter()
        .find(|q| r.doc.levels.get(q.level).is_some_and(|l| l.file == file))
        .map_or_else(|| DEFAULT_VERSION.to_string(), |q| q.version.clone())
}

/// Where each file of a run goes: under `-o DIR` by its name relative to the directory of the
/// file solved, or over itself. A file outside that directory, such as one included from `../`,
/// has no name under `DIR`: that is an error, and nothing is written.
pub fn destinations(run: &SolveRun, target: &Target) -> Result<Vec<PathBuf>, String> {
    let base = dirname(&normalize(&run.entry));
    run.files
        .iter()
        .map(|f| match target {
            Target::InPlace => Ok(PathBuf::from(&f.path)),
            Target::Dir(dir) => match name_in(&f.path, &base) {
                Some(rel) => Ok(dir.join(rel)),
                None => Err(format!(
                    "{} is not in the directory of {}, so it has no name under {}; write it with --in-place",
                    f.path,
                    run.entry,
                    dir.display()
                )),
            },
        })
        .collect()
}

/// Every path the solve may write, including unchanged dependencies copied under `-o`.
/// Used by auxiliary outputs so they cannot collide with either writeback or copying.
pub fn all_destinations(run: &SolveRun, target: &Target) -> Result<Vec<PathBuf>, String> {
    let mut out = destinations(run, target)?;
    if let Target::Dir(dir) = target {
        let base = dirname(&normalize(&run.entry));
        for path in &run.read {
            if let Some(relative) = name_in(path, &base) {
                let path = dir.join(relative);
                if !out.contains(&path) {
                    out.push(path);
                }
            }
        }
    }
    Ok(out)
}

/// `path` relative to the directory `dir`, both normalized; "" is the working directory.
fn name_in(path: &str, dir: &str) -> Option<String> {
    let path = normalize(path);
    if dir.is_empty() {
        let outside =
            path.starts_with('/') || path == ".." || path.starts_with("../") || path.as_bytes().get(1) == Some(&b':');
        return (!outside).then_some(path);
    }
    relative_to(&path, dir).map(String::from)
}

/// What [`write`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Writing {
    /// The files a level was written back into, where they were written, in the order of
    /// [`SolveRun::files`].
    pub written: Vec<PathBuf>,
    /// Under `-o DIR`, the other files the document reads from the directory of the file solved
    /// ([`SolveRun::read`]), copied as they are, so that the files under `DIR` read as the files
    /// solved do: the files included, the packages named by `path=`.
    pub copied: Vec<PathBuf>,
    /// Under `-o DIR`, the files the document reads from outside the directory of the file
    /// solved: they have no name under `DIR` and are not copied, so the files there reach them
    /// only where they name them by an absolute path.
    pub outside: Vec<String>,
    /// What stopped the writing. The files listed were written before it.
    pub error: Option<String>,
}

/// Encode in the source encoding, refusing an unrepresentable character (§3.1).
pub fn encode_as(original: Option<&[u8]>, text: &str) -> Result<Vec<u8>, String> {
    crate::codec::encode(original, text, crate::codec::OutputEncoding::Preserve)
}

/// Writes the files of a run where `target` says, creating the directories they need: the files
/// a level was written back into, each in the encoding its file had ([`encode_as`]), then, under
/// `-o DIR` and when there is any, a copy of each other file the document reads from the
/// directory of the file solved. A copy onto the file itself is left out. It stops at the first
/// file it cannot write, and says what it wrote before. Nothing is written when a file solved
/// has no place under `DIR` ([`destinations`]).
pub fn write(run: &SolveRun, target: &Target) -> Writing {
    write_with_encoding(run, target, crate::codec::OutputEncoding::Preserve)
}

/// Write with the explicit encoding choice. Encoding is checked for every rewritten file before
/// any write; untouched includes copied into a directory keep their original bytes.
pub fn write_with_encoding(run: &SolveRun, target: &Target, encoding: crate::codec::OutputEncoding) -> Writing {
    let mut out = Writing::default();
    let to = match destinations(run, target) {
        Ok(to) => to,
        Err(e) => {
            out.error = Some(e);
            return out;
        }
    };
    let encoded: Result<Vec<Vec<u8>>, String> = run
        .files
        .iter()
        .map(|f| {
            crate::codec::encode_file(Some(Path::new(&f.path)), &f.text, encoding)
                .map_err(|e| format!("{}: {e}", f.path))
        })
        .collect();
    let encoded = match encoded {
        Ok(encoded) => encoded,
        Err(e) => {
            out.error = Some(e);
            return out;
        }
    };
    let make_parent = |path: &Path| -> Result<(), String> {
        match path.parent().filter(|p| !p.as_os_str().is_empty()) {
            Some(parent) => {
                std::fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))
            }
            None => Ok(()),
        }
    };
    for (bytes, path) in encoded.iter().zip(to) {
        let done = make_parent(&path)
            .and_then(|()| std::fs::write(&path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display())));
        if let Err(e) = done {
            out.error = Some(e);
            return out;
        }
        out.written.push(path);
    }
    let Target::Dir(dir) = target else { return out };
    if run.files.is_empty() {
        return out;
    }
    let base = dirname(&normalize(&run.entry));
    for path in run.read.iter().filter(|p| !run.files.iter().any(|f| f.path == **p)) {
        let Some(rel) = name_in(path, &base) else {
            out.outside.push(path.clone());
            continue;
        };
        let copy = dir.join(rel);
        let same = match (std::fs::canonicalize(path), std::fs::canonicalize(&copy)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        };
        if same {
            continue;
        }
        let done = make_parent(&copy).and_then(|()| {
            std::fs::copy(path, &copy).map(|_| ()).map_err(|e| format!("cannot copy {path} to {}: {e}", copy.display()))
        });
        if let Err(e) = done {
            out.error = Some(e);
            return out;
        }
        out.copied.push(copy);
    }
    out
}

/// A point of a fact, by its object's name when it has one, such as `%t` for a text named so, else
/// as diagnostics show its object: `#M1`, `#M1:d`, `{#M1 #M2}` for a centre.
fn point(p: &LevelProblem, r: &PointRef) -> String {
    let label = |k: usize| {
        p.objects.get(k).map_or_else(
            || format!("object {k}"),
            |o| match &o.reference {
                Reference::Name(n) => n.clone(),
                _ => o.label.clone(),
            },
        )
    };
    match r {
        PointRef::Origin(k) => label(*k),
        PointRef::Pin { object, terminal, .. } => format!("{}:{terminal}", label(*object)),
        PointRef::Centre(points) => format!("{{{}}}", points.iter().map(|q| point(p, q)).collect::<Vec<_>>().join(" ")),
    }
}

/// One fact as the report shows it: its rank, what it says, its priority and where it is
/// written, and the definitions of the usages it was expanded from, such as
/// `[9] ALIGN y {#M1 #M2} (normal, rnl-std-layout.rnl:10, from $MIRROR_PAIR)`.
pub fn describe_fact(p: &LevelProblem, k: usize) -> String {
    let Some(f) = p.facts.get(k) else { return format!("[{k}]") };
    let mut context = vec![f.priority.as_str().to_string()];
    if let Some(l) = &f.source {
        context.push(format!("{}:{}", file_name(&l.file), l.line));
    }
    if !f.chain.is_empty() {
        let defs: Vec<&str> = f.chain.iter().filter_map(|u| p.usages.get(*u)).map(|u| u.definition.as_str()).collect();
        context.push(format!("from {}", defs.join(" in ")));
    }
    format!("{} ({})", fact_what(p, k), context.join(", "))
}

/// A fact by its rank and what it says, without its context, such as `[9] ALIGN y {#M1 #M2}`.
pub(crate) fn fact_what(p: &LevelProblem, k: usize) -> String {
    let Some(f) = p.facts.get(k) else { return format!("[{k}]") };
    let list = |ms: &[PointRef]| format!("{{{}}}", ms.iter().map(|m| point(p, m)).collect::<Vec<_>>().join(" "));
    let objects = |ks: &[usize]| list(&ks.iter().map(|k| PointRef::Origin(*k)).collect::<Vec<_>>());
    let what = match &f.kind {
        FactKind::Place { at, axis, value } => format!("the {} of {} = {}", axis.as_str(), point(p, at), num(*value)),
        FactKind::PointCoordinate { point: k, axis, value } => {
            format!("the {} of {} = {}", axis.as_str(), point(p, &PointRef::Origin(*k)), num(*value))
        }
        // the `dx` or `dy` of a text, from the point it is attached to (13.3)
        FactKind::Offset { text, from, axis, value } => format!(
            "the d{} of {} from {} = {}",
            axis.as_str(),
            point(p, &PointRef::Origin(*text)),
            point(p, from),
            num(*value)
        ),
        FactKind::Orient { object, code } => {
            format!("the orientation {} of {}", code.as_str(), point(p, &PointRef::Origin(*object)))
        }
        FactKind::Align { axis, members } => format!("ALIGN {} {}", axis.as_str(), list(members)),
        FactKind::Distribute { axis, members } => format!("DISTRIBUTE {} {}", axis.as_str(), list(members)),
        FactKind::Order { axis, sense, members } => {
            format!("ORDER {} {} {}", axis.as_str(), sense.as_str(), list(members))
        }
        FactKind::Distance { axis, sense, min, max, from, to } => {
            let mut bounds = String::new();
            if let Some(m) = min {
                bounds.push_str(&format!(" min={}", num(*m)));
            }
            if let Some(m) = max {
                bounds.push_str(&format!(" max={}", num(*m)));
            }
            format!("DISTANCE {} {}{bounds} {} {}", axis.as_str(), sense.as_str(), list(from), list(to))
        }
        FactKind::OrientRelation { relation, from, to } => {
            format!("ORIENT-RELATION {} {} {}", relation.as_str(), objects(from), objects(to))
        }
    };
    // a handle, such as %c1, or a label of its own; the label of a coordinate, an offset or an
    // orientation says what the fact says, so the fact is described once
    let own = matches!(
        f.kind,
        FactKind::Place { .. } | FactKind::PointCoordinate { .. } | FactKind::Offset { .. } | FactKind::Orient { .. }
    );
    let named = if own || f.label == f.kind.name() || f.label == what { what } else { format!("{} = {what}", f.label) };
    format!("[{k}] {named}")
}

/// A statement as the hints of 8.5 name it (the draft 0.3, 8.5; 7.6): where it is written, such
/// as `T4-miller-conflicts.sp:37`; for one in the body of a definition, the usage written in the
/// level that it was expanded from, the first of its chain, where it is written, with how the
/// level writes its roles, a role that cannot be referred to left out, and the line of the
/// definition, such as `the usage of $MIRROR_PAIR at T4-miller-conflicts.sp:22 with #M1(a) #M2(b) %ax(axis)
/// (rnl-std-layout.rnl:10 of its definition)`. When the
/// statement is in the body of a definition used inside another, the line is of the innermost,
/// and every definition of the chain is named, outermost first, as for a fact: `%o1, the usage of
/// $OUTER at p.sp:8 with #R1(a) #R2(b) (p.sp:6 of $PAIR, from $OUTER in $PAIR)`. For the coordinates of a
/// point, which have no statement of their own, the point.
pub(crate) fn statement_name(p: &LevelProblem, s: &Statement) -> String {
    if let Some(k) = s.point {
        return format!("the coordinates of {}", point(p, &PointRef::Origin(k)));
    }
    let place = format!("{}:{}", file_name(&s.file), s.line);
    let Some(u) = s.chain.first().and_then(|u| p.usages.get(*u)) else { return place };
    let mut name = match &u.reference {
        Reference::Name(n) => format!("{n}, the usage of {}", u.definition),
        _ => format!("the usage of {}", u.definition),
    };
    if let Some(at) = &u.source {
        name.push_str(&format!(" at {}:{}", file_name(&at.file), at.line));
    }
    let roles: Vec<String> =
        u.roles.iter().filter_map(|r| r.written.as_ref().map(|w| format!("{w}({})", r.role))).collect();
    if !roles.is_empty() {
        name.push_str(&format!(" with {}", roles.join(" ")));
    }
    let defs: Vec<&str> = s.chain.iter().filter_map(|u| p.usages.get(*u)).map(|u| u.definition.as_str()).collect();
    match defs.as_slice() {
        [_, .., inner] => format!("{name} ({place} of {inner}, from {})", defs.join(" in ")),
        _ => format!("{name} ({place} of its definition)"),
    }
}

/// Some ranks as a list: `[3]`, `[0] and [4]`, `[0], [2] and [3]`, or with another last word.
fn ranks(ks: &[usize], last: &str) -> String {
    let names: Vec<String> = ks.iter().map(|k| format!("[{k}]")).collect();
    match names.split_last() {
        None => String::new(),
        Some((one, [])) => one.clone(),
        Some((end, rest)) => format!("{} {last} {end}", rest.join(", ")),
    }
}

/// What the hint of a fact discarded says of it (the draft 0.3, 8.5): every minimal group of facts
/// kept before it that it conflicts with, such as `it conflicts with [3]; or with [0] and [4]`,
/// and that the list is not complete when the time limit cut it.
pub(crate) fn conflicts(d: &Discarded) -> String {
    let mut why = if d.groups.is_empty() {
        "no group of facts it conflicts with was found within the time limit of the solver".to_string()
    } else {
        let groups: Vec<String> = d.groups.iter().map(|g| ranks(g, "and")).collect();
        format!("it conflicts with {}", groups.join("; or with "))
    };
    if !d.complete {
        why.push_str(" (the list is not complete: the time limit was reached)");
    }
    why
}

fn num(v: f64) -> String {
    rnl_runtime::geom::fmt_num(v)
}

/// How many errors the solver reported over the levels of a run: the facts that contradict
/// themselves and fail (the draft 0.3, 8.5; 7.6). `rnl solve` exits with 1 when there is one, or
/// an error of the netlist ([`SolveRun::errors`]).
pub fn errors(run: &SolveRun) -> usize {
    run.levels.iter().map(|l| l.solved.report.errors.len()).sum()
}

/// What the command prints of the levels of a run, in the order of the document's levels: each
/// level solved as [`report`] gives it, each level not solved in one line.
pub fn lines(run: &SolveRun, all: bool) -> Vec<String> {
    let mut out: Vec<String> = run.raised.iter().flat_map(raised).collect();
    for k in 0..=run.levels.len() {
        for n in run.not_solved.iter().filter(|n| n.before == k) {
            out.push(not_solved(n));
        }
        if let Some(level) = run.levels.get(k) {
            out.extend(report(level, all));
        }
    }
    out
}

/// The lines of a file of 0.2 that a record is written into: raised, with a warning for each name in it spelled like
/// a word that 0.3 adds, or kept at 0.2 with a warning (16.1).
pub fn raised(v: &VersionRaise) -> Vec<String> {
    let named = |path: &str, section: &Option<String>| match section {
        Some(s) => format!("{} ({s})", file_name(path)),
        None => file_name(path).to_string(),
    };
    let name = named(&v.path, &v.section);
    if v.raised {
        let series = if v.pinned {
            format!("its header record keeps the standard series 0.2 it was read with: {PIN_STD_0_2} (16.3)")
        } else {
            "the version of the standard package that it pins stays (16.3)".to_string()
        };
        let mut out = vec![format!("{name}: raised from 0.2 to 0.3, as a tool writes only 0.3 (16.1); {series}")];
        out.extend(v.clashes.iter().map(|c| {
            format!("warning: {name}: {c}, has the spelling of a word that 0.3 adds (2.8): raised to 0.3, it may mean something else (16.1)")
        }));
        for f in &v.followers {
            let child = named(&f.path, &f.section);
            out.push(format!(
                "warning: {child}: it writes no VERSION and takes that of {name}: it reads in 0.3 too, its text unchanged (16.1); @VERSION(\"0.2\") written in its header record keeps it at 0.2"
            ));
            out.extend(f.clashes.iter().map(|c| {
                format!("warning: {child}: {c}, has the spelling of a word that 0.3 adds (2.8): read in 0.3, it may mean something else (16.1)")
            }));
        }
        out
    } else {
        vec![format!(
            "warning: {name} stays at 0.2, so what only 0.3 can write is unsolved (16.1): {}",
            v.why.join("; ")
        )]
    }
}

/// The line of a level that is not solved.
pub fn not_solved(n: &NotSolved) -> String {
    format!(
        "level {} ({} in {}): not solved, and nothing is written for it: {}",
        n.title,
        n.kind,
        file_name(&n.path),
        n.reason
    )
}

/// What the command prints of one level: its title, with how many facts are kept, discarded and,
/// when there are any, failed; the facts kept and discarded, every fact with `all`, a fact that
/// fails among them, otherwise the discarded ones, each with its hint (the draft 0.3, 8.5): every
/// minimal group of facts it conflicts with, and whether the list is complete; the hints by
/// statement, each statement whose facts took part in discarding with the facts discarded because
/// of it; the hints of a fact of priority low, and of the statements that only such facts lose,
/// only with `all`, when the solver sought them ([`SolvedLevel::hints_for_low`], 7.6), and with
/// `all` when it did not, the line of such a fact says so; the errors of the solver, the facts
/// that contradict themselves, each on a line of its own beginning with `error:`, and its
/// warnings, each beginning with `warning:` (7.6); what the record cannot hold, the ROUTEs left in
/// parts, the inert objectives, the notes, the work and the time.
pub fn report(level: &SolvedLevel, all: bool) -> Vec<String> {
    let p = &level.problem;
    let r = &level.solved.report;
    let failed = if r.failed.is_empty() { String::new() } else { format!(", {} failed", r.failed.len()) };
    let mut out = vec![format!(
        "level {} ({} in {}): {} kept, {} discarded{failed} of {}",
        p.title,
        p.kind,
        file_name(&level.path),
        r.kept.len(),
        r.discarded.len(),
        count(p.facts.len(), "fact")
    )];
    // Discarding a fact of priority low is expected, and its hints may be left out (the draft 0.3,
    // 8.5; 7.6): they are printed with `all` alone, the hint of the fact and those of the
    // statements that only such facts lose, when the solver sought them.
    let hinted = |f: usize| (all && level.hints_for_low) || p.facts.get(f).is_none_or(|f| f.priority != Priority::Low);
    // a fact discarded and its hint: every minimal group of facts kept before it that it conflicts
    // with, each fact of them described once below it, and whether the list is complete
    let discarded = |out: &mut Vec<String>, k: usize, indent: &str| {
        let Some(d) = r.discarded.iter().find(|d| d.fact == k) else { return };
        let why = if d.undecided {
            "not decided within the time limit of the solver (8.5)".to_string()
        } else if !hinted(k) && all {
            // every fact, of a level solved without the groups of the low facts sought
            "of priority low, which is expected; the solver did not seek what it conflicts with (hints_for_low, \
             which --report sets; 8.5)"
                .to_string()
        } else if !hinted(k) {
            "of priority low, which is expected; --report lists what it conflicts with (8.5)".to_string()
        } else {
            conflicts(d)
        };
        out.push(format!("{indent}discarded {}: {why}", describe_fact(p, k)));
        if hinted(k) {
            for c in d.in_groups() {
                out.push(format!("{indent}    {}", describe_fact(p, c)));
            }
        }
    };
    if all {
        out.push("  facts, in the order of 8.5:".to_string());
        for k in 0..p.facts.len() {
            if r.kept.contains(&k) {
                out.push(format!("    kept {}", describe_fact(p, k)));
            } else if r.failed.contains(&k) {
                out.push(format!("    failed {}: it contradicts itself (8.5, 7.6)", describe_fact(p, k)));
            } else {
                discarded(&mut out, k, "    ");
            }
        }
    } else {
        for d in &r.discarded {
            discarded(&mut out, d.fact, "  ");
        }
    }
    let section = |out: &mut Vec<String>, head: &str, lines: Vec<String>| {
        if !lines.is_empty() {
            out.push(format!("  {head}:"));
            out.extend(lines.into_iter().map(|l| format!("    {l}")));
        }
    };
    // the hints by statement (the draft 0.3, 8.5): each statement whose facts are in a group, with
    // the facts discarded because of it
    let mut statements = Vec::new();
    for (s, hint) in &r.statements {
        let lost: Vec<&Discarded> =
            r.discarded.iter().filter(|d| hint.discarded.contains(&d.fact) && hinted(d.fact)).collect();
        let facts: Vec<String> = hint
            .facts
            .iter()
            .filter(|k| lost.iter().any(|d| d.in_groups().contains(k)))
            .map(|k| fact_what(p, *k))
            .collect();
        if lost.is_empty() || facts.is_empty() {
            continue;
        }
        let lost: Vec<String> = lost.iter().map(|d| format!("[{}]", d.fact)).collect();
        statements.push(format!(
            "{} {} took part in discarding {}",
            statement_name(p, s),
            facts.join(" and "),
            lost.join(", ")
        ));
    }
    let cut = r.discarded.iter().any(|d| !d.undecided && !d.complete && hinted(d.fact));
    let head = if cut {
        "statements whose facts took part in discarding (8.5; the list is not complete: the time limit was reached)"
    } else {
        "statements whose facts took part in discarding (8.5)"
    };
    section(&mut out, head, statements);
    out.extend(r.errors.iter().map(|e| format!("  error: {e}")));
    out.extend(r.warnings.iter().map(|w| format!("  warning: {w}")));
    section(&mut out, "unsolved", level.written.unsolved.iter().map(|u| u.to_string()).collect());
    section(
        &mut out,
        "unrouted",
        r.unrouted
            .iter()
            .map(|u| {
                let why = if u.budget { "the budget of the router ran out" } else { "no path joins them" };
                format!("the net {}: left in {} parts, {why}", u.net, u.parts)
            })
            .collect(),
    );
    section(&mut out, "inert objectives", r.inert_objectives.clone());
    section(&mut out, "notes", r.notes.iter().chain(&level.written.notes).cloned().collect());
    let w = &r.work;
    out.push(format!(
        "  work: {}, {}, {}, {}",
        count(w.resolve_nodes as usize, "resolve node"),
        count(w.pivots as usize, "pivot"),
        count(w.candidates as usize, "candidate"),
        count(w.astar_expansions as usize, "A* expansion")
    ));
    out.push(format!("  time: {:.3} s", level.time.as_secs_f64()));
    out.push(format!("  hash: {}", level.hash));
    out
}

/// A count and its noun, singular for one.
fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_under_the_directory_of_the_file_solved() {
        assert_eq!(name_in("/w/a/t.sp", "/w/a").as_deref(), Some("t.sp"));
        assert_eq!(name_in("/w/a/sub/x.inc", "/w/a").as_deref(), Some("sub/x.inc"));
        assert_eq!(name_in("/w/x.inc", "/w/a"), None);
        assert_eq!(name_in("D:/w/a/t.sp", "D:/w/a").as_deref(), Some("t.sp"));
        // a bare file name is in the working directory
        assert_eq!(name_in("t.sp", "").as_deref(), Some("t.sp"));
        assert_eq!(name_in("./sub/x.inc", "").as_deref(), Some("sub/x.inc"));
        assert_eq!(name_in("../x.inc", ""), None);
        assert_eq!(name_in("/x.inc", ""), None);
        assert_eq!(name_in("D:/x.inc", ""), None);
    }

    #[test]
    fn a_statement_is_named_by_its_usage_and_every_definition_it_is_in() {
        // %row is written in $PAIR, which $OUTER uses as %in, which the level uses as %o1: the line
        // is of $PAIR, and both definitions are named, as for a fact (the draft 0.3, 8.5; 7.6)
        let text = "* nested\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\nR2 b c 1k\n* @RNL {\n\
                    *   $PAIR = @DEFINE(roles=[\"a\", \"b\"]) { %row = {?a ?b} @ALIGN(\"y\") };\n\
                    *   $OUTER = @DEFINE(roles=[\"a\", \"b\"]) { %in = ?a(a) ?b(b) @AS $PAIR };\n\
                    *   %o1 = #R1(a) #R2(b) @AS $OUTER\n* }\n.end\n";
        let files = MemoryProvider::new().with("/w/p.sp", text);
        let options = Options { std: false, ..Options::solver() };
        let r = crate::interpret_file(Path::new("/w/p.sp"), None, &files, &options).unwrap();
        let p = &r.problems[0];
        let f = p.facts.iter().position(|f| f.label == "%row").unwrap();
        let s = Statement::of(p, f);
        assert_eq!(s.chain.len(), 2, "{s:?}");
        let named = "%o1, the usage of $OUTER at p.sp:8 with #R1(a) #R2(b) (p.sp:6 of $PAIR, from $OUTER in $PAIR)";
        assert_eq!(statement_name(p, &s), named);
        assert!(describe_fact(p, f).ends_with("(normal, p.sp:6, from $OUTER in $PAIR)"), "{}", describe_fact(p, f));
        // a role that cannot be referred to is left out, not shown as `?`
        let mut q = p.clone();
        q.usages[s.chain[0]].roles[0].written = None;
        let named = "%o1, the usage of $OUTER at p.sp:8 with #R2(b) (p.sp:6 of $PAIR, from $OUTER in $PAIR)";
        assert_eq!(statement_name(&q, &s), named);
        // a statement in the body of a definition the level uses itself
        let one = Statement { chain: s.chain[..1].to_vec(), ..s.clone() };
        assert_eq!(
            statement_name(p, &one),
            "%o1, the usage of $OUTER at p.sp:8 with #R1(a) #R2(b) (p.sp:6 of its definition)"
        );
    }

    #[test]
    fn a_text_is_written_back_in_the_encoding_of_its_file() {
        let text = "* a \u{b5}A\n.end\n";
        // UTF-8, with and without its byte order mark
        assert_eq!(encode_as(None, text).unwrap(), text.as_bytes());
        assert_eq!(encode_as(Some(b"* old\n"), text).unwrap(), text.as_bytes());
        let marked = [b"\xef\xbb\xbf".as_slice(), b"* old\n"].concat();
        assert_eq!(encode_as(Some(&marked), text).unwrap(), [b"\xef\xbb\xbf".as_slice(), text.as_bytes()].concat());
        // Latin-1, as long as every character is one of it
        let latin1 = b"* old \xb5A\n";
        assert_eq!(encode_as(Some(latin1), text).unwrap(), b"* a \xb5A\n.end\n");
        let wide = "* \u{3a9}\n";
        assert!(encode_as(Some(latin1), wide).unwrap_err().contains("Latin-1"));
    }
}
