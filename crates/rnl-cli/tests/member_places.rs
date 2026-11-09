//! Written set membership, including later edits, and member identity (5.2.3).

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::problem::{FactKind, PointRef};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(version: &str, body: &str) -> Interpretation {
    let source = format!("* places\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 b c 1k\nR3 c d 1k\nR4 d 0 1k\n* @RNL {{ {body} }}\n.end\n");
    let files = MemoryProvider::new().with("t.sp", &source);
    interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}

fn errors(r: &Interpretation) -> Vec<(Code, String)> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| (d.code, r.sink.render(d))).collect()
}

fn order(body: &str) -> Vec<String> {
    let r = read("0.4", body);
    assert_eq!(errors(&r), Vec::new(), "{body}");
    let p = &r.problems[0];
    p.facts
        .iter()
        .find_map(|f| match &f.kind {
            FactKind::Order { members, .. } => Some(
                members
                    .iter()
                    .map(|pnt| match pnt {
                        PointRef::Origin(i) => p.objects[*i].label.clone(),
                        other => panic!("expected an object, got {other:?}"),
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn places_expand_ranges_in_order_and_keep_repeats() {
    assert_eq!(
        order("%a = {#R1 #R2 #R3 #R4}; %a{end:1:2, 40, end-3} @ORDER(\"left-to-right\")"),
        ["#R4", "#R2", "#R1"]
    );
    // Keeping the repeated position makes this strict order contradictory.
    let repeated = errors(&read("0.4", "%a = {#R1 #R2}; %a{1,2,1} @ORDER(\"left-to-right\")"));
    assert_eq!(repeated.len(), 1, "{repeated:?}");
    assert_eq!(repeated[0].0, codes::CONSTRAINT_SELF_CONTRADICTION);
    assert_eq!(order("%b = {#R1 #R2}; %a = {%b #R3}; %a{2:end} @ORDER(\"left-to-right\")"), ["#R2", "#R3"]);
}

#[test]
fn places_read_all_additions_and_removals_in_written_order() {
    for body in [
        "%a = {#R1}; %a{2,1} @ORDER(\"left-to-right\"); #R2 @ADD-TO %a",
        "%a = {#R1 #R2}; %a{1:end} @ORDER(\"left-to-right\"); #R1 @REMOVE-FROM %a; #R3 @ADD-TO %a",
        "%a = {%b}; %b = {#R1}; %a{1:end} @ORDER(\"left-to-right\"); #R2 @ADD-TO %b",
    ] {
        let expected = if body.contains("REMOVE") {
            ["#R2", "#R3"]
        } else if body.contains("{2,1}") {
            ["#R2", "#R1"]
        } else {
            ["#R1", "#R2"]
        };
        assert_eq!(order(body), expected, "{body}");
    }
}

#[test]
fn a_single_place_preserves_a_nested_sets_identity_but_a_range_copies_members() {
    // A directly written inner set is one member. Editing the single result edits that set.
    assert_eq!(
        order("%a = {{#R1 #R2} #R3}; %inner = %a{1}; #R4 @ADD-TO %inner; %inner @ORDER(\"left-to-right\")"),
        ["#R1", "#R2", "#R4"]
    );
    // A range, even 1:1, makes a new outer set. Adding to it leaves the source alone.
    assert_eq!(
        order("%a = {#R1 #R2}; %copy = %a{1:1}; #R3 @ADD-TO %copy; %a @ORDER(\"left-to-right\")"),
        ["#R1", "#R2"]
    );
    assert_eq!(
        order(
            "%a = {#R1 #R2}; %empty = %a{9}; #R3 @ADD-TO %empty; #R4 @ADD-TO %empty; %empty @ORDER(\"left-to-right\")"
        ),
        ["#R3", "#R4"]
    );
}

#[test]
fn selection_dependencies_through_items_union_and_edits_are_errors() {
    for body in [
        "%s = # @SELECT-INSTANCE; %s{1}",
        "%s = # @SELECT-INSTANCE; %a = {%s}; %a{1}",
        "%s = # @SELECT-INSTANCE; %a = %s @UNION; %a{1}",
        "%s = # @SELECT-INSTANCE; %a = {#R1}; %a{1}; %s @ADD-TO %a",
        "%s = # @SELECT-INSTANCE; %a = {#R1}; %a{1}; %s @REMOVE-FROM %a",
        "%s = # @SELECT-INSTANCE; %a = {%b}; %b = {#R1}; %a{1}; %s @ADD-TO %b",
    ] {
        let e = errors(&read("0.4", body));
        assert!(e.iter().any(|(code, _)| *code == codes::SET_PLACES_OF_SELECTION), "{body}: {e:?}");
    }
}

#[test]
fn instances_and_usages_have_no_member_places() {
    for body in ["%a = #R1; %a{1}", "$D = @DEFINE(roles=[\"left\"]) {}; %u = #R1 @AS $D; %u{1}"] {
        let e = errors(&read("0.4", body));
        assert_eq!(e.len(), 1, "{e:?}");
        assert_eq!(e[0].0, codes::SET_PLACES_WRONG_KIND);
    }
    assert!(errors(&read("0.4", "$D = @DEFINE(roles=[\"left\"]) {}; %u = #R1 @AS $D; %u::left @PLACE(1,2)")).is_empty());
}

#[test]
fn set_role_placeholders_and_later_body_edits_are_supported() {
    assert_eq!(
        order("$D = @DEFINE(roles=[\"{ref}\"]) { ?ref{2,1} @ORDER(\"left-to-right\") }; {#R1 #R2} @AS $D"),
        ["#R2", "#R1"]
    );
    assert_eq!(order("$D = @DEFINE(roles=[\"left\", \"right\"]) { %s = {?left}; %s{1:end} @ORDER(\"left-to-right\"); ?right @ADD-TO %s }; #R1 #R2 @AS $D"),
        ["#R1", "#R2"]);
}

#[test]
fn later_edits_in_nested_sets_and_definition_applications_take_effect() {
    assert_eq!(order("%s = {#R1}; { %s{1:end} @ORDER(\"left-to-right\"); #R2 @ADD-TO %s }"), ["#R1", "#R2"]);
    assert_eq!(order("$ADD = @DEFINE(roles=[\"{target}\", \"item\"]) { ?item @ADD-TO ?target }; %s = {#R1}; %s{1:end} @ORDER(\"left-to-right\"); %s(target) #R2(item) @AS $ADD"),
        ["#R1", "#R2"]);
}

#[test]
fn touching_braces_are_places_only_in_0_4() {
    for version in ["0.2", "0.3"] {
        assert!(errors(&read(version, "%a = #R1; %a{#R2} @ALIGN(\"y\")")).is_empty());
    }
    let e = errors(&read("0.4", "%a = #R1; %a{#R2} @ALIGN(\"y\"); #R3 @PLACE(2,3)"));
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].0, codes::SYNTAX_PLACES_TOUCHING_BRACES);
    assert!(errors(&read("0.4", "%a = #R1; %a {#R2} @ALIGN(\"y\")")).is_empty());
}

#[test]
fn out_of_range_places_are_empty_and_ranges_preserve_nested_members() {
    assert_eq!(order("%a = {#R1 #R2 #R3}; %a{999:1:2, end-99, 900} @ORDER(\"left-to-right\")"), ["#R3", "#R1"]);
    assert_eq!(order("%a = {{#R1 #R2} #R3}; %copy = %a{1:1}; %inner = %copy{1}; #R4 @ADD-TO %inner; %inner @ORDER(\"left-to-right\")"),
        ["#R1", "#R2", "#R4"]);
    assert!(errors(&read("0.4", "%a = {}; %a{end}; %a{1:999}")).is_empty());
}

#[test]
fn a_chain_of_forward_edits_and_unions_uses_final_members() {
    assert_eq!(
        order("%a = {#R1}; %a{1:end} @ORDER(\"left-to-right\"); %b @ADD-TO %a; %b = {#R2}; #R3 @ADD-TO %b"),
        ["#R1", "#R2", "#R3"]
    );
    assert_eq!(
        order("%a = {#R1 #R1}; %u = %a @UNION; %u{1:end} @ORDER(\"left-to-right\"); #R2 @ADD-TO %a"),
        ["#R1", "#R2"]
    );
    // Taking one instance retains its identity and supports its sub-objects.
    assert!(errors(&read("0.4", "%a = {#R1}; %a{1}:1 @TEXT(\"terminal\")")).is_empty());
}

#[test]
fn a_place_that_depends_on_its_own_edit_fails_as_a_cycle() {
    for body in ["%a = {#R1}; %a{1} @ADD-TO %a", "%a = {#R1}; %v = %a{1}; %v @REMOVE-FROM %a"] {
        let e = errors(&read("0.4", body));
        assert!(e.iter().any(|(code, _)| *code == codes::REF_NAME_CYCLE), "{body}: {e:?}");
    }
}

#[test]
fn package_place_bindings_are_evaluated_separately_in_each_level() {
    let files = MemoryProvider::new()
        .with("p.rnl", "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"p\",version=\"1\") }\n@RNL { $MEMBERS = {}; $PICK = $MEMBERS{end:1}; $ALIAS = $PICK }")
        .with("t.sp", "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"p\",version=\"1\",path=\"p.rnl\") }\nR1 a b 1k\nR2 b 0 1k\n* @RNL { $ALIAS @ORDER(\"left-to-right\"); {#R1 #R2} @ADD-TO $MEMBERS }\n.subckt child a b\nR3 a b 1k\nR4 b 0 1k\n* @RNL { $ALIAS @ORDER(\"left-to-right\"); {#R3 #R4} @ADD-TO $MEMBERS }\n.ends\n.end\n");
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert_eq!(errors(&r), Vec::new());
    let orders: Vec<Vec<String>> = r
        .problems
        .iter()
        .flat_map(|p| {
            p.facts.iter().filter_map(|f| match &f.kind {
                FactKind::Order { members, .. } => Some(
                    members
                        .iter()
                        .map(|m| match m {
                            PointRef::Origin(i) => p.objects[*i].label.clone(),
                            other => panic!("{other:?}"),
                        })
                        .collect(),
                ),
                _ => None,
            })
        })
        .collect();
    assert_eq!(orders, [["#R2", "#R1"], ["#R4", "#R3"]]);
}

#[test]
fn forward_bindings_inside_bodies_and_nested_sets_see_later_edits() {
    assert_eq!(order("$D = @DEFINE(roles=[\"a\", \"b\"]) { %picked @ORDER(\"left-to-right\"); %picked = %s{1:end}; %s = {?a}; ?b @ADD-TO %s }; #R1 #R2 @AS $D"),
        ["#R1", "#R2"]);
    assert_eq!(
        order("{ %picked @ORDER(\"left-to-right\"); %picked = %s{1:end}; %s = {#R1}; #R2 @ADD-TO %s }"),
        ["#R1", "#R2"]
    );
}

#[test]
fn the_numbering_check_includes_member_places() {
    let a = read("0.4", "%a = {#R1 #R2}; %a{1} @TEXT(\"x\")");
    let b = read("0.4", "%a = {#R1 #R2}; %a{2} @TEXT(\"x\")");
    assert_eq!(errors(&a), Vec::new());
    assert_eq!(errors(&b), Vec::new());
    assert_ne!(a.numbering[0].check, b.numbering[0].check);
    assert!(a.numbering[0].authored.iter().any(|line| line.contains("%a { 1 } @TEXT")), "{:?}", a.numbering);
    let spaced = read("0.4", "%a = {#R1 #R2}; %a{ /* first */ 1 } @TEXT(\"x\")");
    assert_eq!(a.numbering[0].check, spaced.numbering[0].check);
}

#[test]
fn the_source_file_version_controls_places_in_scoped_records() {
    for (outer, inner) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let files = MemoryProvider::new()
            .with("t.sp", &format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"{outer}\") }}\n.include \"inc.sp\"\n* @RNL(scope=\"child\") {{ %a = {{#R1 #R2}}; %a{{2,1}} @ORDER(\"left-to-right\") }}\n.end\n"))
            .with("inc.sp", &format!("* inc\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a b\nR1 a b 1k\nR2 b 0 1k\n.ends\n"));
        let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        assert_eq!(errors(&r), Vec::new());
        let p = r.problems.iter().find(|p| p.objects.iter().any(|o| o.label == "#R1")).unwrap();
        let names: Vec<_> = p
            .facts
            .iter()
            .find_map(|f| match &f.kind {
                FactKind::Order { members, .. } => Some(
                    members
                        .iter()
                        .map(|m| match m {
                            PointRef::Origin(i) => p.objects[*i].label.as_str(),
                            other => panic!("{other:?}"),
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap();
        assert_eq!(names, if outer == "0.4" { ["#R2", "#R1"] } else { ["#R1", "#R2"] });
    }
}
