//! The files a frontend holds for a build or a solve, and how they are searched: one reading of a
//! request's `files`, `pool`, `package_dirs`, `search_paths` and Auto Include, which the WebAssembly
//! core's builds, its solver module, the local viewer's solves and `rnl preview --request` share, so
//! that a solve finds what the build it follows found.
//!
//! The files are searched as the desktop viewer searches its own ([`crate::LibraryProvider`] over
//! the Library's files and the PATH folders): the pool first, every file of it alike; then the
//! package directories and the search paths in order ([`MemoryProvider`]); with Auto Include, the
//! subcircuits the pool's files define, from an index of them ([`LibraryIndex`]).

use std::collections::BTreeMap;

use rnl_runtime::source::{normalize, FilePool, LibraryDefinition, LibraryLookup, MemoryProvider, Provider};
use serde::Deserialize;

use crate::LibraryIndex;

/// The files of a build or a solve, as a request gives them. Its fields are those of the
/// WebAssembly core's build request.
#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
pub struct RequestFiles {
    /// The texts by path, the entry's among them.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// The paths, among those of `files`, of the Library's files: searched first, for packages by
    /// name and for bare includes, and indexed for Auto Include.
    #[serde(default)]
    pub pool: Vec<String>,
    /// The directories, among the paths of `files`, whose packages are found by name, in order,
    /// after the pool: in each, `NAME.rnl` first.
    #[serde(default)]
    pub package_dirs: Vec<String>,
    /// The directories, among the paths of `files`, searched in order for a bare include name that
    /// is not beside the file that includes it, after the pool: `DIR/NAME` in each.
    #[serde(default)]
    pub search_paths: Vec<String>,
    /// Whether a subcircuit that no file read defines is taken from the pool's files that define
    /// it (Auto Include).
    #[serde(default)]
    pub auto_include: bool,
    /// The start of the pool's paths that the index leaves out, so that it names each file as the
    /// Library does: a page that holds the pool's files under `/<pool>` keeps them apart from the
    /// files it opened.
    #[serde(default)]
    pub library_prefix: String,
    /// Texts applied over the pool's files that the index reads, by path without the prefix.
    #[serde(default)]
    pub library_edits: BTreeMap<String, String>,
}

impl RequestFiles {
    /// The provider a build or a solve of the workspace reads, with the formats set for files by
    /// path (those of a build's options). Auto Include's index is made anew; it reads the pool's
    /// files only when Auto Include is on.
    pub fn open(self, formats: &BTreeMap<String, String>) -> Opened {
        self.open_with(formats, None)
    }

    /// [`RequestFiles::open`] with the index a frontend keeps from one build to the next, brought up to
    /// date with the pool: only the files that are new to it, or whose format changed, are read.
    pub fn open_kept(self, formats: &BTreeMap<String, String>, index: &mut LibraryIndex) -> Opened {
        self.open_with(formats, Some(index))
    }

    fn open_with(self, formats: &BTreeMap<String, String>, kept: Option<&mut LibraryIndex>) -> Opened {
        let files = self.files.into_iter().map(|(p, t)| (normalize(&p), t)).collect();
        let files = MemoryProvider { files, package_dirs: self.package_dirs, search_paths: self.search_paths };
        let pool: Vec<String> = self.pool.iter().map(|p| normalize(p)).collect();
        let prefix = self.library_prefix;
        let mut fresh = LibraryIndex::default();
        let index = match kept {
            Some(index) => Some(index),
            None => self.auto_include.then_some(&mut fresh),
        };
        let lookup = index.map(|index| {
            // the index reads the pool by the paths without the prefix, as the formats set for them name them
            let canonical: Vec<String> =
                pool.iter().map(|p| p.strip_prefix(&prefix).unwrap_or(p).to_string()).collect();
            let searched = FilePool { files: &files, paths: &pool };
            let named = Prefixed { files: &searched, prefix: &prefix };
            let read = FilePool { files: &named, paths: &canonical };
            let edits = self.library_edits.iter().map(|(p, t)| (normalize(p), t.clone())).collect();
            let edits = MemoryProvider { files: edits, ..MemoryProvider::default() };
            let set =
                formats.iter().map(|(p, f)| (p.strip_prefix(prefix.as_str()).unwrap_or(p).to_string(), f.clone()));
            index.set_formats(set.collect());
            index.update(&canonical, &read, false);
            index.with_edits(&edits, &read)
        });
        let lookup = lookup.filter(|_| self.auto_include).map(|index| Namespaced { index, prefix: prefix.clone() });
        Opened { files, pool, lookup }
    }
}

/// A workspace as a build or a solve reads it ([`RequestFiles::open`]).
pub struct Opened {
    files: MemoryProvider,
    pool: Vec<String>,
    lookup: Option<Namespaced>,
}

impl Opened {
    fn searched(&self) -> FilePool<'_> {
        FilePool { files: &self.files, paths: &self.pool }
    }
}

impl Provider for Opened {
    fn read(&self, path: &str) -> Option<String> {
        self.files.read(path)
    }
    fn package_files(&self) -> Vec<String> {
        self.searched().package_files()
    }
    fn package_candidates(&self, name: &str) -> Vec<String> {
        self.searched().package_candidates(name)
    }
    fn include_candidates(&self, name: &str) -> Vec<String> {
        self.searched().include_candidates(name)
    }
    fn auto_include(&self) -> Option<&dyn LibraryLookup> {
        self.lookup.as_ref().map(|l| l as &dyn LibraryLookup)
    }
    fn path_key(&self, path: &str) -> String {
        self.files.path_key(path)
    }
}

/// The pool's files by their paths without the prefix, as the index names them.
struct Prefixed<'a> {
    files: &'a dyn Provider,
    prefix: &'a str,
}

impl Provider for Prefixed<'_> {
    fn read(&self, path: &str) -> Option<String> {
        self.files.read(&format!("{}{path}", self.prefix))
    }
    fn path_key(&self, path: &str) -> String {
        self.files.path_key(path)
    }
}

/// The index of the pool, its definitions named by the paths of the workspace, the prefix put back.
struct Namespaced {
    index: LibraryIndex,
    prefix: String,
}

impl Namespaced {
    fn map(&self, mut d: LibraryDefinition) -> LibraryDefinition {
        d.path = format!("{}{}", self.prefix, d.path);
        d.file = format!("{}{}", self.prefix, d.file);
        d
    }
}

impl LibraryLookup for Namespaced {
    fn candidates(&self, name: &str, sensitive: bool) -> Vec<LibraryDefinition> {
        self.index.candidates(name, sensitive).into_iter().map(|d| self.map(d)).collect()
    }
    fn definitions(&self, path: &str) -> Vec<LibraryDefinition> {
        let path = path.strip_prefix(self.prefix.as_str()).unwrap_or(path);
        self.index.definitions(path).into_iter().map(|d| self.map(d)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CELL: &str = "* cell\n.subckt cell a b\nR1 a b 1k\n.ends\n";

    fn workspace(auto_include: bool) -> RequestFiles {
        RequestFiles {
            files: BTreeMap::from([
                ("/top.sp".to_string(), "* top\nX1 a 0 cell\n.end\n".to_string()),
                ("/<pool>/lib/cell.inc".to_string(), CELL.to_string()),
            ]),
            pool: vec!["/<pool>/lib/cell.inc".into()],
            auto_include,
            library_prefix: "/<pool>".into(),
            ..RequestFiles::default()
        }
    }

    #[test]
    fn auto_include_takes_a_subcircuit_from_the_pool_when_it_is_on() {
        let none = BTreeMap::new();
        let on = workspace(true).open(&none);
        let found = on.auto_include().expect("an index").candidates("cell", false);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, "/<pool>/lib/cell.inc");
        assert!(workspace(false).open(&none).auto_include().is_none());
        // a kept index is brought up to date, and still gives nothing with Auto Include off
        let mut kept = LibraryIndex::default();
        assert!(workspace(false).open_kept(&none, &mut kept).auto_include().is_none());
        assert_eq!(kept.candidates("cell", false).len(), 1);
    }

    #[test]
    fn the_pool_is_searched_before_the_directories() {
        let files = RequestFiles {
            files: BTreeMap::from([
                ("/pool/part.inc".to_string(), "R1 a 0 1k\n".to_string()),
                ("/A/part.inc".to_string(), "R2 a 0 1k\n".to_string()),
            ]),
            pool: vec!["/pool/part.inc".into()],
            search_paths: vec!["/A".into()],
            ..RequestFiles::default()
        }
        .open(&BTreeMap::new());
        assert_eq!(files.include_candidates("part.inc"), ["/pool/part.inc", "/A/part.inc"]);
    }
}
