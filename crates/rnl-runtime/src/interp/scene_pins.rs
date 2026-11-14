//! Written named handles and their symbol-dependent scene values (5.2.4, 6.5).

use std::rc::Rc;

use crate::diag::{codes, Code};

use super::ctx::Ctx;
use super::eval::Stop;
use super::obj::{InputMember, ObjData, ObjId, SceneId, SetData, TermKey};
use super::Interp;

impl Interp<'_> {
    /// A pin reached directly or by a terminal's last bound pin, before requiring a direction.
    pub(crate) fn direction_pin(&mut self, scene: SceneId, object: ObjId) -> Option<(ObjId, usize, Option<u16>)> {
        let value = self.scene_value(object);
        let (owner, index) = match *self.o(value) {
            ObjData::ScenePin { owner, index } => (owner, index),
            ObjData::Term { owner, .. } => (owner, self.pin_for(scene, owner, value)?),
            _ => return None,
        };
        if matches!(self.o(owner), ObjData::Inst { bundle: Some(_), .. }) {
            return None;
        }
        let symbol = self.scenes[scene].symbol.get(&owner)?;
        let pin = self.symbol(*symbol).pins.get(index)?;
        Some((owner, index, pin.dir))
    }

    pub(crate) fn is_pin_direction_input(&self, object: ObjId) -> bool {
        matches!(self.o(self.scene_value(object)), ObjData::ScenePin { .. } | ObjData::Term { .. })
    }

    pub(crate) fn scene_pin_object(&mut self, scene: SceneId, owner: ObjId, index: usize) -> ObjId {
        if let Some(&pin) = self.scenes[scene].pins.get(&(owner, index)) {
            return pin;
        }
        let pin = self.new_obj(ObjData::ScenePin { owner, index });
        self.scenes[scene].pins.insert((owner, index), pin);
        pin
    }

    /// Physical connection identity. A position sub-object remains a point and never connects
    /// the pin at that position; a terminal in a wire stands for its last bound pin (5.2.4).
    pub(crate) fn wire_point_handle(&mut self, scene: SceneId, object: ObjId) -> ObjId {
        let object = self.scene_value(object);
        if let ObjData::Term { owner, .. } = *self.o(object) {
            if let Some(index) = self.pin_for(scene, owner, object) {
                return self.scene_pin_object(scene, owner, index);
            }
        }
        object
    }

    /// The host terminals bound to one physical pin. A usage has roles, not terminals (5.2.4).
    pub(crate) fn scene_pin_terminals(&mut self, scene: SceneId, owner: ObjId, index: usize) -> Vec<ObjId> {
        if matches!(self.o(owner), ObjData::Usage(_)) {
            return Vec::new();
        }
        self.terms_of(scene, owner)
            .into_iter()
            .filter(|terminal| self.pins_for(scene, owner, *terminal).contains(&index))
            .collect()
    }

    /// A role filled with no net is distinct from a pin with no binding at all (6.3, 12.1).
    pub(crate) fn scene_pin_has_empty_node(&self, scene: SceneId, owner: ObjId, index: usize) -> bool {
        self.pin_empty_node_inner(scene, owner, index, &mut std::collections::HashSet::new())
    }

    fn pin_empty_node_inner(
        &self,
        scene: SceneId,
        owner: ObjId,
        index: usize,
        visiting: &mut std::collections::HashSet<(ObjId, usize)>,
    ) -> bool {
        if !visiting.insert((owner, index)) {
            return false;
        }
        let result = (|| {
            let ObjData::Usage(usage) = self.o(owner) else { return false };
            let Some(symbol) = self.scenes[scene].symbol.get(&owner) else { return false };
            let Some(pin) = self.symbol(*symbol).pins.get(index) else { return false };
            pin.terms.iter().any(|binding| match binding {
                TermKey::Name(name) => {
                    usage.role(name).is_some_and(|role| self.member_empty_node(scene, role, visiting))
                }
                _ => false,
            })
        })();
        visiting.remove(&(owner, index));
        result
    }

    fn member_empty_node(
        &self,
        scene: SceneId,
        object: ObjId,
        visiting: &mut std::collections::HashSet<(ObjId, usize)>,
    ) -> bool {
        let object = self.scene_value(object);
        match self.o(object) {
            ObjData::EmptyNode { .. } => true,
            ObjData::NamedSubobject { parent, name, .. } if name == "net" => {
                self.member_empty_node(scene, *parent, visiting)
            }
            ObjData::ScenePin { owner, index } => self.pin_empty_node_inner(scene, *owner, *index, visiting),
            _ => false,
        }
    }

    pub(crate) fn scene_pin_nets(&mut self, scene: SceneId, owner: ObjId, index: usize) -> Vec<ObjId> {
        self.scene_pin_nets_inner(scene, owner, index, &mut std::collections::HashSet::new())
    }

    fn scene_pin_nets_inner(
        &mut self,
        scene: SceneId,
        owner: ObjId,
        index: usize,
        visiting: &mut std::collections::HashSet<(ObjId, usize)>,
    ) -> Vec<ObjId> {
        if !visiting.insert((owner, index)) {
            return Vec::new();
        }
        let mut nets = Vec::new();
        if let ObjData::Usage(usage) = self.o(owner).clone() {
            if let Some(&symbol) = self.scenes[scene].symbol.get(&owner) {
                if let Some(pin) = self.symbol(symbol).pins.get(index).cloned() {
                    for binding in pin.terms {
                        let TermKey::Name(name) = binding else { continue };
                        let Some(role) = usage.role(&name) else { continue };
                        let members = self.flatten(scene, role, self.scenes[scene].phase, None);
                        for member in members {
                            nets.extend(self.scene_member_nets(scene, member, visiting));
                        }
                    }
                }
            }
        } else {
            for terminal in self.terms_of(scene, owner) {
                if self.pins_for(scene, owner, terminal).contains(&index) {
                    nets.extend(self.term_nets(scene, terminal));
                }
            }
        }
        visiting.remove(&(owner, index));
        nets
    }

    fn scene_member_nets(
        &mut self,
        scene: SceneId,
        member: ObjId,
        visiting: &mut std::collections::HashSet<(ObjId, usize)>,
    ) -> Vec<ObjId> {
        // A role may refer to another pin's net before that reference has been resolved.
        // Follow its written parent, so resolution order does not change the result.
        if let ObjData::NamedSubobject { parent, name, .. } = self.o(member) {
            if name == "net" {
                return self.scene_member_nets(scene, *parent, visiting);
            }
        }
        let member = self.scene_value(member);
        match self.o(member).clone() {
            ObjData::Net { .. } => vec![member],
            ObjData::Term { .. } => self.term_nets(scene, member),
            ObjData::ScenePin { owner, index } => self.scene_pin_nets_inner(scene, owner, index, visiting),
            ObjData::Set(_) => {
                let mut nets = Vec::new();
                for child in self.flatten(scene, member, self.scenes[scene].phase, None) {
                    nets.extend(self.scene_member_nets(scene, child, visiting));
                }
                nets
            }
            _ => Vec::new(),
        }
    }

    pub(crate) fn scene_pin_is_bus(&self, scene: SceneId, owner: ObjId, index: usize) -> bool {
        let Some(symbol) = self.scenes[scene].symbol.get(&owner) else { return false };
        let Some(pin) = self.symbol(*symbol).pins.get(index) else { return false };
        pin.bus
            || pin.terms.len() > 1
            || matches!(self.o(owner), ObjData::Usage(usage) if pin.terms.iter().any(|term|
                matches!(term, TermKey::Name(name) if usage.role_sets.contains(name))))
    }

    /// A symbol may bind only electrically meaningful role fillers (12.1). This rejects the
    /// assignment, not the semantic usage or the independent objects made by its body.
    pub(crate) fn check_scene_pin_bindings(&mut self, scene: SceneId) -> bool {
        let mut removed = false;
        for usage in self.scenes[scene].usages.clone() {
            if !self.usage_own_point(usage) || self.is_failed(usage) {
                continue;
            }
            let Some(&symbol) = self.scenes[scene].symbol.get(&usage) else { continue };
            if !self.symbol(symbol).independent_pins {
                continue;
            }
            let data = self.usage(usage).clone();
            let Some(site) = &data.site else { continue };
            let mut faults = Vec::new();
            for (index, pin) in self.symbol(symbol).pins.clone().into_iter().enumerate() {
                'bindings: for binding in pin.terms {
                    let TermKey::Name(role) = binding else { continue };
                    let Some(filler) = data.role(&role) else { continue };
                    for member in self.scene_input_members_at(scene, filler, self.scenes[scene].phase, Some(site)) {
                        // A point role cannot bind a pin, even when written with a terminal.
                        // Read its target kind without checking whether a terminal has a pin:
                        // that later failure must not change the generation snapshot (8.2).
                        let point_role = self.role_input_converts_to_point(member.object);
                        let value = self.scene_value(member.object);
                        if !point_role
                            && matches!(
                                self.o(value),
                                ObjData::Net { .. }
                                    | ObjData::Term { .. }
                                    | ObjData::ScenePin { .. }
                                    | ObjData::NamedSubobject { resolved: None, .. }
                            )
                        {
                            continue;
                        }
                        faults.push((index, role, member.object, point_role));
                        break 'bindings;
                    }
                }
            }
            if faults.is_empty() {
                continue;
            }
            let assignment = self.scenes[scene]
                .appear
                .iter()
                .enumerate()
                .find(|(index, fact)| fact.target == usage && self.scenes[scene].symbol_won.contains(index))
                .map(|(_, fact)| fact.clone());
            let at = assignment
                .as_ref()
                .and_then(|fact| fact.ctx.as_ref().and_then(|ctx| self.rec_loc(ctx.rec, fact.off)))
                .map(|loc| format!("; the symbol is given at {}:{}:{}", loc.file, loc.line, loc.col))
                .unwrap_or_default();
            for (index, role, member, point_role) in faults {
                let (label, kind) = if point_role {
                    (format!("the position of {}", self.label(member)), "point")
                } else {
                    (self.label(member), self.kind_of(member))
                };
                self.err(site, codes::ROLE_PIN_FILLED_WITH_OTHER_KIND, format!(
                    "pin {} of {} binds role {role}, filled with {} ({}); a bound role takes a net, terminal or pin, so this symbol assignment fails (5.2.4, 12.1){at}",
                    index + 1, self.label(symbol), label, kind), data.off);
            }
            if let Some(fact) = assignment {
                if let Some(ctx) = fact.ctx {
                    self.note_failed_at(ctx.rec, fact.off, false);
                }
            }
            self.scenes[scene].symbol.remove(&usage);
            if let Some(order) = self.symbol_order.get_mut(&scene) {
                order.retain(|owner| *owner != usage);
            }
            // A binding may resolve to a keyed symbol text only after symbol_labels. Remove
            // the rejected symbol's text layers as well, while keeping explicit TEXT layers.
            for text in self.scenes[scene].texts.clone() {
                match self.om(text) {
                    ObjData::Text { attach, from_symbol: true, failed, .. } if *attach == usage => {
                        *failed = true;
                    }
                    ObjData::KeyText { attach, layers, from_symbol, sym_pos, failed, .. }
                        if *attach == usage && *from_symbol =>
                    {
                        layers.retain(|(order, _, _)| *order != super::obj::pos(&[-1]));
                        *from_symbol = false;
                        *sym_pos = None;
                        *failed = layers.is_empty();
                    }
                    _ => {}
                }
            }
            removed = true;
        }
        removed
    }

    /// Expand resolved set values only for a consumer after symbol assignment. The written
    /// members used for UNION, member places and earlier creation keep their handles (8.2).
    pub(crate) fn scene_input_members(
        &mut self,
        scene: SceneId,
        object: ObjId,
        site: Option<&Ctx>,
    ) -> Vec<InputMember> {
        self.scene_input_members_at(scene, object, 7, site)
    }

    pub(crate) fn scene_input_members_at(
        &mut self,
        scene: SceneId,
        object: ObjId,
        phase: usize,
        site: Option<&Ctx>,
    ) -> Vec<InputMember> {
        if self.is_empty_node(object) {
            return Vec::new();
        }
        if !self.is_set(object) && !self.is_set(self.scene_value(object)) {
            return vec![InputMember::plain(object)];
        }
        let mut out = Vec::new();
        for member in self.flatten_inputs(scene, object, phase, site) {
            let value = self.scene_value(member.object);
            if value != member.object && self.is_set(value) {
                for mut child in self.scene_input_members_at(scene, value, phase, site) {
                    child.depend_on(&member.dependencies);
                    child.depend_on(&[member.object]);
                    out.push(child);
                }
            } else {
                out.push(member);
            }
        }
        out
    }

    /// Only the symbol-dependent input kinds need this later check. Ordinary terminals keep
    /// the existing missing-symbol and missing-pin diagnostics.
    pub(crate) fn wire_vertex_error(&self, object: ObjId) -> Option<(Code, String)> {
        if self.is_failed(object) {
            return Some((
                codes::WIRE_POINT_FAILED,
                format!("a point of the wire, {}, failed (13.1)", self.label(object)),
            ));
        }
        match self.o(object) {
            ObjData::NamedSubobject { .. } => match self.o(self.scene_value(object)) {
                ObjData::ScenePin { owner, .. } if self.bundle_field(*owner).is_some() => Some((
                    codes::WIRE_IN_BUNDLE,
                    format!("{} is in a bundle; wires go to the pins of the bundle (11.8)", self.label(*owner)),
                )),
                ObjData::ScenePin { .. } | ObjData::Point { .. } | ObjData::Pos { .. } => None,
                _ => Some((
                    codes::WIRE_POINT_WRONG_KIND,
                    format!(
                        "the points of a wire are pins or points, not {} ({}) (13.1)",
                        self.label(object),
                        self.kind_of(object)
                    ),
                )),
            },
            _ => None,
        }
    }

    pub(crate) fn check_scene_pin_wires(&mut self, scene: SceneId) {
        for wire in self.scenes[scene].wires.clone() {
            if self.is_failed(wire) {
                continue;
            }
            self.resolve_scene_wire_nets(scene, wire);
            if self.is_failed(wire) {
                continue;
            }
            let rewrites = self.scenes[scene].rewrites.get(&wire).cloned().unwrap_or_default();
            for (index, rewrite) in rewrites.iter().enumerate() {
                if !self.follows_0_4(self.source_unit(&rewrite.site)) {
                    continue;
                }
                if let Some(fault) = self.wire_rewrite_error(&rewrite.site, &rewrite.points) {
                    self.scenes[scene].rewrites.get_mut(&wire).unwrap()[index].fault = Some(fault);
                }
            }
            let Some(shape) = self.wire_shape(scene, wire) else { continue };
            let Some(site) = &shape.site else { continue };
            if !self.follows_0_4(self.source_unit(site)) {
                continue;
            }
            if let Some((code, message)) = shape.points.iter().find_map(|point| self.wire_vertex_error(*point)) {
                let diagnostic = self.err(site, code, message, shape.off);
                self.late_failures.insert(wire, diagnostic);
                self.set_failed(wire);
            }
        }
    }

    pub(crate) fn wire_rewrite_error(&self, site: &Ctx, points: &[ObjId]) -> Option<(Code, String)> {
        if self.follows_0_4(self.source_unit(site)) && self.failed_body(site) {
            return Some((codes::REF_FAILED, "an object used by this wire rewrite failed (7.3, 13.1)".into()));
        }
        points.iter().find_map(|point| self.wire_vertex_error(*point))
    }

    fn resolve_scene_wire_nets(&mut self, scene: SceneId, wire: ObjId) {
        let ObjData::Wire { nets, bus, site: Some(site), off, .. } = self.o(wire).clone() else { return };
        if !self.follows_0_4(self.source_unit(&site))
            || !nets.iter().any(|net| {
                matches!(self.o(*net), ObjData::NamedSubobject { .. } | ObjData::ScenePin { .. })
                    || self.role_inputs.contains_key(net)
            })
        {
            return;
        }
        let mut resolved = Vec::new();
        let mut is_bus = bus;
        let mut empty_node = false;
        for net in nets {
            empty_node |= self.is_empty_node(net);
            is_bus |= self.is_set(self.scene_value(net));
            for member in self.scene_input_members(scene, net, Some(&site)) {
                let source = self.scene_value(member.object);
                let value = match self.convert_member(source, Some("net"), scene, &site, off) {
                    Ok(Some(value)) => value,
                    Ok(None) => source,
                    Err(_) => {
                        self.set_failed(wire);
                        return;
                    }
                };
                let failure = self.failed_obj(member.object);
                empty_node |= self.is_empty_node(value);
                is_bus |= self.is_set(value);
                let values = self.scene_input_members(scene, value, Some(&site));
                if failure.is_some() || values.iter().any(|v| !matches!(self.o(v.object), ObjData::Net { .. })) {
                    let diagnostic = self.err_cause(
                        &site,
                        if failure.is_some() { codes::WIRE_NET_FAILED } else { codes::WIRE_NET_WRONG_KIND },
                        format!("a net of the wire, {}, failed or is not a net (13.1)", self.label(member.object)),
                        off,
                        failure.flatten(),
                    );
                    self.late_failures.insert(wire, diagnostic);
                    self.set_failed(wire);
                    return;
                }
                resolved.extend(values.into_iter().map(|v| self.scene_value(v.object)));
            }
        }
        if resolved.is_empty() && (!empty_node || is_bus) {
            let diagnostic =
                self.err(&site, codes::WIRE_NET_EMPTY_SET, "the net of a wire is an empty set (13.1)", off);
            self.late_failures.insert(wire, diagnostic);
            self.set_failed(wire);
            return;
        }
        // Index the actual nets for connectivity, labels and routing; creation already happened.
        for wires in self.scenes[scene].wires_by_net.values_mut() {
            wires.retain(|candidate| *candidate != wire);
        }
        for net in &resolved {
            let wires = self.scenes[scene].wires_by_net.entry(*net).or_default();
            if !wires.contains(&wire) {
                wires.push(wire);
            }
        }
        if let ObjData::Wire { nets, bus, .. } = self.om(wire) {
            *nets = resolved;
            *bus = is_bus;
        }
    }

    pub(crate) fn failed_body(&self, ctx: &Ctx) -> bool {
        self.context_failure(ctx).is_some()
    }

    fn context_failure(&self, ctx: &Ctx) -> Option<crate::diag::DiagId> {
        ctx.frame_usage()
            .into_iter()
            .chain(ctx.reference_dependencies.borrow().iter().copied())
            .find_map(|object| self.failed_obj(object).flatten())
    }

    /// Symbol-dependent checks do not rewind creation or the symbol queue (8.2).
    pub(crate) fn check_scene_pin_roles(&mut self, scene: SceneId) {
        self.resolve_role_inputs(scene);
        loop {
            let before = self.late_failures.len();
            self.check_scene_pin_wires(scene);
            self.check_scene_pin_graphics(scene);
            for usage in self.scenes[scene].usages.clone() {
                if self.is_failed(usage) {
                    continue;
                }
                let data = self.usage(usage).clone();
                let Some(site) = &data.site else { continue };
                if data.in_symbol || !self.follows_0_4(self.source_unit(site)) {
                    continue;
                }
                if let Some(Some(cause)) = data.parent.and_then(|parent| self.failed_obj(parent)) {
                    self.late_failures.insert(usage, cause);
                    continue;
                }
                if let Some(cause) = self.context_failure(site) {
                    let diagnostic = self.err_cause(
                        site,
                        codes::REF_FAILED,
                        "an object used by this application failed",
                        data.off,
                        Some(cause),
                    );
                    self.fail_scene_usage(scene, usage, diagnostic);
                    continue;
                }
                let definition = self.def(data.definition).clone();
                'roles: for role in &definition.roles {
                    let Some(object) = data.role(&role.name) else { continue };
                    if !role.is_set && self.is_set(self.scene_value(object)) {
                        let diagnostic = self.err(
                            site,
                            codes::USAGE_ROLE_WRONG_KIND,
                            format!(
                                "the role {} takes a single object, but {} resolves to a set (5.2.4, 9.4)",
                                role.name,
                                self.label(object)
                            ),
                            data.off,
                        );
                        self.fail_scene_usage(scene, usage, diagnostic);
                        break 'roles;
                    }
                    let members = if role.is_set {
                        self.scene_input_members(scene, object, Some(site))
                    } else {
                        vec![InputMember::plain(object)]
                    };
                    for member in members {
                        let failure = self
                            .failed_obj(member.object)
                            .or_else(|| member.dependencies.iter().find_map(|object| self.failed_obj(*object)));
                        let result = if let Some(cause) = failure {
                            let diagnostic = self.err_cause(
                                site,
                                codes::USAGE_ROLE_FAILED_OBJECT,
                                format!("the role {} received a failed object", role.name),
                                data.off,
                                cause,
                            );
                            Err(Stop::Fail(Some(diagnostic)))
                        } else {
                            self.check_kind(data.definition, &role.name, &role.kinds, member.object, site, data.off)
                        };
                        if let Err(Stop::Fail(Some(diagnostic))) = result {
                            self.fail_scene_usage(scene, usage, diagnostic);
                            break 'roles;
                        }
                    }
                }
            }
            // Only objects made by a failed body fail with it. A role may name a perfectly
            // valid host object shared with other usages, and must never be failed here.
            let mut children: Vec<_> =
                self.scenes[scene].creation_sites.iter().map(|(&o, site)| (o, site.clone())).collect();
            children.sort_by_key(|(object, _)| *object);
            for (object, site) in children {
                // A keyed text combines independent TEXT layers; failure removes a layer,
                // not the shared handle or another statement's valid text.
                if matches!(self.o(object), ObjData::KeyText { .. }) {
                    continue;
                }
                if let Some(cause) = self.context_failure(&site) {
                    self.late_failures.entry(object).or_insert(cause);
                    self.set_failed(object);
                }
            }
            self.remove_failed_body_texts(scene);
            if self.late_failures.len() == before {
                break;
            }
        }
    }

    fn check_scene_pin_graphics(&mut self, scene: SceneId) {
        let objects: Vec<_> = self.scenes[scene].prims.iter().chain(&self.scenes[scene].texts).copied().collect();
        for &object in &objects {
            if self.is_obj_failed(object) {
                continue;
            }
            let (inputs, site, off, code, description) = match self.o(object).clone() {
                ObjData::Prim { points, site, off, .. } => (
                    points,
                    site,
                    off,
                    codes::KEYWORD_GRAPHIC_NOT_POINTS,
                    "the points of a graphic must be points or pins",
                ),
                ObjData::Text { attach, site, off, .. } | ObjData::KeyText { attach, site, off, .. } => {
                    (vec![attach], site, off, codes::TEXT_ATTACH_NOT_ALLOWED, "TEXT cannot be attached to this object")
                }
                _ => continue,
            };
            let Some(site) = site else { continue };
            for input in inputs {
                let value = self.scene_value(input);
                let terminal_point = self.follows_0_4(self.source_unit(&site))
                    && matches!(self.o(object), ObjData::Prim { .. })
                    && matches!(self.o(value), ObjData::Term { .. });
                if !terminal_point
                    && !matches!(self.o(input), ObjData::NamedSubobject { .. })
                    && !self.role_inputs.contains_key(&input)
                {
                    continue;
                }
                let valid = match self.o(object) {
                    ObjData::Text { .. } | ObjData::KeyText { .. } => {
                        matches!(
                            self.o(value),
                            ObjData::ScenePin { .. }
                                | ObjData::Net { .. }
                                | ObjData::Term { .. }
                                | ObjData::Point { .. }
                                | ObjData::Pos { .. }
                        )
                    }
                    _ if terminal_point => {
                        let ObjData::Term { owner, .. } = *self.o(value) else { unreachable!() };
                        // a terminal converts to its pin, of whatever symbol (5.2.4)
                        self.pin_for(scene, owner, value).is_some()
                    }
                    _ => {
                        matches!(self.o(value), ObjData::ScenePin { .. } | ObjData::Point { .. } | ObjData::Pos { .. })
                    }
                };
                if self.is_failed(input) || !valid {
                    let diagnostic = self.err_cause(
                        &site,
                        code,
                        format!("{description}: {} ({}) (5.2.4)", self.label(input), self.kind_of(input)),
                        off,
                        self.failed_obj(input).flatten(),
                    );
                    self.late_failures.insert(object, diagnostic);
                    self.set_failed(object);
                    break;
                }
                if matches!(self.o(value), ObjData::Net { .. }) {
                    let items = self.text_items(object);
                    if items.dx.is_some() || items.dy.is_some() {
                        let diagnostic = self.err(
                            &site,
                            codes::TEXT_WIRE_NET_NO_OFFSET,
                            "a text attached to a wire or a net takes no dx, dy (13.3)",
                            off,
                        );
                        self.late_failures.insert(object, diagnostic);
                        self.set_failed(object);
                        break;
                    }
                }
            }
        }
    }

    fn fail_scene_usage(&mut self, scene: SceneId, usage: ObjId, diagnostic: crate::diag::DiagId) {
        self.late_failures.insert(usage, diagnostic);
        let data = self.usage(usage).clone();
        let Some(site) = data.site else { return };
        if site.frame.is_some() || site.selected {
            return;
        }
        let Some(span) = site.stmt else { return };
        let stmt = (span.rec, span.start, span.end);
        if self.scenes[scene].failed_usages.iter().any(|failed| failed.stmt == stmt) {
            return;
        }
        let name = self.objs[usage.0 as usize].name.clone();
        self.scenes[scene].failed_usages.push(super::scene::FailedApplication {
            name: (!name.is_empty()).then_some(name),
            definition: Some(data.definition),
            roles: data.roles.into_iter().map(|(name, object)| (Some(name), object)).collect(),
            stmt,
            diag: Some(diagnostic),
        });
    }

    fn remove_failed_body_texts(&mut self, scene: SceneId) {
        let sources = self.scenes[scene].pin_text_sources.clone();
        for source in &sources {
            let Some(cause) = self.context_failure(&source.ctx) else { continue };
            let valid: Vec<_> =
                sources.iter().filter(|other| other.text == source.text && !self.failed_body(&other.ctx)).collect();
            let last = valid.last();
            let direct_source = valid.iter().rev().find(|other| !other.generated);
            if let ObjData::KeyText { layers, site, off, direct, generated, failed, .. } = self.om(source.text) {
                layers.retain(|(order, _, _)| *order != source.order);
                *site = last.map(|source| source.ctx.clone());
                *off = last.map_or(0, |source| source.off);
                *generated = last.is_some_and(|source| source.generated);
                *direct = direct_source.map(|source| (source.ctx.clone(), source.off));
                *failed = layers.is_empty();
            }
            if valid.is_empty() {
                self.late_failures.entry(source.text).or_insert(cause);
            }
        }
    }

    /// The value used by a later operation, without changing a set's written handle.
    pub(crate) fn scene_value(&self, object: ObjId) -> ObjId {
        let object = self.input_value(object);
        match self.o(object) {
            ObjData::NamedSubobject { resolved: Some(value), .. } => *value,
            _ => object,
        }
    }

    pub(crate) fn scene_named_subobject(
        &mut self,
        scene: SceneId,
        parent: ObjId,
        name: &str,
        ctx: &Ctx,
        off: usize,
    ) -> ObjId {
        let key = (parent, name.to_owned());
        if let Some(&object) = self.scenes[scene].named_subobjects.get(&key) {
            return object;
        }
        let object = self.new_obj(ObjData::NamedSubobject {
            parent,
            name: name.to_owned(),
            resolved: None,
            site: Rc::new(ctx.clone()),
            off,
        });
        self.scenes[scene].named_subobjects.insert(key, object);
        if self.scenes[scene].phase >= 7 {
            self.finish_scene_subobjects(scene);
        } else if self.scenes[scene].phase >= 5 {
            self.resolve_scene_subobjects(scene);
        }
        object
    }

    /// Never replace or merge the written handle: sets still compare that handle.
    pub(crate) fn resolve_scene_subobjects(&mut self, scene: SceneId) {
        let mut objects: Vec<_> = self.scenes[scene].named_subobjects.values().copied().collect();
        objects.sort();
        for &object in &objects {
            if self.is_failed(object) {
                continue;
            }
            let ObjData::NamedSubobject { parent, name, .. } = self.o(object).clone() else { continue };
            if name == "net" {
                continue;
            }
            if name == "position" {
                self.resolve_position_reference(scene, object);
                continue;
            }
            let pin = self.scenes[scene]
                .symbol
                .get(&parent)
                .and_then(|symbol| self.symbol(*symbol).pins.iter().position(|pin| pin.names.contains(&name)));
            let value = if let Some(index) = pin {
                Some(self.scene_pin_object(scene, parent, index))
            } else {
                self.scenes[scene].keytexts.get(&(parent, name)).copied()
            };
            if let ObjData::NamedSubobject { resolved, .. } = self.om(object) {
                *resolved = value;
            }
        }
        // Resolve named pins first, including pins of later-created usages. Net references
        // can then follow role bindings independently of the order of the references.
        for object in objects {
            let ObjData::NamedSubobject { parent, name, .. } = self.o(object).clone() else { continue };
            if name != "net" || self.is_failed(object) {
                continue;
            }
            let ObjData::ScenePin { owner, index } = *self.o(self.scene_value(parent)) else { continue };
            let nets = self.scene_pin_nets(scene, owner, index);
            let value = if nets.is_empty() {
                self.scene_pin_has_empty_node(scene, owner, index)
                    .then(|| self.new_obj(ObjData::EmptyNode { direct: false }))
            } else if nets.iter().all(|net| *net == nets[0]) && !self.scene_pin_is_bus(scene, owner, index) {
                Some(nets[0])
            } else {
                Some(self.new_obj(ObjData::Set(Box::new(SetData::plain(
                    nets.into_iter().map(|net| (net, true)).collect(),
                    false,
                )))))
            };
            if let ObjData::NamedSubobject { resolved, .. } = self.om(object) {
                *resolved = value;
            }
        }
    }

    pub(crate) fn finish_scene_subobjects(&mut self, scene: SceneId) {
        self.resolve_scene_subobjects(scene);
        let mut objects: Vec<_> = self.scenes[scene].named_subobjects.values().copied().collect();
        objects.sort();
        for object in objects {
            let ObjData::NamedSubobject { parent, name, resolved, site, off } = self.o(object).clone() else {
                continue;
            };
            if resolved.is_some() {
                continue;
            }
            if name == "position" && self.position_waits_for_role(parent) {
                continue;
            }
            let (code, message) = if name == "net"
                && matches!(self.o(self.scene_value(parent)), ObjData::ScenePin { .. })
            {
                (
                    codes::PIN_ON_NO_NET,
                    format!("{} is on no net: it binds no connected terminal or role (5.2.4, 6.5)", self.label(parent)),
                )
            } else {
                (codes::REF_NO_SUBOBJECT, format!("{} has no sub-object {name} (6.5)", self.label(parent)))
            };
            let diagnostic = self.err(&site, code, message, off);
            let failed = self.new_obj(ObjData::Failed(Some(diagnostic)));
            if let ObjData::NamedSubobject { resolved, .. } = self.om(object) {
                *resolved = Some(failed);
            }
        }
        // These queues run after symbol assignment. Normalize their operation targets,
        // never the handles stored by sets or used for earlier object creation.
        let replacements: std::collections::HashMap<_, _> = self.scenes[scene]
            .named_subobjects
            .values()
            .filter_map(|object| match self.o(*object) {
                ObjData::NamedSubobject { resolved: Some(value), .. } => Some((*object, *value)),
                _ => None,
            })
            .collect();
        let sc = &mut self.scenes[scene];
        for facts in [&mut sc.at, &mut sc.orient, &mut sc.styles, &mut sc.layers, &mut sc.prioritys] {
            for fact in facts {
                if let Some(value) = replacements.get(&fact.target) {
                    fact.target = *value;
                }
            }
        }
        sc.at_by_target.clear();
        for (index, fact) in sc.at.iter().enumerate() {
            sc.at_by_target.entry(fact.target).or_default().push(index);
        }
    }
}
