//! Layout by nested rows and columns of whole groups. Each arrangement specifies list
//! order, orientation choices and optional pin bindings. Exact placement admits these
//! preferences below all kept facts, then routing and the configured objectives rank it.
//! The search tries local list edits, coordinated turns and mirrors, and can release its
//! most recent binding after reaching a local minimum.

use std::collections::{BTreeMap, BTreeSet};

use num_traits::{One, Zero};
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{Axis, FactKind, LevelProblem, PointRef, Sense};

use crate::coords::{place_tree, Placed, Pull, Tie};
use crate::driver::count;
use crate::model::PointId;
use crate::orient::inverse;
use crate::rational::{int, Q};
use crate::resolve::point_value;
use crate::route::{route, Routing, CLEARANCE};
use crate::score::{drawn, measured_layers, Entry};
use crate::search::{
    attach_offset, attachments, effective_box, empty_box, grid_of, orientation_relations,
    orientations_with as orientations, preference, ranked_values, up, with_relations, Attach, Candidate, RelKind,
    Relation, Side, Tagged, Term, HGAP, VGAP,
};
use crate::structure::{GroupTree, NodeKind, Pair, PinKey, PortKind, Role, Source, Structure};
use crate::{SolverOptions, Work};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Dir {
    Row,
    Column,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Item {
    Leaf(usize),
    Group(usize),
    List(Dir, Vec<Item>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Arrangement {
    pub lists: Vec<Option<Item>>,
    pub values: Vec<Orient>,
    pub ways: Vec<usize>,
    pub flipped: BTreeSet<usize>,
    pub bound: Vec<Bound>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Bound {
    pub tie: Tie,
    pub together: Option<(usize, usize, Dir)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Step {
    list: usize,
    dir: Dir,
    index: usize,
    len: usize,
}

#[derive(Clone, Debug, Default)]
struct Walk {
    chains: BTreeMap<usize, Vec<Step>>,
    lists: Vec<(Dir, Vec<Vec<usize>>)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outer {
    Left,
    Right,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Open {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Debug, Default)]
struct Rules {
    left: BTreeSet<(usize, usize)>,
    above: BTreeSet<(usize, usize)>,
    above_pins: BTreeSet<(usize, usize)>,
    same_y: Vec<(usize, usize)>,
    same_x: Vec<(usize, usize)>,
}

#[derive(Clone, Debug)]
pub(crate) struct Evaluated {
    pub placed: Placed,
    pub routing: Routing,
    pub entry: Entry,
}

pub(crate) struct Laid {
    pub candidate: Candidate,
    pub placed: Placed,
    pub routing: Routing,
    pub entry: Entry,
    pub notes: Vec<String>,
    #[allow(dead_code)] // consumed by layouts that start from this arrangement
    pub arrangement: Option<(Arrangement, GroupTree)>,
    #[allow(dead_code)] // alternative arrangements retained for subsequent layout stages
    pub others: Vec<Arrangement>,
}

pub(crate) struct Tree<'a> {
    problem: &'a LevelProblem,
    s: &'a Structure,
    options: &'a SolverOptions,
    groups: GroupTree,
    left_out: Vec<String>,
    leaves: Vec<usize>,
    node_of: BTreeMap<usize, usize>,
    level: BTreeMap<usize, i64>,
    pairs: Vec<Pair>,
    rules: Rules,
    line_rank: Option<crate::lines::Rank>,
}

const ROUNDS: usize = 3;

const RELEASES: usize = 3;

/// Description, arrangement, moved leaves, and whether it turns or mirrors those leaves.
type Move = (String, Arrangement, Vec<usize>, bool);

#[derive(Default)]
pub(crate) struct Stop {
    placed: std::cell::Cell<u64>,
    routed: std::cell::Cell<u64>,
    best_at: std::cell::Cell<Option<u64>>,
}
impl Stop {
    pub(crate) fn reached(&self, _work: &Work) -> bool {
        !crate::budget::permit(crate::budget::Step::Candidate, 1)
    }
}

pub(crate) fn lay_out(
    problem: &LevelProblem,
    s: &Structure,
    options: &SolverOptions,
    work: &mut Work,
    stop: &Stop,
) -> Laid {
    lay_out_from(Tree::new(problem, s, options), work, stop, None)
}

pub(crate) fn lay_out_from(tree: Tree<'_>, work: &mut Work, stop: &Stop, allowance: Option<u64>) -> Laid {
    let started = crate::budget::consumed();
    let first = tree.first(work);
    let mut notes = Vec::new();
    if !tree.left_out.is_empty() {
        notes.push(format!(
            "tree: {} the tree leaves out, as each crosses one taken in before: {}",
            count(tree.left_out.len(), "group"),
            tree.left_out.join("; ")
        ));
    }
    notes.push(format!("tree: the first arrangement: {}", tree.describe(&first)));
    stop.placed.set(stop.placed.get() + 1);
    stop.routed.set(stop.routed.get() + 1);
    let rounds = ROUNDS;
    let (best, taken) = tree.evaluate_rounds(&first, rounds, work);
    if rounds > 0 {
        notes.push(format!("tree: {taken} of {rounds} rounds of straightening after routing kept"));
    }
    let broken = tree.broken_rules(&first);
    if !broken.is_empty() {
        notes.push(format!(
            "tree: the first arrangement breaks {} of the facts' rules for lists: {}",
            broken.len(),
            broken.join("; ")
        ));
    }
    let start = first.clone();
    // The shared budget always bounds initialization. A local stage allowance may stop
    // subsequent improvements, but must not manufacture an unfinished initial baseline.
    let remaining = allowance.map(|limit| limit.saturating_sub(crate::budget::consumed().saturating_sub(started)));
    let search = || tree.search(first, best, rounds, true, work, stop);
    let (arr, best, said, path) = match remaining {
        Some(limit) => crate::budget::phase(limit, search).0,
        None => search(),
    };
    notes.extend(said);
    let mut others: Vec<Arrangement> = Vec::new();
    for a in std::iter::once(start).chain(path) {
        if a != arr && !others.contains(&a) {
            others.push(a);
        }
    }
    if let Some(rank) = &tree.line_rank {
        notes.push(rank.note(tree.s));
    }
    let Evaluated { placed, routing, entry } = tree.given(best);
    let candidate = Candidate {
        values: arr.values.clone(),
        ways: arr.ways.clone(),
        rows: Vec::new(),
        apart: Vec::new(),
        even: false,
        columns: false,
        origin: "tree",
    };
    let arrangement = Some((arr.clone(), tree.groups.clone()));
    Laid { candidate, placed, routing, entry, notes, arrangement, others }
}

impl<'a> Tree<'a> {
    pub(crate) fn new(problem: &'a LevelProblem, s: &'a Structure, options: &'a SolverOptions) -> Tree<'a> {
        let (groups, left_out) = (s.tree.clone(), Vec::new());
        let mut node_of: BTreeMap<usize, usize> = BTreeMap::new();
        for (n, node) in groups.nodes.iter().enumerate() {
            for &k in &node.own {
                node_of.insert(k, n);
            }
        }
        let ports: Vec<usize> = (0..s.objects.len())
            .filter(|&k| {
                let o = &s.objects[k];
                o.role == Role::Port
                    && o.own
                    && !o.left_free
                    && o.stub.is_none()
                    && o.symbol.is_some()
                    && crate::tree_hang::host(problem, k).is_none()
                    && !s.beside_by_netlist(k)
            })
            .collect();
        let members: Vec<Option<BTreeSet<usize>>> = groups
            .nodes
            .iter()
            .map(|n| match n.kind {
                NodeKind::Group(g) => problem.groups.get(g).map(|g| g.members.iter().copied().collect()),
                NodeKind::Union(h, g) => {
                    let mut set: BTreeSet<usize> = BTreeSet::new();
                    for x in [h, g] {
                        set.extend(problem.groups.get(x).map_or(&[][..], |g| &g.members[..]).iter().copied());
                    }
                    Some(set)
                }
                _ => None,
            })
            .collect();
        for &k in &ports {
            let node = (0..groups.nodes.len())
                .filter(|&n| members[n].as_ref().is_some_and(|m| m.contains(&k)))
                .min_by_key(|&n| groups.nodes[n].members.len())
                .unwrap_or(0);
            node_of.insert(k, node);
        }
        let mut leaves: Vec<usize> = s.elements.iter().copied().chain(ports.iter().copied()).collect();
        leaves.sort_unstable();
        let pairs = s.pairs.clone();
        let mut tree = Tree {
            problem,
            s,
            options,
            groups,
            left_out,
            leaves,
            node_of,
            level: BTreeMap::new(),
            pairs,
            rules: Rules::default(),
            line_rank: None,
        };
        if !crate::budget::permit(crate::budget::Step::Structure, tree.leaves.len()) {
            return tree;
        }
        tree.rules = tree.rules();
        if !crate::budget::stopped() {
            tree.level = tree.levels();
        }
        tree
    }

    fn touched(&self, f: usize, axis: usize) -> Vec<usize> {
        let mut out = BTreeSet::new();
        for p in &self.s.model.units[f].touched[axis] {
            let k = match p {
                PointId::Origin(k) | PointId::Pin(k, _) | PointId::PinPosition(k, _) => *k,
                _ => continue,
            };
            if self.node_of.contains_key(&k) {
                out.insert(k);
            }
        }
        out.into_iter().collect()
    }

    fn leaves_of(&self, r: &PointRef, out: &mut Vec<usize>) {
        match r {
            PointRef::Origin(k)
            | PointRef::Pin { object: k, .. }
            | PointRef::SymbolPin { object: k, .. }
            | PointRef::PinPosition { object: k, .. } => {
                if self.node_of.contains_key(k) {
                    out.push(*k);
                }
            }
            PointRef::Centre(members) => {
                for m in members {
                    self.leaves_of(m, out);
                }
            }
        }
    }

    fn aligned_on(&self, f: usize, on: Axis) -> bool {
        match &self.problem.facts[f].kind {
            FactKind::Align { axis, .. } => *axis == on,
            _ => false,
        }
    }

    fn directed(&self, axis: Axis, origins: bool) -> BTreeSet<(usize, usize)> {
        let a = usize::from(axis == Axis::Y);
        let at = |ms: &[PointRef]| -> Vec<(usize, Q)> {
            let mut out = Vec::new();
            for m in ms.iter().filter(|m| {
                !(origins
                    && matches!(m, PointRef::Pin { .. } | PointRef::SymbolPin { .. } | PointRef::PinPosition { .. }))
            }) {
                let Some(v) = point_value(self.problem, &self.s.witness, m, a, 0) else { continue };
                let mut ks = Vec::new();
                self.leaves_of(m, &mut ks);
                out.extend(ks.into_iter().map(|k| (k, v.clone())));
            }
            out
        };
        let before = |p: &Q, q: &Q| if a == 0 { p < q } else { p > q };
        let mut out = BTreeSet::new();
        for &f in &self.s.kept {
            if !crate::budget::permit(crate::budget::Step::Structure, self.leaves.len()) {
                break;
            }
            match &self.problem.facts[f].kind {
                FactKind::Order { axis: on, sense, members } | FactKind::Distribute { axis: on, sense, members }
                    if *on == axis && *sense != Sense::Either =>
                {
                    let at = at(members);
                    for (x, vx) in &at {
                        if !crate::budget::permit(crate::budget::Step::Structure, at.len()) {
                            return out;
                        }
                        out.extend(at.iter().filter(|(y, vy)| x != y && before(vx, vy)).map(|(y, _)| (*x, *y)));
                    }
                }
                FactKind::Distance { axis: on, sense, min, from, to, .. }
                    if *on == axis && *sense != Sense::Either && min.is_some_and(|m| m >= 0.0) =>
                {
                    let (f, t) = (at(from), at(to));
                    for (x, vx) in f.iter().chain(&t) {
                        for (y, vy) in f.iter().chain(&t) {
                            if !crate::budget::permit(crate::budget::Step::Structure, f.len().saturating_add(t.len())) {
                                return out;
                            }
                            let across = f.iter().any(|(k, _)| k == x) != f.iter().any(|(k, _)| k == y);
                            if x != y && across && before(vx, vy) {
                                out.insert((*x, *y));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn rules(&self) -> Rules {
        let above = self.directed(Axis::Y, false);
        let above_pins = above.difference(&self.directed(Axis::Y, true)).copied().collect();
        let mut rules = Rules { left: self.directed(Axis::X, false), above, above_pins, ..Rules::default() };
        for &f in &self.s.kept {
            if !crate::budget::permit(crate::budget::Step::Structure, self.leaves.len()) {
                break;
            }
            for (on, list) in [(Axis::Y, &mut rules.same_y), (Axis::X, &mut rules.same_x)] {
                if self.aligned_on(f, on) {
                    let ks = self.touched(f, usize::from(on == Axis::Y));
                    list.extend(ks.windows(2).map(|w| (w[0], w[1])));
                }
            }
        }
        rules
    }

    fn left_of(&self, flipped: &BTreeSet<usize>) -> BTreeSet<(usize, usize)> {
        let mut out = self.rules.left.clone();
        for (r, rev) in self.s.reversible.iter().enumerate() {
            let flip = flipped.contains(&r);
            out.extend(rev.pairs.iter().map(|&(a, b)| if flip { (b, a) } else { (a, b) }));
        }
        out
    }

    pub(crate) fn broken(&self, arr: &Arrangement) -> usize {
        self.broken_rules(arr).len()
    }

    pub(crate) fn broken_rules(&self, arr: &Arrangement) -> Vec<String> {
        let walk = self.walk(arr);
        let mut out = Vec::new();
        let lca = |a: usize, b: usize| lca(&walk, a, b);
        let name = |k: usize| self.leaf_name(k);
        for &(a, b) in &self.left_of(&arr.flipped) {
            if let Some((_, Dir::Row, ia, ib)) = lca(a, b) {
                if ia > ib {
                    out.push(format!("{} left of {}", name(a), name(b)));
                }
            }
        }
        for &(a, b) in &self.rules.above {
            if let Some((_, Dir::Column, ia, ib)) = lca(a, b) {
                if ia > ib {
                    out.push(format!("{} above {}", name(a), name(b)));
                }
            }
        }
        for &(a, b) in &self.rules.same_y {
            if matches!(lca(a, b), Some((_, Dir::Column, ..))) {
                out.push(format!("{} on the height of {}", name(a), name(b)));
            }
        }
        for &(a, b) in &self.rules.same_x {
            if matches!(lca(a, b), Some((_, Dir::Row, ..))) {
                out.push(format!("{} on the vertical line of {}", name(a), name(b)));
            }
        }
        for b in &arr.bound {
            let Some((x, y, dir)) = b.together else { continue };
            match lca(x, y) {
                Some((_, d, ia, ib)) if d == dir && ia.abs_diff(ib) == 1 => {}
                _ => out.push(format!("{} bound to {}", name(x), name(y))),
            }
        }
        out
    }

    fn levels(&self) -> BTreeMap<usize, i64> {
        let s = self.s;
        let mut level: BTreeMap<usize, i64> = BTreeMap::new();
        for (r, row) in s.rows.iter().enumerate() {
            for &k in row {
                level.insert(k, r as i64);
            }
        }
        let last = s.rows.len() as i64;
        for &k in &s.elements {
            level.entry(k).or_insert(last);
        }
        crate::tree_hang::levels(s, &mut level);
        let mut class: BTreeMap<usize, usize> = s.elements.iter().map(|&k| (k, k)).collect();
        let find = |class: &BTreeMap<usize, usize>, mut k: usize| {
            while class[&k] != k {
                k = class[&k];
            }
            k
        };
        let mut joins: Vec<(usize, usize)> = Vec::new();
        for &(a, b) in &self.rules.same_y {
            joins.push((a, b));
        }
        for p in &self.pairs {
            joins.push((p.a, p.b));
        }
        for (a, b) in joins {
            if !(class.contains_key(&a) && class.contains_key(&b)) {
                continue;
            }
            let (ra, rb) = (find(&class, a), find(&class, b));
            if ra != rb {
                class.insert(ra.max(rb), ra.min(rb));
            }
        }
        let mut of_class: BTreeMap<usize, i64> = BTreeMap::new();
        for &k in &s.elements {
            let r = find(&class, k);
            let l = level[&k];
            let e = of_class.entry(r).or_insert(l);
            *e = (*e).min(l);
        }
        let above: Vec<(usize, usize, i64)> = self
            .rules
            .above
            .iter()
            .filter(|(a, b)| class.contains_key(a) && class.contains_key(b))
            .map(|&(a, b)| (find(&class, a), find(&class, b), i64::from(!self.rules.above_pins.contains(&(a, b)))))
            .filter(|(a, b, _)| a != b)
            .collect();
        for _ in 0..of_class.len() {
            if !crate::budget::permit(crate::budget::Step::Structure, above.len()) {
                break;
            }
            let mut moved = false;
            for &(a, b, down) in &above {
                let la = of_class[&a];
                if of_class[&b] < la + down {
                    of_class.insert(b, la + down);
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
        let used: Vec<i64> = of_class.values().copied().collect::<BTreeSet<i64>>().into_iter().collect();
        s.elements
            .iter()
            .map(|&k| {
                let l = of_class[&find(&class, k)];
                (k, used.iter().position(|u| *u == l).unwrap_or(0) as i64)
            })
            .collect()
    }

    fn port_level(&self, k: usize) -> i64 {
        for &(a, b) in &self.rules.same_y {
            let other = if a == k {
                b
            } else if b == k {
                a
            } else {
                continue;
            };
            if let Some(l) = self.level.get(&other) {
                return *l;
            }
        }
        let mut ls: Vec<i64> = self.s.attach_pins(k).iter().filter_map(|p| self.level.get(&p.0).copied()).collect();
        ls.sort_unstable();
        if ls.is_empty() {
            return 0;
        }
        ls[(ls.len() - 1) / 2]
    }

    fn span(&self, arr: &Arrangement, item: &Item) -> (i64, i64) {
        let mut ls = Vec::new();
        for k in self.item_leaves(arr, item) {
            ls.push(match self.level.get(&k) {
                Some(l) => *l,
                None => self.port_level(k),
            });
        }
        (ls.iter().copied().min().unwrap_or(0), ls.iter().copied().max().unwrap_or(0))
    }

    fn item_leaves(&self, arr: &Arrangement, item: &Item) -> Vec<usize> {
        let mut out = Vec::new();
        self.collect(arr, item, &mut out);
        out
    }

    fn collect(&self, arr: &Arrangement, item: &Item, out: &mut Vec<usize>) {
        match item {
            Item::Leaf(k) => out.push(*k),
            Item::Group(n) => {
                if let Some(it) = arr.lists.get(*n).and_then(|x| x.as_ref()) {
                    self.collect(arr, it, out);
                }
            }
            Item::List(_, items) => {
                for it in items {
                    self.collect(arr, it, out);
                }
            }
        }
    }

    fn walk(&self, arr: &Arrangement) -> Walk {
        let mut w = Walk::default();
        if let Some(root) = arr.lists.first().and_then(|x| x.as_ref()) {
            self.walk_item(arr, root, &mut Vec::new(), &mut w);
        }
        w
    }

    fn walk_item(&self, arr: &Arrangement, item: &Item, chain: &mut Vec<Step>, w: &mut Walk) {
        match item {
            Item::Leaf(k) => {
                w.chains.insert(*k, chain.clone());
            }
            Item::Group(n) => {
                if let Some(it) = arr.lists.get(*n).and_then(|x| x.as_ref()) {
                    self.walk_item(arr, it, chain, w);
                }
            }
            Item::List(dir, items) => {
                let id = w.lists.len();
                let leaves: Vec<Vec<usize>> = items.iter().map(|it| self.item_leaves(arr, it)).collect();
                w.lists.push((*dir, leaves));
                for (index, it) in items.iter().enumerate() {
                    chain.push(Step { list: id, dir: *dir, index, len: items.len() });
                    self.walk_item(arr, it, chain, w);
                    chain.pop();
                }
            }
        }
    }

    pub(crate) fn describe(&self, arr: &Arrangement) -> String {
        let mut parts = Vec::new();
        for (n, list) in arr.lists.iter().enumerate() {
            let Some(item) = list else { continue };
            let name = if n == 0 { "root".to_string() } else { self.node_name(n) };
            parts.push(format!("{name} {}", self.item_text(item)));
        }
        parts.join("; ")
    }

    fn node_name(&self, n: usize) -> String {
        match self.groups.nodes[n].kind {
            NodeKind::Group(g) => match self.problem.groups.get(g) {
                Some(group) if group.direct => group.label.clone(),
                _ => {
                    let members = &self.groups.nodes[n].members;
                    let mut names: Vec<&str> =
                        members.iter().take(3).map(|&k| self.problem.objects[k].label.as_str()).collect();
                    if members.len() > 3 {
                        names.push("...");
                    }
                    format!("group({})", names.join(" "))
                }
            },
            _ => format!("node {n}"),
        }
    }

    fn leaf_name(&self, k: usize) -> String {
        if self.s.objects[k].role == Role::Port {
            if let Some(net) = self.s.ports.iter().find(|p| p.object == k).and_then(|p| p.net.clone()) {
                return format!("port &{net}");
            }
        }
        self.problem.objects[k].label.clone()
    }

    fn item_text(&self, item: &Item) -> String {
        match item {
            Item::Leaf(k) => self.leaf_name(*k),
            Item::Group(n) => self.node_name(*n),
            Item::List(dir, items) => {
                let inner: Vec<String> = items.iter().map(|i| self.item_text(i)).collect();
                format!("{}[{}]", if *dir == Dir::Row { "row" } else { "column" }, inner.join(", "))
            }
        }
    }

    fn code(&self, arr: &Arrangement) -> Vec<u32> {
        let ori = orientations(self.s, &arr.values, &arr.ways);
        let mut code: Vec<u32> = (0..self.s.objects.len())
            .filter(|&k| self.s.objects[k].oriented)
            .map(|k| u32::from(crate::orient::index(ori[k])))
            .collect();
        fn write(item: &Item, code: &mut Vec<u32>) {
            match item {
                Item::Leaf(k) => code.push(10 + 2 * *k as u32),
                Item::Group(n) => code.push(11 + 2 * *n as u32),
                Item::List(dir, items) => {
                    // A singleton adds no ordering. Giving it a distinct tie code can reward
                    // endlessly wrapping the same drawing in rows and columns.
                    if items.len() == 1 {
                        write(&items[0], code);
                        return;
                    }
                    code.push(if *dir == Dir::Row { 0 } else { 2 });
                    for it in items {
                        write(it, code);
                    }
                    code.push(1);
                }
            }
        }
        for (n, list) in arr.lists.iter().enumerate() {
            code.push(u32::MAX - n as u32);
            if let Some(item) = list {
                write(item, &mut code);
            }
        }
        code.extend(arr.ways.iter().map(|w| *w as u32));
        for b in &arr.bound {
            code.extend([u32::MAX, b.tie.a as u32, b.tie.b as u32, b.tie.axis as u32]);
        }
        code
    }

    fn attach(&self, ori: &[Orient]) -> BTreeMap<usize, Attach> {
        attachments(self.s, &[], ori).into_iter().filter(|(k, _)| !self.node_of.contains_key(k)).collect()
    }

    fn boxes(&self, ori: &[Orient]) -> BTreeMap<usize, [Q; 4]> {
        let attach = self.attach(ori);
        self.leaves.iter().map(|&k| (k, effective_box(self.s, k, ori, &attach))).collect()
    }

    fn sketch(&self, arr: &Arrangement, ori: &[Orient]) -> Vec<[Q; 2]> {
        let s = self.s;
        let boxes = self.boxes(ori);
        let mut g = s.witness.clone();
        g.orient = ori.to_vec();
        if let Some(root) = arr.lists.first().and_then(|x| x.as_ref()) {
            let mut sizes: BTreeMap<*const Item, [Q; 2]> = BTreeMap::new();
            self.size(arr, root, &boxes, &mut sizes);
            self.pack(arr, root, Q::zero(), Q::zero(), &boxes, &sizes, &mut g.origin);
        }
        for (j, info) in s.objects.iter().enumerate() {
            if let Some(stub) = &info.stub {
                let (h, hp) = stub.host;
                if let (Some(hpin), Some(sp)) = (s.pin_offset(h, hp, ori[h]), s.pin_offset(j, 0, ori[j])) {
                    let base = g.origin[h].clone();
                    g.origin[j] = [&base[0] + &hpin[0] - &sp[0], &base[1] + &hpin[1] + &stub.rise - &sp[1]];
                }
            }
        }
        for (&k, a) in &self.attach(ori) {
            if let Some(at) = attach_offset(s, k, a, ori) {
                let base = g.origin[a.host.0].clone();
                g.origin[k] = [&base[0] + &at[0], &base[1] + &at[1]];
            }
        }
        s.origins(self.problem, &g)
    }

    fn size(
        &self,
        arr: &Arrangement,
        item: &Item,
        boxes: &BTreeMap<usize, [Q; 4]>,
        sizes: &mut BTreeMap<*const Item, [Q; 2]>,
    ) -> [Q; 2] {
        let out = match item {
            Item::Leaf(k) => {
                let b = boxes.get(k).cloned().unwrap_or_else(empty_box);
                [&b[2] - &b[0], &b[3] - &b[1]]
            }
            Item::Group(n) => match arr.lists.get(*n).and_then(|x| x.as_ref()) {
                Some(it) => self.size(arr, it, boxes, sizes),
                None => [Q::zero(), Q::zero()],
            },
            Item::List(dir, items) => {
                let each: Vec<[Q; 2]> = items.iter().map(|it| self.size(arr, it, boxes, sizes)).collect();
                let gaps = int(items.len().saturating_sub(1) as i64);
                let (along, across) = if *dir == Dir::Row { (0, 1) } else { (1, 0) };
                let total: Q = each.iter().map(|e| e[along].clone()).sum::<Q>()
                    + gaps * int(if *dir == Dir::Row { HGAP } else { VGAP });
                let most = each.iter().map(|e| e[across].clone()).max().unwrap_or_default();
                if *dir == Dir::Row {
                    [total, most]
                } else {
                    [most, total]
                }
            }
        };
        sizes.insert(item as *const Item, out.clone());
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn pack(
        &self,
        arr: &Arrangement,
        item: &Item,
        left: Q,
        top: Q,
        boxes: &BTreeMap<usize, [Q; 4]>,
        sizes: &BTreeMap<*const Item, [Q; 2]>,
        origin: &mut [[Q; 2]],
    ) {
        match item {
            Item::Leaf(k) => {
                let b = boxes.get(k).cloned().unwrap_or_else(empty_box);
                origin[*k] = [&left - &b[0], &top - &b[3]];
            }
            Item::Group(n) => {
                if let Some(it) = arr.lists.get(*n).and_then(|x| x.as_ref()) {
                    self.pack(arr, it, left, top, boxes, sizes, origin);
                }
            }
            Item::List(dir, items) => {
                let whole = sizes[&(item as *const Item)].clone();
                let two = int(2);
                let (mut x, mut y) = (left, top);
                for it in items {
                    let size = sizes[&(it as *const Item)].clone();
                    match dir {
                        Dir::Row => {
                            let t = &y - (&whole[1] - &size[1]) / &two;
                            self.pack(arr, it, x.clone(), t, boxes, sizes, origin);
                            x = &x + &size[0] + int(HGAP);
                        }
                        Dir::Column => {
                            let l = &x + (&whole[0] - &size[0]) / &two;
                            self.pack(arr, it, l, y.clone(), boxes, sizes, origin);
                            y = &y - &size[1] - int(VGAP);
                        }
                    }
                }
            }
        }
    }

    fn node_items(&self, arr: &Arrangement, node: usize) -> Vec<Item> {
        let n = &self.groups.nodes[node];
        let mut items: Vec<Item> = Vec::new();
        for &c in &n.children {
            if arr.lists.get(c).is_some_and(|x| x.is_some()) {
                items.push(Item::Group(c));
            }
        }
        for &k in &self.leaves {
            if self.node_of.get(&k) == Some(&node) {
                items.push(Item::Leaf(k));
            }
        }
        items
    }

    fn flow(&self, arr: &Arrangement, item: &Item) -> i64 {
        if !self.s.has(Source::InputsLeft) {
            return 0;
        }
        let Item::Leaf(k) = item else { return 0 };
        let _ = arr;
        match self.s.ports.iter().find(|p| p.object == *k).map(|p| p.kind) {
            Some(PortKind::Input) => -1,
            Some(PortKind::Output) => 1,
            _ => 0,
        }
    }

    fn order_row(&self, arr: &Arrangement, items: Vec<Item>, key: &dyn Fn(&Item) -> Q) -> Vec<Item> {
        let before = self.left_of(&arr.flipped);
        let sets: Vec<BTreeSet<usize>> =
            items.iter().map(|it| self.item_leaves(arr, it).into_iter().collect()).collect();
        let precedes = |a: usize, b: usize| {
            before.contains(&(a, b))
                || (!before.contains(&(b, a))
                    && self
                        .pairs
                        .iter()
                        .any(|p| (p.a == a && (p.b == b || p.axis == Some(b))) || (p.axis == Some(a) && p.b == b)))
        };
        let n = items.len();
        let mut first: BTreeSet<(usize, usize)> = BTreeSet::new();
        for i in 0..n {
            for j in 0..n {
                if i != j && sets[i].iter().any(|&a| sets[j].iter().any(|&b| precedes(a, b))) {
                    first.insert((i, j));
                }
            }
        }
        let rank = |i: usize| (self.flow(arr, &items[i]), key(&items[i]), sets[i].iter().next().copied().unwrap_or(0));
        let mut done = vec![false; n];
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            let free = |j: usize| !done[j] && (0..n).all(|i| done[i] || i == j || !first.contains(&(i, j)));
            let placed: BTreeSet<usize> = (0..n).filter(|&i| done[i]).flat_map(|i| sets[i].iter().copied()).collect();
            let hung = |j: usize| self.is_port(&items[j]) && crate::tree_hang::joined_within(self.s, &sets[j], &placed);
            let pick = (0..n).filter(|&j| free(j) && hung(j)).min_by(|&a, &b| rank(a).cmp(&rank(b)));
            let pick = pick.or_else(|| (0..n).filter(|&j| free(j)).min_by(|&a, &b| rank(a).cmp(&rank(b))));
            let pick = pick.or_else(|| (0..n).filter(|&j| !done[j]).min_by(|&a, &b| rank(a).cmp(&rank(b))));
            let Some(j) = pick else { break };
            done[j] = true;
            out.push(j);
        }
        let mut slots: Vec<Option<Item>> = items.into_iter().map(Some).collect();
        out.into_iter().filter_map(|j| slots[j].take()).collect()
    }

    fn cut(
        &self,
        arr: &Arrangement,
        items: Vec<Item>,
        key: &dyn Fn(&Item) -> Q,
        ykey: &dyn Fn(&Item) -> Q,
        outer: Outer,
    ) -> Item {
        if items.len() == 1 {
            return items.into_iter().next().expect("one item");
        }
        let spans: Vec<(i64, i64)> = items.iter().map(|it| self.span(arr, it)).collect();
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by_key(|&i| (spans[i], i));
        let mut bands: Vec<Vec<usize>> = Vec::new();
        let mut bottom = i64::MIN;
        for i in order {
            let (t, b) = spans[i];
            match bands.last_mut() {
                Some(band) if t <= bottom => {
                    band.push(i);
                    bottom = bottom.max(b);
                }
                _ => {
                    bands.push(vec![i]);
                    bottom = b;
                }
            }
        }
        let mut slots: Vec<Option<Item>> = items.into_iter().map(Some).collect();
        let mut bands: Vec<Vec<Item>> =
            bands.into_iter().map(|b| b.into_iter().filter_map(|i| slots[i].take()).collect()).collect();
        if bands.len() > 1 {
            let lists = bands.into_iter().map(|b| self.band(arr, b, key, ykey, outer)).collect();
            return Item::List(Dir::Column, lists);
        }
        self.band(arr, bands.pop().unwrap_or_default(), key, ykey, outer)
    }

    fn is_port(&self, item: &Item) -> bool {
        matches!(item, Item::Leaf(k) if self.s.objects[*k].role == Role::Port)
    }

    fn band(
        &self,
        arr: &Arrangement,
        items: Vec<Item>,
        key: &dyn Fn(&Item) -> Q,
        ykey: &dyn Fn(&Item) -> Q,
        outer: Outer,
    ) -> Item {
        if items.len() == 1 {
            return items.into_iter().next().expect("one item");
        }
        let spans: Vec<(i64, i64)> = items.iter().map(|it| self.span(arr, it)).collect();
        let (top, bottom) =
            (spans.iter().map(|s| s.0).min().unwrap_or(0), spans.iter().map(|s| s.1).max().unwrap_or(0));
        let stack = |mut ps: Vec<Item>| -> Item {
            if ps.len() == 1 {
                return ps.pop().expect("one port");
            }
            ps.sort_by(|a, b| ykey(b).cmp(&ykey(a)).then_with(|| a.cmp(b)));
            Item::List(Dir::Column, ps)
        };
        if top == bottom && items.iter().all(|it| self.is_port(it)) {
            return stack(items);
        }
        if top == bottom && items.iter().any(|it| self.is_port(it)) {
            let before = self.left_of(&arr.flipped);
            let leaves = |it: &Item| self.item_leaves(arr, it);
            let ordered = crate::tree_hang::ordered_ports(&before, &items, &|it| self.is_port(it), &leaves);
            let (ports, rest): (Vec<Item>, Vec<Item>) =
                items.into_iter().partition(|it| self.is_port(it) && !ordered.contains(it));
            let rest = self.order_row(arr, rest, key);
            let apart = |a: &Item, b: &Item| {
                let (la, lb) = (leaves(a), leaves(b));
                la.iter().any(|&x| lb.iter().any(|&y| before.contains(&(x, y)) || before.contains(&(y, x))))
            };
            let rest = crate::tree_hang::stacked(rest, &|it| ordered.contains(it), &apart, &stack);
            let middle = rest.iter().map(key).sum::<Q>() / int(rest.len().max(1) as i64);
            let (left, right): (Vec<Item>, Vec<Item>) = match outer {
                Outer::Left => (ports, Vec::new()),
                Outer::Right => (Vec::new(), ports),
                Outer::Both => ports.into_iter().partition(|p| key(p) < middle),
            };
            let mut row = Vec::new();
            if !left.is_empty() {
                row.push(stack(left));
            }
            row.extend(rest);
            if !right.is_empty() {
                row.push(stack(right));
            }
            return if row.len() == 1 { row.pop().expect("one item") } else { Item::List(Dir::Row, row) };
        }
        let spanning: Vec<bool> = spans.iter().map(|&(t, b)| t == top && b == bottom).collect();
        let tagged: Vec<(Item, bool)> = items.into_iter().zip(spanning).collect();
        let order = self.order_row(arr, tagged.iter().map(|(it, _)| it.clone()).collect(), key);
        let order = self.keep_columns(arr, order);
        let sorted: Vec<(Item, bool)> = order
            .into_iter()
            .map(|it| {
                let span = tagged.iter().find(|(x, _)| *x == it).is_some_and(|(_, s)| *s);
                (it, span)
            })
            .collect();
        if top == bottom || sorted.iter().all(|(_, s)| !*s) || sorted.iter().all(|(_, s)| *s) {
            return Item::List(Dir::Row, sorted.into_iter().map(|(it, _)| it).collect());
        }
        let first = sorted.iter().position(|(_, s)| *s).unwrap_or(0);
        let last = sorted.iter().rposition(|(_, s)| *s).unwrap_or(0);
        let side = |i: usize| -> Outer {
            if i < first {
                if outer == Outer::Right {
                    Outer::Both
                } else {
                    Outer::Left
                }
            } else if i > last {
                if outer == Outer::Left {
                    Outer::Both
                } else {
                    Outer::Right
                }
            } else {
                Outer::Both
            }
        };
        let mut slabs: Vec<Item> = Vec::new();
        let mut run: Vec<Item> = Vec::new();
        let mut run_at = 0;
        for (i, (it, span)) in sorted.into_iter().enumerate() {
            if span {
                if !run.is_empty() {
                    slabs.push(self.cut(arr, std::mem::take(&mut run), key, ykey, side(run_at)));
                }
                slabs.push(it);
            } else {
                if run.is_empty() {
                    run_at = i;
                }
                run.push(it);
            }
        }
        if !run.is_empty() {
            slabs.push(self.cut(arr, run, key, ykey, side(run_at)));
        }
        if slabs.len() == 1 {
            return slabs.pop().expect("one slab");
        }
        Item::List(Dir::Row, slabs)
    }

    fn keep_columns(&self, arr: &Arrangement, order: Vec<Item>) -> Vec<Item> {
        if self.rules.same_x.is_empty() {
            return order;
        }
        let sets: Vec<BTreeSet<usize>> =
            order.iter().map(|it| self.item_leaves(arr, it).into_iter().collect()).collect();
        let n = order.len();
        let mut class: Vec<usize> = (0..n).collect();
        fn find(class: &[usize], mut i: usize) -> usize {
            while class[i] != i {
                i = class[i];
            }
            i
        }
        for &(a, b) in &self.rules.same_x {
            let (Some(i), Some(j)) =
                (sets.iter().position(|x| x.contains(&a)), sets.iter().position(|x| x.contains(&b)))
            else {
                continue;
            };
            let (ri, rj) = (find(&class, i), find(&class, j));
            if ri != rj {
                class[ri.max(rj)] = ri.min(rj);
            }
        }
        let roots: Vec<usize> = (0..n).map(|i| find(&class, i)).collect();
        let mut done = vec![false; n];
        let mut out_index = Vec::with_capacity(n);
        for i in 0..n {
            if done[i] {
                continue;
            }
            for j in i..n {
                if !done[j] && roots[j] == roots[i] {
                    done[j] = true;
                    out_index.push(j);
                }
            }
        }
        let mut slots: Vec<Option<Item>> = order.into_iter().map(Some).collect();
        out_index.into_iter().filter_map(|j| slots[j].take()).collect()
    }

    fn barycentre(&self, arr: &Arrangement, item: &Item, at: &[[Q; 2]], ori: &[Orient], axis: usize) -> Q {
        let s = self.s;
        let members = self.item_leaves(arr, item);
        let inside: BTreeSet<usize> = members.iter().copied().collect();
        let mut sum = Q::zero();
        let mut count = 0i64;
        for &m in &members {
            for pin in 0..s.pins(m).len() {
                for &t in s.connections.targets.get(&(m, pin)).map_or(&[][..], |v| v) {
                    if inside.contains(&t.0) || !self.node_of.contains_key(&t.0) {
                        continue;
                    }
                    if let Some(q) = s.pin_at(at, ori, t.0, t.1) {
                        sum += &q[axis];
                        count += 1;
                    }
                }
            }
        }
        if count > 0 {
            sum / int(count)
        } else {
            members.iter().map(|&m| at[m][axis].clone()).sum::<Q>() / int(members.len().max(1) as i64)
        }
    }

    fn build(
        &self,
        arr: &mut Arrangement,
        key: &dyn Fn(&Arrangement, &Item) -> Q,
        ykey: &dyn Fn(&Arrangement, &Item) -> Q,
    ) {
        for node in self.groups.post_order() {
            let items = self.node_items(arr, node);
            let list = if items.is_empty() {
                None
            } else {
                let snapshot = arr.clone();
                let k = |it: &Item| key(&snapshot, it);
                let y = |it: &Item| ykey(&snapshot, it);
                Some(self.cut(&snapshot, items, &k, &y, Outer::Both))
            };
            arr.lists[node] = list;
        }
    }

    fn orient(&self, arr: &mut Arrangement) {
        let s = self.s;
        let ori = orientations(s, &arr.values, &arr.ways);
        let at = self.sketch(arr, &ori);
        let walk = self.walk(arr);
        let rows: Vec<Vec<usize>> = walk
            .lists
            .iter()
            .filter(|(d, _)| *d == Dir::Row)
            .map(|(_, items)| items.iter().filter(|l| l.len() == 1).map(|l| l[0]).collect())
            .collect();
        let mut ori = ori;
        for (c, class) in s.open.classes.iter().enumerate() {
            let ports = class.members.iter().all(|(m, _)| s.objects[*m].role == Role::Port);
            if ports {
                let Some(&(m, label)) = class.members.first() else { continue };
                let mean = self.barycentre(arr, &Item::Leaf(m), &at, &ori, 0);
                let want = if mean >= at[m][0] { (1, 0) } else { (-1, 0) };
                let mut all = Orient::ALL;
                all.sort_by_key(|v| preference(*v));
                if let Some(v) = all.into_iter().find(|v| s.exit(m, 0, v.then(label)) == Some(want)) {
                    arr.values[c] = v;
                }
                continue;
            }
            let ranked = ranked_values(s, &at, &ori, &rows, class);
            if let Some(&best) = ranked.first() {
                arr.values[c] = best;
                for (m, label) in &class.members {
                    ori[*m] = best.then(*label);
                }
            }
        }
        for p in &self.pairs {
            let (Some(ca), Some(cb)) = (s.class_of(p.a), s.class_of(p.b)) else { continue };
            if ca == cb {
                continue;
            }
            let ori = orientations(s, &arr.values, &arr.ways);
            let label_b = s.open.classes[cb].members.iter().find(|(m, _)| *m == p.b).map_or(Orient::R0, |(_, l)| *l);
            arr.values[cb] = ori[p.a].then(Orient::MY).then(inverse(label_b));
        }
    }

    fn pointing(&self, arr: &Arrangement, dir: Dir, first: &Item, second: &Item, ori: &[Orient]) -> i64 {
        let s = self.s;
        let (la, lb): (BTreeSet<usize>, BTreeSet<usize>) =
            (self.item_leaves(arr, first).into_iter().collect(), self.item_leaves(arr, second).into_iter().collect());
        let towards = if dir == Dir::Row { (1, 0) } else { (0, -1) };
        let away = (-towards.0, -towards.1);
        let rigid = |k: usize, pin: usize| -> bool {
            let Some(c) = s.class_of(k) else { return true };
            let class = &s.open.classes[c];
            let label = class.members.iter().find(|(m, _)| *m == k).map_or(Orient::R0, |(_, l)| *l);
            let ways: BTreeSet<Option<(i64, i64)>> =
                crate::search::domain(s, class).into_iter().map(|v| s.exit(k, pin, v.then(label))).collect();
            ways.len() <= 1
        };
        let mut vote = 0i64;
        for &a in &la {
            for pin in 0..s.pins(a).len() {
                for &t in s.connections.targets.get(&(a, pin)).map_or(&[][..], |v| v) {
                    if !lb.contains(&t.0) || !s.connections.routed((a, pin), t) {
                        continue;
                    }
                    for (k, q, out) in [(a, pin, away), (t.0, t.1, towards)] {
                        let w = if rigid(k, q) { 2 } else { 1 };
                        match s.exit(k, q, ori[k]) {
                            Some(e) if e == out => vote += w,
                            Some(e) if e == (-out.0, -out.1) => vote -= w,
                            _ => {}
                        }
                    }
                }
            }
        }
        vote
    }

    fn face(&self, arr: &mut Arrangement) -> bool {
        let ori = orientations(self.s, &arr.values, &arr.ways);
        let mut changed = false;
        for node in 0..arr.lists.len() {
            let Some(item) = arr.lists[node].clone() else { continue };
            let mut paths = Vec::new();
            list_paths(&item, &mut Vec::new(), &mut paths);
            for path in paths {
                let len = match arr.lists[node].as_ref().and_then(|it| list_ref(it, &path)) {
                    Some((_, items)) => items.len(),
                    None => continue,
                };
                for i in 0..len.saturating_sub(1) {
                    let Some((dir, items)) = arr.lists[node].as_ref().and_then(|it| list_ref(it, &path)) else { break };
                    if self.pointing(arr, dir, &items[i], &items[i + 1], &ori) <= 0 {
                        continue;
                    }
                    let mut trial = arr.clone();
                    if let Some(items) = trial.lists[node].as_mut().and_then(|it| list_mut(it, &path)) {
                        items.swap(i, i + 1);
                    }
                    if self.broken(&trial) <= self.broken(arr) {
                        *arr = trial;
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    pub(crate) fn first(&self, work: &mut Work) -> Arrangement {
        let s = self.s;
        let _ = work;
        let flipped: BTreeSet<usize> = (0..s.reversible.len())
            .filter(|&r| {
                let pairs = &s.reversible[r].pairs;
                self.pairs.iter().any(|p| pairs.contains(&(p.b, p.a)))
                    && !self.pairs.iter().any(|p| pairs.contains(&(p.a, p.b)))
            })
            .collect();
        let mut arr = Arrangement {
            lists: vec![None; self.groups.nodes.len()],
            values: vec![Orient::R0; s.open.classes.len()],
            ways: s.open.ways.iter().map(|w| w.taken).collect(),
            flipped,
            bound: Vec::new(),
        };
        let netlist = |a: &Arrangement, it: &Item| int(self.item_leaves(a, it).into_iter().min().unwrap_or(0) as i64);
        let zero = |_: &Arrangement, _: &Item| Q::zero();
        if crate::budget::stopped() {
            arr.lists[0] = Some(Item::List(Dir::Row, self.leaves.iter().map(|k| Item::Leaf(*k)).collect()));
            arr.values = s.open.classes.iter().map(|c| s.witness.orient[c.root]).collect();
            return arr;
        }
        self.build(&mut arr, &netlist, &zero);
        self.orient(&mut arr);
        let mut seen: Vec<(Vec<Option<Item>>, BTreeSet<usize>)> = vec![(arr.lists.clone(), arr.flipped.clone())];
        for _ in 0..8 {
            if !crate::budget::permit(crate::budget::Step::Candidate, s.elements.len()) {
                break;
            }
            let ori = orientations(s, &arr.values, &arr.ways);
            let at = self.sketch(&arr, &ori);
            let x = |a: &Arrangement, it: &Item| self.barycentre(a, it, &at, &ori, 0);
            let y = |a: &Arrangement, it: &Item| self.barycentre(a, it, &at, &ori, 1);
            self.turn_open_facts(&mut arr, &x);
            self.build(&mut arr, &x, &y);
            self.orient(&mut arr);
            if self.face(&mut arr) {
                self.orient(&mut arr);
            }
            let now = (arr.lists.clone(), arr.flipped.clone());
            if seen.contains(&now) {
                break;
            }
            seen.push(now);
        }
        arr
    }

    fn turn_open_facts(&self, arr: &mut Arrangement, key: &dyn Fn(&Arrangement, &Item) -> Q) {
        let s = self.s;
        let walk = self.walk(arr);
        for (r, rev) in s.reversible.iter().enumerate() {
            if self.pairs.iter().any(|p| rev.pairs.contains(&(p.a, p.b)) || rev.pairs.contains(&(p.b, p.a))) {
                continue;
            }
            let flip = arr.flipped.contains(&r);
            let mut vote = 0i64;
            for &(a, b) in &rev.pairs {
                let (a, b) = if flip { (b, a) } else { (a, b) };
                if !matches!(lca(&walk, a, b), Some((_, Dir::Row, ..))) {
                    continue;
                }
                let d = key(arr, &Item::Leaf(b)) - key(arr, &Item::Leaf(a));
                vote += if d.is_positive() {
                    1
                } else if d.is_negative() {
                    -1
                } else {
                    0
                };
            }
            if vote < 0 {
                if flip {
                    arr.flipped.remove(&r);
                } else {
                    arr.flipped.insert(r);
                }
            }
        }
    }

    fn settle_flips(&self, arr: &mut Arrangement) {
        let walk = self.walk(arr);
        for (r, rev) in self.s.reversible.iter().enumerate() {
            let (mut forward, mut backward) = (0, 0);
            for &(a, b) in &rev.pairs {
                if let Some((_, Dir::Row, ia, ib)) = lca(&walk, a, b) {
                    if ia > ib {
                        forward += 1;
                    } else {
                        backward += 1;
                    }
                }
            }
            let flip = arr.flipped.contains(&r);
            let (now, other) = if flip { (backward, forward) } else { (forward, backward) };
            if other < now {
                if flip {
                    arr.flipped.remove(&r);
                } else {
                    arr.flipped.insert(r);
                }
            }
        }
    }

    fn subtree(&self, node: usize) -> Vec<usize> {
        let mut out = vec![node];
        let mut i = 0;
        while i < out.len() {
            out.extend(self.groups.nodes[out[i]].children.iter().copied());
            i += 1;
        }
        out
    }

    fn neighbours(&self, arr: &Arrangement) -> Vec<Move> {
        let s = self.s;
        let mut out: Vec<Move> = Vec::new();
        let other = |d: Dir| if d == Dir::Row { Dir::Column } else { Dir::Row };
        for node in 0..arr.lists.len() {
            if !crate::budget::permit(crate::budget::Step::Candidate, self.leaves.len()) {
                return out;
            }
            let Some(root) = arr.lists[node].clone() else { continue };
            let name = if node == 0 { "root".to_string() } else { self.node_name(node) };
            let mut paths = Vec::new();
            list_paths(&root, &mut Vec::new(), &mut paths);
            let with = |f: &dyn Fn(&mut Item) -> bool| -> Option<Arrangement> {
                let mut a = arr.clone();
                let item = a.lists[node].as_mut()?;
                f(item).then_some(a)
            };
            for path in &paths {
                let Some((dir, items)) = list_ref(&root, path) else { continue };
                let n = items.len();
                for i in 0..n {
                    let moved = self.item_leaves(arr, &items[i]);
                    if i + 1 < n {
                        let mut both = moved.clone();
                        both.extend(self.item_leaves(arr, &items[i + 1]));
                        if let Some(a) = with(&|it| list_mut(it, path).map(|l| l.swap(i, i + 1)).is_some()) {
                            out.push((
                                format!(
                                    "{name}: swap {} and {}",
                                    self.item_text(&items[i]),
                                    self.item_text(&items[i + 1])
                                ),
                                a,
                                both.clone(),
                                false,
                            ));
                        }
                        {
                            let wrapped = Item::List(other(dir), vec![items[i].clone(), items[i + 1].clone()]);
                            if let Some(a) = with(&|it| {
                                list_mut(it, path)
                                    .map(|l| {
                                        l.splice(i..i + 2, [wrapped.clone()]);
                                    })
                                    .is_some()
                            }) {
                                out.push((format!("{name}: wrap {}", self.item_text(&wrapped)), a, both, false));
                            }
                        }
                    }
                    if let Item::List(_, inner) = &items[i] {
                        let inner = inner.clone();
                        if let Some(a) = with(&|it| {
                            list_mut(it, path)
                                .map(|l| {
                                    l.splice(i..i + 1, inner.clone());
                                })
                                .is_some()
                        }) {
                            out.push((
                                format!("{name}: unwrap {}", self.item_text(&items[i])),
                                a,
                                moved.clone(),
                                false,
                            ));
                        }
                    }
                    for to in &paths {
                        if to == path || (to.len() > path.len() && to[..path.len()] == path[..] && to[path.len()] == i)
                        {
                            continue;
                        }
                        let Some((_, target)) = list_ref(&root, to) else { continue };
                        for at in 0..=target.len() {
                            if !crate::budget::permit(crate::budget::Step::Candidate, self.leaves.len()) {
                                return out;
                            }
                            let item = items[i].clone();
                            let Some(a) = with(&|it| move_item(it, path, i, to, at, item.clone())) else { continue };
                            out.push((
                                format!(
                                    "{name}: move {} to place {at} of {}",
                                    self.item_text(&items[i]),
                                    self.item_text(&Item::List(
                                        list_ref(&root, to).map_or(Dir::Row, |x| x.0),
                                        target.clone()
                                    ))
                                ),
                                a,
                                moved.clone(),
                                false,
                            ));
                        }
                    }
                    if !path.is_empty() && n > 1 {
                        let (up, at) = path.split_at(path.len() - 1);
                        let at = at[0];
                        for side in [at, at + 1] {
                            let item = items[i].clone();
                            let Some(a) = with(&|it| {
                                let Some(l) = list_mut(it, path) else { return false };
                                l.remove(i);
                                let Some(parent) = list_mut(it, up) else { return false };
                                parent.insert(side, item.clone());
                                tidy(it);
                                true
                            }) else {
                                continue;
                            };
                            out.push((
                                format!("{name}: {} out of its list", self.item_text(&items[i])),
                                a,
                                moved.clone(),
                                false,
                            ));
                        }
                    }
                }
            }
            let leaves: BTreeSet<usize> = self.item_leaves(arr, &Item::Group(node)).into_iter().collect();
            let mut a = arr.clone();
            for m in self.subtree(node) {
                if let Some(it) = a.lists[m].as_mut() {
                    mirror(it);
                }
            }
            for (c, class) in s.open.classes.iter().enumerate() {
                if class.members.iter().all(|(m, _)| leaves.contains(m)) {
                    a.values[c] = a.values[c].then(Orient::MY);
                }
            }
            out.push((format!("{name}: mirrored"), a, leaves.into_iter().collect(), true));
        }
        for (c, class) in s.open.classes.iter().enumerate() {
            let members: Vec<usize> = class.members.iter().map(|(m, _)| *m).collect();
            let values: Vec<Orient> = if members.iter().all(|m| s.objects[*m].role == Role::Port) {
                let mut all = Orient::ALL.to_vec();
                all.sort_by_key(|v| preference(*v));
                let label = class.members[0].1;
                let mut ways: Vec<Option<(i64, i64)>> = Vec::new();
                all.into_iter()
                    .filter(|v| {
                        let e = s.exit(members[0], 0, v.then(label));
                        let new = matches!(e, Some((_, 0))) && !ways.contains(&e);
                        if new {
                            ways.push(e);
                        }
                        new
                    })
                    .collect()
            } else {
                crate::tree_hang::current_order(s, class).into_iter().map(|(v, _)| v).collect()
            };
            for v in values {
                if !crate::budget::permit(crate::budget::Step::Candidate, self.leaves.len()) {
                    return out;
                }
                if v == arr.values[c] {
                    continue;
                }
                let mut a = arr.clone();
                a.values[c] = v;
                out.push((format!("{} turned {v:?}", self.leaf_name(members[0])), a, members.clone(), true));
            }
        }
        for (i, w) in s.open.ways.iter().enumerate() {
            for v in 0..w.count {
                if v == arr.ways[i] {
                    continue;
                }
                let mut a = arr.clone();
                a.ways[i] = v;
                out.push((format!("the fact [{}] its way {v}", w.fact), a, Vec::new(), false));
            }
        }
        out
    }

    fn straight_pairs(&self, arr: &Arrangement, eval: &Evaluated, ori: &[Orient], released: &[Tie]) -> Vec<Tie> {
        let s = self.s;
        let mut candidates: Vec<Tie> = self.ties(arr, ori).layers.into_iter().next().unwrap_or_default();
        candidates.extend(self.bent_ties(&eval.placed, &eval.routing));
        let at = &eval.placed.origins;
        let gori = &eval.placed.geometry.orient;
        let mut out: Vec<Tie> = Vec::new();
        for t in candidates {
            let Some((pa, pb)) = t.pins else { continue };
            if t.weight < 4 || out.contains(&t) || released.contains(&t) || arr.bound.iter().any(|b| b.tie == t) {
                continue;
            }
            let (Some(p), Some(q)) = (s.pin_at(at, gori, t.a, pa), s.pin_at(at, gori, t.b, pb)) else { continue };
            if p[t.axis] != q[t.axis] {
                continue;
            }
            let straight = eval.routing.paths.iter().any(|path| {
                let ends = (path.first(), path.last());
                let touches = matches!(ends, (Some(x), _) if *x == p || *x == q)
                    || matches!(ends, (_, Some(y)) if *y == p || *y == q);
                touches && crate::objectives::bends(path) == 0
            });
            if straight {
                out.push(t);
            }
        }
        out
    }

    fn bind(&self, arr: &Arrangement, eval: &Evaluated, released: &[Tie]) -> Option<(Arrangement, Vec<String>)> {
        let ori = eval.placed.geometry.orient.clone();
        let pairs = self.straight_pairs(arr, eval, &ori, released);
        if pairs.is_empty() {
            return None;
        }
        let mut a = arr.clone();
        let mut said = Vec::new();
        for t in pairs {
            let dir = if t.axis == 1 { Dir::Row } else { Dir::Column };
            let mut together = None;
            for node in 0..a.lists.len() {
                let Some(root) = a.lists[node].clone() else { continue };
                let mut paths = Vec::new();
                list_paths(&root, &mut Vec::new(), &mut paths);
                for path in paths {
                    let Some((d, items)) = list_ref(&root, &path) else { continue };
                    if d != dir {
                        continue;
                    }
                    let at = |k: usize| items.iter().position(|it| *it == Item::Leaf(k));
                    let (Some(i), Some(j)) = (at(t.a), at(t.b)) else { continue };
                    if i.abs_diff(j) != 1 {
                        continue;
                    }
                    let lo = i.min(j);
                    if items.len() > 2 {
                        if let Some(l) = a.lists[node].as_mut().and_then(|it| list_mut(it, &path)) {
                            let pair = vec![l[lo].clone(), l[lo + 1].clone()];
                            l.splice(lo..lo + 2, [Item::List(dir, pair)]);
                        }
                    }
                    together = Some((t.a, t.b, dir));
                }
            }
            said.push(format!(
                "{} and {}{}",
                self.leaf_name(t.a),
                self.leaf_name(t.b),
                if together.is_some() { ", in a list of their own" } else { "" }
            ));
            a.bound.push(Bound { tie: t, together });
        }
        Some((a, said))
    }

    fn pointing_away(&self, eval: &Evaluated) -> BTreeSet<usize> {
        let s = self.s;
        let ori = &eval.placed.geometry.orient;
        let mut out = BTreeSet::new();
        for (wire, path) in eval.routing.wires.iter().zip(&eval.routing.paths) {
            if crate::objectives::bends(path) == 0 {
                continue;
            }
            let pins: Vec<PinKey> = [wire.points.first(), wire.points.last()]
                .into_iter()
                .filter_map(|w| match w {
                    Some(rnl_runtime::solution::WirePoint::Anchor(
                        PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. },
                    )) => Some((*object, *pin)),
                    _ => None,
                })
                .filter(|p| self.node_of.contains_key(&p.0))
                .collect();
            let (Some(a), Some(b)) = (path.first(), path.last()) else { continue };
            for &p in &pins {
                let Some(e) = s.exit(p.0, p.1, ori[p.0]) else { continue };
                let Some(at) = s.pin_at(&eval.placed.origins, ori, p.0, p.1) else { continue };
                let other = if at == *a { b } else { a };
                let d = [&other[0] - &at[0], &other[1] - &at[1]];
                if (int(e.0) * &d[0] + int(e.1) * &d[1]).is_negative() {
                    out.insert(p.0);
                }
            }
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn search(
        &self,
        first: Arrangement,
        evaluated: Evaluated,
        rounds: usize,
        bindings: bool,
        work: &mut Work,
        stop: &Stop,
    ) -> (Arrangement, Evaluated, Vec<String>, Vec<Arrangement>) {
        let s = self.s;
        let better = |a: &Entry, b: &Entry| crate::score::order(self.active(), &[a.clone(), b.clone()])[0] == 1;
        let mut path: Vec<Arrangement> = Vec::new();
        let ranks = crate::tree_hang::current_ranks(s);
        let (mut current, mut eval) = (first, evaluated);
        let (mut best, mut best_eval) = (current.clone(), eval.clone());
        let mut seen: BTreeSet<Vec<u32>> = BTreeSet::new();
        seen.insert(self.code(&current));
        let mut tried = 0u64;
        let mut steps: Vec<String> = Vec::new();
        let mut bound_said: Vec<String> = Vec::new();
        let mut released: Vec<Tie> = Vec::new();
        let mut releases = 0;
        let mut finished = false;
        'search: loop {
            if stop.reached(work) {
                break;
            }
            if bindings {
                if let Some((a, said)) = self.bind(&current, &eval, &released) {
                    if crate::annotation::estimated_work(self.problem) > 0
                        && !crate::budget::permit(
                            crate::budget::Step::Placement,
                            crate::annotation::estimated_work(self.problem),
                        )
                    {
                        break 'search;
                    }
                    tried += 1;
                    let (next, _) = self.evaluate_rounds(&a, rounds, work);
                    if crate::budget::stopped() {
                        break 'search;
                    }
                    if !better(&eval.entry, &next.entry) {
                        bound_said.extend(said);
                        seen.insert(self.code(&a));
                        current = a;
                        eval = next;
                        if better(&best_eval.entry, &eval.entry) || self.code(&best) == self.code(&current) {
                            best = current.clone();
                            best_eval = eval.clone();
                        }
                    }
                }
            }
            let broken = self.broken(&current);
            let trouble = match drawn(self.problem, s, &eval.placed.geometry, &eval.routing) {
                Ok(wires) => crate::score::trouble(s, &eval.placed.origins, &eval.placed.geometry.orient, &wires),
                Err(_) => vec![0; s.objects.len()],
            };
            let away = if bindings { self.pointing_away(&eval) } else { BTreeSet::new() };
            let mut moves = self.neighbours(&current);
            for m in &mut moves {
                self.settle_flips(&mut m.1);
            }
            let rank =
                |leaves: &[usize]| leaves.iter().map(|&k| trouble.get(k).copied().unwrap_or(0)).max().unwrap_or(0);
            let targeted = |m: &Move| {
                let turn = m.3;
                turn && m.2.iter().any(|k| away.contains(k))
            };
            let current_rank = |m: &Move| crate::tree_hang::move_rank(&ranks, &current, &m.1);
            let mut order: Vec<usize> = (0..moves.len()).collect();
            order.sort_by_cached_key(|&i| {
                (current_rank(&moves[i]), !targeted(&moves[i]), std::cmp::Reverse(rank(&moves[i].2)), i)
            });
            let mut stepped = false;
            for i in order {
                if stop.reached(work) {
                    break 'search;
                }
                let (what, arr, _, _) = &moves[i];
                if !seen.insert(self.code(arr)) || self.broken(arr) > broken {
                    continue;
                }
                if crate::annotation::estimated_work(self.problem) > 0
                    && !crate::budget::permit(
                        crate::budget::Step::Placement,
                        crate::annotation::estimated_work(self.problem),
                    )
                {
                    break 'search;
                }
                tried += 1;
                stop.placed.set(stop.placed.get() + 1);
                stop.routed.set(stop.routed.get() + 1);
                let next = self.screened(arr, &eval.entry, rounds, work);
                if crate::budget::stopped() {
                    break 'search;
                }
                if better(&eval.entry, &next.entry) {
                    steps.push(what.clone());
                    current = arr.clone();
                    path.push(current.clone());
                    eval = next;
                    stepped = true;
                    if better(&best_eval.entry, &eval.entry) {
                        best = current.clone();
                        best_eval = eval.clone();
                        stop.best_at.set(Some(stop.placed.get()));
                    }
                    break;
                }
            }
            if stepped {
                continue;
            }
            if bindings && releases < RELEASES {
                let mut unbound = current.clone();
                if let Some(b) = unbound.bound.pop() {
                    if crate::annotation::estimated_work(self.problem) > 0
                        && !crate::budget::permit(
                            crate::budget::Step::Placement,
                            crate::annotation::estimated_work(self.problem),
                        )
                    {
                        break 'search;
                    }
                    let (next, _) = self.evaluate_rounds(&unbound, rounds, work);
                    if crate::budget::stopped() {
                        break 'search;
                    }
                    releases += 1;
                    released.push(b.tie.clone());
                    current = unbound;
                    eval = next;
                    seen.insert(self.code(&current));
                    continue;
                }
            }
            finished = true;
            break;
        }
        let mut notes = vec![format!(
            "tree: the search tried {tried} arrangements and took {} steps{}",
            steps.len(),
            if finished { ", none of the next ones better" } else { ", stopped by its limit" }
        )];
        if !steps.is_empty() {
            notes.push(format!("tree: the steps taken: {}", steps.join("; ")));
        }
        if bindings {
            notes.push(format!(
                "tree: {} bound, {releases} released{}{}",
                count(bound_said.len(), "pair"),
                if bound_said.is_empty() { "" } else { ": " },
                bound_said.join("; ")
            ));
        }
        notes.push(format!("tree: the arrangement taken: {}", self.describe(&best)));
        (best, best_eval, notes, path)
    }

    fn relations(&self, arr: &Arrangement, ori: &[Orient]) -> Vec<Tagged> {
        if !crate::budget::permit(crate::budget::Step::Structure, self.leaves.len()) {
            return Vec::new();
        }
        let s = self.s;
        let grid = grid_of(self.options);
        let attach = self.attach(ori);
        let eff = self.boxes(ori);
        let o = |k: usize| s.model.origin_point.get(k).copied().flatten();
        let one = Q::one;
        let mut out = Vec::new();
        for (&k, a) in &attach {
            let (Some(p), Some(h)) = (o(k), o(a.host.0)) else { continue };
            let Some(at) = attach_offset(s, k, a, ori) else { continue };
            let terms = || vec![(p, one()), (h, -one())];
            let (exact_axis, gap_axis) = match a.side {
                Side::Left | Side::Right => (1, 0),
                Side::Below | Side::Above => (0, 1),
            };
            out.push(Tagged {
                kind: RelKind::Attachment,
                rel: Relation::Linear {
                    axis: exact_axis,
                    terms: terms(),
                    lo: Some(at[exact_axis].clone()),
                    hi: Some(at[exact_axis].clone()),
                },
            });
            let rel = match a.side {
                Side::Left | Side::Below => {
                    let hi = -up(-at[gap_axis].clone(), &grid);
                    Relation::Linear { axis: gap_axis, terms: terms(), lo: None, hi: Some(hi) }
                }
                Side::Right | Side::Above => {
                    let lo = up(at[gap_axis].clone(), &grid);
                    Relation::Linear { axis: gap_axis, terms: terms(), lo: Some(lo), hi: None }
                }
            };
            out.push(Tagged { kind: RelKind::Attachment, rel });
        }
        for b in &arr.bound {
            let t = &b.tie;
            let (Some(pa), Some(pb)) = (o(t.a), o(t.b)) else { continue };
            let c = &t.cb - &t.ca;
            out.push(Tagged {
                kind: if t.axis == 1 { RelKind::Row } else { RelKind::Column },
                rel: Relation::Linear {
                    axis: t.axis,
                    terms: vec![(pa, one()), (pb, -one())],
                    lo: Some(c.clone()),
                    hi: Some(c),
                },
            });
        }
        let walk = self.walk(arr);
        for (dir, items) in &walk.lists {
            for w in items.windows(2) {
                if !crate::budget::permit(crate::budget::Step::Structure, w[0].len().saturating_add(w[1].len())) {
                    return out;
                }
                let (first, second) = (&w[0], &w[1]);
                let (low, high, axis, gap): (Vec<Term>, Vec<Term>, usize, i64) = match dir {
                    Dir::Row => (
                        first.iter().filter_map(|&k| Some((o(k)?, up(eff[&k][2].clone(), &grid)))).collect(),
                        second.iter().filter_map(|&k| Some((o(k)?, -up(-eff[&k][0].clone(), &grid)))).collect(),
                        0,
                        HGAP,
                    ),
                    Dir::Column => (
                        second.iter().filter_map(|&k| Some((o(k)?, up(eff[&k][3].clone(), &grid)))).collect(),
                        first.iter().filter_map(|&k| Some((o(k)?, -up(-eff[&k][1].clone(), &grid)))).collect(),
                        1,
                        VGAP,
                    ),
                };
                if !low.is_empty() && !high.is_empty() {
                    out.push(Tagged { kind: RelKind::Gap, rel: Relation::Apart { axis, low, high, gap: int(gap) } });
                }
            }
        }
        out
    }

    fn ties(&self, arr: &Arrangement, ori: &[Orient]) -> Pull {
        let empty = || Pull { layers: vec![Vec::new(), Vec::new()] };
        if !crate::budget::permit(crate::budget::Step::Structure, self.leaves.len()) {
            return empty();
        }
        let s = self.s;
        let walk = self.walk(arr);
        let mut straight: Vec<Tie> = Vec::new();
        let pin = |p: PinKey, axis: usize| s.pin_offset(p.0, p.1, ori[p.0]).map(|o| o[axis].clone());
        let exit = |p: PinKey| s.exit(p.0, p.1, ori[p.0]);
        let clearance = int(CLEARANCE);
        for d in s.connections.drawn.iter().filter(|d| d.routed) {
            let pins: Vec<PinKey> = d
                .points
                .iter()
                .filter_map(|p| match p {
                    PointId::Pin(k, pin) if self.node_of.contains_key(k) => Some((*k, *pin)),
                    _ => None,
                })
                .collect();
            for (i, &p) in pins.iter().enumerate() {
                for &q in &pins[i + 1..] {
                    if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                        return Pull { layers: vec![straight, Vec::new()] };
                    }
                    if p.0 == q.0 {
                        continue;
                    }
                    let Some((depth, dir, ip, iq)) = lca(&walk, p.0, q.0) else { continue };
                    if ip.abs_diff(iq) != 1 {
                        continue;
                    }
                    let (f, g) = if ip < iq { (p, q) } else { (q, p) };
                    let (ef, eg) = (exit(f), exit(g));
                    let open = |k: usize, side: Open| exposed(&walk.chains[&k], depth + 1, side);
                    let (Some(ef), Some(eg)) = (ef, eg) else { continue };
                    let tie = |axis: usize, cf: Q, cg: Q, weight: i64| Tie {
                        axis,
                        a: f.0,
                        ca: cf,
                        b: g.0,
                        cb: cg,
                        weight,
                        pins: Some((f.1, g.1)),
                    };
                    let (Some(fx), Some(fy), Some(gx), Some(gy)) = (pin(f, 0), pin(f, 1), pin(g, 0), pin(g, 1)) else {
                        continue;
                    };
                    match dir {
                        Dir::Row => {
                            if ef == (1, 0) && eg == (-1, 0) && open(f.0, Open::Right) && open(g.0, Open::Left) {
                                straight.push(tie(1, fy, gy, 4));
                            } else if ef.0 == 0 && ef == eg {
                                straight.push(tie(1, fy, gy, 2));
                            } else if ef == (1, 0) && eg.0 == 0 && open(f.0, Open::Right) {
                                straight.push(tie(1, fy, gy + &clearance * int(eg.1), 1));
                            } else if eg == (-1, 0) && ef.0 == 0 && open(g.0, Open::Left) {
                                straight.push(tie(1, fy + &clearance * int(ef.1), gy, 1));
                            }
                        }
                        Dir::Column => {
                            if ef == (0, -1) && eg == (0, 1) && open(f.0, Open::Down) && open(g.0, Open::Up) {
                                straight.push(tie(0, fx, gx, 4));
                            } else if ef.1 == 0 && ef == eg {
                                straight.push(tie(0, fx, gx, 2));
                            } else if ef == (0, -1) && eg.1 == 0 && open(f.0, Open::Down) {
                                straight.push(tie(0, fx, gx + &clearance * int(eg.0), 1));
                            } else if eg == (0, 1) && ef.1 == 0 && open(g.0, Open::Up) {
                                straight.push(tie(0, fx + &clearance * int(ef.0), gx, 1));
                            }
                        }
                    }
                }
            }
        }
        let middle = |k: usize, axis: usize| -> Q {
            s.box_offset(k, ori[k]).map_or_else(Q::zero, |b| (&b[axis] + &b[axis + 2]) / int(2))
        };
        let nets = |k: usize| -> BTreeSet<usize> {
            let mut out = BTreeSet::new();
            for pin in 0..s.pins(k).len() {
                for &i in s.connections.nets_of.get(&(k, pin)).map_or(&[][..], |v| v) {
                    let net = &s.connections.nets[i];
                    if !(net.supply || net.ground) {
                        out.insert(i);
                    }
                }
            }
            out
        };
        let mut align: Vec<Tie> = Vec::new();
        for (dir, items) in &walk.lists {
            let axis = if *dir == Dir::Row { 1 } else { 0 };
            for w in items.windows(2) {
                if let ([a], [b]) = (w[0].as_slice(), w[1].as_slice()) {
                    align.push(Tie {
                        axis,
                        a: *a,
                        ca: middle(*a, axis),
                        b: *b,
                        cb: middle(*b, axis),
                        weight: 1,
                        pins: None,
                    });
                    continue;
                }
                let (to_next, to_previous) = match dir {
                    Dir::Row => (Open::Right, Open::Left),
                    Dir::Column => (Open::Down, Open::Up),
                };
                for &a in &w[0] {
                    for &b in &w[1] {
                        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                            return Pull { layers: vec![straight, align] };
                        }
                        let Some((depth, ..)) = lca(&walk, a, b) else { continue };
                        let open = exposed(&walk.chains[&a], depth + 1, to_next)
                            && exposed(&walk.chains[&b], depth + 1, to_previous);
                        if open && !nets(a).is_disjoint(&nets(b)) {
                            align.push(Tie {
                                axis,
                                a,
                                ca: middle(a, axis),
                                b,
                                cb: middle(b, axis),
                                weight: 1,
                                pins: None,
                            });
                        }
                    }
                }
            }
        }
        Pull { layers: vec![straight, align] }
    }

    fn bent_ties(&self, placed: &Placed, routing: &Routing) -> Vec<Tie> {
        use rnl_runtime::solution::WirePoint;
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        enum Node {
            Pin(usize, usize),
            Branch(usize),
            Other,
        }
        let node = |w: Option<&WirePoint>| match w {
            Some(WirePoint::Anchor(PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. })) => {
                Node::Pin(*object, *pin)
            }
            Some(WirePoint::Branch(b)) => Node::Branch(*b),
            _ => Node::Other,
        };
        let mut adjacent: BTreeMap<Node, Vec<Node>> = BTreeMap::new();
        let ends: Vec<(Node, Node)> =
            routing.wires.iter().map(|w| (node(w.points.first()), node(w.points.last()))).collect();
        for &(a, b) in &ends {
            adjacent.entry(a).or_default().push(b);
            adjacent.entry(b).or_default().push(a);
        }
        let ori = &placed.geometry.orient;
        let mut out = Vec::new();
        for ((a, b), path) in ends.iter().zip(&routing.paths) {
            let bends = crate::objectives::bends(path);
            if bends == 0 {
                continue;
            }
            for (from, through) in [(*a, *b), (*b, *a)] {
                let Node::Pin(k, pin) = from else { continue };
                if !self.node_of.contains_key(&k) {
                    continue;
                }
                let mut seen: BTreeSet<Node> = [from, through].into_iter().collect();
                let mut layer = vec![through];
                let mut found: Vec<PinKey> = Vec::new();
                for _ in 0..4 {
                    for n in &layer {
                        if let Node::Pin(q, qp) = n {
                            if *q != k && self.node_of.contains_key(q) {
                                found.push((*q, *qp));
                            }
                        }
                    }
                    if !found.is_empty() {
                        break;
                    }
                    let mut next = Vec::new();
                    for n in &layer {
                        for m in adjacent.get(n).map_or(&[][..], |v| v) {
                            if seen.insert(*m) {
                                next.push(*m);
                            }
                        }
                    }
                    layer = next;
                }
                for q in found {
                    let pin_to_pin = matches!(through, Node::Pin(..));
                    if let Some(t) = self.tie_for((k, pin), q, placed, ori, pin_to_pin.then_some(bends)) {
                        out.push(t);
                    }
                }
            }
        }
        out
    }

    fn tie_for(&self, p: PinKey, q: PinKey, placed: &Placed, ori: &[Orient], bends: Option<usize>) -> Option<Tie> {
        let s = self.s;
        let (ep, eq) = (s.exit(p.0, p.1, ori[p.0])?, s.exit(q.0, q.1, ori[q.0])?);
        let (op, oq) = (s.pin_offset(p.0, p.1, ori[p.0])?, s.pin_offset(q.0, q.1, ori[q.0])?);
        let (pp, pq) = (s.pin_at(&placed.origins, ori, p.0, p.1)?, s.pin_at(&placed.origins, ori, q.0, q.1)?);
        let d = [&pq[0] - &pp[0], &pq[1] - &pp[1]];
        let back = [-d[0].clone(), -d[1].clone()];
        let towards = |e: (i64, i64), d: &[Q; 2]| (int(e.0) * &d[0] + int(e.1) * &d[1]).is_positive();
        let across = |e: (i64, i64)| if e.1 == 0 { 1 } else { 0 };
        if ep == (-eq.0, -eq.1) && towards(ep, &d) && towards(eq, &back) {
            let axis = across(ep);
            return Some(Tie {
                axis,
                a: p.0,
                ca: op[axis].clone(),
                b: q.0,
                cb: oq[axis].clone(),
                weight: 4,
                pins: Some((p.1, q.1)),
            });
        }
        if ep == eq {
            let axis = 1 - across(ep);
            return Some(Tie {
                axis,
                a: p.0,
                ca: op[axis].clone(),
                b: q.0,
                cb: oq[axis].clone(),
                weight: 2,
                pins: Some((p.1, q.1)),
            });
        }
        if ep.0 * eq.0 + ep.1 * eq.1 == 0 && bends.is_none_or(|b| b > 1) {
            let clearance = int(CLEARANCE);
            for ((x, ex, ox), (y, ey, oy), dd) in
                [((p, ep, &op), (q, eq, &oq), &d), ((q, eq, &oq), (p, ep, &op), &back)]
            {
                if !towards(ex, dd) {
                    continue;
                }
                let axis = across(ex);
                let lead = &clearance * int(if axis == 1 { ey.1 } else { ey.0 });
                return Some(Tie {
                    axis,
                    a: x.0,
                    ca: ox[axis].clone(),
                    b: y.0,
                    cb: &oy[axis] + lead,
                    weight: 1,
                    pins: Some((x.1, y.1)),
                });
            }
        }
        None
    }

    fn screened(&self, arr: &Arrangement, _current: &Entry, rounds: usize, work: &mut Work) -> Evaluated {
        self.evaluate_rounds(arr, rounds, work).0
    }

    pub(crate) fn evaluate_rounds(&self, arr: &Arrangement, rounds: usize, work: &mut Work) -> (Evaluated, usize) {
        let (problem, s) = (self.problem, self.s);
        let candidate = Candidate {
            values: arr.values.clone(),
            ways: arr.ways.clone(),
            rows: Vec::new(),
            apart: Vec::new(),
            even: false,
            columns: false,
            origin: "tree",
        };
        let orient = orientation_relations(s, &candidate);
        let actual: Vec<Orient> = with_relations(s, &orient, work, |r, _, _| r.geometry().orient.clone());
        let rels = self.relations(arr, &actual);
        let mut pull = self.ties(arr, &actual);
        let code = self.code(arr);
        let mut best = self.place_route_score(arr, &orient, &rels, &pull, &code, work);
        let mut taken = 0;
        for _ in 0..rounds {
            if !crate::budget::permit(
                crate::budget::Step::Placement,
                s.objects.len().saturating_add(crate::annotation::estimated_work(problem)),
            ) {
                break;
            }
            let mut new: Vec<Tie> = Vec::new();
            for t in self.bent_ties(&best.placed, &best.routing) {
                if !pull.layers[0].contains(&t) && !new.contains(&t) {
                    new.push(t);
                }
            }
            if new.is_empty() {
                break;
            }
            pull.layers[0].extend(new);
            let next = self.place_route_score(arr, &orient, &rels, &pull, &code, work);
            if crate::budget::stopped() {
                break;
            }
            let entries = [best.entry.clone(), next.entry.clone()];
            let better = crate::score::order(self.active(), &entries)[0] == 1;
            if better {
                best = next;
                taken += 1;
            } else {
                break;
            }
        }
        let _ = problem;
        (best, taken)
    }

    fn place_route_score(
        &self,
        arr: &Arrangement,
        orient: &[Tagged],
        rels: &[Tagged],
        pull: &Pull,
        code: &[u32],
        work: &mut Work,
    ) -> Evaluated {
        let (problem, s) = (self.problem, self.s);
        let placed = self.coordinates(arr, orient, rels, pull, work);
        let phase = crate::profile::span("tree", "route");
        let routing = route(problem, s, &placed.geometry, &placed.origins, self.options, work);
        drop(phase);
        let _phase = crate::profile::span("tree", "score");
        let objectives = match (drawn(problem, s, &placed.geometry, &routing), routing.annotation_error) {
            (Ok(wires), None) => measured_layers(problem, s, &placed, Some(&wires)),
            _ => Vec::new(),
        };
        let entry = self.entry(&placed, &routing, objectives, code);
        Evaluated { placed, routing, entry }
    }

    pub(crate) fn with_line_rank(mut self) -> Self {
        self.line_rank = Some(crate::lines::Rank::new(self.s));
        self
    }

    fn entry(&self, placed: &Placed, routing: &Routing, objectives: Vec<Q>, code: &[u32]) -> Entry {
        if let Some(rank) = &self.line_rank {
            if routing.annotation_error.is_some()
                || drawn(self.problem, self.s, &placed.geometry, routing).is_err()
                || crate::annotation::bounds(self.problem, &placed.geometry).is_err()
            {
                return self.objective_entry(placed, routing, objectives, code);
            }
            return rank.entry(self.problem, self.s, self.options, placed, routing, objectives, code);
        }
        self.objective_entry(placed, routing, objectives, code)
    }

    fn objective_entry(&self, placed: &Placed, routing: &Routing, objectives: Vec<Q>, code: &[u32]) -> Entry {
        if let Some(index) =
            routing.annotation_error.or_else(|| crate::annotation::bounds(self.problem, &placed.geometry).err())
        {
            return Entry {
                written_path_error: None,
                annotation_error: Some(index),
                overlaps: 0,
                crowding: 0,
                misleading: 0,
                joins_left: 0,
                layers: Vec::new(),
                code: code.to_vec(),
            };
        }
        let wires = match drawn(self.problem, self.s, &placed.geometry, routing) {
            Ok(wires) => wires,
            Err(wire) => {
                return Entry {
                    written_path_error: Some(wire),
                    annotation_error: None,
                    overlaps: 0,
                    crowding: 0,
                    misleading: 0,
                    joins_left: 0,
                    layers: Vec::new(),
                    code: code.to_vec(),
                }
            }
        };
        Entry {
            written_path_error: None,
            annotation_error: None,
            overlaps: placed.overlaps.len()
                + crate::annotation::overlaps(self.problem, self.s, &placed.geometry, &placed.origins)
                    .expect("validated annotation bounds"),
            crowding: crate::score::crowding(self.s, &placed.origins, &placed.geometry.orient, &placed.dropped),
            misleading: crate::score::misleading(self.problem, self.s, &placed.origins, &placed.geometry, &wires),
            joins_left: routing.joins_left(),
            layers: objectives,
            code: code.to_vec(),
        }
    }
    pub(crate) fn given(&self, mut ev: Evaluated) -> Evaluated {
        if self.line_rank.is_some() {
            let objectives = match (
                drawn(self.problem, self.s, &ev.placed.geometry, &ev.routing),
                crate::annotation::bounds(self.problem, &ev.placed.geometry),
                ev.routing.annotation_error,
            ) {
                (Ok(wires), Ok(_), None) => {
                    crate::score::measured_layers(self.problem, self.s, &ev.placed, Some(&wires))
                }
                _ => Vec::new(),
            };
            ev.entry = self.objective_entry(&ev.placed, &ev.routing, objectives, &ev.entry.code);
        }
        ev
    }
    fn active(&self) -> &crate::objectives::Active {
        self.line_rank.as_ref().map_or(&self.s.active, |r| r.active())
    }

    fn coordinates(
        &self,
        arr: &Arrangement,
        orient: &[Tagged],
        rels: &[Tagged],
        pull: &Pull,
        work: &mut Work,
    ) -> Placed {
        let _ = arr;
        let _phase = crate::profile::span("tree", "coordinates");
        place_tree(self.problem, self.s, orient, rels, pull, self.options, work)
    }
}

fn mirror(item: &mut Item) {
    if let Item::List(dir, items) = item {
        if *dir == Dir::Row {
            items.reverse();
        }
        for it in items.iter_mut() {
            mirror(it);
        }
    }
}

fn move_item(root: &mut Item, from: &[usize], i: usize, to: &[usize], at: usize, item: Item) -> bool {
    let Some(target) = list_mut(root, to) else { return false };
    if at > target.len() {
        return false;
    }
    target.insert(at, item);
    let mut from = from.to_vec();
    if to.len() < from.len() && from[..to.len()] == to[..] && at <= from[to.len()] {
        from[to.len()] += 1;
    }
    let Some(source) = list_mut(root, &from) else { return false };
    if i >= source.len() {
        return false;
    }
    source.remove(i);
    tidy(root);
    true
}

fn tidy(item: &mut Item) {
    if let Item::List(_, items) = item {
        for it in items.iter_mut() {
            tidy(it);
        }
        items.retain(|it| !matches!(it, Item::List(_, l) if l.is_empty()));
        for it in items.iter_mut() {
            if let Item::List(_, l) = it {
                if l.len() == 1 {
                    *it = l.pop().expect("one item");
                }
            }
        }
    }
}

fn list_paths(item: &Item, prefix: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if let Item::List(_, items) = item {
        out.push(prefix.clone());
        for (i, it) in items.iter().enumerate() {
            prefix.push(i);
            list_paths(it, prefix, out);
            prefix.pop();
        }
    }
}

fn list_ref<'b>(item: &'b Item, path: &[usize]) -> Option<(Dir, &'b Vec<Item>)> {
    match item {
        Item::List(dir, items) => match path.split_first() {
            None => Some((*dir, items)),
            Some((i, rest)) => list_ref(items.get(*i)?, rest),
        },
        _ => None,
    }
}

fn list_mut<'b>(item: &'b mut Item, path: &[usize]) -> Option<&'b mut Vec<Item>> {
    match item {
        Item::List(_, items) => match path.split_first() {
            None => Some(items),
            Some((i, rest)) => list_mut(items.get_mut(*i)?, rest),
        },
        _ => None,
    }
}

fn lca(walk: &Walk, a: usize, b: usize) -> Option<(usize, Dir, usize, usize)> {
    let (ca, cb) = (walk.chains.get(&a)?, walk.chains.get(&b)?);
    for (i, (x, y)) in ca.iter().zip(cb).enumerate() {
        if x.list != y.list {
            return None;
        }
        if x.index != y.index {
            return Some((i, x.dir, x.index, y.index));
        }
    }
    None
}

fn exposed(chain: &[Step], from: usize, side: Open) -> bool {
    chain.iter().skip(from).all(|st| match (st.dir, side) {
        (Dir::Row, Open::Left) => st.index == 0,
        (Dir::Row, Open::Right) => st.index + 1 == st.len,
        (Dir::Column, Open::Up) => st.index == 0,
        (Dir::Column, Open::Down) => st.index + 1 == st.len,
        _ => true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{fact, instance, problem_of, Ticks};
    use rnl_runtime::problem::{sig2, ProblemGroup};

    fn problem() -> LevelProblem {
        let mut p = problem_of(
            vec![instance(), instance()],
            vec![
                fact(FactKind::OrientRelation { relation: Orient::R0, from: vec![0], to: vec![1] }),
                fact(FactKind::Align { axis: Axis::Y, members: vec![PointRef::Origin(0), PointRef::Origin(1)] }),
            ],
        );
        p.version = "0.4".into();
        p.orient_lists = vec![(0, vec![Orient::R0, Orient::MY])];
        p.groups.push(ProblemGroup {
            identity: 0,
            first: true,
            direct: true,
            label: "%pair".into(),
            members: vec![0, 1],
            spacing_x: None,
            spacing_y: None,
        });
        p
    }

    fn options() -> SolverOptions {
        let mut o = SolverOptions::without_time_limit();
        o.settings.layout = crate::Layout::Tree;
        o.objectives =
            o.with_overrides("objectives=overlaps,crowding,misleading,wire_over_symbol,area").unwrap().objectives;
        o.safety_seconds = None;
        o
    }

    #[test]
    fn line_search_does_not_reward_singleton_wrappers_in_ties() {
        let p = problem();
        let options = options();
        let (_, exact) = crate::driver::solve_exact(&p, &options);
        let tree = Tree::new(&p, &exact.structure, &options).with_line_rank();
        let mut arr = tree.first(&mut Work::default());
        let original = tree.code(&arr);
        for dir in [Dir::Row, Dir::Column, Dir::Row] {
            let list = arr.lists[1].take().unwrap();
            arr.lists[1] = Some(Item::List(dir, vec![list]));
            assert_eq!(tree.code(&arr), original);
        }
    }

    #[test]
    fn move_types_do_not_come_from_user_labels() {
        let mut p = problem();
        let options = options();
        let (_, exact) = crate::driver::solve_exact(&p, &options);
        let before = Tree::new(&p, &exact.structure, &options);
        let first = before.first(&mut Work::default());
        let moves = before.neighbours(&first);
        assert!(moves.iter().any(|m| !m.3));
        let structural: Vec<_> = moves.iter().map(|m| (m.1.clone(), m.2.clone(), m.3)).collect();
        p.groups[0].label = "%x turned y: mirrored".into();
        p.objects[0].label = "#x turned y: mirrored".into();
        let renamed = Tree::new(&p, &exact.structure, &options);
        let moves: Vec<_> = renamed.neighbours(&first).into_iter().map(|m| (m.1, m.2, m.3)).collect();
        assert_eq!(moves, structural);
    }

    #[test]
    fn exhausted_budget_skips_optional_pin_pairs_and_preserves_facts() {
        let p = problem();
        let options = options();
        let (_, mut exact) = crate::driver::solve_exact(&p, &options);
        exact.structure.connections.drawn.push(crate::structure::Drawn {
            net: "fanout".into(),
            routed: true,
            points: vec![PointId::Pin(0, 0); 10_000],
        });
        let tree = Tree::new(&p, &exact.structure, &options);
        let first = tree.first(&mut Work::default());
        let ((pull, placed), budget) =
            crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(0), None, || {
                let pull = tree.ties(&first, &exact.placed.geometry.orient);
                let placed = place_tree(&p, &exact.structure, &[], &[], &pull, &options, &mut Work::default());
                (pull, placed)
            });
        assert_eq!(budget.work, 0);
        assert!(pull.layers.iter().all(Vec::is_empty));
        let mut drawn = vec![rnl_runtime::previous::Drawn { at: [None, None], orient: None }; p.objects.len()];
        for (object, origin) in placed.origins.iter().enumerate() {
            drawn[object] = rnl_runtime::previous::Drawn {
                at: [Some(crate::rational::to_f64(&origin[0])), Some(crate::rational::to_f64(&origin[1]))],
                orient: Some(placed.geometry.orient[object]),
            };
        }
        for &f in &exact.resolved.kept {
            assert_eq!(crate::problem_fact_holds_as_drawn(&p, &drawn, f, 1.00001e-6), Some(true));
        }
    }

    #[test]
    fn seed_runs_tree_refinement_and_keeps_the_original_facts() {
        let p = problem();
        let mut o = options();
        o.run_limit = Some(crate::RunLimit::Seed);
        let solved = crate::solve(&p, &o);
        assert_eq!(solved.report.kept.len(), p.facts.len());
        assert!(solved.report.notes.iter().any(|n| n.contains("tree: the search tried")));
        assert!(solved.report.notes.iter().any(|n| n.contains("none of the next ones better")));
        assert_eq!(solved, crate::solve(&p, &o));
        assert_eq!(
            SolverOptions::parse(solved.report.canonical_options.as_deref().unwrap()).unwrap().settings.layout,
            crate::Layout::Tree
        );
    }

    #[test]
    fn an_interrupted_challenger_cannot_replace_a_complete_tree_drawing() {
        let p = problem();
        let mut o = options();
        o.objectives = o.with_overrides("objectives=area").unwrap().objectives;
        let resolved = crate::resolve(&p, &o);
        let s = crate::structure::analyse(&p, &resolved, &o, crate::options::CAPS, &mut Work::default());
        let tree = Tree::new(&p, &s, &o);
        let first = tree.first(&mut Work::default());
        let (complete, _) = tree.evaluate_rounds(&first, 0, &mut Work::default());
        let mut evaluated = false;
        for bindings in [false, true] {
            for seconds in [1., 10., 30., 50., 100.] {
                let run = || {
                    let mut work = Work::default();
                    let made = tree.search(first.clone(), complete.clone(), 3, bindings, &mut work, &Stop::default());
                    (made, work)
                };
                let ((made, work), report) =
                    crate::budget::with(&Ticks::default(), crate::RunLimit::Time(seconds), None, run);
                evaluated |= work.pivots > 0;
                assert_eq!(report.reason, crate::StopReason::Time);
                assert_eq!(made.0, first, "binding={bindings}, cutoff={seconds}");
                assert_eq!(made.1.placed.geometry, complete.placed.geometry);
                assert_eq!(made.1.routing.wires, complete.routing.wires);
                assert_eq!(made.1.entry, complete.entry);
                let ((replay, _), _) =
                    crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(report.work), None, run);
                assert_eq!(made.0, replay.0);
                assert_eq!(made.1.placed.geometry, replay.1.placed.geometry);
                assert_eq!(made.1.routing.wires, replay.1.routing.wires);
                assert_eq!(made.1.entry, replay.1.entry);
            }
        }
        assert!(evaluated, "at least one cutoff must be inside challenger placement");
    }

    #[test]
    fn tree_time_cutoffs_replay_as_work_and_preserve_every_kept_fact() {
        let p = problem();
        let record = |solved: &crate::Solved| {
            rnl_runtime::solution::write_record(
                &p,
                &solved.solution,
                crate::AUTHOR,
                &sig2(&p, solved.report.canonical_options.as_ref().unwrap()),
            )
            .record
        };
        for seconds in [0, 1, 10, 50, 100, 250, 500, 1000, 4000] {
            let o = SolverOptions { run_limit: Some(crate::RunLimit::Time(f64::from(seconds))), ..options() };
            let timed = crate::solve_with_clock(&p, &o, &Ticks::default());
            let budget = timed.report.budget.unwrap();
            let work = SolverOptions { run_limit: Some(crate::RunLimit::Work(budget.work)), ..o };
            let replay = crate::solve(&p, &work);
            assert_eq!(record(&timed), record(&replay), "cutoff {seconds}, work {}", budget.work);
            assert_eq!(timed.report.kept, replay.report.kept);
            let mut drawn = vec![rnl_runtime::previous::Drawn { at: [None, None], orient: None }; p.objects.len()];
            for placed in &timed.solution.placements {
                drawn[placed.object] = rnl_runtime::previous::Drawn { at: [placed.x, placed.y], orient: placed.orient };
            }
            for &f in &timed.report.kept {
                assert_eq!(crate::problem_fact_holds_as_drawn(&p, &drawn, f, 1.00001e-6), Some(true));
            }
        }
    }

    #[test]
    fn an_astar_cutoff_during_a_real_binding_keeps_the_incumbent_wires() {
        use rnl_runtime::problem::RouteIntent;
        let mut p = problem();
        p.orient_lists.clear();
        p.facts = vec![
            fact(FactKind::Orient { object: 0, code: Orient::MY }),
            fact(FactKind::Orient { object: 1, code: Orient::R0 }),
            fact(FactKind::Align { axis: Axis::Y, members: vec![PointRef::Origin(0), PointRef::Origin(1)] }),
            fact(FactKind::Distance {
                axis: Axis::X,
                sense: Sense::Increasing,
                min: Some(30.),
                max: None,
                from: vec![PointRef::Origin(0)],
                to: vec![PointRef::Origin(1)],
            }),
        ];
        p.routes.push(RouteIntent {
            tree: true,
            net: "signal".into(),
            mode: "manhattan".into(),
            style: vec![],
            pins: (0..2).map(|object| PointRef::Pin { object, pin: 0, terminal: "1".into() }).collect(),
            wires: vec![],
        });
        let o = options();
        let resolved = crate::resolve(&p, &o);
        let s = crate::structure::analyse(&p, &resolved, &o, crate::options::CAPS, &mut Work::default());
        let tree = Tree::new(&p, &s, &o);
        let first = tree.first(&mut Work::default());
        let (complete, _) = tree.evaluate_rounds(&first, 0, &mut Work::default());
        assert!(!complete.routing.wires.is_empty());
        let (bound, _) = tree.bind(&first, &complete, &[]).expect("the facing pins form an actual binding");
        assert!(!bound.bound.is_empty());
        let ticks = Ticks::default();
        let (full_work, _) = crate::budget::with(&ticks, crate::RunLimit::Time(f64::MAX), None, || {
            let mut work = Work::default();
            assert!(!Stop::default().reached(&work));
            let (a, _) = tree.bind(&first, &complete, &[]).unwrap();
            tree.evaluate_rounds(&a, 0, &mut work);
            work
        });
        assert!(full_work.astar_expansions > 1);
        let mut caught = false;
        for back in 1..=32 {
            let seconds = ticks.reads().saturating_sub(back) as f64;
            let run = || {
                let mut work = Work::default();
                let made = tree.search(first.clone(), complete.clone(), 3, true, &mut work, &Stop::default());
                (made, work)
            };
            let ((made, work), report) =
                crate::budget::with(&Ticks::default(), crate::RunLimit::Time(seconds), None, run);
            if work.astar_expansions == 0 || work.astar_expansions >= full_work.astar_expansions {
                continue;
            }
            assert_eq!(report.reason, crate::StopReason::Time);
            assert_eq!(made.0, first);
            assert_eq!(made.1.placed.geometry, complete.placed.geometry);
            assert_eq!(made.1.routing.wires, complete.routing.wires);
            let ((replayed, replay_work), _) =
                crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(report.work), None, run);
            assert_eq!(work.astar_expansions, replay_work.astar_expansions);
            assert_eq!(made.0, replayed.0);
            assert_eq!(made.1.placed.geometry, replayed.1.placed.geometry);
            assert_eq!(made.1.routing.wires, replayed.1.routing.wires);
            caught = true;
            break;
        }
        assert!(caught, "must interrupt A* after it starts evaluating the bound challenger");
    }
}
