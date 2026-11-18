//! The box that the viewer of 0.3 draws for an instance with no symbol (9.2.6), kept in 0.4.
//!
//! 0.4 draws such an instance with its default block symbol, whose pins lie elsewhere: on the edge
//! of a square as wide as the pins are many, the right ones centred. A wire written to a pin of the
//! box of 0.3 would then end beside it, often on a wire of another net. So the upgrade writes the
//! box of 0.3 as a symbol of the file, `$BOX_<cell>`, drawn as the reading of 0.3 draws it (its
//! dashed outline, the stubs, the pins by terminal number with their labels, and the name): the
//! block symbol of the subcircuit when the file defines it (`# @SET-SYMBOL`), else, in the level
//! of each instance, the symbol of each one. Every instance of a cell has the same box. Deleting
//! the record gives the default block symbol of 0.4.
use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::geom::fmt_num;
use rnl_runtime::host::{HostDocument, LevelId};
use rnl_runtime::interp::obj::{PrimParams, Shape};
use rnl_runtime::model::{quoted, SymbolView};
use rnl_runtime::problem::written_string;
use rnl_runtime::writeback::{carried, carrier_at, record_line};
use rnl_runtime::Interpretation;

use super::edit::Edit;
use super::{Change, Unconverted};

/// The rule of the records written, in the changes and the notices of a file.
pub(super) const RULE: &str = "default-box";

/// One record to write, at the end of a level.
struct Box {
    level: LevelId,
    cell: String,
    name: String,
    record: String,
}

/// The records to write, and the names of the symbols they define.
#[derive(Default)]
pub(super) struct DefaultBoxes {
    boxes: Vec<Box>,
    pub names: BTreeSet<String>,
}

impl DefaultBoxes {
    /// The boxes of the instances that the reading of 0.3, `before`, draws with the viewer's own
    /// stand-in. A box that is not one of the shapes the viewer of 0.3 draws is left alone.
    pub(super) fn new(before: &Interpretation) -> DefaultBoxes {
        let doc = &before.doc;
        // the subcircuit's level, or the level of the instances and the cell: the body of the box
        // and the instances drawn with it
        let mut blocks: BTreeMap<LevelId, (String, String)> = BTreeMap::new();
        let mut local: BTreeMap<(LevelId, String, String), Vec<String>> = BTreeMap::new();
        for level in &before.model.levels {
            if level.language.as_str() >= "0.4" {
                continue;
            }
            for object in &level.objects {
                if object.kind != "instance" || !object.symbol.as_ref().is_some_and(|s| s.default) {
                    continue;
                }
                let Some(view) = view(level, &object.id) else { continue };
                let Some(body) = body(view) else { continue };
                let name = object.id.trim_start_matches('#');
                let Some(instance) =
                    doc.levels[level.level].instances.iter().find(|i| doc.key(&i.name) == doc.key(name))
                else {
                    continue;
                };
                let Some(cell) = instance.model.clone() else { continue };
                match doc.subckt_of(instance) {
                    Some(sub) => {
                        blocks.entry(sub).or_insert((cell, body));
                    }
                    None => local.entry((level.level, cell, body)).or_default().push(instance.name.clone()),
                }
            }
        }
        let mut taken: BTreeSet<String> = BTreeSet::new();
        let mut out = DefaultBoxes::default();
        for (level, (cell, body)) in blocks {
            let name = fresh(doc, &cell, &mut taken);
            let record = format!("@RNL {{\n  {name} = @DEFINE-SYMBOL {{\n{body}  }};\n  # @SET-SYMBOL {name}\n}}");
            out.names.insert(name.clone());
            out.boxes.push(Box { level, cell, name, record });
        }
        for ((level, cell, body), instances) in local {
            let name = fresh(doc, &cell, &mut taken);
            let uses: String = instances.iter().map(|i| format!(";\n  #{} @SET-SYMBOL {name}", quoted(i))).collect();
            let record = format!("@RNL {{\n  {name} = @DEFINE-SYMBOL {{\n{body}  }}{uses}\n}}");
            out.names.insert(name.clone());
            out.boxes.push(Box { level, cell, name, record });
        }
        out
    }

    /// The records of the levels written in the file `path`, each before the end of its level, as
    /// edits of `source`, each with a notice that says how to have the box of 0.4 instead.
    pub(super) fn edits(
        &self,
        doc: &HostDocument,
        path: &str,
        source: &str,
        notices: &mut Vec<Unconverted>,
    ) -> Vec<Edit> {
        let mut out = Vec::new();
        for b in self.boxes.iter().filter(|b| doc.levels.get(b.level).is_some_and(|l| l.path == path)) {
            let Some(edit) = at_end(doc, b.level, path, source, &b.record, RULE) else { continue };
            let line = edit.change.line;
            out.push(edit);
            notices.push(Unconverted {
                rule: RULE.into(),
                line,
                before: String::new(),
                reason: format!(
                    "{} keeps the box that 0.3 draws for an instance of {} with no symbol; delete this record to draw them with the default block symbol of 0.4 instead (9.2.6)",
                    b.name, b.cell
                ),
            });
        }
        out
    }
}

/// The record `record`, carried as the file `path` carries RNL there, written before the end of the
/// level `level` of `source`, under the rule `rule`; None where the level has no such line.
pub(super) fn at_end(
    doc: &HostDocument,
    level: LevelId,
    path: &str,
    source: &str,
    record: &str,
    rule: &str,
) -> Option<Edit> {
    let eol = if source.contains("\r\n") { "\r\n" } else { "\n" };
    let line = record_line(doc, level, source).ok()?;
    let at = line_start(source, line)?;
    let marker = carrier_at(doc, super::rewrite::document_name(doc, path), source, line);
    let mut text: String = carried(record, marker).into_iter().map(|l| format!("{l}{eol}")).collect();
    // at the end of a text whose last line has no end of line
    if at == source.len() && !source.is_empty() && !source.ends_with('\n') {
        text.insert_str(0, eol);
    }
    Some(Edit {
        start: at,
        end: at,
        replacement: text.clone(),
        change: Change { rule: rule.into(), line, before: String::new(), after: text },
    })
}

/// The symbol an object of a level is drawn with, placed or not.
fn view<'a>(level: &'a rnl_runtime::model::LevelView, id: &str) -> Option<&'a SymbolView> {
    let object = level.objects.iter().find(|o| o.id == id)?;
    match object.symbol_view {
        Some(k) => level.symbols.get(k),
        None => {
            let item = level.unplaced_items.iter().find(|i| i.object_id == id)?;
            level.unplaced_symbols.get(item.symbol)
        }
    }
}

/// The body of a symbol drawn as `view`, one statement a line: the outline and stubs that the
/// viewer of 0.3 draws, its pins by terminal number, and its labels. None for anything else.
fn body(view: &SymbolView) -> Option<String> {
    if !view.default || !view.nested.is_empty() || view.box_written {
        return None;
    }
    let mut lines = Vec::new();
    let pt = |p: &(f64, f64)| format!("({}, {})", fmt_num(p.0), fmt_num(p.1));
    for prim in &view.prims {
        let keyword = match prim.shape {
            Shape::Rectangle => "RECTANGLE",
            Shape::Line => "LINE",
            _ => return None,
        };
        if prim.params != PrimParams::default() || prim.text.is_some() {
            return None;
        }
        let style: Vec<String> = prim.style.iter().map(|(k, v)| format!("{k}={}", v.show())).collect();
        let style = if style.is_empty() { String::new() } else { format!("({})", style.join(", ")) };
        let points: Vec<String> = prim.points.iter().map(pt).collect();
        lines.push(format!("@{keyword}{style} {{{}}}", points.join(" ")));
    }
    for pin in &view.pins {
        let [terminal] = pin.terminals.as_slice() else { return None };
        let number = terminal.rsplit(':').next().filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))?;
        let direction = match pin.dir? {
            0 => "right",
            90 => "up",
            180 => "left",
            270 => "down",
            _ => return None,
        };
        let label = pin.label.as_deref().map(|l| format!(", label={}", written_string(l))).unwrap_or_default();
        lines.push(format!("{} @PIN(terminal={number}, direction=\"{direction}\"{label})", pt(&pin.pos)));
    }
    for label in &view.labels {
        let (Some(key), Some(at)) = (&label.key, label.at) else { return None };
        if label.rotate != 0.0 || !label.style.is_empty() {
            return None;
        }
        let mut params = vec![written_string(&label.text), format!("key={}", written_string(key))];
        for (name, value) in [("halign", &label.halign), ("valign", &label.valign)] {
            if let Some(value) = value {
                params.push(format!("{name}={}", written_string(value)));
            }
        }
        lines.push(format!("? @TEXT({}) => @PLACE{}", params.join(", "), pt(&at)));
    }
    Some(
        lines
            .iter()
            .enumerate()
            .map(|(k, l)| format!("    {l}{}\n", if k + 1 < lines.len() { ";" } else { "" }))
            .collect(),
    )
}

/// `$BOX_<cell>`, the cell's letters, digits and `_` kept and others `_`, not spelled like a name
/// any file writes or a box already named.
fn fresh(doc: &HostDocument, cell: &str, taken: &mut BTreeSet<String>) -> String {
    let base: String = cell.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    let used = |name: &str| {
        taken.contains(name)
            || doc.sources.iter().any(|(_, text)| {
                text.match_indices(name).any(|(at, _)| {
                    !text[at + name.len()..].chars().next().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                })
            })
    };
    let mut name = format!("$BOX_{base}");
    let mut k = 2;
    while used(&name) {
        name = format!("$BOX_{base}_{k}");
        k += 1;
    }
    taken.insert(name.clone());
    name
}

/// The byte where the line `line` (from 1) of `text` starts.
fn line_start(text: &str, line: u32) -> Option<usize> {
    if line <= 1 {
        return Some(0);
    }
    text.match_indices('\n').nth(line as usize - 2).map(|(at, _)| at + 1).or_else(|| {
        // one past the last line: the end of the text
        (text.lines().count() + 1 == line as usize).then_some(text.len())
    })
}
