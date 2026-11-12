//! The standard collection follows the language, and has the first root position.
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn source(language: &str, header: &str, body: &str) -> String {
    format!("* standard collection\n* @RNL(type=\"header\") {{ @VERSION(\"{language}\"); {header} }}\nR1 a b 1k\n* @RNL {{ {body} }}\n")
}

fn files(language: &str, version: &str, header: &str, body: &str) -> MemoryProvider {
    let name = if language == "0.4" { "rnl-std-all" } else { "rnl-std" };
    let mut provider = MemoryProvider { package_dirs: vec!["/pkg".into()], ..MemoryProvider::new() }
        .with("/t.sp", &source(language, header, body));
    provider.insert(&format!("/pkg/{version}/{name}.rnl"),&format!(
        "@RNL(type=\"header\") {{ @VERSION(\"{language}\"); @PACKAGE(name=\"{name}\",version=\"{version}\") }}\n@RNL {{ $D = @DEFINE(roles=[\"r\"]) {{}} }}\n"));
    provider
}

fn read(files: &MemoryProvider, options: Options) -> Interpretation {
    rnl_cli::interpret_file(Path::new("/t.sp"), None, files, &options).unwrap()
}

fn no_errors(r: &Interpretation) {
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
}

#[test]
fn a_setting_selects_the_collection_by_language_not_package_version() {
    for language in ["0.2", "0.3", "0.4"] {
        let provider = files(language, "1.7", "", "#R1 @AS $D");
        let r = read(&provider, Options { include_std: true, std_version: Some("1.7".into()), ..Options::solver() });
        no_errors(&r);
        assert_eq!(r.problems[0].usages.len(), 1);
        assert_eq!(
            rnl_runtime::interp::standard_package_name(language),
            if language == "0.4" { "rnl-std-all" } else { "rnl-std" }
        );
    }
}

#[test]
fn a_custom_version_is_saved_with_the_formal_collection_name() {
    let mut provider = files("0.4", "1.7", "", "#R1 @AS $D");
    let r = read(&provider, Options { include_std: true, ..Options::solver() });
    no_errors(&r);
    assert_eq!(r.included_standard.as_deref(), Some("1.7"));
    let text = source("0.4", "", "#R1 @AS $D");
    let saved = rnl_runtime::writeback::with_standard_package(&r.doc, &text, "1.7");
    assert!(saved.contains(r#"@USE-PACKAGE(name="rnl-std-all", version="1.7")"#), "{saved}");
    assert_eq!(saved.matches("@USE-PACKAGE").count(), 1);
    provider.insert("/t.sp", &saved);
    let reread = read(&provider, Options { std: false, ..Options::solver() });
    no_errors(&reread);
    assert_eq!(rnl_runtime::writeback::with_standard_package(&reread.doc, &saved, "1.7"), saved);
}

#[test]
fn an_explicit_standard_is_first_at_the_root_in_either_written_order() {
    let standard = r#"@USE-PACKAGE(name="rnl-std-all",version="1.7")"#;
    let custom = r#"@USE-PACKAGE(name="custom",version="1.0")"#;
    for header in [format!("{standard}; {custom}"), format!("{custom}; {standard}")] {
        let mut provider = files("0.4", "1.7", &header, "$D @ATTACH");
        provider.insert(
            "/pkg/1.0/custom.rnl",
            r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="custom",version="1.0") }
@RNL { $D = @DEFINE(roles=[]) {} }"#,
        );
        let r = read(&provider, Options { std: false, ..Options::solver() });
        no_errors(&r);
        assert_eq!(r.problems[0].usages.len(), 1);
        let hiding = r.diagnostics().into_iter().find(|d| d.code == codes::PACKAGE_HIDES_PACKAGE).unwrap();
        assert!(
            hiding.message.contains("package custom hides") && hiding.message.contains("standard package rnl-std-all"),
            "{}",
            hiding.message
        );
    }
}

#[test]
fn root_position_does_not_change_the_nearest_reference() {
    let mut provider = files("0.4", "0.4", r#"@USE-PACKAGE(name="rnl-std-all",version="0.4")"#, "#R1 @AS $D");
    let main = source(
        "0.4",
        r#"@USE-PACKAGE(name="rnl-std-all",version="0.4")"#,
        r#"@USE-PACKAGE(name="rnl-std-all",version="0.4"); #R1 @AS $D"#,
    );
    provider.insert("/t.sp", &(main + ".include child.sp\nX1 a b cell\n"));
    provider.insert(
        "/child.sp",
        r#"* child
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all",version="1.7") }
.subckt cell a b
R2 a b 1k
* @RNL { $D @ATTACH }
.ends cell
"#,
    );
    provider.insert(
        "/pkg/1.7/rnl-std-all.rnl",
        r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="rnl-std-all",version="1.7") }
@RNL { $D = @DEFINE(roles=[]) {} }"#,
    );
    no_errors(&read(&provider, Options { std: false, ..Options::solver() }));
}

#[test]
fn a_package_body_falls_back_to_the_formal_standard_environment() {
    let mut provider = files(
        "0.4",
        "1.7",
        r#"@USE-PACKAGE(name="rnl-std-all",version="1.7"); @USE-PACKAGE(name="custom",version="1.0")"#,
        "#R1 @AS $WRAPPER",
    );
    provider.insert(
        "/pkg/1.0/custom.rnl",
        r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="custom",version="1.0") }
@RNL { $WRAPPER = @DEFINE(roles=["r"]) { ?r @AS $D } }"#,
    );
    let r = read(&provider, Options { std: false, ..Options::solver() });
    no_errors(&r);
    assert_eq!(r.problems[0].usages.len(), 2);
}

#[test]
fn an_explicit_empty_collection_suppresses_the_setting() {
    let provider = files("0.4", "1.7", r#"@USE-PACKAGE(name="rnl-std-all") {}"#, "#R1 @PLACE(0,0)");
    let r = read(&provider, Options { include_std: true, ..Options::solver() });
    no_errors(&r);
    assert_eq!(r.included_standard, None);
}

#[test]
fn missing_standard_reports_the_requested_collection_and_modern_header_location() {
    for language in ["0.3", "0.4"] {
        let provider = files(language, "1.7", "", "#R1 @PLACE(0,0)");
        let r = read(&provider, Options { include_std: true, std_version: Some("9.9".into()), ..Options::solver() });
        let matching: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::STD_VERSION_NOT_FOUND).collect();
        assert_eq!(matching.len(), 1, "{:?}", r.diagnostics());
        if language == "0.4" {
            assert!(matching[0].message.contains("rnl-std-all 9.9"));
            let loc = matching[0].loc.as_ref().unwrap();
            assert_eq!((loc.file.as_str(), loc.line), ("t.sp", 2));
        } else {
            assert!(matching[0].message.contains("rnl-std 9.9"));
            assert!(matching[0].loc.is_none());
        }
    }
}

#[test]
fn unused_standard_name_hints_follow_the_formal_collection() {
    let provider = files("0.4", "1.7", "", "#R1 @AS $D");
    let r = read(&provider, Options::solver());
    let matching: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::STANDARD_PACKAGE_NOT_USED).collect();
    assert_eq!(matching.len(), 1, "{:?}", r.diagnostics());
    assert!(matching[0].message.contains("package rnl-std-all"));
}

#[test]
fn the_setting_rejects_an_alias_only_collection_once_for_its_root_reference() {
    let mut provider = files("0.4", "1.7", "", "#R1 @PLACE(0,0)");
    let header = r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="rnl-std-all",version="1.7") }"#;
    provider.insert("/pkg/1.7/rnl-std-all.rnl", header);
    let main = source("0.4", "", "#R1 @PLACE(0,0)") + ".include child.sp\nX1 a b cell\n";
    provider.insert("/t.sp", &main);
    provider.insert(
        "/child.sp",
        r#"* child
* @RNL(type="header") { @VERSION("0.4") }
.subckt cell a b
R2 a b 1k
.ends cell
"#,
    );
    let r = read(&provider, Options { include_std: true, ..Options::solver() });
    let matching: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::USE_PACKAGE_ALIAS_PACKAGE).collect();
    assert_eq!(matching.len(), 1, "{:?}", r.diagnostics());
    assert!(matching[0].message.contains("rnl-std-all 1.7"));
    let loc = matching[0].loc.as_ref().unwrap();
    assert_eq!((loc.file.as_str(), loc.line), ("t.sp", 2));
    assert_eq!(r.included_standard, None);
    provider.insert("/pkg/1.7/rnl-std-all.rnl", &format!("{header}\n@RNL {{}}\n"));
    let ordinary = read(&provider, Options { include_std: true, ..Options::solver() });
    no_errors(&ordinary);
    assert_eq!(ordinary.included_standard.as_deref(), Some("1.7"));
}

#[test]
fn a_package_language_failure_precedes_alias_kind_validation() {
    let mut provider = files("0.4", "1.7", "", "#R1 @PLACE(0,0)");
    provider.insert(
        "/pkg/1.7/rnl-std-all.rnl",
        r#"@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="rnl-std-all",version="1.7") }"#,
    );
    let r = read(&provider, Options { include_std: true, std_version: Some("1.7".into()), ..Options::solver() });
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert_eq!(errors.len(), 1, "{:?}", r.diagnostics());
    assert_eq!(errors[0].code, codes::PACKAGE_LANGUAGE_MISMATCH);
    assert_eq!(r.included_standard, None);
}
