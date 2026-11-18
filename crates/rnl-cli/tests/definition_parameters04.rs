//! Parameter signatures known at a definition are checked without substituting default values.
use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

const CALLS: &str = "#R1 @AS $D; #R2 @AS $D";
fn record(body: &str, calls: &str) -> String {
    format!("$D = @DEFINE(roles=[\"r\"],parameters=[[\"gap\",1]]) {{ {body} }}; {calls}")
}
fn source(version: &str, body: &str, calls: &str) -> String {
    format!(
        "* parameters\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ {} }}\n",
        record(body, calls)
    )
}
fn files(provider: MemoryProvider) -> Interpretation {
    interpret_file(Path::new("main.sp"), None, &provider, &Options { std: false, problems: true, ..Default::default() })
        .unwrap()
}
fn read(version: &str, body: &str, calls: &str) -> Interpretation {
    files(MemoryProvider::new().with("main.sp", &source(version, body, calls)))
}
fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}

#[test]
fn known_parameter_errors_are_reported_once_even_without_an_application() {
    for (body, code) in [
        ("?r @PLACE(x=\"far\")", codes::PARAM_WRONG_TYPE),
        ("?r @PLACE(no_such=1)", codes::PARAM_UNKNOWN),
        ("?r @PLACE(1,x=2)", codes::PARAM_WRITTEN_TWICE),
        ("?r @PLACE(1,2,3)", codes::PARAM_TOO_MANY_POSITIONAL),
        ("?r @ORDER", codes::PARAM_MISSING_REQUIRED),
        ("?r @ORIENT(\"diagonal\")", codes::PARAM_BAD_ENUM),
        ("(0,0) @CIRCLE(radius=1,linecolor=\"bad\")", codes::PARAM_BAD_COLOUR),
        ("?r @ORIENT([])", codes::PARAM_EMPTY_LIST),
    ] {
        for calls in ["", CALLS] {
            let r = read("0.4", body, calls);
            assert_eq!(errors(&r), [code], "{body}/{calls}: {:?}", r.diagnostics());
            let diagnostic = r.diagnostics().into_iter().find(|d| d.code == code).unwrap();
            assert!(diagnostic.origin.is_empty());
            assert_eq!(diagnostic.loc.as_ref().unwrap().line, 5);
        }
    }
}

#[test]
fn unknown_values_do_not_hide_known_structure_or_other_bad_literals() {
    for (body, code) in [
        ("?r @PLACE(x=?gap,y=\"far\")", codes::PARAM_WRONG_TYPE),
        ("?r @PLACE(no_such=?gap)", codes::PARAM_UNKNOWN),
        ("?r @PLACE(?gap,x=1)", codes::PARAM_WRITTEN_TWICE),
        ("?r @PLACE(?gap,1,2)", codes::PARAM_TOO_MANY_POSITIONAL),
        ("?r @ORIENT([?gap,\"diagonal\"])", codes::PARAM_BAD_ENUM),
    ] {
        for calls in ["", CALLS] {
            let r = read("0.4", body, calls);
            assert_eq!(errors(&r), [code], "{body}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn actual_parameter_types_are_checked_per_application() {
    assert!(errors(&read("0.4", "?r @PLACE(x=?gap)", "")).is_empty());
    let r = read("0.4", "?r @PLACE(x=?gap)", "#R1 @AS(gap=\"far\") $D; #R2 @AS(gap=\"far\") $D");
    assert_eq!(errors(&r), [codes::PARAM_WRONG_TYPE, codes::PARAM_WRONG_TYPE], "{:?}", r.diagnostics());
    assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| !d.origin.is_empty()));
}

#[test]
fn a_default_value_does_not_stand_for_every_application() {
    let source = source("0.4", "?r @PLACE(x=?gap)", "#R1 @AS(gap=2) $D; #R2 @AS(gap=3) $D")
        .replace("parameters=[[\"gap\",1]]", "parameters=[[\"gap\",\"far\"]]");
    let r = files(MemoryProvider::new().with("main.sp", &source));
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    assert!(errors(&read("0.4", "?r @ORDER(?gap)", "")).is_empty());
}

#[test]
fn nested_parameter_errors_are_checked_in_their_own_member() {
    for calls in ["", CALLS] {
        let r = read("0.4", "{{ (0,0) @CIRCLE(radius=\"far\"); (8,0) @CIRCLE(radius=2) }}", calls);
        assert_eq!(errors(&r), [codes::PARAM_WRONG_TYPE], "{:?}", r.diagnostics());
    }
}

#[test]
fn styles_and_application_parameters_keep_their_open_context() {
    let r = read(
        "0.4",
        "%s = @DEFINE-STYLE(linecolor=\"#123456\") {}; ?r @SET-STYLE %s; (0,0) @CIRCLE(radius=1,linecolor=\"#abcdef\")",
        CALLS,
    );
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    let r = read("0.4", "?r @PLACE(x=?gap)", "#R1 @AS(gap=3) $D");
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
}

#[test]
fn included_definitions_check_their_parameters_once() {
    let lib=format!("* library\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n.subckt cell a b\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ {} }}\n.ends cell\n",record("?r @PLACE(x=\"far\")",CALLS));
    let r = files(
        MemoryProvider::new()
            .with("main.sp", "* main\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include lib.inc\nX1 a b cell\n")
            .with("lib.inc", &lib),
    );
    assert_eq!(errors(&r).len(), 1, "{:?}", r.diagnostics());
    assert!(errors(&r).iter().all(|code| *code == codes::PARAM_WRONG_TYPE));
}
