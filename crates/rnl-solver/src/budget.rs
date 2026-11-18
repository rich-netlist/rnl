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

/// What a [`BudgetProgress`] tells of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressKind {
    /// The solver entered a stage of its own, [`BudgetProgress::solver_stage`].
    Stage,
    /// More work was admitted.
    Work,
    /// The solver took a better candidate than any before in this level.
    Best,
    /// The level's budget closed.
    Complete,
}

/// Observational progress, outside persisted results. Observing never consumes work. The clock
/// is read for `elapsed` and `fraction` only when [`Clock::observes_progress`] says so.
#[derive(Clone, Debug, PartialEq)]
pub struct BudgetProgress {
    pub kind: ProgressKind,
    pub work: u64,
    /// The kind of step that last admitted work, such as "coordinates" or "search"; "complete"
    /// when the budget closed.
    pub stage: &'static str,
    /// The stage of the solver: "resolve", "structure", "search", "coords", "route", "score",
    /// "tree" or "lines"; empty before the first.
    pub solver_stage: &'static str,
    /// The candidates admitted so far in this level.
    pub tried: u64,
    /// The seconds since the level's budget opened, by its clock.
    pub elapsed: f64,
    /// The share of a time or a work limit used so far, from 0 to 1.
    pub fraction: Option<f64>,
    /// With [`ProgressKind::Best`], the new best's cost: lower is better, compared element by
    /// element. Its entries carry no promised meaning.
    pub cost: Vec<f64>,
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
    Line,
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
            Self::Line => "lines",
        }
    }
    fn weight(self) -> u64 {
        match self {
            Self::Resolve => 8,
            Self::Pivot => 2,
            Self::Candidate => 32,
            Self::Placement => 32,
            Self::Route => 1,
            Self::Structure => 4,
            Self::Line => 1,
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
    tried: Cell<u64>,
    solver_stage: Cell<&'static str>,
    step: Cell<&'static str>,
    /// Whether the lines layout's group-tree start ran ([`note_tree_start`]), and the work at which
    /// it stopped refining by its allowance, if it did ([`note_tree_stop`]).
    tree_stop: Cell<Option<Option<u64>>>,
    /// The work between two readings of the clock for the limits ([`Clock::check_every`]), the
    /// work at which it is read next, and the steps admitted since it was read.
    every: u64,
    clock_due: Cell<u64>,
    steps: Cell<u32>,
    /// Inside [`past_limit`]: the work done there, which `used` leaves out, the work at which the
    /// clock is read next for the safety limit, and whether that limit stopped it.
    past: Cell<Option<u64>>,
    past_due: Cell<u64>,
    past_stopped: Cell<bool>,
    /// Whether the safety limit stopped steps past the limit at any time in the run: what those
    /// steps did then hangs on the clock, so that the run is not replayed by its work.
    past_safety: Cell<bool>,
}

/// The steps admitted on one reading of the clock at most ([`Control::late`]).
const CHECK_STEPS: u32 = 256;

scoped_tls_hkt::scoped_thread_local!(static ACTIVE: for<'a> Option<&'a Control<'a>>);

impl Control<'_> {
    fn observe(&self, kind: ProgressKind, cost: Vec<f64>) {
        let (elapsed, fraction) = if self.clock.observes_progress() {
            let elapsed = self.clock.elapsed() - self.start;
            let fraction = match self.limit {
                RunLimit::Time(s) if s > 0. => Some((elapsed / s).clamp(0., 1.)),
                RunLimit::Work(n) if n > 0 => Some((self.used.get() as f64 / n as f64).clamp(0., 1.)),
                _ => None,
            };
            (elapsed, fraction)
        } else {
            (0., None)
        };
        self.clock.progress(BudgetProgress {
            kind,
            work: self.used.get(),
            stage: self.step.get(),
            solver_stage: self.solver_stage.get(),
            tried: self.tried.get(),
            elapsed,
            fraction,
            cost,
        });
    }

    fn permit(&self, cost: u64) -> bool {
        if let Some(past) = self.past.get() {
            return self.permit_past(past, cost);
        }
        if self.stopped.get().is_some() {
            return false;
        }
        let reason = match self.used.get().checked_add(cost) {
            None => Some(StopReason::Work),
            Some(next) if matches!(self.limit, RunLimit::Work(n) if next > n) => Some(StopReason::Work),
            Some(next) => {
                let reason = self.late(next);
                if reason.is_none() {
                    self.used.set(next);
                }
                reason
            }
        };
        if let Some(reason) = reason {
            self.stopped.set(Some(reason));
        }
        reason.is_none()
    }

    /// A step past the limit ([`past_limit`]): admitted until the safety limit passes, its work
    /// counted apart. The clock is read as [`Control::late`] reads it.
    fn permit_past(&self, past: u64, cost: u64) -> bool {
        if self.past_stopped.get() {
            return false;
        }
        let next = past.saturating_add(cost);
        let steps = self.steps.get() + 1;
        if let Some(safety) = self.safety_seconds {
            if steps >= CHECK_STEPS || next >= self.past_due.get() {
                self.steps.set(0);
                self.past_due.set(next.saturating_add(self.every));
                if self.clock.elapsed() - self.start >= safety {
                    self.past_stopped.set(true);
                    self.past_safety.set(true);
                    return false;
                }
            } else {
                self.steps.set(steps);
            }
        }
        self.past.set(Some(next));
        true
    }

    /// Whether the time or the safety limit has passed, for a step that would bring the work to
    /// `next`. The clock is read when [`Clock::check_every`] more work is done since its last
    /// reading, and at each [`CHECK_STEPS`]th step at the latest, since some steps cost a unit and
    /// take a third of a microsecond; in between, a step is admitted on the last reading. A stop
    /// is still at a step, with the work so far, so that the work reported replays it exactly.
    fn late(&self, next: u64) -> Option<StopReason> {
        if !matches!(self.limit, RunLimit::Time(_)) && self.safety_seconds.is_none() {
            return None;
        }
        let steps = self.steps.get() + 1;
        if steps < CHECK_STEPS && next < self.clock_due.get() {
            self.steps.set(steps);
            return None;
        }
        self.steps.set(0);
        self.clock_due.set(next.saturating_add(self.every));
        let elapsed = self.clock.elapsed() - self.start;
        if matches!(self.limit, RunLimit::Time(s) if elapsed >= s) {
            Some(StopReason::Time)
        } else if self.safety_seconds.is_some_and(|s| elapsed >= s) {
            Some(StopReason::Safety)
        } else {
            None
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
        tried: Cell::new(0),
        solver_stage: Cell::new(""),
        step: Cell::new(""),
        tree_stop: Cell::new(None),
        every: clock.check_every(),
        clock_due: Cell::new(0),
        steps: Cell::new(0),
        past: Cell::new(None),
        past_due: Cell::new(0),
        past_stopped: Cell::new(false),
        past_safety: Cell::new(false),
    };
    let result = ACTIVE.set(Some(&control), f);
    control.step.set("complete");
    control.observe(ProgressKind::Complete, Vec::new());
    // steps past the limit that the safety limit stopped did what the clock let them: the run
    // stopped by its safety limit, which its work does not replay
    let reason = if control.past_safety.get() {
        StopReason::Safety
    } else {
        control.stopped.get().unwrap_or(StopReason::Complete)
    };
    (result, BudgetReport { limit, work: control.used.get(), reason })
}

/// Tell the observer that the solver entered `stage`, when it is not the stage it is in.
pub(crate) fn enter(stage: &'static str) {
    if active() {
        ACTIVE.with(|c| {
            let c = c.unwrap();
            if c.solver_stage.get() != stage {
                c.solver_stage.set(stage);
                c.observe(ProgressKind::Stage, Vec::new());
            }
        });
    }
}

/// Tell the observer that the solver took a better candidate, of this cost.
pub(crate) fn best(cost: Vec<f64>) {
    if active() {
        ACTIVE.with(|c| c.unwrap().observe(ProgressKind::Best, cost));
    }
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
    active()
        && ACTIVE.with(|c| {
            let c = c.unwrap();
            if c.past.get().is_some() {
                c.past_stopped.get()
            } else {
                c.stopped.get().is_some()
            }
        })
}

/// Runs `f` past the limit the budget has reached: its steps are admitted until the safety limit
/// passes, the only limit that stops it, and their work is counted apart from the work reported,
/// so that a solve limited by work, run again to the work it reported, stops where it stopped and
/// does the same past it. Where the safety limit stops `f`, the run's reason is
/// [`StopReason::Safety`], whatever stopped it before, as the clock decided what `f` did. A time
/// limit has no safety limit, so that it replays by work: there `f` is bounded by its own caps
/// alone. Gives what `f` gives, the work it did, and whether the safety limit stopped it. Without a
/// budget, or one not stopped, `f` runs as it would.
pub(crate) fn past_limit<T>(f: impl FnOnce() -> T) -> (T, u64, bool) {
    if !stopped() {
        return (f(), 0, false);
    }
    ACTIVE.with(|c| {
        let c = c.unwrap();
        c.past.set(Some(0));
        c.past_due.set(0);
        c.past_stopped.set(false);
    });
    // the scope is left as it was even if `f` unwinds
    struct Leave;
    impl Drop for Leave {
        fn drop(&mut self) {
            ACTIVE.with(|c| c.unwrap().past.set(None));
        }
    }
    let leave = Leave;
    let out = f();
    let (work, safety) = ACTIVE.with(|c| {
        let c = c.unwrap();
        (c.past.get().unwrap_or(0), c.past_stopped.get())
    });
    drop(leave);
    (out, work, safety)
}

/// The product of two counts, as a size: made in `u64`, so that it saturates where it would on
/// any target, not at `usize::MAX` of a 32-bit one.
pub(crate) fn product(a: usize, b: usize) -> u64 {
    (a as u64).saturating_mul(b as u64)
}

/// Never charge zero: otherwise a work count would not uniquely identify the cutoff.
pub(crate) fn permit(step: Step, size: usize) -> bool {
    permit_size(step, size as u64)
}

/// [`permit`] for a size made in `u64`, such as a [`product`] of counts, so that a charge is the
/// same on a 32-bit and a 64-bit target.
pub(crate) fn permit_size(step: Step, size: u64) -> bool {
    let cost = step.weight().saturating_mul(size.max(1));
    permit_weighted(step, cost)
}

/// Admit a calibrated cost without narrowing it to the platform's pointer width.
pub(crate) fn permit_weighted(step: Step, cost: u64) -> bool {
    let cost = cost.max(1);
    !active()
        || ACTIVE.with(|c| {
            let c = c.unwrap();
            let admitted = c.permit(cost);
            if admitted {
                c.step.set(step.name());
                if matches!(step, Step::Candidate) {
                    c.tried.set(c.tried.get() + 1);
                }
            }
            if admitted && c.used.get().saturating_sub(c.last_progress.get()) >= 100_000 {
                c.last_progress.set(c.used.get());
                c.observe(ProgressKind::Work, Vec::new());
            }
            admitted
        })
}

/// The common work count, also used for diagnostic costs of line-search solutions.
pub(crate) fn current_limit() -> Option<RunLimit> {
    if active() {
        ACTIVE.with(|c| Some(c.unwrap().limit))
    } else {
        None
    }
}

/// The group-tree start of the lines layout begins, for the replay of the solve
/// (`SolverOptions::tree_work`).
pub(crate) fn note_tree_start() {
    if active() {
        ACTIVE.with(|c| {
            let c = c.unwrap();
            if c.tree_stop.get().is_none() {
                c.tree_stop.set(Some(None));
            }
        });
    }
}

/// The group-tree start of the lines layout stopped refining by its allowance, at the work so far:
/// the first time only, for the replay of the solve (`SolverOptions::tree_work`).
pub(crate) fn note_tree_stop() {
    if active() {
        ACTIVE.with(|c| {
            let c = c.unwrap();
            if !matches!(c.tree_stop.get(), Some(Some(_))) {
                c.tree_stop.set(Some(Some(c.used.get())));
            }
        });
    }
}

/// Whether the group-tree start of the lines layout ran in this solve, and where it stopped
/// refining by its allowance, if it did.
pub(crate) fn tree_stop() -> Option<Option<u64>> {
    if active() {
        ACTIVE.with(|c| c.unwrap().tree_stop.get())
    } else {
        None
    }
}

pub(crate) fn consumed() -> u64 {
    if active() {
        ACTIVE.with(|c| c.unwrap().used.get())
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StoppedClock;

    #[test]
    fn calibrated_costs_keep_their_full_width_and_zero_costs_still_advance() {
        let large = u64::from(u32::MAX) + 17;
        let (accepted, report) = with(&StoppedClock, RunLimit::Work(large + 1), None, || {
            assert!(permit_weighted(Step::Line, large));
            assert_eq!(consumed(), large);
            assert!(permit_weighted(Step::Line, 0));
            permit_weighted(Step::Line, 1)
        });
        assert!(!accepted);
        assert_eq!(report.work, large + 1);
        assert_eq!(report.reason, StopReason::Work);
    }

    /// Past the limit, steps are admitted and their work is kept out of the work reported, so that
    /// the run replays by work; the budget is stopped again after, and a budget not stopped runs
    /// the steps as before.
    #[test]
    fn past_the_limit_steps_are_admitted_and_left_out_of_the_work_reported() {
        let run = || {
            while permit(Step::Route, 3) {}
            assert!(stopped());
            let (admitted, past, safety) = past_limit(|| {
                assert!(!stopped());
                (0..5).all(|_| permit(Step::Route, 4))
            });
            assert!(admitted && !safety);
            assert_eq!(past, 20);
            assert!(stopped() && !permit(Step::Route, 1));
        };
        let ((), report) = with(&StoppedClock, RunLimit::Work(10), None, run);
        assert_eq!((report.work, report.reason), (9, StopReason::Work));
        let ((), again) = with(&StoppedClock, RunLimit::Work(report.work), None, run);
        assert_eq!((again.work, again.reason), (9, StopReason::Work));
        let ((ran, past, safety), _) =
            with(&StoppedClock, RunLimit::Work(10), None, || past_limit(|| permit(Step::Route, 4)));
        assert!(ran && past == 0 && !safety);
        assert_eq!(consumed(), 0);
    }

    /// The safety limit stops steps past the limit, and the run is then reported stopped by it, not
    /// replayed by its work: how far those steps went hangs on the clock.
    #[test]
    fn the_safety_limit_stops_steps_past_the_limit() {
        let ((steps, safety), report) = with(&Ticks(Cell::new(0)), RunLimit::Time(2.0), Some(40.0), || {
            while permit(Step::Pivot, 1) {}
            let (steps, _, safety) = past_limit(|| {
                let mut steps = 0u64;
                while permit(Step::Pivot, 1) {
                    steps += 1;
                }
                steps
            });
            (steps, safety)
        });
        assert!(safety && steps > 0);
        assert_eq!(report.reason, StopReason::Safety);
        assert!(!report.reproducible());
        // a limit of work, with the steps past it done before the safety limit passes: the reason
        // stays the limit's
        let ((), report) = with(&Ticks(Cell::new(0)), RunLimit::Work(3), Some(1000.0), || {
            while permit(Step::Pivot, 1) {}
            let (_, _, safety) = past_limit(|| (0..3).all(|_| permit(Step::Pivot, 1)));
            assert!(!safety);
        });
        assert_eq!(report.reason, StopReason::Work);
        assert!(report.reproducible());
    }

    /// A solve limited by work whose steps past the limit the safety limit stops, as on a slow
    /// machine, is reported stopped by its safety limit, with no replay offered: those steps went as
    /// far as the clock let them. On a fast machine the same solve stops by its work and replays.
    #[test]
    fn a_solve_whose_steps_past_the_limit_the_safety_limit_stops_is_not_replayed_by_its_work() {
        use crate::resolve::tests::{instance, problem_of};
        use crate::{solve_with_clock, SolverOptions};
        use rnl_runtime::problem::PointRef;
        struct Steps(Cell<f64>, f64);
        impl Clock for Steps {
            fn elapsed(&self) -> f64 {
                let now = self.0.get();
                self.0.set(now + self.1);
                now
            }
        }
        let mut p = problem_of(vec![instance(), instance()], Vec::new());
        p.version = "0.4".into();
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
        let options = SolverOptions::default().with_config("limit=\"work\",work=0,safety_seconds=30").unwrap();
        // each reading of the clock a microsecond on: the safety limit is never reached
        let fast = solve_with_clock(&p, &options, &Steps(Cell::new(0.), 1e-6));
        let budget = fast.report.budget.unwrap();
        assert_eq!(budget.reason, StopReason::Work);
        assert!(budget.reproducible());
        assert!(fast.report.replay_config().is_some());
        // each reading 31 seconds on: the safety limit passes at the first step past the limit
        let slow = solve_with_clock(&p, &options, &Steps(Cell::new(0.), 31.));
        let budget = slow.report.budget.unwrap();
        assert_eq!(budget.reason, StopReason::Safety);
        assert!(!budget.reproducible());
        assert_eq!(slow.report.replay_config(), None);
        assert!(
            slow.report.notes.iter().any(|n| n.starts_with("budget:") && n.contains("not replayed")),
            "{:?}",
            slow.report.notes
        );
    }

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

    /// A clock that asks to be read only every so much work: it is read far less often than
    /// there are steps, the stop is still at a step, and the work it reports replays the run.
    /// So it is where a step costs more than the work between two readings, and where the work
    /// never comes to a reading and the clock is read at each 256th step.
    #[test]
    fn a_clock_read_every_so_much_work_still_replays_by_work() {
        struct Sparse(Cell<u64>, u64);
        impl Clock for Sparse {
            fn check_every(&self) -> u64 {
                self.1
            }
            fn elapsed(&self) -> f64 {
                let n = self.0.get();
                self.0.set(n + 1);
                n as f64
            }
        }
        for (every, scale) in [(100, 1), (100, 500), (u64::MAX, 1)] {
            let run = || {
                let mut admitted = Vec::new();
                for i in 0..2000u64 {
                    if !permit(Step::Route, scale * (1 + (i % 7) as usize)) {
                        break;
                    }
                    admitted.push(i);
                }
                admitted
            };
            let (all, _) = with(&StoppedClock, RunLimit::Unlimited, None, run);
            assert_eq!(all.len(), 2000);
            let mut stopped_within = false;
            for seconds in 0..12 {
                let clock = Sparse(Cell::new(0), every);
                let (timed, report) = with(&clock, RunLimit::Time(seconds as f64), None, run);
                // a reading when the budget starts and one at the first step; then one for each
                // `every` units of work and one for each 256 steps at most
                let most = 3 + report.work / every + timed.len() as u64 / 256;
                assert!(clock.0.get() <= most, "every {every}, scale {scale}: {} readings", clock.0.get());
                let (replayed, again) = with(&StoppedClock, RunLimit::Work(report.work), None, run);
                assert_eq!(timed, replayed, "every {every}, scale {scale}, limit {seconds}");
                assert_eq!(report.work, again.work);
                stopped_within |= report.reason == StopReason::Time && !timed.is_empty() && timed.len() < 2000;
            }
            assert!(stopped_within, "every {every}, scale {scale}: no time limit stopped the run part way");
        }
        // a step that costs more than the work between two readings reads the clock each time
        let clock = Sparse(Cell::new(0), 100);
        let (admitted, _) =
            with(&clock, RunLimit::Time(5.), None, || (0..50).take_while(|_| permit(Step::Route, 500)).count());
        assert_eq!(admitted, 4);
        // under a work limit the safety limit reads the clock as rarely, and changes nothing
        let run = || (0..2000).take_while(|_| permit(Step::Route, 3)).count();
        let clock = Sparse(Cell::new(0), 100);
        let (limited, report) = with(&clock, RunLimit::Work(1000), Some(1e9), run);
        let (plain, _) = with(&StoppedClock, RunLimit::Work(1000), None, run);
        assert_eq!(limited, plain);
        assert_eq!(report.reason, StopReason::Work);
        assert!(clock.0.get() <= 13, "{} readings", clock.0.get());
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

    /// A product of two counts is charged as it is made in `u64`, past what a 32-bit `usize`
    /// holds: 65,536 pins squared is 2^32, not `u32::MAX`.
    #[test]
    fn a_product_of_counts_is_charged_in_u64() {
        // the same on any target
        assert_eq!(product(65_536, 65_536), 1u64 << 32);
        // the largest counts saturate where u64 does: on a 64-bit target, but not on a 32-bit one,
        // whose largest count squared fits
        let max = usize::MAX as u64;
        let squared = if max > u64::from(u32::MAX) { u64::MAX } else { max * max };
        assert_eq!(product(usize::MAX, usize::MAX), squared);
        let weight = Step::Structure.weight();
        let big = product(65_536, 65_536);
        let ((), report) =
            with(&StoppedClock, RunLimit::Unlimited, None, || assert!(permit_size(Step::Structure, big)));
        assert_eq!(report.work, weight * (1u64 << 32));
        // a count charges as before
        let ((), report) = with(&StoppedClock, RunLimit::Unlimited, None, || assert!(permit(Step::Structure, 7)));
        assert_eq!(report.work, weight * 7);
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
            // with where the lines layout's group-tree start stopped, which its work does not tell
            let tree_work = SolverOptions::parse(timed.report.canonical_options.as_deref().unwrap()).unwrap().tree_work;
            let replay =
                SolverOptions { run_limit: Some(RunLimit::Work(work)), time_limit: None, tree_work, ..options };
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
        let (_, report) = with(&Nested, RunLimit::Work(20001), None, || {
            assert!(permit(Step::Pivot, 10_000));
            assert!(permit(Step::Route, 1));
        });
        assert_eq!(report.work, 20001);
        assert_eq!(report.reason, StopReason::Complete);
    }
}
