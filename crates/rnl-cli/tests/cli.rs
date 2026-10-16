//! The `rnl` command itself: exit codes, the status lines of `check`, `view` and its outputs, and
//! `strip-generated`.

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

#[test]
fn view_gives_the_primitives_and_the_svg_as_asked() {
    let (code, out) = rnl(&["view", &case("01-rc-lowpass.sp"), "--primitives", "--no-svg"]);
    assert_eq!(code, 0, "{out}");
    let d: serde_json::Value = serde_json::from_str(&out).unwrap();
    let level = &d["levels"][0];
    assert!(level.get("svg").is_none() && level["scale"] == 14.0);
    let kinds: Vec<&str> =
        level["primitives"].as_array().unwrap().iter().map(|p| p["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"path") && kinds.contains(&"text"), "{kinds:?}");
    // without the flags, the SVG and no primitives
    let d: serde_json::Value = serde_json::from_str(&rnl(&["view", &case("01-rc-lowpass.sp")]).1).unwrap();
    assert!(d["levels"][0]["svg"].is_string() && d["levels"][0].get("primitives").is_none());
}

#[test]
fn draft_reads_the_files_that_declare_the_draft_0_3() {
    let f = Path::new(env!("CARGO_TARGET_TMPDIR")).join("draft-0.3.sp");
    let text = "* a file of the draft 0.3\nR1 a b 1k\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n* @RNL { #R1 @PLACE(3, 4)}\n.end\n";
    std::fs::write(&f, text).unwrap();
    let f = f.to_string_lossy();
    // without the switch, 0.3 is an unsupported version, and no RNL of the file is read (16.1)
    let (code, out) = rnl(&["check", &f]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("unsupported version 0.3"), "{out}");
    let (code, out) = rnl(&["check", &f, "--draft"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.lines().any(|l| l.starts_with("0 errors")), "{out}");
    // the solver's view reads it too
    let places = |args: &[&str]| -> usize {
        let (code, out) = rnl(args);
        assert_eq!(code, 0, "{out}");
        let p: serde_json::Value = serde_json::from_str(&out).unwrap();
        p["levels"][0]["facts"].as_array().unwrap().iter().filter(|f| f["kind"] == "PLACE-x").count()
    };
    assert_eq!(places(&["problem", &f]), 0);
    assert_eq!(places(&["problem", &f, "--draft"]), 1);
    // the switch is in the help, for development only
    let help = Command::new(env!("CARGO_BIN_EXE_rnl")).arg("--help").output().unwrap();
    assert!(String::from_utf8_lossy(&help.stderr).contains("--draft             for development only"));
}

#[test]
fn strip_generated_reads_the_bundled_packages_too() {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("strip-generated-rnl-std-symbols.rnl");
    let (code, _) = rnl(&["strip-generated", "<bundled>/0.2/rnl-std-symbols.rnl", "-o", &out.to_string_lossy()]);
    assert_eq!(code, 0);
    // the standard package has no generated records, so it comes out as it is
    let std = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/standard/0.2/rnl-std-symbols.rnl");
    assert_eq!(std::fs::read_to_string(out).unwrap(), std::fs::read_to_string(std).unwrap());
}
