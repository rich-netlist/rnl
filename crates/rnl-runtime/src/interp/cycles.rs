//! Cycles of sets (standard 5.2.2).
//!
//! A set depends on the sets written in the statement that makes it (inside its braces, in UNION,
//! or as the scope of a selection) and on the sets that ADD-TO and REMOVE-FROM add to it or remove
//! from it. Taking the statements in order of position (8.5), the one whose dependency closes a
//! cycle fails, and its diagnostic lists the whole cycle; the others take effect as usual.
//!
//! Order of position is the order of one level's statements (8.5): the root packages, the content
//! of the levels above (their rules, evaluated again in this level, 10.3), this level's statements
//! with the content of a package at its USE-PACKAGE, and the statements written into it with
//! `scope`. So each level has its own dependencies: those a set has from the statement that makes
//! it, which are fixed when it is made, and the edits that take effect in that level
//! (`SetData::mods_by_scene`). A set from a package, or from a level above, that several levels
//! edit holds in each level only what that level's edits give it (10.3), so edits that would form
//! a cycle only together with those of another level form none. The statements read before any
//! level have an order of their own, by record and member; they are checked only in a package
//! checked on its own (2.7), which has no level, and otherwise are checked in each level they take
//! effect in.
//!
//! The sets a statement makes are new, so its dependencies never close a cycle when it is
//! evaluated; an edit can. The edit is checked when it is evaluated, and when it closes a cycle,
//! the statement that fails is the last one, in order of position, of the cycle that order closes
//! first. That is the edit itself unless a statement placed after it was evaluated before it: a
//! package used in the level, whose rules are evaluated first; a name used before the statement
//! that binds it (1.5 item 1); an edit in what a rule generates in step 5 at the rule's earlier
//! place. Then the interpretation is run again with that statement failing from the start
//! (`Interp::closing`), so that what depends on it fails with it, whatever its order (7.2).

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use super::ctx::Ctx;
use super::eval::{pos_of_ints, Stop, R};
use super::obj::*;
use super::Interp;

/// A statement as it is evaluated in a level (`None`: before any level): its position there and
/// where it is written. It names the same statement in every run of the interpretation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StmtKey {
    pub scene: Option<SceneId>,
    pub pos: Pos,
    pub rec: Option<RecId>,
    pub off: usize,
}

impl StmtKey {
    pub fn of(ctx: &Ctx, off: usize) -> StmtKey {
        StmtKey { scene: ctx.scene, pos: ctx.pos.clone(), rec: ctx.rec, off }
    }
}

/// Where a set made of other sets is made: its statement, and the member of a unit that statement
/// is written in, which gives its position in the other levels.
#[derive(Clone, Debug)]
pub struct MadeAt {
    pub key: StmtKey,
    pub member: MemberKey,
}

/// A dependency: `from` depends on `to`.
#[derive(Clone, Copy, Debug)]
struct Dep {
    from: ObjId,
    to: ObjId,
    by: By,
}

#[derive(Clone, Copy, Debug)]
enum By {
    /// The statement that makes `from`.
    Made,
    /// An edit of `from`: its index in the set's `mods`.
    Edit(usize),
    /// The edit being checked.
    New,
}

impl Interp<'_> {
    /// Fail a statement that an earlier run found to close a cycle of sets (5.2.2).
    pub(crate) fn check_closing(&mut self, ctx: &Ctx, off: usize) -> R<()> {
        if self.closing.is_empty() {
            return Ok(());
        }
        match self.closing.get(&StmtKey::of(ctx, off)) {
            Some(msg) => {
                let msg = msg.clone();
                self.fail(ctx, msg, off)
            }
            None => Ok(()),
        }
    }

    /// Note where a set is made when it is made of other sets (5.2.2).
    pub(crate) fn note_made(&mut self, set: ObjId, ctx: &Ctx, off: usize) {
        if self.set(set).made_of().any(|o| self.is_set(o)) {
            self.made_at.insert(set, MadeAt { key: StmtKey::of(ctx, off), member: ctx.member_key.clone() });
        }
    }

    /// An edit makes its target depend on its source (5.2.2). When that closes a cycle of sets in
    /// the level, the last statement of the cycle in order of position fails: this edit, or one
    /// evaluated before it, and then the interpretation is run again.
    pub(crate) fn check_edit(&mut self, target: ObjId, m: &SetMod, ctx: &Ctx) -> R<()> {
        self.check_closing(ctx, m.off)?;
        let scene = ctx.scene;
        if scene.is_none() && self.package_main.is_none() {
            return Ok(());
        }
        if !self.is_set(m.source) {
            return Ok(());
        }
        // a set is made of sets made before it, so what it depends on was made before it, or
        // before the latest made source of the edits that make a set depend on a later one; a
        // target made after both cannot be among it, and nothing need be gone through
        let later = self.later_sources.get(&scene).copied();
        let closes = if target > m.source && later.is_none_or(|l| target > l) {
            false
        } else {
            self.depends(m.source, target, scene)
        };
        if !closes {
            if m.source > target && later.is_none_or(|l| m.source > l) {
                self.later_sources.insert(scene, m.source);
            }
            return Ok(());
        }
        let mut cycle = vec![(Dep { from: target, to: m.source, by: By::New }, ctx.pos.clone())];
        cycle.extend(self.earliest_path(m.source, target, scene, &ctx.pos));
        // the last in order of position; of several from the same statement, this edit
        let mut last = 0;
        for (i, (_, p)) in cycle.iter().enumerate() {
            if *p > cycle[last].1 {
                last = i;
            }
        }
        let deps: Vec<Dep> = cycle.into_iter().map(|(d, _)| d).collect();
        if last != 0 {
            match self.dep_key(&deps[last]) {
                // placed after this edit, but evaluated before it: the run stops here
                Some(key) if !self.closing.contains_key(&key) => {
                    if self.restart.is_none() {
                        let msg = self.cycle_message(&deps, last, ctx, m.off);
                        self.restart = Some((key, msg));
                    }
                    return Err(Stop::Fail(None));
                }
                // one that failed from the start yet was evaluated: this edit fails instead, so
                // that the runs come to an end
                _ => last = 0,
            }
        }
        let msg = self.cycle_message(&deps, last, ctx, m.off);
        self.fail(ctx, msg, m.off)
    }

    /// Whether a set depends on another in a level, through any number of sets (5.2.2).
    fn depends(&self, from: ObjId, to: ObjId, scene: Option<SceneId>) -> bool {
        if from == to {
            return true;
        }
        let mut seen = HashSet::new();
        let mut todo = vec![from];
        let mut deps = Vec::new();
        while let Some(x) = todo.pop() {
            self.deps_in(x, scene, &mut deps);
            for d in deps.drain(..) {
                if d.to == to {
                    return true;
                }
                if seen.insert(d.to) {
                    todo.push(d.to);
                }
            }
        }
        false
    }

    /// The path of dependencies from one set to another that order of position completes first:
    /// the one whose last dependency comes earliest (8.5), with the position of each. `to` must
    /// depend on `from`; `floor` is the position of the edit that closes the cycle.
    fn earliest_path(&mut self, from: ObjId, to: ObjId, scene: Option<SceneId>, floor: &Pos) -> Vec<(Dep, Pos)> {
        let mut best: HashMap<ObjId, Pos> = HashMap::new();
        let mut came: HashMap<ObjId, (Dep, Pos)> = HashMap::new();
        let mut heap = BinaryHeap::new();
        best.insert(from, floor.clone());
        heap.push(Reverse((floor.clone(), from)));
        while let Some(Reverse((key, x))) = heap.pop() {
            if x == to {
                break;
            }
            if best.get(&x).is_some_and(|b| *b < key) {
                continue;
            }
            let mut deps = Vec::new();
            self.deps_in(x, scene, &mut deps);
            for d in deps {
                let p = self.dep_pos(&d, scene);
                let k = if p > key { p.clone() } else { key.clone() };
                if best.get(&d.to).is_some_and(|b| *b <= k) {
                    continue;
                }
                best.insert(d.to, k.clone());
                came.insert(d.to, (d, p));
                heap.push(Reverse((k, d.to)));
            }
        }
        let mut path = Vec::new();
        let mut at = to;
        while at != from {
            let (d, p) = came.remove(&at).expect("a set on the path");
            at = d.from;
            path.push((d, p));
        }
        path.reverse();
        path
    }

    /// The sets a set depends on in a level (5.2.2): the sets written in the statement that makes
    /// it, and the sources of its edits that take effect in the level that are sets.
    fn deps_in(&self, x: ObjId, scene: Option<SceneId>, out: &mut Vec<Dep>) {
        let ObjData::Set(sd) = self.o(x) else { return };
        for o in sd.made_of() {
            if self.is_set(o) {
                out.push(Dep { from: x, to: o, by: By::Made });
            }
        }
        for &k in sd.mods_by_scene.get(&scene).into_iter().flatten() {
            let source = sd.mods[k].source;
            if self.is_set(source) {
                out.push(Dep { from: x, to: source, by: By::Edit(k) });
            }
        }
    }

    /// The position in a level of the statement a dependency comes from (8.5).
    fn dep_pos(&mut self, d: &Dep, scene: Option<SceneId>) -> Pos {
        match d.by {
            By::Edit(k) => self.set(d.from).mods[k].pos.clone(),
            By::Made => match self.made_at.get(&d.from).cloned() {
                Some(at) => self.made_pos(&at, scene),
                None => Pos::new(),
            },
            By::New => unreachable!("the edit being checked has the position of its context"),
        }
    }

    /// The position of a set's statement in a level's order (8.5). A set is made once: in the
    /// level of the name it is bound to, or before any level in a package. In another level its
    /// statement has the position of the member it is written in there; a statement that cannot be
    /// placed so counts as first.
    fn made_pos(&mut self, at: &MadeAt, scene: Option<SceneId>) -> Pos {
        if at.key.scene == scene {
            return at.key.pos.clone();
        }
        let (Some(s), MemberKey::M { unit, rec, k, embedded }) = (scene, &at.member) else { return Pos::new() };
        let (unit, k, embedded) = (*unit, *k, *embedded);
        let rec = (*rec != usize::MAX).then_some(*rec);
        // as `eval_member_at` places a member
        let base = |it: &mut Self, scene: Option<SceneId>| match scene {
            Some(sc) => it.position_of(sc, unit, rec, if embedded { None } else { Some(k) }),
            None => pos_of_ints(&[9, 0, rec.map(|r| r as i64).unwrap_or(0), k as i64]),
        };
        let home = base(self, at.key.scene);
        let Some(rest) = at.key.pos.strip_prefix(&home[..]) else { return Pos::new() };
        let mut p = base(self, Some(s));
        p.extend_from_slice(rest);
        p
    }

    /// The statement a dependency comes from, other than the edit being checked.
    fn dep_key(&self, d: &Dep) -> Option<StmtKey> {
        match d.by {
            By::Edit(k) => {
                let m = &self.set(d.from).mods[k];
                Some(StmtKey { scene: m.ctx.scene, pos: m.pos.clone(), rec: m.ctx.rec, off: m.off })
            }
            By::Made => self.made_at.get(&d.from).map(|at| at.key.clone()),
            By::New => None,
        }
    }

    /// "a cycle of sets: %a depends on %b here, %b on %c at a.sp:2:5, and %c on %a at a.sp:2:30",
    /// starting from the dependency of the statement that fails.
    fn cycle_message(&self, deps: &[Dep], last: usize, ctx: &Ctx, off: usize) -> String {
        let n = deps.len();
        // (from, to, place), the statement that fails first
        let mut links: Vec<(ObjId, ObjId, String)> = Vec::new();
        for i in 0..n {
            let d = &deps[(last + i) % n];
            let loc = match d.by {
                _ if i == 0 => None,
                By::New => self.rec_loc(ctx.rec, off),
                By::Edit(k) => {
                    let m = &self.set(d.from).mods[k];
                    self.rec_loc(m.ctx.rec, m.off)
                }
                By::Made => self.made_at.get(&d.from).and_then(|at| self.rec_loc(at.key.rec, at.key.off)),
            };
            let place = if i == 0 { " here".to_string() } else { loc.map(|l| format!(" at {l}")).unwrap_or_default() };
            links.push((d.from, d.to, place));
        }
        // a set that its statement makes by putting several objects together for one slot, as in
        // `%a %b @UNION`, is not written: what depends on it depends on its members there
        let mut i = 0;
        while links.len() > 1 && i < links.len() {
            let j = (i + 1) % links.len();
            if links[i].1 == links[j].0 && self.unwritten(links[i].1) {
                let (_, to, place) = links.remove(j);
                let k = if j < i { i - 1 } else { i };
                links[k].1 = to;
                if j == 0 {
                    links[k].2 = place;
                    links.rotate_right(1);
                }
                i = 0;
            } else {
                i += 1;
            }
        }
        let n = links.len();
        let mut parts = Vec::new();
        for (i, (from, to, place)) in links.iter().enumerate() {
            let to = if to == from { "itself".to_string() } else { self.set_name(*to) };
            let verb = if i == 0 { "depends on" } else { "on" };
            let and = if i > 0 && i == n - 1 { "and " } else { "" };
            parts.push(format!("{and}{} {verb} {to}{place}", self.set_name(*from)));
        }
        format!(
            "a cycle of sets: {}; taken in order of position, this statement closes it and fails (5.2.2)",
            parts.join(", ")
        )
    }

    /// A set that no `{…}`, name or keyword makes: several objects put together for one slot.
    fn unwritten(&self, x: ObjId) -> bool {
        let s = self.set(x);
        self.objs[x.0 as usize].name.is_empty() && matches!(s.kind, SetKind::Plain) && !s.explicit
    }

    /// A set as a diagnostic of a cycle names it: by its name, or by what makes it.
    fn set_name(&self, x: ObjId) -> String {
        let name = &self.objs[x.0 as usize].name;
        if name.starts_with('$') {
            return name.clone();
        }
        if !name.is_empty() {
            return format!("%{name}");
        }
        let s = self.set(x);
        match &s.kind {
            SetKind::Select { keyword, .. } => format!("a {keyword}"),
            SetKind::Union { .. } => "a UNION".to_string(),
            SetKind::Gen { .. } => "a generated set".to_string(),
            SetKind::Plain if s.explicit => "{…}".to_string(),
            SetKind::Plain => "a set".to_string(),
        }
    }
}
