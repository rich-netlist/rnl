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
        // opens a release's page for Help > Check for Updates; the capability limits it to the repository's
        .plugin(tauri_plugin_opener::init())
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

#[cfg(test)]
mod tests {
    use serde_json::Value;

    fn config() -> Value {
        serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri.conf.json is JSON")
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
