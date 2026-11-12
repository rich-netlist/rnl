//! Read exact placements and drawn wire paths as line identities and constraints.

use std::collections::{BTreeMap, BTreeSet};

use num_traits::{One, Zero};
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{Filler, LevelProblem, PointRef, Position};
use rnl_runtime::solution::WirePoint;

use super::cross::Cross;
use super::store::Why;
use crate::model::{Bound, PointId, VarKind};
use crate::rational::{int, Q};
use crate::resolve::Geometry;
use crate::route::Routing;
use crate::simplex::LinExpr;
use crate::structure::{NodeKind, Role, Structure};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Term {
    pub var: usize,
    pub off: Q,
}

impl Term {
    pub(crate) fn of(var: usize, off: Q) -> Term {
        Term { var, off }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Key {
    Pin(usize, usize),
    PinPosition(usize, usize),
    Coordinate(usize, usize),
    Point(usize),
    Branch(usize),
    Bend(usize, usize),
}

#[derive(Clone, Debug)]
pub(crate) struct Vertex {
    pub key: Key,
    pub at: [Q; 2],
    pub term: [Term; 2],
    /// A shared electrical vertex can belong to several independently routed trees.
    pub trees: BTreeSet<usize>,
    pub spot: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Seg {
    pub tree: usize,
    pub a: usize,
    pub b: usize,
    pub along: Option<usize>,
    pub slant: bool,
    pub routed: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Path {
    pub tree: usize,
    pub vertices: Vec<usize>,
    pub routed: Option<usize>,
    pub nets: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Spot {
    pub vertices: Vec<usize>,
    pub segs: Vec<usize>,
    pub pins: Vec<(usize, usize)>,
    pub point: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Symbol(usize),
    Text(usize),
    Segment(usize),
    Pin(usize, usize),
    Group(usize),
}

#[derive(Clone, Debug)]
pub(crate) struct Feature {
    pub kind: Kind,
    pub lo: [Term; 2],
    pub hi: [Term; 2],
}

#[derive(Clone, Debug)]
pub(crate) struct GroupBox {
    pub group: Option<usize>,
    pub members: BTreeSet<usize>,
}

#[derive(Clone, Debug)]
pub(crate) enum Made {
    Equal(usize, Q, usize, Q, Why),
    AtLeast(usize, Q, usize, Q, Q, Why),
    Bound(LinExpr, Option<Q>, Option<Q>, Why),
}

#[derive(Clone, Debug)]
pub(crate) struct Skeleton {
    pub vars: usize,
    pub zero: usize,
    pub value: [Vec<Q>; 2],
    pub origin: Vec<Option<usize>>,
    pub orient: Vec<Orient>,
    pub vertices: Vec<Vertex>,
    pub segs: Vec<Seg>,
    pub paths: Vec<Path>,
    pub spots: Vec<Spot>,
    pub features: Vec<Feature>,
    pub groups: Vec<GroupBox>,
    pub made: [Vec<Made>; 2],
    pub facts: Vec<usize>,
    pub tree_objects: BTreeMap<usize, BTreeSet<usize>>,
    pub usages_of: BTreeMap<usize, BTreeSet<usize>>,
    pub groups_of: BTreeMap<usize, BTreeSet<usize>>,
    pub pins: Vec<((usize, usize), [Term; 2])>,
    pub exits: BTreeMap<(usize, usize), (i64, i64)>,
    pub near: Vec<BTreeSet<usize>>,
    pub replaced: Vec<(usize, usize, usize)>,
    pub notes: Vec<String>,
    pub strict: bool,
    pub whole: bool,
    pub within: Option<BTreeSet<usize>>,
    pub levels: Vec<((usize, usize), (usize, usize))>,
    pub rails: Vec<((usize, usize), (usize, usize))>,
    pub crosses: Vec<Cross>,
    pub axes: Vec<usize>,
    pub on_grid: bool,
}

fn lead() -> Q {
    int(crate::route::CLEARANCE)
}

fn min(a: Q, b: Q) -> Q {
    if a < b {
        a
    } else {
        b
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn read(
    problem: &LevelProblem,
    s: &Structure,
    g: &Geometry,
    routing: &Routing,
    keep: &dyn Fn(usize) -> bool,
    strict: bool,
    whole: bool,
    within: Option<&BTreeSet<usize>>,
    crosses: bool,
) -> Option<Skeleton> {
    if !crate::budget::permit(crate::budget::Step::Structure, problem.objects.len()) {
        return None;
    }
    let model = &s.model;
    let n_objects = problem.objects.len();
    let mut value: [Vec<Q>; 2] =
        [0, 1].map(|axis| (0..model.vars.len()).map(|v| g.var_value(model, v, axis)).collect());
    let zero = model.vars.len();
    for v in &mut value {
        v.push(Q::zero());
    }
    let mut origin: Vec<Option<usize>> = (0..n_objects).map(|k| model.origin_point[k].map(|p| model.var(p))).collect();
    for (k, object) in problem.objects.iter().enumerate() {
        if matches!(object.position, Position::InSymbol { replaced: Some(_), .. }) {
            origin[k] = if let Some(info) = model.points.get(&PointId::Text(k)) {
                Some(info.var)
            } else {
                let var = value[0].len();
                for (axis, values) in value.iter_mut().enumerate() {
                    values.push(crate::resolve::point_value(problem, g, &PointRef::Origin(k), axis, 0)?);
                }
                Some(var)
            };
        }
    }
    let axes: Vec<usize> = s.pairs.iter().filter_map(|p| p.axis.and_then(|k| origin[k])).collect();
    let mut sk = Skeleton {
        vars: value[0].len(),
        zero,
        value,
        origin,
        orient: g.orient.clone(),
        vertices: Vec::new(),
        segs: Vec::new(),
        paths: Vec::new(),
        spots: Vec::new(),
        features: Vec::new(),
        groups: Vec::new(),
        made: [Vec::new(), Vec::new()],
        facts: Vec::new(),
        tree_objects: BTreeMap::new(),
        usages_of: BTreeMap::new(),
        groups_of: BTreeMap::new(),
        pins: Vec::new(),
        exits: BTreeMap::new(),
        near: Vec::new(),
        replaced: Vec::new(),
        notes: Vec::new(),
        strict,
        whole,
        within: within.cloned(),
        levels: Vec::new(),
        rails: Vec::new(),
        axes,
        on_grid: true,
        crosses: Vec::new(),
    };
    sk.body(problem, s, g);
    if crate::budget::stopped() {
        return None;
    }
    if !sk.facts_kept(s, g, keep) {
        return None;
    }
    if crate::budget::stopped() {
        return None;
    }
    sk.wires(problem, s, g, routing, crosses);
    if crate::budget::stopped() {
        return None;
    }
    sk.features_of(problem, s, g);
    if crate::budget::stopped() {
        return None;
    }
    sk.texts_own(problem, s, g);
    if crate::budget::stopped() {
        return None;
    }
    super::link::attached(&mut sk, s);
    if crate::budget::stopped() {
        return None;
    }
    sk.groups_of_tree(problem, s);
    if crate::budget::stopped() {
        return None;
    }
    sk.usages(problem);
    if crate::budget::stopped() {
        return None;
    }
    if strict {
        sk.rails_of(s);
    }
    if crate::budget::stopped() {
        None
    } else {
        Some(sk)
    }
}

impl Skeleton {
    fn meter(&self, size: usize) -> bool {
        crate::budget::permit(crate::budget::Step::Structure, size)
    }

    fn new_var(&mut self, at: [Q; 2]) -> usize {
        let v = self.vars;
        self.vars += 1;
        for (values, at) in self.value.iter_mut().zip(at) {
            values.push(at);
        }
        v
    }

    fn reads(&self, k: usize) -> bool {
        self.within.as_ref().is_none_or(|w| w.contains(&k))
    }

    fn reads_reference(&self, reference: &PointRef) -> bool {
        match reference {
            PointRef::Pin { object, .. }
            | PointRef::SymbolPin { object, .. }
            | PointRef::PinPosition { object, .. }
            | PointRef::Origin(object) => self.reads(*object),
            PointRef::Centre(members) => members.iter().all(|r| self.reads_reference(r)),
        }
    }

    pub(crate) fn at(&self, axis: usize, t: &Term) -> Q {
        &self.value[axis][t.var] + &t.off
    }

    fn body(&mut self, problem: &LevelProblem, s: &Structure, g: &Geometry) {
        let model = &s.model;
        for (&p, info) in &model.points {
            if !self.meter(1) {
                return;
            }
            let (Some(k), Some(local)) = (info.object, &info.local) else { continue };
            let Some(base) = model.origin_point[k].map(|b| model.var(b)) else { continue };
            let var = info.var;
            if var == base {
                continue;
            }
            for axis in 0..2 {
                let off = if info.linked(axis) {
                    crate::model::turned(g.orient[k], info.local_for(g.orient[k]).unwrap_or(local), axis)
                } else {
                    &self.value[axis][var] - &self.value[axis][base]
                };
                self.made[axis].push(Made::Equal(var, Q::zero(), base, off, Why::Body));
            }
            let _ = p;
        }
        for (k, o) in problem.objects.iter().enumerate() {
            if let Position::InSymbol { owner, replaced: Some((axis, _)), .. } = &o.position {
                if !s.objects[k].left_free {
                    self.replaced.push((k, *owner, crate::model::ax(*axis)));
                    if let (Some(var), Some(base)) = (self.origin[k], self.origin[*owner]) {
                        for axis in 0..2 {
                            let offset = &self.value[axis][var] - &self.value[axis][base];
                            self.made[axis].push(Made::Equal(var, Q::zero(), base, offset, Why::Body));
                        }
                    }
                }
            }
        }
    }

    fn facts_kept(&mut self, s: &Structure, g: &Geometry, keep: &dyn Fn(usize) -> bool) -> bool {
        let model = &s.model;
        let holds = |b: &Bound| b.holds(|v| g.var_value(model, v, b.axis));
        // A body or origin can depend on a centre without a retained bound naming it
        // directly (for example, a pin of an object positioned at a set's centre).
        // These are geometric definitions, including when the fact using them is omitted.
        let mut centres: BTreeSet<usize> = (0..model.centres.len()).collect();
        for &f in &s.kept {
            if !self.meter(1) {
                return false;
            }
            if !keep(f) {
                continue;
            }
            self.facts.push(f);
            let unit = &model.units[f];
            if model.unfinished.contains(&f) || !unit.fixed.iter().all(holds) {
                return false;
            }
            let mut bounds: Vec<&Bound> = unit.fixed.iter().collect();
            for c in &unit.choices {
                match c.values.iter().find(|bs| bs.iter().all(holds)) {
                    Some(bs) => bounds.extend(bs.iter()),
                    None => return false,
                }
            }
            for b in bounds {
                if !self.meter(b.expr.terms().len()) {
                    return false;
                }
                for (v, _) in b.expr.terms() {
                    match &model.vars[*v] {
                        VarKind::Point(PointId::Centre(c)) | VarKind::CentreLo(c) | VarKind::CentreHi(c) => {
                            centres.insert(*c);
                        }
                        _ => {}
                    }
                }
                self.made[b.axis].push(Made::Bound(b.expr.clone(), b.lo.clone(), b.hi.clone(), Why::Fact(f)));
            }
        }
        for c in centres {
            let members = &model.centres[c].members;
            if !self.meter(members.len()) {
                return false;
            }
            for axis in 0..2 {
                for b in model.centre_definition(c, axis) {
                    self.made[axis].push(Made::Bound(b.expr, b.lo, b.hi, Why::Body));
                }
                if members.len() >= 3 {
                    let values: Vec<Q> = members.iter().map(|p| g.value(model, *p, axis)).collect();
                    let low = (0..values.len()).min_by(|&i, &j| values[i].cmp(&values[j]).then(i.cmp(&j)));
                    let high = (0..values.len()).max_by(|&i, &j| values[i].cmp(&values[j]).then(j.cmp(&i)));
                    for (high, j) in [(false, low), (true, high)] {
                        if let Some(j) = j {
                            let b = model.centre_extreme(c, axis, high, j);
                            self.made[axis].push(Made::Bound(b.expr, b.lo, b.hi, Why::Body));
                        }
                    }
                }
            }
        }
        true
    }

    fn vertex(&mut self, s: &Structure, key: Key, at: &[Q; 2], tree: usize) -> usize {
        if !matches!(key, Key::Bend(..) | Key::PinPosition(..) | Key::Coordinate(..)) {
            if let Some(i) = self.vertices.iter().position(|v| v.key == key) {
                self.vertices[i].trees.insert(tree);
                return i;
            }
        }
        let term = match key {
            Key::Pin(k, pin) | Key::PinPosition(k, pin) => {
                if let Some(exit) = s.exit(k, pin, self.orient[k]) {
                    if matches!(key, Key::Pin(..)) {
                        self.exits.insert((k, pin), exit);
                    }
                }
                let off = s.pin_offset(k, pin, self.orient[k]);
                match (self.origin[k], off) {
                    (Some(o), Some(off)) => Some([Term::of(o, off[0].clone()), Term::of(o, off[1].clone())]),
                    _ => None,
                }
            }
            Key::Point(k) => self.origin[k].map(|o| [Term::of(o, Q::zero()), Term::of(o, Q::zero())]),
            Key::Coordinate(w, j) => s.model.wire_points[w][j].map(|p| {
                let v = s.model.var(p);
                [Term::of(v, Q::zero()), Term::of(v, Q::zero())]
            }),
            _ => None,
        };
        let term = term.unwrap_or_else(|| {
            let v = self.new_var(at.clone());
            [Term::of(v, Q::zero()), Term::of(v, Q::zero())]
        });
        self.vertices.push(Vertex { key, at: at.clone(), term, trees: [tree].into(), spot: 0 });
        self.vertices.len() - 1
    }

    fn wires(&mut self, problem: &LevelProblem, s: &Structure, g: &Geometry, routing: &Routing, crosses: bool) {
        for (i, (wire, path)) in routing.wires.iter().zip(&routing.paths).enumerate() {
            let tree = routing.drawn.get(i).copied().unwrap_or(i);
            let mut vertices = Vec::new();
            for (j, (p, at)) in wire.points.iter().zip(path).enumerate() {
                if !self.meter(self.vertices.len()) {
                    return;
                }
                let key = match p {
                    WirePoint::Anchor(PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. }) => {
                        Key::Pin(*object, *pin)
                    }
                    WirePoint::Anchor(PointRef::PinPosition { object, pin, .. }) => Key::PinPosition(*object, *pin),
                    WirePoint::Anchor(PointRef::Origin(k)) => Key::Point(*k),
                    WirePoint::Branch(b) => Key::Branch(*b),
                    _ => Key::Bend(self.paths.len(), j),
                };
                vertices.push(self.vertex(s, key, at, tree));
            }
            let nets = problem.routes.get(wire.route).map(|r| r.net.clone()).into_iter().collect();
            self.paths.push(Path { tree, vertices, routed: Some(i), nets });
        }
        let routes = problem.routes.len();
        for (w, wire) in problem.wires.iter().enumerate() {
            if !self.meter(wire.anchors.len()) {
                return;
            }
            if !wire.anchors.iter().all(|r| self.reads_reference(r)) {
                continue;
            }
            let tree = problem.routes.iter().position(|r| r.wires.contains(&w)).unwrap_or(routes + w);
            let mut anchors: Vec<(Key, [Q; 2], &PointRef)> = Vec::new();
            for (j, r) in wire.anchors.iter().enumerate() {
                let key = match r {
                    PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. } => {
                        Key::Pin(*object, *pin)
                    }
                    PointRef::PinPosition { object, pin, .. } => Key::PinPosition(*object, *pin),
                    PointRef::Origin(k) => Key::Point(*k),
                    PointRef::Centre(_) => Key::Coordinate(w, j),
                };
                let at = |axis| crate::resolve::point_value(problem, g, r, axis, 0);
                let (Some(x), Some(y)) = (at(0), at(1)) else { continue };
                anchors.push((key, [x, y], r));
            }
            if anchors.len() < 2 {
                continue;
            }
            let id = self.paths.len();
            let mut vertices = Vec::new();
            let first = self.vertex(s, anchors[0].0, &anchors[0].1, tree);
            vertices.push(first);
            for pair in anchors.windows(2) {
                if !self.meter(self.vertices.len()) {
                    return;
                }
                let (a, b) = (&pair[0], &pair[1]);
                let ia = *vertices.last().expect("one");
                let ib = self.vertex(s, b.0, &b.1, tree);
                let aligned = a.1[0] == b.1[0] || a.1[1] == b.1[1];
                if !aligned {
                    let mode = wire.mode.as_str();
                    match mode {
                        "horizontal-vertical" | "vertical-horizontal" => {
                            let hv = mode == "horizontal-vertical";
                            let (tx, ty) = if hv {
                                (self.vertices[ib].term[0].clone(), self.vertices[ia].term[1].clone())
                            } else {
                                (self.vertices[ia].term[0].clone(), self.vertices[ib].term[1].clone())
                            };
                            let at = [self.at(0, &tx), self.at(1, &ty)];
                            let key = Key::Bend(id, vertices.len());
                            self.vertices.push(Vertex { key, at, term: [tx, ty], trees: [tree].into(), spot: 0 });
                            vertices.push(self.vertices.len() - 1);
                        }
                        "manhattan" => {
                            let horizontal = (&b.1[0] - &a.1[0]).abs() >= (&b.1[1] - &a.1[1]).abs();
                            let along = if horizontal { 0 } else { 1 };
                            let mid = (&a.1[along] + &b.1[along]) / int(2);
                            let mut p1 = a.1.clone();
                            let mut p2 = b.1.clone();
                            p1[along] = mid.clone();
                            p2[along] = mid;
                            let v1 = self.vertex(s, Key::Bend(id, vertices.len()), &p1, tree);
                            vertices.push(v1);
                            let v2 = self.vertex(s, Key::Bend(id, vertices.len()), &p2, tree);
                            vertices.push(v2);
                            let (ta, tb, tm) = (
                                self.vertices[ia].term[along].clone(),
                                self.vertices[ib].term[along].clone(),
                                self.vertices[v1].term[along].clone(),
                            );
                            let e = LinExpr::from_terms(
                                [(tm.var, int(2)), (ta.var, -Q::one()), (tb.var, -Q::one())],
                                int(2) * &tm.off - &ta.off - &tb.off,
                            );
                            self.made[along].push(Made::Bound(e, Some(Q::zero()), Some(Q::zero()), Why::Wire));
                        }
                        "manhattan45" if (&b.1[0] - &a.1[0]).abs() != (&b.1[1] - &a.1[1]).abs() => {
                            let delta = [&b.1[0] - &a.1[0], &b.1[1] - &a.1[1]];
                            let along = usize::from(delta[1].abs() > delta[0].abs());
                            let run = (delta[along].abs() - delta[1 - along].abs()) / int(2) * delta[along].signum();
                            let mut p1 = a.1.clone();
                            let mut p2 = b.1.clone();
                            p1[along] += &run;
                            p2[along] -= &run;
                            let v1 = self.vertex(s, Key::Bend(id, vertices.len()), &p1, tree);
                            vertices.push(v1);
                            let v2 = self.vertex(s, Key::Bend(id, vertices.len()), &p2, tree);
                            vertices.push(v2);
                            // Keep the diagonal centred, as the renderer draws this wire mode.
                            let (ta, tb, tm) = (
                                &self.vertices[ia].term[along],
                                &self.vertices[ib].term[along],
                                &self.vertices[v1].term[along],
                            );
                            let diagonal = &p2[along] - &p1[along];
                            let e = LinExpr::from_terms(
                                [(tm.var, int(2)), (ta.var, -Q::one()), (tb.var, -Q::one())],
                                int(2) * &tm.off - &ta.off - &tb.off + diagonal,
                            );
                            self.made[along].push(Made::Bound(e, Some(Q::zero()), Some(Q::zero()), Why::Wire));
                        }
                        _ => {}
                    }
                }
                vertices.push(ib);
            }
            self.paths.push(Path { tree, vertices, routed: None, nets: wire.nets.clone() });
        }
        let found = if crosses { super::cross::find(problem, s, g, routing, self.within.as_ref()) } else { Vec::new() };
        if crate::budget::stopped() {
            return;
        }
        let jogs: BTreeSet<(usize, usize)> = found.iter().flat_map(|f| f.jogs).collect();
        let mut first_seg = Vec::new();
        for (p, path) in self.paths.clone().iter().enumerate() {
            first_seg.push(self.segs.len());
            for (j, pair) in path.vertices.windows(2).enumerate() {
                if !self.meter(1) {
                    return;
                }
                let (a, b) = (pair[0], pair[1]);
                let jog = jogs.contains(&(p, j));
                let (pa, pb) = (self.vertices[a].at.clone(), self.vertices[b].at.clone());
                let along = if pa == pb {
                    None
                } else if pa[1] == pb[1] {
                    Some(0)
                } else if pa[0] == pb[0] {
                    Some(1)
                } else {
                    None
                };
                let slant = pa != pb && along.is_none();
                self.segs.push(Seg { tree: path.tree, a, b, along, slant, routed: path.routed.is_some() });
                let (ta, tb) = (self.vertices[a].term.clone(), self.vertices[b].term.clone());
                for axis in 0..2 {
                    let on_line = along != Some(axis);
                    if (on_line || slant) && !jog {
                        let off = if slant { &pa[axis] - &pb[axis] } else { Q::zero() };
                        self.made[axis].push(Made::Equal(
                            ta[axis].var,
                            &ta[axis].off - &off,
                            tb[axis].var,
                            tb[axis].off.clone(),
                            Why::Wire,
                        ));
                    }
                }
            }
        }
        self.spots_of();
        if crate::budget::stopped() {
            return;
        }
        for (i, seg) in self.segs.clone().iter().enumerate() {
            if !self.meter(1) {
                return;
            }
            let Some(axis) = seg.along else { continue };
            let (a, b) = (&self.vertices[seg.a], &self.vertices[seg.b]);
            let (lo, hi) = if a.at[axis] < b.at[axis] { (seg.a, seg.b) } else { (seg.b, seg.a) };
            let length = &self.vertices[hi].at[axis] - &self.vertices[lo].at[axis];
            let least_lead = if self.strict && self.lead_to_branch(i) { lead() * int(2) } else { lead() };
            let least = if self.collapses(i).is_some() || self.z_middle(i) || (self.strict && !self.lead_or_floor(i)) {
                Q::zero()
            } else if self.whole {
                least_lead
            } else {
                min(length, least_lead)
            };
            let (tl, th) = (self.vertices[lo].term[axis].clone(), self.vertices[hi].term[axis].clone());
            self.made[axis].push(Made::AtLeast(th.var, th.off, tl.var, tl.off, least, Why::Wire));
        }
        for v in &self.vertices {
            if !self.meter(v.trees.len()) {
                return;
            }
            if let Key::Pin(k, _) = v.key {
                for &tree in &v.trees {
                    self.tree_objects.entry(tree).or_default().insert(k);
                }
            }
        }
        for f in &found {
            self.cross(problem, s, routing, &first_seg, f);
        }
        let mut near: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); self.segs.len()];
        for spot in &self.spots {
            for &a in &spot.segs {
                for &b in &spot.segs {
                    if !self.meter(1) {
                        return;
                    }
                    if a != b {
                        near[a].insert(b);
                    }
                }
            }
        }
        for m in 0..self.segs.len() {
            if self.z_middle(m) {
                let ends = [self.vertices[self.segs[m].a].spot, self.vertices[self.segs[m].b].spot];
                let beyond: Vec<usize> = ends.iter().filter_map(|&sp| self.bend_beyond(m, sp)).collect();
                if let [a, b] = beyond.as_slice() {
                    near[*a].insert(*b);
                    near[*b].insert(*a);
                }
            }
        }
        self.near = near;
    }

    fn cross(
        &mut self,
        problem: &LevelProblem,
        s: &Structure,
        routing: &Routing,
        first_seg: &[usize],
        f: &super::cross::Found,
    ) {
        let (axis, across) = (f.axis, 1 - f.axis);
        let (mut jogs, mut low, mut high, mut runs) = ([0; 2], [0; 2], [0; 2], [0; 4]);
        let mut nets: [String; 2] = Default::default();
        let mut objects = BTreeSet::new();
        let mut reached: [BTreeSet<usize>; 2] = Default::default();
        for (k, &(w, j)) in f.jogs.iter().enumerate() {
            let path = &self.paths[w].vertices;
            let (q, r) = (path[j], path[j + 1]);
            let up = self.vertices[path[j - 1]].at[axis] < self.vertices[q].at[axis];
            let (before, after) = (first_seg[w] + j - 1, first_seg[w] + j + 1);
            jogs[k] = first_seg[w] + j;
            (low[k], high[k], runs[k], runs[2 + k]) = if up { (q, r, before, after) } else { (r, q, after, before) };
            nets[k] = problem.routes.get(routing.wires[w].route).map(|x| x.net.clone()).unwrap_or_default();
            reached[k] = self.tree_objects.get(&self.paths[w].tree).cloned().unwrap_or_default();
            objects.extend(reached[k].iter().copied());
            if let Some(r) = problem.routes.get(routing.wires[w].route) {
                reached[k].extend(r.pins.iter().filter_map(|p| match p {
                    PointRef::Pin { object, .. } | PointRef::SymbolPin { object, .. } => Some(*object),
                    _ => None,
                }));
            }
        }
        let pair = s
            .pairs
            .iter()
            .filter(|_| axis == 0)
            .find(|p| {
                (reached[0].contains(&p.a) && reached[1].contains(&p.b))
                    || (reached[1].contains(&p.a) && reached[0].contains(&p.b))
            })
            .and_then(|p| match p.axis.and_then(|k| self.origin.get(k).copied().flatten()) {
                Some(v) => Some(super::cross::Mid::Axis(v)),
                None => {
                    Some(super::cross::Mid::Between(self.origin.get(p.a).copied()??, self.origin.get(p.b).copied()??))
                }
            });
        let term = |sk: &Skeleton, v: usize, a: usize| sk.vertices[v].term[a].clone();
        let mid = |sk: &Skeleton, a: usize| (sk.at(a, &term(sk, low[0], a)) + sk.at(a, &term(sk, low[1], a))) / int(2);
        let at = [mid(self, 0), mid(self, 1)];
        let centre = self.new_var(at);
        let (upper, lower) = (term(self, low[0], across), term(self, low[1], across));
        self.made[across].push(Made::AtLeast(upper.var, upper.off, lower.var, lower.off, Q::one(), Why::Wire));
        for (r, &seg) in runs.iter().enumerate() {
            let corner = if r < 2 { low[r] } else { high[r - 2] };
            let outer = if self.segs[seg].a == corner { self.segs[seg].b } else { self.segs[seg].a };
            let (c, o) = (term(self, corner, axis), term(self, outer, axis));
            let (hi, lo) = if r < 2 { (c, o) } else { (o, c) };
            self.made[axis].push(Made::AtLeast(hi.var, hi.off, lo.var, lo.off, Q::one(), Why::Wire));
        }
        self.crosses.push(Cross { axis, jogs, low, high, runs, centre, objects, nets, pair });
    }

    pub(crate) fn jog_of(&self, seg: usize) -> Option<(&Cross, usize)> {
        self.crosses.iter().find_map(|c| c.jogs.iter().position(|&j| j == seg).map(|k| (c, k)))
    }

    pub(crate) fn corner(&self, v: usize) -> bool {
        self.crosses.iter().any(|c| c.low.contains(&v) || c.high.contains(&v))
    }

    fn texts_own(&mut self, problem: &LevelProblem, s: &Structure, g: &Geometry) {
        let mut reached: [BTreeSet<usize>; 2] = [BTreeSet::new(), BTreeSet::new()];
        for (axis, reached) in reached.iter_mut().enumerate() {
            for m in &self.made[axis] {
                if let Made::Bound(e, _, _, Why::Fact(_)) = m {
                    reached.extend(e.terms().iter().map(|(v, _)| *v));
                }
            }
            if self.strict {
                for m in &self.made[axis] {
                    if let Made::Bound(e, _, _, Why::Body) = m {
                        if e.terms().iter().any(|(v, _)| reached.contains(v)) {
                            let vars: Vec<usize> = e.terms().iter().map(|(v, _)| *v).collect();
                            reached.extend(vars);
                        }
                    }
                }
            }
        }
        let components: Vec<usize> = s.components().collect();
        let on_wires: BTreeSet<usize> =
            self.vertices.iter().filter_map(|v| if let Key::Point(k) = v.key { Some(k) } else { None }).collect();
        for k in 0..problem.objects.len() {
            if !self.meter(self.vars) {
                return;
            }
            let o = &s.objects[k];
            if o.role != Role::Point || !o.own || o.left_free || on_wires.contains(&k) || !self.reads(k) {
                continue;
            }
            let Some(var) = self.origin[k] else { continue };
            let here = [self.value[0][var].clone(), self.value[1][var].clone()];
            let at_vertex = self.vertices.iter().find(|v| v.at == here).map(|v| v.term.clone());
            let at_pin = || {
                self.pins.iter().find(|(_, t)| [self.at(0, &t[0]), self.at(1, &t[1])] == here).map(|(_, t)| t.clone())
            };
            let nearest = || {
                components.iter().copied().filter_map(|c| self.origin[c].map(|v| (c, v))).min_by_key(|&(c, v)| {
                    let d = |a: usize| (&self.value[a][v] - &here[a]).abs();
                    (d(0) + d(1), c)
                })
            };
            let base = at_vertex
                .or_else(at_pin)
                .or_else(|| nearest().map(|(_, v)| [Term::of(v, Q::zero()), Term::of(v, Q::zero())]));
            let Some(base) = base else { continue };
            for axis in 0..2 {
                if reached[axis].contains(&var) || base[axis].var == var {
                    continue;
                }
                let off = &here[axis] - &self.at(axis, &base[axis]) + &base[axis].off;
                self.made[axis].push(Made::Equal(var, Q::zero(), base[axis].var, off, Why::Body));
            }
        }
        for k in 0..problem.objects.len() {
            if !self.meter(self.vars) {
                return;
            }
            let o = &s.objects[k];
            if o.role != Role::Text || !o.own || o.left_free || !self.reads(k) {
                continue;
            }
            let Some(var) = self.origin[k] else { continue };
            let here = [self.value[0][var].clone(), self.value[1][var].clone()];
            if let Some(Filler::Net(net)) = problem.objects[k].text.as_ref().map(|t| &t.attach) {
                let near = self
                    .paths
                    .iter()
                    .filter(|p| p.nets.contains(net))
                    .flat_map(|p| p.vertices.iter().copied())
                    .min_by_key(|&v| {
                        let d = |a: usize| (&self.at(a, &self.vertices[v].term[a]) - &here[a]).abs();
                        (d(0) + d(1), v)
                    });
                if let Some(v) = near {
                    let base = self.vertices[v].term.clone();
                    for axis in 0..2 {
                        if reached[axis].contains(&var) || base[axis].var == var {
                            continue;
                        }
                        let off = &here[axis] - &self.at(axis, &base[axis]) + &base[axis].off;
                        self.made[axis].push(Made::Equal(var, Q::zero(), base[axis].var, off, Why::Body));
                    }
                    continue;
                }
            }
            let attached = s.anchors.get(&k).and_then(|a| a.from.first()).map(|(o, _)| *o);
            let nearest = || {
                components.iter().copied().filter(|&c| self.origin[c].is_some()).min_by_key(|&c| {
                    let d = |a: usize| (&g.origin[c][a] - &g.origin[k][a]).abs();
                    (d(0) + d(1), c)
                })
            };
            let Some(owner) = attached.filter(|&a| self.origin[a].is_some() && a != k).or_else(nearest) else {
                continue;
            };
            let base = self.origin[owner].expect("filtered");
            for (axis, reached) in reached.iter().enumerate() {
                if reached.contains(&var) {
                    continue;
                }
                let off = &self.value[axis][var] - &self.value[axis][base];
                self.made[axis].push(Made::Equal(var, Q::zero(), base, off, Why::Body));
            }
        }
    }

    fn features_of(&mut self, problem: &LevelProblem, s: &Structure, g: &Geometry) {
        let boxed = |var: usize, b: &[Q; 4]| Feature {
            kind: Kind::Symbol(0),
            lo: [Term::of(var, b[0].clone()), Term::of(var, b[1].clone())],
            hi: [Term::of(var, b[2].clone()), Term::of(var, b[3].clone())],
        };
        let mut features = Vec::new();
        for k in s.components().filter(|&k| self.reads(k)) {
            let (Some(var), Some(b)) = (self.origin[k], s.box_offset(k, self.orient[k])) else { continue };
            features.push(Feature { kind: Kind::Symbol(k), ..boxed(var, &b) });
        }
        for k in 0..problem.objects.len() {
            if !self.meter(self.vars) {
                return;
            }
            let Some(var) = self.origin[k] else { continue };
            if s.objects[k].left_free || !self.reads(k) {
                continue;
            }
            for b in s.text_offsets(problem, g, k) {
                features.push(Feature { kind: Kind::Text(k), ..boxed(var, &b) });
            }
        }
        for (i, seg) in self.segs.iter().enumerate() {
            if seg.along.is_none() && !seg.slant {
                continue;
            }
            let (a, b) = (&self.vertices[seg.a], &self.vertices[seg.b]);
            let ends = |axis: usize| {
                if a.at[axis] <= b.at[axis] {
                    (a.term[axis].clone(), b.term[axis].clone())
                } else {
                    (b.term[axis].clone(), a.term[axis].clone())
                }
            };
            let ((lx, hx), (ly, hy)) = (ends(0), ends(1));
            let mut f = Feature { kind: Kind::Segment(i), lo: [lx, ly], hi: [hx, hy] };
            if let Some((c, k)) = self.jog_of(i) {
                f.lo[c.axis] = self.vertices[c.low[k]].term[c.axis].clone();
                f.hi[c.axis] = self.vertices[c.high[k]].term[c.axis].clone();
            }
            features.push(f);
        }
        let solid: BTreeSet<usize> = s.components().collect();
        for &(k, pin) in s.connections.nets_of.keys() {
            if !solid.contains(&k) || !self.reads(k) {
                continue;
            }
            let (Some(var), Some(off)) = (self.origin[k], s.pin_offset(k, pin, self.orient[k])) else { continue };
            let term = [Term::of(var, off[0].clone()), Term::of(var, off[1].clone())];
            self.pins.push(((k, pin), term.clone()));
            features.push(Feature { kind: Kind::Pin(k, pin), lo: term.clone(), hi: term });
        }
        self.features = features;
    }

    fn rails_of(&mut self, s: &Structure) {
        let mut pairs: Vec<(Q, crate::structure::PinKey, crate::structure::PinKey)> = Vec::new();
        for role in [Role::Ground, Role::Supply] {
            let pins: Vec<((usize, usize), Q)> = self
                .pins
                .iter()
                .filter(|(p, _)| s.objects[p.0].role == role)
                .map(|(p, t)| (*p, self.at(1, &t[1])))
                .collect();
            let none = BTreeSet::new();
            let groups = |k: usize| self.groups_of.get(&k).unwrap_or(&none);
            for (i, (p, y)) in pins.iter().enumerate() {
                if !self.meter(pins.len()) {
                    return;
                }
                for (q, z) in &pins[i + 1..] {
                    let (a, b) = (groups(p.0), groups(q.0));
                    if a.is_disjoint(b) && !(a.is_empty() && b.is_empty()) {
                        continue;
                    }
                    pairs.push(((y - z).abs(), *p, *q));
                }
            }
        }
        pairs.sort();
        self.rails = pairs.into_iter().map(|(_, p, q)| (p, q)).collect();
    }

    fn groups_of_tree(&mut self, problem: &LevelProblem, s: &Structure) {
        let boxes: BTreeMap<usize, Feature> = self
            .features
            .iter()
            .filter_map(|f| match f.kind {
                Kind::Symbol(k) => Some((k, f.clone())),
                _ => None,
            })
            .collect();
        for (n, node) in s.tree.nodes.iter().enumerate().skip(1) {
            if !self.meter(node.members.len()) {
                return;
            }
            let mut members: BTreeSet<usize> = node.members.iter().copied().collect();
            if let NodeKind::Group(gi) = node.kind {
                members.extend(problem.groups.get(gi).into_iter().flat_map(|gr| gr.members.iter().copied()));
            }
            let hanging: Vec<usize> =
                members.iter().flat_map(|k| s.hanging.get(k).into_iter().flatten().copied()).collect();
            members.extend(hanging);
            members.retain(|k| boxes.contains_key(k));
            if members.is_empty() {
                continue;
            }
            let mut lo = [None::<Q>, None::<Q>];
            let mut hi = [None::<Q>, None::<Q>];
            for k in &members {
                let f = &boxes[k];
                for axis in 0..2 {
                    let (l, h) = (self.at(axis, &f.lo[axis]), self.at(axis, &f.hi[axis]));
                    if lo[axis].as_ref().is_none_or(|v| l < *v) {
                        lo[axis] = Some(l);
                    }
                    if hi[axis].as_ref().is_none_or(|v| h > *v) {
                        hi[axis] = Some(h);
                    }
                }
            }
            let at = |b: &[Option<Q>; 2]| [b[0].clone().unwrap_or_default(), b[1].clone().unwrap_or_default()];
            let (vlo, vhi) = (self.new_var(at(&lo)), self.new_var(at(&hi)));
            for k in &members {
                let f = &boxes[k];
                for axis in 0..2 {
                    let (fl, fh) = (&f.lo[axis], &f.hi[axis]);
                    self.made[axis].push(Made::AtLeast(fl.var, fl.off.clone(), vlo, Q::zero(), Q::zero(), Why::Group));
                    self.made[axis].push(Made::AtLeast(vhi, Q::zero(), fh.var, fh.off.clone(), Q::zero(), Why::Group));
                }
                self.groups_of.entry(*k).or_default().insert(n);
            }
            let index = self.groups.len();
            self.groups
                .push(GroupBox { members, group: if let NodeKind::Group(g) = node.kind { Some(g) } else { None } });
            self.features.push(Feature {
                kind: Kind::Group(index),
                lo: [Term::of(vlo, Q::zero()), Term::of(vlo, Q::zero())],
                hi: [Term::of(vhi, Q::zero()), Term::of(vhi, Q::zero())],
            });
        }
    }

    fn usages(&mut self, problem: &LevelProblem) {
        fn objects_of(problem: &LevelProblem, u: usize, depth: usize, out: &mut BTreeSet<usize>) {
            if depth > 16 || !crate::budget::permit(crate::budget::Step::Structure, 1) {
                return;
            }
            for role in &problem.usages[u].roles {
                for m in &role.members {
                    if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                        return;
                    }
                    match m {
                        Filler::Object(k) => {
                            out.insert(*k);
                        }
                        Filler::Terminal { object, .. } | Filler::Pin { object, .. } => {
                            out.insert(*object);
                        }
                        Filler::Usage(v) if *v < problem.usages.len() => objects_of(problem, *v, depth + 1, out),
                        _ => {}
                    }
                }
            }
        }
        for (u, usage) in problem.usages.iter().enumerate() {
            if !self.meter(1) {
                return;
            }
            if usage.object.is_some() {
                continue;
            }
            let mut objects = BTreeSet::new();
            objects_of(problem, u, 0, &mut objects);
            for k in objects {
                self.usages_of.entry(k).or_default().insert(u);
            }
        }
    }

    fn spots_of(&mut self) {
        let n = self.vertices.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(parent: &mut [usize], mut i: usize) -> usize {
            while parent[i] != i {
                parent[i] = parent[parent[i]];
                i = parent[i];
            }
            i
        }
        for seg in &self.segs {
            if !self.meter(1) {
                return;
            }
            if seg.along.is_none() && !seg.slant {
                let (ra, rb) = (find(&mut parent, seg.a), find(&mut parent, seg.b));
                parent[ra.max(rb)] = ra.min(rb);
            }
        }
        let mut index: BTreeMap<usize, usize> = BTreeMap::new();
        for v in 0..n {
            if !self.meter(1) {
                return;
            }
            let r = find(&mut parent, v);
            let next = index.len();
            let spot = *index.entry(r).or_insert(next);
            if spot == self.spots.len() {
                self.spots.push(Spot::default());
            }
            self.vertices[v].spot = spot;
            self.spots[spot].vertices.push(v);
            match self.vertices[v].key {
                Key::Pin(k, pin) => self.spots[spot].pins.push((k, pin)),
                Key::Branch(_) => self.spots[spot].point = true,
                Key::Point(_) | Key::Bend(..) | Key::PinPosition(..) | Key::Coordinate(..) => {}
            }
        }
        for (i, seg) in self.segs.iter().enumerate() {
            if !self.meter(1) {
                return;
            }
            if seg.along.is_some() || seg.slant {
                for v in [seg.a, seg.b] {
                    let spot = self.vertices[v].spot;
                    if !self.spots[spot].segs.contains(&i) {
                        self.spots[spot].segs.push(i);
                    }
                }
            }
        }
        for spot in &mut self.spots {
            let point = spot.vertices.iter().any(|&v| matches!(self.vertices[v].key, Key::Point(_)));
            if point && spot.segs.len() != 2 {
                spot.point = true;
            }
        }
    }

    pub(crate) fn other_end(&self, seg: usize, spot: usize) -> usize {
        let s = &self.segs[seg];
        if self.vertices[s.a].spot == spot {
            s.b
        } else {
            s.a
        }
    }

    pub(crate) fn end_at(&self, seg: usize, spot: usize) -> usize {
        let s = &self.segs[seg];
        if self.vertices[s.a].spot == spot {
            s.a
        } else {
            s.b
        }
    }

    pub(crate) fn way_from(&self, seg: usize, spot: usize) -> i64 {
        let Some(axis) = self.segs[seg].along else { return 0 };
        let from = &self.vertices[self.end_at(seg, spot)].at[axis];
        let to = &self.vertices[self.other_end(seg, spot)].at[axis];
        if to > from {
            1
        } else if to < from {
            -1
        } else {
            0
        }
    }

    pub(crate) fn bend_beyond(&self, seg: usize, spot: usize) -> Option<usize> {
        let sp = &self.spots[spot];
        if !sp.pins.is_empty() || sp.point || sp.segs.len() != 2 {
            return None;
        }
        let other = *sp.segs.iter().find(|&&o| o != seg)?;
        let (a, b) = (self.segs[seg].along?, self.segs[other].along?);
        (a != b).then_some(other)
    }

    pub(crate) fn z_middle(&self, seg: usize) -> bool {
        let s = &self.segs[seg];
        let (sa, sb) = (self.vertices[s.a].spot, self.vertices[s.b].spot);
        let (Some(oa), Some(ob)) = (self.bend_beyond(seg, sa), self.bend_beyond(seg, sb)) else { return false };
        let into = -self.way_from(oa, sa);
        let out = self.way_from(ob, sb);
        into != 0 && into == out
    }

    pub(crate) fn collapses(&self, seg: usize) -> Option<usize> {
        let s = &self.segs[seg];
        for (pin_end, other) in [(s.a, s.b), (s.b, s.a)] {
            let sp = self.vertices[pin_end].spot;
            let spot = &self.spots[sp];
            if spot.pins.len() != 1 || spot.point || spot.segs.len() != 1 {
                continue;
            }
            let Some(exit) = self.exit_of(spot.pins[0]) else { continue };
            let way = self.way_from(seg, sp);
            let along = s.along?;
            let unit = if along == 0 { (way, 0) } else { (0, way) };
            if unit == exit {
                continue;
            }
            let bend = self.vertices[other].spot;
            let Some(next) = self.bend_beyond(seg, bend) else { continue };
            let w = self.way_from(next, bend);
            let next_unit = if self.segs[next].along == Some(0) { (w, 0) } else { (0, w) };
            if next_unit == exit {
                return Some(sp);
            }
        }
        None
    }

    pub(crate) fn side(&self, seg: usize, spot: usize) -> i64 {
        let sp = &self.spots[spot];
        if !sp.pins.is_empty() {
            return 0;
        }
        let along = self.segs[seg].along;
        let ways: BTreeSet<i64> = sp
            .segs
            .iter()
            .filter(|&&o| o != seg && self.segs[o].along.is_some() && self.segs[o].along != along)
            .map(|&o| self.way_from(o, spot))
            .collect();
        match ways.len() {
            1 => ways.into_iter().next().unwrap_or(0),
            _ => 0,
        }
    }

    pub(crate) fn lead_or_floor(&self, seg: usize) -> bool {
        let s = &self.segs[seg];
        let Some(along) = s.along else { return false };
        for v in [s.a, s.b] {
            let spot = &self.spots[self.vertices[v].spot];
            if spot.pins.iter().any(|&p| self.exit_of(p).is_some_and(|e| if along == 0 { e.0 != 0 } else { e.1 != 0 }))
            {
                return true;
            }
        }
        let (sa, sb) = (self.side(seg, self.vertices[s.a].spot), self.side(seg, self.vertices[s.b].spot));
        sa != 0 && sa == sb
    }

    pub(crate) fn lead_to_branch(&self, seg: usize) -> bool {
        let s = &self.segs[seg];
        let Some(along) = s.along else { return false };
        let leads = |v: usize| {
            let spot = &self.spots[self.vertices[v].spot];
            spot.pins.iter().any(|&p| self.exit_of(p).is_some_and(|e| if along == 0 { e.0 != 0 } else { e.1 != 0 }))
        };
        let branch = |v: usize| self.spots[self.vertices[v].spot].segs.len() >= 3;
        let (a, b) = (s.a, s.b);
        self.vertices[a].spot != self.vertices[b].spot && ((leads(a) && branch(b)) || (leads(b) && branch(a)))
    }

    pub(crate) fn saves(&self, seg: usize) -> u8 {
        if self.collapses(seg).is_some() || self.z_middle(seg) {
            return 2;
        }
        let s = &self.segs[seg];
        if s.along.is_none() || self.lead_or_floor(seg) {
            return 0;
        }
        let (pa, pb) = (self.vertices[s.a].spot, self.vertices[s.b].spot);
        let branch = |sp: usize| {
            let spot = &self.spots[sp];
            spot.pins.is_empty() && spot.segs.len() >= 3
        };
        let bend = |sp: usize| self.bend_beyond(seg, sp).is_some();
        let (sa, sb) = (self.side(seg, pa), self.side(seg, pb));
        if sa * sb == -1 && ((bend(pa) && branch(pb)) || (bend(pb) && branch(pa))) {
            1
        } else {
            0
        }
    }

    pub(crate) fn pin_in_tree(&self, pin: (usize, usize), tree: usize) -> bool {
        self.vertices.iter().any(|v| v.key == Key::Pin(pin.0, pin.1) && v.trees.contains(&tree))
    }

    fn exit_of(&self, pin: (usize, usize)) -> Option<(i64, i64)> {
        self.exits.get(&pin).copied()
    }

    pub(crate) fn at_object(&self, seg: usize, k: usize) -> bool {
        let s = &self.segs[seg];
        [s.a, s.b].iter().any(|&v| self.spots[self.vertices[v].spot].pins.iter().any(|(o, _)| *o == k))
    }

    pub(crate) fn at_pin(&self, seg: usize, pin: (usize, usize)) -> bool {
        let s = &self.segs[seg];
        [s.a, s.b].iter().any(|&v| self.spots[self.vertices[v].spot].pins.contains(&pin))
    }

    pub(crate) fn bends(&self, none: &BTreeSet<usize>) -> usize {
        let mut count = 0;
        for (sp, spot) in self.spots.iter().enumerate() {
            let live: Vec<usize> = spot.segs.iter().copied().filter(|g| !none.contains(g)).collect();
            let corner = spot.vertices.iter().any(|&v| self.corner(v));
            if spot.pins.is_empty() && !spot.point && live.len() == 2 && !corner {
                let (a, b) = (self.segs[live[0]].along, self.segs[live[1]].along);
                if a.is_some() && b.is_some() && a != b {
                    count += 1;
                }
            }
            if let ([pin], [seg]) = (spot.pins.as_slice(), live.as_slice()) {
                if let (Some(exit), Some(axis)) = (self.exit_of(*pin), self.segs[*seg].along) {
                    let w = self.way_from(*seg, sp);
                    let unit = if axis == 0 { (w, 0) } else { (0, w) };
                    if unit != exit {
                        count += 1;
                    }
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{fact, instance, label, problem_of};
    use crate::{SolverOptions, Work};
    use rnl_runtime::problem::{FactKind, Reference};
    use rnl_runtime::solution::RoutedWire;

    fn structure(p: &LevelProblem) -> Structure {
        let o = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(p, &o, &crate::StoppedClock);
        crate::structure::analyse(p, &resolved, &o, crate::options::CAPS, &mut Work::default())
    }

    #[test]
    fn coordinate_pin_anchors_have_no_electrical_spot_or_lead() {
        let mut p = problem_of(vec![instance()], vec![]);
        p.version = "0.4".into();
        let s = structure(&p);
        let electrical = PointRef::SymbolPin { object: 0, pin: 0, reference: Reference::Anonymous };
        let coordinate = PointRef::PinPosition { object: 0, pin: 0, reference: Reference::Anonymous };
        let mut routing = Routing::default();
        for (i, reference) in [electrical, coordinate.clone(), coordinate].into_iter().enumerate() {
            let end = (-3., 5. * (i + 1) as f64);
            routing
                .wires
                .push(RoutedWire { route: i, points: vec![WirePoint::Anchor(reference), WirePoint::Bend(end)] });
            routing.paths.push(vec![[int(-3), int(0)], [int(-3), int(5 * (i + 1) as i64)]]);
            routing.drawn.push(i);
        }
        let sk = read(&p, &s, &s.witness, &routing, &|_| true, false, false, None, false).unwrap();
        let at: Vec<_> = sk.vertices.iter().filter(|v| matches!(v.key, Key::Pin(..) | Key::PinPosition(..))).collect();
        assert_eq!(at.len(), 3);
        assert_eq!(at.iter().map(|v| v.spot).collect::<BTreeSet<_>>().len(), 3);
        assert_eq!(sk.spots[at[0].spot].pins, vec![(0, 0)]);
        assert!(at[1..].iter().all(|v| sk.spots[v.spot].pins.is_empty()));
        assert_eq!(at[0].term, at[1].term);
    }

    #[test]
    fn body_links_use_the_alternate_text_column_position() {
        let mut p =
            problem_of(vec![instance(), label(None)], vec![fact(FactKind::Orient { object: 0, code: Orient::MX })]);
        p.version = "0.4".into();
        let mut s = structure(&p);
        let id = s.model.point(&p, &PointRef::Origin(1), 0).unwrap();
        let info = s.model.points.get_mut(&id).unwrap();
        info.alternate_local = Some((int(3), int(5)));
        let var = info.var;
        let sk = read(&p, &s, &s.witness, &Routing::default(), &|_| true, false, false, None, false).unwrap();
        for (axis, expected) in [int(3), int(-5)].into_iter().enumerate() {
            assert!(sk.made[axis]
                .iter()
                .any(|m| matches!(m, Made::Equal(v, _, _, off, Why::Body) if *v == var && *off == expected)));
        }
    }

    #[test]
    fn interrupted_read_never_returns_partial_fact_constraints() {
        let mut p = problem_of(vec![instance()], vec![]);
        p.version = "0.4".into();
        let s = structure(&p);
        let (sk, budget) = crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(0), None, || {
            read(&p, &s, &s.witness, &Routing::default(), &|_| true, false, false, None, false)
        });
        assert!(sk.is_none());
        assert_eq!(budget.work, 0);
    }

    #[test]
    fn authored_wire_modes_read_the_same_path_as_the_renderer() {
        use rnl_runtime::problem::ProblemWire;
        for mode in ["straight", "auto", "horizontal-vertical", "vertical-horizontal", "manhattan", "manhattan45"] {
            for end in [[12, 4], [4, 12], [-12, 4], [4, -12], [6, 6], [6, 0]] {
                let mut p = problem_of(vec![instance(), instance()], vec![]);
                p.version = "0.4".into();
                p.wires.push(ProblemWire {
                    label: "wire".into(),
                    reference: Reference::Anonymous,
                    nets: vec![],
                    bus: false,
                    anchors: vec![PointRef::Origin(0), PointRef::Origin(1)],
                    mode: mode.into(),
                    generated: false,
                });
                let s = structure(&p);
                let mut g = s.witness.clone();
                g.origin[0] = [int(0), int(0)];
                g.origin[1] = end.map(int);
                let sk = read(&p, &s, &g, &Routing::default(), &|_| true, false, false, None, false).unwrap();
                let actual: Vec<_> = sk.paths[0].vertices.iter().map(|&v| sk.vertices[v].at.clone()).collect();
                assert_eq!(actual, crate::route::written_path(&p, &g, 0).unwrap(), "{mode} {end:?}");
            }
        }
    }

    #[test]
    fn authored_centre_anchor_uses_the_model_centre_and_keeps_the_path() {
        use rnl_runtime::problem::ProblemWire;
        let mut p = problem_of(vec![instance(), instance(), instance()], vec![]);
        p.version = "0.4".into();
        p.wires.push(ProblemWire {
            label: "wire".into(),
            reference: Reference::Anonymous,
            nets: vec![],
            bus: false,
            anchors: vec![
                PointRef::Origin(0),
                PointRef::Centre(vec![PointRef::Origin(1), PointRef::Origin(2)]),
                PointRef::Origin(2),
            ],
            mode: "manhattan".into(),
            generated: false,
        });
        let s = structure(&p);
        let mut g = s.witness.clone();
        for (k, point) in [[0, 0], [6, 12], [18, 0]].into_iter().enumerate() {
            g.origin[k] = point.map(int);
        }
        let sk = read(&p, &s, &g, &Routing::default(), &|_| true, false, false, None, false).unwrap();
        let actual: Vec<_> = sk.paths[0].vertices.iter().map(|&v| sk.vertices[v].at.clone()).collect();
        assert_eq!(actual, crate::route::written_path(&p, &g, 0).unwrap());
        let centre = sk.vertices.iter().find(|v| v.key == Key::Coordinate(0, 1)).unwrap();
        assert_eq!(centre.term[0].var, s.model.var(s.model.wire_points[0][1].unwrap()));
        assert!(sk.spots[centre.spot].pins.is_empty());
    }

    #[test]
    fn an_unheld_retained_choice_is_not_silently_omitted() {
        use rnl_runtime::problem::{Axis, Sense};
        let mut p = problem_of(
            vec![instance(), instance()],
            vec![fact(FactKind::Distance {
                axis: Axis::X,
                sense: Sense::Either,
                min: Some(8.),
                max: None,
                from: vec![PointRef::Origin(0)],
                to: vec![PointRef::Origin(1)],
            })],
        );
        p.version = "0.4".into();
        let s = structure(&p);
        assert_eq!(s.kept, vec![0]);
        assert!(!s.model.units[0].choices.is_empty());
        let mut g = s.witness.clone();
        g.origin[1] = g.origin[0].clone();
        assert!(read(&p, &s, &g, &Routing::default(), &|_| true, false, false, None, false).is_none());
        assert!(read(&p, &s, &g, &Routing::default(), &|_| false, false, false, None, false).is_some());
    }
}
