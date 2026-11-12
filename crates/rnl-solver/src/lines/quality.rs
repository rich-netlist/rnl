//! Exact quality ordering for completed line-layout candidates.
//! An interrupted measurement cannot rank ahead of a completed drawing.

use super::{axis, current, dots, port, supply};
use crate::budget::{permit, Step};
use crate::coords::Placed;
use crate::rational::{int, Q};
use crate::route::Routing;
use crate::structure::Structure;
use num_traits::Zero;
use rnl_runtime::problem::LevelProblem;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LabOrder {
    pub(super) interrupted: bool,
    pub(super) annotation_cost: (usize, usize),
    pub(super) current: usize,
    pub(super) cb: u64,
    pub(super) hidden: usize,
    pub(super) levels: usize,
    pub(super) off_axis: usize,
    pub(super) near_dots: usize,
    pub(super) length: Q,
    pub(super) lines: usize,
    pub(super) crosses: usize,
    pub(super) area: Q,
    pub(super) crossings: u64,
    pub(super) bends: u64,
    pub(super) weights: (u64, u64),
}

impl LabOrder {
    fn interrupted(weights: (u64, u64)) -> Self {
        Self {
            interrupted: true,
            annotation_cost: (0, 0),
            current: 0,
            cb: 0,
            hidden: 0,
            levels: 0,
            off_axis: 0,
            near_dots: 0,
            length: int(0),
            lines: 0,
            crosses: 0,
            area: int(0),
            crossings: 0,
            bends: 0,
            weights,
        }
    }
    pub(super) fn said(&self) -> String {
        format!(
            "{}{} × {} + {} × {} = {}, {}{}{}{}length {}, {} lines of the wires, {} crosses of the symbols, area {}",
            if self.current > 0 {
                format!("{} the current marks left across it, ", crate::driver::count(self.current, "pin"))
            } else {
                String::new()
            },
            self.weights.0,
            self.crossings,
            self.weights.1,
            self.bends,
            self.cb,
            if self.hidden > 0 {
                format!("{} hidden, ", crate::driver::count(self.hidden, "port"))
            } else {
                String::new()
            },
            if self.levels > 0 {
                format!("{} of the grounds and the supplies beyond one, ", crate::driver::count(self.levels, "height"))
            } else {
                String::new()
            },
            if self.off_axis > 0 {
                format!("{} off the axis they share, ", crate::driver::count(self.off_axis, "mirror pair"))
            } else {
                String::new()
            },
            if self.near_dots > 0 {
                format!("{} less than two from a pin, ", crate::driver::count(self.near_dots, "dot"))
            } else {
                String::new()
            },
            crate::rational::fmt(&self.length),
            self.lines,
            self.crosses,
            crate::rational::fmt(&self.area)
        )
    }
}

pub(crate) const LAB_KEY: usize = 6;

/// Re-evaluate coordinate-dependent annotation graphics after every accepted move.
/// The second count prevents compaction from moving wires through visible graphics.
pub(super) fn annotation_cost(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
) -> Option<(usize, usize)> {
    if problem.annotations.is_empty() {
        return Some((0, 0));
    }
    if routing.annotation_error.is_some() {
        return None;
    }
    let boxes = crate::annotation::bounds(problem, &placed.geometry).ok()?;
    let overlaps = crate::annotation::overlaps(problem, s, &placed.geometry, &placed.origins).ok()?;
    let drawing = crate::score::drawn(problem, s, &placed.geometry, routing).ok()?;
    let hits = boxes
        .iter()
        .map(|(_, bounds)| {
            drawing
                .paths
                .iter()
                .map(|(_, path)| {
                    path.windows(2).filter(|p| p[0] != p[1] && crate::score::through(bounds, &p[0], &p[1])).count()
                })
                .sum::<usize>()
        })
        .sum();
    Some((overlaps, hits))
}

pub(super) fn annotations_preserved(
    problem: &LevelProblem,
    s: &Structure,
    before: (&Placed, &Routing),
    after: (&Placed, &Routing),
) -> bool {
    match (annotation_cost(problem, s, before.0, before.1), annotation_cost(problem, s, after.0, after.1)) {
        (Some(before), Some(after)) => after <= before,
        _ => false,
    }
}

pub(crate) fn lab_key(problem: &LevelProblem, s: &Structure, placed: &Placed, routing: &Routing) -> Vec<Q> {
    let l = lab_order(problem, s, placed, routing, (1, 1), None);
    if l.interrupted {
        return vec![int(i64::MAX); LAB_KEY];
    }
    vec![int(l.crossings as i64), int(l.bends as i64), int(l.lines as i64), int(l.crosses as i64), l.length, l.area]
}

pub(super) fn lab_order(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    weights: (u64, u64),
    within: Option<&BTreeSet<usize>>,
) -> LabOrder {
    use rnl_runtime::problem::PointRef;
    use rnl_runtime::solution::WirePoint;
    if !permit(
        Step::Structure,
        problem
            .objects
            .len()
            .saturating_add(problem.wires.len())
            .saturating_add(routing.wires.len())
            .saturating_add(crate::annotation::estimated_work(problem)),
    ) {
        return LabOrder::interrupted(weights);
    }
    let annotation_cost = if within.is_none() {
        annotation_cost(problem, s, placed, routing)
    } else {
        crate::annotation::bounds(problem, &placed.geometry).ok().map(|_| (0, 0))
    };
    let Some(annotation_cost) = annotation_cost else { return LabOrder::interrupted(weights) };
    let g = &placed.geometry;
    let sign = |v: &Q| {
        if v.is_zero() {
            0
        } else if v.is_positive() {
            1
        } else {
            -1
        }
    };
    type Segment = (usize, [Q; 2], [Q; 2], bool, Option<usize>);
    type PathEnds<T> = (T, Option<PointRef>, Option<PointRef>);
    let mut segs: Vec<Segment> = Vec::new();
    let (mut bends, mut length) = (0u64, Q::zero());
    let (mut xs, mut ys): (BTreeSet<Q>, BTreeSet<Q>) = (BTreeSet::new(), BTreeSet::new());
    let mut lo: Option<[Q; 2]> = None;
    let mut hi: Option<[Q; 2]> = None;
    let mut grow = |p: &[Q; 2]| {
        lo = Some(
            lo.take().map_or_else(|| p.clone(), |l| [l[0].clone().min(p[0].clone()), l[1].clone().min(p[1].clone())]),
        );
        hi = Some(
            hi.take().map_or_else(|| p.clone(), |h| [h[0].clone().max(p[0].clone()), h[1].clone().max(p[1].clone())]),
        );
    };
    let reads = |k: usize| within.is_none_or(|w| w.contains(&k));
    let mut written: Vec<PathEnds<Vec<[Q; 2]>>> = Vec::new();
    for (w, wire) in problem.wires.iter().enumerate() {
        if !wire.anchors.iter().all(|r| port::reads_reference(r, &reads)) {
            continue;
        }
        let path = match crate::route::written_path(problem, g, w) {
            Ok(Some(path)) => path,
            Ok(None) => continue,
            Err(_) => return LabOrder::interrupted(weights),
        };
        written.push((path, wire.anchors.first().cloned(), wire.anchors.last().cloned()));
    }
    let ends = |p: &WirePoint| if let WirePoint::Anchor(r) = p { Some(r.clone()) } else { None };
    let all: Vec<PathEnds<&Vec<[Q; 2]>>> = routing
        .wires
        .iter()
        .zip(&routing.paths)
        .map(|(w, p)| (p, w.points.first().and_then(ends), w.points.last().and_then(ends)))
        .chain(written.iter().map(|(p, a, b)| (p, a.clone(), b.clone())))
        .collect();
    for (i, (path, first, last)) in all.into_iter().enumerate() {
        if !permit(Step::Structure, path.len()) {
            return LabOrder::interrupted(weights);
        }
        let mut pts: Vec<&[Q; 2]> = Vec::new();
        for p in path {
            if pts.last().is_none_or(|q| *q != p) {
                pts.push(p);
            }
            grow(p);
        }
        let axis_of = |a: &[Q; 2], b: &[Q; 2]| {
            if a[1] == b[1] {
                Some(0)
            } else if a[0] == b[0] {
                Some(1)
            } else {
                None
            }
        };
        for (k, pair) in pts.windows(2).enumerate() {
            let (a, b) = (pair[0], pair[1]);
            let runs = if axis_of(a, b).is_none() {
                let before = (k > 0).then(|| axis_of(pts[k - 1], a)).flatten();
                before.or_else(|| pts.get(k + 2).and_then(|c| axis_of(b, c)))
            } else {
                None
            };
            segs.push((i, a.clone(), b.clone(), axis_of(a, b).is_none(), runs));
            length += (&b[0] - &a[0]).abs() + (&b[1] - &a[1]).abs();
            match axis_of(a, b) {
                Some(0) => {
                    ys.insert(a[1].clone());
                }
                Some(_) => {
                    xs.insert(a[0].clone());
                }
                None => {}
            }
        }
        let straight = |w: (i64, i64)| (w.0 == 0) != (w.1 == 0);
        for k in 1..pts.len().saturating_sub(1) {
            let u = (sign(&(&pts[k][0] - &pts[k - 1][0])), sign(&(&pts[k][1] - &pts[k - 1][1])));
            let v = (sign(&(&pts[k + 1][0] - &pts[k][0])), sign(&(&pts[k + 1][1] - &pts[k][1])));
            if u != v && straight(u) && straight(v) {
                bends += 1;
            }
        }
        if pts.len() < 2 {
            continue;
        }
        let n = pts.len();
        for (point, end, next) in [(first, pts[0], pts[1]), (last, pts[n - 1], pts[n - 2])] {
            let Some(PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. }) = point else {
                continue;
            };
            let exit = s.exit(object, pin, g.orient[object]);
            match exit {
                Some((_, 0)) => {
                    ys.insert(end[1].clone());
                }
                Some((0, _)) => {
                    xs.insert(end[0].clone());
                }
                _ => {
                    xs.insert(end[0].clone());
                    ys.insert(end[1].clone());
                }
            }
            let way = (sign(&(&next[0] - &end[0])), sign(&(&next[1] - &end[1])));
            if exit.is_some_and(|e| e != way) {
                bends += 1;
            }
        }
    }
    let mut crossings = 0u64;
    for (i, (wa, a0, a1, sa, ra)) in segs.iter().enumerate() {
        for (wb, b0, b1, sb, rb) in &segs[i + 1..] {
            if !permit(Step::Structure, 1) {
                return LabOrder::interrupted(weights);
            }
            if wa == wb {
                continue;
            }
            if *sa || *sb {
                if let Some(at) = through(a0, a1, b0, b1) {
                    crossings += 1;
                    if let (Some(r), Some(_)) = (ra, rb) {
                        if *r == 0 {
                            xs.insert(at[0].clone());
                        } else {
                            ys.insert(at[1].clone());
                        }
                    }
                }
                continue;
            }
            let cross = |h0: &[Q; 2], h1: &[Q; 2], v0: &[Q; 2], v1: &[Q; 2]| {
                h0[1] == h1[1]
                    && v0[0] == v1[0]
                    && h0[0].clone().min(h1[0].clone()) < v0[0]
                    && v0[0] < h0[0].clone().max(h1[0].clone())
                    && v0[1].clone().min(v1[1].clone()) < h0[1]
                    && h0[1] < v0[1].clone().max(v1[1].clone())
            };
            if cross(a0, a1, b0, b1) || cross(b0, b1, a0, a1) {
                crossings += 1;
            }
        }
    }
    let mut crosses = (BTreeSet::new(), BTreeSet::new());
    for k in s.components().filter(|&k| reads(k)) {
        let o = &placed.origins[k];
        crosses.0.insert(o[0].clone());
        crosses.1.insert(o[1].clone());
        if let Some(b) = s.box_at(&placed.origins, &g.orient, k) {
            grow(&[b[0].clone(), b[1].clone()]);
            grow(&[b[2].clone(), b[3].clone()]);
        }
    }
    let area = match (lo, hi) {
        (Some(l), Some(h)) => (&h[0] - &l[0]) * (&h[1] - &l[1]),
        _ => Q::zero(),
    };
    let result = LabOrder {
        interrupted: false,
        annotation_cost,
        current: current::against(s, placed, within),
        hidden: port::hidden(problem, s, placed, routing, within).len(),
        cb: weights.0 * crossings + weights.1 * bends,
        levels: supply::levels(s, placed, within),
        off_axis: axis::off_axis(s, placed, within),
        near_dots: dots::near_pins(problem, s, placed, routing, within),
        lines: xs.len() + ys.len(),
        crosses: crosses.0.len() + crosses.1.len(),
        length,
        area,
        crossings,
        bends,
        weights,
    };
    if crate::budget::stopped() {
        LabOrder::interrupted(weights)
    } else {
        result
    }
}

fn through(a0: &[Q; 2], a1: &[Q; 2], b0: &[Q; 2], b1: &[Q; 2]) -> Option<[Q; 2]> {
    let d = |p: &[Q; 2], q: &[Q; 2]| [&q[0] - &p[0], &q[1] - &p[1]];
    let cross = |u: &[Q; 2], v: &[Q; 2]| &u[0] * &v[1] - &u[1] * &v[0];
    let (r, q) = (d(a0, a1), d(b0, b1));
    let denom = cross(&r, &q);
    if denom.is_zero() {
        return None;
    }
    let w = d(a0, b0);
    let (t, u) = (cross(&w, &q) / &denom, cross(&w, &r) / &denom);
    let inside = |v: &Q| v.is_positive() && *v < int(1);
    (inside(&t) && inside(&u)).then(|| [&a0[0] + &t * &r[0], &a0[1] + &t * &r[1]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::Ticks;
    use crate::{RunLimit, StoppedClock};

    #[test]
    fn interrupted_measurements_never_beat_completed_drawings_and_replay_exactly() {
        let (p, s, placed, routing) = port::tests::fixture();
        let run = || lab_order(&p, &s, &placed, &routing, (1, 4), None);
        let full = run();
        assert!(!full.interrupted);
        assert_eq!(full.cb, full.crossings + 4 * full.bends);
        assert_eq!(full.hidden, 1);
        let mut interrupted = false;
        let mut complete = false;
        for seconds in 0..80 {
            let (timed, used) = crate::budget::with(&Ticks::default(), RunLimit::Time(seconds as f64), None, run);
            let (work, _) = crate::budget::with(&StoppedClock, RunLimit::Work(used.work), None, run);
            assert_eq!(timed, work, "cutoff {seconds}");
            if timed.interrupted {
                interrupted = true;
                assert!(timed > full);
            } else {
                complete = true;
                assert_eq!(timed, full);
            }
        }
        assert!(interrupted && complete);
        let (key, _) =
            crate::budget::with(&StoppedClock, RunLimit::Work(0), None, || lab_key(&p, &s, &placed, &routing));
        assert_eq!(key, vec![int(i64::MAX); LAB_KEY]);
    }
}
