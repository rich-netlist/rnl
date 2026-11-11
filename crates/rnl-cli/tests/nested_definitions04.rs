//! Fixed nested general definitions are checked even when their parent is unused (6.9).
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn read(version: &str, body: &str, used: bool) -> Interpretation {
    let calls = if used { "$D @ATTACH => @PLACE(0,0); $D @ATTACH => @PLACE(20,0)" } else { "" };
    let text = format!("* nested definitions\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ $D = @DEFINE(roles=[]) {{ {body} }}; {calls} }}\n");
    rnl_cli::interpret_file(
        Path::new("nested.sp"),
        None,
        &MemoryProvider::new().with("nested.sp", &text),
        &Options { std: false, ..Default::default() },
    )
    .unwrap()
}
fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}

#[test]
fn fixed_nested_bodies_are_checked_once_before_any_application() {
    for (body, expected) in [
        ("(0,0) @NOT-A-KEYWORD", codes::KEYWORD_UNKNOWN),
        ("?absent @CIRCLE(radius=1)", codes::REF_HOLE_UNKNOWN),
        ("#R1 @PLACE(0,0)", codes::DEFINE_BODY_HOST_REF),
    ] {
        for used in [false, true] {
            let r = read("0.4", &format!("%I = @DEFINE(roles=[]) {{ {body} }}"), used);
            assert_eq!(errors(&r), [expected], "{body}, used={used}: {:?}", r.diagnostics());
            assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| d.origin.is_empty()));
        }
    }
}

#[test]
fn nested_signatures_and_local_clashes_are_fixed_errors() {
    for declaration in [
        "%I = @DEFINE(roles=[]) { (0,0) @CIRCLE(radius=\"bad\") }",
        "%I = @DEFINE(roles=[]) { (0,0) @CIRCLE }",
        "%I = @DEFINE(roles=[\"r\"]) { %r=(0,0) }",
        "%I = @DEFINE(roles=[[\"r\",\"instance\"]]) { ?r::net }",
    ] {
        for used in [false, true] {
            let r = read("0.4", declaration, used);
            assert_eq!(errors(&r).len(), 1, "{declaration}, used={used}: {:?}", r.diagnostics());
            assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| d.origin.is_empty()));
        }
    }
}

#[test]
fn nested_roles_are_checked_in_their_own_scope() {
    for used in [false, true] {
        let r = read(
            "0.4",
            "%I = @DEFINE(roles=[\"inner\"], parameters=[[\"size\",1]]) { ?inner @CIRCLE(radius=?size) }",
            used,
        );
        assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    }
}

#[test]
fn released_unused_nested_bodies_keep_their_old_behavior() {
    for version in ["0.2", "0.3"] {
        let r = read(version, "%I = @DEFINE(roles=[]) { ?absent }", false);
        assert_eq!(errors(&r), [codes::REF_HOLE_UNKNOWN], "{:?}", r.diagnostics());
    }
}

#[test]
fn an_invalid_nested_pipe_is_omitted_and_its_sibling_still_draws() {
    let r = read(
        "0.4",
        "%I = @DEFINE(roles=[]) { (0,0) @CIRCLE(radius=2) => @NOT-A-KEYWORD; (8,0) @CIRCLE(radius=1) }; %I @ATTACH => @PLACE(0,0)",
        true,
    );
    assert_eq!(errors(&r), [codes::KEYWORD_UNKNOWN], "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 2);
}

#[test]
fn nested_parameter_errors_are_reused_by_real_creations() {
    let r = read(
        "0.4",
        "%I = @DEFINE(roles=[]) { (0,0) @CIRCLE(radius=?missing); (8,0) @CIRCLE(radius=1) }; %I @ATTACH => @PLACE(0,0)",
        true,
    );
    assert_eq!(errors(&r), [codes::REF_HOLE_NOT_A_PARAM], "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 2);
    assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| d.origin.is_empty()));
}

#[test]
fn deep_declarations_and_tolerant_sets_are_checked_without_applications() {
    for body in
        ["{%I = @DEFINE(roles=[]) { ?missing }}", "%I = @DEFINE(roles=[]) { %J = @DEFINE(roles=[]) { ?missing } }"]
    {
        let r = read("0.4", body, false);
        assert_eq!(errors(&r), [codes::REF_HOLE_UNKNOWN], "{:?}", r.diagnostics());
    }
}

#[test]
fn declarations_with_argument_dependent_roles_are_not_given_an_invented_signature() {
    let source="* dynamic declaration\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n* @RNL { $D = @DEFINE(roles=[], parameters=[[\"inner_roles\",[\"p\"]]]) { %I = @DEFINE(roles=?inner_roles) { ?p @CIRCLE(radius=1) }; (0,0) @AS %I => @PLACE(0,0) }; $D @ATTACH => @PLACE(0,0); $D @ATTACH => @PLACE(20,0) }\n";
    let r = rnl_cli::interpret_file(
        Path::new("nested.sp"),
        None,
        &MemoryProvider::new().with("nested.sp", source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap();
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 2);
}

#[test]
fn fixed_keywords_in_a_dynamic_declaration_do_not_wait_for_application() {
    for used in [false, true] {
        let calls = if used { "$D @ATTACH; $D @ATTACH" } else { "" };
        let source=format!("* dynamic signature\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n* @RNL {{ $D = @DEFINE(roles=[], parameters=[[\"items\",[\"p\"]]]) {{ %I = @DEFINE(roles=?items) {{ (0,0) @NOT-A-KEYWORD }} }}; {calls} }}\n");
        let r = rnl_cli::interpret_file(
            Path::new("nested.sp"),
            None,
            &MemoryProvider::new().with("nested.sp", &source),
            &Options { std: false, ..Default::default() },
        )
        .unwrap();
        assert_eq!(errors(&r), [codes::KEYWORD_UNKNOWN], "used={used}: {:?}", r.diagnostics());
        assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| d.origin.is_empty()));
    }
}

#[test]
fn dynamic_role_errors_do_not_poison_later_valid_nested_definitions() {
    let source="* varying nested declarations\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n* @RNL { $D = @DEFINE(roles=[], parameters=[[\"items\",[\"p\"]]]) { %I = @DEFINE(roles=?items) { ?p @CIRCLE(radius=1) }; (0,0) @AS %I => @PLACE(0,0) }; $D @ATTACH(items=[\"q\"]) => @PLACE(0,0); $D @ATTACH(items=[\"p\"]) => @PLACE(10,0); $D @ATTACH(items=[\"q\"]) => @PLACE(20,0) }\n";
    let r = rnl_cli::interpret_file(
        Path::new("nested.sp"),
        None,
        &MemoryProvider::new().with("nested.sp", source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap();
    assert_eq!(errors(&r), [codes::REF_HOLE_UNKNOWN, codes::REF_HOLE_UNKNOWN], "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 1, "{:?}", r.diagnostics());
    assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| !d.origin.is_empty()));
}

#[test]
fn dynamic_parameter_lists_keep_each_creation_independent() {
    let source="* varying nested parameters\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n* @RNL { $D = @DEFINE(roles=[], parameters=[[\"items\",[[\"size\",1]]]]) { %I = @DEFINE(roles=[], parameters=?items) { (0,0) @CIRCLE(radius=?size) }; %I @ATTACH => @PLACE(0,0) }; $D @ATTACH(items=[[\"other\",1]]) => @PLACE(0,0); $D @ATTACH => @PLACE(10,0); $D @ATTACH(items=[[\"other\",1]]) => @PLACE(20,0) }\n";
    let r = rnl_cli::interpret_file(
        Path::new("nested.sp"),
        None,
        &MemoryProvider::new().with("nested.sp", source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap();
    assert_eq!(errors(&r), [codes::REF_HOLE_NOT_A_PARAM, codes::REF_HOLE_NOT_A_PARAM], "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 1);
    assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| !d.origin.is_empty()));
}

#[test]
fn fixed_errors_in_dynamic_bodies_and_their_children_keep_source_diagnostics() {
    let source="* inherited fixed errors\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n* @RNL { $D = @DEFINE(roles=[], parameters=[[\"items\",[\"p\"]]]) { %I = @DEFINE(roles=?items) { (0,0) @NOT-A-KEYWORD; %J = @DEFINE(roles=[]) { (0,0) @CIRCLE(radius=?missing); (8,0) @CIRCLE(radius=1) }; %J @ATTACH => @PLACE(0,0) }; (0,0) @AS %I => @PLACE(0,0) }; $D @ATTACH => @PLACE(0,0); $D @ATTACH => @PLACE(20,0) }\n";
    let r = rnl_cli::interpret_file(
        Path::new("nested.sp"),
        None,
        &MemoryProvider::new().with("nested.sp", source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap();
    assert_eq!(errors(&r), [codes::KEYWORD_UNKNOWN, codes::REF_HOLE_NOT_A_PARAM], "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 2, "{:?}", r.diagnostics());
    assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| d.origin.is_empty()));
}
