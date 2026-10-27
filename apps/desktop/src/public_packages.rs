//! The public packages that this build bundles (scripts/bundle_packages.py), found without a network.
//!
//! The viewer never downloads a package. A build of the release that bundles them has them as files:
//! the installers put them in a `public-packages` folder beside the program, the macOS and Linux
//! bundles among their resources, a Debian package in `/usr/share/rnl-viewer`, and the single-file
//! program of Windows carries them at its own end, which the viewer copies once to its data folder.
//! The folder is `<name>/<version>/…` for each package, whose `<name>` is a directory of the search
//! path that package lookup reads (16.2), a lock file `public-packages.json`, and the licences. They
//! are searched after the user's own PATH directories, and are read only.
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

/// The folder's name, beside the program or among its resources.
const DIR: &str = "public-packages";
/// The lock file of the folder, which says what is in it.
const LOCK: &str = "public-packages.json";
/// The folder of the licences, which is no package.
const LICENSES: &str = "LICENSES";
/// The end of a program that carries the packages: the magic, the lengths of the index and of the
/// archive (u64, little endian) and the archive's id, by which the copy is kept.
const MAGIC: &[u8; 8] = b"RNLPKG01";
const FOOTER: u64 = 40;

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
    let exe = std::env::current_exe().ok();
    let beside = exe.as_deref().and_then(Path::parent).map(|d| d.join(DIR));
    let resources = app.path().resource_dir().ok().map(|d| d.join(DIR));
    let system = PathBuf::from("/usr/share/rnl-viewer").join(DIR);
    for dir in [resources, beside, Some(system)].into_iter().flatten() {
        if usable(&dir) {
            return Some(dir);
        }
    }
    let data = app.path().app_local_data_dir().ok()?.join(DIR);
    match extract(exe.as_deref()?, &data) {
        Ok(found) => found,
        Err(e) => {
            eprintln!("The public packages of this program could not be read: {e}");
            None
        }
    }
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

/// The packages at the end of a program, copied into `data/<id>` unless they are there already; the
/// folder to search, or none for a program that has no packages.
fn extract(exe: &Path, data: &Path) -> Result<Option<PathBuf>, String> {
    let mut file = std::fs::File::open(exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    if len < FOOTER {
        return Ok(None);
    }
    let mut footer = [0u8; FOOTER as usize];
    file.seek(SeekFrom::End(-(FOOTER as i64))).map_err(|e| e.to_string())?;
    file.read_exact(&mut footer).map_err(|e| e.to_string())?;
    if &footer[..8] != MAGIC {
        return Ok(None);
    }
    let le = |range: std::ops::Range<usize>| u64::from_le_bytes(footer[range].try_into().expect("eight bytes"));
    let (table, payload) = (le(8..16), le(16..24));
    let id = std::str::from_utf8(&footer[24..40]).map_err(|_| "the packages have no id")?;
    if !id.bytes().all(|b| b.is_ascii_hexdigit()) || table > payload || payload > len - FOOTER {
        return Err("the packages at the end of the program are damaged".into());
    }
    let dest = data.join(id);
    if dest.join(".complete").is_file() {
        return Ok(Some(dest));
    }
    file.seek(SeekFrom::Start(len - FOOTER - payload)).map_err(|e| e.to_string())?;
    let mut bytes = vec![0u8; payload as usize];
    file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    let (body, index) = bytes.split_at((payload - table) as usize);
    let index: Vec<serde_json::Value> =
        serde_json::from_slice(index).map_err(|e| format!("the packages' index: {e}"))?;
    let partial = data.join(format!("{id}.partial"));
    let _ = std::fs::remove_dir_all(&partial);
    for entry in &index {
        let path = entry["path"].as_str().ok_or("an entry has no path")?;
        let (at, size) = (
            entry["offset"].as_u64().ok_or("an entry has no offset")? as usize,
            entry["size"].as_u64().ok_or("an entry has no size")? as usize,
        );
        let content = body
            .get(at..at.checked_add(size).ok_or("an entry is too long")?)
            .ok_or("an entry is outside the archive")?;
        if path.is_empty()
            || path.starts_with('/')
            || path.contains('\\')
            || path.split('/').any(|p| p.is_empty() || p == "..")
        {
            return Err(format!("invalid path in the packages: {path}"));
        }
        let target = partial.join(path);
        std::fs::create_dir_all(target.parent().ok_or("no parent")?).map_err(|e| e.to_string())?;
        std::fs::write(&target, content).map_err(|e| format!("{}: {e}", target.display()))?;
    }
    std::fs::write(partial.join(".complete"), id).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::rename(&partial, &dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    // the copies of other releases are not needed again
    for other in std::fs::read_dir(data).into_iter().flatten().flatten() {
        if other.path() != dest {
            let _ = std::fs::remove_dir_all(other.path());
        }
    }
    Ok(Some(dest))
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

    /// A program with an archive at its end, as scripts/bundle_packages.py writes it.
    fn program(dir: &Path, files: &[(&str, &str)], id: &str) -> PathBuf {
        let (mut body, mut index) = (Vec::new(), Vec::new());
        for (path, text) in files {
            index.push(serde_json::json!({"path": path, "offset": body.len(), "size": text.len()}));
            body.extend_from_slice(text.as_bytes());
        }
        let table = serde_json::to_vec(&index).unwrap();
        let payload_len = (body.len() + table.len()) as u64;
        let mut bytes = b"MZ not much of a program".to_vec();
        bytes.extend(body);
        bytes.extend(&table);
        bytes.extend_from_slice(MAGIC);
        bytes.extend((table.len() as u64).to_le_bytes());
        bytes.extend(payload_len.to_le_bytes());
        bytes.extend_from_slice(id.as_bytes());
        let exe = dir.join("viewer.exe");
        std::fs::write(&exe, bytes).unwrap();
        exe
    }

    #[test]
    fn a_program_without_packages_has_none_and_one_with_them_is_copied_once() {
        let dir = scratch("extract");
        let plain = dir.join("plain.exe");
        std::fs::write(&plain, vec![7u8; 4000]).unwrap();
        assert_eq!(extract(&plain, &dir.join("data")), Ok(None));

        let files = [
            ("eda-style/0.1/eda-style.rnl", "package"),
            ("eda-style/0.1/examples/a.sp", "* a"),
            (
                "public-packages.json",
                r#"{"packages":[{"name":"eda-style","version":"0.1","kind":"extension"}],"licenses":[]}"#,
            ),
        ];
        let exe = program(&dir, &files, "0123456789abcdef");
        let found = extract(&exe, &dir.join("data")).unwrap().unwrap();
        assert_eq!(found, dir.join("data").join("0123456789abcdef"));
        assert_eq!(std::fs::read_to_string(found.join("eda-style/0.1/examples/a.sp")).unwrap(), "* a");
        assert!(usable(&found));
        // kept: what is there is not written again
        std::fs::write(found.join("eda-style/0.1/eda-style.rnl"), "kept").unwrap();
        assert_eq!(extract(&exe, &dir.join("data")).unwrap().unwrap(), found);
        assert_eq!(std::fs::read_to_string(found.join("eda-style/0.1/eda-style.rnl")).unwrap(), "kept");
        // the copies of another release go
        let newer = program(&dir, &files, "fedcba9876543210");
        let found = extract(&newer, &dir.join("data")).unwrap().unwrap();
        assert_eq!(std::fs::read_dir(dir.join("data")).unwrap().count(), 1);
        assert!(found.ends_with("fedcba9876543210"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_path_outside_the_folder_is_refused() {
        let dir = scratch("unsafe");
        for bad in ["../x", "/x", "a\\b", "a//b", ""] {
            let exe = program(&dir, &[(bad, "x")], "0123456789abcdef");
            assert!(extract(&exe, &dir.join("data")).is_err(), "{bad}");
        }
        assert!(!dir.join("x").exists());
        let _ = std::fs::remove_dir_all(&dir);
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
