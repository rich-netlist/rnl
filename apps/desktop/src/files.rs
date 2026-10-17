//! The files the viewer builds from and writes (decision 0011): the texts the user edited, held over
//! the files on disk when the netlist is built or solved again, and, only when the user saves, the
//! files written back where they were read from, each as it was: its encoding and, with the page
//! restoring them, its line ends. Nothing here writes a file the user did not say to save.

use std::collections::HashMap;
use std::path::PathBuf;

use rnl_runtime::source::{dirname, join, normalize, Layered, MemoryProvider, Provider};
use serde::Deserialize;

/// The texts to build over the files of a netlist, by the path the Document names each file by: a
/// path relative to the directory of the netlist, unless it is rooted.
pub type Edits = HashMap<String, String>;

/// A file to save: the path the Document names it by, its name as shown, and its text as the file has it.
#[derive(Debug, Deserialize)]
pub struct SavedFile {
    pub path: String,
    pub name: String,
    pub text: String,
}

/// The file at `path`, as the Document names it, of the netlist `entry`: where it is on disk.
fn located(entry: &str, path: &str) -> String {
    join(&dirname(&normalize(entry)), path)
}

/// The edits as files in memory, each where the core looks for it when it builds the netlist `entry`.
pub fn held(entry: &str, edits: &Edits) -> MemoryProvider {
    let mut files = MemoryProvider::new();
    for (path, text) in edits {
        files.insert(&located(entry, path), text);
    }
    files
}

/// The files of the netlist `entry` as the core reads them: the edits over the disk.
pub fn layered<'a>(edits: &'a MemoryProvider, disk: &'a dyn Provider) -> Layered<'a> {
    Layered { layers: vec![edits, disk] }
}

/// The bytes that write `text` over a file that is `existing`, in the encoding it has: UTF-8, with
/// its byte order mark if it has one, or, when it is not UTF-8, which the core reads as Latin-1,
/// Latin-1 again. A file that is not there yet is UTF-8.
pub fn encode(existing: Option<&[u8]>, text: &str) -> Result<Vec<u8>, String> {
    let Some(bytes) = existing else { return Ok(text.as_bytes().to_vec()) };
    if std::str::from_utf8(bytes).is_ok() {
        let mut out = if bytes.starts_with(&[0xef, 0xbb, 0xbf]) { vec![0xef, 0xbb, 0xbf] } else { Vec::new() };
        out.extend_from_slice(text.as_bytes());
        return Ok(out);
    }
    text.chars().map(|c| u8::try_from(u32::from(c)).map_err(|_| c)).collect::<Result<Vec<u8>, char>>().map_err(|c| {
        format!("the file is Latin-1, not UTF-8, and cannot hold {c:?}: save the netlist as another file with Save As")
    })
}

/// Write the files of the netlist `entry` where they were read from, each in its own encoding, and
/// say which were written. Stops at the first that cannot be written, and says which had been.
pub fn save_all(entry: &str, files: &[SavedFile]) -> Result<String, String> {
    let mut done: Vec<&str> = Vec::new();
    for f in files {
        let at = PathBuf::from(located(entry, &f.path));
        let existing = std::fs::read(&at).ok();
        let written = encode(existing.as_deref(), &f.text)
            .map_err(|e| format!("{}: {e}", f.name))
            .and_then(|bytes| std::fs::write(&at, bytes).map_err(|e| format!("{}: {e}", at.display())));
        if let Err(e) = written {
            return Err(if done.is_empty() { e } else { format!("{e} (saved before it: {})", done.join(", ")) });
        }
        done.push(&f.name);
    }
    Ok(match done.as_slice() {
        [one] => format!("Saved {one}."),
        many => format!("Saved {}.", many.join(", ")),
    })
}

#[cfg(test)]
mod tests {
    use rnl_cli::FsProvider;
    use rnl_view::{BuildOptions, Document};

    use super::*;

    /// A directory of its own, removed when the test ends.
    struct Dir(PathBuf);

    impl Dir {
        fn new(name: &str) -> Dir {
            let dir = std::env::temp_dir().join(format!("rnl-viewer-test-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Dir(dir)
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn build(entry: &std::path::Path, edits: &Edits) -> Document {
        let entry = std::path::absolute(entry).unwrap().to_string_lossy().into_owned();
        let held = held(&entry, edits);
        let disk = FsProvider::new(&[]);
        rnl_view::build(&entry, &layered(&held, &disk), &BuildOptions::default()).unwrap()
    }

    /// The instances of the Document, by name.
    fn names(doc: &Document) -> Vec<String> {
        let mut names: Vec<String> =
            doc.objects.iter().filter(|o| o.kind == "instance").map(|o| o.name.clone()).collect();
        names.sort();
        names
    }

    /// The texts of the user go over the files on disk: the netlist's own, an include's, by the path
    /// the Document names them by, `../` included; the disk is not touched.
    #[test]
    fn edits_are_built_over_the_disk() {
        let dir = Dir::new("over");
        std::fs::create_dir_all(dir.file("sub")).unwrap();
        std::fs::write(dir.file("top.sp"), "* top\n.include \"sub/lib.inc\"\nR1 a b 1k\n.end\n").unwrap();
        std::fs::write(dir.file("sub/lib.inc"), "* lib\nR2 b c 1k\n").unwrap();
        let plain = build(&dir.file("top.sp"), &Edits::new());
        assert_eq!(names(&plain), ["R1", "R2"]);
        let files: Vec<&str> = plain.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(files, ["top.sp", "sub/lib.inc"]);
        // the include edited: renamed, and one more instance
        let edits = Edits::from([("sub/lib.inc".to_string(), "* lib\nR9 b c 1k\nR8 c d 1k\n".to_string())]);
        let edited = build(&dir.file("top.sp"), &edits);
        assert_eq!(names(&edited), ["R1", "R8", "R9"]);
        // the netlist edited: it no longer includes anything, and the include is gone
        let edits = Edits::from([("top.sp".to_string(), "* top\nR1 a b 1k\n.end\n".to_string())]);
        let edited = build(&dir.file("top.sp"), &edits);
        assert_eq!(names(&edited), ["R1"]);
        assert_eq!(edited.files.len(), 1);
        // an include that the edit adds is read from disk
        let edits =
            Edits::from([("top.sp".to_string(), "* top\n.include \"sub/lib.inc\"\nR1 a b 1k\n.end\n".to_string())]);
        assert_eq!(names(&build(&dir.file("top.sp"), &edits)), ["R1", "R2"]);
        // nothing was written
        assert_eq!(std::fs::read_to_string(dir.file("sub/lib.inc")).unwrap(), "* lib\nR2 b c 1k\n");
    }

    /// A file is written in the encoding it has: UTF-8 with or without its byte order mark, Latin-1
    /// when it is not UTF-8 and the text can be, and a file that is not there in UTF-8.
    #[test]
    fn a_file_is_written_in_its_own_encoding() {
        assert_eq!(encode(None, "é").unwrap(), "é".as_bytes());
        assert_eq!(encode(Some(b"plain"), "é").unwrap(), "é".as_bytes());
        assert_eq!(encode(Some(b"\xef\xbb\xbfbom"), "é").unwrap(), b"\xef\xbb\xbf\xc3\xa9");
        // Latin-1: the existing bytes are not UTF-8, and the text is written as they were
        assert_eq!(encode(Some(b"caf\xe9"), "café\n").unwrap(), b"caf\xe9\n");
        // what Latin-1 cannot hold is refused, and says what to do
        let e = encode(Some(b"caf\xe9"), "\u{4e2d}").unwrap_err();
        assert!(e.contains("Latin-1") && e.contains("Save As"), "{e}");
    }

    /// Only the files given are written, where the Document names them, each as it was; the
    /// sentence names them; a file that cannot be written stops the rest and says what was written.
    #[test]
    fn only_the_files_given_are_written() {
        let dir = Dir::new("save");
        std::fs::create_dir_all(dir.file("sub")).unwrap();
        std::fs::write(dir.file("top.sp"), "* top\r\nR1 a b 1k\r\n").unwrap();
        std::fs::write(dir.file("sub/lib.inc"), "* lib\n").unwrap();
        std::fs::write(dir.file("other.inc"), "* other\n").unwrap();
        let entry = dir.file("top.sp").to_string_lossy().into_owned();
        let file =
            |path: &str, name: &str, text: &str| SavedFile { path: path.into(), name: name.into(), text: text.into() };
        let said = save_all(&entry, &[file("top.sp", "top.sp", "* top\r\nR1 a b 2k\r\n")]).unwrap();
        assert_eq!(said, "Saved top.sp.");
        assert_eq!(std::fs::read(dir.file("top.sp")).unwrap(), b"* top\r\nR1 a b 2k\r\n");
        let said =
            save_all(&entry, &[file("top.sp", "top.sp", "* top\r\n"), file("sub/lib.inc", "sub/lib.inc", "* lib 2\n")])
                .unwrap();
        assert_eq!(said, "Saved top.sp, sub/lib.inc.");
        assert_eq!(std::fs::read_to_string(dir.file("sub/lib.inc")).unwrap(), "* lib 2\n");
        // the file that was not given is as it was
        assert_eq!(std::fs::read_to_string(dir.file("other.inc")).unwrap(), "* other\n");
        // a place that cannot be written: the first is said, and what was written before it
        let e = save_all(&entry, &[file("top.sp", "top.sp", "* again\r\n"), file("gone/x.inc", "gone/x.inc", "x")])
            .unwrap_err();
        assert!(e.contains("x.inc") && e.contains("saved before it: top.sp"), "{e}");
        assert_eq!(std::fs::read(dir.file("top.sp")).unwrap(), b"* again\r\n");
    }
}
