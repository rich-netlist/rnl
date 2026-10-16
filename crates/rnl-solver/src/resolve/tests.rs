//! Tests of the resolution of 8.5: the examples of the development plan (5.3), each checked for
//! what is kept and discarded, for conflict sets that are minimal, and for a witness that
//! satisfies every fact kept by a check written from the standard (14.1–14.5, 14.9), apart from
//! the encoding of the model.

use num_traits::One;
use rnl_runtime::geom::Orient;
use rnl_runtime::interp::obj::Priority;
use rnl_runtime::model::PinView;
use rnl_runtime::problem::*;

use super::*;
use crate::options::CAPS;
use crate::rational::{frac, int};
use crate::StoppedClock;

/// The resolution of 8.5 with a clock that never advances: the time limit is never reached.
fn resolve(problem: &LevelProblem, options: &SolverOptions) -> Resolved {
    super::resolve(problem, options, &StoppedClock)
}

/// A clock that advances one second each time it is read: a time limit of `n` seconds is out at
/// the `n + 1`-th reading after the resolution begins, at the same point on every run (the draft
/// 0.3, 8.5).
#[derive(Debug, Default)]
pub(crate) struct Ticks(std::cell::Cell<u64>);

impl Ticks {
    /// How many times it was read.
    pub(crate) fn reads(&self) -> u64 {
        self.0.get()
    }
}

impl Clock for Ticks {
    fn elapsed(&self) -> f64 {
        let t = self.0.get();
        self.0.set(t + 1);
        t as f64
    }
}

// ---------------------------------------------------------------------------- building problems

/// A MOS-like symbol: the gate at (−3, 0), the drain at (0, 2), the source at (0, −2).
fn symbol() -> ProblemSymbol {
    let pin = |x: f64, y: f64, dir: u16, t: &str| PinView {
        pos: (x, y),
        label: None,
        dir: Some(dir),
        terminals: vec![t.to_string()],
    };
    ProblemSymbol {
        name: "$NMOS".to_string(),
        bbox: [-3.0, -2.0, 0.0, 2.0],
        box_written: true,
        default: false,
        pins: vec![pin(-3.0, 0.0, 180, "g"), pin(0.0, 2.0, 90, "d"), pin(0.0, -2.0, 270, "s")],
    }
}

/// An instance of the symbol, free.
pub(crate) fn instance() -> ProblemObject {
    ProblemObject {
        kind: ObjectKind::Instance,
        label: "#M".to_string(),
        reference: Reference::Name("#M".to_string()),
        generated: false,
        symbol: Some(0),
        position: Position::Own { x: Slot::Free, y: Slot::Free },
        orient: Some(OrientSlot::Free),
        usage: None,
        text: None,
    }
}

/// A free point.
pub(crate) fn point() -> ProblemObject {
    ProblemObject {
        kind: ObjectKind::Point,
        label: "%p".to_string(),
        reference: Reference::Name("%p".to_string()),
        generated: false,
        symbol: None,
        position: Position::Own { x: Slot::Free, y: Slot::Free },
        orient: None,
        usage: None,
        text: None,
    }
}

/// A fact of normal priority.
pub(crate) fn fact(kind: FactKind) -> ProblemFact {
    ProblemFact {
        label: kind.name().to_string(),
        kind,
        priority: Priority::Normal,
        reference: None,
        source: None,
        chain: Vec::new(),
    }
}

/// A level with these objects and these facts, in the order of 8.5.
pub(crate) fn problem_of(objects: Vec<ProblemObject>, facts: Vec<ProblemFact>) -> LevelProblem {
    LevelProblem {
        level: 0,
        title: "t.sp".to_string(),
        kind: "file",
        name: String::new(),
        version: "0.2".to_string(),
        authored_numbers: 0,
        file: "t.sp".to_string(),
        line: 1,
        end_line: 1,
        packages: Vec::new(),
        names: Vec::new(),
        objects,
        symbols: vec![symbol()],
        facts,
        wires: Vec::new(),
        routes: Vec::new(),
        groups: Vec::new(),
        usages: Vec::new(),
    }
}

fn o(k: usize) -> PointRef {
    PointRef::Origin(k)
}

fn pin(k: usize, j: usize) -> PointRef {
    PointRef::Pin { object: k, pin: j, terminal: ["g", "d", "s"][j].to_string() }
}

fn place(at: PointRef, axis: Axis, value: f64) -> ProblemFact {
    fact(FactKind::Place { at, axis, value })
}

fn align(axis: Axis, members: Vec<PointRef>) -> ProblemFact {
    fact(FactKind::Align { axis, members })
}

fn distance(sense: Sense, min: Option<f64>, max: Option<f64>, from: Vec<PointRef>, to: Vec<PointRef>) -> ProblemFact {
    fact(FactKind::Distance { axis: Axis::X, sense, min, max, from, to })
}

fn distribute(members: Vec<PointRef>) -> ProblemFact {
    fact(FactKind::Distribute { axis: Axis::X, members })
}

fn orient(object: usize, code: Orient) -> ProblemFact {
    fact(FactKind::Orient { object, code })
}

fn points(n: usize) -> Vec<ProblemObject> {
    (0..n).map(|_| point()).collect()
}

/// A fact discarded with one minimal conflict group, which is all it has.
fn discarded(fact: usize, group: Vec<usize>) -> Discarded {
    Discarded { fact, groups: vec![group], complete: true, undecided: false }
}

/// A fact discarded with these minimal conflict groups, which are all it has.
fn discarded_with(fact: usize, groups: Vec<Vec<usize>>) -> Discarded {
    Discarded { fact, groups, complete: true, undecided: false }
}

/// Whether some facts, which hold together, have no solution with `f`, decided with the caps of
/// the implementation.
fn conflicts_with(model: &Model, n: usize, set: &[usize], f: usize) -> bool {
    let mut s = Resolver::new(model, n, Limits::Caps(CAPS));
    assert!(set.iter().all(|&c| s.add(c) == Outcome::Kept), "{set:?} do not hold together");
    matches!(s.add(f), Outcome::Conflict(_))
}

/// Checks that a group of `f` is one the hint of 8.5 names (the draft 0.3, 8.5): facts kept before
/// `f`, sorted, that have no solution together with it, and from which no fact can be left out.
fn check_group(r: &Resolved, n: usize, f: usize, group: &[usize], what: &str) {
    assert!(group.windows(2).all(|w| w[0] < w[1]), "{what}: {group:?} not sorted");
    assert!(group.iter().all(|c| *c < f && r.kept.contains(c)), "{what}: {group:?}");
    assert!(conflicts_with(&r.model, n, group, f), "{what}: {group:?} does not conflict with {f}");
    for c in group {
        let rest: Vec<usize> = group.iter().copied().filter(|x| x != c).collect();
        assert!(!conflicts_with(&r.model, n, &rest, f), "{what}: {group:?} of {f} is not minimal: {c} is not needed");
    }
}

/// Every minimal conflict group of `f` among `before`, found by trying the subsets, smallest
/// first: a subset with a subset one fact smaller that conflicts is not minimal and is not tried.
/// None when there are too many subsets.
fn every_group(model: &Model, n: usize, f: usize, before: &[usize]) -> Option<Vec<Vec<usize>>> {
    let m = before.len();
    if m > 10 {
        return None;
    }
    let mut masks: Vec<usize> = (0..1 << m).collect();
    masks.sort_by_key(|mask| mask.count_ones());
    let mut conflicting = vec![false; 1 << m];
    let mut groups = Vec::new();
    for mask in masks {
        let members: Vec<usize> = (0..m).filter(|i| mask >> i & 1 == 1).collect();
        if members.iter().any(|i| conflicting[mask & !(1 << i)]) {
            conflicting[mask] = true;
            continue;
        }
        let set: Vec<usize> = members.iter().map(|&i| before[i]).collect();
        if conflicts_with(model, n, &set, f) {
            conflicting[mask] = true;
            groups.push(set);
        }
    }
    groups.sort();
    Some(groups)
}

// ---------------------------------------------------------------------------- checking results

/// Whether a placement satisfies a fact, from the standard: points as 14.1 counts them, constant
/// bounds within the tolerance, equalities exact.
pub(crate) fn semantic_holds(problem: &LevelProblem, kind: &FactKind, g: &Geometry, tol: &Q) -> bool {
    let v = |r: &PointRef, axis: usize| point_value(problem, g, r, axis, 0).expect("a point of the problem");
    let near = |a: Q, b: f64| {
        let d = a - from_f64(b).unwrap();
        d <= *tol && -d <= *tol
    };
    let axis_of = |a: &Axis| crate::model::ax(*a);
    match kind {
        FactKind::Place { at, axis, value } => near(v(at, axis_of(axis)), *value),
        FactKind::PointCoordinate { point, axis, value } => near(v(&o(*point), axis_of(axis)), *value),
        FactKind::Offset { text, from, axis, value } => {
            let a = axis_of(axis);
            near(v(&o(*text), a) - v(from, a), *value)
        }
        FactKind::Orient { object, code } => g.orient[*object] == *code,
        FactKind::OrientRelation { relation, from, to } => relation_holds(&g.orient, *relation, from, to),
        FactKind::Align { axis, members } => {
            let vals: Vec<Q> = members.iter().map(|m| v(m, axis_of(axis))).collect();
            vals.windows(2).all(|w| w[0] == w[1])
        }
        FactKind::Order { axis, sense, members } => {
            let vals: Vec<Q> = members.iter().map(|m| v(m, axis_of(axis))).collect();
            let step = Q::one() - tol;
            let inc = vals.windows(2).all(|w| &w[1] - &w[0] >= step);
            let dec = vals.windows(2).all(|w| &w[0] - &w[1] >= step);
            match sense {
                Sense::Increasing => inc,
                Sense::Decreasing => dec,
                Sense::Either => inc || dec,
            }
        }
        FactKind::Distribute { axis, members } => {
            let vals: Vec<Q> = members.iter().map(|m| v(m, axis_of(axis))).collect();
            vals.windows(3).all(|w| &w[2] - &w[1] == &w[1] - &w[0])
        }
        FactKind::Distance { axis, sense, min, max, from, to } => {
            let a = axis_of(axis);
            let extent = |ms: &[PointRef]| {
                let vals: Vec<Q> = ms.iter().map(|m| v(m, a)).collect();
                vals.iter().min().cloned().zip(vals.iter().max().cloned())
            };
            let (Some((a_lo, a_hi)), Some((b_lo, b_hi))) = (extent(from), extent(to)) else { return true };
            let (g0, g1) = (&b_lo - &a_hi, &a_lo - &b_hi);
            let d = match sense {
                Sense::Increasing => g0,
                Sense::Decreasing => g1,
                Sense::Either => g0.max(g1),
            };
            min.is_none_or(|m| d >= from_f64(m).unwrap() - tol) && max.is_none_or(|m| d <= from_f64(m).unwrap() + tol)
        }
    }
}

/// Resolves a problem and checks what every resolution must give: the witness satisfies every
/// fact kept, the model's own test agrees with the standard's on every fact, each conflict group
/// is made of facts kept before its fact, conflicts with it and is minimal, the groups of each
/// fact are all of them (the draft 0.3, 8.5), tried subset by subset when there are few enough,
/// the hints by statement are those of the groups, and a second run gives the same.
fn check(problem: &LevelProblem, options: &SolverOptions) -> Resolved {
    let r = resolve(problem, options);
    let tol = r.model.tol.clone();
    let g = &r.witness.geometry;
    let mut all: Vec<usize> = r.kept.clone();
    all.extend(r.discarded.iter().map(|d| d.fact));
    all.extend(&r.failed);
    all.sort_unstable();
    assert_eq!(all, (0..problem.facts.len()).collect::<Vec<_>>());
    for &f in &r.kept {
        assert!(
            semantic_holds(problem, &problem.facts[f].kind, g, &tol),
            "fact {f} kept but violated: {:?}",
            problem.facts[f]
        );
    }
    // the model's test of a unit is the standard's, on every fact
    let mut probe = Resolver::new(&r.model, problem.objects.len(), Limits::Caps(CAPS));
    probe.geometry = g.clone();
    for (f, pf) in problem.facts.iter().enumerate() {
        if r.failed.contains(&f) {
            // left out of the model: no witness satisfies it
            assert!(r.model.left_out.contains(&f) && !semantic_holds(problem, &pf.kind, g, &tol), "fact {f}");
            continue;
        }
        assert_eq!(probe.holds(f), semantic_holds(problem, &pf.kind, g, &tol), "fact {f}: {:?}", pf.kind);
    }
    // each fact that fails has its error (7.6), in order, and fails alone again
    assert_eq!(r.errors.len(), r.failed.len(), "{:?}", r.errors);
    for (e, &f) in r.errors.iter().zip(&r.failed) {
        assert!(e.starts_with(&format!("the fact [{f}] {} ", problem.facts[f].label)), "{e}");
        assert!(e.ends_with(" contradicts itself and fails (8.5, 7.6)"), "{e}");
        let alone = LevelProblem { facts: vec![problem.facts[f].clone()], ..problem.clone() };
        assert_eq!(resolve(&alone, options).failed, [0], "fact {f}");
    }
    let n = problem.objects.len();
    for d in r.discarded.iter().filter(|d| !d.undecided) {
        assert!(d.complete && !d.groups.is_empty(), "{d:?}");
        assert!(d.groups.windows(2).all(|w| w[0] < w[1]), "{d:?}: the groups are not sorted");
        for g in &d.groups {
            check_group(&r, n, d.fact, g, &format!("{d:?}"));
        }
        let before: Vec<usize> = r.kept.iter().copied().filter(|&k| k < d.fact).collect();
        if let Some(every) = every_group(&r.model, n, d.fact, &before) {
            assert_eq!(d.groups, every, "{d:?}: not every group");
        }
    }
    assert_eq!(r.statements, statement_hints(problem, &r.discarded));
    for (s, hint) in &r.statements {
        for &k in &hint.facts {
            assert_eq!(&Statement::of(problem, k), s);
        }
        for &f in &hint.discarded {
            let d = r.discarded.iter().find(|d| d.fact == f).unwrap();
            assert!(hint.facts.iter().any(|k| d.in_groups().contains(k)), "{s:?}: {d:?}");
        }
    }
    assert_eq!(r, resolve(problem, options), "not deterministic");
    r
}

fn x_of(r: &Resolved, k: usize) -> Q {
    r.witness.geometry.origin[k][0].clone()
}

// ---------------------------------------------------------------------------- the plan's examples

#[test]
fn the_example_of_the_brainstorm_message() {
    // written 1 high A.y = 0; 2 ALIGN y {A B}; 3 B.y = 3; 4 low ALIGN x {A B}; 5 DISTANCE x |B − A| ≥ 2
    // resolved in the order 1, 5, 3, 2, 4
    let mut facts = vec![
        place(o(0), Axis::Y, 0.0),
        distance(Sense::Either, Some(2.0), None, vec![o(0)], vec![o(1)]),
        place(o(1), Axis::Y, 3.0),
        align(Axis::Y, vec![o(0), o(1)]),
        align(Axis::X, vec![o(0), o(1)]),
    ];
    facts[0].priority = Priority::High;
    facts[4].priority = Priority::Low;
    let p = problem_of(points(2), facts);
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.kept, vec![0, 1, 2]);
    // fact 2 has exactly one minimal group, facts 1 and 3, and fact 4 exactly one, fact 5 (the
    // draft 0.3, 8.5)
    assert_eq!(r.discarded, vec![discarded(3, vec![0, 2]), discarded(4, vec![1])]);
    assert_eq!(r.discarded[0].groups, [[0, 2]]);
    assert_eq!(r.discarded[1].groups, [[1]]);
}

#[test]
fn inequalities_in_a_cycle() {
    let p = problem_of(
        points(3),
        vec![
            distance(Sense::Increasing, Some(10.0), None, vec![o(0)], vec![o(1)]),
            distance(Sense::Increasing, Some(10.0), None, vec![o(1)], vec![o(2)]),
            distance(Sense::Increasing, None, Some(15.0), vec![o(0)], vec![o(2)]),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.discarded, vec![discarded(2, vec![0, 1])]);
}

#[test]
fn an_equality_of_three_variables() {
    let p = problem_of(
        points(3),
        vec![
            distribute(vec![o(0), o(1), o(2)]),
            distance(Sense::Increasing, Some(10.0), Some(10.0), vec![o(0)], vec![o(1)]),
            distance(Sense::Increasing, Some(15.0), Some(15.0), vec![o(1)], vec![o(2)]),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.discarded, vec![discarded(2, vec![0, 1])]);
}

#[test]
fn an_equality_with_inequalities() {
    let p = problem_of(
        points(3),
        vec![
            distribute(vec![o(0), o(1), o(2)]),
            distance(Sense::Increasing, Some(30.0), None, vec![o(0)], vec![o(2)]),
            distance(Sense::Increasing, None, Some(10.0), vec![o(0)], vec![o(1)]),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.discarded, vec![discarded(2, vec![0, 1])]);
}

#[test]
fn two_orients_of_one_object() {
    let p = problem_of(vec![instance()], vec![orient(0, Orient::R0), orient(0, Orient::MY)]);
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.discarded, vec![discarded(1, vec![0])]);
    assert_eq!(r.orientation(0), Some(Orient::R0));
}

#[test]
fn a_relation_and_two_orients() {
    let p = problem_of(
        vec![instance(), instance()],
        vec![
            fact(FactKind::OrientRelation { relation: Orient::MY, from: vec![0], to: vec![1] }),
            orient(0, Orient::R0),
            orient(1, Orient::R0),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.discarded, vec![discarded(2, vec![0, 1])]);
    assert_eq!((r.orientation(0), r.orientation(1)), (Some(Orient::R0), Some(Orient::MY)));
}

#[test]
fn an_orientation_matters_once_two_points_of_one_object_are_reached() {
    // M's gate to x = 3 and its source to y = 2 reach one point of M on each axis: any orientation
    // will do. Placing its origin at (0, 0) then reaches two on each axis: the gate must be 3 to the
    // right and the source 2 up, which only R180 gives.
    let p = problem_of(
        vec![instance(), point(), point()],
        vec![
            fact(FactKind::PointCoordinate { point: 1, axis: Axis::X, value: 3.0 }),
            fact(FactKind::PointCoordinate { point: 2, axis: Axis::Y, value: 2.0 }),
            align(Axis::X, vec![pin(0, 0), o(1)]),
            align(Axis::Y, vec![pin(0, 2), o(2)]),
            place(o(0), Axis::X, 0.0),
            place(o(0), Axis::Y, 0.0),
            orient(0, Orient::MY),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.kept, vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(r.orientation(0), Some(Orient::R180));
    assert_eq!(r.point(&p, &pin(0, 0)), Some((3.0, 0.0)));
    assert_eq!(r.point(&p, &pin(0, 2)), Some((0.0, 2.0)));
    // MY puts the gate right but the source down: it conflicts with what holds the source's y
    assert_eq!(r.discarded, vec![discarded(6, vec![1, 3, 5])]);
}

#[test]
fn a_fact_against_a_kept_bound_that_a_held_orientation_tightened() {
    // the gate at least 1 above the origin: the orientation turns the gate up, 3 above (R270 or
    // MYR90), and the link holds the row of gate − origin at 3. A DISTRIBUTE that puts the gate at
    // the origin's height then conflicts with the DISTANCE, not with the orientation.
    let p = problem_of(
        vec![instance(), point()],
        vec![
            fact(FactKind::Distance {
                axis: Axis::Y,
                sense: Sense::Decreasing,
                min: Some(1.0),
                max: Some(1.0),
                from: vec![o(1), pin(0, 0)],
                to: vec![o(0)],
            }),
            fact(FactKind::Distribute { axis: Axis::Y, members: vec![o(0), pin(0, 0), pin(0, 0)] }),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.discarded, vec![discarded(1, vec![0])]);
    assert!([Some(Orient::R270), Some(Orient::MYR90)].contains(&r.orientation(0)));
}

/// A text the symbol of object 0 places at (1, 1), with the axis the scene gives it, if any.
pub(crate) fn label(replaced: Option<Axis>) -> ProblemObject {
    ProblemObject {
        kind: ObjectKind::Text,
        label: "#M::name".to_string(),
        reference: Reference::Name("#M::name".to_string()),
        symbol: None,
        position: Position::InSymbol { owner: 0, local: (1.0, 1.0), replaced: replaced.map(|a| (a, Slot::Free)) },
        orient: None,
        ..instance()
    }
}

#[test]
fn a_text_that_the_scene_places_on_one_axis_leaves_its_owner_there() {
    // #M::name @PLACE(x=-3) replaces the x the symbol gives the text, and only that (13.3): M stays
    // at (0, 0), the text's y still follows M, turned with it
    let facts = vec![
        place(o(1), Axis::X, -3.0),
        place(o(0), Axis::X, 0.0),
        place(o(0), Axis::Y, 0.0),
        orient(0, Orient::R90),
        place(o(1), Axis::Y, 5.0),
    ];
    let p = problem_of(vec![instance(), label(Some(Axis::X))], facts.clone());
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.kept, vec![0, 1, 2, 3]);
    // R90 turns the text's (1, 1) to (-1, 1): its y is M's plus 1; no orientation puts it 5 above
    // M's y of 0
    assert_eq!(r.discarded, vec![discarded(4, vec![2])]);
    assert_eq!(r.point(&p, &o(1)), Some((-3.0, 1.0)));
    assert_eq!(r.point(&p, &o(0)), Some((0.0, 0.0)));
    // a text the symbol places on both axes moves its owner instead
    let p = problem_of(vec![instance(), label(None)], facts);
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.discarded[0], discarded(1, vec![0]));
}

#[test]
fn a_fixed_orientation_decides_the_pins() {
    // with ORIENT first, the class has one value, and the pins follow it
    let p = problem_of(
        vec![instance()],
        vec![
            orient(0, Orient::MY),
            place(o(0), Axis::X, 1.0),
            place(pin(0, 0), Axis::X, 4.0),
            place(pin(0, 0), Axis::X, -2.0),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.kept, vec![0, 1, 2]);
    // the gate at −2 conflicts with the gate at 4, and with MY and the origin at 1, which put the
    // gate at 4 too: two minimal groups (the draft 0.3, 8.5)
    assert_eq!(r.discarded, vec![discarded_with(3, vec![vec![0, 1], vec![2]])]);
}

#[test]
fn an_order_given_only_an_axis_turns_round_when_it_must() {
    let p = problem_of(
        points(2),
        vec![
            fact(FactKind::Order { axis: Axis::X, sense: Sense::Either, members: vec![o(0), o(1)] }),
            place(o(0), Axis::X, 5.0),
            place(o(1), Axis::X, 0.0),
            fact(FactKind::Order { axis: Axis::X, sense: Sense::Increasing, members: vec![o(0), o(1)] }),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.kept, vec![0, 1, 2]);
    assert_eq!((x_of(&r, 0), x_of(&r, 1)), (int(5), int(0)));
    assert_eq!(r.discarded, vec![discarded(3, vec![1, 2])]);
}

#[test]
fn distances_between_sets() {
    // gap = min(C, D) − max(A, B) within [2, 3]
    let p = problem_of(
        points(4),
        vec![
            distance(Sense::Increasing, Some(2.0), Some(3.0), vec![o(0), o(1)], vec![o(2), o(3)]),
            place(o(0), Axis::X, 0.0),
            place(o(1), Axis::X, 4.0),
            place(o(2), Axis::X, 10.0),
            place(o(3), Axis::X, 8.0),
            place(o(3), Axis::X, 6.5),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.discarded, vec![discarded(4, vec![0, 1, 2, 3])]);
    assert_eq!(x_of(&r, 3), frac(13, 2));
}

#[test]
fn distances_either_way_between_sets() {
    // {A B} and {C D} apart by at least 2, either way; then C between A and B
    let p = problem_of(
        points(4),
        vec![
            distance(Sense::Either, Some(2.0), Some(4.0), vec![o(0), o(1)], vec![o(2), o(3)]),
            place(o(0), Axis::X, 10.0),
            place(o(1), Axis::X, 12.0),
            place(o(3), Axis::X, 7.0),
            place(o(2), Axis::X, 11.0),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.kept, vec![0, 1, 2, 3]);
    // C at 11 is within a unit of A and of B: either with the DISTANCE is a minimal conflict
    // group, and the hint names both (the draft 0.3, 8.5)
    assert_eq!(r.discarded, vec![discarded_with(4, vec![vec![0, 1], vec![0, 2]])]);
}

#[test]
fn the_centre_of_a_set_of_three() {
    let p = problem_of(
        points(4),
        vec![
            place(o(0), Axis::X, 0.0),
            place(o(1), Axis::X, 10.0),
            place(o(2), Axis::X, 4.0),
            align(Axis::X, vec![PointRef::Centre(vec![o(0), o(1), o(2)]), o(3)]),
            place(o(3), Axis::X, 7.0),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    // D at 7 would need A, B or C elsewhere: C beyond 10 would move the highest point
    assert_eq!(r.discarded, vec![discarded(4, vec![0, 1, 2, 3])]);
    assert_eq!(x_of(&r, 3), int(5));
}

#[test]
fn a_centre_whose_extremes_must_change() {
    // the centre of {A B C} at 6 with A at 0: the highest point must be at 12, which C, free, can be
    let p = problem_of(
        points(3),
        vec![
            place(o(0), Axis::X, 0.0),
            place(o(1), Axis::X, 3.0),
            place(PointRef::Centre(vec![o(0), o(1), o(2)]), Axis::X, 6.0),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.kept, vec![0, 1, 2]);
    assert_eq!(x_of(&r, 2), int(12));
}

#[test]
fn facts_that_contradict_themselves_fail_and_take_no_part() {
    // an ORDER of one point twice, a DISTANCE from a point to itself with min=1 (9.5): no solution
    // on their own, so they fail (the draft 0.3, 8.5; 7.6), and are neither kept nor discarded
    let mut p = problem_of(
        points(2),
        vec![
            fact(FactKind::Order { axis: Axis::X, sense: Sense::Either, members: vec![o(0), o(0)] }),
            distance(Sense::Either, Some(1.0), None, vec![o(1)], vec![o(1)]),
            align(Axis::X, vec![o(1)]),
        ],
    );
    p.facts[1].source = Some(rnl_runtime::diag::Loc::new("cases/t.sp", 7, 3));
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!((r.kept.as_slice(), r.discarded.as_slice(), r.failed.as_slice()), (&[2][..], &[][..], &[0, 1][..]));
    let label = &p.facts[0].label;
    assert_eq!(
        r.errors,
        [
            format!("the fact [0] {label} contradicts itself and fails (8.5, 7.6)"),
            "the fact [1] DISTANCE (t.sp:7) contradicts itself and fails (8.5, 7.6)".to_string()
        ]
    );
    assert!(r.warnings.is_empty() && r.statements.is_empty());
}

/// The facts of `p` without the fact `f`.
fn without(p: &LevelProblem, f: usize) -> LevelProblem {
    let mut q = p.clone();
    q.facts.remove(f);
    q
}

#[test]
fn two_pins_of_one_object_that_no_orientation_aligns() {
    // {#M1:d #M1:g} @ALIGN("y") on the NMOS: the drain at (0, 2) and the gate at (−3, 0), whose y
    // differ by 2, −2, 3 or −3 whatever the orientation: no solution on its own, it fails. The
    // drain and the source, (0, 2) and (0, −2), share a y once turned a quarter
    let p = problem_of(
        vec![instance()],
        vec![align(Axis::Y, vec![pin(0, 1), pin(0, 0)]), align(Axis::Y, vec![pin(0, 1), pin(0, 2)])],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!((r.failed.as_slice(), r.kept.as_slice()), (&[0][..], &[1][..]));
    assert!([Orient::R90, Orient::R270, Orient::MXR90, Orient::MYR90].contains(&r.orientation(0).unwrap()));
    // alone, and without its one solution of ALIGN: the same
    let q = without(&p, 1);
    assert_eq!(check(&q, &SolverOptions::without_time_limit()).failed, [0]);
    // an orientation fixed first changes nothing: the check is of the fact on its own
    let mut p = p;
    p.facts.insert(0, orient(0, Orient::R90));
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!((r.failed.as_slice(), r.kept.as_slice()), (&[1][..], &[0, 2][..]));
}

#[test]
fn an_order_whose_members_are_at_one_place_through_the_centres_of_sets() {
    let c = PointRef::Centre;
    let order = |members: Vec<PointRef>| fact(FactKind::Order { axis: Axis::X, sense: Sense::Either, members });
    let p = problem_of(
        vec![point(), point(), instance()],
        vec![
            // the same set written twice: one centre
            order(vec![c(vec![o(0), o(1)]), c(vec![o(1), o(0)])]),
            // a point and the centre of the set it is the only member of
            order(vec![o(0), c(vec![c(vec![o(0)])])]),
            // the instance and the centre of its drain and its source, (0, 2) and (0, −2): at its
            // origin whatever its orientation
            order(vec![o(2), c(vec![pin(2, 1), pin(2, 2)])]),
            // the centre of its drain and its gate is 1 or 1.5 off its origin: an ORDER holds
            order(vec![o(2), c(vec![pin(2, 1), pin(2, 0)])]),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!((r.failed.as_slice(), r.kept.as_slice()), (&[0, 1, 2][..], &[3][..]));
}

#[test]
fn the_rest_is_resolved_as_without_the_facts_that_fail() {
    // the example of the brainstorm, with a fact that fails among its facts: it reaches two pins
    // of an instance that a later fact reaches too, so that the model without it has other
    // variables; the rest of the resolution is that of the problem without it, the indices of the
    // facts after it one higher
    let at = |line: u32| Some(rnl_runtime::diag::Loc::new("cases/t.sp", line, 3));
    let mut facts = vec![
        place(o(0), Axis::Y, 0.0),
        distance(Sense::Either, Some(2.0), None, vec![o(0)], vec![o(1)]),
        align(Axis::Y, vec![pin(2, 1), pin(2, 0)]),
        place(o(1), Axis::Y, 3.0),
        align(Axis::Y, vec![o(0), o(1)]),
        place(pin(2, 1), Axis::Y, 1.5),
        align(Axis::X, vec![o(0), o(1)]),
        align(Axis::Y, vec![pin(2, 2), o(1)]),
    ];
    for (k, f) in facts.iter_mut().enumerate() {
        f.source = at(10 + k as u32);
    }
    facts[0].priority = Priority::High;
    facts[6].priority = Priority::Low;
    let p = problem_of(vec![point(), point(), instance()], facts);
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.failed, [2]);
    let q = without(&p, 2);
    let s = check(&q, &SolverOptions::without_time_limit());
    let up = |f: usize| if f < 2 { f } else { f + 1 };
    let ups = |fs: &[usize]| fs.iter().map(|&f| up(f)).collect::<Vec<_>>();
    assert_eq!(r.kept, ups(&s.kept));
    let moved: Vec<Discarded> = s
        .discarded
        .iter()
        .map(|d| Discarded { fact: up(d.fact), groups: d.groups.iter().map(|g| ups(g)).collect(), ..d.clone() })
        .collect();
    assert_eq!(r.discarded, moved);
    assert!(!r.discarded.is_empty() && r.discarded.iter().all(|d| !d.in_groups().contains(&2)));
    let hints: BTreeMap<Statement, StatementHint> = s
        .statements
        .iter()
        .map(|(k, h)| (k.clone(), StatementHint { facts: ups(&h.facts), discarded: ups(&h.discarded) }))
        .collect();
    assert_eq!(r.statements, hints);
    assert!(!r.statements.contains_key(&Statement::of(&p, 2)));
    assert_eq!((&r.witness, r.as_written), (&s.witness, s.as_written));
    assert_eq!(r.placing.exact, s.placing.exact.iter().map(|&f| up(f)).collect());
    assert_eq!(r.warnings, s.warnings);
    assert_eq!(r.errors.len(), 1);
    // the orientation of the instance is the one of the problem without it
    assert_eq!(r.orientation(2), s.orientation(2));
}

#[test]
fn a_fact_whose_own_check_runs_out_of_time_is_undecided_not_failed() {
    // the check of a fact on its own is checking (the draft 0.3, 8.5): with the time out before it
    // finishes, the fact is discarded undecided, with its warning, and does not fail
    let p = problem_of(
        vec![instance(), point()],
        vec![place(o(1), Axis::X, 4.0), align(Axis::Y, vec![pin(0, 1), pin(0, 0)]), place(o(1), Axis::Y, 2.0)],
    );
    let (whole, reads) = check_timed(&p, 1e9);
    assert_eq!((whole.failed.as_slice(), whole.kept.as_slice()), (&[1][..], &[0, 2][..]));
    let mut cut = false;
    for limit in 0..=reads {
        let (r, _) = check_timed(&p, limit as f64);
        // never kept: it fails, or is undecided
        assert!(!r.kept.contains(&1), "{limit} s");
        let undecided = r.discarded.iter().any(|d| d.fact == 1 && d.undecided);
        assert_ne!(undecided, r.failed == [1], "{limit} s: {r:?}");
        if undecided {
            cut = true;
            assert!(r.errors.is_empty(), "{limit} s");
            assert!(r.warnings.iter().any(|w| w.starts_with("the fact [1] ")), "{limit} s: {:?}", r.warnings);
        }
    }
    assert!(cut, "no limit cuts the check of the ALIGN on its own");
    // with no time at all, every fact that needs a check is undecided: none fails
    let (r, _) = check_timed(&p, 0.0);
    let undecided: Vec<usize> = r.discarded.iter().filter(|d| d.undecided).map(|d| d.fact).collect();
    assert_eq!((undecided.as_slice(), r.failed.as_slice()), (&[0, 1, 2][..], &[][..]));
}

#[test]
fn the_tolerance_widens_constants_only() {
    // A at 0, B at 1.0000005: ORDER needs 1, which the tolerance of 1e-6 grants; an ALIGN of A
    // and B against PLACEs 5e-7 apart is no closer than exact
    let p = problem_of(
        points(4),
        vec![
            place(o(0), Axis::X, 0.0),
            place(o(1), Axis::X, 0.9999995),
            fact(FactKind::Order { axis: Axis::X, sense: Sense::Increasing, members: vec![o(0), o(1)] }),
            place(o(2), Axis::X, 0.0),
            place(o(3), Axis::X, 0.0000005),
            align(Axis::X, vec![o(2), o(3)]),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.kept, vec![0, 1, 2, 3, 4, 5]);
    // with no tolerance, both conflict
    let r = check(&p, &SolverOptions { tolerance: 0.0, ..SolverOptions::without_time_limit() });
    assert_eq!(r.discarded, vec![discarded(2, vec![0, 1]), discarded(5, vec![3, 4])]);
}

#[test]
fn the_witness_keeps_the_constants_as_written() {
    // the tolerance decides conflicts; the witness sits on the values written where it can
    let p = problem_of(
        points(2),
        vec![
            place(o(0), Axis::X, 4.0),
            distance(Sense::Increasing, Some(2.0), None, vec![o(0)], vec![o(1)]),
            place(o(1), Axis::Y, -1.5),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.position(0), Some((4.0, 0.0)));
    assert_eq!(r.position(1), Some((6.0, -1.5)));
}

#[test]
fn within_the_tolerance_the_witness_keeps_the_constants_of_the_facts_that_rank_first() {
    // B at x 0.4 and A at x 0, on one x, hold together only within 0.5: the constant of the fact
    // that ranks first is held as written, the other within the tolerance (11.7, 8.5)
    let options = SolverOptions { tolerance: 0.5, ..SolverOptions::without_time_limit() };
    let b = from_f64(0.4).unwrap();
    let p = problem_of(
        points(2),
        vec![align(Axis::X, vec![o(0), o(1)]), place(o(1), Axis::X, 0.4), place(o(0), Axis::X, 0.0)],
    );
    let r = check(&p, &options);
    assert_eq!(r.kept, vec![0, 1, 2]);
    assert!(!r.as_written);
    assert_eq!((x_of(&r, 0), x_of(&r, 1)), (b.clone(), b));
    assert_eq!(r.placing.exact, BTreeSet::from([1]));
    // A's 0 first
    let p = problem_of(
        points(2),
        vec![align(Axis::X, vec![o(0), o(1)]), place(o(0), Axis::X, 0.0), place(o(1), Axis::X, 0.4)],
    );
    let r = check(&p, &options);
    assert_eq!((x_of(&r, 0), x_of(&r, 1)), (Q::zero(), Q::zero()));
    assert_eq!(r.placing.exact, BTreeSet::from([1]));
    // what holds as written is held so whole: no fact is singled out
    let p = problem_of(points(2), vec![place(o(1), Axis::X, 0.4), place(o(0), Axis::X, 0.0)]);
    let r = check(&p, &options);
    assert!(r.as_written && r.placing.exact.is_empty() && r.placing.tol.is_zero());
}

/// Instances chained gate to origin on x, the first origin at 0, the last at 1: each link adds
/// −3, 0 or 3, so no orientations reach 1.
fn chain(n: usize) -> LevelProblem {
    let objects: Vec<ProblemObject> = (0..n).map(|_| instance()).collect();
    let mut facts = Vec::new();
    for k in 0..n - 1 {
        facts.push(align(Axis::X, vec![pin(k, 0), o(k + 1)]));
    }
    facts.push(place(o(0), Axis::X, 0.0));
    facts.push(place(o(n - 1), Axis::X, 1.0));
    problem_of(objects, facts)
}

#[test]
fn orientations_that_give_the_same_constants_are_tried_once() {
    // four classes matter, three values each (the gate's x is −3, 0 or 3): 81 combinations at most
    let p = chain(5);
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.discarded, vec![discarded(5, vec![0, 1, 2, 3, 4])]);
    let main = resolve(&p, &SolverOptions::without_time_limit());
    assert!(main.work.resolve_nodes < 2000, "{} nodes", main.work.resolve_nodes);
}

// ---------------------------------------------------------------------------- the time limit

/// Options without a time limit but `seconds`, which [`Ticks`] counts in readings.
fn within(seconds: f64) -> SolverOptions {
    SolverOptions { time_limit: Some(seconds), ..SolverOptions::without_time_limit() }
}

/// Resolves with a time limit of `seconds` on [`Ticks`] and checks what the draft 0.3 says of it
/// (8.5, 7.6): every fact kept or discarded; those kept hold at the witness; each fact discarded
/// undecided has no group and a warning, in order; each other one has groups of facts kept before
/// it, each conflicting with it and minimal, all of them only when it says so; nothing is noted;
/// and a fresh clock gives the same again. Gives the resolution and the readings.
fn check_timed(problem: &LevelProblem, seconds: f64) -> (Resolved, u64) {
    let clock = Ticks::default();
    let r = super::resolve(problem, &within(seconds), &clock);
    let mut all: Vec<usize> = r.kept.clone();
    all.extend(r.discarded.iter().map(|d| d.fact));
    all.extend(&r.failed);
    all.sort_unstable();
    assert_eq!(all, (0..problem.facts.len()).collect::<Vec<_>>());
    let tol = r.model.tol.clone();
    for &f in &r.kept {
        assert!(semantic_holds(problem, &problem.facts[f].kind, &r.witness.geometry, &tol), "{seconds} s: fact {f}");
    }
    let undecided: Vec<&Discarded> = r.discarded.iter().filter(|d| d.undecided).collect();
    assert_eq!(r.warnings.len(), undecided.len(), "{seconds} s: {:?}", r.warnings);
    assert_eq!(r.errors.len(), r.failed.len(), "{seconds} s: {:?}", r.errors);
    for (w, d) in r.warnings.iter().zip(&undecided) {
        let end = format!(" was not decided within the time limit of {} s and is discarded (8.5)", fmt_num(seconds));
        assert!(w.starts_with(&format!("the fact [{}] {} ", d.fact, problem.facts[d.fact].label)), "{w}");
        assert!(w.ends_with(&end), "{w}");
        assert!(d.groups.is_empty() && !d.complete, "{d:?}");
    }
    let n = problem.objects.len();
    for d in r.discarded.iter().filter(|d| !d.undecided) {
        assert!(d.groups.windows(2).all(|w| w[0] < w[1]), "{seconds} s: {d:?}");
        assert!(!d.complete || !d.groups.is_empty(), "{seconds} s: {d:?}");
        for g in &d.groups {
            check_group(&r, n, d.fact, g, &format!("{seconds} s: {d:?}"));
        }
    }
    assert_eq!(r.statements, statement_hints(problem, &r.discarded), "{seconds} s");
    assert!(r.notes.is_empty(), "{seconds} s: {:?}", r.notes);
    assert_eq!(r, super::resolve(problem, &within(seconds), &Ticks::default()), "{seconds} s: not the same twice");
    (r, clock.reads())
}

#[test]
fn with_no_time_at_all_only_what_the_witness_satisfies_is_kept() {
    // the time is out at the first check: a fact that needs one is discarded undecided, a fact that
    // every object at the origin satisfies is kept without one (the draft 0.3, 8.5)
    let mut p = problem_of(
        points(3),
        vec![
            place(o(0), Axis::X, 0.0),
            place(o(1), Axis::X, 5.0),
            align(Axis::X, vec![o(0), o(2)]),
            place(o(2), Axis::Y, 0.0),
            distance(Sense::Either, Some(2.0), None, vec![o(1)], vec![o(2)]),
        ],
    );
    p.facts[4].source = Some(rnl_runtime::diag::Loc::new("cases/t.sp", 12, 5));
    let (r, _) = check_timed(&p, 0.0);
    assert_eq!(r.kept, vec![0, 2, 3]);
    let undecided: Vec<usize> = r.discarded.iter().filter(|d| d.undecided).map(|d| d.fact).collect();
    assert_eq!(undecided, vec![1, 4]);
    // each warning names its fact, with where its statement is written when it has a place
    let label = &p.facts[1].label;
    assert_eq!(
        r.warnings,
        [
            format!("the fact [1] {label} was not decided within the time limit of 0 s and is discarded (8.5)"),
            "the fact [4] DISTANCE (t.sp:12) was not decided within the time limit of 0 s and is discarded (8.5)"
                .to_string(),
        ]
    );
    assert_eq!(r.work.resolve_nodes, 0);
    // the witness is taken as written all the same, which is not timed: the facts kept hold so
    assert!(r.as_written && r.placing.tol.is_zero());
    assert!(r.notes.is_empty(), "{:?}", r.notes);
    // without a limit every fact is decided, and the same on a clock that never advances
    let all = resolve(&p, &SolverOptions::without_time_limit());
    assert_eq!(all.kept, vec![0, 1, 2, 3, 4]);
    assert!(all.warnings.is_empty());
    assert_eq!(resolve(&p, &within(0.0)), all);
}

#[test]
fn a_check_out_of_time_leaves_its_fact_undecided() {
    // the ORDER takes the witness's direction; the last PLACE needs the other, found by a search
    let p = problem_of(
        points(2),
        vec![
            fact(FactKind::Order { axis: Axis::X, sense: Sense::Either, members: vec![o(0), o(1)] }),
            place(o(0), Axis::X, 5.0),
            place(o(1), Axis::X, 0.0),
        ],
    );
    let unlimited = resolve(&p, &SolverOptions::without_time_limit());
    assert_eq!(unlimited.kept, vec![0, 1, 2]);
    let (_, reads) = check_timed(&p, 1e9);
    let mut cut = false;
    for limit in 0..=reads {
        let (r, _) = check_timed(&p, limit as f64);
        cut |= r.kept == [0, 1]
            && r.discarded == [Discarded { fact: 2, groups: Vec::new(), complete: false, undecided: true }];
        if r.warnings.is_empty() {
            // whenever no check reaches the limit, the facts kept are those of 8.5
            assert_eq!(r.kept, unlimited.kept, "{limit} s");
        }
    }
    assert!(cut, "no limit stops the search of the last PLACE alone");
    // with time enough, the result is the one without a limit
    assert_eq!(check_timed(&p, reads as f64).0, unlimited);
}

#[test]
fn with_a_fact_left_undecided_the_witness_is_still_taken_as_written() {
    // A at 5.5, decided; then B at 10, whose check the limit cuts: the witness is taken as written
    // all the same, which is not timed, so A is at 5.5 and not a tolerance below, where the simplex
    // stopped, and the record writes the constant of the file (17)
    let p = problem_of(points(2), vec![place(o(0), Axis::X, 5.5), place(o(1), Axis::X, 10.0)]);
    let (_, reads) = check_timed(&p, 1e9);
    let mut cut = false;
    for limit in 0..=reads {
        let (r, _) = check_timed(&p, limit as f64);
        if r.kept == [0] && r.discarded.iter().any(|d| d.fact == 1 && d.undecided) {
            cut = true;
            assert!(r.as_written, "{limit} s");
            assert_eq!(x_of(&r, 0), frac(11, 2), "{limit} s");
            assert_eq!(r.position(0), Some((5.5, 0.0)), "{limit} s");
        }
    }
    assert!(cut, "no limit decides A and leaves B undecided");
}

#[test]
fn a_search_that_grows_too_large_stops_at_its_time_limit() {
    // nine classes: 3^9 combinations, a search too long for 2000 readings; then a point, whose y
    // at 0 the witness satisfies and whose x at 7 needs a check
    let mut p = chain(10);
    p.objects.push(point());
    p.facts.push(place(o(10), Axis::Y, 0.0));
    p.facts.push(place(o(10), Axis::X, 7.0));
    let (r, reads) = check_timed(&p, 2000.0);
    assert_eq!(r.kept, (0..10).chain([11]).collect::<Vec<_>>());
    let undecided: Vec<usize> = r.discarded.iter().filter(|d| d.undecided).map(|d| d.fact).collect();
    assert_eq!(undecided, vec![10, 12]);
    assert_eq!(r.warnings.len(), 2);
    // a reading at each node and every few pivots: the checks, each fact on its own first, stop
    // within the limit's readings; the witness taken as written after them, which is not timed,
    // adds a node or so for each fact kept
    assert!(r.as_written, "the facts kept hold as written");
    assert!(r.work.resolve_nodes <= 2000 + 20 && reads == 2002, "{} nodes, {reads} readings", r.work.resolve_nodes);
}

#[test]
fn the_time_limit_covers_the_conflict_groups_but_not_the_witness() {
    // the example of the brainstorm: once every fact is decided, the time goes on to the minimal
    // conflict groups alone (the draft 0.3, 8.5); the witness taken as written is counted in work,
    // not timed. A limit that cuts the groups keeps the facts of 8.5 and their witness
    let mut facts = vec![
        place(o(0), Axis::Y, 0.0),
        distance(Sense::Either, Some(2.0), None, vec![o(0)], vec![o(1)]),
        place(o(1), Axis::Y, 3.0),
        align(Axis::Y, vec![o(0), o(1)]),
        align(Axis::X, vec![o(0), o(1)]),
        place(o(1), Axis::X, 0.5),
    ];
    facts[0].priority = Priority::High;
    let p = problem_of(points(2), facts);
    let unlimited = resolve(&p, &SolverOptions::without_time_limit());
    assert_eq!(unlimited.kept, vec![0, 1, 2, 5]);
    assert!(unlimited.discarded.iter().all(|d| d.complete), "{:?}", unlimited.discarded);
    let (_, reads) = check_timed(&p, 1e9);
    let mut cut = false;
    for limit in 0..=reads {
        let (r, _) = check_timed(&p, limit as f64);
        if !r.warnings.is_empty() {
            continue;
        }
        // every fact decided: what is kept, and the witness, are those of 8.5
        assert_eq!(r.kept, unlimited.kept, "{limit} s");
        assert_eq!(r.discarded.iter().map(|d| d.fact).collect::<Vec<_>>(), [3, 4], "{limit} s");
        assert_eq!(r.witness, unlimited.witness, "{limit} s");
        assert_eq!((r.as_written, &r.placing), (unlimited.as_written, &unlimited.placing), "{limit} s");
        // groups the limit cut say so, and are among those of 8.5; the others are those of 8.5
        cut |= r.discarded.iter().any(|d| !d.complete);
        for (d, u) in r.discarded.iter().zip(&unlimited.discarded) {
            if d.complete {
                assert_eq!(d, u, "{limit} s");
            } else {
                assert!(d.groups.iter().all(|g| u.groups.contains(g)), "{limit} s: {d:?}");
            }
        }
    }
    assert!(cut, "the limit never cut the conflict groups");
    assert_eq!(check_timed(&p, reads as f64).0, unlimited);
}

// ---------------------------------------------------------------------------- every group, and the hints by statement

#[test]
fn a_fact_with_two_minimal_groups_names_both() {
    // A.y = 0, B.y = 5 and C.y = 5 kept; ALIGN y {A B C} has no solution with A and B, nor with A
    // and C, and the hint names both groups (the draft 0.3, 8.5)
    let p = problem_of(
        points(3),
        vec![
            place(o(0), Axis::Y, 0.0),
            place(o(1), Axis::Y, 5.0),
            place(o(2), Axis::Y, 5.0),
            align(Axis::Y, vec![o(0), o(1), o(2)]),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.kept, vec![0, 1, 2]);
    assert_eq!(r.discarded, vec![discarded_with(3, vec![vec![0, 1], vec![0, 2]])]);
    assert_eq!(r.discarded[0].first_group(), Some(&[0, 1][..]));
    assert_eq!(r.discarded[0].in_groups(), vec![0, 1, 2]);
}

/// Three points apart on x, at 0, 10 and 20, then an ALIGN x of them all: each pair of the
/// PLACEs is a minimal conflict group of the ALIGN.
fn three_apart() -> LevelProblem {
    problem_of(
        points(3),
        vec![
            place(o(0), Axis::X, 0.0),
            place(o(1), Axis::X, 10.0),
            place(o(2), Axis::X, 20.0),
            align(Axis::X, vec![o(0), o(1), o(2)]),
        ],
    )
}

#[test]
fn every_pair_of_points_placed_apart_is_a_group_of_their_align() {
    let r = check(&three_apart(), &SolverOptions::without_time_limit());
    assert_eq!(r.discarded, vec![discarded_with(3, vec![vec![0, 1], vec![0, 2], vec![1, 2]])]);
}

#[test]
fn groups_of_different_sizes_that_share_a_fact() {
    // A at 0, B 5 right of A, C on A's x, B 5 right of C, A's y at 1; then B at 7: B is at 5 by A
    // and its DISTANCE, and by A, C and theirs, and A's y is in no group
    let exactly =
        |from: usize, to: usize| distance(Sense::Increasing, Some(5.0), Some(5.0), vec![o(from)], vec![o(to)]);
    let p = problem_of(
        points(3),
        vec![
            place(o(0), Axis::X, 0.0),
            exactly(0, 1),
            align(Axis::X, vec![o(0), o(2)]),
            exactly(2, 1),
            place(o(0), Axis::Y, 1.0),
            place(o(1), Axis::X, 7.0),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.kept, vec![0, 1, 2, 3, 4]);
    assert_eq!(r.discarded, vec![discarded_with(5, vec![vec![0, 1], vec![0, 2, 3]])]);
}

#[test]
fn a_limit_that_cuts_the_enumeration_gives_the_groups_found_and_says_so() {
    // three groups to find: a limit that runs out after the first, or the second, gives those, each
    // minimal, and says that the list is not complete (the draft 0.3, 8.5)
    let p = three_apart();
    let unlimited = resolve(&p, &SolverOptions::without_time_limit());
    let all = &unlimited.discarded[0];
    let (_, reads) = check_timed(&p, 1e9);
    let mut cut = Vec::new();
    for limit in 0..=reads {
        let (r, _) = check_timed(&p, limit as f64);
        if !r.warnings.is_empty() {
            continue;
        }
        assert_eq!(r.kept, unlimited.kept, "{limit} s");
        let d = &r.discarded[0];
        if d.complete {
            assert_eq!(d, all, "{limit} s");
        } else {
            assert!(d.groups.iter().all(|g| all.groups.contains(g)), "{limit} s: {d:?}");
            cut.push(d.groups.len());
        }
        // the hints by statement name the statements of the groups found
        let named: Vec<usize> = r.statements.values().flat_map(|h| h.facts.clone()).collect();
        assert_eq!(named, d.in_groups(), "{limit} s");
    }
    // cut before the first group, after it, and after the second
    assert!(cut.contains(&0) && cut.contains(&1) && cut.contains(&2), "{cut:?}");
    assert_eq!(check_timed(&p, reads as f64).0, unlimited);
}

/// `k` independent conflicts: for each of `k` points, a DISTANCE that puts A exactly 5 to its
/// left and a PLACE of it at 0, then A at 0. Each pair is a minimal conflict group of A's PLACE,
/// and the largest sets of the pairs that fit with it, one fact of each pair left out, are 2^k.
fn independent_pairs(k: usize) -> LevelProblem {
    let mut facts = Vec::new();
    for i in 1..=k {
        facts.push(distance(Sense::Increasing, Some(5.0), Some(5.0), vec![o(0)], vec![o(i)]));
        facts.push(place(o(i), Axis::X, 0.0));
    }
    facts.push(place(o(0), Axis::X, 0.0));
    problem_of(points(k + 1), facts)
}

#[test]
fn the_map_of_the_enumeration_propagates_its_clauses() {
    // The map of MARCO for 8 independent groups of two, each seed answered as a resolution would:
    // a seed that holds a pair whole holds a group, the others fit. Every seed is a largest subset
    // that holds no group found and is no subset of a set found to fit; the map gives the 2^8 sets
    // that fit and the 8 groups, then none. Unit propagation over its two kinds of clauses, a
    // group of which a seed leaves one out and a set found to fit outside which it holds one, and
    // a search that goes on from the last seed keep the work small: a search from the start of
    // each seed that tries each position in and out, and finds a clause broken only once every
    // position of it has a value, made a million nodes.
    let k = 8;
    let mut map = Map::new(2 * k, vec![0, 1]);
    let (mut groups, mut fits) = (1, 0);
    loop {
        let seed = match map.seed(Limits::Caps(CAPS)) {
            Seed::Found(seed) => seed,
            Seed::None => break,
            Seed::Stopped => panic!("no time limit"),
        };
        assert!(map.groups.iter().all(|g| !g.iter().all(|i| seed.contains(i))), "{seed:?}");
        assert!(map.fits.iter().all(|s| s.iter().any(|i| seed.contains(i))), "{seed:?}");
        match (0..k).find(|i| seed.contains(&(2 * i)) && seed.contains(&(2 * i + 1))) {
            Some(i) => {
                map.add_group(vec![2 * i, 2 * i + 1]);
                groups += 1;
            }
            None => {
                // as large as can be: one of each pair
                assert_eq!(seed.len(), k, "{seed:?}");
                map.add_fit((0..2 * k).filter(|j| !seed.contains(j)).collect());
                fits += 1;
            }
        }
    }
    assert_eq!((groups, fits), (k, 1 << k));
    // about 3400 nodes; a search from the start of each seed without propagation made a million
    assert!(map.nodes <= 20_000, "{} nodes", map.nodes);
}

#[test]
fn the_map_gives_the_first_subset_not_explored_as_a_search_from_the_start_would() {
    // Random maps against brute force: hidden minimal groups over a few positions answer each seed
    // as a resolution would, the first hidden group that the seed holds whole being found, and the
    // seed is checked against the first subset in the order of the search (positions in before
    // out, the first position first) that holds no group found and is no subset of a set found to
    // fit, grown as the map grows it: the propagation and the search going on from the last seed
    // change the work, not the seeds.
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = move |n: u64| {
        state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) % n
    };
    let reference = |map: &Map| -> Option<Vec<usize>> {
        let n = map.size;
        let first = (0..1u32 << n).rev().find(|mask| {
            let has = |i: usize| mask >> (n - 1 - i) & 1 == 1;
            map.groups.iter().all(|g| !g.iter().all(|&i| has(i))) && map.fits.iter().all(|s| s.iter().any(|&i| has(i)))
        })?;
        let mut seed: Vec<bool> = (0..n).map(|i| first >> (n - 1 - i) & 1 == 1).collect();
        for i in 0..n {
            if !seed[i] {
                seed[i] = true;
                if map.groups.iter().any(|g| g.contains(&i) && g.iter().all(|&j| seed[j])) {
                    seed[i] = false;
                }
            }
        }
        Some((0..n).filter(|&i| seed[i]).collect())
    };
    let mut maps = 0;
    for _ in 0..300 {
        let n = 2 + next(9) as usize;
        let mut hidden: Vec<Vec<usize>> = Vec::new();
        for _ in 0..1 + next(4) {
            let mut g: Vec<usize> = (0..n).filter(|_| next(3) == 0).collect();
            if g.is_empty() {
                g.push(next(n as u64) as usize);
            }
            // minimal: none holds another
            if hidden.iter().all(|h| !h.iter().all(|i| g.contains(i)) && !g.iter().all(|i| h.contains(i))) {
                hidden.push(g);
            }
        }
        let mut map = Map::new(n, hidden[0].clone());
        let mut found = 1;
        loop {
            let want = reference(&map);
            let seed = match map.seed(Limits::Caps(CAPS)) {
                Seed::Found(seed) => Some(seed),
                Seed::None => None,
                Seed::Stopped => panic!("no time limit"),
            };
            assert_eq!(seed, want, "{hidden:?}: {:?} / {:?}", map.groups, map.fits);
            let Some(seed) = seed else { break };
            match hidden.iter().find(|h| h.iter().all(|i| seed.contains(i))) {
                Some(h) => {
                    map.add_group(h.clone());
                    found += 1;
                }
                None => map.add_fit((0..n).filter(|i| !seed.contains(i)).collect()),
            }
        }
        assert_eq!(found, hidden.len(), "{hidden:?}");
        maps += 1;
    }
    assert_eq!(maps, 300);
}

#[test]
fn independent_conflicts_have_all_their_groups() {
    // the resolution itself: the 8 groups, each a pair, found among the 2^8 largest sets that fit
    let k = 8;
    let p = independent_pairs(k);
    let r = resolve(&p, &SolverOptions::without_time_limit());
    let groups: Vec<Vec<usize>> = (0..k).map(|i| vec![2 * i, 2 * i + 1]).collect();
    assert_eq!(r.discarded, vec![discarded_with(2 * k, groups)]);
    // the nodes of the seeds are counted with those of the resolutions
    assert!(r.work.resolve_nodes <= 20_000, "{} nodes", r.work.resolve_nodes);
}

#[test]
fn each_statement_is_told_what_was_discarded_because_of_it() {
    // A's PLACE writes its x and its y on one line; the x of B and the y of B come from one line of a
    // definition, expanded by two usages; a point's coordinate has no statement of its own. ALIGN x
    // {A B} and ALIGN y {A B} conflict with them, and a PLACE of the point's x with its coordinate
    let at = |line: u32| Some(rnl_runtime::diag::Loc::new("cases/t.sp", line, 3));
    let mut facts = vec![
        place(o(0), Axis::X, 0.0),
        place(o(0), Axis::Y, 0.0),
        place(o(1), Axis::X, 4.0),
        place(o(1), Axis::Y, 4.0),
        fact(FactKind::PointCoordinate { point: 2, axis: Axis::X, value: 9.0 }),
        align(Axis::X, vec![o(0), o(1)]),
        align(Axis::Y, vec![o(0), o(1)]),
        place(o(2), Axis::X, 1.0),
    ];
    for f in [0, 1] {
        facts[f].source = at(5);
    }
    for (f, usage) in [(2, 0), (3, 1)] {
        facts[f].source = Some(rnl_runtime::diag::Loc::new("lib/def.rnl", 7, 5));
        facts[f].chain = vec![usage];
    }
    facts[5].source = at(8);
    facts[6].source = at(9);
    facts[7].source = at(10);
    let p = problem_of(points(3), facts);
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.discarded, vec![discarded(5, vec![0, 2]), discarded(6, vec![1, 3]), discarded(7, vec![4])]);
    let statement = |file: &str, line: u32, col: u32, chain: Vec<usize>, point: Option<usize>| Statement {
        file: file.to_string(),
        line,
        col,
        chain,
        point,
    };
    let hint = |facts: Vec<usize>, discarded: Vec<usize>| StatementHint { facts, discarded };
    assert_eq!(
        r.statements,
        BTreeMap::from([
            (statement("", 0, 0, vec![], Some(2)), hint(vec![4], vec![7])),
            (statement("cases/t.sp", 5, 3, vec![], None), hint(vec![0, 1], vec![5, 6])),
            (statement("lib/def.rnl", 7, 5, vec![0], None), hint(vec![2], vec![5])),
            (statement("lib/def.rnl", 7, 5, vec![1], None), hint(vec![3], vec![6])),
        ])
    );
}

// ---------------------------------------------------------------------------- against brute force

/// A fixed xorshift sequence, so every run tests the same problems.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Whether some units have a solution together, decided apart from the search: every
/// orientation of every object tried, every pin linked to its origin whatever reaches it, every
/// value of every choice and of the extremes of every centre a unit constrains tried, each
/// combination checked by a simplex of its own. None when there are too many combinations.
fn brute_feasible(model: &Model, problem: &LevelProblem, units: &[usize]) -> Option<bool> {
    let oriented: Vec<usize> = (0..problem.objects.len()).filter(|&k| model.oriented[k]).collect();
    // the literals: the bounds of each value
    let mut lits: Vec<Vec<Vec<Bound>>> = Vec::new();
    for &u in units {
        for c in &model.units[u].choices {
            lits.push(c.values.clone());
        }
    }
    for axis in 0..2 {
        let mut centres: Vec<usize> = units.iter().flat_map(|&u| model.units[u].centres[axis].clone()).collect();
        centres.sort_unstable();
        centres.dedup();
        for c in centres {
            for high in [false, true] {
                let n = model.centres[c].members.len();
                lits.push((0..n).map(|j| vec![model.centre_extreme(c, axis, high, j)]).collect());
            }
        }
    }
    let combos: usize = lits.iter().map(|l| l.len()).product::<usize>() * 8usize.pow(oriented.len() as u32);
    if combos > 1 << 16 {
        return None;
    }
    for code in 0..8usize.pow(oriented.len() as u32) {
        let mut g = vec![Orient::R0; problem.objects.len()];
        for (i, &k) in oriented.iter().enumerate() {
            g[k] = from_index(((code >> (3 * i)) & 7) as u8);
        }
        let orientations_hold = units.iter().all(|&u| match &model.units[u].orient {
            OrientEffect::None => true,
            OrientEffect::Fix { object, code } => g[*object] == *code,
            OrientEffect::Relate { relation, from, to } => relation_holds(&g, *relation, from, to),
        });
        if !orientations_hold {
            continue;
        }
        let mut pick = vec![0usize; lits.len()];
        loop {
            let mut sx: [Simplex<usize>; 2] = [Simplex::new(), Simplex::new()];
            for s in &mut sx {
                for _ in 0..model.vars.len() {
                    s.new_var();
                }
            }
            let mut ok = true;
            let mut put = |b: &Bound, sx: &mut [Simplex<usize>; 2]| {
                let s = &mut sx[b.axis];
                if let Some(lo) = &b.lo {
                    ok &= s.assert(&b.expr, Rel::Ge, lo, 0).is_ok();
                }
                if let Some(hi) = &b.hi {
                    ok &= s.assert(&b.expr, Rel::Le, hi, 0).is_ok();
                }
            };
            for c in 0..model.centres.len() {
                for axis in 0..2 {
                    for b in model.centre_definition(c, axis) {
                        put(&b, &mut sx);
                    }
                }
            }
            for (p, info) in &model.points {
                if let (Some(k), Some(local)) = (info.object, &info.local) {
                    if let Some(origin) = model.origin_point[k] {
                        if model.oriented[k] && origin != *p {
                            for axis in (0..2).filter(|&a| info.linked(a)) {
                                let c = turned(g[k], local, axis);
                                let e = LinExpr::diff(info.var, model.var(origin));
                                put(&Bound { axis, expr: e, lo: Some(c.clone()), hi: Some(c) }, &mut sx);
                            }
                        }
                    }
                }
            }
            for &u in units {
                for b in &model.units[u].fixed {
                    put(b, &mut sx);
                }
            }
            for (l, &v) in lits.iter().zip(&pick) {
                for b in &l[v] {
                    put(b, &mut sx);
                }
            }
            if ok && sx[0].check().is_ok() && sx[1].check().is_ok() {
                return Some(true);
            }
            // the next combination
            let mut i = 0;
            while i < pick.len() {
                pick[i] += 1;
                if pick[i] < lits[i].len() {
                    break;
                }
                pick[i] = 0;
                i += 1;
            }
            if i == pick.len() {
                break;
            }
        }
    }
    Some(false)
}

/// A random problem: three points and one or two instances, eight facts of every kind.
fn random_problem(rng: &mut Rng) -> LevelProblem {
    let instances = 1 + rng.below(2);
    let mut objects = points(3);
    objects.extend((0..instances).map(|_| instance()));
    let refs = |rng: &mut Rng, centres: bool| -> PointRef {
        match rng.below(if centres { 7 } else { 6 }) {
            0..=2 => o(rng.below(3)),
            3 => o(3 + rng.below(instances)),
            4 | 5 => pin(3 + rng.below(instances), rng.below(3)),
            _ => PointRef::Centre((0..2 + rng.below(2)).map(|_| o(rng.below(3))).collect()),
        }
    };
    let senses = [Sense::Increasing, Sense::Decreasing, Sense::Either];
    let mut facts = Vec::new();
    for _ in 0..8 {
        let axis = if rng.below(2) == 0 { Axis::X } else { Axis::Y };
        let kind = match rng.below(9) {
            0 | 1 => FactKind::Place { at: refs(rng, true), axis, value: rng.below(9) as f64 - 4.0 },
            2 => FactKind::Align { axis, members: vec![refs(rng, true), refs(rng, true)] },
            3 => FactKind::Order {
                axis,
                sense: senses[rng.below(3)],
                members: (0..2 + rng.below(2)).map(|_| refs(rng, true)).collect(),
            },
            4 => FactKind::Distribute { axis, members: (0..3).map(|_| refs(rng, false)).collect() },
            5 | 6 => {
                let min = (rng.below(3) > 0).then(|| rng.below(4) as f64);
                let max = if min.is_none() || rng.below(2) == 0 {
                    Some(min.unwrap_or(0.0) + rng.below(4) as f64)
                } else {
                    None
                };
                FactKind::Distance {
                    axis,
                    sense: senses[rng.below(3)],
                    min,
                    max,
                    from: (0..1 + rng.below(2)).map(|_| refs(rng, false)).collect(),
                    to: (0..1 + rng.below(2)).map(|_| refs(rng, false)).collect(),
                }
            }
            7 => FactKind::Orient { object: 3 + rng.below(instances), code: Orient::ALL[rng.below(8)] },
            _ if instances == 2 => {
                FactKind::OrientRelation { relation: Orient::ALL[rng.below(8)], from: vec![3], to: vec![4] }
            }
            _ => FactKind::Place { at: refs(rng, false), axis, value: rng.below(9) as f64 - 4.0 },
        };
        facts.push(fact(kind));
    }
    problem_of(objects, facts)
}

#[test]
fn the_resolution_agrees_with_brute_force() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let options = SolverOptions::without_time_limit();
    let mut discards = 0;
    let mut failed = 0;
    let mut skipped = 0;
    for case in 0..150 {
        let p = random_problem(&mut rng);
        let r = check(&p, &options);
        // a fact fails when it has no solution on its own, on the model of every fact (the draft
        // 0.3, 8.5); the others are taken in on the model without those that fail
        let whole = Model::new(&p, r.model.tol.clone());
        let mut kept: Vec<usize> = Vec::new();
        let mut decided = true;
        for f in 0..p.facts.len() {
            if let Some(alone) = brute_feasible(&whole, &p, &[f]) {
                assert_eq!(alone, !r.failed.contains(&f), "case {case}: fact {f} on its own");
            }
            if r.failed.contains(&f) {
                failed += 1;
                continue;
            }
            let mut with = kept.clone();
            with.push(f);
            match brute_feasible(&r.model, &p, &with) {
                Some(true) => kept = with,
                Some(false) => {}
                None => {
                    decided = false;
                    break;
                }
            }
        }
        if !decided {
            // too many combinations to try them all: the problem is left out
            skipped += 1;
            continue;
        }
        assert_eq!(r.kept, kept, "case {case}: {:#?}", p.facts.iter().map(|f| &f.kind).collect::<Vec<_>>());
        discards += r.discarded.len();
    }
    // the problems conflict often enough to test the discards too, some facts fail, and few are
    // left out
    assert!(discards > 150 && failed > 0, "{discards} discarded, {failed} failed");
    assert!(skipped < 10, "{skipped}");
}
