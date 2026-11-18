//! Profiling is separate from written solver records and remains readable during a solve.
use rnl_cli::profile::ProfileClock;
use rnl_cli::solve::{solve_with_clock, SolveOptions};
use rnl_runtime::source::MemoryProvider;
use rnl_solver::StoppedClock;
use std::path::Path;

/// The options of a solve within a fixed work and with no time or safety limit: a level of 0.4
/// is solved the same on every run, whatever the clock (17).
fn within_work() -> SolveOptions {
    let mut options = SolveOptions::default();
    options.solver.time_limit = None;
    options.solver.run_limit = Some(rnl_solver::RunLimit::Work(100_000_000));
    options.solver.safety_seconds = None;
    options
}

#[test]
fn profile_json_and_records_are_independent() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("solver-profile");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("complete.json");
    let provider = MemoryProvider::default().with(
        "test.sp",
        ".title Profile\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a b 1k\nR2 b 0 2k\n* #R1 @PLACE(0,0)\n* #R2 @PLACE(10,0)\n.end\n",
    );
    let options = SolveOptions { std: false, ..within_work() };
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
fn disabled_profile_keeps_committed_snapshot_bytes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/integration/solver");
    let case = root.join("T2b-common-source-dragged.sp");
    let run = solve_with_clock(&case, &rnl_cli::FsProvider::new(&[]), &within_work(), &StoppedClock).unwrap();
    let expected = std::fs::read(root.join("solved/T2b-common-source-dragged.sp")).unwrap();
    assert_eq!(run.files.len(), 1);
    assert_eq!(run.files[0].text.as_bytes(), expected);
}
