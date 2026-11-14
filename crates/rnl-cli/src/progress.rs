//! The progress of a solve as events, one JSON object each: what `rnl solve --progress=json`
//! writes on the standard error, one per line, and what [`solve_progress`] and
//! [`crate::preview::preview_progress`] give their callback. The format is described in
//! `docs/solver-progress.md`; fields are only ever added.

use std::cell::RefCell;
use std::path::Path;

use rnl_runtime::source::Provider;
use rnl_solver::{BudgetProgress, Clock, ProgressKind, RunLimit, StopReason};
use serde::Serialize;

use crate::solve::{solve_marked, BackendResult, Mark, SolveOptions, SolveRun};

/// The version of the event format, in [`Event::Run`].
pub const VERSION: u32 = 1;

/// The least time between two [`Event::Stage`] or [`Event::Work`] of the solver in one level, in
/// seconds.
const EVERY: f64 = 0.25;

/// The solver's stages and the stages of a solve outside the solver, with what each does, in
/// English, for a frontend to show. A stage not listed here is shown by its name.
pub const STAGES: &[(&str, &str)] = &[
    ("resolve", "Checking the facts against each other"),
    ("structure", "Reading the circuit's structure and groups"),
    ("search", "Searching for placements"),
    ("coords", "Placing the candidates on the grid"),
    ("route", "Routing the wires"),
    ("score", "Comparing the candidates"),
    ("tree", "Laying out the groups"),
    ("lines", "Laying out along the signal lines"),
    ("respace", "Spacing the drawing again"),
    ("junctions", "Placing the junction dots"),
    ("write", "Writing the record back"),
];

/// One step of a solve. `level` counts the levels of the run from 0; `elapsed` is seconds since
/// that level began.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "event", rename_all = "kebab-case")]
pub enum Event {
    /// Before the first level: how many levels the run solves.
    Run { progress: u32, levels: usize },
    /// A level begins, with the limit it is solved with, its own settings applied: `limit` is
    /// "time" with `seconds`, "work" with `work`, "seed" or "none".
    Level {
        level: usize,
        title: String,
        again: bool,
        limit: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        seconds: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        work: Option<u64>,
    },
    /// A level enters a stage, of the solver or of the solve around it ([`STAGES`]). The solver's
    /// stages are told at most every quarter of a second: one left sooner is not told.
    Stage { level: usize, stage: String, work: u64, elapsed: f64 },
    /// More work, in the stage `stage`, at most every quarter of a second.
    Work {
        level: usize,
        stage: String,
        work: u64,
        elapsed: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        fraction: Option<f64>,
        tried: u64,
    },
    /// A better candidate than any before in the level: `cost` is only to be compared.
    Best { level: usize, tried: u64, work: u64, elapsed: f64, cost: Vec<f64> },
    /// A level is solved.
    LevelDone {
        level: usize,
        status: &'static str,
        work: u64,
        seconds: f64,
        kept: usize,
        discarded: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        junction_seconds: Option<f64>,
    },
    /// The run is over, its files written: the CLI's last event.
    Done {
        seconds: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        exit: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        written: Option<usize>,
    },
    /// The run stopped on an error.
    Error { message: String },
}

impl Event {
    /// The event as one line of JSON.
    pub fn line(&self) -> String {
        serde_json::to_string(self).expect("events serialize")
    }
}

#[derive(Default)]
struct State {
    level: usize,
    /// The reading of the clock when the level began.
    start: f64,
    work: u64,
    /// When the solver's last stage or work was told.
    last: Option<f64>,
    /// The level about to be solved, until its problem is read.
    pending: Option<(String, bool)>,
    cancelled: bool,
    /// Readings since the solve was cancelled.
    after: u64,
    /// The seconds each level's solve took on the caller's clock, in the order solved.
    solved_in: Vec<f64>,
}

/// A clock that turns the solver's progress into [`Event`]s for `sink`. When `sink` answers
/// false, the solve is cancelled: the clock then reads far past any time limit, so that a level
/// solved with one stops at once, and the solve stops before the next level.
struct Observer<'a> {
    inner: &'a dyn Clock,
    sink: &'a dyn Fn(&Event) -> bool,
    state: RefCell<State>,
}

impl Observer<'_> {
    fn send(&self, event: &Event) -> bool {
        let go = (self.sink)(event);
        if !go {
            self.state.borrow_mut().cancelled = true;
        }
        go
    }

    fn since(&self) -> f64 {
        let start = self.state.borrow().start;
        (self.inner.elapsed() - start).max(0.)
    }
}

impl Clock for Observer<'_> {
    fn check_every(&self) -> u64 {
        self.inner.check_every()
    }
    fn advances(&self) -> bool {
        self.inner.advances()
    }
    fn elapsed(&self) -> f64 {
        let mut s = self.state.borrow_mut();
        if s.cancelled {
            s.after += 1;
            1e12 + s.after as f64 * 1e9
        } else {
            drop(s);
            self.inner.elapsed()
        }
    }
    fn profiler(&self) -> Option<rnl_solver::profile::ProfileHandle> {
        self.inner.profiler()
    }
    fn observes_progress(&self) -> bool {
        true
    }
    fn progress(&self, p: BudgetProgress) {
        self.inner.progress(p.clone());
        // nothing more is told once cancelled: the clock reads far past the limit
        if self.state.borrow().cancelled {
            return;
        }
        let (level, elapsed) = {
            let mut s = self.state.borrow_mut();
            s.work = p.work;
            (s.level, p.elapsed)
        };
        let event = match p.kind {
            ProgressKind::Stage | ProgressKind::Work => {
                let mut s = self.state.borrow_mut();
                // the first of a level is always told
                if s.last.is_some_and(|t| elapsed - t < EVERY) {
                    return;
                }
                s.last = Some(elapsed);
                let stage = p.solver_stage.to_string();
                if p.kind == ProgressKind::Stage {
                    Event::Stage { level, stage, work: p.work, elapsed }
                } else {
                    Event::Work { level, stage, work: p.work, elapsed, fraction: p.fraction, tried: p.tried }
                }
            }
            ProgressKind::Best => Event::Best { level, tried: p.tried, work: p.work, elapsed, cost: p.cost },
            ProgressKind::Complete => return,
        };
        self.send(&event);
    }
}

/// The limit of a level's run, as [`Event::Level`] gives it.
fn limit_of(limit: RunLimit) -> (&'static str, Option<f64>, Option<u64>) {
    match limit {
        RunLimit::Time(s) => ("time", Some(s), None),
        RunLimit::Work(n) => ("work", None, Some(n)),
        RunLimit::Unlimited => ("none", None, None),
        RunLimit::Seed => ("seed", None, None),
    }
}

/// How a level's budget stopped, as [`Event::LevelDone`] gives it; "complete" for a solver that
/// reports no budget.
fn status_of(reason: Option<StopReason>, cancelled: bool) -> &'static str {
    match reason {
        _ if cancelled => "cancelled",
        None | Some(StopReason::Complete) => "complete",
        Some(StopReason::Time) => "time",
        Some(StopReason::Work) => "work",
        Some(StopReason::Safety) => "safety",
    }
}

/// [`crate::solve::solve_watched`], with every step of the solve given to `sink` as an [`Event`],
/// in order, on the thread that solves. When `sink` answers false the solve is cancelled: a level
/// with a time limit stops at the solver's next check, and the solve fails with
/// [`crate::solve::CANCELLED`] before the next level. [`Event::Done`] is left to the caller, which
/// knows when the files are written.
pub fn solve_progress(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
    sink: &dyn Fn(&Event) -> bool,
) -> Result<SolveRun, String> {
    solve_progress_using(file, files, options, clock, sink, &|p, options, clock| {
        Ok(BackendResult {
            solved: rnl_solver::solve_with_clock(p, options, clock),
            author: rnl_solver::AUTHOR.to_string(),
            report_available: true,
        })
    })
}

pub(crate) type ClockedBackend<'a> = dyn Fn(&rnl_runtime::problem::LevelProblem, &rnl_solver::SolverOptions, &dyn Clock) -> Result<BackendResult, String>
    + 'a;

/// [`solve_progress`] through a caller-selected backend, given the observing clock.
pub fn solve_progress_using(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
    sink: &dyn Fn(&Event) -> bool,
    backend: &ClockedBackend<'_>,
) -> Result<SolveRun, String> {
    solve_progress_skipping(file, files, options, clock, sink, &|_| None, backend)
}

/// [`solve_progress_using`], the levels `skip` gives a reason for left as they are.
pub(crate) fn solve_progress_skipping(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
    sink: &dyn Fn(&Event) -> bool,
    skip: &crate::solve::Skip<'_>,
    backend: &ClockedBackend<'_>,
) -> Result<SolveRun, String> {
    let observer = Observer { inner: clock, sink, state: RefCell::new(State::default()) };
    let watch = |mark: Mark<'_>| -> bool {
        let event = match mark {
            Mark::Run { levels } => Event::Run { progress: VERSION, levels },
            Mark::Level(p) => {
                let mut s = observer.state.borrow_mut();
                s.level = p.level;
                s.work = 0;
                s.last = None;
                s.pending = Some((p.title.clone(), p.again));
                return !s.cancelled;
            }
            Mark::Problem(solver) => {
                let Some((title, again)) = observer.state.borrow_mut().pending.take() else { return true };
                let (limit, seconds, work) = limit_of(rnl_solver::run_limit(solver, clock));
                let start = clock.elapsed();
                let level = {
                    let mut s = observer.state.borrow_mut();
                    s.start = start;
                    s.level
                };
                Event::Level { level, title, again, limit, seconds, work }
            }
            Mark::Stage(stage) => {
                let s = observer.state.borrow();
                let (level, work) = (s.level, s.work);
                drop(s);
                Event::Stage { level, stage: stage.to_string(), work, elapsed: observer.since() }
            }
            Mark::LevelDone { level, solved, seconds, junction_seconds } => {
                let budget = solved.report.budget;
                // WebAssembly has no clock of its own to time the solve with: the caller's then
                let seconds = match observer.state.borrow().solved_in.last() {
                    Some(&on_the_clock) if OWN_CLOCK_MISSING => on_the_clock,
                    _ => seconds,
                };
                Event::LevelDone {
                    level,
                    status: status_of(budget.map(|b| b.reason), observer.state.borrow().cancelled),
                    work: budget.map_or(0, |b| b.work),
                    seconds,
                    kept: solved.report.kept.len(),
                    discarded: solved.report.discarded.len(),
                    junction_seconds,
                }
            }
        };
        observer.send(&event) && !observer.state.borrow().cancelled
    };
    let timed = |p: &rnl_runtime::problem::LevelProblem, options: &rnl_solver::SolverOptions| {
        let from = clock.elapsed();
        let result = backend(p, options, &observer);
        observer.state.borrow_mut().solved_in.push((clock.elapsed() - from).max(0.));
        result
    };
    let mut run = solve_marked(file, files, options, &watch, skip, &timed)?;
    // cancelled in the last level: nothing after it to stop before
    if observer.state.borrow().cancelled {
        return Err(crate::solve::CANCELLED.to_string());
    }
    if OWN_CLOCK_MISSING {
        // a file raised to 0.3 is solved in two passes, and the run is the second's
        let solved_in = &observer.state.borrow().solved_in;
        let levels = run.levels.len();
        if solved_in.len() == levels || solved_in.len() == 2 * levels {
            for (level, &seconds) in run.levels.iter_mut().zip(&solved_in[solved_in.len() - levels..]) {
                level.time = std::time::Duration::from_secs_f64(seconds);
            }
        }
    }
    Ok(run)
}

/// Whether the build has no clock of its own ([`crate::solve::Stopwatch`]), as WebAssembly: the
/// time of a level's solve is then the one on the caller's clock.
const OWN_CLOCK_MISSING: bool = cfg!(target_arch = "wasm32");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_are_one_line_of_json_each_tagged_by_kind() {
        let e =
            Event::Work { level: 0, stage: "coords".into(), work: 12, elapsed: 0.5, fraction: Some(0.05), tried: 3 };
        assert_eq!(
            e.line(),
            r#"{"event":"work","level":0,"stage":"coords","work":12,"elapsed":0.5,"fraction":0.05,"tried":3}"#
        );
        let e = Event::LevelDone {
            level: 1,
            status: "time",
            work: 9,
            seconds: 10.0,
            kept: 4,
            discarded: 1,
            junction_seconds: None,
        };
        assert_eq!(
            e.line(),
            r#"{"event":"level-done","level":1,"status":"time","work":9,"seconds":10.0,"kept":4,"discarded":1}"#
        );
    }

    #[test]
    fn every_stage_has_a_line_of_its_own() {
        let mut names: Vec<_> = STAGES.iter().map(|(n, _)| *n).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), STAGES.len());
        assert!(STAGES.iter().all(|(_, what)| !what.is_empty()));
    }
}
