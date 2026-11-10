//! Upgrade supported earlier RNL files to 0.4, without filesystem writes.
//!
//! Reading, rewriting and verification are separate phases. A failed input read returns before
//! any output is prepared. Target diagnostics and semantic differences remain in the report.

mod collections;
mod compare;
mod directions;
mod edit;
mod host_terminals;
mod legacy;
mod local_symbols;
mod manual;
mod mechanical;
mod references;
mod rewrite;
mod selections;
mod sets;
mod standard;
mod terminals;

use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::diag::Severity;
use rnl_runtime::host::{HostDocument, LevelKind, Record};
use rnl_runtime::interp::unit::{scan_language_decl, scan_package_decl, DEFAULT_VERSION};
use rnl_runtime::source::{dirname, join, normalize, Layered, MemoryProvider, Provider};
use rnl_runtime::{Interpretation, Options};
use serde::Serialize;

/// Inputs to the reader. Solver records are validated, then discarded before verification.
#[derive(Clone, Debug)]
pub struct UpgradeOptions {
    pub dialect: Option<String>,
    pub formats: BTreeMap<String, String>,
    pub std: bool,
}

impl Default for UpgradeOptions {
    fn default() -> Self {
        Self { dialect: None, formats: BTreeMap::new(), std: true }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Change {
    pub rule: String,
    pub line: u32,
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Unconverted {
    pub rule: String,
    pub line: u32,
    pub before: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct FileReport {
    pub path: String,
    pub from: String,
    pub to: String,
    pub changes: Vec<Change>,
    pub unconverted: Vec<Unconverted>,
    /// Solver records were discarded; solving the upgraded document again is required.
    pub needs_solve: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub severity: String,
    pub code: Option<String>,
    pub name: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Difference {
    pub file: String,
    pub level: String,
    pub category: String,
    pub item: String,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub files: Vec<FileReport>,
    pub diagnostics: Vec<Diagnostic>,
    pub differences: Vec<Difference>,
}

impl Report {
    pub fn successful(&self) -> bool {
        self.files.iter().all(|f| f.unconverted.is_empty())
            && self.differences.is_empty()
            && !self.diagnostics.iter().any(|d| matches!(d.severity.as_str(), "error" | "violation"))
    }
}

/// Proposed texts, keyed by their original paths. The caller chooses where to save them.
#[derive(Clone, Debug)]
pub struct Upgrade {
    pub entry: String,
    pub files: BTreeMap<String, String>,
    pub read: Vec<String>,
    pub report: Report,
}

fn diagnostics(read: &Interpretation) -> Vec<Diagnostic> {
    read.diagnostics()
        .into_iter()
        .map(|d| Diagnostic {
            severity: d.severity.as_str().into(),
            code: d.code.number(),
            name: d.code.name().into(),
            file: d.loc.as_ref().map(|l| l.file.clone()),
            line: d.loc.as_ref().map(|l| l.line),
            message: read.sink.render(d),
        })
        .collect()
}

fn effective_version(doc: &HostDocument, level: usize) -> String {
    let lv = &doc.levels[level];
    for rec in &lv.records {
        if let Some(v) = scan_language_decl(&rec.text) {
            return v;
        }
    }
    lv.parent.map_or_else(|| DEFAULT_VERSION.into(), |parent| effective_version(doc, parent))
}

/// Read an input and prepare its upgrade. An error means no output should be written.
pub fn upgrade(entry: &str, files: &dyn Provider, options: &UpgradeOptions) -> Result<Upgrade, String> {
    let entry = normalize(entry);
    if files.read(&entry).is_some_and(|text| text.contains('\0')) {
        return Err("cannot upgrade text containing NUL characters; use UTF-8 or Latin-1".into());
    }
    let runtime = Options { std: options.std, problems: true, generated: true, ..Options::default() };
    let mut before = rnl_view::interpret_with(&entry, options.dialect.as_deref(), &options.formats, files, &runtime)?;
    let errors: Vec<_> = before
        .diagnostics()
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| before.sink.render(d))
        .collect();
    if !errors.is_empty() || before.model.failed.is_some() || before.unsupported.iter().any(Option::is_some) {
        return Err(format!("cannot upgrade an unreadable source; nothing was written\n{}", errors.join("\n")));
    }
    let authored = Options { generated: false, ..runtime.clone() };
    if before.doc.levels.iter().flat_map(|l| &l.records).any(|r| rnl_runtime::writeback::record_type(r) == "generated")
    {
        before = rnl_view::interpret_with(&entry, options.dialect.as_deref(), &options.formats, files, &authored)?;
    }
    // Package checking keeps its parsed units outside the host document. Recover their
    // carrier locations through the same scanner used by the package reader.
    let package_entry = rnl_view::entry_format(&entry, options.dialect.as_deref(), &options.formats, files)? == "rnl";
    for level in &mut before.doc.levels {
        if (level.kind == LevelKind::Package || (package_entry && level.path == entry)) && level.records.is_empty() {
            if let Some(source) = files.read(&level.path) {
                level.records = rnl_runtime::records::scan_run(
                    &rnl_runtime::records::package_lines(&source),
                    &level.path,
                    &mut rnl_runtime::diag::Sink::default(),
                );
            }
        }
    }
    let bundled = rnl_view::bundled();
    let mut packages: BTreeSet<_> = bundled
        .files
        .iter()
        .filter(|(path, _)| path.starts_with("<bundled>/0.4/"))
        .filter_map(|(_, text)| scan_package_decl(text).map(|(name, _)| name))
        .collect();
    // A familiar name is not proof that a package belongs to the standard
    // series. Keep owned overrides and their paths in the file group (16.2).
    for (_, source) in &before.sources {
        if let Some((name, _)) = scan_package_decl(source) {
            if !bundled.files.values().any(|known| known == source) {
                packages.remove(&name);
            }
        }
    }
    let mut source_records: BTreeMap<String, Vec<Record>> = BTreeMap::new();
    let mut versions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (level, lv) in before.doc.levels.iter().enumerate() {
        let path = normalize(&lv.path);
        if matches!(lv.kind, LevelKind::File | LevelKind::Package) {
            versions.entry(path.clone()).or_default().insert(effective_version(&before.doc, level));
        }
        let records = source_records.entry(path).or_default();
        for record in &lv.records {
            if !records.iter().any(|r| r.lines == record.lines && r.text == record.text) {
                records.push(record.clone());
            }
        }
    }
    // Own packages reached by path are part of the group, including transitive references.
    // Named package-directory dependencies remain external; target validation reports an old
    // language dependency until its owner supplies an upgraded package.
    let mut package_docs = BTreeMap::new();
    let mut inspected = BTreeSet::new();
    while let Some(path) = source_records.keys().find(|p| !inspected.contains(*p)).cloned() {
        inspected.insert(path.clone());
        for reference in rewrite::package_paths(&source_records[&path], &packages) {
            let dependency = join(&dirname(&path), &reference);
            if source_records.contains_key(&dependency) || files.read(&dependency).is_none() {
                continue;
            }
            let package = rnl_view::interpret_with(&dependency, Some("rnl"), &options.formats, files, &runtime)?;
            if package.diagnostics().iter().any(|d| d.severity == Severity::Error) {
                return Err(format!("cannot upgrade unreadable package {dependency}; nothing was written"));
            }
            let mut doc = package.doc;
            let source = files.read(&dependency).ok_or_else(|| format!("cannot reread {dependency}"))?;
            let records = rnl_runtime::records::scan_run(
                &rnl_runtime::records::package_lines(&source),
                &dependency,
                &mut rnl_runtime::diag::Sink::default(),
            );
            doc.levels[0].records = records.clone();
            versions.entry(dependency.clone()).or_default().insert(effective_version(&doc, 0));
            source_records.insert(dependency.clone(), records);
            package_docs.insert(dependency, doc);
        }
    }
    let context = rewrite::Context {
        std: options.std,
        packages,
        references: references::References::new(&before),
        sets: sets::Sets::new(&before, &entry),
    };
    let mut output = BTreeMap::new();
    let mut report = Report { schema_version: 1, files: Vec::new(), diagnostics: Vec::new(), differences: Vec::new() };
    let mut read = Vec::new();
    for (path, records) in source_records {
        let source = files.read(&path).ok_or_else(|| format!("cannot reread {path}"))?;
        if source.contains('\0') {
            return Err(format!("cannot upgrade {path}: text contains NUL characters; use UTF-8 or Latin-1"));
        }
        read.push(path.clone());
        let choices = versions.get(&path);
        if choices.is_some_and(|v| v.len() > 1) {
            return Err(format!("{path} is read in several language versions; upgrade those readings separately"));
        }
        let version = choices.and_then(|v| v.first()).map(String::as_str).unwrap_or(DEFAULT_VERSION);
        if !matches!(version, "0.2" | "0.3" | "0.4") {
            return Err(format!("cannot upgrade language {version} in {path}"));
        }
        if version == "0.4" || (records.is_empty() && path != entry) {
            continue;
        }
        let mut file_report = FileReport {
            path: path.clone(),
            from: version.into(),
            to: "0.4".into(),
            changes: Vec::new(),
            unconverted: Vec::new(),
            needs_solve: false,
        };
        let doc = package_docs.get(&path).unwrap_or(&before.doc);
        let text = rewrite::file(doc, &path, &source, &records, &context, &mut file_report)?;
        output.insert(path, text);
        report.files.push(file_report);
    }
    let overlay = MemoryProvider { files: output.clone(), package_dirs: Vec::new() };
    let provider = Layered { layers: vec![&overlay, files] };
    let target_options = Options { std: false, ..authored };
    let after =
        rnl_view::interpret_with(&entry, options.dialect.as_deref(), &options.formats, &provider, &target_options)?;
    report.diagnostics = diagnostics(&after);
    report.differences = compare::compare(&before, &after);
    Ok(Upgrade { entry, files: output, read, report })
}
