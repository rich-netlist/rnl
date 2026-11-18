//! Projects (apps/viewer-ui/project.js): the page writes and reads a project's ZIP archive and its
//! manifest itself; the desktop asks where, reads the archive's bytes and the Library's files, and
//! writes the archive, or a project's files into the folder the user chose to open it in. Nothing is
//! written anywhere the user did not choose: the app keeps, for each window, the archive and the folder its dialogs gave,
//! and writes the archive to that one alone and a project's files only by their names within that folder, never through a
//! link or a junction, and each file as Save writes one (atomic.rs): an archive that cannot be written whole leaves the
//! old one as it was.

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
    tauri::async_runtime::spawn_blocking(move || {
        crate::atomic::write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Write the request's bytes to the project's file its `path` header names (URI-encoded), relative to the folder the user
/// chose to open the project in (`pick_project_folder`), with the folders it needs (`inside`).
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
        let path = inside(&root, &relative, true)?;
        crate::atomic::write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Whether a file's metadata, not followed, is a link: a symbolic link, or on Windows any reparse point, a junction among
/// them.
fn is_link(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}

/// The file `relative` names within `root`: a relative name of folders and a file, `/` or `\` between them, none empty,
/// `.`, `..` or with a `:` (a drive, or a stream of a file). Each folder on the way is one of its own, not a link or a
/// junction, which could lead out of the root; with `make`, those not there are made. The file is not a link either.
/// Where the folders are found or made, they are still in the root, for a link made in the meantime.
fn inside(root: &Path, relative: &str, make: bool) -> Result<PathBuf, String> {
    let parts: Vec<&str> = relative.split(['/', '\\']).collect();
    if parts.iter().any(|p| p.is_empty() || *p == "." || *p == ".." || p.contains(':')) {
        return Err(format!("{relative}: not a name within the project's folder"));
    }
    let real = std::fs::canonicalize(root).map_err(|e| format!("{}: {e}", root.display()))?;
    let mut at = real.clone();
    for (k, part) in parts.iter().enumerate() {
        at.push(part);
        let last = k + 1 == parts.len();
        match std::fs::symlink_metadata(&at) {
            Ok(meta) if is_link(&meta) => {
                return Err(format!(
                    "{}: a link or a junction; a project's file is not written through it",
                    at.display()
                ))
            }
            Ok(meta) if !last && !meta.is_dir() => return Err(format!("{}: not a folder", at.display())),
            Ok(meta) if last && !meta.is_file() => return Err(format!("{}: not a file", at.display())),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if !last && make {
                    std::fs::create_dir(&at).map_err(|e| format!("{}: {e}", at.display()))?;
                } else if !last {
                    break;
                }
            }
            Err(e) => return Err(format!("{}: {e}", at.display())),
        }
    }
    let folder = at.parent().unwrap_or(&real);
    if folder.exists() {
        let folder = std::fs::canonicalize(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        if !folder.starts_with(&real) {
            return Err(format!("{relative}: not within the project's folder"));
        }
    }
    Ok(real.join(parts.join(std::path::MAIN_SEPARATOR_STR)))
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
/// names a project's file can be written at (`inside`), which the user is told of before anything is written.
#[tauri::command]
pub fn existing_files(window: WebviewWindow, choices: State<'_, Choices>, paths: Vec<String>) -> Vec<String> {
    let Some(root) = choices.get(&window).root else { return paths };
    paths
        .into_iter()
        .filter(|path| inside(&root, path, false).map_or(true, |at| std::fs::symlink_metadata(at).is_ok()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{decode, inside};
    use std::path::{Path, PathBuf};

    struct Dir(PathBuf);
    impl Dir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("rnl-project-test-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Dir(dir)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A link to the folder `to` at `at`: a junction on Windows, which needs no privilege; a symbolic link elsewhere.
    fn link_folder(at: &Path, to: &Path) -> bool {
        #[cfg(windows)]
        {
            std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(at)
                .arg(to)
                .output()
                .is_ok_and(|o| o.status.success())
        }
        #[cfg(not(windows))]
        {
            std::os::unix::fs::symlink(to, at).is_ok()
        }
    }

    #[test]
    fn a_projects_files_are_written_by_their_names_within_the_folder_chosen() {
        let dir = Dir::new("inside");
        let root = dir.0.join("root");
        std::fs::create_dir(&root).unwrap();
        let at = inside(&root, "lib/models/a.inc", true).unwrap();
        assert!(at.starts_with(std::fs::canonicalize(&root).unwrap()));
        assert!(at.parent().unwrap().is_dir(), "the folders it needs are made");
        for name in ["../out.sp", "lib/../../out.sp", "", "a//b.sp", "./a.sp", "C:/out.sp", "a.sp:stream", "/abs.sp"] {
            assert!(inside(&root, name, true).is_err(), "{name}");
        }
        assert!(!dir.0.join("out.sp").exists());
    }

    #[test]
    fn a_link_or_a_junction_in_the_folder_chosen_is_not_written_through() {
        let dir = Dir::new("links");
        let root = dir.0.join("root");
        let outside = dir.0.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        if !link_folder(&root.join("lib"), &outside) {
            eprintln!("no link could be made here: nothing to try");
            return;
        }
        let e = inside(&root, "lib/a.inc", true).unwrap_err();
        assert!(e.contains("link or a junction"), "{e}");
        assert!(std::fs::read_dir(&outside).unwrap().next().is_none(), "nothing is written outside");
        // the folder chosen may itself be reached by a link: the user chose it
        let at = inside(&root.join("lib"), "a.inc", true).unwrap();
        assert!(at.starts_with(std::fs::canonicalize(&outside).unwrap()));
    }

    #[test]
    fn paths_are_decoded_as_encode_uri_component_wrote_them() {
        assert_eq!(decode("C%3A%2Fwork%2F%E7%94%B5%E8%B7%AF%2Ftop.sp"), "C:/work/电路/top.sp");
        assert_eq!(decode("a%2"), "a%2");
        assert_eq!(decode("plain"), "plain");
    }
}
