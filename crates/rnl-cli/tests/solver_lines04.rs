//! Public line-layout selection, persisted identity, and whole-file replay.
use rnl_cli::solve::{solve_with_clock, SolveOptions};
use rnl_runtime::source::MemoryProvider;
use rnl_solver::{Clock, SolverOptions, StoppedClock};
use std::{cell::Cell, path::Path};

const SOURCE: &str = r#"* line layout
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(layout="lines",limit="seed") }
R1 a shared 1k
R2 shared b 2k
* @RNL {
* $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1,direction="left"); (2,0) @PIN(terminal=2,direction="right"); (-1,-1) (1,1) @RECTANGLE };
* {#R1 #R2} @SET-SYMBOL $S;
* {#R1 #R2} @GROUP;
* &shared @ROUTE-NET;
* }
.end
"#;

fn options(config: &str) -> SolveOptions {
    SolveOptions { std: false, solver: SolverOptions::default().with_config(config).unwrap(), ..Default::default() }
}
struct Ticks(Cell<u64>);
impl Clock for Ticks {
    fn elapsed(&self) -> f64 {
        let t = self.0.get();
        self.0.set(t + 1);
        t as f64
    }
}

#[test]
fn lines_is_opt_in_and_its_seed_result_reopens_solved() {
    assert_eq!(SolverOptions::default().settings.layout, rnl_solver::Layout::Rows);
    let files = MemoryProvider::new().with("main.sp", SOURCE);
    let result = solve_with_clock(Path::new("main.sp"), &files, &options(""), &StoppedClock).unwrap();
    assert_eq!(result.errors, 0, "{result:?}");
    let report = &result.levels[0].solved.report;
    let skeleton = report.skeleton.as_ref().expect("line layout supplies its description");
    assert!(!skeleton.tables.is_empty());
    assert!(!skeleton.lines.is_empty());
    assert!(report.canonical_options.as_ref().unwrap().contains("run=seed"));
    assert!(report.notes.iter().any(|n| n.starts_with("lines, round 0:")), "{:?}", report.notes);
    let saved = MemoryProvider::new().with("main.sp", &result.files[0].text);
    let build = rnl_view::BuildOptions {
        status: true,
        svg: false,
        runtime: rnl_runtime::Options { std: false, ..Default::default() },
        ..Default::default()
    };
    assert_eq!(rnl_view::build("main.sp", &saved, &build).unwrap().levels[0].solve, Some("solved"));
}

#[test]
fn seed_finishes_with_two_single_pin_symbols_without_repeating_wrappers() {
    let source =
        SOURCE.replace("R1 a shared 1k", "R1 shared a 1k").replace("(2,0) @PIN(terminal=2,direction=\"right\"); ", "");
    let files = MemoryProvider::new().with("main.sp", &source);
    let result = solve_with_clock(
        Path::new("main.sp"),
        &files,
        &options("limit=\"seed\",safety_seconds=100000"),
        &Ticks(Cell::new(0)),
    )
    .unwrap();
    let report = &result.levels[0].solved.report;
    assert_eq!(report.budget.unwrap().reason, rnl_solver::StopReason::Complete, "{:?}", report.notes);
    assert!(report.notes.iter().any(|n| n.starts_with("lines, round 0:")), "{:?}", report.notes);
}

#[test]
fn line_time_cutoffs_replay_the_complete_file_and_skeleton_report() {
    let files = MemoryProvider::new().with("main.sp", SOURCE);
    for seconds in [0, 100, 1000, 5000, 15000] {
        let timed = solve_with_clock(
            Path::new("main.sp"),
            &files,
            &options(&format!("limit=\"time\",seconds={seconds}")),
            &Ticks(Cell::new(0)),
        )
        .unwrap();
        assert_eq!(timed.errors, 0, "{timed:?}");
        let work = timed.levels[0].solved.report.budget.unwrap().work;
        let replay = solve_with_clock(
            Path::new("main.sp"),
            &files,
            &options(&format!("limit=\"work\",work={work}")),
            &StoppedClock,
        )
        .unwrap();
        assert_eq!(
            timed.levels[0].solved.report.canonical_options, replay.levels[0].solved.report.canonical_options,
            "cutoff {seconds}; budgets {:?} / {:?}",
            timed.levels[0].solved.report.budget, replay.levels[0].solved.report.budget
        );
        assert_eq!(timed.files[0].text, replay.files[0].text, "cutoff {seconds}, work {work}");
        assert_eq!(timed.levels[0].hash, replay.levels[0].hash);
        assert_eq!(timed.levels[0].solved.report.skeleton, replay.levels[0].solved.report.skeleton);
    }
}
