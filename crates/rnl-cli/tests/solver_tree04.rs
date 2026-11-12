//! Tree configuration, complete-file replay and persisted result identity.
use rnl_cli::solve::{solve_with_clock, SolveOptions};
use rnl_runtime::source::MemoryProvider;
use rnl_solver::{Clock, SolverOptions, StoppedClock};
use std::{cell::Cell, path::Path};

const SOURCE: &str = r#"* grouped layout
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(layout="tree") }
R1 shared a 1k
R2 shared b 2k
R3 shared c 3k
R4 shared d 4k
* @RNL {
* $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1,direction="left"); (2,0) @PIN(terminal=2,direction="right"); (-1,-1) (1,-1) (1,1) (-1,1) (-1,-1) @LINE };
* {#R1 #R2 #R3 #R4} @SET-SYMBOL $S;
* %pair = {#R1 #R2} @GROUP; %other = {#R3 #R4} @GROUP;
* {#R1 #R2} @ALIGN("y"); #R1 @ORIENT(["R0","MY"]);
* &shared @ROUTE-NET;
* }
.end
"#;

struct Ticks(Cell<u64>);
impl Clock for Ticks {
    fn elapsed(&self) -> f64 {
        let v = self.0.get();
        self.0.set(v + 1);
        v as f64
    }
}

fn options(config: &str) -> SolveOptions {
    SolveOptions { std: false, solver: SolverOptions::default().with_config(config).unwrap(), ..Default::default() }
}

#[test]
fn tree_file_setting_writes_a_result_that_reopens_solved() {
    let files = MemoryProvider::new().with("main.sp", SOURCE);
    let solved = solve_with_clock(Path::new("main.sp"), &files, &options("work=200000"), &StoppedClock).unwrap();
    assert_eq!(solved.errors, 0, "{solved:?}");
    assert!(solved.levels[0].solved.report.notes.iter().any(|n| n.contains("tree: the first arrangement")));
    let saved = MemoryProvider::new().with("main.sp", &solved.files[0].text);
    let build = rnl_view::BuildOptions {
        status: true,
        svg: false,
        runtime: rnl_runtime::Options { std: false, ..Default::default() },
        ..Default::default()
    };
    assert_eq!(rnl_view::build("main.sp", &saved, &build).unwrap().levels[0].solve, Some("solved"));
    let changed =
        MemoryProvider::new().with("main.sp", &solved.files[0].text.replace("layout=\"tree\"", "layout=\"rows\""));
    assert_eq!(rnl_view::build("main.sp", &changed, &build).unwrap().levels[0].solve, Some("stale"));
}

#[test]
fn tree_time_to_work_replays_the_entire_saved_file() {
    let files = MemoryProvider::new().with("main.sp", SOURCE);
    for seconds in [0, 20, 100, 400, 1500] {
        let timed = solve_with_clock(
            Path::new("main.sp"),
            &files,
            &options(&format!("seconds={seconds}")),
            &Ticks(Cell::new(0)),
        )
        .unwrap();
        assert_eq!(timed.errors, 0, "{timed:?}");
        let work = timed.levels[0].solved.report.budget.unwrap().work;
        let replay =
            solve_with_clock(Path::new("main.sp"), &files, &options(&format!("work={work}")), &StoppedClock).unwrap();
        assert_eq!(timed.files[0].text, replay.files[0].text, "cutoff {seconds}, work {work}");
        assert_eq!(timed.levels[0].hash, replay.levels[0].hash);
    }
}

#[test]
fn seed_with_no_objectives_finishes_without_repeating_singleton_wrappers() {
    let source = r#"* grouped layout
* @RNL(type="header") { @VERSION("0.4") }
R1 shared a 1k
R2 shared b 2k
* @RNL {
* $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1,direction="left"); (-1,-1) (1,1) @RECTANGLE };
* {#R1 #R2} @SET-SYMBOL $S;
* {#R1 #R2} @GROUP;
* &shared @ROUTE-NET;
* }
.end
"#;
    let files = MemoryProvider::new().with("main.sp", source);
    let solved = solve_with_clock(
        Path::new("main.sp"),
        &files,
        &options("layout=\"tree\",limit=\"seed\",objectives=\"none\",safety_seconds=10000"),
        &Ticks(Cell::new(0)),
    )
    .unwrap();
    assert_eq!(solved.errors, 0);
    let report = &solved.levels[0].solved.report;
    assert_eq!(report.budget.unwrap().reason, rnl_solver::StopReason::Complete, "{:?}", report.notes);
    assert!(report.notes.iter().any(|n| n.contains("none of the next ones better")), "{:?}", report.notes);
}

#[test]
fn an_explicit_rows_tool_setting_overrides_the_tree_file_setting() {
    let files = MemoryProvider::new().with("main.sp", SOURCE);
    let solved =
        solve_with_clock(Path::new("main.sp"), &files, &options("layout=\"rows\",work=200000"), &StoppedClock).unwrap();
    assert_eq!(solved.errors, 0);
    assert!(!solved.levels[0].solved.report.notes.iter().any(|n| n.starts_with("tree:")));
}
