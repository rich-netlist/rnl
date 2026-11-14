//! Modern writes validate host input with the same per-reading language rules as interpretation.
use rnl_edit::{write_diagnosed, write_with_options_diagnosed, WriteRequest};
use rnl_runtime::{source::MemoryProvider, Options};
use serde_json::json;

fn request(version: Option<&str>, dialect: &str, netlist: &str) -> WriteRequest {
    serde_json::from_value(
        json!({"language_version":version,"dialect":dialect,"author":"test","netlist":netlist,"ops":[]}),
    )
    .unwrap()
}

#[test]
fn undeclared_input_uses_the_requested_output_language() {
    for (dialect, netlist) in [
        ("spice", "title\n.include\nR1 a b 1k\n"),
        ("spice", "title\n.lib\nR1 a b 1k\n"),
        ("spectre", "// title\ninclude\nR1 (a b) resistor r=1k\n"),
    ] {
        let failure = write_diagnosed(&request(Some("0.4"), dialect, netlist)).unwrap_err();
        assert_eq!(failure.code.number().as_deref(), Some("RNL9012"));
        let d = failure.diagnostics.iter().find(|d| d.code.as_deref() == Some("RNL9012")).unwrap();
        assert_eq!(d.line, 2);
        assert_eq!(d.severity, "error");
    }
}

#[test]
fn editor_host_errors_use_the_named_declaration_location() {
    for (dialect, netlist) in [
        ("spice", "title\n.subckt open_cell a b\nR1 a b 1k\n"),
        ("spectre", "// title\nsubckt open_cell (a b)\nR1 (a b) resistor r=1k\n"),
    ] {
        let failure = write_diagnosed(&request(Some("0.4"), dialect, netlist)).unwrap_err();
        let d = failure.diagnostics.iter().find(|d| d.code.as_deref() == Some("RNL9008")).unwrap();
        assert_eq!(d.line, 2);
        assert!(d.message.contains("open_cell") && d.message.contains("no closing ends declaration"));
    }
}

#[test]
fn includes_keep_their_own_declared_language() {
    for version in ["0.3", "0.4"] {
        let child = format!("* child\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n.include\n");
        let files = MemoryProvider::new().with("/rnl-edit/child.sp", &child);
        let req = request(Some("0.4"), "spice", "title\n.include child.sp\nR1 a b 1k\n");
        let result = write_with_options_diagnosed(&req, &files, &Options { std: false, ..Default::default() });
        if version == "0.4" {
            let failure = result.unwrap_err();
            assert_eq!(failure.code.number().as_deref(), Some("RNL9012"));
            assert!(failure.diagnostics.iter().any(|d| d.line == 3 && d.file.ends_with("child.sp")));
        } else {
            assert!(result.unwrap().diagnostics.iter().all(|d| d.code.as_deref() != Some("RNL9012")));
        }
    }
}

#[test]
fn an_undeclared_include_inherits_the_requested_output_language() {
    let files = MemoryProvider::new().with("/rnl-edit/child.sp", "* child\n.include\n");
    let req = request(Some("0.4"), "spice", "title\n.include child.sp\n");
    let failure =
        write_with_options_diagnosed(&req, &files, &Options { std: false, ..Default::default() }).unwrap_err();
    assert_eq!(failure.code.number().as_deref(), Some("RNL9012"));
}

#[test]
fn library_readings_keep_separate_version_boundaries() {
    for version in ["0.3", "0.4"] {
        let library = format!("* library\n.lib tt\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n.include\n.endl tt\n.lib other\n.include\n.endl other\n");
        let files = MemoryProvider::new().with("/rnl-edit/lib.sp", &library);
        let req = request(Some("0.4"), "spice", "title\n.lib lib.sp tt\n");
        let result = write_with_options_diagnosed(&req, &files, &Options { std: false, ..Default::default() });
        if version == "0.4" {
            let failure = result.unwrap_err();
            assert_eq!(failure.diagnostics.iter().filter(|d| d.code.as_deref() == Some("RNL9012")).count(), 1);
        } else {
            assert!(result.is_ok(), "{result:?}");
        }
    }
}

#[test]
fn host_validation_does_not_evaluate_included_rnl_bodies() {
    let files = MemoryProvider::new()
        .with("/rnl-edit/child.sp", "* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n* @RNL { %missing }\n");
    let req = request(Some("0.4"), "spice", "title\n.include child.sp\n");
    let written = write_with_options_diagnosed(&req, &files, &Options { std: false, ..Default::default() }).unwrap();
    assert!(written.diagnostics.iter().all(|d| d.severity != "error"));
}

#[test]
fn warnings_are_kept_once_at_locations_in_the_written_text() {
    let mut req = request(
        Some("0.4"),
        "spice",
        "title\n.subckt cell a b\nR1 a b 1k\n.ends cell\n.lib unused\n* @RNL {}\n.endl unused\n",
    );
    req.ops = vec![json!({"op":"set","level":"subckt:cell","object":"#R1","at":[0,0]})];
    let written = write_diagnosed(&req).unwrap();
    let found: Vec<_> = written.diagnostics.iter().filter(|d| d.name == "host-rnl-in-unselected-section").collect();
    assert_eq!(found.len(), 1, "{:?}", written.diagnostics);
    let line = written.text.lines().position(|line| line == "* @RNL {}").unwrap() + 1;
    assert_eq!(found[0].line as usize, line);
    assert_eq!(found[0].message, "RNL in a library section that is not selected is not read (2.2)");
}

#[test]
fn released_writes_keep_their_existing_acceptance_and_text() {
    for version in [None, Some("0.3")] {
        for netlist in
            ["title\n.include\nR1 a b 1k\n", "title\n.lib\nR1 a b 1k\n", "title\n.subckt open_cell a b\nR1 a b 1k\n"]
        {
            let written = write_diagnosed(&request(version, "spice", netlist)).unwrap();
            assert!(written.text.contains("@VERSION(\"0.3\")"));
            assert!(!written.diagnostics.iter().any(|d| d.code.as_deref() == Some("RNL9012")));
        }
    }
}

#[test]
fn final_host_locations_follow_junction_insertions_and_keep_junction_errors() {
    for configured in [true, false] {
        let value = if configured { "\"$DOT\"" } else { "false" };
        let package = format!("@RNL(type=\"header\") {{ @VERSION(\"0.4\"); @PACKAGE(name=\"dots\",version=\"1.0\"); @SOLVER-CONFIG(junction={value}) }}\n@RNL {{ $DOT = @DEFINE(roles=[[\"p\",\"point\"]]) {{}} }}\n");
        let mut files = MemoryProvider::new().with("/packages/dots.rnl", &package);
        files.package_dirs.push("/packages".into());
        let mut req = request(
            Some("0.4"),
            "spice",
            "title\n.subckt cell a b\nR1 a b 1k\n.ends cell\n.lib unused\n* @RNL {}\n.endl unused\n",
        );
        req.ops = vec![
            json!({"op":"use-package","name":"dots","version":"1.0"}),
            json!({"op":"add-point","level":"subckt:cell","name":"%j","at":[1,2],"junction":true}),
        ];
        let written =
            write_with_options_diagnosed(&req, &files, &Options { std: false, ..Default::default() }).unwrap();
        let host: Vec<_> = written.diagnostics.iter().filter(|d| d.name == "host-rnl-in-unselected-section").collect();
        assert_eq!(host.len(), 1, "{:?}", written.diagnostics);
        assert_eq!(host[0].line as usize, written.text.lines().position(|line| line == "* @RNL {}").unwrap() + 1);
        assert_eq!(written.text.contains("$DOT @ATTACH %j"), configured);
        assert_eq!(
            written.diagnostics.iter().filter(|d| d.code.as_deref() == Some("RNL3128")).count(),
            usize::from(!configured)
        );
    }
}
