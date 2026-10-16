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
    let options = SolverOptions::default();
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
