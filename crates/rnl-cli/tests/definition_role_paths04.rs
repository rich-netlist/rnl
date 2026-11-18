//! Declared role kinds constrain standard paths before any application.
use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn read(version: &str, kind: &str, body: &str, calls: &str) -> Interpretation {
    let roles = if kind.is_empty() { "[\"r\"]".to_string() } else { format!("[[\"r\",{kind}]]") };
    let body = body.replace('\n', "\n* ");
    let calls = calls.replace('\n', "\n* ");
    let text=format!("* roles\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ $D = @DEFINE(roles={roles}) {{ {body} }}; {calls} }}\n");
    interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &text),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap()
}
fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}

#[test]
fn an_instance_role_has_no_net_even_in_an_unused_definition() {
    for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
        let r = read("0.4", "\"instance\"", "?r::net", calls);
        assert_eq!(errors(&r), [codes::REF_NO_SUBOBJECT], "{:?}", r.diagnostics());
        let diagnostic = r.diagnostics().into_iter().find(|d| d.severity == Severity::Error).unwrap();
        assert!(diagnostic.origin.is_empty());
        assert!(diagnostic.message.contains("?r") && diagnostic.message.contains("instance"));
        assert_eq!(diagnostic.loc.as_ref().unwrap().line, 5);
    }
}

#[test]
fn impossible_standard_paths_are_checked_through_known_intermediate_kinds() {
    for (kind, body, code) in [
        ("\"net\"", "?r::position", codes::REF_NO_SUBOBJECT),
        ("\"wire\"", "?r::position", codes::REF_NO_SUBOBJECT),
        ("\"point\"", "?r::net", codes::REF_NO_SUBOBJECT),
        ("\"point\"", "?r:p", codes::REF_NO_TERMINALS),
        ("\"usage\"", "?r:p", codes::REF_NO_TERMINALS),
        ("\"pin\"", "?r::net::position", codes::REF_NO_SUBOBJECT),
        ("\"instance\"", "?r::position::net", codes::REF_NO_SUBOBJECT),
    ] {
        let r = read("0.4", kind, body, "");
        assert_eq!(errors(&r), [code], "{kind}/{body}: {:?}", r.diagnostics());
    }
}

#[test]
fn any_possible_declared_kind_leaves_application_dependent_paths_open() {
    for (kind, body) in [
        ("[\"instance\",\"terminal\"]", "?r::net"),
        ("\"pin\"", "?r::net"),
        ("\"terminal\"", "?r::position"),
        ("\"annotation\"", "?r::position"),
        ("\"instance\"", "?r::p::net"),
        ("", "?r::net"),
    ] {
        let r = read("0.4", kind, body, "");
        assert!(errors(&r).is_empty(), "{kind}/{body}: {:?}", r.diagnostics());
    }
}

#[test]
fn undeclared_role_kinds_keep_errors_at_each_application() {
    let r = read("0.4", "", "?r::net", "#R1 @AS $D; #R2 @AS $D");
    assert_eq!(errors(&r), [codes::REF_NO_SUBOBJECT, codes::REF_NO_SUBOBJECT]);
    assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| !d.origin.is_empty()));
}

#[test]
fn role_paths_share_the_filler_identity_but_pin_names_keep_written_identity() {
    for forward in [false, true] {
        let application = "%u = #R1 @AS $D;";
        let selected = r#"$P = @DEFINE(roles=["x"]) {}; {%u::r #R1} @UNION => @AS $P;
            $Q = @DEFINE(roles=["x"]) {};
            {#R1::p #R1::q} @UNION => @AS $Q;
            $S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal=1) => %q };
            #R1 @SET-SYMBOL $S;"#;
        let calls = if forward { format!("{selected} {application}") } else { format!("{application} {selected}") };
        let r = read("0.4", "\"instance\"", "", &calls);
        assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
        let usages = &r.problems[0].usages;
        assert_eq!(usages.iter().filter(|u| u.definition == "$P").count(), 1);
        assert_eq!(usages.iter().filter(|u| u.definition == "$Q").count(), 2);
    }
}

#[test]
fn role_paths_place_the_filler_with_the_same_fact_key() {
    use rnl_runtime::problem::{FactKind, PointRef};
    let r = read("0.4", "\"instance\"", "", "%u = #R1 @AS $D; %u::r @PLACE(1,2); #R1 @PLACE(3,4)");
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    let problem = &r.problems[0];
    let host = problem.objects.iter().position(|object| object.label == "#R1").unwrap();
    let places: Vec<_> = problem
        .facts
        .iter()
        .filter_map(|fact| match &fact.kind {
            FactKind::Place { at, .. } => Some(at),
            _ => None,
        })
        .collect();
    assert_eq!(places.len(), 4);
    assert!(places.iter().all(|at| **at == PointRef::Origin(host)));
}

#[test]
fn an_explicit_role_filter_selects_the_converted_pin_after_symbols() {
    use rnl_runtime::problem::Filler;
    let r = read(
        "0.4",
        "\"pin\"",
        "",
        r#"
        %u = #R1:1 @AS $D;
        $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) };
        #R1 @SET-SYMBOL $S;
        $P = @DEFINE(roles=["x"]) {};
        %u @SELECT-PIN(role="r") => @AS $P
    "#,
    );
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    let probes: Vec<_> = r.problems[0].usages.iter().filter(|u| u.definition == "$P").collect();
    assert_eq!(probes.len(), 1);
    assert!(matches!(probes[0].roles[0].members.as_slice(), [Filler::Pin { .. }]));
}
