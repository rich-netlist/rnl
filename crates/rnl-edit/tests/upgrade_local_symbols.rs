use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;
fn convert(statement: &str) -> rnl_edit::upgrade::Upgrade {
    let source = format!(
        "* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\nR1 a 0 1k\n* @RNL {{ {statement} }}\n.end\n"
    );
    upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", &source),
        &UpgradeOptions { std: false, ..UpgradeOptions::default() },
    )
    .unwrap()
}
#[test]
fn pin_result_names_keep_naming_their_input_points() {
    let r = convert("$S = @DEFINE-SYMBOL { %p=(0, 0) @PIN(terminal=1); @LINE {%p (1, 0)} }; #R1 @SET-SYMBOL $S");
    assert!(r.files["t.sp"].contains("%p=(0, 0) ; %rnl_upgrade_local_1 = %p @PIN(terminal=1)"), "{}", r.files["t.sp"]);
    assert!(r.report.files[0].unconverted.is_empty(), "{:?}", r.report);
    assert!(r.report.successful(), "{:?}", r.report);
}
#[test]
fn fixed_text_result_names_keep_naming_their_input_points() {
    let r = convert("$S = @DEFINE-SYMBOL { %p=(0, 0) @TEXT(\"caption\"); @LINE {%p (1, 0)} }; #R1 @SET-SYMBOL $S");
    assert!(r.files["t.sp"].contains("%p=(0, 0) ; %rnl_upgrade_local_1 = %p @TEXT"));
    assert!(!r.report.files[0].unconverted.iter().any(|u| u.rule == "symbol-text-result"));
    assert!(r.report.successful(), "{:?}", r.report);
}
#[test]
fn custom_usage_terminals_become_the_new_local_pin_names() {
    let r = convert("$D = @DEFINE(roles=[[\"p\",\"net\"]]) {}; $S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=\"p\") }; %u=$D @ATTACH &a => @SET-SYMBOL $S; &a @WIRE {%u:p (2, 0)}");
    assert!(r.files["t.sp"].contains("%rnl_upgrade_local_1 = (0, 0) @PIN"), "{}", r.files["t.sp"]);
    assert!(r.files["t.sp"].contains("@WIRE {%u::rnl_upgrade_local_1 (2, 0)}"));
}
#[test]
fn fresh_local_names_do_not_capture_existing_names() {
    let r = convert("$S = @DEFINE-SYMBOL { %rnl_upgrade_local_1=(0, 0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S");
    assert!(r.files["t.sp"].contains("%rnl_upgrade_local_2 = %rnl_upgrade_local_1 @PIN"));
}
#[test]
fn fresh_pin_names_avoid_owner_roles_and_text_keys() {
    for body in [
        "$D = @DEFINE(roles=[[\"rnl_upgrade_local_1\",\"net\"]]) {}; $S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=\"rnl_upgrade_local_1\") }; $D @ATTACH &a => @SET-SYMBOL $S",
        "$S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S; #R1 @TEXT(\"label\", key=\"rnl_upgrade_local_1\")",
    ] {
        let r = convert(body);
        assert!(r.files["t.sp"].contains("%rnl_upgrade_local_2 = (0, 0) @PIN"), "{}", r.files["t.sp"]);
        assert!(!r.report.diagnostics.iter().any(|d| d.name.starts_with("pin-name-is-")), "{:?}", r.report);
    }
}
#[test]
fn repeated_pin_bindings_stay_unmodified_for_manual_review() {
    for second in ["1", "1.0"] {
        let body = format!("$S = @DEFINE-SYMBOL {{ (0, 0) @PIN(terminal=1); (1, 0) @PIN(terminal={second}) }}");
        let r = convert(&format!("{body}; #R1 @SET-SYMBOL $S"));
        assert!(r.files["t.sp"].contains(&body));
        assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "symbol-pin-overrides"));
    }
}

#[test]
fn unresolved_numeric_usage_pins_are_preserved_with_a_specific_note() {
    let r = convert("$D = @DEFINE(roles=[[\"p\",\"net\"]]) {}; $S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1) }; %u=$D @ATTACH &a => @SET-SYMBOL $S; &a @WIRE {%u:p (2, 0)}");
    assert!(r.files["t.sp"].contains("%u:p (2, 0)}"));
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "usage-pin"));
}

#[test]
fn definition_local_names_do_not_resolve_to_scene_usages() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL { $D = @DEFINE(roles=[[\"p\",\"net\"]]) {}; $S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=\"p\") }; $OUT = @DEFINE(roles=[[\"p\",\"net\"]]) { %g=$D @ATTACH ?p => @SET-SYMBOL $S; ?p @WIRE {%g:p (0, 0)} }; %g=$GROUND @ATTACH &0; %outer=$OUT @ATTACH &a; &0 @WIRE {%g:p (1, 0)} }\n.end\n";
    let r = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    assert!(r.files["t.sp"].contains("@WIRE {%g:p (0, 0)}"), "{}", r.files["t.sp"]);
    assert!(r.files["t.sp"].contains("@WIRE {%g::pin (1, 0)}"));
}

#[test]
fn mixed_numeric_and_named_bindings_do_not_guess_the_old_role_pin() {
    let r = convert("$D = @DEFINE(roles=[[\"p\",\"net\"]]) {}; $S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=\"p\"); (1, 0) @PIN(terminal=1) }; %u=$D @ATTACH &a => @SET-SYMBOL $S; &a @WIRE {%u:p (2, 0)}");
    assert!(r.files["t.sp"].contains("%u:p (2, 0)}"));
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "usage-pin"));
}
