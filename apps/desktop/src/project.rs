//! Projects (apps/viewer-ui/project.js): the page writes and reads a project's ZIP archive and its
//! manifest itself; the desktop asks where, reads the archive's bytes and the Library's files, and
//! writes the archive, or a project's files into the folder the user chose to open it in. Nothing is
//! written anywhere the user did not choose.

use std::path::{Path, PathBuf};

use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::menu;

/// The bytes of the file at `path`, as they are.
#[tauri::command]
pub async fn read_bytes(path: String) -> Result<Response, String> {
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::read(&path).map(Response::new).map_err(|e| format!("{path}: {e}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Write the request's bytes to the file its `path` header names (URI-encoded), with the folders it
/// needs: the archive the user chose to save, or a project's file in the folder the user chose.
#[tauri::command]
pub async fn write_bytes(request: Request<'_>) -> Result<(), String> {
    let InvokeBody::Raw(bytes) = request.body() else { return Err("write_bytes takes the file's bytes".into()) };
    let path = request
        .headers()
        .get("path")
        .and_then(|value| value.to_str().ok())
        .map(decode)
        .ok_or("write_bytes needs a path")?;
    let bytes = bytes.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = PathBuf::from(path);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Percent-decoding of `encodeURIComponent`, as UTF-8.
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut k = 0;
    while k < bytes.len() {
        if bytes[k] == b'%' && k + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[k + 1..k + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
            if let Some(byte) = hex {
                out.push(byte);
                k += 3;
                continue;
            }
        }
        out.push(bytes[k]);
        k += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Ask for a project's archive to open; None when the dialog is cancelled.
#[tauri::command]
pub async fn pick_project(window: WebviewWindow) -> Option<String> {
    let dialog = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title(menu::text(window.app_handle(), "Open Project"))
        .add_filter(menu::text(window.app_handle(), "Project archives"), &["zip"]);
    let picked = tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_file()).await.ok()??;
    picked.into_path().ok().map(|p| rnl_cli::path_str(&p))
}

/// Ask where to save a project's archive, named `name` and, when it was opened from one, in the
/// folder of `last`; None when the dialog is cancelled.
#[tauri::command]
pub async fn pick_project_save(window: WebviewWindow, name: String, last: Option<String>) -> Option<String> {
    let last = last.map(PathBuf::from);
    let name = last.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or(name);
    let mut dialog = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title(menu::text(window.app_handle(), "Save Project"))
        .set_file_name(name)
        .add_filter(menu::text(window.app_handle(), "Project archives"), &["zip"]);
    if let Some(dir) = last.as_ref().and_then(|p| p.parent()).filter(|d| d.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    let chosen = tauri::async_runtime::spawn_blocking(move || dialog.blocking_save_file()).await.ok()??;
    chosen.into_path().ok().map(|p| rnl_cli::path_str(&p))
}

/// Ask for the folder to open a project in, starting in the folder of its archive, `near`; None when
/// the dialog is cancelled.
#[tauri::command]
pub async fn pick_project_folder(window: WebviewWindow, near: Option<String>) -> Option<String> {
    let mut dialog = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title(menu::text(window.app_handle(), "Choose the folder to open the project in"));
    if let Some(dir) = near.as_deref().map(Path::new).and_then(|p| p.parent()).filter(|d| d.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    let chosen = tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_folder()).await.ok()??;
    chosen.into_path().ok().map(|p| rnl_cli::path_str(&p))
}

/// Which of the files at `paths`, relative to `root`, are there already.
#[tauri::command]
pub fn existing_files(root: String, paths: Vec<String>) -> Vec<String> {
    let root = PathBuf::from(root);
    paths.into_iter().filter(|path| root.join(path).exists()).collect()
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn paths_are_decoded_as_encode_uri_component_wrote_them() {
        assert_eq!(decode("C%3A%2Fwork%2F%E7%94%B5%E8%B7%AF%2Ftop.sp"), "C:/work/电路/top.sp");
        assert_eq!(decode("a%2"), "a%2");
        assert_eq!(decode("plain"), "plain");
    }
}
