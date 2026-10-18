//! The resolution of 8.5 on documents read from files: the cases written for the solver
//! (`tests/integration/solver`, T1–T6) and every case of the conformance corpus
//! (`tests/conformance/cases`). Each level is read as a solver reads it (`Options::solver()`, the
//! records of the solver left out, 2.8) and resolved; then:
//!
//! - every fact is kept or discarded, once: none contradicts itself (the draft 0.3, 8.5);
//! - every fact kept holds where the witness puts the objects, checked as a viewer checks a
//!   constraint (14.1–14.5, 14.9), in `f64` with the tolerance (11.7);
//! - each fact discarded has all its minimal conflict groups (the draft 0.3, 8.5), but a fact of
//!   priority low, which has its first only unless its hints are asked for (8.5, 7.6), and each
//!   group is made of facts kept before its fact, conflicts with it, and is minimal: the level with
//!   only those facts and the fact discarded discards it again, and without any one of them keeps
//!   them all; the hints by statement are those of the groups of the facts whose hints are given;
//! - a second run gives the same, and the placement of `solve` has no NaN.
//!
//! T4 has a reference resolution (the brief of the solver brainstorm): exactly three facts are
//! discarded, `{#M6 #M7} @ALIGN("x")`, the ALIGN y that the `$MIRROR_PAIR` of M1 and M2 expands
//! to, and the low `{#VIP #VIN} @ALIGN("y")`, each with one minimal group, and the statements of
//! those groups are those of lines 37 to 40 of the file, line 40 only with the hints of the facts
//! of priority low. Corpus 03 discards a low ORDER whose groups cost millions of nodes: by default
//! they are not sought.
//!
//! The solver itself does no file I/O; these tests read the files with `std`.

use std::path::{Path, PathBuf};

use rnl_runtime::geom::Orient;
use rnl_runtime::interp::obj::Priority;
use rnl_runtime::problem::*;
use rnl_runtime::source::Provider;
use rnl_runtime::Options;
use rnl_solver::{resolve, solve, Discarded, Resolved, SolverOptions, Statement};

/// Files from the disk, as the CLI reads them.
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
    rnl_view::interpret(&entry, None, &Files, &Options::solver()).unwrap_or_else(|e| panic!("{entry}: {e}")).problems
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

/// Whether the witness satisfies a fact, as a viewer checks a constraint: in `f64`, a difference
/// within `tol` counting as none (11.7).
fn holds(p: &LevelProblem, r: &Resolved, kind: &FactKind, tol: f64) -> bool {
    let at = |m: &PointRef, axis: Axis| {
        let (x, y) = r.point(p, m).unwrap_or_else(|| panic!("no point for {m:?}"));
        if axis == Axis::X {
            x
        } else {
            y
        }
    };
    let ori = |k: usize| r.orientation(k).unwrap_or(Orient::R0);
    match kind {
        FactKind::Place { at: m, axis, value } => (at(m, *axis) - value).abs() <= tol,
        FactKind::PointCoordinate { point, axis, value } => (at(&PointRef::Origin(*point), *axis) - value).abs() <= tol,
        FactKind::Offset { text, from, axis, value } => {
            (at(&PointRef::Origin(*text), *axis) - at(from, *axis) - value).abs() <= tol
        }
        FactKind::Orient { object, code } => ori(*object) == *code,
        FactKind::OrientRelation { relation, from, to } => {
            let (Some(a), Some(b)) = (from.first(), to.first()) else { return true };
            from.iter().all(|k| ori(*k) == ori(*a))
                && to.iter().all(|k| ori(*k) == ori(*b))
                && ori(*b) == ori(*a).then(*relation)
        }
        FactKind::Align { axis, members } => {
            let v: Vec<f64> = members.iter().map(|m| at(m, *axis)).collect();
            v.iter().all(|x| (x - v[0]).abs() <= tol)
        }
        FactKind::Order { axis, sense, members } => {
            let v: Vec<f64> = members.iter().map(|m| at(m, *axis)).collect();
            let inc = v.windows(2).all(|w| w[1] - w[0] >= 1.0 - tol);
            let dec = v.windows(2).all(|w| w[0] - w[1] >= 1.0 - tol);
            match sense {
                Sense::Increasing => inc,
                Sense::Decreasing => dec,
                Sense::Either => inc || dec,
            }
        }
        FactKind::Distribute { axis, members } => {
            let v: Vec<f64> = members.iter().map(|m| at(m, *axis)).collect();
            v.len() < 3 || v.windows(2).all(|w| ((w[1] - w[0]) - (v[1] - v[0])).abs() <= tol)
        }
        FactKind::Distance { axis, sense, min, max, from, to } => {
            let extent = |ms: &[PointRef]| {
                let v: Vec<f64> = ms.iter().map(|m| at(m, *axis)).collect();
                (v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max))
            };
            if from.is_empty() || to.is_empty() {
                return true;
            }
            let (a, b) = (extent(from), extent(to));
            let d = match sense {
                Sense::Increasing => b.0 - a.1,
                Sense::Decreasing => a.0 - b.1,
                Sense::Either => (b.0 - a.1).max(a.0 - b.1),
            };
            min.is_none_or(|m| d >= m - tol) && max.is_none_or(|m| d <= m + tol)
        }
    }
}

/// The level with only some of its facts, in the order given.
fn with_facts(p: &LevelProblem, facts: &[usize]) -> LevelProblem {
    LevelProblem { facts: facts.iter().map(|&f| p.facts[f].clone()).collect(), ..p.clone() }
}

/// Resolves a level and checks everything the module says.
fn check_level(file: &str, p: &LevelProblem) -> Resolved {
    check_with(file, p, &SolverOptions::without_time_limit())
}

/// Resolves a level with some options and checks everything the module says.
fn check_with(file: &str, p: &LevelProblem, options: &SolverOptions) -> Resolved {
    let options = options.clone();
    let r = resolve(p, &options);
    let what = format!("{file} / {}", p.title);
    let mut all: Vec<usize> = r.kept.iter().copied().chain(r.discarded.iter().map(|d| d.fact)).collect();
    all.sort_unstable();
    assert_eq!(all, (0..p.facts.len()).collect::<Vec<_>>(), "{what}");
    // no fact of the cases contradicts itself (the draft 0.3, 8.5)
    assert!(r.failed.is_empty() && r.errors.is_empty(), "{what}: {:?}", r.errors);
    // the witness is exact; converting it to f64 may cross the edge of the tolerance by rounding
    let tol = options.tolerance.numer().to_string().parse::<f64>().unwrap()
        / options.tolerance.denom().to_string().parse::<f64>().unwrap()
        + 1e-9;
    for &f in &r.kept {
        assert!(holds(p, &r, &p.facts[f].kind, tol), "{what}: fact {f} kept but violated: {:?}", p.facts[f]);
    }
    // a fact of priority low has its first group only, unless its hints are asked for (8.5, 7.6)
    let hinted = |d: &Discarded| options.hints_for_low || p.facts[d.fact].priority != Priority::Low;
    for d in &r.discarded {
        if hinted(d) {
            assert!(!d.undecided && d.complete && !d.groups.is_empty(), "{what}: {d:?}");
        } else {
            assert!(!d.undecided && !d.complete && d.groups.len() == 1, "{what}: {d:?}");
        }
        assert!(d.groups.windows(2).all(|w| w[0] < w[1]), "{what}: {d:?} not sorted");
        for group in &d.groups {
            assert!(group.windows(2).all(|w| w[0] < w[1]), "{what}: {d:?} not sorted");
            assert!(group.iter().all(|c| *c < d.fact && r.kept.contains(c)), "{what}: {d:?}");
            let set: Vec<usize> = group.iter().copied().chain([d.fact]).collect();
            let sub = resolve(&with_facts(p, &set), &options);
            assert_eq!(sub.kept.len(), set.len() - 1, "{what}: {group:?} of {d:?} does not conflict alone: {sub:?}");
            for c in group {
                let rest: Vec<usize> = set.iter().copied().filter(|x| x != c).collect();
                let sub = resolve(&with_facts(p, &rest), &options);
                assert!(sub.discarded.is_empty(), "{what}: {group:?} of {d:?} is not minimal without {c}");
            }
        }
    }
    // and a note says so
    let noted = r.notes.iter().any(|n| n.contains(" are listed only with --report "));
    assert_eq!(noted, r.discarded.iter().any(|d| !hinted(d)), "{what}: {:?}", r.notes);
    // the hints by statement: each statement of a fact in a group, with the facts discarded
    // because of it
    let mut named: Vec<(usize, usize)> = Vec::new();
    for (s, hint) in &r.statements {
        for &k in &hint.facts {
            assert_eq!(&Statement::of(p, k), s, "{what}");
            for &f in &hint.discarded {
                if r.discarded.iter().any(|d| d.fact == f && d.in_groups().contains(&k)) {
                    named.push((k, f));
                }
            }
        }
    }
    named.sort_unstable();
    let mut pairs: Vec<(usize, usize)> = r
        .discarded
        .iter()
        .filter(|d| hinted(d))
        .flat_map(|d| d.in_groups().into_iter().map(move |k| (k, d.fact)))
        .collect();
    pairs.sort_unstable();
    assert_eq!(named, pairs, "{what}: {:?}", r.statements);
    assert_eq!(r, resolve(p, &options), "{what}: not deterministic");
    let solved = solve(p, &options);
    assert_eq!(solved.report.kept, r.kept);
    for pl in &solved.solution.placements {
        if let Some((x, y)) = pl.at() {
            assert!(x.is_finite() && y.is_finite(), "{what}: {pl:?}");
        }
    }
    r
}

fn summary(p: &LevelProblem, r: &Resolved) -> String {
    let d: Vec<String> =
        r.discarded.iter().map(|d| format!("{} {} ⟂ {:?}", d.fact, p.facts[d.fact].label, d.groups)).collect();
    format!(
        "{}: {} facts, kept {}, discarded [{}], {} nodes, {} pivots",
        p.title,
        p.facts.len(),
        r.kept.len(),
        d.join("; "),
        r.work.resolve_nodes,
        r.work.pivots
    )
}

#[test]
fn every_case_of_the_solver_resolves() {
    let files = netlists("tests/integration/solver", "T");
    assert!(files.len() >= 7, "{files:?}");
    for f in files {
        for p in problems(&f) {
            let r = check_level(&f.to_string_lossy(), &p);
            eprintln!("{}", summary(&p, &r));
        }
    }
}

#[test]
fn every_case_of_the_corpus_resolves() {
    let files = netlists("tests/conformance/cases", "");
    assert!(files.len() >= 20, "{files:?}");
    for f in files {
        for p in problems(&f) {
            let r = check_level(&f.to_string_lossy(), &p);
            eprintln!("{}", summary(&p, &r));
        }
    }
}

#[test]
fn the_groups_of_the_low_order_of_corpus_03_are_not_sought_by_default() {
    // corpus 03 discards the low ORDER of its $FLOW, whose one minimal conflict group took about 9
    // million nodes of the search for seeds to be known as the only one. Its hints may be left out
    // (the draft 0.3, 8.5; 7.6): by default it has that group, its first, and the resolution of the
    // level costs a few hundred nodes, as before every group was sought
    let ps = problems(&repo().join("tests/conformance/cases/03-five-transistor-ota.sp"));
    let p = &ps[0];
    let r = check_level("03", p);
    let low: Vec<&Discarded> = r.discarded.iter().filter(|d| p.facts[d.fact].priority == Priority::Low).collect();
    assert_eq!(low.len(), 1, "{}", summary(p, &r));
    assert!(matches!(p.facts[low[0].fact].kind, FactKind::Order { .. }), "{:?}", p.facts[low[0].fact]);
    assert!(r.work.resolve_nodes < 10_000, "{}", summary(p, &r));
}

#[test]
fn with_its_hints_asked_for_the_low_order_of_corpus_03_has_its_one_group_found_whole() {
    // the enumeration of every minimal group on the one large case of the corpus, which `rnl solve
    // --report` runs (hints_for_low): the low ORDER [46] conflicts with [16], [19], [23] and [26]
    // alone, a list known complete after about 9 million nodes, and the facts kept are those kept
    // without it, which gives that group first (the draft 0.3, 8.5)
    let ps = problems(&repo().join("tests/conformance/cases/03-five-transistor-ota.sp"));
    let p = &ps[0];
    let r = resolve(p, &SolverOptions { hints_for_low: true, ..SolverOptions::without_time_limit() });
    let low: Vec<&Discarded> = r.discarded.iter().filter(|d| p.facts[d.fact].priority == Priority::Low).collect();
    assert_eq!(low.len(), 1, "{}", summary(p, &r));
    assert_eq!((low[0].fact, &low[0].groups, low[0].complete), (46, &vec![vec![16, 19, 23, 26]], true));
    let first = resolve(p, &SolverOptions::without_time_limit());
    assert_eq!(r.kept, first.kept);
    let d = first.discarded.iter().find(|d| d.fact == 46).unwrap();
    assert_eq!((&d.groups, d.complete), (&low[0].groups, false));
}

fn object(p: &LevelProblem, label: &str) -> usize {
    p.objects.iter().position(|o| o.label == label).unwrap_or_else(|| panic!("no object {label}"))
}

#[test]
fn t4_resolves_as_the_reference_says() {
    let ps = problems(&repo().join("tests/integration/solver/T4-miller-conflicts.sp"));
    assert_eq!(ps.len(), 1);
    let p = &ps[0];
    let r = check_level("T4", p);
    let origin = |label: &str| PointRef::Origin(object(p, label));
    let find = |what: &str, pred: &dyn Fn(&ProblemFact) -> bool| -> usize {
        let found: Vec<usize> = (0..p.facts.len()).filter(|&f| pred(&p.facts[f])).collect();
        assert_eq!(found.len(), 1, "{what}: {found:?}");
        found[0]
    };
    let align = |axis: Axis, members: Vec<PointRef>| FactKind::Align { axis, members };
    // {#M6 #M7} @ALIGN("x")
    let m6_m7 = find("ALIGN x of M6 and M7", &|f| f.kind == align(Axis::X, vec![origin("#M6"), origin("#M7")]));
    // the ALIGN y of the $MIRROR_PAIR of M1 and M2
    let mirror = find("ALIGN y of M1 and M2", &|f| {
        f.kind == align(Axis::Y, vec![origin("#M1"), origin("#M2")])
            && f.chain.iter().any(|&u| p.usages[u].definition == "$MIRROR_PAIR")
    });
    // {#VIP #VIN} @ALIGN("y", priority="low")
    let inputs = find("low ALIGN y of VIP and VIN", &|f| {
        f.kind == align(Axis::Y, vec![origin("#VIP"), origin("#VIN")]) && f.priority == Priority::Low
    });
    let discarded: Vec<usize> = r.discarded.iter().map(|d| d.fact).collect();
    let mut expected = vec![m6_m7, mirror, inputs];
    expected.sort_unstable();
    assert_eq!(discarded, expected, "{}", summary(p, &r));
    // and what each conflicts with: one minimal group each, every group it has (the draft 0.3, 8.5)
    let groups = |f: usize| r.discarded.iter().find(|d| d.fact == f).unwrap().groups.clone();
    let distance_m6_m7 = find("DISTANCE x of M6 and M7", &|f| {
        matches!(&f.kind, FactKind::Distance { axis: Axis::X, sense: Sense::Either, min: Some(m), from, to, .. }
            if *m == 2.0 && *from == vec![origin("#M6")] && *to == vec![origin("#M7")])
    });
    assert_eq!(groups(m6_m7), [vec![distance_m6_m7]]);
    let place = |label: &str, value: f64| {
        find("PLACE y", &|f| f.kind == FactKind::Place { at: origin(label), axis: Axis::Y, value })
    };
    let (m1_y, m2_y) = (place("#M1", 0.0), place("#M2", 3.0));
    assert_eq!(groups(mirror), [vec![m1_y, m2_y]]);
    let vip_vin = find("DISTANCE y of VIP and VIN", &|f| {
        matches!(&f.kind, FactKind::Distance { axis: Axis::Y, sense: Sense::Increasing, min: Some(a), max: Some(b), .. }
            if *a == 6.0 && *b == 6.0)
    });
    assert_eq!(groups(inputs), [vec![vip_vin]]);
    // the hints by statement name the statements of lines 37 to 40 of the file, each with the facts
    // discarded because of it, as the table of the development report has them; line 40, which only
    // the low fact loses, only with the hints of the facts of priority low (8.5, 7.6)
    let hints = |r: &Resolved| -> Vec<(String, u32, Vec<usize>, Vec<usize>)> {
        r.statements
            .iter()
            .map(|(s, h)| {
                let file = s.file.rsplit('/').next().unwrap_or_default().to_string();
                (file, s.line, h.facts.clone(), h.discarded.clone())
            })
            .collect()
    };
    let at = |line: u32, fact: usize, discarded: usize| {
        ("T4-miller-conflicts.sp".to_string(), line, vec![fact], vec![discarded])
    };
    assert_eq!(hints(&r), [at(37, m1_y, mirror), at(38, m2_y, mirror), at(39, distance_m6_m7, m6_m7)]);
    assert!(r.statements.keys().all(|s| s.chain.is_empty() && s.point.is_none()), "{:?}", r.statements);
    // by default the low fact has its first group, which is all it has, without knowing it, and a
    // note says why; with its hints asked for, the list is known to be complete, and line 40 is
    // named; nothing else changes
    let low = r.discarded.iter().find(|d| d.fact == inputs).unwrap();
    assert!(!low.complete, "{low:?}");
    let note = format!("[{inputs}] has its first group only, and is in no hint by statement");
    assert!(
        r.notes.iter().any(|n| n.contains(" are listed only with --report ") && n.ends_with(&note)),
        "{:?}",
        r.notes
    );
    let all = check_with("T4", p, &SolverOptions { hints_for_low: true, ..SolverOptions::without_time_limit() });
    assert!(all.discarded.iter().all(|d| d.complete), "{:?}", all.discarded);
    let same = |r: &Resolved| r.discarded.iter().map(|d| (d.fact, d.groups.clone())).collect::<Vec<_>>();
    assert_eq!(same(&all), same(&r));
    assert_eq!(
        hints(&all),
        [at(37, m1_y, mirror), at(38, m2_y, mirror), at(39, distance_m6_m7, m6_m7), at(40, vip_vin, inputs)]
    );
    assert!(all.notes.is_empty(), "{:?}", all.notes);
    assert_eq!((&all.kept, all.position(object(p, "#M2"))), (&r.kept, r.position(object(p, "#M2"))));
    // the rest is kept: the witness has M1 at the origin, M2 at y = 3, mirrored
    let (m1, m2) = (object(p, "#M1"), object(p, "#M2"));
    assert_eq!(r.position(m1), Some((0.0, 0.0)));
    assert_eq!(r.position(m2).map(|(_, y)| y), Some(3.0));
    assert_eq!(r.orientation(m2), r.orientation(m1).map(|o| o.then(Orient::MY)));
}
