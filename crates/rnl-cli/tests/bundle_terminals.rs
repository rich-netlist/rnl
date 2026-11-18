//! Bundle terminals follow member names, independently of their electrical order (11.8).
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::problem::{Filler, PointRef};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn source(version: &str, second_names: &str, value: &str, reference: &str) -> String {
    format!(
        r#"* bundle names
* @RNL(type="header") {{ @VERSION("{version}"); @DEVICE(name="R1", terminals=["p","n"]); @DEVICE(name="R2", terminals={second_names}) }}
R1 a b 1k
R2 c d {value}
* @RNL {{ $D = @DEFINE(roles=[["n","net"]]) {{}}; %bundle = {{#R1 #R2}} @BUNDLE; %u = {reference} @AS $D }}
.end
"#
    )
}

fn read(text: &str) -> Interpretation {
    let result = interpret_file(
        Path::new("t.sp"),
        None,
        &MemoryProvider::new().with("t.sp", text),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap();
    assert!(result.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", result.diagnostics());
    result
}

fn role_nets(result: &Interpretation) -> Vec<String> {
    result.problems[0]
        .usages
        .iter()
        .flat_map(|usage| &usage.roles[0].members)
        .filter_map(|filler| match filler {
            Filler::Net(name) => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn mismatch_count(result: &Interpretation) -> usize {
    result.diagnostics().iter().filter(|d| d.code == codes::BUNDLE_MEMBER_KIND_DIFFERS).count()
}

#[test]
fn named_and_numbered_references_follow_the_first_members_terminal_name() {
    for reference in ["%bundle:p::net", "%bundle:1::net"] {
        let result = read(&source("0.4", r#"["n","p"]"#, "1k", reference));
        assert_eq!(role_nets(&result), ["a", "d"]);
        assert_eq!(mismatch_count(&result), 1);
    }
}

#[test]
fn a_member_without_the_terminal_name_contributes_no_net() {
    let result = read(&source("0.4", r#"["x","n"]"#, "1k", "%bundle:p::net"));
    assert_eq!(role_nets(&result), ["a"]);
    assert_eq!(mismatch_count(&result), 1);
}

#[test]
fn output_pin_connections_use_the_same_name_mapping() {
    let result = read(&source("0.4", r#"["n","p"]"#, "1k", "%bundle:p::net"));
    let networks: Vec<_> = result.problems[0]
        .nets
        .iter()
        .filter(|net| net.pins.iter().any(|pin| matches!(pin, PointRef::Pin { terminal, .. } if terminal == "p")))
        .map(|net| net.name.as_str())
        .collect();
    assert_eq!(networks, ["a", "d"]);
}

#[test]
fn spice_terminal_names_ignore_case() {
    let result = read(&source("0.4", r#"["P","N"]"#, "1k", "%bundle:p::net"));
    assert_eq!(role_nets(&result), ["a", "c"]);
    assert_eq!(mismatch_count(&result), 0);
}

#[test]
fn differences_in_parameters_alone_do_not_warn() {
    let result = read(&source("0.4", r#"["p","n"]"#, "1k temp=2", "%bundle:p::net"));
    assert_eq!(mismatch_count(&result), 0);
}

#[test]
fn equal_member_networks_return_one_network() {
    let result = read(&source("0.4", r#"["n","p"]"#, "1k", "%bundle:p::net").replace("R2 c d", "R2 c a"));
    assert_eq!(role_nets(&result), ["a"]);
}

#[test]
fn a_scoped_record_on_an_included_level_follows_terminal_names() {
    let body = r#"$D = @DEFINE(roles=[["n","net"]]) {}; %bundle = {#R1 #R2} @BUNDLE; %u = %bundle:p::net @AS $D"#;
    let main = format!("* mixed\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n.include child.sp\n* @RNL(scope=\"child\") {{ {body} }}\n.end\n");
    let child = format!("* child\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @DEVICE(name=\"R1\", terminals=[\"p\",\"n\"]); @DEVICE(name=\"R2\", terminals=[\"n\",\"p\"]) }}\n.subckt child a b c d\nR1 a b 1k\nR2 c d 1k\n.ends child\n");
    let provider = MemoryProvider::new().with("t.sp", &main).with("child.sp", &child);
    let result =
        interpret_file(Path::new("t.sp"), None, &provider, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(result.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", result.diagnostics());
    let nets: Vec<_> = result
        .problems
        .iter()
        .flat_map(|p| &p.usages)
        .flat_map(|u| &u.roles[0].members)
        .filter_map(|f| match f {
            Filler::Net(name) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(nets, ["a", "d"]);
}

#[test]
fn net_attributes_follow_names_and_collapse_a_shared_net() {
    for (version, second_net, expected) in [("0.4", "d", "a,d"), ("0.4", "a", "a")] {
        let text = source(version, r#"["n","p"]"#, "1k", "%bundle:p::net")
            .replace("R2 c d", &format!("R2 c {second_net}"))
            .replace("%u = %bundle:p::net @AS $D", r#"%bundle @PLACE(0,0); %bundle:p @TEXT("${net}")"#);
        let result = read(&text);
        let labels: Vec<_> = result
            .model
            .levels
            .iter()
            .flat_map(|l| &l.texts)
            .map(|t| t.lines.iter().map(|line| rnl_runtime::markup::plain(line)).collect::<Vec<_>>().join("\n"))
            .collect();
        assert!(labels.iter().any(|label| label == expected), "{version}: {labels:?}");
    }
}

#[test]
fn unnamed_terminals_keep_their_electrical_number() {
    let result = spectre("", "opaque", "opaque", "%bundle:1::net");
    assert_eq!(role_nets(&result), ["a", "c"]);
}

fn spectre(header: &str, first_model: &str, second_model: &str, reference: &str) -> Interpretation {
    let text = format!(
        r#"// bundle terminals
simulator lang=spectre
// @RNL(type="header") {{ @VERSION("0.4"); {header} }}
I1 (a b) {first_model}
I2 (c d) {second_model}
// @RNL {{ $D = @DEFINE(roles=[["n","net"]]) {{}}; %bundle = {{#I1 #I2}} @BUNDLE; {reference} @AS $D }}
"#
    );
    let result = interpret_file(
        Path::new("t.scs"),
        None,
        &MemoryProvider::new().with("t.scs", &text),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap();
    assert!(result.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", result.diagnostics());
    result
}

#[test]
fn spectre_names_and_model_names_are_case_sensitive() {
    let header = r#"@DEVICE(name="I1", terminals=["p","n"]); @DEVICE(name="I2", terminals=["P","n"])"#;
    let result = spectre(header, "opaque", "opaque", "%bundle:p::net");
    assert_eq!(role_nets(&result), ["a"]);
    assert_eq!(mismatch_count(&result), 1);
    let result = spectre("", "opaque", "OPAQUE", "%bundle:1::net");
    assert_eq!(mismatch_count(&result), 1);
    let result = spectre("", "opaque", "opaque", "%bundle:1::net");
    assert_eq!(mismatch_count(&result), 0);
}

#[test]
fn terminal_count_and_model_differences_warn() {
    let text = source("0.4", r#"["p","n"]"#, "1k", "%bundle:p::net")
        .replace("R1 a b 1k", "X1 a b cell")
        .replace("R2 c d 1k", "X2 c d e cell")
        .replace("R1", "X1")
        .replace("R2", "X2");
    assert_eq!(mismatch_count(&read(&text)), 1);
    let text = text.replace("X2 c d e cell", "X2 c d other");
    assert_eq!(mismatch_count(&read(&text)), 1);
}
