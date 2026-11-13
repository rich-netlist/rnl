//! The solver of Rich Netlist (RNL) descriptions (standard 8.5, 11, 13.7, 14, chapter 17).
//!
//! [`solve`] takes the problem of one drawing level, the [`LevelProblem`] that the runtime builds
//! when it reads a document with `rnl_runtime::Options::solver()`, and gives a [`Solution`]: the
//! positions and orientations of the objects and the wires of the ROUTEs. The runtime writes it
//! back as the level's record of the solver:
//!
//! ```text
//! let solved = rnl_solver::solve(problem, &options);
//! let canonical = solved.report.canonical_options.clone().unwrap_or_else(|| options.canonical());
//! let hash = rnl_runtime::problem::sig2(problem, &canonical);
//! let written = rnl_runtime::solution::write_record(problem, &solved.solution, rnl_solver::AUTHOR, &hash);
//! ```
//!
//! [`SolveReport`] says what became of each fact: 8.5 first checks each fact on its own, and a fact
//! that has no solution on its own contradicts itself and fails, with an error (7.6); it takes the
//! others in the order of [`LevelProblem::facts`] and keeps each that fits with those kept before
//! it; a fact that does not is discarded, with every minimal group of facts kept before it that it
//! conflicts with, and each statement whose facts are in such a group is told which facts were
//! discarded because of it (the draft 0.3, 8.5).
//!
//! The work is in stages (the development plan of solver v1):
//!
//! - resolution of 8.5 with exact rationals and an incremental simplex (`rational`, `simplex`,
//!   `model`, `orient`, `resolve`);
//! - structure-first search over the discrete decisions, configured by the layered objectives
//!   (`objectives`, `structure`, `search`);
//! - coordinates by a lexicographic LP, routing of the ROUTEs, layered scoring (`coords`, `route`,
//!   `score`, `driver`).
//!
//! **Status: stage S5.** [`resolve()`] resolves the facts of a level as 8.5 says, exactly, and gives
//! a witness, a placement that satisfies every fact kept. [`solve`] then analyses the structure of
//! the level, searches candidates structure first (rows by current depth, orders, orientations),
//! places each by a lexicographic LP rounded to the grid, keeping every fact kept, routes the
//! ROUTEs of the best of them by a rectilinear A* around the symbols (13.7), with branch points
//! and junctions (13.2), and takes the best by the layered objectives in force on their wires.
//! `rnl solve` (in `rnl-cli`) solves every level of a document and writes the records back.
//!
//! The crate does no file I/O, reads no clock and takes no randomness, so it builds for
//! `wasm32-unknown-unknown`. The resolution of 8.5 may be limited in time
//! ([`SolverOptions::time_limit`], the draft 0.3, 8.5), on a [`Clock`] that the caller gives
//! ([`solve_with_clock`]); [`solve`] gives one that never advances. Without a limit, and whenever
//! no check reaches it, the same input and options give the same result on every platform; the
//! search, the coordinates and the routing are counted in work, never timed, so they always do.

mod annotation;
mod beside;
mod bound;
mod budget;
mod config;
mod coords;
mod driver;
pub mod experimental;
mod group_tree;
mod holds;
mod lines;
mod model;
mod objectives;
mod options;
mod orient;
pub mod profile;
mod rational;
mod resolve;
mod route;
mod score;
mod search;
mod search_choices;
mod simplex;
mod skeleton;
mod structure;
mod tree;
mod tree_hang;

use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::problem::FactKind;

pub use budget::{BudgetProgress, BudgetReport, ProgressKind, RunLimit, StopReason};
pub use config::{is_run_setting, Layout, Settings};
pub use holds::{fact_holds, holds_as_drawn, problem_fact_holds_as_drawn};
pub use lines::engine::{LayoutSkeleton, LineKind, SkeletonItem, SkeletonLine, SkeletonTable, SkeletonTree};
pub use options::{Budget, Layer, Objectives, SolverOptions, DEFAULT_LAYERS, DEFAULT_TIME_LIMIT, OBJECTIVES};
pub use resolve::Resolved;
pub use rnl_runtime::problem::LevelProblem;
pub use rnl_runtime::solution::Solution;

/// The `author` the solver writes in the head of its record (2.8, chapter 17).
pub const AUTHOR: &str = "rnl-solver 0.1";

/// What [`solve`] gives: the solution, and the report of how it was reached.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Solved {
    pub solution: Solution,
    pub report: SolveReport,
}

/// What became of the facts, and how much work it took.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SolveReport {
    /// An authored wire could not be represented reliably. This level must not be written.
    pub fatal_written_wire: Option<usize>,
    /// Index in LevelProblem.annotations of geometry that cannot safely be evaluated.
    pub fatal_annotation: Option<usize>,
    /// The final line-layout tables and line identities, when that layout was requested.
    pub skeleton: Option<LayoutSkeleton>,
    /// Whole-solve budget for 0.4; absent for released input versions.
    pub budget: Option<BudgetReport>,
    /// Canonical options for persisting the result, with a time limit normalized to admitted work.
    pub canonical_options: Option<String>,
    /// The facts kept, by their index in [`LevelProblem::facts`], in that order (8.5).
    pub kept: Vec<usize>,
    /// The facts discarded, in the order of [`LevelProblem::facts`] (8.5), each with every minimal
    /// conflict group it has (the draft 0.3, 8.5); a fact of priority low with its first only,
    /// unless [`SolverOptions::hints_for_low`] asks for its hints.
    pub discarded: Vec<Discarded>,
    /// The facts that contradict themselves, by their index, in order (the draft 0.3, 8.5): each
    /// has no solution on its own, whatever orientations the objects it reaches take and whatever
    /// way it holds in, and fails (7.6). They take no part in the resolution: neither kept nor
    /// discarded, in no group and in no hint by statement; the level is solved as if they were
    /// not written, and the report has an error for each.
    pub failed: Vec<usize>,
    /// The hints of 8.5 by statement (the draft 0.3, 8.5): each statement whose facts are in a
    /// minimal conflict group of a fact discarded, with those facts and the facts discarded because
    /// of them, so that the author sees which statements lost their effect through it; a fact of
    /// priority low only when [`SolverOptions::hints_for_low`] asks for its hints. In the order of
    /// [`Statement`]: by where the statement is written, then by usage.
    pub statements: BTreeMap<Statement, StatementHint>,
    /// The objectives that are on but have nothing to work with, such as the current direction
    /// without a supply or ground symbol, or that this version does not compute: one sentence
    /// each.
    pub inert_objectives: Vec<String>,
    /// The ROUTEs whose nets the router could not join into one tree (13.7), in the order of the
    /// ROUTEs. The wires it made for them are in the solution all the same; a viewer draws flight
    /// lines for what is left. Not an error.
    pub unrouted: Vec<Unrouted>,
    pub work: Work,
    /// The errors of the solve (7.6), one sentence each: a fact that contradicts itself and fails
    /// (the draft 0.3, 8.5), one for each fact of `failed`, in order, naming it by its index, its
    /// label, where its statement is written and the definitions it was expanded from, such as
    /// `the fact [3] ALIGN (amp.sp:12) contradicts itself and fails (8.5, 7.6)`.
    pub errors: Vec<String>,
    /// The warnings of the solve (7.6), one sentence each: a fact discarded because the check for
    /// it did not finish within the time limit (the draft 0.3, 8.5), one for each such fact, in
    /// the order of the facts.
    pub warnings: Vec<String>,
    /// Anything else worth saying, such as a budget that ran out.
    pub notes: Vec<String>,
}

/// A fact the resolution of 8.5 discarded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discarded {
    /// Its index in [`LevelProblem::facts`].
    pub fact: usize,
    /// Its minimal conflict groups, which the hint of 8.5 names (the draft 0.3, 8.5): each a set
    /// of facts kept before it that has no solution together with it and from which no fact can
    /// be left out, by index. Each group is sorted by rank, and the groups are sorted too. Every
    /// group here is minimal; there are all of them when `complete` says so. Never empty but when
    /// the fact is `undecided`, or the time limit ran out before a first group was made minimal: a
    /// fact that cannot hold even alone is not discarded but fails ([`SolveReport::failed`]).
    pub groups: Vec<Vec<usize>>,
    /// Whether `groups` holds every minimal conflict group of the fact. False when the time limit
    /// ([`SolverOptions::time_limit`]) ran out before they were all found, so that the hint says
    /// that its list is not complete (the draft 0.3, 8.5); for a fact of priority low whose other
    /// groups were not sought, as its hints may be left out (8.5, 7.6), unless
    /// [`SolverOptions::hints_for_low`] asks for them, which a note of the report says; and for a
    /// fact `undecided`.
    pub complete: bool,
    /// The check for it did not finish within the time limit ([`SolverOptions::time_limit`], the
    /// draft 0.3, 8.5): it is discarded without a proof that it conflicts, `groups` is empty, and
    /// the report has a warning for it (7.6), which names it by its index, its label and where its
    /// statement is written.
    pub undecided: bool,
}

impl Discarded {
    /// The first of its groups by rank: the facts that rank first among those it conflicts with.
    /// None when it has none.
    pub fn first_group(&self) -> Option<&[usize]> {
        self.groups.first().map(Vec::as_slice)
    }

    /// Every fact in one of its groups, sorted, each once.
    pub fn in_groups(&self) -> Vec<usize> {
        let set: BTreeSet<usize> = self.groups.iter().flatten().copied().collect();
        set.into_iter().collect()
    }
}

/// A statement, as the hints of 8.5 name a fact: by the statement that wrote it (the draft 0.3,
/// 8.5; 7.6). The facts of one statement, such as the x and the y of a PLACE, share it; the same
/// statement of a definition expanded by two usages is two statements, told apart by `chain`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Statement {
    /// The file the statement is written in, as [`ProblemFact::source`] gives it: for a fact
    /// expanded from a usage, the file of the statement in the body of the definition. Empty for
    /// the coordinates of a point, which have no statement of their own.
    ///
    /// [`ProblemFact::source`]: rnl_runtime::problem::ProblemFact::source
    pub file: String,
    /// Its line and column there; 0 for the coordinates of a point.
    pub line: u32,
    pub col: u32,
    /// The usages the facts were expanded from, outermost first, by index in
    /// [`LevelProblem::usages`]: the first is the AS or ATTACH statement written in the level.
    /// Empty for a statement written in the level itself.
    pub chain: Vec<usize>,
    /// For the coordinates of a point, the point, by index in [`LevelProblem::objects`]; none for a
    /// statement.
    pub point: Option<usize>,
}

impl Statement {
    /// The statement that wrote a fact.
    pub fn of(problem: &LevelProblem, fact: usize) -> Statement {
        let Some(f) = problem.facts.get(fact) else { return Statement::default() };
        let point = match &f.kind {
            FactKind::PointCoordinate { point, .. } if f.source.is_none() => Some(*point),
            _ => None,
        };
        let (file, line, col) = f.source.as_ref().map_or((String::new(), 0, 0), |l| (l.file.clone(), l.line, l.col));
        Statement { file, line, col, chain: f.chain.clone(), point }
    }
}

/// The hint of 8.5 for one statement (the draft 0.3, 8.5): which of its facts are in minimal
/// conflict groups, and which facts were discarded because of them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatementHint {
    /// Its facts that are in a group of some fact discarded, by index, sorted.
    pub facts: Vec<usize>,
    /// The facts discarded that have one of `facts` in a group, by index, sorted: the facts that
    /// lost their effect through this statement.
    pub discarded: Vec<usize>,
}

/// A ROUTE whose net the router left in parts (13.7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unrouted {
    /// The ROUTE, by its index in [`LevelProblem::routes`]: the first ROUTE of its net, whose style
    /// the wires routed for it take.
    pub route: usize,
    pub net: String,
    /// The parts the net is left in, two or more: its pins, and the wires written on it, that no
    /// wire joins to the rest.
    pub parts: usize,
    /// The budget of the router for the net ran out ([`Budget::astar_expansions`]); otherwise no
    /// path around the symbols and the other wires joins a part to the rest.
    pub budget: bool,
}

/// The work done. The candidates and the expansions are what the [`Budget`] counts; the nodes and
/// the pivots are counted whatever bounds them: the time limit in the resolution of 8.5, the
/// caps of the implementation in the solver's own uses of it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    /// The nodes of the searches over the literals: those of the resolution of 8.5, the checks of
    /// the facts on their own among them, and those of the solver's own relations; and the nodes
    /// of the search for the seeds of the enumeration of the conflict groups.
    pub resolve_nodes: u64,
    /// The pivots of every simplex.
    pub pivots: u64,
    /// The candidates the search made.
    pub candidates: u64,
    /// The cells the router expanded, over every candidate routed.
    pub astar_expansions: u64,
}

/// The clock that the time limit of the resolution is measured on (the draft 0.3, 8.5): the
/// caller gives it, as the crate reads no clock of its own and builds for
/// `wasm32-unknown-unknown`. `rnl solve` gives one on `std::time::Instant`; a test can give one
/// that advances by a step each time it is read, so that the limit is reached at the same point
/// on every run.
pub trait Clock {
    /// Whether wall time can advance. A stopped clock requests the initial heuristic pass.
    fn advances(&self) -> bool {
        true
    }
    /// Optional progress. Observation can affect a time cutoff, but cannot consume work.
    fn progress(&self, _progress: BudgetProgress) {}
    /// Whether [`Clock::progress`] should be given the elapsed time and the share of the limit
    /// used, which reads the clock at each observation. False by default, so that a clock that
    /// does not watch progress is read only where the limits are checked.
    fn observes_progress(&self) -> bool {
        false
    }
    /// The seconds since some start of the clock's own, never less than at an earlier reading.
    /// Only differences count: the limit of a level runs from the reading when its resolution
    /// begins.
    fn elapsed(&self) -> f64;

    /// Optional profiling, kept outside all solver options and results. Profile timing uses
    /// this observer's `Clock`; the deadline continues to read only `self.elapsed()` at the
    /// original check sites. Wall-clock observation overhead can still move a deadline boundary.
    fn profiler(&self) -> Option<profile::ProfileHandle> {
        None
    }
}

/// A clock that never advances: no time limit is ever reached, and the resolution is that of 8.5
/// whatever [`SolverOptions::time_limit`] says. What [`solve`] and [`resolve()`] measure on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StoppedClock;

impl Clock for StoppedClock {
    fn advances(&self) -> bool {
        false
    }
    fn elapsed(&self) -> f64 {
        0.0
    }
}

/// Solves one drawing level: resolves its facts (8.5), places and orients its objects, and
/// routes its ROUTEs (13.7). The time limit is never reached: [`solve_with_clock`] with a
/// [`StoppedClock`].
///
/// Places every object with a position of its own and orients every object that has an
/// orientation, as the best candidate of the search says, every fact kept holding exactly, the
/// positions that the file fixes included (the draft 0.3, 14.8); a text that its owner's symbol
/// places is given its position when the scene gives it an axis of its own (13.3). What the solver
/// has nothing to place by is left free, neither placed nor oriented: a text attached to a wire or
/// a net, which has no coordinate point, or to anything else the solver has no point for, and that
/// no fact kept reaches, which the viewer places next to what it is attached to (13.3); and a
/// supply, ground or port symbol without a name that no fact kept reaches, on no net with a pin
/// that the problem shows, which a viewer leaves unplaced (11.1). Routes the ROUTEs, each wire from
/// a pin or a branch point to a pin or a branch point (13.2, 13.7). The objectives that are on but
/// inert in the level, and the ROUTEs left in parts, are in the report.
pub fn solve(problem: &LevelProblem, options: &SolverOptions) -> Solved {
    solve_with_clock(problem, options, &StoppedClock)
}

/// [`solve`] measured on the caller's clock. For 0.4, the shared time or work budget covers
/// resolution, structure, search, coordinates and routing. Use the report's canonical options
/// when hashing the persisted result, so time-to-work replay includes the same hash.
/// Released input versions retain their resolution-only time limit.
pub fn solve_with_clock(problem: &LevelProblem, options: &SolverOptions, clock: &dyn Clock) -> Solved {
    budget::without(|| solve_isolated(problem, options, clock, driver::solve))
}

fn solve_isolated(
    problem: &LevelProblem,
    options: &SolverOptions,
    clock: &dyn Clock,
    run: impl FnOnce(&LevelProblem, &SolverOptions, &dyn Clock) -> Solved,
) -> Solved {
    let (effective, warnings) = options.for_problem(problem);
    let options = &effective;
    profile::with(clock.profiler(), || {
        if profile::enabled() {
            profile::emit(profile::ProfileEvent::Level {
                level: problem.level,
                title: problem.title.clone(),
                objects: problem.objects.len(),
                facts: problem.facts.len(),
                routes: problem.routes.len(),
                options: options.canonical(),
                hash: rnl_runtime::problem::sig2(problem, &options.canonical()),
            });
        }
        if profile::enabled() {
            profile::set("input_nets", problem.nets.len() as u64);
            profile::set("input_groups", problem.groups.len() as u64);
            profile::set(
                "input_host_instances",
                problem.objects.iter().filter(|o| o.kind == rnl_runtime::problem::ObjectKind::Instance).count() as u64,
            );
        }
        let mut solved = if problem.version == "0.4" {
            let (limit, safety) = run_limits(options, clock);
            let (mut solved, report) = budget::with(clock, limit, safety, || run(problem, options, clock));
            let replay_limit =
                if options.settings.layout == Layout::Lines && matches!(limit, RunLimit::Seed | RunLimit::Unlimited) {
                    limit
                } else {
                    RunLimit::Work(report.work)
                };
            let replay = SolverOptions { run_limit: Some(replay_limit), time_limit: None, ..options.clone() };
            solved.report.canonical_options = Some(replay.canonical());
            solved.report.budget = Some(report);
            solved.report.notes.push(if matches!(replay_limit, RunLimit::Work(_)) {
                format!(
                    "budget: {} work units; limit: {:?}; stopped: {:?}; replay with limit=\"work\", work={}",
                    report.work, report.limit, report.reason, report.work
                )
            } else {
                format!(
                    "budget: {} work units; limit: {:?}; stopped: {:?}; deterministic {:?} settings retained",
                    report.work, report.limit, report.reason, replay_limit
                )
            });
            solved
        } else {
            run(problem, options, clock)
        };
        solved.report.warnings.extend(warnings);
        profile::emit(profile::ProfileEvent::LevelEnd);
        solved
    })
}

/// Resolves the facts of one drawing level (8.5): checks each on its own first, a fact with no
/// solution on its own failing (the draft 0.3, 8.5; 7.6); takes the others in the order of
/// [`LevelProblem::facts`], keeps each that fits with those kept before it and discards the others,
/// each with every minimal group of facts it conflicts with, says for each statement which facts
/// were discarded because of it (the draft 0.3, 8.5), and gives a witness, a placement that
/// satisfies every fact kept ([`Resolved`]). The time limit is never reached:
/// [`resolve_with_clock`] with a [`StoppedClock`].
pub fn resolve(problem: &LevelProblem, options: &SolverOptions) -> Resolved {
    resolve_with_clock(problem, options, &StoppedClock)
}

/// [`resolve()`], with the time limit ([`SolverOptions::time_limit`], the draft 0.3, 8.5) measured
/// on `clock`.
pub fn resolve_with_clock(problem: &LevelProblem, options: &SolverOptions, clock: &dyn Clock) -> Resolved {
    budget::without(|| {
        let (options, warnings) = options.for_problem(problem);
        let mut resolved = if problem.version == "0.4" {
            let (limit, safety) = run_limits(&options, clock);
            let (mut resolved, report) =
                budget::with(clock, limit, safety, || resolve::resolve(problem, &options, clock));
            resolved.budget = Some(report);
            resolved
        } else {
            resolve::resolve(problem, &options, clock)
        };
        resolved.warnings.extend(warnings);
        resolved
    })
}

/// The limit a level is solved with under `options`, read on `clock`: a time limit on a clock that
/// does not advance is the initial pass instead.
pub fn run_limit(options: &SolverOptions, clock: &dyn Clock) -> RunLimit {
    run_limits(options, clock).0
}

fn run_limits(options: &SolverOptions, clock: &dyn Clock) -> (RunLimit, Option<f64>) {
    let mut limit =
        options.run_limit.unwrap_or(if options.time_limit.is_some() { RunLimit::Time(10.0) } else { RunLimit::Seed });
    if matches!(limit, RunLimit::Time(_)) && !clock.advances() {
        limit = RunLimit::Seed;
    }
    let safety = (!matches!(limit, RunLimit::Time(_))).then_some(options.safety_seconds).flatten();
    (limit, safety)
}
