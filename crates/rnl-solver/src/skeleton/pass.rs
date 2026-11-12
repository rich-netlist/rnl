//! Compact one axis while retaining the other axis. Exact constraints preserve the
//! drawing's topology; optional pin/line merges precede lexicographic optimization.
//! A failed or interrupted pass retains verified coordinates, never unchecked graph values.

use std::collections::{BTreeMap, BTreeSet};

use num_traits::{One, Zero};

use super::cross::Mid;
use super::read::{Feature, Key, Kind, Made, Skeleton, Term};
use super::store::{Outcome, Store, Why};
use crate::rational::{from_f64, int, Q};
use crate::simplex::{LinExpr, Rel, Simplex};
use crate::{SolverOptions, Work};

const SYMBOL_GAP: i64 = 2;
const WIRE_GAP: i64 = 1;
const GROUP_GAP: i64 = 1;
const MOST_MERGES: usize = 20_000;
const MERGE_WORK: u64 = 30_000_000;
pub(super) const PIVOT_CAP: u64 = 200_000;

#[derive(Clone, Debug, Default)]
pub(crate) struct Report {
    pub interrupted: bool,
    pub axis: usize,
    pub lines: (usize, usize),
    pub edges: usize,
    pub general: usize,
    pub bends: (usize, usize),
    pub levels: (usize, usize),
    pub rails: (usize, usize),
    pub merges: [(usize, usize); 4],
    pub with_facts: usize,
    pub undecided: usize,
    pub facts: BTreeMap<usize, usize>,
    pub collapsed: BTreeSet<usize>,
    pub failed: Option<String>,
    pub failed_facts: Vec<usize>,
    pub rounded: bool,
    pub grid_conflict: bool,
    pub merges_left: usize,
    pub conflicts: Vec<(usize, usize)>,
    pub crosses: (usize, usize),
    pub cross_lines: (usize, usize),
    pub again: usize,
    pub new: usize,
    pub relaxations: u64,
    pub pivot_rows: u64,
}

impl Report {
    pub(super) fn stop(&mut self) {
        self.interrupted = true;
        self.failed = Some("the shared solve budget stopped this pass".into());
        self.failed_facts.clear();
        self.grid_conflict = false;
    }
}

struct Wish {
    low: Term,
    high: Term,
    gap: Q,
    pair: (usize, usize),
}

#[derive(Clone, Debug)]
struct Poi {
    term: Term,
    kind: u8,
    trees: Vec<usize>,
    objects: BTreeSet<usize>,
    wire: bool,
    middle: bool,
}

#[derive(Clone, Debug)]
struct Line {
    term: Term,
    at: Q,
    pin_or_bend: bool,
    trees: BTreeSet<usize>,
    usages: BTreeSet<usize>,
    groups: BTreeSet<usize>,
    first: usize,
    wire: bool,
    middle: bool,
}

pub(crate) fn pass(
    sk: &mut Skeleton,
    axis: usize,
    options: &SolverOptions,
    work: &mut Work,
    mut merged: Option<&mut Vec<(Term, Term)>>,
) -> Report {
    let mut report = Report { axis, ..Report::default() };
    if !crate::budget::permit(crate::budget::Step::Structure, sk.vars) {
        report.stop();
        return report;
    }
    let read = sk.value[axis].iter().min().cloned().unwrap_or_default();
    let above = sk.made[axis]
        .iter()
        .filter_map(|m| match m {
            Made::Bound(e, lo, hi, Why::Fact(_)) if e.terms().len() == 1 => {
                let (v, a) = &e.terms()[0];
                let written = [lo, hi].into_iter().flatten().map(|b| (b - e.constant_part()) / a).min()?;
                Some(&sk.value[axis][*v] - &written)
            }
            _ => None,
        })
        .max()
        .filter(|d| *d > Q::zero());
    let low = above.map_or_else(|| read.clone(), |d| &read - &d);
    let mut walls = vec![low; sk.vars];
    walls[sk.zero] = Q::zero();
    let mut store = Store::new(walls, sk.zero, PIVOT_CAP);
    if let Some(g) = options.grid.and_then(from_f64).filter(|g| sk.strict && sk.on_grid && *g > Q::zero()) {
        let mut off: BTreeSet<usize> = BTreeSet::new();
        for c in sk.crosses.iter().filter(|c| c.axis == axis) {
            off.insert(c.centre);
            off.extend(c.low.iter().chain(&c.high).map(|&v| sk.vertices[v].term[axis].var));
        }
        for m in &sk.made[axis] {
            if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                report.stop();
                return report;
            }
            if let Made::Bound(e, _, _, _) = m {
                if e.terms().iter().any(|(_, a)| !a.is_integer()) {
                    off.extend(e.terms().iter().map(|(v, _)| *v));
                }
            }
        }
        store.round_to_grid(g, off);
    }
    for m in &sk.made[axis] {
        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
            report.stop();
            return report;
        }
        match m {
            Made::Equal(a, ca, b, cb, why) => store.equal(*a, ca, *b, cb, *why),
            Made::AtLeast(a, ca, b, cb, w, why) => store.at_least(*a, ca, *b, cb, w, *why),
            Made::Bound(e, lo, hi, why) => store.bound(e, lo.as_ref(), hi.as_ref(), *why),
        }
    }
    for c in sk.crosses.iter().filter(|c| c.axis == axis) {
        let half = cross_height(sk, c) / int(2);
        for k in 0..2 {
            let (lo, hi) = (&sk.vertices[c.low[k]].term[axis], &sk.vertices[c.high[k]].term[axis]);
            store.equal(lo.var, &(&lo.off + &half), c.centre, &Q::zero(), Why::Wire);
            store.equal(hi.var, &(&hi.off - &half), c.centre, &Q::zero(), Why::Wire);
        }
    }
    let wishes = apart(sk, axis, &mut store);
    if crate::budget::stopped() {
        report.stop();
        return report;
    }
    report.rounded = store.any_rounded();
    (report.general, report.edges) = store.counts();
    if !store.settle() {
        if crate::budget::stopped() {
            report.stop();
            return report;
        }
        if matches!(store.failure(), Some(Outcome::Undecided)) {
            report.failed = Some("the graph relaxation did not finish within its budget".into());
            report.grid_conflict = store.any_rounded();
            return report;
        }
        let unsettled = store.unsettled();
        report.grid_conflict = store.grid_in_unsettled();
        report.failed_facts = facts_of(&unsettled);
        report.failed = Some("the drawing as read has no solution of its own constraints".to_string());
        return report;
    }
    if store.has_general() {
        let drawn = sk.value[axis].clone();
        if store.simplex(&|v| drawn[v].clone()).is_err() {
            let conflict = match store.failure() {
                Some(Outcome::Conflict(why)) => why.clone(),
                _ => Vec::new(),
            };
            report.failed_facts = facts_of(&conflict);
            report.grid_conflict = store.grid_in(&conflict);
            report.interrupted = crate::budget::stopped();
            report.failed = Some(
                if matches!(store.failure(), Some(Outcome::Conflict(_))) {
                    "the simplex found no solution of the drawing as read"
                } else {
                    "the constraint check did not finish within its budget"
                }
                .into(),
            );
            return report;
        }
    }
    for c in sk.crosses.iter().filter(|c| c.axis != axis) {
        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
            report.stop();
            return report;
        }
        let t = |v: usize| &sk.vertices[v].term[axis];
        for (a, b) in [(t(c.low[0]), t(c.high[1])), (t(c.low[1]), t(c.high[0]))] {
            let outcome = store.try_equal(a.var, &a.off, b.var, &b.off, Why::Wire);
            if crate::budget::stopped() {
                report.stop();
                return report;
            }
            if outcome != Outcome::Kept {
                let whys = if let Outcome::Conflict(whys) = outcome { whys } else { Vec::new() };
                report.grid_conflict = store.grid_in(&whys);
                report.failed = Some("the lines of an X cannot be the same two on either side".to_string());
                return report;
            }
        }
    }
    if let Some(kept) = merged.as_deref_mut() {
        let mut again = Vec::new();
        for (a, b) in kept.drain(..) {
            if store.try_equal(a.var, &a.off, b.var, &b.off, Why::Body) == Outcome::Kept {
                again.push((a, b));
            }
        }
        report.again = again.len();
        *kept = again;
    }
    for w in wishes {
        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
            report.stop();
            return report;
        }
        let why = Why::Apart(w.pair.0, w.pair.1);
        let gap = store.whole_gap(w.high.var, &w.high.off, w.low.var, &w.low.off, &w.gap);
        if gap != w.gap {
            store.mark_rounded(why, &gap - &w.gap);
        }
        let outcome = store.try_at_least(w.high.var, &w.high.off, w.low.var, &w.low.off, &gap, why);
        if outcome != Outcome::Kept {
            report.conflicts.push(w.pair);
        }
    }
    for c in sk.crosses.iter().filter(|c| c.axis != axis) {
        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
            report.stop();
            return report;
        }
        let (upper, lower) = (&sk.vertices[c.low[0]].term[axis], &sk.vertices[c.low[1]].term[axis]);
        let wide = cross_width(sk, c);
        let _ = store.try_at_least(lower.var, &(&lower.off + &wide), upper.var, &upper.off, &Q::zero(), Why::Wire);
    }
    for c in sk.crosses.iter().filter(|c| c.axis == axis) {
        let _ = match c.pair {
            Some(Mid::Axis(v)) => store.try_equal(c.centre, &Q::zero(), v, &Q::zero(), Why::Wire),
            Some(Mid::Between(a, b)) => {
                let ((ra, oa), (rb, ob)) = (store.find(a), store.find(b));
                if ra != rb {
                    continue;
                }
                let half = (ob - oa) / int(2);
                store.try_equal(c.centre, &Q::zero(), a, &half, Why::Wire)
            }
            None => continue,
        };
    }
    let mut number = 0usize;
    let mut decide = |store: &mut Store, report: &mut Report, a: &Term, b: &Term| -> bool {
        if merged.is_some() {
            let ((ra, oa), (rb, ob)) = (store.find(a.var), store.find(b.var));
            if ra == rb && oa + &a.off == ob + &b.off {
                return true;
            }
        }
        number += 1;
        let outcome = store.try_equal(a.var, &a.off, b.var, &b.off, Why::Candidate(number));
        match outcome {
            Outcome::Kept => {
                if let Some(kept) = merged.as_deref_mut() {
                    kept.push((a.clone(), b.clone()));
                    report.new += 1;
                }
                true
            }
            Outcome::Conflict(whys) => {
                let facts: Vec<usize> =
                    whys.iter().filter_map(|w| if let Why::Fact(f) = w { Some(*f) } else { None }).collect();
                if !facts.is_empty() {
                    report.with_facts += 1;
                }
                for f in facts {
                    *report.facts.entry(f).or_default() += 1;
                }
                false
            }
            Outcome::Undecided => {
                report.undecided += 1;
                false
            }
        }
    };
    if axis == 1 {
        let term = |p: (usize, usize)| sk.pins.iter().find(|q| q.0 == p).map(|q| q.1[1].clone());
        for &(p, q) in &sk.levels {
            if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                report.stop();
                return report;
            }
            let (Some(a), Some(b)) = (term(p), term(q)) else { continue };
            report.levels.0 += 1;
            if decide(&mut store, &mut report, &a, &b) {
                report.levels.1 += 1;
            }
        }
        for &(p, q) in &sk.rails {
            if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                report.stop();
                return report;
            }
            let (Some(a), Some(b)) = (term(p), term(q)) else { continue };
            let ((ra, oa), (rb, ob)) = (store.find(a.var), store.find(b.var));
            if ra == rb && oa + &a.off == ob + &b.off {
                continue;
            }
            report.rails.0 += 1;
            if decide(&mut store, &mut report, &a, &b) {
                report.rails.1 += 1;
            }
        }
    }
    let mut shorten: Vec<(std::cmp::Reverse<u8>, Q, usize)> = Vec::new();
    for (i, seg) in sk.segs.iter().enumerate() {
        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
            report.stop();
            return report;
        }
        if sk.jog_of(i).is_some() {
            continue;
        }
        let saves = if sk.strict { sk.saves(i) } else { u8::from(sk.z_middle(i) || sk.collapses(i).is_some()) * 2 };
        if seg.along != Some(axis) || saves == 0 {
            continue;
        }
        let length = (&sk.vertices[seg.a].at[axis] - &sk.vertices[seg.b].at[axis]).abs();
        shorten.push((std::cmp::Reverse(if sk.strict { saves } else { 0 }), length, i));
    }
    shorten.sort();
    for (_, _, i) in shorten {
        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
            report.stop();
            return report;
        }
        let seg = &sk.segs[i];
        let (a, b) = (sk.vertices[seg.a].term[axis].clone(), sk.vertices[seg.b].term[axis].clone());
        report.bends.0 += 1;
        if decide(&mut store, &mut report, &a, &b) {
            report.bends.1 += 1;
            report.collapsed.insert(i);
        }
    }
    let pois = pois(sk, axis);
    let lines = lines(sk, axis, &store, &pois);
    if crate::budget::stopped() {
        report.stop();
        return report;
    }
    report.lines.0 = lines.len();
    let mut pairs: Vec<(u8, Q, usize, usize, usize, usize)> = Vec::new();
    for i in 0..lines.len() {
        for j in i + 1..lines.len() {
            if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                report.stop();
                return report;
            }
            let (a, b) = (&lines[i], &lines[j]);
            if !(a.pin_or_bend || b.pin_or_bend) || store.root(a.term.var) == store.root(b.term.var) {
                continue;
            }
            if (a.middle && !b.wire) || (b.middle && !a.wire) {
                continue;
            }
            let scope = if !a.trees.is_disjoint(&b.trees) {
                0
            } else if !a.usages.is_disjoint(&b.usages) {
                1
            } else if !a.groups.is_disjoint(&b.groups) {
                2
            } else {
                3
            };
            pairs.push((scope, (&a.at - &b.at).abs(), a.first.min(b.first), a.first.max(b.first), i, j));
        }
    }
    pairs.sort();
    if pairs.len() > MOST_MERGES {
        report.merges_left = pairs.len() - MOST_MERGES;
        pairs.truncate(MOST_MERGES);
    }
    let spent = store.relaxations + store.pivots;
    let total = pairs.len();
    for (tried, (scope, _, _, _, i, j)) in pairs.into_iter().enumerate() {
        if store.relaxations + store.pivots - spent > MERGE_WORK {
            report.merges_left += total - tried;
            break;
        }
        let (a, b) = (&lines[i].term, &lines[j].term);
        let (ra, oa) = store.find(a.var);
        let (rb, ob) = store.find(b.var);
        if ra == rb {
            continue;
        }
        let _ = (oa, ob);
        report.merges[scope as usize].0 += 1;
        if decide(&mut store, &mut report, a, b) {
            report.merges[scope as usize].1 += 1;
        }
    }
    report.lines.1 = {
        let mut keys: BTreeSet<(usize, Q)> = BTreeSet::new();
        for l in &lines {
            let (r, o) = store.find(l.term.var);
            keys.insert((r, o + &l.term.off));
        }
        keys.len()
    };
    if sk.strict {
        let symbols: Vec<(usize, usize)> = sk
            .features
            .iter()
            .filter_map(|f| if let Kind::Symbol(k) = f.kind { sk.origin[k].map(|v| (k, v)) } else { None })
            .collect();
        let count = |store: &Store| symbols.iter().map(|&(_, v)| store.find(v)).collect::<BTreeSet<_>>().len();
        report.cross_lines.0 = count(&store);
        let mut pairs: Vec<(u8, Q, usize, usize)> = Vec::new();
        for (i, &(a, va)) in symbols.iter().enumerate() {
            for &(b, vb) in &symbols[i + 1..] {
                if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                    report.stop();
                    return report;
                }
                let shares = |m: &BTreeMap<usize, BTreeSet<usize>>| {
                    m.get(&a).zip(m.get(&b)).is_some_and(|(x, y)| !x.is_disjoint(y))
                };
                let scope = if shares(&sk.usages_of) {
                    0
                } else if shares(&sk.groups_of) {
                    1
                } else {
                    2
                };
                let d = (&sk.value[axis][va] - &sk.value[axis][vb]).abs();
                pairs.push((scope, d, a.min(b), a.max(b)));
            }
        }
        pairs.sort();
        pairs.truncate(MOST_MERGES);
        let spent = store.relaxations + store.pivots;
        for (_scope, _d, a, b) in pairs {
            if store.relaxations + store.pivots - spent > MERGE_WORK {
                break;
            }
            let (Some(va), Some(vb)) = (sk.origin[a], sk.origin[b]) else { continue };
            if store.root(va) == store.root(vb) {
                continue;
            }
            report.crosses.0 += 1;
            if decide(&mut store, &mut report, &Term::of(va, Q::zero()), &Term::of(vb, Q::zero())) {
                report.crosses.1 += 1;
            }
        }
        report.cross_lines.1 = count(&store);
    }
    let values = finish(sk, axis, &mut store, options, &mut report);
    work.pivots += store.pivots;
    report.relaxations = store.relaxations;
    report.pivot_rows = store.pivot_rows;
    report.rounded = store.any_rounded();
    report.grid_conflict = report.failed.is_some() && report.rounded;
    if crate::budget::stopped() {
        report.stop();
    }
    if let Some(values) = values {
        sk.value[axis] = values;
    }
    report
}

pub(super) fn spacing(sk: &Skeleton, a: &Feature, b: &Feature) -> Option<i64> {
    use Kind::*;
    if sk.strict {
        let tree = |g: usize| sk.segs[g].tree;
        match (a.kind, b.kind) {
            (Segment(x), Segment(y)) if tree(x) == tree(y) => return None,
            (Segment(x), Segment(y)) if sk.crosses.iter().any(|c| c.owns(x) && c.owns(y)) => return None,
            (Segment(g), Pin(k, p)) | (Pin(k, p), Segment(g)) if sk.pin_in_tree((k, p), tree(g)) => return None,
            _ => {}
        }
    }
    match (a.kind, b.kind) {
        (Symbol(_), Symbol(_)) => Some(SYMBOL_GAP),
        (Symbol(k), Segment(g)) | (Segment(g), Symbol(k)) => (!sk.at_object(g, k)).then_some(WIRE_GAP),
        (Symbol(k), Text(o)) | (Text(o), Symbol(k)) => (k != o).then_some(0),
        (Symbol(k), Group(i)) | (Group(i), Symbol(k)) => (!sk.groups[i].members.contains(&k)).then_some(GROUP_GAP),
        (Segment(x), Segment(y)) => (x != y && !sk.near[x].contains(&y)).then_some(WIRE_GAP),
        (Segment(_), Text(_)) | (Text(_), Segment(_)) => Some(0),
        (Segment(g), Pin(k, p)) | (Pin(k, p), Segment(g)) => (!sk.at_pin(g, (k, p))).then_some(WIRE_GAP),
        (Text(x), Text(y)) => (x != y).then_some(0),
        (Group(x), Group(y)) => {
            let (m, n) = (&sk.groups[x].members, &sk.groups[y].members);
            (!m.is_subset(n) && !n.is_subset(m)).then_some(GROUP_GAP)
        }
        _ => None,
    }
}

pub(crate) fn unkept(sk: &Skeleton) -> Option<Vec<(usize, usize)>> {
    let n = sk.features.len();
    if !crate::budget::permit(crate::budget::Step::Structure, n) {
        return None;
    }
    let at = |axis: usize, t: &Term| -> Q { sk.at(axis, t) };
    let lo: Vec<[Q; 2]> = sk.features.iter().map(|f| [at(0, &f.lo[0]), at(1, &f.lo[1])]).collect();
    let hi: Vec<[Q; 2]> = sk.features.iter().map(|f| [at(0, &f.hi[0]), at(1, &f.hi[1])]).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| lo[i][0].cmp(&lo[j][0]).then(i.cmp(&j)));
    let widest = int(SYMBOL_GAP.max(WIRE_GAP).max(GROUP_GAP));
    let mut out = Vec::new();
    for (k, &i) in order.iter().enumerate() {
        for &j in &order[k + 1..] {
            if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                return None;
            }
            if lo[j][0] >= &hi[i][0] + &widest {
                break;
            }
            let (fi, fj) = (&sk.features[i], &sk.features[j]);
            let routed = |f: &Feature| matches!(f.kind, Kind::Segment(g) if sk.segs[g].routed);
            let written = |f: &Feature| matches!(f.kind, Kind::Segment(g) if !sk.segs[g].routed);
            if !(routed(fi) || routed(fj)) || written(fi) || written(fj) {
                continue;
            }
            let Some(gap) = spacing(sk, fi, fj) else { continue };
            if gap == 0 {
                continue;
            }
            let gap = int(gap);
            let apart = |axis: usize| (&lo[j][axis] - &hi[i][axis]).max(&lo[i][axis] - &hi[j][axis]) >= gap;
            if apart(0) || apart(1) {
                continue;
            }
            if let (Kind::Segment(_), Kind::Segment(_)) = (fi.kind, fj.kind) {
                let flat = |f: usize, axis: usize| lo[f][axis] == hi[f][axis];
                let (h, v) = if flat(i, 1) && flat(j, 0) {
                    (i, j)
                } else if flat(j, 1) && flat(i, 0) {
                    (j, i)
                } else {
                    (usize::MAX, usize::MAX)
                };
                if h != usize::MAX {
                    let (x, y) = (&lo[v][0], &lo[h][1]);
                    let clear =
                        x - &lo[h][0] >= gap && &hi[h][0] - x >= gap && y - &lo[v][1] >= gap && &hi[v][1] - y >= gap;
                    if clear {
                        continue;
                    }
                }
            }
            out.push((i.min(j), i.max(j)));
        }
    }
    out.sort_unstable();
    Some(out)
}

fn apart(sk: &Skeleton, axis: usize, store: &mut Store) -> Vec<Wish> {
    let mut wishes = Vec::new();
    let across = 1 - axis;
    let n = sk.features.len();
    let lo_a: Vec<Q> = sk.features.iter().map(|f| sk.at(axis, &f.lo[axis])).collect();
    let hi_a: Vec<Q> = sk.features.iter().map(|f| sk.at(axis, &f.hi[axis])).collect();
    let lo_c: Vec<Q> = sk.features.iter().map(|f| sk.at(across, &f.lo[across])).collect();
    let hi_c: Vec<Q> = sk.features.iter().map(|f| sk.at(across, &f.hi[across])).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| lo_c[i].cmp(&lo_c[j]).then(i.cmp(&j)));
    let widest = int(SYMBOL_GAP.max(WIRE_GAP).max(GROUP_GAP));
    for (at, &i) in order.iter().enumerate() {
        for &j in &order[at + 1..] {
            if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                return wishes;
            }
            if lo_c[j] >= &hi_c[i] + &widest {
                break;
            }
            let (fi, fj) = (&sk.features[i], &sk.features[j]);
            let Some(gap) = spacing(sk, fi, fj) else { continue };
            let gap = int(gap);
            if !(lo_c[i] < &hi_c[j] + &gap && lo_c[j] < &hi_c[i] + &gap) {
                continue;
            }
            let ends =
                |f: &Feature, k: usize| [(f.lo[axis].clone(), lo_a[k].clone()), (f.hi[axis].clone(), hi_a[k].clone())];
            let routed = |f: &Feature| matches!(f.kind, Kind::Segment(g) if sk.segs[g].routed);
            let written = |f: &Feature| matches!(f.kind, Kind::Segment(g) if !sk.segs[g].routed);
            let strict = sk.strict && (routed(fi) || routed(fj)) && !(written(fi) || written(fj));
            let mut keep = |store: &mut Store, (ta, va): &(Term, Q), (tb, vb): &(Term, Q)| {
                let d = vb - va;
                let whole = store.whole_gap(tb.var, &tb.off, ta.var, &ta.off, &gap);
                let w = if d < gap {
                    d.clone()
                } else if d >= whole {
                    whole
                } else {
                    gap.clone()
                };
                if d >= gap && w != gap {
                    store.mark_rounded(Why::Apart(i, j), &w - &gap);
                }
                store.at_least(tb.var, &tb.off, ta.var, &ta.off, &w, Why::Apart(i, j));
                if strict && d < gap {
                    wishes.push(Wish { low: ta.clone(), high: tb.clone(), gap: gap.clone(), pair: (i, j) });
                }
            };
            if hi_a[i] <= lo_a[j] {
                keep(store, &ends(fi, i)[1], &ends(fj, j)[0]);
            } else if hi_a[j] <= lo_a[i] {
                keep(store, &ends(fj, j)[1], &ends(fi, i)[0]);
            } else {
                for ea in ends(fi, i) {
                    for eb in ends(fj, j) {
                        if ea.1 < eb.1 {
                            keep(store, &ea, &eb);
                        } else if eb.1 < ea.1 {
                            keep(store, &eb, &ea);
                        }
                    }
                }
            }
        }
    }
    wishes
}

pub(crate) fn cross_height(sk: &Skeleton, c: &super::cross::Cross) -> Q {
    let across = 1 - c.axis;
    sk.at(across, &sk.vertices[c.low[0]].term[across]) - sk.at(across, &sk.vertices[c.low[1]].term[across])
}

pub(crate) fn cross_width(sk: &Skeleton, c: &super::cross::Cross) -> Q {
    sk.at(c.axis, &sk.vertices[c.high[0]].term[c.axis]) - sk.at(c.axis, &sk.vertices[c.low[0]].term[c.axis])
}

fn pois(sk: &Skeleton, axis: usize) -> Vec<Poi> {
    let mut out = Vec::new();
    for c in sk.crosses.iter().filter(|c| c.axis == axis) {
        if !crate::budget::permit(crate::budget::Step::Structure, c.objects.len()) {
            return out;
        }
        let trees = c.jogs.iter().map(|&g| sk.segs[g].tree).collect();
        let objects = c.objects.clone();
        out.push(Poi { term: Term::of(c.centre, Q::zero()), kind: 1, trees, objects, wire: true, middle: true });
    }
    let across = |spot: &super::read::Spot| spot.segs.iter().any(|&g| sk.segs[g].along == Some(1 - axis));
    for spot in &sk.spots {
        if !crate::budget::permit(crate::budget::Step::Structure, spot.vertices.len() + spot.segs.len()) {
            return out;
        }
        let Some(&v) = spot.vertices.first() else { continue };
        if spot.vertices.iter().any(|&x| sk.corner(x)) {
            continue;
        }
        let vertex = &sk.vertices[v];
        let trees: BTreeSet<_> = spot.vertices.iter().flat_map(|&v| sk.vertices[v].trees.iter().copied()).collect();
        let tree_objects = trees.iter().flat_map(|t| sk.tree_objects.get(t).into_iter().flatten().copied()).collect();
        let trees: Vec<_> = trees.into_iter().collect();
        if !spot.pins.is_empty() {
            for &(k, pin) in &spot.pins {
                if !crate::budget::permit(crate::budget::Step::Structure, sk.vertices.len()) {
                    return out;
                }
                let term = sk
                    .vertices
                    .iter()
                    .find(|x| x.key == Key::Pin(k, pin))
                    .map_or_else(|| vertex.term[axis].clone(), |x| x.term[axis].clone());
                let way = sk.exits.get(&(k, pin)).is_none_or(|e| if axis == 0 { e.0 == 0 } else { e.1 == 0 });
                let wire = across(spot) || way;
                let objects = [k].into_iter().collect();
                out.push(Poi { term, kind: 0, trees: trees.clone(), objects, wire, middle: false });
            }
        } else if spot.segs.len() >= 3 {
            let (term, wire) = (vertex.term[axis].clone(), across(spot));
            out.push(Poi { term, kind: 2, trees, objects: tree_objects, wire, middle: false });
        } else if spot.segs.len() == 2 {
            let (a, b) = (sk.segs[spot.segs[0]].along, sk.segs[spot.segs[1]].along);
            if a.is_some() && b.is_some() && a != b {
                let term = vertex.term[axis].clone();
                out.push(Poi { term, kind: 1, trees, objects: tree_objects, wire: true, middle: false });
            }
        }
    }
    out
}

fn lines(sk: &Skeleton, axis: usize, store: &Store, pois: &[Poi]) -> Vec<Line> {
    let mut by: BTreeMap<(usize, Q), Line> = BTreeMap::new();
    for (i, p) in pois.iter().enumerate() {
        if !crate::budget::permit(crate::budget::Step::Structure, p.objects.len() + p.trees.len()) {
            return Vec::new();
        }
        let (r, o) = store.find(p.term.var);
        let key = (r, o + &p.term.off);
        let line = by.entry(key).or_insert_with(|| Line {
            term: p.term.clone(),
            at: sk.at(axis, &p.term),
            pin_or_bend: false,
            trees: BTreeSet::new(),
            usages: BTreeSet::new(),
            groups: BTreeSet::new(),
            first: i,
            wire: false,
            middle: false,
        });
        line.pin_or_bend |= p.kind < 2;
        line.wire |= p.wire;
        line.middle |= p.middle;
        line.trees.extend(p.trees.iter().copied());
        for k in &p.objects {
            line.usages.extend(sk.usages_of.get(k).into_iter().flatten());
            line.groups.extend(sk.groups_of.get(k).into_iter().flatten());
        }
    }
    let mut out: Vec<Line> = by.into_values().collect();
    out.sort_by_key(|l| l.first);
    out
}

fn facts_of(whys: &[Why]) -> Vec<usize> {
    let facts: BTreeSet<usize> =
        whys.iter().filter_map(|w| if let Why::Fact(f) = w { Some(*f) } else { None }).collect();
    facts.into_iter().collect()
}

fn seeded(sx: &mut Simplex<Why>, value: &Q) -> usize {
    let v = sx.new_var();
    let mark = sx.checkpoint();
    let _ = sx.assert_bounds(v, Some(value.clone()), Some(value.clone()), Why::Finish);
    sx.backtrack(mark);
    v
}

pub(super) fn finish(
    sk: &Skeleton,
    axis: usize,
    store: &mut Store,
    options: &SolverOptions,
    report: &mut Report,
) -> Option<Vec<Q>> {
    let least: Vec<Q> = (0..sk.vars).map(|v| store.value(v)).collect();
    let cap = store.pivot_cap();
    let mut pivots = 0;
    let Ok(sx) = store.simplex(&|v| least[v].clone()) else {
        report.interrupted = crate::budget::stopped();
        report.failed = Some("the simplex could not complete a solution of the constraints kept".to_string());
        return None;
    };
    let fallback: Vec<Q> = (0..sk.vars).map(|v| sx.value(v).clone()).collect();
    let mark = sx.checkpoint();
    let mut length = LinExpr::default();
    for seg in sk.segs.iter().filter(|g| g.along == Some(axis)) {
        let (a, b) = (&sk.vertices[seg.a], &sk.vertices[seg.b]);
        let (lo, hi) =
            if a.at[axis] <= b.at[axis] { (&a.term[axis], &b.term[axis]) } else { (&b.term[axis], &a.term[axis]) };
        length.add_term(hi.var, &Q::one());
        length.add_term(lo.var, &-Q::one());
        length.add_constant(&(&hi.off - &lo.off));
    }
    let drawn: Vec<&Feature> =
        sk.features.iter().filter(|f| matches!(f.kind, Kind::Symbol(_) | Kind::Text(_) | Kind::Segment(_))).collect();
    let value = |sx: &Simplex<Why>, t: &Term| sx.value(t.var) + &t.off;
    let low = drawn.iter().map(|f| value(sx, &f.lo[axis])).min().unwrap_or_default();
    let high = drawn.iter().map(|f| value(sx, &f.hi[axis])).max().unwrap_or_default();
    let (vl, vh) = (seeded(sx, &low), seeded(sx, &high));
    for f in &drawn {
        let (l, h) = (&f.lo[axis], &f.hi[axis]);
        let _ = sx.assert(
            &LinExpr::from_terms([(l.var, Q::one()), (vl, -Q::one())], l.off.clone()),
            Rel::Ge,
            &Q::zero(),
            Why::Finish,
        );
        let _ = sx.assert(
            &LinExpr::from_terms([(vh, Q::one()), (h.var, -Q::one())], -h.off.clone()),
            Rel::Ge,
            &Q::zero(),
            Why::Finish,
        );
    }
    let extent = LinExpr::from_terms([(vh, Q::one()), (vl, -Q::one())], Q::zero());
    let mut moved = LinExpr::default();
    let symbols: Vec<usize> = sk
        .features
        .iter()
        .filter_map(|f| if let Kind::Symbol(k) = f.kind { sk.origin[k] } else { None })
        .collect::<BTreeSet<usize>>()
        .into_iter()
        .collect();
    for &v in &symbols {
        let was = sk.value[axis][v].clone();
        let now = sx.value(v).clone();
        let t = seeded(sx, &(&now - &was).abs());
        let _ = sx.assert(
            &LinExpr::from_terms([(t, Q::one()), (v, -Q::one())], was.clone()),
            Rel::Ge,
            &Q::zero(),
            Why::Finish,
        );
        let _ = sx.assert(&LinExpr::from_terms([(t, Q::one()), (v, Q::one())], -was), Rel::Ge, &Q::zero(), Why::Finish);
        moved.add_term(t, &Q::one());
    }
    let before = sx.pivots();
    sx.set_pivot_limit(Some(before + cap));
    let mut ok = sx.check().is_ok();
    let extent = if drawn.is_empty() { LinExpr::default() } else { extent };
    for objective in [&length, &extent, &moved] {
        if !ok {
            break;
        }
        ok = sx.minimize(objective).is_ok() && sx.keep_optimum(objective, Why::Finish).is_ok();
    }
    pivots += sx.pivots() - before;
    if !ok {
        sx.backtrack(mark);
        report.failed = Some("the simplex ran out of pivots on the wire length, the extent or the moves".to_string());
        let values = fallback.clone();
        let rows = sx.row_count();
        store.count_pivots(pivots, rows);
        return Some(values);
    }
    sx.backtrack(mark);
    let grid = options.grid.and_then(from_f64).filter(|g| *g > Q::zero());
    let axes: Vec<usize> = sk.axes.iter().copied().filter(|v| symbols.contains(v)).collect();
    let mut visit: Vec<usize> = axes.clone();
    visit.extend(symbols.iter().copied().filter(|v| !axes.contains(v)));
    visit.extend(sk.crosses.iter().filter(|c| c.axis == axis).map(|c| c.centre));
    let mut seen: BTreeSet<usize> = visit.iter().copied().collect();
    for v in sk.origin.iter().flatten().copied().chain(sk.vertices.iter().map(|x| x.term[axis].var)) {
        if seen.insert(v) && v != sk.zero {
            visit.push(v);
        }
    }
    let before = sx.pivots();
    for v in visit {
        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
            break;
        }
        let x = sx.value(v).clone();
        let on = |q: &Q| grid.as_ref().is_none_or(|g| (q / g).is_integer());
        let tries: Vec<Q> = match &grid {
            Some(g) if !on(&x) => {
                let down = (&x / g).floor() * g;
                let up = &down + g;
                if &x - &down <= &up - &x {
                    vec![down, up]
                } else {
                    vec![up, down]
                }
            }
            _ => vec![x.clone()],
        };
        for t in tries {
            let m = sx.checkpoint();
            sx.set_pivot_limit(Some(sx.pivots() + cap));
            let asserted = sx.assert_bounds(v, Some(t.clone()), Some(t.clone()), Why::Finish);
            let checked = if asserted.is_ok() { sx.check() } else { asserted };
            if checked.is_ok() {
                break;
            }
            sx.backtrack(m);
            sx.set_pivot_limit(Some(sx.pivots() + cap));
            let _ = sx.check();
        }
    }
    sx.set_pivot_limit(Some(sx.pivots() + cap));
    let solved = sx.check().is_ok();
    pivots += sx.pivots() - before;
    let values = if solved { (0..sk.vars).map(|v| sx.value(v).clone()).collect() } else { fallback.clone() };
    let rows = sx.row_count();
    if !solved {
        report.failed = Some("the simplex ran out of pivots putting the drawing on the grid".to_string());
    }
    store.count_pivots(pivots, rows);
    Some(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{instance, problem_of, Ticks};
    use crate::{RunLimit, StoppedClock};

    #[test]
    fn interrupted_optimization_returns_a_general_constraint_witness() {
        let mut p = problem_of(vec![instance(), instance()], vec![]);
        p.version = "0.4".into();
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
        let sk = super::super::read::read(
            &p,
            &s,
            &s.witness,
            &crate::route::Routing::default(),
            &|_| true,
            false,
            false,
            None,
            false,
        )
        .unwrap();
        let (a, b) = (sk.origin[0].unwrap(), sk.origin[1].unwrap());
        let general = LinExpr::from_terms([(a, int(1)), (b, int(1))], int(0));
        let mut interrupted_with_values = false;
        for seconds in 1..100 {
            let mut store = Store::new(vec![int(0); sk.vars], sk.zero, PIVOT_CAP);
            store.bound(&general, Some(&int(6)), None, Why::Fact(0));
            assert!(store.settle());
            assert_eq!(store.value(a) + store.value(b), int(0), "graph values alone violate the general bound");
            let mut report = Report::default();
            let (values, _) = crate::budget::with(&Ticks::default(), RunLimit::Time(seconds as f64), None, || {
                finish(&sk, 0, &mut store, &options, &mut report)
            });
            if let Some(values) = values {
                assert!(general.eval(|v| values[v].clone()) >= int(6), "cutoff {seconds}: {values:?}");
                interrupted_with_values |= report.failed.is_some();
            }
        }
        assert!(interrupted_with_values, "exercise the optimizer's fallback after a checked LP");
    }
}
