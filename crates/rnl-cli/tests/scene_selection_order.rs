//! Scene-scoped selections follow creation steps and source positions (10.3).
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::{FactKind, Filler, PointRef};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(version: &str, body: &str) -> Interpretation {
    let source = format!("* scene order\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 b c 2k\n* @RNL {{ $P = @DEFINE(roles=[\"x\"]) {{}}; {body} }}\n.end\n");
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

fn labels(result: &Interpretation) -> Vec<String> {
    let problem = &result.problems[0];
    problem
        .usages
        .iter()
        .filter(|u| u.definition == "$P")
        .map(|u| match &u.roles[0].members[0] {
            Filler::Object(index) => {
                let object = &problem.objects[*index];
                object
                    .text
                    .as_ref()
                    .map(|text| rnl_runtime::markup::plain(&text.lines[0]))
                    .unwrap_or_else(|| object.label.clone())
            }
            Filler::Usage(index) => problem.usages[*index].label.clone(),
            Filler::Wire(index) => problem.wires[*index].label.clone(),
            Filler::Other(name) => name.clone(),
            other => panic!("unexpected filler {other:?}"),
        })
        .collect()
}

const ANNOTATIONS: &str =
    "%t1=(0,0) @TEXT(\"first\"); (1,0) @CIRCLE(radius=1); %t2=(2,0) @TEXT(\"last\"); # @SELECT-ANNOTATION => @AS $P";

#[test]
fn scene_annotations_interleave_graphics_and_texts_by_source_position() {
    assert_eq!(labels(&read("0.4", ANNOTATIONS)), ["first", "CIRCLE", "last"]);
}

#[test]
fn forward_references_do_not_reorder_usages() {
    let body = "$D = @DEFINE(roles=[\"x\"]) {}; %later @PLACE(4,5); %first=#R1 @AS $D; %later=#R2 @AS $D; # @SELECT-USAGE $D => @AS $P";
    assert_eq!(labels(&read("0.4", body)), ["%first", "%later"]);
}

#[test]
fn forward_references_do_not_reorder_wires() {
    let body = "$S = @DEFINE-STYLE(linecolor=\"#ff0000\") {}; %later @SET-STYLE $S; %first=&a @WIRE {(0,0) (1,0)}; %later=&b @WIRE {(0,1) (1,1)}; # @SELECT-WIRE => @AS $P";
    assert_eq!(labels(&read("0.4", body)), ["%first", "%later"]);
}

#[test]
fn symbol_labels_follow_usage_source_order_after_explicit_labels() {
    let body = "$D = @DEFINE(roles=[\"x\"]) {}; $S = @DEFINE-SYMBOL { ? @TEXT(\"symbol\",key=\"mark\") }; %later @PLACE(4,5); %first=#R1 @AS $D; %later=#R2 @AS $D; # @SELECT-USAGE $D => @SET-SYMBOL $S; %later @TEXT(\"scene\",key=\"own\"); # @SELECT-LABEL => @ORDER(\"left-to-right\")";
    let result = read("0.4", body);
    let problem = &result.problems[0];
    let members = problem
        .facts
        .iter()
        .find_map(|fact| match &fact.kind {
            FactKind::Order { members, .. } => Some(members),
            _ => None,
        })
        .unwrap();
    let names: Vec<_> = members
        .iter()
        .map(|member| {
            let PointRef::Origin(index) = member else { panic!("label point expected") };
            problem.objects[*index].label.as_str()
        })
        .filter(|name| name.starts_with('%'))
        .collect();
    assert_eq!(names, ["%later::own", "%first::mark", "%later::mark"]);
}

#[test]
fn bodies_follow_the_usage_that_expands_them() {
    let body = "$Leaf = @DEFINE(roles=[\"x\"]) {}; $Wrap = @DEFINE(roles=[\"x\"]) { %nested=?x @AS $Leaf }; %outer=#R1 @AS $Wrap; %last=#R2 @AS $Leaf; # @SELECT-USAGE => @AS $P";
    assert_eq!(labels(&read("0.4", body)), ["%outer", "%nested", "%last"]);
}

#[test]
fn later_steps_follow_explicit_objects_even_when_the_rule_is_written_first() {
    let body = "$D = @DEFINE(roles=[\"x\"]) {}; # @SELECT-INSTANCE(name=\"R1\") => @AS $D; %explicit=#R2 @AS $D; # @SELECT-USAGE $D => @ORDER(\"left-to-right\")";
    let result = read("0.4", body);
    let problem = &result.problems[0];
    let members = problem
        .facts
        .iter()
        .find_map(|fact| match &fact.kind {
            FactKind::Order { members, .. } => Some(members),
            _ => None,
        })
        .unwrap();
    assert_eq!(members.len(), 2);
    // the usages have no symbol, so each stands for its member (14.1): #R2 of %explicit first
    let PointRef::Origin(first) = members[0] else { panic!("an instance expected") };
    assert_eq!(problem.objects[first].label, "#R2");
}

#[test]
fn released_scene_annotations_keep_their_existing_order() {
    assert_eq!(labels(&read("0.3", ANNOTATIONS)), ["CIRCLE", "first", "last"]);
}
