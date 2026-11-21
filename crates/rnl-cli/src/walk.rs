//! The folders and files a viewer lists from the file system: those of a PATH folder, as package
//! and include lookup finds them ([`path_listing`]), and those of a folder the user includes in the
//! Library ([`folder_files`]). The desktop viewer and the local viewer list them with these, so that
//! both list the same files.
//!
//! Both walks leave out what is hidden (a name that starts with `.`), enter no folder through a
//! link, and leave out a file whose link leads out of the folder walked. A file is listed by its
//! suffix ([`rnl_view::is_listed`]).

use std::path::{Path, PathBuf};

/// How far a walk goes below the folder it starts in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Walk {
    /// The most levels of folders below the first that are entered; None for all.
    pub depth: Option<usize>,
    /// The most items the walk gives: folders for [`path_listing`], files for [`folder_files`].
    pub limit: usize,
}

/// The sub-folders a PATH folder added with them brings in: all of them, up to 500.
pub const PATH_SUBFOLDERS: Walk = Walk { depth: None, limit: 500 };

/// The files a folder included in the Library brings in: those of up to 8 levels of folders below
/// it, up to 2000 files.
pub const INCLUDE_FOLDER: Walk = Walk { depth: Some(8), limit: 2000 };

/// What a PATH folder lists, and how lookup searches it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PathListing {
    /// The folders lookup searches for it, in order: the folder, then with its sub-folders each of
    /// them ([`subfolders`]). Each is a package directory and a search path for bare includes.
    pub dirs: Vec<PathBuf>,
    /// The packages and netlists in them, as lookup finds them, sorted.
    pub files: Vec<PathBuf>,
    /// Whether there were more sub-folders than [`PATH_SUBFOLDERS`] brings in.
    pub truncated: bool,
}

fn hidden(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

/// The folder as the file system resolves it, or the error to show.
fn root(dir: &Path) -> Result<PathBuf, String> {
    match std::fs::canonicalize(dir) {
        Ok(real) if real.is_dir() => Ok(real),
        Ok(_) => Err(format!("{} is not a folder", dir.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err("Directory not found.".to_owned()),
        Err(e) => Err(format!("{}: {e}", dir.display())),
    }
}

/// The entries of a folder that are not hidden, by name; none when it cannot be read.
fn entries(dir: &Path) -> Vec<std::fs::DirEntry> {
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<_> = read.filter_map(Result::ok).filter(|e| !hidden(&e.file_name())).collect();
    out.sort_by_key(|e| e.file_name());
    out
}

/// A folder that is one, not a link to one.
fn real_dir(entry: &std::fs::DirEntry) -> bool {
    entry.file_type().is_ok_and(|t| t.is_dir())
}

/// A listed file that is in `root` as the file system resolves it, with nothing hidden on the way.
fn listed_file(path: &Path, root: &Path) -> bool {
    rnl_view::is_listed(&path.to_string_lossy())
        && std::fs::canonicalize(path)
            .is_ok_and(|real| real.is_file() && real.strip_prefix(root).is_ok_and(|below| !below.iter().any(hidden)))
}

/// The folders below `dir`, depth first and by name, up to `walk`: not the hidden ones, not links,
/// and not the folders named by a version number, which lookup reads with their parent; and whether
/// there were more than its limit.
pub fn subfolders(dir: &Path, walk: Walk) -> (Vec<PathBuf>, bool) {
    fn below(dir: &Path, level: usize, walk: Walk, out: &mut Vec<PathBuf>) -> bool {
        if walk.depth.is_some_and(|d| level >= d) {
            return false;
        }
        for entry in entries(dir) {
            if !real_dir(&entry) || rnl_runtime::source::version_directory(&entry.file_name().to_string_lossy()) {
                continue;
            }
            if out.len() == walk.limit {
                return true;
            }
            out.push(entry.path());
            if below(&entry.path(), level + 1, walk, out) {
                return true;
            }
        }
        false
    }
    let mut out = Vec::new();
    let more = below(dir, 0, walk, &mut out);
    (out, more)
}

/// The packages and netlists of one folder of PATH as lookup finds them there: its files, and those
/// of its folders named by a version number, that are in `root`.
fn path_folder(dir: &Path, root: &Path, out: &mut Vec<PathBuf>) {
    for entry in entries(dir) {
        let path = entry.path();
        if real_dir(&entry) {
            if rnl_runtime::source::version_directory(&entry.file_name().to_string_lossy()) {
                out.extend(entries(&path).into_iter().map(|e| e.path()).filter(|p| listed_file(p, root)));
            }
        } else if listed_file(&path, root) {
            out.push(path);
        }
    }
}

/// What the PATH folder `dir` lists, and the folders lookup searches for it: the folder, and with
/// `with_subfolders` those below it ([`subfolders`], [`PATH_SUBFOLDERS`]). Each file is named below
/// `dir` as given. An error when `dir` is not a folder.
pub fn path_listing(dir: &Path, with_subfolders: bool) -> Result<PathListing, String> {
    let root = root(dir)?;
    let mut dirs = vec![dir.to_path_buf()];
    let mut truncated = false;
    if with_subfolders {
        let (below, more) = subfolders(dir, PATH_SUBFOLDERS);
        dirs.extend(below);
        truncated = more;
    }
    let mut files = Vec::new();
    for folder in &dirs {
        path_folder(folder, &root, &mut files);
    }
    files.sort();
    files.dedup();
    Ok(PathListing { dirs, files, truncated })
}

/// The netlists and packages of the folder `dir` and of those below it, up to `walk`
/// ([`INCLUDE_FOLDER`]), sorted; and whether there were more files than its limit. Each file is
/// named below `dir` as given. An error when `dir` is not a folder.
pub fn folder_files(dir: &Path, walk: Walk) -> Result<(Vec<PathBuf>, bool), String> {
    fn below(dir: &Path, root: &Path, level: usize, walk: Walk, out: &mut Vec<PathBuf>) -> bool {
        let (folders, files): (Vec<_>, Vec<_>) = entries(dir).into_iter().partition(real_dir);
        for file in files.into_iter().map(|e| e.path()).filter(|p| listed_file(p, root)) {
            if out.len() == walk.limit {
                return true;
            }
            out.push(file);
        }
        if walk.depth.is_some_and(|d| level >= d) {
            return false;
        }
        folders.into_iter().any(|f| below(&f.path(), root, level + 1, walk, out))
    }
    let root = root(dir)?;
    let mut out = Vec::new();
    let more = below(dir, &root, 0, walk, &mut out);
    out.sort();
    Ok((out, more))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rnl-walk-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn names(dir: &Path, paths: &[PathBuf]) -> Vec<String> {
        paths.iter().map(|p| p.strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/")).collect()
    }

    fn write(dir: &Path, files: &[&str]) {
        for name in files {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "text").unwrap();
        }
    }

    #[test]
    fn a_path_folder_lists_its_files_and_those_of_its_version_folders() {
        let dir = temp("path");
        write(
            &dir,
            &[
                "direct.rnl",
                "0.3/versioned.RNL",
                "0.3/cell.spi",
                "other/ignored.rnl",
                "other/ignored.sp",
                "notes.txt",
                "cell.NETLIST",
                "cell.va",
                ".hidden.sp",
                ".git/x.sp",
            ],
        );
        let listing = path_listing(&dir, false).unwrap();
        assert_eq!(listing.dirs, vec![dir.clone()]);
        assert_eq!(
            names(&dir, &listing.files),
            ["0.3/cell.spi", "0.3/versioned.RNL", "cell.NETLIST", "cell.va", "direct.rnl"]
        );
        // with its sub-folders, `other` follows it, and its files are listed too
        let deep = path_listing(&dir, true).unwrap();
        assert_eq!(names(&dir, &deep.dirs[1..]), ["other"]);
        assert_eq!(deep.files.len(), 7);
        assert!(!deep.truncated);
        assert_eq!(path_listing(&dir.join("missing"), false).unwrap_err(), "Directory not found.");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sub_folders_are_depth_first_by_name_up_to_the_limit() {
        let dir = temp("subfolders");
        for sub in ["b/x", "a", ".git/objects", "c/0.3", "c/d"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        let (found, more) = subfolders(&dir, PATH_SUBFOLDERS);
        assert_eq!(names(&dir, &found), ["a", "b", "b/x", "c", "c/d"]);
        assert!(!more);
        let (found, more) = subfolders(&dir, Walk { depth: Some(1), limit: 2 });
        assert_eq!((names(&dir, &found), more), (vec!["a".to_string(), "b".to_string()], true));
        let (found, more) = subfolders(&dir, Walk { depth: Some(1), limit: 500 });
        assert_eq!((names(&dir, &found), more), (vec!["a".to_string(), "b".to_string(), "c".to_string()], false));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_path_folder_with_its_sub_folders_stops_at_the_limit_and_says_so() {
        let dir = temp("path-many");
        for i in 0..PATH_SUBFOLDERS.limit + 5 {
            write(&dir, &[&format!("d{i:04}/f.sp")]);
        }
        let flat = path_listing(&dir, false).unwrap();
        assert!(flat.files.is_empty() && !flat.truncated);
        let deep = path_listing(&dir, true).unwrap();
        assert!(deep.truncated);
        assert_eq!((deep.files.len(), deep.dirs.len()), (PATH_SUBFOLDERS.limit, PATH_SUBFOLDERS.limit + 1));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_included_folder_gives_its_files_to_a_depth_and_up_to_a_limit() {
        let dir = temp("include");
        write(
            &dir,
            &["top.sp", "sub/models.inc", "sub/parts.RNL", "sub/0.4/p.rnl", "notes.txt", ".h/x.sp", "a/b/c.sp"],
        );
        let (files, more) = folder_files(&dir, INCLUDE_FOLDER).unwrap();
        assert_eq!(names(&dir, &files), ["a/b/c.sp", "sub/0.4/p.rnl", "sub/models.inc", "sub/parts.RNL", "top.sp"]);
        assert!(!more);
        let (files, more) = folder_files(&dir, Walk { depth: Some(1), limit: 2000 }).unwrap();
        assert_eq!(names(&dir, &files), ["sub/models.inc", "sub/parts.RNL", "top.sp"]);
        assert!(!more);
        let (files, more) = folder_files(&dir, Walk { depth: None, limit: 2 }).unwrap();
        assert_eq!((files.len(), more), (2, true));
        assert!(folder_files(&dir.join("missing"), INCLUDE_FOLDER).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn no_walk_leaves_the_folder_through_a_link() {
        let dir = temp("links");
        let outside = temp("links-outside");
        write(&outside, &["far.sp", "deep/far.inc"]);
        write(&dir, &["in.sp"]);
        std::os::unix::fs::symlink(outside.join("far.sp"), dir.join("link.sp")).unwrap();
        std::os::unix::fs::symlink(outside.join("deep"), dir.join("deep")).unwrap();
        assert_eq!(names(&dir, &folder_files(&dir, INCLUDE_FOLDER).unwrap().0), ["in.sp"]);
        let listing = path_listing(&dir, true).unwrap();
        assert_eq!((names(&dir, &listing.files), listing.dirs.len()), (vec!["in.sp".to_string()], 1));
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
    }
}
