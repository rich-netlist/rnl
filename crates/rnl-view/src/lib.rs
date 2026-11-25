//! The core interface of decision 0008: a workspace in, a Document out. A frontend names the file
//! to open and gives its own file access; the core reads what the document needs through it, with
//! the standard packages bundled here, and returns what a viewer shows.
//!
//! The crate does no file I/O, so it builds for `wasm32-unknown-unknown`.

mod catalog;
mod constraints;
mod document;
mod file_result;
pub mod folder;
mod formats;
pub mod hints;
mod library;
mod outline;
pub mod overlays;
mod params;
#[cfg(test)]
mod pin_attachments;
#[cfg(test)]
mod pin_objects;
mod primitive;
mod reading_cache;
pub use overlays::{CurrentArrow, OverlayOptions, OverlayStyle, UsageFrame, UsageMode};
mod snapshot;
#[cfg(test)]
mod types;
mod workspace;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use rnl_runtime::diag::Sink;
use rnl_runtime::host::format::Reading;
use rnl_runtime::host::{file_name, package, FileFormat, HostDocument};
use rnl_runtime::model::LevelView;
use rnl_runtime::previous::{solve_status, Previous, SolveStatus};
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::source::{dirname, normalize, prefer_filename, Provider};
use rnl_runtime::writeback::record_type;
use rnl_runtime::{Interpretation, Options};

pub use catalog::{
    Catalog, CatalogDefinition, CatalogLabel, CatalogOptions, CatalogPackage, CatalogPin, CatalogSet, CatalogSymbol,
    Terminal,
};
pub use constraints::{
    constraint, solver_settings, Constraint, ConstraintDirectedPin, ConstraintGroup, ConstraintKind, ConstraintObject,
    ConstraintPoint, ConstraintReference, ConstraintRoute, ConstraintSource, ConstraintTableCell, ConstraintUsage,
    LevelConstraints, SolverLiteral, SolverSettingView,
};
pub use document::{
    AutoIncludeCandidate, Child, Diagnostic, Document, Drawings, FileHints, FileRest, Hit, InspectedLabel,
    InspectedPin, InspectedSymbol, Level, LevelHint, MemberOf, NetLabel, Object, PinObject, RoleOf, RuleAt, SourceFile,
    SourceInclude, Span, Unplaced, UnplacedDrawing, Usage, UsageRole,
};
pub use file_result::{
    core_version, FileCall, FileChild, FileInput, FileInputs, FileOutline, FileResult, FileSymbols, SubcktSymbol,
    FILE_RESULT_FORMAT,
};
pub use formats::{detect_format, formats, is_listed, listed_suffixes, validate_format, FormatInfo};
pub use library::{LibraryIndex, LibraryProvider};
pub use outline::{Outline, OutlineCall, OutlineFile, OutlineLevel};
pub use params::{Bind, ParamValue, SubcktParam};
pub use primitive::{
    Bar, Cap, Color, HAlign, Join, Layer, Primitive, Run, RunStyle, Segment, Shape, Stroke, Text, TextRun, Unit, VAlign,
};
/// Texts held in memory, for a frontend that builds one, such as a netlist from the clipboard.
pub use reading_cache::ReadingCache;
pub use rnl_runtime::source::MemoryProvider;
pub use snapshot::{
    FileRef, GroupView, HostInstance, HostLevel, HostNet, HostTerminal, LevelModel, LevelSnapshot, ObjectView,
    PackageUse, Parts, PinLabel, PinView, PointView, ShapeView, Slot, Snapshot, SymbolDefinition, SymbolPin,
    SymbolPrim, SymbolRef, TextView, UsageOf, Vertex, WireView, SCHEMA,
};
pub use workspace::{Opened, RequestFiles};

include!(concat!(env!("OUT_DIR"), "/bundled.rs"));

/// The directory of the bundled packages, as the runtime names it. Each version of the standard
/// series has a directory of its own in it, as in `libraries/standard`, such as `<bundled>/0.4`.
/// Extension and PDK packages are supplied by the frontend or its search paths.
pub const BUNDLED_DIR: &str = "<bundled>";

/// The bundled packages, each directory a package directory of its own (16.2).
pub fn bundled() -> MemoryProvider {
    let mut p = MemoryProvider::default();
    for &(path, text) in BUNDLED {
        p.insert(&format!("{BUNDLED_DIR}/{path}"), text);
        let dir = format!("{BUNDLED_DIR}/{}", dirname(path));
        if !p.package_dirs.contains(&dir) {
            p.package_dirs.push(dir);
        }
    }
    p
}

/// The bundled package files of a draft series, by their paths in [`bundled`]: those written in a language version newer than the tool's default
/// (16.1), whose standard is not released yet. A frontend that lists the bundled packages, as the
/// desktop viewer does, leaves them out, as the web viewer's library does; the core reads them all
/// the same, so a file of that version uses them (16.3).
pub fn bundled_drafts() -> BTreeSet<String> {
    let key = rnl_runtime::interp::catalog::version_key;
    let draft = |text: &str| {
        let language = rnl_runtime::interp::unit::scan_language_decl(text);
        language.is_some_and(|l| key(&l) > key(rnl_runtime::interp::unit::DEFAULT_VERSION))
    };
    bundled().files.into_iter().filter(|(_, text)| draft(text)).map(|(path, _)| path).collect()
}

/// The bundled package files a frontend lists among its built-in packages, by their paths in [`bundled`] with
/// their texts: all but those of a draft series ([`bundled_drafts`]). Every viewer lists these and only these.
pub fn bundled_listed() -> BTreeMap<String, String> {
    let drafts = bundled_drafts();
    bundled().files.into_iter().filter(|(path, _)| !drafts.contains(path)).collect()
}

/// How to build a Document, and what to give of each level's drawing.
#[derive(Clone, Debug)]
pub struct BuildOptions {
    /// The format of the entry, "spice" or "spectre", instead of guessing from it.
    pub dialect: Option<String>,
    /// The format set for files, by path as the frontend names them, as it names the entry: only
    /// those the user set. A file not among them is read by [`detect_format`], by its suffix and
    /// its first lines, and the entry by `dialect` before them. An unknown format name fails the build; a
    /// file the netlist does not read is left alone. Paths compare by [`Provider::path_key`].
    pub formats: BTreeMap<String, String>,
    /// The runtime's options, such as using the standard package at the root (16.3), or reading
    /// the levels without their generated records, as a solver does (2.8).
    pub runtime: Options,
    /// Write each level's SVG ([`Level::svg`]); on by default.
    pub svg: bool,
    /// Give each level's primitives ([`Level::primitives`]); off by default.
    pub primitives: bool,
    /// Draw each level's flylines (13.3, 13.7) in its SVG and primitives, those of every net, to
    /// check what is not wired; off by default. In the SVG each is a group of class "flyline",
    /// which a viewer can hide, and each has a hit ([`Hit`]).
    pub flylines: bool,
    /// Give each level its geometry hints ([`Level::hints`], [`hints`]), as `rnl check` prints
    /// them; off by default, and experimental. They take the problems of the levels, for the lines
    /// of the records that place the objects they name.
    pub hints: bool,
    /// Give each level its solve status ([`Level::solve`]), what a frontend decides from whether
    /// to solve it (decision 0008); off by default. It takes the problems of the levels read
    /// without their records of the solver, so a document that has such records is read once more.
    pub status: bool,
    /// Give each level its constraints after expansion ([`Level::constraints`]), with no solver;
    /// off by default, and experimental. They are the problems of the levels read without their
    /// records of the solver, so a document that has such records is read once more.
    pub constraints: bool,
    /// The values to build a subcircuit's parameters with, for showing ([`Bind`]): only the
    /// Document's lists of parameters change ([`Level::params`], [`Child::effective`]).
    pub bind: Option<Bind>,
}

impl Default for BuildOptions {
    fn default() -> BuildOptions {
        BuildOptions {
            dialect: None,
            formats: BTreeMap::new(),
            runtime: Options::default(),
            svg: true,
            primitives: false,
            flylines: false,
            hints: false,
            status: false,
            constraints: false,
            bind: None,
        }
    }
}

/// Build the Document of `entry`, the path of the file to open as the frontend names it,
/// `/`-separated (a `\` is read as `/`). `files` is the frontend's file access: the disk, or texts
/// held in memory, untitled ones included. The bundled packages are layered under it. Fails when
/// the entry itself cannot be read.
///
/// Name the files of a workspace by rooted paths, such as `/w/top.sp`, as the desktop names them
/// by absolute ones: see [`SourceFile::path`] for what a relative entry in a directory loses.
pub fn build(entry: &str, files: &dyn Provider, options: &BuildOptions) -> Result<Document, String> {
    Ok(build_in(entry, files, options, false, &[], None, false, |_, d| d)?.0)
}

/// [`build`], taking each reading it needs from `taken` where one was read with options that read
/// the document alike ([`Options::reads_as`]) instead of reading it again. Each must be a reading
/// of `entry` over the same texts as `files`, with the same `dialect` and `formats`
/// ([`read_noting`]): the Document is then the one `build` gives.
pub fn build_taking(
    entry: &str,
    files: &dyn Provider,
    options: &BuildOptions,
    taken: &[&DocumentReading],
) -> Result<Document, String> {
    Ok(build_in(entry, files, options, false, taken, None, false, |_, d| d)?.0)
}

/// A reading of a document, as [`read_noting`] gives it: what it was read with, the interpretation,
/// and the files it looked for and did not find, in that order, which a Document lists
/// ([`Document::missing`]).
#[derive(Clone)]
pub struct DocumentReading {
    pub options: Options,
    pub interpretation: std::rc::Rc<Interpretation>,
    pub missing: Vec<String>,
}

/// [`interpret_with`], with the files the reading missed, for a build to take it
/// ([`build_taking`]).
pub fn read_noting(
    entry: &str,
    dialect: Option<&str>,
    formats: &BTreeMap<String, String>,
    files: &dyn Provider,
    options: &Options,
) -> Result<DocumentReading, String> {
    read_noting_in(entry, dialect, formats, files, options, None)
}

/// [`read_noting`], the objects of the authored records taken from `counts` when it has them, and
/// `counts` given those the reading counts ([`rnl_runtime::interpret_taking`] says what they have
/// to be).
pub fn read_noting_taking(
    entry: &str,
    dialect: Option<&str>,
    formats: &BTreeMap<String, String>,
    files: &dyn Provider,
    options: &Options,
    counts: &mut rnl_runtime::AuthoredCounts,
) -> Result<DocumentReading, String> {
    read_noting_in(entry, dialect, formats, files, options, Some(counts))
}

/// [`read_noting`], taking and giving the objects of the authored records with `counts` when it is
/// given ([`read_noting_taking`]).
fn read_noting_in(
    entry: &str,
    dialect: Option<&str>,
    formats: &BTreeMap<String, String>,
    files: &dyn Provider,
    options: &Options,
    counts: Option<&mut rnl_runtime::AuthoredCounts>,
) -> Result<DocumentReading, String> {
    let ws = Workspace::with_formats(files, formats);
    let r = match package_in(entry, (dialect, formats), &ws, options)? {
        Some(r) => r,
        None => {
            let mut sink = Sink::new();
            let doc = read_host_in(entry, (dialect, formats), &ws, &mut sink)?;
            match counts {
                Some(counts) => rnl_runtime::interpret_taking(doc, &ws, options, sink, counts),
                None => rnl_runtime::interpret(doc, &ws, options, sink),
            }
        }
    };
    Ok(DocumentReading {
        options: options.clone(),
        interpretation: std::rc::Rc::new(r),
        missing: ws.missing.into_inner(),
    })
}

/// [`build`], with the drawings of the objects that each level lists without a position kept apart
/// from the Document: a netlist with no positions lists every object, and each drawing is a few
/// hundred bytes of SVG, which the frontend asks for as it shows them ([`Drawings::unplaced`])
/// instead of carrying them all. The Document is the one `build` gives but for the `svg` and
/// `primitives` of `Level::unplaced`, which it has none of.
pub fn build_drawn(entry: &str, files: &dyn Provider, options: &BuildOptions) -> Result<(Document, Drawings), String> {
    let (document, drawings) = build_with(entry, files, options, true)?;
    Ok((document, drawings.expect("drawings were asked for")))
}

/// The Document of `entry`, and when `lazy` the drawings of the objects without a position.
fn build_with(
    entry: &str,
    files: &dyn Provider,
    options: &BuildOptions,
    lazy: bool,
) -> Result<(Document, Option<Drawings>), String> {
    let (document, drawings, _) = build_in(entry, files, options, lazy, &[], None, false, |_, d| d)?;
    Ok((document, drawings))
}

/// The reading of one file of the document `entry`, `file`, as the document names it
/// ([`SourceFile::path`], [`Outline`]'s files), in the context of the entry
/// ([`rnl_runtime::Options::only_file`]): the Document of the file's levels, each the level the
/// whole document's Document has, with the subcircuits they instantiate and what the reading
/// depends on ([`FileResult`]). `files` and `options` are those of [`build`]; `options.runtime`'s
/// `only_file` is set to `file`. Fails as [`build`] does, and when `file` is not a file of the
/// document and the document misses no file: when it misses some, the result has no level, and
/// lists them in `missing` for the frontend to read and ask again.
pub fn build_file(entry: &str, file: &str, files: &dyn Provider, options: &BuildOptions) -> Result<FileResult, String> {
    Ok(build_file_with(entry, file, files, options, false, &[], None)?.0)
}

/// [`build_file`], taking the host document from `cache` when it would be read the same, and keeping
/// it there for the calls that follow on the document ([`ReadingCache`]). The result is the one
/// [`build_file`] gives. Without [`BuildOptions::hints`], the reading is kept in `cache` for
/// [`file_hints_cached`] to give the hints of.
pub fn build_file_cached(
    entry: &str,
    file: &str,
    files: &dyn Provider,
    options: &BuildOptions,
    cache: &mut ReadingCache,
) -> Result<FileResult, String> {
    with_cache(cache, |cache| Ok(build_file_with(entry, file, files, options, false, &[], Some(cache))?.0))
}

/// [`build_file_drawn`], taking the host document from `cache` as [`build_file_cached`] does.
pub fn build_file_drawn_cached(
    entry: &str,
    file: &str,
    files: &dyn Provider,
    options: &BuildOptions,
    cache: &mut ReadingCache,
) -> Result<(FileResult, Drawings), String> {
    with_cache(cache, |cache| {
        let (result, drawings) = build_file_with(entry, file, files, options, true, &[], Some(cache))?;
        Ok((result, drawings.expect("drawings were asked for")))
    })
}

/// The geometry hints of the levels of `file`, a file of `entry`, from the reading of it that the
/// last [`build_file_cached`] or [`build_file_drawn_cached`] through `cache` made without
/// [`BuildOptions::hints`], without reading again: those that call would have given with `hints`
/// ([`Level::hints`]), by level. The reading must be of the same `entry` and `file`, with options
/// that read alike, and every question it asked of `files` must have the same answer now; else
/// this fails with [`NOT_KEPT`], and the frontend asks for the file with `hints`. The reading is
/// given up: a second call fails. In debug builds the file is read again with `hints` and checked.
pub fn file_hints_cached(
    entry: &str,
    file: &str,
    files: &dyn Provider,
    options: &BuildOptions,
    cache: &mut ReadingCache,
) -> Result<FileHints, String> {
    let key = reading_key(entry, file, options);
    let Some(kept) = cache.take_reading(&key, files) else { return Err(NOT_KEPT.to_string()) };
    let hints = document::file_hints(entry, &kept.reading, &kept.files);
    #[cfg(debug_assertions)]
    check_rest(entry, file, files, options, &hints, None);
    Ok(hints)
}

/// What the answer of `file`, a file of `entry`, was left without, from the reading of it that
/// the cache keeps with its spans ([`ReadingCache::keep_spans`]), without reading again: its spans,
/// as [`Document::spans`] has them, and the hints of its levels, as [`file_hints_cached`] gives
/// them. It needs what [`file_hints_cached`] needs, and fails with [`NOT_KEPT`] as it does, and
/// when the spans were not kept. The reading is given up. In debug builds the file is read again
/// with `hints` and checked.
pub fn file_rest_cached(
    entry: &str,
    file: &str,
    files: &dyn Provider,
    options: &BuildOptions,
    cache: &mut ReadingCache,
) -> Result<FileRest, String> {
    let key = reading_key(entry, file, options);
    let Some(mut kept) = cache.take_reading(&key, files) else { return Err(NOT_KEPT.to_string()) };
    let Some(spans) = kept.spans.take() else { return Err(NOT_KEPT.to_string()) };
    let hints = document::file_hints(entry, &kept.reading, &kept.files);
    #[cfg(debug_assertions)]
    check_rest(entry, file, files, options, &hints, Some(&spans));
    Ok(FileRest { spans, hints })
}

/// Leave the texts of `result`'s files out, for a frontend that has them: each [`SourceFile::text`]
/// is made empty, but those of the bundled packages ([`BUNDLED_DIR`]), which the frontend did not
/// give. A file's path is relative to the directory of the entry when it is in it, as a bundled
/// package read as the entry is. The frontend puts back the texts of the files it gave; the texts
/// left out are given, by [`SourceFile::path`], those that were not empty, for a frontend that keeps
/// them to put back.
pub fn leave_out_texts(result: &mut FileResult) -> HashMap<String, String> {
    let dir = dirname(&result.document.path);
    let mut taken = HashMap::new();
    for f in &mut result.document.files {
        let bundled =
            f.path.starts_with(BUNDLED_DIR) || rnl_runtime::source::join(&dir, &f.path).starts_with(BUNDLED_DIR);
        if !bundled && !f.text.is_empty() {
            taken.insert(f.path.clone(), std::mem::take(&mut f.text));
        }
    }
    taken
}

/// What a reading of `file` through the cache is of besides the files ([`reading_cache::ReadingKey`]).
fn reading_key(entry: &str, file: &str, options: &BuildOptions) -> reading_cache::ReadingKey {
    reading_cache::ReadingKey {
        host: host_key(entry, (options.dialect.as_deref(), &options.formats)),
        file: file.to_string(),
        runtime: Options { only_file: Some(file.to_string()), ..options.runtime.clone() },
    }
}

/// Check the hints, and the spans when given, that a reading kept gave against `file` read again
/// with `hints`.
#[cfg(debug_assertions)]
fn check_rest(
    entry: &str,
    file: &str,
    files: &dyn Provider,
    options: &BuildOptions,
    hints: &FileHints,
    spans: Option<&[Span]>,
) {
    let fresh = build_file(entry, file, files, &BuildOptions { hints: true, ..options.clone() })
        .expect("a file kept reads again");
    fn json<T: serde::Serialize + ?Sized>(value: &T) -> String {
        serde_json::to_string(value).expect("serializes")
    }
    let want: BTreeMap<usize, String> = fresh.document.levels.iter().map(|l| (l.host, json(&l.hints))).collect();
    let got: BTreeMap<usize, String> = hints.levels.iter().map(|(host, h)| (*host, json(h))).collect();
    assert_eq!(got, want, "the hints of the reading kept of {file} changed");
    if let Some(spans) = spans {
        assert!(json(spans) == json(&fresh.document.spans), "the spans kept of {file} changed");
    }
}

/// The error of [`file_hints_cached`] when the cache keeps no reading of the file to give the hints
/// of.
pub const NOT_KEPT: &str = "not kept";

/// `call` with `cache` lent as a cell, which the workspace of a reading holds; it is given back
/// after, whatever `call` gives.
fn with_cache<T>(cache: &mut ReadingCache, call: impl FnOnce(&RefCell<ReadingCache>) -> T) -> T {
    let cell = RefCell::new(std::mem::take(cache));
    let out = call(&cell);
    *cache = cell.into_inner();
    out
}

/// [`build_file`], taking the readings it needs from `taken` as [`build_taking`] does. A reading of
/// the file alone is one with its `only_file` set to `file`.
pub fn build_file_taking(
    entry: &str,
    file: &str,
    files: &dyn Provider,
    options: &BuildOptions,
    taken: &[&DocumentReading],
) -> Result<FileResult, String> {
    Ok(build_file_with(entry, file, files, options, false, taken, None)?.0)
}

/// [`build_file`], with the drawings of the objects without a position kept apart, as
/// [`build_drawn`] keeps them.
pub fn build_file_drawn(
    entry: &str,
    file: &str,
    files: &dyn Provider,
    options: &BuildOptions,
) -> Result<(FileResult, Drawings), String> {
    let (result, drawings) = build_file_with(entry, file, files, options, true, &[], None)?;
    Ok((result, drawings.expect("drawings were asked for")))
}

fn build_file_with(
    entry: &str,
    file: &str,
    files: &dyn Provider,
    options: &BuildOptions,
    lazy: bool,
    taken: &[&DocumentReading],
    cache: Option<&RefCell<ReadingCache>>,
) -> Result<(FileResult, Option<Drawings>), String> {
    let runtime = Options { only_file: Some(file.to_string()), ..options.runtime.clone() };
    let options = BuildOptions { runtime, ..options.clone() };
    // a reading through the cache without the hints is kept for them, with every question it asks
    // of the files (file_hints_cached)
    let keep = cache.is_some() && !options.hints;
    if let Some(cache) = cache {
        cache.borrow_mut().drop_reading();
    }
    let asked = RefCell::new(Vec::new());
    let noting = keep.then(|| reading_cache::noting(files, &asked));
    let read_from: &dyn Provider = match &noting {
        Some(noting) => noting,
        None => files,
    };
    let built = build_in(entry, read_from, &options, lazy, taken, cache, keep, |r, d| {
        file_result::file_result(entry, file, r, d)
    });
    let (result, drawings, reading) = built?;
    // a file not read yet may be among those missing, which a frontend reads to ask again
    if !result.document.files.iter().any(|f| f.path == file) && result.document.missing.is_empty() {
        return Err(format!("{file} is not a file of {entry}"));
    }
    if let (Some(cache), Some(reading)) = (cache, reading) {
        let key = reading_cache::ReadingKey {
            host: host_key(entry, (options.dialect.as_deref(), &options.formats)),
            file: file.to_string(),
            runtime: options.runtime.clone(),
        };
        let files = result.document.files.iter().enumerate().map(|(k, f)| (f.path.clone(), k)).collect();
        cache.borrow_mut().keep_reading(key, asked.into_inner(), reading, files);
    }
    Ok((result, drawings))
}

/// The symbols the subcircuits of `file`, a file of the document `entry` as [`build_file`] names
/// it, give themselves (9.2.6), what the levels that instantiate them use of them, read in the
/// context of the entry without building any level ([`rnl_runtime::Options::symbols_only`]).
/// `files` and the reading options of `options` are those of [`build`]. Fails as [`build_file`]
/// does.
pub fn symbols(entry: &str, file: &str, files: &dyn Provider, options: &BuildOptions) -> Result<FileSymbols, String> {
    let ws = Workspace::with_formats(files, &options.formats);
    if ws.read(&slashed(entry)).is_none() {
        return Err(format!("cannot read {entry}"));
    }
    let runtime =
        Options { only_file: Some(file.to_string()), symbols_only: true, problems: false, ..options.runtime.clone() };
    let r = interpret_in(entry, (options.dialect.as_deref(), &options.formats), &ws, &runtime)?;
    let missing = ws.missing.into_inner();
    if !r.doc.sources.iter().any(|(p, _)| p == file) && missing.is_empty() {
        return Err(format!("{file} is not a file of {entry}"));
    }
    Ok(file_result::file_symbols(file, &r, missing))
}

/// What [`build_in`] gives: what `finish` made, the drawings kept apart, and the reading kept.
type Built<T> = (T, Option<Drawings>, Option<Box<Interpretation>>);

/// The Document of `entry` made into what `finish` makes of it with its interpretation, and when
/// `lazy` the drawings of the objects without a position. A reading of `taken` made with options
/// that read alike is taken instead of reading again, but when `lazy`, which takes the drawings out
/// of the reading. When `keep`, the reading is built with its problems, which change nothing else,
/// and given too, with its drawings copied rather than taken, for the hints asked for after.
#[allow(clippy::too_many_arguments)]
fn build_in<T>(
    entry: &str,
    files: &dyn Provider,
    options: &BuildOptions,
    lazy: bool,
    taken: &[&DocumentReading],
    cache: Option<&RefCell<ReadingCache>>,
    keep: bool,
    finish: impl FnOnce(&Interpretation, Document) -> T,
) -> Result<Built<T>, String> {
    let ws = Workspace::with_formats(files, &options.formats).cached(cache);
    if ws.read(&slashed(entry)).is_none() {
        return Err(format!("cannot read {entry}"));
    }
    let reading = (options.dialect.as_deref(), &options.formats);
    // a reading taken notes the files it missed, as reading again would
    let read = |runtime: &Options| -> Result<Readed, String> {
        match taken.iter().find(|t| !lazy && t.options.reads_as(runtime)) {
            Some(t) => {
                let mut missing = ws.missing.borrow_mut();
                for m in &t.missing {
                    if !missing.contains(m) {
                        missing.push(m.clone());
                    }
                }
                Ok(Readed::Taken(t.interpretation.clone()))
            }
            None => Ok(Readed::Read(Box::new(interpret_in(entry, reading, &ws, runtime)?))),
        }
    };
    // the solve status and the constraints take the problems; built in this reading too, they
    // change nothing else
    let solver_view = options.status || options.constraints;
    let runtime = Options {
        problems: options.runtime.problems || solver_view || options.hints || keep,
        ..options.runtime.clone()
    };
    let without = Options { generated: false, ..runtime.clone() };
    let taking = |o: &Options| !lazy && taken.iter().any(|t| t.options.reads_as(o));
    let (mut r, other) = if solver_view && runtime.generated && !taking(&runtime) && !taking(&without) {
        // both read here: the reading without the records of the solver first, whose objects of
        // the authored records the reading with them takes instead of counting them (5.9)
        let (r, other) = read_both(entry, reading, &ws, &runtime, &without)?;
        (Readed::Read(Box::new(r)), other.map(|o| Readed::Read(Box::new(o))))
    } else {
        let r = read(&runtime)?;
        // a document without records of the solver reads the same with them and without; looked for
        // only for the solve status and the constraints, as reading each record's head parses it
        let records = || r.doc.levels.iter().any(|l| l.records.iter().any(|rec| record_type(rec) == "generated"));
        let other = if !solver_view || !records() {
            None
        } else if runtime.generated {
            Some(read(&without)?)
        } else {
            Some(read(&Options { generated: true, problems: false, ..runtime.clone() })?)
        };
        (r, other)
    };
    let (with, without): (&Interpretation, &Interpretation) = match &other {
        None => (&r, &r),
        Some(o) if runtime.generated => (&r, o),
        Some(o) => (o, &r),
    };
    let solve = if options.status { solve_statuses(with, without) } else { HashMap::new() };
    let missing = ws.missing.into_inner();
    let mut d = match cache {
        Some(cache) => {
            let mut kept = std::mem::take(&mut cache.borrow_mut().semantics);
            let d = document::document(entry, &r, missing, options, lazy, Some(&mut kept));
            cache.borrow_mut().semantics = kept;
            d
        }
        None => document::document(entry, &r, missing, options, lazy, None),
    };
    for level in &mut d.levels {
        level.solve = solve.get(&level.host).map(|s| s.as_str());
    }
    if options.constraints {
        for (k, lv) in r.model.levels.iter().enumerate() {
            let Some(p) = without.problems.iter().find(|p| p.level == lv.level) else { continue };
            let found = constraints::level_constraints(p, &r, lv, &d.levels[k], &d.objects);
            d.levels[k].constraints = Some(found);
        }
    }
    drop(other);
    let drawings = match &mut r {
        Readed::Read(r) if lazy => Some(Drawings::take(&mut r.model.levels, &d, options, keep)),
        _ => None,
    };
    let out = finish(&r, d);
    let kept = match r {
        Readed::Read(r) if keep => Some(r),
        _ => None,
    };
    Ok((out, drawings, kept))
}

/// The readings of `entry` that [`build_in`] makes with the solve status or the constraints: with
/// the records of the solver, `with`, and, when the document has records of the solver, without
/// them, `without`, which is made first, its objects of the authored records taken by the other
/// ([`rnl_runtime::interpret_taking`]). The host document is read once for both.
fn read_both(
    entry: &str,
    reading: Formats,
    ws: &Workspace,
    with: &Options,
    without: &Options,
) -> Result<(Interpretation, Option<Interpretation>), String> {
    if let Some(r) = package_in(entry, reading, ws, with)? {
        return Ok((r, None));
    }
    let mut sink = Sink::new();
    let doc = read_host_in(entry, reading, ws, &mut sink)?;
    if !doc.levels.iter().any(|l| l.records.iter().any(|rec| record_type(rec) == "generated")) {
        return Ok((rnl_runtime::interpret(doc, ws, with, sink), None));
    }
    let both = move |counts: &mut rnl_runtime::AuthoredCounts| {
        let other = rnl_runtime::interpret_taking(doc.clone(), ws, without, sink.clone(), counts);
        let r = rnl_runtime::interpret_taking(doc, ws, with, sink, counts);
        (r, Some(other))
    };
    Ok(match ws.cache {
        Some(cache) => reading_cache::with_counts(cache, both),
        None => both(&mut rnl_runtime::AuthoredCounts::default()),
    })
}

/// A reading [`build_in`] made, or one it took.
enum Readed {
    Read(Box<Interpretation>),
    Taken(std::rc::Rc<Interpretation>),
}

impl std::ops::Deref for Readed {
    type Target = Interpretation;

    fn deref(&self) -> &Interpretation {
        match self {
            Readed::Read(r) => r,
            Readed::Taken(r) => r,
        }
    }
}

/// The solve status of each drawing level, by host level ([`solve_status`]): its problem and model
/// read without the records of the solver, `without`, against its record of the solver read with
/// them, `with`.
fn solve_statuses(with: &Interpretation, without: &Interpretation) -> HashMap<usize, SolveStatus> {
    let views: HashMap<usize, &LevelView> = without.model.levels.iter().map(|lv| (lv.level, lv)).collect();
    let mut out = HashMap::new();
    for p in &without.problems {
        if let Some(view) = views.get(&p.level) {
            out.insert(p.level, solve_status(p, view, with));
        }
    }
    out
}

/// The snapshot of `entry` for the editor interface, with the parts `parts` asks for, from the
/// same interpretation as its Document; the Document in it is built as `options` ask. Fails as
/// [`build`] does.
pub fn snapshot(entry: &str, files: &dyn Provider, options: &BuildOptions, parts: Parts) -> Result<Snapshot, String> {
    let ws = Workspace::with_formats(files, &options.formats);
    if ws.read(&slashed(entry)).is_none() {
        return Err(format!("cannot read {entry}"));
    }
    let r = interpret_in(entry, (options.dialect.as_deref(), &options.formats), &ws, &options.runtime)?;
    // without the drawing, the Document is built only for its files, levels and diagnostics
    let draw =
        if parts.drawing { options.clone() } else { BuildOptions { svg: false, primitives: false, ..options.clone() } };
    let document = document::document(entry, &r, ws.missing.into_inner(), &draw, false, None);
    Ok(snapshot::snapshot(&r, document, parts, &options.runtime, !options.formats.is_empty()))
}

/// The outline of `entry`: what reading its files gives before any RNL is interpreted, the files,
/// the levels and the subcircuits each calls, the language version and packages of each file, and
/// the diagnostics of reading the netlist ([`Outline`]). `files` and the `dialect` and `formats` of
/// `options` are read as [`build`] reads them; its other options do not change it. A package file
/// ([`entry_format`]) is not read: its outline is the file alone. The package files that records
/// name by path are read as the reading reads them, so that those `files` does not have are among
/// its missing files. Fails as [`build`] does.
pub fn outline(entry: &str, files: &dyn Provider, options: &BuildOptions) -> Result<Outline, String> {
    outline_in(entry, files, options, None)
}

/// [`outline`], taking the host document from `cache` when it would be read the same, and keeping
/// it there for the calls that follow on the document ([`ReadingCache`]). The outline is the one
/// [`outline`] gives.
pub fn outline_cached(
    entry: &str,
    files: &dyn Provider,
    options: &BuildOptions,
    cache: &mut ReadingCache,
) -> Result<Outline, String> {
    with_cache(cache, |cache| outline_in(entry, files, options, Some(cache)))
}

fn outline_in(
    entry: &str,
    files: &dyn Provider,
    options: &BuildOptions,
    cache: Option<&RefCell<ReadingCache>>,
) -> Result<Outline, String> {
    let ws = Workspace::with_formats(files, &options.formats).cached(cache);
    let path = slashed(entry);
    let Some(text) = ws.read(&path) else { return Err(format!("cannot read {entry}")) };
    let reading = (options.dialect.as_deref(), &options.formats);
    let (main, default) = entry_format_in(entry, reading, &ws)?;
    if formats::name(main) == package::NAME {
        let (format, default) = (formats::name(main), formats::name(default));
        return Ok(outline::package_outline(entry, &normalize(&path), &text, format, default));
    }
    let mut sink = Sink::new();
    let doc = read_host_in(entry, reading, &ws, &mut sink)?;
    outline::read_named_packages(&doc, &ws);
    Ok(outline::outline(entry, doc, sink, ws.missing.into_inner()))
}

/// The catalog of the packages the core has: those of the package directories of `files`, the
/// frontend's file access, and the bundled standard series under them. It lists their symbols, with their pins, boxes and labels, their
/// ordinary definitions and their sets, each package file read as `rnl check` reads it, checked on
/// its own with `options.runtime` (2.7); of each name the newest version, or every version with
/// `options.all_versions`, and only the names `options.packages` gives, when it gives them. A file
/// that the runtime does not read as a package is not listed.
pub fn catalog(files: &dyn Provider, options: &CatalogOptions) -> Catalog {
    catalog::catalog(&Workspace::new(files), options)
}

/// Is the entry Spectre? By `dialect`, by the `.scs` extension, or a `simulator lang=spectre`
/// near the top.
pub fn is_spectre(entry: &str, text: &str, dialect: Option<&str>) -> bool {
    if let Some(d) = dialect {
        return d == "spectre";
    }
    if extension(entry).is_some_and(|e| e.eq_ignore_ascii_case("scs")) {
        return true;
    }
    text.lines().take(50).any(|l| {
        let l = l.trim().to_lowercase();
        l.starts_with("simulator") && l.contains("lang") && l.contains("spectre")
    })
}

/// How the files of a netlist are read: the format of the entry, as `dialect` names it, and the
/// formats set for files by path ([`BuildOptions::formats`]).
type Formats<'f> = (Option<&'f str>, &'f BTreeMap<String, String>);

/// No format set for any file.
static NONE_SET: BTreeMap<String, String> = BTreeMap::new();

/// Read a host document, each file with the adapter of its format: the entry's by `dialect` or by
/// [`detect_format`], and every file it includes by [`detect_format`]. The bundled packages are layered
/// under `files`, so an entry among them can be read as well.
pub fn read_host(
    entry: &str,
    dialect: Option<&str>,
    files: &dyn Provider,
    sink: &mut Sink,
) -> Result<HostDocument, String> {
    read_host_in(entry, (dialect, &NONE_SET), &Workspace::new(files), sink)
}

/// [`read_host`], with the formats set for files by path ([`BuildOptions::formats`]).
pub fn read_host_with(
    entry: &str,
    dialect: Option<&str>,
    formats: &BTreeMap<String, String>,
    files: &dyn Provider,
    sink: &mut Sink,
) -> Result<HostDocument, String> {
    read_host_in(entry, (dialect, formats), &Workspace::with_formats(files, formats), sink)
}

/// [`read_host`] in a workspace, which has the bundled packages already.
fn read_host_in(entry: &str, reading: Formats, ws: &Workspace, sink: &mut Sink) -> Result<HostDocument, String> {
    match ws.cache {
        Some(cache) => {
            let key = host_key(entry, reading);
            reading_cache::read_host(cache, key, ws, sink, |files, sink| read_host_from(entry, reading, files, sink))
        }
        None => read_host_from(entry, reading, ws, sink),
    }
}

/// What the host document of `entry` is read with besides its files ([`reading_cache::HostKey`]).
fn host_key(entry: &str, (dialect, formats): Formats) -> reading_cache::HostKey {
    reading_cache::HostKey { entry: slashed(entry), dialect: dialect.map(str::to_string), formats: formats.clone() }
}

/// [`read_host_in`] from `files`, a workspace or what notes the questions a reading asks of one.
fn read_host_from(
    entry: &str,
    (dialect, set): Formats,
    files: &dyn Provider,
    sink: &mut Sink,
) -> Result<HostDocument, String> {
    let path = slashed(entry);
    let main = entry_format_in(entry, (dialect, set), files)?;
    let set = formats::set_formats(set, files)?;
    let doc = library::resolve(&path, files, sink, |extra, sink| {
        formats::with(main, &set, files, |f| Reading::read(&path, files, sink, f, extra, true))
    });
    doc.ok_or_else(|| format!("cannot read {entry}"))
}

/// The format `entry` is read in, by its name: the one set for it in `formats`, else the one
/// `dialect` names, else the one [`detect_format`] gives. A package file, of the format `rnl`, is
/// checked on its own ([`interpret`]); a file of another is read as a netlist.
pub fn entry_format(
    entry: &str,
    dialect: Option<&str>,
    formats: &BTreeMap<String, String>,
    files: &dyn Provider,
) -> Result<&'static str, String> {
    Ok(formats::name(entry_format_in(entry, (dialect, formats), &Workspace::new(files))?.0))
}

/// The format of `entry` and its default, by index ([`formats::main_format`]).
fn entry_format_in(entry: &str, (dialect, set): Formats, ws: &dyn Provider) -> Result<(usize, usize), String> {
    let path = slashed(entry);
    let text = ws.read(&path).ok_or_else(|| format!("cannot read {entry}"))?;
    let set = formats::set_formats(set, ws)?;
    Ok(formats::main_format(&path, &text, dialect, &set, ws))
}

/// The package file `entry` checked on its own (2.7), when it is of the format `rnl`
/// ([`entry_format`]), with that format and its default noted for it.
fn package_in(
    entry: &str,
    reading: Formats,
    ws: &Workspace,
    options: &Options,
) -> Result<Option<Interpretation>, String> {
    let (main, default) = entry_format_in(entry, reading, ws)?;
    if formats::name(main) != package::NAME {
        return Ok(None);
    }
    let mut r = rnl_runtime::check_package(&slashed(entry), ws, options);
    let file = r.doc.main().file.clone();
    let (format, default) = (formats::name(main).to_string(), formats::name(default).to_string());
    r.doc.file_formats.push(FileFormat { file, format, default });
    Ok(Some(r))
}

/// Interpret an entry: a netlist, or a package file checked on its own, one of the format `rnl`
/// ([`entry_format`]). The bundled packages are layered under `files`.
pub fn interpret(
    entry: &str,
    dialect: Option<&str>,
    files: &dyn Provider,
    options: &Options,
) -> Result<Interpretation, String> {
    interpret_in(entry, (dialect, &NONE_SET), &Workspace::new(files), options)
}

/// [`interpret`], with the formats set for files by path ([`BuildOptions::formats`]).
pub fn interpret_with(
    entry: &str,
    dialect: Option<&str>,
    formats: &BTreeMap<String, String>,
    files: &dyn Provider,
    options: &Options,
) -> Result<Interpretation, String> {
    interpret_in(entry, (dialect, formats), &Workspace::with_formats(files, formats), options)
}

/// Both readings of an entry that a solver makes, with the bundled packages under `files`
/// ([`rnl_runtime::previous::read_previous`]): the interpretation without the records of the
/// solver, with the problems, and the previous result of each level, from the reading with them,
/// in the order of the problems. `options` are those of both readings; `generated` and `problems`
/// are set here. A package file ([`entry_format`]) has no problems.
pub fn read_previous(
    entry: &str,
    dialect: Option<&str>,
    files: &dyn Provider,
    options: &Options,
) -> Result<(Interpretation, Vec<Option<Previous>>), String> {
    read_previous_with(entry, dialect, &NONE_SET, files, options)
}

/// [`interpret_with`], the objects of the authored records taken from `counts` and given to it as
/// [`rnl_runtime::interpret_taking`] takes and gives them (5.9).
pub fn interpret_with_taking(
    entry: &str,
    dialect: Option<&str>,
    formats: &BTreeMap<String, String>,
    files: &dyn Provider,
    options: &Options,
    counts: &mut rnl_runtime::AuthoredCounts,
) -> Result<Interpretation, String> {
    let ws = Workspace::with_formats(files, formats);
    if let Some(r) = package_in(entry, (dialect, formats), &ws, options)? {
        return Ok(r);
    }
    let mut sink = Sink::new();
    let doc = read_host_in(entry, (dialect, formats), &ws, &mut sink)?;
    Ok(rnl_runtime::interpret_taking(doc, &ws, options, sink, counts))
}

/// [`read_previous`], with the formats set for files by path ([`BuildOptions::formats`]).
pub fn read_previous_with(
    entry: &str,
    dialect: Option<&str>,
    formats: &BTreeMap<String, String>,
    files: &dyn Provider,
    options: &Options,
) -> Result<(Interpretation, Vec<Option<Previous>>), String> {
    let ws = Workspace::with_formats(files, formats);
    if let Some(r) = package_in(entry, (dialect, formats), &ws, options)? {
        return Ok((r, Vec::new()));
    }
    let mut sink = Sink::new();
    let doc = read_host_in(entry, (dialect, formats), &ws, &mut sink)?;
    Ok(rnl_runtime::previous::read_previous(doc, &ws, options, sink))
}

/// [`interpret`] in a workspace, which has the bundled packages already.
fn interpret_in(entry: &str, reading: Formats, ws: &Workspace, options: &Options) -> Result<Interpretation, String> {
    if let Some(r) = package_in(entry, reading, ws, options)? {
        return Ok(r);
    }
    let mut sink = Sink::new();
    let doc = read_host_in(entry, reading, ws, &mut sink)?;
    // through the reading cache, the objects of the authored records taken from the reading of the
    // document it kept, when they were counted of the same
    Ok(match ws.cache {
        Some(cache) => {
            reading_cache::with_counts(cache, |counts| rnl_runtime::interpret_taking(doc, ws, options, sink, counts))
        }
        None => rnl_runtime::interpret(doc, ws, options, sink),
    })
}

/// The record of the solver of a level, finished, with the bundled packages under `files` as
/// [`interpret`] reads them: [`rnl_runtime::solution::finish_record`]. `record` is what
/// [`rnl_runtime::solution::write_record`] gives for `problem`; `doc` is the host document the
/// problem was read from ([`Interpretation::doc`]), and `files` and `options` are what it was read
/// with. In language versions that support numeric references, a record using them gets
/// `numbering`, the check of its numbers (5.9), from the document read again with it; otherwise
/// it stays as it is.
pub fn finish_record(
    doc: &HostDocument,
    files: &dyn Provider,
    options: &Options,
    problem: &LevelProblem,
    record: &str,
) -> String {
    finish_record_with_formats(doc, files, &NONE_SET, options, problem, record)
}

/// Finish a solver record with the same explicit format overrides as [`interpret_with`].
/// Package discovery and exclusions then match the interpretation that produced the problem.
pub fn finish_record_with_formats(
    doc: &HostDocument,
    files: &dyn Provider,
    formats: &BTreeMap<String, String>,
    options: &Options,
    problem: &LevelProblem,
    record: &str,
) -> String {
    RecordFinisher::new(doc, files, formats, options, problem).finish(record)
}

/// [`finish_record_with_formats`] for records of the solver of one level one after another, such
/// as the trials of the dots of a level (13.2): the objects of the level's authored records are
/// counted once ([`rnl_runtime::solution::LevelFinisher`]).
pub struct RecordFinisher<'a> {
    level: rnl_runtime::solution::LevelFinisher<'a>,
    files: Workspace<'a>,
    formats: BTreeMap<String, String>,
}

impl<'a> RecordFinisher<'a> {
    /// The finisher of the records of the level of `problem`, read from `doc` with `files`,
    /// `formats` and `options`, as [`finish_record_with_formats`] takes them.
    pub fn new(
        doc: &'a HostDocument,
        files: &'a dyn Provider,
        formats: &BTreeMap<String, String>,
        options: &Options,
        problem: &'a LevelProblem,
    ) -> RecordFinisher<'a> {
        RecordFinisher {
            level: rnl_runtime::solution::LevelFinisher::new(doc, options, problem),
            files: Workspace::with_formats(files, formats),
            formats: formats.clone(),
        }
    }

    /// The record finished, as [`finish_record_with_formats`] gives it.
    pub fn finish(&self, record: &str) -> String {
        self.level.finish(&self.files, record)
    }

    /// A record that uses a number ([`rnl_runtime::solution::record_uses_numbers`]) finished on
    /// the document as the solve writes it: `entry` read, with `dialect` and the formats of this
    /// finisher, from `files`, which have `record` written into the level's file in place of its
    /// records of the solver ([`rnl_runtime::solution::LevelFinisher::finish_read`], `counts` as
    /// it takes them). With it, the lines of the level's record of the solver in that file, when
    /// it has one there. [`Self::finish`] gives a record that uses no number as it is, reading
    /// nothing.
    pub fn finish_written(
        &self,
        entry: &str,
        dialect: Option<&str>,
        files: &dyn Provider,
        record: &str,
        counts: &mut rnl_runtime::AuthoredCounts,
    ) -> Result<(String, Option<(u32, u32)>), String> {
        let ws = Workspace::with_formats(files, &self.formats);
        let doc = read_host_in(entry, (dialect, &self.formats), &ws, &mut Sink::new())?;
        let lines = rnl_runtime::solution::solver_record_lines(&doc, self.level.level());
        Ok((self.level.finish_read(doc, &ws, record, counts), lines))
    }
}

/// What a drawing level still lacks without a solver: objects without a position, and the
/// constraints by the status their check against the written coordinates gave. None when
/// everything is placed and every constraint holds.
pub fn level_status(lv: &LevelView) -> Option<String> {
    let mut parts = Vec::new();
    if !lv.unplaced.is_empty() {
        parts.push(format!("{} without a position", lv.unplaced.len()));
    }
    let mut by_status: Vec<(&str, usize)> = Vec::new();
    for c in &lv.constraints {
        match by_status.iter_mut().find(|(s, _)| *s == c.status) {
            Some((_, n)) => *n += 1,
            None => by_status.push((&c.status, 1)),
        }
    }
    if by_status.iter().any(|(s, _)| *s != "ok") {
        let list: Vec<String> = by_status.iter().map(|(s, n)| format!("{n} {s}")).collect();
        parts.push(format!("constraints: {}", list.join(", ")));
    }
    (!parts.is_empty()).then(|| parts.join("; "))
}

/// The status of every level that lacks something, one line each, as `rnl check` prints them.
pub fn status_lines(r: &Interpretation) -> Vec<String> {
    r.model.levels.iter().filter_map(|lv| level_status(lv).map(|s| format!("status {}: {s}", lv.title))).collect()
}

/// The frontend's files with the bundled packages under them. It notes the files asked for that
/// the frontend does not have.
struct Workspace<'a> {
    files: &'a dyn Provider,
    bundled: MemoryProvider,
    missing: RefCell<Vec<String>>,
    /// The files set to the format `rnl` ([`BuildOptions::formats`]): packages that USE-PACKAGE finds
    /// by name (16.2), before those of the package directories, whatever their suffix.
    packages: Vec<String>,
    /// The keys of the paths set to another format: no package, whatever their suffix.
    others: Vec<String>,
    /// The reading cache of the frontend, when the call has one: its host document is read
    /// through it ([`read_host_in`]).
    cache: Option<&'a RefCell<ReadingCache>>,
}

impl<'a> Workspace<'a> {
    /// The workspace, reading its host document through `cache` when there is one.
    fn cached(self, cache: Option<&'a RefCell<ReadingCache>>) -> Workspace<'a> {
        Workspace { cache, ..self }
    }
}

impl Workspace<'_> {
    fn new(files: &dyn Provider) -> Workspace<'_> {
        Workspace::with_formats(files, &NONE_SET)
    }

    /// The workspace of `files`, where the files set to the format `rnl` in `formats` are packages,
    /// and those set to another are not.
    fn with_formats<'a>(files: &'a dyn Provider, formats: &BTreeMap<String, String>) -> Workspace<'a> {
        let (mut packages, mut others) = (Vec::new(), Vec::new());
        for (path, name) in formats {
            let path = normalize(&slashed(path));
            if name == package::NAME {
                packages.push(path);
            } else {
                others.push(files.path_key(&path));
            }
        }
        Workspace { files, bundled: bundled(), missing: RefCell::new(Vec::new()), packages, others, cache: None }
    }

    /// Whether a package file of the frontend's package directories or pool stays one: not when
    /// its format is set, to another or to `rnl`, which [`Workspace::packages`] lists already.
    fn kept(&self, path: &str) -> bool {
        let key = self.files.path_key(&normalize(path));
        !self.others.contains(&key) && !self.packages.iter().any(|p| self.files.path_key(p) == key)
    }
}

impl Provider for Workspace<'_> {
    fn auto_include(&self) -> Option<&dyn rnl_runtime::source::LibraryLookup> {
        self.files.auto_include()
    }
    fn path_key(&self, path: &str) -> String {
        self.files.path_key(path)
    }
    fn read(&self, path: &str) -> Option<String> {
        if path.starts_with(BUNDLED_DIR) {
            return self.bundled.read(path);
        }
        let text = self.files.read(path);
        if text.is_none() {
            let mut missing = self.missing.borrow_mut();
            if !missing.iter().any(|m| m == path) {
                missing.push(path.to_string());
            }
        }
        text
    }

    /// The files set to the format `rnl`, the frontend's package directories, then the bundled
    /// packages (16.2).
    fn package_files(&self) -> Vec<String> {
        let mut out = self.packages.clone();
        out.extend(self.files.package_files().into_iter().filter(|p| self.kept(p)));
        out.extend(self.bundled.package_files());
        out
    }
    fn package_candidates(&self, name: &str) -> Vec<String> {
        let mut out = self.packages.clone();
        prefer_filename(&mut out, name);
        out.extend(self.files.package_candidates(name).into_iter().filter(|p| self.kept(p)));
        out.extend(self.bundled.package_candidates(name));
        out
    }
    fn include_candidates(&self, name: &str) -> Vec<String> {
        self.files.include_candidates(name)
    }
}

/// A path with `/` for every `\`, as the runtime reads paths.
fn slashed(path: &str) -> String {
    path.replace('\\', "/")
}

/// The extension of a path's file name, as `std::path::Path::extension` gives it.
fn extension(path: &str) -> Option<&str> {
    let name = file_name(path);
    match name.rfind('.') {
        Some(k) if k > 0 => Some(&name[k + 1..]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnl_runtime::diag::Severity;

    /// The frontend's file access in the tests: the files of the repository, read from the disk.
    pub(crate) struct Disk;

    impl Provider for Disk {
        fn read(&self, path: &str) -> Option<String> {
            std::fs::read_to_string(path).ok()
        }
    }

    /// A file of the conformance corpus, such as `cases/01-rc-lowpass.sp`.
    pub(crate) fn corpus(name: &str) -> String {
        rnl_runtime::source::normalize(&format!("{}/../../tests/conformance/{name}", env!("CARGO_MANIFEST_DIR")))
    }

    const NETLIST: &str = "* untitled
R1 a 0 1k
C1 a 0 1p
* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-spice\", version=\"0.4\")}
* @RNL {
*   #R1 @PLACE(0, 0);
*   #C1 @PLACE(4, 0)
* }
.end
";

    /// The head of each record of the corpus and of the bundled packages, read alone, is the head
    /// the whole record gives ([`rnl_runtime::syntax::parse_record_head`]).
    #[test]
    fn the_head_of_each_record_read_alone_is_the_head_of_the_whole_record() {
        use rnl_runtime::syntax::{parse_record, parse_record_head};
        let mut texts: Vec<String> = Vec::new();
        for dir in ["cases-0.4/from-0.2", "cases-0.4/from-0.3", "equivalence-0.4"] {
            let dir = corpus(dir);
            let mut entries: Vec<String> = std::fs::read_dir(&dir)
                .unwrap()
                .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string()))
                .filter(|f| f.ends_with(".sp") || f.ends_with(".scs"))
                .map(|f| format!("{dir}/{f}"))
                .collect();
            entries.sort();
            for entry in entries {
                let Ok(doc) = read_host(&entry, None, &Disk, &mut Sink::new()) else { continue };
                texts.extend(doc.levels.iter().flat_map(|l| l.records.iter().map(|r| r.text.clone())));
            }
        }
        for (path, text) in bundled().files {
            let records = rnl_runtime::records::scan_package_run(
                &rnl_runtime::records::package_lines(&text),
                &path,
                &mut Sink::new(),
            );
            texts.extend(records.into_iter().map(|r| r.text));
        }
        assert!(texts.len() > 200, "{}", texts.len());
        for text in &texts {
            assert_eq!(parse_record_head(text), parse_record(text).header, "{text}");
        }
    }

    #[test]
    fn an_untitled_text_is_an_entry_in_memory() {
        let files = MemoryProvider::new().with("untitled-1", NETLIST);
        let d = build("untitled-1", &files, &BuildOptions::default()).unwrap();
        assert_eq!((d.path.as_str(), d.name.as_str(), d.dialect.as_str()), ("untitled-1", "untitled-1", "spice"));
        assert_eq!(d.files.len(), 1);
        assert_eq!((d.files[0].path.as_str(), d.files[0].text.as_str()), ("untitled-1", NETLIST));
        // the standard package draws the resistor and the capacitor
        assert!(d.diagnostics.iter().all(|x| x.severity != "error"), "{:?}", d.diagnostics);
        assert!(d.levels[0].svg.as_ref().is_some_and(|s| s.starts_with("<svg")) && !d.levels[0].empty);
        assert_eq!(d.levels[0].hits.iter().filter(|h| h.kind == "box").count(), 2);
        assert!(d.missing.is_empty(), "{:?}", d.missing);
    }

    #[test]
    fn a_pin_name_is_shown_past_the_pins_lead() {
        // a block as an editor draws it: pins one unit outside its body, a lead from each to it
        let netlist = "* block
X1 a b blk
* @RNL(type=\"header\") { @VERSION(\"0.4\")}
* @RNL {
*   $blk = @DEFINE-SYMBOL(box=[-3, -2, 3, 2]) {
*     (-3, 0) @PIN(terminal=1, label=\"IN\", direction=\"left\");
*     (3, 0) @PIN(terminal=2, label=\"OUT\", direction=\"right\");
*     (0, 0) @RECTANGLE(width=4, height=4);
*     @LINE {(-3, 0) (-2, 0)}
*   };
*   #X1 @SET-SYMBOL $blk => @PLACE(10, 0)
* }
.end
";
        let files = MemoryProvider::new().with("/w/b.sp", netlist);
        let options = BuildOptions { svg: false, primitives: true, ..BuildOptions::default() };
        let d = serde_json::to_value(build("/w/b.sp", &files, &options).unwrap()).unwrap();
        let anchor = |name: &str| {
            let prims = d["levels"][0]["primitives"].as_array().unwrap();
            let text = prims.iter().find(|p| p["kind"] == "text" && p["lines"][0][0]["text"] == name).unwrap();
            (text["anchor"][0].as_f64().unwrap(), text["anchor"][1].as_f64().unwrap())
        };
        // IN: past its lead, 0.45 inside the body
        let (x, y) = anchor("IN");
        assert!((x - 8.45).abs() < 1e-9 && y.abs() < 1e-9, "{x} {y}");
        // OUT has no lead: 0.45 inside the pin
        let (x, y) = anchor("OUT");
        assert!((x - 12.55).abs() < 1e-9 && y.abs() < 1e-9, "{x} {y}");
    }

    #[test]
    fn a_missing_include_is_listed_once_and_diagnosed() {
        let text = "* top
.include \"lib/parts.inc\"
.include \"lib/parts.inc\"
X1 a b part
.end
";
        let files = MemoryProvider::new().with("/w/top.sp", text);
        let d = build("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        assert_eq!(d.missing, ["/w/lib/parts.inc"]);
        let errors: Vec<_> = d.diagnostics.iter().filter(|x| x.severity == "error").collect();
        let found = errors.iter().any(|x| x.message.contains("lib/parts.inc cannot be found") && x.line == 2);
        assert!(found, "{errors:?}");
        // the same diagnostics as before the core noted what was missing
        let again = interpret("/w/top.sp", None, &files, &Options::default()).unwrap();
        assert_eq!(d.diagnostics.len(), again.diagnostics().len());

        // once the frontend has the file, nothing is missing and its subcircuit is drawn
        let files = files.with("/w/lib/parts.inc", ".subckt part p q\nR1 p q 1k\n.ends\n");
        let d = build("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        assert!(d.missing.is_empty(), "{:?}", d.missing);
        assert_eq!(d.files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["top.sp", "lib/parts.inc"]);
        assert!(d.levels.iter().any(|l| l.title == "part"));
        assert!(!d.diagnostics.iter().any(|x| x.message.contains("cannot be found")), "{:?}", d.diagnostics);
    }

    #[test]
    fn a_package_file_by_path_is_missing_until_given() {
        let text = "* top
R1 a 0 1k
* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"acme\", path=\"pkg/acme.rnl\")}
.end
";
        let files = MemoryProvider::new().with("top.sp", text);
        let d = build("top.sp", &files, &BuildOptions::default()).unwrap();
        assert_eq!(d.missing, ["pkg/acme.rnl"]);
        // the bundled packages are read as well, but are never missing
        assert!(d.missing.iter().all(|m| !m.starts_with(BUNDLED_DIR)));
    }

    #[test]
    fn a_package_by_name_comes_from_the_package_directories_and_is_never_missing() {
        let text = "* top
R1 a 0 1k
* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"acme\", version=\"1.0\")}
.end
";
        let files = MemoryProvider::new().with("/w/top.sp", text);
        let d = build("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        assert!(d.missing.is_empty(), "{:?}", d.missing);
        let error = |d: &Document| d.diagnostics.iter().any(|x| x.message.contains("acme 1.0 cannot be found"));
        assert!(error(&d), "{:?}", d.diagnostics);

        // a frontend gives its package directories before it builds
        // a package that defines something: one that only declares itself is an alias package (16.5)
        let package = "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"acme\", version=\"1.0\")}\n\
                       @RNL { $ACME_MARK = @DEFINE(roles=[\"p\"]) {} }\n";
        let mut files = files.with("/w/packages/acme.rnl", package);
        files.package_dirs.push("/w/packages".into());
        let d = build("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        assert!(d.diagnostics.iter().all(|x| x.severity != "error"), "{:?}", d.diagnostics);
        assert!(d.missing.is_empty(), "{:?}", d.missing);
    }

    #[test]
    fn rooted_paths_keep_the_files_outside_the_entry_directory_apart() {
        // `x.inc` beside the entry and `../x.inc` above it
        let top = "* top
.include \"x.inc\"
.include \"../x.inc\"
X1 a b inner
X2 a b outer
.end
";
        let inner = "* inner
.subckt inner p q
R1 p q 1k
.ends
";
        let outer = "* outer
.subckt outer p q
R2 p q 1k
.ends
";
        let files = MemoryProvider::new().with("/w/top.sp", top).with("/w/x.inc", inner).with("/x.inc", outer);
        let d = build("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        let paths: Vec<&str> = d.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["top.sp", "x.inc", "/x.inc"]);
        let lines = |name: &str| d.objects.iter().find(|o| o.name == name).and_then(|o| o.lines);
        assert_eq!((lines("R1"), lines("R2")), (Some([1, 3, 3]), Some([2, 3, 3])));
        // the entry's directory joined with a file's path is the file in the workspace
        let dir = rnl_runtime::source::dirname(&d.path);
        let keys: Vec<String> = paths.iter().map(|p| rnl_runtime::source::join(&dir, p)).collect();
        assert_eq!(keys, ["/w/top.sp", "/w/x.inc", "/x.inc"]);
        assert!(keys.iter().all(|k| files.read(k).is_some()));
    }

    #[test]
    fn a_file_above_the_root_is_the_providers_to_give() {
        // `../x.inc` above an entry at the root is not `x.inc` beside it: the core asks for
        // `/../x.inc`, which a workspace in memory does not have unless it is given
        let top = "* top
.include \"x.inc\"
.include \"../x.inc\"
X1 a b inner
.end
";
        let inner = "* inner
.subckt inner p q
R1 p q 1k
.ends
";
        let files = MemoryProvider::new().with("/top.sp", top).with("/x.inc", inner);
        let d = build("/top.sp", &files, &BuildOptions::default()).unwrap();
        let paths: Vec<&str> = d.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["top.sp", "x.inc"]);
        assert_eq!(d.missing, ["/../x.inc"]);
        let error = d.diagnostics.iter().find(|x| x.message == "the included file ../x.inc cannot be found");
        assert_eq!(error.map(|x| (x.file_index, x.line)), Some((Some(0), 3)), "{:?}", d.diagnostics);

        // given, it is named by its path, as a file outside the entry's directory is
        let d = build("/top.sp", &files.with("/../x.inc", "* above\n"), &BuildOptions::default()).unwrap();
        let paths: Vec<&str> = d.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["top.sp", "x.inc", "/../x.inc"]);
        assert!(d.missing.is_empty(), "{:?}", d.missing);
    }

    #[test]
    fn the_dialect_can_be_given() {
        // Spectre's instance syntax; without the override, the `.sp` name says SPICE
        let text = "// t\nr1 (a 0) resistor r=1k\n";
        let files = MemoryProvider::new().with("chain.sp", text);
        let guessed = build("chain.sp", &files, &BuildOptions::default()).unwrap();
        assert_eq!(guessed.dialect, "spice");
        let options = BuildOptions { dialect: Some("spectre".into()), ..BuildOptions::default() };
        let given = build("chain.sp", &files, &options).unwrap();
        assert_eq!(given.dialect, "spectre");
        assert!(given.objects.iter().any(|o| o.kind == "instance" && o.name == "r1"), "{:?}", given.objects);
        // and the other way round
        let files = MemoryProvider::new().with("chain.scs", "* t\nR1 a 0 1k\n.end\n");
        let options = BuildOptions { dialect: Some("spice".into()), ..BuildOptions::default() };
        assert_eq!(build("chain.scs", &files, &options).unwrap().dialect, "spice");
        assert_eq!(build("chain.scs", &files, &BuildOptions::default()).unwrap().dialect, "spectre");
    }

    #[test]
    fn a_missing_entry_is_an_error() {
        let files = MemoryProvider::new().with("a.sp", NETLIST);
        for entry in ["b.sp", "b.scs", "pkg/b.rnl"] {
            assert_eq!(build(entry, &files, &BuildOptions::default()).unwrap_err(), format!("cannot read {entry}"));
        }
    }

    #[test]
    fn the_standard_package_can_be_left_out() {
        // a file that uses no package, read with the setting that adds the standard package (16.3)
        let text = NETLIST.replace("; @USE-PACKAGE(name=\"rnl-std-spice\", version=\"0.4\")", "");
        assert_ne!(text, NETLIST);
        let files = MemoryProvider::new().with("a.sp", &text);
        let runtime = |std: bool| Options { std, include_std: true, ..Options::default() };
        let options = |std: bool| BuildOptions { runtime: runtime(std), ..BuildOptions::default() };
        let without = build("a.sp", &files, &options(false)).unwrap();
        let with = build("a.sp", &files, &options(true)).unwrap();
        assert_ne!(without.diagnostics.len(), with.diagnostics.len());
    }

    #[test]
    fn a_package_file_is_checked_on_its_own() {
        let d = build(&format!("{BUNDLED_DIR}/0.4/rnl-std-all.rnl"), &MemoryProvider::new(), &BuildOptions::default());
        let d = d.unwrap();
        assert!(d.diagnostics.is_empty(), "{:?}", d.diagnostics);
        assert!(d.missing.is_empty());
        assert_eq!(d.files.len(), 1);
        assert!(d.files[0].text.contains("@PACKAGE"));
        assert!(!d.files[0].records.is_empty());
    }

    #[test]
    fn the_bundled_packages_are_by_version() {
        // the series of libraries/standard with its Chinese aliases in a directory of its own, by
        // version: the series 0.4
        let names: Vec<&str> = BUNDLED.iter().map(|(path, _)| *path).collect();
        assert_eq!(
            names,
            [
                "0.4/alias-zh.rnl",
                "0.4/rnl-std-45deg.rnl",
                "0.4/rnl-std-all.rnl",
                "0.4/rnl-std-basic.rnl",
                "0.4/rnl-std-fill.rnl",
                "0.4/rnl-std-functional.rnl",
                "0.4/rnl-std-logic.rnl",
                "0.4/rnl-std-no-instance-labels.rnl",
                "0.4/rnl-std-no-labels.rnl",
                "0.4/rnl-std-rf.rnl",
                "0.4/rnl-std-semantics.rnl",
                "0.4/rnl-std-semiconductors.rnl",
                "0.4/rnl-std-signal-flow.rnl",
                "0.4/rnl-std-spectre.rnl",
                "0.4/rnl-std-spice.rnl",
                "0.4/rnl-std-style.rnl",
            ]
        );
        let p = bundled();
        assert_eq!(p.package_dirs, [format!("{BUNDLED_DIR}/0.4")]);
        assert_eq!(p.package_files().len(), names.len());
    }

    #[test]
    fn the_bundled_drafts_are_the_series_newer_than_the_default() {
        // a series is written in the language version of its directory's name (16.1, 16.3)
        let key = rnl_runtime::interp::catalog::version_key;
        let default = key(rnl_runtime::interp::unit::DEFAULT_VERSION);
        let newer: BTreeSet<String> = BUNDLED
            .iter()
            .filter(|(path, _)| key(path.split('/').next().unwrap()) > default)
            .map(|(path, _)| format!("{BUNDLED_DIR}/{path}"))
            .collect();
        assert_eq!(bundled_drafts(), newer);
        // the files of a draft are still bundled, for the core to read
        assert!(newer.iter().all(|path| bundled().files.contains_key(path)));
    }

    #[test]
    fn a_bundled_package_file_is_read_like_any_host_file() {
        let mut sink = Sink::new();
        let path = format!("{BUNDLED_DIR}/0.4/rnl-std-all.rnl");
        let doc = read_host(&path, None, &MemoryProvider::new(), &mut sink).unwrap();
        assert_eq!(doc.main().file, "rnl-std-all.rnl");
        let text = BUNDLED.iter().find(|(p, _)| *p == "0.4/rnl-std-all.rnl").map(|(_, text)| *text);
        assert!(text.is_some());
        assert_eq!(doc.source("rnl-std-all.rnl"), text);
    }

    #[test]
    fn backslashes_separate_directories_too() {
        let d = build(r"w\top.sp", &MemoryProvider::new().with("w/top.sp", NETLIST), &BuildOptions::default());
        let d = d.unwrap();
        assert_eq!((d.path.as_str(), d.name.as_str()), (r"w\top.sp", "top.sp"));
        assert_eq!(d.files[0].path, "top.sp");
    }

    #[test]
    fn extensions_are_those_of_the_file_name() {
        assert_eq!(extension("a/b.SCS"), Some("SCS"));
        assert_eq!(extension("a.d/b"), None);
        assert_eq!(extension("a/.rnl"), None);
        assert_eq!(extension(r"x\y.rnl"), Some("rnl"));
    }

    #[test]
    fn spectre_by_option_extension_or_simulator_statement() {
        assert!(is_spectre("a.SCS", "", None));
        assert!(!is_spectre("a.scs", "", Some("spice")));
        assert!(is_spectre("a.sp", "", Some("spectre")));
        assert!(is_spectre("a.net", "// t\nsimulator lang=spectre\n", None));
        assert!(!is_spectre("dir.scs/a.sp", "* t\n", None));
    }

    #[test]
    fn the_listed_suffixes_are_the_netlists_first_then_what_they_include() {
        assert_eq!(
            listed_suffixes(),
            ["sp", "spi", "spice", "cir", "ckt", "net", "netlist", "scs", "inc", "lib", "rnl", "va", "vams"]
        );
        let spice = formats().into_iter().find(|f| f.name == "spice").unwrap();
        assert_eq!(spice.entries.len(), 7);
        assert!(spice.listed.starts_with(&spice.entries));
        let text = formats().into_iter().find(|f| f.name == "text").unwrap();
        assert_eq!(text.listed, ["va", "vams"]);
        assert!(text.entries.is_empty());
        for f in formats() {
            assert!(f.entries.iter().all(|e| f.listed.contains(e)), "{}", f.name);
        }
        assert!(is_listed("a/b.SP") && is_listed(r"c:\x\m.VAMS") && is_listed("lib/0.4/p.rnl"));
        assert!(!is_listed("a/b.v") && !is_listed("a/.sp") && !is_listed("a.sp/readme") && !is_listed("noext"));
    }

    #[test]
    fn the_listed_bundled_packages_are_all_but_a_draft_series() {
        let listed = bundled_listed();
        let drafts = bundled_drafts();
        assert!(!listed.is_empty());
        assert_eq!(listed.len() + drafts.len(), bundled().files.len());
        assert!(listed.keys().all(|p| p.starts_with(&format!("{BUNDLED_DIR}/")) && !drafts.contains(p)));
    }

    #[test]
    fn an_untitled_text_is_interpreted_with_the_bundled_packages() {
        let files = MemoryProvider::new().with("untitled-1", NETLIST);
        let r = interpret("untitled-1", None, &files, &Options::default()).unwrap();
        assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].items.iter().filter(|i| !i.host.is_empty()).count(), 2);
        let drafts = bundled_drafts();
        for (path, _) in BUNDLED {
            // Every bundled standard package reads without diagnostics, but those of a draft series,
            // which the rnl command's conformance tests check against what is not read yet.
            if drafts.contains(&format!("{BUNDLED_DIR}/{path}")) {
                continue;
            }
            let options = Options::default();
            let package = interpret(&format!("{BUNDLED_DIR}/{path}"), None, &MemoryProvider::new(), &options);
            let package = package.unwrap();
            let loud: Vec<_> = package.diagnostics().into_iter().filter(|d| d.severity != Severity::Hint).collect();
            assert!(loud.is_empty(), "{path}: {loud:?}");
            assert!(package.diagnostics().is_empty(), "{path}: {:?}", package.diagnostics());
        }
    }

    #[test]
    fn the_document_keeps_the_field_names_the_viewers_read() {
        let d = build("a.sp", &MemoryProvider::new().with("a.sp", NETLIST), &BuildOptions::default()).unwrap();
        let v = serde_json::to_value(&d).unwrap();
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        let want = [
            "symbols",
            "path",
            "name",
            "dialect",
            "formats",
            "levels",
            "diagnostics",
            "failed",
            "files",
            "objects",
            "spans",
            "missing",
        ];
        assert_eq!(keys, want);
        assert_eq!(v["missing"], serde_json::json!([]));
    }

    #[test]
    fn the_frontend_chooses_the_outputs() {
        let files = MemoryProvider::new().with("a.sp", NETLIST);
        let level = |svg: bool, primitives: bool| {
            let options = BuildOptions { svg, primitives, ..BuildOptions::default() };
            let d = build("a.sp", &files, &options).unwrap();
            serde_json::to_value(&d).unwrap()["levels"][0].as_object().unwrap().clone()
        };
        let keys = |l: &serde_json::Map<String, serde_json::Value>| {
            l.keys()
                .filter(|k| !["ports", "symbol", "currents"].contains(&k.as_str()))
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
        };
        // by default the SVG and no primitives, as the viewers have read the Document so far
        let default = BuildOptions::default();
        let l = level(default.svg, default.primitives);
        assert_eq!(keys(&l), "usages host title kind lines svg scale empty status hits net_labels children unplaced");
        let l = level(true, true);
        assert_eq!(
            keys(&l),
            "usages host title kind lines svg primitives scale empty status hits net_labels children unplaced"
        );
        assert_eq!(l["scale"], 14.0);
        assert!(l["primitives"].as_array().is_some_and(|p| p.len() > 2));
        assert_eq!(
            keys(&level(false, true)),
            "usages host title kind lines primitives scale empty status hits net_labels children unplaced"
        );
        assert_eq!(
            keys(&level(false, false)),
            "usages host title kind lines scale empty status hits net_labels children unplaced"
        );
        // the solve status when asked for: the resistor and the capacitor are placed, not turned
        let options = BuildOptions { svg: false, status: true, ..BuildOptions::default() };
        let d = serde_json::to_value(build("a.sp", &files, &options).unwrap()).unwrap();
        let l = d["levels"][0].as_object().unwrap();
        assert_eq!(keys(l), "usages host title kind lines scale empty status solve hits net_labels children unplaced");
        assert_eq!(l["solve"], "unsolved");
    }

    #[test]
    fn a_corpus_case_builds_from_the_disk() {
        let d = build(&corpus("cases-0.4/from-0.2/04-hierarchy.sp"), &Disk, &BuildOptions::default()).unwrap();
        let names: Vec<&str> = d.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["04-hierarchy.sp", "04-hierarchy-lib.inc"]);
        assert!(d.missing.is_empty(), "{:?}", d.missing);
        // a subcircuit's definition, from its header to its .ends, in the file that has it
        let lines = |title: &str| d.levels.iter().find(|l| l.title == title).and_then(|l| l.lines);
        assert_eq!((d.levels[0].kind, d.levels[0].lines), ("file", None));
        assert_eq!(lines("amp"), Some([0, 8, 52]));
        assert_eq!(lines("rdiv"), Some([1, 3, 32]));
    }

    #[test]
    fn a_subcircuit_without_its_end_runs_to_the_end_of_its_file() {
        let spice = MemoryProvider::new().with("a.sp", "* title\n.subckt x a b\nR1 a b 1k\n");
        let d = build("a.sp", &spice, &BuildOptions::default()).unwrap();
        assert_eq!(d.levels.iter().find(|l| l.title == "x").and_then(|l| l.lines), Some([0, 2, 3]));
        // and one with its end, to its `ends`, after the title line that Spectre reads as a comment
        let spectre = MemoryProvider::new().with("a.scs", "// title\nsubckt x (a b)\nR1 (a b) resistor r=1k\nends x\n");
        let d = build("a.scs", &spectre, &BuildOptions::default()).unwrap();
        assert_eq!(d.levels.iter().find(|l| l.title == "x").and_then(|l| l.lines), Some([0, 2, 4]));
    }
}
