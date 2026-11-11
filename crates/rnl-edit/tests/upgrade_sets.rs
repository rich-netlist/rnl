use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;

#[test]
fn default_net_labels_stay_off_and_explicit_whole_net_routes_win() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a b 1k\nR2 a b 2k\n* @RNL { {&a &b} @REMOVE-FROM $WIRE_NET_SET; &b @ROUTE; # @SELECT-TERMINAL(wired=false) => @TEXT(\"${net}\", key=\"netname\") }\n.end\n";
    let r = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    let text = &r.files["t.sp"];
    assert!(text.contains("# @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET"));
    assert!(text.contains("{&a} @UNROUTE-NET(priority=\"high\")"), "{text}");
    assert!(text.contains("&b @ROUTE-NET"));
    assert!(text.contains("@TEXT(\"${net}\", key=\"netname\")"));
}

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
    let policy = text.split("\r\n").find(|line| line.starts_with("* @RNL {")).unwrap();
    assert!(policy.contains("$RNL_UPGRADE_PORT_FILL_SET"), "{text}");
    assert!(policy.contains("@REMOVE-FROM $LABEL_FILL_SET"), "{text}");
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

#[test]
fn four_terminal_mos_retains_default_symbol_and_bulk_ground_supply_fill() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.model nch nmos\n.model pch pmos\nM1 d g s 0 nch\nM2 d g vdd vdd pch\n* @RNL { &vdd @ADD-TO $SUPPLY_NET_SET }\n.end\n";
    let r = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    let text = &r.files["t.sp"];
    assert!(text.contains("# @SELECT-INSTANCE(type=\"nmos\", recursive=true) => @SET-SYMBOL $NMOS_4TERM"), "{text}");
    assert!(text.contains("# @SELECT-INSTANCE(type=\"pmos\", recursive=true) => @SET-SYMBOL $PMOS_4TERM"), "{text}");
    assert!(text.contains("%rnl_upgrade_bulk_ground @ADD-TO $GROUND_FILL_SET"), "{text}");
    assert!(text.contains("%rnl_upgrade_bulk_supply @ADD-TO $SUPPLY_FILL_SET"), "{text}");
}

#[test]
fn three_terminal_mos_without_four_terminal_instances_adds_no_bulk_fill_rules() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.model nch nmos\nM1 d g 0 0 nch\n* @RNL { #M1 @SET-SYMBOL $NMOS }\n.end\n";
    let r = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    let text = &r.files["t.sp"];
    assert!(!text.contains("rnl_upgrade_bulk_"), "{text}");
    assert!(!text.contains("=> @SET-SYMBOL $NMOS_4TERM"), "{text}");
}
