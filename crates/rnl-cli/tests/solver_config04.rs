//! Effective literal solver settings and context boundaries (14.10).
use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::interp::obj::Lit;
use rnl_runtime::problem::input_digest;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::collections::BTreeMap;
use std::path::Path;

fn read_files(files: &MemoryProvider) -> Interpretation {
    interpret_file(Path::new("main.sp"), None, files, &Options { std: false, problems: true, ..Default::default() })
        .unwrap()
}
fn source(version: &str, header: &str, body: &str) -> String {
    format!(
        "* settings\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {header} }}\nR1 a 0 1k\n* @RNL {{ {body} }}\n"
    )
}
fn read(header: &str, body: &str) -> Interpretation {
    read_files(&MemoryProvider::new().with("main.sp", &source("0.4", header, body)))
}
fn clean(r: &Interpretation) {
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
}
fn settings(r: &Interpretation, index: usize) -> BTreeMap<String, String> {
    r.problems[index].solver_config.iter().map(|s| (s.key.clone(), s.value.show())).collect()
}
fn package(name: &str, header: &str, body: &str) -> String {
    format!("@RNL(type=\"header\") {{ @VERSION(\"0.4\"); @PACKAGE(name=\"{name}\",version=\"1.0\"); {header} }}\n@RNL {{ {body} }}\n")
}
fn with_packages(main: &str, packages: &[(&str, String)]) -> MemoryProvider {
    let mut files = MemoryProvider::new().with("main.sp", main);
    files.package_dirs.push("pkg".into());
    for (name, text) in packages {
        files.insert(&format!("pkg/{name}.rnl"), text);
    }
    files
}

#[test]
fn open_keys_pass_unchanged_and_the_core_judges_only_the_keys_the_standard_names() {
    let r = read(
        "@SOLVER-CONFIG(spacing_x=\"solver decides\",junction=3,custom=[1,\"two\",true],empty=[])",
        "@SOLVER-CONFIG(other=false, spacing_y=1.5)",
    );
    // a value of a named key that the core does not take is an error for that key alone, and is
    // not passed on; the other keys of the same statement are
    let all = r.diagnostics();
    let mut errors: Vec<_> = all.iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect();
    errors.sort();
    assert_eq!(
        errors,
        [
            codes::SOLVER_CONFIG_SPACING_NOT_WHOLE,
            codes::SOLVER_CONFIG_SPACING_NOT_WHOLE,
            codes::SOLVER_CONFIG_JUNCTION_NOT_TAKEN
        ]
    );
    let s = settings(&r, 0);
    assert!(!s.contains_key("spacing_x") && !s.contains_key("spacing_y") && !s.contains_key("junction"));
    assert_eq!(s["custom"], "[1, \"two\", true]");
    assert_eq!(s["other"], "false");
    assert_eq!(s["empty"], "[]");
    assert!(r.problems[0].facts.is_empty());
    assert!(r.problems[0].solver_config.iter().all(|s| s.source.is_some()));
}

#[test]
fn overrides_are_per_key_and_later_level_records_win() {
    let r = read("@SOLVER-CONFIG(a=1,b=2); @SOLVER-CONFIG(a=3)", "@SOLVER-CONFIG(b=4); @SOLVER-CONFIG(c=5)");
    clean(&r);
    assert_eq!(
        settings(&r, 0),
        BTreeMap::from([("a".into(), "3".into()), ("b".into(), "4".into()), ("c".into(), "5".into())])
    );
}

#[test]
fn failed_statement_does_not_partially_override_other_keys() {
    for params in ["a=9,b=[[1]]", "a=9,b=1,b=2", "2"] {
        for header in [false, true] {
            let invalid = format!("@SOLVER-CONFIG({params})");
            let r = if header {
                read(&format!("@SOLVER-CONFIG(a=1); {invalid}"), "@SOLVER-CONFIG(c=3)")
            } else {
                read("@SOLVER-CONFIG(a=1)", &format!("{invalid}; @SOLVER-CONFIG(c=3)"))
            };
            assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
            assert_eq!(settings(&r, 0)["a"], "1");
            assert_eq!(settings(&r, 0)["c"], "3");
            assert!(!settings(&r, 0).contains_key("b"));
        }
    }
}

#[test]
fn package_headers_precede_file_header_and_level_records() {
    let main = source("0.4", "@USE-PACKAGE(name=\"p\"); @SOLVER-CONFIG(b=20)", "@SOLVER-CONFIG(c=30)");
    let files = with_packages(
        &main,
        &[
            ("p", package("p", "@USE-PACKAGE(name=\"q\"); @SOLVER-CONFIG(a=2,b=2,c=2)", "")),
            ("q", package("q", "@SOLVER-CONFIG(a=1,b=1,c=1,d=1)", "")),
        ],
    );
    let r = read_files(&files);
    clean(&r);
    let s = settings(&r, 0);
    assert_eq!((s["a"].as_str(), s["b"].as_str(), s["c"].as_str(), s["d"].as_str()), ("2", "20", "30", "1"));
    // every value of a key is given, in that order: q's, p's, the header's, the level's
    let of = |key: &str| -> Vec<(String, String)> {
        let values = r.problems[0].solver_config.iter().filter(|s| s.key == key);
        values.map(|s| (s.package.clone(), s.value.show())).collect()
    };
    let value = |package: &str, value: &str| (package.to_string(), value.to_string());
    assert_eq!(of("a"), [value("q", "1"), value("p", "2")]);
    assert_eq!(of("c"), [value("q", "1"), value("p", "2"), value("", "30")]);
    assert_eq!(of("d"), [value("q", "1")]);
}

#[test]
fn a_refused_value_leaves_the_one_before_it_and_the_other_keys_apply() {
    // the package's grid=2 and the level's grid="x", with a good beam in the same statement
    let main = source("0.4", "@USE-PACKAGE(name=\"p\")", "@SOLVER-CONFIG(grid=\"x\", beam=6)");
    let r = read_files(&with_packages(&main, &[("p", package("p", "@SOLVER-CONFIG(grid=2)", ""))]));
    clean(&r);
    let grid: Vec<_> = r.problems[0].solver_config.iter().filter(|s| s.key == "grid").map(|s| s.value.show()).collect();
    assert_eq!(grid, ["2", "\"x\""]);
    let (o, warnings) = rnl_solver::SolverOptions::default().for_problem(&r.problems[0]);
    assert_eq!((o.grid, o.beam), (Some(2.0), 6));
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].starts_with("grid must be a finite number; grid=\"x\" at main.sp:4 is left out, and grid=2 at"),
        "{warnings:?}"
    );
    // an earlier value is input too, as it stands when a later one is refused
    let other = read_files(&with_packages(&main, &[("p", package("p", "@SOLVER-CONFIG(grid=3)", ""))]));
    assert_ne!(input_digest(&r.problems[0]), input_digest(&other.problems[0]));
}

#[test]
fn a_package_used_only_by_language_headers_contributes_no_settings() {
    let main=source("0.4","","").replace("R1 a", "* @RNL(type=\"header\",language=\"spice\") { @USE-PACKAGE(name=\"p\") }\n* @RNL(type=\"header\",language=\"spectre\") { @USE-PACKAGE(name=\"p\") }\nR1 a");
    let packages = [
        ("p", package("p", "@USE-PACKAGE(name=\"q\"); @SOLVER-CONFIG(a=1)", "")),
        ("q", package("q", "@SOLVER-CONFIG(b=2)", "")),
    ];
    let r = read_files(&with_packages(&main, &packages));
    clean(&r);
    assert!(settings(&r, 0).is_empty());
    let main = main.replace("@VERSION(\"0.4\");", "@VERSION(\"0.4\"); @USE-PACKAGE(name=\"p\");");
    let r = read_files(&with_packages(&main, &packages));
    clean(&r);
    assert_eq!(settings(&r, 0).len(), 2);
}

#[test]
fn language_and_generated_records_warn_and_have_no_effect() {
    let mut text = source("0.4", "@SOLVER-CONFIG(a=1)", "@SOLVER-CONFIG(b=2)");
    text = text.replace("R1 a", "* @RNL(type=\"header\",language=\"spice\") { @SOLVER-CONFIG(a=9) }\nR1 a");
    text.push_str(
        "* @RNL(language=\"spice\") { @SOLVER-CONFIG(b=9) }\n* @RNL(type=\"generated\") { @SOLVER-CONFIG(c=9) }\n",
    );
    let r = read_files(&MemoryProvider::new().with("main.sp", &text));
    clean(&r);
    assert_eq!(settings(&r, 0).len(), 2);
    assert_eq!(settings(&r, 0)["a"], "1");
    assert_eq!(settings(&r, 0)["b"], "2");
    assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::SOLVER_CONFIG_CONTEXT).count(), 3);
}

#[test]
fn package_content_and_inline_package_settings_do_not_escape() {
    let main = source(
        "0.4",
        "@USE-PACKAGE(name=\"p\")",
        "@USE-PACKAGE(name=\"inline\") { @SOLVER-CONFIG(c=3) }; @SOLVER-CONFIG(d=4)",
    );
    let r = read_files(&with_packages(&main, &[("p", package("p", "@SOLVER-CONFIG(a=1)", "@SOLVER-CONFIG(b=2)"))]));
    clean(&r);
    let s = settings(&r, 0);
    assert_eq!(s.len(), 2);
    assert_eq!(s["a"], "1");
    assert_eq!(s["d"], "4");
}

#[test]
fn file_defaults_apply_to_its_subcircuits_but_level_settings_do_not_inherit() {
    let text = source("0.4", "@SOLVER-CONFIG(a=1)", "@SOLVER-CONFIG(a=2)")
        + ".subckt child x\nR2 x 0 1k\n* @RNL { @SOLVER-CONFIG(b=3) }\n.ends\n";
    let r = read_files(&MemoryProvider::new().with("main.sp", &text));
    clean(&r);
    assert_eq!(settings(&r, 0)["a"], "2");
    let child = r.problems.iter().position(|p| p.name == "child").unwrap();
    assert_eq!(settings(&r, child)["a"], "1");
    assert_eq!(settings(&r, child)["b"], "3");
}

#[test]
fn every_value_of_a_key_affects_the_input_digest_not_the_order_of_the_keys() {
    let a = read("@SOLVER-CONFIG(a=1,b=2)", "@SOLVER-CONFIG(a=3)");
    let b = read("@SOLVER-CONFIG(b=2,a=3)", "");
    let c = read("@SOLVER-CONFIG(b=2,a=4)", "");
    let d = read("@SOLVER-CONFIG(b=2,a=1)", "@SOLVER-CONFIG(a=3)");
    for r in [&a, &b, &c, &d] {
        clean(r);
    }
    // a=1 is input too, as it stands when the level's a=3 is refused
    assert_ne!(input_digest(&a.problems[0]), input_digest(&b.problems[0]));
    assert_ne!(input_digest(&b.problems[0]), input_digest(&c.problems[0]));
    // the order in which the keys are written, and where, are not
    assert_eq!(input_digest(&a.problems[0]), input_digest(&d.problems[0]));
}

#[test]
fn settings_keep_string_source_and_make_no_output_object() {
    let r = read("@SOLVER-CONFIG(junction=\"$CUSTOM\")", "$CUSTOM = @DEFINE(roles=[[\"p\",\"point\"]]) {}");
    clean(&r);
    assert!(matches!(&r.problems[0].solver_config[0].value,Lit::Str(s) if s.unit.is_some()));
    let r = read("", "#R1 @SOLVER-CONFIG(a=1)");
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
    assert!(settings(&r, 0).is_empty());
}

#[test]
fn released_versions_do_not_gain_solver_config() {
    for version in ["0.2", "0.3"] {
        let r = read_files(
            &MemoryProvider::new().with("main.sp", &source(version, "@SOLVER-CONFIG(a=1)", "@SOLVER-CONFIG(b=2)")),
        );
        assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
        assert!(settings(&r, 0).is_empty());
        let unknown = r.diagnostics().into_iter().find(|d| d.code == codes::KEYWORD_UNKNOWN).unwrap();
        assert_eq!(unknown.message, "unknown keyword @SOLVER-CONFIG (16.4)");
    }
}

#[test]
fn standard_key_aliases_work_and_duplicate_alias_spellings_fail() {
    let r = read("@ALIAS(\"gap\",\"spacing_x\")", "@SOLVER-CONFIG(gap=-2)");
    clean(&r);
    assert_eq!(settings(&r, 0)["spacing_x"], "-2");
    assert!(!settings(&r, 0).contains_key("gap"));
    let r = read("@ALIAS(\"gap\",\"spacing_x\"); @SOLVER-CONFIG(spacing_x=1)", "@SOLVER-CONFIG(gap=2,spacing_x=3)");
    assert!(r.diagnostics().iter().any(|d| d.code == codes::PARAM_WRITTEN_TWICE));
    assert_eq!(settings(&r, 0)["spacing_x"], "1");
}

#[test]
fn public_problem_and_viewer_constraints_keep_typed_values_and_sources() {
    let header = "@SOLVER-CONFIG(spacing_x=-2,layout=\"lines\",weights=[1,true,\"two\"])";
    let r = read(header, "");
    clean(&r);
    let problem = rnl_cli::problem::level(&r.problems[0], "");
    let files = MemoryProvider::new().with("/w/main.sp", &source("0.4", header, ""));
    let doc = rnl_view::build(
        "/w/main.sp",
        &files,
        &rnl_view::BuildOptions {
            constraints: true,
            runtime: Options { std: false, ..Default::default() },
            ..Default::default()
        },
    )
    .unwrap();
    let view = serde_json::to_value(doc.levels[0].constraints.as_ref().unwrap()).unwrap();
    for data in [&problem, &view] {
        let values = data["solver_config"].as_array().unwrap();
        assert_eq!(values[0]["key"], "layout");
        assert_eq!(values[0]["value"], "lines");
        assert_eq!(values[1]["value"], -2.0);
        assert_eq!(values[2]["value"], serde_json::json!([1.0, true, "two"]));
        assert_eq!(values[0]["source"]["line"], 2);
        assert!(values[0]["source"]["col"].as_u64().unwrap() > 1);
        assert_eq!(values[0]["package"], "");
    }
    for version in ["0.2", "0.3", "0.4"] {
        let files = MemoryProvider::new().with("/w/main.sp", &source(version, "", ""));
        let doc = rnl_view::build(
            "/w/main.sp",
            &files,
            &rnl_view::BuildOptions {
                constraints: true,
                runtime: Options { std: false, ..Default::default() },
                ..Default::default()
            },
        )
        .unwrap();
        let view = serde_json::to_value(doc.levels[0].constraints.as_ref().unwrap()).unwrap();
        assert!(view.get("solver_config").is_none());
        let r = read_files(&MemoryProvider::new().with("main.sp", &source(version, "", "")));
        assert!(rnl_cli::problem::level(&r.problems[0], "").get("solver_config").is_none());
    }
}

#[test]
fn included_files_keep_their_own_header_defaults_and_scoped_records_override() {
    let main="* settings\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @SOLVER-CONFIG(a=1,b=1) }\n.include child.sp\n* @RNL(scope=\"child\") { @SOLVER-CONFIG(b=3) }\n";
    let child="* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @SOLVER-CONFIG(a=2,b=2) }\n.subckt child a\nR1 a 0 1k\n.ends\n";
    let r = read_files(&MemoryProvider::new().with("main.sp", main).with("child.sp", child));
    clean(&r);
    let p = r.problems.iter().position(|p| p.name == "child").unwrap();
    assert_eq!(settings(&r, p), BTreeMap::from([("a".into(), "2".into()), ("b".into(), "3".into())]));
}

#[test]
fn qualified_transitive_edges_do_not_supply_defaults_but_another_path_can() {
    let p = package("p", "@SOLVER-CONFIG(a=1)", "").replace("@RNL {  }\n", "")
        + "@RNL(type=\"header\",language=\"spice\") { @USE-PACKAGE(name=\"q\") }\n";
    let packages = [("p", p), ("q", package("q", "@SOLVER-CONFIG(b=2)", ""))];
    for direct in [false, true] {
        let main = source(
            "0.4",
            if direct { "@USE-PACKAGE(name=\"p\"); @USE-PACKAGE(name=\"q\")" } else { "@USE-PACKAGE(name=\"p\")" },
            "",
        );
        let r = read_files(&with_packages(&main, &packages));
        clean(&r);
        assert_eq!(settings(&r, 0)["a"], "1");
        assert_eq!(settings(&r, 0).contains_key("b"), direct);
    }
}

#[test]
fn custom_keys_are_not_aliases_and_quoted_standard_keys_remain_literal() {
    let r = read(
        "@ALIAS(\"layout\",\"spacing_x\"); @ALIAS(\"gap\",\"spacing_y\")",
        "@SOLVER-CONFIG(\"layout\"=\"custom\",\"gap\"=2)",
    );
    clean(&r);
    let values = settings(&r, 0);
    assert_eq!(values["layout"], "\"custom\"");
    assert_eq!(values["gap"], "2");
    assert!(!values.contains_key("spacing_x"));
    assert!(!values.contains_key("spacing_y"));
}

#[test]
fn failed_usage_bodies_cannot_override_settings() {
    let body = r#"$S = @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S; $D = @DEFINE(roles=[["r","instance"]]) { @SOLVER-CONFIG(a=9) }; #R1::p @AS $D"#;
    let r = read("@SOLVER-CONFIG(a=1)", body);
    assert_eq!(settings(&r, 0)["a"], "1", "{:?}", r.diagnostics());
}

#[test]
fn literal_definition_parameters_are_read_at_the_usage() {
    let r = read(
        "@SOLVER-CONFIG(a=1)",
        r#"$D = @DEFINE(roles=[],parameters=["amount"]) { @SOLVER-CONFIG(a=?amount) }; $D @ATTACH(amount=4)"#,
    );
    clean(&r);
    assert_eq!(settings(&r, 0)["a"], "4");
    let r = read(
        "@SOLVER-CONFIG(a=1)",
        r#"$D = @DEFINE(roles=[],parameters=["amount"]) { @SOLVER-CONFIG(a=?amount) }; $D @ATTACH"#,
    );
    clean(&r);
    assert_eq!(settings(&r, 0)["a"], "1");
}

#[test]
fn tool_overrides_are_explicit_and_change_only_the_effective_hash_input() {
    let r = read("@SOLVER-CONFIG(a=1,b=2)", "");
    clean(&r);
    let mut p = r.problems[0].clone();
    p.solver_config = rnl_runtime::solver_settings::with_overrides(&p.solver_config, "b=3").unwrap();
    assert_eq!(p.solver_config[0].value, Lit::Num(1.0));
    assert!(p.solver_config[0].source.is_some());
    assert!(p.solver_config[1].source.is_none());
    assert_ne!(input_digest(&p), input_digest(&r.problems[0]));
    let other = read("@SOLVER-CONFIG(a=1,b=3)", "");
    assert_eq!(input_digest(&p), input_digest(&other.problems[0]));
}

#[test]
fn new_settings_words_do_not_expand_the_released_alias_vocabulary() {
    for version in ["0.2", "0.3", "0.4"] {
        let files = MemoryProvider::new().with(
            "main.sp",
            &source(
                version,
                r#"@ALIAS("@SC","@SOLVER-CONFIG"); @ALIAS("dot","junction")"#,
                if version == "0.4" {
                    r#"$CUSTOM = @DEFINE(roles=["p"]) {}; @SC(dot="$CUSTOM")"#
                } else {
                    r#"@SC(dot="$CUSTOM")"#
                },
            ),
        );
        let r = read_files(&files);
        if version == "0.4" {
            clean(&r);
            assert_eq!(settings(&r, 0)["junction"], "\"$CUSTOM\"");
        } else {
            assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::ALIAS_ORIGINAL_NOT_STANDARD).count(), 2);
            assert!(r
                .diagnostics()
                .iter()
                .any(|d| d.code == codes::KEYWORD_UNKNOWN && d.message == "unknown keyword @SC (16.4)"));
            assert!(settings(&r, 0).is_empty());
        }
    }
}

#[test]
fn alias_packages_and_ordinary_packages_check_new_words_in_their_version() {
    for version in ["0.2", "0.3", "0.4"] {
        let main = source(version, r#"@USE-ALIASES(name="alias"); @USE-PACKAGE(name="ordinary")"#, "@SC(dot=\"$D\")");
        let a=format!("@RNL(type=\"header\") {{ @VERSION(\"{version}\"); @PACKAGE(name=\"alias\",version=\"1.0\"); @ALIAS(\"@SC\",\"@SOLVER-CONFIG\"); @ALIAS(\"dot\",\"junction\") }}\n");
        let p=format!("@RNL(type=\"header\") {{ @VERSION(\"{version}\"); @PACKAGE(name=\"ordinary\",version=\"1.0\"); @ALIAS(\"dot\",\"junction\") }}\n@RNL {{ $D = @DEFINE(roles=[\"p\"]) {{}} }}\n");
        let r = read_files(&with_packages(&main, &[("alias", a), ("ordinary", p)]));
        if version == "0.4" {
            clean(&r);
            assert_eq!(settings(&r, 0)["junction"], "\"$D\"");
        } else {
            assert_eq!(
                r.diagnostics().iter().filter(|d| d.code == codes::USE_ALIASES_ORIGINAL_NOT_IN_VERSION).count(),
                2
            );
            assert!(r.diagnostics().iter().any(|d| d.code == codes::ALIAS_ORIGINAL_NOT_STANDARD));
            assert!(settings(&r, 0).is_empty());
        }
    }
}
