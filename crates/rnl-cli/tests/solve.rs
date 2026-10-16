//! `rnl solve` (standard 2.8, 8.5, 13.7, 14.8, 16.1, chapter 17), through the library as the
//! command runs it: each case written for the solver (`tests/integration/solver`) is read as a
//! solver reads it, every drawing level solved and its record of the solver written back, level by
//! level, into a temporary directory, and the file read again as a viewer reads it: no error,
//! nothing unplaced, every constraint `ok` but those that 8.5 discarded, which are `dropped`, the
//! hash the same, and the text the snapshot in `tests/integration/solver/solved`. After a change to
//! the solver that is meant, `RNL_BLESS=1 cargo test -p rnl-cli --test solve` writes the snapshots
//! again.

use std::path::{Path, PathBuf};
use std::process::Command;

use rnl_cli::solve::{describe_fact, destinations, lines, solve, write, SolveOptions, SolveRun, Target};
use rnl_cli::{interpret_file, path_str, FsProvider};
use rnl_runtime::diag::Severity;
use rnl_runtime::geom::{Orient, Pt};
use rnl_runtime::problem::{
    same_input, Axis, FactKind, LevelProblem, OrientSlot, PointRef, Position, Reference, Sense, Slot,
};
use rnl_runtime::solution::{Decision, Solution};
use rnl_runtime::source::{normalize, Layered, MemoryProvider, Provider};
use rnl_runtime::Options;
use rnl_solver::SolverOptions;

/// The cases written for the solver: T1–T6, and T2 after a solve and a drag in an editor.
const CASES: [&str; 7] = [
    "T1-rc-lowpass",
    "T2-common-source",
    "T2b-common-source-dragged",
    "T3-ota5t",
    "T4-miller-conflicts",
    "T5-inverting-amp-rlc",
    "T6-r2r-ladder",
];

fn solver_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/integration/solver")
}

fn case(name: &str) -> PathBuf {
    solver_dir().join(format!("{name}.sp"))
}

fn corpus(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases").join(name)
}

/// A directory of its own for a test, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-solve").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The text `rnl solve` wrote for a case is its snapshot, byte for byte; with `RNL_BLESS` set it
/// becomes the snapshot.
fn compare_or_bless(name: &str, text: &str) {
    let path = solver_dir().join("solved").join(format!("{name}.sp"));
    if std::env::var("RNL_BLESS").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        return;
    }
    let want = std::fs::read(&path).unwrap_or_default();
    assert!(
        want == text.as_bytes(),
        "{} is not what `rnl solve` writes now; run with RNL_BLESS=1 after a change that is meant:\n{text}",
        path.display()
    );
}

/// Where a solution puts the points of its level: each object where the solver placed it, or
/// where the problem fixes it, turned as the solver or the problem says (11.1, 11.2, 12.1, 14.1).
struct Placed<'a> {
    p: &'a LevelProblem,
    s: &'a Solution,
}

impl Placed<'_> {
    fn orient(&self, k: usize) -> Orient {
        let solved = self.s.placements.iter().find(|q| q.object == k).and_then(|q| q.orient);
        let fixed = match &self.p.objects[k].orient {
            Some(OrientSlot::Fixed { code, .. }) => Some(*code),
            _ => None,
        };
        solved.or(fixed).unwrap_or(Orient::R0)
    }

    fn origin(&self, k: usize) -> Pt {
        match &self.p.objects[k].position {
            Position::At(r) => self.at(r),
            Position::InSymbol { owner, local, replaced } => {
                let (x, y) = self.origin(*owner);
                let (dx, dy) = self.orient(*owner).apply(*local);
                // on the axis that the scene gives it, where the solver puts it, or else where its
                // PLACE or its offset puts it (13.3)
                let solved = self.s.placements.iter().find(|q| q.object == k).and_then(|q| q.at);
                let own = |axis: Axis, s: &Slot| match (solved, s) {
                    (Some(at), _) => self.along(at, axis),
                    (None, Slot::Fixed { value, .. }) => *value,
                    (None, Slot::Free) => self.offset(k, axis).unwrap_or(0.0),
                };
                match replaced {
                    Some((Axis::X, s)) => (own(Axis::X, s), y + dy),
                    Some((Axis::Y, s)) => (x + dx, own(Axis::Y, s)),
                    None => (x + dx, y + dy),
                }
            }
            Position::Own { x, y } => {
                let solved = self.s.placements.iter().find(|q| q.object == k).and_then(|q| q.at);
                // without a position from the solver, where the file puts it: by its coordinate
                // facts, or as a text by its offsets (13.3)
                let own = |axis: Axis, s: &Slot| match s {
                    Slot::Fixed { value, .. } => *value,
                    Slot::Free => self.offset(k, axis).unwrap_or(0.0),
                };
                solved.unwrap_or((own(Axis::X, x), own(Axis::Y, y)))
            }
        }
    }

    /// Where the first offset of a text on an axis puts it (13.3).
    fn offset(&self, k: usize, axis: Axis) -> Option<f64> {
        self.p.facts.iter().find_map(|f| match &f.kind {
            FactKind::Offset { text, from, axis: a, value } if *text == k && *a == axis => {
                Some(self.coordinate(from, axis) + value)
            }
            _ => None,
        })
    }

    fn at(&self, r: &PointRef) -> Pt {
        match r {
            PointRef::Origin(k) => self.origin(*k),
            PointRef::Pin { object, pin, .. } => {
                let (x, y) = self.origin(*object);
                let symbol = &self.p.symbols[self.p.objects[*object].symbol.unwrap()];
                let (dx, dy) = self.orient(*object).apply(symbol.pins[*pin].pos);
                (x + dx, y + dy)
            }
            PointRef::Centre(rs) => {
                let points: Vec<Pt> = rs.iter().map(|r| self.at(r)).collect();
                let middle = |c: fn(&Pt) -> f64| {
                    let lo = points.iter().map(c).fold(f64::INFINITY, f64::min);
                    let hi = points.iter().map(c).fold(f64::NEG_INFINITY, f64::max);
                    (lo + hi) / 2.0
                };
                (middle(|q| q.0), middle(|q| q.1))
            }
        }
    }

    fn coordinate(&self, r: &PointRef, axis: Axis) -> f64 {
        self.along(self.at(r), axis)
    }

    fn along(&self, (x, y): Pt, axis: Axis) -> f64 {
        if axis == Axis::X {
            x
        } else {
            y
        }
    }

    /// Does the fact hold, the constants within `tol` (11.7, 14)?
    fn holds(&self, kind: &FactKind, tol: f64) -> bool {
        let c = |r: &PointRef, axis: Axis| self.coordinate(r, axis);
        let extent = |rs: &[PointRef], axis: Axis| -> Option<(f64, f64)> {
            let vs: Vec<f64> = rs.iter().map(|r| c(r, axis)).collect();
            let lo = vs.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = vs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (!vs.is_empty()).then_some((lo, hi))
        };
        match kind {
            FactKind::Place { at, axis, value } => (c(at, *axis) - value).abs() <= tol,
            FactKind::PointCoordinate { point, axis, value } => {
                (c(&PointRef::Origin(*point), *axis) - value).abs() <= tol
            }
            FactKind::Offset { text, from, axis, value } => {
                (c(&PointRef::Origin(*text), *axis) - c(from, *axis) - value).abs() <= tol
            }
            FactKind::Orient { object, code } => self.orient(*object) == *code,
            FactKind::Align { axis, members } => {
                members.windows(2).all(|w| (c(&w[0], *axis) - c(&w[1], *axis)).abs() <= tol)
            }
            FactKind::Distribute { axis, members } => members.windows(3).all(|w| {
                let (a, b, d) = (c(&w[0], *axis), c(&w[1], *axis), c(&w[2], *axis));
                ((b - a) - (d - b)).abs() <= tol
            }),
            FactKind::Order { axis, sense, members } => {
                let up = members.windows(2).all(|w| c(&w[1], *axis) - c(&w[0], *axis) >= 1.0 - tol);
                let down = members.windows(2).all(|w| c(&w[0], *axis) - c(&w[1], *axis) >= 1.0 - tol);
                match sense {
                    Sense::Increasing => up,
                    Sense::Decreasing => down,
                    Sense::Either => up || down,
                }
            }
            FactKind::Distance { axis, sense, min, max, from, to } => {
                let (Some(a), Some(b)) = (extent(from, *axis), extent(to, *axis)) else { return true };
                let within = |d: f64| min.is_none_or(|m| d >= m - tol) && max.is_none_or(|m| d <= m + tol);
                // 14.5: from the extent of `from` to that of `to`, the gap between them
                let (up, down) = (within(b.0 - a.1), within(a.0 - b.1));
                match sense {
                    Sense::Increasing => up,
                    Sense::Decreasing => down,
                    Sense::Either => up || down,
                }
            }
            FactKind::OrientRelation { relation, from, to } => {
                let one = |ks: &[usize]| ks.windows(2).all(|w| self.orient(w[0]) == self.orient(w[1]));
                let turned = match (from.first(), to.first()) {
                    (Some(a), Some(b)) => self.orient(*b) == self.orient(*a).then(*relation),
                    _ => true,
                };
                one(from) && one(to) && turned
            }
        }
    }
}

/// Fact by fact, the solution of each level solved holds the facts that 8.5 kept and not those it
/// discarded, which conflict with them.
fn check_facts(run: &SolveRun, what: &str) {
    for level in &run.levels {
        let p = &level.problem;
        let placed = Placed { p, s: &level.solved.solution };
        let report = &level.solved.report;
        for (k, f) in p.facts.iter().enumerate() {
            let kept = report.kept.contains(&k);
            assert_eq!(placed.holds(&f.kind, 1e-6), kept, "{what} / {}: {}", p.title, describe_fact(p, k));
        }
    }
}

/// Whether a fact reaches one of the objects: by its origin, a pin, or in the centre of a set.
fn reaches(f: &FactKind, objects: &[usize]) -> bool {
    fn point(r: &PointRef, objects: &[usize]) -> bool {
        match r {
            PointRef::Origin(k) | PointRef::Pin { object: k, .. } => objects.contains(k),
            PointRef::Centre(rs) => rs.iter().any(|r| point(r, objects)),
        }
    }
    match f {
        FactKind::Place { at, .. } => point(at, objects),
        FactKind::PointCoordinate { point: k, .. } | FactKind::Orient { object: k, .. } => objects.contains(k),
        FactKind::Offset { text, from, .. } => objects.contains(text) || point(from, objects),
        FactKind::Align { members, .. } | FactKind::Order { members, .. } | FactKind::Distribute { members, .. } => {
            members.iter().any(|m| point(m, objects))
        }
        FactKind::Distance { from, to, .. } => from.iter().chain(to).any(|m| point(m, objects)),
        FactKind::OrientRelation { from, to, .. } => from.iter().chain(to).any(|k| objects.contains(k)),
    }
}

/// Reads the file solved again, as a viewer and as a solver read it, with `files`: no error;
/// every level solved has nothing unplaced, and its constraints are `ok` but for those that 8.5
/// discarded, which are `dropped` (8.5), and those that reach an object whose position the record
/// cannot hold (`Written::unsolved`, such as a point without a name in a file of 0.2), which
/// cannot be checked; read as a solver reads it, each level has the same input as the hash of its
/// record says (17). The cases written for the solver have nothing unsolved.
fn check_read_back(run: &SolveRun, entry: &Path, files: &dyn Provider, draft: bool, what: &str) {
    let r = interpret_file(entry, None, files, &Options { draft, ..Options::default() }).unwrap();
    let errors: Vec<String> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{what}: {errors:?}");
    let again = interpret_file(entry, None, files, &Options { draft, ..Options::solver() }).unwrap();
    for level in &run.levels {
        let p = &level.problem;
        let what = format!("{what} / {}", p.title);
        let lv = r.model.levels.iter().find(|l| l.level == p.level).unwrap();
        assert_eq!(lv.title, p.title);
        assert!(lv.unplaced.is_empty(), "{what}: {:?}", lv.unplaced);
        let unwritable: Vec<usize> = level
            .written
            .unsolved
            .iter()
            .filter_map(|u| match u.decision {
                Decision::Position { object } => Some(object),
                _ => None,
            })
            .collect();
        // the constraints, by kind and priority, with the status each must read back with
        let mut want: Vec<(String, &str, &str)> = p
            .facts
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                !matches!(
                    f.kind,
                    FactKind::Place { .. }
                        | FactKind::PointCoordinate { .. }
                        | FactKind::Offset { .. }
                        | FactKind::Orient { .. }
                )
            })
            .map(|(k, f)| {
                let status = if reaches(&f.kind, &unwritable) {
                    "unsolved"
                } else if level.solved.report.discarded.iter().any(|d| d.fact == k) {
                    "dropped"
                } else {
                    "ok"
                };
                (f.kind.name().to_string(), f.priority.as_str(), status)
            })
            .collect();
        let mut got: Vec<(String, &str, &str)> = lv
            .constraints
            .iter()
            .filter(|c| c.status != "disabled")
            .map(|c| (c.kind.clone(), c.priority.as_str(), c.status.as_str()))
            .collect();
        want.sort();
        got.sort();
        assert_eq!(got, want, "{what}");
        let q = again.problems.iter().find(|q| q.level == p.level).unwrap();
        assert_eq!(same_input(&level.hash, q), Some(true), "{what}");
    }
}

#[test]
fn every_case_solved_and_written_back_reads_back_whole() {
    let fs = FsProvider::new(&[]);
    for name in CASES {
        let f = case(name);
        let run = solve(&f, &fs, &SolveOptions::default()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(run.errors, 0, "{name}");
        assert!(run.not_solved.is_empty(), "{name}: {:?}", run.not_solved);
        // every drawing level of the file, in one file
        let all = interpret_file(&f, None, &fs, &Options::solver()).unwrap().problems;
        let titles: Vec<&str> = run.levels.iter().map(|l| l.problem.title.as_str()).collect();
        assert_eq!(titles, all.iter().map(|p| p.title.as_str()).collect::<Vec<_>>(), "{name}");
        assert_eq!(run.files.len(), 1, "{name}");
        assert_eq!(run.read, [run.files[0].path.clone()], "{name}");
        for level in &run.levels {
            let report = &level.solved.report;
            assert_eq!(report.kept.len() + report.discarded.len(), level.problem.facts.len(), "{name}");
            assert!(report.unrouted.is_empty(), "{name}: {:?}", report.unrouted);
            // the record holds every decision: every object is placed and every constraint read
            // back (plan 5.11)
            assert!(level.written.unsolved.is_empty(), "{name}: {:?}", level.written.unsolved);
            assert!(level.record.starts_with("@RNL(type=\"generated\""), "{name}");
            assert!(level.record.contains(&format!("hash=\"{}\"", level.hash)), "{name}");
            assert!(level.hash.ends_with(&rnl_runtime::problem::options_digest(&SolverOptions::default().canonical())));
            // every level of the file was written back: its record is in the text, the last one
            // finished, the earlier ones as the levels after them found them
            assert!(run.files[0].text.contains(&level.hash), "{name}");
        }
        check_facts(&run, name);
        let dir = scratch(name);
        let writing = write(&run, &Target::Dir(dir.clone()));
        assert_eq!(writing.error, None, "{name}");
        let out = dir.join(format!("{name}.sp"));
        assert_eq!(writing.written, std::slice::from_ref(&out), "{name}");
        assert!(writing.copied.is_empty() && writing.outside.is_empty(), "{name}: {writing:?}");
        let text = std::fs::read_to_string(&out).unwrap();
        assert_eq!(text, run.files[0].text);
        check_read_back(&run, &out, &fs, false, name);
        compare_or_bless(name, &text);
    }
}

#[test]
fn the_discarded_facts_of_t4_are_those_of_its_reference() {
    // 00-brief.md of the brainstorm: {#M6 #M7} @ALIGN("x"), the ALIGN on y of the mirror pair M1
    // M2, and the low {#VIP #VIN} @ALIGN("y")
    let run = solve(&case("T4-miller-conflicts"), &FsProvider::new(&[]), &SolveOptions::default()).unwrap();
    let level = &run.levels[0];
    let lines = rnl_cli::solve::report(level, false);
    let discarded: Vec<&str> = lines.iter().filter(|l| l.starts_with("  discarded ")).map(String::as_str).collect();
    assert_eq!(
        discarded,
        [
            "  discarded [7] ALIGN x {#M6 #M7} (normal, T4-miller-conflicts.sp:28): it conflicts with",
            "  discarded [16] ALIGN y {#M1 #M2} (normal, rnl-std.rnl:227, from $MIRROR_PAIR): it conflicts with",
            "  discarded [40] ALIGN y {#VIP #VIN} (low, T4-miller-conflicts.sp:31): it conflicts with",
        ]
    );
    // each followed by the facts it conflicts with
    let at = lines.iter().position(|l| l.starts_with("  discarded [16]")).unwrap();
    assert_eq!(lines[at + 1], "      [0] the y of #M1 = 0 (high, T4-miller-conflicts.sp:37)");
    assert_eq!(lines[at + 2], "      [4] the y of #M2 = 3 (normal, T4-miller-conflicts.sp:38)");
    assert!(lines[0].starts_with(
        "level T4-miller-conflicts.sp (file in T4-miller-conflicts.sp): 38 kept, 3 discarded of 41 facts"
    ));
    // the middle point of $TAIL_CENTER is named through its usage, %tc::mid: the record holds it
    assert!(!lines.iter().any(|l| l == "  unsolved:"), "{lines:#?}");
    assert!(level.record.contains("%tc::mid @PLACE("), "{}", level.record);
    assert!(lines.iter().any(|l| l.starts_with("  work: ")) && lines.iter().any(|l| l.starts_with("  time: ")));
    // with --report every fact, in the order of 8.5
    let all = rnl_cli::solve::report(level, true);
    assert_eq!(all.iter().filter(|l| l.starts_with("    kept [")).count(), 38);
    assert_eq!(all.iter().filter(|l| l.starts_with("    discarded [")).count(), 3);
    assert!(all.iter().any(|l| l == "    kept [0] the y of #M1 = 0 (high, T4-miller-conflicts.sp:37)"), "{all:#?}");
}

#[test]
fn an_orientation_is_described_once() {
    let run =
        solve(&corpus("10-constraints-and-priority.sp"), &FsProvider::new(&[]), &SolveOptions::default()).unwrap();
    let all = rnl_cli::solve::report(&run.levels[0], true);
    let orient: Vec<&String> = all.iter().filter(|l| l.contains(" the orientation ")).collect();
    assert!(!orient.is_empty(), "{all:#?}");
    for l in orient {
        assert!(!l.contains(" = ORIENT "), "{l}");
    }
    assert!(all.iter().any(|l| l.contains("] the orientation R0 of #R1 (normal, 10-constraints-and-priority.sp:")));
}

#[test]
fn a_solve_is_the_same_twice_and_again_on_its_own_output() {
    let fs = FsProvider::new(&[]);
    let options = SolveOptions::default();
    for name in CASES {
        let f = case(name);
        let a = solve(&f, &fs, &options).unwrap();
        let b = solve(&f, &fs, &options).unwrap();
        assert_eq!(a.files, b.files, "{name}: not the same twice");
        // the solver reads the file without its records of the solver (2.8): solved again, it is
        // the same problem, and the record replaces itself
        let mem = MemoryProvider::new().with(&a.files[0].path, &a.files[0].text);
        let c = solve(&f, &Layered { layers: vec![&mem, &fs] }, &options).unwrap();
        assert_eq!(c.files, a.files, "{name}: not the same on its own output");
    }
}

#[test]
fn in_a_file_of_the_draft_the_record_reaches_every_object() {
    // T3 and T4 declaring the draft 0.3, with their $TAIL_CENTER without a name: its middle point,
    // which a file of 0.2 could not refer to, is placed by its number, with the check of the
    // numbers (5.9), so every constraint holds but those that 8.5 discarded
    let fs = FsProvider::new(&[]);
    for name in ["T3-ota5t", "T4-miller-conflicts"] {
        let f = case(name);
        let text = fs.read(&path_str(&f)).unwrap();
        let text = text.replace("@VERSION(\"0.2\")", "@VERSION(\"0.3\")").replace("%tc = #M1(a)", "#M1(a)");
        let mem = MemoryProvider::new().with(&path_str(&f), &text);
        let both = Layered { layers: vec![&mem, &fs] };
        let run = solve(&f, &both, &SolveOptions { draft: true, ..SolveOptions::default() }).unwrap();
        for level in &run.levels {
            assert!(level.written.unsolved.is_empty(), "{name}: {:?}", level.written.unsolved);
        }
        let with_mid = run.levels.iter().find(|l| l.record.contains("::mid @PLACE(")).unwrap();
        assert!(with_mid.record.contains("numbering=\""), "{}", with_mid.record);
        assert!(!with_mid.record.contains("%tc"), "{}", with_mid.record);
        let solved = MemoryProvider::new().with(&path_str(&f), &run.files[0].text);
        check_read_back(&run, &f, &Layered { layers: vec![&solved, &fs] }, true, name);
    }
}

#[test]
fn a_level_whose_rnl_is_not_interpreted_is_not_solved() {
    let fs = FsProvider::new(&[]);
    // a file of a version the tool does not support: no RNL of it is read, and it is kept as it is
    // (16.1), so its authored PLACE is not a fact and nothing is written into it
    let f = corpus("13-unsupported-version.sp");
    let run = solve(&f, &fs, &SolveOptions::default()).unwrap();
    assert!(run.levels.is_empty() && run.files.is_empty(), "{:?}", run.levels.len());
    assert_eq!(run.not_solved.len(), 1);
    assert_eq!(
        lines(&run, false),
        ["level 13-unsupported-version.sp (file in 13-unsupported-version.sp): not solved, and nothing is written \
             for it: 13-unsupported-version.sp declares the version 9.0, which this reading does not interpret: none \
             of its RNL is read, and it is kept as it is (16.1)"]
    );
    let dir = scratch("unsupported");
    let writing = write(&run, &Target::Dir(dir.join("out")));
    assert_eq!(writing, Default::default());
    assert!(!dir.join("out").exists());

    // a file of the draft 0.3 read without --draft, and with it
    let t1 = case("T1-rc-lowpass");
    let text = fs.read(&path_str(&t1)).unwrap().replace("@VERSION(\"0.2\")", "@VERSION(\"0.3\")");
    let mem = MemoryProvider::new().with(&path_str(&t1), &text);
    let both = Layered { layers: vec![&mem, &fs] };
    let run = solve(&t1, &both, &SolveOptions::default()).unwrap();
    assert!(run.levels.is_empty() && run.files.is_empty());
    assert_eq!(run.not_solved[0].title, "T1-rc-lowpass.sp");
    assert!(
        run.not_solved[0].reason.ends_with("(16.1); --draft reads the draft 0.3 as well"),
        "{}",
        run.not_solved[0].reason
    );
    let run = solve(&t1, &both, &SolveOptions { draft: true, ..SolveOptions::default() }).unwrap();
    assert_eq!(run.levels.len(), 1);
    assert!(run.not_solved.is_empty());

    // a subcircuit in an included file of an unsupported version, its file being read for its
    // netlist only; the main level is solved, and the order of the levels kept
    let files = MemoryProvider::new()
        .with(
            "/w/top.sp",
            "* top\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.include lib.inc\nX1 a 0 div\nR1 a 0 1k\n.end\n",
        )
        .with(
            "/w/lib.inc",
            "* lib\n* @RNL(type=\"header\") { @VERSION(\"9.0\")}\n.subckt div p n\nR1 p m 1k\nR2 m n 1k\n\
             * @RNL { #R1 @PLACE(0, 0) }\n.ends div\n",
        );
    let run = solve(Path::new("/w/top.sp"), &files, &SolveOptions::default()).unwrap();
    assert_eq!(run.levels.iter().map(|l| l.problem.title.as_str()).collect::<Vec<_>>(), ["top.sp"]);
    assert_eq!(run.not_solved.len(), 1);
    let n = &run.not_solved[0];
    assert_eq!((n.title.as_str(), n.kind, n.path.as_str()), ("div", "subckt", "/w/lib.inc"));
    assert!(n.reason.starts_with("lib.inc declares the version 9.0, "), "{}", n.reason);
    assert_eq!(run.files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["/w/top.sp"]);
    let printed = lines(&run, false);
    let at = |prefix: &str| printed.iter().position(|l| l.starts_with(prefix)).unwrap();
    assert!(at("level top.sp (file in top.sp): ") < at("level div (subckt in lib.inc): not solved"), "{printed:#?}");
    // the levels asked for by title alone
    let only = SolveOptions { level: Some("div".into()), ..SolveOptions::default() };
    let run = solve(Path::new("/w/top.sp"), &files, &only).unwrap();
    assert!(run.levels.is_empty() && run.files.is_empty());
    assert_eq!(run.not_solved.len(), 1);
}

#[test]
fn under_a_directory_the_other_files_read_are_copied() {
    let fs = FsProvider::new(&[]);
    // a package found by path= beside the netlist (16.2)
    let f = corpus("11-header-alias-device.sp");
    let run = solve(&f, &fs, &SolveOptions::default()).unwrap();
    assert_eq!(run.errors, 0);
    let names: Vec<&str> = run.read.iter().map(|p| rnl_runtime::host::file_name(p)).collect();
    assert_eq!(names, ["11-header-alias-device.sp", "11-acme-style.rnl"]);
    let dir = scratch("copies");
    let writing = write(&run, &Target::Dir(dir.clone()));
    assert_eq!(writing.error, None);
    assert_eq!(writing.written, [dir.join("11-header-alias-device.sp")]);
    assert_eq!(writing.copied, [dir.join("11-acme-style.rnl")]);
    assert!(writing.outside.is_empty());
    assert_eq!(
        std::fs::read(dir.join("11-acme-style.rnl")).unwrap(),
        std::fs::read(corpus("11-acme-style.rnl")).unwrap()
    );
    let back = dir.join("11-header-alias-device.sp");
    let r = interpret_file(&back, None, &fs, &Options::default()).unwrap();
    let errors: Vec<String> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let again = interpret_file(&back, None, &fs, &Options::solver()).unwrap();
    for level in &run.levels {
        let q = again.problems.iter().find(|q| q.level == level.problem.level).unwrap();
        assert_eq!(same_input(&level.hash, q), Some(true), "{}", level.problem.title);
    }

    // an include of models only, beside the netlist, and one from outside its directory
    let root = scratch("includes");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("shared")).unwrap();
    std::fs::write(
        root.join("src/top.sp"),
        "* top\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.include models.inc\n.include ../shared/params.inc\n\
         M1 d g 0 0 nch\nR1 vdd d 1k\nV1 vdd 0 1.8\nV2 g 0 0.9\n.end\n",
    )
    .unwrap();
    std::fs::write(root.join("src/models.inc"), ".model nch nmos level=1\n").unwrap();
    std::fs::write(root.join("shared/params.inc"), ".param w=1u\n").unwrap();
    let top = root.join("src/top.sp");
    let run = solve(&top, &fs, &SolveOptions::default()).unwrap();
    assert_eq!(run.errors, 0);
    let out = root.join("out");
    let writing = write(&run, &Target::Dir(out.clone()));
    assert_eq!(writing.error, None);
    assert_eq!(writing.written, [out.join("top.sp")]);
    assert_eq!(writing.copied, [out.join("models.inc")]);
    assert_eq!(writing.outside, [normalize(&path_str(&root.join("shared/params.inc")))]);
    let r = interpret_file(&out.join("top.sp"), None, &fs, &Options::default()).unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error));
    // under the directory of the file itself, the files are written over themselves, and nothing is
    // copied onto itself
    let writing = write(&run, &Target::Dir(root.join("src")));
    assert_eq!(writing.error, None);
    assert_eq!(writing.written, [root.join("src").join("top.sp")]);
    assert!(writing.copied.is_empty());
    assert_eq!(std::fs::read_to_string(root.join("src/models.inc")).unwrap(), ".model nch nmos level=1\n");
    // nothing is copied when no level was written back
    let options = SolveOptions { level: Some("nope".into()), ..SolveOptions::default() };
    assert!(solve(&top, &fs, &options).is_err());
}

#[test]
fn a_file_is_written_back_in_its_encoding() {
    let dir = scratch("encoding");
    let t1 = std::fs::read(case("T1-rc-lowpass")).unwrap();
    let first = t1.iter().position(|&b| b == b'\n').unwrap() + 1;
    // Latin-1, which is not UTF-8: µ is one byte
    let latin1 = [&t1[..first], b"* the filter of 1 \xb5F\n", &t1[first..]].concat();
    let f = dir.join("latin1.sp");
    std::fs::write(&f, &latin1).unwrap();
    // UTF-8 with a byte order mark
    let marked = [&[0xEF, 0xBB, 0xBF][..], &t1].concat();
    let g = dir.join("marked.sp");
    std::fs::write(&g, &marked).unwrap();
    let fs = FsProvider::new(&[]);
    for file in [&f, &g] {
        let run = solve(file, &fs, &SolveOptions::default()).unwrap();
        let writing = write(&run, &Target::InPlace);
        assert_eq!(writing.error, None);
        assert_eq!(writing.written.len(), 1);
    }
    let bytes = std::fs::read(&f).unwrap();
    assert!(bytes.windows(4).any(|w| w == b"1 \xb5F"), "{}", String::from_utf8_lossy(&bytes));
    assert!(std::str::from_utf8(&bytes).is_err());
    assert!(String::from_utf8_lossy(&bytes).contains("type=\"generated\""));
    let bytes = std::fs::read(&g).unwrap();
    assert!(bytes.starts_with(&[0xEF, 0xBB, 0xBF]) && !bytes[3..].starts_with(&[0xEF, 0xBB, 0xBF]));
    assert!(String::from_utf8(bytes).unwrap().contains("type=\"generated\""));
}

#[test]
fn one_level_alone() {
    let fs = FsProvider::new(&[]);
    let f = case("T3-ota5t");
    let text = fs.read(&path_str(&f)).unwrap();
    let run = solve(&f, &fs, &SolveOptions { level: Some("ota5t".into()), ..SolveOptions::default() }).unwrap();
    assert_eq!(run.levels.iter().map(|l| l.problem.title.as_str()).collect::<Vec<_>>(), ["ota5t"]);
    // one record, at the end of the subcircuit; the rest of the file as it was
    let out = &run.files[0].text;
    assert_eq!(out.matches("type=\"generated\"").count(), 1);
    let record = out.find("type=\"generated\"").unwrap();
    let ends = out.find(".ends ota5t").unwrap();
    assert!(record < ends);
    assert_eq!(out[ends..], text[text.find(".ends ota5t").unwrap()..]);
    let err = solve(&f, &fs, &SolveOptions { level: Some("nope".into()), ..SolveOptions::default() }).unwrap_err();
    assert_eq!(err, "no drawing level is titled nope");
}

#[test]
fn the_levels_of_an_included_file_are_written_into_it() {
    // the main file, the subcircuit amp in it, and rdiv in the file it includes
    let fs = FsProvider::new(&[]);
    let f = corpus("04-hierarchy.sp");
    let run = solve(&f, &fs, &SolveOptions::default()).unwrap();
    let titles: Vec<&str> = run.levels.iter().map(|l| l.problem.title.as_str()).collect();
    assert_eq!(titles, ["04-hierarchy.sp", "rdiv", "amp"]);
    let names: Vec<&str> = run.files.iter().map(|f| rnl_runtime::host::file_name(&f.path)).collect();
    assert_eq!(names, ["04-hierarchy.sp", "04-hierarchy-lib.inc"]);
    let dir = scratch("04-hierarchy");
    let writing = write(&run, &Target::Dir(dir.clone()));
    assert_eq!(writing.error, None);
    assert_eq!(writing.written, [dir.join("04-hierarchy.sp"), dir.join("04-hierarchy-lib.inc")]);
    assert!(writing.copied.is_empty());
    check_read_back(&run, &dir.join("04-hierarchy.sp"), &fs, false, "04-hierarchy");
    // the level of the included file alone: the main file is copied beside it, so that it reads
    let only = solve(&f, &fs, &SolveOptions { level: Some("rdiv".into()), ..SolveOptions::default() }).unwrap();
    let dir = scratch("04-hierarchy-rdiv");
    let writing = write(&only, &Target::Dir(dir.clone()));
    assert_eq!(writing.written, [dir.join("04-hierarchy-lib.inc")]);
    assert_eq!(writing.copied, [dir.join("04-hierarchy.sp")]);
    check_read_back(&only, &dir.join("04-hierarchy.sp"), &fs, false, "04-hierarchy rdiv");
    // a file out of the directory of the file solved has no name under -o DIR
    let files = MemoryProvider::new()
        .with(
            "/w/a/top.sp",
            "* top\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.include ../lib/div.inc\nX1 a 0 div\nR1 a 0 1k\n.end\n",
        )
        .with("/w/lib/div.inc", ".subckt div p n\nR1 p m 1k\nR2 m n 1k\n.ends div\n");
    let run = solve(Path::new("/w/a/top.sp"), &files, &SolveOptions::default()).unwrap();
    assert_eq!(run.files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["/w/a/top.sp", "/w/lib/div.inc"]);
    let err = destinations(&run, &Target::Dir(PathBuf::from("/out"))).unwrap_err();
    assert!(err.starts_with("/w/lib/div.inc is not in the directory of /w/a/top.sp"), "{err}");
    let writing = write(&run, &Target::Dir(PathBuf::from("/out")));
    assert!(writing.written.is_empty() && writing.error.as_deref() == Some(err.as_str()), "{writing:?}");
    let to = destinations(&run, &Target::InPlace).unwrap();
    assert_eq!(to, [PathBuf::from("/w/a/top.sp"), PathBuf::from("/w/lib/div.inc")]);
}

#[test]
fn a_write_that_fails_says_what_it_wrote_before() {
    let fs = FsProvider::new(&[]);
    let f = corpus("04-hierarchy.sp");
    let run = solve(&f, &fs, &SolveOptions::default()).unwrap();
    // the second file cannot be written: a directory has its name
    let dir = scratch("failing");
    std::fs::create_dir_all(dir.join("04-hierarchy-lib.inc")).unwrap();
    let writing = write(&run, &Target::Dir(dir.clone()));
    assert_eq!(writing.written, [dir.join("04-hierarchy.sp")]);
    let error = writing.error.unwrap();
    assert!(error.starts_with(&format!("cannot write {}", dir.join("04-hierarchy-lib.inc").display())), "{error}");
    // the command says so before it fails
    let dir = scratch("failing-command");
    std::fs::create_dir_all(dir.join("04-hierarchy-lib.inc")).unwrap();
    let (code, stdout, stderr) = rnl(&["solve", &path_str(&f), "-o", &path_str(&dir)]);
    assert_eq!(code, 2, "{stdout}{stderr}");
    assert!(stdout.ends_with(&format!("wrote {}\n", path_str(&dir.join("04-hierarchy.sp")))), "{stdout}");
    assert!(stderr.contains("rnl: cannot write "), "{stderr}");
}

fn rnl(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).output().unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (out.status.code().unwrap_or(-1), text(&out.stdout), text(&out.stderr))
}

#[test]
fn the_command_writes_where_it_is_told_and_reports_each_level() {
    let f = path_str(&case("T1-rc-lowpass"));
    let dir = scratch("command");
    let out = path_str(&dir.join("out"));
    // where to write is not optional, and what does not read is an error
    let (code, _, err) = rnl(&["solve", &f]);
    assert_eq!(code, 2);
    assert!(err.contains("solve needs -o DIR"), "{err}");
    assert_eq!(rnl(&["solve", &f, "-o", &out, "--in-place"]).0, 2);
    let (code, _, err) = rnl(&["solve", &f, "-o", &out, "--solver-options", "speed=3"]);
    assert_eq!(code, 2);
    assert!(err.contains("--solver-options: the solver has no option `speed`"), "{err}");
    assert_eq!(rnl(&["solve", &f, "-o", &out, "--level", "nope"]).0, 2);
    assert_eq!(rnl(&["solve", &path_str(&dir.join("missing.sp")), "-o", &out]).0, 2);
    assert!(!dir.join("out").exists());

    let (code, stdout, _) = rnl(&["solve", &f, "-o", &out, "--report", "--solver-options", "beam=2"]);
    assert_eq!(code, 0, "{stdout}");
    let options = SolverOptions { beam: 2, ..SolverOptions::default() }.canonical();
    assert!(stdout.starts_with(&format!("solver options: {options}\n")), "{stdout}");
    assert!(stdout.contains("\nlevel T1-rc-lowpass.sp (file in T1-rc-lowpass.sp): "), "{stdout}");
    assert!(stdout.contains("\n    kept [0] "), "{stdout}");
    assert!(!stdout.contains("stage S"), "{stdout}");
    let written = dir.join("out").join("T1-rc-lowpass.sp");
    assert!(stdout.ends_with(&format!("wrote {}\n", path_str(&written))), "{stdout}");
    let text = std::fs::read_to_string(&written).unwrap();
    assert!(text.contains(&format!(".{}\"", rnl_runtime::problem::options_digest(&options))), "{text}");

    // in place, a copy of the file becomes the snapshot of the default options
    let copy = dir.join("T1-rc-lowpass.sp");
    std::fs::copy(case("T1-rc-lowpass"), &copy).unwrap();
    let (code, stdout, _) = rnl(&["solve", &path_str(&copy), "--in-place"]);
    assert_eq!(code, 0, "{stdout}");
    let snapshot = std::fs::read(solver_dir().join("solved/T1-rc-lowpass.sp")).unwrap();
    assert!(std::fs::read(&copy).unwrap() == snapshot);

    // a file whose RNL is not interpreted: said, and nothing written
    let unsupported = path_str(&corpus("13-unsupported-version.sp"));
    let (code, stdout, _) = rnl(&["solve", &unsupported, "-o", &path_str(&dir.join("none"))]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("): not solved, and nothing is written for it: "), "{stdout}");
    assert!(!stdout.contains("\nwrote "), "{stdout}");
    assert!(!dir.join("none").exists());

    // the files beside the netlist that it reads are copied
    let alias = path_str(&corpus("11-header-alias-device.sp"));
    let (code, stdout, _) = rnl(&["solve", &alias, "-o", &path_str(&dir.join("alias"))]);
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.ends_with(&format!("copied {}\n", path_str(&dir.join("alias").join("11-acme-style.rnl")))),
        "{stdout}"
    );
}

/// The messages of the diagnostics of a reading of one severity, sorted, without where they are,
/// also where a message names a place: writing a record back moves the lines after it.
fn messages(r: &rnl_runtime::Interpretation, severity: Severity) -> Vec<String> {
    let place = |w: &str| {
        let core = w.trim_end_matches([';', ',', ')', '.']);
        let line = core.rsplit(':').next().is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()));
        let file = [".sp:", ".inc:", ".scs:", ".rnl:"].iter().any(|e| core.contains(e));
        if file && line {
            format!("<place>{}", &w[core.len()..])
        } else {
            w.to_string()
        }
    };
    let mut out: Vec<String> = r
        .diagnostics()
        .into_iter()
        .filter(|d| d.severity == severity)
        .map(|d| d.message.split(' ').map(place).collect::<Vec<_>>().join(" "))
        .collect();
    out.sort();
    out
}

/// The hints of 8.5 among them: a fact dropped in a conflict.
fn dropped(r: &rnl_runtime::Interpretation) -> Vec<String> {
    messages(r, Severity::Hint).into_iter().filter(|m| m.contains("in the order of 8.5")).collect()
}

/// The main files of the conformance corpus.
fn corpus_cases() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(corpus(""))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "sp" || x == "scs"))
        .collect();
    out.sort();
    out
}

#[test]
fn every_corpus_case_solved_reads_back_as_its_authors_wrote_it() {
    // what `rnl solve` writes changes the drawing only where the authors left it to a solver: read
    // back, the case has the errors it has as written, without its records of the solver (2.8),
    // and no fact the authors wrote is dropped that is not dropped there; every level solved is
    // placed but for what its record cannot hold, has the input its hash says, and solved again
    // comes out the same
    let fs = FsProvider::new(&[]);
    let mut solved = 0;
    for f in corpus_cases() {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let run = solve(&f, &fs, &SolveOptions::default()).unwrap_or_else(|e| panic!("{name}: {e}"));
        if run.files.is_empty() {
            assert!(run.levels.is_empty(), "{name}");
            continue;
        }
        solved += 1;
        let dir = scratch(&format!("corpus-{name}"));
        let writing = write(&run, &Target::Dir(dir.clone()));
        assert_eq!(writing.error, None, "{name}");
        let back = dir.join(&name);
        let authored = interpret_file(&f, None, &fs, &Options { generated: false, ..Options::default() }).unwrap();
        let r = interpret_file(&back, None, &fs, &Options::default()).unwrap();
        assert_eq!(messages(&r, Severity::Error), messages(&authored, Severity::Error), "{name}: the errors");
        // a fact dropped once solved is one the solver discarded, in a conflict that a reading
        // without a solver may not see; but a coordinate that such a reading draws as written is
        // not one to discard
        let (before, after) = (dropped(&authored), dropped(&r));
        let discarded: Vec<&rnl_runtime::problem::ProblemFact> = run
            .levels
            .iter()
            .flat_map(|l| l.solved.report.discarded.iter().map(|d| &l.problem.facts[d.fact]))
            .collect();
        let by_solver = |m: &String| discarded.iter().any(|f| m.starts_with(&format!("{} conflicts with", f.label)));
        let new: Vec<&String> = after.iter().filter(|m| !before.contains(m) && !by_solver(m)).collect();
        assert!(new.is_empty(), "{name}: dropped once solved: {new:#?}");
        for f in discarded.iter().filter(|f| {
            matches!(f.kind, FactKind::Place { .. } | FactKind::PointCoordinate { .. } | FactKind::Offset { .. })
        }) {
            let said = before.iter().any(|m| m.starts_with(&format!("{} conflicts with", f.label)));
            assert!(said, "{name}: the solver discarded {}, which holds as written", f.label);
        }
        let again = interpret_file(&back, None, &fs, &Options::solver()).unwrap();
        for level in &run.levels {
            let p = &level.problem;
            let lv = r.model.levels.iter().find(|l| l.level == p.level).unwrap();
            let unsolved: Vec<&str> = level
                .written
                .unsolved
                .iter()
                .filter(|u| matches!(u.decision, Decision::Position { .. }))
                .map(|u| u.label.as_str())
                .collect();
            let left: Vec<&String> = lv.unplaced.iter().filter(|u| !unsolved.contains(&u.as_str())).collect();
            assert!(left.is_empty(), "{name} / {}: {left:?}, the record cannot hold {unsolved:?}", p.title);
            let q = again.problems.iter().find(|q| q.level == p.level).unwrap();
            assert_eq!(same_input(&level.hash, q), Some(true), "{name} / {}", p.title);
        }
        // solved again from what it wrote: the same
        let texts = run.files.iter().fold(MemoryProvider::new(), |m, f| m.with(&f.path, &f.text));
        let twice = solve(&f, &Layered { layers: vec![&texts, &fs] }, &SolveOptions::default()).unwrap();
        assert_eq!(twice.files, run.files, "{name}: not the same solved again");
    }
    assert!(solved >= 18, "{solved}");
}

#[test]
fn a_label_that_the_scene_places_on_one_axis_keeps_its_owner_where_it_is() {
    // corpus 20: #R1::name @PLACE(x=-3) replaces only the x its symbol gives it, and an offset
    // replaces it on its axis (13.3); R1 and R3 stay where their PLACE puts them, and the solver
    // writes no PLACE over an offset, so the texts read back where they were written
    let fs = FsProvider::new(&[]);
    let f = corpus("20-label-positions.sp");
    let run = solve(&f, &fs, &SolveOptions::default()).unwrap();
    let level = &run.levels[0];
    assert!(level.solved.report.discarded.is_empty(), "{:#?}", lines(&run, false));
    check_facts(&run, "20-label-positions");
    let p = &level.problem;
    let at = |label: &str| p.objects.iter().find(|o| o.label == label).unwrap().position.clone();
    assert!(matches!(at("#R1::name"), Position::InSymbol { replaced: Some((Axis::X, Slot::Fixed { .. })), .. }));
    assert!(matches!(at("#R2::name"), Position::InSymbol { replaced: Some((Axis::X, Slot::Free)), .. }));
    assert!(matches!(at("#R5::name"), Position::InSymbol { replaced: None, .. }));
    assert!(matches!(at("#R6::name"), Position::Own { x: Slot::Free, y: Slot::Free }));
    // the record writes every position, those the file fixes too (14.8), but where a text follows
    // its offset or its symbol: the x of #R3::name and #R4::value and not their y, the x of
    // #R1::name and not its y, nothing for #R2::name, #R5::name and the texts of R6
    for line in [
        "#R1 @PLACE(0, 0) => @ORIENT(\"R0\");",
        "#R3 @PLACE(20, 0) => @ORIENT(\"R0\");",
        "#R1::name @PLACE(x=-3);",
        "#R3::name @PLACE(x=25);",
        "#R4::value @PLACE(x=1)",
    ] {
        assert!(level.record.contains(line), "{}", level.record);
    }
    for text in ["#R2::name", "#R5::name", "#R6::name", "#R6::note"] {
        assert!(!level.record.contains(text), "{}", level.record);
    }
    assert!(level.written.unsolved.is_empty(), "{:?}", level.written.unsolved);
    let anchors = |r: &rnl_runtime::Interpretation| {
        let mut out: Vec<(String, Pt)> = r.model.levels[0].texts.iter().map(|t| (t.id.clone(), t.anchor)).collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    };
    let before = interpret_file(&f, None, &fs, &Options::default()).unwrap();
    let mem = MemoryProvider::new().with(&run.files[0].path, &run.files[0].text);
    let after = interpret_file(&f, None, &Layered { layers: vec![&mem, &fs] }, &Options::default()).unwrap();
    assert_eq!(anchors(&after), anchors(&before));
    assert!(after.diagnostics().is_empty(), "{:?}", after.diagnostics());
}

#[test]
fn a_text_whose_offset_the_solver_discards_reads_back_where_the_solver_put_it() {
    // an offset is a fact that 8.5 resolves, and a PLACE written after it replaces it on its axis
    // (13.3): when the solver discards it, the record writes the text's position on that axis, so
    // that the level read back needs no solver; on an axis that its offset or its symbol still
    // places, the record leaves it to them. A text of its own with dx and dy, and a label of the
    // symbol whose dy the scene gives.
    for (text, line) in [
        ("#R1 @TEXT(\"note\", key=\"note\", dx=-1, dy=2)", "#R1::note @PLACE(y=10)"),
        ("#R1 @TEXT(\"${name}\", key=\"name\", dy=2)", "#R1::name @PLACE(y=10)"),
    ] {
        let label = &line[..line.find(' ').unwrap()];
        let body = format!(
            "* offsets\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\nR1 a b 1k\nR2 b c 1k\n* @RNL {{ {text}; \
             #R1 @PLACE(0, 0); #R2 @PLACE(10, 10); {{{label} #R2}} @ALIGN(\"y\", priority=\"high\") }}\n.end\n"
        );
        let files = MemoryProvider::new().with("/w/t.sp", &body);
        let run = solve(Path::new("/w/t.sp"), &files, &SolveOptions::default()).unwrap();
        let level = &run.levels[0];
        let p = &level.problem;
        let discarded: Vec<&FactKind> = level.solved.report.discarded.iter().map(|d| &p.facts[d.fact].kind).collect();
        assert!(
            matches!(discarded.as_slice(), [FactKind::Offset { axis: Axis::Y, value, .. }] if *value == 2.0),
            "{label}: {discarded:?}"
        );
        assert!(level.record.contains(line), "{}", level.record);
        assert!(!level.record.contains(&format!("{label} @PLACE(-1")), "{}", level.record);
        assert!(level.written.unsolved.is_empty(), "{label}: {:?}", level.written.unsolved);
        check_facts(&run, label);
        let solved = MemoryProvider::new().with("/w/t.sp", &run.files[0].text);
        check_read_back(&run, Path::new("/w/t.sp"), &Layered { layers: vec![&solved, &files] }, false, label);
    }
}

#[test]
fn a_statement_written_after_the_record_that_conflicts_with_it_is_dropped() {
    // the record writes every position, those the file fixes too, with priority high (14.8): a
    // statement of normal priority written after it, as between two solves, that conflicts with
    // the solver's results is dropped until the next solve, and not drawn as needing a solver
    let body = "* between\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\nR2 b c 1k\n* @RNL { #R1 @PLACE(0, 0) }\n.end\n";
    let files = MemoryProvider::new().with("/w/t.sp", body);
    let run = solve(Path::new("/w/t.sp"), &files, &SolveOptions::default()).unwrap();
    let record = &run.levels[0].record;
    assert!(record.contains("#R1 @PLACE(0, 0) => @ORIENT("), "{record}");
    assert!(record.contains("#R2 @PLACE(") && !record.contains("#R2 @PLACE(0, "), "{record}");
    let edited = run.files[0].text.replace(".end", "* @RNL { {#R1 #R2} @ALIGN(\"x\")}\n.end");
    let files = MemoryProvider::new().with("/w/t.sp", &edited);
    let r = interpret_file(Path::new("/w/t.sp"), None, &files, &Options::default()).unwrap();
    let statuses: Vec<(&str, &str)> =
        r.model.levels[0].constraints.iter().map(|c| (c.kind.as_str(), c.status.as_str())).collect();
    assert_eq!(statuses, vec![("ALIGN", "dropped")]);
    assert!(dropped(&r).iter().any(|m| m.starts_with("ALIGN conflicts with fixed coordinates")), "{:?}", dropped(&r));
}

#[test]
fn a_file_saved_without_a_header_gets_one() {
    // tools write the header record and the version when they save a file (16.1): a main file
    // after its title, an included file after its first line, which is a comment
    let fs = FsProvider::new(&[]);
    let dir = scratch("header");
    let plain = dir.join("plain.sp");
    std::fs::write(&plain, "* plain\nR1 a 0 1k\nR2 a b 2k\n.end\n").unwrap();
    let hint = "the main file has no header record";
    let first = interpret_file(&plain, None, &fs, &Options::default()).unwrap();
    assert!(messages(&first, Severity::Hint).iter().any(|m| m.starts_with(hint)));
    let run = solve(&plain, &fs, &SolveOptions::default()).unwrap();
    assert!(run.files[0].text.starts_with("* plain\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a 0 1k\n"));
    let out = dir.join("out");
    assert_eq!(write(&run, &Target::Dir(out.clone())).error, None);
    let back = interpret_file(&out.join("plain.sp"), None, &fs, &Options::default()).unwrap();
    assert!(!messages(&back, Severity::Hint).iter().any(|m| m.starts_with(hint)), "{:?}", back.diagnostics());
    check_read_back(&run, &out.join("plain.sp"), &fs, false, "plain");
    let again = solve(&out.join("plain.sp"), &fs, &SolveOptions::default()).unwrap();
    assert_eq!(again.files[0].text, run.files[0].text);

    // the included file of 04-hierarchy, and a subcircuit on the first line of one
    let run = solve(&corpus("04-hierarchy.sp"), &fs, &SolveOptions::default()).unwrap();
    let lib = run.files.iter().find(|f| f.path.ends_with("04-hierarchy-lib.inc")).unwrap();
    let text = lib.text.replace("\r\n", "\n");
    let head = "* library for 04-hierarchy: a resistor divider without a block symbol\n\
                * @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.subckt rdiv";
    assert!(text.starts_with(head), "{text}");
    let files = MemoryProvider::new()
        .with(
            "/w/top.sp",
            "* top\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.include div.inc\nX1 a 0 div\nR1 a 0 1k\n.end\n",
        )
        .with("/w/div.inc", ".subckt div p n\nR1 p m 1k\nR2 m n 1k\n.ends div\n");
    let run = solve(Path::new("/w/top.sp"), &files, &SolveOptions::default()).unwrap();
    let div = run.files.iter().find(|f| f.path == "/w/div.inc").unwrap();
    assert!(div.text.starts_with("*\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.subckt div p n\n"), "{}", div.text);
    let solved = run.files.iter().fold(MemoryProvider::new(), |m, f| m.with(&f.path, &f.text));
    let both = Layered { layers: vec![&solved, &files] };
    check_read_back(&run, Path::new("/w/top.sp"), &both, false, "div.inc");
    let again = solve(Path::new("/w/top.sp"), &both, &SolveOptions::default()).unwrap();
    assert_eq!(again.files, run.files);
}

#[test]
fn a_record_of_the_solver_written_with_scope_is_replaced_by_its_subcircuit() {
    // a record of the solver does not use scope= (2.6), but one written so applies to its
    // subcircuit: solving the subcircuit replaces it, wherever it is written, and solving the
    // level it is written in leaves it to the subcircuit
    let stale =
        "* @RNL(type=\"generated\", scope=\"pair\", priority=\"high\") { #R1 @PLACE(0, 0); #R2 @PLACE(50, 50) }\n";
    let top = format!(
        "* scoped\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\n.subckt pair a b\nR1 a m 1k\nR2 m b 1k\n\
         * @RNL {{ {{#R1 #R2}} @ALIGN(\"y\") }}\n.ends\nX1 p q pair\n{stale}.end\n"
    );
    let files = MemoryProvider::new().with("/w/scoped.sp", &top);
    let entry = Path::new("/w/scoped.sp");
    let pair = SolveOptions { level: Some("pair".into()), ..SolveOptions::default() };
    let run = solve(entry, &files, &pair).unwrap();
    let text = &run.files[0].text;
    assert!(!text.contains("scope=") && text.matches("type=\"generated\"").count() == 1, "{text}");
    let solved = MemoryProvider::new().with("/w/scoped.sp", text);
    let both = Layered { layers: vec![&solved, &files] };
    let r = interpret_file(entry, None, &both, &Options::default()).unwrap();
    assert!(dropped(&r).is_empty(), "{:?}", dropped(&r));
    check_read_back(&run, entry, &both, false, "pair");
    // the file level alone keeps it
    let only = SolveOptions { level: Some("scoped.sp".into()), ..SolveOptions::default() };
    let run = solve(entry, &files, &only).unwrap();
    assert!(run.files[0].text.contains(stale.trim_start_matches("* ").trim_end()), "{}", run.files[0].text);

    // written in the main file for a subcircuit of the file it includes: both files are written
    let files = MemoryProvider::new()
        .with(
            "/w/top.sp",
            &format!(
                "* top\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\n.include pair.inc\nX1 p q pair\n{stale}.end\n"
            ),
        )
        .with(
            "/w/pair.inc",
            "* pair\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.subckt pair a b\nR1 a m 1k\nR2 m b 1k\n.ends\n",
        );
    let run = solve(Path::new("/w/top.sp"), &files, &pair).unwrap();
    let paths: Vec<&str> = run.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["/w/pair.inc", "/w/top.sp"]);
    assert!(!run.files[1].text.contains("generated"), "{}", run.files[1].text);
    let solved = run.files.iter().fold(MemoryProvider::new(), |m, f| m.with(&f.path, &f.text));
    check_read_back(&run, Path::new("/w/top.sp"), &Layered { layers: vec![&solved, &files] }, false, "pair.inc");
}

#[test]
fn a_free_text_goes_beside_what_it_is_attached_to() {
    // a text that no fact places goes at the point it is attached to, or beside the object, as a
    // viewer that does not solve puts it (13.3); the report names a text by its name
    let files = MemoryProvider::new().with(
        "/w/t.sp",
        "* texts\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 p 0 1k\nC1 p 0 1p\n* @RNL {\n\
         *   %t = #R1:p @TEXT(\"hi\");\n*   %u = #C1 @TEXT(\"cap\");\n*   %q = (_, _);\n*   %v = %q @TEXT(\"at q\");\n\
         *   {%t #C1} @ALIGN(\"x\")\n* }\n.end\n",
    );
    let run = solve(Path::new("/w/t.sp"), &files, &SolveOptions::default()).unwrap();
    let level = &run.levels[0];
    let report = rnl_cli::solve::report(level, true);
    assert!(report.iter().any(|l| l.starts_with("    kept [0] ALIGN x {%t #C1} (normal, t.sp:10)")), "{report:#?}");
    let p = &level.problem;
    let placed = Placed { p, s: &level.solved.solution };
    let named = |n: &str| p.objects.iter().position(|o| o.reference == Reference::Name(n.into())).unwrap();
    let at = |n: &str| placed.origin(named(n));
    let pin = placed.at(&PointRef::Pin { object: 0, pin: 0, terminal: "p".into() });
    let near = |a: Pt, b: Pt| (a.0 - b.0).abs() <= 1.5 && (a.1 - b.1).abs() <= 1.5;
    // %t: on C1's x by the ALIGN, its y beside the pin
    let t = at("%t");
    assert_eq!(t.0, at("#C1").0);
    assert!((t.1 - pin.1).abs() <= 1.5, "{t:?} {pin:?}");
    assert!(near(at("%u"), at("#C1")), "{:?} {:?}", at("%u"), at("#C1"));
    assert_eq!(at("%v"), at("%q"));
    let solved = MemoryProvider::new().with("/w/t.sp", &run.files[0].text);
    check_read_back(&run, Path::new("/w/t.sp"), &Layered { layers: vec![&solved, &files] }, false, "texts");
}

#[test]
fn the_problem_command_hashes_the_options_as_solve_writes_them() {
    let f = path_str(&case("T1-rc-lowpass"));
    let hash = |args: &[&str]| {
        let (code, out, err) = rnl(args);
        assert_eq!(code, 0, "{err}");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        v["levels"][0]["hash"].as_str().unwrap().to_string()
    };
    let snapshot = std::fs::read_to_string(solver_dir().join("solved/T1-rc-lowpass.sp")).unwrap();
    assert!(snapshot.contains(&format!("hash=\"{}\"", hash(&["problem", &f]))));
    let beam = SolverOptions { beam: 2, ..SolverOptions::default() }.canonical();
    let with = hash(&["problem", &f, "--solver-options", "beam=2"]);
    assert!(with.ends_with(&rnl_runtime::problem::options_digest(&beam)), "{with}");
    let (code, _, err) = rnl(&["problem", &f, "--solver-options", "speed=3"]);
    assert_eq!(code, 2);
    assert!(err.contains("--solver-options: the solver has no option `speed`"), "{err}");
}

/// The texts solved, as `rnl solve` wrote them back, read again as a viewer reads them, with the
/// files the document is read from besides.
fn read_solved(run: &SolveRun, files: &dyn Provider, entry: &Path) -> rnl_runtime::Interpretation {
    let solved = run.files.iter().fold(MemoryProvider::new(), |m, f| m.with(&f.path, &f.text));
    interpret_file(entry, None, &Layered { layers: vec![&solved, files] }, &Options::default()).unwrap()
}

#[test]
fn a_header_after_a_subcircuit_solved_gets_its_version_where_it_is() {
    // a header record without VERSION after the subcircuit that is solved, in an included file and
    // in a main file solved a level at a time: the record of the subcircuit goes in before its
    // `.ends`, above the header, and VERSION into the header, not into that record (16.1, 17)
    let files = MemoryProvider::new()
        .with(
            "/w/top4.sp",
            "* top\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.include lib4.inc\nX1 p q pair\n.end\n",
        )
        .with("/w/lib4.inc", "* lib\n.subckt pair a b\nR1 a m 1k\nR2 m b 1k\n.ends\n* @RNL(type=\"header\") { }\n");
    let entry = Path::new("/w/top4.sp");
    let run = solve(entry, &files, &SolveOptions::default()).unwrap();
    let lib = &run.files.iter().find(|f| f.path == "/w/lib4.inc").unwrap().text;
    assert!(lib.ends_with("* }\n.ends\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n"), "{lib}");
    assert_eq!(lib.matches("@VERSION").count(), 1, "{lib}");
    let solved = run.files.iter().fold(MemoryProvider::new(), |m, f| m.with(&f.path, &f.text));
    let both = Layered { layers: vec![&solved, &files] };
    check_read_back(&run, entry, &both, false, "lib4.inc");
    assert_eq!(solve(entry, &both, &SolveOptions::default()).unwrap().files, run.files);

    let top = "* top\n.subckt pair a b\nR1 a m 1k\nR2 m b 1k\n.ends\n* @RNL(type=\"header\") { }\nX1 p q pair\n.end\n";
    let files = MemoryProvider::new().with("/w/top.sp", top);
    let entry = Path::new("/w/top.sp");
    for level in ["pair", "top.sp"] {
        let one = SolveOptions { level: Some(level.into()), ..SolveOptions::default() };
        let run = solve(entry, &files, &one).unwrap();
        let text = &run.files[0].text;
        let header = "\n.ends\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\nX1 p q pair\n";
        assert!(text.contains(header) && text.matches("@VERSION").count() == 1, "{level}: {text}");
        let solved = MemoryProvider::new().with("/w/top.sp", text);
        let both = Layered { layers: vec![&solved, &files] };
        check_read_back(&run, entry, &both, false, level);
        // the other level solved after it finds the header whole
        let all = solve(entry, &both, &SolveOptions::default()).unwrap();
        assert_eq!(all.files[0].text.matches("@VERSION").count(), 1, "{level}: {}", all.files[0].text);
        let solved = MemoryProvider::new().with("/w/top.sp", &all.files[0].text);
        check_read_back(&all, entry, &Layered { layers: vec![&solved, &files] }, false, level);
    }
}

#[test]
fn an_included_file_that_switches_to_spectre_gets_its_header_after_the_switch() {
    // a file that is not a .scs file is read in SPICE mode until its `simulator lang=spectre`,
    // where `//` is no comment (2.2): the header in `//` goes after that statement
    let files = MemoryProvider::new()
        .with(
            "/w/top.scs",
            "// top\n// @RNL(type=\"header\") { @VERSION(\"0.2\")}\nsimulator lang=spectre\ninclude \"div.inc\"\n\
             X1 (p 0) div\n",
        )
        .with(
            "/w/div.inc",
            "simulator lang=spectre\nsubckt div (p n)\nR1 (p m) resistor r=1k\nR2 (m n) resistor r=1k\nends div\n",
        );
    let entry = Path::new("/w/top.scs");
    let run = solve(entry, &files, &SolveOptions::default()).unwrap();
    assert_eq!(run.levels.iter().map(|l| l.problem.title.as_str()).collect::<Vec<_>>(), ["top.scs", "div"]);
    let div = &run.files.iter().find(|f| f.path == "/w/div.inc").unwrap().text;
    let head = "simulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.2\")}\nsubckt div (p n)\n";
    assert!(div.starts_with(head), "{div}");
    assert!(div.contains("\n// @RNL(type=\"generated\""), "{div}");
    let solved = run.files.iter().fold(MemoryProvider::new(), |m, f| m.with(&f.path, &f.text));
    let both = Layered { layers: vec![&solved, &files] };
    check_read_back(&run, entry, &both, false, "div.inc");
    assert_eq!(solve(entry, &both, &SolveOptions::default()).unwrap().files, run.files);
}

#[test]
fn a_text_whose_offset_or_place_the_solver_discards_is_written_where_it_put_it() {
    // high ALIGNs against the normal offsets of a label and of a text of its own (o7), and against
    // the one-axis PLACE of a label (p8): the record writes the position the solver chose on that
    // axis, which replaces the offset there (13.3), and the level reads back with every fact kept
    // ok and no fact dropped but the PLACE that the solver discarded (8.5). Each fact discarded is
    // described once.
    let cases = [
        (
            "o7",
            "#R1 @PLACE(0, 0, priority=\"high\"); #C1 @PLACE(20, 10, priority=\"high\"); \
             #R1 @TEXT(\"${name}\", key=\"name\", dx=3); %t = #C1 @TEXT(\"note\", dx=2, dy=2); \
             {#R1::name #C1} @ALIGN(\"x\", priority=\"high\"); {%t #R1} @ALIGN(\"y\", priority=\"high\")",
            vec!["the dy of %t from #C1 = 2", "the dx of #R1::name from #R1 = 3"],
            vec!["#R1::name @PLACE(x=20)", "%t @PLACE(y=0)"],
            vec![],
        ),
        (
            "p8",
            "#R1 @PLACE(0, 0); #C1 @PLACE(40, 10, priority=\"high\"); #R1::name @PLACE(x=-30); \
             {#R1::name #C1} @ALIGN(\"x\", priority=\"high\")",
            vec!["the x of #R1::name = -30"],
            vec!["#R1::name @PLACE(x=40)"],
            vec!["the x of #R1::name = -30 conflicts with the x of #R1::name = 40 and is dropped in the order of 8.5 (8.5)"],
        ),
    ];
    for (name, body, discarded, lines, dropped_back) in cases {
        let path = format!("/w/{name}.sp");
        let text = format!(
            "* {name}\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\nR1 a b 1k\nC1 b 0 1p\n* @RNL {{ {body} }}\n.end\n"
        );
        let files = MemoryProvider::new().with(&path, &text);
        let entry = Path::new(&path);
        let run = solve(entry, &files, &SolveOptions::default()).unwrap();
        let level = &run.levels[0];
        let p = &level.problem;
        let described: Vec<String> = level.solved.report.discarded.iter().map(|d| describe_fact(p, d.fact)).collect();
        let want: Vec<String> = discarded
            .iter()
            .zip(&level.solved.report.discarded)
            .map(|(w, d)| format!("[{}] {w} (normal, {name}.sp:5)", d.fact))
            .collect();
        assert_eq!(described, want, "{name}");
        for line in &lines {
            assert!(level.record.contains(line), "{name}: {line}\n{}", level.record);
        }
        assert!(level.written.unsolved.is_empty(), "{name}: {:?}", level.written.unsolved);
        check_facts(&run, name);
        let solved = MemoryProvider::new().with(&path, &run.files[0].text);
        check_read_back(&run, entry, &Layered { layers: vec![&solved, &files] }, false, name);
        let r = read_solved(&run, &files, entry);
        assert_eq!(dropped(&r), dropped_back, "{name}");
        // the texts written read back where the solver put them
        let placed = Placed { p, s: &level.solved.solution };
        for line in lines {
            let id = &line[..line.find(' ').unwrap()];
            let k = p.objects.iter().position(|o| o.reference == Reference::Name(id.into())).unwrap();
            let texts = &r.model.levels[0].texts;
            let t = texts.iter().find(|t| t.object_id == id).unwrap_or_else(|| panic!("{name}: {id} in {texts:?}"));
            assert_eq!(t.anchor, placed.origin(k), "{name}: {id}");
        }
    }
}

#[test]
fn an_orientation_without_a_name_is_unsolved_only_where_the_file_does_not_fix_it() {
    // two grounds without a name in a file of 0.2, which the record cannot refer to (5.9): the one
    // whose ORIENT the solver keeps is drawn so all the same, and its orientation is not unsolved;
    // the other's the solver chooses, and it is (14.8)
    let files = MemoryProvider::new().with(
        "/w/an.sp",
        "* anonymous\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a 0 1k\nR2 b 0 1k\n\
         * @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(10, 0); $GND @ATTACH #R1:n => @ORIENT(\"R90\"); $GND @ATTACH #R2:n }\n\
         .end\n",
    );
    let entry = Path::new("/w/an.sp");
    let run = solve(entry, &files, &SolveOptions::default()).unwrap();
    let level = &run.levels[0];
    let p = &level.problem;
    let grounds: Vec<usize> = (0..p.objects.len()).filter(|&k| p.objects[k].label == "a usage of $GND").collect();
    let fixed: Vec<usize> = grounds
        .iter()
        .copied()
        .filter(|&k| matches!(p.objects[k].orient, Some(OrientSlot::Fixed { code: Orient::R90, .. })))
        .collect();
    let free: Vec<usize> =
        grounds.iter().copied().filter(|&k| matches!(p.objects[k].orient, Some(OrientSlot::Free))).collect();
    assert_eq!((grounds.len(), fixed.len(), free.len()), (2, 1, 1), "{:?}", p.objects);
    let turned = |k: usize| level.solved.solution.placements.iter().find(|pl| pl.object == k).and_then(|pl| pl.orient);
    assert_eq!(turned(fixed[0]), Some(Orient::R90));
    assert!(turned(free[0]).is_some());
    let unsolved: Vec<Decision> = level.written.unsolved.iter().map(|u| u.decision).collect();
    assert_eq!(unsolved, [Decision::Orientation { object: free[0] }], "{:?}", level.written.unsolved);
    let solved = MemoryProvider::new().with("/w/an.sp", &run.files[0].text);
    check_read_back(&run, entry, &Layered { layers: vec![&solved, &files] }, false, "an");
}

#[test]
fn within_its_tolerance_the_record_writes_the_constants_of_the_facts_that_rank_first() {
    // with tolerance=0.5 the high facts hold together: #R1 at x 0 and #R2 at x 0.4, on one x. A
    // viewer checks with its own tolerance (11.7); the record keeps the constant of the fact that
    // ranks first, #R2's 0.4, and a viewer drops only the last, #R1's 0 (8.5, 17)
    let files = MemoryProvider::new().with(
        "/w/tol.sp",
        "* tol\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\nR2 b c 1k\n\
         * @RNL { #R1 @PLACE(0, 0, priority=\"high\"); #R2 @PLACE(x=0.4, priority=\"high\"); \
         {#R1 #R2} @ALIGN(\"x\", priority=\"high\") }\n.end\n",
    );
    let entry = Path::new("/w/tol.sp");
    let solver = SolverOptions { tolerance: 0.5, ..SolverOptions::default() };
    let run = solve(entry, &files, &SolveOptions { solver, ..SolveOptions::default() }).unwrap();
    let level = &run.levels[0];
    let p = &level.problem;
    assert!(level.solved.report.discarded.is_empty(), "{:#?}", lines(&run, true));
    let placed = Placed { p, s: &level.solved.solution };
    for f in &p.facts {
        assert!(placed.holds(&f.kind, 0.5), "{}", f.label);
    }
    for line in ["#R1 @PLACE(0.4, 0)", "#R2 @PLACE(0.4, "] {
        assert!(level.record.contains(line), "{line}\n{}", level.record);
    }
    let r = read_solved(&run, &files, entry);
    assert_eq!(
        dropped(&r),
        ["the x of #R1 = 0 conflicts with the x of #R1 = 0.4 and is dropped in the order of 8.5 (8.5)"]
    );
    // with the default tolerance the first conflict is the solver's to resolve: nothing changes there
    let run = solve(entry, &files, &SolveOptions::default()).unwrap();
    let discarded: Vec<&str> = run.levels[0]
        .solved
        .report
        .discarded
        .iter()
        .map(|d| run.levels[0].problem.facts[d.fact].label.as_str())
        .collect();
    assert_eq!(discarded, ["the x of #R1 = 0"]);
}
