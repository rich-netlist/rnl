//! Files from the file system, and the package directories given on the command line (16.2). The
//! packages that ship with the tool come from `rnl-view`, which layers them under these.

use std::path::{Path, PathBuf};

use rnl_runtime::source::Provider;

pub struct FsProvider {
    dirs: Vec<PathBuf>,
    search_paths: Vec<PathBuf>,
    /// `--confine`: the one folder files are read from, as the file system has it, links followed.
    confine: Option<PathBuf>,
}

impl FsProvider {
    pub fn new(dirs: &[PathBuf]) -> FsProvider {
        FsProvider { dirs: dirs.to_vec(), search_paths: Vec::new(), confine: None }
    }

    pub fn with_search_paths(mut self, dirs: &[PathBuf]) -> Self {
        self.search_paths = dirs.to_vec();
        self
    }

    /// Read files only within the folder `dir` (`--confine`): a file outside it, as an include by an absolute path names,
    /// or one a link in it leads out to, is not found. An error when the folder is not there.
    pub fn confined_to(mut self, dir: &Path) -> std::io::Result<Self> {
        self.confine = Some(std::fs::canonicalize(dir)?);
        Ok(self)
    }

    /// Whether the file at `path` may be read: within the folder of `--confine`, when there is one.
    fn readable(&self, path: &str) -> bool {
        match &self.confine {
            None => true,
            Some(root) => std::fs::canonicalize(path).is_ok_and(|real| real.starts_with(root)),
        }
    }
}

pub fn path_str(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

impl Provider for FsProvider {
    fn read(&self, path: &str) -> Option<String> {
        if !self.readable(path) {
            return None;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_confined_provider_reads_nothing_outside_its_folder() {
        let dir = std::env::temp_dir().join(format!("rnl-confine-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("in/sub")).unwrap();
        std::fs::write(dir.join("in/sub/a.inc"), "* inside\n").unwrap();
        std::fs::write(dir.join("out.inc"), "* outside\n").unwrap();
        let p = FsProvider::new(&[]).confined_to(&dir.join("in")).unwrap();
        assert_eq!(p.read(&path_str(&dir.join("in/sub/a.inc"))).as_deref(), Some("* inside\n"));
        assert_eq!(p.read(&path_str(&dir.join("out.inc"))), None, "an absolute path out of the folder");
        assert_eq!(p.read(&path_str(&dir.join("in/../out.inc"))), None, "a path that climbs out of it");
        // without --confine, as before
        assert!(FsProvider::new(&[]).read(&path_str(&dir.join("out.inc"))).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
