//! RNL Viewer: open a netlist, draw its levels, show its text and diagnostics, and follow an
//! instance or a net between the drawing and the text. It only reads; it neither edits nor
//! solves, so objects without a position are not drawn.

// no console window behind the app in release builds on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod menu;

use std::path::PathBuf;

use rnl_cli::FsProvider;
use rnl_view::{BuildOptions, Document};
use tauri::{Emitter, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

/// The extensions offered first in the open dialog.
const NETLISTS: [&str; 10] = ["sp", "spi", "spice", "cir", "ckt", "net", "inc", "lib", "scs", "rnl"];

/// Read and draw a file, and name the window after it.
#[tauri::command]
async fn open_document(window: WebviewWindow, path: String) -> Result<Document, String> {
    let doc = tauri::async_runtime::spawn_blocking(move || {
        let path = std::path::absolute(&path).map_err(|e| format!("{path}: {e}"))?;
        rnl_view::build(&path.to_string_lossy(), &FsProvider::new(&[]), &BuildOptions::default())
    })
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .menu(menu::menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "exit" => app.exit(0),
            id => {
                let _ = app.emit_to("main", "menu", id);
            }
        })
        .invoke_handler(tauri::generate_handler![open_document, pick_file, initial_file, menu::set_view_check])
        .run(tauri::generate_context!())
        .expect("the viewer window could not be started");
}
