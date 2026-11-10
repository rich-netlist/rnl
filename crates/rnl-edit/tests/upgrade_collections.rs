use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;
fn convert(statement: &str) -> rnl_edit::upgrade::Upgrade {
    let source=format!("* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\nR1 a 0 1k\nR2 b 0 1k\n* @RNL {{ {statement} }}\n.end\n");
    upgrade("t.sp", &MemoryProvider::new().with("t.sp", &source), &UpgradeOptions::default()).unwrap()
}
#[test]
fn a_named_set_position_is_reported_at_its_use() {
    let r = convert("%s={#R1 #R2}; %s::position @TEXT(\"centre\")");
    assert!(r.files["t.sp"].contains("%s::position @TEXT(\"centre\")"));
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "set-position"));
}
#[test]
fn a_named_set_distance_keeps_the_old_extent_constraint() {
    let r = convert("%s={#R1 #R2}; %s (5, 0) @DISTANCE(\"left-to-right\", min=2, max=4)");
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "distance-sets"));
}
#[test]
fn collective_placement_is_not_silently_applied_to_each_member() {
    let r = convert("%s={#R1 #R2}; %s @PLACE(2, 3)");
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "set-placement"));
}
#[test]
fn selection_dependencies_survive_named_sets_and_aliases() {
    let r = convert("%s=# @SELECT-TERMINAL(net=\"a\"); %t={%s}; %alias=%t; &a @WIRE {%alias (2, 0)}");
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "selection-dependent-creation"));
}
#[test]
fn ordinary_instance_positions_are_not_reported_as_set_positions() {
    let r = convert("%instance=#R1; %instance::position @TEXT(\"point\")");
    assert!(!r.report.files[0].unconverted.iter().any(|u| u.rule == "set-position"));
}
