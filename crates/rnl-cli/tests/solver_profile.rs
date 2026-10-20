//! Profiling is separate from written solver records and remains readable during a solve.
use rnl_cli::profile::ProfileClock;
use rnl_cli::solve::{solve_with_clock, SolveOptions};
use rnl_runtime::source::MemoryProvider;
use rnl_solver::StoppedClock;
use std::path::Path;

#[test]
fn profile_json_and_records_are_independent() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("solver-profile");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("complete.json");
    let provider = MemoryProvider::default().with(
        "test.sp",
        ".title Profile\n* @VERSION(\"0.3\")\nR1 a b 1k\nR2 b 0 2k\n* #R1 @PLACE(0,0)\n* #R2 @PLACE(10,0)\n.end\n",
    );
    let options = SolveOptions { std: false, ..SolveOptions::default() };
    let plain = solve_with_clock(Path::new("test.sp"), &provider, &options, &StoppedClock).unwrap();
    let _ = std::fs::remove_file(&path);
    let clock = ProfileClock::new(&path).unwrap();
    // A durable valid partial exists before any solver work starts.
    let initial: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(initial["status"], "running");
    assert_eq!(initial["counts_complete"], false);
    let measured = solve_with_clock(Path::new("test.sp"), &provider, &options, &clock).unwrap();
    clock.finish(true).unwrap();
    assert_eq!(plain.files, measured.files);
    assert!(!plain.levels.is_empty());
    for (a, b) in plain.levels.iter().zip(&measured.levels) {
        assert_eq!(a.hash, b.hash);
        assert_eq!(a.record.as_bytes(), b.record.as_bytes());
        assert_eq!(a.solved, b.solved);
    }
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(json["status"], "complete");
    assert_eq!(json["counts_complete"], true);
    assert_eq!(json["schema_version"], 1);
    assert!(json.get("work_diagnostics_schema_version").is_none());
    assert!(json.get("tableau_details").is_none());
    assert!(json["levels"].as_array().unwrap().iter().all(|l| l["status"] == "complete"));
}

#[test]
fn cli_profile_paths_cannot_overwrite_inputs_or_solved_outputs() {
    use std::process::Command;
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("profile-paths-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("input.sp");
    let include = dir.join("included.sp");
    let text = ".title Profile\n* @VERSION(\"0.3\")\n.subckt top a b\n.include included.sp\n.ends top\n.end\n";
    std::fs::write(&input, text).unwrap();
    std::fs::write(&include, "* include must survive\n").unwrap();
    let before = std::fs::read(&include).unwrap();
    let out = dir.join("out");
    let result = Command::new(env!("CARGO_BIN_EXE_rnl"))
        .arg("solve")
        .arg(&input)
        .arg("--no-std")
        .arg("--level")
        .arg("top")
        .arg("-o")
        .arg(&out)
        .arg("--profile")
        .arg(&include)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("cannot create new profile"));
    assert_eq!(std::fs::read(&include).unwrap(), before);
    std::fs::create_dir_all(&out).unwrap();
    let collision = out.join("input.sp");
    let _ = std::fs::remove_file(&collision);
    let result = Command::new(env!("CARGO_BIN_EXE_rnl"))
        .arg("solve")
        .arg(&input)
        .arg("--no-std")
        .arg("--level")
        .arg("top")
        .arg("-o")
        .arg(&out)
        .arg("--profile")
        .arg(&collision)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("conflicts with a solved output"),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let snapshot: serde_json::Value = serde_json::from_slice(&std::fs::read(&collision).unwrap()).unwrap();
    assert_eq!(snapshot["schema_version"], 1);
    let copied_collision = out.join("included.sp");
    let _ = std::fs::remove_file(&copied_collision);
    let result = Command::new(env!("CARGO_BIN_EXE_rnl"))
        .arg("solve")
        .arg(&input)
        .arg("--no-std")
        .arg("--level")
        .arg("top")
        .arg("-o")
        .arg(&out)
        .arg("--profile")
        .arg(&copied_collision)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("conflicts with a solved output"),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read(&include).unwrap(), before);
    let _: serde_json::Value = serde_json::from_slice(&std::fs::read(&copied_collision).unwrap()).unwrap();
    assert_eq!(std::fs::read(&input).unwrap(), text.as_bytes());
}

#[test]
fn disabled_profile_keeps_committed_t1_bytes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/integration/solver");
    let options = SolveOptions { solver: rnl_solver::SolverOptions::without_time_limit(), ..SolveOptions::default() };
    let run = solve_with_clock(&root.join("T1-rc-lowpass.sp"), &rnl_cli::FsProvider::new(&[]), &options, &StoppedClock)
        .unwrap();
    let expected = std::fs::read(root.join("solved/T1-rc-lowpass.sp")).unwrap();
    assert_eq!(run.files.len(), 1);
    assert_eq!(run.files[0].text.as_bytes(), expected);
}

#[test]
fn work_profile_is_opt_in_and_preserves_files_records_and_options() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("solver-work-profile");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("work.json");
    let _ = std::fs::remove_file(&path);
    let provider = MemoryProvider::default()
        .with("test.sp", ".title Work\n* @VERSION(\"0.3\")\nR1 a b 1k\nR2 b 0 2k\n* #R1 @PLACE(0,0)\n.end\n");
    let options = SolveOptions { std: false, ..SolveOptions::default() };
    let plain = solve_with_clock(Path::new("test.sp"), &provider, &options, &StoppedClock).unwrap();
    let clock = ProfileClock::new_work(&path).unwrap();
    let measured = solve_with_clock(Path::new("test.sp"), &provider, &options, &clock).unwrap();
    clock.finish(true).unwrap();
    assert_eq!(plain.files, measured.files);
    assert!(!plain.levels.is_empty());
    assert_eq!(plain.levels.len(), measured.levels.len());
    for (a, b) in plain.levels.iter().zip(&measured.levels) {
        assert_eq!(a.hash, b.hash);
        assert_eq!(a.record, b.record);
        assert_eq!(a.solved, b.solved);
    }
    let data: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(data["schema_version"], 1);
    assert_eq!(data["work_diagnostics_schema_version"], 1);
    assert_eq!(data["diagnostic_mode"], "work");
    assert_eq!(data["tableau_details"]["status"], "not_observed");
    let level = &data["levels"][0];
    assert!(level["metrics"]["work_tableaux_created"].as_u64().unwrap() > 0);
    assert!(level["metrics"].get("max_numerator_bits").is_none());
    assert!(level["metrics"].get("tableau_scans").is_none());
    assert!(level["spans"].as_array().unwrap().iter().any(|s| s["category"] == "with_relations"));
    assert_eq!(level["metrics"]["work_counter_overflow"], 0);
}

#[test]
fn cli_work_profile_requires_an_explicit_profile() {
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_rnl"))
        .args(["solve", "unused.sp", "-o", "unused-output", "--profile-work"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--profile-work requires solve with --profile"));
}
