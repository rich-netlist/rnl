use super::*;
use crate::resolve::tests::{fact, instance, point, problem_of};
use crate::resolve::{Deadline, Tag};
use crate::simplex::{Failure, Rel};
use crate::StoppedClock;
use rnl_runtime::problem::{Axis, FactKind, PointRef};

fn structure() -> Structure {
    let problem = problem_of(vec![instance(), point(), point()], Vec::new());
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve(&problem, &options);
    crate::structure::analyse(&problem, &resolved, &options, crate::options::CAPS, &mut Work::default())
}

fn bound(kind: RelKind, axis: usize, terms: &[(usize, i64)], lo: Option<i64>, hi: Option<i64>) -> Tagged {
    Tagged {
        kind,
        rel: Relation::Linear {
            axis,
            terms: terms.iter().map(|(p, a)| (PointId::Origin(*p), int(*a))).collect(),
            lo: lo.map(int),
            hi: hi.map(int),
        },
    }
}

fn one_of(axis: usize, a: usize, b: usize, gap: i64) -> Tagged {
    Tagged {
        kind: RelKind::Separation,
        rel: Relation::OneOf(vec![
            vec![(axis, vec![(PointId::Origin(a), int(1)), (PointId::Origin(b), int(-1))], int(gap))],
            vec![(axis, vec![(PointId::Origin(b), int(1)), (PointId::Origin(a), int(-1))], int(gap))],
        ]),
    }
}

fn inspect(r: &mut Resolver, _: &Model, t: &Taken) -> (ResolverSnapshot, Vec<bool>, bool) {
    (r.snapshot().unwrap(), t.kept.clone(), t.undecided)
}

/// Comparing the entire state, rather than just a feasible point, also checks basis, rows,
/// rejected-unit residue, tags, trail, nogoods, exact pivot cost and each remaining axis budget.
fn compare(s: &Structure, relations: &[Tagged], cache: &mut RelationReuse) -> (Vec<bool>, bool) {
    let mut cold_work = Work::default();
    let cold = super::super::with_relations(s, relations, &mut cold_work, inspect);
    let mut warm_work = Work::default();
    let warm = cache.with_relations(s, relations, &mut warm_work, inspect);
    assert_eq!(cold, warm);
    assert_eq!(cold_work, warm_work);
    (warm.1, warm.2)
}

#[test]
fn complete_replay_and_changed_future_suffix_match_the_cold_state() {
    let s = structure();
    let mut cache = RelationReuse::default();
    let prefix = vec![
        Tagged { kind: RelKind::Orientation, rel: Relation::Orient { root: 0, value: Orient::MY } },
        one_of(0, 1, 0, 3),
        bound(RelKind::Row, 1, &[(1, 1), (0, -1)], Some(0), Some(0)),
        bound(RelKind::Gap, 0, &[(2, 1), (1, -1)], Some(2), None),
    ];
    let mut relations = prefix.clone();
    relations.push(bound(RelKind::RowsApart, 1, &[(2, 1), (0, -1)], Some(3), None));
    compare(&s, &relations, &mut cache);
    assert!(!cache.last.hit);
    compare(&s, &relations, &mut cache);
    assert!(cache.last.hit);
    assert_eq!(cache.last.reused_relations, relations.len());
    relations[4] = bound(RelKind::RowsApart, 1, &[(2, 1), (0, -1)], Some(5), None);
    compare(&s, &relations, &mut cache);
    assert!(cache.last.hit);
    assert_eq!(cache.last.reused_relations, prefix.len());
    assert!(cache.previous.as_ref().unwrap().prefixes.len() <= 3);
}

#[test]
fn every_attempted_unit_and_tag_is_checked_including_rejected_units() {
    let s = structure();
    let mut cache = RelationReuse::default();
    let mut relations = vec![
        bound(RelKind::Separation, 0, &[(1, 1), (0, -1)], Some(3), None),
        bound(RelKind::Separation, 0, &[(1, 1), (0, -1)], None, Some(1)),
        bound(RelKind::RowsApart, 1, &[(2, 1), (0, -1)], Some(3), None),
    ];
    let (kept, _) = compare(&s, &relations, &mut cache);
    assert_eq!(kept, [true, false, true]);
    compare(&s, &relations, &mut cache);
    assert!(cache.last.hit);
    // This rejected unit is absent from kept, but its attempted prefix may leave tableau state.
    relations[1] = bound(RelKind::Separation, 0, &[(1, 1), (0, -1)], None, Some(2));
    compare(&s, &relations, &mut cache);
    assert!(!cache.last.hit);
    assert_eq!(cache.last.miss_reason, "attempted_prefix");
    // The exact explanatory kind/order is part of identity, even if the bound is unchanged.
    relations[0].kind = RelKind::Gap;
    compare(&s, &relations, &mut cache);
    assert!(!cache.last.hit);
}

#[test]
fn consumer_objectives_grid_locks_and_new_variables_never_leak_into_the_saved_prefix() {
    let s = structure();
    let mut cache = RelationReuse::default();
    let relations = vec![one_of(0, 1, 0, 3), bound(RelKind::Row, 1, &[(1, 1), (0, -1)], Some(0), Some(0))];
    let consumer = |r: &mut Resolver, model: &Model, t: &Taken| {
        let before = r.snapshot().unwrap();
        let mut minima = Vec::new();
        for axis in 0..2 {
            let sx = r.simplex_mut(axis);
            let temporary = sx.new_var();
            sx.assert_bounds(temporary, Some(int(0)), None, Tag::Def).unwrap();
            let objective = LinExpr::from_terms([(temporary, int(1)), (model.var(PointId::Origin(2)), int(1))], int(0));
            sx.assert(&objective, Rel::Ge, &int(-7), Tag::Def).unwrap();
            minima.push(sx.minimize(&objective).unwrap());
            sx.keep_optimum(&objective, Tag::Def).unwrap();
            // A grid-like permanent equality, after the objective has changed the basis.
            let v = model.var(PointId::Origin(0));
            let value = sx.value(v).clone();
            sx.assert_bounds(v, Some(value.clone()), Some(value), Tag::Def).unwrap();
            sx.check().unwrap();
            sx.assert_invariants();
        }
        (before, minima, t.kept.clone(), r.snapshot().unwrap())
    };
    let cold = super::super::with_relations(&s, &relations, &mut Work::default(), consumer);
    let first = cache.with_relations(&s, &relations, &mut Work::default(), consumer);
    assert_eq!(first, cold);
    let saved = cache.previous.as_ref().unwrap().prefixes.last().unwrap().state.clone();
    assert_eq!(saved, cold.0);
    assert_ne!(saved, cold.3);
    let hit = cache.with_relations(&s, &relations, &mut Work::default(), consumer);
    assert_eq!(hit, cold);
    assert_eq!(cache.previous.as_ref().unwrap().prefixes.last().unwrap().state, saved);
}

#[test]
fn changed_definitions_numbering_seed_kept_and_caps_force_cold_replay() {
    let s = structure();
    let relations = vec![one_of(0, 1, 0, 3)];
    for change in 0..5 {
        let mut cache = RelationReuse::default();
        compare(&s, &relations, &mut cache);
        let mut changed = s.clone();
        match change {
            0 => {
                changed.model.tol += int(1);
            }
            1 => {
                changed.model.vars.swap(0, 1);
                changed.model.points.get_mut(&PointId::Origin(0)).unwrap().var = 1;
                changed.model.points.get_mut(&PointId::Origin(1)).unwrap().var = 0;
            }
            2 => {
                changed.witness.origin[2][0] = int(1);
            }
            3 => {
                changed.caps.nodes += 1;
            }
            _ => {
                changed.caps.pivots += 1;
            }
        }
        compare(&changed, &relations, &mut cache);
        assert!(!cache.last.hit, "change {change}");
    }
    // A changed future separator changes the complete preallocated VarKind mapping.
    let mut cache = RelationReuse::default();
    compare(&s, &relations, &mut cache);
    let mut extended = relations;
    extended.push(Tagged {
        kind: RelKind::RowsApart,
        rel: Relation::Apart {
            axis: 1,
            low: vec![(PointId::Origin(0), int(0)), (PointId::Origin(1), int(0))],
            high: vec![(PointId::Origin(2), int(0))],
            gap: int(2),
        },
    });
    compare(&s, &extended, &mut cache);
    assert!(!cache.last.hit);
    assert_eq!(cache.last.miss_reason, "model_definitions");
}

#[test]
fn nodes_are_fresh_per_search_and_pivot_budget_residue_matches_cold_for_each_axis() {
    let mut s = structure();
    let relations =
        vec![one_of(0, 1, 0, 3), one_of(1, 2, 0, 4), bound(RelKind::RowsApart, 0, &[(2, 1), (1, -1)], Some(2), None)];
    for nodes in [0, 1, 2, 8] {
        for pivots in [0, 1, 2, 3, 8] {
            s.caps = Caps { nodes, pivots };
            let mut cache = RelationReuse::default();
            compare(&s, &relations, &mut cache);
            compare(&s, &relations, &mut cache);
            assert!(cache.last.hit);
            let mut changed = relations.clone();
            changed[2] = bound(RelKind::RowsApart, 0, &[(2, 1), (1, -1)], Some(5), None);
            compare(&s, &changed, &mut cache);
            assert!(cache.last.hit);
        }
    }
}

#[test]
fn a_changed_suffix_must_search_again_with_a_fresh_local_node_budget() {
    let mut s = structure();
    for nodes in [1, 2, 8] {
        s.caps = Caps { nodes, pivots: 100 };
        let mut cache = RelationReuse::default();
        let mut relations = vec![one_of(0, 1, 0, 3), bound(RelKind::RowsApart, 0, &[(1, 1), (0, -1)], None, Some(5))];
        assert_eq!(compare(&s, &relations, &mut cache), (vec![true, true], false));
        // The old held alternative has x1-x0 >= 3. This suffix forces the other alternative,
        // so the quick check conflicts with a literal and a real DFS must change its value.
        relations[1] = bound(RelKind::RowsApart, 0, &[(1, 1), (0, -1)], None, Some(-3));
        let (kept, undecided) = compare(&s, &relations, &mut cache);
        assert!(cache.last.hit && cache.last.reused_relations == 1);
        assert!(cache.last.logical_prefix_nodes > 0);
        if nodes == 1 {
            assert_eq!(kept, [true, false]);
            assert!(undecided);
        } else {
            assert_eq!(kept, [true, true]);
            assert!(!undecided);
            let mut work = Work::default();
            let total_nodes = cache.with_relations(&s, &relations, &mut work, |r, _, _| r.nodes);
            // In particular nodes=2 succeeds even though the resolver's accumulated nodes
            // exceed that cap. A new Search spends its own nodes, not the resolver total.
            assert!(total_nodes >= 3);
            if nodes == 2 {
                assert!(total_nodes > nodes);
            }
            assert_eq!(work.resolve_nodes, total_nodes);
        }
    }
}

#[test]
fn undecided_prefix_is_preserved_and_cannot_be_matched_by_the_kept_subset() {
    let mut s = structure();
    s.caps = Caps { nodes: 0, pivots: 0 };
    let mut cache = RelationReuse::default();
    let mut relations = vec![one_of(0, 1, 0, 3), bound(RelKind::RowsApart, 1, &[(2, 1), (0, -1)], Some(3), None)];
    let (kept, undecided) = compare(&s, &relations, &mut cache);
    assert!(!kept[0] && undecided);
    compare(&s, &relations, &mut cache);
    assert!(cache.last.hit);
    relations[0] = one_of(0, 1, 0, 4);
    compare(&s, &relations, &mut cache);
    assert!(!cache.last.hit);
    assert_eq!(cache.last.miss_reason, "attempted_prefix");
}

#[test]
fn base_units_and_their_admission_order_are_part_of_the_identity() {
    let problem = problem_of(
        vec![point(), point(), point()],
        vec![
            fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 0.0 }),
            fact(FactKind::Place { at: PointRef::Origin(1), axis: Axis::X, value: 1.0 }),
        ],
    );
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve(&problem, &options);
    let s = crate::structure::analyse(&problem, &resolved, &options, crate::options::CAPS, &mut Work::default());
    let relations = vec![one_of(1, 2, 0, 3)];
    for change in 0..2 {
        let mut cache = RelationReuse::default();
        compare(&s, &relations, &mut cache);
        let mut changed = s.clone();
        if change == 0 {
            // Both are kept and independent, but insertion order and explanatory tags matter.
            changed.kept.reverse();
        } else {
            changed.model.units[0].fixed[0].hi = Some(int(1));
        }
        compare(&changed, &relations, &mut cache);
        assert!(!cache.last.hit);
        assert_eq!(cache.last.miss_reason, if change == 0 { "context" } else { "base_units" });
    }
}

#[test]
fn time_resolutions_cannot_be_snapshotted_and_restored_caps_are_not_reset() {
    let s = structure();
    let deadline = Deadline::new(&StoppedClock, Some(0.0));
    let timed = Resolver::new(&s.model, s.objects.len(), Limits::Time(&deadline));
    assert!(timed.snapshot().is_none());
    let mut r = Resolver::new(&s.model, s.objects.len(), Limits::Caps(Caps { nodes: 1, pivots: 0 }));
    let saved = r.snapshot().unwrap();
    let mut restored = saved.restore(&s.model);
    for resolver in [&mut r, &mut restored] {
        let sx = resolver.simplex_mut(0);
        let a = s.model.var(PointId::Origin(0));
        let b = s.model.var(PointId::Origin(1));
        let objective = LinExpr::diff(a, b);
        sx.assert(&objective, Rel::Ge, &int(3), Tag::Def).unwrap();
        assert_eq!(sx.check(), Err(Failure::Budget));
    }
    assert_eq!(r.snapshot(), restored.snapshot());
}

#[test]
fn a_lightweight_observer_does_not_enable_profiling_or_read_the_deadline_clock() {
    use crate::profile::{with_state_reuse_observer, StateReuseObserver};
    use crate::resolve::tests::Ticks;
    use crate::Clock;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    #[derive(Default)]
    struct Observer {
        reads: Cell<u64>,
        reports: RefCell<Vec<StateReuseReport>>,
    }
    impl Clock for Observer {
        fn elapsed(&self) -> f64 {
            self.reads.set(self.reads.get() + 1);
            self.reads.get() as f64
        }
    }
    impl StateReuseObserver for Observer {
        fn record(&self, report: StateReuseReport) {
            self.reports.borrow_mut().push(report);
        }
    }
    let problem = problem_of(
        vec![instance(), point(), point()],
        vec![fact(FactKind::Align { axis: Axis::Y, members: vec![PointRef::Origin(0), PointRef::Origin(1)] })],
    );
    let options = SolverOptions::default();
    let plain_clock = Ticks::default();
    let plain = crate::solve_with_clock(&problem, &options, &plain_clock);
    let observed_clock = Ticks::default();
    let observer = Rc::new(Observer::default());
    let observed = with_state_reuse_observer(Some(observer.clone()), || {
        assert!(!crate::profile::enabled());
        crate::solve_with_clock(&problem, &options, &observed_clock)
    });
    assert_eq!(plain, observed);
    assert_eq!(plain_clock.reads(), observed_clock.reads());
    assert!(observer.reads.get() > 0 && !observer.reports.borrow().is_empty());
    assert!(crate::profile::state_reuse_observer().is_none());
    let hash = rnl_runtime::problem::sig2(&problem, &options.canonical());
    let a = rnl_runtime::solution::write_record(&problem, &plain.solution, crate::AUTHOR, &hash);
    let b = rnl_runtime::solution::write_record(&problem, &observed.solution, crate::AUTHOR, &hash);
    assert_eq!(a.record.as_bytes(), b.record.as_bytes());
}
