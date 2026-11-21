//! The readings kept on disk, beside those kept in memory (build_cache.rs): one file each, in the app's cache folder,
//! shared by the windows and the app's processes, so that a file read once is shown again without reading it, also
//! after the app is started again, while what it read holds. Within a limit in bytes, 2 GiB by default, the least
//! recently used go first, whatever the version of the app that wrote them. A file is written to a name of its own and
//! renamed into place, so that a reader finds a whole file or none; one that cannot be read as a reading is removed.
//! Nothing here knows what a reading holds: its bytes are build_cache.rs's.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime};

use rnl_runtime::numbering::{hex, sha256};

pub const DEFAULT_LIMIT: u64 = 2 * 1024 * 1024 * 1024;

/// The most the limit may be set to, in MiB.
pub const MAX_MIB: u64 = 65536;

/// The cache of the app, once `init` has found its folder.
static SHARED: OnceLock<Arc<DiskCache>> = OnceLock::new();

/// Set up the app's cache in its cache folder (`readings` in Tauri's `app_cache_dir`), for the readings of this
/// version of the app and its core. Without a cache folder, nothing is kept on disk.
pub fn init(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Ok(dir) = app.path().app_cache_dir() {
        let _ = SHARED.set(Arc::new(DiskCache::new(dir.join("readings"), &format())));
    }
}

/// The app's cache, when it has one.
pub fn shared() -> Option<Arc<DiskCache>> {
    SHARED.get().cloned()
}

/// What the readings of this app are written by: the core's version with its commit, the shape of its results, and
/// the app's version, as 16 hexadecimal digits. The readings of another are in a folder of their own.
fn format() -> String {
    let made = format!("{} {} {}", rnl_view::core_version(), rnl_view::FILE_RESULT_FORMAT, env!("CARGO_PKG_VERSION"));
    hex(&sha256(made.as_bytes()))[..16].to_string()
}

/// The readings on disk under `root`: those of this app in `root/<format>`, each at `<first two digits>/<key>.json`.
pub struct DiskCache {
    root: PathBuf,
    format: String,
    limit: AtomicU64,
    /// For the names of the files being written.
    written: AtomicUsize,
}

/// A file of the cache: where, how big, and when it was last used.
struct Found {
    path: PathBuf,
    bytes: u64,
    used: SystemTime,
}

impl DiskCache {
    pub fn new(root: PathBuf, format: &str) -> Self {
        Self { root, format: format.to_string(), limit: AtomicU64::new(DEFAULT_LIMIT), written: AtomicUsize::new(0) }
    }

    pub fn limit(&self) -> u64 {
        self.limit.load(Ordering::Relaxed)
    }

    /// Set the limit, and remove the least recently used readings until they fit. 0 keeps none.
    pub fn set_limit(&self, limit: u64) {
        if self.limit.swap(limit, Ordering::Relaxed) != limit {
            self.trim();
        }
    }

    /// The file of the reading `key` (hexadecimal digits).
    fn path(&self, key: &str) -> PathBuf {
        self.root.join(&self.format).join(&key[..2.min(key.len())]).join(format!("{key}.json"))
    }

    /// The bytes of the reading `key`, which is then the most recently used; None when there is none, or the limit is
    /// 0.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let bytes = self.peek(key)?;
        let _ = fs::File::options().write(true).open(self.path(key)).and_then(|f| f.set_modified(SystemTime::now()));
        Some(bytes)
    }

    /// The bytes of the reading `key`, without using it.
    pub fn peek(&self, key: &str) -> Option<Vec<u8>> {
        if self.limit() == 0 {
            return None;
        }
        fs::read(self.path(key)).ok()
    }

    /// Remove the reading `key`, one that cannot be read.
    pub fn remove(&self, key: &str) {
        let _ = fs::remove_file(self.path(key));
    }

    /// Keep `bytes` as the reading `key`, unless they are more than a quarter of the limit; then remove the least
    /// recently used readings until all fit. A failure only leaves it out.
    pub fn put(&self, key: &str, bytes: &[u8]) {
        let limit = self.limit();
        if limit == 0 || bytes.len() as u64 > limit / 4 {
            return;
        }
        let path = self.path(key);
        let Some(dir) = path.parent() else { return };
        if fs::create_dir_all(dir).is_err() {
            return;
        }
        let n = self.written.fetch_add(1, Ordering::Relaxed);
        let part = dir.join(format!("{key}.{}.{n}.part", std::process::id()));
        if fs::write(&part, bytes).is_err() || fs::rename(&part, &path).is_err() {
            let _ = fs::remove_file(&part);
            return;
        }
        self.trim();
    }

    /// How many readings there are, and their bytes, of every version.
    pub fn stats(&self) -> (usize, u64) {
        let found = self.found();
        (found.len(), found.iter().map(|f| f.bytes).sum())
    }

    /// Remove every reading, of every version.
    pub fn clear(&self) {
        for f in self.found() {
            let _ = fs::remove_file(&f.path);
        }
    }

    /// Remove the least recently used readings, of any version, until the rest fit the limit; and the parts of files
    /// left by a write that did not finish, an hour later.
    fn trim(&self) {
        let limit = self.limit();
        let mut found = self.found();
        let mut total: u64 = found.iter().map(|f| f.bytes).sum();
        if total <= limit {
            return;
        }
        found.sort_by_key(|f| f.used);
        for f in found {
            if total <= limit {
                break;
            }
            if fs::remove_file(&f.path).is_ok() {
                total -= f.bytes;
            }
        }
    }

    /// The readings under the root, of every version; a part left by a write an hour ago or more is removed.
    fn found(&self) -> Vec<Found> {
        let mut out = Vec::new();
        let stale = SystemTime::now() - Duration::from_secs(3600);
        walk(&self.root, 0, &mut |path, meta| {
            let used = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            match path.extension().and_then(|e| e.to_str()) {
                Some("json") => out.push(Found { path: path.to_path_buf(), bytes: meta.len(), used }),
                Some("part") if used < stale => {
                    let _ = fs::remove_file(path);
                }
                _ => {}
            }
        });
        out
    }
}

/// Each file below `dir`, three folders deep at most, with its metadata; links are not followed.
fn walk(dir: &Path, depth: usize, each: &mut dyn FnMut(&Path, &fs::Metadata)) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let Ok(meta) = fs::symlink_metadata(entry.path()) else { continue };
        if meta.is_dir() && depth < 2 {
            walk(&entry.path(), depth + 1, each);
        } else if meta.is_file() {
            each(&entry.path(), &meta);
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A folder of its own for a test, removed when it is dropped.
    pub struct Folder(pub PathBuf);
    impl Folder {
        pub fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("rnl-disk-cache-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Folder(dir)
        }
    }
    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn key(n: u8) -> String {
        hex(&sha256(&[n]))
    }

    #[test]
    fn a_reading_put_is_got_again_by_its_key_and_another_version_does_not_see_it() {
        let folder = Folder::new("put");
        let cache = DiskCache::new(folder.0.clone(), "aaaa");
        assert_eq!(cache.get(&key(1)), None);
        cache.put(&key(1), b"one");
        assert_eq!(cache.get(&key(1)).as_deref(), Some(&b"one"[..]));
        assert_eq!(cache.stats(), (1, 3));
        let other = DiskCache::new(folder.0.clone(), "bbbb");
        assert_eq!(other.get(&key(1)), None);
        // the other version's readings count with its own, and Clear Cache removes them all
        other.put(&key(2), b"two");
        assert_eq!(cache.stats(), (2, 6));
        cache.clear();
        assert_eq!(other.stats(), (0, 0));
        // no part of a file is left behind
        assert!(fs::read_dir(folder.0.join("aaaa").join(&key(1)[..2])).unwrap().all(|e| e
            .unwrap()
            .path()
            .extension()
            .unwrap()
            != "part"));
    }

    #[test]
    fn the_least_recently_used_go_first_and_a_reading_too_big_is_not_kept() {
        let folder = Folder::new("lru");
        let cache = DiskCache::new(folder.0.clone(), "aaaa");
        cache.set_limit(40);
        let old = SystemTime::now() - Duration::from_secs(60);
        for n in 1..=3 {
            cache.put(&key(n), &[n; 10]);
            let at = cache.path(&key(n));
            fs::File::options()
                .write(true)
                .open(at)
                .unwrap()
                .set_modified(old + Duration::from_secs(n as u64))
                .unwrap();
        }
        // using the first makes the second the least recently used
        assert!(cache.get(&key(1)).is_some());
        cache.put(&key(4), &[4; 10]);
        cache.put(&key(5), &[5; 10]);
        assert_eq!(cache.stats(), (4, 40));
        assert!(cache.peek(&key(2)).is_none());
        assert!(cache.peek(&key(1)).is_some() && cache.peek(&key(3)).is_some());
        // more than a quarter of the limit
        cache.put(&key(6), &[6; 11]);
        assert!(cache.peek(&key(6)).is_none());
        // a lower limit removes the oldest at once, and 0 keeps nothing nor gives any
        cache.set_limit(20);
        assert_eq!(cache.stats().1, 20);
        cache.set_limit(0);
        assert_eq!(cache.stats(), (0, 0));
        cache.put(&key(7), b"x");
        assert_eq!(cache.stats(), (0, 0));
    }

    #[test]
    fn a_reading_removed_is_gone_and_a_missing_folder_is_an_empty_cache() {
        let folder = Folder::new("remove");
        let cache = DiskCache::new(folder.0.join("not-yet"), "aaaa");
        assert_eq!(cache.stats(), (0, 0));
        cache.put(&key(1), b"one");
        cache.remove(&key(1));
        assert_eq!(cache.get(&key(1)), None);
    }
}
