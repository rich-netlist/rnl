//! Symbols that the standard series 0.4 draws turned from those of 0.3: the diodes and the
//! transformer stand upright. Each new symbol is the old one turned, so an instance keeps its
//! drawing when its orientation is composed after the turn back: the code `c` of 0.3 becomes
//! `turn.then(c)` (the turn first, then `c`). An instance whose orientation no ORIENT gives stays
//! free: the eight orientations of the new symbol draw what those of the old one did.
use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::geom::Orient;
use rnl_runtime::host::Record;
use rnl_runtime::interp::unit::scan_package_decl;
use rnl_runtime::model::SymbolOrigin;
use rnl_runtime::problem::{FactKind, Reference};
use rnl_runtime::source::normalize;
use rnl_runtime::syntax::{Call, Member, Value};
use rnl_runtime::Interpretation;

use super::edit::{record_edit, Edit};

pub(super) const RULE: &str = "turned-symbol-orientation";

/// The symbols of 0.4 that are those of 0.3 turned, and the turn that draws a new one as the old:
/// the diodes are the old ones turned by R270, the transformer by R90.
pub(super) const TURNED: [(&str, Orient); 7] = [
    ("$DIODE", Orient::R90),
    ("$ZENER_DIODE", Orient::R90),
    ("$LED", Orient::R90),
    ("$SCHOTTKY_DIODE", Orient::R90),
    ("$SHOCKLEY_DIODE", Orient::R90),
    ("$PUT", Orient::R90),
    ("$TRANSFORMER", Orient::R270),
];

/// The turn of a symbol of 0.4 by its name with `$`.
pub(super) fn turn(name: &str) -> Option<Orient> {
    TURNED.iter().find(|(n, _)| *n == name).map(|(_, t)| *t)
}

/// Where a statement is written: the file, line and column of its segment.
type Site = (String, u32, u32);

#[derive(Default)]
pub(super) struct Turned {
    /// By the level's title and the object's id: the turn of an instance drawn with a symbol that
    /// 0.4 turns.
    pub objects: BTreeMap<(String, String), Orient>,
    /// By where an ORIENT is written: the turn of every object it orients, or None when they differ
    /// (or only some are turned).
    orients: BTreeMap<Site, Option<Orient>>,
    /// Where an ORIENT-RELATION is written that relates objects of different turns.
    relations: BTreeSet<Site>,
}

fn site(loc: &rnl_runtime::diag::Loc) -> Site {
    (normalize(&loc.file), loc.line, loc.col)
}

/// Whether the files of the package the file group read are those bundled, of series 0.3.
fn standard_03(input: &Interpretation, level: &rnl_runtime::model::LevelView, package: &str) -> bool {
    if !package.starts_with("rnl-std") || !level.packages.iter().any(|p| p.name == package && p.version == "0.3") {
        return false;
    }
    let bundled = rnl_view::bundled();
    let sources: Vec<_> =
        input.sources.iter().filter(|(_, text)| scan_package_decl(text).is_some_and(|(n, _)| n == package)).collect();
    !sources.is_empty() && sources.iter().all(|(_, text)| bundled.files.values().any(|b| b == text))
}

impl Turned {
    /// `canvas` gives the name in 0.4 of a symbol of analog-canvas-extensions by its level and its
    /// name in 0.1–0.6 without `$`.
    pub(super) fn new(input: &Interpretation, canvas: impl Fn(usize, &str) -> Option<&'static str>) -> Self {
        let mut out = Self::default();
        let mut by_level: BTreeMap<usize, BTreeMap<String, Orient>> = BTreeMap::new();
        for level in &input.model.levels {
            for object in &level.objects {
                let Some(symbol) = &object.symbol else { continue };
                let SymbolOrigin::Package(package) = &symbol.origin else { continue };
                let turn = if package == super::canvas::PACKAGE {
                    symbol
                        .name
                        .strip_prefix('$')
                        .and_then(|old| canvas(level.level, old))
                        .and_then(|new| turn(&format!("${new}")))
                } else if standard_03(input, level, package) {
                    turn(&symbol.name)
                } else {
                    None
                };
                if let Some(turn) = turn {
                    out.objects.insert((level.title.clone(), object.id.clone()), turn);
                    by_level.entry(level.level).or_default().insert(object.id.clone(), turn);
                }
            }
        }
        if by_level.is_empty() {
            return out;
        }
        let mut orients: BTreeMap<Site, BTreeSet<&str>> = BTreeMap::new();
        for problem in &input.problems {
            let turns = by_level.get(&problem.level);
            let turn_of = |object: usize| {
                let id = match &problem.objects.get(object)?.reference {
                    Reference::Name(id) => id,
                    _ => return None,
                };
                turns.and_then(|t| t.get(id)).copied()
            };
            for fact in &problem.facts {
                let Some(source) = &fact.source else { continue };
                match &fact.kind {
                    FactKind::Orient { object, .. } => {
                        orients
                            .entry(site(source))
                            .or_default()
                            .insert(turn_of(*object).unwrap_or(Orient::R0).as_str());
                    }
                    FactKind::OrientRelation { from, to, .. } => {
                        let all: BTreeSet<_> =
                            from.iter().chain(to).map(|o| turn_of(*o).unwrap_or(Orient::R0).as_str()).collect();
                        if all.len() > 1 {
                            out.relations.insert(site(source));
                        }
                    }
                    _ => {}
                }
            }
        }
        for (at, turns) in orients {
            let mut turns = turns.into_iter();
            let first = turns.next().and_then(Orient::parse).unwrap_or_default();
            if turns.next().is_some() {
                out.orients.insert(at, None);
            } else if first != Orient::R0 {
                out.orients.insert(at, Some(first));
            }
        }
        out
    }

    /// The ORIENTs of a member that orient turned instances, composed after the turn; and whether
    /// one of them, or an ORIENT-RELATION, orients instances of different turns together.
    pub(super) fn edits(&self, source: &str, record: &Record, member: &Member) -> Result<(Vec<Edit>, bool), String> {
        let mut edits = Vec::new();
        let mut mixed = false;
        if self.orients.is_empty() && self.relations.is_empty() {
            return Ok((edits, mixed));
        }
        for segment in &member.segments {
            let Some(call) = &segment.call else { continue };
            let at = site(&record.loc(segment.start));
            match call.kw.as_str() {
                "ORIENT" => match self.orients.get(&at) {
                    Some(Some(turn)) => match composed(call, *turn) {
                        Some(text) => edits.push(record_edit(source, record, call.start..call.end, text, RULE)?),
                        None => mixed = true,
                    },
                    Some(None) => mixed = true,
                    None => {}
                },
                "ORIENT-RELATION" => mixed |= self.relations.contains(&at),
                _ => {}
            }
        }
        Ok((edits, mixed))
    }
}

/// The manual review of an ORIENT or ORIENT-RELATION that orients turned and other instances together.
pub(super) const MIXED: super::manual::Reason = (
    RULE,
    "this orients instances of a diode or a transformer, which series 0.4 draws upright, together with others; give each its own orientation (11.2)",
);

/// The code written for `code` of 0.3: the turn first, then the code.
fn code(value: &Value, turn: Orient) -> Option<String> {
    match value {
        Value::Str(s) => Some(rnl_runtime::problem::written_string(turn.then(Orient::parse(s)?).as_str())),
        Value::Arr(items) if !items.is_empty() => {
            let codes = items.iter().map(|v| code(v, turn)).collect::<Option<Vec<_>>>()?;
            Some(format!("[{}]", codes.join(", ")))
        }
        _ => None,
    }
}

/// The ORIENT call with its codes composed after the turn; None when a code is not a literal one.
fn composed(call: &Call, turn: Orient) -> Option<String> {
    let params = call.params.as_ref()?;
    if params.positional.is_empty() && params.get("code").is_none() {
        return None;
    }
    let mut parts = Vec::new();
    if let Some((first, rest)) = params.positional.split_first() {
        parts.push(code(first, turn)?);
        parts.extend(rest.iter().map(super::rewrite::value));
    }
    for named in &params.named {
        let key = if named.quoted { rnl_runtime::problem::written_string(&named.name) } else { named.name.clone() };
        let value = if named.name == "code" { code(&named.value, turn)? } else { super::rewrite::value(&named.value) };
        parts.push(format!("{key}={value}"));
    }
    Some(format!("@ORIENT({})", parts.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each turn draws the symbol of 0.4 as that of 0.3: a pin of 0.4 turned is the pin of 0.3.
    #[test]
    fn the_turn_draws_the_new_pins_where_the_old_ones_were() {
        // (0.4 pin, 0.4 direction) → (0.3 pin, 0.3 direction)
        let diode = [((0.0, 2.0), 90, (-2.0, 0.0), 180), ((0.0, -2.0), 270, (2.0, 0.0), 0)];
        let put_gate = ((2.0, 1.0), 0, (-1.0, 2.0), 90);
        let transformer = [((-1.0, 2.0), 90, (2.0, 1.0), 0), ((-1.0, -2.0), 270, (-2.0, 1.0), 180)];
        for (new, dir, old, old_dir) in diode.into_iter().chain([put_gate]) {
            assert_eq!(turn("$PUT").unwrap().apply(new), old);
            assert_eq!(turn("$PUT").unwrap().direction(dir), Some(old_dir));
        }
        for (new, dir, old, old_dir) in transformer {
            assert_eq!(turn("$TRANSFORMER").unwrap().apply(new), old);
            assert_eq!(turn("$TRANSFORMER").unwrap().direction(dir), Some(old_dir));
        }
    }
}
