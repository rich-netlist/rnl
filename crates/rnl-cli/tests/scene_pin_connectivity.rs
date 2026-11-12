//! Physical pins and shared point objects determine 0.4 connectivity (5.2.4, 13.1).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::{PointRef, Reference, RouteIntent};
use rnl_runtime::solution::{write_record, RoutedWire, Solution, WirePoint, WireRoute};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn source(version: &str, body: &str, generated: &str) -> String {
    let body = body.replace('\n', "\n* ");
    let generated = generated.lines().map(|line| format!("* {line}\n")).collect::<String>();
    format!("* connectivity\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n{generated}.end\n")
}

fn interpret(text: &str) -> Interpretation {
    let r = interpret_file(
        Path::new("pins.sp"),
        None,
        &MemoryProvider::new().with("pins.sp", text),
        &Options { std: false, problems: true, ..Options::default() },
    )
    .unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    r
}

fn parts(r: &Interpretation) -> usize {
    r.model.levels[0].net_labels.iter().filter(|label| label.net == "a").count()
}

#[test]
fn a_pin_position_stays_disconnected_after_wire_writeback() {
    for (name, point) in [("%p =", "#R1::p::position"), ("%p =", "#R1:1"), ("", "#R1:1"), ("", "#R1:1::position")] {
        let body = format!(
            r#"
            $S = @DEFINE-SYMBOL {{ {name} (-3,0) @PIN(terminal=1) }};
            #R1 @SET-SYMBOL $S => @PLACE(0,0) => @ORIENT("R0");
            $D = @DEFINE(roles=[["p","point"],["n","net"]]) {{ %end = (0,2); %w = ?n @WIRE {{?p %end}} }};
            %u = {point} &a @AS $D
        "#
        );
        let r = interpret(&source("0.4", &body, ""));
        assert_eq!(parts(&r), 2);
        let problem = &r.problems[0];
        let anchors = &problem.wires[0].anchors;
        assert!(matches!(anchors[0], PointRef::PinPosition { .. }));
        assert_eq!(anchors[0].pin_key(), None);
        let solution = Solution {
            wire_routes: vec![WireRoute {
                wire: 0,
                points: vec![
                    WirePoint::Anchor(anchors[0].clone()),
                    WirePoint::Bend((-3.0, 2.0)),
                    WirePoint::Anchor(anchors[1].clone()),
                ],
            }],
            ..Solution::default()
        };
        let written = write_record(problem, &solution, "test", "");
        assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
        let reread = interpret(&source("0.4", &body, &written.record));
        assert_eq!(parts(&reread), 2, "{}", written.record);
        assert_eq!(reread.model.levels[0].wires[0].points[0], (-3.0, 0.0));
    }
}

#[test]
fn solving_preserves_a_pin_position_and_its_owner_transform() {
    let body = r#"
        $S = @DEFINE-SYMBOL { %p = (-3,0) @PIN(terminal=1,direction="left") };
        #R1 @SET-SYMBOL $S => @PLACE(5,2) => @ORIENT("R90");
        %end = (0,4); %w = &a @WIRE {#R1::p::position %end}
    "#;
    let r = interpret(&source("0.4", body, ""));
    let p = &r.problems[0];
    let solved = rnl_solver::solve(p, &rnl_solver::SolverOptions::without_time_limit());
    assert!(solved.report.failed.is_empty() && solved.report.discarded.is_empty(), "{:?}", solved.report);
    let written = write_record(p, &solved.solution, "test", "");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    let reread = interpret(&source("0.4", body, &written.record));
    assert_eq!(parts(&reread), 2);
    assert_eq!(reread.model.levels[0].wires[0].points[0], (5.0, -1.0));
}

#[test]
fn every_independent_pin_is_present_in_network_connectivity() {
    let r = interpret(&source(
        "0.4",
        r#"
        $S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal=1); %q = (4,0) @PIN(terminal=1) };
        #R1 @SET-SYMBOL $S => @PLACE(0,0)
    "#,
        "",
    ));
    assert_eq!(parts(&r), 2);
    assert_eq!(r.model.levels[0].flylines.len(), 1);
    assert_eq!(r.model.levels[0].flylines[0].a, (0.0, 0.0));
    assert_eq!(r.model.levels[0].flylines[0].b, (4.0, 0.0));
}

#[test]
fn coincident_pins_need_an_explicit_wire_to_connect() {
    for (wire, expected) in [("", 2), ("&a @WIRE {#R1::p #R1::q};", 1)] {
        let body = format!(
            r#"
            $S = @DEFINE-SYMBOL {{ %p = (0,0) @PIN(terminal=1); %q = (0,0) @PIN(terminal=1) }};
            #R1 @SET-SYMBOL $S => @PLACE(0,0); {wire}
        "#
        );
        assert_eq!(parts(&interpret(&source("0.4", &body, ""))), expected);
    }
}

#[test]
fn a_terminal_and_names_of_its_last_pin_connect_to_one_physical_pin() {
    let r = interpret(&source(
        "0.4",
        r#"
        $S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal=1) => %q };
        #R1 @SET-SYMBOL $S => @PLACE(0,0);
        &a @WIRE {#R1:1 (2,0)};
        &a @WIRE {#R1::p (0,2)};
        &a @WIRE {#R1::q (-2,0)}
    "#,
        "",
    ));
    assert_eq!(parts(&r), 1);
}

#[test]
fn equal_coordinates_do_not_connect_distinct_points_in_0_4() {
    let separate = "&a @WIRE {(0,0) (1,0)}; &a @WIRE {(1,0) (2,0)}";
    for (version, expected) in [("0.2", 1), ("0.3", 1), ("0.4", 2)] {
        assert_eq!(parts(&interpret(&source(version, separate, ""))), expected, "{version}");
    }
    let shared = "%junction = (1,0); &a @WIRE {(0,0) %junction}; &a @WIRE {%junction (2,0)}";
    assert_eq!(parts(&interpret(&source("0.4", shared, ""))), 1);
}

#[test]
fn the_position_of_a_pin_does_not_connect_the_pin() {
    let r = interpret(&source(
        "0.4",
        r#"
        $S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal=1) };
        #R1 @SET-SYMBOL $S => @PLACE(0,0);
        &a @WIRE {#R1::p::position (2,0)}
    "#,
        "",
    ));
    assert_eq!(parts(&r), 2);
}

#[test]
fn usage_pins_on_a_role_network_are_all_present() {
    let r = interpret(&source(
        "0.4",
        r#"
        $D = @DEFINE(roles=["n"]) {};
        $S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal="n"); %q = (4,0) @PIN(terminal="n") };
        %u = &a @AS $D;
        %u @SET-SYMBOL $S => @PLACE(0,0)
    "#,
        "",
    ));
    assert_eq!(parts(&r), 2);
}

#[test]
fn solver_writeback_reaches_an_earlier_pin_by_its_quoted_name() {
    for name in [r#""first pin""#, r#""p""q""#] {
        let body = format!(
            r#"
            $S = @DEFINE-SYMBOL {{ %{name} = (-3,0) @PIN(terminal=1); (3,0) @PIN(terminal=1) }};
            #R1 @SET-SYMBOL $S => @PLACE(0,0) => @ORIENT("R0")
        "#
        );
        let r = interpret(&source("0.4", &body, ""));
        let mut p = r.problems[0].clone();
        let pin = p.nets.iter().find(|net| net.name == "a").unwrap().pins[0].clone();
        assert!(matches!(&pin, PointRef::SymbolPin { pin: 0, reference: Reference::Name(n), .. }
            if n == &format!("#R1::{name}")));
        p.routes.push(RouteIntent {
            tree: false,
            net: "a".into(),
            mode: "straight".into(),
            style: vec![],
            pins: vec![pin.clone()],
            wires: vec![],
        });
        let solution = Solution {
            wires: vec![RoutedWire { route: 0, points: vec![WirePoint::Anchor(pin), WirePoint::Bend((-6.0, 0.0))] }],
            ..Solution::default()
        };
        let written = write_record(&p, &solution, "test", "");
        assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
        assert!(written.record.contains(&format!("#R1::{name}")), "{}", written.record);
        let reread = interpret(&source("0.4", &body, &written.record));
        assert_eq!(reread.model.levels[0].wires[0].points[0], (-3.0, 0.0));
    }
}
