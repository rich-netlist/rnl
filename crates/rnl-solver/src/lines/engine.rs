//! Combine tree initialization, group settling and strict coordinate compaction.

use super::quality::{lab_order, LabOrder};
use super::{open, port, settle, track};
use crate::coords::Placed;
use crate::rational::{to_f64, Q};
use crate::route::Routing;
use crate::score::{drawn, measured_layers, Entry};
use crate::structure::{GroupTree, NodeKind, Structure};
use crate::tree::Stop;
use crate::tree::{Arrangement, Dir, Item, Laid};
use crate::{SolverOptions, Work};
use num_traits::Zero;
use rnl_runtime::problem::LevelProblem;
use std::collections::{BTreeMap, BTreeSet};

const ROUNDS: usize = 3;
const ALL_SIDES: u8 = 15;
type Cell = (Item, (usize, usize), (usize, usize));
type BoxedCell<'a> = (&'a Item, (usize, usize), (usize, usize), [f64; 4]);

pub(super) fn entry(problem: &LevelProblem, s: &Structure, placed: &Placed, routing: &Routing, code: &[u32]) -> Entry {
    let wires = match drawn(problem, s, &placed.geometry, routing) {
        Ok(wires) => wires,
        Err(wire) => {
            return Entry {
                written_path_error: Some(wire),
                overlaps: 0,
                crowding: 0,
                misleading: 0,
                joins_left: 0,
                layers: vec![],
                code: code.to_vec(),
            }
        }
    };
    Entry {
        written_path_error: None,
        overlaps: placed.overlaps.len(),
        crowding: crate::score::crowding(s, &placed.origins, &placed.geometry.orient, &placed.dropped),
        misleading: crate::score::misleading(problem, s, &placed.origins, &placed.geometry, &wires),
        joins_left: routing.joins_left(),
        layers: measured_layers(problem, s, placed, Some(&wires)),
        code: code.to_vec(),
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutSkeleton {
    pub tables: Vec<SkeletonTable>,
    pub lines: Vec<SkeletonLine>,
    pub trees: Vec<SkeletonTree>,
    pub fallback: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkeletonTable {
    pub group: Option<String>,
    pub rows: usize,
    pub columns: usize,
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub items: Vec<SkeletonItem>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkeletonItem {
    pub label: String,
    pub group: bool,
    pub rows: (usize, usize),
    pub columns: (usize, usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Side,
    Pin,
    Wire,
    Group,
}

impl LineKind {
    pub fn name(self) -> &'static str {
        match self {
            LineKind::Side => "side",
            LineKind::Pin => "pin",
            LineKind::Wire => "wire",
            LineKind::Group => "group",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkeletonLine {
    pub axis: u8,
    pub kind: LineKind,
    pub rank: Option<usize>,
    pub at: f64,
    pub owner: String,
    pub tree: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkeletonTree {
    pub route: usize,
    pub net: String,
    pub xs: Vec<usize>,
    pub ys: Vec<usize>,
}

pub(crate) fn lay_out(
    problem: &LevelProblem,
    s: &Structure,
    options: &SolverOptions,
    work: &mut Work,
    stop: &Stop,
    tree_allowance: Option<u64>,
) -> (Laid, LayoutSkeleton) {
    let _span = crate::profile::span("lines", "layout");

    let span = crate::profile::span("lines", "tree");
    let (pivots, expansions) = (work.pivots, work.astar_expansions);
    let mut laid = super::tree_start(problem, s, options, work, stop, tree_allowance);
    crate::profile::add("tree_pivots", work.pivots - pivots);
    crate::profile::add("tree_expansions", work.astar_expansions - expansions);
    drop(span);

    if laid.entry.written_path_error.is_some() {
        return (laid, LayoutSkeleton { fallback: true, ..LayoutSkeleton::default() });
    }
    let before = laid.entry.clone();
    let mut notes = std::mem::take(&mut laid.notes);
    let weights = (1, 4);
    let fallback = |mut laid: Laid, mut notes: Vec<String>, why: String, work: &mut Work| -> (Laid, LayoutSkeleton) {
        notes.push(format!("lines: {why}; the level is laid out as the group-tree layout has it"));
        if !crate::budget::stopped() {
            let spaced = crate::skeleton::space::space(
                problem,
                s,
                &laid.placed,
                &laid.routing,
                options.settings.spacing.map(i64::from),
                options,
                work,
            );
            notes.extend(spaced.notes.iter().map(|n| format!("lines: {n}")));
            (laid.placed, laid.routing) = (spaced.placed, spaced.routing);
            laid.entry = entry(problem, s, &laid.placed, &laid.routing, &laid.entry.code);
        }
        let skeleton = LayoutSkeleton {
            tables: tables_of(problem, s, &laid.placed, laid.arrangement.as_ref()),
            lines: symbol_lines(problem, s, &laid.placed),
            trees: Vec::new(),
            fallback: true,
        };
        (Laid { notes, ..laid }, skeleton)
    };
    if problem.routes.is_empty() || s.components().next().is_none() {
        return fallback(laid, notes, "nothing to route".to_string(), work);
    }
    let Some(arrangement) = laid.arrangement.clone() else {
        return fallback(laid, notes, "the group-tree layout gave no tables".to_string(), work);
    };
    let theirs = lab_order(problem, s, &laid.placed, &laid.routing, weights, None);
    if theirs.interrupted {
        return fallback(laid, notes, "budget exhausted before line search".into(), work);
    }
    let others = std::mem::take(&mut laid.others);
    let made = match attempt(problem, s, options, &laid.placed, &arrangement, &others, &mut notes, work, stop) {
        Ok(made) => made,
        Err(why) => return fallback(laid, notes, why, work),
    };
    let Made { made, routed, groups, aligned, mine, lists } = made;
    notes.push(format!(
        "lines: by the line-layout order, the drawing made {} against the group-tree layout's {}",
        mine.said(),
        theirs.said()
    ));
    if mine.hidden > 0 || theirs.hidden > 0 {
        let names = |placed: &Placed, routing: &Routing| {
            let hid = port::hidden(problem, s, placed, routing, None);
            hid.iter().map(|&k| problem.objects[k].label.clone()).collect::<Vec<_>>().join(", ")
        };
        notes.push(format!(
            "lines: ports hidden, their way out shut in by a symbol or another net's wire: in the drawing made [{}], \
             in the group-tree layout's [{}]",
            names(&made.placed, &made.routing),
            names(&laid.placed, &laid.routing)
        ));
    }
    if mine.interrupted || mine > theirs {
        return fallback(
            laid,
            notes,
            "the drawing made ranks after the group-tree layout's by the line-layout order".to_string(),
            work,
        );
    }
    let made = if crate::budget::stopped() {
        made
    } else {
        let saved = made.clone();
        let candidate = crate::skeleton::spaced(problem, s, made, options, work, &mut notes);
        if candidate.interrupted {
            saved
        } else {
            candidate
        }
    };
    let (mut arrangement, groups_tree) = arrangement;
    arrangement.lists = lists;
    let arrangement = Some((arrangement, groups_tree));
    notes.extend(open::open_orders(problem, s, &groups, &made.placed, &aligned));
    let skeleton = LayoutSkeleton {
        tables: tables_of(problem, s, &made.placed, arrangement.as_ref()),
        lines: lines_of(problem, s, &made.placed, &routed.lines, &made.moved),
        trees: trees_of(problem, &routed.lines),
        fallback: false,
    };
    let entry = entry(problem, s, &made.placed, &made.routing, &before.code);
    notes.push(ranked(s, &before, &entry));
    let candidate = crate::search::Candidate { origin: "lines", ..laid.candidate };
    let laid =
        Laid { candidate, placed: made.placed, routing: made.routing, entry, notes, arrangement, others: Vec::new() };
    (laid, skeleton)
}

struct Made {
    made: crate::skeleton::Strict,
    routed: track::Routed,
    groups: Vec<settle::GroupSettled>,
    aligned: Vec<(usize, (usize, usize))>,
    mine: LabOrder,
    lists: Vec<Option<Item>>,
}

#[allow(clippy::too_many_arguments)]
fn attempt(
    problem: &LevelProblem,
    s: &Structure,
    options: &SolverOptions,
    placed: &Placed,
    arrangement: &(Arrangement, GroupTree),
    others: &[Arrangement],
    notes: &mut Vec<String>,
    work: &mut Work,
    stop: &Stop,
) -> Result<Made, String> {
    let mut clear: BTreeMap<usize, u8> = BTreeMap::new();
    let mut taken: Option<settle::Taken> = None;
    for attempt in 0..ROUNDS {
        if crate::budget::stopped() {
            return Err("budget exhausted before another routing round".into());
        }
        let span = crate::profile::span("lines", "settle");
        let settled =
            settle::settle(problem, s, placed, arrangement, others, options, &clear, taken.as_ref(), work, stop)?;
        drop(span);
        if attempt == 0 {
            notes.extend(settled.notes.iter().cloned());
        }
        taken = Some(settled.taken.clone());
        let settle::Settled { placed, routing, routed, groups, aligned, made, lists, quality, .. } = settled;
        let crossings = routed.crossings;
        let bends: u32 = routed.costs.iter().map(|c| c.1).sum();
        let made_lines: [usize; 2] = [0, 1].map(|a| {
            let ax = &routed.lines.axes[a];
            ax.order.iter().filter(|&&l| l >= ax.read).count()
        });
        notes.push(format!(
            "lines, round {attempt}: {} routed at the level on the lines of x ({} read, {} made) and y ({} read, {} \
             made); {crossings} crossings and {bends} bends, the ends that leave their pins the wrong way counted; \
             {} of {} crossing nets routed again kept; {} steps",
            crate::driver::count(routed.outs.iter().filter(|o| !o.wires.is_empty()).count(), "tree"),
            routed.lines.axes[0].read,
            made_lines[0],
            routed.lines.axes[1].read,
            made_lines[1],
            routed.again.1,
            routed.again.0,
            routed.expansions
        ));
        let _ = (placed, routing);
        let made = made.ok_or_else(|| "the settled level has no completed compaction".to_string())?;
        notes.extend(made.notes.iter().map(|n| format!("lines, round {attempt}: {n}")));
        if made.failed {
            return Err(format!(
                "a pass of the coordinates found no solution{}",
                facts_said(problem, &made.failed_facts)
            ));
        }
        if made.conflicts.is_empty() {
            return Ok(Made { made, routed, groups, aligned, mine: quality, lists });
        }
        if attempt + 1 == ROUNDS {
            return Err(format!("{} wires cannot keep their gaps where they run", made.conflicts.len()));
        }
        if attempt == 0 {
            for (&k, &sides) in &made.sides {
                *clear.entry(k).or_default() |= sides;
            }
            notes.push(format!(
                "lines, round {attempt}: {} wires could not keep their gaps; the sides of {} they were too near kept \
                 clear",
                made.conflicts.len(),
                crate::driver::count(made.sides.len(), "symbol")
            ));
        } else {
            for k in s.components() {
                clear.insert(k, ALL_SIDES);
            }
            notes.push(format!(
                "lines, round {attempt}: {} wires could not keep their gaps; every side of every symbol kept clear",
                made.conflicts.len()
            ));
        }
    }
    Err("no round kept every gap".to_string())
}

fn facts_said(problem: &LevelProblem, facts: &[usize]) -> String {
    if facts.is_empty() {
        return String::new();
    }
    let said: Vec<String> = facts.iter().map(|&f| settle::fact_name(problem, f)).collect();
    format!(" (the facts in its conflict: {})", said.join(", "))
}

fn ranked(s: &Structure, before: &Entry, made: &Entry) -> String {
    let strip = |e: &Entry| {
        let mut e = e.clone();
        for (i, layer) in s.active.layers.iter().enumerate() {
            if layer.items.iter().all(|(o, _)| *o == crate::objectives::Objective::ComponentAlignment) {
                if let Some(v) = e.layers.get_mut(i) {
                    *v = Q::zero();
                }
            }
        }
        e
    };
    let (b, m) = (strip(before), strip(made));
    let first = s.active.layers.iter().zip(b.layers.iter().zip(&m.layers)).find(|(_, (x, y))| x != y);
    match first {
        Some((layer, (x, y))) => {
            let name: Vec<&str> = layer.items.iter().map(|(o, _)| o.name()).collect();
            let after = crate::score::order(&s.active, &[b.clone(), m.clone()])[0] == 0;
            format!(
                "lines: the drawing made ranks {} the group-tree layout's by the objectives in force but the alignment of \
                 the symbols' origins, first on {}: {} against {}{}",
                if after { "after" } else { "before" },
                name.join("+"),
                crate::rational::fmt(y),
                crate::rational::fmt(x),
                if after { "; it is taken all the same" } else { "" }
            )
        }
        None => "lines: the drawing made ranks with the group-tree layout's by the objectives in force".to_string(),
    }
}

fn lines_of(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    lines: &track::Lines,
    moved: &[(usize, [Q; 2], [Q; 2])],
) -> Vec<SkeletonLine> {
    let g = &placed.geometry;
    let mut out = Vec::new();
    for axis in 0..2 {
        let ax = &lines.axes[axis];
        for (rank, &l) in ax.order.iter().enumerate() {
            let of_symbols = l < ax.read && ax.owners[l].iter().any(|o| matches!(o, track::Owner::Pin(..)));
            if l < ax.read {
                for o in &ax.owners[l] {
                    let line = match *o {
                        track::Owner::Side(k, side) => s.box_at(&placed.origins, &g.orient, k).map(|b| SkeletonLine {
                            axis: axis as u8,
                            kind: LineKind::Side,
                            rank: Some(rank),
                            at: to_f64(&b[axis + 2 * usize::from(side)]),
                            owner: format!("{} {}", problem.objects[k].label, if side == 0 { "low" } else { "high" }),
                            tree: None,
                        }),
                        track::Owner::Pin(k, pin) => {
                            s.pin_at(&placed.origins, &g.orient, k, pin).map(|p| SkeletonLine {
                                axis: axis as u8,
                                kind: LineKind::Pin,
                                rank: Some(rank),
                                at: to_f64(&p[axis]),
                                owner: pin_name(problem, s, k, pin),
                                tree: None,
                            })
                        }
                        _ => None,
                    };
                    out.extend(line);
                }
            }
            if !of_symbols {
                let nets: BTreeSet<usize> =
                    ax.occ[l].iter().map(|(m, _, _)| *m).filter(|&m| m < problem.routes.len()).collect();
                for m in nets {
                    let was = &ax.at[l];
                    let at = moved
                        .iter()
                        .find(|(t, pre, _)| *t == m && pre[axis] == *was)
                        .map_or_else(|| to_f64(was), |(_, _, post)| to_f64(&post[axis]));
                    out.push(SkeletonLine {
                        axis: axis as u8,
                        kind: LineKind::Wire,
                        rank: Some(rank),
                        at,
                        owner: problem.routes.get(m).map_or_else(String::new, |r| r.net.clone()),
                        tree: Some(m),
                    });
                }
            }
        }
    }
    out.extend(group_lines(problem, s, placed));
    out
}

fn symbol_lines(problem: &LevelProblem, s: &Structure, placed: &Placed) -> Vec<SkeletonLine> {
    let g = &placed.geometry;
    let mut out = Vec::new();
    for k in s.components() {
        if let Some(b) = s.box_at(&placed.origins, &g.orient, k) {
            for axis in 0..2 {
                for side in 0..2usize {
                    out.push(SkeletonLine {
                        axis: axis as u8,
                        kind: LineKind::Side,
                        rank: None,
                        at: to_f64(&b[axis + 2 * side]),
                        owner: format!("{} {}", problem.objects[k].label, if side == 0 { "low" } else { "high" }),
                        tree: None,
                    });
                }
            }
        }
        for pin in 0..s.pins(k).len() {
            if let Some(p) = s.pin_at(&placed.origins, &g.orient, k, pin) {
                for (axis, value) in p.iter().enumerate() {
                    out.push(SkeletonLine {
                        axis: axis as u8,
                        kind: LineKind::Pin,
                        rank: None,
                        at: to_f64(value),
                        owner: pin_name(problem, s, k, pin),
                        tree: None,
                    });
                }
            }
        }
    }
    for axis in 0..2u8 {
        let mut ats: Vec<f64> = out.iter().filter(|l| l.axis == axis).map(|l| l.at).collect();
        ats.sort_by(f64::total_cmp);
        ats.dedup();
        for l in out.iter_mut().filter(|l| l.axis == axis) {
            l.rank = ats.binary_search_by(|a| a.total_cmp(&l.at)).ok();
        }
    }
    out.extend(group_lines(problem, s, placed));
    out
}

fn pin_name(problem: &LevelProblem, s: &Structure, k: usize, pin: usize) -> String {
    let terminal = s.pins(k).get(pin).and_then(|p| p.terminals.first().cloned()).unwrap_or_else(|| pin.to_string());
    format!("{}:{terminal}", problem.objects[k].label)
}

fn group_lines(problem: &LevelProblem, s: &Structure, placed: &Placed) -> Vec<SkeletonLine> {
    let g = &placed.geometry;
    let mut out = Vec::new();
    for group in &problem.groups {
        let mut b: Option<[Q; 4]> = None;
        for &k in &group.members {
            let Some(x) = s.box_at(&placed.origins, &g.orient, k) else { continue };
            b = Some(match b {
                None => x,
                Some(y) => [
                    y[0].clone().min(x[0].clone()),
                    y[1].clone().min(x[1].clone()),
                    y[2].clone().max(x[2].clone()),
                    y[3].clone().max(x[3].clone()),
                ],
            });
        }
        let Some(b) = b else { continue };
        for axis in 0..2 {
            for side in 0..2usize {
                out.push(SkeletonLine {
                    axis: axis as u8,
                    kind: LineKind::Group,
                    rank: None,
                    at: to_f64(&b[axis + 2 * side]),
                    owner: format!("{} {}", group.label, if side == 0 { "low" } else { "high" }),
                    tree: None,
                });
            }
        }
    }
    out
}

fn trees_of(problem: &LevelProblem, lines: &track::Lines) -> Vec<SkeletonTree> {
    let mut out = Vec::new();
    for (m, joins) in lines.nets.iter().enumerate() {
        if joins.is_empty() {
            continue;
        }
        let (mut xs, mut ys) = (BTreeSet::new(), BTreeSet::new());
        for path in joins {
            for pair in path.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                if a[1] == b[1] && a[0] != b[0] {
                    ys.insert(lines.axes[1].rank[a[1]]);
                } else if a[0] == b[0] && a[1] != b[1] {
                    xs.insert(lines.axes[0].rank[a[0]]);
                }
            }
        }
        out.push(SkeletonTree {
            route: m,
            net: problem.routes.get(m).map_or_else(String::new, |r| r.net.clone()),
            xs: xs.into_iter().collect(),
            ys: ys.into_iter().collect(),
        });
    }
    out
}

fn tables_of(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    arrangement: Option<&(Arrangement, GroupTree)>,
) -> Vec<SkeletonTable> {
    let Some((arr, groups)) = arrangement else { return Vec::new() };
    let g = &placed.geometry;
    let boxes: BTreeMap<usize, [f64; 4]> = s
        .components()
        .filter_map(|k| {
            s.box_at(&placed.origins, &g.orient, k)
                .map(|b| (k, [to_f64(&b[0]), to_f64(&b[1]), to_f64(&b[2]), to_f64(&b[3])]))
        })
        .collect();
    let label_of = |n: usize| -> Option<String> {
        match groups.nodes.get(n)?.kind {
            NodeKind::Group(i) => problem.groups.get(i).map(|g| g.label.clone()),
            NodeKind::Union(a, b) => Some(format!(
                "{} ∪ {}",
                problem.groups.get(a).map_or("", |g| g.label.as_str()),
                problem.groups.get(b).map_or("", |g| g.label.as_str())
            )),
            NodeKind::Part(..) => Some(format!("part {n}")),
            NodeKind::Root => None,
        }
    };
    fn leaves(arr: &Arrangement, item: &Item, out: &mut Vec<usize>) {
        match item {
            Item::Leaf(k) => out.push(*k),
            Item::Group(n) => {
                if let Some(Some(it)) = arr.lists.get(*n) {
                    leaves(arr, it, out);
                }
            }
            Item::List(_, items) => items.iter().for_each(|i| leaves(arr, i, out)),
        }
    }
    let box_of = |item: &Item| -> Option<[f64; 4]> {
        let mut ks = Vec::new();
        leaves(arr, item, &mut ks);
        let mut b: Option<[f64; 4]> = None;
        for k in ks {
            let Some(x) = boxes.get(&k) else { continue };
            b = Some(match b {
                None => *x,
                Some(y) => [y[0].min(x[0]), y[1].min(x[1]), y[2].max(x[2]), y[3].max(x[3])],
            });
        }
        b
    };
    fn size(item: &Item) -> (usize, usize) {
        match item {
            Item::Leaf(_) | Item::Group(_) => (1, 1),
            Item::List(Dir::Row, items) => items.iter().map(size).fold((0, 0), |(r, c), (ri, ci)| (r.max(ri), c + ci)),
            Item::List(Dir::Column, items) => {
                items.iter().map(size).fold((0, 0), |(r, c), (ri, ci)| (r + ri, c.max(ci)))
            }
        }
    }
    fn place(item: &Item, r0: usize, c0: usize, rows: usize, cols: usize, out: &mut Vec<Cell>) {
        match item {
            Item::Leaf(_) | Item::Group(_) => out.push((item.clone(), (r0, r0 + rows), (c0, c0 + cols))),
            Item::List(dir, items) => {
                let n = items.len();
                let mut at = 0;
                for (i, it) in items.iter().enumerate() {
                    let (ri, ci) = size(it);
                    match dir {
                        Dir::Row => {
                            let w = if i + 1 == n { cols - at } else { ci };
                            place(it, r0, c0 + at, rows, w, out);
                            at += ci;
                        }
                        Dir::Column => {
                            let h = if i + 1 == n { rows - at } else { ri };
                            place(it, r0 + at, c0, h, cols, out);
                            at += ri;
                        }
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    for (n, list) in arr.lists.iter().enumerate() {
        let Some(item) = list else { continue };
        let (rows, cols) = size(item);
        if rows == 0 || cols == 0 {
            continue;
        }
        let mut cells = Vec::new();
        place(item, 0, 0, rows, cols, &mut cells);
        let with_box: Vec<BoxedCell<'_>> =
            cells.iter().filter_map(|(it, r, c)| box_of(it).map(|b| (it, *r, *c, b))).collect();
        if with_box.is_empty() {
            continue;
        }
        let pad = 1.0;
        let all = with_box.iter().fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |a, (_, _, _, b)| {
            [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
        });
        let mut xs = vec![all[0] - pad];
        for c in 1..cols {
            let ends =
                with_box.iter().filter(|(_, _, cc, _)| cc.1 == c).map(|(_, _, _, b)| b[2]).fold(f64::MIN, f64::max);
            let starts =
                with_box.iter().filter(|(_, _, cc, _)| cc.0 == c).map(|(_, _, _, b)| b[0]).fold(f64::MAX, f64::min);
            let last = *xs.last().expect("one");
            xs.push(match (ends > f64::MIN, starts < f64::MAX) {
                (true, true) => (ends + starts) / 2.0,
                (true, false) => ends + pad,
                (false, true) => starts - pad,
                (false, false) => last + pad,
            });
        }
        xs.push(all[2] + pad);
        let mut ys = vec![all[3] + pad];
        for r in 1..rows {
            let above =
                with_box.iter().filter(|(_, rr, _, _)| rr.1 == r).map(|(_, _, _, b)| b[1]).fold(f64::MAX, f64::min);
            let below =
                with_box.iter().filter(|(_, rr, _, _)| rr.0 == r).map(|(_, _, _, b)| b[3]).fold(f64::MIN, f64::max);
            let last = *ys.last().expect("one");
            ys.push(match (above < f64::MAX, below > f64::MIN) {
                (true, true) => (above + below) / 2.0,
                (true, false) => above - pad,
                (false, true) => below + pad,
                (false, false) => last - pad,
            });
        }
        ys.push(all[1] - pad);
        let items = cells
            .iter()
            .map(|(it, r, c)| {
                let (label, group) = match it {
                    Item::Leaf(k) => (problem.objects[*k].label.clone(), false),
                    Item::Group(m) => (label_of(*m).unwrap_or_default(), true),
                    Item::List(..) => (String::new(), false),
                };
                SkeletonItem { label, group, rows: *r, columns: *c }
            })
            .collect();
        out.push(SkeletonTable { group: label_of(n), rows, columns: cols, xs, ys, items });
    }
    out
}

/// Port cleanup can add new bends. Describe the final drawing at its actual coordinates.
pub(super) fn refresh(problem: &LevelProblem, s: &Structure, laid: &Laid, skeleton: &mut LayoutSkeleton) {
    skeleton.tables = tables_of(problem, s, &laid.placed, laid.arrangement.as_ref());
    skeleton.lines = symbol_lines(problem, s, &laid.placed);
    skeleton.trees.clear();
    if skeleton.fallback {
        return;
    }
    let mut tracks: BTreeMap<usize, [BTreeSet<Q>; 2]> = BTreeMap::new();
    for (wire, path) in laid.routing.wires.iter().zip(&laid.routing.paths) {
        for pair in path.windows(2) {
            if pair[0] == pair[1] {
                continue;
            }
            let axis = if pair[0][0] == pair[1][0] {
                Some(0)
            } else if pair[0][1] == pair[1][1] {
                Some(1)
            } else {
                None
            };
            if let Some(axis) = axis {
                tracks.entry(wire.route).or_default()[axis].insert(pair[0][axis].clone());
            }
        }
    }
    for (&tree, axes) in &tracks {
        for (axis, values) in axes.iter().enumerate() {
            for at in values {
                skeleton.lines.push(SkeletonLine {
                    axis: axis as u8,
                    kind: LineKind::Wire,
                    rank: None,
                    at: to_f64(at),
                    owner: problem.routes[tree].net.clone(),
                    tree: Some(tree),
                });
            }
        }
    }
    for axis in 0..2 {
        let mut coordinates: Vec<_> = skeleton
            .lines
            .iter()
            .filter(|l| usize::from(l.axis) == axis && l.kind != LineKind::Group)
            .map(|l| l.at)
            .collect();
        coordinates.sort_by(f64::total_cmp);
        coordinates.dedup();
        for line in skeleton.lines.iter_mut().filter(|l| usize::from(l.axis) == axis && l.kind != LineKind::Group) {
            line.rank = coordinates.binary_search_by(|v| v.total_cmp(&line.at)).ok();
        }
    }
    for (route, _) in tracks {
        let ranks = |axis| {
            skeleton.lines.iter().filter(|l| l.tree == Some(route) && l.axis == axis).filter_map(|l| l.rank).collect()
        };
        skeleton.trees.push(SkeletonTree { route, net: problem.routes[route].net.clone(), xs: ranks(0), ys: ranks(1) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::Ticks;
    use crate::{Clock, RunLimit, StoppedClock};

    #[test]
    fn local_tree_allowance_does_not_interrupt_the_initial_candidate() {
        let (p, s, options, _, _) = settle::tests::seed();
        let tree = crate::tree::Tree::new(&p, &s, &options).with_line_rank();
        let first = tree.first(&mut Work::default());
        let (expected, _) = tree.evaluate_rounds(&first, 3, &mut Work::default());
        let (laid, report) = crate::budget::with(&StoppedClock, RunLimit::Work(100_000_000), None, || {
            super::super::tree_start(&p, &s, &options, &mut Work::default(), &Stop::default(), Some(0))
        });
        assert_eq!(report.reason, crate::StopReason::Complete);
        assert!(report.work > 0);
        assert_eq!(laid.placed, expected.placed);
        assert_eq!(laid.routing, expected.routing);
        assert!(!laid.routing.wires.is_empty());
        assert!(laid.routing.forced.is_empty());
        assert!(laid.notes.iter().any(|n| n.contains("the search tried 0 arrangements")));
    }

    #[test]
    fn combined_line_stages_replay_across_the_local_tree_allowance() {
        let (p, s, options, _, _) = settle::tests::seed();
        let run = || lay_out(&p, &s, &options, &mut Work::default(), &Stop::default(), Some(250_000));
        let clock = Ticks::default();
        let ((full, skeleton), used) = crate::budget::with(&clock, RunLimit::Time(1e9), None, run);
        let ticks = clock.elapsed() as usize;
        assert_eq!(used.reason, crate::StopReason::Complete);
        assert!(full.notes.iter().any(|n| n.starts_with("lines, round 0:")), "{:?}", full.notes);
        assert!(!full.routing.wires.is_empty());
        assert!(!skeleton.tables.is_empty());
        assert!(skeleton.lines.iter().all(|l| l.at.is_finite()));
        let snapshot =
            |(laid, skeleton): (Laid, LayoutSkeleton)| (laid.placed, laid.routing, laid.entry, laid.notes, skeleton);
        for cutoff in [0, 1, ticks / 4, ticks / 2, 3 * ticks / 4, ticks.saturating_sub(1), ticks] {
            let (timed, used) = crate::budget::with(&Ticks::default(), RunLimit::Time(cutoff as f64), None, run);
            let (work, _) = crate::budget::with(&StoppedClock, RunLimit::Work(used.work), None, run);
            assert_eq!(snapshot(timed), snapshot(work), "cutoff {cutoff}, work {}", used.work);
        }
    }
}
