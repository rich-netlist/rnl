//! The reading cache: what reading a document gives, kept in memory between the calls a frontend
//! makes on the document it shows, and taken again only when it would be read the same.
//!
//! The host document is read from what the frontend's files answer: their texts, the files a
//! name may be included from, the package files and the libraries. The cache keeps every question
//! the reading asked and its answer, in order. A later reading of the same entry with the same
//! reading settings asks them again, in the same order, through the same workspace: when every
//! answer is the same, the reading would be the same, and the host document kept is taken. The
//! files the frontend does not have are asked for again too, so they are missing as they were.
//!
//! Nothing is written anywhere: the cache lives as long as the frontend holds it.

use std::cell::RefCell;
use std::collections::BTreeMap;

use rnl_runtime::diag::Sink;
use rnl_runtime::host::HostDocument;
use rnl_runtime::source::{LibraryDefinition, LibraryLookup, Provider};

/// What reading a document gives, kept between calls on one document ([`crate::outline_cached`],
/// [`crate::build_file_cached`]). It keeps one document: a call with another entry, or other
/// settings of the reading, drops what it holds. Calls made without it neither take nor drop it.
#[derive(Default)]
pub struct ReadingCache {
    host: Option<KeptHost>,
}

impl ReadingCache {
    pub fn new() -> ReadingCache {
        ReadingCache::default()
    }

    /// Drop all it holds.
    pub fn clear(&mut self) {
        self.host = None;
    }
}

/// The settings a host document is read with besides its files: the entry, the dialect asked for
/// and the formats set for files ([`crate::BuildOptions`]).
#[derive(Clone, PartialEq)]
pub(crate) struct HostKey {
    pub entry: String,
    pub dialect: Option<String>,
    pub formats: BTreeMap<String, String>,
}

/// A host document kept, with the questions its reading asked and their answers.
struct KeptHost {
    key: HostKey,
    asked: Vec<Asked>,
    doc: HostDocument,
    sink: Sink,
}

/// A question the reading of a host document asked of the files, with its answer.
#[derive(PartialEq, Debug)]
enum Asked {
    Read(String, Option<String>),
    PathKey(String, String),
    PackageFiles(Vec<String>),
    PackageCandidates(String, Vec<String>),
    IncludeCandidates(String, Vec<String>),
    AutoInclude(bool),
    LibraryCandidates(String, bool, Vec<LibraryDefinition>),
    LibraryDefinitions(String, Vec<LibraryDefinition>),
}

impl Asked {
    /// The same question asked of `files` again, with its answer now.
    fn again(&self, files: &dyn Provider) -> Asked {
        match self {
            Asked::Read(path, _) => Asked::Read(path.clone(), files.read(path)),
            Asked::PathKey(path, _) => Asked::PathKey(path.clone(), files.path_key(path)),
            Asked::PackageFiles(_) => Asked::PackageFiles(files.package_files()),
            Asked::PackageCandidates(name, _) => Asked::PackageCandidates(name.clone(), files.package_candidates(name)),
            Asked::IncludeCandidates(name, _) => Asked::IncludeCandidates(name.clone(), files.include_candidates(name)),
            Asked::AutoInclude(_) => Asked::AutoInclude(files.auto_include().is_some()),
            Asked::LibraryCandidates(name, case_sensitive, _) => Asked::LibraryCandidates(
                name.clone(),
                *case_sensitive,
                files.auto_include().map_or_else(Vec::new, |l| l.candidates(name, *case_sensitive)),
            ),
            Asked::LibraryDefinitions(path, _) => Asked::LibraryDefinitions(
                path.clone(),
                files.auto_include().map_or_else(Vec::new, |l| l.definitions(path)),
            ),
        }
    }
}

/// The files of a reading, each question asked of them noted with its answer.
struct Noting<'a> {
    files: &'a dyn Provider,
    asked: &'a RefCell<Vec<Asked>>,
    library: Option<NotingLibrary<'a>>,
}

struct NotingLibrary<'a> {
    library: &'a dyn LibraryLookup,
    asked: &'a RefCell<Vec<Asked>>,
}

impl Provider for Noting<'_> {
    fn read(&self, path: &str) -> Option<String> {
        let text = self.files.read(path);
        self.asked.borrow_mut().push(Asked::Read(path.to_string(), text.clone()));
        text
    }
    fn path_key(&self, path: &str) -> String {
        let key = self.files.path_key(path);
        self.asked.borrow_mut().push(Asked::PathKey(path.to_string(), key.clone()));
        key
    }
    fn package_files(&self) -> Vec<String> {
        let found = self.files.package_files();
        self.asked.borrow_mut().push(Asked::PackageFiles(found.clone()));
        found
    }
    fn package_candidates(&self, name: &str) -> Vec<String> {
        let found = self.files.package_candidates(name);
        self.asked.borrow_mut().push(Asked::PackageCandidates(name.to_string(), found.clone()));
        found
    }
    fn include_candidates(&self, name: &str) -> Vec<String> {
        let found = self.files.include_candidates(name);
        self.asked.borrow_mut().push(Asked::IncludeCandidates(name.to_string(), found.clone()));
        found
    }
    fn auto_include(&self) -> Option<&dyn LibraryLookup> {
        self.asked.borrow_mut().push(Asked::AutoInclude(self.library.is_some()));
        self.library.as_ref().map(|l| l as &dyn LibraryLookup)
    }
}

impl LibraryLookup for NotingLibrary<'_> {
    fn candidates(&self, name: &str, case_sensitive: bool) -> Vec<LibraryDefinition> {
        let found = self.library.candidates(name, case_sensitive);
        self.asked.borrow_mut().push(Asked::LibraryCandidates(name.to_string(), case_sensitive, found.clone()));
        found
    }
    fn definitions(&self, path: &str) -> Vec<LibraryDefinition> {
        let found = self.library.definitions(path);
        self.asked.borrow_mut().push(Asked::LibraryDefinitions(path.to_string(), found.clone()));
        found
    }
}

/// The host document of `key` read from `files` into `sink`, which is empty, by `read`: the one
/// `cache` keeps when every question its reading asked has the same answer now, asked again in
/// the same order; else read, and kept in place of what the cache held. In debug builds a document
/// taken is read again and checked.
pub(crate) fn read_host(
    cache: &RefCell<ReadingCache>,
    key: HostKey,
    files: &dyn Provider,
    sink: &mut Sink,
    read: impl Fn(&dyn Provider, &mut Sink) -> Result<HostDocument, String>,
) -> Result<HostDocument, String> {
    debug_assert!(sink.all().is_empty());
    let kept = {
        let held = cache.borrow();
        match &held.host {
            Some(kept) if kept.key == key && kept.asked.iter().all(|asked| asked.again(files) == *asked) => {
                Some((kept.doc.clone(), kept.sink.clone()))
            }
            _ => None,
        }
    };
    if let Some((doc, kept_sink)) = kept {
        #[cfg(debug_assertions)]
        {
            let mut again = Sink::new();
            let fresh = read(files, &mut again);
            assert!(
                fresh.as_ref().is_ok_and(|fresh| *fresh == doc),
                "the host document kept for {} changed",
                key.entry
            );
            assert_eq!(format!("{:?}", again.all()), format!("{:?}", kept_sink.all()));
        }
        *sink = kept_sink;
        return Ok(doc);
    }
    #[cfg(test)]
    tests::READ_FRESH.with(|count| count.set(count.get() + 1));
    let asked = RefCell::new(Vec::new());
    let noting = Noting {
        files,
        asked: &asked,
        library: files.auto_include().map(|library| NotingLibrary { library, asked: &asked }),
    };
    let doc = read(&noting, sink);
    let mut held = cache.borrow_mut();
    held.host = match &doc {
        Ok(doc) => Some(KeptHost { key, asked: asked.into_inner(), doc: doc.clone(), sink: sink.clone() }),
        Err(_) => None,
    };
    doc
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use crate::tests::{corpus, Disk};
    use crate::{build_file, build_file_cached, outline, outline_cached, BuildOptions, MemoryProvider, ReadingCache};
    use rnl_runtime::source::Provider;

    thread_local! {
        /// How many host documents a reading cache read rather than took, on this thread.
        pub(super) static READ_FRESH: Cell<usize> = const { Cell::new(0) };
    }

    fn fresh() -> usize {
        READ_FRESH.with(Cell::get)
    }

    fn json<T: serde::Serialize>(value: &T) -> String {
        serde_json::to_string(value).unwrap()
    }

    const TOP: &str = "* top\n.include \"cells.inc\"\n.include \"gone.inc\"\nX1 a b cell\n.end\n";
    const CELLS: &str = "* cells\n.subckt cell p n\nR1 p n 1k\n.ends\n";

    #[test]
    fn a_document_read_again_the_same_is_taken_from_the_cache_and_gives_the_same() {
        let options = BuildOptions::default();
        let files = MemoryProvider::new().with("/w/top.sp", TOP).with("/w/cells.inc", CELLS);
        let mut cache = ReadingCache::new();
        let before = fresh();
        let o = outline_cached("/w/top.sp", &files, &options, &mut cache).unwrap();
        assert_eq!(json(&o), json(&outline("/w/top.sp", &files, &options).unwrap()));
        assert_eq!(o.missing, ["/w/gone.inc"]);
        // the files read the same: the outline's host document serves each file's reading, and a
        // file missing is missing still
        for file in ["top.sp", "cells.inc"] {
            let r = build_file_cached("/w/top.sp", file, &files, &options, &mut cache).unwrap();
            assert_eq!(json(&r), json(&build_file("/w/top.sp", file, &files, &options).unwrap()));
            assert_eq!(r.document.missing, ["/w/gone.inc"]);
        }
        assert_eq!(fresh() - before, 1);
        // given the file missing, each file's reading has its levels
        let given = files.with(
            "/w/gone.inc",
            "* gone
",
        );
        for file in ["top.sp", "cells.inc"] {
            let r = build_file_cached("/w/top.sp", file, &given, &options, &mut cache).unwrap();
            assert_eq!(json(&r), json(&build_file("/w/top.sp", file, &given, &options).unwrap()));
            assert!(!r.document.levels.is_empty(), "{file}");
        }
        assert_eq!(fresh() - before, 2);
    }

    #[test]
    fn a_change_of_any_answer_of_the_files_reads_the_document_again() {
        let options = BuildOptions::default();
        let files = MemoryProvider::new().with("/w/top.sp", TOP).with("/w/cells.inc", CELLS);
        let mut cache = ReadingCache::new();
        outline_cached("/w/top.sp", &files, &options, &mut cache).unwrap();
        let before = fresh();
        let check = |files: &MemoryProvider, cache: &mut ReadingCache| {
            let o = outline_cached("/w/top.sp", files, &options, cache).unwrap();
            assert_eq!(json(&o), json(&outline("/w/top.sp", files, &options).unwrap()));
            // each file as the outline names it
            for file in o.files.iter().map(|f| f.path.clone()) {
                let r = build_file_cached("/w/top.sp", &file, files, &options, cache).unwrap();
                assert_eq!(json(&r), json(&build_file("/w/top.sp", &file, files, &options).unwrap()), "{file}");
            }
        };
        // an included file changed
        let changed = files.clone().with("/w/cells.inc", &CELLS.replace("1k", "2k"));
        check(&changed, &mut cache);
        assert_eq!(fresh() - before, 1);
        // a file missing given
        let given = changed.clone().with("/w/gone.inc", "* gone\n");
        check(&given, &mut cache);
        assert_eq!(fresh() - before, 2);
        // another entry, and back: each read again
        let other = given.clone().with("/w/other.sp", "* other\nR1 a 0 1k\n.end\n");
        outline_cached("/w/other.sp", &other, &options, &mut cache).unwrap();
        check(&other, &mut cache);
        assert_eq!(fresh() - before, 4);
        // other reading settings
        let dialect = BuildOptions { dialect: Some("spice".into()), ..BuildOptions::default() };
        outline_cached("/w/top.sp", &other, &dialect, &mut cache).unwrap();
        assert_eq!(fresh() - before, 5);
        // a cache cleared holds nothing
        cache.clear();
        check(&other, &mut cache);
        assert_eq!(fresh() - before, 6);
    }

    #[test]
    fn each_file_of_the_corpus_reads_with_the_cache_as_without() {
        let options = BuildOptions { svg: false, ..BuildOptions::default() };
        for dir in ["cases-0.4/from-0.2", "equivalence-0.4"] {
            let dir = corpus(dir);
            let mut entries: Vec<String> = std::fs::read_dir(&dir)
                .unwrap()
                .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string()))
                .filter(|f| f.ends_with(".sp") || f.ends_with(".scs"))
                .map(|f| format!("{dir}/{f}"))
                .collect();
            entries.sort();
            for entry in entries {
                let mut cache = ReadingCache::new();
                let o = outline_cached(&entry, &Disk, &options, &mut cache).unwrap();
                assert_eq!(json(&o), json(&outline(&entry, &Disk, &options).unwrap()), "{entry}");
                for file in o.files.iter().map(|f| f.path.clone()) {
                    let cached = build_file_cached(&entry, &file, &Disk, &options, &mut cache).map(|r| json(&r));
                    let read = build_file(&entry, &file, &Disk, &options).map(|r| json(&r));
                    assert_eq!(cached, read, "{entry} {file}");
                }
                assert!(Disk.read(&entry).is_some());
            }
        }
    }
}
