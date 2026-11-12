//! A level's shared, replayable budget. Only admitted checkpoints consume work. A time
//! run and a work run therefore stop before the same operation, including inside a stage.
//! The borrowed clock lives only for this scope; nested solves restore the outer budget.

use std::cell::Cell;

use crate::Clock;

/// The limit of a complete 0.4 solve, including fact resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RunLimit {
    Time(f64),
    Work(u64),
    Unlimited,
    /// The initial layout pass, including its layout-specific greedy refinement.
    Seed,
}

/// Why the solver stopped admitting optional work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Complete,
    Time,
    Work,
    Safety,
}

/// Work admitted by the budget, independently of optional profiling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BudgetReport {
    pub limit: RunLimit,
    pub work: u64,
    pub reason: StopReason,
}

/// Observational progress, outside persisted results.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetProgress {
    pub work: u64,
    pub stage: &'static str,
}

impl BudgetReport {
    /// A safety interruption depends on elapsed time, even in work mode.
    pub fn reproducible(self) -> bool {
        self.reason != StopReason::Safety
    }
}

/// Fixed integer weights. Costs also scale with the size of the operation's input.
/// Changing these weights changes the replay protocol and requires a solver version change.
#[derive(Clone, Copy)]
pub(crate) enum Step {
    Resolve,
    Pivot,
    Candidate,
    Placement,
    Route,
    Structure,
}

impl Step {
    fn name(self) -> &'static str {
        match self {
            Self::Resolve => "resolution",
            Self::Pivot => "coordinates",
            Self::Candidate => "search",
            Self::Placement => "placement",
            Self::Route => "routing",
            Self::Structure => "structure",
        }
    }
    fn weight(self) -> u64 {
        match self {
            Self::Resolve => 8,
            Self::Pivot => 16,
            Self::Candidate => 32,
            Self::Placement => 32,
            Self::Route => 1,
            Self::Structure => 4,
        }
    }
}

struct Control<'a> {
    clock: &'a dyn Clock,
    start: f64,
    limit: RunLimit,
    safety_seconds: Option<f64>,
    used: Cell<u64>,
    stopped: Cell<Option<StopReason>>,
    last_progress: Cell<u64>,
}

scoped_tls_hkt::scoped_thread_local!(static ACTIVE: for<'a> Option<&'a Control<'a>>);

impl Control<'_> {
    fn permit(&self, cost: u64) -> bool {
        if self.stopped.get().is_some() {
            return false;
        }
        let next = self.used.get().checked_add(cost);
        let reason = if next.is_none() || matches!(self.limit, RunLimit::Work(n) if next.unwrap() > n) {
            Some(StopReason::Work)
        } else if matches!(self.limit, RunLimit::Time(_)) || self.safety_seconds.is_some() {
            let elapsed = self.clock.elapsed() - self.start;
            if matches!(self.limit, RunLimit::Time(s) if elapsed >= s) {
                Some(StopReason::Time)
            } else if self.safety_seconds.is_some_and(|s| elapsed >= s) {
                Some(StopReason::Safety)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(reason) = reason {
            self.stopped.set(Some(reason));
            false
        } else {
            self.used.set(next.unwrap());
            true
        }
    }
}

pub(crate) fn with<T>(
    clock: &dyn Clock,
    limit: RunLimit,
    safety_seconds: Option<f64>,
    f: impl FnOnce() -> T,
) -> (T, BudgetReport) {
    let control = Control {
        clock,
        start: clock.elapsed(),
        limit,
        safety_seconds,
        used: Cell::new(0),
        stopped: Cell::new(None),
        last_progress: Cell::new(0),
    };
    let result = ACTIVE.set(Some(&control), f);
    clock.progress(BudgetProgress { work: control.used.get(), stage: "complete" });
    (
        result,
        BudgetReport { limit, work: control.used.get(), reason: control.stopped.get().unwrap_or(StopReason::Complete) },
    )
}

pub(crate) fn active() -> bool {
    ACTIVE.is_set() && ACTIVE.with(|c| c.is_some())
}

/// Public entry points cannot borrow a caller's budget through an observer callback.
pub(crate) fn without<T>(f: impl FnOnce() -> T) -> T {
    ACTIVE.set(None, f)
}

pub(crate) fn continues_rows() -> bool {
    active() && ACTIVE.with(|c| c.unwrap().limit != RunLimit::Seed)
}

pub(crate) fn stopped() -> bool {
    active() && ACTIVE.with(|c| c.unwrap().stopped.get().is_some())
}

/// Never charge zero: otherwise a work count would not uniquely identify the cutoff.
pub(crate) fn permit(step: Step, size: usize) -> bool {
    !active()
        || ACTIVE.with(|c| {
            let c = c.unwrap();
            let admitted = c.permit(step.weight().saturating_mul((size as u64).max(1)));
            if admitted && c.used.get().saturating_sub(c.last_progress.get()) >= 100_000 {
                c.last_progress.set(c.used.get());
                c.clock.progress(BudgetProgress { work: c.used.get(), stage: step.name() });
            }
            admitted
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StoppedClock;

    struct Ticks(Cell<u64>);
    impl Clock for Ticks {
        fn elapsed(&self) -> f64 {
            let n = self.0.get();
            self.0.set(n + 1);
            n as f64
        }
    }

    #[test]
    fn time_work_replay_has_the_same_admitted_prefix() {
        for seconds in 0..20 {
            let run = || {
                let mut admitted = Vec::new();
                for i in 0..30 {
                    if !permit(Step::Pivot, i) {
                        break;
                    }
                    admitted.push(i);
                }
                admitted
            };
            let (timed, report) = with(&Ticks(Cell::new(0)), RunLimit::Time(seconds as f64), None, run);
            let (replayed, again) = with(&StoppedClock, RunLimit::Work(report.work), None, run);
            assert_eq!(timed, replayed);
            assert_eq!(report.work, again.work);
        }
    }

    #[test]
    fn scope_restores_after_nested_solve_and_panic_and_is_thread_local() {
        let (_, report) = with(&StoppedClock, RunLimit::Work(5), None, || {
            assert!(permit(Step::Route, 2));
            let _ = std::panic::catch_unwind(|| {
                with(&StoppedClock, RunLimit::Work(0), None, || {
                    assert!(!permit(Step::Route, 1));
                    panic!("scope unwinds");
                })
            });
            assert!(permit(Step::Route, 3));
            assert!(!permit(Step::Route, 1));
            std::thread::spawn(|| assert!(!active())).join().unwrap();
        });
        assert_eq!(report.work, 5);
        assert!(!active());
    }

    #[test]
    fn safety_stop_is_explicit_and_latched() {
        let clock = Ticks(Cell::new(0));
        let (_, report) = with(&clock, RunLimit::Work(100), Some(2.0), || {
            assert!(permit(Step::Route, 1));
            assert!(!permit(Step::Route, 1));
            let reads = clock.0.get();
            assert!(!permit(Step::Route, 1));
            assert_eq!(clock.0.get(), reads);
        });
        assert_eq!(report.reason, StopReason::Safety);
        assert!(!report.reproducible());
    }

    #[test]
    fn full_solve_replays_at_stage_and_inner_loop_boundaries() {
        use crate::resolve::tests::{fact, instance, problem_of};
        use crate::{solve_with_clock, SolverOptions};
        use rnl_runtime::problem::{sig2, Axis, FactKind, PointRef};
        let mut p = problem_of(
            vec![instance(), instance()],
            vec![
                fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 4.0 }),
                fact(FactKind::Place { at: PointRef::Origin(1), axis: Axis::Y, value: 3.0 }),
                fact(FactKind::Align { axis: Axis::X, members: vec![PointRef::Origin(0), PointRef::Origin(1)] }),
            ],
        );
        p.version = "0.4".into();
        p.facts.push(fact(FactKind::Orient { object: 0, code: rnl_runtime::geom::Orient::R0 }));
        p.orient_lists.push((3, vec![rnl_runtime::geom::Orient::R0, rnl_runtime::geom::Orient::MY]));
        p.facts.push(fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 4.0000015 }));
        p.facts.push(fact(FactKind::Place {
            at: PointRef::Pin { object: 0, pin: 0, terminal: "g".into() },
            axis: Axis::Y,
            value: 2.0,
        }));
        p.routes.push(rnl_runtime::problem::RouteIntent {
            tree: true,
            net: "n".into(),
            mode: "auto".into(),
            style: Vec::new(),
            wires: Vec::new(),
            pins: vec![
                PointRef::Pin { object: 0, pin: 0, terminal: "g".into() },
                PointRef::Pin { object: 1, pin: 0, terminal: "g".into() },
            ],
        });
        for seconds in (0..30).chain([40, 60, 100, 200, 500]) {
            let options = SolverOptions { run_limit: Some(RunLimit::Time(seconds as f64)), ..SolverOptions::default() };
            let timed = solve_with_clock(&p, &options, &Ticks(Cell::new(0)));
            let work = timed.report.budget.unwrap().work;
            let replay = SolverOptions { run_limit: Some(RunLimit::Work(work)), time_limit: None, ..options };
            let again = solve_with_clock(&p, &replay, &StoppedClock);
            assert_eq!(timed.solution, again.solution, "cutoff {seconds}, work {work}");
            assert_eq!(timed.report.kept, again.report.kept);
            assert_eq!(timed.report.discarded, again.report.discarded);
            let mut drawn = vec![rnl_runtime::previous::Drawn { at: [None, None], orient: None }; p.objects.len()];
            for placed in &timed.solution.placements {
                drawn[placed.object] = rnl_runtime::previous::Drawn { at: [placed.x, placed.y], orient: placed.orient };
            }
            for &f in &timed.report.kept {
                assert_eq!(
                    crate::problem_fact_holds_as_drawn(&p, &drawn, f, 1.00001e-6),
                    Some(true),
                    "kept fact {f}, cutoff {seconds}"
                );
            }
            let written = |s: &crate::Solved| {
                let hash = sig2(&p, s.report.canonical_options.as_ref().unwrap());
                rnl_runtime::solution::write_record(&p, &s.solution, crate::AUTHOR, &hash).record
            };
            assert_eq!(written(&timed), written(&again), "record cutoff {seconds}");
        }
    }

    #[test]
    fn table_expansion_stops_without_materializing_its_cartesian_product() {
        use crate::resolve::tests::{fact, point, problem_of};
        use rnl_runtime::problem::{Axis, FactKind, PointRef, TableCell};
        let row = |object| TableCell::List(vec![TableCell::Point(PointRef::Origin(object)); 5000]);
        let p = problem_of(
            vec![point(), point()],
            vec![fact(FactKind::Table { axis: Axis::X, rows: TableCell::List(vec![row(0), row(1)]) })],
        );
        let (model, report) =
            with(&StoppedClock, RunLimit::Work(32), None, || crate::model::Model::new(&p, crate::rational::int(0)));
        assert_eq!(report.reason, StopReason::Work);
        assert_eq!(report.work, 32);
        assert!(model.unfinished.contains(&0));
        assert!(model.units[0].fixed.is_empty());
    }

    #[test]
    fn routing_time_cutoff_after_expansions_replays_the_complete_record() {
        use crate::resolve::tests::{fact, instance, problem_of};
        use rnl_runtime::problem::{sig2, Axis, FactKind, PointRef, RouteIntent};
        let mut p = problem_of(
            vec![instance(), instance()],
            vec![
                fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 0.0 }),
                fact(FactKind::Place { at: PointRef::Origin(1), axis: Axis::X, value: 20.0 }),
            ],
        );
        p.version = "0.4".into();
        p.routes.push(RouteIntent {
            tree: true,
            net: "n".into(),
            mode: "auto".into(),
            style: vec![],
            wires: vec![],
            pins: (0..2).map(|object| PointRef::Pin { object, pin: 0, terminal: "g".into() }).collect(),
        });
        for seconds in 0..400 {
            let options = crate::SolverOptions {
                objectives: crate::Objectives::none(),
                beam: 1,
                budget: crate::Budget { candidates: 0, astar_expansions: 200_000 },
                run_limit: Some(RunLimit::Time(f64::from(seconds))),
                ..Default::default()
            };
            let timed = crate::solve_with_clock(&p, &options, &Ticks(Cell::new(0)));
            let report = timed.report.budget.unwrap();
            if report.reason != StopReason::Time || timed.report.work.astar_expansions == 0 {
                continue;
            }
            let replay =
                crate::solve(&p, &crate::SolverOptions { run_limit: Some(RunLimit::Work(report.work)), ..options });
            let written = |s: &crate::Solved| {
                rnl_runtime::solution::write_record(
                    &p,
                    &s.solution,
                    crate::AUTHOR,
                    &sig2(&p, s.report.canonical_options.as_ref().unwrap()),
                )
                .record
            };
            assert_eq!(written(&timed), written(&replay));
            assert!(timed.report.unrouted.is_empty());
            return;
        }
        panic!("the test must interrupt a route after at least one A* expansion");
    }

    #[test]
    fn observer_nested_public_calls_cannot_consume_the_outer_budget() {
        struct Nested;
        impl Clock for Nested {
            fn elapsed(&self) -> f64 {
                0.0
            }
            fn progress(&self, _: BudgetProgress) {
                use crate::resolve::tests::{fact, point, problem_of};
                use rnl_runtime::problem::{Axis, FactKind, PointRef};
                let mut p = problem_of(
                    vec![point()],
                    vec![fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 4.0 })],
                );
                let options = crate::SolverOptions::without_time_limit();
                assert_eq!(crate::solve(&p, &options).report.kept, vec![0]);
                assert_eq!(crate::resolve(&p, &options).kept, vec![0]);
                p.version = "0.4".into();
                let stopped =
                    crate::resolve(&p, &crate::SolverOptions { run_limit: Some(RunLimit::Work(0)), ..options });
                assert_eq!(stopped.budget.unwrap().work, 0);
            }
        }
        let (_, report) = with(&Nested, RunLimit::Work(160001), None, || {
            assert!(permit(Step::Pivot, 10_000));
            assert!(permit(Step::Route, 1));
        });
        assert_eq!(report.work, 160001);
        assert_eq!(report.reason, StopReason::Complete);
    }
}
