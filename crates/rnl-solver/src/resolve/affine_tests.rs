//! Small semantic oracles for the placement-only numerical boundary. Their answers follow
//! from the written one/two-variable constraints, independently of affine elimination.

use super::tests::{fact, instance, point, problem_of};
use super::*;
use crate::model::{Choice, ChoiceKind};
use crate::rational::int;
use rnl_runtime::problem::Axis as ProblemAxis;

fn unit(fixed: Vec<Bound>, choices: Vec<Choice>, x: Vec<PointId>) -> crate::model::Unit {
    crate::model::Unit {
        fixed,
        choices,
        orient: OrientEffect::None,
        touched: [x, Vec::new()],
        centres: [Vec::new(), Vec::new()],
    }
}

fn binary(unit: usize, v: Var) -> Choice {
    Choice {
        key: LitKey::Side(unit),
        kind: ChoiceKind::Ranked,
        values: [0, 1].into_iter().map(|n| vec![Bound::equal(0, LinExpr::var(v), int(n))]).collect(),
    }
}

fn binary_fixture() -> Model {
    let p = problem_of(vec![point(), point()], Vec::new());
    let mut model = Model::new(&p, int(0));
    let (x, y) = (model.var(PointId::Origin(0)), model.var(PointId::Origin(1)));
    model.units = vec![
        unit(
            vec![Bound::equal(0, LinExpr::diff(y, x), int(0))],
            Vec::new(),
            vec![PointId::Origin(0), PointId::Origin(1)],
        ),
        unit(Vec::new(), vec![binary(1, x)], vec![PointId::Origin(0)]),
        unit(vec![Bound::equal(0, LinExpr::var(x), int(1))], Vec::new(), vec![PointId::Origin(0)]),
        unit(vec![Bound::equal(0, LinExpr::var(x), int(2))], Vec::new(), vec![PointId::Origin(0)]),
    ];
    model
}

#[test]
fn a_frozen_core_leaves_an_old_choice_retractable_after_restore() {
    let model = binary_fixture();
    // Hand oracle: y=x, x in {0,1}. Adding x=2 is impossible; adding x=1 has exactly
    // x=y=1. Run the original representation as a second reference, not as the oracle.
    for frozen in [false, true] {
        let mut r = Resolver::new(&model, 2, Limits::Caps(CAPS));
        assert_eq!(r.add(0), Outcome::Kept);
        assert_eq!(r.add(1), Outcome::Kept);
        assert_eq!(r.held[&Held::Choice(1, 0)], 0);
        if frozen {
            r.freeze_placement_core();
        }
        let Outcome::Conflict(why) = r.add(3) else { panic!("2 is neither binary choice") };
        assert!(why.contains(&1));
        assert!(why.iter().all(|f| [0, 1].contains(f)));
        assert_eq!(r.kept, vec![0, 1]);
        assert_eq!(r.geometry.origin[0][0], int(0));
        assert_eq!(r.held[&Held::Choice(1, 0)], 0);
        assert_eq!(r.add(2), Outcome::Kept, "the old choice must leave the new base");
        assert_eq!(r.kept, vec![0, 1, 2]);
        assert_eq!(r.held[&Held::Choice(1, 0)], 1);
        assert_eq!(r.geometry.origin[0][0], int(1));
        assert_eq!(r.geometry.origin[1][0], int(1));
    }
}

#[test]
fn learned_nogoods_keep_original_choices_and_core_facts() {
    let p = problem_of(vec![point(), point(), point()], Vec::new());
    let mut model = Model::new(&p, int(0));
    let [x, y, z] = [0, 1, 2].map(|k| model.var(PointId::Origin(k)));
    model.units = vec![
        unit(
            vec![Bound::equal(0, LinExpr::diff(z, y), int(0))],
            Vec::new(),
            vec![PointId::Origin(1), PointId::Origin(2)],
        ),
        unit(Vec::new(), vec![binary(1, x)], vec![PointId::Origin(0)]),
        unit(Vec::new(), vec![binary(2, y)], vec![PointId::Origin(1)]),
        unit(
            vec![Bound::equal(0, LinExpr::from_terms([(x, int(1)), (z, int(1))], int(0)), int(2))],
            Vec::new(),
            vec![PointId::Origin(0), PointId::Origin(2)],
        ),
    ];
    let mut r = Resolver::new(&model, 3, Limits::Caps(CAPS));
    for u in 0..3 {
        assert_eq!(r.add(u), Outcome::Kept);
    }
    r.freeze_placement_core();
    assert_eq!(r.add(3), Outcome::Kept);
    // x,y in {0,1}, z=y and x+z=2 force all three to 1. In particular the learned
    // x=0 branch is impossible in the original facts, irrespective of reduced Var IDs.
    assert!(r.geometry.origin.iter().all(|xy| xy[0] == int(1)));
    assert!(!r.nogoods.is_empty(), "the failed inner branch should leave a reusable nogood");
    for ng in &r.nogoods {
        assert!(!ng.lits.is_empty());
        assert!(ng.lits.iter().all(|t| matches!(t, Tag::Lit(LitKey::Side(1 | 2), 0 | 1))));
        assert!(ng.facts.iter().all(|t| matches!(t, Tag::Fact(0..=3))));
        assert!(ng.facts.contains(&Tag::Fact(0)), "the eliminated z=y fact remains a source");
    }
}

#[test]
fn check_conflicts_include_the_eliminated_original_equation() {
    let p = problem_of(vec![point(), point(), point()], Vec::new());
    let mut model = Model::new(&p, int(0));
    let [x, y, z] = [0, 1, 2].map(|k| model.var(PointId::Origin(k)));
    model.units.push(unit(
        vec![Bound::equal(0, LinExpr::diff(y, x), int(0))],
        Vec::new(),
        vec![PointId::Origin(0), PointId::Origin(1)],
    ));
    let mut r = Resolver::new(&model, 3, Limits::Caps(CAPS));
    assert_eq!(r.add(0), Outcome::Kept);
    r.freeze_placement_core();
    let bounds = [
        Bound::at_least(0, LinExpr::var(x), int(1)),
        Bound::at_least(0, LinExpr::var(z), int(0)),
        Bound::at_most(0, LinExpr::from_terms([(y, int(1)), (z, int(1))], int(0)), int(0)),
    ];
    for (i, b) in bounds.iter().enumerate() {
        assert_bound(&mut r.sx[0], b, Tag::Fact(i + 1)).unwrap();
    }
    let Err(Failure::Conflict(why)) = r.check() else { panic!("y=x>=1 and z>=0 contradict y+z<=0") };
    assert_eq!(why, vec![Tag::Fact(0), Tag::Fact(1), Tag::Fact(2), Tag::Fact(3)]);
}

#[test]
fn fixed_links_use_physical_seeds_and_complete_relate_fix_support() {
    let origin = |k| PointRef::Origin(k);
    let gate = PointRef::Pin { object: 1, pin: 0, terminal: "g".into() };
    let p = problem_of(
        vec![instance(), instance()],
        vec![
            fact(FactKind::OrientRelation { relation: Orient::MY, from: vec![0], to: vec![1] }),
            fact(FactKind::Orient { object: 0, code: Orient::R0 }),
            fact(FactKind::Place { at: origin(1), axis: ProblemAxis::X, value: 0.0 }),
            fact(FactKind::Place { at: gate, axis: ProblemAxis::X, value: -3.0 }),
        ],
    );
    let model = Model::new(&p, int(0));
    let pin = model.var(PointId::Pin(1, 0));
    let mut r = Resolver::new(&model, 2, Limits::Caps(CAPS));
    for u in 0..3 {
        assert_eq!(r.add(u), Outcome::Kept);
    }
    assert_eq!(r.sx[0].value(pin), int(0), "the untouched old numeric pin is not a physical seed");
    assert_eq!(r.geometry.value(&model, PointId::Pin(1, 0), 0), int(3));
    r.freeze_placement_core();
    assert_eq!(r.sx[0].value(pin), int(3));
    let b = &model.units[3].fixed[0];
    let why = assert_bound(&mut r.sx[0], b, Tag::Fact(3)).unwrap_err();
    assert_eq!(why, vec![Tag::Fact(0), Tag::Fact(1), Tag::Fact(2), Tag::Fact(3)]);
    // The support is in the original domain and each original fact is necessary here:
    // remove relate => object 1 can be R0; remove fix => 0=MY,1=R0; remove origin=0
    // => origin 1 can be -6. Each explicit witness then places the gate at -3.
    for (removed, ori, at) in [
        (0, [Orient::R0, Orient::R0], int(0)),
        (1, [Orient::MY, Orient::R0], int(0)),
        (2, [Orient::R0, Orient::MY], int(-6)),
    ] {
        let mut g = Geometry::new(2);
        g.orient = ori.to_vec();
        g.origin[1][0] = at;
        for u in (0..4).filter(|u| *u != removed) {
            let unit = &model.units[u];
            assert!(match &unit.orient {
                OrientEffect::None => true,
                OrientEffect::Fix { object, code } => g.orient[*object] == *code,
                OrientEffect::Relate { relation, from, to } => relation_holds(&g.orient, *relation, from, to),
            });
            assert!(unit.fixed.iter().all(|b| b.holds(|v| g.var_value(&model, v, b.axis))));
        }
    }
}

#[test]
fn an_unfixed_held_orientation_is_not_part_of_the_new_base() {
    let p = problem_of(
        vec![instance()],
        vec![fact(FactKind::Place {
            at: PointRef::Pin { object: 0, pin: 0, terminal: "g".into() },
            axis: ProblemAxis::X,
            value: 3.0,
        })],
    );
    let mut model = Model::new(&p, int(0));
    let (origin, pin) = (model.var(PointId::Origin(0)), model.var(PointId::Pin(0, 0)));
    let touched = vec![PointId::Origin(0), PointId::Pin(0, 0)];
    model.units = vec![
        unit(vec![Bound::equal(0, LinExpr::var(origin), int(0))], Vec::new(), vec![PointId::Origin(0)]),
        unit(vec![Bound::within(0, LinExpr::diff(pin, origin), int(-3), int(3))], Vec::new(), touched.clone()),
        unit(vec![Bound::equal(0, LinExpr::var(pin), int(3))], Vec::new(), touched),
    ];
    let mut r = Resolver::new(&model, 1, Limits::Caps(CAPS));
    assert_eq!(r.add(0), Outcome::Kept);
    assert_eq!(r.add(1), Outcome::Kept);
    assert_eq!(r.geometry.orient[0], Orient::R0);
    assert_eq!(r.sx[0].value(pin), int(-3));
    r.freeze_placement_core();
    assert!(r.classes.fixed(0).is_none());
    assert_eq!(r.add(2), Outcome::Kept);
    assert_ne!(r.geometry.orient[0], Orient::R0);
    assert_eq!(r.geometry.origin[0][0], int(0));
    assert_eq!(r.geometry.value(&model, PointId::Pin(0, 0), 0), int(3));
}

#[test]
fn held_centre_extremes_remain_mutable_after_the_boundary() {
    let centre = PointRef::Centre((0..3).map(PointRef::Origin).collect());
    let p = problem_of(
        vec![point(), point(), point()],
        vec![
            fact(FactKind::Place { at: PointRef::Origin(0), axis: ProblemAxis::X, value: 0.0 }),
            fact(FactKind::Place { at: PointRef::Origin(1), axis: ProblemAxis::X, value: 3.0 }),
            fact(FactKind::Place { at: centre.clone(), axis: ProblemAxis::X, value: 0.0 }),
            fact(FactKind::Place { at: centre, axis: ProblemAxis::X, value: 6.0 }),
        ],
    );
    let mut model = Model::new(&p, int(0));
    model.units[2].fixed[0].hi = Some(int(6));
    let mut r = Resolver::new(&model, 3, Limits::Caps(CAPS));
    for u in 0..3 {
        assert_eq!(r.add(u), Outcome::Kept);
    }
    assert_eq!(r.held[&Held::Centre(0, 0, true)], 1);
    r.freeze_placement_core();
    assert_eq!(r.add(3), Outcome::Kept);
    // min(0,3,C)+max(0,3,C)=12 forces C=12, so the old highest member B must go.
    assert_eq!(r.geometry.origin[2][0], int(12));
    assert_eq!(r.held[&Held::Centre(0, 0, true)], 2);
}

#[test]
fn zero_search_nodes_stays_zero_across_core_compilation() {
    let model = binary_fixture();
    let mut r = Resolver::new(&model, 2, Limits::Caps(Caps { nodes: 0, pivots: CAPS.pivots }));
    assert_eq!(r.add(0), Outcome::Kept);
    assert_eq!(r.add(1), Outcome::Kept);
    assert_eq!(r.nodes, 0);
    r.freeze_placement_core();
    assert_eq!(r.nodes, 0);
    assert_eq!(r.add(2), Outcome::Undecided);
    assert_eq!(r.nodes, 0, "there is no extra search or cold retry");
    assert_eq!(r.kept, vec![0, 1]);
    assert_eq!(r.held[&Held::Choice(1, 0)], 0);
    assert_eq!(r.geometry.origin[0][0], int(0));
}

#[test]
fn actual_prefix_pivots_are_shared_by_take_and_consumer() {
    let p = problem_of(vec![point(), point(), point(), point()], Vec::new());
    let mut model = Model::new(&p, int(0));
    let [x, y, z, w] = [0, 1, 2, 3].map(|k| model.var(PointId::Origin(k)));
    model.units = vec![
        unit(
            vec![Bound::equal(0, LinExpr::from_terms([(x, int(1)), (y, int(1))], int(0)), int(1))],
            Vec::new(),
            vec![PointId::Origin(0), PointId::Origin(1)],
        ),
        unit(
            vec![Bound::at_least(0, LinExpr::from_terms([(z, int(1)), (w, int(1))], int(0)), int(1))],
            Vec::new(),
            vec![PointId::Origin(2), PointId::Origin(3)],
        ),
    ];
    let mut r = Resolver::new(&model, 4, Limits::Caps(Caps { nodes: CAPS.nodes, pivots: 1 }));
    assert_eq!(r.add(0), Outcome::Kept);
    let used = r.sx[0].pivots();
    assert_eq!(used, 1, "the prefix has actually consumed its sole pivot");
    let nodes = r.nodes;
    r.freeze_placement_core();
    assert_eq!(r.nodes, nodes);
    assert_eq!(r.sx[0].pivots(), used);
    assert_eq!(r.sx[0].pivot_limit(), Some(1));
    assert_eq!(r.add(1), Outcome::Undecided);
    assert_eq!(r.sx[0].pivots(), used);
    assert_eq!(r.kept, vec![0]);
    let mut consumer = r.sx[0].compact().unwrap();
    assert_eq!(consumer.pivots(), used);
    assert_eq!(consumer.pivot_limit(), Some(1));
    assert_bound(&mut consumer, &model.units[1].fixed[0], Tag::Fact(1)).unwrap();
    assert_eq!(consumer.check(), Err(Failure::Budget));
    assert_eq!(consumer.pivots(), used);
}
