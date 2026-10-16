//! `rnl solve` (standard 2.8, 8.5, 13.7, 14.8, chapter 17), through the library as the command
//! runs it: each case written for the solver (`tests/integration/solver`) is read as a solver
//! reads it, every drawing level solved and its record of the solver written back, level by
//! level, into a temporary directory, and the file read again as a viewer reads it: no error,
//! nothing unplaced, every constraint holding unless 8.5 discarded it, the hash the same, and the
//! text the snapshot in `tests/integration/solver/solved`. After a change to the solver that is
//! meant, `RNL_BLESS=1 cargo test -p rnl-cli --test solve` writes the snapshots again.

use std::path::{Path, PathBuf};
use std::process::Command;

use rnl_cli::solve::{destinations, solve, write, SolveOptions, SolveRun, Target};
use rnl_cli::{interpret_file, path_str, FsProvider};
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::{same_input, FactKind, PointRef, ProblemFact};
use rnl_runtime::solution::Decision;
use rnl_runtime::source::{Layered, MemoryProvider, Provider};
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

/// Whether a fact reaches one of the objects: by its origin, a pin, or in the centre of a set.
fn reaches(f: &ProblemFact, objects: &[usize]) -> bool {
    fn point(r: &PointRef, objects: &[usize]) -> bool {
        match r {
            PointRef::Origin(k) | PointRef::Pin { object: k, .. } => objects.contains(k),
            PointRef::Centre(rs) => rs.iter().any(|r| point(r, objects)),
        }
    }
    match &f.kind {
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
/// every level solved has nothing unplaced, and its constraints hold, but for those that 8.5
/// discarded, which are dropped (8.5), and those that reach an object whose position the record
/// cannot hold (`Written::unsolved`, such as a point without a name in a file of 0.2), which
/// cannot be checked; read as a solver reads it, each level has the same input as the hash of its
/// record says (17).
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
        let mut want: Vec<(String, &str)> = p
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
                let status = if reaches(f, &unwritable) {
                    "unsolved"
                } else if level.solved.report.discarded.iter().any(|d| d.fact == k) {
                    "dropped"
                } else {
                    "ok"
                };
                (f.kind.name().to_string(), status)
            })
            .collect();
        let mut got: Vec<(String, &str)> = lv
            .constraints
            .iter()
            .filter(|c| c.status != "disabled")
            .map(|c| (c.kind.clone(), c.status.as_str()))
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
        // every drawing level of the file, in one file
        let all = interpret_file(&f, None, &fs, &Options::solver()).unwrap().problems;
        let titles: Vec<&str> = run.levels.iter().map(|l| l.problem.title.as_str()).collect();
        assert_eq!(titles, all.iter().map(|p| p.title.as_str()).collect::<Vec<_>>(), "{name}");
        assert_eq!(run.files.len(), 1, "{name}");
        for level in &run.levels {
            let report = &level.solved.report;
            assert_eq!(report.kept.len() + report.discarded.len(), level.problem.facts.len(), "{name}");
            assert!(report.unrouted.is_empty(), "{name}: {:?}", report.unrouted);
            assert!(level.record.starts_with("@RNL(type=\"generated\""), "{name}");
            assert!(level.record.contains(&format!("hash=\"{}\"", level.hash)), "{name}");
            assert!(level.hash.ends_with(&rnl_runtime::problem::options_digest(&SolverOptions::default().canonical())));
            // every level of the file was written back: its record is in the text, the last one
            // finished, the earlier ones as the levels after them found them
            assert!(run.files[0].text.contains(&level.hash), "{name}");
        }
        let dir = scratch(name);
        let written = write(&run, &Target::Dir(dir.clone())).unwrap();
        let out = dir.join(format!("{name}.sp"));
        assert_eq!(written, std::slice::from_ref(&out), "{name}");
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
    // the middle point of $TAIL_CENTER has no name: a file of 0.2 cannot place it (5.9)
    assert!(lines.iter().any(|l| l.starts_with("    the position of %mid: it has no name")), "{lines:#?}");
    assert!(lines.iter().any(|l| l.starts_with("  work: ")) && lines.iter().any(|l| l.starts_with("  time: ")));
    // with --report every fact, in the order of 8.5
    let all = rnl_cli::solve::report(level, true);
    assert_eq!(all.iter().filter(|l| l.starts_with("    kept [")).count(), 38);
    assert_eq!(all.iter().filter(|l| l.starts_with("    discarded [")).count(), 3);
    assert!(all.iter().any(|l| l == "    kept [0] the y of #M1 = 0 (high, T4-miller-conflicts.sp:37)"), "{all:#?}");
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
    // T3 and T4 declaring the draft 0.3: the middle point of $TAIL_CENTER, which a file of 0.2
    // cannot refer to, is placed by its number, with the check of the numbers (5.9), so every
    // constraint holds but those that 8.5 discarded
    let fs = FsProvider::new(&[]);
    for name in ["T3-ota5t", "T4-miller-conflicts"] {
        let f = case(name);
        let text = fs.read(&path_str(&f)).unwrap().replace("@VERSION(\"0.2\")", "@VERSION(\"0.3\")");
        let mem = MemoryProvider::new().with(&path_str(&f), &text);
        let both = Layered { layers: vec![&mem, &fs] };
        let run = solve(&f, &both, &SolveOptions { draft: true, ..SolveOptions::default() }).unwrap();
        for level in &run.levels {
            assert!(level.written.unsolved.is_empty(), "{name}: {:?}", level.written.unsolved);
        }
        let with_mid = run.levels.iter().find(|l| l.record.contains("::mid @PLACE(")).unwrap();
        assert!(with_mid.record.contains("numbering=\""), "{}", with_mid.record);
        let solved = MemoryProvider::new().with(&path_str(&f), &run.files[0].text);
        check_read_back(&run, &f, &Layered { layers: vec![&solved, &fs] }, true, name);
    }
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
    let f = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases/04-hierarchy.sp");
    let run = solve(&f, &fs, &SolveOptions::default()).unwrap();
    let titles: Vec<&str> = run.levels.iter().map(|l| l.problem.title.as_str()).collect();
    assert_eq!(titles, ["04-hierarchy.sp", "rdiv", "amp"]);
    let names: Vec<&str> = run.files.iter().map(|f| rnl_runtime::host::file_name(&f.path)).collect();
    assert_eq!(names, ["04-hierarchy.sp", "04-hierarchy-lib.inc"]);
    let dir = scratch("04-hierarchy");
    let written = write(&run, &Target::Dir(dir.clone())).unwrap();
    assert_eq!(written, [dir.join("04-hierarchy.sp"), dir.join("04-hierarchy-lib.inc")]);
    check_read_back(&run, &dir.join("04-hierarchy.sp"), &fs, false, "04-hierarchy");
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
    let to = destinations(&run, &Target::InPlace).unwrap();
    assert_eq!(to, [PathBuf::from("/w/a/top.sp"), PathBuf::from("/w/lib/div.inc")]);
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
}
