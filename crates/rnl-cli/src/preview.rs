//! A solve shown before anything is written, as the desktop viewer's Solve menu runs it (decision
//! 0010): [`solve_watched`] run in memory, the texts it would write held over the files and drawn
//! as a Document, with a report of what became of each fact, every line of it located in those
//! texts; and a copy of the files solved, saved where the user says, never over a file that was
//! read. Nothing here writes a file but [`save_copy`].

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use rnl_runtime::diag::Loc;
use rnl_runtime::previous::drawn;
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::source::{dirname, normalize, relative_to, Layered, MemoryProvider, Provider};
use rnl_runtime::{Interpretation, Options};
pub use rnl_solver::Clock;
use rnl_solver::{problem_fact_holds_as_drawn, Statement};
use rnl_view::{BuildOptions, Document};
use serde::Serialize;

use crate::solve::{
    conflicts, describe_fact, fact_what, lines, solve_watched, statement_name, write_with_encoding, Progress,
    SolveOptions, SolveRun, SolvedFile, Target, Writing,
};

/// A solve held in memory: what a copy of it saves, the netlist drawn as solved, and the report.
/// It can go to another thread, as a frontend that solves on one of its own needs.
#[derive(Debug)]
pub struct Preview {
    pub copy: Copy,
    /// The netlist as the files solved read: the texts the solve would write, held over the files,
    /// drawn with their records of the solver (2.8).
    pub document: Document,
    pub report: Report,
    /// When the solve read one file ([`SolveOptions::file`]): what the reading of that file alone
    /// gives beside its Document (`rnl_view::FileResult`). `document` is then the Document of that
    /// file, as a viewer that reads the file shown alone has it: the file's levels, numbered as the
    /// whole document numbers them, and the texts of the other files left out.
    pub file: Option<OfFile>,
}

/// What the reading of one file gives beside its Document (`rnl_view::FileResult`).
#[derive(Debug, Serialize)]
pub struct OfFile {
    pub file: String,
    pub children: Vec<rnl_view::FileChild>,
    pub inputs: rnl_view::FileInputs,
}

/// A preview's Document as a viewer is given it: the Document, and with [`Preview::file`] what
/// the reading of that file adds, in one object shaped as `rnl_view::FileResult`.
#[derive(Serialize)]
pub struct Shown<'a> {
    #[serde(flatten)]
    pub document: &'a Document,
    #[serde(flatten)]
    pub file: Option<&'a OfFile>,
}

impl Preview {
    /// The Document as a viewer is given it ([`Shown`]): as the Document alone when the solve read
    /// the whole document.
    pub fn shown(&self) -> Shown<'_> {
        Shown { document: &self.document, file: self.file.as_ref() }
    }
}

/// What became of the facts, level by level, each line located in the texts of the Document.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    /// The lines of the levels not solved.
    pub notes: Vec<String>,
    pub levels: Vec<LevelReport>,
    /// How many errors the netlist has as a solver reads it, and the solver reported.
    pub errors: usize,
    /// The files the solve writes into, as the Document names them.
    pub files: Vec<String>,
    /// Whether a copy is saved as one file ([`one_file`]), or as a folder.
    pub one_file: bool,
    /// The whole report as `rnl solve --report` prints it.
    pub text: Vec<String>,
    /// "solve", or "respace" for [`respace`]: the levels spaced again, nothing searched.
    pub mode: &'static str,
}

/// One level solved.
#[derive(Clone, Debug, Serialize)]
pub struct LevelReport {
    pub title: String,
    /// "file" or "subckt".
    pub kind: &'static str,
    /// The host level, as `Level.host` names it.
    pub host: usize,
    /// The digest of the input solved. When a level of a Document has the same in its
    /// `constraints.input`, `facts[k]` here is `constraints.facts[k]` there.
    pub input: String,
    /// Such as `5 kept, 1 discarded of 6 facts`.
    pub summary: String,
    /// Every fact, in the order of 8.5.
    pub facts: Vec<FactLine>,
    /// The hints by statement (the draft 0.3, 8.5): each statement whose facts took part in
    /// discarding, with the facts discarded because of it.
    pub statements: Vec<Line>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    /// What the record cannot hold.
    pub unsolved: Vec<String>,
    pub unrouted: Vec<String>,
    pub notes: Vec<String>,
    /// The settings of the level, each with where it comes from, and their warnings and errors,
    /// each error with the key it is about, as `rnl solve --check --json` gives them: what a page
    /// points at. The settings are those of the files before the solve, with the tool's. Left out
    /// when the files could not be read for it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check: Option<crate::solve::CheckedLevel>,
}

/// A fact: its rank, what became of it, what it says and where it is written, such as
/// `[3] ALIGN y {#M1 #M2} (normal, amp.sp:12)`, and for one discarded what it conflicts with.
#[derive(Clone, Debug, Serialize)]
pub struct FactLine {
    pub rank: usize,
    /// "kept", "discarded", "undecided" (discarded when its check did not finish within the time
    /// limit, 8.5) or "failed" (it contradicts itself, 8.5, 7.6): what the resolution decided, not
    /// a measure of the drawing.
    pub status: &'static str,
    /// Whether it holds on the drawing, whatever its status: measured where the texts of the
    /// solve, read back, draw the objects, within the tolerance of the solver (11.7). None when a
    /// point it needs has no position there or one that the viewer chooses (13.3), or the level has
    /// no record of the solver in the texts.
    pub holds: Option<bool>,
    pub text: String,
    /// For a fact discarded, its hint: the groups of facts it conflicts with, by rank.
    pub why: Option<String>,
    pub at: Option<At>,
}

/// A line of the report and where it points.
#[derive(Clone, Debug, Serialize)]
pub struct Line {
    pub text: String,
    pub at: Option<At>,
}

/// A place in the texts of the Document: the index of the file in [`Document::files`], and the
/// line and column, from 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct At {
    pub file_index: usize,
    pub line: u32,
    pub col: u32,
}

/// Solve `entry` in memory and draw it as solved, as [`solve_watched`] says, nothing written: the
/// Document is built from the texts of the solve held over `files`, with the records of the
/// solver, and the report locates each fact in those texts, read again as a solver reads them.
pub fn preview(
    entry: &str,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
    watch: &dyn Fn(&Progress) -> bool,
) -> Result<Preview, String> {
    let run = solve_watched(Path::new(entry), files, options, clock, watch)?;
    as_solved(entry, run, files, options)
}

/// [`preview`], with every step of the solve given to `sink` as a [`crate::progress::Event`], as
/// [`crate::progress::solve_progress`] gives them; false cancels.
pub fn preview_progress(
    entry: &str,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
    sink: &dyn Fn(&crate::progress::Event) -> bool,
) -> Result<Preview, String> {
    let run = crate::progress::solve_progress(Path::new(entry), files, options, clock, sink)?;
    as_solved(entry, run, files, options)
}

/// [`preview_progress`] of [`crate::respace::respace_progress`]: each level as the record of the
/// solver last drew it, spaced again, nothing searched; the levels that cannot be are listed in the
/// report's notes, with the reason.
pub fn respace(
    entry: &str,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
    sink: &dyn Fn(&crate::progress::Event) -> bool,
) -> Result<Preview, String> {
    let run = crate::respace::respace_progress(Path::new(entry), files, options, clock, sink)?;
    let mut shown = as_solved(entry, run, files, options)?;
    shown.report.mode = "respace";
    Ok(shown)
}

/// The run drawn as solved, as [`preview`] gives it. What the solve read already, the same texts
/// with the same options, is taken from its readings instead of read again
/// ([`crate::solve::Readings`]).
fn as_solved(entry: &str, mut run: SolveRun, files: &dyn Provider, options: &SolveOptions) -> Result<Preview, String> {
    let readings = std::mem::take(&mut run.readings);
    let mut written = MemoryProvider::new();
    for f in &run.files {
        written.insert(&f.path, &f.text);
    }
    let both = Layered { layers: vec![&written, files] };
    let build = BuildOptions {
        dialect: options.dialect.clone(),
        formats: options.formats.clone(),
        constraints: true,
        // the viewer shows it, with its flylines, which it hides by their class
        flylines: true,
        ..BuildOptions::default()
    };
    // one file read, its Document; else the whole document's
    let (document, file) = match options.level.as_ref().and(options.file.as_ref()) {
        Some(file) => {
            let read = rnl_view::build_file(entry, file, &both, &build)?;
            (read.document, Some(OfFile { file: read.file, children: read.children, inputs: read.inputs }))
        }
        None => (rnl_view::build(entry, &both, &build)?, None),
    };
    // the texts as a solver reads them, without the records of the solver, so that the facts are
    // those solved, at the lines where the Document shows them
    let again =
        rnl_view::interpret_with(entry, options.dialect.as_deref(), &options.formats, &both, &options.runtime())?;
    // and with them, which is what is drawn: the facts are measured there. The last check that a
    // record reads back read the same: the texts it held over the files are those the solve wrote
    let with = match readings.written.filter(|(texts, _)| run.entry == entry && holds_the_files(texts, &run)) {
        Some((_, read)) => read,
        None => {
            let with = Options { generated: true, problems: true, ..options.runtime() };
            std::rc::Rc::new(rnl_view::interpret_with(
                entry,
                options.dialect.as_deref(),
                &options.formats,
                &both,
                &with,
            )?)
        }
    };
    let before = readings.before.as_deref().filter(|_| run.entry == entry);
    let checked = check(entry, files, options, before);
    let report = report(&run, &document, &again.problems, &with, options.solver.tolerance, checked);
    Ok(Preview { copy: Copy::of(&run), document, report, file })
}

/// Whether `texts`, held over the files by path, are the files the run wrote and no others, so that
/// a reading over them read what the run wrote.
fn holds_the_files(texts: &std::collections::BTreeMap<String, String>, run: &SolveRun) -> bool {
    texts.len() == run.files.len() && run.files.iter().all(|f| texts.get(&normalize(&f.path)) == Some(&f.text))
}

/// The settings of each level of `entry`, as `rnl solve --check` gives them, with the tool's
/// settings of `options`, given as old `--solver-options` when the options say so; none when the
/// file cannot be read for it. `before` is the solve's reading of the files as they were, taken
/// when the check reads them so too.
fn check(
    entry: &str,
    files: &dyn Provider,
    options: &SolveOptions,
    before: Option<&Interpretation>,
) -> Option<crate::solve::Checked> {
    let given = &options.solver.settings.overrides;
    let text = given.iter().map(|s| format!("{}={}", s.key, s.value.show())).collect::<Vec<_>>().join(",");
    let text = if options.solver.settings.legacy_alias {
        crate::solve::ToolSettings::Options(text)
    } else {
        crate::solve::ToolSettings::Config(text)
    };
    let tool: Vec<crate::solve::ToolSettings> = (!given.is_empty()).then_some(text).into_iter().collect();
    crate::solve::check_settings_read(Path::new(entry), files, options, &tool, before).ok()
}

/// The report of a run, located in the texts of `document`; `problems` are those of these texts
/// read as a solver reads them, which give each fact its line there, and `with` is the same read
/// with their records of the solver, which says where the facts are measured within `tolerance`.
/// Each level gets its part of `checked`.
fn report(
    run: &SolveRun,
    document: &Document,
    problems: &[LevelProblem],
    with: &Interpretation,
    tolerance: f64,
    checked: Option<crate::solve::Checked>,
) -> Report {
    let mut checked = checked.map(|c| c.levels).unwrap_or_default();
    let at = |loc: &Loc| -> Option<At> {
        let file_index = document.files.iter().position(|f| f.path == loc.file)?;
        (loc.line > 0).then_some(At { file_index, line: loc.line, col: loc.col })
    };
    let mut levels = Vec::new();
    for level in &run.levels {
        let p = &level.problem;
        let r = &level.solved.report;
        let drawn = drawn(p, with);
        // the same level of the texts written, whose facts are those solved, in the same order
        let q = problems.iter().find(|q| q.level == p.level && q.title == p.title && q.facts.len() == p.facts.len());
        let fact_at = |k: usize| q.unwrap_or(p).facts.get(k).and_then(|f| f.source.as_ref()).and_then(at);
        let facts = (0..p.facts.len())
            .map(|k| {
                let d = r.discarded.iter().find(|d| d.fact == k);
                let status = if r.kept.contains(&k) {
                    "kept"
                } else if r.failed.contains(&k) {
                    "failed"
                } else if d.is_some_and(|d| d.undecided) {
                    "undecided"
                } else {
                    "discarded"
                };
                let why = d.map(|d| {
                    if d.undecided {
                        "not decided within the time limit of the solver (8.5)".to_string()
                    } else {
                        conflicts(d)
                    }
                });
                let holds = problem_fact_holds_as_drawn(p, &drawn, k, tolerance);
                FactLine { rank: k, status, holds, text: describe_fact(p, k), why, at: fact_at(k) }
            })
            .collect();
        let statements = r
            .statements
            .iter()
            .filter(|(_, hint)| !hint.discarded.is_empty())
            .map(|(s, hint): (&Statement, _)| {
                let facts: Vec<String> = hint.facts.iter().map(|k| fact_what(p, *k)).collect();
                let lost: Vec<String> = hint.discarded.iter().map(|k| format!("[{k}]")).collect();
                let text = format!(
                    "{} {} took part in discarding {}",
                    statement_name(p, s),
                    facts.join(" and "),
                    lost.join(", ")
                );
                Line { text, at: hint.facts.first().and_then(|&k| fact_at(k)) }
            })
            .collect();
        let unrouted = r
            .unrouted
            .iter()
            .map(|u| {
                let why = if u.budget { "the budget of the router ran out" } else { "no path joins them" };
                format!("the net {}: left in {} parts, {why}", u.net, u.parts)
            })
            .collect();
        let failed = if r.failed.is_empty() { String::new() } else { format!(", {} failed", r.failed.len()) };
        let noun = if p.facts.len() == 1 { "fact" } else { "facts" };
        levels.push(LevelReport {
            title: p.title.clone(),
            kind: p.kind,
            host: p.level,
            input: rnl_runtime::problem::input_digest(p),
            summary: format!(
                "{} kept, {} discarded{failed} of {} {noun}",
                r.kept.len(),
                r.discarded.len(),
                p.facts.len()
            ),
            facts,
            statements,
            errors: r
                .errors
                .iter()
                .cloned()
                .chain(level.write_errors.iter().map(|error| rnl_runtime::diag::Sink::new().render(error)))
                .collect(),
            warnings: r
                .warnings
                .iter()
                .cloned()
                .chain(level.written.warnings.iter().map(|w| rnl_runtime::diag::Sink::new().render(w)))
                .collect(),
            unsolved: level.written.unsolved.iter().map(|u| u.to_string()).collect(),
            unrouted,
            notes: r.notes.iter().chain(&level.written.notes).cloned().collect(),
            check: checked.iter().position(|c| c.title.as_deref() == Some(p.title.as_str())).map(|i| checked.remove(i)),
        });
    }
    let notes: Vec<String> = run.not_solved.iter().map(crate::solve::not_solved).collect();
    let named = |path: &str| {
        let path = normalize(path);
        document
            .files
            .iter()
            .find(|f| normalize(&rnl_runtime::source::join(&dirname(&normalize(&run.entry)), &f.path)) == path)
            .map_or(path.clone(), |f| f.name.clone())
    };
    Report {
        notes,
        levels,
        errors: run.errors + crate::solve::errors(run),
        files: run.files.iter().map(|f| named(&f.path)).collect(),
        one_file: one_file(&Copy::of(run)),
        text: lines(run, true),
        mode: "solve",
    }
}

/// The wall clock, until `stop` is set: then each reading is far past the one before it, so that
/// the time limit of the resolution, which runs from the reading when a level begins (8.5), is
/// reached at once, in the level being solved and in any after it. A solve cancelled so stops
/// checking the facts of the level it is in, and its watcher stops it before the next level. With
/// no time limit, only the watcher stops it.
pub struct StoppableClock {
    start: Instant,
    stop: Arc<AtomicBool>,
    /// The readings since `stop` was set.
    after: AtomicU64,
}

impl StoppableClock {
    pub fn new(stop: Arc<AtomicBool>) -> StoppableClock {
        StoppableClock { start: Instant::now(), stop, after: AtomicU64::new(0) }
    }
}

/// How far each reading of a stopped clock is past the one before, in seconds: more than any time
/// limit a user sets.
const STOPPED_STEP: f64 = 1e9;

impl Clock for StoppableClock {
    fn check_every(&self) -> u64 {
        rnl_solver::WALL_CLOCK_CHECK_EVERY
    }
    fn elapsed(&self) -> f64 {
        if self.stop.load(Ordering::Relaxed) {
            let k = self.after.fetch_add(1, Ordering::Relaxed) + 1;
            // past every reading of the wall clock, and never less than an earlier reading
            1e12 + k as f64 * STOPPED_STEP
        } else {
            self.start.elapsed().as_secs_f64()
        }
    }
}

/// The paths a request names its files by (`paths`), each by its normal path, which a solve names the file by
/// ([`rnl_view::RequestFiles`]); an error when two of them are one file.
pub fn request_paths<'a>(
    paths: impl IntoIterator<Item = &'a String>,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    let mut asked = std::collections::BTreeMap::new();
    for path in paths {
        if let Some(other) = asked.insert(normalize(path), path.clone()) {
            return Err(format!("files: {other} and {path} are one file"));
        }
    }
    Ok(asked)
}

/// What a copy of a solve saves: the file solved, the texts the solve wrote, and the files it read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Copy {
    /// The file solved, `/`-separated ([`SolveRun::entry`]).
    pub entry: String,
    /// The files written, with their texts ([`SolveRun::files`]).
    pub files: Vec<SolvedFile>,
    /// The files read ([`SolveRun::read`]).
    pub read: Vec<String>,
}

impl Copy {
    pub fn of(run: &SolveRun) -> Copy {
        Copy { entry: run.entry.clone(), files: run.files.clone(), read: run.read.clone() }
    }

    /// The texts written, by the paths a request names their files by ([`request_paths`]), as a viewer that solves
    /// a request is answered: it draws a solve's preview again from them (Export Image). A file the request does not
    /// name is given by its own path.
    pub fn written_by(
        &self,
        asked: &std::collections::BTreeMap<String, String>,
    ) -> std::collections::BTreeMap<String, String> {
        self.files
            .iter()
            .map(|f| (asked.get(&normalize(&f.path)).cloned().unwrap_or_else(|| f.path.clone()), f.text.clone()))
            .collect()
    }

    /// A run of these files and no level, which is what writing them takes.
    fn run(&self) -> SolveRun {
        SolveRun {
            entry: self.entry.clone(),
            levels: Vec::new(),
            not_solved: Vec::new(),
            files: self.files.clone(),
            read: self.read.clone(),
            errors: 0,
            readings: Default::default(),
        }
    }
}

/// Whether a copy is one file: the solve writes one file and reads no other beside it, so that
/// the file reads the same wherever it is saved. Otherwise the copy is a folder that holds the
/// files written and the others read, in their places relative to the file solved.
pub fn one_file(copy: &Copy) -> bool {
    match copy.files.as_slice() {
        [f] => copy.read.iter().all(|r| normalize(r) == normalize(&f.path)),
        _ => false,
    }
}

/// What a copy is refused with when it would replace a file that was read.
pub const OVER_A_FILE_READ: &str = "a copy is never saved over a file that was read: choose another file or folder";

/// Save a copy of a solve: as the file `to` when it is [`one_file`], otherwise into the folder `to`,
/// as `rnl solve -o DIR` writes it, the files written and the others read in their places
/// relative to the file solved. Refused, with nothing written, when a file of the copy would be
/// one that was read, the file solved or any other, and when there is nothing to save.
pub fn save_copy(copy: &Copy, to: &Path) -> Result<Writing, String> {
    save_copy_with_encoding(copy, to, crate::codec::OutputEncoding::Preserve)
}

/// Save the copy with the user's explicit output encoding; other safety checks are unchanged.
pub fn save_copy_with_encoding(
    copy: &Copy,
    to: &Path,
    encoding: crate::codec::OutputEncoding,
) -> Result<Writing, String> {
    let run = &copy.run();
    if run.files.is_empty() {
        return Err("the solve wrote no file: there is nothing to save".to_string());
    }
    let read: Vec<PathBuf> =
        run.read.iter().chain(run.files.iter().map(|f| &f.path)).filter_map(|p| real(Path::new(p))).collect();
    let over = |path: &Path| real(path).is_some_and(|p| read.contains(&p));
    if one_file(copy) {
        if over(to) {
            return Err(OVER_A_FILE_READ.to_string());
        }
        let f = &run.files[0];
        let bytes = crate::codec::encode_file(Some(Path::new(&f.path)), &f.text, encoding)
            .map_err(|e| format!("{}: {e}", f.path))?;
        if let Some(parent) = to.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))?;
        }
        std::fs::write(to, bytes).map_err(|e| format!("cannot write {}: {e}", to.display()))?;
        return Ok(Writing { written: vec![to.to_path_buf()], ..Writing::default() });
    }
    // every file of the folder, written and copied, before any is written
    let base = dirname(&normalize(&run.entry));
    let target = Target::Dir(to.to_path_buf());
    let mut places = crate::solve::destinations(run, &target)?;
    places.extend(run.read.iter().filter_map(|p| relative_to(&normalize(p), &base).map(|rel| to.join(rel))));
    if places.iter().any(|p| over(p)) {
        return Err(OVER_A_FILE_READ.to_string());
    }
    let writing = write_with_encoding(run, &target, encoding);
    match &writing.error {
        Some(e) => Err(e.clone()),
        None => Ok(writing),
    }
}

/// A path as it is on disk, for telling two names of one file apart: the file itself when it
/// exists, else its folder's, with its name; None when neither is there.
fn real(path: &Path) -> Option<PathBuf> {
    if let Ok(p) = std::fs::canonicalize(path) {
        return Some(p);
    }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    Some(std::fs::canonicalize(parent).ok()?.join(path.file_name()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_texts_written_are_named_by_the_paths_of_the_request() {
        let paths = ["/w/./top.sp".to_string(), "/w/lib/../cell.inc".to_string()];
        let asked = request_paths(&paths).unwrap();
        let copy = Copy {
            entry: "/w/top.sp".into(),
            files: vec![
                SolvedFile { path: "/w/top.sp".into(), text: "solved".into() },
                SolvedFile { path: "/w/other.inc".into(), text: "other".into() },
            ],
            read: Vec::new(),
        };
        let written = copy.written_by(&asked);
        assert_eq!(written.get("/w/./top.sp").map(String::as_str), Some("solved"));
        // a file the request does not name, by its own path
        assert_eq!(written.get("/w/other.inc").map(String::as_str), Some("other"));
        // two paths of one file are an error
        let twice = ["/w/top.sp".to_string(), "/w/./top.sp".to_string()];
        assert!(request_paths(&twice).unwrap_err().contains("are one file"));
    }

    /// The preview takes the readings the solve made instead of reading the same texts again, and
    /// draws and reports what reading them again gives: the same Document and the same report,
    /// the settings checked among it.
    #[test]
    fn the_readings_a_solve_made_give_what_reading_again_gives() {
        const NET: &str = "* preview\n\
            * @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }\n\
            R1 a b 1k\nR2 b 0 1k\nC1 a 0 1p\n\
            * @RNL { #R1 @PLACE(0, 0) }\n\
            * @RNL { {#R1 #R2} @ALIGN(\"x\") }\n.end\n";
        let files = MemoryProvider::new().with("/w/t.sp", NET);
        let options =
            SolveOptions { solver: rnl_solver::SolverOptions::without_time_limit(), ..SolveOptions::default() };
        let clock = crate::solve::WallClock::new();
        let run = crate::solve::solve_watched(Path::new("/w/t.sp"), &files, &options, &clock, &|_| true).unwrap();
        // the files as they were, and the document with the record written, as its check read it
        assert!(run.readings.before.is_some());
        let (texts, _) = run.readings.written.as_ref().expect("the record's check read it back");
        assert!(holds_the_files(texts, &run));
        let mut bare = run.clone();
        bare.readings = crate::solve::Readings::default();
        let taken = as_solved("/w/t.sp", run, &files, &options).unwrap();
        let read = as_solved("/w/t.sp", bare, &files, &options).unwrap();
        let json = |shown: &Preview| serde_json::to_string(&(&shown.document, &shown.report)).unwrap();
        assert_eq!(json(&taken), json(&read));
        assert!(taken.report.levels.iter().all(|l| l.check.is_some()), "{:?}", taken.report.levels);
    }

    /// A level solved with the file it is written in read alone writes what the solve of the whole
    /// document writes, and its preview is the reading of that file of the texts solved.
    #[test]
    fn a_level_solved_in_its_file_alone_writes_the_same_and_shows_that_file() {
        const TOP: &str = "* top\n\
            * @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }\n\
            .include \"sub.sp\"\nX1 in out amp\nR9 out 0 1k\n.end\n";
        const SUB: &str = "* sub\n\
            * @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }\n\
            .subckt amp a b\nR1 a m 1k\nR2 m b 1k\nC1 m 0 1p\n\
            * @RNL { #R1 @PLACE(0, 0) }\n.ends\n";
        let files = MemoryProvider::new().with("/w/top.sp", TOP).with("/w/sub.sp", SUB);
        let whole = rnl_view::build("/w/top.sp", &files, &rnl_view::BuildOptions::default()).unwrap();
        let file = whole.files.iter().map(|f| f.path.clone()).find(|p| p.ends_with("sub.sp")).unwrap();
        let options = SolveOptions {
            level: Some("amp".into()),
            solver: rnl_solver::SolverOptions::without_time_limit(),
            ..SolveOptions::default()
        };
        let one = SolveOptions { file: Some(file.clone()), ..options.clone() };
        let clock = crate::solve::WallClock::new();
        let texts = |run: &SolveRun| run.files.iter().map(|f| (f.path.clone(), f.text.clone())).collect::<Vec<_>>();
        let a = crate::solve::solve_watched(Path::new("/w/top.sp"), &files, &options, &clock, &|_| true).unwrap();
        let b = crate::solve::solve_watched(Path::new("/w/top.sp"), &files, &one, &clock, &|_| true).unwrap();
        assert_eq!(texts(&a), texts(&b));
        assert_eq!(a.levels.len(), 1);
        assert_eq!(b.levels.len(), 1);
        assert_eq!(a.levels[0].record, b.levels[0].record);

        let shown = as_solved("/w/top.sp", b, &files, &one).unwrap();
        let mut written = MemoryProvider::new();
        for f in &shown.copy.files {
            written.insert(&f.path, &f.text);
        }
        let both = Layered { layers: vec![&written, &files] };
        let build = rnl_view::BuildOptions { constraints: true, flylines: true, ..rnl_view::BuildOptions::default() };
        let read = rnl_view::build_file("/w/top.sp", &file, &both, &build).unwrap();
        assert_eq!(serde_json::to_string(&shown.shown()).unwrap(), serde_json::to_string(&read).unwrap());
        assert_eq!(shown.file.as_ref().map(|f| f.file.as_str()), Some(file.as_str()));
        assert!(shown.document.levels.iter().all(|l| shown.report.levels.iter().any(|r| r.host == l.host)));
        // the whole document's preview reports the same, each line in its own Document
        let full = as_solved("/w/top.sp", a, &files, &options).unwrap();
        assert!(full.file.is_none());
        let lines = |p: &Preview| {
            serde_json::to_value(&p.report.levels).unwrap().to_string().replace(
                &format!("\"file_index\":{}", p.document.files.iter().position(|f| f.path == file).unwrap()),
                "\"file_index\":F",
            )
        };
        assert_eq!(lines(&shown), lines(&full));
    }

    /// Texts held over the files are those of a run only when they are its files, each with its
    /// text, and no other.
    #[test]
    fn texts_hold_the_files_of_a_run_only_when_they_are_its_files() {
        let mut run = Copy {
            entry: "/w/t.sp".into(),
            files: vec![SolvedFile { path: "/w/t.sp".into(), text: "solved".into() }],
            read: Vec::new(),
        }
        .run();
        let texts = |pairs: &[(&str, &str)]| {
            pairs.iter().map(|(p, t)| (p.to_string(), t.to_string())).collect::<std::collections::BTreeMap<_, _>>()
        };
        assert!(holds_the_files(&texts(&[("/w/t.sp", "solved")]), &run));
        assert!(!holds_the_files(&texts(&[("/w/t.sp", "as read")]), &run));
        assert!(!holds_the_files(&texts(&[("/w/t.sp", "solved"), ("/w/moved.sp", "header moved")]), &run));
        run.files.push(SolvedFile { path: "/w/other.sp".into(), text: "other".into() });
        assert!(!holds_the_files(&texts(&[("/w/t.sp", "solved")]), &run));
    }
}
