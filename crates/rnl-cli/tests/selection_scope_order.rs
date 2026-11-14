//! Selection scopes contribute each member's results in order, once (10.3).
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::Filler;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(version: &str, body: &str, selection: &str) -> Interpretation {
    let source=format!("* scope order\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 b c 2k\n* @RNL {{ $P = @DEFINE(roles=[\"x\"]) {{}}; {body}; {selection} => @AS $P }}\n.end\n");
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
    let p = &result.problems[0];
    p.usages
        .iter()
        .filter(|u| u.definition == "$P")
        .map(|u| match &u.roles[0].members[0] {
            Filler::Object(i) => p.objects[*i].label.clone(),
            Filler::Wire(i) => p.wires[*i].label.clone(),
            Filler::Usage(i) => p.usages[*i].label.clone(),
            Filler::Net(name) | Filler::Other(name) => name.clone(),
            other => panic!("unexpected filler: {other:?}"),
        })
        .collect()
}

const WIRES: &str = "%wa = &a @WIRE {(0,0) (1,0)}; %wb = &b @WIRE {(0,1) (1,1)}; %wa2 = &a @WIRE {(2,0) (3,0)}";
const LABELS: &str =
    "#R1 @TEXT(\"first\",key=\"first\"); #R2 @TEXT(\"second\",key=\"second\"); #R1 @TEXT(\"third\",key=\"third\")";

#[test]
fn wire_scopes_follow_net_order_and_keep_each_wire_once() {
    assert_eq!(labels(&read("0.4", WIRES, "{&b &a &b %wa} @SELECT-WIRE")), ["%wb", "%wa", "%wa2"]);
}

#[test]
fn explicit_wire_scopes_keep_their_written_order() {
    assert_eq!(labels(&read("0.4", WIRES, "{%wa2 %wa %wa2 %wb} @SELECT-WIRE")), ["%wa2", "%wa", "%wb"]);
}

#[test]
fn label_scopes_follow_owner_order_and_keep_each_label_once() {
    assert_eq!(
        labels(&read("0.4", LABELS, "{#R2 #R1 #R2} @SELECT-LABEL")),
        ["#R2::second", "#R2::name", "#R2::model", "#R1::first", "#R1::third", "#R1::name", "#R1::model"]
    );
}

#[test]
fn a_label_itself_is_given_at_its_first_scope_occurrence() {
    assert_eq!(
        labels(&read("0.4", LABELS, "{#R1::third #R2 #R1} @SELECT-LABEL")),
        ["#R1::third", "#R2::second", "#R2::name", "#R2::model", "#R1::first", "#R1::name", "#R1::model"]
    );
}

#[test]
fn ancestor_usage_scopes_follow_the_member_order() {
    let body = "$D = @DEFINE(roles=[\"{xs}\"]) {}; %u1 = {#R1} @AS $D; %u2 = {#R2} @AS $D; %both = {#R1 #R2} @AS $D";
    assert_eq!(labels(&read("0.4", body, "{#R2 #R1 #R2} @SELECT-ANCESTOR-USAGE $D")), ["%u2", "%both", "%u1"]);
    assert_eq!(labels(&read("0.4", body, "{%u2 #R2 #R1} @SELECT-ANCESTOR-USAGE $D")), ["%both", "%u1"]);
}

#[test]
fn net_selections_deduplicate_overlapping_scopes() {
    assert_eq!(labels(&read("0.4", "", "{&b #R1 &a #R2} @SELECT-NET")), ["b", "a", "c"]);
}

#[test]
fn annotation_selections_deduplicate_repeated_members() {
    let body = "%a=(0,0) @CIRCLE(radius=1); %b=(3,0) @RECTANGLE(width=1,height=1)";
    assert_eq!(labels(&read("0.4", body, "{%b %a %b} @SELECT-ANNOTATION")), ["RECTANGLE", "CIRCLE"]);
}

#[test]
fn released_wire_and_label_scopes_keep_their_existing_scene_order() {
    assert_eq!(labels(&read("0.3", WIRES, "{&b &a} @SELECT-WIRE")), ["%wa", "%wb", "%wa2"]);
    assert_eq!(labels(&read("0.3", LABELS, "{#R2 #R1} @SELECT-LABEL")), ["#R1::first", "#R2::second", "#R1::third"]);
}

#[test]
fn usage_members_do_not_gain_an_extra_scope_step() {
    let body = format!("{WIRES}; $D = @DEFINE(roles=[\"x\"]) {{}}; %u=&a @AS $D");
    assert_eq!(labels(&read("0.4", &body, "{%u &b} @SELECT-WIRE")), ["%wb"]);
    let body = format!("{LABELS}; $D = @DEFINE(roles=[\"x\"]) {{}}; %u=#R1 @AS $D");
    assert_eq!(labels(&read("0.4", &body, "{%u #R2} @SELECT-LABEL")), ["#R2::second", "#R2::name", "#R2::model"]);
}
