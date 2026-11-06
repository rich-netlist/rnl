//! The solver service of the page (the same the development server of the web viewer offers, apps/web/solver-service.mjs):
//!
//!   GET  /rnl-solver        {"service": "rnl-solver", "version": ..., "path": ..., "options": ...}
//!   POST /rnl-solver/solve  {"entry", "files", "pool", "options"}  ->  {"document", "report"}
//!
//! The page sends the files of the netlist, with what it has applied over them, and the program solves them in memory
//! with the core's own `rnl_cli::preview`, as the desktop viewer does (decision 0010): nothing is read from or written
//! to disk, and the core runs here, not in the browser, so that the solver has the machine's cores and its memory.

use std::collections::BTreeMap;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rnl_cli::preview::{preview, StoppableClock};
use rnl_cli::solve::{Progress, SolveOptions};
use rnl_runtime::source::MemoryProvider;
use rnl_solver::SolverOptions;
use serde::Deserialize;

/// What the page sends for a solve.
#[derive(Debug, Deserialize)]
pub struct Request {
    pub entry: String,
    pub files: BTreeMap<String, String>,
    /// The paths of the files of the Library among them: the packages by name, the includes by path.
    #[serde(default)]
    pub pool: Vec<String>,
    /// The solver's settings, written as `rnl solve --solver-options` takes them.
    #[serde(default)]
    pub options: String,
}

/// What the service says of itself.
pub fn info() -> serde_json::Value {
    serde_json::json!({
        "service": "rnl-solver",
        "version": format!("rnl {} ({})", rnl_cli::CORE_VERSION, rnl_cli::BUILD_COMMIT),
        "path": "built into the program",
        "options": "",
    })
}

/// The directory of a path of the page's workspace, `/`-separated.
fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(d, _)| d)
}

/// Solve a request: the JSON text of `{document, report}`. `peer` is the connection of the page, which is watched for the
/// page going away, as a cancel is: the solve stops at its next level.
pub fn solve(request: &Request, peer: &TcpStream) -> Result<String, String> {
    if !request.entry.starts_with('/') || request.entry.contains("/../") || !request.files.contains_key(&request.entry) {
        return Err("the request has no entry file".into());
    }
    let mut files = MemoryProvider::new();
    for (path, text) in &request.files {
        if path.starts_with('/') && !path.split('/').any(|part| part == "..") {
            files.insert(path, text);
        }
    }
    // the packages of the Library are found by name in the folders that hold them
    for path in &request.pool {
        if path.ends_with(".rnl") && request.files.contains_key(path) {
            let dir = dir_of(path).to_string();
            if !files.package_dirs.contains(&dir) {
                files.package_dirs.push(dir);
            }
        }
    }
    let solver = SolverOptions::default().with_overrides(&request.options)?;
    let options = SolveOptions { solver, ..SolveOptions::default() };

    let stop = Arc::new(AtomicBool::new(false));
    let clock = StoppableClock::new(stop.clone());
    // the page going away is a cancel: asked at each level, without waiting for anything
    let gone = || {
        if peer.set_nonblocking(true).is_err() {
            return false;
        }
        let mut byte = [0u8; 1];
        let closed = matches!(peer.peek(&mut byte), Ok(0));
        let _ = peer.set_nonblocking(false);
        closed
    };
    let watch = |_: &Progress| {
        if gone() {
            stop.store(true, Ordering::Relaxed);
        }
        !stop.load(Ordering::Relaxed)
    };
    let shown = preview(&request.entry, &files, &options, &clock, &watch)?;
    #[derive(serde::Serialize)]
    struct Solved<'a> {
        document: &'a rnl_view::Document,
        report: &'a rnl_cli::preview::Report,
    }
    serde_json::to_string(&Solved { document: &shown.document, report: &shown.report }).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};

    fn pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        (client, server)
    }

    fn request(entry: &str, text: &str) -> Request {
        Request { entry: entry.into(), files: BTreeMap::from([(entry.to_string(), text.to_string())]), pool: Vec::new(), options: String::new() }
    }

    #[test]
    fn a_netlist_is_solved_in_memory_and_the_document_and_the_report_come_back() {
        let (_client, server) = pair();
        let text = "R1 a b 1k\nR2 b 0 1k\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n* @RNL { {#R1 #R2} @ALIGN(\"y\") }\n.end\n";
        let answer = solve(&request("/top.sp", text), &server).unwrap();
        let json: serde_json::Value = serde_json::from_str(&answer).unwrap();
        assert!(json["document"]["levels"].as_array().is_some_and(|l| !l.is_empty()));
        assert!(json["report"]["levels"].as_array().is_some());
    }

    #[test]
    fn a_request_without_its_entry_or_with_a_path_that_climbs_is_refused() {
        let (_client, server) = pair();
        let mut r = request("/top.sp", "R1 a b 1k\n.end\n");
        r.entry = "/other.sp".into();
        assert!(solve(&r, &server).is_err());
        let mut r = request("/a/../top.sp", "R1 a b 1k\n.end\n");
        assert!(solve(&r, &server).is_err());
        r.entry = "top.sp".into();
        assert!(solve(&r, &server).is_err());
    }

    #[test]
    fn a_solver_option_that_is_wrong_is_an_error_in_the_solvers_words() {
        let (_client, server) = pair();
        let mut r = request("/top.sp", "R1 a b 1k\n.end\n");
        r.options = "no_such_option=1".into();
        let error = solve(&r, &server).unwrap_err();
        assert!(!error.is_empty());
    }

    #[test]
    fn the_service_says_what_it_is() {
        let i = info();
        assert_eq!(i["service"], "rnl-solver");
        assert!(i["version"].as_str().unwrap().starts_with("rnl "));
    }
}
