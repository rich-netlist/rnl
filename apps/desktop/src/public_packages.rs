//! The public packages that this build bundles (scripts/bundle_packages.py), found without a network.
//!
//! The viewer never downloads a package. A build of the release that bundles them has them as files in
//! a folder `public-packages`: beside the program (the installers, and the zip of the single executable
//! with packages, which has the folder beside the executable), among its resources (the macOS and Linux
//! bundles), or in `/usr/share/rnl-viewer` (a Debian package). Nothing is copied or written: the program
//! alone, as the plain single executable is, has none. The folder is `<name>/<version>/…` for each
//! package, whose `<name>` is a directory of the search path that package lookup reads (16.2), a lock
//! file `public-packages.json`, and the licences. They are searched after the user's own PATH
//! directories, and are read only.
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

/// The folder's name, beside the program or among its resources.
const DIR: &str = "public-packages";
/// The lock file of the folder, which says what is in it.
const LOCK: &str = "public-packages.json";
/// The folder of the licences, which is no package.
const LICENSES: &str = "LICENSES";
static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Find the packages of this build, once, at the start of the program.
pub fn init(app: &AppHandle) {
    ROOT.get_or_init(|| find(app));
}

fn find(app: &AppHandle) -> Option<PathBuf> {
    // a folder named by the environment, for tests and for a build put together by hand
    if let Some(dir) = std::env::var_os("RNL_PUBLIC_PACKAGES").map(PathBuf::from).filter(|d| usable(d)) {
        return Some(dir);
    }
    let beside = std::env::current_exe().ok().as_deref().and_then(Path::parent).map(|d| d.join(DIR));
    let resources = app.path().resource_dir().ok().map(|d| d.join(DIR));
    let system = PathBuf::from("/usr/share/rnl-viewer").join(DIR);
    [resources, beside, Some(system)].into_iter().flatten().find(|dir| usable(dir))
}

/// Whether `dir` is a folder of public packages: it has a lock file.
fn usable(dir: &Path) -> bool {
    dir.join(LOCK).is_file()
}

/// The folder of the packages this build bundles, if it does.
pub fn root() -> Option<&'static Path> {
    ROOT.get().and_then(|r| r.as_deref())
}

/// The directories of the bundled packages, one for each package, in the order of their names: what
/// the search path gets after the user's own directories.
pub fn dirs() -> Vec<PathBuf> {
    root().map(directories).unwrap_or_default()
}

fn directories(root: &Path) -> Vec<PathBuf> {
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
    name: String,
    version: String,
    kind: String,
    /// The package file, which the Library lists and opens.
    path: String,
}

/// What this build bundles, for the Library and Help > Public Packages.
#[derive(Serialize, Debug, Default, PartialEq)]
pub struct Info {
    bundled: bool,
    /// The catalog the packages were taken from, and the day they were.
    catalog: String,
    fetched: String,
    packages: Vec<Package>,
    /// The licence files, by their paths.
    licenses: Vec<String>,
}

/// The packages of a folder, by its lock file, and failing that by what it holds.
fn info(root: &Path) -> Info {
    let lock: serde_json::Value =
        std::fs::read(root.join(LOCK)).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
    let field = |v: &serde_json::Value, key: &str| v[key].as_str().unwrap_or_default().to_owned();
    let mut packages = Vec::new();
    for p in lock["packages"].as_array().into_iter().flatten() {
        let (name, version) = (field(p, "name"), field(p, "version"));
        let file = root.join(&name).join(&version).join(format!("{name}.rnl"));
        if file.is_file() {
            packages.push(Package { kind: field(p, "kind"), path: rnl_cli::path_str(&file), name, version });
        }
    }
    let licenses = lock["licenses"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|l| rnl_cli::path_str(&root.join(field(l, "path"))))
        .collect();
    Info {
        bundled: !packages.is_empty(),
        catalog: field(&lock, "catalog"),
        fetched: field(&lock, "fetched"),
        packages,
        licenses,
    }
}

/// What this build bundles: nothing when it is the build without the public packages.
#[tauri::command]
pub fn public_packages() -> Info {
    root().map(info).unwrap_or_default()
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
        let got = info(&dir);
        assert!(got.bundled);
        assert_eq!(
            got.packages.iter().map(|p| format!("{} {}", p.name, p.version)).collect::<Vec<_>>(),
            ["eda-style 0.1", "pdk-sky130 0.1"]
        );
        assert_eq!((got.catalog.as_str(), got.fetched.as_str()), ("https://example.test/catalog.json", "2026-10-27"));
        assert_eq!(got.licenses.len(), 1);
        // the licences are no package directory
        assert_eq!(directories(&dir), vec![dir.join("eda-style"), dir.join("pdk-sky130")]);
        assert_eq!(info(&scratch("empty")), Info::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
