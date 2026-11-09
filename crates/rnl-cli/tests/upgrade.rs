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
fn requires_one_input_and_one_write_target() {
    let dir = scratch("arguments");
    std::fs::write(dir.join("top.sp"), SOURCE).unwrap();
    for args in [vec!["top.sp"], vec!["top.sp", "--in-place", "-o", "out"], vec!["top.sp", "top.sp", "--in-place"]] {
        assert_eq!(run(&dir, &args).status.code(), Some(2));
    }
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
