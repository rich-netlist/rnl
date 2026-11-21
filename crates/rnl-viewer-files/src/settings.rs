//! The user's settings file, for every viewer that keeps one on the machine (the desktop app in its configuration folder,
//! the local program in `~/.config/rnl-viewer-local`): a JSON object, read once, and each setting written into it at once.
//! Two windows, or two programs, may keep the same file: a setting is written under a lock of the file's own
//! (`settings.lock`), into what the file holds then, so neither loses what the other wrote. A file that is not a JSON object
//! is an error, and it is left as it is, for the user to look at.

use std::path::Path;

use serde_json::{Map, Value};

/// The settings of the file, as one setting is kept.
pub type Values = Map<String, Value>;

/// Replace the file at `path` with `values` whole, through a file beside it.
pub fn write(path: &Path, values: &Values) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or("the settings have no folder")?).map_err(|e| e.to_string())?;
    let temp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec_pretty(values).map_err(|e| e.to_string())?;
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, path).map_err(|e| format!("cannot save {}: {e}", path.display()))
}

fn read_file(path: &Path) -> Result<Option<Values>, String> {
    match std::fs::read(path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map(Some).map_err(|e| format!("cannot read {}: {e}", path.display()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("cannot read {}: {e}", path.display())),
    }
}

/// The settings the file at `path` holds: none when there is no file yet, which is not made.
pub fn read(path: &Path) -> Result<Values, String> {
    Ok(read_file(path)?.unwrap_or_default())
}

/// The settings the file at `path` holds; when there is no file yet, `first`, which is written to it (the settings a
/// viewer kept elsewhere before, or none).
pub fn read_or_migrate(path: &Path, first: Values) -> Result<Values, String> {
    match read_file(path)? {
        Some(values) => Ok(values),
        None => {
            write(path, &first)?;
            Ok(first)
        }
    }
}

/// The lock of the settings file at `path`, held until it is dropped. The operating system's advisory lock keeps the whole
/// read, change and replace of one program from meeting another's; closing the file lets it go, on every system.
pub fn lock(path: &Path) -> Result<std::fs::File, String> {
    std::fs::create_dir_all(path.parent().ok_or("the settings have no folder")?).map_err(|e| e.to_string())?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.with_extension("lock"))
        .map_err(|e| e.to_string())?;
    file.lock().map_err(|e| e.to_string())?;
    Ok(file)
}

/// Read the settings at `path` under its lock, as [`read_or_migrate`] reads them.
pub fn load(path: &Path, first: Values) -> Result<Values, String> {
    let _lock = lock(path)?;
    read_or_migrate(path, first)
}

/// Keep the setting `key` in the file at `path`: what the file holds now, with it, written at once under its lock, so that
/// a closed window loses nothing and another writer's settings stay. Gives what the file holds after it.
pub fn update(path: &Path, key: String, value: Value) -> Result<Values, String> {
    let _lock = lock(path)?;
    let mut next = read_or_migrate(path, Map::new())?;
    next.insert(key, value);
    write(path, &next)?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_writers_keep_each_others_settings() {
        let dir = std::env::temp_dir().join(format!("rnl-settings-writers-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let start = std::sync::Arc::new(std::sync::Barrier::new(4));
        let writers: Vec<_> = (0..4)
            .map(|n| {
                let (path, start) = (path.clone(), start.clone());
                std::thread::spawn(move || {
                    start.wait();
                    for value in 0..8 {
                        update(&path, format!("window-{n}"), value.into()).unwrap();
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }
        let saved = read_or_migrate(&path, Map::new()).unwrap();
        for n in 0..4 {
            assert_eq!(saved[&format!("window-{n}")], 7);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn migration_persistence_and_a_bad_file_left_alone() {
        let dir = std::env::temp_dir().join(format!("rnl-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let legacy =
            serde_json::from_value(serde_json::json!({"solve-options": "grid=2", "search-paths": ["/p"]})).unwrap();
        assert_eq!(load(&path, legacy).unwrap()["solve-options"], "grid=2");
        let mut values = read_or_migrate(&path, Map::new()).unwrap();
        values.insert("grid-shown".into(), Value::Bool(false));
        write(&path, &values).unwrap();
        assert_eq!(read_or_migrate(&path, Map::new()).unwrap(), values);
        // not an object: refused, and left alone
        for bad in ["broken", "[1, 2]"] {
            std::fs::write(&path, bad).unwrap();
            assert!(read_or_migrate(&path, Map::new()).is_err());
            assert!(update(&path, "k".into(), Value::Null).is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), bad);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
