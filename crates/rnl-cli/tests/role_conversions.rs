//! Kind conversions belong to an input occurrence, after symbol assignment when necessary (5.2.4).

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::{FactKind, Filler, Position, Slot};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(version: &str, body: &str) -> Interpretation {
    let body = body.replace('\n', "\n* ");
    let source = format!(
        "* conversions\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n.end\n"
    );
    interpret_file(
        Path::new("convert.sp"),
        None,
        &MemoryProvider::new().with("convert.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

const SYMBOL: &str = r#"$S = @DEFINE-SYMBOL { %p = (-3,0) @PIN(terminal=1,direction="left"); (3,0) @PIN(terminal=2) }; #R1 @SET-SYMBOL $S => @PLACE(0,0)"#;

#[test]
fn ancestor_selection_sees_generated_roles_after_symbol_assignment() {
    let r = checked(
        r#"
        $Bare = @DEFINE-SYMBOL { (0,0) @PIN(terminal="p") };
        $Labeled = @DEFINE-SYMBOL { $Bare; ? @TEXT("${p}",key="name") => @PLACE(0,-2) };
        $Marker = @DEFINE(roles=[["p","net"]]) {};
        # @SELECT-USAGE(recursive=true) $Marker => @SET-SYMBOL $Bare;
        $Stub = @DEFINE(roles=[["t","pin"]]) { %m = $Marker @ATTACH ?t::net };
        #R1 @SELECT-PIN => @AS $Stub;
        &b @SELECT-ANCESTOR-USAGE(recursive=true) $Marker => @SET-SYMBOL $Labeled
        "#,
    );
    let p = &r.problems[0];
    let symbols: Vec<_> = p.objects.iter().filter_map(|o| o.symbol).map(|i| p.symbols[i].name.as_str()).collect();
    assert_eq!(symbols.iter().filter(|s| **s == "$Bare").count(), 1);
    assert_eq!(symbols.iter().filter(|s| **s == "$Labeled").count(), 1);
}

#[test]
fn ancestor_selection_matches_converted_scalar_set_and_indirect_members() {
    for (roles, creation, scope) in [
        (r#"[["p","pin"]]"#, "%u = #R1:1 @AS $D", "#R1::p"),
        (r#"[["p","pin"]]"#, "%u = #R1::p @AS $D", "#R1:1 @SELECT-PIN"),
        (r#"[["{p}","pin"]]"#, "%u = {#R1:1 #R1:2} @AS $D", "#R1::p"),
        (r#"[["p","net"]]"#, "%u = #R1::p::net @AS $D", "&a"),
        (
            r#"["child"]"#,
            "$Inner = @DEFINE(roles=[[\"p\",\"pin\"]]) {}; %i = #R1:1 @AS $Inner; %u = %i @AS $D",
            "#R1::p",
        ),
    ] {
        let r = checked(&format!(
            r#"
            $D = @DEFINE(roles={roles}) {{}};
            {creation};
            {scope} => @SELECT-ANCESTOR-USAGE $D => @PLACE(10,20)
            "#
        ));
        let p = &r.problems[0];
        let object = p.objects.iter().find(|o| o.label == "%u").unwrap();
        assert!(
            matches!(
                object.position,
                Position::Own { x: Slot::Fixed { value: 10.0, .. }, y: Slot::Fixed { value: 20.0, .. } }
            ),
            "{creation}: {:?}",
            object.position
        );
    }
}

#[test]
fn ancestor_selection_keeps_written_roles_before_symbol_assignment() {
    let r = checked(
        r#"
        $D = @DEFINE(roles=[["p","pin"]]) {};
        %u = #R1:1 @AS $D;
        $Written = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=1) };
        #R1:1 @SELECT-ANCESTOR-USAGE $D => @SET-SYMBOL $Written
        "#,
    );
    let p = &r.problems[0];
    let object = p.objects.iter().find(|o| o.label == "%u").unwrap();
    assert_eq!(object.symbol.map(|i| p.symbols[i].name.as_str()), Some("$Written"));
}

#[test]
fn ancestor_selection_expands_a_bus_pin_net_role_after_symbols() {
    let r = read(
        "0.4",
        r#"
        $Part = @DEFINE-SYMBOL { %bus = (0,0) @PIN(terminal=[1,2]) };
        #R1 @SET-SYMBOL $Part;
        $D = @DEFINE(roles=[["{p}","net"]]) {};
        %u = {#R1::bus} @AS $D;
        &b @SELECT-ANCESTOR-USAGE $D => @PLACE(10,20)
        "#,
    );
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    let p = &r.problems[0];
    let object = p.objects.iter().find(|o| o.label == "%u").unwrap();
    assert!(
        matches!(
            object.position,
            Position::Own { x: Slot::Fixed { value: 10.0, .. }, y: Slot::Fixed { value: 20.0, .. } }
        ),
        "{:?}",
        object.position
    );
}

#[test]
fn wire_net_slots_convert_terminals_and_pins() {
    for source in ["#R1:1", "#R1::p", "{#R1:1 #R1::p}"] {
        let r = checked(&format!("{source} @WIRE {{(0,0) (1,1)}}"));
        assert_eq!(r.problems[0].wires.len(), 1);
        assert!(r.problems[0].wires[0].nets.iter().all(|net| net == "a"));
    }
}

#[test]
fn wire_net_slots_convert_bus_pins_after_symbol_assignment() {
    for terminals in ["[1]", "[1,2]"] {
        let r = read(
            "0.4",
            &format!(
                r#"
            $S = @DEFINE-SYMBOL {{ %p = (0,0) @PIN(terminal={terminals}) }};
            #R1::p @WIRE {{(0,0) (1,1)}};
            #R1 @SET-SYMBOL $S
        "#
            ),
        );
        assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
        let wire = &r.problems[0].wires[0];
        assert!(wire.bus);
        assert_eq!(wire.nets.len(), if terminals == "[1]" { 1 } else { 2 });
    }
}

#[test]
fn a_pin_role_can_fill_a_wire_net_slot_without_changing_the_role() {
    let r = checked(
        r#"
        $D = @DEFINE(roles=[["p","pin"]]) { ?p @WIRE {?p (0,2)} };
        #R1:1 @AS $D
    "#,
    );
    assert!(matches!(r.problems[0].usages[0].roles[0].members[..], [Filler::Pin { .. }]));
    assert_eq!(r.problems[0].wires[0].nets, ["a"]);
}

#[test]
fn wire_net_conversion_rejects_unbound_pins_and_preserves_old_versions() {
    let r = read(
        "0.4",
        r#"
        $S = @DEFINE-SYMBOL { %p = (0,0) @PIN };
        #R1::p @WIRE {(0,0) (1,1)}; #R1 @SET-SYMBOL $S;
        &a @WIRE {(2,0) (3,0)}
    "#,
    );
    assert!(r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::WIRE_NET_WRONG_KIND));
    assert_eq!(r.problems[0].wires.len(), 1);
    let old = read("0.3", "#R1:1 @WIRE {(0,0) (1,1)}");
    assert!(old.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::WIRE_NET_WRONG_KIND));
    assert!(old.problems[0].wires.is_empty());
}

fn checked(body: &str) -> Interpretation {
    let result = read("0.4", &format!("{SYMBOL}; {body}"));
    assert!(!result.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", result.diagnostics());
    result
}

#[test]
fn terminal_and_pin_fill_each_convertible_single_role() {
    for source in ["#R1:1", "#R1::p"] {
        for kind in ["pin", "point", "terminal", "net"] {
            let r = checked(&format!(r#"$D = @DEFINE(roles=[["r","{kind}"]]) {{}}; {source} @AS $D"#));
            let members = &r.problems[0].usages[0].roles[0].members;
            match kind {
                "net" => assert_eq!(members, &[Filler::Net("a".into())]),
                "pin" => assert!(matches!(members[..], [Filler::Pin { pin: 0, .. }])),
                "terminal" => assert!(matches!(members[..], [Filler::Terminal { .. }])),
                _ => assert_eq!(members.len(), 1),
            }
        }
    }
}

#[test]
fn body_operations_use_the_converted_net_and_pin() {
    let r = checked(
        r#"
        $D = @DEFINE(roles=[["n","net"],["p","pin"]]) {
            ?n @WIRE {?p (0,2)};
            ?p @ORIENT("up")
        };
        #R1:1 #R1:1 @AS $D
    "#,
    );
    assert_eq!(r.problems[0].wires[0].nets, ["a"]);
    assert!(r.problems[0].facts.iter().any(|f| matches!(f.kind, FactKind::PinOrient { .. })));
}

#[test]
fn exact_kind_wins_before_the_first_convertible_kind() {
    let r = checked(
        r#"
        $D = @DEFINE(roles=[["r",["point","terminal"]]]) {};
        #R1:1 @AS $D
    "#,
    );
    assert!(matches!(r.problems[0].usages[0].roles[0].members[..], [Filler::Terminal { .. }]));
}

#[test]
fn different_roles_of_one_input_convert_independently() {
    let r = checked(
        r#"
        $D = @DEFINE(roles=[["t","terminal"],["p","pin"],["n","net"]]) {};
        #R1:1 #R1:1 #R1:1 @AS $D;
        $T = @DEFINE(roles=[["r","terminal"]]) {}; #R1:1 @AS $T
    "#,
    );
    let roles = &r.problems[0].usages[0].roles;
    assert!(matches!(roles[0].members[..], [Filler::Terminal { .. }]));
    assert!(matches!(roles[1].members[..], [Filler::Pin { .. }]));
    assert_eq!(roles[2].members, [Filler::Net("a".into())]);
    assert!(matches!(r.problems[0].usages[1].roles[0].members[..], [Filler::Terminal { .. }]));
}

#[test]
fn set_roles_convert_members_after_all_edits() {
    let r = checked(
        r#"
        $D = @DEFINE(roles=[["{r}","net"]]) {};
        %s = {#R1:1}; %u = %s @AS $D;
        #R1:2 @ADD-TO %s; #R1:1 @REMOVE-FROM %s
    "#,
    );
    assert_eq!(r.problems[0].usages[0].roles[0].members, [Filler::Net("b".into())]);
}

#[test]
fn a_bus_pin_fails_a_single_role_without_repeating_creation() {
    for binding in ["[1]", "[1,2]"] {
        let r = read(
            "0.4",
            &format!(
                r#"
            $S = @DEFINE-SYMBOL {{ %p = (0,0) @PIN(terminal={binding}) }};
            $D = @DEFINE(roles=[["r","terminal"]]) {{}};
            #R1::p @AS $D; #R1 @SET-SYMBOL $S
        "#
            ),
        );
        assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
        assert!(r.problems[0].usages.is_empty());
    }
}

#[test]
fn old_typed_roles_do_not_convert() {
    let r = read("0.3", r#"$D = @DEFINE(roles=[["n","net"]]) {}; #R1:1 @AS $D"#);
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
    assert!(r.problems[0].usages.is_empty());
}

#[test]
fn a_point_role_takes_position_without_connecting_the_original_pin() {
    for kind in ["point", "pin"] {
        let r = checked(&format!(
            r#"
            $D = @DEFINE(roles=[["r","{kind}"],"n","q"]) {{ ?n @WIRE {{?r ?q}} }};
            #R1:1 &a (0,2) @AS $D
        "#
        ));
        assert_eq!(r.model.levels[0].wires[0].points[0], (-3.0, 0.0));
        let parts = r.model.levels[0].net_labels.iter().filter(|label| label.net == "a").count();
        assert_eq!(parts, if kind == "point" { 2 } else { 1 }, "{kind}");
    }
}

#[test]
fn nested_bodies_keep_the_outer_conversion() {
    let r = checked(
        r#"
        $INNER = @DEFINE(roles=["r","n","q"]) { ?n @WIRE {?r ?q} };
        $OUTER = @DEFINE(roles=[["p","point"],"n","q"]) { ?p ?n ?q @AS $INNER };
        %u = #R1:1 &a (0,2) @AS $OUTER;
        &a @WIRE {%u::p (0,-2)}
    "#,
    );
    assert_eq!(r.problems[0].usages.len(), 2);
    assert_eq!(r.model.levels[0].wires.len(), 2);
    assert_eq!(r.model.levels[0].net_labels.iter().filter(|label| label.net == "a").count(), 2);
}

#[test]
fn internal_input_views_do_not_add_written_set_members() {
    let r = checked(
        r#"
        $INNER = @DEFINE(roles=["r"]) {};
        $OUTER = @DEFINE(roles=[["p","point"],"original"]) {
            {?p ?original} @UNION => @AS $INNER
        };
        #R1:1 #R1:1 @AS $OUTER
    "#,
    );
    assert_eq!(r.problems[0].usages.iter().filter(|u| u.definition == "$INNER").count(), 1);
}

#[test]
fn converted_point_roles_cannot_bind_a_usage_symbol_pin() {
    let r = read(
        "0.4",
        &format!(
            r#"{SYMBOL};
        $B = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal="r") }};
        $D = @DEFINE(roles=[["r","point"]]) {{}};
        %u = #R1:1 @AS $D => @SET-SYMBOL $B => @PLACE(5,0)
    "#
        ),
    );
    assert!(r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::ROLE_PIN_FILLED_WITH_OTHER_KIND));
    assert_eq!(r.problems[0].usages.len(), 1);
    assert!(!r.model.levels[0].items.iter().any(|item| item.object_id == "%u"));
}

#[test]
fn a_generated_converted_role_keeps_its_original_writeback_reference() {
    let baseline = checked(r#"$D = @DEFINE(roles=["p"]) {}; #R1 @SELECT-TERMINAL => @AS $D"#);
    let r = checked(
        r#"
        $D = @DEFINE(roles=[["p","point"]]) {};
        #R1 @SELECT-TERMINAL => @AS $D
    "#,
    );
    assert_eq!(r.problems[0].usages.len(), 2);
    for (converted, original) in r.problems[0].usages.iter().zip(&baseline.problems[0].usages) {
        assert_eq!(converted.roles[0].written, original.roles[0].written);
        assert!(converted.roles[0].written.is_some());
    }
}

#[test]
fn a_set_member_conversion_is_retained_when_the_body_creates_usages() {
    let r = checked(
        r#"
        $INNER = @DEFINE(roles=["r"]) {};
        $D = @DEFINE(roles=[["{r}","pin"]]) { ?r @AS $INNER };
        {#R1:1 #R1:2} @AS $D
    "#,
    );
    let members: Vec<_> =
        r.problems[0].usages.iter().filter(|u| u.definition == "$INNER").map(|u| &u.roles[0].members).collect();
    assert_eq!(members.len(), 2);
    assert!(members.iter().all(|members| matches!(members[..], [Filler::Pin { .. }])));
}

#[test]
fn the_first_convertible_kind_decides_whether_a_wire_connects() {
    for (kinds, expected) in [(r#"["point","pin"]"#, 2), (r#"["pin","point"]"#, 1)] {
        let r = checked(&format!(
            r#"
            $D = @DEFINE(roles=[["r",{kinds}],"n","q"]) {{ ?n @WIRE {{?r ?q}} }};
            #R1:1 &a (0,2) @AS $D
        "#
        ));
        assert_eq!(r.model.levels[0].net_labels.iter().filter(|label| label.net == "a").count(), expected);
    }
}

#[test]
fn a_usage_pin_converts_to_its_network_but_not_a_terminal() {
    for kind in ["net", "terminal"] {
        let r = read(
            "0.4",
            &format!(
                r#"
            $U = @DEFINE(roles=["n"]) {{}};
            $S = @DEFINE-SYMBOL {{ %p = (0,0) @PIN(terminal="n") }};
            %u = &a @AS $U => @SET-SYMBOL $S;
            $D = @DEFINE(roles=[["r","{kind}"]]) {{}}; %u::p @AS $D
        "#
            ),
        );
        if kind == "net" {
            assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
            assert_eq!(r.problems[0].usages[1].roles[0].members, [Filler::Net("a".into())]);
        } else {
            assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
            assert_eq!(r.problems[0].usages.len(), 1);
        }
    }
}

#[test]
fn a_failed_reference_in_a_converted_set_role_fails_the_usage() {
    let r = read(
        "0.4",
        &format!(
            r#"{SYMBOL};
        $D = @DEFINE(roles=[["{{r}}","point"]]) {{}};
        {{#R1::missing #R1::p}} @AS $D
    "#
        ),
    );
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
    assert!(r.problems[0].usages.is_empty());
}
