//! A solve that cannot write a level reports an error, without duplicating host errors.
use rnl_cli::solve::{errors, lines, solve, SolveOptions, WallClock};
use rnl_runtime::{
    diag::{codes, Severity},
    source::MemoryProvider,
};
use rnl_solver::SolverOptions;
use std::path::Path;

fn options() -> SolveOptions {
    SolveOptions { std: false, solver: SolverOptions::without_time_limit(), ..Default::default() }
}
fn header(version: &str) -> String {
    format!("* source\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n")
}
fn providers(top_version: &str, lib_version: &str, open: &str, close: &str) -> MemoryProvider {
    MemoryProvider::new()
        .with("main.sp", &format!("{}.include lib.inc\nX1 a b cell\n", header(top_version)))
        .with("lib.inc", &format!("{}{open}\n.subckt cell a b\nR1 a b 1k\n.ends cell\n{close}\n", header(lib_version)))
}

#[test]
fn encrypted_and_excluded_levels_are_errors_with_locations() {
    for (open, close, code) in
        [(".prot", ".unprot", codes::WRITEBACK_REGION_ENCRYPTED), (".if 1", ".endif", codes::WRITEBACK_REGION_EXCLUDED)]
    {
        let files = providers("0.4", "0.4", open, close);
        let run = solve(Path::new("main.sp"), &files, &options()).unwrap();
        let level = run.levels.iter().find(|l| l.problem.title == "cell").unwrap();
        assert_eq!(level.write_errors.len(), 1, "{:?}", lines(&run, true));
        let diagnostic = &level.write_errors[0];
        assert_eq!(diagnostic.code, code);
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.message.contains("cell"));
        assert_eq!(diagnostic.loc.as_ref().unwrap().line, 4);
        assert!(diagnostic.loc.as_ref().unwrap().file.ends_with("lib.inc"));
        assert!(level.written.notes.iter().all(|n| !n.contains("the record is not written")));
        assert_eq!(errors(&run), 1);
        assert!(!run.files.iter().any(|f| f.path.ends_with("lib.inc")));
        assert!(lines(&run, false)
            .iter()
            .any(|l| l.contains(&code.to_string()) && l.trim_start().starts_with("error ")));
    }
}

#[test]
fn an_included_file_without_a_header_refuses_as_the_including_file() {
    let files = MemoryProvider::new()
        .with("main.sp", &format!("{}.include lib.inc\nX1 a b cell\n", header("0.4")))
        .with("lib.inc", "* source\n.prot\n.subckt cell a b\nR1 a b 1k\n.ends cell\n.unprot\n");
    let run = solve(Path::new("main.sp"), &files, &options()).unwrap();
    let level = run.levels.iter().find(|l| l.problem.title == "cell").unwrap();
    assert_eq!(level.write_errors.len(), 1, "{:?}", lines(&run, true));
    assert!(level.written.notes.iter().all(|n| !n.contains("the record is not written")));
    assert_eq!(errors(&run), 1);
}

#[test]
fn the_preview_includes_write_refusals_in_its_errors() {
    let shown = rnl_cli::preview::preview(
        "main.sp",
        &providers("0.4", "0.4", ".prot", ".unprot"),
        &options(),
        &WallClock::new(),
        &|_| true,
    )
    .unwrap();
    assert_eq!(shown.report.errors, 1);
    let level = shown.report.levels.iter().find(|l| l.title == "cell").unwrap();
    assert_eq!(level.errors.len(), 1);
    assert!(level.errors[0].contains("RNL8400"));
    assert!(!level.notes.iter().any(|n| n.contains("the record is not written")));
}

#[test]
fn a_missing_section_keeps_the_host_error_only() {
    let files = MemoryProvider::new()
        .with("main.sp", &format!("{}.lib lib.inc missing\n", header("0.4")))
        .with("lib.inc", &format!("{}.lib present\nR1 a b 1k\n.endl present\n", header("0.4")));
    let run = solve(Path::new("main.sp"), &files, &options()).unwrap();
    assert_eq!(run.errors, 1, "{:?}", lines(&run, true));
    assert_eq!(errors(&run), 0);
    assert!(!lines(&run, true).iter().any(|line| line.contains("the selected library section")));
}

#[test]
fn a_record_outside_the_encrypted_tail_is_still_written() {
    let files = MemoryProvider::new().with(
        "main.sp",
        &format!("{}.subckt cell a b\nR1 a b 1k\n.prot\n.ends cell\n.unprot\nX1 a b cell\n", header("0.4")),
    );
    let run = solve(Path::new("main.sp"), &files, &options()).unwrap();
    assert_eq!(errors(&run), 0);
    assert!(run.levels.iter().all(|l| l.write_errors.is_empty()));
    assert!(run.files[0].text.contains("type=\"generated\""));
}

#[test]
fn a_cli_solve_exits_with_failure_for_an_unwritable_level() {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("write-refusal04");
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join("main.sp");
    let text = format!("{}.prot\n.subckt cell a b\nR1 a b 1k\n.ends cell\n.unprot\nX1 a b cell\n", header("0.4"));
    std::fs::write(&file, &text).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rnl"))
        .arg("solve")
        .arg(&file)
        .args(["--no-std", "--report", "-o"])
        .arg(folder.join("out"))
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(combined.contains("RNL8400"), "{combined}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
}
