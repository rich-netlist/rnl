//! A failed creation does not reserve a level name (6.6).
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn read(version: &str, body: &str) -> Interpretation {
    let source=format!("* global bindings\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 b c 2k\n* @RNL {{ {body} }}\n");
    rnl_cli::interpret_file(
        Path::new("bindings.sp"),
        None,
        &MemoryProvider::new().with("bindings.sp", &source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap()
}
fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}

#[test]
fn a_failed_level_reference_does_not_reserve_the_name() {
    for body in ["%p=%missing; %p=(4,0); %p @CIRCLE(radius=2)", "%p @CIRCLE(radius=2); %p=%missing; %p=(4,0)"] {
        let r = read("0.4", body);
        assert_eq!(errors(&r), [codes::REF_NOT_FOUND], "{body}: {:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].shapes.len(), 1);
        assert_eq!(r.model.levels[0].shapes[0].points, [(4., 0.)]);
    }
}

#[test]
fn a_failed_declaration_does_not_reserve_the_name() {
    for body in [
        "$A = @DEFINE(roles=[\"r\",\"r\"]) {}; $A = @DEFINE(roles=[]) { (4,0) @CIRCLE(radius=2) }; $A @ATTACH => @PLACE(0,0)",
        "$A @ATTACH => @PLACE(0,0); $A = @DEFINE(roles=[\"r\",\"r\"]) {}; $A = @DEFINE(roles=[]) { (4,0) @CIRCLE(radius=2) }",
    ] {
        let r = read("0.4", body);
        assert_eq!(errors(&r), [codes::DEFINE_ROLE_NAME_INVALID], "{body}: {:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].shapes.len(), 1);
    }
}

#[test]
fn the_first_valid_binding_is_kept_and_only_later_valid_conflicts_are_reported() {
    let r = read("0.4", "%p=%missing; %p=(4,0); %p=(8,0); %p @CIRCLE(radius=2)");
    assert_eq!(errors(&r), [codes::REF_NOT_FOUND, codes::NAME_BOUND_TWICE], "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 1);
    assert_eq!(r.model.levels[0].shapes[0].points, [(4., 0.)]);
}

#[test]
fn failed_modifiers_keep_their_valid_input_binding() {
    let r = read(
        "0.4",
        "#R1 @PLACE(0,0); #R2 @PLACE(10,0); %p=#R1 @PLACE(\"bad\",0); %p=#R2; %p @TEXT(\"kept\",key=\"name\")",
    );
    assert_eq!(errors(&r).len(), 2, "{:?}", r.diagnostics());
    assert!(errors(&r).contains(&codes::NAME_BOUND_TWICE));
    assert_eq!(r.model.levels[0].texts.iter().find(|t| t.markup == "kept").unwrap().attach.as_deref(), Some("#R1"));
}

#[test]
fn released_versions_keep_failed_first_binding_behavior() {
    for version in ["0.2", "0.3"] {
        let r = read(version, "%p=%missing; %p=(4,0); %p @CIRCLE(radius=2)");
        assert_eq!(
            errors(&r),
            [codes::REF_NOT_FOUND, codes::REF_FAILED, codes::NAME_BOUND_TWICE],
            "{:?}",
            r.diagnostics()
        );
        assert!(r.model.levels[0].shapes.is_empty());
    }
}

#[test]
fn a_prefix_and_an_intermediate_binding_are_compared_in_written_order() {
    let r = read("0.4", "%p=(0,0) => %p => @CIRCLE(radius=1); %p @CIRCLE(radius=2)");
    assert_eq!(errors(&r).len(), 2, "{:?}", r.diagnostics());
    assert!(errors(&r).contains(&codes::NAME_BOUND_TWICE));
    assert!(errors(&r).contains(&codes::KEYWORD_GRAPHIC_NOT_POINTS));
    assert_eq!(r.model.levels[0].shapes.len(), 1);
}

#[test]
fn a_failed_prefix_leaves_a_valid_intermediate_binding_available() {
    let r = read("0.4", "%p=(4,0) => %p => @AS $Missing; %p @CIRCLE(radius=2)");
    assert_eq!(errors(&r), [codes::REF_NOT_FOUND], "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 1);
    assert_eq!(r.model.levels[0].shapes[0].points, [(4., 0.)]);
}

#[test]
fn repeating_a_valid_handle_and_cycles_with_a_valid_later_binding() {
    let same = read("0.4", "%p=(4,0) => %p; %q=%p; %q=%p; %q @CIRCLE(radius=2)");
    assert!(errors(&same).is_empty(), "{:?}", same.diagnostics());
    for body in ["%x=%x; %x=(4,0); %x @CIRCLE(radius=2)", "%x=%y; %y=%x; %x=(4,0); %x @CIRCLE(radius=2)"] {
        let r = read("0.4", body);
        assert!(errors(&r).contains(&codes::REF_NAME_CYCLE), "{body}: {:?}", r.diagnostics());
        assert!(!errors(&r).contains(&codes::NAME_BOUND_TWICE), "{body}: {:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].shapes.len(), 1);
    }
}

#[test]
fn package_bindings_recover_before_forward_aliases_and_applications() {
    let files=MemoryProvider::new()
        .with("p.rnl","@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"p\",version=\"1.0\") }\n@RNL { $B=$A; $A = @DEFINE(roles=[\"r\",\"r\"]) {}; $A = @DEFINE(roles=[]) { (1,0) @CIRCLE(radius=1) } }\n")
        .with("main.sp","* package bindings\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"p\",path=\"p.rnl\") }\n* @RNL { $B @ATTACH => @PLACE(0,0) }\n");
    for path in ["p.rnl", "main.sp"] {
        let r = rnl_cli::interpret_file(Path::new(path), None, &files, &Options { std: false, ..Default::default() })
            .unwrap();
        assert_eq!(errors(&r), [codes::DEFINE_ROLE_NAME_INVALID], "{path}: {:?}", r.diagnostics());
        assert_eq!(
            r.diagnostics().into_iter().find(|d| d.code == codes::DEFINE_ROLE_NAME_INVALID).unwrap().in_package,
            "p"
        );
        if path == "main.sp" {
            assert_eq!(r.model.levels[0].shapes.len(), 1);
        }
    }
}

#[test]
fn included_bindings_follow_their_source_language_version() {
    for (main, version) in [("0.3", "0.4"), ("0.4", "0.3")] {
        let files=MemoryProvider::new()
            .with("main.sp",&format!("* bindings\n* @RNL(type=\"header\") {{ @VERSION(\"{main}\") }}\n.include \"child.sp\"\n"))
            .with("child.sp",&format!("* child\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n.subckt child a b\nR1 a b 1k\n* @RNL {{ %p=%missing; %p=(4,0); %p @CIRCLE(radius=2) }}\n.ends child\n"));
        let r =
            rnl_cli::interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Default::default() })
                .unwrap();
        assert_eq!(
            errors(&r),
            if version == "0.4" {
                vec![codes::REF_NOT_FOUND]
            } else {
                vec![codes::REF_NOT_FOUND, codes::REF_FAILED, codes::NAME_BOUND_TWICE]
            },
            "{version}: {:?}",
            r.diagnostics()
        );
        assert_eq!(r.model.levels.iter().map(|l| l.shapes.len()).sum::<usize>(), usize::from(version == "0.4"));
    }
}

#[test]
fn a_late_pin_failure_keeps_the_created_usage_binding() {
    let r=read("0.4","$S = @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal=1) }; $D = @DEFINE(roles=[[\"t\",\"pin\"]]) {}; %x=#R1::missing @AS $D; %x=(4,0); %x @CIRCLE(radius=2); #R1 @SET-SYMBOL $S => @PLACE(0,0)");
    assert!(errors(&r).contains(&codes::NAME_BOUND_TWICE), "{:?}", r.diagnostics());
    assert!(errors(&r).contains(&codes::REF_NO_SUBOBJECT), "{:?}", r.diagnostics());
    assert!(r.model.levels[0].failed_usages.iter().any(|u| u.id.as_deref() == Some("%x")));
    assert!(r.model.levels[0].shapes.is_empty());
}

#[test]
fn exported_points_keep_the_first_valid_name() {
    let r = read("0.4", "%p=%missing; %p=(4,0); %p @CIRCLE(radius=2)");
    assert_eq!(errors(&r), [codes::REF_NOT_FOUND], "{:?}", r.diagnostics());
    assert!(r.model.levels[0].points.iter().any(|p| p.id == "%p"), "{:?}", r.model.levels[0].points);
}

#[test]
fn a_record_outside_the_host_language_does_not_reserve_the_name() {
    let files = MemoryProvider::new().with(
        "main.sp",
        r#"* bindings
* @RNL(type="header") { @VERSION("0.4") }
R1 a b 1k
* @RNL(language="spectre") { %p=(8,0) }
* @RNL(language="spice") { %p=(4,0) }
* @RNL { %p @CIRCLE(radius=2) }
"#,
    );
    let r = rnl_cli::interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Default::default() })
        .unwrap();
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 1);
    assert_eq!(r.model.levels[0].shapes[0].points, [(4., 0.)]);
}

#[test]
fn package_catalog_uses_the_surviving_definition_and_symbol() {
    let files = MemoryProvider::new().with(
        "p.rnl",
        r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="p",version="1.0") }
@RNL { $D=$missing; $D = @DEFINE(roles=["r"]) {}; $S=$missing; $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) } }
"#,
    );
    let catalog =
        rnl_runtime::interp::catalog::package_contents("p.rnl", &files, &Options { std: false, ..Default::default() })
            .unwrap();
    assert_eq!(catalog.definitions.len(), 1);
    assert_eq!(catalog.definitions[0].name, "$D");
    assert_eq!(catalog.definitions[0].roles.len(), 1);
    assert_eq!(catalog.definitions[0].roles[0].str(), Some("r"));
    assert_eq!(catalog.symbols.len(), 1);
    assert_eq!(catalog.symbols[0].name, "$S");
}

#[test]
fn a_junction_setting_uses_the_first_valid_definition() {
    let files = MemoryProvider::new().with(
        "main.sp",
        r#"* bindings
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$D") }
R1 a b 1k
* @RNL { $D=$missing; $D = @DEFINE(roles=[["p","point"]]) {}; %p=(4,0); $D @ATTACH %p }
"#,
    );
    let r = rnl_cli::interpret_file(
        Path::new("main.sp"),
        None,
        &files,
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap();
    assert_eq!(errors(&r), [codes::REF_NOT_FOUND], "{:?}", r.diagnostics());
    assert_eq!(r.problems[0].junction.definition.as_deref(), Some("$D"));
    assert!(r.model.levels[0].points.iter().find(|p| p.id == "%p").unwrap().junction);
}

#[test]
fn unused_scoped_alias_checks_use_the_surviving_definition() {
    let files = MemoryProvider::new().with(
        "main.sp",
        r#"* bindings
* @RNL(type="header") { @VERSION("0.4"); @ALIAS("r","other",definition="$D") }
R1 a b 1k
* @RNL { $D=$missing; $D = @DEFINE(roles=["r"]) {} }
"#,
    );
    let r = rnl_cli::interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Default::default() })
        .unwrap();
    assert_eq!(errors(&r), [codes::REF_NOT_FOUND, codes::ALIAS_SCOPED_LIKE_OWN_NAME], "{:?}", r.diagnostics());
}

#[test]
fn a_recovered_callee_is_checked_at_the_definition_even_when_forward_or_unused() {
    for failed in ["$C=$missing;", "$C = @DEFINE(roles=[\"p\",\"p\"]) {};"] {
        for forward in [false, true] {
            for used in [false, true] {
                let callee = format!("{failed} $C = @DEFINE(roles=[],parameters=[\"known\"]) {{}};");
                let definition =
                    "$D = @DEFINE(roles=[]) { (0,0) @CIRCLE(radius=1) => @AS(unknown=1) $C; (4,0) @CIRCLE(radius=2) };";
                let calls = if used { "$D @ATTACH => @PLACE(0,0); $D @ATTACH => @PLACE(10,0)" } else { "" };
                let body = if forward {
                    format!("{definition}{callee}{calls}")
                } else {
                    format!("{callee}{definition}{calls}")
                };
                let r = read("0.4", &body);
                let diagnostics: Vec<_> =
                    r.diagnostics().into_iter().filter(|d| d.code == codes::PARAM_UNKNOWN).collect();
                assert_eq!(diagnostics.len(), 1, "{body}: {:?}", r.diagnostics());
                assert!(diagnostics[0].origin.is_empty(), "{body}: {:?}", r.diagnostics());
                assert_eq!(r.model.levels[0].shapes.len(), if used { 2 } else { 0 }, "{body}: {:?}", r.diagnostics());
            }
        }
    }
}

#[test]
fn inactive_declarations_do_not_hide_a_junction_or_a_callee_signature() {
    let files=MemoryProvider::new().with("main.sp",r#"* bindings
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$C") }
R1 a b 1k
* @RNL(language="spectre") { $C=$missing }
* @RNL { $D = @DEFINE(roles=[]) { (0,0) @AS(unknown=1) $C }; $C = @DEFINE(roles=[["p","point"]]) {}; %p=(4,0); $C @ATTACH %p }
"#);
    let r = rnl_cli::interpret_file(
        Path::new("main.sp"),
        None,
        &files,
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap();
    assert_eq!(errors(&r), [codes::PARAM_UNKNOWN], "{:?}", r.diagnostics());
    assert_eq!(r.problems[0].junction.definition.as_deref(), Some("$C"));
    assert!(r.model.levels[0].points.iter().find(|p| p.id == "%p").unwrap().junction);
}

#[test]
fn a_recovered_package_callee_is_checked_without_a_scene() {
    let files = MemoryProvider::new()
        .with(
            "p.rnl",
            r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="p",version="1.0") }
@RNL { $D = @DEFINE(roles=[]) { (0,0) @AS(unknown=1) $C }; $C=$missing; $C = @DEFINE(roles=["p"]) {} }
"#,
        )
        .with(
            "main.sp",
            r#"* bindings
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="p",path="p.rnl") }
* @RNL { $D @ATTACH => @PLACE(0,0) }
"#,
        );
    for path in ["p.rnl", "main.sp"] {
        let r = rnl_cli::interpret_file(Path::new(path), None, &files, &Options { std: false, ..Default::default() })
            .unwrap();
        assert_eq!(errors(&r).len(), 2, "{path}: {:?}", r.diagnostics());
        let diagnostics: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::PARAM_UNKNOWN).collect();
        assert_eq!(diagnostics.len(), 1, "{path}: {:?}", r.diagnostics());
        assert!(diagnostics[0].origin.is_empty());
        assert_eq!(diagnostics[0].in_package, "p");
    }
}

#[test]
fn a_recovered_definition_does_not_see_its_own_name() {
    for prefix in ["$", "%"] {
        let r =
            read("0.4", &format!("{prefix}D={prefix}missing; {prefix}D = @DEFINE(roles=[]) {{ {prefix}D @ATTACH }}"));
        let expected =
            if prefix == "$" { codes::REF_DEFINE_SELF_NO_OUTER } else { codes::REF_DEFINE_SELF_NO_OUTER_LOCAL };
        assert_eq!(errors(&r), [codes::REF_NOT_FOUND, expected], "{prefix}: {:?}", r.diagnostics());
    }
}

#[test]
fn a_recovered_definition_still_uses_the_outer_package_definition() {
    let files = MemoryProvider::new()
        .with(
            "p.rnl",
            r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="p",version="1.0") }
@RNL { $D = @DEFINE(roles=[],parameters=["outer"]) {} }
"#,
        )
        .with(
            "main.sp",
            r#"* bindings
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="p",path="p.rnl") }
* @RNL { $D=$missing; $D = @DEFINE(roles=[]) { $D @ATTACH(outer=1) } }
"#,
        );
    let r = rnl_cli::interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Default::default() })
        .unwrap();
    assert_eq!(errors(&r), [codes::REF_NOT_FOUND], "{:?}", r.diagnostics());
}
