//! Chosen wire geometry survives actual 0.4 writeback and budget replay.
use rnl_cli::solve::{solve_with_clock, SolveOptions};
use rnl_runtime::{solution::WirePoint, source::MemoryProvider};
use rnl_solver::{Clock, SolverOptions, StoppedClock};
use std::{cell::Cell, path::Path};

const SOURCE: &str = r#"* authored wire
* @RNL(type="header") { @VERSION("0.4") }
R1 a b 1k
R2 a c 2k
* @RNL {
* $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1,direction="up"); (-1,-1) (1,-1) (1,1) (-1,1) (-1,-1) @LINE };
* {#R1 #R2} @SET-SYMBOL $S => @ORIENT("R0");
* #R1 @PLACE(0,0); #R2 @PLACE(4,2);
* %w = &a @WIRE(mode="auto") {#R1:1 #R2:1};
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
fn authored_auto_wire_is_written_with_its_core_path_and_reopens_solved() {
    let files = MemoryProvider::new().with("main.sp", SOURCE);
    let result = solve_with_clock(Path::new("main.sp"), &files, &options("limit=\"seed\""), &StoppedClock).unwrap();
    assert_eq!(result.errors, 0, "{result:?}");
    let level = &result.levels[0];
    assert_eq!(level.solved.solution.wire_routes.len(), 1);
    assert_eq!(
        level.solved.solution.wire_routes[0].points,
        vec![
            WirePoint::Anchor(level.problem.wires[0].anchors[0].clone()),
            WirePoint::Bend((0., 3.)),
            WirePoint::Bend((4., 3.)),
            WirePoint::Anchor(level.problem.wires[0].anchors[1].clone()),
        ],
    );
    let saved = MemoryProvider::new().with("main.sp", &result.files[0].text);
    assert!(level.written.unsolved.is_empty(), "{:?}", level.written.unsolved);
    let back = rnl_cli::interpret_file(
        Path::new("main.sp"),
        None,
        &saved,
        &rnl_runtime::Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap();
    let coordinates = rnl_runtime::previous::drawn(&level.problem, &back);
    for &f in &level.solved.report.kept {
        assert_eq!(rnl_solver::problem_fact_holds_as_drawn(&level.problem, &coordinates, f, 1e-6), Some(true));
    }
    let build = rnl_view::BuildOptions {
        status: true,
        svg: false,
        runtime: rnl_runtime::Options { std: false, ..Default::default() },
        ..Default::default()
    };
    let drawing = rnl_view::build("main.sp", &saved, &build).unwrap();
    assert_eq!(drawing.levels[0].solve, Some("solved"), "{}", result.files[0].text);
}

#[test]
fn auto_wire_writeback_is_identical_after_time_to_work_replay() {
    let files = MemoryProvider::new().with("main.sp", SOURCE);
    for seconds in [0, 20, 100, 1000] {
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
        assert_eq!(timed.files[0].text, replay.files[0].text, "cutoff {seconds}");
    }
}
