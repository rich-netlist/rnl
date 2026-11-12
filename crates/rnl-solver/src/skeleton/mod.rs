//! Rebuild a drawing from line identities and exact one-axis constraints.
//! The drawing retains its orientations and wire topology while each axis is compacted.

mod cross;
mod link;
mod pass;
mod read;
pub(crate) mod space;
mod spacing;
mod store;
#[cfg(test)]
mod tests;

use crate::coords::{overlapping, Cause, Overlap, Placed};
use crate::rational::{int, to_f64, Q};
use crate::route::Routing;
use crate::structure::Structure;
use crate::{SolverOptions, Work};
use read::{Key, Skeleton};
use rnl_runtime::problem::{LevelProblem, Position};
use rnl_runtime::solution::{RoutedWire, WirePoint};
use std::collections::{BTreeMap, BTreeSet};

const SWEEPS: usize = 2;

/// The tree drawing compacted once, then optionally rerouted and compacted again.
pub(crate) fn lay_out(
    problem: &LevelProblem,
    s: &Structure,
    options: &SolverOptions,
    work: &mut Work,
    rounds: u8,
) -> crate::tree::Laid {
    let mut laid = crate::tree::lay_out(problem, s, options, work, &crate::tree::Stop::default());
    if laid.entry.written_path_error.is_some() || laid.entry.annotation_error.is_some() {
        laid.notes.push("skeleton: written wire geometry is unevaluable; compaction skipped".into());
        return laid;
    }
    let before = laid.entry.clone();
    let score = |placed: &Placed, routing: &Routing| {
        if let Err(error) = crate::annotation::bounds(problem, &placed.geometry) {
            return crate::score::Entry {
                written_path_error: None,
                annotation_error: Some(error),
                overlaps: 0,
                crowding: 0,
                misleading: 0,
                joins_left: 0,
                layers: Vec::new(),
                code: before.code.clone(),
            };
        }
        let wires = match crate::score::drawn(problem, s, &placed.geometry, routing) {
            Ok(wires) => wires,
            Err(wire) => {
                return crate::score::Entry {
                    written_path_error: Some(wire),
                    annotation_error: None,
                    overlaps: 0,
                    crowding: 0,
                    misleading: 0,
                    joins_left: 0,
                    layers: Vec::new(),
                    code: before.code.clone(),
                }
            }
        };
        crate::score::Entry {
            written_path_error: None,
            annotation_error: None,
            overlaps: placed.overlaps.len()
                + crate::annotation::overlaps(problem, s, &placed.geometry, &placed.origins)
                    .expect("validated annotation bounds"),
            crowding: crate::score::crowding(s, &placed.origins, &placed.geometry.orient, &placed.dropped),
            misleading: crate::score::misleading(problem, s, &placed.origins, &placed.geometry, &wires),
            joins_left: routing.left.iter().map(|n| n.parts.saturating_sub(1)).sum(),
            layers: crate::score::measured_layers(problem, s, placed, Some(&wires)),
            code: before.code.clone(),
        }
    };
    for round in 0..=rounds {
        if !crate::budget::permit(
            crate::budget::Step::Structure,
            problem.objects.len().saturating_add(crate::annotation::estimated_work(problem)),
        ) {
            laid.notes.push(format!("skeleton, round {round}: shared budget exhausted; drawing retained"));
            break;
        }
        let routed = if round > 0 {
            let routing = crate::route::route(problem, s, &laid.placed.geometry, &laid.placed.origins, options, work);
            if crate::budget::stopped() {
                laid.notes.push(format!("skeleton, round {round}: rerouting interrupted; previous drawing retained"));
                break;
            }
            routing
        } else {
            laid.routing.clone()
        };
        let (placed, routing, notes) = compact(problem, s, &laid.placed, &routed, &|_| true, options, work);
        if notes.is_empty() {
            break;
        }
        laid.notes.extend(notes.into_iter().map(|n| format!("skeleton, round {round}: {n}")));
        let made = score(&placed, &routing);
        if made.written_path_error.is_some() || made.annotation_error.is_some() {
            laid.notes
                .push(format!("skeleton, round {round}: unevaluable written wire geometry; previous drawing retained"));
            break;
        }
        if round > 0
            && crate::score::order(&s.active, &[without_origins(s, &laid.entry), without_origins(s, &made)])[0] == 0
        {
            laid.notes.push(format!("skeleton, round {round}: not kept, as it ranks after the round before"));
            break;
        }
        (laid.placed, laid.routing, laid.entry) = (placed, routing, made);
    }
    let (first, last) = (without_origins(s, &before), without_origins(s, &laid.entry));
    if let Some((layer, (b, a))) =
        s.active.layers.iter().zip(first.layers.iter().zip(&last.layers)).find(|(_, (b, a))| b != a)
    {
        let name: Vec<_> = layer.items.iter().map(|(objective, _)| objective.name()).collect();
        let worse = crate::score::order(&s.active, &[first.clone(), last.clone()])[0] == 0;
        laid.notes.push(format!(
            "skeleton: the drawing made ranks {} the group-tree layout's by the objectives in force but the alignment of the symbols' origins, first on {}: {} against {}{}",
            if worse { "after" } else { "before" }, name.join("+"), crate::rational::fmt(a), crate::rational::fmt(b),
            if worse { "; it is taken all the same" } else { "" }
        ));
    }
    laid.candidate.origin = "skeleton";
    laid
}

fn without_origins(s: &Structure, entry: &crate::score::Entry) -> crate::score::Entry {
    let mut entry = entry.clone();
    for (i, layer) in s.active.layers.iter().enumerate() {
        if layer.items.iter().all(|(objective, _)| *objective == crate::objectives::Objective::ComponentAlignment) {
            if let Some(value) = entry.layers.get_mut(i) {
                *value = int(0);
            }
        }
    }
    entry
}

/// Compact a drawing while retaining every selected fact and the topology of its wires.
pub(crate) fn compact(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    keep: &dyn Fn(usize) -> bool,
    options: &SolverOptions,
    work: &mut Work,
) -> (Placed, Routing, Vec<String>) {
    let Some(mut sk) = read::read(problem, s, &placed.geometry, routing, keep, false, false, None, false) else {
        return (
            placed.clone(),
            routing.clone(),
            vec!["skeleton read incomplete or a retained fact does not hold; drawing unchanged".into()],
        );
    };
    if sk.features.is_empty() {
        return (placed.clone(), routing.clone(), Vec::new());
    }
    let mut notes = std::mem::take(&mut sk.notes);
    let bends_read = sk.bends(&BTreeSet::new());
    let mut collapsed: BTreeSet<usize> = BTreeSet::new();
    'sweeps: for sweep in 0..SWEEPS {
        for axis in 0..2 {
            let r = pass::pass(&mut sk, axis, options, work, None);
            collapsed.extend(r.collapsed.iter().copied());
            notes.push(said(&r, sweep, problem));
            if r.interrupted {
                break 'sweeps;
            }
        }
    }
    notes.push(format!(
        "{} lines, {} segments and {} features read; bends of the wires read {}, made {}",
        sk.vars,
        sk.segs.len(),
        sk.features.len(),
        bends_read,
        sk.bends(&collapsed)
    ));
    let (placed, routing) = written(problem, s, placed, routing, &sk);
    (placed, routing, notes)
}

fn said(r: &pass::Report, sweep: usize, problem: &LevelProblem) -> String {
    let axis = if r.axis == 0 { "x" } else { "y" };
    if let Some(why) = &r.failed {
        return format!("{axis} pass {sweep}: left as it was: {why}");
    }
    let facts: Vec<String> = r.facts.iter().map(|(f, n)| format!("{} ({n})", problem.facts[*f].label)).collect();
    format!(
        "{axis} pass {sweep}: {} lines through pins, bends and branch points, {} after; {}bends {}/{} kept; lines merged \
         by tree {}/{}, structure {}/{}, group {}/{}, any {}/{}{}; {} refused for a fact, {} undecided{}; {} edges, {} \
         bounds of more variables",
        r.lines.0,
        r.lines.1,
        if r.levels.0 > 0 || r.rails.0 > 0 {
            format!(
                "pins at one depth on one line {}/{} kept, grounds and supplies of one group on one \
                 line {}/{} kept; ",
                r.levels.1, r.levels.0, r.rails.1, r.rails.0
            )
        } else {
            String::new()
        },
        r.bends.1,
        r.bends.0,
        r.merges[0].1,
        r.merges[0].0,
        r.merges[1].1,
        r.merges[1].0,
        r.merges[2].1,
        r.merges[2].0,
        r.merges[3].1,
        r.merges[3].0,
        if r.merges_left > 0 { format!(", {} more not tried", r.merges_left) } else { String::new() },
        r.with_facts,
        r.undecided,
        if facts.is_empty() { String::new() } else { format!(" (facts in their conflicts: {})", facts.join(", ")) },
        r.edges,
        r.general,
    )
}

/// The drawing as the skeleton's values have it: the objects where their origins are, the texts
/// with a coordinate of their own moved with their owners, the wires routed through their vertices
/// with the bends of segments of no length gone.
fn written(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    sk: &Skeleton,
) -> (Placed, Routing) {
    let mut g = placed.geometry.clone();
    for (k, o) in problem.objects.iter().enumerate() {
        if let (Position::Own { .. }, Some(v)) = (&o.position, sk.origin[k]) {
            if !s.objects[k].left_free {
                g.origin[k] = [sk.value[0][v].clone(), sk.value[1][v].clone()];
            }
        }
    }
    let moved = s.origins(problem, &g);
    for &(k, owner, axis) in &sk.replaced {
        let shift = &moved[owner][axis] - &placed.origins[owner][axis];
        g.origin[k][axis] = &placed.geometry.origin[k][axis] + &shift;
    }
    let origins = s.origins(problem, &g);
    let overlaps = overlapping(s, &origins, &g.orient)
        .into_iter()
        .map(|(a, b)| {
            let cause = placed.overlaps.iter().find(|o| (o.a, o.b) == (a, b)).map_or(Cause::Others, |o| o.cause);
            Overlap { a, b, cause }
        })
        .collect();
    let mut out = routing.clone();
    let at = |v: usize| -> [Q; 2] {
        let x = &sk.vertices[v];
        [&sk.value[0][x.term[0].var] + &x.term[0].off, &sk.value[1][x.term[1].var] + &x.term[1].off]
    };
    for path in &sk.paths {
        let Some(i) = path.routed else { continue };
        let points: Vec<(WirePoint, [Q; 2])> =
            routing.wires[i].points.iter().zip(&path.vertices).map(|(p, &v)| (p.clone(), at(v))).collect();
        let points = simplified(points);
        out.paths[i] = points.iter().map(|(_, q)| q.clone()).collect();
        out.wires[i] = RoutedWire {
            route: routing.wires[i].route,
            points: points
                .into_iter()
                .map(|(p, q)| match p {
                    WirePoint::Bend(_) => WirePoint::Bend((to_f64(&q[0]), to_f64(&q[1]))),
                    other => other,
                })
                .collect(),
        };
        for &v in &path.vertices {
            if let Key::Branch(b) = sk.vertices[v].key {
                let q = at(v);
                if let Some(branch) = out.branches.get_mut(b) {
                    branch.at = (to_f64(&q[0]), to_f64(&q[1]));
                }
            }
        }
    }
    let placed = Placed {
        geometry: g,
        origins,
        dropped: placed.dropped.clone(),
        overlaps,
        optimised: placed.optimised,
        undecided: placed.undecided,
    };
    (placed, out)
}

/// A wire's points with the bends that are no longer bends left out: a bend where the wire does not
/// turn, or at the same place as the point before it.
fn simplified(points: Vec<(WirePoint, [Q; 2])>) -> Vec<(WirePoint, [Q; 2])> {
    let n = points.len();
    if n <= 2 {
        return points;
    }
    let mut out: Vec<(WirePoint, [Q; 2])> = Vec::new();
    for (i, p) in points.into_iter().enumerate() {
        let end = i == 0 || i == n - 1;
        if !end && matches!(p.0, WirePoint::Bend(_)) && out.last().is_some_and(|q| q.1 == p.1) {
            continue;
        }
        if i == n - 1 && out.len() > 1 && out.last().is_some_and(|q| q.1 == p.1 && matches!(q.0, WirePoint::Bend(_))) {
            out.pop();
        }
        out.push(p);
    }
    // a bend in line with its neighbours, going on the same way
    let mut i = 1;
    while i + 1 < out.len() {
        let (a, b, c) = (&out[i - 1].1, &out[i].1, &out[i + 1].1);
        let same_x = a[0] == b[0] && b[0] == c[0];
        let same_y = a[1] == b[1] && b[1] == c[1];
        let onward = |axis: usize| (&b[axis] - &a[axis]) * (&c[axis] - &b[axis]) >= Q::default();
        if matches!(out[i].0, WirePoint::Bend(_)) && ((same_x && onward(1)) || (same_y && onward(0))) {
            out.remove(i);
        } else {
            i += 1;
        }
    }
    out
}

const FIX_UPS: usize = 3;
const MOST_SWEEPS: usize = 6;

pub(crate) type Part<'a> = (&'a BTreeSet<usize>, &'a dyn Fn(usize) -> bool);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Strict {
    pub placed: Placed,
    pub routing: Routing,
    pub notes: Vec<String>,
    pub conflicts: Vec<(usize, Q)>,
    pub failed: bool,
    pub interrupted: bool,
    pub failed_facts: Vec<usize>,
    pub near: BTreeSet<usize>,
    pub sides: BTreeMap<usize, u8>,
    pub moved: Vec<(usize, [Q; 2], [Q; 2])>,
    pub pivot_rows: u64,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn compact_strict(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    options: &SolverOptions,
    work: &mut Work,
    part: Option<Part>,
    levels: &[(crate::structure::PinKey, crate::structure::PinKey)],
) -> Strict {
    let within = part.map(|p| p.0);
    let keep = |f: usize| part.is_none_or(|p| (p.1)(f));
    let mut out = Strict {
        placed: placed.clone(),
        routing: routing.clone(),
        notes: Vec::new(),
        conflicts: Vec::new(),
        failed: false,
        interrupted: false,
        failed_facts: Vec::new(),
        near: BTreeSet::new(),
        sides: BTreeMap::new(),
        moved: Vec::new(),
        pivot_rows: 0,
    };
    let Some(mut sk) = read::read_for_compaction(problem, s, &placed.geometry, routing, &keep, true, within, true)
    else {
        out.failed = true;
        out.interrupted = crate::budget::stopped();
        out.notes.push("skeleton read incomplete or a retained fact does not hold; drawing unchanged".into());
        return out;
    };
    sk.levels = levels.to_vec();
    out.notes = std::mem::take(&mut sk.notes);
    let mut bends_read = sk.bends(&BTreeSet::new());
    let mut collapsed: BTreeSet<usize> = BTreeSet::new();
    let tries: &[(bool, bool)] = if sk.crosses.is_empty() {
        &[(true, false), (false, false)]
    } else {
        &[(true, true), (true, false), (false, true), (false, false)]
    };
    let tries: Vec<(bool, bool, bool)> =
        [true, false].into_iter().flat_map(|grid| tries.iter().map(move |&(w, c)| (w, c, grid))).collect();
    let mut grid_in_conflict = false;
    'tries: for (t, &(whole, crosses, grid)) in tries.iter().enumerate() {
        if !grid && sk.on_grid && !grid_in_conflict {
            out.notes.push(
                "no solution as tried before; the same with the differences not made whole steps of the grid \
                     not tried, as no conflict came of those steps"
                    .to_string(),
            );
            break;
        }
        if t > 0 {
            let Some(next) =
                read::read_for_compaction(problem, s, &placed.geometry, routing, &keep, whole, within, crosses)
            else {
                out.failed = true;
                out.interrupted = crate::budget::stopped();
                break;
            };
            sk = next;
            sk.levels = levels.to_vec();
            sk.on_grid = grid;
            bends_read = sk.bends(&BTreeSet::new());
            collapsed.clear();
            out.failed = false;
            out.failed_facts.clear();
            let lead = if whole { "every segment a whole lead" } else { "the segments as long as they are read" };
            let swaps = if crosses { "the swaps drawn as an X" } else { "the swaps drawn as routed" };
            let off = if grid { "" } else { ", the differences not made whole steps of the grid" };
            out.notes.push(format!("no solution as tried before; now {lead}, {swaps}{off}"));
        }
        let mut merged: [Vec<(read::Term, read::Term)>; 2] = [Vec::new(), Vec::new()];
        let mut failed = [false, false];
        let mut rounded = false;
        for sweep in 0..MOST_SWEEPS {
            let mut new = 0;
            for axis in 0..2 {
                let r = pass::pass(&mut sk, axis, options, work, Some(&mut merged[axis]));
                work.astar_expansions += r.relaxations;
                out.pivot_rows += r.pivot_rows;
                if r.interrupted {
                    out.failed = true;
                    out.interrupted = true;
                    out.failed_facts.clear();
                    out.notes.push(said(&r, MOST_SWEEPS, problem));
                    break 'tries;
                }
                collapsed = collapsed_of(&sk);
                failed[axis] = r.failed.is_some();
                grid_in_conflict |= grid && r.grid_conflict;
                rounded |= r.rounded;
                out.failed_facts.extend(r.failed_facts.iter().copied());
                new += r.new;
                let mut line = said(&r, sweep, problem);
                line.push_str(&format!("; {} kept again, {} new", r.again, r.new));
                if r.cross_lines.0 > 0 {
                    line.push_str(&format!(
                        "; crosses of the symbols made one {}/{}, their lines {} before, {} after",
                        r.crosses.1, r.crosses.0, r.cross_lines.0, r.cross_lines.1
                    ));
                }
                if !r.conflicts.is_empty() {
                    line.push_str(&format!("; {} pairs not their whole gap apart on this axis", r.conflicts.len()));
                }
                out.notes.push(line);
            }
            if sweep + 1 >= SWEEPS && new == 0 {
                break;
            }
        }
        let askew = |sk: &Skeleton| -> BTreeSet<usize> {
            let off = sk.crosses.iter().filter(|c| pass::cross_height(sk, c) != pass::cross_width(sk, c));
            off.map(|c| c.axis).collect()
        };
        for _ in 0..FIX_UPS {
            let axes = askew(&sk);
            if axes.is_empty() || failed[0] || failed[1] {
                break;
            }
            for axis in axes {
                let r = pass::pass(&mut sk, axis, options, work, Some(&mut merged[axis]));
                work.astar_expansions += r.relaxations;
                out.pivot_rows += r.pivot_rows;
                if r.interrupted {
                    out.failed = true;
                    out.interrupted = true;
                    out.failed_facts.clear();
                    out.notes.push(said(&r, MOST_SWEEPS, problem));
                    break 'tries;
                }
                failed[axis] |= r.failed.is_some();
                grid_in_conflict |= grid && r.grid_conflict;
                rounded |= r.rounded;
                out.failed_facts.extend(r.failed_facts.iter().copied());
                out.notes.push(format!("{}; an X made as wide as its lines are apart", said(&r, MOST_SWEEPS, problem)));
            }
            collapsed = collapsed_of(&sk);
        }
        if !askew(&sk).is_empty() {
            out.notes.push("an X is still not at 45° once made".to_string());
            failed[0] = true;
            grid_in_conflict |= grid && rounded;
        }
        out.failed = failed[0] || failed[1];
        out.failed_facts.sort_unstable();
        out.failed_facts.dedup();
        if !out.failed {
            break;
        }
    }
    let boxes: BTreeMap<usize, usize> = sk
        .features
        .iter()
        .enumerate()
        .filter_map(|(f, ft)| if let read::Kind::Symbol(k) = ft.kind { Some((k, f)) } else { None })
        .collect();
    let unkept = pass::unkept(&sk);
    if unkept.is_none() {
        out.failed = true;
        out.interrupted = true;
        out.failed_facts.clear();
    }
    for (i, j) in unkept.into_iter().flatten() {
        for (seg, other) in [(i, j), (j, i)] {
            let (read::Kind::Segment(_), read::Kind::Symbol(k) | read::Kind::Pin(k, _)) =
                (sk.features[seg].kind, sk.features[other].kind)
            else {
                continue;
            };
            let Some(&b) = boxes.get(&k) else { continue };
            let v = |f: usize, a: usize, hi: bool| {
                let ft = &sk.features[f];
                sk.at(a, if hi { &ft.hi[a] } else { &ft.lo[a] })
            };
            let mut side = 0u8;
            if v(seg, 0, true) <= v(b, 0, false) {
                side |= 1;
            }
            if v(seg, 0, false) >= v(b, 0, true) {
                side |= 2;
            }
            if v(seg, 1, true) <= v(b, 1, false) {
                side |= 4;
            }
            if v(seg, 1, false) >= v(b, 1, true) {
                side |= 8;
            }
            *out.sides.entry(k).or_default() |= if side == 0 { 15 } else { side };
        }
        for f in [i, j] {
            match sk.features[f].kind {
                read::Kind::Segment(g) => {
                    let seg = &sk.segs[g];
                    for v in [seg.a, seg.b] {
                        for (axis, at) in sk.vertices[v].at.iter().enumerate() {
                            out.conflicts.push((axis, at.clone()));
                        }
                    }
                }
                read::Kind::Symbol(k) | read::Kind::Pin(k, _) | read::Kind::Text(k) => {
                    out.near.insert(k);
                }
                read::Kind::Group(_) => {}
            }
        }
    }
    out.conflicts.sort();
    out.conflicts.dedup();
    for c in &sk.crosses {
        let across = 1 - c.axis;
        let at = |v: usize, a: usize| crate::rational::fmt(&sk.at(a, &sk.vertices[v].term[a]));
        let (height, width) = (pass::cross_height(&sk, c), pass::cross_width(&sk, c));
        let middle = &sk.value[c.axis][c.centre];
        let on_axis = match c.pair {
            Some(cross::Mid::Axis(v)) => Some(middle == &sk.value[c.axis][v]),
            Some(cross::Mid::Between(a, b)) => Some(middle * int(2) == &sk.value[c.axis][a] + &sk.value[c.axis][b]),
            None => None,
        };
        let pair = match on_axis {
            None => "",
            Some(true) => " on the axis of the mirror pair its nets join",
            Some(false) => " off the axis of the mirror pair its nets join, which the gaps keep it from",
        };
        out.notes.push(format!(
            "the nets {} and {} swap {}, drawn as an X between the lines {} = {} and {}, its middle at {} = {}{pair}{} \
            ",
            c.nets[0],
            c.nets[1],
            if c.axis == 0 { "up and down" } else { "left and right" },
            if across == 1 { "y" } else { "x" },
            at(c.low[1], across),
            at(c.low[0], across),
            if c.axis == 0 { "x" } else { "y" },
            crate::rational::fmt(&sk.value[c.axis][c.centre]),
            if height == width {
                ", at 45°".to_string()
            } else {
                format!(", {} wide for {} apart", crate::rational::fmt(&width), crate::rational::fmt(&height))
            }
        ));
    }
    out.notes.push(format!(
        "{} lines, {} segments and {} features read; bends of the wires read {}, made {}",
        sk.vars,
        sk.segs.len(),
        sk.features.len(),
        bends_read,
        sk.bends(&collapsed)
    ));
    let value = |t: &read::Term, axis: usize| &sk.value[axis][t.var] + &t.off;
    out.moved = sk
        .vertices
        .iter()
        .flat_map(|v| v.trees.iter().map(|&tree| (tree, v.at.clone(), [value(&v.term[0], 0), value(&v.term[1], 1)])))
        .collect();
    // A tentative line arrangement may start with unsatisfied fixed bounds.
    // Never expose a partially repaired drawing as a completed candidate.
    if out.failed || out.interrupted {
        return out;
    }
    let (p, r) = written(problem, s, placed, routing, &sk);
    out.failed_facts.clear();
    let g = &p.geometry;
    for &f in &s.kept {
        if !keep(f) {
            continue;
        }
        if !crate::budget::permit(crate::budget::Step::Structure, s.model.vars.len().saturating_add(1)) {
            out.failed = true;
            out.interrupted = true;
            return out;
        }
        let tol = if s.model.exact.contains(&f) { Q::default() } else { s.model.tol.clone() };
        let at = |point: &rnl_runtime::problem::PointRef, axis| {
            crate::resolve::point_value(problem, g, point, crate::model::ax(axis), 0)
        };
        let ori = |k: usize| g.orient.get(k).copied();
        if crate::holds::check_problem(problem, f, &at, &ori, &tol, &Q::default()) != Some(true) {
            out.failed_facts.push(f);
        }
    }
    if !out.failed_facts.is_empty() {
        out.failed = true;
        out.notes.push("compacted coordinates do not satisfy every retained fact; candidate rejected".into());
        return out;
    }
    out.placed = p;
    out.routing = r;
    out
}

/// Apply final spacing and compose each routed vertex's displacement with compaction.
pub(crate) fn spaced(
    problem: &LevelProblem,
    s: &Structure,
    mut made: Strict,
    options: &SolverOptions,
    work: &mut Work,
    notes: &mut Vec<String>,
) -> Strict {
    let spaced =
        space::space(problem, s, &made.placed, &made.routing, options.settings.spacing.map(i64::from), options, work);
    notes.extend(spaced.notes.iter().map(|n| format!("lines: {n}")));
    // A vertex removed with a collapsed segment follows the nearest surviving line below it.
    let mut map: [BTreeMap<Q, Q>; 2] = [BTreeMap::new(), BTreeMap::new()];
    for (_, was, now) in &spaced.moved {
        for axis in 0..2 {
            map[axis].entry(was[axis].clone()).or_insert_with(|| now[axis].clone());
        }
    }
    let at = |axis: usize, q: &Q| -> Q {
        let below = map[axis].range(..=q.clone()).next_back().or_else(|| map[axis].iter().next());
        below.map_or_else(|| q.clone(), |(was, now)| now + &(q - was))
    };
    made.moved =
        made.moved.iter().map(|(t, pre, post)| (*t, pre.clone(), [at(0, &post[0]), at(1, &post[1])])).collect();
    made.placed = spaced.placed;
    made.routing = spaced.routing;
    made.interrupted |= crate::budget::stopped();
    made
}

fn collapsed_of(sk: &Skeleton) -> BTreeSet<usize> {
    (0..sk.segs.len())
        .filter(|&g| {
            let seg = &sk.segs[g];
            let (a, b) = (&sk.vertices[seg.a], &sk.vertices[seg.b]);
            seg.along.is_some()
                && sk.at(0, &a.term[0]) == sk.at(0, &b.term[0])
                && sk.at(1, &a.term[1]) == sk.at(1, &b.term[1])
        })
        .collect()
}
