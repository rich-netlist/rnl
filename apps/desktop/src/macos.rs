//! Finder sends file URLs through the application lifecycle, including before the page listens.
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Default)]
pub struct OpenedFiles(Mutex<Pending>);

#[derive(Default)]
struct Pending {
    ready: bool,
    paths: Vec<String>,
}

pub fn initial(app: &AppHandle) -> Option<String> {
    let state = app.state::<OpenedFiles>();
    let mut pending = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if pending.paths.is_empty() {
        None
    } else {
        Some(pending.paths.remove(0))
    }
}

pub fn ready(app: &AppHandle) {
    let state = app.state::<OpenedFiles>();
    let mut pending = state.0.lock().unwrap_or_else(|e| e.into_inner());
    pending.ready = true;
    let paths = std::mem::take(&mut pending.paths);
    if !paths.is_empty() {
        let _ = app.emit_to("main", "opened-files", paths);
    }
}

pub fn opened(app: &AppHandle, urls: Vec<tauri::Url>) {
    let paths: Vec<_> = urls
        .into_iter()
        .filter_map(|url| url.to_file_path().ok())
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    if paths.is_empty() {
        return;
    }
    let state = app.state::<OpenedFiles>();
    let mut pending = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if pending.ready {
        let _ = app.emit_to("main", "opened-files", paths);
    } else {
        pending.paths.extend(paths);
    }
}
