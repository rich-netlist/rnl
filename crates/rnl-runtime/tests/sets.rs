//! Cycles of sets through ADD-TO and REMOVE-FROM (5.2.2). Whether a set is lazy (10.3) once
//! followed the sources of its edits round such a cycle without end: in one level, through a set
//! added to itself, and through edits that only make a cycle when the scenes they are made in are
//! ignored.

use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::host::{Dialect, HostDocument, Instance, Level, LevelKind};
use rnl_runtime::records::{package_lines, scan_run};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{interpret, Options};

fn resistor(name: &str, a: &str, b: &str, file: &str) -> Instance {
    Instance {
        name: name.into(),
        letter: Some('R'),
        nodes: vec![a.into(), b.into()],
        value: Some("1k".into()),
        file: file.into(),
        line: 1,
        order: 0,
        default_terminals: Some(vec!["p".into(), "n".into()]),
        ..Instance::default()
    }
}

/// The errors of R1 n1 n2 with `statements` written in the main file, and of the subcircuits
/// (name, ports, statements), each of R1 between its two ports and instantiated once.
fn errors_in(statements: &str, subckts: &[(&str, [&str; 2], &str)]) -> Vec<String> {
    let mut sink = Sink::new();
    let mut main = Level::new(LevelKind::File, "sets.sp", "sets.sp", "sets.sp");
    main.circuit = true;
    main.instances.push(resistor("R1", "n1", "n2", "sets.sp"));
    for (k, (name, _, _)) in subckts.iter().enumerate() {
        main.instances.push(Instance {
            name: format!("X{k}"),
            letter: Some('X'),
            nodes: vec![format!("a{k}"), format!("b{k}")],
            model: Some(name.to_string()),
            file: "sets.sp".into(),
            line: 2,
            order: k + 1,
            is_subckt: true,
            ..Instance::default()
        });
    }
    let records = format!("@RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\n@RNL {{ {statements} }}");
    main.records = scan_run(&package_lines(&records), "sets.sp", &mut sink);
    let dialect = Dialect { name: "spice".into(), case_sensitive: false, ground_aliases: vec!["gnd".into()] };
    let mut doc = HostDocument::new(main, dialect);
    for &(name, [a, b], statements) in subckts {
        let mut lv = Level::new(LevelKind::Subckt, name, "sets.sp", "sets.sp");
        lv.circuit = true;
        lv.ports = vec![a.to_string(), b.to_string()];
        lv.instances.push(resistor("R1", a, b, "sets.sp"));
        lv.records = scan_run(&package_lines(&format!("@RNL {{ {statements} }}")), "sets.sp", &mut sink);
        let id = doc.add_level(lv, Some(0));
        doc.subckts.insert(doc.key(name), id);
    }
    let r = interpret(doc, &MemoryProvider::new(), &Options { std: false }, sink);
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect()
}

fn errors(statements: &str) -> Vec<String> {
    errors_in(statements, &[])
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

#[test]
fn a_set_added_to_itself_is_interpreted() {
    let shown = errors("%a = {&n1}; %a @ADD-TO %a; %a @ROUTE");
    assert!(shown.is_empty(), "{shown:?}");
}

#[test]
fn edits_of_package_sets_in_two_levels_are_not_a_cycle() {
    // one level adds $SUPPLIES to $RAILS, the other $RAILS to $SUPPLIES: each level sees its own
    let shown = errors_in(
        "@USE-PACKAGE(name=\"house\") { $RAILS = {}; $SUPPLIES = {} }; &n1 @ADD-TO $SUPPLIES; $SUPPLIES @ROUTE",
        &[
            ("core", ["vdd", "out"], "$SUPPLIES @ADD-TO $RAILS; $RAILS @ROUTE"),
            ("pad", ["vio", "x"], "&vio @ADD-TO $RAILS; $RAILS @ADD-TO $SUPPLIES; $SUPPLIES @ROUTE"),
        ],
    );
    assert!(shown.is_empty(), "{shown:?}");
}
