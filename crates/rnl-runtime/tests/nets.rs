//! How nets are drawn without a solver: a label on a net sits on one of the net's wires, or on one
//! of its pins when no wire of it is drawn (13.3), a ROUTE draws flylines between the parts of its
//! net that its wires do not join yet (13.7), and each part of a net has a place for the net's name.

use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::host::{Dialect, HostDocument, Instance, Level, LevelKind};
use rnl_runtime::markup::Run;
use rnl_runtime::records::{package_lines, scan_run};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{interpret, Interpretation, Options};

const RECORDS: &str = r#"@RNL(type="header") { @VERSION("0.2")}
@RNL {
  $BAR = @DEFINE-SYMBOL {(0, 1) @PIN(terminal=1, direction="up"); (0, -1) @PIN(terminal=2, direction="down"); @LINE {(0, 1) (0, -1)}};
  # @SELECT-INSTANCE => @SET-SYMBOL $BAR;
  $TP = @DEFINE(roles=[["p", ["net", "terminal"]]]) {};
  $TP_SYM = @DEFINE-SYMBOL {(0, 0) @PIN(terminal="p"); (0, 0) @CIRCLE(radius=0.35)};
  # @SELECT-USAGE $TP => @SET-SYMBOL $TP_SYM;
  #R1 @PLACE(0, 0) => @ORIENT("R90"); #R2 @PLACE(4, 0); #R3 @PLACE(8, 0);
  $TP @ATTACH &c => @PLACE(8, -4);
  &c @WIRE {#R2:2 (6, -1)};
  %p = (_, _);
  &b @WIRE {%p #R2:1};
  &b @WIRE {#R1:2 #R2:1};
  &b @TEXT("on b");
  &a @TEXT("on a");
  {&b &c} @ROUTE
}"#;

/// R1 a b, R2 b c, R3 c d, with RECORDS written in the main file.
fn interpret_nets() -> Interpretation {
    interpret_records(RECORDS)
}

/// R1 a b, R2 b c, R3 c d, with `records` written in the main file.
fn interpret_records(records: &str) -> Interpretation {
    let mut sink = Sink::new();
    let mut main = Level::new(LevelKind::File, "nets.sp", "nets.sp", "nets.sp");
    main.circuit = true;
    for (k, (name, a, b)) in [("R1", "a", "b"), ("R2", "b", "c"), ("R3", "c", "d")].into_iter().enumerate() {
        main.instances.push(Instance {
            name: name.into(),
            letter: Some('R'),
            nodes: vec![a.into(), b.into()],
            value: Some("1k".into()),
            file: "nets.sp".into(),
            line: k as u32 + 1,
            order: k,
            default_terminals: Some(vec!["p".into(), "n".into()]),
            ..Instance::default()
        });
    }
    main.records = scan_run(&package_lines(records), "nets.sp", &mut sink);
    let dialect = Dialect { name: "spice".into(), case_sensitive: false, ground_aliases: vec!["gnd".into()] };
    interpret(
        HostDocument::new(main, dialect),
        &MemoryProvider::new(),
        &Options { std: false, ..Options::default() },
        sink,
    )
}

/// The ends of the flylines of a level on `net`.
fn ends_on(lv: &rnl_runtime::model::LevelView, net: &str) -> Vec<((f64, f64), (f64, f64))> {
    lv.flylines.iter().filter(|f| f.nets == [net]).map(|f| (f.a, f.b)).collect()
}

fn text_of(lines: &[Vec<Run>]) -> String {
    lines
        .iter()
        .flatten()
        .filter_map(|run| match run {
            Run::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_label_on_a_net_sits_on_its_first_drawn_wire() {
    let r = interpret_nets();
    let shown: Vec<String> = r.diagnostics().iter().map(|d| r.sink.render(d)).collect();
    assert!(shown.is_empty(), "{shown:?}");
    let lv = &r.model.levels[0];
    let anchors: Vec<(String, (f64, f64))> = lv.texts.iter().map(|t| (text_of(&t.lines), t.anchor)).collect();
    // not on the wire of c, nor on the wire of b whose point is free: at #R1:2, where the next one starts;
    // a has no wire, so its label goes on its pin #R1:1, R1 being turned R90
    assert_eq!(anchors, [("on b".to_string(), (1.0, 0.0)), ("on a".to_string(), (-1.0, 0.0))]);
}

#[test]
fn flylines_join_what_the_wires_of_a_net_leave_apart() {
    let r = interpret_nets();
    let lv = &r.model.levels[0];
    // b is wired already; on c the wire joins #R2:2 to (6, -1), and #R3:1 and the test point are
    // left, both nearest to (6, -1); a and d have one pin each
    assert_eq!(ends_on(lv, "c"), [((6.0, -1.0), (8.0, 1.0)), ((6.0, -1.0), (8.0, -4.0))]);
    assert_eq!(lv.flylines.len(), 2, "{:?}", lv.flylines);
}

#[test]
fn wires_through_junctions_leave_no_flyline() {
    // #R1:2 and #R2:1 on b are wired through two junctions, and the wire from one junction to the
    // other touches no pin: b has nothing left to join
    let wired = r#"@RNL(type="header") { @VERSION("0.3")}
@RNL {
  $BAR = @DEFINE-SYMBOL {(0, 1) @PIN(terminal=1, direction="up"); (0, -1) @PIN(terminal=2, direction="down"); @LINE {(0, 1) (0, -1)}};
  # @SELECT-INSTANCE => @SET-SYMBOL $BAR;
  #R1 @PLACE(0, 0); #R2 @PLACE(4, 0); #R3 @PLACE(8, 0);
  %j1 = (0, -3); %j2 = (4, -3);
  &b @WIRE {#R1:2 %j1};
  &b @WIRE {%j1 %j2};
  &b @WIRE {%j2 #R2:1};
  &b @ROUTE
}"#;
    let r = interpret_records(wired);
    let shown: Vec<String> = r.diagnostics().iter().map(|d| r.sink.render(d)).collect();
    assert!(shown.is_empty(), "{shown:?}");
    assert_eq!(ends_on(&r.model.levels[0], "b"), []);
    // without the wire between the junctions, the two halves are apart: a flyline joins them where
    // they are nearest, from junction to junction
    let r = interpret_records(&wired.replace("  &b @WIRE {%j1 %j2};\n", ""));
    assert_eq!(ends_on(&r.model.levels[0], "b"), [((0.0, -3.0), (4.0, -3.0))]);
}

/// The errors and warnings of an interpretation, not its hints, such as those of the objects left
/// to a solver.
fn errors_and_warnings(r: &Interpretation) -> Vec<String> {
    r.diagnostics().iter().filter(|d| d.severity != Severity::Hint).map(|d| r.sink.render(d)).collect()
}

/// The symbol $BAR with `body` after it.
fn bars(body: &str) -> String {
    format!(
        r#"@RNL(type="header") {{ @VERSION("0.3")}}
@RNL {{
  $BAR = @DEFINE-SYMBOL {{(0, 1) @PIN(terminal=1, direction="up"); (0, -1) @PIN(terminal=2, direction="down"); @LINE {{(0, 1) (0, -1)}}}};
  # @SELECT-INSTANCE => @SET-SYMBOL $BAR;
  {body}
}}"#
    )
}

#[test]
fn a_flyline_is_the_shortest_in_a_straight_line() {
    // R2:2 is at (0, -1) and R3:1 at (10, 1); the wire of c from R2:2 turns at (7, -2) and goes on
    // to (10, 6). In a straight line (7, -2) is the nearer to R3:1, though not along the axes
    let records = bars(
        "#R2 @PLACE(0, 0); #R3 @PLACE(10, 0);
  &c @WIRE(mode=\"straight\") {#R2:2 (7, -2)};
  &c @WIRE(mode=\"straight\") {(7, -2) (10, 6)}",
    );
    let r = interpret_records(&records);
    assert_eq!(errors_and_warnings(&r), Vec::<String>::new());
    // the two wires meet at (7, -2): one part, which one flyline joins to R3:1
    assert_eq!(ends_on(&r.model.levels[0], "c"), [((7.0, -2.0), (10.0, 1.0))]);
}

#[test]
fn a_flyline_starts_at_a_bend_of_a_wire_as_drawn() {
    // the wire of c from R2:2 at (0, -1) to (6, 5) is drawn along x first, so it bends at (6, -1),
    // which is nearer to R3:1 at (10, 1) than the points written
    let records = bars(
        "#R2 @PLACE(0, 0); #R3 @PLACE(10, 0);
  &c @WIRE(mode=\"horizontal-vertical\") {#R2:2 (6, 5)}",
    );
    let r = interpret_records(&records);
    let lv = &r.model.levels[0];
    assert_eq!(lv.wires[0].points, [(0.0, -1.0), (6.0, -1.0), (6.0, 5.0)]);
    assert_eq!(ends_on(lv, "c"), [((6.0, -1.0), (10.0, 1.0))]);
}

#[test]
fn wires_that_meet_at_their_ends_are_one_part_and_others_are_not() {
    // on b: a wire from (0, -3) to (8, -3); one from its end down to (8, -6), which meets it; one
    // from its middle down to (4, -6), which ends on it; and one from (2, -1) to (2, -5), which
    // crosses it. R1:2 at (20, -1) and R2:1 at (30, 1) are apart
    let records = bars(
        "#R1 @PLACE(20, 0); #R2 @PLACE(30, 0);
  &b @WIRE(mode=\"straight\") {(0, -3) (8, -3)};
  &b @WIRE(mode=\"straight\") {(8, -3) (8, -6)};
  &b @WIRE(mode=\"straight\") {(4, -3) (4, -6)};
  &b @WIRE(mode=\"straight\") {(2, -1) (2, -5)}",
    );
    let r = interpret_records(&records);
    assert_eq!(errors_and_warnings(&r), Vec::<String>::new());
    let lv = &r.model.levels[0];
    // five parts, four flylines, none from a wire to a wire it meets
    let labels: Vec<(f64, f64)> = lv.net_labels.iter().filter(|l| l.net == "b").map(|l| l.at).collect();
    assert_eq!(labels.len(), 5, "{:?}", lv.net_labels);
    let lines = ends_on(lv, "b");
    assert_eq!(lines.len(), 4, "{:?}", lv.flylines);
    assert!(lines.iter().all(|(a, b)| a != b), "{lines:?}");
    // from the first two, the crossing wire is nearest, at (2, -1) as at (2, -5): the first; the
    // wire that ends on the middle of the first is nearest to the crossing one; then the pins
    assert_eq!(
        lines,
        [
            ((0.0, -3.0), (2.0, -1.0)),
            ((2.0, -5.0), (4.0, -6.0)),
            ((8.0, -3.0), (20.0, -1.0)),
            ((20.0, -1.0), (30.0, 1.0))
        ]
    );
    // the label of the first two is on the middle of the segment nearest the centre of their
    // points, (16/3, -4); the others' on their middles; a pin's on it
    assert_eq!(labels, [(4.0, -3.0), (4.0, -4.5), (2.0, -3.0), (20.0, -1.0), (30.0, 1.0)]);
    // a net with nothing drawn has no label: d is on R3, which has no position
    assert!(lv.net_labels.iter().all(|l| l.net != "d"), "{:?}", lv.net_labels);
}

#[test]
fn every_net_has_flylines_whether_a_route_asks_to_wire_it_or_not() {
    // no ROUTE: R1:2 and R2:1 on b, apart, have one flyline, and so has c between R2:2 and R3:1
    let r = interpret_records(&bars("#R1 @PLACE(0, 0); #R2 @PLACE(4, 0); #R3 @PLACE(8, 0)"));
    let lv = &r.model.levels[0];
    let lines: Vec<_> = lv.flylines.iter().map(|f| (f.nets[0].as_str(), f.a, f.b)).collect();
    assert_eq!(lines, [("b", (0.0, -1.0), (4.0, 1.0)), ("c", (4.0, -1.0), (8.0, 1.0))]);
}
