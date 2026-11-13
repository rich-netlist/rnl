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

#[test]
fn a_smaller_spacing_leaves_the_drawing_as_it_is() {
    let before = solved();
    let files = MemoryProvider::new().with("main.sp", &before);
    let wide = respace(Path::new("main.sp"), &files, &options("spacing_x=4"), &StoppedClock).unwrap();
    let wide = wide.files[0].text.clone();
    assert_ne!(places(&before), places(&wide));
    // from the wider file, a smaller spacing and none at all widen nothing
    let files = MemoryProvider::new().with("main.sp", &wide);
    for config in ["spacing_x=1", "spacing_x=0", "spacing_x=4"] {
        let run = respace(Path::new("main.sp"), &files, &options(config), &StoppedClock).unwrap();
        assert_eq!(places(&wide), places(&run.files[0].text), "{config}");
    }
}

#[test]
fn a_respace_that_its_limit_stops_is_an_error_and_nothing_is_written() {
    let files = MemoryProvider::new().with("main.sp", &solved());
    let config = "spacing_x=4,limit=\"work\",work=2000";
    let Err(e) = respace(Path::new("main.sp"), &files, &options(config), &StoppedClock) else {
        panic!("a limit of 2000 units of work stops the respace");
    };
    assert!(e.contains("is not respaced, and nothing is written") && e.contains("limit"), "{e}");
}

#[test]
fn a_record_that_breaks_a_written_fact_is_an_error() {
    // R2 is written where it is; the record is then edited to put it elsewhere
    let fixed = SOURCE.replace("* &a @ROUTE-NET;", "* #R2 @PLACE(5, 3);\n* &a @ROUTE-NET;");
    let files = MemoryProvider::new().with("main.sp", &fixed);
    let run = solve_with_clock(Path::new("main.sp"), &files, &options("limit=\"seed\""), &StoppedClock).unwrap();
    let text = run.files[0].text.clone();
    // the record as the solver wrote it is respaced
    let files = MemoryProvider::new().with("main.sp", &text);
    let run = respace(Path::new("main.sp"), &files, &options("spacing_x=4"), &StoppedClock).unwrap();
    assert!(run.not_solved.is_empty() && run.files.len() == 1, "{:?}", run.not_solved);
    // R2 moved in the record alone: the input digest is the same, the written fact no longer holds
    let generated = text.rfind("type=\"generated\"").expect("a record of the solver");
    let was = "#R2 @PLACE(5, 3)";
    let at = generated + text[generated..].find(was).expect("R2 in the record, where it is written");
    let edited = format!("{}#R2 @PLACE(31, 17){}", &text[..at], &text[at + was.len()..]);
    let files = MemoryProvider::new().with("main.sp", &edited);
    let Err(e) = respace(Path::new("main.sp"), &files, &options("spacing_x=4"), &StoppedClock) else {
        panic!("the record breaks the written PLACE");
    };
    // a PLACE is a fact on x and one on y
    assert!(e.contains("does not keep 2 facts"), "{e}");
}

#[test]
fn a_level_whose_record_is_stale_is_left_while_the_others_are_respaced() {
    let two = SOURCE.replace(
        "R1 in a 1k",
        ".subckt cell p q\nR9 p m 1k\nR8 m q 1k\n* @RNL {\n* $T = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1,direction=\"left\"); (2,0) @PIN(terminal=2,direction=\"right\"); (-1,-1) (1,-1) (1,1) (-1,1) (-1,-1) @LINE };\n* {#R9 #R8} @SET-SYMBOL $T;\n* &m @ROUTE-NET;\n* }\n.ends\nR1 in a 1k",
    );
    let files = MemoryProvider::new().with("main.sp", &two);
    let run = solve_with_clock(Path::new("main.sp"), &files, &options("limit=\"seed\""), &StoppedClock).unwrap();
    assert_eq!(run.levels.len(), 2, "{:?}", run.not_solved);
    // the subcircuit is edited after its solve; the file level is not
    let edited = run.files[0].text.replace("R8 m q 1k", "R8 m q 2k\nR7 m q 1k");
    let files = MemoryProvider::new().with("main.sp", &edited);
    let run = respace(Path::new("main.sp"), &files, &options("spacing_x=4"), &StoppedClock).unwrap();
    assert_eq!(run.levels.len(), 1);
    assert_eq!(run.not_solved.len(), 1);
    assert!(run.not_solved[0].reason.starts_with("mismatch: "), "{:?}", run.not_solved);
    // the stale level's record is written as it was
    let record = |t: &str| {
        let from = t.find(".subckt cell").unwrap();
        t[from..t.find(".ends").unwrap()].to_string()
    };
    assert_eq!(record(&edited), record(&run.files[0].text));
}
