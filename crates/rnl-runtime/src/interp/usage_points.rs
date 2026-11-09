//! Own points of usages and coordinates in general definition bodies (0.4, 9.4, 12.1).

use super::ctx::{Ctx, FrameKind};
use super::obj::*;
use super::scene::{Fact, FactData};
use super::Interp;
use crate::diag::{codes, Severity};
use std::collections::{HashMap, HashSet, VecDeque};

/// None is the fixed scene origin. An edge says `to = from + offset` on one axis.
type Node = Option<ObjId>;
type Edges = HashMap<Node, Vec<(Node, f64, usize)>>;

/// Weighted disjoint sets keep insertion near-linear even for long chains of body offsets.
#[derive(Default)]
struct Coordinates {
    parent: HashMap<Node, (Node, f64)>,
    size: HashMap<Node, usize>,
}

impl Coordinates {
    fn root(&self, mut node: Node) -> (Node, f64) {
        let mut value = 0.0;
        while let Some(&(parent, offset)) = self.parent.get(&node) {
            value += offset;
            node = parent;
        }
        (node, value)
    }

    fn insert(&mut self, from: Node, to: Node, value: f64, tolerance: f64) -> bool {
        let (a, av) = self.root(from);
        let (b, bv) = self.root(to);
        if a == b {
            return (bv - av - value).abs() <= tolerance;
        }
        let offset = value + av - bv;
        let sa = *self.size.get(&a).unwrap_or(&1);
        let sb = *self.size.get(&b).unwrap_or(&1);
        if sa >= sb {
            self.parent.insert(b, (a, offset));
            self.size.insert(a, sa + sb);
        } else {
            self.parent.insert(a, (b, -offset));
            self.size.insert(b, sa + sb);
        }
        true
    }
}

fn paths_from(edges: &Edges, from: Node) -> HashMap<Node, (f64, Vec<usize>)> {
    let mut found = HashMap::from([(from, (0.0, Vec::new()))]);
    let mut queue = VecDeque::from([from]);
    while let Some(node) = queue.pop_front() {
        let (value, proof) = found[&node].clone();
        for &(next, offset, fact) in edges.get(&node).into_iter().flatten() {
            if found.contains_key(&next) {
                continue;
            }
            let mut path = proof.clone();
            path.push(fact);
            found.insert(next, (value + offset, path));
            queue.push_back(next);
        }
    }
    found
}

/// Every simple path is a minimal set tying the two coordinates. Bound the diagnostic search,
/// not the interpretation, and say when the list is incomplete (8.5).
fn conflict_paths(edges: &Edges, from: Node, to: Node) -> (Vec<Vec<usize>>, bool) {
    let mut stack = vec![(from, 0)];
    let mut visited = HashSet::from([from]);
    let mut path = Vec::new();
    let mut found = Vec::new();
    let mut steps = 0;
    while let Some((node, next)) = stack.last_mut() {
        if steps >= 4096 || found.len() >= 256 {
            return (found, false);
        }
        steps += 1;
        if *node == to {
            found.push(path.clone());
        } else if let Some(&(dest, _, fact)) = edges.get(node).and_then(|es| es.get(*next)) {
            *next += 1;
            if visited.insert(dest) {
                stack.push((dest, 0));
                path.push(fact);
            }
            continue;
        }
        let (done, _) = stack.pop().unwrap();
        visited.remove(&done);
        path.pop();
    }
    (found, true)
}

impl Interp<'_> {
    /// TEXT offsets replace earlier PLACEs before geometric resolution (13.3).
    fn text_replaces_coordinate(&self, s: SceneId, f: &Fact) -> bool {
        if !matches!(self.o(f.target), ObjData::Text { .. } | ObjData::KeyText { .. }) {
            return false;
        }
        let axis = match f.data {
            FactData::Coord(a, _) | FactData::RelativeCoord(a, _, _) => a,
            _ => return false,
        };
        let items = self.text_items(f.target);
        if (if axis == 'x' { items.dx } else { items.dy }).is_none() {
            return false;
        }
        let latest = self.scenes[s]
            .at_by_target
            .get(&f.target)
            .into_iter()
            .flatten()
            .map(|&k| &self.scenes[s].at[k])
            .filter(|f| {
                matches!(f.data, FactData::Coord(a, _) | FactData::RelativeCoord(a, _, _) if a == axis)
                    && self.fact_priority(s, f) != Priority::Disabled
            })
            .map(|f| f.order())
            .max();
        latest.is_none_or(|p| self.offset_pos(s, f.target, axis == 'x') > p)
    }
    /// A positional relation reaches a pin through its owner, not through net membership.
    fn position_owner(&self, object: ObjId) -> ObjId {
        let object = self.coordinate_object(object);
        match self.o(object) {
            ObjData::Term { owner, .. } => self.bundle_field(*owner).unwrap_or(*owner),
            _ => object,
        }
    }

    /// Only actual positional facts connect a usage point to a drawing. Role membership and
    /// the ancestry of an expanded body do not. Follow each axis separately and transitively.
    pub(super) fn hint_idle_usage_points(&mut self, s: SceneId) {
        if !self.scenes[s].usages.iter().any(|&u| self.usage_own_point(u)) {
            return;
        }
        type Coordinate = (ObjId, char);
        let mut edges: HashMap<Coordinate, Vec<Coordinate>> = HashMap::new();
        let mut link = |a: ObjId, b: ObjId, axis: char| {
            edges.entry((a, axis)).or_default().push((b, axis));
            edges.entry((b, axis)).or_default().push((a, axis));
        };
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();
        for c in self.scenes[s].constraints.clone() {
            let ObjData::Constraint(cd) = self.o(c).clone() else { continue };
            if cd.failed
                || matches!(cd.status, Some("disabled" | "dropped"))
                || cd.kind == ConstraintKind::OrientRelation
            {
                continue;
            }
            let members = self.leaves(s, &cd.args);
            let axis = cd.params.axis.unwrap_or('x');
            let members: Vec<_> = members.into_iter().map(|o| self.position_owner(o)).collect();
            for &o in &members {
                if matches!(self.o(o), ObjData::Usage(_)) && self.usage_own_point(o) && seen.insert(o) {
                    candidates.push((o, cd.site.clone(), cd.off));
                }
                if let Some(&first) = members.first() {
                    link(first, o, axis);
                }
            }
        }
        for &k in &self.scenes[s].relative_kept {
            let f = &self.scenes[s].at[k];
            if let FactData::RelativeCoord(axis, _, base) = f.data {
                link(self.position_owner(f.target), self.position_owner(base), axis);
            }
        }
        let mut drawn: HashSet<Coordinate> = self.scenes[s].symbol.keys().flat_map(|&o| [(o, 'x'), (o, 'y')]).collect();
        for &p in &self.scenes[s].prims {
            if let ObjData::Prim { points, .. } = self.o(p) {
                for &point in points {
                    let point = self.position_owner(point);
                    drawn.extend([(point, 'x'), (point, 'y')]);
                }
            }
        }
        for w in self.scenes[s].wires.clone() {
            if self.is_obj_failed(w) {
                continue;
            }
            if let Some(shape) = self.wire_shape(s, w) {
                for point in shape.points {
                    let point = self.position_owner(point);
                    drawn.extend([(point, 'x'), (point, 'y')]);
                }
            }
        }
        for &t in &self.scenes[s].texts {
            if self.is_obj_failed(t) || self.text_items(t).text.as_ref().is_none_or(|text| text.as_str().is_empty()) {
                continue;
            }
            drawn.extend([(t, 'x'), (t, 'y')]);
            let items = self.text_items(t);
            let attached = self.position_owner(self.text_attach(t));
            for (axis, offset) in [('x', items.dx), ('y', items.dy)] {
                let latest = self.scenes[s]
                    .at_by_target
                    .get(&t)
                    .into_iter()
                    .flatten()
                    .map(|&k| &self.scenes[s].at[k])
                    .filter(|f| {
                        matches!(f.data, FactData::Coord(a, _) | FactData::RelativeCoord(a, _, _) if a == axis)
                            && self.fact_priority(s, f) != Priority::Disabled
                    })
                    .map(|f| f.order())
                    .max();
                if latest.is_none()
                    || offset.is_some() && latest.is_some_and(|p| self.offset_pos(s, t, axis == 'x') > p)
                {
                    link(t, attached, axis);
                }
            }
        }
        let mut queue: VecDeque<_> = drawn.iter().copied().collect();
        while let Some(node) = queue.pop_front() {
            for &next in edges.get(&node).into_iter().flatten() {
                if drawn.insert(next) {
                    queue.push_back(next);
                }
            }
        }
        for (usage, site, off) in candidates {
            if !drawn.contains(&(usage, 'x')) && !drawn.contains(&(usage, 'y')) {
                self.diag_site(Severity::Hint, codes::USAGE_POINT_IDLE_04, &site, off,
                    format!("{} has no symbol, and its point is tied to nothing that is drawn: the constraints on it change nothing drawn (14.1)", self.label(usage)));
            }
        }
    }

    /// An explicit ::position and its owner share one coordinate variable.
    pub(super) fn coordinate_object(&self, object: ObjId) -> ObjId {
        match self.o(object) {
            ObjData::Pos { parent } if !self.is_set(*parent) => self.coordinate_object(*parent),
            _ => object,
        }
    }

    /// Resolve only coordinate equalities. Other positional constraints are still checked by
    /// the runtime and solved by the solver, as for ordinary PLACE facts.
    pub(super) fn resolve_body_coordinates(&mut self, s: SceneId) {
        let facts = self.scenes[s].at.clone();
        let mut ranked: Vec<_> = self.scenes[s]
            .at_live
            .iter()
            .copied()
            .filter(|&k| {
                self.fact_priority(s, &facts[k]) != Priority::Disabled && !self.text_replaces_coordinate(s, &facts[k])
            })
            .collect();
        ranked.sort_by_key(|&k| std::cmp::Reverse((self.fact_priority(s, &facts[k]) as u8, facts[k].order(), k)));
        self.scenes[s].pos.clear();
        self.scenes[s].at_best.clear();
        for axis in ['x', 'y'] {
            let mut edges: Edges = HashMap::new();
            let mut coordinates = Coordinates::default();
            for &k in &ranked {
                let f = &facts[k];
                let (a, value, base) = match f.data {
                    FactData::Coord(a, v) => (a, v, None),
                    FactData::RelativeCoord(a, v, b) => (a, v, Some(self.coordinate_object(b))),
                    _ => continue,
                };
                if a != axis {
                    continue;
                }
                let object = self.coordinate_object(f.target);
                let target = Some(object);
                if !coordinates.insert(base, target, value, self.tol) {
                    let (paths, complete) = conflict_paths(&edges, base, target);
                    let sets =
                        paths.into_iter().map(|path| path.into_iter().map(|i| facts[i].clone()).collect()).collect();
                    self.note_coordinate_conflict(s, f.clone(), sets, complete);
                    continue;
                }
                edges.entry(base).or_default().push((target, value, k));
                edges.entry(target).or_default().push((base, -value, k));
                self.scenes[s].relative_kept.insert(k);
                if base.is_none() {
                    let priority = self.fact_priority(s, f);
                    self.scenes[s].at_best.entry((object, axis)).or_insert((priority, f.order(), k));
                }
            }
            for (node, (value, proof)) in paths_from(&edges, None) {
                let Some(object) = node else { continue };
                self.scenes[s].pos.entry(object).or_insert([None, None])[(axis == 'y') as usize] = Some(value);
                self.scenes[s].coordinate_proofs.insert((object, axis), proof);
            }
        }
    }
    /// Versioning follows the file that creates the usage, including mixed-version includes.
    pub(super) fn usage_own_point(&self, usage: ObjId) -> bool {
        let u = self.usage(usage);
        !u.in_symbol && u.site.as_ref().is_some_and(|c| self.follows_0_4(self.source_unit(c)))
    }

    /// Body coordinates translate with this point; symbol coordinates keep their rigid frame.
    pub(super) fn coordinate_base(&self, ctx: &Ctx) -> Option<ObjId> {
        if ctx.frame_kind() == Some(FrameKind::Definition) && self.follows_0_4(self.source_unit(ctx)) {
            ctx.frame_usage()
        } else {
            None
        }
    }
}
