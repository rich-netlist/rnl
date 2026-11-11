//! Header, body and syntax parameter errors share kinds in language 0.4.
use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn source(version: &str, header: &str, body: &str) -> String {
    format!("* parameters\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {header} }}\n* @RNL {{ {body} }}\n")
}
fn files(provider: MemoryProvider) -> Interpretation {
    interpret_file(Path::new("main.sp"), None, &provider, &Options { std: false, ..Default::default() }).unwrap()
}
fn read(version: &str, header: &str, body: &str) -> Interpretation {
    files(MemoryProvider::new().with("main.sp", &source(version, header, body)))
}
fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}

#[test]
fn unknown_parameters_share_their_kind_and_suggestion() {
    for (header, body, keyword, name, suggestion) in [
        ("@DEVICE(nam=\"R*\")", "", "DEVICE", "nam", "name"),
        ("", "(0,0) @CIRCLE(radiu=1)", "CIRCLE", "radiu", "radius"),
    ] {
        let r = read("0.4", header, body);
        assert_eq!(errors(&r), [codes::PARAM_UNKNOWN], "{:?}", r.diagnostics());
        let d = r.diagnostics().into_iter().find(|d| d.code == codes::PARAM_UNKNOWN).unwrap();
        assert_eq!(d.message, format!("@{keyword} has no parameter {name}, did you mean {suggestion}? (16.4)"));
    }
}

#[test]
fn positional_parameters_share_their_kind_and_actionable_example() {
    for (header, body, example) in
        [("@DEVICE(\"R*\")", "", "as name=value"), ("", "$D = @DEFINE(\"r\") {}", "@DEFINE(roles=")]
    {
        let r = read("0.4", header, body);
        assert_eq!(errors(&r), [codes::PARAM_POSITIONAL_NOT_ALLOWED], "{:?}", r.diagnostics());
        assert!(r.diagnostics().iter().any(|d| d.message.contains(example)));
    }
}

#[test]
fn too_many_positions_keep_the_expected_order_and_actual_count() {
    for (header, body, expected) in [
        (
            "@ALIAS(\"alias\",\"name\",\"extra\")",
            "",
            "@ALIAS takes at most 2 parameters by position (alias, original), 3 are written (4.2 rule 9)",
        ),
        ("", "(0,0) @PLACE(1,2,3)", "@PLACE takes at most 2 parameters by position (x, y), 3 are written (4.2 rule 9)"),
    ] {
        let r = read("0.4", header, body);
        assert_eq!(errors(&r), [codes::PARAM_TOO_MANY_POSITIONAL], "{:?}", r.diagnostics());
        assert!(r.diagnostics().iter().any(|d| d.message == expected));
    }
}

#[test]
fn literal_and_resolved_duplicates_share_the_kind_and_keyword() {
    for (header, body, keyword, parameter) in [
        ("@DEVICE(name=\"R*\",name=\"C*\")", "", "DEVICE", "name"),
        ("@ALIAS(\"alias\",alias=\"again\",original=\"name\")", "", "ALIAS", "alias"),
        ("", "(0,0) @CIRCLE(radius=1,radius=2)", "CIRCLE", "radius"),
        ("", "(0,0) @PLACE(1,x=2)", "PLACE", "x"),
    ] {
        let r = read("0.4", header, body);
        assert_eq!(errors(&r), [codes::PARAM_WRITTEN_TWICE], "{:?}", r.diagnostics());
        assert!(r
            .diagnostics()
            .iter()
            .any(|d| d.message == format!("parameter {parameter} of @{keyword} is written twice (4.2 rule 9)")));
    }
}

#[test]
fn non_ascii_unknown_names_explain_the_actual_alias_context() {
    for quoted in [false, true] {
        let name = if quoted { "\"半径\"" } else { "半径" };
        for header in [false, true] {
            let call = format!("@{}({name}=1)", if header { "DEVICE" } else { "CIRCLE" });
            let body = format!("(0,0) {call}");
            let r = if header { read("0.4", &call, "") } else { read("0.4", "", &body) };
            assert_eq!(errors(&r), [codes::PARAM_UNKNOWN], "{:?}", r.diagnostics());
            let d = r.diagnostics().into_iter().find(|d| d.code == codes::PARAM_UNKNOWN).unwrap();
            if quoted {
                assert!(!d.message.contains("alias"));
            } else if header {
                assert!(d.message.contains("; a header record is read in the standard form, without aliases (16.5)"));
            } else {
                assert!(d.message.contains("; is an alias package not loaded?"));
            }
        }
    }
}

#[test]
fn a_literal_duplicate_points_to_the_second_written_parameter() {
    let body = "(0,0) @CIRCLE(radius=1,radius=2); (9,0) @CIRCLE(radius=1)";
    let r = read("0.4", "", body);
    assert_eq!(errors(&r), [codes::PARAM_WRITTEN_TWICE]);
    let d = r.diagnostics().into_iter().find(|d| d.code == codes::PARAM_WRITTEN_TWICE).unwrap();
    let loc = d.loc.as_ref().unwrap();
    assert_eq!(loc.line, 3);
    assert_eq!(loc.col as usize, format!("* @RNL {{ {body} }}").find("radius=2").unwrap() + 1);
}

#[test]
fn released_versions_keep_the_separate_header_and_syntax_kinds() {
    for version in ["0.2", "0.3"] {
        assert_eq!(errors(&read(version, "@DEVICE(1)", "")), [codes::HEADER_PARAM_NOT_POSITIONAL]);
        assert_eq!(errors(&read(version, "@DEVICE(nope=1)", "")), [codes::HEADER_PARAM_UNKNOWN]);
        assert_eq!(errors(&read(version, "", "(0,0) @CIRCLE(radius=1,radius=2)")), [codes::SYNTAX_PARAM_TWICE]);
        let r = read(version, "", "(0,0) @CIRCLE(半径=1)");
        assert!(r.diagnostics().iter().any(|d| d.message.contains(" Is an alias package not loaded?")));
    }
}

#[test]
fn included_files_use_their_own_parameter_diagnostic_version() {
    for (root, child) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let main = source(root, "@DEVICE(nope=1)", "") + ".include child.sp\n";
        let r =
            files(MemoryProvider::new().with("main.sp", &main).with("child.sp", &source(child, "@DEVICE(nope=1)", "")));
        for (file, version) in [("main.sp", root), ("child.sp", child)] {
            let expected = if version == "0.4" { codes::PARAM_UNKNOWN } else { codes::HEADER_PARAM_UNKNOWN };
            assert!(
                r.diagnostics()
                    .iter()
                    .any(|d| d.code == expected && d.loc.as_ref().is_some_and(|l| l.file.ends_with(file))),
                "{:?}",
                r.diagnostics()
            );
        }
    }
}

#[test]
fn aliases_cannot_hide_duplicate_parameters_and_quotes_disable_aliases() {
    let header = "@ALIAS(\"rad\",\"radius\")";
    let r = read("0.4", header, "(0,0) @CIRCLE(radius=1,rad=2)");
    assert_eq!(errors(&r), [codes::PARAM_WRITTEN_TWICE], "{:?}", r.diagnostics());
    let r = read("0.4", header, "(0,0) @CIRCLE(radius=1,\"rad\"=2)");
    assert_eq!(errors(&r), [codes::PARAM_UNKNOWN], "{:?}", r.diagnostics());
}

#[test]
fn package_parameter_errors_keep_the_package_and_source_location() {
    let provider = MemoryProvider::new()
        .with("main.sp", &source("0.4", "@USE-PACKAGE(name=\"p\",path=\"p.rnl\")", ""))
        .with("p.rnl", "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"p\",version=\"1.0\"); @DEVICE(nope=1) }\n@RNL { $D = @DEFINE(roles=[]) { (0,0) @CIRCLE(radius=1,radius=2) } }\n");
    let r = files(provider);
    assert_eq!(errors(&r), [codes::PARAM_UNKNOWN, codes::PARAM_WRITTEN_TWICE], "{:?}", r.diagnostics());
    for d in r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error) {
        assert_eq!(d.in_package, "p");
        assert!(d.loc.as_ref().unwrap().file.ends_with("p.rnl"));
        assert!(d.origin.is_empty());
    }
}

#[test]
fn header_parameters_before_version_use_the_files_declared_version() {
    for version in ["0.3", "0.4"] {
        let text = format!("* header order\n* @RNL(type=\"header\") {{ @DEVICE(nope=1); @VERSION(\"{version}\") }}\n");
        let r = files(MemoryProvider::new().with("main.sp", &text));
        assert_eq!(
            errors(&r),
            [if version == "0.4" { codes::PARAM_UNKNOWN } else { codes::HEADER_PARAM_UNKNOWN }],
            "{:?}",
            r.diagnostics()
        );
    }
}
