//! RNL Viewer: open a netlist, draw its levels, list its diagnostics. It only reads; it neither
//! edits nor solves, so objects without a position are not drawn.

// no console window behind the app in release builds on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod document;

use std::path::PathBuf;

use document::Document;
use tauri::WebviewWindow;
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![open_document, pick_file, initial_file])
        .run(tauri::generate_context!())
        .expect("the viewer window could not be started");
}
