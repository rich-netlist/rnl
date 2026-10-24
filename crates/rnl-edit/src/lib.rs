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
//! own format and writes it anew: a netlist that has RNL records is refused.

pub mod form;
pub mod ops;

use std::collections::BTreeMap;

use rnl_runtime::diag::Sink;
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
    let dialect = request.dialect.as_str();
    let formats = rnl_view::formats();
    if !formats.iter().any(|f| f.name == dialect) {
        let names: Vec<&str> = formats.iter().map(|f| f.name.as_str()).collect();
        return Err(format!("dialect \"{dialect}\" is not {}", names.join(" or ")));
    }
    let ops: Vec<LevelOp> = request.ops.iter().enumerate().map(|(k, v)| ops::parse(k, v)).collect::<Result<_, _>>()?;

    let path = entry_of(request);
    let read = |text: &str| {
        let files = MemoryProvider::new().with(&path, text);
        rnl_view::read_host(&path, Some(dialect), &files, &mut Sink::new())
    };
    let doc = read(&request.netlist)?;
    // the netlist as the reader names it in its levels and records
    let file = doc.main().file.clone();
    if doc.levels.iter().any(|lv| lv.records.iter().any(|r| r.file == file)) {
        return Err("the netlist has RNL records already; this writer writes only into a netlist without them".into());
    }

    // the header's packages, and each level's statements in the order of the ops
    let mut packages: Vec<String> = Vec::new();
    let mut levels: BTreeMap<LevelId, Vec<String>> = BTreeMap::new();
    let mut first_op: BTreeMap<LevelId, usize> = BTreeMap::new();
    for (k, lop) in ops.iter().enumerate() {
        if let Op::UsePackage { name, version, path } = &lop.op {
            let p = form::use_package(name, version.as_deref(), path.as_deref());
            if !packages.contains(&p) {
                packages.push(p);
            }
            continue;
        }
        let id = lop.level.as_deref().unwrap_or_default();
        let level = find_level(&doc, &file, id).map_err(|why| format!("op {k} ({}): {why}", lop.op.name()))?;
        levels.entry(level).or_default().extend(form::statements(&lop.op));
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
                format!("op {k} ({what}): the selected library section {} was not found (2.2)", selected.section.as_deref().unwrap())
            } else if doc.excluded.iter().any(|(f, x, y)| f == &file && (*x, *y) == (a, b))
                || doc.unselected_sections.iter().any(|(l, x, y)| *l == file_level && (*x, *y) == (a, b)) {

                format!("op {k} ({what}): {title} lies in a host region excluded from writing, lines {a} to {b} (2.2)")
            } else {
                format!(
                    "op {k} ({what}): {title} lies in a region written to be encrypted, lines {a} to {b}, where a record \
                     is gone once the region is encrypted (2.2)"
                )
            }
        })?;
    }

    // the header first, where a new header goes; then the netlist is read again, so that the
    // records follow its carrier (2.2), at the places of the text that has it
    let mut text = add_header(&doc, &file, &request.netlist, |marker| header(&packages, marker));
    let doc = read(&text)?;
    let mut at: BTreeMap<LevelId, u32> = BTreeMap::new();
    for &level in levels.keys() {
        let line = record_line(&doc, level, &text).map_err(|_| "the netlist reads otherwise with its header")?;
        at.insert(level, line);
    }
    // from the bottom up, so that the places of the levels above stay where the reader found them
    let mut order: Vec<LevelId> = levels.keys().copied().collect();
    order.sort_by_key(|lv| std::cmp::Reverse(at[lv]));
    for lv in order {
        let record = record(&request.author, &levels[&lv], carrier_at(&doc, &file, &text, at[&lv]));
        text = append_record(&doc, lv, &text, &record);
    }
    Ok(text)
}

/// The level an op names: `subckt:NAME`, a subcircuit of the netlist, by the dialect's case rule;
/// `file` or `file:…`, the netlist's file level.
fn find_level(doc: &HostDocument, file: &str, id: &str) -> Result<LevelId, String> {
    if id == "file" || id.starts_with("file:") {
        return Ok(0);
    }
    let name = id.strip_prefix("subckt:").ok_or_else(|| format!("level \"{id}\" is not subckt:NAME or file:PATH"))?;
    doc.levels
        .iter()
        .position(|lv| {
            lv.kind == LevelKind::Subckt
                && lv.file == file
                && if lv.case_sensitive { lv.name == name } else { lv.name.eq_ignore_ascii_case(name) }
        })
        .ok_or_else(|| format!("the netlist has no subcircuit {name}"))
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
fn header(packages: &[String], marker: &str) -> String {
    let statements: Vec<String> =
        std::iter::once(format!("@VERSION({})", form::quote(VERSION))).chain(packages.iter().cloned()).collect();
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
