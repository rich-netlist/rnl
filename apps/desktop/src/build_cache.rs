//! Per-window LRU of the readings of one file of a document in the context of its entry (`rnl_view::build_file`),
//! serialized, and their lazy drawings. A hit checks the complete
//! observed input: source/package text (including failed reads), lookup order and auto-includes.
//! The foreground core owns this cache; completed background builds may be offered to it.
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::mem::size_of;
use std::sync::Arc;
use std::time::Instant;

use rnl_runtime::source::{dirname, join, normalize, LibraryDefinition, LibraryLookup, Provider};
use rnl_view::{Bind, BuildOptions, Drawings, Outline, UnplacedDrawing};

use crate::formats::Formats;
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
    fn path_key(&self, path: &str) -> String {
        self.files.path_key(path)
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

/// The reading of one file of a document, in the context of its entry (`rnl_view::build_file`), as it is kept: its
/// FileResult as JSON, with the texts of the document's other files filled in for the netlist pane, what it read,
/// and the drawings of its objects without a position.
pub struct Prepared {
    entry: String,
    /// The file read, as the document names it (`SourceFile::path`).
    file: String,
    /// Whether the file is the entry's own, the first of the outline's: the one an open of the entry shows.
    own: bool,
    auto_include: bool,
    /// The formats chosen for files that it was built with: another choice is another entry.
    formats: Formats,
    inputs: Inputs,
    json: String,
    name: String,
    drawings: Arc<Drawings>,
    bytes: usize,
    speculative: bool,
    /// How long the reading took, in milliseconds.
    took: f64,
}

pub struct Built {
    /// The FileResult, with the outline of the whole document as `outline`.
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
    pub prefetched: usize,
}

/// Of a file of the document shown, for the entry page (entry-page.js): the symbols its subcircuits give themselves
/// (`FileSymbols::subckts`, None when they cannot be read), whether a reading of it is kept that still holds, and how
/// long the last reading of it kept took, in milliseconds.
#[derive(Serialize)]
pub struct Summary {
    pub subckts: Option<Vec<rnl_view::SubcktSymbol>>,
    pub kept: bool,
    pub took: Option<f64>,
}

pub struct BuildCache {
    entries: VecDeque<Prepared>, // least recently used first
    active: Option<Arc<Drawings>>,
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

/// The options a reading is built with: flylines and the layout hints are asked for, not left to the core's default:
/// View > Show Flylines hides the flylines (flylines.js), and the problems list has the hints (problems.js).
fn options(formats: &Formats, bind: Option<&Bind>, constraints: bool) -> BuildOptions {
    BuildOptions {
        formats: formats.clone(),
        bind: bind.cloned(),
        constraints,
        flylines: true,
        hints: true,
        ..BuildOptions::default()
    }
}

/// The outline of the document `entry` (`rnl_view::outline`), which reads every file it includes, and the file to read:
/// `file`, or the entry's own, the first of the outline's.
fn outline_of(
    entry: &str,
    file: Option<&str>,
    files: &dyn Provider,
    formats: &Formats,
) -> Result<(Outline, String), String> {
    let outline = rnl_view::outline(entry, files, &options(formats, None, false))?;
    let file = match file {
        Some(file) => file.to_string(),
        None => outline.files.first().map(|f| f.path.clone()).ok_or_else(|| format!("{entry} has no file to read"))?,
    };
    Ok((outline, file))
}

/// The JSON of a reading, `json`, an object, with the outline as its `outline`, as the page takes it (`doc.outline`).
fn with_outline(json: &str, outline: &Outline) -> Result<String, String> {
    let outline = serde_json::to_string(outline).map_err(|e| e.to_string())?;
    let body = json.strip_suffix('}').ok_or("a reading is not an object")?;
    Ok(if body.ends_with('{') {
        format!("{body}\"outline\":{outline}}}")
    } else {
        format!("{body},\"outline\":{outline}}}")
    })
}

impl BuildCache {
    pub fn stats(&self) -> Stats {
        Stats {
            limit: self.limit,
            used: self.used,
            entries: self.entries.len(),
            hits: self.hits,
            misses: self.misses,
            prefetched: self.entries.iter().filter(|e| e.speculative).count(),
        }
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
        let at = self.entries.iter().position(|e| e.speculative).unwrap_or(0);
        if let Some(entry) = self.entries.remove(at) {
            self.used -= entry.bytes;
        }
    }
    pub fn unplaced(&self, level: usize, items: &[usize], names: &[String]) -> Result<Vec<UnplacedDrawing>, String> {
        self.active.as_ref().ok_or_else(|| "no document has kept drawings".to_string())?.unplaced(level, items, names)
    }

    fn position(&self, entry: &str, file: &str, auto_include: bool, formats: &Formats) -> Option<usize> {
        self.entries
            .iter()
            .position(|e| e.entry == entry && e.file == file && e.auto_include == auto_include && e.formats == *formats)
    }

    /// The reading kept of `file` of `entry`, taken out of the entries, when it still holds for `files`; one that does
    /// not hold is dropped.
    fn take_valid(&mut self, entry: &str, file: &str, files: &dyn Provider, formats: &Formats) -> Option<Prepared> {
        let at = self.position(entry, file, files.auto_include().is_some(), formats)?;
        let entry = self.entries.remove(at).unwrap();
        self.used -= entry.bytes;
        entry.inputs.valid(files).then_some(entry)
    }

    /// Keep `entry` as the most recently used, when it fits.
    fn keep(&mut self, entry: Prepared) {
        if entry.bytes <= self.limit {
            while self.used > self.limit - entry.bytes {
                self.evict();
            }
            self.used += entry.bytes;
            self.entries.push_back(entry);
        }
    }

    /// Read `file` of the document `entry`, alone in the context of the entry, or the entry's own file when `file` is
    /// None: the reading kept while what it read holds (unless `fresh`), else a new one, which is kept. Its drawings
    /// become the active ones. The result has the outline of the whole document, which is read each time.
    pub fn build(
        &mut self,
        entry: &str,
        file: Option<&str>,
        files: &dyn Provider,
        formats: &Formats,
        fresh: bool,
    ) -> Result<Built, String> {
        let entry = normalize(entry);
        let (outline, file) = outline_of(&entry, file, files, formats)?;
        if let Some(mut kept) = self.take_valid(&entry, &file, files, formats).filter(|_| !fresh) {
            kept.speculative = false;
            self.active = Some(kept.drawings.clone());
            let built = Built { json: with_outline(&kept.json, &outline)?, name: kept.name.clone(), hit: true };
            self.used += kept.bytes;
            self.entries.push_back(kept);
            self.hits += 1;
            return Ok(built);
        }
        self.misses += 1;
        let own = outline.files.first().is_some_and(|f| f.path == file);
        let read = Prepared::build_with(&entry, &file, own, files, formats, None, false)?;
        self.active = Some(read.drawings.clone());
        let built = Built { json: with_outline(&read.json, &outline)?, name: read.name.clone(), hit: false };
        self.keep(read);
        Ok(built)
    }

    /// Read as `build` does, with the parameters of a subcircuit bound (`bind`), to show, or with the constraints of the
    /// levels (`constraints`), which the entries do not hold: its drawings are the active ones now, but it is not kept
    /// among the entries, which are the files' own, as the next time the same file is opened its parameters are its own
    /// again.
    pub fn build_unkept(
        &mut self,
        entry: &str,
        file: Option<&str>,
        files: &dyn Provider,
        formats: &Formats,
        bind: Option<&Bind>,
        constraints: bool,
    ) -> Result<Built, String> {
        let entry = normalize(entry);
        let (outline, file) = outline_of(&entry, file, files, formats)?;
        self.misses += 1;
        let read = Prepared::build_with(&entry, &file, false, files, formats, bind, constraints)?;
        self.active = Some(read.drawings.clone());
        Ok(Built { json: with_outline(&read.json, &outline)?, name: read.name, hit: false })
    }

    /// The reading of `file` of `entry`, as `build` gives it but without the outline and without changing the active
    /// drawings: Check All Files. A new reading is kept as a speculative one is, the first to go.
    pub fn check(
        &mut self,
        entry: &str,
        file: &str,
        files: &dyn Provider,
        formats: &Formats,
    ) -> Result<String, String> {
        let entry = normalize(entry);
        if let Some(kept) = self.take_valid(&entry, file, files, formats) {
            let json = kept.json.clone();
            self.used += kept.bytes;
            self.entries.push_back(kept);
            self.hits += 1;
            return Ok(json);
        }
        self.misses += 1;
        let mut read = Prepared::build_with(&entry, file, false, files, formats, None, false)?;
        read.speculative = true;
        let json = read.json.clone();
        self.keep(read);
        Ok(json)
    }

    /// Of each of `wanted`, files of the document `entry`: its summary for the entry page, read with `files`. Looking
    /// does not use the readings kept: it changes neither their order nor the hits and misses.
    pub fn summaries(
        &self,
        entry: &str,
        wanted: &[String],
        files: &dyn Provider,
        formats: &Formats,
    ) -> BTreeMap<String, Summary> {
        let entry = normalize(entry);
        let automatic = files.auto_include().is_some();
        let options = options(formats, None, false);
        wanted
            .iter()
            .map(|file| {
                let subckts = rnl_view::symbols(&entry, file, files, &options).ok().map(|s| s.subckts);
                let kept = self.position(&entry, file, automatic, formats).map(|at| &self.entries[at]);
                let summary =
                    Summary { subckts, kept: kept.is_some_and(|k| k.inputs.valid(files)), took: kept.map(|k| k.took) };
                (file.clone(), summary)
            })
            .collect()
    }

    /// Do not waste background CPU on an already retained entry. Foreground hits still validate
    /// every dependency, including negative reads and library lookups, before using its result.
    pub fn contains(&self, entry: &str, auto_include: bool, formats: &Formats) -> bool {
        let entry = normalize(entry);
        self.entries
            .iter()
            .any(|e| e.entry == entry && e.own && e.auto_include == auto_include && e.formats == *formats)
    }

    pub fn room(&self) -> usize {
        self.limit.saturating_sub(self.used)
    }

    /// Speculation may use spare space, but must never evict a result the user visited (or a
    /// higher-priority candidate already prepared). It does not change active lazy drawings.
    pub fn offer(&mut self, mut entry: Prepared) -> bool {
        if self.position(&entry.entry, &entry.file, entry.auto_include, &entry.formats).is_some()
            || entry.bytes > self.room()
        {
            return false;
        }
        entry.speculative = true;
        self.used += entry.bytes;
        self.entries.push_back(entry);
        true
    }
}

impl Prepared {
    /// The reading of the entry's own file of the document `entry`, as an open of the entry reads it: for preloading.
    pub fn build(entry: &str, files: &dyn Provider, formats: &Formats) -> Result<Self, String> {
        let entry = normalize(entry);
        let (_, file) = outline_of(&entry, None, files, formats)?;
        Self::build_with(&entry, &file, true, files, formats, None, false)
    }

    /// The reading of `file` of `entry` (a normalized path), with the parameters of a subcircuit bound, and the
    /// constraints of the levels.
    fn build_with(
        entry: &str,
        file: &str,
        own: bool,
        files: &dyn Provider,
        formats: &Formats,
        bind: Option<&Bind>,
        constraints: bool,
    ) -> Result<Self, String> {
        let started = Instant::now();
        let observed = Observed { files, inputs: RefCell::new(Inputs::default()) };
        let (mut result, drawings) =
            rnl_view::build_file_drawn(entry, file, &observed, &options(formats, bind, constraints))?;
        // the texts of the other files, which the reading of one leaves out, for the netlist pane: what they read now
        let dir = dirname(entry);
        for f in result.document.files.iter_mut().filter(|f| f.text.is_empty()) {
            let path = if f.path.starts_with('<') { f.path.clone() } else { join(&dir, &f.path) };
            if let Some(text) = observed.read(&path) {
                f.text = text;
            }
        }
        let json = serde_json::to_string(&result).map_err(|e| e.to_string())?;
        let drawings = Arc::new(drawings);
        let inputs = observed.inputs.into_inner();
        let name = std::mem::take(&mut result.document.name);
        let bytes = 2 * size_of::<Prepared>()
            + 128
            + 2 * entry.len()
            + 2 * file.len()
            + string_bytes(&json)
            + string_bytes(&name)
            + formats.iter().map(|(p, f)| string_bytes(p) + string_bytes(f)).sum::<usize>()
            + inputs.bytes()
            + drawings.retained_bytes();
        Ok(Self {
            entry: entry.to_string(),
            file: file.to_string(),
            own,
            auto_include: files.auto_include().is_some(),
            formats: formats.clone(),
            inputs,
            json,
            name,
            drawings,
            bytes,
            speculative: false,
            took: started.elapsed().as_secs_f64() * 1000.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnl_runtime::source::{Layered, MemoryProvider};
    use rnl_view::{LibraryIndex, LibraryProvider};

    fn none() -> Formats {
        Formats::new()
    }

    #[test]
    fn another_choice_of_formats_is_another_entry() {
        let mut cache = BuildCache::default();
        let spectre = Formats::from([("/a.sp".to_string(), "spectre".to_string())]);
        assert!(!cache.build("/a.sp", None, &files(), &none(), false).unwrap().hit);
        let chosen = cache.build("/a.sp", None, &files(), &spectre, false).unwrap();
        assert!(!chosen.hit);
        assert!(chosen.json.contains(r#""format":"spectre""#));
        assert!(cache.build("/a.sp", None, &files(), &none(), false).unwrap().hit);
        assert!(cache.build("/a.sp", None, &files(), &spectre, false).unwrap().hit);
        assert!(cache.contains("/a.sp", false, &spectre));
        assert!(!cache.contains("/a.sp", false, &Formats::from([("/a.sp".to_string(), "spice".to_string())])));
    }

    /// A file system that ignores case, as on Windows and macOS: paths compare in lower case.
    struct Caseless(MemoryProvider);
    impl Provider for Caseless {
        fn read(&self, path: &str) -> Option<String> {
            self.0.read(&path.to_lowercase())
        }
        fn path_key(&self, path: &str) -> String {
            path.to_lowercase()
        }
    }

    #[test]
    fn a_format_chosen_in_another_case_reaches_the_file_through_every_layer() {
        // the page keeps the choice in lower case; the include is written in another case
        let disk = Caseless(
            MemoryProvider::new()
                .with("/w/top.sp", "* top\n.include \"Models.SCS\"\nR1 a 0 1k\n.end\n")
                .with("/w/models.scs", "simulator lang=spectre\nR2 (a 0) resistor r=1k\n"),
        );
        let formats = Formats::from([("/w/models.scs".to_string(), "spice".to_string())]);
        let held = MemoryProvider::new();
        let layered = Layered { layers: vec![&held, &disk] };
        let mut cache = BuildCache::default();
        let built = cache.build("/W/Top.sp", None, &layered, &formats, false).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&built.json).unwrap();
        let included =
            doc["files"].as_array().unwrap().iter().find(|f| f["path"].as_str().unwrap().ends_with("odels.SCS"));
        assert_eq!(included.map(|f| f["format"].clone()), Some(serde_json::json!("spice")), "{doc:#}");
    }

    fn files() -> MemoryProvider {
        MemoryProvider::new()
            .with("/a.sp", "* a\nR1 a 0 1k\n.end\n")
            .with("/b.sp", "* b\nR2 b 0 2k\n.end\n")
            .with("/c.sp", "* c\nR3 c 0 3k\n.end\n")
    }

    #[test]
    fn background_result_moves_threads_without_replacing_active_drawings() {
        let mut cache = BuildCache::default();
        cache.build("/a.sp", None, &files(), &none(), false).unwrap();
        let prepared = std::thread::spawn(|| Prepared::build("/b.sp", &files(), &none()).unwrap()).join().unwrap();
        assert!(cache.offer(prepared));
        assert_eq!(cache.stats().prefetched, 1);
        assert!(cache.unplaced(0, &[0], &["R1".into()]).unwrap()[0].svg.is_some());
        assert!(cache.build("/b.sp", None, &files(), &none(), false).unwrap().hit);
        assert_eq!(cache.stats().prefetched, 0);
        assert!(cache.unplaced(0, &[0], &["R2".into()]).unwrap()[0].svg.is_some());
    }

    #[test]
    fn speculative_results_cannot_evict_visited_and_are_evicted_first() {
        let mut cache = BuildCache::default();
        cache.build("/a.sp", None, &files(), &none(), false).unwrap();
        let b = Prepared::build("/b.sp", &files(), &none()).unwrap();
        cache.set_limit(cache.used + b.bytes);
        assert!(cache.offer(b));
        assert!(!cache.offer(Prepared::build("/c.sp", &files(), &none()).unwrap()));
        assert_eq!(cache.stats().entries, 2);
        cache.build("/c.sp", None, &files(), &none(), false).unwrap();
        assert!(cache.contains("/a.sp", false, &none()));
        assert!(!cache.contains("/b.sp", false, &none()));
        assert!(cache.used <= cache.limit);
    }

    #[test]
    fn speculative_hit_checks_file_changes_and_fresh_bypasses_it() {
        let mut cache = BuildCache::default();
        assert!(cache.offer(Prepared::build("/a.sp", &files(), &none()).unwrap()));
        let changed = files().with("/a.sp", "* a\nR9 a 0 1k\n.end\n");
        assert!(!cache.build("/a.sp", None, &changed, &none(), false).unwrap().hit);
        assert!(cache.unplaced(0, &[0], &["R9".into()]).unwrap()[0].svg.is_some());
        cache.clear();
        assert!(cache.offer(Prepared::build("/a.sp", &files(), &none()).unwrap()));
        assert!(!cache.build("/a.sp", None, &files(), &none(), true).unwrap().hit);
    }

    #[test]
    fn a_build_with_parameters_bound_shows_them_and_is_not_kept() {
        let mut cache = BuildCache::default();
        let files =
            MemoryProvider::new().with("/p.sp", "* p\n.subckt amp a b w=1u\nR1 a b 1k\n.ends\nX1 a b amp\n.end\n");
        let own = cache.build("/p.sp", None, &files, &none(), false).unwrap();
        assert!(own.json.contains(r#""value":"1u""#));
        let bind = Bind { subckt: "amp".into(), params: BTreeMap::from([("w".to_string(), "2u".to_string())]) };
        let bound = cache.build_unkept("/p.sp", None, &files, &none(), Some(&bind), false).unwrap();
        assert!(!bound.hit);
        assert!(bound.json.contains(r#""value":"2u""#) && bound.json.contains(r#""from":"bound""#));
        // the entry is the file's own: opening it again is a hit, with its own parameters
        let again = cache.build("/p.sp", None, &files, &none(), false).unwrap();
        assert!(again.hit);
        assert_eq!(again.json, own.json);
        assert_eq!(cache.stats().entries, 1);
    }

    #[test]
    fn a_build_with_constraints_has_them_and_is_not_kept() {
        let mut cache = BuildCache::default();
        let files = MemoryProvider::new().with(
            "/p.sp",
            "* p
R1 a b 1k
R2 b c 1k
.end
",
        );
        let own = cache.build("/p.sp", None, &files, &none(), false).unwrap();
        assert!(!own.json.contains(r#""constraints""#));
        let constrained = cache.build_unkept("/p.sp", None, &files, &none(), None, true).unwrap();
        assert!(!constrained.hit);
        assert!(constrained.json.contains(r#""constraints""#) && constrained.json.contains(r#""facts""#));
        // the entry is the file's own: opening it again is a hit, without them
        let again = cache.build("/p.sp", None, &files, &none(), false).unwrap();
        assert!(again.hit);
        assert_eq!(again.json, own.json);
        assert_eq!(cache.stats().entries, 1);
    }

    #[test]
    fn switching_back_reuses_document_and_reactivates_its_lazy_drawings() {
        let mut cache = BuildCache::default();
        let files = files();
        let a = cache.build("/a.sp", None, &files, &none(), false).unwrap();
        assert!(!a.hit);
        cache.build("/b.sp", None, &files, &none(), false).unwrap();
        assert!(cache.unplaced(0, &[0], &["R1".into()]).is_err());
        let again = cache.build("/a.sp", None, &files, &none(), false).unwrap();
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
        cache.build("/a.sp", None, &files, &none(), false).unwrap();
        files.insert("/a.sp", "* a\nR1 a 0 9k\n.end\n");
        let changed = cache.build("/a.sp", None, &files, &none(), false).unwrap();
        assert!(!changed.hit && changed.json.contains("9k"));
        let edits = MemoryProvider::new().with("/a.sp", "* edited\nRedit a 0 8k\n");
        let overlay = Layered { layers: vec![&edits, &files] };
        let edited = cache.build("/a.sp", None, &overlay, &none(), false).unwrap();
        assert!(!edited.hit && edited.json.contains("Redit"));
        assert!(cache.build("/a.sp", None, &overlay, &none(), false).unwrap().hit);
        assert!(!cache.build("/a.sp", None, &files, &none(), false).unwrap().hit);
        files.files.remove("/a.sp");
        assert!(cache.build("/a.sp", None, &files, &none(), false).is_err());
    }

    #[test]
    fn missing_includes_appearing_and_dependency_edits_invalidate_the_root() {
        let mut cache = BuildCache::default();
        let mut files = MemoryProvider::new().with("/a.sp", "* a\n.include part.inc\n");
        cache.build("/a.sp", None, &files, &none(), false).unwrap();
        assert!(cache.build("/a.sp", None, &files, &none(), false).unwrap().hit);
        files.insert("/part.inc", "R1 a 0 1k\n");
        assert!(!cache.build("/a.sp", None, &files, &none(), false).unwrap().hit);
        files.insert("/part.inc", "R1 a 0 2k\n");
        let changed = cache.build("/a.sp", None, &files, &none(), false).unwrap();
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
        cache.build("/top.sp", None, &files, &none(), false).unwrap();
        let enabled = LibraryProvider { files: &files, index: &index, enabled: true };
        let imported = cache.build("/top.sp", None, &enabled, &none(), false).unwrap();
        assert!(!imported.hit && imported.json.contains("subcircuit cell was added from /cell.inc by Auto Include"));
        assert!(cache.build("/top.sp", None, &enabled, &none(), false).unwrap().hit);
        index.update(&["/cell.inc".into(), "/duplicate.inc".into()], &files, false);
        let collision = LibraryProvider { files: &files, index: &index, enabled: true };
        let changed = cache.build("/top.sp", None, &collision, &none(), false).unwrap();
        assert!(!changed.hit && changed.json.contains("times in the Library"));
        assert!(cache.build("/top.sp", None, &files, &none(), false).unwrap().hit);
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
        cache.build("/a.sp", None, &files, &none(), false).unwrap();
        cache.build("/b.sp", None, &files, &none(), false).unwrap();
        let budget = cache.stats().used;
        cache.set_limit(budget);
        assert!(cache.build("/a.sp", None, &files, &none(), false).unwrap().hit);
        cache.build("/c.sp", None, &files, &none(), false).unwrap();
        assert!(cache.stats().used <= budget);
        assert!(cache.entries.iter().all(|e| e.entry != "/b.sp"));
        assert!(cache.build("/a.sp", None, &files, &none(), false).unwrap().hit);
        assert!(!cache.build("/b.sp", None, &files, &none(), false).unwrap().hit);
        cache.set_limit(0);
        assert_eq!((cache.stats().used, cache.stats().entries), (0, 0));
        assert!(cache.unplaced(0, &[0], &["R2".into()]).is_ok());
        assert!(!cache.build("/b.sp", None, &files, &none(), false).unwrap().hit);
        assert_eq!(cache.stats().entries, 0);
        cache.set_limit(1);
        assert!(!cache.build("/a.sp", None, &files, &none(), false).unwrap().hit);
        assert_eq!(cache.stats().used, 0); // oversized documents still open but are not cached
    }

    #[test]
    fn force_reload_and_clear_rebuild_while_preserving_the_current_thumbnails() {
        let mut cache = BuildCache::default();
        let files = files();
        cache.build("/a.sp", None, &files, &none(), false).unwrap();
        assert!(!cache.build("/a.sp", None, &files, &none(), true).unwrap().hit);
        assert!(cache.build("/a.sp", None, &files, &none(), false).unwrap().hit);
        cache.clear();
        assert_eq!(cache.stats().entries, 0);
        assert!(cache.unplaced(0, &[0], &["R1".into()]).is_ok());
        assert!(!cache.build("/a.sp", None, &files, &none(), false).unwrap().hit);
    }

    /// A document of two files: the entry, which instantiates a subcircuit, and the file that defines it.
    fn two_files() -> MemoryProvider {
        MemoryProvider::new()
            .with("/w/top.sp", "* top\n.include \"cells.inc\"\nX1 a 0 cell\n.end\n")
            .with("/w/cells.inc", ".subckt cell p n\nR1 p n 1k\n.ends\n")
    }

    #[test]
    fn a_file_of_the_document_is_read_alone_in_the_context_of_its_entry_with_the_outline() {
        let mut cache = BuildCache::default();
        let files = two_files();
        let built = cache.build("/w/top.sp", Some("cells.inc"), &files, &none(), false).unwrap();
        assert!(!built.hit);
        let doc: serde_json::Value = serde_json::from_str(&built.json).unwrap();
        assert_eq!(doc["file"], "cells.inc");
        assert_eq!(doc["path"], "/w/top.sp");
        let titles: Vec<&str> =
            doc["levels"].as_array().unwrap().iter().map(|l| l["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["cell"]);
        // the outline of the whole document, and the other file's text for the netlist pane
        let outline: Vec<&str> =
            doc["outline"]["files"].as_array().unwrap().iter().map(|f| f["path"].as_str().unwrap()).collect();
        assert_eq!(outline, ["top.sp", "cells.inc"]);
        let top = doc["files"].as_array().unwrap().iter().find(|f| f["path"] == "top.sp").unwrap();
        assert!(top["text"].as_str().unwrap().contains("X1 a 0 cell"), "{top:#}");
        // the entry's own file is another reading
        let own = cache.build("/w/top.sp", None, &files, &none(), false).unwrap();
        assert!(!own.hit && own.json.contains(r#""file":"top.sp""#));
        let again = cache.build("/w/top.sp", Some("cells.inc"), &files, &none(), false).unwrap();
        assert!(again.hit);
        assert_eq!(again.json, built.json);
        assert_eq!(cache.stats().entries, 2);
    }

    #[test]
    fn a_reading_kept_does_not_hold_once_a_file_it_read_changes() {
        let mut cache = BuildCache::default();
        let mut files = two_files();
        cache.build("/w/top.sp", Some("cells.inc"), &files, &none(), false).unwrap();
        files.insert("/w/cells.inc", ".subckt cell p n\nR1 p n 2k\n.ends\n");
        let changed = cache.build("/w/top.sp", Some("cells.inc"), &files, &none(), false).unwrap();
        assert!(!changed.hit && changed.json.contains("2k"));
        // the entry is read in the context of the reading too
        files.insert("/w/top.sp", "* top\n.include \"cells.inc\"\nX1 a 0 cell\nX2 b 0 cell\n.end\n");
        assert!(!cache.build("/w/top.sp", Some("cells.inc"), &files, &none(), false).unwrap().hit);
        // another entry that includes the file is another document
        files.insert("/w/other.sp", "* other\n.include \"cells.inc\"\n.end\n");
        assert!(!cache.build("/w/other.sp", Some("cells.inc"), &files, &none(), false).unwrap().hit);
        assert!(cache.build("/w/top.sp", Some("cells.inc"), &files, &none(), false).unwrap().hit);
    }

    #[test]
    fn a_file_that_is_not_one_of_the_document_is_refused() {
        let mut cache = BuildCache::default();
        let err = cache.build("/w/top.sp", Some("nowhere.inc"), &two_files(), &none(), false).err().unwrap();
        assert!(err.contains("is not a file of"), "{err}");
    }

    #[test]
    fn checking_a_file_keeps_its_reading_without_changing_the_drawings_shown() {
        let mut cache = BuildCache::default();
        let files = two_files();
        cache.build("/w/top.sp", None, &files, &none(), false).unwrap();
        let checked = cache.check("/w/top.sp", "cells.inc", &files, &none()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&checked).unwrap();
        assert!(value["file"] == "cells.inc" && value.get("outline").is_none());
        assert_eq!(cache.stats().prefetched, 1);
        assert!(cache.build("/w/top.sp", Some("cells.inc"), &files, &none(), false).unwrap().hit);
        assert_eq!(cache.check("/w/top.sp", "cells.inc", &files, &none()).unwrap(), checked);
    }

    #[test]
    fn a_summary_tells_the_symbols_and_whether_a_reading_is_kept_without_using_it() {
        let mut cache = BuildCache::default();
        let mut files = two_files();
        let wanted = ["cells.inc".to_string()];
        let before = cache.summaries("/w/top.sp", &wanted, &files, &none());
        let first = &before["cells.inc"];
        let titles = first.subckts.as_ref().map(|s| s.iter().map(|s| s.title.clone()).collect::<Vec<_>>());
        assert_eq!(titles, Some(vec!["cell".to_string()]));
        assert!(!first.kept && first.took.is_none());
        cache.build("/w/top.sp", Some("cells.inc"), &files, &none(), false).unwrap();
        let (hits, misses) = (cache.stats().hits, cache.stats().misses);
        let summaries = cache.summaries("/w/top.sp", &wanted, &files, &none());
        let kept = &summaries["cells.inc"];
        assert!(kept.kept && kept.took.is_some_and(|t| t >= 0.0));
        assert_eq!((cache.stats().hits, cache.stats().misses), (hits, misses));
        files.insert("/w/cells.inc", ".subckt cell p n\nR1 p n 2k\n.ends\n");
        let summaries = cache.summaries("/w/top.sp", &wanted, &files, &none());
        let stale = &summaries["cells.inc"];
        assert!(!stale.kept && stale.took.is_some());
    }

    #[test]
    fn the_outline_is_added_to_a_reading_as_its_last_field() {
        let outline = rnl_view::outline("/w/top.sp", &two_files(), &BuildOptions::default()).unwrap();
        let json = with_outline(r#"{"file":"top.sp"}"#, &outline).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["file"], "top.sp");
        assert_eq!(value["outline"]["path"], "/w/top.sp");
        assert!(serde_json::from_str::<serde_json::Value>(&with_outline("{}", &outline).unwrap()).is_ok());
    }
}
