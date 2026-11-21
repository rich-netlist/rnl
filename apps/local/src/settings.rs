//! The user's settings (the View menu's choices, the PATH folders, the solver's options), kept in a file of the user's own
//! (`$XDG_CONFIG_HOME/rnl-viewer-local/settings.json`, else `~/.config/…`), as the desktop viewer keeps its own. Not in the
//! browser: its storage is of one address, and this program's address changes with every run. The file is kept as the
//! desktop keeps its own (rnl_viewer_files::settings): two programs that keep it lose nothing of each other's.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rnl_viewer_files::settings;
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

impl Settings {
    /// The settings in `path`: those the file holds, none when it has none yet, and an error that says why when it holds
    /// something else than a JSON object (it is left as it is, for the user to look at).
    pub fn open(path: PathBuf) -> Result<Settings, String> {
        let values = settings::read(&path)?;
        Ok(Settings { path, values: Mutex::new(values) })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn values(&self) -> Map<String, Value> {
        self.values.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// Keep a setting: written at once, so that a closed window loses nothing, into what the file holds then.
    pub fn set(&self, key: String, value: Value) -> Result<(), String> {
        let mut values = self.values.lock().map_err(|e| e.to_string())?;
        *values = settings::update(&self.path, key, value)?;
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
        // another program's setting, written since, stays
        again.set("grid-shown".into(), Value::Bool(true)).unwrap();
        settings.set("usages-shown".into(), Value::Bool(true)).unwrap();
        let both = Settings::open(path.clone()).unwrap().values();
        assert_eq!((both["usages-shown"].clone(), both["grid-shown"].clone()), (Value::Bool(true), Value::Bool(true)));
        // not an object: refused, and left alone
        std::fs::write(&path, "[1, 2]").unwrap();
        assert!(Settings::open(path.clone()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"[1, 2]");
    }
}
