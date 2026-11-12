//! A no-connect union role retains the kind and identity of its filler (0.4, 12.6).
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::{codes, Severity},
    problem::Filler,
    source::MemoryProvider,
    Interpretation, Options,
};

const SYMBOL: &str = r#"
    $S = @DEFINE-SYMBOL {
        %p = (0,0) @PIN(terminal=1);
        %q = (1,0) @PIN(terminal=2)
    };
    #R1 @SET-SYMBOL $S
"#;

fn read(role: &str, body: &str) -> Interpretation {
    // This local definition has the standard mark's role name and body. It does not
    // load or modify the standard package, so the role semantics are tested in isolation.
    let source = format!(
        "* no-connect roles\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n\
         R1 a b 1k\n* @RNL {{\n\
         * $NO_CONNECT = @DEFINE(roles=[[\"t\",{role}]]) {{ {{? ?t}} @ALIGN(\"x\"); {{? ?t}} @ALIGN(\"y\") }};\n\
         * $PROBE = @DEFINE(roles=[\"x\"]) {{}};\n* {}\n* }}\n.end\n",
        body.replace('\n', "\n* ")
    );
    interpret_file(
        Path::new("no-connect.sp"),
        None,
        &MemoryProvider::new().with("no-connect.sp", &source),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap()
}

fn assert_no_errors(r: &Interpretation) {
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
}

fn fillers(r: &Interpretation, definition: &str) -> Vec<Filler> {
    r.problems[0]
        .usages
        .iter()
        .filter(|u| u.definition == definition)
        .flat_map(|u| u.roles.iter().flat_map(|role| role.members.iter().cloned()))
        .collect()
}

#[test]
fn union_terminal_remains_a_terminal_without_an_assigned_symbol() {
    let r = read(r#"["terminal","pin"]"#, "%u = $NO_CONNECT @ATTACH #R1:1");
    assert_no_errors(&r);
    let mark = fillers(&r, "$NO_CONNECT");
    assert!(matches!(mark.as_slice(), [Filler::Terminal { terminal, .. }] if terminal == "p"), "{mark:?}");
    let owner = r.problems[0].objects.iter().find(|o| o.label == "#R1").unwrap();
    assert!(
        owner.symbol.is_none_or(|i| r.problems[0].symbols[i].default),
        "the test must not assign a symbol to the host"
    );
}

#[test]
fn union_terminal_selection_keeps_the_role_members_kind() {
    let body = format!(
        r#"{SYMBOL}; %u = $NO_CONNECT @ATTACH #R1:1;
        %u @SELECT-TERMINAL(role="t") => @AS $PROBE"#
    );
    let r = read(r#"["terminal","pin"]"#, &body);
    assert_no_errors(&r);
    let mark = fillers(&r, "$NO_CONNECT");
    let selected = fillers(&r, "$PROBE");
    assert!(matches!(mark.as_slice(), [Filler::Terminal { terminal, .. }] if terminal == "p"), "{mark:?}");
    assert_eq!(selected, mark, "role selection must retain the exact terminal");

    let body = format!(
        r#"{SYMBOL}; %u = $NO_CONNECT @ATTACH #R1:1;
        %u @SELECT-PIN(role="t") => @AS $PROBE"#
    );
    let r = read(r#"["terminal","pin"]"#, &body);
    assert_no_errors(&r);
    assert!(fillers(&r, "$PROBE").is_empty(), "terminal role members are not reinterpreted as pin scopes");
}

#[test]
fn union_named_pin_stays_the_same_pin_and_pin_selection_finds_it() {
    let body = format!(
        r#"{SYMBOL}; %u = $NO_CONNECT @ATTACH #R1::p;
        %u @SELECT-PIN(role="t") => @AS $PROBE"#
    );
    let r = read(r#"["terminal","pin"]"#, &body);
    assert_no_errors(&r);
    let mark = fillers(&r, "$NO_CONNECT");
    assert!(matches!(mark.as_slice(), [Filler::Pin { pin: 0, .. }]), "{mark:?}");
    assert_eq!(fillers(&r, "$PROBE"), mark, "selection must retain the exact physical pin");
}

#[test]
fn pin_only_role_converts_a_terminal_after_symbol_assignment() {
    let body = format!(
        r#"{SYMBOL}; %u = $NO_CONNECT @ATTACH #R1:1;
        %u @SELECT-PIN(role="t") => @AS $PROBE"#
    );
    let r = read(r#""pin""#, &body);
    assert_no_errors(&r);
    let mark = fillers(&r, "$NO_CONNECT");
    assert!(matches!(mark.as_slice(), [Filler::Pin { pin: 0, .. }]), "{mark:?}");
    assert_eq!(fillers(&r, "$PROBE"), mark);
}

#[test]
fn nonexistent_terminal_is_reported_and_makes_no_mark() {
    let r = read(r#"["terminal","pin"]"#, "%u = $NO_CONNECT @ATTACH #R1:99");
    assert!(
        r.diagnostics().iter().any(|d| d.severity == Severity::Error && d.code == codes::REF_NO_TERMINAL),
        "{:?}",
        r.diagnostics()
    );
    assert!(fillers(&r, "$NO_CONNECT").is_empty());
}
