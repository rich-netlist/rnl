use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;

fn convert(statement: &str) -> rnl_edit::upgrade::Upgrade {
    let source = format!("* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std-logic\", version=\"0.3\") }}\n.subckt cell data clock output complement\n.ends\nX1 a b c d cell\n* @RNL {{ {statement} }}\n.end\n");
    upgrade("t.sp", &MemoryProvider::new().with("t.sp", &source), &UpgradeOptions::default()).unwrap()
}

#[test]
fn ordinal_standard_pins_keep_the_original_host_connections() {
    let r = convert("#X1 @SET-SYMBOL $D_FLIP_FLOP");
    assert!(
        r.files["t.sp"]
            .contains("@SET-SYMBOL(terminals=[[1, \"d\"], [2, \"ck\"], [3, \"q\"], [4, \"qb\"]]) $D_FLIP_FLOP"),
        "{}",
        r.files["t.sp"]
    );
    assert!(r.report.files[0].changes.iter().any(|c| c.rule == "standard-terminal-map"));
}

#[test]
fn a_selection_of_instances_keeps_ordinal_binding_for_each_member() {
    let r = convert("# @SELECT-INSTANCE(model=\"cell\") => @SET-SYMBOL $D_FLIP_FLOP");
    assert!(r.files["t.sp"].contains("recursive=true) => @SET-SYMBOL(terminals=[[1, \"d\"]"), "{}", r.files["t.sp"]);
}

#[test]
fn a_direct_set_of_hosts_uses_the_same_mapping() {
    let r = convert("{#X1} @SET-SYMBOL $D_FLIP_FLOP");
    assert!(r.report.files[0].changes.iter().any(|c| c.rule == "standard-terminal-map"));
}

#[test]
fn local_symbol_overrides_keep_their_own_bindings() {
    let r = convert("$D_FLIP_FLOP = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1) }; #X1 @SET-SYMBOL $D_FLIP_FLOP");
    assert!(!r.report.files[0].changes.iter().any(|c| c.rule == "standard-terminal-map"));
    assert!(r.files["t.sp"].contains("#X1 @SET-SYMBOL $D_FLIP_FLOP"));
}

#[test]
fn usage_role_binding_is_not_guessed_from_host_ordinals() {
    let r = convert("$D = @DEFINE(roles=[[\"p\",\"net\"]], parameters=[[\"name\",\"U\"]]) {}; %u=$D @ATTACH &a => @SET-SYMBOL $D_FLIP_FLOP");
    assert!(!r.report.files[0].changes.iter().any(|c| c.rule == "standard-terminal-map"));
}

#[test]
fn block_symbols_keep_the_subcircuit_port_order() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std-logic\", version=\"0.3\") }\n.subckt cell data clock output complement\n* @RNL { # @SET-SYMBOL $D_FLIP_FLOP }\n.ends\n.end\n";
    let r = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    assert!(r.report.files[0].changes.iter().any(|c| c.rule == "standard-terminal-map"));
}
