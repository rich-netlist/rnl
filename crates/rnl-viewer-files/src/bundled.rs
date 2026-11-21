//! The public packages that a build of the viewers bundles (scripts/bundle_packages.py), found without a network: a
//! folder `public-packages` that holds `<name>/<version>/…` for each package, whose `<name>` is a directory that package
//! lookup searches (16.2), a lock file `public-packages.json` that says what is in it, and the licences. The desktop app
//! and the local program read it so, and search its directories for packages only, after the user's own PATH. Nothing is
//! copied or written: the folder is read only.

use serde::Serialize;
use std::path::{Path, PathBuf};

/// The folder's name, beside the program or among its resources.
pub const DIR: &str = "public-packages";
/// The lock file of the folder, which says what is in it.
pub const LOCK: &str = "public-packages.json";
/// The folder of the licences, which is no package.
const LICENSES: &str = "LICENSES";

/// Whether `dir` is a folder of public packages: it has a lock file.
pub fn usable(dir: &Path) -> bool {
    dir.join(LOCK).is_file()
}

/// The folder of public packages: one named by the environment (`RNL_PUBLIC_PACKAGES`, for tests and a build put
/// together by hand), else the first of `places` that is one.
pub fn find(places: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    std::env::var_os("RNL_PUBLIC_PACKAGES")
        .map(PathBuf::from)
        .filter(|d| usable(d))
        .or_else(|| places.into_iter().find(|dir| usable(dir)))
}

/// The directories of the packages of the folder `root`, one for each package, in the order of their names.
pub fn directories(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else { return Vec::new() };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()) && e.file_name() != LICENSES)
        .map(|e| e.path())
        .collect();
    dirs.sort();
    dirs
}

/// One bundled package.
#[derive(Serialize, Debug, PartialEq)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub kind: String,
    /// The package file, which the Library lists and opens.
    pub path: String,
}

/// What a build bundles, for the Library and Help › Public Packages.
#[derive(Serialize, Debug, Default, PartialEq)]
pub struct Info {
    pub bundled: bool,
    /// The catalog the packages were taken from, and the day they were.
    pub catalog: String,
    pub fetched: String,
    pub packages: Vec<Package>,
    /// The licence files, by their paths.
    pub licenses: Vec<String>,
    /// The directories that package lookup searches, one for each package (`directories`), by their paths.
    pub dirs: Vec<String>,
}

/// The packages of the folder `root`, by its lock file, each path named by `named`.
pub fn info(root: &Path, named: impl Fn(&Path) -> String) -> Info {
    let lock: serde_json::Value =
        std::fs::read(root.join(LOCK)).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
    let field = |v: &serde_json::Value, key: &str| v[key].as_str().unwrap_or_default().to_owned();
    let mut packages = Vec::new();
    for p in lock["packages"].as_array().into_iter().flatten() {
        let (name, version) = (field(p, "name"), field(p, "version"));
        let file = root.join(&name).join(&version).join(format!("{name}.rnl"));
        if file.is_file() {
            packages.push(Package { kind: field(p, "kind"), path: named(&file), name, version });
        }
    }
    let licenses =
        lock["licenses"].as_array().into_iter().flatten().map(|l| named(&root.join(field(l, "path")))).collect();
    Info {
        bundled: !packages.is_empty(),
        catalog: field(&lock, "catalog"),
        fetched: field(&lock, "fetched"),
        packages,
        licenses,
        dirs: directories(root).iter().map(|d| named(d)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rnl-public-packages-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_folder_lists_its_packages_by_the_lock_and_gives_their_directories() {
        let dir = scratch("info");
        for (path, text) in
            [("eda-style/0.1/eda-style.rnl", "p"), ("pdk-sky130/0.1/pdk-sky130.rnl", "q"), ("LICENSES/MIT.txt", "m")]
        {
            let file = dir.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        }
        std::fs::write(
            dir.join(LOCK),
            r#"{"catalog":"https://example.test/catalog.json","fetched":"2026-10-27",
                "packages":[{"name":"eda-style","version":"0.1","kind":"extension"},{"name":"pdk-sky130","version":"0.1","kind":"pdk"},
                            {"name":"gone","version":"1.0","kind":"x"}],
                "licenses":[{"path":"LICENSES/MIT.txt"}]}"#,
        )
        .unwrap();
        let named = |p: &Path| p.to_string_lossy().replace('\\', "/");
        let got = info(&dir, named);
        assert!(got.bundled);
        assert_eq!(
            got.packages.iter().map(|p| format!("{} {}", p.name, p.version)).collect::<Vec<_>>(),
            ["eda-style 0.1", "pdk-sky130 0.1"]
        );
        assert_eq!((got.catalog.as_str(), got.fetched.as_str()), ("https://example.test/catalog.json", "2026-10-27"));
        assert_eq!(got.licenses.len(), 1);
        // the licences are no package directory
        assert_eq!(directories(&dir), vec![dir.join("eda-style"), dir.join("pdk-sky130")]);
        assert_eq!(got.dirs, vec![named(&dir.join("eda-style")), named(&dir.join("pdk-sky130"))]);
        assert_eq!(info(&scratch("empty"), named), Info::default());
        assert!(usable(&dir) && !usable(&scratch("none")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
