//! The public packages that this build bundles (scripts/bundle_packages.py), found without a network.
//!
//! The viewer never downloads a package. A build of the release that bundles them has them as files in
//! a folder `public-packages`: beside the program (the installers, and the zip of the single executable
//! with packages, which has the folder beside the executable), among its resources (the macOS bundle and
//! the AppImage with packages), or in `/usr/share/rnl-viewer` (the Debian package of the public packages). Nothing is copied or written: the program
//! alone, as the plain single executable is, has none. The folder is `<name>/<version>/…` for each
//! package, whose `<name>` is a directory of the search path that package lookup reads (16.2), a lock
//! file `public-packages.json`, and the licences. They are searched after the user's own PATH
//! directories, and are read only (rnl_viewer_files::bundled, as the local program reads them).

use rnl_viewer_files::bundled::{self, Info, DIR};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Find the packages of this build, once, at the start of the program.
pub fn init(app: &AppHandle) {
    ROOT.get_or_init(|| find(app));
}

fn find(app: &AppHandle) -> Option<PathBuf> {
    let beside = std::env::current_exe().ok().as_deref().and_then(Path::parent).map(|d| d.join(DIR));
    let resources = app.path().resource_dir().ok().map(|d| d.join(DIR));
    let system = PathBuf::from("/usr/share/rnl-viewer").join(DIR);
    bundled::find([resources, beside, Some(system)].into_iter().flatten())
}

/// The folder of the packages this build bundles, if it does.
pub fn root() -> Option<&'static Path> {
    ROOT.get().and_then(|r| r.as_deref())
}

/// The provider of files for a build: the user's PATH directories (`paths`) for packages and for bare includes,
/// and after them the directories of the bundled public packages for packages only, so that no bare include is
/// looked for in them, nor named among the places that were searched.
pub fn provider(paths: &[PathBuf]) -> rnl_cli::FsProvider {
    let packages: Vec<PathBuf> = paths.iter().cloned().chain(dirs()).collect();
    rnl_cli::FsProvider::new(&packages).with_search_paths(paths)
}

/// The directories of the bundled packages, one for each package, in the order of their names.
fn dirs() -> Vec<PathBuf> {
    root().map(bundled::directories).unwrap_or_default()
}

/// What this build bundles: nothing when it is the build without the public packages.
#[tauri::command]
pub fn public_packages() -> Info {
    root().map(|r| bundled::info(r, rnl_cli::path_str)).unwrap_or_default()
}
