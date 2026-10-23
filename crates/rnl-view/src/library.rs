//! The viewer's session index. Definitions are parsed when files enter the library, not when a
//! missing instance asks for them. Reload replaces the index, including removed definitions.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use rnl_runtime::diag::{Loc, Sink};
use rnl_runtime::host::format::Reading;
use rnl_runtime::host::{HostDocument, LevelKind};
use rnl_runtime::source::{normalize, LibraryDefinition, LibraryLookup, MemoryProvider, Provider};

#[derive(Clone)]
struct Entry {
    definitions: Vec<LibraryDefinition>,
    /// Definition indexes that require the section selected by this root's include tree.
    sectioned: HashSet<usize>,
    dependencies: HashMap<String, u64>,
}

#[derive(Clone, Default)]
pub struct LibraryIndex {
    entries: BTreeMap<String, Arc<Entry>>,
    // Standalone readings of the physical definition files reached through library roots.
    // These are import targets, not additional library roots or a second source of candidates.
    imports: BTreeMap<String, Arc<Entry>>,
    names: Arc<HashMap<String, Vec<LibraryDefinition>>>,
    dirty: bool,
    /// The formats set for files by path ([`crate::BuildOptions::formats`]), that the library
    /// files and their includes are read with ([`LibraryIndex::set_formats`]).
    formats: BTreeMap<String, String>,
}

fn fingerprint(text: &str) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hash);
    hash.finish()
}

impl LibraryIndex {
    /// Read the library files with the formats set for files by path, as a build reads them
    /// ([`crate::BuildOptions::formats`]); call it before [`LibraryIndex::update`]. The files whose
    /// format changed, and those that read them, are read again at the next update. A format name
    /// that is not known leaves its file as if none were set.
    pub fn set_formats(&mut self, formats: BTreeMap<String, String>) {
        if formats == self.formats {
            return;
        }
        let changed: Vec<String> = formats
            .keys()
            .chain(self.formats.keys())
            .filter(|p| formats.get(*p) != self.formats.get(*p))
            .map(|p| p.replace('\\', "/"))
            .collect();
        self.formats = formats;
        self.invalidate(&changed);
    }

    /// Replacing an existing Library item also invalidates roots that include that item.
    pub fn invalidate(&mut self, paths: &[String]) {
        let paths: HashSet<_> = paths.iter().map(|p| normalize(p)).collect();
        let count = self.entries.len();
        self.entries.retain(|_, entry| !entry.dependencies.keys().any(|p| paths.contains(p)));
        self.dirty |= count != self.entries.len();
        let count = self.imports.len();
        self.imports.retain(|_, entry| !entry.dependencies.keys().any(|p| paths.contains(p)));
        self.dirty |= count != self.imports.len();
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
            match Self::entry(path, files, &self.formats) {
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
            self.refresh_imports(files, reload);
            self.reindex();
        }
        errors.sort();
        errors
    }

    fn entry(path: &str, files: &dyn Provider, formats: &BTreeMap<String, String>) -> Option<Entry> {
        let text = files.read(path)?;
        let mut definitions = Vec::new();
        let mut sectioned = HashSet::new();
        let mut dependencies = HashMap::from([(path.to_string(), fingerprint(&text))]);
        if !path.to_ascii_lowercase().ends_with(".rnl") {
            let mut sink = Sink::new();
            // a library file is read on its own, by its format, its first line a statement
            let set = crate::formats::known_formats(formats, files);
            let main = crate::formats::main_format(path, &text, None, &set, files);
            let doc =
                crate::formats::with(main, &set, files, |f| Reading::read(path, files, &mut sink, f, &[], false))?;
            for (id, lv) in doc.levels.iter().enumerate() {
                let source = normalize(&lv.path);
                if let std::collections::hash_map::Entry::Vacant(entry) = dependencies.entry(source) {
                    if let Some(text) = files.read(entry.key()) {
                        entry.insert(fingerprint(&text));
                    }
                }
                if lv.kind == LevelKind::Subckt {
                    let mut ancestor = Some(id);
                    while let Some(at) = ancestor {
                        if doc.levels[at].section.is_some() {
                            sectioned.insert(definitions.len());
                            break;
                        }
                        ancestor = doc.levels[at].parent;
                    }
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
        Some(Entry { definitions, sectioned, dependencies })
    }

    fn refresh_imports(&mut self, files: &dyn Provider, reload: bool) {
        let paths: HashSet<_> = self.entries.values().flat_map(|e| e.definitions.iter().map(|d| &d.file)).collect();
        let mut imports = BTreeMap::new();
        for path in paths {
            let entry = self
                .entries
                .get(path)
                .cloned()
                .or_else(|| (!reload).then(|| self.imports.get(path).cloned()).flatten())
                .or_else(|| Self::entry(path, files, &self.formats).map(Arc::new));
            if let Some(entry) = entry {
                imports.insert(path.clone(), entry);
            }
        }
        self.imports = imports;
    }

    fn reindex(&mut self) {
        let mut names: HashMap<String, Vec<LibraryDefinition>> = HashMap::new();
        let direct: HashSet<_> = self
            .imports
            .iter()
            .flat_map(|(path, entry)| {
                entry
                    .definitions
                    .iter()
                    .filter(move |d| d.file == *path)
                    .map(|d| (d.file.as_str(), d.line, d.name.as_str()))
            })
            .collect();
        for entry in self.entries.values() {
            for (id, def) in entry.definitions.iter().enumerate() {
                let mut target = def.clone();
                // An umbrella/testbench can lead us to a definition without being its import
                // target. Import the defining file directly if it exposes this exact definition
                // on its own. Otherwise retain the wrapper (e.g. a selected .lib section).
                if !entry.sectioned.contains(&id) && direct.contains(&(def.file.as_str(), def.line, def.name.as_str()))
                {
                    target.path = def.file.clone();
                }
                names.entry(def.name.to_lowercase()).or_default().push(target);
            }
        }
        // An included definition reached through several library roots is still one definition.
        // Prefer the defining file itself when it is independently importable, then stable path order.
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
            if let Some(entry) = Self::entry(path, files, &self.formats) {
                out.entries.insert(path.clone(), Arc::new(entry));
            } else {
                out.entries.remove(path);
            }
        }
        let count = out.imports.len();
        out.imports.retain(|_, entry| {
            !edits.iter().any(|(p, hash)| entry.dependencies.get(*p).is_some_and(|previous| hash != previous))
        });
        if !changed.is_empty() || count != out.imports.len() {
            out.refresh_imports(files, false);
            out.reindex();
        }
        out
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
        self.imports.get(path).or_else(|| self.entries.get(path)).map(|e| e.definitions.clone()).unwrap_or_default()
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
    fn path_key(&self, path: &str) -> String {
        self.files.path_key(path)
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
        let mut approved = HashSet::new();
        let mut reserved: HashMap<String, LibraryDefinition> = HashMap::new();
        for lv in &doc.levels {
            for inst in &lv.instances {
                let Some(model) = inst.model.as_deref() else { continue };
                if !inst.is_subckt || doc.subckt_of(inst).is_some() {
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
                    // A library can define thousands of missing masters. Validate its complete
                    // definition list once per pass, not once per referenced master (quadratic).
                    if approved.contains(&def.path) {
                        automatic.push((model.to_string(), def.source.clone(), loc));
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
                        approved.insert(def.path.clone());
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
    fn nested_definition_imports_its_own_file_not_the_testbench_that_includes_the_entry() {
        for spectre in [false, true] {
            let (top, tb, child, leaf, ext) = if spectre {
                (
                    "simulator lang=spectre\nsubckt top (a b)\nX1 (a b) cell\nends top\n",
                    "include \"top.scs\"\ninclude \"cells/cell.scs\"\nXtb (a b) top\n",
                    "include \"leaf.scs\"\nsubckt cell (p n)\nX2 (p n) leaf\nends cell\n",
                    "subckt leaf (p n)\nends leaf\n",
                    "scs",
                )
            } else {
                (
                    "* title\n.subckt top a b\nX1 a b cell\n.ends\n",
                    ".include top.sp\n.include cells/cell.sp\nXtb a b top\n",
                    ".include leaf.sp\n.subckt cell p n\nX2 p n leaf\n.ends\n",
                    ".subckt leaf p n\n.ends\n",
                    "sp",
                )
            };
            let top_path = format!("/top.{ext}");
            let tb_path = format!("/tb.{ext}");
            let child_path = format!("/cells/cell.{ext}");
            let leaf_path = format!("/cells/leaf.{ext}");
            let files = MemoryProvider::new()
                .with(&top_path, top)
                .with(&tb_path, tb)
                .with(&child_path, child)
                .with(&leaf_path, leaf);
            // PATH exposes only the two root files; the cell is discovered through the TB.
            let index = index(&files, &[&top_path, &tb_path]);
            let defs = index.candidates("cell", spectre);
            assert_eq!(defs.len(), 1);
            assert_eq!(defs[0].path, child_path);
            assert_eq!(index.definitions(&child_path).len(), 2);
            let (doc, sink) = host(&files, &index, true, &top_path);
            assert!(doc.subckt("cell").is_some() && doc.subckt("leaf").is_some());
            assert!(doc.sources.iter().all(|(path, _)| !path.ends_with(&format!("tb.{ext}"))));
            assert!(sink.all().iter().all(|d| d.severity == rnl_runtime::diag::Severity::Warning), "{:?}", sink.all());
            assert_eq!(sink.all().len(), 1);
            assert!(host(&files, &index, false, &top_path).0.subckt("cell").is_none());
        }
    }

    #[test]
    fn nested_import_checks_collateral_conflicts_in_its_own_include_tree() {
        let files = MemoryProvider::new()
            .with("/top.sp", "* title\n.subckt known p n\n.ends\nX1 a 0 cell\n")
            .with("/tb.sp", ".include cells/cell.sp\n")
            .with("/cells/cell.sp", ".include other.sp\n.subckt cell p n\n.ends\n")
            .with("/cells/other.sp", ".subckt known p n\n.ends\n");
        let index = index(&files, &["/tb.sp"]);
        let (doc, sink) = host(&files, &index, true, "/top.sp");
        assert!(doc.subckt("cell").is_none());
        assert!(sink.all().iter().any(|d| d.message.contains("known conflicts")));
    }

    #[test]
    fn a_section_only_definition_keeps_its_selecting_wrapper() {
        for spectre in [false, true] {
            let (top, wrapper, sections, ext) = if spectre {
                ("simulator lang=spectre\nX1 (a b) cell\n",
                 "include \"cells.scs\" section=tt\n",
                 "library cells\nsection tt\nsubckt cell (p n)\nends cell\nendsection tt\nsection ff\nsubckt hidden (p n)\nends hidden\nendsection ff\nendlibrary cells\n", "scs")
            } else {
                (
                    "* title\nX1 a b cell\n",
                    ".lib cells.sp tt\n",
                    ".lib tt\n.subckt cell p n\n.ends\n.endl tt\n.lib ff\n.subckt hidden p n\n.ends\n.endl ff\n",
                    "sp",
                )
            };
            let top_path = format!("/top.{ext}");
            let wrapper_path = format!("/wrapper.{ext}");
            let files = MemoryProvider::new()
                .with(&top_path, top)
                .with(&wrapper_path, wrapper)
                .with(&format!("/cells.{ext}"), sections);
            let index = index(&files, &[&wrapper_path]);
            assert_eq!(index.candidates("cell", spectre)[0].path, wrapper_path);
            assert!(index.candidates("hidden", spectre).is_empty());
            let (doc, sink) = host(&files, &index, true, &top_path);
            assert!(doc.subckt("cell").is_some());
            assert!(doc.subckt("hidden").is_none());
            assert!(sink.all().iter().all(|d| d.severity == rnl_runtime::diag::Severity::Warning), "{:?}", sink.all());
        }
    }

    #[test]
    fn a_definition_included_by_a_selected_section_keeps_its_model_context() {
        for spectre in [false, true] {
            let (top, wrapper, sections, cell, ext) = if spectre {
                ("simulator lang=spectre\nX1 (a b) cell\n", "include \"corners.scs\" section=tt\n",
                 "library corners\nsection tt\nmodel nch bsim4 type=n\ninclude \"cell.scs\"\nendsection tt\nendlibrary corners\n",
                 "subckt cell (p n)\nM1 (p p n n) nch\nends cell\n", "scs")
            } else {
                (
                    "* title\nX1 a b cell\n",
                    ".lib corners.sp tt\n",
                    ".lib tt\n.model nch nmos\n.include cell.sp\n.endl tt\n",
                    ".subckt cell p n\nM1 p p n n nch\n.ends\n",
                    "sp",
                )
            };
            let top_path = format!("/top.{ext}");
            let wrapper_path = format!("/wrapper.{ext}");
            let files = MemoryProvider::new()
                .with(&top_path, top)
                .with(&wrapper_path, wrapper)
                .with(&format!("/corners.{ext}"), sections)
                .with(&format!("/cell.{ext}"), cell);
            let index = index(&files, &[&wrapper_path]);
            assert_eq!(index.candidates("cell", spectre)[0].path, wrapper_path);
            let (doc, _) = host(&files, &index, true, &top_path);
            let cell = &doc.levels[doc.subckt("cell").unwrap()];
            assert_eq!(cell.instances[0].mtype.as_deref(), Some("nmos"));
            assert!(!cell.instances[0].model_missing);
        }
    }

    #[test]
    fn nested_import_cache_tracks_edits_refresh_reload_and_library_removal() {
        let mut files = MemoryProvider::new()
            .with("/top.sp", "* title\nX1 a b cell\n")
            .with("/tb.sp", ".include cells/cell.sp\n")
            .with("/cells/cell.sp", ".subckt cell p n\n.ends\n");
        let mut index = index(&files, &["/tb.sp"]);
        let edits = MemoryProvider::new().with("/cells/cell.sp", ".subckt edited p n\n.ends\n");
        let layered = Layered { layers: vec![&edits, &files] };
        let overlay = index.with_edits(&edits, &layered);
        assert!(overlay.candidates("cell", false).is_empty());
        assert_eq!(overlay.candidates("edited", false)[0].path, "/cells/cell.sp");
        assert_eq!(overlay.definitions("/cells/cell.sp")[0].name, "edited");
        assert_eq!(index.candidates("cell", false).len(), 1);
        files.insert("/cells/cell.sp", ".subckt refreshed p n\n.ends\n");
        index.invalidate(&["/cells/cell.sp".into()]);
        index.update(&["/tb.sp".into()], &files, false);
        assert_eq!(index.candidates("refreshed", false)[0].path, "/cells/cell.sp");
        assert_eq!(index.definitions("/cells/cell.sp")[0].name, "refreshed");
        files.files.remove("/cells/cell.sp");
        index.update(&["/tb.sp".into()], &files, true);
        assert!(index.candidates("refreshed", false).is_empty());
        assert!(index.definitions("/cells/cell.sp").is_empty());
        files.insert("/cells/cell.sp", ".subckt restored p n\n.ends\n");
        index.update(&["/tb.sp".into()], &files, true);
        assert_eq!(index.candidates("restored", false).len(), 1);
        index.update(&[], &files, false);
        assert!(index.candidates("restored", false).is_empty());
        assert!(index.imports.is_empty());
    }

    #[test]
    fn many_missing_masters_validate_their_library_only_once() {
        use std::cell::Cell;
        struct Count<'a> {
            files: &'a MemoryProvider,
            index: &'a LibraryIndex,
            validations: Cell<usize>,
        }
        impl LibraryLookup for Count<'_> {
            fn candidates(&self, name: &str, sensitive: bool) -> Vec<LibraryDefinition> {
                self.index.candidates(name, sensitive)
            }
            fn definitions(&self, path: &str) -> Vec<LibraryDefinition> {
                self.validations.set(self.validations.get() + 1);
                self.index.definitions(path)
            }
        }
        impl Provider for Count<'_> {
            fn read(&self, path: &str) -> Option<String> {
                self.files.read(path)
            }
            fn auto_include(&self) -> Option<&dyn LibraryLookup> {
                Some(self)
            }
        }
        let mut top = "* top\n".to_string();
        let mut lib = String::new();
        for i in 0..200 {
            top.push_str(&format!("X{i} a 0 cell{i}\n"));
            lib.push_str(&format!(".subckt cell{i} p n\nR1 p n 1k\n.ends\n"));
        }
        let files = MemoryProvider::new().with("/top.sp", &top).with("/cells.inc", &lib);
        let index = index(&files, &["/cells.inc"]);
        let count = Count { files: &files, index: &index, validations: Cell::new(0) };
        let mut sink = Sink::new();
        let doc = crate::read_host("/top.sp", None, &count, &mut sink).unwrap();
        assert_eq!(doc.subckts.len(), 200);
        assert_eq!(count.validations.get(), 1);
        assert_eq!(sink.all().iter().filter(|d| d.message.contains("added by Auto Include")).count(), 200);
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
