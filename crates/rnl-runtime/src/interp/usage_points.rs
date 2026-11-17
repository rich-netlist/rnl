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
    /// the ancestry of an expanded body do not, and neither does a statement that places the
    /// point itself, `%u::position` written as an item of a statement not of its own body: that
    /// statement moves the point and nothing the point is tied to. Follow each axis separately
    /// and transitively. Each statement that places the point of a usage without a symbol that is
    /// then tied to nothing drawn has the hint, which says to write the usage itself, which stands
    /// for its members (14.1).
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
        // the point of a usage without a symbol that a statement other than one of its own body
        // places; the usage itself, written as an item, stands for its members and is not here
        let placed = |this: &Self, o: ObjId, chain: &[ObjId]| {
            matches!(this.o(o), ObjData::Usage(_))
                && this.usage_own_point(o)
                && !this.scenes[s].symbol.contains_key(&o)
                && !chain.contains(&o)
        };
        for c in self.scenes[s].constraints.clone() {
            let ObjData::Constraint(cd) = self.o(c).clone() else { continue };
            if cd.failed
                || matches!(cd.status, Some("disabled" | "dropped"))
                || cd.kind == ConstraintKind::OrientRelation
            {
                continue;
            }
            let key = |o: ObjId| (o, cd.site.as_ref().map(std::rc::Rc::as_ptr), cd.off);
            if let Some(rows) = &cd.table {
                for &point in rows.points() {
                    let object = self.position_owner(point);
                    if placed(self, object, &cd.chain) && seen.insert(key(object)) {
                        candidates.push((object, cd.site.clone(), cd.off));
                    }
                }
                for (axis, from, to) in rows.orders(crate::problem::Axis::of(cd.params.axis.unwrap_or('y'))) {
                    let axis = if axis == crate::problem::Axis::X { 'x' } else { 'y' };
                    let (from, to) = (self.position_owner(*from), self.position_owner(*to));
                    if !placed(self, from, &cd.chain) && !placed(self, to, &cd.chain) {
                        link(from, to, axis);
                    }
                }
                continue;
            }
            let members = self.leaves(s, &cd.args);
            let axis = cd.params.axis.unwrap_or('x');
            let members: Vec<_> = members.into_iter().map(|o| self.position_owner(o)).collect();
            let mut tied = Vec::new();
            for &o in &members {
                if placed(self, o, &cd.chain) {
                    if seen.insert(key(o)) {
                        candidates.push((o, cd.site.clone(), cd.off));
                    }
                } else {
                    tied.push(o);
                }
            }
            for &o in &tied {
                link(tied[0], o, axis);
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
            // a statement of a package, which the document cannot rewrite, has no hint
            if site.as_ref().is_some_and(|c| !self.units[c.unit].pkg().is_empty()) {
                continue;
            }
            if !drawn.contains(&(usage, 'x')) && !drawn.contains(&(usage, 'y')) {
                let name = self.label(usage);
                let advice = if name.starts_with('%') {
                    format!("write {name} to constrain its members")
                } else {
                    "write the usage itself to constrain its members".to_string()
                };
                self.diag_site(Severity::Hint, codes::USAGE_POINT_IDLE_04, &site, off,
                    format!("{name} has no symbol, and no fact ties its ::position to what is drawn: this statement moves only that point, not its members; {advice} (14.1)"));
            }
        }
    }

    /// An explicit ::position and its owner share one coordinate variable.
    pub(super) fn coordinate_object(&self, object: ObjId) -> ObjId {
        match self.o(object) {
            // the `::position` of a RECTANGLE, CIRCLE or ELLIPSE written with a centre is its centre
            // point, which PLACE takes and moves; the graphic itself it does not take (11.1, 13.4)
            ObjData::Pos { parent } => match self.o(*parent) {
                ObjData::Prim { shape: Shape::Rectangle | Shape::Circle | Shape::Ellipse, points, .. }
                    if points.len() == 1 =>
                {
                    self.coordinate_object(points[0])
                }
                ObjData::Prim { .. } => object,
                _ if self.is_set(*parent) => object,
                _ => self.coordinate_object(*parent),
            },
            ObjData::Annotation(data) => match data.anchor {
                AnnotationAnchor::Independent(point) => self.coordinate_object(point),
                AnnotationAnchor::Follow(object) => self.coordinate_object(object),
            },
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
