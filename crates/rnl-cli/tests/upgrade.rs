//! Upgrade writes preserve the source bytes outside RNL records.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-upgrade").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rnl")).current_dir(dir).arg("upgrade").args(args).output().unwrap()
}

const SOURCE: &str = "* café\r\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\r\nR1 a 0 1k\r\n.end\r\n";

#[test]
fn preserves_utf8_bom_and_latin1() {
    for encoding in ["utf8", "latin1"] {
        let dir = scratch(encoding);
        let encode = |text: &str| -> Vec<u8> {
            match encoding {
                "utf8" => [b"\xef\xbb\xbf".as_slice(), text.as_bytes()].concat(),
                _ => text.chars().map(|c| c as u8).collect(),
            }
        };
        std::fs::write(dir.join("top.sp"), encode(SOURCE)).unwrap();
        let output = run(&dir, &["top.sp", "--no-std", "--in-place", "--report", "report.json"]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(std::fs::read(dir.join("top.sp")).unwrap(), encode(&SOURCE.replace("0.3", "0.4")));
        assert_eq!(String::from_utf8(output.stdout).unwrap().lines().count(), 1);
        let report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("report.json")).unwrap()).unwrap();
        assert_eq!(report["files"][0]["changes"][0]["rule"], "language-version");
    }
}

#[test]
fn invalid_source_writes_neither_output_nor_report() {
    let dir = scratch("invalid");
    let source = "* title\n* @RNL { #missing @PLACE(1, 2) }\n.end\n";
    std::fs::write(dir.join("top.sp"), source).unwrap();
    let output = run(&dir, &["top.sp", "--no-std", "-o", "out", "--report", "report.json"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(!dir.join("out").exists());
    assert!(!dir.join("report.json").exists());
    assert_eq!(std::fs::read_to_string(dir.join("top.sp")).unwrap(), source);
}

#[test]
fn output_directory_copies_unchanged_includes_byte_for_byte() {
    let dir = scratch("include");
    let source = SOURCE.replace(".end", ".include child.inc\r\n.end");
    let child = b"* caf\xe9\r\n.subckt child a\r\nR1 a 0 1k\r\n.ends\r\n";
    std::fs::write(dir.join("top.sp"), &source).unwrap();
    std::fs::write(dir.join("child.inc"), child).unwrap();
    let output = run(&dir, &["top.sp", "--no-std", "-o", "out"]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(std::fs::read(dir.join("out/child.inc")).unwrap(), child);
    assert_eq!(std::fs::read_to_string(dir.join("top.sp")).unwrap(), source);
}

#[test]
fn report_collisions_are_rejected_before_writeback() {
    for report in ["top.sp", "out/top.sp", "child.inc", "out/child.inc", "out", "out/top.sp/details.json"] {
        let dir = scratch(&report.replace('/', "-"));
        let source = SOURCE.replace(".end", ".include child.inc\r\n.end");
        std::fs::write(dir.join("top.sp"), &source).unwrap();
        std::fs::write(dir.join("child.inc"), "* child\n.subckt child a\n.ends\n").unwrap();
        let output = run(&dir, &["top.sp", "--no-std", "-o", "out", "--report", report]);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(!dir.join("out").exists());
        assert_eq!(std::fs::read_to_string(dir.join("top.sp")).unwrap(), source);
    }
}

#[test]
fn an_unconverted_statement_is_retained_and_reported() {
    let dir = scratch("unconverted");
    let statement = "(0, 0) (1, 0) @DISTANCE(\"x\", min=1)";
    let source = format!("* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n* @RNL {{ {statement} }}\n.end\n");
    std::fs::write(dir.join("top.sp"), &source).unwrap();
    let output = run(&dir, &["top.sp", "--no-std", "-o", "out", "--report", "report.json"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let upgraded = std::fs::read_to_string(dir.join("out/top.sp")).unwrap();
    assert!(upgraded.contains(statement));
    assert!(upgraded.contains("/* rnl upgrade:"));
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["files"][0]["unconverted"][0]["line"], 3);
}

#[test]
fn requires_one_input_and_accepts_at_most_one_write_target() {
    let dir = scratch("arguments");
    std::fs::write(dir.join("top.sp"), SOURCE).unwrap();
    for args in [vec!["top.sp", "--in-place", "-o", "out"], vec!["top.sp", "top.sp", "--in-place"]] {
        assert_eq!(run(&dir, &args).status.code(), Some(2));
    }
    assert_eq!(std::fs::read_to_string(dir.join("top.sp")).unwrap(), SOURCE);
}

#[test]
fn preview_keeps_inputs_unchanged_with_or_without_a_report() {
    for report in [false, true] {
        let dir = scratch(if report { "preview-report" } else { "preview" });
        std::fs::write(dir.join("top.sp"), SOURCE).unwrap();
        let mut args = vec!["top.sp", "--no-std"];
        if report {
            args.extend(["--report", "report.json"]);
        }
        let output = run(&dir, &args);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(std::fs::read_to_string(dir.join("top.sp")).unwrap(), SOURCE);
        assert!(String::from_utf8(output.stdout).unwrap().starts_with("0 files written,"));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), if report { 2 } else { 1 });
        if report {
            let data: serde_json::Value =
                serde_json::from_slice(&std::fs::read(dir.join("report.json")).unwrap()).unwrap();
            assert_eq!(data["files"][0]["to"], "0.4");
            assert_eq!(data["files"][0]["changes"][0]["rule"], "language-version");
        }
    }
}

#[test]
fn preview_reports_unconverted_statements_without_writing_netlists() {
    let dir = scratch("preview-unconverted");
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n* @RNL { (0, 0) (1, 0) @DISTANCE(\"x\", min=1) }\n.end\n";
    std::fs::write(dir.join("top.sp"), source).unwrap();
    let output = run(&dir, &["top.sp", "--no-std", "--report", "report.json"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(std::fs::read_to_string(dir.join("top.sp")).unwrap(), source);
    let data: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("report.json")).unwrap()).unwrap();
    assert_eq!(data["files"][0]["unconverted"][0]["rule"], "direction");
}

#[test]
fn preview_cannot_overwrite_an_input_with_its_report() {
    let dir = scratch("preview-collision");
    std::fs::write(dir.join("top.sp"), SOURCE).unwrap();
    let output = run(&dir, &["top.sp", "--no-std", "--report", "top.sp"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(std::fs::read_to_string(dir.join("top.sp")).unwrap(), SOURCE);
}

#[test]
fn an_unchanged_include_outside_the_output_tree_is_rejected_before_writing() {
    let dir = scratch("outside");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/top.sp"), SOURCE.replace(".end", ".include ../child.inc\r\n.end")).unwrap();
    std::fs::write(dir.join("child.inc"), "* child\n.subckt child a\n.ends\n").unwrap();
    let output = run(&dir, &["src/top.sp", "--no-std", "-o", "out"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(!dir.join("out").exists());
}

#[test]
fn utf16_is_rejected_without_corrupting_the_file() {
    let dir = scratch("unsupported-encoding");
    let bytes = [vec![0xff, 0xfe], SOURCE.encode_utf16().flat_map(u16::to_le_bytes).collect()].concat();
    std::fs::write(dir.join("top.sp"), &bytes).unwrap();
    let output = run(&dir, &["top.sp", "--no-std", "--in-place"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(std::fs::read(dir.join("top.sp")).unwrap(), bytes);
}

#[test]
fn removing_solver_records_is_reported_as_needing_a_new_solve() {
    let dir = scratch("solver-records");
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n* @RNL { %p=(_, _); %p @TEXT(\"note\") }\n* @RNL(type=\"generated\") { %p @PLACE(2, 3) }\n.end\n";
    std::fs::write(dir.join("top.sp"), source).unwrap();
    let output = run(&dir, &["top.sp", "--no-std", "-o", "out", "--report", "report.json"]);
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8(output.stdout).unwrap().contains("1 files need solving"));
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["files"][0]["needs_solve"], true);
    assert!(!std::fs::read_to_string(dir.join("out/top.sp")).unwrap().contains("generated"));
}

#[test]
fn approximate_is_explicit_and_has_its_own_report_and_summary_count() {
    let dir = scratch("approximate");
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n* @RNL { {(0,0) (2,0) (4,0)} @DISTRIBUTE(\"x\") }\n.end\n";
    std::fs::write(dir.join("top.sp"), source).unwrap();
    for (flags, report_path, count) in [
        (vec!["top.sp", "--no-std", "--report", "exact.json"], "exact.json", 0),
        (vec!["top.sp", "--no-std", "--approximate", "--report", "approx.json"], "approx.json", 1),
    ] {
        let result = run(&dir, &flags);
        assert_ne!(result.status.code(), Some(2), "{result:?}");
        let report: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join(report_path)).unwrap()).unwrap();
        assert_eq!(report["files"][0]["approximations"].as_array().unwrap().len(), count);
        assert!(String::from_utf8(result.stdout).unwrap().contains(&format!("{count} approximations")));
    }
    assert_eq!(std::fs::read_to_string(dir.join("top.sp")).unwrap(), source);
}

#[test]
fn keep_generated_upgrades_the_solver_record_instead_of_discarding_it() {
    let dir = scratch("keep-generated");
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL(type=\"generated\", priority=\"high\", author=\"reference\") {\n*   #R1 @PLACE(3, 4)\n* }\n.end\n";
    std::fs::write(dir.join("top.sp"), source).unwrap();
    for (flags, kept) in [
        (vec!["top.sp", "--no-std", "-o", "dropped"], false),
        (vec!["top.sp", "--no-std", "--keep-generated", "-o", "kept"], true),
    ] {
        let result = run(&dir, &flags);
        assert_ne!(result.status.code(), Some(2), "{result:?}");
        let written = std::fs::read_to_string(dir.join(flags[flags.len() - 1]).join("top.sp")).unwrap();
        assert_eq!(written.contains("author=\"reference\""), kept, "{written}");
        assert_eq!(written.contains("@PLACE(3, 4)"), kept, "{written}");
    }
    assert_eq!(std::fs::read_to_string(dir.join("top.sp")).unwrap(), source);
}

#[test]
fn a_change_of_the_default_look_does_not_fail_the_upgrade_but_a_real_difference_does() {
    let dir = scratch("default-look");
    std::fs::write(
        dir.join("look.sp"),
        "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nM1 d g s b nmos\nR1 d vdd 1k\n.end\n",
    )
    .unwrap();
    let output = run(&dir, &["look.sp", "-o", "look-out", "--report", "look.json"]);
    assert!(output.status.success(), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("look.json")).unwrap()).unwrap();
    assert_eq!(report["differences"].as_array().unwrap().len(), 0);
    assert!(String::from_utf8(output.stdout).unwrap().contains("0 differences"));
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL { $S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (2, 0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S }\n.end\n";
    std::fs::write(dir.join("real.sp"), source).unwrap();
    let output = run(&dir, &["real.sp", "--no-std", "-o", "real-out"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
}
