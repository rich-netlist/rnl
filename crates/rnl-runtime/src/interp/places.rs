//! Member places (5.2.3) read written sets after all their edits, including later statements.
//!
//! A reading records which edits each reference has seen. At the end of the interpretation,
//! an unseen edit becomes a dependency of that reference and the reading starts again. On the
//! next reading the edit's statement is evaluated before the reference, through the ordinary
//! member memoisation. Only statement locations survive a restart, never object handles or
//! provisional facts. Dependencies grow monotonically, as with the set-cycle checks (5.2.2).
//! This also covers edits in definition bodies and nested sets, whose members are registered
//! before their first member is evaluated. Files without the new syntax keep their evaluation.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::diag::codes;
use crate::syntax::{Member, MemberPlaces, SetExpr};

use super::ctx::{Ctx, NestedRef};
use super::eval::{pos_of_ints, Stop, R};
use super::obj::*;
use super::unit::UnitMember;
use super::Interp;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Site {
    scene: Option<SceneId>,
    member: MemberKey,
    pos: Pos,
}

impl Site {
    fn of(ctx: &Ctx) -> Self {
        Self { scene: ctx.scene, member: ctx.member_key.clone(), pos: order_pos(&ctx.pos, &ctx.segments) }
    }
}

/// Inline-package members can have the same position. Both the root member and the pipe
/// segments belong to a reference's identity, including references inside repeated usages.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Query {
    site: Site,
    rec: Option<RecId>,
    off: usize,
}

#[derive(Clone, Default)]
pub(super) struct Plan {
    edits: HashSet<Site>,
    selection: bool,
}

struct Read {
    key: Query,
    source: ObjId,
    seen: HashSet<Site>,
}

#[derive(Clone)]
enum Task {
    Root(UnitId, UnitMember, Option<SceneId>),
    Body(Rc<Member>, usize, Ctx),
    Nested(Rc<Member>, Ctx),
}

#[derive(Default)]
pub(super) struct Places {
    pub enabled: bool,
    pub plans: HashMap<Query, Plan>,
    /// Bindings that need the drawing level because they read its final set membership.
    pub rules: HashSet<MemberKey>,
    reads: Vec<Read>,
    tasks: HashMap<Site, Task>,
    active: HashSet<Site>,
}

impl Interp<'_> {
    pub(super) fn register_place_root(&mut self, u: UnitId, m: &UnitMember, scene: Option<SceneId>) {
        if !self.places.enabled {
            return;
        }
        let pos = match scene {
            Some(s) => self.position_of(s, u, m.rec, if m.embedded { None } else { Some(m.k) }),
            None => pos_of_ints(&[9, 0, m.rec.map_or(0, |r| r as i64), m.k as i64]),
        };
        self.places.tasks.insert(Site { scene, member: m.key(u), pos }, Task::Root(u, m.clone(), scene));
    }

    pub(super) fn register_place_body(&mut self, body: &SetExpr, ctx: &Ctx) {
        if !self.places.enabled {
            return;
        }
        for (k, m) in body.members.iter().enumerate() {
            let sub = ctx.with_pos(pos_plus(&ctx.pos, "b", k as i64));
            self.places.tasks.insert(Site::of(&sub), Task::Body(m.clone(), k, ctx.clone()));
        }
    }

    pub(super) fn register_place_nested(&mut self, set: &SetExpr, ctx: &Ctx, nested: &NestedRef) {
        if !self.places.enabled {
            return;
        }
        for (k, m) in set.members.iter().enumerate() {
            let mut sub = ctx.clone();
            sub.nested = Some(nested.clone());
            sub.pos.extend([PosElem::Tag("s"), PosElem::Int(set.start as i64), PosElem::Int(k as i64)]);
            self.places.tasks.insert(Site::of(&sub), Task::Nested(m.clone(), sub));
        }
    }

    fn place_edit(&mut self, site: &Site, ctx: &Ctx, off: usize) -> R<()> {
        // A future expansion is not registered yet: evaluate its closest enclosing member.
        let mut at = site.clone();
        let task = loop {
            if let Some(task) = self.places.tasks.get(&at) {
                break task.clone();
            }
            if at.pos.pop().is_none() {
                return Ok(());
            }
        };
        if !self.places.active.insert(at.clone()) {
            return self.fail(
                ctx,
                codes::REF_NAME_CYCLE,
                "member places and the edits of their set depend on each other in a cycle (5.2.3)",
                off,
            );
        }
        let result = match task {
            Task::Root(u, m, scene) => self.eval_member_at(u, m.rec, m.k, m.embedded, &m.m, scene, m.header, None, ""),
            Task::Body(m, k, ctx) => self.definition_member(&m, k, &ctx),
            Task::Nested(m, ctx) => self.nested_member(&m, &ctx, ctx.nested.as_ref().unwrap()),
        };
        self.places.active.remove(&at);
        // A failed edit takes no effect. Its own diagnostic is retained, as for any other edit.
        match result {
            Err(Stop::Fail(d)) => Err(Stop::Fail(d)),
            _ => Ok(()),
        }
    }

    /// Selection dependency is structural, separate from rule classification (`is_lazy`). In
    /// particular a plain set edited by a selection can be classified as not lazy (10.4).
    fn place_dependencies(&self, source: ObjId, scene: Option<SceneId>) -> (HashSet<Site>, bool) {
        let mut todo = vec![(source, true)];
        let mut seen = HashSet::new();
        let mut edits = HashSet::new();
        let mut selection = false;
        while let Some((o, counted)) = todo.pop() {
            if !seen.insert((o, counted)) || !self.is_set(o) {
                continue;
            }
            let set = self.set(o);
            match &set.kind {
                SetKind::Select { .. } | SetKind::Gen { .. } => selection = true,
                SetKind::Union { source } => todo.push((*source, counted)),
                SetKind::Plain => {}
            }
            for &(item, spread) in &set.items {
                let expanded = self.is_set(item) && (spread || !self.set(item).explicit);
                todo.push((item, counted && expanded));
            }
            for &k in set.mods_by_scene.get(&scene).into_iter().flatten() {
                let m = &set.mods[k];
                // A directly written inner set counts as one member regardless of its edits;
                // those edits still count when checking for a selection dependency.
                if counted {
                    edits.insert(Site::of(&m.ctx));
                }
                todo.push((m.source, counted));
                selection |= m.ctx.selected || m.ctx.frame_generated();
            }
        }
        (edits, selection)
    }

    pub(super) fn settle_places(&mut self) -> bool {
        let mut changed = false;
        for read in std::mem::take(&mut self.places.reads) {
            let (edits, selection) = self.place_dependencies(read.source, read.key.site.scene);
            let plan = self.places.plans.entry(read.key).or_default();
            for edit in edits.difference(&read.seen) {
                changed |= plan.edits.insert(edit.clone());
            }
            if selection && !plan.selection {
                plan.selection = true;
                changed = true;
            }
        }
        changed
    }

    pub(super) fn member_places(&mut self, source: ObjId, places: &MemberPlaces, off: usize, ctx: &Ctx) -> R<ObjId> {
        if ctx.scene.is_none() && self.package_main.is_none() {
            // A package's set is edited independently in each drawing level. Bindings that
            // read it are consequently evaluated there, including aliases of those bindings.
            for (key, scene) in &self.in_progress {
                if scene.is_none() {
                    self.places.rules.insert(key.clone());
                    self.rules.insert(key.clone());
                }
            }
            return Err(Stop::NoCircuit);
        }
        if !self.is_set(source) {
            return self.fail(
                ctx,
                codes::SET_PLACES_WRONG_KIND,
                format!("member places require a set, not {} ({}) (5.2.3)", self.label(source), self.kind_of(source)),
                off,
            );
        }
        let key = Query { site: Site::of(ctx), rec: ctx.rec, off };
        let plan = self.places.plans.get(&key).cloned().unwrap_or_default();
        let (already, selection) = self.place_dependencies(source, ctx.scene);
        if !plan.selection && !selection {
            let mut edits: Vec<_> = plan.edits.difference(&already).cloned().collect();
            edits.sort_by(|a, b| (&a.pos, &a.member).cmp(&(&b.pos, &b.member)));
            for edit in edits {
                self.place_edit(&edit, ctx, off)?;
            }
        }
        let (seen, selection) = self.place_dependencies(source, ctx.scene);
        self.places.reads.push(Read { key, source, seen });
        if plan.selection || selection {
            return self.fail(
                ctx,
                codes::SET_PLACES_OF_SELECTION,
                "member places cannot be taken from a selection or a set that depends on a selection (5.2.3)",
                off,
            );
        }
        let members = self.written_members(source, ctx.scene, &mut HashSet::new());
        let selected: Vec<_> = places.expand(members.len()).into_iter().map(|i| members[i]).collect();
        if places.single() && selected.len() == 1 {
            return Ok(selected[0]);
        }
        let set = self
            .new_obj(ObjData::Set(Box::new(SetData::plain(selected.into_iter().map(|o| (o, false)).collect(), true))));
        self.note_made(set, ctx, off);
        Ok(set)
    }

    /// Read one level of a written set without the phase caches: more edits may still be found
    /// in this provisional reading. The selected handles themselves are kept, including sets.
    fn written_members(&self, obj: ObjId, scene: Option<SceneId>, busy: &mut HashSet<ObjId>) -> Vec<ObjId> {
        if obj == LEVEL {
            return vec![scene.map_or(LEVEL, |s| self.scenes[s].levelobj)];
        }
        if !self.is_set(obj) {
            return if self.is_failed(obj) { Vec::new() } else { vec![obj] };
        }
        if !busy.insert(obj) {
            return Vec::new();
        }
        let set = self.set(obj);
        let mut out = match set.kind {
            SetKind::Union { source } => {
                let mut unique = HashSet::new();
                self.written_members(source, scene, busy).into_iter().filter(|o| unique.insert(*o)).collect()
            }
            _ => {
                let mut out = Vec::new();
                for &(o, spread) in &set.items {
                    if self.is_failed(o) {
                        continue;
                    }
                    if o == LEVEL || (self.is_set(o) && (spread || !self.set(o).explicit)) {
                        out.extend(self.written_members(o, scene, busy));
                    } else {
                        out.push(o);
                    }
                }
                out
            }
        };
        let mut edits = set.mods_by_scene.get(&scene).cloned().unwrap_or_default();
        edits.sort_by_cached_key(|&k| {
            let m = &set.mods[k];
            (order_pos(&m.pos, &m.ctx.segments), m.ctx.member_key.clone(), m.off)
        });
        for k in edits {
            let edit = &set.mods[k];
            let members = self.written_members(edit.source, scene, busy);
            if edit.add {
                out.extend(members);
            } else {
                let removed: HashSet<_> = members.into_iter().collect();
                out.retain(|o| !removed.contains(o));
            }
        }
        busy.remove(&obj);
        out
    }
}
