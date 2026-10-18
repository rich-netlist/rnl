//! A solve shown before anything is written, as the desktop viewer's Solve menu runs it (decision
//! 0010): [`solve_watched`] run in memory, the texts it would write held over the files and drawn
//! as a Document, with a report of what became of each fact, every line of it located in those
//! texts; and a copy of the files solved, saved where the user says, never over a file that was
//! read. Nothing here writes a file but [`save_copy`].

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use rnl_runtime::diag::Loc;
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::source::{dirname, normalize, relative_to, Layered, MemoryProvider, Provider};
pub use rnl_solver::Clock;
use rnl_solver::Statement;
use rnl_view::{BuildOptions, Document};
use serde::Serialize;

use crate::solve::{
    conflicts, describe_fact, fact_what, lines, solve_watched, statement_name, write, Progress, SolveOptions, SolveRun,
    SolvedFile, Target, Writing,
};

/// A solve held in memory: what a copy of it saves, the netlist drawn as solved, and the report.
/// It can go to another thread, as a frontend that solves on one of its own needs.
#[derive(Debug)]
pub struct Preview {
    pub copy: Copy,
    /// The netlist as the files solved read: the texts the solve would write, held over the files,
    /// drawn with their records of the solver (2.8).
    pub document: Document,
    pub report: Report,
}

/// What became of the facts, level by level, each line located in the texts of the Document.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub config_source: Option<String>,
    pub invocation: crate::solve::Invocation,
    /// The lines of the files raised to 0.3 or kept at 0.2 (16.1), and of the levels not solved.
    pub notes: Vec<String>,
    pub levels: Vec<LevelReport>,
    /// How many errors the netlist has as a solver reads it, and the solver reported.
    pub errors: usize,
    /// The files the solve writes into, as the Document names them.
    pub files: Vec<String>,
    /// Whether a copy is saved as one file ([`one_file`]), or as a folder.
    pub one_file: bool,
    /// The whole report as `rnl solve --report` prints it.
    pub text: Vec<String>,
}

/// One level solved.
#[derive(Clone, Debug, Serialize)]
pub struct LevelReport {
    pub title: String,
    /// "file" or "subckt".
    pub kind: &'static str,
    /// Such as `5 kept, 1 discarded of 6 facts`.
    pub summary: String,
    /// Every fact, in the order of 8.5.
    pub facts: Vec<FactLine>,
    /// The hints by statement (the draft 0.3, 8.5): each statement whose facts took part in
    /// discarding, with the facts discarded because of it.
    pub statements: Vec<Line>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    /// What the record cannot hold.
    pub unsolved: Vec<String>,
    pub routing_fallbacks: Vec<rnl_solver::RoutingFallback>,
    pub scoring: rnl_solver::ScoreReport,
    pub notes: Vec<String>,
}

/// A fact: its rank, what became of it, what it says and where it is written, such as
/// `[3] ALIGN y {#M1 #M2} (normal, amp.sp:12)`, and for one discarded what it conflicts with.
#[derive(Clone, Debug, Serialize)]
pub struct FactLine {
    pub rank: usize,
    /// "kept", "discarded", "undecided" (discarded when its check did not finish within the time
    /// limit, 8.5) or "failed" (it contradicts itself, 8.5, 7.6).
    pub status: &'static str,
    pub text: String,
    /// For a fact discarded, its hint: the groups of facts it conflicts with, by rank.
    pub why: Option<String>,
    pub at: Option<At>,
}

/// A line of the report and where it points.
#[derive(Clone, Debug, Serialize)]
pub struct Line {
    pub text: String,
    pub at: Option<At>,
}

/// A place in the texts of the Document: the index of the file in [`Document::files`], and the
/// line and column, from 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct At {
    pub file_index: usize,
    pub line: u32,
    pub col: u32,
}

/// Solve `entry` in memory and draw it as solved, as [`solve_watched`] says, nothing written: the
/// Document is built from the texts of the solve held over `files`, with the records of the
/// solver, and the report locates each fact in those texts, read again as a solver reads them.
pub fn preview(
    entry: &str,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
    watch: &dyn Fn(&Progress) -> bool,
) -> Result<Preview, String> {
    let run = solve_watched(Path::new(entry), files, options, clock, watch)?;
    let mut written = MemoryProvider::new();
    for f in &run.files {
        written.insert(&f.path, &f.text);
    }
    let both = Layered { layers: vec![&written, files] };
    let build = BuildOptions { dialect: options.dialect.clone(), ..BuildOptions::default() };
    let document = rnl_view::build(entry, &both, &build)?;
    // the texts as a solver reads them, without the records of the solver, so that the facts are
    // those solved, at the lines where the Document shows them
    let again = rnl_view::interpret(entry, options.dialect.as_deref(), &both, &options.runtime())?;
    let report = report(&run, &document, &again.problems);
    Ok(Preview { copy: Copy::of(&run), document, report })
}

/// The report of a run, located in the texts of `document`; `problems` are those of these texts
/// read as a solver reads them, which give each fact its line there.
fn report(run: &SolveRun, document: &Document, problems: &[LevelProblem]) -> Report {
    let at = |loc: &Loc| -> Option<At> {
        let file_index = document.files.iter().position(|f| f.path == loc.file)?;
        (loc.line > 0).then_some(At { file_index, line: loc.line, col: loc.col })
    };
    let mut levels = Vec::new();
    for level in &run.levels {
        let p = &level.problem;
        let r = &level.solved.report;
        // the same level of the texts written, whose facts are those solved, in the same order
        let q = problems.iter().find(|q| q.level == p.level && q.title == p.title && q.facts.len() == p.facts.len());
        let fact_at = |k: usize| q.unwrap_or(p).facts.get(k).and_then(|f| f.source.as_ref()).and_then(at);
        let facts = (0..p.facts.len())
            .map(|k| {
                let d = r.discarded.iter().find(|d| d.fact == k);
                let status = if r.kept.contains(&k) {
                    "kept"
                } else if r.failed.contains(&k) {
                    "failed"
                } else if d.is_some_and(|d| d.undecided) {
                    "undecided"
                } else {
                    "discarded"
                };
                let why = d.map(|d| {
                    if d.undecided {
                        "not decided within the time limit of the solver (8.5)".to_string()
                    } else {
                        conflicts(d)
                    }
                });
                FactLine { rank: k, status, text: describe_fact(p, k), why, at: fact_at(k) }
            })
            .collect();
        let statements = r
            .statements
            .iter()
            .filter(|(_, hint)| !hint.discarded.is_empty())
            .map(|(s, hint): (&Statement, _)| {
                let facts: Vec<String> = hint.facts.iter().map(|k| fact_what(p, *k)).collect();
                let lost: Vec<String> = hint.discarded.iter().map(|k| format!("[{k}]")).collect();
                let text = format!(
                    "{} {} took part in discarding {}",
                    statement_name(p, s),
                    facts.join(" and "),
                    lost.join(", ")
                );
                Line { text, at: hint.facts.first().and_then(|&k| fact_at(k)) }
            })
            .collect();
        let failed = if r.failed.is_empty() { String::new() } else { format!(", {} failed", r.failed.len()) };
        let noun = if p.facts.len() == 1 { "fact" } else { "facts" };
        levels.push(LevelReport {
            title: p.title.clone(),
            kind: p.kind,
            summary: format!(
                "{} kept, {} discarded{failed} of {} {noun}",
                r.kept.len(),
                r.discarded.len(),
                p.facts.len()
            ),
            facts,
            statements,
            errors: r.errors.clone(),
            warnings: r.warnings.clone(),
            unsolved: level.written.unsolved.iter().map(|u| u.to_string()).collect(),
            routing_fallbacks: r.routing_fallbacks.clone(),
            scoring: r.scoring.clone(),
            notes: r.notes.iter().chain(&level.written.notes).cloned().collect(),
        });
    }
    let mut notes: Vec<String> = run.raised.iter().flat_map(crate::solve::raised).collect();
    notes.extend(run.not_solved.iter().map(crate::solve::not_solved));
    let named = |path: &str| {
        let path = normalize(path);
        document
            .files
            .iter()
            .find(|f| normalize(&rnl_runtime::source::join(&dirname(&normalize(&run.entry)), &f.path)) == path)
            .map_or(path.clone(), |f| f.name.clone())
    };
    Report {
        config_source: None,
        invocation: run.invocation.clone(),
        notes,
        levels,
        errors: run.errors + crate::solve::errors(run),
        files: run.files.iter().map(|f| named(&f.path)).collect(),
        one_file: one_file(&Copy::of(run)),
        text: lines(run, true),
    }
}

/// The wall clock, until `stop` is set: then each reading is far past the one before it, so that
/// the time limit of the resolution, which runs from the reading when a level begins (8.5), is
/// reached at once, in the level being solved and in any after it. A solve cancelled so stops
/// checking the facts of the level it is in, and its watcher stops it before the next level. With
/// no time limit, only the watcher stops it.
pub struct StoppableClock {
    start: Instant,
    stop: Arc<AtomicBool>,
    /// The readings since `stop` was set.
    after: AtomicU64,
}

impl StoppableClock {
    pub fn new(stop: Arc<AtomicBool>) -> StoppableClock {
        StoppableClock { start: Instant::now(), stop, after: AtomicU64::new(0) }
    }
}

/// How far each reading of a stopped clock is past the one before, in seconds: more than any time
/// limit a user sets.
const STOPPED_STEP: f64 = 1e9;

impl Clock for StoppableClock {
    fn elapsed(&self) -> f64 {
        if self.stop.load(Ordering::Relaxed) {
            let k = self.after.fetch_add(1, Ordering::Relaxed) + 1;
            // past every reading of the wall clock, and never less than an earlier reading
            1e12 + k as f64 * STOPPED_STEP
        } else {
            self.start.elapsed().as_secs_f64()
        }
    }
}

/// What a copy of a solve saves: the file solved, the texts the solve wrote, and the files it read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Copy {
    /// The file solved, `/`-separated ([`SolveRun::entry`]).
    pub entry: String,
    /// The files written, with their texts ([`SolveRun::files`]).
    pub files: Vec<SolvedFile>,
    /// The files read ([`SolveRun::read`]).
    pub read: Vec<String>,
}

impl Copy {
    pub fn of(run: &SolveRun) -> Copy {
        Copy { entry: run.entry.clone(), files: run.files.clone(), read: run.read.clone() }
    }

    /// A run of these files and no level, which is what writing them takes.
    fn run(&self) -> SolveRun {
        SolveRun {
            invocation: crate::solve::Invocation::default(),
            entry: self.entry.clone(),
            levels: Vec::new(),
            not_solved: Vec::new(),
            files: self.files.clone(),
            read: self.read.clone(),
            errors: 0,
            raised: Vec::new(),
        }
    }
}

/// Whether a copy is one file: the solve writes one file and reads no other beside it, so that
/// the file reads the same wherever it is saved. Otherwise the copy is a folder that holds the
/// files written and the others read, in their places relative to the file solved.
pub fn one_file(copy: &Copy) -> bool {
    match copy.files.as_slice() {
        [f] => copy.read.iter().all(|r| normalize(r) == normalize(&f.path)),
        _ => false,
    }
}

/// What a copy is refused with when it would replace a file that was read.
pub const OVER_A_FILE_READ: &str = "a copy is never saved over a file that was read: choose another file or folder";

/// Save a copy of a solve: as the file `to` when it is [`one_file`], otherwise into the folder `to`,
/// as `rnl solve -o DIR` writes it, the files written and the others read in their places
/// relative to the file solved. Refused, with nothing written, when a file of the copy would be
/// one that was read, the file solved or any other, and when there is nothing to save.
pub fn save_copy(copy: &Copy, to: &Path) -> Result<Writing, String> {
    let run = &copy.run();
    if run.files.is_empty() {
        return Err("the solve wrote no file: there is nothing to save".to_string());
    }
    let read: Vec<PathBuf> =
        run.read.iter().chain(run.files.iter().map(|f| &f.path)).filter_map(|p| real(Path::new(p))).collect();
    let over = |path: &Path| real(path).is_some_and(|p| read.contains(&p));
    if one_file(copy) {
        if over(to) {
            return Err(OVER_A_FILE_READ.to_string());
        }
        let f = &run.files[0];
        let original = std::fs::read(&f.path).ok();
        if let Some(parent) = to.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))?;
        }
        std::fs::write(to, crate::solve::encode_as(original.as_deref(), &f.text))
            .map_err(|e| format!("cannot write {}: {e}", to.display()))?;
        return Ok(Writing { written: vec![to.to_path_buf()], ..Writing::default() });
    }
    // every file of the folder, written and copied, before any is written
    let base = dirname(&normalize(&run.entry));
    let target = Target::Dir(to.to_path_buf());
    let mut places = crate::solve::destinations(run, &target)?;
    places.extend(run.read.iter().filter_map(|p| relative_to(&normalize(p), &base).map(|rel| to.join(rel))));
    if places.iter().any(|p| over(p)) {
        return Err(OVER_A_FILE_READ.to_string());
    }
    let writing = write(run, &target);
    match &writing.error {
        Some(e) => Err(e.clone()),
        None => Ok(writing),
    }
}

/// A path as it is on disk, for telling two names of one file apart: the file itself when it
/// exists, else its folder's, with its name; None when neither is there.
fn real(path: &Path) -> Option<PathBuf> {
    if let Ok(p) = std::fs::canonicalize(path) {
        return Some(p);
    }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    Some(std::fs::canonicalize(parent).ok()?.join(path.file_name()?))
}
