//! The whole file's time, `--timeall`: the levels of a time limit share it, each its share of what
//! is left, and only its seed once it is spent.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::process::Command;

use rnl_cli::progress::{solve_progress_using, Event};
use rnl_cli::solve::{BackendResult, SolveOptions};
use rnl_runtime::source::MemoryProvider;
use rnl_solver::{Clock, RunLimit, SolverOptions, StoppedClock};

const LEVEL: &str = r#"* @RNL {
* $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1,direction="left"); (2,0) @PIN(terminal=2,direction="right"); (-1,-1) (1,-1) (1,1) (-1,1) (-1,-1) @LINE };
* #R1 @SET-SYMBOL $S;
* }"#;

/// Three levels: two subcircuits and the file.
fn source() -> String {
    source_of("0.4")
}

fn source_of(version: &str) -> String {
    format!(
        "* timeall\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n.subckt one a b\nR1 a b 1k\n{LEVEL}\n.ends one\n\
         .subckt two a b\nR1 a b 1k\n{LEVEL}\n.ends two\nR1 in 0 1k\n{LEVEL}\n.end\n"
    )
}

/// The limit each level of a solve was given, what its `level` event told, and its hash.
type Solve = (Vec<RunLimit>, Vec<(String, Option<f64>)>, Vec<String>);

/// A clock whose hands move only when a level is solved.
struct Hands(Cell<f64>);

impl Clock for Hands {
    fn check_every(&self) -> u64 {
        1
    }
    fn elapsed(&self) -> f64 {
        self.0.get()
    }
}

/// The run limit each level was given and the limit and seconds its event told, each level taking
/// `each` seconds on the clock.
fn solve(total: Option<f64>, solver: SolverOptions, each: f64) -> (Vec<RunLimit>, Vec<(String, Option<f64>)>) {
    let (given, told, _) = solve_text(&source(), total, solver, each);
    (given, told)
}

/// [`solve`] of `text`, with the hash of each level.
fn solve_text(text: &str, total: Option<f64>, solver: SolverOptions, each: f64) -> Solve {
    let files = MemoryProvider::new().with("/w/main.sp", text);
    let options = SolveOptions { std: false, solver, total_seconds: total, ..Default::default() };
    let hands = Hands(Cell::new(100.));
    let given = RefCell::new(Vec::new());
    let told = RefCell::new(Vec::new());
    let sink = |e: &Event| {
        if let Event::Level { limit, seconds, .. } = e {
            told.borrow_mut().push((limit.to_string(), *seconds));
        }
        true
    };
    let run = solve_progress_using(Path::new("/w/main.sp"), &files, &options, &hands, &sink, &|p, options, _| {
        given.borrow_mut().push(rnl_solver::run_limit(&options.for_problem(p).0, &hands));
        hands.0.set(hands.0.get() + each);
        Ok(BackendResult {
            solved: rnl_solver::solve_with_clock(p, options, &StoppedClock),
            author: rnl_solver::AUTHOR.to_string(),
            report_available: true,
        })
    })
    .unwrap();
    assert_eq!(run.levels.len(), 3);
    let hashes = run.levels.iter().map(|l| l.hash.clone()).collect();
    (given.into_inner(), told.into_inner(), hashes)
}

#[test]
fn the_levels_of_a_time_limit_share_the_whole_files_time_and_then_have_their_seed() {
    // 9 seconds for three levels of 10 each: 3, then 2 of the 4 left, then nothing
    let (given, told) = solve(Some(9.), SolverOptions::default(), 5.);
    assert_eq!(given, [RunLimit::Time(3.), RunLimit::Time(2.), RunLimit::Seed]);
    assert_eq!(told, [("time".to_string(), Some(3.)), ("time".to_string(), Some(2.)), ("seed".to_string(), None)]);
    // what one level leaves goes to those after it
    let (given, _) = solve(Some(9.), SolverOptions::default(), 1.);
    assert_eq!(given, [RunLimit::Time(3.), RunLimit::Time(4.), RunLimit::Time(7.)]);
    // the share is under the tool's own seconds, which each level applies again
    let twenty = SolverOptions::default().with_config(r#"limit="time",seconds=20"#).unwrap();
    let (given, _) = solve(Some(9.), twenty, 5.);
    assert_eq!(given, [RunLimit::Time(3.), RunLimit::Time(2.), RunLimit::Seed]);
}

#[test]
fn a_level_keeps_its_own_limit_when_its_share_is_more_or_its_limit_is_not_of_time() {
    let ten = [RunLimit::Time(10.); 3];
    let (given, told) = solve(Some(60.), SolverOptions::default(), 5.);
    assert_eq!(given, ten);
    assert!(told.iter().all(|(limit, seconds)| limit == "time" && *seconds == Some(10.)), "{told:?}");
    let (given, _) = solve(None, SolverOptions::default(), 5.);
    assert_eq!(given, ten);
    let work = SolverOptions::default().with_config(r#"limit="work",work=1000"#).unwrap();
    let (given, _) = solve(Some(1.), work, 5.);
    assert_eq!(given, [RunLimit::Work(1000); 3]);
    // a share in whole milliseconds
    let (given, _) = solve(Some(1.), SolverOptions::default(), 0.);
    assert_eq!(given, [RunLimit::Time(0.333), RunLimit::Time(0.5), RunLimit::Time(1.)]);
}

#[test]
fn a_level_before_0_4_does_not_share_and_its_hash_does_not_change() {
    let text = source_of("0.3");
    let (given, _, hashes) = solve_text(&text, Some(9.), SolverOptions::default(), 5.);
    assert_eq!(given, [RunLimit::Time(10.); 3]);
    let (_, _, alone) = solve_text(&text, None, SolverOptions::default(), 5.);
    assert_eq!(hashes, alone);
}

#[test]
fn timeall_is_an_option_of_solve_and_preview_with_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("main.sp");
    std::fs::write(&file, source()).unwrap();
    let file = file.to_str().unwrap();
    let out = dir.path().join("out");
    let out = out.to_str().unwrap();
    let rnl = |args: &[&str]| {
        let o = Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).output().unwrap();
        (o.status.code(), String::from_utf8_lossy(&o.stderr).into_owned())
    };
    let (code, err) =
        rnl(&["solve", file, "--no-std", "-o", out, "--timeall", "30", "--solver-config", r#"limit="seed""#]);
    assert_eq!(code, Some(0), "{err}");
    for seconds in ["0", "-1", "inf", "x"] {
        let (code, err) = rnl(&["solve", file, "-o", out, "--timeall", seconds]);
        assert_ne!(code, Some(0));
        assert!(err.contains("--timeall needs positive finite seconds"), "{err}");
    }
    let (code, err) = rnl(&["problem", file, "--timeall", "5"]);
    assert_ne!(code, Some(0));
    assert!(err.contains("--timeall is not an option of problem"), "{err}");
    let (_, err) = rnl(&["solve", file, "-o", out, "--respace", "--timeall", "5"]);
    assert!(err.contains("--respace searches nothing, so it takes no --timeall"), "{err}");
    let (_, err) = rnl(&["solve", file, "--check", "--timeall", "5"]);
    assert!(err.contains("takes no --timeall"), "{err}");
}
