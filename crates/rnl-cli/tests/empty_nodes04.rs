//! The empty node is a value for a net role or WIRE, never a selectable net (6.3).
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::{codes, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};

fn read(body: &str) -> Interpretation {
    let body = body.replace('\n', "\n* ");
    let source = format!(
        "* empty nodes\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n.end\n"
    );
    interpret_file(
        Path::new("empty.sp"),
        None,
        &MemoryProvider::new().with("empty.sp", &source),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap()
}
fn no_errors(result: &Interpretation) {
    assert!(!result.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", result.diagnostics());
}

const PAIR: &str = r#"
    $S = @DEFINE-SYMBOL { %pin=(0,0) @PIN(terminal="n") };
    $D = @DEFINE(roles=[["n","net"]]) { ? @SET-SYMBOL $S };
    %a = $D @ATTACH & => @PLACE(0,0);
    %b = $D @ATTACH & => @PLACE(4,0);
"#;

#[test]
fn a_single_net_role_can_be_filled_without_a_network() {
    for kind in [r#""net""#, r#"["terminal","net"]"#, r#"["net","point"]"#] {
        let r = read(&format!("$D= @DEFINE(roles=[[\"n\",{kind}]]) {{}}; %u=$D @ATTACH &"));
        no_errors(&r);
        assert_eq!(r.problems[0].usages.len(), 1);
    }
}

#[test]
fn untyped_and_non_net_roles_cannot_take_the_empty_node() {
    for roles in [r#"["n"]"#, r#"[["n","point"]]"#] {
        let r = read(&format!("$D= @DEFINE(roles={roles}) {{}}; $D @ATTACH &"));
        assert!(
            r.diagnostics().iter().any(|d| d.code == codes::USAGE_ROLE_WRONG_KIND),
            "{roles}: {:?}",
            r.diagnostics()
        );
    }
}

#[test]
fn independent_empty_node_pins_and_wires_have_no_network() {
    let r = read(&format!("{PAIR} & @WIRE {{%a::pin %b::pin}}"));
    no_errors(&r);
    assert_eq!(r.model.levels[0].wires.len(), 1);
    assert!(r.model.levels[0].wires[0].nets.is_empty());
    assert_eq!(r.problems[0].wires.len(), 1);
    assert!(r.problems[0].wires[0].nets.is_empty());
}

#[test]
fn the_net_subobject_of_a_bound_empty_pin_is_the_empty_node() {
    let r = read(&format!("{PAIR} %a::pin::net @WIRE {{%a::pin %b::pin}}"));
    no_errors(&r);
    assert_eq!(r.model.levels[0].wires.len(), 1);
    assert!(r.model.levels[0].wires[0].nets.is_empty());
}

#[test]
fn an_empty_node_wire_makes_its_pin_endpoints_wired() {
    let r = read(&format!(
        r#"{PAIR}
        & @WIRE {{%a::pin %b::pin}};
        $CHECK = @DEFINE(roles=[["p","pin"]]) {{}};
        {{%a::pin %b::pin}} @SELECT-PIN(wired=true) => @AS $CHECK
    "#
    ));
    no_errors(&r);
    assert_eq!(r.problems[0].usages.len(), 4);
}

#[test]
fn direct_empty_node_scopes_and_assertions_have_no_members() {
    let r = read(
        r#"$D= @DEFINE(roles=["x"]) {};
        & @SELECT-NET => @AS $D;
        & @SELECT-PIN => @AS $D;
        & @ASSERT-NET; & @ASSERT-IN {}"#,
    );
    no_errors(&r);
    assert!(r.problems[0].usages.is_empty());
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Violation));
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Warning));
}

#[test]
fn an_empty_node_reached_through_a_role_does_not_warn() {
    let r = read(
        r#"$D= @DEFINE(roles=[["n","net"]]) {
        ?n @ASSERT-NET; ?n @SELECT-NET; ?n @ASSERT-IN {}
    }; $D @ATTACH &"#,
    );
    no_errors(&r);
    assert!(
        !r.diagnostics().iter().any(|d| matches!(d.severity, Severity::Warning | Severity::Violation)),
        "{:?}",
        r.diagnostics()
    );
}

#[test]
fn the_empty_node_has_no_bindable_handle() {
    for body in ["%n=&", "$n=&"] {
        let r = read(body);
        assert!(
            r.diagnostics().iter().any(|d| d.severity == Severity::Error && d.message.contains("empty node")),
            "{:?}",
            r.diagnostics()
        );
    }
}

#[test]
fn named_missing_nets_remain_missing_and_unbound_pins_are_distinct() {
    let r = read("&missing @WIRE {(0,0) (1,0)}");
    assert!(r.diagnostics().iter().any(|d| d.code == codes::REF_NO_NET));
    let r = read(
        r#"$S= @DEFINE-SYMBOL { %p=(0,0) @PIN }; #R1 @SET-SYMBOL $S;
        #R1::p::net @ASSERT-NET"#,
    );
    assert!(r.diagnostics().iter().any(|d| d.code == codes::PIN_ON_NO_NET));
}

#[test]
fn sets_drop_empty_nodes_before_counting_positions_or_building_buses() {
    let r = read(
        r#"
        %s = {& &a}; & @ADD-TO %s;
        $D = @DEFINE(roles=[["n","net"]]) {};
        %s{1} @AS $D;
        %s @WIRE {(0,0) (1,0)};
        {&} @WIRE {(0,1) (1,1)}
    "#,
    );
    assert_eq!(r.problems[0].usages.len(), 1);
    assert_eq!(r.problems[0].wires.len(), 1);
    assert_eq!(r.problems[0].wires[0].nets, ["a"]);
    assert_eq!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).count(), 1, "{:?}", r.diagnostics());
    assert!(r.diagnostics().iter().any(|d| d.code == codes::WIRE_NET_EMPTY_SET));
}

#[test]
fn wires_warn_in_both_directions_but_not_for_unbound_pins() {
    let r = read(&format!(
        r#"{PAIR}
        $FREE = @DEFINE-SYMBOL {{ %p=(0,0) @PIN; (1,0) @PIN(terminal=1) }};
        #R1 @SET-SYMBOL $FREE;
        &a @WIRE {{%a::pin (1,0)}};
        & @WIRE {{#R1:1 (2,0)}};
        &a @WIRE {{#R1::p (3,0)}};
        & @WIRE {{#R1::p (4,0)}}
    "#
    ));
    no_errors(&r);
    assert_eq!(r.problems[0].wires.len(), 4);
    assert_eq!(
        r.diagnostics().iter().filter(|d| d.code == codes::WIRE_NET_MISMATCH).count(),
        2,
        "{:?}",
        r.diagnostics()
    );
}

#[test]
fn no_network_interpolation_is_empty_and_indirect_roles_do_not_warn() {
    let r = read(&format!(
        r#"{PAIR}
        $TS = @DEFINE-SYMBOL {{ ? @TEXT("[${{n}}]") }};
        $TEXT = @DEFINE(roles=[["n","net"]]) {{ ? @SET-SYMBOL $TS }};
        %a::pin @AS $TEXT;
        $OUTER = @DEFINE(roles=[["n","net"]]) {{ ?n @AS $TEXT }};
        $OUTER @ATTACH &;
        %a::pin @TEXT("[${{net}}]");
        & @WIRE {{%a::pin %b::pin}} => @TEXT("[${{net}}]")
    "#
    ));
    no_errors(&r);
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Warning), "{:?}", r.diagnostics());
    assert_eq!(r.problems[0].usages.len(), 5);
}

#[test]
fn indirect_pin_net_roles_resolve_without_order_dependence() {
    let r = read(
        r#"
        $S = @DEFINE-SYMBOL { %pin=(0,0) @PIN(terminal="n") };
        $D = @DEFINE(roles=[["n","net"]]) { ? @SET-SYMBOL $S };
        %a = $D @ATTACH %b::pin::net;
        %b = $D @ATTACH %c::pin;
        %c = $D @ATTACH &;
        %a::pin::net @WIRE {%a::pin %b::pin};
        %b::pin::net @WIRE {%b::pin %c::pin}
    "#,
    );
    no_errors(&r);
    assert_eq!(r.problems[0].wires.len(), 2);
    assert!(r.problems[0].wires.iter().all(|wire| wire.nets.is_empty()));
    assert!(r.problems[0].usages.iter().all(|usage| usage.roles[0].members.is_empty()));
    assert_eq!(r.problems[0].usages[2].roles[0].written.as_deref(), Some("&"));
    assert!(r.model.levels[0]
        .objects
        .iter()
        .filter(|o| o.definition.is_some())
        .all(|o| matches!(o.attach[0].1, rnl_runtime::model::Attach::Set(ref members) if members.is_empty())));
}

#[test]
fn editing_a_wire_on_no_net_writes_a_bare_ampersand() {
    let request = serde_json::json!({
        "dialect":"spice", "language_version":"0.4", "author":"test",
        "netlist":"* empty nodes\nR1 a b 1k\n.end\n",
        "ops":[{"op":"add-wire", "level":"file", "nets":[],
                 "via":[{"at":[0,0]},{"at":[1,0]}]}]
    });
    let request = serde_json::from_value(request).unwrap();
    let written = rnl_edit::write(&request).unwrap();
    assert!(written.contains("& @WIRE"), "{written}");
    let result = interpret_file(
        Path::new("edited.sp"),
        None,
        &MemoryProvider::new().with("edited.sp", &written),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap();
    no_errors(&result);
    assert_eq!(result.problems[0].wires.len(), 1);
    assert!(result.problems[0].wires[0].nets.is_empty());
}

#[test]
fn a_bus_with_only_empty_converted_members_still_fails() {
    let r = read(&format!("{PAIR} {{%a::pin}} @WIRE {{(0,0) (1,0)}}"));
    assert!(r.diagnostics().iter().any(|d| d.code == codes::WIRE_NET_EMPTY_SET), "{:?}", r.diagnostics());
    assert!(r.problems[0].wires.is_empty());
}

#[test]
fn a_set_role_receives_no_member_and_only_direct_empty_nodes_warn() {
    let definitions = r#"
        $SET = @DEFINE(roles=[["{nets}","net"]]) {};
        $ONE = @DEFINE(roles=[["n","net"]]) { ?n @AS $SET }
    "#;
    let direct = read(&format!("{definitions}; & @AS $SET"));
    no_errors(&direct);
    assert!(direct.problems[0].usages[0].roles[0].members.is_empty());
    assert!(direct.diagnostics().iter().any(|d| d.code == codes::EMPTY_NODE_IGNORED));
    let indirect = read(&format!("{definitions}; & @AS $ONE"));
    no_errors(&indirect);
    assert_eq!(indirect.problems[0].usages.len(), 2);
    assert!(!indirect.diagnostics().iter().any(|d| d.severity == Severity::Warning));
}

#[test]
fn document_and_snapshot_keep_no_net_wires_and_pins_independent() {
    let body = format!("{PAIR} & @WIRE {{%a::pin %b::pin}}; & @WIRE {{%a::pin (0,2)}}").replace('\n', "\n* ");
    let source = format!(
        "* empty nodes\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n.end\n"
    );
    let files = MemoryProvider::new().with("empty.sp", &source);
    let options =
        rnl_view::BuildOptions { runtime: Options { std: false, ..Default::default() }, ..Default::default() };
    let doc = rnl_view::build("empty.sp", &files, &options).unwrap();
    let wires: Vec<_> = doc.levels[0].hits.iter().filter(|hit| hit.kind == "wire").collect();
    assert_eq!(wires.len(), 2);
    assert!(wires.iter().all(|hit| hit.objects.is_empty()));
    assert_ne!(wires[0].id, wires[1].id);
    let pins: Vec<_> = doc.levels[0].hits.iter().filter(|hit| hit.kind == "pin").collect();
    assert_eq!(pins.len(), 2);
    assert!(pins.iter().all(|hit| hit.objects.is_empty() && hit.pin.is_some()));
    assert_ne!(pins[0].pin, pins[1].pin);
    let snapshot =
        rnl_view::snapshot("empty.sp", &files, &options, rnl_view::Parts { drawing: false, host: false, model: true })
            .unwrap();
    let snapshot = serde_json::to_value(snapshot).unwrap();
    let model = &snapshot["levels"][0]["model"];
    assert_eq!(model["wires"].as_array().unwrap().len(), 2);
    assert!(model["wires"].as_array().unwrap().iter().all(|wire| wire["nets"] == serde_json::json!([])));
}
