//! Exact quality ordering for completed line-layout candidates.
//! An interrupted measurement cannot rank ahead of a completed drawing.

use super::{axis, current, dots, reads, virtuals};
use crate::budget::{permit, Step};
use crate::coords::Placed;
use crate::rational::{int, Q};
use crate::route::Routing;
use crate::structure::Structure;
use crate::SolverOptions;
use num_traits::Zero;
use rnl_runtime::problem::LevelProblem;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LabOrder {
    pub(super) interrupted: bool,
    pub(super) annotation_cost: (usize, usize),
    pub(super) current: usize,
    pub(super) cb: u64,
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
    /// The rows and columns of the symbols, one only virtual symbols take counting
    /// `virtual_weight` ([`virtuals::weighed_lines`]).
    pub(super) crosses_weighed: Q,
    /// The layers of `lines_order`, each the weighted sum of its items ([`Ranking`]).
    pub(super) quality: Vec<Q>,
}

/// An item of the line-layout order after its rules, as `lines_order` names it
/// ([`crate::LINES_ITEMS`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Item {
    Current,
    CrossingsAndBends,
    MirrorPairs,
    DotsNearPins,
    Length,
    WireLines,
    SymbolLines,
    Area,
}

impl Item {
    /// The item `lines_order` names so.
    pub(crate) fn from_name(name: &str) -> Option<Item> {
        Some(match name {
            "current" => Item::Current,
            "crossings_and_bends" => Item::CrossingsAndBends,
            "mirror_pairs" => Item::MirrorPairs,
            "dots_near_pins" => Item::DotsNearPins,
            "length" => Item::Length,
            "wire_lines" => Item::WireLines,
            "symbol_lines" => Item::SymbolLines,
            "area" => Item::Area,
            _ => return None,
        })
    }
}

/// How the line layout ranks its drawings after the facts, the false joins, the annotations and
/// the configured rules (`lines_order`, `lines_weights`, `virtual_weight`): the layers of
/// `lines_order`, each the weighted sum of its items, compared exactly, the first first; an item
/// left out counts in none. The crossings and bends weigh crossing × crossings + bend × bends; a
/// row or column of the symbols that only virtual symbols take counts `virtual_weight`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Ranking {
    pub(crate) weights: (u64, u64),
    pub(crate) virtual_weight: Q,
    pub(crate) layers: Vec<Vec<(Item, u32)>>,
}

impl Ranking {
    pub(crate) fn of(options: &SolverOptions) -> Ranking {
        let layers = options
            .settings
            .lines_order
            .layers
            .iter()
            .map(|l| l.items.iter().filter_map(|(name, w)| Item::from_name(name).map(|i| (i, *w))).collect())
            .collect();
        Ranking {
            weights: options.settings.lines_weights.pair(),
            virtual_weight: options.settings.virtual_weight.q(),
            layers,
        }
    }

    fn takes(&self, item: Item) -> bool {
        self.layers.iter().any(|l| l.iter().any(|(i, _)| *i == item))
    }
}

impl Default for Ranking {
    fn default() -> Self {
        Ranking::of(&SolverOptions::default())
    }
}

impl LabOrder {
    #[allow(clippy::type_complexity)]
    fn key(&self) -> ((bool, (usize, usize)), &Vec<Q>, &Vec<Q>, (u64, u64)) {
        ((self.interrupted, self.annotation_cost), &self.rules, &self.quality, (self.crossings, self.bends))
    }

    /// The value of an item ([`Item`]).
    fn value(&self, item: Item) -> Q {
        let count = |n: usize| int(n as i64);
        match item {
            Item::Current => count(self.current),
            Item::CrossingsAndBends => int(self.cb as i64),
            Item::MirrorPairs => count(self.off_axis),
            Item::DotsNearPins => count(self.near_dots),
            Item::Length => self.length.clone(),
            Item::WireLines => count(self.lines),
            Item::SymbolLines => self.crosses_weighed.clone(),
            Item::Area => self.area.clone(),
        }
    }

    /// The layers of the unified key after the rules: those of `lines_order` ([`Ranking`]).
    pub(super) fn quality(&self) -> &[Q] {
        &self.quality
    }

    /// Its layers after the rules by `ranking`, from the counts measured.
    fn rank_by(&mut self, ranking: &Ranking) {
        self.quality = ranking
            .layers
            .iter()
            .map(|layer| layer.iter().map(|&(item, w)| self.value(item) * int(i64::from(w))).sum())
            .collect();
    }
}

impl PartialOrd for LabOrder {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LabOrder {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

impl LabOrder {
    fn interrupted(weights: (u64, u64)) -> Self {
        Self {
            interrupted: true,
            annotation_cost: (0, 0),
            current: 0,
            cb: 0,
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
            crosses_weighed: int(0),
            quality: Vec::new(),
        }
    }
    pub(super) fn said(&self) -> String {
        format!(
            "{}{} × {} + {} × {} = {}, {}{}length {}, {} lines of the wires, {} crosses of the symbols, area {}",
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
    ranking: &Ranking,
    within: Option<&BTreeSet<usize>>,
) -> LabOrder {
    let mut l = lab_measure(problem, s, placed, routing, ranking, within);
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
    ranking: &Ranking,
    within: Option<&BTreeSet<usize>>,
) -> LabOrder {
    use rnl_runtime::problem::PointRef;
    let weights = ranking.weights;
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
        if !wire.anchors.iter().all(|r| reads::reads_reference(r, &reads)) {
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
                    // An X, two diagonals of two wires that cross, as compaction draws two nets
                    // that swap: one crossing and two bends, as the corners of its diagonals
                    // count none. The two jogs it stands for would count four.
                    if *sa && *sb {
                        bends += 2;
                    }
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
    // below a weight of 1, each row and column by whether a symbol other than a virtual one takes it
    let virtuals = (ranking.virtual_weight != int(1) && ranking.takes(Item::SymbolLines))
        .then(|| virtuals::virtual_symbols(problem, s));
    let mut firm: (std::collections::BTreeMap<Q, bool>, std::collections::BTreeMap<Q, bool>) = Default::default();
    for k in s.components().filter(|&k| reads(k)) {
        let o = &placed.origins[k];
        crosses.0.insert(o[0].clone());
        crosses.1.insert(o[1].clone());
        if let Some(virtuals) = &virtuals {
            let f = !virtuals.contains(&k);
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
    let crosses_weighed = match &virtuals {
        None => int((crosses.0.len() + crosses.1.len()) as i64),
        Some(_) => {
            virtuals::weighed_lines(&firm.0, &ranking.virtual_weight)
                + virtuals::weighed_lines(&firm.1, &ranking.virtual_weight)
        }
    };
    let mut result = LabOrder {
        interrupted: false,
        annotation_cost,
        current: current::against(s, placed, within),
        cb: weights.0 * crossings + weights.1 * bends,
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
        crosses_weighed,
        quality: Vec::new(),
    };
    result.rank_by(ranking);
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

    fn ranking(text: &str) -> Ranking {
        Ranking::of(&SolverOptions::default().with_overrides(text).unwrap())
    }

    /// A drawing measured with each count 0 but one, `item`, at `v`, ranked by `ranking`.
    fn drawing(item: Item, v: i64, ranking: &Ranking) -> LabOrder {
        let mut l = LabOrder::interrupted((1, 4));
        l.interrupted = false;
        let n = v as usize;
        match item {
            Item::Current => l.current = n,
            Item::CrossingsAndBends => l.cb = v as u64,
            Item::MirrorPairs => l.off_axis = n,
            Item::DotsNearPins => l.near_dots = n,
            Item::Length => l.length = int(v),
            Item::WireLines => l.lines = n,
            Item::SymbolLines => l.crosses_weighed = int(v),
            Item::Area => l.area = int(v),
        }
        l.rank_by(ranking);
        l
    }

    /// Each item ranks while `lines_order` takes it and not once it is left out; `none` ranks by
    /// none of them. By default the mirror pairs and the dots near pins do not rank.
    #[test]
    fn each_item_ranks_only_while_lines_order_takes_it() {
        let d = Ranking::default();
        assert_eq!(d.layers.len(), 5);
        let all = ranking(&format!("lines_order=\"{}\"", crate::LINES_ITEMS.join(",")));
        for name in crate::LINES_ITEMS {
            let item = Item::from_name(name).unwrap();
            assert!(drawing(item, 1, &all) > drawing(item, 0, &all), "{name}");
            let off = [Item::MirrorPairs, Item::DotsNearPins];
            assert_eq!(d.takes(item), !off.contains(&item), "{name}");
            let alone = ranking(&format!("lines_order=\"{name}\""));
            assert_eq!(drawing(item, 1, &alone).quality(), [int(1)], "{name}");
            let others: Vec<&str> = crate::LINES_ITEMS.into_iter().filter(|n| *n != name).collect();
            let without = ranking(&format!("lines_order=\"{}\"", others.join(",")));
            assert_eq!(
                drawing(item, 1, &without).cmp(&drawing(item, 0, &without)),
                std::cmp::Ordering::Equal,
                "{name}"
            );
        }
        let none = ranking("lines_order=\"none\"");
        assert!(drawing(Item::CrossingsAndBends, 9, &none).quality().is_empty());
    }

    /// The layers rank in the order `lines_order` gives: a mirror pair off its axis before dots near
    /// pins in one order, after them when the order is turned.
    #[test]
    fn the_layers_rank_in_the_order_given() {
        let both = |r: &Ranking| {
            let mut a = drawing(Item::MirrorPairs, 1, r);
            a.near_dots = 0;
            let mut b = drawing(Item::DotsNearPins, 3, r);
            b.off_axis = 0;
            a.rank_by(r);
            b.rank_by(r);
            (a, b)
        };
        let (a, b) = both(&ranking("lines_order=\"mirror_pairs,dots_near_pins\""));
        assert!(a > b);
        let (a, b) = both(&ranking("lines_order=\"dots_near_pins,mirror_pairs\""));
        assert!(a < b);
    }

    /// Alignment is one layer by default: a line of the wires counts two, a row or column of the
    /// symbols one; the weights of the layer set how many of one outweigh the other.
    #[test]
    fn the_lines_of_the_wires_and_of_the_symbols_weigh_in_one_layer() {
        let d = Ranking::default();
        assert_eq!(d.layers[2], [(Item::SymbolLines, 1), (Item::WireLines, 2)]);
        let mixed = |wires: i64, symbols: i64, r: &Ranking| {
            let mut l = drawing(Item::WireLines, wires, r);
            l.crosses_weighed = int(symbols);
            l.rank_by(r);
            l
        };
        // one line of the wires against one row of the symbols: the wires weigh more; against two, alike
        assert!(mixed(1, 0, &d) > mixed(0, 1, &d));
        assert_eq!(mixed(1, 0, &d).quality(), mixed(0, 2, &d).quality());
        let alike = ranking("lines_order=\"symbol_lines+wire_lines\"");
        assert_eq!(mixed(1, 0, &alike).quality(), mixed(0, 1, &alike).quality());
        let symbols_first = ranking("lines_order=\"symbol_lines*3+wire_lines\"");
        assert!(mixed(1, 0, &symbols_first) < mixed(0, 1, &symbols_first));
    }

    /// A row or column of the symbols that only a virtual symbol takes counts `virtual_weight`:
    /// the fixture's first object, made the object of a usage, takes a column of its own.
    #[test]
    fn a_line_only_virtual_symbols_take_counts_the_virtual_weight() {
        let (mut p, s, placed, routing) = reads::tests::fixture();
        p.objects[0].kind = rnl_runtime::problem::ObjectKind::Usage;
        let at = |w: &str| lab_order(&p, &s, &placed, &routing, &ranking(&format!("virtual_weight={w}")), None);
        let (half, one, none) = (at("0.5"), at("1"), at("0"));
        assert_eq!(half.crosses, one.crosses);
        assert_eq!(one.crosses_weighed, int(one.crosses as i64));
        assert_eq!(&one.crosses_weighed - &half.crosses_weighed, int(1) / int(2));
        assert_eq!(&one.crosses_weighed - &none.crosses_weighed, int(1));
        // the alignment layer: the rows and columns of the symbols weighed, and the lines of the wires twice
        assert_eq!(half.quality()[2], &half.crosses_weighed + int(2 * half.lines as i64));
        // an instance's row is not weighed: with no symbol_lines in the order nothing is weighed
        let off = lab_order(&p, &s, &placed, &routing, &ranking("lines_order=\"length\""), None);
        assert_eq!(off.crosses_weighed, int(off.crosses as i64));
    }

    /// Two nets that swap drawn as an X, two diagonals that cross, count one crossing and two
    /// bends; drawn as two jogs that cross once, one crossing and four bends.
    #[test]
    fn an_x_counts_one_crossing_and_two_bends() {
        use rnl_runtime::solution::{RoutedWire, WirePoint};
        let (p, s, placed, routing) = port::tests::fixture();
        let with = |paths: [[[i64; 2]; 4]; 2]| {
            let mut r = routing.clone();
            for (route, path) in paths.into_iter().enumerate() {
                let points = path.iter().map(|xy| WirePoint::Bend((xy[0] as f64, xy[1] as f64))).collect();
                r.wires.push(RoutedWire { route, points });
                r.paths.push(path.iter().map(|xy| xy.map(int)).collect());
                r.drawn.push(r.wires.len() - 1);
            }
            lab_order(&p, &s, &placed, &r, &Ranking::default(), None)
        };
        let base = lab_order(&p, &s, &placed, &routing, &Ranking::default(), None);
        let x = with([[[20, 2], [22, 2], [24, 0], [26, 0]], [[20, 0], [22, 0], [24, 2], [26, 2]]]);
        let jogs = with([[[20, 2], [23, 2], [23, 0], [26, 0]], [[20, -1], [24, -1], [24, 3], [26, 3]]]);
        assert_eq!((x.crossings - base.crossings, x.bends - base.bends), (1, 2));
        assert_eq!((jogs.crossings - base.crossings, jogs.bends - base.bends), (1, 4));
        assert!(x < jogs);
    }

    #[test]
    fn interrupted_measurements_never_beat_completed_drawings_and_replay_exactly() {
        let (p, s, placed, routing) = reads::tests::fixture();
        let run = || lab_order(&p, &s, &placed, &routing, &Ranking::default(), None);
        let full = run();
        assert!(!full.interrupted);
        assert_eq!(full.cb, full.crossings + 4 * full.bends);
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
            lab_order(&p, &s, &placed, &routing, &Ranking::default(), None)
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
        let back = lab_order(&p, &s, &turned, &routing, &Ranking::default(), None);
        let straight = lab_order(&p, &s, &facing, &routing, &Ranking::default(), None);
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
