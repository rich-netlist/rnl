//! Group spacing is solver input, never a geometric fact (11.5).
use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::problem::input_digest;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn source(version: &str, body: &str) -> String {
    format!("* spacing\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a 0 1k\nR2 b 0 2k\n* @RNL {{ $S = @DEFINE-SYMBOL {{ (-1,0) @PIN(terminal=1); (1,0) @PIN(terminal=2) }}; # @SELECT-INSTANCE => @SET-SYMBOL $S; {body} }}\n")
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
fn spacings(r: &Interpretation) -> Vec<(Option<f64>, Option<f64>)> {
    r.problems.iter().flat_map(|p| &p.groups).map(|g| (g.spacing_x, g.spacing_y)).collect()
}

#[test]
fn signed_whole_spacings_reach_problem_and_model_without_facts() {
    let r = read("0.4", "%g={#R1 #R2} @GROUP(spacing_x=-2,spacing_y=3)");
    clean(&r);
    assert_eq!(spacings(&r), [(Some(-2.0), Some(3.0))]);
    let g = &r.model.levels[0].groups[0];
    assert_eq!((g.spacing_x, g.spacing_y), (Some(-2.0), Some(3.0)));
    assert!(r.problems[0].facts.is_empty());
}

#[test]
fn each_axis_is_optional_and_explicit_zero_is_preserved() {
    for (params, want) in [("", (None, None)), ("spacing_x=0", (Some(0.0), None)), ("spacing_y=-4", (None, Some(-4.0)))]
    {
        let r = read("0.4", &format!("{{#R1 #R2}} @GROUP({params})"));
        clean(&r);
        assert_eq!(spacings(&r), [want]);
    }
}

#[test]
fn fractional_spacing_fails_only_that_modification_and_retains_input() {
    let r = read("0.4", "%s={#R1 #R2}; %s @GROUP(spacing_x=0.5) => @GROUP(spacing_y=2)");
    let error = r.diagnostics().into_iter().find(|d| d.code == codes::PARAM_SPACING_NOT_WHOLE).unwrap();
    assert_eq!(error.severity, Severity::Error);
    assert!(error.loc.is_some());
    assert_eq!(spacings(&r), [(None, Some(2.0))]);
}

#[test]
fn wrong_types_and_positional_values_are_rejected() {
    for params in ["spacing_x=\"2\"", "spacing_y=true", "spacing_x=[2]", "2", "spacing_x=1,spacing_x=2", "other=2"] {
        let r = read("0.4", &format!("{{#R1 #R2}} @GROUP({params}); {{#R1 #R2}} @GROUP"));
        assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{params}: {:?}", r.diagnostics());
        assert_eq!(spacings(&r), [(None, None)]);
    }
}

#[test]
fn definition_parameters_and_rules_keep_spacing_with_final_members() {
    let r = read(
        "0.4",
        r#"$D = @DEFINE(roles=["{items}"],parameters=[["gap",-3]]) { ?items @GROUP(spacing_x=?gap) }; %s={#R1}; %u=%s @AS $D; #R2 @ADD-TO %s; # @SELECT-INSTANCE => @GROUP(spacing_y=2)"#,
    );
    clean(&r);
    assert!(spacings(&r).contains(&(Some(-3.0), None)));
    assert!(spacings(&r).contains(&(None, Some(2.0))));
    assert!(r.problems[0].groups.iter().all(|g| g.members.len() == 2));
}

#[test]
fn disabled_groups_keep_inspector_settings_but_are_not_solver_input() {
    let r = read("0.4", "%s={#R1 #R2} @GROUP(spacing_x=2); %s @PRIORITY(\"disabled\")");
    clean(&r);
    assert!(spacings(&r).is_empty());
    assert!(r.model.levels[0].groups[0].disabled);
    assert_eq!(r.model.levels[0].groups[0].spacing_x, Some(2.0));
}

#[test]
fn spacing_is_hashed_and_table_has_no_implicit_spacing() {
    let a = read("0.4", "{#R1 #R2} @GROUP");
    let b = read("0.4", "{#R1 #R2} @GROUP(spacing_x=0)");
    let c = read("0.4", "{#R1 #R2} @GROUP(spacing_x=1)");
    for r in [&a, &b, &c] {
        clean(r);
    }
    assert_ne!(input_digest(&a.problems[0]), input_digest(&b.problems[0]));
    assert_ne!(input_digest(&b.problems[0]), input_digest(&c.problems[0]));
    let r = read("0.4", "@TABLE layout: {#R1 #R2}");
    clean(&r);
    assert_eq!(spacings(&r), [(None, None)]);
}

#[test]
fn public_constraint_json_omits_unwritten_spacings() {
    for (body, written) in [("{#R1 #R2} @GROUP", false), ("{#R1 #R2} @GROUP(spacing_x=-1)", true)] {
        let r = read("0.4", body);
        clean(&r);
        let files = MemoryProvider::new().with("/w/main.sp", &source("0.4", body));
        let document = rnl_view::build(
            "/w/main.sp",
            &files,
            &rnl_view::BuildOptions {
                constraints: true,
                runtime: Options { std: false, ..Options::default() },
                ..Default::default()
            },
        )
        .unwrap();
        let data = serde_json::to_value(document.levels[0].constraints.as_ref().unwrap()).unwrap();
        assert_eq!(data["groups"][0].get("spacing_x").is_some(), written);
        assert!(data["groups"][0].get("spacing_y").is_none());
    }
}

#[test]
fn singleton_and_empty_groups_keep_settings_without_creating_positions() {
    for body in ["#R1 @GROUP(spacing_x=2)", "{} @GROUP(spacing_x=2)"] {
        let r = read("0.4", body);
        clean(&r);
        assert_eq!(spacings(&r), [(Some(2.0), None)]);
        assert!(r.problems[0].groups[0].members.len() < 2);
        assert!(r.problems[0].facts.is_empty());
    }
}

#[test]
fn snapshot_model_carries_spacing_and_omits_absent_axes() {
    let files = MemoryProvider::new().with("/w/main.sp", &source("0.4", "%g={#R1 #R2} @GROUP(spacing_y=-2)"));
    let snapshot = rnl_view::snapshot(
        "/w/main.sp",
        &files,
        &rnl_view::BuildOptions { runtime: Options { std: false, ..Options::default() }, ..Default::default() },
        rnl_view::Parts { drawing: false, host: false, model: true },
    )
    .unwrap();
    let value = serde_json::to_value(snapshot).unwrap();
    let group = &value["levels"][0]["model"]["groups"][0];
    assert_eq!(group["spacing_y"], -2.0, "{value}");
    assert!(group.get("spacing_x").is_none());
}

#[test]
fn spacing_words_can_be_aliased() {
    let aliases = r#"; @ALIAS("gap_x","spacing_x"); @ALIAS("gap_y","spacing_y")"#;
    let text = source("0.4", "{#R1 #R2} @GROUP(gap_x=1,gap_y=2)")
        .replace("@VERSION(\"0.4\")", &format!("@VERSION(\"0.4\"){aliases}"));
    let r = read_files(&MemoryProvider::new().with("main.sp", &text));
    clean(&r);
    assert_eq!(spacings(&r), [(Some(1.), Some(2.))]);
}

#[test]
fn spacing_aliases_of_an_alias_package_reach_the_file() {
    let uses = r#"; @USE-ALIASES(name="alias"); @USE-PACKAGE(name="ordinary")"#;
    let text = source("0.4", "{#R1 #R2} @GROUP(gap_x=1,gap_y=2)")
        .replace("@VERSION(\"0.4\")", &format!("@VERSION(\"0.4\"){uses}"));
    let a = "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"alias\",version=\"1.0\"); @ALIAS(\"gap_x\",\"spacing_x\"); @ALIAS(\"gap_y\",\"spacing_y\") }\n";
    let p = "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"ordinary\",version=\"1.0\"); @ALIAS(\"gap\",\"spacing_y\") }\n@RNL { $D = @DEFINE(roles=[\"p\"]) {} }\n";
    let mut f = MemoryProvider::new().with("main.sp", &text).with("pkg/a.rnl", a).with("pkg/p.rnl", p);
    f.package_dirs.push("pkg".into());
    let r = read_files(&f);
    clean(&r);
    assert_eq!(spacings(&r), [(Some(1.), Some(2.))]);
}
