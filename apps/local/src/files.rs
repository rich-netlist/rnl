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

/// The folders the page may use, and the single files it may use where their folder is too broad to give.
#[derive(Default)]
pub struct Roots {
    dirs: Mutex<Vec<PathBuf>>,
    files: Mutex<Vec<PathBuf>>,
}

/// The largest file the page may read: a netlist, a Library file; not what fills the memory of a shared machine.
pub const MAX_FILE: u64 = 256 * 1024 * 1024;

/// Whether a path has a hidden part: a name that starts with a dot.
fn hidden(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
}

/// Whether a folder is too broad to give to the page for a file it chose: the root of the file system, the user's home
/// folder, or one that holds it. The user may give one with `--allow`, which is theirs to say.
fn broad(dir: &Path) -> bool {
    dir.parent().is_none() || home().and_then(|h| canonical(&h)).is_some_and(|h| h.starts_with(dir))
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
pub fn canonical(path: &Path) -> Option<PathBuf> {
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
            if !broad(up) {
                self.allow(up)?;
            }
        }
        Ok(())
    }

    /// A file the page chose to open (not one named on the command line, which the user named): a regular file, with no
    /// hidden part in its path. Its folder is allowed as for a file of the command line, unless that folder is too broad,
    /// and then the file alone is.
    pub fn open_file(&self, file: &Path) -> Result<PathBuf, String> {
        let real = canonical(file).filter(|f| f.is_file()).ok_or_else(|| format!("{} is not a file", file.display()))?;
        if hidden(&real) {
            return Err("a hidden file, or one in a hidden folder, is not opened".into());
        }
        if std::fs::metadata(&real).is_ok_and(|m| m.len() > MAX_FILE) {
            return Err(format!("{} is too large", real.display()));
        }
        let dir = real.parent().ok_or_else(|| format!("{} has no folder", real.display()))?;
        if broad(dir) {
            let mut files = self.files.lock().map_err(|e| e.to_string())?;
            if !files.contains(&real) {
                files.push(real.clone());
            }
        } else {
            self.allow_file(&real)?;
        }
        Ok(real)
    }

    /// A folder the page chose to include: not the root, the user's home folder or one that holds it, and with no hidden
    /// part in its path (a `.ssh` is not the page's to give away, though a session holder asks for it).
    pub fn allow_chosen(&self, dir: &Path) -> Result<(), String> {
        let real = canonical(dir).filter(|d| d.is_dir()).ok_or_else(|| format!("{} is not a folder", dir.display()))?;
        if hidden(&real) {
            return Err("a hidden folder is not given to the page".into());
        }
        if broad(&real) {
            return Err(format!("{} is too much to give: choose the folder of a project (or start the program with --allow)", real.display()));
        }
        self.allow(&real)
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
        if self.files.lock().is_ok_and(|files| files.contains(&resolved)) {
            return Ok(resolved);
        }
        let dirs = self.list();
        let root = dirs.iter().find(|d| resolved.starts_with(d)).ok_or_else(|| {
            format!("{} is not in a folder the program was told to allow; start it with --allow FOLDER to add one", resolved.display())
        })?;
        let below = resolved.strip_prefix(root).unwrap_or(&resolved);
        if hidden(below) {
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

/// The most entries of a folder that are listed: each is looked at (its size and time), on the thread of the request, and
/// the folder may be on a network file system that takes a moment for each.
pub const MAX_ENTRIES: usize = 10_000;

/// What a folder holds, folders first, each in the order of its names: for the chooser, which browses the whole machine to
/// find what to open (the names, sizes and times, not the contents: those are the allowed folders'). Hidden names are left
/// out. At most `MAX_ENTRIES` are listed, the first the system gives, which the last of the answer says. A folder that
/// cannot be listed is an error that says why.
pub fn list_dir(dir: &Path) -> Result<(PathBuf, Vec<Entry>, bool), String> {
    let real = canonical(dir).filter(|d| d.is_dir()).ok_or_else(|| format!("{} is not a folder", dir.display()))?;
    let mut entries = Vec::new();
    let mut truncated = false;
    for entry in std::fs::read_dir(&real).map_err(|e| format!("{}: {e}", real.display()))? {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if entries.len() >= MAX_ENTRIES {
            truncated = true;
            break;
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
    Ok((real, entries, truncated))
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
    // (a file too large for the page to have read is not what it read)
    std::fs::metadata(at).is_ok_and(|m| m.len() <= MAX_FILE) && std::fs::read(at).is_ok_and(|bytes| lf(&codec::decode(&bytes)) == lf(read))
}

/// Write the files of the netlist `entry` where they were read from, each in its own encoding, and say which were
/// written. Unless `overwrite`, a file that no longer holds what the page read from it is not written, and nor is any of
/// the others: the names of those files are given back, for the user to say. Each path is checked against the folders
/// allowed; a file that is not below one is an error before anything is written.
pub fn save_all(roots: &Roots, entry: &str, files: &[SavedFile], overwrite: bool, encoding: OutputEncoding) -> Result<Saved, String> {
    let mut targets = Vec::new();
    for f in files {
        let at = roots.check(&located(entry, &f.path), true).map_err(|e| format!("{}: {e}", f.name))?;
        if !at.is_file() {
            return Err(format!("{}: not a regular file", f.name));
        }
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
        if let Err(e) = write_replacing(at, &bytes) {
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

/// Replace the contents of a file that is there. Where it can be done without changing anything else of the file, whole or
/// not at all: the bytes are written to a new file of its folder with the owner, group and mode the file has, flushed to
/// the disk, and renamed over it, so that a program that stops, a disk that fills or a reader at that moment sees the old
/// file or the new, never half. A rename makes another file, so where that would change what is the file's (it has other
/// names, hard links; it is another user's or another group's, which the user cannot give to the new one; the folder is
/// not the user's to write in, or has the sticky bit), the bytes are written in place instead, and read back to see that
/// they are what was meant. A file the user may not write is not replaced either way.
///
/// What a rename does not keep and nothing here can: ACLs and extended attributes (the new file has those of its folder's
/// defaults). A group's netlist is the case it is for: the group stays, the mode stays.
fn write_replacing(at: &Path, bytes: &[u8]) -> Result<(), String> {
    let say = |e: std::io::Error| format!("{}: {e}", at.display());
    // (a rename would replace a file the user may read only, as a write would not)
    std::fs::OpenOptions::new().write(true).open(at).map_err(say)?;
    let meta = std::fs::metadata(at).map_err(say)?;
    if replaced_by_rename(at, bytes, &meta).map_err(say)? {
        return Ok(());
    }
    write_in_place(at, bytes).map_err(say)
}

/// Write the bytes over the file, and read them back.
fn write_in_place(at: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().write(true).truncate(true).open(at)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    if std::fs::read(at)? != bytes {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "what was written is not what is read back"));
    }
    Ok(())
}

/// Replace the file by a new one with the same owner, group and mode: Ok(true) when it is, Ok(false) when that cannot be
/// done without changing something of the file (nothing is left of the attempt), and an error when it fails for another
/// reason, such as a disk that is full.
#[cfg(unix)]
fn replaced_by_rename(at: &Path, bytes: &[u8], old: &std::fs::Metadata) -> std::io::Result<bool> {
    use std::io::{ErrorKind, Write};
    use std::os::unix::fs::{fchown, MetadataExt};
    let Some(dir) = at.parent() else { return Ok(false) };
    // another name of the file would keep the old contents
    if old.nlink() > 1 {
        return Ok(false);
    }
    let temp = dir.join(format!(".rnl-save-{}", &crate::security::random_hex()[..16]));
    let mut file = match std::fs::OpenOptions::new().write(true).create_new(true).open(&temp) {
        Ok(file) => file,
        // (a folder that is not the user's to write in, or whose sticky bit would not let the file be replaced)
        Err(e) if e.kind() == ErrorKind::PermissionDenied || e.kind() == ErrorKind::ReadOnlyFilesystem => return Ok(false),
        Err(e) => return Err(e),
    };
    let done = (|| -> std::io::Result<bool> {
        let now = file.metadata()?;
        // the owner and group of the file, which the new one has not unless the user is the owner and the folder's group is
        // the file's: set them, and where the system does not let the user (another user's file, a group the user is not in),
        // the file is not replaced
        if (now.uid(), now.gid()) != (old.uid(), old.gid()) {
            match fchown(&file, Some(old.uid()), Some(old.gid())) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::PermissionDenied => return Ok(false),
                Err(e) => return Err(e),
            }
        }
        // (after the owner: changing it may clear the mode's set-user-id and set-group-id)
        file.set_permissions(old.permissions())?;
        file.write_all(bytes)?;
        file.sync_all()?;
        match std::fs::rename(&temp, at) {
            Ok(()) => {}
            // (a folder with the sticky bit does not let one user replace the file of another)
            Err(e) if e.kind() == ErrorKind::PermissionDenied => return Ok(false),
            Err(e) => return Err(e),
        }
        // (the rename is in the folder: flushed too, so that it is not undone by a crash)
        if let Ok(folder) = std::fs::File::open(dir) {
            let _ = folder.sync_all();
        }
        Ok(true)
    })();
    drop(file);
    if !matches!(done, Ok(true)) {
        let _ = std::fs::remove_file(&temp);
    }
    done
}

#[cfg(not(unix))]
fn replaced_by_rename(at: &Path, bytes: &[u8], old: &std::fs::Metadata) -> std::io::Result<bool> {
    use std::io::Write;
    let Some(dir) = at.parent() else { return Ok(false) };
    let temp = dir.join(format!(".rnl-save-{}", &crate::security::random_hex()[..16]));
    let done = (|| -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temp)?;
        file.set_permissions(old.permissions())?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, at)
    })();
    if done.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    done.map(|()| true)
}

/// What Save As came to: the file written, or the file that is there already, which is not replaced unless the user said so.
#[derive(Debug, PartialEq)]
pub enum SavedAs {
    Written(PathBuf),
    Exists(PathBuf),
}

/// Save As: the text of a netlist written to a file the user chose, which the folders allowed must hold, in the encoding
/// of the netlist it came from (`source`, which must be a file the page may read too), unless the user chose UTF-8. Encoded
/// before the destination is touched. A file of that name that is there is not replaced unless `overwrite`, and a link is
/// never written through, whatever it points to: the name chosen is a file of its folder or it is not written.
pub fn save_as(roots: &Roots, source: Option<&Path>, to: &Path, text: &str, encoding: OutputEncoding, overwrite: bool) -> Result<SavedAs, String> {
    let to = roots.check(to, false)?;
    let source = source.and_then(|s| roots.check(s, true).ok()).filter(|s| std::fs::metadata(s).is_ok_and(|m| m.is_file() && m.len() <= MAX_FILE));
    let bytes = codec::encode_file(source.as_deref(), text, encoding)?;
    match std::fs::symlink_metadata(&to) {
        Ok(meta) if meta.file_type().is_symlink() => return Err(format!("{} is a link: it is not written through; choose another name", to.display())),
        Ok(meta) if !meta.is_file() => return Err(format!("{} is not a file", to.display())),
        Ok(_) if !overwrite => return Ok(SavedAs::Exists(to)),
        Ok(_) => write_replacing(&to, &bytes)?,
        // (create_new does not follow a link that appeared since the look above: it fails)
        Err(_) => {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&to).map_err(|e| format!("{}: {e}", to.display()))?;
            file.write_all(&bytes).and_then(|()| file.sync_all()).map_err(|e| format!("{}: {e}", to.display()))?;
        }
    }
    Ok(SavedAs::Written(to))
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
        let (_, entries, truncated) = list_dir(&dir).unwrap();
        assert_eq!(entries.iter().map(|e| (e.name.as_str(), e.dir)).collect::<Vec<_>>(), vec![("b", true), ("a.sp", false)]);
        assert!(!truncated);
        assert!(list_dir(&dir.join("a.sp")).is_err());
    }

    #[test]
    fn a_folder_of_very_many_files_is_listed_up_to_a_limit_and_says_so() {
        let dir = temp("many");
        for i in 0..MAX_ENTRIES + 5 {
            std::fs::write(dir.join(format!("f{i}.sp")), "").unwrap();
        }
        let (_, entries, truncated) = list_dir(&dir).unwrap();
        assert_eq!((entries.len(), truncated), (MAX_ENTRIES, true));
        std::fs::remove_file(dir.join("f0.sp")).unwrap();
        std::fs::remove_file(dir.join("f1.sp")).unwrap();
        std::fs::remove_file(dir.join("f2.sp")).unwrap();
        std::fs::remove_file(dir.join("f3.sp")).unwrap();
        std::fs::remove_file(dir.join("f4.sp")).unwrap();
        std::fs::remove_file(dir.join("f5.sp")).unwrap();
        let (_, entries, truncated) = list_dir(&dir).unwrap();
        assert_eq!((entries.len(), truncated), (MAX_ENTRIES - 1, false));
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
        let SavedAs::Written(to) = save_as(&roots, Some(&from), &dir.join("copy.sp"), "* caf\u{e9}\n", OutputEncoding::Preserve, false).unwrap() else { panic!("not written") };
        assert_eq!(std::fs::read(to).unwrap(), b"* caf\xe9\n");
        assert!(save_as(&roots, Some(&from), &dir.join("sub/x.sp"), "x", OutputEncoding::Utf8, false).is_err(), "its folder does not exist");
        assert!(save_as(&roots, None, Path::new("/nope/x.sp"), "x", OutputEncoding::Utf8, false).is_err());
    }

    #[test]
    fn save_as_does_not_replace_a_file_unasked_and_does_not_write_through_a_link() {
        let dir = temp("saveas-exists");
        let roots = Roots::default();
        roots.allow(&dir).unwrap();
        let there = dir.join("there.sp");
        std::fs::write(&there, "mine\n").unwrap();
        assert_eq!(save_as(&roots, None, &there, "new\n", OutputEncoding::Utf8, false).unwrap(), SavedAs::Exists(there.clone()));
        assert_eq!(std::fs::read_to_string(&there).unwrap(), "mine\n");
        assert_eq!(save_as(&roots, None, &there, "new\n", OutputEncoding::Utf8, true).unwrap(), SavedAs::Written(there.clone()));
        assert_eq!(std::fs::read_to_string(&there).unwrap(), "new\n");
        // a folder is no file to write
        std::fs::create_dir_all(dir.join("folder")).unwrap();
        assert!(save_as(&roots, None, &dir.join("folder"), "x", OutputEncoding::Utf8, true).is_err());
        #[cfg(unix)]
        {
            let target = dir.join("precious.txt");
            std::fs::write(&target, "precious\n").unwrap();
            std::os::unix::fs::symlink(&target, dir.join("link.sp")).unwrap();
            // (a link that dangles, which creating the file would follow to a place of the link's choosing)
            std::os::unix::fs::symlink(dir.join("not-yet.txt"), dir.join("dangling.sp")).unwrap();
            for name in ["link.sp", "dangling.sp"] {
                for overwrite in [false, true] {
                    let err = save_as(&roots, None, &dir.join(name), "x\n", OutputEncoding::Utf8, overwrite).unwrap_err();
                    assert!(err.contains("link"), "{err}");
                }
            }
            assert_eq!(std::fs::read_to_string(&target).unwrap(), "precious\n");
            assert!(!dir.join("not-yet.txt").exists());
        }
    }

    #[test]
    fn a_save_replaces_the_file_whole_keeps_its_mode_and_leaves_nothing_beside_it() {
        let dir = temp("atomic");
        let top = dir.join("top.sp");
        std::fs::write(&top, "old\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&top, std::fs::Permissions::from_mode(0o640)).unwrap();
        }
        write_replacing(&top, b"new text\n").unwrap();
        assert_eq!(std::fs::read(&top).unwrap(), b"new text\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&top).unwrap().permissions().mode() & 0o777, 0o640);
        }
        let names: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec!["top.sp".to_string()], "no temporary file is left");
        // a file the user may not write is not replaced, as a write would not (a read-only file)
        let mut mode = std::fs::metadata(&top).unwrap().permissions();
        mode.set_readonly(true);
        std::fs::set_permissions(&top, mode).unwrap();
        // (root may write what it likes: the check is the user's)
        if std::fs::OpenOptions::new().write(true).open(&top).is_err() {
            assert!(write_replacing(&top, b"x").is_err());
            assert_eq!(std::fs::read(&top).unwrap(), b"new text\n");
        }
        let mut mode = std::fs::metadata(&top).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        mode.set_readonly(false);
        std::fs::set_permissions(&top, mode).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_save_keeps_the_owner_group_and_other_names_of_the_file() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let dir = temp("shared");
        let top = dir.join("top.sp");
        std::fs::write(&top, "old\n").unwrap();
        std::fs::set_permissions(&top, std::fs::Permissions::from_mode(0o660)).unwrap();
        let before = std::fs::metadata(&top).unwrap();
        write_replacing(&top, b"new\n").unwrap();
        let after = std::fs::metadata(&top).unwrap();
        assert_eq!((after.uid(), after.gid(), after.mode() & 0o7777), (before.uid(), before.gid(), 0o660));
        assert_eq!(std::fs::read(&top).unwrap(), b"new\n");
        assert_ne!(after.ino(), before.ino(), "a file that has no other name is replaced whole");
        // a group the user is in that is not the folder's: the new file is given it, as the old file had it
        let groups = std::process::Command::new("id").arg("-G").output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
        if let Some(other) = groups.split_whitespace().filter_map(|g| g.parse::<u32>().ok()).find(|g| *g != after.gid()) {
            std::os::unix::fs::chown(&top, None, Some(other)).unwrap();
            write_replacing(&top, b"newer\n").unwrap();
            assert_eq!(std::fs::metadata(&top).unwrap().gid(), other, "the group of the file stays");
        }
        // a file with another name is written in place, so that the other name has the text too
        let linked = dir.join("linked.sp");
        std::fs::hard_link(&top, &linked).unwrap();
        let inode = std::fs::metadata(&top).unwrap().ino();
        write_replacing(&top, b"both\n").unwrap();
        assert_eq!(std::fs::read(&linked).unwrap(), b"both\n");
        assert_eq!(std::fs::metadata(&top).unwrap().ino(), inode);
        assert_eq!(std::fs::metadata(&top).unwrap().nlink(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_in_a_folder_the_user_cannot_write_in_is_written_in_place() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let dir = temp("readonly-folder");
        let top = dir.join("top.sp");
        std::fs::write(&top, "old\n").unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let inode = std::fs::metadata(&top).unwrap().ino();
        // (the super-user may write in any folder: there, the file is replaced, which is as well)
        let may_write_in_it = std::fs::File::create(dir.join("probe")).is_ok();
        let _ = std::fs::remove_file(dir.join("probe"));
        let saved = write_replacing(&top, b"new text\n");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        saved.unwrap();
        assert_eq!(std::fs::read(&top).unwrap(), b"new text\n");
        if !may_write_in_it {
            assert_eq!(std::fs::metadata(&top).unwrap().ino(), inode, "the file was written, not replaced");
        }
        let names: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec!["top.sp".to_string()]);
    }

    #[test]
    fn a_file_too_large_for_the_page_to_have_read_is_not_what_it_read() {
        let dir = temp("large");
        let top = dir.join("top.sp");
        std::fs::write(&top, "R1 a b 1k\n").unwrap();
        assert!(unchanged(&top, "R1 a b 1k\n"));
        let big = std::fs::File::create(dir.join("big.sp")).unwrap();
        big.set_len(MAX_FILE + 1).unwrap();
        assert!(!unchanged(&dir.join("big.sp"), ""));
    }

    #[test]
    fn a_save_only_writes_regular_files() {
        let dir = temp("regular");
        std::fs::create_dir_all(dir.join("folder")).unwrap();
        let roots = Roots::default();
        roots.allow(&dir).unwrap();
        let entry = dir.join("top.sp").to_string_lossy().into_owned();
        let err = save_all(&roots, &entry, &[saved(&dir.join("folder").to_string_lossy(), "x", None)], true, OutputEncoding::Preserve).unwrap_err();
        assert!(err.contains("regular file"), "{err}");
    }

    #[test]
    fn a_file_the_page_chose_is_not_hidden_and_a_folder_too_broad_gives_only_that_file() {
        let dir = temp("chosen");
        std::fs::create_dir_all(dir.join(".ssh")).unwrap();
        std::fs::write(dir.join(".ssh/id_rsa"), "secret").unwrap();
        std::fs::write(dir.join("a.sp"), "x").unwrap();
        std::fs::write(dir.join("b.sp"), "x").unwrap();
        let roots = Roots::default();
        assert!(roots.open_file(&dir.join(".ssh/id_rsa")).is_err());
        assert!(roots.open_file(&dir).is_err(), "a folder is no file");
        assert!(roots.allow_chosen(&dir.join(".ssh")).is_err());
        assert!(roots.check(&dir.join(".ssh/id_rsa"), true).is_err());
        // the folder of a project is given with the file
        roots.open_file(&dir.join("a.sp")).unwrap();
        assert!(roots.check(&dir.join("b.sp"), true).is_ok());
        // the root of the file system is too broad for the folder of a file: that file alone is given
        let root = dir.ancestors().last().unwrap().to_path_buf();
        assert!(roots.allow_chosen(&root).is_err());
        assert!(broad(&root));
        assert!(home().and_then(|h| canonical(&h)).is_none_or(|h| broad(&h)));
        assert!(!broad(&dir));
    }
}
