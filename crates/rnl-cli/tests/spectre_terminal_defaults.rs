//! Spectre terminal defaults of transmission lines and transformers (6.12).
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(source: &str, extra: Option<&str>) -> Interpretation {
    let mut files = MemoryProvider::new().with("t.scs", source);
    if let Some(extra) = extra {
        files = files.with("cells.scs", extra);
    }
    let result =
        interpret_file(Path::new("t.scs"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(!result.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", result.diagnostics());
    result
}

fn source(version: &str, master: &str, header: &str) -> String {
    format!("// terminal defaults\nsimulator lang=spectre\n// @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {header} }}\nT1 (a b c d) {master}\n")
}

fn names(result: &Interpretation) -> Vec<Option<String>> {
    result
        .model
        .levels
        .iter()
        .find_map(|lv| {
            result.doc.levels[lv.level]
                .instances
                .iter()
                .position(|i| i.name == "T1")
                .map(|i| lv.terminal_names[i].clone())
        })
        .unwrap()
}

fn named(values: &[&str]) -> Vec<Option<String>> {
    values.iter().map(|v| Some((*v).to_owned())).collect()
}

#[test]
fn transmission_lines_and_transformers_share_the_draft_names() {
    for master in ["tline", "transformer"] {
        let input = source("0.4", master, "")
            + "// @RNL { $D = @DEFINE(roles=[\"{ts}\"]) {}; {#T1:p1 #T1:n1 #T1:p2 #T1:n2 #T1:1} @AS $D }\n";
        let result = read(&input, None);
        assert_eq!(names(&result), named(&["p1", "n1", "p2", "n2"]));
        let members = &result.problems[0].usages[0].roles[0].members;
        assert_eq!(members.len(), 5);
        assert_eq!(members[0], members[4]);
    }
}

#[test]
fn subcircuit_ports_keep_the_names_of_the_definition() {
    let input = source("0.4", "line_cell", "")
        + "subckt line_cell left high right low\nR1 (left right) resistor r=1k\nends line_cell\n";
    assert_eq!(names(&read(&input, None)), named(&["left", "high", "right", "low"]));
}

#[test]
fn explicit_device_names_still_override_the_defaults() {
    for master in ["tline", "transformer"] {
        let header = "@DEVICE(name=\"T*\", terminals=[\"a\",\"b\",\"c\",\"d\"])";
        assert_eq!(names(&read(&source("0.4", master, header), None)), named(&["a", "b", "c", "d"]));
    }
}

#[test]
fn included_instances_get_the_defaults_too() {
    let main =
        "// main\nsimulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.4\") }\ninclude \"cells.scs\"\n";
    let child = source("0.4", "transformer", "");
    assert_eq!(names(&read(main, Some(&child))), named(&["p1", "n1", "p2", "n2"]));
}
