//! A written set item contributes alternatives, each combination an independent fact (14.1).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::problem::{FactKind, PointRef};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(version: &str, body: &str) -> Interpretation {
    let body = body.replace('\n', "\n* ");
    let source = format!("* constraints\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 b c 1k\nR3 c d 1k\nR4 d 0 1k\n* @RNL {{ {body} }}\n.end\n");
    interpret_file(
        Path::new("t.sp"),
        None,
        &MemoryProvider::new().with("t.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn errors(r: &Interpretation) -> Vec<(Code, String)> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| (d.code, r.sink.render(d))).collect()
}

fn orders(r: &Interpretation) -> Vec<Vec<String>> {
    let p = &r.problems[0];
    let mut rows: Vec<_> = p
        .facts
        .iter()
        .filter_map(|f| match &f.kind {
            FactKind::Order { members, .. } => Some(
                members
                    .iter()
                    .map(|m| match m {
                        PointRef::Origin(k) => p.objects[*k].label.clone(),
                        other => panic!("unexpected composite point {other:?}"),
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .collect();
    rows.sort();
    rows
}

fn rows(body: &str) -> Vec<Vec<String>> {
    let r = read("0.4", body);
    assert_eq!(errors(&r), [], "{body}");
    orders(&r)
}

#[test]
fn references_and_literal_sets_are_alternative_items() {
    for body in [
        "{#R1 {#R2 #R3} #R4} @ORDER(\"left-to-right\")",
        "%s = {#R2 #R3}; {#R1 %s #R4} @ORDER(\"left-to-right\")",
        "%s = {#R2 {#R3}}; #R1 %s #R4 @ORDER(\"left-to-right\")",
    ] {
        assert_eq!(rows(body), [["#R1", "#R2", "#R4"], ["#R1", "#R3", "#R4"]]);
    }
    assert_eq!(rows("%s = {#R2 #R3}; {#R1 %s{1} %s{2} #R4} @ORDER(\"left-to-right\")"), [["#R1", "#R2", "#R3", "#R4"]]);
}

#[test]
fn a_single_set_is_split_once_before_its_items_are_read() {
    for body in [
        "%s = {#R1 #R2 #R3}; %s @ORDER(\"left-to-right\")",
        "%s = {#R1 #R2 #R3}; {%s} @ORDER(\"left-to-right\")",
        "{{#R1 #R2 #R3}} @ORDER(\"left-to-right\")",
    ] {
        assert_eq!(rows(body), [["#R1", "#R2", "#R3"]]);
    }
    assert_eq!(rows("{{{#R1 #R2}} #R3} @ORDER(\"left-to-right\")"), [["#R1", "#R3"], ["#R2", "#R3"]]);
}

#[test]
fn every_combination_is_an_independent_align_or_distribute() {
    for (keyword, parameter) in [("ALIGN", "y"), ("DISTRIBUTE", "left-to-right")] {
        let r = read("0.4", &format!("{{{{#R1 #R2}} {{#R3 #R4}}}} @{keyword}(\"{parameter}\")"));
        assert_eq!(errors(&r), []);
        let facts: Vec<_> = r.problems[0]
            .facts
            .iter()
            .filter(|f| matches!(f.kind, FactKind::Align { .. } | FactKind::Distribute { .. }))
            .collect();
        assert_eq!(facts.len(), 4);
        for f in facts {
            let members = match &f.kind {
                FactKind::Align { members, .. } | FactKind::Distribute { members, .. } => members,
                _ => unreachable!(),
            };
            assert_eq!(members.len(), 2);
            assert!(members.iter().all(|m| matches!(m, PointRef::Origin(_))));
        }
    }
}

#[test]
fn distance_has_one_fact_for_each_pair_and_none_for_an_empty_side() {
    let r = read("0.4", "{#R1 #R2} {#R3 #R4} @DISTANCE(\"left-to-right\", min=2)");
    assert_eq!(errors(&r), []);
    let pairs: Vec<_> = r.problems[0]
        .facts
        .iter()
        .filter_map(|f| match &f.kind {
            FactKind::Distance { from, to, .. } => Some((from.len(), to.len())),
            _ => None,
        })
        .collect();
    assert_eq!(pairs, [(1, 1); 4]);
    for body in ["{} #R1 @DISTANCE(\"left-to-right\", min=2)", "#R1 {} @DISTANCE(\"left-to-right\", min=2)"] {
        let r = read("0.4", body);
        assert_eq!(errors(&r), []);
        assert!(r.problems[0].facts.is_empty());
    }
}

#[test]
fn orientation_lists_stay_together_in_each_expanded_relation() {
    let r = read("0.4", "%c = #R1 {#R2 #R3} @ORIENT-RELATION([\"R0\",\"MY\"]); %c{1} @PRIORITY(\"disabled\")");
    assert_eq!(errors(&r), []);
    let p = &r.problems[0];
    assert_eq!(p.facts.len(), 1);
    assert!(
        matches!(&p.facts[0].kind, FactKind::OrientRelation { from, to, .. } if from.len()==1 && to.len()==1 && p.objects[to[0]].label=="#R3")
    );
    assert_eq!(p.orient_lists.len(), 1);
    assert_eq!(p.orient_lists[0].1.len(), 2);
}

#[test]
fn named_results_are_real_sets_with_stable_member_identity() {
    assert_eq!(
        rows("%c = {#R1 {#R2 #R3} #R4} @ORDER(\"left-to-right\"); %c{1} @PRIORITY(\"disabled\")"),
        [["#R1", "#R3", "#R4"]]
    );
    assert_eq!(
        rows("%c = {#R1 {#R2 #R3} #R4} @ORDER(\"left-to-right\"); %c @PRIORITY(\"disabled\")"),
        Vec::<Vec<String>>::new()
    );
    let r = read("0.4", "%c = {#R1 {#R2} #R4} @ORDER(\"left-to-right\", priority=\"low\"); %c{1} @PRIORITY(\"high\")");
    assert_eq!(errors(&r), []);
    assert_eq!(r.problems[0].facts[0].label, "%c");
    assert_eq!(r.problems[0].facts[0].priority.as_str(), "high");
    // A plain chain remains a constraint, rather than a set containing one constraint.
    let r = read("0.4", "%c = {#R1 #R2} @ORDER(\"left-to-right\"); %c{1} @PRIORITY(\"high\")");
    assert!(errors(&r).iter().any(|(c, _)| *c == codes::SET_PLACES_WRONG_KIND));
}

#[test]
fn late_edits_change_the_combinations_before_names_are_used() {
    assert_eq!(
        rows("%s = {#R2}; %c = {#R1 %s #R4} @ORDER(\"left-to-right\"); %c{2} @PRIORITY(\"disabled\"); #R3 @ADD-TO %s"),
        [["#R1", "#R2", "#R4"]]
    );
    assert_eq!(
        rows("%s = {#R1 {#R2 #R3}}; %s @ORDER(\"left-to-right\"); #R2 @REMOVE-FROM %s; #R4 @ADD-TO %s"),
        [["#R1", "#R3", "#R4"]]
    );
    assert_eq!(
        rows("%s = {#R1}; %s @ORDER(\"left-to-right\"); {#R2 #R3} @ADD-TO %s"),
        [["#R1", "#R2"], ["#R1", "#R3"]]
    );
}

#[test]
fn the_single_set_split_sees_late_edits_too() {
    let r = read("0.4", "%s = {#R1}; %s @DISTANCE(\"left-to-right\", min=1); #R2 @ADD-TO %s");
    assert_eq!(errors(&r), []);
    assert_eq!(r.problems[0].facts.len(), 1);
}

#[test]
fn late_edits_inside_definition_bodies_are_seen() {
    assert_eq!(rows("$D = @DEFINE(roles=[\"a\",\"b\",\"c\"]) { %s = {?b}; {?a %s} @ORDER(\"left-to-right\"); ?c @ADD-TO %s }; #R1 #R2 #R3 @AS $D"), [["#R1", "#R2"], ["#R1", "#R3"]]);
}

#[test]
fn an_empty_item_is_omitted_and_short_combinations_give_a_hint() {
    assert_eq!(rows("{#R1 {} #R2} @ORDER(\"left-to-right\")"), [["#R1", "#R2"]]);
    let r = read("0.4", "{#R1 {}} @ORDER(\"left-to-right\")");
    assert_eq!(errors(&r), []);
    assert!(r
        .diagnostics()
        .iter()
        .any(|d| d.code == codes::CONSTRAINT_EXPANSION_TOO_FEW_PLACES && d.severity == Severity::Hint));
}

#[test]
fn repeated_members_fail_only_the_contradictory_combination() {
    let r = read("0.4", "{#R1 {#R1 #R2}} @ORDER(\"left-to-right\")");
    assert_eq!(errors(&r).len(), 1);
    assert_eq!(errors(&r)[0].0, codes::CONSTRAINT_SELF_CONTRADICTION);
    assert_eq!(orders(&r), [["#R1", "#R2"]]);
    let r = read("0.4", "{#R1 {#R2 #R2}} @ORDER(\"left-to-right\")");
    assert_eq!(errors(&r), []);
    assert_eq!(orders(&r), [["#R1", "#R2"], ["#R1", "#R2"]]);
}

#[test]
fn the_expansion_limit_fails_the_whole_statement() {
    let column = std::iter::repeat_n("#R2", 33).collect::<Vec<_>>().join(" ");
    let r = read("0.4", &format!("{{#R1 {{{column}}} {{{column}}}}} @ALIGN(\"y\")"));
    assert_eq!(errors(&r).len(), 1);
    assert_eq!(errors(&r)[0].0, codes::CONSTRAINT_EXPANSION_TOO_MANY);
    assert!(errors(&r)[0].1.contains("implementation's limit"));
    assert!(r.problems[0].facts.is_empty());
}

#[test]
fn selection_items_expand_at_the_geometry_phase_and_accept_priority() {
    let r = read(
        "0.4",
        "%s = {#R2 #R3} @SELECT-INSTANCE; %c = {#R1 %s #R4} @ORDER(\"left-to-right\"); %c @PRIORITY(\"high\")",
    );
    assert_eq!(errors(&r), []);
    assert_eq!(orders(&r), [["#R1", "#R2", "#R4"], ["#R1", "#R3", "#R4"]]);
    assert!(r.problems[0].facts.iter().all(|f| f.priority.as_str() == "high"));
    let r = read("0.4", "%s = {#R2 #R3} @SELECT-INSTANCE; %c = %s @ORDER(\"left-to-right\"); %c @PRIORITY(\"high\")");
    assert_eq!(errors(&r), []);
    assert_eq!(orders(&r), [["#R2", "#R3"]]);
}

#[test]
fn edits_of_selections_and_unions_preserve_written_items() {
    for initial in ["{#R1} @SELECT-INSTANCE", "{#R1} @UNION"] {
        assert_eq!(
            rows(&format!("%s = {initial}; %s @ORDER(\"left-to-right\"); {{#R2 #R3}} @ADD-TO %s")),
            [["#R1", "#R2"], ["#R1", "#R3"]]
        );
    }
    assert_eq!(rows("%s = {#R2}; %c = {#R1 %s #R4} @ORDER(\"left-to-right\"); {#R3} @SELECT-INSTANCE => @ADD-TO %s; %c @PRIORITY(\"high\")"), [["#R1", "#R2", "#R4"], ["#R1", "#R3", "#R4"]]);
}

#[test]
fn priority_on_a_collection_keeps_pending_single_constraint_handles() {
    assert!(rows("%s = {#R1 #R2} @SELECT-INSTANCE; %c = %s @ORDER(\"left-to-right\"); {%c} @PRIORITY(\"disabled\")")
        .is_empty());
    let r =
        read("0.4", "%s = {#R1 #R2} @SELECT-INSTANCE; %c = %s @ORDER(\"left-to-right\"); %c{1} @PRIORITY(\"high\")");
    assert!(errors(&r).iter().any(|(c, _)| *c == codes::SET_PLACES_WRONG_KIND));
}

#[test]
fn selection_dependencies_prevent_indexing_expanded_results() {
    for edit in ["@ADD-TO", "@REMOVE-FROM"] {
        let r = read("0.4", &format!("%s = {{#R2 #R3}}; %c = {{#R1 %s #R4}} @ORDER(\"left-to-right\"); %c{{1}} @PRIORITY(\"high\"); {{#R3}} @SELECT-INSTANCE => {edit} %s"));
        assert!(errors(&r).iter().any(|(c, _)| *c == codes::SET_PLACES_OF_SELECTION), "{:?}", errors(&r));
    }
}

#[test]
fn generation_is_finished_before_constraints_read_selections() {
    let r = read("0.4", "$S = @DEFINE-SYMBOL { (-1,-1) (1,1) @RECTANGLE }; $D = @DEFINE(roles=[\"n\"]) {}; # @SELECT-NET(name=\"a\") => @AS $D => @SET-SYMBOL $S; %s = # @SELECT-USAGE $D; {#R1 %s #R4} @ORDER(\"left-to-right\")");
    assert_eq!(errors(&r), []);
    let p = &r.problems[0];
    let members = p
        .facts
        .iter()
        .find_map(|f| match &f.kind {
            FactKind::Order { members, .. } => Some(members),
            _ => None,
        })
        .unwrap();
    assert_eq!(members.len(), 3);
    assert!(matches!(members[1], PointRef::Origin(k) if p.objects[k].kind == rnl_runtime::problem::ObjectKind::Usage));
}

#[test]
fn scoped_constraint_expansion_uses_the_source_file_version() {
    for (outer, inner, count) in [("0.4", "0.3", 2), ("0.3", "0.4", 1)] {
        let files = MemoryProvider::new()
            .with("t.sp", &format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"{outer}\") }}\n.include \"inc.sp\"\n* @RNL(scope=\"child\") {{ {{#R1 {{#R2 #R3}} #R4}} @ORDER(\"left-to-right\") }}\n.end\n"))
            .with("inc.sp", &format!("* inc\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a b\nR1 a b 1k\nR2 b c 1k\nR3 c d 1k\nR4 d 0 1k\n.ends\n"));
        let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        assert_eq!(errors(&r), []);
        assert_eq!(
            r.problems.iter().flat_map(|p| &p.facts).filter(|f| matches!(f.kind, FactKind::Order { .. })).count(),
            count
        );
    }
}

#[test]
fn a_selection_can_fill_the_two_slots_by_the_single_set_split() {
    let r = read("0.4", "%s = {#R1 #R2} @SELECT-INSTANCE; %s @DISTANCE(\"left-to-right\", min=1)");
    assert_eq!(errors(&r), []);
    assert_eq!(r.problems[0].facts.len(), 1);
}

#[test]
fn released_files_keep_composite_constraints() {
    for version in ["0.2", "0.3"] {
        let r = read(version, "{#R1 {#R2 #R3} #R4} @ORDER(\"left-to-right\"); {#R1 #R2} {#R3 #R4} @DISTANCE(\"left-to-right\", min=1); #R1 {#R2 #R3} @ORIENT-RELATION(\"R0\")");
        assert_eq!(errors(&r), []);
        assert_eq!(r.problems[0].facts.len(), 3);
        assert!(r.problems[0]
            .facts
            .iter()
            .any(|f| matches!(&f.kind, FactKind::Distance { from, to, .. } if from.len()==2 && to.len()==2)));
    }
}

#[test]
fn edits_of_pending_constraint_collections_survive_materialization() {
    assert!(rows("%extra = {#R3 #R4} @ORDER(\"left-to-right\"); %s = {#R2 #R3} @SELECT-INSTANCE; %c = {#R1 %s} @ORDER(\"left-to-right\"); %extra @ADD-TO %c; %c @PRIORITY(\"disabled\")").is_empty());
    let r = read("0.3", "%s = {#R1} @UNION; #R2 @ADD-TO %s");
    assert_eq!(errors(&r)[0].0, codes::SET_EDIT_TARGET_NOT_SET);
}

#[test]
fn invalid_members_fail_only_their_own_combinations() {
    for setup in ["%bad = &b", "$D = @DEFINE(roles=[\"a\"]) {}; %bad = $D", "%bad = (0,0) (1,1) @LINE"] {
        let r = read("0.4", &format!("{setup}; {{#R1 {{#R2 %bad}}}} @ORDER(\"left-to-right\")"));
        assert_eq!(errors(&r).len(), 1, "{:?}", errors(&r));
        assert_eq!(errors(&r)[0].0, codes::CONSTRAINT_MEMBER_NOT_LOCATABLE);
        assert_eq!(orders(&r), [["#R1", "#R2"]]);
    }
}

#[test]
fn sets_have_no_position_in_0_4() {
    for initial in ["{#R1 #R2}", "{#R1 #R2} @UNION", "# @SELECT-INSTANCE"] {
        let r = read("0.4", &format!("%s = {initial}; {{%s::position #R3}} @ALIGN(\"x\")"));
        assert!(errors(&r).iter().any(|(code, _)| *code == codes::SET_POSITION_04), "{:?}", errors(&r));
    }
    let r = read("0.3", "%s = {#R1 #R2}; {%s::position #R3} @ALIGN(\"x\")");
    assert_eq!(errors(&r), []);
}

#[test]
fn centred_graphics_contribute_their_centres() {
    for graphic in ["@RECTANGLE(width=2,height=3)", "@CIRCLE(radius=2)", "@ELLIPSE(width=2,height=3)"] {
        let r = read("0.4", &format!("%p = (_, _); %g = %p {graphic}; {{%g #R1}} @ALIGN(\"x\")"));
        assert_eq!(errors(&r), []);
        assert!(r.problems[0].facts.iter().any(|f| matches!(&f.kind, FactKind::Align { members, .. } if members.len()==2 && matches!(members[0], PointRef::Origin(k) if r.problems[0].objects[k].label=="%p"))));
    }
    let r = read("0.4", "%g = (0,0) (1,1) @RECTANGLE; {%g #R1} @ALIGN(\"x\")");
    assert_eq!(errors(&r)[0].0, codes::CONSTRAINT_MEMBER_NOT_LOCATABLE);
}

#[test]
fn conflicts_are_decided_independently_for_each_pair() {
    let r = read("0.4", "#R1 @PLACE(0,0); #R2 @PLACE(10,0); #R3 @PLACE(3,0); #R4 @PLACE(12,0); {#R1 #R2} {#R3 #R4} @DISTANCE(\"left-to-right\",min=2,priority=\"low\")");
    assert_eq!(errors(&r), []);
    let statuses: Vec<_> = r.model.levels[0].constraints.iter().map(|c| c.status.as_str()).collect();
    assert_eq!(statuses.iter().filter(|s| **s == "ok").count(), 3);
    assert_eq!(statuses.iter().filter(|s| **s == "dropped").count(), 1);
}

#[test]
fn package_constraints_use_each_levels_final_members() {
    let files = MemoryProvider::new()
        .with("p.rnl", "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"p\",version=\"1\") }\n@RNL { $S = {}; $ROW = $S @ORDER(\"left-to-right\") }")
        .with("t.sp", "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"p\",version=\"1\",path=\"p.rnl\") }\nR1 a b 1k\nR2 b c 1k\nR3 c 0 1k\n* @RNL { $ROW @PRIORITY(\"high\"); #R1 @ADD-TO $S; {#R2 #R3} @ADD-TO $S }\n.subckt child a b\nR4 a b 1k\nR5 b c 1k\nR6 c 0 1k\n* @RNL { $ROW @PRIORITY(\"high\"); #R4 @ADD-TO $S; {#R5 #R6} @ADD-TO $S }\n.ends\n.end\n");
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert_eq!(errors(&r), []);
    assert_eq!(r.problems.len(), 2);
    for p in &r.problems {
        assert_eq!(p.facts.len(), 2);
        assert!(p.facts.iter().all(|f| f.label == "$ROW" && f.priority.as_str() == "high"));
    }
}

#[test]
fn generated_bodies_can_set_the_priority_of_their_expanded_constraints() {
    let r = read("0.4", "%a = {#R1} @SELECT-INSTANCE; %s = {#R2 #R3} @SELECT-INSTANCE; $D = @DEFINE(roles=[\"a\",\"{refs}\"]) { %c = {?a ?refs} @ORDER(\"left-to-right\"); %c @PRIORITY(\"high\") }; %a(a) %s(refs) @AS $D");
    assert_eq!(errors(&r), []);
    assert_eq!(orders(&r), [["#R1", "#R2"], ["#R1", "#R3"]]);
    assert!(r.problems[0].facts.iter().all(|f| f.priority.as_str() == "high"));
}

#[test]
fn result_sets_keep_the_dependencies_of_their_inputs() {
    for input in ["{#R2 #R3}", "{#R2 #R3} @SELECT-INSTANCE"] {
        let r = read("0.4", &format!("%s = {input}; %c = {{#R1 %s}} @ORDER(\"left-to-right\"); %c @ADD-TO %s"));
        assert!(errors(&r).iter().any(|(code, _)| *code == codes::CYCLE_OF_SETS), "{:?}", errors(&r));
        assert_eq!(orders(&r), [["#R1", "#R2"], ["#R1", "#R3"]]);
    }
}

#[test]
fn splitting_constraint_sets_returns_constraints_instead_of_their_inputs() {
    let r = read("0.4", "%c = {#R1 {#R2 #R3}} @ORDER(\"left-to-right\"); %c @DISTANCE(\"left-to-right\", min=1)");
    assert_eq!(errors(&r).len(), 1, "{:?}", errors(&r));
    assert_eq!(errors(&r)[0].0, codes::CONSTRAINT_MEMBER_NOT_LOCATABLE);
    assert_eq!(orders(&r), [["#R1", "#R2"], ["#R1", "#R3"]]);
}
