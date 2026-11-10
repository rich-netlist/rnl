use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;
fn check(statement: &str, rule: &str) {
    let source = format!(
        "* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\nR1 a 0 1k\n* @RNL {{ {statement} }}\n.end\n"
    );
    let source = if statement.starts_with("@DEVICE") {
        source
            .replace("@VERSION(\"0.3\")", &format!("@VERSION(\"0.3\"); {statement}"))
            .replace(&format!("* @RNL {{ {statement} }}\n"), "")
    } else {
        source
    };
    let r = upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", &source),
        &UpgradeOptions { std: false, ..UpgradeOptions::default() },
    )
    .unwrap();
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == rule), "{:?}", r.report);
    assert!(r.files["t.sp"].contains("/* rnl upgrade:"));
    assert!(!r.report.successful());
}
#[test]
fn absolute_definition_coordinates_need_an_origin_decision() {
    check("$D = @DEFINE(roles=[\"p\"]) { (1, 2) @TEXT(\"fixed\") }; %u = #R1 @AS $D", "definition-coordinates");
}
#[test]
fn definition_local_terminal_paths_require_expansion_context() {
    check("$D = @DEFINE(roles=[[\"p\",\"net\"]]) {}; $S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=\"p\") }; $OUT = @DEFINE(roles=[[\"p\",\"net\"]]) { %u=$D @ATTACH ?p => @SET-SYMBOL $S; ?p @WIRE {%u:p (_, _)} }; $OUT @ATTACH &a", "definition-references");
}
#[test]
fn held_constraint_sets_keep_their_centre_explanation() {
    check("{(0, 0) {(2, 1) (4, -1)}} @ALIGN(\"y\")", "constraint-items");
}
#[test]
fn set_distance_reports_its_extent_semantics() {
    check("{(0, 0) (1, 0)} {(4, 0) (5, 0)} @DISTANCE(\"left-to-right\", min=1)", "distance-sets");
}
#[test]
fn displaced_symbol_pins_are_reported_before_the_definition() {
    check("$S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (1, 0) @PIN(terminal=1) }", "symbol-pin-overrides");
}
#[test]
fn inherited_pin_overrides_need_fragment_review() {
    check(
        "$A = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1) }; $B = @DEFINE-SYMBOL { $A; (1, 0) @PIN(terminal=1) }",
        "symbol-pin-inheritance",
    );
}
#[test]
fn fixed_text_with_multiple_result_names_needs_contextual_rewriting() {
    check("$S = @DEFINE-SYMBOL { %t = (0, 0) @TEXT(\"fixed\") => %other; @LINE {%t (1, 0)} }", "symbol-text-result");
}
#[test]
fn repeated_local_symbol_names_need_renaming() {
    check("$S = @DEFINE-SYMBOL { %p=(0, 0); %p=(1, 0); %p @PIN(terminal=1) }", "symbol-local-names");
}
#[test]
fn numeric_device_names_are_not_silently_reinterpreted_as_ordinals() {
    check("@DEVICE(name=\"R*\", terminals=[\"2\",\"1\"])", "device-terminal-names");
}
#[test]
fn numeric_pin_names_require_assignment_specific_review() {
    check("$S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=\"01\") }; #R1 @SET-SYMBOL $S", "numeric-pin-name");
}
#[test]
fn newly_meaningful_markup_is_reported() {
    check(r#"(0, 0) @TEXT("\subdigits R10")"#, "text-meaning");
}

#[test]
fn selected_wire_members_need_an_explicit_routing_decision() {
    check("$S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (1, 0) @PIN(terminal=2) }; #R1 @SET-SYMBOL $S; &a @WIRE {# @SELECT-TERMINAL(net=\"a\"); (2, 0)}", "selection-dependent-creation");
}
#[test]
fn orientation_of_two_sets_needs_conflict_preservation() {
    check("$S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (1, 0) @PIN(terminal=2) }; #R1 @SET-SYMBOL $S; {#R1} {#R1} @ORIENT-RELATION(\"R0\")", "orientation-sets");
}

#[test]
fn an_unchanged_local_semantic_usage_needs_no_generic_review_note() {
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL { $D = @DEFINE(roles=[\"member\"]) {}; %u=#R1 @AS $D }\n.end\n";
    let r = upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", source),
        &UpgradeOptions { std: false, ..UpgradeOptions::default() },
    )
    .unwrap();
    assert!(r.report.files[0].unconverted.is_empty(), "{:?}", r.report);
}
#[test]
fn a_local_drawn_usage_reports_the_point_relation_it_would_lose() {
    check("$D = @DEFINE(roles=[[\"p\",\"point\"]]) {}; $S = @DEFINE-SYMBOL { (0, 0) @CIRCLE(radius=1) }; %u=$D @ATTACH (2, 3) => @SET-SYMBOL $S", "usage-main-point");
}
#[test]
fn removed_and_changed_standard_bodies_have_distinct_reasons() {
    for (statement, rule) in [
        ("%u=#R1(in_p) #R2(in_n) {}(tail) @AS $DIFFERENTIAL_PAIR", "standard-definition-removed"),
        ("%u={#R1}(ref) {#R2}(out) @AS $CURRENT_MIRROR", "standard-definition-body"),
    ] {
        let source=format!("* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\nR1 a 0 1k\nR2 b 0 1k\n* @RNL {{ {statement} }}\n.end\n");
        let r = upgrade("t.sp", &MemoryProvider::new().with("t.sp", &source), &UpgradeOptions::default()).unwrap();
        assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == rule), "{:?}", r.report);
    }
}

#[test]
fn standard_review_notes_follow_multiline_record_locations() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\nR2 b 0 1k\n* @RNL {\n*   #R1(in_p) #R2(in_n) {}(tail) @AS $DIFFERENTIAL_PAIR => %pair;\n* }\n.end\n";
    for path in ["t.sp", "/examples/nested/t.sp"] {
        let r = upgrade(path, &MemoryProvider::new().with(path, source), &UpgradeOptions::default()).unwrap();
        assert!(
            r.report.files[0].unconverted.iter().any(|u| u.rule == "standard-definition-removed"),
            "{:?}",
            r.report
        );
    }
}

#[test]
fn unplaced_symbol_pin_changes_are_compared() {
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL { $S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (2, 0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S }\n.end\n";
    let r = upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", source),
        &UpgradeOptions { std: false, ..UpgradeOptions::default() },
    )
    .unwrap();
    assert!(r.report.differences.iter().any(|d| d.category == "unplaced-drawings"));
}
#[test]
fn unchanged_unplaced_geometry_does_not_make_a_false_difference() {
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL { $S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (2, 0) @PIN(terminal=2) }; #R1 @SET-SYMBOL $S }\n.end\n";
    let r = upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", source),
        &UpgradeOptions { std: false, ..UpgradeOptions::default() },
    )
    .unwrap();
    assert!(r.report.successful(), "{:?}", r.report);
}
