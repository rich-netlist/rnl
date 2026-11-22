//! File selection for the session's library, and enumeration of PATH packages.
use std::path::Path;
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

/// Keep native file drops to actual netlists/packages; directories and unrelated files do not open.
#[tauri::command]
pub async fn dropped_files(paths: Vec<String>) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .into_iter()
            .filter_map(|name| {
                let path = Path::new(&name);
                (path.is_file() && rnl_view::is_listed(&name)).then_some(name)
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

/// The netlists and packages a folder included in the Library brings in, as every viewer lists them
/// ([`rnl_cli::walk::folder_files`], [`rnl_cli::walk::INCLUDE_FOLDER`]): up to 8 levels of folders
/// below it and 2000 files, nothing hidden, no link out of it.
pub fn folder_files(dir: &Path) -> Result<Vec<String>, String> {
    let listed = rnl_cli::walk::folder_files(dir, rnl_cli::walk::INCLUDE_FOLDER)?;
    Ok(listed.files.iter().map(|p| rnl_cli::path_str(p)).collect())
}

#[tauri::command]
pub async fn pick_pool(window: WebviewWindow, kind: String) -> Result<Vec<String>, String> {
    let dialog = window.dialog().file().set_parent(&window);
    let filter = crate::menu::text(window.app_handle(), "Netlists and packages");
    let all = crate::menu::text(window.app_handle(), "All files");
    tauri::async_runtime::spawn_blocking(move || {
        if kind == "folder" || kind == "path" {
            let Some(chosen) = dialog.blocking_pick_folder() else { return Ok(Vec::new()) };
            let dir = chosen.into_path().map_err(|e| e.to_string())?;
            if kind == "path" {
                return Ok(vec![rnl_cli::path_str(&dir)]);
            }
            folder_files(&dir)
        } else {
            dialog
                .add_filter(filter, &rnl_view::listed_suffixes())
                .add_filter(all, &["*"])
                .blocking_pick_files()
                .unwrap_or_default()
                .into_iter()
                .map(|f| f.into_path().map(|p| rnl_cli::path_str(&p)).map_err(|e| e.to_string()))
                .collect()
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The bundled packages the Library lists as built in, as the core lists them for every viewer
/// ([`rnl_view::bundled_listed`]): all but those of a draft series, which the core reads all the same.
#[tauri::command]
pub fn bundled_files() -> std::collections::BTreeMap<String, String> {
    rnl_view::bundled_listed()
}

/// What a PATH directory lists in the Library: its packages and netlists, the directories lookup
/// searches for it, and whether it was added with sub-folders that are more than lookup searches
/// ([`rnl_cli::walk::PathListing`]).
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct PathFiles {
    files: Vec<String>,
    dirs: Vec<String>,
    truncated: bool,
}

/// The packages and netlists of a PATH directory, as lookup finds them, and with `subfolders` those
/// of each sub-folder that the settings bring in with it, as every viewer lists them
/// ([`rnl_cli::walk::path_listing`]).
#[tauri::command]
pub fn path_packages(dir: String, subfolders: Option<bool>) -> Result<PathFiles, String> {
    let listing = rnl_cli::walk::path_listing(Path::new(&dir), subfolders == Some(true))?;
    let names = |paths: &[std::path::PathBuf]| paths.iter().map(|p| rnl_cli::path_str(p)).collect();
    Ok(PathFiles { files: names(&listing.files), dirs: names(&listing.dirs), truncated: listing.truncated })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_drops_keep_existing_netlists_in_order_and_ignore_directories() {
        let dir = std::env::temp_dir().join(format!("rnl-drop-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("folder.sp")).unwrap();
        let names = ["中文 电路.NETLIST", "part.SPI", "model.scs", "notes.txt"];
        for name in names {
            std::fs::write(dir.join(name), "R1 a 0 1k").unwrap();
        }
        let paths = names
            .into_iter()
            .chain(["folder.sp", "missing.sp"])
            .map(|name| rnl_cli::path_str(&dir.join(name)))
            .collect();
        let kept = tauri::async_runtime::block_on(dropped_files(paths)).unwrap();
        assert_eq!(kept, names[..3].iter().map(|name| rnl_cli::path_str(&dir.join(name))).collect::<Vec<_>>());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn path_packages_matches_lookup_direct_and_version_directories_only() {
        let dir = std::env::temp_dir().join(format!("rnl-path-packages-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("0.3")).unwrap();
        std::fs::create_dir_all(dir.join("other")).unwrap();
        for name in [
            "direct.rnl",
            "0.3/versioned.RNL",
            "other/ignored.rnl",
            "notes.txt",
            "cell.NETLIST",
            "0.3/cell.spi",
            "cell.sp",
            "cell.scs",
            "cell.inc",
            "cell.lib",
            "other/ignored.sp",
        ] {
            std::fs::write(dir.join(name), "text").unwrap();
        }
        let direct = [
            "0.3/cell.spi",
            "0.3/versioned.RNL",
            "cell.NETLIST",
            "cell.inc",
            "cell.lib",
            "cell.scs",
            "cell.sp",
            "direct.rnl",
        ];
        let listed = |names: &[&str]| names.iter().map(|name| rnl_cli::path_str(&dir.join(name))).collect::<Vec<_>>();
        let top = rnl_cli::path_str(&dir);
        assert_eq!(
            path_packages(top.clone(), None).unwrap(),
            PathFiles { files: listed(&direct), dirs: vec![top.clone()], truncated: false }
        );
        // with its sub-folders, those of `other` too, which lookup searches after it
        let mut deep = direct.to_vec();
        deep.extend(["other/ignored.rnl", "other/ignored.sp"]);
        deep.sort();
        assert_eq!(
            path_packages(top.clone(), Some(true)).unwrap(),
            PathFiles {
                files: listed(&deep),
                dirs: vec![top, rnl_cli::path_str(&dir.join("other"))],
                truncated: false
            }
        );
        assert_eq!(path_packages(rnl_cli::path_str(&dir.join("missing")), None).unwrap_err(), "Directory not found.");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn include_folder_collects_nested_netlists_and_packages_in_stable_order() {
        let dir = std::env::temp_dir().join(format!("rnl-folder-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        for name in ["top.sp", "sub/models.inc", "sub/parts.RNL", "notes.txt"] {
            std::fs::write(dir.join(name), "text").unwrap();
        }
        let files = folder_files(&dir).unwrap();
        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|p| p.ends_with("sub/parts.RNL")));
        assert!(files.windows(2).all(|w| w[0] <= w[1]));
        assert!(folder_files(&dir.join("missing")).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
