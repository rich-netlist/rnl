//! The solver service of the page (the same the development server of the web viewer offers, apps/web/solver-service.mjs):
//!
//!   GET  /rnl-solver        {"service": "rnl-solver", "version": ..., "path": ..., "config": ...}
//!   GET  /rnl-solver/table  the table of the solver's settings (docs/solver-settings-table.md), which the page's settings
//!                           dialog is drawn from: what `rnl solver-defaults --json` prints
//!   POST /rnl-solver/check  {"config"}  ->  what `rnl solve --check --json` prints of those settings, with no file
//!   POST /rnl-solver/solve  {"entry", "files", "pool", "config", "timeall", "level", "stream"}  ->  {"document", "report"}
//!
//! `config` is the settings that differ from the defaults, as RNL named parameters, as `--solver-config` takes them.
//!
//! `level` is the title of the one level to solve (`rnl solve --level`), none for every level. With `"stream": true` the
//! answer is lines of JSON (`application/x-ndjson`): the events of the solve as they happen (docs/solver-progress.md), then
//! `{"result": {"document", "report"}}`, or `{"error": message}` if the solve fails after the first line was sent; a request
//! that is wrong is answered as it is without `stream`, as a 400 with `{"error"}`. The page going away stops the solve.
//!
//! The page sends the files of the netlist, with what it has applied over them, and the program solves them in memory
//! with the core's own `rnl_cli::preview`, as the desktop viewer does (decision 0010): nothing is read from or written
//! to disk, and the core runs here, not in the browser, so that the solver has the machine's cores and its memory.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rnl_cli::preview::{preview, preview_progress, StoppableClock};
use rnl_cli::progress::Event;
use rnl_cli::solve::{check_settings, Progress, SolveOptions, ToolSettings};
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
    /// The solver's settings, written as RNL named parameters as `rnl solve --solver-config` takes them.
    #[serde(default)]
    pub config: String,
    /// The seconds that all levels of a time limit share (`rnl solve --timeall`): none, each level has its own.
    #[serde(default)]
    pub timeall: Option<f64>,
    /// The title of the one level to solve: none, or empty, solves every level.
    #[serde(default)]
    pub level: Option<String>,
    /// Whether the answer is the events of the solve, as lines of JSON, and then the result.
    #[serde(default)]
    pub stream: bool,
}

/// What the service says of itself.
pub fn info() -> serde_json::Value {
    serde_json::json!({
        "service": "rnl-solver",
        "version": format!("rnl {} ({})", rnl_cli::CORE_VERSION, rnl_cli::BUILD_COMMIT),
        "path": "built into the program",
        "config": "",
    })
}

/// The table of the solver's settings, which the page's settings dialog is drawn from.
pub fn table() -> serde_json::Value {
    rnl_cli::solver_table::settings_table_json()
}

/// What the page asks to check: settings as named parameters.
#[derive(Debug, Deserialize)]
pub struct Check {
    #[serde(default)]
    pub config: String,
}

/// What `rnl solve --check --json` says of settings written as named parameters, with no file: one level, with each
/// setting in effect and the problems of each, by key. Err when the text does not read at all.
pub fn check(request: &Check) -> Result<serde_json::Value, String> {
    let checked = check_settings(
        None,
        &MemoryProvider::new(),
        &SolveOptions::default(),
        &[ToolSettings::Config(request.config.clone())],
    )?;
    serde_json::to_value(checked).map_err(|e| e.to_string())
}

/// The directory of a path of the page's workspace, `/`-separated.
fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(d, _)| d)
}

/// A request that is right: what a solve is made of.
pub struct Prepared {
    entry: String,
    files: MemoryProvider,
    options: SolveOptions,
}

/// Check a request and make what it solves: an error says what is wrong with it.
pub fn prepare(request: &Request) -> Result<Prepared, String> {
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
    let solver = SolverOptions::default().with_config(&request.config)?;
    let level = request.level.clone().filter(|title| !title.is_empty());
    if request.timeall.is_some_and(|s| !s.is_finite() || s <= 0.0) {
        return Err("timeall must be a positive number of seconds".into());
    }
    let options = SolveOptions { solver, level, total_seconds: request.timeall, ..SolveOptions::default() };
    Ok(Prepared { entry: request.entry.clone(), files, options })
}

/// Whether the page has closed its connection: asked without waiting for anything.
fn gone(peer: &TcpStream) -> bool {
    if peer.set_nonblocking(true).is_err() {
        return false;
    }
    let mut byte = [0u8; 1];
    let closed = matches!(peer.peek(&mut byte), Ok(0));
    let _ = peer.set_nonblocking(false);
    closed
}

/// The JSON text of `{document, report}` of a solve that is done.
fn solved_json(shown: &rnl_cli::preview::Preview) -> Result<String, String> {
    #[derive(serde::Serialize)]
    struct Solved<'a> {
        document: &'a rnl_view::Document,
        report: &'a rnl_cli::preview::Report,
    }
    serde_json::to_string(&Solved { document: &shown.document, report: &shown.report }).map_err(|e| e.to_string())
}

/// Solve a request: the JSON text of `{document, report}`. `peer` is the connection of the page, which is watched for the
/// page going away, as a cancel is: the solve stops at its next level.
pub fn solve(request: &Request, peer: &TcpStream) -> Result<String, String> {
    let Prepared { entry, files, options } = prepare(request)?;
    let stop = Arc::new(AtomicBool::new(false));
    let clock = StoppableClock::new(stop.clone());
    // the page going away is a cancel: asked at each level, without waiting for anything
    let watch = |_: &Progress| {
        if gone(peer) {
            stop.store(true, Ordering::Relaxed);
        }
        !stop.load(Ordering::Relaxed)
    };
    let shown = preview(&entry, &files, &options, &clock, &watch)?;
    solved_json(&shown)
}

/// Solve a prepared request and tell the page, on its connection, a line of JSON for each event of the solve as it
/// happens, then the result, or the error. A line that cannot be written is the page gone, as its closing the connection
/// is: the solve stops at its next step.
pub fn stream(prepared: Prepared, out: &mut TcpStream) -> io::Result<()> {
    let Prepared { entry, files, options } = prepared;
    let stop = Arc::new(AtomicBool::new(false));
    let clock = StoppableClock::new(stop.clone());
    let out = RefCell::new(out);
    let tell = |line: &str| -> io::Result<()> {
        let mut out = out.borrow_mut();
        out.write_all(line.as_bytes())?;
        out.write_all(b"
")?;
        out.flush()
    };
    let sink = |event: &Event| {
        let sent = serde_json::to_string(event).map_err(io::Error::other).and_then(|line| tell(&line));
        if sent.is_err() || gone(&out.borrow()) {
            stop.store(true, Ordering::Relaxed);
        }
        !stop.load(Ordering::Relaxed)
    };
    let end = match preview_progress(&entry, &files, &options, &clock, &sink).and_then(|shown| solved_json(&shown)) {
        // (the document is already JSON: it is put in the line as it is)
        Ok(result) => format!("{{\"result\":{result}}}"),
        Err(message) => serde_json::json!({ "error": message }).to_string(),
    };
    tell(&end)
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
        Request { entry: entry.into(), files: BTreeMap::from([(entry.to_string(), text.to_string())]), pool: Vec::new(), config: String::new(), timeall: None, level: None, stream: false }
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
    fn a_solver_setting_that_is_wrong_is_an_error_in_the_solvers_words() {
        let (_client, server) = pair();
        let mut r = request("/top.sp", "R1 a b 1k\n.end\n");
        r.config = "beam=0".into();
        let error = solve(&r, &server).unwrap_err();
        assert!(error.contains("the beam must be at least 1"), "{error}");
    }

    #[test]
    fn the_table_of_settings_and_the_check_are_the_solvers() {
        let table = table();
        assert_eq!(table["version"], 1);
        assert!(table["settings"].as_array().is_some_and(|s| s.iter().any(|row| row["key"] == "beam")));
        // settings that do: ok, and each says where it comes from
        let checked = check(&Check { config: "beam=8,layout=\"rows\"".into() }).unwrap();
        assert_eq!(checked["ok"], true);
        assert_eq!(checked["levels"][0]["settings"]["beam"]["from"], "command-line");
        // settings that do not: not ok, and the error says which setting
        let checked = check(&Check { config: "beam=0".into() }).unwrap();
        assert_eq!(checked["ok"], false);
        assert_eq!(checked["levels"][0]["errors"][0]["key"], "beam");
    }

    /// The lines a streamed solve of `request` tells, read as the page reads them.
    fn streamed(request: &Request) -> Vec<serde_json::Value> {
        use std::io::Read;
        let (mut client, mut server) = pair();
        let prepared = prepare(request).unwrap();
        // (on a thread: the answer is larger than the connection holds until it is read)
        let writer = std::thread::spawn(move || stream(prepared, &mut server).unwrap());
        let mut text = String::new();
        client.read_to_string(&mut text).unwrap();
        writer.join().unwrap();
        text.lines().map(|line| serde_json::from_str(line).unwrap()).collect()
    }

    const HIERARCHY: &str = include_str!("../../../tests/conformance/cases/04-hierarchy.sp");

    #[test]
    fn a_streamed_solve_tells_its_events_and_then_the_result() {
        let mut r = request("/top.sp", HIERARCHY);
        r.stream = true;
        let lines = streamed(&r);
        assert_eq!(lines[0]["event"], "run");
        assert_eq!(lines[0]["levels"], 2);
        assert!(lines.iter().any(|l| l["event"] == "level"));
        assert!(lines.iter().filter(|l| l["event"] == "level-done").count() >= 2);
        let last = lines.last().unwrap();
        assert!(last["result"]["document"]["levels"].as_array().is_some_and(|l| !l.is_empty()));
        assert!(last["result"]["report"]["levels"].as_array().is_some());
        // the events come before the result, which is the last line
        assert_eq!(lines.iter().filter(|l| l.get("result").is_some()).count(), 1);
    }

    #[test]
    fn the_whole_files_time_is_a_positive_number_or_none() {
        let (_client, server) = pair();
        let mut r = request("/top.sp", "R1 a b 1k\n.end\n");
        r.timeall = Some(60.0);
        assert!(solve(&r, &server).is_ok());
        for bad in [0.0, -3.0, f64::INFINITY] {
            r.timeall = Some(bad);
            let error = solve(&r, &server).unwrap_err();
            assert!(error.contains("timeall must be a positive number"), "{bad}: {error}");
        }
    }

    #[test]
    fn a_level_given_by_its_title_is_the_only_one_solved() {
        let mut r = request("/top.sp", HIERARCHY);
        r.stream = true;
        r.level = Some("amp".into());
        let lines = streamed(&r);
        assert_eq!(lines[0]["levels"], 1);
        let titles: Vec<_> = lines.iter().filter(|l| l["event"] == "level").map(|l| l["title"].as_str().unwrap().to_string()).collect();
        // (a file read as 0.2 is solved again as 0.3: the same level, told twice)
        assert!(!titles.is_empty() && titles.iter().all(|t| t == "amp"), "{titles:?}");
        assert!(lines.last().unwrap().get("result").is_some());
        // an empty title is none
        r.level = Some(String::new());
        assert_eq!(streamed(&r)[0]["levels"], 2);
    }

    #[test]
    fn a_solve_that_fails_after_its_first_line_ends_with_the_error() {
        let mut r = request("/top.sp", HIERARCHY);
        r.stream = true;
        r.level = Some("no such level".into());
        let lines = streamed(&r);
        let last = lines.last().unwrap();
        assert!(last["error"].as_str().is_some_and(|e| !e.is_empty()), "{last}");
    }

    #[test]
    fn the_answer_to_a_request_without_stream_is_the_result_alone_and_takes_a_level() {
        let (_client, server) = pair();
        let mut r = request("/top.sp", HIERARCHY);
        r.level = Some("amp".into());
        let json: serde_json::Value = serde_json::from_str(&solve(&r, &server).unwrap()).unwrap();
        assert_eq!(json["report"]["levels"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn the_service_says_what_it_is() {
        let i = info();
        assert_eq!(i["service"], "rnl-solver");
        assert!(i["version"].as_str().unwrap().starts_with("rnl "));
    }
}
