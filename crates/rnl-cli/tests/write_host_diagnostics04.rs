//! Host refusals reach the write command before any output is replaced.
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rnl-write-host-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(body: &str, version: Option<&str>, existing: bool) -> (i32, String, Option<String>) {
    let dir = Scratch::new();
    let input = dir.0.join("request.json");
    let out = dir.0.join("out.sp");
    fs::write(&input, serde_json::to_vec(&json!({"language_version":version,"dialect":"spice","author":"test","netlist":format!("title\n{body}"),"ops":[]})).unwrap()).unwrap();
    if existing {
        fs::write(&out, "keep this file\n").unwrap();
    }
    let result = Command::new(env!("CARGO_BIN_EXE_rnl")).arg("write").arg(&input).arg("-o").arg(&out).output().unwrap();
    (result.status.code().unwrap_or(-1), String::from_utf8(result.stderr).unwrap(), fs::read_to_string(out).ok())
}

#[test]
fn malformed_modern_includes_refuse_without_overwriting_output() {
    for body in [".include\n", ".lib\n"] {
        let (code, stderr, output) = run(body, Some("0.4"), true);
        assert_eq!(code, 2, "{stderr}");
        assert!(stderr.contains("RNL9012") && stderr.contains("out.sp:2"), "{stderr}");
        assert_eq!(output.as_deref(), Some("keep this file\n"));
    }
}

#[test]
fn a_host_refusal_names_the_subcircuit_at_its_declaration() {
    let (code, stderr, output) = run(".subckt open_cell a b\nR1 a b 1k\n", Some("0.4"), false);
    assert_eq!(code, 2);
    assert!(stderr.contains("RNL9008") && stderr.contains("open_cell") && stderr.contains("out.sp:2"), "{stderr}");
    assert!(output.is_none());
}

#[test]
fn released_write_commands_keep_the_existing_output_behavior() {
    for version in [None, Some("0.3")] {
        let (code, stderr, output) = run(".include\nR1 a b 1k\n", version, false);
        assert_eq!(code, 0, "{stderr}");
        assert!(stderr.is_empty());
        let output = output.unwrap();
        assert!(output.contains(".include\n") && output.contains("@VERSION(\"0.3\")"));
    }
}
