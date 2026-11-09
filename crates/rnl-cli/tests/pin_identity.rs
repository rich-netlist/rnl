//! Independent symbol pins and exact solver endpoints in language 0.4 (9.2.4).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::problem::{input_digest, FactKind, PointRef, Reference};
use rnl_runtime::solution::{write_record, RoutedWire, Solution, WirePoint, WireRoute};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(version: &str, body: &str) -> Interpretation {
    let text = format!(
        "* pins\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n.end\n"
    );
    let files = MemoryProvider::new().with("pins.sp", &text);
    interpret_file(Path::new("pins.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}

fn no_errors(r: &Interpretation) {
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
}

const TWO: &str = "$S = @DEFINE-SYMBOL { (-3,0) @PIN(terminal=1); (3,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S => @PLACE(0,0) => @ORIENT(\"R0\"); &a @WIRE(mode=\"straight\") {#R1:1 (6,0)}";

#[test]
fn every_pin_remains_but_a_terminal_uses_its_last_pin() {
    for version in ["0.2", "0.3", "0.4"] {
        let r = read(version, TWO);
        no_errors(&r);
        let lv = &r.model.levels[0];
        let item = &lv.items[0];
        let count = if version == "0.4" { 2 } else { 1 };
        assert_eq!(lv.symbols[item.symbol].pins.len(), count);
        assert_eq!(item.pin_nets, vec![vec!["a"]; count]);
        let info = lv.objects.iter().find(|o| o.id == "#R1").unwrap();
        assert_eq!(info.pins.iter().map(|p| &p.nets).collect::<Vec<_>>(), vec![&vec!["&a".to_string()]; count]);
        assert_eq!(lv.wires[0].points[0], (3.0, 0.0));
        let p = &r.problems[0];
        let net = p.nets.iter().find(|n| n.name == "a").unwrap();
        assert_eq!(net.pins.len(), count);
        assert_eq!(p.wires[0].anchors[0].pin_key(), Some((0, count - 1)));
        if version == "0.4" {
            assert!(matches!(net.pins[0], PointRef::SymbolPin { pin: 0, reference: Reference::Anonymous, .. }));
        }
    }
}

#[test]
fn a_bus_pin_keeps_all_bindings_and_an_exact_reachable_terminal() {
    let r = read("0.4", "$BASE = @DEFINE-SYMBOL { (-3,0) @PIN(terminal=[1,2,1]) }; $S = @DEFINE-SYMBOL { $BASE; (3,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S => @PLACE(0,0)");
    no_errors(&r);
    let p = &r.problems[0];
    assert_eq!(p.symbols[0].pins[0].terminals, ["1", "2", "1"]);
    assert_eq!(p.nets.iter().find(|n| n.name == "a").unwrap().pins.len(), 2);
    let early = &p.nets.iter().find(|n| n.name == "a").unwrap().pins[0];
    assert!(matches!(early, PointRef::Pin { pin: 0, terminal, .. } if terminal == "n"));
    let lv = &r.model.levels[0];
    assert_eq!(lv.items[0].pin_nets, [vec!["a", "b"], vec!["a"]]);
}

#[test]
fn unbound_pins_are_drawn_and_stay_off_all_nets() {
    let r = read("0.4", "$S = @DEFINE-SYMBOL { (0,0) @PIN(label=\"figure\"); (2,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S => @PLACE(0,0)");
    no_errors(&r);
    let lv = &r.model.levels[0];
    assert_eq!(lv.symbols[lv.items[0].symbol].pins.len(), 2);
    assert_eq!(lv.items[0].pin_nets, [Vec::<String>::new(), vec!["a".into()]]);
    assert!(r.problems[0].nets.iter().flat_map(|n| &n.pins).all(|p| p.pin_key() != Some((0, 0))));
    for version in ["0.2", "0.3"] {
        let old = read(version, "$S = @DEFINE-SYMBOL { (0,0) @PIN }");
        assert!(old.diagnostics().iter().any(|d| d.severity == Severity::Error));
    }
    let empty = read("0.4", "$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=[]) }");
    assert!(empty.diagnostics().iter().any(|d| d.code == codes::PARAM_PIN_TERMINAL_EMPTY));
}

#[test]
fn coincident_pins_and_repeated_inclusions_keep_separate_identities() {
    let r = read("0.4", "$B = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; $S = @DEFINE-SYMBOL { $B; $B }; #R1 @SET-SYMBOL $S => @PLACE(0,0)");
    no_errors(&r);
    let p = &r.problems[0];
    let pins = &p.nets.iter().find(|n| n.name == "a").unwrap().pins;
    assert_eq!(pins.iter().map(PointRef::pin_key).collect::<Vec<_>>(), [Some((0, 0)), Some((0, 1))]);
    assert_eq!(p.symbols[0].pins[0].pos, p.symbols[0].pins[1].pos);
    let mut changed = p.clone();
    changed.nets[0].pins.remove(0);
    assert_ne!(input_digest(p), input_digest(&changed));
}

#[test]
fn spice_case_matching_does_not_remove_either_pin() {
    let r = read("0.4", "$S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"P\"); (2,0) @PIN(terminal=\"p\") }; #R1 @SET-SYMBOL $S => @PLACE(0,0); &a @WIRE(mode=\"straight\") {#R1:p (4,0)}");
    no_errors(&r);
    let lv = &r.model.levels[0];
    assert_eq!(lv.items[0].pin_nets, [vec!["a"], vec!["a"]]);
    assert_eq!(lv.wires[0].points[0], (2.0, 0.0));
    assert_eq!(r.problems[0].symbols[0].pins[0].terminals, ["P"]);
}

#[test]
fn usages_bind_role_names_only_and_report_unmatched_bindings() {
    let r = read("0.4", "$S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1); (0,0) @PIN(terminal=\"p\"); (2,0) @PIN(terminal=\"P\"); (3,0) @PIN }; $U = @DEFINE(roles=[\"p\"]) {}; %u = &a @AS $U => @SET-SYMBOL $S => @PLACE(10,0)");
    no_errors(&r);
    let lv = &r.model.levels[0];
    let u = lv.items.iter().find(|i| i.id == "%u").unwrap();
    assert_eq!(u.pin_nets, [vec![], vec!["a".to_string()], vec![], vec![]]);
    for code in [codes::PIN_NUMBER_BINDS_NO_ROLE, codes::PIN_TERMINAL_NAMES_NO_ROLE] {
        let ds: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == code).collect();
        assert_eq!(ds.len(), 1, "{:?}", r.diagnostics());
        assert!(ds[0].message.contains("%u"));
        assert!(ds[0].loc.is_some());
    }
}

#[test]
fn unmatched_instance_bindings_warn_without_losing_the_pin_or_valid_bus_members() {
    let r =
        read("0.4", "$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=[1,9,\"absent\"]) }; #R1 @SET-SYMBOL $S => @PLACE(0,0)");
    no_errors(&r);
    let ds: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::PIN_TERMINAL_NOT_IN_OWNER).collect();
    assert_eq!(ds.len(), 2);
    assert!(ds.iter().all(|d| d.message.contains("pin 1 of $S") && d.message.contains("#R1")));
    assert_eq!(r.model.levels[0].items[0].pin_nets, [vec!["a"]]);
}

#[test]
fn solver_geometry_uses_the_pin_index_and_writeback_never_substitutes_a_later_pin() {
    let r = read("0.4", TWO);
    no_errors(&r);
    let mut p = r.problems[0].clone();
    let early = p.nets.iter().find(|n| n.name == "a").unwrap().pins[0].clone();
    let mut fact = p.facts[0].clone();
    fact.kind = FactKind::Place { at: early.clone(), axis: rnl_runtime::problem::Axis::X, value: -3.0 };
    fact.label = "earlier pin at x=-3".into();
    p.facts.push(fact);
    let solved = rnl_solver::solve(&p, &rnl_solver::SolverOptions::without_time_limit());
    assert!(solved.report.failed.is_empty(), "{:?}", solved.report);
    assert!(solved.report.discarded.is_empty(), "{:?}", solved.report);
    let solution = Solution {
        wire_routes: vec![WireRoute { wire: 0, points: vec![WirePoint::Anchor(early), WirePoint::Bend((6.0, 0.0))] }],
        ..Solution::default()
    };
    let written = write_record(&p, &solution, "test", "");
    assert_eq!(written.unsolved.len(), 1);
    assert_eq!(written.unsolved[0].code, codes::UNSOLVED_WIRE_POINT_UNREACHABLE);
    let solution = Solution {
        wire_routes: vec![WireRoute {
            wire: 0,
            points: vec![WirePoint::Anchor(p.wires[0].anchors[0].clone()), WirePoint::Bend((6.0, 0.0))],
        }],
        ..Solution::default()
    };
    assert!(write_record(&p, &solution, "test", "").unsolved.is_empty());
}

#[test]
fn source_file_version_controls_pins_in_scoped_symbol_definitions() {
    for (outer, inner, count) in [("0.4", "0.3", 2), ("0.3", "0.4", 1)] {
        let files = MemoryProvider::new()
            .with("t.sp", &format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"{outer}\") }}\n.include \"inc.sp\"\n* @RNL(scope=\"child\") {{ $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1); (2,0) @PIN(terminal=1) }}; #R1 @SET-SYMBOL $S => @PLACE(0,0) }}\n.end\n"))
            .with("inc.sp", &format!("* inc\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a\nR1 a 0 1k\n.ends\n"));
        let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        no_errors(&r);
        let child = r.model.levels.iter().find(|l| l.items.iter().any(|i| i.id == "#R1")).unwrap();
        assert_eq!(child.symbols[child.items[0].symbol].pins.len(), count);
    }
}

#[test]
fn overridden_symbols_do_not_emit_unmatched_binding_warnings() {
    let r = read("0.4", "$BAD = @DEFINE-SYMBOL { (0,0) @PIN(terminal=9) }; $GOOD = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $BAD; #R1 @SET-SYMBOL $GOOD => @PLACE(0,0)");
    no_errors(&r);
    assert!(!r.diagnostics().iter().any(|d| d.code == codes::PIN_TERMINAL_NOT_IN_OWNER));
}

#[test]
fn an_earlier_bus_pin_written_through_its_other_terminal_reads_back_at_that_pin() {
    let body = "$S = @DEFINE-SYMBOL { (-3,0) @PIN(terminal=[1,2]); (3,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S => @PLACE(0,0) => @ORIENT(\"R0\")";
    let r = read("0.4", body);
    no_errors(&r);
    let mut p = r.problems[0].clone();
    let early = p.nets.iter().find(|n| n.name == "a").unwrap().pins[0].clone();
    p.routes.push(rnl_runtime::problem::RouteIntent {
        net: "a".into(),
        mode: "straight".into(),
        style: vec![],
        pins: vec![early.clone()],
        wires: vec![],
    });
    let solution = Solution {
        wires: vec![RoutedWire { route: 0, points: vec![WirePoint::Anchor(early), WirePoint::Bend((-6.0, 0.0))] }],
        ..Solution::default()
    };
    let written = write_record(&p, &solution, "test", "");
    assert!(written.unsolved.is_empty());
    assert!(written.record.contains("#R1:n"), "{}", written.record);
    let source = format!(
        "* pins\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n{}\n.end\n",
        written.record.lines().map(|l| format!("* {l}\n")).collect::<String>()
    );
    let files = MemoryProvider::new().with("pins.sp", &source);
    let viewed =
        interpret_file(Path::new("pins.sp"), None, &files, &Options { std: false, ..Options::default() }).unwrap();
    no_errors(&viewed);
    assert_eq!(viewed.model.levels[0].wires[0].points[0], (-3.0, 0.0));
}
