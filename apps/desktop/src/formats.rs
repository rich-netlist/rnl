//! The parse formats: the core's list, and the format it detects for a Library file that the
//! document shown does not read. The page keeps the formats chosen for files and gives them with each
//! build, preload, solve and Library update, as `{ path: format }`.
use std::collections::BTreeMap;

use rnl_cli::FsProvider;
use rnl_runtime::source::Provider;
use tauri::State;

/// The formats chosen for files, by path; a file not in it reads by default.
pub type Formats = BTreeMap<String, String>;

/// The core's formats, in its order, as the Library's menus list them.
#[tauri::command]
pub fn formats() -> Vec<rnl_view::FormatInfo> {
    rnl_view::formats()
}

/// The format the core detects for the file at `path`, read from the disk.
#[tauri::command]
pub async fn detect_format(settings: State<'_, crate::settings::Settings>, path: String) -> Result<String, String> {
    let paths = settings.paths()?;
    tauri::async_runtime::spawn_blocking(move || {
        let text = FsProvider::new(&paths).read(&path).ok_or_else(|| format!("cannot read {path}"))?;
        Ok(rnl_view::detect_format(&path, &text).to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
