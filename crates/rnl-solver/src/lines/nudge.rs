//! A last pass over the drawing the line layout takes: a wire that still bends or crosses after the
//! strict merge may run straight if an element at one of its ends moves along one axis to the line
//! of the wire's other end. The merge itself never makes such a move where the element would pass
//! another symbol or another net's wire, as it keeps the drawing's topology. Here the element moves,
//! with the elements tied to it by a straight wire of two pins across the move, the nets the move
//! touches are routed again, the others kept as they are drawn, and the drawing is merged again. A
//! move is taken only when the one key of the line layout (`super::Rank`) ranks the drawing strictly
//! before the one it had, and never where a fact kept that held breaks.
//!
//! The setting `nudge_wires`, on by default.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use num_traits::Zero;
use rnl_runtime::problem::{Axis, Filler, LevelProblem, PointRef, Position};
use rnl_runtime::solution::WirePoint;

use super::cost::{Step, Tally};
use super::{settle, track};
use crate::budget::{permit, Step as BudgetStep};
use crate::coords::Placed;
use crate::rational::{from_f64, int, Q};
use crate::resolve::{point_value, Geometry};
use crate::route::Routing;
use crate::skeleton::{compact_strict, Strict};
use crate::structure::Structure;
use crate::{SolverOptions, Work};

/// The moves taken at most, each the best of a round.
const ROUNDS: usize = 3;
/// The moves of a round routed and merged at most, after the checks that need neither.
const TRIES: usize = 8;
/// The elements that move together at most.
const UNIT: usize = 4;
/// The routings of a move at most: the second with the sides kept clear that the first came too
/// near.
const ROUTINGS: usize = 2;
/// The gaps the merge keeps between two symbols, a symbol and a group it is not in
/// (`skeleton::pass`).
const SYMBOL_GAP: i64 = 2;
const GROUP_GAP: i64 = 1;
/// The gap the merge keeps between a wire and a symbol.
const WIRE_GAP: i64 = 1;

/// The drawing with the moves taken: merged, and routed as the line router gives it; and its
/// line-layout order.
pub(super) struct Nudged {
    pub made: Strict,
    pub routed: track::Routed,
    pub mine: super::quality::LabOrder,
}

/// The elements moved, the axis, by how much, and the wire they move for, by its index in the
/// drawing's routing; where that wire's other end is and the object it is a pin of, if any.
#[derive(Clone, Debug)]
struct Move {
    unit: Vec<usize>,
    axis: usize,
    by: Q,
    wire: usize,
    far: [Q; 2],
    far_owner: Option<usize>,
    at: [Q; 2],
}

/// The pass. It moves and routes on the drawing before its merge (`pre`), where the wires keep the
/// room the merge takes from them, and merges each move once, after all its routing. None when no
/// move is taken, or when the budget stops it, which `super::engine` does again past the limit.
#[allow(clippy::too_many_arguments)]
pub(super) fn nudge(
    problem: &LevelProblem,
    s: &Structure,
    options: &SolverOptions,
    made: &Strict,
    pre: &(Placed, Routing),
    clear: &BTreeMap<usize, u8>,
    work: &mut Work,
    notes: &mut Vec<String>,
) -> Option<Nudged> {
    let rank = super::Rank::new(s, options);
    let ranking = super::quality::Ranking::of(options);
    let key = |placed: &Placed, routing: &Routing| -> Option<Vec<Q>> {
        if routing.written_path_error.is_some() || routing.annotation_error.is_some() {
            return None;
        }
        crate::annotation::bounds(problem, &placed.geometry).ok()?;
        let wires = crate::score::drawn(problem, s, &placed.geometry, routing).ok()?;
        let objectives = crate::score::measured_layers(problem, s, placed, Some(&wires));
        Some(rank.entry(problem, s, options, placed, routing, objectives, &[]).layers)
    };
    let mut current = made.clone();
    let mut before = pre.clone();
    let mut now = key(&current.placed, &current.routing)?;
    if crate::budget::stopped() {
        return None;
    }
    let first = super::quality::lab_order(problem, s, &current.placed, &current.routing, &ranking, None);
    let mut routed: Option<track::Routed> = None;
    let mut tally = Tally::new(problem.objects.len());
    let (mut considered, mut tried) = (0usize, 0usize);
    let mut alone = (0usize, 0usize);
    let mut said = Vec::new();
    for _ in 0..ROUNDS {
        let bent = bent(s, &before.0, &before.1, ranking.weights)?;
        let moves = moves(problem, s, &before.0, &before.1, &bent);
        let held = holding(problem, s, options, &before.0.geometry)?;
        let mut best: Option<(Vec<Q>, Moved, Move)> = None;
        let mut evaluated = 0;
        for m in moves {
            if evaluated == TRIES {
                break;
            }
            considered += 1;
            let Some(g) = fits(problem, s, options, &before.0, &m, &held) else {
                if crate::budget::stopped() {
                    return None;
                }
                continue;
            };
            evaluated += 1;
            tried += 1;
            let crossed = &bent[m.wire].1;
            let next =
                again(problem, s, options, &before, &current.levels, g, &m, crossed, clear, &key, &mut tally, work);
            if crate::budget::stopped() {
                return None;
            }
            let Some((strict, r, pre, (routed_alone, kept_alone))) = next else {
                continue;
            };
            alone = (alone.0 + routed_alone, alone.1 + kept_alone);
            let Some(k) = key(&strict.placed, &strict.routing) else { continue };
            if crate::budget::stopped() {
                return None;
            }
            if k < now && best.as_ref().is_none_or(|b| k < b.0) {
                best = Some((k, (strict, r, pre, (routed_alone, kept_alone)), m));
            }
        }
        let Some((k, (strict, r, pre, _), m)) = best else { break };
        let was = super::quality::lab_order(problem, s, &current.placed, &current.routing, &ranking, None);
        let is = super::quality::lab_order(problem, s, &strict.placed, &strict.routing, &ranking, None);
        said.push(format!(
            "lines: {} moved {} by {}, for a wire of {}: {} crossings and {} bends, {} weighed, before {} and {}, {}",
            m.unit.iter().map(|&k| problem.objects[k].label.as_str()).collect::<Vec<_>>().join(", "),
            match (m.axis, m.by > Q::zero()) {
                (0, true) => "right",
                (0, false) => "left",
                (_, true) => "up",
                (_, false) => "down",
            },
            crate::rational::fmt(&m.by.abs()),
            problem.routes.get(before.1.wires[m.wire].route).map_or("", |r| r.net.as_str()),
            is.crossings,
            is.bends,
            is.cb,
            was.crossings,
            was.bends,
            was.cb
        ));
        (now, current, routed, before) = (k, strict, Some(r), pre);
    }
    if crate::budget::stopped() {
        return None;
    }
    notes.push(format!(
        "lines: the ends of the wires that bend or cross: {} moved, {} of {} moves routed and merged again; of the \
         nets they routed again that cross a wire, {} of {} routed once more alone kept",
        said.len(),
        tried,
        considered,
        alone.1,
        alone.0
    ));
    let routed = routed?;
    notes.extend(said);
    let last = super::quality::lab_order(problem, s, &current.placed, &current.routing, &ranking, None);
    notes.push(format!(
        "lines: with the ends moved, by the line-layout order, the drawing made {} against {} before",
        last.said(),
        first.said()
    ));
    Some(Nudged { made: current, routed, mine: last })
}

/// The object and pin a wire's point is the pin of.
fn pin_of(p: &WirePoint) -> Option<(usize, usize)> {
    match p {
        WirePoint::Anchor(PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. }) => {
            Some((*object, *pin))
        }
        _ => None,
    }
}

/// Whether an object can move: one with a position of its own that the solution places.
fn own(problem: &LevelProblem, s: &Structure, k: usize) -> bool {
    matches!(problem.objects[k].position, Position::Own { .. }) && !s.objects[k].left_free
}

/// The own points of the usages without a symbol that can move, each with the objects the usage
/// is made of, through the usages that fill its roles: such a point moves with them when they all
/// move, as a usage's facts tie it to its members (`{?members ?} @ALIGN` of a branch).
fn usage_points(problem: &LevelProblem, s: &Structure) -> Vec<(usize, BTreeSet<usize>)> {
    let semantic: BTreeMap<usize, usize> = problem
        .usages
        .iter()
        .enumerate()
        .filter_map(|(u, usage)| usage.object.filter(|&k| problem.objects[k].symbol.is_none()).map(|k| (k, u)))
        .collect();
    fn members(
        problem: &LevelProblem,
        semantic: &BTreeMap<usize, usize>,
        u: usize,
        depth: usize,
        out: &mut BTreeSet<usize>,
    ) {
        if depth > 16 {
            return;
        }
        for role in &problem.usages[u].roles {
            for m in &role.members {
                match m {
                    Filler::Object(k) => match semantic.get(k) {
                        Some(&v) => members(problem, semantic, v, depth + 1, out),
                        None => {
                            out.insert(*k);
                        }
                    },
                    Filler::Terminal { object, .. } | Filler::Pin { object, .. } => {
                        out.insert(*object);
                    }
                    Filler::Usage(v) if *v < problem.usages.len() => members(problem, semantic, *v, depth + 1, out),
                    _ => {}
                }
            }
        }
    }
    semantic
        .iter()
        .filter(|(&k, _)| own(problem, s, k))
        .map(|(&k, &u)| {
            let mut out = BTreeSet::new();
            members(problem, &semantic, u, 0, &mut out);
            out.retain(|&j| j != k && own(problem, s, j));
            (k, out)
        })
        .filter(|(_, out)| !out.is_empty())
        .collect()
}

/// The points of a path, each once in a row.
fn points(path: &[[Q; 2]]) -> Vec<[Q; 2]> {
    let mut out: Vec<[Q; 2]> = Vec::new();
    for q in path {
        if out.last() != Some(q) {
            out.push(q.clone());
        }
    }
    out
}

fn sign(v: &Q) -> i64 {
    if v.is_zero() {
        0
    } else if *v > Q::zero() {
        1
    } else {
        -1
    }
}

/// Each routed wire's crossings and bends weighed, as the line layout's order counts them
/// (`super::quality::lab_measure`): a turn between two runs along the axes a bend, an end that
/// leaves its pin another way than the pin's a bend, a crossing of another wire one, an X one
/// crossing and a bend of each of its two wires; and the wires it crosses. None when the budget
/// stops it.
fn bent(s: &Structure, placed: &Placed, routing: &Routing, weights: (u64, u64)) -> Option<Vec<(u64, BTreeSet<usize>)>> {
    let g = &placed.geometry;
    let n = routing.wires.len();
    let pts: Vec<Vec<[Q; 2]>> = routing.paths.iter().map(|p| points(p)).collect();
    let mut bends = vec![0u64; n];
    let mut crossings = vec![0u64; n];
    let mut crossed: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    let straight = |w: (i64, i64)| (w.0 == 0) != (w.1 == 0);
    let way = |a: &[Q; 2], b: &[Q; 2]| (sign(&(&b[0] - &a[0])), sign(&(&b[1] - &a[1])));
    // the segments: the wire, its ends, whether it runs along neither axis
    let mut segs: Vec<(usize, [Q; 2], [Q; 2], bool)> = Vec::new();
    for (i, p) in pts.iter().enumerate() {
        if !permit(BudgetStep::Structure, p.len()) {
            return None;
        }
        for k in 1..p.len().saturating_sub(1) {
            let (u, v) = (way(&p[k - 1], &p[k]), way(&p[k], &p[k + 1]));
            if u != v && straight(u) && straight(v) {
                bends[i] += 1;
            }
        }
        if p.len() < 2 {
            continue;
        }
        let w = &routing.wires[i];
        let m = p.len();
        for (point, end, next) in [(w.points.first(), &p[0], &p[1]), (w.points.last(), &p[m - 1], &p[m - 2])] {
            let Some((k, pin)) = point.and_then(pin_of) else { continue };
            if s.exit(k, pin, g.orient[k]).is_some_and(|e| e != way(end, next)) {
                bends[i] += 1;
            }
        }
        for pair in p.windows(2) {
            let along = pair[0][0] == pair[1][0] || pair[0][1] == pair[1][1];
            segs.push((i, pair[0].clone(), pair[1].clone(), !along));
        }
    }
    let side = |a: &[Q; 2], b: &[Q; 2], c: &[Q; 2]| {
        sign(&((&b[0] - &a[0]) * (&c[1] - &a[1]) - (&b[1] - &a[1]) * (&c[0] - &a[0])))
    };
    let through = |h0: &[Q; 2], h1: &[Q; 2], v0: &[Q; 2], v1: &[Q; 2]| {
        h0[1] == h1[1]
            && v0[0] == v1[0]
            && h0[0].clone().min(h1[0].clone()) < v0[0]
            && v0[0] < h0[0].clone().max(h1[0].clone())
            && v0[1].clone().min(v1[1].clone()) < h0[1]
            && h0[1] < v0[1].clone().max(v1[1].clone())
    };
    for (a, (wa, a0, a1, da)) in segs.iter().enumerate() {
        if !permit(BudgetStep::Structure, segs.len() - a) {
            return None;
        }
        for (wb, b0, b1, db) in &segs[a + 1..] {
            if wa == wb {
                continue;
            }
            let crosses = if *da || *db {
                side(a0, a1, b0) * side(a0, a1, b1) < 0 && side(b0, b1, a0) * side(b0, b1, a1) < 0
            } else {
                through(a0, a1, b0, b1) || through(b0, b1, a0, a1)
            };
            if !crosses {
                continue;
            }
            for (w, other) in [(*wa, *wb), (*wb, *wa)] {
                crossings[w] += 1;
                crossed[w].insert(other);
                if *da && *db {
                    bends[w] += 1;
                }
            }
        }
    }
    Some((0..n).map(|i| (weights.0 * crossings[i] + weights.1 * bends[i], std::mem::take(&mut crossed[i]))).collect())
}

/// The moves for the wires that bend or cross, those of the most weighed first: each end of such a
/// wire that is the pin of an element that can move, moved along an axis the pin does not leave by,
/// to the line of the wire's other end on that axis, where the wire could run straight from it.
fn moves(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    bent: &[(u64, BTreeSet<usize>)],
) -> Vec<Move> {
    let g = &placed.geometry;
    let components: BTreeSet<usize> = s.components().collect();
    let movable = |k: usize| components.contains(&k) && own(problem, s, k);
    let mut order: Vec<usize> = (0..bent.len()).filter(|&i| bent[i].0 > 0).collect();
    order.sort_by_key(|&i| (Reverse(bent[i].0), i));
    let points = usage_points(problem, s);
    let mut out = Vec::new();
    let mut seen: BTreeSet<(Vec<usize>, usize, Q)> = BTreeSet::new();
    for i in order {
        let w = &routing.wires[i];
        let path = &routing.paths[i];
        let m = path.len();
        if m < 2 || w.points.len() != m {
            continue;
        }
        for (end, other) in [(0, m - 1), (m - 1, 0)] {
            let Some((k, pin)) = pin_of(&w.points[end]) else { continue };
            if !movable(k) {
                continue;
            }
            let far_pin = pin_of(&w.points[other]);
            let (p, r) = (&path[end], &path[other]);
            for axis in 0..2 {
                let by = &r[axis] - &p[axis];
                if by.is_zero() {
                    continue;
                }
                // the wire, straight, runs along the other axis: its pins must leave along it
                let across = |e: (i64, i64)| if axis == 0 { e.0 != 0 } else { e.1 != 0 };
                if s.exit(k, pin, g.orient[k]).is_some_and(across) {
                    continue;
                }
                if far_pin.and_then(|(j, q)| s.exit(j, q, g.orient[j])).is_some_and(across) {
                    continue;
                }
                let Some(unit) = unit_of(problem, s, placed, routing, k, axis, i, &movable, &points) else { continue };
                if !seen.insert((unit.clone(), axis, by.clone())) {
                    continue;
                }
                out.push(Move {
                    unit,
                    axis,
                    by,
                    wire: i,
                    far: r.clone(),
                    far_owner: far_pin.map(|(j, _)| j),
                    at: p.clone(),
                });
            }
        }
    }
    out
}

/// The elements that move with `k` along `axis`, `k` among them: those joined to one of them by a
/// wire, other than the wire moved for, that is the only one of its net, has a pin at each end and
/// runs straight across the move, so that it stays straight; and the objects with a pin where one
/// of theirs is; and the own point of a usage all of whose objects move ([`usage_points`]). None when
/// one of them cannot move, or more than [`UNIT`] elements would.
#[allow(clippy::too_many_arguments)]
fn unit_of(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    k: usize,
    axis: usize,
    target: usize,
    movable: &dyn Fn(usize) -> bool,
    points: &[(usize, BTreeSet<usize>)],
) -> Option<Vec<usize>> {
    let g = &placed.geometry;
    let pins_at = |u: usize| -> Vec<[Q; 2]> {
        (0..s.pins(u).len()).filter_map(|p| s.pin_at(&placed.origins, &g.orient, u, p)).collect()
    };
    let mut unit = vec![k];
    let mut at = 0;
    while at < unit.len() {
        let u = unit[at];
        at += 1;
        for (w, wire) in routing.wires.iter().enumerate() {
            let n = wire.points.len();
            if w == target || n < 2 {
                continue;
            }
            let (Some(a), Some(b)) = (pin_of(&wire.points[0]), pin_of(&wire.points[n - 1])) else { continue };
            let v = if a.0 == u && b.0 != u {
                b.0
            } else if b.0 == u && a.0 != u {
                a.0
            } else {
                continue;
            };
            if unit.contains(&v) || routing.wires.iter().filter(|x| x.route == wire.route).count() != 1 {
                continue;
            }
            let path = &routing.paths[w];
            if !path.iter().all(|q| q[axis] == path[0][axis]) {
                continue;
            }
            if !movable(v) {
                return None;
            }
            unit.push(v);
        }
        let here = pins_at(u);
        for j in 0..problem.objects.len() {
            if unit.contains(&j) || !own(problem, s, j) {
                continue;
            }
            if pins_at(j).iter().any(|q| here.contains(q)) {
                unit.push(j);
            }
        }
        if unit.iter().filter(|&&j| movable(j)).count() > UNIT {
            return None;
        }
        if at == unit.len() {
            for (point, members) in points {
                if !unit.contains(point) && members.iter().all(|j| unit.contains(j)) {
                    unit.push(*point);
                }
            }
        }
    }
    unit.sort_unstable();
    Some(unit)
}

/// Whether each fact kept holds in a geometry, by its index in `s.kept`. None when the budget stops
/// it.
fn holding(problem: &LevelProblem, s: &Structure, options: &SolverOptions, g: &Geometry) -> Option<Vec<bool>> {
    if !permit(BudgetStep::Structure, s.kept.len()) {
        return None;
    }
    let tol = from_f64(options.tolerance).unwrap_or_else(Q::zero);
    let at = |r: &PointRef, axis: Axis| point_value(problem, g, r, usize::from(axis == Axis::Y), 0);
    let ori = |k: usize| g.orient.get(k).copied();
    Some(s.kept.iter().map(|&f| crate::holds::check_problem(problem, f, &at, &ori, &tol, &tol) == Some(true)).collect())
}

/// How far apart two boxes are: by the larger of the gaps along the axes, less than 0 where they
/// overlap.
fn apart(a: &[Q; 4], b: &[Q; 4]) -> Q {
    let x = (&b[0] - &a[2]).max(&a[0] - &b[2]);
    let y = (&b[1] - &a[3]).max(&a[1] - &b[3]);
    x.max(y)
}

fn union(boxes: impl Iterator<Item = [Q; 4]>) -> Option<[Q; 4]> {
    boxes.reduce(|a, b| {
        [
            a[0].clone().min(b[0].clone()),
            a[1].clone().min(b[1].clone()),
            a[2].clone().max(b[2].clone()),
            a[3].clone().max(b[3].clone()),
        ]
    })
}

/// The boxes of a drawing that its symbols and texts take, each with the object it goes with and
/// whether it is a symbol's.
fn shapes(
    problem: &LevelProblem,
    s: &Structure,
    g: &Geometry,
    origins: &[[Q; 2]],
) -> Option<Vec<(usize, [Q; 4], bool)>> {
    let mut out = Vec::new();
    for k in s.components() {
        if let Some(b) = s.box_at(origins, &g.orient, k) {
            out.push((k, b, true));
        }
    }
    for (k, at) in origins.iter().enumerate().take(problem.objects.len()) {
        if s.objects[k].left_free {
            continue;
        }
        for b in s.text_offsets(problem, g, k) {
            out.push((k, [&b[0] + &at[0], &b[1] + &at[1], &b[2] + &at[0], &b[3] + &at[1]], false));
        }
    }
    for (object, b) in crate::annotation::bounds(problem, g).ok()? {
        out.push((crate::annotation::attachment_owner(problem, object).unwrap_or(object), b, false));
    }
    Some(out)
}

/// The geometry of a move, when it passes the checks that need no routing: no symbol or text it
/// moves comes nearer another than the merge keeps them, or than they were; no symbol comes into
/// the box of a group it is not in, nor a group's box into another's; every fact kept that held
/// still holds (`held`, by [`holding`]); and no symbol stands where the wire would run straight.
fn fits(
    problem: &LevelProblem,
    s: &Structure,
    options: &SolverOptions,
    placed: &Placed,
    m: &Move,
    held: &[bool],
) -> Option<Geometry> {
    let old = &placed.geometry;
    let mut g = old.clone();
    for &k in &m.unit {
        g.origin[k][m.axis] = &g.origin[k][m.axis] + &m.by;
    }
    let origins = s.origins(problem, &g);
    let (before, after) = (shapes(problem, s, old, &placed.origins)?, shapes(problem, s, &g, &origins)?);
    if before.len() != after.len() {
        return None;
    }
    let moved: Vec<usize> = (0..after.len()).filter(|&i| before[i].1 != after[i].1).collect();
    for &i in &moved {
        if !permit(BudgetStep::Structure, after.len()) {
            return None;
        }
        for j in 0..after.len() {
            if before[j].1 != after[j].1 || after[i].0 == after[j].0 {
                continue;
            }
            let gap = int(if after[i].2 && after[j].2 { SYMBOL_GAP } else { 0 });
            let was = apart(&before[i].1, &before[j].1);
            if apart(&after[i].1, &after[j].1) < was.min(gap) {
                return None;
            }
        }
    }
    // the groups' boxes, of the boxes of their members' symbols
    let boxes = |g: &Geometry, origins: &[[Q; 2]]| -> Vec<Option<[Q; 4]>> {
        problem
            .groups
            .iter()
            .map(|group| union(group.members.iter().filter_map(|&k| s.box_at(origins, &g.orient, k))))
            .collect()
    };
    let (was, now) = (boxes(old, &placed.origins), boxes(&g, &origins));
    let gap = int(GROUP_GAP);
    let changed: BTreeSet<usize> = m.unit.iter().copied().collect();
    for (h, group) in problem.groups.iter().enumerate() {
        let (Some(bw), Some(bn)) = (&was[h], &now[h]) else { continue };
        if !permit(BudgetStep::Structure, problem.objects.len() + problem.groups.len()) {
            return None;
        }
        for k in s.components() {
            if group.members.contains(&k) || (bw == bn && !changed.contains(&k)) {
                continue;
            }
            let (Some(kw), Some(kn)) = (s.box_at(&placed.origins, &old.orient, k), s.box_at(&origins, &g.orient, k))
            else {
                continue;
            };
            if apart(bn, &kn) < apart(bw, &kw).min(gap.clone()) {
                return None;
            }
        }
        for (i, other) in problem.groups.iter().enumerate() {
            let (Some(ow), Some(on)) = (&was[i], &now[i]) else { continue };
            let nested = |a: &Vec<usize>, b: &Vec<usize>| a.iter().all(|k| b.contains(k));
            if i == h
                || (bw == bn && ow == on)
                || nested(&group.members, &other.members)
                || nested(&other.members, &group.members)
            {
                continue;
            }
            if apart(bn, on) < apart(bw, ow).min(gap.clone()) {
                return None;
            }
        }
    }
    let holds = holding(problem, s, options, &g)?;
    if held.iter().zip(&holds).any(|(was, is)| *was && !is) {
        return None;
    }
    // the straight run from the pin moved to the wire's other end
    let o = 1 - m.axis;
    let t = &m.far[m.axis];
    let (lo, hi) = (m.at[o].clone().min(m.far[o].clone()), m.at[o].clone().max(m.far[o].clone()));
    for k in s.components() {
        if m.unit.contains(&k) || m.far_owner == Some(k) {
            continue;
        }
        let Some(b) = s.box_at(&origins, &g.orient, k) else { continue };
        if b[m.axis] < *t && *t < b[m.axis + 2] && b[o] < hi && lo < b[o + 2] {
            return None;
        }
    }
    Some(g)
}

/// The layout the line router reads of a drawing: every object, the preferred candidate of each
/// route the solver completes, as the line search gives them.
fn layout(problem: &LevelProblem, s: &Structure, g: &Geometry, origins: &[[Q; 2]]) -> Option<Layout> {
    let objects: BTreeSet<usize> = (0..problem.objects.len()).collect();
    let (mut layout, pins, written_net, refs) = settle::layout_of(problem, s, g, origins, &objects, true).ok()?;
    for (i, route) in problem.routes.iter().enumerate() {
        if settle::complete_auto_route(route, &refs[i]) {
            layout.nets[i].preferred = crate::route::default_candidate(&layout, i);
        }
    }
    Some((layout, pins, written_net, refs))
}

/// What [`settle::layout_of`] gives: the layout, the pins, the net each written path is of, the
/// points each route's anchors are.
type Layout =
    (crate::route::Layout, Vec<(usize, usize, [Q; 2], Option<(i64, i64)>)>, Vec<Option<usize>>, Vec<Vec<PointRef>>);

/// A move's drawing: merged, as the router routed it, before its merge, and how many nets were
/// routed once more alone and how many of those kept.
type Moved = (Strict, track::Routed, (Placed, Routing), (usize, usize));

/// A move routed and merged. On the drawing before its merge (`pre`), moved as `g` has it, the nets
/// the move touches are routed again, with those whose wires now run within a wire's gap of what it
/// moved, those of the wires the wire it is for crosses, and those drawn with an X; the other nets'
/// wires kept as they are drawn there. Then each net routed again that crosses a wire is routed once
/// more on its own, every other wire kept: the router routes the nets one at a time, those of more
/// pins first, and a net routed before another does not see that one's wires. That routing is kept
/// where the drawing then ranks strictly before by the line layout's key (`key`), its crossings and
/// bends weighed, then its lines and its length. Only then is the drawing merged, once, so that the
/// merge does not take the room a wire routed after it would have run in. As the line layout's
/// rounds do (`super::engine`), a merge that leaves wires nearer a symbol than their gap is routed
/// once more with the sides of those symbols kept clear. With the drawing before the merge, and how
/// many nets were routed once more alone and how many kept. None when the routing or the merge
/// fails, or the merge leaves a wire nearer a symbol than its gap. Its work is charged as the line
/// search charges a candidate's.
#[allow(clippy::too_many_arguments)]
fn again(
    problem: &LevelProblem,
    s: &Structure,
    options: &SolverOptions,
    pre: &(Placed, Routing),
    levels: &[(crate::structure::PinKey, crate::structure::PinKey)],
    g: Geometry,
    m: &Move,
    crossed: &BTreeSet<usize>,
    clear: &BTreeMap<usize, u8>,
    key: &dyn Fn(&Placed, &Routing) -> Option<Vec<Q>>,
    tally: &mut Tally,
    work: &mut Work,
) -> Option<Moved> {
    let origins = s.origins(problem, &g);
    let read = layout(problem, s, &g, &origins)?;
    let was = &pre.0.origins;
    let mut nets: BTreeSet<usize> = BTreeSet::new();
    for (i, net) in read.0.nets.iter().enumerate() {
        if net.anchors.iter().any(|a| a.owner.is_some_and(|k| was.get(k) != origins.get(k))) {
            nets.insert(i);
        }
    }
    // the boxes of what moved, widened by a wire's gap
    let gap = int(WIRE_GAP);
    let mut taken: Vec<[Q; 4]> = Vec::new();
    for k in (0..problem.objects.len()).filter(|&k| was.get(k) != origins.get(k)) {
        if let Some(b) = s.box_at(&origins, &g.orient, k) {
            taken.push(b);
        }
        for b in s.text_offsets(problem, &g, k) {
            let at = &origins[k];
            taken.push([&b[0] + &at[0], &b[1] + &at[1], &b[2] + &at[0], &b[3] + &at[1]]);
        }
    }
    let widened: Vec<[Q; 4]> = taken.iter().map(|b| [&b[0] - &gap, &b[1] - &gap, &b[2] + &gap, &b[3] + &gap]).collect();
    let routing = &pre.1;
    if !permit(BudgetStep::Structure, routing.wires.len().saturating_mul(widened.len().max(1))) {
        return None;
    }
    for (w, (wire, path)) in routing.wires.iter().zip(&routing.paths).enumerate() {
        let runs_in = path.windows(2).any(|p| {
            let seg = [
                p[0][0].clone().min(p[1][0].clone()),
                p[0][1].clone().min(p[1][1].clone()),
                p[0][0].clone().max(p[1][0].clone()),
                p[0][1].clone().max(p[1][1].clone()),
            ];
            widened.iter().any(|b| seg[0] < b[2] && b[0] < seg[2] && seg[1] < b[3] && b[1] < seg[3])
        });
        if diagonal(path) || runs_in || crossed.contains(&w) || w == m.wire {
            nets.insert(wire.route);
        }
    }
    let placed = Placed {
        geometry: g,
        origins,
        dropped: pre.0.dropped.clone(),
        overlaps: Vec::new(),
        optimised: true,
        undecided: false,
    };
    let weights = options.settings.lines_weights.pair();
    let mut alone = (0usize, 0usize);
    let mut kept_clear = clear.clone();
    for _ in 0..ROUTINGS {
        let mut r = route(options, &read, kept(routing, |i| !nets.contains(&i)), &kept_clear, tally, work)?;
        let mut drawn = settle::routing_of(problem, &r, &read.3);
        let mut k = key(&placed, &drawn);
        for &i in &nets {
            let Some(bent) = bent(s, &placed, &drawn, weights) else { break };
            if !drawn.wires.iter().zip(&bent).any(|(w, b)| w.route == i && !b.1.is_empty()) {
                continue;
            }
            alone.0 += 1;
            let once = route(options, &read, kept(&drawn, |j| j != i), &kept_clear, tally, work);
            if crate::budget::stopped() {
                return None;
            }
            let Some(once) = once else { continue };
            let other = settle::routing_of(problem, &once, &read.3);
            let other_key = key(&placed, &other);
            if crate::budget::stopped() {
                return None;
            }
            if other_key.is_some() && (k.is_none() || other_key < k) {
                (r, drawn, k) = (once, other, other_key);
                alone.1 += 1;
            }
        }
        let relaxations = work.astar_expansions;
        let mut made = compact_strict(problem, s, &placed, &drawn, options, work, None, levels);
        super::written::defaults(problem, s, &made.placed, &mut made.routing, None, false);
        tally.charge(work, Step::Merge, 1);
        tally.charge(work, Step::MergePivotRow, made.pivot_rows);
        tally.charge(work, Step::MergeRelaxation, work.astar_expansions - relaxations);
        if made.failed || made.interrupted {
            return None;
        }
        if made.conflicts.is_empty() {
            return Some((made, r, (placed, drawn), alone));
        }
        for (&k, &sides) in &made.sides {
            *kept_clear.entry(k).or_default() |= sides;
        }
    }
    None
}

/// Whether a wire's path has a diagonal, as an X has.
fn diagonal(path: &[[Q; 2]]) -> bool {
    path.windows(2).any(|p| p[0][0] != p[1][0] && p[0][1] != p[1][1])
}

/// The wires of `routing` kept as they are drawn: those of the nets `keep` takes, but a net's with
/// a diagonal, which the router does not read.
fn kept(routing: &Routing, keep: impl Fn(usize) -> bool) -> Vec<(usize, Vec<[Q; 2]>)> {
    let x: BTreeSet<usize> =
        routing.wires.iter().zip(&routing.paths).filter(|(_, p)| diagonal(p)).map(|(w, _)| w.route).collect();
    routing
        .wires
        .iter()
        .zip(&routing.paths)
        .filter(|(w, p)| keep(w.route) && !x.contains(&w.route) && p.len() >= 2)
        .map(|(w, p)| (w.route, p.clone()))
        .collect()
}

/// The drawing read as `read`, routed by the line router, the wires of `fixed` kept, its work
/// charged. None when it fails, or the budget stops it.
fn route(
    options: &SolverOptions,
    read: &Layout,
    fixed: Vec<(usize, Vec<[Q; 2]>)>,
    kept_clear: &BTreeMap<usize, u8>,
    tally: &mut Tally,
    work: &mut Work,
) -> Option<track::Routed> {
    let (layout, pins, written_net, _) = read;
    let input = track::Input {
        layout,
        pins: pins.clone(),
        written_net: written_net.clone(),
        groups: Vec::new(),
        weights: options.settings.lines_weights.pair(),
        budget: settle::STEPS,
        full: [BTreeSet::new(), BTreeSet::new()],
        kept_clear: kept_clear.clone(),
        fixed,
        clip: None,
        lenient: false,
        reroute: options.settings.reroute_nets,
    };
    let routed = track::route(&input);
    tally.charge(work, Step::Route, 1);
    tally.charge(work, Step::Expansion, routed.expansions);
    (routed.failed.is_none() && !crate::budget::stopped()).then_some(routed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::Ticks;
    use crate::tree::{Laid, Stop};
    use crate::{RunLimit, StoppedClock};
    use rnl_runtime::solution::RoutedWire;

    /// A two-stage amplifier with its compensation as a horizontal branch, every net routed.
    const AMPLIFIER: &str = r#"* a two-stage amplifier
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4") }
.model nch nmos level=1
.model pch pmos level=1
M1 x inn t vss nch
M2 y inp t vss nch
M3 x x vdd vdd pch
M4 y x vdd vdd pch
M5 t b vss vss nch
M6 out y vdd vdd pch
M7 out b vss vss nch
M8 b b vss vss nch
IB vdd b 10u
RZ y z 1k
CC z out 1p
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   %nets = # @SELECT-NET;
*   %nets @ROUTE-NET;
*   %dp = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
*   %load = {#M3}(ref) {#M4}(out) @AS $CURRENT_MIRROR_P;
*   %bias = {#M8}(ref) {#M5 #M7}(out) @AS $CURRENT_MIRROR_N;
*   %comp = {#RZ #CC} @AS $BRANCH_HORIZONTAL;
*   %out = {#M6 #M7} @AS $BRANCH;
*   %ib = {#IB #M8} @AS $BRANCH;
*   {%ib %dp %comp %out} @ORDER("left-to-right");
* }
"#;

    fn level(text: &str) -> (LevelProblem, Structure, SolverOptions) {
        let files = rnl_runtime::source::MemoryProvider::new().with("t.sp", text);
        let p = rnl_view::interpret("t.sp", None, &files, &rnl_runtime::Options::solver())
            .unwrap()
            .problems
            .into_iter()
            .next()
            .unwrap();
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
        let s = crate::search_choices::for_layout(&s, &mut Work::default());
        (p, s, options)
    }

    /// The bends and crossings of each wire, as the line layout's order counts them: a jog two
    /// bends, a crossing one for each of the two wires, an X a crossing and a bend for each.
    #[test]
    fn each_wire_counts_its_bends_and_crossings_as_the_order_does() {
        let (_, s, _, start, _) = super::super::settle::tests::seed();
        let q = |x: i64, y: i64| [int(x), int(y)];
        let wire = |route: usize, path: Vec<[Q; 2]>| {
            let n = path.len();
            let mut points = vec![WirePoint::Branch(2 * route)];
            points.extend(
                path[1..n - 1]
                    .iter()
                    .map(|p| WirePoint::Bend((crate::rational::to_f64(&p[0]), crate::rational::to_f64(&p[1])))),
            );
            points.push(WirePoint::Branch(2 * route + 1));
            (RoutedWire { route, points }, path)
        };
        let wires = [
            wire(0, vec![q(0, 0), q(0, 4), q(4, 4), q(4, 8)]),
            wire(1, vec![q(-2, 2), q(6, 2)]),
            wire(2, vec![q(10, 0), q(12, 2)]),
            wire(3, vec![q(10, 2), q(12, 0)]),
        ];
        let routing = Routing {
            wires: wires.iter().map(|w| w.0.clone()).collect(),
            paths: wires.iter().map(|w| w.1.clone()).collect(),
            ..Routing::default()
        };
        let bent = bent(&s, &start.placed, &routing, (1, 4)).unwrap();
        let weighed: Vec<u64> = bent.iter().map(|b| b.0).collect();
        assert_eq!(weighed, [1 + 4 * 2, 1, 1 + 4, 1 + 4]);
        let crossed: Vec<Vec<usize>> = bent.iter().map(|b| b.1.iter().copied().collect()).collect();
        assert_eq!(crossed, [vec![1], vec![0], vec![3], vec![2]]);
    }

    /// With the pass on, as by default, a run of time cut in the line search, with the pass then run past the
    /// limit, replays alike by the work it reports; and at the last cut the pass routed at least
    /// one move of the amplifier. (The amplifier's search does not end within a test's time, so
    /// the pass runs within the budget only in the runs of the command line.)
    #[test]
    fn a_time_run_with_the_ends_moved_replays_by_its_work() {
        let (p, s, options) = level(AMPLIFIER);
        assert!(options.settings.nudge_wires);
        let allowance = crate::tree::Allowance { ns: 250_000, work: None };
        let run =
            || super::super::engine::lay_out(&p, &s, &options, &mut Work::default(), &Stop::default(), Some(allowance));
        let snapshot = |(laid, skeleton): (Laid, super::super::engine::LayoutSkeleton)| {
            (laid.placed, laid.routing, laid.entry, laid.notes, skeleton)
        };
        // the clock's readings in a run of the work at which the search has made a few drawings
        let clock = Ticks::default();
        crate::budget::with(&clock, RunLimit::Work(10_000_000_000), Some(f64::MAX), run);
        let ticks = clock.reads() as usize;
        let mut routed = Vec::new();
        for cutoff in [ticks / 2, 3 * ticks / 4, ticks] {
            let (timed, used) = crate::budget::with(&Ticks::default(), RunLimit::Time(cutoff as f64), None, run);
            let pass = timed.0.notes.iter().find(|n| n.starts_with("lines: the ends of the wires that bend or cross"));
            // "{moved} moved, {tried} of {considered} moves routed ...": a move routed when tried is not 0
            routed.push(pass.is_some_and(|n| n.split(", ").nth(1).is_some_and(|t| !t.starts_with("0 of "))));
            let (work, _) = crate::budget::with(&StoppedClock, RunLimit::Work(used.work), None, run);
            assert_eq!(snapshot(timed), snapshot(work), "cutoff {cutoff} of {ticks}, work {}", used.work);
        }
        assert_eq!(routed.last(), Some(&true), "{routed:?}");
    }

    /// The pass runs only when `nudge_wires` is on: off, the line layout notes nothing of it.
    #[test]
    fn the_pass_runs_only_when_on() {
        let (p, s, on) = level(AMPLIFIER);
        let mut off = on.clone();
        off.settings.nudge_wires = false;
        let allowance = crate::tree::Allowance { ns: 250_000, work: None };
        let run = |options: &SolverOptions| {
            let limit = RunLimit::Work(10_000_000_000);
            crate::budget::with(&StoppedClock, limit, None, || {
                super::super::engine::lay_out(&p, &s, options, &mut Work::default(), &Stop::default(), Some(allowance))
            })
            .0
             .0
        };
        let (off, on) = (run(&off), run(&on));
        let pass = |n: &String| n.starts_with("lines: the ends of the wires that bend or cross");
        assert!(!off.notes.iter().any(pass), "{:?}", off.notes);
        assert!(on.notes.iter().any(pass), "{:?}", on.notes);
    }
}
