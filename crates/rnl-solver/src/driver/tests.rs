//! Tests of a solve: small levels built in code, then every level of the cases written for the
//! solver and of the conformance corpus read from the files.

use super::*;
use crate::resolve::tests::{fact, instance, label, point, problem_of, Ticks};
use crate::Objectives;
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{Axis, FactKind, PointRef};

#[test]
fn an_empty_level_gives_an_empty_solution_and_says_what_is_done() {
    let p = problem_of(Vec::new(), Vec::new());
    let solved = crate::solve(&p, &SolverOptions::without_time_limit());
    assert_eq!(solved.solution, Solution::default());
    assert_eq!(solved.report.notes[0], METHOD_NOTE);
    assert!(solved.report.kept.is_empty() && solved.report.discarded.is_empty());
    assert_eq!(solved, crate::solve(&p, &SolverOptions::without_time_limit()));
}

#[test]
fn options_that_make_no_sense_are_noted() {
    let mut options = SolverOptions { beam: 0, ..SolverOptions::without_time_limit() };
    options.objectives = Objectives::none();
    let solved = crate::solve(&problem_of(Vec::new(), Vec::new()), &options);
    assert!(solved.report.notes.iter().any(|n| n.contains("beam")));
}

#[test]
fn every_object_is_placed_where_the_facts_put_it() {
    let p = problem_of(
        vec![instance(), point()],
        vec![
            fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 4.0 }),
            fact(FactKind::Orient { object: 0, code: Orient::MY }),
            fact(FactKind::Align { axis: Axis::Y, members: vec![PointRef::Origin(1), PointRef::Origin(0)] }),
            fact(FactKind::Place { at: PointRef::Origin(1), axis: Axis::Y, value: 2.5 }),
        ],
    );
    let solved = crate::solve(&p, &SolverOptions::without_time_limit());
    assert_eq!(solved.report.kept, vec![0, 1, 2, 3]);
    assert_eq!(
        solved.solution.placements,
        vec![
            Placement { object: 0, x: Some(4.0), y: Some(2.5), orient: Some(Orient::MY) },
            Placement { object: 1, x: Some(0.0), y: Some(2.5), orient: None },
        ]
    );
    assert!(solved.report.work.resolve_nodes > 0);
}

/// A position that the file fixes is given all the same (plan 3.2), and the record writes it for
/// an object it reaches, so that it ranks with the record's priority between two solves (the draft
/// 0.3, 14.8). A point without a name, which the record of a file of 0.2 cannot refer to, is not
/// unsolved where the solution has it where its coordinates put it, a viewer drawing it there all
/// the same, and is where the solver discarded one of them.
#[test]
fn a_position_the_file_fixes_is_given_and_written_where_the_record_reaches_it() {
    use rnl_runtime::problem::{ProblemObject, Reference, Slot};
    let fixed = |value: f64, fact: usize| Slot::Fixed { value, fact };
    let m = ProblemObject { position: Position::Own { x: fixed(-10.0, 2), y: fixed(2.0, 3) }, ..instance() };
    let anonymous = |label: &str, x: Slot, y: Slot| ProblemObject {
        label: label.to_string(),
        reference: Reference::Anonymous,
        position: Position::Own { x, y },
        ..point()
    };
    let n = ProblemObject {
        label: "#N".to_string(),
        reference: Reference::Name("#N".to_string()),
        position: Position::Own { x: fixed(9.0, 6), y: fixed(-10.0, 7) },
        ..instance()
    };
    let p = problem_of(
        vec![
            m,
            anonymous("(3, 4)", fixed(3.0, 4), fixed(4.0, 5)),
            n,
            anonymous("(7, 8)", fixed(7.0, 8), fixed(8.0, 9)),
        ],
        vec![
            fact(FactKind::Align { axis: Axis::X, members: vec![PointRef::Origin(2), PointRef::Origin(1)] }),
            fact(FactKind::Align { axis: Axis::X, members: vec![PointRef::Origin(3), PointRef::Origin(0)] }),
            fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: -10.0 }),
            fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::Y, value: 2.0 }),
            fact(FactKind::PointCoordinate { point: 1, axis: Axis::X, value: 3.0 }),
            fact(FactKind::PointCoordinate { point: 1, axis: Axis::Y, value: 4.0 }),
            fact(FactKind::Place { at: PointRef::Origin(2), axis: Axis::X, value: 9.0 }),
            fact(FactKind::Place { at: PointRef::Origin(2), axis: Axis::Y, value: -10.0 }),
            fact(FactKind::PointCoordinate { point: 3, axis: Axis::X, value: 7.0 }),
            fact(FactKind::PointCoordinate { point: 3, axis: Axis::Y, value: 8.0 }),
        ],
    );
    let solved = crate::solve(&p, &SolverOptions::without_time_limit());
    assert_eq!(solved.report.kept, vec![0, 1, 2, 3, 4, 5, 7, 9]);
    let at: Vec<(usize, Option<(f64, f64)>)> =
        solved.solution.placements.iter().map(|pl| (pl.object, pl.at())).collect();
    assert_eq!(
        at,
        vec![(0, Some((-10.0, 2.0))), (1, Some((3.0, 4.0))), (2, Some((3.0, -10.0))), (3, Some((-10.0, 8.0)))]
    );
    assert!(solved.solution.placements[0].orient.is_some());
    let written = rnl_runtime::solution::write_record(&p, &solved.solution, "t", "h");
    let unsolved: Vec<String> = written.unsolved.iter().map(|u| u.to_string()).collect();
    assert_eq!(unsolved.len(), 1, "{unsolved:?}");
    // its y is where the file fixes it, so only its x is unsolved
    assert!(unsolved[0].starts_with("the x of (7, 8): "), "{unsolved:?}");
    assert!(written.record.contains("#M @PLACE(-10, 2) => @ORIENT("), "{}", written.record);
    assert!(written.record.contains("#N @PLACE(3, -10) => @ORIENT("), "{}", written.record);
}

#[test]
fn a_text_that_an_offset_places_on_one_axis_follows_its_owner_there_only() {
    // the dx of #M::name replaces the x its symbol gives it (13.3): wherever M goes, the text is 3
    // right of it, its y still from the symbol, turned with M; the solution places M only
    let p = problem_of(
        vec![instance(), label(Some(Axis::X)), point()],
        vec![
            fact(FactKind::Offset { text: 1, from: PointRef::Origin(0), axis: Axis::X, value: 3.0 }),
            fact(FactKind::PointCoordinate { point: 2, axis: Axis::X, value: 10.0 }),
            fact(FactKind::PointCoordinate { point: 2, axis: Axis::Y, value: 4.0 }),
            fact(FactKind::Align { axis: Axis::X, members: vec![PointRef::Origin(0), PointRef::Origin(2)] }),
            fact(FactKind::Orient { object: 0, code: Orient::R180 }),
        ],
    );
    let (solved, e) = check_level("offset", &p);
    assert_eq!(solved.report.kept, vec![0, 1, 2, 3, 4]);
    let g = &e.placed.geometry;
    let at = |r: &PointRef, axis: usize| point_value(&p, g, r, axis, 0).unwrap();
    assert_eq!(at(&PointRef::Origin(0), 0), int(10));
    assert_eq!(at(&PointRef::Origin(1), 0), int(13));
    // R180 turns (1, 1) to (-1, -1)
    assert_eq!(at(&PointRef::Origin(1), 1), at(&PointRef::Origin(0), 1) - int(1));
    // the solution gives the text where it is; the record leaves its x to the offset and its y to
    // the symbol, which still place it there (13.3)
    let text: Vec<&Placement> = solved.solution.placements.iter().filter(|pl| pl.object == 1).collect();
    assert_eq!(text.len(), 1, "{:?}", solved.solution.placements);
    assert_eq!(text[0].x, Some(13.0));
    let written = rnl_runtime::solution::write_record(&p, &solved.solution, "t", "h");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert!(written.record.contains("#M @PLACE(10, ") && !written.record.contains("#M::name"), "{}", written.record);
}

/// A text whose offset the solver discards (8.5) is where the solver puts it, and the record writes
/// that axis, which replaces the offset there (13.3): a label of its owner's symbol whose `dy` the
/// scene gives, and a text of its own with `dx` and `dy`, which keeps its `dx`.
#[test]
fn a_text_whose_offset_is_discarded_is_written_on_that_axis() {
    use rnl_runtime::problem::{ProblemObject, Reference, Slot};
    let fixed = |value: f64, fact: usize| Slot::Fixed { value, fact };
    let m = ProblemObject { position: Position::Own { x: fixed(0.0, 2), y: fixed(0.0, 3) }, ..instance() };
    let n = ProblemObject {
        label: "#N".to_string(),
        reference: Reference::Name("#N".to_string()),
        position: Position::Own { x: fixed(10.0, 4), y: fixed(10.0, 5) },
        ..instance()
    };
    let name = label(Some(Axis::Y));
    let note = ProblemObject {
        label: "#M::note".to_string(),
        reference: Reference::Name("#M::note".to_string()),
        position: Position::Own { x: Slot::Free, y: Slot::Free },
        ..label(None)
    };
    let p = problem_of(
        vec![m, name, n, note],
        vec![
            fact(FactKind::Align { axis: Axis::Y, members: vec![PointRef::Origin(1), PointRef::Origin(2)] }),
            fact(FactKind::Align { axis: Axis::Y, members: vec![PointRef::Origin(3), PointRef::Origin(2)] }),
            fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 0.0 }),
            fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::Y, value: 0.0 }),
            fact(FactKind::Place { at: PointRef::Origin(2), axis: Axis::X, value: 10.0 }),
            fact(FactKind::Place { at: PointRef::Origin(2), axis: Axis::Y, value: 10.0 }),
            fact(FactKind::Offset { text: 1, from: PointRef::Origin(0), axis: Axis::Y, value: 2.0 }),
            fact(FactKind::Offset { text: 3, from: PointRef::Origin(0), axis: Axis::X, value: -1.0 }),
            fact(FactKind::Offset { text: 3, from: PointRef::Origin(0), axis: Axis::Y, value: 2.0 }),
        ],
    );
    let (solved, _) = check_level("discarded offsets", &p);
    let discarded: Vec<usize> = solved.report.discarded.iter().map(|d| d.fact).collect();
    assert_eq!(discarded, vec![6, 8]);
    let written = rnl_runtime::solution::write_record(&p, &solved.solution, "t", "h");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    for line in ["#M::name @PLACE(y=10)", "#M::note @PLACE(y=10)"] {
        assert!(written.record.contains(line), "{}", written.record);
    }
    assert!(!written.record.contains("#M::note @PLACE(-1"), "{}", written.record);
}

// ------------------------------------------------------------------------------------ the files

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use num_traits::{One, Signed, Zero};
use rnl_runtime::problem::{LevelProblem, ObjectKind, Sense};
use rnl_runtime::source::Provider;

use crate::rational::{from_f64, int};
use crate::resolve::point_value;
use crate::search::{orientation_relations, with_relations, Relation, Tagged};
use crate::structure::{PortKind, Role};

/// Files from the disk, as the CLI reads them. The crate itself reads none.
struct Files;

impl Provider for Files {
    fn read(&self, path: &str) -> Option<String> {
        let bytes = std::fs::read(path).ok()?;
        Some(match String::from_utf8(bytes) {
            Ok(s) => s.strip_prefix('\u{feff}').map(String::from).unwrap_or(s),
            Err(e) => e.into_bytes().iter().map(|&b| b as char).collect(),
        })
    }
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn problems(path: &Path) -> Vec<LevelProblem> {
    let entry = path.to_string_lossy().replace('\\', "/");
    rnl_view::interpret(&entry, None, &Files, &rnl_runtime::Options::solver())
        .unwrap_or_else(|e| panic!("{entry}: {e}"))
        .problems
}

/// The netlists of a directory of the repository, by name.
fn netlists(dir: &str, prefix: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(repo().join(dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            name.starts_with(prefix) && name.ends_with(".sp")
        })
        .collect();
    out.sort();
    out
}

/// Whether a fact holds under a placement, exactly, as the standard says (14.1–14.5, 14.9): the
/// constants within `tol` (11.7), the equalities between points (ALIGN, DISTRIBUTE) exact, as the
/// model keeps them.
fn holds_exactly(p: &LevelProblem, g: &crate::resolve::Geometry, kind: &FactKind, tol: &Q) -> bool {
    let at = |m: &PointRef, axis: Axis| {
        let a = usize::from(axis == Axis::Y);
        point_value(p, g, m, a, 0).unwrap_or_else(|| panic!("no point for {m:?}"))
    };
    let num = |v: f64| from_f64(v).unwrap();
    let ori = |k: usize| g.orient[k];
    let near = |a: Q, b: Q| (a - b).abs() <= *tol;
    match kind {
        FactKind::Place { at: m, axis, value } => near(at(m, *axis), num(*value)),
        FactKind::PointCoordinate { point, axis, value } => near(at(&PointRef::Origin(*point), *axis), num(*value)),
        FactKind::Offset { text, from, axis, value } => {
            near(at(&PointRef::Origin(*text), *axis) - at(from, *axis), num(*value))
        }
        FactKind::Orient { object, code } => ori(*object) == *code,
        FactKind::OrientRelation { relation, from, to } => {
            let (Some(a), Some(b)) = (from.first(), to.first()) else { return true };
            from.iter().all(|k| ori(*k) == ori(*a))
                && to.iter().all(|k| ori(*k) == ori(*b))
                && ori(*b) == ori(*a).then(*relation)
        }
        FactKind::Align { axis, members } => {
            let v: Vec<Q> = members.iter().map(|m| at(m, *axis)).collect();
            v.iter().all(|x| *x == v[0])
        }
        FactKind::Order { axis, sense, members } => {
            let v: Vec<Q> = members.iter().map(|m| at(m, *axis)).collect();
            let step = Q::one() - tol;
            let inc = v.windows(2).all(|w| &w[1] - &w[0] >= step);
            let dec = v.windows(2).all(|w| &w[0] - &w[1] >= step);
            match sense {
                Sense::Increasing => inc,
                Sense::Decreasing => dec,
                Sense::Either => inc || dec,
            }
        }
        FactKind::Distribute { axis, members } => {
            let v: Vec<Q> = members.iter().map(|m| at(m, *axis)).collect();
            v.len() < 3 || v.windows(2).all(|w| &w[1] - &w[0] == &v[1] - &v[0])
        }
        FactKind::Distance { axis, sense, min, max, from, to } => {
            if from.is_empty() || to.is_empty() {
                return true;
            }
            let extent = |ms: &[PointRef]| {
                let v: Vec<Q> = ms.iter().map(|m| at(m, *axis)).collect();
                (v.iter().min().unwrap().clone(), v.iter().max().unwrap().clone())
            };
            let (a, b) = (extent(from), extent(to));
            let up = &b.0 - &a.1;
            let down = &a.0 - &b.1;
            let gaps = match sense {
                Sense::Increasing => vec![up],
                Sense::Decreasing => vec![down],
                Sense::Either => vec![up, down],
            };
            // "x" or "y": one side has the lower bound; the upper bound holds of the largest gap
            let largest = gaps.iter().max().unwrap().clone();
            min.is_none_or(|m| gaps.iter().any(|g| *g >= num(m) - tol)) && max.is_none_or(|m| largest <= num(m) + tol)
        }
    }
}

/// Whether the facts kept leave no side to put two symbols apart, with the orientations the
/// solve gave every free class: each of the four sides, touching allowed, is dropped.
fn forced_overlap(e: &Exact, a: usize, b: usize) -> bool {
    // with the caps of the implementation, whatever caps the solve had
    let s = &Structure { caps: CAPS, ..e.structure.clone() };
    let ori = &e.placed.geometry.orient;
    let values: Vec<Orient> = s.open.classes.iter().map(|c| ori[c.root]).collect();
    let candidate = crate::search::Candidate { values, ..e.candidate.clone() };
    let base = orientation_relations(s, &candidate);
    let (pa, pb) = (s.model.origin_point[a].unwrap(), s.model.origin_point[b].unwrap());
    let (ba, bb) = (s.box_offset(a, ori[a]).unwrap(), s.box_offset(b, ori[b]).unwrap());
    let one = Q::one;
    let sides = [
        (0, vec![(pb, one()), (pa, -one())], &ba[2] - &bb[0]),
        (0, vec![(pa, one()), (pb, -one())], &bb[2] - &ba[0]),
        (1, vec![(pb, one()), (pa, -one())], &ba[3] - &bb[1]),
        (1, vec![(pa, one()), (pb, -one())], &bb[3] - &ba[1]),
    ];
    sides.into_iter().all(|(axis, terms, c)| {
        let mut rels = base.clone();
        rels.push(Tagged {
            kind: crate::search::RelKind::Separation,
            rel: Relation::Linear { axis, terms, lo: Some(c), hi: None },
        });
        let mut work = crate::Work::default();
        with_relations(s, &rels, &mut work, |_, _, t| !t.kept[t.kept.len() - 1])
    })
}

fn summary(p: &LevelProblem, solved: &Solved, e: &Exact) -> String {
    let mut lines = vec![format!(
        "{}: {} objects, kept {}/{} facts, candidate {}, score {:?}, {} candidates, {} pivots",
        p.title,
        p.objects.len(),
        solved.report.kept.len(),
        p.facts.len(),
        e.candidate.origin,
        e.score.iter().map(crate::rational::fmt).collect::<Vec<_>>(),
        solved.report.work.candidates,
        solved.report.work.pivots
    )];
    for pl in &solved.solution.placements {
        let o = &p.objects[pl.object];
        if matches!(o.kind, ObjectKind::Text) {
            continue;
        }
        lines.push(format!("  {:>24} {:?} {:?}", o.label, pl.at(), pl.orient.map(|x| x.as_str())));
    }
    for n in &solved.report.notes[1..] {
        lines.push(format!("  note: {n}"));
    }
    for n in &solved.report.inert_objectives {
        lines.push(format!("  inert: {n}"));
    }
    for w in &solved.solution.wires {
        let points: Vec<String> = w
            .points
            .iter()
            .map(|q| match q {
                rnl_runtime::solution::WirePoint::Anchor(PointRef::Pin { object, terminal, .. }) => {
                    format!("{}:{terminal}", p.objects[*object].label)
                }
                rnl_runtime::solution::WirePoint::Anchor(a) => format!("{a:?}"),
                rnl_runtime::solution::WirePoint::Branch(b) => format!("branch {b}"),
                rnl_runtime::solution::WirePoint::Bend((x, y)) => format!("({x}, {y})"),
            })
            .collect();
        lines.push(format!("  wire &{}: {}", p.routes[w.route].net, points.join(" ")));
    }
    lines.join("\n")
}

/// The wires of the ROUTEs (13.7, 13.2): each from a pin of its net, a point of a wire written on
/// it or a branch point, to another, its bends between, as the exact paths have them; along the
/// axes and through no symbol's box; every branch point the end of three wires or more, or of two
/// on a pin or a point, one of them the wire of length 0 to it, with a junction; every net one
/// tree but those the report leaves in parts, in as many as it says.
fn check_wires(p: &LevelProblem, solved: &Solved, e: &Exact, what: &str) {
    use rnl_runtime::solution::WirePoint;
    let s = &e.structure;
    let g = &e.placed.geometry;
    let sol = &solved.solution;
    let r = &e.routing;
    assert_eq!((&sol.wires, &sol.branches), (&r.wires, &r.branches), "{what}");
    assert_eq!(r.paths.len(), sol.wires.len(), "{what}");
    let at = |q: &PointRef| [point_value(p, g, q, 0, 0).unwrap(), point_value(p, g, q, 1, 0).unwrap()];
    let boxes: Vec<(usize, [Q; 4])> =
        s.components().filter_map(|k| Some((k, s.box_at(&e.placed.origins, &g.orient, k)?))).collect();
    let mut ends_at = vec![0usize; sol.branches.len()];
    let mut on_anchor = vec![false; sol.branches.len()];
    for (w, path) in sol.wires.iter().zip(&r.paths) {
        if let [WirePoint::Branch(b), WirePoint::Anchor(_)] = w.points.as_slice() {
            on_anchor[*b] |= path.len() == 2 && path[0] == path[1];
        }
        let what = format!("{what}: {w:?}");
        let net = &p.routes[w.route].net;
        assert_eq!(p.routes.iter().position(|x| x.net == *net), Some(w.route), "{what}: the first ROUTE of its net");
        let routes: Vec<&rnl_runtime::problem::RouteIntent> = p.routes.iter().filter(|x| x.net == *net).collect();
        let n = w.points.len();
        assert!(n >= 2 && path.len() == n, "{what}");
        for (i, q) in w.points.iter().enumerate() {
            let end = i == 0 || i == n - 1;
            match q {
                WirePoint::Anchor(a) => {
                    assert!(end, "{what}");
                    let pin = routes.iter().any(|x| x.pins.contains(a));
                    let written = routes.iter().flat_map(|x| &x.wires).any(|&k| p.wires[k].anchors.contains(a));
                    assert!(pin || written, "{what}: {a:?} is not a point of the net");
                    assert_eq!(at(a), path[i], "{what}");
                }
                WirePoint::Branch(b) => {
                    assert!(end, "{what}");
                    ends_at[*b] += 1;
                    let xy = sol.branches[*b].at;
                    assert_eq!(
                        (from_f64(xy.0).unwrap(), from_f64(xy.1).unwrap()),
                        (path[i][0].clone(), path[i][1].clone())
                    );
                }
                WirePoint::Bend(xy) => {
                    assert!(!end && xy.0.is_finite() && xy.1.is_finite(), "{what}");
                    assert_eq!(*xy, (crate::rational::to_f64(&path[i][0]), crate::rational::to_f64(&path[i][1])));
                }
            }
        }
        // a pin strictly inside its own box may lead out through it
        let inside_own = |k: usize, b: &[Q; 4]| {
            [&w.points[0], &w.points[n - 1]].iter().any(|q| match q {
                WirePoint::Anchor(a @ PointRef::Pin { object, .. }) if *object == k => {
                    let xy = at(a);
                    b[0] < xy[0] && xy[0] < b[2] && b[1] < xy[1] && xy[1] < b[3]
                }
                _ => false,
            })
        };
        for seg in path.windows(2) {
            assert!(seg[0][0] == seg[1][0] || seg[0][1] == seg[1][1], "{what}: not along an axis");
            for (k, b) in &boxes {
                assert!(
                    !crate::route::meets_inside(b, &seg[0], &seg[1]) || inside_own(*k, b),
                    "{what}: through the box of {}",
                    p.objects[*k].label
                );
            }
        }
    }
    for (b, n) in ends_at.iter().enumerate() {
        assert!(
            (*n >= 3 || (*n == 2 && on_anchor[b])) && sol.branches[b].junction,
            "{what}: branch point {b} ends {n} wires"
        );
    }
    // no T without a junction: a pin at which a wire routed ends ends no other wire of its net,
    // routed or written, and a point ends fewer than three; the wire of length 0 from a branch
    // point on it is the junction's
    let mut ends: Vec<(&String, &PointRef, usize)> = Vec::new();
    for (w, path) in sol.wires.iter().zip(&r.paths) {
        if matches!(w.points[0], WirePoint::Branch(_)) && path.len() == 2 && path[0] == path[1] {
            continue;
        }
        let net = &p.routes[w.route].net;
        for q in [&w.points[0], &w.points[w.points.len() - 1]] {
            if let WirePoint::Anchor(a) = q {
                match ends.iter_mut().find(|(n, b, _)| *n == net && *b == a) {
                    Some(e) => e.2 += 1,
                    None => ends.push((net, a, 1)),
                }
            }
        }
    }
    for (net, a, routed) in ends {
        let mut wires: Vec<usize> = p.routes.iter().filter(|x| x.net == *net).flat_map(|x| x.wires.clone()).collect();
        wires.sort_unstable();
        wires.dedup();
        let mut written = 0;
        for k in wires {
            let anchors = &p.wires[k].anchors;
            for (i, b) in anchors.iter().enumerate() {
                if b == a {
                    written += if i == 0 || i == anchors.len() - 1 { 1 } else { 2 };
                }
            }
        }
        let lead = usize::from(matches!(a, PointRef::Pin { .. }));
        assert!(lead + routed + written < 3, "{what}: {a:?} ends {routed} wires routed and {written} written");
    }
    for x in sol.branches.iter().map(|b| b.at) {
        assert!(x.0.is_finite() && x.1.is_finite(), "{what}");
    }
    // every net one tree, but those left in parts
    let mut nets: Vec<&String> = p.routes.iter().map(|x| &x.net).collect();
    nets.dedup();
    nets.sort();
    nets.dedup();
    for net in nets {
        let mut parent: BTreeMap<(u8, usize, usize), (u8, usize, usize)> = BTreeMap::new();
        fn find(
            parent: &mut BTreeMap<(u8, usize, usize), (u8, usize, usize)>,
            k: (u8, usize, usize),
        ) -> (u8, usize, usize) {
            let mut k = k;
            while let Some(&up) = parent.get(&k) {
                if up == k {
                    break;
                }
                k = up;
            }
            parent.entry(k).or_insert(k);
            k
        }
        let key = |q: &PointRef| match q {
            PointRef::Pin { object, pin, .. } => Some((0u8, *object, *pin)),
            PointRef::Origin(k) if p.objects[*k].kind == ObjectKind::Point => Some((1, *k, 0)),
            _ => None,
        };
        let unite = |parent: &mut BTreeMap<_, _>, a: (u8, usize, usize), b: (u8, usize, usize)| {
            let (ra, rb) = (find(parent, a), find(parent, b));
            if ra != rb {
                parent.insert(ra.max(rb), ra.min(rb));
            }
        };
        let mut points: Vec<(u8, usize, usize)> = Vec::new();
        for x in p.routes.iter().filter(|x| x.net == *net) {
            points.extend(x.pins.iter().filter_map(key));
            for &k in &x.wires {
                let ends: Vec<_> = p.wires[k].anchors.iter().filter_map(key).collect();
                points.extend(ends.iter().copied());
                for pair in ends.windows(2) {
                    unite(&mut parent, pair[0], pair[1]);
                }
            }
        }
        points.sort_unstable();
        points.dedup();
        for w in sol.wires.iter().filter(|w| p.routes[w.route].net == *net) {
            let end = |q: &WirePoint| match q {
                WirePoint::Anchor(a) => key(a),
                WirePoint::Branch(b) => Some((2, *b, 0)),
                WirePoint::Bend(_) => None,
            };
            let (a, b) = (end(&w.points[0]).unwrap(), end(&w.points[w.points.len() - 1]).unwrap());
            unite(&mut parent, a, b);
        }
        let mut roots: Vec<_> = points.iter().map(|q| find(&mut parent, *q)).collect();
        roots.sort_unstable();
        roots.dedup();
        let left = solved.report.unrouted.iter().find(|u| u.net == *net);
        if points.len() < 2 {
            assert!(left.is_none(), "{what}: &{net}");
            continue;
        }
        match left {
            None => assert_eq!(roots.len(), 1, "{what}: &{net} is not one tree"),
            Some(u) => assert_eq!(roots.len(), u.parts, "{what}: &{net}"),
        }
    }
}

/// Solves a level and checks it (see the tests below).
fn check_level(file: &str, p: &LevelProblem) -> (Solved, Exact) {
    check_with(file, p, &SolverOptions::without_time_limit())
}

/// Solves a level with some options and checks it.
fn check_with(file: &str, p: &LevelProblem, options: &SolverOptions) -> (Solved, Exact) {
    check_capped(file, p, options, &StoppedClock, CAPS)
}

/// Solves a level with some options, the time limit on `clock` and the solver's own resolutions
/// within `caps`, and checks it.
fn check_capped(
    file: &str,
    p: &LevelProblem,
    options: &SolverOptions,
    clock: &dyn Clock,
    caps: Caps,
) -> (Solved, Exact) {
    let options = options.clone();
    let (solved, e) = solve_exact_with(p, &options, clock, caps);
    let what = format!("{file} / {}", p.title);
    // a fact discarded undecided, and only such a fact, has its warning (7.6)
    assert_eq!(solved.report.warnings, e.resolved.warnings, "{what}");
    let undecided = solved.report.discarded.iter().filter(|d| d.undecided).count();
    assert_eq!(solved.report.warnings.len(), undecided, "{what}: {:?}", solved.report.warnings);
    // a fact that contradicts itself, and only such a fact, fails with its error (the draft 0.3,
    // 8.5; 7.6)
    assert_eq!((&solved.report.failed, &solved.report.errors), (&e.resolved.failed, &e.resolved.errors), "{what}");
    assert_eq!(solved.report.errors.len(), solved.report.failed.len(), "{what}: {:?}", solved.report.errors);
    // every fact kept holds where the objects are, exactly, before they become f64
    let tol = if e.resolved.as_written { Q::zero() } else { from_f64(options.tolerance).unwrap() };
    for &f in &solved.report.kept {
        assert!(
            holds_exactly(p, &e.placed.geometry, &p.facts[f].kind, &tol),
            "{what}: fact {f} kept but violated: {:?}\n{}",
            p.facts[f],
            summary(p, &solved, &e)
        );
    }
    // no two instances overlap unless the facts make them, or the cap of pivots ran out
    let tight = caps.pivots < CAPS.pivots;
    for o in &e.placed.overlaps {
        let (a, b) = (o.a, o.b);
        let instance = |k: usize| matches!(p.objects[k].kind, ObjectKind::Instance | ObjectKind::Bundle);
        if tight && o.cause == crate::coords::Cause::Budget {
            continue;
        }
        if instance(a) && instance(b) {
            assert_eq!(o.cause, crate::coords::Cause::Facts, "{what}: {o:?}\n{}", summary(p, &solved, &e));
            assert!(
                forced_overlap(&e, a, b),
                "{what}: {} and {} overlap, and the facts allow them apart\n{}",
                p.objects[a].label,
                p.objects[b].label,
                summary(p, &solved, &e)
            );
        }
    }
    // the same again, and nothing that RNL cannot write
    assert_eq!(solved, solve_exact_with(p, &options, clock, caps).0, "{what}: not deterministic");
    for pl in &solved.solution.placements {
        if let Some((x, y)) = pl.at() {
            assert!(x.is_finite() && y.is_finite(), "{what}: {pl:?}");
        }
    }
    // every object with a position of its own placed, those the file fixes too (plan 3.2), but what
    // the solver leaves free (13.3, 11.1), and every text that its symbol places on one axis only,
    // where it is (13.3); and every orientation given, but to what is left free
    for (k, o) in p.objects.iter().enumerate() {
        let pl: Vec<&Placement> = solved.solution.placements.iter().filter(|pl| pl.object == k).collect();
        assert!(pl.len() <= 1, "{what}: {pl:?}");
        let at = pl.first().and_then(|pl| pl.at());
        let orient = pl.first().and_then(|pl| pl.orient);
        let own = match o.position {
            Position::Own { .. } => !left_free(&e.structure, k),
            Position::InSymbol { replaced: Some(_), .. } => true,
            _ => false,
        };
        assert_eq!(at.is_some(), own, "{what}: {} {pl:?}", o.label);
        if let (Some((x, y)), Position::InSymbol { .. }) = (at, &o.position) {
            let origin = &e.placed.origins[k];
            assert_eq!((from_f64(x).as_ref(), from_f64(y).as_ref()), (Some(&origin[0]), Some(&origin[1])), "{what}");
        }
        let oriented =
            o.orient.is_some() && !matches!(o.position, Position::InSymbol { .. }) && !left_free(&e.structure, k);
        assert_eq!(orient.is_some(), oriented, "{what}: {} {pl:?}", o.label);
    }
    check_wires(p, &solved, &e, &what);
    (solved, e)
}

#[test]
fn every_case_of_the_solver_is_placed_keeping_every_fact_kept() {
    let files = netlists("tests/integration/solver", "T");
    assert!(files.len() >= 7, "{files:?}");
    let mut routed = 0;
    for f in files {
        for p in problems(&f) {
            let (solved, e) = check_level(&f.to_string_lossy(), &p);
            eprintln!("{}", summary(&p, &solved, &e));
            // every ROUTE of the cases written for the solver is routed whole
            assert!(solved.report.unrouted.is_empty(), "{}: {:?}", p.title, solved.report.unrouted);
            if !p.routes.is_empty() {
                assert!(!solved.solution.wires.is_empty(), "{}", p.title);
                routed += 1;
            }
        }
    }
    assert!(routed >= 7, "{routed}");
}

/// A budget too small for anything still gives a solution that RNL can write: every object placed
/// and oriented, every fact kept holding, the wires made valid and the rest reported, the same
/// twice. The solver's own resolutions, of the rows, of the relations of the candidates and of the
/// coordinate LP, have caps of two nodes and 30 pivots.
#[test]
fn a_tiny_budget_still_gives_a_valid_solution() {
    let options =
        SolverOptions::without_time_limit().with_overrides("budget=candidates:0,astar_expansions:12").unwrap();
    let caps = Caps { nodes: 2, pivots: 30 };
    let mut left = 0;
    for f in netlists("tests/integration/solver", "T") {
        for p in problems(&f) {
            let (solved, _) = check_capped(&f.to_string_lossy(), &p, &options, &StoppedClock, caps);
            left += solved.report.unrouted.len();
            for u in &solved.report.unrouted {
                assert!(u.parts >= 2, "{}: {u:?}", p.title);
                assert!(solved.report.notes.iter().any(|n| n.contains(&format!("&{} is left in", u.net))));
            }
        }
    }
    assert!(left > 0, "12 expansions leave some net in parts");
}

/// A resolution that runs out of time (the draft 0.3, 8.5), on a clock that counts its readings,
/// still gives a solution that RNL can write, every fact kept holding, a warning for each fact
/// left undecided, the same twice; with no time at all, and with a little.
#[test]
fn a_resolution_out_of_time_still_gives_a_valid_solution() {
    let mut undecided = 0;
    for seconds in [0.0, 25.0] {
        let options = SolverOptions { time_limit: Some(seconds), ..SolverOptions::without_time_limit() };
        for f in netlists("tests/integration/solver", "T") {
            for p in problems(&f) {
                let what = format!("{} at {seconds} s", f.to_string_lossy());
                let (solved, e) = check_capped(&what, &p, &options, &Ticks::default(), CAPS);
                undecided += solved.report.warnings.len();
                // the solver's own work is not timed: the layout is searched and placed as usual
                assert!(e.placed.optimised, "{what}");
                assert!(!solved.report.notes.iter().any(|n| n.contains("a cap of work")), "{what}");
            }
        }
    }
    assert!(undecided > 0, "no time leaves some fact undecided");
}

#[test]
fn every_case_of_the_corpus_is_placed_keeping_every_fact_kept() {
    let files = netlists("tests/conformance/cases", "");
    assert!(files.len() >= 20, "{files:?}");
    for f in files {
        for p in problems(&f) {
            let (solved, e) = check_level(&f.to_string_lossy(), &p);
            eprintln!("{}", summary(&p, &solved, &e));
        }
    }
}

/// The wires written, which their mode draws between the points the placement moves (13.1), do
/// not mislead where a candidate can avoid it (13.2; `crate::score::misleading`): in the RC
/// low-pass filter of the corpus, `&in` and `&out` were drawn on one line through R1, over the
/// pin of V1, when the order of candidates did not count it; now no wire touches one of another
/// net, runs over a pin of another net or through a symbol. The same for the common-source stage,
/// whose `&bias` ran through the symbols of Vdd and Vb, and for their equivalents.
#[test]
fn wires_written_do_not_touch_other_nets_or_run_through_symbols() {
    for file in [
        "tests/conformance/cases/01-rc-lowpass.sp",
        "tests/conformance/cases/02-cs-amplifier.sp",
        "tests/conformance/equivalence/rc-lowpass.sp",
        "tests/conformance/equivalence/cs-amplifier.sp",
    ] {
        for p in problems(&repo().join(file)) {
            let (solved, e) = check_level(file, &p);
            let wires = crate::score::drawn(&p, &e.structure, &e.placed.geometry, &e.routing);
            assert!(!wires.paths.is_empty(), "{file}");
            let n = crate::score::misleading(&p, &e.structure, &e.placed.origins, &e.placed.geometry, &wires);
            assert_eq!(n, 0, "{file}: {n} places where the wires mislead\n{}", summary(&p, &solved, &e));
        }
    }
}

fn object(p: &LevelProblem, label: &str) -> usize {
    p.objects.iter().position(|o| o.label == label).unwrap_or_else(|| panic!("no object {label}"))
}

/// T3, the five-transistor OTA: the picture of plan 5.11.
#[test]
fn t3_is_drawn_as_the_plan_pictures_it() {
    let ps = problems(&repo().join("tests/integration/solver/T3-ota5t.sp"));
    let p = ps.iter().find(|p| p.title == "ota5t").expect("the level of the subcircuit");
    let (solved, e) = check_level("T3", p);
    let what = summary(p, &solved, &e);
    let s = &e.structure;
    let g = &e.placed.geometry;
    let at = &e.placed.origins;
    let [m1, m2, m3, m4, m5] = ["#M1", "#M2", "#M3", "#M4", "#M5"].map(|l| object(p, l));
    let ax = object(p, "%ax");
    let (x, y) = (|k: usize| at[k][0].clone(), |k: usize| at[k][1].clone());
    // three rows by current depth: the mirror on top, the pair in the middle, the tail below
    assert!(y(m3) == y(m4) && y(m1) == y(m2), "{what}");
    assert!(y(m3) > y(m1) && y(m1) > y(m5), "{what}");
    // M3 and M4 mirrored about %ax, and M1 and M2; M5 under the midpoint
    assert_eq!(g.orient[m4], g.orient[m3].then(Orient::MY), "{what}");
    assert_eq!(g.orient[m2], g.orient[m1].then(Orient::MY), "{what}");
    assert!(x(m3) < x(m4) && x(m1) < x(m2), "{what}");
    assert_eq!(x(m3) + x(m4), x(ax) * int(2), "{what}");
    assert_eq!(x(m1) + x(m2), x(ax) * int(2), "{what}");
    assert_eq!(x(m5), x(ax), "{what}");
    // the current runs down: NMOS sources below their drains, PMOS sources above
    let pin = |k: usize, terminal: &str| {
        let j = s.pins(k).iter().position(|pi| pi.terminals.iter().any(|t| t == terminal)).unwrap();
        s.pin_at(at, &g.orient, k, j).unwrap()
    };
    for k in [m1, m2, m5] {
        assert!(pin(k, "3")[1] < pin(k, "1")[1], "{what}: {}", p.objects[k].label);
    }
    for k in [m3, m4] {
        assert!(pin(k, "3")[1] > pin(k, "1")[1], "{what}: {}", p.objects[k].label);
    }
    // the gates: the pair's outwards, the mirror's facing each other, towards the gate net between
    // them. The plan's picture labels M3 R0 and M4 MY; with the gate of $PMOS on its left, that
    // turns the mirror's gates outwards, away from that net. The solver turns pins towards what
    // they connect to, which gives M3 MY and M4 R0, the textbook drawing: a deviation from the
    // labels of the picture, not from its layout
    let gate = |k: usize| s.exit(k, s.pins(k).iter().position(|pi| pi.terminals == ["2"]).unwrap(), g.orient[k]);
    assert_eq!(gate(m1), Some((-1, 0)), "{what}");
    assert_eq!(gate(m2), Some((1, 0)), "{what}");
    assert_eq!(gate(m3), Some((1, 0)), "{what}");
    assert_eq!(gate(m4), Some((-1, 0)), "{what}");
    // the supplies above M3 and M4, the ground below M5
    let supplies: Vec<usize> = (0..p.objects.len()).filter(|&k| s.objects[k].role == Role::Supply).collect();
    let grounds: Vec<usize> = (0..p.objects.len()).filter(|&k| s.objects[k].role == Role::Ground).collect();
    assert_eq!((supplies.len(), grounds.len()), (2, 1), "{what}");
    for k in supplies {
        let host = s.objects[k].stub.as_ref().unwrap().host.0;
        assert!([m3, m4].contains(&host) && y(k) > y(host) && x(k) == x(host), "{what}");
    }
    assert_eq!(s.objects[grounds[0]].stub.as_ref().unwrap().host.0, m5);
    assert!(y(grounds[0]) < y(m5) && x(grounds[0]) == x(m5), "{what}");
    assert_eq!(g.orient[grounds[0]], Orient::R0, "{what}");
    // the ports beside what they connect to: inp left of M1's gate, inn right of M2's, vbias left
    // of M5's gate
    let port = |net: &str| s.ports.iter().find(|q| q.net.as_deref() == Some(net)).unwrap().object;
    let tag = |net: &str| s.pin_at(at, &g.orient, port(net), 0).unwrap();
    for (net, host, terminal, left) in [("inp", m1, "2", true), ("inn", m2, "2", false), ("vbias", m5, "2", true)] {
        let (t, h) = (tag(net), pin(host, terminal));
        assert_eq!(t[1], h[1], "{what}: {net}");
        assert!(if left { t[0] < h[0] } else { t[0] > h[0] }, "{what}: {net}");
    }
    // and out right of M4's drain, which leaves down to M2's: two below it, in the gap between the
    // two rows, above M2's drain (quality review SE-13)
    let (t, top, bottom) = (tag("out"), pin(m4, "1"), pin(m2, "1"));
    assert!(t[0] > top[0], "{what}");
    assert_eq!(t[1], &top[1] - int(2), "{what}");
    assert!(bottom[1] < t[1], "{what}");
    assert_eq!(s.ports.iter().find(|q| q.net.as_deref() == Some("inp")).unwrap().kind, PortKind::Input);
    assert_eq!(s.ports.iter().find(|q| q.net.as_deref() == Some("out")).unwrap().kind, PortKind::Output);
    // nothing overlaps, and every relation of the layout fits but the column of M5 under M1,
    // which the facts that centre M5 leave out
    assert!(e.placed.overlaps.is_empty(), "{what}");
    assert_eq!(e.placed.dropped, vec![crate::search::RelKind::Column], "{what}");
    // on the grid
    for k in [m1, m2, m3, m4, m5] {
        assert!(x(k).is_integer() && y(k).is_integer(), "{what}");
    }
    // the current direction is in force: none of its objectives is inert but those not computed
    assert_eq!(solved.report.inert_objectives.len(), 1, "{:?}", solved.report.inert_objectives);
    assert!(solved.report.inert_objectives[0].starts_with("terminal_wire_alignment"));
}

/// T6 has no supply symbol: the current direction is inert, and the report says why.
#[test]
fn t6_without_a_supply_reports_the_current_direction_inert() {
    let ps = problems(&repo().join("tests/integration/solver/T6-r2r-ladder.sp"));
    assert_eq!(ps.len(), 1);
    let (solved, e) = check_level("T6", &ps[0]);
    let inert = &solved.report.inert_objectives;
    let current = inert.iter().find(|i| i.starts_with("current_direction")).unwrap_or_else(|| panic!("{inert:?}"));
    assert!(current.contains("no supply symbol") && !current.contains("no ground symbol"), "{current}");
    assert!(e.structure.depth.is_none());
    // the ground direction is in force: there are grounds, below their pins
    assert!(!inert.iter().any(|i| i.starts_with("ground_direction")), "{inert:?}");
    for (k, o) in e.structure.objects.iter().enumerate() {
        if o.role == Role::Ground {
            assert_eq!(e.placed.geometry.orient[k], Orient::R0, "{}", ps[0].objects[k].label);
        }
    }
    assert!(!solved.report.inert_objectives.iter().any(|i| i.is_empty()));
}

/// The options change what is searched and how it is ranked, never what holds: with no objective,
/// without a grid, with a beam of one and no budget for candidates, every fact kept still holds
/// exactly, and a solve is still the same twice.
#[test]
fn options_change_the_search_and_never_the_facts() {
    let ps = problems(&repo().join("tests/integration/solver/T3-ota5t.sp"));
    for text in
        ["objectives=none", "grid=none", "beam=1", "budget=candidates:0", "grid=0.5;objectives=area,wire_length"]
    {
        let options = SolverOptions::without_time_limit().with_overrides(text).unwrap();
        for p in &ps {
            let (solved, e) = solve_exact(p, &options);
            let tol = if e.resolved.as_written { Q::zero() } else { from_f64(options.tolerance).unwrap() };
            for &f in &solved.report.kept {
                assert!(holds_exactly(p, &e.placed.geometry, &p.facts[f].kind, &tol), "{text}: {} fact {f}", p.title);
            }
            assert_eq!(solved, crate::solve(p, &options), "{text}: {}", p.title);
            if options.grid.is_none() {
                continue;
            }
            // what the facts leave free is on the grid
            let g = from_f64(options.grid.unwrap()).unwrap();
            for &k in &e.structure.elements {
                for axis in 0..2 {
                    let v = &e.placed.geometry.origin[k][axis];
                    assert!((v / &g).is_integer(), "{text}: {} {} at {v}", p.title, p.objects[k].label);
                }
            }
        }
    }
    // with no objective, the inert ones are not reported either: nothing is configured
    let solved = crate::solve(&ps[0], &SolverOptions::without_time_limit().with_overrides("objectives=none").unwrap());
    assert!(solved.report.inert_objectives.is_empty());
}

/// Tolerances on the layers make "equal" not transitive (0 ≈ 0.6 t ≈ 1.2 t, yet 0 < 1.2 t): the
/// candidates are still ranked in a well-defined order, the same twice, and every fact kept holds.
#[test]
fn layers_with_tolerances_rank_the_candidates_in_a_well_defined_order() {
    let texts = [
        "objectives=component_alignment~2,distribution~1,area~50,wire_length~10,symmetry~1",
        "objectives=current_direction~1,turns~3,ground_direction~1,component_alignment~4,distribution~2,symmetry~1,\
         area~20,crossings~2,wire_length~8,aspect~0.5",
    ];
    for (i, text) in texts.into_iter().enumerate() {
        let options = SolverOptions::without_time_limit().with_overrides(text).unwrap();
        for f in netlists("tests/integration/solver", "T") {
            for p in problems(&f) {
                let (solved, e) = solve_exact(&p, &options);
                let tol = if e.resolved.as_written { Q::zero() } else { from_f64(options.tolerance).unwrap() };
                for &k in &solved.report.kept {
                    assert!(
                        holds_exactly(&p, &e.placed.geometry, &p.facts[k].kind, &tol),
                        "{text}: {} fact {k}",
                        p.title
                    );
                }
                // the same twice, for the first; the second only once, to keep the test short
                if i == 0 {
                    assert_eq!(solved, crate::solve(&p, &options), "{text}: {}", p.title);
                }
            }
        }
    }
}

/// A note on two symbols that overlap says why: with too few pivots, the cap of work; with the
/// facts of corpus 03, which tie CL to M2, the facts.
#[test]
fn an_overlap_is_noted_with_its_cause() {
    let ps = problems(&repo().join("tests/integration/solver/T4-miller-conflicts.sp"));
    let options = SolverOptions::without_time_limit();
    let caps = Caps { pivots: 40, ..CAPS };
    let mut budget = false;
    for p in &ps {
        let (solved, e) = solve_exact_with(p, &options, &StoppedClock, caps);
        let tol = if e.resolved.as_written { Q::zero() } else { from_f64(options.tolerance).unwrap() };
        for &k in &solved.report.kept {
            assert!(holds_exactly(p, &e.placed.geometry, &p.facts[k].kind, &tol), "{} fact {k}", p.title);
        }
        for o in &e.placed.overlaps {
            budget |= o.cause == crate::coords::Cause::Budget;
            assert!(matches!(o.cause, crate::coords::Cause::Budget | crate::coords::Cause::Facts), "{o:?}");
        }
        for n in solved.report.notes.iter().filter(|n| n.contains(" overlap: ")) {
            assert!(!n.contains("the facts kept") || !e.placed.undecided, "{n}");
        }
        if e.placed.undecided {
            assert!(
                solved.report.notes.iter().any(|n| n.contains("a cap of work ran out")),
                "{:?}",
                solved.report.notes
            );
        }
    }
    assert!(budget, "40 pivots leave some symbols overlapping for want of work");
    let ps = problems(&repo().join("tests/conformance/cases/03-five-transistor-ota.sp"));
    let (solved, e) = solve_exact(&ps[0], &SolverOptions::without_time_limit());
    let [m2, cl] = ["#M2", "#CL"].map(|l| object(&ps[0], l));
    assert!(
        e.placed
            .overlaps
            .iter()
            .any(|o| (o.a, o.b) == (m2.min(cl), m2.max(cl)) && o.cause == crate::coords::Cause::Facts),
        "{:?}",
        e.placed.overlaps
    );
    assert!(solved.report.notes.iter().any(|n| n.contains("overlap: the facts kept, with the orientations taken")));
}

// ------------------------------------------------------------------------ netlists given as text

/// The levels of a netlist given as text, read as the CLI reads a file.
fn problems_of(text: &str) -> Vec<LevelProblem> {
    let files = rnl_runtime::source::MemoryProvider::new().with("t.sp", text);
    rnl_view::interpret("t.sp", None, &files, &rnl_runtime::Options::solver()).unwrap().problems
}

/// Solves the one level of a netlist given as text, checked as the cases of the files are.
fn solve_text(text: &str) -> (LevelProblem, Solved, Exact) {
    let ps = problems_of(text);
    assert_eq!(ps.len(), 1, "{text}");
    let p = ps.into_iter().next().unwrap();
    let (solved, e) = check_level("text", &p);
    (p, solved, e)
}

/// A lower bound whose constant is off the grid: the free coordinate goes to the grid point past it,
/// not to the nearer one, which the bound does not allow.
#[test]
fn a_bound_off_the_grid_puts_what_it_moves_on_the_next_grid_point() {
    let (p, solved, e) = solve_text(
        "* g1\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\nR2 a b 1k\n\
         * @RNL { #R1 @PLACE(0, 0); #R1 #R2 @DISTANCE(\"left-to-right\", min=7.4); {#R1 #R2} @ALIGN(\"y\") }\n.end\n",
    );
    let what = summary(&p, &solved, &e);
    let [r1, r2] = ["#R1", "#R2"].map(|l| object(&p, l));
    let at = &e.placed.geometry.origin;
    assert_eq!(at[r1], [int(0), int(0)], "{what}");
    assert!(at[r2][0].is_integer() && at[r2][0] >= crate::rational::frac(37, 5), "{what}");
    assert!(at[r2][1].is_integer(), "{what}");
    // the same with points, which go on the grid after the symbols
    let (p, solved, e) = solve_text(
        "* g2\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\n\
         * @RNL { %a = (0, 0); %b = (_, _); %a %b @DISTANCE(\"left-to-right\", min=0.4) }\n.end\n",
    );
    let what = summary(&p, &solved, &e);
    let b = object(&p, "%b");
    let at = &e.placed.geometry.origin;
    assert!(at[b][0].is_integer() && at[b][0] >= crate::rational::frac(2, 5), "{what}");
    for k in &e.structure.elements {
        assert!(at[*k][0].is_integer() && at[*k][1].is_integer(), "{what}");
    }
}

/// A symbol with a text a half unit off the grid from it: the symbol is on the grid, and the text,
/// which its offset makes follow, is off it.
#[test]
fn a_text_with_an_offset_off_the_grid_leaves_its_symbol_on_the_grid() {
    let (p, solved, e) = solve_text(
        "* h1\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\nR2 b c 1k\n\
         * @RNL { #R1 @TEXT(\"x\", key=\"code\", dx=-2, dy=3.5); #R2 @TEXT(\"y\", key=\"code\", dx=0.5, dy=-1.5) }\n.end\n",
    );
    let what = summary(&p, &solved, &e);
    let at = &e.placed.geometry.origin;
    for &k in &e.structure.elements {
        assert!(at[k][0].is_integer() && at[k][1].is_integer(), "{what}: {}", p.objects[k].label);
    }
    let texts: Vec<usize> = (0..p.objects.len())
        .filter(|&k| {
            matches!(p.objects[k].kind, ObjectKind::Text) && matches!(p.objects[k].position, Position::Own { .. })
        })
        .collect();
    assert_eq!(texts.len(), 2, "{what}");
    assert!(texts.iter().all(|&t| !at[t][1].is_integer()), "{what}");
}

/// A supply and a ground drawn right on the terminals (12.1): their terminals are on the supply and
/// the ground, so the current depth puts the PMOS above the NMOS.
#[test]
fn a_supply_and_a_ground_on_terminals_give_the_current_depth() {
    let (p, solved, e) = solve_text(
        "* s1\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.model nch nmos level=1\n.model pch pmos level=1\n\
         M1 out in vdd vdd pch\nM2 out in vss vss nch\n\
         * @RNL {\n\
         *   # @SELECT-INSTANCE(model=\"nch\") => @SET-SYMBOL $NMOS;\n\
         *   # @SELECT-INSTANCE(model=\"pch\") => @SET-SYMBOL $PMOS;\n\
         *   $SUPPLY @ATTACH #M1:3;\n\
         *   $GND @ATTACH #M2:3;\n\
         *   $WIRE_NET_SET @ROUTE\n\
         * }\n.end\n",
    );
    let what = summary(&p, &solved, &e);
    let s = &e.structure;
    let [m1, m2] = ["#M1", "#M2"].map(|l| object(&p, l));
    let depth = s.depth.as_ref().unwrap_or_else(|| panic!("no current depth: {what}"));
    assert_eq!((depth[m1], depth[m2]), (Some(0), Some(1)), "{what}");
    assert_eq!(s.rows, vec![vec![m1], vec![m2]], "{what}");
    let at = &e.placed.origins;
    assert!(at[m1][1] > at[m2][1], "{what}");
    assert!(!solved.report.inert_objectives.iter().any(|i| i.starts_with("current_direction")), "{what}");
}

/// An ORDER on x is a row (plan 5.4), even when a neighbour of one of its members sits at another
/// height.
#[test]
fn an_order_on_x_is_one_row() {
    let (p, solved, e) = solve_text(
        "* o1\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\nR2 c d 1k\nR3 e f 1k\nR4 g h 1k\nR5 c k 1k\n\
         * @RNL {\n\
         *   {#R1 #R2 #R3 #R4} @ORDER(\"left-to-right\");\n\
         *   #R1 #R5 @DISTANCE(\"bottom-to-top\", min=5);\n\
         *   $WIRE_NET_SET @ROUTE\n\
         * }\n.end\n",
    );
    let what = summary(&p, &solved, &e);
    let [r1, r2, r3, r4, r5] = ["#R1", "#R2", "#R3", "#R4", "#R5"].map(|l| object(&p, l));
    assert!(e.structure.rows.contains(&vec![r1, r2, r3, r4]), "{what}: {:?}", e.structure.rows);
    let y = |k: usize| e.placed.origins[k][1].clone();
    assert!(y(r1) == y(r2) && y(r2) == y(r3) && y(r3) == y(r4), "{what}");
    assert!(y(r5) >= y(r1) + int(5), "{what}");
}

/// An ORDER given only an axis leaves its direction open (14.3): it fixes no precedence, the search
/// may reverse it (`search::tests::a_fact_whose_side_is_open_is_reversed`), and its members share a
/// row.
#[test]
fn an_order_given_only_an_axis_fixes_no_precedence() {
    let (p, solved, e) = solve_text(
        "* r1\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\nR2 b c 1k\nR3 c d 1k\n\
         * @RNL { {#R1 #R3} @ORDER(\"x\"); $WIRE_NET_SET @ROUTE }\n.end\n",
    );
    let [r1, r3] = ["#R1", "#R3"].map(|l| object(&p, l));
    let s = &e.structure;
    assert!(s.precedence.is_empty(), "{:?}", s.precedence);
    assert_eq!(s.reversible.len(), 1, "{:?}", s.reversible);
    assert!(s.rows.iter().any(|row| row.contains(&r1) && row.contains(&r3)), "{:?}", s.rows);
    assert!(e.candidate.rows.iter().any(|row| row.contains(&r1) && row.contains(&r3)), "{}", summary(&p, &solved, &e));
}

/// An ALIGN on x is a column: its members go in rows of their own, one above the other, not side by
/// side in one row, where the gaps of the row could not hold.
#[test]
fn an_align_on_x_is_a_column() {
    let (p, solved, e) = solve_text(
        "* c1\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\nR2 b c 1k\nR3 c d 1k\n\
         * @RNL { {#R1 #R2} @ALIGN(\"x\"); $WIRE_NET_SET @ROUTE }\n.end\n",
    );
    let what = summary(&p, &solved, &e);
    let [r1, r2] = ["#R1", "#R2"].map(|l| object(&p, l));
    assert!(!e.structure.rows.iter().any(|row| row.contains(&r1) && row.contains(&r2)), "{what}");
    assert!(e.placed.overlaps.is_empty(), "{what}");
    assert!(!e.placed.dropped.contains(&crate::search::RelKind::Gap), "{what}: {:?}", e.placed.dropped);
}

/// Groups whose parts or union are the same set as another group (synthesis 3.5): each set is one
/// block of the tree, and the solve places every element once, keeping every fact.
#[test]
fn groups_that_make_the_same_set_twice_are_solved() {
    let chain = "R1 a b 1k\nR2 b c 1k\nR3 c d 1k\nR4 d e 1k\nR5 f g 1k\n";
    for (groups, options) in [
        // the first part of the union of %b and %c is %a
        ("%a = {#R1 #R2} @GROUP; %b = {#R1 #R2 #R3} @GROUP; %c = {#R3 #R4} @GROUP", "objectives=none"),
        ("%a = {#R1 #R2} @GROUP; %b = {#R1 #R2 #R3} @GROUP; %c = {#R3 #R4} @GROUP", ""),
        // the union of %g2 and %g1 is %g0, inside %top
        (
            "%top = {#R1 #R2 #R3 #R4 #R5} @GROUP; %g0 = {#R1 #R2 #R3 #R4} @GROUP; %g2 = {#R3 #R4} @GROUP; \
             %g1 = {#R1 #R2 #R3} @GROUP",
            "",
        ),
        // a group written after the union that is one of its two groups again
        ("%b = {#R1 #R2 #R3} @GROUP; %c = {#R3 #R4} @GROUP; %d = {#R1 #R2 #R3} @GROUP", ""),
    ] {
        let text = format!(
            "* d\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\n{chain}* @RNL {{ {groups}; $WIRE_NET_SET @ROUTE }}\n.end\n"
        );
        let ps = problems_of(&text);
        let options = SolverOptions::without_time_limit().with_overrides(options).unwrap();
        let (solved, e) = solve_exact(&ps[0], &options);
        let what = summary(&ps[0], &solved, &e);
        let tree = &e.structure.tree;
        let sets: Vec<&Vec<usize>> = tree.nodes[1..].iter().map(|n| &n.members).collect();
        for (i, a) in sets.iter().enumerate() {
            assert!(!sets[i + 1..].contains(a), "{groups}: {:?}", tree.nodes);
        }
        let mut placed: Vec<usize> = e.candidate.rows.concat();
        placed.sort_unstable();
        assert_eq!(placed, e.structure.elements, "{groups}: {what}");
        check_level("groups", &ps[0]);
    }
}

/// The parts of the union of two groups that overlap stay in their order or the reverse, the
/// shared one between the other two, in every candidate of the search (algorithm synthesis 3.5);
/// in the placement taken their boxes follow one another on x, so that both groups are in one piece.
#[test]
fn overlapping_groups_keep_the_shared_part_between_the_others() {
    let chain = "R1 a b 1k\nR2 b c 1k\nR3 c d 1k\nR4 d e 1k\n";
    for groups in
        ["%g2 = {#R3 #R4} @GROUP; %g1 = {#R1 #R2 #R3} @GROUP", "%g1 = {#R1 #R4} @GROUP; %g2 = {#R1 #R2} @GROUP"]
    {
        let text = format!(
            "* u\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\n{chain}* @RNL {{ {groups}; $WIRE_NET_SET @ROUTE }}\n.end\n"
        );
        let ps = problems_of(&text);
        let p = &ps[0];
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve(p, &options);
        let mut work = crate::Work::default();
        let s = analyse(p, &resolved, &options, CAPS, &mut work);
        let nodes = &s.tree.nodes;
        let union = nodes.iter().position(|n| n.fixed_order).unwrap_or_else(|| panic!("{groups}: {nodes:?}"));
        let parts: Vec<Vec<usize>> = nodes[union].children.iter().map(|&c| nodes[c].members.clone()).collect();
        assert_eq!(parts.len(), 3, "{groups}: {nodes:?}");
        let forward = vec![(parts[0].clone(), parts[1].clone()), (parts[1].clone(), parts[2].clone())];
        let backward = vec![(parts[2].clone(), parts[1].clone()), (parts[1].clone(), parts[0].clone())];
        let candidates = search(p, &s, &options, &mut work);
        assert!(candidates.iter().filter(|c| c.origin == "beam").count() >= 2, "{groups}");
        for c in candidates.iter().filter(|c| c.origin == "beam") {
            let between: Vec<(Vec<usize>, Vec<usize>)> =
                c.apart.iter().filter(|(a, b)| parts.contains(a) && parts.contains(b)).cloned().collect();
            assert!(between == forward || between == backward, "{groups}: {:?}", c.apart);
        }
        let (solved, e) = check_level("union", p);
        let what = summary(p, &solved, &e);
        assert!(!e.placed.dropped.contains(&RelKind::BlocksApart), "{groups}: {what}");
        let extent = |part: &[usize]| {
            let boxes: Vec<[Q; 4]> = part
                .iter()
                .map(|&k| e.structure.box_at(&e.placed.origins, &e.placed.geometry.orient, k).unwrap())
                .collect();
            let lo = boxes.iter().map(|b| b[0].clone()).min().unwrap();
            let hi = boxes.iter().map(|b| b[2].clone()).max().unwrap();
            (lo, hi)
        };
        let x: Vec<(Q, Q)> = parts.iter().map(|part| extent(part)).collect();
        let follow = |a: &(Q, Q), b: &(Q, Q)| a.1 <= b.0;
        assert!(
            (follow(&x[0], &x[1]) && follow(&x[1], &x[2])) || (follow(&x[2], &x[1]) && follow(&x[1], &x[0])),
            "{groups}: {x:?}\n{what}"
        );
    }
}

/// Groups and a fact drawn at random over a chain of five resistors. Every search orders every
/// element once in each candidate and, where no fact orders elements on x, keeps the shared part
/// of each union between the other two; one level in five is solved and checked as the files are
/// (`check_level`).
#[test]
fn groups_and_facts_drawn_at_random_are_solved() {
    let mut rng = crate::structure::tests::Lcg(0x9e37_79b9_7f4a_7c15);
    let chain = "R1 a b 1k\nR2 b c 1k\nR3 c d 1k\nR4 d e 1k\nR5 e f 1k\n";
    let options = SolverOptions::without_time_limit();
    let mut unions = 0;
    for i in 0..100 {
        let mut lines: Vec<String> = rng
            .groups(5)
            .iter()
            .enumerate()
            .filter(|(_, g)| !g.is_empty())
            .map(|(j, g)| {
                let members: Vec<String> = g.iter().map(|k| format!("#R{}", k + 1)).collect();
                format!("%g{j} = {{{}}} @GROUP", members.join(" "))
            })
            .collect();
        let (a, b) = (rng.below(5) + 1, rng.below(5) + 1);
        let fact = match rng.below(6) {
            0 => format!("#R{a} #R{b} @DISTANCE(\"left-to-right\", min=3)"),
            1 => format!("{{#R{a} #R{b}}} @ALIGN(\"y\")"),
            2 => format!("{{#R{a} #R{b}}} @ALIGN(\"x\")"),
            3 => format!("{{#R{a} #R{b}}} @ORDER(\"x\")"),
            4 => format!("{{#R{a} #R{b}}} @ORDER(\"left-to-right\")"),
            _ => String::new(),
        };
        if a != b && !fact.is_empty() {
            lines.push(fact);
        }
        lines.push("$WIRE_NET_SET @ROUTE".to_string());
        let text = format!(
            "* f\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\n{chain}* @RNL {{ {} }}\n.end\n",
            lines.join("; ")
        );
        let ps = problems_of(&text);
        let p = &ps[0];
        let resolved = crate::resolve(p, &options);
        let mut work = crate::Work::default();
        let s = analyse(p, &resolved, &options, CAPS, &mut work);
        let free = s.precedence.is_empty() && s.reversible.is_empty();
        let nodes = &s.tree.nodes;
        for c in search(p, &s, &options, &mut work) {
            let mut placed: Vec<usize> = c.rows.concat();
            placed.sort_unstable();
            assert_eq!(placed, s.elements, "{text}: {c:?}");
            if c.origin != "beam" || !free {
                continue;
            }
            for n in nodes.iter().filter(|n| n.fixed_order) {
                unions += 1;
                let parts: Vec<&Vec<usize>> = n.children.iter().map(|&c| &nodes[c].members).collect();
                let between: Vec<(&Vec<usize>, &Vec<usize>)> = c
                    .apart
                    .iter()
                    .filter(|(a, b)| parts.contains(&a) && parts.contains(&b))
                    .map(|(a, b)| (a, b))
                    .collect();
                let forward = vec![(parts[0], parts[1]), (parts[1], parts[2])];
                let backward = vec![(parts[2], parts[1]), (parts[1], parts[0])];
                assert!(between == forward || between == backward, "{text}: {:?}", c.apart);
            }
        }
        if i % 5 == 0 {
            check_level(&text, p);
        }
    }
    assert!(unions > 20, "{unions}");
}

/// A netlist given as the body of a file of 0.2.
fn netlist(body: &str) -> String {
    format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\n{body}.end\n")
}

/// Solves the one level of a netlist given as text with some options, checked as the cases of the
/// files are, and asserts that every ROUTE is routed whole.
fn routed_whole(body: &str, options: &str) -> (LevelProblem, Solved, Exact) {
    let ps = problems_of(&netlist(body));
    assert_eq!(ps.len(), 1, "{body}");
    let p = ps.into_iter().next().unwrap();
    let (solved, e) = check_with(body, &p, &SolverOptions::without_time_limit().with_overrides(options).unwrap());
    assert!(
        solved.report.unrouted.is_empty(),
        "{body} / {options}: {:?}\n{}",
        solved.report.unrouted,
        summary(&p, &solved, &e)
    );
    (p, solved, e)
}

/// Pins of two nets face to face across a gap of 2 between neighbours, where both lead to one node
/// (rows keep such pins of two nets routed three apart now, but facts may put them closer): the net
/// routed second leaves its pin along the edge of its symbol, and every net is one tree, with and
/// without the objectives (review of S4).
#[test]
fn pins_of_two_nets_face_to_face_in_a_row_are_both_routed() {
    for body in [
        "R1 a b 1k\nR2 a b 1k\nR3 a c 1k\n* @RNL { {&a &b} @ROUTE }\n",
        "R1 a b 1k\nR2 b c 1k\nR3 c a 1k\n* @RNL { #R1 #R2 @DISTANCE(\"left-to-right\", min=4, max=4); $WIRE_NET_SET @ROUTE }\n",
        "R1 hub x 1k\nR2 hub x 1k\nR3 hub x 1k\nR4 hub x 1k\nR5 hub x 1k\nR6 hub x 1k\nR7 x b 1k\nR8 hub d 1k\n\
         * @RNL { $WIRE_NET_SET @ROUTE }\n",
        "R1 a b 1k\nR2 b c 1k\nR3 c d 1k\nR4 d e 1k\n* @RNL { %f = (_, _); &b @WIRE {#R1:2 %f}; $WIRE_NET_SET @ROUTE }\n",
    ] {
        for options in ["", "objectives=none"] {
            let (p, solved, _) = routed_whole(body, options);
            assert!(!solved.solution.wires.is_empty(), "{}", p.title);
        }
    }
}

/// A ROUTE on a supply and a ground net that carry the standard package's stubs (13.7: every net
/// but the ground in the standard's own example): the stubs' wires are part of the trees, and each
/// net is one tree, joined from pins whose own way their stub takes along the edges of their
/// symbols; so too a ground drawn right on a pin (12.1) (review of S4).
#[test]
fn supply_and_ground_nets_with_stubs_are_routed() {
    let circuit = "V1 vdd 0 1\nR1 vdd out 1k\nC1 out 0 1n\nR2 out 0 1k\n";
    for (rnl, nets) in [
        ("&vdd @ADD-TO $SUPPLY_NET_SET; # @SELECT-NET => @ROUTE", vec!["vdd", "0", "out"]),
        ("$GND @ATTACH #C1:2; &0 @ROUTE", vec!["0"]),
    ] {
        let body = format!("{circuit}* @RNL {{ {rnl} }}\n");
        let (p, solved, _) = routed_whole(&body, "");
        for net in nets {
            let wires = solved.solution.wires.iter().filter(|w| p.routes[w.route].net == net).count();
            assert!(wires > 0, "{rnl}: &{net}");
        }
        // the stubs are there, and their wires are written on the nets routed
        assert!(p.routes.iter().any(|r| !r.wires.is_empty()), "{rnl}");
    }
}

/// A symbol placed far from the rest no longer makes the lattice too large (review of S4).
#[test]
fn symbols_placed_far_apart_are_routed() {
    let (_, solved, _) = routed_whole(
        "R1 a b 1k\nR2 a b 1k\n* @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(25000, 0); $WIRE_NET_SET @ROUTE }\n",
        "",
    );
    assert_eq!(solved.solution.wires.len(), 2);
}

/// Two ROUTEs of one net with different styles: the wires take the first's, and the report says
/// so; with the same style, nothing is noted. The counts of the notes are singular for one.
#[test]
fn routes_of_one_net_with_different_styles_are_noted() {
    let note = "the ROUTEs of &b give different styles: its wires take the style of the first of them";
    let (p, solved, _) =
        routed_whole("R1 a b 1k\nR2 a b 1k\n* @RNL { &b @ROUTE(linestyle=\"dashed\"); &b @ROUTE }\n", "");
    assert!(solved.report.notes.iter().any(|n| n == note), "{:?}", solved.report.notes);
    assert!(solved.solution.wires.iter().all(|w| w.route == 0));
    assert_eq!(p.routes.len(), 2);
    assert!(
        solved
            .report
            .notes
            .iter()
            .any(|n| n.starts_with("1 wire and 0 branch points routed for the ROUTEs of 1 net, ")),
        "{:?}",
        solved.report.notes
    );
    for rnl in ["&b @ROUTE(linestyle=\"dashed\"); &b @ROUTE(linestyle=\"dashed\")", "&b @ROUTE; &b @ROUTE"] {
        let (_, solved, _) = routed_whole(&format!("R1 a b 1k\nR2 a b 1k\n* @RNL {{ {rnl} }}\n"), "");
        assert!(!solved.report.notes.iter().any(|n| n.contains("give different styles")), "{rnl}");
    }
    assert_eq!(
        (count(0, "net"), count(1, "net"), count(2, "branch point")),
        ("0 nets".into(), "1 net".into(), "2 branch points".into())
    );
}

/// T3: the wire of the port of &out meets the wire between the drains of M4 and M2 in its middle,
/// at a branch point with a junction, not at M4's drain by a wire of length 0, which the port
/// beside the drain at its height made (13.2; quality review SE-13).
#[test]
fn t3_has_a_junction_where_the_port_meets_the_wire_between_the_drains() {
    let ps = problems(&repo().join("tests/integration/solver/T3-ota5t.sp"));
    let p = ps.iter().find(|p| p.title == "ota5t").unwrap();
    let (solved, e) = check_level("T3", p);
    let what = summary(p, &solved, &e);
    for (w, path) in solved.solution.wires.iter().zip(&e.routing.paths) {
        assert!(path.windows(2).any(|s| s[0] != s[1]), "{what}: of length 0: {w:?}");
    }
    let (m2, m4) = (object(p, "#M2"), object(p, "#M4"));
    let s = &e.structure;
    let drain = |k: usize| {
        let j = s.pins(k).iter().position(|pi| pi.terminals.iter().any(|t| t == "1")).unwrap();
        s.pin_at(&e.placed.origins, &e.placed.geometry.orient, k, j).unwrap()
    };
    let (top, bottom) = (drain(m4), drain(m2));
    assert_eq!(top[0], bottom[0], "{what}");
    let inside = solved.solution.branches.iter().filter(|b| {
        let (x, y) = (from_f64(b.at.0).unwrap(), from_f64(b.at.1).unwrap());
        x == top[0] && bottom[1] < y && y < top[1]
    });
    assert_eq!(inside.filter(|b| b.junction).count(), 1, "{what}");
}

/// Without objectives the candidates differ, and every ROUTE of T1–T6 is still routed whole (T4
/// left two nets in parts before leads along the edges of symbols; review of S4).
#[test]
fn every_case_of_the_solver_is_routed_whole_without_objectives() {
    let options = SolverOptions::without_time_limit().with_overrides("objectives=none").unwrap();
    for f in netlists("tests/integration/solver", "T") {
        for p in problems(&f) {
            let (solved, _) = check_with(&f.to_string_lossy(), &p, &options);
            assert!(solved.report.unrouted.is_empty(), "{}: {:?}", p.title, solved.report.unrouted);
        }
    }
}

// ------------------------------------------------------- the quality review of the first drawings

/// The bends of the wires routed for each net, by its name (the turns of the objectives on the
/// wires, net by net).
fn bends_by_net(p: &LevelProblem, solved: &Solved, e: &Exact) -> BTreeMap<String, usize> {
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for (w, path) in solved.solution.wires.iter().zip(&e.routing.paths) {
        *out.entry(p.routes[w.route].net.clone()).or_default() += crate::objectives::bends(path);
    }
    out
}

/// The ends of the wires routed that leave a pin against its exit (9.2.4), each said with its net
/// and its pin: a wire from a pin whose first step of some length is not the pin's own way, as a
/// lead along the edge of its symbol is (`crate::route`).
fn violated_exits(p: &LevelProblem, solved: &Solved, e: &Exact) -> Vec<String> {
    use rnl_runtime::solution::WirePoint;
    let (s, ori) = (&e.structure, &e.placed.geometry.orient);
    let mut out = Vec::new();
    for (w, path) in solved.solution.wires.iter().zip(&e.routing.paths) {
        let backwards: Vec<[Q; 2]> = path.iter().rev().cloned().collect();
        for (end, path) in [(w.points.first(), path), (w.points.last(), &backwards)] {
            let Some(WirePoint::Anchor(PointRef::Pin { object, pin, terminal })) = end else { continue };
            let Some((x, y)) = s.exit(*object, *pin, ori[*object]) else { continue };
            let Some(start) = path.first() else { continue };
            let Some(next) = path.iter().find(|q| *q != start) else { continue };
            let way = ((&next[0] - &start[0]).signum(), (&next[1] - &start[1]).signum());
            if way != (int(x), int(y)) {
                let label = &p.objects[*object].label;
                out.push(format!("{} at {label}:{terminal}, its exit ({x}, {y}): {path:?}", p.routes[w.route].net));
            }
        }
    }
    out
}

/// The level of a case of the solver with a title, solved and checked.
fn case_level(file: &str, title: &str) -> (LevelProblem, Solved, Exact) {
    level_in("tests/integration/solver", file, title)
}

/// The level with a title of a netlist in a directory of the repository, solved and checked.
fn level_in(dir: &str, file: &str, title: &str) -> (LevelProblem, Solved, Exact) {
    let ps = problems(&repo().join(dir).join(file));
    let p = ps.into_iter().find(|p| p.title == title).unwrap_or_else(|| panic!("{file}: no level {title}"));
    let (solved, e) = check_level(file, &p);
    (p, solved, e)
}

/// T1, the RC low-pass filter: a pin that leaves sideways towards its neighbour and a pin that
/// leaves up are not put on one height, which leaves the router only a jog of three bends, but
/// the sideways one above the other's lead, an L of one bend (quality review SE-1, LP-1, SC-1):
/// R1 is the rail on top, C1 lower, two bends in all.
#[test]
fn t1_is_drawn_with_an_l_for_each_wire() {
    let (p, solved, e) = case_level("T1-rc-lowpass.sp", "T1-rc-lowpass.sp");
    let what = summary(&p, &solved, &e);
    let bends = bends_by_net(&p, &solved, &e);
    assert_eq!(bends.get("in"), Some(&1), "{what}");
    assert_eq!(bends.get("out"), Some(&1), "{what}");
    let at = &e.placed.origins;
    let [vin, r1, c1] = ["#VIN", "#R1", "#C1"].map(|l| object(&p, l));
    let pin = |k: usize, j: usize| e.structure.pin_at(at, &e.placed.geometry.orient, k, j).unwrap();
    // R1 lies across, above the top pins of VIN and C1
    assert_eq!(pin(r1, 0)[1], pin(r1, 1)[1], "{what}");
    for k in [vin, c1] {
        let top = (0..2).map(|j| pin(k, j)[1].clone()).max().unwrap();
        assert!(top < pin(r1, 0)[1], "{what}: {}", p.objects[k].label);
    }
}

/// T2 and T2b, the common-source stage: the gate wire from VIN is an L, not a jog of three bends
/// (quality review SE-1, LP-1, SC-1).
#[test]
fn t2_has_an_l_from_the_source_to_the_gate() {
    for file in ["T2-common-source.sp", "T2b-common-source-dragged.sp"] {
        let (p, solved, e) = case_level(file, file);
        assert_eq!(bends_by_net(&p, &solved, &e).get("in"), Some(&1), "{}", summary(&p, &solved, &e));
    }
}

/// T3's testbench: &inn goes up from VIN and turns once into the input of XU1, with no jog round
/// the corner of its box (quality review LP-1).
#[test]
fn t3_has_no_jog_into_the_input_of_the_ota() {
    let (p, solved, e) = case_level("T3-ota5t.sp", "T3-ota5t.sp");
    assert_eq!(bends_by_net(&p, &solved, &e).get("inn"), Some(&1), "{}", summary(&p, &solved, &e));
}

/// T5: &vin is an L from the source to RI; the tops of RL and C1, which both leave up to &n2, on
/// one line (quality review SE-1, SE-16); the output of the level of the subcircuit an L from the
/// port two above E1's pin (SE-13).
#[test]
fn t5_has_an_l_for_the_input_and_one_rail_on_the_load() {
    let (p, solved, e) = case_level("T5-inverting-amp-rlc.sp", "T5-inverting-amp-rlc.sp");
    let what = summary(&p, &solved, &e);
    assert_eq!(bends_by_net(&p, &solved, &e).get("vin"), Some(&1), "{what}");
    let s = &e.structure;
    let top = |label: &str| {
        let k = object(&p, label);
        let j = s.pins(k).iter().position(|pi| pi.terminals.iter().any(|t| t == "1")).unwrap();
        assert_eq!(s.exit(k, j, e.placed.geometry.orient[k]), Some((0, 1)), "{what}: {label}");
        s.pin_at(&e.placed.origins, &e.placed.geometry.orient, k, j).unwrap()
    };
    assert_eq!(top("#RL")[1], top("#C1")[1], "{what}");
    let (p, solved, e) = case_level("T5-inverting-amp-rlc.sp", "opamp");
    assert_eq!(bends_by_net(&p, &solved, &e).get("out"), Some(&1), "{}", summary(&p, &solved, &e));
}

/// T6, the R-2R ladder: each source next to its leg of 2R in the row joins it by an L, and &n0
/// has no jog: RT, R2B0 and RS0 meet on one line (quality review SE-1). The legs whose sources
/// the order of the row puts elsewhere are SE-10's.
#[test]
fn t6_joins_each_source_to_its_leg_by_an_l() {
    let (p, solved, e) = case_level("T6-r2r-ladder.sp", "T6-r2r-ladder.sp");
    let what = summary(&p, &solved, &e);
    let bends = bends_by_net(&p, &solved, &e);
    let row = &e.candidate.rows[0];
    let mut neighbours = 0;
    for i in 0..8 {
        let (vb, leg) = (object(&p, &format!("#VB{i}")), object(&p, &format!("#R2B{i}")));
        let at = |k: usize| row.iter().position(|&j| j == k).unwrap();
        if at(vb).abs_diff(at(leg)) == 1 {
            neighbours += 1;
            assert_eq!(bends.get(&format!("b{i}")), Some(&1), "{what}: b{i}");
        }
    }
    assert!(neighbours >= 6, "{neighbours}: {what}");
    assert_eq!(bends.get("n0"), Some(&0), "{what}");
}

/// No wire routed for T1–T6 leaves a pin against its exit (9.2.4). The compacted tail of T6's
/// row put pins of two nets face to face two apart, R2B5's of b5 and R2B7's of b7, RS6's of n6 and
/// RS5's of n5, their leads ending on one node, which neither net then keeps: b5 left R2B5 and n5
/// left RS5 along the edges of their symbols. Such pins are three apart now (review of phase 1 of
/// the quality review). T4 is left out: &tail leaves M1's source, at the corner of its box,
/// sideways to the branch point beside it, as the router prices a violated exit as a bend.
#[test]
fn wires_routed_for_t1_to_t6_leave_each_pin_its_own_way() {
    for file in netlists("tests/integration/solver", "T") {
        if file.ends_with("T4-miller-conflicts.sp") {
            continue;
        }
        for p in problems(&file) {
            let (solved, e) = check_level(&file.to_string_lossy(), &p);
            let bad = violated_exits(&p, &solved, &e);
            assert!(bad.is_empty(), "{}: {bad:#?}\n{}", p.title, summary(&p, &solved, &e));
        }
    }
}

/// A port beside a pin that leaves up or down stays at the pin's height when wires written draw
/// its net, or nothing does: only the router leaves a pin by its lead, and a wire written is drawn
/// by its mode (13.1). Raised past the pin, corpus 03's port of &out, written from the junction to
/// the port, put the port's box in the way of the oblique &bias written, and the ports of rdiv in
/// corpus 04 turned R1 and R2 round, so that &mid ran lengthwise over R1 (review of phase 1 of the
/// quality review, SE-13).
#[test]
fn a_port_on_a_net_written_stays_at_the_height_of_its_pin() {
    use crate::search::{attachments, Side};
    for (file, title) in [("03-five-transistor-ota.sp", "03-five-transistor-ota.sp"), ("04-hierarchy.sp", "rdiv")] {
        let (p, solved, e) = level_in("tests/conformance/cases", file, title);
        let what = summary(&p, &solved, &e);
        let (s, ori, at) = (&e.structure, &e.placed.geometry.orient, &e.placed.origins);
        let mut vertical = 0;
        for (k, a) in attachments(s, &e.candidate.rows, ori) {
            if s.objects[k].role != Role::Port || !matches!(a.side, Side::Left | Side::Right) {
                continue;
            }
            let label = &p.objects[k].label;
            assert!(!s.connections.routed((k, 0), a.host), "{what}: {label}");
            let (port, host) = (s.pin_at(at, ori, k, 0).unwrap(), s.pin_at(at, ori, a.host.0, a.host.1).unwrap());
            assert_eq!(port[1], host[1], "{what}: {label}");
            vertical += usize::from(matches!(s.exit(a.host.0, a.host.1, ori[a.host.0]), Some((0, _))));
        }
        assert!(vertical > 0, "{what}");
    }
}

/// A pin that leaves up and one that leaves sideways towards it, neighbours in a row: the router
/// joins them by an L, the sideways one above the other's lead; a wire written that its mode draws
/// straight keeps them on one height, so that it is not oblique; one written vertical first from
/// the pin that leaves up is an L too (quality review SE-1; 13.1).
#[test]
fn neighbours_are_held_as_the_wire_between_them_is_drawn() {
    let body = |statement: &str| {
        format!(
            "V1 a 0 1
R1 a b 1k
* @RNL {{ {{#V1 #R1}} @AS $FLOW; {statement} }}
"
        )
    };
    for (statement, l) in [
        ("&a @ROUTE", true),
        ("&a @WIRE {#V1:1 #R1:1}", false),
        ("&a @WIRE(mode=\"vertical-horizontal\") {#V1:1 #R1:1}", true),
    ] {
        let (p, solved, e) = routed_whole(&body(statement), "");
        let what = format!(
            "{statement}
{}",
            summary(&p, &solved, &e)
        );
        let s = &e.structure;
        let pin = |label: &str| {
            let k = object(&p, label);
            let j = s.pins(k).iter().position(|pi| pi.terminals.iter().any(|t| t == "1")).unwrap();
            (
                s.exit(k, j, e.placed.geometry.orient[k]),
                s.pin_at(&e.placed.origins, &e.placed.geometry.orient, k, j).unwrap(),
            )
        };
        let ((up, source), (left, resistor)) = (pin("#V1"), pin("#R1"));
        assert_eq!((up, left), (Some((0, 1)), Some((-1, 0))), "{what}");
        if l {
            assert!(resistor[1] > source[1], "{what}");
        } else {
            assert_eq!(resistor[1], source[1], "{what}");
        }
        assert!(e.placed.dropped.is_empty(), "{what}: {:?}", e.placed.dropped);
    }
}

/// The width and the height of a level as it is drawn: the boxes of its components and its wires
/// routed (the extent of the metrics of the review, without the stubs' own short wires, which lie
/// in their boxes' columns).
fn extent(e: &Exact) -> [Q; 2] {
    let s = &e.structure;
    let (at, ori) = (&e.placed.origins, &e.placed.geometry.orient);
    let mut points: Vec<[Q; 2]> = Vec::new();
    for k in s.components() {
        if let Some(b) = s.box_at(at, ori, k) {
            points.push([b[0].clone(), b[1].clone()]);
            points.push([b[2].clone(), b[3].clone()]);
        }
    }
    points.extend(e.routing.paths.iter().flatten().cloned());
    let span = |axis: usize| {
        let lo = points.iter().map(|q| &q[axis]).min().cloned().unwrap_or_default();
        let hi = points.iter().map(|q| &q[axis]).max().cloned().unwrap_or_default();
        hi - lo
    };
    [span(0), span(1)]
}

/// The length of the wires routed for each net, by its name.
fn length_by_net(p: &LevelProblem, solved: &Solved, e: &Exact) -> BTreeMap<String, Q> {
    let mut out: BTreeMap<String, Q> = BTreeMap::new();
    for (w, path) in solved.solution.wires.iter().zip(&e.routing.paths) {
        let length: Q = path.windows(2).map(|s| (&s[1][0] - &s[0][0]).abs() + (&s[1][1] - &s[0][1]).abs()).sum();
        *out.entry(p.routes[w.route].net.clone()).or_default() += length;
    }
    out
}

/// Whether the steps between the origins of some elements, left to right, are all the same.
fn evenly_spaced(e: &Exact, ks: &[usize]) -> bool {
    let mut xs: Vec<Q> = ks.iter().map(|&k| e.placed.origins[k][0].clone()).collect();
    xs.sort();
    xs.windows(3).all(|w| &w[2] - &w[1] == &w[1] - &w[0])
}

/// Phase 2 of the quality review, "stop stretching the drawing": nothing evenly spaces a row of
/// elements in no group, the texts are as wide as they are drawn and on the outer side of a
/// source, two rows are kept apart only where their elements are neighbours on x, and what only
/// stubs join is drawn together (SE-3, ST-7, SE-8, SE-4, LP-2). The numbers of phase 1 in the
/// comments.
#[test]
fn the_drawings_are_not_stretched() {
    let num = |v: f64| from_f64(v).unwrap();
    // T2: M1 risen towards RD, the wire between them 3 long (6), as the ground of VDD, left of
    // M1, no longer sets the gap between the rows: a gap below RD would make it 2, but the wire
    // from VIN to the gate of M1 runs under that ground, a unit clear of it; VIN nearer M1 (14),
    // its value on its outer side
    let (p, solved, e) = case_level("T2-common-source.sp", "T2-common-source.sp");
    let what = summary(&p, &solved, &e);
    assert!(length_by_net(&p, &solved, &e)["out"] <= int(3), "{what}");
    let [vin, m1] = ["#VIN", "#M1"].map(|l| object(&p, l));
    assert!(&e.placed.origins[m1][0] - &e.placed.origins[vin][0] < int(14), "{what}");
    assert_eq!(e.placed.geometry.orient[vin], Orient::MY, "{what}");
    // T3's testbench: the lower row not evenly spaced (16 apart), CL nearer the output (&out 33
    // long), VDD beside XU1 (21 apart), narrower (50.5); the wires from the sources up to XU1
    // still pass under the grounds of VDD and XU1, crossing nothing, as the rows keep their way
    let (p, solved, e) = case_level("T3-ota5t.sp", "T3-ota5t.sp");
    let what = summary(&p, &solved, &e);
    let row = ["#VIP", "#VIN", "#VB", "#CL"].map(|l| object(&p, l));
    assert!(!evenly_spaced(&e, &row), "{what}");
    assert!(length_by_net(&p, &solved, &e)["out"] <= int(24), "{what}");
    let [vdd, xu1] = ["#VDD", "#XU1"].map(|l| object(&p, l));
    assert!(&e.placed.origins[xu1][0] - &e.placed.origins[vdd][0] <= int(12), "{what}");
    assert!(extent(&e)[0] < num(50.5), "{what}");
    let wires = crate::score::drawn(&p, &e.structure, &e.placed.geometry, &e.routing);
    let view = crate::objectives::View { s: &e.structure, at: &e.placed.origins, orient: &e.placed.geometry.orient };
    let crossings = crate::objectives::score(crate::objectives::Objective::Crossings, &view, Some(&wires));
    assert!(crossings.is_zero(), "{what}");
    // T4: lower (44.3), the rows of the PMOS and of the input pair nearer each other; M5 and M7
    // where they were (-15), not lower: &vb runs from VB, which leaves up on the left of the row
    // of the pair, to their gates, under the ground of VIP, and the rows stay apart as wholes over
    // its span (a_wire_between_two_rows_crosses_no_lead_of_a_stub); risen to -12, M5 and M7 had it
    // cross that ground's lead. Until VB hangs by the gates (quality review ST-3)
    let (p, solved, e) = case_level("T4-miller-conflicts.sp", "T4-miller-conflicts.sp");
    let what = summary(&p, &solved, &e);
    for label in ["#M5", "#M7"] {
        assert!(e.placed.origins[object(&p, label)][1] >= int(-15), "{what}: {label}");
    }
    assert!(extent(&e)[1] < num(44.3), "{what}");
    // T5: the row not evenly spaced (13 apart), narrower (80.5)
    let (p, solved, e) = case_level("T5-inverting-amp-rlc.sp", "T5-inverting-amp-rlc.sp");
    let what = summary(&p, &solved, &e);
    let row = e.candidate.rows[0].clone();
    assert!(row.len() >= 7 && !evenly_spaced(&e, &row), "{what}");
    assert!(extent(&e)[0] < num(80.5), "{what}");
    // T6: compact again (348 wide after phase 1, 302 before it), in the order of the bits: the
    // legs of 2R from b0 to b5 and the series arm from RS0 to RS4 left to right, the tail after
    // them SE-10's; each source next to its leg joined to it by an L
    // (t6_joins_each_source_to_its_leg_by_an_l)
    let (p, solved, e) = case_level("T6-r2r-ladder.sp", "T6-r2r-ladder.sp");
    let what = summary(&p, &solved, &e);
    assert!(!e.candidate.even, "{what}");
    assert!(extent(&e)[0] <= int(220), "{what}");
    let x = |label: String| e.placed.origins[object(&p, &label)][0].clone();
    let legs: Vec<Q> = (0..6).map(|i| x(format!("#R2B{i}"))).collect();
    let arm: Vec<Q> = (0..5).map(|i| x(format!("#RS{i}"))).collect();
    for xs in [&legs, &arm] {
        assert!(xs.windows(2).all(|w| w[0] < w[1]), "{xs:?}: {what}");
    }
}

/// The places where a wire routed crosses the lead of a stub of another net, from the pin it hangs
/// from to its symbol's pin, strictly inside both, each said with the nets and the stub's host.
fn stub_leads_crossed(p: &LevelProblem, solved: &Solved, e: &Exact) -> Vec<String> {
    let (s, at, ori) = (&e.structure, &e.placed.origins, &e.placed.geometry.orient);
    let net_names = |k: usize| -> Vec<&str> {
        s.connections
            .nets_of
            .get(&(k, 0))
            .map_or(Vec::new(), |v| v.iter().map(|&n| s.connections.nets[n].name.as_str()).collect())
    };
    let inside = |v: &Q, a: &Q, b: &Q| (v > a && v < b) || (v > b && v < a);
    let mut out = Vec::new();
    for (j, info) in s.objects.iter().enumerate() {
        let Some(stub) = &info.stub else { continue };
        let (Some(top), Some(end)) = (s.pin_at(at, ori, stub.host.0, stub.host.1), s.pin_at(at, ori, j, 0)) else {
            continue;
        };
        // a lead runs up or down from its pin
        if top[0] != end[0] {
            continue;
        }
        let nets = net_names(j);
        for (w, path) in solved.solution.wires.iter().zip(&e.routing.paths) {
            let net = &p.routes[w.route].net;
            if nets.contains(&net.as_str()) {
                continue;
            }
            for seg in path.windows(2) {
                let (a, b) = (&seg[0], &seg[1]);
                if a[1] == b[1] && inside(&top[0], &a[0], &b[0]) && inside(&a[1], &top[1], &end[1]) {
                    let host = &p.objects[stub.host.0].label;
                    out.push(format!("{net} over the lead of {nets:?} from {host} at ({}, {})", top[0], a[1]));
                }
            }
        }
    }
    out
}

/// A wire routed between two rows crosses no lead of a stub hanging from the upper one: the rows
/// stay apart as wholes over its span (`crate::search::row_pairs`). Pair by pair alone, M5 and M7
/// rose beside the ground of VIP in T4, and &vb, which leaves VB upwards on the left of the upper
/// row, ran across that ground's lead to their gates (review of phase 2 of the quality review).
/// T6 is left out: &b5 crosses the lead of a leg's ground, as the order of the row puts its source
/// far from its leg (quality review SE-10).
#[test]
fn a_wire_between_two_rows_crosses_no_lead_of_a_stub() {
    let files = netlists("tests/integration/solver", "T").into_iter().chain(netlists("tests/conformance/cases", ""));
    for file in files {
        if file.ends_with("T6-r2r-ladder.sp") {
            continue;
        }
        for p in problems(&file) {
            let (solved, e) = check_level(&file.to_string_lossy(), &p);
            let crossed = stub_leads_crossed(&p, &solved, &e);
            assert!(crossed.is_empty(), "{}: {crossed:#?}\n{}", p.title, summary(&p, &solved, &e));
        }
    }
}

/// The structure of the level with a title of a case of the solver, not solved.
fn case_structure(file: &str, title: &str) -> (LevelProblem, Structure) {
    let ps = problems(&repo().join("tests/integration/solver").join(file));
    let p = ps.into_iter().find(|p| p.title == title).unwrap_or_else(|| panic!("{file}: no level {title}"));
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve(&p, &options);
    let s = crate::structure::analyse(&p, &resolved, &options, CAPS, &mut crate::Work::default());
    (p, s)
}

/// What a candidate with even rows spaces evenly (quality review SE-3): the members of a group,
/// those that a DISTRIBUTE or a mirror pair joins, and those of one kind and one role, such as the
/// legs of a ladder; never a row as such, a chain of a `$FLOW` or the elements in no group, which
/// are no scope of the objectives measured within groups either.
#[test]
fn only_groups_pairs_and_elements_of_one_role_are_spaced_evenly() {
    let labels = |p: &LevelProblem, set: &[usize]| -> Vec<String> {
        let mut v: Vec<String> = set.iter().map(|&k| p.objects[k].label.clone()).collect();
        v.sort();
        v
    };
    let names = |prefix: &str, n: usize| -> Vec<String> {
        let mut v: Vec<String> = (0..n).map(|i| format!("#{prefix}{i}")).collect();
        v.sort();
        v
    };
    // T6, no group: the legs of 2R, the sources and the series arm, each a set; not the row
    let (p, s) = case_structure("T6-r2r-ladder.sp", "T6-r2r-ladder.sp");
    let sets: Vec<Vec<String>> = s.even_sets.iter().map(|set| labels(&p, set)).collect();
    assert_eq!(sets.len(), 3, "{sets:?}");
    for want in [names("R2B", 8), names("VB", 8), names("RS", 7)] {
        assert!(sets.contains(&want), "{want:?} in {sets:?}");
    }
    assert!(s.scopes.is_empty());
    // T5: a chain of a $FLOW and a load of two elements hung from one net: nothing
    let (_, s) = case_structure("T5-inverting-amp-rlc.sp", "T5-inverting-amp-rlc.sp");
    assert!(s.even_sets.is_empty(), "{:?}", s.even_sets);
    // T4: the two groups, and no set of the two halves of a mirror pair alone
    let (p, s) = case_structure("T4-miller-conflicts.sp", "T4-miller-conflicts.sp");
    let sets: Vec<Vec<String>> = s.even_sets.iter().map(|set| labels(&p, set)).collect();
    assert_eq!(sets, vec![labels(&p, &p.groups[0].members), labels(&p, &p.groups[1].members)]);
    assert_eq!(s.scopes.len(), 2);
}

/// The texts are as wide as the renderer lays them out, with a margin, not 0.6 of their size a
/// character (quality review ST-7): the value of T2's source, 72.62 pixels at 14 a unit, 5.19
/// wide, was taken 6.6 wide. A text of a symbol turned with its owner reads upright, as the
/// renderer keeps it: its anchor turns, and a baseline stays below the glyphs, where a box turned
/// whole lay mirrored across it (review of phase 2 of the quality review: the names and values of
/// T6's legs and RS6, turned R270).
#[test]
fn a_text_is_as_wide_as_it_is_drawn() {
    use rnl_runtime::problem::{Filler, TextInfo};
    let text = |s: &str, halign: &str, rotate: f64| TextInfo {
        attach: Filler::Other(String::new()),
        key: None,
        lines: rnl_runtime::markup::parse(s),
        halign: halign.to_string(),
        valign: "baseline".to_string(),
        rotate,
        style: Vec::new(),
    };
    let text_box =
        |t: &TextInfo, at: (i64, i64), turn: Orient| crate::structure::text_box(t, &(int(at.0), int(at.1)), turn);
    let b = text_box(&text("DC 0.6 AC 1", "start", 0.0), (0, 0), Orient::R0).unwrap();
    let w = &b[2] - &b[0];
    assert!(w > from_f64(5.19).unwrap() && w < from_f64(5.5).unwrap(), "{b:?}");
    // the baseline 0.8 of the size below the top, 0.4 above the bottom
    assert_eq!((b[1].clone(), b[3].clone()), (from_f64(-0.4).unwrap(), from_f64(0.8).unwrap()));
    // ending at the anchor, and turned a quarter about it: along y, below the anchor
    let b = text_box(&text("DC 0.6 AC 1", "end", 90.0), (1, 2), Orient::R0).unwrap();
    let q = |v: f64| from_f64(v).unwrap();
    assert_eq!([&b[0], &b[2], &b[3]], [&q(0.2), &q(1.4), &q(2.1)], "{b:?}");
    assert_eq!(&b[3] - &b[1], w);
    // its owner turned R270: the anchor at (2, -1), the text reading up to it from below, as it
    // would read down, the tops of its glyphs on the left, 0.8 left of the anchor and 0.4 right of
    // it (a box turned whole had them 0.4 left and 0.8 right)
    let b = text_box(&text("DC 0.6 AC 1", "start", 0.0), (1, 2), Orient::R270).unwrap();
    assert_eq!([&b[0], &b[2], &b[3]], [&q(1.2), &q(2.4), &q(-0.9)], "{b:?}");
    assert_eq!(&b[3] - &b[1], w);
    // mirrored upside down: the anchor at (1, -2), the glyphs still above their baseline
    let b = text_box(&text("DC 0.6 AC 1", "start", 0.0), (1, 2), Orient::MX).unwrap();
    assert_eq!([&b[0], &b[1], &b[3]], [&q(0.9), &q(-2.4), &q(-1.2)], "{b:?}");
    assert_eq!(&b[2] - &b[0], w);
    // a text hanging from its anchor by its top stands on it by its bottom, mirrored so
    let mut top = text("DC 0.6 AC 1", "start", 0.0);
    top.valign = "top".to_string();
    let (upright, mirrored) =
        (text_box(&top, (0, 0), Orient::R0).unwrap(), text_box(&top, (0, 0), Orient::MX).unwrap());
    let middle = |b: &[Q; 4]| &b[1] + &b[3];
    assert!(middle(&upright) < Q::zero() && middle(&mirrored) > Q::zero(), "{upright:?} {mirrored:?}");
    assert!(text_box(&text("", "start", 0.0), (0, 0), Orient::R0).is_none());
}

/// A text that offsets place from an element's origin on both axes is part of the element's box
/// (`Structure::offset_labels`): corpus 06's codes, `@TEXT(..., dx, dy)` on each instance, clear
/// of each other and of the symbols. Compacted without them, the code of R8 lay over that of D1
/// (review of phase 2 of the quality review).
#[test]
fn texts_that_offsets_place_are_kept_clear() {
    let (p, solved, e) = level_in("tests/conformance/cases", "06-symbols.sp", "06-symbols.sp");
    let what = summary(&p, &solved, &e);
    let (s, at, ori) = (&e.structure, &e.placed.origins, &e.placed.geometry.orient);
    let placed = |k: usize, b: &[Q; 4]| [&at[k][0] + &b[0], &at[k][1] + &b[1], &at[k][0] + &b[2], &at[k][1] + &b[3]];
    let texts: Vec<(usize, [Q; 4])> =
        s.offset_labels.iter().flat_map(|(&k, boxes)| boxes.iter().map(move |b| (k, placed(k, b)))).collect();
    assert!(texts.len() >= 10, "{what}");
    let meet = |a: &[Q; 4], b: &[Q; 4]| a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3];
    for (i, (k, a)) in texts.iter().enumerate() {
        for (j, b) in &texts[i + 1..] {
            assert!(k == j || !meet(a, b), "{} {}: {what}", p.objects[*k].label, p.objects[*j].label);
        }
        for j in s.components().filter(|j| j != k) {
            let Some(b) = s.box_at(at, ori, j) else { continue };
            assert!(!meet(a, &b), "{} {}: {what}", p.objects[*k].label, p.objects[j].label);
        }
    }
}

// -------------------------------------------------- what the solver places, and what it leaves free

use rnl_runtime::problem::{Filler, Reference};

use crate::model::{ax, PointId};

/// Every netlist of the cases of the solver and of the conformance corpus, `.sp` and `.scs`: T1–T6
/// and T2b, the cases of 0.2 and of the draft 0.3, and the pairs of equivalent netlists.
fn every_netlist() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in [
        "tests/integration/solver",
        "tests/conformance/cases",
        "tests/conformance/cases-0.3",
        "tests/conformance/equivalence",
    ] {
        let mut files: Vec<PathBuf> = std::fs::read_dir(repo().join(dir))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "sp" || x == "scs"))
            .collect();
        files.sort();
        out.extend(files);
    }
    out
}

/// The points that a relation of the solver's own bounds, by axis.
fn relation_points(r: &Relation) -> Vec<(usize, Vec<PointId>)> {
    let points = |terms: &[(PointId, Q)]| terms.iter().map(|(p, _)| *p).collect::<Vec<PointId>>();
    match r {
        Relation::Orient { .. } => Vec::new(),
        Relation::Linear { axis, terms, .. } => vec![(*axis, points(terms))],
        Relation::Apart { axis, low, high, .. } => vec![(*axis, points(&[low.clone(), high.clone()].concat()))],
        Relation::OneOf(alternatives) => {
            alternatives.iter().flatten().map(|(axis, terms, _)| (*axis, points(terms))).collect()
        }
        Relation::Even { axis, points } => vec![(*axis, points.clone())],
    }
}

/// Whether the LP decides each object's coordinate on each axis, by object: false where it is
/// left where the witness of the resolution happens to be. The LP's first three layers, the facts
/// kept, the links of pins and texts to their objects, the wires, the relations of the candidate
/// `c` taken, with the orientations `ori`, and the anchors of texts all bound differences of
/// coordinates only, so a set of objects that they join on an axis can move as one along it,
/// unless it holds an element, which the search places in its rows, or a fact kept that fixes a
/// coordinate (a PLACE, or a coordinate of a point, 11.1). Otherwise only layer 4, the distance
/// to where the resolution put it, holds it, and its coordinates there are where the witness
/// happens to be, such as 0 for what nothing reaches; the repair of overlaps only pushes it out of
/// the way of the symbols placed, and is not counted. Such is a point `(_, _)` in no wire on an
/// axis that no fact reaches, and a supply, ground or port symbol that no fact reaches and that
/// the candidate puts beside no pin; a text attached to a wire or a net that no fact reaches would
/// be but for its anchor.
fn grounded(problem: &LevelProblem, s: &Structure, c: &Candidate, ori: &[Orient]) -> Vec<[bool; 2]> {
    let n = problem.objects.len();
    // the objects, then one node for what is held: a union-find on each axis
    let mut parent: [Vec<usize>; 2] = [(0..=n).collect(), (0..=n).collect()];
    fn root(parent: &mut [usize], mut k: usize) -> usize {
        while parent[k] != k {
            parent[k] = parent[parent[k]];
            k = parent[k];
        }
        k
    }
    let mut join = |axis: usize, a: usize, b: usize| {
        let (ra, rb) = (root(&mut parent[axis], a), root(&mut parent[axis], b));
        if ra != rb {
            parent[axis][ra.max(rb)] = ra.min(rb);
        }
    };
    let held = n;
    let object_of = |p: &PointId| match p {
        PointId::Origin(k) | PointId::Pin(k, _) | PointId::Text(k) => Some(*k),
        PointId::Centre(_) => None,
    };
    // the elements, in the rows of the candidate
    for &k in c.rows.iter().flatten() {
        join(0, k, held);
        join(1, k, held);
    }
    for (k, o) in problem.objects.iter().enumerate() {
        match &o.position {
            // drawn at the point of its main object (12.1)
            Position::At(_) => {
                if let Some(j) = s.model.origin_point[k].as_ref().and_then(object_of) {
                    join(0, k, j);
                    join(1, k, j);
                }
            }
            // a point of its owner on the axes that its symbol gives (13.3)
            Position::InSymbol { owner, .. } => {
                for axis in [Axis::X, Axis::Y] {
                    if o.position.from_symbol(axis) {
                        join(ax(axis), k, *owner);
                    }
                }
            }
            Position::Own { .. } => {}
        }
    }
    for &f in &s.kept {
        let (Some(unit), Some(fact)) = (s.model.units.get(f), problem.facts.get(f)) else { continue };
        let fixes = match &fact.kind {
            FactKind::Place { axis, .. } | FactKind::PointCoordinate { axis, .. } => Some(ax(*axis)),
            _ => None,
        };
        for axis in 0..2 {
            let objects: Vec<usize> = unit.touched[axis].iter().filter_map(object_of).collect();
            for w in objects.windows(2) {
                join(axis, w[0], w[1]);
            }
            if let (Some(&first), true) = (objects.first(), fixes == Some(axis)) {
                join(axis, first, held);
            }
        }
    }
    // the relations of the candidate: rows, gaps, rows and blocks apart, ports and supply and
    // ground symbols beside their pins, columns, even rows
    for t in crate::search::relations(s, c, ori, &SolverOptions::without_time_limit()) {
        for (axis, points) in relation_points(&t.rel) {
            let objects: Vec<usize> = points.iter().filter_map(object_of).collect();
            for w in objects.windows(2) {
                join(axis, w[0], w[1]);
            }
        }
    }
    for d in &s.connections.drawn {
        let objects: Vec<usize> = d.points.iter().filter_map(object_of).collect();
        for w in objects.windows(2) {
            join(0, w[0], w[1]);
            join(1, w[0], w[1]);
        }
    }
    for (k, a) in &s.anchors {
        for &(j, _) in &a.from {
            join(0, *k, j);
            join(1, *k, j);
        }
    }
    let ground = [root(&mut parent[0], held), root(&mut parent[1], held)];
    (0..n).map(|k| [root(&mut parent[0], k) == ground[0], root(&mut parent[1], k) == ground[1]]).collect()
}

/// The texts of a level: on the wire `%w` and on the net `&b`, which no fact reaches, and `%l` on
/// the wire, whose y an ALIGN with R2 gives.
const TEXTS_ON_WIRES: &str = concat!(
    "R1 a b 1k\n",
    "R2 b c 1k\n",
    "* @RNL {\n",
    "*   %w = &b @WIRE {#R1:2 #R2:1};\n",
    "*   %w @TEXT(\"on the wire\");\n",
    "*   &b @TEXT(\"on the net\");\n",
    "*   %l = %w @TEXT(\"aligned\");\n",
    "*   {%l #R2} @ALIGN(\"y\")\n",
    "* }\n",
);

/// A text attached to a wire or a net, which has no coordinate point (13.3), and that no fact
/// reaches, is left free: the viewer places it next to what it is attached to, and the report
/// says so. Placed, the LP would have left it where the witness of the resolution is, such as the
/// origin. A text on the wire whose y a fact gives is placed, its x where the viewer puts a label
/// of the wire, in the middle of its two points, which no fact reaches and the grid does not move.
#[test]
fn a_text_on_a_wire_or_a_net_that_no_fact_reaches_is_left_to_the_viewer() {
    let (p, solved, e) = solve_text(&netlist(TEXTS_ON_WIRES));
    let attached = |unnamed: fn(&Filler) -> bool| {
        let found = p.objects.iter().position(|o| {
            o.reference != Reference::Name("%l".into()) && o.text.as_ref().is_some_and(|t| unnamed(&t.attach))
        });
        found.unwrap()
    };
    let placed = |k: usize| solved.solution.placements.iter().find(|pl| pl.object == k).and_then(|pl| pl.at());
    let on_wire = attached(|f| matches!(f, Filler::Wire(_)));
    let on_net = attached(|f| matches!(f, Filler::Net(n) if n == "b"));
    for k in [on_wire, on_net] {
        assert!(left_free(&e.structure, k), "{}", p.objects[k].label);
        assert_eq!(placed(k), None, "{}", p.objects[k].label);
    }
    assert!(
        solved.report.notes.iter().any(|n| n.starts_with("2 texts that no fact kept reaches are left free")),
        "{:#?}",
        solved.report.notes
    );
    let l = p.objects.iter().position(|o| o.reference == Reference::Name("%l".into())).unwrap();
    assert!(!left_free(&e.structure, l));
    assert_eq!(e.structure.objects[l].fact_axes, [false, true]);
    assert_eq!(grounded(&p, &e.structure, &e.candidate, &e.placed.geometry.orient)[l], [true, true]);
    let (x, y) = placed(l).expect("the aligned text is placed");
    assert_eq!(y, to_f64(&e.placed.origins[object(&p, "#R2")][1]));
    // in the middle of the wire's two points, the pins of R1 and R2, on x
    let pin_x = |r: &PointRef| {
        let PointRef::Pin { object, pin, .. } = r else { panic!("{r:?}") };
        let at = e.structure.pin_offset(*object, *pin, e.placed.geometry.orient[*object]).unwrap();
        &e.placed.origins[*object][0] + &at[0]
    };
    let middle = (pin_x(&p.wires[0].anchors[0]) + pin_x(&p.wires[0].anchors[1])) / int(2);
    assert_eq!(from_f64(x), Some(middle), "{x}");
}

/// A supply, ground or port symbol without a name (5.9) that no fact reaches and that is on no net
/// whose pins the problem shows, as no ROUTE or wire written draws one, has no pin to go beside:
/// the solution leaves it free, neither placed nor oriented, it is no component and no symbol
/// that others are put apart from, and the report says so. One with a name is placed as before,
/// and one without a name on a net that a wire draws goes below its pin.
#[test]
fn a_symbol_without_a_name_on_no_net_drawn_is_left_free() {
    let (p, solved, e) = solve_text(&netlist(concat!(
        "R1 a b 1k\n",
        "R2 b c 1k\n",
        "* @RNL {\n",
        "*   &b @WIRE {#R1:2 #R2:1};\n",
        "*   $GND @ATTACH &c;\n",
        "*   %g = $GND @ATTACH &a;\n",
        "*   $GND @ATTACH &b\n",
        "* }\n",
    )));
    let grounds: Vec<usize> = (0..p.objects.len()).filter(|&k| e.structure.objects[k].role == Role::Ground).collect();
    let [on_c, named, on_b] = grounds.as_slice() else { panic!("{grounds:?}") };
    let (on_c, named, on_b) = (*on_c, *named, *on_b);
    assert!(matches!(p.objects[on_c].reference, Reference::Number { .. }), "{:?}", p.objects[on_c]);
    assert_eq!(p.objects[named].reference, Reference::Name("%g".into()));
    assert!(left_free(&e.structure, on_c));
    assert!(solved.solution.placements.iter().all(|pl| pl.object != on_c), "{:?}", solved.solution.placements);
    assert!(!e.structure.components().any(|k| k == on_c) && !crate::coords::solid(&e.structure, on_c));
    assert!(e.placed.overlaps.iter().all(|o| o.a != on_c && o.b != on_c));
    let note = "1 symbol without a name that no fact kept reaches is left free";
    assert!(solved.report.notes.iter().any(|n| n.starts_with(note)), "{:#?}", solved.report.notes);
    // the one with a name is placed, though nothing places it either
    let placed = |k: usize| solved.solution.placements.iter().find(|pl| pl.object == k).and_then(|pl| pl.at());
    assert!(!left_free(&e.structure, named) && placed(named).is_some());
    // the one on the net drawn goes below the first pin of an element on it, R1's
    let ori = &e.placed.geometry.orient;
    let attach = crate::search::attachments(&e.structure, &e.candidate.rows, ori);
    assert_eq!(attach.get(&on_b).map(|a| (a.host.0, a.side)), Some((object(&p, "#R1"), crate::search::Side::Below)));
    assert!(!left_free(&e.structure, on_b) && placed(on_b).is_some());
    assert_eq!(grounded(&p, &e.structure, &e.candidate, ori)[on_b], [true, true]);
    let host = attach[&on_b].host;
    let hpin = e.structure.pin_at(&e.placed.origins, ori, host.0, host.1).unwrap();
    let gpin = e.structure.pin_at(&e.placed.origins, ori, on_b, 0).unwrap();
    assert_eq!(gpin[0], hpin[0]);
    assert!(gpin[1] < hpin[1], "{gpin:?} below {hpin:?}");
}

/// Nothing is placed where only the witness of the resolution puts it, in every case of the
/// solver and of the corpus, `.sp` and `.scs`, of 0.2 and of the draft 0.3: every object the
/// solution places is held on each axis by a fact kept, a relation of the candidate taken, an
/// element of its rows or what it is attached to ([`grounded`]), but for the points `(_, _)` that
/// are not drawn, the axes of `$MIRROR_PAIR` and the middle point of `$TAIL_CENTER` (T3, T4, corpus
/// 03 and 15) and a point aligned on x only (corpus 10), on y (11.1), and the ports that rules
/// make, which the record names (10.5), on nets that nothing draws: no pin of an element on them
/// is known to the solver, which places them as before (`crate::structure::ObjectInfo::left_free`);
/// in the files of 0.3 too, now that their `$IO_PORT` is told a port by its drawing, where it was an
/// element of the rows.
/// What it leaves free is the bus label of corpus 09, on a wire (13.3), and the ground without a
/// name of the subcircuit of 0.3/09, on a net that nothing draws; the solution gives it no
/// orientation either, and nothing that the candidate puts beside a pin is left free.
#[test]
fn nothing_is_placed_where_only_the_witness_puts_it() {
    let mut unheld: BTreeMap<String, usize> = BTreeMap::new();
    let mut left = Vec::new();
    for f in every_netlist() {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        for p in problems(&f) {
            let (solved, e) = solve_exact(&p, &SolverOptions::without_time_limit());
            let ori = &e.placed.geometry.orient;
            let g = grounded(&p, &e.structure, &e.candidate, ori);
            for pl in solved.solution.placements.iter().filter(|pl| pl.at().is_some()) {
                let o = &p.objects[pl.object];
                let free: Vec<usize> = (0..2).filter(|&a| !g[pl.object][a]).collect();
                if free.is_empty() || o.kind == ObjectKind::Point {
                    continue;
                }
                let what = format!("{name} / {}: {} on {free:?}", p.title, o.label);
                let port = e.structure.objects[pl.object].role == Role::Port;
                assert!(port && matches!(o.reference, Reference::Generated { .. }), "{what}");
                *unheld.entry(format!("{name} / {}", p.title)).or_default() += 1;
            }
            for k in (0..p.objects.len()).filter(|&k| left_free(&e.structure, k)) {
                left.push(format!("{name} / {}: {}", p.title, p.objects[k].label));
                assert!(solved.solution.placements.iter().all(|pl| pl.object != k), "{name} / {}", p.title);
            }
            for k in crate::search::attachments(&e.structure, &e.candidate.rows, ori).keys() {
                assert!(!left_free(&e.structure, *k), "{name} / {}: {}", p.title, p.objects[*k].label);
            }
        }
    }
    let unheld: Vec<(&str, usize)> = unheld.iter().map(|(k, n)| (k.as_str(), *n)).collect();
    assert_eq!(
        unheld,
        [
            ("02-dialect-rules.scs / cell", 2),
            ("02-dialect-rules.scs / mid", 2),
            ("02-dialect-rules.scs / spsub", 2),
            ("03-written-rules.sp / resistor", 2),
            ("04-no-rule-packages.scs / cell", 2),
            ("06-symbols.sp / shapes", 2),
            ("07-virtuoso-netlist.scs / inv", 2),
            ("08-styles-and-text.sp / opamp", 3),
            ("09-buses-and-bundles.sp / dff", 3),
            ("09-buses-and-bundles.sp / reg4", 1),
            ("09-include-without-header.sp / divider", 2),
            ("12-diagnostics.sp / sub", 2),
            ("13-numbering-order.sp / cell", 2),
            ("14-numbering-first-name.sp / first_p", 1),
            ("14-numbering-first-name.sp / first_q", 1),
            ("15-numbering-stale-and-errors.sp / stale_sub", 1),
            ("18-dialect-switch.scs / box_cell", 2),
            ("18-dialect-switch.scs / sub_dropped", 2),
            ("18-dialect-switch.scs / sub_inc", 2),
            ("18-dialect-switch.scs / sub_kept", 2),
            ("18-dialect-switch.scs / sub_sec_scs", 2),
            ("18-dialect-switch.scs / sub_sec_sp", 2),
            ("19-header-and-lib-versions.sp / sub_02", 2),
            ("19-header-and-lib-versions.sp / sub_03", 2),
            ("19-header-and-lib-versions.sp / sub_late", 2),
            ("19-set-cycles.sp / cell", 2),
            ("21-scope-bindings.sp / cell", 2),
            ("23-generation-set-difference.sp / cell", 1),
            ("inverter-chain.scs / inv", 1),
            ("inverter-chain.sp / inv", 1),
        ]
    );
    assert_eq!(
        left,
        [
            "09-buses-and-bundles.sp / 09-buses-and-bundles.sp: a text",
            "09-include-without-header.sp / divider: a usage of $GND",
        ]
    );
}

/// Where the texts of the symbols are drawn (13.3): each text that its owner's symbol places, its
/// box as wide as it is laid out (`crate::structure::text_box`) about the owner's origin and turned
/// with it, with its owner.
fn symbol_texts(p: &LevelProblem, e: &Exact) -> Vec<(usize, [Q; 4])> {
    let (at, ori) = (&e.placed.origins, &e.placed.geometry.orient);
    let mut out = Vec::new();
    for o in &p.objects {
        let (Position::InSymbol { owner, local, replaced: None }, Some(t)) = (&o.position, &o.text) else { continue };
        let local = (from_f64(local.0).unwrap(), from_f64(local.1).unwrap());
        let Some(b) = crate::structure::text_box(t, &local, ori[*owner]) else { continue };
        let a = &at[*owner];
        out.push((*owner, [&a[0] + &b[0], &a[1] + &b[1], &a[0] + &b[2], &a[1] + &b[3]]));
    }
    out
}

/// The members of a DISTRIBUTE on x stay in its order, one way or the other, so that its even
/// steps keep the gaps between neighbours, and no text is drawn over another symbol (review of
/// phase 2 of the quality review). Without groups, area picks the candidate; a swap of neighbours
/// broke the order of the DISTRIBUTE, its row taken as [C4 R2 R3 R1 VIN], and the resolution
/// dropped a gap: R1, R2, R3 and C4 two apart, R2's `100meg` over R1's body (S2). A random sweep
/// found the same of `{#R1 #V2 #R0} @DISTRIBUTE("x")`, V2's `SIN(0 1 1meg)` over R0's body.
#[test]
fn the_members_of_a_distribute_keep_its_order_and_their_texts_clear() {
    let s2 = "VIN in 0 DC 1\nR1 in 0 1\nR2 in 0 100meg\nR3 in 0 4.7k\nC4 in 0 10p\n\
              * @RNL { {#R1 #R2 #R3 #C4} @DISTRIBUTE(\"x\"); {#R1 #R2 #R3 #C4} @ALIGN(\"y\"); $WIRE_NET_SET @ROUTE }\n";
    let sweep = "R0 a 0 1k\nR1 a b 1k\nV2 b 0 SIN(0 1 1meg)\n\
                 * @RNL { {#R1 #V2 #R0} @DISTRIBUTE(\"x\"); $WIRE_NET_SET @ROUTE }\n";
    for (body, members) in [(s2, &["#R1", "#R2", "#R3", "#C4"][..]), (sweep, &["#R1", "#V2", "#R0"][..])] {
        let (p, solved, e) = solve_text(&netlist(body));
        let what = summary(&p, &solved, &e);
        assert!(!e.placed.dropped.contains(&crate::search::RelKind::Gap), "{:?}\n{what}", e.placed.dropped);
        // in the order of the DISTRIBUTE, one way or the other, apart
        let xs: Vec<&Q> = members.iter().map(|m| &e.placed.origins[object(&p, m)][0]).collect();
        let (up, down) = (xs.windows(2).all(|w| w[0] < w[1]), xs.windows(2).all(|w| w[0] > w[1]));
        assert!(up || down, "{xs:?}\n{what}");
        let ks: Vec<usize> = members.iter().map(|m| object(&p, m)).collect();
        let row = e.candidate.rows.iter().find(|r| r.contains(&ks[0])).unwrap();
        let places: Vec<usize> = ks.iter().filter_map(|k| row.iter().position(|j| j == k)).collect();
        assert_eq!(places.len(), ks.len(), "{:?}\n{what}", e.candidate.rows);
        assert!(
            places.windows(2).all(|w| w[0] < w[1]) || places.windows(2).all(|w| w[0] > w[1]),
            "{:?}\n{what}",
            e.candidate.rows
        );
        // no text over another symbol: as the solver counts them, and each text as it is drawn
        let (s, at, ori) = (&e.structure, &e.placed.origins, &e.placed.geometry.orient);
        assert_eq!(crate::score::texts_over(s, at, ori), 0, "{what}");
        let texts = symbol_texts(&p, &e);
        assert!(texts.len() >= 2 * members.len(), "{texts:?}\n{what}");
        let meet = |a: &[Q; 4], b: &[Q; 4]| a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3];
        // a stub and its host go together
        let host = |k: usize| s.objects[k].stub.as_ref().map(|stub| stub.host.0);
        for &(k, ref t) in &texts {
            for j in s.components().filter(|&j| j != k && host(j) != Some(k) && host(k) != Some(j)) {
                let Some(b) = s.box_at(at, ori, j) else { continue };
                assert!(!meet(t, &b), "{} over {}: {what}", p.objects[k].label, p.objects[j].label);
            }
        }
    }
}

/// Where every pin of every component is drawn, by the component and the pin.
fn component_pins(e: &Exact) -> Vec<((usize, usize), [Q; 2])> {
    let (s, at, ori) = (&e.structure, &e.placed.origins, &e.placed.geometry.orient);
    let mut out = Vec::new();
    for k in s.components() {
        out.extend((0..s.pins(k).len()).filter_map(|pin| Some(((k, pin), s.pin_at(at, ori, k, pin)?))));
    }
    out
}

/// Whether two solves of one netlist draw every pin of every component at the same place, up to a
/// shift and, with `mirror`, a mirror about an upright line.
fn pins_alike(a: &Exact, b: &Exact, mirror: bool) -> bool {
    let (pa, pb) = (component_pins(a), component_pins(b));
    if pa.len() != pb.len() || pa.iter().zip(&pb).any(|(x, y)| x.0 != y.0) {
        return false;
    }
    let x = |p: &[Q; 2], q: &[Q; 2]| if mirror { &p[0] + &q[0] } else { &q[0] - &p[0] };
    let Some(((_, p0), (_, q0))) = pa.first().zip(pb.first()) else { return true };
    let (dx, dy) = (x(p0, q0), &q0[1] - &p0[1]);
    pa.iter().zip(&pb).all(|((_, p), (_, q))| x(p, q) == dx && &q[1] - &p[1] == dy)
}

/// A DISTRIBUTE has no direction (14.4): its members written the other way round mean the same,
/// and give the same drawing, or its mirror (review of the fix of phase 2: where the witness puts
/// all the members at one x, their order was taken as written, and drove the first plan, the
/// barycentre rounds and the swaps; S2 written `{#C4 #R3 #R2 #R1}` was drawn with its parts
/// turned, two crossings and wires half as long again, and a sweep case's step grew from 8 to 19).
/// A DISTRIBUTE of two members holds wherever they are, and orders nothing.
#[test]
fn a_distribute_written_the_other_way_round_is_drawn_alike() {
    let s2 = "VIN in 0 DC 1\nR1 in 0 1\nR2 in 0 100meg\nR3 in 0 4.7k\nC4 in 0 10p\n\
              * @RNL { {#R1 #R2 #R3 #C4} @DISTRIBUTE(\"x\"); {#R1 #R2 #R3 #C4} @ALIGN(\"y\"); $WIRE_NET_SET @ROUTE }\n";
    let sweep = "R0 a 0 1k\nR1 a b 1k\nV2 b 0 SIN(0 1 1meg)\n\
                 * @RNL { {#R1 #V2 #R0} @DISTRIBUTE(\"x\"); $WIRE_NET_SET @ROUTE }\n";
    let grouped = "V0 n1 n2 DC 0 AC 1\nR1 n2 n1 4.7k\nV2 n1 0 DC 1\nV3 n2 0 PULSE(0 1 0 1n 1n 5n 10n)\n\
                   * @RNL { %g1 = {#V0 #V3 #R1} @GROUP; {#V3 #R1 #V0} @DISTRIBUTE(\"x\"); {#V2 #R1} @ALIGN(\"y\"); \
                   $WIRE_NET_SET @ROUTE }\n";
    let two = "VIN a 0 DC 1\nR1 a b 1k\nR2 b 0 100meg\nC3 b 0 10p\n\
               * @RNL { {#R1 #R2} @DISTRIBUTE(\"x\"); {#VIN #R1 #R2 #C3} @ALIGN(\"y\"); $WIRE_NET_SET @ROUTE }\n";
    let cases = [
        (s2, "{#R1 #R2 #R3 #C4}", "{#C4 #R3 #R2 #R1}"),
        (sweep, "{#R1 #V2 #R0}", "{#R0 #V2 #R1}"),
        (grouped, "{#V3 #R1 #V0}", "{#V0 #R1 #V3}"),
        (two, "{#R1 #R2} @DISTRIBUTE", "{#R2 #R1} @DISTRIBUTE"),
    ];
    for (body, written, reversed) in cases {
        assert!(body.contains(written), "{body}");
        let (p, solved, e) = solve_text(&netlist(body));
        let (q, again, f) = solve_text(&netlist(&body.replacen(written, reversed, 1)));
        let (what, other) = (summary(&p, &solved, &e), summary(&q, &again, &f));
        assert!(pins_alike(&e, &f, false) || pins_alike(&e, &f, true), "{written}:\n{what}\n{reversed}:\n{other}");
        assert_eq!(e.placed.dropped, f.placed.dropped, "{what}\n{other}");
    }
}

// ------------------------------------------------------------------------ supply, ground and ports by their drawing

/// A common-source stage of a PMOS load and an NMOS driver, in the language version given, with
/// the supply net marked so that the fill of the standard package hangs stubs from the pins on the
/// supply and the ground, and a port on the output.
fn stage(version: &str) -> String {
    format!(
        "* cs\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\n.model nch nmos level=1\n.model pch pmos level=1\n\
         M1 out in vdd vdd pch\nM2 out in 0 0 nch\n\
         * @RNL {{\n\
         *   # @SELECT-INSTANCE(model=\"nch\") => @SET-SYMBOL $NMOS;\n\
         *   # @SELECT-INSTANCE(model=\"pch\") => @SET-SYMBOL $PMOS;\n\
         *   &vdd @ADD-TO $SUPPLY_NET_SET;\n\
         *   $WIRE_NET_SET @ROUTE\n\
         * }}\n.end\n"
    )
}

/// The stubs of the standard series 0.3 (`$GROUND_STUB`, `$SUPPLY_STUB`) are told by their drawing
/// and their facts, not by their names: the grounds hang below their pins as stubs, not as elements
/// of the rows, the current direction and the ground direction are in force, and the current depth
/// puts the PMOS above the NMOS, each upright (the review of the solver's drawings of 0.3,
/// 2026-10-24: every ground was an element of the bottom row, and the transistors lay any way).
#[test]
fn the_stubs_of_the_series_0_3_hang_from_their_pins_by_their_drawing() {
    use num_traits::Signed;
    let (p, solved, e) = solve_text(&stage("0.3"));
    let what = summary(&p, &solved, &e);
    let s = &e.structure;
    let [m1, m2] = ["#M1", "#M2"].map(|l| object(&p, l));
    assert!(solved.report.inert_objectives.iter().all(|i| i.starts_with("terminal_wire_alignment")), "{what}");
    assert!(!s.power.ground.is_empty() && !s.power.supply.is_empty(), "{what}");
    for &k in s.power.ground.iter().chain(&s.power.supply) {
        let stub = s.objects[k].stub.as_ref().unwrap_or_else(|| panic!("{} is no stub: {what}", p.objects[k].label));
        let up = s.power.supply.contains(&k);
        assert_eq!(stub.rise.is_positive(), up, "{}: {what}", p.objects[k].label);
        assert!(stub.host.0 == m1 || stub.host.0 == m2, "{what}");
    }
    assert_eq!(s.elements, vec![m1, m2], "{what}");
    let depth = s.depth.as_ref().unwrap_or_else(|| panic!("no current depth: {what}"));
    assert_eq!((depth[m1], depth[m2]), (Some(0), Some(1)), "{what}");
    let at = &e.placed.origins;
    assert!(at[m1][1] > at[m2][1], "{what}");
    // each source towards its rail: the PMOS's up, the NMOS's down
    let pin = |k: usize, terminal: &str| {
        let i = s.pins(k).iter().position(|q| q.terminals.iter().any(|t| t == terminal)).unwrap();
        s.pin_at(at, &e.placed.geometry.orient, k, i).unwrap()
    };
    assert!(pin(m1, "3")[1] > pin(m1, "1")[1] && pin(m2, "3")[1] < pin(m2, "1")[1], "{what}");
}

/// A port of the standard series 0.3 (`$IO_PORT`) is a port, which goes beside the pin it joins,
/// not an element of the rows.
#[test]
fn a_port_of_the_series_0_3_is_a_port_by_its_drawing() {
    let (p, solved, e) = solve_text(
        "* p\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a out 1k\nR2 out 0 1k\n\
         * @RNL { $IO_PORT @ATTACH &out => %po; $WIRE_NET_SET @ROUTE }\n.end\n",
    );
    let what = summary(&p, &solved, &e);
    let po = object(&p, "%po");
    assert_eq!(e.structure.objects[po].role, crate::structure::Role::Port, "{what}");
    assert!(!e.structure.elements.contains(&po), "{what}");
}

/// The same stage in the series 0.2 and 0.3 has the same supply, ground, current depth and rows:
/// what the solver reads of the drawing does not depend on the names the series gives.
#[test]
fn the_series_0_2_and_0_3_give_the_same_structure() {
    let shape = |version: &str| {
        let (p, solved, e) = solve_text(&stage(version));
        let s = &e.structure;
        let label = |k: &usize| p.objects[*k].label.clone();
        let hosts = |ks: &[usize]| -> Vec<String> {
            let mut v: Vec<String> =
                ks.iter().filter_map(|k| s.objects[*k].stub.as_ref()).map(|st| label(&st.host.0)).collect();
            v.sort();
            v
        };
        let rows: Vec<Vec<String>> = s.rows.iter().map(|r| r.iter().map(label).collect()).collect();
        let depth: Vec<(String, Option<u32>)> =
            s.elements.iter().map(|k| (label(k), s.depth.as_ref().and_then(|d| d[*k]))).collect();
        (hosts(&s.power.supply), hosts(&s.power.ground), rows, depth, solved.report.inert_objectives.clone())
    };
    assert_eq!(shape("0.2"), shape("0.3"));
}
