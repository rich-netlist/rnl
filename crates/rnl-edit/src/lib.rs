//! Writing RNL into host netlists: edit transactions (the editor interface draft, section 4).
//!
//! This is the first part: [`write`] puts a transaction's ops into a netlist that has no RNL yet,
//! as a tool does that prints a netlist and its drawing together, such as Analog Canvas's
//! export. It writes:
//!
//! - the header record after line 1, which is never read (2.2, 2.8): VERSION, then the packages
//!   of the `use-package` ops, each name and version once;
//! - one record of the author's for each level the ops name, at the end of the level's own text
//!   (17): before the `.ends` of a subcircuit, or the `.end` of the file, or at the end of the file;
//! - in the comment carrier of the dialect, `*` or `//`;
//! - the statements in the standard form (4.3), in the order of the ops, each line at most 132
//!   characters long, broken only at spaces outside strings and brackets.
//!
//! Merging with records already in the file, `replace_own`, `base` and the refusal of ops that do
//! not take effect come later; until then a netlist that has RNL records is refused.

pub mod form;
pub mod ops;

use std::collections::BTreeMap;

use rnl_runtime::diag::Sink;
use rnl_runtime::host::{HostDocument, LevelId, LevelKind};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::writeback::{append_record, carried, carrier};
use serde::Deserialize;
use serde_json::Value;

pub use ops::{LevelOp, Op};

/// The language version the header record declares: the one this toolkit reads (16.1).
pub const VERSION: &str = "0.2";

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
}

/// The path the netlist is read under: its extension tells the adapters the dialect too.
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
    if dialect != "spice" && dialect != "spectre" {
        return Err(format!("dialect \"{dialect}\" is not spice or spectre"));
    }
    let ops: Vec<LevelOp> = request.ops.iter().enumerate().map(|(k, v)| ops::parse(k, v)).collect::<Result<_, _>>()?;

    let path = entry(dialect);
    let files = MemoryProvider::new().with(path, &request.netlist);
    let mut sink = Sink::new();
    let doc = rnl_view::read_host(path, Some(dialect), &files, &mut sink)?;
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
    // a record at the end of a level goes before its last line; in a region written to be
    // encrypted it would be gone once the region is encrypted (2.2)
    for (&level, &k) in &first_op {
        let at = doc.levels[level].end_line;
        if let Some((_, a, b)) = doc.protected.iter().find(|(f, a, b)| *f == file && *a < at && at <= *b) {
            let (what, title) = (ops[k].op.name(), doc.levels[level].title());
            return Err(format!(
                "op {k} ({what}): {title} ends in a region written to be encrypted, lines {a} to {b}, where a record \
                 is gone once the region is encrypted (2.2)"
            ));
        }
    }

    let marker = carrier(&doc, &file, &request.netlist);
    let mut text = request.netlist.clone();
    // from the bottom up, so that the end lines of the levels above stay where the reader found them
    let mut order: Vec<LevelId> = levels.keys().copied().collect();
    order.sort_by_key(|lv| std::cmp::Reverse(doc.levels[*lv].end_line));
    for lv in order {
        let record = record(&request.author, &levels[&lv], marker);
        text = append_record(&doc, lv, &text, &record);
    }
    Ok(header_after_first_line(&text, &header(&packages, marker), marker, dialect == "spectre"))
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

/// Put the header record after line 1, and after the lines that continue it: SPICE's `+` lines, or
/// the next line when a Spectre line ends with `\` (2.2).
fn header_after_first_line(text: &str, header: &str, marker: &str, spectre: bool) -> String {
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.split_inclusive('\n').map(String::from).collect();
    let body = |l: &str| l.trim_end_matches(['\n', '\r']).to_string();
    if lines.is_empty() {
        // line 1 stays the title, even when there is none
        lines.push(eol.to_string());
    }
    let mut at = 1;
    while at < lines.len() {
        let continues =
            if spectre { body(&lines[at - 1]).ends_with('\\') } else { body(&lines[at]).trim_start().starts_with('+') };
        if !continues {
            break;
        }
        at += 1;
    }
    if at == lines.len() {
        if let Some(last) = lines.last_mut() {
            if !last.ends_with('\n') {
                last.push_str(eol);
            }
        }
    }
    let added: Vec<String> = carried(header, marker).into_iter().map(|l| format!("{l}{eol}")).collect();
    lines.splice(at..at, added);
    lines.concat()
}

#[cfg(test)]
mod tests;
