use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;

#[test]
fn a_standard_usage_terminal_becomes_its_named_pin() {
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL { %g = $GROUND @ATTACH &0; %g @PLACE(0, 0); &0 @WIRE {%g:p (1, 0)} }\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    assert!(result.files["t.sp"].contains("@WIRE {%g::pin (1, 0)}"), "{}", result.files["t.sp"]);
    assert!(result.report.files[0].changes.iter().any(|c| c.rule == "usage-named-pin"));
}

#[test]
fn standard_numbered_and_inherited_pins_resolve_without_a_solved_position() {
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL { %g = $GROUND @ATTACH &0 => @SET-SYMBOL $NAMED_GROUND_SYMBOL; &0 @WIRE {%g:1 (1, 0)} }\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    assert!(result.files["t.sp"].contains("@WIRE {%g::pin (1, 0)}"), "{}", result.files["t.sp"]);
}

#[test]
fn a_local_definition_with_a_standard_name_does_not_get_a_standard_pin() {
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL { $GROUND = @DEFINE(roles=[[\"p\", \"net\"]]) {}; $GROUND_SYMBOL = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=\"p\") }; %g = $GROUND @ATTACH &0 => @SET-SYMBOL $GROUND_SYMBOL; &0 @WIRE {%g:p (1, 0)} }\n.end\n";
    let result = upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", source),
        &UpgradeOptions { std: false, ..UpgradeOptions::default() },
    )
    .unwrap();
    assert!(result.files["t.sp"].contains("@WIRE {%g:p (1, 0)}"));
    assert!(!result.report.files[0].changes.iter().any(|c| c.rule == "usage-named-pin"));
}

#[test]
fn standard_drawing_role_and_its_references_follow_the_new_name() {
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n* @RNL { %point=(0, 0); %a=%point(p) @AS $ARROW; %a::p @TEXT(\"note\") }\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    assert!(result.files["t.sp"].contains("%point(at) @AS $ARROW"), "{}", result.files["t.sp"]);
    assert!(result.files["t.sp"].contains("%a::at @TEXT(\"note\")"));
}

#[test]
fn standard_mirrored_roles_keep_their_members() {
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\nR2 b 0 1k\n* @RNL { %pair=#R1(a) #R2(b) (0, 0)(axis) @AS $MIRRORED; %pair::a @TEXT(\"left\") }\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    assert!(
        result.files["t.sp"].contains("#R1(left) #R2(right) (0, 0)(axis) @AS $MIRRORED"),
        "{}",
        result.files["t.sp"]
    );
    assert!(result.files["t.sp"].contains("%pair::left @TEXT(\"left\")"));
}

#[test]
fn a_local_role_definition_shadows_the_standard_mapping() {
    let source="* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n* @RNL { $ARROW = @DEFINE(roles=[[\"p\",\"point\"]]) {}; %point=(0, 0); %a=%point(p) @AS $ARROW; %a::p @TEXT(\"note\") }\n.end\n";
    let result = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    assert!(result.files["t.sp"].contains("%point(p) @AS $ARROW"));
    assert!(result.files["t.sp"].contains("%a::p @TEXT(\"note\")"));
    assert!(!result.report.files[0].changes.iter().any(|c| c.rule.starts_with("standard-role")));
}
