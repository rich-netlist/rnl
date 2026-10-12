//! Files from the file system, and the packages that ship with the tool (16.2, 16.3).

use std::path::{Path, PathBuf};

use rnl_runtime::source::{MemoryProvider, Provider};

/// The standard packages bundled with the tool.
const BUNDLED: [(&str, &str); 2] = [
    ("rnl-std.rnl", include_str!("../../../libraries/standard/rnl-std.rnl")),
    ("rnl-zh.rnl", include_str!("../../../libraries/standard/rnl-zh.rnl")),
];
const BUNDLED_DIR: &str = "<bundled>";

pub struct FsProvider {
    dirs: Vec<PathBuf>,
    bundled: MemoryProvider,
}

impl FsProvider {
    pub fn new(dirs: &[PathBuf]) -> FsProvider {
        let mut bundled = MemoryProvider { package_dirs: vec![BUNDLED_DIR.into()], ..MemoryProvider::default() };
        for (name, text) in BUNDLED {
            bundled.insert(&format!("{BUNDLED_DIR}/{name}"), text);
        }
        FsProvider { dirs: dirs.to_vec(), bundled }
    }
}

pub fn path_str(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

impl Provider for FsProvider {
    fn read(&self, path: &str) -> Option<String> {
        if path.starts_with(BUNDLED_DIR) {
            return self.bundled.read(path);
        }
        let bytes = std::fs::read(path).ok()?;
        Some(match String::from_utf8(bytes) {
            Ok(s) => s.strip_prefix('\u{feff}').map(String::from).unwrap_or(s),
            Err(e) => e.into_bytes().iter().map(|&b| b as char).collect(), // Latin-1
        })
    }

    /// The package directories given on the command line, then the bundled packages (16.2).
    fn package_files(&self) -> Vec<String> {
        let mut out = Vec::new();
        for d in &self.dirs {
            let mut files: Vec<PathBuf> = match std::fs::read_dir(d) {
                Ok(rd) => rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "rnl")).collect(),
                Err(_) => Vec::new(),
            };
            files.sort();
            out.extend(files.iter().map(|p| path_str(p)));
        }
        out.extend(self.bundled.package_files());
        out
    }
}
