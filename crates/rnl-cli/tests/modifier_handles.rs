//! A failed fact does not invalidate the input handle in language 0.4 (5.5, 6.6).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(version: &str, body: &str) -> Interpretation {
    let source = format!("* modifiers\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ #R1 @PLACE(0,0); #R2 @PLACE(10,0); {body} }}\n.end\n");
    interpret_file(
        Path::new("modifiers.sp"),
        None,
        &MemoryProvider::new().with("modifiers.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn errors(r: &Interpretation) -> usize {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).count()
}

#[test]
fn first_segment_parameter_failures_keep_the_existing_object() {
    for modifier in [
        "@PLACE(\"bad\",0)",
        "@ORIENT(\"bad\")",
        "@LAYER(\"bad\")",
        "@PRIORITY(\"bad\")",
        "@SET-SYMBOL(terminals=0) $S",
        "@SET-STYLE(bad=1) $STYLE",
    ] {
        let r = read("0.4", &format!("$S = @DEFINE-SYMBOL {{ (0,0) @CIRCLE(radius=1) }}; $STYLE = @DEFINE-STYLE(linecolor=\"#ff0000\") {{}}; %saved = #R1 {modifier}; %saved @TEXT(\"kept\",key=\"ok\")"));
        assert_eq!(errors(&r), 1, "{modifier}: {:?}", r.diagnostics());
        assert!(r.model.levels[0].texts.iter().any(|t| t.markup == "kept"), "{modifier}");
    }
}

#[test]
fn failed_current_keeps_its_terminal_handle() {
    let r = read("0.4", "%saved = #R1:1 @CURRENT(\"bad\"); %saved @TEXT(\"terminal\",key=\"ok\")");
    assert_eq!(errors(&r), 1, "{:?}", r.diagnostics());
    assert!(r.model.levels[0].texts.iter().any(|t| t.markup == "terminal"));
}

#[test]
fn failed_modifiers_do_not_release_a_valid_name_for_rebinding() {
    let r = read("0.4", "%saved = #R1 @PLACE(\"bad\",0); %saved = #R2; %saved @TEXT(\"first\",key=\"ok\")");
    assert_eq!(errors(&r), 2, "{:?}", r.diagnostics());
    let text = r.model.levels[0].texts.iter().find(|t| t.markup == "first").unwrap();
    assert_eq!(text.attach.as_deref(), Some("#R1"));
}

#[test]
fn explicit_set_identity_survives_failed_parameters() {
    let r = read("0.4", "%targets = {#R1 #R2}; %saved = %targets @ORIENT(\"bad\"); %targets = %saved; %saved @TEXT(\"both\",key=\"ok\")");
    assert_eq!(errors(&r), 1, "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].texts.iter().filter(|t| t.markup == "both").count(), 2);
}

#[test]
fn loose_inputs_form_the_recovered_set() {
    let r = read("0.4", "%saved = #R1 #R2 @PLACE(\"bad\",0); %saved @TEXT(\"both\",key=\"ok\")");
    assert_eq!(errors(&r), 1, "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].texts.iter().filter(|t| t.markup == "both").count(), 2);
}

#[test]
fn a_failed_add_or_remove_passes_the_target_instead_of_the_pipe_source() {
    for modifier in ["ADD-TO", "REMOVE-FROM"] {
        let r = read("0.4", &format!("%targets = {{#R2}}; %saved = #R1 => @{modifier}(bad=1) %targets; %targets = %saved; %saved @TEXT(\"target\",key=\"ok\")"));
        assert_eq!(errors(&r), 1, "{modifier}: {:?}", r.diagnostics());
        let texts: Vec<_> = r.model.levels[0].texts.iter().filter(|t| t.markup == "target").collect();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].attach.as_deref(), Some("#R2"));
    }
}

#[test]
fn failed_group_parameters_keep_the_original_set() {
    let r = read(
        "0.4",
        "%targets = {#R1 #R2}; %saved = %targets @GROUP(bad=1); %targets = %saved; %saved @TEXT(\"both\",key=\"ok\")",
    );
    assert_eq!(errors(&r), 1, "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].texts.iter().filter(|t| t.markup == "both").count(), 2);
}

#[test]
fn a_rejected_wire_rewrite_keeps_the_wire_for_the_next_description() {
    let r = read("0.4", "%w = &a @WIRE(mode=\"straight\") {#R1:1 #R2:1}; %saved = %w @WIRE(mode=\"bad\") {(0,5) (10,5)}; %w = %saved; %saved @TEXT(\"wire\",key=\"ok\")");
    assert_eq!(errors(&r), 1, "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].wires.len(), 1);
    assert!(r.model.levels[0].texts.iter().any(|t| t.markup == "wire"));
}

#[test]
fn a_rejected_new_wire_does_not_pass_its_net_as_a_wire() {
    let r = read("0.4", "%saved = &a @WIRE(mode=\"bad\") {#R1:1 #R2:1}; %saved @TEXT(\"absent\",key=\"ok\")");
    assert!(r.diagnostics().iter().any(|d| d.code == codes::REF_FAILED));
    assert!(!r.model.levels[0].texts.iter().any(|t| t.markup == "absent"));
}

#[test]
fn a_failed_creation_stays_failed_through_modifiers() {
    let r = read("0.4", "%saved = {#R1 #R2} @BUNDLE(bad=1) => @PLACE(2,3); %saved @TEXT(\"absent\",key=\"ok\")");
    assert!(r.diagnostics().iter().any(|d| d.code == codes::REF_FAILED));
    assert!(!r.model.levels[0].texts.iter().any(|t| t.markup == "absent"));
}

#[test]
fn a_bad_set_member_does_not_stop_later_modifications() {
    let r = read("0.4", "%targets = {#R1 #R2:1}; %saved = %targets @CURRENT(\"in\"); %targets = %saved");
    assert_eq!(errors(&r), 1, "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].currents.len(), 1);
    assert_eq!(r.model.levels[0].currents[0].direction, "in");
    assert!(r.model.levels[0].currents[0].terminal.starts_with("#R2:"));
}

#[test]
fn deferred_priority_checks_continue_after_a_bad_member() {
    let r = read("0.4", "%c = {#R1 #R2} @ALIGN(\"y\"); {#R1 %c} @PRIORITY(\"disabled\")");
    assert_eq!(errors(&r), 1, "{:?}", r.diagnostics());
    assert!(!r.problems[0].facts.iter().any(|f| matches!(f.kind, rnl_runtime::problem::FactKind::Align { .. })));
}

#[test]
fn released_versions_keep_first_segment_failure_behavior() {
    for version in ["0.2", "0.3"] {
        let r = read(version, "%saved = #R1 @PLACE(\"bad\",0); %saved @TEXT(\"absent\",key=\"ok\")");
        assert!(r.diagnostics().iter().any(|d| d.code == codes::REF_FAILED));
        assert!(!r.model.levels[0].texts.iter().any(|t| t.markup == "absent"));
    }
}

#[test]
fn a_symbol_body_keeps_a_local_text_after_failed_placement() {
    let r = read("0.4", "$S = @DEFINE-SYMBOL { %label = ? @TEXT(\"local\",key=\"ok\"); %saved = %label @PLACE(\"bad\",0); %saved @PLACE(0,4) }; #R1 @SET-SYMBOL $S");
    assert_eq!(errors(&r), 1, "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].texts.iter().find(|t| t.markup == "local").unwrap().anchor, (0.0, 4.0));
}

#[test]
fn recovery_reads_marked_roles_and_keyword_aliases() {
    let files = MemoryProvider::new().with("alias.sp", "* aliases\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @ALIAS(\"@put\",\"@PLACE\"); @ALIAS(\"onto\",\"target\") }\nR1 a b 1k\n* @RNL { #R1 @PLACE(0,0); %saved = #R1(onto) @put(\"bad\",0); %saved @TEXT(\"kept\",key=\"ok\") }\n.end\n");
    let r = interpret_file(Path::new("alias.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert_eq!(errors(&r), 1, "{:?}", r.diagnostics());
    assert!(r.model.levels[0].texts.iter().any(|t| t.markup == "kept"));
}

#[test]
fn ambiguous_slot_shapes_are_not_guessed_during_recovery() {
    let r = read("0.4", "%saved = #R1(wrong) @PLACE(\"bad\",0); %saved @TEXT(\"absent\",key=\"ok\")");
    assert_eq!(errors(&r), 2, "{:?}", r.diagnostics());
    assert!(r.diagnostics().iter().any(|d| d.code == codes::REF_FAILED));
    assert!(!r.model.levels[0].texts.iter().any(|t| t.markup == "absent"));
}

#[test]
fn the_statement_source_version_controls_recovery_in_mixed_includes() {
    for (outer, inner) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let files = MemoryProvider::new()
            .with("outer.sp", &format!("* outer\n* @RNL(type=\"header\") {{ @VERSION(\"{outer}\") }}\n.include \"inner.sp\"\n* @RNL(scope=\"child\") {{ #R1 @PLACE(0,0); %saved = #R1 @PLACE(\"bad\",0); %saved @TEXT(\"scoped\",key=\"ok\") }}\n.end\n"))
            .with("inner.sp", &format!("* inner\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a b\nR1 a b 1k\n.ends\n"));
        let r =
            interpret_file(Path::new("outer.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        assert_eq!(errors(&r), if outer == "0.4" { 1 } else { 2 }, "{:?}", r.diagnostics());
        assert_eq!(r.model.levels.iter().flat_map(|level| &level.texts).any(|t| t.markup == "scoped"), outer == "0.4");
    }
}

#[test]
fn missing_appearance_definitions_do_not_invalidate_the_target() {
    for keyword in ["SET-SYMBOL", "SET-STYLE"] {
        let r = read("0.4", &format!("%saved = #R1 @{keyword} $MISSING; %saved @TEXT(\"kept\",key=\"ok\")"));
        assert_eq!(errors(&r), 1, "{keyword}: {:?}", r.diagnostics());
        assert!(r.model.levels[0].texts.iter().any(|t| t.markup == "kept"));
    }
}

#[test]
fn a_failed_appearance_definition_does_not_invalidate_the_target() {
    let r = read(
        "0.4",
        "$BAD = @DEFINE-STYLE(linecolor=\"bad\") {}; %saved = #R1 @SET-STYLE $BAD; %saved @TEXT(\"kept\",key=\"ok\")",
    );
    assert_eq!(errors(&r), 2, "{:?}", r.diagnostics());
    assert!(r.model.levels[0].texts.iter().any(|t| t.markup == "kept"));
}

#[test]
fn a_missing_source_does_not_invalidate_an_add_or_remove_target() {
    for keyword in ["ADD-TO", "REMOVE-FROM"] {
        let r = read("0.4", &format!("%targets = {{#R2}}; %saved = %missing @{keyword} %targets; %targets = %saved; %saved @TEXT(\"kept\",key=\"ok\")"));
        assert_eq!(errors(&r), 1, "{keyword}: {:?}", r.diagnostics());
        let texts: Vec<_> = r.model.levels[0].texts.iter().filter(|t| t.markup == "kept").collect();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].attach.as_deref(), Some("#R2"));
    }
}

#[test]
fn missing_replacement_points_do_not_invalidate_an_existing_wire() {
    let r = read("0.4", "%w = &a @WIRE(mode=\"straight\") {#R1:1 #R2:1}; %saved = %w @WIRE %missing; %w = %saved; %saved @TEXT(\"kept\",key=\"ok\")");
    assert_eq!(errors(&r), 1, "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].wires.len(), 1);
    assert!(r.model.levels[0].texts.iter().any(|t| t.markup == "kept"));
}

#[test]
fn a_missing_loose_input_does_not_stop_valid_members() {
    let r = read("0.4", "%saved = %missing #R1 @PLACE(5,0); %saved @TEXT(\"kept\",key=\"ok\")");
    assert_eq!(errors(&r), 1, "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].items.iter().find(|i| i.id == "#R1").unwrap().at, (5.0, 0.0));
    let texts: Vec<_> = r.model.levels[0].texts.iter().filter(|t| t.markup == "kept").collect();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].attach.as_deref(), Some("#R1"));
}

#[test]
fn released_versions_still_stop_at_a_missing_loose_input() {
    for version in ["0.2", "0.3"] {
        let r = read(version, "%saved = %missing #R1 @PLACE(5,0); %saved @TEXT(\"absent\",key=\"ok\")");
        assert!(r.diagnostics().iter().any(|d| d.code == codes::REF_FAILED));
        assert_eq!(r.model.levels[0].items.iter().find(|i| i.id == "#R1").unwrap().at, (0.0, 0.0));
        assert!(!r.model.levels[0].texts.iter().any(|t| t.markup == "absent"));
    }
}

#[test]
fn a_constraint_result_unpacks_before_the_modifier_target_is_recovered() {
    let r = read("0.4", "%pair = (20,0) {#R1 #R2} @DISTANCE(\"left-to-right\",min=0); %saved = %pair @ADD-TO(bad=1); %saved @PRIORITY(\"disabled\")");
    assert_eq!(errors(&r), 1, "{:?}", r.diagnostics());
    assert_eq!(
        r.problems[0]
            .facts
            .iter()
            .filter(|f| matches!(f.kind, rnl_runtime::problem::FactKind::Distance { .. }))
            .count(),
        1
    );
}

#[test]
fn recovery_keeps_deep_forward_symbol_expansion_within_the_stack_limit() {
    let mut body = String::from("#R1 @SET-SYMBOL $S63; $MARK = @DEFINE(roles=[\"at\"]) {}; ");
    for n in (1..=63).rev() {
        body.push_str(&format!(
            "$S{n} = @DEFINE-SYMBOL {{ $MARK @ATTACH (1,2) => @SET-SYMBOL $S{} => @ORIENT(\"R90\") }}; ",
            n - 1
        ));
    }
    body.push_str("$S0 = @DEFINE-SYMBOL { @RECTANGLE {(2,3) (6,5)}; (3,4) @TEXT(\"deep\") }; ");
    let r = read("0.4", &body);
    assert_eq!(errors(&r), 0, "{:?}", r.diagnostics());
    let level = &r.model.levels[0];
    let item = level.items.iter().find(|i| i.id == "#R1").unwrap();
    assert_eq!(level.symbols[item.symbol].name, "$S63");
    assert_eq!(item.texts.len(), 1);
    assert!(item.texts.values().flatten().any(|line| rnl_runtime::markup::plain(line) == "deep"));
}
