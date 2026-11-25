//! The outline of a document: what reading its files gives before any RNL is interpreted. A viewer
//! shows the tree of its levels, the files and how they include each other from it, and keys what
//! it keeps of each file by it, without building the Document.

use std::collections::{BTreeSet, HashMap, HashSet};

use rnl_runtime::diag::{Diag, Sink};
use rnl_runtime::host::{dialect_key, file_name, package, HostDocument, LevelKind, Record};
use rnl_runtime::interp::unit::{header_packages, is_package_named, is_read_version, package_paths, PackagePath};
use rnl_runtime::numbering::{hex, sha256};
use rnl_runtime::records::{package_lines, scan_package_run};
use rnl_runtime::source::{dirname, join, relative_to, Provider};
use rnl_runtime::syntax::{parse_record_head, Value};
use serde::Serialize;

use crate::document::diagnostic;
use crate::{Diagnostic, SourceInclude};

/// What reading a document gives before any RNL is interpreted, as [`crate::outline`] gives it:
/// its files, its levels and which subcircuits each calls, the language version and the packages
/// each file's header record declares, and the diagnostics of reading the netlist.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Outline {
    /// The entry as the frontend named it.
    pub path: String,
    /// Its file name.
    pub name: String,
    /// The format of the main file, as [`crate::Document::dialect`] names it.
    pub dialect: String,
    /// The host files, in the order of [`crate::Document::files`]: the opened file first, then the
    /// files it includes.
    pub files: Vec<OutlineFile>,
    /// Every level of the netlist, in the order of the host levels: the index of a level here is
    /// the host level that [`crate::Level::host`] and [`crate::Object::level`] name.
    pub levels: Vec<OutlineLevel>,
    /// The subcircuits that no instance of the netlist calls, the roots of its tree, as indices
    /// into `levels`, in the order of the levels. A Verilog-A module is not one.
    pub roots: Vec<usize>,
    /// The global nets the netlist declares (`.global`, Spectre's `global`), `0` aside, each by
    /// the key its names compare by, in sorted order. A global net is global in every level,
    /// whichever file declares it.
    pub globals: Vec<String>,
    /// The diagnostics of reading the netlist: of the host readers, such as an include that is not
    /// found, and of the language versions of the files. Those of interpreting the RNL are not
    /// among them.
    pub diagnostics: Vec<Diagnostic>,
    /// The files the core asked for and did not get, as [`crate::Document::missing`]: a frontend
    /// that can read them adds them and asks again. Among them are the package files that records
    /// name by `path=`, read as the reading reads them, so that the reading that follows has them.
    pub missing: Vec<String>,
}

/// A host file of an [`Outline`].
#[derive(Serialize, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct OutlineFile {
    /// The file as the runtime knows it, as [`crate::SourceFile::path`].
    pub path: String,
    /// The file as shown, as [`crate::SourceFile::name`].
    pub name: String,
    /// The format the file was read with, as [`crate::SourceFile::format`].
    pub format: Option<String>,
    /// The format it is read with when none is set for it.
    pub default_format: Option<String>,
    /// Its include directives and the files they loaded, as [`crate::SourceFile::includes`].
    pub includes: Vec<SourceInclude>,
    /// Its levels, as indices into [`Outline::levels`], in order: its file level, or one for each
    /// time the netlist reads it, then its subcircuits.
    pub levels: Vec<usize>,
    /// The hash of its text: the first 16 hexadecimal digits of the SHA-256 of its UTF-8 bytes.
    pub text_hash: String,
    /// The hash of its records, the header record among them, in the order of the file: of their
    /// texts as the runtime reads them, each ended by a line feed, as `text_hash` is made. A
    /// change of the file's netlist alone leaves it as it is.
    pub records_hash: String,
    /// How many records it holds besides its header records (`type="header"`, 2.8), each once,
    /// though the netlist reads the file more than once: 0 for a file whose RNL is a header
    /// record alone, or that has none.
    pub records: usize,
}

/// A level of an [`Outline`]: a reading of a file, or a subcircuit.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct OutlineLevel {
    /// "file", "subckt", or "module" for a module of a Verilog-A file.
    pub kind: &'static str,
    /// The subcircuit's name, or the name of the file of a file level.
    pub title: String,
    /// The file it is written in, an index into [`Outline::files`]; None when it is not one of them.
    pub file: Option<usize>,
    /// The first line of its definition, and the line of its end, as [`crate::Level::lines`]
    /// gives them; None for a file level.
    pub lines: Option<[u32; 2]>,
    /// The level it is written in, an index into [`Outline::levels`]: for a subcircuit, the level
    /// whose text holds its definition; for an included file, the level that includes it. None
    /// for the main file.
    pub parent: Option<usize>,
    /// Whether it is a circuit, one with instances and nets of its own: the main file and the
    /// subcircuits (6.1).
    pub circuit: bool,
    /// Its terminals, in the order its definition writes them.
    pub ports: Vec<String>,
    /// How many instances it has; for a module of Verilog-A, the instances of modules it writes.
    pub instances: usize,
    /// The subcircuits its instances call, one for each name, in the order first called; for a
    /// module of Verilog-A, the modules its instances name (2.9), though it has no circuit.
    pub calls: Vec<OutlineCall>,
    /// For a file level, the language version its text is read in: the one its header record
    /// declares, or the one of the file that includes it, or the tool's default (16.1).
    pub version: Option<String>,
    /// For a file level, the packages its header record uses, by name, in the order written
    /// (16.2); which version of each is used, the interpretation says.
    pub packages: Vec<String>,
}

/// The instances of a level that call one subcircuit name.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct OutlineCall {
    /// The name as the first of them writes it.
    pub master: String,
    /// The level that defines it, an index into [`Outline::levels`]; None when the netlist does
    /// not define it, such as a device of a PDK.
    pub level: Option<usize>,
    /// How many instances call it.
    pub count: usize,
    /// When the netlist defines it, the names of the instances that call it, in the order of the
    /// netlist; left out otherwise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(test, ts(as = "Option<Vec<String>>", optional))]
    pub instances: Vec<String>,
    /// The line of each of `instances`, in the same order, counted from 1 as `lines` of
    /// [`OutlineLevel`] counts them, in the file of the level; left out with them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(test, ts(as = "Option<Vec<u32>>", optional))]
    pub lines: Vec<u32>,
}

/// The outline of a host document read with the host reader's diagnostics in `sink`, and the
/// language versions of its file levels in `versions`
/// ([`rnl_runtime::interp::read_file_versions`]).
pub(crate) fn outline(
    entry: &str,
    doc: HostDocument,
    sink: Sink,
    versions: &[Option<String>],
    missing: Vec<String>,
) -> Outline {
    let dir = dirname(entry);
    let shown = |file: &str| relative_to(file, &dir).unwrap_or(file).to_string();

    // the files in the order of the Document's: the opened file first
    let mut paths: Vec<&str> = doc.sources.iter().map(|(p, _)| p.as_str()).collect();
    if let Some(k) = paths.iter().position(|p| *p == doc.main().file) {
        let main = paths.remove(k);
        paths.insert(0, main);
    }
    let file_index: HashMap<String, usize> = paths.iter().enumerate().map(|(k, p)| (p.to_string(), k)).collect();
    let mut files: Vec<OutlineFile> = paths
        .iter()
        .map(|&p| {
            let text = doc.source(p).unwrap_or_default();
            OutlineFile {
                path: p.to_string(),
                name: shown(p),
                format: doc.file_format(p).map(|f| f.format.clone()),
                default_format: doc.file_format(p).map(|f| f.default.clone()),
                includes: Vec::new(),
                levels: Vec::new(),
                text_hash: hash(text),
                records_hash: String::new(),
                records: 0,
            }
        })
        .collect();

    let mut called = vec![false; doc.levels.len()];
    let mut levels = Vec::with_capacity(doc.levels.len());
    for (k, lv) in doc.levels.iter().enumerate() {
        let file = file_index.get(&lv.file).copied();
        if let Some(f) = file {
            files[f].levels.push(k);
        }
        if let (Some(site), Some(target)) = (&lv.include_site, file) {
            if let Some(&at) = file_index.get(&site.file) {
                let link = SourceInclude { line: site.line, file: target };
                if !files[at].includes.contains(&link) {
                    files[at].includes.push(link);
                }
            }
        }
        let mut calls: Vec<OutlineCall> = Vec::new();
        let mut by_name: HashMap<(Option<usize>, String), usize> = HashMap::new();
        let written = lv.instances.iter().filter(|i| i.is_subckt).chain(&lv.module_instances);
        for inst in written {
            let master = inst.model.clone().unwrap_or_default();
            let defined = doc.subckt_of(inst);
            if let Some(d) = defined {
                called[d] = true;
            }
            // the name compares by the rules of the file that writes the instance, as it is looked up
            let key = (defined, dialect_key(doc.rules(&inst.file), &master));
            let at = *by_name.entry(key).or_insert_with(|| {
                calls.push(OutlineCall { master, level: defined, count: 0, instances: Vec::new(), lines: Vec::new() });
                calls.len() - 1
            });
            calls[at].count += 1;
            if defined.is_some() {
                calls[at].instances.push(inst.name.clone());
                calls[at].lines.push(inst.line);
            }
        }
        let (kind, lines) = match lv.kind {
            LevelKind::Subckt => {
                let kind = if doc.is_module(k) { "module" } else { "subckt" };
                (kind, (lv.line > 0).then(|| [lv.line, end_line(&doc, k)]))
            }
            _ => ("file", None),
        };
        let packages = match (lv.kind, lv.records.first()) {
            (LevelKind::File, Some(rec)) => header_packages(&rec.text),
            _ => Vec::new(),
        };
        levels.push(OutlineLevel {
            kind,
            title: lv.title(),
            file,
            lines,
            parent: lv.parent,
            circuit: lv.circuit,
            ports: lv.ports.clone(),
            instances: lv.instances.len() + lv.module_instances.len(),
            calls,
            version: versions[k].clone(),
            packages,
        });
    }

    // the records of each file once, though it may be read more than once, in the order of the file
    let mut records: Vec<Vec<((u32, i64), &str)>> = vec![Vec::new(); files.len()];
    for lv in &doc.levels {
        for rec in &lv.records {
            if let Some(&f) = file_index.get(&rec.file) {
                if !records[f].iter().any(|(at, _)| *at == start(rec)) {
                    records[f].push((start(rec), &rec.text));
                }
            }
        }
    }
    for (f, mut recs) in records.into_iter().enumerate() {
        recs.sort_by_key(|(at, _)| *at);
        let text: String = recs.iter().map(|(_, t)| format!("{t}\n")).collect();
        files[f].records_hash = hash(&text);
        files[f].records = recs.iter().filter(|(_, t)| !is_header(t)).count();
    }

    let roots = (0..doc.levels.len())
        .filter(|&k| doc.levels[k].kind == LevelKind::Subckt && !doc.is_module(k) && !called[k])
        .collect();
    let mut globals: BTreeSet<String> = doc.globals.iter().cloned().collect();
    globals.extend(doc.foreign_globals.iter().map(|(g, _)| g.clone()));
    globals.remove("0");

    let diagnostics =
        sink.shown("").into_iter().map(|d: &Diag| diagnostic(d, &sink, &shown, &file_index)).collect::<Vec<_>>();
    Outline {
        path: entry.to_string(),
        name: file_name(entry).to_string(),
        dialect: doc.dialect.name.clone(),
        files,
        levels,
        roots,
        globals: globals.into_iter().collect(),
        diagnostics,
        missing,
    }
}

/// Read the package files that the records of `doc` name by path, as the reading reads them first
/// (16.2, 16.5): joined to the directory of the file that holds the record. Of each that is the
/// package it names, the packages its header record names by path are read in turn, joined to its
/// own directory. A file `files` does not have is then among the missing files of the outline,
/// before any RNL is interpreted. Only records that write USE-PACKAGE or USE-ALIASES are parsed,
/// and only those of a file whose language version, of `versions`, the reading reads: no RNL of
/// the others is interpreted (16.1).
pub(crate) fn read_named_packages(doc: &HostDocument, versions: &[Option<String>], files: &dyn Provider) {
    let mut named: Vec<(String, PackagePath)> = Vec::new();
    for (k, lv) in doc.levels.iter().enumerate() {
        let read = versions.get(file_level(doc, k)).and_then(Option::as_deref).is_none_or(is_read_version);
        if !read {
            continue;
        }
        let mut dir = None;
        for rec in lv.records.iter().filter(|r| r.text.contains("USE-PACKAGE") || r.text.contains("USE-ALIASES")) {
            let dir = dir.get_or_insert_with(|| file_dir(doc, k));
            named.extend(package_paths(&rec.text).into_iter().map(|p| (dir.clone(), p)));
        }
    }
    let mut seen: HashSet<(String, String, Option<String>)> = HashSet::new();
    let mut at = 0;
    while at < named.len() {
        let (dir, package) = named[at].clone();
        at += 1;
        let target = join(&dir, &package.path);
        if !seen.insert((target.clone(), package.name.clone(), package.version.clone())) {
            continue;
        }
        let Some(text) = files.read(&target) else { continue };
        if !is_package_named(&text, &package) {
            continue;
        }
        // a package file names the packages it refers to in its header record, its first (16.2)
        let records = scan_package_run(&package_lines(&text), &target, &mut Sink::new());
        if let Some(head) = records.first().filter(|r| is_header(&r.text)) {
            let dir = dirname(&target);
            named.extend(package_paths(&head.text).into_iter().map(|p| (dir.clone(), p)));
        }
    }
}

/// The directory of the file a level's records are read in, as the reading joins a path to it: of
/// the file level that holds the level ([`file_level`]).
fn file_dir(doc: &HostDocument, level: usize) -> String {
    dirname(&doc.levels[file_level(doc, level)].path)
}

/// The file level that holds a level, itself when it is one; else the main file's.
fn file_level(doc: &HostDocument, level: usize) -> usize {
    let mut at = Some(level);
    while let Some(k) = at {
        let lv = &doc.levels[k];
        if lv.kind == LevelKind::File {
            return k;
        }
        at = lv.parent;
    }
    0
}

/// The outline of a package file, which is checked on its own and has no netlist: the file alone.
pub(crate) fn package_outline(entry: &str, path: &str, text: &str, format: &str, default: &str) -> Outline {
    let records =
        rnl_runtime::records::scan_package_run(&rnl_runtime::records::package_lines(text), path, &mut Sink::new());
    let recs: String = records.iter().map(|r| format!("{}\n", r.text)).collect();
    Outline {
        path: entry.to_string(),
        name: file_name(entry).to_string(),
        dialect: package::NAME.to_string(),
        files: vec![OutlineFile {
            // the runtime knows a package checked on its own by its file name
            path: file_name(path).to_string(),
            name: file_name(path).to_string(),
            format: Some(format.to_string()),
            default_format: Some(default.to_string()),
            includes: Vec::new(),
            levels: Vec::new(),
            text_hash: hash(text),
            records_hash: hash(&recs),
            records: records.iter().filter(|r| !is_header(&r.text)).count(),
        }],
        levels: Vec::new(),
        roots: Vec::new(),
        globals: Vec::new(),
        diagnostics: Vec::new(),
        missing: Vec::new(),
    }
}

/// The line of the end of a subcircuit, as [`crate::Level::lines`] gives it: its `.ends`, or its
/// file's last line when it has none.
fn end_line(doc: &HostDocument, level: usize) -> u32 {
    let host = &doc.levels[level];
    let last = (doc.source(&host.file).map_or(0, |t| t.lines().count()) as u32).max(host.line);
    if host.end_line >= host.line {
        host.end_line.min(last)
    } else {
        last
    }
}

/// The first 16 hexadecimal digits of the SHA-256 of a text, as the numbering's check is made (5.9).
/// The hash of the records of `file` in `doc`, as [`OutlineFile::records_hash`] gives it.
pub(crate) fn records_hash(doc: &HostDocument, file: &str) -> String {
    let mut recs: Vec<((u32, i64), &str)> = Vec::new();
    for lv in &doc.levels {
        for rec in lv.records.iter().filter(|rec| rec.file == file) {
            if !recs.iter().any(|(at, _)| *at == start(rec)) {
                recs.push((start(rec), &rec.text));
            }
        }
    }
    recs.sort_by_key(|(at, _)| *at);
    hash(&recs.iter().map(|(_, t)| format!("{t}\n")).collect::<String>())
}

/// Where a record starts in its file, its line and column: two records of a file may start on one
/// line (2.4), and a file read more than once has its records once for each reading.
fn start(rec: &Record) -> (u32, i64) {
    (rec.lines.0, rec.segs.first().map_or(0, |seg| seg.col))
}

/// Whether a record is a header record, `type="header"` (2.8), read from its head alone, as the
/// parser reads it ([`parse_record_head`]): a record's body can be long, and the outline does not
/// read it.
fn is_header(text: &str) -> bool {
    text.starts_with("@RNL")
        && parse_record_head(text).and_then(|h| h.get("type").cloned()) == Some(Value::Str("header".into()))
}

fn hash(text: &str) -> String {
    hex(&sha256(text.as_bytes()))[..16].to_string()
}

#[cfg(test)]
mod tests {
    use crate::tests::{corpus, Disk};
    use crate::{build, outline, BuildOptions, Document, MemoryProvider, Outline};

    /// The netlists and the files they include of the conformance corpus, and its package files.
    fn inputs() -> Vec<String> {
        let mut out = Vec::new();
        for dir in ["cases-0.4/from-0.2", "cases-0.4/from-0.3", "equivalence-0.4"] {
            let dir = corpus(dir);
            let mut files: Vec<String> = std::fs::read_dir(&dir)
                .unwrap()
                .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string()))
                .filter(|f| [".sp", ".scs", ".inc", ".rnl"].iter().any(|e| f.ends_with(e)))
                .map(|f| format!("{dir}/{f}"))
                .collect();
            files.sort();
            out.extend(files);
        }
        out
    }

    /// Check that the outline says of each file and level what the Document says of it.
    fn agree(o: &Outline, d: &Document) {
        assert_eq!((&o.path, &o.name, &o.dialect), (&d.path, &d.name, &d.dialect));
        let paths = |files: Vec<&String>| files.into_iter().cloned().collect::<Vec<_>>();
        assert_eq!(paths(o.files.iter().map(|f| &f.path).collect()), paths(d.files.iter().map(|f| &f.path).collect()));
        for (a, b) in o.files.iter().zip(&d.files) {
            assert_eq!((&a.name, &a.format, &a.default_format), (&b.name, &b.format, &b.default_format));
            assert_eq!(a.includes, b.includes, "{}", a.path);
        }
        for lv in &d.levels {
            let at = &o.levels[lv.host];
            assert_eq!((at.title.as_str(), at.ports.as_slice()), (lv.title.as_str(), lv.ports.as_slice()));
            assert_eq!(at.kind == "file", lv.kind == "file", "{}", lv.title);
            assert_eq!(lv.lines.map(|[f, a, b]| (f as usize, [a, b])), at.file.zip(at.lines), "{}", lv.title);
            // each subcircuit the level's instances call that the netlist defines
            for child in lv.children.iter().filter(|c| c.level.is_some()) {
                let host = d.levels[child.level.unwrap()].host;
                let call =
                    at.calls.iter().find(|c| c.level == Some(host)).unwrap_or_else(|| panic!("{}", child.master));
                assert!(call.master.eq_ignore_ascii_case(&child.master), "{} {}", call.master, child.master);
                assert!(call.instances.contains(&child.instance), "{}", child.instance);
            }
            let calls: usize = at.calls.iter().map(|c| c.count).sum();
            assert_eq!(calls, lv.children.len(), "{}", lv.title);
        }
        // the diagnostics of reading the netlist are the Document's
        for x in &o.diagnostics {
            let same = |y: &&crate::Diagnostic| {
                (&y.code, &y.full_path, y.line, y.col, &y.message) == (&x.code, &x.full_path, x.line, x.col, &x.message)
            };
            assert!(d.diagnostics.iter().any(|y| same(&y)), "{x:?}");
        }
        // the outline asks for what the Document asks for: the package files read by path too
        let set = |m: &[String]| m.iter().cloned().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(set(&o.missing), set(&d.missing), "{}", o.path);
    }

    #[test]
    fn the_outline_says_what_the_document_says_of_each_file_and_level() {
        let options = BuildOptions { svg: false, ..BuildOptions::default() };
        for path in inputs() {
            let o = outline(&path, &Disk, &options).unwrap();
            let d = build(&path, &Disk, &options).unwrap();
            agree(&o, &d);
        }
    }

    const TOP: &str = "* top
.include \"lib/cells.inc\"
.include \"lib/none.inc\"
.global vdd
X1 a b inv
X2 b c inv
M1 c a 0 0 nch
* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-spice\", version=\"0.4\")}
.end
";
    const CELLS: &str = "* cells
.subckt inv in out
M1 out in 0 0 nch
M2 out in vdd vdd pch
.ends
.subckt spare p q
X1 p q INV
R1 p q 1k
.ends
.subckt buf in out
X1 in m inv
X2 m out inv
.ends
* @RNL { #M1 @PLACE(0, 0) }
";

    /// Two records on one line are two records (2.4): a header, then a record that draws.
    #[test]
    fn two_records_on_one_line_are_both_counted() {
        let entry = "* entry\n\
            * @RNL(type=\"header\") { @VERSION(\"0.4\") } @RNL { (0, 0) @TEXT(\"ENTRY\") }\n\
            .include \"root.inc\"\n\
            .end\n";
        let root = "* root\n.subckt root a\nR1 a 0 1k\n.ends\n";
        let files = MemoryProvider::new().with("/w/e.sp", entry).with("/w/root.inc", root);
        let o = outline("/w/e.sp", &files, &BuildOptions::default()).unwrap();
        let d = build("/w/e.sp", &files, &BuildOptions::default()).unwrap();
        agree(&o, &d);
        assert_eq!(o.files[0].records, 1);
        // the same records on two lines: the same count
        let apart = entry.replace("} @RNL {", "}\n* @RNL {");
        let files = files.with("/w/e.sp", &apart);
        assert_eq!(outline("/w/e.sp", &files, &BuildOptions::default()).unwrap().files[0].records, 1);
    }

    /// A header record is told by its head as the parser reads it: a `)` in a comment in the head
    /// does not end it.
    #[test]
    fn a_header_is_told_by_its_head_as_the_parser_reads_it() {
        let entry = "* entry\n* @RNL(/* ) */ type=\"header\") { @VERSION(\"0.4\") }\nR1 a 0 1k\n.end\n";
        let files = MemoryProvider::new().with("/w/e.sp", entry);
        let o = outline("/w/e.sp", &files, &BuildOptions::default()).unwrap();
        agree(&o, &build("/w/e.sp", &files, &BuildOptions::default()).unwrap());
        assert_eq!(o.files[0].records, 0);
        // the header of a package file named by path: the packages it names by path are read
        let top = "* top\n\
            * @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"acme\", version=\"1.0\", path=\"pkg/acme.rnl\") }\n\
            R1 a 0 1k\n\
            .end\n";
        let acme = "@RNL(/* ) */ type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"acme\", version=\"1.0\"); \
            @USE-PACKAGE(name=\"base\", version=\"1.0\", path=\"base.rnl\") }\n";
        let files = MemoryProvider::new().with("/w/top.sp", top).with("/w/pkg/acme.rnl", acme);
        let o = outline("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        agree(&o, &build("/w/top.sp", &files, &BuildOptions::default()).unwrap());
        assert_eq!(o.missing, ["/w/pkg/base.rnl"]);
    }

    /// A file of a version the reading does not read names a package file by path: the outline
    /// does not read it, as the reading does not (16.1).
    #[test]
    fn a_file_of_a_version_not_read_names_no_package_file() {
        let top = "* top\n\
            * @RNL(type=\"header\") { @VERSION(\"0.4\") }\n\
            .include \"old.inc\"\n\
            R1 a 0 1k\n\
            .end\n";
        let old = "* old\n\
            * @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"old\", version=\"1.0\", path=\"gone.rnl\") }\n\
            R2 a 0 1k\n";
        let files = MemoryProvider::new().with("/w/top.sp", top).with("/w/old.inc", old);
        let o = outline("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        agree(&o, &build("/w/top.sp", &files, &BuildOptions::default()).unwrap());
        assert_eq!(o.missing, Vec::<String>::new());
    }

    #[test]
    fn an_outline_lists_the_files_levels_and_calls() {
        let files = MemoryProvider::new().with("/w/top.sp", TOP).with("/w/lib/cells.inc", CELLS);
        let o = outline("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        let d = build("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        agree(&o, &d);
        assert_eq!(o.files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["top.sp", "lib/cells.inc"]);
        assert_eq!(o.missing, ["/w/lib/none.inc"]);
        assert!(o.diagnostics.iter().any(|x| x.message.contains("lib/none.inc cannot be found") && x.line == 3));
        let titles: Vec<(&str, &str)> = o.levels.iter().map(|l| (l.kind, l.title.as_str())).collect();
        assert_eq!(
            titles,
            [("file", "top.sp"), ("file", "cells.inc"), ("subckt", "inv"), ("subckt", "spare"), ("subckt", "buf")]
        );
        let top = &o.levels[0];
        assert_eq!((top.instances, top.version.as_deref()), (3, Some("0.4")));
        assert_eq!(top.packages, ["rnl-std-spice"]);
        // the two instances of inv, and a device of no subcircuit of the netlist
        assert_eq!(top.calls.len(), 1);
        assert_eq!((top.calls[0].master.as_str(), top.calls[0].level, top.calls[0].count), ("inv", Some(2), 2));
        assert_eq!(top.calls[0].instances, ["X1", "X2"]);
        assert_eq!(top.calls[0].lines, [5, 6]);
        // an included file without a header record is read in the version of the file that includes it
        assert_eq!((o.levels[1].version.as_deref(), o.levels[1].parent), (Some("0.4"), Some(0)));
        assert_eq!(o.levels[2].ports, ["in", "out"]);
        assert_eq!((o.levels[2].lines, o.levels[2].parent, o.levels[2].file), (Some([2, 5]), Some(1), Some(1)));
        // SPICE names compare without case: INV is inv
        assert_eq!(o.levels[3].calls[0].level, Some(2));
        assert_eq!(o.files[1].levels, [1, 2, 3, 4]);
        // spare and buf call inv, and nothing calls them
        assert_eq!(o.roots, [3, 4]);
        assert_eq!(o.globals, ["vdd"]);
    }

    #[test]
    fn a_module_of_verilog_a_calls_the_modules_its_instances_name() {
        let top = "// top\n// @RNL(type=\"header\") { @VERSION(\"0.4\")}\nsimulator lang=spectre\nahdl_include \"a.va\"\nahdl_include \"b.va\"\nI1 (in out 0) top_va\n";
        let a = "module top_va(x, y, g);\n  inout x, y, g;\n  electrical x, y, g, mid;\n  gain #(.k(2)) u1 (x, mid);\n  amp u2 (mid, y);\n  ghost u3 (y, g);\n  gain u4 (.a(y), .b(g));\n  analog V(y) <+ 0;\nendmodule\nmodule gain(a, b);\n  inout a, b;\n  electrical a, b;\nendmodule\n";
        let b = "module amp(a, b);\n  inout a, b;\n  electrical a, b;\nendmodule\n";
        let files = MemoryProvider::new().with("/w/top.scs", top).with("/w/a.va", a).with("/w/b.va", b);
        let o = outline("/w/top.scs", &files, &BuildOptions::default()).unwrap();
        let at = |title: &str| o.levels.iter().position(|l| l.title == title).unwrap();
        // each call: the name, the level that defines it, how many, the instances and their lines
        type Call<'a> = (&'a str, Option<usize>, usize, Vec<&'a str>, Vec<u32>);
        let calls = |k: usize| -> Vec<Call> {
            o.levels[k]
                .calls
                .iter()
                .map(|c| {
                    let instances = c.instances.iter().map(String::as_str).collect();
                    (c.master.as_str(), c.level, c.count, instances, c.lines.clone())
                })
                .collect()
        };
        // the netlist's instance of the module
        assert_eq!(calls(0), [("top_va", Some(at("top_va")), 1, vec!["I1"], vec![6])]);
        // a module of the same file, one of another file, and a name no module has
        let top_va = &o.levels[at("top_va")];
        assert_eq!((top_va.kind, top_va.circuit, top_va.instances), ("module", false, 4));
        assert_eq!(
            calls(at("top_va")),
            [
                ("gain", Some(at("gain")), 2, vec!["u1", "u4"], vec![4, 7]),
                ("amp", Some(at("amp")), 1, vec!["u2"], vec![5]),
                ("ghost", None, 1, vec![], vec![]),
            ]
        );
        assert_eq!(o.levels[at("amp")].file, Some(2));
        assert!(o.levels[at("gain")].calls.is_empty() && o.levels[at("amp")].instances == 0);
        // a module is no root, though nothing in the netlist calls gain
        assert!(o.roots.is_empty());
        // and nothing of it is drawn: the Document has the netlist's level alone
        let d = build("/w/top.scs", &files, &BuildOptions::default()).unwrap();
        assert_eq!(d.levels.len(), 1);
        let json = serde_json::to_value(&o.levels[at("top_va")].calls[2]).unwrap();
        assert!(json.get("instances").is_none() && json.get("lines").is_none(), "{json}");
    }

    #[test]
    fn the_hash_of_the_records_is_that_of_the_records_alone() {
        let files = MemoryProvider::new().with("/w/top.sp", TOP).with("/w/lib/cells.inc", CELLS);
        let o = outline("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        let netlist = CELLS.replace("R1 p q 1k", "R1 p q 2k");
        let changed = files.clone().with("/w/lib/cells.inc", &netlist);
        let n = outline("/w/top.sp", &changed, &BuildOptions::default()).unwrap();
        assert_ne!(o.files[1].text_hash, n.files[1].text_hash);
        assert_eq!(o.files[1].records_hash, n.files[1].records_hash);
        let records = CELLS.replace("@PLACE(0, 0)", "@PLACE(1, 0)");
        let r = outline("/w/top.sp", &files.with("/w/lib/cells.inc", &records), &BuildOptions::default()).unwrap();
        assert_ne!(o.files[1].records_hash, r.files[1].records_hash);
        assert_eq!(o.files[0], r.files[0]);
    }

    #[test]
    fn a_file_counts_its_records_besides_its_header() {
        // an entry of includes alone, whose RNL is its header record
        let top = "* top\n* @RNL(type=\"header\", author=\"a (b) \"\"c)\") { @VERSION(\"0.4\")}\n.include \"cells.inc\"\n.include \"bare.inc\"\n.end\n";
        // a header, a record of the file and one of a subcircuit, written over two lines
        let cells = "* cells\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\n* @RNL { #X1 @PLACE(0, 0) }\n.subckt inv in out\nM1 out in 0 0 nch\n* @RNL(author=\"x\") { #M1 @PLACE(0, 0);\n*   #M1 @ORIENT(\"R0\") }\n.ends\nX1 a b inv\n";
        let bare = "* no RNL\nR1 a b 1k\n";
        let files = MemoryProvider::new().with("/w/top.sp", top).with("/w/cells.inc", cells).with("/w/bare.inc", bare);
        let o = outline("/w/top.sp", &files, &BuildOptions::default()).unwrap();
        let records = |name: &str| o.files.iter().find(|f| f.name == name).unwrap().records;
        assert_eq!((records("top.sp"), records("cells.inc"), records("bare.inc")), (0, 2, 0));
        // a record of the solver counts as any other
        let solved = top.replace(".end", "* @RNL(type=\"generated\") { #X9 @PLACE(0, 0) }\n.end");
        let s = outline("/w/top.sp", &files.with("/w/top.sp", &solved), &BuildOptions::default()).unwrap();
        assert_eq!(s.files[0].records, 1);
        assert_eq!(serde_json::to_value(&o.files[0]).unwrap()["records"], 0);
    }

    #[test]
    fn the_package_files_named_by_path_are_missing_until_given() {
        // the entry names a package by path, and so does a file it includes from below it
        let top = "* top\n.include \"sub/cells.inc\"\nX1 a b inv\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"acme\", path=\"pkg/acme.rnl\")}\n.end\n";
        // a path relative to the including file's directory; one with a body or a malformed version
        // is not read
        let cells = "* cells\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"deep\", version=\"1.0\", path=\"../lib/deep.rnl\"); @USE-PACKAGE(name=\"inline\", path=\"no/inline.rnl\") {}; @USE-PACKAGE(name=\"bad\", version=\"1\", path=\"no/bad.rnl\")}\n.subckt inv in out\nR1 in out 1k\n.ends\n";
        let package = |name: &str, header: &str| {
            format!("@RNL(type=\"header\") {{ @VERSION(\"0.4\"); @PACKAGE(name=\"{name}\", version=\"1.0\"){header}}}\n@RNL {{ $MARK_{name} = @DEFINE(roles=[\"p\"]) {{}} }}\n")
        };
        let files = MemoryProvider::new().with("/w/top.sp", top).with("/w/sub/cells.inc", cells);
        let read = |files: &MemoryProvider| {
            let o = outline("/w/top.sp", files, &BuildOptions::default()).unwrap();
            let d = build("/w/top.sp", files, &BuildOptions::default()).unwrap();
            agree(&o, &d);
            o.missing
        };
        assert_eq!(read(&files), ["/w/pkg/acme.rnl", "/w/lib/deep.rnl"]);

        // a package given names one more by path, from its own directory
        let acme = package("acme", "; @USE-PACKAGE(name=\"base\", path=\"base/base.rnl\")");
        let files = files.with("/w/pkg/acme.rnl", &acme);
        assert_eq!(read(&files), ["/w/lib/deep.rnl", "/w/pkg/base/base.rnl"]);
        // a file of another package is not that package: what it names is not read
        let other =
            files.clone().with("/w/lib/deep.rnl", &package("other", "; @USE-PACKAGE(name=\"x\", path=\"x.rnl\")"));
        assert_eq!(read(&other), ["/w/pkg/base/base.rnl"]);

        // with all given, nothing is missing
        let files =
            files.with("/w/lib/deep.rnl", &package("deep", "")).with("/w/pkg/base/base.rnl", &package("base", ""));
        assert!(read(&files).is_empty());
    }

    #[test]
    fn the_outline_of_a_package_file_is_the_file_alone() {
        let package = "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"acme\", version=\"1.0\")}\n@RNL {}\n";
        let files = MemoryProvider::new().with("/p/acme.rnl", package);
        let o = outline("/p/acme.rnl", &files, &BuildOptions::default()).unwrap();
        assert_eq!((o.dialect.as_str(), o.files.len(), o.levels.len()), ("rnl", 1, 0));
        assert_eq!(o.files[0].records, 1);
        assert!(outline("/p/none.sp", &files, &BuildOptions::default()).is_err());
    }
}
