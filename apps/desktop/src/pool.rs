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
                (path.is_file()
                    && path.extension().is_some_and(|e| super::NETLISTS.iter().any(|x| e.eq_ignore_ascii_case(x))))
                .then_some(name)
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

pub fn folder_files(dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        let path = entry.path();
        if kind.is_dir() {
            folder_files(&path, out)?;
        } else if kind.is_file()
            && path.extension().is_some_and(|e| super::NETLISTS.iter().any(|x| e.eq_ignore_ascii_case(x)))
        {
            out.push(rnl_cli::path_str(&path));
        }
    }
    out.sort();
    Ok(())
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
            let mut paths = Vec::new();
            folder_files(&dir, &mut paths)?;
            Ok(paths)
        } else {
            dialog
                .add_filter(filter, &super::NETLISTS)
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

#[tauri::command]
pub fn bundled_files() -> std::collections::BTreeMap<String, String> {
    rnl_view::bundled().files
}

#[tauri::command]
pub fn path_packages(dir: String) -> Result<Vec<String>, String> {
    let path = Path::new(&dir);
    // Report directory errors to the Library; enumeration itself uses the lookup provider's rule.
    std::fs::read_dir(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "Directory not found.".to_owned()
        } else {
            e.to_string()
        }
    })?;
    let mut files = rnl_cli::provider::directory_packages(path);
    // Netlists follow the same direct/numeric-version layout as packages. Arbitrary child
    // directories remain opt-in through Include Folder, rather than recursively searching PATH.
    for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let p = entry.path();
        if p.is_file() && p.extension().is_some_and(|e| super::NETLISTS.iter().any(|x| e.eq_ignore_ascii_case(x))) {
            files.push(rnl_cli::path_str(&p));
        } else if p.is_dir() && rnl_runtime::source::version_directory(&entry.file_name().to_string_lossy()) {
            for file in std::fs::read_dir(&p).map_err(|e| e.to_string())? {
                let file = file.map_err(|e| e.to_string())?.path();
                if file.is_file()
                    && file.extension().is_some_and(|e| super::NETLISTS.iter().any(|x| e.eq_ignore_ascii_case(x)))
                {
                    files.push(rnl_cli::path_str(&file));
                }
            }
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
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
        assert_eq!(
            path_packages(rnl_cli::path_str(&dir)).unwrap(),
            [
                "0.3/cell.spi",
                "0.3/versioned.RNL",
                "cell.NETLIST",
                "cell.inc",
                "cell.lib",
                "cell.scs",
                "cell.sp",
                "direct.rnl"
            ]
            .map(|name| rnl_cli::path_str(&dir.join(name)))
        );
        assert_eq!(path_packages(rnl_cli::path_str(&dir.join("missing"))).unwrap_err(), "Directory not found.");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn include_folder_collects_nested_netlists_and_packages_in_stable_order() {
        let dir = std::env::temp_dir().join(format!("rnl-folder-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        for name in ["top.sp", "sub/models.inc", "sub/parts.RNL", "notes.txt"] {
            std::fs::write(dir.join(name), "text").unwrap();
        }
        let mut files = Vec::new();
        folder_files(&dir, &mut files).unwrap();
        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|p| p.ends_with("sub/parts.RNL")));
        assert!(files.windows(2).all(|w| w[0] <= w[1]));
        assert!(folder_files(&dir.join("missing"), &mut Vec::new()).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
