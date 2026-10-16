//! `rnl solve`: every drawing level of a document solved, and its record of the solver written
//! back (standard 2.8, 8.5, 13.7, 14.8, chapter 17).
//!
//! The document is read as a solver reads it, `Options::solver()`, without the records with
//! `type="generated"` (2.8). Each level is solved by `rnl-solver`; its record is written with
//! `solution::write_record`, the hash `sig1` of its problem and of the solver's canonical options in
//! the head, finished with `rnl_view::finish_record`, which adds the check of its numbers in a file
//! of the draft 0.3 (5.9), and put in place of the level's records of the solver with
//! `writeback::replace_generated`, at the end of the level's own text (17). A file may hold several
//! levels, and writing one back moves the lines of those after it, so the document is read again
//! before the next level, the texts written so far held in memory over the files.
//!
//! [`solve`] does the work without writing a file; [`write`] puts the texts where `-o DIR` or
//! `--in-place` says; [`report`] is what the command prints of each level.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rnl_runtime::diag::Severity;
use rnl_runtime::problem::{sig1, FactKind, LevelProblem, PointRef};
use rnl_runtime::solution::{write_record, Written};
use rnl_runtime::source::{dirname, normalize, relative_to, Layered, MemoryProvider, Provider};
use rnl_runtime::writeback::replace_generated;
use rnl_runtime::{Interpretation, Options};
use rnl_solver::{Solved, SolverOptions, AUTHOR};

use crate::provider::path_str;

/// What `rnl solve` takes besides the file.
#[derive(Clone, Debug)]
pub struct SolveOptions {
    /// "spice" or "spectre", instead of guessing from the file.
    pub dialect: Option<String>,
    /// Use the standard package implicitly at the root (16.3).
    pub std: bool,
    /// For development only: read the files that declare the draft 0.3 as well.
    pub draft: bool,
    /// Only the level with this title: a subcircuit name, or the file name.
    pub level: Option<String>,
    /// The solver's options; their canonical text is the second segment of the hash (17).
    pub solver: SolverOptions,
}

impl Default for SolveOptions {
    fn default() -> SolveOptions {
        SolveOptions { dialect: None, std: true, draft: false, level: None, solver: SolverOptions::default() }
    }
}

impl SolveOptions {
    /// The runtime's options the levels are read with: as a solver reads them (2.8).
    pub fn runtime(&self) -> Options {
        Options { std: self.std, draft: self.draft, ..Options::solver() }
    }
}

/// One level solved and written back.
#[derive(Clone, Debug)]
pub struct SolvedLevel {
    /// The problem solved, as the document read before this level was written back gave it.
    pub problem: LevelProblem,
    pub solved: Solved,
    /// The record as [`write_record`] gives it, with what it cannot hold (`unsolved`) and what it
    /// holds although a solver should not do it (`notes`).
    pub written: Written,
    /// The record written into the file: [`Written::record`] finished by
    /// `rnl_view::finish_record`.
    pub record: String,
    /// The `hash` of the record head (17).
    pub hash: String,
    /// The path of the file the level's text is in, as it was read.
    pub path: String,
    /// The time the solver took for the level. Nothing written depends on it.
    pub time: Duration,
}

/// A file a level was written back into, with its text after every level in it was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolvedFile {
    /// The path it was read from, `/`-separated.
    pub path: String,
    pub text: String,
}

/// What [`solve`] gives.
#[derive(Clone, Debug)]
pub struct SolveRun {
    /// The file solved, `/`-separated.
    pub entry: String,
    /// The levels, in the order of the document's drawing levels.
    pub levels: Vec<SolvedLevel>,
    /// Every file a level was written back into, in the order it was first written.
    pub files: Vec<SolvedFile>,
    /// The errors of the document read as a solver reads it, before anything was written back.
    pub errors: usize,
}

/// Where [`write`] puts the files solved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Under this directory, each file named as it is relative to the directory of the file
    /// solved (`-o DIR`).
    Dir(PathBuf),
    /// Over the files read (`--in-place`).
    InPlace,
}

/// Reads `file` as a solver reads it, solves every drawing level, or the one `options.level`
/// names, and writes each level's record of the solver back into the text of its file: the
/// texts are in [`SolveRun::files`], and nothing is written to disk. Fails when the file cannot be
/// read, or when no level has the title asked for.
pub fn solve(file: &Path, files: &dyn Provider, options: &SolveOptions) -> Result<SolveRun, String> {
    let entry = path_str(file);
    let runtime = options.runtime();
    let canonical = options.solver.canonical();
    let mut memory = MemoryProvider::new();
    let read = |memory: &MemoryProvider| -> Result<Interpretation, String> {
        let both = Layered { layers: vec![memory, files] };
        rnl_view::interpret(&entry, options.dialect.as_deref(), &both, &runtime)
    };
    let first = read(&memory)?;
    let errors = first.diagnostics().iter().filter(|d| d.severity == Severity::Error).count();
    let targets: Vec<(usize, String)> = first
        .problems
        .iter()
        .filter(|p| options.level.as_ref().is_none_or(|t| *t == p.title))
        .map(|p| (p.level, p.title.clone()))
        .collect();
    if targets.is_empty() {
        if let Some(t) = &options.level {
            return Err(format!("no drawing level is titled {t}"));
        }
    }
    let mut levels = Vec::new();
    let mut solved_files: Vec<SolvedFile> = Vec::new();
    let mut current = Some(first);
    for (level, title) in targets {
        // the document as written so far: the lines of this level have moved if one before it in
        // its file was written back
        let r = match current.take() {
            Some(r) => r,
            None => read(&memory)?,
        };
        let p = r
            .problems
            .iter()
            .find(|p| p.level == level && p.title == title)
            .ok_or_else(|| format!("the level {title} is not read again after the levels before it were solved"))?;
        let start = Instant::now();
        let solved = rnl_solver::solve(p, &options.solver);
        let time = start.elapsed();
        let hash = sig1(p, &canonical);
        let written = write_record(p, &solved.solution, AUTHOR, &hash);
        let host = &r.doc.levels[p.level];
        let path = normalize(&host.path);
        let (record, text) = {
            let both = Layered { layers: vec![&memory, files] };
            let record = rnl_view::finish_record(&r.doc, &both, &runtime, p, &written.record);
            // the text as the adapter read it, which the positions of its records refer to
            let text = match r.doc.source(&host.file) {
                Some(t) => t.to_string(),
                None => both.read(&path).ok_or_else(|| format!("cannot read {path}"))?,
            };
            (record, text)
        };
        let out = replace_generated(&r.doc, p.level, &text, &record);
        memory.insert(&path, &out);
        match solved_files.iter_mut().find(|f| f.path == path) {
            Some(f) => f.text = out,
            None => solved_files.push(SolvedFile { path: path.clone(), text: out }),
        }
        levels.push(SolvedLevel { problem: p.clone(), solved, written, record, hash, path, time });
    }
    Ok(SolveRun { entry, levels, files: solved_files, errors })
}

/// Where each file of a run goes: under `-o DIR` by its name relative to the directory of the
/// file solved, or over itself. A file outside that directory, such as one included from `../`,
/// has no name under `DIR`: that is an error, and nothing is written.
pub fn destinations(run: &SolveRun, target: &Target) -> Result<Vec<PathBuf>, String> {
    let base = dirname(&normalize(&run.entry));
    run.files
        .iter()
        .map(|f| match target {
            Target::InPlace => Ok(PathBuf::from(&f.path)),
            Target::Dir(dir) => match name_in(&f.path, &base) {
                Some(rel) => Ok(dir.join(rel)),
                None => Err(format!(
                    "{} is not in the directory of {}, so it has no name under {}; write it with --in-place",
                    f.path,
                    run.entry,
                    dir.display()
                )),
            },
        })
        .collect()
}

/// `path` relative to the directory `dir`, both normalized; "" is the working directory.
fn name_in(path: &str, dir: &str) -> Option<String> {
    let path = normalize(path);
    if dir.is_empty() {
        let outside =
            path.starts_with('/') || path == ".." || path.starts_with("../") || path.as_bytes().get(1) == Some(&b':');
        return (!outside).then_some(path);
    }
    relative_to(&path, dir).map(String::from)
}

/// Writes the files of a run where `target` says, creating the directories they need, and gives
/// the paths written, in the order of [`SolveRun::files`].
pub fn write(run: &SolveRun, target: &Target) -> Result<Vec<PathBuf>, String> {
    let to = destinations(run, target)?;
    for (f, out) in run.files.iter().zip(&to) {
        if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))?;
        }
        std::fs::write(out, &f.text).map_err(|e| format!("cannot write {}: {e}", out.display()))?;
    }
    Ok(to)
}

/// A point of a fact, as diagnostics show its object: `#M1`, `#M1:d`, `{#M1 #M2}` for a centre.
fn point(p: &LevelProblem, r: &PointRef) -> String {
    let label = |k: usize| p.objects.get(k).map_or_else(|| format!("object {k}"), |o| o.label.clone());
    match r {
        PointRef::Origin(k) => label(*k),
        PointRef::Pin { object, terminal, .. } => format!("{}:{terminal}", label(*object)),
        PointRef::Centre(points) => format!("{{{}}}", points.iter().map(|q| point(p, q)).collect::<Vec<_>>().join(" ")),
    }
}

/// One fact as the report shows it: its rank, what it says, its priority and where it is
/// written, and the definitions of the usages it was expanded from, such as
/// `[9] ALIGN y {#M1 #M2} (normal, rnl-std.rnl:227, from $MIRROR_PAIR)`.
pub fn describe_fact(p: &LevelProblem, k: usize) -> String {
    let Some(f) = p.facts.get(k) else { return format!("[{k}]") };
    let list = |ms: &[PointRef]| format!("{{{}}}", ms.iter().map(|m| point(p, m)).collect::<Vec<_>>().join(" "));
    let objects = |ks: &[usize]| list(&ks.iter().map(|k| PointRef::Origin(*k)).collect::<Vec<_>>());
    let what = match &f.kind {
        FactKind::Place { at, axis, value } => format!("the {} of {} = {}", axis.as_str(), point(p, at), num(*value)),
        FactKind::PointCoordinate { point: k, axis, value } => {
            format!("the {} of {} = {}", axis.as_str(), point(p, &PointRef::Origin(*k)), num(*value))
        }
        FactKind::Offset { text, from, axis, value } => format!(
            "{} {} from {} by {}",
            point(p, &PointRef::Origin(*text)),
            axis.as_str(),
            point(p, from),
            num(*value)
        ),
        FactKind::Orient { object, code } => {
            format!("ORIENT {} {}", point(p, &PointRef::Origin(*object)), code.as_str())
        }
        FactKind::Align { axis, members } => format!("ALIGN {} {}", axis.as_str(), list(members)),
        FactKind::Distribute { axis, members } => format!("DISTRIBUTE {} {}", axis.as_str(), list(members)),
        FactKind::Order { axis, sense, members } => {
            format!("ORDER {} {} {}", axis.as_str(), sense.as_str(), list(members))
        }
        FactKind::Distance { axis, sense, min, max, from, to } => {
            let mut bounds = String::new();
            if let Some(m) = min {
                bounds.push_str(&format!(" min={}", num(*m)));
            }
            if let Some(m) = max {
                bounds.push_str(&format!(" max={}", num(*m)));
            }
            format!("DISTANCE {} {}{bounds} {} {}", axis.as_str(), sense.as_str(), list(from), list(to))
        }
        FactKind::OrientRelation { relation, from, to } => {
            format!("ORIENT-RELATION {} {} {}", relation.as_str(), objects(from), objects(to))
        }
    };
    // a handle, such as %c1, or a label of its own
    let coordinate = matches!(f.kind, FactKind::Place { .. } | FactKind::PointCoordinate { .. });
    let named =
        if coordinate || f.label == f.kind.name() || f.label == what { what } else { format!("{} = {what}", f.label) };
    let mut context = vec![f.priority.as_str().to_string()];
    if let Some(l) = &f.source {
        context.push(format!("{}:{}", rnl_runtime::host::file_name(&l.file), l.line));
    }
    if !f.chain.is_empty() {
        let defs: Vec<&str> = f.chain.iter().filter_map(|u| p.usages.get(*u)).map(|u| u.definition.as_str()).collect();
        context.push(format!("from {}", defs.join(" in ")));
    }
    format!("[{k}] {named} ({})", context.join(", "))
}

fn num(v: f64) -> String {
    rnl_runtime::geom::fmt_num(v)
}

/// What the command prints of one level: its title, the facts kept and discarded (every fact with
/// `all`, otherwise the discarded ones with the facts they conflict with), what the record cannot
/// hold, the ROUTEs left in parts, the inert objectives, the notes, the work and the time.
pub fn report(level: &SolvedLevel, all: bool) -> Vec<String> {
    let p = &level.problem;
    let r = &level.solved.report;
    let mut out = vec![format!(
        "level {} ({} in {}): {} kept, {} discarded of {}",
        p.title,
        p.kind,
        rnl_runtime::host::file_name(&level.path),
        r.kept.len(),
        r.discarded.len(),
        count(p.facts.len(), "fact")
    )];
    // a fact discarded, and the minimal set of facts kept before it that it conflicts with
    let discarded = |out: &mut Vec<String>, k: usize, indent: &str| {
        let Some(d) = r.discarded.iter().find(|d| d.fact == k) else { return };
        let why = match (d.undecided, d.conflicts_with.is_empty()) {
            (true, true) => "undecided: the budget for deciding it ran out",
            (true, false) => "undecided: the budget for deciding it ran out; the conflict found by then is with",
            (false, true) => "it cannot hold even alone",
            (false, false) => "it conflicts with",
        };
        out.push(format!("{indent}discarded {}: {why}", describe_fact(p, k)));
        for c in &d.conflicts_with {
            out.push(format!("{indent}    {}", describe_fact(p, *c)));
        }
    };
    if all {
        out.push("  facts, in the order of 8.5:".to_string());
        for k in 0..p.facts.len() {
            if r.kept.contains(&k) {
                out.push(format!("    kept {}", describe_fact(p, k)));
            } else {
                discarded(&mut out, k, "    ");
            }
        }
    } else {
        for d in &r.discarded {
            discarded(&mut out, d.fact, "  ");
        }
    }
    let section = |out: &mut Vec<String>, head: &str, lines: Vec<String>| {
        if !lines.is_empty() {
            out.push(format!("  {head}:"));
            out.extend(lines.into_iter().map(|l| format!("    {l}")));
        }
    };
    section(&mut out, "unsolved", level.written.unsolved.iter().map(|u| u.to_string()).collect());
    section(
        &mut out,
        "unrouted",
        r.unrouted
            .iter()
            .map(|u| {
                let why = if u.budget { "the budget of the router ran out" } else { "no path joins them" };
                format!("the net {}: left in {} parts, {why}", u.net, u.parts)
            })
            .collect(),
    );
    section(&mut out, "inert objectives", r.inert_objectives.clone());
    section(&mut out, "notes", r.notes.iter().chain(&level.written.notes).cloned().collect());
    let w = &r.work;
    out.push(format!(
        "  work: {}, {}, {}, {}",
        count(w.resolve_nodes as usize, "resolve node"),
        count(w.pivots as usize, "pivot"),
        count(w.candidates as usize, "candidate"),
        count(w.astar_expansions as usize, "A* expansion")
    ));
    out.push(format!("  time: {:.3} s", level.time.as_secs_f64()));
    out.push(format!("  hash: {}", level.hash));
    out
}

/// A count and its noun, singular for one.
fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_under_the_directory_of_the_file_solved() {
        assert_eq!(name_in("/w/a/t.sp", "/w/a").as_deref(), Some("t.sp"));
        assert_eq!(name_in("/w/a/sub/x.inc", "/w/a").as_deref(), Some("sub/x.inc"));
        assert_eq!(name_in("/w/x.inc", "/w/a"), None);
        assert_eq!(name_in("D:/w/a/t.sp", "D:/w/a").as_deref(), Some("t.sp"));
        // a bare file name is in the working directory
        assert_eq!(name_in("t.sp", "").as_deref(), Some("t.sp"));
        assert_eq!(name_in("./sub/x.inc", "").as_deref(), Some("sub/x.inc"));
        assert_eq!(name_in("../x.inc", ""), None);
        assert_eq!(name_in("/x.inc", ""), None);
        assert_eq!(name_in("D:/x.inc", ""), None);
    }
}
