//! What the program answers: the web viewer's own files to anyone on the loopback address, and everything that reads or
//! writes the user's files, or solves, only to the page that holds the session (security.rs).
//!
//!   GET  /__start?t=TOKEN        the token, once, for the key of the session; goes on to the page, with the key in its fragment
//!   GET  /api/session            what the page needs to start: the files named on the command line, the home folder
//!   GET  /api/settings           the user's settings;  PUT /api/settings  {key, value}
//!   GET  /api/ls?dir=            a folder's entries, for the file chooser
//!   POST /api/open               {path}: a file the user chose or named; its folder is allowed; where it is now
//!   POST /api/allow              {dir}: a folder the user chose to include, allowed
//!   POST /api/save               {entry, files, overwrite, encoding}: the desktop's Save
//!   POST /api/save-as            {source, to, text, encoding, overwrite}: the desktop's Save As; a file that is there is not replaced unless told
//!   GET  /api/changes?since=N    the files read that changed on disk since version N
//!   GET  /f/ABSOLUTE/PATH        a file, below the folders allowed
//!   GET  /rnl-solver             what the solver service is;  GET /rnl-solver/table  the table of its settings;
//!   POST /rnl-solver/check  a check of settings;  POST /rnl-solver/solve  a solve (service.rs)

use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use rnl_cli::codec::OutputEncoding;
use serde::Deserialize;
use serde_json::json;

use crate::assets::Site;
use crate::files::{self, Roots, SavedAs, SavedFile};
use crate::http::{encode_path, Rejected, Request, Response};
use crate::security::{self, Auth};
use crate::settings::Settings;
use crate::watch::Watcher;

/// The most a request that saves or solves may carry: the files of a netlist and its Library.
pub const MAX_BODY: usize = 512 * 1024 * 1024;

/// The most any other request may carry: a path, a setting.
const MAX_SMALL_BODY: usize = 1024 * 1024;

/// The most that the bodies being read at one moment may add up to, so that a few requests together cannot fill the memory of
/// a machine that is shared.
const MAX_BODIES: usize = 1024 * 1024 * 1024;

pub struct App {
    pub site: Site,
    pub auth: Auth,
    pub roots: Roots,
    pub settings: Settings,
    pub watcher: Watcher,
    /// The files named on the command line, as absolute paths, for the page to open.
    pub initial: Vec<PathBuf>,
    /// When the page last asked what changed: it asks every second, so a page that is closed is known by that.
    pub heartbeat: Mutex<Option<Instant>>,
    /// How many files with changes not saved the page said it had, the last time it asked what changed.
    pub unsaved: AtomicUsize,
    pub version: String,
    /// The file the browser was told to open, which holds the one-time address: removed once the address is used.
    pub address_file: Mutex<Option<PathBuf>>,
    /// The folders of this run (the browser's profile): removed when the run ends.
    pub run_dirs: Mutex<Vec<PathBuf>>,
    /// The bytes of the bodies being read now.
    pub bodies: AtomicUsize,
}

impl App {
    pub fn set_address_file(&self, file: Option<PathBuf>) {
        if let Ok(mut held) = self.address_file.lock() {
            *held = file;
        }
    }

    /// Remove the file with the address in it, and the folder it was made in: it is of no use once the address has been, and
    /// must not be left lying about.
    pub fn remove_address_file(&self) {
        if let Some(file) = self.address_file.lock().ok().and_then(|mut f| f.take()) {
            let _ = std::fs::remove_file(&file);
            if let Some(dir) = file.parent() {
                let _ = std::fs::remove_dir(dir);
            }
        }
    }

    /// Everything of this run that is to go with it: the address file, and the browser's profile.
    pub fn clean_up(&self) {
        self.remove_address_file();
        for dir in self.run_dirs.lock().map(|mut d| std::mem::take(&mut *d)).unwrap_or_default() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    /// Whether the page, the last time it asked, had changes that are not saved: they are in the page, and nowhere else.
    pub fn has_unsaved(&self) -> usize {
        self.unsaved.load(Ordering::Relaxed)
    }

    /// The seconds since a page last asked what changed; None before the first time.
    pub fn quiet_for(&self) -> Option<u64> {
        self.heartbeat.lock().ok().and_then(|h| *h).map(|t| t.elapsed().as_secs())
    }
}

/// Whether the request is let in, and how large a body it may have: the program asks this of a request's head, before its body
/// is read, so that what is not let in costs nothing. The page's own files are for anyone who can reach the port with a Host
/// the program knows (they hold nothing of the user's); everything that reads or writes the user's files, or solves, is for
/// the one that holds the key, and only that request has a body (a path, a setting: little; files to save, to solve: much).
pub fn admit(app: &App, request: &Request) -> Result<usize, Response> {
    if !security::host_ok(request.header("host").unwrap_or("")) {
        return Err(Response::error(403, "this address is not the program's own: it is for 127.0.0.1 and localhost"));
    }
    let method = request.method.as_str();
    if !matches!(method, "GET" | "HEAD" | "POST" | "PUT") {
        return Err(Response::error(405, "GET, HEAD, POST and PUT only"));
    }
    if request.path == "/__start" || !is_protected(&request.path) {
        return Ok(0);
    }
    if !app.auth.signed_in(request) {
        return Err(Response::error(403, "not signed in: open the address the program printed, which works once"));
    }
    if matches!(method, "POST" | "PUT") && !security::from_the_page(request) {
        return Err(Response::error(403, "only the page of this program may change anything"));
    }
    Ok(if matches!(request.path.as_str(), "/rnl-solver/solve" | "/api/save" | "/api/save-as") {
        MAX_BODY
    } else {
        MAX_SMALL_BODY
    })
}

/// Whether a path is of what reads or writes the user's files, or solves.
fn is_protected(path: &str) -> bool {
    path.starts_with("/api/") || path.starts_with("/f/") || path.starts_with("/rnl-solver")
}

/// Read one request from the connection and answer it: the head first (and `head_read`, which is told whether it is of the
/// session, and says whether there is a place for it), then, if the request is let in and the bodies being
/// read at this moment do not already add up to too much, the body. Says whether only the head of the answer is to be sent.
pub fn respond(app: &App, stream: &mut TcpStream, head_read: impl FnOnce(bool) -> bool) -> (Response, bool) {
    let rejected = |Rejected(status, why): Rejected| (Response::error(status, why), false);
    let head = match crate::http::read_head(stream) {
        Ok(head) => head,
        Err(r) => return rejected(r),
    };
    let head_only = head.request.method == "HEAD";
    // (the caller counts the request now: it is of the session, or it is not, which has fewer places)
    // (the way in, `/__start`, has the session's places too: it is the user's own address, a request of a few bytes, and what
    // fills the others must not be able to close it)
    if !head_read(app.auth.signed_in(&head.request) || head.request.path == "/__start") {
        return (Response::error(503, "the program is busy with other requests: try again"), head_only);
    }
    let limit = match admit(app, &head.request) {
        Ok(limit) => limit,
        Err(response) => return (response, head_only),
    };
    if head.length > limit {
        return (Response::error(413, "the body is too large"), head_only);
    }
    let reserved = head.length;
    if reserved > 0 && app.bodies.fetch_add(reserved, Ordering::SeqCst) + reserved > MAX_BODIES {
        app.bodies.fetch_sub(reserved, Ordering::SeqCst);
        return (Response::error(503, "the program is busy with other requests: try again"), head_only);
    }
    // (what is reserved is held while the body is in memory: until the answer is made)
    let read = crate::http::read_body(stream, head);
    let answer = match read {
        Ok(request) => {
            // (a solve runs for as long as it takes: nothing is read from the connection but to see that the page is still there)
            let _ = stream.set_read_timeout(None);
            (handle(app, &request, stream), head_only)
        }
        Err(r) => rejected(r),
    };
    if reserved > 0 {
        app.bodies.fetch_sub(reserved, Ordering::SeqCst);
    }
    answer
}

/// Answer a request whose head and body have been read. `stream` is the connection it came on, which a long answer watches
/// for the page going away.
pub fn handle(app: &App, request: &Request, stream: &TcpStream) -> Response {
    if let Err(refused) = admit(app, request) {
        return refused;
    }
    if request.path == "/__start" {
        return start(app, request);
    }
    if !is_protected(&request.path) {
        return page(app, request);
    }
    match route(app, request, stream) {
        Ok(response) => response,
        Err(Failure(status, message)) => Response::error(status, &message),
    }
}

/// A failed request: the status and why, in words the page shows.
struct Failure(u16, String);

fn bad(message: impl Into<String>) -> Failure {
    Failure(400, message.into())
}

fn forbidden(message: impl Into<String>) -> Failure {
    Failure(403, message.into())
}

/// The page's own files, and the way in: the token for the cookie.
fn start(app: &App, request: &Request) -> Response {
    let Some(token) = request.param("t") else {
        return Response::text(403, "this address is for the program's own page: open the address it printed");
    };
    match app.auth.exchange(&token) {
        Some(key) => {
            app.remove_address_file();
            // (the key is in the fragment: no server is sent it, and the page moves it into its own storage and takes it from the address)
            Response::redirect(&format!("/?shell=local#k={key}"))
        }
        None => Response::text(403, "this address works once, and has been used or is wrong: start the program again"),
    }
}

fn page(app: &App, request: &Request) -> Response {
    if !matches!(request.method.as_str(), "GET" | "HEAD") {
        return Response::error(405, "GET only");
    }
    if app.site.is_empty() {
        return Response::text(
            404,
            "this program was built without the web viewer: build it with `node apps/web/build.mjs` and rebuild, or start it with --site DIR",
        );
    }
    let path = request.path.trim_start_matches('/');
    // the page without its shell is the web viewer; this program's page is the local one, and takes nothing else in its
    // address: not `?url=` or `?manifest=`, with which another site could make it open what that site chose, nor `?embed=`
    // or `?api=`, with which the web viewer talks to the page that opened or framed it
    if (path.is_empty() || path == "index.html") && !page_query_ok(&request.query) {
        return Response::redirect("/?shell=local");
    }
    match app.site.get(path) {
        Some(asset) => Response::new(200, asset.content_type, asset.bytes.into_owned()),
        None => Response::text(404, "no such file"),
    }
}

/// Whether the query of the page's address is what the local shell has: `shell=local`, and, for the window that New Window
/// opens, `viewer-session` with the token of that handoff (which only the window that opened it can answer).
fn page_query_ok(query: &str) -> bool {
    let mut shell = false;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("shell", "local")) => shell = true,
            Some(("viewer-session", token))
                if !token.is_empty()
                    && token.len() <= 64
                    && token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') => {}
            _ => return false,
        }
    }
    shell
}

fn json_body<T: for<'de> Deserialize<'de>>(request: &Request) -> Result<T, Failure> {
    if !request.header("content-type").is_some_and(|c| c.to_ascii_lowercase().starts_with("application/json")) {
        return Err(Failure(415, "the body is JSON".into()));
    }
    serde_json::from_slice(&request.body).map_err(|e| bad(format!("the body is not what is asked: {e}")))
}

fn encoding(name: &Option<String>) -> Result<OutputEncoding, Failure> {
    name.as_deref().unwrap_or("preserve").parse().map_err(bad)
}

fn route(app: &App, request: &Request, stream: &TcpStream) -> Result<Response, Failure> {
    let method = request.method.as_str();
    let get = matches!(method, "GET" | "HEAD");
    match request.path.as_str() {
        "/rnl-solver" if get => Ok(Response::json(200, &crate::service::info())),
        "/rnl-solver/table" if get => Ok(Response::json(200, &crate::service::table())),
        "/rnl-solver/check" if method == "POST" => {
            let check: crate::service::Check = json_body(request)?;
            Ok(Response::json(200, &crate::service::check(&check).map_err(bad)?))
        }
        "/rnl-solver/solve" if method == "POST" => {
            let solve: crate::service::Request = json_body(request)?;
            if solve.stream {
                // (a request that is wrong is answered as such, before a head that says it is not: what fails later is a line)
                let prepared = crate::service::prepare(&solve).map_err(bad)?;
                return Ok(Response::streamed("application/x-ndjson; charset=utf-8", move |out| {
                    crate::service::stream(prepared, out)
                }));
            }
            let answer = crate::service::solve(&solve, stream).map_err(bad)?;
            Ok(Response::new(200, "application/json; charset=utf-8", answer.into_bytes()))
        }
        "/api/session" if get => Ok(Response::json(
            200,
            &json!({
                "version": app.version,
                "initial": app.initial.iter().map(|p| files::page_path(p)).collect::<Vec<_>>(),
                "home": files::home().map(|h| files::page_path(&h)),
                "allowed": app.roots.list().iter().map(|p| files::page_path(p)).collect::<Vec<_>>(),
                "settings": app.settings.path().to_string_lossy(),
            }),
        )),
        "/api/settings" if get => Ok(Response::json(
            200,
            &json!({ "path": app.settings.path().to_string_lossy(), "values": app.settings.values() }),
        )),
        "/api/settings" if matches!(method, "PUT" | "POST") => {
            #[derive(Deserialize)]
            struct Set {
                key: String,
                value: serde_json::Value,
            }
            let set: Set = json_body(request)?;
            app.settings.set(set.key, set.value).map_err(|e| Failure(500, e))?;
            Ok(Response::json(200, &json!({})))
        }
        "/api/ls" if get => {
            let dir = request
                .param("dir")
                .filter(|d| !d.is_empty())
                .map(|d| files::machine_path(&d))
                .or_else(files::home)
                .ok_or_else(|| bad("which folder?"))?;
            let (real, entries, truncated) = files::list_dir(&dir).map_err(bad)?;
            Ok(Response::json(
                200,
                &json!({ "dir": files::page_path(&real), "parent": real.parent().map(files::page_path), "entries": entries, "truncated": truncated }),
            ))
        }
        "/api/open" if method == "POST" => {
            #[derive(Deserialize)]
            struct Open {
                path: String,
            }
            let open: Open = json_body(request)?;
            let path = files::machine_path(&open.path);
            if !path.is_absolute() {
                return Err(bad("the path is not absolute"));
            }
            let real = app.roots.open_file(&path).map_err(forbidden)?;
            app.watcher.watch(&real);
            Ok(Response::json(200, &json!({ "path": files::page_path(&real) })))
        }
        "/api/allow" if method == "POST" => {
            #[derive(Deserialize)]
            struct Allow {
                dir: String,
            }
            let allow: Allow = json_body(request)?;
            let dir = files::machine_path(&allow.dir);
            if !dir.is_absolute() {
                return Err(bad("the path is not absolute"));
            }
            app.roots.allow_chosen(&dir).map_err(forbidden)?;
            Ok(Response::json(200, &json!({ "dir": allow.dir })))
        }
        "/api/save" if method == "POST" => {
            #[derive(Deserialize)]
            struct Save {
                entry: String,
                files: Vec<SavedFile>,
                #[serde(default)]
                overwrite: bool,
                #[serde(default)]
                encoding: Option<String>,
            }
            let save: Save = json_body(request)?;
            let saved =
                files::save_all(&app.roots, &save.entry, &save.files, save.overwrite, encoding(&save.encoding)?)
                    .map_err(|e| Failure(500, e))?;
            if saved.said.is_some() {
                for f in &save.files {
                    if let Ok(at) = app.roots.check(&files::located(&save.entry, &f.path), true) {
                        app.watcher.wrote(&at);
                    }
                }
            }
            Ok(Response::json(200, &json!({ "said": saved.said, "changed": saved.changed })))
        }
        "/api/save-as" if method == "POST" => {
            #[derive(Deserialize)]
            struct SaveAs {
                source: Option<String>,
                to: String,
                text: String,
                #[serde(default)]
                encoding: Option<String>,
                #[serde(default)]
                overwrite: bool,
            }
            let save: SaveAs = json_body(request)?;
            let source = save.source.as_deref().map(files::machine_path);
            let to = files::machine_path(&save.to);
            match files::save_as(
                &app.roots,
                source.as_deref(),
                &to,
                &save.text,
                encoding(&save.encoding)?,
                save.overwrite,
            )
            .map_err(|e| Failure(500, e))?
            {
                SavedAs::Written(to) => {
                    app.watcher.watch(&to);
                    app.watcher.wrote(&to);
                    Ok(Response::json(200, &json!({ "path": files::page_path(&to) })))
                }
                // a file of that name is there: the page asks the user, and asks again with `overwrite`
                SavedAs::Exists(to) => Ok(Response::json(200, &json!({ "exists": files::page_path(&to) }))),
            }
        }
        "/api/changes" if get => {
            if let Ok(mut heartbeat) = app.heartbeat.lock() {
                *heartbeat = Some(Instant::now());
            }
            if let Some(unsaved) = request.param("unsaved").and_then(|n| n.parse().ok()) {
                app.unsaved.store(unsaved, Ordering::Relaxed);
            }
            let since = request.param("since").and_then(|s| s.parse().ok()).unwrap_or(0);
            let (version, changed) = app.watcher.since(since);
            Ok(Response::json(200, &json!({ "version": version, "changed": changed })))
        }
        path if path.starts_with("/f/") && get => file(app, &path[2..]),
        path if path.starts_with("/f/") && method == "PUT" => {
            Err(Failure(405, "files are written by /api/save".into()))
        }
        _ => Err(Failure(404, "no such address".into())),
    }
}

/// A file below the folders allowed, by its absolute path.
fn file(app: &App, path: &str) -> Result<Response, Failure> {
    let real = app.roots.check(&files::machine_path(path), true).map_err(forbidden)?;
    let meta = std::fs::metadata(&real).map_err(|e| Failure(404, format!("{}: {e}", real.display())))?;
    if !meta.is_file() {
        return Err(Failure(404, format!("{} is not a file", real.display())));
    }
    if meta.len() > files::MAX_FILE {
        return Err(Failure(413, format!("{} is too large", real.display())));
    }
    let bytes = std::fs::read(&real).map_err(|e| Failure(404, format!("{}: {e}", real.display())))?;
    app.watcher.watch(&real);
    Ok(Response::new(200, "application/octet-stream", bytes)
        .header("X-RNL-Path", &encode_path(&files::page_path(&real))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::Path;

    /// An app on a temporary folder, with a netlist in it, and a session.
    fn app(site: Site) -> (App, String, PathBuf) {
        // (a folder in a folder: the folder above the file's is then the app's own, not the temporary folder of the machine)
        let dir = std::env::temp_dir()
            .join(format!("rnl-local-app-{}-{}", std::process::id(), &crate::security::random_hex()[..8]))
            .join("work");
        std::fs::create_dir_all(&dir).unwrap();
        let top = dir.join("top.sp");
        std::fs::write(&top, "R1 a b 1k\n.end\n").unwrap();
        let (auth, token) = Auth::new();
        let app = App {
            site,
            auth,
            roots: Roots::default(),
            settings: Settings::open(dir.join("settings.json")).unwrap(),
            watcher: Watcher::default(),
            initial: vec![top.clone()],
            heartbeat: Mutex::new(None),
            unsaved: AtomicUsize::new(0),
            version: "test".into(),
            address_file: Mutex::new(None),
            run_dirs: Mutex::new(Vec::new()),
            bodies: AtomicUsize::new(0),
        };
        app.roots.allow_file(&top).unwrap();
        (app, token, dir)
    }

    /// Send raw request text through the real reading and answering, and give the response's status, headers and body.
    fn ask(app: &App, raw: &str, body: &[u8]) -> (u16, Vec<(String, String)>, Vec<u8>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let mut message = raw.replace('\n', "\r\n").into_bytes();
        message.extend_from_slice(b"\r\n");
        message.extend_from_slice(body);
        client.write_all(&message).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        let (response, head_only) = respond(app, &mut server, |_| true);
        crate::http::write_response(&mut server, &response, head_only).unwrap();
        drop(server);
        let mut text = Vec::new();
        client.read_to_end(&mut text).unwrap();
        let split = text.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&text[..split]).into_owned();
        let mut lines = head.split("\r\n");
        let status = lines.next().unwrap().split(' ').nth(1).unwrap().parse().unwrap();
        let headers =
            lines.filter_map(|l| l.split_once(": ")).map(|(n, v)| (n.to_ascii_lowercase(), v.to_string())).collect();
        (status, headers, text[split + 4..].to_vec())
    }

    /// The address of a file: `/f`, and its absolute path (a development build on Windows has the drive first).
    fn furl(path: &Path) -> String {
        format!("/f{}", encode_path(&files::page_path(path)))
    }

    fn get(path: &str, key: Option<&str>) -> String {
        let key = key.map(|k| format!("X-RNL-Key: {k}\n")).unwrap_or_default();
        format!("GET {path} HTTP/1.1\nHost: 127.0.0.1:1\n{key}")
    }

    fn post(path: &str, cookie: &str, json: &serde_json::Value) -> (String, Vec<u8>) {
        let body = json.to_string().into_bytes();
        let head = format!(
            "POST {path} HTTP/1.1\nHost: 127.0.0.1:1\nX-RNL-Key: {cookie}\nX-RNL: 1\nContent-Type: application/json\nContent-Length: {}\n",
            body.len()
        );
        (head, body)
    }

    fn sign_in(app: &App, token: &str) -> String {
        let (status, headers, _) = ask(app, &get(&format!("/__start?t={token}"), None), b"");
        assert_eq!(status, 302);
        // no cookie, which a browser sends to every port of the address: the key is in the fragment of the page's address
        assert!(!headers.iter().any(|(n, _)| n == "set-cookie"));
        let location = headers.iter().find(|(n, _)| n == "location").unwrap().1.clone();
        location.strip_prefix("/?shell=local#k=").expect(&location).to_string()
    }

    #[test]
    fn nothing_of_the_users_is_served_without_the_session_and_a_foreign_host_is_refused() {
        let (app, token, dir) = app(Site::Embedded);
        let file = furl(&dir.join("top.sp"));
        for path in [file.as_str(), "/api/session", "/api/ls?dir=/", "/api/settings", "/rnl-solver", "/api/changes"] {
            assert_eq!(ask(&app, &get(path, None), b"").0, 403, "{path}");
            assert_eq!(ask(&app, &get(path, Some("wrong")), b"").0, 403, "{path} with a wrong key");
        }
        let cookie = sign_in(&app, &token);
        assert_eq!(ask(&app, &get(&file, Some(&cookie)), b"").0, 200);
        assert_eq!(ask(&app, &get("/api/session", Some(&cookie)), b"").0, 200);
        let foreign = format!("GET /api/session HTTP/1.1\nHost: example.com\nX-RNL-Key: {cookie}\n");
        assert_eq!(ask(&app, &foreign, b"").0, 403);
        // the token worked once
        assert_eq!(ask(&app, &get(&format!("/__start?t={token}"), None), b"").0, 403);
    }

    #[test]
    fn a_file_is_served_only_below_the_folders_allowed_and_a_change_needs_the_page() {
        let (app, token, dir) = app(Site::Embedded);
        let outside = std::env::temp_dir().join(format!("rnl-local-outside-{}.txt", std::process::id()));
        std::fs::write(&outside, "x").unwrap();
        let cookie = sign_in(&app, &token);
        let inside = furl(&dir.join("top.sp"));
        let (status, headers, body) = ask(&app, &get(&inside, Some(&cookie)), b"");
        assert_eq!((status, body.as_slice()), (200, b"R1 a b 1k\n.end\n".as_slice()));
        assert!(headers.iter().any(|(n, _)| n == "x-rnl-path"));
        assert_eq!(ask(&app, &get(&furl(&outside), Some(&cookie)), b"").0, 403);
        assert_eq!(ask(&app, &get(&furl(&dir.join("../x")), Some(&cookie)), b"").0, 403);
        // a post without the header of the page is refused, though the cookie is right
        let body = json!({ "path": files::page_path(&outside) }).to_string();
        let raw = format!(
            "POST /api/open HTTP/1.1\nHost: 127.0.0.1:1\nX-RNL-Key: {cookie}\nContent-Type: application/json\nContent-Length: {}\n",
            body.len()
        );
        assert_eq!(ask(&app, &raw, body.as_bytes()).0, 403);
    }

    #[test]
    fn open_allows_the_folder_of_the_file_and_save_writes_it_with_the_desktops_checks() {
        let (app, token, dir) = app(Site::Embedded);
        let cookie = sign_in(&app, &token);
        let other = dir.join("lib");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("m.inc"), "* m\n").unwrap();
        let (head, body) = post("/api/open", &cookie, &json!({ "path": files::page_path(&other.join("m.inc")) }));
        let (status, _, answer) = ask(&app, &head, &body);
        assert_eq!(status, 200, "{}", String::from_utf8_lossy(&answer));
        // a save with a file that is not what was read asks; with overwrite it writes
        let top = dir.join("top.sp");
        let save = |overwrite: bool| {
            json!({ "entry": files::page_path(&top), "overwrite": overwrite,
                "files": [{ "path": "top.sp", "name": "top.sp", "text": "R1 a b 2k\n.end\n", "read": "R1 a b 9k\n.end\n" }] })
        };
        let (head, body) = post("/api/save", &cookie, &save(false));
        let (status, _, answer) = ask(&app, &head, &body);
        let said: serde_json::Value = serde_json::from_slice(&answer).unwrap();
        assert_eq!((status, said["changed"].clone()), (200, json!(["top.sp"])));
        assert_eq!(std::fs::read_to_string(&top).unwrap(), "R1 a b 1k\n.end\n");
        let (head, body) = post("/api/save", &cookie, &save(true));
        let (status, _, answer) = ask(&app, &head, &body);
        let said: serde_json::Value = serde_json::from_slice(&answer).unwrap();
        assert_eq!((status, said["said"].clone()), (200, json!("Saved top.sp.")));
        assert_eq!(std::fs::read_to_string(&top).unwrap(), "R1 a b 2k\n.end\n");
        // the program's own write is not a change of the file under the page
        app.watcher.poll();
        assert_eq!(app.watcher.since(0).1, Vec::<String>::new());
    }

    #[test]
    fn the_page_is_served_with_its_types_and_the_way_in_is_the_local_shell() {
        let dir = std::env::temp_dir().join(format!("rnl-local-site-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("core")).unwrap();
        std::fs::write(dir.join("index.html"), "<html></html>").unwrap();
        std::fs::write(dir.join("core/a.wasm"), [0, 97, 115, 109]).unwrap();
        let (app, _token, _dir) = app(Site::Directory(dir));
        let (status, headers, _) = ask(&app, &get("/core/a.wasm", None), b"");
        assert_eq!(status, 200);
        assert!(headers.iter().any(|(n, v)| n == "content-type" && v == "application/wasm"));
        // the page of the web viewer, with no shell named, is the local one
        let (status, headers, _) = ask(&app, &get("/", None), b"");
        assert_eq!(status, 302);
        assert!(headers.iter().any(|(n, v)| n == "location" && v == "/?shell=local"));
        assert_eq!(ask(&app, &get("/?shell=local", None), b"").0, 200);
        // the page takes nothing else in its address: what another site may put there does not reach it
        for query in [
            "shell=local&url=https://evil.example/x.sp",
            "shell=local&manifest=https://evil.example/m.json",
            "embed=1&shell=local",
            "shell=local&api=1&origin=*",
            "url=x",
        ] {
            let (status, headers, _) = ask(&app, &get(&format!("/?{query}"), None), b"");
            assert_eq!(status, 302, "{query}");
            assert!(headers.iter().any(|(n, v)| n == "location" && v == "/?shell=local"), "{query}");
        }
        // (New Window opens the page with the token of its handoff)
        assert_eq!(ask(&app, &get("/?shell=local&viewer-session=0a1b-2c3d", None), b"").0, 200);
        assert_eq!(ask(&app, &get("/index.html?shell=local&url=https://evil.example/x.sp", None), b"").0, 302);
        // and no page of another site frames it
        let (_, headers, _) = ask(&app, &get("/?shell=local", None), b"");
        assert!(headers.iter().any(|(n, v)| n == "content-security-policy" && v.contains("frame-ancestors 'none'")));
        assert!(headers.iter().any(|(n, v)| n == "x-frame-options" && v == "DENY"));
        assert_eq!(ask(&app, &get("/no-such-file", None), b"").0, 404);
        assert_eq!(ask(&app, &get("/../etc/passwd", None), b"").0, 404);
    }

    #[test]
    fn a_request_that_is_not_plain_http_is_rejected_whole() {
        let (app, _t, _d) = app(Site::Embedded);
        // two Hosts, a chunked body, a folded header, a method that is not taken
        assert_eq!(ask(&app, "GET / HTTP/1.1\nHost: 127.0.0.1:1\nHost: 127.0.0.1:1\n", b"").0, 400);
        assert_eq!(ask(&app, "POST /api/open HTTP/1.1\nHost: 127.0.0.1:1\nTransfer-Encoding: chunked\n", b"").0, 501);
        assert_eq!(ask(&app, "GET / HTTP/1.1\nHost: 127.0.0.1:1\nX-A: b\n c\n", b"").0, 400);
        assert_eq!(ask(&app, "DELETE /api/session HTTP/1.1\nHost: 127.0.0.1:1\n", b"").0, 405);
        assert_eq!(ask(&app, "GET / HTTP/1.1\nHost: 127.0.0.1:1\nContent-Length: 99999999999\n", b"").0, 413);
    }

    #[test]
    fn a_cookie_is_no_key_and_a_wrong_token_does_not_spend_the_right_one() {
        let (app, token, _dir) = app(Site::Embedded);
        // a page of another port of the address has any cookie the browser sends it: that opens nothing
        let key = app.auth.exchange("not-it");
        assert!(key.is_none());
        let with_cookie = "GET /api/session HTTP/1.1\nHost: 127.0.0.1:1\nCookie: rnl_session=anything\n";
        assert_eq!(ask(&app, with_cookie, b"").0, 403);
        // a wrong token is refused, and the user's own address still works
        assert_eq!(ask(&app, &get("/__start?t=wrong", None), b"").0, 403);
        assert_eq!(ask(&app, &get("/__start", None), b"").0, 403);
        let key = sign_in(&app, &token);
        assert_eq!(key.len(), 64);
        assert_eq!(ask(&app, &get("/api/session", Some(&key)), b"").0, 200);
    }

    #[test]
    fn a_body_is_not_read_for_a_request_that_is_not_let_in_and_is_small_where_it_is_small() {
        let (app, token, _dir) = app(Site::Embedded);
        // 400 MB that are never sent: with no key the answer comes from the head, and the body is not waited for
        let big = "POST /api/save HTTP/1.1\nHost: 127.0.0.1:1\nX-RNL: 1\nContent-Type: application/json\nContent-Length: 400000000\n";
        assert_eq!(ask(&app, big, b"").0, 403);
        assert_eq!(ask(&app, "POST /api/open HTTP/1.1\nHost: 127.0.0.1:1\nContent-Length: 5\n", b"").0, 403);
        // a page's own file takes no body at all
        assert_eq!(ask(&app, "GET /index.html HTTP/1.1\nHost: 127.0.0.1:1\nContent-Length: 5\n", b"").0, 413);
        // with the key, a request that is of a path or a setting takes 1 MiB, not the 512 MiB of a save
        let key = sign_in(&app, &token);
        let head = |path: &str, length: usize| {
            format!("POST {path} HTTP/1.1\nHost: 127.0.0.1:1\nX-RNL-Key: {key}\nX-RNL: 1\nContent-Type: application/json\nContent-Length: {length}\n")
        };
        assert_eq!(ask(&app, &head("/api/open", 2 * 1024 * 1024), b"").0, 413);
        assert_eq!(ask(&app, &head("/api/save", MAX_BODY + 1), b"").0, 413);
        assert_eq!(app.bodies.load(Ordering::SeqCst), 0, "nothing stays reserved");
    }

    #[test]
    fn the_page_cannot_open_or_include_what_is_hidden_or_too_broad_or_list_hidden_names() {
        let (app, token, dir) = app(Site::Embedded);
        let key = sign_in(&app, &token);
        std::fs::create_dir_all(dir.join(".ssh")).unwrap();
        std::fs::write(dir.join(".ssh/id_rsa"), "secret").unwrap();
        std::fs::write(dir.join(".hidden.sp"), "x").unwrap();
        let call = |path: &str, body: serde_json::Value| {
            let (head, body) = post(path, &key, &body);
            ask(&app, &head, &body).0
        };
        assert_eq!(call("/api/open", json!({ "path": files::page_path(&dir.join(".ssh/id_rsa")) })), 403);
        assert_eq!(call("/api/open", json!({ "path": files::page_path(&dir.join(".hidden.sp")) })), 403);
        assert_eq!(call("/api/allow", json!({ "dir": files::page_path(&dir.join(".ssh")) })), 403);
        // the folder is not allowed by that either: the file is not served
        assert_eq!(ask(&app, &get(&furl(&dir.join(".ssh/id_rsa")), Some(&key)), b"").0, 403);
        // the user's home folder, and the root, are too much to give
        let home = files::home().expect("a home folder");
        assert_eq!(call("/api/allow", json!({ "dir": files::page_path(&home) })), 403);
        let root = dir.ancestors().last().unwrap().to_path_buf();
        assert_eq!(call("/api/allow", json!({ "dir": files::page_path(&root) })), 403);
        // a folder of a project is not
        let project = dir.join("project");
        std::fs::create_dir_all(&project).unwrap();
        assert_eq!(call("/api/allow", json!({ "dir": files::page_path(&project) })), 200);
        // the chooser lists names, never hidden ones, whatever it is asked
        std::fs::write(dir.join("shown.sp"), "x").unwrap();
        let (status, _, body) =
            ask(&app, &get(&format!("/api/ls?dir={}&hidden=1", encode_path(&files::page_path(&dir))), Some(&key)), b"");
        assert_eq!(status, 200);
        let listing = String::from_utf8_lossy(&body).into_owned();
        assert!(
            listing.contains("shown.sp") && !listing.contains(".ssh") && !listing.contains(".hidden.sp"),
            "{listing}"
        );
        assert!(
            listing.contains("\"truncated\":false"),
            "a listing says whether it is the whole of the folder: {listing}"
        );
        // the page says each time it asks what changed how many files it has with changes not saved: the program does not stop while it has
        assert_eq!(app.has_unsaved(), 0);
        assert_eq!(ask(&app, &get("/api/changes?since=0&unsaved=3", Some(&key)), b"").0, 200);
        assert_eq!(app.has_unsaved(), 3);
        assert_eq!(ask(&app, &get("/api/changes?since=0&unsaved=0", Some(&key)), b"").0, 200);
        assert_eq!(app.has_unsaved(), 0);
    }

    #[test]
    fn save_as_asks_before_replacing_a_file_and_never_writes_through_a_link() {
        let (app, token, dir) = app(Site::Embedded);
        let key = sign_in(&app, &token);
        let call = |overwrite: bool, to: &Path| {
            let (head, body) = post(
                "/api/save-as",
                &key,
                &json!({ "source": files::page_path(&dir.join("top.sp")), "to": files::page_path(to), "text": "R1 a b 7k\n", "encoding": "utf8", "overwrite": overwrite }),
            );
            let (status, _, answer) = ask(&app, &head, &body);
            (status, serde_json::from_slice::<serde_json::Value>(&answer).unwrap())
        };
        let new = dir.join("copy.sp");
        let (status, said) = call(false, &new);
        assert_eq!((status, said["path"].is_string()), (200, true));
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "R1 a b 7k\n");
        // the name is taken: nothing is written until the user says
        std::fs::write(&new, "mine\n").unwrap();
        let (status, said) = call(false, &new);
        assert_eq!((status, said["exists"].is_string()), (200, true));
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "mine\n");
        let (status, said) = call(true, &new);
        assert_eq!((status, said["path"].is_string()), (200, true));
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "R1 a b 7k\n");
        // a link at the name is not followed to what it points to, with or without the user's word
        #[cfg(unix)]
        {
            let target = dir.join("target.sp");
            std::fs::write(&target, "precious\n").unwrap();
            std::os::unix::fs::symlink(&target, dir.join("link.sp")).unwrap();
            for overwrite in [false, true] {
                let (status, _) = call(overwrite, &dir.join("link.sp"));
                assert_eq!(status, 500);
                assert_eq!(std::fs::read_to_string(&target).unwrap(), "precious\n");
            }
        }
    }
}
