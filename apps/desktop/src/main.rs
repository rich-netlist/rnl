//! RNL Viewer: open a netlist, draw its levels, show its text and diagnostics, and follow an
//! instance or a net between the drawing and the text. It only reads; it neither edits nor
//! solves, so objects without a position are not drawn.

// no console window behind the app in release builds on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod document;

use std::path::PathBuf;

use document::Document;
use tauri::menu::{
    AboutMetadata, CheckMenuItem, CheckMenuItemBuilder, Menu, MenuItem, MenuItemBuilder, PredefinedMenuItem,
    SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow, Wry};
use tauri_plugin_dialog::DialogExt;

/// The extensions offered first in the open dialog.
const NETLISTS: [&str; 10] = ["sp", "spi", "spice", "cir", "ckt", "net", "inc", "lib", "scs", "rnl"];

/// Read and draw a file, and name the window after it.
#[tauri::command]
async fn open_document(window: WebviewWindow, path: String) -> Result<Document, String> {
    let doc = tauri::async_runtime::spawn_blocking(move || document::load(&PathBuf::from(path)))
        .await
        .map_err(|e| e.to_string())??;
    let _ = window.set_title(&format!("{} - RNL Viewer", doc.name));
    Ok(doc)
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
    let arg = std::env::args_os().nth(1)?;
    let path = std::path::absolute(PathBuf::from(arg)).ok()?;
    Some(path.to_string_lossy().into_owned())
}

/// The View menu's check marks, which follow the panels however they are shown or hidden.
struct ViewChecks {
    netlist: CheckMenuItem<Wry>,
    problems: CheckMenuItem<Wry>,
}

/// Set a check mark of the View menu: "netlist" or "problems".
#[tauri::command]
fn set_view_check(checks: State<'_, ViewChecks>, panel: String, shown: bool) {
    let item = match panel.as_str() {
        "netlist" => &checks.netlist,
        "problems" => &checks.problems,
        _ => return,
    };
    let _ = item.set_checked(shown);
}

/// A menu item's text with its shortcut. On Windows the page handles the shortcut keys, which
/// reach the WebView before any menu accelerator, so the shortcut is only shown; elsewhere the menu
/// handles it, and the page never sees the key.
fn labelled(text: &str, shortcut: Option<(&str, &str)>) -> (String, Option<String>) {
    match shortcut {
        Some((_, shown)) if cfg!(windows) => (format!("{text}\t{shown}"), None),
        Some((accel, _)) => (text.to_string(), Some(accel.to_string())),
        None => (text.to_string(), None),
    }
}

fn item(app: &AppHandle, id: &str, text: &str, shortcut: Option<(&str, &str)>) -> tauri::Result<MenuItem<Wry>> {
    let (text, accel) = labelled(text, shortcut);
    let mut b = MenuItemBuilder::with_id(id, text);
    if let Some(a) = accel {
        b = b.accelerator(a);
    }
    b.build(app)
}

fn check(app: &AppHandle, id: &str, text: &str, shortcut: (&str, &str)) -> tauri::Result<CheckMenuItem<Wry>> {
    let (text, accel) = labelled(text, Some(shortcut));
    let mut b = CheckMenuItemBuilder::with_id(id, text).checked(true);
    if let Some(a) = accel {
        b = b.accelerator(a);
    }
    b.build(app)
}

/// The menu bar: File, View, Help. Every item but Exit and About is passed to the page as a
/// `menu` event carrying the item's id.
fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let file = SubmenuBuilder::new(app, "&File")
        .item(&item(app, "open", "&Open...", Some(("CmdOrCtrl+O", "Ctrl+O")))?)
        .item(&item(app, "reload", "&Reload", Some(("F5", "F5")))?)
        .separator()
        .item(&item(app, "exit", "E&xit", None)?)
        .build()?;
    let netlist = check(app, "toggle-netlist", "&Netlist Panel", ("CmdOrCtrl+B", "Ctrl+B"))?;
    let problems = check(app, "toggle-problems", "&Problems", ("CmdOrCtrl+Shift+M", "Ctrl+Shift+M"))?;
    let view = SubmenuBuilder::new(app, "&View")
        .item(&item(app, "fit", "&Fit to Window", Some(("F", "F")))?)
        .item(&item(app, "zoom-in", "Zoom &In", Some(("CmdOrCtrl+=", "Ctrl++")))?)
        .item(&item(app, "zoom-out", "Zoom &Out", Some(("CmdOrCtrl+-", "Ctrl+-")))?)
        .separator()
        .item(&netlist)
        .item(&problems)
        .build()?;
    let about = AboutMetadata {
        name: Some("RNL Viewer".into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        comments: Some(
            "Draws netlists with Rich Netlist (RNL) records. It only reads: it neither edits nor solves.".into(),
        ),
        ..Default::default()
    };
    let help = SubmenuBuilder::new(app, "&Help")
        .item(&PredefinedMenuItem::about(app, Some("&About RNL Viewer"), Some(about))?)
        .build()?;
    app.manage(ViewChecks { netlist, problems });
    Menu::with_items(app, &[&file, &view, &help])
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .menu(menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "exit" => app.exit(0),
            id => {
                let _ = app.emit_to("main", "menu", id);
            }
        })
        .invoke_handler(tauri::generate_handler![open_document, pick_file, initial_file, set_view_check])
        .run(tauri::generate_context!())
        .expect("the viewer window could not be started");
}
