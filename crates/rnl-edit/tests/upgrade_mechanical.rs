use rnl_edit::upgrade::{upgrade, Upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;

fn convert(record: &str) -> Upgrade {
    let source =
        format!("* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\nR1 a 0 1k\n* @RNL {{ {record} }}\n.end\n");
    upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", &source),
        &UpgradeOptions { std: false, ..UpgradeOptions::default() },
    )
    .unwrap()
}

#[test]
fn route_changes_only_the_keyword() {
    let result = convert("&a @ROUTE(mode=\"straight\", /* keep */ linewidth=1)");
    assert!(result.files["t.sp"].contains("&a @ROUTE-NET(mode=\"straight\", /* keep */ linewidth=1)"));
    assert!(result.report.files[0].unconverted.is_empty());
    assert_eq!(result.report.files[0].changes.iter().find(|c| c.rule == "route-net").unwrap().before, "@ROUTE");
}

#[test]
fn implicit_recursion_is_explicit_with_existing_parameters_and_comments() {
    for (source, target) in [
        ("# @SELECT-NET", "# @SELECT-NET(recursive=true)"),
        ("# @SELECT-NET(/* keep */)", "# @SELECT-NET(/* keep */recursive=true)"),
        ("# @SELECT-NET(name=\"a\")", "# @SELECT-NET(name=\"a\", recursive=true)"),
        ("# @SELECT-NET(recursive=false)", "# @SELECT-NET(recursive=false)"),
        ("# @SELECT-NET(recursive=true)", "# @SELECT-NET(recursive=true)"),
    ] {
        let result = convert(source);
        assert!(result.files["t.sp"].contains(target), "{}", result.files["t.sp"]);
        assert!(result.report.files[0].unconverted.is_empty());
    }
}

#[test]
fn mixed_recursion_keeps_the_statement_and_reports_it() {
    let source = "# @SELECT-NET(recursive=false) => @SELECT-TERMINAL";
    let result = convert(source);
    assert!(result.files["t.sp"].contains(source));
    assert!(result.report.files[0].unconverted.iter().any(|u| u.rule == "recursive"));
    assert!(!result.report.successful());
}

#[test]
fn an_adjacent_set_stays_a_separate_object() {
    let result = convert("%p=(0, 0); %q=(1, 0); %s={%p%q}; %s{%p} @ALIGN(\"x\")");
    assert!(result.files["t.sp"].contains("%s {%p} @ALIGN"), "{}", result.files["t.sp"]);
    assert!(result.report.files[0].changes.iter().any(|c| c.rule == "separate-set"));
}

#[test]
fn recursion_inside_a_mixed_scope_is_not_partially_changed() {
    let source = "{# @SELECT-NET(recursive=false); # @SELECT-NET} @SELECT-TERMINAL";
    let result = convert(source);
    assert!(!result.files["t.sp"].contains("recursive=true"));
    assert!(result.report.files[0].unconverted.iter().any(|u| u.rule == "recursive"));
}

#[test]
fn strings_and_comments_are_not_rewritten_as_calls() {
    let source = "(0, 0) @TEXT(\"@ROUTE # @SELECT-NET\"); /* @ROUTE */ &a @ROUTE";
    let result = convert(source);
    assert!(result.files["t.sp"].contains("@TEXT(\"@ROUTE # @SELECT-NET\")"));
    assert!(result.files["t.sp"].contains("/* @ROUTE */ &a @ROUTE-NET"));
}

#[test]
fn distance_intervals_that_change_meaning_are_retained_and_reported() {
    for bounds in ["min=-2, max=3", "exact=-2", "max=3"] {
        let statement = format!("(0, 0) (1, 0) @DISTANCE(\"left-to-right\", {bounds})");
        let result = convert(&statement);
        assert!(result.files["t.sp"].contains(&statement));
        assert!(result.report.files[0].unconverted.iter().any(|u| u.rule == "distance-interval"));
        assert!(!result.report.successful());
    }
    let result = convert("(0, 0) (1, 0) @DISTANCE(\"left-to-right\", min=0, max=3)");
    assert!(result.report.files[0].unconverted.is_empty());
}
