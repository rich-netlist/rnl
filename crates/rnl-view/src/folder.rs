//! How every viewer lists a folder that the user includes in the Library: the walk of its files, once for all of them.
//! The desktop app and the local program walk a folder on disk with it (`rnl_cli::walk::folder_files`), and the web
//! viewer walks the paths a browser gives it for a folder chosen or dropped ([`folders_listing`], which the core in the
//! page answers as a `folders` request).
//!
//! A walk leaves out what is hidden (a name that starts with `.`), takes a file by its suffix ([`crate::is_listed`]),
//! enters folders down to its depth, and stops at its limit: the files of a folder by name, then each folder in it by
//! name, depth first.

use std::collections::BTreeMap;

/// How far a walk goes below the folder it starts in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Walk {
    /// The most levels of folders below the first that are entered; None for all.
    pub depth: Option<usize>,
    /// The most items the walk gives: folders for a PATH folder's sub-folders, files for a folder included.
    pub limit: usize,
}

/// The files a folder included in the Library brings in: those of up to 8 levels of folders below
/// it, up to 2000 files.
pub const INCLUDE_FOLDER: Walk = Walk { depth: Some(8), limit: 2000 };

/// Whether a name is hidden: it starts with a dot.
pub fn hidden(name: &str) -> bool {
    name.starts_with('.')
}

/// What a folder holds, as a walk is given it: a file, or a folder to enter.
pub enum Entry<F, D> {
    File(F),
    Folder(D),
}

/// What a folder holds, each entry by its name.
pub type Entries<F, D> = Vec<(String, Entry<F, D>)>;

/// The files of the folder `root` and of those below it, as `walk` takes them, in the order it meets them; and whether
/// there were more than its limit. `entries` gives what a folder holds, each by its name; `listed` whether a file is
/// taken (by its suffix, and on disk whether it is in the folder walked).
pub fn walk_folder<F, D>(
    root: &D,
    walk: Walk,
    entries: &dyn Fn(&D) -> Entries<F, D>,
    listed: &dyn Fn(&F) -> bool,
) -> (Vec<F>, bool) {
    fn below<F, D>(
        dir: &D,
        level: usize,
        walk: Walk,
        entries: &dyn Fn(&D) -> Entries<F, D>,
        listed: &dyn Fn(&F) -> bool,
        out: &mut Vec<F>,
    ) -> bool {
        let mut shown: Vec<_> = entries(dir).into_iter().filter(|(name, _)| !hidden(name)).collect();
        shown.sort_by(|a, b| a.0.cmp(&b.0));
        let mut folders = Vec::new();
        for (_, entry) in shown {
            match entry {
                Entry::File(file) if listed(&file) => {
                    if out.len() == walk.limit {
                        return true;
                    }
                    out.push(file);
                }
                Entry::File(_) => {}
                Entry::Folder(folder) => folders.push(folder),
            }
        }
        if walk.depth.is_some_and(|d| level >= d) {
            return false;
        }
        folders.into_iter().any(|f| below(&f, level + 1, walk, entries, listed, out))
    }
    let mut out = Vec::new();
    let more = below(root, 0, walk, entries, listed, &mut out);
    (out, more)
}

/// The files of folders, as a browser gives a folder chosen or dropped: their paths, `/`-separated, whose first part is
/// the folder included ("lib/sub/cell.inc"), or the file itself when it has no other ("top.sp", a file given alone); a
/// leading `/` is not a part. What is listed: each folder walked as [`walk_folder`] walks it, the files given alone that
/// are listed, sorted by their parts as paths sort; and the folders that had more files than the walk's limit, each of
/// which is listed in part, by their names (the first parts of their paths).
pub fn folders_listing(paths: &[String], walk: Walk) -> (Vec<String>, Vec<String>) {
    #[derive(Default)]
    struct Node {
        files: BTreeMap<String, String>,
        folders: BTreeMap<String, Node>,
    }
    let mut alone = Vec::new();
    let mut roots: BTreeMap<String, Node> = BTreeMap::new();
    for path in paths {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        match parts.as_slice() {
            [] => {}
            [file] => {
                if crate::is_listed(file) {
                    alone.push(path.clone());
                }
            }
            [root, below @ .., name] => {
                let mut node = roots.entry(root.to_string()).or_default();
                for part in below {
                    node = node.folders.entry(part.to_string()).or_default();
                }
                node.files.insert(name.to_string(), path.clone());
            }
        }
    }
    /// A folder's files, each by its name with its path, and its folders.
    fn entries<'a>(node: &&'a Node) -> Entries<(String, String), &'a Node> {
        let files = node.files.iter().map(|(name, path)| (name.clone(), Entry::File((name.clone(), path.clone()))));
        files.chain(node.folders.iter().map(|(name, folder)| (name.clone(), Entry::Folder(folder)))).collect()
    }
    let listed = |(name, _): &(String, String)| crate::is_listed(name);
    let mut out = alone;
    let mut truncated = Vec::new();
    for (name, root) in &roots {
        let (files, more) = walk_folder(&root, walk, &entries, &listed);
        out.extend(files.into_iter().map(|(_, path)| path));
        if more {
            truncated.push(name.clone());
        }
    }
    let parts = |p: &String| p.split('/').filter(|s| !s.is_empty()).map(str::to_owned).collect::<Vec<_>>();
    out.sort_by_key(parts);
    (out, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listing(paths: &[&str], walk: Walk) -> (Vec<String>, Vec<String>) {
        folders_listing(&paths.iter().map(|p| p.to_string()).collect::<Vec<_>>(), walk)
    }

    #[test]
    fn a_folder_given_as_paths_lists_its_files_nothing_hidden_down_to_the_depth() {
        let deep = |levels: usize| {
            format!("lib/{}/cell.inc", (0..levels).map(|i| format!("d{i}")).collect::<Vec<_>>().join("/"))
        };
        let (eight, nine) = (deep(8), deep(9));
        let given = [
            "lib/top.sp",
            "lib/notes.txt",
            "lib/.hidden.sp",
            "lib/.git/x.sp",
            "lib/sub/parts.RNL",
            &eight,
            &nine,
            "alone.sp",
            "alone.txt",
        ];
        let (files, more) = listing(&given, INCLUDE_FOLDER);
        assert_eq!(files, ["alone.sp", eight.as_str(), "lib/sub/parts.RNL", "lib/top.sp"]);
        assert!(more.is_empty());
    }

    #[test]
    fn the_limit_keeps_what_the_walk_meets_first_a_folders_files_then_its_folders() {
        let walk = Walk { depth: Some(8), limit: 3 };
        let (files, more) = listing(&["lib/b/1.sp", "lib/z.sp", "lib/a/1.sp", "lib/a.sp", "other/x.sp"], walk);
        // lib: a.sp and z.sp, then a/1.sp; b/1.sp is not met; other has a limit of its own. Sorted as paths sort
        assert_eq!(files, ["lib/a/1.sp", "lib/a.sp", "lib/z.sp", "other/x.sp"]);
        // lib had more; other, its one file
        assert_eq!(more, ["lib"]);
    }

    #[test]
    fn a_leading_slash_is_not_a_part() {
        let (files, _) = listing(&["/lib/top.sp", "/lib/.cache/top.sp", "/top.sp"], INCLUDE_FOLDER);
        assert_eq!(files, ["/lib/top.sp", "/top.sp"]);
    }
}
