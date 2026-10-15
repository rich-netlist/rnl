//! How nets are drawn without a solver: a label on a net sits on one of the net's wires (13.3),
//! and a ROUTE draws flylines between the pins its net's wires do not join yet (13.7).

use rnl_runtime::diag::Sink;
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
    main.records = scan_run(&package_lines(RECORDS), "nets.sp", &mut sink);
    let dialect = Dialect { name: "spice".into(), case_sensitive: false, ground_aliases: vec!["gnd".into()] };
    interpret(
        HostDocument::new(main, dialect),
        &MemoryProvider::new(),
        &Options { std: false, ..Options::default() },
        sink,
    )
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
    // a has no wire, so its label is not drawn
    assert_eq!(anchors, [("on b".to_string(), (1.0, 0.0))]);
}

#[test]
fn a_route_joins_what_the_wires_of_its_net_leave_apart() {
    let r = interpret_nets();
    let lv = &r.model.levels[0];
    // b is wired already; on c the wire joins #R2:2 to (6, -1), and #R3:1 and the test point are left
    assert_eq!(lv.flylines, [((8.0, 1.0), (6.0, -1.0)), ((8.0, 1.0), (8.0, -4.0))]);
}
