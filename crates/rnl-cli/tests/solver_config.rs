use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rnl-cli-config-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rnl"))
            .current_dir(&self.0)
            .env_remove("RNL_SOLVER_CONFIG")
            .args(args)
            .output()
            .unwrap()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn success(out: Output) -> String {
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn solver_config_prints_copyable_defaults_and_default_is_beside_executable() {
    let d = Directory::new();
    let defaults = success(d.run(&["solver-config", "--defaults"]));
    assert_eq!(rnl_solver::SolverOptions::from_toml(&defaults).unwrap(), rnl_solver::SolverOptions::builtin());
    let exe_dir = d.0.join("bin");
    std::fs::create_dir(&exe_dir).unwrap();
    let exe = exe_dir.join(if cfg!(windows) { "rnl.exe" } else { "rnl" });
    std::fs::copy(env!("CARGO_BIN_EXE_rnl"), &exe).unwrap();
    let run = || {
        Command::new(&exe).current_dir(&d.0).env_remove("RNL_SOLVER_CONFIG").args(["solver-config"]).output().unwrap()
    };
    assert!(success(run()).starts_with("# solver config: builtin\n"));
    std::fs::write(exe_dir.join("rnl-solver.toml"), "[rules]\nbeam = 3\n").unwrap();
    d.file("rnl-solver.toml", "invalid TOML");
    let selected = success(run());
    assert!(selected.contains("beam = 3"));
    assert!(selected.contains(&exe_dir.to_string_lossy().to_string()));
}

#[test]
fn selected_configuration_and_overrides_are_shared_by_problem_preview_and_solve() {
    let d = Directory::new();
    d.file("rc.sp", "* rc\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a b 1k\nC1 b 0 1p\n.end\n");
    d.file("config.toml", "[rules]\nbeam = 2\ntime_limit = \"none\"\n");
    let options = ["--solver-config", "config.toml", "--solver-options", "beam=3"];
    let command = |name: &str| {
        let mut args = vec![name, "rc.sp"];
        args.extend(options);
        d.run(&args)
    };
    let problem: Value = serde_json::from_str(&success(command("problem"))).unwrap();
    let preview: Value = serde_json::from_str(&success(command("preview"))).unwrap();
    let canonical = preview["report"]["levels"][0]["scoring"]["config"].as_str().unwrap();
    assert_eq!(
        problem["levels"][0]["hash"].as_str().unwrap().split('.').nth(1),
        Some(rnl_runtime::problem::options_digest(canonical).as_str())
    );
    assert_eq!(preview["report"]["config_source"], "config.toml");
    assert!(preview["report"]["invocation"]["solver_revision"].as_str().is_some_and(|s| !s.is_empty()));
    assert_eq!(preview["report"]["invocation"]["inputs"][0]["text_sha256"].as_str().unwrap().len(), 64);
    let mut args = vec!["solve", "rc.sp", "-o", "out", "--report"];
    args.extend(options);
    let output = success(d.run(&args));
    assert!(output.starts_with("solver config: config.toml"));
    let solved = std::fs::read_to_string(d.0.join("out/rc.sp")).unwrap();
    assert!(solved.contains(problem["levels"][0]["hash"].as_str().unwrap()));
}

#[test]
fn upgraded_zero_two_reports_original_decoded_input_and_include_hashes() {
    let d = Directory::new();
    let original = "* legacy\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n.include child.inc\nR1 a b 1k\n.end\n";
    let child = ".subckt child a b\nR2 a b 2k\n.ends child\n";
    d.file("legacy.sp", &format!("\u{feff}{original}"));
    d.file("child.inc", child);
    let output = success(d.run(&[
        "solve",
        "legacy.sp",
        "-o",
        "out",
        "--report",
        "--solver-options",
        "budget=candidates:0;time_limit=none",
    ]));
    let manifest: Value = serde_json::from_str(
        output.lines().find_map(|line| line.strip_prefix("input manifest: ")).expect("CLI input manifest"),
    )
    .unwrap();
    let sha = |text: &str| {
        rnl_runtime::numbering::sha256(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>()
    };
    let inputs = manifest.as_array().unwrap();
    assert_eq!(inputs.len(), 2, "{manifest}");
    for (name, text) in [("legacy.sp", original), ("child.inc", child)] {
        let input =
            inputs.iter().find(|input| input["path"].as_str().is_some_and(|path| path.ends_with(name))).unwrap();
        assert_eq!(input["text_sha256"], sha(text));
    }
    let written = std::fs::read_to_string(d.0.join("out/legacy.sp")).unwrap();
    assert!(written.contains("@VERSION(\"0.3\")"), "upgrade was not exercised: {written}");
}

#[test]
fn every_solve_operational_error_is_one_and_unknown_cli_option_is_two() {
    let d = Directory::new();
    d.file("rc.sp", "* rc\nR1 a 0 1k\n.end\n");
    d.file("bad.toml", "[rules]\nbeam = 0\n");
    d.file("blocked", "a file, not an output directory");
    for args in [
        vec!["solve", "missing.sp", "-o", "out"],
        vec!["solve", "rc.sp"],
        vec!["solve", "rc.sp", "-o", "blocked"],
        vec!["solve", "rc.sp", "-o", "out", "--solver-config", "missing.toml"],
        vec!["solve", "rc.sp", "-o", "out", "--solver-config", "bad.toml"],
        vec!["solve", "rc.sp", "-o", "out", "--solver-options", "objectives=none"],
        vec!["solve", "rc.sp", "-o", "out", "--solver-options", "unknown=1"],
        vec!["solve", "rc.sp", "-o", "out", "--level", "missing"],
    ] {
        assert_eq!(d.run(&args).status.code(), Some(1), "{args:?}");
    }
    assert_eq!(d.run(&["solve", "rc.sp", "--unknown-option"]).status.code(), Some(2));
    let env = Command::new(env!("CARGO_BIN_EXE_rnl"))
        .current_dir(&d.0)
        .env("RNL_SOLVER_CONFIG", Path::new("missing-env.toml"))
        .args(["solve", "rc.sp", "-o", "out"])
        .output()
        .unwrap();
    assert_eq!(env.status.code(), Some(1));
}
