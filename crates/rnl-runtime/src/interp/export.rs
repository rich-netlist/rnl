//! From scenes to the solver's view of each level (crate::problem), taken before the interpreter
//! is dropped.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::geom::Orient;
use crate::host::LevelKind;
use crate::previous::{Given, Key, RecordResult};
use crate::problem::*;

use super::ctx::Ctx;
use super::obj::*;
use super::scene::{FactData, WireShape};
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

/// A fact before it is put in the order of 8.5: priority, position in the order of resolution (of
/// the segments of one pipe the later is later), and what breaks a tie within one position (the
/// kind of fact, then x before y), then the order the facts were collected in.
struct Ranked {
    key: (u8, Pos, u8, u8, usize),
    fact: ProblemFact,
}

impl Interp<'_> {
    /// The problem of each drawing level and, read with the records of the solver, what the last of
    /// each level gives ([`crate::previous`]).
    pub(crate) fn build_problems(&mut self) -> (Vec<LevelProblem>, Vec<RecordResult>) {
        // building reads the scenes and reports nothing: what it would add to the diagnostics, the
        // references found and the dead text is dropped
        let sink = self.sink.clone();
        let (nrefs, nfailed) = (self.refs.len(), self.failed_spans.len());
        let mut problems = Vec::new();
        let mut results = Vec::new();
        for s in 0..self.scenes.len() {
            let (p, result) = self.level_problem(s);
            problems.push(p);
            results.extend(result);
        }
        self.sink = sink;
        self.refs.truncate(nrefs);
        self.failed_spans.truncate(nfailed);
        (problems, results)
    }

    fn level_problem(&mut self, s: SceneId) -> (LevelProblem, Option<RecordResult>) {
        let lv = self.scenes[s].level;
        let level = self.doc.levels[lv].clone();
        let packages = self
            .effective_packages(lv)
            .entries
            .iter()
            .map(|e| (self.units[e.unit].pkg_name.clone(), self.units[e.unit].pkg_version.clone()))
            .collect();
        let mut ix = Index::default();

        let (mut objs, usages, annotations) = self.problem_objects(s);
        for (k, (o, _)) in objs.iter().enumerate() {
            ix.objects.insert(*o, k);
        }
        for (k, a) in usages.iter().enumerate() {
            ix.usages.insert(*a, k);
        }
        let mut wires = Vec::new();
        for w in self.scenes[s].wires.clone() {
            if !self.is_obj_failed(w) && !self.drawing_ignored(s, w) && !ix.wires.contains_key(&w) {
                ix.wires.insert(w, wires.len());
                wires.push(w);
            }
        }

        // what each object is referred to by: a name, else a number (5.7, 10.5, 13.3; the draft
        // 0.3, 5.9); through `::` the local objects of a usage that has either
        let names = self.level_names(s);
        for (name, o) in &names {
            ix.refs.entry(*o).or_insert_with(|| Reference::Name(format!("%{}", written_name(name))));
        }
        // An independent anchor is addressable through a named annotation, not an invented name.
        for (name, object) in &names {
            if let ObjData::Annotation(data) = self.o(*object) {
                if let AnnotationAnchor::Independent(point) = data.anchor {
                    ix.refs
                        .entry(point)
                        .or_insert_with(|| Reference::Name(format!("%{}::position", written_name(name))));
                }
            }
        }
        let mut numbers: Vec<(ObjId, usize)> = self.problem_numbers(s).into_iter().collect();
        numbers.sort();
        let by_number: HashMap<usize, ObjId> = numbers.iter().map(|&(o, n)| (n, o)).collect();
        for (o, number) in numbers {
            ix.refs.entry(o).or_insert(Reference::Number { number, path: Vec::new() });
        }
        // the objects without a name of a usage a rule makes, by their place among the objects
        // that the statement writing the usage out makes (5.9, 10.5)
        let bodies = self.generated_bodies(s);
        for &a in &usages {
            for (k, o) in bodies.get(&a).into_iter().flatten().enumerate() {
                let r = Reference::GeneratedNumber { usage: ix.usages[&a], index: k + 1, path: Vec::new() };
                ix.refs.entry(*o).or_insert(r);
            }
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

        // Resolve annotation anchors after the real point/owner indices exist. A failure stays
        // explicit in AnnotationInfo and does not manufacture an independently placeable point.
        let mut annotation_anchors = HashMap::new();
        let mut annotation_templates = Vec::new();
        for id in annotations {
            let ObjData::Annotation(data) = self.o(id).clone() else { continue };
            let anchor = match data.anchor {
                AnnotationAnchor::Independent(o) | AnnotationAnchor::Follow(o) => o,
            };
            let point = self.point_ref(s, &ix, anchor);
            let Some(view) = self.annotation_template_view(s, id) else { continue };
            let object = if let Some(point) = point {
                let index = objs.len();
                ix.objects.insert(id, index);
                objs.push((id, ObjectKind::Annotation));
                annotation_anchors.insert(id, point);
                Some(index)
            } else {
                None
            };
            annotation_templates.push(AnnotationInfo { object, view });
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

        let (facts, orient_lists) = self.problem_facts(s, &ix, &objs);

        // the first coordinate fact of each object and axis, and the first ORIENT, in the order of 8.5;
        // the axes of the texts that an offset places (13.3)
        let mut first_coord: HashMap<(usize, Axis), (f64, usize)> = HashMap::new();
        let mut first_orient: HashMap<usize, (Orient, usize)> = HashMap::new();
        let mut offset: HashSet<(usize, Axis)> = HashSet::new();
        for (i, f) in facts.iter().enumerate() {
            match f.kind {
                FactKind::Place { at: PointRef::Origin(k), axis, value }
                | FactKind::PointCoordinate { point: k, axis, value } => {
                    first_coord.entry((k, axis)).or_insert((value, i));
                }
                FactKind::Orient { object, code } => {
                    first_orient.entry(object).or_insert((code, i));
                }
                FactKind::Offset { text, axis, .. } => {
                    offset.insert((text, axis));
                }
                _ => {}
            }
        }

        // the objects themselves
        // A relative PLACE also replaces a label's symbol coordinate on that axis (13.3),
        // although it is represented by a DISTANCE rather than an absolute fixed slot.
        for &k in &self.scenes[s].at_live {
            let f = &self.scenes[s].at[k];
            if let FactData::RelativeCoord(axis, _, _) = f.data {
                if self.fact_priority(s, f) != Priority::Disabled {
                    if let Some(&object) = ix.objects.get(&f.target) {
                        offset.insert((object, Axis::of(axis)));
                    }
                }
            }
        }
        let mut objects = Vec::new();
        for (k, (o, kind)) in objs.iter().enumerate() {
            let slot = |axis: Axis| match first_coord.get(&(k, axis)) {
                Some(&(value, fact)) => Slot::Fixed { value, fact },
                None => Slot::Free,
            };
            let own = Position::Own { x: slot(Axis::X), y: slot(Axis::Y) };
            let orient = match kind {
                ObjectKind::Point | ObjectKind::Text | ObjectKind::Annotation => None,
                ObjectKind::Usage if object_symbol[k].is_none() => None,
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
                ObjectKind::Annotation => {
                    // Only successfully mapped annotations were inserted into objs above.
                    position = Position::At(annotation_anchors[o].clone());
                }
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
                        // the scene replaces the symbol's position axis by axis, by a PLACE or an
                        // offset, whichever is written later, and only that one is a fact (13.3)
                        let replaced =
                            |axis: Axis| matches!(slot(axis), Slot::Fixed { .. }) || offset.contains(&(k, axis));
                        let partly =
                            |axis: Axis| Position::InSymbol { owner, local, replaced: Some((axis, slot(axis))) };
                        position = match (replaced(Axis::X), replaced(Axis::Y)) {
                            (true, true) => own.clone(),
                            (false, false) => Position::InSymbol { owner, local, replaced: None },
                            (true, false) => partly(Axis::X),
                            (false, true) => partly(Axis::Y),
                        };
                    }
                    let lines = match &items.text {
                        Some(t) => self.text_lines(s, t, att),
                        None => Vec::new(),
                    };
                    let style = self.scenes[s]
                        .style
                        .get(o)
                        .map(|m| {
                            m.iter()
                                .filter(|(k, _)| {
                                    matches!(k.as_str(), "size" | "font" | "bold" | "italic" | "textvisible")
                                })
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
                        fix_rotate: items.fix_rotate.unwrap_or(false),
                        upright: items.upright.unwrap_or(true),
                        baseline_mirrors: items.modern_text
                            || items.text.as_ref().and_then(|t| t.unit).is_some_and(|u| self.follows_0_4(u)),
                        column_position: self.text_column_position(s, *o),
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
                nets: Vec::new(),
            });
        }

        // wires (13.1), a rewritten one as its rewrite gives it (the draft 0.3): read without the
        // records of the solver, as by a solver, the rewrites of the authored records
        let mut problem_wires = Vec::new();
        for &w in &wires {
            let ObjData::Wire { nets, bus, generated, .. } = self.o(w).clone() else { continue };
            let Some(WireShape { points, mode, site, .. }) = self.wire_shape(s, w) else { continue };
            let default_auto = mode == "auto" && self.modern_wire_source(s, site.as_ref());
            let anchors: Vec<PointRef> = points.iter().filter_map(|q| self.point_ref(s, &ix, *q)).collect();
            problem_wires.push(ProblemWire {
                label: self.label(w),
                reference: ix.refs.get(&w).cloned().unwrap_or(Reference::Anonymous),
                nets: nets.iter().map(|n| self.net_display(*n)).collect(),
                bus,
                anchors,
                mode,
                default_auto,
                generated,
            });
        }

        // ROUTE (13.7): the pins of the net, and the wires already on it
        let mut routes = Vec::new();
        let route_objs = self.scenes[s].routes.clone();
        if !self.scenes[s].routing_requests.is_empty() {
            for tree in self.scenes[s].resolved_trees.clone() {
                let pins = tree.pins.iter().filter_map(|pin| self.point_ref(s, &ix, *pin)).collect();
                routes.push(RouteIntent {
                    tree: true,
                    net: self.net_display(tree.net),
                    mode: tree.mode,
                    style: tree.style.into_iter().collect(),
                    pins,
                    wires: tree.wires.iter().filter_map(|wire| ix.wires.get(wire).copied()).collect(),
                });
            }
        } else if !route_objs.is_empty() {
            let terms = self.route_terms(s);
            for r in route_objs {
                let ObjData::Route { net, mode, style, .. } = self.o(r).clone() else { continue };
                let mut pins = Vec::new();
                let mut seen: HashSet<(ObjId, u32)> = HashSet::new();
                for &(ow, t, i) in terms.get(&net).into_iter().flatten() {
                    if matches!(self.o(t), ObjData::ScenePin { .. }) {
                        if let Some(pin) = self.point_ref(s, &ix, t) {
                            pins.push(pin);
                        }
                        continue;
                    }
                    if self.scenes[s].symbol.get(&ow).is_some_and(|sym| self.symbol(*sym).independent_pins) {
                        for pin in self.pins_for(s, ow, t) {
                            if let Some(p) = self.symbol_pin_ref(s, &ix, ow, pin) {
                                if !pins.iter().any(|q: &PointRef| q.pin_key() == p.pin_key()) {
                                    pins.push(p);
                                }
                            }
                        }
                        continue;
                    }
                    if self.has_pin(s, t) && seen.insert((ow, i)) {
                        if let Some(p) = self.point_ref(s, &ix, t) {
                            pins.push(p);
                        }
                    }
                }
                let on_net = self.scenes[s].wires_by_net.get(&net).cloned().unwrap_or_default();
                routes.push(RouteIntent {
                    tree: false,
                    net: self.net_display(net),
                    mode,
                    style: style.into_iter().collect(),
                    pins,
                    wires: on_net.iter().filter_map(|w| ix.wires.get(w).copied()).collect(),
                });
            }
        }

        // CURRENT is resolved after symbol-dependent conversions (8.2, 15.2). Read only
        // effective declarations and use the final direction of each host terminal.
        let mut currents = Vec::new();
        if self.follows_0_4(self.level_unit[lv]) {
            let marked: Vec<ObjId> = self.scenes[s]
                .currents
                .iter()
                .filter(|fact| self.current_fact_active(s, fact, usize::MAX))
                .filter_map(|fact| fact.terminal)
                .collect();
            let mut seen = HashSet::new();
            for terminal in marked {
                if !seen.insert(terminal) || !self.has_pin(s, terminal) {
                    continue;
                }
                let direction = self.current_of(s, terminal);
                if direction == "unspecified" {
                    continue;
                }
                if let Some(pin) = self.point_ref(s, &ix, terminal) {
                    currents.push(TerminalCurrent { pin, direction });
                }
            }
        }

        // the nets (6.3), in the order the netlist first names them: each with the wires that draw
        // it, the ROUTEs that ask for it and its place among the ports of the subcircuit
        let port_keys: Vec<String> =
            level.ports.iter().map(|p| self.doc.net_key_in(self.scenes[s].level, p, &level.file)).collect();
        let mut nets: Vec<ProblemNet> = Vec::new();
        let mut net_at: HashMap<ObjId, usize> = HashMap::new();
        for n in self.scenes[s].nets.clone() {
            let ObjData::Net { key, .. } = self.o(n).clone() else { continue };
            let name = self.net_display(n);
            let on_net = self.scenes[s].wires_by_net.get(&n).cloned().unwrap_or_default();
            let mut on_wires: Vec<usize> = on_net.iter().filter_map(|w| ix.wires.get(w).copied()).collect();
            on_wires.sort_unstable();
            on_wires.dedup();
            net_at.insert(n, nets.len());
            nets.push(ProblemNet {
                reference: Reference::Name(format!("&{}", written_name(&name))),
                pins: Vec::new(),
                wires: on_wires,
                routes: routes.iter().enumerate().filter(|(_, r)| r.net == name).map(|(i, _)| i).collect(),
                port: port_keys.iter().position(|p| *p == key),
                name,
            });
        }
        // the pins on each net: every terminal with a pin of an instance, a bundle or a virtual
        // part, on each of its nets; and the nets of each virtual part and of each text, by what it
        // is attached to
        let mut owners: Vec<(usize, ObjId)> = ix.objects.iter().map(|(&o, &k)| (k, o)).collect();
        owners.sort_unstable();
        for (k, o) in owners {
            let kind = objects[k].kind;
            let mut on: Vec<ObjId> = Vec::new();
            match kind {
                ObjectKind::Instance | ObjectKind::Bundle | ObjectKind::Usage => {
                    for t in self.terms_of(s, o) {
                        let at = self.term_nets(s, t);
                        if kind == ObjectKind::Usage {
                            on.extend(at.iter().copied());
                        }
                        for pin in self.pins_for(s, o, t) {
                            // Keep the terminal chosen by older exports unchanged. Only an earlier
                            // independent pin needs a different reference.
                            let reference = if self.pin_for(s, o, t) == Some(pin) {
                                self.point_ref(s, &ix, t)
                            } else {
                                self.symbol_pin_ref(s, &ix, o, pin)
                            };
                            let Some(reference) = reference else { continue };
                            for n in &at {
                                let Some(&i) = net_at.get(n) else { continue };
                                let pins = &mut nets[i].pins;
                                if !pins.iter().any(|q| q.pin_key() == reference.pin_key()) {
                                    pins.push(reference.clone());
                                }
                            }
                        }
                    }
                }
                ObjectKind::Text => {
                    let att = self.scene_value(self.text_attach(o));
                    on = match self.o(att).clone() {
                        ObjData::Net { .. } => vec![att],
                        ObjData::Term { .. } => self.term_nets(s, att),
                        ObjData::ScenePin { owner, index } => self.scene_pin_nets(s, owner, index),
                        ObjData::Wire { nets, .. } => nets,
                        _ => Vec::new(),
                    };
                }
                ObjectKind::Point | ObjectKind::Annotation => {}
            }
            let mut at: Vec<usize> = on.iter().filter_map(|n| net_at.get(n).copied()).collect();
            at.sort_unstable();
            at.dedup();
            objects[k].nets = at;
        }
        for n in &mut nets {
            n.pins.sort_by_key(|q| q.pin_key().unwrap_or((usize::MAX, usize::MAX)));
        }

        // GROUP (11.5), without those PRIORITY turns off
        let mut groups = Vec::new();
        let mut group_ids = std::collections::BTreeMap::new();
        for g in self.scenes[s].groups.clone() {
            let set = g.set;
            if self.failed_group(&g) {
                continue;
            }
            let next = group_ids.len();
            let set_value = self.scene_value(set);
            let first = !group_ids.contains_key(&set_value);
            let identity = *group_ids.entry(set_value).or_insert(next);
            if self.group_disabled(s, &g) {
                continue;
            }
            let members = self.group_members(s, &g);
            groups.push(ProblemGroup {
                identity,
                first,
                direct: g.chain.is_empty(),
                spacing_x: g.spacing_x,
                spacing_y: g.spacing_y,
                label: self.label(set),
                members: members.iter().filter_map(|m| ix.objects.get(m).copied()).collect(),
            });
        }

        // usages (9.4, 12.1)
        let mut entries = Vec::new();
        for &a in &usages {
            let u = self.usage(a).clone();
            let mut roles = Vec::new();
            // the points written as coordinates in the roles of a usage that a rule makes are
            // objects of the statement that writes it out, numbered after those of its body (5.9)
            let mut role_points = 0;
            for (role, filler) in &u.roles {
                let members = self.scene_input_members(s, *filler, None);
                let written = self.written_filler(s, &ix, *filler);
                role_points += written.as_ref().map_or(0, |(_, n)| *n);
                roles.push(RoleFill {
                    role: role.clone(),
                    members: members.iter().map(|m| self.filler(s, &ix, m.object)).collect(),
                    written: written.map(|(w, _)| w),
                });
            }
            let written_numbers =
                if self.rule_made(a) { bodies.get(&a).map_or(0, |b| b.len()) + role_points } else { 0 };
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
                written_numbers,
                source: u.site.as_ref().and_then(|c| self.rec_loc(c.rec, u.off)),
            });
        }

        let version = self.version_of(Some(self.level_unit[lv]));
        let authored_numbers = self.level_numbers(s).authored.len();
        let result = if self.read_generated {
            let refs: Vec<(ObjId, Reference)> =
                objs.iter().zip(&objects).map(|((o, _), po)| (*o, po.reference.clone())).collect();
            self.record_result(s, &ix, &refs, &wires, &by_number, authored_numbers)
        } else {
            None
        };
        let object_ids: Vec<ObjId> = objs.iter().map(|(o, _)| *o).collect();
        let ids = self.problem_ids(s, &object_ids, &usages);
        let solver_config = self.solver_config_for(s);
        let (junction, definition) = self.junction_with_tools_for(s, &solver_config);
        let mut existing_junctions = Vec::new();
        if self.follows_0_4(self.level_unit[lv]) {
            let replaced_records = self.solver_records(s);
            for &a in &usages {
                let mut root = a;
                let mut chain = Vec::new();
                while let Some(parent) = self.usage(root).parent {
                    chain.push(parent);
                    root = parent;
                }
                chain.reverse();
                if self
                    .usage(root)
                    .site
                    .as_ref()
                    .and_then(|ctx| ctx.rec)
                    .is_some_and(|rec| replaced_records.contains(&rec))
                {
                    continue;
                }
                let u = self.usage(a);
                if Some(u.definition) != definition
                    || self.eff_priority(s, u.priority, &chain, Some(a)) == Priority::Disabled
                    || self.drawing_ignored(s, a)
                {
                    continue;
                }
                let role = u.roles.first().map(|(_, r)| *r);
                if let Some(role) = role {
                    for obj in self.flatten(s, role, 7, None) {
                        if let Some(point) = self.point_ref(s, &ix, obj) {
                            if let Some(key) = junction_point_identity(&point) {
                                if !existing_junctions.iter().any(|p| junction_point_identity(p) == Some(key)) {
                                    existing_junctions.push(point);
                                }
                            }
                        }
                    }
                }
            }
        }
        let problem = LevelProblem {
            annotations: annotation_templates,
            junction,
            existing_junctions,
            solver_config,
            level: lv,
            title: level.title(),
            kind: if level.kind == LevelKind::Subckt { "subckt" } else { "file" },
            name: level.name.clone(),
            version,
            authored_numbers,
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
            currents,
            groups,
            usages: entries,
            nets,
            orient_lists,
            ids,
        };
        (problem, result)
    }

    /// What the last record of the solver of scene `s` gives, read with it: the coordinates and
    /// orientations of its PLACE and ORIENT statements, the routes of the wires it rewrites and the
    /// wires it writes itself, with their points as drawn (the draft 0.3, 13.1, 14.8). The objects
    /// and wires are given by the keys that a problem built without the record finds them by:
    /// their names, the numbers of the authored records, and the places of the objects without a
    /// name in the bodies of the usages that the record writes out (5.9, 10.5). `objects` are the
    /// level's objects with their references, `wires` its wires, `by_number` the numbered objects.
    fn record_result(
        &mut self,
        s: SceneId,
        ix: &Index,
        objects: &[(ObjId, Reference)],
        wires: &[ObjId],
        by_number: &HashMap<usize, ObjId>,
        authored: usize,
    ) -> Option<RecordResult> {
        let g = *self.solver_records(s).last()?;
        // the objects without a name in the bodies of the usages it writes out, by the usage's name
        let mut bodies: HashMap<ObjId, (String, usize)> = HashMap::new();
        for (u, list) in self.written_bodies(s, g) {
            let Some(Reference::Name(name)) = ix.refs.get(&u) else { continue };
            for (k, o) in list.into_iter().enumerate() {
                bodies.insert(o, (name.clone(), k + 1));
            }
        }
        let key = |r: &Reference| match r {
            Reference::Name(n) => Some(Key::Name(n.clone())),
            Reference::Number { number, path } => match by_number.get(number).and_then(|o| bodies.get(o)) {
                Some((usage, k)) => Some(Key::Body(usage.clone(), *k, path.concat())),
                None if *number <= authored => Some(Key::Number(*number, path.concat())),
                None => None,
            },
            _ => None,
        };
        // a statement of the record itself, not of the body of a usage it makes
        let of_record = |ctx: Option<&Rc<Ctx>>| ctx.is_some_and(|c| c.rec == Some(g) && c.frame_usage().is_none());

        // the coordinates and orientations it gives, the latest of each in the order of writing (5.5)
        let sc = &self.scenes[s];
        let mut coords: HashMap<(ObjId, char), (Pos, f64)> = HashMap::new();
        for &k in &sc.at_live {
            let f = &sc.at[k];
            let FactData::Coord(axis, value) = f.data else { continue };
            if f.chain.is_empty() && of_record(f.ctx.as_ref()) {
                let order = f.order();
                let e = coords.entry((f.target, axis)).or_insert((order.clone(), value));
                if order >= e.0 {
                    *e = (order, value);
                }
            }
        }
        let mut orients: HashMap<ObjId, (Pos, Orient)> = HashMap::new();
        for &k in &sc.orient_live {
            let f = &sc.orient[k];
            let FactData::Orient(code) = f.data else { continue };
            if f.chain.is_empty() && of_record(f.ctx.as_ref()) {
                let order = f.order();
                let e = orients.entry(f.target).or_insert((order.clone(), code));
                if order >= e.0 {
                    *e = (order, code);
                }
            }
        }
        let mut placed = Vec::new();
        for (o, r) in objects {
            let x = coords.get(&(*o, 'x')).map(|e| e.1);
            let y = coords.get(&(*o, 'y')).map(|e| e.1);
            let orient = orients.get(o).map(|e| e.1);
            if x.is_none() && y.is_none() && orient.is_none() {
                continue;
            }
            if let Some(key) = key(r) {
                placed.push(Given { key, x, y, orient });
            }
        }

        // the wires it writes itself, and the routes of those it rewrites (13.1)
        let mut routes = Vec::new();
        let mut written = Vec::new();
        for &w in wires {
            let ObjData::Wire { nets, points, mode, generated, site, .. } = self.o(w).clone() else { continue };
            if !generated && of_record(site.as_ref()) {
                let nets = nets.iter().map(|n| self.net_display(*n)).collect();
                let modern = self.modern_wire_source(s, site.as_ref());
                if let Some(path) = self.routed_wire_run(s, &points, &mode, modern, (true, true)) {
                    written.push((nets, path));
                }
                continue;
            }
            let rewrite = self.scenes[s].rewrites.get(&w).and_then(|rs| {
                rs.iter()
                    .filter(|r| r.fault.is_none() && !r.generated && of_record(Some(&r.site)))
                    .max_by(|a, b| (&a.pos, a.seq).cmp(&(&b.pos, b.seq)))
                    .map(|r| (r.points.clone(), r.mode.clone(), r.site.clone()))
            });
            let (Some((points, mode, site)), Some(k)) = (rewrite, ix.refs.get(&w).and_then(key)) else { continue };
            let modern = self.modern_wire_source(s, Some(&site));
            if let Some(path) = self.routed_wire_run(s, &points, &mode, modern, (true, true)) {
                routes.push((k, path));
            }
        }
        // where each object is drawn with the record, as far as the text says
        let mut drawn = HashMap::new();
        for (o, r) in objects {
            let Some(k) = key(r) else { continue };
            let at = match self.o(*o) {
                ObjData::Text { .. } | ObjData::KeyText { .. } => self.text_axes(s, *o).0,
                _ => self.scenes[s].pos.get(o).copied().unwrap_or([None, None]),
            };
            drawn.insert(k, (at, self.orientation(s, *o)));
        }
        // what a problem read without the record finds each object and usage by, with its id here
        let level_ids = self.problem_ids(s, &objects.iter().map(|(o, _)| *o).collect::<Vec<_>>(), &[]);
        let mut ids: HashMap<Key, String> = HashMap::new();
        for ((_, r), id) in objects.iter().zip(level_ids.objects) {
            if let Some(k) = key(r) {
                ids.insert(k, id);
            }
        }
        let usages: Vec<ObjId> = ix.usages.keys().copied().collect();
        let usage_ids = self.problem_ids(s, &[], &usages).usages;
        for (a, id) in usages.iter().zip(usage_ids) {
            if let Some(k) = ix.refs.get(a).and_then(key) {
                ids.insert(k, id);
            }
        }
        Some(RecordResult { level: self.scenes[s].level, placed, routes, wires: written, ids, drawn })
    }

    /// The `%` names the level binds, those of the records written into it with scope= among them
    /// (2.6, 6.7), with their objects in this scene, in the order they are declared; a name bound
    /// twice keeps its first binding (6.6).
    /// The objects of a level's problem that have a place of their own, kind by kind, each in the
    /// order it was made: instances and bundles, usages drawn or with a point of their own, points
    /// and texts; with the usages and the annotations, which find their place later.
    pub(super) fn problem_objects(&mut self, s: SceneId) -> (Vec<(ObjId, ObjectKind)>, Vec<ObjId>, Vec<ObjId>) {
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
        let mut usages: Vec<ObjId> = self.scenes[s]
            .usages
            .iter()
            .copied()
            .filter(|a| !self.usage(*a).in_symbol && !self.is_failed(*a))
            .collect();
        usages.sort();
        usages.dedup();
        for &a in &usages {
            if self.scenes[s].symbol.contains_key(&a) || self.usage_own_point(a) {
                objs.push((a, ObjectKind::Usage));
            }
        }
        let annotations: Vec<ObjId> = self.scenes[s]
            .prims
            .iter()
            .copied()
            .filter(|&o| {
                matches!(self.o(o), ObjData::Annotation(_)) && !self.is_obj_failed(o) && !self.drawing_ignored(s, o)
            })
            .collect();
        let mut points = self.scenes[s].points.clone();
        points.retain(|p| !self.is_failed(*p));
        points.sort();
        points.dedup();
        objs.extend(points.into_iter().map(|p| (p, ObjectKind::Point)));
        let mut texts: Vec<ObjId> = self.scenes[s]
            .texts
            .iter()
            .copied()
            .filter(|t| !self.is_obj_failed(*t) && !self.drawing_ignored(s, *t))
            .collect();
        texts.sort();
        texts.dedup();
        objs.extend(texts.into_iter().map(|t| (t, ObjectKind::Text)));
        (objs, usages, annotations)
    }

    pub(crate) fn level_names(&self, s: SceneId) -> Vec<(String, ObjId)> {
        let u = self.level_unit[self.scenes[s].level];
        let mut out = Vec::new();
        for key in &self.units[u].name_order {
            if key.0 != '%' {
                continue;
            }
            let b = &self.units[u].names[key];
            if let Some(o) = self.evaluated_binding(b, Some(s)) {
                if o != NOTHING && o != LEVEL && o != OWNER && !self.is_failed(o) {
                    out.push((b.name.clone(), o));
                }
            }
        }
        out
    }

    /// A usage made by a generation rule itself (10.5), not in the expansion of another
    /// generated usage.
    pub(crate) fn rule_made(&self, a: ObjId) -> bool {
        let u = self.usage(a);
        u.generated && u.parent.is_none_or(|p| !self.usage(p).generated)
    }

    /// Made by a rule (10.5): a generated usage, wire or text, or any object made in a generation,
    /// whose position carries the mark of the generation rule.
    pub(crate) fn made_by_rule(&self, s: SceneId, o: ObjId) -> bool {
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
        let o = self.input_value(o);
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
            ObjData::ScenePin { owner, index } if self.follows_0_4(self.level_unit[self.scenes[s].level]) => {
                match self.symbol_pin_ref(s, ix, owner, index) {
                    Some(PointRef::SymbolPin { reference, .. }) => reference,
                    Some(PointRef::Pin { terminal, .. }) => {
                        self.reference_of(s, ix, owner).then(&format!(":{}", written_name(&terminal)))
                    }
                    _ => Reference::Anonymous,
                }
            }
            ObjData::NamedSubobject { parent, name, .. } => {
                self.reference_of(s, ix, parent).then(&format!("::{}", written_name(&name)))
            }
            ObjData::KeyText { .. } => self.text_reference(s, o, ix),
            _ => ix.refs.get(&o).cloned().unwrap_or(Reference::Anonymous),
        }
    }

    /// How the level's text writes what fills a role: a name or handle, a point with both
    /// coordinates written, or a set of them in `{…}`, with how many points it writes as
    /// coordinates. None when a part cannot be referred to by a name the level writes.
    fn written_filler(&mut self, s: SceneId, ix: &Index, o: ObjId) -> Option<(String, usize)> {
        let o = self.written_input(o);
        if self.is_empty_node(o) {
            return Some(("&".into(), 0));
        }
        if let ObjData::Set(_) = self.o(o) {
            let items = self.items_of(s, o, 7, None);
            let (mut out, mut points) = (Vec::new(), 0);
            for m in items {
                let (w, n) = self.written_filler(s, ix, m)?;
                out.push(w);
                points += n;
            }
            return Some((format!("{{{}}}", out.join(" ")), points));
        }
        match self.reference_of(s, ix, o) {
            Reference::Name(n) => Some((n, 0)),
            Reference::Number { number, path }
                if self.follows_0_4(self.level_unit[self.scenes[s].level])
                    && number > 0
                    && number <= self.level_numbers(s).authored.len() =>
            {
                // Authored numbers survive replacement of the solver record. Its
                // own numbers do not, and must never become role references here.
                Some((format!("%#{number}{}", path.concat()), 0))
            }
            _ => match self.o(o) {
                ObjData::Point { x: Some(x), y: Some(y), .. } => {
                    Some((format!("({}, {})", crate::geom::fmt_num(*x), crate::geom::fmt_num(*y)), 1))
                }
                _ => None,
            },
        }
    }

    /// What fills a role, or what a text is attached to, as the problem refers to it.
    fn filler(&mut self, s: SceneId, ix: &Index, o: ObjId) -> Filler {
        let o = self.scene_value(o);
        if let Some(&k) = ix.objects.get(&o) {
            return Filler::Object(k);
        }
        match self.o(o).clone() {
            ObjData::ScenePin { owner, index } if ix.objects.contains_key(&owner) => {
                Filler::Pin { object: ix.objects[&owner], pin: index }
            }
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
        let o = self.input_value(o);
        match self.o(o).clone() {
            ObjData::NamedSubobject { resolved, .. } => {
                let value = resolved?;
                if let ObjData::ScenePin { owner, index } = *self.o(value) {
                    Some(PointRef::SymbolPin {
                        object: *ix.objects.get(&owner)?,
                        pin: index,
                        reference: self.reference_of(s, ix, o),
                    })
                } else {
                    self.point_ref(s, ix, value)
                }
            }
            ObjData::Annotation(data) => {
                let anchor = match data.anchor {
                    AnnotationAnchor::Independent(o) | AnnotationAnchor::Follow(o) => o,
                };
                self.point_ref(s, ix, anchor)
            }
            ObjData::ScenePin { owner, index } => self.symbol_pin_ref(s, ix, owner, index),
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
            ObjData::Pos { parent } => {
                let point = self.point_ref(s, ix, parent)?;
                match point {
                    PointRef::SymbolPin { object, pin, reference } => {
                        Some(PointRef::PinPosition { object, pin, reference: reference.then("::position") })
                    }
                    PointRef::Pin { object, pin, terminal }
                        if matches!(self.o(parent), ObjData::ScenePin { .. })
                            || matches!(self.o(parent), ObjData::Term { owner, .. }
                                if self.scenes[s].symbol.get(owner).is_some_and(|s| self.symbol(*s).independent_pins)) =>
                    {
                        let reference = match *self.o(parent) {
                            ObjData::ScenePin { owner, .. } => {
                                self.reference_of(s, ix, owner).then(&format!(":{}", written_name(&terminal)))
                            }
                            _ => self.reference_of(s, ix, parent),
                        };
                        Some(PointRef::PinPosition { object, pin, reference: reference.then("::position") })
                    }
                    point => Some(point),
                }
            }
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

    /// Reference exactly this pin. A bus pin can use any terminal for which it is the last
    /// binding, but an earlier pin must never borrow the reference of a later one (5.2.4).
    fn symbol_pin_ref(&mut self, s: SceneId, ix: &Index, owner: ObjId, pin: usize) -> Option<PointRef> {
        let object = *ix.objects.get(&owner)?;
        let name = self.scenes[s]
            .symbol
            .get(&owner)
            .and_then(|symbol| self.symbol(*symbol).pins.get(pin))
            .and_then(|pin| pin.names.first())
            .cloned();
        if let Some(name) = name {
            let reference = self.reference_of(s, ix, owner).then(&format!("::{}", written_name(&name)));
            return Some(PointRef::SymbolPin { object, pin, reference });
        }
        for t in self.terms_of(s, owner) {
            if self.pin_for(s, owner, t) == Some(pin) {
                return self.point_ref(s, ix, t);
            }
        }
        Some(PointRef::SymbolPin { object, pin, reference: Reference::Anonymous })
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
            bbox: sd.bbox.unwrap_or_else(|| self.symbol_extent(sym)),
            box_written: sd.bbox.is_some(),
            default: sd.default,
            pins: sd
                .pins
                .iter()
                .map(|p| crate::model::PinView {
                    identity: sd.independent_pins.then(|| crate::model::SymbolPinIdentity {
                        names: p.names.clone(),
                        bus: p.bus || p.terms.len() > 1,
                    }),
                    pos: p.pos,
                    label: p.label.clone(),
                    label_name_formats: p.label_name_formats,
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
    /// The facts of scene `s` in the order of 8.5, and the lists of codes of those of them written
    /// with one ([`LevelProblem::orient_lists`]).
    #[allow(clippy::type_complexity)]
    fn problem_facts(
        &mut self,
        s: SceneId,
        ix: &Index,
        objs: &[(ObjId, ObjectKind)],
    ) -> (Vec<ProblemFact>, Vec<(usize, Vec<Orient>)>) {
        let mut ranked: Vec<Ranked> = Vec::new();
        // the lists of codes of 0.4 (11.2, 14.9), by the number each fact is collected with
        let mut lists: HashMap<usize, Vec<Orient>> = HashMap::new();
        let rank = |p: Priority| p as u8;
        // the texts whose offset on an axis replaces their PLACE on that axis, or the reverse; either
        // replaces the position that the symbol gives a text on that axis (13.3)
        let mut offset_wins: HashMap<(ObjId, char), bool> = HashMap::new();
        for (o, kind) in objs {
            if *kind != ObjectKind::Text {
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
                        matches!(f.data, FactData::Coord(a, _) | FactData::RelativeCoord(a, _, _) if a == axis)
                            && self.fact_priority(s, f) != Priority::Disabled
                    })
                    .map(|f| f.order())
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
            let (axis, value, base) = match f.data {
                FactData::Coord(axis, value) => (axis, value, None),
                FactData::RelativeCoord(axis, value, base) => (axis, value, Some(base)),
                _ => continue,
            };
            if offset_wins.get(&(f.target, axis)) == Some(&true) {
                continue;
            }
            let point = f.ctx.is_none() && matches!(self.o(f.target), ObjData::Point { .. });
            let kind = if let Some(base) = base {
                let Some(from) = self.point_ref(s, ix, base) else { continue };
                let Some(to) = self.point_ref(s, ix, f.target) else { continue };
                FactKind::Distance {
                    axis: Axis::of(axis),
                    sense: if value < 0.0 { Sense::Decreasing } else { Sense::Increasing },
                    min: Some(value.abs()),
                    max: Some(value.abs()),
                    from: vec![from],
                    to: vec![to],
                }
            } else if point {
                let Some(&point) = ix.objects.get(&f.target) else { continue };
                FactKind::PointCoordinate { point, axis: Axis::of(axis), value }
            } else {
                let Some(at) = self.point_ref(s, ix, f.target) else { continue };
                FactKind::Place { at, axis: Axis::of(axis), value }
            };
            let label = self.fact_text(f);
            let source = f.ctx.as_ref().and_then(|c| self.rec_loc(c.rec, f.off));
            ranked.push(Ranked {
                key: (rank(p), f.order(), 0, (axis == 'y') as u8, ranked.len()),
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
                // where the offset is written, in the statement that made the text or in a later
                // TEXT with the same key, in the order of writing
                let order = self.offset_pos(s, *o, axis == 'x');
                ranked.push(Ranked {
                    key: (rank(p), order, 1, (axis == 'y') as u8, ranked.len()),
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
            let source = f.ctx.as_ref().and_then(|c| self.rec_loc(c.rec, f.off));
            let kind = if let Some(pin) = self.export_directed_pin(s, ix, f.target) {
                let codes = self.scenes[s].orient_codes.get(&k).cloned().unwrap_or_else(|| vec![code]);
                FactKind::PinOrient { pin, codes }
            } else {
                let Some(&object) = ix.objects.get(&f.target) else { continue };
                if let Some(codes) = self.scenes[s].orient_codes.get(&k) {
                    lists.insert(ranked.len(), codes.clone());
                }
                FactKind::Orient { object, code }
            };
            ranked.push(Ranked {
                key: (rank(p), f.order(), 2, 0, ranked.len()),
                fact: ProblemFact {
                    kind,
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
            if cd.failed || cd.status == Some("ignored") {
                continue;
            }
            let p = cd.eff.unwrap_or_else(|| self.eff_priority(s, cd.priority_param, &cd.chain, Some(c)));
            if p == Priority::Disabled {
                continue;
            }
            let Some(kind) = self.constraint_kind(s, ix, &cd) else { continue };
            let source = cd.site.as_ref().and_then(|ctx| self.rec_loc(ctx.rec, cd.off));
            let reference = match ix.refs.get(&c).or_else(|| cd.name_owner.and_then(|p| ix.refs.get(&p))) {
                Some(Reference::Anonymous) | None => None,
                Some(r) => Some(r.clone()),
            };
            let mut pos = self.scenes[s].created.get(&c).cloned().unwrap_or_default();
            if let Some(site) = &cd.site {
                pos = order_pos(&pos, &site.segments);
            }
            if matches!(kind, FactKind::OrientRelation { .. }) && !cd.params.relations.is_empty() {
                lists.insert(ranked.len(), cd.params.relations.clone());
            }
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
        let mut orient_lists = Vec::new();
        let facts = ranked
            .into_iter()
            .enumerate()
            .map(|(i, r)| {
                if let Some(codes) = lists.remove(&r.key.4) {
                    orient_lists.push((i, codes));
                }
                r.fact
            })
            .collect();
        (facts, orient_lists)
    }

    fn export_directed_pin(&mut self, s: SceneId, ix: &Index, object: ObjId) -> Option<DirectedPin> {
        let (owner, pin, direction) = self.direction_pin(s, object)?;
        Some(DirectedPin { object: *ix.objects.get(&owner)?, pin, direction: direction? })
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
                // a set without members is no member (14.1)
                if it.is_set(m) && it.flatten(s, m, 7, None).is_empty() {
                    continue;
                }
                out.push(it.point_ref(s, ix, m)?);
            }
            Some(out)
        };
        Some(match cd.kind {
            ConstraintKind::Table => FactKind::Table {
                axis,
                rows: cd.table.as_ref()?.try_map(&mut |object| self.point_ref(s, ix, *object))?,
            },
            ConstraintKind::Align => FactKind::Align { axis, members: members(self)? },
            ConstraintKind::Order => FactKind::Order { axis, sense, members: members(self)? },
            ConstraintKind::Distribute => FactKind::Distribute { axis, sense, members: members(self)? },
            ConstraintKind::Distance => FactKind::Distance {
                axis,
                sense,
                min: cd.params.min,
                max: cd.params.max,
                from: self.extent_points(s, ix, cd.args[0])?,
                to: self.extent_points(s, ix, cd.args[1])?,
            },
            ConstraintKind::OrientRelation => {
                if cd.args.len() == 2 && self.is_pin_direction_input(cd.args[0]) {
                    return Some(FactKind::PinOrientRelation {
                        from: self.export_directed_pin(s, ix, cd.args[0])?,
                        to: self.export_directed_pin(s, ix, cd.args[1])?,
                        relations: cd.params.relations(),
                    });
                }
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
