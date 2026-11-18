use serde_json::json;
use std::process::Command;

#[test]
fn modern_write_prints_the_kind_and_never_overwrites_output_on_refusal() {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("rnl-edit-diagnostics-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let input = dir.join("request.json");
    let output = dir.join("out.sp");
    let mut request = json!({"dialect":"spice", "author":"test", "netlist":"title\nR1 a 0 1k\n",
        "ops":[{"op":"set","level":"file","object":"#R1"}]});
    request["language_version"] = json!("0.4");
    std::fs::write(&input, request.to_string()).unwrap();
    std::fs::write(&output, "existing output").unwrap();
    let result =
        Command::new(env!("CARGO_BIN_EXE_rnl")).arg("write").arg(&input).arg("-o").arg(&output).output().unwrap();
    assert!(!result.status.success());
    let error = String::from_utf8(result.stderr).unwrap();
    assert!(error.contains("RNL8619 edit-set-nothing"), "{error}");
    assert!(error.contains("nothing to set"), "{error}");
    assert_eq!(std::fs::read_to_string(&output).unwrap(), "existing output");
    std::fs::remove_file(input).unwrap();
    std::fs::remove_file(output).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
