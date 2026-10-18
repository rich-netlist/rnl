//! Host-side solver configuration lookup. The solver itself never reads the filesystem.

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use rnl_solver::SolverOptions;

pub const CONFIG_NAME: &str = "rnl-solver.toml";

#[derive(Clone, Debug)]
pub struct LoadedConfig {
    pub options: SolverOptions,
    /// The selected file, or None for the embedded standard configuration.
    pub path: Option<PathBuf>,
    /// The original TOML, suitable for copying and editing.
    pub text: String,
}

impl LoadedConfig {
    pub fn source(&self) -> String {
        self.path.as_ref().map_or_else(|| "builtin".into(), |p| p.display().to_string())
    }
}

/// Explicit path, nonempty environment path, then the default in the executable/resource
/// directory. Only an absent default file permits the builtin fallback.
pub fn resolve_from(
    explicit: Option<&Path>,
    environment: Option<&OsStr>,
    default_directory: &Path,
    overrides: &str,
) -> Result<LoadedConfig, String> {
    let environment = environment.filter(|p| !p.is_empty()).map(PathBuf::from);
    let selected = explicit.map(Path::to_path_buf).or(environment);
    let default = selected.is_none();
    let path = selected.unwrap_or_else(|| default_directory.join(CONFIG_NAME));
    let (path, text) = match std::fs::read_to_string(&path) {
        Ok(text) => (Some(path), text),
        Err(e) if default && e.kind() == ErrorKind::NotFound => (None, rnl_solver::DEFAULT_CONFIG.to_string()),
        Err(e) => return Err(format!("solver config {}: {e}", path.display())),
    };
    let source = path.as_ref().map_or_else(|| "builtin".into(), |p| p.display().to_string());
    let options = SolverOptions::from_toml(&text)
        .map_err(|e| format!("solver config {source}: {e}"))?
        .with_overrides(overrides)
        .map_err(|e| format!("--solver-options: {e}"))?;
    Ok(LoadedConfig { options, path, text })
}

/// Desktop passes Tauri's resource directory; the CLI uses its executable's directory.
pub fn resolve(
    explicit: Option<&Path>,
    resource_directory: Option<&Path>,
    overrides: &str,
) -> Result<LoadedConfig, String> {
    let executable;
    let directory = match resource_directory {
        Some(path) => path,
        None => {
            executable = std::env::current_exe().map_err(|e| format!("cannot locate the executable: {e}"))?;
            executable.parent().ok_or("the executable has no parent directory")?
        }
    };
    resolve_from(explicit, std::env::var_os("RNL_SOLVER_CONFIG").as_deref(), directory, overrides)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rnl-config-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn file(&self, name: &str, text: impl AsRef<[u8]>) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, text).unwrap();
            path
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn lookup_precedence_and_rules_last() {
        let d = Directory::new();
        let file = d.file(CONFIG_NAME, "[rules]\nbeam = 2\n");
        let env = d.file("env.toml", "[rules]\nbeam = 3\n");
        let explicit = d.file("explicit.toml", "[rules]\nbeam = 5\n");
        let read = |path: Option<&Path>, environment: Option<&OsStr>, overrides| {
            resolve_from(path, environment, &d.0, overrides).unwrap()
        };
        assert_eq!(read(None, None, "").path, Some(file));
        assert_eq!(read(None, Some(OsStr::new("")), "").options.beam, 2);
        assert_eq!(read(None, Some(env.as_os_str()), "").options.beam, 3);
        assert_eq!(read(Some(&explicit), Some(env.as_os_str()), "beam=7").options.beam, 7);
    }

    #[test]
    fn only_missing_default_falls_back() {
        let d = Directory::new();
        assert!(resolve_from(None, None, &d.0, "").unwrap().path.is_none());
        let missing = d.0.join("missing.toml");
        assert!(resolve_from(Some(&missing), None, &d.0, "").is_err());
        assert!(resolve_from(None, Some(missing.as_os_str()), &d.0, "").is_err());
        for bytes in [b"\xff".as_slice(), b"[rules\n", b"[rules]\nbeam = 0\n"] {
            d.file(CONFIG_NAME, bytes);
            assert!(resolve_from(None, None, &d.0, "").is_err());
        }
        std::fs::remove_file(d.0.join(CONFIG_NAME)).unwrap();
        std::fs::create_dir(d.0.join(CONFIG_NAME)).unwrap();
        assert!(resolve_from(None, None, &d.0, "").is_err());
    }
}
