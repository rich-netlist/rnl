//! Per-process viewer library index. IPC updates finish before a dependent reading starts.
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};

use rnl_runtime::numbering::{hex, sha256};
use rnl_runtime::source::{FilePool, Provider};
use rnl_view::LibraryIndex;
use tauri::State;

/// The Library: its index, and the hash of the text of each of its files, by the path the page gives it, as the core
/// hashes a text (the first 16 hexadecimal digits of its SHA-256), which the page stamps the Library with
/// (apps/viewer-ui/file-results.js `libraryStamps`): a file read again with the same text leaves the stamp as it was.
#[derive(Clone, Default)]
pub struct Library(pub Arc<Mutex<LibraryIndex>>, Arc<Mutex<BTreeMap<String, String>>>);

impl Library {
    pub fn snapshot(&self) -> Result<LibraryIndex, String> {
        self.0.lock().map(|i| i.clone()).map_err(|e| e.to_string())
    }
}

/// Hash the files of `pool` that are new, or read again (`reload`, `refresh`), into `hashes`, and drop those of the
/// files no longer there or not read.
fn rehash(
    hashes: &mut BTreeMap<String, String>,
    pool: &[String],
    files: &dyn Provider,
    reload: bool,
    refresh: &[String],
) {
    let listed: HashSet<&String> = pool.iter().collect();
    hashes.retain(|path, _| listed.contains(path));
    for path in pool {
        if !reload && !refresh.contains(path) && hashes.contains_key(path) {
            continue;
        }
        match files.read(path) {
            Some(text) => {
                hashes.insert(path.clone(), hex(&sha256(text.as_bytes()))[..16].to_string());
            }
            None => {
                hashes.remove(path);
            }
        }
    }
}

#[tauri::command]
pub async fn update_library(
    library: State<'_, Library>,
    settings: State<'_, crate::settings::Settings>,
    pool: Vec<String>,
    reload: Option<bool>,
    refresh: Option<Vec<String>>,
    formats: Option<crate::formats::Formats>,
) -> Result<Vec<String>, String> {
    let library = library.inner().clone();
    let paths = settings.paths()?;
    tauri::async_runtime::spawn_blocking(move || {
        let disk = crate::public_packages::provider(&paths);
        let files = FilePool { files: &disk, paths: &pool };
        let mut index = library.0.lock().map_err(|e| e.to_string())?;
        let refresh = refresh.unwrap_or_default();
        let reload = reload.unwrap_or(false);
        index.invalidate(&refresh);
        // a file whose format changed, and those that include it, are read again
        index.set_formats(formats.unwrap_or_default());
        let errors = index.update(&pool, &files, reload);
        drop(index);
        rehash(&mut *library.1.lock().map_err(|e| e.to_string())?, &pool, &files, reload, &refresh);
        Ok(errors)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The hash of the text of each file of the Library, by its path, as `update_library` read it last.
#[tauri::command]
pub fn library_hashes(library: State<'_, Library>) -> Result<BTreeMap<String, String>, String> {
    library.1.lock().map(|h| h.clone()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use rnl_view::MemoryProvider;

    use super::*;

    #[test]
    fn the_hashes_follow_the_files_and_a_text_read_again_keeps_its_hash() {
        let pool = vec!["/l/a.sp".to_string(), "/l/b.sp".to_string()];
        let files = MemoryProvider::new().with("/l/a.sp", "* a\n").with("/l/b.sp", "* b\n");
        let mut hashes = BTreeMap::new();
        rehash(&mut hashes, &pool, &files, false, &[]);
        assert_eq!(hashes.len(), 2);
        assert_eq!(hashes["/l/a.sp"], hex(&sha256(b"* a\n"))[..16]);
        let before = hashes.clone();
        // read again, all of them, with the same texts: the same hashes
        rehash(&mut hashes, &pool, &files, true, &[]);
        assert_eq!(hashes, before);
        // a file changed, and read again; a file gone
        let files = MemoryProvider::new().with("/l/a.sp", "* a again\n");
        rehash(&mut hashes, &pool[..1], &files, false, &["/l/a.sp".to_string()]);
        assert_eq!(hashes.keys().collect::<Vec<_>>(), ["/l/a.sp"]);
        assert_ne!(hashes["/l/a.sp"], before["/l/a.sp"]);
    }
}
