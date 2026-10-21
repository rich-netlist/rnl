//! Per-window LRU of serialized documents and their lazy drawings. A hit checks the complete
//! observed input: source/package text (including failed reads), lookup order and auto-includes.
//! Only the core thread owns this cache: Drawings contains non-Send runtime values.
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::mem::size_of;
use std::rc::Rc;

use rnl_runtime::source::{normalize, LibraryDefinition, LibraryLookup, Provider};
use rnl_view::{BuildOptions, Drawings, UnplacedDrawing};
use serde::Serialize;

pub const DEFAULT_LIMIT: usize = 256 * 1024 * 1024;

#[derive(Default)]
struct Inputs {
    reads: BTreeMap<String, Option<String>>,
    packages: Option<Vec<String>>,
    package_queries: BTreeMap<String, Vec<String>>,
    include_queries: BTreeMap<String, Vec<String>>,
    candidates: BTreeMap<(String, bool), Vec<LibraryDefinition>>,
    definitions: BTreeMap<String, Vec<LibraryDefinition>>,
}

fn string_bytes(s: &String) -> usize {
    s.capacity() + 32
}
fn map_bytes<K, V>(m: &BTreeMap<K, V>) -> usize {
    m.len() * (16 * size_of::<(K, V)>() + 16 * size_of::<usize>() + 256)
}
fn strings_bytes(v: &Vec<String>) -> usize {
    v.capacity() * size_of::<String>() + 32 + v.iter().map(string_bytes).sum::<usize>()
}
fn definitions_bytes(v: &Vec<LibraryDefinition>) -> usize {
    v.capacity() * size_of::<LibraryDefinition>()
        + 32
        + v.iter()
            .map(|d| [&d.name, &d.path, &d.file, &d.source].into_iter().map(string_bytes).sum::<usize>())
            .sum::<usize>()
}
impl Inputs {
    fn valid(&self, files: &dyn Provider) -> bool {
        self.reads.iter().all(|(p, text)| files.read(p).as_ref() == text.as_ref())
            && self.packages.as_ref().is_none_or(|v| files.package_files() == *v)
            && self.package_queries.iter().all(|(n, v)| files.package_candidates(n) == *v)
            && self.include_queries.iter().all(|(n, v)| files.include_candidates(n) == *v)
            && self
                .candidates
                .iter()
                .all(|((n, sensitive), v)| files.auto_include().is_some_and(|i| i.candidates(n, *sensitive) == *v))
            && self.definitions.iter().all(|(p, v)| files.auto_include().is_some_and(|i| i.definitions(p) == *v))
    }

    fn bytes(&self) -> usize {
        // Charge BTree node slack generously, plus every owned string/vector capacity.
        size_of::<Self>()
            + map_bytes(&self.reads)
            + map_bytes(&self.package_queries)
            + map_bytes(&self.include_queries)
            + map_bytes(&self.candidates)
            + map_bytes(&self.definitions)
            + self.reads.iter().map(|(p, t)| string_bytes(p) + t.as_ref().map_or(0, string_bytes)).sum::<usize>()
            + self.packages.as_ref().map_or(0, strings_bytes)
            + self
                .package_queries
                .iter()
                .chain(&self.include_queries)
                .map(|(n, v)| string_bytes(n) + strings_bytes(v))
                .sum::<usize>()
            + self.candidates.iter().map(|((n, _), v)| string_bytes(n) + definitions_bytes(v)).sum::<usize>()
            + self.definitions.iter().map(|(p, v)| string_bytes(p) + definitions_bytes(v)).sum::<usize>()
    }
}

struct Observed<'a> {
    files: &'a dyn Provider,
    inputs: RefCell<Inputs>,
}
impl Provider for Observed<'_> {
    fn read(&self, path: &str) -> Option<String> {
        if let Some(text) = self.inputs.borrow().reads.get(path) {
            return text.clone();
        }
        let text = self.files.read(path);
        self.inputs.borrow_mut().reads.insert(path.to_string(), text.clone());
        text
    }
    fn package_files(&self) -> Vec<String> {
        let out = self.files.package_files();
        self.inputs.borrow_mut().packages = Some(out.clone());
        out
    }
    fn package_candidates(&self, name: &str) -> Vec<String> {
        let out = self.files.package_candidates(name);
        self.inputs.borrow_mut().package_queries.insert(name.to_string(), out.clone());
        out
    }
    fn include_candidates(&self, name: &str) -> Vec<String> {
        let out = self.files.include_candidates(name);
        self.inputs.borrow_mut().include_queries.insert(name.to_string(), out.clone());
        out
    }
    fn auto_include(&self) -> Option<&dyn LibraryLookup> {
        self.files.auto_include().map(|_| self as &dyn LibraryLookup)
    }
}
impl LibraryLookup for Observed<'_> {
    fn candidates(&self, name: &str, sensitive: bool) -> Vec<LibraryDefinition> {
        let out = self.files.auto_include().unwrap().candidates(name, sensitive);
        self.inputs.borrow_mut().candidates.insert((name.to_string(), sensitive), out.clone());
        out
    }
    fn definitions(&self, path: &str) -> Vec<LibraryDefinition> {
        let out = self.files.auto_include().unwrap().definitions(path);
        self.inputs.borrow_mut().definitions.insert(path.to_string(), out.clone());
        out
    }
}

struct Entry {
    path: String,
    auto_include: bool,
    inputs: Inputs,
    json: String,
    name: String,
    drawings: Rc<Drawings>,
    bytes: usize,
}

pub struct Built {
    pub json: String,
    pub name: String,
    pub hit: bool,
}

#[derive(Serialize)]
pub struct Stats {
    pub limit: usize,
    pub used: usize,
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
}

pub struct BuildCache {
    entries: VecDeque<Entry>, // least recently used first
    active: Option<Rc<Drawings>>,
    limit: usize,
    used: usize,
    hits: u64,
    misses: u64,
}
impl Default for BuildCache {
    fn default() -> Self {
        Self { entries: VecDeque::new(), active: None, limit: DEFAULT_LIMIT, used: 0, hits: 0, misses: 0 }
    }
}
impl BuildCache {
    pub fn stats(&self) -> Stats {
        Stats { limit: self.limit, used: self.used, entries: self.entries.len(), hits: self.hits, misses: self.misses }
    }
    pub fn clear(&mut self) {
        self.entries = VecDeque::new();
        self.used = 0;
        // The currently displayed document must still be able to request Not drawn thumbnails.
    }
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        while self.used > limit {
            self.evict();
        }
        self.entries.shrink_to_fit();
        if limit == 0 {
            self.clear();
        }
    }
    fn evict(&mut self) {
        if let Some(entry) = self.entries.pop_front() {
            self.used -= entry.bytes;
        }
    }
    pub fn unplaced(&self, level: usize, items: &[usize], names: &[String]) -> Result<Vec<UnplacedDrawing>, String> {
        self.active.as_ref().ok_or_else(|| "no document has kept drawings".to_string())?.unplaced(level, items, names)
    }

    pub fn build(&mut self, path: &str, files: &dyn Provider, fresh: bool) -> Result<Built, String> {
        let path = normalize(path);
        let automatic = files.auto_include().is_some();
        if let Some(at) = self.entries.iter().position(|e| e.path == path && e.auto_include == automatic) {
            let entry = self.entries.remove(at).unwrap();
            self.used -= entry.bytes;
            if !fresh && entry.inputs.valid(files) {
                self.active = Some(entry.drawings.clone());
                let built = Built { json: entry.json.clone(), name: entry.name.clone(), hit: true };
                self.used += entry.bytes;
                self.entries.push_back(entry);
                self.hits += 1;
                return Ok(built);
            }
        }
        self.misses += 1;
        let observed = Observed { files, inputs: RefCell::new(Inputs::default()) };
        let (doc, drawings) = rnl_view::build_drawn(&path, &observed, &BuildOptions::default())?;
        let json = serde_json::to_string(&doc).map_err(|e| e.to_string())?;
        let drawings = Rc::new(drawings);
        let inputs = observed.inputs.into_inner();
        let bytes = 2 * size_of::<Entry>()
            + 128
            + string_bytes(&path)
            + string_bytes(&json)
            + string_bytes(&doc.name)
            + inputs.bytes()
            + drawings.retained_bytes();
        self.active = Some(drawings.clone());
        if bytes <= self.limit {
            while self.used > self.limit - bytes {
                self.evict();
            }
            self.entries.shrink_to_fit();
            self.entries.push_back(Entry {
                path,
                auto_include: automatic,
                inputs,
                json: json.clone(),
                name: doc.name.clone(),
                drawings,
                bytes,
            });
            self.used += bytes;
        }
        Ok(Built { json, name: doc.name, hit: false })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnl_runtime::source::{Layered, MemoryProvider};
    use rnl_view::{LibraryIndex, LibraryProvider};

    fn files() -> MemoryProvider {
        MemoryProvider::new()
            .with("/a.sp", "* a\nR1 a 0 1k\n.end\n")
            .with("/b.sp", "* b\nR2 b 0 2k\n.end\n")
            .with("/c.sp", "* c\nR3 c 0 3k\n.end\n")
    }

    #[test]
    fn switching_back_reuses_document_and_reactivates_its_lazy_drawings() {
        let mut cache = BuildCache::default();
        let files = files();
        let a = cache.build("/a.sp", &files, false).unwrap();
        assert!(!a.hit);
        cache.build("/b.sp", &files, false).unwrap();
        assert!(cache.unplaced(0, &[0], &["R1".into()]).is_err());
        let again = cache.build("/a.sp", &files, false).unwrap();
        assert!(again.hit);
        assert_eq!(again.json, a.json);
        assert!(cache.unplaced(0, &[0], &["R1".into()]).unwrap()[0].svg.is_some());
        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cache.stats().misses, 2);
    }

    #[test]
    fn same_size_edits_deleted_files_and_applied_overlays_cannot_reuse_stale_results() {
        let mut cache = BuildCache::default();
        let mut files = files();
        cache.build("/a.sp", &files, false).unwrap();
        files.insert("/a.sp", "* a\nR1 a 0 9k\n.end\n");
        let changed = cache.build("/a.sp", &files, false).unwrap();
        assert!(!changed.hit && changed.json.contains("9k"));
        let edits = MemoryProvider::new().with("/a.sp", "* edited\nRedit a 0 8k\n");
        let overlay = Layered { layers: vec![&edits, &files] };
        let edited = cache.build("/a.sp", &overlay, false).unwrap();
        assert!(!edited.hit && edited.json.contains("Redit"));
        assert!(cache.build("/a.sp", &overlay, false).unwrap().hit);
        assert!(!cache.build("/a.sp", &files, false).unwrap().hit);
        files.files.remove("/a.sp");
        assert!(cache.build("/a.sp", &files, false).is_err());
    }

    #[test]
    fn missing_includes_appearing_and_dependency_edits_invalidate_the_root() {
        let mut cache = BuildCache::default();
        let mut files = MemoryProvider::new().with("/a.sp", "* a\n.include part.inc\n");
        cache.build("/a.sp", &files, false).unwrap();
        assert!(cache.build("/a.sp", &files, false).unwrap().hit);
        files.insert("/part.inc", "R1 a 0 1k\n");
        assert!(!cache.build("/a.sp", &files, false).unwrap().hit);
        files.insert("/part.inc", "R1 a 0 2k\n");
        let changed = cache.build("/a.sp", &files, false).unwrap();
        assert!(!changed.hit && changed.json.contains("2k"));
    }

    #[test]
    fn library_conflicts_and_auto_include_switch_are_part_of_cache_validity() {
        let mut cache = BuildCache::default();
        let files = MemoryProvider::new()
            .with("/top.sp", "* top\nX1 a 0 cell\n")
            .with("/cell.inc", ".subckt cell p n\nR1 p n 1k\n.ends\n")
            .with("/duplicate.inc", ".subckt cell p n\n.ends\n");
        let mut index = LibraryIndex::default();
        index.update(&["/cell.inc".into()], &files, false);
        cache.build("/top.sp", &files, false).unwrap();
        let enabled = LibraryProvider { files: &files, index: &index, enabled: true };
        let imported = cache.build("/top.sp", &enabled, false).unwrap();
        assert!(!imported.hit && imported.json.contains("added by Auto Include"));
        assert!(cache.build("/top.sp", &enabled, false).unwrap().hit);
        index.update(&["/cell.inc".into(), "/duplicate.inc".into()], &files, false);
        let collision = LibraryProvider { files: &files, index: &index, enabled: true };
        let changed = cache.build("/top.sp", &collision, false).unwrap();
        assert!(!changed.hit && changed.json.contains("conflicting definitions"));
        assert!(cache.build("/top.sp", &files, false).unwrap().hit);
    }

    #[test]
    fn lookup_order_and_new_package_candidates_are_checked_even_when_old_files_stay_identical() {
        let mut files = files();
        files.package_dirs = vec!["/lib".into()];
        files.insert("/lib/test.rnl", "package one");
        let observed = Observed { files: &files, inputs: RefCell::new(Inputs::default()) };
        observed.read("/a.sp");
        observed.package_files();
        observed.package_candidates("test");
        observed.include_candidates("part.inc");
        let inputs = observed.inputs.into_inner();
        assert!(inputs.valid(&files));
        files.insert("/lib/2.0/test.rnl", "package two");
        assert!(!inputs.valid(&files));
        let first: Vec<String> = vec!["/first/part.inc".into(), "/second/part.inc".into()];
        let second = vec![first[1].clone(), first[0].clone()];
        let a = rnl_runtime::source::FilePool { files: &files, paths: &first };
        let b = rnl_runtime::source::FilePool { files: &files, paths: &second };
        let observed = Observed { files: &a, inputs: RefCell::new(Inputs::default()) };
        observed.include_candidates("part.inc");
        assert!(!observed.inputs.into_inner().valid(&b));
    }

    #[test]
    fn lru_evicts_the_least_recently_used_result_and_respects_lowered_limits() {
        let mut cache = BuildCache::default();
        let files = files();
        cache.build("/a.sp", &files, false).unwrap();
        cache.build("/b.sp", &files, false).unwrap();
        let budget = cache.stats().used;
        cache.set_limit(budget);
        assert!(cache.build("/a.sp", &files, false).unwrap().hit);
        cache.build("/c.sp", &files, false).unwrap();
        assert!(cache.stats().used <= budget);
        assert!(cache.entries.iter().all(|e| e.path != "/b.sp"));
        assert!(cache.build("/a.sp", &files, false).unwrap().hit);
        assert!(!cache.build("/b.sp", &files, false).unwrap().hit);
        cache.set_limit(0);
        assert_eq!((cache.stats().used, cache.stats().entries), (0, 0));
        assert!(cache.unplaced(0, &[0], &["R2".into()]).is_ok());
        assert!(!cache.build("/b.sp", &files, false).unwrap().hit);
        assert_eq!(cache.stats().entries, 0);
        cache.set_limit(1);
        assert!(!cache.build("/a.sp", &files, false).unwrap().hit);
        assert_eq!(cache.stats().used, 0); // oversized documents still open but are not cached
    }

    #[test]
    fn force_reload_and_clear_rebuild_while_preserving_the_current_thumbnails() {
        let mut cache = BuildCache::default();
        let files = files();
        cache.build("/a.sp", &files, false).unwrap();
        assert!(!cache.build("/a.sp", &files, true).unwrap().hit);
        assert!(cache.build("/a.sp", &files, false).unwrap().hit);
        cache.clear();
        assert_eq!(cache.stats().entries, 0);
        assert!(cache.unplaced(0, &[0], &["R1".into()]).is_ok());
        assert!(!cache.build("/a.sp", &files, false).unwrap().hit);
    }
}
