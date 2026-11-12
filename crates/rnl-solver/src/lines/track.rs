//! Route hierarchical trees on ordered pin, box and wire lines.
//! Positive shared-budget checkpoints bound allocation and A-star expansion.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use num_traits::Zero;

use crate::rational::{frac, int, Q};
use crate::route::{End, Layout, NetOut, WireOut};
const STEP: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
const NONE: u8 = 4;
const MARGIN: i32 = 12;
const LEAD: u8 = 8;
const AGAIN: usize = 3;
const MOST_POINTS: usize = 1_500_000;
const WHOLE_POINTS: usize = 250_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Owner {
    Side(usize, u8),
    Pin(usize, usize),
    PinPoint(usize, usize),
    Text(usize, u8),
    Written(usize),
    Fixed(usize),
}
#[derive(Clone, Debug, Default)]
pub(crate) struct AxisLines {
    pub at: Vec<Q>,
    pub read: usize,
    pub owners: Vec<Vec<Owner>>,
    bare: Vec<bool>,
    gap_of: Vec<usize>,
    pub order: Vec<usize>,
    pub rank: Vec<usize>,
    pub occ: Vec<Vec<(usize, usize, usize)>>,
    made: Vec<usize>,
    room: Vec<Option<usize>>,
}

impl AxisLines {
    fn slot(&self, line: usize) -> i32 {
        2 * self.rank[line] as i32 + 1
    }
    fn slots(&self) -> i32 {
        2 * self.order.len() as i32 + 1
    }
    fn gap_at(&self, slot: i32) -> usize {
        let place = (slot / 2) as usize;
        if place == 0 {
            return 0;
        }
        let before = self.order[place - 1];
        if before < self.read {
            before + 1
        } else {
            self.gap_of[before]
        }
    }
    fn fresh(&self, slot: i32) -> bool {
        slot % 2 == 0 || {
            let l = self.order[(slot / 2) as usize];
            l < self.read && self.bare[l]
        }
    }
    fn has_room(&self, slot: i32) -> bool {
        let g = self.gap_at(slot);
        self.room[g].is_none_or(|r| self.made[g] < r)
    }
    fn make(&mut self, slot: i32) -> usize {
        let place = (slot / 2) as usize;
        let id = self.at.len();
        let g = self.gap_at(slot);
        self.at.push(Q::zero());
        self.gap_of.resize(id, 0);
        self.gap_of.push(g);
        self.occ.push(Vec::new());
        self.made[g] += 1;
        self.order.insert(place, id);
        self.rank.resize(id + 1, 0);
        for (i, &l) in self.order.iter().enumerate() {
            self.rank[l] = i;
        }
        id
    }
    fn read_line(&self, v: &Q) -> Option<usize> {
        self.at[..self.read].binary_search(v).ok()
    }
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Lines {
    pub axes: [AxisLines; 2],
    pub nets: Vec<Vec<Vec<[usize; 2]>>>,
    pub fixed: Vec<usize>,
}
pub(crate) type PinInput = (usize, usize, [Q; 2], Option<(i64, i64)>);

pub(crate) struct Input<'a> {
    pub layout: &'a Layout,
    pub pins: Vec<PinInput>,
    pub written_net: Vec<Option<usize>>,
    pub groups: Vec<GroupIn>,
    pub weights: (u64, u64),
    pub budget: u64,
    pub full: [BTreeSet<usize>; 2],
    pub kept_clear: BTreeMap<usize, u8>,
    pub fixed: Vec<(usize, Vec<[Q; 2]>)>,
    pub clip: Option<[Q; 4]>,
    pub lenient: bool,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct GroupIn {
    pub members: BTreeSet<usize>,
    pub depth: usize,
    pub inside: Vec<usize>,
    pub boxed: Option<[Q; 4]>,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Routed {
    pub outs: Vec<NetOut>,
    pub lines: Lines,
    pub expansions: u64,
    pub failed: Option<String>,
    pub again: (usize, usize),
    pub costs: Vec<(u32, u32)>,
    pub crossings: u32,
    pub parts: Parts,
    pub made: usize,
    pub steps: usize,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Parts {
    pub inner: usize,
    pub left: usize,
    pub through: usize,
    pub failed: usize,
}
type Pt = (i32, i32);
struct Scene {
    boxes: Vec<(usize, [usize; 4])>,
    at: Vec<(usize, [f64; 4])>,
    kept_clear: BTreeMap<usize, u8>,
    texts: Vec<[usize; 4]>,
    pins: Vec<([usize; 2], (usize, usize))>,
    written: Vec<([usize; 2], usize)>,
    attached: Vec<BTreeSet<usize>>,
    regions: Vec<Option<[(usize, usize); 2]>>,
    clip: Option<[(usize, usize); 2]>,
}
#[derive(Clone, Debug, Default)]
struct Bound {
    clip: Option<[(i32, i32); 2]>,
    barred: Vec<[(i32, i32); 2]>,
}
pub(crate) fn route(input: &Input) -> Routed {
    let layout = input.layout;
    let size = layout.nets.len().saturating_add(layout.boxes.len()).saturating_add(layout.pins.len());
    if !crate::budget::permit(crate::budget::Step::Route, size) {
        return Routed { failed: Some("the shared solve budget stopped line routing".into()), ..Default::default() };
    }
    let mut out = Routed { outs: vec![NetOut::default(); layout.nets.len()], ..Routed::default() };
    let (mut lines, scene) = read(input);
    lines.nets = vec![Vec::new(); layout.nets.len()];
    lines.fixed = vec![0; layout.nets.len()];
    for (net, path) in &input.fixed {
        if *net >= layout.nets.len() || path.len() < 2 {
            continue;
        }
        let slots: Vec<Pt> = path
            .iter()
            .map(|p| {
                let x = lines.axes[0].read_line(&p[0]).expect("read");
                let y = lines.axes[1].read_line(&p[1]).expect("read");
                (lines.axes[0].slot(x), lines.axes[1].slot(y))
            })
            .collect();
        commit(&mut lines, *net, &[slots]);
        lines.fixed[*net] += 1;
    }
    let mut costs = vec![(0u32, 0u32); layout.nets.len()];
    let order: Vec<usize> = (0..layout.nets.len()).filter(|&i| layout.nets[i].anchors.len() >= 2).collect();
    let mut stages: Vec<(Option<usize>, usize)> = Vec::new();
    for &i in &order {
        stages.extend(groups_of_net(input, i).into_iter().map(|g| (Some(g), i)));
        stages.push((None, i));
    }
    stages.sort_by_key(|&(g, i)| {
        let pins = Reverse(layout.nets[i].anchors.len());
        match g {
            Some(g) => (0, Reverse(input.groups[g].depth), g, pins, i),
            None => (1, Reverse(0), 0, pins, i),
        }
    });
    let mut spent = 0u64;
    let interrupted = |mut out: Routed, spent| {
        out.expansions = spent;
        out.failed = Some("the shared solve budget stopped line routing".into());
        out
    };
    for &(g, i) in &stages {
        if let Err(why) = route_part(input, &scene, &mut lines, i, g, &mut spent, &mut out.parts) {
            out.expansions = spent;
            out.failed = Some(format!("{}: {why}", net_name(i)));
            return out;
        }
    }
    let (wc, wb) = input.weights;
    let weighed = |c: (u32, u32)| wc * u64::from(c.0) + wb * u64::from(c.1);
    for _round in 0..AGAIN {
        let mut kept = 0;
        for &i in &order {
            let crossings = crossings_of(&lines, i);
            if crate::budget::stopped() {
                return interrupted(out, spent);
            }
            if crossings == 0 {
                continue;
            }
            if spent > input.budget {
                break;
            }
            out.again.0 += 1;
            let before = (lines.clone(), out.parts);
            let old = (crossings, bends_of(input, &lines, i));
            unroute(&mut lines, i);
            match route_net(input, &scene, &mut lines, i, &mut spent, &mut out.parts) {
                Ok(())
                    if out.parts.failed == before.1.failed
                        && weighed((crossings_of(&lines, i), bends_of(input, &lines, i))) < weighed(old) =>
                {
                    kept += 1;
                    out.again.1 += 1;
                }
                _ => (lines, out.parts) = before,
            }
            if crate::budget::stopped() {
                return interrupted(out, spent);
            }
        }
        if kept == 0 {
            break;
        }
    }
    for &i in &order {
        costs[i] = (crossings_of(&lines, i), bends_of(input, &lines, i));
        if crate::budget::stopped() {
            return interrupted(out, spent);
        }
    }
    out.crossings = crossings_total(&lines);
    if crate::budget::stopped() {
        return interrupted(out, spent);
    }
    drop_empty(&mut lines);
    out.made = (0..2).map(|a| lines.axes[a].order.iter().filter(|&&l| l >= lines.axes[a].read).count()).sum();
    for (net, joins) in lines.nets.iter().enumerate() {
        for path in &joins[lines.fixed[net].min(joins.len())..] {
            for pair in path.windows(2) {
                for (a, axis) in lines.axes.iter().enumerate() {
                    let (p, q) = (axis.rank[pair[0][a]], axis.rank[pair[1][a]]);
                    out.steps += 2 * p.abs_diff(q);
                }
            }
        }
    }
    lines.place();
    for i in 0..layout.nets.len() {
        out.outs[i] = if layout.nets[i].anchors.len() >= 2 {
            net_out(input, &lines, i)
        } else {
            NetOut { parts: layout.nets[i].anchors.len().min(1), ..NetOut::default() }
        };
    }
    out.expansions = spent;
    out.costs = costs;
    out.lines = lines;
    out
}

fn net_name(i: usize) -> String {
    format!("the tree of ROUTE {i}")
}
fn read(input: &Input) -> (Lines, Scene) {
    let layout = input.layout;
    let mut coords: [BTreeMap<Q, Vec<Owner>>; 2] = [BTreeMap::new(), BTreeMap::new()];
    let mut put = |axis: usize, v: &Q, o: Owner| coords[axis].entry(v.clone()).or_default().push(o);
    for (index, (owner, b)) in layout.boxes.iter().enumerate() {
        let k = layout.box_sources.get(&index).copied().unwrap_or(*owner);
        put(0, &b[0], Owner::Side(k, 0));
        put(0, &b[2], Owner::Side(k, 1));
        put(1, &b[1], Owner::Side(k, 0));
        put(1, &b[3], Owner::Side(k, 1));
    }
    for (t, b) in layout.texts.iter().enumerate() {
        put(0, &b[0], Owner::Text(t, 0));
        put(0, &b[2], Owner::Text(t, 1));
        put(1, &b[1], Owner::Text(t, 0));
        put(1, &b[3], Owner::Text(t, 1));
    }
    for (k, pin, at, exit) in &input.pins {
        let (x, y) = match exit {
            Some((_, 0)) => (Owner::PinPoint(*k, *pin), Owner::Pin(*k, *pin)),
            Some((0, _)) => (Owner::Pin(*k, *pin), Owner::PinPoint(*k, *pin)),
            _ => (Owner::Pin(*k, *pin), Owner::Pin(*k, *pin)),
        };
        put(0, &at[0], x);
        put(1, &at[1], y);
    }
    for (w, path) in layout.written.iter().enumerate() {
        for p in path {
            put(0, &p[0], Owner::Written(w));
            put(1, &p[1], Owner::Written(w));
        }
    }
    for (net, path) in &input.fixed {
        for p in path {
            put(0, &p[0], Owner::Fixed(*net));
            put(1, &p[1], Owner::Fixed(*net));
        }
    }
    for net in &layout.nets {
        for a in &net.anchors {
            coords[0].entry(a.at[0].clone()).or_default();
            coords[1].entry(a.at[1].clone()).or_default();
        }
        for p in net.preferred.iter().flatten() {
            coords[0].entry(p[0].clone()).or_default();
            coords[1].entry(p[1].clone()).or_default();
        }
    }
    for b in input.clip.iter().chain(input.groups.iter().filter_map(|g| g.boxed.as_ref())) {
        for (axis, i) in [(0, 0), (1, 1), (0, 2), (1, 3)] {
            coords[axis].entry(b[i].clone()).or_default();
        }
    }
    let mut lines = Lines::default();
    for (axis, coords) in coords.iter_mut().enumerate() {
        let mut ax = AxisLines::default();
        for (v, mut owners) in std::mem::take(coords) {
            owners.sort();
            owners.dedup();
            ax.at.push(v);
            ax.owners.push(owners);
        }
        let n = ax.at.len();
        ax.read = n;
        ax.bare = ax
            .owners
            .iter()
            .map(|o| !o.iter().any(|x| matches!(x, Owner::Pin(..) | Owner::Written(_) | Owner::Fixed(_))))
            .collect();
        ax.gap_of = vec![0; n];
        ax.order = (0..n).collect();
        ax.rank = (0..n).collect();
        ax.occ = vec![Vec::new(); n];
        ax.made = vec![0; n + 1];
        let objects = |l: usize| -> BTreeSet<usize> {
            ax.owners[l]
                .iter()
                .filter_map(|o| match o {
                    Owner::Side(k, _) | Owner::Pin(k, _) | Owner::PinPoint(k, _) => Some(*k),
                    _ => None,
                })
                .collect()
        };
        ax.room = vec![None; n + 1];
        for g in 1..n {
            if !objects(g - 1).is_disjoint(&objects(g)) {
                let d = &ax.at[g] - &ax.at[g - 1] - int(1);
                let fit = if d.is_negative() { 0 } else { crate::rational::to_f64(&d.floor()) as usize };
                ax.room[g] = Some(fit);
            }
        }
        for &g in &input.full[axis] {
            if let Some(r) = ax.room.get_mut(g) {
                *r = Some(0);
            }
        }
        lines.axes[axis] = ax;
    }
    let line = |axis: usize, v: &Q| lines.axes[axis].read_line(v).expect("read");
    let boxes = layout
        .boxes
        .iter()
        .map(|(k, b)| (*k, [line(0, &b[0]), line(0, &b[2]), line(1, &b[1]), line(1, &b[3])]))
        .collect();
    let texts = layout.texts.iter().map(|b| [line(0, &b[0]), line(0, &b[2]), line(1, &b[1]), line(1, &b[3])]).collect();
    let pins = input.pins.iter().map(|(k, p, at, _)| ([line(0, &at[0]), line(1, &at[1])], (*k, *p))).collect();
    let mut written: Vec<(usize, usize, usize, usize, usize)> = Vec::new();
    let mut points: Vec<([usize; 2], usize)> = Vec::new();
    for (w, path) in layout.written.iter().enumerate() {
        let net = input.written_net.get(w).copied().flatten().unwrap_or(layout.nets.len() + w);
        points.extend(path.iter().map(|p| ([line(0, &p[0]), line(1, &p[1])], net)));
        for pair in path.windows(2) {
            let (a, b) = ([line(0, &pair[0][0]), line(1, &pair[0][1])], [line(0, &pair[1][0]), line(1, &pair[1][1])]);
            if a[1] == b[1] && a[0] != b[0] {
                written.push((1, a[1], net, a[0].min(b[0]), a[0].max(b[0])));
            } else if a[0] == b[0] && a[1] != b[1] {
                written.push((0, a[0], net, a[1].min(b[1]), a[1].max(b[1])));
            }
        }
    }
    let mut attached: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); layout.nets.len()];
    for (i, net) in layout.nets.iter().enumerate() {
        let ends: BTreeSet<[usize; 2]> = net.anchors.iter().map(|a| [line(0, &a.at[0]), line(1, &a.at[1])]).collect();
        for &(p, m) in &points {
            if m >= layout.nets.len() && ends.contains(&p) {
                attached[i].insert(m);
            }
        }
    }
    let lines_of_box = |b: &[Q; 4]| [(line(0, &b[0]), line(0, &b[2])), (line(1, &b[1]), line(1, &b[3]))];
    let clip = input.clip.as_ref().map(lines_of_box);
    let regions = input
        .groups
        .iter()
        .map(|group| {
            if let Some(b) = &group.boxed {
                return Some(lines_of_box(b));
            }
            let mut region: Option<[(usize, usize); 2]> = None;
            for (k, b) in layout.boxes.iter().filter(|(k, _)| group.members.contains(k)) {
                let _ = k;
                let ls = [(line(0, &b[0]), line(0, &b[2])), (line(1, &b[1]), line(1, &b[3]))];
                region = Some(match region {
                    None => ls,
                    Some(r) => [0, 1].map(|a| (r[a].0.min(ls[a].0), r[a].1.max(ls[a].1))),
                });
            }
            region
        })
        .collect();
    for (axis, l, net, lo, hi) in written {
        lines.axes[axis].occ[l].push((net, lo, hi));
    }
    let at = layout.boxes.iter().map(|(k, b)| (*k, [0, 1, 2, 3].map(|i| crate::rational::to_f64(&b[i])))).collect();
    let kept_clear = input.kept_clear.clone();
    (lines, Scene { boxes, at, kept_clear, texts, pins, written: points, attached, regions, clip })
}
fn groups_of_net(input: &Input, i: usize) -> Vec<usize> {
    let anchors = &input.layout.nets[i].anchors;
    let mut out: Vec<usize> = (0..input.groups.len())
        .filter(|&g| {
            let members = &input.groups[g].members;
            anchors.iter().filter(|a| a.owner.is_some_and(|k| members.contains(&k))).count() >= 2
        })
        .collect();
    out.sort_by_key(|&g| (Reverse(input.groups[g].depth), g));
    out
}
fn route_net(
    input: &Input,
    scene: &Scene,
    lines: &mut Lines,
    i: usize,
    spent: &mut u64,
    parts: &mut Parts,
) -> Result<(), String> {
    for g in groups_of_net(input, i) {
        route_part(input, scene, lines, i, Some(g), spent, parts)?;
    }
    route_part(input, scene, lines, i, None, spent, parts)
}
fn region_slots(ax: &AxisLines, lo: usize, hi: usize) -> (i32, i32) {
    let first = if lo == 0 { 0 } else { ax.slot(lo - 1) + 1 };
    let last = if hi + 1 >= ax.read { ax.slots() - 1 } else { ax.slot(hi + 1) - 1 };
    (first, last)
}
#[allow(clippy::too_many_arguments)]
fn route_part(
    input: &Input,
    scene: &Scene,
    lines: &mut Lines,
    i: usize,
    group: Option<usize>,
    spent: &mut u64,
    counts: &mut Parts,
) -> Result<(), String> {
    let net = &input.layout.nets[i];
    let n = net.anchors.len();
    let at: Vec<Pt> = net
        .anchors
        .iter()
        .map(|a| {
            let x = lines.axes[0].read_line(&a.at[0]).expect("read");
            let y = lines.axes[1].read_line(&a.at[1]).expect("read");
            (lines.axes[0].slot(x), lines.axes[1].slot(y))
        })
        .collect();
    let here: Vec<usize> = (0..n)
        .filter(|&a| match group {
            Some(g) => net.anchors[a].owner.is_some_and(|k| input.groups[g].members.contains(&k)),
            None => true,
        })
        .collect();
    if here.len() < 2 {
        return Ok(());
    }
    let joins: Vec<Vec<Pt>> = lines.nets[i]
        .iter()
        .map(|vs| {
            let corners: Vec<Pt> = vs.iter().map(|v| (lines.axes[0].slot(v[0]), lines.axes[1].slot(v[1]))).collect();
            points_of(&corners)
        })
        .collect();
    let mut part: Vec<usize> = (0..n + joins.len()).collect();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    fn unite(p: &mut [usize], a: usize, b: usize) {
        let (ra, rb) = (find(p, a), find(p, b));
        if ra != rb {
            p[ra.max(rb)] = ra.min(rb);
        }
    }
    for &(a, b) in &net.joined {
        unite(&mut part, a, b);
    }
    let mut by_point: BTreeMap<Pt, usize> = BTreeMap::new();
    let elements = at.iter().enumerate().map(|(a, p)| (a, *p));
    let points = joins.iter().enumerate().flat_map(|(j, pts)| pts.iter().map(move |p| (n + j, *p)));
    for (e, p) in elements.chain(points) {
        match by_point.get(&p) {
            Some(&f) => unite(&mut part, e, f),
            None => {
                by_point.insert(p, e);
            }
        }
    }
    let roots: Vec<usize> = (0..n + joins.len()).map(|e| find(&mut part, e)).collect();
    let mut parts: BTreeMap<usize, (Vec<usize>, BTreeSet<Pt>)> = BTreeMap::new();
    for &a in &here {
        parts.entry(roots[a]).or_default();
    }
    if parts.len() < 2 {
        return Ok(());
    }
    for (a, root) in roots.iter().take(n).enumerate() {
        if let Some(p) = parts.get_mut(root) {
            p.0.push(a);
        }
    }
    for (j, pts) in joins.iter().enumerate() {
        if let Some(p) = parts.get_mut(&roots[n + j]) {
            p.1.extend(pts.iter().copied());
        }
    }
    let (sx, sy) = here.iter().fold((0i64, 0i64), |(x, y), &a| (x + i64::from(at[a].0), y + i64::from(at[a].1)));
    let mid = (sx as f64 / here.len() as f64, sy as f64 / here.len() as f64);
    let d = |p: Pt| (f64::from(p.0) - mid.0).abs() + (f64::from(p.1) - mid.1).abs();
    let first = here.iter().copied().min_by(|&a, &b| d(at[a]).total_cmp(&d(at[b])).then(a.cmp(&b))).expect("pins");
    let mut tree: BTreeMap<Pt, Option<usize>> = BTreeMap::new();
    let add = |tree: &mut BTreeMap<Pt, Option<usize>>, (anchors, points): &(Vec<usize>, BTreeSet<Pt>)| {
        for &a in anchors {
            tree.insert(at[a], Some(a));
        }
        for &p in points {
            tree.entry(p).or_insert(None);
        }
    };
    add(&mut tree, &parts[&roots[first]]);
    let mut left: Vec<usize> = parts.keys().copied().filter(|&r| r != roots[first]).collect();
    let rect = |r: [(usize, usize); 2]| [0, 1].map(|a| region_slots(&lines.axes[a], r[a].0, r[a].1));
    let given = |r: [(usize, usize); 2], inner: i32| {
        [0, 1].map(|a| (lines.axes[a].slot(r[a].0) + inner, lines.axes[a].slot(r[a].1) - inner))
    };
    let region = |c: usize| match input.groups[c].boxed {
        Some(_) => scene.regions[c].map(|r| given(r, 0)),
        None => scene.regions[c].map(rect),
    };
    let clip = match group {
        Some(g) => region(g),
        None => scene.clip.map(|r| given(r, 1)),
    };
    let inside: Vec<usize> = match group {
        Some(g) => input.groups[g].inside.clone(),
        None => (0..input.groups.len()).collect(),
    };
    let holds = |r: &[(i32, i32); 2], p: &Pt| r[0].0 <= p.0 && p.0 <= r[0].1 && r[1].0 <= p.1 && p.1 <= r[1].1;
    let mut paths: Vec<Vec<Pt>> = Vec::new();
    while !left.is_empty() {
        let gap = |r: usize| {
            let (anchors, points) = &parts[&r];
            let near = |q: Pt| tree.keys().map(|p| (p.0 - q.0).abs() + (p.1 - q.1).abs()).min().unwrap_or(0);
            anchors.iter().map(|&a| at[a]).chain(points.iter().copied()).map(near).min().unwrap_or(0)
        };
        let k = (0..left.len()).min_by_key(|&k| (gap(left[k]), left[k])).expect("left");
        let r = left.remove(k);
        let (anchors, points) = &parts[&r];
        let mut sources: Vec<(Pt, Option<(i64, i64)>)> =
            anchors.iter().map(|&a| (at[a], net.anchors[a].exit)).collect();
        sources.extend(points.iter().filter(|p| !anchors.iter().any(|&a| at[a] == **p)).map(|&p| (p, None)));
        let touching = sources.iter().find(|(p, _)| tree.contains_key(p)).map(|(p, _)| *p);
        let found = match touching {
            Some(p) => Some((vec![p], 0, 0)),
            None => {
                let barred: Vec<[(i32, i32); 2]> = inside
                    .iter()
                    .filter_map(|&c| region(c))
                    .filter(|b| !sources.iter().any(|(p, _)| holds(b, p)) && !tree.keys().any(|p| holds(b, p)))
                    .collect();
                let bound = Bound { clip, barred };
                let seek = |bound: &Bound, spent: &mut u64| match join(
                    input, scene, lines, i, &sources, &tree, &at, bound, spent,
                ) {
                    Err(_) if input.lenient => Ok(None),
                    other => other,
                };
                let mut found = seek(&bound, spent)?;
                if found.is_none() && !bound.barred.is_empty() {
                    found = seek(&Bound { clip, barred: Vec::new() }, spent)?;
                    counts.through += usize::from(found.is_some());
                }
                if found.is_none() && group.is_some() && clip.is_some() {
                    let outer = scene.clip.map(|r| given(r, 1));
                    found = seek(&Bound { clip: outer, barred: Vec::new() }, spent)?;
                    counts.left += usize::from(found.is_some());
                }
                found
            }
        };
        if found.is_none() && input.lenient {
            counts.failed += 1;
            continue;
        }
        let Some((path, _, _)) = found else {
            return Err(format!("no way joins its pin {} to the rest", anchors.first().copied().unwrap_or(0)));
        };
        add(&mut tree, &parts[&r]);
        for p in points_of(&path) {
            tree.entry(p).or_insert(None);
        }
        paths.push(path);
    }
    counts.inner += usize::from(group.is_some());
    // Certify the core's candidate before reserving it for subsequent trees.
    // Existing joins, fixed wires and multi-anchor trees retain their topology.
    if n == 2 && net.joined.is_empty() && joins.is_empty() && paths.len() == 1 {
        if let Some(preferred) = preferred_slots(net, lines, i, clip) {
            paths[0] = preferred;
        }
    }
    commit(lines, i, &paths);
    Ok(())
}

fn preferred_slots(
    net: &crate::route::NetIn,
    lines: &Lines,
    index: usize,
    clip: Option<[(i32, i32); 2]>,
) -> Option<Vec<Pt>> {
    let path: Vec<Pt> = net
        .preferred
        .as_ref()?
        .iter()
        .map(|p| {
            Some((
                lines.axes[0].slot(lines.axes[0].read_line(&p[0])?),
                lines.axes[1].slot(lines.axes[1].read_line(&p[1])?),
            ))
        })
        .collect::<Option<_>>()?;
    for p in points_of(&path) {
        if !crate::budget::permit(crate::budget::Step::Route, 1) {
            return None;
        }
        if clip.is_some_and(|b| p.0 < b[0].0 || p.0 > b[0].1 || p.1 < b[1].0 || p.1 > b[1].1) {
            return None;
        }
        for (axis, slot, across) in [(0, p.0, p.1), (1, p.1, p.0)] {
            if slot % 2 == 0 {
                continue;
            }
            let ax = &lines.axes[axis];
            let other = &lines.axes[1 - axis];
            if ax.occ[ax.order[(slot / 2) as usize]]
                .iter()
                .any(|&(owner, lo, hi)| owner != index && other.slot(lo) <= across && across <= other.slot(hi))
            {
                return None;
            }
        }
    }
    Some(path)
}
fn points_of(path: &[Pt]) -> Vec<Pt> {
    let mut out = Vec::new();
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let (dx, dy) = ((b.0 - a.0).signum(), (b.1 - a.1).signum());
        let mut p = a;
        out.push(p);
        while p != b {
            p = (p.0 + dx, p.1 + dy);
            out.push(p);
        }
    }
    if out.is_empty() {
        out.extend(path.first().copied());
    }
    out
}
struct Window {
    x0: i32,
    y0: i32,
    w: i32,
    h: i32,
    boxed: Vec<bool>,
    inner: Vec<bool>,
    margin: Vec<bool>,
    hard: Vec<bool>,
    text: Vec<bool>,
    barred: Vec<bool>,
    along_x: Vec<i32>,
    along_y: Vec<i32>,
}

impl Window {
    fn index(&self, p: Pt) -> Option<usize> {
        let (x, y) = (p.0 - self.x0, p.1 - self.y0);
        (x >= 0 && y >= 0 && x < self.w && y < self.h).then(|| (y * self.w + x) as usize)
    }
}
fn window_size(w: i32, h: i32) -> Option<usize> {
    let size = u64::try_from(w).ok()?.checked_mul(u64::try_from(h).ok()?)?;
    (size > 0 && size <= MOST_POINTS as u64).then_some(size as usize)
}

fn window(lines: &Lines, scene: &Scene, net: usize, points: &[Pt], margin: i32, bound: &Bound) -> Option<Window> {
    let checkpoint = |n| crate::budget::permit(crate::budget::Step::Route, n).then_some(());
    let (nx, ny) = (lines.axes[0].slots(), lines.axes[1].slots());
    let [(cx0, cx1), (cy0, cy1)] = bound.clip.unwrap_or([(0, nx - 1), (0, ny - 1)]);
    let x0 = (points.iter().map(|p| p.0).min()? - margin).max(cx0);
    let x1 = (points.iter().map(|p| p.0).max()? + margin).min(cx1);
    let y0 = (points.iter().map(|p| p.1).min()? - margin).max(cy0);
    let y1 = (points.iter().map(|p| p.1).max()? + margin).min(cy1);
    if x1 < x0 || y1 < y0 {
        return None;
    }
    let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
    let size = window_size(w, h)?;
    if !crate::budget::permit(crate::budget::Step::Route, size) {
        return None;
    }
    let mut win = Window {
        x0,
        y0,
        w,
        h,
        boxed: vec![false; size],
        inner: vec![false; size],
        margin: vec![false; size],
        hard: vec![false; size],
        text: vec![false; size],
        barred: vec![false; size],
        along_x: vec![-1; size],
        along_y: vec![-1; size],
    };
    if !bound.barred.is_empty() {
        let (dw, dh) = (w as usize + 1, h as usize + 1);
        let mut diff = vec![0i32; dw * dh];
        for b in &bound.barred {
            checkpoint(1)?;
            let (bx0, bx1, by0, by1) = (b[0].0.max(x0), b[0].1.min(x1), b[1].0.max(y0), b[1].1.min(y1));
            if bx1 < bx0 || by1 < by0 {
                continue;
            }
            let (a0, a1, c0, c1) =
                ((bx0 - x0) as usize, (bx1 - x0) as usize + 1, (by0 - y0) as usize, (by1 - y0) as usize + 1);
            diff[c0 * dw + a0] += 1;
            diff[c0 * dw + a1] -= 1;
            diff[c1 * dw + a0] -= 1;
            diff[c1 * dw + a1] += 1;
        }
        for y in 0..dh {
            checkpoint(dw)?;
            for x in 0..dw {
                let mut v = diff[y * dw + x];
                if x > 0 {
                    v += diff[y * dw + x - 1];
                }
                if y > 0 {
                    v += diff[(y - 1) * dw + x];
                }
                if x > 0 && y > 0 {
                    v -= diff[(y - 1) * dw + x - 1];
                }
                diff[y * dw + x] = v;
            }
        }
        for y in 0..h as usize {
            checkpoint(w as usize)?;
            for x in 0..w as usize {
                win.barred[y * w as usize + x] = diff[y * dw + x] > 0;
            }
        }
    }
    let span = |axis: usize, a: usize, b: usize| (lines.axes[axis].slot(a), lines.axes[axis].slot(b));
    let fill = |field: &mut Vec<bool>, xs: (i32, i32), ys: (i32, i32)| -> Option<()> {
        for y in ys.0.max(y0)..=ys.1.min(y1) {
            checkpoint((xs.1.min(x1) - xs.0.max(x0) + 1).max(0) as usize)?;
            for x in xs.0.max(x0)..=xs.1.min(x1) {
                field[((y - y0) * w + (x - x0)) as usize] = true;
            }
        }
        Some(())
    };
    checkpoint(lines.axes.iter().map(|axis| axis.order.len()).sum::<usize>().saturating_mul(4))?;
    let spans: [Vec<(f64, f64)>; 2] = [0, 1].map(|axis| {
        let ax = &lines.axes[axis];
        let n = ax.order.len();
        let mut before = vec![f64::NEG_INFINITY; n + 1];
        let mut last = f64::NEG_INFINITY;
        for (p, &l) in ax.order.iter().enumerate() {
            before[p] = last;
            if l < ax.read {
                last = crate::rational::to_f64(&ax.at[l]);
            }
        }
        before[n] = last;
        let mut after = vec![f64::INFINITY; n + 1];
        let mut next = f64::INFINITY;
        for p in (0..n).rev() {
            let l = ax.order[p];
            if l < ax.read {
                next = crate::rational::to_f64(&ax.at[l]);
            }
            after[p] = next;
        }
        (0..2 * n + 1)
            .map(|s| {
                let place = s / 2;
                if s % 2 == 1 && ax.order[place] < ax.read {
                    let c = crate::rational::to_f64(&ax.at[ax.order[place]]);
                    (c, c)
                } else if s % 2 == 1 {
                    (before[place], after[place + 1])
                } else {
                    (before[place], if place < n { after[place] } else { f64::INFINITY })
                }
            })
            .collect()
    });
    let clearance = crate::route::CLEARANCE as f64;
    let near = |axis: usize, s: i32, lo: f64, hi: f64| {
        let (a, b) = spans[axis][s as usize];
        if a == b {
            lo - clearance < a && a < hi + clearance
        } else {
            a >= lo - clearance && b <= hi + clearance
        }
    };
    for (k, b) in &scene.boxes {
        checkpoint(scene.at.len())?;
        let (xs, ys) = (span(0, b[0], b[1]), span(1, b[2], b[3]));
        fill(&mut win.boxed, xs, ys)?;
        fill(&mut win.inner, (xs.0 + 1, xs.1 - 1), (ys.0 + 1, ys.1 - 1))?;
        let Some(&sides) = scene.kept_clear.get(k) else { continue };
        let Some(&(_, c)) = scene.at.iter().find(|(o, _)| o == k) else { continue };
        let (mut mx0, mut mx1, mut my0, mut my1) = (xs.0, xs.1, ys.0, ys.1);
        while mx0 > 0 && near(0, mx0 - 1, c[0], c[2]) {
            checkpoint(1)?;
            mx0 -= 1;
        }
        while mx1 + 1 < lines.axes[0].slots() && near(0, mx1 + 1, c[0], c[2]) {
            checkpoint(1)?;
            mx1 += 1;
        }
        while my0 > 0 && near(1, my0 - 1, c[1], c[3]) {
            checkpoint(1)?;
            my0 -= 1;
        }
        while my1 + 1 < lines.axes[1].slots() && near(1, my1 + 1, c[1], c[3]) {
            checkpoint(1)?;
            my1 += 1;
        }
        if sides & 1 != 0 {
            fill(&mut win.margin, (mx0, xs.0), (my0, my1))?;
        }
        if sides & 2 != 0 {
            fill(&mut win.margin, (xs.1, mx1), (my0, my1))?;
        }
        if sides & 4 != 0 {
            fill(&mut win.margin, (mx0, mx1), (my0, ys.0))?;
        }
        if sides & 8 != 0 {
            fill(&mut win.margin, (mx0, mx1), (ys.1, my1))?;
        }
    }
    for b in &scene.texts {
        checkpoint(1)?;
        fill(&mut win.text, span(0, b[0], b[1]), span(1, b[2], b[3]))?;
    }
    for (p, _) in &scene.pins {
        checkpoint(1)?;
        let q = (lines.axes[0].slot(p[0]), lines.axes[1].slot(p[1]));
        if let Some(i) = win.index(q) {
            win.hard[i] = true;
        }
    }
    for axis in 0..2 {
        let other = 1 - axis;
        for (line, occ) in lines.axes[axis].occ.iter().enumerate() {
            checkpoint(1)?;
            let s = lines.axes[axis].slot(line);
            let (lo, hi) = if axis == 0 { (x0, x1) } else { (y0, y1) };
            if s < lo || s > hi {
                continue;
            }
            for &(m, a, b) in occ {
                checkpoint(1)?;
                let m = if scene.attached.get(net).is_some_and(|s| s.contains(&m)) { net } else { m };
                let (sa, sb) = (lines.axes[other].slot(a), lines.axes[other].slot(b));
                let (olo, ohi) = if axis == 0 { (y0, y1) } else { (x0, x1) };
                for t in sa.max(olo)..=sb.min(ohi) {
                    if (t - sa.max(olo)) % 256 == 0 {
                        checkpoint((sb.min(ohi) - t + 1).min(256) as usize)?;
                    }
                    let p = if axis == 0 { (s, t) } else { (t, s) };
                    let i = win.index(p).expect("inside");
                    let field = if axis == 0 { &mut win.along_y } else { &mut win.along_x };
                    if field[i] < 0 {
                        field[i] = m as i32;
                    }
                }
            }
        }
    }
    for &(p, m) in &scene.written {
        checkpoint(1)?;
        let own = m == net || scene.attached.get(net).is_some_and(|s| s.contains(&m));
        let q = (lines.axes[0].slot(p[0]), lines.axes[1].slot(p[1]));
        if let (false, Some(i)) = (own, win.index(q)) {
            win.hard[i] = true;
        }
    }
    for (m, joins) in lines.nets.iter().enumerate() {
        if m == net {
            continue;
        }
        for path in joins {
            for v in path {
                checkpoint(1)?;
                let q = (lines.axes[0].slot(v[0]), lines.axes[1].slot(v[1]));
                if let Some(i) = win.index(q) {
                    win.hard[i] = true;
                }
            }
        }
    }
    Some(win)
}
fn pack(c1: u64, c2: u64, c3: u64) -> u64 {
    (c1 << 44) | (c2 << 28) | c3
}
#[allow(clippy::too_many_arguments)]
fn join(
    input: &Input,
    scene: &Scene,
    lines: &Lines,
    net: usize,
    sources: &[(Pt, Option<(i64, i64)>)],
    tree: &BTreeMap<Pt, Option<usize>>,
    at: &[Pt],
    bound: &Bound,
    spent: &mut u64,
) -> Result<Option<(Vec<Pt>, u32, u32)>, String> {
    let mut points: Vec<Pt> = sources.iter().map(|(p, _)| *p).collect();
    points.extend(tree.keys().copied());
    let full = lines.axes[0].slots().max(lines.axes[1].slots());
    let area = match bound.clip {
        Some([x, y]) => {
            (i64::from(x.1) - i64::from(x.0) + 1).max(0) as u64 * (i64::from(y.1) - i64::from(y.0) + 1).max(0) as u64
        }
        None => lines.axes[0].slots() as u64 * lines.axes[1].slots() as u64,
    };
    let mut best: Option<(Vec<Pt>, u32, u32)> = None;
    if area > WHOLE_POINTS as u64 {
        let Some(win) = window(lines, scene, net, &points, MARGIN, bound) else {
            return Err("its search would cover too much of the level".to_string());
        };
        best = search(input, scene, lines, &win, net, sources, tree, at, true, spent)?;
        if best.as_ref().is_some_and(|b| b.1 == 0) || area > MOST_POINTS as u64 {
            return Ok(best);
        }
    }
    for texts in [true, false] {
        if !texts && (best.is_some() || scene.texts.is_empty()) {
            break;
        }
        let Some(win) = window(lines, scene, net, &points, full, bound) else {
            return Ok(best);
        };
        let found = search(input, scene, lines, &win, net, sources, tree, at, texts, spent)?;
        let (wc, wb) = input.weights;
        let cost = |f: &(Vec<Pt>, u32, u32)| wc * u64::from(f.1) + wb * u64::from(f.2);
        if let Some(f) = found {
            if best.as_ref().is_none_or(|b| cost(&f) < cost(b)) {
                best = Some(f);
            }
        }
    }
    Ok(best)
}
#[allow(clippy::too_many_arguments)]
fn search(
    input: &Input,
    _scene: &Scene,
    lines: &Lines,
    win: &Window,
    net: usize,
    sources: &[(Pt, Option<(i64, i64)>)],
    tree: &BTreeMap<Pt, Option<usize>>,
    at: &[Pt],
    texts: bool,
    spent: &mut u64,
) -> Result<Option<(Vec<Pt>, u32, u32)>, String> {
    let anchors = &input.layout.nets[net].anchors;
    let size = (win.w * win.h) as usize;
    if !crate::budget::permit(crate::budget::Step::Route, size.saturating_mul(5)) {
        return Err("the shared solve budget stopped line routing".into());
    }
    let me = net as i32;
    let mut open = vec![false; size];
    let mut lead = vec![0u8; size];
    let mut lane = vec![0u8; size];
    let mut target = vec![0u8; size]; // 1 a point of the tree, 2 an anchor there
    let mut exit_at: BTreeMap<usize, (i64, i64)> = BTreeMap::new();
    for (a, anchor) in anchors.iter().enumerate() {
        let p = at[a];
        let Some(i) = win.index(p) else { continue };
        open[i] = true;
        if let Some(e) = anchor.exit {
            if tree.get(&p) == Some(&Some(a)) {
                exit_at.insert(i, e);
            }
        }
        for (d, step) in STEP.iter().enumerate() {
            let way = (i64::from(step.0), i64::from(step.1));
            let its_way = anchor.exit.is_none_or(|e| e == way);
            let mut run = Vec::new();
            let mut q = (p.0 + step.0, p.1 + step.1);
            let clear = loop {
                let Some(j) = win.index(q) else { break false };
                if tree.contains_key(&q) || at.contains(&q) {
                    break true;
                }
                if win.hard[j] || win.inner[j] {
                    break false;
                }
                if !(win.boxed[j] || win.margin[j]) {
                    break true;
                }
                run.push(j);
                q = (q.0 + step.0, q.1 + step.1);
            };
            if !clear {
                continue;
            }
            let _ = d;
            for j in run {
                let cost = if its_way { 0 } else { LEAD };
                if !open[j] || cost < lead[j] {
                    lead[j] = cost;
                }
                if !open[j] || lane[j] != 0 {
                    lane[j] |= if step.1 == 0 { 1 } else { 2 };
                }
                open[j] = true;
            }
        }
    }
    let mut lo = (i32::MAX, i32::MAX);
    let mut hi = (i32::MIN, i32::MIN);
    for (p, anchor) in tree {
        let Some(i) = win.index(*p) else { continue };
        if anchor.is_none()
            && (win.along_x[i] >= 0 && win.along_x[i] != me || win.along_y[i] >= 0 && win.along_y[i] != me)
        {
            continue;
        }
        target[i] = if anchor.is_some() { 2 } else { 1 };
        lo = (lo.0.min(p.0), lo.1.min(p.1));
        hi = (hi.0.max(p.0), hi.1.max(p.1));
    }
    if lo.0 == i32::MAX {
        return Ok(None);
    }
    let h = |p: Pt| -> u64 {
        let dx = if p.0 < lo.0 {
            lo.0 - p.0
        } else if p.0 > hi.0 {
            p.0 - hi.0
        } else {
            0
        };
        let dy = if p.1 < lo.1 {
            lo.1 - p.1
        } else if p.1 > hi.1 {
            p.1 - hi.1
        } else {
            0
        };
        (dx + dy) as u64
    };
    let free = |i: usize| -> bool {
        !win.barred[i] && (open[i] || !(win.boxed[i] || win.margin[i] || win.hard[i] || texts && win.text[i]))
    };
    let (wc, wb) = input.weights;
    let mut dist = vec![u64::MAX; size * 5];
    let mut from = vec![u32::MAX; size * 5];
    let mut heap: BinaryHeap<Reverse<(u64, u32)>> = BinaryHeap::new();
    for (p, _) in sources {
        let Some(i) = win.index(*p) else { continue };
        let crossed = |along: &Vec<i32>| along[i] >= 0 && along[i] != me;
        if !at.contains(p) && (crossed(&win.along_x) || crossed(&win.along_y)) {
            continue;
        }
        let s = (i * 5 + NONE as usize) as u32;
        dist[s as usize] = 0;
        heap.push(Reverse((h(*p), s)));
    }
    let way = |e: (i64, i64)| STEP.iter().position(|s| (i64::from(s.0), i64::from(s.1)) == e).map(|d| d as u8);
    let exit_of: BTreeMap<usize, Option<u8>> =
        sources.iter().filter_map(|(p, e)| win.index(*p).map(|i| (i, e.and_then(way)))).collect();
    let mut found: Option<u32> = None;
    while let Some(Reverse((_, s))) = heap.pop() {
        let (i, d) = ((s / 5) as usize, (s % 5) as u8);
        let cost = dist[s as usize];
        if !crate::budget::permit(crate::budget::Step::Route, 1) {
            return Err("the shared solve budget stopped line routing".into());
        }
        *spent += 1;
        if *spent > input.budget {
            return Err("the budget of steps ran out".to_string());
        }
        if d != NONE && target[i] > 0 {
            found = Some(s);
            break;
        }
        let p = (win.x0 + (i as i32 % win.w), win.y0 + (i as i32 / win.w));
        let (c1, c2, c3) = (cost >> 44, (cost >> 28) & 0xffff, cost & 0x0fff_ffff);
        for e in 0..4u8 {
            if d != NONE && e == (d + 2) % 4 {
                continue;
            }
            let q = (p.0 + STEP[e as usize].0, p.1 + STEP[e as usize].1);
            let Some(j) = win.index(q) else { continue };
            if !(free(j) || target[j] > 0) {
                continue;
            }
            let horizontal = e % 2 == 0;
            let bit = if horizontal { 1 } else { 2 };
            if lane[i] != 0 && lane[i] & bit == 0 || lane[j] != 0 && lane[j] & bit == 0 {
                continue;
            }
            let along = if horizontal { &win.along_x } else { &win.along_y };
            if (along[i] >= 0 && along[i] != me) || (along[j] >= 0 && along[j] != me) {
                continue;
            }
            let lane = if horizontal { p.1 } else { p.0 };
            let ax = if horizontal { 1 } else { 0 };
            let gap = lane % 2 == 0;
            if gap && !lines.axes[ax].has_room(lane) {
                continue;
            }
            let mut n1 = c1 + wb * u64::from(lead[j]);
            let across = if horizontal { &win.along_y } else { &win.along_x };
            if across[j] >= 0 && across[j] != me {
                n1 += wc;
            }
            if d != NONE && e != d {
                n1 += wb;
            }
            if d == NONE {
                if let Some(Some(x)) = exit_of.get(&i) {
                    if *x != e {
                        n1 += wb;
                    }
                }
            }
            if target[j] == 2 {
                if let Some(x) = exit_at.get(&j).and_then(|x| way(*x)) {
                    if (x + 2) % 4 != e {
                        n1 += wb;
                    }
                }
            }
            let n2 = c2 + u64::from(lines.axes[ax].fresh(lane) && d != e);
            let n3 = c3 + 1;
            let next = pack(n1, n2, n3);
            let t = j * 5 + e as usize;
            if next < dist[t] {
                dist[t] = next;
                from[t] = s;
                heap.push(Reverse((next + h(q), t as u32)));
            }
        }
    }
    let Some(end) = found else { return Ok(None) };
    let mut states = vec![end];
    while let Some(&s) = states.last() {
        let f = from[s as usize];
        if f == u32::MAX {
            break;
        }
        states.push(f);
    }
    states.reverse();
    let pt = |s: u32| {
        let i = (s / 5) as i32;
        (win.x0 + i % win.w, win.y0 + i / win.w)
    };
    let mut corners: Vec<Pt> = vec![pt(states[0])];
    for k in 1..states.len() {
        let here = pt(states[k]);
        let turn = k + 1 < states.len() && states[k + 1] % 5 != states[k] % 5;
        if turn || k + 1 == states.len() {
            corners.push(here);
        }
    }
    let (mut crossings, mut bends) = (0u32, 0u32);
    for k in 1..states.len() {
        let j = (states[k] / 5) as usize;
        let e = (states[k] % 5) as u8;
        let horizontal = e % 2 == 0;
        let across = if horizontal { &win.along_y } else { &win.along_x };
        if across[j] >= 0 && across[j] != me {
            crossings += 1;
        }
        if k >= 2 && (states[k - 1] % 5) as u8 != e {
            bends += 1;
        }
    }
    if states.len() >= 2 {
        let start = (states[0] / 5) as usize;
        if let Some(Some(x)) = exit_of.get(&start) {
            bends += u32::from(*x != (states[1] % 5) as u8);
        }
        let (j, e) = ((end / 5) as usize, (end % 5) as u8);
        if target[j] == 2 {
            if let Some(x) = exit_at.get(&j).and_then(|x| way(*x)) {
                bends += u32::from((x + 2) % 4 != e);
            }
        }
    }
    Ok(Some((corners, crossings, bends)))
}
fn commit(lines: &mut Lines, net: usize, paths: &[Vec<Pt>]) {
    let mut gaps: [BTreeSet<i32>; 2] = [BTreeSet::new(), BTreeSet::new()];
    for path in paths {
        for pair in path.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a.1 == b.1 && a.0 != b.0 && a.1 % 2 == 0 {
                gaps[1].insert(a.1);
            }
            if a.0 == b.0 && a.1 != b.1 && a.0 % 2 == 0 {
                gaps[0].insert(a.0);
            }
            for p in [a, b] {
                if p.0 % 2 == 0 {
                    gaps[0].insert(p.0);
                }
                if p.1 % 2 == 0 {
                    gaps[1].insert(p.1);
                }
            }
        }
        if path.len() == 1 {
            let p = path[0];
            if p.0 % 2 == 0 {
                gaps[0].insert(p.0);
            }
            if p.1 % 2 == 0 {
                gaps[1].insert(p.1);
            }
        }
    }
    let mut of: [BTreeMap<i32, usize>; 2] = [BTreeMap::new(), BTreeMap::new()];
    for axis in 0..2 {
        let ax = &lines.axes[axis];
        let mut slots: BTreeSet<i32> = BTreeSet::new();
        for path in paths {
            for p in path {
                slots.insert(if axis == 0 { p.0 } else { p.1 });
            }
        }
        for &s in &slots {
            if s % 2 == 1 {
                of[axis].insert(s, ax.order[(s / 2) as usize]);
            }
        }
        for &g in gaps[axis].iter().rev() {
            let id = lines.axes[axis].make(g);
            of[axis].insert(g, id);
        }
    }
    let mut joins: Vec<Vec<[usize; 2]>> = Vec::new();
    for path in paths {
        let vs: Vec<[usize; 2]> = path.iter().map(|p| [of[0][&p.0], of[1][&p.1]]).collect();
        for pair in vs.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a[1] == b[1] && a[0] != b[0] {
                let ax = &lines.axes[0];
                let (lo, hi) = if ax.rank[a[0]] < ax.rank[b[0]] { (a[0], b[0]) } else { (b[0], a[0]) };
                lines.axes[1].occ[a[1]].push((net, lo, hi));
            } else if a[0] == b[0] && a[1] != b[1] {
                let ax = &lines.axes[1];
                let (lo, hi) = if ax.rank[a[1]] < ax.rank[b[1]] { (a[1], b[1]) } else { (b[1], a[1]) };
                lines.axes[0].occ[a[0]].push((net, lo, hi));
            }
        }
        joins.push(vs);
    }
    lines.nets[net].extend(joins);
}
fn unroute(lines: &mut Lines, net: usize) {
    for axis in 0..2 {
        for occ in &mut lines.axes[axis].occ {
            occ.retain(|(m, _, _)| *m != net);
        }
    }
    let fixed = lines.fixed.get(net).copied().unwrap_or(0);
    let kept: Vec<Vec<[usize; 2]>> = lines.nets[net].drain(..).take(fixed).collect();
    for vs in kept {
        for pair in vs.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            for along in 0..2 {
                let across = 1 - along;
                if a[across] == b[across] && a[along] != b[along] {
                    let ax = &lines.axes[along];
                    let (lo, hi) =
                        if ax.rank[a[along]] < ax.rank[b[along]] { (a[along], b[along]) } else { (b[along], a[along]) };
                    lines.axes[across].occ[a[across]].push((net, lo, hi));
                }
            }
        }
        lines.nets[net].push(vs);
    }
}
pub(crate) fn polylines(lines: &Lines) -> Vec<(usize, Vec<[Q; 2]>)> {
    let mut out = Vec::new();
    for (net, joins) in lines.nets.iter().enumerate() {
        for path in joins {
            let vs: Vec<[Q; 2]> =
                path.iter().map(|v| [lines.axes[0].at[v[0]].clone(), lines.axes[1].at[v[1]].clone()]).collect();
            out.push((net, corners(vs)));
        }
    }
    out
}
fn drop_empty(lines: &mut Lines) {
    for axis in 0..2 {
        let ax = &mut lines.axes[axis];
        let used: BTreeSet<usize> = lines.nets.iter().flatten().flatten().map(|v| v[axis]).collect();
        let keep: Vec<usize> =
            ax.order.iter().copied().filter(|&l| l < ax.read || used.contains(&l) || !ax.occ[l].is_empty()).collect();
        ax.order = keep;
        for (i, &l) in ax.order.iter().enumerate() {
            ax.rank[l] = i;
        }
    }
}
fn crossings_of(lines: &Lines, net: usize) -> u32 {
    crossings_with(lines, net, &|_| true)
}
fn crossings_total(lines: &Lines) -> u32 {
    let routed = lines.nets.len();
    (0..routed).map(|i| crossings_with(lines, i, &|m| m > i)).sum()
}
fn crossings_with(lines: &Lines, net: usize, other_net: &dyn Fn(usize) -> bool) -> u32 {
    let mut n = 0;
    for axis in 0..2 {
        let other = 1 - axis;
        for (line, occ) in lines.axes[axis].occ.iter().enumerate() {
            if !crate::budget::permit(crate::budget::Step::Route, occ.len().saturating_add(1)) {
                return n;
            }
            for &(m, a, b) in occ.iter().filter(|(m, _, _)| *m == net) {
                let _ = m;
                let (ra, rb) = (lines.axes[other].rank[a], lines.axes[other].rank[b]);
                let r = lines.axes[axis].rank[line];
                for &l in &lines.axes[other].order[ra + 1..rb] {
                    if !crate::budget::permit(
                        crate::budget::Step::Route,
                        lines.axes[other].occ[l].len().saturating_add(1),
                    ) {
                        return n;
                    }
                    for &(m2, c, d) in &lines.axes[other].occ[l] {
                        if m2 == net || !other_net(m2) {
                            continue;
                        }
                        let (rc, rd) = (lines.axes[axis].rank[c], lines.axes[axis].rank[d]);
                        if rc < r && r < rd {
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    n
}
fn bends_of(input: &Input, lines: &Lines, net: usize) -> u32 {
    let anchors = &input.layout.nets[net].anchors;
    let mut n = 0u32;
    for path in &lines.nets[net] {
        n += path.len().saturating_sub(2) as u32;
        for (end, next) in [(0usize, 1usize), (path.len().saturating_sub(1), path.len().saturating_sub(2))] {
            if path.len() < 2 {
                continue;
            }
            let (p, q) = (path[end], path[next]);
            let Some(a) = anchors.iter().position(|a| {
                lines.axes[0].read_line(&a.at[0]) == Some(p[0]) && lines.axes[1].read_line(&a.at[1]) == Some(p[1])
            }) else {
                continue;
            };
            let Some(e) = anchors[a].exit else { continue };
            let way = (
                (lines.axes[0].rank[q[0]] as i64 - lines.axes[0].rank[p[0]] as i64).signum(),
                (lines.axes[1].rank[q[1]] as i64 - lines.axes[1].rank[p[1]] as i64).signum(),
            );
            if way != e {
                n += 1;
            }
        }
    }
    n
}

impl Lines {
    pub(crate) fn place(&mut self) {
        for ax in &mut self.axes {
            let order = ax.order.clone();
            let mut i = 0;
            while i < order.len() {
                if order[i] < ax.read {
                    i += 1;
                    continue;
                }
                let start = i;
                while i < order.len() && order[i] >= ax.read {
                    i += 1;
                }
                let run = &order[start..i];
                let n = run.len() as i64;
                let before = (start > 0).then(|| ax.at[order[start - 1]].clone());
                let after = (i < order.len()).then(|| ax.at[order[i]].clone());
                for (j, &l) in run.iter().enumerate() {
                    let j = j as i64;
                    ax.at[l] = match (&before, &after) {
                        (Some(a), Some(b)) => a + (b - a) * frac(j + 1, n + 1),
                        (None, Some(b)) => b - int(n - j),
                        (Some(a), None) => a + int(j + 1),
                        (None, None) => int(j),
                    };
                }
            }
        }
    }
}
fn net_out(input: &Input, lines: &Lines, net: usize) -> NetOut {
    let anchors = &input.layout.nets[net].anchors;
    let joins = &lines.nets[net];
    let key = |v: [usize; 2]| (lines.axes[0].rank[v[0]], lines.axes[1].rank[v[1]]);
    let mut nodes: BTreeSet<(usize, usize)> = BTreeSet::new();
    for path in joins {
        for &v in path {
            nodes.insert(key(v));
        }
    }
    let mut anchor_at: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for (a, an) in anchors.iter().enumerate() {
        let (Some(x), Some(y)) = (lines.axes[0].read_line(&an.at[0]), lines.axes[1].read_line(&an.at[1])) else {
            continue;
        };
        anchor_at.entry((lines.axes[0].rank[x], lines.axes[1].rank[y])).or_default().push(a);
    }
    for k in anchor_at.keys() {
        nodes.insert(*k);
    }
    let mut adj: BTreeMap<(usize, usize), BTreeSet<(usize, usize)>> = BTreeMap::new();
    for path in joins {
        for pair in path.windows(2) {
            let (a, b) = (key(pair[0]), key(pair[1]));
            let mut on: Vec<(usize, usize)> = nodes
                .iter()
                .copied()
                .filter(|n| {
                    (a.0 == b.0 && n.0 == a.0 && n.1 >= a.1.min(b.1) && n.1 <= a.1.max(b.1))
                        || (a.1 == b.1 && n.1 == a.1 && n.0 >= a.0.min(b.0) && n.0 <= a.0.max(b.0))
                })
                .collect();
            on.sort();
            for w in on.windows(2) {
                adj.entry(w[0]).or_default().insert(w[1]);
                adj.entry(w[1]).or_default().insert(w[0]);
            }
        }
    }
    let at_rank = |n: (usize, usize)| {
        [lines.axes[0].at[lines.axes[0].order[n.0]].clone(), lines.axes[1].at[lines.axes[1].order[n.1]].clone()]
    };
    let mut out = NetOut { parts: 1, ..NetOut::default() };
    let degree = |n: &(usize, usize)| adj.get(n).map_or(0, |s| s.len());
    let mut end_of: BTreeMap<(usize, usize), End> = BTreeMap::new();
    let mut at_nodes: BTreeSet<(usize, usize)> = adj.keys().copied().collect();
    at_nodes.extend(anchor_at.iter().filter(|(_, v)| v.len() >= 2).map(|(k, _)| *k));
    for n in &at_nodes {
        let here: &[usize] = anchor_at.get(n).map_or(&[], |v| v.as_slice());
        match here {
            [a, b] if degree(n) == 0 => {
                out.wires.push(WireOut {
                    start: End::Anchor(*a),
                    end: End::Anchor(*b),
                    vertices: vec![at_rank(*n), at_rank(*n)],
                });
            }
            [a] if degree(n) <= 1 || (!anchors[*a].connects_pin && degree(n) == 2) || anchors[*a].existing_junction => {
                end_of.insert(*n, End::Anchor(*a));
            }
            pins if pins.iter().any(|&a| anchors[a].existing_junction) => {
                let existing = *pins.iter().find(|&&a| anchors[a].existing_junction).unwrap();
                for &a in pins.iter().filter(|&&a| a != existing) {
                    out.wires.push(WireOut {
                        start: End::Anchor(a),
                        end: End::Anchor(existing),
                        vertices: vec![at_rank(*n), at_rank(*n)],
                    });
                }
                end_of.insert(*n, End::Anchor(existing));
            }
            [] if degree(n) >= 3 => {
                let b = out.branches.len();
                out.branches.push(at_rank(*n));
                end_of.insert(*n, End::Branch(b));
            }
            [] => {}
            pins => {
                let b = out.branches.len();
                out.branches.push(at_rank(*n));
                for &a in pins {
                    out.wires.push(WireOut {
                        start: End::Anchor(a),
                        end: End::Branch(b),
                        vertices: vec![at_rank(*n), at_rank(*n)],
                    });
                }
                end_of.insert(*n, End::Branch(b));
            }
        }
    }
    let mut done: BTreeSet<((usize, usize), (usize, usize))> = BTreeSet::new();
    for (&start, &e0) in &end_of {
        for &first in adj.get(&start).into_iter().flatten() {
            if done.contains(&(start, first)) {
                continue;
            }
            let mut vertices = vec![at_rank(start)];
            let (mut prev, mut here) = (start, first);
            done.insert((start, first));
            loop {
                if let Some(&e1) = end_of.get(&here) {
                    done.insert((here, prev));
                    vertices.push(at_rank(here));
                    out.wires.push(WireOut { start: e0, end: e1, vertices: corners(vertices) });
                    break;
                }
                vertices.push(at_rank(here));
                let next = adj[&here].iter().copied().find(|&n| n != prev);
                let Some(next) = next else { break };
                done.insert((here, next));
                (prev, here) = (here, next);
            }
        }
    }
    out
}
fn corners(vs: Vec<[Q; 2]>) -> Vec<[Q; 2]> {
    if vs.len() <= 2 {
        return vs;
    }
    let mut out = vec![vs[0].clone()];
    for i in 1..vs.len() - 1 {
        let (a, b, c) = (&vs[i - 1], &vs[i], &vs[i + 1]);
        let straight = (a[0] == b[0] && b[0] == c[0]) || (a[1] == b[1] && b[1] == c[1]);
        if !straight {
            out.push(b.clone());
        }
    }
    out.push(vs[vs.len() - 1].clone());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_windows_are_rejected_before_word_size_multiplication() {
        assert_eq!(window_size(65_536, 65_536), None);
        assert_eq!(window_size(i32::MAX, i32::MAX), None);
        assert_eq!(window_size(1_000, 1_500), Some(MOST_POINTS));
        assert_eq!(window_size(1_000, 1_501), None);
    }
    use crate::route::{AnchorIn, NetIn};
    fn layout(electrical_middle: bool) -> Layout {
        Layout {
            modern: true,
            nets: vec![NetIn {
                anchors: (0..3)
                    .map(|i| AnchorIn {
                        at: [int(i * 4), int(0)],
                        exit: None,
                        owner: None,
                        connects_pin: i == 1 && electrical_middle,
                        existing_junction: false,
                    })
                    .collect(),
                joined: vec![],
                preferred: None,
            }],
            ..Default::default()
        }
    }
    fn input(layout: &Layout) -> Input<'_> {
        Input {
            layout,
            pins: vec![],
            written_net: vec![],
            groups: vec![],
            weights: (1, 4),
            budget: 10000,
            full: Default::default(),
            kept_clear: Default::default(),
            fixed: vec![],
            clip: None,
            lenient: false,
        }
    }
    #[test]
    fn coordinate_anchors_do_not_acquire_an_electrical_pin_lead() {
        let plain = route(&input(&layout(false)));
        assert!(plain.failed.is_none(), "{:?}", plain.failed);
        assert!(!plain.outs[0].wires.is_empty());
        assert!(plain.outs[0].branches.is_empty());
        let pin = route(&input(&layout(true)));
        assert!(pin.failed.is_none(), "{:?}", pin.failed);
        assert_eq!(pin.outs[0].branches.len(), 1);
    }

    #[test]
    fn an_existing_junction_is_used_as_the_anchor_without_another_branch_dot() {
        let mut layout = layout(true);
        layout.nets[0].anchors[1].existing_junction = true;
        let made = route(&input(&layout));
        assert!(made.failed.is_none(), "{:?}", made.failed);
        assert!(made.outs[0].branches.is_empty());
        assert!(made.outs[0].wires.iter().any(|w| w.start == End::Anchor(1) || w.end == End::Anchor(1)));
    }

    #[test]
    fn simple_trees_use_the_core_path_for_each_pair_of_exit_directions() {
        let directions = [None, Some((1, 0)), Some((0, 1)), Some((-1, 0)), Some((0, -1))];
        for first in directions {
            for last in directions {
                let mut layout = layout(false);
                layout.nets[0].anchors.truncate(2);
                layout.nets[0].anchors[1].at = [int(4), int(3)];
                layout.nets[0].anchors[0].exit = first;
                layout.nets[0].anchors[1].exit = last;
                let expected = crate::route::default_candidate(&layout, 0).unwrap();
                layout.nets[0].preferred = Some(expected.clone());
                let out = route(&input(&layout));
                assert!(out.failed.is_none(), "{first:?} {last:?}: {:?}", out.failed);
                assert_eq!(out.outs[0].wires.len(), 1);
                let wire = &out.outs[0].wires[0];
                let mut actual = wire.vertices.clone();
                if wire.start == End::Anchor(1) {
                    actual.reverse();
                }
                assert_eq!(actual, expected, "{first:?} {last:?}");
            }
        }
    }

    #[test]
    fn preferred_path_respects_existing_tracks_and_group_clip() {
        let mut layout = layout(false);
        layout.nets[0].anchors.truncate(2);
        layout.nets[0].anchors[1].at = [int(4), int(3)];
        layout.nets[0].preferred = crate::route::default_candidate(&layout, 0);
        let mut input = input(&layout);
        input.fixed.push((1, vec![[int(2), int(-1)], [int(2), int(1)]]));
        let (mut lines, _) = read(&input);
        let preferred = preferred_slots(&layout.nets[0], &lines, 0, None).unwrap();
        assert!(preferred_slots(&layout.nets[0], &lines, 0, Some([(0, 1), (0, 1)])).is_none());
        lines.nets = vec![vec![], vec![]];
        let path = input.fixed[0]
            .1
            .iter()
            .map(|p| {
                (
                    lines.axes[0].slot(lines.axes[0].read_line(&p[0]).unwrap()),
                    lines.axes[1].slot(lines.axes[1].read_line(&p[1]).unwrap()),
                )
            })
            .collect();
        commit(&mut lines, 1, &[path]);
        assert!(preferred_slots(&layout.nets[0], &lines, 0, None).is_none());
        assert!(!preferred.is_empty());
    }
    #[test]
    fn window_masks_stop_within_the_shared_budget() {
        let layout = layout(false);
        let input = input(&layout);
        let (lines, scene) = read(&input);
        let points = [(0, 0), (4, 0)];
        let bound = Bound { clip: None, barred: vec![[(1, 3), (0, 0)]] };
        let (made, report) = crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(5), None, || {
            window(&lines, &scene, 0, &points, 0, &bound)
        });
        assert!(made.is_none());
        assert_eq!(report.reason, crate::StopReason::Work);
        assert_eq!(report.work, 5);
        let complete = window(&lines, &scene, 0, &points, 0, &bound).unwrap();
        assert!(complete.barred.iter().any(|&cell| cell));
    }

    #[test]
    fn a_cutoff_during_final_crossing_measurement_rejects_the_route() {
        let layout = layout(false);
        let input = input(&layout);
        let (complete, report) =
            crate::budget::with(&crate::StoppedClock, crate::RunLimit::Unlimited, None, || route(&input));
        assert!(complete.failed.is_none());
        assert!(!complete.outs[0].wires.is_empty());
        let (cut, stopped) =
            crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(report.work - 1), None, || route(&input));
        assert_eq!(stopped.reason, crate::StopReason::Work);
        assert!(cut.failed.as_ref().is_some_and(|why| why.contains("budget")));
        assert!(cut.outs.iter().all(|out| out.wires.is_empty()));
    }

    #[test]
    fn line_routing_time_cutoffs_replay_as_the_same_work_prefix() {
        let layout = layout(false);
        let input = input(&layout);
        for seconds in [0., 1., 10., 50., 100., 500.] {
            let (timed, report) = crate::budget::with(
                &crate::resolve::tests::Ticks::default(),
                crate::RunLimit::Time(seconds),
                None,
                || route(&input),
            );
            let (work, _) =
                crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(report.work), None, || route(&input));
            assert_eq!(timed.outs, work.outs);
            assert_eq!(timed.expansions, work.expansions);
            assert_eq!(timed.costs, work.costs);
            assert_eq!(timed.failed, work.failed);
            if seconds == 0. {
                assert_eq!(report.work, 0);
                assert!(timed.failed.unwrap().contains("budget"));
            }
        }
    }
}
