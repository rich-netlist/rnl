//! Written items and independent constraints (5.2, 14.1).

use std::collections::HashSet;
use std::rc::Rc;

use crate::diag::codes;
use crate::syntax::Segment;

use super::ctx::Ctx;
use super::eval::{ArgV, Stop, R};
use super::obj::*;
use super::places::Query;
use super::Interp;

const MAX_CONSTRAINTS: usize = 1024;

pub(super) struct Pending {
    template: ConstraintData,
    args: Vec<ArgV>,
    segment: Segment,
    scene: SceneId,
    key: Query,
}

struct Expansion {
    atoms: Vec<ConstraintData>,
    collection: bool,
}

impl Interp<'_> {
    pub(super) fn constraint_members(&mut self, s: SceneId, obj: ObjId, phase: usize, ctx: &Ctx) -> Vec<ObjId> {
        self.constraint_inputs(s, obj, phase, ctx).into_iter().map(|member| member.object).collect()
    }

    fn constraint_inputs(&mut self, s: SceneId, obj: ObjId, phase: usize, ctx: &Ctx) -> Vec<InputMember> {
        if phase < 7 {
            self.written_inputs(obj, Some(s), &mut HashSet::new())
        } else {
            self.input_items(s, obj, phase, Some(ctx))
        }
    }

    fn constraint_leaves(&mut self, s: SceneId, obj: ObjId, phase: usize, ctx: &Ctx) -> Vec<InputMember> {
        let obj = self.positions_now(s, obj, phase, ctx);
        if !self.is_set(obj) {
            return vec![InputMember::plain(obj)];
        }
        let mut out = Vec::new();
        for item in self.constraint_inputs(s, obj, phase, ctx) {
            for mut leaf in self.constraint_leaves(s, item.object, phase, ctx) {
                leaf.depend_on(&item.dependencies);
                out.push(leaf);
            }
        }
        out
    }

    /// Unlike the ordinary members view, a reference to a set stays one written item.
    /// Edits preserve the structure inside each item, including a single nested item that
    /// a keyword will split one level (5.2.1).
    pub(super) fn written_items(&mut self, s: SceneId, obj: ObjId, phase: usize, ctx: &Ctx) -> Vec<ObjId> {
        self.written_input_items(s, obj, phase, ctx).into_iter().map(|member| member.object).collect()
    }

    pub(super) fn written_input_items(&mut self, s: SceneId, obj: ObjId, phase: usize, ctx: &Ctx) -> Vec<InputMember> {
        self.written_items_with_failures(s, obj, phase, ctx, false)
    }

    pub(super) fn written_items_with_failures(
        &mut self,
        s: SceneId,
        obj: ObjId,
        phase: usize,
        ctx: &Ctx,
        keep_failed: bool,
    ) -> Vec<InputMember> {
        let obj = self.positions_now(s, obj, phase, ctx);
        self.materialize_constraint(obj);
        if !self.is_set(obj) {
            return vec![InputMember::plain(obj)];
        }
        let sd = self.set(obj).clone();
        let mut items: Vec<InputMember> = match sd.kind {
            SetKind::Plain => sd
                .items
                .iter()
                .enumerate()
                .filter(|(_, (o, _))| keep_failed || !self.is_failed(*o))
                .map(|(index, &(object, _))| InputMember {
                    object,
                    dependencies: sd.item_dependencies.get(index).cloned().unwrap_or_default(),
                })
                .collect(),
            SetKind::Select { .. } => {
                self.select(s, obj, phase, Some(ctx)).into_iter().map(InputMember::plain).collect()
            }
            SetKind::Gen { ref by_scene } => {
                by_scene.get(&s).into_iter().flatten().copied().map(InputMember::plain).collect()
            }
            SetKind::Constraints { ref members, .. } => members.iter().copied().map(InputMember::plain).collect(),
            SetKind::Union { source } => {
                let mut unique = HashSet::new();
                self.constraint_inputs(s, source, phase, ctx)
                    .into_iter()
                    .filter(|m| unique.insert(self.written_input(m.object)))
                    .collect()
            }
        };
        let mut mods = sd.mods_by_scene.get(&Some(s)).cloned().unwrap_or_default();
        mods.sort_by_cached_key(|&k| {
            let m = &sd.mods[k];
            (order_pos(&m.pos, &m.ctx.segments), m.ctx.member_key.clone(), m.off)
        });
        for k in mods {
            let edit = &sd.mods[k];
            if edit.add {
                items.push(InputMember {
                    object: edit.source,
                    dependencies: edit.ctx.reference_dependencies.borrow().clone(),
                });
            } else {
                let removed: HashSet<_> = self
                    .constraint_members(s, edit.source, phase, &edit.ctx)
                    .into_iter()
                    .map(|object| self.written_input(object))
                    .collect();
                items = items
                    .into_iter()
                    .filter_map(|item| self.constraint_item_without(s, item, phase, ctx, &removed, keep_failed))
                    .collect();
            }
        }
        items
    }

    fn constraint_item_without(
        &mut self,
        s: SceneId,
        item: InputMember,
        phase: usize,
        ctx: &Ctx,
        removed: &HashSet<ObjId>,
        keep_failed: bool,
    ) -> Option<InputMember> {
        if removed.contains(&self.written_input(item.object)) {
            return None;
        }
        if !self.is_set(item.object) {
            return Some(item);
        }
        let before = self.written_items_with_failures(s, item.object, phase, ctx, keep_failed);
        let after: Vec<_> = before
            .iter()
            .filter_map(|o| self.constraint_item_without(s, o.clone(), phase, ctx, removed, keep_failed))
            .collect();
        if before == after {
            Some(item)
        } else {
            let mut data = SetData::plain(after.iter().map(|m| (m.object, false)).collect(), true);
            data.item_dependencies = after.into_iter().map(|m| m.dependencies).collect();
            Some(InputMember { object: self.new_obj(ObjData::Set(Box::new(data))), dependencies: item.dependencies })
        }
    }

    pub(super) fn constraint_call(
        &mut self,
        template: ConstraintData,
        args: &[ArgV],
        segment: &Segment,
        ctx: &Ctx,
        selected: bool,
    ) -> R<(ObjId, bool)> {
        let s = ctx.scene.unwrap();
        if selected {
            let key = Self::constraint_result_key(ctx, segment.start);
            let data = if self.planned_constraint_set(&key) {
                ObjData::Set(Box::new(SetData {
                    kind: SetKind::Constraints { members: Vec::new(), selected: true },
                    ..SetData::plain(args.iter().map(|a| (a.obj, false)).collect(), true)
                }))
            } else {
                ObjData::Constraint(Box::new(template.clone()))
            };
            let result = self.new_obj(data);
            self.scenes[s].note_created(result, ctx.pos.clone(), ctx);
            if self.is_set(result) {
                self.note_made(result, ctx, segment.start);
            }
            self.pending_constraints
                .insert(result, Pending { template, args: args.to_vec(), segment: segment.clone(), scene: s, key });
            self.note_rule(ctx);
            return Ok((result, false));
        }
        let expansion = self.expand_constraint_call(&template, args, segment, ctx, 3)?;
        Ok((self.install_constraints(s, None, expansion, ctx, segment.start, args), false))
    }

    /// Whether a constraint of scene `s` waits to be made, when a set that holds it is first read
    /// at the constraint phase or later.
    pub(super) fn constraint_pending_in(&self, s: SceneId) -> bool {
        self.pending_constraints.values().any(|p| p.scene == s)
    }

    pub(super) fn materialize_constraints(&mut self, s: SceneId) {
        let mut pending: Vec<_> =
            self.pending_constraints.iter().filter_map(|(&o, p)| (p.scene == s).then_some(o)).collect();
        pending.sort();
        for result in pending {
            self.materialize_constraint(result);
        }
    }

    pub(super) fn materialize_constraint(&mut self, result: ObjId) {
        let Some(p) = self.pending_constraints.get(&result) else { return };
        if self.scenes[p.scene].phase < 7 {
            return;
        }
        let p = self.pending_constraints.remove(&result).unwrap();
        let ctx = p.template.site.as_ref().unwrap().clone();
        match self.expand_constraint_call(&p.template, &p.args, &p.segment, &ctx, 7) {
            Ok(expansion) => {
                // A selection can decide whether there is an item to expand. Re-read if it
                // changes the result's kind, so earlier type checks and set edits see it too.
                self.remember_constraint_set(p.key, expansion.collection);
                self.install_constraints(p.scene, Some(result), expansion, &ctx, p.segment.start, &p.args);
            }
            Err(Stop::Language) => {
                self.inactive_objects.insert((p.scene, result));
                *self.om(result) = ObjData::Set(Box::new(SetData::plain(Vec::new(), false)));
            }
            Err(stop) => {
                self.note_failed_at(ctx.rec, p.segment.start, false);
                let cause = if let Stop::Fail(d) = stop { d } else { None };
                *self.om(result) = ObjData::Failed(cause);
            }
        }
    }

    fn install_constraints(
        &mut self,
        s: SceneId,
        result: Option<ObjId>,
        mut expansion: Expansion,
        ctx: &Ctx,
        off: usize,
        args: &[ArgV],
    ) -> ObjId {
        if !expansion.collection {
            let atom = expansion.atoms.pop().unwrap();
            let site = atom.site.clone();
            let root = if let Some(root) = result {
                *self.om(root) = ObjData::Constraint(Box::new(atom));
                root
            } else {
                self.new_obj(ObjData::Constraint(Box::new(atom)))
            };
            self.scene_add_constraint(s, root, ctx.pos.clone(), site.as_deref().unwrap_or(ctx));
            return root;
        }
        let root = result.unwrap_or_else(|| self.new_obj(ObjData::Set(Box::new(SetData::plain(Vec::new(), true)))));
        let mut members = Vec::new();
        for mut atom in expansion.atoms {
            atom.name_owner = Some(root);
            let site = atom.site.clone();
            let c = self.new_obj(ObjData::Constraint(Box::new(atom)));
            self.scene_add_constraint(s, c, ctx.pos.clone(), site.as_deref().unwrap_or(ctx));
            members.push(c);
        }
        let mut data = SetData {
            kind: SetKind::Constraints { members, selected: result.is_some() },
            ..SetData::plain(args.iter().map(|a| (a.obj, false)).collect(), true)
        };
        if let ObjData::Set(previous) = self.o(root) {
            data.mods = previous.mods.clone();
            data.mod_sources = previous.mod_sources.clone();
            data.mod_source_set = previous.mod_source_set.clone();
            data.mods_by_scene = previous.mods_by_scene.clone();
            data.lazy_sources = previous.lazy_sources.clone();
        }
        *self.om(root) = ObjData::Set(Box::new(data));
        self.note_made(root, ctx, off);
        root
    }

    fn expand_constraint_call(
        &mut self,
        template: &ConstraintData,
        args: &[ArgV],
        seg: &Segment,
        ctx: &Ctx,
        phase: usize,
    ) -> R<Expansion> {
        if !self.language_inputs_reach(ctx, phase) {
            return Err(Stop::Language);
        }
        let s = ctx.scene.unwrap();
        let kw = template.kind.as_str();
        let positions = super::kw::sig_positions(kw);
        let mapped = self.map_args(kw, args, seg, ctx, positions)?;
        let args: Vec<_> = positions.iter().map(|p| mapped[&p.name].obj).collect();
        for &a in &args {
            if let Some(cause) = self.failed_input(a) {
                let d = self.err_cause(
                    ctx,
                    codes::CONSTRAINT_MEMBER_FAILED,
                    format!("a member of {kw} failed, so the whole constraint fails (7.3)"),
                    seg.start,
                    cause,
                );
                return Err(Stop::Fail(Some(d)));
            }
        }
        let pair = matches!(template.kind, ConstraintKind::Distance | ConstraintKind::OrientRelation);
        let (columns, collection) = if pair {
            // the `::position` of a set known only now is a set like the others (6.5)
            let args: Vec<ObjId> = args.iter().map(|&a| self.positions_now(s, a, phase, ctx)).collect();
            let columns: Vec<_> = args.iter().map(|a| self.constraint_leaves(s, *a, phase, ctx)).collect();
            if template.kind == ConstraintKind::OrientRelation && columns.iter().all(|c| c.len() > 1) {
                let shown: Vec<_> = columns
                    .iter()
                    .map(|c| format!("{{{}}}", c.iter().map(|o| self.label(o.object)).collect::<Vec<_>>().join(" ")))
                    .collect();
                return self.fail(ctx, codes::CONSTRAINT_ORIENT_RELATION_SETS_BOTH_SIDES, format!("{kw} relates one object to one object, and both its slots have more than one member, {} and {}: pairing them in order and relating every pair would both make sense; this constraint fails (5.2.1, 14.9)", shown[0], shown[1]), seg.start);
            }
            (columns, args.iter().any(|a| self.is_set(*a)))
        } else {
            let mut items = self.written_input_items(s, args[0], phase, ctx);
            if items.len() == 1 && self.is_set(items[0].object) {
                let parent = items.pop().unwrap();
                items = self.written_input_items(s, parent.object, phase, ctx);
                for member in &mut items {
                    member.depend_on(&parent.dependencies);
                }
            }
            for item in &mut items {
                item.object = self.positions_now(s, item.object, phase, ctx);
            }
            let collection = items.iter().any(|o| self.is_set(o.object));
            let columns: Vec<_> = items
                .into_iter()
                .map(|item| {
                    let mut leaves = self.constraint_leaves(s, item.object, phase, ctx);
                    for leaf in &mut leaves {
                        leaf.depend_on(&item.dependencies);
                    }
                    leaves
                })
                .filter(|c| !c.is_empty())
                .collect();
            if columns.len() < 2 {
                self.hint_at(ctx, codes::CONSTRAINT_EXPANSION_TOO_FEW_PLACES, format!("{kw} has fewer than two items after empty sets are omitted, so its constraints hold trivially (14.1)"), seg.start);
            }
            (columns, collection)
        };
        let count = columns.iter().fold(1usize, |n, c| n.saturating_mul(c.len()));
        if count == 0 {
            return Ok(Expansion { atoms: Vec::new(), collection: true });
        }
        if count > MAX_CONSTRAINTS {
            return self.fail(ctx, codes::CONSTRAINT_EXPANSION_TOO_MANY, format!("{kw} exceeds this implementation's limit of {MAX_CONSTRAINTS} expanded constraints in one statement (3.7, 14.1)"), seg.start);
        }
        let mut combinations = vec![Vec::new()];
        for column in columns {
            combinations = combinations
                .into_iter()
                .flat_map(|row| {
                    column.iter().map(move |o| {
                        let mut next = row.clone();
                        next.push(o.clone());
                        next
                    })
                })
                .collect();
        }
        // a combination that takes one object twice is left out (14.1, 14.5); a usage that may
        // stand for its members (14.1) is not yet one object: it is kept until the symbols are
        // known and each side has stood for its members (`split_usage_constraints`)
        let deferred = matches!(
            template.kind,
            ConstraintKind::Align | ConstraintKind::Order | ConstraintKind::Distribute | ConstraintKind::Distance
        );
        if template.kind != ConstraintKind::OrientRelation && !combinations.is_empty() {
            combinations.retain(|row| {
                let mut seen = HashSet::new();
                row.iter().all(|m| {
                    let object = self.one_object(m.object);
                    seen.insert(object) || (deferred && matches!(self.o(object), ObjData::Usage(_)))
                })
            });
            if combinations.is_empty() {
                self.hint_at(ctx, codes::CONSTRAINT_EXPANSION_TOO_FEW_PLACES, format!("{kw} relates no two different objects once a combination that takes one object twice is left out, so its constraints hold trivially (14.1)"), seg.start);
                return Ok(Expansion { atoms: Vec::new(), collection: true });
            }
        }
        let mut atoms = Vec::new();
        for row in combinations {
            let atom_ctx = ctx.with_dependencies(&[]);
            let row: Vec<_> = row
                .into_iter()
                .map(|member| {
                    atom_ctx.depend_on(&member.dependencies);
                    let point = if template.kind != ConstraintKind::OrientRelation {
                        self.constraint_point(member.object)
                    } else {
                        member.object
                    };
                    if point != member.object {
                        atom_ctx.depend_on(&[member.object]);
                    }
                    point
                })
                .collect();
            let mut atom = template.clone();
            atom.site = Some(Rc::new(atom_ctx));
            atom.args = if pair {
                row
            } else {
                vec![self.new_obj(ObjData::Set(Box::new(SetData::plain(
                    row.into_iter().map(|o| (o, false)).collect(),
                    true,
                ))))]
            };
            atoms.push(atom);
        }
        Ok(Expansion { atoms, collection })
    }

    /// What an item of a constraint stands for (14.1): a usage without a symbol,
    /// its members, the objects that fill its roles, a usage among them in turn for its own members,
    /// down to the objects that are not such usages, nets left out, as a GROUP takes them (11.5); any
    /// other object, itself, and so is a usage whose body the statement is of, which `?` names
    /// there: that is its own point (6.5). The usages it went through are added to `through`.
    fn standing_for(
        &mut self,
        s: SceneId,
        item: ObjId,
        ctx: &Ctx,
        chain: &[ObjId],
        through: &mut Vec<ObjId>,
    ) -> Vec<ObjId> {
        let start = self.scene_value(item);
        if !matches!(self.o(start), ObjData::Usage(_))
            || self.scenes[s].symbol.contains_key(&start)
            || chain.contains(&start)
        {
            return vec![item];
        }
        let mut out = Vec::new();
        let mut active = HashSet::new();
        let mut stack = vec![(start, false)];
        while let Some((written, leaving)) = stack.pop() {
            let object = self.scene_value(written);
            if leaving {
                active.remove(&object);
                continue;
            }
            if self.is_failed(written) || self.is_failed(object) {
                continue;
            }
            let children = if self.is_set(object) {
                vec![object]
            } else if let ObjData::Usage(usage) = self.o(object) {
                if self.scenes[s].symbol.contains_key(&object) {
                    out.push(object);
                    continue;
                }
                if !through.contains(&object) {
                    through.push(object);
                }
                usage.roles.iter().map(|(_, filler)| *filler).collect()
            } else {
                if !matches!(self.o(object), ObjData::Net { .. }) {
                    out.push(object);
                }
                continue;
            };
            if !active.insert(object) {
                continue;
            }
            stack.push((object, true));
            let mut members = Vec::new();
            for child in children {
                for member in self.scene_input_members(s, child, Some(ctx)) {
                    if !member.dependencies.iter().any(|dependency| self.is_failed(*dependency)) {
                        members.push((member.object, false));
                    }
                }
            }
            stack.extend(members.into_iter().rev());
        }
        out
    }

    /// Once the symbols are known, a usage without a symbol among the items of
    /// an ALIGN, ORDER, DISTRIBUTE or DISTANCE stands for its members (14.1): each expanded
    /// constraint that has one becomes one for each combination of what its items stand for, each
    /// a unit of its own as the combinations of a set are and in their order, the first in its
    /// place and the others split off it. A combination that has an object more than once is left
    /// out; a constraint with none left takes no part. The limit of expanded constraints holds for
    /// each constraint so split.
    pub(super) fn split_usage_constraints(&mut self, s: SceneId) {
        for c in self.scenes[s].constraints.clone() {
            if self.is_failed(c) {
                continue;
            }
            let ObjData::Constraint(cd) = self.o(c).clone() else { continue };
            let pair = cd.kind == ConstraintKind::Distance;
            if cd.failed
                || cd.table.is_some()
                || !(pair
                    || matches!(cd.kind, ConstraintKind::Align | ConstraintKind::Order | ConstraintKind::Distribute))
            {
                continue;
            }
            let Some(ctx) = cd.site.clone() else { continue };
            let row: Vec<ObjId> = if pair {
                cd.args.clone()
            } else {
                match self.o(cd.args[0]) {
                    ObjData::Set(set) => set.items.iter().map(|(o, _)| *o).collect(),
                    _ => vec![cd.args[0]],
                }
            };
            let mut through = Vec::new();
            let columns: Vec<Vec<ObjId>> =
                row.iter().map(|&o| self.standing_for(s, o, &ctx, &cd.chain, &mut through)).collect();
            if through.is_empty() {
                // a usage with a symbol is one object: written twice, the combination is left out
                let mut seen = HashSet::new();
                if !row.iter().all(|&o| seen.insert(self.one_object(o))) {
                    self.diag_site(crate::diag::Severity::Hint, codes::CONSTRAINT_EXPANSION_TOO_FEW_PLACES, &cd.site, cd.off,
                        format!("{kw} relates no two different objects once a combination that takes one object twice is left out, so its constraints hold trivially (14.1)", kw = cd.kind.as_str()));
                    self.scenes[s].constraints.retain(|&x| x != c);
                    self.remove_from_collection(cd.name_owner, c);
                }
                continue;
            }
            self.hint_members_placed_apart(s, &cd, &row, &columns);
            let columns: Vec<Vec<ObjId>> = columns.into_iter().filter(|c| !c.is_empty()).collect();
            let kw = cd.kind.as_str();
            let count = columns.iter().fold(1usize, |n, c| n.saturating_mul(c.len()));
            if count > MAX_CONSTRAINTS {
                self.diag_site(crate::diag::Severity::Error, codes::CONSTRAINT_EXPANSION_TOO_MANY, &cd.site, cd.off,
                    format!("{kw} exceeds this implementation's limit of {MAX_CONSTRAINTS} expanded constraints in one statement once its usages stand for their members (3.7, 14.1)"));
                self.constraint_mut(c).failed = true;
                continue;
            }
            if columns.len() < 2 {
                self.diag_site(crate::diag::Severity::Hint, codes::CONSTRAINT_EXPANSION_TOO_FEW_PLACES, &cd.site, cd.off,
                    format!("{kw} has fewer than two items once its usages stand for their members, so its constraints hold trivially (14.1)"));
            }
            let mut rows = vec![Vec::new()];
            if columns.len() >= 2 {
                for column in &columns {
                    rows = rows
                        .into_iter()
                        .flat_map(|row: Vec<ObjId>| {
                            column.iter().map(move |o| {
                                let mut next = row.clone();
                                next.push(*o);
                                next
                            })
                        })
                        .collect();
                }
            } else {
                rows.clear();
            }
            // a combination that takes one object twice is left out
            rows.retain(|row| {
                let mut seen = HashSet::new();
                row.iter().all(|&o| seen.insert(self.one_object(o)))
            });
            if rows.is_empty() {
                if columns.len() >= 2 {
                    self.diag_site(crate::diag::Severity::Hint, codes::CONSTRAINT_EXPANSION_TOO_FEW_PLACES, &cd.site, cd.off,
                        format!("{kw} relates no two different objects once its usages stand for their members, so its constraints hold trivially (14.1)"));
                }
                self.scenes[s].constraints.retain(|&x| x != c);
                self.remove_from_collection(cd.name_owner, c);
                continue;
            }
            let make_args = |this: &mut Self, row: Vec<ObjId>| -> Vec<ObjId> {
                if pair {
                    row
                } else {
                    vec![this.new_obj(ObjData::Set(Box::new(SetData::plain(
                        row.into_iter().map(|o| (o, false)).collect(),
                        true,
                    ))))]
                }
            };
            let mut rows = rows.into_iter();
            let first = make_args(self, rows.next().unwrap());
            self.constraint_mut(c).args = first;
            let mut after = c;
            for row in rows {
                let mut data = cd.clone();
                data.args = make_args(self, row);
                data.name_owner = Some(cd.name_owner.unwrap_or(c));
                let clone = self.new_obj(ObjData::Constraint(data));
                self.split_from.insert(clone, self.split_from.get(&c).copied().unwrap_or(c));
                let scene = &mut self.scenes[s];
                // the later in the list is the earlier in the order of 8.5, as for the combinations of
                // a set: each one split off goes before the one before it
                let at = scene.constraints.iter().position(|&x| x == after).unwrap_or(scene.constraints.len());
                scene.constraints.insert(at, clone);
                if let Some(v) = scene.created.get(&c).cloned() {
                    scene.created.insert(clone, v);
                }
                if let Some(v) = scene.created_order.get(&c).cloned() {
                    scene.created_order.insert(clone, v);
                }
                if let Some(v) = scene.created_phase.get(&c).copied() {
                    scene.created_phase.insert(clone, v);
                }
                if let Some(v) = scene.creation_sites.get(&c).cloned() {
                    scene.creation_sites.insert(clone, v);
                }
                self.add_to_collection(cd.name_owner, after, clone);
                after = clone;
            }
        }
    }

    /// An ALIGN, or a DISTRIBUTE of three items or more, whose item is a usage without a symbol
    /// relates each member of the usage to the same other objects, so it holds only if they share
    /// the coordinate of its axis. When the usage's own body places two of them apart on that
    /// axis, the statement has the hint, which says that `::position` names the usage's own point
    /// (6.5, 14.1). A statement of a package, which the document cannot rewrite, has none.
    fn hint_members_placed_apart(&mut self, s: SceneId, cd: &ConstraintData, row: &[ObjId], columns: &[Vec<ObjId>]) {
        let items = columns.iter().filter(|c| !c.is_empty()).count();
        let firm = match cd.kind {
            ConstraintKind::Align => items >= 2,
            ConstraintKind::Distribute => items >= 3,
            _ => false,
        };
        let Some(axis) = cd.params.axis else { return };
        if !firm || cd.site.as_ref().is_some_and(|c| !self.units[c.unit].pkg().is_empty()) {
            return;
        }
        for (item, column) in row.iter().zip(columns) {
            let usage = self.scene_value(*item);
            if column.len() < 2
                || !matches!(self.o(usage), ObjData::Usage(_))
                || self.scenes[s].symbol.contains_key(&usage)
            {
                continue;
            }
            let offsets: Vec<f64> = column.iter().filter_map(|&o| self.offset_in_body(s, o, usage, axis)).collect();
            let apart = offsets.iter().any(|a| offsets.iter().any(|b| (a - b).abs() > self.tol));
            if !apart {
                continue;
            }
            let kw = cd.kind.as_str();
            let name = self.label(usage);
            let verb = if cd.kind == ConstraintKind::Align { "align" } else { "distribute" };
            let advice = if name.starts_with('%') {
                format!("to {verb} its own point write {name}::position")
            } else {
                format!("to {verb} its own point name it and write its ::position")
            };
            self.diag_site(crate::diag::Severity::Hint, codes::USAGE_MEMBERS_PLACED_APART, &cd.site, cd.off,
                format!("{kw} stands for each member of {name}, and the definition of {name} places them apart in {axis}, so its constraints cannot all hold; {advice} (6.5, 14.1)"));
        }
    }

    /// The coordinate of `object` in `axis` relative to the point of `usage`, as the facts of the
    /// usage's body give it, through the points of the usages nested in it; none if they do not.
    fn offset_in_body(&self, s: SceneId, object: ObjId, usage: ObjId, axis: char) -> Option<f64> {
        let mut at = self.coordinate_object(object);
        if let ObjData::Term { owner, .. } = self.o(at) {
            at = self.bundle_field(*owner).unwrap_or(*owner);
        }
        let mut sum = 0.0;
        for _ in 0..32 {
            if at == usage {
                return Some(sum);
            }
            let step = self.scenes[s].at_by_target.get(&at).into_iter().flatten().find_map(|&k| {
                let f = &self.scenes[s].at[k];
                match f.data {
                    super::scene::FactData::RelativeCoord(a, v, base)
                        if a == axis && f.chain.contains(&usage) && self.fact_priority(s, f) != Priority::Disabled =>
                    {
                        Some((v, self.coordinate_object(base)))
                    }
                    _ => None,
                }
            })?;
            sum += step.0;
            at = step.1;
        }
        None
    }

    /// The object a member of a constraint is, for leaving out a combination that takes one
    /// object twice: the object it names, a `::position` by the object it is of (6.5, 14.1).
    fn one_object(&self, member: ObjId) -> ObjId {
        let object = self.scene_value(member);
        match self.o(object) {
            ObjData::Pos { parent } if !self.is_set(*parent) => self.scene_value(*parent),
            _ => object,
        }
    }

    /// Put `added` after `after` among the members of the collection `owner`, if any.
    fn add_to_collection(&mut self, owner: Option<ObjId>, after: ObjId, added: ObjId) {
        let Some(owner) = owner else { return };
        if let ObjData::Set(set) = self.om(owner) {
            if let SetKind::Constraints { members, .. } = &mut set.kind {
                let at = members.iter().position(|&x| x == after).map_or(members.len(), |p| p + 1);
                members.insert(at, added);
            }
        }
    }

    /// Take `removed` out of the members of the collection `owner`, if any.
    fn remove_from_collection(&mut self, owner: Option<ObjId>, removed: ObjId) {
        let Some(owner) = owner else { return };
        if let ObjData::Set(set) = self.om(owner) {
            if let SetKind::Constraints { members, .. } = &mut set.kind {
                members.retain(|&x| x != removed);
            }
        }
    }

    /// A centred graphic contributes the point it was written on, not its bounding box (14.1).
    fn constraint_point(&self, obj: ObjId) -> ObjId {
        let parent = match self.o(obj) {
            ObjData::Pos { parent } => *parent,
            _ => obj,
        };
        match self.o(parent) {
            ObjData::Prim { shape: Shape::Rectangle | Shape::Circle | Shape::Ellipse, points, .. }
                if points.len() == 1 =>
            {
                points[0]
            }
            _ => obj,
        }
    }
}
