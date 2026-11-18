//! Symbol labels belong to the snapshot read by generation rules (10.5).
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read_files(source: &str, extra: Option<&str>) -> Interpretation {
    let mut files = MemoryProvider::new().with("t.sp", source);
    if let Some(extra) = extra {
        files = files.with("rules.inc", extra);
    }
    let result = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(!result.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", result.diagnostics());
    result
}

fn source(version: &str, body: &str) -> String {
    format!("* symbol labels\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ $P = @DEFINE(roles=[\"x\"]) {{}}; {body} }}\n.end\n")
}

fn read(version: &str, body: &str) -> Interpretation {
    read_files(&source(version, body), None)
}
fn probes(result: &Interpretation) -> usize {
    result.problems[0].usages.iter().filter(|u| u.definition == "$P").count()
}
const SYMBOL: &str = "$S = @DEFINE-SYMBOL { ? @TEXT(\"symbol\",key=\"mark\") }; #R1 @SET-SYMBOL $S";
const SELECT: &str = "# @SELECT-LABEL(key=\"mark\") => @AS $P";

#[test]
fn symbol_labels_are_available_before_generation() {
    assert_eq!(probes(&read("0.4", &format!("{SYMBOL}; {SELECT}"))), 1);
}

#[test]
fn repeated_generations_read_the_same_symbol_label() {
    assert_eq!(probes(&read("0.4", &format!("{SYMBOL}; {SELECT}; {SELECT}"))), 2);
}

#[test]
fn a_generation_overriding_a_key_does_not_remove_the_existing_label_from_the_snapshot() {
    let change = "# @SELECT-INSTANCE => @TEXT(\"changed\",key=\"mark\")";
    for body in [format!("{SYMBOL}; {change}; {SELECT}"), format!("{SYMBOL}; {SELECT}; {change}")] {
        let result = read("0.4", &body);
        assert_eq!(probes(&result), 1);
        let label = result.problems[0].objects.iter().find(|object| object.label == "#R1::mark").unwrap();
        assert!(!label.generated, "overriding a key does not create another object");
    }
}

#[test]
fn a_new_key_created_by_generation_stays_out_of_the_snapshot() {
    let body = "$S = @DEFINE-SYMBOL { ? @TEXT(\"original\",key=\"other\") }; #R1 @SET-SYMBOL $S; # @SELECT-INSTANCE => @TEXT(\"new\",key=\"mark\")";
    assert_eq!(probes(&read("0.4", &format!("{body}; {SELECT}"))), 0);
}

#[test]
fn an_explicit_key_override_keeps_one_label() {
    assert_eq!(probes(&read("0.4", &format!("{SYMBOL}; #R1 @TEXT(\"explicit\",key=\"mark\"); {SELECT}"))), 1);
}

#[test]
fn labels_of_generated_usages_stay_out_of_the_snapshot() {
    let body = "$S = @DEFINE-SYMBOL { ? @TEXT(\"generated\",key=\"mark\") }; $D = @DEFINE(roles=[\"x\"]) { ? @SET-SYMBOL $S }; # @SELECT-INSTANCE => @AS $D";
    assert_eq!(probes(&read("0.4", &format!("{body}; {SELECT}"))), 0);
}

#[test]
fn anonymous_symbol_labels_are_not_materialized_twice() {
    let body = "$S = @DEFINE-SYMBOL { ? @TEXT(\"symbol\") }; #R1 @SET-SYMBOL $S; #R1 @SELECT-LABEL => @AS $P";
    let result = read("0.4", body);
    assert_eq!(probes(&result), 1);
    assert_eq!(result.problems[0].objects.iter().filter(|o| o.text.is_some()).count(), 1);
}

#[test]
fn a_scoped_selection_sees_the_symbol_labels_of_an_included_level() {
    let input = format!("* scoped rules\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n.include rules.inc\n* @RNL(scope=\"child\") {{ $P = @DEFINE(roles=[\"x\"]) {{}}; {SYMBOL}; {SELECT} }}\n.end\n");
    let extra = "* included circuit\n.subckt child a b\nR1 a b 1k\n.ends child\n";
    let result = read_files(&input, Some(extra));
    let count =
        result.problems.iter().flat_map(|problem| &problem.usages).filter(|usage| usage.definition == "$P").count();
    assert_eq!(count, 1);
}

#[test]
fn empty_keyed_text_hides_the_symbol_label_without_drawing_anything() {
    let options = rnl_render::RenderOptions { flylines: false, ..Default::default() };
    let before = read("0.4", &format!("{SYMBOL}; #R1 @PLACE(0,0)"));
    assert!(!rnl_render::primitives(&before.model.levels[0], &options).is_empty());
    for text in ["", "${model}"] {
        let result = read("0.4", &format!("{SYMBOL}; #R1 @PLACE(0,0); #R1 @TEXT(\"{text}\",key=\"mark\")"));
        assert!(rnl_render::primitives(&result.model.levels[0], &options).is_empty());
        assert!(rnl_render::text_boxes(&result.model.levels[0], &options).iter().all(Option::is_none));
    }
}

#[test]
fn an_empty_annotation_has_no_rendered_box() {
    let options = rnl_render::RenderOptions { flylines: false, ..Default::default() };
    let result = read("0.4", "(0,0) @TEXT(\"\")");
    assert!(rnl_render::primitives(&result.model.levels[0], &options).is_empty());
    assert!(rnl_render::text_boxes(&result.model.levels[0], &options).iter().all(Option::is_none));
}
