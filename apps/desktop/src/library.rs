//! Per-process viewer library index. IPC updates finish before a dependent build starts.
use rnl_cli::FsProvider;
use rnl_runtime::source::FilePool;
use rnl_view::LibraryIndex;
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Clone, Default)]
pub struct Library(pub Arc<Mutex<LibraryIndex>>);

impl Library {
    pub fn snapshot(&self) -> Result<LibraryIndex, String> {
        self.0.lock().map(|i| i.clone()).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn update_library(
    library: State<'_, Library>,
    settings: State<'_, crate::settings::Settings>,
    pool: Vec<String>,
    reload: Option<bool>,
    refresh: Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    let library = library.inner().clone();
    let paths = settings.paths()?;
    tauri::async_runtime::spawn_blocking(move || {
        let disk = FsProvider::new(&paths).with_search_paths(&paths);
        let files = FilePool { files: &disk, paths: &pool };
        let mut index = library.0.lock().map_err(|e| e.to_string())?;
        index.invalidate(&refresh.unwrap_or_default());
        Ok(index.update(&pool, &files, reload.unwrap_or(false)))
    })
    .await
    .map_err(|e| e.to_string())?
}
