//! Circuit and drawing selections of 0.4 (10.1).

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::host::glob;

use super::ctx::Ctx;
use super::obj::*;
use super::scene::name_match;
use super::Interp;

impl Interp<'_> {
    pub(super) fn writes_selection(&self, member: &crate::syntax::Member, ctx: &Ctx) -> bool {
        member.segments.iter().any(|segment| {
            if let Some(call) = &segment.call {
                let keyword = self.kw(&call.kw, ctx.unit);
                if keyword.starts_with("SELECT-") {
                    return true;
                }
                if matches!(keyword.as_str(), "DEFINE" | "DEFINE-SYMBOL" | "DEFINE-STYLE" | "USE-PACKAGE") {
                    return false;
                }
            }
            segment.args.iter().any(|arg| match &arg.obj {
                crate::syntax::ObjExpr::Set(set) => set.members.iter().any(|member| self.writes_selection(member, ctx)),
                _ => false,
            })
        })
    }

    /// A recursive selection written anywhere in a pipe reaches the lower levels for the
    /// whole statement; a later SELECT-PIN must not narrow that reach to its own level (10.1).
    pub(super) fn writes_recursive_selection(&self, member: &crate::syntax::Member, ctx: &Ctx) -> bool {
        use crate::syntax::{ObjExpr, Value};
        member.segments.iter().any(|segment| {
            if let Some(call) = &segment.call {
                let keyword = self.kw(&call.kw, ctx.unit);
                if keyword.starts_with("SELECT-")
                    && call.params.as_ref().is_some_and(|params| {
                        params.named.iter().any(|param| {
                            let name = if param.quoted { param.name.clone() } else { self.word(&param.name, ctx.unit) };
                            name == "recursive"
                                && match &param.value {
                                    Value::Bool(value) => *value,
                                    Value::Word(word) => self.word(word, ctx.unit) == "true",
                                    Value::Hole(name) => ctx.frame.as_ref().is_some_and(|frame| {
                                        frame
                                            .borrow()
                                            .param(name)
                                            .and_then(|value| value.as_ref())
                                            .and_then(Lit::boolean)
                                            == Some(true)
                                    }),
                                    _ => false,
                                }
                        })
                    })
                {
                    return true;
                }
                // An encapsulated body is another statement scope (5.4, 9.4).
                if matches!(keyword.as_str(), "DEFINE" | "DEFINE-SYMBOL" | "DEFINE-STYLE" | "USE-PACKAGE") {
                    return false;
                }
            }
            segment.args.iter().any(|arg| match &arg.obj {
                ObjExpr::Set(set) => set.members.iter().any(|member| self.writes_recursive_selection(member, ctx)),
                _ => false,
            })
        })
    }

    /// A symbol queue cannot read the result of assigning symbols, even through named sets.
    pub(super) fn symbol_selection_dependency(&mut self, object: ObjId) -> bool {
        #[derive(Clone, PartialEq, Eq, Hash)]
        struct Scope {
            keyword: String,
            role: Option<String>,
            unit: UnitId,
        }
        fn visit(
            it: &mut Interp<'_>,
            object: ObjId,
            scope: Option<Scope>,
            seen: &mut HashSet<(ObjId, Option<Scope>)>,
        ) -> bool {
            if !seen.insert((object, scope.clone())) {
                return false;
            }
            if let Some(input) = it.role_inputs.get(&object).cloned() {
                if scope.is_some()
                    && matches!(it.o(input.source), ObjData::Term { .. })
                    && !input.kinds.iter().any(|kind| kind == it.kind_of(input.source))
                    && input.kinds.iter().any(|kind| matches!(kind.as_str(), "pin" | "point"))
                {
                    return true;
                }
                return visit(it, input.source, scope, seen);
            }
            match it.o(object).clone() {
                ObjData::NamedSubobject { .. } | ObjData::ScenePin { .. } if scope.is_some() => true,
                ObjData::Pos { parent } if scope.is_some() => {
                    matches!(it.o(parent), ObjData::Term { .. } | ObjData::ScenePin { .. })
                        || (matches!(it.o(parent), ObjData::NamedSubobject { .. } | ObjData::Pos { .. })
                            || it.role_inputs.contains_key(&parent))
                            && visit(it, parent, scope, seen)
                }
                ObjData::Set(set) => {
                    let blocked = match &set.kind {
                        SetKind::Select { keyword, scope: selected_scope, params, writer, .. } => {
                            let selected = Scope {
                                keyword: keyword.clone(),
                                role: params.get("role").and_then(Lit::str).map(String::from),
                                unit: match params.get("role") {
                                    Some(Lit::Str(s)) => s.unit.unwrap_or(*writer),
                                    _ => *writer,
                                },
                            };
                            it.follows_0_4(*writer)
                                && (keyword == "SELECT-PIN"
                                    || params.contains_key("symbol")
                                    || params.contains_key("drawn")
                                    || visit(it, *selected_scope, Some(selected), seen))
                        }
                        SetKind::Union { source } => visit(it, *source, scope.clone(), seen),
                        _ => false,
                    };
                    blocked
                        || set.items.iter().any(|(item, _)| visit(it, *item, scope.clone(), seen))
                        || set.mods.iter().any(|edit| visit(it, edit.source, scope.clone(), seen))
                }
                ObjData::Usage(usage) => {
                    let Some(mut scope) = scope else { return false };
                    if matches!(scope.keyword.as_str(), "SELECT-USAGE" | "SELECT-ANCESTOR-USAGE") {
                        return false;
                    }
                    let want = scope.role.take().map(|role| {
                        if usage.role(&role).is_some() {
                            role
                        } else {
                            it.scoped_for(scope.unit, usage.definition)
                                .iter()
                                .find(|(alias, _)| alias == &role)
                                .map(|(_, name)| name.clone())
                                .unwrap_or(role)
                        }
                    });
                    usage
                        .roles
                        .iter()
                        .filter(|(name, _)| want.as_ref().is_none_or(|want| want == name))
                        .any(|(_, filler)| visit(it, *filler, Some(scope.clone()), seen))
                }
                _ => false,
            }
        }
        visit(self, object, None, &mut HashSet::new())
    }

    pub(super) fn check_symbol_selection(
        &mut self,
        scene: SceneId,
        object: ObjId,
        ctx: &Ctx,
        off: usize,
    ) -> super::eval::R<()> {
        if !self.follows_0_4(self.source_unit(ctx)) || !self.symbol_selection_dependency(object) {
            return Ok(());
        }
        let first = self.scenes[scene].bad_symbol_selections.insert((ctx.rec, off));
        if first {
            self.fail(ctx, crate::diag::codes::SELECT_SYMBOL_DEPENDENCY,
                "a selection of SET-SYMBOL depends on symbols; SELECT-PIN, symbol=, drawn= and scopes through pins cannot be used in the symbol queue (8.2, 10.1)", off)
        } else {
            Err(super::eval::Stop::Fail(None))
        }
    }

    pub(crate) fn object_drawn(&self, scene: SceneId, object: ObjId) -> bool {
        !self.is_failed(object)
            && self.scenes[scene].symbol.contains_key(&object)
            && !self.scenes[scene].default_symbol.contains(&object)
    }

    pub(crate) fn symbol_matches(&self, scene: SceneId, object: ObjId, pattern: &str) -> bool {
        self.object_drawn(scene, object)
            && self.scenes[scene].symbol_names.get(&object).is_some_and(|name| glob(pattern, name, true))
    }

    pub(super) fn selection_symbol_matches(
        &self,
        scene: SceneId,
        object: ObjId,
        params: &BTreeMap<String, Lit>,
    ) -> bool {
        params.get("drawn").and_then(Lit::boolean).is_none_or(|want| self.object_drawn(scene, object) == want)
            && params.get("symbol").and_then(Lit::str).is_none_or(|pattern| self.symbol_matches(scene, object, pattern))
    }

    /// The pieces of each net that a selection sees: in step 6, those fixed as it begins
    /// ([`Self::net_pieces`]), before anything it makes.
    pub(super) fn pieces_seen(&mut self, scene: SceneId, snapshot: bool) -> HashMap<ObjId, Vec<Vec<ObjId>>> {
        match &self.scenes[scene].selection_pieces {
            Some(pieces) if snapshot => pieces.clone(),
            _ => self.net_pieces(scene, snapshot),
        }
    }

    /// 10.2: the pieces of each net, as `wired` sees the drawing. The pins on a net, those of
    /// virtual parts included, that one routing tree joins (13.7) or that written wires join are
    /// one piece; a pin in neither is a piece by itself. Each piece lists its pins in order, its
    /// first pin first: the pins of virtual parts by the order of the positions of their usages
    /// (8.5), then those of instances by the order of their lines and then of their terminal
    /// numbers.
    pub(super) fn net_pieces(&mut self, scene: SceneId, snapshot: bool) -> HashMap<ObjId, Vec<Vec<ObjId>>> {
        fn find(up: &mut HashMap<ObjId, ObjId>, x: ObjId) -> ObjId {
            let mut root = x;
            while let Some(&next) = up.get(&root).filter(|next| **next != root) {
                root = next;
            }
            up.insert(x, root);
            root
        }
        fn union(up: &mut HashMap<ObjId, ObjId>, a: ObjId, b: ObjId) {
            let (a, b) = (find(up, a), find(up, b));
            if a != b {
                up.insert(b, a);
            }
        }
        // the pins of each net, in order: those of virtual parts first, in the order of selection,
        // which is that of the positions of their usages; then those of instances, in the order
        // of their lines and of the lowest terminal number each pin draws
        let owners = self.selection_drawables(scene, snapshot, true);
        // (virtual part or instance, place, lowest terminal, pin index), net, pin
        type Ordered = ((bool, usize, u32, usize), ObjId, ObjId);
        let mut ordered: Vec<Ordered> = Vec::new();
        for (rank, owner) in owners.into_iter().enumerate() {
            let instance = matches!(self.o(owner), ObjData::Inst { .. } | ObjData::Bundle { .. });
            for pin in self.owner_pins(scene, owner) {
                let ObjData::ScenePin { owner, index } = *self.o(pin) else { continue };
                let terminal = self
                    .selected_pin_terminals(scene, owner, index)
                    .into_iter()
                    .filter_map(|t| match self.o(t) {
                        ObjData::Term { index, .. } => Some(*index),
                        _ => None,
                    })
                    .min()
                    .unwrap_or(u32::MAX);
                for net in self.scene_pin_nets(scene, owner, index) {
                    ordered.push(((instance, rank, terminal, index), net, pin));
                }
            }
        }
        ordered.sort_by_key(|(key, _, _)| *key);
        let mut up: HashMap<ObjId, ObjId> = HashMap::new();
        // written wires join what they pass, a named point they share included; the wires that
        // step 6 generates are not seen, as for `wired`
        for w in self.scenes[scene].wires.clone() {
            if !matches!(self.o(w), ObjData::Wire { generated: false, .. }) {
                continue;
            }
            let Some(shape) = self.wire_shape_of(scene, w, false) else { continue };
            let points: Vec<ObjId> = shape.points.iter().map(|q| self.wire_point_handle(scene, *q)).collect();
            for pair in points.windows(2) {
                union(&mut up, pair[0], pair[1]);
            }
        }
        // a tree joins its pins; trees that share a pin are one
        for tree in self.scenes[scene].condition_trees.clone() {
            for pair in tree.pins.windows(2) {
                union(&mut up, pair[0], pair[1]);
            }
        }
        let mut pieces: HashMap<ObjId, Vec<Vec<ObjId>>> = HashMap::new();
        let mut roots: HashMap<(ObjId, ObjId), usize> = HashMap::new();
        for (_, net, pin) in ordered {
            let root = find(&mut up, pin);
            let of_net = pieces.entry(net).or_default();
            match roots.get(&(net, root)) {
                Some(&k) => of_net[k].push(pin),
                None => {
                    roots.insert((net, root), of_net.len());
                    of_net.push(vec![pin]);
                }
            }
        }
        pieces
    }

    fn owner_pins(&mut self, scene: SceneId, owner: ObjId) -> Vec<ObjId> {
        if !self.object_drawn(scene, owner) {
            return Vec::new();
        }
        let symbol = self.scenes[scene].symbol[&owner];
        (0..self.symbol(symbol).pins.len()).map(|index| self.scene_pin_object(scene, owner, index)).collect()
    }

    pub(super) fn selection_value(&mut self, object: ObjId, phase: usize) -> ObjId {
        if phase >= 5 && self.role_inputs.get(&object).is_some_and(|input| !input.collection) {
            self.resolve_role_input(object);
        }
        self.scene_value(object)
    }

    /// Ancestors compare the same post-symbol role values as their selection scope (10.1).
    /// This consumer walk never rewrites the handles used by earlier creation or set edits.
    pub(super) fn selected_usage_members(
        &mut self,
        scene: SceneId,
        usage: ObjId,
        phase: usize,
        ctx: Option<&Ctx>,
    ) -> Vec<ObjId> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![usage];
        while let Some(object) = stack.pop() {
            let object = if phase >= 5 { self.selection_value(object, phase) } else { self.written_input(object) };
            if self.is_failed(object) || !seen.insert(object) {
                continue;
            }
            if self.is_set(object) {
                for input in self.scene_input_members_at(scene, object, phase, ctx) {
                    if !input.dependencies.iter().any(|dependency| self.is_failed(*dependency)) {
                        stack.push(input.object);
                    }
                }
            } else {
                if object != usage {
                    out.push(object);
                }
                if let ObjData::Usage(member) = self.o(object).clone() {
                    for (_, filler) in member.roles {
                        for input in self.scene_input_members_at(scene, filler, phase, ctx) {
                            if !input.dependencies.iter().any(|dependency| self.is_failed(*dependency)) {
                                stack.push(input.object);
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// Scope usages contribute role members of the requested kind, never another scope step.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn walk_selection_usages(
        &mut self,
        scene: SceneId,
        pool: &[ObjId],
        role: Option<&Lit>,
        writer: UnitId,
        phase: usize,
        ctx: Option<&Ctx>,
        keyword: &str,
    ) -> Vec<ObjId> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut stack: Vec<_> = pool.iter().rev().map(|object| (*object, true)).collect();
        let unit = match role {
            Some(Lit::Str(value)) => value.unit.unwrap_or(writer),
            _ => writer,
        };
        while let Some((object, top)) = stack.pop() {
            let object = self.selection_value(object, phase);
            if self.is_set(object) {
                let members = self.scene_input_members_at(scene, object, phase, ctx);
                stack.extend(members.into_iter().rev().map(|member| (member.object, top)));
                continue;
            }
            if self.is_failed(object) {
                continue;
            }
            if let ObjData::Usage(usage) = self.o(object).clone() {
                if top && keyword == "SELECT-PIN" && role.is_none() && self.object_drawn(scene, object) {
                    for pin in self.owner_pins(scene, object) {
                        out.push(pin);
                    }
                    continue;
                }
                if !seen.insert((object, top)) {
                    continue;
                }
                let want = if top {
                    role.and_then(Lit::str).map(|name| {
                        if usage.role(name).is_some() {
                            name.to_string()
                        } else {
                            self.scoped_for(unit, usage.definition)
                                .iter()
                                .find(|(alias, _)| alias == name)
                                .map(|(_, original)| original.clone())
                                .unwrap_or_else(|| name.to_string())
                        }
                    })
                } else {
                    None
                };
                let mut children = Vec::new();
                for (name, filler) in usage.roles {
                    if want.as_ref().is_some_and(|want| want != &name) {
                        continue;
                    }
                    for input in self.scene_input_members_at(scene, filler, phase, ctx) {
                        if !input.dependencies.iter().any(|dependency| self.is_failed(*dependency)) {
                            children.push((input.object, false));
                        }
                    }
                }
                stack.extend(children.into_iter().rev());
            } else if top
                || match keyword {
                    "SELECT-INSTANCE" => self.obj_kind(object) == Some("instance"),
                    "SELECT-TERMINAL" => matches!(self.o(object), ObjData::Term { .. }),
                    "SELECT-NET" => matches!(self.o(object), ObjData::Net { .. }),
                    "SELECT-PIN" => self.obj_kind(object) == Some("pin"),
                    "SELECT-WIRE" => self.obj_kind(object) == Some("wire"),
                    "SELECT-LABEL" => self.obj_kind(object) == Some("label"),
                    "SELECT-ANNOTATION" => self.obj_kind(object) == Some("annotation"),
                    _ => false,
                }
            {
                // Decide for this occurrence: the same object can occur later as a direct
                // scope, without making this role member take an extra step (10.1, 10.3).
                out.push(object);
            }
        }
        out
    }

    pub(super) fn selected_pin_terminals(&mut self, scene: SceneId, owner: ObjId, index: usize) -> Vec<ObjId> {
        if !matches!(self.o(owner), ObjData::Inst { .. } | ObjData::Bundle { .. }) {
            return Vec::new();
        }
        self.terms_of(scene, owner)
            .into_iter()
            .filter(|terminal| self.pins_for(scene, owner, *terminal).contains(&index))
            .collect()
    }

    pub(super) fn select_pins(
        &mut self,
        scene: SceneId,
        pool: Option<Vec<ObjId>>,
        via: &HashSet<ObjId>,
        params: &BTreeMap<String, Lit>,
        snapshot: bool,
        writer: UnitId,
    ) -> Vec<ObjId> {
        let mut all = Vec::new();
        for owner in self.selection_drawables(scene, snapshot, true) {
            all.extend(self.owner_pins(scene, owner));
        }
        let mut candidates = Vec::new();
        if let Some(pool) = pool {
            for object in pool {
                match self.o(object).clone() {
                    ObjData::ScenePin { .. } => candidates.push(object),
                    _ if via.contains(&object) => {}
                    ObjData::Inst { .. } | ObjData::Bundle { .. } => {
                        let owner = self.bundle_of(object);
                        candidates.extend(self.owner_pins(scene, owner));
                    }
                    ObjData::Term { owner, .. } if self.object_drawn(scene, owner) => {
                        for index in self.pins_for(scene, owner, object) {
                            candidates.push(self.scene_pin_object(scene, owner, index));
                        }
                    }
                    ObjData::Net { .. } => {
                        for &pin in &all {
                            let ObjData::ScenePin { owner, index } = *self.o(pin) else { continue };
                            if self.scene_pin_nets(scene, owner, index).contains(&object) {
                                candidates.push(pin);
                            }
                        }
                    }
                    _ => {}
                }
            }
        } else {
            candidates = all;
        }
        let ends = if params.contains_key("wired") { self.wire_ends(scene) } else { Default::default() };
        let firsts: HashSet<ObjId> = if params.contains_key("piece_first") {
            self.pieces_seen(scene, snapshot)
                .into_values()
                .flatten()
                .filter_map(|piece| piece.first().copied())
                .collect()
        } else {
            HashSet::new()
        };
        let mut seen = HashSet::new();
        candidates
            .into_iter()
            .filter(|pin| {
                if !seen.insert(*pin) {
                    return false;
                }
                let ObjData::ScenePin { owner, index } = *self.o(*pin) else { return false };
                if !self.object_drawn(scene, owner) || !self.inst_match(owner, params, true, writer) {
                    return false;
                }
                if let Some(want) = params.get("terminal") {
                    // in 0.4 a string of digits is the terminal number (6.4)
                    let want = &if self.follows_0_4(writer) {
                        super::terminal_maps::terminal_value(want)
                    } else {
                        want.clone()
                    };
                    let matched = if let ObjData::Usage(_) = self.o(owner) {
                        let symbol = self.scenes[scene].symbol[&owner];
                        self.symbol(symbol).pins[index].terms.iter().any(|term| match (term, want) {
                            (TermKey::Name(name), Lit::Str(want)) => name == want.as_str(),
                            _ => false,
                        })
                    } else {
                        self.selected_pin_terminals(scene, owner, index).iter().any(|terminal| {
                            let ObjData::Term { index, name, .. } = self.o(*terminal) else { return false };
                            match want {
                                Lit::Num(want) => *index as f64 == *want,
                                Lit::Str(want) => name.as_ref().is_some_and(|name| {
                                    if self.host_case(owner) {
                                        name == want.as_str()
                                    } else {
                                        name.eq_ignore_ascii_case(want.as_str())
                                    }
                                }),
                                _ => false,
                            }
                        })
                    };
                    if !matched {
                        return false;
                    }
                }
                if let Some(pattern) = params.get("net").and_then(Lit::str) {
                    let matched = self.scene_pin_nets(scene, owner, index).iter().any(|net| match self.o(*net) {
                        ObjData::Net { key, .. } => name_match(self, self.scenes[scene].level, pattern, key),
                        _ => false,
                    });
                    if !matched {
                        return false;
                    }
                }
                if params.get("piece_first").and_then(Lit::boolean).is_some_and(|want| firsts.contains(pin) != want) {
                    return false;
                }
                params
                    .get("wired")
                    .and_then(Lit::boolean)
                    .is_none_or(|want| (self.wired(scene, *pin, &ends) || self.routing_pin_wired(scene, *pin)) == want)
            })
            .collect()
    }
}
