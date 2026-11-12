use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;
fn convert(header: &str, statement: &str) -> rnl_edit::upgrade::Upgrade {
    let source = format!("* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\"); {header} }}\nR1 a 0 1k\n* @RNL {{ {statement} }}\n.end\n");
    upgrade("t.sp", &MemoryProvider::new().with("t.sp", &source), &UpgradeOptions::default()).unwrap()
}
#[test]
fn legacy_symbol_names_include_their_current_package() {
    let r = convert("", "#R1 @SET-SYMBOL $RES");
    assert!(r.files["t.sp"].contains("@SET-SYMBOL $RESISTOR"), "{}", r.files["t.sp"]);
    assert!(r.report.files[0].changes.iter().any(|c| c.rule == "legacy-standard-name"));
}
#[test]
fn old_package_collections_follow_their_merged_destination() {
    let r = convert("@USE-PACKAGE(name=\"rnl-std-virtual\", version=\"0.2\")", "$GND @ATTACH &0");
    assert!(r.files["t.sp"].contains("@USE-PACKAGE(name=\"rnl-std-basic\", version=\"0.4\")"));
    assert!(!r.files["t.sp"].contains("name=\"rnl-std-virtual\""));
    assert!(r.files["t.sp"].contains("$GROUND @ATTACH &0"));
}
#[test]
fn a_moved_symbol_gets_its_explicit_package_without_a_name_change() {
    let r = convert("", "#R1 @SET-SYMBOL $OPAMP");
    assert!(
        r.files["t.sp"].contains("@USE-PACKAGE(name=\"rnl-std-functional\", version=\"0.4\")"),
        "{}",
        r.files["t.sp"]
    );
    assert!(
        r.files["t.sp"]
            .contains("@SET-SYMBOL(terminals=[[1, \"inp\"], [2, \"inn\"], [3, \"out\"]]) $OPAMP => @ORIENT(\"MX\")"),
        "{}",
        r.files["t.sp"]
    );
}

#[test]
fn series_02_opamp_composes_mx_and_preserves_input_pin_positions() {
    let source = "* title\n\
* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n\
.subckt op a b y\n\
.ends\n\
X0 inp0 inn0 out0 op\n\
X1 inp1 inn1 out1 op\n\
X2 inp2 inn2 out2 op\n\
* @RNL {\n\
*   $FIG = @DEFINE(roles=[[\"p\", \"point\"]], parameters=[[\"name\", \"U\"]]) {};\n\
*   #X0 @SET-SYMBOL $OPAMP => @PLACE(0, 0);\n\
*   #X1 @SET-SYMBOL $OPAMP => @PLACE(20, 0) => @ORIENT(\"R90\");\n\
*   #X2 @SET-SYMBOL $OPAMP;\n\
*   #X2 @PLACE(40, 0) => @ORIENT(\"R270\");\n\
*   %fig = (0, -10) @AS $FIG => @SET-SYMBOL $OPAMP\n\
* }\n\
.end\n";
    let files = MemoryProvider::new().with("t.sp", source);
    let before =
        rnl_view::interpret_with("t.sp", None, &Default::default(), &files, &rnl_runtime::Options::default()).unwrap();
    let r = upgrade("t.sp", &files, &UpgradeOptions::default()).unwrap();
    let text = &r.files["t.sp"];
    assert!(
        text.contains("#X0 @SET-SYMBOL(terminals=[[1, \"inp\"], [2, \"inn\"], [3, \"out\"]]) $OPAMP => @ORIENT(\"MX\") => @PLACE(0, 0)"),
        "{text}"
    );
    assert!(
        text.contains("#X1 @SET-SYMBOL(terminals=[[1, \"inp\"], [2, \"inn\"], [3, \"out\"]]) $OPAMP => @PLACE(20, 0) => @ORIENT(\"MXR90\")"),
        "{text}"
    );
    assert!(text.contains("#X2 @SET-SYMBOL(terminals=[[1, \"inp\"], [2, \"inn\"], [3, \"out\"]]) $OPAMP;"), "{text}");
    assert!(text.contains("#X2 @PLACE(40, 0) => @ORIENT(\"MYR90\")"), "{text}");
    assert!(text.contains("%fig = (0, -10) @AS $FIG => @SET-SYMBOL $OPAMP => @ORIENT(\"MX\")"), "{text}");

    let after_files = MemoryProvider::new().with("t.sp", text);
    let after = rnl_view::interpret_with(
        "t.sp",
        None,
        &Default::default(),
        &after_files,
        &rnl_runtime::Options { std: false, ..rnl_runtime::Options::default() },
    )
    .unwrap();
    for id in ["#X0", "#X1", "#X2"] {
        let b = before.model.levels[0].objects.iter().find(|o| o.id == id).unwrap();
        let a = after.model.levels[0].objects.iter().find(|o| o.id == id).unwrap();
        assert_eq!(a.pins[0].at, b.pins[0].at, "{id} inp");
        assert_eq!(a.pins[0].nets, b.pins[0].nets, "{id} inp nets");
        assert_eq!(a.pins[1].at, b.pins[1].at, "{id} inn");
        assert_eq!(a.pins[1].nets, b.pins[1].nets, "{id} inn nets");
        assert_eq!(a.pins[2].nets, b.pins[2].nets, "{id} out nets");
    }
}
#[test]
fn local_old_and_new_names_are_not_captured_by_standard_renames() {
    for local in ["RES", "RESISTOR"] {
        let r = convert("", &format!("${local} = @DEFINE-SYMBOL {{ (0, 0) @PIN(terminal=1) }}; #R1 @SET-SYMBOL $RES"));
        assert!(r.files["t.sp"].contains("@SET-SYMBOL $RES"));
        assert!(!r.report.files[0].changes.iter().any(|c| c.rule == "legacy-standard-name"));
    }
}
#[test]
fn a_renamed_definition_is_not_described_as_missing() {
    let r = convert("", "$GND @ATTACH &0");
    assert!(!r.report.files[0].unconverted.iter().any(|u| u.rule == "standard-definition-removed"));
    assert!(r.files["t.sp"].contains("$GROUND @ATTACH &0"));
}

#[test]
fn inline_merged_package_overrides_remain_explicit_review_cases() {
    let r = convert("@USE-PACKAGE(name=\"rnl-std-layout\", version=\"0.2\") {}", "");
    assert!(r.files["t.sp"].contains("@USE-PACKAGE(name=\"rnl-std-layout\", version=\"0.2\") {}"));
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "legacy-package-override"));
}
#[test]
fn removed_legacy_sets_are_kept_with_their_policy_review_reason() {
    let r = convert("", "$WIRE_NET_SET @ROUTE");
    assert!(r.files["t.sp"].contains("$WIRE_NET_SET"));
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "standard-name-removed"));
}
#[test]
fn aliases_are_reported_as_a_context_boundary() {
    let r = convert("@USE-ALIASES(name=\"rnl-zh\", version=\"0.2\")", "");
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "alias-context"));
}
