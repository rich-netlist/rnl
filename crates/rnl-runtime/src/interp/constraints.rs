//! Written items and independent constraints in files of 0.4 (5.2, 14.1).

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
            if let Some(cause) = self.failed_input(a, ctx) {
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
