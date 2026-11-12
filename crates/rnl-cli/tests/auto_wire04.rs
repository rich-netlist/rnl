//! 0.4 default auto wires use the same complete-route geometry in every runtime consumer.
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::{source::MemoryProvider, Interpretation, Options};
use std::path::Path;

const SYMBOL: &str = r#"$S = @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal=1,direction="up"); %q=(0,0) @PIN(terminal=1,direction="right") }; #R1 @SET-SYMBOL $S => @PLACE(0,0)"#;
fn file(version: &str, body: &str) -> String {
    format!("* auto\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n.end\n")
}
fn read(version: &str, body: &str) -> Interpretation {
    let text = file(version, body);
    let files = MemoryProvider::new().with("t.sp", &text);
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
    r
}
fn paths(r: &Interpretation) -> Vec<Vec<(f64, f64)>> {
    r.model.levels[0].wires.iter().map(|w| w.points.clone()).collect()
}

#[test]
fn last_bound_terminal_named_pin_and_pin_position_are_distinct() {
    let r = read(
        "0.4",
        &format!(
            r#"{SYMBOL}; &a @WIRE {{#R1:1 (4,3)}}; &a @WIRE {{#R1::p (4,3)}}; &a @WIRE {{#R1::p::position (4,3)}}"#
        ),
    );
    let p = paths(&r);
    assert_eq!(p[0], [(0., 0.), (4., 0.), (4., 3.)], "terminal uses last bound right pin");
    assert_eq!(p[1], [(0., 0.), (0., 3.), (4., 3.)], "named p uses up pin");
    assert_eq!(p[2], [(0., 0.), (4., 0.), (4., 3.)], "position is undirected even where pin is up");
}

#[test]
fn owner_orientation_is_applied_to_pin_direction() {
    let body = r#"$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1,direction="right") }; #R1 @SET-SYMBOL $S => @PLACE(0,0) => @ORIENT("R90"); &a @WIRE {#R1:1 (4,3)}"#;
    assert_eq!(paths(&read("0.4", body))[0], [(0., 0.), (0., 3.), (4., 3.)]);
}

#[test]
fn old_versions_and_strict_manhattan_are_unchanged() {
    let simple = r#"&a @WIRE {(0,0) (4,3)}"#;
    assert_eq!(paths(&read("0.2", simple))[0], [(0., 0.), (4., 3.)]);
    assert_eq!(paths(&read("0.3", simple))[0], [(0., 0.), (4., 3.)]);
    assert_eq!(paths(&read("0.4", simple))[0], [(0., 0.), (4., 0.), (4., 3.)]);
    assert_eq!(
        paths(&read("0.4", r#"&a @WIRE(mode="manhattan") {(0,0) (4,3)}"#))[0],
        [(0., 0.), (2., 0.), (2., 3.), (4., 3.)]
    );
}

#[test]
fn partially_placed_runs_keep_only_actual_wire_endpoint_directions() {
    let r = read("0.4", &format!(r#"{SYMBOL}; &a @WIRE {{#R1::p (4,3) (_,_) (8,3) (8,7)}}"#));
    let p = paths(&r);
    assert_eq!(p, vec![vec![(0., 0.), (0., 3.), (4., 3.)], vec![(8., 3.), (8., 7.)]]);
    assert!(r.model.levels[0].flylines.iter().any(|f| f.a == (4., 3.) && f.b == (8., 3.)));
}

#[test]
fn coincident_distinct_waypoints_remain_in_via_and_do_not_reverse() {
    let r = read("0.4", r#"%p=(0,0); %q=(0,0); &a @WIRE {%p %q (4,3)}"#);
    let w = &r.model.levels[0].wires[0];
    assert_eq!(w.via.len(), 3, "source vertices stay distinct");
    assert_eq!(w.points, [(0., 0.), (0., 0.), (4., 0.), (4., 3.)]);
}

#[test]
fn source_file_version_not_target_level_controls_the_effective_wire() {
    for (main, child, expected) in
        [("0.4", "0.3", vec![(0., 0.), (4., 0.), (4., 3.)]), ("0.3", "0.4", vec![(0., 0.), (4., 3.)])]
    {
        let text=format!("* main\n* @RNL(type=\"header\") {{ @VERSION(\"{main}\") }}\n.include child.sp\n* @RNL(scope=\"cell\") {{ &a @WIRE {{(0,0) (4,3)}} }}\n.end\n");
        let inc = format!(
            "* child\n* @RNL(type=\"header\") {{ @VERSION(\"{child}\") }}\n.subckt cell a b\nR1 a b 1k\n.ends cell\n"
        );
        let files = MemoryProvider::new().with("main.sp", &text).with("child.sp", &inc);
        let r =
            interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
        assert_eq!(r.model.levels.iter().find(|l| l.title == "cell").unwrap().wires[0].points, expected);
        assert_eq!(r.problems.iter().find(|p| p.name == "cell").unwrap().wires[0].default_auto, main == "0.4");
    }
}

#[test]
fn previous_generated_rewrite_uses_the_same_04_auto_path() {
    let text = "* prev\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a b 1k\n\
        * @RNL { %w = &a @WIRE {(0,0) (4,3)} }\n\
        * @RNL(type=\"generated\",priority=\"high\") { %w @WIRE {(0,0) (4,3)} }\n.end\n";
    let files = MemoryProvider::new().with("prev.sp", text);
    let (r, previous) =
        rnl_view::read_previous("prev.sp", None, &files, &Options { std: false, ..Options::default() }).unwrap();
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
    let previous = previous[0].as_ref().unwrap();
    assert_eq!(previous.wire_routes.len(), 1);
    assert_eq!(previous.wire_routes[0].points, [(0., 0.), (4., 0.), (4., 3.)]);
}

#[test]
fn effective_rewrite_source_version_controls_problem_marker() {
    for (main, child, want) in [("0.4", "0.3", true), ("0.3", "0.4", false)] {
        let text = format!("* main\n* @RNL(type=\"header\") {{ @VERSION(\"{main}\") }}\n.include child.sp\n* @RNL(scope=\"cell\") {{ %w @WIRE {{(0,0) (4,3)}} }}\n.end\n");
        let child = format!("* child\n* @RNL(type=\"header\") {{ @VERSION(\"{child}\") }}\n.subckt cell a b\nR1 a b 1k\n* @RNL {{ %w = &a @WIRE {{(0,0) (4,3)}} }}\n.ends cell\n");
        let files = MemoryProvider::new().with("main.sp", &text).with("child.sp", &child);
        let r =
            interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
        assert_eq!(r.problems.iter().find(|p| p.name == "cell").unwrap().wires[0].default_auto, want);
    }
}

#[test]
fn rejected_rewrite_keeps_earlier_wire_mode_and_problem_marker() {
    let body = r#"$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1,direction="up") }; #R1 @SET-SYMBOL $S => @PLACE(0,0); %w=&a @WIRE {#R1:1 (4,3)}; %w @WIRE(mode="straight") {#R1:2 (4,3)}"#;
    let text = file("0.4", body);
    let files = MemoryProvider::new().with("t.sp", &text);
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(
        r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::WIRE_REWRITE_NO_PIN),
        "{:?}",
        r.diagnostics()
    );
    let wire = &r.problems[0].wires[0];
    assert_eq!(wire.mode, "auto");
    assert!(wire.default_auto);
    assert_eq!(paths(&r)[0], [(0., 0.), (0., 3.), (4., 3.)]);
}
