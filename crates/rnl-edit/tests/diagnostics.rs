use rnl_edit::{ops::parse_diagnosed, write, write_diagnosed, WriteRequest};
use serde_json::{json, Value};

fn request(ops: Value) -> WriteRequest {
    serde_json::from_value(json!({"language_version":"0.4", "dialect":"spice", "author":"test",
        "netlist":"title\nR1 a 0 1k\n", "ops":ops}))
    .unwrap()
}

#[test]
fn operation_failures_have_stable_kinds_and_keep_the_op_index() {
    let cases = [
        (json!(null), "RNL8610"),
        (json!({}), "RNL8608"),
        (json!({"op":"absent"}), "RNL8612"),
        (json!({"op":"remove"}), "RNL8611"),
        (json!({"op":"set","extra":1}), "RNL8613"),
        (json!({"op":"set"}), "RNL8609"),
        (json!({"op":"set","level":false}), "RNL8603"),
        (json!({"op":"set","level":"file"}), "RNL8604"),
        (json!({"op":"set","level":"file","object":"#R1"}), "RNL8619"),
        (json!({"op":"add-text","level":"file","markup":"a","lines":[]}), "RNL8622"),
        (json!({"op":"add-text","level":"file"}), "RNL8623"),
        (json!({"op":"add-text","level":"file","markup":"a"}), "RNL8624"),
        (json!({"op":"add-shape","level":"file","shape":"unknown","points":[]}), "RNL8620"),
        (json!({"op":"add-shape","level":"file","shape":"line","points":[[0,0]]}), "RNL8621"),
        (json!({"op":"define-symbol","level":"file","name":"$S","pins":[{}]}), "RNL8614"),
        (json!({"op":"define-symbol","level":"file","name":"$S","prims":[{"shape":"line","points":[]}]}), "RNL8621"),
    ];
    for (value, code) in cases {
        let error = parse_diagnosed(7, &value).unwrap_err();
        assert_eq!(error.code.number(), code, "{value}: {error}");
        assert!(error.message.starts_with("op 7"), "{error}");
        assert_eq!(rnl_edit::ops::parse(7, &value).unwrap_err(), error.message);
    }
}

#[test]
fn writer_failures_use_the_catalog_without_changing_legacy_messages() {
    for (dialect, code) in [("text", "RNL8601"), ("rnl", "RNL8600"), ("unknown", "RNL8602")] {
        let mut req = request(json!([]));
        req.dialect = dialect.into();
        let error = write_diagnosed(&req).unwrap_err();
        assert_eq!(error.code.number(), code);
        assert_eq!(write(&req).unwrap_err(), error.message);
    }
    for (level, code) in [("unknown", "RNL8605"), ("subckt:missing", "RNL8606")] {
        let req = request(json!([{"op":"set","level":level,"object":"#R1","at":[0,0]}]));
        let error = write_diagnosed(&req).unwrap_err();
        assert_eq!(error.code.number(), code);
        assert!(error.message.starts_with("op 0 (set):"));
    }
}

#[test]
fn host_errors_keep_their_original_number_and_location_and_refuse_the_write() {
    let mut req = request(json!([]));
    req.netlist = "title\n.include missing.sp\nR1 a 0 1k\n".into();
    let error = write_diagnosed(&req).unwrap_err();
    assert!(!error.diagnostics.is_empty());
    let diagnostic = error.diagnostics.iter().find(|d| d.severity == "error").unwrap();
    assert_eq!(diagnostic.code, error.code.number());
    assert_eq!(diagnostic.line, 2);
    assert!(diagnostic.full_path.ends_with("netlist.sp"));
    assert!(diagnostic.message.contains("missing.sp"));
}

#[test]
fn a_valid_edit_has_the_same_text_through_both_interfaces() {
    for version in [None, Some("0.4")] {
        let mut req = request(json!([{"op":"set","level":"file","object":"#R1","at":[2,3]}]));
        req.language_version = version.map(String::from);
        let written = write_diagnosed(&req).unwrap();
        assert_eq!(written.text, write(&req).unwrap());
        assert!(written.diagnostics.iter().all(|d| d.severity != "error"));
    }
}

#[test]
fn provided_includes_are_available_to_the_host_validation() {
    let mut req = request(json!([]));
    req.netlist = "title\n.include part.sp\nR1 a 0 1k\n".into();
    let files = rnl_runtime::source::MemoryProvider::new().with("/rnl-edit/part.sp", "* part\nR2 a 0 2k\n");
    let written = rnl_edit::write_with_options_diagnosed(&req, &files, &rnl_runtime::Options::default()).unwrap();
    assert!(written.diagnostics.iter().all(|d| d.severity != "error"));
    assert!(written.text.contains(".include part.sp"));
    assert!(!written.text.contains("R2"));
}
