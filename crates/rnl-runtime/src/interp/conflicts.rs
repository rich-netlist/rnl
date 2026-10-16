//! The hints of a conflict of geometric facts in a level of the draft 0.3 (its 8.5). A fact
//! discarded in a conflict gives a hint that names every set of facts taken in that has no
//! solution together with it and from which no fact can be left out, each fact by the statement
//! that wrote it (7.6); each statement whose facts are in such a set gets a hint that names the
//! facts discarded because of it. They are given once the geometry of the level is resolved
//! (`phases.rs`), when the facts kept are known. A level of 0.2 gives the hints it always gave.

use std::rc::Rc;

use crate::diag::{Extra, Loc, Severity};
use crate::geom::Orient;

use super::ctx::Ctx;
use super::obj::*;
use super::scene::{Fact, FactData};
use super::Interp;

/// The conflicts found while the geometry of a level of the draft 0.3 is resolved, whose hints
/// name every set of facts taken in that a discarded fact conflicts with (its 8.5): they are given
/// once the level's geometry is resolved, when the facts kept are known (`report_discards`).
#[derive(Default)]
pub(crate) struct Conflicts {
    /// The objects and axes whose facts were found in a conflict, `'o'` for the orientation, by
    /// scene, in the order found.
    keys: Vec<(SceneId, ObjId, char)>,
    /// The constraints discarded against fixed coordinates or orientations, by scene: each with
    /// the sets of facts taken in that it conflicts with, and whether the search for these sets
    /// finished.
    constraints: Vec<(SceneId, ObjId, Vec<Vec<Fact>>, bool)>,
}

/// The hints that tell each statement which facts were discarded because of it (8.5, in the draft
/// 0.3), by the place of the statement and the origin it is shown with (7.6): each discarded fact
/// named once, in the order named.
#[derive(Default)]
struct Notes {
    items: Vec<(Loc, String, Vec<String>)>,
}

impl Notes {
    /// `discarded`, named with its place, was discarded because of the statement at `at`.
    fn add(&mut self, at: Option<Loc>, origin: String, discarded: &str) {
        let Some(at) = at else { return };
        match self.items.iter_mut().find(|(l, o, _)| *l == at && *o == origin) {
            Some((_, _, list)) => {
                if !list.iter().any(|d| d == discarded) {
                    list.push(discarded.to_string());
                }
            }
            None => self.items.push((at, origin, vec![discarded.to_string()])),
        }
    }
}

/// What a statement wrote, named by the place of the statement and the origin a diagnostic there
/// shows (7.6), such as `the x of #R1 = 0 at t.sp:9:5 [applying $PAIR (t.sp:12:5)]`.
fn named_at(what: String, at: Option<Loc>, origin: &str) -> String {
    match (at, origin) {
        (None, _) => what,
        (Some(at), "") => format!("{what} at {at}"),
        (Some(at), origin) => format!("{what} at {at} [{origin}]"),
    }
}

/// The next combination of `idx.len()` of the numbers below `n`, in lexicographic order; false
/// after the last.
fn next_combination(idx: &mut [usize], n: usize) -> bool {
    let k = idx.len();
    let Some(i) = (0..k).rev().find(|&i| idx[i] < n - k + i) else { return false };
    let first = idx[i] + 1;
    for (d, v) in idx[i..].iter_mut().enumerate() {
        *v = first + d;
    }
    true
}

impl Interp<'_> {
    /// Does the level of scene `s` follow the draft 0.3? Then the hint of a fact discarded in a
    /// conflict names every set of facts taken in that it conflicts with, and the statements in
    /// these sets are told; a level of 0.2 gives the hints it always gave.
    pub(super) fn conflicts_0_3(&self, s: SceneId) -> bool {
        self.follows_0_3(self.level_unit[self.scenes[s].level])
    }

    /// A fact lost in a conflict as the coordinates and orientations of a level of the draft 0.3
    /// are picked (`drop_fact`): its object and axis are weighed again once the pick is done.
    pub(super) fn note_conflict(&mut self, s: SceneId, f: &Fact) {
        let key = match f.data {
            FactData::Coord(axis, _) => (s, f.target, axis),
            _ => (s, f.target, 'o'),
        };
        if !self.conflicts.keys.contains(&key) {
            self.conflicts.keys.push(key);
        }
    }

    /// A constraint of a level of the draft 0.3 discarded against fixed coordinates or
    /// orientations (`check_constraints`), with the sets of facts it conflicts with.
    pub(super) fn note_discarded(
        &mut self,
        s: SceneId,
        c: ObjId,
        cd: &ConstraintData,
        members: &[ObjId],
        involved: &[Fact],
    ) {
        let (sets, complete) = self.constraint_sets(s, cd, members, involved);
        self.conflicts.constraints.push((s, c, sets, complete));
    }

    /// Where a fact is written, with the origin it is shown with (7.6): the place of its
    /// statement, or, for the coordinates of a point, where the point is written.
    fn fact_site(&self, f: &Fact) -> (Option<Loc>, String) {
        if let Some(c) = &f.ctx {
            return (self.rec_loc(c.rec, f.off), c.origin.to_string());
        }
        let made = self.birth_of.get(&f.target).map(|&i| &self.births[i]);
        match made.and_then(|b| b.stmt.map(|st| (st.rec, b.off))) {
            Some((rec, off)) => (self.rec_loc(rec, off), String::new()),
            None => (None, String::new()),
        }
    }

    /// A fact named by the statement that wrote it (8.5), with its origin as a diagnostic shows it
    /// (7.6).
    fn fact_named(&self, f: &Fact) -> String {
        let (at, origin) = self.fact_site(f);
        named_at(self.fact_text(f), at, &origin)
    }

    /// Whether two facts of one object and axis, or of its orientation, contradict each other, as
    /// the pick of the fact kept compares them: coordinates beyond the tolerance (8.5, 11.7).
    fn contradicts(&self, a: &Fact, b: &Fact) -> bool {
        match (&a.data, &b.data) {
            (FactData::Coord(_, x), FactData::Coord(_, y)) => (x - y).abs() > self.tol,
            (FactData::Orient(x), FactData::Orient(y)) => x != y,
            _ => false,
        }
    }

    /// The hints of the facts discarded in a level of the draft 0.3, once its geometry is resolved
    /// (8.5). The toolkit does not solve. Of the facts of one object on one axis, or of its
    /// orientation, it keeps the one first in the order of 8.5, and takes in with it those that do
    /// not contradict it; each of the others is discarded, and the sets of facts taken in that have
    /// no solution together with it are the kept facts that contradict it, one fact each. Sets
    /// that go through constraints are not looked for: that would take a solver. A constraint
    /// discarded against fixed coordinates or orientations names the sets found when it was
    /// weighed. Then each statement whose facts are in these sets is told which facts were
    /// discarded because of it. Discarding a `"low"` fact is expected and gives no hint (7.6).
    pub(super) fn report_discards(&mut self, s: SceneId) {
        if !self.conflicts_0_3(s) {
            return;
        }
        let mut notes = Notes::default();
        let (keys, rest): (Vec<_>, Vec<_>) =
            std::mem::take(&mut self.conflicts.keys).into_iter().partition(|k| k.0 == s);
        self.conflicts.keys = rest;
        for (_, o, axis) in keys {
            // the fact kept, and the facts weighed on the object and axis in the order they were made
            let sc = &self.scenes[s];
            let (best, weighed): (Option<usize>, Vec<(usize, Fact)>) = if axis == 'o' {
                let ks = sc.orient_live.iter().filter(|&&k| sc.orient[k].target == o);
                (sc.ori_best.get(&o).map(|b| b.2), ks.map(|&k| (k, sc.orient[k].clone())).collect())
            } else {
                let ks = sc.at_by_target.get(&o).into_iter().flatten().filter(|&&k| {
                    sc.at_live.binary_search(&k).is_ok() && matches!(sc.at[k].data, FactData::Coord(a, _) if a == axis)
                });
                (sc.at_best.get(&(o, axis)).map(|b| b.2), ks.map(|&k| (k, sc.at[k].clone())).collect())
            };
            let Some(best) = best else { continue };
            let Some(kept) = weighed.iter().find(|(k, _)| *k == best).map(|(_, f)| f.clone()) else { continue };
            let mut taken = vec![kept.clone()];
            let mut discarded = Vec::new();
            for (k, f) in weighed {
                if k == best || self.fact_priority(s, &f) == Priority::Disabled {
                    continue;
                }
                if self.contradicts(&f, &kept) {
                    discarded.push(f);
                } else {
                    taken.push(f);
                }
            }
            for f in discarded {
                if self.fact_priority(s, &f) == Priority::Low || f.ctx.is_none() {
                    continue;
                }
                self.scenes[s].dropped += 1;
                if let Some(c) = &f.ctx {
                    self.note_failed_at(c.rec, f.off, true);
                }
                let sets: Vec<Vec<Fact>> =
                    taken.iter().filter(|k| self.contradicts(&f, k)).map(|k| vec![k.clone()]).collect();
                let what = self.fact_text(&f);
                self.discard_hint(&f.ctx, f.off, &what, &sets, true, &mut notes);
            }
        }
        let (constraints, rest): (Vec<_>, Vec<_>) =
            std::mem::take(&mut self.conflicts.constraints).into_iter().partition(|c| c.0 == s);
        self.conflicts.constraints = rest;
        for (_, c, sets, complete) in constraints {
            let ObjData::Constraint(cd) = self.o(c).clone() else { continue };
            let what = self.label(c);
            self.discard_hint(&cd.site, cd.off, &what, &sets, complete, &mut notes);
        }
        // the statements by their places
        notes.items.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
        for (at, origin, discarded) in notes.items {
            self.sink.add(
                Severity::Hint,
                format!(
                    "facts discarded in the order of 8.5 because of this statement: {} (8.5)",
                    discarded.join("; ")
                ),
                Some(at),
                Extra { cause: None, origin, in_package: String::new() },
            );
        }
    }

    /// The hint of a fact discarded in a level of the draft 0.3 (8.5), at the statement that wrote
    /// it: every set of facts taken in that has no solution together with it, each fact by the
    /// statement that wrote it. Each statement in these sets is noted in `notes`.
    fn discard_hint(
        &mut self,
        site: &Option<Rc<Ctx>>,
        off: usize,
        what: &str,
        sets: &[Vec<Fact>],
        complete: bool,
        notes: &mut Notes,
    ) {
        let named: Vec<String> = sets
            .iter()
            .map(|set| {
                let facts: Vec<String> = set.iter().map(|f| self.fact_named(f)).collect();
                match facts.as_slice() {
                    [one] => one.clone(),
                    [rest @ .., last] => format!("{} and {last} together", rest.join(", ")),
                    [] => String::new(),
                }
            })
            .collect();
        let mut msg = if named.is_empty() {
            format!("{what} conflicts with the facts taken in and is discarded in the order of 8.5")
        } else {
            format!("{what} is discarded in the order of 8.5: it has no solution with {}", named.join(", nor with "))
        };
        if !complete {
            msg.push_str("; not every such set could be found, so this list may not be complete");
        }
        let (at, origin) = match site {
            Some(c) => (self.rec_loc(c.rec, off), c.origin.to_string()),
            None => (None, String::new()),
        };
        // shown like the document's own, as the later phases' diagnostics are (7.6)
        let extra = Extra { cause: None, origin: origin.clone(), in_package: String::new() };
        self.sink.add(Severity::Hint, format!("{msg} (8.5)"), at.clone(), extra);
        let discarded = named_at(what.to_string(), at, &origin);
        for f in sets.iter().flatten() {
            let (at, origin) = self.fact_site(f);
            notes.add(at, origin, &discarded);
        }
    }

    /// The sets of facts taken in that a constraint conflicts with, for its hint in a level of the
    /// draft 0.3 (8.5), when fixed coordinates violate it (fixed orientations, for
    /// ORIENT-RELATION) that all come before it in the order of 8.5. Of the kept facts that place
    /// its members on its axis, or orient them, a set is named when the constraint has no solution
    /// while the facts of the set keep their values and the members they do not fix are free, and
    /// when no fact can be left out of it; a member that no such fact fixes stays where it is
    /// drawn. The sets are searched smallest first, up to a limit. When the limit stops the
    /// search, or nothing is found that a fact could be left out of, the facts are named as one
    /// set, and the search is reported as not finished.
    fn constraint_sets(
        &mut self,
        s: SceneId,
        cd: &ConstraintData,
        members: &[ObjId],
        involved: &[Fact],
    ) -> (Vec<Vec<Fact>>, bool) {
        // the most facts searched through, and the most sets checked
        const MOST_FACTS: usize = 16;
        const MOST_CHECKS: usize = 4096;
        let relation = cd.kind == ConstraintKind::OrientRelation;
        let axis = cd.params.axis.unwrap_or('x');
        // the facts that fix the members, by index in `at`, or in `orient` for ORIENT-RELATION;
        // a member's bits are those of its facts
        let mut units: Vec<usize> = Vec::new();
        let mut bit = |it: &Self, m: ObjId| -> u64 {
            let k = if relation {
                it.scenes[s].ori_best.get(&m).map(|b| b.2)
            } else {
                let target = match it.o(m) {
                    ObjData::Term { owner, .. } => *owner,
                    _ => m,
                };
                it.scenes[s].at_best.get(&(target, axis)).map(|b| b.2)
            };
            let Some(k) = k else { return 0 };
            let i = units.iter().position(|&u| u == k).unwrap_or_else(|| {
                units.push(k);
                units.len() - 1
            });
            if i < 64 {
                1 << i
            } else {
                0
            }
        };
        // the members, each with its coordinate on the axis, or its orientation, and its bits
        let mut items: Vec<(f64, u64)> = Vec::new();
        let mut sides: Vec<Vec<(f64, u64)>> = Vec::new();
        let mut oriented: Vec<Vec<(Option<Orient>, u64)>> = Vec::new();
        if matches!(cd.kind, ConstraintKind::Distance | ConstraintKind::OrientRelation) {
            for &a in &cd.args {
                let side = if self.is_set(a) { self.flatten(s, a, 7, None) } else { vec![a] };
                let (mut coords, mut codes) = (Vec::new(), Vec::new());
                for m in side {
                    if relation {
                        let code = self.scenes[s].ori.get(&m).copied();
                        codes.push((code, bit(self, m)));
                    } else {
                        let v = self.point_of(s, m).map_or(0.0, |p| if axis == 'x' { p.0 } else { p.1 });
                        coords.push((v, bit(self, m)));
                    }
                }
                sides.push(coords);
                oriented.push(codes);
            }
        } else {
            for &m in members {
                let at = if self.is_set(m) { self.set_center(s, m) } else { self.point_of(s, m) };
                let v = at.map_or(0.0, |p| if axis == 'x' { p.0 } else { p.1 });
                let mut bits = 0;
                for l in self.leaves(s, &[m]) {
                    bits |= bit(self, l);
                }
                items.push((v, bits));
            }
        }
        let n = units.len();
        let tol = self.tol;
        // whether the constraint can hold while the facts of `set` keep their values: as
        // `satisfied` checks it, with the members that these facts do not fix free
        let holds = |set: u64| -> bool {
            let fixed = |bits: u64| bits & !set == 0;
            match cd.kind {
                ConstraintKind::Align => {
                    let mut vals = items.iter().filter(|it| fixed(it.1)).map(|it| it.0);
                    let Some(first) = vals.next() else { return true };
                    let (lo, hi) = vals.fold((first, first), |(lo, hi), v| (lo.min(v), hi.max(v)));
                    hi - lo <= tol
                }
                ConstraintKind::Order | ConstraintKind::Distribute => {
                    // the members fixed, by their places in the order
                    let at: Vec<(f64, f64)> = items
                        .iter()
                        .enumerate()
                        .filter(|(_, it)| fixed(it.1))
                        .map(|(i, it)| (i as f64, it.0))
                        .collect();
                    if cd.kind == ConstraintKind::Order {
                        // neighbours differ by one unit at least (14.3), so members with free
                        // members between them by as many units as they are places apart
                        let inc = at.windows(2).all(|w| w[1].1 - w[0].1 >= (w[1].0 - w[0].0) * (1.0 - tol));
                        let dec = at.windows(2).all(|w| w[0].1 - w[1].1 >= (w[1].0 - w[0].0) * (1.0 - tol));
                        match cd.params.sense {
                            Some("increasing") => inc,
                            Some("decreasing") => dec,
                            _ => inc || dec,
                        }
                    } else {
                        // one step between neighbours (14.4): the mean steps between the members
                        // fixed are the same
                        let mut steps = at.windows(2).map(|w| (w[1].1 - w[0].1) / (w[1].0 - w[0].0));
                        let Some(first) = steps.next() else { return true };
                        let (lo, hi) = steps.fold((first, first), |(lo, hi), v| (lo.min(v), hi.max(v)));
                        items.len() < 3 || hi - lo <= tol
                    }
                }
                ConstraintKind::Distance => {
                    // the extent of each side from its members fixed (14.5), which a free member
                    // can widen: the sides can come closer, not farther apart
                    let extent = |side: &[(f64, u64)]| {
                        let mut range: Option<(f64, f64)> = None;
                        let mut free = false;
                        for &(v, bits) in side {
                            if fixed(bits) {
                                range = Some(range.map_or((v, v), |(lo, hi)| (lo.min(v), hi.max(v))));
                            } else {
                                free = true;
                            }
                        }
                        (range, free)
                    };
                    let (Some(a), Some(b)) = (sides.first(), sides.get(1)) else { return true };
                    let ((Some(ra), free_a), (Some(rb), free_b)) = (extent(a), extent(b)) else { return true };
                    let d = match cd.params.sense {
                        Some("increasing") => rb.0 - ra.1,
                        Some("decreasing") => ra.0 - rb.1,
                        _ => (rb.0 - ra.1).max(ra.0 - rb.1),
                    };
                    cd.params.min.is_none_or(|lo| d >= lo - tol)
                        && (free_a || free_b || cd.params.max.is_none_or(|hi| d <= hi + tol))
                }
                ConstraintKind::OrientRelation => {
                    // one orientation on each side, the second the first turned by the relation (14.9)
                    let mut codes = Vec::new();
                    for side in &oriented {
                        let mut code = None;
                        for &(o, bits) in side {
                            let Some(o) = o.filter(|_| fixed(bits)) else { continue };
                            if code.is_some_and(|c| c != o) {
                                return false;
                            }
                            code = Some(o);
                        }
                        codes.push(code);
                    }
                    match (codes.first().copied().flatten(), codes.get(1).copied().flatten(), cd.params.relation) {
                        (Some(a), Some(b), Some(rel)) => b == a.then(rel),
                        _ => true,
                    }
                }
            }
        };
        let mut found: Vec<u64> = Vec::new();
        let mut complete = n <= MOST_FACTS;
        let mut checks = 0;
        if complete {
            'search: for size in 0..=n {
                let mut idx: Vec<usize> = (0..size).collect();
                loop {
                    let set = idx.iter().fold(0u64, |m, &i| m | 1 << i);
                    // one that holds a set found is not one from which no fact can be left out
                    if !found.iter().any(|&f| f & !set == 0) {
                        checks += 1;
                        if checks > MOST_CHECKS {
                            complete = false;
                            break 'search;
                        }
                        if !holds(set) {
                            found.push(set);
                            if set == 0 {
                                break 'search; // it does not hold where the members are drawn
                            }
                        }
                    }
                    if !next_combination(&mut idx, n) {
                        break;
                    }
                }
            }
        }
        let fact =
            |it: &Self, k: usize| if relation { it.scenes[s].orient[k].clone() } else { it.scenes[s].at[k].clone() };
        let mut sets: Vec<Vec<Fact>> = Vec::new();
        if !found.contains(&0) {
            for &set in &found {
                sets.push((0..n).filter(|i| set >> i & 1 == 1).map(|i| fact(self, units[i])).collect());
            }
        }
        if sets.is_empty() {
            // nothing that a fact can be left out of: the facts as one set
            let all = if n == 0 { involved.to_vec() } else { units.iter().map(|&k| fact(self, k)).collect() };
            return (vec![all], false);
        }
        (sets, complete)
    }
}
