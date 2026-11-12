//! Change spacing while retaining facts, line identities, and wire topology.
use std::collections::{BTreeMap, BTreeSet};

use num_traits::{One, Zero};
use rnl_runtime::problem::LevelProblem;

use super::pass::{finish, spacing, Report, PIVOT_CAP};
use super::read::{self, Kind, Made, Skeleton, Term};
use super::spacing::Policy;
use super::store::{Outcome, Store, Why};
use crate::coords::Placed;
use crate::rational::{int, Q};
use crate::route::Routing;
use crate::structure::Structure;
use crate::{SolverOptions, Work};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Spaced {
    pub placed: Placed,
    pub routing: Routing,
    pub notes: Vec<String>,
    pub moved: Vec<(usize, [Q; 2], [Q; 2])>,
}

struct Wish {
    low: Term,
    high: Term,
    from: Q,
    to: Q,
    pair: (usize, usize),
}

#[derive(Default)]
struct Grown {
    asked: usize,
    full: usize,
    less: usize,
    none: usize,
    facts: BTreeMap<usize, usize>,
}

pub(crate) fn space(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    spacing: [i64; 2],
    options: &SolverOptions,
    work: &mut Work,
) -> Spaced {
    let mut out = Spaced { placed: placed.clone(), routing: routing.clone(), notes: Vec::new(), moved: Vec::new() };
    if spacing == [0, 0]
        && !s.selected_groups.iter().any(|&g| {
            let group = &problem.groups[g];
            [group.spacing_x, group.spacing_y].into_iter().flatten().any(|n| n != 0.)
        })
    {
        return out;
    }
    let Some(mut sk) = read::read(problem, s, &placed.geometry, routing, &|_| true, true, false, None, false) else {
        out.notes.push("spacing read incomplete or a retained fact does not hold; drawing unchanged".into());
        return out;
    };
    if sk.features.is_empty() {
        return out;
    }
    let Some(policy) = Policy::new(problem, s, &sk, spacing) else {
        return stopped(out);
    };
    let lead = int(crate::route::CLEARANCE);
    for (si, seg) in sk.segs.clone().into_iter().enumerate() {
        if !meter(1) {
            return stopped(out);
        }
        let Some(axis) = seg.along else { continue };
        let (a, b) = (&sk.vertices[seg.a], &sk.vertices[seg.b]);
        let (lo, hi) = if a.at[axis] < b.at[axis] { (a, b) } else { (b, a) };
        let length = &hi.at[axis] - &lo.at[axis];
        let feature = sk.features.iter().position(|f| f.kind == Kind::Segment(si));
        let extra = feature.map_or(spacing[axis], |f| policy.extra(f, f, axis));
        let least = if extra >= 0 || length < lead { length } else { lead.clone() };
        let (tl, th) = (lo.term[axis].clone(), hi.term[axis].clone());
        sk.made[axis].push(Made::AtLeast(th.var, th.off, tl.var, tl.off, least, Why::Wire));
    }
    let middles = middles(&mut sk);
    if crate::budget::stopped() {
        return stopped(out);
    }
    if !meter(sk.vertices.len() + sk.pins.len() + sk.features.len()) {
        return stopped(out);
    }
    let axes: Vec<usize> =
        s.pairs.iter().filter_map(|p| p.axis.and_then(|k| sk.origin.get(k).copied().flatten())).collect();
    let lines = lines(&sk, &middles, &axes);
    let mut failed = false;
    let mut said = Vec::new();
    for axis in (0..2).filter(|&a| policy.active(a)) {
        if !meter(sk.vars) {
            return stopped(out);
        }
        let name = if axis == 0 { "x" } else { "y" };
        let setting = policy.description(axis);
        let ties: Vec<(Term, Term)> = lines[axis]
            .iter()
            .flat_map(|by| by.values())
            .flat_map(|terms| terms[1..].iter().map(|t| (terms[0].clone(), t.clone())))
            .collect();
        let read = sk.value[axis].clone();
        let mut apart: Vec<(Term, Term, Q)> = Vec::new();
        let (mut report, mut grown) = (Report::default(), Grown::default());
        for _ in 0..MOST_AGAIN {
            sk.value[axis] = read.clone();
            (report, grown) = pass(&mut sk, axis, &policy, &ties, &apart, options, work);
            if crate::budget::stopped() {
                return stopped(out);
            }
            let met = if report.failed.is_none() { together(&sk, axis, &lines[axis]) } else { Vec::new() };
            if met.is_empty() {
                break;
            }
            apart.extend(met);
        }
        if report.failed.is_none() && !together(&sk, axis, &lines[axis]).is_empty() {
            report.failed = Some("two lines apart stay together".to_string());
        }
        if let Some(why) = report.failed {
            sk.value[axis] = read;
            out.notes.push(format!("spacing {setting}: on {name}, {why}; the drawing is left as it was"));
            failed = true;
            break;
        }
        let facts: Vec<String> = grown.facts.iter().map(|(f, n)| format!("{} ({n})", fact_name(problem, *f))).collect();
        said.push(format!(
            "spacing {setting} on {name}: {} gaps asked to be wider, {} as asked, {} less, {} not at all{}",
            grown.asked,
            grown.full,
            grown.less,
            grown.none,
            if facts.is_empty() { String::new() } else { format!("; the facts that held gaps: {}", facts.join(", ")) }
        ));
    }
    if failed {
        return out;
    }
    out.notes = said;
    let (p, r) = super::written(problem, s, placed, routing, &sk);
    out.placed = p;
    out.routing = r;
    out.moved = sk
        .vertices
        .iter()
        .flat_map(|v| v.trees.iter().map(|&tree| (tree, v.at.clone(), [sk.at(0, &v.term[0]), sk.at(1, &v.term[1])])))
        .collect();
    out
}

fn fact_name(problem: &LevelProblem, f: usize) -> String {
    let fact = &problem.facts[f];
    let name = if fact.label.is_empty() { format!("fact {f}") } else { fact.label.clone() };
    match fact.chain.last().and_then(|&u| problem.usages.get(u)) {
        Some(u) if !u.label.is_empty() => format!("{name} of {}", u.label),
        _ => name,
    }
}

fn middles(sk: &mut Skeleton) -> Vec<usize> {
    let span = |sk: &Skeleton, g: usize, axis: usize| {
        let (a, b) = (&sk.vertices[sk.segs[g].a], &sk.vertices[sk.segs[g].b]);
        if a.at[axis] <= b.at[axis] {
            (a.term[axis].clone(), b.term[axis].clone(), a.at[axis].clone(), b.at[axis].clone())
        } else {
            (b.term[axis].clone(), a.term[axis].clone(), b.at[axis].clone(), a.at[axis].clone())
        }
    };
    let slants: Vec<usize> = (0..sk.segs.len()).filter(|&g| sk.segs[g].slant).collect();
    let mut out = Vec::new();
    for (i, &g) in slants.iter().enumerate() {
        for &h in &slants[i + 1..] {
            if !meter(1) {
                return out;
            }
            let same = (0..2).all(|a| {
                let (x, y) = (span(sk, g, a), span(sk, h, a));
                x.2 == y.2 && x.3 == y.3
            });
            let (ag, ah) = (&sk.vertices[sk.segs[g].a].at, &sk.vertices[sk.segs[h].a].at);
            let crossing = ag != ah && ag != &sk.vertices[sk.segs[h].b].at;
            if !same || !crossing {
                continue;
            }
            let at = [0, 1].map(|a| {
                let (_, _, lo, hi) = span(sk, g, a);
                (lo + hi) / int(2)
            });
            let v = sk.vars;
            sk.vars += 1;
            for (axis, value) in at.into_iter().enumerate() {
                sk.value[axis].push(value);
                let (lo, hi, _, _) = span(sk, g, axis);
                let mut e = crate::simplex::LinExpr::default();
                e.add_term(v, &int(2));
                e.add_term(lo.var, &-Q::one());
                e.add_term(hi.var, &-Q::one());
                e.add_constant(&-(&lo.off + &hi.off));
                sk.made[axis].push(Made::Bound(e, Some(Q::zero()), Some(Q::zero()), Why::Wire));
            }
            out.push(v);
        }
    }
    out
}

const MOST_AGAIN: usize = 4;

fn lines(sk: &Skeleton, middles: &[usize], axes: &[usize]) -> [[BTreeMap<Q, Vec<Term>>; 2]; 2] {
    [0, 1].map(|axis| {
        let mut wires: BTreeMap<Q, Vec<Term>> = BTreeMap::new();
        let on = |by: &mut BTreeMap<Q, Vec<Term>>, t: &Term| by.entry(sk.at(axis, t)).or_default().push(t.clone());
        for v in &sk.vertices {
            on(&mut wires, &v.term[axis]);
        }
        for (_, t) in &sk.pins {
            on(&mut wires, &t[axis]);
        }
        for &v in middles.iter().chain(axes) {
            on(&mut wires, &Term::of(v, Q::zero()));
        }
        let mut origins: BTreeMap<Q, Vec<Term>> = BTreeMap::new();
        let symbols: BTreeSet<usize> =
            sk.features.iter().filter_map(|f| if let Kind::Symbol(k) = f.kind { sk.origin[k] } else { None }).collect();
        for v in symbols {
            on(&mut origins, &Term::of(v, Q::zero()));
        }
        [wires, origins]
    })
}

fn together(sk: &Skeleton, axis: usize, lines: &[BTreeMap<Q, Vec<Term>>; 2]) -> Vec<(Term, Term, Q)> {
    let mut out = Vec::new();
    for by in lines {
        let mut now: BTreeMap<Q, Vec<(&Q, &Term)>> = BTreeMap::new();
        for (was, terms) in by {
            now.entry(sk.at(axis, &terms[0])).or_default().push((was, &terms[0]));
        }
        for met in now.values().filter(|m| m.len() > 1) {
            for pair in met.windows(2) {
                let ((a, ta), (b, tb)) = (pair[0], pair[1]);
                let d = b - a;
                out.push((ta.clone(), tb.clone(), if d < int(1) { d } else { int(1) }));
            }
        }
    }
    out
}

fn pass(
    sk: &mut Skeleton,
    axis: usize,
    policy: &Policy,
    ties: &[(Term, Term)],
    kept: &[(Term, Term, Q)],
    options: &SolverOptions,
    work: &mut Work,
) -> (Report, Grown) {
    let mut report = Report { axis, ..Report::default() };
    let mut grown = Grown::default();
    if !meter(sk.vars) {
        report.stop();
        return (report, grown);
    }
    let low = sk.value[axis].iter().min().cloned().unwrap_or_default();
    let mut walls = vec![low; sk.vars];
    walls[sk.zero] = Q::zero();
    let mut store = Store::new(walls, sk.zero, PIVOT_CAP);
    for m in &sk.made[axis] {
        if !meter(1) {
            report.stop();
            return (report, grown);
        }
        match m {
            Made::Equal(a, ca, b, cb, why) => store.equal(*a, ca, *b, cb, *why),
            Made::AtLeast(a, ca, b, cb, w, why) => store.at_least(*a, ca, *b, cb, w, *why),
            Made::Bound(e, lo, hi, why) => store.bound(e, lo.as_ref(), hi.as_ref(), *why),
        }
    }
    for (a, b) in ties {
        if !meter(1) {
            report.stop();
            return (report, grown);
        }
        store.equal(a.var, &a.off, b.var, &b.off, Why::Body);
    }
    for (lo, hi, w) in kept {
        if !meter(1) {
            report.stop();
            return (report, grown);
        }
        store.at_least(hi.var, &hi.off, lo.var, &lo.off, w, Why::Body);
    }
    let mut wishes = apart(sk, axis, policy, &mut store);
    if crate::budget::stopped() {
        report.stop();
        return (report, grown);
    }
    if !store.settle() {
        report.failed = Some(
            if matches!(store.failure(), Some(Outcome::Undecided)) {
                "the drawing constraint check was incomplete"
            } else {
                "the drawing as read has no solution of its own constraints"
            }
            .into(),
        );
        return (report, grown);
    }
    if store.has_general() {
        let drawn = sk.value[axis].clone();
        if store.simplex(&|v| drawn[v].clone()).is_err() {
            report.failed = Some(
                if matches!(store.failure(), Some(Outcome::Undecided)) {
                    "the drawing simplex check was incomplete"
                } else {
                    "the simplex found no solution of the drawing as read"
                }
                .into(),
            );
            return (report, grown);
        }
    }
    wishes.sort_by(|a, b| a.from.cmp(&b.from).then(a.pair.cmp(&b.pair)));
    grown.asked = wishes.len();
    for w in wishes {
        if !meter(1) {
            report.stop();
            return (report, grown);
        }
        let why = Why::Apart(w.pair.0, w.pair.1);
        let mut to = w.to.clone();
        let mut held = BTreeSet::new();
        while to > w.from {
            if !meter(1) {
                report.stop();
                return (report, grown);
            }
            match store.try_at_least(w.high.var, &w.high.off, w.low.var, &w.low.off, &to, why) {
                Outcome::Kept => break,
                Outcome::Conflict(whys) => {
                    held.extend(whys.iter().filter_map(|w| if let Why::Fact(f) = w { Some(*f) } else { None }));
                }
                Outcome::Undecided => {}
            }
            to = &to - int(1);
        }
        if to >= w.to {
            grown.full += 1;
        } else if to > w.from {
            grown.less += 1;
        } else {
            grown.none += 1;
        }
        for f in held {
            *grown.facts.entry(f).or_default() += 1;
        }
    }
    let values = finish(sk, axis, &mut store, options, &mut report);
    if crate::budget::stopped() {
        report.stop();
    }
    work.pivots += store.pivots;
    work.astar_expansions += store.relaxations;
    if let Some(values) = values {
        sk.value[axis] = values;
    }
    (report, grown)
}

fn asked(gap: i64, extra: i64) -> i64 {
    if extra >= 0 {
        gap + extra
    } else {
        (gap + extra).max(gap.min(1))
    }
}

fn apart(sk: &Skeleton, axis: usize, policy: &Policy, store: &mut Store) -> Vec<Wish> {
    let mut wishes = Vec::new();
    let across = 1 - axis;
    let n = sk.features.len();
    if !meter(n) {
        return wishes;
    }
    let lo_a: Vec<Q> = sk.features.iter().map(|f| sk.at(axis, &f.lo[axis])).collect();
    let hi_a: Vec<Q> = sk.features.iter().map(|f| sk.at(axis, &f.hi[axis])).collect();
    let lo_c: Vec<Q> = sk.features.iter().map(|f| sk.at(across, &f.lo[across])).collect();
    let hi_c: Vec<Q> = sk.features.iter().map(|f| sk.at(across, &f.hi[across])).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| lo_c[i].cmp(&lo_c[j]).then(i.cmp(&j)));
    let widest = int(2);
    for (at, &i) in order.iter().enumerate() {
        for &j in &order[at + 1..] {
            if !meter(1) {
                return wishes;
            }
            if lo_c[j] >= &hi_c[i] + &widest {
                break;
            }
            let (fi, fj) = (&sk.features[i], &sk.features[j]);
            let Some(gap) = spacing(sk, fi, fj) else { continue };
            let extra = policy.extra(i, j, axis);
            let (g, to) = (int(gap), int(asked(gap, extra)));
            if !(lo_c[i] < &hi_c[j] + &g && lo_c[j] < &hi_c[i] + &g) {
                continue;
            }
            if hi_a[i] <= lo_a[j] || hi_a[j] <= lo_a[i] {
                let ((l, vl), (h, vh)) = if hi_a[i] <= lo_a[j] {
                    ((&fi.hi[axis], &hi_a[i]), (&fj.lo[axis], &lo_a[j]))
                } else {
                    ((&fj.hi[axis], &hi_a[j]), (&fi.lo[axis], &lo_a[i]))
                };
                let d = vh - vl;
                let w = if extra >= 0 || d < to { d.clone() } else { to.clone() };
                store.at_least(h.var, &h.off, l.var, &l.off, &w, Why::Apart(i, j));
                if extra != 0 && d < to {
                    wishes.push(Wish { low: l.clone(), high: h.clone(), from: d, to: to.clone(), pair: (i, j) });
                }
                continue;
            }
            let near = if to < g { to } else { g };
            let ends = |f: usize| [(&sk.features[f].lo[axis], &lo_a[f]), (&sk.features[f].hi[axis], &hi_a[f])];
            for (ta, va) in ends(i) {
                for (tb, vb) in ends(j) {
                    let (l, h, d) = if va < vb {
                        (ta, tb, vb - va)
                    } else if vb < va {
                        (tb, ta, va - vb)
                    } else {
                        continue;
                    };
                    let w = if extra >= 0 || d < near { d } else { near.clone() };
                    store.at_least(h.var, &h.off, l.var, &l.off, &w, Why::Apart(i, j));
                }
            }
        }
    }
    wishes
}

fn meter(n: usize) -> bool {
    crate::budget::permit(crate::budget::Step::Structure, n)
}

fn stopped(mut out: Spaced) -> Spaced {
    out.notes.push("spacing stopped at the shared budget; drawing unchanged".into());
    out
}
