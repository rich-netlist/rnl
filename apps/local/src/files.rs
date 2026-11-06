//! The files of the machine the program runs on that the page may read and write, and what it may not.
//!
//! The server runs as the user who started it, so what it could read is all that user can; the page may read and
//! write only below the folders the user has *allowed*: the folder of each file opened and the one above it, the
//! folders named with `--allow`, and the PATH folders of the Library. Nothing hidden (a name that starts with a
//! dot) below an allowed folder is served, so that a `.ssh` or `.git` beside a netlist is not. A link that leaves
//! an allowed folder does not count: a path is checked as the file system resolves it.
//!
//! Saving is the desktop's (decision 0011), written again here with the same words and the same checks: each file
//! in the encoding it was read in, its line ends as the page restored them, and only if the file on disk still is
//! what the page read, or the user said to overwrite it.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rnl_cli::codec::{self, OutputEncoding};
use serde::{Deserialize, Serialize};

/// The folders the page may use.
#[derive(Default)]
pub struct Roots {
    dirs: Mutex<Vec<PathBuf>>,
}

/// A path as the page names it: rooted, `/`-separated; on Windows, which only a development build runs on, `C:` is `/C:/a/b`.
pub fn page_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) {
        format!("/{}", text.replace('\\', "/"))
    } else {
        text.into_owned()
    }
}

/// The path on this machine of a path as the page names it: the inverse of [`page_path`].
pub fn machine_path(text: &str) -> PathBuf {
    if cfg!(windows) && text.as_bytes().get(2) == Some(&b':') {
        PathBuf::from(text[1..].replace('/', "\\"))
    } else {
        PathBuf::from(text)
    }
}

/// The user's home folder, as the environment names it.
pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).filter(|p| p.is_absolute())
}

/// The folder of a path that is a file, as it is on disk now.
fn canonical(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok().map(simple)
}

/// A path without the `\\?\` prefix Windows gives a canonical one, so that paths compare and show as the user wrote them.
fn simple(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path
}

impl Roots {
    /// Allow a folder, and everything below it that is not hidden. A folder already allowed, or below one, adds nothing.
    pub fn allow(&self, dir: &Path) -> Result<(), String> {
        let dir = canonical(dir).filter(|d| d.is_dir()).ok_or_else(|| format!("{} is not a folder", dir.display()))?;
        let mut dirs = self.dirs.lock().map_err(|e| e.to_string())?;
        if !dirs.iter().any(|d| dir.starts_with(d)) {
            dirs.retain(|d| !d.starts_with(&dir));
            dirs.push(dir);
        }
        Ok(())
    }

    /// Allow the folder of a file, and the one above it: a netlist includes what is beside it and in `../models`. Not the
    /// folder above when that is the user's home folder, or holds it, or is the root of the file system: that is a
    /// choice the user makes with `--allow`.
    pub fn allow_file(&self, file: &Path) -> Result<(), String> {
        let file = canonical(file).ok_or_else(|| format!("{} cannot be read", file.display()))?;
        let dir = file.parent().ok_or_else(|| format!("{} has no folder", file.display()))?.to_path_buf();
        self.allow(&dir)?;
        if let Some(up) = dir.parent() {
            let broad = up.parent().is_none() || home().and_then(|h| canonical(&h)).is_some_and(|h| h.starts_with(up));
            if !broad {
                self.allow(up)?;
            }
        }
        Ok(())
    }

    /// The folders allowed, as they are.
    pub fn list(&self) -> Vec<PathBuf> {
        self.dirs.lock().map(|d| d.clone()).unwrap_or_default()
    }

    /// The path as the file system resolves it, when the page may use it: below an allowed folder, with nothing hidden
    /// between that folder and the file. `must_exist` false is for a file to be written, whose folder must exist.
    pub fn check(&self, path: &Path, must_exist: bool) -> Result<PathBuf, String> {
        if !path.is_absolute() {
            return Err("the path is not absolute".into());
        }
        let resolved = if must_exist {
            canonical(path).ok_or_else(|| format!("{} cannot be read", path.display()))?
        } else {
            let name = path.file_name().ok_or("the path names no file")?;
            let dir = canonical(path.parent().ok_or("the path has no folder")?).ok_or("the folder does not exist")?;
            dir.join(name)
        };
        let dirs = self.list();
        let root = dirs.iter().find(|d| resolved.starts_with(d)).ok_or_else(|| {
            format!("{} is not in a folder the program was told to allow; start it with --allow FOLDER to add one", resolved.display())
        })?;
        let below = resolved.strip_prefix(root).unwrap_or(&resolved);
        if below.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.')) {
            return Err("a hidden file or folder is not served".into());
        }
        Ok(resolved)
    }
}

/// One entry of a folder, for the file chooser of the page.
#[derive(Debug, Serialize)]
pub struct Entry {
    pub name: String,
    pub dir: bool,
    pub size: u64,
    /// Milliseconds since 1970, or 0 when the system does not say.
    pub modified: u64,
}

/// What a folder holds, folders first, each in the order of its names: for the chooser. Hidden names are left out unless
/// `hidden`. A folder that cannot be listed is an error that says why.
pub fn list_dir(dir: &Path, hidden: bool) -> Result<(PathBuf, Vec<Entry>), String> {
    let real = canonical(dir).filter(|d| d.is_dir()).ok_or_else(|| format!("{} is not a folder", dir.display()))?;
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&real).map_err(|e| format!("{}: {e}", real.display()))? {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        if !hidden && name.starts_with('.') {
            continue;
        }
        // (a link to a folder is a folder, as the chooser follows it)
        let Ok(meta) = std::fs::metadata(entry.path()) else { continue };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis() as u64);
        entries.push(Entry { name, dir: meta.is_dir(), size: meta.len(), modified });
    }
    entries.sort_by(|a, b| b.dir.cmp(&a.dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok((real, entries))
}

/// The modification time of a file in milliseconds since 1970: what the program watches for a change; None when it is gone.
pub fn modified(path: &Path) -> Option<u64> {
    let meta = std::fs::metadata(path).ok()?;
    let t = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(t.as_millis() as u64 ^ meta.len().rotate_left(40))
}

/// A file to save: the path the Document names it by, its name as shown, and its text as the file has it. `read` is the
/// text of the file as the page read it, which the file on disk must still be, unless the user chose to overwrite it.
#[derive(Debug, Deserialize)]
pub struct SavedFile {
    pub path: String,
    pub name: String,
    pub text: String,
    #[serde(default)]
    pub read: Option<String>,
}

/// What saving came to: a sentence that says what was written, or, with nothing written, the names of the files that are
/// not what they were when they were read, which the user is to be asked about.
#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Saved {
    pub said: Option<String>,
    pub changed: Vec<String>,
}

/// The file at `path`, as the Document names it, of the netlist `entry`: where it is on disk; a path that is rooted is its own.
pub fn located(entry: &str, path: &str) -> PathBuf {
    if path.starts_with('/') || Path::new(path).is_absolute() {
        return machine_path(path);
    }
    machine_path(entry).parent().map_or_else(|| PathBuf::from("/"), Path::to_path_buf).join(path)
}

/// A text with its line ends as `\n`, as the page keeps it.
fn lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Whether the file at `at` still holds `read`, which the page read from it, line ends apart: a file that is gone, or
/// that another program changed since, does not.
fn unchanged(at: &Path, read: &str) -> bool {
    std::fs::read(at).is_ok_and(|bytes| lf(&codec::decode(&bytes)) == lf(read))
}

/// Write the files of the netlist `entry` where they were read from, each in its own encoding, and say which were
/// written. Unless `overwrite`, a file that no longer holds what the page read from it is not written, and nor is any of
/// the others: the names of those files are given back, for the user to say. Each path is checked against the folders
/// allowed; a file that is not below one is an error before anything is written.
pub fn save_all(roots: &Roots, entry: &str, files: &[SavedFile], overwrite: bool, encoding: OutputEncoding) -> Result<Saved, String> {
    let mut targets = Vec::new();
    for f in files {
        let at = roots.check(&located(entry, &f.path), true).map_err(|e| format!("{}: {e}", f.name))?;
        targets.push(at);
    }
    if !overwrite {
        let changed: Vec<String> = files
            .iter()
            .zip(&targets)
            .filter(|(f, at)| f.read.as_deref().is_some_and(|read| !unchanged(at, read)))
            .map(|(f, _)| f.name.clone())
            .collect();
        if !changed.is_empty() {
            return Ok(Saved { said: None, changed });
        }
    }
    // (every encoding is settled before the first write)
    let mut prepared = Vec::new();
    for (f, at) in files.iter().zip(&targets) {
        let bytes = codec::encode_file(Some(at), &f.text, encoding).map_err(|e| format!("{}: {e}", f.name))?;
        prepared.push((f, at, bytes));
    }
    let mut done: Vec<&str> = Vec::new();
    for (f, at, bytes) in prepared {
        if let Err(e) = std::fs::write(at, bytes) {
            let before = if done.is_empty() { String::new() } else { format!(" ({} had been saved)", done.join(", ")) };
            return Err(format!("{}: {e}{before}", at.display()));
        }
        done.push(&f.name);
    }
    let said = match done.as_slice() {
        [one] => format!("Saved {one}."),
        many => format!("Saved {} files.", many.len()),
    };
    Ok(Saved { said: Some(said), changed: Vec::new() })
}

/// Save As: the text of a netlist written to a file the user chose, which the folders allowed must hold, in the encoding
/// of the netlist it came from (`source`), unless the user chose UTF-8. Encoded before the destination is touched.
pub fn save_as(roots: &Roots, source: Option<&Path>, to: &Path, text: &str, encoding: OutputEncoding) -> Result<PathBuf, String> {
    let to = roots.check(to, false)?;
    let bytes = codec::encode_file(source, text, encoding)?;
    std::fs::write(&to, bytes).map_err(|e| format!("{}: {e}", to.display()))?;
    Ok(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rnl-local-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        canonical(&dir).unwrap()
    }

    #[test]
    fn only_what_is_below_an_allowed_folder_is_served() {
        let dir = temp("roots");
        std::fs::create_dir_all(dir.join("proj/sim")).unwrap();
        std::fs::write(dir.join("proj/sim/top.sp"), "R1 a b 1k\n").unwrap();
        std::fs::write(dir.join("proj/models.inc"), "* m\n").unwrap();
        std::fs::write(dir.join("secret.txt"), "no").unwrap();
        let roots = Roots::default();
        roots.allow_file(&dir.join("proj/sim/top.sp")).unwrap();
        // the folder of the file, and the one above it
        assert!(roots.check(&dir.join("proj/sim/top.sp"), true).is_ok());
        assert!(roots.check(&dir.join("proj/models.inc"), true).is_ok());
        // not a folder beside that one, nor above it
        assert!(roots.check(&dir.join("secret.txt"), true).is_err());
        // a path that climbs out is resolved first
        assert!(roots.check(&dir.join("proj/sim/../../secret.txt"), true).is_err());
        assert!(roots.check(Path::new("relative/x"), true).is_err());
    }

    #[test]
    fn nothing_hidden_below_an_allowed_folder_is_served() {
        let dir = temp("hidden");
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join(".git/config"), "x").unwrap();
        std::fs::write(dir.join(".env"), "x").unwrap();
        std::fs::write(dir.join("a.sp"), "x").unwrap();
        let roots = Roots::default();
        roots.allow(&dir).unwrap();
        assert!(roots.check(&dir.join("a.sp"), true).is_ok());
        assert!(roots.check(&dir.join(".git/config"), true).is_err());
        assert!(roots.check(&dir.join(".env"), true).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_that_leaves_an_allowed_folder_is_not_followed() {
        let dir = temp("link");
        std::fs::create_dir_all(dir.join("in")).unwrap();
        std::fs::create_dir_all(dir.join("out")).unwrap();
        std::fs::write(dir.join("out/x.sp"), "x").unwrap();
        std::os::unix::fs::symlink(dir.join("out/x.sp"), dir.join("in/x.sp")).unwrap();
        let roots = Roots::default();
        roots.allow(&dir.join("in")).unwrap();
        assert!(roots.check(&dir.join("in/x.sp"), true).is_err());
    }

    #[test]
    fn the_folder_above_a_file_is_allowed_unless_it_is_too_broad() {
        let dir = temp("broad");
        std::fs::write(dir.join("top.sp"), "x").unwrap();
        let roots = Roots::default();
        roots.allow_file(&dir.join("top.sp")).unwrap();
        // the folder of the file is allowed, and the one above it when it is not the user's home or the root
        let allowed = roots.list();
        assert!(allowed.iter().any(|d| *d == dir || dir.starts_with(d)));
        let filesystem_root = dir.ancestors().last().unwrap().to_path_buf();
        assert!(!allowed.contains(&filesystem_root));
    }

    #[test]
    fn a_folder_lists_folders_first_and_hides_dot_names() {
        let dir = temp("list");
        std::fs::create_dir_all(dir.join("b")).unwrap();
        std::fs::write(dir.join("a.sp"), "x").unwrap();
        std::fs::write(dir.join(".hidden"), "x").unwrap();
        let (_, entries) = list_dir(&dir, false).unwrap();
        assert_eq!(entries.iter().map(|e| (e.name.as_str(), e.dir)).collect::<Vec<_>>(), vec![("b", true), ("a.sp", false)]);
        let (_, all) = list_dir(&dir, true).unwrap();
        assert_eq!(all.len(), 3);
        assert!(list_dir(&dir.join("a.sp"), false).is_err());
    }

    fn saved(path: &str, text: &str, read: Option<&str>) -> SavedFile {
        SavedFile { path: path.into(), name: path.rsplit('/').next().unwrap().into(), text: text.into(), read: read.map(String::from) }
    }

    #[test]
    fn save_writes_each_file_as_it_was_and_asks_about_one_that_changed() {
        let dir = temp("save");
        let top = dir.join("top.sp");
        std::fs::write(&top, "R1 a b 1k\r\n.end\r\n").unwrap();
        let roots = Roots::default();
        roots.allow(&dir).unwrap();
        let entry = top.to_string_lossy().into_owned();
        // another program changed it since the page read it: nothing is written
        std::fs::write(&top, "R1 a b 2k\r\n.end\r\n").unwrap();
        let asked = save_all(&roots, &entry, &[saved("top.sp", "R1 a b 3k\n.end\n", Some("R1 a b 1k\n.end\n"))], false, OutputEncoding::Preserve).unwrap();
        assert_eq!(asked, Saved { said: None, changed: vec!["top.sp".into()] });
        assert_eq!(std::fs::read(&top).unwrap(), b"R1 a b 2k\r\n.end\r\n");
        // the user said to overwrite
        let done = save_all(&roots, &entry, &[saved("top.sp", "R1 a b 3k\n.end\n", Some("R1 a b 1k\n.end\n"))], true, OutputEncoding::Preserve).unwrap();
        assert_eq!(done.said.as_deref(), Some("Saved top.sp."));
        assert_eq!(std::fs::read(&top).unwrap(), b"R1 a b 3k\n.end\n");
        // a file that is what was read is written, line ends apart
        std::fs::write(&top, "R1 a b 1k\r\n").unwrap();
        let again = save_all(&roots, &entry, &[saved("top.sp", "R1 a b 5k\r\n", Some("R1 a b 1k\n"))], false, OutputEncoding::Preserve).unwrap();
        assert!(again.changed.is_empty());
    }

    #[test]
    fn a_latin1_file_stays_latin1_and_a_text_it_cannot_hold_is_refused_before_any_write() {
        let dir = temp("latin");
        let top = dir.join("top.sp");
        std::fs::write(&top, b"* caf\xe9\n").unwrap();
        let other = dir.join("other.sp");
        std::fs::write(&other, "x\n").unwrap();
        let roots = Roots::default();
        roots.allow(&dir).unwrap();
        let entry = top.to_string_lossy().into_owned();
        save_all(&roots, &entry, &[saved("top.sp", "* caf\u{e9} 2\n", None)], true, OutputEncoding::Preserve).unwrap();
        assert_eq!(std::fs::read(&top).unwrap(), b"* caf\xe9 2\n");
        let refused = save_all(
            &roots,
            &entry,
            &[saved("other.sp", "changed\n", None), saved("top.sp", "* \u{7f51}\n", None)],
            true,
            OutputEncoding::Preserve,
        );
        assert!(refused.unwrap_err().contains("Latin-1"));
        assert_eq!(std::fs::read(&other).unwrap(), b"x\n", "the first file was not written either");
    }

    #[test]
    fn a_file_outside_the_allowed_folders_is_not_written() {
        let dir = temp("outside");
        let inside = dir.join("in");
        let outside = dir.join("out");
        std::fs::create_dir_all(&inside).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("x.sp"), "x").unwrap();
        let roots = Roots::default();
        roots.allow(&inside).unwrap();
        let err = save_all(&roots, &inside.join("a.sp").to_string_lossy(), &[saved(&outside.join("x.sp").to_string_lossy(), "y", None)], true, OutputEncoding::Preserve).unwrap_err();
        assert!(err.contains("--allow"), "{err}");
        assert_eq!(std::fs::read(outside.join("x.sp")).unwrap(), b"x");
    }

    #[test]
    fn save_as_writes_a_new_file_in_the_encoding_of_its_source() {
        let dir = temp("saveas");
        let from = dir.join("from.sp");
        std::fs::write(&from, b"* caf\xe9\n").unwrap();
        let roots = Roots::default();
        roots.allow(&dir).unwrap();
        let to = save_as(&roots, Some(&from), &dir.join("copy.sp"), "* caf\u{e9}\n", OutputEncoding::Preserve).unwrap();
        assert_eq!(std::fs::read(to).unwrap(), b"* caf\xe9\n");
        assert!(save_as(&roots, Some(&from), &dir.join("sub/x.sp"), "x", OutputEncoding::Utf8).is_err(), "its folder does not exist");
        assert!(save_as(&roots, None, Path::new("/nope/x.sp"), "x", OutputEncoding::Utf8).is_err());
    }
}
