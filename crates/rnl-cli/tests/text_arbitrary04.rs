//! 0.4 text properties and the released language boundary.
use rnl_runtime::{diag::Severity, source::MemoryProvider, Options};
use std::path::Path;

fn read(version: &str, body: &str) -> rnl_runtime::Interpretation {
    let src = format!(
        "* text\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n.end\n"
    );
    rnl_cli::interpret_file(
        Path::new("text.sp"),
        None,
        &MemoryProvider::new().with("text.sp", &src),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap()
}
fn errors(r: &rnl_runtime::Interpretation) -> usize {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).count()
}
#[test]
fn angles_and_new_fields_follow_the_source_version() {
    for version in ["0.2", "0.3"] {
        assert!(errors(&read(version, "(0,0) @TEXT(\"x\",rotate=30)")) > 0);
        assert!(errors(&read(version, "(0,0) @TEXT(\"x\",upright=false)")) > 0);
        assert!(errors(&read(version, "(0,0) @TEXT(\"x\",textvisible=false)")) > 0);
    }
    for angle in ["-360", "30.5", "90", "180", "270", "-45", "390"] {
        let r = read("0.4", &format!("(0,0) @TEXT(\"x\",rotate={angle},upright=false)"));
        assert_eq!(errors(&r), 0, "{:?}", r.diagnostics());
    }
}
#[test]
fn hidden_text_keeps_semantics_and_has_no_drawing_or_solver_obstacle() {
    let r = read("0.4", "(0,0) @TEXT(\"hidden\",textvisible=false)");
    assert_eq!(errors(&r), 0, "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].texts.len(), 1);
    assert!(rnl_render::text_boxes(&r.model.levels[0], &Default::default()).iter().all(Option::is_none));
    assert!(rnl_render::primitives(&r.model.levels[0], &Default::default())
        .iter()
        .all(|p| !matches!(p.kind, rnl_render::Kind::Text(_))));
    assert!(r.problems[0]
        .objects
        .iter()
        .filter_map(|o| o.text.as_ref())
        .any(|t| t.style.iter().any(|(k, v)| k == "textvisible" && v.boolean() == Some(false))));
}
#[test]
fn text_visibility_uses_a_style_definition_in_04() {
    let r = read("0.4", "$hidden = @DEFINE-STYLE(textvisible=false) {}; (0,0) @TEXT(\"hidden\") => @SET-STYLE $hidden");
    assert_eq!(errors(&r), 0, "{:?}", r.diagnostics());
    assert!(rnl_render::text_boxes(&r.model.levels[0], &Default::default()).iter().all(Option::is_none));
}
#[test]
fn set_style_rejects_direct_text_visibility_parameters_in_04() {
    for body in [
        "(0,0) @TEXT(\"hidden\") => @SET-STYLE(textvisible=false)",
        "$hidden = @DEFINE-STYLE {}; (0,0) @TEXT(\"hidden\") => @SET-STYLE(textvisible=false) $hidden",
    ] {
        let r = read("0.4", body);
        assert!(errors(&r) > 0, "{:?}", r.diagnostics());
    }
}
