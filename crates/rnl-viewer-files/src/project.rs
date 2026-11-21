//! A project opened into a folder the user chose (apps/viewer-ui/project.js): its files are written there by their names
//! within that folder alone, never through a link or a junction, which could lead out of it, and each as Save writes one,
//! whole or not at all. A file of the folder that has other names, hard links, is replaced at its name in the folder alone
//! ([`crate::atomic::write_own`]), so that a name outside the folder the user chose keeps what it had.

use std::path::{Path, PathBuf};

/// Whether a file's metadata, not followed, is a link: a symbolic link, or on Windows any reparse point, a junction among
/// them.
fn is_link(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}

/// The file `relative` names within `root`: a relative name of folders and a file, `/` or `\` between them, none empty,
/// `.`, `..` or with a `:` (a drive, or a stream of a file). Each folder on the way is one of its own, not a link or a
/// junction, which could lead out of the root; with `make`, those not there are made. The file is not a link either.
/// Where the folders are found or made, they are still in the root, for a link made in the meantime.
pub fn inside(root: &Path, relative: &str, make: bool) -> Result<PathBuf, String> {
    let parts: Vec<&str> = relative.split(['/', '\\']).collect();
    if parts.iter().any(|p| p.is_empty() || *p == "." || *p == ".." || p.contains(':')) {
        return Err(format!("{relative}: not a name within the project's folder"));
    }
    let real = std::fs::canonicalize(root).map_err(|e| format!("{}: {e}", root.display()))?;
    let mut at = real.clone();
    for (k, part) in parts.iter().enumerate() {
        at.push(part);
        let last = k + 1 == parts.len();
        match std::fs::symlink_metadata(&at) {
            Ok(meta) if is_link(&meta) => {
                return Err(format!(
                    "{}: a link or a junction; a project's file is not written through it",
                    at.display()
                ))
            }
            Ok(meta) if !last && !meta.is_dir() => return Err(format!("{}: not a folder", at.display())),
            Ok(meta) if last && !meta.is_file() => return Err(format!("{}: not a file", at.display())),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if !last && make {
                    std::fs::create_dir(&at).map_err(|e| format!("{}: {e}", at.display()))?;
                } else if !last {
                    break;
                }
            }
            Err(e) => return Err(format!("{}: {e}", at.display())),
        }
    }
    let folder = at.parent().unwrap_or(&real);
    if folder.exists() {
        let folder = std::fs::canonicalize(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        if !folder.starts_with(&real) {
            return Err(format!("{relative}: not within the project's folder"));
        }
    }
    Ok(real.join(parts.join(std::path::MAIN_SEPARATOR_STR)))
}

/// Which of the files `relative` names within `root` are there already, or are not names a project's file can be written
/// at ([`inside`]), which the user is told of before anything is written.
pub fn existing(root: &Path, relative: Vec<String>) -> Vec<String> {
    relative
        .into_iter()
        .filter(|path| inside(root, path, false).map_or(true, |at| std::fs::symlink_metadata(at).is_ok()))
        .collect()
}

/// Write `bytes` as the project's file `relative` names within `root` ([`inside`]), with the folders it needs.
pub fn write_file(root: &Path, relative: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let path = inside(root, relative, true)?;
    // (a file of the folder with other names, hard links, is replaced at its name in the folder alone)
    crate::atomic::write_own(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::{existing, inside, write_file};
    use std::path::{Path, PathBuf};

    struct Dir(PathBuf);
    impl Dir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("rnl-project-test-{}-{name}", std::process::id()));
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

    /// A link to the folder `to` at `at`: a junction on Windows, which needs no privilege; a symbolic link elsewhere.
    fn link_folder(at: &Path, to: &Path) -> bool {
        #[cfg(windows)]
        {
            std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(at)
                .arg(to)
                .output()
                .is_ok_and(|o| o.status.success())
        }
        #[cfg(not(windows))]
        {
            std::os::unix::fs::symlink(to, at).is_ok()
        }
    }

    #[test]
    fn a_projects_files_are_written_by_their_names_within_the_folder_chosen() {
        let dir = Dir::new("inside");
        let root = dir.0.join("root");
        std::fs::create_dir(&root).unwrap();
        let at = inside(&root, "lib/models/a.inc", true).unwrap();
        assert!(at.starts_with(std::fs::canonicalize(&root).unwrap()));
        assert!(at.parent().unwrap().is_dir(), "the folders it needs are made");
        for name in ["../out.sp", "lib/../../out.sp", "", "a//b.sp", "./a.sp", "C:/out.sp", "a.sp:stream", "/abs.sp"] {
            assert!(inside(&root, name, true).is_err(), "{name}");
        }
        assert!(!dir.0.join("out.sp").exists());
    }

    #[test]
    fn the_files_there_and_the_names_that_cannot_be_written_are_told_before_any_is_written() {
        let dir = Dir::new("existing");
        let root = dir.0.join("root");
        std::fs::create_dir_all(root.join("lib")).unwrap();
        std::fs::write(root.join("lib/a.inc"), "* a\n").unwrap();
        let named = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        assert_eq!(
            existing(&root, named(&["lib/a.inc", "lib/b.inc", "new/c.inc", "../out.sp"])),
            named(&["lib/a.inc", "../out.sp"])
        );
        write_file(&root, "new/c.inc", b"* c\n").unwrap();
        assert_eq!(std::fs::read_to_string(root.join("new/c.inc")).unwrap(), "* c\n");
        assert!(write_file(&root, "../out.sp", b"x").is_err());
    }

    #[test]
    fn a_link_or_a_junction_in_the_folder_chosen_is_not_written_through() {
        let dir = Dir::new("links");
        let root = dir.0.join("root");
        let outside = dir.0.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        if !link_folder(&root.join("lib"), &outside) {
            eprintln!("no link could be made here: nothing to try");
            return;
        }
        let e = inside(&root, "lib/a.inc", true).unwrap_err();
        assert!(e.contains("link or a junction"), "{e}");
        assert!(std::fs::read_dir(&outside).unwrap().next().is_none(), "nothing is written outside");
        // the folder chosen may itself be reached by a link: the user chose it
        let at = inside(&root.join("lib"), "a.inc", true).unwrap();
        assert!(at.starts_with(std::fs::canonicalize(&outside).unwrap()));
    }

    #[test]
    fn a_file_with_another_name_outside_the_folder_is_replaced_at_its_name_in_the_folder_alone() {
        let dir = Dir::new("hard-link");
        let root = dir.0.join("root");
        std::fs::create_dir(&root).unwrap();
        let outside = dir.0.join("outside.sp");
        std::fs::write(&outside, "* outside\n").unwrap();
        if std::fs::hard_link(&outside, root.join("top.sp")).is_err() {
            eprintln!("no hard link could be made here: nothing to try");
            return;
        }
        let at = write_file(&root, "top.sp", b"* the project's\n").unwrap();
        assert_eq!(std::fs::read_to_string(&at).unwrap(), "* the project's\n");
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "* outside\n", "the name outside keeps what it had");
        // a file of one name is replaced as Save replaces it
        crate::atomic::write_own(&outside, b"* again\n").unwrap();
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "* again\n");
    }
}
