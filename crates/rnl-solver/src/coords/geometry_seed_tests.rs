//! Semantic checks for a fresh placement resolution with a previous geometry, not an LP cache.
use super::*;
use crate::options::{Caps, CAPS};
use crate::profile::{self, ProfileEvent, ProfileObserver};
use crate::resolve::tests::{fact, instance, point, problem_of, Ticks};
use crate::resolve::{Limits, Resolver, Tag};
use crate::search::Taken;
use rnl_runtime::problem::{Axis, FactKind, PointRef, ProblemFact, ProblemObject};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct Observer(RefCell<Vec<ProfileEvent>>);
impl crate::Clock for Observer {
    fn elapsed(&self) -> f64 {
        panic!("geometry seed observation must not read the deadline clock");
    }
}
impl ProfileObserver for Observer {
    fn record(&self, event: ProfileEvent) {
        self.0.borrow_mut().push(event);
    }
    fn work_diagnostics(&self) -> bool {
        true
    }
}
impl Observer {
    fn sum(&self, key: &str) -> u64 {
        self.0
            .borrow()
            .iter()
            .filter_map(|e| match e {
                ProfileEvent::Add { name, value } if *name == key => Some(*value),
                _ => None,
            })
            .sum()
    }
    fn calls(&self) -> usize {
        self.0.borrow().iter().filter(|e| matches!(e, ProfileEvent::Begin { category: "with_relations", .. })).count()
    }
}

fn fixture(
    objects: Vec<ProblemObject>,
    facts: Vec<ProblemFact>,
    caps: Caps,
) -> (LevelProblem, SolverOptions, Structure) {
    let p = problem_of(objects, facts);
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve(&p, &options);
    let s = crate::structure::analyse(&p, &resolved, &options, caps, &mut Work::default());
    (p, options, s)
}

#[derive(Debug, PartialEq)]
struct Round {
    witness: Geometry,
    geometry: Geometry,
    kept: Vec<bool>,
    undecided: bool,
    optimised: bool,
    seed_complete: bool,
    placed_layers: Vec<Q>,
}

fn run_round(
    p: &LevelProblem,
    s: &Structure,
    options: &SolverOptions,
    rels: &[Tagged],
    seed: Option<&Geometry>,
) -> Round {
    let consume = |r: &mut Resolver, model: &Model, t: &Taken| {
        let witness = r.geometry().clone();
        let (geometry, optimised, seed_complete) = optimise(r, model, s, options);
        let origins = s.origins(p, &geometry);
        Round {
            placed_layers: crate::score::layers(s, &origins, &geometry, None),
            witness,
            geometry,
            kept: t.kept.clone(),
            undecided: t.undecided,
            optimised,
            seed_complete,
        }
    };
    match seed {
        Some(seed) => with_placement_seed(s, rels, &mut Work::default(), Some(seed), &s.witness.orient, consume),
        // The original entry point has no seed proposal or geometry gate.
        None => with_relations(s, rels, &mut Work::default(), "placement", consume),
    }
}

fn linear(kind: RelKind, axis: usize, lo: Option<Q>, hi: Option<Q>) -> Tagged {
    Tagged { kind, rel: Relation::Linear { axis, terms: vec![(PointId::Origin(0), int(1))], lo, hi } }
}

fn assert_facts(s: &Structure, geometry: &Geometry) {
    let mut check = Resolver::new(&s.model, s.objects.len(), Limits::Caps(s.caps));
    check.seed(geometry.clone());
    assert!(s.kept.iter().all(|&f| check.holds(f)));
}

#[test]
fn geometry_seed_checks_shape_orientation_and_every_kept_fact_before_add() {
    let (_, _, s) = fixture(
        vec![instance(), point()],
        vec![
            fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 4.0 }),
            fact(FactKind::Orient { object: 0, code: Orient::MY }),
            fact(FactKind::Place { at: PointRef::Origin(1), axis: Axis::Y, value: 2.5 }),
        ],
        CAPS,
    );
    assert_eq!(s.kept, vec![0, 1, 2]);
    let mut valid = s.witness.clone();
    valid.origin[0][1] = int(7);
    let mut shape = valid.clone();
    shape.origin.pop();
    let mut orient = valid.clone();
    orient.orient[0] = Orient::R0;
    let mut last_fact = valid.clone();
    last_fact.origin[1][1] = int(9);
    for (seed, reason, expected) in [
        (&valid, "coords_seed_accepted", &valid),
        (&shape, "coords_seed_rejected_shape", &s.witness),
        (&orient, "coords_seed_rejected_orientation", &s.witness),
        (&last_fact, "coords_seed_rejected_kept", &s.witness),
    ] {
        let observer = Rc::new(Observer::default());
        let geometry = profile::with(Some(observer.clone()), || {
            with_placement_seed(&s, &[], &mut Work::default(), Some(seed), &s.witness.orient, |r, _, _| {
                r.geometry().clone()
            })
        });
        assert_eq!(&geometry, expected, "{reason}");
        assert_eq!(observer.sum("coords_seed_proposed"), 1);
        assert_eq!(observer.sum(reason), 1);
        assert_eq!(observer.sum("fact_add_calls"), s.kept.len() as u64);
        assert_eq!(observer.calls(), 1, "no cold resolution rerun");
        let events = observer.0.borrow();
        let gate = events.iter().position(|e| matches!(e, ProfileEvent::Add { name, .. } if *name == reason)).unwrap();
        let first_add =
            events.iter().position(|e| matches!(e, ProfileEvent::Add { name: "fact_add_calls", .. })).unwrap();
        assert!(gate < first_add);
    }
}

#[test]
fn geometry_seed_rechecks_new_separation_and_replaced_suffix_without_old_lp_or_held_bounds() {
    let (p, options, s) = fixture(vec![point()], Vec::new(), CAPS);
    let old = vec![
        linear(RelKind::RowsApart, 0, Some(int(10)), None),
        Tagged {
            kind: RelKind::Separation,
            rel: Relation::OneOf(vec![
                vec![(1, vec![(PointId::Origin(0), int(1))], int(4))],
                vec![(1, vec![(PointId::Origin(0), int(-1))], int(4))],
            ]),
        },
    ];
    // Complete the first consumer, then leave an extra LP-only bound behind. Neither that
    // bound, the grid pins nor the old disjunction's held literal may survive the fresh call.
    let previous = with_relations(&s, &old, &mut Work::default(), "placement", |r, model, t| {
        assert_eq!(t.kept, vec![true, true]);
        let (geometry, optimised, complete) = optimise(r, model, &s, &options);
        assert!(optimised && complete);
        let sx = r.simplex_mut(0);
        let x = model.var(PointId::Origin(0));
        let cp = sx.checkpoint();
        sx.backtrack(0);
        sx.assert(&LinExpr::var(x), Rel::Eq, &int(100), Tag::Def).unwrap();
        sx.check().unwrap();
        assert!(cp > 0);
        geometry
    });
    assert!(previous.origin[0][0] >= int(10));
    assert!(previous.origin[0][1] >= int(4));
    let current = vec![
        linear(RelKind::Separation, 0, None, Some(int(-3))),
        linear(RelKind::RowsApart, 1, None, Some(int(-6))),
        linear(RelKind::RowsApart, 0, Some(int(10)), None),
    ];
    let cold = run_round(&p, &s, &options, &current, None);
    let warm = run_round(&p, &s, &options, &current, Some(&previous));
    for round in [&cold, &warm] {
        assert_eq!(round.kept, vec![true, true, false], "new higher-priority separation wins");
        assert!(!round.undecided && round.optimised && round.seed_complete);
        assert!(round.geometry.origin[0][0] <= int(-3));
        assert!(round.geometry.origin[0][1] <= int(-6));
        assert_facts(&s, &round.geometry);
    }
    assert_eq!(cold.placed_layers, warm.placed_layers, "complete placed estimate vector on this fixture");
    let replaced = vec![linear(RelKind::RowsApart, 0, Some(int(8)), Some(int(8)))];
    let after = run_round(&p, &s, &options, &replaced, Some(&warm.geometry));
    assert_eq!(after.kept, vec![true]);
    assert_eq!(after.geometry.origin[0][0], int(8), "removed old suffix has no bound left");
    assert_facts(&s, &after.geometry);
}

#[test]
fn geometry_seed_excludes_incomplete_layers_axis_cap_and_undecided_without_retry() {
    let (p, options, s) = fixture(vec![point()], Vec::new(), Caps { pivots: 0, ..CAPS });
    let mut proposed = s.witness.clone();
    proposed.origin[0][0] = int(10);
    let round = run_round(&p, &s, &options, &[], Some(&proposed));
    // The original optimiser may still return a feasible optimised result after a layer
    // stopped. This public flag's meaning is unchanged; seed eligibility is stricter.
    assert!(round.optimised);
    assert!(!round.seed_complete);
    assert!(subsequent_seed(round.geometry, round.optimised, false, round.seed_complete).is_none());

    let (p, options, s) = fixture(
        vec![point()],
        vec![fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 0.0 })],
        Caps { nodes: 0, ..CAPS },
    );
    let impossible = vec![Tagged {
        kind: RelKind::Separation,
        rel: Relation::OneOf(vec![
            vec![(0, vec![(PointId::Origin(0), int(1))], int(1))],
            vec![(0, vec![(PointId::Origin(0), int(-1))], int(1))],
        ]),
    }];
    let observer = Rc::new(Observer::default());
    let round = profile::with(Some(observer.clone()), || run_round(&p, &s, &options, &impossible, Some(&s.witness)));
    assert!(round.undecided);
    assert_eq!(round.kept, vec![false]);
    assert_eq!(observer.calls(), 1);
    assert_eq!(observer.sum("coords_seed_accepted"), 1);
    assert_facts(&s, &round.geometry);
    assert!(subsequent_seed(round.geometry, round.optimised, round.undecided, round.seed_complete).is_none());
    let good = s.witness.clone();
    assert!(subsequent_seed(good.clone(), false, false, true).is_none());
    assert!(subsequent_seed(good.clone(), true, false, false).is_none());
    assert_eq!(subsequent_seed(good.clone(), true, false, true), Some(good));
}

#[test]
fn geometry_seed_full_solve_repeats_exact_scores_and_preserves_fact_clock_and_fallback_notes() {
    let p = problem_of(
        vec![instance(), instance()],
        (0..2)
            .flat_map(|k| {
                [
                    fact(FactKind::Place { at: PointRef::Origin(k), axis: Axis::X, value: 0.0 }),
                    fact(FactKind::Place { at: PointRef::Origin(k), axis: Axis::Y, value: 0.0 }),
                    fact(FactKind::Orient { object: k, code: Orient::R0 }),
                ]
            })
            .collect(),
    );
    let options = SolverOptions { time_limit: Some(1000.0), ..SolverOptions::default() };
    let reference_clock = Ticks::default();
    let resolved = crate::resolve::resolve(&p, &options, &reference_clock);
    let observer = Rc::new(Observer::default());
    let clock_a = Ticks::default();
    let (a, exact_a) =
        profile::with(Some(observer.clone()), || crate::driver::solve_exact_with(&p, &options, &clock_a, CAPS));
    let clock_b = Ticks::default();
    let (b, exact_b) = crate::driver::solve_exact_with(&p, &options, &clock_b, CAPS);
    assert_eq!(a, b);
    assert_eq!(exact_a.score, exact_b.score);
    assert_eq!(exact_a.placed.geometry, exact_b.placed.geometry);
    assert_eq!(exact_a.resolved.kept, resolved.kept);
    assert_eq!(exact_a.resolved.discarded, resolved.discarded);
    assert_eq!(clock_a.reads(), reference_clock.reads());
    assert_eq!(clock_b.reads(), reference_clock.reads());
    assert!(observer.sum("coords_seed_proposed") > 0, "forced overlap enters a later placement round");
    assert!(observer.sum("coords_seed_accepted") > 0);
    assert_facts(&exact_a.structure, &exact_a.placed.geometry);
    assert!(!exact_a.placed.overlaps.is_empty(), "fixed facts are not relaxed to improve the drawing");
    for origin in &exact_a.placed.geometry.origin {
        assert_eq!(origin, &[int(0), int(0)]);
    }
}
