use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;
#[test]
fn quoted_numeric_port_names_keep_their_original_positions() {
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.subckt child 3 1 2\n.ends\nX1 a b c child\n* @RNL { $S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (1, 0) @PIN(terminal=2); (2, 0) @PIN(terminal=3) }; #X1 @SET-SYMBOL $S; &b @WIRE {#X1:\"1\" (0, 0)} }\n.end\n";
    let r = upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", source),
        &UpgradeOptions { std: false, ..UpgradeOptions::default() },
    )
    .unwrap();
    assert!(r.files["t.sp"].contains("@WIRE {#X1:2 (0, 0)}"));
    assert!(r.files["t.sp"].contains(".subckt child 3 1 2"));
    assert!(r.report.successful(), "{:?}", r.report);
}
#[test]
fn changed_spectre_default_terminal_names_use_the_host_order() {
    let source="simulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.3\") }\nT (a b c d) transformer n1=1 n2=1\n// @RNL { $S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (1, 0) @PIN(terminal=2); (2, 0) @PIN(terminal=3); (3, 0) @PIN(terminal=4) }; #T @SET-SYMBOL $S; &a @WIRE {#T:p_plus (0, 0)} }\n";
    let r = upgrade(
        "t.scs",
        &MemoryProvider::new().with("t.scs", source),
        &UpgradeOptions { std: false, ..UpgradeOptions::default() },
    )
    .unwrap();
    assert!(r.files["t.scs"].contains("@WIRE {#T:1 (0, 0)}"));
    assert!(r.files["t.scs"].contains("T (a b c d) transformer n1=1 n2=1"));
}
