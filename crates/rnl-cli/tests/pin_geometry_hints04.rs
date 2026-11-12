//! Drawing connectivity uses physical pins, independently of the spelling of a reference.
use std::path::Path;

use rnl_runtime::{diag::Severity, source::MemoryProvider, Interpretation, Options};
use rnl_view::hints::{interpretation_hints, Hint, Kind};

fn read(devices: &str, body: &str, generated: &str) -> Interpretation {
    let text = format!(
        "* pin hints\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n{devices}\n* @RNL {{\n* {}\n* }}\n{generated}.end\n",
        body.replace('\n', "\n* ")
    );
    let r = rnl_cli::interpret_file(
        Path::new("pins.sp"),
        None,
        &MemoryProvider::new().with("pins.sp", &text),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    r
}

fn hints(r: &Interpretation, kind: Kind) -> Vec<Hint> {
    interpretation_hints(r).into_iter().flat_map(|l| l.hints).filter(|h| h.kind == kind).collect()
}

const SYMBOL: &str = r#"
    $S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal=1, direction="left") => %alias };
    #R1 @SET-SYMBOL $S => @PLACE(0,0) => @ORIENT("R0");
"#;

#[test]
fn aliases_and_terminal_references_join_the_same_pin() {
    for reference in ["#R1::p", "#R1::alias", "#R1:1"] {
        let r = read("R1 a b 1k", &format!("{SYMBOL} &a @WIRE {{ {reference} (-3,0) }}"), "");
        assert!(hints(&r, Kind::SilentOpen).is_empty(), "{reference}: {:?}", hints(&r, Kind::SilentOpen));
        assert!(hints(&r, Kind::WrongExit).is_empty());
    }
}

#[test]
fn a_pin_position_or_coordinates_still_warn_and_offer_a_valid_pin_reference() {
    for reference in ["#R1::p::position", "(0,0)"] {
        let r = read("R1 a b 1k", &format!("{SYMBOL} &a @WIRE {{ {reference} (-3,0) }}"), "");
        let hs = hints(&r, Kind::SilentOpen);
        assert_eq!(hs.len(), 1, "{hs:?}");
        assert!(hs[0].message.contains("write #R1::p in the wire"), "{hs:?}");
    }
}

#[test]
fn named_pin_direction_is_checked_at_its_real_position() {
    let r = read("R1 a b 1k", &format!("{SYMBOL} &a @WIRE {{ #R1::p (3,0) }}"), "");
    assert_eq!(hints(&r, Kind::WrongExit).len(), 1);
    assert!(hints(&r, Kind::SilentOpen).is_empty());
}

#[test]
fn distinct_pins_of_one_terminal_do_not_join_wires() {
    let r = read(
        "R1 a b 1k",
        r#"
        $S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal=1); %q = (0,4) @PIN(terminal=1) };
        #R1 @SET-SYMBOL $S => @PLACE(0,0);
        &a @WIRE {#R1::p (-2,0) (-2,2)};
        &a @WIRE {#R1::q (-2,4) (-2,2)}
    "#,
        "",
    );
    let hs = hints(&r, Kind::SilentOpen);
    assert!(!hs.is_empty() && hs.iter().all(|h| h.message.contains("(-2, 2)")), "{hs:?}");
}

#[test]
fn terminal_spelling_connects_only_the_last_bound_pin() {
    let r = read(
        "R1 a b 1k",
        r#"
        $S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal=1); %q = (0,4) @PIN(terminal=1) };
        #R1 @SET-SYMBOL $S => @PLACE(0,0);
        &a @WIRE {#R1::p (-2,0) (-2,2)};
        &a @WIRE {#R1:1 (-2,4) (-2,2)}
    "#,
        "",
    );
    let hs = hints(&r, Kind::SilentOpen);
    assert!(!hs.is_empty() && hs.iter().all(|h| h.message.contains("(-2, 2)")), "{hs:?}");
}

#[test]
fn bundle_pin_members_connect_by_the_symbol_pin() {
    let r = read(
        ".subckt cell d q\n.ends\nX0 a0 b0 cell\nX1 a1 b1 cell",
        r#"
        $S = @DEFINE-SYMBOL { %input = (-3,0) @PIN(terminal="d"); %output = (3,0) @PIN(terminal="q") };
        {#X0 #X1} @BUNDLE => @SET-SYMBOL $S => %bundle;
        %bundle @PLACE(0,0);
        {&a0 &a1} @WIRE {(-6,0) %bundle::input};
        {&a0 &a1} @WIRE {(-3,3) %bundle:1};
        {&b0 &b1} @WIRE {%bundle::output (6,0)}
    "#,
        "",
    );
    assert!(hints(&r, Kind::SilentOpen).is_empty(), "{:?}", hints(&r, Kind::SilentOpen));
}

#[test]
fn expanded_usage_pin_connections_survive_solver_writeback() {
    let body = r#"
        $S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal=1) };
        $MARK = @DEFINE(roles=[["n","net"]]) {};
        $M = @DEFINE-SYMBOL { %pin = (0,0) @PIN(terminal="n") };
        $STUB = @DEFINE(roles=[["t","pin"]]) {
            %mark = $MARK @ATTACH ?t::net;
            %mark @SET-SYMBOL $M;
            ?t::net @WIRE {?t %mark::pin}
        };
        #R1 @SET-SYMBOL $S => @PLACE(0,0);
        %stub = #R1::p @AS $STUB;
        %stub::mark @PLACE(-3,0)
    "#;
    let r = read("R1 a b 1k", body, "");
    assert!(hints(&r, Kind::SilentOpen).is_empty(), "{:?}", hints(&r, Kind::SilentOpen));
    let p = &r.problems[0];
    let solved = rnl_solver::solve(p, &rnl_solver::SolverOptions::without_time_limit());
    assert!(solved.report.failed.is_empty() && solved.report.discarded.is_empty(), "{:?}", solved.report);
    let written = rnl_runtime::solution::write_record(p, &solved.solution, "test", "");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    let generated = written.record.lines().map(|s| format!("* {s}\n")).collect::<String>();
    let reread = read("R1 a b 1k", body, &generated);
    assert!(hints(&reread, Kind::SilentOpen).is_empty(), "{:?}", hints(&reread, Kind::SilentOpen));
}
