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
            Placement { object: 0, at: Some((4.0, 2.5)), orient: Some(Orient::MY) },
            Placement { object: 1, at: Some((0.0, 2.5)), orient: None },
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
    let at: Vec<(usize, Option<(f64, f64)>)> = solved.solution.placements.iter().map(|pl| (pl.object, pl.at)).collect();
    assert_eq!(
        at,
        vec![(0, Some((-10.0, 2.0))), (1, Some((3.0, 4.0))), (2, Some((3.0, -10.0))), (3, Some((-10.0, 8.0)))]
    );
    assert!(solved.solution.placements[0].orient.is_some());
    let written = rnl_runtime::solution::write_record(&p, &solved.solution, "t", "h");
    let unsolved: Vec<String> = written.unsolved.iter().map(|u| u.to_string()).collect();
    assert_eq!(unsolved.len(), 1, "{unsolved:?}");
    assert!(unsolved[0].starts_with("the position of (7, 8): "), "{unsolved:?}");
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
    assert_eq!(text[0].at.map(|(x, _)| x), Some(13.0));
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
        lines.push(format!("  {:>24} {:?} {:?}", o.label, pl.at, pl.orient.map(|x| x.as_str())));
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
        if let Some((x, y)) = pl.at {
            assert!(x.is_finite() && y.is_finite(), "{what}: {pl:?}");
        }
    }
    // every object with a position of its own placed, those the file fixes too (plan 3.2), and
    // every text that its symbol places on one axis only, where it is (13.3); and every
    // orientation given
    for (k, o) in p.objects.iter().enumerate() {
        let pl: Vec<&Placement> = solved.solution.placements.iter().filter(|pl| pl.object == k).collect();
        assert!(pl.len() <= 1, "{what}: {pl:?}");
        let at = pl.first().and_then(|pl| pl.at);
        let orient = pl.first().and_then(|pl| pl.orient);
        let own = matches!(o.position, Position::Own { .. } | Position::InSymbol { replaced: Some(_), .. });
        assert_eq!(at.is_some(), own, "{what}: {} {pl:?}", o.label);
        if let (Some((x, y)), Position::InSymbol { .. }) = (at, &o.position) {
            let origin = &e.placed.origins[k];
            assert_eq!((from_f64(x).as_ref(), from_f64(y).as_ref()), (Some(&origin[0]), Some(&origin[1])), "{what}");
        }
        let oriented = o.orient.is_some() && !matches!(o.position, Position::InSymbol { .. });
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
/// twice. The solver's own resolutions have caps of two nodes and 30 pivots.
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
    // the ports beside what they connect to: inp left of M1's gate, inn right of M2's, out right
    // of M4's drain, vbias left of M5's gate
    let port = |net: &str| s.ports.iter().find(|q| q.net.as_deref() == Some(net)).unwrap().object;
    let tag = |net: &str| s.pin_at(at, &g.orient, port(net), 0).unwrap();
    for (net, host, terminal, left) in
        [("inp", m1, "2", true), ("inn", m2, "2", false), ("out", m4, "1", false), ("vbias", m5, "2", true)]
    {
        let (t, h) = (tag(net), pin(host, terminal));
        assert_eq!(t[1], h[1], "{what}: {net}");
        assert!(if left { t[0] < h[0] } else { t[0] > h[0] }, "{what}: {net}");
    }
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
        let s = analyse(p, &resolved, &options, &mut work);
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
        let s = analyse(p, &resolved, &options, &mut work);
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

/// Rows without facts put pins of two nets face to face across the gap of 2 between neighbours,
/// where both lead to one node: the net routed second leaves its pin along the edge of its
/// symbol, and every net is one tree, with and without the objectives (review of S4).
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

/// T3: the two wires of &out that meet at M4's drain end at a branch point there with a junction,
/// which a wire of length 0 joins to the drain (13.2; review of S4).
#[test]
fn t3_has_a_junction_where_two_wires_meet_at_a_drain() {
    let ps = problems(&repo().join("tests/integration/solver/T3-ota5t.sp"));
    let p = ps.iter().find(|p| p.title == "ota5t").unwrap();
    let (solved, e) = check_level("T3", p);
    let m4 = object(p, "#M4");
    let drain = |q: &rnl_runtime::solution::WirePoint| matches!(q, rnl_runtime::solution::WirePoint::Anchor(PointRef::Pin { object, terminal, .. }) if *object == m4 && terminal == "d");
    let at_drain: Vec<(usize, &rnl_runtime::solution::RoutedWire)> = solved
        .solution
        .wires
        .iter()
        .enumerate()
        .filter(|(_, w)| p.routes[w.route].net == "out" && w.points.iter().any(drain))
        .collect();
    assert_eq!(at_drain.len(), 1, "{}", summary(p, &solved, &e));
    let (i, w) = at_drain[0];
    let path = &e.routing.paths[i];
    assert_eq!(path[0], path[1], "of length 0: {w:?}");
    let rnl_runtime::solution::WirePoint::Branch(b) = w.points[0] else { panic!("{w:?}") };
    assert!(solved.solution.branches[b].junction);
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
