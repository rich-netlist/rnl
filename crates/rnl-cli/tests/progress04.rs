//! The progress of a solve as events (docs/solver-progress.md): their order, that observing
//! them changes nothing written, cancelling, and `rnl solve --progress=json`.
use rnl_cli::progress::{solve_progress, Event};
use rnl_cli::solve::{solve_with_clock, SolveOptions, CANCELLED};
use rnl_runtime::source::MemoryProvider;
use rnl_solver::{SolverOptions, StoppedClock};
use std::{cell::RefCell, path::Path, process::Command};

const SOURCE: &str = r#"* progress
* @RNL(type="header") { @VERSION("0.4") }
R1 in mid 1k
R2 mid out 2k
R3 out 0 3k
* @RNL {
* $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1,direction="left"); (2,0) @PIN(terminal=2,direction="right"); (-1,-1) (1,-1) (1,1) (-1,1) (-1,-1) @LINE };
* {#R1 #R2 #R3} @SET-SYMBOL $S;
* {#R1 #R2} @ALIGN("y");
* &mid @ROUTE-NET; &out @ROUTE-NET;
* }
.end
"#;

fn options() -> SolveOptions {
    let solver = SolverOptions::default().with_config("limit=\"work\",work=2000000000").unwrap();
    SolveOptions { std: false, solver, ..Default::default() }
}

#[test]
fn events_come_in_order_and_change_nothing_written() {
    let files = MemoryProvider::new().with("main.sp", SOURCE);
    let plain = solve_with_clock(Path::new("main.sp"), &files, &options(), &StoppedClock).unwrap();
    let events = RefCell::new(Vec::new());
    let run = solve_progress(Path::new("main.sp"), &files, &options(), &StoppedClock, &|e| {
        events.borrow_mut().push(e.clone());
        true
    })
    .unwrap();
    assert_eq!(plain.files, run.files, "observing the solve changes nothing written");
    let events = events.into_inner();
    assert_eq!(events[0], Event::Run { progress: 1, levels: 1 });
    match &events[1] {
        Event::Level { level: 0, title, limit: "work", work: Some(2000000000), seconds: None, .. } => {
            assert_eq!(title, "main.sp")
        }
        e => panic!("{e:?}"),
    }
    assert!(matches!(&events[2], Event::Stage { stage, .. } if stage == "resolve"), "{:?}", events[2]);
    let budget = run.levels[0].solved.report.budget.unwrap();
    match events.last().unwrap() {
        Event::LevelDone { level: 0, status, work, kept, .. } => {
            assert!(["complete", "work"].contains(status), "{status}");
            assert_eq!(*work, budget.work);
            assert_eq!(*kept, run.levels[0].solved.report.kept.len());
        }
        e => panic!("{e:?}"),
    }
    assert!(events.iter().any(|e| matches!(e, Event::Stage { stage, .. } if stage == "write")));
    // tried never goes down; each best is told once it is taken
    let mut tried = 0;
    for e in &events {
        if let Event::Work { tried: t, .. } | Event::Best { tried: t, .. } = e {
            assert!(*t >= tried, "{events:?}");
            tried = *t;
        }
    }
    assert!(events.iter().any(|e| matches!(e, Event::Best { cost, .. } if !cost.is_empty())), "{events:?}");
}

#[test]
fn a_callback_that_says_no_cancels_the_solve() {
    let files = MemoryProvider::new().with("main.sp", SOURCE);
    for at in ["level", "stage"] {
        let result = solve_progress(Path::new("main.sp"), &files, &options(), &StoppedClock, &|e| match e {
            Event::Level { .. } => at != "level",
            Event::Stage { .. } => at != "stage",
            _ => true,
        });
        assert_eq!(result.err().as_deref(), Some(CANCELLED), "cancelled at its first {at}");
    }
}

#[test]
fn the_command_writes_one_event_per_line_on_the_standard_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.sp"), SOURCE).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rnl"))
        .current_dir(dir.path())
        .args(["solve", "main.sp", "-o", "out", "--no-std", "--progress=json"])
        .args(["--solver-config", "limit=\"work\",work=2000000000"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stderr = String::from_utf8(output.stderr).unwrap();
    let events: Vec<serde_json::Value> =
        stderr.lines().filter(|l| l.starts_with('{')).map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(events[0]["event"], "run");
    assert_eq!(events[1]["event"], "level");
    let done = events.last().unwrap();
    assert_eq!(
        (done["event"].as_str(), done["exit"].as_u64(), done["written"].as_u64()),
        (Some("done"), Some(0), Some(1))
    );
    assert!(!String::from_utf8(output.stdout).unwrap().contains("\"event\""), "the standard output keeps the report");
}
