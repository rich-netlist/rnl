//! The `rnl` command itself: exit codes and the status lines of `check`.

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
