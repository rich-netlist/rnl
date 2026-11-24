//! `rnl solve`: every drawing level of a document solved, and its record of the solver written
//! back (standard 2.8, 8.5, 13.7, 14.8, 16.1, chapter 17).
//!
//! The document is read as a solver reads it, `Options::solver()`, without the records with
//! `type="generated"` (2.8). Each level is solved by `rnl-solver`; its record is written with
//! `solution::write_record`, the hash `sig2` of its problem and of the solver's canonical options in
//! the head, finished with `rnl_view::finish_record_with_formats`, which adds the check of its
//! numbers (5.9), and put in place of the level's records of the solver with
//! `writeback::replace_generated_with_header`, at the end of the level's own text (17). A record of
//! the solver written for the level with scope= in another file, which a solver does not write
//! (2.6) but which applies to the level all the same, is removed from that file. Every file written
//! has its header record and its version (16.1), added when it has none, in the same pass as the
//! record, at the positions the reading gives; but a file read only through library sections,
//! where what lies outside them is not read (2.2), gets no new header. The header record stays the
//! first record of its file (2.8): one written after a subcircuit that gets its record moves where
//! a new header goes, to line 2 (16.3 and 2.2), before any level of its file is
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
//! The resolution of each level is limited in time by the solver's `time_limit` (8.5), 5 seconds unless `--solver-options` says otherwise, measured on the wall clock
//! ([`WallClock`]): a fact whose check does not finish within it is discarded undecided, and the
//! report of the level has a warning for it (7.6); a fact whose minimal conflict groups are not
//! all found within it has those found, and its hint says that the list is not complete.
//! `time_limit=none` gives the resolution of 8.5 exactly, the same on every run, however long it
//! takes.
//!
//! The report of a level gives each fact discarded with every minimal group of facts it conflicts
//! with, and each statement whose facts are in such a group with the facts discarded because of it
//! (8.5); the hints of a fact of priority low, and of the statements that only such
//! facts lose, only with `--report` ([`report`]), as 7.6 allows them to be left out. Without it the
//! solver does not seek the groups of such a fact beyond its first, which can cost far more than
//! the rest of the resolution (`SolverOptions::hints_for_low`), and a note says so.
//!
//! A fact that has no solution on its own contradicts itself and fails (8.5; 7.6):
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
//! [`solve`] does the work without writing a file; [`write`] puts the texts where `-o DIR` or
//! `--in-place` says, with the other files the document reads copied under `DIR`; [`lines`] is
//! what the command prints of the levels.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use rnl_runtime::diag::{codes, Diag, Loc, Severity};
use rnl_runtime::host::file_name;
use rnl_runtime::interp::obj::Priority;
use rnl_runtime::interp::unit::DEFAULT_VERSION;
use rnl_runtime::problem::{sig2, FactKind, LevelProblem, PointRef, Reference};
use rnl_runtime::solution::{write_record, Written};
use rnl_runtime::source::{dirname, normalize, relative_to, Layered, MemoryProvider, Provider};
use rnl_runtime::writeback::{
    generated_files, move_header_for, record_line, remove_generated_with_header, replace_generated_with_header,
};
use rnl_runtime::{Interpretation, Options};
use rnl_solver::{Clock, Discarded, RunLimit, Solved, SolverOptions, Statement, StopReason, AUTHOR};

mod check;
pub(crate) use check::check_settings_read;
pub use check::{
    check_lines, check_settings, named_syntax, setting_json, Checked, CheckedLevel, CheckedProblem, CheckedSetting,
    ToolSettings,
};

use crate::provider::path_str;

/// What `rnl solve` takes besides the file.
#[derive(Clone, Debug)]
pub struct SolveOptions {
    /// "spice" or "spectre", instead of guessing from the file.
    pub dialect: Option<String>,
    /// The formats set for files by path (`rnl_view::BuildOptions::formats`).
    pub formats: BTreeMap<String, String>,
    /// Allow the standard package that `include_std` adds (16.3).
    pub std: bool,
    /// Add the standard package to the main file and persist it when that file is written (16.3).
    pub include_std: bool,
    /// Only the level with this title: a subcircuit name, or the file name.
    pub level: Option<String>,
    /// With `level`, the file of the document that the level is written in, as the document names
    /// it (`rnl_view::SourceFile::path`): only that file's levels are read, in the context of the
    /// file solved ([`Options::only_file`]), as a viewer that reads the file shown alone reads it.
    /// Each level so read is the one the whole document has (`rnl_runtime::per_file`), so what is
    /// solved and written is the same; the preview is the Document of that file
    /// ([`crate::preview::Preview::file`]). Not read without `level`.
    pub file: Option<String>,
    /// The solver's options; their canonical text is the second segment of the hash (17). Its
    /// time limit is the toolkit's default of 5 seconds unless set otherwise (8.5).
    pub solver: SolverOptions,
    /// The whole file's time (`--timeall`): the seconds that the levels of 0.4 of a time limit, or
    /// of none, share, on the clock of the solve. Each has at most what is left of them shared
    /// evenly among those still to solve, itself among them, and only its seed once they are
    /// spent. A level of a limit of work, or of its seed, is solved as its limit says, its time
    /// counted. A respace does not read it. None by default: each level has its own limit only.
    pub total_seconds: Option<f64>,
}

impl Default for SolveOptions {
    fn default() -> SolveOptions {
        SolveOptions {
            dialect: None,
            formats: BTreeMap::new(),
            std: true,
            include_std: false,
            level: None,
            file: None,
            solver: SolverOptions::default(),
            total_seconds: None,
        }
    }
}

impl SolveOptions {
    /// The runtime's options the levels are read with: as a solver reads them (2.8).
    pub fn runtime(&self) -> Options {
        Options {
            std: self.std,
            include_std: self.include_std,
            solver_config: self
                .solver
                .settings
                .overrides
                .iter()
                .filter(|s| !rnl_solver::is_run_setting(&s.key))
                .cloned()
                .collect(),
            only_file: self.level.as_ref().and(self.file.clone()),
            ..Options::solver()
        }
    }
}

/// One level solved and written back.
#[derive(Clone, Debug)]
pub struct SolvedLevel {
    /// The problem solved, as the document read before this level was written back gave it.
    pub problem: LevelProblem,
    pub solved: Solved,
    /// False when an external solver supplied no fact or work report.
    pub report_available: bool,
    /// The record as [`write_record`] gives it, with what it cannot hold (`unsolved`) and what it
    /// holds although a solver should not do it (`notes`); and, among the notes, why it is not
    /// written into the file when the level lies in a region written to be encrypted (2.2).
    pub written: Written,
    /// Draft 0.4 errors that prevent this level's record from being written.
    pub write_errors: Vec<Diag>,
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
    /// The errors of the document read as a solver reads it, before anything was written back.
    pub errors: usize,
    /// Readings the solve made that a caller can take instead of reading the same texts again,
    /// as [`crate::preview`] does; empty when the run is not of a solve.
    pub readings: Readings,
}

/// Readings of the document a solve made, each with what it was read from, so that a caller reads
/// the same texts with the same options only once ([`SolveRun::readings`]). Each is the reading
/// those texts give: one taken in place of reading again gives what reading again gives.
#[derive(Clone, Default)]
pub struct Readings {
    /// The files as they were, read as a solver reads them ([`SolveOptions::runtime`]), before
    /// anything was written back.
    pub(crate) before: Option<Rc<Interpretation>>,
    /// The last check that a level's record reads back: the texts held over the files then, by
    /// path, and the document read with them, its records and its problems, with the files it
    /// missed.
    pub(crate) written: Option<(BTreeMap<String, String>, rnl_view::DocumentReading)>,
}

impl std::fmt::Debug for Readings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Readings")
            .field("before", &self.before.is_some())
            .field("written", &self.written.as_ref().map(|(texts, _)| texts.keys().collect::<Vec<_>>()))
            .finish()
    }
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

/// The clock that the time limit of the resolution is measured on (8.5): the
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
    fn check_every(&self) -> u64 {
        rnl_solver::WALL_CLOCK_CHECK_EVERY
    }
    fn elapsed(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}

/// The time a step takes, for the report: none on WebAssembly, which has no clock of its own; a
/// solve there is measured on the clock its caller gives.
pub(crate) struct Stopwatch(#[cfg(not(target_arch = "wasm32"))] Instant);

impl Stopwatch {
    pub(crate) fn start() -> Stopwatch {
        Stopwatch(
            #[cfg(not(target_arch = "wasm32"))]
            Instant::now(),
        )
    }

    pub(crate) fn elapsed(&self) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        return self.0.elapsed();
        #[cfg(target_arch = "wasm32")]
        Duration::ZERO
    }
}

/// The files given by another provider, with the paths of those it gave noted.
pub(crate) struct Noting<'a> {
    pub(crate) files: &'a dyn Provider,
    pub(crate) read: RefCell<Vec<String>>,
}

impl<'a> Noting<'a> {
    pub(crate) fn new(files: &'a dyn Provider) -> Noting<'a> {
        Noting { files, read: RefCell::new(Vec::new()) }
    }
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

/// [`solve`], with the time limit of the resolution of each level measured on `clock` (8.5): the limit of a level runs from the reading when its resolution begins.
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
    solve_clocked(file, files, options, clock, watch, &|p, options| {
        Ok(BackendResult {
            solved: rnl_solver::solve_with_clock(p, options, clock),
            author: AUTHOR.to_string(),
            report_available: true,
        })
    })
}

/// Validated decisions from a caller-selected solver.
pub struct BackendResult {
    pub solved: Solved,
    pub author: String,
    /// Whether the solver supplied a report; missing reports must not be shown as zero facts.
    pub report_available: bool,
}

type Backend<'a> = dyn Fn(&LevelProblem, &SolverOptions) -> Result<BackendResult, String> + 'a;

/// Solve through a caller-selected backend, preserving the normal in-memory writeback. The backend must validate its complete response against `problem`
/// before returning. No files are written by this function, including when a later level fails.
/// The returned author is quoted by the runtime writer, never parsed as RNL code.
pub fn solve_using(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
    watch: &dyn Fn(&Progress) -> bool,
    backend: &Backend<'_>,
) -> Result<SolveRun, String> {
    // the whole file's time is measured on the wall clock, which is read only when it is given
    let wall = options.total_seconds.map(|_| WallClock::new());
    let clock: &dyn Clock = match &wall {
        Some(wall) => wall,
        None => &rnl_solver::StoppedClock,
    };
    solve_clocked(file, files, options, clock, watch, backend)
}

/// [`solve_using`], the whole file's time measured on `clock`.
fn solve_clocked(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
    watch: &dyn Fn(&Progress) -> bool,
    backend: &Backend<'_>,
) -> Result<SolveRun, String> {
    solve_marked(
        file,
        files,
        options,
        clock,
        &|mark| match mark {
            Mark::Level(p) => watch(p),
            _ => true,
        },
        &|_| None,
        backend,
    )
}

/// What a pass tells its watcher of, in order: how many levels it solves, then for each level its
/// start, the stages outside the solver, and its end. Only the answer to [`Mark::Level`] counts:
/// false stops the solve there.
pub(crate) enum Mark<'a> {
    Run {
        levels: usize,
    },
    Level(&'a Progress),
    /// The level's problem is read: the options it is solved with, its own settings applied.
    Problem(&'a SolverOptions),
    Stage(&'static str),
    LevelDone {
        level: usize,
        solved: &'a Solved,
        seconds: f64,
        junction_seconds: Option<f64>,
    },
}

/// Why a level is not solved, when it is not: [`NotSolved::reason`].
pub(crate) type Skip<'a> = dyn Fn(&LevelProblem) -> Option<String> + 'a;

/// The whole file's time ([`SolveOptions::total_seconds`]): its seconds, measured on `clock` from
/// `begun`, the start of the pass.
#[derive(Clone, Copy)]
struct Total<'a> {
    seconds: f64,
    begun: f64,
    clock: &'a dyn Clock,
}

impl Total<'_> {
    /// The limit of the level `p` solved with `solver`, its own settings applied, when it shares
    /// the time: a level of a time limit, as seconds, or of none, as infinite.
    fn own(&self, solver: &SolverOptions, p: &LevelProblem) -> Option<f64> {
        match rnl_solver::run_limit(&solver.for_problem(p).0, self.clock) {
            RunLimit::Time(seconds) => Some(seconds),
            RunLimit::Unlimited => Some(f64::INFINITY),
            RunLimit::Work(_) | RunLimit::Seed => None,
        }
    }

    /// The options of the level `p` with its share of what is left, the first of `levels` levels
    /// that share it: its own limit, when that is no more, and its seed alone when nothing is
    /// left. None when its own options stand.
    fn share(&self, solver: &SolverOptions, p: &LevelProblem, levels: usize) -> Result<Option<SolverOptions>, String> {
        let Some(own) = self.own(solver, p) else { return Ok(None) };
        let left = (self.seconds - (self.clock.elapsed() - self.begun)).max(0.);
        // whole milliseconds, which the text of the options writes as they are
        let share = (left / levels.max(1) as f64 * 1000.).floor() / 1000.;
        if share >= own {
            return Ok(None);
        }
        // as the tool's settings, the last, which the level applies over its own
        let text = if share > 0. {
            format!("limit=\"time\", seconds={}", rnl_runtime::geom::fmt_num(share))
        } else {
            "limit=\"seed\"".to_string()
        };
        solver.with_config(&text).map(Some).map_err(|e| format!("--timeall: the share of {}: {e}", p.title))
    }
}

/// What a level's junction placement is expected to take, in readings of the document: the dots
/// all at once, as most levels place them, and the record without any dot, with room to spare. A
/// level that places its dots one by one may take more, and leaves out those its limit has no time
/// left for; a reading of a level with its record can take far longer than one of the document.
const RESERVED_READINGS: f64 = 16.;

/// The most of a level's limit kept for junction placement and the write.
const RESERVED_SHARE: f64 = 0.25;

/// A level's limit of time, or of safety, less what junction placement and the write after the
/// solve are expected to take: [`RESERVED_READINGS`] readings of the document, and no more than
/// [`RESERVED_SHARE`] of the limit. The solve stops that much sooner, so that the level ends near
/// its limit, not past it by the time its dots take.
struct Reserve {
    /// The level's options with the limit less the reserve, as the tool's last settings; none when
    /// nothing is kept.
    options: Option<SolverOptions>,
    /// The limit, and what is left of it for the solve, in whole milliseconds.
    limit: f64,
    left: f64,
    /// The safety limit, when it is that limit that is kept: what junction placement has, from
    /// the start of the solve, when it stopped the solve.
    safety: Option<f64>,
}

impl Reserve {
    /// The reserve of the level `p` solved with `solver`, a reading of its document taking
    /// `reading` seconds: none for a level before 0.4, for a level with no limit of time or of
    /// safety, or on a clock that does not advance, where no limit of time is read.
    fn of(
        solver: &SolverOptions,
        p: &LevelProblem,
        clock: &dyn Clock,
        reading: f64,
    ) -> Result<Option<Reserve>, String> {
        if !clock.advances() {
            return Ok(None);
        }
        let own = solver.for_problem(p).0;
        let (limit, key, safety) = match rnl_solver::run_limit(&own, clock) {
            RunLimit::Time(seconds) => (seconds, "limit=\"time\", seconds", false),
            _ => match own.safety_seconds {
                Some(seconds) => (seconds, "safety_seconds", true),
                None => return Ok(None),
            },
        };
        if !limit.is_finite() {
            return Ok(None);
        }
        // whole milliseconds, which the text of the options writes as they are: kept rounded down
        let kept = ((RESERVED_READINGS * reading).min(RESERVED_SHARE * limit) * 1000.).floor() / 1000.;
        let left = ((limit - kept) * 1000.).round() / 1000.;
        let options = if kept > 0. && left < limit {
            let text = format!("{key}={}", rnl_runtime::geom::fmt_num(left));
            Some(solver.with_config(&text).map_err(|e| format!("the reserve of {}: {e}", p.title))?)
        } else {
            None
        };
        Ok(Some(Reserve { options, limit, left: left.min(limit), safety: safety.then_some(limit) }))
    }

    /// What the level is told of the reserve, when something was kept.
    fn note(&self) -> Option<String> {
        self.options.as_ref()?;
        Some(format!(
            "the level's {} limit of {}s solved within {}s, {}s kept for junction placement and the write",
            if self.safety.is_some() { "safety" } else { "time" },
            rnl_runtime::geom::fmt_num(self.limit),
            rnl_runtime::geom::fmt_num(self.left),
            rnl_runtime::geom::fmt_num(((self.limit - self.left) * 1000.).round() / 1000.)
        ))
    }
}

/// [`solve_using`], its watcher told of every [`Mark`], and the levels `skip` gives a reason for
/// left as they are, in [`SolveRun::not_solved`]. The whole file's time, when it is given, is
/// measured on `clock`.
pub(crate) fn solve_marked(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
    watch: &dyn Fn(Mark<'_>) -> bool,
    skip: &Skip<'_>,
    backend: &Backend<'_>,
) -> Result<SolveRun, String> {
    pass(file, files, options, options.total_seconds, clock, watch, skip, backend)
}

/// The solve of [`solve_marked`]: each level read, solved and written back.
#[allow(clippy::too_many_arguments)]
fn pass(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
    total: Option<f64>,
    clock: &dyn Clock,
    watch: &dyn Fn(Mark<'_>) -> bool,
    skip: &Skip<'_>,
    backend: &Backend<'_>,
) -> Result<SolveRun, String> {
    let entry = path_str(file);
    // the whole file's time runs from the start of the solve
    let total = total.map(|seconds| Total { seconds, begun: clock.elapsed(), clock });
    let runtime = options.runtime();
    let files = Noting::new(files);
    let mut memory = MemoryProvider::new();
    let read = |memory: &MemoryProvider| -> Result<Interpretation, String> {
        let both = Layered { layers: vec![memory, &files] };
        rnl_view::interpret_with(&entry, options.dialect.as_deref(), &options.formats, &both, &runtime)
    };
    // the time of a reading of the document, on the clock the limits are read on
    let reading = clock.elapsed();
    let mut first = Rc::new(read(&memory)?);
    let mut read_seconds = clock.elapsed() - reading;
    // the files as they are, which a caller may take ([`Readings::before`])
    let before = Rc::clone(&first);
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
            None => match skip(p) {
                Some(reason) => not_solved.push(NotSolved {
                    title: p.title.clone(),
                    kind: p.kind,
                    path: normalize(&first.doc.levels[p.level].path),
                    reason,
                    before: targets.len(),
                }),
                None => targets.push((p.level, p.title.clone())),
            },
        }
    }
    if targets.is_empty() && not_solved.is_empty() {
        if let Some(t) = &options.level {
            return Err(format!("no drawing level is titled {t}"));
        }
    }
    if let Some(version) = first
        .included_standard
        .clone()
        .filter(|_| targets.iter().any(|(level, _)| first.doc.levels[*level].file == first.doc.main().file))
    {
        let main = first.doc.main();
        let path = normalize(&main.path);
        let text = text_of(&first, &Layered { layers: vec![&memory, &files] }, &main.file, &path)?;
        let out = rnl_runtime::writeback::with_standard_package(&first.doc, &text, &version);
        if out == text {
            return Err(format!("cannot write the standard-package reference into the header of {} (16.3)", main.file));
        }
        memory.insert(&path, &out);
        first = Rc::new(read(&memory)?);
    }
    // a header that the record of a level would come before moves before any level of its file is
    // written (2.8; 16.3 and 2.2), so that every record written into the file
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
    // the levels that share the whole file's time
    let timed: Vec<bool> = targets
        .iter()
        .map(|(level, title)| {
            let p = first.problems.iter().find(|p| p.level == *level && p.title == *title);
            total.zip(p).is_some_and(|(total, p)| total.own(&options.solver, p).is_some())
        })
        .collect();
    let mut levels = Vec::new();
    let mut solved_files: Vec<SolvedFile> = Vec::new();
    // the document read again when a header moved
    let mut current = if moved.is_empty() { Some(first) } else { None };
    let mut last_read_back: Option<(BTreeMap<String, String>, rnl_view::DocumentReading)> = None;
    let count = targets.len();
    watch(Mark::Run { levels: count });
    for (k, (level, title)) in targets.into_iter().enumerate() {
        let progress = Progress { level: k, levels: count, title: title.clone() };
        // a reading back of a level before is not of the files once this level is written
        last_read_back = None;
        if !watch(Mark::Level(&progress)) {
            return Err(CANCELLED.to_string());
        }
        // the document as written so far: the lines of this level have moved if one before it in
        // its file was written back
        let r = match current.take() {
            Some(r) => r,
            None => {
                let reading = clock.elapsed();
                let r = Rc::new(read(&memory)?);
                read_seconds = clock.elapsed() - reading;
                r
            }
        };
        let p = r
            .problems
            .iter()
            .find(|p| p.level == level && p.title == title)
            .ok_or_else(|| format!("the level {title} is not read again after the levels before it were solved"))?;
        // the level's share of the whole file's time, when it is less than its own limit
        let shared = match total.filter(|_| timed[k]) {
            Some(total) => total.share(&options.solver, p, timed[k..].iter().filter(|t| **t).count())?,
            None => None,
        };
        let solver = shared.as_ref().unwrap_or(&options.solver);
        // the level's limit of time, or of safety, less what junction placement and the write take
        let reserve = Reserve::of(solver, p, clock, read_seconds)?;
        // the level is told of its own limit; the reserve is the tool's, which a note tells of
        watch(Mark::Problem(&solver.for_problem(p).0));
        let solver = reserve.as_ref().and_then(|r| r.options.as_ref()).unwrap_or(solver);
        let start = Stopwatch::start();
        let begun = clock.elapsed();
        let BackendResult { solved, author, report_available } = backend(p, solver)?;
        let time = start.elapsed();
        // a level stopped by its safety limit, which is not replayed by its work, places its dots
        // within what is left of that limit; one stopped by its time, or by its work, places them
        // all, as its replay does
        let deadline = reserve
            .as_ref()
            .and_then(|r| r.safety)
            .filter(|_| solved.report.budget.is_some_and(|b| b.reason == StopReason::Safety));
        let deadline = deadline.map(|limit| {
            crate::junction_completion_driver::Deadline::after(Duration::from_secs_f64(
                (limit - (clock.elapsed() - begun)).max(0.),
            ))
        });
        // without the solver's own text of the options, theirs as the level takes them: a level
        // before 0.4 has no layout to name
        let hash = match solved.report.canonical_options.as_deref() {
            Some(canonical) => sig2(p, canonical),
            None => sig2(p, &solver.for_problem(p).0.canonical()),
        };
        let mut written = write_record(p, &solved.solution, &author, &hash);
        // told only of a level its limit stopped, which the reserve changed
        let stopped = solved.report.budget.is_some_and(|b| matches!(b.reason, StopReason::Time | StopReason::Safety));
        if let Some(r) = reserve.as_ref().filter(|_| stopped) {
            written.notes.extend(r.note());
        }
        let mut write_errors = Vec::new();
        let host = &r.doc.levels[p.level];
        let path = normalize(&host.path);
        if solved.report.fatal_written_wire.is_some() || solved.report.fatal_annotation.is_some() {
            let why = if let Some(wire) = solved.report.fatal_written_wire {
                format!("written wire [{wire}] has unevaluable geometry")
            } else {
                format!("annotation [{}] has unevaluable geometry", solved.report.fatal_annotation.unwrap())
            };
            written.notes.push(format!("the record is not written: {why}"));
            let record = written.record.clone();
            watch(Mark::LevelDone { level: k, solved: &solved, seconds: time.as_secs_f64(), junction_seconds: None });
            levels.push(SolvedLevel {
                problem: p.clone(),
                solved,
                report_available,
                written,
                write_errors,
                record,
                hash,
                path,
                time,
                hints_for_low: options.solver.hints_for_low,
            });
            continue;
        }
        let mut junction_seconds = None;
        // the record the dots were placed in, finished with the check of its numbers as their last
        // trial read it: written as it is, not finished again
        let mut numbered: Option<(String, String)> = None;
        if !written.junctions.is_empty() {
            watch(Mark::Stage("junctions"));
            if !report_available {
                return Err(format!("junction placement unresolved in {title}: backend omitted retained-fact report"));
            }
            let both = Layered { layers: vec![&memory, &files] };
            let original_text = text_of(&r, &both, &host.file, &path)?;
            let context = crate::junction_completion_driver::Context {
                entry: &entry,
                dialect: options.dialect.as_deref(),
                formats: &options.formats,
                files: &both,
                runtime: &runtime,
                original: &r,
                problem: p,
                original_text: &original_text,
                host_path: &path,
                kept: &solved.report.kept,
                solution: &solved.solution,
                tolerance: runtime.tolerance.min(options.solver.for_problem(p).0.tolerance),
                on_interpret: None,
            };
            let completion =
                crate::junction_completion_driver::complete_what_can_be_by(&context, &written, deadline.as_ref())
                    .map_err(|why| {
                        if why.ends_with(crate::junction_completion_driver::LOST) {
                            format!("the result of {title} does not read back as it was solved: {why}")
                        } else {
                            format!("junction placement unresolved in {title}: {why}")
                        }
                    })?;
            written.notes.push(format!(
                "junction postprocessing: {} candidate trials in {:.3}s (including numbering checks, outside solver budget)",
                completion.trials,
                completion.elapsed.as_secs_f64()
            ));
            // a dot that cannot be placed costs only itself: the rest of the level is written
            for (index, why) in &completion.dropped {
                let junction = &written.junctions[*index];
                let mut unsolved = rnl_runtime::solution::Unsolved::junction_unplaced(junction);
                if why == crate::junction_completion_driver::RAN_OUT {
                    // the time, not the place: the dot was not tried
                    unsolved.reason =
                        "its dot is not written: the time of the level ran out before it was tried (13.2)".into();
                }
                written.unsolved.push(unsolved);
                written
                    .notes
                    .push(format!("junction postprocessing: the dot at {} is not written: {why}", junction.target));
            }
            junction_seconds = Some(completion.elapsed.as_secs_f64());
            numbered = Some((completion.record.clone(), completion.numbered));
            written.record = completion.record;
            // the trial that passed read the document with the record written: taken, as the check
            // below takes its reading, instead of read again
            let mut texts = memory.files.clone();
            texts.insert(path.clone(), completion.text);
            last_read_back = Some((texts, completion.reading));
        } else if p.version == "0.4" && !written.record.is_empty() {
            // no dots to place: the record still has to read back as it was solved. The check
            // compares the objects of two readings and needs no report of facts, so a backend
            // that leaves the report out is checked too
            let both = Layered { layers: vec![&memory, &files] };
            let original_text = text_of(&r, &both, &host.file, &path)?;
            let context = crate::junction_completion_driver::Context {
                entry: &entry,
                dialect: options.dialect.as_deref(),
                formats: &options.formats,
                files: &both,
                runtime: &runtime,
                original: &r,
                problem: p,
                original_text: &original_text,
                host_path: &path,
                kept: &solved.report.kept,
                solution: &solved.solution,
                tolerance: runtime.tolerance.min(options.solver.for_problem(p).0.tolerance),
                on_interpret: None,
            };
            match crate::junction_completion_driver::reads_back(&context, &written) {
                // the record finished there is the one written, and the reading is the document
                // with it, both taken instead of finished and read again
                Ok(back) => {
                    numbered = Some((written.record.clone(), back.finished));
                    let mut texts = memory.files.clone();
                    texts.insert(path.clone(), back.text);
                    last_read_back = Some((texts, back.reading));
                }
                Err(why) if why.ends_with(crate::junction_completion_driver::LOST) => {
                    return Err(format!("the result of {title} does not read back as it was solved: {why}"));
                }
                Err(_) => {}
            }
        }
        watch(Mark::Stage("write"));
        let (record, texts) = {
            let both = Layered { layers: vec![&memory, &files] };
            let record = if let Some((_, finished)) = numbered.filter(|(record, _)| *record == written.record) {
                finished
            } else {
                rnl_view::finish_record_with_formats(&r.doc, &both, &options.formats, &runtime, p, &written.record)
            };
            let text = text_of(&r, &both, &host.file, &path)?;
            let mut texts = Vec::new();
            let mut refuse = |code, message: String| {
                // The host already reports a missing selected section. Keep that one cause.
                if code == codes::WRITEBACK_SECTION_NOT_FOUND
                    && r.diagnostics().iter().any(|d| {
                        d.code == codes::HOST_SECTION_NOT_FOUND
                            && d.loc.as_ref().is_some_and(|loc| loc.file == host.file)
                    })
                {
                    return;
                }
                write_errors.push(Diag {
                    severity: Severity::Error,
                    code,
                    message,
                    loc: Some(Loc::line(&host.file, host.line)),
                    cause: None,
                    origin: String::new(),
                    in_package: String::new(),
                });
            };
            match record_line(&r.doc, p.level, &text) {
                // a level whose own text lies in a region written to be encrypted has no place for a
                // record that is not gone once the region is encrypted (2.2): nothing is written for it
                Err(_)
                    if r.doc.levels[r.doc.file_level(p.level)].section.is_some()
                        && r.doc.levels[r.doc.file_level(p.level)].section_lines.is_none() =>
                {
                    refuse(
                        codes::WRITEBACK_SECTION_NOT_FOUND,
                        format!(
                            "the record is not written: the selected library section {} was not found in {} (2.2)",
                            r.doc.levels[r.doc.file_level(p.level)].section.as_deref().unwrap(),
                            file_name(&host.file)
                        ),
                    );
                }
                Err((a, b))
                    if r.doc.excluded.iter().any(|(f, x, y)| f == &host.file && (*x, *y) == (a, b))
                        || r.doc
                            .unselected_sections
                            .iter()
                            .any(|(l, x, y)| *l == r.doc.file_level(p.level) && (*x, *y) == (a, b)) =>
                {
                    refuse(codes::WRITEBACK_REGION_EXCLUDED, format!(
                        "the record is not written: {} lies in a host region excluded from writing, lines {a} to {b} of {} (2.2)",
                        p.title,
                        file_name(&host.file)
                    ));
                }
                Err((a, b)) => refuse(
                    codes::WRITEBACK_REGION_ENCRYPTED,
                    format!(
                    "the record is not written: {} lies in a region written to be encrypted, lines {a} to {b} of {}, \
                     where a record is gone once the region is encrypted (2.2)",
                    p.title,
                    file_name(&host.file)
                ),
                ),
                Ok(_) => {
                    // the record and the header in one pass, at the positions of this reading: a
                    // header below the end of a subcircuit written moves to line 2, to stay the first
                    // record (2.8)
                    let out = replace_generated_with_header(&r.doc, p.level, &text, &record, &p.version);
                    texts.push((path.clone(), out));
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
        watch(Mark::LevelDone { level: k, solved: &solved, seconds: time.as_secs_f64(), junction_seconds });
        levels.push(SolvedLevel {
            problem: p.clone(),
            solved,
            report_available,
            written,
            write_errors,
            record,
            hash,
            path,
            time,
            hints_for_low,
        });
    }
    let packages: Vec<String> = files.files.package_files().iter().map(|p| normalize(p)).collect();
    let read: Vec<String> = files.read.take().into_iter().filter(|p| !packages.contains(p)).collect();
    let readings = Readings { before: Some(before), written: last_read_back };
    Ok(SolveRun { entry, levels, not_solved, files: solved_files, read, errors, readings })
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
    if let Err(e) = overwrites_no_input(run, target, &to) {
        out.error = Some(e);
        return out;
    }
    let make_parent = |path: &Path| -> Result<(), String> {
        match path.parent().filter(|p| !p.as_os_str().is_empty()) {
            Some(parent) => {
                std::fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))
            }
            None => Ok(()),
        }
    };
    for (bytes, path) in encoded.iter().zip(to) {
        let done = make_parent(&path).and_then(|()| write_whole(&path, bytes));
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

/// The file a path names, to tell whether two paths name one file: its canonical path, None when
/// it does not exist (it is then no input).
fn existing(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

/// Refuses, before anything is written, a destination that is another input of the run: a file
/// written or copied over a file that the run reads and has not yet written or copied would lose
/// that input, such as `-o lib` for an entry beside a `lib/` that holds a file of its name. A file
/// written over itself (`-o` its own directory) is no other input.
fn overwrites_no_input(run: &SolveRun, target: &Target, written: &[PathBuf]) -> Result<(), String> {
    let mut inputs: Vec<(PathBuf, &str)> = Vec::new();
    for path in run.files.iter().map(|f| f.path.as_str()).chain(run.read.iter().map(String::as_str)) {
        if let Some(key) = existing(Path::new(path)) {
            inputs.push((key, path));
        }
    }
    let mut pairs: Vec<(&str, PathBuf)> =
        run.files.iter().map(|f| f.path.as_str()).zip(written.iter().cloned()).collect();
    if let Target::Dir(dir) = target {
        let base = dirname(&normalize(&run.entry));
        for path in run.read.iter().filter(|p| !run.files.iter().any(|f| f.path == **p)) {
            if let Some(rel) = name_in(path, &base) {
                pairs.push((path.as_str(), dir.join(rel)));
            }
        }
    }
    for (source, destination) in pairs {
        let Some(key) = existing(&destination) else { continue };
        let own = existing(Path::new(source));
        if let Some((_, input)) = inputs.iter().find(|(k, _)| *k == key && own.as_ref() != Some(k)) {
            return Err(format!(
                "{} would be written over {input}, another file the document reads; nothing was written: write the output to another directory",
                destination.display()
            ));
        }
    }
    Ok(())
}

/// Writes `bytes` to `path` whole or not at all: into a new file beside it, then put in its place,
/// so that a write that fails part way, such as on a full disk, leaves the file as it was.
fn write_whole(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let temporary = path.with_file_name(format!(".{name}.{}.rnl-write", std::process::id()));
    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        // the file it replaces keeps its permissions
        if let Ok(meta) = std::fs::metadata(path) {
            std::fs::set_permissions(&temporary, meta.permissions())?;
        }
        std::fs::rename(&temporary, path)
    })();
    result.map_err(|e| {
        let _ = std::fs::remove_file(&temporary);
        format!("cannot write {}: {e}", path.display())
    })
}

/// A point of a fact, by its object's name when it has one, such as `%t` for a text named so, else
/// as diagnostics show its object: `#M1`, `#M1:d`, `{#M1 #M2}` for a centre.
fn point(p: &LevelProblem, r: &PointRef) -> String {
    rnl_runtime::problem::point_name(p, r)
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
    let what = rnl_runtime::problem::fact_statement(p, k);
    // a handle, such as %c1, or a label of its own; the label of a coordinate, an offset or an
    // orientation says what the fact says, so the fact is described once
    let own = matches!(
        f.kind,
        FactKind::Place { .. }
            | FactKind::PointCoordinate { .. }
            | FactKind::Offset { .. }
            | FactKind::Orient { .. }
            | FactKind::PinOrient { .. }
    );
    let named = if own || f.label == f.kind.name() || f.label == what { what } else { format!("{} = {what}", f.label) };
    format!("[{k}] {named}")
}

/// A statement as the hints of 8.5 name it (8.5; 7.6): where it is written, such
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

/// What the hint of a fact discarded says of it (8.5): every minimal group of facts
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

/// How many errors the solver reported over the levels of a run: the facts that contradict
/// themselves and fail (8.5; 7.6). `rnl solve` exits with 1 when there is one, or
/// an error of the netlist ([`SolveRun::errors`]).
pub fn errors(run: &SolveRun) -> usize {
    run.levels
        .iter()
        .map(|l| {
            // a junction given to the tool that a level does not take is an error of the writing (14.10)
            let written = l.written.warnings.iter().filter(|d| d.severity == Severity::Error).count();
            l.solved.report.errors.len() + l.write_errors.len() + written
        })
        .sum()
}

/// What the command prints of the levels of a run, in the order of the document's levels: each
/// level solved as [`report`] gives it, each level not solved in one line.
pub fn lines(run: &SolveRun, all: bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
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
/// fails among them, otherwise the discarded ones, each with its hint (8.5): every
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
    if !level.report_available {
        out[0] = format!("level {} ({} in {}): fact report not supplied", p.title, p.kind, file_name(&level.path));
    }
    // the options the level is signed with replay it, but after a stop by the safety limit, which
    // the clock made: its work does not replay it ([`rnl_solver::BudgetReport::reproducible`])
    if let Some(canonical) = &r.canonical_options {
        if r.budget.is_some_and(|b| !b.reproducible()) {
            out.push(format!(
                "  options: {canonical} (not replayed by them: the safety limit stopped the solve by the clock)"
            ));
        } else {
            out.push(format!("  replay options: {canonical}"));
        }
    }
    // Discarding a fact of priority low is expected, and its hints may be left out (8.5; 7.6): they are printed with `all` alone, the hint of the fact and those of the
    // statements that only such facts lose, when the solver sought them.
    let hinted = |f: usize| (all && level.hints_for_low) || p.facts.get(f).is_none_or(|f| f.priority != Priority::Low);
    // a fact discarded and its hint: every minimal group of facts kept before it that it conflicts
    // with, each fact of them described once below it, and whether the list is complete
    let discarded = |out: &mut Vec<String>, k: usize, indent: &str| {
        let Some(d) = r.discarded.iter().find(|d| d.fact == k) else { return };
        let why = if d.undecided {
            if r.budget.is_some() {
                "not decided within the shared solve budget (8.5)".to_string()
            } else {
                "not decided within the time limit of the solver (8.5)".to_string()
            }
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
    if all && level.report_available {
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
    // the hints by statement (8.5): each statement whose facts are in a group, with
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
    out.extend(level.write_errors.iter().map(|error| format!("  {}", rnl_runtime::diag::Sink::new().render(error))));
    out.extend(r.warnings.iter().map(|w| format!("  warning: {w}")));
    out.extend(level.written.warnings.iter().map(|w| format!("  {}", rnl_runtime::diag::Sink::new().render(w))));
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
    if level.report_available {
        out.push(format!(
            "  work: {}, {}, {}, {}",
            count(w.resolve_nodes as usize, "resolve node"),
            count(w.pivots as usize, "pivot"),
            count(w.candidates as usize, "candidate"),
            count(w.astar_expansions as usize, "A* expansion")
        ));
    }
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
        // is of $PAIR, and both definitions are named, as for a fact (8.5, 7.6)
        let text = "* nested\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\nR1 a b 1k\nR2 b c 1k\n* @RNL {\n\
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

    /// A record of the solver that uses a number is finished on the document with it written, and
    /// the reading of the document with it written with its `numbering` takes the objects of the
    /// authored records that the finish counted (5.9), instead of counting them again by reading the
    /// document without its records of the solver. So does the trial of a dot, whose text differs
    /// from the one counted in that record alone, and the reading of the trial.
    #[test]
    fn the_reading_of_a_record_written_takes_the_count_of_its_finish() {
        // a dot on the net b, which the solve places and checks by reading its trial; the usage of
        // $PARALLEL_HORIZONTAL has no name, so the record places it by its number
        const NET: &str = "* counted\n\
            * @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }\n\
            R1 a b 1k\nR2 b 0 1k\nR3 b 0 1k\nC1 a 0 1p\nR4 a 0 1k\nC2 a 0 1p\n\
            * @RNL { #R1 @PLACE(0, 0); {#R4 #C2} @AS $PARALLEL_HORIZONTAL; {#R1:2 #R2:1 #R3:1} @ROUTE-OBJECT }\n\
            .end\n";
        let files = MemoryProvider::new().with("/w/t.sp", NET);
        let options =
            SolveOptions { solver: rnl_solver::SolverOptions::without_time_limit(), ..SolveOptions::default() };
        let before = rnl_runtime::countings();
        let run = solve(Path::new("/w/t.sp"), &files, &options).unwrap();
        let counted = rnl_runtime::countings() - before;
        let text = &run.files[0].text;
        assert!(text.contains("$JUNCTION"), "the dot is placed");
        assert!(text.contains("numbering="), "the record uses a number");
        // the finish of the record counts; the reading of it written, the finish of the trial and
        // the reading of the trial take that count. Before, the reading of the record written and
        // the reading of the trial each counted again (3)
        assert_eq!(counted, 1);
    }
}
