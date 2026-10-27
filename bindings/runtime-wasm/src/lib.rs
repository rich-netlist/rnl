//! The core for WebAssembly (decision 0008): [`rnl_view::build`], [`rnl_edit::write`] with a build
//! of what it wrote, [`rnl_view::catalog`], and the drawings of the objects a build lists without a
//! position, on request, behind a small C-style interface over linear memory,
//! a JSON request in and a JSON response out. It needs no generated glue: the module imports
//! nothing, and a page or a Worker calls its exports as `js/core.js` does.
//!
//! A call goes like this:
//!
//! 1. `rnl_request(len)` makes room for a request of `len` bytes and returns where to write it.
//! 2. The caller writes the request there: a [`Request`] as UTF-8 JSON.
//! 3. `rnl_build()` answers it and returns the length of the answer, a [`Response`] as UTF-8
//!    JSON; `rnl_response()` returns where it is. It stays there until the next call.
//!
//! A panic traps: the call throws in JavaScript, and the instance must not be used for another
//! request. The panic hook keeps the message first, so the caller can read it with
//! `rnl_panic_len()` and `rnl_panic_message()` before it replaces the instance. Running out of
//! memory traps without a panic; `rnl_alloc_failed()` then says how much could not be allocated.
//!
//! Addresses and lengths are `usize`, an `i32` to WebAssembly. The buffers are statics owned by
//! the module, so the exports need no unsafe code: the caller writes only into the room
//! `rnl_request` made, and reads only what the other exports point to. The only unsafe code passes
//! allocations on to the system allocator, to note one that fails.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, Once, PoisonError};

use rnl_runtime::source::{normalize, LibraryDefinition, LibraryLookup, MemoryProvider, Provider};
use rnl_runtime::Options;
use rnl_view::{BuildOptions, Catalog, CatalogOptions, Document, Drawings, Parts, Snapshot, UnplacedDrawing};
use serde::{Deserialize, Serialize};

/// What to build: the workspace and how. Fields the core does not know are ignored, so that a
/// frontend can ask for options an older core does not have yet.
#[derive(Deserialize, Debug)]
pub struct Request {
    /// Optional inspection SVG export; geometry is always available in the Document.
    #[serde(default)]
    pub overlays: Option<rnl_view::OverlayOptions>,
    /// The path of the file to open, as the frontend names it (see [`rnl_view::build`]).
    pub entry: String,
    /// Explicit temporary files, searched before package directories.
    #[serde(default)]
    pub pool: Vec<String>,
    #[serde(default)]
    pub auto_include: bool,
    /// Namespace used by the browser to keep independently opened files ahead of pool files.
    #[serde(default)]
    pub library_prefix: String,
    #[serde(default)]
    pub library_edits: BTreeMap<String, String>,
    /// The texts of the workspace by path, the entry's among them.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// The directories, among the paths of `files`, whose packages are found by name (16.2).
    #[serde(default)]
    pub package_dirs: Vec<String>,
    /// "spice" or "spectre", instead of guessing from the entry.
    pub dialect: Option<String>,
    /// The format set for files, by path as in `files` ([`rnl_view::BuildOptions::formats`]); the
    /// library index reads its files with them too.
    #[serde(default)]
    pub formats: BTreeMap<String, String>,
    /// Use the standard package at the root (16.3); true when not given.
    #[serde(default = "yes")]
    pub std: bool,
    /// The version of the standard package at the root, whatever the files' language version, such
    /// as "0.2" ([`rnl_runtime::Options::std_version`]); by the language version when not given.
    #[serde(default)]
    pub std_version: Option<String>,
    /// Write each level's SVG; true when not given.
    #[serde(default = "yes")]
    pub svg: bool,
    /// Give each level's primitives; false when not given.
    #[serde(default)]
    pub primitives: bool,
    /// Give each level its solve status, what a frontend decides from whether to solve it
    /// (decision 0008); false when not given.
    #[serde(default)]
    pub status: bool,
    /// Leave the drawings of the objects without a position out of the Document and keep them to
    /// give on request ([`DrawingsRequest`]), until the next build that asks for it; false, the
    /// Document carrying them, when not given.
    #[serde(default)]
    pub lazy_unplaced: bool,
    /// The parts of the editor interface's snapshot to give: with them, the answer is a
    /// [`rnl_view::Snapshot`] instead of a Document.
    pub parts: Option<PartsRequest>,
    /// Panic with this message, to test that the caller still gets an answer.
    #[cfg(feature = "test-panic")]
    pub panic: Option<String>,
    /// Allocate until there is no memory left, to test that the caller learns why the core stopped.
    #[cfg(feature = "test-panic")]
    #[serde(default)]
    pub out_of_memory: bool,
}

fn yes() -> bool {
    true
}

/// The parts of a snapshot ([`rnl_view::Parts`]): the Document unless `drawing` is false, and
/// each level's host circuit and model when asked for.
#[derive(Deserialize, Debug)]
pub struct PartsRequest {
    #[serde(default = "yes")]
    pub drawing: bool,
    #[serde(default)]
    pub host: bool,
    #[serde(default)]
    pub model: bool,
}

/// What to write: an edit transaction's ops into its netlist ([`rnl_edit::write`]), and the
/// Document of what was written. The fields beside `write` are those of a [`Request`], but
/// `entry` and `dialect`: the netlist is the entry, in the dialect of the write.
#[derive(Deserialize, Debug)]
pub struct WriteRequest {
    #[serde(default)]
    pub overlays: Option<rnl_view::OverlayOptions>,
    pub write: rnl_edit::WriteRequest,
    /// Further files, such as the packages of `package_dirs`.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    pub package_dirs: Vec<String>,
    #[serde(default = "yes")]
    pub std: bool,
    #[serde(default)]
    pub std_version: Option<String>,
    #[serde(default = "yes")]
    pub svg: bool,
    #[serde(default)]
    pub primitives: bool,
    #[serde(default)]
    pub status: bool,
}

/// What a write answers: the netlist with the ops written into it, and its Document, whose
/// diagnostics say how what was written reads.
#[derive(Serialize, Debug)]
pub struct Written {
    pub text: String,
    pub document: Document,
}

/// What to list: the catalog of the packages the core has ([`rnl_view::catalog`]), those of
/// `package_dirs` among `files` and the bundled ones under them, as a build finds packages by name.
#[derive(Deserialize, Debug)]
pub struct CatalogRequest {
    pub catalog: CatalogQuery,
    /// The frontend's files, among them those of its package directories, as in a [`Request`].
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    pub package_dirs: Vec<String>,
    /// Read each package with the standard package of its language version (16.3); true when not
    /// given.
    #[serde(default = "yes")]
    pub std: bool,
    /// The version of the standard package each package is read with, as in a [`Request`].
    #[serde(default)]
    pub std_version: Option<String>,
}

/// Which packages a catalog lists ([`rnl_view::CatalogOptions`]).
#[derive(Deserialize, Debug)]
pub struct CatalogQuery {
    /// Only the packages of these names; every package when not given.
    pub packages: Option<Vec<String>>,
    /// Every version of each package, not only the newest; false when not given.
    #[serde(default)]
    pub all_versions: bool,
}

/// What to draw: the objects at `items` of `levels[level].unplaced` of the Document that the last
/// build with `lazy_unplaced` gave, whose drawings the core kept.
#[derive(Deserialize, Debug)]
pub struct DrawingsRequest {
    pub drawings: DrawingsQuery,
}

#[derive(Deserialize, Debug)]
pub struct DrawingsQuery {
    pub level: usize,
    pub items: Vec<usize>,
    /// The names of the objects as the caller has them, which must be those of the build, so that
    /// a request about a Document that was built again is refused, not answered with other objects.
    #[serde(default)]
    pub names: Vec<String>,
}

/// What to list: the formats a file can be read with, and the format each of `detect`, by path
/// and text, has when none is set for it ([`rnl_view::detect_format`]).
#[derive(Deserialize, Debug)]
pub struct FormatsRequest {
    pub formats: FormatsQuery,
}

#[derive(Deserialize, Debug, Default)]
pub struct FormatsQuery {
    #[serde(default)]
    pub detect: BTreeMap<String, String>,
}

/// What a [`FormatsRequest`] answers.
#[derive(Serialize, Debug)]
pub struct FormatList {
    pub list: Vec<rnl_view::FormatInfo>,
    /// The format of each file of `detect`, by its path.
    pub detected: BTreeMap<String, String>,
}

/// The answer to a request: `{"document": ...}`, `{"snapshot": ...}`, `{"written": ...}`,
/// `{"catalog": ...}`, `{"drawings": ...}`, `{"formats": ...}` or `{"error": "..."}`.
#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Response {
    Library(Vec<String>),
    /// The Document of the entry.
    Document(Box<Document>),
    /// The snapshot of the entry, when the request asks for `parts`.
    Snapshot(Box<Snapshot>),
    /// The netlist a [`WriteRequest`] wrote, and its Document.
    Written(Box<Written>),
    /// The catalog a [`CatalogRequest`] asks for.
    Catalog(Box<Catalog>),
    /// The drawings a [`DrawingsRequest`] asks for, one for each object it names, in order.
    Drawings(Vec<UnplacedDrawing>),
    /// The formats a [`FormatsRequest`] asks for.
    Formats(Box<FormatList>),
    /// Why there is none: the request is not a [`Request`], a [`WriteRequest`], a
    /// [`CatalogRequest`] or a [`DrawingsRequest`], the entry cannot be read, an op cannot be
    /// written, or no build kept the drawings asked for.
    Error(String),
}

thread_local! {
    static LIBRARY: RefCell<rnl_view::LibraryIndex> = RefCell::new(rnl_view::LibraryIndex::default());
    /// The drawings of the last build with `lazy_unplaced`. They hold what the model does, which is
    /// not `Send`, so they stay in the thread that built them; a module is one thread.
    static DRAWN: RefCell<Option<Drawings>> = const { RefCell::new(None) };
}

/// Answer a request given as JSON: a write when it has a `write` field, a catalog when it has a
/// `catalog` field, drawings when it has a `drawings` field, the formats when it has a `formats`
/// field and no `entry`, else a build.
pub fn respond(request: &[u8]) -> Response {
    let invalid = |e: serde_json::Error| Response::Error(format!("the request is not valid: {e}"));
    let value: serde_json::Value = match serde_json::from_slice(request) {
        Ok(v) => v,
        Err(e) => return invalid(e),
    };
    if value.get("library").is_some() {
        #[derive(Deserialize)]
        struct Update {
            library: Query,
            #[serde(default)]
            pool: Vec<String>,
            #[serde(default)]
            files: BTreeMap<String, String>,
            /// The formats set for files by path, as a build's ([`Request::formats`]).
            #[serde(default)]
            formats: BTreeMap<String, String>,
        }
        #[derive(Deserialize)]
        struct Query {
            #[serde(default)]
            reload: bool,
            #[serde(default)]
            refresh: Vec<String>,
        }
        return match serde_json::from_value::<Update>(value) {
            Ok(update) => {
                let files = MemoryProvider {
                    files: update.files.into_iter().map(|(p, t)| (normalize(&p), t)).collect(),
                    package_dirs: Vec::new(),
                };
                let pool: Vec<_> = update.pool.iter().map(|p| normalize(p)).collect();
                let files = rnl_runtime::source::FilePool { files: &files, paths: &pool };
                Response::Library(LIBRARY.with(|index| {
                    let mut index = index.borrow_mut();
                    index.set_formats(update.formats);
                    index.invalidate(&update.library.refresh);
                    index.update(&pool, &files, update.library.reload)
                }))
            }
            Err(e) => invalid(e),
        };
    }
    if value.get("write").is_some() {
        return match serde_json::from_value(value) {
            Ok(w) => write(w),
            Err(e) => invalid(e),
        };
    }
    if value.get("catalog").is_some() {
        return match serde_json::from_value(value) {
            Ok(c) => catalog(c),
            Err(e) => invalid(e),
        };
    }
    if value.get("drawings").is_some() {
        return match serde_json::from_value::<DrawingsRequest>(value) {
            Ok(d) => drawings(d.drawings),
            Err(e) => invalid(e),
        };
    }
    // a build sets formats too: a request for the list has no entry
    if value.get("formats").is_some() && value.get("entry").is_none() {
        return match serde_json::from_value::<FormatsRequest>(value) {
            Ok(f) => {
                let detected = f
                    .formats
                    .detect
                    .iter()
                    .map(|(p, t)| (p.clone(), rnl_view::detect_format(p, t).to_string()))
                    .collect();
                Response::Formats(Box::new(FormatList { list: rnl_view::formats(), detected }))
            }
            Err(e) => invalid(e),
        };
    }
    let request: Request = match serde_json::from_value(value) {
        Ok(r) => r,
        Err(e) => return invalid(e),
    };
    #[cfg(feature = "test-panic")]
    if let Some(message) = &request.panic {
        panic!("{message}");
    }
    #[cfg(feature = "test-panic")]
    if request.out_of_memory {
        // room that is never written, so that it costs address space rather than pages
        let mut kept: Vec<Vec<u8>> = Vec::new();
        loop {
            kept.push(Vec::with_capacity(1 << 30));
        }
    }
    let files = request.files.into_iter().map(|(path, text)| (normalize(&path), text)).collect();
    let files = MemoryProvider { files, package_dirs: request.package_dirs };
    let paths: Vec<String> = request.pool.iter().map(|p| normalize(p)).collect();
    let files = rnl_runtime::source::FilePool { files: &files, paths: &paths };
    let canonical: Vec<_> =
        paths.iter().map(|p| p.strip_prefix(&request.library_prefix).unwrap_or(p).to_string()).collect();
    let namespace = PoolNamespace { files: &files, prefix: &request.library_prefix };
    let pool = rnl_runtime::source::FilePool { files: &namespace, paths: &canonical };
    let edits = MemoryProvider {
        files: request.library_edits.into_iter().map(|(p, t)| (normalize(&p), t)).collect(),
        package_dirs: Vec::new(),
    };
    // the index reads the canonical paths, as the formats set for them name them
    let prefix = request.library_prefix.as_str();
    let set = request.formats.iter().map(|(p, f)| (p.strip_prefix(prefix).unwrap_or(p).to_string(), f.clone()));
    let index_formats: BTreeMap<String, String> = set.collect();
    let index = LIBRARY.with(|index| {
        let mut index = index.borrow_mut();
        index.set_formats(index_formats);
        index.update(&canonical, &pool, false);
        index.with_edits(&edits, &pool)
    });
    let lookup = NamespacedIndex { index: &index, prefix: &request.library_prefix };
    let files = AutoLibrary { files: &files, index: &lookup, enabled: request.auto_include };
    let options = BuildOptions {
        dialect: request.dialect,
        formats: request.formats,
        runtime: Options { std: request.std, std_version: request.std_version.clone(), ..Options::default() },
        svg: request.svg,
        primitives: request.primitives,
        status: request.status,
    };
    if let Some(p) = request.parts {
        let parts = Parts { drawing: p.drawing, host: p.host, model: p.model };
        return match rnl_view::snapshot(&request.entry, &files, &options, parts) {
            Ok(mut s) => {
                if let (Some(d), Some(o)) = (&mut s.drawing, &request.overlays) {
                    if let Err(e) = rnl_view::overlays::apply(d, o) {
                        return Response::Error(e);
                    }
                }
                Response::Snapshot(Box::new(s))
            }
            Err(e) => Response::Error(e),
        };
    }
    if request.lazy_unplaced {
        return match rnl_view::build_drawn(&request.entry, &files, &options) {
            Ok((mut d, drawn)) => {
                if let Some(o) = &request.overlays {
                    if let Err(e) = rnl_view::overlays::apply(&mut d, o) {
                        return Response::Error(e);
                    }
                }
                DRAWN.with(|slot| *slot.borrow_mut() = Some(drawn));
                Response::Document(Box::new(d))
            }
            Err(e) => Response::Error(e),
        };
    }
    match rnl_view::build(&request.entry, &files, &options) {
        Ok(mut d) => {
            if let Some(o) = &request.overlays {
                if let Err(e) = rnl_view::overlays::apply(&mut d, o) {
                    return Response::Error(e);
                }
            }
            Response::Document(Box::new(d))
        }
        Err(e) => Response::Error(e),
    }
}

/// Index in canonical pool paths; resolve the same entries in the browser's source namespace.
struct PoolNamespace<'a> {
    files: &'a dyn Provider,
    prefix: &'a str,
}
impl Provider for PoolNamespace<'_> {
    fn read(&self, path: &str) -> Option<String> {
        self.files.read(&format!("{}{path}", self.prefix))
    }
    fn path_key(&self, path: &str) -> String {
        self.files.path_key(path)
    }
}
struct NamespacedIndex<'a> {
    index: &'a rnl_view::LibraryIndex,
    prefix: &'a str,
}
impl NamespacedIndex<'_> {
    fn map(&self, mut d: LibraryDefinition) -> LibraryDefinition {
        d.path = format!("{}{}", self.prefix, d.path);
        d.file = format!("{}{}", self.prefix, d.file);
        d
    }
}
impl LibraryLookup for NamespacedIndex<'_> {
    fn candidates(&self, name: &str, sensitive: bool) -> Vec<LibraryDefinition> {
        self.index.candidates(name, sensitive).into_iter().map(|d| self.map(d)).collect()
    }
    fn definitions(&self, path: &str) -> Vec<LibraryDefinition> {
        self.index
            .definitions(path.strip_prefix(self.prefix).unwrap_or(path))
            .into_iter()
            .map(|d| self.map(d))
            .collect()
    }
}
struct AutoLibrary<'a> {
    files: &'a dyn Provider,
    index: &'a dyn LibraryLookup,
    enabled: bool,
}
impl Provider for AutoLibrary<'_> {
    fn read(&self, path: &str) -> Option<String> {
        self.files.read(path)
    }
    fn package_files(&self) -> Vec<String> {
        self.files.package_files()
    }
    fn package_candidates(&self, name: &str) -> Vec<String> {
        self.files.package_candidates(name)
    }
    fn include_candidates(&self, name: &str) -> Vec<String> {
        self.files.include_candidates(name)
    }
    fn auto_include(&self) -> Option<&dyn LibraryLookup> {
        self.enabled.then_some(self.index)
    }
    fn path_key(&self, path: &str) -> String {
        self.files.path_key(path)
    }
}

/// The drawings of the objects a request names, from the last build that kept them.
fn drawings(query: DrawingsQuery) -> Response {
    DRAWN.with(|slot| match slot.borrow().as_ref() {
        None => Response::Error("no build has kept drawings: build with lazy_unplaced first".to_string()),
        Some(d) => match d.unplaced(query.level, &query.items, &query.names) {
            Ok(drawn) => Response::Drawings(drawn),
            Err(e) => Response::Error(e),
        },
    })
}

/// Write, then build what was written as the entry, among the request's files.
fn write(request: WriteRequest) -> Response {
    let text = match rnl_edit::write(&request.write) {
        Ok(t) => t,
        Err(e) => return Response::Error(e),
    };
    // the netlist built under the name it was written under, so that it reads the same
    let entry = rnl_edit::entry_of(&request.write);
    let mut files: BTreeMap<String, String> =
        request.files.into_iter().map(|(path, text)| (normalize(&path), text)).collect();
    files.insert(normalize(&entry), text.clone());
    let files = MemoryProvider { files, package_dirs: request.package_dirs };
    let options = BuildOptions {
        dialect: Some(request.write.dialect),
        formats: BTreeMap::new(),
        runtime: Options { std: request.std, std_version: request.std_version.clone(), ..Options::default() },
        svg: request.svg,
        primitives: request.primitives,
        status: request.status,
    };
    match rnl_view::build(&entry, &files, &options) {
        Ok(mut document) => {
            if let Some(o) = &request.overlays {
                if let Err(e) = rnl_view::overlays::apply(&mut document, o) {
                    return Response::Error(e);
                }
            }
            Response::Written(Box::new(Written { text, document }))
        }
        Err(e) => Response::Error(e),
    }
}

/// List the packages a catalog request asks for.
fn catalog(request: CatalogRequest) -> Response {
    let files = request.files.into_iter().map(|(path, text)| (normalize(&path), text)).collect();
    let files = MemoryProvider { files, package_dirs: request.package_dirs };
    let options = CatalogOptions {
        packages: request.catalog.packages,
        all_versions: request.catalog.all_versions,
        runtime: Options { std: request.std, std_version: request.std_version, ..Options::default() },
    };
    Response::Catalog(Box::new(rnl_view::catalog(&files, &options)))
}

/// [`respond`], with the answer as JSON.
pub fn respond_json(request: &[u8]) -> String {
    serde_json::to_string(&respond(request)).expect("a response has only string keys")
}

/// The request, written by the caller where [`rnl_request`] said.
static REQUEST: Mutex<Vec<u8>> = Mutex::new(Vec::new());
/// The answer to the last request.
static RESPONSE: Mutex<String> = Mutex::new(String::new());
/// The message of the last panic, kept by the panic hook for the caller to read after the trap.
static PANIC: Mutex<String> = Mutex::new(String::new());
/// The size of an allocation that failed in the last call; 0 when none did.
static FAILED_ALLOCATION: AtomicUsize = AtomicUsize::new(0);

/// The system allocator, noting the size of an allocation that fails. Running out of memory
/// aborts without calling the panic hook, so this is how the caller learns of it.
struct NotingAllocator;

#[global_allocator]
static ALLOCATOR: NotingAllocator = NotingAllocator;

// SAFETY: every call goes to `System` as it came, with the caller's guarantees; a failure is noted
// in an atomic, which does not allocate.
unsafe impl GlobalAlloc for NotingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        noted(System.alloc(layout), layout.size())
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        noted(System.alloc_zeroed(layout), layout.size())
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        noted(System.realloc(ptr, layout, new_size), new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

/// An allocation's result, after noting its size when it failed.
fn noted(ptr: *mut u8, size: usize) -> *mut u8 {
    if ptr.is_null() {
        FAILED_ALLOCATION.store(size, Ordering::Relaxed);
    }
    ptr
}

/// Make room for a request of `len` bytes and return where to write it.
#[no_mangle]
pub extern "C" fn rnl_request(len: usize) -> *mut u8 {
    begin();
    let mut request = lock(&REQUEST);
    request.clear();
    request.resize(len, 0);
    request.as_mut_ptr()
}

/// Answer the request written where [`rnl_request`] said, and return the length of the answer.
#[no_mangle]
pub extern "C" fn rnl_build() -> usize {
    begin();
    let request = std::mem::take(&mut *lock(&REQUEST));
    let answer = respond_json(&request);
    let mut response = lock(&RESPONSE);
    *response = answer;
    response.len()
}

/// Where the answer to the last request is.
#[no_mangle]
pub extern "C" fn rnl_response() -> *const u8 {
    lock(&RESPONSE).as_ptr()
}

/// The length of the message of the panic that stopped the last call; 0 when none did.
#[no_mangle]
pub extern "C" fn rnl_panic_len() -> usize {
    lock(&PANIC).len()
}

/// Where the message of the panic that stopped the last call is.
#[no_mangle]
pub extern "C" fn rnl_panic_message() -> *const u8 {
    lock(&PANIC).as_ptr()
}

/// The size in bytes of the allocation that failed in the last call; 0 when none did.
#[no_mangle]
pub extern "C" fn rnl_alloc_failed() -> usize {
    FAILED_ALLOCATION.load(Ordering::Relaxed)
}

/// Start a call: keep the messages of panics, and forget what stopped the call before.
fn begin() {
    keep_panic_messages();
    lock(&PANIC).clear();
    FAILED_ALLOCATION.store(0, Ordering::Relaxed);
}

/// A buffer behind its lock. A panic that held the lock leaves nothing half done that matters:
/// on WebAssembly it ends the instance anyway.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Keep the message of a panic for the caller, before the hook that was there. On
/// `wasm32-unknown-unknown` a panic aborts, which traps, and the default hook has nowhere to print.
fn keep_panic_messages() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            {
                let mut message = lock(&PANIC);
                message.clear();
                let _ = write!(message, "{info}");
            }
            previous(info);
        }));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

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

    fn answer(request: Value) -> Value {
        serde_json::from_str(&respond_json(request.to_string().as_bytes())).unwrap()
    }

    // Compare both sides after the same JSON transport. Parsing a JSON float without
    // float_roundtrip may round its final bit differently from an in-memory to_value.
    fn wire(value: impl Serialize) -> Value {
        serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap()
    }

    #[test]
    fn file_pool_is_separate_from_open_files_and_resolves_packages_and_includes() {
        let top = "* top\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"acme\", version=\"1.0\") }\n.include \"part.inc\"\n.end\n";
        let files = json!({
            "/work/top.sp": top,
            "/pool/custom.rnl": "@RNL(type=\"header\") { @VERSION(\"0.3\"); @PACKAGE(name=\"acme\", version=\"1.0\") }",
            "/pool/part.inc": "R1 a 0 1k\n"
        });
        let without = answer(json!({ "entry": "/work/top.sp", "files": files }));
        assert!(without["document"]["diagnostics"].as_array().unwrap().iter().any(|d| d["severity"] == "error"));
        let with =
            answer(json!({ "entry": "/work/top.sp", "files": files, "pool": ["/pool/custom.rnl", "/pool/part.inc"] }));
        assert!(
            !with["document"]["diagnostics"].as_array().unwrap().iter().any(|d| d["severity"] == "error"),
            "{with}"
        );
        assert_eq!(with["document"]["files"].as_array().unwrap().len(), 2);
        assert!(with["document"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["message"].as_str().unwrap().contains("/pool/part.inc")));
    }

    #[test]
    fn a_request_can_ask_for_a_version_of_the_standard_package() {
        // a file of 0.3 that writes $GND of the series 0.2: an error, unless the request asks for it
        let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a 0 1k\n* @RNL { $GND @ATTACH &a }\n.end\n";
        let errors = |request: Value| {
            let v = answer(request);
            let d = v["document"]["diagnostics"].as_array().unwrap().clone();
            d.into_iter().filter(|d| d["severity"] == "error").count()
        };
        assert_eq!(errors(json!({ "entry": "t.sp", "files": { "t.sp": text } })), 1);
        assert_eq!(errors(json!({ "entry": "t.sp", "files": { "t.sp": text }, "std_version": "0.2" })), 0);
    }

    #[test]
    fn an_untitled_text_is_an_entry_in_memory() {
        let v = answer(json!({ "entry": "untitled-1", "files": { "untitled-1": NETLIST } }));
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["document"]);
        let d = &v["document"];
        assert_eq!((&d["path"], &d["dialect"]), (&json!("untitled-1"), &json!("spice")));
        assert_eq!(d["files"][0]["text"], NETLIST);
        assert_eq!(d["missing"], json!([]));
        assert_eq!(d["levels"][0]["empty"], false);
    }

    #[test]
    fn the_document_is_the_one_rnl_view_builds() {
        let files = MemoryProvider::new().with("/w/top.sp", NETLIST);
        let native = rnl_view::build("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        let v = answer(json!({ "entry": "/w/top.sp", "files": { "/w/top.sp": NETLIST } }));
        assert_eq!(v["document"], wire(native));
    }

    #[test]
    fn a_missing_include_is_listed() {
        let top = "* top\n.include \"lib/parts.inc\"\nX1 a b part\n.end\n";
        let v = answer(json!({ "entry": "/w/top.sp", "files": { "/w/top.sp": top } }));
        assert_eq!(v["document"]["missing"], json!(["/w/lib/parts.inc"]));
        // the paths of the workspace are read as the runtime reads them
        let files = json!({ "/w/./top.sp": top, "\\w\\lib\\parts.inc": ".subckt part p q\nR1 p q 1k\n.ends\n" });
        let v = answer(json!({ "entry": "/w/top.sp", "files": files }));
        assert_eq!(v["document"]["missing"], json!([]));
        assert_eq!(v["document"]["files"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn library_updates_auto_include_namespaces_and_reload_replaces_definitions() {
        let cell = ".subckt cell p n\nR1 p n 1k\n.ends\n";
        let updated =
            answer(json!({ "library": {"reload": true}, "pool": ["/cell.inc"], "files": {"/cell.inc": cell} }));
        assert_eq!(updated["library"], json!([]));
        let build = |name: &str, text: &str, enabled: bool| {
            answer(json!({
                "entry": "/top.sp", "pool": ["/<pool>/cell.inc"], "library_prefix": "/<pool>", "auto_include": enabled,
                "files": {"/top.sp": format!("* top\nX1 a 0 {name}\n.end\n"), "/<pool>/cell.inc": text}
            }))
        };
        let off = build("cell", cell, false);
        assert_eq!(off["document"]["files"].as_array().unwrap().len(), 1);
        let on = build("cell", cell, true);
        assert_eq!(on["document"]["files"].as_array().unwrap().len(), 2);
        assert!(on["document"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["severity"] == "warning" && d["message"].as_str().unwrap().contains("added by Auto Include")));
        let replacement = cell.replace("cell", "replacement");
        answer(json!({ "library": {"reload": true}, "pool": ["/cell.inc"], "files": {"/cell.inc": replacement} }));
        let reloaded = build("replacement", &replacement, true);
        assert_eq!(reloaded["document"]["files"].as_array().unwrap().len(), 2);
        let removed = build("cell", &replacement, true);
        assert_eq!(removed["document"]["files"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn formats_are_set_by_path_listed_and_detected() {
        // a SPICE netlist whose include is written in Spectre's language
        let files = json!({
            "/w/top.sp": "* top\n.include \"cells.scs\"\nX1 a 0 amp\n.end\n",
            "/w/cells.scs": "simulator lang=spectre\nsubckt amp (p n)\nR1 (p n) resistor r=1k\nends amp\n",
        });
        let v = answer(json!({ "entry": "/w/top.sp", "files": files, "formats": { "/w/cells.scs": "spice" } }));
        let d = &v["document"];
        assert_eq!((&d["files"][1]["format"], &d["files"][1]["default_format"]), (&json!("spice"), &json!("spectre")));
        assert!(!d["levels"].as_array().unwrap().iter().any(|l| l["title"] == "amp"));
        assert_eq!(d["formats"], serde_json::to_value(rnl_view::formats()).unwrap());
        // without it, by its own suffix; an unknown name is the request's error
        let v = answer(json!({ "entry": "/w/top.sp", "files": files }));
        assert_eq!(v["document"]["files"][1]["format"], "spectre");
        assert!(v["document"]["levels"].as_array().unwrap().iter().any(|l| l["title"] == "amp"));
        let v = answer(json!({ "entry": "/w/top.sp", "files": files, "formats": { "/w/cells.scs": "cdl" } }));
        assert_eq!(v["error"], "unknown format \"cdl\" for /w/cells.scs; the formats are spice, spectre, text");
        // the list, and the format of files by path and text
        let v =
            answer(json!({ "formats": { "detect": { "/w/a.inc": "* a\n", "/w/b.lib": "simulator lang=spectre\n" } } }));
        assert_eq!(v["formats"]["list"], serde_json::to_value(rnl_view::formats()).unwrap());
        assert_eq!(v["formats"]["detected"], json!({ "/w/a.inc": "spice", "/w/b.lib": "spectre" }));
        assert_eq!(answer(json!({ "formats": {} }))["formats"]["detected"], json!({}));
        // the library index reads with them: a `.scs` file written in SPICE defines its subcircuit
        let lib = json!({ "/lib/cells.scs": ".subckt cell p n\nR1 p n 1k\n.ends\n" });
        let update = |formats: Value| {
            answer(
                json!({ "library": { "reload": true }, "pool": ["/lib/cells.scs"], "files": lib, "formats": formats }),
            )
        };
        let top = json!({ "/top.sp": "* top\nX1 a 0 cell\n.end\n", "/lib/cells.scs": lib["/lib/cells.scs"] });
        let build = |formats: Value| {
            let request = json!({ "entry": "/top.sp", "files": top, "pool": ["/lib/cells.scs"], "auto_include": true, "formats": formats });
            answer(request)["document"]["levels"].as_array().unwrap().iter().any(|l| l["title"] == "cell")
        };
        assert_eq!(update(json!({})), json!({ "library": [] }));
        assert!(!build(json!({})));
        update(json!({ "/lib/cells.scs": "spice" }));
        assert!(build(json!({ "/lib/cells.scs": "spice" })));
    }

    #[test]
    fn a_verilog_a_file_is_text_and_not_read() {
        // the build gives the core's verdict: the format "text", and one hint instead of SPICE's warnings
        let files = json!({ "/w/res.va": "module res(p, n);\n  electrical p, n;\n  analog V(p, n) <+ 1k * I(p, n);\nendmodule\n" });
        let v = answer(json!({ "entry": "/w/res.va", "files": files }));
        let d = &v["document"];
        assert_eq!(
            (&d["dialect"], &d["files"][0]["format"], &d["files"][0]["default_format"]),
            (&json!("text"), &json!("text"), &json!("text"))
        );
        let said: Vec<(&Value, &Value)> =
            d["diagnostics"].as_array().unwrap().iter().map(|x| (&x["severity"], &x["message"])).collect();
        assert_eq!(said, [(&json!("hint"), &json!("`res.va` is not a netlist (Verilog-A); it is not read"))]);
        let v = answer(json!({ "formats": { "detect": { "/w/res.va": "", "/w/top.sp": "" } } }));
        assert_eq!(v["formats"]["detected"], json!({ "/w/res.va": "text", "/w/top.sp": "spice" }));
    }

    #[test]
    fn the_options_are_passed_on() {
        let files = json!({ "chain.sp": "// t\nr1 (a 0) resistor r=1k\n" });
        let v = answer(json!({ "entry": "chain.sp", "files": files, "dialect": "spectre" }));
        assert_eq!(v["document"]["dialect"], "spectre");
        let files = json!({ "a.sp": NETLIST });
        let with = answer(json!({ "entry": "a.sp", "files": files }));
        let without = answer(json!({ "entry": "a.sp", "files": files, "std": false }));
        let count = |v: &Value| v["document"]["diagnostics"].as_array().unwrap().len();
        assert_ne!(count(&with), count(&without));
        // the solve status only when asked for
        assert!(with["document"]["levels"][0].get("solve").is_none());
        let status = answer(json!({ "entry": "a.sp", "files": files, "status": true }));
        assert_eq!(status["document"]["levels"][0]["solve"], "unsolved");
        let native = BuildOptions { status: true, ..BuildOptions::default() };
        let native = rnl_view::build("a.sp", &MemoryProvider::new().with("a.sp", NETLIST), &native).unwrap();
        assert_eq!(status["document"], wire(native));
        // a file of 0.3 is read without any switch; `draft`, which the core once accepted with no
        // effect, is ignored as every field it does not know
        let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a 0 1k\n* @RNL { #R1 @PLACE(0, 0) }\n.end\n";
        let files = json!({ "t.sp": text });
        let plain = answer(json!({ "entry": "t.sp", "files": files }));
        assert_eq!(answer(json!({ "entry": "t.sp", "files": files, "draft": true })), plain);
        let diagnostics = plain["document"]["diagnostics"].as_array().unwrap();
        assert!(diagnostics.iter().all(|d| d["severity"] != "error"), "{diagnostics:?}");
    }

    #[test]
    fn packages_are_found_by_name_in_the_package_directories() {
        let top = "* top
R1 a 0 1k
* @RNL(type=\"header\") { @VERSION(\"0.2\"); @USE-PACKAGE(name=\"acme\", version=\"1.0\")}
.end
";
        let package = "@RNL(type=\"header\") { @VERSION(\"0.2\"); @PACKAGE(name=\"acme\", version=\"1.0\")}";
        let files = json!({ "/w/top.sp": top, "/w/packages/acme.rnl": package });
        let errors = |v: &Value| {
            let diagnostics = v["document"]["diagnostics"].as_array().unwrap();
            diagnostics.iter().filter(|d| d["severity"] == "error").count()
        };
        assert!(errors(&answer(json!({ "entry": "/w/top.sp", "files": files }))) > 0);
        let request = json!({ "entry": "/w/top.sp", "files": files, "package_dirs": ["/w/packages"] });
        assert_eq!(errors(&answer(request)), 0);
    }

    #[test]
    fn a_request_gives_the_same_answer_every_time() {
        // each build hashes with new keys, so an order taken from a hash map would show
        let text = include_str!("../../../tests/conformance/cases/16-ties.sp");
        let request = json!({ "entry": "/w/16-ties.sp", "files": { "/w/16-ties.sp": text } });
        let first = answer(request.clone());
        for _ in 0..8 {
            assert_eq!(answer(request.clone()), first);
        }
        let d = &first["document"];
        let diagnostics: Vec<_> = d["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| json!([d["severity"], d["file"], d["line"], d["col"], d["message"]]))
            .collect();
        assert_eq!(
            diagnostics,
            [json!(["error", "16-ties.sp", 32, 21, "$ABCZ cannot be found, did you mean $ABCV? (6.6)"])]
        );
        let lines: Vec<&Value> = d["files"][0]["failed"].as_array().unwrap().iter().map(|f| &f[0]).collect();
        // the statement that failed, then the overridden ones in the order they are written
        assert_eq!(lines, [32, 25, 26, 27, 28, 29, 30]);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let v = answer(json!({ "entry": "a.sp", "files": { "a.sp": NETLIST }, "unknown": { "of": [1, 2] } }));
        assert!(v["document"].is_object(), "{v}");
    }

    #[test]
    fn what_cannot_be_built_is_an_error() {
        let v = answer(json!({ "entry": "b.sp", "files": { "a.sp": NETLIST } }));
        assert_eq!(v, json!({ "error": "cannot read b.sp" }));
        let v = answer(json!({ "files": {} }));
        assert!(v["error"].as_str().unwrap().starts_with("the request is not valid: missing field `entry`"), "{v}");
        let invalid: [&[u8]; 3] = [b"null", b"{ \"entry\": ", b"{ \"entry\": \"a.sp\", \"std\": \"yes\" }"];
        for request in invalid {
            let v: Value = serde_json::from_str(&respond_json(request)).unwrap();
            assert!(v["error"].as_str().unwrap().starts_with("the request is not valid: "), "{v}");
        }
    }

    #[test]
    fn a_write_answers_the_text_and_its_document() {
        let netlist = "\n.subckt rc VSS\nR1 in VSS 1k\n.ends rc\n";
        let package = "@RNL(type=\"header\") { @VERSION(\"0.2\"); @PACKAGE(name=\"acme\", version=\"1.0\")}";
        let ops = json!([
            {"level": "subckt:rc", "op": "use-package", "name": "acme", "version": "1.0"},
            {"level": "subckt:rc", "op": "set", "object": "#R1", "at": [2, 0]}
        ]);
        let write = json!({ "dialect": "spice", "netlist": netlist, "author": "test", "ops": ops });
        let v = answer(
            json!({ "write": write, "files": { "/p/acme.rnl": package }, "package_dirs": ["/p"], "svg": false }),
        );
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["written"]);
        let text = v["written"]["text"].as_str().unwrap();
        assert_eq!(
            text,
            "\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"acme\", version=\"1.0\")}\n.subckt rc VSS\nR1 in VSS 1k\n* @RNL(author=\"test\") {\n*   #R1 @PLACE(2, 0)\n* }\n.ends rc\n"
        );
        let d = &v["written"]["document"];
        assert_eq!(
            (&d["path"], &d["dialect"], &d["files"][0]["text"]),
            (&json!("/rnl-edit/netlist.sp"), &json!("spice"), &json!(text))
        );
        let errors =
            |d: &Value| d["diagnostics"].as_array().unwrap().iter().filter(|d| d["severity"] == "error").count();
        assert_eq!(errors(d), 0, "{d}");
        assert_eq!(d["levels"][1]["svg"], Value::Null, "{d}");
        // without the package directory, the header's package cannot be found
        let v = answer(json!({ "write": write, "svg": false }));
        assert!(errors(&v["written"]["document"]) > 0);
        // Spectre: its extension and carrier
        let scs = json!({ "dialect": "spectre", "netlist": "simulator lang=spectre\nr1 (a 0) resistor r=1k\n", "author": "test",
                          "ops": [{"level": "file:x", "op": "set", "object": "#r1", "at": [0, 0]}] });
        let v = answer(json!({ "write": scs }));
        assert_eq!(v["written"]["document"]["path"], "/rnl-edit/netlist.scs");
        assert!(
            v["written"]["text"]
                .as_str()
                .unwrap()
                .contains("// @RNL(author=\"test\") {\n//   #r1 @PLACE(0, 0)\n// }\n"),
            "{v}"
        );
        // saved under a name that is not a .scs file: read so, in the SPICE mode up to its switch,
        // with the header after it, and built under that name; a request without it is as above
        let named = json!({ "dialect": "spectre", "netlist": "* t\nsimulator lang=spectre\nr1 (a 0) resistor r=1k\n",
                            "author": "test", "file": "top.net", "ops": [] });
        let v = answer(json!({ "write": named, "svg": false }));
        assert_eq!(v["written"]["document"]["path"], "/rnl-edit/top.net");
        assert_eq!(
            v["written"]["text"],
            "* t\nsimulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.3\")}\nr1 (a 0) resistor r=1k\n"
        );
        assert_eq!(errors(&v["written"]["document"]), 0, "{v}");
    }

    #[test]
    fn parts_ask_for_a_snapshot() {
        let files = json!({ "/w/a.sp": NETLIST });
        let v = answer(
            json!({ "entry": "/w/a.sp", "files": files, "parts": { "drawing": false, "host": true, "model": true } }),
        );
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["snapshot"]);
        let s = &v["snapshot"];
        assert_eq!(s["schema"], "rnl-snapshot/1");
        assert!(s.get("drawing").is_none());
        let level = &s["levels"][0];
        assert_eq!(level["host"]["instances"][0]["id"], "#R1");
        assert_eq!(level["model"]["objects"][1]["at"]["value"], json!([4.0, 0.0]));
        // the snapshot is the one rnl-view gives, and its drawing the Document of a build
        let provider = MemoryProvider::new().with("/w/a.sp", NETLIST);
        let parts = Parts { drawing: true, host: false, model: true };
        let native = rnl_view::snapshot("/w/a.sp", &provider, &BuildOptions::default(), parts).unwrap();
        let v = answer(json!({ "entry": "/w/a.sp", "files": files, "parts": { "model": true } }));
        assert_eq!(v["snapshot"], wire(native));
        let built = answer(json!({ "entry": "/w/a.sp", "files": files }));
        assert_eq!(v["snapshot"]["drawing"], built["document"]);
    }

    #[test]
    fn a_catalog_request_answers_the_catalog() {
        let v = answer(json!({ "catalog": {} }));
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["catalog"]);
        let symbols = v["catalog"]["symbols"].as_array().unwrap();
        let resistor = symbols.iter().find(|s| s["symbol"] == "$RESISTOR").unwrap();
        assert_eq!((&resistor["package"], &resistor["version"]), (&json!("rnl-std-basic"), &json!("0.3")));
        assert_eq!(resistor["pins"][0], json!({"terminal": 1, "at": [0.0, 2.0], "direction": "up"}));
        // the catalog rnl-view lists
        let native = rnl_view::catalog(&MemoryProvider::new(), &CatalogOptions::default());
        assert_eq!(v["catalog"], wire(native));
        // the frontend's own packages with the bundled ones, by name, every version
        let package = "@RNL(type=\"header\") { @VERSION(\"0.3\"); @PACKAGE(name=\"acme\", version=\"1.0\")}";
        let request = json!({ "catalog": { "packages": ["acme", "rnl-std"], "all_versions": true },
                              "files": { "/p/acme.rnl": package }, "package_dirs": ["/p"] });
        let v = answer(request);
        let listed: Vec<(&str, &str)> = v["catalog"]["packages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p["name"].as_str().unwrap(), p["version"].as_str().unwrap()))
            .collect();
        assert_eq!(listed, [("acme", "1.0"), ("rnl-std", "0.2"), ("rnl-std", "0.3")]);
        // a catalog request of another shape is not valid
        let v = answer(json!({ "catalog": { "all_versions": "yes" } }));
        assert!(v["error"].as_str().unwrap().starts_with("the request is not valid: "), "{v}");
    }

    #[test]
    fn the_drawings_of_the_unplaced_are_kept_for_a_request() {
        let text =
            "* some left\n.model nmos nmos\nR1 a b 1k\nC1 b 0 1p\nM1 d g s 0 nmos\n* @RNL { #R1 @PLACE(0, 0) }\n.end\n";
        let files = json!({ "a.sp": text });
        let carried = answer(json!({ "entry": "a.sp", "files": files, "primitives": true }));
        // nothing was kept by a build that did not ask for it
        let ask =
            |items: Value, names: Value| answer(json!({ "drawings": { "level": 0, "items": items, "names": names } }));
        assert!(ask(json!([0]), json!([]))["error"].as_str().unwrap().contains("lazy_unplaced"));
        let lazy = answer(json!({ "entry": "a.sp", "files": files, "primitives": true, "lazy_unplaced": true }));
        let listed = carried["document"]["levels"][0]["unplaced"].as_array().unwrap().clone();
        assert!(listed.len() >= 2);
        // the Document without the objects' drawings, and the drawings as the Document carries them
        let mut without = carried.clone();
        for u in without["document"]["levels"][0]["unplaced"].as_array_mut().unwrap() {
            let u = u.as_object_mut().unwrap();
            assert!(u.remove("svg").is_some() && u.remove("primitives").is_some());
        }
        assert_eq!(lazy, without);
        let names: Vec<Value> = listed.iter().map(|u| u["name"].clone()).collect();
        let all: Vec<usize> = (0..listed.len()).collect();
        let v = ask(json!(all), json!(names));
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["drawings"]);
        for (u, d) in listed.iter().zip(v["drawings"].as_array().unwrap()) {
            assert_eq!((&u["svg"], &u["primitives"]), (&d["svg"], &d["primitives"]), "{u}");
        }
        // some, in the order asked
        let v = ask(json!([1, 0]), json!([names[1], names[0]]));
        assert_eq!(v["drawings"][0]["svg"], listed[1]["svg"]);
        assert_eq!(v["drawings"][1]["svg"], listed[0]["svg"]);
        // an object that is not there, and names of another build, are refused
        assert!(ask(json!([99]), json!([]))["error"].as_str().unwrap().contains("99"));
        assert!(ask(json!([0]), json!(["R9"]))["error"].as_str().unwrap().contains("built again"));
        assert!(answer(json!({ "drawings": { "level": 5, "items": [0] } }))["error"].is_string());
        assert!(answer(json!({ "drawings": { "level": 0 } }))["error"]
            .as_str()
            .unwrap()
            .starts_with("the request is not valid: "));
        // a build that does not ask for it leaves what was kept, and one that does replaces it
        answer(json!({ "entry": "a.sp", "files": files }));
        assert!(ask(json!([0]), json!([]))["drawings"].is_array());
        let other = json!({ "a.sp": "* one\n.model npn npn\nQ1 c b e npn\n.end\n" });
        answer(json!({ "entry": "a.sp", "files": other, "lazy_unplaced": true }));
        assert!(ask(json!([0]), json!([names[0]]))["error"].as_str().unwrap().contains("built again"));
    }

    #[test]
    fn what_cannot_be_written_is_an_error() {
        let write = json!({ "dialect": "spice", "netlist": "\nR1 a 0 1k\n", "author": "test",
                            "ops": [{"level": "subckt:none", "op": "set", "object": "#R1", "at": [0, 0]}] });
        assert_eq!(
            answer(json!({ "write": write })),
            json!({ "error": "op 0 (set): the netlist has no subcircuit none" })
        );
        let v = answer(json!({ "write": { "dialect": "spice", "netlist": "" } }));
        assert!(v["error"].as_str().unwrap().starts_with("the request is not valid: missing field `author`"), "{v}");
    }

    #[test]
    fn the_exports_answer_a_request_written_into_the_module() {
        let request = json!({ "entry": "untitled-1", "files": { "untitled-1": NETLIST } }).to_string();
        let at = rnl_request(request.len());
        // SAFETY: `rnl_request` made room for `request.len()` bytes at `at`, as the caller of the
        // module writes them there.
        unsafe { std::ptr::copy_nonoverlapping(request.as_ptr(), at, request.len()) };
        let len = rnl_build();
        // SAFETY: `rnl_build` left `len` bytes of answer at `rnl_response()`, until the next call.
        let answer = unsafe { std::slice::from_raw_parts(rnl_response(), len) };
        assert_eq!(answer, respond_json(request.as_bytes()).as_bytes());
        assert_eq!((rnl_panic_len(), rnl_alloc_failed()), (0, 0));
        // an allocation that fails is noted for the caller, until the next call
        let huge = Layout::from_size_align(usize::MAX / 4, 1).unwrap();
        // SAFETY: the layout's size is not zero; the null pointer that comes back is not used.
        assert!(unsafe { ALLOCATOR.alloc(huge) }.is_null());
        assert_eq!(rnl_alloc_failed(), usize::MAX / 4);
        rnl_request(0);
        assert_eq!(rnl_alloc_failed(), 0);
    }
}
