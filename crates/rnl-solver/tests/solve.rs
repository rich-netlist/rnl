//! A solve written back and read again (standard 2.8, 13.2, 13.7, chapter 17), through the public
//! interface only, as a tool that solves does it: every level of the cases written for the solver
//! is read from its file as a solver reads it, solved, its record written into the file text in
//! place of the level's old one, and the file read again as a viewer reads it.

use std::path::{Path, PathBuf};

use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::problem::{same_input, sig2, LevelProblem};
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
            let hash = sig2(&p, &options.canonical());
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
            // the ROUTEs are wired: a viewer has no flight line left to draw on their nets
            let routed = |f: &rnl_runtime::model::FlylineView| p.routes.iter().any(|r| f.nets.contains(&r.net));
            assert!(!lv.flylines.iter().any(routed), "{what}: {:?}\n{record}", lv.flylines);
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
        let hash = sig2(p, &options.canonical());
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
        let routed = |f: &rnl_runtime::model::FlylineView| p.routes.iter().any(|r| f.nets.contains(&r.net));
        assert!(!lv.flylines.iter().any(routed), "{body}: {:?}\n{record}", lv.flylines);
        let straight = lv.wires.iter().filter(|w| w.mode == "straight").count();
        assert_eq!(straight, solved.solution.wires.len(), "{body}");
        let dots = lv.items.iter().filter(|i| i.kind == "usage" && lv.symbols[i.symbol].name == "$JUNCTION_DOT");
        assert_eq!(dots.count(), solved.solution.branches.len(), "{body}");
        let q = problems(entry, &again);
        assert_eq!(same_input(&hash, &q[0]), Some(true), "{body}");
    }
}

/// Two trees of one net, as two structures make them: routed apart by default, each joined by the
/// net's name, and as one with `join_trees`, every pin of the net in one connected drawing, each
/// wire given back by the first tree of its net. A tree of another net after them keeps its own
/// index, as the problem has it.
#[test]
fn join_trees_routes_the_trees_of_a_net_as_one() {
    let text = r#"* two trees
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4") }
.subckt two a b
R1 a n 1k
R2 n b 1k
R3 a n 1k
R4 n b 1k
R5 a m 1k
R6 m b 1k
* @RNL {
*   {#R1:2 #R2:1} @ROUTE-OBJECT;
*   {#R3:2 #R4:1} @ROUTE-OBJECT;
*   {#R5:2 #R6:1} @ROUTE-OBJECT;
* }
.ends
"#;
    let files = MemoryProvider::new().with("/w/two.sp", text);
    let p = problems("/w/two.sp", &files).into_iter().find(|p| p.title == "two").unwrap();
    assert_eq!(p.routes.iter().map(|r| r.net.as_str()).collect::<Vec<_>>(), ["n", "n", "m"]);
    // the parts the wires join the pins into, by their counts of pins
    let parts = |solved: &rnl_solver::Solved| {
        let mut part: Vec<std::collections::BTreeSet<String>> = Vec::new();
        for w in &solved.solution.wires {
            let mut ends: std::collections::BTreeSet<String> = w
                .points
                .iter()
                .filter_map(|q| match q {
                    WirePoint::Anchor(a) => Some(format!("{a:?}")),
                    WirePoint::Branch(b) => Some(format!("branch {b}")),
                    WirePoint::Bend(_) => None,
                })
                .collect();
            part.retain(|s| {
                let apart = s.is_disjoint(&ends);
                if !apart {
                    ends.extend(s.iter().cloned());
                }
                apart
            });
            part.push(ends);
        }
        let mut sizes: Vec<usize> =
            part.iter().map(|s| s.iter().filter(|k| !k.starts_with("branch")).count()).collect();
        sizes.sort_unstable();
        sizes
    };
    // in rows, which route the trees as the line layout does in far less time; the skeleton of the
    // line layout is given back as the wires are (`join_tests` in the crate)
    let options = SolverOptions::default().with_config(r#"layout="rows", limit="seed""#).unwrap();
    let apart = solve(&p, &options);
    assert!(apart.report.unrouted.is_empty(), "{:?}", apart.report.unrouted);
    assert_eq!(parts(&apart), [2, 2, 2], "{:?}", apart.solution.wires);
    let joined = solve(&p, &options.with_config("join_trees=true").unwrap());
    assert!(joined.report.unrouted.is_empty(), "{:?}", joined.report.unrouted);
    assert_eq!(parts(&joined), [2, 4], "{:?}", joined.solution.wires);
    // each wire by the first tree of its net in the problem: 0 for n, 2 for m
    let mut routes: Vec<usize> = joined.solution.wires.iter().map(|w| w.route).collect();
    routes.sort_unstable();
    routes.dedup();
    assert_eq!(routes, [0, 2], "{:?}", joined.solution.wires);
    let note = "join_trees: the 3 trees of the ROUTEs routed as 2, one for each net";
    assert!(joined.report.notes.iter().any(|n| n == note), "{:?}", joined.report.notes);
    assert!(joined.report.canonical_options.as_deref().is_some_and(|c| c.contains(";join_trees=true")));
}

/// A level whose search is stopped while it tries a group both stretched in bands and dissolved,
/// between the two: the solutions the bands made are held in the trial, the best among them, and
/// were lost, so that the level panicked, its best not made at a node. They are kept, and the level
/// is drawn. Two groups of two rows, their rows aligned across in the crossing order, try both.
#[test]
fn a_search_stopped_between_bands_and_dissolved_keeps_what_the_bands_made() {
    let text = r#"* bands
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4") }
.subckt bands vdd vss x y
R1 vdd g0c0r1 1k
R2 g0c0r1 vss 1k
R3 vdd g0c1r1 1k
R4 g0c1r1 vss 1k
R5 g0c0r1 g0c1r1 1k
R6 vdd g1c0r1 1k
R7 g1c0r1 vss 1k
R8 vdd g1c1r1 1k
R9 g1c1r1 vss 1k
R10 g1c0r1 g1c1r1 1k
R11 g0c1r1 g1c0r1 1k
* @RNL(author="model") {
*   &vss @ADD-TO $GROUND_NET_SET;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   {#R1::p #R2::p} @ALIGN("x");
*   {#R1 #R2} @ORDER("top-to-bottom");
*   {#R3::p #R4::p} @ALIGN("x");
*   {#R3 #R4} @ORDER("top-to-bottom");
*   {#R1 #R3} @ALIGN("y");
*   {#R2 #R4} @ALIGN("y");
*   {#R1 #R3 #R2 #R4 #R5} @GROUP;
*   {#R6::p #R7::p} @ALIGN("x");
*   {#R6 #R7} @ORDER("top-to-bottom");
*   {#R8::p #R9::p} @ALIGN("x");
*   {#R8 #R9} @ORDER("top-to-bottom");
*   {#R6 #R8} @ALIGN("y");
*   {#R7 #R9} @ALIGN("y");
*   {#R6 #R8 #R7 #R9 #R10} @GROUP;
*   #R3 #R6 @DISTANCE("left-to-right", min=12);
*   {#R3 #R7} @ALIGN("y");
*   {#R4 #R6} @ALIGN("y");
*   {&g0c0r1 &g0c1r1 &g1c0r1 &g1c1r1} @ROUTE-NET;
* }
.ends
"#;
    let files = MemoryProvider::new().with("/w/bands.sp", text);
    let p = problems("/w/bands.sp", &files).into_iter().find(|p| p.title == "bands").unwrap();
    // just short of the end of the first batch of the bands: another solver moves it, and it is
    // found again as the work at which the level's first trial turns to the group dissolved, less one
    let options = SolverOptions::default().with_config(r#"limit="work", work=244596889398"#).unwrap();
    let solved = solve(&p, &options);
    let notes = solved.report.notes.join("\n");
    assert!(notes.contains("tried both stretched in bands and dissolved"), "{notes}");
    // stopped in the window: the group dissolved placed nothing, and the level has a batch of each
    // half only, where a stop after it has more
    assert!(notes.contains("in 2 batches over 2 settings of what is in it"), "{notes}");
    assert!(solved.report.errors.is_empty(), "{:?}", solved.report.errors);
    assert!(!solved.solution.placements.is_empty(), "{notes}");
}
