//! What changed on disk under the page: the files of the document shown are looked at once a second, and the page asks
//! which of them changed since the last time it asked, to tell the user that File › Reload (F5) would show something
//! else (apps/viewer-ui/disk-changes.js). Polling, not the system's notifications: it also works on the network file
//! systems that servers mount, where those say nothing of what another machine wrote. The viewer's own writes are not
//! changes. The desktop app and the local program both watch so.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// How many changes are remembered for a page that asks late.
const LOG: usize = 500;

/// What a file is now, to tell a change by: its time of change and its length, or None when it is not there.
pub fn modified(path: &Path) -> Option<u64> {
    let meta = std::fs::metadata(path).ok()?;
    let t = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(t.as_millis() as u64 ^ meta.len().rotate_left(40))
}

#[derive(Default)]
struct Inner {
    stamps: HashMap<PathBuf, Option<u64>>,
    version: u64,
    log: Vec<(u64, PathBuf)>,
}

/// The files watched, and the changes seen, each under a version.
#[derive(Default)]
pub struct Watcher {
    inner: Mutex<Inner>,
}

impl Watcher {
    /// Start looking at a file the page read, from what it is now; a file watched already is left as it is.
    pub fn watch(&self, path: &Path) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.stamps.entry(path.to_path_buf()).or_insert_with(|| modified(path));
        }
    }

    /// The file was written by the viewer: its new state is not a change.
    pub fn wrote(&self, path: &Path) {
        if let Ok(mut inner) = self.inner.lock() {
            if inner.stamps.contains_key(path) {
                inner.stamps.insert(path.to_path_buf(), modified(path));
            }
        }
    }

    /// Look at every file once: those that changed, or went, or came back, are logged under a new version.
    pub fn poll(&self) {
        let paths: Vec<PathBuf> = match self.inner.lock() {
            Ok(inner) => inner.stamps.keys().cloned().collect(),
            Err(_) => return,
        };
        // (the file system is asked without the lock: a slow network mount does not hold up the page)
        let now: Vec<(PathBuf, Option<u64>)> =
            paths.into_iter().map(|p| (modified(&p), p)).map(|(m, p)| (p, m)).collect();
        let Ok(mut inner) = self.inner.lock() else { return };
        for (path, stamp) in now {
            if inner.stamps.get(&path) != Some(&stamp) {
                inner.stamps.insert(path.clone(), stamp);
                inner.version += 1;
                let version = inner.version;
                inner.log.push((version, path));
                if inner.log.len() > LOG {
                    inner.log.remove(0);
                }
            }
        }
    }

    /// The version now, and the files that changed after the version asked for, each once, as they were given to
    /// [`Watcher::watch`].
    pub fn since(&self, version: u64) -> (u64, Vec<PathBuf>) {
        let Ok(inner) = self.inner.lock() else { return (0, Vec::new()) };
        let mut changed: Vec<PathBuf> = Vec::new();
        for (v, path) in &inner.log {
            if *v > version && !changed.contains(path) {
                changed.push(path.clone());
            }
        }
        (inner.version, changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_is_seen_once_and_a_write_of_the_viewer_is_not_one() {
        let dir = std::env::temp_dir().join(format!("rnl-viewer-files-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.sp");
        std::fs::write(&file, "one").unwrap();
        let watcher = Watcher::default();
        watcher.watch(&file);
        watcher.poll();
        assert_eq!(watcher.since(0), (0, Vec::new()));
        // another program writes it (a different length, so that a coarse clock cannot hide it)
        std::fs::write(&file, "another text").unwrap();
        watcher.poll();
        let (version, changed) = watcher.since(0);
        assert_eq!(version, 1);
        assert_eq!(changed, vec![file.clone()]);
        assert!(watcher.since(1).1.is_empty(), "asked again after that version: nothing");
        // the viewer's own write
        std::fs::write(&file, "the viewer's text").unwrap();
        watcher.wrote(&file);
        watcher.poll();
        assert_eq!(watcher.since(1), (1, Vec::new()));
        // gone, then back
        std::fs::remove_file(&file).unwrap();
        watcher.poll();
        assert_eq!(watcher.since(1).0, 2);
        std::fs::write(&file, "back").unwrap();
        watcher.poll();
        assert_eq!(watcher.since(1).0, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
