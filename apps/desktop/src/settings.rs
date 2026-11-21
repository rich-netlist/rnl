//! User settings live in Tauri's application configuration directory, never beside the binary, in a file that
//! rnl_viewer_files::settings keeps, as the local program keeps its own.
use rnl_viewer_files::settings;
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
pub struct Settings(pub Mutex<Option<(PathBuf, Map<String, Value>)>>);

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
        let values = settings::load(&path, legacy)?;
        *state = Some((path, values));
    }
    let (path, values) = state.as_ref().unwrap();
    Ok(serde_json::json!({ "path": path, "values": values }))
}

#[tauri::command]
pub fn save_setting(state: State<'_, Settings>, key: String, value: Value) -> Result<(), String> {
    let mut state = state.0.lock().map_err(|e| e.to_string())?;
    let (path, values) = state.as_mut().ok_or("settings have not been loaded")?;
    *values = settings::update(path, key, value)?;
    Ok(())
}

impl Settings {
    /// The PATH directories in order, each followed by its sub-folders when it was added with them
    /// (`search-paths-subfolders`, [`rnl_cli::walk::subfolders`]), as package and include lookup searches them. The public packages that
    /// this build bundles are searched for packages only, after these (`public_packages::provider`).
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
            let sub = deep.contains(&dir).then(|| rnl_cli::walk::subfolders(&dir, rnl_cli::walk::PATH_SUBFOLDERS).0);
            out.push(dir);
            out.extend(sub.into_iter().flatten());
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_added_with_subfolders_is_followed_by_them() {
        let dir = std::env::temp_dir().join(format!("rnl-subfolders-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["b/x", "a", ".git/objects", "c/0.3", "c/d"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        // a directory added without its sub-folders stays alone; one added with them is followed by them, as the
        // core walks them (rnl_cli::walk::subfolders)
        let flat = std::env::temp_dir().join(format!("rnl-subfolders-flat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&flat);
        for i in 0..3 {
            std::fs::create_dir_all(flat.join(format!("d{i:04}"))).unwrap();
        }
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
}
