//! Independent viewer processes exchange a one-use JSON session, never editor buffers for disk files.
use serde_json::Value;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{State, WebviewWindow};

pub struct Initial(pub Mutex<Option<Value>>);

pub fn read(args: impl IntoIterator<Item = OsString>) -> Result<Option<Value>, String> {
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg != "--session" {
            continue;
        }
        let path = PathBuf::from(args.next().ok_or("--session needs a JSON file")?);
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        // Delete even a malformed session after reading: it may contain clipboard text.
        std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        return serde_json::from_slice(&bytes).map(Some).map_err(|e| format!("invalid viewer session: {e}"));
    }
    Ok(None)
}

#[tauri::command]
pub fn initial_session(state: State<'_, Initial>) -> Result<Option<Value>, String> {
    Ok(state.0.lock().map_err(|e| e.to_string())?.take())
}

#[tauri::command]
pub fn new_window(window: WebviewWindow, mut session: Value) -> Result<u32, String> {
    let position = window.outer_position().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    session["position"] =
        serde_json::json!({ "x": position.x as f64 / scale + 24.0, "y": position.y as f64 / scale + 24.0 });
    let stamp =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let path = std::env::temp_dir().join(format!("rnl-viewer-session-{}-{stamp}.json", std::process::id()));
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(&serde_json::to_vec(&session).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        drop(file);
        let mut command = std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
        command.arg("--session").arg(&path);
        // The native smoke test inspects the child through its own optional DevTools port.
        if let Some(port) = session.get("testPort").and_then(Value::as_u64).filter(|p| *p > 0 && *p <= u16::MAX as u64)
        {
            command.arg(format!("--remote-debugging-port={port}"));
        }
        // Each independent WebView needs a profile of its own; smoke tests may supply a parent profile.
        if let Some(profile) = std::env::var_os("WEBVIEW2_USER_DATA_FOLDER") {
            command.env("WEBVIEW2_USER_DATA_FOLDER", PathBuf::from(profile).join(format!("window-{stamp}")));
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW: debug builds need no console either
        }
        command.spawn().map(|child| child.id()).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_session_is_read_once_and_removed_including_malformed_json() {
        let path = std::env::temp_dir().join(format!("rnl-session-test-{}.json", std::process::id()));
        std::fs::write(&path, r#"{"entry":{"path":"/a.sp"},"level":"cell","pool":[]}"#).unwrap();
        let args = || vec![OsString::from("--session"), path.clone().into_os_string()];
        assert_eq!(read(args()).unwrap().unwrap()["level"], "cell");
        assert!(!path.exists());
        std::fs::write(&path, "broken").unwrap();
        assert!(read(args()).is_err());
        assert!(!path.exists());
    }
}
