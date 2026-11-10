use rnl_edit::upgrade::{upgrade, Upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;
fn convert(statement: &str, std: bool) -> Upgrade {
    let source = format!("* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\nR1 a 0 1k\nR2 b 0 2k\n* @RNL {{ {statement} }}\n.end\n");
    upgrade("t.sp", &MemoryProvider::new().with("t.sp", &source), &UpgradeOptions { std, ..UpgradeOptions::default() })
        .unwrap()
}
fn pending(r: &Upgrade, rule: &str) -> bool {
    r.report.files.iter().flat_map(|f| &f.unconverted).any(|u| u.rule == rule)
}
#[test]
fn directed_nonnegative_set_minima_are_equivalent() {
    for direction in ["left-to-right", "right-to-left", "top-to-bottom", "bottom-to-top"] {
        for slots in ["{#R1 #R2} (8, 9)", "%a %b"] {
            let r = convert(
                &format!("%a={{#R1 #R2}}; %b={{(8, 9) (12, 20)}}; {slots} @DISTANCE(\"{direction}\", min=2)"),
                false,
            );
            assert!(!pending(&r, "distance-sets"), "{:?}", r.report);
        }
    }
}
#[test]
fn a_singleton_on_one_side_does_not_make_a_maximum_equivalent() {
    for bounds in ["max=2", "exact=2", "min=-1", "min=0, max=2"] {
        let r = convert(&format!("{{#R1 #R2}} (8, 9) @DISTANCE(\"left-to-right\", {bounds})"), false);
        assert!(pending(&r, "distance-sets"), "{:?}", r.report);
    }
    let r = convert("{#R1} {(8, 9)} @DISTANCE(\"left-to-right\", exact=2)", false);
    assert!(!pending(&r, "distance-sets"), "{:?}", r.report);
    let r = convert("{#R1} {(8, 9)} @DISTANCE(\"left-to-right\", max=2)", false);
    assert!(pending(&r, "distance-sets"));
}
#[test]
fn standard_point_marks_already_restore_the_main_relation() {
    let r = convert("%p=(2, 3); $JUNCTION @ATTACH %p; $ARROW @ATTACH %p", true);
    assert!(!pending(&r, "usage-main-point"), "{:?}", r.report);
    assert!(!pending(&r, "standard-definition-body"), "{:?}", r.report);
}
#[test]
fn assertions_are_informational_but_violations_still_fail() {
    let r = convert("$IO_PORT @ATTACH &a", true);
    assert!(!pending(&r, "standard-definition-body"), "{:?}", r.report);
    assert!(r.report.files[0].notices.iter().any(|n| n.rule == "standard-assertion"), "{:?}", r.report);
    assert!(!r.report.successful());
}
#[test]
fn old_stub_defaults_are_explicit_and_authored_values_survive() {
    let r = convert("#R1:1 @AS $GROUND_STUB; #R2:1 @AS(gap=7) $SUPPLY_STUB", true);
    assert!(r.files["t.sp"].contains("@AS(gap=2) $GROUND_STUB"), "{}", r.files["t.sp"]);
    assert!(r.files["t.sp"].contains("@AS(gap=7) $SUPPLY_STUB"));
    assert!(!pending(&r, "standard-definition-body"), "{:?}", r.report);
}
#[test]
fn local_drawn_usages_gain_alignments_without_copying_points() {
    let r=convert("$D= @DEFINE(roles=[[\"p\",\"point\"]]) {}; $S= @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=1) }; %p=(2,3); %u=$D @ATTACH %p => @SET-SYMBOL $S",false);
    assert!(r.files["t.sp"].contains("{%u %u::p} @ALIGN(\"x\")"), "{}", r.files["t.sp"]);
    assert!(r.files["t.sp"].contains("{%u %u::p} @ALIGN(\"y\")"));
    assert!(!pending(&r, "usage-main-point"));
}
#[test]
fn new_drawing_facts_in_mirrors_remain_manual() {
    let r = convert("{#R1}(ref) {#R2}(out) @AS $CURRENT_MIRROR", true);
    let note = r.report.files[0].unconverted.iter().find(|u| u.rule == "standard-definition-body").unwrap();
    assert!(note.reason.contains("ALIGN y"));
}

#[test]
fn singleton_literal_groups_need_no_centre_decision() {
    for call in ["ALIGN(\"x\")", "ORDER(\"left-to-right\")"] {
        let r = convert(&format!("{{{{#R1}} {{#R2}}}} @{call}"), false);
        assert!(!pending(&r, "constraint-items"), "{:?}", r.report.files[0]);
    }
}
#[test]
fn anonymous_main_point_names_do_not_capture_existing_spelling() {
    let r=convert("$D = @DEFINE(roles=[[\"p\",\"point\"]]) {}; $S = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=1) }; %rnl_upgrade_usage_0=(7,8); $D @ATTACH (2,3) => @SET-SYMBOL $S",false);
    assert!(r.files["t.sp"].contains("=> %rnl_upgrade_usage_1"), "{}", r.files["t.sp"]);
    assert!(r.files["t.sp"].contains("{%rnl_upgrade_usage_1 %rnl_upgrade_usage_1::p}"));
    assert!(!r.report.diagnostics.iter().any(|d| d.severity == "error"), "{:?}", r.report.diagnostics);
}
#[test]
fn ground_on_a_terminal_preserves_the_electrical_net_and_point() {
    let r = convert("%g=$GROUND @ATTACH #R1:1", true);
    assert!(r.files["t.sp"].contains("@ATTACH #R1:1::net"), "{}", r.files["t.sp"]);
    assert!(r.files["t.sp"].contains("{%g #R1:1} @ALIGN(\"x\")"));
    assert!(!pending(&r, "usage-main-point"));
}
