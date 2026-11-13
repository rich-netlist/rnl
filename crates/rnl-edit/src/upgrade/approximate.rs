//! Explicitly opted-in migration choices, kept separate from exact rewrites.
use super::edit::{record_edit, Edit};
use super::{Approximation, FileReport};
use rnl_runtime::geom::{Orient, Pt};
use rnl_runtime::host::{LevelKind, Record};
use rnl_runtime::problem::{
    Axis, FactKind, Filler, LevelProblem, ObjectKind, OrientSlot, PointRef, Position, Reference, Sense, Slot,
    UsageEntry,
};
use rnl_runtime::source::normalize;
use rnl_runtime::syntax::{Arg, Member, ObjExpr, Ref, Segment, Sep, Value};
use rnl_runtime::Interpretation;
use std::collections::{BTreeMap, BTreeSet};

/// The definitions that an approximate rule turns a usage into, as (old, new) without the `$`: the
/// one list of them, which the comparison also reads to tell a conversion on purpose from a mistake.
pub(super) const CONVERSIONS: &[(&str, &str)] = &[
    ("IO_PORT", "NET_LABEL"),
    ("IOPORT", "NET_LABEL"),
    ("CURRENT_MIRROR", "CURRENT_MIRROR_N"),
    ("CURRENT_MIRROR", "CURRENT_MIRROR_P"),
    ("DIFFERENTIAL_PAIR", "DIFFERENTIAL_PAIR_N"),
    ("DIFFERENTIAL_PAIR", "DIFFERENTIAL_PAIR_P"),
];

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
    pairs: BTreeMap<Location, Pair>,
    conflicts: BTreeMap<Location, bool>,
    pair_roles: BTreeMap<(usize, String), Pair>,
    mirrors: BTreeMap<Location, (&'static str, &'static str)>,
    top_io_ports: BTreeSet<Location>,
    flows: BTreeMap<Location, Flow>,
}

#[derive(Clone, PartialEq, Eq)]
struct Pair {
    kind: &'static str,
    first_role: &'static str,
    second_role: &'static str,
    first_left: bool,
    evidence: &'static str,
}

impl Pair {
    fn role(&self, old: &str) -> Option<&'static str> {
        let left = if old == self.first_role {
            self.first_left
        } else if old == self.second_role {
            !self.first_left
        } else {
            return None;
        };
        Some(if left { "left" } else { "right" })
    }
}

#[derive(PartialEq, Eq)]
struct Flow {
    order: String,
    grouped: bool,
}
#[derive(Default)]
pub(super) struct Outcome {
    pub edits: Vec<Edit>,
    pub resolved: BTreeSet<&'static str>,
    pub review: Option<super::manual::Reason>,
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

fn role_object(usage: &UsageEntry, role: &str) -> Option<usize> {
    match usage.roles.iter().find(|r| r.role == role)?.members.as_slice() {
        [Filler::Object(i)] => Some(*i),
        _ => None,
    }
}

fn application(segment: &Segment) -> Option<(&str, &[Arg])> {
    let (definition, args) = match segment.call.as_ref()?.kw.as_str() {
        "AS" => segment.args.split_last()?,
        "ATTACH" => segment.args.split_first()?,
        _ => return None,
    };
    match &definition.obj {
        ObjExpr::Ref(r) if r.prefix == '$' && r.path.is_empty() => Some((r.name.as_deref()?, args)),
        _ => None,
    }
}

fn pair_direction(
    p: &LevelProblem,
    first: usize,
    second: usize,
    mirrored: &BTreeSet<usize>,
) -> Result<Option<bool>, ()> {
    let a = PointRef::Origin(first);
    let b = PointRef::Origin(second);
    let mut evidence = BTreeSet::new();
    let mut positions = [Vec::new(), Vec::new()];
    for fact in &p.facts {
        if fact.chain.is_empty() {
            if let FactKind::Place { at, axis: Axis::X, value } = &fact.kind {
                if at == &a {
                    positions[0].push(*value);
                } else if at == &b {
                    positions[1].push(*value);
                }
            }
        } else if !matches!(fact.chain.as_slice(), [usage] if mirrored.contains(usage)) {
            continue;
        }
        match &fact.kind {
            FactKind::Order { axis: Axis::X, sense, members } if *sense != Sense::Either => {
                let i = members.iter().position(|r| single(r) == &a);
                let j = members.iter().position(|r| single(r) == &b);
                if let (Some(i), Some(j)) = (i, j) {
                    evidence.insert((i < j) == (*sense == Sense::Increasing));
                }
            }
            FactKind::Distance { axis: Axis::X, sense, min: Some(min), from, to, .. }
                if *sense != Sense::Either && *min >= 0.0 =>
            {
                // A nonnegative extent minimum orders every point across the two sides.
                if from.iter().any(|r| single(r) == &a) && to.iter().any(|r| single(r) == &b) {
                    evidence.insert(*sense == Sense::Increasing);
                }
                if from.iter().any(|r| single(r) == &b) && to.iter().any(|r| single(r) == &a) {
                    evidence.insert(*sense == Sense::Decreasing);
                }
            }
            _ => {}
        }
    }
    for x in &positions[0] {
        for y in &positions[1] {
            if x != y {
                evidence.insert(x < y);
            }
        }
    }
    if evidence.len() > 1 {
        return Err(());
    }
    Ok(evidence.first().copied())
}

fn flow(p: &LevelProblem, usage: usize) -> Option<Flow> {
    let mut facts = p.facts.iter().filter(|f| f.chain == [usage]);
    let fact = facts.next()?;
    let FactKind::Order { axis, sense, members } = &fact.kind else { return None };
    if facts.next().is_some() || *sense == Sense::Either {
        return None;
    }
    fn written(p: &LevelProblem, point: &PointRef) -> Option<String> {
        match single(point) {
            PointRef::Origin(i) => match &p.objects.get(*i)?.reference {
                Reference::Name(n) if !n.contains(':') => Some(n.clone()),
                _ => None,
            },
            PointRef::Centre(members) => {
                let members = members.iter().map(|m| written(p, m)).collect::<Option<Vec<_>>>()?;
                Some(format!("{{{}}}", members.join(" ")))
            }
            _ => None,
        }
    }
    let path = members.iter().map(|m| written(p, m)).collect::<Option<Vec<_>>>()?.join(" ");
    let direction = direction(*axis, *sense == Sense::Increasing);
    let order = format!("{{{path}}} @ORDER(\"{direction}\", priority=\"{}\")", fact.priority.as_str());
    Some(Flow { order, grouped: members.iter().any(|m| matches!(single(m), PointRef::Centre(_))) })
}

fn record_approximation(
    source: &str,
    record: &Record,
    member: &Member,
    edits: &[Edit],
    rule: &str,
    risk: String,
    report: &mut FileReport,
) -> Result<(), String> {
    let span = record_edit(source, record, member.text_start()..member.end, String::new(), "preview")?;
    let mut preview = edits.to_vec();
    for edit in &mut preview {
        edit.start -= span.start;
        edit.end -= span.start;
    }
    let after = super::edit::apply(&span.change.before, &mut preview)?;
    report.approximations.push(Approximation {
        rule: rule.into(),
        line: span.change.line,
        before: span.change.before,
        after,
        risk,
    });
    Ok(())
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
        let mut applications = BTreeMap::new();
        for record in input.doc.levels.iter().flat_map(|l| &l.records) {
            let Some(body) = rnl_runtime::syntax::parse_record_features(&record.text, true, false).body else {
                continue;
            };
            for member in &body.members {
                let mut uses = member.segments.iter().filter(|s| application(s).is_some());
                let Some(segment) = uses.next().filter(|_| uses.next().is_none()) else { continue };
                let Some((name, args)) = application(segment) else { continue };
                if !matches!(name, "MIRRORED" | "DIFFERENTIAL_PAIR") || segment.piped {
                    continue;
                }
                let first = args
                    .iter()
                    .filter_map(|arg| arg.role.as_deref())
                    .find(|role| matches!(*role, "a" | "b" | "in_p" | "in_n"));
                let loc = record.loc(segment.start);
                applications
                    .insert((normalize(&loc.file), loc.line, loc.col), first.is_none_or(|r| matches!(r, "a" | "in_p")));
            }
        }
        let device_polarity = |p: &LevelProblem, i: usize| -> Option<bool> {
            let Reference::Name(name) = &p.objects.get(i)?.reference else { return None };
            let host = &input.doc.levels[p.level];
            let instance = host
                .instances
                .iter()
                .find(|inst| host.same_name(&inst.name, name.strip_prefix('#').unwrap_or(name)))?;
            match instance.mtype.as_deref()? {
                "nmos" | "njf" | "npn" | "nmf" => Some(true),
                "pmos" | "pjf" | "pnp" | "lpnp" | "pmf" => Some(false),
                _ => None,
            }
        };
        let mut sites = BTreeMap::<_, Vec<_>>::new();
        let mut pair_names = Vec::new();
        for p in &input.problems {
            let mut written_sides = BTreeMap::new();
            let mut io_sites = BTreeMap::new();
            for f in &p.facts {
                let Some(loc) = &f.source else { continue };
                if !f.chain.is_empty() {
                    continue;
                }
                if let Some((axis, Sense::Either, ms)) = members(&f.kind) {
                    out.directions.insert((p.level, normalize(&loc.file), loc.line, loc.col), choose(p, axis, &ms));
                }
            }
            let mirrored = p
                .usages
                .iter()
                .enumerate()
                .filter_map(|(i, u)| {
                    (u.definition == "$MIRRORED"
                        && !u.generated
                        && u.parent.is_none()
                        && policies.contains_key(&(p.level, u.definition.clone())))
                    .then_some(i)
                })
                .collect();
            for (index, u) in p.usages.iter().enumerate() {
                if u.generated || u.parent.is_some() {
                    continue;
                }
                let Some(loc) = &u.source else { continue };
                let file = normalize(&loc.file);
                let key = (p.level, file.clone(), loc.line, loc.col);
                let site = (file, loc.line, loc.col);
                if applications.contains_key(&site) {
                    sites.entry(site.clone()).or_default().push(key.clone());
                }
                let Some(policy) = policies.get(&(p.level, u.definition.clone())) else { continue };
                if matches!(u.definition.as_str(), "$IO_PORT" | "$IOPORT") {
                    let eligible = input.doc.levels[p.level].kind == LevelKind::File
                        && match u.roles.first().map(|r| r.members.as_slice()) {
                            Some([Filler::Net(n)]) => {
                                p.nets.iter().find(|net| net.name == *n).is_some_and(|net| net.port.is_none())
                            }
                            Some([Filler::Terminal { .. }]) => {
                                u.object.and_then(|o| p.objects.get(o)).is_some_and(|obj| {
                                    !obj.nets.is_empty()
                                        && obj
                                            .nets
                                            .iter()
                                            .all(|&ni| p.nets.get(ni).is_some_and(|net| net.port.is_none()))
                                })
                            }
                            _ => false,
                        };
                    io_sites.entry(key).and_modify(|v| *v &= eligible).or_insert(eligible);
                    continue;
                }
                if policy.restore_flow {
                    if let Some(flow) = flow(p, index) {
                        out.flows.insert(key, flow);
                    }
                    continue;
                }
                if u.definition == "$CURRENT_MIRROR" && p.version == "0.3" {
                    let ref_role = u.roles.iter().find(|r| r.role == "ref");
                    let out_role = u.roles.iter().find(|r| r.role == "out");
                    if let (Some(ref_role), Some(out_role)) = (ref_role, out_role) {
                        if !ref_role.members.is_empty() && !out_role.members.is_empty() {
                            let mut polarities = ref_role.members.iter().chain(&out_role.members).map(|m| match m {
                                Filler::Object(i) => device_polarity(p, *i),
                                _ => None,
                            });
                            if let Some(Some(is_n)) = polarities.next() {
                                if polarities.all(|pol| pol == Some(is_n)) {
                                    let target_def = if is_n { "CURRENT_MIRROR_N" } else { "CURRENT_MIRROR_P" };
                                    out.mirrors.insert(
                                        key,
                                        (
                                            target_def,
                                            "All ref and out devices share one resolved model polarity. The target definition adds low-priority ALIGN y, orientation relations, ROUTE-OBJECT and model/topology assertions (16.3).",
                                        ),
                                    );
                                }
                            }
                        }
                    }
                    continue;
                }
                let (first_role, second_role) = match u.definition.as_str() {
                    "$DIFFERENTIAL_PAIR" => ("in_p", "in_n"),
                    "$MIRRORED" if p.version == "0.3" => ("a", "b"),
                    _ => continue,
                };
                let Some(fallback) = applications.get(&site).copied() else { continue };
                let (Some(first), Some(second)) = (role_object(u, first_role), role_object(u, second_role)) else {
                    continue;
                };
                if first == second {
                    continue;
                }
                let kind = if first_role == "a" {
                    let endpoints = [first, second]
                        .iter()
                        .all(|&i| matches!(p.objects[i].kind, ObjectKind::Instance | ObjectKind::Usage));
                    let axis = u.roles.iter().find(|r| r.role == "axis");
                    if !endpoints
                        || !axis.is_none_or(|r| match r.members.as_slice() {
                            [] | [Filler::Usage(_)] => true,
                            [Filler::Object(i)] => matches!(
                                p.objects[*i].kind,
                                ObjectKind::Instance | ObjectKind::Point | ObjectKind::Usage
                            ),
                            _ => false,
                        })
                    {
                        continue;
                    }
                    "MIRRORED"
                } else {
                    let tail = u.roles.iter().find(|r| r.role == "tail");
                    if !tail.is_none_or(|r| match r.members.as_slice() {
                        [] | [Filler::Usage(_)] => true,
                        [Filler::Object(i)] => {
                            matches!(p.objects[*i].kind, ObjectKind::Instance | ObjectKind::Usage)
                        }
                        _ => false,
                    }) {
                        continue;
                    }
                    let Some(is_n) = device_polarity(p, first).filter(|k| Some(*k) == device_polarity(p, second))
                    else {
                        continue;
                    };
                    if is_n {
                        "DIFFERENTIAL_PAIR_N"
                    } else {
                        "DIFFERENTIAL_PAIR_P"
                    }
                };
                let Ok(side) = pair_direction(p, first, second, &mirrored) else {
                    out.conflicts.insert(key, true);
                    continue;
                };
                let unordered = (first.min(second), first.max(second));
                let fallback_left = *written_sides.entry(unordered).or_insert(if fallback { first } else { second });
                let pair = Pair {
                    kind,
                    first_role,
                    second_role,
                    first_left: side.unwrap_or(fallback_left == first),
                    evidence: if side.is_some() {
                        "Left/right follows the source mirror order, directed constraint or explicit x positions."
                    } else {
                        "No source evidence determines left/right; used the written argument order of the first application of this pair."
                    },
                };
                if let Reference::Name(name) = &u.reference {
                    pair_names.push(((p.level, name.clone()), key.clone()));
                }
                out.pairs.insert(key, pair);
            }
            out.top_io_ports.extend(io_sites.into_iter().filter_map(|(k, ok)| ok.then_some(k)));
        }
        // A statement read in several drawings gets one edit. Keep its role
        // references unchanged too when those readings cannot share a plan.
        let candidates = std::mem::take(&mut out.pairs);
        for keys in sites.values() {
            let Some(pair) = candidates.get(&keys[0]) else { continue };
            let unique: BTreeSet<_> = keys.iter().collect();
            if unique.len() == keys.len() && keys.iter().all(|k| candidates.get(k) == Some(pair)) {
                out.pairs.extend(keys.iter().map(|k| (k.clone(), pair.clone())));
            }
        }
        for (name, key) in pair_names {
            if let Some(pair) = out.pairs.get(&key) {
                out.pair_roles.insert(name, pair.clone());
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
        let object = format!("%{}", rnl_runtime::problem::written_name(name));
        let mut roles =
            readings.iter().map(|l| self.pair_roles.get(&(*l, object.clone())).and_then(|p| p.role(&part.name)));
        let Some(Some(role)) = roles.next() else { return Ok(None) };
        if roles.any(|other| other != Some(role)) {
            return Ok(None);
        }
        let mut text = object;
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
        enabled: bool,
        report: &mut FileReport,
    ) -> Result<Outcome, String> {
        let mut out = Outcome::default();
        if self.aliases {
            return Ok(out);
        }
        let mut directions = 0;
        let mut converted = 0;
        let single_application = member.segments.iter().filter(|s| application(s).is_some()).count() == 1;
        for (i, s) in member.segments.iter().enumerate() {
            let Some(call) = &s.call else { continue };
            if enabled
                && matches!(call.kw.as_str(), "ORDER" | "DISTRIBUTE" | "DISTANCE")
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
            let Some((name, args)) = application(s) else { continue };
            if !single_application {
                continue;
            }
            let (start, end) = (s.start, s.end);
            if name == "FLOW" {
                if let Some(flow) =
                    Self::at(&self.flows, record, start, end, readings).filter(|f| !f.grouped || enabled)
                {
                    let rule = if flow.grouped { "approximate-flow-order" } else { "restore-flow-order" };
                    let mut flow_edits = Vec::new();
                    if let Some(params) =
                        call.params.as_ref().filter(|p| p.get("direction").is_some() && p.positional.is_empty())
                    {
                        let kept: Vec<String> = params
                            .named
                            .iter()
                            .filter(|n| n.name != "direction")
                            .map(|n| {
                                format!(
                                    "{}={}",
                                    rnl_runtime::problem::written_name(&n.name),
                                    super::rewrite::value(&n.value)
                                )
                            })
                            .collect();
                        let replacement = if kept.is_empty() {
                            format!("@{}", call.kw)
                        } else {
                            format!("@{}({})", call.kw, kept.join(", "))
                        };
                        flow_edits.push(record_edit(
                            source,
                            record,
                            call.start..call.end,
                            replacement,
                            "standard-parameter-removed",
                        )?);
                    }
                    flow_edits.push(record_edit(
                        source,
                        record,
                        member.end..member.end,
                        format!("; {}", flow.order),
                        rule,
                    )?);
                    if flow.grouped {
                        record_approximation(source, record, member, &flow_edits, rule,
                            "Restored FLOW's direction and priority. ORDER now compares every combination of members, where the old order compared group centres.".into(), report)?;
                    }
                    out.edits.extend(flow_edits);
                    out.resolved.insert("standard-definition-body");
                }
                continue;
            }
            if !enabled {
                continue;
            }
            if matches!(name, "IO_PORT" | "IOPORT")
                && !member.segments.iter().any(|seg| seg.call.as_ref().is_some_and(|c| c.kw == "SET-SYMBOL"))
                && !readings.is_empty()
                && readings.iter().all(|reading| {
                    let a = record.loc(start);
                    let b = record.loc(end);
                    self.top_io_ports.iter().any(|(lv, f, ln, col)| {
                        lv == reading
                            && *f == normalize(&a.file)
                            && (*ln, *col) >= (a.line, a.col)
                            && (*ln, *col) <= (b.line, b.col)
                    })
                })
            {
                let definition = if call.kw == "AS" { s.args.last() } else { s.args.first() }.unwrap();
                let edit = record_edit(
                    source,
                    record,
                    definition.start..definition.end,
                    "$NET_LABEL".into(),
                    "approximate-top-level-io-port",
                )?;
                record_approximation(
                    source,
                    record,
                    member,
                    std::slice::from_ref(&edit),
                    "approximate-top-level-io-port",
                    "Top-level non-port IO_PORT replaced by NET_LABEL. NET_LABEL_SYMBOL's pin faces left (opposite to IO_PORT_SYMBOL's right-facing pin), so check any explicit orientation or placement."
                        .into(),
                    report,
                )?;
                out.edits.push(edit);
                continue;
            }
            if let Some((target_def, risk)) = Self::at(&self.mirrors, record, start, end, readings) {
                let definition = if call.kw == "AS" { s.args.last() } else { s.args.first() }.unwrap();
                let edit = record_edit(
                    source,
                    record,
                    definition.start..definition.end,
                    format!("${target_def}"),
                    "approximate-current-mirror",
                )?;
                record_approximation(
                    source,
                    record,
                    member,
                    std::slice::from_ref(&edit),
                    "approximate-current-mirror",
                    (*risk).into(),
                    report,
                )?;
                out.edits.push(edit);
                out.resolved.extend([
                    "standard-definition-body",
                    "standard-definition-roles",
                    "standard-definition-removed",
                    "standard-name-removed",
                ]);
                continue;
            }
            if readings
                .iter()
                .any(|reading| Self::at(&self.conflicts, record, start, end, std::slice::from_ref(reading)).is_some())
            {
                out.resolved.extend([
                    "standard-definition-body",
                    "standard-definition-roles",
                    "standard-definition-removed",
                    "standard-name-removed",
                ]);
                out.review = Some((
                    "pair-direction-conflict",
                    "source mirror orders, directed constraints or explicit x positions disagree about left/right",
                ));
                continue;
            }
            let Some(pair) = Self::at(&self.pairs, record, start, end, readings) else { continue };
            let mirror = pair.kind == "MIRRORED";
            let rule = if mirror { "approximate-mirrored" } else { "approximate-differential-pair" };
            let mut changes = Vec::new();
            if !mirror {
                let definition = if call.kw == "AS" { s.args.last() } else { s.args.first() }.unwrap();
                changes.push(record_edit(
                    source,
                    record,
                    definition.start..definition.end,
                    format!("${}", pair.kind),
                    rule,
                )?);
            }
            let chars = record.text.chars().collect::<Vec<_>>();
            for (index, arg) in args.iter().enumerate() {
                let positional = [pair.first_role, pair.second_role, if mirror { "axis" } else { "tail" }];
                let Some(old) = arg.role.as_deref().or_else(|| positional.get(index).copied()) else { continue };
                let role = pair.role(old).unwrap_or(old);
                if matches!(old, "tail" | "axis") {
                    if let ObjExpr::Set(set) = &arg.obj {
                        if set.members.is_empty() {
                            let mut del_start = arg.start;
                            while del_start > s.start && matches!(chars[del_start - 1], ' ' | '\t') {
                                del_start -= 1;
                            }
                            let range = if del_start < arg.start {
                                del_start..arg.end
                            } else {
                                let mut del_end = arg.end;
                                while del_end < s.end && matches!(chars[del_end], ' ' | '\t') {
                                    del_end += 1;
                                }
                                arg.start..del_end
                            };
                            changes.push(record_edit(source, record, range, String::new(), rule)?);
                            continue;
                        } else if set.members.len() == 1
                            && set.members[0].binds.is_empty()
                            && set.members[0].segments.len() == 1
                            && set.members[0].segments[0].call.is_none()
                            && !set.members[0].segments[0].piped
                            && set.members[0].segments[0].args.len() == 1
                            && set.members[0].segments[0].args[0].role.is_none()
                        {
                            let inner = &set.members[0].segments[0].args[0];
                            let inner_text: String = chars[inner.start..inner.end].iter().collect();
                            changes.push(record_edit(
                                source,
                                record,
                                arg.start..arg.end,
                                format!("{inner_text}({role})"),
                                rule,
                            )?);
                            continue;
                        }
                    }
                }
                let at = if arg.role.is_some() {
                    let Some(offset) = chars[arg.start..arg.end].iter().rposition(|c| *c == '(') else { continue };
                    arg.start + offset
                } else {
                    arg.end
                };
                changes.push(record_edit(source, record, at..arg.end, format!("({role})"), rule)?);
            }
            let risk = if mirror {
                "The singleton axis keeps its point. The directed distribution changes minimum spacing and conflict grouping."
            } else {
                "Both input devices have the same resolved model polarity. The new definition adds directed distribution/order, routing, grouping, pin orientation and assertions."
            };
            record_approximation(source, record, member, &changes, rule, format!("{risk} {}", pair.evidence), report)?;
            out.edits.extend(changes);
            out.resolved.extend([
                "standard-definition-body",
                "standard-definition-roles",
                "standard-definition-removed",
                "standard-name-removed",
            ]);
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
        if enabled
            && !constraints.is_empty()
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
