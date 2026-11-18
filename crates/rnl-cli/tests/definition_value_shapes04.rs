//! The shape of a literal array is known even when some elements are parameters.
use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn read(version: &str, body: &str, calls: &str) -> Interpretation {
    let text = format!("* shapes\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ $D = @DEFINE(roles=[\"r\"],parameters=[[\"v\",1]]) {{ {body} }}; {calls} }}\n");
    interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &text),
        &Options { std: false, ..Default::default() },
    )
    .unwrap()
}
fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}

#[test]
fn an_array_cannot_become_a_scalar_by_substituting_its_elements() {
    for body in [
        "?r @PLACE(x=[?v])",
        "(0,0) @CIRCLE(radius=[?v])",
        "?r @TEXT(text=[?v])",
        "?r @TEXT(text=\"ok\",fix_rotate=[?v])",
        "(0,0) @CIRCLE(radius=1,linecolor=[?v])",
    ] {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let r = read("0.4", body, calls);
            assert_eq!(errors(&r), [codes::PARAM_WRONG_TYPE], "{body}: {:?}", r.diagnostics());
            assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| d.origin.is_empty()));
        }
    }
}

#[test]
fn a_list_of_enum_values_does_not_accept_a_nested_array() {
    for body in ["?r @ORIENT([[?v]])", "?r @ORIENT([\"R0\",[?v]])", "?r @ORDER([?v])"] {
        let r = read("0.4", body, "");
        assert_eq!(errors(&r), [codes::PARAM_BAD_ENUM], "{body}: {:?}", r.diagnostics());
    }
}

#[test]
fn unknown_scalar_values_and_valid_outer_lists_remain_deferred() {
    for body in ["?r @PLACE(x=?v)", "?r @ORIENT([?v,\"R0\"])", "?r @TEXT(text=?v)"] {
        let r = read("0.4", body, "");
        assert!(errors(&r).is_empty(), "{body}: {:?}", r.diagnostics());
    }
    let r = read("0.4", "?r @ORIENT([?v,\"R0\"])", "#R1 @AS(v=\"R0\") $D; #R2 @AS(v=\"R90\") $D");
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
}
