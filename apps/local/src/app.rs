//! What the program answers: the web viewer's own files to anyone on the loopback address, and everything that reads or
//! writes the user's files, or solves, only to the page that holds the session (security.rs).
//!
//!   GET  /__start?t=TOKEN        the token, once, for the session cookie; goes on to the page
//!   GET  /api/session            what the page needs to start: the files named on the command line, the home folder
//!   GET  /api/settings           the user's settings;  PUT /api/settings  {key, value}
//!   GET  /api/ls?dir=&hidden=    a folder's entries, for the file chooser
//!   POST /api/open               {path}: a file the user chose or named; its folder is allowed; where it is now
//!   POST /api/allow              {dir}: a folder the user chose to include, allowed
//!   POST /api/save               {entry, files, overwrite, encoding}: the desktop's Save
//!   POST /api/save-as            {source, to, text, encoding}: the desktop's Save As
//!   GET  /api/changes?since=N    the files read that changed on disk since version N
//!   GET  /f/ABSOLUTE/PATH        a file, below the folders allowed
//!   GET  /rnl-solver             what the solver service is;  POST /rnl-solver/solve  a solve (service.rs)

use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use rnl_cli::codec::OutputEncoding;
use serde::Deserialize;
use serde_json::json;

use crate::assets::Site;
use crate::files::{self, Roots, SavedFile};
use crate::http::{encode_path, Request, Response};
use crate::security::{self, Auth};
use crate::settings::Settings;
use crate::watch::Watcher;

/// The most a request may carry: the files of a netlist and its Library, for a solve.
pub const MAX_BODY: usize = 512 * 1024 * 1024;

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
    pub version: String,
    /// The file the browser was told to open, which holds the one-time address: removed once the address is used.
    pub address_file: Mutex<Option<PathBuf>>,
}

impl App {
    pub fn set_address_file(&self, file: Option<PathBuf>) {
        if let Ok(mut held) = self.address_file.lock() {
            *held = file;
        }
    }

    /// Remove the file with the address in it: it is of no use once the address has been, and must not be left lying about.
    pub fn remove_address_file(&self) {
        if let Some(file) = self.address_file.lock().ok().and_then(|mut f| f.take()) {
            let _ = std::fs::remove_file(file);
        }
    }

    /// The seconds since a page last asked what changed; None before the first time.
    pub fn quiet_for(&self) -> Option<u64> {
        self.heartbeat.lock().ok().and_then(|h| *h).map(|t| t.elapsed().as_secs())
    }
}

/// Answer a request. `stream` is the connection it came on, which a long answer watches for the page going away.
pub fn handle(app: &App, request: &Request, stream: &TcpStream) -> Response {
    if !security::host_ok(request.header("host").unwrap_or("")) {
        return Response::error(403, "this address is not the program's own: it is for 127.0.0.1 and localhost");
    }
    let method = request.method.as_str();
    if !matches!(method, "GET" | "HEAD" | "POST" | "PUT") {
        return Response::error(405, "GET, HEAD, POST and PUT only");
    }
    if request.path == "/__start" {
        return start(app, request);
    }
    let protected = request.path.starts_with("/api/") || request.path.starts_with("/f/") || request.path.starts_with("/rnl-solver");
    if !protected {
        return page(app, request);
    }
    if !app.auth.signed_in(request) {
        return Response::error(403, "not signed in: open the address the program printed, which works once");
    }
    if matches!(method, "POST" | "PUT") && !security::from_the_page(request) {
        return Response::error(403, "only the page of this program may change anything");
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
        Some(_) => {
            app.remove_address_file();
            Response::redirect("/?shell=local").header("Set-Cookie", &app.auth.cookie())
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
    // the page without its shell is the web viewer; this program's page is the local one
    if (path.is_empty() || path == "index.html") && request.param("shell").is_none() {
        return Response::redirect("/?shell=local");
    }
    match app.site.get(path) {
        Some(asset) => Response::new(200, asset.content_type, asset.bytes.into_owned()),
        None => Response::text(404, "no such file"),
    }
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
        "/rnl-solver/solve" if method == "POST" => {
            let solve: crate::service::Request = json_body(request)?;
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
        "/api/settings" if get => Ok(Response::json(200, &json!({ "path": app.settings.path().to_string_lossy(), "values": app.settings.values() }))),
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
            let dir = request.param("dir").filter(|d| !d.is_empty()).map(|d| files::machine_path(&d)).or_else(files::home).ok_or_else(|| bad("which folder?"))?;
            let (real, entries) = files::list_dir(&dir, request.param("hidden").as_deref() == Some("1")).map_err(bad)?;
            Ok(Response::json(
                200,
                &json!({ "dir": files::page_path(&real), "parent": real.parent().map(files::page_path), "entries": entries }),
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
            if !path.is_file() {
                return Err(bad(format!("{} is not a file", open.path)));
            }
            app.roots.allow_file(&path).map_err(bad)?;
            let real = app.roots.check(&path, true).map_err(forbidden)?;
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
            app.roots.allow(&dir).map_err(bad)?;
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
            let saved = files::save_all(&app.roots, &save.entry, &save.files, save.overwrite, encoding(&save.encoding)?).map_err(|e| Failure(500, e))?;
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
            }
            let save: SaveAs = json_body(request)?;
            let source = save.source.as_deref().map(files::machine_path);
            let to = files::save_as(&app.roots, source.as_deref(), &files::machine_path(&save.to), &save.text, encoding(&save.encoding)?).map_err(|e| Failure(500, e))?;
            app.watcher.watch(&to);
            Ok(Response::json(200, &json!({ "path": files::page_path(&to) })))
        }
        "/api/changes" if get => {
            if let Ok(mut heartbeat) = app.heartbeat.lock() {
                *heartbeat = Some(Instant::now());
            }
            let since = request.param("since").and_then(|s| s.parse().ok()).unwrap_or(0);
            let (version, changed) = app.watcher.since(since);
            Ok(Response::json(200, &json!({ "version": version, "changed": changed })))
        }
        path if path.starts_with("/f/") && get => file(app, &path[2..]),
        path if path.starts_with("/f/") && method == "PUT" => Err(Failure(405, "files are written by /api/save".into())),
        _ => Err(Failure(404, "no such address".into())),
    }
}

/// A file below the folders allowed, by its absolute path.
fn file(app: &App, path: &str) -> Result<Response, Failure> {
    let real = app.roots.check(&files::machine_path(path), true).map_err(forbidden)?;
    if !real.is_file() {
        return Err(Failure(404, format!("{} is not a file", real.display())));
    }
    let bytes = std::fs::read(&real).map_err(|e| Failure(404, format!("{}: {e}", real.display())))?;
    app.watcher.watch(&real);
    Ok(Response::new(200, "application/octet-stream", bytes).header("X-RNL-Path", &encode_path(&files::page_path(&real))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::read_request;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// An app on a temporary folder, with a netlist in it, and a session.
    fn app(site: Site) -> (App, String, PathBuf) {
        // (a folder in a folder: the folder above the file's is then the app's own, not the temporary folder of the machine)
        let dir = std::env::temp_dir().join(format!("rnl-local-app-{}-{}", std::process::id(), &crate::security::random_hex()[..8])).join("work");
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
            version: "test".into(),
            address_file: Mutex::new(None),
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
        let response = match read_request(&mut server, MAX_BODY) {
            Ok(request) => handle(app, &request, &server),
            Err(crate::http::Rejected(status, why)) => Response::error(status, why),
        };
        crate::http::write_response(&mut server, &response, false).unwrap();
        drop(server);
        let mut text = Vec::new();
        client.read_to_end(&mut text).unwrap();
        let split = text.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&text[..split]).into_owned();
        let mut lines = head.split("\r\n");
        let status = lines.next().unwrap().split(' ').nth(1).unwrap().parse().unwrap();
        let headers = lines.filter_map(|l| l.split_once(": ")).map(|(n, v)| (n.to_ascii_lowercase(), v.to_string())).collect();
        (status, headers, text[split + 4..].to_vec())
    }

    /// The address of a file: `/f`, and its absolute path (a development build on Windows has the drive first).
    fn furl(path: &Path) -> String {
        format!("/f{}", encode_path(&files::page_path(path)))
    }

    fn get(path: &str, cookie: Option<&str>) -> String {
        let cookie = cookie.map(|c| format!("Cookie: {}={c}\n", security::COOKIE)).unwrap_or_default();
        format!("GET {path} HTTP/1.1\nHost: 127.0.0.1:1\n{cookie}")
    }

    fn post(path: &str, cookie: &str, json: &serde_json::Value) -> (String, Vec<u8>) {
        let body = json.to_string().into_bytes();
        let head = format!(
            "POST {path} HTTP/1.1\nHost: 127.0.0.1:1\nCookie: {}={cookie}\nX-RNL: 1\nContent-Type: application/json\nContent-Length: {}\n",
            security::COOKIE,
            body.len()
        );
        (head, body)
    }

    fn sign_in(app: &App, token: &str) -> String {
        let (status, headers, _) = ask(app, &get(&format!("/__start?t={token}"), None), b"");
        assert_eq!(status, 302);
        let cookie = headers.iter().find(|(n, _)| n == "set-cookie").unwrap().1.clone();
        assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"));
        cookie.split(';').next().unwrap().split_once('=').unwrap().1.to_string()
    }

    #[test]
    fn nothing_of_the_users_is_served_without_the_session_and_a_foreign_host_is_refused() {
        let (app, token, dir) = app(Site::Embedded);
        let file = furl(&dir.join("top.sp"));
        for path in [file.as_str(), "/api/session", "/api/ls?dir=/", "/api/settings", "/rnl-solver", "/api/changes"] {
            assert_eq!(ask(&app, &get(path, None), b"").0, 403, "{path}");
            assert_eq!(ask(&app, &get(path, Some("wrong")), b"").0, 403, "{path} with a wrong cookie");
        }
        let cookie = sign_in(&app, &token);
        assert_eq!(ask(&app, &get(&file, Some(&cookie)), b"").0, 200);
        assert_eq!(ask(&app, &get("/api/session", Some(&cookie)), b"").0, 200);
        let foreign = format!("GET /api/session HTTP/1.1\nHost: example.com\nCookie: {}={cookie}\n", security::COOKIE);
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
            "POST /api/open HTTP/1.1\nHost: 127.0.0.1:1\nCookie: {}={cookie}\nContent-Type: application/json\nContent-Length: {}\n",
            security::COOKIE,
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
}
