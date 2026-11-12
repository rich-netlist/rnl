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

#[test]
fn renamed_03_symbols_and_inherited_pins_preserve_terminal_mappings() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std-full\", version=\"0.3\") }\n.subckt op5 a b vp vn y\n.ends\n.subckt sum x1 x2 y\n.ends\n.subckt dly i o\n.ends\nXop a b vdd 0 out op5\nXsum a b s sum\nXdly s y dly\nE1 p1 0 c1 0 2\nE2 p2 0 c2 0 1\nG1 i1 0 c1 0 1m\n* @RNL {\n*   #Xop @SET-SYMBOL $OPAMP_5TERM;\n*   #Xsum @SET-SYMBOL $ADDER;\n*   #Xdly @SET-SYMBOL $DELAY_CELL_SYMBOL;\n*   #E1 @SET-SYMBOL $VCVS;\n*   #E2 @SET-SYMBOL $VCVS_4TERM;\n*   #G1 @SET-SYMBOL $VCCS;\n*   (0, 0) @AS $FIGURE => @SET-SYMBOL $CURRENT_ARROW_SYMBOL\n* }\n.end\n";
    let r = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    let text = &r.files["t.sp"];
    assert!(text.contains("#Xop @SET-SYMBOL(terminals=[[1, \"inp\"], [2, \"inn\"], [5, \"out\"]]) $OPAMP"), "{text}");
    assert!(
        text.contains("#Xsum @SET-SYMBOL(terminals=[[1, \"in1\"], [2, \"in2\"], [3, \"out\"]]) $ADDER_SYMBOL"),
        "{text}"
    );
    assert!(text.contains("#Xdly @SET-SYMBOL(terminals=[[1, \"a\"], [2, \"y\"]]) $DELAY_CELL"), "{text}");
    assert!(text.contains("#E1 @SET-SYMBOL(terminals=[[1, \"p\"], [2, \"n\"]]) $CONTROLLED_VOLTAGE_SOURCE"), "{text}");
    assert!(
        text.contains("#E2 @SET-SYMBOL(terminals=[[1, \"p\"], [2, \"n\"], [3, \"cp\"], [4, \"cn\"]]) $VCVS_4TERM"),
        "{text}"
    );
    assert!(text.contains("#G1 @SET-SYMBOL(terminals=[[1, \"p\"], [2, \"n\"]]) $CONTROLLED_CURRENT_SOURCE"), "{text}");
    assert!(text.contains("=> @SET-SYMBOL $ARROW_SYMBOL"), "{text}");
}
