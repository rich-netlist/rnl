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
//! It keeps the objects of the authored records that the last reading of a document with records
//! of the solver counted too ([`rnl_runtime::AuthoredCounts`]), with what they were counted of: a
//! later reading takes them instead of counting again when it would count them of the same, as
//! [`rnl_runtime::interpret_taking`] decides.
//!
//! It keeps the reading of the last file asked for without its hints too, with every question
//! that reading asked of the files and its answer, so that the hints are given after the file
//! ([`crate::file_hints_cached`]) when every answer is the same, without reading again. The spans
//! of that file's answer can be kept with it, and given after with the hints
//! ([`ReadingCache::keep_spans`], [`crate::file_rest_cached`]).
//!
//! Nothing is written anywhere: the cache lives as long as the frontend holds it.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};

use rnl_runtime::diag::Sink;
use rnl_runtime::host::HostDocument;
use rnl_runtime::source::{LibraryDefinition, LibraryLookup, Provider};

/// What reading a document gives, kept between calls on one document ([`crate::outline_cached`],
/// [`crate::build_file_cached`]). It keeps one document: a call with another entry, or other
/// settings of the reading, drops what it holds. Calls made without it neither take nor drop it.
#[derive(Default)]
pub struct ReadingCache {
    host: Option<KeptHost>,
    /// The objects of the authored records the last reading through the cache gave, with what
    /// they were counted of.
    counts: rnl_runtime::AuthoredCounts,
    /// The reading of the file the last file reading through the cache made without the hints.
    reading: Option<KeptReading>,
}

impl ReadingCache {
    pub fn new() -> ReadingCache {
        ReadingCache::default()
    }

    /// Drop all it holds.
    pub fn clear(&mut self) {
        self.host = None;
        self.counts = rnl_runtime::AuthoredCounts::default();
        self.reading = None;
    }

    /// Keep the reading of a file of `key`, which asked `asked` of the files, and whose Document
    /// numbers its files by `files`, in place of the one held.
    pub(crate) fn keep_reading(
        &mut self,
        key: ReadingKey,
        asked: Vec<Asked>,
        reading: Box<rnl_runtime::Interpretation>,
        files: HashMap<String, usize>,
    ) {
        self.reading = Some(KeptReading { key, asked: distinct(asked), reading, files, spans: None });
    }

    /// Keep the spans of `result` with the reading of its file that the cache keeps, and leave
    /// `result` without them ([`crate::Document::spans`]), for [`crate::file_rest_cached`] to give:
    /// `result` must be the answer of the file reading through the cache that kept it, the last
    /// one ([`crate::build_file_cached`] without [`crate::BuildOptions::hints`]). False, and
    /// `result` left as it is, when the cache keeps no reading of `result`'s file, or keeps its
    /// spans already.
    pub fn keep_spans(&mut self, result: &mut crate::FileResult) -> bool {
        match &mut self.reading {
            Some(kept) if kept.key.file == result.file && kept.spans.is_none() => {
                kept.spans = Some(std::mem::take(&mut result.document.spans));
                true
            }
            _ => false,
        }
    }

    /// Drop the reading held.
    pub(crate) fn drop_reading(&mut self) {
        self.reading = None;
    }

    /// The reading held, given up, when it is one of `key` and every question it asked has the
    /// same answer from `files` now: reading again would read the same.
    pub(crate) fn take_reading(&mut self, key: &ReadingKey, files: &dyn Provider) -> Option<KeptReading> {
        let kept = self.reading.take()?;
        let same = kept.key.host == key.host
            && kept.key.file == key.file
            && kept.key.runtime.reads_as(&key.runtime)
            && kept.asked.iter().all(|asked| asked.again(files) == *asked);
        same.then_some(kept)
    }
}

/// What a reading of a file through the cache is of besides the files: the settings of its host
/// document, the file and the runtime's options.
pub(crate) struct ReadingKey {
    pub host: HostKey,
    pub file: String,
    pub runtime: rnl_runtime::Options,
}

/// A reading of a file kept for its hints, with each question it asked of the files, once, with
/// its answer.
pub(crate) struct KeptReading {
    key: ReadingKey,
    asked: Vec<Asked>,
    pub reading: Box<rnl_runtime::Interpretation>,
    /// The files of its Document, by path, numbered as the Document numbers them.
    pub files: HashMap<String, usize>,
    /// The spans its answer was left without ([`ReadingCache::keep_spans`]).
    pub spans: Option<Vec<crate::Span>>,
}

/// `asked` with each question asked again with the same answer left out.
fn distinct(asked: Vec<Asked>) -> Vec<Asked> {
    let first: Vec<bool> = {
        let mut seen = HashSet::new();
        asked.iter().map(|a| seen.insert(a)).collect()
    };
    asked.into_iter().zip(first).filter_map(|(a, first)| first.then_some(a)).collect()
}

/// The files of a reading, through which each question the reading asks of `files` is noted in
/// `asked` with its answer.
pub(crate) fn noting<'a>(files: &'a dyn Provider, asked: &'a RefCell<Vec<Asked>>) -> Noting<'a> {
    Noting { files, asked, library: files.auto_include().map(|library| NotingLibrary { library, asked }) }
}

/// `read` with the objects of the authored records `cache` keeps, which it takes when they were
/// counted of what it would count of ([`rnl_runtime::interpret_taking`]); `cache` then keeps those
/// it gives in their place.
pub(crate) fn with_counts<T>(
    cache: &RefCell<ReadingCache>,
    read: impl FnOnce(&mut rnl_runtime::AuthoredCounts) -> T,
) -> T {
    let mut counts = std::mem::take(&mut cache.borrow_mut().counts);
    let out = read(&mut counts);
    cache.borrow_mut().counts = counts;
    out
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

/// A question a reading asked of the files, with its answer.
#[derive(PartialEq, Eq, Debug)]
pub(crate) enum Asked {
    Read(String, Option<String>),
    PathKey(String, String),
    PackageFiles(Vec<String>),
    PackageCandidates(String, Vec<String>),
    IncludeCandidates(String, Vec<String>),
    AutoInclude(bool),
    LibraryCandidates(String, bool, Vec<LibraryDefinition>),
    LibraryDefinitions(String, Vec<LibraryDefinition>),
}

impl Hash for Asked {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let definitions = |found: &[LibraryDefinition], h: &mut H| {
            for d in found {
                (&d.name, &d.path, &d.file, &d.source, d.line).hash(h);
            }
        };
        std::mem::discriminant(self).hash(h);
        match self {
            Asked::Read(path, text) => (path, text).hash(h),
            Asked::PathKey(path, key) => (path, key).hash(h),
            Asked::PackageFiles(found) => found.hash(h),
            Asked::PackageCandidates(name, found) | Asked::IncludeCandidates(name, found) => (name, found).hash(h),
            Asked::AutoInclude(some) => some.hash(h),
            Asked::LibraryCandidates(name, case_sensitive, found) => {
                (name, case_sensitive).hash(h);
                definitions(found, h);
            }
            Asked::LibraryDefinitions(path, found) => {
                path.hash(h);
                definitions(found, h);
            }
        }
    }
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
pub(crate) struct Noting<'a> {
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
    let doc = read(&noting(files, &asked), sink);
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
    use std::collections::BTreeMap;

    use crate::tests::{corpus, Disk};
    use crate::{build_file, build_file_cached, outline, outline_cached, BuildOptions, MemoryProvider, ReadingCache};
    use crate::{build_file_drawn, build_file_drawn_cached, file_hints_cached, file_rest_cached, leave_out_texts};
    use crate::{FileResult, NOT_KEPT};
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

    /// A document whose record of the solver uses a number (5.9): reading it with its records of
    /// the solver counts the objects of the authored records, by reading it without them first.
    const SOLVED: &str = "* counted\n\
        * @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }\n\
        R1 a b 1k\n\
        R2 b 0 1k\n\
        R3 b 0 1k\n\
        C1 a 0 1p\n\
        R4 a 0 1k\n\
        C2 a 0 1p\n\
        * @RNL { #R1 @PLACE(0, 0); {#R4 #C2} @AS $PARALLEL_HORIZONTAL; {#R1:2 #R2:1 #R3:1} @ROUTE-OBJECT }\n\
        * @RNL(type=\"generated\", priority=\"high\", author=\"rnl-solver 0.1\", hash=\"sig2:56b05b841fd87eed.93f2018247edcd14\", numbering=\"b42673d973bb674b\") {\n\
        *   %ground_stub_R2_n = $GROUND_STUB @ATTACH(gap=3) #R2::n;\n\
        *   %ground_stub_R3_n = $GROUND_STUB @ATTACH(gap=3) #R3::n;\n\
        *   %ground_stub_C1_n = $GROUND_STUB @ATTACH(gap=3) #C1::n;\n\
        *   %ground_stub_R4_n = $GROUND_STUB @ATTACH(gap=3) #R4::n;\n\
        *   %ground_stub_C2_n = $GROUND_STUB @ATTACH(gap=3) #C2::n;\n\
        *   %net_label_stub_R1_p = $NET_LABEL_STUB @ATTACH #R1::p;\n\
        *   %net_label_stub_C1_p = $NET_LABEL_STUB @ATTACH #C1::p;\n\
        *   %rt1 = (0, -3);\n\
        *   #R1 @PLACE(0, 0) => @ORIENT(\"R0\");\n\
        *   #R2 @PLACE(0, -6) => @ORIENT(\"R0\");\n\
        *   #R3 @PLACE(5, -6) => @ORIENT(\"R0\");\n\
        *   #C1 @PLACE(-5, -6) => @ORIENT(\"R0\");\n\
        *   #R4 @PLACE(10, -6) => @ORIENT(\"R0\");\n\
        *   #C2 @PLACE(15, -6) => @ORIENT(\"R0\");\n\
        *   %#1 @PLACE(12.5, -6);\n\
        *   %#2 @PLACE(0, 0);\n\
        *   %ground_stub_R2_n @PLACE(0, 0);\n\
        *   %ground_stub_R2_n::g @PLACE(0, -10) => @ORIENT(\"R0\");\n\
        *   %ground_stub_R3_n @PLACE(0, 0);\n\
        *   %ground_stub_R3_n::g @PLACE(5, -10) => @ORIENT(\"R0\");\n\
        *   %ground_stub_C1_n @PLACE(0, 0);\n\
        *   %ground_stub_C1_n::g @PLACE(-5, -10) => @ORIENT(\"R0\");\n\
        *   %ground_stub_R4_n @PLACE(0, 0);\n\
        *   %ground_stub_R4_n::g @PLACE(10, -10) => @ORIENT(\"R0\");\n\
        *   %ground_stub_C2_n @PLACE(0, 0);\n\
        *   %ground_stub_C2_n::g @PLACE(15, -10) => @ORIENT(\"R0\");\n\
        *   %net_label_stub_R1_p @PLACE(0, 0);\n\
        *   %net_label_stub_R1_p::l @PLACE(0, 2) => @ORIENT(\"R270\");\n\
        *   %net_label_stub_C1_p @PLACE(0, 0);\n\
        *   %net_label_stub_C1_p::l @PLACE(-5, -4) => @ORIENT(\"R270\");\n\
        *   &a @WIRE(mode=\"straight\") {#R4::p (10, -3) (15, -3) #C2::p};\n\
        *   &b @WIRE(mode=\"straight\") {#R2::p %rt1};\n\
        *   &b @WIRE(mode=\"straight\") {%rt1 #R1::n};\n\
        *   &b @WIRE(mode=\"straight\") {%rt1 (5, -3) #R3::p};\n\
        *   $JUNCTION @ATTACH %rt1 => @PLACE(0, -3)\n\
        * }\n\
        .end\n";

    /// The objects of the authored records that a reading through the cache counted are taken by
    /// the next reading through it that would count them of the same, though its host document was
    /// read again; a reading that would count others counts, and so does a reading without the
    /// cache, or with one cleared. Each answer is the one without the cache.
    #[test]
    fn a_reading_through_the_cache_takes_the_objects_the_one_before_counted() {
        let plain = BuildOptions { svg: false, ..BuildOptions::default() };
        let hints = BuildOptions { hints: true, ..plain.clone() };
        let files = MemoryProvider::new().with("/w/t.sp", SOLVED);
        let mut cache = ReadingCache::new();
        let counts = |files: &MemoryProvider, options: &BuildOptions, cache: &mut ReadingCache| {
            let before = rnl_runtime::countings();
            let r = build_file_cached("/w/t.sp", "t.sp", files, options, cache).unwrap();
            let counted = rnl_runtime::countings() - before;
            assert_eq!(json(&r), json(&build_file("/w/t.sp", "t.sp", files, options).unwrap()));
            counted
        };
        let before = rnl_runtime::countings();
        build_file("/w/t.sp", "t.sp", &files, &plain).unwrap();
        assert_eq!(rnl_runtime::countings() - before, 1, "a reading without the cache counts");
        assert_eq!(counts(&files, &plain, &mut cache), 1);
        assert_eq!(counts(&files, &plain, &mut cache), 0);
        // the problems built too: counted of the same
        assert_eq!(counts(&files, &hints, &mut cache), 0);
        // the record of the solver changed on its own lines: the host document is read again, and
        // the objects are taken
        let fresh_before = fresh();
        let moved = files.clone().with("/w/t.sp", &SOLVED.replace("#R3 @PLACE(5, -6)", "#R3 @PLACE(6, -6)"));
        assert_eq!(counts(&moved, &plain, &mut cache), 0);
        assert_eq!(fresh() - fresh_before, 1);
        // an authored record changed: counted again, then taken
        let authored = moved.clone().with("/w/t.sp", &SOLVED.replace("#R1 @PLACE(0, 0); {", "#R1 @PLACE(0, 1); {"));
        assert_eq!(counts(&authored, &plain, &mut cache), 1);
        assert_eq!(counts(&authored, &plain, &mut cache), 0);
        // a cache cleared holds none
        cache.clear();
        assert_eq!(counts(&authored, &plain, &mut cache), 1);
    }

    /// The hints of each level of `r`, by host, as [`crate::FileHints`] gives them.
    fn hints_of(r: &FileResult) -> String {
        let levels: BTreeMap<usize, String> = r.document.levels.iter().map(|l| (l.host, json(&l.hints))).collect();
        json(&levels)
    }

    /// The hints [`file_hints_cached`] gives, by host.
    fn hints_given(hints: &crate::FileHints) -> String {
        let levels: BTreeMap<usize, String> = hints.levels.iter().map(|(host, h)| (*host, json(h))).collect();
        json(&levels)
    }

    /// The hints of a file are given from the reading a file request through the cache kept without
    /// them, once, as the request with them gives them; with no such reading of the file over the
    /// same answers of the files and options that read alike, there are none.
    #[test]
    fn the_hints_of_a_file_are_given_from_the_reading_kept_for_them() {
        let plain = BuildOptions { svg: false, flylines: true, ..BuildOptions::default() };
        let hints = BuildOptions { hints: true, ..plain.clone() };
        // R3 drawn over R2
        let over = SOLVED.replace("#R3 @PLACE(5, -6)", "#R3 @PLACE(0, -6)");
        let files = MemoryProvider::new().with("/w/t.sp", &over).with("/w/top.sp", TOP).with("/w/cells.inc", CELLS);
        let mut cache = ReadingCache::new();
        let not_kept = |r: Result<crate::FileHints, String>| r.err().as_deref() == Some(NOT_KEPT);
        let want = hints_of(&build_file("/w/t.sp", "t.sp", &files, &hints).unwrap());
        assert!(want.contains("RNL"), "the document has hints: {want}");
        // kept, given once
        let r = build_file_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).unwrap();
        assert_eq!(json(&r), json(&build_file("/w/t.sp", "t.sp", &files, &plain).unwrap()));
        assert_eq!(hints_given(&file_hints_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).unwrap()), want);
        assert!(not_kept(file_hints_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache)));
        // the drawings kept apart too
        let (r, drawings) = build_file_drawn_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).unwrap();
        let (plain_r, plain_drawings) = build_file_drawn("/w/t.sp", "t.sp", &files, &plain).unwrap();
        assert_eq!(json(&r), json(&plain_r));
        let unplaced = |d: &crate::Drawings, r: &FileResult| {
            let level = r.document.levels.iter().position(|l| !l.unplaced.is_empty());
            level.map(|k| {
                json(&d.unplaced(k, &(0..r.document.levels[k].unplaced.len()).collect::<Vec<_>>(), &[]).unwrap())
            })
        };
        assert_eq!(unplaced(&drawings, &r), unplaced(&plain_drawings, &plain_r));
        assert_eq!(hints_given(&file_hints_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).unwrap()), want);
        // none for a file read with its hints, for another file, or with other options
        build_file_cached("/w/t.sp", "t.sp", &files, &hints, &mut cache).unwrap();
        assert!(not_kept(file_hints_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache)));
        build_file_cached("/w/top.sp", "cells.inc", &files, &plain, &mut cache).unwrap();
        assert!(not_kept(file_hints_cached("/w/top.sp", "top.sp", &files, &plain, &mut cache)));
        build_file_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).unwrap();
        let other =
            BuildOptions { runtime: rnl_runtime::Options { std: false, ..plain.runtime.clone() }, ..plain.clone() };
        assert!(not_kept(file_hints_cached("/w/t.sp", "t.sp", &files, &other, &mut cache)));
        // drawing options read alike
        build_file_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).unwrap();
        let drawn = BuildOptions { svg: true, flylines: false, ..plain.clone() };
        assert_eq!(hints_given(&file_hints_cached("/w/t.sp", "t.sp", &files, &drawn, &mut cache).unwrap()), want);
        // none once a file it read changed, or is given, or the cache is cleared
        build_file_cached("/w/top.sp", "top.sp", &files, &plain, &mut cache).unwrap();
        let changed = files.clone().with("/w/cells.inc", &CELLS.replace("1k", "2k"));
        assert!(not_kept(file_hints_cached("/w/top.sp", "top.sp", &changed, &plain, &mut cache)));
        build_file_cached("/w/top.sp", "top.sp", &files, &plain, &mut cache).unwrap();
        let given = files.clone().with("/w/gone.inc", "* gone\n");
        assert!(not_kept(file_hints_cached("/w/top.sp", "top.sp", &given, &plain, &mut cache)));
        build_file_cached("/w/top.sp", "top.sp", &files, &plain, &mut cache).unwrap();
        cache.clear();
        assert!(not_kept(file_hints_cached("/w/top.sp", "top.sp", &files, &plain, &mut cache)));
        // without the cache, nothing is kept
        build_file("/w/top.sp", "top.sp", &files, &plain).unwrap();
        assert!(not_kept(file_hints_cached("/w/top.sp", "top.sp", &files, &plain, &mut cache)));
    }

    /// A file's answer is left without its spans, kept with the reading, and without the texts of
    /// the files given; the spans are given after with the hints, once.
    #[test]
    fn the_spans_of_a_file_are_kept_and_given_after_with_its_hints() {
        let plain = BuildOptions { svg: false, ..BuildOptions::default() };
        let hints = BuildOptions { hints: true, ..plain.clone() };
        let over = SOLVED.replace("#R3 @PLACE(5, -6)", "#R3 @PLACE(0, -6)");
        let files = MemoryProvider::new().with("/w/t.sp", &over);
        let mut cache = ReadingCache::new();
        let full = build_file("/w/t.sp", "t.sp", &files, &hints).unwrap();
        assert!(!full.document.spans.is_empty());
        let mut r = build_file_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).unwrap();
        assert!(cache.keep_spans(&mut r));
        assert!(r.document.spans.is_empty());
        assert!(!cache.keep_spans(&mut r), "kept once");
        leave_out_texts(&mut r);
        // the texts of the bundled packages stay, those given are left out
        assert!(r.document.files.iter().all(|f| f.text.is_empty() != f.path.starts_with(crate::BUNDLED_DIR)));
        let rest = file_rest_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).unwrap();
        assert_eq!(json(&rest.spans), json(&full.document.spans));
        assert_eq!(hints_given(&rest.hints), hints_of(&full));
        assert!(file_rest_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).is_err());
        // without the spans kept, or once the hints were given, there is no rest
        build_file_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).unwrap();
        assert_eq!(file_rest_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).err().as_deref(), Some(NOT_KEPT));
        let mut r = build_file_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).unwrap();
        assert!(cache.keep_spans(&mut r));
        file_hints_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).unwrap();
        assert_eq!(file_rest_cached("/w/t.sp", "t.sp", &files, &plain, &mut cache).err().as_deref(), Some(NOT_KEPT));
        // a reading with the hints keeps none
        let mut r = build_file_cached("/w/t.sp", "t.sp", &files, &hints, &mut cache).unwrap();
        assert!(!cache.keep_spans(&mut r));
        assert_eq!(json(&r.document.spans), json(&full.document.spans));
    }

    #[test]
    fn each_file_of_the_corpus_reads_with_the_cache_as_without() {
        let options = BuildOptions { svg: false, ..BuildOptions::default() };
        let hints = BuildOptions { hints: true, ..options.clone() };
        let mut with_hints = 0;
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
                    // the hints from the reading kept, as the file with them gives them
                    let given = file_hints_cached(&entry, &file, &Disk, &options, &mut cache).map(|h| hints_given(&h));
                    let want = build_file(&entry, &file, &Disk, &hints);
                    match (&cached, &want) {
                        (Ok(_), Ok(want)) => {
                            with_hints += usize::from(hints_of(want).contains("RNL"));
                            assert_eq!(given, Ok(hints_of(want)), "{entry} {file}");
                        }
                        _ => assert_eq!(given, Err(NOT_KEPT.to_string()), "{entry} {file}"),
                    }
                    // the spans kept, and given after with the hints
                    if let Ok(want) = &want {
                        let mut r = build_file_cached(&entry, &file, &Disk, &options, &mut cache).unwrap();
                        assert!(cache.keep_spans(&mut r), "{entry} {file}");
                        let rest = file_rest_cached(&entry, &file, &Disk, &options, &mut cache).unwrap();
                        assert_eq!(json(&rest.spans), json(&want.document.spans), "{entry} {file}");
                        assert_eq!(hints_given(&rest.hints), hints_of(want), "{entry} {file}");
                    }
                }
                assert!(Disk.read(&entry).is_some());
            }
        }
        assert!(with_hints > 0, "some file of the corpus has hints");
    }
}
