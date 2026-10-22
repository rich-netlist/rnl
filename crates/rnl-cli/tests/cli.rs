//! The `rnl` command itself: exit codes, the status lines of `check`, `view` and its outputs,
//! `strip-generated` and `catalog`.

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
fn version_names_the_core_and_build_commit() {
    let (code, out) = rnl(&["--version"]);
    assert_eq!(code, 0);
    assert_eq!(out, format!("rnl {} ({})\n", rnl_cli::CORE_VERSION, rnl_cli::BUILD_COMMIT));
    assert_eq!(rnl(&["-V"]), (code, out));
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
fn files_of_0_3_are_read() {
    let f = Path::new(env!("CARGO_TARGET_TMPDIR")).join("version-0.3.sp");
    let text =
        "* a file of 0.3\nR1 a b 1k\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n* @RNL { #R1 @PLACE(3, 4)}\n.end\n";
    std::fs::write(&f, text).unwrap();
    let f = f.to_string_lossy();
    // 0.3 is read without any switch (16.1)
    let (code, out) = rnl(&["check", &f]);
    assert_eq!(code, 0, "{out}");
    assert!(out.lines().any(|l| l.starts_with("0 errors")), "{out}");
    // the solver's view reads it too
    let places = |args: &[&str]| -> usize {
        let (code, out) = rnl(args);
        assert_eq!(code, 0, "{out}");
        let p: serde_json::Value = serde_json::from_str(&out).unwrap();
        p["levels"][0]["facts"].as_array().unwrap().iter().filter(|f| f["kind"] == "PLACE-x").count()
    };
    assert_eq!(places(&["problem", &f]), 1);
    // --draft, which once read them and then had no effect, is gone
    let (code, out) = rnl(&["check", &f, "--draft"]);
    assert_eq!(code, 2, "{out}");
    let help = Command::new(env!("CARGO_BIN_EXE_rnl")).arg("--help").output().unwrap();
    assert!(!String::from_utf8_lossy(&help.stderr).contains("--draft"));
}

#[test]
fn a_version_that_is_not_read_is_unsupported() {
    let f = Path::new(env!("CARGO_TARGET_TMPDIR")).join("version-0.4.sp");
    let text =
        "* a file of 0.4\nR1 a b 1k\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\n* @RNL { #R1 @PLACE(3, 4)}\n.end\n";
    std::fs::write(&f, text).unwrap();
    let f = f.to_string_lossy();
    // no RNL of the file is read (16.1)
    let (code, out) = rnl(&["check", &f]);
    assert_eq!(code, 1, "{out}");
    let message =
        "unsupported version 0.4: no RNL of this file is interpreted (16.1; this implementation reads 0.2 and 0.3)";
    assert!(out.contains(message), "{out}");
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

#[test]
fn std_version_reads_a_file_with_another_standard_series() {
    // a file of 0.3 that writes $GND of the series 0.2: an error that names $GROUND, and none when
    // the series 0.2 is asked for
    let dir = std::env::temp_dir().join(format!("rnl-cli-std-version-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.sp");
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a 0 1k\n* @RNL { $GND @ATTACH &a }\n.end\n";
    std::fs::write(&file, text).unwrap();
    let path = file.to_string_lossy();
    let (code, out) = rnl(&["check", &path]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("the standard series 0.3 calls it $GROUND"), "{out}");
    let (code, out) = rnl(&["check", &path, "--std-version", "0.2"]);
    assert_eq!(code, 0, "{out}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn catalog_prints_the_packages_the_core_has() {
    let json = |out: &str| -> serde_json::Value { serde_json::from_str(out).unwrap() };
    let (code, out) = rnl(&["catalog"]);
    assert_eq!(code, 0, "{out}");
    let c = json(&out);
    let symbols = c["symbols"].as_array().unwrap();
    let resistor = symbols.iter().find(|s| s["symbol"] == "$RESISTOR").unwrap();
    assert_eq!((resistor["package"].as_str(), resistor["version"].as_str()), (Some("rnl-std-basic"), Some("0.3")));
    // what rnl-view lists, read back from the same text
    let native = rnl_view::catalog(&rnl_cli::FsProvider::new(&[]), &rnl_view::CatalogOptions::default());
    assert_eq!(c, json(&serde_json::to_string_pretty(&native).unwrap()));

    // the names asked for, every version of each
    let (code, out) = rnl(&["catalog", "--package", "rnl-std", "--package", "rnl-zh", "--all-versions"]);
    assert_eq!(code, 0, "{out}");
    let listed: Vec<(String, String)> = json(&out)["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap().to_string(), p["version"].as_str().unwrap().to_string()))
        .collect();
    let want = [("rnl-std", "0.2"), ("rnl-std", "0.3"), ("rnl-zh", "0.2"), ("rnl-zh", "0.3")]
        .map(|(n, v)| (n.to_string(), v.to_string()));
    assert_eq!(listed, want);

    // a package of a package directory
    let dir = std::env::temp_dir().join(format!("rnl-cli-catalog-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let acme = "@RNL(type=\"header\") { @VERSION(\"0.3\"); @PACKAGE(name=\"acme\", version=\"1.0\")}\n\
                @RNL { $ACME = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1, direction=\"up\") } }\n";
    std::fs::write(dir.join("acme.rnl"), acme).unwrap();
    let (code, out) = rnl(&["catalog", "--package-dir", &dir.to_string_lossy(), "--package", "acme"]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(code, 0, "{out}");
    let c = json(&out);
    assert_eq!(c["packages"][0]["file"], "acme.rnl");
    assert_eq!(c["symbols"][0]["symbol"], "$ACME");
    assert_eq!(c["symbols"][0]["pins"], serde_json::json!([{"terminal": 1, "at": [0.0, 0.0], "direction": "up"}]));

    // the catalog takes no FILE
    assert_eq!(rnl(&["catalog", &case("01-rc-lowpass.sp")]).0, 2);
}
