use super::*;
use crate::resolve::tests::{instance, problem_of};

#[test]
fn a_relation_between_two_points_is_one_bound_and_between_sets_a_separator() {
    let p = problem_of(vec![instance(), instance(), instance()], Vec::new());
    let mut model = Model::new(&p, Q::zero());
    let o = PointId::Origin;
    let one = (Q::one(), Q::one());
    let rel =
        Relation::Apart { axis: 1, low: vec![(o(0), one.0.clone())], high: vec![(o(1), -one.1.clone())], gap: int(2) };
    let u = rel.unit(&mut model, 0);
    assert_eq!(u.fixed.len(), 1);
    // y1 − 1 ≥ y0 + 1 + 2
    assert_eq!(u.fixed[0].lo, Some(int(4)));
    assert_eq!(u.touched[1], vec![o(0), o(1)]);
    let vars = model.vars.len();
    let rel = Relation::Apart {
        axis: 0,
        low: vec![(o(0), int(1)), (o(1), int(1))],
        high: vec![(o(2), int(-3))],
        gap: int(2),
    };
    let u = rel.unit(&mut model, 1);
    assert_eq!(model.vars.len(), vars + 1);
    assert_eq!(u.fixed.len(), 3);
    let rel = Relation::OneOf(vec![
        vec![(0, vec![(o(1), int(1)), (o(0), int(-1))], int(3))],
        vec![(1, vec![(o(0), int(1))], int(0))],
    ]);
    let u = rel.unit(&mut model, 2);
    assert_eq!(u.choices.len(), 1);
    assert_eq!(u.choices[0].values.len(), 2);
    assert_eq!(u.choices[0].kind, ChoiceKind::Ranked);
    assert_eq!(u.touched, [vec![o(0), o(1)], vec![o(0)]]);
}

/// An ORDER given only an axis fixes no precedence; reversed, its members swap their order,
/// the others staying where they were.
#[test]
fn a_fact_whose_side_is_open_is_reversed() {
    use rnl_runtime::problem::{Axis, FactKind, PointRef, Sense};
    let o = PointRef::Origin;
    let order = FactKind::Order { axis: Axis::X, sense: Sense::Either, members: vec![o(0), o(2)] };
    let p = problem_of(vec![instance(), instance(), instance()], vec![crate::resolve::tests::fact(order)]);
    let options = SolverOptions::without_time_limit();
    let mut work = Work::default();
    let resolved = crate::resolve(&p, &options);
    let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut work);
    assert!(s.precedence.is_empty());
    assert_eq!(s.reversible.len(), 1);
    let (a, b) = s.reversible[0].pairs[0];
    let mut search = Search { problem: &p, s: &s, options: &options, made: 0, work: &mut work };
    let border = vec![Item::Element(0), Item::Element(1), Item::Element(2)];
    let mut plan = p0_plan(&s);
    plan.items.insert(0, border.clone());
    let reversed = search.reversed(0, &plan, &border);
    assert_eq!(reversed.len(), 1);
    let q = &reversed[0].2;
    assert_eq!(q.flipped, BTreeSet::from([0]));
    let at = |k: usize| q.order.iter().position(|&j| j == k).unwrap();
    // the witness had a before b: now b goes first
    assert!(at(b) < at(a), "{:?}", q.order);
    // and reversed again, back as the witness had it
    let again = search.reversed(0, q, &q.items[&0].clone());
    assert!(again[0].2.flipped.is_empty());
    let at = |k: usize| again[0].2.order.iter().position(|&j| j == k).unwrap();
    assert!(at(a) < at(b), "{:?}", again[0].2.order);
}

#[test]
fn constants_go_up_to_the_grid() {
    let g = Some(int(1));
    assert_eq!(up(crate::rational::frac(16, 5), &g), int(4));
    assert_eq!(up(int(3), &g), int(3));
    assert_eq!(up(crate::rational::frac(-1, 2), &g), int(0));
    assert_eq!(up(crate::rational::frac(16, 5), &None), crate::rational::frac(16, 5));
    assert_eq!(up(crate::rational::frac(3, 4), &Some(crate::rational::frac(1, 2))), int(1));
}

fn budget_fixture() -> (LevelProblem, SolverOptions) {
    use rnl_runtime::problem::{CurrentDirection, CurrentTerminal, ProblemCurrent, ProblemGroup, Reference};
    let mut objects: Vec<_> = (0..6)
        .map(|k| {
            let mut object = instance();
            object.label = format!("#M{k}");
            object.reference = Reference::Name(object.label.clone());
            object
        })
        .collect();
    let mut problem = problem_of(std::mem::take(&mut objects), Vec::new());
    problem.groups = vec![
        ProblemGroup { label: "inner".into(), members: vec![0, 1, 2] },
        ProblemGroup { label: "middle".into(), members: vec![0, 1, 2, 3] },
        ProblemGroup { label: "outer".into(), members: (0..6).collect() },
    ];
    for object in 0..6 {
        for (pin, direction) in [(0, CurrentDirection::In), (1, CurrentDirection::Out)] {
            problem.currents.push(ProblemCurrent {
                terminal: CurrentTerminal::Instance { instance: object, terminal: format!("p{pin}") },
                net: "feedback".into(),
                direction,
                display: Some(rnl_runtime::problem::PointRef::Pin { object, pin, terminal: format!("p{pin}") }),
            });
        }
    }
    let mut options = SolverOptions::without_time_limit();
    for direction in [crate::Direction::Left, crate::Direction::Right] {
        options.objectives.layers.push(crate::Layer {
            items: vec![crate::Term { direction: Some(direction), ..crate::Term::new("current_direction") }],
            tolerance: Q::zero(),
        });
    }
    (problem, options)
}

#[test]
fn candidate_budget_zero_one_two_bound_actual_work_and_keep_p0_in_a_deep_tree() {
    let (problem, mut options) = budget_fixture();
    for budget in [0, 1, 2] {
        options.budget.candidates = budget;
        let resolved = crate::resolve(&problem, &options);
        let mut work = Work::default();
        let structure = crate::structure::analyse(&problem, &resolved, &options, crate::options::CAPS, &mut work);
        assert!(structure.tree.nodes.len() >= 4);
        assert!(structure.has(Source::EvenRows));
        assert!(structure.has(Source::Barycentre));
        assert!(structure.depth.is_some());
        assert!(!evenly(&structure, &structure.rows).is_empty());
        let candidates = search(&problem, &structure, &options, &mut work);
        assert!(work.candidates <= budget.max(1), "budget={budget}, work={work:?}");
        assert!(candidates.len() as u64 <= work.candidates);
        assert!(candidates.iter().any(|candidate| candidate.origin == "P0" && !candidate.even));
        if budget <= 1 {
            assert_eq!(work.candidates, 1);
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].origin, "P0");
        }
    }
}

#[test]
fn even_twins_and_additional_current_frames_are_admitted_and_counted() {
    let (problem, mut options) = budget_fixture();
    options.budget.candidates = 2000;
    let resolved = crate::resolve(&problem, &options);
    let mut work = Work::default();
    let structure = crate::structure::analyse(&problem, &resolved, &options, crate::options::CAPS, &mut work);
    let candidates = search(&problem, &structure, &options, &mut work);
    assert!(work.candidates < options.budget.candidates, "fixture must exercise an unexhausted search");
    assert!(candidates.iter().any(|candidate| candidate.even));
    let frames: BTreeSet<_> = candidates.iter().filter_map(|candidate| candidate.flow).collect();
    assert_eq!(frames, BTreeSet::from([crate::Direction::Down, crate::Direction::Left, crate::Direction::Right]));
    assert!(candidates.len() as u64 <= work.candidates);
    for budget in [3, 5, 9, work.candidates - 1] {
        options.budget.candidates = budget;
        let mut limited = Work::default();
        let candidates = search(&problem, &structure, &options, &mut limited);
        assert!(limited.candidates <= budget);
        assert!(candidates.len() as u64 <= limited.candidates);
        assert!(candidates.iter().any(|candidate| candidate.origin == "P0" && !candidate.even));
    }
}

#[test]
fn candidate_budget_stops_preparation_for_unadmitted_orders() {
    let problem = problem_of(vec![instance(), instance(), instance()], Vec::new());
    for budget in [0, 1, 2] {
        let mut options = SolverOptions::without_time_limit();
        options.budget.candidates = budget;
        let resolved = crate::resolve(&problem, &options);
        let mut work = Work::default();
        let structure = crate::structure::analyse(&problem, &resolved, &options, crate::options::CAPS, &mut work);
        assert_eq!(structure.tree.nodes.len(), 1);
        assert!(structure.has(Source::Barycentre));
        crate::test_profile::reset();
        let candidates = search(&problem, &structure, &options, &mut work);
        let calls = crate::test_profile::take();
        assert_eq!(work.candidates, budget.max(1));
        assert!(candidates.iter().any(|c| c.origin == "P0"));
        assert_eq!(calls.get("search.barycentre"), None, "budget={budget}: {calls}");
        let orientations = calls["search.orient"]["calls"].as_u64().unwrap_or(0);
        assert_eq!(orientations, u64::from(budget == 2), "budget={budget}: {calls}");
    }
}
