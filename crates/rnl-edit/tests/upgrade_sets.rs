use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;

#[test]
fn removed_routing_set_retains_its_selection_and_user_edits() {
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL { &a @REMOVE-FROM $WIRE_NET_SET; $WIRE_NET_SET @ROUTE }\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    let text = &result.files["t.sp"];
    assert!(text.contains("$RNL_UPGRADE_WIRE_NET_SET = # @SELECT-NET(recursive=true)"), "{text}");
    assert!(text.contains("&a @REMOVE-FROM $RNL_UPGRADE_WIRE_NET_SET"));
    assert!(text.contains("$RNL_UPGRADE_WIRE_NET_SET @ROUTE-NET"));
    assert!(!text.contains(" $WIRE_NET_SET"));
}

#[test]
fn old_port_fill_is_explicit_without_adding_port_list_layout_facts() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.subckt child a\nR1 a 0 1k\n.ends\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    let text = &result.files["t.sp"];
    assert!(text.contains("$RNL_UPGRADE_PORT_FILL_SET = # @SELECT-NET(port=true, recursive=true)"), "{text}");
    assert!(text.contains("$IO_PORT @ATTACH $RNL_UPGRADE_PORT_FILL_SET"));
    assert!(!text.contains("$PORT_LIST"));
}

#[test]
fn user_defined_routing_sets_keep_their_own_identity() {
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL { $WIRE_NET_SET = {&a}; $WIRE_NET_SET @ROUTE }\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    assert!(result.files["t.sp"].contains("$WIRE_NET_SET = {&a}; $WIRE_NET_SET @ROUTE-NET"));
    assert!(!result.files["t.sp"].contains("$RNL_UPGRADE_WIRE_NET_SET"));
}

#[test]
fn disabled_standard_fill_gets_no_replacement_policy() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std-fill\", version=\"0.3\") {} }\nR1 a 0 1k\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    assert!(!result.files["t.sp"].contains("RNL_UPGRADE_"));
}

#[test]
fn fresh_set_names_do_not_capture_existing_names() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL { $RNL_UPGRADE_WIRE_NET_SET = {}; $WIRE_NET_SET @ROUTE }\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    let text = &result.files["t.sp"];
    assert!(text.contains("$RNL_UPGRADE_WIRE_NET_SET = {}"));
    assert!(text.contains("$RNL_UPGRADE_WIRE_NET_SET_2 = # @SELECT-NET"));
    assert!(text.contains("$RNL_UPGRADE_WIRE_NET_SET_2 @ROUTE-NET"));
}

#[test]
fn new_policy_without_a_header_keeps_host_lines_and_crlf() {
    let source = "* title\r\n.subckt child a\r\nR1 a 0 1k\r\n.ends\r\n.end\r\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    let text = &result.files["t.sp"];
    assert!(text.contains("\r\n* @RNL { $RNL_UPGRADE_PORT_FILL_SET"), "{text}");
    assert!(!text.replace("\r\n", "").contains('\n'));
    let host = |s: &str| s.lines().filter(|l| !l.starts_with("* @RNL")).collect::<Vec<_>>().join("\n");
    assert_eq!(host(source), host(text));
}

#[test]
fn included_set_edits_use_the_single_entry_policy() {
    let provider = MemoryProvider::new()
        .with("t.sp", "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.include child.inc\n.end\n")
        .with("child.inc", "* child\n.subckt child a\nR1 a 0 1k\n* @RNL { &a @REMOVE-FROM $WIRE_NET_SET; $WIRE_NET_SET @ROUTE }\n.ends\n");
    let result = upgrade("t.sp", &provider, &UpgradeOptions::default()).unwrap();
    assert!(result.files["t.sp"].contains("$RNL_UPGRADE_WIRE_NET_SET = #"));
    assert!(result.files["child.inc"].contains("&a @REMOVE-FROM $RNL_UPGRADE_WIRE_NET_SET"));
    assert!(!result.files["child.inc"].contains("$RNL_UPGRADE_WIRE_NET_SET ="));
}
