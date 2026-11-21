//! Projects (apps/viewer-ui/project.js): the page writes and reads a project's ZIP archive and its
//! manifest itself; the desktop asks where, reads the archive's bytes and the Library's files, and
//! writes the archive, or a project's files into the folder the user chose to open it in. Nothing is
//! written anywhere the user did not choose: the app keeps, for each window, the archive and the folder its dialogs gave,
//! and writes the archive to that one alone and a project's files only by their names within that folder, never through a
//! link or a junction, and each file as Save writes one: an archive that cannot be written whole leaves the old one as it
//! was. A file of the folder that has other names, hard links, is the one exception: Save keeps them, but a project's file
//! replaces it at its name in the folder alone, so that a name outside the folder the user chose keeps what it had
//! (`rnl_viewer_files::project`, as the local viewer writes a project's files).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{Manager, State, WebviewWindow};
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

/// `text` as bytes in the encoding the file at `path` has (UTF-8 with or without its byte order mark, or Latin-1), as
/// Save writes an edit: a project archives an edited file so (apps/viewer-ui/project.js).
#[tauri::command]
pub async fn encode_like(path: String, text: String) -> Result<Response, String> {
    tauri::async_runtime::spawn_blocking(move || {
        rnl_cli::codec::encode_file(Some(Path::new(&path)), &text, rnl_cli::codec::OutputEncoding::Preserve)
            .map(Response::new)
            .map_err(|e| format!("{path}: {e}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// What the dialogs of a window last gave: the archive to save a project as, and the folder to open one in.
#[derive(Default, Clone)]
struct Chosen {
    archive: Option<PathBuf>,
    root: Option<PathBuf>,
}

/// The choices of each window, by its label: the only places a project is written.
#[derive(Default)]
pub struct Choices(Mutex<HashMap<String, Chosen>>);
impl Choices {
    fn get(&self, window: &WebviewWindow) -> Chosen {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).get(window.label()).cloned().unwrap_or_default()
    }
    fn set(&self, window: &WebviewWindow, change: impl FnOnce(&mut Chosen)) {
        change(self.0.lock().unwrap_or_else(|e| e.into_inner()).entry(window.label().to_string()).or_default());
    }
}

/// Write the request's bytes to the archive the user chose to save the project as (`pick_project_save`), as Save writes a
/// file: whole, or the old archive is left as it was.
#[tauri::command]
pub async fn write_project_archive(
    window: WebviewWindow,
    choices: State<'_, Choices>,
    request: Request<'_>,
) -> Result<(), String> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("write_project_archive takes the archive's bytes".into());
    };
    let path = choices.get(&window).archive.ok_or("no archive was chosen to save the project as")?;
    let bytes = bytes.clone();
    tauri::async_runtime::spawn_blocking(move || rnl_viewer_files::save::write_chosen(&path, &bytes, true).map(|_| ()))
        .await
        .map_err(|e| e.to_string())?
}

/// Write the request's bytes to the project's file its `path` header names (URI-encoded), relative to the folder the user
/// chose to open the project in (`pick_project_folder`), with the folders it needs (`rnl_viewer_files::project`).
#[tauri::command]
pub async fn write_project_file(
    window: WebviewWindow,
    choices: State<'_, Choices>,
    request: Request<'_>,
) -> Result<(), String> {
    let InvokeBody::Raw(bytes) = request.body() else { return Err("write_project_file takes the file's bytes".into()) };
    let relative = request
        .headers()
        .get("path")
        .and_then(|value| value.to_str().ok())
        .map(decode)
        .ok_or("write_project_file needs a path")?;
    let root = choices.get(&window).root.ok_or("no folder was chosen to open the project in")?;
    let bytes = bytes.clone();
    tauri::async_runtime::spawn_blocking(move || {
        rnl_viewer_files::project::write_file(&root, &relative, &bytes).map(|_| ())
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
pub async fn pick_project_save(
    window: WebviewWindow,
    choices: State<'_, Choices>,
    name: String,
    last: Option<String>,
) -> Result<Option<String>, String> {
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
    let chosen = tauri::async_runtime::spawn_blocking(move || dialog.blocking_save_file()).await.ok().flatten();
    let path = chosen.and_then(|c| c.into_path().ok());
    choices.set(&window, |c| c.archive = path.clone());
    Ok(path.map(|p| rnl_cli::path_str(&p)))
}

/// Ask for the folder to open a project in, starting in the folder of its archive, `near`; None when
/// the dialog is cancelled.
#[tauri::command]
pub async fn pick_project_folder(
    window: WebviewWindow,
    choices: State<'_, Choices>,
    near: Option<String>,
) -> Result<Option<String>, String> {
    let mut dialog = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title(menu::text(window.app_handle(), "Choose the folder to open the project in"));
    if let Some(dir) = near.as_deref().map(Path::new).and_then(|p| p.parent()).filter(|d| d.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    let chosen = tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_folder()).await.ok().flatten();
    let path = chosen.and_then(|c| c.into_path().ok());
    choices.set(&window, |c| c.root = path.clone());
    Ok(path.map(|p| rnl_cli::path_str(&p)))
}

/// Which of the files at `paths`, relative to the folder chosen to open the project in, are there already, or are not
/// names a project's file can be written at, which the user is told of before anything is written.
#[tauri::command]
pub fn existing_files(window: WebviewWindow, choices: State<'_, Choices>, paths: Vec<String>) -> Vec<String> {
    let Some(root) = choices.get(&window).root else { return paths };
    rnl_viewer_files::project::existing(&root, paths)
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
