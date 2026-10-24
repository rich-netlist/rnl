//! User settings live in Tauri's application configuration directory, never beside the binary.
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
pub struct Settings(pub Mutex<Option<(PathBuf, Map<String, Value>)>>);

fn write(path: &Path, values: &Map<String, Value>) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or("settings have no parent directory")?).map_err(|e| e.to_string())?;
    let temp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec_pretty(values).map_err(|e| e.to_string())?;
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, path).map_err(|e| format!("cannot save {}: {e}", path.display()))
}

fn read_or_migrate(path: &Path, legacy: Map<String, Value>) -> Result<Map<String, Value>, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("cannot read {}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            write(path, &legacy)?;
            Ok(legacy)
        }
        Err(e) => Err(format!("cannot read {}: {e}", path.display())),
    }
}

// OS advisory locks serialize the complete read/modify/replace across independent processes.
// Keep this file alive until the update is finished. Closing it releases the lock on all platforms.
fn lock(path: &Path) -> Result<std::fs::File, String> {
    std::fs::create_dir_all(path.parent().ok_or("settings have no parent directory")?).map_err(|e| e.to_string())?;
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

fn update(path: &Path, key: String, value: Value) -> Result<Map<String, Value>, String> {
    let _lock = lock(path)?;
    let mut next = read_or_migrate(path, Map::new())?;
    next.insert(key, value);
    write(path, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn load_settings(app: AppHandle, state: State<'_, Settings>, legacy: Map<String, Value>) -> Result<Value, String> {
    let mut state = state.0.lock().map_err(|e| e.to_string())?;
    if state.is_none() {
        let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        // Smoke tests isolate native settings as well as the WebView profile. Release builds
        // always use Tauri's application directory.
        #[cfg(debug_assertions)]
        let dir = std::env::var_os("RNL_VIEWER_TEST_CONFIG_DIR").map(PathBuf::from).unwrap_or(dir);
        let path = dir.join("settings.json");
        let _lock = lock(&path)?;
        let values = read_or_migrate(&path, legacy)?;
        *state = Some((path, values));
    }
    let (path, values) = state.as_ref().unwrap();
    Ok(serde_json::json!({ "path": path, "values": values }))
}

#[tauri::command]
pub fn save_setting(state: State<'_, Settings>, key: String, value: Value) -> Result<(), String> {
    let mut state = state.0.lock().map_err(|e| e.to_string())?;
    let (path, values) = state.as_mut().ok_or("settings have not been loaded")?;
    *values = update(path, key, value)?;
    Ok(())
}

impl Settings {
    pub fn preload_enabled(&self) -> Result<bool, String> {
        let state = self.0.lock().map_err(|e| e.to_string())?;
        Ok(state
            .as_ref()
            .ok_or("settings have not been loaded")?
            .1
            .get("background-preload")
            .and_then(Value::as_bool)
            .unwrap_or(true))
    }

    pub fn cache_limit(&self) -> Result<usize, String> {
        let state = self.0.lock().map_err(|e| e.to_string())?;
        let values = &state.as_ref().ok_or("settings have not been loaded")?.1;
        Ok(values
            .get("render-cache-mib")
            .and_then(Value::as_u64)
            .filter(|n| *n <= 4096)
            .map_or(crate::build_cache::DEFAULT_LIMIT, |n| n as usize * 1024 * 1024))
    }

    /// The PATH directories in order, each followed by its sub-folders when it was added with them
    /// (`search-paths-subfolders`), as package and include lookup searches them.
    pub fn paths(&self) -> Result<Vec<PathBuf>, String> {
        let state = self.0.lock().map_err(|e| e.to_string())?;
        let values = &state.as_ref().ok_or("settings have not been loaded")?.1;
        let list = |key: &str| -> Result<Vec<PathBuf>, String> {
            match values.get(key) {
                None => Ok(Vec::new()),
                Some(value) => serde_json::from_value(value.clone()).map_err(|e| format!("invalid {key}: {e}")),
            }
        };
        let (dirs, deep) = (list("search-paths")?, list("search-paths-subfolders")?);
        drop(state);
        let mut out = Vec::new();
        for dir in dirs {
            let sub = deep.contains(&dir).then(|| subfolders(&dir).0);
            out.push(dir);
            out.extend(sub.into_iter().flatten());
        }
        Ok(out)
    }
}

/// The most sub-folders one PATH directory added with them brings in; past it, the rest are not
/// searched, and the Library says so.
pub const SUBFOLDER_LIMIT: usize = 500;

/// The sub-folders of `dir`, depth first and by name, and whether there were more than
/// [`SUBFOLDER_LIMIT`]. Hidden folders (a name starting with `.`), links and the numeric version
/// folders of packages, which their parent's lookup reads already, are left out, with what is in them.
pub fn subfolders(dir: &Path) -> (Vec<PathBuf>, bool) {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else { return false };
        let mut children: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                !name.starts_with('.') && !rnl_runtime::source::version_directory(&name)
            })
            .map(|e| e.path())
            .collect();
        children.sort();
        for child in children {
            if out.len() == SUBFOLDER_LIMIT {
                return true;
            }
            out.push(child.clone());
            if walk(&child, out) {
                return true;
            }
        }
        false
    }
    let mut out = Vec::new();
    let more = walk(dir, &mut out);
    (out, more)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_added_with_subfolders_brings_them_in_depth_first_up_to_the_limit() {
        let dir = std::env::temp_dir().join(format!("rnl-subfolders-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["b/x", "a", ".git/objects", "c/0.3", "c/d"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        let (found, more) = subfolders(&dir);
        let names: Vec<String> =
            found.iter().map(|p| p.strip_prefix(&dir).unwrap().to_string_lossy().replace('\\', "/")).collect();
        assert_eq!(names, ["a", "b", "b/x", "c", "c/d"]);
        assert!(!more);
        // a directory added without its sub-folders stays alone; one added with them is followed by them
        let flat = std::env::temp_dir().join(format!("rnl-subfolders-flat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&flat);
        for i in 0..SUBFOLDER_LIMIT + 3 {
            std::fs::create_dir_all(flat.join(format!("d{i:04}"))).unwrap();
        }
        let (found, more) = subfolders(&flat);
        assert_eq!((found.len(), more), (SUBFOLDER_LIMIT, true));
        let settings = Settings(Mutex::new(Some((
            dir.join("settings.json"),
            serde_json::from_value(serde_json::json!({
                "search-paths": [flat, dir],
                "search-paths-subfolders": [dir],
            }))
            .unwrap(),
        ))));
        let paths = settings.paths().unwrap();
        assert_eq!(paths[..3], [flat.clone(), dir.clone(), dir.join("a")]);
        assert_eq!(paths.len(), 2 + 5);
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&flat).unwrap();
    }
    #[test]
    fn cache_limits_default_validate_and_disable() {
        for (value, expected) in [
            (Value::Null, 256),
            (serde_json::json!(-1), 256),
            (serde_json::json!(4097), 256),
            (serde_json::json!(0), 0),
            (serde_json::json!(128), 128),
            (serde_json::json!(4096), 4096),
        ] {
            let values = Map::from_iter([("render-cache-mib".to_string(), value)]);
            let settings = Settings(Mutex::new(Some((PathBuf::new(), values))));
            assert_eq!(settings.cache_limit().unwrap(), expected * 1024 * 1024);
        }
    }
    #[test]
    fn independent_writers_preserve_each_others_settings() {
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
    fn migration_persistence_and_corrupt_file_preservation() {
        let dir = std::env::temp_dir().join(format!("rnl-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let legacy =
            serde_json::from_value(serde_json::json!({"solve-options": "grid=2", "search-paths": ["/p"]})).unwrap();
        assert_eq!(read_or_migrate(&path, legacy).unwrap()["solve-options"], "grid=2");
        let mut values = read_or_migrate(&path, Map::new()).unwrap();
        values.insert("grid-shown".into(), Value::Bool(false));
        write(&path, &values).unwrap();
        assert_eq!(read_or_migrate(&path, Map::new()).unwrap(), values);
        std::fs::write(&path, "broken").unwrap();
        assert!(read_or_migrate(&path, Map::new()).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
