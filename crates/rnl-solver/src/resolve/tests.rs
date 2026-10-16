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

fn discarded(fact: usize, conflicts_with: Vec<usize>) -> Discarded {
    Discarded { fact, conflicts_with, minimal: true, undecided: false }
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
/// fact kept, the model's own test agrees with the standard's on every fact, each conflict set is
/// made of facts kept before its fact, conflicts with it and is minimal, and a second run gives
/// the same.
fn check(problem: &LevelProblem, options: &SolverOptions) -> Resolved {
    let r = resolve(problem, options);
    let tol = r.model.tol.clone();
    let g = &r.witness.geometry;
    let mut all: Vec<usize> = r.kept.clone();
    all.extend(r.discarded.iter().map(|d| d.fact));
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
        assert_eq!(probe.holds(f), semantic_holds(problem, &pf.kind, g, &tol), "fact {f}: {:?}", pf.kind);
    }
    for d in r.discarded.iter().filter(|d| !d.undecided) {
        assert!(d.conflicts_with.iter().all(|c| *c < d.fact && r.kept.contains(c)), "{d:?}");
        let fits = |set: &[usize]| {
            let mut s = Resolver::new(&r.model, problem.objects.len(), Limits::Caps(CAPS));
            assert!(set.iter().all(|&c| s.add(c) == Outcome::Kept));
            s.add(d.fact) == Outcome::Kept
        };
        assert!(!fits(&d.conflicts_with), "{d:?} does not conflict");
        for c in &d.conflicts_with {
            let rest: Vec<usize> = d.conflicts_with.iter().copied().filter(|x| x != c).collect();
            assert!(fits(&rest), "{d:?} is not minimal: {c} is not needed");
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
    assert_eq!(r.discarded, vec![discarded(3, vec![0, 2]), discarded(4, vec![1])]);
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
    assert_eq!(r.discarded, vec![discarded(3, vec![2])]);
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
    assert_eq!(r.discarded.len(), 1);
    assert_eq!(r.discarded[0].fact, 4);
    assert!([vec![0, 1], vec![0, 2]].contains(&r.discarded[0].conflicts_with), "{:?}", r.discarded);
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
fn a_fact_that_cannot_hold_alone_conflicts_with_nothing() {
    let p = problem_of(
        points(2),
        vec![
            fact(FactKind::Order { axis: Axis::X, sense: Sense::Either, members: vec![o(0), o(0)] }),
            distance(Sense::Either, Some(1.0), None, vec![o(1)], vec![o(1)]),
            align(Axis::X, vec![o(1)]),
        ],
    );
    let r = check(&p, &SolverOptions::without_time_limit());
    assert_eq!(r.kept, vec![2]);
    assert_eq!(r.discarded, vec![discarded(0, vec![]), discarded(1, vec![])]);
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
/// undecided has no conflict set and a warning, in order; each other one conflicts with facts kept
/// before it, with a set that is minimal unless it says otherwise; nothing is noted; and a fresh
/// clock gives the same again. Gives the resolution and the readings.
fn check_timed(problem: &LevelProblem, seconds: f64) -> (Resolved, u64) {
    let clock = Ticks::default();
    let r = super::resolve(problem, &within(seconds), &clock);
    let mut all: Vec<usize> = r.kept.clone();
    all.extend(r.discarded.iter().map(|d| d.fact));
    all.sort_unstable();
    assert_eq!(all, (0..problem.facts.len()).collect::<Vec<_>>());
    let tol = r.model.tol.clone();
    for &f in &r.kept {
        assert!(semantic_holds(problem, &problem.facts[f].kind, &r.witness.geometry, &tol), "{seconds} s: fact {f}");
    }
    let undecided: Vec<&Discarded> = r.discarded.iter().filter(|d| d.undecided).collect();
    assert_eq!(r.warnings.len(), undecided.len(), "{seconds} s: {:?}", r.warnings);
    for (w, d) in r.warnings.iter().zip(&undecided) {
        let end = format!(" was not decided within the time limit of {} s and is discarded (8.5)", fmt_num(seconds));
        assert!(w.starts_with(&format!("the fact [{}] {} ", d.fact, problem.facts[d.fact].label)), "{w}");
        assert!(w.ends_with(&end), "{w}");
        assert!(d.conflicts_with.is_empty() && !d.minimal, "{d:?}");
    }
    let n = problem.objects.len();
    let conflicts = |set: &[usize], f: usize| {
        let mut s = Resolver::new(&r.model, n, Limits::Caps(CAPS));
        assert!(set.iter().all(|&c| s.add(c) == Outcome::Kept), "{seconds} s: {set:?} do not fit");
        matches!(s.add(f), Outcome::Conflict(_))
    };
    for d in r.discarded.iter().filter(|d| !d.undecided) {
        assert!(d.conflicts_with.iter().all(|c| *c < d.fact && r.kept.contains(c)), "{seconds} s: {d:?}");
        assert!(conflicts(&d.conflicts_with, d.fact), "{seconds} s: {d:?} does not conflict");
        if d.minimal {
            for c in &d.conflicts_with {
                let rest: Vec<usize> = d.conflicts_with.iter().copied().filter(|x| x != c).collect();
                assert!(!conflicts(&rest, d.fact), "{seconds} s: {d:?} is not minimal without {c}");
            }
        }
    }
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
    // the witness is the resolution's, within the tolerance: nothing more was resolved
    assert!(!r.as_written && r.placing.tol == r.model.tol);
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
            && r.discarded == [Discarded { fact: 2, conflicts_with: Vec::new(), minimal: false, undecided: true }];
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
    // a reading at each node and every few pivots: the search stops within the limit's readings
    assert!(r.work.resolve_nodes <= 2000 && reads == 2002, "{} nodes, {reads} readings", r.work.resolve_nodes);
}

#[test]
fn the_time_limit_covers_the_conflict_sets_but_not_the_witness() {
    // the example of the brainstorm: once every fact is decided, the time goes on to the minimal
    // conflict sets alone (the draft 0.3, 8.5); the witness taken as written is counted in work,
    // not timed. A limit that cuts the sets keeps the facts of 8.5 and their witness
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
    assert!(unlimited.discarded.iter().all(|d| d.minimal), "{:?}", unlimited.discarded);
    let (_, reads) = check_timed(&p, 1e9);
    let mut sets = false;
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
        // a set the limit cut says so; the others are those of 8.5
        sets |= r.discarded.iter().any(|d| !d.minimal);
        for (d, u) in r.discarded.iter().zip(&unlimited.discarded) {
            if d.minimal {
                assert_eq!(d, u, "{limit} s");
            }
        }
    }
    assert!(sets, "the limit never cut the conflict sets");
    assert_eq!(check_timed(&p, reads as f64).0, unlimited);
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
    let mut skipped = 0;
    for case in 0..150 {
        let p = random_problem(&mut rng);
        let r = check(&p, &options);
        let mut kept: Vec<usize> = Vec::new();
        let mut decided = true;
        for f in 0..p.facts.len() {
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
    // the problems conflict often enough to test the discards too, and few are left out
    assert!(discards > 150, "{discards}");
    assert!(skipped < 10, "{skipped}");
}
