//! All generation rules read the state before any of them runs (10.5).
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(version: &str, body: &str) -> Interpretation {
    let source = format!("* snapshot\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ $P = @DEFINE(roles=[\"x\"]) {{}}; {body} }}\n.end\n");
    let result = interpret_file(
        Path::new("t.sp"),
        None,
        &MemoryProvider::new().with("t.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap();
    assert!(!result.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", result.diagnostics());
    result
}

fn probes(result: &Interpretation) -> usize {
    result.problems[0].usages.iter().filter(|u| u.definition == "$P").count()
}

const DEFINITION: &str = "$D = @DEFINE(roles=[\"x\"]) { (0,0) @CIRCLE(radius=1) }";

#[test]
fn a_generated_body_graphic_is_not_an_input_to_another_generation() {
    let body = format!("{DEFINITION}; # @SELECT-INSTANCE => @AS $D; # @SELECT-ANNOTATION => @AS $P");
    assert_eq!(probes(&read("0.4", &body)), 0);
}

#[test]
fn a_nested_generated_body_graphic_is_not_an_input_either() {
    let body = format!("{DEFINITION}; $E = @DEFINE(roles=[\"x\"]) {{ ?x @AS $D }}; # @SELECT-INSTANCE => @AS $E; # @SELECT-ANNOTATION => @AS $P");
    assert_eq!(probes(&read("0.4", &body)), 0);
}

#[test]
fn explicit_body_graphics_stay_in_the_snapshot() {
    let body = format!("{DEFINITION}; #R1 @AS $D; # @SELECT-INSTANCE => @AS $D; # @SELECT-ANNOTATION => @AS $P; # @SELECT-ANNOTATION => @AS $P");
    assert_eq!(probes(&read("0.4", &body)), 2);
}

#[test]
fn later_styles_see_the_generated_graphics() {
    let body = format!("{DEFINITION}; $Red = @DEFINE-STYLE(linecolor=\"#ff0000\") {{}}; # @SELECT-INSTANCE => @AS $D; # @SELECT-USAGE $D => @PLACE(0,0); # @SELECT-ANNOTATION => @SET-STYLE $Red");
    let result = read("0.4", &body);
    assert_eq!(result.model.levels[0].shapes.len(), 1);
    assert_eq!(result.model.levels[0].shapes[0].style["linecolor"].str(), Some("#ff0000"));
}
