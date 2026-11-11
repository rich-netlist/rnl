//! Check the fixed part of an application at the containing definition (6.9).
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn read(version: &str, header: &str, setup: &str, body: &str, after: &str, used: bool) -> Interpretation {
    let calls = if !used {
        ""
    } else if version == "0.4" {
        "$D @ATTACH(x=2) => @PLACE(0,0); $D @ATTACH(x=3) => @PLACE(10,0)"
    } else {
        "$D @ATTACH(x=2); $D @ATTACH(x=3)"
    };
    let source = format!("* callees\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {header} }}\n* @RNL {{ {setup} $D = @DEFINE(roles=[],parameters=[\"x\"]) {{ {body}; (5,0) @CIRCLE(radius=1) }}; {after} {calls} }}\n");
    rnl_cli::interpret_file(
        Path::new("callees.sp"),
        None,
        &MemoryProvider::new().with("callees.sp", &source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap()
}

fn errors(reading: &Interpretation) -> Vec<Code> {
    reading.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}

const PARAM: &str = "$C = @DEFINE(roles=[],parameters=[\"known\"]) {};";

#[test]
fn callee_parameter_names_and_counts_are_checked_without_applying_the_body() {
    for (body, expected) in [
        ("$C @ATTACH(unknown=1)", codes::PARAM_UNKNOWN),
        ("$C @ATTACH(1,2)", codes::PARAM_TOO_MANY_POSITIONAL),
        ("$C @ATTACH(1,known=2)", codes::PARAM_WRITTEN_TWICE),
        ("$C @ATTACH(priority=\"invalid\")", codes::PARAM_BAD_ENUM),
        ("$C @ATTACH(unknown=?x)", codes::PARAM_UNKNOWN),
    ] {
        for used in [false, true] {
            let r = read("0.4", "", PARAM, body, "", used);
            assert_eq!(errors(&r), [expected], "{body}: {:?}", r.diagnostics());
            assert!(r.diagnostics().iter().filter(|d| d.code == expected).all(|d| d.origin.is_empty()));
            assert_eq!(r.model.levels[0].shapes.len(), if used { 2 } else { 0 });
        }
    }
}

#[test]
fn actual_role_lists_are_checked_once_at_the_definition() {
    for (setup, body, expected) in [
        ("$C = @DEFINE(roles=[\"p\"]) {};", "$C @ATTACH", codes::USAGE_ROLE_NOT_FILLED),
        ("$C = @DEFINE(roles=[\"p\"]) {};", "$C (0,0) (1,0) @ATTACH", codes::USAGE_TOO_MANY_OBJECTS),
        ("$C = @DEFINE(roles=[\"p\"]) {};", "$C (0,0)(other) @ATTACH", codes::USAGE_UNKNOWN_ROLE),
        ("$C = @DEFINE(roles=[\"p\",\"q\"]) {};", "$C (0,0)(p) (1,0) @ATTACH", codes::ROLE_LABELS_MIXED),
        (
            "$C = @DEFINE(roles=[\"p\",\"q\"]) {};",
            "$C (0,0)(p) (1,0)(p) @ATTACH",
            codes::KEYWORD_POSITION_LABELLED_TWICE,
        ),
        ("$C = @DEFINE(roles=[[\"p\",\"net\"]]) {};", "(0,0) @AS $C", codes::USAGE_ROLE_WRONG_KIND),
    ] {
        for used in [false, true] {
            let r = read("0.4", "", setup, body, "", used);
            assert_eq!(errors(&r), [expected], "{body}: {:?}", r.diagnostics());
            assert!(r.diagnostics().iter().filter(|d| d.code == expected).all(|d| d.origin.is_empty()));
        }
    }
}

#[test]
fn callee_resolution_covers_forward_and_aliased_definitions() {
    for (setup, body, after) in [
        ("", "$C @ATTACH(unknown=1)", PARAM),
        ("$Original = @DEFINE(roles=[],parameters=[\"known\"]) {}; $C=$Original;", "$C @ATTACH(unknown=1)", ""),
        ("", "%C = @DEFINE(roles=[],parameters=[\"known\"]) {}; %C @ATTACH(unknown=1)", ""),
        ("", "%C @ATTACH(unknown=1); %C = @DEFINE(roles=[],parameters=[\"known\"]) {}", ""),
        ("", "%Alias=%C; %Alias @ATTACH(unknown=1); %C = @DEFINE(roles=[],parameters=[\"known\"]) {}", ""),
    ] {
        for used in [false, true] {
            let r = read("0.4", "", setup, body, after, used);
            assert_eq!(errors(&r), [codes::PARAM_UNKNOWN], "{body}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn nested_set_callees_keep_their_lexical_scope() {
    let setup = "$C = @DEFINE(roles=[],parameters=[\"outer\"]) {};";
    for used in [false, true] {
        let r = read(
            "0.4",
            "",
            setup,
            "%set={ %C = @DEFINE(roles=[],parameters=[\"inner\"]) {}; %C @ATTACH(inner=?x) }; $C @ATTACH(outer=?x)",
            "",
            used,
        );
        assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
        let r = read(
            "0.4",
            "",
            setup,
            "%set={ %C = @DEFINE(roles=[],parameters=[\"inner\"]) {}; %C @ATTACH(outer=?x) }",
            "",
            used,
        );
        assert_eq!(errors(&r), [codes::PARAM_UNKNOWN], "{:?}", r.diagnostics());
    }
}

#[test]
fn dynamic_values_and_omitted_set_roles_remain_valid() {
    for (setup, body) in [
        ("$C = @DEFINE(roles=[\"p\"],parameters=[\"known\"]) {};", "(?x,0) @AS(known=?x) $C"),
        ("$C = @DEFINE(roles=[\"{p}\"]) {};", "$C @ATTACH"),
        ("$C = @DEFINE(roles=[\"{p}\"]) {};", "(0,0) @AS $C"),
        ("", "%C = @DEFINE(roles=[],parameters=[[\"known\",?x]]) {}; %C @ATTACH(known=?x)"),
        ("$C = @DEFINE(roles=[],parameters=[\"known\"]) {};", "$C @ATTACH(known=[?x,1])"),
    ] {
        for used in [false, true] {
            let r = read("0.4", "", setup, body, "", used);
            assert!(errors(&r).is_empty(), "{body}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn pipes_count_as_single_unlabelled_role_arguments() {
    for used in [false, true] {
        let good = read("0.4", "", "$C = @DEFINE(roles=[\"p\"]) {};", "(0,0) => @AS $C", "", used);
        assert!(errors(&good).is_empty(), "{:?}", good.diagnostics());
        let bad = read("0.4", "", "$C = @DEFINE(roles=[\"p\",\"q\"]) {};", "(0,0) => $C @ATTACH", "", used);
        assert_eq!(errors(&bad), [codes::USAGE_ROLE_NOT_FILLED], "{:?}", bad.diagnostics());
    }
}

#[test]
fn declarations_are_not_expanded_to_check_their_signatures() {
    let setup = "$C = @DEFINE(roles=[]) { $D @ATTACH };";
    let r = read("0.4", "", setup, "$C @ATTACH", "", false);
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 0);
}

#[test]
fn released_definitions_keep_application_time_checks() {
    for version in ["0.2", "0.3"] {
        let unused = read(version, "", PARAM, "$C @ATTACH(unknown=1)", "", false);
        assert!(errors(&unused).is_empty(), "{:?}", unused.diagnostics());
        let used = read(version, "", PARAM, "$C @ATTACH(unknown=1)", "", true);
        assert_eq!(errors(&used), [codes::PARAM_UNKNOWN, codes::PARAM_UNKNOWN], "{:?}", used.diagnostics());
    }
}

#[test]
fn scoped_aliases_and_quoted_names_use_the_callees_namespace() {
    let header = "@ALIAS(\"alias_param\",\"known\",definition=\"$C\"); @ALIAS(\"alias_role\",\"p\",definition=\"$C\")";
    let setup = "$C = @DEFINE(roles=[\"p\"],parameters=[\"known\"]) {};";
    for used in [false, true] {
        let good = read("0.4", header, setup, "(0,0)(alias_role) @AS(alias_param=?x) $C", "", used);
        assert!(errors(&good).is_empty(), "{:?}", good.diagnostics());
        let quoted = read("0.4", header, setup, "(0,0)(alias_role) @AS(\"alias_param\"=?x) $C", "", used);
        assert_eq!(errors(&quoted), [codes::PARAM_UNKNOWN], "{:?}", quoted.diagnostics());
        let twice = read("0.4", header, setup, "$C (0,0)(p) (1,0)(alias_role) @ATTACH", "", used);
        assert_eq!(errors(&twice), [codes::USAGE_ROLE_LABELLED_TWICE], "{:?}", twice.diagnostics());
    }
}

#[test]
fn priority_alias_ambiguity_is_known_at_the_definition() {
    for used in [false, true] {
        let r = read(
            "0.4",
            "@ALIAS(\"rank\",\"priority\")",
            "$C = @DEFINE(roles=[],parameters=[\"rank\"]) {};",
            "$C @ATTACH(rank=?x)",
            "",
            used,
        );
        assert_eq!(errors(&r), [codes::USAGE_PARAM_ALIAS_AMBIGUOUS], "{:?}", r.diagnostics());
        let quoted = read(
            "0.4",
            "@ALIAS(\"rank\",\"priority\")",
            "$C = @DEFINE(roles=[],parameters=[\"rank\"]) {};",
            "$C @ATTACH(\"rank\"=?x)",
            "",
            used,
        );
        assert!(errors(&quoted).is_empty(), "{:?}", quoted.diagnostics());
    }
}

#[test]
fn declaration_keyword_and_field_aliases_keep_the_same_signature() {
    let header = "@ALIAS(\"@DECL\",\"@DEFINE\"); @ALIAS(\"args\",\"parameters\")";
    for used in [false, true] {
        let r = read("0.4", header, "$C = @DECL(roles=[],args=[\"known\"]) {};", "$C @ATTACH(unknown=1)", "", used);
        assert_eq!(errors(&r), [codes::PARAM_UNKNOWN], "{:?}", r.diagnostics());
    }
}

#[test]
fn package_signatures_are_resolved_where_the_body_is_written() {
    let files=MemoryProvider::new()
        .with("p.rnl","@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"p\",version=\"1.0\") }\n@RNL { $C = @DEFINE(roles=[\"p\"]) {}; $D = @DEFINE(roles=[]) { $C @ATTACH } }\n")
        .with("main.sp","* package callees\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"p\",path=\"p.rnl\") }\n* @RNL { $C = @DEFINE(roles=[]) {}; $D @ATTACH; $D @ATTACH }\n");
    for path in ["p.rnl", "main.sp"] {
        let r = rnl_cli::interpret_file(Path::new(path), None, &files, &Options { std: false, ..Default::default() })
            .unwrap();
        assert_eq!(errors(&r), [codes::USAGE_ROLE_NOT_FILLED], "{:?}", r.diagnostics());
        let diagnostic = r.diagnostics().into_iter().find(|d| d.code == codes::USAGE_ROLE_NOT_FILLED).unwrap();
        assert_eq!(diagnostic.in_package, "p");
        assert!(diagnostic.loc.as_ref().unwrap().file.ends_with("p.rnl"));
        assert!(diagnostic.origin.is_empty());
    }
}

#[test]
fn invalid_callee_headers_do_not_supply_a_guessed_signature() {
    let r = read("0.4", "", "$C = @DEFINE(roles=[\"p\",\"p\"]) {};", "$C @ATTACH(unknown=1)", "", false);
    assert_eq!(errors(&r), [codes::DEFINE_ROLE_NAME_INVALID, codes::REF_FAILED], "{:?}", r.diagnostics());
    let missing =
        read("0.4", "", "$C = @DEFINE(roles=?outside,parameters=[\"known\"]) {};", "$C @ATTACH(unknown=1)", "", false);
    assert_eq!(
        errors(&missing),
        [codes::REF_HOLE_OUTSIDE_DEFINITION, codes::REF_FAILED],
        "{:?}",
        missing.diagnostics()
    );
}

#[test]
fn declared_role_kinds_preserve_alternatives_and_late_conversions() {
    for (kind, target, expected) in [
        ("\"terminal\"", "pin", None),
        ("\"pin\"", "net", None),
        ("[\"point\",\"net\"]", "net", None),
        ("\"point\"", "net", Some(codes::USAGE_ROLE_WRONG_KIND)),
    ] {
        let source=format!("* kinds\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n* @RNL {{ $C = @DEFINE(roles=[[\"r\",\"{target}\"]]) {{}}; $D = @DEFINE(roles=[[\"p\",{kind}]]) {{ ?p @AS $C }} }}\n");
        let r = rnl_cli::interpret_file(
            Path::new("kinds.sp"),
            None,
            &MemoryProvider::new().with("kinds.sp", &source),
            &Options { std: false, ..Default::default() },
        )
        .unwrap();
        assert_eq!(errors(&r), expected.into_iter().collect::<Vec<_>>(), "{:?}", r.diagnostics());
    }
}

#[test]
fn additional_names_preserve_the_original_definition_alias_namespace() {
    let header = "@ALIAS(\"alias_param\",\"known\",definition=\"$Original\")";
    for setup in [
        "$Original = @DEFINE(roles=[],parameters=[\"known\"]) {} => $C;",
        "$Original = @DEFINE(roles=[],parameters=[\"known\"]) {}; $C=$Original;",
    ] {
        for used in [false, true] {
            let r = read("0.4", header, setup, "$C @ATTACH(alias_param=1)", "", used);
            assert!(errors(&r).is_empty(), "{setup}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn outer_aliases_are_resolved_in_their_own_set_scope() {
    let body="%C = @DEFINE(roles=[],parameters=[\"outer\"]) {}; %Alias=%C; %set={ %C = @DEFINE(roles=[],parameters=[\"inner\"]) {}; %Alias @ATTACH(outer=1) }";
    for used in [false, true] {
        let r = read("0.4", "", "", body, "", used);
        assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    }
}

#[test]
fn scoped_declarations_use_their_source_files_language_version() {
    for (source, target) in [("0.3", "0.4"), ("0.4", "0.3")] {
        let main=format!("* scoped declaration\n* @RNL(type=\"header\") {{ @VERSION(\"{source}\") }}\n.include \"child.sp\"\nX1 a b cell\n* @RNL(scope=\"cell\") {{ $C = @DEFINE(roles=[[\"p\",\"pin\"]]) {{}} }}\n");
        let child=format!("* scoped target\n* @RNL(type=\"header\") {{ @VERSION(\"{target}\") }}\n.subckt cell a b\nR1 a b 1k\n* @RNL {{ $D = @DEFINE(roles=[]) {{ $C @ATTACH(unknown=1) }} }}\n.ends cell\n");
        let files = MemoryProvider::new().with("main.sp", &main).with("child.sp", &child);
        let r =
            rnl_cli::interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Default::default() })
                .unwrap();
        assert_eq!(
            errors(&r),
            if source == "0.3" { vec![codes::REF_FAILED, codes::DEFINE_ROLE_UNKNOWN_KIND] } else { vec![] },
            "{source}/{target}: {:?}",
            r.diagnostics()
        );
    }
}

#[test]
fn a_definition_extending_its_own_name_uses_the_outer_callee() {
    for (parameter, expected) in [("known", None), ("unknown", Some(codes::PARAM_UNKNOWN))] {
        let source=format!("* extending\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n* @RNL {{ $D = @DEFINE(roles=[],parameters=[\"known\"]) {{}} }}\n.subckt cell a b\nR1 a b 1k\n* @RNL {{ $D = @DEFINE(roles=[]) {{ $D @ATTACH({parameter}=1) }} }}\n.ends cell\nX1 a b cell\n");
        let r = rnl_cli::interpret_file(
            Path::new("main.sp"),
            None,
            &MemoryProvider::new().with("main.sp", &source),
            &Options { std: false, ..Default::default() },
        )
        .unwrap();
        assert_eq!(errors(&r), expected.into_iter().collect::<Vec<_>>(), "{:?}", r.diagnostics());
    }
}

#[test]
fn a_dynamic_header_field_does_not_hide_the_other_fixed_signature_parts() {
    for (declaration, call, expected) in [
        ("roles=?items,parameters=[\"known\"]", "%C @ATTACH(unknown=1)", codes::PARAM_UNKNOWN),
        ("roles=[\"p\"],parameters=?items", "%C @ATTACH", codes::USAGE_ROLE_NOT_FILLED),
        ("roles=?items,parameters=?items", "%C @ATTACH(priority=\"invalid\")", codes::PARAM_BAD_ENUM),
        ("roles=?items,parameters=?items", "%C @ATTACH(value=1,value=2)", codes::SYNTAX_PARAM_TWICE),
        ("roles=?items,parameters=?items", "%C (0,0)(p) (1,0) @ATTACH", codes::ROLE_LABELS_MIXED),
    ] {
        for used in [false, true] {
            let calls = if used { "$D @ATTACH; $D @ATTACH" } else { "" };
            let source=format!("* dynamic headers\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n* @RNL {{ $D = @DEFINE(roles=[],parameters=[[\"items\",[]]]) {{ %C = @DEFINE({declaration}) {{}}; {call} }}; {calls} }}\n");
            let r = rnl_cli::interpret_file(
                Path::new("dynamic.sp"),
                None,
                &MemoryProvider::new().with("dynamic.sp", &source),
                &Options { std: false, ..Default::default() },
            )
            .unwrap();
            assert_eq!(errors(&r), [expected], "{declaration}/{call}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn argument_dependent_role_counts_remain_application_checks() {
    for used in [false, true] {
        let calls = if used { "$D @ATTACH; $D @ATTACH" } else { "" };
        let source=format!("* dynamic roles\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n* @RNL {{ $D = @DEFINE(roles=[],parameters=[[\"items\",[\"p\"]]]) {{ %C = @DEFINE(roles=?items) {{}}; %C @ATTACH }}; {calls} }}\n");
        let r = rnl_cli::interpret_file(
            Path::new("dynamic.sp"),
            None,
            &MemoryProvider::new().with("dynamic.sp", &source),
            &Options { std: false, ..Default::default() },
        )
        .unwrap();
        assert_eq!(
            errors(&r),
            if used { vec![codes::USAGE_ROLE_NOT_FILLED; 2] } else { vec![] },
            "{:?}",
            r.diagnostics()
        );
        assert!(r
            .diagnostics()
            .iter()
            .filter(|d| d.code == codes::USAGE_ROLE_NOT_FILLED)
            .all(|d| !d.origin.is_empty()));
    }
}
