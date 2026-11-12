//! Detect unobstructed swaps of two routed trees for compacting diagonal crossings.

use std::collections::BTreeSet;

use num_traits::Zero;
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::solution::WirePoint;

use crate::rational::Q;
use crate::resolve::Geometry;
use crate::route::Routing;
use crate::structure::Structure;

#[derive(Clone, Debug)]
pub(crate) struct Cross {
    pub axis: usize,
    pub jogs: [usize; 2],
    pub low: [usize; 2],
    pub high: [usize; 2],
    pub runs: [usize; 4],
    pub centre: usize,
    pub objects: BTreeSet<usize>,
    pub nets: [String; 2],
    pub pair: Option<Mid>,
}

#[derive(Clone, Debug)]
pub(crate) enum Mid {
    Axis(usize),
    Between(usize, usize),
}

impl Cross {
    pub(crate) fn owns(&self, seg: usize) -> bool {
        self.jogs.contains(&seg) || self.runs.contains(&seg)
    }
}

#[derive(Clone, Debug)]
pub(super) struct Found {
    pub axis: usize,
    pub jogs: [(usize, usize); 2],
}

#[derive(Clone, Debug)]
struct Jog {
    wire: usize,
    at: usize,
    along: Q,
    low: Q,
    high: Q,
    reach: (Q, Q),
    ends: (bool, bool),
}

pub(super) fn find(
    problem: &LevelProblem,
    s: &Structure,
    g: &Geometry,
    routing: &Routing,
    within: Option<&BTreeSet<usize>>,
) -> Vec<Found> {
    if !crate::budget::permit(crate::budget::Step::Structure, problem.objects.len()) {
        return Vec::new();
    }
    let origins = s.origins(problem, g);
    let net = |w: usize| &problem.routes[routing.wires[w].route].net;
    let mut used: BTreeSet<(usize, usize)> = BTreeSet::new();
    let own = |j: &Jog| [(j.wire, j.at - 1), (j.wire, j.at), (j.wire, j.at + 1)];
    let mut out = Vec::new();
    for axis in 0..2 {
        let jogs = jogs(routing, axis);
        if crate::budget::stopped() {
            return out;
        }
        for (i, a) in jogs.iter().enumerate() {
            for b in &jogs[i + 1..] {
                if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                    return out;
                }
                if net(a.wire) == net(b.wire) || own(a).iter().chain(&own(b)).any(|x| used.contains(x)) {
                    continue;
                }
                let Some((first, second)) = swap(problem, s, g, &origins, routing, within, axis, a, b) else {
                    continue;
                };
                used.extend(own(a));
                used.extend(own(b));
                let pair = if first.low > second.low { [first, second] } else { [second, first] };
                out.push(Found { axis, jogs: pair.map(|j| (j.wire, j.at)) });
            }
        }
    }
    out
}

fn jogs(routing: &Routing, axis: usize) -> Vec<Jog> {
    let across = 1 - axis;
    let mut out = Vec::new();
    for (w, path) in routing.paths.iter().enumerate() {
        let points = &routing.wires[w].points;
        if points.len() != path.len() {
            continue;
        }
        for j in 1..path.len().saturating_sub(2) {
            if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                return out;
            }
            let (p, q, r, t) = (&path[j - 1], &path[j], &path[j + 1], &path[j + 2]);
            let bends = matches!(points[j], WirePoint::Bend(_)) && matches!(points[j + 1], WirePoint::Bend(_));
            if !bends
                || q[axis] != r[axis]
                || q[across] == r[across]
                || p[across] != q[across]
                || r[across] != t[across]
            {
                continue;
            }
            let (before, after) = (&q[axis] - &p[axis], &t[axis] - &r[axis]);
            if before.is_zero() || after.is_zero() || before.is_positive() != after.is_positive() {
                continue;
            }
            let end = |k: usize| !matches!(points[k], WirePoint::Anchor(_));
            let (low, high, reach, ends) = if before.is_positive() {
                (q[across].clone(), r[across].clone(), (p[axis].clone(), t[axis].clone()), (end(j - 1), end(j + 2)))
            } else {
                (r[across].clone(), q[across].clone(), (t[axis].clone(), p[axis].clone()), (end(j + 2), end(j - 1)))
            };
            out.push(Jog { wire: w, at: j, along: q[axis].clone(), low, high, reach, ends });
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn swap(
    problem: &LevelProblem,
    s: &Structure,
    g: &Geometry,
    origins: &[[Q; 2]],
    routing: &Routing,
    within: Option<&BTreeSet<usize>>,
    axis: usize,
    a: &Jog,
    b: &Jog,
) -> Option<(Jog, Jog)> {
    if a.along == b.along {
        return None;
    }
    let (first, second) = if a.along < b.along { (a, b) } else { (b, a) };
    if (&first.high - &first.low).is_positive() == (&second.high - &second.low).is_positive() {
        return None;
    }
    let (at_first, at_second) = (&first.low - &second.low, &first.high - &second.high);
    if at_first.is_zero() || at_second.is_zero() || at_first.is_positive() == at_second.is_positive() {
        return None;
    }
    let (from, to) = (first.along.clone(), second.along.clone());
    let fits = [first, second].iter().all(|j| {
        (j.reach.0 < from || j.ends.0 && j.reach.0 == from) && (j.reach.1 > to || j.ends.1 && j.reach.1 == to)
    });
    if !fits {
        return None;
    }
    let near = |w: usize, k: usize| -> Vec<([Q; 2], [Q; 2])> {
        let path = &routing.paths[w];
        (k.saturating_sub(1)..(k + 2).min(path.len() - 1)).map(|i| (path[i].clone(), path[i + 1].clone())).collect()
    };
    let crossings = near(first.wire, first.at)
        .iter()
        .flat_map(|x| near(second.wire, second.at).into_iter().map(move |y| (x.clone(), y)))
        .filter(|(x, y)| crosses(x, y))
        .count();
    if crossings != 1 {
        return None;
    }
    let mine = [(first.wire, first.at), (second.wire, second.at)];
    clear(problem, s, g, origins, routing, within, axis, &mine, &from, &to, first, second)
        .then(|| (first.clone(), second.clone()))
}

fn crosses(x: &([Q; 2], [Q; 2]), y: &([Q; 2], [Q; 2])) -> bool {
    let between = |v: &Q, a: &Q, b: &Q| (v > a && v < b) || (v > b && v < a);
    let along = |s: &([Q; 2], [Q; 2])| {
        if s.0[0] == s.1[0] {
            Some(1)
        } else if s.0[1] == s.1[1] {
            Some(0)
        } else {
            None
        }
    };
    match (along(x), along(y)) {
        (Some(ax), Some(ay)) if ax != ay => {
            let c = 1 - ax;
            between(&x.0[c], &y.0[c], &y.1[c]) && between(&y.0[ax], &x.0[ax], &x.1[ax])
        }
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn clear(
    problem: &LevelProblem,
    s: &Structure,
    g: &Geometry,
    origins: &[[Q; 2]],
    routing: &Routing,
    within: Option<&BTreeSet<usize>>,
    axis: usize,
    mine: &[(usize, usize)],
    from: &Q,
    to: &Q,
    first: &Jog,
    second: &Jog,
) -> bool {
    if !crate::budget::permit(crate::budget::Step::Structure, routing.paths.len().saturating_add(problem.objects.len()))
    {
        return false;
    }
    let across = 1 - axis;
    let reads = |k: usize| within.is_none_or(|w| w.contains(&k));
    let ys = [&first.low, &first.high, &second.low, &second.high];
    let (lo, hi) = (ys.iter().min().map(|v| (*v).clone()), ys.iter().max().map(|v| (*v).clone()));
    let (Some(lo), Some(hi)) = (lo, hi) else { return false };
    let inside = |p: &[Q; 2]| p[axis] > *from && p[axis] < *to && p[across] > lo && p[across] < hi;
    let meets = |a: &[Q; 2], b: &[Q; 2], open: bool| -> bool {
        let (amin, amax) = if a[axis] <= b[axis] { (&a[axis], &b[axis]) } else { (&b[axis], &a[axis]) };
        let (cmin, cmax) = if a[across] <= b[across] { (&a[across], &b[across]) } else { (&b[across], &a[across]) };
        if open {
            amax > from && amin < to && *cmax > lo && *cmin < hi
        } else {
            amax >= from && amin <= to && *cmax >= lo && *cmin <= hi
        }
    };
    let net = |w: usize| &problem.routes[routing.wires[w].route].net;
    let nets = [net(first.wire), net(second.wire)];
    for (w, path) in routing.paths.iter().enumerate() {
        let open = nets.contains(&net(w));
        for i in 0..path.len().saturating_sub(1) {
            if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                return false;
            }
            let own = mine.iter().any(|&(mw, k)| mw == w && i + 1 >= k && i <= k + 1);
            if own {
                continue;
            }
            if meets(&path[i], &path[i + 1], open) {
                return false;
            }
        }
    }
    for w in 0..problem.wires.len() {
        if !crate::budget::permit(crate::budget::Step::Structure, problem.wires[w].anchors.len().saturating_mul(3)) {
            return false;
        }
        use rnl_runtime::problem::PointRef;
        let read = problem.wires[w].anchors.iter().all(|r| match r {
            PointRef::Pin { object, .. }
            | PointRef::SymbolPin { object, .. }
            | PointRef::PinPosition { object, .. }
            | PointRef::Origin(object) => reads(*object),
            PointRef::Centre(_) => true,
        });
        if !read {
            continue;
        }
        let Some(path) = crate::route::written_path(problem, g, w) else { continue };
        if path.windows(2).any(|e| meets(&e[0], &e[1], problem.wires[w].nets.iter().any(|n| nets.contains(&n)))) {
            return false;
        }
    }
    let overlaps = |b: &[Q; 4]| b[2 + axis] > *from && b[axis] < *to && b[2 + across] > lo && b[across] < hi;
    for k in s.components().filter(|&k| reads(k)) {
        if !crate::budget::permit(crate::budget::Step::Structure, s.pins(k).len()) {
            return false;
        }
        if s.box_at(origins, &g.orient, k).is_some_and(|b| overlaps(&b)) {
            return false;
        }
        for pin in 0..s.pins(k).len() {
            if let Some(p) = s.pin_at(origins, &g.orient, k, pin) {
                if inside(&p) {
                    return false;
                }
            }
        }
    }
    for (k, _) in s.objects.iter().enumerate().filter(|(k, object)| reads(*k) && !object.left_free) {
        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
            return false;
        }
        for text in s.text_offsets(problem, g, k) {
            let at = &origins[k];
            if overlaps(&[&at[0] + &text[0], &at[1] + &text[1], &at[0] + &text[2], &at[1] + &text[3]]) {
                return false;
            }
        }
    }
    for (w, path) in routing.paths.iter().enumerate() {
        let points = &routing.wires[w].points;
        for (p, wp) in path.iter().zip(points) {
            if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                return false;
            }
            if matches!(wp, WirePoint::Branch(_)) && inside(p) {
                return false;
            }
        }
    }
    true
}
