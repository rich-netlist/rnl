//! Writing RNL into host netlists: edit transactions (the editor interface draft, section 4).
//!
//! This is the first part: [`write`] puts a transaction's ops into a netlist that has no RNL yet,
//! as a tool does that prints a netlist and its drawing together, such as Analog Canvas's
//! export. It writes:
//!
//! - the header record where a new header goes (`writeback::add_header`; 2.2, 2.8, 16.3): on line
//!   2, after the title, which is never read, and after the lines that continue it; in a Spectre
//!   netlist saved under a name that is not a `.scs` file, whose first statement is `simulator
//!   lang=spectre`, after that statement with `//`, and otherwise above it with `*`. It holds
//!   VERSION, then the packages of the `use-package` ops, each name and version once;
//! - one record of the author's for each level the ops name, at the end of the level's own text
//!   (17): before the `.ends` of a subcircuit, or the `.end` of the file, or at the end of the file;
//!   before a region written to be encrypted (`.prot … .unprot`, `protect … unprotect`) that the
//!   end lies in, where a record is gone once the region is encrypted (2.2), and a level whose own
//!   text lies in one is refused;
//! - in the comment carrier of the header, `*` or `//`, and `*` in a `simulator lang=spice` section
//!   of a Spectre netlist, where `//` is no comment (2.2);
//! - the statements in the standard form (4.3), in the order of the ops, each line at most 132
//!   characters long, broken only at spaces outside strings and brackets.
//!
//! Merging with records already in the file, `replace_own`, `base` and the refusal of ops that do
//! not take effect are set aside, as Analog Canvas, which writes with this crate, reads RNL into its
//! own format and writes it anew: a netlist that has RNL records is refused. The one thing it does
//! with a netlist that has records is [`replace::replace_records`]: the whole new drawing of one
//! level put in place of the records of that level, in the file that holds it, with no merging and
//! no other line changed.

mod error;
pub use error::{Refusal, Written};
pub mod form;
pub mod ops;
pub mod replace;
pub mod upgrade;

use std::collections::BTreeMap;

use rnl_runtime::diag::{codes, Severity, Sink};
use rnl_runtime::host::file_name;
use rnl_runtime::host::{HostDocument, LevelId, LevelKind};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::writeback::{add_header, append_record, carrier_at, record_line};
use serde::Deserialize;
use serde_json::Value;

pub use ops::{LevelOp, Op};

/// The language version the header record declares: the toolkit's default (16.1).
pub const VERSION: &str = "0.3";

/// The longest line written, comment marker included (4.3).
pub const LINE: usize = 132;

/// What to write: a netlist, its dialect, and the ops of one author.
#[derive(Deserialize, Debug, Clone)]
pub struct WriteRequest {
    /// Explicit language opt-in. None retains the released writer default.
    #[serde(default)]
    pub language_version: Option<String>,
    /// "spice" or "spectre".
    pub dialect: String,
    /// The netlist, without RNL records.
    pub netlist: String,
    /// The `author` of the records written, such as "analog-canvas".
    pub author: String,
    /// The ops, as the editor interface draft writes them (4.1); read one at a time, so that a
    /// refusal can say which.
    #[serde(default)]
    pub ops: Vec<Value>,
    /// The name the netlist is saved under, such as `top.net`, when it is known; only its last
    /// part counts. Its suffix tells the Spectre adapter the mode the file starts in (2.2): a
    /// `.scs` file starts in the Spectre mode, any other file in the SPICE mode up to its
    /// `simulator lang=spectre`, so the header and the records are written where that reading
    /// keeps them. Without it, a Spectre netlist is read as a `.scs` file and a SPICE netlist as a
    /// `.sp` file ([`entry`]).
    #[serde(default)]
    pub file: Option<String>,
}

/// The path the netlist of a request is read under: in `/rnl-edit/`, by the name of its `file`,
/// or [`entry`] when it names none.
pub fn entry_of(request: &WriteRequest) -> String {
    match request.file.as_deref().map(|f| file_name(&f.replace('\\', "/")).to_string()) {
        Some(name) if !name.is_empty() => format!("/rnl-edit/{name}"),
        _ => entry(&request.dialect).to_string(),
    }
}

/// The path a netlist is read under when its request names no file: a `.scs` file for Spectre,
/// a `.sp` file for SPICE.
pub fn entry(dialect: &str) -> &'static str {
    if dialect == "spectre" {
        "/rnl-edit/netlist.scs"
    } else {
        "/rnl-edit/netlist.sp"
    }
}

/// Write the ops into the netlist; the netlist's own text stays as it is, line endings included.
/// An op that cannot be written refuses the whole request, with the reason.
pub fn write(request: &WriteRequest) -> Result<String, String> {
    write_diagnosed(request).map(|w| w.text).map_err(|e| e.message)
}

/// Write with stable diagnostic kinds and the host reader's diagnostics. Draft 0.4 refuses
/// host errors before writing; released output versions retain their existing behavior.
pub fn write_diagnosed(request: &WriteRequest) -> Result<Written, Refusal> {
    let written = write_in(request, None)?;
    finish_write(request, written, &MemoryProvider::new(), &rnl_runtime::Options::default())
}

fn write_in(request: &WriteRequest, files: Option<&dyn rnl_runtime::source::Provider>) -> Result<Written, Refusal> {
    let version = request.language_version.as_deref().unwrap_or(VERSION);
    if !["0.3", "0.4"].contains(&version) {
        return Err(Refusal::new(
            codes::EDIT_FIELD_INVALID,
            format!("unsupported output language version `{version}`"),
        ));
    }
    let dialect = request.dialect.as_str();
    // the formats of the files that are not netlists, a Verilog-A model or a package file (2.7): this
    // writer does not write into them
    let (text, package) = (rnl_runtime::host::text::NAME, rnl_runtime::host::package::NAME);
    if dialect == text {
        return Err(Refusal::new(
            codes::EDIT_DIALECT_TEXT,
            "dialect \"text\" is that of a file that is not a netlist; RNL is written only into a netlist",
        ));
    }
    if dialect == package {
        return Err(Refusal::new(
            codes::EDIT_DIALECT_PACKAGE,
            "dialect \"rnl\" is that of a package file (2.7), which is not a netlist; RNL is written \
                    here only into a netlist",
        ));
    }
    let formats: Vec<_> = rnl_view::formats().into_iter().filter(|f| f.name != text && f.name != package).collect();
    if !formats.iter().any(|f| f.name == dialect) {
        let names: Vec<&str> = formats.iter().map(|f| f.name.as_str()).collect();
        return Err(Refusal::new(
            codes::EDIT_DIALECT_UNKNOWN,
            format!("dialect \"{dialect}\" is not {}", names.join(" or ")),
        ));
    }
    let ops: Vec<LevelOp> = request
        .ops
        .iter()
        .enumerate()
        .map(|(k, v)| ops::parse_for_language(k, v, request.language_version.as_deref() == Some("0.4")))
        .collect::<Result<_, _>>()?;

    for op in &ops {
        match &op.op {
            Op::AddText { text, .. } => {
                form::check_math_content(text, version).map_err(|e| Refusal::new(codes::EDIT_FIELD_INVALID, e))?
            }
            Op::DefineSymbol { pins, .. } => {
                for pin in pins {
                    if let Some(label) = &pin.label {
                        form::check_math_content(label, version)
                            .map_err(|e| Refusal::new(codes::EDIT_PIN_INVALID, e))?;
                    }
                }
            }
            _ => {}
        }
    }

    if version != "0.4" {
        for op in &ops {
            if matches!(&op.op, Op::AddText { fix_rotate: Some(_), .. }) {
                return Err(Refusal::new(codes::EDIT_FIELD_INVALID, "fix_rotate requires language_version 0.4"));
            }
            if let Op::DefineSymbol { pins, .. } = &op.op {
                if pins.iter().any(|p| p.terminals.is_empty() || !p.names.is_empty() || p.bus && p.terminals.len() == 1)
                {
                    return Err(Refusal::new(
                        codes::EDIT_PIN_INVALID,
                        "independent pin names, unbound pins and one-terminal bus pins require language_version 0.4",
                    ));
                }
            }
        }
    }

    let path = entry_of(request);
    let mut sink = Sink::new();
    let mut read = |text: &str| {
        let memory = MemoryProvider::new().with(&path, text);
        let mut layers: Vec<&dyn rnl_runtime::source::Provider> = vec![&memory];
        // Modern writes read includes from the caller's workspace too.
        if version == "0.4" {
            layers.extend(files);
        }
        let source = rnl_runtime::source::Layered { layers };
        // Each modern read reports the source it just checked, without stale locations
        // from the input before the header or the requested records were inserted.
        if version == "0.4" {
            sink = Sink::new();
        }
        let doc = rnl_view::read_host(&path, Some(dialect), &source, &mut sink);
        let doc = match doc {
            Ok(doc) if version == "0.4" => {
                let (doc, checked) =
                    rnl_runtime::interp::resolve_host_diagnostics(doc, version, std::mem::take(&mut sink));
                sink = checked;
                Ok(doc)
            }
            other => other,
        };
        if version == "0.4" {
            refuse_host_errors(&sink)?;
        }
        doc.map_err(|message| Refusal {
            code: codes::VIEW_CANNOT_READ,
            message,
            diagnostics: error::diagnostics(&sink),
        })
    };
    let doc = read(&request.netlist)?;
    // the netlist as the reader names it in its levels and records
    let file = doc.main().file.clone();
    if doc.levels.iter().any(|lv| lv.records.iter().any(|r| r.file == file)) {
        return Err(Refusal::new(
            codes::EDIT_NETLIST_HAS_RECORDS,
            "the netlist has RNL records already; this writer writes only into a netlist without them",
        ));
    }

    // the header's packages, and each level's statements in the order of the ops
    let mut packages: Vec<String> = Vec::new();
    let mut levels: BTreeMap<LevelId, Vec<String>> = BTreeMap::new();
    let mut first_op: BTreeMap<LevelId, usize> = BTreeMap::new();
    // the styles of `set` and `add-usage`: one definition in each level for the styles of the same
    // properties and values, first in its record, numbered over the whole request, so that a level
    // hides no style of a level above it (6.8) and each level reads the same on its own
    let mut styles: BTreeMap<LevelId, Vec<(Vec<String>, String, String)>> = BTreeMap::new();
    let mut next_style = 1;
    for (k, lop) in ops.iter().enumerate() {
        if let Op::UsePackage { name, version, path } = &lop.op {
            let p = form::use_package(name, version.as_deref(), path.as_deref());
            if !packages.contains(&p) {
                packages.push(p);
            }
            continue;
        }
        let id = lop.level.as_deref().unwrap_or_default();
        let level = find_level(&doc, &file, id).map_err(|e| e.context(format!("op {k} ({})", lop.op.name())))?;
        let style = match &lop.op {
            Op::Set { style, .. } | Op::AddUsage { style, .. } if !style.is_empty() => {
                let defined = styles.entry(level).or_default();
                let mut key: Vec<String> = style.iter().map(|(p, v)| format!("{p}={v:?}")).collect();
                key.sort();
                Some(match defined.iter().find(|(k, ..)| *k == key) {
                    Some((_, name, _)) => name.clone(),
                    None => {
                        let name = format!("$style{next_style}");
                        next_style += 1;
                        defined.push((key, name.clone(), form::define_style(&name, style)));
                        name
                    }
                })
            }
            _ => None,
        };
        levels.entry(level).or_default().extend(if version == "0.4" {
            form::statements_for_language(&lop.op, style.as_deref(), None, true)
        } else {
            form::statements(&lop.op, style.as_deref())
        });
        first_op.entry(level).or_insert(k);
    }
    // a record goes at the end of a level, or before a region written to be encrypted that the end
    // lies in, where it would be gone once the region is encrypted (2.2); a level whose own text
    // lies in such a region has no place for it
    for (&level, &k) in &first_op {
        record_line(&doc, level, &request.netlist).map_err(|(a, b)| {
            let (what, title) = (ops[k].op.name(), doc.levels[level].title());
            let file_level = doc.file_level(level);
            let selected = &doc.levels[file_level];
            if selected.section.is_some() && selected.section_lines.is_none() {
                Refusal::new(codes::EDIT_SECTION_NOT_FOUND, format!("op {k} ({what}): the selected library section {} was not found (2.2)", selected.section.as_deref().unwrap()))
            } else if doc.excluded.iter().any(|(f, x, y)| f == &file && (*x, *y) == (a, b))
                || doc.unselected_sections.iter().any(|(l, x, y)| *l == file_level && (*x, *y) == (a, b)) {

                Refusal::new(codes::EDIT_REGION_EXCLUDED, format!("op {k} ({what}): {title} lies in a host region excluded from writing, lines {a} to {b} (2.2)"))
            } else {
                Refusal::new(codes::EDIT_REGION_ENCRYPTED, format!(
                    "op {k} ({what}): {title} lies in a region written to be encrypted, lines {a} to {b}, where a record \
                     is gone once the region is encrypted (2.2)"
                ))
            }
        })?;
    }

    // the header first, where a new header goes; then the netlist is read again, so that the
    // records follow its carrier (2.2), at the places of the text that has it
    let mut text = add_header(&doc, &file, &request.netlist, |marker| header(&packages, marker, version));
    let doc = read(&text)?;
    let mut at: BTreeMap<LevelId, u32> = BTreeMap::new();
    for &level in levels.keys() {
        let line = record_line(&doc, level, &text)
            .map_err(|_| Refusal::new(codes::EDIT_REREAD_DIFFERS, "the netlist reads otherwise with its header"))?;
        at.insert(level, line);
    }
    // from the bottom up, so that the places of the levels above stay where the reader found them
    let mut order: Vec<LevelId> = levels.keys().copied().collect();
    order.sort_by_key(|lv| std::cmp::Reverse(at[lv]));
    for lv in order {
        let mut statements: Vec<String> = styles.get(&lv).into_iter().flatten().map(|(.., d)| d.clone()).collect();
        statements.extend(levels[&lv].iter().cloned());
        let record = record(&request.author, &statements, carrier_at(&doc, &file, &text, at[&lv]));
        text = append_record(&doc, lv, &text, &record);
    }
    Ok(Written { text, diagnostics: error::diagnostics(&sink) })
}

/// Write a new drawing with the tool's package settings. In draft 0.4, persist the exact
/// standard-package version selected by `include_std`, so the result needs no tool setting.
/// Existing explicit package ops take precedence. Released writers keep their output.
pub fn write_with_options(
    request: &WriteRequest,
    files: &dyn rnl_runtime::source::Provider,
    options: &rnl_runtime::Options,
) -> Result<String, String> {
    write_with_options_diagnosed(request, files, options).map(|w| w.text).map_err(|e| e.message)
}

/// The package-aware writer, retaining numbered refusals and host diagnostics.
pub fn write_with_options_diagnosed(
    request: &WriteRequest,
    files: &dyn rnl_runtime::source::Provider,
    options: &rnl_runtime::Options,
) -> Result<Written, Refusal> {
    let mut written = write_in(request, Some(files))?;
    let text = &written.text;
    if !options.include_std || request.language_version.as_deref() != Some("0.4") {
        return finish_write(request, written, files, options);
    }
    let entry = entry_of(request);
    let memory = MemoryProvider::new().with(&entry, text);
    let both = rnl_runtime::source::Layered { layers: vec![&memory, files] };
    let read = rnl_view::interpret_with(&entry, Some(&request.dialect), &BTreeMap::new(), &both, options)
        .map_err(|e| Refusal::new(codes::VIEW_CANNOT_READ, e))?;
    let Some(version) = read.included_standard else { return finish_write(request, written, files, options) };
    let saved = rnl_runtime::writeback::with_standard_package(&read.doc, text, &version);
    if saved == *text {
        return Err(Refusal::new(
            codes::EDIT_REREAD_DIFFERS,
            "cannot write the standard-package reference into the main header (16.3)",
        ));
    }
    written.text = saved;
    finish_write(request, written, files, options)
}

fn refuse_host_errors(sink: &Sink) -> Result<(), Refusal> {
    if let Some(d) = sink.all().iter().find(|d| d.severity == Severity::Error) {
        return Err(Refusal {
            code: d.code,
            message: d.loc.as_ref().map(|loc| format!("{loc}: {}", d.message)).unwrap_or_else(|| d.message.clone()),
            diagnostics: error::diagnostics(sink),
        });
    }
    Ok(())
}

/// Keep host warning locations in the final text, including package and junction insertions.
fn finish_write(
    request: &WriteRequest,
    written: Written,
    files: &dyn rnl_runtime::source::Provider,
    options: &rnl_runtime::Options,
) -> Result<Written, Refusal> {
    let host_count = written.diagnostics.len();
    let mut written = add_junctions(request, written, files, options)?;
    if request.language_version.as_deref() != Some("0.4") {
        return Ok(written);
    }
    let extra = written.diagnostics.split_off(host_count);
    let path = entry_of(request);
    let memory = MemoryProvider::new().with(&path, &written.text);
    let source = rnl_runtime::source::Layered { layers: vec![&memory, files] };
    let mut sink = Sink::new();
    let doc = rnl_view::read_host(&path, Some(&request.dialect), &source, &mut sink).map_err(|message| Refusal {
        code: codes::VIEW_CANNOT_READ,
        message,
        diagnostics: error::diagnostics(&sink),
    })?;
    let (_, sink) = rnl_runtime::interp::resolve_host_diagnostics(doc, "0.4", sink);
    refuse_host_errors(&sink)?;
    written.diagnostics = error::diagnostics(&sink);
    written.diagnostics.extend(extra);
    Ok(written)
}

/// Resolve only after package ops and include_std have been written. These usages are in a
/// final record, so they see exactly the target-level names that a solver record would see.
fn add_junctions(
    request: &WriteRequest,
    mut written: Written,
    files: &dyn rnl_runtime::source::Provider,
    options: &rnl_runtime::Options,
) -> Result<Written, Refusal> {
    if request.language_version.as_deref() != Some("0.4") {
        return Ok(written);
    }
    let ops: Vec<_> = request
        .ops
        .iter()
        .enumerate()
        .map(|(k, v)| ops::parse_for_language(k, v, request.language_version.as_deref() == Some("0.4")))
        .collect::<Result<_, _>>()?;
    if !ops.iter().any(|op| matches!(op.op, Op::AddPoint { junction: true, .. })) {
        return Ok(written);
    }
    let entry = entry_of(request);
    let memory = MemoryProvider::new().with(&entry, &written.text);
    let both = rnl_runtime::source::Layered { layers: vec![&memory, files] };
    let options = rnl_runtime::Options { problems: true, generated: true, ..options.clone() };
    let read = rnl_view::interpret_with(&entry, Some(&request.dialect), &BTreeMap::new(), &both, &options)
        .map_err(|e| Refusal::new(codes::VIEW_CANNOT_READ, e))?;
    let doc = &read.doc;
    let file = &doc.main().file;
    let mut levels: BTreeMap<LevelId, Vec<String>> = BTreeMap::new();
    let mut warnings = Sink::new();
    for op in ops {
        let Op::AddPoint { name, junction: true, .. } = op.op else { continue };
        let level = find_level(doc, file, op.level.as_deref().unwrap_or_default())?;
        let Some(problem) = read.problems.iter().find(|p| p.level == level) else { continue };
        if let Some(d) = problem.junction.warning() {
            warnings.add(
                d.severity,
                d.code,
                d.message,
                d.loc,
                rnl_runtime::diag::Extra { cause: None, origin: d.origin, in_package: d.in_package },
            );
        }
        if let Some(definition) = &problem.junction.definition {
            levels.entry(level).or_default().push(format!("{definition} @ATTACH {name}"));
        }
    }
    written.diagnostics.extend(error::diagnostics(&warnings));
    let mut order = Vec::new();
    for &level in levels.keys() {
        let line = record_line(doc, level, &written.text)
            .map_err(|_| Refusal::new(codes::EDIT_REREAD_DIFFERS, "the netlist reads otherwise with its header"))?;
        order.push((line, level));
    }
    order.sort_by_key(|(line, _)| std::cmp::Reverse(*line));
    for (line, level) in order {
        let record = record(&request.author, &levels[&level], carrier_at(doc, file, &written.text, line));
        written.text = append_record(doc, level, &written.text, &record);
    }
    Ok(written)
}

/// The level an op names: `subckt:NAME`, a subcircuit of the netlist, by the dialect's case rule;
/// `file` or `file:…`, the netlist's file level.
fn find_level(doc: &HostDocument, file: &str, id: &str) -> Result<LevelId, Refusal> {
    if id == "file" || id.starts_with("file:") {
        return Ok(0);
    }
    let name = id.strip_prefix("subckt:").ok_or_else(|| {
        Refusal::new(codes::EDIT_LEVEL_ID_INVALID, format!("level \"{id}\" is not subckt:NAME or file:PATH"))
    })?;
    doc.levels
        .iter()
        .position(|lv| {
            lv.kind == LevelKind::Subckt
                && lv.file == file
                && if lv.case_sensitive { lv.name == name } else { lv.name.eq_ignore_ascii_case(name) }
        })
        .ok_or_else(|| Refusal::new(codes::EDIT_LEVEL_NO_SUBCKT, format!("the netlist has no subcircuit {name}")))
}

/// A record of the author's: one statement after another, separated by `;`, each line within
/// [`LINE`] when carried.
fn record(author: &str, statements: &[String], marker: &str) -> String {
    // "* " carries the record, "  " indents a statement
    let width = LINE.saturating_sub(marker.len() + 3);
    let mut out = vec![format!("@RNL(author={}) {{", form::quote(author))];
    for (k, s) in statements.iter().enumerate() {
        let s = if k + 1 < statements.len() { format!("{s};") } else { s.clone() };
        // a statement of several lines, such as a symbol's definition, keeps its lines' indents
        for line in s.lines() {
            let body = line.trim_start();
            let indent = line.len() - body.len();
            let pad = " ".repeat(2 + indent);
            out.extend(form::wrap(body, width.saturating_sub(indent), 4).into_iter().map(|l| format!("{pad}{l}")));
        }
    }
    out.push("}".into());
    out.join("\n")
}

/// The header record: on one line when it fits, else one statement a line.
fn header(packages: &[String], marker: &str, version: &str) -> String {
    let statements: Vec<String> =
        std::iter::once(format!("@VERSION({})", form::quote(version))).chain(packages.iter().cloned()).collect();
    let one = format!("@RNL(type=\"header\") {{ {}}}", statements.join("; "));
    if marker.len() + 1 + one.chars().count() <= LINE {
        return one;
    }
    let mut out = vec!["@RNL(type=\"header\") {".to_string()];
    for (k, s) in statements.iter().enumerate() {
        out.push(format!("  {s}{}", if k + 1 < statements.len() { ";" } else { "" }));
    }
    out.push("}".into());
    out.join("\n")
}

#[cfg(test)]
mod tests;
