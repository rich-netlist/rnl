//! `rnl solve` (standard 2.8, 8.5, 13.7, 14.8, 16.1, chapter 17), through the library as the
//! command runs it: each case written for the solver (`tests/integration/solver`) is read as a
//! solver reads it, every drawing level solved and its record of the solver written back, level by
//! level, into a temporary directory, and the file read again as a viewer reads it: no error,
//! nothing unplaced, every constraint `ok` but those that 8.5 discarded, which are `dropped`, the
//! hash the same, and the text the snapshot in `tests/integration/solver/solved`. After a change to
//! the solver that is meant, `RNL_BLESS=1 cargo test -p rnl-cli --test solve` writes the snapshots
//! again. Every solve here has no time limit, so that it is the resolution of 8.5 exactly and the
//! same on every run (the draft 0.3, 8.5), but where a test is about the limit: those measure it on
//! a clock that counts its readings ([`Ticks`]), or check the default of the command.

use std::path::{Path, PathBuf};
use std::process::Command;

use rnl_cli::solve::{
    describe_fact, destinations, lines, solve, solve_with_clock, write, SolveOptions, SolveRun, Target,
};
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

/// The options of `rnl solve` without a time limit: the resolution of 8.5 exactly, the same on every
/// run, as the snapshots are written (the draft 0.3, 8.5).
fn unlimited() -> SolveOptions {
    SolveOptions { solver: SolverOptions::without_time_limit(), ..SolveOptions::default() }
}

/// A clock that advances one second each time it is read: a time limit is out at the same point
/// on every run (the draft 0.3, 8.5).
#[derive(Default)]
struct Ticks(std::cell::Cell<u64>);

impl Ticks {
    /// How many times it was read.
    fn reads(&self) -> u64 {
        self.0.get()
    }
}

impl rnl_solver::Clock for Ticks {
    fn elapsed(&self) -> f64 {
        let t = self.0.get();
        self.0.set(t + 1);
        t as f64
    }
}

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
        let run = solve(&f, &fs, &unlimited()).unwrap_or_else(|e| panic!("{name}: {e}"));
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
            // no fact contradicts itself (the draft 0.3, 8.5)
            assert!(report.failed.is_empty() && report.errors.is_empty(), "{name}: {:?}", report.errors);
            assert!(report.unrouted.is_empty(), "{name}: {:?}", report.unrouted);
            // the record holds every decision: every object is placed and every constraint read
            // back (plan 5.11)
            assert!(level.written.unsolved.is_empty(), "{name}: {:?}", level.written.unsolved);
            assert!(level.record.starts_with("@RNL(type=\"generated\""), "{name}");
            assert!(level.record.contains(&format!("hash=\"{}\"", level.hash)), "{name}");
            let options = SolverOptions::without_time_limit().canonical();
            assert!(level.hash.ends_with(&rnl_runtime::problem::options_digest(&options)));
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
    let run = solve(&case("T4-miller-conflicts"), &FsProvider::new(&[]), &unlimited()).unwrap();
    let level = &run.levels[0];
    let lines = rnl_cli::solve::report(level, false);
    let discarded: Vec<&str> = lines.iter().filter(|l| l.starts_with("  discarded ")).map(String::as_str).collect();
    // each with its one minimal group (the draft 0.3, 8.5); the low one without its hint (7.6)
    assert_eq!(
        discarded,
        [
            "  discarded [7] ALIGN x {#M6 #M7} (normal, T4-miller-conflicts.sp:28): it conflicts with [3]",
            "  discarded [16] ALIGN y {#M1 #M2} (normal, rnl-std-layout.rnl:10, from $MIRROR_PAIR): it conflicts \
             with [0] and [4]",
            "  discarded [40] ALIGN y {#VIP #VIN} (low, T4-miller-conflicts.sp:31): of priority low, which is \
             expected; --report lists what it conflicts with (8.5)",
        ]
    );
    // each followed by the facts it conflicts with
    let at = lines.iter().position(|l| l.starts_with("  discarded [16]")).unwrap();
    assert_eq!(lines[at + 1], "      [0] the y of #M1 = 0 (high, T4-miller-conflicts.sp:37)");
    assert_eq!(lines[at + 2], "      [4] the y of #M2 = 3 (normal, T4-miller-conflicts.sp:38)");
    let at = lines.iter().position(|l| l.starts_with("  discarded [40]")).unwrap();
    assert!(!lines[at + 1].starts_with("      ["), "{lines:#?}");
    // then the statements of those groups, each with the facts discarded because of it; the one
    // that only the low fact loses only with --report
    let statements = [
        "    T4-miller-conflicts.sp:37 [0] the y of #M1 = 0 took part in discarding [16]",
        "    T4-miller-conflicts.sp:38 [4] the y of #M2 = 3 took part in discarding [16]",
        "    T4-miller-conflicts.sp:39 [3] DISTANCE x either min=2 {#M6} {#M7} took part in discarding [7]",
        "    T4-miller-conflicts.sp:40 [2] DISTANCE y increasing min=6 max=6 {#VIP} {#VIN} took part in discarding \
         [40]",
    ];
    let head = lines.iter().position(|l| l == "  statements whose facts took part in discarding (8.5):").unwrap();
    assert_eq!(lines[head + 1..head + 4], statements[..3], "{lines:#?}");
    assert!(!lines[head + 4].starts_with("    "), "{lines:#?}");
    assert!(lines[0].starts_with(
        "level T4-miller-conflicts.sp (file in T4-miller-conflicts.sp): 38 kept, 3 discarded of 41 facts"
    ));
    // the middle point of $TAIL_CENTER is named through its usage, %tc::mid: the record holds it
    assert!(!lines.iter().any(|l| l == "  unsolved:"), "{lines:#?}");
    assert!(level.record.contains("%tc::mid @PLACE("), "{}", level.record);
    assert!(lines.iter().any(|l| l.starts_with("  work: ")) && lines.iter().any(|l| l.starts_with("  time: ")));
    // the solver gave the low fact its first group only, which a note says (8.5, 7.6), so that even
    // the report of every fact leaves out its hints
    let low = level.solved.report.discarded.iter().find(|d| d.fact == 40).unwrap();
    assert_eq!((&low.groups, low.complete), (&vec![vec![2]], false));
    let note = "    the minimal conflict groups of a fact of priority low discarded are listed only with --report \
                (hints_for_low), as its hints may be left out (8.5, 7.6): [40] has its first group only, and is in \
                no hint by statement";
    assert!(lines.iter().any(|l| l == note), "{lines:#?}");
    let every = rnl_cli::solve::report(level, true);
    assert!(every.contains(&format!("  {}", discarded[2])), "{every:#?}");
    // with --report the solver seeks them: every fact, in the order of 8.5, the same solution, and
    // no note
    let options = SolveOptions {
        solver: SolverOptions { hints_for_low: true, ..SolverOptions::without_time_limit() },
        ..unlimited()
    };
    let reported = solve(&case("T4-miller-conflicts"), &FsProvider::new(&[]), &options).unwrap();
    assert_eq!(reported.files, run.files);
    let level = &reported.levels[0];
    assert!(level.solved.report.discarded.iter().all(|d| d.complete), "{:?}", level.solved.report.discarded);
    let all = rnl_cli::solve::report(level, true);
    assert!(!all.iter().any(|l| l == note), "{all:#?}");
    assert_eq!(all.iter().filter(|l| l.starts_with("    kept [")).count(), 38);
    assert_eq!(all.iter().filter(|l| l.starts_with("    discarded [")).count(), 3);
    assert!(all.iter().any(|l| l == "    kept [0] the y of #M1 = 0 (high, T4-miller-conflicts.sp:37)"), "{all:#?}");
    // and the hints of the low fact, and of the statement that only it loses
    let at = all.iter().position(|l| l.starts_with("    discarded [40]")).unwrap();
    assert_eq!(
        all[at],
        "    discarded [40] ALIGN y {#VIP #VIN} (low, T4-miller-conflicts.sp:31): it conflicts with [2]"
    );
    assert_eq!(
        all[at + 1],
        "        [2] DISTANCE y increasing min=6 max=6 {#VIP} {#VIN} (normal, T4-miller-conflicts.sp:40)"
    );
    let head = all.iter().position(|l| l == "  statements whose facts took part in discarding (8.5):").unwrap();
    assert_eq!(all[head + 1..head + 5], statements, "{all:#?}");
    // the command asks for them with --report
    let t4 = path_str(&case("T4-miller-conflicts"));
    let out = path_str(&scratch("t4-report"));
    let (code, stdout, _) = rnl(&["solve", &t4, "-o", &out, "--report", "--solver-options", "time_limit=none"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains(&format!("\n{}\n{}\n", all[at], all[at + 1])), "{stdout}");
    assert!(!stdout.contains(" are listed only with --report "), "{stdout}");
}

#[test]
fn an_orientation_is_described_once() {
    let run = solve(&corpus("10-constraints-and-priority.sp"), &FsProvider::new(&[]), &unlimited()).unwrap();
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
    let options = unlimited();
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
        let run = solve(&f, &both, &SolveOptions { draft: true, ..unlimited() }).unwrap();
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
    let run = solve(&f, &fs, &unlimited()).unwrap();
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
    let run = solve(&t1, &both, &unlimited()).unwrap();
    assert!(run.levels.is_empty() && run.files.is_empty());
    assert_eq!(run.not_solved[0].title, "T1-rc-lowpass.sp");
    assert!(
        run.not_solved[0].reason.ends_with("(16.1); --draft reads the draft 0.3 as well"),
        "{}",
        run.not_solved[0].reason
    );
    let run = solve(&t1, &both, &SolveOptions { draft: true, ..unlimited() }).unwrap();
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
    let run = solve(Path::new("/w/top.sp"), &files, &unlimited()).unwrap();
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
    let only = SolveOptions { level: Some("div".into()), ..unlimited() };
    let run = solve(Path::new("/w/top.sp"), &files, &only).unwrap();
    assert!(run.levels.is_empty() && run.files.is_empty());
    assert_eq!(run.not_solved.len(), 1);
}

#[test]
fn under_a_directory_the_other_files_read_are_copied() {
    let fs = FsProvider::new(&[]);
    // a package found by path= beside the netlist (16.2)
    let f = corpus("11-header-alias-device.sp");
    let run = solve(&f, &fs, &unlimited()).unwrap();
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
    let run = solve(&top, &fs, &unlimited()).unwrap();
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
    let options = SolveOptions { level: Some("nope".into()), ..unlimited() };
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
        let run = solve(file, &fs, &unlimited()).unwrap();
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
    let run = solve(&f, &fs, &SolveOptions { level: Some("ota5t".into()), ..unlimited() }).unwrap();
    assert_eq!(run.levels.iter().map(|l| l.problem.title.as_str()).collect::<Vec<_>>(), ["ota5t"]);
    // one record, at the end of the subcircuit; the rest of the file as it was
    let out = &run.files[0].text;
    assert_eq!(out.matches("type=\"generated\"").count(), 1);
    let record = out.find("type=\"generated\"").unwrap();
    let ends = out.find(".ends ota5t").unwrap();
    assert!(record < ends);
    assert_eq!(out[ends..], text[text.find(".ends ota5t").unwrap()..]);
    let err = solve(&f, &fs, &SolveOptions { level: Some("nope".into()), ..unlimited() }).unwrap_err();
    assert_eq!(err, "no drawing level is titled nope");
}

#[test]
fn the_levels_of_an_included_file_are_written_into_it() {
    // the main file, the subcircuit amp in it, and rdiv in the file it includes
    let fs = FsProvider::new(&[]);
    let f = corpus("04-hierarchy.sp");
    let run = solve(&f, &fs, &unlimited()).unwrap();
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
    let only = solve(&f, &fs, &SolveOptions { level: Some("rdiv".into()), ..unlimited() }).unwrap();
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
    let run = solve(Path::new("/w/a/top.sp"), &files, &unlimited()).unwrap();
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
    let run = solve(&f, &fs, &unlimited()).unwrap();
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
    let (code, stdout, stderr) =
        rnl(&["solve", &path_str(&f), "-o", &path_str(&dir), "--solver-options", "time_limit=none"]);
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
    let (code, _, err) = rnl(&["solve", &f, "--solver-options", "time_limit=none"]);
    assert_eq!(code, 2);
    assert!(err.contains("solve needs -o DIR"), "{err}");
    assert_eq!(rnl(&["solve", &f, "-o", &out, "--in-place", "--solver-options", "time_limit=none"]).0, 2);
    let (code, _, err) = rnl(&["solve", &f, "-o", &out, "--solver-options", "speed=3"]);
    assert_eq!(code, 2);
    assert!(err.contains("--solver-options: the solver has no option `speed`"), "{err}");
    assert_eq!(rnl(&["solve", &f, "-o", &out, "--level", "nope", "--solver-options", "time_limit=none"]).0, 2);
    assert_eq!(
        rnl(&["solve", &path_str(&dir.join("missing.sp")), "-o", &out, "--solver-options", "time_limit=none"]).0,
        2
    );
    assert!(!dir.join("out").exists());

    // the command's time limit is the toolkit's 5 seconds unless the options say otherwise
    let (code, stdout, _) = rnl(&["solve", &f, "-o", &out, "--report", "--solver-options", "beam=2"]);
    assert_eq!(code, 0, "{stdout}");
    let options = SolverOptions { beam: 2, ..SolverOptions::default() }.canonical();
    assert!(options.ends_with(";time_limit=5"), "{options}");
    assert!(stdout.starts_with(&format!("solver options: {options}\n")), "{stdout}");
    assert!(stdout.contains("\nlevel T1-rc-lowpass.sp (file in T1-rc-lowpass.sp): "), "{stdout}");
    assert!(stdout.contains("\n    kept [0] "), "{stdout}");
    assert!(!stdout.contains("stage S"), "{stdout}");
    let written = dir.join("out").join("T1-rc-lowpass.sp");
    assert!(stdout.ends_with(&format!("wrote {}\n", path_str(&written))), "{stdout}");
    let text = std::fs::read_to_string(&written).unwrap();
    assert!(text.contains(&format!(".{}\"", rnl_runtime::problem::options_digest(&options))), "{text}");

    // in place and without a time limit, a copy of the file becomes the snapshot
    let copy = dir.join("T1-rc-lowpass.sp");
    std::fs::copy(case("T1-rc-lowpass"), &copy).unwrap();
    let (code, stdout, _) = rnl(&["solve", &path_str(&copy), "--in-place", "--solver-options", "time_limit=none"]);
    assert_eq!(code, 0, "{stdout}");
    let options = SolverOptions::without_time_limit().canonical();
    assert!(options.ends_with(";time_limit=none"), "{options}");
    assert!(stdout.starts_with(&format!("solver options: {options}\n")), "{stdout}");
    assert!(!stdout.contains("warning: "), "{stdout}");
    let snapshot = std::fs::read(solver_dir().join("solved/T1-rc-lowpass.sp")).unwrap();
    assert!(std::fs::read(&copy).unwrap() == snapshot);

    // a file whose RNL is not interpreted: said, and nothing written; its version is an error of
    // the netlist (16.1), so the command exits with 1
    let unsupported = path_str(&corpus("13-unsupported-version.sp"));
    let (code, stdout, stderr) =
        rnl(&["solve", &unsupported, "-o", &path_str(&dir.join("none")), "--solver-options", "time_limit=none"]);
    assert_eq!(code, 1, "{stdout}{stderr}");
    assert!(stderr.contains(" errors in "), "{stderr}");
    assert!(stdout.contains("): not solved, and nothing is written for it: "), "{stdout}");
    assert!(!stdout.contains("\nwrote "), "{stdout}");
    assert!(!dir.join("none").exists());

    // the files beside the netlist that it reads are copied
    let alias = path_str(&corpus("11-header-alias-device.sp"));
    let (code, stdout, _) =
        rnl(&["solve", &alias, "-o", &path_str(&dir.join("alias")), "--solver-options", "time_limit=none"]);
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
        let run = solve(&f, &fs, &unlimited()).unwrap_or_else(|e| panic!("{name}: {e}"));
        if run.files.is_empty() {
            assert!(run.levels.is_empty(), "{name}");
            continue;
        }
        solved += 1;
        for level in &run.levels {
            let report = &level.solved.report;
            assert!(report.failed.is_empty() && report.errors.is_empty(), "{name}: {:?}", report.errors);
        }
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
        let twice = solve(&f, &Layered { layers: vec![&texts, &fs] }, &unlimited()).unwrap();
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
    let run = solve(&f, &fs, &unlimited()).unwrap();
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
        let run = solve(Path::new("/w/t.sp"), &files, &unlimited()).unwrap();
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
    let run = solve(Path::new("/w/t.sp"), &files, &unlimited()).unwrap();
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
    let run = solve(&plain, &fs, &unlimited()).unwrap();
    assert!(run.files[0].text.starts_with("* plain\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a 0 1k\n"));
    let out = dir.join("out");
    assert_eq!(write(&run, &Target::Dir(out.clone())).error, None);
    let back = interpret_file(&out.join("plain.sp"), None, &fs, &Options::default()).unwrap();
    assert!(!messages(&back, Severity::Hint).iter().any(|m| m.starts_with(hint)), "{:?}", back.diagnostics());
    check_read_back(&run, &out.join("plain.sp"), &fs, false, "plain");
    let again = solve(&out.join("plain.sp"), &fs, &unlimited()).unwrap();
    assert_eq!(again.files[0].text, run.files[0].text);

    // the included file of 04-hierarchy, and a subcircuit on the first line of one
    let run = solve(&corpus("04-hierarchy.sp"), &fs, &unlimited()).unwrap();
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
    let run = solve(Path::new("/w/top.sp"), &files, &unlimited()).unwrap();
    let div = run.files.iter().find(|f| f.path == "/w/div.inc").unwrap();
    assert!(div.text.starts_with("*\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.subckt div p n\n"), "{}", div.text);
    let solved = run.files.iter().fold(MemoryProvider::new(), |m, f| m.with(&f.path, &f.text));
    let both = Layered { layers: vec![&solved, &files] };
    check_read_back(&run, Path::new("/w/top.sp"), &both, false, "div.inc");
    let again = solve(Path::new("/w/top.sp"), &both, &unlimited()).unwrap();
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
    let pair = SolveOptions { level: Some("pair".into()), ..unlimited() };
    let run = solve(entry, &files, &pair).unwrap();
    let text = &run.files[0].text;
    assert!(!text.contains("scope=") && text.matches("type=\"generated\"").count() == 1, "{text}");
    let solved = MemoryProvider::new().with("/w/scoped.sp", text);
    let both = Layered { layers: vec![&solved, &files] };
    let r = interpret_file(entry, None, &both, &Options::default()).unwrap();
    assert!(dropped(&r).is_empty(), "{:?}", dropped(&r));
    check_read_back(&run, entry, &both, false, "pair");
    // the file level alone keeps it
    let only = SolveOptions { level: Some("scoped.sp".into()), ..unlimited() };
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
    let run = solve(Path::new("/w/t.sp"), &files, &unlimited()).unwrap();
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

// TODO: under RNL_BLESS=1 this test can read T1's snapshot before the test that blesses it has
// written it again, as the tests run in parallel; a second plain run passes. Bless with
// `--test-threads=1`.
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
    let unlimited = hash(&["problem", &f, "--solver-options", "time_limit=none"]);
    assert!(snapshot.contains(&format!("hash=\"{unlimited}\"")), "{unlimited}");
    // the default, with the time limit of 5 seconds, is another hash
    let default = hash(&["problem", &f]);
    assert!(default.ends_with(&rnl_runtime::problem::options_digest(&SolverOptions::default().canonical())));
    assert_ne!(default, unlimited);
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
fn a_header_after_a_subcircuit_solved_moves_to_line_2() {
    // a header record without VERSION after the subcircuit that is solved, in an included file and
    // in a main file solved a level at a time: the record of the subcircuit goes in before its
    // `.ends`, and would come before the header, which is the first record of its file (2.8); so
    // the header moves where a new header goes, to line 2 after the first line (the draft 0.3,
    // 16.3 and 2.2), after a bare marker put first when that line is a statement, and VERSION goes
    // into it, not into that record (16.1, 17)
    let header = "* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n";
    let body = ".subckt pair a b\nR1 a m 1k\nR2 m b 1k\n.ends\n* @RNL(type=\"header\") { }\n";
    let model = ".model nch nmos level=1\n";
    for (head, want) in
        [(format!("* lib\n{model}"), format!("* lib\n{header}{model}")), (String::new(), format!("*\n{header}"))]
    {
        let files = MemoryProvider::new()
            .with(
                "/w/top4.sp",
                "* top\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.include lib4.inc\nX1 p q pair\n.end\n",
            )
            .with("/w/lib4.inc", &format!("{head}{body}"));
        let entry = Path::new("/w/top4.sp");
        let run = solve(entry, &files, &unlimited()).unwrap();
        let lib = &run.files.iter().find(|f| f.path == "/w/lib4.inc").unwrap().text;
        assert!(lib.starts_with(&format!("{want}.subckt pair a b\n")) && lib.ends_with("* }\n.ends\n"), "{lib}");
        assert_eq!(lib.matches("type=\"header\"").count(), 1, "{lib}");
        let solved = run.files.iter().fold(MemoryProvider::new(), |m, f| m.with(&f.path, &f.text));
        let both = Layered { layers: vec![&solved, &files] };
        check_read_back(&run, entry, &both, false, "lib4.inc");
        assert_eq!(solve(entry, &both, &unlimited()).unwrap().files, run.files, "{head}");
    }

    let top =
        format!("* top\n{model}.subckt pair a b\nR1 a m 1k\nR2 m b 1k\n.ends\n* @RNL(type=\"header\") {{ }}\nX1 p q pair\n.end\n");
    let files = MemoryProvider::new().with("/w/top.sp", &top);
    let entry = Path::new("/w/top.sp");
    // the main file alone keeps the header below the subcircuit, its record going at the end
    let moved = format!("{header}{model}.subckt pair a b\n");
    let below = "\n.ends\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\nX1 p q pair\n";
    for (level, header) in [("pair", format!("* top\n{moved}")), ("top.sp", below.to_string())] {
        let one = SolveOptions { level: Some(level.into()), ..unlimited() };
        let run = solve(entry, &files, &one).unwrap();
        let text = &run.files[0].text;
        assert!(text.contains(&header) && text.matches("type=\"header\"").count() == 1, "{level}: {text}");
        assert_eq!(text.matches("@VERSION").count(), 1, "{level}: {text}");
        let solved = MemoryProvider::new().with("/w/top.sp", text);
        let both = Layered { layers: vec![&solved, &files] };
        check_read_back(&run, entry, &both, false, level);
        // the other level solved after it finds the header whole, on line 2 at the end
        let all = solve(entry, &both, &unlimited()).unwrap();
        let text = &all.files[0].text;
        assert!(text.starts_with(&format!("* top\n{moved}")), "{level}: {text}");
        assert_eq!(text.matches("type=\"header\"").count(), 1, "{level}: {text}");
        assert_eq!(text.matches("@VERSION").count(), 1, "{level}: {text}");
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
    let run = solve(entry, &files, &unlimited()).unwrap();
    assert_eq!(run.levels.iter().map(|l| l.problem.title.as_str()).collect::<Vec<_>>(), ["top.scs", "div"]);
    let div = &run.files.iter().find(|f| f.path == "/w/div.inc").unwrap().text;
    let head = "simulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.2\")}\nsubckt div (p n)\n";
    assert!(div.starts_with(head), "{div}");
    assert!(div.contains("\n// @RNL(type=\"generated\""), "{div}");
    let solved = run.files.iter().fold(MemoryProvider::new(), |m, f| m.with(&f.path, &f.text));
    let both = Layered { layers: vec![&solved, &files] };
    check_read_back(&run, entry, &both, false, "div.inc");
    assert_eq!(solve(entry, &both, &unlimited()).unwrap().files, run.files);
}

#[test]
fn above_the_switch_to_spectre_an_included_file_gets_its_header_in_stars() {
    // a file that is not a .scs file and does not start with `simulator lang=spectre` is in SPICE
    // mode up to that statement, where only `*` is a comment (2.2): the header there, and the bare
    // marker before a statement, are written with `*`, and the record of the solver follows them;
    // so is a header put before a record that the reading takes above the switch, written with
    // `*` there, so that it is the first record (2.8)
    let top = "// top\n// @RNL(type=\"header\") { @VERSION(\"0.2\")}\nsimulator lang=spectre\ninclude \"div.inc\"\n\
               X1 (p 0) div\n";
    let body = "simulator lang=spectre\nsubckt div (p n)\nR1 (p m) resistor r=1k\nR2 (m n) resistor r=1k\nends div\n";
    let star = "* @RNL(type=\"header\") { @VERSION(\"0.2\")}";
    for (head, want) in [
        ("* lib\n.param rr=1k\n", format!("* lib\n{star}\n.param rr=1k\n")),
        ("simulator lang=spice\n.param rr=1k\n", format!("*\n{star}\nsimulator lang=spice\n.param rr=1k\n")),
        ("* lib\n* @RNL { /* note */ }\n", format!("* lib\n{star}\n* @RNL {{ /* note */ }}\n")),
    ] {
        let files = MemoryProvider::new().with("/w/top.scs", top).with("/w/div.inc", &format!("{head}{body}"));
        let entry = Path::new("/w/top.scs");
        let before = interpret_file(entry, None, &files, &Options::default()).unwrap();
        let run = solve(entry, &files, &unlimited()).unwrap();
        let div = &run.files.iter().find(|f| f.path == "/w/div.inc").unwrap().text;
        assert!(div.starts_with(&format!("{want}{body}").replace("ends div\n", "")), "{head}: {div}");
        assert!(div.contains("\n* @RNL(type=\"generated\"") && !div.contains("\n// @RNL(type="), "{head}: {div}");
        let solved = run.files.iter().fold(MemoryProvider::new(), |m, f| m.with(&f.path, &f.text));
        let both = Layered { layers: vec![&solved, &files] };
        check_read_back(&run, entry, &both, false, head);
        let after = read_solved(&run, &files, entry);
        assert_eq!(messages(&after, Severity::Warning), messages(&before, Severity::Warning), "{head}");
        assert_eq!(solve(entry, &both, &unlimited()).unwrap().files, run.files, "{head}");
    }
}

#[test]
fn a_header_in_slashes_moved_into_spice_mode_carries_every_record_of_its_file() {
    // a main file that is not a .scs file and whose first statement is not `simulator
    // lang=spectre`, with its header in `//` after a subcircuit: the record of the subcircuit
    // would come before it, so it moves to line 2, where only `*` is a comment (2.2, 2.8). It
    // moves before any level of the file is written, so that the record of the file level, solved
    // first, is in `*` as well, as the next solve, which finds the header in `*`, writes it: the
    // same text twice, and no line left holding a marker alone
    let net = "* title\n.param rr=1k\nsimulator lang=spectre\nsubckt pair (a b)\nR1 (a m) resistor r=1k\n\
               R2 (m b) resistor r=1k\nends pair\n// @RNL(type=\"header\") { }\nX1 (p q) pair\n";
    let files = MemoryProvider::new().with("/w/top.net", net);
    let entry = Path::new("/w/top.net");
    let alone = |text: &str| text.lines().any(|l| l.trim() == "*" || l.trim() == "//");
    let run = solve(entry, &files, &unlimited()).unwrap();
    assert_eq!(run.levels.iter().map(|l| l.problem.title.as_str()).collect::<Vec<_>>(), ["top.net", "pair"]);
    let text = &run.files[0].text;
    let head = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n.param rr=1k\nsimulator lang=spectre\n";
    assert!(text.starts_with(head), "{text}");
    assert_eq!(text.matches("\n* @RNL(type=\"generated\"").count(), 2, "{text}");
    assert!(!text.contains("// @RNL") && !alone(text), "{text}");
    let solved = MemoryProvider::new().with("/w/top.net", text);
    let both = Layered { layers: vec![&solved, &files] };
    check_read_back(&run, entry, &both, false, "top.net");
    assert_eq!(solve(entry, &both, &unlimited()).unwrap().files, run.files);
    // the file level solved alone keeps the header where it is, in `//`, and writes its record in
    // `//`; the next solve of every level moves the header, and its record in `//` is replaced by
    // one in `*`, leaving no `//` alone on its line
    let one = SolveOptions { level: Some("top.net".into()), ..unlimited() };
    let first = solve(entry, &files, &one).unwrap();
    let text = &first.files[0].text;
    assert!(text.contains("\nends pair\n// @RNL(type=\"header\") { @VERSION(\"0.2\") }\n"), "{text}");
    assert!(text.contains("\n// @RNL(type=\"generated\""), "{text}");
    let solved = MemoryProvider::new().with("/w/top.net", text);
    let after = solve(entry, &Layered { layers: vec![&solved, &files] }, &unlimited()).unwrap();
    assert_eq!(after.files, run.files, "{}", after.files[0].text);
}

#[test]
fn a_file_read_only_through_a_library_section_gets_no_header_outside_it() {
    // a file read through `.lib file sec` or `include … section=` is read only inside the
    // section, and tools must not write outside it (2.2): it gets no header record there, the
    // record of the subcircuit goes inside the section, no new warning comes, and a second solve
    // writes the same. A header inside the section is read, and gets its version there.
    let spice = "* top\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.lib models.lib tt\nX1 p q pair\n.end\n";
    let tt = ".lib tt\n.subckt pair a b\nR1 a m 1k\nR2 m b 1k\n.ends\n.endl tt\n";
    let ff = ".lib ff\n.subckt pair a b\nR1 a m 2k\nR2 m b 2k\n.ends\n.endl ff\n";
    let spectre = "// top\n// @RNL(type=\"header\") { @VERSION(\"0.2\")}\nsimulator lang=spectre\n\
                   include \"models.scs\" section=tt\nX1 (p 0) div\n";
    let scs = "// models\nsection tt\nsubckt div (p n)\nR1 (p m) resistor r=1k\nR2 (m n) resistor r=1k\nends div\n\
               endsection tt\n";
    let inside = format!("* models\n{}", tt.replacen("\n", "\n* @RNL(type=\"header\") {}\n", 1));
    for (entry, top, lib_path, lib, header) in [
        ("/w/top.sp", spice, "/w/models.lib", format!("* models\n{tt}{ff}"), None),
        ("/w/top.scs", spectre, "/w/models.scs", scs.to_string(), None),
        (
            "/w/top.sp",
            spice,
            "/w/models.lib",
            format!("{inside}{ff}"),
            Some(".lib tt\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n"),
        ),
    ] {
        let files = MemoryProvider::new().with(entry, top).with(lib_path, &lib);
        let entry = Path::new(entry);
        let before = interpret_file(entry, None, &files, &Options::default()).unwrap();
        let run = solve(entry, &files, &unlimited()).unwrap();
        let text = &run.files.iter().find(|f| f.path == lib_path).unwrap().text;
        match header {
            None => assert!(!text.contains("type=\"header\""), "{text}"),
            Some(h) => assert!(text.contains(h) && text.matches("type=\"header\"").count() == 1, "{text}"),
        }
        let record = text.find("@RNL(type=\"generated\"").unwrap();
        assert!(record < text.find("endsection tt").or(text.find(".endl tt")).unwrap(), "{text}");
        let solved = run.files.iter().fold(MemoryProvider::new(), |m, f| m.with(&f.path, &f.text));
        let both = Layered { layers: vec![&solved, &files] };
        check_read_back(&run, entry, &both, false, lib_path);
        let after = read_solved(&run, &files, entry);
        assert_eq!(messages(&after, Severity::Warning), messages(&before, Severity::Warning), "{lib_path}");
        assert_eq!(solve(entry, &both, &unlimited()).unwrap().files, run.files, "{lib_path}");
    }
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
        let run = solve(entry, &files, &unlimited()).unwrap();
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
    let run = solve(entry, &files, &unlimited()).unwrap();
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
    let solver = SolverOptions { tolerance: 0.5, ..SolverOptions::without_time_limit() };
    let run = solve(entry, &files, &SolveOptions { solver, ..unlimited() }).unwrap();
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
    let run = solve(entry, &files, &unlimited()).unwrap();
    let discarded: Vec<&str> = run.levels[0]
        .solved
        .report
        .discarded
        .iter()
        .map(|d| run.levels[0].problem.facts[d.fact].label.as_str())
        .collect();
    assert_eq!(discarded, ["the x of #R1 = 0"]);
}

/// Every fact kept holds where the solution puts the objects.
fn check_kept(run: &SolveRun, what: &str) {
    for level in &run.levels {
        let p = &level.problem;
        let placed = Placed { p, s: &level.solved.solution };
        for &k in &level.solved.report.kept {
            assert!(placed.holds(&p.facts[k].kind, 1e-6), "{what} / {}: {}", p.title, describe_fact(p, k));
        }
    }
}

/// The options of `rnl solve` with a time limit of `seconds`, which [`Ticks`] counts in readings.
fn within(seconds: f64) -> SolveOptions {
    let solver = SolverOptions { time_limit: Some(seconds), ..SolverOptions::without_time_limit() };
    SolveOptions { solver, ..unlimited() }
}

/// What the report of a level says of the facts its resolution did not decide: a line for each,
/// discarded for want of time, and its warning on a line of its own (7.6).
fn check_undecided(level: &rnl_cli::solve::SolvedLevel, what: &str) {
    let p = &level.problem;
    let r = &level.solved.report;
    let lines = rnl_cli::solve::report(level, false);
    let undecided: Vec<usize> = r.discarded.iter().filter(|d| d.undecided).map(|d| d.fact).collect();
    assert_eq!(r.warnings.len(), undecided.len(), "{what}: {:?}", r.warnings);
    for (k, w) in undecided.iter().zip(&r.warnings) {
        assert!(w.starts_with(&format!("the fact [{k}] ")) && w.ends_with(" and is discarded (8.5)"), "{w}");
        // with where its statement is written and the definitions it was expanded from
        let f = &p.facts[*k];
        if let Some(l) = &f.source {
            let at = format!("{} ({}:{}", f.label, rnl_runtime::host::file_name(&l.file), l.line);
            assert!(w.contains(&at), "{w}: {at}");
        }
        for u in &f.chain {
            assert!(w.contains(&p.usages[*u].definition), "{w}");
        }
        let line =
            format!("  discarded {}: not decided within the time limit of the solver (8.5)", describe_fact(p, *k));
        assert!(lines.contains(&line), "{what}: {line}\n{lines:#?}");
        assert!(lines.contains(&format!("  warning: {w}")), "{what}: {w}\n{lines:#?}");
    }
    // the warnings are not notes
    assert!(!r.notes.iter().any(|n| n.contains("was not decided")), "{what}: {:?}", r.notes);
}

#[test]
fn with_no_time_at_all_only_what_the_origin_satisfies_is_kept_and_the_rest_is_warned() {
    // the time is out at the first check (the draft 0.3, 8.5): a fact that needs one is discarded
    // undecided, with a warning (7.6); a fact that every object at the origin satisfies is kept
    let body =
        "* timed\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\nR2 b c 1k\n* @RNL { #R1 @PLACE(0, 0); \
                #R2 @PLACE(10, 0); {#R1 #R2} @ALIGN(\"y\") }\n.end\n";
    let files = MemoryProvider::new().with("/w/t.sp", body);
    let run = solve_with_clock(Path::new("/w/t.sp"), &files, &within(0.0), &Ticks::default()).unwrap();
    let level = &run.levels[0];
    let p = &level.problem;
    let r = &level.solved.report;
    let fact = |text: &str| (0..p.facts.len()).find(|&k| describe_fact(p, k).contains(text)).unwrap();
    let (x2, align) = (fact("the x of #R2 = 10"), fact("ALIGN y {#R1 #R2}"));
    assert!(r.discarded.iter().any(|d| d.fact == x2 && d.undecided && d.groups.is_empty() && !d.complete), "{r:?}");
    for kept in [align, fact("the x of #R1 = 0"), fact("the y of #R1 = 0"), fact("the y of #R2 = 0")] {
        assert!(r.kept.contains(&kept), "{}: {r:?}", describe_fact(p, kept));
    }
    assert!(r.warnings.iter().any(|w| w.starts_with(&format!("the fact [{x2}] ")) && w.contains(" 0 s ")));
    check_undecided(level, "timed");
    check_kept(&run, "timed");
    assert!(level.hash.ends_with(&rnl_runtime::problem::options_digest(&within(0.0).solver.canonical())));
    // the record reads back without an error, and the same again on a fresh clock
    let solved = MemoryProvider::new().with("/w/t.sp", &run.files[0].text);
    let both = Layered { layers: vec![&solved, &files] };
    let back = interpret_file(Path::new("/w/t.sp"), None, &both, &Options::default()).unwrap();
    let errors: Vec<_> = back.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let again = solve_with_clock(Path::new("/w/t.sp"), &files, &within(0.0), &Ticks::default()).unwrap();
    assert_eq!(again.files, run.files);
    // without a limit, every fact is decided
    let all = solve(Path::new("/w/t.sp"), &files, &unlimited()).unwrap();
    let report = &all.levels[0].solved.report;
    assert!(report.discarded.is_empty() && report.warnings.is_empty(), "{report:?}");
}

#[test]
fn every_minimal_group_is_named_and_a_limit_that_cuts_them_says_the_list_is_not_complete() {
    // an ALIGN on x of three resistors written before their PLACEs apart on x, which rank first:
    // each pair of the PLACEs is a minimal conflict group of the ALIGN, and its line names the three
    // (the draft 0.3, 8.5). A limit that runs out after every fact is decided, while the groups are
    // found, keeps the facts of 8.5 and gives the groups found by then, each minimal, and the line
    // of the ALIGN says that the list is not complete, as does the head of the hints by statement;
    // with more time, the groups are those of 8.5
    let body = "* cut\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\nR2 b c 1k\nR3 c d 1k\n* @RNL { \
                {#R1 #R2 #R3} @ALIGN(\"x\"); #R1 @PLACE(0, 0); #R2 @PLACE(10, 0); #R3 @PLACE(20, 0) }\n.end\n";
    let files = MemoryProvider::new().with("/w/t.sp", body);
    let clock = Ticks::default();
    let all = solve_with_clock(Path::new("/w/t.sp"), &files, &within(1e9), &clock).unwrap();
    let reads = clock.reads();
    let level = &all.levels[0];
    let p = &level.problem;
    let whole = &level.solved.report;
    assert_eq!(whole, &solve(Path::new("/w/t.sp"), &files, &unlimited()).unwrap().levels[0].solved.report);
    let fact = |text: &str| (0..p.facts.len()).find(|&k| describe_fact(p, k).contains(text)).unwrap();
    let align = fact("ALIGN x");
    let xs = [fact("the x of #R1 = 0"), fact("the x of #R2 = 10"), fact("the x of #R3 = 20")];
    let mut groups: Vec<Vec<usize>> = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(a, b)| {
            let mut g = vec![xs[a], xs[b]];
            g.sort_unstable();
            g
        })
        .collect();
    groups.sort();
    assert_eq!(whole.discarded.len(), 1, "{whole:?}");
    let d = &whole.discarded[0];
    assert_eq!((d.fact, &d.groups, d.complete), (align, &groups, true));
    let lines = rnl_cli::solve::report(level, false);
    let listed: Vec<String> = groups.iter().map(|g| format!("[{}] and [{}]", g[0], g[1])).collect();
    let line = format!("  discarded {}: it conflicts with {}", describe_fact(p, align), listed.join("; or with "));
    assert!(lines.contains(&line), "{line}\n{lines:#?}");
    // each PLACE's statement took part in discarding the ALIGN
    for x in xs {
        let end = format!(" took part in discarding [{align}]");
        let named =
            |l: &&String| l.starts_with("    t.sp:6 ") && l.contains(&format!(" [{x}] the x of ")) && l.ends_with(&end);
        assert!(lines.iter().any(|l| named(&l)), "{x}: {lines:#?}");
    }
    let incomplete = " (the list is not complete: the time limit was reached)";
    assert!(!lines.iter().any(|l| l.contains(incomplete)), "{lines:#?}");
    let mut cut = Vec::new();
    for limit in 0..=reads {
        let run = solve_with_clock(Path::new("/w/t.sp"), &files, &within(limit as f64), &Ticks::default()).unwrap();
        let level = &run.levels[0];
        let r = &level.solved.report;
        check_undecided(level, &format!("{limit} s"));
        check_kept(&run, &format!("{limit} s"));
        if !r.warnings.is_empty() {
            continue;
        }
        assert_eq!(r.kept, whole.kept, "{limit} s");
        let d = &r.discarded[0];
        let lines = rnl_cli::solve::report(level, false);
        if d.complete {
            assert_eq!(r.discarded, whole.discarded, "{limit} s");
            assert_eq!(r.statements, whole.statements, "{limit} s");
            assert!(!lines.iter().any(|l| l.contains(incomplete)), "{limit} s: {lines:#?}");
            continue;
        }
        cut.push(d.groups.len());
        assert!(d.groups.iter().all(|g| groups.contains(g)), "{limit} s: {d:?}");
        let start = format!("  discarded {}: ", describe_fact(p, align));
        let line = lines.iter().find(|l| l.starts_with(&start)).unwrap();
        assert!(line.ends_with(incomplete), "{limit} s: {line}");
        let head =
            "  statements whose facts took part in discarding (8.5; the list is not complete: the time limit was reached):";
        assert_eq!(lines.iter().any(|l| l == head), !d.groups.is_empty(), "{limit} s: {lines:#?}");
    }
    // cut before the first group was found, after it, and after the second
    assert!(cut.contains(&0) && cut.contains(&1) && cut.contains(&2), "{cut:?}");
}

#[test]
fn a_case_whose_resolution_runs_out_of_time_keeps_what_it_decided() {
    // T4 with 35 readings of the clock, of which the checks of its facts on their own take about
    // 25 (the draft 0.3, 8.5): the facts decided by then are those of 8.5 as far as they go, two
    // of them discarded, each later fact that needs a check is undecided, and every fact kept holds
    let fs = FsProvider::new(&[]);
    let run = solve_with_clock(&case("T4-miller-conflicts"), &fs, &within(35.0), &Ticks::default()).unwrap();
    let level = &run.levels[0];
    let r = &level.solved.report;
    assert!(!r.warnings.is_empty() && !r.kept.is_empty(), "{r:?}");
    assert!(r.discarded.iter().any(|d| !d.undecided), "the time ran out before the order of 8.5: {r:?}");
    check_undecided(level, "T4");
    check_kept(&run, "T4");
    let first = r.discarded.iter().find(|d| d.undecided).unwrap().fact;
    // after it, a fact is kept, as the placement reached by then satisfies it, or undecided
    assert!(r.discarded.iter().filter(|d| d.fact > first).all(|d| d.undecided), "{r:?}");
    let all = solve(&case("T4-miller-conflicts"), &fs, &unlimited()).unwrap();
    let whole = &all.levels[0].solved.report;
    // before the first fact left undecided, what is kept is what 8.5 keeps
    let before = |kept: &[usize]| kept.iter().copied().filter(|&k| k < first).collect::<Vec<_>>();
    assert_eq!(before(&r.kept), before(&whole.kept));
    let again = solve_with_clock(&case("T4-miller-conflicts"), &fs, &within(35.0), &Ticks::default()).unwrap();
    assert_eq!(again.files, run.files, "not the same on a fresh clock");
    let lines = lines(&run, false);
    assert!(lines.iter().any(|l| l.starts_with("  warning: the fact [")), "{lines:#?}");
}

/// A netlist whose RNL has, with `statements` in it, an NMOS, two resistors placed apart that an
/// ALIGN y puts on one height, and the ground stubs of the standard package.
fn with_statements(statements: &str) -> String {
    format!(
        "* facts that contradict themselves\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\n\
         .model nch nmos level=1\nM1 out in 0 0 nch W=4u L=1u\nR1 out a 1k\nR2 a 0 1k\n* @RNL {{\n\
         *   # @SELECT-INSTANCE(model=\"nch\") => @SET-SYMBOL $NMOS;\n{statements}\
         *   #R1 @PLACE(0, 0);\n*   #R2 @PLACE(0, 5);\n*   {{#R1 #R2}} @ALIGN(\"y\")\n* }}\n.end\n"
    )
}

/// What a fact says and where, without its rank.
fn unranked(p: &LevelProblem, k: usize) -> String {
    let d = describe_fact(p, k);
    d[d.find("] ").unwrap() + 2..].to_string()
}

#[test]
fn a_fact_that_contradicts_itself_fails_and_the_level_is_solved_without_it() {
    // The drain and the gate of the NMOS, at (0, 2) and (−3, 0), at one height whatever its
    // orientation; an ORDER of the NMOS and the centre of its drain and its source, which is its
    // origin whatever its orientation; its drain and its source at least 5 apart on x, where they
    // are 0 or 4 apart; its gate and its drain exactly 1 apart on y, where they are 2 or 3 apart.
    // Only the pins of its symbol and its orientations tie these points, which the runtime does
    // not look at (8.5, 9.5): it gives them to the solver, and the solver finds that none has a
    // solution on its own: each fails with an error (the draft 0.3, 8.5; 7.6), takes no part in
    // the resolution, and the level is solved as without them.
    let failing = "*   {#M1:d #M1:g} @ALIGN(\"y\");\n*   {#M1 {#M1:d #M1:s}} @ORDER(\"x\");\n\
                   *   #M1:d #M1:s @DISTANCE(\"x\", min=5);\n*   #M1:g #M1:d @DISTANCE(\"y\", exact=1);\n";
    let files = MemoryProvider::new().with("/w/self.sp", &with_statements(failing));
    let run = solve(Path::new("/w/self.sp"), &files, &unlimited()).unwrap();
    assert_eq!(run.errors, 0, "the runtime does not fail them");
    let level = &run.levels[0];
    let p = &level.problem;
    let r = &level.solved.report;
    let fact = |text: &str| (0..p.facts.len()).find(|&k| describe_fact(p, k).contains(text)).unwrap();
    let mut failed = vec![
        fact("ALIGN y {#M1:d #M1:g}"),
        fact("ORDER x either {#M1 {#M1:d #M1:s}}"),
        fact("{#M1:d} {#M1:s}"),
        fact("{#M1:g} {#M1:d}"),
    ];
    failed.sort_unstable();
    assert_eq!(r.failed, failed);
    // in no list but their own: not kept, not discarded, in no group and no hint by statement
    assert!(failed.iter().all(|k| !r.kept.contains(k)), "{r:?}");
    assert!(r.discarded.iter().all(|d| !failed.contains(&d.fact) && d.in_groups().iter().all(|k| !failed.contains(k))));
    assert!(r.statements.values().all(|h| h.facts.iter().all(|k| !failed.contains(k))), "{:?}", r.statements);
    assert_eq!(r.kept.len() + r.discarded.len() + r.failed.len(), p.facts.len());
    // an error for each, on a line of its own
    let lines = rnl_cli::solve::report(level, false);
    assert_eq!(r.errors.len(), 4);
    for (e, &k) in r.errors.iter().zip(&failed) {
        let line = p.facts[k].source.as_ref().unwrap().line;
        assert_eq!(
            e,
            &format!("the fact [{k}] {} (self.sp:{line}) contradicts itself and fails (8.5, 7.6)", p.facts[k].label)
        );
        assert!(lines.contains(&format!("  error: {e}")), "{e}\n{lines:#?}");
    }
    assert!(
        lines[0].ends_with(&format!(" kept, {} discarded, 4 failed of 13 facts", r.discarded.len())),
        "{}",
        lines[0]
    );
    let all = rnl_cli::solve::report(level, true);
    for &k in &failed {
        let line = format!("    failed {}: it contradicts itself (8.5, 7.6)", describe_fact(p, k));
        assert!(all.contains(&line), "{line}\n{all:#?}");
    }
    // the rest is solved as without them, the lines left empty: the same facts kept and discarded,
    // with the same groups, and the same solution
    let blank = "*\n".repeat(4);
    let none = MemoryProvider::new().with("/w/self.sp", &with_statements(&blank));
    let without = solve(Path::new("/w/self.sp"), &none, &unlimited()).unwrap();
    let other = &without.levels[0];
    let q = &other.problem;
    let described = |p: &LevelProblem, ks: &[usize]| ks.iter().map(|&k| unranked(p, k)).collect::<Vec<_>>();
    assert_eq!(described(p, &r.kept), described(q, &other.solved.report.kept));
    let groups = |p: &LevelProblem, ds: &[rnl_solver::Discarded]| {
        ds.iter()
            .map(|d| (unranked(p, d.fact), d.groups.iter().map(|g| described(p, g)).collect::<Vec<_>>()))
            .collect::<Vec<_>>()
    };
    assert_eq!(groups(p, &r.discarded), groups(q, &other.solved.report.discarded));
    assert!(!r.discarded.is_empty());
    assert_eq!(level.solved.solution, other.solved.solution);
    // the record is written all the same
    assert!(run.files[0].text.contains(&level.hash), "{}", run.files[0].text);
    // with no time at all, their checks on their own do not finish: undecided, not failed
    let cut = solve_with_clock(Path::new("/w/self.sp"), &files, &within(0.0), &Ticks::default()).unwrap();
    let r = &cut.levels[0].solved.report;
    assert!(r.failed.is_empty() && r.errors.is_empty(), "{r:?}");
    assert!(failed.iter().all(|k| r.discarded.iter().any(|d| d.fact == *k && d.undecided)), "{r:?}");
    check_undecided(&cut.levels[0], "no time");

    // the command prints the errors, writes the file and exits with 1
    let dir = scratch("self-contradiction");
    let file = dir.join("self.sp");
    std::fs::write(&file, with_statements(failing)).unwrap();
    let out = dir.join("out");
    let (code, stdout, stderr) =
        rnl(&["solve", &path_str(&file), "-o", &path_str(&out), "--solver-options", "time_limit=none"]);
    assert_eq!(code, 1, "{stdout}{stderr}");
    assert_eq!(stdout.matches("\n  error: the fact [").count(), 4, "{stdout}");
    assert!(stdout.ends_with(&format!("wrote {}\n", path_str(&out.join("self.sp")))), "{stdout}");
    assert!(out.join("self.sp").exists());
    // and without them, 0
    std::fs::write(&file, with_statements("")).unwrap();
    let (code, stdout, _) =
        rnl(&["solve", &path_str(&file), "-o", &path_str(&out), "--solver-options", "time_limit=none"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(!stdout.contains("error: "), "{stdout}");
}

#[test]
fn a_fact_that_the_runtime_sees_contradict_itself_never_reaches_the_solver() {
    // an ORDER of one set written twice, and an ORDER of a resistor and the set of it alone: from
    // what is written, two members at one place. The runtime fails each itself, with an error at
    // its statement, and leaves it out of the problem (8.5, 9.5): the solver fails nothing, and the
    // level is solved as without them; the command gives the errors of the netlist on the standard
    // error, writes the file and exits with 1, as for any error (7.6)
    let failing = "*   {{#R1 #R2} {#R1 #R2}} @ORDER(\"y\");\n*   {#R1 {#R1}} @ORDER(\"y\");\n";
    let files = MemoryProvider::new().with("/w/self.sp", &with_statements(failing));
    let run = solve(Path::new("/w/self.sp"), &files, &unlimited()).unwrap();
    assert_eq!(run.errors, 2, "the runtime fails them");
    let level = &run.levels[0];
    let p = &level.problem;
    let r = &level.solved.report;
    assert!((0..p.facts.len()).all(|k| !describe_fact(p, k).contains("ORDER")), "{:#?}", p.facts);
    assert!(r.failed.is_empty() && r.errors.is_empty(), "{r:?}");
    let none = MemoryProvider::new().with("/w/self.sp", &with_statements(&"*\n".repeat(2)));
    let without = solve(Path::new("/w/self.sp"), &none, &unlimited()).unwrap();
    let other = &without.levels[0];
    let described = |p: &LevelProblem| (0..p.facts.len()).map(|k| unranked(p, k)).collect::<Vec<_>>();
    assert_eq!(described(p), described(&other.problem));
    assert_eq!(level.solved.solution, other.solved.solution);

    let dir = scratch("runtime-contradiction");
    let file = dir.join("self.sp");
    std::fs::write(&file, with_statements(failing)).unwrap();
    let out = dir.join("out");
    let (code, stdout, stderr) =
        rnl(&["solve", &path_str(&file), "-o", &path_str(&out), "--solver-options", "time_limit=none"]);
    assert_eq!(code, 1, "{stdout}{stderr}");
    assert!(stderr.contains("2 errors in "), "{stderr}");
    assert!(!stdout.contains("error: "), "{stdout}");
    assert!(stdout.ends_with(&format!("wrote {}\n", path_str(&out.join("self.sp")))), "{stdout}");
    assert!(out.join("self.sp").exists());
}

#[test]
fn the_command_exits_with_1_on_the_errors_of_the_netlist() {
    // the corpus cases written with errors (7.6) are solved and written all the same, and the
    // command exits with 1, as `rnl check` does, with their number on the standard error
    for name in ["12-diagnostics.sp", "16-ties.sp", "19-set-cycles.sp"] {
        let f = corpus(name);
        let errors = interpret_file(&f, None, &FsProvider::new(&[]), &Options::solver()).unwrap();
        let errors = errors.diagnostics().iter().filter(|d| d.severity == Severity::Error).count();
        assert!(errors > 0, "{name}");
        let (code, _, _) = rnl(&["check", &path_str(&f), "--no-generated"]);
        assert_eq!(code, 1, "{name}");
        let out = scratch(&format!("errors-{name}"));
        let (code, stdout, stderr) =
            rnl(&["solve", &path_str(&f), "-o", &path_str(&out), "--solver-options", "time_limit=none"]);
        assert_eq!(code, 1, "{name}: {stdout}{stderr}");
        assert!(stderr.contains(&format!("{errors} errors in ")), "{name}: {stderr}");
        assert!(stdout.contains(&format!("wrote {}", path_str(&out.join(name)))), "{name}: {stdout}");
    }
}

#[test]
fn a_place_decided_before_the_time_ran_out_is_written_as_written() {
    // #R1 at (5.5, 0) is decided; #R2 at x = 10, after it in the order of 8.5, is not, for want of
    // time. The positions are still taken from the facts kept with their constants as written,
    // which is not timed: the record writes 5.5, not a tolerance off it, every fact kept holds at
    // 1e-6 where the solution puts the objects, and read back, no fact kept is dropped against
    // the record (8.5, 17)
    let body = "* decimal\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\nR2 b c 1k\n\
                * @RNL { #R2 @PLACE(x=10); #R1 @PLACE(5.5, 0) }\n.end\n";
    let files = MemoryProvider::new().with("/w/t.sp", body);
    let entry = Path::new("/w/t.sp");
    let clock = Ticks::default();
    solve_with_clock(entry, &files, &within(1e9), &clock).unwrap();
    let mut cut = 0;
    for limit in 0..=clock.reads() {
        let run = solve_with_clock(entry, &files, &within(limit as f64), &Ticks::default()).unwrap();
        let level = &run.levels[0];
        let p = &level.problem;
        let r = &level.solved.report;
        let fact = |text: &str| (0..p.facts.len()).find(|&k| describe_fact(p, k).contains(text)).unwrap();
        let (x1, x2) = (fact("the x of #R1 = 5.5"), fact("the x of #R2 = 10"));
        if !(r.kept.contains(&x1) && r.discarded.iter().any(|d| d.fact == x2 && d.undecided)) {
            continue;
        }
        cut += 1;
        check_kept(&run, &format!("{limit} s"));
        assert!(level.record.contains("#R1 @PLACE(5.5, 0)"), "{limit} s: {}", level.record);
        let back = read_solved(&run, &files, entry);
        let dropped = dropped(&back);
        for &k in &r.kept {
            let label = &p.facts[k].label;
            assert!(
                !dropped.iter().any(|m| m.starts_with(&format!("{label} conflicts with"))),
                "{limit} s: {dropped:?}"
            );
        }
    }
    assert!(cut > 0, "no limit decides #R1 and leaves #R2 undecided");
}
