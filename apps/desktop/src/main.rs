//! RNL Viewer: open a netlist, draw its levels, show its text and diagnostics, and follow an
//! instance or a net between the drawing and the text. It can edit the text, build it again from the
//! texts in memory, and write the files the user changed when the user saves them (decision 0011);
//! it can solve the netlist as a preview, and save a solved copy where the user says (decision
//! 0010). It writes no file otherwise, and as opened, objects without a position are not drawn.

// no console window behind the app in release builds on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod files;
mod menu;
mod solve;

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Mutex;

use files::{Edits, Saved, SavedFile};
use rnl_cli::FsProvider;
use rnl_view::{BuildOptions, Document, MemoryProvider};
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
    args.into_iter().find(|a| !a.to_string_lossy().starts_with(DEVTOOLS_SWITCH))
}

/// The extensions offered first in the open dialog.
const NETLISTS: [&str; 10] = ["sp", "spi", "spice", "cir", "ckt", "net", "inc", "lib", "scs", "rnl"];

/// Read and draw a file, and name the window after it. With `edits`, the texts of some of its
/// files are those, over what is on disk.
#[tauri::command]
async fn open_document(
    window: WebviewWindow,
    unsaved: State<'_, Unsaved>,
    path: String,
    edits: Option<Edits>,
) -> Result<Document, String> {
    let doc = tauri::async_runtime::spawn_blocking(move || {
        let path = std::path::absolute(&path).map_err(|e| format!("{path}: {e}"))?;
        let entry = path.to_string_lossy().into_owned();
        let held = files::held(&entry, &edits.unwrap_or_default());
        let disk = FsProvider::new(&[]);
        rnl_view::build(&entry, &files::layered(&held, &disk), &BuildOptions::default())
    })
    .await
    .map_err(|e| e.to_string())??;
    unsaved.set(&window, |state| state.0 = doc.name.clone());
    Ok(doc)
}

/// Draw a netlist held in memory, such as one from the clipboard, as the file `name`, whose
/// extension says how it is read, alone in a workspace of its own: what it includes is missing.
#[tauri::command]
async fn open_text(
    window: WebviewWindow,
    unsaved: State<'_, Unsaved>,
    name: String,
    text: String,
) -> Result<Document, String> {
    let doc = tauri::async_runtime::spawn_blocking(move || {
        let entry = format!("/{name}");
        rnl_view::build(&entry, &MemoryProvider::new().with(&entry, &text), &BuildOptions::default())
    })
    .await
    .map_err(|e| e.to_string())??;
    unsaved.set(&window, |state| state.0 = doc.name.clone());
    Ok(doc)
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

fn main() {
    let mut context = tauri::generate_context!();
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
            save_files,
            save_file_as,
            set_dirty,
            quit,
            pick_file,
            initial_file,
            menu::set_view_check,
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
        let allowed = serde_json::json!({ "identifier": "opener:allow-open-url", "allow": [{ "url": scope }] });
        assert_eq!(opener, [&allowed]);

        // the addresses the page asks for and opens
        let page = include_str!("../../viewer-ui/updates.js");
        assert!(page.contains("\"https://api.github.com/repos/rich-netlist/rnl/releases?"));
        assert!(page.contains(&format!("\"{}releases\"", scope.trim_end_matches('*'))));
    }
}
