//! Scene objects enter ordinary definition bodies through roles (6.9).
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

const SCENE: Code = Code::new(3417, "define-body-scene-ref");
const TWO: &str = "$D @ATTACH => @PLACE(0,0); $D @ATTACH => @PLACE(10,0)";

fn read(version: &str, setup: &str, body: &str, after: &str) -> Interpretation {
    let source=format!("* scene references\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {setup} $D = @DEFINE(roles=[]) {{ {body}; (5,0) @CIRCLE(radius=2) }}; {after} }}\n");
    rnl_cli::interpret_file(
        Path::new("scenes.sp"),
        None,
        &MemoryProvider::new().with("scenes.sp", &source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap()
}
fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}

#[test]
fn external_points_are_rejected_once_in_used_and_unused_definitions() {
    for (setup, body, later) in [
        ("%p=(1,2);", "%p @CIRCLE(radius=1)", ""),
        ("", "%p @CIRCLE(radius=1)", "%p=(1,2);"),
        ("%p=(1,2); %q=%p;", "%q @CIRCLE(radius=1)", ""),
        ("%p=(1,2);", "%p::position @CIRCLE(radius=1)", ""),
        ("%p=(1,2); %s={%p};", "%s @CIRCLE(radius=1)", ""),
        ("%p=(1,2); %s={%p};", "%s{1} @CIRCLE(radius=1)", ""),
    ] {
        for calls in ["", TWO] {
            let r = read("0.4", setup, body, &format!("{calls}; {later}"));
            assert_eq!(errors(&r), [SCENE], "{setup} / {body}: {:?}", r.diagnostics());
            assert!(r.diagnostics().into_iter().find(|d| d.code == SCENE).unwrap().origin.is_empty());
            assert_eq!(r.model.levels[0].shapes.len(), if calls.is_empty() { 0 } else { 2 });
        }
    }
}

#[test]
fn invalid_scene_reference_omits_earlier_pipe_creations() {
    let r = read("0.4", "%p=(1,2);", "(0,0) @CIRCLE(radius=1) => @SET-STYLE %p", TWO);
    assert_eq!(errors(&r), [SCENE], "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 2);
}

#[test]
fn local_points_private_definitions_and_selection_rules_remain_valid() {
    for (setup, body) in [
        ("", "%p=(1,2); %p @CIRCLE(radius=1)"),
        ("%helper= @DEFINE(roles=[]) {};", "%helper @ATTACH"),
        ("$selected=# @SELECT-INSTANCE;", "$selected @ORIENT(\"R0\")"),
        ("$A= @DEFINE(roles=[]) {}; $defs={$A};", "$defs @ATTACH"),
        ("%p=(1,2);", "{%p=(2,3); %p @CIRCLE(radius=1)}"),
    ] {
        for calls in ["", TWO] {
            let r = read("0.4", setup, body, calls);
            assert!(errors(&r).is_empty(), "{setup} / {body}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn released_versions_still_allow_same_level_scene_references() {
    for version in ["0.2", "0.3"] {
        let r = read(version, "%p=(1,2);", "%p @CIRCLE(radius=1)", "$D @ATTACH; $D @ATTACH");
        assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].shapes.len(), 4);
    }
}

#[test]
fn private_package_points_are_not_visible_from_bodies() {
    let files = MemoryProvider::new()
        .with("p.rnl", "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"p\",version=\"1.0\") }\n@RNL { %q=%p; $D = @DEFINE(roles=[]) { %q @CIRCLE(radius=1) }; %p=(1,2) }\n")
        .with("main.sp", "* private point\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"p\",path=\"p.rnl\") }\n* @RNL { %q = @DEFINE(roles=[]) {}; $D @ATTACH; $D @ATTACH }\n");
    for path in ["p.rnl", "main.sp"] {
        let r = rnl_cli::interpret_file(Path::new(path), None, &files, &Options { std: false, ..Default::default() })
            .unwrap();
        assert_eq!(errors(&r), [SCENE], "{path}: {:?}", r.diagnostics());
        let diagnostic = r.diagnostics().into_iter().find(|d| d.code == SCENE).unwrap();
        assert_eq!(diagnostic.in_package, "p");
        assert!(diagnostic.origin.is_empty());
        assert!(diagnostic.loc.as_ref().unwrap().file.ends_with("p.rnl"));
    }
}

#[test]
fn scene_object_kinds_and_aliases_are_rejected_without_forcing_expansion() {
    for (setup, body) in [
        ("%u = $C @ATTACH; $C = @DEFINE(roles=[]) {};", "%u @PLACE(2,0)"),
        ("%g=(0,0) @CIRCLE(radius=1);", "%g"),
        ("%t=(0,0) @TEXT(\"label\");", "%t @PLACE(2,0)"),
        ("%w = &a @WIRE {(0,0) (1,0)};", "%w"),
        ("%i = #R1;", "%i @PLACE(1,2)"),
        ("%t = #R1:1;", "%t"),
        ("%n = &a;", "%n"),
        ("%p=(1,2) => %q;", "%q @CIRCLE(radius=1)"),
    ] {
        for calls in ["", TWO] {
            let r = read("0.4", setup, body, calls);
            assert_eq!(errors(&r), [SCENE], "{setup} / {body}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn nested_failed_members_do_not_discard_sibling_members() {
    let r = read("0.4", "%p=(1,2);", "{(0,0) @CIRCLE(radius=1) => @SET-STYLE %p; (2,0) @CIRCLE(radius=1)}", TWO);
    assert_eq!(errors(&r), [SCENE], "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 4);
}

#[test]
fn included_bodies_keep_their_own_language_version() {
    for version in ["0.3", "0.4"] {
        let files=MemoryProvider::new()
            .with("main.sp","* includes\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include \"defs.sp\"\n")
            .with("defs.sp",&format!("* definitions\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n* @RNL {{ %p=(1,2); $D = @DEFINE(roles=[]) {{ %p @CIRCLE(radius=1) }} }}\n"));
        let r =
            rnl_cli::interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Default::default() })
                .unwrap();
        assert_eq!(errors(&r), if version == "0.4" { vec![SCENE] } else { vec![] }, "{version}: {:?}", r.diagnostics());
    }
}

#[test]
fn external_percent_collections_are_forbidden_without_reading_their_members() {
    for (setup, later) in [
        ("%s={};", ""),
        ("$C = @DEFINE(roles=[]) {}; %s={$C};", ""),
        ("%p=(1,2); %s={%p};", "%p @REMOVE-FROM %s;"),
        ("%s=# @SELECT-INSTANCE;", ""),
        ("", "%s=# @SELECT-INSTANCE;"),
        ("%s={};", "#R1 @ADD-TO %s;"),
    ] {
        for calls in ["", TWO] {
            let r = read("0.4", setup, "%s", &format!("{later}{calls}"));
            assert_eq!(errors(&r), [SCENE], "{setup}/{later}/{calls}: {:?}", r.diagnostics());
            let d = r.diagnostics().into_iter().find(|d| d.code == SCENE).unwrap();
            assert!(d.origin.is_empty());
            assert_eq!(r.model.levels[0].shapes.len(), if calls.is_empty() { 0 } else { 2 });
        }
    }
}

#[test]
fn dollar_collections_are_evaluated_in_each_usage_level() {
    for (set, counts) in [
        ("# @SELECT-INSTANCE(name=\"R*\")", vec![1, 0]),
        ("# @SELECT-INSTANCE(name=\"R*\",recursive=true)", vec![1, 1]),
        ("{#R1}", vec![1, 0]),
    ] {
        let source = format!(
            r#"* shared sets
* @RNL(type="header") {{ @VERSION("0.4") }}
.subckt child p q
R2 p q 1k
* @RNL {{ $D @ATTACH }}
.ends child
R1 a b 1k
X1 a b child
* @RNL {{ $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1) }}; # @SELECT-INSTANCE(name="R*",recursive=true) => @SET-SYMBOL $S; $items={set}; $D = @DEFINE(roles=[]) {{ $items @TEXT("mark",key="mark") }}; $D @ATTACH }}
"#
        );
        let r = rnl_cli::interpret_file(
            Path::new("main.sp"),
            None,
            &MemoryProvider::new().with("main.sp", &source),
            &Options { std: false, ..Options::solver() },
        )
        .unwrap();
        assert!(errors(&r).is_empty(), "{set}: {:?}", r.diagnostics());
        let actual: Vec<_> = r
            .problems
            .iter()
            .map(|p| {
                p.objects.iter().filter(|o| o.text.as_ref().is_some_and(|t| t.key.as_deref() == Some("mark"))).count()
            })
            .collect();
        assert_eq!(actual, counts, "{set}: {:?}", r.diagnostics());
    }
}

#[test]
fn scene_checks_use_the_first_valid_binding() {
    for (value, body, forbidden) in
        [("(1,2)", "%x @CIRCLE(radius=1)", true), ("{}", "%x", true), ("@DEFINE(roles=[]) {}", "%x @ATTACH", false)]
    {
        for calls in ["", TWO] {
            let r = read("0.4", &format!("%x=%missing; %x = {value};"), body, calls);
            assert_eq!(errors(&r).len(), if forbidden { 2 } else { 1 }, "{value}: {:?}", r.diagnostics());
            assert!(errors(&r).contains(&codes::REF_NOT_FOUND));
            assert_eq!(errors(&r).contains(&SCENE), forbidden, "{value}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn symbol_bodies_reject_external_points_and_collections() {
    for (setup, reference) in [("%p=(1,2);", "%p"), ("%p=(1,2);", "%p @CIRCLE(radius=1)"), ("%s={};", "%s")] {
        for used in [false, true] {
            let r = read(
                "0.4",
                &format!("{setup} $S = @DEFINE-SYMBOL {{ {reference}; (0,0) @PIN(terminal=1) }};"),
                "",
                if used { "#R1 @SET-SYMBOL $S" } else { "" },
            );
            assert_eq!(errors(&r), [SCENE], "{setup}/{reference}: {:?}", r.diagnostics());
            assert!(r.diagnostics().into_iter().find(|d| d.code == SCENE).unwrap().origin.is_empty());
        }
    }
}

#[test]
fn symbol_local_points_and_released_external_points_remain_allowed() {
    let r = read("0.4", "$S = @DEFINE-SYMBOL { %p=(1,2); %p; %p @CIRCLE(radius=1) };", "", "#R1 @SET-SYMBOL $S");
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    for version in ["0.2", "0.3"] {
        let r = read(version, "%p=(1,2); $S = @DEFINE-SYMBOL { %p };", "", "#R1 @SET-SYMBOL $S");
        assert!(errors(&r).is_empty(), "{version}: {:?}", r.diagnostics());
    }
}

#[test]
fn a_nested_symbol_checks_external_scene_names_at_the_ordinary_definition() {
    for calls in ["", TWO] {
        let r = read("0.4", "%p=(1,2);", "%S = @DEFINE-SYMBOL { %p; (0,0) @PIN(terminal=1) }", calls);
        assert_eq!(errors(&r), [SCENE], "{calls}: {:?}", r.diagnostics());
        assert!(r.diagnostics().into_iter().find(|d| d.code == SCENE).unwrap().origin.is_empty());
    }
}

#[test]
fn nested_declarations_keep_their_local_scene_names() {
    for body in [
        "%S = @DEFINE-SYMBOL { %p=(2,3); %p @CIRCLE(radius=1) }",
        "%Inner = @DEFINE(roles=[]) { %p=(2,3); %p @CIRCLE(radius=1) }; %Inner @ATTACH",
    ] {
        for calls in ["", TWO] {
            let r = read("0.4", "%p=(1,2);", body, calls);
            assert!(errors(&r).is_empty(), "{body}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn nested_ordinary_definitions_report_external_names_only_once() {
    for calls in ["", TWO] {
        let r = read("0.4", "%s={};", "%Inner = @DEFINE(roles=[]) { %s }; %Inner @ATTACH", calls);
        assert_eq!(errors(&r), [SCENE], "{calls}: {:?}", r.diagnostics());
        assert!(r.diagnostics().into_iter().find(|d| d.code == SCENE).unwrap().origin.is_empty());
    }
}

#[test]
fn a_nested_definition_hides_its_own_name_without_capturing_outer_locals() {
    for inner in ["@DEFINE-SYMBOL { %p }", "@DEFINE(roles=[]) { %p @CIRCLE(radius=1) }"] {
        for calls in ["", TWO] {
            let r = read("0.4", "%p=(1,2);", &format!("%p = {inner}"), calls);
            assert_eq!(errors(&r), [SCENE], "{inner}/{calls}: {:?}", r.diagnostics());
            assert!(r.diagnostics().into_iter().find(|d| d.code == SCENE).unwrap().origin.is_empty());
        }
    }
}
