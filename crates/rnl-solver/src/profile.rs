//! Optional observations of a solve. The caller owns the clock and any output.
//!
//! The scope is local to the current thread (also on single-threaded wasm). No solver result,
//! option, budget or deadline depends on these events. A disabled observer reads no extra clock.
use std::cell::RefCell;
use std::rc::Rc;

use crate::Clock;

pub(crate) mod work;

/// A caller-owned observer. Timing must use the existing `Clock` interface. Implementations
/// should accumulate counters in memory and throttle durable checkpoints.
pub trait ProfileObserver: Clock {
    fn record(&self, event: ProfileEvent);

    /// Explicit work diagnostics; ordinary profiling keeps its existing events and scans.
    fn work_diagnostics(&self) -> bool {
        false
    }

    /// Work diagnostics skip the expensive stored-rational scan, rather than reporting zeros.
    fn tableau_details(&self) -> bool {
        !self.work_diagnostics()
    }
}

/// A shared observer supplied by `Clock::profiler`.
pub type ProfileHandle = Rc<dyn ProfileObserver>;

/// Observations, separate from the solution and the canonical solver options.
#[derive(Clone, Debug)]
pub enum ProfileEvent {
    Level {
        level: usize,
        title: String,
        objects: usize,
        facts: usize,
        routes: usize,
        options: String,
        hash: String,
    },
    LevelEnd,
    Begin {
        category: &'static str,
        name: String,
    },
    End,
    Add {
        name: &'static str,
        value: u64,
    },
    Max {
        name: &'static str,
        value: u64,
    },
    Set {
        name: &'static str,
        value: u64,
    },
    Text {
        name: &'static str,
        value: String,
    },
    /// One batch at a coarse boundary. Gauges are current state, not cumulative work.
    Work {
        counters: Vec<(&'static str, u64)>,
        maxima: Vec<(&'static str, u64)>,
        gauges: Vec<(&'static str, u64)>,
    },
    Checkpoint,
}

thread_local! {
    static OBSERVER: RefCell<Option<ProfileHandle>> = const { RefCell::new(None) };
}

pub(crate) fn enabled() -> bool {
    OBSERVER.with(|o| o.borrow().is_some())
}

pub(crate) fn work_enabled() -> bool {
    OBSERVER.with(|o| o.borrow().as_ref().is_some_and(|o| o.work_diagnostics()))
}

pub(crate) fn tableau_details_enabled() -> bool {
    OBSERVER.with(|o| o.borrow().as_ref().is_some_and(|o| o.tableau_details()))
}

pub(crate) fn emit(event: ProfileEvent) {
    if matches!(event, ProfileEvent::Begin { .. } | ProfileEvent::End | ProfileEvent::LevelEnd) {
        work::flush();
    }
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

pub(crate) fn work_span(category: &'static str, name: &'static str) -> Span {
    if work_enabled() {
        span(category, name)
    } else {
        Span(false)
    }
}

pub(crate) fn work_set(name: &'static str, value: u64) {
    if work_enabled() {
        set(name, value);
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
    let _work = work::Scope::new(work_enabled());
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
    struct Observer(RefCell<Vec<ProfileEvent>>, bool);
    impl Clock for Observer {
        fn elapsed(&self) -> f64 {
            0.0
        }
    }
    impl ProfileObserver for Observer {
        fn record(&self, e: ProfileEvent) {
            self.0.borrow_mut().push(e);
        }
        fn work_diagnostics(&self) -> bool {
            self.1
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
            let work_observer = Rc::new(Observer(RefCell::new(Vec::new()), true));
            let work = Ticks { reads: Cell::new(0), observer: Some(work_observer.clone()) };
            let c = crate::solve_with_clock(&p, &options, &work);
            assert_eq!(a, b, "limit {limit:?}");
            assert_eq!(a, c, "work mode at limit {limit:?}");
            assert_eq!(plain.reads.get(), measured.reads.get(), "deadline reads at {limit:?}");
            assert_eq!(plain.reads.get(), work.reads.get(), "work deadline reads at {limit:?}");
            let hash = rnl_runtime::problem::sig2(&p, &options.canonical());
            let ra = rnl_runtime::solution::write_record(&p, &a.solution, crate::AUTHOR, &hash);
            let rb = rnl_runtime::solution::write_record(&p, &b.solution, crate::AUTHOR, &hash);
            let rc = rnl_runtime::solution::write_record(&p, &c.solution, crate::AUTHOR, &hash);
            assert_eq!(ra.record.as_bytes(), rb.record.as_bytes());
            assert_eq!(ra.record.as_bytes(), rc.record.as_bytes());
            let events = observer.0.borrow();
            assert!(matches!(events.first(), Some(ProfileEvent::Level { hash: h, .. }) if h == &hash));
            assert!(matches!(events.last(), Some(ProfileEvent::LevelEnd)));
            assert!(!events.iter().any(|e| matches!(e, ProfileEvent::Work { .. })));
            let work_events = work_observer.0.borrow();
            assert!(work_events.iter().any(|e| matches!(e, ProfileEvent::Work { .. })));
            assert!(!work_events.iter().any(|e| matches!(
                e,
                ProfileEvent::Add { name: "tableau_scans", .. }
                    | ProfileEvent::Max { name: "max_numerator_bits" | "max_denominator_bits", .. }
            )));
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
