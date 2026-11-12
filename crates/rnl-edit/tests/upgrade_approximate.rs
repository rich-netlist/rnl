use rnl_edit::upgrade::{upgrade, Upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;
fn convert(statement: &str, approximate: bool) -> Upgrade {
    let source=format!("* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\nR1 a 0 1k\nR2 b 0 1k\n* @RNL {{ {statement} }}\n.end\n");
    upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", &source),
        &UpgradeOptions { std: false, approximate, ..UpgradeOptions::default() },
    )
    .unwrap()
}
#[test]
fn approximate_directions_require_opt_in_and_record_the_source_of_the_choice() {
    let s = "{(4, 0) (2, 0) (0, 0)} @DISTRIBUTE(\"x\", priority=\"low\")";
    let exact = convert(s, false);
    assert!(exact.files["t.sp"].contains(s));
    assert!(exact.report.files[0].approximations.is_empty());
    let r = convert(s, true);
    assert!(r.files["t.sp"].contains("@DISTRIBUTE(\"right-to-left\", priority=\"low\")"), "{}", r.files["t.sp"]);
    assert!(r.report.files[0].approximations[0].risk.contains("existing source positions"));
    assert!(r.report.files[0].approximations[0].risk.contains("zero step"));
    assert!(r.report.files[0].unconverted.is_empty());
    assert!(r.report.files[0].changes.iter().all(|c| !c.rule.starts_with("approximate-")));
}
#[test]
fn a_directed_authored_witness_precedes_written_order() {
    let r = convert("{#R1 #R2} @ORDER(\"right-to-left\"); {#R1 #R2} @ORDER(\"x\")", true);
    assert_eq!(r.files["t.sp"].matches("@ORDER(\"right-to-left\")").count(), 2, "{}", r.files["t.sp"]);
    assert!(r.report.files[0].approximations[0].risk.contains("authored directed"));
}
#[test]
fn missing_or_conflicting_witnesses_use_the_documented_fallback() {
    for (axis, direction) in [("x", "left-to-right"), ("y", "top-to-bottom")] {
        let r = convert(&format!("{{#R1 #R2}} @ORDER(\"{axis}\")"), true);
        assert!(r.files["t.sp"].contains(&format!("@ORDER(\"{direction}\")")));
        assert!(r.report.files[0].approximations[0].risk.contains("written member order"));
    }
}
#[test]
fn held_order_is_reported_as_stronger_but_align_and_distribute_remain_manual() {
    let r = convert("{{#R1 #R2} (10, 0)} @ORDER(\"left-to-right\")", true);
    assert!(r.report.files[0].unconverted.is_empty(), "{:?}", r.report.files[0]);
    assert!(r.report.files[0].approximations.iter().any(|a| a.rule == "approximate-order-sets"));
    for call in ["ALIGN(\"x\")", "DISTRIBUTE(\"x\")"] {
        let r = convert(&format!("{{{{#R1 #R2}} (10, 0) (20, 0)}} @{call}"), true);
        assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "constraint-items"));
    }
}
#[test]
fn directed_set_minimum_after_choosing_an_axis_is_not_a_second_manual_choice() {
    let r = convert("{#R1 #R2} (10,0) @DISTANCE(\"x\", min=1)", true);
    assert!(r.report.files[0].unconverted.is_empty(), "{:?}", r.report.files[0]);
    let r = convert("{#R1 #R2} (10,0) @DISTANCE(\"x\", max=1)", true);
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "distance-sets"));
}
fn pair(models: &str, second: &str, approximate: bool) -> Upgrade {
    let source=format!("* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n{models}\nM1 a ip tail 0 first w=1u l=1u\nM2 b in tail 0 {second} w=1u l=1u\n* @RNL {{ %pair = #M1(in_p) #M2(in_n) {{}}(tail) @AS $DIFFERENTIAL_PAIR; %pair::in_p @TEXT(\"input\") }}\n.end\n");
    upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", &source),
        &UpgradeOptions { approximate, ..UpgradeOptions::default() },
    )
    .unwrap()
}
#[test]
fn pair_replacement_uses_both_resolved_model_polarities() {
    for (model, suffix) in [("nmos", "N"), ("pmos", "P")] {
        let models = format!(".model first {model}\n.model second {model}");
        let r = pair(&models, "second", true);
        assert!(r.files["t.sp"].contains(&format!("@AS $DIFFERENTIAL_PAIR_{suffix}")), "{}", r.files["t.sp"]);
        assert!(r.files["t.sp"].contains("#M1(left) #M2(right)"));
        assert!(r.files["t.sp"].contains("%pair::left"));
        assert!(r.report.files[0].approximations.iter().any(|a| a.rule == "approximate-differential-pair"));
        let r = pair(&models, "second", false);
        assert!(!r.files["t.sp"].contains(&format!("@AS $DIFFERENTIAL_PAIR_{suffix}")));
    }
}
#[test]
fn mismatched_or_unknown_polarities_are_never_guessed_from_names() {
    for (models, second) in
        [(".model first nmos\n.model second pmos", "second"), (".model first nmos", "nmos_like_name")]
    {
        let r = pair(models, second, true);
        assert!(!r.files["t.sp"].contains("$DIFFERENTIAL_PAIR_N"));
        assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "standard-definition-removed"));
    }
}

#[test]
fn definition_directions_use_only_their_own_role_witnesses() {
    let r=convert("$D = @DEFINE(roles=[\"a\",\"axis\",\"b\"]) { {?a ?b} @ORDER(\"bottom-to-top\"); {?a ?axis ?b} @DISTRIBUTE(\"y\") }; %a=(0,4); %b=(0,0); %a(a) (0,2)(axis) %b(b) @AS $D",true);
    assert!(r.files["t.sp"].contains("@DISTRIBUTE(\"bottom-to-top\")"), "{}", r.files["t.sp"]);
    assert!(r.report.files[0].approximations.iter().any(|a| a.risk.contains("same definition body")));
    assert!(!r.report.files[0].unconverted.iter().any(|u| u.rule == "direction"));
}

#[test]
fn spectre_model_polarity_is_read_from_the_host() {
    let source="// title\nsimulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.3\") }\nmodel input_device bsim4 type=p\nM1 (a ip tail 0) input_device w=1u l=1u\nM2 (b in tail 0) input_device w=1u l=1u\n// @RNL { #M1(in_p) #M2(in_n) {}(tail) @AS $DIFFERENTIAL_PAIR }\n";
    let r = upgrade(
        "t.scs",
        &MemoryProvider::new().with("t.scs", source),
        &UpgradeOptions { approximate: true, ..UpgradeOptions::default() },
    )
    .unwrap();
    assert!(r.files["t.scs"].contains("$DIFFERENTIAL_PAIR_P"));
    assert!(r.files["t.scs"].contains("model input_device bsim4 type=p\nM1 (a ip tail 0) input_device w=1u l=1u"));
}

fn layout(statement: &str, approximate: bool) -> Upgrade {
    let source = format!(
        "* layout migration\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n.model channel nmos\nM1 a ip tail 0 channel\nM2 b in tail 0 channel\nR1 a b 1k\n* @RNL {{ {statement} }}\n.end\n"
    );
    upgrade(
        "layout.sp",
        &MemoryProvider::new().with("layout.sp", &source),
        &UpgradeOptions { approximate, ..UpgradeOptions::default() },
    )
    .unwrap()
}

#[test]
fn differential_pair_and_mirror_agree_on_sides_and_role_references() {
    let r = layout("%axis = (_, _); %mirror = #M1(a) #M2(b) %axis(axis) @AS $MIRRORED; %pair = #M2(in_p) #M1(in_n) @AS $DIFFERENTIAL_PAIR; %pair::in_p @TEXT(\"input\")", true);
    let text = &r.files["layout.sp"];
    assert!(text.contains("#M1(left) #M2(right) %axis(axis) @AS $MIRRORED"), "{text}");
    assert!(text.contains("#M2(right) #M1(left) @AS $DIFFERENTIAL_PAIR_N"), "{text}");
    assert!(text.contains("%pair::right @TEXT"));
    assert!(r.report.files[0].unconverted.is_empty(), "{:?}", r.report.files[0].unconverted);
}

#[test]
fn authored_directions_and_x_only_placement_cannot_disagree() {
    let r = layout(
        "#M1 @PLACE(x=0); #M2 @PLACE(x=2); {#M1 #M2} @ORDER(\"right-to-left\"); #M1 #M2 @AS $DIFFERENTIAL_PAIR",
        true,
    );
    assert!(r.files["layout.sp"].contains("#M1 #M2 @AS $DIFFERENTIAL_PAIR"));
    assert!(!r.files["layout.sp"].contains("$DIFFERENTIAL_PAIR_N"));
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "pair-direction-conflict"));
}

#[test]
fn reversed_named_inputs_without_a_witness_follow_the_written_order() {
    let r = layout("#M2(in_n) #M1(in_p) @AS $DIFFERENTIAL_PAIR", true);
    assert!(r.files["layout.sp"].contains("#M2(left) #M1(right)"), "{}", r.files["layout.sp"]);
    assert!(r.report.files[0].approximations.iter().any(|a| a.risk.contains("written argument order")));
}

#[test]
fn flow_keeps_the_usage_and_restores_its_effective_direction_and_priority() {
    let r = layout("%path = {#M1 #R1 #M2} @AS(direction=\"right-to-left\", priority=\"high\") $FLOW", false);
    let text = &r.files["layout.sp"];
    assert!(text.contains("%path = {#M1 #R1 #M2} @AS(priority=\"high\") $FLOW"), "{text}");
    assert!(!text.contains("direction="), "{text}");
    assert!(text.contains("{#M1 #R1 #M2} @ORDER(\"right-to-left\", priority=\"high\")"), "{text}");
    assert!(r.report.files[0].unconverted.is_empty(), "{:?}", r.report.files[0].unconverted);
    assert!(
        r.report
            .diagnostics
            .iter()
            .all(|d| d.severity != "error" || d.file.as_deref().is_some_and(|f| f.ends_with(".rnl"))),
        "{:?}",
        r.report.diagnostics
    );
}

#[test]
fn current_mirror_additions_require_opt_in() {
    let statement = "#M1(ref) #M2(out) @AS $CURRENT_MIRROR";
    let exact = layout(statement, false);
    assert!(exact.report.files[0].unconverted.iter().any(|u| u.rule == "standard-definition-removed"));
    let approximate = layout(statement, true);
    assert!(
        approximate.files["layout.sp"].contains("#M1(ref) #M2(out) @AS $CURRENT_MIRROR_N"),
        "{}",
        approximate.files["layout.sp"]
    );
    assert!(approximate.report.files[0].unconverted.is_empty());
    assert!(approximate.report.files[0].approximations.iter().any(|a| a.rule == "approximate-current-mirror"));
}

#[test]
fn current_mirror_with_unknown_or_mixed_polarity_stays_manual() {
    let r = layout("#R1(ref) #R1(out) @AS $CURRENT_MIRROR", true);
    assert!(r.files["layout.sp"].contains("@AS $CURRENT_MIRROR"));
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "standard-definition-removed"));
}

#[test]
fn singleton_set_tail_is_unwrapped_while_multi_member_tail_stays_manual() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.model nch nmos\nM1 a ip tail 0 nch\nM2 b in tail 0 nch\nM5 tail vb 0 0 nch\nM6 tail vb 0 0 nch\n* @RNL { #M1(in_p) #M2(in_n) {#M5}(tail) @AS $DIFFERENTIAL_PAIR }\n.end\n";
    let r = upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", source),
        &UpgradeOptions { approximate: true, ..UpgradeOptions::default() },
    )
    .unwrap();
    assert!(r.files["t.sp"].contains("#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N"), "{}", r.files["t.sp"]);
    assert!(r.report.files[0].unconverted.is_empty());

    let multi = source.replace("{#M5}(tail)", "{#M5 #M6}(tail)");
    let r_multi = upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", &multi),
        &UpgradeOptions { approximate: true, ..UpgradeOptions::default() },
    )
    .unwrap();
    assert!(r_multi.report.files[0].unconverted.iter().any(|u| u.rule == "standard-definition-removed"));
}

#[test]
fn mirrored_net_axis_stays_manual_even_with_approximate() {
    for statement in [
        "#M1(a) #M2(b) #R1:1(axis) @AS $MIRRORED",
        "#M1(a) #M2(b) {#R1 (0, 0)}(axis) @AS $MIRRORED",
        "#M1(a) #M2(b) &a(axis) @AS(priority=\"disabled\") $MIRRORED",
    ] {
        let r = layout(statement, true);
        assert!(
            r.report.files[0].unconverted.iter().any(|u| u.rule == "standard-definition-body"),
            "{statement}: {:?}",
            r.report.files[0]
        );
    }
}

#[test]
fn top_level_io_port_becomes_net_label_only_with_approximate() {
    let source = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.subckt buf a y\nR1 a y 1k\n* @RNL { $IO_PORT @ATTACH &a; $IO_PORT @ATTACH &y }\n.ends\nX1 vin vout buf\n* @RNL { $IO_PORT @ATTACH &vout => %pout }\n.end\n";
    let exact = upgrade("t.sp", &MemoryProvider::new().with("t.sp", source), &UpgradeOptions::default()).unwrap();
    assert!(exact.files["t.sp"].contains("$IO_PORT @ATTACH &vout => %pout"));
    assert_eq!(exact.report.assertion_violations.len(), 1);

    let approx = upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", source),
        &UpgradeOptions { approximate: true, ..UpgradeOptions::default() },
    )
    .unwrap();
    let text = &approx.files["t.sp"];
    assert!(text.contains("$IO_PORT @ATTACH &a; $IO_PORT @ATTACH &y"), "{text}");
    assert!(text.contains("$NET_LABEL @ATTACH &vout => %pout"), "{text}");
    assert!(approx.report.assertion_violations.is_empty(), "{:?}", approx.report.assertion_violations);
    assert!(approx.report.files[0].approximations.iter().any(|a| a.rule == "approximate-top-level-io-port"));
}
