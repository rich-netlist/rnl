//! Exact constraints for one skeleton axis: offset classes, a longest-path graph,
//! and the shared simplex for general linear bounds. Optional trials are transactional;
//! rejected or interrupted trials restore the previous graph and class values.

use std::collections::{BTreeSet, VecDeque};

use num_traits::Zero;

use crate::rational::Q;
use crate::simplex::{Failure, LinExpr, Rel, Simplex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Why {
    Fact(usize),
    Body,
    Wire,
    Apart(usize, usize),
    Group,
    Candidate(usize),
    Finish,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Kept,
    Conflict(Vec<Why>),
    Undecided,
}

#[derive(Clone, Debug)]
struct Edge {
    to: usize,
    w: Q,
    why: Why,
}

#[derive(Clone, Debug)]
enum Undo {
    Least(usize, Q),
    Union { child: usize, root: usize, members: usize, wall: Q },
    Edge(usize),
}

const RELAX_PER_VAR: u64 = 64;

pub(crate) struct Store {
    failure: Option<Outcome>,
    fixed_conflict: Option<Vec<Why>>,
    parent: Vec<usize>,
    off: Vec<Q>,
    link: Vec<Vec<Why>>,
    size: Vec<usize>,
    members: Vec<Vec<usize>>,
    wall: Vec<Q>,
    out: Vec<Vec<Edge>>,
    least: Vec<Q>,
    zero: usize,
    general: Vec<(LinExpr, Option<Q>, Option<Q>, Why)>,
    sx: Option<Simplex<Why>>,
    pending: Vec<(LinExpr, Rel, Q, Why)>,
    trail: Vec<Undo>,
    pred: Vec<Option<(usize, usize, Why)>>,
    pub(crate) relaxations: u64,
    pub(crate) pivots: u64,
    pub(crate) pivot_rows: u64,
    pivot_cap: u64,
    grid: Option<(Q, BTreeSet<usize>)>,
    rounded: std::collections::BTreeMap<Why, Q>,
}

impl Store {
    pub(crate) fn new(walls: Vec<Q>, zero: usize, pivot_cap: u64) -> Store {
        let n = walls.len();
        Store {
            failure: None,
            fixed_conflict: None,
            parent: (0..n).collect(),
            off: vec![Q::zero(); n],
            link: vec![Vec::new(); n],
            size: vec![1; n],
            members: (0..n).map(|v| vec![v]).collect(),
            least: walls.clone(),
            wall: walls,
            out: vec![Vec::new(); n],
            zero,
            general: Vec::new(),
            sx: None,
            pending: Vec::new(),
            trail: Vec::new(),
            pred: vec![None; n],
            relaxations: 0,
            pivots: 0,
            pivot_rows: 0,
            pivot_cap,
            grid: None,
            rounded: std::collections::BTreeMap::new(),
        }
    }

    pub(crate) fn round_to_grid(&mut self, grid: Q, off: BTreeSet<usize>) {
        self.grid = Some((grid, off));
    }

    pub(crate) fn whole_gap(&self, a: usize, ca: &Q, b: usize, cb: &Q, w: &Q) -> Q {
        match &self.grid {
            Some((g, off)) if !off.contains(&a) && !off.contains(&b) => {
                let weight = w + cb - ca;
                (&weight / g).ceil() * g - cb + ca
            }
            _ => w.clone(),
        }
    }

    pub(crate) fn mark_rounded(&mut self, why: Why, more: Q) {
        let e = self.rounded.entry(why).or_insert_with(Q::zero);
        if more > *e {
            *e = more;
        }
    }

    pub(crate) fn grid_in(&self, whys: &[Why]) -> bool {
        !self.rounded.is_empty() && (whys.is_empty() || whys.iter().any(|w| self.rounded.contains_key(w)))
    }

    pub(crate) fn grid_in_unsettled(&self) -> bool {
        if let Some(why) = &self.fixed_conflict {
            return self.grid_in(why);
        }
        if self.rounded.is_empty() {
            return false;
        }
        let Some(edges) = self.cycle_edges() else { return true };
        let mut weight = Q::zero();
        for (m, to, why) in edges {
            let w = self.out[m].iter().filter(|e| e.to == to && e.why == why).map(|e| e.w.clone()).max();
            let Some(w) = w else { return true };
            weight += &self.find(m).1 + &w - &self.find(to).1;
            if let Some(more) = self.rounded.get(&why) {
                weight -= more;
            }
        }
        !weight.is_positive()
    }

    fn cycle_edges(&self) -> Option<Vec<(usize, usize, Why)>> {
        for start in (0..self.len()).filter(|&r| self.parent[r] == r && self.pred[r].is_some()) {
            let mut seen = std::collections::BTreeSet::new();
            let mut at = start;
            while seen.insert(at) {
                let Some((m, _, _)) = self.pred[at] else { break };
                at = self.root(m);
            }
            if self.pred[at].is_some() && seen.contains(&at) {
                let mut out = Vec::new();
                let r = at;
                for _ in 0..=self.len() {
                    let edge = self.pred[at]?;
                    out.push(edge);
                    let next = self.root(edge.0);
                    if next == r {
                        return Some(out);
                    }
                    at = next;
                }
                return None;
            }
        }
        None
    }

    pub(crate) fn any_rounded(&self) -> bool {
        !self.rounded.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.parent.len()
    }

    pub(crate) fn find(&self, v: usize) -> (usize, Q) {
        let mut at = v;
        let mut offset = Q::zero();
        while self.parent[at] != at {
            offset += &self.off[at];
            at = self.parent[at];
        }
        (at, offset)
    }

    pub(crate) fn root(&self, v: usize) -> usize {
        let mut at = v;
        while self.parent[at] != at {
            at = self.parent[at];
        }
        at
    }

    pub(crate) fn value(&self, v: usize) -> Q {
        let (r, o) = self.find(v);
        &self.least[r] + &o
    }

    pub(crate) fn has_general(&self) -> bool {
        !self.general.is_empty()
    }

    pub(crate) fn counts(&self) -> (usize, usize) {
        (self.general.len(), self.out.iter().map(Vec::len).sum())
    }

    pub(crate) fn equal(&mut self, a: usize, ca: &Q, b: usize, cb: &Q, why: Why) {
        let (ra, oa) = self.find(a);
        let (rb, ob) = self.find(b);
        let d = &ob + cb - &oa - ca;
        if ra == rb {
            if !d.is_zero() && self.fixed_conflict.is_none() {
                let mut reasons = self.reasons(a, b, why);
                reasons.push(why);
                self.fixed_conflict = Some(reasons);
            }
            return;
        }
        let reasons = self.reasons(a, b, why);
        self.unite(ra, rb, d, reasons);
        self.pending.push((LinExpr::diff(a, b), Rel::Eq, cb - ca, why));
    }

    pub(crate) fn at_least(&mut self, a: usize, ca: &Q, b: usize, cb: &Q, w: &Q, why: Why) {
        let weight = w + cb - ca;
        if a == b {
            if weight > Q::zero() && self.fixed_conflict.is_none() {
                self.fixed_conflict = Some(vec![why]);
            }
            return;
        }
        self.out[b].push(Edge { to: a, w: weight.clone(), why });
        self.pending.push((LinExpr::diff(a, b), Rel::Ge, weight, why));
    }

    pub(crate) fn bound(&mut self, e: &LinExpr, lo: Option<&Q>, hi: Option<&Q>, why: Why) {
        let c = e.constant_part().clone();
        match e.terms() {
            [] => {
                if (lo.is_some_and(|lo| c < *lo) || hi.is_some_and(|hi| c > *hi)) && self.fixed_conflict.is_none() {
                    self.fixed_conflict = Some(vec![why]);
                }
            }
            [(v, a)] => {
                let (lo, hi) = scaled(lo, hi, &c, a);
                self.difference(*v, self.zero, lo.as_ref(), hi.as_ref(), why);
            }
            [(u, a), (v, b)] if (a + b).is_zero() => {
                let (lo, hi) = scaled(lo, hi, &c, a);
                self.difference(*u, *v, lo.as_ref(), hi.as_ref(), why);
            }
            _ => {
                self.general.push((e.clone(), lo.cloned(), hi.cloned(), why));
                if let Some(lo) = lo {
                    self.pending.push((e.clone(), Rel::Ge, lo.clone(), why));
                }
                if let Some(hi) = hi {
                    self.pending.push((e.clone(), Rel::Le, hi.clone(), why));
                }
            }
        }
    }

    fn difference(&mut self, u: usize, v: usize, lo: Option<&Q>, hi: Option<&Q>, why: Why) {
        let zero = Q::zero();
        match (lo, hi) {
            (Some(l), Some(h)) if l == h => self.equal(u, &zero, v, l, why),
            _ => {
                if let Some(l) = lo {
                    self.at_least(u, &zero, v, &zero, l, why);
                }
                if let Some(h) = hi {
                    self.at_least(v, &zero, u, &zero, &-h, why);
                }
            }
        }
    }

    fn reasons(&self, a: usize, b: usize, why: Why) -> Vec<Why> {
        let mut out = self.links(a);
        out.extend(self.links(b));
        if matches!(why, Why::Fact(_) | Why::Candidate(_)) {
            out.push(why);
        }
        out.sort();
        out.dedup();
        out
    }

    fn unite(&mut self, ra: usize, rb: usize, d: Q, reasons: Vec<Why>) -> usize {
        let (root, child, off) = if (self.size[ra], rb) > (self.size[rb], ra) { (ra, rb, -d) } else { (rb, ra, d) };
        self.trail.push(Undo::Union { child, root, members: self.members[root].len(), wall: self.wall[root].clone() });
        self.trail.push(Undo::Least(root, self.least[root].clone()));
        self.parent[child] = root;
        self.off[child] = off.clone();
        self.link[child] = reasons;
        self.size[root] += self.size[child];
        let moved = self.members[child].clone();
        self.members[root].extend(moved);
        let wall = &self.wall[child] - &off;
        if wall > self.wall[root] {
            self.wall[root] = wall;
        }
        let least = &self.least[child] - &off;
        if least > self.least[root] {
            self.least[root] = least;
        }
        root
    }

    pub(crate) fn settle(&mut self) -> bool {
        if let Some(why) = &self.fixed_conflict {
            self.failure = Some(Outcome::Conflict(why.clone()));
            return false;
        }
        let roots: Vec<usize> = (0..self.len()).filter(|&v| self.parent[v] == v).collect();
        for &r in &roots {
            self.least[r] = self.wall[r].clone();
        }
        self.failure = match self.relax(roots, None) {
            Err(None) => Some(Outcome::Undecided),
            Err(Some(why)) => Some(Outcome::Conflict(why)),
            Ok(()) if self.zero_holds() => None,
            Ok(()) => Some(Outcome::Conflict(self.cycle(self.root(self.zero)))),
        };
        let ok = self.failure.is_none();
        self.trail.clear();
        ok
    }

    pub(crate) fn unsettled(&self) -> Vec<Why> {
        if let Some(why) = &self.fixed_conflict {
            return why.clone();
        }
        for start in (0..self.len()).filter(|&r| self.parent[r] == r && self.pred[r].is_some()) {
            let mut seen = std::collections::BTreeSet::new();
            let mut at = start;
            while seen.insert(at) {
                let Some((m, _, _)) = self.pred[at] else { break };
                at = self.root(m);
            }
            if self.pred[at].is_some() && seen.contains(&at) {
                return self.cycle(at);
            }
        }
        Vec::new()
    }

    fn zero_holds(&self) -> bool {
        !self.value(self.zero).is_positive()
    }

    fn relax(&mut self, queue: Vec<usize>, start: Option<usize>) -> Result<(), Option<Vec<Why>>> {
        let budget = RELAX_PER_VAR * self.len() as u64 + 1000;
        let mut made = 0u64;
        let mut waiting = vec![false; self.len()];
        for &r in &queue {
            waiting[r] = true;
        }
        self.pred.fill(None);
        let mut fifo: VecDeque<usize> = queue.into_iter().collect();
        while let Some(r) = fifo.pop_front() {
            if !crate::budget::permit(crate::budget::Step::Structure, self.members[r].len()) {
                return Err(None);
            }
            waiting[r] = false;
            let members = self.members[r].clone();
            for m in members {
                let base = &self.least[r] + &self.find(m).1;
                for e in 0..self.out[m].len() {
                    if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                        return Err(None);
                    }
                    let (to, w, why) = {
                        let e = &self.out[m][e];
                        (e.to, e.w.clone(), e.why)
                    };
                    let (rt, ot) = self.find(to);
                    let need = &base + &w - &ot;
                    if need > self.least[rt] {
                        made += 1;
                        self.relaxations += 1;
                        if made > budget {
                            return Err(None);
                        }
                        self.trail.push(Undo::Least(rt, self.least[rt].clone()));
                        self.least[rt] = need;
                        self.pred[rt] = Some((m, to, why));
                        // An inequality within one merged class that raises its
                        // own value is already a positive cycle, even at settle.
                        if Some(rt) == start || rt == r {
                            return Err(Some(self.cycle(rt)));
                        }
                        if !waiting[rt] {
                            waiting[rt] = true;
                            fifo.push_back(rt);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn cycle(&self, r: usize) -> Vec<Why> {
        let mut out = Vec::new();
        let mut at = r;
        for _ in 0..=self.len() {
            let Some((m, to, why)) = self.pred[at] else { break };
            out.push(why);
            out.extend(self.links(m));
            out.extend(self.links(to));
            let next = self.root(m);
            if next == r {
                break;
            }
            at = next;
        }
        out.sort();
        out.dedup();
        out
    }

    fn links(&self, v: usize) -> Vec<Why> {
        let mut out = Vec::new();
        let mut at = v;
        while self.parent[at] != at {
            out.extend(self.link[at].iter().copied());
            at = self.parent[at];
        }
        out
    }

    pub(crate) fn try_equal(&mut self, a: usize, ca: &Q, b: usize, cb: &Q, why: Why) -> Outcome {
        if !crate::budget::permit(crate::budget::Step::Structure, self.len()) {
            return Outcome::Undecided;
        }
        let mark = self.trail.len();
        let (ra, oa) = self.find(a);
        let (rb, ob) = self.find(b);
        let d = &ob + cb - &oa - ca;
        if ra == rb {
            if d.is_zero() {
                return Outcome::Kept;
            }
            let mut whys = self.links(a);
            whys.extend(self.links(b));
            return Outcome::Conflict(with(whys, why));
        }
        let reasons = self.reasons(a, b, why);
        let root = self.unite(ra, rb, d, reasons);
        let outcome = match self.relax(vec![root], Some(root)) {
            Err(None) => Outcome::Undecided,
            Err(Some(whys)) => Outcome::Conflict(with(whys, why)),
            Ok(()) if !self.zero_holds() => {
                let z = self.root(self.zero);
                let mut whys = self.cycle(z);
                whys.extend(self.links(self.zero));
                Outcome::Conflict(with(whys, why))
            }
            Ok(()) => Outcome::Kept,
        };
        if outcome != Outcome::Kept {
            self.undo(mark);
            return outcome;
        }
        let form = LinExpr::diff(a, b);
        let c = cb - ca;
        if self.general.is_empty() || self.general_hold() {
            self.pending.push((form, Rel::Eq, c, why));
            self.trail.clear();
            return Outcome::Kept;
        }
        match self.simplex_check(&form, &c, why) {
            Outcome::Kept => {
                self.trail.clear();
                Outcome::Kept
            }
            refused => {
                self.undo(mark);
                refused
            }
        }
    }

    pub(crate) fn try_at_least(&mut self, a: usize, ca: &Q, b: usize, cb: &Q, w: &Q, why: Why) -> Outcome {
        if !crate::budget::permit(crate::budget::Step::Structure, self.len()) {
            return Outcome::Undecided;
        }
        let weight = w + cb - ca;
        let (ra, oa) = self.find(a);
        let (rb, ob) = self.find(b);
        if ra == rb {
            return if oa - ob >= weight {
                Outcome::Kept
            } else {
                let mut whys = self.links(a);
                whys.extend(self.links(b));
                Outcome::Conflict(with(whys, why))
            };
        }
        let mark = self.trail.len();
        self.out[b].push(Edge { to: a, w: weight.clone(), why });
        self.trail.push(Undo::Edge(b));
        let outcome = match self.relax(vec![rb], Some(rb)) {
            Err(None) => Outcome::Undecided,
            Err(Some(whys)) => Outcome::Conflict(with(whys, why)),
            Ok(()) if !self.zero_holds() => {
                let z = self.root(self.zero);
                let mut whys = self.cycle(z);
                whys.extend(self.links(self.zero));
                Outcome::Conflict(with(whys, why))
            }
            Ok(()) => Outcome::Kept,
        };
        if outcome != Outcome::Kept {
            self.undo(mark);
            return outcome;
        }
        let form = LinExpr::diff(a, b);
        if self.general.is_empty() || self.general_hold() {
            self.pending.push((form, Rel::Ge, weight, why));
            self.trail.clear();
            return Outcome::Kept;
        }
        let values: Vec<Q> = (0..self.len()).map(|v| self.value(v)).collect();
        let cap = self.pivot_cap;
        let refused = match self.simplex(&|v| values[v].clone()) {
            Ok(sx) => {
                let m = sx.checkpoint();
                let before = sx.pivots();
                sx.set_pivot_limit(Some(before + cap));
                let r = match sx.assert(&form, Rel::Ge, &weight, why) {
                    Ok(()) => sx.check(),
                    Err(e) => Err(e),
                };
                let outcome = match r {
                    Ok(()) => None,
                    Err(Failure::Conflict(whys)) => Some(Outcome::Conflict(whys)),
                    Err(_) => Some(Outcome::Undecided),
                };
                if outcome.is_some() {
                    sx.backtrack(m);
                }
                outcome
            }
            Err(()) => Some(Outcome::Undecided),
        };
        match refused {
            None => {
                self.trail.clear();
                Outcome::Kept
            }
            Some(o) => {
                self.undo(mark);
                o
            }
        }
    }

    fn general_hold(&self) -> bool {
        self.general.iter().all(|(e, lo, hi, _)| {
            let v = e.eval(|x| self.value(x));
            lo.as_ref().is_none_or(|l| v >= *l) && hi.as_ref().is_none_or(|h| v <= *h)
        })
    }

    fn undo(&mut self, mark: usize) {
        while self.trail.len() > mark {
            match self.trail.pop() {
                Some(Undo::Least(r, v)) => self.least[r] = v,
                Some(Undo::Union { child, root, members, wall }) => {
                    self.parent[child] = child;
                    self.off[child] = Q::zero();
                    self.link[child] = Vec::new();
                    self.size[root] -= self.size[child];
                    self.members[root].truncate(members);
                    self.wall[root] = wall;
                }
                Some(Undo::Edge(from)) => {
                    self.out[from].pop();
                }
                None => break,
            }
        }
    }

    pub(crate) fn simplex(&mut self, seed: &dyn Fn(usize) -> Q) -> Result<&mut Simplex<Why>, ()> {
        if let Some(why) = &self.fixed_conflict {
            self.failure = Some(Outcome::Conflict(why.clone()));
            return Err(());
        }
        self.failure = None;
        if !crate::budget::permit(crate::budget::Step::Structure, self.len().saturating_add(self.pending.len())) {
            self.failure = Some(Outcome::Undecided);
            return Err(());
        }
        if self.sx.is_none() {
            let mut sx: Simplex<Why> = Simplex::new();
            for _ in 0..self.len() {
                sx.new_var();
            }
            let mark = sx.checkpoint();
            for v in 0..self.len() {
                let x = seed(v);
                let _ = sx.assert_bounds(v, Some(x.clone()), Some(x), Why::Finish);
            }
            sx.backtrack(mark);
            let _ = sx.assert_bounds(self.zero, Some(Q::zero()), Some(Q::zero()), Why::Body);
            self.sx = Some(sx);
        }
        let pending = std::mem::take(&mut self.pending);
        let cap = self.pivot_cap;
        let sx = self.sx.as_mut().expect("made");
        let mark = sx.checkpoint();
        for (e, rel, c, why) in &pending {
            if let Err(error) = sx.assert(e, *rel, c, *why) {
                sx.backtrack(mark);
                self.pending = pending;
                self.failure = Some(match error {
                    Failure::Conflict(why) => Outcome::Conflict(why),
                    _ => Outcome::Undecided,
                });
                return Err(());
            }
        }
        let before = sx.pivots();
        sx.set_pivot_limit(Some(before + cap));
        let checked = sx.check();
        let (pivots, rows) = (sx.pivots() - before, sx.row_count());
        self.count_pivots(pivots, rows);
        match checked {
            Ok(()) => Ok(self.sx.as_mut().expect("made")),
            Err(error) => {
                self.failure = Some(match error {
                    Failure::Conflict(why) => Outcome::Conflict(why),
                    _ => Outcome::Undecided,
                });
                Err(())
            }
        }
    }

    pub(crate) fn failure(&self) -> Option<&Outcome> {
        self.failure.as_ref()
    }

    fn simplex_check(&mut self, form: &LinExpr, c: &Q, why: Why) -> Outcome {
        let values: Vec<Q> = (0..self.len()).map(|v| self.value(v)).collect();
        let cap = self.pivot_cap;
        let Ok(sx) = self.simplex(&|v| values[v].clone()) else { return Outcome::Undecided };
        let mark = sx.checkpoint();
        let before = sx.pivots();
        sx.set_pivot_limit(Some(before + cap));
        let result = match sx.assert(form, Rel::Eq, c, why) {
            Ok(()) => sx.check(),
            Err(e) => Err(e),
        };
        let pivots = sx.pivots() - before;
        let rows = sx.row_count();
        let outcome = match result {
            Ok(()) => Outcome::Kept,
            Err(Failure::Conflict(whys)) => {
                sx.backtrack(mark);
                Outcome::Conflict(whys)
            }
            Err(_) => {
                sx.backtrack(mark);
                Outcome::Undecided
            }
        };
        self.count_pivots(pivots, rows);
        outcome
    }

    pub(crate) fn count_pivots(&mut self, pivots: u64, rows: usize) {
        self.pivots += pivots;
        self.pivot_rows += pivots * rows as u64;
    }

    pub(crate) fn pivot_cap(&self) -> u64 {
        self.pivot_cap
    }
}

fn scaled(lo: Option<&Q>, hi: Option<&Q>, c: &Q, a: &Q) -> (Option<Q>, Option<Q>) {
    let f = |b: Option<&Q>| b.map(|b| (b - c) / a);
    if a.is_negative() {
        (f(hi), f(lo))
    } else {
        (f(lo), f(hi))
    }
}

fn with(mut whys: Vec<Why>, why: Why) -> Vec<Why> {
    whys.push(why);
    whys.sort();
    whys.dedup();
    whys
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::int;

    fn q(n: i64) -> Q {
        int(n)
    }

    #[test]
    fn rejected_pending_bounds_remain_rejected_on_retry() {
        let mut s = Store::new(vec![q(0); 2], 1, 1000);
        s.pending.push((LinExpr::var(0), Rel::Ge, q(2), Why::Fact(0)));
        s.pending.push((LinExpr::var(0), Rel::Le, q(1), Why::Fact(1)));
        for _ in 0..2 {
            assert!(s.simplex(&|_| q(0)).is_err());
            assert_eq!(s.pending.len(), 2);
        }
    }

    #[test]
    fn interrupted_merge_restores_classes_and_coordinates() {
        let mut s = Store::new(vec![q(0); 4], 3, 1000);
        let z = q(0);
        s.at_least(1, &z, 0, &z, &q(2), Why::Body);
        assert!(s.settle());
        let before: Vec<_> = (0..s.len()).map(|v| (s.find(v), s.value(v))).collect();
        let (outcome, report) = crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(16), None, || {
            s.try_equal(1, &z, 2, &z, Why::Candidate(0))
        });
        assert_eq!(outcome, Outcome::Undecided);
        assert_eq!(report.work, 16);
        assert_eq!((0..s.len()).map(|v| (s.find(v), s.value(v))).collect::<Vec<_>>(), before);
        assert_eq!(s.try_equal(1, &z, 2, &z, Why::Candidate(0)), Outcome::Kept);
    }

    #[test]
    fn a_positive_cycle_is_a_conflict() {
        let mut s = Store::new(vec![q(-10), q(-10), q(0)], 2, 1000);
        s.equal(0, &q(0), 2, &q(0), Why::Fact(1));
        s.equal(1, &q(0), 2, &q(3), Why::Fact(3));
        assert!(s.settle());
        assert_eq!(s.value(1), q(3));
        match s.try_equal(0, &q(0), 1, &q(0), Why::Fact(2)) {
            Outcome::Conflict(whys) => {
                assert!(whys.contains(&Why::Fact(1)) && whys.contains(&Why::Fact(3)), "{whys:?}")
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(s.value(0), q(0));
        assert_eq!(s.value(1), q(3));
    }

    #[test]
    fn merged_coordinate_inequalities_detect_positive_self_cycles() {
        for offset in [-3, 0, 5] {
            for extra in [-1, 0, 1] {
                for equality_first in [false, true] {
                    let mut s = Store::new(vec![q(0); 3], 2, 1000);
                    let equal = |s: &mut Store| s.equal(1, &q(0), 0, &q(offset), Why::Fact(0));
                    let bound = |s: &mut Store| s.at_least(1, &q(0), 0, &q(0), &q(offset + extra), Why::Fact(1));
                    if equality_first {
                        equal(&mut s);
                        bound(&mut s);
                    } else {
                        bound(&mut s);
                        equal(&mut s);
                    }
                    if extra > 0 {
                        assert!(!s.settle());
                        let Some(Outcome::Conflict(why)) = s.failure() else {
                            panic!("a positive self-cycle is a proof, not an exhausted relaxation limit");
                        };
                        assert!(why.contains(&Why::Fact(0)) && why.contains(&Why::Fact(1)));
                        assert_eq!(s.relaxations, 1);
                    } else {
                        assert!(s.settle());
                        assert_eq!(s.value(1) - s.value(0), q(offset));
                    }
                }
            }
        }
    }

    #[test]
    fn longest_path_and_a_merge() {
        let mut s = Store::new(vec![q(0); 5], 4, 1000);
        let z = q(0);
        s.at_least(1, &z, 0, &z, &q(3), Why::Apart(0, 0));
        s.at_least(3, &z, 1, &z, &q(2), Why::Apart(0, 0));
        s.at_least(2, &z, 0, &z, &q(1), Why::Apart(0, 0));
        s.at_least(3, &z, 2, &z, &q(6), Why::Apart(0, 0));
        assert!(s.settle());
        assert_eq!((s.value(0), s.value(1), s.value(2), s.value(3)), (q(0), q(3), q(1), q(7)));
        assert_eq!(s.try_equal(1, &z, 2, &z, Why::Candidate(0)), Outcome::Kept);
        assert_eq!((s.value(2), s.value(3)), (q(3), q(9)));
        assert!(matches!(s.try_equal(0, &z, 3, &z, Why::Candidate(1)), Outcome::Conflict(_)));
        assert_eq!(s.value(3), q(9));
    }

    #[test]
    fn a_distribute_is_decided_by_the_simplex() {
        let mut s = Store::new(vec![q(0); 4], 3, 1000);
        let z = q(0);
        s.at_least(1, &z, 0, &z, &q(2), Why::Apart(0, 0));
        s.at_least(2, &z, 1, &z, &q(2), Why::Apart(0, 0));
        let e = LinExpr::from_terms([(0, q(1)), (1, q(-2)), (2, q(1))], q(0));
        s.bound(&e, Some(&q(0)), Some(&q(0)), Why::Fact(0));
        assert!(s.settle());
        let drawn = [q(0), q(4), q(8), q(0)];
        assert!(s.simplex(&|v| drawn[v].clone()).is_ok());
        assert_eq!(s.try_equal(2, &z, 0, &q(6), Why::Candidate(0)), Outcome::Kept);
        assert!(matches!(s.try_equal(1, &z, 0, &q(2), Why::Candidate(1)), Outcome::Conflict(_)));
    }
}
#[test]
fn contradictory_fixed_constraints_remain_conflicts() {
    use crate::rational::int as q;
    for case in 0..4 {
        let mut s = Store::new(vec![q(0); 2], 1, 1000);
        match case {
            0 => {
                s.equal(0, &q(0), 1, &q(0), Why::Fact(0));
                s.equal(0, &q(0), 1, &q(1), Why::Wire);
            }
            1 => s.at_least(0, &q(0), 0, &q(0), &q(1), Why::Wire),
            2 => s.bound(&LinExpr::from_terms([], q(2)), None, Some(&q(1)), Why::Fact(1)),
            _ => s.bound(&LinExpr::from_terms([], q(2)), Some(&q(3)), None, Why::Fact(1)),
        }
        for _ in 0..2 {
            assert!(!s.settle(), "case {case}");
            assert!(!s.unsettled().is_empty());
            assert!(matches!(s.failure(), Some(Outcome::Conflict(_))));
            assert!(s.simplex(&|_| q(0)).is_err());
            assert!(matches!(s.failure(), Some(Outcome::Conflict(_))));
        }
    }
    let mut s = Store::new(vec![q(0); 2], 1, 1000);
    s.equal(0, &q(0), 0, &q(0), Why::Wire);
    s.at_least(0, &q(1), 0, &q(0), &q(1), Why::Wire);
    s.bound(&LinExpr::from_terms([], q(2)), Some(&q(2)), Some(&q(2)), Why::Fact(0));
    assert!(s.settle());
    assert!(s.simplex(&|_| q(0)).is_ok());
}
