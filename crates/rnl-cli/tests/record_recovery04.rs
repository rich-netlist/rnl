//! A broken member delimiter does not make a complete draft record fail (2.4, 7.4).
use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(files: &MemoryProvider, name: &str) -> Interpretation {
    interpret_file(Path::new(name), None, files, &Options { std: false, ..Options::solver() }).unwrap()
}
fn source(version: &str, body: &str) -> String {
    format!("* recovery\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a 0 1k\n* @RNL {{ (0,0) @TEXT(\"before\"); {body}; (3,0) @TEXT(\"after\") }}\n* @RNL {{ (7,0) @TEXT(\"next\") }}\n.end\n")
}
fn labels(result: &Interpretation) -> Vec<String> {
    result.model.levels.iter().flat_map(|level| &level.texts).map(|text| text.markup.clone()).collect()
}
fn read_body(version: &str, body: &str) -> Interpretation {
    read(&MemoryProvider::new().with("t.sp", &source(version, body)), "t.sp")
}
#[test]
fn member_delimiters_preserve_the_other_members() {
    for body in [
        r#"#R1 @PLACE(1,2"#,
        r#"(1,2 @TEXT("bad")"#,
        r#"#R1 @ORIENT(["R0""#,
        r#"#R1(left @TEXT("bad")"#,
        r#"#R[1:2 @TEXT("bad")"#,
    ] {
        let result = read_body("0.4", body);
        assert_eq!(labels(&result), ["before", "after", "next"], "{body}: {:?}", result.diagnostics());
        assert_eq!(
            result.diagnostics().iter().filter(|d| d.severity == Severity::Error).count(),
            1,
            "{body}: {:?}",
            result.diagnostics()
        );
        assert!(!result
            .diagnostics()
            .iter()
            .any(|d| matches!(d.code, codes::RECORD_NEW_BEFORE_END | codes::RECORD_CUT_OFF)));
    }
}
#[test]
fn incomplete_strings_comments_and_record_braces_still_fail_the_whole_record() {
    for body in [r#"#R1 @TEXT("open"#, "/* unclosed", r#"{#R1 @TEXT("bad")"#] {
        let result = read_body("0.4", body);
        assert_eq!(labels(&result), ["next"], "{body}: {:?}", result.diagnostics());
        assert_eq!(
            result.diagnostics().iter().filter(|d| d.severity == Severity::Error).count(),
            1,
            "{body}: {:?}",
            result.diagnostics()
        );
    }
}
#[test]
fn the_version_declaration_survives_an_unclosed_later_header_member() {
    let text="* header\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @DEVICE(name=\"unused\"; }\nR1 a 0 1k\n* @RNL { (0,0) @TEXT(\"before\"); #R1 @PLACE(1,2; (3,0) @TEXT(\"after\") }\n.end\n";
    let result = read(&MemoryProvider::new().with("t.sp", text), "t.sp");
    assert_eq!(labels(&result), ["before", "after"], "{:?}", result.diagnostics());
    assert_eq!(result.problems[0].version, "0.4");
}
#[test]
fn inherited_versions_apply_even_when_the_include_precedes_the_header() {
    for version in ["0.3", "0.4"] {
        let main =
            format!("* inheritance\n.include inc.sp\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n.end\n");
        let child="* child\n.subckt child a\nR1 a 0 1k\n* @RNL { (0,0) @TEXT(\"before\"); #R1 @PLACE(1,2; (3,0) @TEXT(\"after\") }\n.ends\n";
        let result = read(&MemoryProvider::new().with("t.sp", &main).with("inc.sp", child), "t.sp");
        assert_eq!(
            labels(&result),
            if version == "0.4" { vec!["before", "after"] } else { vec![] },
            "{version}: {:?}",
            result.diagnostics()
        );
    }
}
#[test]
fn an_include_uses_its_own_explicit_version() {
    for (outer, inner) in [("0.3", "0.4"), ("0.4", "0.3")] {
        let main = format!("* main\n* @RNL(type=\"header\") {{ @VERSION(\"{outer}\") }}\n.include inc.sp\n.end\n");
        let child = source(inner, "#R1 @PLACE(1,2");
        let result = read(&MemoryProvider::new().with("t.sp", &main).with("inc.sp", &child), "t.sp");
        let child_records = result.doc.levels.iter().find(|level| level.file == "inc.sp").unwrap().records.len();
        assert_eq!(child_records, if inner == "0.4" { 3 } else { 2 }, "{outer}/{inner}: {:?}", result.diagnostics());
    }
}
#[test]
fn package_records_use_the_same_member_boundary() {
    let package = r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="p",version="1.0") }
@RNL { $D = @DEFINE(roles=["i"]) {}; @TEXT("broken"; $E = @DEFINE(roles=["i"]) {} }
"#;
    let main="* package\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"p\",version=\"1.0\",path=\"p.rnl\") }\nR1 a 0 1k\n* @RNL { #R1 @AS $D; #R1 @AS $E }\n.end\n";
    let result = read(&MemoryProvider::new().with("t.sp", main).with("p.rnl", package), "t.sp");
    assert_eq!(result.problems[0].usages.len(), 2, "{:?}", result.diagnostics());
}
#[test]
fn spectre_and_spice_sections_share_the_versioned_scanner() {
    let text="// recovery\n// @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 (a 0) resistor r=1k\n// @RNL { (0,0) @TEXT(\"spectre\"); #R1 @PLACE(1,2; (3,0) @TEXT(\"after-spectre\") }\nsimulator lang=spice\nR2 a 0 1k\n* @RNL { (0,1) @TEXT(\"spice\"); #R2 @PLACE(1,2; (3,1) @TEXT(\"after-spice\") }\n";
    let result = read(&MemoryProvider::new().with("t.scs", text), "t.scs");
    assert_eq!(labels(&result), ["spectre", "after-spectre", "spice", "after-spice"], "{:?}", result.diagnostics());
}

#[test]
fn two_readings_of_one_file_can_have_different_inherited_versions() {
    let files=MemoryProvider::new()
        .with("t.sp","* main\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include old.sp\n.include new.sp\n.end\n")
        .with("old.sp","* old\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.include shared.sp\n")
        .with("new.sp","* new\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include shared.sp\n")
        .with("shared.sp","* shared\n* @RNL { (0,0) @TEXT(\"before\"); @TEXT(\"broken\"; (3,0) @TEXT(\"after\") }\n* @RNL { (7,0) @TEXT(\"next\") }\n");
    let result = read(&files, "t.sp");
    let counts: Vec<_> =
        result.doc.levels.iter().filter(|level| level.file == "shared.sp").map(|level| level.records.len()).collect();
    assert_eq!(counts, [1, 2], "{:?}", result.diagnostics());
    assert_eq!(result.diagnostics().iter().filter(|d| d.code == codes::RECORD_NEW_BEFORE_END).count(), 1);
}
#[test]
fn valid_nested_sets_and_parameter_strings_keep_their_record_boundaries() {
    let result = read_body("0.4", r#"$D = @DEFINE(roles=["i"]) { (0,0) @TEXT("{) ; }(") }; #R1 @AS $D"#);
    assert!(!result.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", result.diagnostics());
    assert_eq!(result.problems[0].usages.len(), 1);
    assert!(labels(&result).contains(&"after".to_string()));
    assert_eq!(result.doc.levels[0].records.len(), 3);
}
#[test]
fn a_duplicate_version_does_not_change_the_first_version() {
    let text = source("0.4", "#R1 @PLACE(1,2").replace("@VERSION(\"0.4\")", "@VERSION(\"0.4\"); @VERSION(\"0.3\")");
    let result = read(&MemoryProvider::new().with("t.sp", &text), "t.sp");
    assert_eq!(labels(&result), ["before", "after", "next"], "{:?}", result.diagnostics());
    assert_eq!(result.problems[0].version, "0.4");
}
