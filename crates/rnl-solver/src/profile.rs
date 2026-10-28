//! Optional observations of a solve. The caller owns the clock and any output.
//!
//! The scope is local to the current thread (also on single-threaded wasm). No solver result,
//! option, budget or deadline depends on these events. A disabled observer reads no extra clock.
use std::cell::RefCell;
use std::rc::Rc;

use crate::Clock;

/// A caller-owned observer. Timing must use the existing `Clock` interface. Implementations
/// should accumulate counters in memory and throttle durable checkpoints.
pub trait ProfileObserver: Clock {
    fn record(&self, event: ProfileEvent);
}

/// A shared observer supplied by `Clock::profiler`.
pub type ProfileHandle = Rc<dyn ProfileObserver>;

/// A small, separate observer for the feasibility-reuse prototype. Installing it does not
/// enable tableau scans or the full profiler. The caller supplies its observational clock and
/// output; neither is the resolution's deadline clock.
pub trait StateReuseObserver: Clock {
    fn record(&self, report: StateReuseReport);
}

pub type StateReuseHandle = Rc<dyn StateReuseObserver>;

#[derive(Clone, Debug, Default)]
pub struct StateReuseReport {
    pub hit: bool,
    pub miss_reason: &'static str,
    pub reused_relations: usize,
    pub total_relations: usize,
    pub save_clones: u64,
    pub save_ns: u64,
    pub restore_ns: u64,
    pub model_clone_ns: u64,
    pub logical_prefix_nodes: u64,
    pub logical_prefix_pivots: u64,
}

/// Observations, separate from the solution and the canonical solver options.
#[derive(Clone, Debug)]
pub enum ProfileEvent {
    Level { level: usize, title: String, objects: usize, facts: usize, routes: usize, options: String, hash: String },
    LevelEnd,
    Begin { category: &'static str, name: String },
    End,
    Add { name: &'static str, value: u64 },
    Max { name: &'static str, value: u64 },
    Set { name: &'static str, value: u64 },
    Text { name: &'static str, value: String },
    Checkpoint,
}

thread_local! {
    static OBSERVER: RefCell<Option<ProfileHandle>> = const { RefCell::new(None) };
    static STATE_REUSE_OBSERVER: RefCell<Option<StateReuseHandle>> = const { RefCell::new(None) };
}

pub(crate) fn state_reuse_observer() -> Option<StateReuseHandle> {
    STATE_REUSE_OBSERVER.with(|o| o.borrow().clone())
}

/// Run with an optional lightweight reuse observer. Restores the previous caller's observer
/// even on unwinding, just as the full profiler does.
pub fn with_state_reuse_observer<R>(observer: Option<StateReuseHandle>, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<StateReuseHandle>);
    impl Drop for Restore {
        fn drop(&mut self) {
            STATE_REUSE_OBSERVER.with(|o| *o.borrow_mut() = self.0.take());
        }
    }
    let previous = STATE_REUSE_OBSERVER.with(|o| o.replace(observer));
    let _restore = Restore(previous);
    f()
}

pub(crate) fn enabled() -> bool {
    OBSERVER.with(|o| o.borrow().is_some())
}

pub(crate) fn emit(event: ProfileEvent) {
    let observer = OBSERVER.with(|o| o.borrow().clone());
    if let Some(observer) = observer {
        observer.record(event);
    }
}

pub(crate) fn add(name: &'static str, value: u64) {
    if enabled() {
        emit(ProfileEvent::Add { name, value });
    }
}

pub(crate) fn max(name: &'static str, value: u64) {
    if enabled() {
        emit(ProfileEvent::Max { name, value });
    }
}

pub(crate) fn set(name: &'static str, value: u64) {
    if enabled() {
        emit(ProfileEvent::Set { name, value });
    }
}

pub(crate) fn checkpoint() {
    if enabled() {
        emit(ProfileEvent::Checkpoint);
    }
}

pub(crate) struct Span(bool);

pub(crate) fn span(category: &'static str, name: impl Into<String>) -> Span {
    let active = enabled();
    if active {
        emit(ProfileEvent::Begin { category, name: name.into() });
    }
    Span(active)
}

pub(crate) fn span_lazy(category: &'static str, name: impl FnOnce() -> String) -> Span {
    if enabled() {
        span(category, name())
    } else {
        Span(false)
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if self.0 {
            emit(ProfileEvent::End);
        }
    }
}

pub(crate) fn with<R>(observer: Option<ProfileHandle>, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<ProfileHandle>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OBSERVER.with(|o| *o.borrow_mut() = self.0.take());
        }
    }
    let previous = OBSERVER.with(|o| o.replace(observer));
    let _restore = Restore(previous);
    f()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{fact, instance, point, problem_of};
    use crate::{Clock, SolverOptions};
    use rnl_runtime::problem::{Axis, FactKind, PointRef};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    #[derive(Default)]
    struct Observer(RefCell<Vec<ProfileEvent>>);
    impl Clock for Observer {
        fn elapsed(&self) -> f64 {
            0.0
        }
    }
    impl ProfileObserver for Observer {
        fn record(&self, e: ProfileEvent) {
            self.0.borrow_mut().push(e);
        }
    }
    struct Ticks {
        reads: Cell<u64>,
        observer: Option<ProfileHandle>,
    }
    impl Clock for Ticks {
        fn elapsed(&self) -> f64 {
            let n = self.reads.get();
            self.reads.set(n + 1);
            n as f64
        }
        fn profiler(&self) -> Option<ProfileHandle> {
            self.observer.clone()
        }
    }

    #[test]
    fn profiling_preserves_results_hashes_and_deadline_read_sequence() {
        let p = problem_of(
            vec![instance(), point()],
            vec![
                fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 4.0 }),
                fact(FactKind::Align { axis: Axis::Y, members: vec![PointRef::Origin(1), PointRef::Origin(0)] }),
                fact(FactKind::Place { at: PointRef::Origin(1), axis: Axis::Y, value: 2.5 }),
                fact(FactKind::Place { at: PointRef::Origin(1), axis: Axis::Y, value: 9.0 }),
            ],
        );
        for limit in [None, Some(0.0), Some(2.0), Some(8.0), Some(1000.0)] {
            let options = SolverOptions { time_limit: limit, ..SolverOptions::default() };
            let plain = Ticks { reads: Cell::new(0), observer: None };
            let observer = Rc::new(Observer::default());
            let measured = Ticks { reads: Cell::new(0), observer: Some(observer.clone()) };
            let a = crate::solve_with_clock(&p, &options, &plain);
            let b = crate::solve_with_clock(&p, &options, &measured);
            assert_eq!(a, b, "limit {limit:?}");
            assert_eq!(plain.reads.get(), measured.reads.get(), "deadline reads at {limit:?}");
            let hash = rnl_runtime::problem::sig2(&p, &options.canonical());
            let ra = rnl_runtime::solution::write_record(&p, &a.solution, crate::AUTHOR, &hash);
            let rb = rnl_runtime::solution::write_record(&p, &b.solution, crate::AUTHOR, &hash);
            assert_eq!(ra.record.as_bytes(), rb.record.as_bytes());
            let events = observer.0.borrow();
            assert!(matches!(events.first(), Some(ProfileEvent::Level { hash: h, .. }) if h == &hash));
            assert!(matches!(events.last(), Some(ProfileEvent::LevelEnd)));
            assert!(!enabled());
        }
    }

    #[test]
    fn observer_scope_is_restored_after_unwinding() {
        let observer = Rc::new(Observer::default());
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| with(Some(observer), || panic!("test"))));
        assert!(!enabled());
    }
    #[test]
    fn nested_scopes_restore_observers_and_threads_are_isolated() {
        let outer = Rc::new(Observer::default());
        let inner = Rc::new(Observer::default());
        with(Some(outer.clone()), || {
            add("outer", 1);
            with(None, || {
                assert!(!enabled());
                add("disabled", 1);
            });
            assert!(enabled());
            with(Some(inner.clone()), || add("inner", 1));
            std::thread::spawn(|| {
                assert!(!enabled());
                add("other_thread", 1);
            })
            .join()
            .unwrap();
            add("outer", 1);
        });
        assert!(!enabled());
        assert_eq!(outer.0.borrow().len(), 2);
        assert_eq!(inner.0.borrow().len(), 1);
    }
}
