//! Files from the file system, and the package directories given on the command line (16.2). The
//! packages that ship with the tool come from `rnl-view`, which layers them under these.

use std::path::{Path, PathBuf};

use rnl_runtime::source::Provider;

pub struct FsProvider {
    dirs: Vec<PathBuf>,
    search_paths: Vec<PathBuf>,
}

impl FsProvider {
    pub fn new(dirs: &[PathBuf]) -> FsProvider {
        FsProvider { dirs: dirs.to_vec(), search_paths: Vec::new() }
    }

    pub fn with_search_paths(mut self, dirs: &[PathBuf]) -> Self {
        self.search_paths = dirs.to_vec();
        self
    }
}

pub fn path_str(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

impl Provider for FsProvider {
    fn read(&self, path: &str) -> Option<String> {
        let bytes = std::fs::read(path).ok()?;
        Some(crate::codec::decode(&bytes))
    }

    /// The package directories given on the command line (16.2).
    fn package_files(&self) -> Vec<String> {
        let mut out = Vec::new();
        for d in &self.dirs {
            out.extend(directory_packages(d));
        }
        out
    }

    fn package_candidates(&self, name: &str) -> Vec<String> {
        let mut out = Vec::new();
        for d in &self.dirs {
            let mut files = directory_packages(d);
            rnl_runtime::source::prefer_filename(&mut files, name);
            let direct = path_str(&d.join(format!("{name}.rnl")));
            files.sort_by_key(|p| p != &direct);
            out.extend(files);
        }
        out
    }

    fn include_candidates(&self, name: &str) -> Vec<String> {
        self.search_paths.iter().map(|d| path_str(&d.join(name))).collect()
    }

    /// On Windows and macOS, whose file systems ignore case, in lower case.
    fn path_key(&self, path: &str) -> String {
        if cfg!(any(windows, target_os = "macos")) {
            path.to_lowercase()
        } else {
            path.to_string()
        }
    }
}

/// Direct files and one level of version directories, sorted for reproducible fallback order.
pub fn directory_packages(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|x| x.eq_ignore_ascii_case("rnl")) {
            out.push(path_str(&path));
        } else if entry.file_type().is_ok_and(|t| t.is_dir()) {
            let name = entry.file_name();
            let version = name.to_string_lossy();
            if rnl_runtime::source::version_directory(&version) {
                if let Ok(children) = std::fs::read_dir(path) {
                    out.extend(
                        children
                            .flatten()
                            .map(|e| e.path())
                            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("rnl")))
                            .map(|p| path_str(&p)),
                    );
                }
            }
        }
    }
    out.sort();
    out
}
