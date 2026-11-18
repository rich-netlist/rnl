//! Definition-time names use their lexical scope and keep dynamic objects deferred (6.9).
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn read(version: &str, setup: &str, body: &str, calls: &str) -> Interpretation {
    let source=format!("* definition names\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n* @RNL {{ {setup} $D = @DEFINE(roles=[]) {{ {body}; (5,0) @CIRCLE(radius=1) }}; {calls} }}\n");
    rnl_cli::interpret_file(
        Path::new("names.sp"),
        None,
        &MemoryProvider::new().with("names.sp", &source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap()
}
fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}
const TWO: &str = "$D @ATTACH => @PLACE(0,0); $D @ATTACH => @PLACE(10,0)";

#[test]
fn missing_names_are_reported_once_even_without_an_application() {
    for body in ["$Missing @ATTACH", "%missing @CIRCLE(radius=1)", "%missing::position @CIRCLE(radius=1)"] {
        for calls in ["", TWO] {
            let r = read("0.4", "", body, calls);
            assert_eq!(errors(&r), [codes::REF_NOT_FOUND], "{body}: {:?}", r.diagnostics());
            assert!(r.diagnostics().into_iter().find(|d| d.code == codes::REF_NOT_FOUND).unwrap().origin.is_empty());
            assert_eq!(r.model.levels[0].shapes.len(), if calls.is_empty() { 0 } else { 2 });
        }
    }
}

#[test]
fn forward_local_and_global_bindings_stay_available() {
    for (setup, body) in [
        ("", "%p @CIRCLE(radius=1); %p=(2,0)"),
        ("$Other = @DEFINE(roles=[]) {};", "$Other @ATTACH"),
        ("", "%inner = @DEFINE(roles=[\"t\"]) {}; (0,0) @AS %inner"),
    ] {
        for calls in ["", TWO] {
            let r = read("0.4", setup, body, calls);
            assert!(errors(&r).is_empty(), "{body}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn nested_definition_roles_do_not_belong_to_the_outer_definition() {
    let body = "%inner = @DEFINE(roles=[\"t\"]) { ?t @PLACE(1,2) }; (0,0) @AS %inner";
    for calls in ["", TWO] {
        let r = read("0.4", "", body, calls);
        assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    }
}

#[test]
fn nested_definition_locals_do_not_escape() {
    for calls in ["", TWO] {
        let r = read("0.4", "", "%inner = @DEFINE(roles=[]) { %private=(1,0) }; %private @CIRCLE(radius=1)", calls);
        assert_eq!(errors(&r), [codes::REF_NOT_FOUND], "{:?}", r.diagnostics());
    }
}

#[test]
fn nested_plain_sets_resolve_their_own_forward_names() {
    let body = "%set = { %p @CIRCLE(radius=1); %p=(2,0) }";
    for calls in ["", TWO] {
        let r = read("0.4", "", body, calls);
        assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].shapes.len(), if calls.is_empty() { 0 } else { 4 });
    }
    let outside = read("0.4", "", &format!("{body}; %p @CIRCLE(radius=1)"), "");
    assert_eq!(errors(&outside), [codes::REF_NOT_FOUND], "{:?}", outside.diagnostics());
}

#[test]
fn later_global_declarations_are_found_without_evaluating_them() {
    let r = read("0.4", "", "$Later @ATTACH", "$Later = @DEFINE(roles=[]) {}; ");
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    let r = read("0.4", "", "$Later @ATTACH", &format!("{TWO}; $Later = @DEFINE(roles=[]) {{}}"));
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
}

#[test]
fn unused_package_definitions_keep_their_own_name_environment_and_location() {
    let files = MemoryProvider::new()
        .with("p.rnl", "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"p\",version=\"1.0\") }\n@RNL { $D = @DEFINE(roles=[]) { $Caller @ATTACH } }\n")
        .with("main.sp", "* package names\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"p\",path=\"p.rnl\") }\n* @RNL { $Caller = @DEFINE(roles=[]) {}; $D @ATTACH; $D @ATTACH }\n");
    for path in ["p.rnl", "main.sp"] {
        let r = rnl_cli::interpret_file(Path::new(path), None, &files, &Options { std: false, ..Default::default() })
            .unwrap();
        assert_eq!(errors(&r), [codes::REF_NOT_FOUND], "{path}: {:?}", r.diagnostics());
        let diagnostic = r.diagnostics().into_iter().find(|d| d.code == codes::REF_NOT_FOUND).unwrap();
        assert_eq!(diagnostic.in_package, "p");
        assert!(diagnostic.loc.as_ref().unwrap().file.ends_with("p.rnl"));
        assert!(diagnostic.origin.is_empty());
    }
}

#[test]
fn included_definitions_check_their_names() {
    let files = MemoryProvider::new()
        .with("main.sp", "* names\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include \"defs.sp\"\n")
        .with("defs.sp", "* definitions\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n* @RNL { $D = @DEFINE(roles=[]) { $Missing @ATTACH } }\n");
    let r = rnl_cli::interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Default::default() })
        .unwrap();
    assert_eq!(errors(&r), [codes::REF_NOT_FOUND], "{:?}", r.diagnostics());
}

#[test]
fn nested_definition_keyword_aliases_also_own_their_roles() {
    let source="* aliases\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @ALIAS(\"@INNER\",\"@DEFINE\") }\n* @RNL { $D = @DEFINE(roles=[]) { %inner = @INNER(roles=[\"t\"]) { ?t @PLACE(1,2) }; (0,0) @AS %inner }; $D @ATTACH }\n";
    let r = rnl_cli::interpret_file(
        Path::new("names.sp"),
        None,
        &MemoryProvider::new().with("names.sp", source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap();
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
}
