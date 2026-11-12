//! Routing requests, their condition snapshots and independent trees (10.2, 10.4, 13.7).

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::ctx::Ctx;
use super::eval::R;
use super::obj::{InputMember, ObjData, ObjId, Pos, Priority, SceneId, StyleMap};
use super::Interp;
use crate::diag::codes;

/// Parameters of one of the four routing keywords. Net requests apply member by member;
/// object requests retain their complete input set as one request.
#[derive(Clone, Debug)]
pub(super) struct RoutingSpec {
    pub net: bool,
    pub join: bool,
    pub mode: String,
    pub style: StyleMap,
    pub priority: Option<Priority>,
}

#[derive(Clone)]
pub(super) struct RoutingRequest {
    pub input: ObjId,
    pub members: Vec<InputMember>,
    pub frozen_sets: HashMap<ObjId, Vec<InputMember>>,
    pub captured: bool,
    pub spec: RoutingSpec,
    pub pos: Pos,
    pub site: Rc<Ctx>,
    pub chain: Vec<ObjId>,
    pub off: usize,
    /// ROUTE-NET creates a route object; the other keywords pass their input through.
    pub object: Option<ObjId>,
}

/// Pins in one resolved tree are physical pin objects, not terminal or written-name aliases.
#[derive(Clone, Debug)]
pub(super) struct RouteTree {
    pub net: ObjId,
    pub pins: Vec<ObjId>,
    pub wires: Vec<ObjId>,
    pub mode: String,
    pub style: StyleMap,
    pub first: usize,
    pub restyled: bool,
}

/// A positive request after resolving its scope and applying pin priorities.
#[derive(Clone)]
pub(super) struct RouteCover {
    pub net: ObjId,
    pub pins: Vec<ObjId>,
    pub mode: String,
    pub style: StyleMap,
    pub first: usize,
}

/// One connected component of written wires, already grouped by net and shared handles.
pub(super) struct WrittenPiece {
    pub net: ObjId,
    pub pins: Vec<ObjId>,
    pub wires: Vec<ObjId>,
}

/// One exclusion on one net. It is compared against each positive request,
/// rather than deciding a global enabled/disabled state for a pin (13.7).
struct UnrouteCover {
    net: ObjId,
    pins: HashSet<ObjId>,
    priority: Priority,
    order: Pos,
}

fn remove_unrouted_pins(cover: &mut RouteCover, priority: Priority, order: &Pos, exclusions: &[UnrouteCover]) {
    if priority == Priority::Disabled {
        cover.pins.clear();
        return;
    }
    cover.pins.retain(|pin| {
        !exclusions.iter().any(|unroute| {
            unroute.net == cover.net
                && unroute.priority != Priority::Disabled
                && unroute.pins.contains(pin)
                && (unroute.priority > priority || unroute.priority == priority && unroute.order > *order)
        })
    });
}

fn root(parents: &mut [usize], mut at: usize) -> usize {
    while parents[at] != at {
        parents[at] = parents[parents[at]];
        at = parents[at];
    }
    at
}

/// Join only intersecting covers. A common net name alone never joins two trees.
/// Covers arrive in written order so the first one's mode and style remain authoritative.
pub(super) fn merge_covers(covers: &[RouteCover], pieces: &[WrittenPiece]) -> Vec<RouteTree> {
    let mut parents: Vec<_> = (0..covers.len()).collect();
    let mut by_pin = HashMap::new();
    for (i, cover) in covers.iter().enumerate() {
        for &pin in &cover.pins {
            if let Some(&j) = by_pin.get(&(cover.net, pin)) {
                let a = root(&mut parents, i);
                let b = root(&mut parents, j);
                parents[a] = b;
            } else {
                by_pin.insert((cover.net, pin), i);
            }
        }
    }
    let mut touched = Vec::new();
    for piece in pieces {
        let hits: Vec<_> = piece.pins.iter().filter_map(|pin| by_pin.get(&(piece.net, *pin)).copied()).collect();
        if let Some(&first) = hits.first() {
            for &index in &hits[1..] {
                let a = root(&mut parents, first);
                let b = root(&mut parents, index);
                parents[a] = b;
            }
        }
        touched.push(hits);
    }
    let mut trees: Vec<RouteTree> = Vec::new();
    let mut tree_roots: Vec<usize> = Vec::new();
    for (index, cover) in covers.iter().enumerate() {
        if cover.pins.is_empty() {
            continue;
        }
        let key = root(&mut parents, index);
        let at = match tree_roots.iter().position(|value| *value == key) {
            Some(at) => at,
            None => {
                tree_roots.push(key);
                trees.push(RouteTree {
                    net: cover.net,
                    pins: Vec::new(),
                    wires: Vec::new(),
                    mode: cover.mode.clone(),
                    style: cover.style.clone(),
                    first: cover.first,
                    restyled: false,
                });
                trees.len() - 1
            }
        };
        let tree = &mut trees[at];
        tree.restyled |= tree.style != cover.style;
        for &pin in &cover.pins {
            if !tree.pins.contains(&pin) {
                tree.pins.push(pin);
            }
        }
    }
    for (piece, hits) in pieces.iter().zip(touched) {
        let Some(&first) = hits.first() else { continue };
        let key = root(&mut parents, first);
        let at = tree_roots.iter().position(|value| *value == key).unwrap();
        for &wire in &piece.wires {
            if !trees[at].wires.contains(&wire) {
                trees[at].wires.push(wire);
            }
        }
    }
    trees
}

impl Interp<'_> {
    /// Capture set membership when the statement or rule runs, before later generation can
    /// change a named set. Physical pin resolution still waits for the required symbols.
    fn routing_members(&mut self, scene: SceneId, input: ObjId, site: &Ctx) -> Vec<InputMember> {
        fn visit(
            it: &mut Interp,
            scene: SceneId,
            member: InputMember,
            site: &Ctx,
            seen: &mut HashSet<ObjId>,
            out: &mut Vec<InputMember>,
        ) {
            let object = member.object;
            if !seen.insert(object) {
                return;
            }
            if it.is_set(object) {
                let phase = it.scenes[scene].phase.max(3);
                for mut child in it.flatten_inputs(scene, object, phase, Some(site)) {
                    child.depend_on(&member.dependencies);
                    visit(it, scene, child, site, seen, out);
                }
                return;
            }
            if it.role_inputs.contains_key(&object) {
                out.push(member);
                return;
            }
            if let ObjData::Usage(usage) = it.o(it.scene_value(object)).clone() {
                // A usage contributes its own symbol pins and the objects in its roles (13.7).
                for (_, role) in usage.roles {
                    let mut child = InputMember::plain(role);
                    child.depend_on(&member.dependencies);
                    visit(it, scene, child, site, seen, out);
                }
            }
            out.push(member);
        }
        let mut out = Vec::new();
        visit(self, scene, InputMember::plain(input), site, &mut HashSet::new(), &mut out);
        out
    }

    pub(super) fn record_routing(
        &mut self,
        scene: SceneId,
        input: ObjId,
        spec: RoutingSpec,
        pos: Pos,
        site: &Ctx,
        off: usize,
    ) -> R<ObjId> {
        if let Some(cause) = self.failed_obj(input) {
            let diagnostic = self.err_cause(
                site,
                codes::KEYWORD_OBJECT_FAILED,
                "the object of the routing statement failed",
                off,
                cause,
            );
            return Err(super::eval::Stop::Fail(Some(diagnostic)));
        }
        if spec.net
            && !self.late_input(input)
            && !matches!(self.o(input), ObjData::Net { .. } | ObjData::Term { .. } | ObjData::ScenePin { .. })
        {
            return self.fail(
                site,
                codes::ROUTE_TARGET_NOT_NET,
                format!("a net routing keyword takes nets, not {} (5.2.4, 13.7)", self.label(input)),
                off,
            );
        }
        let object = if spec.net && spec.join {
            let route = self.new_obj(ObjData::Route {
                net: input,
                mode: spec.mode.clone(),
                style: spec.style.clone(),
                site: Some(Rc::new(site.clone())),
            });
            self.scenes[scene].routes.push(route);
            self.scenes[scene].note_created(route, pos.clone(), site);
            Some(route)
        } else {
            None
        };
        // Static statements see the final ordinary edits, after creation has
        // finished. Routing rules and generated bodies capture when they run.
        let captured = self.scenes[scene].phase >= 4;
        let members = if spec.net || !captured {
            vec![InputMember::plain(input)]
        } else {
            self.routing_members(scene, input, site)
        };
        let frozen_sets = if captured { self.capture_routing_sets(scene, input, site) } else { HashMap::new() };
        let request = RoutingRequest {
            input,
            members,
            frozen_sets,
            captured,
            spec,
            pos: super::obj::order_pos(&pos, &site.segments),
            site: Rc::new(site.clone()),
            chain: self.apply_chain(site),
            off,
            object,
        };
        self.scenes[scene].routing_requests.push(request);
        Ok(object.unwrap_or(input))
    }
}

/// A conversion preview preserves the difference between one object and a one-member bus set.
struct RouteValue {
    members: Vec<ObjId>,
    set: bool,
}

impl Interp<'_> {
    /// Capture collections through role views and nested usages without making proxy
    /// objects or changing the public role's own conversion and failure handling.
    fn capture_routing_sets(&mut self, scene: SceneId, input: ObjId, site: &Ctx) -> HashMap<ObjId, Vec<InputMember>> {
        let mut frozen = HashMap::new();
        let mut seen = HashSet::new();
        let mut pending = vec![input];
        while let Some(object) = pending.pop() {
            if !seen.insert(object) {
                continue;
            }
            if let Some(view) = self.role_inputs.get(&object) {
                pending.push(view.source);
                continue;
            }
            if self.is_set(object) {
                let members = self.flatten_inputs(scene, object, self.scenes[scene].phase.max(3), Some(site));
                pending.extend(members.iter().map(|member| member.object));
                frozen.insert(object, members);
            } else if let ObjData::Usage(usage) = self.o(self.scene_value(object)) {
                pending.extend(usage.roles.iter().map(|(_, role)| *role));
            }
        }
        frozen
    }

    fn routing_value(
        &mut self,
        scene: SceneId,
        object: ObjId,
        frozen: &HashMap<ObjId, Vec<InputMember>>,
    ) -> Option<RouteValue> {
        if self.is_failed(object) {
            return None;
        }
        if let Some(view) = self.role_inputs.get(&object).cloned() {
            // Reapply only the conversion to captured members. The shared role's
            // late resolved set may include members added by later generation.
            let source = self.routing_value(scene, view.source, frozen)?;
            let spread = view.collection || view.allow_set && source.set;
            if source.set && !spread {
                return None;
            }
            let mut result_set = spread;
            let mut result = Vec::new();
            for member in source.members {
                let converted = if view.kinds.iter().any(|kind| self.role_kind_matches(member, kind)) {
                    RouteValue { members: vec![member], set: false }
                } else {
                    let target = self.conversion_kind(member, &view.kinds)?;
                    self.routing_conversion(scene, member, &target)?
                };
                if converted.set && !view.allow_set {
                    return None;
                }
                result_set |= converted.set;
                result.extend(converted.members);
            }
            return Some(RouteValue { members: result, set: result_set });
        }
        let object = self.scene_value(object);
        if self.is_set(object) {
            let captured = frozen.get(&object)?;
            let mut members = Vec::new();
            for member in captured {
                if member.dependencies.iter().any(|dependency| self.is_failed(*dependency)) {
                    return None;
                }
                members.extend(self.routing_value(scene, member.object, frozen)?.members);
            }
            return Some(RouteValue { members, set: true });
        }
        if matches!(self.o(object), ObjData::NamedSubobject { resolved: None, .. }) {
            return None;
        }
        Some(RouteValue { members: vec![object], set: false })
    }

    /// The same permitted conversions as 5.2.4, without reporting a role failure before its
    /// final check. No creation or set identity is revised by this preview (8.2).
    fn routing_conversion(&mut self, scene: SceneId, object: ObjId, target: &str) -> Option<RouteValue> {
        let single = |object| Some(RouteValue { members: vec![object], set: false });
        if self.role_kind_matches(object, target) {
            return single(object);
        }
        match (self.o(object).clone(), target) {
            (ObjData::Term { owner, .. }, "pin" | "point") => {
                let index = self.pin_for(scene, owner, object)?;
                let pin = self.scene_pin_object(scene, owner, index);
                single(if target == "point" { self.scene_pos_of(scene, pin) } else { pin })
            }
            (ObjData::Term { .. }, "net") => {
                let members = self.tnets(scene, object);
                let first = *members.first()?;
                if members.iter().all(|member| *member == first) {
                    single(first)
                } else {
                    Some(RouteValue { members, set: true })
                }
            }
            (ObjData::ScenePin { .. }, "point") => single(self.scene_pos_of(scene, object)),
            (ObjData::ScenePin { owner, index }, "terminal" | "net") => {
                let members = if target == "net" {
                    self.scene_pin_nets(scene, owner, index)
                } else {
                    self.scene_pin_terminals(scene, owner, index)
                };
                if members.is_empty() {
                    return (target == "net" && self.scene_pin_has_empty_node(scene, owner, index))
                        .then_some(RouteValue { members, set: false });
                }
                let set = members.len() != 1 || self.scene_pin_is_bus(scene, owner, index);
                Some(RouteValue { members, set })
            }
            _ => None,
        }
    }

    fn routing_object_pins(
        &mut self,
        scene: SceneId,
        object: ObjId,
        frozen: &HashMap<ObjId, Vec<InputMember>>,
        seen: &mut HashSet<ObjId>,
        out: &mut HashSet<ObjId>,
    ) -> bool {
        if !seen.insert(object) {
            return true;
        }
        let Some(value) = self.routing_value(scene, object, frozen) else { return false };
        for member in value.members {
            match self.o(member).clone() {
                ObjData::Term { owner, .. } => {
                    if matches!(self.o(owner), ObjData::Inst { bundle: Some(_), .. }) {
                        continue;
                    }
                    if let Some(index) = self.pin_for(scene, owner, member) {
                        out.insert(self.scene_pin_object(scene, owner, index));
                    }
                }
                ObjData::ScenePin { owner, .. } => {
                    if !matches!(self.o(owner), ObjData::Inst { bundle: Some(_), .. }) && self.has_pin(scene, member) {
                        out.insert(member);
                    }
                }
                ObjData::Inst { bundle: Some(_), .. } => {}
                ObjData::Inst { .. } | ObjData::Bundle { .. } | ObjData::Usage(_) => {
                    if let Some(&symbol) = self.scenes[scene].symbol.get(&member) {
                        for index in 0..self.symbol(symbol).pins.len() {
                            out.insert(self.scene_pin_object(scene, member, index));
                        }
                    }
                    // Roles of ordinary usages were captured in routing_members. A usage
                    // produced through a role conversion keeps its immutable role objects.
                    if member != object {
                        if let ObjData::Usage(usage) = self.o(member).clone() {
                            for (_, role) in usage.roles {
                                if !self.routing_object_pins(scene, role, frozen, seen, out) {
                                    return false;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        true
    }
}

impl Interp<'_> {
    /// Written wires connect through shared handles, including intermediate points. Coordinates
    /// do not connect wires. Conditions keep the written shape even if a later pin check fails.
    fn routing_wire_pieces(&mut self, scene: SceneId, conditions: bool) -> Vec<WrittenPiece> {
        let mut pieces = Vec::new();
        for net in self.scenes[scene].nets.clone() {
            let mut wires = Vec::new();
            let mut pins = Vec::new();
            let mut parents = Vec::new();
            let mut at: HashMap<ObjId, usize> = HashMap::new();
            for wire in self.scenes[scene].wires_by_net.get(&net).cloned().unwrap_or_default() {
                if conditions && !matches!(self.o(wire), ObjData::Wire { generated: false, .. }) {
                    continue;
                }
                if !conditions && self.is_obj_failed(wire) {
                    continue;
                }
                let shape =
                    if conditions { self.wire_shape_of(scene, wire, false) } else { self.wire_shape(scene, wire) };
                let Some(shape) = shape else { continue };
                let index = wires.len();
                wires.push(wire);
                parents.push(index);
                let mut wire_pins = Vec::new();
                for point in shape.points {
                    let key = self.wire_point_handle(scene, point);
                    if matches!(self.o(key), ObjData::ScenePin { .. }) && !wire_pins.contains(&key) {
                        wire_pins.push(key);
                    }
                    if let Some(&other) = at.get(&key) {
                        let a = root(&mut parents, index);
                        let b = root(&mut parents, other);
                        parents[a] = b;
                    } else {
                        at.insert(key, index);
                    }
                }
                pins.push(wire_pins);
            }
            let mut roots = Vec::new();
            let mut grouped: Vec<WrittenPiece> = Vec::new();
            for (index, wire) in wires.into_iter().enumerate() {
                let key = root(&mut parents, index);
                let at = match roots.iter().position(|value| *value == key) {
                    Some(at) => at,
                    None => {
                        roots.push(key);
                        grouped.push(WrittenPiece { net, pins: Vec::new(), wires: Vec::new() });
                        grouped.len() - 1
                    }
                };
                grouped[at].wires.push(wire);
                for &pin in &pins[index] {
                    if !grouped[at].pins.contains(&pin) {
                        grouped[at].pins.push(pin);
                    }
                }
            }
            pieces.extend(grouped);
        }
        pieces
    }

    pub(super) fn routing_pin_wired(&mut self, scene: SceneId, object: ObjId) -> bool {
        let point = self.wire_point_handle(scene, object);
        self.scenes[scene].condition_trees.iter().any(|tree| tree.pins.contains(&point))
    }

    /// 0.4 counts host terminals and routing trees. A virtual part alone never makes its net
    /// wired; generated explicit wires are already excluded by wire_ends (10.2).
    pub(super) fn routing_net_wired(&mut self, scene: SceneId, net: ObjId, ends: &HashMap<ObjId, Vec<ObjId>>) -> bool {
        if self.scenes[scene].condition_trees.iter().any(|tree| tree.net == net) {
            return true;
        }
        for owner in self.inst_like(scene) {
            for terminal in self.terms_of(scene, owner) {
                if self.tnet_has(scene, terminal, net) && self.wired(scene, terminal, ends) {
                    return true;
                }
            }
        }
        false
    }
}

struct RankedCover {
    cover: RouteCover,
    join: bool,
    priority: Priority,
    pos: Pos,
}

impl Interp<'_> {
    fn routing_net_value(
        &mut self,
        scene: SceneId,
        input: ObjId,
        frozen: &HashMap<ObjId, Vec<InputMember>>,
    ) -> Option<Option<ObjId>> {
        if self.is_empty_node(input) {
            return Some(None);
        }
        let value = self.routing_value(scene, input, frozen)?;
        if value.members.is_empty() && !value.set {
            return Some(None);
        }
        if value.set || value.members.len() != 1 {
            return None;
        }
        let converted = self.routing_conversion(scene, value.members[0], "net")?;
        if converted.members.is_empty() && !converted.set {
            return Some(None);
        }
        if converted.set || converted.members.len() != 1 {
            return None;
        }
        Some(Some(converted.members[0]))
    }

    fn routing_sources(&mut self, scene: SceneId) -> Vec<RoutingRequest> {
        for index in 0..self.scenes[scene].routing_requests.len() {
            if self.scenes[scene].routing_requests[index].captured {
                continue;
            }
            let request = self.scenes[scene].routing_requests[index].clone();
            let members = if request.spec.net {
                vec![InputMember::plain(request.input)]
            } else {
                self.routing_members(scene, request.input, &request.site)
            };
            let frozen = self.capture_routing_sets(scene, request.input, &request.site);
            let saved = &mut self.scenes[scene].routing_requests[index];
            saved.members = members;
            saved.frozen_sets = frozen;
            saved.captured = true;
        }
        let mut sources = self.scenes[scene].routing_requests.clone();
        let modern: HashSet<_> = sources.iter().filter_map(|request| request.object).collect();
        for &route in &self.scenes[scene].routes {
            if modern.contains(&route) {
                continue;
            }
            let ObjData::Route { net, mode, style, site: Some(site) } = self.o(route) else { continue };
            let Some(pos) = self.scenes[scene].created_order.get(&route) else { continue };
            sources.push(RoutingRequest {
                input: *net,
                members: vec![InputMember::plain(*net)],
                frozen_sets: HashMap::new(),
                captured: true,
                spec: RoutingSpec { net: true, join: true, mode: mode.clone(), style: style.clone(), priority: None },
                pos: pos.clone(),
                site: site.clone(),
                chain: self.apply_chain(site),
                off: 0,
                object: Some(route),
            });
        }
        sources
    }

    /// Resolve scopes against the current symbols. Collection membership is the captured input;
    /// conversions may still resolve a pin to its net without revisiting creation.
    fn routing_covers(&mut self, scene: SceneId, report: bool) -> (Vec<RankedCover>, Vec<RoutingRequest>) {
        let sources = self.routing_sources(scene);
        let terms = self.route_terms(scene);
        let mut by_net: HashMap<ObjId, Vec<ObjId>> = HashMap::new();
        for &net in &self.scenes[scene].nets.clone() {
            let mut pins = Vec::new();
            for &(owner, object, _) in terms.get(&net).into_iter().flatten() {
                let candidates = if matches!(self.o(object), ObjData::ScenePin { .. }) {
                    vec![object]
                } else {
                    self.pins_for(scene, owner, object)
                        .into_iter()
                        .map(|index| self.scene_pin_object(scene, owner, index))
                        .collect()
                };
                for pin in candidates {
                    if self.has_pin(scene, pin) && !pins.contains(&pin) {
                        pins.push(pin);
                    }
                }
            }
            by_net.insert(net, pins);
        }
        let mut covers = Vec::new();
        'request: for (first, source) in sources.iter().enumerate() {
            let priority = self.eff_priority(scene, source.spec.priority, &source.chain, source.object);
            if priority == Priority::Disabled
                || self.failed_body(&source.site)
                || self.is_failed(source.input)
                || source.members.iter().any(|member| {
                    self.is_failed(member.object) || member.dependencies.iter().any(|object| self.is_failed(*object))
                })
            {
                continue;
            }
            if source.spec.net {
                let Some(net) = self.routing_net_value(scene, source.input, &source.frozen_sets) else {
                    if report && self.scenes[scene].routing_failed.insert(first) {
                        let diagnostic = self.err(
                            &source.site,
                            codes::ROUTE_TARGET_NOT_NET,
                            format!(
                                "a net routing keyword requires one net; {} cannot convert to one net (5.2.4, 13.7)",
                                self.label(source.input)
                            ),
                            source.off,
                        );
                        self.note_failed_at(source.site.rec, source.off, false);
                        if let Some(object) = source.object {
                            self.late_failures.insert(object, diagnostic);
                        }
                    }
                    continue;
                };
                let Some(net) = net else { continue };
                covers.push(RankedCover {
                    cover: RouteCover {
                        net,
                        pins: by_net.get(&net).cloned().unwrap_or_default(),
                        mode: source.spec.mode.clone(),
                        style: source.spec.style.clone(),
                        first,
                    },
                    join: source.spec.join,
                    priority,
                    pos: source.pos.clone(),
                });
            } else {
                let mut selected = HashSet::new();
                let mut seen = HashSet::new();
                for member in &source.members {
                    if !self.routing_object_pins(scene, member.object, &source.frozen_sets, &mut seen, &mut selected) {
                        continue 'request;
                    }
                }
                for net in self.scenes[scene].nets.clone() {
                    let pins: Vec<_> =
                        by_net.get(&net).into_iter().flatten().filter(|pin| selected.contains(pin)).copied().collect();
                    if source.spec.join && pins.len() < 2 || pins.is_empty() {
                        continue;
                    }
                    covers.push(RankedCover {
                        cover: RouteCover {
                            net,
                            pins,
                            mode: source.spec.mode.clone(),
                            style: source.spec.style.clone(),
                            first,
                        },
                        join: source.spec.join,
                        priority,
                        pos: source.pos.clone(),
                    });
                }
            }
        }
        covers.sort_by(|a, b| a.pos.cmp(&b.pos));
        (covers, sources)
    }

    /// Resolve every positive request separately before merging connected trees.
    /// A stronger positive request cannot undo an exclusion from another request.
    pub(super) fn resolve_routing_trees(&mut self, scene: SceneId, conditions: bool, report: bool) -> Vec<RouteTree> {
        let (ranked, sources) = self.routing_covers(scene, report);
        let exclusions: Vec<_> = ranked
            .iter()
            .filter(|request| !request.join)
            .map(|request| UnrouteCover {
                net: request.cover.net,
                pins: request.cover.pins.iter().copied().collect(),
                priority: request.priority,
                order: request.pos.clone(),
            })
            .collect();
        let mut covers = Vec::new();
        for mut request in ranked.into_iter().filter(|request| request.join) {
            let original = request.cover.pins.clone();
            remove_unrouted_pins(&mut request.cover, request.priority, &request.pos, &exclusions);
            if report {
                let source = &sources[request.cover.first];
                let authored = source
                    .site
                    .rec
                    .is_some_and(|rec| self.units[self.rec_unit[rec]].kind != super::unit::UnitKind::Package)
                    && source.site.header.type_ != "generated";
                if authored {
                    let counted: Vec<_> = original
                        .iter()
                        .filter(|pin| match self.o(**pin) {
                            ObjData::ScenePin { owner, .. } => !self.is_generated(*owner),
                            _ => true,
                        })
                        .collect();
                    let removed = counted.iter().filter(|pin| !request.cover.pins.contains(pin)).count();
                    if removed > 0 {
                        let total = counted.len();
                        let noun = if total == 1 { "pin" } else { "pins" };
                        let kind = if source.spec.net { "ROUTE-NET" } else { "ROUTE-OBJECT" };
                        self.diag_site(
                            crate::diag::Severity::Hint,
                            codes::ROUTE_OVERRULED,
                            &Some(source.site.clone()),
                            source.off,
                            format!(
                                "UNROUTE removes {removed} of {total} {noun} from this {kind} tree on {} (13.7)",
                                self.net_display(request.cover.net)
                            ),
                        );
                    }
                }
            }
            if !request.cover.pins.is_empty() {
                covers.push(request.cover);
            }
        }
        let pieces = self.routing_wire_pieces(scene, conditions);
        let mut trees = merge_covers(&covers, &pieces);
        trees.retain(|tree| {
            let mut points: HashSet<_> = tree.pins.iter().copied().collect();
            for &wire in &tree.wires {
                let shape =
                    if conditions { self.wire_shape_of(scene, wire, false) } else { self.wire_shape(scene, wire) };
                if let Some(shape) = shape {
                    points.extend(shape.points.into_iter().map(|point| self.wire_point_handle(scene, point)));
                }
            }
            points.len() >= 2
        });
        if report {
            for tree in &trees {
                if tree.restyled {
                    let source = &sources[tree.first];
                    self.diag_site(crate::diag::Severity::Hint, codes::ROUTE_TREES_DIFFERENT_STYLES,
                        &Some(source.site.clone()), source.off,
                        format!("joined routing trees on {} have different styles; the first routing statement supplies the style (13.7)", self.net_display(tree.net)));
                }
            }
        }
        trees
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::obj::Lit;

    fn cover(net: u32, pins: &[u32], first: usize) -> RouteCover {
        RouteCover {
            net: ObjId(net),
            pins: pins.iter().copied().map(ObjId).collect(),
            mode: "auto".into(),
            style: StyleMap::new(),
            first,
        }
    }

    #[test]
    fn a_shared_net_name_does_not_merge_disjoint_trees() {
        let trees = merge_covers(&[cover(1, &[10, 11], 0), cover(1, &[12, 13], 1)], &[]);
        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].pins, [ObjId(10), ObjId(11)]);
        assert_eq!(trees[1].pins, [ObjId(12), ObjId(13)]);
    }

    #[test]
    fn shared_pins_merge_transitively_and_keep_the_first_style() {
        let mut covers = vec![cover(1, &[10, 11], 0), cover(1, &[12, 13], 1), cover(1, &[11, 12], 2)];
        covers[0].mode = "straight".into();
        covers[1].style.insert("width".into(), Lit::Num(2.0));
        let trees = merge_covers(&covers, &[]);
        assert_eq!(trees.len(), 1);
        assert_eq!(trees[0].pins, [ObjId(10), ObjId(11), ObjId(12), ObjId(13)]);
        assert_eq!(trees[0].first, 0);
        assert_eq!(trees[0].mode, "straight");
        assert!(trees[0].style.is_empty());
        assert!(trees[0].restyled);
    }

    #[test]
    fn connected_written_wires_merge_only_the_trees_they_touch() {
        let covers = [cover(1, &[10, 11], 0), cover(1, &[12, 13], 1), cover(1, &[14, 15], 2)];
        let pieces =
            [WrittenPiece { net: ObjId(1), pins: vec![ObjId(11), ObjId(12)], wires: vec![ObjId(20), ObjId(21)] }];
        let trees = merge_covers(&covers, &pieces);
        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].wires, [ObjId(20), ObjId(21)]);
        assert!(trees[1].wires.is_empty());
    }

    #[test]
    fn a_bus_pin_does_not_merge_trees_on_different_nets() {
        let trees = merge_covers(&[cover(1, &[10, 11], 0), cover(2, &[10, 12], 1)], &[]);
        assert_eq!(trees.len(), 2);
    }

    fn order(index: i64) -> Pos {
        vec![super::super::obj::PosElem::Int(index)]
    }

    fn unroute(net: u32, pins: &[u32], priority: Priority, position: i64) -> UnrouteCover {
        UnrouteCover {
            net: ObjId(net),
            pins: pins.iter().copied().map(ObjId).collect(),
            priority,
            order: order(position),
        }
    }

    #[test]
    fn a_stronger_route_does_not_restore_pins_removed_from_a_weaker_route() {
        let exclusions = [unroute(1, &[11, 12], Priority::Normal, 1)];
        let mut whole = cover(1, &[10, 11, 12, 13], 0);
        let mut pair = cover(1, &[11, 12], 2);
        remove_unrouted_pins(&mut whole, Priority::Low, &order(0), &exclusions);
        remove_unrouted_pins(&mut pair, Priority::High, &order(2), &exclusions);
        let trees = merge_covers(&[whole, pair], &[]);
        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].pins, [ObjId(10), ObjId(13)]);
        assert_eq!(trees[1].pins, [ObjId(11), ObjId(12)]);
    }

    #[test]
    fn equal_priority_exclusions_only_remove_pins_from_earlier_routes() {
        let exclusions = [unroute(1, &[10], Priority::Normal, 1)];
        let mut earlier = cover(1, &[10, 11], 0);
        let mut later = cover(1, &[10, 12], 2);
        remove_unrouted_pins(&mut earlier, Priority::Normal, &order(0), &exclusions);
        remove_unrouted_pins(&mut later, Priority::Normal, &order(2), &exclusions);
        assert_eq!(earlier.pins, [ObjId(11)]);
        assert_eq!(later.pins, [ObjId(10), ObjId(12)]);
    }

    #[test]
    fn disabled_exclusions_and_other_net_bits_do_not_remove_a_bus_pin() {
        let exclusions = [unroute(1, &[10], Priority::Disabled, 9), unroute(2, &[10], Priority::High, 9)];
        let mut routed = cover(1, &[10, 11], 0);
        remove_unrouted_pins(&mut routed, Priority::Low, &order(0), &exclusions);
        assert_eq!(routed.pins, [ObjId(10), ObjId(11)]);
        remove_unrouted_pins(&mut routed, Priority::Disabled, &order(0), &exclusions);
        assert!(routed.pins.is_empty());
    }
}
