//! Junction identity and writeback use the configured definition in the target level (13.2).
use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::{codes, Severity},
    problem::LevelProblem,
    solution::{write_record, Branch, Solution},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;
fn source(version: &str, header: &str, body: &str) -> String {
    let body = body.replace('\n', "\n* ");
    format!(
        "* junction\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {header} }}\nR1 a 0 1k\n* @RNL {{ {body} }}\n"
    )
}
fn files(files: &MemoryProvider) -> Interpretation {
    interpret_file(Path::new("main.sp"), None, files, &Options { std: false, problems: true, ..Default::default() })
        .unwrap()
}
fn read(header: &str, body: &str) -> Interpretation {
    files(&MemoryProvider::new().with("main.sp", &source("0.4", header, body)))
}
fn clean(r: &Interpretation) {
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
}
fn write(p: &LevelProblem) -> rnl_runtime::solution::Written {
    write_record(
        p,
        &Solution { branches: vec![Branch { at: (2., 3.), junction: true }], ..Default::default() },
        "test",
        "hash",
    )
}
const DOT: &str = r#"$DOT = @DEFINE(roles=[["p","point"]]) {};"#;
#[test]
fn configured_definition_identity_and_first_role_mark_points() {
    let r = read(
        r#"@SOLVER-CONFIG(junction="$DOT")"#,
        r#"$DOT = @DEFINE(roles=[["p","point"],["other","point"]]) {}; $ALSO = $DOT;
        $JUNCTION = @DEFINE(roles=[["p","point"]]) {};
        %a=(0,0); %b=(1,0); %c=(2,0); $ALSO @ATTACH %a %b; $JUNCTION @ATTACH %c"#,
    );
    clean(&r);
    let points = &r.model.levels[0].points;
    assert!(points.iter().find(|p| p.id == "%a").unwrap().junction);
    assert!(!points.iter().find(|p| p.id == "%b").unwrap().junction);
    assert!(!points.iter().find(|p| p.id == "%c").unwrap().junction);
    assert_eq!(r.problems[0].junction.definition.as_deref(), Some("$DOT"));
}
#[test]
fn solver_writes_configured_reference_and_round_trips() {
    let body = format!("{DOT} %a=(0,0)");
    let header = r#"@SOLVER-CONFIG(junction="$DOT")"#;
    let r = read(header, &body);
    clean(&r);
    let w = write(&r.problems[0]);
    assert!(w.warnings.is_empty());
    assert!(w.record.contains("$DOT @ATTACH %rt1"), "{}", w.record);
    let text = source("0.4", header, &body) + &w.record.lines().map(|l| format!("* {l}\n")).collect::<String>();
    let again = files(&MemoryProvider::new().with("main.sp", &text));
    clean(&again);
    assert!(again.model.levels[0].points.iter().find(|p| p.id == "%rt1").unwrap().junction);
}
#[test]
fn absent_and_empty_settings_do_not_guess_a_standard_name() {
    for header in ["", r#"@SOLVER-CONFIG(junction="")"#] {
        let r = read(header, r#"$JUNCTION = @DEFINE(roles=[["p","point"]]) {}; %p=(0,0); $JUNCTION @ATTACH %p"#);
        clean(&r);
        assert!(!r.model.levels[0].points.iter().find(|p| p.id == "%p").unwrap().junction);
        let w = write(&r.problems[0]);
        assert!(!w.record.contains("@ATTACH"));
        assert!(w.warnings.is_empty());
    }
}
#[test]
fn invalid_config_is_a_located_consumer_warning_not_a_read_error() {
    for value in [
        "3",
        "true",
        "[]",
        r#""$MISSING""#,
        r#""DOT""#,
        r#""$DOT; @NOTHING""#,
        r#""$DOT::p""#,
        r#""$DOT{1}""#,
        r#""$""#,
    ] {
        let r = read(&format!("@SOLVER-CONFIG(junction={value})"), DOT);
        clean(&r);
        assert!(!r.diagnostics().iter().any(|d| d.code == codes::JUNCTION_DEFINITION_INVALID));
        let w = write(&r.problems[0]);
        assert!(!w.record.contains("@ATTACH"), "{value}");
        assert_eq!(w.warnings.len(), 1, "{value}");
        let d = &w.warnings[0];
        assert_eq!(d.code, codes::JUNCTION_DEFINITION_INVALID);
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(d.loc.as_ref().unwrap().line, 2);
    }
}
#[test]
fn symbol_style_roleless_and_nonpoint_first_role_are_invalid() {
    for definition in [
        r#"$DOT = @DEFINE-SYMBOL {}"#,
        r#"$DOT = @DEFINE-STYLE {}"#,
        r#"$DOT = @DEFINE(roles=[]) {}"#,
        r#"$DOT = @DEFINE(roles=[["p","net"],["q","point"]]) {}"#,
    ] {
        let r = read(r#"@SOLVER-CONFIG(junction="$DOT")"#, definition);
        clean(&r);
        assert!(r.problems[0].junction.definition.is_none());
        assert_eq!(write(&r.problems[0]).warnings.len(), 1);
    }
}
#[test]
fn untyped_union_and_set_point_roles_are_accepted() {
    for roles in [r#"["p"]"#, r#"[["p",["point","terminal"]]]"#, r#"[["{p}","point"]]"#] {
        let r = read(
            r#"@SOLVER-CONFIG(junction="$DOT")"#,
            &format!("$DOT = @DEFINE(roles={roles}) {{}}; %p=(0,0); $DOT @ATTACH %p"),
        );
        clean(&r);
        assert!(r.problems[0].junction.definition.is_some(), "{roles}");
        assert!(r.model.levels[0].points.iter().find(|p| p.id == "%p").unwrap().junction, "{roles}");
    }
}
#[test]
fn quoted_names_and_file_aliases_resolve_in_target_file() {
    let r = read(r#"@SOLVER-CONFIG(junction="$alias"); @ALIAS("$alias","$DOT")"#, DOT);
    clean(&r);
    assert!(write(&r.problems[0]).record.contains("$DOT @ATTACH"));
    let r = read(r#"@SOLVER-CONFIG(junction="$""draw dot""")"#, r#"$"draw dot" = @DEFINE(roles=[["p","point"]]) {}"#);
    clean(&r);
    assert!(write(&r.problems[0]).record.contains("$\"draw dot\" @ATTACH"));
}
#[test]
fn package_setting_uses_target_shadow_not_package_private_definition() {
    let pkg = r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="dots",version="1.0"); @SOLVER-CONFIG(junction="$DOT")}
        @RNL {$DOT = @DEFINE(roles=[["p","point"]]) {}}"#;
    let mut fs = MemoryProvider::new().with("pkg/dots.rnl", pkg);
    fs.package_dirs.push("pkg".into());
    let main = source("0.4", r#"@USE-PACKAGE(name="dots")"#, r#"$DOT = @DEFINE(roles=[["n","net"]]) {}"#);
    fs.insert("main.sp", &main);
    let r = files(&fs);
    clean(&r);
    assert!(r.problems[0].junction.definition.is_none());
    let warning = write(&r.problems[0]).warnings.remove(0);
    assert_eq!(warning.in_package, "dots");
    assert!(warning.loc.unwrap().file.ends_with("dots.rnl"));
    fs.insert("main.sp", &source("0.4", r#"@USE-PACKAGE(name="dots")"#, ""));
    let r = files(&fs);
    clean(&r);
    assert_eq!(r.problems[0].junction.definition.as_deref(), Some("$DOT"));
}
#[test]
fn subcircuit_names_override_file_names_independently() {
    let text = source("0.4", r#"@SOLVER-CONFIG(junction="$DOT")"#, DOT)
        + ".subckt child a b\nR2 a b 1k\n* @RNL {$DOT = @DEFINE(roles=[[\"n\",\"net\"]]) {}}\n.ends child\n";
    let r = files(&MemoryProvider::new().with("main.sp", &text));
    clean(&r);
    assert_eq!(r.problems[0].junction.definition.as_deref(), Some("$DOT"));
    assert!(r.problems.iter().find(|p| p.name == "child").unwrap().junction.invalid.is_some());
}
#[test]
fn released_versions_keep_standard_name_writeback() {
    for version in ["0.2", "0.3"] {
        let r = files(&MemoryProvider::new().with(
            "main.sp",
            &source(version, "", r#"$JUNCTION = @DEFINE(roles=["p"]) {}; %p=(0,0); $JUNCTION @ATTACH %p"#),
        ));
        clean(&r);
        assert!(r.model.levels[0].points.iter().find(|p| p.id == "%p").unwrap().junction);
        let w = write(&r.problems[0]);
        assert!(w.record.contains("$JUNCTION @ATTACH"));
        assert!(w.warnings.is_empty());
    }
}

#[test]
fn quoted_dollar_aliases_resolve_as_they_do_in_target_records() {
    for alias in ["alias", "标记"] {
        let header = format!("@ALIAS(\"${alias}\",\"$DOT\"); @SOLVER-CONFIG(junction=\"$\"\"{alias}\"\"\")");
        let r = read(&header, DOT);
        clean(&r);
        let w = write(&r.problems[0]);
        assert!(w.record.contains("$DOT @ATTACH"), "{}", w.record);
        let text = source("0.4", &header, DOT) + &w.record.lines().map(|l| format!("* {l}\n")).collect::<String>();
        let again = files(&MemoryProvider::new().with("main.sp", &text));
        clean(&again);
        assert!(again.model.levels[0].points.iter().find(|p| p.id == "%rt1").unwrap().junction);
    }
}

#[test]
fn target_alias_resolution_changes_the_input_digest_without_hashing_source_locations() {
    let body = format!("{DOT} $OTHER = @DEFINE(roles=[[\"p\",\"point\"]]) {{}};");
    let a = read(r#"@ALIAS("$alias","$DOT"); @SOLVER-CONFIG(junction="$alias")"#, &body);
    let b = read(r#"@ALIAS("$alias","$OTHER"); @SOLVER-CONFIG(junction="$alias")"#, &body);
    clean(&a);
    clean(&b);
    assert_ne!(rnl_runtime::problem::input_digest(&a.problems[0]), rnl_runtime::problem::input_digest(&b.problems[0]));
    let mut moved = a.problems[0].clone();
    moved.solver_config[0].source.as_mut().unwrap().line += 100;
    assert_eq!(rnl_runtime::problem::input_digest(&a.problems[0]), rnl_runtime::problem::input_digest(&moved));
}

#[test]
fn level_local_definition_is_written_and_reads_back() {
    for name in ["%dot", "%\"local dot\""] {
        let quoted = name.replace('"', "\"\"");
        let body = format!("{name} = @DEFINE(roles=[[\"p\",\"point\"]]) {{}}; @SOLVER-CONFIG(junction=\"{quoted}\"); %p=(0,0); {name} @ATTACH %p");
        let r = read("", &body);
        clean(&r);
        assert_eq!(r.problems[0].junction.definition.as_deref(), Some(name));
        assert!(r.model.levels[0].points.iter().find(|p| p.id == "%p").unwrap().junction);
        let w = write(&r.problems[0]);
        assert!(w.warnings.is_empty());
        assert!(w.record.contains(&format!("{name} @ATTACH %rt1")), "{}", w.record);
        let back = source("0.4", "", &body) + &w.record.lines().map(|l| format!("* {l}\n")).collect::<String>();
        let again = files(&MemoryProvider::new().with("main.sp", &back));
        clean(&again);
        assert!(again.model.levels[0].points.iter().find(|p| p.id == "%rt1").unwrap().junction);
    }
}

#[test]
fn local_junction_names_do_not_escape_their_subcircuit() {
    let text = source("0.4", "", "%dot = @DEFINE(roles=[[\"p\",\"point\"]]) {}; @SOLVER-CONFIG(junction=\"%dot\")")
        + ".subckt child a b\nR2 a b 1k\n* @RNL { @SOLVER-CONFIG(junction=\"%dot\")}\n.ends child\n"
        + ".subckt other a b\nR3 a b 1k\n* @RNL {%dot = @DEFINE(roles=[[\"p\",\"point\"]]) {}; @SOLVER-CONFIG(junction=\"%dot\")}\n.ends other\n";
    let r = files(&MemoryProvider::new().with("main.sp", &text));
    clean(&r);
    assert_eq!(r.problems[0].junction.definition.as_deref(), Some("%dot"));
    assert!(r.problems.iter().find(|p| p.name == "child").unwrap().junction.invalid.is_some());
    assert_eq!(r.problems.iter().find(|p| p.name == "other").unwrap().junction.definition.as_deref(), Some("%dot"));
}

#[test]
fn local_junction_names_are_not_accepted_in_headers_or_tool_overrides() {
    let body = "%dot = @DEFINE(roles=[[\"p\",\"point\"]]) {}";
    let r = read(r#"@SOLVER-CONFIG(junction="%dot")"#, body);
    clean(&r);
    assert!(r.problems[0].junction.invalid.is_some());
    let input = source("0.4", "", &format!("{body}; @SOLVER-CONFIG(junction=\"%dot\")"));
    let r = interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &input),
        &Options {
            std: false,
            problems: true,
            solver_config: rnl_runtime::solver_settings::parse_overrides("junction=\"%dot\"").unwrap(),
            ..Default::default()
        },
    )
    .unwrap();
    clean(&r);
    assert_eq!(r.problems[0].junction.definition.as_deref(), Some("%dot"));
    assert!(r.problems[0].junction.effective().invalid.is_some());
}

#[test]
fn a_junction_that_names_no_definition_leaves_the_earlier_one_in_effect() {
    // the level's value names nothing here, so the header's stands, with a warning of the level's
    let r = read(r#"@SOLVER-CONFIG(junction="$DOT")"#, &format!(r#"{DOT} @SOLVER-CONFIG(junction="$MISSING")"#));
    clean(&r);
    let junction = &r.problems[0].junction;
    assert_eq!(junction.definition.as_deref(), Some("$DOT"));
    assert_eq!(junction.invalid.as_ref().unwrap().value.str(), Some("$MISSING"));
    let w = write(&r.problems[0]);
    assert!(w.record.contains("$DOT @ATTACH %rt1"), "{}", w.record);
    assert_eq!(w.warnings.len(), 1);
    assert!(w.warnings[0].message.contains("it is ignored, and the earlier $DOT is used"), "{:?}", w.warnings);
    assert_eq!(w.warnings[0].loc.as_ref().unwrap().line, 4);
    // a later value that names a definition replaces the earlier one, with no warning
    let r = read(r#"@SOLVER-CONFIG(junction="$MISSING")"#, &format!(r#"{DOT} @SOLVER-CONFIG(junction="$DOT")"#));
    clean(&r);
    assert_eq!(r.problems[0].junction.definition.as_deref(), Some("$DOT"));
    assert!(r.problems[0].junction.invalid.is_none());
    // an empty value after a good one draws no dots, and with none that names one there are none
    let r = read(r#"@SOLVER-CONFIG(junction="$DOT")"#, &format!(r#"{DOT} @SOLVER-CONFIG(junction="")"#));
    assert!(r.problems[0].junction.definition.is_none() && r.problems[0].junction.invalid.is_none());
    let r = read(r#"@SOLVER-CONFIG(junction="$GONE")"#, r#"@SOLVER-CONFIG(junction="$MISSING")"#);
    assert!(r.problems[0].junction.definition.is_none());
    assert!(write(&r.problems[0]).warnings[0].message.contains("no dots are written"));
}

#[test]
fn local_junction_still_requires_a_definition_and_a_complete_name() {
    for (value, body) in [
        ("%p", "%p=(0,0)"),
        ("%dot", "%dot = @DEFINE(roles=[[\"n\",\"net\"]]) {}"),
        ("%dot::p", "%dot = @DEFINE(roles=[[\"p\",\"point\"]]) {}"),
        ("%dot{1}", "%dot = @DEFINE(roles=[[\"p\",\"point\"]]) {}"),
    ] {
        let r = read("", &format!("{body}; @SOLVER-CONFIG(junction=\"{value}\")"));
        clean(&r);
        assert!(r.problems[0].junction.invalid.is_some(), "{value}");
    }
}
#[test]
fn junction_write_has_stable_statement_ordinal_and_target() {
    let body = format!("{DOT} %a=(0,0)");
    let r = read(r#"@SOLVER-CONFIG(junction="$DOT")"#, &body);
    let w = write(&r.problems[0]);
    assert_eq!(w.junctions.len(), 1);
    assert_eq!(w.junctions[0].target, "%rt1");
    assert_eq!(&w.record[w.junctions[0].span.clone()], "$DOT @ATTACH %rt1");
}
