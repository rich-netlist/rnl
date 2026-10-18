use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::host::{Dialect, HostDocument, Instance, Level, LevelKind};
use rnl_runtime::problem::{CurrentDirection, CurrentTerminal, PointRef};
use rnl_runtime::records::{package_lines, scan_run};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{interpret, Interpretation, Options};

fn read(statements: &str) -> Interpretation {
    let mut sink = Sink::new();
    let mut level = Level::new(LevelKind::File, "current.sp", "current.sp", "current.sp");
    level.circuit = true;
    level.ports = vec!["a".into(), "z".into()];
    for (order, name) in ["R1", "R2"].into_iter().enumerate() {
        level.instances.push(Instance {
            name: name.into(),
            letter: Some('R'),
            nodes: vec!["a".into(), "z".into()],
            file: "current.sp".into(),
            line: 1,
            order,
            default_terminals: Some(vec!["p".into(), "n".into()]),
            ..Instance::default()
        });
    }
    let records = format!("@RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n@RNL {{ {statements} }}");
    level.records = scan_run(&package_lines(&records), "current.sp", &mut sink);
    let dialect = Dialect { name: "spice".into(), case_sensitive: false, ground_aliases: vec![] };
    let result = interpret(
        HostDocument::new(level, dialect),
        &MemoryProvider::new(),
        &Options { std: false, ..Options::solver() },
        sink,
    );
    let errors: Vec<_> =
        result.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| result.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}");
    result
}

#[test]
fn current_is_resolved_per_semantic_terminal_before_bundle_mapping() {
    let result = read(
        r#"
        $S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=["p", "n"], direction="right") };
        {#R1 #R2} @BUNDLE => @SET-SYMBOL $S;
        #R1:p @CURRENT("in"); #R1:p @CURRENT("out");
        #R1:n @CURRENT("in"); #R2:p @CURRENT("in");
        #R2:n @CURRENT("out"); #R2:n @CURRENT("unspecified");
        #:a @CURRENT("in")
    "#,
    );
    let problem = &result.problems[0];
    assert_eq!(problem.currents.len(), 4);
    assert_eq!(problem.currents[0].terminal, CurrentTerminal::Instance { instance: 0, terminal: "p".into() });
    assert_eq!(problem.currents[1].terminal, CurrentTerminal::Instance { instance: 0, terminal: "n".into() });
    assert_eq!(problem.currents[2].terminal, CurrentTerminal::Instance { instance: 1, terminal: "p".into() });
    assert_eq!(problem.currents[3].terminal, CurrentTerminal::Port { terminal: "a".into() });
    assert_eq!(problem.currents[0].direction, CurrentDirection::Out);
    assert_eq!(problem.currents[1].direction, CurrentDirection::In);
    assert_eq!(problem.currents[0].net, "a");
    assert_eq!(problem.currents[1].net, "z");
    let physical = |i: usize| match problem.currents[i].display.as_ref().unwrap() {
        PointRef::Pin { object, pin, .. } => (*object, *pin),
        other => panic!("{other:?}"),
    };
    assert_eq!(physical(0), physical(1));
    assert_eq!(physical(1), physical(2));
    assert!(problem.currents[3].display.is_none());
    assert_eq!(problem.objects.iter().filter(|o| o.kind == rnl_runtime::problem::ObjectKind::Instance).count(), 0);
}

#[test]
fn written_direction_survives_inheritance_and_pin_replacement() {
    let result = read(
        r#"
        $BASE = @DEFINE-SYMBOL { (0, 0) @PIN(terminal="p", direction="up"); (1, 0) @PIN(terminal="n", direction="right") };
        $INHERIT = @DEFINE-SYMBOL { $BASE };
        $OVERRIDE = @DEFINE-SYMBOL { $BASE; (2, 0) @PIN(terminal="p") };
        #R1 @SET-SYMBOL $INHERIT; #R2 @SET-SYMBOL $OVERRIDE;
        #R1:p @CURRENT("out"); #R2:p @CURRENT("in")
    "#,
    );
    let p = &result.problems[0];
    let symbol = |label: &str| &p.symbols[p.objects.iter().find(|o| o.label == label).unwrap().symbol.unwrap()];
    assert!(symbol("#R1").pins.iter().all(|pin| pin.direction_written));
    let mut generated = p.clone();
    for symbol in &mut generated.symbols {
        for pin in &mut symbol.pins {
            pin.direction_written = false;
        }
    }
    assert_eq!(rnl_runtime::problem::canonical_sig1(p), rnl_runtime::problem::canonical_sig1(&generated));
    assert_eq!(rnl_runtime::problem::canonical_sig2(p), rnl_runtime::problem::canonical_sig2(&generated));
    assert_ne!(rnl_runtime::problem::canonical_sig3(p), rnl_runtime::problem::canonical_sig3(&generated));
    let replaced = symbol("#R2").pins.iter().find(|pin| pin.terminals == ["p"]).unwrap();
    assert!(!replaced.direction_written);
    assert_eq!(replaced.dir, None);
    let unmarked = read("#R1:p @CURRENT(\"out\")");
    assert!(unmarked.problems[0].symbols.iter().all(|s| s.default && s.pins.iter().all(|pin| !pin.direction_written)));
}
