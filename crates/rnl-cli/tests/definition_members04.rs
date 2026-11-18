//! A statically invalid body member is omitted as a whole; independent members still work.
use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn source(version: &str, body: &str, tail: &str) -> String {
    format!("* body members\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ $D = @DEFINE(roles=[\"r\"],parameters=[[\"distance\",1]]) {{ {body} }}; %u = #R1 @AS $D; %u @PLACE(0,0); {tail} }}\n")
}
fn read(version: &str, body: &str, tail: &str) -> Interpretation {
    interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &source(version, body, tail)),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap()
}
fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}
fn points(r: &Interpretation) -> Vec<Vec<(f64, f64)>> {
    r.model.levels[0].shapes.iter().map(|s| s.points.clone()).collect()
}

#[test]
fn a_local_name_clash_omits_the_graphic_but_keeps_other_members() {
    for name in ["r", "position", "net"] {
        let body = format!("%{name}=(0,0) @CIRCLE(radius=1); (8,0) @CIRCLE(radius=2)");
        let r = read("0.4", &body, "");
        assert_eq!(errors(&r), [codes::DEFINE_LOCAL_NAME_CLASH], "{name}: {:?}", r.diagnostics());
        assert_eq!(points(&r), [vec![(8., 0.)]], "{name}");
    }
}

#[test]
fn an_invalid_later_binding_keeps_the_first_local_object() {
    let r = read("0.4", "%p=(1,0); %p=(4,0) @CIRCLE(radius=1); %p @CIRCLE(radius=2)", "%u::p @PLACE(1,0)");
    assert_eq!(errors(&r), [codes::DEFINE_LOCAL_NAME_TWICE], "{:?}", r.diagnostics());
    assert_eq!(points(&r), [vec![(1., 0.)]]);
}

#[test]
fn a_late_static_error_omits_earlier_segments_too() {
    for (bad, code) in [
        ("(0,0) @CIRCLE(radius=1) => @NOT-A-KEYWORD", codes::KEYWORD_UNKNOWN),
        ("(0,0) @CIRCLE(radius=1) => @PIN(terminal=1)", codes::DEFINE_BODY_PIN),
        ("(0,0) @CIRCLE(radius=1) => @PLACE(x=?r)", codes::REF_HOLE_NOT_A_PARAM),
        ("(0,0) @CIRCLE(radius=1) => @VERSION", codes::KEYWORD_HEADER_ONLY),
    ] {
        let r = read("0.4", &format!("{bad}; (8,0) @CIRCLE(radius=2)"), "");
        assert_eq!(errors(&r), [code], "{bad}: {:?}", r.diagnostics());
        assert_eq!(points(&r), [vec![(8., 0.)]], "{bad}");
    }
}

#[test]
fn a_failed_nested_member_does_not_discard_its_siblings() {
    let r = read("0.4", "{{ (0,0) @CIRCLE(radius=1) => @NOT-A-KEYWORD; (8,0) @CIRCLE(radius=2) }}", "");
    assert_eq!(errors(&r), [codes::KEYWORD_UNKNOWN], "{:?}", r.diagnostics());
    assert_eq!(points(&r), [vec![(8., 0.)]]);
}

#[test]
fn references_to_an_omitted_local_keep_the_original_failure() {
    let r = read(
        "0.4",
        "%bad=(0,0) @CIRCLE(radius=1) => @NOT-A-KEYWORD; %bad @CIRCLE(radius=3); (8,0) @CIRCLE(radius=2)",
        "%u::bad @PLACE(2,0)",
    );
    assert_eq!(
        errors(&r),
        [codes::KEYWORD_UNKNOWN, codes::REF_FAILED, codes::KEYWORD_OBJECT_FAILED],
        "{:?}",
        r.diagnostics()
    );
    let failures: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert_eq!(failures[0].cause, None);
    assert!(failures[0].origin.is_empty());
    assert_eq!(failures[1].cause, Some(rnl_runtime::diag::DiagId(0)));
    assert_eq!(failures[2].cause, Some(rnl_runtime::diag::DiagId(0)));
    assert_eq!(points(&r), [vec![(8., 0.)]]);
}

#[test]
fn nested_syntax_errors_are_reported_even_in_unused_definitions() {
    let source = source("0.4", "{{@VERSION}; (8,0) @CIRCLE(radius=2)}", "");
    for source in [source.clone(), source.replace("%u = #R1 @AS $D; %u @PLACE(0,0);", "")] {
        let r = interpret_file(
            Path::new("main.sp"),
            None,
            &MemoryProvider::new().with("main.sp", &source),
            &Options { std: false, ..Default::default() },
        )
        .unwrap();
        assert_eq!(errors(&r).len(), 1, "{:?}", r.diagnostics());
        assert!(r.diagnostics().iter().find(|d| d.severity == Severity::Error).unwrap().origin.is_empty());
    }
}

#[test]
fn nested_symbol_bodies_keep_their_own_context() {
    let r = read(
        "0.4",
        "%s = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); (0,0) @CIRCLE(radius=1) }; ?r @SET-SYMBOL %s; ?r @PLACE(0,0)",
        "",
    );
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].items.len(), 1);
}
