use rnl_runtime::{diag::Severity, source::MemoryProvider, Options};
use rnl_solver::{RunLimit, SolverOptions};
use rnl_solver_io::{InputDocument, LevelInput};
use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

fn input(options: SolverOptions) -> InputDocument {
    let source="* standalone\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a n 1k\nR2 n b 1k\n* @RNL { $S = @DEFINE-SYMBOL { (-1,-1) (1,1) @RECTANGLE; (-2,0) @PIN(terminal=1); (2,0) @PIN(terminal=2) }; {#R1 #R2} @SET-SYMBOL $S; &n @ROUTE-NET }\n.end\n";
    let files = MemoryProvider::new().with("input.sp", source);
    let read = rnl_view::interpret("input.sp", None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(read.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", read.diagnostics());
    InputDocument { levels: vec![LevelInput { problem: read.problems[0].clone(), options }] }
}
fn process(bytes: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rnl-solver"))
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    child.wait_with_output().unwrap()
}
fn run(input: &InputDocument) -> Output {
    process(&serde_json::to_vec(&rnl_solver_io::encode_input(input).unwrap()).unwrap())
}

#[test]
fn independent_version_and_help_are_not_the_workspace_version() {
    let version = Command::new(env!("CARGO_BIN_EXE_rnl-solver")).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert_eq!(String::from_utf8(version.stdout).unwrap(), "rnl-solver 0.2.0\n");
    let help = Command::new(env!("CARGO_BIN_EXE_rnl-solver")).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout).unwrap().contains("--solver-input"));
}

#[test]
fn subprocess_solution_matches_the_embedded_library() {
    let request = input(SolverOptions {
        run_limit: Some(RunLimit::Work(20000)),
        safety_seconds: None,
        ..SolverOptions::without_time_limit()
    });
    let output = run(&request);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let result = rnl_solver_io::decode_result(
        &rnl_solver_io::parse_json(&output.stdout).unwrap(),
        &[request.levels[0].problem.clone()],
    )
    .unwrap();
    let native = rnl_solver::solve(&request.levels[0].problem, &request.levels[0].options);
    assert_eq!(result.levels[0].solution, native.solution);
    assert_eq!(result.levels[0].options, native.report.canonical_options.unwrap());
    let report = result.levels[0].report.as_ref().unwrap();
    assert_eq!(report.kept, native.report.kept);
    assert_eq!(report.discarded, native.report.discarded);
    assert_eq!(report.work, native.report.work);
}

#[test]
fn time_result_replays_as_identical_stdout_bytes() {
    for seconds in [0., 0.002, 0.02] {
        let mut request = input(SolverOptions {
            run_limit: Some(RunLimit::Time(seconds)),
            safety_seconds: None,
            ..SolverOptions::without_time_limit()
        });
        let timed = run(&request);
        assert!(timed.status.success(), "{}", String::from_utf8_lossy(&timed.stderr));
        let result = rnl_solver_io::decode_result(
            &rnl_solver_io::parse_json(&timed.stdout).unwrap(),
            &[request.levels[0].problem.clone()],
        )
        .unwrap();
        let work = result.levels[0].report.as_ref().unwrap().budget.unwrap().work;
        request.levels[0].options.run_limit = Some(RunLimit::Work(work));
        let replay = run(&request);
        assert!(replay.status.success(), "{}", String::from_utf8_lossy(&replay.stderr));
        assert_eq!(timed.stdout, replay.stdout, "cutoff={seconds},work={work}");
    }
}

#[test]
fn malformed_requests_produce_no_result_on_stdout() {
    for bytes in [b"not json".as_slice(), br#"{"format":"rnl-solver-input","version":999,"levels":[]}"#] {
        let output = process(bytes);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn deep_runtime_literals_are_read_on_the_executable_main_stack() {
    use rnl_runtime::{interp::obj::Lit, problem::SolverSetting};
    let mut request = input(SolverOptions {
        run_limit: Some(RunLimit::Work(0)),
        safety_seconds: None,
        ..SolverOptions::without_time_limit()
    });
    let mut value = Lit::Num(1.25);
    for _ in 0..64 {
        value = Lit::Arr(vec![value]);
    }
    request.levels[0].problem.solver_config.push(SolverSetting {
        key: "fixture_depth".into(),
        value,
        source: None,
        package: String::new(),
    });
    let output = run(&request);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    rnl_solver_io::decode_result(
        &rnl_solver_io::parse_json(&output.stdout).unwrap(),
        &[request.levels[0].problem.clone()],
    )
    .unwrap();
}

#[test]
fn external_backend_writes_and_reopens_the_same_time_work_bytes() {
    use rnl_cli::solve::{solve_using, SolveOptions};
    use std::path::Path;
    let source = "* external\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a b 1k\n* @RNL { $S = @DEFINE-SYMBOL { (-1,-1) (1,1) @RECTANGLE }; #R1 @SET-SYMBOL $S; #R1 @PLACE(2,4) }\n.end\n";
    let files = MemoryProvider::new().with("input.sp", source);
    let options = SolveOptions {
        std: false,
        solver: SolverOptions {
            run_limit: Some(RunLimit::Time(0.02)),
            safety_seconds: None,
            ..SolverOptions::without_time_limit()
        },
        ..Default::default()
    };
    let backend = |problem: &_, options: &_| {
        rnl_cli::external_solver::solve(Path::new(env!("CARGO_BIN_EXE_rnl-solver")), &[], 30., problem, options)
    };
    let first = solve_using(Path::new("input.sp"), &files, &options, &|_| true, &backend).unwrap();
    assert_eq!(first.levels.len(), 1);
    assert!(first.levels[0].report_available);
    assert!(!first.levels[0].solved.report.kept.is_empty());
    let work = first.levels[0].solved.report.budget.unwrap().work;
    let options =
        SolveOptions { solver: SolverOptions { run_limit: Some(RunLimit::Work(work)), ..options.solver }, ..options };
    let replay = solve_using(Path::new("input.sp"), &files, &options, &|_| true, &backend).unwrap();
    assert_eq!(first.files.len(), 1);
    assert_eq!(first.files[0].text.as_bytes(), replay.files[0].text.as_bytes());
    let reread = rnl_view::interpret(
        "input.sp",
        None,
        &MemoryProvider::new().with("input.sp", &first.files[0].text),
        &Options { std: false, ..Options::default() },
    )
    .unwrap();
    assert!(reread.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", reread.diagnostics());
    assert!(first.files[0].text.contains("rnl-solver 0.2.0"));
}

#[test]
fn external_backend_rejects_process_failures_invalid_results_and_timeout() {
    use std::path::Path;
    let request = input(SolverOptions {
        run_limit: Some(RunLimit::Work(20000)),
        safety_seconds: None,
        ..SolverOptions::without_time_limit()
    });
    let level = &request.levels[0];
    for (args, seconds, message) in [
        (vec!["--unknown".to_string()], 30., "process exited"),
        (vec!["--version".to_string()], 30., "JSON"),
        (vec![], 0., "process exceeded"),
    ] {
        let error = match rnl_cli::external_solver::solve(
            Path::new(env!("CARGO_BIN_EXE_rnl-solver")),
            &args,
            seconds,
            &level.problem,
            &level.options,
        ) {
            Ok(_) => panic!("unexpected success"),
            Err(e) => e,
        };
        assert!(error.contains(message), "{error}");
    }
}
