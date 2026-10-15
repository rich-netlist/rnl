//! The core for WebAssembly (decision 0008): [`rnl_view::build`] behind a small C-style interface
//! over linear memory, a JSON request in and a JSON response out. It needs no generated glue: the
//! module imports nothing, and a page or a Worker calls its exports as `js/core.js` does.
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
use std::collections::BTreeMap;
use std::fmt::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, Once, PoisonError};

use rnl_runtime::source::{normalize, MemoryProvider};
use rnl_runtime::Options;
use rnl_view::{BuildOptions, Document};
use serde::{Deserialize, Serialize};

/// What to build: the workspace and how. Fields the core does not know are ignored, so that a
/// frontend can ask for options an older core does not have yet.
#[derive(Deserialize, Debug)]
pub struct Request {
    /// The path of the file to open, as the frontend names it (see [`rnl_view::build`]).
    pub entry: String,
    /// The texts of the workspace by path, the entry's among them.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// The directories, among the paths of `files`, whose packages are found by name (16.2).
    #[serde(default)]
    pub package_dirs: Vec<String>,
    /// "spice" or "spectre", instead of guessing from the entry.
    pub dialect: Option<String>,
    /// Use the standard package at the root (16.3); true when not given.
    #[serde(default = "yes")]
    pub std: bool,
    /// Write each level's SVG; true when not given.
    #[serde(default = "yes")]
    pub svg: bool,
    /// Give each level's primitives; false when not given.
    #[serde(default)]
    pub primitives: bool,
    /// For development only: read the files that declare the draft 0.3 as well; false when not
    /// given.
    #[serde(default)]
    pub draft: bool,
    /// Give each level its solve status, what a frontend decides from whether to solve it
    /// (decision 0008); false when not given.
    #[serde(default)]
    pub status: bool,
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

/// The answer to a request: `{"document": ...}` or `{"error": "..."}`.
#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Response {
    /// The Document of the entry.
    Document(Box<Document>),
    /// Why there is none: the request is not a [`Request`], or the entry cannot be read.
    Error(String),
}

/// Answer a request given as JSON.
pub fn respond(request: &[u8]) -> Response {
    let request: Request = match serde_json::from_slice(request) {
        Ok(r) => r,
        Err(e) => return Response::Error(format!("the request is not valid: {e}")),
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
    let options = BuildOptions {
        dialect: request.dialect,
        runtime: Options { std: request.std, draft: request.draft, ..Options::default() },
        svg: request.svg,
        primitives: request.primitives,
        status: request.status,
    };
    match rnl_view::build(&request.entry, &files, &options) {
        Ok(d) => Response::Document(Box::new(d)),
        Err(e) => Response::Error(e),
    }
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
        assert_eq!(v["document"], serde_json::to_value(native).unwrap());
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
        assert_eq!(status["document"], serde_json::to_value(native).unwrap());
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
        let hint = d["diagnostics"].as_array().unwrap().iter().find_map(|d| d["message"].as_str());
        assert_eq!(hint, Some("$ABCZ cannot be found, did you mean $ABCV? (6.6)"));
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
