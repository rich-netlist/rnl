//! The reading of one file of a document in the context of its entry ([`crate::build_file`]): the
//! Document of the file's levels, the subcircuits they instantiate and where those are defined,
//! and what the reading depends on, for a viewer to keep the result and know when it still holds.
//! And the symbols the subcircuits of a file give themselves ([`crate::symbols`]), what the levels
//! that instantiate them use of them.

use std::collections::{BTreeSet, HashSet};

use rnl_runtime::host::{dialect_key, HostDocument, LevelKind};
use rnl_runtime::model::BlockSymbol;
use rnl_runtime::numbering::{hex, sha256};
use rnl_runtime::per_file::renumbered;
use rnl_runtime::Interpretation;
use serde::Serialize;

use crate::document::symbols::inspect_block;
use crate::{AutoIncludeCandidate, Document, InspectedSymbol};

/// The version of the shape of [`FileResult`] and of what its hashes are made of: a viewer that
/// keeps results drops those of another version.
pub const FILE_RESULT_FORMAT: u32 = 1;

/// The reading of one file of a document in the context of its entry ([`crate::build_file`]): the
/// Document of the file's levels, with the subcircuits they instantiate and what it depends on.
/// Its `levels` are the file's, each with its `host` as the outline numbers the levels; its `files`
/// are all the files of the document, in the order of the outline's, but only the file read has its
/// text, records and failed statements; its objects and spans are those of the file's levels,
/// numbered for this result alone; its diagnostics are those of the file's levels, and those of
/// reading the document that are about the file or the files it depends on ([`FileInputs`]).
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FileResult {
    /// The Document of the file's levels.
    #[serde(flatten)]
    #[cfg_attr(test, ts(flatten))]
    pub document: Document,
    /// The file read, as the document names it (`SourceFile::path`).
    pub file: String,
    /// The subcircuits the file's levels instantiate, each name once, in the order first
    /// instantiated.
    pub children: Vec<FileChild>,
    /// What the result depends on.
    pub inputs: FileInputs,
}

/// A subcircuit that a file's levels instantiate.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FileChild {
    /// Its name, as the first instance writes it.
    pub master: String,
    /// The level that defines it, by its index in the document's levels, as `Level::host` and the
    /// outline's levels give it; None when nothing defines it.
    pub host: Option<usize>,
    /// The file that defines it, as the document names it; None when nothing defines it.
    pub file: Option<String>,
    /// The title of the level that defines it; None when nothing defines it.
    pub title: Option<String>,
    /// When nothing defines it and Auto Include found it defined more than once in the Library,
    /// those definitions, as `Child::candidates` gives them; left out otherwise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(test, ts(as = "Option<Vec<AutoIncludeCandidate>>", optional))]
    pub candidates: Vec<AutoIncludeCandidate>,
}

/// What a reading of one file depends on: with the core, its version and the options of the
/// reading, a reading with the same inputs gives the same result.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FileInputs {
    /// The version of the core, with its commit: "0.6.1 (d96a7a66)".
    pub core: String,
    /// The version of the shape of the result ([`FILE_RESULT_FORMAT`]).
    pub format: u32,
    /// The entry, as the request names it.
    pub entry: String,
    /// The file read, as the document names it.
    pub file: String,
    /// The files and packages the result depends on, each once: the file read, the files above
    /// it, the files that define the subcircuits it instantiates and the files above those, and
    /// the packages read.
    pub files: Vec<FileInput>,
    /// The facts of the whole document the result depends on: where the names the file's levels
    /// instantiate are defined, and the global nets.
    pub outline: FileOutline,
}

/// A file or package a reading of one file depends on.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FileInput {
    /// "body", the file read; "context", a file above it, whose rules and definitions reach it
    /// (10.1); "symbol", a file that defines a subcircuit it instantiates, or a file above that
    /// one; "package", a package file read.
    #[cfg_attr(test, ts(type = "\"body\" | \"context\" | \"symbol\" | \"package\""))]
    pub role: &'static str,
    /// The file as the document names it (`SourceFile::path`); a package by its file name.
    pub path: String,
    /// The first 16 hexadecimal digits of the SHA-256 of its text, as `OutlineFile::text_hash`.
    pub hash: String,
    /// What of the file the result depends on, when it is less than its text: of a file above,
    /// the hash of its records (`OutlineFile::records_hash`); of a file that defines a subcircuit
    /// instantiated, the hash of the symbols of its subcircuits (`FileSymbols::hash`). Not to be
    /// relied on before a format version says so; left out for the others.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub narrow: Option<String>,
    /// A package bundled with the core, which only a version of the core changes; left out
    /// otherwise.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(test, ts(as = "Option<bool>", optional))]
    pub bundled: bool,
}

/// The facts of the whole document a reading of one file depends on.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FileOutline {
    /// Where each subcircuit the file's levels instantiate is defined, as [`FileChild`] gives it.
    pub calls: Vec<FileCall>,
    /// The global nets, as `Outline::globals` gives them.
    pub globals: Vec<String>,
}

/// Where a name a file's levels instantiate is defined.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FileCall {
    pub master: String,
    pub file: Option<String>,
    pub title: Option<String>,
}

/// The symbols the subcircuits of a file give themselves (9.2.6) ([`crate::symbols`]).
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FileSymbols {
    /// The file, as the document names it.
    pub file: String,
    /// Its subcircuits, in the order of the file.
    pub subckts: Vec<SubcktSymbol>,
    /// The hash of all of them, as `FileInput::narrow` gives it for a file that defines a
    /// subcircuit instantiated.
    pub hash: String,
    /// The files the core asked for and did not get, as `Document::missing`.
    pub missing: Vec<String>,
}

/// What the levels that instantiate a subcircuit use of it.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SubcktSymbol {
    pub title: String,
    /// Its level, by its index in the document's levels.
    pub host: usize,
    pub ports: Vec<String>,
    /// The symbol it gives itself with `# @SET-SYMBOL`, as `Document::symbols` gives a symbol;
    /// None when it gives itself none, and its instances are drawn with a default block symbol.
    pub symbol: Option<InspectedSymbol>,
    /// The first 16 hexadecimal digits of the SHA-256 of its title, its ports and its symbol.
    pub hash: String,
}

/// The first 16 hexadecimal digits of the SHA-256 of `text`.
fn hash(text: &str) -> String {
    hex(&sha256(text.as_bytes()))[..16].to_string()
}

/// The version of the core, with its commit.
fn core_version() -> String {
    format!("{} ({})", env!("CARGO_PKG_VERSION"), env!("RNL_BUILD_COMMIT"))
}

/// The text a subcircuit's symbol hash is made of: its title, its ports and its symbol, with the
/// numbers of the reading made the order they come in ([`renumbered`]).
fn symbol_text(doc: &HostDocument, lv: usize, block: Option<&BlockSymbol>) -> String {
    let level = &doc.levels[lv];
    let symbol = block.map(|b| format!("{:?}\n{:?}\n{:?}", b.info, b.view, b.symbols)).unwrap_or_default();
    let symbol = renumbered(&[symbol]).remove(0);
    format!("{}\n{}\n{}\n", level.title(), level.ports.join(" "), symbol)
}

/// The hash of the symbols of the subcircuits `subckts` of a file, each by its own hash.
fn file_symbols_hash(hashes: &[String]) -> String {
    hash(&hashes.iter().map(|h| format!("{h}\n")).collect::<String>())
}

/// Make the Document of a reading of `file` a [`FileResult`]: the hosts of the subcircuits its
/// levels instantiate, the texts of the other files left out, and what it depends on.
pub(crate) fn file_result(entry: &str, file: &str, r: &Interpretation, mut d: Document) -> FileResult {
    let doc = &r.doc;
    let given: Vec<usize> = (0..doc.levels.len()).filter(|&lv| doc.levels[lv].file == file).collect();

    // the subcircuits instantiated, by the levels drawn and by every level of the file
    for (lv, level) in r.model.levels.iter().zip(&mut d.levels) {
        let instances = doc.levels[lv.level].instances.iter().filter(|i| i.is_subckt);
        for (inst, child) in instances.zip(&mut level.children) {
            child.host = doc.subckt_of(inst);
        }
    }
    let mut children: Vec<FileChild> = Vec::new();
    let mut seen: HashSet<(Option<usize>, String)> = HashSet::new();
    for &lv in &given {
        for inst in doc.levels[lv].instances.iter().filter(|i| i.is_subckt) {
            let master = inst.model.clone().unwrap_or_default();
            let host = doc.subckt_of(inst);
            // the name compares by the rules of the file that writes the instance, as it is looked up
            if !seen.insert((host, dialect_key(doc.rules(&inst.file), &master))) {
                continue;
            }
            let candidates = d
                .levels
                .iter()
                .flat_map(|l| &l.children)
                .find(|c| c.instance == inst.name && c.master == master)
                .map(|c| c.candidates.clone())
                .unwrap_or_default();
            children.push(FileChild {
                master,
                host,
                file: host.map(|h| doc.levels[h].file.clone()),
                title: host.map(|h| doc.levels[h].title()),
                candidates,
            });
        }
    }

    // the files it depends on, and what of each
    let text_of = |path: &str| d.files.iter().find(|f| f.path == path).map(|f| f.text.as_str());
    let mut inputs: Vec<FileInput> = Vec::new();
    let mut add = |role: &'static str, path: &str, text: Option<&str>, narrow: Option<String>| {
        if inputs.iter().any(|i| i.path == path && i.role != "package") {
            return;
        }
        let Some(text) = text else { return };
        inputs.push(FileInput { role, path: path.to_string(), hash: hash(text), narrow, bundled: false });
    };
    add("body", file, text_of(file), None);
    // the files above, from the top
    let above = |lv: usize| -> Vec<String> {
        let mut files: Vec<String> = Vec::new();
        for a in doc.ancestors(lv).into_iter().rev() {
            let f = &doc.levels[a].file;
            if !files.contains(f) {
                files.push(f.clone());
            }
        }
        files
    };
    for &lv in &given {
        for f in above(lv) {
            add("context", &f, text_of(&f), Some(crate::outline::records_hash(doc, &f)));
        }
    }
    // the files that define the subcircuits instantiated, then the files above each
    let symbols_of = |f: &str| -> String {
        let hashes: Vec<String> = r
            .block_symbols
            .iter()
            .filter(|(lv, _)| doc.levels[*lv].file == f)
            .map(|(lv, b)| hash(&symbol_text(doc, *lv, b.as_ref())))
            .collect();
        file_symbols_hash(&hashes)
    };
    for c in &children {
        let Some(h) = c.host else { continue };
        let f = &doc.levels[h].file;
        add("symbol", f, text_of(f), Some(symbols_of(f)));
    }
    for c in &children {
        let Some(h) = c.host else { continue };
        for f in above(h) {
            add("symbol", &f, text_of(&f), None);
        }
    }
    // the packages read, as a frontend can tell those it has from those bundled with the core
    let host_files: HashSet<&str> = doc.sources.iter().map(|(p, _)| p.as_str()).collect();
    let bundled: HashSet<&str> = crate::bundled_texts();
    for (name, text) in r.sources.iter().filter(|(p, _)| !host_files.contains(p.as_str())) {
        let input = FileInput {
            role: "package",
            path: name.clone(),
            hash: hash(text),
            narrow: None,
            bundled: bundled.contains(text.as_str()),
        };
        if !inputs.contains(&input) {
            inputs.push(input);
        }
    }

    // the diagnostics of reading the document that are about files it does not depend on are left
    // out: the outline gives them
    let depends: HashSet<usize> = d
        .files
        .iter()
        .enumerate()
        .filter(|(_, f)| inputs.iter().any(|i| i.role != "package" && i.path == f.path))
        .map(|(k, _)| k)
        .collect();
    d.diagnostics.retain(|g| g.file_index.is_none_or(|f| depends.contains(&f)));
    // the texts of the other files are the frontend's already
    for f in d.files.iter_mut().filter(|f| f.path != file) {
        f.text = String::new();
        f.records.clear();
        f.failed.clear();
        f.semantics.clear();
    }

    let calls = children
        .iter()
        .map(|c| FileCall { master: c.master.clone(), file: c.file.clone(), title: c.title.clone() })
        .collect();
    let mut globals: BTreeSet<String> = doc.globals.iter().cloned().collect();
    globals.extend(doc.foreign_globals.iter().map(|(g, _)| g.clone()));
    globals.remove("0");
    let inputs = FileInputs {
        core: core_version(),
        format: FILE_RESULT_FORMAT,
        entry: entry.to_string(),
        file: file.to_string(),
        files: inputs,
        outline: FileOutline { calls, globals: globals.into_iter().collect() },
    };
    FileResult { document: d, file: file.to_string(), children, inputs }
}

/// The symbols of the subcircuits of `file` in a reading with [`rnl_runtime::Options::symbols_only`].
pub(crate) fn file_symbols(file: &str, r: &Interpretation, missing: Vec<String>) -> FileSymbols {
    let subckts: Vec<SubcktSymbol> = r
        .block_symbols
        .iter()
        .filter(|(lv, _)| r.doc.levels[*lv].kind == LevelKind::Subckt)
        .map(|(lv, b)| SubcktSymbol {
            title: r.doc.levels[*lv].title(),
            host: *lv,
            ports: r.doc.levels[*lv].ports.clone(),
            symbol: b.as_ref().map(|b| inspect_block(r, b)),
            hash: hash(&symbol_text(&r.doc, *lv, b.as_ref())),
        })
        .collect();
    let hashes: Vec<String> = subckts.iter().map(|s| s.hash.clone()).collect();
    FileSymbols { file: file.to_string(), hash: file_symbols_hash(&hashes), subckts, missing }
}
