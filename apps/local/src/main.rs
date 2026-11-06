//! RNL Viewer for older enterprise Linux (CentOS 7 / RHEL 8 / RHEL 9): the web viewer, served on the loopback address by one
//! program that has no libraries to find on the machine (a static build), and shown by a browser. It needs no root, no network
//! and no desktop libraries; it solves with the core's own solver here, writes the files the user saves where they were read
//! from, and tells the user when a file changed on disk. Over `ssh -L` the browser may be on another computer.
//!
//!   rnl-viewer-local [OPTIONS] [FILE]...
//!
//!   --port N          the port to listen on (default: one the system gives); the same on both sides of `ssh -L`
//!   --no-browser      show no browser: print the address, which works once, to open in one
//!   --browser PATH    the browser to use, a name on the PATH or a path ($RNL_BROWSER does too)
//!   --allow FOLDER    let the page read and write below FOLDER too (the folder of each file opened, and the one above it,
//!                     are allowed already); may be repeated
//!   --site DIR        serve the web viewer from DIR instead of the copy in the program
//!   --settings FILE   keep the settings in FILE
//!   --version, --help

mod app;
mod assets;
mod files;
mod http;
mod launch;
mod security;
mod service;
mod settings;
mod watch;

use std::cell::Cell;
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use app::App;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How many requests are served at once, once their heads are in: the page asks for its files in parallel.
const MAX_ACTIVE: usize = 96;

/// How many requests that are not of the session (the page's own files, which anyone who can reach the port may have) are
/// served at once: fewer than `MAX_ACTIVE`, so that places are left for the page that holds the key, whatever the others do
/// (a client that does not read its answer holds its place until the write times out).
const MAX_ANONYMOUS: usize = 40;

/// How many connections may be waiting for their request's head at once. When another comes, the one that has waited longest is
/// closed: a client that sends a byte at a time cannot keep the places for itself, and the page, whose head comes in the first
/// moment, always has one, so that the program stays the user's to use, and does not stop for want of the page's asking.
const MAX_WAITING: usize = 48;

/// The connections that are waiting for the head of a request, and the requests being served.
#[derive(Default)]
struct Gate {
    waiting: Mutex<Vec<(u64, TcpStream)>>,
    next: AtomicU64,
    active: AtomicUsize,
    anonymous: AtomicUsize,
}

impl Gate {
    /// A connection comes: it waits for its head, in a place that is its own until the head has come or another has taken it.
    fn enter(&self, stream: &TcpStream) -> Option<u64> {
        let kept = stream.try_clone().ok()?;
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let mut waiting = self.waiting.lock().ok()?;
        if waiting.len() >= MAX_WAITING {
            let (_, oldest) = waiting.remove(0);
            let _ = oldest.shutdown(Shutdown::Both);
        }
        waiting.push((id, kept));
        Some(id)
    }

    fn leave(&self, id: u64) {
        if let Ok(mut waiting) = self.waiting.lock() {
            waiting.retain(|(i, _)| *i != id);
        }
    }
}

/// The page asks what changed every second, but a browser slows the timers of a tab it does not show to one a minute: a page
/// that has not asked for this many seconds is gone.
const GONE_AFTER: u64 = 180;

#[derive(Debug)]
struct Options {
    port: u16,
    browser: bool,
    named_browser: Option<String>,
    allow: Vec<PathBuf>,
    site: Option<PathBuf>,
    settings: Option<PathBuf>,
    files: Vec<PathBuf>,
}

const HELP: &str = "\
RNL Viewer, local: the web viewer served on this computer, shown by a browser

usage: rnl-viewer-local [OPTIONS] [FILE]...

  --port N          the port to listen on (default: one the system gives); use the same on both sides of ssh -L
  --no-browser      show no browser: print the address, which works once, to open in one
  --browser PATH    the browser to use: a name on the PATH or a path ($RNL_BROWSER does too)
  --allow FOLDER    let the page read and write below FOLDER too; may be repeated. The folder of each file
                    opened, and the one above it, are allowed already
  --site DIR        serve the web viewer from DIR instead of the copy in the program
  --settings FILE   keep the settings in FILE (default: ~/.config/rnl-viewer-local/settings.json)
  --version, --help
";

fn parse(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut o = Options { port: 0, browser: true, named_browser: std::env::var("RNL_BROWSER").ok().filter(|b| !b.is_empty()), allow: Vec::new(), site: None, settings: None, files: Vec::new() };
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--port" => o.port = value("--port")?.parse().map_err(|_| "--port takes a number from 0 to 65535".to_string())?,
            "--no-browser" => o.browser = false,
            "--browser" => o.named_browser = Some(value("--browser")?),
            "--allow" => o.allow.push(PathBuf::from(value("--allow")?)),
            "--site" => o.site = Some(PathBuf::from(value("--site")?)),
            "--settings" => o.settings = Some(PathBuf::from(value("--settings")?)),
            "--help" | "-h" => return Err(String::new()),
            "--version" | "-V" => {
                println!("rnl-viewer-local {VERSION} (core {} {})", rnl_cli::CORE_VERSION, rnl_cli::BUILD_COMMIT);
                std::process::exit(0);
            }
            flag if flag.starts_with("--") => return Err(format!("{flag} is not an option (--help lists them)")),
            file => o.files.push(PathBuf::from(file)),
        }
    }
    Ok(o)
}

fn main() {
    let options = match parse(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(message) => {
            if message.is_empty() {
                print!("{HELP}");
                return;
            }
            eprintln!("rnl-viewer-local: {message}");
            std::process::exit(2);
        }
    };
    if let Err(message) = run(options) {
        eprintln!("rnl-viewer-local: {message}");
        std::process::exit(1);
    }
}

fn run(options: Options) -> Result<(), String> {
    let site = match &options.site {
        Some(dir) => assets::Site::Directory(dir.clone()),
        None => assets::Site::Embedded,
    };
    if site.is_empty() {
        return Err("this program has no web viewer in it: build it with `node apps/web/build.mjs` and rebuild, or give --site DIR".into());
    }
    let settings = settings::Settings::open(options.settings.clone().unwrap_or_else(settings::default_path))?;
    let roots = files::Roots::default();
    for dir in &options.allow {
        roots.allow(dir)?;
    }
    let mut initial = Vec::new();
    for file in &options.files {
        let absolute = std::path::absolute(file).map_err(|e| format!("{}: {e}", file.display()))?;
        if !absolute.is_file() {
            return Err(format!("{} is not a file", file.display()));
        }
        roots.allow_file(&absolute)?;
        initial.push(files::canonical(&absolute).unwrap_or(absolute));
    }

    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, options.port))).map_err(|e| format!("cannot listen on 127.0.0.1:{}: {e}", options.port))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let (auth, token) = security::Auth::new();
    let app = Arc::new(App {
        site,
        auth,
        roots,
        settings,
        watcher: watch::Watcher::default(),
        initial,
        heartbeat: Mutex::new(None),
        version: format!("{VERSION} (core {} {})", rnl_cli::CORE_VERSION, rnl_cli::BUILD_COMMIT),
        address_file: Mutex::new(None),
        run_dirs: Mutex::new(Vec::new()),
        bodies: AtomicUsize::new(0),
    });
    for file in &app.initial {
        app.watcher.watch(file);
    }

    println!("RNL Viewer, local {VERSION}: serving on http://127.0.0.1:{port}/ (this computer only)");
    let address = format!("http://127.0.0.1:{port}/__start?t={token}");
    let mut browser_child = None;
    let mut address_file = None;
    if options.browser {
        let beside = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from));
        match launch::find(options.named_browser.as_deref(), beside.as_deref()) {
            Ok(browser) => {
                if !launch::acceptable(browser.kind, browser.major) {
                    eprintln!("{}", launch::too_old(&browser));
                    std::process::exit(1);
                }
                let profile = launch::profile_dir()?;
                let file = match launch::address_file(&address) {
                    Ok(file) => file,
                    Err(message) => {
                        let _ = std::fs::remove_dir_all(&profile);
                        return Err(message);
                    }
                };
                // (a profile for this run alone: the one of an earlier run may still be locked)
                if let Ok(mut dirs) = app.run_dirs.lock() {
                    dirs.push(profile.clone());
                }
                match launch::open(&browser, &launch::file_url(&file), &profile) {
                    Ok(child) => {
                        println!("Opened in {} {}.", browser.path.display(), browser.major.map_or(String::new(), |v| v.to_string()));
                        browser_child = Some(child);
                        address_file = Some(file);
                    }
                    Err(message) => {
                        app.set_address_file(Some(file));
                        app.clean_up();
                        return Err(message);
                    }
                }
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("No browser was started; the address to open one with is below.");
                print_address(&address, port);
            }
        }
    } else {
        print_address(&address, port);
    }
    app.set_address_file(address_file);

    // the watcher: the files the page has read, once a second
    {
        let app = app.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            app.watcher.poll();
        });
    }
    // a browser opened by this program, and then closed: so is the program. The browser's own process ends with its window,
    // unless the program that was started only started another and ended (a wrapper script): then the page's silence says so.
    if let Some(mut child) = browser_child {
        let app = app.clone();
        let started = std::time::Instant::now();
        std::thread::spawn(move || {
            let mut watching = true;
            loop {
                std::thread::sleep(Duration::from_secs(2));
                if watching && child.try_wait().is_ok_and(|status| status.is_some()) {
                    if started.elapsed() > Duration::from_secs(5) {
                        println!("The browser was closed: stopping.");
                        app.clean_up();
                        std::process::exit(0);
                    }
                    watching = false;
                }
                if app.quiet_for().is_some_and(|s| s > GONE_AFTER) {
                    println!("The page was closed: stopping.");
                    app.clean_up();
                    std::process::exit(0);
                }
                // (the browser that was started ended at once, and no page came: it was not shown, and nobody is to wait on it)
                if app.quiet_for().is_none() && started.elapsed() > Duration::from_secs(GONE_AFTER) {
                    eprintln!("The page was never opened: the browser did not show it. Stopping; start the program again, or with --no-browser.");
                    app.clean_up();
                    std::process::exit(1);
                }
            }
        });
    }

    let gate = Arc::new(Gate::default());
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let (app, gate) = (app.clone(), gate.clone());
        std::thread::spawn(move || serve(&app, &gate, stream));
    }
    Ok(())
}

fn print_address(address: &str, port: u16) {
    println!("Open this address in a browser on this computer (it works once):\n  {address}");
    println!("To use a browser on another computer, start the program with --port {port} (or any port) on both sides of\n  ssh -L {port}:127.0.0.1:{port} THIS-HOST\nand open the address in that computer's browser.");
}

/// One connection: one request, one answer. The head of the request has `http::HEAD_TIME` to come, in a place of the gate that
/// is taken from it if too many wait; once it is in, the request is one of at most `MAX_ACTIVE`.
fn serve(app: &App, gate: &Gate, mut stream: TcpStream) {
    let _ = stream.set_write_timeout(Some(Duration::from_secs(120)));
    let waiting = gate.enter(&stream);
    // (which count it is in: what the session's page asks is not kept waiting by what is not of it)
    let counted: Cell<Option<&AtomicUsize>> = Cell::new(None);
    let (response, head_only) = app::respond(app, &mut stream, |signed| {
        if let Some(id) = waiting {
            gate.leave(id);
        }
        let (count, most) = if signed { (&gate.active, MAX_ACTIVE) } else { (&gate.anonymous, MAX_ANONYMOUS) };
        if count.fetch_add(1, Ordering::SeqCst) >= most {
            count.fetch_sub(1, Ordering::SeqCst);
            return false;
        }
        counted.set(Some(count));
        true
    });
    if let Some(id) = waiting {
        gate.leave(id);
    }
    let _ = http::write_response(&mut stream, &response, head_only);
    if let Some(count) = counted.get() {
        count.fetch_sub(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Options, String> {
        parse(list.iter().map(|s| s.to_string()))
    }

    #[test]
    fn when_too_many_wait_for_their_head_the_one_that_waited_longest_is_closed() {
        use std::io::Read;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let gate = Gate::default();
        let mut clients = Vec::new();
        let mut servers = Vec::new();
        for _ in 0..MAX_WAITING + 1 {
            clients.push(TcpStream::connect(listener.local_addr().unwrap()).unwrap());
            let (server, _) = listener.accept().unwrap();
            gate.enter(&server).unwrap();
            servers.push(server);
        }
        assert_eq!(gate.waiting.lock().unwrap().len(), MAX_WAITING);
        // the first was closed to make room for the last: it reads the end of the stream, the others still wait
        servers[0].set_read_timeout(Some(Duration::from_millis(500))).unwrap();
        assert_eq!(servers[0].read(&mut [0u8; 1]).unwrap_or(0), 0);
        let last = servers.last_mut().unwrap();
        last.set_read_timeout(Some(Duration::from_millis(100))).unwrap();
        assert!(last.read(&mut [0u8; 1]).is_err(), "a place that was given is not closed");
        // a connection that has its head leaves, and its place is free again
        let id = gate.enter(&servers[1]).unwrap();
        gate.leave(id);
        assert!(gate.waiting.lock().unwrap().iter().all(|(i, _)| *i != id));
    }

    #[test]
    fn the_command_line_names_files_and_options() {
        let o = args(&["--port", "8765", "--no-browser", "--allow", "/data", "a.sp", "b.sp", "--site", "/s"]).unwrap();
        assert_eq!((o.port, o.browser), (8765, false));
        assert_eq!(o.allow, vec![PathBuf::from("/data")]);
        assert_eq!(o.files, vec![PathBuf::from("a.sp"), PathBuf::from("b.sp")]);
        assert_eq!(o.site, Some(PathBuf::from("/s")));
        assert!(args(&["--port"]).is_err());
        assert!(args(&["--port", "x"]).is_err());
        assert!(args(&["--port", "70000"]).is_err());
        assert!(args(&["--nope"]).unwrap_err().contains("not an option"));
        assert_eq!(args(&["--help"]).err(), Some(String::new()));
    }
}
