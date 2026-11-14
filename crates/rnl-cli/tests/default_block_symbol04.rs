//! The default block symbol (9.2.6): an instance that no rule, statement or block symbol gives a
//! symbol has a square box centred on its origin, with a pin for each port of the subcircuit or
//! module the file reads, or for each node; it is drawn like any other, so what is written about
//! its drawing takes effect (9.2.8).
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(lines: &str, body: &str) -> Interpretation {
    let source =
        format!("* boxes\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n{lines}* @RNL {{ {body} }}\n.end\n");
    interpret_file(
        Path::new("t.sp"),
        None,
        &MemoryProvider::new().with("t.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect()
}

/// The main level's problem.
fn top(r: &Interpretation) -> &LevelProblem {
    r.problems.iter().find(|p| p.title == "t.sp").expect("the main level")
}

/// The box and the pins, as (terminals, position, direction, label), of the symbol of `object`.
#[allow(clippy::type_complexity)]
fn symbol_of(p: &LevelProblem, object: &str) -> ([f64; 4], Vec<(String, (f64, f64), u16, String)>) {
    let k = p.ids.objects.iter().position(|o| o == object).unwrap_or_else(|| panic!("no {object}"));
    let symbol = &p.symbols[p.objects[k].symbol.expect("a symbol")];
    assert!(symbol.default, "{object} has the default block symbol");
    let pins = symbol
        .pins
        .iter()
        .map(|pin| (pin.terminals.join(","), pin.pos, pin.dir.unwrap_or(999), pin.label.clone().unwrap_or_default()))
        .collect();
    (symbol.bbox, pins)
}

#[test]
fn the_pins_of_an_instance_of_a_cell_not_read_are_its_nodes_by_number() {
    let r = read("X1 a b c pdk_cell\nR1 p q 1k\nV1 r 0 1\n", "");
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let p = top(&r);
    // 3 nodes: 2 on the left, from 1 below the top corner down; 1 on the right, centred
    let (bbox, pins) = symbol_of(p, "#X1");
    assert_eq!(bbox, [-2.0, -2.0, 2.0, 2.0]);
    assert_eq!(
        pins,
        [
            ("1".to_string(), (-2.0, 1.0), 180, "1".to_string()),
            ("2".to_string(), (-2.0, -1.0), 180, "2".to_string()),
            ("3".to_string(), (2.0, 0.0), 0, "3".to_string())
        ]
    );
    // 2 nodes: a box of side 2, a pin in the middle of each side
    let (bbox, pins) = symbol_of(p, "#R1");
    assert_eq!(bbox, [-1.0, -1.0, 1.0, 1.0]);
    assert_eq!(pins.iter().map(|q| (q.1, q.2)).collect::<Vec<_>>(), [((-1.0, 0.0), 180), ((1.0, 0.0), 0)]);
}

#[test]
fn the_pins_of_an_instance_of_a_subcircuit_read_are_its_ports_by_name() {
    let lines = ".subckt amp inp inn out vdd vss\nR1 inp out 1k\n.ends\nX1 a b c d e amp\n";
    let r = read(lines, "");
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    // 5 ports: 3 on the left, 2 on the right; a box of side 6
    let (bbox, pins) = symbol_of(top(&r), "#X1");
    assert_eq!(bbox, [-3.0, -3.0, 3.0, 3.0]);
    let got: Vec<(&str, (f64, f64), &str)> = pins.iter().map(|q| (q.0.as_str(), q.1, q.3.as_str())).collect();
    assert_eq!(
        got,
        [
            ("inp", (-3.0, 2.0), "inp"),
            ("inn", (-3.0, 0.0), "inn"),
            ("out", (-3.0, -2.0), "out"),
            ("vdd", (3.0, 1.0), "vdd"),
            ("vss", (3.0, -1.0), "vss")
        ]
    );
}

#[test]
fn a_symbol_given_to_the_instance_replaces_the_default_block_symbol() {
    let r = read(
        "R1 a b 1k\n",
        "$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); (4,0) @PIN(terminal=2) }; #R1 @SET-SYMBOL $S",
    );
    let p = top(&r);
    let k = p.ids.objects.iter().position(|o| o == "#R1").unwrap();
    assert!(!p.symbols[p.objects[k].symbol.unwrap()].default);
}

#[test]
fn what_is_written_about_its_drawing_takes_effect() {
    // a device whose kind no rule tells: placed, turned, wired and labelled like any other
    let lines = "M1 d g s b missing\nR1 g 0 1k\n";
    let body =
        "$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); (4,0) @PIN(terminal=2) }; #R1 @SET-SYMBOL $S => @PLACE(10,0); \
                #M1 @PLACE(0,0); #M1 @ORIENT(\"R90\"); &g {#M1:2 #R1:1} @WIRE; #M1:2 @TEXT(\"gate\")";
    let r = read(lines, body);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let level = r.model.levels.iter().find(|l| l.title == "t.sp").unwrap();
    assert!(level.undrawn.is_empty());
    assert_eq!(level.wires.len(), 1);
    assert!(level.texts.iter().any(|t| format!("{:?}", t.markup).contains("gate")));
    // its texts: the name above the box and the model below
    let labels: Vec<&str> = level.texts.iter().map(|t| t.object_id.as_str()).collect();
    assert!(labels.contains(&"#M1::name") && labels.contains(&"#M1::model"), "{labels:?}");
}

#[test]
fn a_terminal_of_the_default_block_symbol_is_a_pin_for_selections() {
    let r = read("X1 a b c pdk_cell\n", "$P = @DEFINE(roles=[\"x\"]) {}; #X1 @SELECT-PIN => @AS $P");
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(top(&r).usages.iter().filter(|u| u.definition == "$P").count(), 3);
}
