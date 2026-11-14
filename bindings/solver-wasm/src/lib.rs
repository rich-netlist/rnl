//! The solver for WebAssembly, a module of its own beside the core's (`bindings/runtime-wasm`), so
//! that a page loads it only to solve: [`solve`], a [`SolveRequest`] in and the Document drawn as
//! solved out, as `rnl preview` gives it, behind the same small C-style interface over linear
//! memory, with UTF-8 JSON in and out.
//!
//! A call goes like this:
//!
//! 1. `rnl_request(len)` makes room for a request of `len` bytes and returns where to write it.
//! 2. The caller writes the request there: a [`SolveRequest`] as UTF-8 JSON.
//! 3. `rnl_solve()` solves it and returns the length of the answer, an [`Answer`] as UTF-8 JSON;
//!    `rnl_response()` returns where it is.
//!
//! Unlike the core's module, this one imports three functions from `env`, which the Worker that
//! runs it gives:
//!
//! - `rnl_now() -> f64`: milliseconds from some start, such as `performance.now()`, which the
//!   solver's time limit is measured on. It has to go on: a time limit never ends on a clock that
//!   stands still, so a caller with such a clock gives a limit of work, or the seed;
//! - `rnl_progress(ptr, len)`: one progress event, UTF-8 JSON at `ptr`, as `rnl solve
//!   --progress=json` writes them (`docs/solver-progress.md`);
//! - `rnl_cancelled() -> i32`: not 0 to cancel, asked as the events come.
//!
//! The call runs to its end in the Worker. To stop it sooner, a page terminates the Worker and
//! starts another; nothing of the core's Worker depends on this one, since every request carries
//! all it reads. A panic traps, with its message kept as the core's module keeps it. After a
//! trap, or an exception thrown through the call from an import, the instance is not to be used
//! again: what the call held is not released. `js/solver.js` makes a new one.

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, Once, PoisonError};

use rnl_cli::solve::{SolveOptions, CANCELLED};
use rnl_runtime::source::{normalize, MemoryProvider};
use rnl_solver::{Clock, SolverOptions};
use serde::{Deserialize, Serialize};

/// What to solve: the texts of the workspace and the settings. Every field the solve reads is
/// here. Fields it does not know are ignored, those of a build request of the core's module that
/// are not here among them (`pool`, `auto_include`, `library_prefix`, `library_edits`,
/// `std_version`).
#[derive(Deserialize, Debug)]
pub struct SolveRequest {
    /// The path of the file to solve, as the frontend names it.
    pub entry: String,
    /// The texts of the workspace by path, the entry's among them. Two paths that are one file,
    /// such as `a/b.sp` and `a\b.sp`, are an error. The answer names each file written by the
    /// path it has here.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// The directories, among the paths of `files`, whose packages are found by name (16.2).
    #[serde(default)]
    pub package_dirs: Vec<String>,
    /// "spice" or "spectre", instead of guessing from the entry.
    pub dialect: Option<String>,
    /// The format set for files, by path as in `files`.
    #[serde(default)]
    pub formats: BTreeMap<String, String>,
    /// Use the standard package at the root (16.3); true when not given.
    #[serde(default = "yes")]
    pub std: bool,
    /// Add the standard package to draft 0.4 files; false when not given.
    #[serde(default)]
    pub include_std: bool,
    /// The solver settings as RNL named parameters, as `rnl solve --solver-config` takes them,
    /// such as `limit="time",seconds=10`.
    #[serde(default)]
    pub solver_config: String,
    /// Only the level with this title.
    pub level: Option<String>,
    /// Space each level again from its record of the solver, nothing searched, as `rnl solve
    /// --respace` does (`docs/solver-respace.md`).
    #[serde(default)]
    pub respace: bool,
}

fn yes() -> bool {
    true
}

/// The answer to a [`SolveRequest`].
#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Answer {
    /// The Document drawn as solved, the report, and the texts the solve wrote by path, as
    /// `rnl preview` gives them.
    Solved {
        document: serde_json::Value,
        report: serde_json::Value,
        files: BTreeMap<String, String>,
    },
    /// The solve was cancelled: nothing is given back.
    Cancelled {
        cancelled: bool,
    },
    Error {
        error: String,
    },
}

/// The clock of a solve: milliseconds from `now`, in seconds. It is a wall clock, and reading it
/// is a call out of the module, so the budget reads it every so much work, not at every step.
/// The time is counted from the first reading that is a number. A reading that is not a number,
/// or is earlier than one before, counts as the latest one before it: the solver's clock never
/// goes back, and it does not start with a leap.
struct Now<'a> {
    now: &'a dyn Fn() -> f64,
    /// The first reading that is a number and the latest, in seconds.
    read: std::cell::Cell<Option<(f64, f64)>>,
}

impl Clock for Now<'_> {
    fn elapsed(&self) -> f64 {
        let now = (self.now)() / 1000.;
        let (first, latest) = match self.read.get() {
            Some((first, latest)) => (first, if now.is_finite() && now > latest { now } else { latest }),
            None if now.is_finite() => (now, now),
            None => return 0.,
        };
        self.read.set(Some((first, latest)));
        latest - first
    }

    fn check_every(&self) -> u64 {
        rnl_solver::WALL_CLOCK_CHECK_EVERY
    }
}

/// Solve `request`, a [`SolveRequest`] as JSON, with the time from `now` in milliseconds, and each
/// progress event given to `progress` as JSON, which answers false to cancel.
pub fn solve(request: &[u8], now: &dyn Fn() -> f64, progress: &dyn Fn(&str) -> bool) -> Answer {
    let request: SolveRequest = match serde_json::from_slice(request) {
        Ok(r) => r,
        Err(e) => return Answer::Error { error: format!("the request is not valid: {e}") },
    };
    let solver = match SolverOptions::default().with_config(&request.solver_config) {
        Ok(s) => s,
        Err(e) => return Answer::Error { error: format!("solver_config: {e}") },
    };
    let options = SolveOptions {
        dialect: request.dialect,
        formats: request.formats,
        std: request.std,
        include_std: request.include_std,
        level: request.level,
        solver,
    };
    // the solve names files by their normal paths; the answer, by the paths of the request
    let mut asked: BTreeMap<String, String> = BTreeMap::new();
    for path in request.files.keys() {
        if let Some(other) = asked.insert(normalize(path), path.clone()) {
            return Answer::Error { error: format!("files: {other} and {path} are one file") };
        }
    }
    let files = MemoryProvider {
        files: request.files.into_iter().map(|(p, t)| (normalize(&p), t)).collect(),
        package_dirs: request.package_dirs.iter().map(|p| normalize(p)).collect(),
    };
    let clock = Now { now, read: std::cell::Cell::new(None) };
    let sink = |e: &rnl_cli::progress::Event| progress(&e.line());
    let shown = if request.respace {
        rnl_cli::preview::respace(&request.entry, &files, &options, &clock, &sink)
    } else {
        rnl_cli::preview::preview_progress(&request.entry, &files, &options, &clock, &sink)
    };
    match shown {
        Ok(shown) => Answer::Solved {
            document: serde_json::to_value(&shown.document).expect("a Document serializes"),
            report: serde_json::to_value(&shown.report).expect("a report serializes"),
            files: shown
                .copy
                .files
                .into_iter()
                .map(|f| (asked.get(&normalize(&f.path)).cloned().unwrap_or(f.path), f.text))
                .collect(),
        },
        Err(e) if e == CANCELLED => Answer::Cancelled { cancelled: true },
        Err(error) => Answer::Error { error },
    }
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn rnl_now() -> f64;
    fn rnl_progress(ptr: *const u8, len: usize);
    fn rnl_cancelled() -> i32;
}

/// The request, written by the caller where [`rnl_request`] said.
static REQUEST: Mutex<Vec<u8>> = Mutex::new(Vec::new());
/// The answer to the last request.
static RESPONSE: Mutex<String> = Mutex::new(String::new());
/// The message of the last panic, kept by the panic hook for the caller to read after the trap.
static PANIC: Mutex<String> = Mutex::new(String::new());
/// The size of an allocation that failed in the last call; 0 when none did.
static FAILED_ALLOCATION: AtomicUsize = AtomicUsize::new(0);

/// The system allocator, noting the size of an allocation that fails, as the core's module does.
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

/// Solve the request written where [`rnl_request`] said, and return the length of the answer.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn rnl_solve() -> usize {
    begin();
    let request = std::mem::take(&mut *lock(&REQUEST));
    // SAFETY: the imports take and give plain numbers; the event's bytes live through the call.
    let now = || unsafe { rnl_now() };
    let progress = |event: &str| unsafe {
        rnl_progress(event.as_ptr(), event.len());
        rnl_cancelled() == 0
    };
    let answer = serde_json::to_string(&solve(&request, &now, &progress)).expect("an answer serializes");
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

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Keep the message of a panic for the caller, before the hook that was there.
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
    use std::cell::RefCell;
    use std::path::Path;

    const SOURCE: &str = r#"* wasm
* @RNL(type="header") { @VERSION("0.4") }
R1 in a 1k
R2 a 0 1k
* @RNL {
* $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1,direction="left"); (2,0) @PIN(terminal=2,direction="right"); (-1,-1) (1,-1) (1,1) (-1,1) (-1,-1) @LINE };
* {#R1 #R2} @SET-SYMBOL $S;
* &a @ROUTE-NET;
* }
.end
"#;

    const CONFIG: &str = r#"limit="work",work=2000000000"#;

    fn request(extra: serde_json::Value) -> Vec<u8> {
        let mut r = serde_json::json!({
            "entry": "/w/main.sp",
            "files": { "/w/main.sp": SOURCE },
            "std": false,
            "solver_config": CONFIG,
        });
        r.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        serde_json::to_vec(&r).unwrap()
    }

    #[test]
    fn a_request_is_solved_with_its_events_and_writes_what_the_cli_writes() {
        let events = RefCell::new(Vec::new());
        // a clock that stands still, as the CLI's below, so that no time limit of the resolution
        // is reached in either
        let now = || 0.;
        let answer = solve(&request(serde_json::json!({})), &now, &|e| {
            events.borrow_mut().push(e.to_string());
            true
        });
        let Answer::Solved { files, report, document } = answer else { panic!("{answer:?}") };
        assert_eq!(report["mode"], "solve");
        assert!(document["levels"].is_array());
        let options = SolveOptions {
            std: false,
            solver: SolverOptions::default().with_config(CONFIG).unwrap(),
            ..Default::default()
        };
        let given = MemoryProvider::new().with("/w/main.sp", SOURCE);
        let cli =
            rnl_cli::solve::solve_with_clock(Path::new("/w/main.sp"), &given, &options, &rnl_solver::StoppedClock)
                .unwrap();
        assert_eq!(files.values().next(), Some(&cli.files[0].text), "the same text as the CLI writes");
        let events = events.into_inner();
        assert!(events[0].starts_with(r#"{"event":"run""#), "{events:?}");
        assert!(events.iter().any(|e| e.starts_with(r#"{"event":"level-done""#)));
    }

    #[test]
    fn a_cancel_gives_nothing_back_and_a_bad_request_says_why() {
        let answer = solve(&request(serde_json::json!({})), &|| 0., &|_| false);
        assert!(matches!(answer, Answer::Cancelled { cancelled: true }), "{answer:?}");
        let answer = solve(&request(serde_json::json!({ "solver_config": "beam=0" })), &|| 0., &|_| true);
        assert!(matches!(&answer, Answer::Error { error } if error.starts_with("solver_config:")), "{answer:?}");
        assert!(matches!(solve(b"{", &|| 0., &|_| true), Answer::Error { .. }));
    }
}
