//! Style parameters do not postpone a fixed graphic error until a definition is applied.
use std::path::Path;

use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(version: &str, aliases: &str, body: &str, calls: &str) -> Interpretation {
    let anchor = if version == "0.4" { "; # @SELECT-USAGE $D => @PLACE(0,0)" } else { "" };
    let text=format!("* graphic definitions\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {aliases} }}\nR1 a b 1k\n* @RNL {{ $D = @DEFINE(roles=[],parameters=[\"colour\",\"size\"]) {{ {body} }}; {calls}{anchor} }}\n");
    rnl_cli::interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &text),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}

#[test]
fn styles_do_not_hide_fixed_invalid_point_counts() {
    for shape in ["LINE", "ARC", "BEZIER", "RECTANGLE", "CIRCLE", "ELLIPSE"] {
        for style in ["linecolor=\"#123456\"", "linecolor=?colour"] {
            for calls in ["", "$D @ATTACH; $D @ATTACH"] {
                let r = read("0.4", "", &format!("(0,0) @{shape}({style}); (2,2) @CIRCLE(radius=1)"), calls);
                assert_eq!(
                    errors(&r),
                    [codes::KEYWORD_GRAPHIC_POINT_COUNT],
                    "{shape}/{style}/{calls}: {:?}",
                    r.diagnostics()
                );
                let d = r.diagnostics().into_iter().find(|d| d.code == codes::KEYWORD_GRAPHIC_POINT_COUNT).unwrap();
                assert!(d.origin.is_empty());
                assert_eq!(r.model.levels[0].shapes.len(), if calls.is_empty() { 0 } else { 2 });
            }
        }
    }
}

#[test]
fn complete_sizes_and_size_aliases_preserve_centred_graphics() {
    for (shape, params) in [("CIRCLE", "radius=1"), ("RECTANGLE", "width=1,height=2"), ("ELLIPSE", "width=1,height=2")]
    {
        let r = read("0.4", "", &format!("{{(0,0) (2,2)}} @{shape}({params},linecolor=\"#123456\")"), "$D @ATTACH");
        assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].shapes.len(), 2);
    }
    let r = read("0.4", "@ALIAS(\"r\",\"radius\")", "(0,0) @CIRCLE(r=1)", "$D @ATTACH");
    assert!(errors(&r).is_empty(), "{:?}", r.diagnostics());
    let r = read("0.4", "@ALIAS(\"colour\",\"linecolor\")", "(0,0) @LINE(colour=\"#123456\")", "");
    assert_eq!(errors(&r), [codes::KEYWORD_GRAPHIC_POINT_COUNT]);
}

#[test]
fn unknown_sizes_are_checked_only_for_the_application() {
    assert!(errors(&read("0.4", "", "(0,0) @CIRCLE(radius=?size)", "")).is_empty());
    assert!(errors(&read("0.4", "", "(0,0) @CIRCLE(radius=?size)", "$D @ATTACH(size=1)")).is_empty());
    let r = read("0.4", "", "(0,0) @CIRCLE(radius=?size)", "$D @ATTACH; $D @ATTACH");
    assert_eq!(errors(&r), [codes::KEYWORD_GRAPHIC_POINT_COUNT, codes::KEYWORD_GRAPHIC_POINT_COUNT]);
    assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| !d.origin.is_empty()));
}

#[test]
fn fixed_partial_dimensions_fail_once_at_the_definition() {
    for shape in ["RECTANGLE", "ELLIPSE"] {
        for dimension in ["width=1", "height=1"] {
            for calls in ["", "$D @ATTACH; $D @ATTACH"] {
                let r = read("0.4", "", &format!("(0,0) @{shape}({dimension})"), calls);
                assert_eq!(errors(&r), [codes::PARAM_GRAPHIC_WIDTH_HEIGHT], "{:?}", r.diagnostics());
                assert!(r.diagnostics().iter().filter(|d| d.severity == Severity::Error).all(|d| d.origin.is_empty()));
            }
        }
    }
}

#[test]
fn released_definitions_keep_application_time_errors() {
    for version in ["0.2", "0.3"] {
        let body = "(0,0) @LINE(linecolor=\"#123456\")";
        assert!(errors(&read(version, "", body, "")).is_empty());
        let r = read(version, "", body, "$D @ATTACH; $D @ATTACH");
        assert_eq!(errors(&r), [codes::KEYWORD_GRAPHIC_POINT_COUNT, codes::KEYWORD_GRAPHIC_POINT_COUNT]);
    }
}
