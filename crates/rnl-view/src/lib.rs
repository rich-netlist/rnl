//! The core interface of decision 0008: a workspace in, a Document out. A frontend names the file
//! to open and gives its own file access; the core reads what the document needs through it, with
//! the standard packages bundled here, and returns what a viewer shows.
//!
//! The crate does no file I/O, so it builds for `wasm32-unknown-unknown`.

mod catalog;
mod document;
mod library;
mod primitive;
mod snapshot;
#[cfg(test)]
mod types;

use std::cell::RefCell;
use std::collections::HashMap;

use rnl_runtime::diag::Sink;
use rnl_runtime::host::{file_name, HostDocument};
use rnl_runtime::model::LevelView;
use rnl_runtime::previous::{solve_status, Previous, SolveStatus};
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::source::{dirname, Provider};
use rnl_runtime::writeback::record_type;
use rnl_runtime::{Interpretation, Options};

pub use catalog::{
    Catalog, CatalogDefinition, CatalogLabel, CatalogOptions, CatalogPackage, CatalogPin, CatalogSet, CatalogSymbol,
    Terminal,
};
pub use document::{
    Child, Diagnostic, Document, Drawings, Hit, InspectedLabel, InspectedPin, InspectedSymbol, Level, Object,
    SourceFile, Span, Unplaced, UnplacedDrawing,
};
pub use library::{LibraryIndex, LibraryProvider};
pub use primitive::{
    Bar, Cap, Color, HAlign, Join, Layer, Primitive, Run, RunStyle, Segment, Shape, Stroke, Text, TextRun, Unit, VAlign,
};
/// Texts held in memory, for a frontend that builds one, such as a netlist from the clipboard.
pub use rnl_runtime::source::MemoryProvider;
pub use snapshot::{
    FileRef, HostInstance, HostLevel, HostNet, HostTerminal, LevelModel, LevelSnapshot, ObjectView, PackageUse, Parts,
    PinLabel, PinView, PointView, ShapeView, Slot, Snapshot, SymbolDefinition, SymbolPin, SymbolPrim, SymbolRef,
    TextView, Vertex, WireView, SCHEMA,
};

include!(concat!(env!("OUT_DIR"), "/bundled.rs"));

/// The directory of the bundled packages, as the runtime names it. Each version of the standard
/// series has a directory of its own in it, as in `libraries/standard`, such as `<bundled>/0.2`.
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

/// How to build a Document, and what to give of each level's drawing.
#[derive(Clone, Debug)]
pub struct BuildOptions {
    /// "spice" or "spectre", instead of guessing from the entry.
    pub dialect: Option<String>,
    /// The runtime's options, such as using the standard package at the root (16.3), or reading
    /// the levels without their generated records, as a solver does (2.8).
    pub runtime: Options,
    /// Write each level's SVG ([`Level::svg`]); on by default.
    pub svg: bool,
    /// Give each level's primitives ([`Level::primitives`]); off by default.
    pub primitives: bool,
    /// Give each level its solve status ([`Level::solve`]), what a frontend decides from whether
    /// to solve it (decision 0008); off by default. It takes the problems of the levels read
    /// without their records of the solver, so a document that has such records is read once more.
    pub status: bool,
}

impl Default for BuildOptions {
    fn default() -> BuildOptions {
        BuildOptions { dialect: None, runtime: Options::default(), svg: true, primitives: false, status: false }
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
    Ok(build_with(entry, files, options, false)?.0)
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
    let ws = Workspace::new(files);
    if ws.read(&slashed(entry)).is_none() {
        return Err(format!("cannot read {entry}"));
    }
    let dialect = options.dialect.as_deref();
    // the solve status takes the problems; built in this reading too, they change nothing else
    let runtime = Options { problems: options.runtime.problems || options.status, ..options.runtime.clone() };
    let mut r = interpret_in(entry, dialect, &ws, &runtime)?;
    let solve = if options.status { solve_statuses(entry, dialect, &ws, &runtime, &r)? } else { HashMap::new() };
    let mut d = document::document(entry, &r, ws.missing.into_inner(), options, lazy);
    for level in &mut d.levels {
        level.solve = solve.get(&level.host).map(|s| s.as_str());
    }
    let drawings = lazy.then(|| Drawings::take(&mut r.model.levels, &d, options));
    Ok((d, drawings))
}

/// The solve status of each drawing level of `r`, by host level ([`solve_status`]): its problem
/// and model read without the records of the solver, against its record of the solver read with
/// them. `r` was read with `runtime`, problems included; so a document without records of the
/// solver, which reads the same with them and without, is not read again.
fn solve_statuses(
    entry: &str,
    dialect: Option<&str>,
    ws: &Workspace,
    runtime: &Options,
    r: &Interpretation,
) -> Result<HashMap<usize, SolveStatus>, String> {
    let records = r.doc.levels.iter().any(|l| l.records.iter().any(|rec| record_type(rec) == "generated"));
    let other;
    let (with, without) = if !records {
        (r, r)
    } else if runtime.generated {
        other = interpret_in(entry, dialect, ws, &Options { generated: false, ..runtime.clone() })?;
        (r, &other)
    } else {
        other = interpret_in(entry, dialect, ws, &Options { generated: true, problems: false, ..runtime.clone() })?;
        (&other, r)
    };
    let views: HashMap<usize, &LevelView> = without.model.levels.iter().map(|lv| (lv.level, lv)).collect();
    let mut out = HashMap::new();
    for p in &without.problems {
        if let Some(view) = views.get(&p.level) {
            out.insert(p.level, solve_status(p, view, with));
        }
    }
    Ok(out)
}

/// The snapshot of `entry` for the editor interface, with the parts `parts` asks for, from the
/// same interpretation as its Document; the Document in it is built as `options` ask. Fails as
/// [`build`] does.
pub fn snapshot(entry: &str, files: &dyn Provider, options: &BuildOptions, parts: Parts) -> Result<Snapshot, String> {
    let ws = Workspace::new(files);
    if ws.read(&slashed(entry)).is_none() {
        return Err(format!("cannot read {entry}"));
    }
    let r = interpret_in(entry, options.dialect.as_deref(), &ws, &options.runtime)?;
    // without the drawing, the Document is built only for its files, levels and diagnostics
    let draw =
        if parts.drawing { options.clone() } else { BuildOptions { svg: false, primitives: false, ..options.clone() } };
    let document = document::document(entry, &r, ws.missing.into_inner(), &draw, false);
    Ok(snapshot::snapshot(&r, document, parts, &options.runtime))
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

/// Read a host document with the adapter its dialect needs. The bundled packages are layered
/// under `files`, so an entry among them can be read as well.
pub fn read_host(
    entry: &str,
    dialect: Option<&str>,
    files: &dyn Provider,
    sink: &mut Sink,
) -> Result<HostDocument, String> {
    read_host_in(entry, dialect, &Workspace::new(files), sink)
}

/// [`read_host`] in a workspace, which has the bundled packages already.
fn read_host_in(entry: &str, dialect: Option<&str>, ws: &Workspace, sink: &mut Sink) -> Result<HostDocument, String> {
    let path = slashed(entry);
    let text = ws.read(&path).ok_or_else(|| format!("cannot read {entry}"))?;
    let doc = library::resolve(&path, ws, sink, |extra, sink| {
        if is_spectre(&path, &text, dialect) {
            rnl_host_spectre::read_with_includes(&path, ws, sink, extra)
        } else {
            rnl_host_spice::read_with_includes(&path, ws, sink, extra)
        }
    });
    doc.ok_or_else(|| format!("cannot read {entry}"))
}

/// Interpret an entry: a netlist, or a package file (`.rnl`) checked on its own. The bundled
/// packages are layered under `files`.
pub fn interpret(
    entry: &str,
    dialect: Option<&str>,
    files: &dyn Provider,
    options: &Options,
) -> Result<Interpretation, String> {
    interpret_in(entry, dialect, &Workspace::new(files), options)
}

/// Both readings of an entry that a solver makes, with the bundled packages under `files`
/// ([`rnl_runtime::previous::read_previous`]): the interpretation without the records of the
/// solver, with the problems, and the previous result of each level, from the reading with them,
/// in the order of the problems. `options` are those of both readings; `generated` and `problems`
/// are set here. A package file (`.rnl`) has no problems.
pub fn read_previous(
    entry: &str,
    dialect: Option<&str>,
    files: &dyn Provider,
    options: &Options,
) -> Result<(Interpretation, Vec<Option<Previous>>), String> {
    let ws = Workspace::new(files);
    if extension(entry).is_some_and(|e| e.eq_ignore_ascii_case("rnl")) {
        return Ok((rnl_runtime::check_package(&slashed(entry), &ws, options), Vec::new()));
    }
    let mut sink = Sink::new();
    let doc = read_host_in(entry, dialect, &ws, &mut sink)?;
    Ok(rnl_runtime::previous::read_previous(doc, &ws, options, sink))
}

/// [`interpret`] in a workspace, which has the bundled packages already.
fn interpret_in(
    entry: &str,
    dialect: Option<&str>,
    ws: &Workspace,
    options: &Options,
) -> Result<Interpretation, String> {
    if extension(entry).is_some_and(|e| e.eq_ignore_ascii_case("rnl")) {
        return Ok(rnl_runtime::check_package(&slashed(entry), ws, options));
    }
    let mut sink = Sink::new();
    let doc = read_host_in(entry, dialect, ws, &mut sink)?;
    Ok(rnl_runtime::interpret(doc, ws, options, sink))
}

/// The record of the solver of a level, finished, with the bundled packages under `files` as
/// [`interpret`] reads them: [`rnl_runtime::solution::finish_record`]. `record` is what
/// [`rnl_runtime::solution::write_record`] gives for `problem`; `doc` is the host document the
/// problem was read from ([`Interpretation::doc`]), and `files` and `options` are what it was read
/// with. In a level of a file of the draft 0.3, the record gets `numbering`, the check of its
/// numbers (5.9), from the document read again with it; otherwise it stays as it is.
pub fn finish_record(
    doc: &HostDocument,
    files: &dyn Provider,
    options: &Options,
    problem: &LevelProblem,
    record: &str,
) -> String {
    rnl_runtime::solution::finish_record(doc, &Workspace::new(files), options, problem, record)
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
}

impl Workspace<'_> {
    fn new(files: &dyn Provider) -> Workspace<'_> {
        Workspace { files, bundled: bundled(), missing: RefCell::new(Vec::new()) }
    }
}

impl Provider for Workspace<'_> {
    fn auto_include(&self) -> Option<&dyn rnl_runtime::source::LibraryLookup> {
        self.files.auto_include()
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

    /// The frontend's package directories, then the bundled packages (16.2).
    fn package_files(&self) -> Vec<String> {
        let mut out = self.files.package_files();
        out.extend(self.bundled.package_files());
        out
    }
    fn package_candidates(&self, name: &str) -> Vec<String> {
        let mut out = self.files.package_candidates(name);
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
* @RNL(type=\"header\") { @VERSION(\"0.2\")}
* @RNL {
*   #R1 @PLACE(0, 0);
*   #C1 @PLACE(4, 0)
* }
.end
";

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
* @RNL(type=\"header\") { @VERSION(\"0.2\")}
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
* @RNL(type=\"header\") { @VERSION(\"0.2\"); @USE-PACKAGE(name=\"acme\", path=\"pkg/acme.rnl\")}
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
* @RNL(type=\"header\") { @VERSION(\"0.2\"); @USE-PACKAGE(name=\"acme\", version=\"1.0\")}
.end
";
        let files = MemoryProvider::new().with("/w/top.sp", text);
        let d = build("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        assert!(d.missing.is_empty(), "{:?}", d.missing);
        let error = |d: &Document| d.diagnostics.iter().any(|x| x.message.contains("acme 1.0 cannot be found"));
        assert!(error(&d), "{:?}", d.diagnostics);

        // a frontend gives its package directories before it builds
        let package = "@RNL(type=\"header\") { @VERSION(\"0.2\"); @PACKAGE(name=\"acme\", version=\"1.0\")}";
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
        let files = MemoryProvider::new().with("a.sp", NETLIST);
        let options = BuildOptions { runtime: Options { std: false, ..Options::default() }, ..BuildOptions::default() };
        let without = build("a.sp", &files, &options).unwrap();
        let with = build("a.sp", &files, &BuildOptions::default()).unwrap();
        assert_ne!(without.diagnostics.len(), with.diagnostics.len());
    }

    #[test]
    fn a_package_file_is_checked_on_its_own() {
        let d = build(&format!("{BUNDLED_DIR}/0.2/rnl-std.rnl"), &MemoryProvider::new(), &BuildOptions::default());
        let d = d.unwrap();
        assert!(d.diagnostics.is_empty(), "{:?}", d.diagnostics);
        assert!(d.missing.is_empty());
        assert_eq!(d.files.len(), 1);
        assert!(d.files[0].text.contains("@PACKAGE"));
        assert!(!d.files[0].records.is_empty());
    }

    #[test]
    fn the_bundled_packages_are_by_version() {
        // the series 0.2 and the series 0.3, each with its Chinese aliases in its own directory, and
        // Analog Canvas's package in one of its own
        let names: Vec<&str> = BUNDLED.iter().map(|(path, _)| *path).collect();
        let series = |v: &str| names.iter().filter(|n| n.starts_with(&format!("{v}/"))).count();
        assert_eq!(
            names[..9],
            [
                "0.2/rnl-std-core.rnl",
                "0.2/rnl-std-fill.rnl",
                "0.2/rnl-std-layout.rnl",
                "0.2/rnl-std-semantics.rnl",
                "0.2/rnl-std-style.rnl",
                "0.2/rnl-std-symbols.rnl",
                "0.2/rnl-std-virtual.rnl",
                "0.2/rnl-std.rnl",
                "0.2/rnl-zh.rnl",
            ]
        );
        // the series 0.3: the packages of 16.3 of the language 0.3, and rnl-zh 0.3
        assert_eq!(series("0.3"), 16);
        for name in
            ["rnl-std", "rnl-std-core", "rnl-std-basic", "rnl-std-spice", "rnl-std-spectre", "rnl-std-full", "rnl-zh"]
        {
            assert!(names.contains(&format!("0.3/{name}.rnl").as_str()), "{name}");
        }
        assert!(names.iter().all(|name| name.starts_with("0.2/") || name.starts_with("0.3/")));
        assert_eq!(names.len(), 9 + 16);
        let p = bundled();
        let dirs = ["0.2", "0.3"].map(|d| format!("{BUNDLED_DIR}/{d}"));
        assert_eq!(p.package_dirs, dirs);
        assert_eq!(p.package_files().len(), names.len());
    }

    #[test]
    fn a_bundled_package_file_is_read_like_any_host_file() {
        let mut sink = Sink::new();
        let path = format!("{BUNDLED_DIR}/0.2/rnl-std.rnl");
        let doc = read_host(&path, None, &MemoryProvider::new(), &mut sink).unwrap();
        assert_eq!(doc.main().file, "rnl-std.rnl");
        let text = BUNDLED.iter().find(|(p, _)| *p == "0.2/rnl-std.rnl").map(|(_, text)| *text);
        assert_eq!(doc.source("rnl-std.rnl"), text);
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
    fn an_untitled_text_is_interpreted_with_the_bundled_packages() {
        let files = MemoryProvider::new().with("untitled-1", NETLIST);
        let r = interpret("untitled-1", None, &files, &Options::default()).unwrap();
        assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].items.iter().filter(|i| !i.host.is_empty()).count(), 2);
        for (path, _) in BUNDLED {
            // Every bundled standard package reads without diagnostics.
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
            l.keys().filter(|k| !["ports", "symbol"].contains(&k.as_str())).cloned().collect::<Vec<_>>().join(" ")
        };
        // by default the SVG and no primitives, as the viewers have read the Document so far
        let default = BuildOptions::default();
        let l = level(default.svg, default.primitives);
        assert_eq!(keys(&l), "host title kind lines svg scale empty status hits children unplaced");
        let l = level(true, true);
        assert_eq!(keys(&l), "host title kind lines svg primitives scale empty status hits children unplaced");
        assert_eq!(l["scale"], 14.0);
        assert!(l["primitives"].as_array().is_some_and(|p| p.len() > 2));
        assert_eq!(
            keys(&level(false, true)),
            "host title kind lines primitives scale empty status hits children unplaced"
        );
        assert_eq!(keys(&level(false, false)), "host title kind lines scale empty status hits children unplaced");
        // the solve status when asked for: the resistor and the capacitor are placed, not turned
        let options = BuildOptions { svg: false, status: true, ..BuildOptions::default() };
        let d = serde_json::to_value(build("a.sp", &files, &options).unwrap()).unwrap();
        let l = d["levels"][0].as_object().unwrap();
        assert_eq!(keys(l), "host title kind lines scale empty status solve hits children unplaced");
        assert_eq!(l["solve"], "unsolved");
    }

    #[test]
    fn a_corpus_case_builds_from_the_disk() {
        let d = build(&corpus("cases/04-hierarchy.sp"), &Disk, &BuildOptions::default()).unwrap();
        let names: Vec<&str> = d.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["04-hierarchy.sp", "04-hierarchy-lib.inc"]);
        assert!(d.missing.is_empty(), "{:?}", d.missing);
        // a subcircuit's definition, from its header to its .ends, in the file that has it
        let lines = |title: &str| d.levels.iter().find(|l| l.title == title).and_then(|l| l.lines);
        assert_eq!((d.levels[0].kind, d.levels[0].lines), ("file", None));
        assert_eq!(lines("amp"), Some([0, 8, 48]));
        assert_eq!(lines("rdiv"), Some([1, 2, 19]));
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
