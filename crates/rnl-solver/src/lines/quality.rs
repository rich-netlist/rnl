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

#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// The configured rules of the lines layout (overlaps, crowding, misleading, wire over symbol),
    /// as the tree's rank has them; measured for a whole level only.
    pub(super) rules: Vec<Q>,
    /// The weight of a loose virtual symbol (`virtual_weight`); 1 ranks by the counts.
    pub(super) virtual_weight: Q,
    /// The heights of the grounds and supplies beyond one, a height only loose symbols take
    /// weighed by `virtual_weight`.
    pub(super) levels_weighed: Q,
    /// The rows and columns of the symbols, one only loose virtual symbols take weighed by
    /// `virtual_weight`.
    pub(super) crosses_weighed: Q,
}

/// Two weighed counts as the line layout chooses by them below a weight of 1: alike when they
/// differ by less than a whole unit, within [`unit_tolerance`] as the rank of its last choice has
/// it.
fn by_unit(a: &Q, b: &Q) -> std::cmp::Ordering {
    if (a - b).abs() <= unit_tolerance() {
        std::cmp::Ordering::Equal
    } else {
        a.cmp(b)
    }
}

/// The tolerance of a weighed layer in a rank ([`super::Rank::for_choice`]): just under a whole
/// unit.
pub(super) fn unit_tolerance() -> Q {
    int(1) - int(1) / int(1 << 20)
}

impl LabOrder {
    #[allow(clippy::type_complexity)]
    fn key(
        &self,
    ) -> ((bool, (usize, usize)), &Vec<Q>, (usize, u64), (usize, usize, usize, usize), (&Q, usize, usize, &Q), (u64, u64))
    {
        (
            (self.interrupted, self.annotation_cost),
            &self.rules,
            (self.current, self.cb),
            (self.hidden, self.levels, self.off_axis, self.near_dots),
            (&self.length, self.lines, self.crosses, &self.area),
            (self.crossings, self.bends),
        )
    }

    /// The layers of the unified key after the annotations: the current marks, the weighed crossings
    /// and bends, the hidden ports, the heights, the pairs off their axis, the dots near pins.
    pub(super) fn head(&self) -> Vec<Q> {
        [
            self.current as u64,
            self.cb,
            self.hidden as u64,
            self.levels as u64,
            self.off_axis as u64,
            self.near_dots as u64,
        ]
        .into_iter()
        .map(|v| int(v as i64))
        .collect()
    }

    /// The layers of the unified key after the rules: length, lines of the wires, crosses, area.
    pub(super) fn tail(&self) -> Vec<Q> {
        vec![self.length.clone(), int(self.lines as i64), int(self.crosses as i64), self.area.clone()]
    }

    /// [`LabOrder::head`] with the heights weighed by `virtual_weight`.
    pub(super) fn head_weighed(&self) -> Vec<Q> {
        let mut head = self.head();
        head[3] = self.levels_weighed.clone();
        head
    }

    /// [`LabOrder::tail`] with the rows and columns of the symbols weighed by `virtual_weight`.
    pub(super) fn tail_weighed(&self) -> Vec<Q> {
        let mut tail = self.tail();
        tail[2] = self.crosses_weighed.clone();
        tail
    }
}

impl PartialOrd for LabOrder {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LabOrder {
    /// By the key, a total order; below a `virtual_weight` of 1 with the heights and the rows and
    /// columns of the symbols weighed. Where the line layout chooses a drawing it compares by
    /// [`LabOrder::choose`].
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.compare(other, false)
    }
}

impl LabOrder {
    /// As the line layout chooses between two drawings: [`Ord::cmp`], but below a
    /// `virtual_weight` of 1 the two weighed counts alike unless they differ by a whole unit
    /// ([`by_unit`]). Not transitive, so never the order of a sort.
    pub(super) fn choose(&self, other: &Self) -> std::cmp::Ordering {
        self.compare(other, true)
    }

    fn compare(&self, other: &Self, units: bool) -> std::cmp::Ordering {
        if self.virtual_weight == int(1) && other.virtual_weight == int(1) {
            return self.key().cmp(&other.key());
        }
        let by_unit = |x: &Q, y: &Q| if units { by_unit(x, y) } else { x.cmp(y) };
        let (a, b) = (self.key(), other.key());
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(b.1))
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.3 .0.cmp(&b.3 .0))
            .then_with(|| by_unit(&self.levels_weighed, &other.levels_weighed))
            .then_with(|| (a.3 .2, a.3 .3).cmp(&(b.3 .2, b.3 .3)))
            .then_with(|| (a.4 .0, a.4 .1).cmp(&(b.4 .0, b.4 .1)))
            .then_with(|| by_unit(&self.crosses_weighed, &other.crosses_weighed))
            .then_with(|| a.4 .3.cmp(b.4 .3))
            .then_with(|| a.5.cmp(&b.5))
    }
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
            rules: Vec::new(),
            virtual_weight: int(1),
            levels_weighed: int(0),
            crosses_weighed: int(0),
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

pub(super) fn lab_order(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    weights: (u64, u64),
    virtual_weight: &Q,
    within: Option<&BTreeSet<usize>>,
) -> LabOrder {
    let mut l = lab_measure(problem, s, placed, routing, weights, virtual_weight, within);
    if within.is_none() && !l.interrupted {
        let objectives = crate::score::drawn(problem, s, &placed.geometry, routing)
            .map(|w| crate::score::measured_layers(problem, s, placed, Some(&w)))
            .unwrap_or_default();
        l.rules =
            super::rank::rule_layers(s).iter().map(|&i| objectives.get(i).cloned().unwrap_or_else(Q::zero)).collect();
    }
    l
}

pub(super) fn lab_measure(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    weights: (u64, u64),
    virtual_weight: &Q,
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
    // below a weight of 1, each row and column by whether a firm symbol takes it
    let loose = (*virtual_weight != int(1)).then(|| supply::loose(problem, s));
    let mut firm: (std::collections::BTreeMap<Q, bool>, std::collections::BTreeMap<Q, bool>) = Default::default();
    for k in s.components().filter(|&k| reads(k)) {
        let o = &placed.origins[k];
        crosses.0.insert(o[0].clone());
        crosses.1.insert(o[1].clone());
        if let Some(loose) = &loose {
            let f = !loose.contains(&k);
            *firm.0.entry(o[0].clone()).or_insert(false) |= f;
            *firm.1.entry(o[1].clone()).or_insert(false) |= f;
        }
        if let Some(b) = s.box_at(&placed.origins, &g.orient, k) {
            grow(&[b[0].clone(), b[1].clone()]);
            grow(&[b[2].clone(), b[3].clone()]);
        }
    }
    let area = match (lo, hi) {
        (Some(l), Some(h)) => (&h[0] - &l[0]) * (&h[1] - &l[1]),
        _ => Q::zero(),
    };
    let levels = supply::levels(s, placed, within);
    let (levels_weighed, crosses_weighed) = match &loose {
        None => (int(levels as i64), int((crosses.0.len() + crosses.1.len()) as i64)),
        Some(loose) => (
            supply::levels_weighed(s, placed, within, loose, virtual_weight),
            supply::weighed_lines(&firm.0, virtual_weight) + supply::weighed_lines(&firm.1, virtual_weight),
        ),
    };
    let result = LabOrder {
        interrupted: false,
        annotation_cost,
        current: current::against(s, placed, within),
        hidden: port::hidden(problem, s, placed, routing, within).len(),
        cb: weights.0 * crossings + weights.1 * bends,
        levels,
        off_axis: axis::off_axis(s, placed, within),
        near_dots: dots::near_pins(problem, s, placed, routing, within),
        lines: xs.len() + ys.len(),
        crosses: crosses.0.len() + crosses.1.len(),
        length,
        area,
        crossings,
        bends,
        weights,
        rules: Vec::new(),
        virtual_weight: virtual_weight.clone(),
        levels_weighed,
        crosses_weighed,
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

    /// A loose symbol at a height of its own against a drawing with more dots near pins: by the
    /// counts the height decides; at a weight of one half it is half a unit, which does not, and
    /// two such heights, a whole unit, do.
    #[test]
    fn below_a_weight_of_one_the_weighed_counts_differ_only_by_a_whole_unit() {
        let order = |w: Q, levels: usize, weighed: Q, dots: usize| {
            let mut l = LabOrder::interrupted((1, 4));
            l.interrupted = false;
            (l.virtual_weight, l.levels, l.levels_weighed, l.near_dots) = (w, levels, weighed, dots);
            l.crosses_weighed = int(0);
            l
        };
        let half = int(1) / int(2);
        assert!(order(int(1), 1, int(1), 0).choose(&order(int(1), 0, int(0), 5)).is_gt());
        assert!(order(half.clone(), 1, half.clone(), 0).choose(&order(half.clone(), 0, int(0), 5)).is_lt());
        assert!(order(half.clone(), 2, int(1), 0).choose(&order(half.clone(), 0, int(0), 5)).is_gt());
        // the rows and columns of the symbols alike, after the length and the lines of the wires
        let lines = |w: Q, weighed: Q, area: i64| {
            let mut l = order(w, 0, int(0), 0);
            (l.crosses_weighed, l.area) = (weighed, int(area));
            l
        };
        assert!(lines(half.clone(), half.clone(), 1).choose(&lines(half.clone(), int(0), 2)).is_lt());
        assert!(lines(half.clone(), int(1), 1).choose(&lines(half.clone(), int(0), 2)).is_gt());
    }

    /// By whole units the choice is not transitive: at a weight of 0.6, heights 1.2, 0.6 and 0 with
    /// 0, 3 and 5 dots near pins choose round a cycle. The order the line search sorts by compares
    /// the weighed counts exactly, a total order.
    #[test]
    fn a_sort_takes_a_total_order_where_the_choice_by_whole_units_is_not() {
        let w = int(3) / int(5);
        let order = |weighed: Q, dots: usize| {
            let mut l = LabOrder::interrupted((1, 4));
            l.interrupted = false;
            (l.virtual_weight, l.levels_weighed, l.near_dots) = (w.clone(), weighed, dots);
            l.crosses_weighed = int(0);
            l
        };
        let (x, y, z) = (order(&w * int(2), 0), order(w.clone(), 3), order(int(0), 5));
        assert!(x.choose(&y).is_lt() && y.choose(&z).is_lt() && z.choose(&x).is_lt());
        assert!(z < y && y < x && z < x);
        let mut all = [x.clone(), y.clone(), z.clone(), x, y, z];
        all.sort();
        assert!(all.windows(2).all(|p| p[0] <= p[1]));
    }
    use crate::resolve::tests::Ticks;
    use crate::{RunLimit, StoppedClock};

    #[test]
    fn interrupted_measurements_never_beat_completed_drawings_and_replay_exactly() {
        let (p, s, placed, routing) = port::tests::fixture();
        let run = || lab_order(&p, &s, &placed, &routing, (1, 4), &int(1), None);
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
        let (stopped, _) = crate::budget::with(&StoppedClock, RunLimit::Work(0), None, || {
            lab_order(&p, &s, &placed, &routing, (1, 4), &int(1), None)
        });
        assert!(stopped.interrupted && stopped > full);
    }

    /// A wire that takes the default route of `auto` (13.1) from a pin that leaves sideways to a
    /// pin one unit below it leaves the first at a right angle, straight down into the second: one
    /// bend, the turn at the pin, and a length of one. The order of the lines layout and the
    /// routing objectives count it as it is drawn, so that a placement whose pins face each
    /// other, with a straight wire of length one and no bend, ranks before it. Neither is taken
    /// for a drawing that could not be measured.
    #[test]
    fn a_default_auto_wire_that_turns_back_counts_its_bends_and_its_length() {
        use crate::resolve::tests::{instance, problem_of};
        use crate::{SolverOptions, Work};
        use rnl_runtime::geom::Orient;
        use rnl_runtime::problem::{PointRef, ProblemWire, Reference};
        let mut p = problem_of(vec![instance(), instance()], vec![]);
        p.version = "0.4".into();
        p.wires.push(ProblemWire {
            label: "w".into(),
            reference: Reference::Anonymous,
            nets: vec!["n".into()],
            bus: false,
            anchors: vec![
                PointRef::Pin { object: 0, pin: 0, terminal: "g".into() },
                PointRef::Pin { object: 1, pin: 1, terminal: "d".into() },
            ],
            mode: "auto".into(),
            default_auto: true,
            generated: false,
        });
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
        let placed_at = |first: Orient, origin: [i64; 2]| {
            let mut geometry = s.witness.clone();
            geometry.orient[0] = first;
            geometry.origin = vec![origin.map(int), [int(-3), int(-3)]];
            Placed {
                origins: s.origins(&p, &geometry),
                geometry,
                dropped: vec![],
                overlaps: vec![],
                optimised: true,
                undecided: false,
            }
        };
        // the drain of the second faces up at (-3, -1); the gate of the first is one unit above it
        let down = Orient::ALL.into_iter().find(|&o| s.exit(0, 0, o) == Some((0, -1))).unwrap();
        let sideways = Orient::ALL.into_iter().find(|&o| s.exit(0, 0, o) == Some((-1, 0))).unwrap();
        let gate = |o: Orient| s.pin_offset(0, 0, o).unwrap();
        let origin_for = |o: Orient| {
            let off = gate(o);
            [int(-3) - &off[0], int(0) - &off[1]]
        };
        let whole = |q: &Q| crate::rational::to_f64(q) as i64;
        let turned = placed_at(sideways, origin_for(sideways).each_ref().map(whole));
        let facing = placed_at(down, origin_for(down).each_ref().map(whole));
        let routing = Routing::default();
        let back = lab_order(&p, &s, &turned, &routing, (1, 4), &int(1), None);
        let straight = lab_order(&p, &s, &facing, &routing, (1, 4), &int(1), None);
        assert!(!back.interrupted && !straight.interrupted);
        assert_eq!((back.crossings, back.bends, back.length.clone()), (0, 1, int(1)));
        assert_eq!((straight.crossings, straight.bends, straight.length.clone()), (0, 0, int(1)));
        assert!(straight < back);
        // the routing objectives are given the same paths
        let drawn = crate::score::drawn(&p, &s, &turned.geometry, &routing).expect("a wire that can be drawn");
        assert_eq!(drawn.paths.len(), 1);
        assert_eq!(drawn.paths[0].1.len(), 2);
        let drawn = crate::score::drawn(&p, &s, &facing.geometry, &routing).expect("a wire that can be drawn");
        assert_eq!(drawn.paths[0].1.len(), 2);
    }
}
