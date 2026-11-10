//! Explicitly opted-in migration choices, kept separate from exact rewrites.
use super::edit::{record_edit, Edit};
use super::{Approximation, FileReport};
use rnl_runtime::geom::{Orient, Pt};
use rnl_runtime::host::Record;
use rnl_runtime::problem::{
    Axis, FactKind, Filler, LevelProblem, OrientSlot, PointRef, Position, Reference, Sense, Slot,
};
use rnl_runtime::source::normalize;
use rnl_runtime::syntax::{Member, ObjExpr, Ref, Sep, Value};
use rnl_runtime::Interpretation;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, PartialEq)]
pub(super) struct Choice {
    pub direction: String,
    pub evidence: &'static str,
}
type Location = (usize, String, u32, u32);
#[derive(Default)]
pub(super) struct Approximate {
    aliases: bool,
    scoped: super::scoped_directions::Scoped,
    directions: BTreeMap<Location, Choice>,
    pairs: BTreeMap<Location, &'static str>,
    pair_roles: BTreeSet<(usize, String)>,
}
#[derive(Default)]
pub(super) struct Outcome {
    pub edits: Vec<Edit>,
    pub resolved: BTreeSet<&'static str>,
}

fn slot(slot: &Slot) -> Option<f64> {
    if let Slot::Fixed { value, .. } = slot {
        Some(*value)
    } else {
        None
    }
}
fn orientation(p: &LevelProblem, object: usize) -> Orient {
    match p.objects[object].orient {
        Some(OrientSlot::Fixed { code, .. }) => code,
        _ => Orient::R0,
    }
}
fn point(p: &LevelProblem, r: &PointRef, depth: usize) -> Option<Pt> {
    if depth > 64 {
        return None;
    }
    match r {
        PointRef::Origin(i) => match &p.objects.get(*i)?.position {
            Position::Own { x, y } => Some((slot(x)?, slot(y)?)),
            Position::At(r) => point(p, r, depth + 1),
            Position::InSymbol { owner, local, replaced } => {
                let origin = point(p, &PointRef::Origin(*owner), depth + 1)?;
                let local = orientation(p, *owner).apply(*local);
                let mut at = (origin.0 + local.0, origin.1 + local.1);
                if let Some((axis, value)) = replaced {
                    if *axis == Axis::X {
                        at.0 = slot(value)?;
                    } else {
                        at.1 = slot(value)?;
                    }
                }
                Some(at)
            }
        },
        PointRef::Pin { object, pin, .. }
        | PointRef::SymbolPin { object, pin, .. }
        | PointRef::PinPosition { object, pin, .. } => {
            let origin = point(p, &PointRef::Origin(*object), depth + 1)?;
            let symbol = p.symbols.get(p.objects.get(*object)?.symbol?)?;
            let local = orientation(p, *object).apply(symbol.pins.get(*pin)?.pos);
            Some((origin.0 + local.0, origin.1 + local.1))
        }
        PointRef::Centre(members) => {
            let at = members.iter().map(|r| point(p, r, depth + 1)).collect::<Option<Vec<_>>>()?;
            if at.is_empty() {
                return None;
            }
            Some((
                (at.iter().map(|p| p.0).fold(f64::INFINITY, f64::min)
                    + at.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max))
                    / 2.0,
                (at.iter().map(|p| p.1).fold(f64::INFINITY, f64::min)
                    + at.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max))
                    / 2.0,
            ))
        }
    }
}
fn direction(axis: Axis, increasing: bool) -> String {
    match (axis, increasing) {
        (Axis::X, true) => "left-to-right",
        (Axis::X, false) => "right-to-left",
        (Axis::Y, true) => "bottom-to-top",
        (Axis::Y, false) => "top-to-bottom",
    }
    .into()
}
fn members(kind: &FactKind) -> Option<(Axis, Sense, Vec<PointRef>)> {
    match kind {
        FactKind::Order { axis, sense, members } | FactKind::Distribute { axis, sense, members } => {
            Some((*axis, *sense, members.clone()))
        }
        FactKind::Distance { axis, sense, from, to, .. } => {
            Some((*axis, *sense, vec![PointRef::Centre(from.clone()), PointRef::Centre(to.clone())]))
        }
        _ => None,
    }
}
fn single(r: &PointRef) -> &PointRef {
    if let PointRef::Centre(v) = r {
        if v.len() == 1 {
            return single(&v[0]);
        }
    }
    r
}
fn choose(p: &LevelProblem, axis: Axis, ms: &[PointRef]) -> Choice {
    if let Some(at) =
        ms.iter().map(|m| point(p, m, 0).map(|p| if axis == Axis::X { p.0 } else { p.1 })).collect::<Option<Vec<_>>>()
    {
        if at.len() >= 2 && at.windows(2).all(|w| w[0] <= w[1]) && at.first() != at.last() {
            return Choice { direction: direction(axis, true), evidence: "existing source positions" };
        }
        if at.len() >= 2 && at.windows(2).all(|w| w[0] >= w[1]) && at.first() != at.last() {
            return Choice { direction: direction(axis, false), evidence: "existing source positions" };
        }
    }
    let mut evidence = BTreeSet::new();
    for f in &p.facts {
        if !f.chain.is_empty() || !matches!(f.kind, FactKind::Order { .. } | FactKind::Distance { .. }) {
            continue;
        }
        let Some((other_axis, sense, other)) = members(&f.kind) else { continue };
        if other_axis != axis || sense == Sense::Either {
            continue;
        }
        // A directed distance whose range permits negative values does not give
        // an ordering witness; its spelling alone is insufficient.
        if let FactKind::Distance { min, .. } = f.kind {
            if min.is_none_or(|m| m < 0.0) {
                continue;
            }
        }
        for i in 0..ms.len() {
            for j in i + 1..ms.len() {
                let a = other.iter().position(|r| single(r) == single(&ms[i]));
                let b = other.iter().position(|r| single(r) == single(&ms[j]));
                if let (Some(a), Some(b)) = (a, b) {
                    if a != b {
                        evidence.insert((a < b) == (sense == Sense::Increasing));
                    }
                }
            }
        }
    }
    if evidence.len() == 1 {
        return Choice {
            direction: direction(axis, *evidence.first().unwrap()),
            evidence: "an authored directed ORDER or DISTANCE on the same axis",
        };
    }
    Choice {
        direction: direction(axis, axis == Axis::X),
        evidence: "written member order (no unambiguous positional or directed witness)",
    }
}
impl Approximate {
    pub(super) fn new(input: &Interpretation, policies: &BTreeMap<(usize, String), super::standard::Policy>) -> Self {
        let mut out = Self { aliases: super::standard::shadows(input).1, ..Self::default() };
        if out.aliases {
            return out;
        }
        out.scoped = super::scoped_directions::Scoped::new(input);
        for p in &input.problems {
            for f in &p.facts {
                let Some(loc) = &f.source else { continue };
                if !f.chain.is_empty() {
                    continue;
                }
                if let Some((axis, Sense::Either, ms)) = members(&f.kind) {
                    out.directions.insert((p.level, normalize(&loc.file), loc.line, loc.col), choose(p, axis, &ms));
                }
            }
            for u in &p.usages {
                if u.definition != "$DIFFERENTIAL_PAIR"
                    || !policies.contains_key(&(p.level, u.definition.clone()))
                    || u.generated
                    || u.parent.is_some()
                {
                    continue;
                }
                let Some(loc) = &u.source else { continue };
                let polarities = u
                    .roles
                    .iter()
                    .take(2)
                    .map(|r| {
                        let [Filler::Object(i)] = r.members.as_slice() else { return None };
                        let Reference::Name(name) = &p.objects.get(*i)?.reference else { return None };
                        let host = &input.doc.levels[p.level];
                        let instance = host
                            .instances
                            .iter()
                            .find(|i| host.same_name(&i.name, name.strip_prefix('#').unwrap_or(name)))?;
                        match instance.mtype.as_deref()? {
                            "nmos" | "njf" | "npn" | "nmf" => Some("N"),
                            "pmos" | "pjf" | "pnp" | "lpnp" | "pmf" => Some("P"),
                            _ => None,
                        }
                    })
                    .collect::<Option<Vec<_>>>();
                if let Some(polarities) = polarities.filter(|v| v.len() == 2 && v[0] == v[1]) {
                    out.pairs.insert((p.level, normalize(&loc.file), loc.line, loc.col), polarities[0]);
                    if let Reference::Name(name) = &u.reference {
                        out.pair_roles.insert((p.level, name.clone()));
                    }
                }
            }
        }
        out
    }
    fn at<'a, T: PartialEq>(
        map: &'a BTreeMap<Location, T>,
        record: &Record,
        start: usize,
        end: usize,
        readings: &[usize],
    ) -> Option<&'a T> {
        let a = record.loc(start);
        let b = record.loc(end);
        let mut values = readings.iter().map(|reading| {
            let matches = map
                .iter()
                .filter(|((level, file, line, col), _)| {
                    level == reading
                        && *file == normalize(&a.file)
                        && (*line, *col) >= (a.line, a.col)
                        && (*line, *col) <= (b.line, b.col)
                })
                .map(|(_, v)| v)
                .collect::<Vec<_>>();
            if matches.len() == 1 {
                Some(matches[0])
            } else {
                None
            }
        });
        let value = values.next()??;
        if values.any(|v| v != Some(value)) {
            None
        } else {
            Some(value)
        }
    }
    pub(super) fn role_reference(
        &self,
        source: &str,
        record: &Record,
        r: &Ref,
        readings: &[usize],
    ) -> Result<Option<Edit>, String> {
        if r.prefix != '%' || r.sub.is_some() || r.places.is_some() {
            return Ok(None);
        }
        let Some(name) = &r.name else { return Ok(None) };
        let Some(part) = r.path.first().filter(|p| p.sep == Sep::Sub && p.sub.is_none()) else { return Ok(None) };
        let role = match part.name.as_str() {
            "in_p" => "left",
            "in_n" => "right",
            _ => return Ok(None),
        };
        if readings.is_empty()
            || !readings
                .iter()
                .all(|l| self.pair_roles.contains(&(*l, format!("%{}", rnl_runtime::problem::written_name(name)))))
        {
            return Ok(None);
        }
        let mut text = format!("%{}", rnl_runtime::problem::written_name(name));
        for (i, p) in r.path.iter().enumerate() {
            if p.sub.is_some() {
                return Ok(None);
            }
            text.push_str(if p.sep == Sep::Sub { "::" } else { ":" });
            text.push_str(&rnl_runtime::problem::written_name(if i == 0 { role } else { &p.name }));
        }
        record_edit(source, record, r.start..r.end, text, "approximate-pair-role-reference").map(Some)
    }
    pub(super) fn edits(
        &self,
        source: &str,
        record: &Record,
        member: &Member,
        readings: &[usize],
        report: &mut FileReport,
    ) -> Result<Outcome, String> {
        let mut out = Outcome::default();
        if self.aliases {
            return Ok(out);
        }
        let mut directions = 0;
        let mut converted = 0;
        for (i, s) in member.segments.iter().enumerate() {
            let Some(call) = &s.call else { continue };
            if matches!(call.kw.as_str(), "ORDER" | "DISTRIBUTE" | "DISTANCE")
                && super::directions::replacement(member, i).is_none()
            {
                let key = if call.kw == "DISTRIBUTE" { "axis" } else { "direction" };
                if let Some(Value::Str(axis)) = call
                    .params
                    .as_ref()
                    .and_then(|p| p.positional.first().or_else(|| p.get(key)))
                    .filter(|v| matches!(v,Value::Str(a) if a=="x" || a=="y"))
                {
                    directions += 1;
                    let fallback = Choice {
                        direction: if axis == "x" { "left-to-right" } else { "top-to-bottom" }.into(),
                        evidence: "written member order (no retained source fact)",
                    };
                    let choice = self
                        .scoped
                        .get(record, call.start)
                        .or_else(|| Self::at(&self.directions, record, s.start, s.end, readings))
                        .unwrap_or(&fallback);
                    let edit = record_edit(
                        source,
                        record,
                        call.start..call.end,
                        super::directions::render(call, &choice.direction, key, None),
                        "approximate-direction",
                    )?;
                    report.approximations.push(Approximation {rule:edit.change.rule.clone(),line:edit.change.line,before:edit.change.before.clone(),after:edit.change.after.clone(),risk:format!("Direction chosen from {}; the opposite solution is excluded even if source position facts are later discarded. DISTRIBUTE no longer allows a zero step, requires at least one unit, and orders even two members (14.3-14.5).",choice.evidence)});
                    out.edits.push(edit);
                    converted += 1;
                }
            }
            if matches!(call.kw.as_str(), "AS" | "ATTACH") {
                if let Some(r) = s.args.iter().find_map(|a| match &a.obj {
                    ObjExpr::Ref(r)
                        if r.prefix == '$' && r.name.as_deref() == Some("DIFFERENTIAL_PAIR") && r.path.is_empty() =>
                    {
                        Some(r)
                    }
                    _ => None,
                }) {
                    if let Some(polarity) = Self::at(&self.pairs, record, member.text_start(), member.end, readings) {
                        out.edits.push(record_edit(
                            source,
                            record,
                            r.start..r.end,
                            format!("$DIFFERENTIAL_PAIR_{polarity}"),
                            "approximate-differential-pair",
                        )?);
                        for a in &s.args {
                            if let Some(role) = a.role.as_deref() {
                                let new = match role {
                                    "in_p" => "left",
                                    "in_n" => "right",
                                    _ => continue,
                                };
                                let chars = record.text.chars().collect::<Vec<_>>();
                                if let Some(offset) = chars[a.start..a.end].iter().rposition(|c| *c == '(') {
                                    out.edits.push(record_edit(
                                        source,
                                        record,
                                        a.start + offset..a.end,
                                        format!("({new})"),
                                        "approximate-pair-role",
                                    )?);
                                }
                            }
                        }
                        let span =
                            record_edit(source, record, member.text_start()..member.end, String::new(), "preview")?;
                        let mut preview = out
                            .edits
                            .iter()
                            .filter(|e| e.start >= span.start && e.end <= span.end)
                            .cloned()
                            .map(|mut e| {
                                e.start -= span.start;
                                e.end -= span.start;
                                e
                            })
                            .collect::<Vec<_>>();
                        let after = super::edit::apply(&span.change.before, &mut preview)?;
                        report.approximations.push(Approximation {rule:"approximate-differential-pair".into(),line:record.loc(call.start).line,before:span.change.before,after,risk:"Both input devices have the same resolved model polarity. The new definition additionally imposes directed distribution/order, routing, grouping, pin orientation and assertions; the old definition only distributed the input/tail centres on x (16.3).".into()});
                        out.resolved.extend(["standard-definition-removed", "standard-name-removed"]);
                    }
                }
            }
        }
        if directions > 0 && directions == converted {
            out.resolved.insert("direction");
            if member.segments.iter().filter_map(|s| s.call.as_ref()).filter(|c| c.kw == "DISTANCE").all(|c| {
                c.params.as_ref().is_some_and(|p| {
                    p.get("max").is_none()
                        && p.get("exact").is_none()
                        && matches!(p.get("min"),Some(Value::Num {value,..}) if *value>=0.0)
                })
            }) {
                out.resolved.insert("distance-sets");
            }
        }
        let constraints = member
            .segments
            .iter()
            .filter_map(|s| s.call.as_ref())
            .filter(|c| matches!(c.kw.as_str(), "ALIGN" | "ORDER" | "DISTRIBUTE"))
            .collect::<Vec<_>>();
        if !constraints.is_empty()
            && constraints.iter().all(|c| c.kw == "ORDER")
            && super::manual::collection_constraints(member).is_some_and(|r| r.0 == "constraint-items")
        {
            let text: String =
                record.text.chars().skip(member.text_start()).take(member.end - member.text_start()).collect();
            report.approximations.push(Approximation {rule:"approximate-order-sets".into(),line:record.loc(member.text_start()).line,before:text.clone(),after:text,risk:"Kept the written ORDER. Every combination of members is now ordered; groups cannot interleave as they could when only their centres were compared (14.1, 14.3).".into()});
            out.resolved.insert("constraint-items");
        }
        Ok(out)
    }
}
