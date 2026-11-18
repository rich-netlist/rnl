//! Symbols that the standard series 0.4 draws turned or moved from those of 0.3 and of
//! analog-canvas-extensions 0.1–0.6: the diodes and the transformer stand upright, and the UJT has
//! its origin on the column of b1 and b2. Each new symbol is the old one turned and moved
//! ([`Frame`]), so an instance keeps its drawing when its orientation is composed after the turn
//! back and its origin moves by the shift: the code `c` of 0.3 becomes `turn.then(c)` (the turn
//! first, then `c`), and each coordinate a PLACE writes for its origin moves by `c` applied to the
//! shift. An instance whose orientation no ORIENT gives stays free: the eight orientations of the
//! new symbol draw what those of the old one did; its PLACE moves as for R0, the orientation a
//! viewer draws it in (11.2).
use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::geom::{fmt_num, Orient, Pt};
use rnl_runtime::host::Record;
use rnl_runtime::interp::unit::scan_package_decl;
use rnl_runtime::model::SymbolOrigin;
use rnl_runtime::problem::{Axis, FactKind, OrientSlot, PointRef, Reference};
use rnl_runtime::source::normalize;
use rnl_runtime::syntax::{Call, Member, Value};
use rnl_runtime::Interpretation;

use super::edit::{record_edit, Edit};

pub(super) const RULE: &str = "turned-symbol-orientation";

/// The rule of the PLACEs that move with the origin of a symbol.
pub(super) const ORIGIN_RULE: &str = "moved-symbol-origin";

/// How a symbol of 0.4 draws the earlier one: the point `p` of the new symbol is the point
/// `turn(p) + shift` of the old.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Frame {
    pub turn: Orient,
    pub shift: Pt,
}

impl Frame {
    const fn turned(turn: Orient) -> Self {
        Self { turn, shift: (0.0, 0.0) }
    }

    const fn shifted(x: f64, y: f64) -> Self {
        Self { turn: Orient::R0, shift: (x, y) }
    }

    /// The point of the old symbol that the point `p` of the new one is.
    pub(super) fn old(self, p: Pt) -> Pt {
        let (x, y) = self.turn.apply(p);
        (x + self.shift.0, y + self.shift.1)
    }

    /// What the origin of an instance that `code` of 0.3 orients moves by, so that the new symbol
    /// draws the old one where it was: the shift, oriented.
    pub(super) fn offset(self, code: Orient) -> Pt {
        let (x, y) = code.apply(self.shift);
        (x + 0.0, y + 0.0)
    }
}

/// The symbols of 0.4 that draw the earlier ones turned or moved: the diodes are the old ones
/// turned by R270, the transformer by R90; the UJT is the one of analog-canvas-extensions moved
/// one unit to the left.
pub(super) const MOVED: [(&str, Frame); 8] = [
    ("$DIODE", Frame::turned(Orient::R90)),
    ("$ZENER_DIODE", Frame::turned(Orient::R90)),
    ("$LED", Frame::turned(Orient::R90)),
    ("$SCHOTTKY_DIODE", Frame::turned(Orient::R90)),
    ("$SHOCKLEY_DIODE", Frame::turned(Orient::R90)),
    ("$PUT", Frame::turned(Orient::R90)),
    ("$TRANSFORMER", Frame::turned(Orient::R270)),
    ("$UJT", Frame::shifted(1.0, 0.0)),
];

/// The frame of a symbol of 0.4 by its name with `$`.
pub(super) fn frame(name: &str) -> Option<Frame> {
    MOVED.iter().find(|(n, _)| *n == name).map(|(_, f)| *f)
}

/// The coordinate of a point on an axis.
fn along(axis: Axis, (x, y): Pt) -> f64 {
    match axis {
        Axis::X => x,
        Axis::Y => y,
    }
}

/// Where a statement is written: the file, line and column of its segment.
type Site = (String, u32, u32);

#[derive(Default)]
pub(super) struct Turned {
    /// By the level's file and title and the object's id: the frame of an instance drawn with a
    /// symbol that 0.4 turns or moves.
    pub objects: BTreeMap<(String, String, String), Frame>,
    /// By where an ORIENT is written: the turn of every object it orients, or None when they differ
    /// (or only some are turned).
    orients: BTreeMap<Site, Option<Orient>>,
    /// Where an ORIENT-RELATION is written that relates objects of different turns.
    relations: BTreeSet<Site>,
    /// By where a PLACE is written: what the origin of every object it places moves by, or None
    /// when they differ (or only some move).
    places: BTreeMap<Site, Option<Pt>>,
    /// Where a fact is written that measures from the origin of a moved instance along an axis on
    /// which it moves: an ALIGN, ORDER, DISTRIBUTE, DISTANCE or TABLE, or an offset of a text.
    origins: BTreeSet<Site>,
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
        let mut by_level: BTreeMap<usize, BTreeMap<String, Frame>> = BTreeMap::new();
        for level in &input.model.levels {
            for object in &level.objects {
                let Some(symbol) = &object.symbol else { continue };
                let SymbolOrigin::Package(package) = &symbol.origin else { continue };
                let frame = if package == super::canvas::PACKAGE {
                    symbol
                        .name
                        .strip_prefix('$')
                        .and_then(|old| canvas(level.level, old))
                        .and_then(|new| frame(&format!("${new}")))
                } else if standard_03(input, level, package) {
                    frame(&symbol.name)
                } else {
                    None
                };
                if let Some(frame) = frame {
                    let file = input.doc.levels[level.level].path.clone();
                    out.objects.insert((file, level.title.clone(), object.id.clone()), frame);
                    by_level.entry(level.level).or_default().insert(object.id.clone(), frame);
                }
            }
        }
        if by_level.is_empty() {
            return out;
        }
        let mut orients: BTreeMap<Site, BTreeSet<&str>> = BTreeMap::new();
        let mut places: BTreeMap<Site, [Vec<f64>; 2]> = BTreeMap::new();
        for problem in &input.problems {
            let frames = by_level.get(&problem.level);
            let frame_of = |object: usize| {
                let id = match &problem.objects.get(object)?.reference {
                    Reference::Name(id) => id,
                    _ => return None,
                };
                frames.and_then(|t| t.get(id)).copied()
            };
            let turn_of = |object: usize| frame_of(object).map(|f| f.turn);
            // what the origin of an object moves by: the shift, oriented as 0.3 orients it, or as
            // a viewer draws it when no ORIENT does (11.2)
            let offset_of = |object: usize| {
                let frame = frame_of(object)?;
                let code = match &problem.objects.get(object)?.orient {
                    Some(OrientSlot::Fixed { code, .. }) => *code,
                    _ => Orient::R0,
                };
                Some(frame.offset(code)).filter(|&o| o != (0.0, 0.0))
            };
            let moves = |at: &PointRef, axis: Axis| moved_along(at, axis, &offset_of);
            for fact in &problem.facts {
                let Some(source) = &fact.source else { continue };
                let measured = match &fact.kind {
                    FactKind::Align { axis, members }
                    | FactKind::Order { axis, members, .. }
                    | FactKind::Distribute { axis, members, .. } => members.iter().any(|m| moves(m, *axis)),
                    FactKind::Distance { axis, from, to, .. } => from.iter().chain(to).any(|m| moves(m, *axis)),
                    FactKind::Table { axis, rows } => rows.points().into_iter().any(|m| moves(m, *axis)),
                    FactKind::Offset { from, axis, .. } => moves(from, *axis),
                    _ => false,
                };
                if measured {
                    out.origins.insert(site(source));
                }
                match &fact.kind {
                    // each axis on its own: a PLACE that writes one coordinate moves only on it
                    FactKind::Place { at: PointRef::Origin(object), axis, .. } => {
                        let by = along(*axis, offset_of(*object).unwrap_or((0.0, 0.0)));
                        let axes = places.entry(site(source)).or_default();
                        let offsets = &mut axes[usize::from(*axis == Axis::Y)];
                        if !offsets.contains(&by) {
                            offsets.push(by);
                        }
                    }
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
        for (at, [x, y]) in places {
            // what the PLACE moves by on an axis: nothing when it writes none there, None when the
            // objects it places move differently
            let on = |offsets: &[f64]| match offsets {
                [] => Some(0.0),
                [by] => Some(*by),
                _ => None,
            };
            match (on(&x), on(&y)) {
                (Some(0.0), Some(0.0)) => {}
                (Some(dx), Some(dy)) => {
                    out.places.insert(at, Some((dx, dy)));
                }
                _ => {
                    out.places.insert(at, None);
                }
            }
        }
        out
    }

    /// The ORIENTs of a member that orient turned instances, composed after the turn, and the
    /// PLACEs that place moved ones, moved with their origin; and the manual reviews of what
    /// cannot be: an ORIENT or ORIENT-RELATION that orients instances of different turns together,
    /// a PLACE that places instances that move differently, or one whose coordinate is not a
    /// number, and a fact that measures from the origin of a moved instance.
    pub(super) fn edits(
        &self,
        source: &str,
        record: &Record,
        member: &Member,
    ) -> Result<(Vec<Edit>, Vec<super::manual::Reason>), String> {
        let mut edits = Vec::new();
        let mut reasons = Vec::new();
        let mut review = |reason: super::manual::Reason| {
            if !reasons.contains(&reason) {
                reasons.push(reason);
            }
        };
        if self.orients.is_empty() && self.relations.is_empty() && self.places.is_empty() && self.origins.is_empty() {
            return Ok((edits, reasons));
        }
        for segment in &member.segments {
            let Some(call) = &segment.call else { continue };
            let at = site(&record.loc(segment.start));
            match call.kw.as_str() {
                "ORIENT" => match self.orients.get(&at) {
                    Some(Some(turn)) => match composed(call, *turn) {
                        Some(text) => edits.push(record_edit(source, record, call.start..call.end, text, RULE)?),
                        None => review(MIXED),
                    },
                    Some(None) => review(MIXED),
                    None => {}
                },
                "ORIENT-RELATION" if self.relations.contains(&at) => review(MIXED),
                "PLACE" => match self.places.get(&at) {
                    Some(Some(offset)) => match placed(call, *offset) {
                        Some(text) => edits.push(record_edit(source, record, call.start..call.end, text, ORIGIN_RULE)?),
                        None => review(MOVED_PLACE),
                    },
                    Some(None) => review(MOVED_PLACE),
                    None => {}
                },
                _ => {}
            }
            if self.origins.contains(&at) {
                review(MEASURED);
            }
        }
        Ok((edits, reasons))
    }
}

/// Whether a point is on the origin of an object that moves along `axis`.
fn moved_along(at: &PointRef, axis: Axis, offset_of: &impl Fn(usize) -> Option<Pt>) -> bool {
    match at {
        PointRef::Origin(object) => offset_of(*object).is_some_and(|o| along(axis, o) != 0.0),
        PointRef::Centre(points) => points.iter().any(|p| moved_along(p, axis, offset_of)),
        _ => false,
    }
}

/// The manual review of an ORIENT or ORIENT-RELATION that orients turned and other instances together.
pub(super) const MIXED: super::manual::Reason = (
    RULE,
    "this orients instances of a diode or a transformer, which series 0.4 draws upright, together with others; give each its own orientation (11.2)",
);

/// The manual review of a PLACE of instances whose origin moves, that cannot move with it.
pub(super) const MOVED_PLACE: super::manual::Reason = (
    ORIGIN_RULE,
    "this places an instance of a symbol whose origin series 0.4 moves, together with others or by a coordinate that is not a number; place it on its own (11.1)",
);

/// The manual review of a fact that measures from the origin of an instance whose origin moves.
pub(super) const MEASURED: super::manual::Reason = (
    ORIGIN_RULE,
    "this measures from the origin of an instance of a symbol whose origin series 0.4 moves; measure from its pins, or move the coordinates with it (11.1, 14.1)",
);

/// The PLACE call with each coordinate it writes moved by `offset`; None when one that moves is not
/// a number.
fn placed(call: &Call, (dx, dy): Pt) -> Option<String> {
    let params = call.params.as_ref()?;
    let moved = |value: &Value, by: f64| match value {
        _ if by == 0.0 => Some(super::rewrite::value(value)),
        Value::Num { value, .. } => Some(fmt_num(value + by)),
        Value::Word(free) if free == "_" => Some(free.clone()),
        _ => None,
    };
    let mut parts = Vec::new();
    for (index, value) in params.positional.iter().enumerate() {
        parts.push(match index {
            0 => moved(value, dx)?,
            1 => moved(value, dy)?,
            _ => super::rewrite::value(value),
        });
    }
    for named in &params.named {
        let key = if named.quoted { rnl_runtime::problem::written_string(&named.name) } else { named.name.clone() };
        let value = match named.name.as_str() {
            "x" => moved(&named.value, dx)?,
            "y" => moved(&named.value, dy)?,
            _ => super::rewrite::value(&named.value),
        };
        parts.push(format!("{key}={value}"));
    }
    Some(format!("@PLACE({})", parts.join(", ")))
}

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
            assert_eq!(frame("$PUT").unwrap().old(new), old);
            assert_eq!(frame("$PUT").unwrap().turn.direction(dir), Some(old_dir));
        }
        for (new, dir, old, old_dir) in transformer {
            assert_eq!(frame("$TRANSFORMER").unwrap().old(new), old);
            assert_eq!(frame("$TRANSFORMER").unwrap().turn.direction(dir), Some(old_dir));
        }
    }

    /// The UJT of 0.4 is the one of analog-canvas-extensions moved one unit to the left: each of its
    /// pins is the old one, and an instance moves by the shift as its orientation turns it.
    #[test]
    fn the_shift_draws_the_new_pins_where_the_old_ones_were() {
        let ujt = frame("$UJT").unwrap();
        // (0.4 pin) → (0.6 pin of CANVAS_UJT)
        for (new, old) in [((-3.0, -1.0), (-2.0, -1.0)), ((0.0, -2.0), (1.0, -2.0)), ((0.0, 2.0), (1.0, 2.0))] {
            assert_eq!(ujt.old(new), old);
        }
        assert_eq!(ujt.offset(Orient::R0), (1.0, 0.0));
        assert_eq!(ujt.offset(Orient::R90), (0.0, 1.0));
        assert_eq!(ujt.offset(Orient::MY), (-1.0, 0.0));
        // a turned symbol does not move
        assert_eq!(frame("$DIODE").unwrap().offset(Orient::R90), (0.0, 0.0));
    }
}
