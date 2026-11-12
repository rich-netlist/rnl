use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;

fn options() -> UpgradeOptions {
    UpgradeOptions { std: false, ..UpgradeOptions::default() }
}

#[test]
fn header_and_fixed_points_keep_netlist_and_crlf() {
    let source = "* title\r\n* @RNL(type=\"header\") { /* Ω */ @VERSION(\"0.3\") }\r\nR1 a 0 1k\r\n* @RNL { %p = (3, 4); %p @TEXT(\"note\") }\r\n.end\r\n";
    let files = MemoryProvider::new().with("/w/top.sp", source);
    let result = upgrade("/w/top.sp", &files, &options()).unwrap();
    assert_eq!(result.files["/w/top.sp"], source.replace("@VERSION(\"0.3\")", "@VERSION(\"0.4\")"));
    assert!(result.report.successful(), "{:?}", result.report);
    assert_eq!(result.report.files[0].changes[0].line, 2);
}

#[test]
fn missing_header_is_inserted_after_title() {
    let files = MemoryProvider::new().with("/w/top.sp", "* title\nR1 a 0 1k\n.end\n");
    let result = upgrade("/w/top.sp", &files, &options()).unwrap();
    assert!(
        result.files["/w/top.sp"].starts_with("* title\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a 0 1k\n")
    );
    assert!(result.report.successful(), "{:?}", result.report);
}

#[test]
fn includes_use_their_own_declared_or_inherited_versions() {
    let files = MemoryProvider::new()
        .with("/w/top.sp", "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.include child.inc\n.include old.inc\n.end\n")
        .with("/w/child.inc", "* child\n.subckt child a\n* @RNL { (1, 2) @TEXT(\"child\") }\n.ends\n")
        .with("/w/old.inc", "* old\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n.subckt old a\n* @RNL { (2, 3) @TEXT(\"old\") }\n.ends\n");
    let result = upgrade("/w/top.sp", &files, &options()).unwrap();
    assert_eq!(result.files.len(), 3);
    assert_eq!(result.report.files.iter().find(|f| f.path.ends_with("old.inc")).unwrap().from, "0.2");
    assert!(result.report.successful(), "{:?}", result.report);
}

#[test]
fn an_invalid_source_has_no_proposed_output() {
    let files = MemoryProvider::new().with("t.sp", "* title\n* @RNL { #missing @PLACE(1, 2) }\n.end\n");
    assert!(upgrade("t.sp", &files, &options()).unwrap_err().contains("nothing was written"));
}

#[test]
fn standard_package_is_explicit_and_pinned_to_target_series() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    assert!(result.files["t.sp"].contains("@USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\")"));
    assert!(result.report.files[0].changes.iter().any(|c| c.rule == "explicit-header"));
    // An incomplete target core must never turn a syntactic conversion into verified success.
    if result.report.diagnostics.iter().any(|d| d.severity == "error") {
        assert!(!result.report.successful());
    }
}

#[test]
fn explicit_empty_standard_override_is_preserved() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\") {} }\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    assert_eq!(result.files["t.sp"].matches("@USE-PACKAGE").count(), 1);
    assert!(result.files["t.sp"].contains("name=\"rnl-std-all\", version=\"0.4\") {}"));
    assert!(result.report.successful(), "{:?}", result.report);
}

#[test]
fn explicit_03_package_names_map_to_their_04_counterparts() {
    for (old, expected) in [
        ("@USE-PACKAGE(name=\"rnl-std-full\", version=\"0.3\")", "name=\"rnl-std-all\", version=\"0.4\""),
        ("@USE-PACKAGE(name=\"rnl-std-core\", version=\"0.3\")", "name=\"rnl-std-all\", version=\"0.4\""),
        ("@USE-PACKAGE(name=\"rnl-std-structures\", version=\"0.3\")", "name=\"rnl-std-semantics\", version=\"0.4\""),
        ("@USE-ALIASES(name=\"rnl-zh\", version=\"0.3\")", "name=\"alias-zh\", version=\"0.4\""),
    ] {
        let source = format!("* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"); {old} }}\nR1 a 0 1k\n.end\n");
        let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", &source), &options()).unwrap();
        assert!(result.files["t.sp"].contains(expected), "{}", result.files["t.sp"]);
    }
}

#[test]
fn spectre_comments_and_host_statements_stay_intact() {
    let source = "// title\nsimulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 (a 0) resistor r=1k\n// @RNL { (1, 2) @TEXT(\"Ω\") }\n";
    let result = upgrade("t.scs", &MemoryProvider::new().with("t.scs", source), &options()).unwrap();
    assert_eq!(result.files["t.scs"], source.replace("@VERSION(\"0.3\")", "@VERSION(\"0.4\")"));
    assert!(result.report.successful(), "{:?}", result.report);
}

#[test]
fn target_errors_are_reported_and_prevent_success() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n* @RNL { (0, 0) (1, 0) @DISTANCE(\"x\", min=1) }\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &options()).unwrap();
    assert!(!result.report.successful());
    assert!(result.report.diagnostics.iter().any(|d| d.severity == "error"));
    assert!(!result.report.differences.is_empty());
}

#[test]
fn explicit_custom_packages_are_upgraded_with_the_group() {
    let files = MemoryProvider::new()
        .with("/w/top.sp", "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"custom\", path=\"custom.rnl\") }\n.end\n")
        .with("/w/custom.rnl", "@RNL(type=\"header\") { @VERSION(\"0.3\"); @PACKAGE(name=\"custom\", version=\"1.0\"); @USE-PACKAGE(name=\"nested\", path=\"nested.rnl\") }\n")
        .with("/w/nested.rnl", "@RNL(type=\"header\") { @VERSION(\"0.2\"); @PACKAGE(name=\"nested\", version=\"1.0\") }\n@RNL {}\n");
    let result = upgrade("/w/top.sp", &files, &options()).unwrap();
    assert_eq!(result.files.len(), 3);
    assert!(result.files["/w/custom.rnl"].contains("@VERSION(\"0.4\")"));
    assert!(result.files["/w/nested.rnl"].contains("@VERSION(\"0.4\")"));
    assert!(result.report.successful(), "{:?}", result.report);
}

#[test]
fn a_path_package_is_not_replaced_just_because_its_name_is_standard() {
    let files = MemoryProvider::new()
        .with("/w/top.sp", "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std-basic\", version=\"0.3\", path=\"owned.rnl\") }\n.end\n")
        .with("/w/owned.rnl", "@RNL(type=\"header\") { @VERSION(\"0.3\"); @PACKAGE(name=\"rnl-std-basic\", version=\"0.3\") }\n@RNL { $OWN = @DEFINE(roles=[\"member\"]) {} }\n");
    let result = upgrade("/w/top.sp", &files, &options()).unwrap();
    assert!(result.files["/w/top.sp"].contains("version=\"0.3\", path=\"owned.rnl\""));
    assert!(result.files["/w/owned.rnl"].contains("@VERSION(\"0.4\")"));
    assert!(result.files["/w/owned.rnl"].contains("@PACKAGE(name=\"rnl-std-basic\", version=\"0.3\")"));
    assert!(result.files["/w/owned.rnl"].contains("$OWN = @DEFINE"));
}

#[test]
fn a_second_upgrade_has_no_changes() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n* @RNL { (1, 2) @TEXT(\"note\") }\n.end\n";
    let first = upgrade("top.sp", &MemoryProvider::new().with("top.sp", source), &options()).unwrap();
    let files = MemoryProvider { files: first.files, package_dirs: Vec::new() };
    let second = upgrade("top.sp", &files, &options()).unwrap();
    assert!(second.files.is_empty());
    assert!(second.report.successful(), "{:?}", second.report);
}

#[test]
fn a_package_entry_uses_its_own_header() {
    let source = "@RNL(type=\"header\") { @VERSION(\"0.3\"); @PACKAGE(name=\"custom\", version=\"1.0\") }\n";
    let result = upgrade("custom.rnl", &MemoryProvider::new().with("custom.rnl", source), &options()).unwrap();
    assert_eq!(result.files["custom.rnl"], source.replace("0.3", "0.4"));
    assert!(result.report.successful(), "{:?}", result.report);
}

#[test]
fn solver_records_are_discarded_and_request_solving_without_a_conversion_failure() {
    let source="* title\r\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\r\n* @RNL { %p = (_, _); %p @TEXT(\"authored\") }\r\n* @RNL(type=\"generated\") {\r\n* %p @PLACE(3, 4)\r\n* }\r\nR1 a 0 1k\r\n.end\r\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &options()).unwrap();
    let output = &result.files["t.sp"];
    assert!(!output.contains("generated"));
    assert!(!output.contains("@PLACE(3, 4)"));
    assert!(output.contains("%p = (_, _); %p @TEXT(\"authored\")"));
    assert!(output.ends_with("R1 a 0 1k\r\n.end\r\n"));
    assert_eq!(output.lines().count(), source.lines().count());
    assert!(result.report.files[0].needs_solve);
    assert!(result.report.files[0].changes.iter().any(|c| c.rule == "discard-solver-record" && c.line == 4));
    assert!(result.report.successful(), "{:?}", result.report);
}

#[test]
fn a_malformed_legacy_package_version_is_not_reported_as_a_successful_upgrade() {
    let source = r#"@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="custom", version="1") }"#;
    let result = upgrade("custom.rnl", &MemoryProvider::new().with("custom.rnl", source), &options()).unwrap();
    assert!(!result.report.successful());
    assert!(result.report.diagnostics.iter().any(|d| d.code.as_deref() == Some("RNL2219")));
    assert!(result.files["custom.rnl"].contains(r#"version="1""#));
}
