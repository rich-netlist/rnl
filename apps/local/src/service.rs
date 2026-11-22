//! The solver service of the page (the same the development server of the web viewer offers, apps/web/solver-service.mjs):
//!
//!   GET  /rnl-solver        {"service": "rnl-solver", "version": ..., "path": ..., "config": ...}
//!   GET  /rnl-solver/table  the table of the solver's settings (docs/solver-settings-table.md), which the page's settings
//!                           dialog is drawn from: what `rnl solver-defaults --json` prints
//!   POST /rnl-solver/check  {"config"}  ->  what `rnl solve --check --json` prints of those settings, with no file
//!   POST /rnl-solver/solve  {"entry", "files", "pool", "package_dirs", "search_paths", "auto_include", "library_prefix",
//!                            "library_edits", "formats", "config", "timeall", "level", "file", "respace", "stream"}
//!                           ->  {"document", "report", "files"}
//!
//! `config` is the settings that differ from the defaults, as RNL named parameters, as `--solver-config` takes them.
//! The files are those of the page's build request, read as the core reads them for every frontend
//! (`rnl_view::RequestFiles`): the pool first, then the PATH folders, and Auto Include.
//! `formats` is the format set for each file, by its path, as the page read the netlist with (`rnl --format`).
//! The answer's `files` are the texts the solve wrote, by the request's paths, which the page draws the preview again
//! from (Export Image), as `rnl preview --request` and the WebAssembly solver give them.
//!
//! `level` is the title of the one level to solve (`rnl solve --level`), none for every level; `file`, with `level`, the file
//! of the document it is written in, as the document names it, when the page shows that file read alone: only that file is
//! read for the level, and the answer's Document is that file's (`rnl solve --file`). With `"stream": true` the
//! answer is lines of JSON (`application/x-ndjson`): the events of the solve as they happen (docs/solver-progress.md), then
//! `{"result": {"document", "report", "files"}}`, or `{"error": message}` if the solve fails after the first line was sent; a request
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
use std::sync::{Arc, Mutex};

use rnl_cli::preview::{preview, preview_progress, respace, Copy, StoppableClock};
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
    /// The paths of the files of the Library among them: searched first, for packages by name and for bare includes, and
    /// indexed for Auto Include.
    #[serde(default)]
    pub pool: Vec<String>,
    /// The PATH folders among them, in order: searched for packages by name after the pool (`package_dirs`) and for bare
    /// includes (`search_paths`), as a build request has them.
    #[serde(default)]
    pub package_dirs: Vec<String>,
    #[serde(default)]
    pub search_paths: Vec<String>,
    /// Whether a subcircuit no file read defines is taken from the Library's files (Auto Include).
    #[serde(default)]
    pub auto_include: bool,
    /// The start of the Library's paths that its index leaves out, and the texts applied over its files, as a build
    /// request has them.
    #[serde(default)]
    pub library_prefix: String,
    #[serde(default)]
    pub library_edits: BTreeMap<String, String>,
    /// The format set for each file by its path, such as `spectre`, as the page opened it with: the solve reads the files
    /// as the page did (`rnl --format`).
    #[serde(default)]
    pub formats: BTreeMap<String, String>,
    /// The solver's settings, written as RNL named parameters as `rnl solve --solver-config` takes them.
    #[serde(default)]
    pub config: String,
    /// The seconds that all levels of a time limit share (`rnl solve --timeall`): none, each level has its own.
    #[serde(default)]
    pub timeall: Option<f64>,
    /// The title of the one level to solve: none, or empty, solves every level.
    #[serde(default)]
    pub level: Option<String>,
    /// With `level`, the file of the document the level is written in, as the document names it: only that file is read
    /// for the level (`rnl solve --file`), and the Document answered is that file's, as the page reads it alone.
    #[serde(default)]
    pub file: Option<String>,
    /// Each level as the record of the solver in the file last drew it, spaced again as the settings say, nothing
    /// searched (`rnl preview --respace`).
    #[serde(default)]
    pub respace: bool,
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

/// A request that is right: what a solve is made of.
pub struct Prepared {
    entry: String,
    files: rnl_view::Opened,
    options: SolveOptions,
    respace: bool,
    /// The request's paths, by the normal path the solve names each file by.
    asked: std::collections::BTreeMap<String, String>,
}

/// Check a request and make what it solves: an error says what is wrong with it.
pub fn prepare(request: &Request) -> Result<Prepared, String> {
    if !request.entry.starts_with('/') || request.entry.contains("/../") || !request.files.contains_key(&request.entry)
    {
        return Err("the request has no entry file".into());
    }
    let inside = |path: &str| path.starts_with('/') && !path.split('/').any(|part| part == "..");
    let workspace = rnl_view::RequestFiles {
        files: request
            .files
            .iter()
            .filter(|(path, _)| inside(path.as_str()))
            .map(|(p, t)| (p.clone(), t.clone()))
            .collect(),
        pool: request.pool.clone(),
        package_dirs: request.package_dirs.clone(),
        search_paths: request.search_paths.clone(),
        auto_include: request.auto_include,
        library_prefix: request.library_prefix.clone(),
        library_edits: request.library_edits.clone(),
    };
    let solver = SolverOptions::default().with_config(&request.config)?;
    let level = request.level.clone().filter(|title| !title.is_empty());
    if request.timeall.is_some_and(|s| !s.is_finite() || s <= 0.0) {
        return Err("timeall must be a positive number of seconds".into());
    }
    // (the formats of the files the request has; a name the core does not know is its error, as the page's build was)
    let formats = request
        .formats
        .iter()
        .filter(|(path, _)| request.files.contains_key(*path))
        .map(|(p, f)| (p.clone(), f.clone()));
    let options = SolveOptions {
        solver,
        level,
        file: request.file.clone().filter(|file| !file.is_empty()),
        total_seconds: request.timeall,
        formats: formats.collect(),
        ..SolveOptions::default()
    };
    let asked = rnl_cli::preview::request_paths(workspace.files.keys())?;
    let files = workspace.open(&options.formats);
    Ok(Prepared { entry: request.entry.clone(), files, options, respace: request.respace, asked })
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

/// The JSON text of `{document, report, files}` of a solve that is done: `files` the texts it wrote, by the request's paths.
fn solved_json(
    shown: &rnl_cli::preview::Preview,
    asked: &std::collections::BTreeMap<String, String>,
) -> Result<String, String> {
    #[derive(serde::Serialize)]
    struct Solved<'a> {
        document: rnl_cli::preview::Shown<'a>,
        report: &'a rnl_cli::preview::Report,
        files: std::collections::BTreeMap<String, String>,
    }
    let files = shown.copy.written_by(asked);
    serde_json::to_string(&Solved { document: shown.shown(), report: &shown.report, files }).map_err(|e| e.to_string())
}

/// Solve a request: the JSON text of `{document, report, files}`. `peer` is the connection of the page, which is watched for the
/// page going away, as a cancel is: the solve stops at its next level.
pub fn solve(request: &Request, peer: &TcpStream, last: &LastSolve) -> Result<String, String> {
    let Prepared { entry, files, options, respace: spaced, asked } = prepare(request)?;
    let stop = Arc::new(AtomicBool::new(false));
    let clock = StoppableClock::new(stop.clone());
    // the page going away is a cancel: asked at each level, without waiting for anything
    let watch = |_: &Progress| {
        if gone(peer) {
            stop.store(true, Ordering::Relaxed);
        }
        !stop.load(Ordering::Relaxed)
    };
    let shown = if spaced {
        // (a respace tells its events, not the steps of a search: the page going away is asked at each)
        respace(&entry, &files, &options, &clock, &|_: &Event| {
            if gone(peer) {
                stop.store(true, Ordering::Relaxed);
            }
            !stop.load(Ordering::Relaxed)
        })?
    } else {
        preview(&entry, &files, &options, &clock, &watch)?
    };
    keep(last, &shown.copy);
    solved_json(&shown, &asked)
}

/// The last solve shown, as the desktop keeps it for Solve › Save Solved Copy As: the file solved, the texts the solve
/// wrote and the files it read, by the paths the page gave them.
pub type LastSolve = Arc<Mutex<Option<Copy>>>;

fn keep(last: &LastSolve, copy: &Copy) {
    *last.lock().unwrap_or_else(|e| e.into_inner()) = Some(copy.clone());
}

/// Solve a prepared request and tell the page, on its connection, a line of JSON for each event of the solve as it
/// happens, then the result, or the error. A line that cannot be written is the page gone, as its closing the connection
/// is: the solve stops at its next step.
pub fn stream(prepared: Prepared, out: &mut TcpStream, last: &LastSolve) -> io::Result<()> {
    let Prepared { entry, files, options, respace: spaced, asked } = prepared;
    let stop = Arc::new(AtomicBool::new(false));
    let clock = StoppableClock::new(stop.clone());
    let out = RefCell::new(out);
    let tell = |line: &str| -> io::Result<()> {
        let mut out = out.borrow_mut();
        out.write_all(line.as_bytes())?;
        out.write_all(
            b"
",
        )?;
        out.flush()
    };
    let sink = |event: &Event| {
        let sent = serde_json::to_string(event).map_err(io::Error::other).and_then(|line| tell(&line));
        if sent.is_err() || gone(&out.borrow()) {
            stop.store(true, Ordering::Relaxed);
        }
        !stop.load(Ordering::Relaxed)
    };
    let shown = if spaced {
        respace(&entry, &files, &options, &clock, &sink)
    } else {
        preview_progress(&entry, &files, &options, &clock, &sink)
    };
    let end = match shown.and_then(|shown| {
        keep(last, &shown.copy);
        solved_json(&shown, &asked)
    }) {
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
        Request {
            entry: entry.into(),
            files: BTreeMap::from([(entry.to_string(), text.to_string())]),
            pool: Vec::new(),
            package_dirs: Vec::new(),
            search_paths: Vec::new(),
            auto_include: false,
            library_prefix: String::new(),
            library_edits: BTreeMap::new(),
            config: String::new(),
            timeall: None,
            level: None,
            file: None,
            respace: false,
            stream: false,
            formats: BTreeMap::new(),
        }
    }

    #[test]
    fn a_file_is_read_in_the_format_the_page_set_for_it() {
        let (_client, server) = pair();
        // Spectre written in a .sp file: read as SPICE, its nets would be `(a` and `0)`
        let text = "r1 (a 0) resistor r=1k\n";
        let nets = |r: &Request| {
            let json: serde_json::Value =
                serde_json::from_str(&solve(r, &server, &LastSolve::default()).unwrap()).unwrap();
            let mut nets: Vec<String> = json["document"]["objects"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|o| o["kind"] == "net")
                .map(|o| o["name"].as_str().unwrap().to_string())
                .collect();
            nets.sort();
            (nets, json["document"]["files"][0]["format"].as_str().map(str::to_string))
        };
        let mut r = request("/top.sp", text);
        r.formats.insert("/top.sp".into(), "spectre".into());
        assert_eq!(nets(&r).1.as_deref(), Some("spectre"));
        assert!(!nets(&r).0.iter().any(|n| n.contains('(') || n.contains(')')), "{:?}", nets(&r).0);
        r.formats.clear();
        assert_eq!(nets(&r).1.as_deref(), Some("spice"));
    }

    #[test]
    fn a_level_solved_with_its_file_comes_back_as_the_reading_of_that_file() {
        let (_client, server) = pair();
        let mut r = request("/top.sp", "* top\n.include \"cells.inc\"\nX1 a b cell\nR9 b 0 1k\n.end\n");
        r.files.insert("/cells.inc".into(), "* cells\n.subckt cell p n\nR1 p m 1k\nR2 m n 1k\n.ends\n".into());
        r.level = Some("cell".into());
        let solved = |r: &Request| -> serde_json::Value {
            serde_json::from_str(&solve(r, &server, &LastSolve::default()).unwrap()).unwrap()
        };
        let titles = |json: &serde_json::Value| -> Vec<String> {
            json["document"]["levels"]
                .as_array()
                .unwrap()
                .iter()
                .map(|l| l["title"].as_str().unwrap().to_string())
                .collect()
        };
        // the whole document, as without the file
        let whole = solved(&r);
        assert!(whole["document"].get("file").is_none());
        assert!(titles(&whole).len() > 1, "{:?}", titles(&whole));
        // the file the level is written in: that file's reading, its levels alone
        r.file = Some("cells.inc".into());
        let one = solved(&r);
        assert_eq!(one["document"]["file"], "cells.inc");
        assert_eq!(titles(&one), ["cell"]);
        assert_eq!(one["files"], whole["files"]);
    }

    #[test]
    fn a_netlist_is_solved_in_memory_and_the_document_and_the_report_come_back() {
        let (_client, server) = pair();
        let text = "R1 a b 1k\nR2 b 0 1k\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n* @RNL { {#R1 #R2} @ALIGN(\"y\") }\n.end\n";
        let last = LastSolve::default();
        let answer = solve(&request("/top.sp", text), &server, &last).unwrap();
        let json: serde_json::Value = serde_json::from_str(&answer).unwrap();
        assert!(json["document"]["levels"].as_array().is_some_and(|l| !l.is_empty()));
        assert!(json["report"]["levels"].as_array().is_some());
        // kept for Save Solved Copy As, as the desktop keeps it
        assert_eq!(last.lock().unwrap().as_ref().map(|c| c.entry.as_str()), Some("/top.sp"));
    }

    #[test]
    fn a_solve_reads_the_library_and_auto_include_as_the_pages_build() {
        let (_client, server) = pair();
        let top = "* top
* @RNL(type=\"header\") { @VERSION(\"0.4\") }
X1 a 0 cell
X2 a 0 part
.include part.inc
.end
";
        let mut r = request("/w/top.sp", top);
        r.files.insert(
            "/<pool>/lib/cell.inc".into(),
            "* cell
.subckt cell a b
R1 a b 1k
.ends
"
            .into(),
        );
        r.files.insert(
            "/P/part.inc".into(),
            "* part
.subckt part a b
R1 a b 1k
.ends
"
            .into(),
        );
        r.pool = vec!["/<pool>/lib/cell.inc".into()];
        r.library_prefix = "/<pool>".into();
        r.search_paths = vec!["/P".into()];
        let read = |r: &Request| {
            let json: serde_json::Value =
                serde_json::from_str(&solve(r, &server, &LastSolve::default()).unwrap()).unwrap();
            let files = json["document"]["files"].as_array().unwrap().clone();
            let has = |name: &str| files.iter().any(|f| f["path"].as_str().unwrap().ends_with(name));
            (has("cell.inc"), has("part.inc"))
        };
        // a bare include from a PATH folder; a subcircuit from the Library with Auto Include, not without
        assert_eq!(read(&r), (false, true));
        r.auto_include = true;
        assert_eq!(read(&r), (true, true));
    }

    #[test]
    fn a_request_without_its_entry_or_with_a_path_that_climbs_is_refused() {
        let (_client, server) = pair();
        let mut r = request("/top.sp", "R1 a b 1k\n.end\n");
        r.entry = "/other.sp".into();
        assert!(solve(&r, &server, &LastSolve::default()).is_err());
        let mut r = request("/a/../top.sp", "R1 a b 1k\n.end\n");
        assert!(solve(&r, &server, &LastSolve::default()).is_err());
        r.entry = "top.sp".into();
        assert!(solve(&r, &server, &LastSolve::default()).is_err());
    }

    #[test]
    fn a_solver_setting_that_is_wrong_is_an_error_in_the_solvers_words() {
        let (_client, server) = pair();
        let mut r = request("/top.sp", "R1 a b 1k\n.end\n");
        r.config = "beam=0".into();
        let error = solve(&r, &server, &LastSolve::default()).unwrap_err();
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
        let writer = std::thread::spawn(move || stream(prepared, &mut server, &LastSolve::default()).unwrap());
        let mut text = String::new();
        client.read_to_string(&mut text).unwrap();
        writer.join().unwrap();
        text.lines().map(|line| serde_json::from_str(line).unwrap()).collect()
    }

    const HIERARCHY: &str = include_str!("../../../tests/conformance/cases-0.4/from-0.2/04-hierarchy.sp");

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
        assert!(solve(&r, &server, &LastSolve::default()).is_ok());
        for bad in [0.0, -3.0, f64::INFINITY] {
            r.timeall = Some(bad);
            let error = solve(&r, &server, &LastSolve::default()).unwrap_err();
            assert!(error.contains("timeall must be a positive number"), "{bad}: {error}");
        }
    }

    #[test]
    fn a_respace_is_a_preview_with_its_mode_and_leaves_a_level_without_a_record() {
        let (_client, server) = pair();
        let mut r = request("/top.sp", HIERARCHY);
        r.config = "spacing_x=3".into();
        r.respace = true;
        let json: serde_json::Value =
            serde_json::from_str(&solve(&r, &server, &LastSolve::default()).unwrap()).unwrap();
        assert_eq!(json["report"]["mode"], "respace");
        // (no level of this netlist has a record of the solver to respace from: each is left as it is, with the reason)
        assert!(
            json["report"]["notes"].as_array().is_some_and(|n| n
                .iter()
                .any(|t| t.as_str().is_some_and(|t| t.contains("not solved") && t.contains("no hash")))),
            "{}",
            json["report"]["notes"]
        );
        // and a solve says it is one
        r.respace = false;
        let json: serde_json::Value =
            serde_json::from_str(&solve(&r, &server, &LastSolve::default()).unwrap()).unwrap();
        assert_eq!(json["report"]["mode"], "solve");
    }

    #[test]
    fn a_level_given_by_its_title_is_the_only_one_solved() {
        let mut r = request("/top.sp", HIERARCHY);
        r.stream = true;
        r.level = Some("amp".into());
        let lines = streamed(&r);
        assert_eq!(lines[0]["levels"], 1);
        let titles: Vec<_> =
            lines.iter().filter(|l| l["event"] == "level").map(|l| l["title"].as_str().unwrap().to_string()).collect();
        // (the same level may be told more than once)
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
        let json: serde_json::Value =
            serde_json::from_str(&solve(&r, &server, &LastSolve::default()).unwrap()).unwrap();
        assert_eq!(json["report"]["levels"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn the_service_says_what_it_is() {
        let i = info();
        assert_eq!(i["service"], "rnl-solver");
        assert!(i["version"].as_str().unwrap().starts_with("rnl "));
    }
}
