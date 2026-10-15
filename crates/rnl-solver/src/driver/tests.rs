//! Tests of a solve: small levels built in code, then every level of the cases written for the
//! solver and of the conformance corpus read from the files.

use super::*;
use crate::resolve::tests::{fact, instance, point, problem_of};
use crate::Objectives;
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{Axis, FactKind, PointRef};

#[test]
fn an_empty_level_gives_an_empty_solution_and_says_what_is_done() {
    let p = problem_of(Vec::new(), Vec::new());
    let solved = crate::solve(&p, &SolverOptions::default());
    assert_eq!(solved.solution, Solution::default());
    assert_eq!(solved.report.notes[0], STAGE_NOTE);
    assert!(solved.report.kept.is_empty() && solved.report.discarded.is_empty());
    assert_eq!(solved, crate::solve(&p, &SolverOptions::default()));
}

#[test]
fn options_that_make_no_sense_are_noted() {
    let mut options = SolverOptions { beam: 0, ..SolverOptions::default() };
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
    let solved = crate::solve(&p, &SolverOptions::default());
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

// ------------------------------------------------------------------------------------ the files

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
    let s = &e.structure;
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
        with_relations(s, &rels, SolverOptions::default().budget, &mut work, |_, _, kept| !kept[kept.len() - 1])
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
    lines.join("\n")
}

/// Solves a level and checks it (see the tests below).
fn check_level(file: &str, p: &LevelProblem) -> (Solved, Exact) {
    let options = SolverOptions::default();
    let (solved, e) = solve_exact(p, &options);
    let what = format!("{file} / {}", p.title);
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
    // no two instances overlap unless the facts make them
    for &(a, b) in &e.placed.overlaps {
        let instance = |k: usize| matches!(p.objects[k].kind, ObjectKind::Instance | ObjectKind::Bundle);
        if instance(a) && instance(b) {
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
    assert_eq!(solved, crate::solve(p, &options), "{what}: not deterministic");
    for pl in &solved.solution.placements {
        if let Some((x, y)) = pl.at {
            assert!(x.is_finite() && y.is_finite(), "{what}: {pl:?}");
        }
    }
    // every object with a position of its own placed, and every orientation given
    let own = p.objects.iter().filter(|o| matches!(o.position, Position::Own { .. })).count();
    assert_eq!(solved.solution.placements.iter().filter(|pl| pl.at.is_some()).count(), own, "{what}");
    for pl in &solved.solution.placements {
        assert_eq!(pl.orient.is_some(), p.objects[pl.object].orient.is_some(), "{what}: {pl:?}");
    }
    (solved, e)
}

#[test]
fn every_case_of_the_solver_is_placed_keeping_every_fact_kept() {
    let files = netlists("tests/integration/solver", "T");
    assert!(files.len() >= 7, "{files:?}");
    for f in files {
        for p in problems(&f) {
            let (solved, e) = check_level(&f.to_string_lossy(), &p);
            eprintln!("{}", summary(&p, &solved, &e));
        }
    }
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
    // the gates: the pair's outwards, the mirror's facing each other
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
        let options = SolverOptions::parse(text).unwrap();
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
    let solved = crate::solve(&ps[0], &SolverOptions::parse("objectives=none").unwrap());
    assert!(solved.report.inert_objectives.is_empty());
}
