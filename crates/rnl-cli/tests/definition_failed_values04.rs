//! Failed names that do not depend on arguments are checked once (6.9).
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn read(version: &str, setup: &str, body: &str, used: bool) -> Interpretation {
    let calls = if used { "$D @ATTACH => @PLACE(0,0); $D @ATTACH => @PLACE(10,0)" } else { "" };
    let source=format!("* failed definition names\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n* @RNL {{ {setup} $D = @DEFINE(roles=[]) {{ {body}; (5,0) @CIRCLE(radius=2) }}; {calls} }}\n");
    rnl_cli::interpret_file(
        Path::new("failed.sp"),
        None,
        &MemoryProvider::new().with("failed.sp", &source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap()
}
fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}

#[test]
fn failed_external_names_are_checked_even_when_unused() {
    for (setup, body, first) in [
        ("%p=%missing;", "%p @CIRCLE(radius=1)", codes::REF_NOT_FOUND),
        ("$A = @DEFINE(roles=[\"r\",\"r\"]) {};", "$A @ATTACH", codes::DEFINE_ROLE_NAME_INVALID),
    ] {
        for used in [false, true] {
            let r = read("0.4", setup, body, used);
            assert_eq!(errors(&r), [first, codes::REF_FAILED], "{:?}", r.diagnostics());
            assert!(r.diagnostics().into_iter().find(|d| d.code == codes::REF_FAILED).unwrap().origin.is_empty());
            assert_eq!(r.model.levels[0].shapes.len(), if used { 2 } else { 0 });
        }
    }
}

#[test]
fn failed_external_reference_omits_the_whole_statement() {
    let r = read("0.4", "%p=%missing;", "(0,0) @CIRCLE(radius=1) => @SET-STYLE %p", true);
    assert_eq!(errors(&r), [codes::REF_NOT_FOUND, codes::REF_FAILED], "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 2);
}

#[test]
fn a_failed_local_name_is_checked_once_and_does_not_hide_a_later_valid_binding() {
    for used in [false, true] {
        let failed = read("0.4", "", "%p=%missing; %p @CIRCLE(radius=1)", used);
        assert_eq!(errors(&failed), [codes::REF_NOT_FOUND, codes::REF_FAILED], "{:?}", failed.diagnostics());
        assert!(failed.diagnostics().into_iter().find(|d| d.code == codes::REF_FAILED).unwrap().origin.is_empty());
        let recovered = read("0.4", "", "%p=%missing; %p=(0,0); %p @CIRCLE(radius=1)", used);
        assert_eq!(errors(&recovered), [codes::REF_NOT_FOUND], "{:?}", recovered.diagnostics());
        assert_eq!(recovered.model.levels[0].shapes.len(), if used { 4 } else { 0 });
    }
}

#[test]
fn released_failed_names_keep_their_application_diagnostics() {
    for version in ["0.2", "0.3"] {
        for used in [false, true] {
            let r = read(version, "%p=%missing;", "%p @CIRCLE(radius=1)", used);
            assert_eq!(
                errors(&r).iter().filter(|c| **c == codes::REF_FAILED).count(),
                if used { 2 } else { 0 },
                "{:?}",
                r.diagnostics()
            );
        }
    }
}

fn assert_causes(r: &Interpretation) {
    for diagnostic in r.diagnostics().into_iter().filter(|d| d.code == codes::REF_FAILED) {
        assert!(diagnostic.cause.is_some(), "{diagnostic:?}");
        let mut seen = std::collections::HashSet::new();
        let mut cause = diagnostic.cause;
        while let Some(id) = cause {
            assert!(seen.insert(id), "cyclic diagnostic cause: {diagnostic:?}");
            cause = r.sink.get(id).cause;
        }
    }
}

#[test]
fn forward_failed_names_keep_the_current_upstream_cause() {
    for used in [false, true] {
        let calls = if used { "$D @ATTACH; $D @ATTACH" } else { "" };
        let source = format!("* forward failure\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n* @RNL {{ $D = @DEFINE(roles=[]) {{ %p }}; {calls}; %p=%missing }}\n");
        let r = rnl_cli::interpret_file(
            Path::new("failed.sp"),
            None,
            &MemoryProvider::new().with("failed.sp", &source),
            &Options { std: false, ..Default::default() },
        )
        .unwrap();
        let actual = errors(&r);
        assert_eq!(actual.len(), 2, "{:?}", r.diagnostics());
        assert!(actual.contains(&codes::REF_FAILED) && actual.contains(&codes::REF_NOT_FOUND));
        assert_causes(&r);
        let failed = r.diagnostics().into_iter().find(|d| d.code == codes::REF_FAILED).unwrap();
        assert!(failed.origin.is_empty());
        assert_eq!(r.sink.get(failed.cause.unwrap()).code, codes::REF_NOT_FOUND);
    }
}

#[test]
fn failed_local_alias_chains_and_nested_sets_keep_tolerant_siblings() {
    for body in ["%p=%missing; %q=%p; %q @CIRCLE(radius=1)", "%q @CIRCLE(radius=1); %q=%p; %p=%missing"] {
        let r = read("0.4", "", body, true);
        assert_eq!(errors(&r).len(), 3, "{body}: {:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].shapes.len(), 2);
        assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| d.origin.is_empty()));
        assert_causes(&r);
    }
    let r = read("0.4", "", "{%p=%missing; %p @CIRCLE(radius=1); (2,0) @CIRCLE(radius=1)}", true);
    assert_eq!(errors(&r), [codes::REF_NOT_FOUND, codes::REF_FAILED], "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 4);
    assert_causes(&r);
}

#[test]
fn an_inner_valid_name_shadows_an_outer_failed_local() {
    let r = read("0.4", "", "%p=%missing; {%p=(2,0); %p @CIRCLE(radius=1)}", true);
    assert_eq!(errors(&r), [codes::REF_NOT_FOUND], "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 4);
}

#[test]
fn package_failed_helpers_are_checked_when_the_definition_is_unused() {
    let files = MemoryProvider::new()
        .with("p.rnl", "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"p\",version=\"1.0\") }\n@RNL { $D = @DEFINE(roles=[]) { %p }; %p=%missing }\n")
        .with("main.sp", "* package failure\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"p\",path=\"p.rnl\") }\n");
    for path in ["p.rnl", "main.sp"] {
        let r = rnl_cli::interpret_file(Path::new(path), None, &files, &Options { std: false, ..Default::default() })
            .unwrap();
        assert_eq!(errors(&r).len(), 2, "{path}: {:?}", r.diagnostics());
        let failed = r.diagnostics().into_iter().find(|d| d.code == codes::REF_FAILED).unwrap();
        assert!(failed.origin.is_empty());
        assert_eq!(failed.in_package, "p");
        assert_causes(&r);
    }
}

#[test]
fn mixed_includes_check_failed_names_by_the_definition_source_version() {
    for version in ["0.3", "0.4"] {
        let files = MemoryProvider::new()
            .with("main.sp", "* includes\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include \"defs.sp\"\n")
            .with("defs.sp", &format!("* definitions\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n* @RNL {{ %p=%missing; $D = @DEFINE(roles=[]) {{ %p }} }}\n"));
        let r =
            rnl_cli::interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Default::default() })
                .unwrap();
        assert_eq!(errors(&r).len(), if version == "0.4" { 2 } else { 1 }, "{version}: {:?}", r.diagnostics());
        assert_causes(&r);
    }
}

#[test]
fn role_dependent_failed_locals_are_still_checked_at_each_application() {
    for used in [false, true] {
        let calls = if used { "#R1 @AS $D; #R2 @AS $D" } else { "" };
        let source=format!("* dynamic failures\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\nR2 b c 2k\n* @RNL {{ $D = @DEFINE(roles=[\"r\"]) {{ %p=?r::net; %p }}; {calls} }}\n");
        let r = rnl_cli::interpret_file(
            Path::new("failed.sp"),
            None,
            &MemoryProvider::new().with("failed.sp", &source),
            &Options { std: false, ..Default::default() },
        )
        .unwrap();
        assert_eq!(errors(&r).len(), if used { 4 } else { 0 }, "{:?}", r.diagnostics());
        assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| !d.origin.is_empty()));
    }
}

#[test]
fn nested_unused_declarations_check_fixed_missing_and_failed_names() {
    for inner in ["%Inner = @DEFINE(roles=[]) { %p=%missing; %p }", "%S = @DEFINE-SYMBOL { %p=%missing; %p }"] {
        for used in [false, true] {
            let r = read("0.4", "", inner, used);
            assert_eq!(errors(&r), [codes::REF_NOT_FOUND, codes::REF_FAILED], "{inner}: {:?}", r.diagnostics());
            assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| d.origin.is_empty()));
            assert_causes(&r);
        }
    }
}

#[test]
fn nested_unused_declarations_keep_their_own_name_hidden() {
    for inner in ["%Inner = @DEFINE(roles=[]) { %Inner }", "%Inner = @DEFINE-SYMBOL { %Inner }"] {
        for used in [false, true] {
            let r = read("0.4", "", inner, used);
            assert_eq!(errors(&r), [codes::REF_DEFINE_SELF_NO_OUTER_LOCAL], "{inner}: {:?}", r.diagnostics());
            assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| d.origin.is_empty()));
        }
    }
}
