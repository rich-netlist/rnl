//! The `rnl` command itself: exit codes, the status lines of `check`, and `view`.

use std::path::Path;
use std::process::Command;

fn rnl(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).output().unwrap();
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"))
}

fn case(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/cases")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn check_reports_what_a_solver_would_have_to_do() {
    let (code, out) = rnl(&["check", &case("10-constraints-and-priority.sp")]);
    let status = out.lines().find(|l| l.starts_with("status ")).expect(&out);
    assert!(status.contains("1 needs-solver") && status.contains("1 unsolved"), "{status}");
    assert_eq!(code, if out.contains("\n0 errors") { 0 } else { 1 });
}

#[test]
fn check_fails_on_errors_and_lists_unplaced_objects() {
    let (code, out) = rnl(&["check", &case("12-diagnostics.sp")]);
    assert_eq!(code, 1, "{out}");
    assert!(out.lines().any(|l| l.starts_with("status ") && l.contains("2 without a position")), "{out}");
}

#[test]
fn usage_errors_exit_with_two() {
    assert_eq!(rnl(&["check"]).0, 2);
    assert_eq!(rnl(&["frobnicate", &case("01-rc-lowpass.sp")]).0, 2);
}

#[test]
fn view_prints_the_document_a_viewer_shows() {
    let (code, out) = rnl(&["view", &case("04-hierarchy.sp")]);
    assert_eq!(code, 0, "{out}");
    let d: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(d["name"], "04-hierarchy.sp");
    assert!(d["levels"].as_array().unwrap().iter().all(|l| l["svg"].as_str().unwrap().starts_with("<svg")));
    assert_eq!(d["files"][1]["name"], "04-hierarchy-lib.inc");
    assert_eq!(d["missing"], serde_json::json!([]));
    assert_eq!(rnl(&["view", &case("no-such-case.sp")]).0, 2);
}
