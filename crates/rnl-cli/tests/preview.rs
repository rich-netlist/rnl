//! The solve that the desktop viewer previews (decision 0010): run in memory and drawn, nothing
//! written; its report, each fact located at its line in the texts drawn; a copy saved as one file
//! or as a folder, never over a file that was read; and a solve stopped by its watcher or cut by
//! the time limit.

use std::path::{Path, PathBuf};

use rnl_cli::preview::{one_file, preview, save_copy, OVER_A_FILE_READ};
use rnl_cli::solve::{Progress, SolveOptions, WallClock, CANCELLED};
use rnl_cli::FsProvider;
use rnl_solver::SolverOptions;

fn unlimited() -> SolveOptions {
    SolveOptions { solver: SolverOptions::without_time_limit(), ..SolveOptions::default() }
}

/// A fresh directory of its own under the tests' temporary directory.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-preview").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn path(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// A clock that advances one second each time it is read.
#[derive(Default)]
struct Ticks(std::cell::Cell<u64>);

impl rnl_solver::Clock for Ticks {
    fn elapsed(&self) -> f64 {
        let t = self.0.get();
        self.0.set(t + 1);
        t as f64
    }
}

/// Two resistors placed apart on x, and a high ALIGN on x that the solver keeps first; of the two
/// PLACEs, the later is kept (8.5), and the x of R1 is discarded, with the ALIGN in its conflict.
const NET: &str = "* preview\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a b 1k\nR2 b 0 1k\n\
                   * @RNL { #R1 @PLACE(0, 0) }\n\
                   * @RNL { {#R1 #R2} @ALIGN(\"x\", priority=\"high\") }\n\
                   * @RNL { #R2 @PLACE(4, 5) }\n.end\n";

#[test]
fn the_preview_writes_nothing_and_draws_the_files_as_solved() {
    let dir = scratch("draws");
    let file = dir.join("t.sp");
    std::fs::write(&file, NET).unwrap();
    let watched = std::cell::RefCell::new(Vec::new());
    let watch = |p: &Progress| {
        watched.borrow_mut().push(p.clone());
        true
    };
    let shown = preview(&path(&file), &FsProvider::new(&[]), &unlimited(), &WallClock::new(), &watch).unwrap();
    // nothing on disk changed, nor was any file made
    assert_eq!(std::fs::read_to_string(&file).unwrap(), NET);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    // the Document holds the texts as solved: the record of the solver, drawn
    let text = &shown.document.files[0].text;
    assert!(text.contains("@RNL(type=\"generated\""), "{text}");
    assert_eq!(shown.document.levels.len(), 1);
    let progress: Vec<(usize, usize, String)> =
        watched.borrow().iter().map(|p| (p.level, p.levels, p.title.clone())).collect();
    assert_eq!(progress, [(0, 1, "t.sp".to_string())]);
    // the report: every fact, the one discarded with its conflict, each at its line in the text
    let report = &shown.report;
    assert!(report.one_file && report.notes.is_empty(), "{:?}", report.notes);
    let [level] = report.levels.as_slice() else { panic!("{:?}", report.levels) };
    assert_eq!(level.title, "t.sp");
    let lines: Vec<&str> = text.lines().collect();
    let line_of = |needle: &str| lines.iter().position(|l| l.contains(needle)).unwrap() as u32 + 1;
    let statuses: Vec<(&str, u32)> = level.facts.iter().map(|f| (f.status, f.at.map_or(0, |a| a.line))).collect();
    assert!(statuses.contains(&("discarded", line_of("#R1 @PLACE(0, 0)"))), "{statuses:?}");
    assert!(statuses.contains(&("kept", line_of("@ALIGN"))), "{statuses:?}");
    assert!(statuses.contains(&("kept", line_of("#R2 @PLACE(4, 5)"))), "{statuses:?}");
    let discarded = level.facts.iter().find(|f| f.status == "discarded").unwrap();
    assert!(discarded.why.as_deref().is_some_and(|w| w.starts_with("it conflicts with")), "{discarded:?}");
    // a fact of a statement in the netlist is in its file; one that a rule of the standard
    // package makes, such as the stub of a ground, is written in no file of the Document
    let written = level.facts.iter().filter(|f| !f.text.contains(".rnl:"));
    assert!(written.clone().count() >= 4 && written.clone().all(|f| f.at.is_some_and(|a| a.file_index == 0)));
    assert!(level.facts.iter().filter(|f| f.text.contains(".rnl:")).all(|f| f.at.is_none()), "{:?}", level.facts);
    assert!(!level.statements.is_empty() && level.statements.iter().all(|s| s.at.is_some()), "{:?}", level.statements);
    assert!(report.text.iter().any(|l| l.contains("discarded")), "{:?}", report.text);
}

#[test]
fn a_copy_of_one_file_is_saved_anywhere_but_over_the_file() {
    let dir = scratch("one");
    let file = dir.join("t.sp");
    std::fs::write(&file, NET).unwrap();
    let shown = preview(&path(&file), &FsProvider::new(&[]), &unlimited(), &WallClock::new(), &|_| true).unwrap();
    assert!(one_file(&shown.copy));
    // over the file itself, by another spelling of its path too: refused, nothing written
    assert_eq!(save_copy(&shown.copy, &file).unwrap_err(), OVER_A_FILE_READ);
    assert_eq!(save_copy(&shown.copy, &dir.join(".").join("t.sp")).unwrap_err(), OVER_A_FILE_READ);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), NET);
    // elsewhere, the text the solve wrote, which reads back solved
    let copy = dir.join("copies").join("t-solved.sp");
    let saved = save_copy(&shown.copy, &copy).unwrap();
    assert_eq!(saved.written, std::slice::from_ref(&copy));
    assert_eq!(std::fs::read_to_string(&copy).unwrap(), shown.copy.files[0].text);
    // solved again, it writes the same, but for its hash, which covers the name of the file
    let again = preview(&path(&copy), &FsProvider::new(&[]), &unlimited(), &WallClock::new(), &|_| true).unwrap();
    let unhashed = |t: &str| t.lines().map(|l| l.split(" hash=").next().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!(unhashed(&again.copy.files[0].text), unhashed(&shown.copy.files[0].text));
}

#[test]
fn a_copy_of_files_that_include_others_is_a_folder_and_never_the_files_read() {
    let dir = scratch("folder");
    let src = dir.join("src");
    std::fs::create_dir_all(src.join("lib")).unwrap();
    let top = "* top\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n.include lib/div.inc\n.include lib/plain.inc\n\
               X1 a 0 div\nR1 a 0 1k\n.end\n";
    let div = "* div\n.subckt div p n\nR1 p m 1k\nR2 m n 1k\n.ends div\n";
    let plain = "* nothing to draw\n.param r=1k\n";
    std::fs::write(src.join("top.sp"), top).unwrap();
    std::fs::write(src.join("lib").join("div.inc"), div).unwrap();
    std::fs::write(src.join("lib").join("plain.inc"), plain).unwrap();
    let entry = src.join("top.sp");
    let shown = preview(&path(&entry), &FsProvider::new(&[]), &unlimited(), &WallClock::new(), &|_| true).unwrap();
    assert!(!one_file(&shown.copy) && !shown.report.one_file);
    assert_eq!(shown.report.files, ["top.sp", "lib/div.inc"]);
    // into the folder of the files read, or one below it that holds them: refused
    assert_eq!(save_copy(&shown.copy, &src).unwrap_err(), OVER_A_FILE_READ);
    assert_eq!(std::fs::read_to_string(src.join("top.sp")).unwrap(), top);
    assert_eq!(std::fs::read_to_string(src.join("lib").join("div.inc")).unwrap(), div);
    // into another folder: the files written, and the other file read, in their places
    let out = dir.join("out");
    let saved = save_copy(&shown.copy, &out).unwrap();
    assert_eq!(saved.written, [out.join("top.sp"), out.join("lib/div.inc")]);
    assert_eq!(saved.copied, [out.join("lib/plain.inc")]);
    assert_eq!(std::fs::read_to_string(out.join("lib").join("plain.inc")).unwrap(), plain);
    let again =
        preview(&path(&out.join("top.sp")), &FsProvider::new(&[]), &unlimited(), &WallClock::new(), &|_| true).unwrap();
    for (a, b) in again.copy.files.iter().zip(&shown.copy.files) {
        assert_eq!(a.text, b.text);
    }
}

#[test]
fn a_solve_stops_when_its_watcher_says_so() {
    let dir = scratch("stop");
    let file = dir.join("t.sp");
    let text = "* stop\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n.subckt a p n\nR1 p n 1k\n.ends\n\
                .subckt b p n\nR1 p n 1k\n.ends\nX1 1 0 a\nX2 1 0 b\n.end\n";
    std::fs::write(&file, text).unwrap();
    // at once, and after the first level: nothing comes back but the word that it stopped
    for stop_at in [0, 1] {
        let watch = |p: &Progress| p.level < stop_at;
        let stopped = preview(&path(&file), &FsProvider::new(&[]), &unlimited(), &WallClock::new(), &watch);
        assert_eq!(stopped.unwrap_err(), CANCELLED, "{stop_at}");
    }
    assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
}

#[test]
fn the_time_limit_leaves_facts_undecided_in_the_report() {
    // with no time at all, on a clock that ticks at each reading, the facts that need a check are
    // discarded undecided (8.5), and the report says so
    let dir = scratch("limit");
    let file = dir.join("t.sp");
    std::fs::write(&file, NET).unwrap();
    let options =
        SolveOptions { solver: SolverOptions { time_limit: Some(0.0), ..SolverOptions::default() }, ..unlimited() };
    let shown = preview(&path(&file), &FsProvider::new(&[]), &options, &Ticks::default(), &|_| true).unwrap();
    let level = &shown.report.levels[0];
    let undecided: Vec<&str> =
        level.facts.iter().filter(|f| f.status == "undecided").map(|f| f.text.as_str()).collect();
    assert!(!undecided.is_empty(), "{:?}", level.facts);
    assert!(level
        .facts
        .iter()
        .filter(|f| f.status == "undecided")
        .all(|f| f.why.as_deref().is_some_and(|w| w.contains("time limit"))));
    assert!(!level.warnings.is_empty(), "{level:?}");
}

#[test]
fn a_stopped_clock_is_past_the_time_limit() {
    // cancelled while a level is being solved, the clock is past the limit: the facts still to be
    // checked are discarded undecided (8.5); before, it runs as the wall clock
    use rnl_cli::preview::{Clock, StoppableClock};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    let stop = Arc::new(AtomicBool::new(false));
    let clock = StoppableClock::new(stop.clone());
    assert!(clock.elapsed() < 60.0);
    stop.store(true, Ordering::Relaxed);
    assert!(clock.elapsed() > 1e9);
    let dir = scratch("stopped");
    let file = dir.join("t.sp");
    std::fs::write(&file, NET).unwrap();
    let options = SolveOptions::default();
    let shown = preview(&path(&file), &FsProvider::new(&[]), &options, &clock, &|_| true).unwrap();
    assert!(shown.report.levels[0].facts.iter().any(|f| f.status == "undecided"), "{:?}", shown.report.levels[0].facts);
}
