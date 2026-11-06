//! The user's settings (the View menu's choices, the PATH folders, the solver's options), kept in a file of the user's own
//! (`$XDG_CONFIG_HOME/rnl-viewer-local/settings.json`, else `~/.config/…`), as the desktop viewer keeps its own. Not in the
//! browser: its storage is of one address, and this program's address changes with every run.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{Map, Value};

pub struct Settings {
    path: PathBuf,
    values: Mutex<Map<String, Value>>,
}

/// The file the settings are kept in.
pub fn default_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| crate::files::home().map(|h| h.join(".config")))
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    base.join("rnl-viewer-local").join("settings.json")
}

fn write(path: &Path, values: &Map<String, Value>) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or("the settings have no folder")?).map_err(|e| e.to_string())?;
    let temp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec_pretty(values).map_err(|e| e.to_string())?;
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, path).map_err(|e| format!("cannot save {}: {e}", path.display()))
}

impl Settings {
    /// The settings in `path`: those the file holds, none when it has none yet, and an error that says why when it holds
    /// something else than a JSON object (it is left as it is, for the user to look at).
    pub fn open(path: PathBuf) -> Result<Settings, String> {
        let values = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("cannot read {}: {e}", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Map::new(),
            Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
        };
        Ok(Settings { path, values: Mutex::new(values) })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn values(&self) -> Map<String, Value> {
        self.values.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// Keep a setting: written at once, so that a closed window loses nothing.
    pub fn set(&self, key: String, value: Value) -> Result<(), String> {
        let mut values = self.values.lock().map_err(|e| e.to_string())?;
        let mut next = values.clone();
        next.insert(key, value);
        write(&self.path, &next)?;
        *values = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_kept_between_runs_and_a_bad_file_is_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("rnl-local-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("settings.json");
        let settings = Settings::open(path.clone()).unwrap();
        assert!(settings.values().is_empty());
        settings.set("usages-shown".into(), Value::Bool(false)).unwrap();
        let again = Settings::open(path.clone()).unwrap();
        assert_eq!(again.values().get("usages-shown"), Some(&Value::Bool(false)));
        // not an object: refused, and left alone
        std::fs::write(&path, "[1, 2]").unwrap();
        assert!(Settings::open(path.clone()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"[1, 2]");
    }
}
