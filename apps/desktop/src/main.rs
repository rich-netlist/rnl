//! RNL Viewer: open a netlist, draw its levels, show its text and diagnostics, and follow an
//! instance or a net between the drawing and the text. It can edit the text, build it again from the
//! texts in memory, and write the files the user changed when the user saves them (decision 0011);
//! it can solve the netlist as a preview, and save a solved copy where the user says (decision
//! 0010). It writes no file otherwise, and as opened, objects without a position are not drawn.

// Viewer is a GUI application, including local fast/debug builds. The CLI keeps its console.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod core_thread;
mod files;
mod library;
mod load_progress;
mod menu;
mod pool;
mod session;
mod settings;
mod solve;
mod startup;

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Mutex;

use core_thread::Core;
use files::{Edits, Saved, SavedFile};
use rnl_cli::FsProvider;
use rnl_view::{BuildOptions, Document, UnplacedDrawing};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow, WindowEvent};
use tauri_plugin_dialog::DialogExt;

/// The name of the document shown, and whether its files have changes that are not saved: for the
/// window's title, and for asking before the window closes (decision 0011).
#[derive(Default)]
struct Unsaved(Mutex<(String, bool)>);

impl Unsaved {
    /// Change the name or the flag, and name the window by them: `● name - RNL Viewer` while changes are not saved.
    fn set(&self, window: &WebviewWindow, change: impl FnOnce(&mut (String, bool))) {
        if let Ok(mut state) = self.0.lock() {
            change(&mut state);
            let mark = if state.1 { "\u{25cf} " } else { "" };
            let _ = window.set_title(&format!("{mark}{} - RNL Viewer", state.0));
        }
    }

    fn unsaved(&self) -> bool {
        self.0.lock().map(|state| state.1).unwrap_or(false)
    }
}

/// Close the app, or, with changes not saved, ask the page, which asks the user: it quits when they say.
fn request_exit(app: &AppHandle) {
    if app.state::<Unsaved>().unsaved() {
        let _ = app.emit_to("main", "close-requested", ());
    } else {
        app.exit(0);
    }
}

/// The switch that opens the WebView's DevTools port on 127.0.0.1, as in Chromium and Electron:
/// `--remote-debugging-port=PORT`, through which the smoke test drives the page. The app gives it to
/// WebView2 itself, which takes it from the app also where the app runs elevated, as on GitHub's
/// runners, and ignores WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS there.
const DEVTOOLS_SWITCH: &str = "--remote-debugging-port=";

/// What wry gives WebView2 when the app gives it nothing, which arguments of the app's own must
/// repeat: no mini menu, PDF menu nor SmartScreen, and media that may play by itself.
const WEBVIEW_ARGS: &str =
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required";

/// The port of the DevTools switch among the command line's arguments, if any.
fn devtools_port(args: impl IntoIterator<Item = OsString>) -> Option<u16> {
    args.into_iter().find_map(|a| a.to_str()?.strip_prefix(DEVTOOLS_SWITCH)?.parse().ok())
}

/// The file among the command line's arguments: the first that is not the DevTools switch.
fn file_argument(args: impl IntoIterator<Item = OsString>) -> Option<OsString> {
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--session" {
            args.next();
            continue;
        }
        if !arg.to_string_lossy().starts_with(DEVTOOLS_SWITCH) {
            return Some(arg);
        }
    }
    None
}

/// The extensions offered first in the open dialog.
const NETLISTS: [&str; 11] = ["sp", "spi", "spice", "cir", "ckt", "net", "netlist", "inc", "lib", "scs", "rnl"];

/// Read and draw a file, and name the window after it. With `edits`, the texts of some of its
/// files are those, over what is on disk. The drawings of the objects it lists without a position
/// are kept for `unplaced_drawings`, in place of those of the last document.
#[tauri::command]
// Tauri injects the window and managed state alongside the separate IPC arguments.
#[allow(clippy::too_many_arguments)]
async fn open_document(
    window: WebviewWindow,
    unsaved: State<'_, Unsaved>,
    core: State<'_, Core>,
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    auto_include: Option<bool>,
    pool: Option<Vec<String>>,
    path: String,
    edits: Option<Edits>,
    request_id: Option<String>,
) -> Result<Document, String> {
    let core = core.inner().clone();
    let paths = settings.paths()?;
    let library = library.snapshot()?;
    let pool = pool.unwrap_or_default();
    let progress_window = window.clone();
    let doc = tauri::async_runtime::spawn_blocking(move || {
        core.run(move |drawn| {
            let path = if path.starts_with(rnl_view::BUNDLED_DIR) {
                PathBuf::from(&path)
            } else {
                std::path::absolute(&path).map_err(|e| format!("{path}: {e}"))?
            };
            let entry = path.to_string_lossy().into_owned();
            let held = files::held(&entry, &edits.unwrap_or_default());
            let disk = FsProvider::new(&paths).with_search_paths(&paths);
            let disk = rnl_runtime::source::FilePool { files: &disk, paths: &pool };
            let layered = files::layered(&held, &disk);
            let index = library.with_edits(&held, &layered);
            let layered =
                rnl_view::LibraryProvider { files: &layered, index: &index, enabled: auto_include.unwrap_or(false) };
            let provider = load_progress::LoadingProvider::new(&layered, &progress_window, request_id.as_deref());
            let (doc, kept) = rnl_view::build_drawn(&entry, &provider, &BuildOptions::default())?;
            *drawn = Some(kept);
            Ok::<_, String>(doc)
        })
    })
    .await
    .map_err(|e| e.to_string())???;
    unsaved.set(&window, |state| state.0 = doc.name.clone());
    Ok(doc)
}

/// Draw a netlist held in memory, such as one from the clipboard, as the file `name`, whose
/// extension says how it is read. Bare includes can use the file pool and PATH.
#[tauri::command]
// Tauri injects the window and managed state alongside the separate IPC arguments.
#[allow(clippy::too_many_arguments)]
async fn open_text(
    window: WebviewWindow,
    unsaved: State<'_, Unsaved>,
    core: State<'_, Core>,
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    auto_include: Option<bool>,
    pool: Option<Vec<String>>,
    name: String,
    text: String,
    edits: Option<Edits>,
    request_id: Option<String>,
) -> Result<Document, String> {
    let core = core.inner().clone();
    let paths = settings.paths()?;
    let library = library.snapshot()?;
    let pool = pool.unwrap_or_default();
    let progress_window = window.clone();
    let doc = tauri::async_runtime::spawn_blocking(move || {
        core.run(move |drawn| {
            let entry = format!("/{name}");
            let held = files::held_text(&entry, &text, &edits.unwrap_or_default());
            let disk = FsProvider::new(&paths).with_search_paths(&paths);
            let disk = rnl_runtime::source::FilePool { files: &disk, paths: &pool };
            let files = files::layered(&held, &disk);
            let index = library.with_edits(&held, &files);
            let files =
                rnl_view::LibraryProvider { files: &files, index: &index, enabled: auto_include.unwrap_or(false) };
            let provider = load_progress::LoadingProvider::new(&files, &progress_window, request_id.as_deref());
            let (doc, kept) = rnl_view::build_drawn(&entry, &provider, &BuildOptions::default())?;
            *drawn = Some(kept);
            Ok::<_, String>(doc)
        })
    })
    .await
    .map_err(|e| e.to_string())???;
    unsaved.set(&window, |state| state.0 = doc.name.clone());
    Ok(doc)
}

/// The drawings of the objects at `items` of `levels[level].unplaced` of the document opened last:
/// `{ svg }` each, in order. The page asks for the rows it shows, with their `names`, which must
/// be those of that document: a request about one that was opened since is refused.
#[tauri::command]
async fn unplaced_drawings(
    core: State<'_, Core>,
    level: usize,
    items: Vec<usize>,
    names: Vec<String>,
) -> Result<Vec<UnplacedDrawing>, String> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.run(move |drawn| match drawn {
            Some(d) => d.unplaced(level, &items, &names),
            None => Err("no document has kept drawings".to_string()),
        })
    })
    .await
    .map_err(|e| e.to_string())??
}

/// Write the files of the netlist at `entry` that the user chose to save where they were read from,
/// each in its encoding, and say which: nothing else is written (decision 0011). Unless the user
/// said to `overwrite`, a file that is not what it was when it was read is not written, nor are the
/// others; the names of those files are given back.
#[tauri::command]
async fn save_files(entry: String, files: Vec<SavedFile>, overwrite: Option<bool>) -> Result<Saved, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let entry = std::path::absolute(&entry).map_err(|e| format!("{entry}: {e}"))?;
        files::save_all(&entry.to_string_lossy(), &files, overwrite.unwrap_or(false))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Ask where to save the text of the netlist, write it there, and give its path; None when the
/// dialog is cancelled. `entry` is the file the netlist was opened from, if it was, for the folder
/// the dialog starts in.
#[tauri::command]
async fn save_file_as(
    window: WebviewWindow,
    entry: Option<String>,
    name: String,
    text: String,
) -> Result<Option<String>, String> {
    let name = name.rsplit(['/', '\\']).next().unwrap_or("netlist.sp").to_string();
    let mut dialog = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Save As")
        .set_file_name(name)
        .add_filter("Netlists and packages", &NETLISTS);
    if let Some(dir) = entry.map(PathBuf::from).and_then(|p| p.parent().map(PathBuf::from)).filter(|d| d.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    let chosen =
        tauri::async_runtime::spawn_blocking(move || dialog.blocking_save_file()).await.map_err(|e| e.to_string())?;
    let Some(chosen) = chosen else { return Ok(None) };
    let to = chosen.into_path().map_err(|e| e.to_string())?;
    let bytes = files::encode(std::fs::read(&to).ok().as_deref(), &text)?;
    std::fs::write(&to, bytes).map_err(|e| format!("{}: {e}", to.display()))?;
    Ok(Some(to.to_string_lossy().into_owned()))
}

/// The page says whether files have changes that are not saved: the window's title shows it, and
/// closing the window asks the page first.
#[tauri::command]
fn set_dirty(window: WebviewWindow, unsaved: State<'_, Unsaved>, dirty: bool) {
    unsaved.set(&window, |state| state.1 = dirty);
}

/// Close the app, which the page asked the user about.
#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

/// The initialized page is ready to replace the native Windows startup surface.
#[tauri::command]
fn finish_startup(window: WebviewWindow, startup: State<'_, startup::Startup>) -> Result<(), String> {
    startup.finish(&window)
}

/// Ask for a file to open; None when the dialog is cancelled.
#[tauri::command]
async fn pick_file(window: WebviewWindow) -> Option<String> {
    let dialog = window.dialog().file().set_parent(&window).add_filter("Netlists and packages", &NETLISTS);
    let picked =
        tauri::async_runtime::spawn_blocking(move || dialog.add_filter("All files", &["*"]).blocking_pick_file())
            .await
            .ok()??;
    picked.into_path().ok().map(|p| p.to_string_lossy().into_owned())
}

/// The file named on the command line, if any.
#[tauri::command]
fn initial_file() -> Option<String> {
    let arg = file_argument(std::env::args_os().skip(1))?;
    let path = std::path::absolute(PathBuf::from(arg)).ok()?;
    Some(path.to_string_lossy().into_owned())
}

/// Windows owns the choice of a default application; show its supported settings UI.
#[tauri::command]
fn open_default_apps(app: AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        use tauri_plugin_opener::OpenerExt;
        app.opener().open_url("ms-settings:defaultapps", None::<&str>).map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("File associations are configured in the operating system.".into())
    }
}

fn main() {
    let startup = startup::Startup::new();
    let session = session::read(std::env::args_os().skip(1)).expect("the viewer session could not be read");
    let mut context = tauri::generate_context!();
    // Keep the small-logo strokes in the title bar and taskbar, including at higher system DPI.
    #[cfg(windows)]
    context.set_default_window_icon(Some(tauri::include_image!("icons/32x32.png")));
    if let Some(position) = session.as_ref().and_then(|s| s.get("position")) {
        if let Some(window) = context.config_mut().app.windows.first_mut() {
            window.x = position["x"].as_f64();
            window.y = position["y"].as_f64();
        }
    }
    if let Some(port) = devtools_port(std::env::args_os().skip(1)) {
        for window in &mut context.config_mut().app.windows {
            window.additional_browser_args = Some(format!("{WEBVIEW_ARGS} {DEVTOOLS_SWITCH}{port}"));
        }
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // opens a release's page for Help > Check for Updates; the capability limits it to the repository's
        .plugin(tauri_plugin_opener::init())
        .manage(solve::Solves::default())
        .manage(Unsaved::default())
        .manage(settings::Settings::default())
        .manage(session::Initial(Mutex::new(session)))
        .manage(Core::new())
        .manage(library::Library::default())
        .manage(startup)
        .menu(menu::menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "exit" => request_exit(app),
            id => {
                let _ = app.emit_to("main", "menu", id);
            }
        })
        // the window that is asked to close with changes not saved stays, and the page asks the user
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.state::<Unsaved>().unsaved() {
                    api.prevent_close();
                    let _ = window.emit("close-requested", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            open_document,
            open_text,
            unplaced_drawings,
            save_files,
            save_file_as,
            set_dirty,
            quit,
            finish_startup,
            pick_file,
            initial_file,
            open_default_apps,
            session::initial_session,
            session::new_window,
            settings::load_settings,
            settings::save_setting,
            pool::pick_pool,
            pool::dropped_files,
            pool::bundled_files,
            pool::path_packages,
            library::update_library,
            menu::set_view_check,
            menu::set_command_enabled,
            solve::solve_preview,
            solve::solver_defaults,
            solve::check_solver_options,
            solve::cancel_solve,
            solve::save_solved_copy
        ])
        .run(context)
        .expect("the viewer window could not be started");
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use serde_json::Value;

    use super::{devtools_port, file_argument};

    fn config() -> Value {
        serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri.conf.json is JSON")
    }

    /// The DevTools switch opens the port, and the file is the other argument, before or after it.
    #[test]
    fn the_devtools_switch_is_not_the_file() {
        let args = |a: &[&str]| a.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(devtools_port(args(&["--remote-debugging-port=9222", "a.sp"])), Some(9222));
        assert_eq!(file_argument(args(&["--remote-debugging-port=9222", "a.sp"])), Some("a.sp".into()));
        assert_eq!(file_argument(args(&["a.sp", "--remote-debugging-port=9222"])), Some("a.sp".into()));
        assert_eq!(devtools_port(args(&["a.sp"])), None);
        assert_eq!(file_argument(args(&["a.sp"])), Some("a.sp".into()));
        assert_eq!(devtools_port(args(&["--remote-debugging-port=a.sp"])), None);
        assert_eq!(file_argument(args(&["--remote-debugging-port=9222"])), None);
        assert_eq!(file_argument(args(&["--session", "session.json", "--remote-debugging-port=9222"])), None);
        assert_eq!(file_argument(args(&["--session", "session.json", "a.sp"])), Some("a.sp".into()));
    }

    /// The version in `[workspace.package]` of a Cargo manifest.
    fn workspace_version(manifest: &str) -> Option<&str> {
        let section = manifest.split("\n[").find(|s| s.starts_with("workspace.package]"))?;
        let line = section.lines().find(|l| l.trim_start().starts_with("version"))?;
        line.split('"').nth(1)
    }

    /// The app has one version: the one Tauri reports and the installers carry, the crate's, which
    /// the About box shows, and the toolkit's, which the web viewer's About box shows.
    #[test]
    fn one_version() {
        let version = env!("CARGO_PKG_VERSION");
        assert_eq!(config()["version"], version, "tauri.conf.json");
        assert_eq!(workspace_version(include_str!("../../../Cargo.toml")), Some(version), "the workspace's Cargo.toml");
    }

    /// The licence that the installers show and install next to the app: the repository's MIT
    /// License, and the fonts' copyright notice with the SIL Open Font License, word for word, with a
    /// line per paragraph for the installers' windows to wrap, and the dashes in ASCII, since the
    /// MSI's licence dialog reads the text as Windows-1252.
    #[test]
    fn the_installers_licence_has_both_licences() {
        let words = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        let text = include_str!("../LICENSE.txt");
        assert_eq!(config()["bundle"]["licenseFile"], "LICENSE.txt");
        assert!(text.is_ascii());
        assert!(words(text).contains(&words(include_str!("../../../LICENSE"))));
        let dash = char::from_u32(0x2014).unwrap();
        assert!(words(text).contains(&words(&include_str!("../../../LICENSES/OFL-1.1.txt").replace(dash, "--"))));
        assert!(text.contains("Copyright 2014-2021 Adobe (http://www.adobe.com/)"));
    }

    /// Help > Check for Updates: the page may ask GitHub's API for the releases, and may open the
    /// repository's pages on GitHub in the default browser, but no other address.
    #[test]
    fn the_update_check_reaches_github_and_opens_only_the_repository() {
        let config = config();
        let csp = config["app"]["security"]["csp"].as_str().unwrap_or_default();
        let connect = csp.split(';').map(str::trim).find(|d| d.starts_with("connect-src ")).unwrap_or_default();
        assert!(connect.split_whitespace().any(|s| s == "https://api.github.com"), "{csp}");

        let capability: Value =
            serde_json::from_str(include_str!("../capabilities/default.json")).expect("the capability is JSON");
        let opener: Vec<&Value> = capability["permissions"]
            .as_array()
            .expect("the capability's permissions")
            .iter()
            .filter(|p| p.as_str().or(p["identifier"].as_str()).is_some_and(|id| id.starts_with("opener:")))
            .collect();
        let scope = "https://github.com/rich-netlist/rnl/*";
        let allowed = serde_json::json!({ "identifier": "opener:allow-open-url", "allow": [
            { "url": scope }, { "url": "https://rich-netlist.github.io/rnl/en/packages.html" }
        ] });
        assert_eq!(opener, [&allowed]);

        // the addresses the page asks for and opens
        let page = include_str!("../../viewer-ui/updates.js");
        assert!(page.contains("\"https://api.github.com/repos/rich-netlist/rnl/releases?"));
        assert!(page.contains(&format!("\"{}releases\"", scope.trim_end_matches('*'))));
    }
}
