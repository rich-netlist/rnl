//! Written sub-object handles determine creation before symbol-dependent conversion (5.2, 6.5, 8.2).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(body: &str, symbol_first: bool) -> Interpretation {
    let r = read_with_errors(body, symbol_first);
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    r
}

fn read_with_errors(body: &str, symbol_first: bool) -> Interpretation {
    let assignment = "#R1 @SET-SYMBOL $S;";
    let source = format!("* handles\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n* @RNL {{ $S = @DEFINE-SYMBOL {{ %p = (0,0) @PIN(terminal=1) => %q }}; $D = @DEFINE(roles=[\"r\"]) {{}}; {} {body}; {} }}\n.end\n", if symbol_first {assignment} else {""}, if symbol_first {""} else {assignment});
    interpret_file(
        Path::new("handles.sp"),
        None,
        &MemoryProvider::new().with("handles.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn count(r: &Interpretation) -> usize {
    r.model.levels[0].objects.iter().filter(|o| o.definition.as_deref() == Some("$D")).count()
}

#[test]
fn different_names_of_one_pin_remain_two_members_before_and_after_assignment() {
    for symbol_first in [false, true] {
        assert_eq!(count(&read("{#R1::p #R1::q} @UNION => @AS $D", symbol_first)), 2);
    }
}

#[test]
fn repeating_the_same_parent_and_name_deduplicates() {
    assert_eq!(count(&read("{#R1::p #R1::p} @UNION => @AS $D", false)), 1);
}

#[test]
fn a_bound_reference_keeps_the_same_handle() {
    assert_eq!(count(&read("%alias = #R1::p; {%alias #R1::p #R1::q} @UNION => @AS $D", false)), 2);
}

#[test]
fn removing_one_name_does_not_remove_another_name_of_the_same_pin() {
    assert_eq!(count(&read("%s = {#R1::p #R1::q}; #R1::p @REMOVE-FROM %s; %s @AS $D", false)), 1);
}

#[test]
fn positions_keep_both_written_handles_and_repeated_positions() {
    assert_eq!(count(&read("%s = {#R1::p #R1::q}; %s{1,2,1} @AS $D", false)), 3);
}

#[test]
fn the_appearance_queue_sees_both_usages_created_from_written_handles() {
    let r = read("$BOX = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=1) }; {#R1::p #R1::q} @UNION => @AS $D; # @SELECT-USAGE $D => @SET-SYMBOL $BOX => @PLACE(5,0)", false);
    assert_eq!(count(&r), 2);
    let level = &r.model.levels[0];
    assert_eq!(level.items.iter().filter(|item| level.symbols[item.symbol].name == "$BOX").count(), 2);
}

#[test]
fn different_named_handles_resolve_to_the_same_pin_position() {
    let r = read("#R1 @PLACE(10,20); #R1::p::position @TEXT(\"p\"); #R1::q::position @TEXT(\"q\")", false);
    for name in ["p", "q"] {
        let text = r.model.levels[0].texts.iter().find(|t| t.markup == name).unwrap();
        assert_eq!(text.anchor, (10.0, 20.0));
    }
}

#[test]
fn an_existing_text_key_remains_a_text_target() {
    let r = read("#R1 @PLACE(0,0); #R1::note @PLACE(4,5); #R1 @TEXT(\"note\",key=\"note\")", false);
    assert_eq!(r.model.levels[0].texts.iter().find(|t| t.markup == "note").unwrap().anchor, (4.0, 5.0));
}

#[test]
fn named_pin_position_uses_the_owners_transform() {
    let source = "* transformed pin\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a b 1k\n* @RNL { $S = @DEFINE-SYMBOL { %p = (2,3) @PIN }; #R1 @SET-SYMBOL $S => @PLACE(10,20) => @ORIENT(\"R90\"); #R1::p::position @TEXT(\"pin\") }\n.end\n";
    let r = interpret_file(
        Path::new("transform.sp"),
        None,
        &MemoryProvider::new().with("transform.sp", source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].texts.iter().find(|t| t.markup == "pin").unwrap().anchor, (7.0, 22.0));
}

#[test]
fn a_pin_role_is_checked_after_symbols_are_assigned() {
    let r = read("$P = @DEFINE(roles=[[\"p\",\"pin\"]]) {}; {#R1::p #R1::q} @UNION => @AS $P", false);
    assert_eq!(r.model.levels[0].objects.iter().filter(|o| o.definition.as_deref() == Some("$P")).count(), 2);
    let problem = &r.problems[0];
    let owner = problem.objects.iter().position(|o| o.label == "#R1").unwrap();
    let usages: Vec<_> = problem.usages.iter().filter(|u| u.definition == "$P").collect();
    assert_eq!(usages.len(), 2);
    for usage in usages {
        assert_eq!(usage.roles[0].members, [rnl_runtime::problem::Filler::Pin { object: owner, pin: 0 }]);
    }
}

#[test]
fn a_late_role_failure_drops_later_body_effects_but_keeps_earlier_symbol_assignments() {
    let r = read_with_errors("$BOX = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=1) }; %mark = #R1 @AS $D; $BAD = @DEFINE(roles=[[\"r\",\"instance\"],[\"target\",\"usage\"]]) { ?target @SET-SYMBOL $BOX; ?target @PLACE(100,100); %p = (3,4); %p @TEXT(\"bad\") }; %bad = #R1::p(r) %mark(target) @AS $BAD; %mark @PLACE(5,6); %bad::p @TEXT(\"dependent\")", true);
    let level = &r.model.levels[0];
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    assert!(level.objects.iter().all(|o| o.definition.as_deref() != Some("$BAD")));
    assert!(level.failed_usages.iter().any(|u| u.id.as_deref() == Some("%bad")));
    assert!(level.texts.iter().all(|t| t.markup != "bad" && t.markup != "dependent"));
    let item = level.items.iter().find(|i| i.object_id == "%mark").unwrap();
    assert_eq!(level.symbols[item.symbol].name, "$BOX");
    assert_eq!(item.at, (5.0, 6.0));
}

#[test]
fn unknown_pin_fails_its_usage_without_failing_an_independent_usage() {
    let r = read_with_errors("#R1::missing @AS $D; #R1::p @AS $D", false);
    assert_eq!(count(&r), 1);
    assert!(r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::REF_NO_SUBOBJECT));
}

#[test]
fn a_failed_body_text_layer_does_not_remove_an_independent_text_with_the_same_key() {
    for body_first in [true, false] {
        let bad = "$BAD = @DEFINE(roles=[[\"r\",\"instance\"],[\"target\",\"instance\"]]) { ?target @TEXT(\"bad\",key=\"note\") }; #R1::p(r) #R1(target) @AS $BAD;";
        let valid = "#R1 @TEXT(\"kept\",key=\"note\");";
        let body = if body_first { format!("{bad} {valid}") } else { format!("{valid} {bad}") };
        let r = read_with_errors(&format!("#R1 @PLACE(0,0); {body}"), false);
        assert!(r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::USAGE_ROLE_WRONG_KIND));
        assert_eq!(r.model.levels[0].texts.iter().map(|t| t.markup.as_str()).collect::<Vec<_>>(), ["kept"]);
    }
}

#[test]
fn late_failure_propagates_to_created_children_without_poisoning_shared_role_fillers() {
    let r = read_with_errors("$INNER = @DEFINE(roles=[\"r\"]) { %v = (1,2); %v @TEXT(\"child\") }; $BAD = @DEFINE(roles=[[\"r\",\"instance\"],[\"host\",\"instance\"]]) { ?host @AS $INNER; ?host @PLACE(100,100) }; #R1::p(r) #R1(host) @AS $BAD; #R1 @AS $INNER => @PLACE(0,0); #R1 @PLACE(5,6)", false);
    let level = &r.model.levels[0];
    assert!(r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::USAGE_ROLE_WRONG_KIND));
    assert_eq!(level.objects.iter().filter(|o| o.definition.as_deref() == Some("$INNER")).count(), 1);
    assert_eq!(level.texts.iter().filter(|t| t.markup == "child").count(), 1);
    assert_eq!(level.items.iter().find(|i| i.object_id == "#R1").unwrap().at, (5.0, 6.0));
    assert_eq!(r.problems[0].usages.iter().filter(|u| u.definition == "$INNER").count(), 1);
}

#[test]
fn named_role_fillers_keep_two_pins_bound_to_the_same_terminal_distinct() {
    let source = "* distinct pins\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a b 1k\n* @RNL { $S = @DEFINE-SYMBOL { %p = (1,0) @PIN(terminal=1); %q = (2,0) @PIN(terminal=1) }; $D = @DEFINE(roles=[[\"r\",\"pin\"]]) {}; {#R1::p #R1::q} @UNION => @AS $D; #R1 @SET-SYMBOL $S }\n.end\n";
    let r = interpret_file(
        Path::new("distinct.sp"),
        None,
        &MemoryProvider::new().with("distinct.sp", source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    let p = &r.problems[0];
    let owner = p.objects.iter().position(|o| o.label == "#R1").unwrap();
    let members: Vec<_> =
        p.usages.iter().filter(|u| u.definition == "$D").flat_map(|u| u.roles[0].members.clone()).collect();
    assert_eq!(
        members,
        [
            rnl_runtime::problem::Filler::Pin { object: owner, pin: 0 },
            rnl_runtime::problem::Filler::Pin { object: owner, pin: 1 },
        ]
    );
}

#[test]
fn a_reference_through_a_failed_usage_does_not_modify_its_valid_role_filler() {
    for through in [
        "%bad::host @PLACE(100,100)",
        "%alias = %bad::host; %alias @PLACE(100,100)",
        "%alias = %bad::host; %other = %alias; %other @PLACE(100,100)",
        "%bad::host => @PLACE(100,100)",
        "{%alias = %bad::host; %alias @PLACE(100,100)}",
    ] {
        let r = read_with_errors(&format!("$BAD = @DEFINE(roles=[[\"r\",\"instance\"],[\"host\",\"instance\"]]) {{}}; %bad = #R1::p(r) #R1(host) @AS $BAD; {through}; #R1 @PLACE(5,6,priority=\"low\")"), false);
        assert_eq!(r.model.levels[0].items.iter().find(|i| i.object_id == "#R1").unwrap().at, (5.0, 6.0), "{through}");
    }
}

#[test]
fn an_application_through_a_failed_role_alias_fails_without_rewinding_creation() {
    let r = read_with_errors("$BAD = @DEFINE(roles=[[\"r\",\"instance\"],[\"host\",\"instance\"]]) {}; %bad = #R1::p(r) #R1(host) @AS $BAD; %alias = %bad::host; %derived = %alias @AS $D; #R1 @AS $D", false);
    assert_eq!(count(&r), 1);
    assert!(r.model.levels[0].failed_usages.iter().any(|u| u.id.as_deref() == Some("%derived")));
}

#[test]
fn a_wire_keeps_the_named_pin_in_the_model_and_solver_input() {
    let r = read("#R1 @PLACE(0,0); &a @WIRE {#R1::q (4,0)}", false);
    let wire = &r.model.levels[0].wires[0];
    assert!(
        matches!(&wire.via[0], rnl_runtime::model::Vertex::Pin { owner, pin: 0, reference } if owner == "#R1" && reference.as_deref() == Some("#R1::q"))
    );
    assert_eq!(wire.points.first(), Some(&(0.0, 0.0)));
    let problem = &r.problems[0];
    assert!(
        matches!(&problem.wires[0].anchors[0], rnl_runtime::problem::PointRef::SymbolPin { pin: 0, reference: rnl_runtime::problem::Reference::Name(name), .. } if name == "#R1::q")
    );
}

#[test]
fn a_text_key_is_not_a_pin_at_a_wire_endpoint() {
    let r = read_with_errors("#R1 @PLACE(0,0); #R1 @TEXT(\"label\",key=\"note\"); &a @WIRE {#R1::note (4,0)}", false);
    assert!(r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::WIRE_POINT_WRONG_KIND));
    assert!(r.model.levels[0].wires.is_empty());
    assert!(r.problems[0].wires.is_empty());
    assert_eq!(r.model.levels[0].texts.len(), 1);
}

#[test]
fn a_bad_named_pin_rewrite_keeps_the_valid_wire_and_its_style() {
    let r = read_with_errors("#R1 @PLACE(0,0); #R1 @TEXT(\"label\",key=\"note\"); %w = &a @WIRE(mode=\"straight\",linewidth=2) {#R1::p (4,0)}; %w @WIRE(linewidth=7) {#R1::note (8,0)}", false);
    let wire = &r.model.levels[0].wires[0];
    assert_eq!(wire.points.last(), Some(&(4.0, 0.0)));
    assert_eq!(wire.style.get("linewidth").and_then(|value| value.num()), Some(2.0));
    assert!(r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::WIRE_POINT_WRONG_KIND));
}

#[test]
fn a_named_pin_reports_the_net_it_binds_when_a_wire_names_another_net() {
    let r = read("#R1 @PLACE(0,0); &b @WIRE {#R1::p (4,0)}", false);
    let mismatch: Vec<_> =
        r.diagnostics().into_iter().filter(|d| d.code == rnl_runtime::diag::codes::WIRE_NET_MISMATCH).collect();
    assert_eq!(mismatch.len(), 1);
    assert!(mismatch[0].message.contains("pin #R1::p"));
}

#[test]
fn a_named_pin_supports_graphics_and_a_label_with_its_net_attribute() {
    let r =
        read("#R1 @PLACE(10,20); #R1::p @CIRCLE(radius=2); #R1::p @TEXT(\"${net}\",key=\"netlabel\",dx=1,dy=2)", false);
    let level = &r.model.levels[0];
    assert_eq!(level.shapes[0].points[0], (10.0, 20.0));
    let text = &level.texts[0];
    assert_eq!(text.anchor, (11.0, 22.0));
    assert!(matches!(text.target, Some(rnl_runtime::model::TextTarget::Pin(_, 0))));
    assert_eq!(text.attach.as_deref(), Some("#R1::p"));
    let problem = &r.problems[0];
    let label = problem.objects.iter().find_map(|object| object.text.as_ref()).unwrap();
    assert!(matches!(label.attach, rnl_runtime::problem::Filler::Pin { pin: 0, .. }));
}

#[test]
fn a_text_key_does_not_implicitly_convert_to_a_graphics_point() {
    let r = read_with_errors("#R1 @PLACE(0,0); #R1 @TEXT(\"label\",key=\"note\"); #R1::note @CIRCLE(radius=1)", false);
    assert!(r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::KEYWORD_GRAPHIC_NOT_POINTS));
    assert!(r.model.levels[0].shapes.is_empty());
    assert_eq!(r.model.levels[0].texts.len(), 1);
}

#[test]
fn a_text_key_on_a_named_pin_can_be_referenced_before_its_text_statement() {
    for before in [true, false] {
        let label = "#R1::p @TEXT(\"pin label\",key=\"note\")";
        let placement = "#R1::p::note @PLACE(7,8)";
        let statements = if before { format!("{placement}; {label}") } else { format!("{label}; {placement}") };
        let r = read(&format!("#R1 @PLACE(0,0); {statements}"), false);
        let text = r.model.levels[0].texts.iter().find(|text| text.markup == "pin label").unwrap();
        assert_eq!(text.anchor, (7.0, 8.0));
        assert_eq!(text.object_id, "#R1::p::note");
    }
}

#[test]
fn an_unknown_subobject_of_a_named_pin_fails_without_creating_a_text() {
    let r = read_with_errors("#R1 @PLACE(0,0); #R1::p::missing @PLACE(7,8)", false);
    assert!(r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::REF_NO_SUBOBJECT));
    assert!(r.model.levels[0].texts.is_empty());
}

#[test]
fn a_late_failed_usage_cannot_rewrite_an_independent_wire() {
    for inside in [true, false] {
        let body = if inside { "?w @WIRE(linewidth=7) {(0,0) (8,0)}" } else { "" };
        let outside = if inside { "" } else { "%alias = %bad::w; %alias @WIRE(linewidth=7) {(0,0) (8,0)}" };
        let r = read_with_errors(&format!("%w = &a @WIRE(mode=\"straight\",linewidth=2) {{(0,0) (4,0)}}; $BAD = @DEFINE(roles=[[\"r\",\"instance\"],[\"w\",\"wire\"]]) {{ {body} }}; %bad = #R1::p(r) %w(w) @AS $BAD; {outside}"), false);
        assert!(r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::USAGE_ROLE_WRONG_KIND));
        let wire = &r.model.levels[0].wires[0];
        assert_eq!(wire.points.last(), Some(&(4.0, 0.0)), "inside: {inside}");
        assert_eq!(wire.style.get("linewidth").and_then(|value| value.num()), Some(2.0));
        assert_eq!(r.problems[0].wires.len(), 1);
    }
}

#[test]
fn a_set_members_reference_failure_does_not_disable_the_other_members() {
    let r = read_with_errors("%one = (1,1); %two = (2,2); $BAD = @DEFINE(roles=[[\"r\",\"instance\"],\"host\"]) {}; %bad = #R1::p(r) %one(host) @AS $BAD; {%bad::host %two} @PLACE(7,8); %one @TEXT(\"one\"); %two @TEXT(\"two\")", false);
    let texts = &r.model.levels[0].texts;
    assert_eq!(texts.iter().find(|text| text.markup == "one").unwrap().anchor, (1.0, 1.0));
    assert_eq!(texts.iter().find(|text| text.markup == "two").unwrap().anchor, (7.0, 8.0));
}

#[test]
fn repeated_set_members_keep_their_own_reference_dependencies() {
    for set in [
        "%s = {%bad::host #R1}",
        "%s = {#R1 %bad::host}",
        "%alias = %bad::host; %s = {%alias #R1}",
        "%s = {%alias = %bad::host; #R1}",
        "%inner = {%bad::host}; %s = {%inner #R1}",
        "%s = {#R1}; %bad::host @ADD-TO %s",
        "%inner = {%bad::host #R1}; %s = %inner{1,2}",
        "%inner = {%bad::host #R1}; %s = {%inner{1} %inner{2}}",
        "%inner = {%bad::host}; %s = {%inner{1} #R1}",
    ] {
        let r = read_with_errors(&format!("$BAD = @DEFINE(roles=[[\"r\",\"instance\"],[\"host\",\"instance\"]]) {{}}; %bad = #R1::p(r) #R1(host) @AS $BAD; {set}; %s @AS $D; %s @PLACE(7,8); %s @TEXT(\"member\"); #R1 @PLACE(0,0,priority=\"low\")"), false);
        assert_eq!(count(&r), 1, "{set}");
        assert_eq!(
            r.model.levels[0].items.iter().find(|item| item.object_id == "#R1").unwrap().at,
            (7.0, 8.0),
            "{set}"
        );
        assert_eq!(r.model.levels[0].texts.iter().filter(|text| text.markup == "member").count(), 1, "{set}");
    }
}

#[test]
fn loose_members_keep_independent_failure_dependencies() {
    for inputs in ["%bad::host %two", "%bad::host => %two"] {
        let r = read_with_errors(&format!("%one = (1,1); %two = (2,2); $BAD = @DEFINE(roles=[[\"r\",\"instance\"],\"host\"]) {{}}; %bad = #R1::p(r) %one(host) @AS $BAD; {inputs} @PLACE(7,8); {inputs} @TEXT(\"member\"); %one @TEXT(\"one\"); %two @TEXT(\"two\")"), false);
        let texts = &r.model.levels[0].texts;
        assert_eq!(texts.iter().find(|text| text.markup == "one").unwrap().anchor, (1.0, 1.0), "{inputs}");
        assert_eq!(texts.iter().find(|text| text.markup == "two").unwrap().anchor, (7.0, 8.0), "{inputs}");
        assert_eq!(texts.iter().filter(|text| text.markup == "member").count(), 1, "{inputs}");
    }
}

#[test]
fn graphic_centres_fail_individually_but_a_wire_or_line_with_a_failed_point_fails_whole() {
    for centres in ["{%bad::host %two}", "%bad::host %two"] {
        let r = read_with_errors(&format!("%one = (1,1); %two = (2,2); $BAD = @DEFINE(roles=[[\"r\",\"instance\"],\"host\"]) {{}}; %bad = #R1::p(r) %one(host) @AS $BAD; {centres} @CIRCLE(radius=1); {{%bad::host %two}} @LINE; &a @WIRE {{%bad::host %two}}"), false);
        let level = &r.model.levels[0];
        assert_eq!(level.shapes.len(), 1, "{centres}");
        assert_eq!(level.shapes[0].points, [(2.0, 2.0)], "{centres}");
        assert!(level.wires.is_empty());
        assert!(r.problems[0].wires.is_empty());
    }
}

#[test]
fn a_failed_reference_only_invalidates_the_constraint_combinations_that_use_it() {
    for constraint in [
        "{#R1 {%bad::host %two}} @ORDER(\"left-to-right\")",
        "%choices = {%bad::host %two}; {#R1 %choices} @ORDER(\"left-to-right\")",
        "#R1 {%bad::host %two} @DISTANCE(\"left-to-right\",min=1)",
    ] {
        let r = read_with_errors(&format!("%one = (1,1); %two = (2,2); #R1 @PLACE(0,0); $BAD = @DEFINE(roles=[[\"r\",\"instance\"],\"host\"]) {{}}; %bad = #R1::p(r) %one(host) @AS $BAD; {constraint}"), false);
        let facts: Vec<_> = r.problems[0]
            .facts
            .iter()
            .filter(|fact| {
                matches!(
                    fact.kind,
                    rnl_runtime::problem::FactKind::Order { .. } | rnl_runtime::problem::FactKind::Distance { .. }
                )
            })
            .collect();
        assert_eq!(facts.len(), 1, "{constraint}");
    }
}

#[test]
fn a_set_role_fails_whole_when_one_member_reference_fails_later() {
    let r = read_with_errors("$BAD = @DEFINE(roles=[[\"r\",\"instance\"],\"host\"]) {}; %bad = #R1::p(r) #R1(host) @AS $BAD; $SET = @DEFINE(roles=[\"{members}\"]) { ?members @PLACE(100,100) }; %set_usage = {%bad::host #R1} @AS $SET; #R1 @PLACE(5,6)", false);
    assert!(r.model.levels[0].objects.iter().all(|object| object.definition.as_deref() != Some("$SET")));
    assert_eq!(r.model.levels[0].items.iter().find(|item| item.object_id == "#R1").unwrap().at, (5.0, 6.0));
    assert!(r
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == rnl_runtime::diag::codes::USAGE_ROLE_FAILED_OBJECT));
}

#[test]
fn a_text_key_with_no_surviving_layer_fails_its_dependent_usage() {
    let r = read_with_errors("$BAD = @DEFINE(roles=[[\"r\",\"instance\"],\"host\"]) { ?host @TEXT(\"bad\",key=\"note\") }; #R1::p(r) #R1(host) @AS $BAD; #R1::note @AS $D; #R1 @PLACE(0,0)", false);
    assert_eq!(count(&r), 0);
    assert!(r.model.levels[0].texts.is_empty());
    assert!(r
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == rnl_runtime::diag::codes::USAGE_ROLE_FAILED_OBJECT));
}
