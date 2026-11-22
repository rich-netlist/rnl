//! File selection for the session's library, and enumeration of PATH packages.
use rnl_cli::walk::Listing;
use std::path::Path;
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

/// Files dropped on the window, or opened with the app: the netlists and packages among them, the files of a
/// listed suffix (an unrelated file does not open), and with a folder among them, `library`: then they join the Library
/// as a folder included does, with the files of each folder ([`folder_files`]), as the web viewer takes a folder dropped.
/// The files, the folders listed in part and the limit are as every viewer gives a folder's listing
/// ([`rnl_view::folder::Listing`]), which the page says.
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Dropped {
    #[serde(flatten)]
    listed: Listing,
    library: bool,
}

#[tauri::command]
pub async fn dropped_files(paths: Vec<String>) -> Result<Dropped, String> {
    tauri::async_runtime::spawn_blocking(move || dropped(paths)).await.map_err(|e| e.to_string())?
}

fn dropped(paths: Vec<String>) -> Result<Dropped, String> {
    let (folders, others): (Vec<String>, Vec<String>) = paths.into_iter().partition(|p| Path::new(p).is_dir());
    let files = others.into_iter().filter(|name| Path::new(name).is_file() && rnl_view::is_listed(name)).collect();
    let mut listed = Listing { files, truncated: Vec::new(), limit: rnl_cli::walk::INCLUDE_FOLDER.limit };
    for folder in &folders {
        let found = folder_files(Path::new(folder))?;
        listed.files.extend(found.files);
        listed.truncated.extend(found.truncated);
    }
    Ok(Dropped { listed, library: !folders.is_empty() })
}

/// The netlists and packages a folder included in the Library brings in, as every viewer lists them
/// ([`rnl_cli::walk::folder_files`], [`rnl_cli::walk::INCLUDE_FOLDER`]): up to 8 levels of folders
/// below it and 2000 files, nothing hidden, no link out of it; with the folder's name in `truncated`
/// when it had more files than that.
pub fn folder_files(dir: &Path) -> Result<Listing, String> {
    let listed = rnl_cli::walk::folder_files(dir, rnl_cli::walk::INCLUDE_FOLDER)?;
    let files = listed.files.iter().map(|p| rnl_cli::path_str(p)).collect();
    Ok(Listing { files, truncated: listed.truncated, limit: listed.limit })
}

/// The files chosen for the Library, as a folder's listing is given ([`Listing`]): those of a folder included, with the
/// folder named in `truncated` when it had more files than are listed, which the page says; the files chosen; or the
/// folder chosen for PATH.
#[tauri::command]
pub async fn pick_pool(window: WebviewWindow, kind: String) -> Result<Listing, String> {
    let dialog = window.dialog().file().set_parent(&window);
    let filter = crate::menu::text(window.app_handle(), "Netlists and packages");
    let all = crate::menu::text(window.app_handle(), "All files");
    let whole = |files| Listing { files, truncated: Vec::new(), limit: rnl_cli::walk::INCLUDE_FOLDER.limit };
    tauri::async_runtime::spawn_blocking(move || {
        if kind == "folder" || kind == "path" {
            let Some(chosen) = dialog.blocking_pick_folder() else { return Ok(whole(Vec::new())) };
            let dir = chosen.into_path().map_err(|e| e.to_string())?;
            if kind == "path" {
                return Ok(whole(vec![rnl_cli::path_str(&dir)]));
            }
            folder_files(&dir)
        } else {
            let paths = dialog
                .add_filter(filter, &rnl_view::listed_suffixes())
                .add_filter(all, &["*"])
                .blocking_pick_files()
                .unwrap_or_default()
                .into_iter()
                .map(|f| f.into_path().map(|p| rnl_cli::path_str(&p)).map_err(|e| e.to_string()))
                .collect::<Result<_, _>>()?;
            Ok(whole(paths))
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
    fn native_drops_keep_existing_netlists_in_order() {
        let dir = std::env::temp_dir().join(format!("rnl-drop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let names = ["中文 电路.NETLIST", "part.SPI", "model.scs", "notes.txt"];
        for name in names {
            std::fs::write(dir.join(name), "R1 a 0 1k").unwrap();
        }
        let paths = names.into_iter().chain(["missing.sp"]).map(|name| rnl_cli::path_str(&dir.join(name))).collect();
        let kept = tauri::async_runtime::block_on(dropped_files(paths)).unwrap();
        let files = names[..3].iter().map(|name| rnl_cli::path_str(&dir.join(name))).collect::<Vec<_>>();
        assert_eq!(kept, Dropped { listed: Listing { files, truncated: Vec::new(), limit: 2000 }, library: false });
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
        let listed = folder_files(&dir).unwrap();
        let files = &listed.files;
        assert_eq!(files.len(), 3);
        assert!(listed.truncated.is_empty());
        assert!(files.iter().any(|p| p.ends_with("sub/parts.RNL")));
        assert!(files.windows(2).all(|w| w[0] <= w[1]));
        assert!(folder_files(&dir.join("missing")).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn a_folder_dropped_joins_the_library_as_a_folder_included_and_files_alone_open() {
        let dir = std::env::temp_dir().join(format!("rnl-dropped-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lib/.hidden")).unwrap();
        for name in ["top.sp", "notes.txt", "lib/cell.inc", "lib/.hidden/no.sp", "lib/readme.md"] {
            std::fs::write(dir.join(name), "text").unwrap();
        }
        let at = |name: &str| rnl_cli::path_str(&dir.join(name));
        // files alone: those of a listed suffix, to open
        assert_eq!(
            dropped(vec![at("top.sp"), at("notes.txt")]).unwrap(),
            Dropped {
                listed: Listing { files: vec![at("top.sp")], truncated: Vec::new(), limit: 2000 },
                library: false
            }
        );
        // a folder among them: the files dropped and those the folder lists, for the Library
        let listed = folder_files(&dir.join("lib")).unwrap().files;
        assert_eq!(listed.len(), 1);
        assert!(listed[0].ends_with("cell.inc"));
        assert_eq!(
            dropped(vec![at("top.sp"), at("lib")]).unwrap(),
            Dropped {
                listed: Listing { files: [vec![at("top.sp")], listed].concat(), truncated: Vec::new(), limit: 2000 },
                library: true
            }
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
