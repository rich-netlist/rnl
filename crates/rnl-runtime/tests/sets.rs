//! Cycles of sets through ADD-TO and REMOVE-FROM (5.2.2). Whether a set is lazy (10.3) once
//! followed the sources of its edits round such a cycle without end.

use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::host::{Dialect, HostDocument, Instance, Level, LevelKind};
use rnl_runtime::records::{package_lines, scan_run};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{interpret, Options};

/// The errors of R1 n1 n2 with `statements` written in the main file.
fn errors(statements: &str) -> Vec<String> {
    let mut sink = Sink::new();
    let mut main = Level::new(LevelKind::File, "sets.sp", "sets.sp", "sets.sp");
    main.circuit = true;
    main.instances.push(Instance {
        name: "R1".into(),
        letter: Some('R'),
        nodes: vec!["n1".into(), "n2".into()],
        value: Some("1k".into()),
        file: "sets.sp".into(),
        line: 1,
        order: 0,
        default_terminals: Some(vec!["p".into(), "n".into()]),
        ..Instance::default()
    });
    let records = format!("@RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\n@RNL {{ {statements} }}");
    main.records = scan_run(&package_lines(&records), "sets.sp", &mut sink);
    let dialect = Dialect { name: "spice".into(), case_sensitive: false, ground_aliases: vec!["gnd".into()] };
    let r = interpret(HostDocument::new(main, dialect), &MemoryProvider::new(), &Options { std: false }, sink);
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect()
}

#[test]
fn a_cycle_of_sets_of_nets_without_a_removal_is_interpreted() {
    let shown = errors("%a = {&n1}; %b = {&n2}; %a @ADD-TO %b; %b @ADD-TO %a; %a @ROUTE");
    assert!(shown.is_empty(), "{shown:?}");
}

#[test]
fn a_cycle_of_sets_of_nets_with_a_removal_fails() {
    let shown = errors("%a = {&n1}; %b = {&n2}; %a @ADD-TO %b; %b @REMOVE-FROM %a; %a @ROUTE");
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert!(shown[0].contains("a cycle of sets with removals has no definite result"), "{shown:?}");
}
