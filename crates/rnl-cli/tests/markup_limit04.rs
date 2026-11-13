//! Limits on math markup must fail 0.4 statements without affecting siblings.
use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::{codes, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn formula(depth: usize) -> String {
    format!(r"\({}x{}\)", "{".repeat(depth), "}".repeat(depth))
}
fn read(version: &str, body: &str) -> Interpretation {
    let text = format!(
        "* markup\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a 0 1k\nR2 b 0 1k\n* @RNL {{ {body} }}\n.end\n"
    );
    let files = MemoryProvider::new().with("main.sp", &text);
    interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, problems: true, ..Default::default() })
        .unwrap()
}
#[test]
fn text_limit_is_error_and_does_not_draw_but_sibling_survives() {
    let f = formula(65);
    let r = read("0.4", &format!(r#"(0,0) @TEXT("{f}"); (4,0) @TEXT("after")"#));
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::MARKUP_MATH_DEPTH_LIMIT).collect();
    assert_eq!(errors.len(), 1, "{:?}", r.diagnostics());
    assert_eq!(errors[0].severity, Severity::Error);
    assert_eq!(errors[0].loc.as_ref().unwrap().line, 5);
    assert!(r.model.levels[0].texts.iter().any(|t| t.markup == "after"));
    assert!(!r.model.levels[0].texts.iter().any(|t| t.markup == f));
}
#[test]
fn old_versions_keep_literal_math_without_a_limit_error() {
    let f = formula(65);
    for version in ["0.2", "0.3"] {
        let r = read(version, &format!(r#"(0,0) @TEXT("{f}")"#));
        assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error || d.code == codes::MARKUP_MATH_DEPTH));
        let text = r.model.levels[0].texts.iter().find(|t| t.markup == f).unwrap();
        assert_eq!(rnl_runtime::markup::plain(&text.lines[0]), f);
    }
}
#[test]
fn pin_label_limit_fails_only_pin_and_preserves_other_pin() {
    let f = formula(65);
    let r = read(
        "0.4",
        &format!(
            r#"$S = @DEFINE-SYMBOL {{ (0,0) @PIN(label="{f}"); (2,0) @PIN(terminal=1) }}; #R1 @SET-SYMBOL $S => @PLACE(0,0)"#
        ),
    );
    assert!(
        r.diagnostics().iter().any(|d| d.code == codes::MARKUP_MATH_DEPTH_LIMIT
            && d.severity == Severity::Error
            && d.loc.as_ref().is_some_and(|l| l.line == 5)),
        "{:?}",
        r.diagnostics()
    );
    let p = &r.problems[0];
    let owner = p.objects.iter().find(|o| o.label == "#R1").unwrap();
    let symbol = &p.symbols[owner.symbol.unwrap()];
    assert_eq!(symbol.pins.len(), 1);
    assert_eq!(symbol.pins[0].pos, (2., 0.));
}
#[test]
fn valid_depth_and_other_markup_warnings_keep_working() {
    let f = formula(63);
    let r = read("0.4", &format!(r#"(0,0) @TEXT("{f}"); (4,0) @TEXT("\unknown")"#));
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Warning));
    assert_eq!(r.model.levels[0].texts.len(), 2);
}

#[test]
fn fixed_definition_text_is_checked_once_even_when_unused_or_applied_twice() {
    let f = formula(65);
    for calls in ["", "#R1 @AS $D => @PLACE(0,0); #R2 @AS $D => @PLACE(10,0)"] {
        let body =
            format!(r#"$D = @DEFINE(roles=[["o","instance"]]) {{ (0,0) @TEXT("{f}"); (4,0) @TEXT("kept") }}; {calls}"#);
        let r = read("0.4", &body);
        let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::MARKUP_MATH_DEPTH_LIMIT).collect();
        assert_eq!(errors.len(), 1, "{calls}: {:?}", r.diagnostics());
        assert_eq!(errors[0].severity, Severity::Error);
        assert!(errors[0].origin.is_empty());
        assert_eq!(errors[0].loc.as_ref().unwrap().line, 5);
        assert!(!r.model.levels[0].texts.iter().any(|t| t.markup == f));
        if !calls.is_empty() {
            assert_eq!(r.model.levels[0].texts.iter().filter(|t| t.markup == "kept").count(), 2);
        }
    }
}

#[test]
fn annotation_omits_fixed_failed_definition_text_once() {
    let f = formula(65);
    let body = format!(
        r#"$D = @DEFINE(roles=[]) {{ ? @TEXT("{f}"); (0,0) (1,0) @LINE }}; (2,3) @ANNOTATION $D; (4,5) @ANNOTATION $D"#
    );
    let r = read("0.4", &body);
    assert_eq!(
        r.diagnostics().iter().filter(|d| d.code == codes::MARKUP_MATH_DEPTH_LIMIT).count(),
        1,
        "{:?}",
        r.diagnostics()
    );
    assert_eq!(r.model.levels[0].annotations.len(), 2);
    for a in &r.model.levels[0].annotations {
        assert_eq!(a.graphics.len(), 1);
        assert!(a.graphics.iter().all(|g| g.primitive.text.is_none()));
    }
}
#[test]
fn annotation_checks_dynamic_definition_text_before_materialization() {
    let f = formula(65);
    let body = format!(
        r#"$D = @DEFINE(roles=[],parameters=[["s","ok"]]) {{ ? @TEXT(?s); (0,0) (1,0) @LINE }}; (2,3) @ANNOTATION(s="{f}") $D"#
    );
    let r = read("0.4", &body);
    assert_eq!(
        r.diagnostics().iter().filter(|d| d.code == codes::MARKUP_MATH_DEPTH_LIMIT).count(),
        1,
        "{:?}",
        r.diagnostics()
    );
    assert_eq!(r.model.levels[0].annotations.len(), 1);
    let a = &r.model.levels[0].annotations[0];
    assert_eq!(a.graphics.len(), 1);
    assert!(a.graphics.iter().all(|g| g.primitive.text.is_none()));
}

#[test]
fn forwarded_math_uses_the_version_that_wrote_the_string() {
    let f = formula(65);
    for (writer, definition) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let main = format!(
            r#"* forwarded math
* @RNL(type="header") {{ @VERSION("{writer}") }}
.include "defs.inc"
* @RNL(scope="cell") {{ #R1 @AS(s="{f}") $D }}
.end
"#
        );
        let defs = format!(
            r#"* definition
* @RNL(type="header") {{ @VERSION("{definition}") }}
.subckt cell a b
R1 a b 1k
* @RNL {{ $S = @DEFINE-SYMBOL {{ (0,0) @CIRCLE(radius=1) }}; #R1 @SET-SYMBOL $S => @PLACE(0,0); $D = @DEFINE(roles=["r"],parameters=["s"]) {{ ?r @TEXT(?s) }} }}
.ends cell
"#
        );
        let files = MemoryProvider::new().with("main.sp", &main).with("defs.inc", &defs);
        let r =
            interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Default::default() }).unwrap();
        let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
        let level = r.model.levels.iter().find(|level| level.title == "cell").unwrap();
        if writer == "0.4" {
            assert_eq!(errors.len(), 1, "{:?}", r.diagnostics());
            assert_eq!(errors[0].code, codes::MARKUP_MATH_DEPTH_LIMIT);
            assert!(errors[0].loc.is_some());
            assert!(!level.texts.iter().any(|t| t.markup == f));
        } else {
            assert!(errors.is_empty(), "{:?}", r.diagnostics());
            let text = level.texts.iter().find(|t| t.markup == f).unwrap();
            assert_eq!(rnl_runtime::markup::plain(&text.lines[0]), f);
        }
    }
}
