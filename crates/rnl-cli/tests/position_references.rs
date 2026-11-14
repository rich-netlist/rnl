//! Explicit position references must have a position of their own (5.2.4, 6.5).
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(version: &str, body: &str) -> Interpretation {
    let source = format!(
        "* positions\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {} }}\n.end\n",
        body.replace('\n', "\n* ")
    );
    interpret_file(
        Path::new("positions.sp"),
        None,
        &MemoryProvider::new().with("positions.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

const PIN: &str = "$S = @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal=1) => %alias }; #R1 @SET-SYMBOL $S => @PLACE(3,4)";

#[test]
fn a_net_wire_or_multi_point_graphic_has_no_position_subobject() {
    for body in [
        "&a::position @CIRCLE(radius=1)",
        "%w=&a @WIRE {(0,0) (1,0)}; %w::position @CIRCLE(radius=1)",
        "%l= @LINE {(0,0) (1,0)}; %l::position @CIRCLE(radius=1)",
        "%a= @ARC {(0,0) (1,1) (2,0)}; %a::position @CIRCLE(radius=1)",
        "%b= @BEZIER {(0,0) (1,1) (2,1) (3,0)}; %b::position @CIRCLE(radius=1)",
        "%r= @RECTANGLE {(0,0) (2,2)}; %r::position @CIRCLE(radius=1)",
        "%c= @CIRCLE {(0,0) (2,2)}; %c::position @CIRCLE(radius=1)",
        "%e= @ELLIPSE {(0,0) (2,2)}; %e::position @CIRCLE(radius=1)",
    ] {
        let r = read("0.4", body);
        assert!(errors(&r).iter().any(|e| e.contains("has no position sub-object")), "{body}: {:?}", errors(&r));
        let old = read("0.3", body);
        assert!(errors(&old).is_empty(), "{body}: {:?}", errors(&old));
    }
}

#[test]
fn objects_with_one_position_keep_their_explicit_origin() {
    for body in [
        format!("{PIN}; #R1::position @CIRCLE(radius=1)"),
        format!("{PIN}; #R1:1::position @CIRCLE(radius=1)"),
        format!("{PIN}; #R1::p::position @CIRCLE(radius=1)"),
        "%p=(3,4); %p::position @CIRCLE(radius=1)".into(),
        "%t=(3,4) @TEXT(\"x\"); %t::position @CIRCLE(radius=1)".into(),
        "%c=(3,4) @CIRCLE(radius=2); %c::position @CIRCLE(radius=1)".into(),
        "%r=(3,4) @RECTANGLE(width=2,height=2); %r::position @CIRCLE(radius=1)".into(),
        "%e=(3,4) @ELLIPSE(width=2,height=2); %e::position @CIRCLE(radius=1)".into(),
        "$D = @DEFINE(roles=[\"n\"]) {}; %u=&a @AS $D => @PLACE(3,4); %u::position @CIRCLE(radius=1)".into(),
    ] {
        let r = read("0.4", &body);
        assert!(errors(&r).is_empty(), "{body}: {:?}", errors(&r));
        assert_eq!(r.model.levels[0].shapes.last().unwrap().points, [(3.0, 4.0)], "{body}");
    }
}

#[test]
fn a_terminal_without_a_pin_cannot_get_a_position_through_a_reference() {
    for consumer in [
        "#R1:2::position @CIRCLE(radius=1)",
        "{#R1:2::position (0,0)} @ALIGN(\"x\")",
        "&b @WIRE {#R1:2::position (0,0)}",
        "$D = @DEFINE(roles=[[\"p\",\"point\"]]) {}; #R1:2::position @AS $D",
    ] {
        let r = read("0.4", &format!("{consumer}; {PIN}"));
        assert!(errors(&r).iter().any(|e| e.contains("position")), "{consumer}: {:?}", errors(&r));
        let old = read("0.3", &format!("{consumer}; {PIN}"));
        if consumer.starts_with("$D") {
            assert!(errors(&old).iter().any(|error| error.contains("takes point")), "{:?}", errors(&old));
        } else {
            assert!(errors(&old).is_empty(), "{consumer}: {:?}", errors(&old));
        }
    }
}

#[test]
fn role_positions_use_the_converted_role_kind() {
    for kind in ["pin", "point"] {
        let r = read(
            "0.4",
            &format!(
                r#"{PIN}; $D = @DEFINE(roles=[["t","{kind}"]]) {{ ?t::position @CIRCLE(radius=1) }}; #R1:1 @AS $D"#
            ),
        );
        assert!(errors(&r).is_empty(), "{kind}: {:?}", errors(&r));
        assert_eq!(r.model.levels[0].shapes.len(), 1);
    }
    let r = read(
        "0.4",
        &format!(r#"{PIN}; $D = @DEFINE(roles=[["t","net"]]) {{ ?t::position @CIRCLE(radius=1) }}; #R1::p @AS $D"#),
    );
    assert!(errors(&r).iter().any(|e| e.contains("position")), "{:?}", errors(&r));
    assert!(r.model.levels[0].shapes.is_empty());
}

#[test]
fn a_position_of_a_converted_role_can_fill_another_point_role() {
    let r = read(
        "0.4",
        &format!(
            r#"{PIN}; $P = @DEFINE(roles=[["p","point"]]) {{ ?p @CIRCLE(radius=1) }}; $D = @DEFINE(roles=[["t","pin"]]) {{ ?t::position @AS $P }}; #R1:1 @AS $D"#
        ),
    );
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(r.model.levels[0].shapes.len(), 1);
}

#[test]
fn a_wire_at_a_pin_position_does_not_connect_the_pin() {
    let r = read(
        "0.4",
        &format!(
            r#"{PIN}; &a @WIRE {{#R1::p::position (0,4)}}; $D = @DEFINE(roles=["p"]) {{}}; #R1 @SELECT-PIN(terminal=1,wired=false) => @AS $D"#
        ),
    );
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(r.problems[0].usages.iter().filter(|usage| usage.definition == "$D").count(), 1);
}

#[test]
fn symbol_local_pins_and_points_keep_their_positions() {
    for body in [
        "%p=(0,0) @PIN(terminal=1); %p::position @CIRCLE(radius=1)",
        "%p=(0,0); %p::position @CIRCLE(radius=1)",
        "%g=(0,0) @CIRCLE(radius=2); %g::position @CIRCLE(radius=1)",
        r#"%t=(0,0) @TEXT("local",dx=2,dy=3); %t::position @CIRCLE(radius=1)"#,
        r#"%t=? @TEXT("name",key="name") => @PLACE(2,3); %t::position @CIRCLE(radius=1)"#,
    ] {
        let r = read("0.4", &format!("$S = @DEFINE-SYMBOL {{ {body} }}; #R1 @SET-SYMBOL $S => @PLACE(0,0)"));
        assert!(errors(&r).is_empty(), "{body}: {:?}", errors(&r));
        let symbol = r.model.levels[0].symbols.iter().find(|symbol| symbol.name == "$S").unwrap();
        let expected = if body.starts_with("%t=") { (2.0, 3.0) } else { (0.0, 0.0) };
        assert_eq!(symbol.prims.last().unwrap().points, [expected], "{body}");
    }
}

#[test]
fn a_renderer_standin_terminal_still_has_a_position() {
    let r = read("0.4", "#R1 @PLACE(0,0); #R1:1::position @CIRCLE(radius=1)");
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(r.model.levels[0].shapes.len(), 1);
}

#[test]
fn position_reference_checks_follow_the_statement_version() {
    for (outer, inner) in [("0.3", "0.4"), ("0.4", "0.3")] {
        let top = format!("* top\n* @RNL(type=\"header\") {{ @VERSION(\"{outer}\") }}\n.include child.sp\n* @RNL(scope=\"child\") {{ &a::position @CIRCLE(radius=1) }}\n.end\n");
        let child = format!(
            "* child\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a b\nR1 a b 1k\n.ends\n"
        );
        let files = MemoryProvider::new().with("top.sp", &top).with("child.sp", &child);
        let r =
            interpret_file(Path::new("top.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        assert_eq!(
            errors(&r).iter().any(|e| e.contains("has no position sub-object")),
            outer == "0.4",
            "{outer}/{inner}: {:?}",
            errors(&r)
        );
    }
}

#[test]
fn position_aliases_keep_written_identity_before_and_after_symbol_assignment() {
    for first in [true, false] {
        let uses = r#"$D = @DEFINE(roles=[["p","point"]]) {}; {#R1::p::position #R1::alias::position #R1::p::position} @UNION => @AS $D"#;
        let body = if first { format!("{PIN}; {uses}") } else { format!("{uses}; {PIN}") };
        let r = read("0.4", &body);
        assert!(errors(&r).is_empty(), "{:?}", errors(&r));
        assert_eq!(r.problems[0].usages.iter().filter(|u| u.definition == "$D").count(), 2);
    }
}

#[test]
fn a_failed_role_position_does_not_modify_an_independent_object() {
    let r = read(
        "0.4",
        &format!(
            r#"{PIN};
        $D = @DEFINE(roles=[["p","net"]]) {{ ?p::position @CIRCLE(radius=1) }};
        #R1::p @AS $D;
        #R1::position @CIRCLE(radius=2)
    "#
        ),
    );
    assert!(errors(&r).iter().any(|e| e.contains("position")), "{:?}", errors(&r));
    assert_eq!(r.model.levels[0].shapes.len(), 1);
    assert_eq!(r.model.levels[0].shapes[0].points, [(3.0, 4.0)]);
}
