use rnl_cli::{interpret_file, FsProvider};
use rnl_runtime::Options;
use rnl_solver::SolverOptions;
use std::{
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct InputFile(PathBuf);
impl InputFile {
    fn new(version: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rnl-protocol-{}-{}.sp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let config = if version == "0.4" { "; @SOLVER-CONFIG(spacing_x=2)" } else { "" };
        let source=format!("* input protocol\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"){config} }}\nR1 a b 1k\n* @RNL {{ $S = @DEFINE-SYMBOL {{(-1,-1) (1,1) @RECTANGLE; (-2,0) @PIN(terminal=1); (2,0) @PIN(terminal=2)}}; #R1 @SET-SYMBOL $S; #R1 @PLACE(0,0) }}\n.end\n");
        std::fs::write(&path, source).unwrap();
        Self(path)
    }
}
impl Drop for InputFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn opt_in_solver_input_keeps_original_models_and_explicit_zero_overrides() {
    let file = InputFile::new("0.4");
    let output = Command::new(env!("CARGO_BIN_EXE_rnl"))
        .args(["problem", "--no-std", "--solver-input", "--solver-config", "spacing_x=0,limit=\"work\",work=1000"])
        .arg(&file.0)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let input = rnl_solver_io::decode_input(&rnl_solver_io::parse_json(&output.stdout).unwrap()).unwrap();
    assert_eq!(input.levels.len(), 1);
    let level = &input.levels[0];
    assert_eq!(level.problem.solver_config.iter().find(|s| s.key == "spacing_x").unwrap().value.num(), Some(2.));
    assert_eq!(level.options.for_problem(&level.problem).0.settings.spacing, [0, 0]);
    assert_eq!(level.options.for_problem(&level.problem).0.run_limit, Some(rnl_solver::RunLimit::Work(1000)));
}

#[test]
fn readable_problem_output_keeps_its_original_bytes() {
    for version in ["0.2", "0.3", "0.4"] {
        let file = InputFile::new(version);
        let output =
            Command::new(env!("CARGO_BIN_EXE_rnl")).args(["problem", "--no-std"]).arg(&file.0).output().unwrap();
        assert!(output.status.success());
        let read =
            interpret_file(&file.0, None, &FsProvider::new(&[]), &Options { std: false, ..Options::solver() }).unwrap();
        let levels: Vec<_> = read
            .problems
            .iter()
            .map(|p| rnl_cli::problem::level(p, &SolverOptions::default().for_problem(p).0.canonical()))
            .collect();
        let expected = format!("{}\n", serde_json::to_string_pretty(&serde_json::json!({"levels":levels})).unwrap());
        assert_eq!(output.stdout, expected.as_bytes());
    }
}

#[test]
fn solver_input_refuses_source_errors_before_emitting_a_request() {
    let file = InputFile::new("0.4");
    let source = std::fs::read_to_string(&file.0).unwrap();
    std::fs::write(&file.0, source.replace("#R1 @PLACE(0,0)", "#R1 @ORIENT(\"invalid\")")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rnl"))
        .args(["problem", "--no-std", "--solver-input"])
        .arg(&file.0)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let read =
        interpret_file(&file.0, None, &FsProvider::new(&[]), &Options { std: false, ..Options::solver() }).unwrap();
    let diagnostics = read.diagnostics();
    let error = diagnostics.iter().find(|d| d.severity == rnl_runtime::diag::Severity::Error).unwrap();
    assert!(String::from_utf8_lossy(&output.stderr).contains(&read.sink.render(error)));
}

#[test]
fn solver_input_is_scoped_to_problem_and_does_not_discard_previous_hints_silently() {
    let file = InputFile::new("0.4");
    for args in [vec!["check", "--solver-input"], vec!["problem", "--solver-input", "--previous"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).arg(&file.0).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--solver-input"));
    }
}

#[test]
fn failed_external_process_never_writes_the_input() {
    let file = InputFile::new("0.4");
    let before = std::fs::read(&file.0).unwrap();
    for argument in ["--help", "--not-a-command"] {
        let output = Command::new(env!("CARGO_BIN_EXE_rnl"))
            .args([
                "solve",
                "--no-std",
                "--in-place",
                "--external-solver",
                env!("CARGO_BIN_EXE_rnl"),
                "--external-arg",
                argument,
            ])
            .arg(&file.0)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("wrote "));
        assert_eq!(std::fs::read(&file.0).unwrap(), before);
    }
}

#[test]
fn external_process_options_are_explicit_and_scoped() {
    let file = InputFile::new("0.4");
    for args in [
        vec!["check", "--external-solver", "unused"],
        vec!["solve", "--external-arg", "unused"],
        vec!["solve", "--external-timeout", "1"],
        vec!["solve", "--external-solver", "unused", "--external-timeout", "nan"],
        vec!["solve", "--external-solver", "unused", "--external-timeout", "-1"],
        vec!["solve", "--external-solver", "unused", "--profile", "unused.json"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).arg(&file.0).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn absent_external_report_does_not_claim_zero_facts_or_work() {
    use rnl_cli::solve::{report, solve_using, BackendResult, SolveOptions};
    let file = InputFile::new("0.4");
    let run = solve_using(
        &file.0,
        &FsProvider::new(&[]),
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &|_, _| {
            Ok(BackendResult { solved: Default::default(), author: "third-party 1".into(), report_available: false })
        },
    )
    .unwrap();
    let lines = report(&run.levels[0], true).join("\n");
    assert!(lines.contains("fact report not supplied"));
    assert!(!lines.contains("0 kept"));
    assert!(!lines.contains("work:"));
    assert!(!lines.contains("facts, in the order"));
}

#[test]
fn later_backend_failure_discards_earlier_in_memory_writebacks() {
    use rnl_cli::solve::{solve_using, BackendResult, SolveOptions};
    let file = InputFile::new("0.4");
    let source = std::fs::read_to_string(&file.0)
        .unwrap()
        .replace(".end\n", ".subckt child p n\nR2 p n 2k\n* @RNL { #R2 @PLACE(1,2) }\n.ends\n.end\n");
    std::fs::write(&file.0, &source).unwrap();
    let calls = std::cell::Cell::new(0);
    let options = SolveOptions {
        std: false,
        solver: SolverOptions { run_limit: Some(rnl_solver::RunLimit::Work(0)), ..Default::default() },
        ..Default::default()
    };
    let result = solve_using(&file.0, &FsProvider::new(&[]), &options, &|_| true, &|p, options| {
        calls.set(calls.get() + 1);
        if calls.get() == 2 {
            return Err("later response rejected".into());
        }
        Ok(BackendResult {
            solved: rnl_solver::solve(p, options),
            author: "third-party 1".into(),
            report_available: true,
        })
    });
    assert_eq!(calls.get(), 2);
    assert!(matches!(result, Err(e) if e == "later response rejected"));
    assert_eq!(std::fs::read_to_string(&file.0).unwrap(), source);
}
