//! Settle nested groups from their tables, then route and compact each completed candidate.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::{Rc, Weak};

use num_traits::Zero;
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{Axis, FactKind, LevelProblem, PointRef, Position, Sense};
use rnl_runtime::solution::{Branch, RoutedWire, WirePoint};

use crate::budget::{permit, Step as BudgetStep};
use crate::coords::{overlapping, Cause, Overlap, Placed};
use crate::rational::{from_f64, int, to_f64, Q};
use crate::resolve::{point_value, Geometry};
use crate::route::{written_path, AnchorIn, End, Layout, NetIn, Routing};
use crate::search::{attach_offset, attachments, orientations_given, Attach};
use crate::skeleton::{compact_strict, Strict};
use crate::structure::{text_box, GroupTree, NodeKind, PinKey, Role, Source, Structure};
use crate::tree::Stop;
use crate::tree::{Arrangement, Dir, Item};
use crate::{SolverOptions, Work};

use super::band::{self, Bands};
use super::cost::{Step, Tally};
use super::fingerprint::{Fingerprint, Look};
use super::quality::{lab_order, LabOrder};
use super::rule::{allowed, minbend};
use super::track;
use crate::tree_hang::current_order;

const ORDER: [Orient; 8] =
    [Orient::R0, Orient::MY, Orient::MX, Orient::R180, Orient::R90, Orient::R270, Orient::MXR90, Orient::MYR90];
const LISTED: usize = 200_000;
const CACHED: usize = 4 * LISTED;
// Logical byte allowances use a fixed 64-bit representation on every target.
// They cover vector growth, allocator/tree overhead and per-core ranking indices;
// they are conservative search limits, not measurements of the process heap.
const LIST_BYTES: usize = 64 * 1024 * 1024;
const CACHE_BYTES: usize = 4 * LIST_BYTES;
const WAY_SETS: usize = 1 << 14;
const WAY_NODES: usize = 1 << 20;
const STEPS: u64 = 20_000_000;
const MOST_DISSOLVED: usize = 8;
const PROBE: usize = 4;

fn combination_count(sizes: impl Iterator<Item = usize>) -> u64 {
    sizes.fold(1u64, |count, n| count.saturating_mul(n.max(1) as u64))
}

thread_local! {
    static RELAXED: Cell<u64> = const { Cell::new(0) };
    static ROUTED: Cell<u64> = const { Cell::new(0) };
}

/// Cost counters belong to one settling run. Restore them for reentrant solver calls
/// and discard an interrupted run's uncharged tail before the next independent solve.
struct Counters([u64; 2]);
impl Counters {
    fn enter() -> Self {
        Self([RELAXED.with(|v| v.replace(0)), ROUTED.with(|v| v.replace(0))])
    }
}
impl Drop for Counters {
    fn drop(&mut self) {
        RELAXED.with(|v| v.set(self.0[0]));
        ROUTED.with(|v| v.set(self.0[1]));
    }
}

#[derive(Clone, Debug)]
enum Rule {
    Stub { pin: usize, rise: Q },
    Pinned { pin: usize, own: usize },
    Attach(Attach),
    Keep([Q; 2]),
    Beyond { pin: usize, b: [Q; 4], keep: [Q; 2] },
}

#[derive(Clone, Debug, Default)]
struct Block {
    at: BTreeMap<usize, [Q; 2]>,
    wires: Vec<(usize, Vec<[Q; 2]>)>,
    bbox: [Q; 4],
    levels: Vec<(PinKey, PinKey)>,
    parts: Vec<([Q; 4], Vec<usize>)>,
    cuts: [Vec<Q>; 2],
    band: BTreeMap<usize, [usize; 2]>,
    crossed: [Vec<usize>; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unit {
    Leaf(usize),
    Block(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Taken {
    values: Vec<Orient>,
    ways: Vec<usize>,
    gaps: BTreeMap<usize, (Vec<usize>, Vec<usize>)>,
    levels: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct GroupSettled {
    pub name: String,
    pub own: Vec<usize>,
    pub all: Vec<usize>,
}

pub(crate) struct Settled {
    pub(super) quality: LabOrder,
    pub placed: Placed,
    pub routing: Routing,
    pub routed: track::Routed,
    pub notes: Vec<String>,
    pub taken: Taken,
    pub aligned: Vec<(usize, (usize, usize))>,
    pub groups: Vec<GroupSettled>,
    pub made: Option<Strict>,
    pub lists: Vec<Option<Item>>,
}

type Score = (bool, usize, usize, bool, usize, LabOrder, usize);

type Choice = (usize, Vec<usize>, Vec<usize>);

pub(super) type Bounds = (Vec<usize>, Vec<usize>);

const LAYERINGS: [&str; 3] =
    ["down the current", "up against the current", "as the group-tree layout's table has them"];

#[derive(Clone, Debug, Default)]
pub(super) struct Spread {
    r: usize,
    d: usize,
    s: usize,
    pub(super) gaps: Vec<usize>,
    pub(super) out: Vec<usize>,
}

type Combo = (Vec<Orient>, Vec<usize>, (usize, usize, usize, u32, usize));

type Listed = (Rc<Vec<Combo>>, Vec<usize>, Vec<usize>, u64);

type ListedKey = (usize, Vec<usize>, Vec<(usize, u8)>, Vec<(usize, u8)>);

fn combo_bytes(classes: usize, ways: usize) -> usize {
    256usize.saturating_add(classes.saturating_mul(8)).saturating_add(ways.saturating_mul(16))
}

#[derive(Default)]
struct Lists {
    cached: BTreeMap<ListedKey, Listed>,
    // Cores and trials can retain lists after eviction. Count each allocation
    // once until its last strong owner drops it; clearing the map is not enough.
    live: Vec<(Weak<Vec<Combo>>, usize)>,
}

impl Lists {
    fn held(&mut self) -> (usize, usize) {
        self.live.retain(|(list, _)| list.strong_count() != 0);
        self.live
            .iter()
            .filter_map(|(list, bytes)| list.upgrade().map(|l| (l.len(), *bytes)))
            .fold((0usize, 0usize), |(n, b), (len, bytes)| (n.saturating_add(len), b.saturating_add(bytes)))
    }

    fn room(&mut self, entry_bytes: usize) -> usize {
        let (mut count, mut bytes) = self.held();
        if count.saturating_add(LISTED) > CACHED || bytes.saturating_add(LIST_BYTES) > CACHE_BYTES {
            self.cached.clear();
            (count, bytes) = self.held();
        }
        LISTED.min(CACHED.saturating_sub(count)).min(LIST_BYTES.min(CACHE_BYTES.saturating_sub(bytes)) / entry_bytes)
    }

    fn insert(&mut self, key: ListedKey, list: Listed) {
        let bytes = combo_bytes(list.1.len(), list.2.len()).saturating_mul(list.0.len());
        self.live.push((Rc::downgrade(&list.0), bytes));
        self.cached.insert(key, list);
    }
}

type Rank = (bool, usize, std::cmp::Reverse<(u64, usize)>, usize, usize);

struct Tried {
    score: (usize, usize, u64, usize, usize),
    dissolve: Vec<usize>,
    stretch: Vec<(usize, usize)>,
    crossings: u32,
    bends: u32,
    ori: Vec<Orient>,
    places: Vec<[Q; 2]>,
    bands: Bands,
    geometry: Geometry,
    routed: track::Routed,
    aligned: Vec<(usize, (usize, usize))>,
    refs: Vec<Vec<PointRef>>,
    left_out: Vec<usize>,
    table_out: usize,
    objects: BTreeSet<usize>,
    gaps: Vec<usize>,
    pairs: Pairs,
    hanging: Hanging,
    layering: Layering,
    levels: Levels,
    boxes: (usize, usize),
}

#[derive(Clone, Debug, Default)]
struct Layering {
    count: usize,
    way: usize,
    refused: usize,
    against: usize,
    facts: usize,
}

#[derive(Clone, Debug, Default)]
struct Levels {
    count: usize,
    tried: bool,
    refused: usize,
    apart: usize,
    kept: Vec<(PinKey, PinKey)>,
}

#[derive(Clone, Debug, Default)]
struct Hanging {
    count: usize,
    hung: usize,
    refused: usize,
}

#[derive(Clone, Debug, Default)]
struct Pairs {
    count: usize,
    kept: usize,
    refused: usize,
    passed: usize,
    left: usize,
    order: Vec<usize>,
}

enum Came {
    Anew,
    Same,
    Unplaced,
}

enum Evaluated {
    Unplaced(u64, Option<Bounds>),
    Same,
    Done(Box<Tried>),
}

#[derive(Clone, Debug)]
struct Edge {
    axis: usize,
    from: PointRef,
    to: PointRef,
    gap: Option<Q>,
    sides: (i8, i8),
}

type Lead = ((usize, usize), (i64, i64), [At; 2]);

type Facing = (bool, Q, (usize, usize), (usize, usize), usize, At, At);

#[derive(Clone, Debug)]
struct At {
    unit: usize,
    off: Q,
}

#[derive(Clone, Debug, Default)]
struct Order {
    n: usize,
    out: Vec<Vec<(usize, Q)>>,
}

impl Order {
    fn new(n: usize) -> Order {
        Order { n, out: vec![Vec::new(); n] }
    }

    fn at_least(&mut self, u: &At, v: &At, gap: Q) {
        if u.unit == v.unit {
            return;
        }
        self.out[u.unit].push((v.unit, &u.off + &gap - &v.off));
    }

    fn equal(&mut self, u: &At, v: &At) {
        if u.unit == v.unit {
            return;
        }
        self.out[u.unit].push((v.unit, &u.off - &v.off));
        self.out[v.unit].push((u.unit, &v.off - &u.off));
    }

    fn least(&self, start: Vec<Q>) -> Option<Vec<Q>> {
        if !permit(BudgetStep::Placement, self.n) {
            return None;
        }
        let mut d = start;
        let mut queue: VecDeque<usize> = (0..self.n).collect();
        let mut queued = vec![true; self.n];
        let mut count = vec![0usize; self.n];
        let mut relaxed = 0u64;
        let out = loop {
            let Some(u) = queue.pop_front() else { break Some(()) };
            queued[u] = false;
            let mut cycle = false;
            for (v, w) in &self.out[u] {
                if !permit(BudgetStep::Placement, 1) {
                    return None;
                }
                relaxed += 1;
                let c = &d[u] + w;
                if c > d[*v] {
                    d[*v] = c;
                    count[*v] += 1;
                    if count[*v] > self.n + 1 {
                        cycle = true;
                        break;
                    }
                    if !queued[*v] {
                        queued[*v] = true;
                        queue.push_back(*v);
                    }
                }
            }
            if cycle {
                break None;
            }
        };
        RELAXED.with(|r| r.set(r.get() + relaxed));
        out.map(|()| d)
    }
}

struct Level<'a> {
    problem: &'a LevelProblem,
    s: &'a Structure,
    tree: &'a Placed,
    nodes: &'a GroupTree,
    weights: (u64, u64),
    clear: &'a BTreeMap<usize, u8>,
    hung: BTreeMap<usize, Vec<(usize, usize, Rule)>>,
    leaves: BTreeSet<usize>,
    edges: Vec<Vec<Edge>>,
    implied_alignments: Vec<Edge>,
    fact_objects: Vec<BTreeSet<usize>>,
    flips: BTreeMap<usize, (Vec<Edge>, bool)>,
    nearest: BTreeMap<(usize, usize), (usize, usize)>,
    levels: bool,
    bands: bool,
    others: Vec<Vec<Option<Item>>>,
}

#[derive(Clone)]
struct State {
    values: Vec<Orient>,
    ways: Vec<usize>,
    ori: Vec<Orient>,
    settled: BTreeSet<usize>,
    blocks: BTreeMap<usize, (Rc<Block>, usize)>,
    own: Vec<Vec<usize>>,
    kids: Vec<Vec<usize>>,
    lists: Vec<Option<Item>>,
}

type Key = (usize, Vec<(usize, usize)>, Vec<(usize, u8)>);

fn code(o: Orient) -> u8 {
    ORDER.iter().position(|x| *x == o).unwrap_or(ORDER.len()) as u8
}

#[derive(Clone, Default)]
struct Core {
    n: usize,
    own: Vec<usize>,
    kids: Vec<usize>,
    list: Option<Item>,
    units: Vec<Unit>,
    others: Vec<Item>,
    classes: Vec<usize>,
    open_ways: Vec<usize>,
    combos: Rc<Vec<Combo>>,
    rank: Vec<usize>,
    ports: usize,
    implied: Vec<Edge>,
    currents: Vec<(usize, usize)>,
    levels: Vec<(PinKey, PinKey)>,
    bounds: BTreeMap<usize, Bounds>,
    done: BTreeSet<Choice>,
    results: Vec<usize>,
    seen: BTreeSet<Fingerprint>,
    round: usize,
    queue: VecDeque<Choice>,
    spread: VecDeque<Spread>,
    exhausted: bool,
    batches: usize,
    tried: usize,
    dissolved: usize,
    banded: BTreeMap<usize, [bool; 2]>,
    notes: Vec<String>,
}

impl Core {
    fn table(&self, t: usize) -> Option<&Item> {
        match t {
            0 => self.list.as_ref(),
            t => self.others.get(t - 1).or(self.list.as_ref()),
        }
    }
}

struct Outcome {
    core: usize,
    choice: Choice,
    tried: Tried,
    score: Score,
    values: Vec<Orient>,
    ways: Vec<usize>,
    block: Option<Rc<Block>>,
    made: Option<Strict>,
    group: GroupSettled,
    own: (u64, usize),
}

struct Node {
    step: usize,
    state: State,
    core: usize,
    path: Vec<usize>,
    carried: BTreeSet<usize>,
    visits: usize,
}

struct Search<'a, 'b> {
    lv: &'b Level<'a>,
    options: &'b SolverOptions,
    steps: Vec<usize>,
    batch: usize,
    given: Option<&'b Taken>,
    cores: Vec<Core>,
    keys: BTreeMap<Key, usize>,
    listed: Lists,
    outcomes: Vec<Outcome>,
    nodes: Vec<Node>,
    learned: BTreeMap<(usize, Vec<u8>, Vec<usize>), Score>,
    solutions: Vec<(usize, u64, usize)>,
    best: Option<usize>,
    lost: Vec<(usize, u64, usize)>,
    fallback: Vec<(usize, u64, usize)>,
    trials: BTreeMap<usize, Trial>,
    start: u64,
    tally: Tally,
}

type SolutionCost = (usize, u64, usize);
type TriedRange = (Option<usize>, Vec<SolutionCost>, std::ops::Range<usize>);
struct Trial {
    other: Core,
    tried: Option<TriedRange>,
    made: usize,
}

fn freed(state: &State, own: &[usize]) -> (State, BTreeSet<usize>) {
    let mut out = state.clone();
    let loose: BTreeSet<usize> = own.iter().copied().filter(|k| out.settled.remove(k)).collect();
    (out, loose)
}

fn spent(_work: &Work) -> u64 {
    crate::budget::consumed()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn settle(
    problem: &LevelProblem,
    s: &Structure,
    tree: &Placed,
    arrangement: &(Arrangement, GroupTree),
    others: &[Arrangement],
    options: &SolverOptions,
    clear: &BTreeMap<usize, u8>,
    given: Option<&Taken>,
    work: &mut Work,
    stop: &Stop,
) -> Result<Settled, String> {
    let _counters = Counters::enter();
    if !permit(BudgetStep::Candidate, problem.objects.len().saturating_add(crate::annotation::estimated_work(problem)))
    {
        return Err("budget exhausted".into());
    }
    let (arr, nodes) = (&arrangement.0, &arrangement.1);
    let mut lv = Level {
        problem,
        s,
        tree,
        nodes,
        weights: (1, 4),
        clear,
        hung: BTreeMap::new(),
        leaves: BTreeSet::new(),
        edges: vec![Vec::new(); problem.facts.len()],
        implied_alignments: super::distribution::alignments(problem, s)
            .ok_or_else(|| "budget exhausted while deriving distribution alignments".to_string())?
            .into_iter()
            .map(|a| Edge { axis: a.axis, from: a.from, to: a.to, gap: None, sides: (0, 0) })
            .collect(),
        fact_objects: vec![BTreeSet::new(); problem.facts.len()],
        nearest: BTreeMap::new(),
        flips: BTreeMap::new(),
        levels: true,
        bands: true,
        others: others.iter().map(|a| a.lists.clone()).collect(),
    };
    lv.nearest = lv.nearest_pins();
    let mut own = vec![Vec::new(); nodes.nodes.len()];
    let mut kids = vec![Vec::new(); nodes.nodes.len()];
    for &f in &s.kept {
        lv.edges[f] = fact_edges(problem, s, tree, arr, f, false);
        if open_on_y(&problem.facts[f].kind) {
            let down = lv.down_the_current(&lv.edges[f]);
            if down != 0 {
                lv.flips.insert(f, (fact_edges(problem, s, tree, arr, f, true), down > 0));
            }
        }
    }
    for (f, edges) in written_edges(problem, &s.kept) {
        lv.edges[f] = edges;
    }
    for (f, fact) in problem.facts.iter().enumerate() {
        for r in fact_points(&fact.kind) {
            objects_of(&r, &mut lv.fact_objects[f]);
        }
    }
    for (n, list) in arr.lists.iter().enumerate() {
        if let Some(item) = list {
            walk(item, &mut own[n], &mut kids[n]);
            own[n].sort_unstable();
            lv.leaves.extend(own[n].iter().copied());
        }
    }
    for k in s.components() {
        if s.objects[k].own
            && !lv.leaves.contains(&k)
            && s.objects[k].stub.is_none()
            && s.objects[k].role == Role::Element
        {
            own[0].push(k);
            lv.leaves.insert(k);
        }
    }
    for (k, info) in s.objects.iter().enumerate() {
        if lv.leaves.contains(&k) || !info.own || info.left_free || info.symbol.is_some() {
            continue;
        }
        if !matches!(problem.objects[k].position, Position::Own { .. }) {
            continue;
        }
        let mut related: BTreeSet<usize> = BTreeSet::new();
        for &f in &s.kept {
            let points = fact_points(&problem.facts[f].kind);
            if points.iter().any(|r| refers_to(r, k)) {
                for r in &points {
                    objects_of(r, &mut related);
                }
            }
        }
        for w in &problem.wires {
            if w.anchors.iter().any(|r| refers_to(r, k)) {
                for r in &w.anchors {
                    objects_of(r, &mut related);
                }
            }
        }
        related.retain(|j| *j != k && lv.leaves.contains(j));
        if related.is_empty() {
            continue;
        }
        let node = nodes.lowest_common(&related.into_iter().collect::<Vec<_>>());
        own[node].push(k);
        own[node].sort_unstable();
        lv.leaves.insert(k);
    }
    own[0].sort_unstable();
    lv.hung = hanging(problem, s, tree, &lv.leaves);
    let steps: Vec<usize> =
        nodes.post_order().into_iter().filter(|&n| !own[n].is_empty() || !kids[n].is_empty()).collect();
    if steps.last() != Some(&0) {
        return Err("the level's group has nothing to settle".to_string());
    }
    let state = State {
        values: given.map_or_else(|| arr.values.clone(), |t| t.values.clone()),
        ways: given.map_or_else(|| arr.ways.clone(), |t| t.ways.clone()),
        ori: tree.geometry.orient.clone(),
        settled: BTreeSet::new(),
        blocks: BTreeMap::new(),
        own,
        kids,
        lists: arr.lists.clone(),
    };
    let back = given.is_none() && crate::budget::current_limit().or(options.run_limit) != Some(crate::RunLimit::Seed);
    let tries: &[bool] = if given.is_some_and(|t| !t.levels) { &[false] } else { &[true, false] };
    for &levels in tries {
        lv.levels = levels;
        let mut search = Search {
            lv: &lv,
            options,
            steps: steps.clone(),
            batch: 16,
            given,
            cores: Vec::new(),
            keys: BTreeMap::new(),
            listed: Lists::default(),
            outcomes: Vec::new(),
            nodes: Vec::new(),
            learned: BTreeMap::new(),
            solutions: Vec::new(),
            best: None,
            lost: Vec::new(),
            fallback: Vec::new(),
            trials: BTreeMap::new(),
            start: spent(work),
            tally: Tally::new(problem.objects.len()),
        };
        let mut id = search.node(0, state.clone(), Vec::new());
        let mut failed = None;
        loop {
            if crate::budget::stopped() {
                break;
            }
            search.batch(id, search.batch, true, work, stop)?;
            if search.nodes[id].step + 1 == search.steps.len() {
                break;
            }
            let Some(r) = search.best_uncarried(id) else {
                failed = Some(search.steps[search.nodes[id].step]);
                break;
            };
            id = search.carry(id, r);
        }
        if !crate::budget::stopped()
            && search.best.is_none()
            && levels
            && search.cores.iter().any(|c| !c.levels.is_empty())
        {
            continue;
        }
        if let (Some(n), false) = (failed, back) {
            return Err(format!("{} could not be placed and routed in any orientation tried", lv.name(n)));
        }
        let mut exhausted = false;
        if back {
            loop {
                if stop.reached(work) {
                    break;
                }
                let Some(id) = search.pick() else {
                    exhausted = true;
                    break;
                };
                search.visit(id, work, stop)?;
            }
        }
        let mut settled = search.finish(exhausted, work)?;
        if !levels && given.is_none() {
            settled.notes.push(
                "lines: the level's groups made no complete solution with their elements at one depth on one line \
                ; settled again with them as they come"
                    .to_string(),
            );
        }
        return Ok(settled);
    }
    unreachable!("the second settling leaves the levels as they come")
}

impl Search<'_, '_> {
    fn node(&mut self, step: usize, state: State, path: Vec<usize>) -> usize {
        let core = self.core_for(step, &state);
        self.nodes.push(Node { step, state, core, path, carried: BTreeSet::new(), visits: 0 });
        self.nodes.len() - 1
    }

    fn core_for(&mut self, step: usize, state: &State) -> usize {
        let s = self.lv.s;
        let n = self.steps[step];
        let kids: Vec<(usize, usize)> =
            state.kids[n].iter().map(|&c| (c, state.blocks.get(&c).map_or(usize::MAX, |b| b.1))).collect();
        let own: BTreeSet<usize> = state.own[n].iter().copied().collect();
        let mut fixed: Vec<(usize, u8)> = Vec::new();
        for class in s.open.classes.iter().filter(|c| c.members.iter().any(|(m, _)| own.contains(m))) {
            for &(m, _) in &class.members {
                if state.settled.contains(&m) {
                    fixed.push((m, code(state.ori[m])));
                }
            }
        }
        fixed.sort_unstable();
        fixed.dedup();
        let key = (n, kids, fixed);
        if let Some(&c) = self.keys.get(&key) {
            return c;
        }
        let mut core = Core { n, ..Core::default() };
        core.own.clone_from(&state.own[n]);
        core.kids.clone_from(&state.kids[n]);
        core.list.clone_from(&state.lists[n]);
        let items = |list: &Item| {
            let (mut own, mut kids) = (Vec::new(), Vec::new());
            walk(list, &mut own, &mut kids);
            own.sort_unstable();
            kids.sort_unstable();
            (own, kids)
        };
        core.others.clear();
        for other in self.lv.others.iter().filter_map(|lists| lists.get(n).cloned().flatten()) {
            let fits = core.list.as_ref().is_some_and(|list| *list != other && items(list) == items(&other));
            if fits && !core.others.contains(&other) {
                core.others.push(other);
            }
        }
        self.prepare(&mut core, state, &key.2, &BTreeSet::new());
        self.cores.push(core);
        self.keys.insert(key, self.cores.len() - 1);
        self.cores.len() - 1
    }

    fn prepare(&mut self, core: &mut Core, state: &State, fixed: &[(usize, u8)], loose: &BTreeSet<usize>) {
        let _span = crate::profile::span("lines", "prepare");
        let (lv, s) = (self.lv, self.lv.s);
        let n = core.n;
        core.units = core.own.iter().map(|&k| Unit::Leaf(k)).chain(core.kids.iter().map(|&c| Unit::Block(c))).collect();
        core.ports =
            core.units.iter().filter(|u| matches!(u, Unit::Leaf(k) if s.objects[*k].role == Role::Port)).count();
        let started: Vec<(usize, u8)> = loose.iter().map(|&k| (k, code(state.ori[k]))).collect();
        let listed_key = (n, core.own.clone(), fixed.to_vec(), started);
        let (combos, classes, open_ways, total) = match self.listed.cached.get(&listed_key) {
            Some(x) => x.clone(),
            None if self.given.is_some() => {
                let (classes, open_ways) = lv.open_of(&core.own);
                (Rc::new(Vec::new()), classes, open_ways, 0)
            }
            None => {
                let (classes, ways) = lv.open_of(&core.own);
                let room = self.listed.room(combo_bytes(classes.len(), ways.len()));
                let (list, classes, open_ways, total) =
                    lv.combos(&core.own, &state.values, &state.ways, &state.ori, &state.settled, loose, room);
                let x = (Rc::new(list), classes, open_ways, total);
                self.listed.insert(listed_key, x.clone());
                if crate::profile::enabled() {
                    let (count, bytes) = self.listed.held();
                    crate::profile::max("listed_live_entries", count as u64);
                    crate::profile::max("listed_logical_bytes", bytes as u64);
                }
                x
            }
        };
        if total == u64::MAX {
            core.notes.push(format!(
                "lines: {}: {} combinations of orientations listed within the search and storage limits",
                lv.name(n),
                combos.len()
            ));
        } else if total > combos.len() as u64 {
            core.notes.push(format!(
                "lines: {}: {} of {total} combinations of orientations listed, the others ranked last by the rule never \
                 tried",
                lv.name(n),
                combos.len()
            ));
        }
        core.currents = lv.current_pairs(&core.units, &state.blocks);
        core.levels = lv.level_pairs(&core.own);

        core.classes = classes;
        core.open_ways = open_ways;
        core.combos = combos;
        if let Some(t) = self.given {
            let values: Vec<Orient> = core.classes.iter().map(|&c| t.values[c]).collect();
            let ways: Vec<usize> = core.open_ways.iter().map(|&i| t.ways[i]).collect();
            core.combos = Rc::new(vec![(values, ways, (0, 0, 0, 0, 0))]);
        }
        let mut rank: Vec<usize> = (0..core.combos.len()).collect();
        let learned = |i: usize| {
            let values: Vec<u8> = core.combos[i].0.iter().map(|&o| code(o)).collect();
            self.learned.get(&(n, values, core.combos[i].1.clone())).cloned()
        };
        let known: Vec<Option<Score>> = rank.iter().map(|&i| learned(i)).collect();
        rank.sort_by(|&i, &j| {
            let by = |x: &Option<Score>, y: &Option<Score>| match (x, y) {
                (Some(a), Some(b)) => a.cmp(b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            };
            by(&known[i], &known[j]).then(core.combos[i].2.cmp(&core.combos[j].2)).then(i.cmp(&j))
        });
        core.rank = rank;
        let mut inside: BTreeSet<usize> = core.own.iter().copied().collect();
        for &c in &core.kids {
            if let Some((b, _)) = state.blocks.get(&c) {
                inside.extend(b.at.keys().copied().filter(|k| lv.leaves.contains(k)));
            }
        }
        core.implied = lv.implied(&inside);
        core.bounds.clear();
        core.done.clear();
        core.results.clear();
        core.seen.clear();
        core.round = 0;
        core.queue.clear();
        core.spread.clear();
        core.exhausted = false;
    }

    fn next_choice(&mut self, c: usize) -> Option<Choice> {
        let batch = self.batch;
        let given = self.given.and_then(|t| t.gaps.get(&self.cores[c].n).cloned());
        let core = &mut self.cores[c];
        loop {
            while let Some(choice) = core.queue.pop_front() {
                if !core.done.contains(&choice) {
                    return Some(choice);
                }
            }
            while let Some(mut sp) = core.spread.pop_front() {
                let Some(bounds) = core.bounds.get(&sp.r) else { continue };
                let mut out: Vec<usize> = sp.out.iter().map(|&i| bounds.1[i]).collect();
                out.sort_unstable();
                let choice = (sp.r, sp.gaps.clone(), out);
                if spread_next(&mut sp, bounds) {
                    core.spread.push_front(sp);
                }
                if !core.done.contains(&choice) {
                    return Some(choice);
                }
            }
            if core.exhausted {
                return None;
            }
            if self.given.is_some() {
                core.exhausted = true;
                let (gaps, out) = given.clone().unwrap_or_else(|| (vec![0; core.ports + 4], Vec::new()));
                core.queue.push_back((0, gaps, out));
                continue;
            }
            let k = core.round;
            core.round += 1;
            let mut fresh: Vec<Choice> = Vec::new();
            let mut spread: Vec<(Option<Score>, Spread)> = Vec::new();
            let mut later = false;
            let slots = core.ports + 4;
            for (pos, &r) in core.rank.iter().enumerate() {
                let b = pos / batch;
                if b > k {
                    later = true;
                    continue;
                }
                let d = k - b;
                if d == 0 {
                    fresh.push((r, vec![0; slots], Vec::new()));
                    continue;
                }
                let Some(bounds) = core.bounds.get(&r) else { continue };
                let most: usize = bounds.0.iter().map(|b| b.saturating_sub(1)).sum::<usize>() + bounds.1.len();
                if d < most {
                    later = true;
                }
                if d > most {
                    continue;
                }
                let first = core.results.iter().find(|&&o| self.outcomes[o].choice == (r, vec![0; slots], Vec::new()));
                let score = first.map(|&o| self.outcomes[o].score.clone());
                if let Some(sp) = spread_first(r, d, bounds) {
                    spread.push((score, sp));
                }
            }
            spread.sort_by(|a, b| {
                let by = |x: &Option<Score>, y: &Option<Score>| match (x, y) {
                    (Some(p), Some(q)) => p.cmp(q),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                };
                by(&a.0, &b.0).then(a.1.r.cmp(&b.1.r))
            });
            core.queue.extend(fresh);
            core.spread.extend(spread.into_iter().map(|(_, sp)| sp));
            if core.queue.is_empty() && core.spread.is_empty() && !later {
                core.exhausted = true;
            }
        }
    }

    fn batch(&mut self, id: usize, size: usize, force: bool, work: &mut Work, stop: &Stop) -> Result<(), String> {
        let c = self.nodes[id].core;
        loop {
            // a trial stopped between its halves is still settled below: the solutions of its first
            // half are held in it, and the best may be among them
            let halfway = self.trials.get(&c).is_some_and(|t| t.tried.is_some());
            if crate::budget::stopped() && !halfway {
                return Ok(());
            }
            let first = self.cores[c].batches == 0;
            let made_before = self.cores[c].results.len();
            let (mut count, mut beyond, mut missed) = (0, 0, 0);
            while count < size && missed < size {
                let relaxed = RELAXED.with(|r| r.replace(0));
                crate::profile::add("relaxations_between", relaxed);
                self.tally.charge(work, Step::Relaxation, relaxed);
                let placed = !self.cores[c].results.is_empty();
                if stop.reached(work) {
                    if !force || placed || beyond >= size {
                        break;
                    }
                    beyond += 1;
                }
                let Some(choice) = self.next_choice(c) else { break };
                self.cores[c].done.insert(choice.clone());
                match self.evaluate(id, choice, work)? {
                    Came::Anew => (count, missed) = (count + 1, 0),
                    Came::Unplaced => missed += 1,
                    Came::Same => {}
                }
            }
            self.cores[c].batches += 1;
            if !first {
                return Ok(());
            }
            let best = self.cores[c].results[made_before..]
                .iter()
                .copied()
                .min_by(|&a, &b| self.outcomes[a].score.cmp(&self.outcomes[b].score).then(a.cmp(&b)));
            let root = self.nodes[id].step + 1 == self.steps.len();
            let mut best = best;
            if let Some(trial) = self.trials.remove(&c) {
                let mine: Vec<(usize, u64, usize)> = if root {
                    let all = std::mem::take(&mut self.solutions);
                    let (mine, rest) = all.into_iter().partition(|s| self.outcomes[s.0].core == c);
                    self.solutions = rest;
                    mine
                } else {
                    Vec::new()
                };
                match trial.tried {
                    None => {
                        let bands = std::mem::replace(&mut self.cores[c], trial.other);
                        let (made, now) = (trial.made, self.outcomes.len());
                        self.trials.insert(c, Trial { other: bands, tried: Some((best, mine, made..now)), made: now });
                        continue;
                    }
                    Some((theirs, solutions, made)) => {
                        let bands = match (theirs, best) {
                            (Some(a), Some(b)) => self.outcomes[a].score < self.outcomes[b].score,
                            (Some(a), None) => !self.outcomes[a].score.3,
                            _ => false,
                        };

                        let lost = if bands {
                            let dissolved = std::mem::replace(&mut self.cores[c], trial.other);
                            self.solutions.extend(solutions);
                            best = theirs;
                            (dissolved, trial.made..self.outcomes.len(), mine)
                        } else {
                            self.solutions.extend(mine);
                            (trial.other, made, solutions)
                        };

                        let kept = if bands { "the bands" } else { "the group dissolved" };
                        let n = self.cores[c].n;
                        self.cores[c].notes.push(format!(
                            "lines: {} tried both stretched in bands and dissolved : {kept} kept",
                            self.lv.name(n)
                        ));
                        let (mut other, results, solutions) = lost;
                        if !solutions.is_empty() {
                            let l = self.cores.len();
                            for o in results {
                                if self.outcomes[o].core == c {
                                    self.outcomes[o].core = l;
                                }
                            }
                            (other.exhausted, other.queue, other.spread) = (true, VecDeque::new(), VecDeque::new());
                            other.notes.push(format!(
                                "lines: {} tried both stretched in bands and dissolved : {} kept, \
                                 and a complete solution of {} taken, the better drawing",
                                self.lv.name(n),
                                kept,
                                if bands { "the group dissolved" } else { "the bands" }
                            ));
                            self.cores.push(other);
                            self.lost.extend(solutions);
                        }
                        if root {
                            self.best =
                                self.solutions.iter().map(|s| s.0).min_by(|&a, &b| {
                                    self.outcomes[a].score.cmp(&self.outcomes[b].score).then(a.cmp(&b))
                                });
                        }
                    }
                }
            }
            let Some(best) = best else { return Ok(()) };
            let dissolve = self.outcomes[best].tried.dissolve.clone();
            let stretch = self.outcomes[best].tried.stretch.clone();
            if stretch.is_empty() && (dissolve.is_empty() || self.cores[c].dissolved >= MOST_DISSOLVED) {
                return Ok(());
            }
            let state = self.nodes[id].state.clone();
            let mut core = std::mem::take(&mut self.cores[c]);
            let before = root.then(|| core.clone());
            let fixed: Vec<(usize, u8)> =
                self.keys.iter().find(|(_, &v)| v == c).map(|(k, _)| k.2.clone()).unwrap_or_default();
            if !stretch.is_empty() && core.dissolved < MOST_DISSOLVED {
                let mut other = core.clone();
                let blocks: BTreeSet<usize> = stretch.iter().map(|&(k, _)| k).chain(dissolve.iter().copied()).collect();
                self.dissolve(&mut other, &state, &blocks);
                other.batches = 0;
                self.prepare(&mut other, &state, &fixed, &BTreeSet::new());
                self.trials.insert(c, Trial { other, tried: None, made: self.outcomes.len() });
            }
            for &(k, a) in &stretch {
                core.banded.entry(k).or_default()[a] = true;
                core.notes.push(format!(
                    "lines: {} stretches in {} bands on {} in {}: a fact round it holds only so (decisions 8 and 26)",
                    self.lv.name(k),
                    state.blocks.get(&k).map_or(1, |b| b.0.cuts[a].len() + 1),
                    if a == 0 { "x" } else { "y" },
                    self.lv.name(core.n)
                ));
            }
            if stretch.is_empty() {
                let blocks: BTreeSet<usize> = dissolve.iter().copied().collect();
                self.dissolve(&mut core, &state, &blocks);
            }
            core.batches = 0;
            let (given, loose) = freed(&state, &core.own);
            self.prepare(&mut core, &given, &fixed, &loose);
            self.cores[c] = core;
            if root {
                let (old, rest): (Vec<_>, Vec<_>) =
                    std::mem::take(&mut self.solutions).into_iter().partition(|s| self.outcomes[s.0].core == c);
                self.solutions = rest;
                if let Some(mut before) = before.filter(|_| !old.is_empty()) {
                    let l = self.cores.len();
                    for s in &old {
                        self.outcomes[s.0].core = l;
                    }
                    (before.exhausted, before.queue, before.spread) = (true, VecDeque::new(), VecDeque::new());
                    before.notes.push(format!(
                        "lines: {} taken as it was before it dissolved or stretched what its best asked, as no complete \
                         solution made after can be drawn",
                        self.lv.name(before.n)
                    ));
                    self.cores.push(before);
                    self.fallback.extend(old);
                }
                self.best = self
                    .solutions
                    .iter()
                    .map(|s| s.0)
                    .min_by(|&a, &b| self.outcomes[a].score.cmp(&self.outcomes[b].score).then(a.cmp(&b)));
            }
        }
    }

    fn dissolve(&self, core: &mut Core, state: &State, blocks: &BTreeSet<usize>) {
        let mut held: Vec<usize> = core.own.clone();
        for &k in blocks {
            let inner = state.lists[k].clone();
            if let (Some(item), Some(with)) = (core.list.as_mut(), inner) {
                inline(item, k, &with);
                held.extend(state.own[k].iter().copied());
            }
            core.banded.remove(&k);
            core.notes.push(format!(
                "lines: {} dissolved into {}: what is round it must get into its box",
                self.lv.name(k),
                self.lv.name(core.n)
            ));
            core.dissolved += 1;
        }
        let (mut own, mut kids) = (Vec::new(), Vec::new());
        if let Some(item) = &core.list {
            walk(item, &mut own, &mut kids);
        }
        own.extend(held);
        own.sort_unstable();
        own.dedup();
        (core.own, core.kids) = (own, kids);
        core.others.clear();
    }

    fn evaluate(&mut self, id: usize, choice: Choice, work: &mut Work) -> Result<Came, String> {
        if !permit(
            BudgetStep::Candidate,
            self.lv.problem.objects.len().saturating_add(crate::annotation::estimated_work(self.lv.problem)),
        ) {
            return Ok(Came::Same);
        }
        let (lv, s, problem) = (self.lv, self.lv.s, self.lv.problem);
        let c = self.nodes[id].core;
        let root = self.nodes[id].step + 1 == self.steps.len();
        let state = &self.nodes[id].state;
        let core = &mut self.cores[c];
        let n = core.n;
        let combo = core.combos[choice.0].clone();
        let mut values = state.values.clone();
        for (j, &cl) in core.classes.iter().enumerate() {
            values[cl] = combo.0[j];
        }
        let mut ways = state.ways.clone();
        for (j, &i) in core.open_ways.iter().enumerate() {
            ways[i] = combo.1[j];
        }
        let full = orientations_given(s, &values, &ways);
        let mut ori = state.ori.clone();
        for &k in &core.own {
            ori[k] = full[k];
        }
        for &k in &core.own {
            for (j, _, rule) in lv.hung.get(&k).into_iter().flatten() {
                match rule {
                    Rule::Attach(a) => super::facing::turn(s, *j, a, &mut ori),
                    Rule::Pinned { .. } => crate::beside::follow(s, *j, &mut ori),
                    _ => {}
                }
            }
        }
        let span = crate::profile::span("lines", "choice");
        let list = core.table(choice.1.get(core.ports + 3).copied().unwrap_or(0)).cloned();
        let list = list.as_ref();
        let evaluated = lv.evaluate(
            n,
            &core.units,
            &ori,
            &state.blocks,
            &core.banded,
            &core.implied,
            &core.currents,
            &core.levels,
            root,
            (list, 1 + core.others.len()),
            &choice.1,
            &choice.2,
            &mut core.seen,
        );
        let relaxed = RELAXED.with(|r| r.replace(0));
        crate::profile::add("relaxations", relaxed);
        drop(span);
        let tally = &mut self.tally;
        tally.charge(work, Step::Choice, 1);
        tally.charge(work, Step::Relaxation, relaxed);
        tally.charge(work, Step::Route, ROUTED.with(|r| r.replace(0)));
        let first = !choice.1.iter().any(|&g| g > 0) && choice.2.is_empty();
        let t = match evaluated {
            Evaluated::Unplaced(steps, bounds) => {
                tally.charge(work, Step::Expansion, steps);
                let at = core.ports + 1;
                if bounds.as_ref().is_some_and(|b| b.0.get(at) == Some(&2)) && choice.1.get(at) == Some(&0) {
                    let mut gaps = choice.1.clone();
                    gaps[at] = 1;
                    let next = (choice.0, gaps, choice.2.clone());
                    if !core.done.contains(&next) {
                        core.queue.push_front(next);
                    }
                }
                if let (true, Some(bounds)) = (first, bounds) {
                    core.bounds.insert(choice.0, bounds);
                }
                return Ok(Came::Unplaced);
            }
            Evaluated::Same => return Ok(Came::Same),
            Evaluated::Done(t) => *t,
        };
        core.tried += 1;
        tally.charge(work, Step::Expansion, t.routed.expansions);
        let (combo_values, combo_ways) = (combo.0, combo.1);
        let units = core.units.clone();
        let own = core.own.clone();
        let kids = core.kids.clone();
        if first {
            core.bounds.insert(choice.0, (t.gaps.clone(), t.pairs.order.clone()));
        }
        let origins = s.origins(problem, &t.geometry);
        let placed = Placed {
            geometry: t.geometry.clone(),
            origins,
            dropped: lv.tree.dropped.clone(),
            overlaps: Vec::new(),
            optimised: true,
            undecided: false,
        };
        let routing = routing_of(problem, &t.routed, &t.refs);
        let within = (!root).then_some(&t.objects);
        let made = if root || self.given.is_none() {
            let holds = |f: usize| lv.fact_objects[f].is_subset(&t.objects);
            let part = within.map(|w| (w, &holds as &dyn Fn(usize) -> bool));
            let mut levels = t.levels.kept.clone();
            for unit in &units {
                if let Unit::Block(b) = unit {
                    levels.extend(state.blocks[b].0.levels.iter().copied());
                }
            }
            let span = crate::profile::span("lines", "merge");
            let (pivots, relaxations) = (work.pivots, work.astar_expansions);
            let mut made = compact_strict(problem, s, &placed, &routing, self.options, work, part, &levels);
            super::written::defaults(problem, s, &made.placed, &mut made.routing, within, false);
            let (pivots, relaxations) = (work.pivots - pivots, work.astar_expansions - relaxations);
            crate::profile::add("merge_pivots", pivots);
            crate::profile::add("merge_pivot_rows", made.pivot_rows);
            crate::profile::add("merge_relaxations", relaxations);
            drop(span);
            self.tally.charge(work, Step::Merge, 1);
            self.tally.charge(work, Step::MergePivotRow, made.pivot_rows);
            self.tally.charge(work, Step::MergeRelaxation, relaxations);
            Some(made)
        } else {
            None
        };

        let span = crate::profile::span("lines", "weigh");
        let lab = match &made {
            Some(m) => lab_order(problem, s, &m.placed, &m.routing, lv.weights, within),
            None => lab_order(problem, s, &placed, &routing, lv.weights, within),
        };
        drop(span);
        let failed = made.as_ref().is_some_and(|m| m.failed);
        let score: Score = (
            root && failed,
            t.score.0,
            t.score.1,
            failed,
            made.as_ref().map_or(0, |m| m.conflicts.len()),
            lab,
            t.score.4,
        );

        let state = &self.nodes[id].state;
        let block = (!root).then(|| Rc::new(lv.block(&units, &t, &state.blocks)));
        let drawn = |k: &usize| s.objects[*k].symbol.is_some();
        let mut all: Vec<usize> = own.iter().copied().filter(drawn).collect();
        for &k in &kids {
            if let Some((b, _)) = state.blocks.get(&k) {
                all.extend(b.at.keys().copied().filter(|j| lv.leaves.contains(j) && drawn(j)));
            }
        }
        all.sort_unstable();
        let group = GroupSettled { name: lv.name(n), own: own.iter().copied().filter(drawn).collect(), all };
        if first {
            let key = (n, combo_values.iter().map(|&o| code(o)).collect(), combo_ways.clone());
            let better = self.learned.get(&key).is_none_or(|b| score < *b);
            if better {
                self.learned.insert(key, score.clone());
            }
        }
        let o = self.outcomes.len();
        let inner = kids.iter().filter_map(|k| state.blocks.get(k)).map(|b| &self.outcomes[b.1].score.5);
        let (cb, lines) = inner.fold((0, 0), |(c, l), x| (c + x.cb, l + x.lines));
        let own = (score.5.cb.saturating_sub(cb), score.5.lines.saturating_sub(lines));
        if crate::budget::stopped() || score.5.interrupted {
            return Ok(Came::Same);
        }
        self.outcomes.push(Outcome {
            core: c,
            choice,
            tried: t,
            score,
            values: combo_values,
            ways: combo_ways,
            block,
            made: if root { made } else { None },
            group,
            own,
        });
        self.cores[c].results.push(o);
        if root {
            self.solutions.push((o, spent(work) - self.start, id));
            if self.best.is_none_or(|b| self.outcomes[o].score < self.outcomes[b].score) {
                self.best = Some(o);
            }
        }
        Ok(Came::Anew)
    }

    fn best_uncarried(&self, id: usize) -> Option<usize> {
        let node = &self.nodes[id];
        self.cores[node.core]
            .results
            .iter()
            .copied()
            .filter(|o| !node.carried.contains(o))
            .min_by(|&a, &b| self.outcomes[a].score.cmp(&self.outcomes[b].score).then(a.cmp(&b)))
    }

    fn carry(&mut self, id: usize, o: usize) -> usize {
        self.nodes[id].carried.insert(o);
        let mut state = self.nodes[id].state.clone();
        let out = &self.outcomes[o];
        let core = &self.cores[out.core];
        let n = core.n;
        for (j, &cl) in core.classes.iter().enumerate() {
            state.values[cl] = out.values[j];
        }
        for (j, &i) in core.open_ways.iter().enumerate() {
            state.ways[i] = out.ways[j];
        }
        for &k in &core.own {
            state.ori[k] = out.tried.ori[k];
            state.settled.insert(k);
            for (j, _, _) in self.lv.hung.get(&k).into_iter().flatten() {
                state.ori[*j] = out.tried.ori[*j];
            }
        }
        state.own[n].clone_from(&core.own);
        state.kids[n].clone_from(&core.kids);
        state.lists[n] = core.table(out.choice.1.get(core.ports + 3).copied().unwrap_or(0)).cloned();
        if let Some(b) = &out.block {
            state.blocks.insert(n, (b.clone(), o));
        }
        let mut path = self.nodes[id].path.clone();
        path.push(o);
        let step = self.nodes[id].step + 1;
        self.node(step, state, path)
    }

    fn best_path(&self) -> (Vec<usize>, Option<usize>) {
        let Some(b) = self.best else { return (Vec::new(), None) };
        let Some(&(_, _, id)) = self.solutions.iter().find(|s| s.0 == b) else { return (Vec::new(), None) };
        let mut path = self.nodes[id].path.clone();
        path.push(b);
        (path, Some(id))
    }

    fn pick(&self) -> Option<usize> {
        let best = self.best_path().0;
        let last = self.steps.len() - 1;
        let mut pick: Option<Rank> = None;
        for (id, node) in self.nodes.iter().enumerate() {
            let core = &self.cores[node.core];
            let left = !core.exhausted || !core.queue.is_empty() || !core.spread.is_empty();
            let uncarried = node.step < last && core.results.iter().any(|o| !node.carried.contains(o));
            if !left && !uncarried {
                continue;
            }
            let on = node.path.len() <= best.len() && best[..node.path.len()] == node.path[..];
            let here = if on { best.get(node.step).copied() } else { None };
            let weigh = here
                .or_else(|| {
                    core.results.iter().copied().min_by(|&a, &b| self.outcomes[a].score.cmp(&self.outcomes[b].score))
                })
                .map_or((0, 0), |o| self.outcomes[o].own);
            let key = (!on, node.visits, std::cmp::Reverse(weigh), node.step, id);
            if pick.as_ref().is_none_or(|p| key < *p) {
                pick = Some(key);
            }
        }
        pick.map(|p| p.4)
    }

    fn visit(&mut self, id: usize, work: &mut Work, stop: &Stop) -> Result<(), String> {
        self.nodes[id].visits += 1;

        let last = self.steps.len() - 1;
        let core = &self.cores[self.nodes[id].core];
        let left = !core.exhausted || !core.queue.is_empty() || !core.spread.is_empty();
        if self.nodes[id].step == last {
            return self.batch(id, self.batch, false, work, stop);
        }
        let uncarried = self.best_uncarried(id).is_some();
        if left && (!uncarried || self.nodes[id].visits % 2 == 1) {
            self.batch(id, self.batch, false, work, stop)?;
        }
        let mut id = id;
        while let Some(o) = self.best_uncarried(id) {
            id = self.carry(id, o);
            let c = self.nodes[id].core;
            if self.cores[c].batches == 0 {
                if stop.reached(work) {
                    return Ok(());
                }
                let n = self.cores[c].n;
                let known =
                    self.learned.range((n, Vec::new(), Vec::new())..(n + 1, Vec::new(), Vec::new())).next().is_some();
                let size = if known { (self.batch / PROBE).max(1) } else { self.batch };
                self.batch(id, size, false, work, stop)?;
            }
            if self.nodes[id].step == last {
                break;
            }
        }
        Ok(())
    }

    fn finish(mut self, exhausted: bool, work: &Work) -> Result<Settled, String> {
        let drawing = |o: usize| {
            let s = &self.outcomes[o].score;
            (s.0, s.3, s.2, s.4, &s.5, s.6)
        };
        let other = self.lost.iter().map(|s| s.0).min_by(|&a, &b| drawing(a).cmp(&drawing(b)).then(a.cmp(&b)));
        let better = |o: usize| self.best.is_none_or(|b| drawing(o) < drawing(b));
        if let Some(o) = other.filter(|&o| better(o)) {
            let lost = std::mem::take(&mut self.lost);
            self.solutions.extend(lost);
            self.best = Some(o);
        }
        if self.best.is_none_or(|b| self.outcomes[b].score.0) {
            let other = self
                .fallback
                .iter()
                .map(|s| s.0)
                .filter(|&o| !self.outcomes[o].score.0)
                .min_by(|&a, &b| self.outcomes[a].score.cmp(&self.outcomes[b].score).then(a.cmp(&b)));
            if let Some(o) = other {
                let fallback = std::mem::take(&mut self.fallback);
                self.solutions.extend(fallback);
                self.best = Some(o);
            }
        }
        let (lv, problem, s) = (self.lv, self.lv.problem, self.lv.s);
        let Some(best) = self.best else {
            return Err("the level's group could not be placed and routed in any orientation tried".to_string());
        };
        let (path, at) = self.best_path();
        let mut notes = Vec::new();
        let mut aligned = Vec::new();
        let mut groups = Vec::new();
        let state = &self.nodes[at.expect("the best was made at a node")].state;
        let mut taken =
            Taken { values: state.values.clone(), ways: state.ways.clone(), gaps: BTreeMap::new(), levels: lv.levels };
        let mut lists = state.lists.clone();
        let mut by_group: BTreeMap<usize, (usize, usize, usize)> = BTreeMap::new();
        for core in &self.cores {
            let e = by_group.entry(core.n).or_default();
            e.0 += core.tried;
            e.1 += core.batches;
            e.2 += 1;
        }
        for &o in &path {
            let out = &self.outcomes[o];
            let core = &self.cores[out.core];
            let n = core.n;
            for (j, &cl) in core.classes.iter().enumerate() {
                if cl < taken.values.len() {
                    taken.values[cl] = out.values[j];
                }
            }
            for (j, &i) in core.open_ways.iter().enumerate() {
                if i < taken.ways.len() {
                    taken.ways[i] = out.ways[j];
                }
            }
            taken.gaps.insert(n, (out.choice.1.clone(), out.choice.2.clone()));
            notes.extend(core.notes.iter().cloned());
            let table = out.choice.1.get(core.ports + 3).copied().unwrap_or(0);
            lists[n] = core.table(table).cloned();
            if table > 0 && table <= core.others.len() {
                notes.push(format!(
                    "lines: {}: placed by its list in another arrangement of the group-tree layout's search, number {} \
                     of {} (its first, then those it stepped through), not the one it took",
                    lv.name(n),
                    table,
                    core.others.len()
                ));
            }
            let port_gaps = &out.choice.1[..core.ports.min(out.choice.1.len())];
            let (tried, batches, settings) = by_group.get(&n).copied().unwrap_or_default();
            let t = &out.tried;
            let lab = &out.score.5;
            notes.push(format!(
                "lines: {}: {} placed, routed and merged in {} batch{}{}; the one taken leaves {} joins, {} crossings \
                 and {} bends routed, {} crossings and {} bends merged ({} weighed), {} lines of the wires{}",
                lv.name(n),
                crate::driver::count(tried, "choice"),
                batches,
                if batches == 1 { "" } else { "es" },
                if settings > 1 { format!(" over {settings} settings of what is in it") } else { String::new() },
                t.score.1,
                t.crossings,
                t.bends,
                lab.crossings,
                lab.bends,
                lab.cb,
                lab.lines,
                if port_gaps.iter().any(|&g| g > 0) {
                    format!(", its ports on gaps {port_gaps:?}")
                } else {
                    String::new()
                } + &if out.choice.2.is_empty() {
                    String::new()
                } else {
                    format!(", {} left unaligned", crate::driver::count(out.choice.2.len(), "facing pair"))
                }
            ));
            if t.hanging.count > 0 {
                notes.push(format!(
                    "lines: {}: {} with a coordinate no fact sets and a line it connects to: {} \
                     hung on it, {} refused by the order",
                    lv.name(n),
                    crate::driver::count(t.hanging.count, "part"),
                    t.hanging.hung,
                    t.hanging.refused
                ));
            }
            if t.pairs.count > 0 {
                notes.push(format!(
                    "lines: {}: {} of pins facing: {} aligned, {} refused by the order, {} passed over as \
                     a pin of them was aligned already, {} left unaligned by the choice",
                    lv.name(n),
                    crate::driver::count(t.pairs.count, "pair"),
                    t.pairs.kept,
                    t.pairs.refused,
                    t.pairs.passed,
                    t.pairs.left
                ));
            }
            if t.layering.count > 0 || t.layering.facts > 0 {
                notes.push(format!(
                    "lines: {}: {} of its items the current runs down through and {} that run either way on y \
                    , laid {}: {} refused by the order, {} not one above the other the way the current \
                     runs",
                    lv.name(n),
                    crate::driver::count(t.layering.count, "pair"),
                    crate::driver::count(t.layering.facts, "fact"),
                    LAYERINGS[t.layering.way],
                    t.layering.refused,
                    t.layering.against
                ));
            }
            if t.boxes.1 > 0 {
                notes.push(format!(
                    "lines: {}: the facts force the boxes of {} of {} pairs of neighbours of an ORDER to overlap along \
                     its axis; they overlap (language-0.4 item 2)",
                    lv.name(n),
                    t.boxes.1,
                    t.boxes.0
                ));
            }
            if t.levels.count > 0 {
                notes.push(format!(
                    "lines: {}: {} of its elements at one depth with the current from one net or to one, {}: {} refused by the order, {} not on one line",
                    lv.name(n),
                    crate::driver::count(t.levels.count, "pair"),
                    if t.levels.tried { "put on one line" } else { "left as they come" },
                    t.levels.refused,
                    t.levels.apart
                ));
            }
            if t.table_out > 0 {
                notes.push(format!(
                    "lines: {}: {} of its table left out of its placement, as the facts go against them",
                    lv.name(n),
                    crate::driver::count(t.table_out, "order")
                ));
            }
            if !t.left_out.is_empty() {
                let labels: Vec<&str> = t.left_out.iter().map(|&f| problem.facts[f].label.as_str()).collect();
                notes.push(format!(
                    "lines: {}: {} left out of its placement, as its order could not keep them: {}",
                    lv.name(n),
                    crate::driver::count(labels.len(), "fact"),
                    labels.join(", ")
                ));
            }
            aligned.extend(t.aligned.iter().copied());
            groups.push(out.group.clone());
        }
        let found = self.solutions.iter().position(|&(o, _, _)| o == best).unwrap_or(0);
        let tried: usize = self.cores.iter().map(|c| c.tried).sum();
        notes.push(format!(
            "lines: the search made {} from {} placed, routed and merged in {}, work {}; the best is solution {}, \
             made at work {}; {}",
            crate::driver::count(self.solutions.len(), "complete solution"),
            crate::driver::count(tried, "choice"),
            crate::driver::count(self.steps.len(), "group"),
            spent(work) - self.start,
            found + 1,
            self.solutions[found].1,
            if self.given.is_some() {
                "the choices were given".to_string()
            } else if crate::budget::current_limit().or(self.options.run_limit) == Some(crate::RunLimit::Seed) {
                "the first alone, as the search is to make its first candidates only".to_string()
            } else if exhausted {
                "every choice of every group in every setting was tried".to_string()
            } else {
                "stopped by the limit of work".to_string()
            }
        ));
        let unmade: Vec<&Outcome> = self.solutions.iter().map(|s| &self.outcomes[s.0]).filter(|o| o.score.0).collect();
        if !unmade.is_empty() {
            let mut facts: BTreeMap<usize, usize> = BTreeMap::new();
            for f in unmade.iter().filter_map(|o| o.made.as_ref()).flat_map(|m| &m.failed_facts) {
                *facts.entry(*f).or_default() += 1;
            }
            let said: Vec<String> = facts.iter().map(|(f, n)| format!("{} ({n})", fact_name(problem, *f))).collect();
            notes.push(format!(
                "lines: {} of {} had no solution of the coordinates of its drawing{}; {}",
                unmade.len(),
                crate::driver::count(self.solutions.len(), "complete solution"),
                if said.is_empty() {
                    String::new()
                } else {
                    format!(", the facts in their conflicts {}", said.join(", "))
                },
                if self.outcomes[best].score.0 { "none can be drawn" } else { "dropped, another taken " }
            ));
        }
        let mut outcomes = self.outcomes;
        let mut out = outcomes.swap_remove(best);
        let t = out.tried;

        let routing = routing_of(problem, &t.routed, &t.refs);
        let origins = s.origins(problem, &t.geometry);
        let overlaps = overlapping(s, &origins, &t.geometry.orient)
            .into_iter()
            .map(|(a, b)| Overlap { a, b, cause: Cause::Others })
            .collect();
        let placed = Placed {
            geometry: t.geometry,
            origins,
            dropped: lv.tree.dropped.clone(),
            overlaps,
            optimised: true,
            undecided: false,
        };
        let mut shown = BTreeSet::from([0]);
        for list in lists.iter().flatten() {
            let (mut own, mut kids) = (Vec::new(), Vec::new());
            walk(list, &mut own, &mut kids);
            shown.extend(kids);
        }
        for (n, list) in lists.iter_mut().enumerate() {
            if !shown.contains(&n) {
                *list = None;
            }
        }
        Ok(Settled {
            quality: out.score.5,
            placed,
            routing,
            routed: t.routed,
            notes,
            taken,
            aligned,
            groups,
            made: out.made.take(),
            lists,
        })
    }
}

pub(super) fn spread_first(r: usize, d: usize, bounds: &Bounds) -> Option<Spread> {
    (0..=d.min(bounds.1.len()))
        .find_map(|s| first_composition(&bounds.0, d - s).map(|gaps| Spread { r, d, s, gaps, out: (0..s).collect() }))
}

pub(super) fn spread_next(sp: &mut Spread, bounds: &Bounds) -> bool {
    let n = bounds.1.len();
    if next_combination(&mut sp.out, n) {
        return true;
    }
    if next_composition(&mut sp.gaps, &bounds.0) {
        sp.out = (0..sp.s).collect();
        return true;
    }
    for s in sp.s + 1..=sp.d.min(n) {
        if let Some(gaps) = first_composition(&bounds.0, sp.d - s) {
            (sp.s, sp.gaps, sp.out) = (s, gaps, (0..s).collect());
            return true;
        }
    }
    false
}

fn next_combination(c: &mut [usize], n: usize) -> bool {
    let s = c.len();
    for i in (0..s).rev() {
        if c[i] < n - s + i {
            c[i] += 1;
            for j in i + 1..s {
                c[j] = c[j - 1] + 1;
            }
            return true;
        }
    }
    false
}

pub(super) fn first_composition(bounds: &[usize], d: usize) -> Option<Vec<usize>> {
    let mut v = vec![0; bounds.len()];
    fill(&mut v, bounds, 0, d).then_some(v)
}

pub(super) fn next_composition(v: &mut [usize], bounds: &[usize]) -> bool {
    let d: usize = v.iter().sum();
    for i in (0..v.len().saturating_sub(1)).rev() {
        let before: usize = v[..i].iter().sum();
        if v[i] + 1 < bounds[i].max(1) && before + v[i] < d {
            let rest = d - before - v[i] - 1;
            let room: usize = bounds[i + 1..].iter().map(|b| b.saturating_sub(1)).sum();
            if rest <= room {
                v[i] += 1;
                return fill(v, bounds, i + 1, rest);
            }
        }
    }
    false
}

fn fill(v: &mut [usize], bounds: &[usize], from: usize, mut left: usize) -> bool {
    for j in (from..v.len()).rev() {
        v[j] = left.min(bounds[j].saturating_sub(1));
        left -= v[j];
    }
    left == 0
}

pub(super) fn fact_name(problem: &LevelProblem, f: usize) -> String {
    let fact = &problem.facts[f];
    let name = if fact.label.is_empty() { format!("fact {f}") } else { fact.label.clone() };
    match fact.chain.last().and_then(|&u| problem.usages.get(u)) {
        Some(u) if !u.label.is_empty() => format!("{name} of {}", u.label),
        _ => name,
    }
}

fn written_edges(problem: &LevelProblem, kept: &[usize]) -> Vec<(usize, Vec<Edge>)> {
    let mut first: [Option<(PointRef, Q)>; 2] = [None, None];
    let mut out = Vec::new();
    for &f in kept {
        let (r, axis, value) = match &problem.facts[f].kind {
            FactKind::Place { at, axis, value } => (at.clone(), axis, *value),
            FactKind::PointCoordinate { point, axis, value } => (PointRef::Origin(*point), axis, *value),
            _ => continue,
        };
        let (a, Some(v)) = (usize::from(*axis == Axis::Y), from_f64(value)) else { continue };
        match &first[a] {
            None => first[a] = Some((r, v)),
            Some((r0, v0)) => {
                let d = &v - v0;
                let edge = |from: &PointRef, to: &PointRef, gap: Q| Edge {
                    axis: a,
                    from: from.clone(),
                    to: to.clone(),
                    gap: Some(gap),
                    sides: (0, 0),
                };
                out.push((f, vec![edge(r0, &r, d.clone()), edge(&r, r0, -d)]));
            }
        }
    }
    out
}

fn written_objects(problem: &LevelProblem, kept: &[usize]) -> BTreeSet<usize> {
    let mut axes: BTreeMap<usize, [bool; 2]> = BTreeMap::new();
    for &f in kept {
        let (k, axis) = match &problem.facts[f].kind {
            FactKind::Place { at: PointRef::Origin(k), axis, .. } => (*k, axis),
            FactKind::PointCoordinate { point, axis, .. } => (*point, axis),
            _ => continue,
        };
        axes.entry(k).or_default()[usize::from(*axis == Axis::Y)] = true;
    }
    axes.into_iter().filter(|(_, a)| a[0] && a[1]).map(|(k, _)| k).collect()
}

fn open_on_y(kind: &FactKind) -> bool {
    match kind {
        FactKind::Order { axis, sense, .. }
        | FactKind::Distance { axis, sense, .. }
        | FactKind::Distribute { axis, sense, .. } => *axis == Axis::Y && *sense == Sense::Either,
        _ => false,
    }
}

fn fact_edges(
    problem: &LevelProblem,
    _s: &Structure,
    tree: &Placed,
    _arr: &Arrangement,
    f: usize,
    flip: bool,
) -> Vec<Edge> {
    fn plain(r: &PointRef) -> Option<PointRef> {
        match r {
            PointRef::Centre(ms) if ms.len() == 1 => plain(&ms[0]),
            other => Some(other.clone()),
        }
    }
    let all = |ms: &[PointRef]| -> Option<Vec<PointRef>> { ms.iter().map(plain).collect() };
    let was = |r: &PointRef, a: usize| point_value(problem, &tree.geometry, r, a, 0);
    let within = |a: usize, ms: &[PointRef]| -> Vec<Edge> {
        let mut out = Vec::new();
        for m in ms {
            let PointRef::Centre(inner) = m else { continue };
            let Some(points) = all(inner) else { continue };
            let at: Vec<(Q, usize)> = points.iter().enumerate().filter_map(|(i, p)| Some((was(p, a)?, i))).collect();
            let (Some(lo), Some(hi)) = (at.iter().min(), at.iter().max()) else { continue };
            out.push(Edge { axis: a, from: points[lo.1].clone(), to: m.clone(), gap: Some(Q::zero()), sides: (0, 0) });
            out.push(Edge { axis: a, from: m.clone(), to: points[hi.1].clone(), gap: Some(Q::zero()), sides: (0, 0) });
        }
        out
    };
    let ax = |a: &Axis| usize::from(*a == Axis::Y);
    let chain = |a: usize, ms: Vec<PointRef>, up: bool| -> Vec<Edge> {
        ms.windows(2)
            .map(|w| {
                let (p, q) = if up { (w[0].clone(), w[1].clone()) } else { (w[1].clone(), w[0].clone()) };
                Edge { axis: a, from: p, to: q, gap: Some(int(1)), sides: (0, 0) }
            })
            .collect()
    };
    let level = |a: usize, ms: Vec<PointRef>| -> Vec<Edge> {
        ms.iter()
            .skip(1)
            .map(|m| Edge { axis: a, from: ms[0].clone(), to: m.clone(), gap: None, sides: (0, 0) })
            .collect()
    };
    let rising = |a: usize, first: &PointRef, last: &PointRef| {
        flip != match (was(first, a), was(last, a)) {
            (Some(x), Some(y)) => y >= x,
            _ => true,
        }
    };
    match &problem.facts[f].kind {
        FactKind::Align { axis, members } => {
            let Some(ms) = all(members) else { return Vec::new() };
            let mut out = within(ax(axis), &ms);
            out.extend(level(ax(axis), ms));
            out
        }

        FactKind::Table { axis, rows } => rows
            .orders(*axis)
            .into_iter()
            .map(|(axis, from, to)| {
                let (from, to) = if axis == Axis::Y { (to, from) } else { (from, to) };
                Edge { axis: ax(&axis), from: from.clone(), to: to.clone(), gap: Some(int(1)), sides: (0, 0) }
            })
            .collect(),
        FactKind::Order { axis, sense, members } => {
            let (a, Some(ms)) = (ax(axis), all(members)) else { return Vec::new() };
            if ms.len() < 2 {
                return Vec::new();
            }
            let up = match sense {
                Sense::Increasing => true,
                Sense::Decreasing => false,
                Sense::Either => rising(a, &ms[0], &ms[ms.len() - 1]),
            };
            let span = |r: &PointRef| -> Option<((PointRef, i8), (PointRef, i8))> {
                match r {
                    PointRef::Centre(inner) => {
                        let points = all(inner)?;
                        let at: Vec<(Q, usize)> =
                            points.iter().enumerate().filter_map(|(i, p)| Some((was(p, a)?, i))).collect();
                        let (lo, hi) = (at.iter().min()?, at.iter().max()?);
                        Some(((points[lo.1].clone(), 0), (points[hi.1].clone(), 0)))
                    }
                    other => Some(((other.clone(), 0), (other.clone(), 0))),
                }
            };
            let Some(spans) = ms.iter().map(span).collect::<Option<Vec<_>>>() else { return Vec::new() };
            let mut out = Vec::new();
            for w in spans.windows(2) {
                let (first, next) = if up { (&w[0], &w[1]) } else { (&w[1], &w[0]) };
                let points = first.1 .1 == 0 && next.0 .1 == 0;
                out.push(Edge {
                    axis: a,
                    from: first.1 .0.clone(),
                    to: next.0 .0.clone(),
                    gap: Some(if points { int(1) } else { Q::zero() }),
                    sides: (first.1 .1, next.0 .1),
                });
            }
            out
        }
        FactKind::Distribute { axis, sense, members } => {
            let (a, Some(ms)) = (ax(axis), all(members)) else { return Vec::new() };
            if ms.len() < 2 {
                return Vec::new();
            }
            let up = match sense {
                Sense::Increasing => true,
                Sense::Decreasing => false,
                Sense::Either => rising(a, &ms[0], &ms[ms.len() - 1]),
            };
            let mut out = within(a, &ms);
            out.extend(chain(a, ms, up));
            out
        }
        FactKind::Distance { axis, sense, min, from, to, .. } => {
            let (a, Some(fs), Some(ts)) = (ax(axis), all(from), all(to)) else { return Vec::new() };
            let up = match sense {
                Sense::Increasing => true,
                Sense::Decreasing => false,
                Sense::Either => {
                    let mean = |rs: &[PointRef]| -> Option<f64> {
                        let vs: Vec<f64> = rs.iter().filter_map(|r| was(r, a)).map(|v| to_f64(&v)).collect();
                        (!vs.is_empty()).then(|| vs.iter().sum::<f64>() / vs.len() as f64)
                    };
                    flip != mean(&ts).zip(mean(&fs)).is_none_or(|(t, f)| t >= f)
                }
            };
            let least = min.and_then(from_f64).unwrap_or_else(|| int(1));
            let mut out = within(a, &fs);
            out.extend(within(a, &ts));
            for p in &fs {
                for q in &ts {
                    let (x, y) = if up { (p.clone(), q.clone()) } else { (q.clone(), p.clone()) };
                    out.push(Edge { axis: a, from: x, to: y, gap: Some(least.clone()), sides: (0, 0) });
                }
            }
            out
        }
        _ => Vec::new(),
    }
}

fn inline(item: &mut Item, c: usize, with: &Item) {
    match item {
        Item::Group(g) if *g == c => *item = with.clone(),
        Item::List(_, items) => items.iter_mut().for_each(|i| inline(i, c, with)),
        _ => {}
    }
}

fn walk(item: &Item, own: &mut Vec<usize>, kids: &mut Vec<usize>) {
    match item {
        Item::Leaf(k) => own.push(*k),
        Item::Group(c) => kids.push(*c),
        Item::List(_, items) => items.iter().for_each(|i| walk(i, own, kids)),
    }
}

fn hanging(
    problem: &LevelProblem,
    s: &Structure,
    tree: &Placed,
    leaves: &BTreeSet<usize>,
) -> BTreeMap<usize, Vec<(usize, usize, Rule)>> {
    let ori = &tree.geometry.orient;
    let attach: BTreeMap<usize, Attach> =
        attachments(s, &[], ori).into_iter().filter(|(k, _)| !leaves.contains(k)).collect();
    let at = &tree.origins;
    let dist = |a: usize, b: usize| (&at[a][0] - &at[b][0]).abs() + (&at[a][1] - &at[b][1]).abs();
    let written = written_objects(problem, &s.kept);
    let fixed: Vec<usize> = leaves.iter().copied().filter(|k| written.contains(k)).collect();
    let nearest = |k: usize| {
        let pool: Vec<usize> =
            if written.contains(&k) && !fixed.is_empty() { fixed.clone() } else { leaves.iter().copied().collect() };
        pool.into_iter().min_by(|&a, &b| dist(k, a).cmp(&dist(k, b)).then(a.cmp(&b)))
    };
    let mut from: BTreeMap<usize, (usize, Rule)> = BTreeMap::new();
    for (k, info) in s.objects.iter().enumerate() {
        if leaves.contains(&k) || !info.own || info.left_free {
            continue;
        }
        if !matches!(problem.objects[k].position, Position::Own { .. }) {
            continue;
        }
        let beyond = s.anchors.get(&k).and_then(|a| {
            let &[(object, Some(pin))] = a.from.as_slice() else { return None };
            let text = problem.objects[k].text.as_ref()?;
            if object == k
                || !matches!(
                    text.attach,
                    rnl_runtime::problem::Filler::Terminal { .. } | rnl_runtime::problem::Filler::Pin { .. }
                )
            {
                return None;
            }
            let b = text_box(text, &(int(0), int(0)), Orient::R0)?;
            Some((object, pin, b))
        });
        let rule = if let Some(stub) = &info.stub {
            Some((stub.host.0, Rule::Stub { pin: stub.host.1, rise: stub.rise.clone() }))
        } else if let Some(&((o, pin), own)) = s.pinned.get(&k) {
            Some((o, Rule::Pinned { pin, own }))
        } else if let Some((o, pin, b)) = beyond {
            let keep = [&at[k][0] - &at[o][0], &at[k][1] - &at[o][1]];
            Some((o, Rule::Beyond { pin, b, keep }))
        } else {
            attach.get(&k).map(|a| (a.host.0, Rule::Attach(*a)))
        };
        let (host, rule) = match rule {
            Some(r) => r,
            None => {
                let Some(h) = nearest(k) else { continue };
                (h, Rule::Keep([&at[k][0] - &at[h][0], &at[k][1] - &at[h][1]]))
            }
        };
        from.insert(k, (host, rule));
    }
    let mut out: BTreeMap<usize, Vec<(usize, usize, Rule)>> = BTreeMap::new();
    let mut depth: Vec<(usize, usize, usize)> = Vec::new();
    let objects: Vec<usize> = from.keys().copied().collect();
    for k in objects {
        let (mut h, mut d) = (k, 0);
        while let Some((next, _)) = from.get(&h) {
            h = *next;
            d += 1;
            if d > 8 {
                break;
            }
        }
        if leaves.contains(&h) {
            depth.push((d, k, h));
        } else if let Some(l) = nearest(k) {
            from.insert(k, (l, Rule::Keep([&at[k][0] - &at[l][0], &at[k][1] - &at[l][1]])));
            depth.push((1, k, l));
        }
    }
    depth.sort_unstable();
    for (_, k, leaf) in depth {
        let (via, rule) = from[&k].clone();
        out.entry(leaf).or_default().push((k, via, rule));
    }
    out
}

fn hung_offset(s: &Structure, k: usize, via: usize, rule: &Rule, ori: &[Orient]) -> Option<[Q; 2]> {
    match rule {
        Rule::Stub { pin, rise } => {
            let hp = s.pin_offset(via, *pin, ori[via])?;
            let sp = s.pin_offset(k, 0, ori[k])?;
            Some([&hp[0] - &sp[0], &hp[1] + rise - &sp[1]])
        }
        Rule::Pinned { pin, own } => {
            let hp = s.pin_offset(via, *pin, ori[via])?;
            let op = s.pin_offset(k, *own, ori[k])?;
            Some([&hp[0] - &op[0], &hp[1] - &op[1]])
        }
        Rule::Attach(a) => attach_offset(s, k, &super::facing::now(s, k, a, ori), ori),
        Rule::Keep(off) => Some(off.clone()),
        Rule::Beyond { pin, b, keep } => {
            let at = s.pin_offset(via, *pin, ori[via])?;
            let (x, y) = s.exit(via, *pin, ori[via])?;
            let gap = crate::rational::frac(7, 20);
            let along = |way: i64, low: &Q, high: &Q| match way.signum() {
                1 => &gap - low,
                -1 => -&gap - high,
                _ => -(low + high) / int(2),
            };
            let off = [along(x, &b[0], &b[2]), along(y, &b[1], &b[3])];
            Some([0, 1].map(|a| if s.objects[k].fact_axes[a] { keep[a].clone() } else { &at[a] + &off[a] }))
        }
    }
}

fn unite(b: &mut Option<[Q; 4]>, x: &[Q; 4]) {
    *b = Some(match b.take() {
        None => x.clone(),
        Some(y) => [
            y[0].clone().min(x[0].clone()),
            y[1].clone().min(x[1].clone()),
            y[2].clone().max(x[2].clone()),
            y[3].clone().max(x[3].clone()),
        ],
    });
}

fn moved(b: &[Q; 4], d: &[Q; 2]) -> [Q; 4] {
    [&b[0] + &d[0], &b[1] + &d[1], &b[2] + &d[0], &b[3] + &d[1]]
}

impl Level<'_> {
    fn name(&self, n: usize) -> String {
        match self.nodes.nodes[n].kind {
            NodeKind::Root => "the level".to_string(),
            NodeKind::Group(g) => {
                let label = self.problem.groups.get(g).map_or("", |g| g.label.as_str());
                if label.starts_with('%') {
                    format!("group {label}")
                } else {
                    let ls: Vec<&str> =
                        self.nodes.nodes[n].members.iter().map(|&k| self.problem.objects[k].label.as_str()).collect();
                    format!("group ({})", ls.join(" "))
                }
            }
            _ => format!("part {n} of the groups"),
        }
    }

    fn implied(&self, inside: &BTreeSet<usize>) -> Vec<Edge> {
        let mut out = Vec::new();
        for a in 0..2 {
            let mut refs: Vec<(PointRef, i8)> = Vec::new();
            let mut adj: Vec<Vec<(usize, Q)>> = Vec::new();
            for &f in &self.s.kept {
                for e in self.edges[f].iter().filter(|e| e.axis == a) {
                    let mut id = |r: &PointRef, side: i8| -> usize {
                        if let Some(i) = refs.iter().position(|(q, s)| *s == side && same_point(q, r)) {
                            return i;
                        }
                        refs.push((r.clone(), side));
                        adj.push(Vec::new());
                        refs.len() - 1
                    };
                    let (u, v) = (id(&e.from, e.sides.0), id(&e.to, e.sides.1));
                    match &e.gap {
                        None => {
                            adj[u].push((v, Q::zero()));
                            adj[v].push((u, Q::zero()));
                        }
                        Some(g) => adj[u].push((v, g.clone())),
                    }
                }
            }
            let n = refs.len();
            let is_in: Vec<bool> = refs
                .iter()
                .map(|(r, _)| !matches!(r, PointRef::Centre(_)) && inside.contains(&ref_object(r)))
                .collect();
            for p in (0..n).filter(|&p| is_in[p]) {
                let mut d: Vec<Option<Q>> = vec![None; n];
                let mut count = vec![0usize; n];
                let mut queue: VecDeque<usize> = VecDeque::new();
                let mut cycle = false;
                for (v, w) in &adj[p] {
                    if !is_in[*v] && d[*v].as_ref().is_none_or(|x| w > x) {
                        d[*v] = Some(w.clone());
                        queue.push_back(*v);
                    }
                }
                while let Some(u) = queue.pop_front() {
                    let du = d[u].clone().expect("reached");
                    RELAXED.with(|r| r.set(r.get() + adj[u].len() as u64));
                    for (v, w) in &adj[u] {
                        let c = &du + w;
                        if *v == p || d[*v].as_ref().is_some_and(|x| c <= *x) {
                            continue;
                        }
                        d[*v] = Some(c);
                        if !is_in[*v] {
                            count[*v] += 1;
                            if count[*v] > n + 1 {
                                cycle = true;
                                break;
                            }
                            queue.push_back(*v);
                        }
                    }
                    if cycle {
                        break;
                    }
                }
                if cycle {
                    continue;
                }
                for q in (0..n).filter(|&q| is_in[q] && q != p) {
                    if let Some(w) = &d[q] {
                        out.push(Edge {
                            axis: a,
                            from: refs[p].0.clone(),
                            to: refs[q].0.clone(),
                            gap: Some(w.clone()),
                            sides: (refs[p].1, refs[q].1),
                        });
                    }
                }
            }
        }
        out
    }

    fn boxed(&self, e: &Edge) -> Option<Edge> {
        let boxed = |r: &PointRef| matches!(r, PointRef::Origin(k) if self.problem.objects.get(*k).is_some_and(|o| o.symbol.is_some() && matches!(o.position, Position::Own { .. })));
        let (from, to) = (boxed(&e.from), boxed(&e.to));
        if !from && !to {
            return None;
        }
        Some(Edge {
            axis: e.axis,
            from: e.from.clone(),
            to: e.to.clone(),
            gap: Some(Q::zero()),
            sides: (if from { 1 } else { 0 }, if to { -1 } else { 0 }),
        })
    }

    fn down_the_current(&self, edges: &[Edge]) -> i64 {
        let Some(depth) = self.s.depth.as_ref() else { return 0 };
        let object = |r: &PointRef| match r {
            PointRef::Origin(k)
            | PointRef::Pin { object: k, .. }
            | PointRef::SymbolPin { object: k, .. }
            | PointRef::PinPosition { object: k, .. } => Some(*k),
            PointRef::Centre(_) => None,
        };
        edges
            .iter()
            .filter(|e| e.axis == 1 && e.gap.is_some())
            .filter_map(|e| {
                let (low, high) = (object(&e.from)?, object(&e.to)?);
                let (dl, dh) = (depth.get(low).copied().flatten()?, depth.get(high).copied().flatten()?);
                (low != high).then(|| match dh.cmp(&dl) {
                    std::cmp::Ordering::Less => 1,
                    std::cmp::Ordering::Greater => -1,
                    std::cmp::Ordering::Equal => 0,
                })
            })
            .sum()
    }

    fn current_pairs(&self, units: &[Unit], blocks: &BTreeMap<usize, (Rc<Block>, usize)>) -> Vec<(usize, usize)> {
        let s = self.s;
        let Some(depth) = s.depth.as_ref().filter(|_| s.has(Source::DepthRows)) else { return Vec::new() };
        let mut unit_of: BTreeMap<usize, usize> = BTreeMap::new();
        for (u, unit) in units.iter().enumerate() {
            match unit {
                Unit::Leaf(k) => {
                    unit_of.insert(*k, u);
                }
                Unit::Block(c) => {
                    for &k in blocks.get(c).map(|b| b.0.at.keys()).into_iter().flatten() {
                        unit_of.insert(k, u);
                    }
                }
            }
        }
        let mut span: BTreeMap<usize, (u32, u32)> = BTreeMap::new();
        for (&k, &u) in &unit_of {
            if let Some(d) = depth.get(k).copied().flatten().filter(|_| s.objects[k].role == Role::Element) {
                let e = span.entry(u).or_insert((d, d));
                *e = (e.0.min(d), e.1.max(d));
            }
        }
        let mut below: BTreeSet<(usize, usize)> = BTreeSet::new();
        for net in &s.connections.nets {
            let marked = |out: bool| {
                net.pins
                    .iter()
                    .filter(move |p| s.objects[p.0].role == Role::Element && s.currents.get(p) == Some(&out))
                    .filter_map(|p| unit_of.get(&p.0).copied())
            };
            for a in marked(true) {
                below.extend(marked(false).filter(|&b| b != a).map(|b| (a, b)));
            }
        }
        below
            .into_iter()
            .filter(|(a, b)| matches!((span.get(a), span.get(b)), (Some(x), Some(y)) if x.1 < y.0))
            .collect()
    }

    fn level_pairs(&self, own: &[usize]) -> Vec<(PinKey, PinKey)> {
        let s = self.s;
        let Some(depth) = s.depth.as_ref().filter(|_| s.has(Source::DepthRows) && self.levels) else {
            return Vec::new();
        };
        let own: BTreeSet<usize> = own.iter().copied().filter(|&k| s.objects[k].role == Role::Element).collect();
        let mut done: BTreeSet<(usize, usize)> = BTreeSet::new();
        let mut out = Vec::new();
        for marked in [false, true] {
            for net in &s.connections.nets {
                let mut at: BTreeMap<u32, Vec<PinKey>> = BTreeMap::new();
                for p in net.pins.iter().filter(|p| own.contains(&p.0) && s.currents.get(*p) == Some(&marked)) {
                    if let Some(d) = depth.get(p.0).copied().flatten() {
                        at.entry(d).or_default().push(*p);
                    }
                }
                for pins in at.values_mut() {
                    pins.sort_unstable();
                    pins.dedup_by_key(|p| p.0);
                    for w in pins.windows(2) {
                        if done.insert((w[0].0, w[1].0)) {
                            out.push((w[0], w[1]));
                        }
                    }
                }
            }
        }
        out
    }

    fn open_of(&self, own_leaves: &[usize]) -> (Vec<usize>, Vec<usize>) {
        let (problem, s) = (self.problem, self.s);
        let own: BTreeSet<usize> = own_leaves.iter().copied().collect();
        let classes: Vec<usize> = (0..s.open.classes.len())
            .filter(|&c| s.open.classes[c].members.iter().any(|(m, _)| own.contains(m)))
            .collect();
        let open_ways: Vec<usize> = (0..s.open.ways.len())
            .filter(|&i| match &problem.facts[s.open.ways[i].fact].kind {
                FactKind::OrientRelation { from, to, .. } => from.iter().chain(to).any(|k| own.contains(k)),
                FactKind::Orient { object, .. } => own.contains(object),
                FactKind::PinOrient { pin, .. } => own.contains(&pin.object),
                _ => false,
            })
            .collect();
        (classes, open_ways)
    }

    #[allow(clippy::too_many_arguments)]
    fn combos(
        &self,
        own_leaves: &[usize],
        values: &[Orient],
        ways: &[usize],
        ori_now: &[Orient],
        settled: &BTreeSet<usize>,
        loose: &BTreeSet<usize>,
        room: usize,
    ) -> (Vec<Combo>, Vec<usize>, Vec<usize>, u64) {
        let (problem, s) = (self.problem, self.s);
        let own: BTreeSet<usize> = own_leaves.iter().copied().collect();
        let (classes, open_ways) = self.open_of(own_leaves);
        let limit = room.min(LISTED).min(LIST_BYTES / combo_bytes(classes.len(), open_ways.len()));
        if limit == 0 {
            return (Vec::new(), classes, open_ways, u64::MAX);
        }
        let chosen = |i: usize| match &problem.facts[s.open.ways[i].fact].kind {
            FactKind::OrientRelation { from, to, .. } => {
                let mut of = from.iter().chain(to).filter(|k| own.contains(k)).peekable();
                of.peek().is_some() && of.all(|k| loose.contains(k))
            }
            FactKind::Orient { object, .. } => own.contains(object) && loose.contains(object),
            FactKind::PinOrient { pin, .. } => own.contains(&pin.object) && loose.contains(&pin.object),
            _ => false,
        };
        let free: Vec<usize> = open_ways.iter().copied().filter(|&i| !chosen(i)).collect();
        let (mut way_sets, mut cut) = way_sets(s, ways, &free);
        let free_ways = way_sets.clone();
        'turned: for &i in open_ways.iter().filter(|&&i| chosen(i)) {
            for w in &free_ways {
                for t in (0..s.open.ways[i].count).filter(|&t| t != ways[i]) {
                    if way_sets.len() >= WAY_SETS || !permit(BudgetStep::Structure, s.objects.len()) {
                        cut = true;
                        break 'turned;
                    }
                    let mut w = w.clone();
                    w[i] = t;
                    way_sets.push(w);
                }
            }
        }
        let way_of: BTreeMap<usize, usize> = s.open.ways.iter().enumerate().map(|(i, way)| (way.fact, i)).collect();
        let fits = |ori: &[Orient], members: &[(usize, Orient)]| {
            members.iter().all(|&(m, _)| !settled.contains(&m) || ori[m] == ori_now[m])
        };
        let turned_up = |m: usize, ori: &[Orient]| {
            s.objects[m].role == Role::Port && s.exit(m, 0, ori[m]).is_some_and(|e| e.1 != 0)
        };
        let upright = |ori: &[Orient]| own.iter().filter(|&&m| turned_up(m, ori)).count();
        let in_column = |m: usize| {
            s.kept.iter().any(|&f| match &problem.facts[f].kind {
                FactKind::Align { axis: Axis::X, members } => {
                    members.len() >= 2 && members.iter().any(|r| refers_to(r, m))
                }
                _ => false,
            })
        };
        let drawn = |m: usize, o: Orient| -> Vec<Q> {
            let mut out = Vec::new();
            for p in 0..s.pins(m).len() {
                out.extend(s.pin_offset(m, p, o).into_iter().flatten());
            }
            out.extend(s.box_offset(m, o).into_iter().flatten());
            out
        };
        let mut out: Vec<Combo> = Vec::new();
        let mut seen: BTreeSet<Fingerprint> = BTreeSet::new();
        let mut total = 0u64;
        let mut unlisted = 0u64;
        'listing: for w in &way_sets {
            if out.len() >= limit || !permit(BudgetStep::Structure, s.objects.len()) {
                cut = true;
                break;
            }
            if !ways_fit(s, &way_of, w) {
                continue;
            }
            let mut options: Vec<Vec<(Orient, u32, bool, u8)>> = Vec::new();
            let mut home: Vec<Option<usize>> = Vec::new();
            for &c in &classes {
                let class = &s.open.classes[c];
                let freed: Vec<usize> = class.members.iter().map(|&(m, _)| m).filter(|m| loose.contains(m)).collect();
                let mut at: Option<usize> = None;
                let mut vs: Vec<(Orient, u32, bool, u8)> = Vec::new();
                let mut drawn_as: BTreeMap<Vec<Vec<Q>>, usize> = BTreeMap::new();
                let mut dom = current_order(s, class);
                dom.sort_by_key(|(v, r)| (*r, ORDER.iter().position(|o| o == v)));
                for (v, current) in dom {
                    if !super::cost::listing(s.objects.len()) {
                        cut = true;
                        break 'listing;
                    }
                    let mut trial = values.to_vec();
                    trial[c] = v;
                    let ori = orientations_given(s, &trial, w);
                    if !fits(&ori, &class.members)
                        || !allowed(problem, s, class.members.iter().map(|&(m, _)| (m, ori[m])))
                    {
                        continue;
                    }
                    let look: Vec<Vec<Q>> = class
                        .members
                        .iter()
                        .filter(|(m, _)| !settled.contains(m))
                        .map(|&(m, _)| drawn(m, ori[m]))
                        .collect();
                    let i = match drawn_as.get(&look).copied() {
                        Some(i) => i,
                        None => {
                            let up = class
                                .members
                                .iter()
                                .any(|&(m, _)| own.contains(&m) && turned_up(m, &ori) && !in_column(m));
                            vs.push((v, self.rule_cost(&class.members, &own, &ori, false), up, current));
                            drawn_as.insert(look, vs.len() - 1);
                            vs.len() - 1
                        }
                    };
                    if !freed.is_empty() && at.is_none() && freed.iter().all(|&m| ori[m] == ori_now[m]) {
                        at = Some(i);
                    }
                }
                home.push((!freed.is_empty()).then(|| at.unwrap_or(0)));
                options.push(vs);
            }
            let mut turns: Vec<(usize, usize)> = (0..options.len())
                .filter_map(|j| home[j].map(|h| (j, h)))
                .flat_map(|(j, h)| (0..options[j].len()).filter(move |&i| i != h).map(move |i| (j, i)))
                .collect();
            turns.sort_by_key(|&(j, i)| (options[j][i].3, options[j][i].1, j, i));
            let mut moves: Vec<Option<(usize, usize)>> = vec![None];
            moves.extend(turns.into_iter().map(Some));
            let size = |o: &[Vec<(Orient, u32, bool, u8)>]| {
                combination_count((0..o.len()).filter(|&j| home[j].is_none()).map(|j| o[j].len()))
            };
            unlisted = unlisted
                .saturating_add(size(&options).saturating_mul(moves.len() as u64).saturating_sub(LISTED as u64));
            while size(&options).saturating_mul(moves.len() as u64) > LISTED as u64 {
                let Some(c) = (0..options.len())
                    .filter(|&c| home[c].is_none() && options[c].len() > 1)
                    .max_by_key(|&c| options[c].len())
                else {
                    moves.truncate(((LISTED as u64 / size(&options)).max(1)) as usize);
                    break;
                };
                let worst =
                    (0..options[c].len()).max_by_key(|&i| (options[c][i].3, options[c][i].1, i)).expect("values");
                options[c].remove(worst);
            }
            if options.iter().any(|o| o.is_empty()) {
                continue;
            }
            let late: Vec<Vec<bool>> = options
                .iter()
                .map(|vs| {
                    let level = vs.iter().any(|v| !v.2);
                    vs.iter().map(|v| v.2 && level).collect()
                })
                .collect();
            let mut index: Vec<usize> = home.iter().map(|h| h.unwrap_or(0)).collect();
            let mut m = 0;
            loop {
                if out.len() >= limit || !super::cost::listing(s.objects.len()) {
                    cut = true;
                    break 'listing;
                }
                let mut pick = index.clone();
                if let Some((j, i)) = moves[m] {
                    pick[j] = i;
                }
                let mut trial = values.to_vec();
                let (mut lates, mut against, mut sideways) = (0usize, 0usize, 0usize);
                for (j, &c) in classes.iter().enumerate() {
                    trial[c] = options[j][pick[j]].0;
                    lates += usize::from(late[j][pick[j]]);
                    against += usize::from(options[j][pick[j]].3 == 2);
                    sideways += usize::from(options[j][pick[j]].3 == 1);
                }
                let ori = orientations_given(s, &trial, w);
                let every: Vec<(usize, Orient)> = (0..ori.len()).map(|m| (m, ori[m])).collect();
                // List-valued ORIENT facts may have no free class at all, while
                // ORIENT-RELATION alternatives change a class's member labels.
                if fits(&ori, &every) && allowed(problem, s, own_leaves.iter().map(|&m| (m, ori[m]))) {
                    let mut look = Look::new();
                    for &m in own_leaves {
                        look.row(&drawn(m, ori[m]));
                    }
                    if seen.insert(look.finish()) {
                        total += 1;
                        let joint = self.rule_cost(&every, &own, &ori, true);
                        out.push((
                            classes.iter().map(|&c| trial[c]).collect(),
                            open_ways.iter().map(|&i| w[i]).collect(),
                            (lates, against, sideways, joint, upright(&ori)),
                        ));
                    }
                }
                let mut j = 0;
                while j < index.len() {
                    if home[j].is_some() {
                        j += 1;
                        continue;
                    }
                    index[j] += 1;
                    if index[j] < options[j].len() {
                        break;
                    }
                    index[j] = 0;
                    j += 1;
                }
                if j == index.len() {
                    m += 1;
                    if m == moves.len() {
                        break;
                    }
                }
            }
        }
        (out, classes, open_ways, if cut { u64::MAX } else { total.saturating_add(unlisted) })
    }

    fn nearest_pins(&self) -> BTreeMap<(usize, usize), (usize, usize)> {
        let (problem, s, tree) = (self.problem, self.s, self.tree);
        let at = |k: usize, p: usize| -> Option<(f64, f64)> {
            let off = s.pin_offset(k, p, tree.geometry.orient[k])?;
            Some((to_f64(&(&tree.origins[k][0] + &off[0])), to_f64(&(&tree.origins[k][1] + &off[1]))))
        };
        let mut out = BTreeMap::new();
        for r in &problem.routes {
            let pins: Vec<(usize, usize)> = r.pins.iter().filter_map(ref_pin).collect();
            for &(k, p) in &pins {
                let Some(here) = at(k, p) else { continue };
                let near = pins
                    .iter()
                    .filter(|(q, _)| *q != k)
                    .filter_map(|&(q, qp)| {
                        let x = at(q, qp)?;
                        Some(((x.0 - here.0).abs() + (x.1 - here.1).abs(), q, qp))
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0).then((a.1, a.2).cmp(&(b.1, b.2))));
                if let Some((_, q, qp)) = near {
                    out.entry((k, p)).or_insert((q, qp));
                }
            }
        }
        out
    }

    fn rule_cost(&self, members: &[(usize, Orient)], own: &BTreeSet<usize>, ori: &[Orient], joint: bool) -> u32 {
        let (problem, s, tree) = (self.problem, self.s, self.tree);
        let at = |k: usize, p: usize, o: Orient| -> Option<(f64, f64)> {
            let off = s.pin_offset(k, p, o)?;
            Some((to_f64(&(&tree.origins[k][0] + &off[0])), to_f64(&(&tree.origins[k][1] + &off[1]))))
        };
        let turned = |q: usize| if joint && own.contains(&q) { ori[q] } else { tree.geometry.orient[q] };
        let mut cost = 0;
        for &(m, _) in members.iter().filter(|(m, _)| own.contains(m)) {
            for r in &problem.routes {
                let pins: Vec<(usize, usize)> = r.pins.iter().filter_map(ref_pin).collect();
                for &(k, p) in pins.iter().filter(|(k, _)| *k == m) {
                    let (Some(here), Some(a)) = (at(k, p, ori[k]), s.exit(k, p, ori[k])) else { continue };
                    let near = pins
                        .iter()
                        .filter(|(q, _)| *q != k)
                        .filter_map(|&(q, qp)| {
                            let x = at(q, qp, turned(q))?;
                            Some(((x.0 - here.0).abs() + (x.1 - here.1).abs(), q, qp, x))
                        })
                        .min_by(|a, b| a.0.total_cmp(&b.0).then((a.1, a.2).cmp(&(b.1, b.2))));
                    let Some((_, q, qp, x)) = near else { continue };
                    let Some(e) = s.exit(q, qp, turned(q)) else { continue };
                    cost += minbend(a, (-e.0, -e.1), (x.0 - here.0, x.1 - here.1));
                }
            }
        }
        cost
    }

    fn leaf_box(&self, k: usize, ori: &[Orient]) -> ([Q; 4], Vec<(usize, [Q; 2])>) {
        let s = self.s;
        let mut b: Option<[Q; 4]> = None;
        let zero = [Q::zero(), Q::zero()];
        let dress = |b: &mut Option<[Q; 4]>, j: usize, d: &[Q; 2]| {
            if let Some(x) = s.box_offset(j, ori[j]) {
                unite(b, &moved(&x, d));
            }
            if let Some(x) = s.label_offset(j, ori[j]) {
                unite(b, &moved(&x, d));
            }
        };
        dress(&mut b, k, &zero);
        let mut offs: BTreeMap<usize, [Q; 2]> = BTreeMap::new();
        offs.insert(k, zero.clone());
        let hung = self.hung.get(&k).map_or(&[][..], |h| &h[..]);
        for (j, via, rule) in hung {
            let (Some(base), Some(d)) = (offs.get(via).cloned(), hung_offset(s, *j, *via, rule, ori)) else { continue };
            offs.insert(*j, [&base[0] + &d[0], &base[1] + &d[1]]);
        }
        let attached: Vec<(usize, Option<Attach>)> = hung
            .iter()
            .map(|(j, _, rule)| {
                (*j, if let Rule::Attach(a) = rule { Some(super::facing::now(s, *j, a, ori)) } else { None })
            })
            .collect();
        super::facing::clear(s, k, &attached, &mut offs, ori);
        for (j, _, _) in hung {
            if let Some(at) = offs.get(j) {
                dress(&mut b, *j, at);
            }
        }
        offs.remove(&k);
        (b.unwrap_or_else(|| [Q::zero(), Q::zero(), Q::zero(), Q::zero()]), offs.into_iter().collect())
    }

    #[allow(clippy::too_many_arguments)]
    fn evaluate(
        &self,
        _n: usize,
        units: &[Unit],
        ori: &[Orient],
        blocks: &BTreeMap<usize, (Rc<Block>, usize)>,
        banded: &BTreeMap<usize, [bool; 2]>,
        implied: &[Edge],
        currents: &[(usize, usize)],
        levels: &[(PinKey, PinKey)],
        root: bool,
        (list, tables): (Option<&Item>, usize),
        gaps: &[usize],
        out: &[usize],
        seen: &mut BTreeSet<Fingerprint>,
    ) -> Evaluated {
        let (problem, s) = (self.problem, self.s);
        let mut boxes: Vec<[Q; 4]> = Vec::new();
        let mut where_: BTreeMap<usize, (usize, [Q; 2])> = BTreeMap::new();
        let mut hung_objects: BTreeSet<usize> = BTreeSet::new();
        for (u, unit) in units.iter().enumerate() {
            match unit {
                Unit::Leaf(k) => {
                    let (b, offs) = self.leaf_box(*k, ori);
                    boxes.push(b);
                    where_.insert(*k, (u, [Q::zero(), Q::zero()]));
                    for (j, d) in offs {
                        where_.insert(j, (u, d));
                        hung_objects.insert(j);
                    }
                }
                Unit::Block(c) => {
                    let block = &blocks[c].0;
                    boxes.push(block.bbox.clone());
                    for (k, at) in &block.at {
                        where_.insert(*k, (u, at.clone()));
                        if !self.leaves.contains(k) {
                            hung_objects.insert(*k);
                        }
                    }
                }
            }
        }
        let unit_of = |k: usize| where_.get(&k).map(|w| w.0);
        let counts: Vec<[usize; 2]> = units
            .iter()
            .map(|unit| match unit {
                Unit::Block(c) => {
                    let on = banded.get(c).copied().unwrap_or_default();
                    [0, 1].map(|a| if on[a] { blocks[c].0.cuts[a].len() + 1 } else { 1 })
                }
                Unit::Leaf(_) => [1, 1],
            })
            .collect();
        let bands = Bands::new(&counts);
        let band_of = |k: usize, a: usize, u: usize| match units[u] {
            Unit::Block(c) => bands.unit(u, a, blocks[&c].0.band.get(&k).map_or(0, |j| j[a])),
            Unit::Leaf(_) => u,
        };
        let edge = |u: usize, side: usize| At {
            unit: if side >= 2 { bands.last(u, side - 2) } else { u },
            off: boxes[u][side].clone(),
        };
        let placed_box = |u: usize, places: &[Vec<Q>; 2]| {
            [0, 1, 2, 3].map(|side| {
                let e = edge(u, side);
                &places[side % 2][e.unit] + &e.off
            })
        };
        let point = |r: &PointRef, side: i8, axis: usize| -> Option<At> {
            match r {
                PointRef::Origin(k) => {
                    if hung_objects.contains(k) {
                        return None;
                    }
                    let (u, d) = where_.get(k)?;
                    let edge = match side {
                        0 => Q::zero(),
                        _ => s.box_offset(*k, ori[*k])?[axis + if side > 0 { 2 } else { 0 }].clone(),
                    };
                    Some(At { unit: band_of(*k, axis, *u), off: &d[axis] + &edge })
                }
                PointRef::Pin { object, pin, .. }
                | PointRef::SymbolPin { object, pin, .. }
                | PointRef::PinPosition { object, pin, .. } => {
                    if hung_objects.contains(object) {
                        return None;
                    }
                    let (u, d) = where_.get(object)?;
                    let p = s.pin_offset(*object, *pin, ori[*object])?;
                    Some(At { unit: band_of(*object, axis, *u), off: &d[axis] + &p[axis] })
                }
                PointRef::Centre(ms) if ms.len() == 1 => match &ms[0] {
                    PointRef::Centre(_) => None,
                    m => {
                        if hung_objects.contains(&ref_object(m)) {
                            return None;
                        }
                        let (u, d) = where_.get(&ref_object(m))?;
                        let off = match m {
                            PointRef::Pin { object, pin, .. }
                            | PointRef::SymbolPin { object, pin, .. }
                            | PointRef::PinPosition { object, pin, .. } => {
                                &d[axis] + &s.pin_offset(*object, *pin, ori[*object])?[axis]
                            }
                            _ => d[axis].clone(),
                        };
                        Some(At { unit: band_of(ref_object(m), axis, *u), off })
                    }
                },
                PointRef::Centre(_) => None,
            }
        };
        let mut orders = [Order::new(bands.total), Order::new(bands.total)];
        let mut places: [Vec<Q>; 2] = [vec![Q::zero(); bands.total], vec![Q::zero(); bands.total]];
        for (a, lo, hi) in bands.chains() {
            orders[a].at_least(&At { unit: lo, off: Q::zero() }, &At { unit: hi, off: Q::zero() }, Q::zero());
        }
        let ports = units.iter().filter(|u| matches!(u, Unit::Leaf(k) if s.objects[*k].role == Role::Port)).count();
        let way = gaps.get(ports).copied().unwrap_or(0).min(LAYERINGS.len() - 1);
        let mut layering = Layering { count: currents.len(), way, ..Layering::default() };
        let mut level = Levels {
            count: levels.len(),
            tried: way < 2 && gaps.get(ports + 1).is_none_or(|&g| g == 0),
            ..Levels::default()
        };
        let mut left_out = Vec::new();
        let mut table_out = 0usize;
        let mut dissolve: BTreeSet<usize> = BTreeSet::new();
        let mut stretch: BTreeSet<(usize, usize)> = BTreeSet::new();
        type PlacementStep = (Option<usize>, Vec<(usize, At, At, Option<Q>)>);
        let mut steps: Vec<PlacementStep> = Vec::new();
        let bounds_of = |edges: &[Edge]| -> Vec<(usize, At, At, Option<Q>)> {
            edges
                .iter()
                .filter_map(|e| {
                    Some((e.axis, point(&e.from, e.sides.0, e.axis)?, point(&e.to, e.sides.1, e.axis)?, e.gap.clone()))
                })
                .filter(|b| b.1.unit != b.2.unit)
                .collect()
        };
        let mut ordered: Vec<Edge> = Vec::new();
        for &f in &self.s.kept {
            let edges = match self.flips.get(&f) {
                Some((other, down)) if way < 2 && *down == (way == 1) => other,
                _ => &self.edges[f],
            };
            let bounds = bounds_of(edges);
            if self.flips.contains_key(&f) && !bounds.is_empty() {
                layering.facts += 1;
            }
            if matches!(problem.facts[f].kind, FactKind::Order { .. }) {
                ordered.extend(edges.iter().filter_map(|e| self.boxed(e)));
            }
            steps.push((Some(f), bounds));
        }
        for e in implied.iter().chain(&self.implied_alignments) {
            steps.push((None, bounds_of(std::slice::from_ref(e))));
        }
        let boxes_from = steps.len();
        let mut apart = (0usize, 0usize);
        for e in &ordered {
            let bounds = bounds_of(std::slice::from_ref(e));
            apart.0 += usize::from(!bounds.is_empty());
            steps.push((None, bounds));
        }
        let current_from = steps.len();
        if way < 2 {
            for &(a, b) in currents {
                let (a, b) = if way == 0 { (a, b) } else { (b, a) };
                let gap = if matches!(units[a], Unit::Leaf(_)) && matches!(units[b], Unit::Leaf(_)) { 2 } else { 1 };
                let bound = (1, edge(b, 3), edge(a, 1), Some(int(gap)));
                steps.push((None, vec![bound]));
            }
        }
        let level_from = steps.len();
        let pin_y = |(k, pin): PinKey| point(&PointRef::Pin { object: k, pin, terminal: String::new() }, 0, 1);
        let (mut level_steps, mut level_out) = (Vec::new(), BTreeSet::new());
        if level.tried {
            for &(p, q) in levels {
                if let (Some(a), Some(b)) = (pin_y(p), pin_y(q)) {
                    level_steps.push((p, q));
                    steps.push((None, vec![(1, a, b, None)]));
                }
            }
        }
        let table_from = steps.len();
        if let Some(item) = list {
            let unit_index = |it: &Item| -> Option<usize> {
                match it {
                    Item::Leaf(k) => units.iter().position(|u| *u == Unit::Leaf(*k)),
                    Item::Group(c) => units.iter().position(|u| *u == Unit::Block(*c)),
                    Item::List(..) => None,
                }
            };
            let mut table = Vec::new();
            compile(item, &unit_index, units, &edge, &mut table);
            steps.extend(table.into_iter().map(|b| (None, vec![b])));
        }
        let mut by_facts: [BTreeSet<usize>; 2] = [BTreeSet::new(), BTreeSet::new()];
        for (i, (f, bounds)) in steps.into_iter().enumerate() {
            if bounds.is_empty() {
                continue;
            }
            let saved = orders.clone();
            for (a, p, q, gap) in &bounds {
                match gap {
                    None => orders[*a].equal(p, q),
                    Some(g) => orders[*a].at_least(p, q, g.clone()),
                }
            }
            let mut next = places.clone();
            let mut kept = true;
            for a in 0..2 {
                if !bounds.iter().any(|b| b.0 == a) {
                    continue;
                }
                match orders[a].least(places[a].clone()) {
                    Some(v) => next[a] = v,
                    None => kept = false,
                }
            }
            if kept {
                places = next;
                if i < boxes_from {
                    for (a, p, q, _) in &bounds {
                        by_facts[*a].extend([p.unit, q.unit]);
                    }
                }
            } else {
                orders = saved;
                left_out.extend(f);
                table_out += usize::from(i >= table_from);
                layering.refused += usize::from((current_from..level_from).contains(&i));
                apart.1 += usize::from((boxes_from..current_from).contains(&i));
                if (level_from..table_from).contains(&i) {
                    level.refused += 1;
                    level_out.insert(i - level_from);
                }
                if i < boxes_from {
                    for (a, p, q, _) in &bounds {
                        let (x, y) = (bands.item(p.unit), bands.item(q.unit));
                        if x == y {
                            continue;
                        }
                        for u in [x, y] {
                            if let Unit::Block(c) = units[u] {
                                if self.bands && bands.count(u, *a) == 1 && !blocks[&c].0.cuts[*a].is_empty() {
                                    stretch.insert((c, *a));
                                } else {
                                    dissolve.insert(c);
                                }
                            }
                        }
                    }
                }
            }
        }

        let value = |p: &At, a: usize, places: &[Vec<Q>; 2]| &places[a][p.unit] + &p.off;
        let mut trees: Vec<Vec<Lead>> = Vec::new();
        for r in &problem.routes {
            let mut pins: Vec<Lead> = Vec::new();
            for p in &r.pins {
                let Some((k, pin)) = ref_pin(p) else { continue };
                if s.objects[k].role == Role::Port || pins.iter().any(|q| q.0 == (k, pin)) {
                    continue;
                }
                let (Some(e), Some(x), Some(y)) = (s.exit(k, pin, ori[k]), point(p, 0, 0), point(p, 0, 1)) else {
                    continue;
                };
                pins.push(((k, pin), e, [x, y]));
            }
            trees.push(pins);
        }
        for pins in &trees {
            for i in 0..pins.len() {
                for j in i + 1..pins.len() {
                    let ((pa, ea, ta), (pb, eb, tb)) = (&pins[i], &pins[j]);
                    if pa.0 == pb.0
                        || bands.item(ta[0].unit) == bands.item(tb[0].unit)
                        || (ea.0, ea.1) != (-eb.0, -eb.1)
                    {
                        continue;
                    }
                    let (along, way) = if ea.1 == 0 { (0, ea.0) } else { (1, ea.1) };
                    let nearest = self.nearest.get(pa) == Some(pb) || self.nearest.get(pb) == Some(pa);
                    if !nearest || value(&tb[along], along, &places) != value(&ta[along], along, &places) {
                        continue;
                    }
                    let mut trial = orders[along].clone();
                    if way > 0 {
                        trial.at_least(&ta[along], &tb[along], int(1));
                    } else {
                        trial.at_least(&tb[along], &ta[along], int(1));
                    }
                    if let Some(v) = trial.least(places[along].clone()) {
                        orders[along] = trial;
                        places[along] = v;
                    }
                }
            }
        }
        let mut facing: Vec<Facing> = Vec::new();
        for pins in &trees {
            for i in 0..pins.len() {
                for j in i + 1..pins.len() {
                    let ((pa, ea, ta), (pb, eb, tb)) = (&pins[i], &pins[j]);
                    if pa.0 == pb.0
                        || bands.item(ta[0].unit) == bands.item(tb[0].unit)
                        || (ea.0, ea.1) != (-eb.0, -eb.1)
                    {
                        continue;
                    }
                    let (along, way) = if ea.1 == 0 { (0, ea.0) } else { (1, ea.1) };
                    let ahead = value(&tb[along], along, &places) - value(&ta[along], along, &places);
                    if (way > 0 && ahead <= Q::zero()) || (way < 0 && ahead >= Q::zero()) {
                        continue;
                    }
                    let across = 1 - along;
                    let off = value(&tb[across], across, &places) - value(&ta[across], across, &places);
                    let straight = self.nearest.get(pa) == Some(pb) || self.nearest.get(pb) == Some(pa);
                    let (ta, tb) = (ta[across].clone(), tb[across].clone());
                    facing.push((!straight, ahead.abs() + off.abs(), *pa, *pb, across, ta, tb));
                }
            }
        }
        facing.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.cmp(&y.1)).then((x.2, x.3).cmp(&(y.2, y.3))));
        let mut pairs = Pairs { count: facing.len(), ..Pairs::default() };
        let mut paired: BTreeSet<(usize, usize)> = BTreeSet::new();
        let mut kept: Vec<usize> = Vec::new();
        for (i, (_, _, pa, pb, a, ta, tb)) in facing.iter().enumerate() {
            if out.contains(&i) {
                pairs.left += 1;
                continue;
            }
            if paired.contains(pa) || paired.contains(pb) {
                pairs.passed += 1;
                continue;
            }
            let mut trial = orders[*a].clone();
            trial.equal(ta, tb);
            match trial.least(places[*a].clone()) {
                Some(v) => {
                    orders[*a] = trial;
                    places[*a] = v;
                    paired.insert(*pa);
                    paired.insert(*pb);
                    kept.push(i);
                }
                None => pairs.refused += 1,
            }
        }
        pairs.kept = kept.len();
        pairs.order = kept.iter().rev().copied().chain((0..facing.len()).rev().filter(|i| !kept.contains(i))).collect();
        let hang = gaps.get(ports + 2).is_none_or(|&g| g == 0);
        let mut hanging = Hanging::default();
        for (u, unit) in units.iter().enumerate() {
            let Unit::Leaf(k) = *unit else { continue };
            if s.objects[k].role != Role::Element || s.objects[k].symbol.is_none() {
                continue;
            }
            let marked = (0..s.pins(k).len()).any(|p| s.currents.contains_key(&(k, p)));
            for a in 0..2 {
                if by_facts[a].contains(&u) || (a == 1 && marked) {
                    continue;
                }
                let mut best: Option<(Q, At, At)> = None;
                for pin in 0..s.pins(k).len() {
                    let Some(e) = s.exit(k, pin, ori[k]) else { continue };
                    let way = if a == 0 { e.0 } else { e.1 };
                    if way == 0 {
                        continue;
                    }
                    let Some(me) = point(&PointRef::Pin { object: k, pin, terminal: String::new() }, 0, a) else {
                        continue;
                    };
                    let me = At { unit: me.unit, off: &me.off + int(way) };
                    for r in problem.routes.iter().filter(|r| r.pins.iter().any(|p| ref_pin(p) == Some((k, pin)))) {
                        for p in &r.pins {
                            let Some((q, qp)) = ref_pin(p) else { continue };
                            if q == k || unit_of(q) == Some(u) || s.objects[q].role == Role::Port {
                                continue;
                            }
                            let Some(eq) = s.exit(q, qp, ori[q]) else { continue };
                            if (if a == 0 { eq.0 } else { eq.1 }) != 0 {
                                continue;
                            }
                            let Some(t) = point(p, 0, a) else { continue };
                            let d = (value(&t, a, &places) - value(&me, a, &places)).abs();
                            if best.as_ref().is_none_or(|b| d < b.0) {
                                best = Some((d, me.clone(), t));
                            }
                        }
                    }
                }
                let Some((_, me, t)) = best else { continue };
                hanging.count += 1;
                if !hang {
                    continue;
                }
                let mut trial = orders[a].clone();
                trial.equal(&t, &me);
                match trial.least(places[a].clone()) {
                    Some(v) => {
                        orders[a] = trial;
                        places[a] = v;
                        hanging.hung += 1;
                    }
                    None => hanging.refused += 1,
                }
            }
        }
        let mut aligned = Vec::new();
        let mut counts = Vec::new();
        for (u, unit) in units.iter().enumerate() {
            let Unit::Leaf(k) = *unit else { continue };
            if s.objects[k].role != Role::Port {
                continue;
            }
            let want = gaps.get(counts.len()).copied().unwrap_or(0);
            let free = s.exit(k, 0, ori[k]).map(|e| usize::from(e.1 == 0)).filter(|&a| !s.objects[k].fact_axes[a]);
            let me = free.and_then(|a| point(&PointRef::Pin { object: k, pin: 0, terminal: String::new() }, 0, a));
            let (Some(a), Some(me)) = (free, me) else {
                counts.push(1);
                continue;
            };
            let mine = &places[a][u] + &me.off;
            let mut pins: Vec<(Q, usize, usize, At)> = Vec::new();
            for r in problem.routes.iter().filter(|r| r.pins.iter().any(|p| ref_pin(p) == Some((k, 0)))) {
                for p in &r.pins {
                    let Some((q, qp)) = ref_pin(p) else { continue };
                    if q == k || unit_of(q) == Some(u) {
                        continue;
                    }
                    let Some(t) = point(p, 0, a) else { continue };
                    let d = (&places[a][t.unit] + &t.off - &mine).abs();
                    pins.push((d, q, qp, t));
                }
            }
            pins.sort_by(|x, y| x.0.cmp(&y.0).then((x.1, x.2).cmp(&(y.1, y.2))));
            pins.dedup_by(|x, y| (x.1, x.2) == (y.1, y.2));
            let mut lines: Vec<(Q, At)> = Vec::new();
            for w in (0..units.len()).filter(|&w| w != u) {
                for side in [a, a + 2] {
                    let e = edge(w, side);
                    lines.push((&places[a][e.unit] + &e.off, e));
                }
            }
            for (obj, (w, d)) in where_.iter().filter(|(_, (w, _))| *w != u) {
                let w = band_of(*obj, a, *w);
                for pin in 0..s.pins(*obj).len() {
                    if let Some(p) = s.pin_offset(*obj, pin, ori[*obj]) {
                        let off = &d[a] + &p[a];
                        lines.push((&places[a][w] + &off, At { unit: w, off }));
                    }
                }
            }
            lines.sort_by(|x, y| x.0.cmp(&y.0));
            lines.dedup_by(|x, y| x.0 == y.0);
            enum Gap {
                On(usize, usize, At),
                Between(Option<At>, Option<At>),
            }
            let target = pins.first().map_or_else(|| mine.clone(), |p| &places[a][p.3.unit] + &p.3.off);
            let mut rest: Vec<(Q, Q, Gap)> = Vec::new();
            for (_, q, qp, t) in pins.iter().skip(1) {
                let at = &places[a][t.unit] + &t.off;
                rest.push(((&at - &target).abs(), at, Gap::On(*q, *qp, t.clone())));
            }
            for i in 0..=lines.len() {
                let (lo, hi) = (i.checked_sub(1).map(|j| &lines[j]), lines.get(i));
                let at = match (lo, hi) {
                    (Some(l), Some(h)) => (&l.0 + &h.0) / int(2),
                    (Some(l), None) => &l.0 + int(1),
                    (None, Some(h)) => &h.0 - int(1),
                    (None, None) => continue,
                };
                rest.push(((&at - &target).abs(), at, Gap::Between(lo.map(|l| l.1.clone()), hi.map(|h| h.1.clone()))));
            }
            rest.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.cmp(&y.1)));
            let net: Vec<Gap> = pins.into_iter().map(|(_, q, qp, t)| Gap::On(q, qp, t)).collect();
            let list: Vec<&Gap> = net.iter().take(1).chain(rest.iter().map(|r| &r.2)).collect();
            counts.push(list.len().max(1));
            let tries: Vec<&Gap> =
                if want == 0 { net.iter().collect() } else { list.get(want).copied().into_iter().collect() };
            let mut took = want == 0;
            for gap in tries {
                let mut trial = orders[a].clone();
                match gap {
                    Gap::On(_, _, t) => trial.equal(t, &me),
                    Gap::Between(lo, hi) => {
                        if let Some(l) = lo {
                            trial.at_least(l, &me, int(1));
                        }
                        if let Some(h) = hi {
                            trial.at_least(&me, h, int(1));
                        }
                    }
                }
                if let Some(v) = trial.least(places[a].clone()) {
                    orders[a] = trial;
                    places[a] = v;
                    if let Gap::On(q, qp, _) = gap {
                        aligned.push((k, (*q, *qp)));
                    }
                    took = true;
                    break;
                }
            }
            if !took {
                return Evaluated::Same;
            }
        }
        for (u, unit) in units.iter().enumerate() {
            let Unit::Leaf(k) = *unit else { continue };
            if s.objects[k].symbol.is_some() {
                continue;
            }
            let next: Vec<&PointRef> = problem
                .wires
                .iter()
                .flat_map(|w| {
                    let at: Vec<usize> = (0..w.anchors.len()).filter(|&i| refers_to(&w.anchors[i], k)).collect();
                    at.into_iter()
                        .flat_map(move |i| [i.checked_sub(1), Some(i + 1)])
                        .flatten()
                        .filter_map(|j| w.anchors.get(j))
                })
                .filter(|r| ref_pin(r).is_some())
                .collect();
            let across = |r: &PointRef, a: usize| {
                ref_pin(r).and_then(|(q, qp)| s.exit(q, qp, ori[q])).is_some_and(|e| {
                    if a == 0 {
                        e.1 != 0
                    } else {
                        e.0 != 0
                    }
                })
            };
            for a in (0..2).filter(|&a| !s.objects[k].fact_axes[a]) {
                let Some(me) = point(&PointRef::Origin(k), 0, a) else { continue };
                let mine = value(&me, a, &places);
                let mut pins: Vec<(Q, At)> = next
                    .iter()
                    .filter(|r| across(r, a))
                    .filter_map(|r| point(r, 0, a))
                    .filter(|t| t.unit != u)
                    .map(|t| ((value(&t, a, &places) - &mine).abs(), t))
                    .collect();
                pins.sort_by(|x, y| x.0.cmp(&y.0));
                for (_, t) in pins {
                    let mut trial = orders[a].clone();
                    trial.equal(&t, &me);
                    if let Some(v) = trial.least(places[a].clone()) {
                        orders[a] = trial;
                        places[a] = v;
                        break;
                    }
                }
            }
        }
        counts.push(if currents.is_empty() && layering.facts == 0 { 1 } else { LAYERINGS.len() });
        counts.push(if levels.is_empty() || false { 1 } else { 2 });
        counts.push(if hanging.count == 0 { 1 } else { 2 });
        counts.push(tables);
        for &f in &self.s.kept {
            let at = |p: &At, a: usize, places: &[Vec<Q>; 2]| &places[a][p.unit] + &p.off;
            let mut tries: Vec<Vec<(usize, At, At, Q)>> = Vec::new();
            match &problem.facts[f].kind {
                FactKind::Distribute { axis, sense, members } if members.len() >= 3 => {
                    let a = usize::from(*axis == Axis::Y);
                    let Some(ps) = members.iter().map(|m| point(m, 0, a)).collect::<Option<Vec<At>>>() else {
                        continue;
                    };
                    if ps.iter().all(|p| bands.item(p.unit) == bands.item(ps[0].unit)) {
                        continue;
                    }
                    let first = at(&ps[0], a, &places);
                    let up = match sense {
                        Sense::Increasing => true,
                        Sense::Decreasing => false,
                        Sense::Either => at(&ps[ps.len() - 1], a, &places) >= first,
                    };
                    let sign = if up { int(1) } else { int(-1) };
                    let step = ps
                        .windows(2)
                        .map(|w| (at(&w[1], a, &places) - at(&w[0], a, &places)).abs())
                        .max()
                        .unwrap_or_default();
                    for more in 0..8 {
                        let d = (&step + int(more)) * &sign;
                        tries.push(
                            ps.iter()
                                .enumerate()
                                .skip(1)
                                .map(|(i, p)| (a, ps[0].clone(), p.clone(), &d * int(i as i64)))
                                .collect(),
                        );
                    }
                }
                FactKind::Align { axis, members } => {
                    let a = usize::from(*axis == Axis::Y);
                    let Some((c, inner)) = members.iter().enumerate().find_map(|(i, m)| match m {
                        PointRef::Centre(ms) if ms.len() >= 2 => Some((i, ms)),
                        _ => None,
                    }) else {
                        continue;
                    };
                    let Some(ps) = inner.iter().map(|m| point(m, 0, a)).collect::<Option<Vec<At>>>() else { continue };
                    let others: Option<Vec<At>> =
                        members.iter().enumerate().filter(|(i, _)| *i != c).map(|(_, m)| point(m, 0, a)).collect();
                    let Some(others) = others else { continue };
                    let lo = (0..ps.len())
                        .min_by(|&i, &j| at(&ps[i], a, &places).cmp(&at(&ps[j], a, &places)))
                        .expect("points");
                    let hi = (0..ps.len())
                        .max_by(|&i, &j| at(&ps[i], a, &places).cmp(&at(&ps[j], a, &places)))
                        .expect("points");
                    let width = at(&ps[hi], a, &places) - at(&ps[lo], a, &places);
                    for more in 0..8 {
                        let w = &width + int(more);
                        let half = &w / int(2);
                        let mut bounds = vec![(a, ps[lo].clone(), ps[hi].clone(), w.clone())];
                        bounds.extend(others.iter().map(|o| (a, ps[lo].clone(), o.clone(), half.clone())));
                        tries.push(bounds);
                    }
                }
                _ => {}
            }
            for bounds in tries {
                if bounds.iter().all(|b| bands.item(b.1.unit) == bands.item(b.2.unit)) {
                    break;
                }
                let saved = orders.clone();
                for (a, p, q, d) in &bounds {
                    let shifted = At { unit: p.unit, off: &p.off + d };
                    orders[*a].equal(&shifted, q);
                }
                let mut next = places.clone();
                let mut kept = true;
                for a in 0..2 {
                    if !bounds.iter().any(|b| b.0 == a) {
                        continue;
                    }
                    match orders[a].least(places[a].clone()) {
                        Some(v) => next[a] = v,
                        None => kept = false,
                    }
                }
                if kept {
                    places = next;
                    break;
                }
                orders = saved;
            }
        }

        let mut apart_bands: Vec<(usize, usize, usize, usize)> = Vec::new();
        for (u, unit) in units.iter().enumerate() {
            let Unit::Block(c) = unit else { continue };
            for (a, axis_crossings) in blocks[c].0.crossed.iter().enumerate() {
                for j in 1..bands.count(u, a) {
                    let crossed = axis_crossings.get(j - 1).copied().unwrap_or(0);
                    apart_bands.push((crossed, a, u, j));
                }
            }
        }
        apart_bands.sort_by(|x, y| y.0.cmp(&x.0).then((x.1, x.2, x.3).cmp(&(y.1, y.2, y.3))));
        for (_, a, u, j) in apart_bands {
            let (lo, hi) = (bands.unit(u, a, j - 1), bands.unit(u, a, j));
            if places[a][lo] == places[a][hi] {
                continue;
            }
            let mut trial = orders[a].clone();
            trial.at_least(&At { unit: hi, off: Q::zero() }, &At { unit: lo, off: Q::zero() }, Q::zero());
            if let Some(v) = trial.least(places[a].clone()) {
                orders[a] = trial;
                places[a] = v;
            }
        }
        let mut stuck: BTreeSet<(usize, usize)> = BTreeSet::new();
        for _ in 0..units.len() * units.len() + 4 {
            let placed: Vec<[Q; 4]> = (0..units.len()).map(|u| placed_box(u, &places)).collect();
            let mut pair = None;
            'find: for u in 0..units.len() {
                for v in u + 1..units.len() {
                    let (bu, bv) = (&placed[u], &placed[v]);
                    if bu[0] < bv[2] && bv[0] < bu[2] && bu[1] < bv[3] && bv[1] < bu[3] && !stuck.contains(&(u, v)) {
                        pair = Some((u, v, bu.clone(), bv.clone()));
                        break 'find;
                    }
                }
            }
            let Some((u, v, bu, bv)) = pair else { break };
            let gap = if matches!(units[u], Unit::Leaf(_)) && matches!(units[v], Unit::Leaf(_)) { 2 } else { 1 };
            let mut ways: Vec<(Q, usize, usize, usize)> = Vec::new();
            for a in 0..2 {
                for (lo, hi, blo, bhi) in [(u, v, &bu, &bv), (v, u, &bv, &bu)] {
                    ways.push((&blo[a + 2] + int(gap) - &bhi[a], a, lo, hi));
                }
            }
            ways.sort();
            let mut done = false;
            for (_, a, lo, hi) in ways {
                let saved = orders[a].clone();
                orders[a].at_least(&edge(lo, a + 2), &edge(hi, a), int(gap));
                if let Some(v) = orders[a].least(places[a].clone()) {
                    places[a] = v;
                    done = true;
                    break;
                }
                orders[a] = saved;
            }
            if !done {
                stuck.insert((u, v));
                for w in [u, v] {
                    if let Unit::Block(c) = units[w] {
                        dissolve.insert(c);
                    }
                }
            }
        }

        layering.against = currents
            .iter()
            .filter(|&&(a, b)| {
                let (x, y) = (edge(a, 1), edge(b, 3));
                &places[1][x.unit] + &x.off < &places[1][y.unit] + &y.off
            })
            .count();
        level.kept =
            level_steps.into_iter().enumerate().filter(|(i, _)| !level_out.contains(i)).map(|(_, p)| p).collect();
        level.apart = levels
            .iter()
            .filter(|&&(p, q)| match (pin_y(p), pin_y(q)) {
                (Some(a), Some(b)) => value(&a, 1, &places) != value(&b, 1, &places),
                _ => true,
            })
            .count();
        let mut look: Vec<Q> = places[0].iter().chain(&places[1]).cloned().collect();
        for unit in units {
            if let Unit::Leaf(k) = unit {
                look.push(int(ORDER.iter().position(|o| *o == ori[*k]).unwrap_or(8) as i64));
            }
        }
        look.push(int(level.kept.len() as i64));
        let mut key = Look::new();
        key.row(&look);
        if !seen.insert(key.finish()) {
            return Evaluated::Same;
        }
        let mut g = self.tree.geometry.clone();
        g.orient = ori.to_vec();
        let mut objects: BTreeSet<usize> = BTreeSet::new();
        for (k, (u, d)) in &where_ {
            let at = [&places[0][band_of(*k, 0, *u)] + &d[0], &places[1][band_of(*k, 1, *u)] + &d[1]];
            if matches!(problem.objects[*k].position, Position::Own { .. }) {
                g.origin[*k] = at;
            }
            objects.insert(*k);
        }
        objects = geometry_members(problem, &objects);
        let mut fixed: Vec<(usize, Vec<[Q; 2]>)> = Vec::new();
        let mut inner: Vec<track::GroupIn> = Vec::new();
        let mut bbox: Option<[Q; 4]> = None;
        for (u, unit) in units.iter().enumerate() {
            let placed = placed_box(u, &places);
            unite(&mut bbox, &placed);
            let Unit::Block(c) = unit else { continue };
            let block = &blocks[c].0;
            let moved =
                |p: &[Q; 2]| [0, 1].map(|a| &p[a] + &places[a][bands.unit(u, a, band::band(&block.cuts[a], &p[a]))]);
            for (net, path) in &block.wires {
                fixed.push((*net, path.iter().map(moved).collect()));
            }
            inner.push(track::GroupIn {
                members: block.at.keys().copied().collect(),
                depth: 0,
                inside: Vec::new(),
                boxed: Some(placed),
            });
        }
        let Some(bbox) = bbox else { return Evaluated::Unplaced(0, Some((counts, pairs.order))) };
        let clip = (!root).then(|| [&bbox[0] - int(1), &bbox[1] - int(1), &bbox[2] + int(1), &bbox[3] + int(1)]);
        let origins = s.origins(problem, &g);
        let Ok((mut layout, pins, written_net, refs)) = layout_of(problem, s, &g, &origins, &objects, root) else {
            return Evaluated::Unplaced(0, None);
        };
        for (i, route) in problem.routes.iter().enumerate() {
            if complete_auto_route(route, &refs[i]) {
                layout.nets[i].preferred = crate::route::default_candidate(&layout, i);
            }
        }
        let input = track::Input {
            layout: &layout,
            pins,
            written_net,
            groups: inner,
            weights: self.weights,
            budget: STEPS,
            full: [BTreeSet::new(), BTreeSet::new()],
            kept_clear: self.clear.clone(),
            fixed,
            clip,
            lenient: !root,
        };
        let span = crate::profile::span("lines", "route");
        ROUTED.with(|r| r.set(r.get() + 1));
        let routed = track::route(&input);
        crate::profile::add("expansions", routed.expansions);
        drop(span);

        if routed.failed.is_some() {
            return Evaluated::Unplaced(routed.expansions, Some((counts, pairs.order)));
        }
        let bends: u32 = routed.costs.iter().map(|c| c.1).sum();
        let (wc, wb) = self.weights;
        let score = (
            dissolve.len() + stretch.len(),
            routed.parts.failed,
            wc * u64::from(routed.crossings) + wb * u64::from(bends),
            routed.made,
            routed.steps,
        );
        Evaluated::Done(Box::new(Tried {
            score,
            crossings: routed.crossings,
            bends,
            ori: ori.to_vec(),
            places: (0..bands.total).map(|u| [places[0][u].clone(), places[1][u].clone()]).collect(),
            bands,
            geometry: g,
            routed,
            aligned,
            refs,
            left_out,
            table_out,
            dissolve: dissolve.into_iter().collect(),
            stretch: stretch.into_iter().collect(),
            objects,
            gaps: counts,
            pairs,
            hanging,
            layering,
            levels: level,
            boxes: apart,
        }))
    }

    fn block(&self, units: &[Unit], best: &Tried, blocks: &BTreeMap<usize, (Rc<Block>, usize)>) -> Block {
        let s = self.s;
        let bands = &best.bands;
        let mut at: BTreeMap<usize, [Q; 2]> = BTreeMap::new();
        let mut parts: Vec<([Q; 4], Vec<usize>)> = Vec::new();
        for (u, unit) in units.iter().enumerate() {
            let d = &best.places[u];
            match unit {
                Unit::Leaf(k) => {
                    at.insert(*k, d.clone());
                    let (b, offs) = self.leaf_box(*k, &best.ori);
                    let mut extent = Some(moved(&b, d));
                    let dot = |x: &[Q; 2]| [x[0].clone(), x[1].clone(), x[0].clone(), x[1].clone()];
                    let mut objects = vec![*k];
                    for (j, o) in std::iter::once((*k, [Q::zero(), Q::zero()])).chain(offs) {
                        let origin = [&d[0] + &o[0], &d[1] + &o[1]];
                        unite(&mut extent, &dot(&origin));
                        for pin in 0..s.pins(j).len() {
                            if let Some(p) = s.pin_offset(j, pin, best.ori[j]) {
                                unite(&mut extent, &dot(&[&origin[0] + &p[0], &origin[1] + &p[1]]));
                            }
                        }
                        if j == *k {
                            continue;
                        }
                        if matches!(self.problem.objects[j].position, Position::Own { .. }) {
                            at.insert(j, origin);
                        }
                        objects.push(j);
                    }
                    parts.push((extent.unwrap_or_else(|| moved(&b, d)), objects));
                }
                Unit::Block(c) => {
                    let block = &blocks[c].0;
                    let shift = |k: &usize| {
                        let j = block.band.get(k).copied().unwrap_or_default();
                        [0, 1].map(|a| best.places[bands.unit(u, a, j[a])][a].clone())
                    };
                    for (k, p) in &block.at {
                        let d = shift(k);
                        at.insert(*k, [&p[0] + &d[0], &p[1] + &d[1]]);
                    }
                    for (b, objects) in &block.parts {
                        let d = objects.first().map_or_else(|| d.clone(), shift);
                        parts.push((moved(b, &d), objects.clone()));
                    }
                }
            }
        }
        let wires = track::polylines(&best.routed.lines);
        let mut bbox: Option<[Q; 4]> = None;
        let origins = s.origins(self.problem, &best.geometry);
        for &k in at.keys() {
            if let Some(b) = s.box_at(&origins, &best.ori, k) {
                unite(&mut bbox, &b);
            }
            if let Some(l) = s.label_offset(k, best.ori[k]) {
                unite(&mut bbox, &moved(&l, &origins[k]));
            }
        }
        for (_, path) in &wires {
            for p in path {
                unite(&mut bbox, &[p[0].clone(), p[1].clone(), p[0].clone(), p[1].clone()]);
            }
        }
        for (u, unit) in units.iter().enumerate() {
            if let Unit::Block(c) = unit {
                let b = &blocks[c].0.bbox;
                let far = [&best.places[bands.last(u, 0)][0], &best.places[bands.last(u, 1)][1]];
                let d = &best.places[u];
                unite(&mut bbox, &[&b[0] + &d[0], &b[1] + &d[1], &b[2] + far[0], &b[3] + far[1]]);
            }
        }
        let b = bbox.unwrap_or_else(|| [Q::zero(), Q::zero(), Q::zero(), Q::zero()]);
        let mut levels = best.levels.kept.clone();
        for unit in units {
            if let Unit::Block(c) = unit {
                levels.extend(blocks[c].0.levels.iter().copied());
            }
        }
        let boxes: Vec<[Q; 4]> = parts.iter().map(|p| p.0.clone()).collect();
        let cuts = [0, 1].map(|a| {
            let corners: Vec<Q> = wires.iter().flat_map(|(_, path)| path.iter().map(move |p| p[a].clone())).collect();
            band::cuts(&boxes, &corners, a)
        });
        let mut band = BTreeMap::new();
        for (b, objects) in &parts {
            let j = [band::band(&cuts[0], &b[0]), band::band(&cuts[1], &b[1])];
            band.extend(objects.iter().map(|&k| (k, j)));
        }
        let crossed = [0, 1].map(|a| band::crossed(&cuts[a], wires.iter().map(|w| w.1.as_slice()), a));
        Block {
            at,
            wires,
            bbox: [&b[0] - int(1), &b[1] - int(1), &b[2] + int(1), &b[3] + int(1)],
            levels,
            parts,
            cuts,
            band,
            crossed,
        }
    }
}

fn compile(
    item: &Item,
    unit_index: &dyn Fn(&Item) -> Option<usize>,
    units: &[Unit],
    edge: &dyn Fn(usize, usize) -> At,
    out: &mut Vec<(usize, At, At, Option<Q>)>,
) {
    let Item::List(dir, items) = item else { return };
    fn units_of(item: &Item, unit_index: &dyn Fn(&Item) -> Option<usize>, out: &mut Vec<usize>) {
        match item {
            Item::List(_, items) => items.iter().for_each(|i| units_of(i, unit_index, out)),
            other => out.extend(unit_index(other)),
        }
    }
    let parts: Vec<Vec<usize>> = items
        .iter()
        .map(|i| {
            let mut out = Vec::new();
            units_of(i, unit_index, &mut out);
            out
        })
        .collect();
    for w in parts.windows(2) {
        for &a in &w[0] {
            for &b in &w[1] {
                let gap = if matches!(units[a], Unit::Leaf(_)) && matches!(units[b], Unit::Leaf(_)) { 2 } else { 1 };
                out.push(match dir {
                    Dir::Row => (0, edge(a, 2), edge(b, 0), Some(int(gap))),
                    Dir::Column => (1, edge(b, 3), edge(a, 1), Some(int(gap))),
                });
            }
        }
    }
    for i in items {
        compile(i, unit_index, units, edge, out);
    }
}

fn fact_points(kind: &FactKind) -> Vec<PointRef> {
    match kind {
        FactKind::Align { members, .. } | FactKind::Order { members, .. } | FactKind::Distribute { members, .. } => {
            members.clone()
        }
        FactKind::Table { rows, .. } => rows.points().into_iter().cloned().collect(),
        FactKind::Distance { from, to, .. } => from.iter().chain(to).cloned().collect(),
        _ => Vec::new(),
    }
}

fn refers_to(r: &PointRef, k: usize) -> bool {
    match r {
        PointRef::Origin(j)
        | PointRef::Pin { object: j, .. }
        | PointRef::SymbolPin { object: j, .. }
        | PointRef::PinPosition { object: j, .. } => *j == k,
        PointRef::Centre(ms) => ms.iter().any(|m| refers_to(m, k)),
    }
}

fn objects_of(r: &PointRef, out: &mut BTreeSet<usize>) {
    match r {
        PointRef::Origin(j)
        | PointRef::Pin { object: j, .. }
        | PointRef::SymbolPin { object: j, .. }
        | PointRef::PinPosition { object: j, .. } => {
            out.insert(*j);
        }
        PointRef::Centre(ms) => ms.iter().for_each(|m| objects_of(m, out)),
    }
}

fn ref_object(r: &PointRef) -> usize {
    match r {
        PointRef::Origin(k)
        | PointRef::Pin { object: k, .. }
        | PointRef::SymbolPin { object: k, .. }
        | PointRef::PinPosition { object: k, .. } => *k,
        PointRef::Centre(ms) => ms.first().map_or(usize::MAX, ref_object),
    }
}

/// A group's temporary two-pin slice of a larger tree is not a complete auto wire.
/// Leave that topology available for the parent to join; only complete trees use
/// the core's unmerged default. Compare physical pins across reference spellings.
pub(super) fn complete_auto_route(route: &rnl_runtime::problem::RouteIntent, refs: &[PointRef]) -> bool {
    route.mode == "auto"
        && route.wires.is_empty()
        && refs.len() == 2
        && route.pins.iter().all(|p| refs.iter().any(|q| same_point(p, q)))
}

fn same_point(a: &PointRef, b: &PointRef) -> bool {
    match (a, b) {
        (PointRef::Origin(x), PointRef::Origin(y)) => x == y,
        (
            PointRef::Pin { object: x, pin: p, .. } | PointRef::SymbolPin { object: x, pin: p, .. },
            PointRef::Pin { object: y, pin: q, .. } | PointRef::SymbolPin { object: y, pin: q, .. },
        ) => x == y && p == q,
        (PointRef::PinPosition { object: x, pin: p, .. }, PointRef::PinPosition { object: y, pin: q, .. }) => {
            x == y && p == q
        }
        (PointRef::Centre(xs), PointRef::Centre(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| same_point(x, y))
        }
        _ => false,
    }
}

fn ref_pin(r: &PointRef) -> Option<(usize, usize)> {
    if let PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. } = r {
        Some((*object, *pin))
    } else {
        None
    }
}

/// Drawing scope follows geometric dependencies, without introducing connections.
fn geometry_members(problem: &LevelProblem, members: &BTreeSet<usize>) -> BTreeSet<usize> {
    let mut out = members.clone();
    loop {
        if !permit(BudgetStep::Structure, problem.objects.len()) {
            return out;
        }
        let before = out.len();
        for (k, object) in problem.objects.iter().enumerate() {
            let follows = match &object.position {
                Position::Own { .. } => false,
                Position::At(r) => super::port::reads_reference(r, &|j| out.contains(&j)),
                Position::InSymbol { owner, .. } => out.contains(owner),
            };
            if follows {
                out.insert(k);
            }
        }
        if out.len() == before {
            return out;
        }
    }
}

#[derive(Debug)]
pub(super) enum LayoutError {
    Written(crate::route::WrittenPathError),
    Annotation(crate::annotation::AnnotationError),
}

#[allow(clippy::type_complexity)]
pub(super) fn layout_of(
    problem: &LevelProblem,
    s: &Structure,
    g: &Geometry,
    origins: &[[Q; 2]],
    objects: &BTreeSet<usize>,
    all_annotations: bool,
) -> Result<
    (Layout, Vec<(usize, usize, [Q; 2], Option<(i64, i64)>)>, Vec<Option<usize>>, Vec<Vec<PointRef>>),
    LayoutError,
> {
    let mut layout = Layout { modern: true, ..Layout::default() };
    for (object, bounds) in crate::annotation::bounds(problem, g).map_err(LayoutError::Annotation)? {
        let owner = crate::annotation::attachment_owner(problem, object);
        let attached = match &problem.objects[object].position {
            Position::At(point) => super::port::reads_reference(point, &|k| objects.contains(&k)),
            _ => false,
        };
        if all_annotations || objects.contains(&object) || owner.is_some_and(|k| objects.contains(&k)) || attached {
            layout.box_sources.insert(layout.boxes.len(), object);
            layout.boxes.push((owner.unwrap_or(object), bounds));
        }
    }
    let drawn: Vec<usize> = objects.iter().copied().filter(|&k| !s.objects[k].left_free).collect();
    let components: BTreeSet<usize> = s.components().collect();
    for &k in &drawn {
        if components.contains(&k) {
            if let Some(b) = s.box_at(origins, &g.orient, k) {
                layout.boxes.push((k, b));
            }
        }
    }
    let mut pins = Vec::new();
    for &k in &drawn {
        for pin in 0..s.pins(k).len() {
            if let Some(at) = s.pin_at(origins, &g.orient, k, pin) {
                layout.pins.push(at.clone());
                pins.push((k, pin, at, s.exit(k, pin, g.orient[k])));
            }
        }
    }
    for &k in &drawn {
        let at = &origins[k];
        layout.texts.extend(s.text_offsets(problem, g, k).iter().map(|b| moved(b, at)));
    }
    let inside = |r: &PointRef| super::port::reads_reference(r, &|k| objects.contains(&k));
    let mut written_net = Vec::new();
    let mut wires_in: BTreeSet<usize> = BTreeSet::new();
    for (w, wire) in problem.wires.iter().enumerate() {
        if !wire.anchors.iter().all(inside) {
            continue;
        }
        if let Some(path) = written_path(problem, g, w).map_err(LayoutError::Written)? {
            layout.written.push(path);
            written_net.push(problem.routes.iter().position(|r| r.wires.contains(&w)));
            wires_in.insert(w);
        }
    }
    let mut refs = Vec::new();
    for r in &problem.routes {
        let mut net = NetIn::default();
        let mut points: Vec<PointRef> = Vec::new();
        let add = |p: &PointRef, net: &mut NetIn, points: &mut Vec<PointRef>| -> Option<usize> {
            if let Some(i) = points.iter().position(|q| same_point(q, p)) {
                return Some(i);
            }
            if !inside(p) {
                return None;
            }
            let at = [point_value(problem, g, p, 0, 0)?, point_value(problem, g, p, 1, 0)?];
            let (exit, owner) = match ref_pin(p) {
                Some((k, pin)) => (s.exit(k, pin, g.orient[k]), Some(k)),
                None => (
                    None,
                    match p {
                        PointRef::PinPosition { object, .. } => Some(*object),
                        _ => None,
                    },
                ),
            };
            let identity = rnl_runtime::problem::junction_point_identity(p);
            let existing_junction = identity.is_some()
                && problem
                    .existing_junctions
                    .iter()
                    .any(|q| rnl_runtime::problem::junction_point_identity(q) == identity);
            net.anchors.push(AnchorIn { at, exit, owner, connects_pin: ref_pin(p).is_some(), existing_junction });
            points.push(p.clone());
            Some(points.len() - 1)
        };
        for p in r.pins.iter().filter(|p| ref_pin(p).is_some()) {
            add(p, &mut net, &mut points);
        }
        for &w in r.wires.iter().filter(|w| wires_in.contains(w)) {
            let ends: Vec<usize> =
                problem.wires[w].anchors.iter().filter_map(|p| add(p, &mut net, &mut points)).collect();
            for pair in ends.windows(2) {
                net.joined.push((pair[0], pair[1]));
            }
        }
        layout.nets.push(net);
        refs.push(points);
    }
    Ok((layout, pins, written_net, refs))
}

fn routing_of(problem: &LevelProblem, routed: &track::Routed, refs: &[Vec<PointRef>]) -> Routing {
    let mut routing = Routing { expansions: routed.expansions, ..Routing::default() };
    for (j, out) in routed.outs.iter().enumerate() {
        let offset = routing.branches.len();
        routing
            .branches
            .extend(out.branches.iter().map(|b| Branch { at: (to_f64(&b[0]), to_f64(&b[1])), junction: true }));
        for w in &out.wires {
            let end = |e: End| match e {
                End::Anchor(a) => WirePoint::Anchor(refs[j][a].clone()),
                End::Branch(b) => WirePoint::Branch(offset + b),
            };
            let mut points = vec![end(w.start)];
            let n = w.vertices.len();
            for v in &w.vertices[1..n - 1] {
                points.push(WirePoint::Bend((to_f64(&v[0]), to_f64(&v[1]))));
            }
            points.push(end(w.end));
            routing.wires.push(RoutedWire { route: j, points });
            routing.paths.push(w.vertices.clone());
            routing.drawn.push(j);
        }
        if out.forced {
            routing.forced.push(problem.routes[j].net.clone());
        }
    }
    routing
}

fn ways_fit(s: &Structure, way_of: &BTreeMap<usize, usize>, w: &[usize]) -> bool {
    let mut classes = crate::orient::OrientClasses::new(s.objects.len());
    s.kept.iter().all(|&f| match way_of.get(&f) {
        Some(&i) => {
            let mut unit = s.model.units[f].clone();
            unit.orient = crate::search_choices::effect(&unit.orient, w[i]);
            crate::search_choices::take(&mut classes, &unit.orient, f).is_ok()
        }
        None => crate::search_choices::take(&mut classes, &s.model.units[f].orient, f).is_ok(),
    })
}

fn way_sets(s: &Structure, ways: &[usize], free: &[usize]) -> (Vec<Vec<usize>>, bool) {
    use crate::model::OrientEffect;
    let n = s.objects.len();
    let definite = |e: &OrientEffect| matches!(e, OrientEffect::Fix { .. } | OrientEffect::Relate { .. });
    let way_of: BTreeMap<usize, usize> = s.open.ways.iter().enumerate().map(|(i, way)| (way.fact, i)).collect();
    let mut base = crate::orient::OrientClasses::new(n);
    for &f in &s.kept {
        let mut unit = s.model.units[f].clone();
        match way_of.get(&f) {
            Some(i) if free.contains(i) => continue,
            Some(&i) => unit.orient = crate::search_choices::effect(&unit.orient, ways[i]),
            None => {}
        }
        if definite(&unit.orient) && crate::search_choices::take(&mut base, &unit.orient, f).is_err() {
            return (Vec::new(), false);
        }
    }
    struct Walk<'a> {
        s: &'a Structure,
        free: &'a [usize],
        out: Vec<Vec<usize>>,
        nodes: usize,
        cut: bool,
    }
    impl Walk<'_> {
        fn step(&mut self, depth: usize, classes: &crate::orient::OrientClasses, now: &mut Vec<usize>) {
            if depth == self.free.len() {
                self.out.push(now.clone());
                return;
            }
            let i = self.free[depth];
            let way = &self.s.open.ways[i];
            for t in 0..way.count {
                if !permit(BudgetStep::Structure, self.s.objects.len()) {
                    self.cut = true;
                    return;
                }
                if self.out.len() >= WAY_SETS || self.nodes >= WAY_NODES {
                    self.cut = true;
                    return;
                }
                self.nodes += 1;
                let mut unit = self.s.model.units[way.fact].clone();
                unit.orient = crate::search_choices::effect(&unit.orient, t);
                let mut next = classes.clone();
                let fits = !matches!(unit.orient, OrientEffect::Fix { .. } | OrientEffect::Relate { .. })
                    || crate::search_choices::take(&mut next, &unit.orient, way.fact).is_ok();
                if fits {
                    now[i] = t;
                    self.step(depth + 1, &next, now);
                }
            }
        }
    }
    let mut walk = Walk { s, free, out: Vec::new(), nodes: 0, cut: false };
    walk.step(0, &base, &mut ways.to_vec());
    (walk.out, walk.cut)
}

#[cfg(test)]
pub(super) mod tests {
    #[test]
    fn orientation_counts_do_not_saturate_at_the_wasm_word_size() {
        assert_eq!(super::combination_count([8; 11].into_iter()), 8_589_934_592);
        assert_eq!(super::combination_count([8; 22].into_iter()), u64::MAX);
    }

    use super::*;
    use crate::resolve::tests::{fact, instance, problem_of, Ticks};
    use crate::{RunLimit, SolverOptions, StoppedClock};
    use rnl_runtime::problem::{Reference, RouteIntent, TableCell};

    fn pin(k: usize) -> PointRef {
        PointRef::SymbolPin { object: k, pin: 0, reference: Reference::Anonymous }
    }

    pub(in crate::lines) fn seed(
    ) -> (LevelProblem, Structure, SolverOptions, crate::tree::Evaluated, (Arrangement, GroupTree)) {
        let mut p = problem_of(
            vec![instance(), instance()],
            vec![
                fact(FactKind::Orient { object: 0, code: Orient::MY }),
                fact(FactKind::Orient { object: 1, code: Orient::R0 }),
                fact(FactKind::Table {
                    axis: Axis::X,
                    rows: TableCell::List(vec![
                        TableCell::List(vec![TableCell::Point(PointRef::Origin(0))]),
                        TableCell::Point(PointRef::Origin(1)),
                    ]),
                }),
            ],
        );
        p.version = "0.4".into();
        p.symbols[0].pins.truncate(1);
        p.routes = vec![RouteIntent {
            net: "signal".into(),
            tree: true,
            mode: "auto".into(),
            style: vec![],
            pins: vec![pin(0), pin(1)],
            wires: vec![],
        }];
        let mut options = SolverOptions::without_time_limit();
        options.run_limit = Some(RunLimit::Seed);
        let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
        let s = crate::search_choices::for_layout(&s, &mut Work::default());
        let tree = crate::tree::Tree::new(&p, &s, &options);
        let arr = tree.first(&mut Work::default());
        let (start, _) = tree.evaluate_rounds(&arr, 0, &mut Work::default());
        let arrangement = (arr, s.tree.clone());
        (p, s, options, start, arrangement)
    }

    #[test]
    fn physical_pin_references_keep_the_same_orientation_cost() {
        let (mut p, s, _, mut start, _) = seed();
        start.placed.origins[0] = [int(0), int(0)];
        start.placed.origins[1] = [int(20), int(0)];
        let own = BTreeSet::from([0, 1]);
        let members = [(0, Orient::R0), (1, Orient::R0)];
        let cost = |p: &LevelProblem, ori: &[Orient], joint| {
            let level = Level {
                problem: p,
                s: &s,
                tree: &start.placed,
                nodes: &s.tree,
                weights: (1, 4),
                clear: &BTreeMap::new(),
                hung: BTreeMap::new(),
                leaves: BTreeSet::new(),
                edges: vec![],
                implied_alignments: vec![],
                fact_objects: vec![],
                nearest: BTreeMap::new(),
                flips: BTreeMap::new(),
                levels: true,
                bands: true,
                others: vec![],
            };
            level.rule_cost(&members, &own, ori, joint)
        };
        let terminal = |object| PointRef::Pin { object, pin: 0, terminal: "g".into() };
        let mut nonzero = false;
        for joint in [false, true] {
            for a in Orient::ALL {
                for b in Orient::ALL {
                    let ori = [a, b];
                    p.routes[0].pins = vec![terminal(0), terminal(1)];
                    let expected = cost(&p, &ori, joint);
                    nonzero |= expected > 0;
                    for pins in [vec![pin(0), pin(1)], vec![pin(0), terminal(1)], vec![terminal(0), pin(1)]] {
                        p.routes[0].pins = pins;
                        assert_eq!(cost(&p, &ori, joint), expected, "{a:?}/{b:?}, joint={joint}");
                    }
                    // A position at a pin is a coordinate, not an electrical endpoint.
                    p.routes[0].pins =
                        vec![pin(0), PointRef::PinPosition { object: 1, pin: 0, reference: Reference::Anonymous }];
                    assert_eq!(cost(&p, &ori, joint), 0);
                }
            }
        }
        assert!(nonzero, "the fixture must exercise a bend penalty");
    }

    #[test]
    fn shared_distribution_spans_align_an_inner_group_before_compaction() {
        use rnl_runtime::problem::ProblemGroup;
        let mut facts: Vec<_> = (0..4).map(|object| fact(FactKind::Orient { object, code: Orient::R0 })).collect();
        for middle in [0, 1] {
            facts.push(fact(FactKind::Distribute {
                axis: Axis::X,
                sense: Sense::Increasing,
                members: vec![PointRef::Origin(2), PointRef::Origin(middle), PointRef::Origin(3)],
            }));
        }
        let mut p = problem_of(vec![instance(); 4], facts);
        p.version = "0.4".into();
        p.groups.push(ProblemGroup {
            identity: 0,
            first: true,
            direct: true,
            label: "inner".into(),
            members: vec![0, 1],
            spacing_x: None,
            spacing_y: None,
        });
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
        let s = crate::search_choices::for_layout(&s, &mut Work::default());
        assert_eq!(s.kept.len(), 6);
        let tree = crate::tree::Tree::new(&p, &s, &options);
        let mut arr = tree.first(&mut Work::default());
        let inner = s.tree.nodes.iter().position(|node| node.members == vec![0, 1]).unwrap();
        // The suggestion is side by side, but the two retained distributions
        // imply the inner objects have the same x even before their ends join.
        arr.lists[inner] = Some(Item::List(Dir::Row, vec![Item::Leaf(0), Item::Leaf(1)]));
        let (start, _) = tree.evaluate_rounds(&arr, 0, &mut Work::default());
        let arrangement = (arr, s.tree.clone());
        let run = || {
            settle(
                &p,
                &s,
                &start.placed,
                &arrangement,
                &[],
                &options,
                &BTreeMap::new(),
                None,
                &mut Work::default(),
                &Stop::default(),
            )
        };
        let (result, _) = crate::budget::with(&StoppedClock, RunLimit::Work(100_000_000), None, run);
        let result = result.expect("the group must have a completed candidate");
        let made = result.made.expect("the root must be compacted");
        assert!(!made.failed && !made.interrupted, "{:?}", made.notes);
        assert!(made.placed.overlaps.is_empty());
        let geometry = &made.placed.geometry;
        assert_eq!(geometry.origin[0][0], geometry.origin[1][0]);
        for &f in &s.kept {
            let at = |r: &PointRef, a: Axis| point_value(&p, geometry, r, usize::from(a == Axis::Y), 0);
            let ori = |k| geometry.orient.get(k).copied();
            assert_eq!(crate::holds::check_problem(&p, f, &at, &ori, &int(0), &int(0)), Some(true));
        }
    }

    #[test]
    fn listed_orientation_choices_keep_local_pin_orders() {
        for related in [false, true] {
            let object = usize::from(related);
            let outside = object + 1;
            let mut p = problem_of(
                vec![instance(); outside + 1],
                vec![
                    fact(if related {
                        FactKind::OrientRelation { relation: Orient::R0, from: vec![0], to: vec![1] }
                    } else {
                        FactKind::Orient { object: 0, code: Orient::R0 }
                    }),
                    fact(FactKind::Order {
                        axis: Axis::X,
                        sense: Sense::Increasing,
                        members: vec![PointRef::Pin { object, pin: 0, terminal: "a".into() }, PointRef::Origin(object)],
                    }),
                ],
            );
            p.facts.extend([
                fact(FactKind::Orient { object: outside, code: Orient::R0 }),
                fact(FactKind::Order {
                    axis: Axis::X,
                    sense: Sense::Increasing,
                    members: vec![
                        PointRef::Pin { object: outside, pin: 0, terminal: "a".into() },
                        PointRef::Origin(outside),
                    ],
                }),
            ]);
            p.version = "0.4".into();
            p.orient_lists = vec![(0, vec![Orient::R0, Orient::MY]), (2, vec![Orient::R0, Orient::MY])];
            p.symbols[0].pins[0].pos = (-2., 0.);
            let options = SolverOptions::without_time_limit();
            let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
            let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
            let s = crate::search_choices::for_layout(&s, &mut Work::default());
            assert_eq!(s.kept.len(), 4);
            assert_eq!(s.open.classes.is_empty(), !related);
            let tree = crate::tree::Tree::new(&p, &s, &options);
            let mut arr = tree.first(&mut Work::default());
            let (start, _) = tree.evaluate_rounds(&arr, 0, &mut Work::default());
            // Another group's pending alternative must not prevent this group's
            // legal orientations from being listed. It will be checked there.
            let pending = s.open.ways.iter().position(|w| w.fact == 2).unwrap();
            arr.ways[pending] = 1;
            let level = Level {
                problem: &p,
                s: &s,
                tree: &start.placed,
                nodes: &s.tree,
                weights: (1, 4),
                clear: &BTreeMap::new(),
                hung: BTreeMap::new(),
                leaves: BTreeSet::new(),
                edges: vec![Vec::new(); p.facts.len()],
                implied_alignments: vec![],
                fact_objects: vec![BTreeSet::new(); p.facts.len()],
                nearest: BTreeMap::new(),
                flips: BTreeMap::new(),
                levels: true,
                bands: true,
                others: vec![],
            };
            let own: Vec<_> = (0..=object).collect();
            let run = || {
                level.combos(
                    &own,
                    &arr.values,
                    &arr.ways,
                    &start.placed.geometry.orient,
                    &BTreeSet::new(),
                    &BTreeSet::new(),
                    LISTED,
                )
            };
            let (combos, classes, ways, _) = run();
            assert!(!combos.is_empty());
            for (values, chosen, _) in combos {
                let mut trial = arr.values.clone();
                let mut alternatives = arr.ways.clone();
                for (c, v) in classes.iter().zip(values) {
                    trial[*c] = v;
                }
                for (w, v) in ways.iter().zip(chosen) {
                    alternatives[*w] = v;
                }
                let ori = orientations_given(&s, &trial, &alternatives);
                let offset = s.pin_offset(object, 0, ori[object]).unwrap();
                assert!(offset[0] <= int(-1), "related={related}, orientations={ori:?}");
            }
            for seconds in [0., 1., 10., 100.] {
                let (timed, report) = crate::budget::with(&Ticks::default(), RunLimit::Time(seconds), None, run);
                let (replayed, _) = crate::budget::with(&StoppedClock, RunLimit::Work(report.work), None, run);
                assert_eq!(timed, replayed);
            }
        }
    }

    #[test]
    fn line_groups_route_and_keep_nested_table_facts_under_replayable_cutoffs() {
        let (p, s, options, start, arrangement) = seed();
        let run = || {
            settle(
                &p,
                &s,
                &start.placed,
                &arrangement,
                &[],
                &options,
                &BTreeMap::new(),
                None,
                &mut Work::default(),
                &Stop::default(),
            )
        };
        let (full, report) = crate::budget::with(&StoppedClock, RunLimit::Work(10_000_000), None, run);
        let full = full.expect("a small group must settle completely");
        assert_eq!(report.reason, crate::StopReason::Complete);
        let made = full.made.as_ref().expect("the level is compacted before ranking");
        assert!(!made.failed && !made.interrupted);
        assert!(!made.routing.wires.is_empty());
        assert_eq!(made.routing.joins_left(), 0);
        for &f in &s.kept {
            let at = |r: &PointRef, a: Axis| point_value(&p, &made.placed.geometry, r, usize::from(a == Axis::Y), 0);
            let ori = |k| made.placed.geometry.orient.get(k).copied();
            assert_eq!(crate::holds::check_problem(&p, f, &at, &ori, &int(0), &int(0)), Some(true));
        }
        let snapshot = |result: Result<Settled, String>| {
            result
                .map(|r| (r.placed, r.routing, r.notes, r.made.map(|m| (m.placed, m.routing, m.failed, m.interrupted))))
        };
        for seconds in [0., 1., 10., 100., 500., 2000.] {
            let (timed, used) = crate::budget::with(&Ticks::default(), RunLimit::Time(seconds), None, run);
            let (work, _) = crate::budget::with(&StoppedClock, RunLimit::Work(used.work), None, run);
            assert_eq!(snapshot(timed), snapshot(work), "cutoff {seconds}, work {}", used.work);
        }
    }

    #[test]
    fn large_settled_groups_list_single_class_moves_instead_of_cartesian_products() {
        let mut previous = None;
        for n in [16, 32, 64] {
            let mut p = problem_of(vec![instance(); n], vec![]);
            p.version = "0.4".into();
            p.symbols[0].pins.truncate(1);
            p.symbols[0].pins[0].pos = (1., 2.);
            let options = SolverOptions::without_time_limit();
            let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
            let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
            let s = crate::search_choices::for_layout(&s, &mut Work::default());
            let arrangement = crate::tree::Tree::new(&p, &s, &options).first(&mut Work::default());
            let mut geometry = resolved.witness.geometry.clone();
            geometry.orient = orientations_given(&s, &arrangement.values, &arrangement.ways);
            let placed = Placed {
                origins: s.origins(&p, &geometry),
                geometry,
                dropped: vec![],
                overlaps: vec![],
                optimised: false,
                undecided: false,
            };
            let level = Level {
                problem: &p,
                s: &s,
                tree: &placed,
                nodes: &s.tree,
                weights: (1, 4),
                clear: &BTreeMap::new(),
                hung: BTreeMap::new(),
                leaves: BTreeSet::new(),
                edges: vec![],
                implied_alignments: vec![],
                fact_objects: vec![],
                nearest: BTreeMap::new(),
                flips: BTreeMap::new(),
                levels: true,
                bands: true,
                others: vec![],
            };
            let own: Vec<_> = (0..n).collect();
            let loose: BTreeSet<_> = own.iter().copied().collect();
            let _counters = Counters::enter();
            let (combos, classes, _, total) = level.combos(
                &own,
                &arrangement.values,
                &arrangement.ways,
                &placed.geometry.orient,
                &BTreeSet::new(),
                &loose,
                LISTED,
            );
            assert_eq!(classes.len(), n);
            assert!(combos.len() > n && combos.len() <= 1 + 7 * n);
            assert_eq!(total, combos.len() as u64, "listing must finish without a truncation cap");
            for (values, _, _) in &combos {
                assert!(classes.iter().zip(values).filter(|(c, v)| arrangement.values[**c] != **v).count() <= 1);
            }
            if let Some(before) = previous {
                assert_eq!(combos.len() - 1, 2 * (before - 1));
            }
            previous = Some(combos.len());
        }
    }

    #[test]
    fn cache_eviction_counts_lists_still_owned_by_search_cores() {
        let mut lists = Lists::default();
        let bytes = combo_bytes(48, 0);
        let mut owners = Vec::new();
        for n in 0..8 {
            let room = lists.room(bytes);
            let list = Rc::new(vec![(vec![], vec![], (0, 0, 0, 0, 0)); room]);
            lists.insert((n, vec![], vec![], vec![]), (list.clone(), (0..48).collect(), vec![], 0));
            owners.push(list);
            let (count, held) = lists.held();
            assert!(count <= CACHED && held <= CACHE_BYTES);
        }
        assert_eq!(lists.room(bytes), 0, "eviction cannot reclaim live core lists");
        owners.clear();
        lists.cached.clear();
        assert_eq!(lists.held(), (0, 0));
        assert_eq!(lists.room(bytes), LISTED.min(LIST_BYTES / bytes));
    }

    #[test]
    fn free_group_listing_is_bounded_and_time_work_replays_its_prefix() {
        let n = 48;
        let mut p = problem_of(vec![instance(); n], vec![]);
        p.version = "0.4".into();
        p.symbols[0].pins.truncate(1);
        p.symbols[0].pins[0].pos = (1., 2.);
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
        let s = crate::search_choices::for_layout(&s, &mut Work::default());
        let arrangement = crate::tree::Tree::new(&p, &s, &options).first(&mut Work::default());
        let mut geometry = resolved.witness.geometry.clone();
        geometry.orient = orientations_given(&s, &arrangement.values, &arrangement.ways);
        let placed = Placed {
            origins: s.origins(&p, &geometry),
            geometry,
            dropped: vec![],
            overlaps: vec![],
            optimised: false,
            undecided: false,
        };
        let level = Level {
            problem: &p,
            s: &s,
            tree: &placed,
            nodes: &s.tree,
            weights: (1, 4),
            clear: &BTreeMap::new(),
            hung: BTreeMap::new(),
            leaves: BTreeSet::new(),
            edges: vec![],
            implied_alignments: vec![],
            fact_objects: vec![],
            nearest: BTreeMap::new(),
            flips: BTreeMap::new(),
            levels: true,
            bands: true,
            others: vec![],
        };
        let run = |room| {
            level.combos(
                &(0..n).collect::<Vec<_>>(),
                &arrangement.values,
                &arrangement.ways,
                &placed.geometry.orient,
                &BTreeSet::new(),
                &BTreeSet::new(),
                room,
            )
        };
        let limited = run(257);
        assert_eq!(limited.0.len(), 257);
        assert_eq!(limited.3, u64::MAX);
        assert!(limited.0.iter().all(|(v, w, _)| v.len() == n && w.is_empty()));
        struct Wall(std::time::Instant);
        impl crate::Clock for Wall {
            fn elapsed(&self) -> f64 {
                self.0.elapsed().as_secs_f64()
            }
        }
        let clock = Wall(std::time::Instant::now());
        let (timed, report) = crate::budget::with(&clock, RunLimit::Time(1.), None, || run(LISTED));
        assert!(clock.0.elapsed().as_secs_f64() < 6., "listing ignored its time limit");
        assert!(timed.0.len() <= LISTED);
        assert_eq!(timed.3, u64::MAX);
        let (replay, _) = crate::budget::with(&StoppedClock, RunLimit::Work(report.work), None, || run(LISTED));
        assert_eq!(timed, replay);
        // A real full solve also has to return promptly, including its initial
        // tree candidate and deterministic materialization after search stops.
        let mut options = options;
        options.settings.layout = crate::Layout::Lines;
        options.run_limit = Some(RunLimit::Time(1.));
        let clock = Wall(std::time::Instant::now());
        let _ = crate::solve_with_clock(&p, &options, &clock);
        assert!(clock.0.elapsed().as_secs_f64() < 6., "solve exceeded its bounded cleanup");
    }

    #[test]
    fn exhausted_budget_stops_orientation_listing_before_geometry_is_measured() {
        let (p, s, _, start, arrangement) = seed();
        let level = Level {
            problem: &p,
            s: &s,
            tree: &start.placed,
            nodes: &arrangement.1,
            weights: (1, 4),
            clear: &BTreeMap::new(),
            hung: BTreeMap::new(),
            leaves: BTreeSet::new(),
            edges: vec![Vec::new(); p.facts.len()],
            implied_alignments: vec![],
            fact_objects: vec![BTreeSet::new(); p.facts.len()],
            nearest: BTreeMap::new(),
            flips: BTreeMap::new(),
            levels: true,
            bands: true,
            others: vec![],
        };
        let _counters = Counters::enter();
        let (listed, report) = crate::budget::with(&StoppedClock, RunLimit::Work(0), None, || {
            level.combos(
                &[0, 1],
                &arrangement.0.values,
                &arrangement.0.ways,
                &start.placed.geometry.orient,
                &BTreeSet::new(),
                &BTreeSet::new(),
                LISTED,
            )
        });
        assert_eq!(report.reason, crate::StopReason::Work);
        assert!(listed.0.is_empty());
        assert_eq!(report.work, 0);
    }

    #[test]
    fn routing_anchors_keep_electrical_and_position_identities_separate() {
        let a = pin(0);
        let b = PointRef::Pin { object: 0, pin: 0, terminal: "1".into() };
        let position = PointRef::PinPosition { object: 0, pin: 0, reference: Reference::Anonymous };
        assert!(same_point(&a, &b));
        assert!(!same_point(&a, &position));
        assert!(same_point(&position, &position));
        assert!(!same_point(&position, &PointRef::Origin(0)));
    }

    #[test]
    fn geometry_scope_follows_complete_centres_and_indirect_symbol_positions() {
        let mut objects = vec![instance(), instance(), instance(), instance()];
        objects[2].position = Position::At(PointRef::Centre(vec![PointRef::Origin(0), PointRef::Origin(1)]));
        objects[3].position = Position::At(pin(2));
        let p = problem_of(objects, vec![]);
        assert_eq!(geometry_members(&p, &[0].into()), [0].into());
        assert_eq!(geometry_members(&p, &[0, 1].into()), [0, 1, 2, 3].into());
    }
}
