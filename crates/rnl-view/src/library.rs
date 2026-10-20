//! The viewer's session index. Definitions are parsed when files enter the library, not when a
//! missing instance asks for them. Reload replaces the index, including removed definitions.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use rnl_runtime::diag::{Loc, Sink};
use rnl_runtime::host::{HostDocument, LevelKind};
use rnl_runtime::source::{normalize, LibraryDefinition, LibraryLookup, MemoryProvider, Provider};

#[derive(Clone)]
struct Entry {
    definitions: Vec<LibraryDefinition>,
    dependencies: HashMap<String, u64>,
}

#[derive(Clone, Default)]
pub struct LibraryIndex {
    entries: BTreeMap<String, Arc<Entry>>,
    names: Arc<HashMap<String, Vec<LibraryDefinition>>>,
    dirty: bool,
}

fn fingerprint(text: &str) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hash);
    hash.finish()
}

impl LibraryIndex {
    /// Replacing an existing Library item also invalidates roots that include that item.
    pub fn invalidate(&mut self, paths: &[String]) {
        let paths: HashSet<_> = paths.iter().map(|p| normalize(p)).collect();
        let count = self.entries.len();
        self.entries.retain(|_, entry| !entry.dependencies.keys().any(|p| paths.contains(p)));
        self.dirty |= count != self.entries.len();
    }

    /// Only new files are read normally. Reload reads every file, packages included. Failed reads
    /// remove stale definitions and are reported to the caller; subsequent updates may retry them.
    pub fn update(&mut self, paths: &[String], files: &dyn Provider, reload: bool) -> Vec<String> {
        let paths: HashSet<_> = paths.iter().map(|p| normalize(p)).collect();
        let count = self.entries.len();
        self.entries.retain(|p, _| paths.contains(p));
        self.dirty |= count != self.entries.len();
        let mut errors = Vec::new();
        for path in &paths {
            if !reload && self.entries.contains_key(path) {
                continue;
            }
            match Self::entry(path, files) {
                Some(entry) => {
                    self.dirty = true;
                    self.entries.insert(path.clone(), Arc::new(entry));
                }
                None => {
                    self.dirty |= self.entries.remove(path).is_some();
                    errors.push(format!("Cannot read library file {path}"));
                }
            }
        }
        if self.dirty {
            self.reindex();
        }
        errors.sort();
        errors
    }

    fn entry(path: &str, files: &dyn Provider) -> Option<Entry> {
        let text = files.read(path)?;
        let mut definitions = Vec::new();
        let mut dependencies = HashMap::from([(path.to_string(), fingerprint(&text))]);
        if !path.to_ascii_lowercase().ends_with(".rnl") {
            let mut sink = Sink::new();
            let doc = if super::is_spectre(path, &text, None) {
                rnl_host_spectre::read_library(path, files, &mut sink)
            } else {
                rnl_host_spice::read_library(path, files, &mut sink)
            }?;
            for lv in &doc.levels {
                let source = normalize(&lv.path);
                if !dependencies.contains_key(&source) {
                    if let Some(text) = files.read(&source) {
                        dependencies.insert(source, fingerprint(&text));
                    }
                }
                if lv.kind == LevelKind::Subckt {
                    definitions.push(LibraryDefinition {
                        name: lv.name.clone(),
                        path: path.to_string(),
                        file: normalize(&lv.path),
                        source: normalize(&lv.path),
                        line: lv.line,
                    });
                }
            }
        }
        Some(Entry { definitions, dependencies })
    }

    fn reindex(&mut self) {
        let mut names: HashMap<String, Vec<LibraryDefinition>> = HashMap::new();
        for entry in self.entries.values() {
            for def in &entry.definitions {
                names.entry(def.name.to_lowercase()).or_default().push(def.clone());
            }
        }
        // An included definition reached through several library roots is still one definition.
        // Prefer the defining file itself when it is also a library item, then stable path order.
        for definitions in names.values_mut() {
            definitions.sort_by_key(|d| (d.path != d.file, d.path.clone()));
            let mut seen = HashSet::new();
            definitions.retain(|d| seen.insert((d.file.clone(), d.line, d.name.clone())));
        }
        self.names = Arc::new(names);
        self.dirty = false;
    }

    /// Applied text takes precedence, including a changed dependency of an indexed library root.
    /// The stored index is not changed by unsaved text in one document.
    pub fn with_edits(&self, edits: &MemoryProvider, files: &dyn Provider) -> Self {
        let mut out = self.clone();
        let edits: HashMap<_, _> = edits.files.iter().map(|(p, t)| (p, fingerprint(t))).collect();
        let changed: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(path, entry)| {
                let dirty =
                    edits.iter().any(|(p, hash)| entry.dependencies.get(*p).is_some_and(|previous| hash != previous));
                dirty.then_some(path.clone())
            })
            .collect();
        for path in &changed {
            if let Some(entry) = Self::entry(path, files) {
                out.entries.insert(path.clone(), Arc::new(entry));
            } else {
                out.entries.remove(path);
            }
        }
        if !changed.is_empty() {
            out.reindex();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnl_runtime::source::Layered;

    fn index(files: &MemoryProvider, paths: &[&str]) -> LibraryIndex {
        let mut index = LibraryIndex::default();
        assert!(index.update(&paths.iter().map(|p| p.to_string()).collect::<Vec<_>>(), files, false).is_empty());
        index
    }

    fn host(files: &dyn Provider, index: &LibraryIndex, enabled: bool, path: &str) -> (HostDocument, Sink) {
        let provider = LibraryProvider { files, index, enabled };
        let mut sink = Sink::new();
        let doc = crate::read_host(path, None, &provider, &mut sink).unwrap();
        (doc, sink)
    }

    #[test]
    fn opt_in_preserves_source_text_lines_and_resolves_recursive_dependencies() {
        let files = MemoryProvider::new()
            .with("/top.sp", "* title\nX1 a 0 cell\n.end\n")
            .with("/lib/cell.inc", ".subckt cell p n\nX2 p n leaf\n.ends\n")
            .with("/lib/leaf.inc", ".subckt leaf p n\nR1 p n 1k\n.ends\n");
        let index = index(&files, &["/lib/cell.inc", "/lib/leaf.inc"]);
        let (off, warnings) = host(&files, &index, false, "/top.sp");
        assert!(off.subckt("cell").is_none());
        assert!(warnings.all().is_empty());
        let (doc, sink) = host(&files, &index, true, "/top.sp");
        assert!(doc.subckt("cell").is_some() && doc.subckt("leaf").is_some());
        assert_eq!(doc.main().instances[0].line, 2);
        assert_eq!(doc.sources[0].1, files.read("/top.sp").unwrap());
        assert_eq!(sink.all().iter().filter(|d| d.message.contains("added by Auto Include")).count(), 2);
        assert!(sink.all().iter().all(|d| d.severity == rnl_runtime::diag::Severity::Warning));
    }

    #[test]
    fn conflicts_are_not_arbitrarily_selected_and_explicit_include_wins() {
        let mut files = MemoryProvider::new()
            .with("/top.sp", "* title\nX1 a 0 CELL\n")
            .with("/a.inc", ".subckt cell p n\n.ends\n")
            .with("/b.inc", ".subckt CELL p n\n.ends\n");
        let index = index(&files, &["/a.inc", "/b.inc"]);
        let (doc, sink) = host(&files, &index, true, "/top.sp");
        assert!(doc.subckt("cell").is_none());
        assert!(sink.all().iter().any(|d| d.message.contains("conflicting definitions")
            && d.message.contains("/a.inc")
            && d.message.contains("/b.inc")));
        files.insert("/top.sp", "* title\n.include \"/a.inc\"\nX1 a 0 CELL\n");
        let (doc, sink) = host(&files, &index, true, "/top.sp");
        assert!(doc.subckt("cell").is_some());
        assert!(!sink.all().iter().any(|d| d.message.contains("Auto Include")));
    }

    #[test]
    fn same_file_duplicate_and_collateral_redefinition_are_conflicts() {
        let files = MemoryProvider::new()
            .with("/top.sp", "* title\n.subckt known p n\n.ends\nX1 a 0 cell\n")
            .with("/dup.inc", ".subckt duplicate p n\n.ends\n.subckt duplicate p n\n.ends\n")
            .with("/cell.inc", ".subckt cell p n\n.ends\n.subckt known p n\n.ends\n");
        let index = index(&files, &["/dup.inc", "/cell.inc"]);
        assert_eq!(index.candidates("duplicate", false).len(), 2);
        let (doc, sink) = host(&files, &index, true, "/top.sp");
        assert!(doc.subckt("cell").is_none());
        assert!(sink.all().iter().any(|d| d.message.contains("known conflicts")));
    }

    #[test]
    fn spectre_is_case_sensitive_and_auto_imported_model_cards_are_resolved() {
        let files = MemoryProvider::new()
            .with("/top.scs", "simulator lang=spectre\nX1 (a 0) Cell\n")
            .with(
                "/cell.scs",
                "simulator lang=spectre\nmodel nch bsim4 type=n\nsubckt Cell (p n)\nM1 (p p n n) nch\nends Cell\n",
            )
            .with("/other.scs", "subckt cell (p n)\nends cell\n");
        let index = index(&files, &["/cell.scs", "/other.scs"]);
        let (doc, sink) = host(&files, &index, true, "/top.scs");
        let cell = &doc.levels[doc.subckt("Cell").unwrap()];
        assert_eq!(cell.instances[0].mtype.as_deref(), Some("nmos"));
        assert!(!cell.instances[0].model_missing);
        assert!(doc.subckt("cell").is_none());
        assert_eq!(sink.all().len(), 1);
    }

    #[test]
    fn reload_exclusion_and_failed_reads_remove_stale_definitions_and_read_packages() {
        let mut files =
            MemoryProvider::new().with("/cell.inc", ".subckt old p n\n.ends\n").with("/pack.rnl", "package");
        let mut index = index(&files, &["/cell.inc", "/pack.rnl"]);
        files.insert("/cell.inc", ".subckt new p n\n.ends\n");
        let paths = vec!["/cell.inc".to_string(), "/pack.rnl".to_string()];
        index.update(&paths, &files, false);
        assert_eq!(index.candidates("old", false).len(), 1);
        index.update(&paths, &files, true);
        assert!(index.candidates("old", false).is_empty());
        assert_eq!(index.candidates("new", false).len(), 1);
        files.files.remove("/cell.inc");
        files.files.remove("/pack.rnl");
        assert_eq!(index.update(&paths, &files, true).len(), 2);
        assert!(index.candidates("new", false).is_empty());
        files.insert("/cell.inc", ".subckt fresh p n\n.ends\n");
        index.update(&paths, &files, false);
        index.update(&[], &files, false);
        assert!(index.candidates("fresh", false).is_empty());
    }

    #[test]
    fn applied_library_edits_reindex_only_the_document_overlay() {
        let files = MemoryProvider::new()
            .with("/top.sp", "* title\nX1 a 0 new\n")
            .with("/cell.inc", ".subckt old p n\n.ends\n");
        let index = index(&files, &["/cell.inc"]);
        let edits = MemoryProvider::new().with("/cell.inc", ".subckt new p n\n.ends\n");
        let layered = Layered { layers: vec![&edits, &files] };
        let changed = index.with_edits(&edits, &layered);
        let (doc, _) = host(&layered, &changed, true, "/top.sp");
        assert!(doc.subckt("new").is_some());
        assert!(index.candidates("new", false).is_empty());
    }

    #[test]
    fn included_definition_is_not_a_conflict_when_reached_through_two_library_files() {
        let files = MemoryProvider::new()
            .with("/top.sp", "* title\nX1 a 0 cell\n")
            .with("/first.inc", ".include \"child.inc\"\n")
            .with("/second.inc", ".include \"child.inc\"\n")
            .with("/child.inc", ".subckt cell p n\n.ends\n");
        let index = index(&files, &["/first.inc", "/second.inc", "/child.inc"]);
        let defs = index.candidates("cell", false);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].path, "/child.inc");
        assert!(host(&files, &index, true, "/top.sp").0.subckt("cell").is_some());
    }

    #[test]
    fn repeated_auto_include_uses_the_index_without_reading_unrelated_library_files() {
        use std::cell::RefCell;
        struct Count {
            files: MemoryProvider,
            reads: RefCell<HashMap<String, usize>>,
        }
        impl Provider for Count {
            fn read(&self, p: &str) -> Option<String> {
                *self.reads.borrow_mut().entry(p.to_string()).or_default() += 1;
                self.files.read(p)
            }
        }
        let files = Count {
            files: MemoryProvider::new()
                .with("/top.sp", "* title\nX1 a 0 cell\n")
                .with("/cell.inc", ".subckt cell p n\n.ends\n")
                .with("/unrelated.inc", ".subckt unused p n\n.ends\n"),
            reads: RefCell::default(),
        };
        let mut index = LibraryIndex::default();
        let paths = vec!["/cell.inc".to_string(), "/unrelated.inc".to_string()];
        index.update(&paths, &files, false);
        files.reads.borrow_mut().clear();
        for _ in 0..3 {
            index.update(&paths, &files, false);
            assert!(host(&files, &index, true, "/top.sp").0.subckt("cell").is_some());
        }
        assert!(!files.reads.borrow().contains_key("/unrelated.inc"));
    }
}

impl LibraryLookup for LibraryIndex {
    fn candidates(&self, name: &str, case_sensitive: bool) -> Vec<LibraryDefinition> {
        self.names
            .get(&name.to_lowercase())
            .into_iter()
            .flatten()
            .filter(|d| !case_sensitive || d.name == name)
            .cloned()
            .collect()
    }
    fn definitions(&self, path: &str) -> Vec<LibraryDefinition> {
        self.entries.get(path).map(|e| e.definitions.clone()).unwrap_or_default()
    }
}

/// Enables the viewer's opt-in behavior over normal file access. With it off, all reads use the
/// ordinary host adapter, and there is no library lookup in the parser.
pub struct LibraryProvider<'a> {
    pub files: &'a dyn Provider,
    pub index: &'a LibraryIndex,
    pub enabled: bool,
}

impl Provider for LibraryProvider<'_> {
    fn read(&self, path: &str) -> Option<String> {
        self.files.read(path)
    }
    fn package_files(&self) -> Vec<String> {
        self.files.package_files()
    }
    fn package_candidates(&self, name: &str) -> Vec<String> {
        self.files.package_candidates(name)
    }
    fn include_candidates(&self, name: &str) -> Vec<String> {
        self.files.include_candidates(name)
    }
    fn auto_include(&self) -> Option<&dyn LibraryLookup> {
        self.enabled.then_some(self.index)
    }
}

/// First resolve normal includes. Add uniquely indexed library files until no new dependency can
/// be resolved. Each pass uses the host reader, so model cards, relative includes and line numbers
/// retain their normal meaning. Discard intermediate diagnostics; only the final reading is shown.
pub(crate) fn resolve(
    path: &str,
    files: &dyn Provider,
    sink: &mut Sink,
    reader: impl Fn(&[String], &mut Sink) -> Option<HostDocument>,
) -> Option<HostDocument> {
    let Some(index) = files.auto_include() else { return reader(&[], sink) };
    let mut extra = Vec::new();
    let mut conflicts = Vec::new();
    let mut automatic = Vec::new();
    let mut attempted = HashSet::new();
    loop {
        let mut parsed = Sink::new();
        let doc = reader(&extra, &mut parsed)?;
        let mut next = Vec::new();
        let mut reserved: HashMap<String, LibraryDefinition> = HashMap::new();
        for lv in &doc.levels {
            for inst in &lv.instances {
                let Some(model) = inst.model.as_deref() else { continue };
                if !inst.is_subckt || doc.subckt(model).is_some() {
                    continue;
                }
                let key = if inst.case_sensitive { model.to_string() } else { model.to_lowercase() };
                if !attempted.insert((inst.case_sensitive, key)) {
                    continue;
                }
                let candidates = index.candidates(model, inst.case_sensitive);
                let loc = Some(Loc::line(&inst.file, inst.line));
                if candidates.len() > 1 {
                    let names: Vec<_> = candidates.iter().map(|d| format!("{}:{}", d.source, d.line)).collect();
                    conflicts.push((model.to_string(), format!("Auto Include: subcircuit {model} has conflicting definitions in {}; add an explicit include to choose one", names.join(", ")), loc));
                } else if let Some(def) = candidates.first() {
                    if extra.contains(&def.path) || normalize(path) == def.path {
                        continue;
                    }
                    let definitions = index.definitions(&def.path);
                    let mut own: HashMap<String, LibraryDefinition> = HashMap::new();
                    let conflict = definitions.iter().find(|d| {
                        let key = doc.key(&d.name);
                        let present = doc.subckt(&d.name).is_some_and(|id| {
                            let existing = &doc.levels[id];
                            normalize(&existing.path) != d.file || existing.line != d.line
                        });
                        let other = reserved.get(&key).or_else(|| own.get(&key));
                        let duplicate = other.is_some_and(|other| other.file != d.file || other.line != d.line);
                        own.insert(key, (*d).clone());
                        present || duplicate
                    });
                    if let Some(other) = conflict {
                        conflicts.push((model.to_string(), format!("Auto Include: {} was not included for subcircuit {model} because its definition of {} conflicts with an existing definition", def.source, other.name), loc));
                    } else {
                        automatic.push((model.to_string(), def.source.clone(), loc));
                        for d in definitions {
                            reserved.insert(doc.key(&d.name), d);
                        }
                        if !next.contains(&def.path) {
                            next.push(def.path.clone());
                        }
                    }
                }
            }
        }
        if !next.is_empty() {
            extra.extend(next);
            continue;
        }
        *sink = parsed;
        for (model, message, loc) in conflicts {
            if doc.subckt(&model).is_none() {
                sink.warning(message, loc);
            }
        }
        for (model, file, loc) in automatic {
            let message = if doc.subckt(&model).is_some() {
                format!("Subcircuit {model} was not included; found in {file} and added by Auto Include")
            } else {
                format!("Auto Include could not resolve subcircuit {model} from {file}; reload Library to refresh its definitions")
            };
            sink.warning(message, loc);
        }
        return Some(doc);
    }
}
