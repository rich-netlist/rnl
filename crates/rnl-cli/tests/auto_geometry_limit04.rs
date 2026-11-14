//! A placed 0.4 auto wire that cannot be represented reports one error.
use rnl_cli::interpret_file;
use rnl_runtime::{diag::Severity, source::MemoryProvider, Interpretation, Options};
use std::path::Path;

const MAX: &str = "1.7976931348623157e308";
fn read(body: &str) -> Interpretation {
    let source = format!("* auto geometry limit\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\nM1 d g s b missing\n* @RNL {{ {} }}\n.end\n", body.replace('\n', "\n* "));
    interpret_file(
        Path::new("limit.sp"),
        None,
        &MemoryProvider::new().with("limit.sp", &source),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap()
}
fn geometry_errors(r: &Interpretation) -> usize {
    r.diagnostics()
        .iter()
        .filter(|d| d.severity == Severity::Error && d.code.number().as_deref() == Some("RNL9405"))
        .count()
}
/// A pin at the largest coordinate that faces up: a wire to a point below it cannot leave it
/// upward and come back past it, nor at a right angle, as no finite coordinate lies beside it.
fn prefix() -> String {
    format!(
        r#"$S = @DEFINE-SYMBOL {{ %p = (0,0) @PIN(terminal=1,direction="up") }};
        #R1 @SET-SYMBOL $S => @PLACE({MAX},0)"#
    )
}

#[test]
fn known_auto_path_failure_is_one_error_across_model_and_problem_consumers() {
    let r = read(&format!(r#"{}; &a @WIRE {{#R1::p ({MAX},-4)}}"#, prefix()));
    assert_eq!(geometry_errors(&r), 1, "{:?}", r.diagnostics());
    assert!(r.model.levels[0].wires.is_empty(), "no diagonal fallback is drawn");
}

#[test]
fn unresolved_gap_is_not_an_auto_geometry_error() {
    let r = read(&format!(r#"{}; &a @WIRE {{#R1::p (_,_)}}"#, prefix()));
    assert_eq!(geometry_errors(&r), 0, "{:?}", r.diagnostics());
}

#[test]
fn ignored_undrawn_wire_does_not_report_geometry_error() {
    let r = read(&format!(r#"{}; &a @WIRE {{#R1::p ({MAX},4) #M1:g}}"#, prefix()));
    assert!(!r.model.levels[0].undrawn.is_empty());
    assert_eq!(geometry_errors(&r), 0, "{:?}", r.diagnostics());
}

#[test]
fn effective_rewrite_is_checked_and_reported_once() {
    let r = read(&format!(
        r#"{}; %w = &a @WIRE(mode="straight") {{(0,0) (1,0)}};
        %w @WIRE {{#R1::p ({MAX},-4)}}"#,
        prefix()
    ));
    assert_eq!(geometry_errors(&r), 1, "{:?}", r.diagnostics());
}
