//! Respacing (`rnl solve --respace`): a level as its record of the solver last drew it, spaced
//! again, nothing searched; a level without a record that fits the netlist as it is now is left
//! as it is, with the reason.
use rnl_cli::respace::respace;
use rnl_cli::solve::{solve_with_clock, SolveOptions};
use rnl_runtime::source::MemoryProvider;
use rnl_solver::{SolverOptions, StoppedClock};
use std::path::Path;

const SOURCE: &str = r#"* respace
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(layout="lines") }
R1 in a 1k
R2 a b 1k
R3 b out 1k
R4 out 0 1k
* @RNL {
* $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1,direction="left"); (2,0) @PIN(terminal=2,direction="right"); (-1,-1) (1,-1) (1,1) (-1,1) (-1,-1) @LINE };
* {#R1 #R2 #R3 #R4} @SET-SYMBOL $S;
* &a @ROUTE-NET; &b @ROUTE-NET; &out @ROUTE-NET;
* }
.end
"#;

fn options(config: &str) -> SolveOptions {
    SolveOptions { std: false, solver: SolverOptions::default().with_config(config).unwrap(), ..Default::default() }
}

fn places(text: &str) -> Vec<String> {
    text.lines().filter(|l| l.contains("@PLACE(")).map(str::to_string).collect()
}

fn solved() -> String {
    let files = MemoryProvider::new().with("main.sp", SOURCE);
    let run = solve_with_clock(Path::new("main.sp"), &files, &options("limit=\"seed\""), &StoppedClock).unwrap();
    run.files[0].text.clone()
}

#[test]
fn a_level_is_spaced_again_as_its_record_draws_it() {
    let before = solved();
    let files = MemoryProvider::new().with("main.sp", &before);
    let config = "spacing_x=4,limit=\"work\",work=100000000000";
    let run = respace(Path::new("main.sp"), &files, &options(config), &StoppedClock).unwrap();
    assert!(run.not_solved.is_empty(), "{:?}", run.not_solved);
    let after = &run.files[0].text;
    assert_ne!(places(&before), places(after), "spacing_x=4 moves the resistors apart");
    let orients = |t: &str| {
        t.lines().flat_map(|l| l.match_indices("@ORIENT(").map(|(i, _)| l[i..].to_string())).collect::<Vec<_>>()
    };
    assert_eq!(orients(&before), orients(after), "nothing is turned");
    let level = &run.levels[0];
    assert!(level.solved.report.notes.iter().any(|n| n.starts_with("respaced from the previous result")));
    // the same record and settings give the same file
    let again = respace(Path::new("main.sp"), &files, &options(config), &StoppedClock).unwrap();
    assert_eq!(after, &again.files[0].text);
}

#[test]
fn a_level_without_a_record_that_fits_is_left_as_it_is() {
    let files = MemoryProvider::new().with("main.sp", SOURCE);
    let run = respace(Path::new("main.sp"), &files, &options("spacing_x=4"), &StoppedClock).unwrap();
    assert!(run.levels.is_empty() && run.files.is_empty());
    assert!(run.not_solved[0].reason.starts_with("no-generated: "), "{:?}", run.not_solved);
    let edited = solved().replace("R2 a b 1k", "R2 a b 2k\nR5 b 0 1k");
    let files = MemoryProvider::new().with("main.sp", &edited);
    let run = respace(Path::new("main.sp"), &files, &options("spacing_x=4"), &StoppedClock).unwrap();
    assert!(run.levels.is_empty());
    assert!(run.not_solved[0].reason.starts_with("mismatch: "), "{:?}", run.not_solved);
}

#[test]
fn a_level_before_04_is_not_respaced() {
    let old = SOURCE.replace("@VERSION(\"0.4\"); @SOLVER-CONFIG(layout=\"lines\")", "@VERSION(\"0.3\")");
    let files = MemoryProvider::new().with("main.sp", &old);
    let run = respace(Path::new("main.sp"), &files, &options("spacing_x=4"), &StoppedClock).unwrap();
    assert!(run.levels.is_empty() && run.files.is_empty());
    assert!(run.not_solved[0].reason.starts_with("unsupported: "), "{:?}", run.not_solved);
}

#[test]
fn the_preview_says_respace_and_draws_the_level_spaced_again() {
    let before = solved();
    let files = MemoryProvider::new().with("main.sp", &before);
    let config = "spacing_x=4,limit=\"work\",work=100000000000";
    let shown = rnl_cli::preview::respace("main.sp", &files, &options(config), &StoppedClock, &|_| true).unwrap();
    assert_eq!(shown.report.mode, "respace");
    // as many files as `solve --respace` writes
    let run = respace(Path::new("main.sp"), &files, &options(config), &StoppedClock).unwrap();
    assert_eq!(shown.report.files.len(), run.files.len());
    // a solve's preview says solve
    let plain = MemoryProvider::new().with("main.sp", SOURCE);
    let solved = rnl_cli::preview::preview("main.sp", &plain, &options("limit=\"seed\""), &StoppedClock, &|_| true);
    assert_eq!(solved.unwrap().report.mode, "solve");
}
