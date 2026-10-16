//! A solve written back and read again (standard 2.8, 13.2, 13.7, chapter 17), through the public
//! interface only, as a tool that solves does it: every level of the cases written for the solver
//! is read from its file as a solver reads it, solved, its record written into the file text in
//! place of the level's old one, and the file read again as a viewer reads it.

use std::path::{Path, PathBuf};

use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::problem::{same_input, sig1, LevelProblem};
use rnl_runtime::solution::{write_record, Decision, WirePoint};
use rnl_runtime::source::{Layered, MemoryProvider, Provider};
use rnl_runtime::writeback::replace_generated;
use rnl_runtime::Options;
use rnl_solver::{solve, SolverOptions, AUTHOR};

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

fn cases() -> Vec<String> {
    let dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/integration/solver");
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "sp"))
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    out.sort();
    out
}

fn problems(entry: &str, files: &dyn Provider) -> Vec<LevelProblem> {
    rnl_view::interpret(entry, None, files, &Options::solver()).unwrap_or_else(|e| panic!("{entry}: {e}")).problems
}

#[test]
fn every_level_solved_and_written_back_is_drawn_with_its_wires() {
    let options = SolverOptions::without_time_limit();
    let files = cases();
    assert!(files.len() >= 7, "{files:?}");
    let mut levels = 0;
    for entry in files {
        let text = Files.read(&entry).unwrap();
        for p in problems(&entry, &Files) {
            let what = format!("{entry} / {}", p.title);
            let solved = solve(&p, &options);
            assert_eq!(solved, solve(&p, &options), "{what}: not the same twice");
            assert!(solved.report.unrouted.is_empty(), "{what}: {:?}", solved.report.unrouted);
            // nothing that RNL cannot write
            for w in &solved.solution.wires {
                for q in &w.points {
                    if let WirePoint::Bend((x, y)) = q {
                        assert!(x.is_finite() && y.is_finite(), "{what}: {w:?}");
                    }
                }
            }
            let hash = sig1(&p, &options.canonical());
            let written = write_record(&p, &solved.solution, AUTHOR, &hash);
            let lost: Vec<String> = written
                .unsolved
                .iter()
                .filter(|u| matches!(u.decision, Decision::RoutedWire { .. } | Decision::Branch { .. }))
                .map(|u| u.to_string())
                .collect();
            assert!(lost.is_empty(), "{what}: {lost:?}");
            let doc = rnl_view::read_host(&entry, None, &Files, &mut Sink::new()).unwrap();
            let record = rnl_view::finish_record(&doc, &Files, &Options::solver(), &p, &written.record);
            let out = replace_generated(&doc, p.level, &text, &record);
            let mem = MemoryProvider::new().with(&entry, &out);
            let both = Layered { layers: vec![&mem, &Files] };
            let r = rnl_view::interpret(&entry, None, &both, &Options::default()).unwrap();
            let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
            assert!(errors.is_empty(), "{what}: {errors:?}\n{record}");
            let lv = r.model.levels.iter().find(|l| l.level == p.level).unwrap();
            assert!(lv.unplaced.is_empty(), "{what}: {:?}", lv.unplaced);
            // the ROUTEs are wired: a viewer has no flight line left to draw
            assert!(lv.flylines.is_empty(), "{what}: {:?}\n{record}", lv.flylines);
            // the wires routed are drawn as written, straight between their bends
            let straight = lv.wires.iter().filter(|w| w.mode == "straight").count();
            assert_eq!(straight, solved.solution.wires.len(), "{what}");
            if !p.routes.is_empty() {
                assert!(straight > 0, "{what}");
            }
            // the dots of the branch points are there: one $JUNCTION each
            let dots = lv.items.iter().filter(|i| i.kind == "usage" && lv.symbols[i.symbol].name == "$JUNCTION_DOT");
            assert_eq!(dots.count(), solved.solution.branches.len(), "{what}");
            // read as a solver reads it, the input is the same: the hash still holds
            let again = problems(&entry, &both);
            let q = again.iter().find(|q| q.level == p.level).unwrap();
            assert_eq!(same_input(&hash, q), Some(true), "{what}");
            levels += 1;
        }
    }
    assert!(levels >= 9, "{levels}");
}

/// The netlists of the review of the routing (stage S4), as files: pins of two nets face to face in
/// a row, a star of resistors, a supply and a ground net routed with their stubs, a ground drawn on
/// a pin, symbols placed far apart. Each is solved with every ROUTE whole, written back and read
/// again: no error, nothing unplaced, every net routed without a flight line, a straight wire for
/// each wire routed, a dot for each branch point, the hash the same.
#[test]
fn the_netlists_of_the_review_are_routed_whole_and_read_back() {
    let head = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n";
    let circuit = "V1 vdd 0 1\nR1 vdd out 1k\nC1 out 0 1n\nR2 out 0 1k\n";
    let star = "R1 hub x 1k\nR2 hub x 1k\nR3 hub x 1k\nR4 hub x 1k\nR5 hub x 1k\nR6 hub x 1k\nR7 x b 1k\nR8 hub d 1k\n";
    let bodies = [
        "R1 a b 1k\nR2 a b 1k\nR3 a c 1k\n* @RNL { # @SELECT-NET => @ROUTE }\n".to_string(),
        format!("{star}* @RNL {{ # @SELECT-NET => @ROUTE }}\n"),
        format!("{circuit}* @RNL {{ &vdd @ADD-TO $SUPPLY_NET_SET; # @SELECT-NET => @ROUTE }}\n"),
        format!("{circuit}* @RNL {{ $GND @ATTACH #C1:2; # @SELECT-NET => @ROUTE }}\n"),
        "R1 a b 1k\nR2 a b 1k\n* @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(25000, 0); # @SELECT-NET => @ROUTE }\n"
            .to_string(),
    ];
    let options = SolverOptions::without_time_limit();
    for body in bodies {
        let entry = "t.sp";
        let text = format!("{head}{body}.end\n");
        let files = MemoryProvider::new().with(entry, &text);
        let ps = problems(entry, &files);
        assert_eq!(ps.len(), 1, "{body}");
        let p = &ps[0];
        let solved = solve(p, &options);
        assert!(solved.report.unrouted.is_empty(), "{body}: {:?}", solved.report.unrouted);
        let hash = sig1(p, &options.canonical());
        let written = write_record(p, &solved.solution, AUTHOR, &hash);
        let lost: Vec<String> = written
            .unsolved
            .iter()
            .filter(|u| matches!(u.decision, Decision::RoutedWire { .. } | Decision::Branch { .. }))
            .map(|u| u.to_string())
            .collect();
        assert!(lost.is_empty(), "{body}: {lost:?}");
        let doc = rnl_view::read_host(entry, None, &files, &mut Sink::new()).unwrap();
        let record = rnl_view::finish_record(&doc, &files, &Options::solver(), p, &written.record);
        let out = replace_generated(&doc, p.level, &text, &record);
        let again = MemoryProvider::new().with(entry, &out);
        let r = rnl_view::interpret(entry, None, &again, &Options::default()).unwrap();
        let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
        assert!(errors.is_empty(), "{body}: {errors:?}\n{record}");
        let lv = r.model.levels.iter().find(|l| l.level == p.level).unwrap();
        assert!(lv.unplaced.is_empty(), "{body}: {:?}", lv.unplaced);
        assert!(lv.flylines.is_empty(), "{body}: {:?}\n{record}", lv.flylines);
        let straight = lv.wires.iter().filter(|w| w.mode == "straight").count();
        assert_eq!(straight, solved.solution.wires.len(), "{body}");
        let dots = lv.items.iter().filter(|i| i.kind == "usage" && lv.symbols[i.symbol].name == "$JUNCTION_DOT");
        assert_eq!(dots.count(), solved.solution.branches.len(), "{body}");
        let q = problems(entry, &again);
        assert_eq!(same_input(&hash, &q[0]), Some(true), "{body}");
    }
}
