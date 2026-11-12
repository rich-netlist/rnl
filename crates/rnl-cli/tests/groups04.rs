//! GROUP follows semantic roles after symbols and final set edits (11.5).
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::model::Attach;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

const SYMBOLS: &str = r#"$S = @DEFINE-SYMBOL { (-1,0) @PIN(terminal=1); (1,0) @PIN(terminal=2); (-0.5,-0.5) (0.5,0.5) @RECTANGLE }; $U = @DEFINE-SYMBOL { (-0.5,-0.5) (0.5,0.5) @RECTANGLE }; # @SELECT-INSTANCE => @SET-SYMBOL $S"#;
fn source(version: &str, body: &str) -> String {
    format!("* groups\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a 0 1k\nR2 b 0 2k\nR3 c 0 3k\n* @RNL {{ {SYMBOLS}; {} }}\n",body.replace('\n'," "))
}
fn read_files(files: &MemoryProvider) -> Interpretation {
    interpret_file(Path::new("main.sp"), None, files, &Options { std: false, ..Options::solver() }).unwrap()
}
fn read(version: &str, body: &str) -> Interpretation {
    read_files(&MemoryProvider::new().with("main.sp", &source(version, body)))
}
fn clean(r: &Interpretation) {
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
}

#[test]
fn solver_groups_export_provenance_and_set_identity_independently_of_labels() {
    let r = read(
        "0.4",
        r#"$D = @DEFINE(roles=["a","b"]) { %inner = {?a ?b} @GROUP }; %u = #R1 #R2 @AS $D; {#R2 #R3} @GROUP; %u::inner @GROUP"#,
    );
    clean(&r);
    let p = &r.problems[0];
    assert_eq!(p.groups.len(), 3);
    let expanded: Vec<_> = p.groups.iter().filter(|g| !g.direct).collect();
    assert_eq!(expanded.len(), 1);
    let repeated: Vec<_> = p.groups.iter().filter(|g| g.identity == expanded[0].identity).collect();
    assert_eq!(repeated.len(), 2);
    assert!(repeated.iter().any(|g| g.direct));
    assert_eq!(p.groups.iter().filter(|g| g.direct).count(), 2);
    let shown = rnl_cli::problem::level(p, "");
    assert_eq!(shown["groups"][0]["direct"], p.groups[0].direct);
    assert_eq!(shown["groups"][0]["identity"], p.groups[0].identity);
    let mut changed = p.clone();
    changed.groups[0].direct = !changed.groups[0].direct;
    assert_ne!(rnl_runtime::problem::input_digest(p), rnl_runtime::problem::input_digest(&changed));
    let mut old = p.clone();
    old.version = "0.3".into();
    changed.version = "0.3".into();
    changed.groups[0].identity += 100;
    assert_eq!(rnl_runtime::problem::input_digest(&old), rnl_runtime::problem::input_digest(&changed));
}
fn problem(r: &Interpretation, name: &str) -> Vec<String> {
    let p = r.problems.iter().find(|p| p.groups.iter().any(|g| g.label == name)).unwrap();
    p.groups.iter().find(|g| g.label == name).unwrap().members.iter().map(|i| p.objects[*i].label.clone()).collect()
}
fn model(r: &Interpretation, name: &str) -> Vec<String> {
    r.model
        .levels
        .iter()
        .flat_map(|l| &l.groups)
        .find(|g| g.id == name)
        .unwrap()
        .members
        .iter()
        .map(|m| match m {
            Attach::Object(id) => id.clone(),
            other => panic!("{other:?}"),
        })
        .collect()
}
fn members(r: &Interpretation, name: &str, expected: &[&str]) {
    assert_eq!(problem(r, name), expected);
    assert_eq!(model(r, name), expected);
}

#[test]
fn semantic_usages_expand_but_symbolized_usages_remain_members() {
    let body = "$D = @DEFINE(roles=[\"a\",\"b\"]) {}; %u = #R1 #R2 @AS $D; %g = {%u #R3} @GROUP";
    let r = read("0.4", body);
    clean(&r);
    members(&r, "%g", &["#R1", "#R2", "#R3"]);
    let r = read("0.4", &format!("{body}; %u @SET-SYMBOL $U"));
    clean(&r);
    members(&r, "%g", &["%u", "#R3"]);
    for version in ["0.2", "0.3"] {
        let r = read(version, body);
        clean(&r);
        assert_eq!(problem(&r, "%g"), ["#R3"]);
        assert_eq!(model(&r, "%g"), ["%u", "#R3"]);
    }
}

#[test]
fn nested_roles_keep_order_and_repeated_occurrences() {
    let r = read(
        "0.4",
        r#"$D = @DEFINE(roles=["a","b"]) {}; $E = @DEFINE(roles=["a"]) {}; %u = #R1 #R2 @AS $D; %v = %u @AS $E; %g = {%v {#R3 %u} %v} @GROUP"#,
    );
    clean(&r);
    members(&r, "%g", &["#R1", "#R2", "#R3", "#R1", "#R2", "#R1", "#R2"]);
}

#[test]
fn set_roles_follow_final_modifications_and_generation() {
    let r = read(
        "0.4",
        r#"$D = @DEFINE(roles=["{items}"]) {}; %s = {#R1 #R2}; %u = %s @AS $D; %g = {%u} @GROUP; #R1 @REMOVE-FROM %s; #R3 @ADD-TO %s; $E = @DEFINE(roles=["i"]) {}; %all = # @SELECT-INSTANCE => @AS $E; %generated = {%all} @GROUP"#,
    );
    clean(&r);
    members(&r, "%g", &["#R2", "#R3"]);
    members(&r, "%generated", &["#R1", "#R2", "#R3"]);
}

#[test]
fn cycles_through_roles_terminate_without_losing_other_members() {
    let r = read(
        "0.4",
        r#"$D = @DEFINE(roles=["{items}"]) {}; %s = {#R1}; %u = %s @AS $D; %u @ADD-TO %s; %g = {%u #R2} @GROUP"#,
    );
    clean(&r);
    members(&r, "%g", &["#R1", "#R2"]);
}

#[test]
fn groups_in_disabled_bodies_can_be_reenabled_directly() {
    let body = r#"$E = @DEFINE(roles=["a"]) {}; $D = @DEFINE(roles=["a","b"]) { %u = ?a @AS $E; %g = {%u ?b} @GROUP }; %d = #R1 #R2 @AS $D => @PRIORITY("disabled")"#;
    let r = read("0.4", body);
    clean(&r);
    assert!(r.problems[0].groups.is_empty());
    assert!(r.model.levels[0].groups[0].disabled);
    let r = read("0.4", &format!("{body}; %d::g @PRIORITY(\"normal\")"));
    clean(&r);
    let p = &r.problems[0];
    assert_eq!(p.groups.len(), 1);
    assert_eq!(p.groups[0].members.iter().map(|i| p.objects[*i].label.as_str()).collect::<Vec<_>>(), ["#R1", "#R2"]);
    assert!(!r.model.levels[0].groups[0].disabled);
}

#[test]
fn singleton_groups_keep_the_set_handle() {
    let r = read("0.4", "%s = {#R1}; %g = %s @GROUP; #R2 @ADD-TO %g");
    clean(&r);
    assert_eq!(r.problems[0].groups[0].members.len(), 2);
}

#[test]
fn mixed_version_scoped_groups_follow_the_writing_file() {
    for (version, other) in [("0.3", "0.4"), ("0.4", "0.3")] {
        let main=format!("* mixed\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n.include extra.sp\n* @RNL(scope=\"child\") {{ %own = {{%u #R3}} @GROUP }}\n");
        let extra =
            source(other, "$D = @DEFINE(roles=[\"a\",\"b\"]) {}; %u = #R1 #R2 @AS $D; %other = {%u #R3} @GROUP")
                .replace("R1 a", ".subckt child a b c\nR1 a")
                + ".ends\n";
        let r = read_files(&MemoryProvider::new().with("main.sp", &main).with("extra.sp", &extra));
        clean(&r);
        members(&r, "%own", if version == "0.4" { &["#R1", "#R2", "#R3"] } else { &["%u", "#R3"] });
        if other == "0.4" {
            members(&r, "%other", &["#R1", "#R2", "#R3"]);
        } else {
            assert_eq!(problem(&r, "%other"), ["#R3"]);
            assert_eq!(model(&r, "%other"), ["%u", "#R3"]);
        }
    }
}

#[test]
fn body_contents_do_not_replace_roles_and_empty_roles_contribute_nothing() {
    let r = read(
        "0.4",
        r#"$D = @DEFINE(roles=["a"]) { (8,9) @TEXT("body") }; $E = @DEFINE(roles=[]) { (3,4) @TEXT("empty") }; %u = #R1 @AS $D; %e = $E @ATTACH; %g = {%u %e #R2} @GROUP"#,
    );
    clean(&r);
    members(&r, "%g", &["#R1", "#R2"]);
}

#[test]
fn a_failed_expansion_cannot_group_a_shared_set() {
    let body = r#"$P = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal=1); (1,0) @PIN(terminal=2) }; #R1 @SET-SYMBOL $P; %g = {#R2 #R3}; $D = @DEFINE(roles=[["r","instance"],"{members}"]) { ?members @GROUP }; %bad = #R1::p %g @AS $D"#;
    let r = read("0.4", body);
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
    assert!(r.problems[0].groups.is_empty(), "{:?}", r.problems[0].groups);
    assert!(r.model.levels[0].groups.is_empty());
    let r = read("0.4", &format!("{body}; %g @GROUP"));
    members(&r, "%g", &["#R2", "#R3"]);
    assert_eq!(r.problems[0].groups.len(), 1);
    assert_eq!(r.model.levels[0].groups.len(), 1);
}

#[test]
fn a_disabled_first_group_is_not_replaced_by_a_later_declaration() {
    let r = read(
        "0.4",
        r#"$D = @DEFINE(roles=["{members}"]) { ?members @GROUP }; %g = {#R1 #R2}; %d = %g @AS $D => @PRIORITY("disabled"); %g @GROUP"#,
    );
    clean(&r);
    let p = &r.problems[0];
    assert_eq!(p.groups.len(), 1);
    assert!(p.groups[0].direct);
    assert!(!p.groups[0].first);
    let mut changed = p.clone();
    changed.groups[0].first = true;
    assert_ne!(rnl_runtime::problem::input_digest(p), rnl_runtime::problem::input_digest(&changed));
}
