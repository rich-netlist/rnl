//! Files from the file system, and the package directories given on the command line (16.2). The
//! packages that ship with the tool come from `rnl-view`, which layers them under these.

use std::path::{Path, PathBuf};

use rnl_runtime::source::Provider;

pub struct FsProvider {
    dirs: Vec<PathBuf>,
}

impl FsProvider {
    pub fn new(dirs: &[PathBuf]) -> FsProvider {
        FsProvider { dirs: dirs.to_vec() }
    }
}

pub fn path_str(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

impl Provider for FsProvider {
    fn read(&self, path: &str) -> Option<String> {
        let bytes = std::fs::read(path).ok()?;
        Some(match String::from_utf8(bytes) {
            Ok(s) => s.strip_prefix('\u{feff}').map(String::from).unwrap_or(s),
            Err(e) => e.into_bytes().iter().map(|&b| b as char).collect(), // Latin-1
        })
    }

    /// The package directories given on the command line (16.2).
    fn package_files(&self) -> Vec<String> {
        let mut out = Vec::new();
        for d in &self.dirs {
            let mut files: Vec<PathBuf> = match std::fs::read_dir(d) {
                Ok(rd) => rd
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().is_some_and(|x| x == "rnl"))
                    .collect(),
                Err(_) => Vec::new(),
            };
            files.sort();
            out.extend(files.iter().map(|p| path_str(p)));
        }
        out
    }
}
