//! Selective routing retains independent trees and phase snapshots (10.2, 10.4, 13.7).
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

const DRAW: &str = r#"
$S = @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal=1) };
{#R1 #R2 #R3 #R4} @SET-SYMBOL $S => @ORIENT("R0");
#R1 @PLACE(0,0); #R2 @PLACE(10,0); #R3 @PLACE(30,0); #R4 @PLACE(40,0)
"#;

fn read(version: &str, body: &str) -> Interpretation {
    read_with_header(version, body, "")
}

fn read_with_header(version: &str, body: &str, header: &str) -> Interpretation {
    let source = format!("* routes\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 2k\nR3 a b 3k\nR4 a b 4k\n* @RNL{header} {{ {}; {} }}\n.end\n", DRAW.replace('\n', "\n* "), body.replace('\n', "\n* "));
    interpret_file(
        Path::new("routes.sp"),
        None,
        &MemoryProvider::new().with("routes.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn valid(body: &str) -> Interpretation {
    let r = read("0.4", body);
    assert!(errors(&r).is_empty(), "{body}: {:?}", errors(&r));
    r
}

fn pins(r: &Interpretation) -> Vec<Vec<String>> {
    let p = &r.problems[0];
    p.routes
        .iter()
        .map(|tree| {
            tree.pins
                .iter()
                .map(|point| {
                    let (object, _) = point.pin_key().unwrap();
                    p.objects[object].label.clone()
                })
                .collect()
        })
        .collect()
}

fn usages(r: &Interpretation, definition: &str) -> usize {
    r.problems[0].usages.iter().filter(|u| u.definition == definition).count()
}

#[test]
fn disjoint_object_requests_on_one_net_remain_two_trees() {
    let r = valid("{#R1 #R2} @ROUTE-OBJECT; {#R3 #R4} @ROUTE-OBJECT");
    assert_eq!(pins(&r), [vec!["#R1", "#R2"], vec!["#R3", "#R4"]]);
    assert!(r.problems[0].routes.iter().all(|tree| tree.tree));
}

#[test]
fn whole_net_routing_joins_all_bound_pins_and_ignores_empty_nets() {
    let r = valid("{&a &b} @ROUTE-NET");
    assert_eq!(pins(&r), [vec!["#R1", "#R2", "#R3", "#R4"]]);
}

#[test]
fn net_routing_reads_later_ordinary_set_edits_before_creating_results() {
    for (body, expected) in [
        ("%n={&b}; %n @ROUTE-NET; &a @ADD-TO %n", 1),
        ("%n={&a &b}; %n @ROUTE-NET; &a @REMOVE-FROM %n", 0),
        ("&a @ROUTE-NET; %n={&b}; %n @UNROUTE-NET; &a @ADD-TO %n", 0),
        ("&a @ROUTE-NET; %n={&a &b}; %n @UNROUTE-NET; &a @REMOVE-FROM %n", 1),
    ] {
        let r = valid(body);
        assert_eq!(r.problems[0].routes.len(), expected, "{body}");
    }
}

#[test]
fn net_routing_does_not_replay_later_generated_set_edits() {
    let r = valid(
        r#"
        $nets={&b}; $nets @ROUTE-NET;
        $ADD = @DEFINE(roles=[["n", "net"]]) { ?n @ADD-TO $nets };
        &a @SELECT-NET => @AS $ADD
    "#,
    );
    assert!(r.problems[0].routes.is_empty());
    assert_eq!(usages(&r, "$ADD"), 1);
}

#[test]
fn net_route_result_handles_include_later_members() {
    let r = valid(
        r#"
        $Both = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); (0,4) @PIN(terminal=2) };
        {#R1 #R2 #R3 #R4} @SET-SYMBOL $Both;
        $All = @DEFINE(roles=["r"]) {};
        $Second = @DEFINE(roles=["r"]) {};
        %nets={&a}; %routes=%nets @ROUTE-NET;
        %routes @AS $All; %routes{2} @AS $Second;
        &b @ADD-TO %nets
    "#,
    );
    assert_eq!(r.problems[0].routes.len(), 2);
    assert_eq!(usages(&r, "$All"), 2);
    assert_eq!(usages(&r, "$Second"), 1);
}

#[test]
fn shared_pins_and_written_wire_chains_merge_trees() {
    for bridge in
        ["{#R2 #R3} @ROUTE-OBJECT", "&a @WIRE {#R2:1 #R3:1}", "%j=(20,0); &a @WIRE {#R2:1 %j}; &a @WIRE {%j #R3:1}"]
    {
        let r = valid(&format!("{{#R1 #R2}} @ROUTE-OBJECT; {{#R3 #R4}} @ROUTE-OBJECT; {bridge}"));
        assert_eq!(pins(&r).len(), 1, "{bridge}");
        assert_eq!(pins(&r)[0].len(), 4);
    }
}

#[test]
fn coincident_points_and_pin_positions_do_not_join_trees() {
    for bridge in ["&a @WIRE {#R2:1 (20,0)}; &a @WIRE {(20,0) #R3:1}", "&a @WIRE {#R2::p::position #R3::p::position}"] {
        let r = valid(&format!("{{#R1 #R2}} @ROUTE-OBJECT; {{#R3 #R4}} @ROUTE-OBJECT; {bridge}"));
        assert_eq!(pins(&r).len(), 2, "{bridge}");
    }
}

#[test]
fn unroute_removes_pins_by_priority_and_written_order() {
    for body in [
        "&a @ROUTE-NET(priority=\"low\"); #R1 @UNROUTE-OBJECT(priority=\"high\")",
        "#R1 @UNROUTE-OBJECT(priority=\"high\"); &a @ROUTE-NET(priority=\"low\")",
        "&a @ROUTE-NET; #R1 @UNROUTE-OBJECT",
    ] {
        let r = valid(body);
        assert_eq!(pins(&r), [vec!["#R2", "#R3", "#R4"]], "{body}");
    }
    let r = valid("&a @ROUTE-NET; &a @UNROUTE-NET");
    assert!(r.problems[0].routes.is_empty());
}

#[test]
fn a_high_priority_pair_does_not_restore_the_weaker_whole_net_tree() {
    let r = valid(
        r#"
        &a @ROUTE-NET(priority="low");
        {#R2 #R3} @UNROUTE-OBJECT;
        {#R2 #R3} @ROUTE-OBJECT(priority="high")
    "#,
    );
    assert_eq!(pins(&r), [vec!["#R1", "#R4"], vec!["#R2", "#R3"]]);
}

#[test]
fn disabled_requests_and_disabled_usage_bodies_do_not_route() {
    let r = valid(
        r#"
        &a @ROUTE-NET(priority="disabled");
        $D = @DEFINE(roles=["n"]) { ?n @ROUTE-NET };
        &a @AS(priority="disabled") $D
    "#,
    );
    assert!(r.problems[0].routes.is_empty());
}

#[test]
fn object_routing_expands_usage_roles_and_also_its_own_pins() {
    let r = valid(
        r#"
        $U = @DEFINE(roles=["left","right","n"]) {};
        $V = @DEFINE-SYMBOL { (0,0) @PIN(terminal="n") };
        %u = #R1 #R2 &a @AS $U => @SET-SYMBOL $V => @PLACE(5,5);
        %u @ROUTE-OBJECT
    "#,
    );
    assert_eq!(pins(&r), [vec!["#R1", "#R2", "%u"]]);
}

#[test]
fn object_routing_passes_its_input_set_on_without_changing_identity() {
    let r = valid(
        r#"
        %s={#R1 #R2}; %result=%s @ROUTE-OBJECT;
        #R3 @ADD-TO %result; %s @ROUTE-OBJECT
    "#,
    );
    assert_eq!(pins(&r), [vec!["#R1", "#R2", "#R3"]]);
}

#[test]
fn a_terminal_selects_its_last_pin_but_an_instance_selects_every_pin() {
    let r = valid(
        r#"
        $T = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); (0,2) @PIN(terminal=1) };
        #R1 @SET-SYMBOL $T;
        {#R1:1 #R2:1} @ROUTE-OBJECT
    "#,
    );
    let p = &r.problems[0];
    assert_eq!(p.routes[0].pins[0].pin_key().unwrap().1, 1);
    let all = valid(
        r#"
        $T = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); (0,2) @PIN(terminal=1) };
        #R1 @SET-SYMBOL $T; {#R1 #R2} @ROUTE-OBJECT
    "#,
    );
    assert_eq!(all.problems[0].routes[0].pins.len(), 3);
}

#[test]
fn route_rules_read_only_the_direct_request_snapshot() {
    let r = valid(
        r#"
        &a @SELECT-NET(wired=false) => @ROUTE-NET;
        &a @SELECT-NET(wired=true) => @UNROUTE-NET;
        $D= @DEFINE(roles=["p"]) {};
        # @SELECT-PIN(wired=false) => @AS $D
    "#,
    );
    assert_eq!(pins(&r), [vec!["#R1", "#R2", "#R3", "#R4"]]);
    assert_eq!(usages(&r, "$D"), 0);
}

#[test]
fn generation_and_later_conditions_never_read_generated_routes() {
    let r = valid(
        r#"
        $D= @DEFINE(roles=["p"]) { ?p::net @ROUTE-NET };
        $E= @DEFINE(roles=["p"]) {};
        # @SELECT-PIN(wired=false) => @AS $D;
        # @SELECT-PIN(wired=false) => @AS $E;
        &a @ASSERT-NET(wired=false)
    "#,
    );
    assert_eq!(usages(&r, "$D"), 4);
    assert_eq!(usages(&r, "$E"), 4);
    assert_eq!(pins(&r).len(), 1);
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Violation), "{:?}", r.diagnostics());
}

#[test]
fn unroute_never_removes_an_explicit_wire_from_wired_conditions() {
    let r = valid(
        r#"
        &a @WIRE {#R1:1 #R2:1}; &a @ROUTE-NET; &a @UNROUTE-NET(priority="high");
        $D= @DEFINE(roles=["p"]) {}; # @SELECT-PIN(wired=true) => @AS $D;
        &a @ASSERT-NET(wired=true)
    "#,
    );
    assert!(r.problems[0].routes.is_empty());
    assert_eq!(r.problems[0].wires.len(), 1);
    assert_eq!(usages(&r, "$D"), 2);
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Violation));
}

#[test]
fn a_typed_point_role_cannot_recover_its_source_pin_for_routing() {
    let r = valid(
        r#"
        $D= @DEFINE(roles=[["p","point"],"other"]) { {?p ?other} @ROUTE-OBJECT };
        #R1:1 #R2 @AS $D
    "#,
    );
    assert!(r.problems[0].routes.is_empty());
}

#[test]
fn net_requests_convert_terminal_and_pin_inputs_but_do_not_split_bus_conversions() {
    for input in ["#R1:1", "#R1::p"] {
        let r = valid(&format!("{input} @ROUTE-NET"));
        assert_eq!(pins(&r)[0].len(), 4);
    }
    let r = read(
        "0.4",
        r#"
        $B= @DEFINE-SYMBOL { %bus=(0,0) @PIN(terminal=[1]) };
        #R1 @SET-SYMBOL $B; #R1::bus @ROUTE-NET
    "#,
    );
    assert!(!errors(&r).is_empty(), "{:?}", r.diagnostics());
}

#[test]
fn routing_vocabulary_is_versioned_and_legacy_intents_keep_their_shape() {
    let old = read("0.3", "&a @ROUTE");
    assert!(errors(&old).is_empty());
    assert_eq!(old.problems[0].routes.len(), 1);
    assert!(!old.problems[0].routes[0].tree);
    assert!(!errors(&read("0.4", "&a @ROUTE")).is_empty());
    for keyword in ["ROUTE-NET", "ROUTE-OBJECT", "UNROUTE-NET", "UNROUTE-OBJECT"] {
        assert!(!errors(&read("0.3", &format!("&a @{keyword}"))).is_empty(), "{keyword}");
    }
}

#[test]
fn typed_collection_roles_keep_members_from_before_generation() {
    let r = valid(
        r#"
        $s={#R1 #R2};
        $D = @DEFINE(roles=[["{r}","instance"]]) { ?r @ROUTE-OBJECT };
        $ADD = @DEFINE(roles=["new"]) { ?new @ADD-TO $s };
        $s @AS $D;
        # @SELECT-INSTANCE(name="R3") => @AS $ADD
    "#,
    );
    assert_eq!(pins(&r), [vec!["#R1", "#R2"]]);
    let usage = r.problems[0].usages.iter().find(|usage| usage.definition == "$D").unwrap();
    assert_eq!(usage.roles[0].members.len(), 3, "the later edit must actually take effect");
}

#[test]
fn ordinary_later_edits_are_counted_before_typed_pin_conversion() {
    let r = valid(
        r#"
        %s={#R1:1 #R2:1};
        $D = @DEFINE(roles=[["{r}","pin"]]) { ?r @ROUTE-OBJECT };
        %s @AS $D;
        #R3:1 @ADD-TO %s
    "#,
    );
    assert_eq!(pins(&r), [vec!["#R1", "#R2", "#R3"]]);
}

#[test]
fn empty_node_pins_are_only_joined_by_the_explicit_wire() {
    let r = valid(
        r#"
        $V = @DEFINE(roles=[["n","net"]]) {};
        $P = @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal="n") };
        %u=$V @ATTACH & => @SET-SYMBOL $P => @PLACE(0,3);
        %v=$V @ATTACH & => @SET-SYMBOL $P => @PLACE(5,3);
        {%u %v #R1} @ROUTE-OBJECT;
        %u::p @ROUTE-NET;
        & @ROUTE-NET;
        & @WIRE {%u::p %v::p}
    "#,
    );
    assert!(r.problems[0].routes.is_empty());
    assert_eq!(r.problems[0].wires.len(), 1);
    assert!(r.problems[0].wires[0].nets.is_empty());
}

#[test]
fn a_generated_usage_wire_joins_final_trees() {
    let r = valid(
        r#"
        {#R1 #R2} @ROUTE-OBJECT;
        {#R3 #R4} @ROUTE-OBJECT;
        $JOIN = @DEFINE(roles=[["a","instance"],["b","instance"]]) {
            ?a:1::net @WIRE {?a:1 ?b:1}
        };
        # @SELECT-INSTANCE(name="R2") => #R3 @AS $JOIN
    "#,
    );
    assert_eq!(pins(&r).len(), 1);
    assert_eq!(pins(&r)[0].len(), 4);
    assert_eq!(r.problems[0].routes[0].wires.len(), 1);
}

#[test]
fn an_explicit_route_priority_overrides_the_disabled_record_default() {
    for (body, expected) in [
        ("&a @ROUTE-NET", 0),
        ("&a @ROUTE-NET(priority=\"high\"); &a @UNROUTE-NET", 1),
        ("&a @ROUTE-NET(priority=\"normal\"); &a @UNROUTE-NET(priority=\"high\")", 0),
    ] {
        let r = read_with_header("0.4", body, r#"(priority="disabled")"#);
        assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
        assert_eq!(r.problems[0].routes.len(), expected, "{body}");
    }
}

#[test]
fn viewer_routes_preserve_independent_trees_without_changing_legacy_json() {
    for (version, body, count, tree) in
        [("0.4", "{#R1 #R2} @ROUTE-OBJECT; {#R3 #R4} @ROUTE-OBJECT", 2, true), ("0.3", "&a @ROUTE", 1, false)]
    {
        let r = read(version, body);
        assert!(errors(&r).is_empty());
        let input = &r.doc.sources[0].1;
        let document = rnl_view::build(
            "routes.sp",
            &MemoryProvider::new().with("routes.sp", input),
            &rnl_view::BuildOptions {
                constraints: true,
                runtime: Options { std: false, ..Default::default() },
                ..Default::default()
            },
        )
        .unwrap();
        let data = serde_json::to_value(document.levels[0].constraints.as_ref().unwrap()).unwrap();
        let routes = data["routes"].as_array().unwrap();
        assert_eq!(routes.len(), count);
        for route in routes {
            assert_eq!(route.get("tree"), tree.then_some(&serde_json::Value::Bool(true)));
        }
        if tree {
            assert_eq!(routes[0]["pins"].as_array().unwrap().len(), 2);
        }
    }
}

#[test]
fn independent_trees_solve_and_write_back_without_cross_tree_wires() {
    use rnl_runtime::solution::WirePoint;
    let r = valid("{#R1 #R2} @ROUTE-OBJECT; {#R3 #R4} @ROUTE-OBJECT");
    let p = &r.problems[0];
    let solved = rnl_solver::solve(p, &rnl_solver::SolverOptions::without_time_limit());
    assert!(solved.report.errors.is_empty(), "{:?}", solved.report.errors);
    assert!(solved.report.unrouted.is_empty(), "{:?}", solved.report.unrouted);
    assert!(
        solved.report.notes.iter().any(|note| note.contains("2 routing trees on 1 net")),
        "{:?}",
        solved.report.notes
    );
    let ids: std::collections::BTreeSet<_> = solved.solution.wires.iter().map(|wire| wire.route).collect();
    assert_eq!(ids, [0, 1].into_iter().collect());
    for wire in &solved.solution.wires {
        for point in &wire.points {
            if let WirePoint::Anchor(anchor) = point {
                assert!(p.routes[wire.route].pins.contains(anchor), "cross-tree anchor: {wire:?}");
            }
        }
    }
    let written = rnl_runtime::solution::write_record(p, &solved.solution, "test", "hash");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert_eq!(written.record.matches("@WIRE").count(), 2, "{}", written.record);
    let text = rnl_runtime::writeback::append_record(&r.doc, 0, &r.doc.sources[0].1, &written.record);
    let again = interpret_file(
        Path::new("routes.sp"),
        None,
        &MemoryProvider::new().with("routes.sp", &text),
        &Options { std: false, ..Default::default() },
    )
    .unwrap();
    assert!(errors(&again).is_empty(), "{:?}", again.diagnostics());
    assert_eq!(again.model.levels[0].wires.len(), 2);
}

#[test]
fn partial_and_complete_unroute_overrides_are_both_hints() {
    for (tail, count) in [("#R1 @UNROUTE-OBJECT", 1), ("&a @UNROUTE-NET", 4)] {
        let r = valid(&format!("&a @ROUTE-NET; {tail}"));
        let diagnostics: Vec<_> =
            r.diagnostics().into_iter().filter(|d| d.code == rnl_runtime::diag::codes::ROUTE_OVERRULED).collect();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Hint);
        assert!(diagnostics[0].message.contains(&format!("{count} of 4 pins")));
        assert!(diagnostics[0].loc.is_some());
    }
    let r = valid("&a @ROUTE-NET(priority=\"disabled\"); &a @UNROUTE-NET");
    assert!(!r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::ROUTE_OVERRULED));
}

#[test]
fn joined_trees_keep_the_first_style_and_only_hint_when_styles_differ() {
    for (second, hints) in [("#ff0000", 0), ("#0000ff", 1)] {
        let r = valid(&format!(
            "{{#R1 #R2}} @ROUTE-OBJECT(linecolor=\"#ff0000\"); {{#R2 #R3}} @ROUTE-OBJECT(linecolor=\"{second}\")"
        ));
        assert_eq!(r.problems[0].routes.len(), 1);
        assert!(r.problems[0].routes[0]
            .style
            .iter()
            .any(|(key, value)| key == "linecolor" && value.str() == Some("#ff0000")));
        let diagnostics: Vec<_> = r
            .diagnostics()
            .into_iter()
            .filter(|d| d.code == rnl_runtime::diag::codes::ROUTE_TREES_DIFFERENT_STYLES)
            .collect();
        assert_eq!(diagnostics.len(), hints);
        assert!(diagnostics.iter().all(|d| d.severity == Severity::Hint));
    }
}

#[test]
fn fixed_net_routing_body_errors_are_reported_once_even_when_unused() {
    for keyword in ["ROUTE-NET", "UNROUTE-NET"] {
        for body in [
            format!("(0,0) @{keyword}"),
            format!("%p=(0,0); %p @{keyword}"),
            format!("%p @{keyword}; %p=(0,0)"),
            format!("%q=%p; %p=(0,0); %q @{keyword}"),
            format!("(0,0) @TEXT(\"bad\") => @{keyword}"),
            format!("{{(0,0)}} @{keyword}"),
        ] {
            for calls in ["", "$D @ATTACH; $D @ATTACH"] {
                let r = read("0.4", &format!("$D = @DEFINE(roles=[]) {{ {body} }}; {calls}"));
                let diagnostics: Vec<_> = r
                    .diagnostics()
                    .into_iter()
                    .filter(|d| d.code == rnl_runtime::diag::codes::ROUTE_TARGET_NOT_NET)
                    .collect();
                assert_eq!(diagnostics.len(), 1, "{body}; {calls}: {:?}", r.diagnostics());
                assert!(diagnostics[0].origin.is_empty());
            }
        }
    }
}

#[test]
fn fixed_route_types_include_definitions_and_declared_point_roles() {
    for body in [
        "$Other = @DEFINE-SYMBOL {}; $D = @DEFINE(roles=[]) { $Other @ROUTE-NET }",
        "$D = @DEFINE(roles=[]) { %Other = @DEFINE-STYLE {}; %Other @UNROUTE-NET }",
        "$D = @DEFINE(roles=[[\"p\",\"point\"]]) { ?p @ROUTE-NET }",
        "$D = @DEFINE(roles=[[\"p\",\"point\"]]) { ?p @UNROUTE-NET }",
    ] {
        let r = read("0.4", body);
        assert_eq!(
            r.diagnostics().into_iter().filter(|d| d.code == rnl_runtime::diag::codes::ROUTE_TARGET_NOT_NET).count(),
            1,
            "{body}: {:?}",
            r.diagnostics()
        );
    }
    for kind in ["net", "pin", "terminal"] {
        let r = read("0.4", &format!("$D = @DEFINE(roles=[[\"n\",\"{kind}\"]]) {{ ?n @ROUTE-NET }}"));
        assert!(errors(&r).is_empty(), "{kind}: {:?}", r.diagnostics());
    }
}

#[test]
fn unknown_route_role_kinds_still_check_each_application() {
    let r = read("0.4", "$D = @DEFINE(roles=[\"x\"]) { ?x @ROUTE-NET }; &a @AS $D; (0,0) @AS $D; (1,0) @AS $D");
    let diagnostics: Vec<_> =
        r.diagnostics().into_iter().filter(|d| d.code == rnl_runtime::diag::codes::ROUTE_TARGET_NOT_NET).collect();
    assert_eq!(diagnostics.len(), 2, "{:?}", r.diagnostics());
    assert!(diagnostics.iter().all(|d| !d.origin.is_empty()));
    assert_eq!(r.problems[0].routes.len(), 1);
}

#[test]
fn a_fixed_route_error_omits_the_whole_statement_and_preserves_siblings() {
    let r=read("0.4", "$D = @DEFINE(roles=[]) { (0,0) @TEXT(\"bad\") => @ROUTE-NET; (2,0) @CIRCLE(radius=1) }; $D @ATTACH => @PLACE(0,0); $D @ATTACH => @PLACE(10,0)");
    assert_eq!(
        r.diagnostics().into_iter().filter(|d| d.code == rnl_runtime::diag::codes::ROUTE_TARGET_NOT_NET).count(),
        1,
        "{:?}",
        r.diagnostics()
    );
    assert_eq!(r.model.levels[0].shapes.len(), 2);
    assert!(r.model.levels[0].texts.is_empty());
}

#[test]
fn a_virtual_only_wire_marks_its_pins_but_not_the_host_net_wired() {
    for (routing, nets, pins) in [("", 0, 2), ("{#R1 #R2} @ROUTE-OBJECT;", 1, 4)] {
        let r = valid(&format!(
            r#"
            $V = @DEFINE(roles=[["n","net"]]) {{}};
            $VS = @DEFINE-SYMBOL {{ %p=(0,0) @PIN(terminal="n") }};
            %u = $V @ATTACH &a; %v = $V @ATTACH &a;
            {{%u %v}} @SET-SYMBOL $VS;
            &a @WIRE {{%u::p %v::p}}; {routing}
            $Net = @DEFINE(roles=["x"]) {{}}; $Pin = @DEFINE(roles=["x"]) {{}};
            # @SELECT-NET(wired=true) => @AS $Net;
            # @SELECT-PIN(wired=true) => @AS $Pin
        "#
        ));
        assert_eq!(usages(&r, "$Net"), nets);
        assert_eq!(usages(&r, "$Pin"), pins);
    }
}
