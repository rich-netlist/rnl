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
use rnl_runtime::problem::{same_input, Axis, FactKind, LevelProblem, OrientSlot, PointRef, Position, Sense, Slot};
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
        let slot = |s: &Slot| match s {
            Slot::Fixed { value, .. } => *value,
            Slot::Free => 0.0,
        };
        match &self.p.objects[k].position {
            Position::At(r) => self.at(r),
            Position::InSymbol { owner, local } => {
                let (x, y) = self.origin(*owner);
                let (dx, dy) = self.orient(*owner).apply(*local);
                (x + dx, y + dy)
            }
            Position::Own { x, y } => {
                let solved = self.s.placements.iter().find(|q| q.object == k).and_then(|q| q.at);
                solved.unwrap_or((slot(x), slot(y)))
            }
        }
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
        let (x, y) = self.at(r);
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
