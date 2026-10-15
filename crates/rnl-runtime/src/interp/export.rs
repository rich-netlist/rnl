//! From scenes to the solver's view of each level (crate::problem), taken before the interpreter
//! is dropped.

use std::collections::{HashMap, HashSet};

use crate::geom::Orient;
use crate::host::LevelKind;
use crate::problem::*;

use super::obj::*;
use super::scene::FactData;
use super::Interp;

/// The indexes of a problem's lists by the interpreter's objects, and what the objects are
/// referred to by. Only looked up, never gone through, so the problem's order is the scene's.
#[derive(Default)]
struct Index {
    objects: HashMap<ObjId, usize>,
    usages: HashMap<ObjId, usize>,
    wires: HashMap<ObjId, usize>,
    refs: HashMap<ObjId, Reference>,
}

/// A fact before it is put in the order of 8.5: priority, position, and what breaks a tie within
/// one position (the kind of fact, then x before y), then the order the facts were collected in.
struct Ranked {
    key: (u8, Pos, u8, u8, usize),
    fact: ProblemFact,
}

impl Interp<'_> {
    pub(crate) fn build_problems(&mut self) -> Vec<LevelProblem> {
        // building reads the scenes and reports nothing: what it would add to the diagnostics, the
        // references found and the dead text is dropped
        let sink = self.sink.clone();
        let (nrefs, nfailed) = (self.refs.len(), self.failed_spans.len());
        let out = (0..self.scenes.len()).map(|s| self.level_problem(s)).collect();
        self.sink = sink;
        self.refs.truncate(nrefs);
        self.failed_spans.truncate(nfailed);
        out
    }

    fn level_problem(&mut self, s: SceneId) -> LevelProblem {
        let lv = self.scenes[s].level;
        let level = self.doc.levels[lv].clone();
        let packages = self
            .effective_packages(lv)
            .iter()
            .map(|e| (self.units[e.unit].pkg_name.clone(), self.units[e.unit].pkg_version.clone()))
            .collect();
        let mut ix = Index::default();

        // the objects, kind by kind, each in the order it was made
        let mut objs: Vec<(ObjId, ObjectKind)> = Vec::new();
        for o in self.inst_like(s) {
            if self.bundle_empty(o) {
                continue;
            }
            let kind =
                if matches!(self.o(o), ObjData::Bundle { .. }) { ObjectKind::Bundle } else { ObjectKind::Instance };
            objs.push((o, kind));
        }
        let mut usages: Vec<ObjId> =
            self.scenes[s].usages.iter().copied().filter(|a| !self.usage(*a).in_symbol).collect();
        usages.sort();
        usages.dedup();
        for &a in &usages {
            if self.scenes[s].symbol.contains_key(&a) {
                objs.push((a, ObjectKind::Usage));
            }
        }
        let mut points = self.scenes[s].points.clone();
        points.sort();
        points.dedup();
        objs.extend(points.into_iter().map(|p| (p, ObjectKind::Point)));
        let mut texts: Vec<ObjId> = self.scenes[s].texts.iter().copied().filter(|t| !self.is_obj_failed(*t)).collect();
        texts.sort();
        texts.dedup();
        objs.extend(texts.into_iter().map(|t| (t, ObjectKind::Text)));
        for (k, (o, _)) in objs.iter().enumerate() {
            ix.objects.insert(*o, k);
        }
        for (k, a) in usages.iter().enumerate() {
            ix.usages.insert(*a, k);
        }
        let mut wires = Vec::new();
        for w in self.scenes[s].wires.clone() {
            if !self.is_obj_failed(w) && !ix.wires.contains_key(&w) {
                ix.wires.insert(w, wires.len());
                wires.push(w);
            }
        }

        // what each object is referred to by (5.7, 10.5, 13.3)
        let names = self.level_names(s);
        for (name, o) in &names {
            ix.refs.entry(*o).or_insert_with(|| Reference::Name(format!("%{}", written_name(name))));
        }
        for &a in &usages {
            let u = self.usage(a).clone();
            let r = match ix.refs.get(&a) {
                Some(r) => r.clone(),
                None if self.rule_made(a) => Reference::Generated { usage: ix.usages[&a], path: Vec::new() },
                None => Reference::Anonymous,
            };
            ix.refs.insert(a, r.clone());
            for (name, o) in &u.locals {
                ix.refs.entry(*o).or_insert_with(|| r.then(&format!("::{}", written_name(name))));
            }
        }

        // symbols, in the order the objects first use them
        let mut symbols: Vec<ProblemSymbol> = Vec::new();
        let mut symbol_index: HashMap<ObjId, usize> = HashMap::new();
        let mut object_symbol: Vec<Option<usize>> = Vec::new();
        for (o, _) in &objs {
            let Some(&sym) = self.scenes[s].symbol.get(o) else {
                object_symbol.push(None);
                continue;
            };
            let k = match symbol_index.get(&sym) {
                Some(&k) => k,
                None => {
                    let k = symbols.len();
                    symbol_index.insert(sym, k);
                    symbols.push(self.problem_symbol(sym));
                    k
                }
            };
            object_symbol.push(Some(k));
        }

        let facts = self.problem_facts(s, &ix, &objs);

        // the first coordinate fact of each object and axis, and the first ORIENT, in the order of 8.5
        let mut first_coord: HashMap<(usize, Axis), (f64, usize)> = HashMap::new();
        let mut first_orient: HashMap<usize, (Orient, usize)> = HashMap::new();
        for (i, f) in facts.iter().enumerate() {
            match f.kind {
                FactKind::Place { at: PointRef::Origin(k), axis, value }
                | FactKind::PointCoordinate { point: k, axis, value } => {
                    first_coord.entry((k, axis)).or_insert((value, i));
                }
                FactKind::Orient { object, code } => {
                    first_orient.entry(object).or_insert((code, i));
                }
                _ => {}
            }
        }

        // the objects themselves
        let mut objects = Vec::new();
        for (k, (o, kind)) in objs.iter().enumerate() {
            let slot = |axis: Axis| match first_coord.get(&(k, axis)) {
                Some(&(value, fact)) => Slot::Fixed { value, fact },
                None => Slot::Free,
            };
            let own = Position::Own { x: slot(Axis::X), y: slot(Axis::Y) };
            let orient = match kind {
                ObjectKind::Point | ObjectKind::Text => None,
                _ => Some(match first_orient.get(&k) {
                    Some(&(code, fact)) => OrientSlot::Fixed { code, fact },
                    None => OrientSlot::Free,
                }),
            };
            let mut position = own.clone();
            let mut text = None;
            let mut usage = None;
            let reference = match kind {
                ObjectKind::Instance => Reference::Name(format!("#{}", written_name(&self.inst_of(*o).name))),
                ObjectKind::Text => self.text_reference(s, *o, &ix),
                _ => ix.refs.get(o).cloned().unwrap_or(Reference::Anonymous),
            };
            match kind {
                ObjectKind::Usage => {
                    usage = ix.usages.get(o).copied();
                    if self.at_point(*o) {
                        if let Some(m) = self.main_of(*o) {
                            if let Some(r) = self.point_ref(s, &ix, m) {
                                position = Position::At(r);
                            }
                        }
                    }
                }
                ObjectKind::Text => {
                    let att = self.text_attach(*o);
                    let items = self.text_items(*o);
                    let drawable =
                        matches!(self.o(att), ObjData::Inst { .. } | ObjData::Usage(_) | ObjData::Bundle { .. });
                    if let (Some(local), true, Some(&owner)) = (self.text_sym_pos(*o), drawable, ix.objects.get(&att)) {
                        let both = matches!(&own, Position::Own { x: Slot::Fixed { .. }, y: Slot::Fixed { .. } });
                        if !both {
                            position = Position::InSymbol { owner, local };
                        }
                    }
                    let lines = match &items.text {
                        Some(t) => crate::markup::parse(&self.expand_text(s, t, att)),
                        None => Vec::new(),
                    };
                    let style = self.scenes[s]
                        .style
                        .get(o)
                        .map(|m| {
                            m.iter()
                                .filter(|(k, _)| matches!(k.as_str(), "size" | "font" | "bold" | "italic"))
                                .map(|(k, (v, _))| (k.clone(), v.clone()))
                                .collect()
                        })
                        .unwrap_or_default();
                    let key = match self.o(*o) {
                        ObjData::KeyText { key, .. } => Some(key.clone()),
                        _ => None,
                    };
                    text = Some(TextInfo {
                        attach: self.filler(s, &ix, att),
                        key,
                        lines,
                        halign: items.halign.clone().unwrap_or_else(|| "start".into()),
                        valign: items.valign.clone().unwrap_or_else(|| "baseline".into()),
                        rotate: items.rotate.unwrap_or(0.0),
                        style,
                    });
                }
                _ => {}
            }
            objects.push(ProblemObject {
                kind: *kind,
                label: self.label(*o),
                reference,
                generated: self.made_by_rule(s, *o),
                symbol: object_symbol[k],
                position,
                orient,
                usage,
                text,
            });
        }

        // wires (13.1)
        let mut problem_wires = Vec::new();
        for &w in &wires {
            let ObjData::Wire { nets, bus, points, mode, generated, .. } = self.o(w).clone() else { continue };
            let anchors: Vec<PointRef> = points.iter().filter_map(|q| self.point_ref(s, &ix, *q)).collect();
            problem_wires.push(ProblemWire {
                label: self.label(w),
                reference: ix.refs.get(&w).cloned().unwrap_or(Reference::Anonymous),
                nets: nets.iter().map(|n| self.net_display(*n)).collect(),
                bus,
                anchors,
                mode,
                generated,
            });
        }

        // ROUTE (13.7): the pins of the net, and the wires already on it
        let mut routes = Vec::new();
        let route_objs = self.scenes[s].routes.clone();
        if !route_objs.is_empty() {
            let terms = self.route_terms(s);
            for r in route_objs {
                let ObjData::Route { net, mode, style } = self.o(r).clone() else { continue };
                let mut pins = Vec::new();
                let mut seen: HashSet<(ObjId, u32)> = HashSet::new();
                for &(ow, t, i) in terms.get(&net).into_iter().flatten() {
                    if self.has_pin(s, t) && seen.insert((ow, i)) {
                        if let Some(p) = self.point_ref(s, &ix, t) {
                            pins.push(p);
                        }
                    }
                }
                let on_net = self.scenes[s].wires_by_net.get(&net).cloned().unwrap_or_default();
                routes.push(RouteIntent {
                    net: self.net_display(net),
                    mode,
                    style: style.into_iter().collect(),
                    pins,
                    wires: on_net.iter().filter_map(|w| ix.wires.get(w).copied()).collect(),
                });
            }
        }

        // GROUP (11.5), without those PRIORITY turns off
        let mut groups = Vec::new();
        for (_, set, chain) in self.scenes[s].groups.clone() {
            if self.eff_priority(s, None, &chain, Some(set)) == Priority::Disabled {
                continue;
            }
            let members = self.flatten(s, set, 7, None);
            groups.push(ProblemGroup {
                label: self.label(set),
                members: members.iter().filter_map(|m| ix.objects.get(m).copied()).collect(),
            });
        }

        // usages (9.4, 12.1)
        let mut entries = Vec::new();
        for &a in &usages {
            let u = self.usage(a).clone();
            let mut roles = Vec::new();
            for (role, filler) in &u.roles {
                let members = if self.is_set(*filler) { self.flatten(s, *filler, 7, None) } else { vec![*filler] };
                roles.push(RoleFill {
                    role: role.clone(),
                    members: members.iter().map(|m| self.filler(s, &ix, *m)).collect(),
                    written: self.written_filler(s, &ix, *filler),
                });
            }
            entries.push(UsageEntry {
                definition: self.label(u.definition),
                label: self.label(a),
                reference: ix.refs.get(&a).cloned().unwrap_or(Reference::Anonymous),
                generated: u.generated,
                rule_made: self.rule_made(a),
                roles,
                params: u.params.iter().filter_map(|(n, v)| v.clone().map(|v| (n.clone(), v))).collect(),
                priority: u.priority,
                parent: u.parent.and_then(|p| ix.usages.get(&p).copied()),
                object: ix.objects.get(&a).copied(),
            });
        }

        LevelProblem {
            level: lv,
            title: level.title(),
            kind: if level.kind == LevelKind::Subckt { "subckt" } else { "file" },
            name: level.name.clone(),
            file: level.file.clone(),
            line: level.line,
            end_line: level.end_line,
            packages,
            names: names.into_iter().map(|(n, _)| n).collect(),
            objects,
            symbols,
            facts,
            wires: problem_wires,
            routes,
            groups,
            usages: entries,
        }
    }

    /// The `%` names the level binds, those of the records written into it with scope= among them
    /// (2.6, 6.7), with their objects in this scene, in the order they are declared; a name bound
    /// twice keeps its first binding (6.6).
    fn level_names(&self, s: SceneId) -> Vec<(String, ObjId)> {
        let u = self.level_unit[self.scenes[s].level];
        let mut out = Vec::new();
        for key in &self.units[u].name_order {
            if key.0 != '%' {
                continue;
            }
            let b = &self.units[u].names[key];
            let mkey = b.key();
            if let Some(super::Memo::Val(v)) = self.memo.get(&(mkey.clone(), Some(s))) {
                let o = self.bound_value(&mkey, Some(s), '%', &b.name, *v);
                if o != NOTHING && o != LEVEL && o != OWNER && !self.is_failed(o) {
                    out.push((b.name.clone(), o));
                }
            }
        }
        out
    }

    /// A usage made by a generation rule itself (10.5), not in the expansion of another
    /// generated usage.
    fn rule_made(&self, a: ObjId) -> bool {
        let u = self.usage(a);
        u.generated && u.parent.is_none_or(|p| !self.usage(p).generated)
    }

    /// Made by a rule (10.5): a generated usage, wire or text, or any object made in a generation,
    /// whose position carries the mark of the generation rule.
    fn made_by_rule(&self, s: SceneId, o: ObjId) -> bool {
        self.is_generated(o) || self.scenes[s].created.get(&o).is_some_and(|p| p.contains(&PosElem::Tag("g")))
    }

    /// A text with a key is the sub-object `object::key` (13.3); one without is referred to by
    /// its name only.
    fn text_reference(&mut self, s: SceneId, t: ObjId, ix: &Index) -> Reference {
        match self.o(t).clone() {
            ObjData::KeyText { attach, key, .. } => {
                self.reference_of(s, ix, attach).then(&format!("::{}", written_name(&key)))
            }
            _ => ix.refs.get(&t).cloned().unwrap_or(Reference::Anonymous),
        }
    }

    /// What any object of the scene is referred to by: host objects by their host names, the
    /// rest as `Index::refs` has them.
    fn reference_of(&mut self, s: SceneId, ix: &Index, o: ObjId) -> Reference {
        match self.o(o).clone() {
            ObjData::Inst { .. } => Reference::Name(format!("#{}", written_name(&self.inst_of(o).name))),
            ObjData::LevelObj { .. } => Reference::Name("#".into()),
            ObjData::Net { display, .. } => Reference::Name(format!("&{}", written_name(&display))),
            ObjData::Term { owner, index, name } => {
                let t = name.unwrap_or_else(|| index.to_string());
                let base = self.reference_of(s, ix, owner);
                match base {
                    Reference::Name(n) if n == "#" => Reference::Name(format!("#:{}", written_name(&t))),
                    b => b.then(&format!(":{}", written_name(&t))),
                }
            }
            ObjData::Pos { parent } => self.reference_of(s, ix, parent).then("::position"),
            ObjData::KeyText { .. } => self.text_reference(s, o, ix),
            _ => ix.refs.get(&o).cloned().unwrap_or(Reference::Anonymous),
        }
    }

    /// How the level's text writes what fills a role: a name or handle, a point with both
    /// coordinates written, or a set of them in `{…}`. None when a part cannot be referred to
    /// by a name the level writes.
    fn written_filler(&mut self, s: SceneId, ix: &Index, o: ObjId) -> Option<String> {
        if let ObjData::Set(_) = self.o(o) {
            let items = self.items_of(s, o, 7, None);
            let mut out = Vec::new();
            for m in items {
                out.push(self.written_filler(s, ix, m)?);
            }
            return Some(format!("{{{}}}", out.join(" ")));
        }
        match self.reference_of(s, ix, o) {
            Reference::Name(n) => Some(n),
            _ => match self.o(o) {
                ObjData::Point { x: Some(x), y: Some(y), .. } => {
                    Some(format!("({}, {})", crate::geom::fmt_num(*x), crate::geom::fmt_num(*y)))
                }
                _ => None,
            },
        }
    }

    /// What fills a role, or what a text is attached to, as the problem refers to it.
    fn filler(&mut self, s: SceneId, ix: &Index, o: ObjId) -> Filler {
        if let Some(&k) = ix.objects.get(&o) {
            return Filler::Object(k);
        }
        match self.o(o).clone() {
            ObjData::Inst { bundle: Some(b), .. } if ix.objects.contains_key(&b) => Filler::Object(ix.objects[&b]),
            ObjData::Term { owner, index, name } => {
                let owner = self.bundle_field(owner).unwrap_or(owner);
                match ix.objects.get(&owner) {
                    Some(&object) => {
                        let t = self.term_obj(s, owner, index, name.clone());
                        let pin = self.pin_for(s, owner, t);
                        Filler::Terminal { object, terminal: name.unwrap_or_else(|| index.to_string()), pin }
                    }
                    None => Filler::Other(self.label(o)),
                }
            }
            ObjData::Usage(_) if ix.usages.contains_key(&o) => Filler::Usage(ix.usages[&o]),
            ObjData::Net { .. } => Filler::Net(self.net_display(o)),
            ObjData::Wire { .. } if ix.wires.contains_key(&o) => Filler::Wire(ix.wires[&o]),
            _ => Filler::Other(self.label(o)),
        }
    }

    /// The point an object counts by (14.1): an origin, the pin of a terminal, or the centre of
    /// a set. None for what has no point, or is not in the problem.
    fn point_ref(&mut self, s: SceneId, ix: &Index, o: ObjId) -> Option<PointRef> {
        match self.o(o).clone() {
            ObjData::Inst { bundle: Some(b), .. } => Some(PointRef::Origin(*ix.objects.get(&b)?)),
            ObjData::Inst { .. }
            | ObjData::Bundle { .. }
            | ObjData::Usage(_)
            | ObjData::Point { .. }
            | ObjData::Text { .. }
            | ObjData::KeyText { .. } => Some(PointRef::Origin(*ix.objects.get(&o)?)),
            ObjData::Term { owner, index, name } => {
                let owner = self.bundle_field(owner).unwrap_or(owner);
                let object = *ix.objects.get(&owner)?;
                let t = self.term_obj(s, owner, index, name.clone());
                let pin = self.pin_for(s, owner, t)?;
                Some(PointRef::Pin { object, pin, terminal: name.unwrap_or_else(|| index.to_string()) })
            }
            ObjData::Pos { parent } => self.point_ref(s, ix, parent),
            ObjData::Set(_) => {
                let leaves = self.flatten(s, o, 7, None);
                let mut points = Vec::new();
                for m in leaves {
                    points.push(self.point_ref(s, ix, m)?);
                }
                Some(PointRef::Centre(points))
            }
            _ => None,
        }
    }

    /// The points of a member of DISTANCE, whose extent counts (14.5): a set's members, nested
    /// sets followed down, or the one point of an object.
    fn extent_points(&mut self, s: SceneId, ix: &Index, o: ObjId) -> Option<Vec<PointRef>> {
        if self.is_set(o) {
            let leaves = self.flatten(s, o, 7, None);
            let mut out = Vec::new();
            for m in leaves {
                out.push(self.point_ref(s, ix, m)?);
            }
            return Some(out);
        }
        Some(vec![self.point_ref(s, ix, o)?])
    }

    fn problem_symbol(&mut self, sym: ObjId) -> ProblemSymbol {
        let sd = self.symbol(sym).clone();
        ProblemSymbol {
            name: self.label(sym),
            bbox: sd.bbox.unwrap_or_else(|| self.symbol_extent(sym, 0)),
            box_written: sd.bbox.is_some(),
            default: sd.default,
            pins: sd
                .pins
                .iter()
                .map(|p| crate::model::PinView {
                    pos: p.pos,
                    label: p.label.clone(),
                    dir: p.dir,
                    terminals: p
                        .terms
                        .iter()
                        .map(|t| match t {
                            TermKey::Num(n) => n.to_string(),
                            TermKey::Name(s) => s.clone(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// The usages a fact was expanded from, as indexes of the problem's usages.
    fn chain_of(ix: &Index, chain: &[ObjId]) -> Vec<usize> {
        chain.iter().filter_map(|a| ix.usages.get(a).copied()).collect()
    }

    /// The geometric facts of a scene in the order of resolution of 8.5.
    fn problem_facts(&mut self, s: SceneId, ix: &Index, objs: &[(ObjId, ObjectKind)]) -> Vec<ProblemFact> {
        let mut ranked: Vec<Ranked> = Vec::new();
        let rank = |p: Priority| p as u8;
        // the texts whose offset on an axis replaces their PLACE on that axis, or the reverse (13.3)
        let mut offset_wins: HashMap<(ObjId, char), bool> = HashMap::new();
        for (o, kind) in objs {
            if *kind != ObjectKind::Text || self.text_sym_pos(*o).is_some() {
                continue;
            }
            let items = self.text_items(*o);
            for (axis, d) in [('x', items.dx), ('y', items.dy)] {
                if d.is_none() {
                    continue;
                }
                let sc = &self.scenes[s];
                let latest_place: Option<Pos> = sc
                    .at_by_target
                    .get(o)
                    .into_iter()
                    .flatten()
                    .map(|&j| &sc.at[j])
                    .filter(|f| {
                        matches!(f.data, FactData::Coord(a, _) if a == axis)
                            && self.fact_priority(s, f) != Priority::Disabled
                    })
                    .map(|f| f.pos.clone())
                    .max();
                let off = self.offset_pos(s, *o, axis == 'x');
                offset_wins.insert((*o, axis), latest_place.is_none_or(|p| off > p));
            }
        }

        // PLACE and the coordinates of points (11.1, 13.1)
        let at = self.scenes[s].at.clone();
        for k in self.scenes[s].at_live.clone() {
            let f = &at[k];
            let p = self.fact_priority(s, f);
            if p == Priority::Disabled {
                continue;
            }
            let FactData::Coord(axis, value) = f.data else { continue };
            if offset_wins.get(&(f.target, axis)) == Some(&true) {
                continue;
            }
            let point = f.ctx.is_none() && matches!(self.o(f.target), ObjData::Point { .. });
            let kind = if point {
                let Some(&point) = ix.objects.get(&f.target) else { continue };
                FactKind::PointCoordinate { point, axis: Axis::of(axis), value }
            } else {
                let Some(at) = self.point_ref(s, ix, f.target) else { continue };
                FactKind::Place { at, axis: Axis::of(axis), value }
            };
            let label = format!("the {axis} of {} = {}", self.label(f.target), crate::geom::fmt_num(value));
            let source = f.ctx.as_ref().and_then(|c| self.rec_loc(c.rec, f.off));
            ranked.push(Ranked {
                key: (rank(p), f.pos.clone(), 0, (axis == 'y') as u8, ranked.len()),
                fact: ProblemFact {
                    kind,
                    priority: p,
                    label,
                    reference: None,
                    source,
                    chain: Self::chain_of(ix, &f.chain),
                },
            });
        }

        // the offsets of texts (13.3)
        for (o, kind) in objs {
            if *kind != ObjectKind::Text {
                continue;
            }
            let items = self.text_items(*o);
            let (site, att) = match self.o(*o) {
                ObjData::Text { site, attach, .. } | ObjData::KeyText { site, attach, .. } => (site.clone(), *attach),
                _ => continue,
            };
            for (axis, d) in [('x', items.dx), ('y', items.dy)] {
                let Some(value) = d else { continue };
                if offset_wins.get(&(*o, axis)) != Some(&true) {
                    continue;
                }
                let Some(from) = self.point_ref(s, ix, att) else { continue };
                let chain = site.as_ref().map(|c| self.apply_chain(c)).unwrap_or_default();
                let head = site.as_ref().and_then(|c| c.header.priority);
                let p = self.eff_priority(s, head, &chain, None);
                if p == Priority::Disabled {
                    continue;
                }
                let source = site.as_ref().and_then(|c| self.rec_loc(c.rec, self.text_off(*o)));
                let label = format!("the d{axis} of {} = {}", self.label(*o), crate::geom::fmt_num(value));
                ranked.push(Ranked {
                    key: (rank(p), self.offset_pos(s, *o, axis == 'x'), 1, (axis == 'y') as u8, ranked.len()),
                    fact: ProblemFact {
                        kind: FactKind::Offset { text: ix.objects[o], from, axis: Axis::of(axis), value },
                        priority: p,
                        label,
                        reference: None,
                        source,
                        chain: Self::chain_of(ix, &chain),
                    },
                });
            }
        }

        // ORIENT (11.2)
        let orient = self.scenes[s].orient.clone();
        for k in self.scenes[s].orient_live.clone() {
            let f = &orient[k];
            let p = self.fact_priority(s, f);
            if p == Priority::Disabled {
                continue;
            }
            let FactData::Orient(code) = f.data else { continue };
            let Some(&object) = ix.objects.get(&f.target) else { continue };
            let source = f.ctx.as_ref().and_then(|c| self.rec_loc(c.rec, f.off));
            ranked.push(Ranked {
                key: (rank(p), f.pos.clone(), 2, 0, ranked.len()),
                fact: ProblemFact {
                    kind: FactKind::Orient { object, code },
                    priority: p,
                    label: format!("the orientation {} of {}", code.as_str(), self.label(f.target)),
                    reference: None,
                    source,
                    chain: Self::chain_of(ix, &f.chain),
                },
            });
        }

        // the atomic constraints and ORIENT-RELATION (14)
        for c in self.scenes[s].constraints.clone() {
            let ObjData::Constraint(cd) = self.o(c).clone() else { continue };
            if cd.failed {
                continue;
            }
            let p = cd.eff.unwrap_or_else(|| self.eff_priority(s, cd.priority_param, &cd.chain, Some(c)));
            if p == Priority::Disabled {
                continue;
            }
            let Some(kind) = self.constraint_kind(s, ix, &cd) else { continue };
            let source = cd.site.as_ref().and_then(|ctx| self.rec_loc(ctx.rec, cd.off));
            let reference = match ix.refs.get(&c) {
                Some(Reference::Anonymous) | None => None,
                Some(r) => Some(r.clone()),
            };
            let pos = self.scenes[s].created.get(&c).cloned().unwrap_or_default();
            ranked.push(Ranked {
                key: (rank(p), pos, 3, 0, ranked.len()),
                fact: ProblemFact {
                    kind,
                    priority: p,
                    label: self.label(c),
                    reference,
                    source,
                    chain: Self::chain_of(ix, &cd.chain),
                },
            });
        }

        // higher priority first, then the later first (8.5)
        ranked.sort_by(|a, b| b.key.cmp(&a.key));
        ranked.into_iter().map(|r| r.fact).collect()
    }

    fn text_off(&self, t: ObjId) -> usize {
        match self.o(t) {
            ObjData::Text { off, .. } | ObjData::KeyText { off, .. } => *off,
            _ => 0,
        }
    }

    /// The members of a constraint as the problem refers to them, as the check of the written
    /// coordinates takes them (14.1).
    fn constraint_kind(&mut self, s: SceneId, ix: &Index, cd: &ConstraintData) -> Option<FactKind> {
        let axis = Axis::of(cd.params.axis.unwrap_or('x'));
        let sense = Sense::of(cd.params.sense);
        let members = |it: &mut Self| -> Option<Vec<PointRef>> {
            let ms = if it.is_set(cd.args[0]) { it.items_of(s, cd.args[0], 7, None) } else { vec![cd.args[0]] };
            let mut out = Vec::new();
            for m in ms {
                out.push(it.point_ref(s, ix, m)?);
            }
            Some(out)
        };
        Some(match cd.kind {
            ConstraintKind::Align => FactKind::Align { axis, members: members(self)? },
            ConstraintKind::Order => FactKind::Order { axis, sense, members: members(self)? },
            ConstraintKind::Distribute => FactKind::Distribute { axis, members: members(self)? },
            ConstraintKind::Distance => FactKind::Distance {
                axis,
                sense,
                min: cd.params.min,
                max: cd.params.max,
                from: self.extent_points(s, ix, cd.args[0])?,
                to: self.extent_points(s, ix, cd.args[1])?,
            },
            ConstraintKind::OrientRelation => {
                let mut sides = Vec::new();
                for a in &cd.args[..2] {
                    let ms = if self.is_set(*a) { self.flatten(s, *a, 7, None) } else { vec![*a] };
                    let mut side = Vec::new();
                    for m in ms {
                        let m = self.bundle_field(m).unwrap_or(m);
                        side.push(*ix.objects.get(&m)?);
                    }
                    sides.push(side);
                }
                let to = sides.pop()?;
                let from = sides.pop()?;
                FactKind::OrientRelation { relation: cd.params.relation.unwrap_or(Orient::R0), from, to }
            }
        })
    }
}
