//! `rnl solve`: every drawing level of a document solved, and its record of the solver written
//! back (standard 2.8, 8.5, 13.7, 14.8, 16.1, chapter 17).
//!
//! The document is read as a solver reads it, `Options::solver()`, without the records with
//! `type="generated"` (2.8). Each level is solved by `rnl-solver`; its record is written with
//! `solution::write_record`, the hash `sig1` of its problem and of the solver's canonical options in
//! the head, finished with `rnl_view::finish_record`, which adds the check of its numbers in a file
//! of the draft 0.3 (5.9), and put in place of the level's records of the solver with
//! `writeback::replace_generated_with_header`, at the end of the level's own text (17). A record of
//! the solver written for the level with scope= in another file, which a solver does not write
//! (2.6) but which applies to the level all the same, is removed from that file. Every file written
//! has its header record and its version (16.1), added when it has none, in the same pass as the
//! record, at the positions the reading gives; but a file read only through library sections,
//! where what lies outside them is not read (2.2), gets no new header. A file may hold several
//! levels, and writing one back moves the lines of those after it, so the document is read again
//! before the next level, the texts written so far held in memory over the files.
//!
//! A level whose file, or a file above it, declares a language version that the reading does not
//! interpret is not solved: none of the RNL of that file is read, and the file is kept as it is
//! (16.1), so nothing is written into it.
//!
//! [`solve`] does the work without writing a file; [`write`] puts the texts where `-o DIR` or
//! `--in-place` says, with the other files the document reads copied under `DIR`; [`lines`] is
//! what the command prints of the levels.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rnl_runtime::diag::Severity;
use rnl_runtime::host::file_name;
use rnl_runtime::interp::unit::{DRAFT_VERSION, SUPPORTED_VERSION};
use rnl_runtime::problem::{sig1, FactKind, LevelProblem, PointRef, Reference};
use rnl_runtime::solution::{write_record, Written};
use rnl_runtime::source::{dirname, normalize, relative_to, Layered, MemoryProvider, Provider};
use rnl_runtime::writeback::{generated_files, remove_generated_with_header, replace_generated_with_header};
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

/// A level that is not solved, because the reading does not interpret its RNL (16.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotSolved {
    pub title: String,
    /// "file" or "subckt".
    pub kind: &'static str,
    /// The path of the file the level's text is in, as it was read.
    pub path: String,
    /// Why: the file that declares a version the reading does not interpret, and the version.
    pub reason: String,
    /// How many of [`SolveRun::levels`] come before it in the order of the document's levels.
    pub before: usize,
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
    /// The levels solved, in the order of the document's drawing levels.
    pub levels: Vec<SolvedLevel>,
    /// The levels asked for that are not solved, because their RNL is not interpreted (16.1).
    pub not_solved: Vec<NotSolved>,
    /// Every file a level was written back into, in the order it was first written.
    pub files: Vec<SolvedFile>,
    /// Every file the document was read from, normalized and in the order first read: the file
    /// solved, the files it includes, the package files named by `path=` (16.2). Not the packages
    /// of the package directories, which a file reaches by name, nor the bundled ones.
    pub read: Vec<String>,
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

/// The files given by another provider, with the paths of those it gave noted.
struct Noting<'a> {
    files: &'a dyn Provider,
    read: RefCell<Vec<String>>,
}

impl Provider for Noting<'_> {
    fn read(&self, path: &str) -> Option<String> {
        let text = self.files.read(path)?;
        let path = normalize(path);
        let mut read = self.read.borrow_mut();
        if !read.contains(&path) {
            read.push(path);
        }
        Some(text)
    }

    fn package_files(&self) -> Vec<String> {
        self.files.package_files()
    }
}

/// Reads `file` as a solver reads it, solves every drawing level, or the one `options.level`
/// names, and writes each level's record of the solver back into the text of its file: the
/// texts are in [`SolveRun::files`], and nothing is written to disk. A record of the solver that
/// applies to the level from another file, written there with scope=, is removed from that file,
/// and each file written gets the header record it lacks (16.1). A level whose RNL is not
/// interpreted, because its file or one above it declares a version the reading does not read,
/// is not solved and nothing is written into it (16.1). Fails when the file cannot be read, or
/// when no level has the title asked for.
pub fn solve(file: &Path, files: &dyn Provider, options: &SolveOptions) -> Result<SolveRun, String> {
    let entry = path_str(file);
    let runtime = options.runtime();
    let canonical = options.solver.canonical();
    let files = Noting { files, read: RefCell::new(Vec::new()) };
    let mut memory = MemoryProvider::new();
    let read = |memory: &MemoryProvider| -> Result<Interpretation, String> {
        let both = Layered { layers: vec![memory, &files] };
        rnl_view::interpret(&entry, options.dialect.as_deref(), &both, &runtime)
    };
    let first = read(&memory)?;
    let errors = first.diagnostics().iter().filter(|d| d.severity == Severity::Error).count();
    let mut targets: Vec<(usize, String)> = Vec::new();
    let mut not_solved = Vec::new();
    for p in first.problems.iter().filter(|p| options.level.as_ref().is_none_or(|t| *t == p.title)) {
        match first.unsupported.get(p.level).cloned().flatten() {
            Some((at, version)) => {
                let mut reason = format!(
                    "{} declares the version {version}, which this reading does not interpret: none of its RNL is \
                     read, and it is kept as it is (16.1)",
                    file_name(&first.doc.levels[at].path)
                );
                if version == DRAFT_VERSION && !options.draft {
                    reason.push_str(&format!("; --draft reads the draft {DRAFT_VERSION} as well"));
                }
                let path = normalize(&first.doc.levels[p.level].path);
                not_solved.push(NotSolved {
                    title: p.title.clone(),
                    kind: p.kind,
                    path,
                    reason,
                    before: targets.len(),
                });
            }
            None => targets.push((p.level, p.title.clone())),
        }
    }
    if targets.is_empty() && not_solved.is_empty() {
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
        let (record, texts) = {
            let both = Layered { layers: vec![&memory, &files] };
            let record = rnl_view::finish_record(&r.doc, &both, &runtime, p, &written.record);
            // the text as the adapter read it, which the positions of its records refer to
            let text_of = |file: &str, path: &str| match r.doc.source(file) {
                Some(t) => Ok(t.to_string()),
                None => both.read(path).ok_or_else(|| format!("cannot read {path}")),
            };
            // the record and the header in one pass, at the positions of this reading: a header
            // below the level's end keeps its place
            let text = text_of(&host.file, &path)?;
            let out = replace_generated_with_header(&r.doc, p.level, &text, &record, &p.version);
            let mut texts = vec![(path.clone(), out)];
            // a record of the solver for this level written with scope= in another file (2.6)
            for other in generated_files(&r.doc, p.level).into_iter().filter(|f| *f != host.file) {
                let Some(lv) = r.doc.levels.iter().find(|l| l.file == other) else { continue };
                let at = normalize(&lv.path);
                let text = text_of(&other, &at)?;
                let out = remove_generated_with_header(&r.doc, p.level, &other, &text, &version_of(&r, &other));
                texts.push((at, out));
            }
            (record, texts)
        };
        for (at, out) in texts {
            memory.insert(&at, &out);
            match solved_files.iter_mut().find(|f| f.path == at) {
                Some(f) => f.text = out,
                None => solved_files.push(SolvedFile { path: at, text: out }),
            }
        }
        levels.push(SolvedLevel { problem: p.clone(), solved, written, record, hash, path, time });
    }
    let packages: Vec<String> = files.files.package_files().iter().map(|p| normalize(p)).collect();
    let read: Vec<String> = files.read.take().into_iter().filter(|p| !packages.contains(p)).collect();
    Ok(SolveRun { entry, levels, not_solved, files: solved_files, read, errors })
}

/// The version a file of the document was read in (16.1): that of a level in it, else the tool's
/// default, which a file without a header is read in.
fn version_of(r: &Interpretation, file: &str) -> String {
    r.problems
        .iter()
        .find(|q| r.doc.levels.get(q.level).is_some_and(|l| l.file == file))
        .map_or_else(|| SUPPORTED_VERSION.to_string(), |q| q.version.clone())
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

/// What [`write`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Writing {
    /// The files a level was written back into, where they were written, in the order of
    /// [`SolveRun::files`].
    pub written: Vec<PathBuf>,
    /// Under `-o DIR`, the other files the document reads from the directory of the file solved
    /// ([`SolveRun::read`]), copied as they are, so that the files under `DIR` read as the files
    /// solved do: the files included, the packages named by `path=`.
    pub copied: Vec<PathBuf>,
    /// Under `-o DIR`, the files the document reads from outside the directory of the file
    /// solved: they have no name under `DIR` and are not copied, so the files there reach them
    /// only where they name them by an absolute path.
    pub outside: Vec<String>,
    /// What stopped the writing. The files listed were written before it.
    pub error: Option<String>,
}

/// The byte order mark of UTF-8.
const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// A text written back as its file was, which [`crate::FsProvider`] read: with the byte order
/// mark again when the file began with one, and in Latin-1 when it was not UTF-8 and every
/// character of the text is one of Latin-1. Otherwise UTF-8.
pub fn encode_as(original: Option<&[u8]>, text: &str) -> Vec<u8> {
    match original {
        Some(bytes) if std::str::from_utf8(bytes).is_err() && text.chars().all(|c| u32::from(c) < 0x100) => {
            text.chars().map(|c| u32::from(c) as u8).collect()
        }
        Some(bytes) if bytes.starts_with(&BOM) && !text.as_bytes().starts_with(&BOM) => {
            [&BOM[..], text.as_bytes()].concat()
        }
        _ => text.as_bytes().to_vec(),
    }
}

/// Writes the files of a run where `target` says, creating the directories they need: the files
/// a level was written back into, each in the encoding its file had ([`encode_as`]), then, under
/// `-o DIR` and when there is any, a copy of each other file the document reads from the
/// directory of the file solved. A copy onto the file itself is left out. It stops at the first
/// file it cannot write, and says what it wrote before. Nothing is written when a file solved
/// has no place under `DIR` ([`destinations`]).
pub fn write(run: &SolveRun, target: &Target) -> Writing {
    let mut out = Writing::default();
    let to = match destinations(run, target) {
        Ok(to) => to,
        Err(e) => {
            out.error = Some(e);
            return out;
        }
    };
    let make_parent = |path: &Path| -> Result<(), String> {
        match path.parent().filter(|p| !p.as_os_str().is_empty()) {
            Some(parent) => {
                std::fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))
            }
            None => Ok(()),
        }
    };
    for (f, path) in run.files.iter().zip(to) {
        let original = std::fs::read(&f.path).ok();
        let done = make_parent(&path).and_then(|()| {
            std::fs::write(&path, encode_as(original.as_deref(), &f.text))
                .map_err(|e| format!("cannot write {}: {e}", path.display()))
        });
        if let Err(e) = done {
            out.error = Some(e);
            return out;
        }
        out.written.push(path);
    }
    let Target::Dir(dir) = target else { return out };
    if run.files.is_empty() {
        return out;
    }
    let base = dirname(&normalize(&run.entry));
    for path in run.read.iter().filter(|p| !run.files.iter().any(|f| f.path == **p)) {
        let Some(rel) = name_in(path, &base) else {
            out.outside.push(path.clone());
            continue;
        };
        let copy = dir.join(rel);
        let same = match (std::fs::canonicalize(path), std::fs::canonicalize(&copy)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        };
        if same {
            continue;
        }
        let done = make_parent(&copy).and_then(|()| {
            std::fs::copy(path, &copy).map(|_| ()).map_err(|e| format!("cannot copy {path} to {}: {e}", copy.display()))
        });
        if let Err(e) = done {
            out.error = Some(e);
            return out;
        }
        out.copied.push(copy);
    }
    out
}

/// A point of a fact, by its object's name when it has one, such as `%t` for a text named so, else
/// as diagnostics show its object: `#M1`, `#M1:d`, `{#M1 #M2}` for a centre.
fn point(p: &LevelProblem, r: &PointRef) -> String {
    let label = |k: usize| {
        p.objects.get(k).map_or_else(
            || format!("object {k}"),
            |o| match &o.reference {
                Reference::Name(n) => n.clone(),
                _ => o.label.clone(),
            },
        )
    };
    match r {
        PointRef::Origin(k) => label(*k),
        PointRef::Pin { object, terminal, .. } => format!("{}:{terminal}", label(*object)),
        PointRef::Centre(points) => format!("{{{}}}", points.iter().map(|q| point(p, q)).collect::<Vec<_>>().join(" ")),
    }
}

/// One fact as the report shows it: its rank, what it says, its priority and where it is
/// written, and the definitions of the usages it was expanded from, such as
/// `[9] ALIGN y {#M1 #M2} (normal, rnl-std-layout.rnl:10, from $MIRROR_PAIR)`.
pub fn describe_fact(p: &LevelProblem, k: usize) -> String {
    let Some(f) = p.facts.get(k) else { return format!("[{k}]") };
    let list = |ms: &[PointRef]| format!("{{{}}}", ms.iter().map(|m| point(p, m)).collect::<Vec<_>>().join(" "));
    let objects = |ks: &[usize]| list(&ks.iter().map(|k| PointRef::Origin(*k)).collect::<Vec<_>>());
    let what = match &f.kind {
        FactKind::Place { at, axis, value } => format!("the {} of {} = {}", axis.as_str(), point(p, at), num(*value)),
        FactKind::PointCoordinate { point: k, axis, value } => {
            format!("the {} of {} = {}", axis.as_str(), point(p, &PointRef::Origin(*k)), num(*value))
        }
        // the `dx` or `dy` of a text, from the point it is attached to (13.3)
        FactKind::Offset { text, from, axis, value } => format!(
            "the d{} of {} from {} = {}",
            axis.as_str(),
            point(p, &PointRef::Origin(*text)),
            point(p, from),
            num(*value)
        ),
        FactKind::Orient { object, code } => {
            format!("the orientation {} of {}", code.as_str(), point(p, &PointRef::Origin(*object)))
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
    // a handle, such as %c1, or a label of its own; the label of a coordinate, an offset or an
    // orientation says what the fact says, so the fact is described once
    let own = matches!(
        f.kind,
        FactKind::Place { .. } | FactKind::PointCoordinate { .. } | FactKind::Offset { .. } | FactKind::Orient { .. }
    );
    let named = if own || f.label == f.kind.name() || f.label == what { what } else { format!("{} = {what}", f.label) };
    let mut context = vec![f.priority.as_str().to_string()];
    if let Some(l) = &f.source {
        context.push(format!("{}:{}", file_name(&l.file), l.line));
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

/// What the command prints of the levels of a run, in the order of the document's levels: each
/// level solved as [`report`] gives it, each level not solved in one line.
pub fn lines(run: &SolveRun, all: bool) -> Vec<String> {
    let mut out = Vec::new();
    for k in 0..=run.levels.len() {
        for n in run.not_solved.iter().filter(|n| n.before == k) {
            out.push(not_solved(n));
        }
        if let Some(level) = run.levels.get(k) {
            out.extend(report(level, all));
        }
    }
    out
}

/// The line of a level that is not solved.
pub fn not_solved(n: &NotSolved) -> String {
    format!(
        "level {} ({} in {}): not solved, and nothing is written for it: {}",
        n.title,
        n.kind,
        file_name(&n.path),
        n.reason
    )
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
        file_name(&level.path),
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

    #[test]
    fn a_text_is_written_back_in_the_encoding_of_its_file() {
        let text = "* a \u{b5}A\n.end\n";
        // UTF-8, with and without its byte order mark
        assert_eq!(encode_as(None, text), text.as_bytes());
        assert_eq!(encode_as(Some(b"* old\n"), text), text.as_bytes());
        let marked = [&BOM[..], b"* old\n"].concat();
        assert_eq!(encode_as(Some(&marked), text), [&BOM[..], text.as_bytes()].concat());
        // Latin-1, as long as every character is one of it
        let latin1 = b"* old \xb5A\n";
        assert_eq!(encode_as(Some(latin1), text), b"* a \xb5A\n.end\n");
        let wide = "* \u{3a9}\n";
        assert_eq!(encode_as(Some(latin1), wide), wide.as_bytes());
    }
}
