use rnl_cli::solve::{solve_using, BackendResult, SolveOptions};
use rnl_runtime::source::{MemoryProvider, Provider};
use rnl_solver::Solved;
use std::path::Path;

#[test]
fn fatal_geometry_preserves_existing_record_and_file() {
    let text = "* writeback fixture\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a b 1k\n* @RNL { &a @WIRE(mode=\"straight\") { (0,0) (1,0) } }\n* @RNL(type=\"generated\", author=\"test\") { #R1 @PLACE(9,9) }\n.end\n";
    let files = MemoryProvider::new().with("fatal.sp", text);
    let backend = |_: &rnl_runtime::problem::LevelProblem, _: &rnl_solver::SolverOptions| {
        let mut solved = Solved::default();
        solved.report.fatal_written_wire = Some(0);
        solved.report.errors.push("wire geometry failed".into());
        Ok(BackendResult { solved, author: "test".into(), report_available: true })
    };
    let run = solve_using(
        Path::new("fatal.sp"),
        &files,
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &backend,
    )
    .unwrap();
    assert!(!run.levels.is_empty());
    assert!(run.files.is_empty(), "fatal level must not create an edit or remove its existing record");
    assert_eq!(files.read("fatal.sp").unwrap(), text);
}

#[test]
fn ordinary_report_errors_keep_existing_writeback_behavior() {
    let text =
        "* writeback fixture\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a b 1k\n* @RNL { %p=(_,_) }\n.end\n";
    let files = MemoryProvider::new().with("ordinary.sp", text);
    let backend = |problem: &rnl_runtime::problem::LevelProblem, _: &rnl_solver::SolverOptions| {
        let mut solved = Solved::default();
        let object = problem
            .objects
            .iter()
            .position(|o| o.reference == rnl_runtime::problem::Reference::Name("%p".into()))
            .unwrap();
        solved.solution.placements.push(rnl_runtime::solution::Placement {
            object,
            x: Some(2.0),
            y: Some(3.0),
            orient: None,
        });
        solved.report.errors.push("ordinary fact conflict".into());
        Ok(BackendResult { solved, author: "test".into(), report_available: true })
    };
    let run = solve_using(
        Path::new("ordinary.sp"),
        &files,
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &backend,
    )
    .unwrap();
    assert!(!run.files.is_empty());
}

#[test]
fn fatal_write_in_place_leaves_existing_bytes_untouched() {
    use rnl_cli::solve::{write, Target};
    use rnl_cli::FsProvider;
    let dir = std::env::temp_dir().join(format!("rnl-auto-fatal-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("existing.sp");
    let text = "* writeback fixture\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a b 1k\n* @RNL { &a @WIRE(mode=\"straight\") { (0,0) (1,0) } }\n* @RNL(type=\"generated\", author=\"test\") { #R1 @PLACE(9,9) }\n.end\n";
    std::fs::write(&path, text).unwrap();
    let backend = |_: &rnl_runtime::problem::LevelProblem, _: &rnl_solver::SolverOptions| {
        let mut solved = Solved::default();
        solved.report.fatal_written_wire = Some(0);
        solved.report.errors.push("wire geometry failed".into());
        Ok(BackendResult { solved, author: "test".into(), report_available: true })
    };
    let run = solve_using(
        &path,
        &FsProvider::new(&[]),
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &backend,
    )
    .unwrap();
    let outcome = write(&run, &Target::InPlace);
    assert!(outcome.error.is_none());
    assert!(outcome.written.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), text.as_bytes());
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(&dir).unwrap();
}
