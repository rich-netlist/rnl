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

#[test]
fn line_results_complete_configured_junctions_and_replay_the_written_file() {
    let original = include_str!("junction_completion_fixtures/no-existing-dot.sp").replace(
        "*   %p = (0,0);",
        "*   #R1 @TEXT(\"label\", key=\"note\", dx=0.1, dy=0.2);\n*   $EMPTY = @DEFINE(roles=[]) {}; $EMPTY @ATTACH => @SET-SYMBOL $DOT_SYMBOL;\n*   %p = (0,0);",
    );
    let original = original.replace(
        "*   %p = (0,0);",
        "*   $GEN = @DEFINE(roles=[\"r\"],parameters=[[\"dy\",2],[\"direction\",\"left-to-right\"]]) {};\n*   $WRAP = @DEFINE(roles=[\"p\"]) { %items = # @SELECT-INSTANCE => @AS(dy=2) $GEN; %items @ALIGN(\"x\") };\n*   #R3 @AS $WRAP; # @SELECT-USAGE $GEN => @PLACE(8,9);\n*   %p = (0,0);",
    );
    let original = original.replace(
        "*   %p = (0,0);",
        "*   {#R1 #R2} @ORIENT(\"R0\"); #R1 #R2 @ORIENT-RELATION(\"R90\",priority=\"low\");\n*   %p = (0,0);",
    );
    for offset in [0, 2] {
        let source = if offset == 0 {
            original.to_string()
        } else {
            original.replace(
                "{ {? ?p} @ALIGN(\"x\"); {? ?p} @ALIGN(\"y\") }",
                "{ ?p ? @DISTANCE(\"left-to-right\", exact=2); {? ?p} @ALIGN(\"y\") }",
            )
        };
        let files = MemoryProvider::new().with("main.sp", &source);
        let seed =
            solve_with_clock(Path::new("main.sp"), &files, &options("layout=\"lines\",limit=\"seed\""), &StoppedClock)
                .unwrap();
        assert_eq!(seed.errors, 0, "{seed:?}");
        assert!(seed.levels[0].solved.report.skeleton.is_some());
        let file = seed.files.iter().find(|file| file.path == "main.sp").unwrap();
        assert!(file.text.contains("numbering=\""), "{}", file.text);
        assert!(file.text.contains(&format!("=> @PLACE({offset}, 0)")), "{}", file.text);
        let saved = MemoryProvider::new().with("main.sp", &file.text);
        let read = rnl_cli::interpret_file(
            Path::new("main.sp"),
            None,
            &saved,
            &rnl_runtime::Options { std: false, generated: true, problems: true, ..Default::default() },
        )
        .unwrap();
        assert!(read.model.levels[0].unplaced.is_empty(), "{:?}", read.model.levels[0].unplaced);
        assert!(read.model.levels[0].constraints.iter().any(|c| c.status == "dropped" && c.priority == "low"));
        let level = &seed.levels[0];
        let line_map: Vec<_> = (0..=source.lines().count() as u32).map(Some).collect();
        let check_retained = |trial: &rnl_runtime::problem::LevelProblem| {
            rnl_cli::junction_completion_problem::retained_facts_preserved(
                &level.problem,
                trial,
                "main.sp",
                &line_map,
                &level.solved.report.kept,
                &read,
                1e-6,
            )
        };
        assert!(!level.solved.report.kept.is_empty());
        assert!(check_retained(&read.problems[0]).is_ok());
        let mut changed = read.problems[0].clone();
        let usage = changed.usages.iter_mut().find(|u| u.definition == "$GEN").unwrap();
        usage.roles[0].members = vec![rnl_runtime::problem::Filler::Net("different".into())];
        assert!(check_retained(&changed).is_err(), "reified roles must retain their identity");
        let mut changed_parameter = read.problems[0].clone();
        let usage = changed_parameter.usages.iter_mut().find(|u| u.definition == "$GEN").unwrap();
        usage.params[0].1 = rnl_runtime::interp::obj::Lit::Num(999.);
        assert!(check_retained(&changed_parameter).is_err(), "reified parameter values must be preserved");
        let mut contradicted = level.problem.clone();
        contradicted.facts[level.solved.report.kept[0]].kind = rnl_runtime::problem::FactKind::PointCoordinate {
            point: 0,
            axis: rnl_runtime::problem::Axis::X,
            value: 12345.,
        };
        assert!(
            rnl_cli::junction_completion_problem::retained_facts_preserved(
                &contradicted,
                &read.problems[0],
                "main.sp",
                &line_map,
                &level.solved.report.kept,
                &read,
                1e-6,
            )
            .is_err(),
            "a claimed kept fact must actually hold in the drawing"
        );
        for seconds in [2000, 100000] {
            let timed = solve_with_clock(
                Path::new("main.sp"),
                &files,
                &options(&format!("layout=\"lines\",limit=\"time\",seconds={seconds}")),
                &Ticks(Cell::new(0)),
            )
            .unwrap();
            let work = timed.levels[0].solved.report.budget.unwrap().work;
            let replay = solve_with_clock(
                Path::new("main.sp"),
                &files,
                &options(&format!("layout=\"lines\",limit=\"work\",work={work}")),
                &StoppedClock,
            )
            .unwrap();
            assert_eq!(timed.files[0].text, replay.files[0].text, "offset {offset}, cutoff {seconds}");
        }
    }
}
