//! Save and Save As (decision 0011: they write where the file is), for every viewer that writes files: the shell finds
//! where each file is and may refuse a place (the local program writes only below the folders it allows), and this writes.

use std::io::Write;
use std::path::{Path, PathBuf};

use rnl_cli::codec::{self, OutputEncoding};
use serde::{Deserialize, Serialize};

/// The largest file that is taken to be one the page read: a file larger than it has changed since.
pub const MAX_FILE: u64 = 256 * 1024 * 1024;

/// A file to save: the path the Document names it by, its name as shown, and its text as the file has it. `read` is the
/// text of the file as the page read it, which the file on disk must still be, unless the user chose to overwrite it: none,
/// and it is not looked at.
#[derive(Debug, Deserialize)]
pub struct SavedFile {
    pub path: String,
    pub name: String,
    pub text: String,
    #[serde(default)]
    pub read: Option<String>,
}

/// What saving came to: the names of the files written, or, with nothing written, the names of the files that are not
/// what they were when they were read, which the user is to be asked about. The page says it.
#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Saved {
    pub written: Vec<String>,
    pub changed: Vec<String>,
}

/// A text with its line ends as `\n`, as the page keeps it.
fn lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Whether the file at `at` still holds `read`, which the page read from it, line ends apart: a file that is gone, or
/// that another program changed since, does not.
pub fn unchanged(at: &Path, read: &str) -> bool {
    // (a file too large for the page to have read is not what it read)
    std::fs::metadata(at).is_ok_and(|m| m.len() <= MAX_FILE)
        && std::fs::read(at).is_ok_and(|bytes| lf(&codec::decode(&bytes)) == lf(read))
}

/// Write `files`, each at its place on disk (`at`, which the shell found and allowed), in its own encoding unless
/// `encoding` says UTF-8. Unless `overwrite`, a file that no longer holds what the page read from it is not written, and
/// nor is any of the others: their names are given back, for the user to say. Every encoding is settled before the first
/// write; a write that fails stops the others, and the error names the files written before it.
pub fn save_all(files: &[(&SavedFile, PathBuf)], overwrite: bool, encoding: OutputEncoding) -> Result<Saved, String> {
    if !overwrite {
        let changed: Vec<String> = files
            .iter()
            .filter(|(f, at)| f.read.as_deref().is_some_and(|read| !unchanged(at, read)))
            .map(|(f, _)| f.name.clone())
            .collect();
        if !changed.is_empty() {
            return Ok(Saved { written: Vec::new(), changed });
        }
    }
    let mut prepared = Vec::new();
    for (f, at) in files {
        let bytes = codec::encode_file(Some(at), &f.text, encoding).map_err(|e| format!("{}: {e}", f.name))?;
        prepared.push((f, at, bytes));
    }
    let mut written = Vec::new();
    for (f, at, bytes) in prepared {
        if let Err(e) = crate::atomic::write(at, &bytes) {
            let before = if written.is_empty() { String::new() } else { format!(" ({})", written.join(", ")) };
            return Err(format!("{}: {e}{before}", at.display()));
        }
        written.push(f.name.clone());
    }
    Ok(Saved { written, changed: Vec::new() })
}

/// What a file saved under a name the user chose came to: written, or a file of that name is there already, which is not
/// replaced unless the user said so.
#[derive(Debug, PartialEq)]
pub enum SavedAs {
    Written(PathBuf),
    Exists(PathBuf),
}

/// Write `bytes` as the file `to`, a name the user chose (Save As, a project's archive, an image, a solved copy): a file
/// there is replaced, whole or not at all, only with `overwrite`; a link is never written through, whatever it points to,
/// nor anything that is not a file: the name chosen is a file of its folder, or nothing is written.
pub fn write_chosen(to: &Path, bytes: &[u8], overwrite: bool) -> Result<SavedAs, String> {
    match std::fs::symlink_metadata(to) {
        Ok(meta) if meta.file_type().is_symlink() => {
            Err(format!("{} is a link: it is not written through; choose another name", to.display()))
        }
        Ok(meta) if !meta.is_file() => Err(format!("{} is not a file", to.display())),
        Ok(_) if !overwrite => Ok(SavedAs::Exists(to.to_path_buf())),
        Ok(_) => {
            crate::atomic::write(to, bytes).map_err(|e| format!("{}: {e}", to.display()))?;
            Ok(SavedAs::Written(to.to_path_buf()))
        }
        // (create_new does not follow a link that appeared since the look above: it fails)
        Err(_) => {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(to)
                .map_err(|e| format!("{}: {e}", to.display()))?;
            file.write_all(bytes).and_then(|()| file.sync_all()).map_err(|e| format!("{}: {e}", to.display()))?;
            Ok(SavedAs::Written(to.to_path_buf()))
        }
    }
}

/// Save As: the text of a netlist written as the file `to`, in the encoding of the netlist it came from (`source`, a file
/// the shell may read; none for a text that no file holds), unless the user chose UTF-8; encoded before `to` is touched,
/// and a source that is gone is an error, not a new text: its encoding is not known. As `write_chosen`.
pub fn save_as(
    source: Option<&Path>,
    to: &Path,
    text: &str,
    encoding: OutputEncoding,
    overwrite: bool,
) -> Result<SavedAs, String> {
    let bytes = codec::encode_file(source, text, encoding)?;
    write_chosen(to, &bytes, overwrite)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own, removed when the test ends.
    struct Dir(PathBuf);

    impl Dir {
        fn new(name: &str) -> Dir {
            let dir = std::env::temp_dir().join(format!("rnl-viewer-files-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Dir(dir)
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn file(path: &str, text: &str, read: Option<&str>) -> SavedFile {
        SavedFile { path: path.into(), name: path.into(), text: text.into(), read: read.map(str::to_string) }
    }

    #[test]
    fn save_writes_each_file_and_names_them_and_none_when_one_changed_on_disk() {
        let dir = Dir::new("save");
        let (a, b) = (dir.0.join("a.sp"), dir.0.join("b.inc"));
        std::fs::write(&a, "* a\r\nR1 1 0 1k\r\n").unwrap();
        std::fs::write(&b, "* b\n").unwrap();
        // (the page gives the text with the line ends of the file; what it read, with `\n`, is compared line ends apart)
        let fa = file("a.sp", "* a\r\nR1 1 0 2k\r\n", Some("* a\nR1 1 0 1k\n"));
        let fb = file("b.inc", "* b changed\n", Some("* b\n"));
        let saved = save_all(&[(&fa, a.clone()), (&fb, b.clone())], false, OutputEncoding::Preserve).unwrap();
        assert_eq!(saved, Saved { written: vec!["a.sp".into(), "b.inc".into()], changed: vec![] });
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "* a\r\nR1 1 0 2k\r\n");
        // another program changes b: nothing is written, and b is named
        std::fs::write(&b, "* someone else\n").unwrap();
        let fa = file("a.sp", "* a\nR1 1 0 3k\n", Some("* a\nR1 1 0 2k\n"));
        let saved = save_all(&[(&fa, a.clone()), (&fb, b.clone())], false, OutputEncoding::Preserve).unwrap();
        assert_eq!(saved, Saved { written: vec![], changed: vec!["b.inc".into()] });
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "* a\r\nR1 1 0 2k\r\n");
        // the user says to overwrite it
        let saved = save_all(&[(&fa, a.clone()), (&fb, b.clone())], true, OutputEncoding::Preserve).unwrap();
        assert_eq!(saved.written.len(), 2);
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "* b changed\n");
    }

    #[test]
    fn a_name_chosen_is_written_new_asked_about_when_there_and_never_through_a_link() {
        let dir = Dir::new("chosen");
        let to = dir.0.join("copy.sp");
        assert_eq!(
            save_as(None, &to, "* new\n", OutputEncoding::Preserve, false).unwrap(),
            SavedAs::Written(to.clone())
        );
        assert_eq!(
            save_as(None, &to, "* again\n", OutputEncoding::Preserve, false).unwrap(),
            SavedAs::Exists(to.clone())
        );
        assert_eq!(std::fs::read_to_string(&to).unwrap(), "* new\n");
        assert_eq!(write_chosen(&to, b"* again\n", true).unwrap(), SavedAs::Written(to.clone()));
        assert_eq!(std::fs::read_to_string(&to).unwrap(), "* again\n");
        // a folder of that name is not a file
        std::fs::create_dir(dir.0.join("folder")).unwrap();
        assert!(write_chosen(&dir.0.join("folder"), b"x", true).is_err());
        #[cfg(unix)]
        {
            let link = dir.0.join("link.sp");
            std::os::unix::fs::symlink(&to, &link).unwrap();
            let refused = write_chosen(&link, b"* through\n", true).unwrap_err();
            assert!(refused.contains("is a link"), "{refused}");
            assert_eq!(std::fs::read_to_string(&to).unwrap(), "* again\n");
        }
    }
}
