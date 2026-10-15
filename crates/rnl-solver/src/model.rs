//! The problem as variables and linear forms, axis by axis (plan 5.1; standard 11.1, 14.1–14.5,
//! 6.5): a variable for the origin of each object with a position of its own, for each pin that a
//! fact, a wire or a ROUTE reaches, for each centre of a set and for the extents of sets; each
//! fact as bounds on linear forms, some of them chosen by a literal. Objects drawn at the point of
//! their main object (12.1) have no variable of their own: their origin is that point. Texts placed
//! by their symbol (13.3) are points of their owner, as pins are.
//!
//! **Points.** A point is the origin of an object with a position of its own, a pin, a text placed
//! by its symbol, or the centre of a set ([`PointId`]). A pin is the origin of its object plus its
//! position in the symbol turned by the object's orientation (9.2.4, 11.2): on each axis a
//! constant that the orientation decides. A pin's variable is linked to its origin's,
//! `x_pin − x_origin = c(orientation)`, only once the orientation matters (plan 5.1): when facts
//! reach two points of one object on one axis, the origin counted as one. With one point reached,
//! the origin, which nothing else holds, takes up any constant, and the orientation cannot make a
//! fact fail. The links are the resolution's to assert ([`crate::resolve`]), with the orientation
//! as a literal.
//!
//! **Facts.** Each fact of the problem is one unit of 8.5 ([`Unit`]): bounds that hold whatever
//! is chosen (`fixed`), and choices, each a literal whose value picks one list of bounds among
//! several ([`Choice`]), for what is a disjunction:
//!
//! - PLACE, a coordinate of a point and the `dx` or `dy` of a text: `x = v`; ALIGN: equal members;
//!   DISTRIBUTE: equal steps; ORIENT and ORIENT-RELATION constrain no coordinate
//!   ([`OrientEffect`]).
//! - ORDER (14.3): neighbours at least one unit apart, increasing or decreasing; given only an
//!   axis, the direction of the whole chain is a literal.
//! - DISTANCE (14.5) between the extents of two sets of points, `A` (`from`) and `B` (`to`): gap 0
//!   is `min(B) − max(A)`, gap 1 is `min(A) − max(B)`; an increasing direction bounds gap 0, a
//!   decreasing one gap 1, and `"x"` or `"y"` their maximum, so a lower bound picks a side with a
//!   literal and an upper bound bounds both gaps. A lower bound `min(B) − max(A) ≥ m` holds for
//!   every pair, `b − a ≥ m`, written with a separator `t` as `a ≤ t` and `b ≥ t + m` when both
//!   sides have several points; an upper bound `min(B) − max(A) ≤ M` asks for one pair close
//!   enough, a literal over the pairs.
//! - The centre of a set (6.5, 14.1) is the middle of the extent of its points: `c = p` for one,
//!   `2c = p₁ + p₂` for two; for three or more, `lo ≤ pᵢ ≤ hi` for each point and `2c = lo + hi`,
//!   which always hold ([`Centre`]), and two literals, which point is the lowest and which the
//!   highest, `lo = pⱼ`, `hi = pₖ`.
//!
//! **Tolerance** (11.7; plan section 4): only the constant bounds of the facts are widened, `x = v`
//! becoming `v − tol ≤ x ≤ v + tol`, the `min` of DISTANCE lowered and its `max` raised by it, and
//! so the one unit of ORDER; equalities between variables, ALIGN, DISTRIBUTE and the centres, stay
//! exact.
//!
//! Non-overlap and the grid are not facts (11.7) and are not here.

use std::collections::BTreeMap;

use num_traits::{One, Zero};
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{Axis, FactKind, LevelProblem, PointRef, Position, Sense};

use crate::rational::{from_f64, int, Q};
use crate::simplex::{LinExpr, Var};

/// The index of an axis: x 0, y 1.
pub(crate) fn ax(a: Axis) -> usize {
    match a {
        Axis::X => 0,
        Axis::Y => 1,
    }
}

/// A local position turned by an orientation, on one axis, exactly (11.2).
pub(crate) fn turned(o: Orient, local: &(Q, Q), axis: usize) -> Q {
    let (a, b, c, d) = o.matrix();
    let (p, q) = if axis == 0 { (a, b) } else { (c, d) };
    int(i64::from(p)) * &local.0 + int(i64::from(q)) * &local.1
}

/// A point that facts, wires and ROUTEs reach, with a variable on each axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum PointId {
    /// The origin of an object with a position of its own (11.1), by object.
    Origin(usize),
    /// A pin of an object, by object and by the index of the pin in its symbol (9.2.4).
    Pin(usize, usize),
    /// A text placed by its owner's symbol (13.3), by object: a point of the owner.
    Text(usize),
    /// The centre of a set (6.5, 14.1), by its index in [`Model::centres`].
    Centre(usize),
}

/// What a variable stands for. Every variable exists on both axes, with the same index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum VarKind {
    Point(PointId),
    /// The lowest point of a centre's set, by centre.
    CentreLo(usize),
    /// The highest point of a centre's set, by centre.
    CentreHi(usize),
    /// The separator of a lower bound of DISTANCE between sets: the points of `before` are at
    /// most it, those of the other side at least it plus the bound.
    Separator {
        unit: usize,
        gap: u8,
        before: Vec<PointId>,
    },
}

/// A point and its variable.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PointInfo {
    pub var: Var,
    /// The object it is a point of, whose orientation turns it: the object itself for an origin,
    /// the owner for a pin or a text; none for a centre.
    pub object: Option<usize>,
    /// For a pin or a text: its position in the object's symbol (9.2.4, 13.3).
    pub local: Option<(Q, Q)>,
}

/// The centre of a set of points (6.5, 14.1): the middle of their extent.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Centre {
    /// Its points, sets followed down to their members, sorted, without repeats.
    pub members: Vec<PointId>,
    pub var: Var,
    pub lo: Var,
    pub hi: Var,
}

/// `lo ≤ expr ≤ hi` on one axis, either side optional.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Bound {
    pub axis: usize,
    pub expr: LinExpr,
    pub lo: Option<Q>,
    pub hi: Option<Q>,
}

impl Bound {
    fn at_least(axis: usize, expr: LinExpr, lo: Q) -> Bound {
        Bound { axis, expr, lo: Some(lo), hi: None }
    }

    fn at_most(axis: usize, expr: LinExpr, hi: Q) -> Bound {
        Bound { axis, expr, lo: None, hi: Some(hi) }
    }

    fn within(axis: usize, expr: LinExpr, lo: Q, hi: Q) -> Bound {
        Bound { axis, expr, lo: Some(lo), hi: Some(hi) }
    }

    fn equal(axis: usize, expr: LinExpr, v: Q) -> Bound {
        Bound { axis, expr, lo: Some(v.clone()), hi: Some(v) }
    }

    /// Whether it holds with each variable given the value `value` gives it.
    pub(crate) fn holds(&self, value: impl FnMut(Var) -> Q) -> bool {
        let v = self.expr.eval(value);
        self.lo.as_ref().is_none_or(|lo| v >= *lo) && self.hi.as_ref().is_none_or(|hi| v <= *hi)
    }
}

/// A literal of the search (plan 5.3): a choice that a disjunction leaves open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum LitKey {
    /// The direction of an ORDER given only an axis, by unit: 0 increasing, 1 decreasing.
    Order(usize),
    /// The side of a DISTANCE given only an axis with a lower bound, by unit: 0 `to` after
    /// `from`, 1 `from` after `to`.
    Side(usize),
    /// The pair of points that keeps a gap of a DISTANCE within its upper bound, by unit and gap.
    Pair(usize, u8),
    /// The lowest point of a centre's set on an axis, by centre and axis.
    CentreLo(usize, u8),
    /// The highest point of a centre's set on an axis, by centre and axis.
    CentreHi(usize, u8),
    /// The orientation of a class, by its root object.
    Class(usize),
}

/// A literal that a unit owns: one list of bounds per value.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Choice {
    pub key: LitKey,
    pub kind: ChoiceKind,
    pub values: Vec<Vec<Bound>>,
}

/// What a choice chooses, to read its value off a witness.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ChoiceKind {
    /// ORDER given only an axis: 0 increasing, 1 decreasing.
    Order { axis: usize, members: Vec<PointId> },
    /// The side of a DISTANCE given only an axis: 0 gap 0, `to` after `from`; 1 gap 1.
    Side { axis: usize, from: Vec<PointId>, to: Vec<PointId> },
    /// Which pair `(a, b)`, `a` of `from` and `b` of `to`, keeps a gap within the upper bound: one
    /// value per pair, in this order.
    Pair { axis: usize, gap: u8, pairs: Vec<(PointId, PointId)> },
}

/// What a unit says of orientations (11.2, 14.9).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum OrientEffect {
    None,
    /// ORIENT: the object has this orientation.
    Fix {
        object: usize,
        code: Orient,
    },
    /// ORIENT-RELATION: the members of each side share one orientation, and `to` is `from` then
    /// `relation`.
    Relate {
        relation: Orient,
        from: Vec<usize>,
        to: Vec<usize>,
    },
}

/// One unit of 8.5: a fact, as bounds and choices.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Unit {
    /// The bounds that hold whatever is chosen.
    pub fixed: Vec<Bound>,
    pub choices: Vec<Choice>,
    pub orient: OrientEffect,
    /// The points it constrains, by axis, a centre with the points of its set.
    pub touched: [Vec<PointId>; 2],
    /// The centres of three or more points it constrains, by axis: their literals are open while
    /// it is in.
    pub centres: [Vec<usize>; 2],
}

/// The variables of a problem and its facts as units.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Model {
    /// The tolerance (11.7).
    pub tol: Q,
    pub vars: Vec<VarKind>,
    pub points: BTreeMap<PointId, PointInfo>,
    pub centres: Vec<Centre>,
    centre_index: BTreeMap<Vec<PointId>, usize>,
    /// One per fact of the problem, by the same index.
    pub units: Vec<Unit>,
    /// For each object, the point that is its origin: its own for an object with a position of
    /// its own, the point of its main object for one drawn there (12.1); none for a text placed by
    /// its symbol.
    pub origin_point: Vec<Option<PointId>>,
    /// For each object, whether it has an orientation (11.2).
    pub oriented: Vec<bool>,
    /// For each object, whether it has a position of its own.
    pub own: Vec<bool>,
    /// For each object, its points that the model has: its origin, pins and texts (not the point
    /// of the main object it is drawn at).
    pub object_points: Vec<Vec<PointId>>,
    /// The points of each wire, for the stages that route (13.1).
    pub wire_points: Vec<Vec<Option<PointId>>>,
    /// The pins of each ROUTE (13.7).
    pub route_points: Vec<Vec<Option<PointId>>>,
}

/// How deep the main objects of objects drawn at a point may nest.
const MAX_DEPTH: usize = 16;

impl Model {
    /// The model of a problem, with the tolerance `tol` (11.7).
    pub(crate) fn new(problem: &LevelProblem, tol: Q) -> Model {
        let n = problem.objects.len();
        let mut m = Model {
            tol,
            vars: Vec::new(),
            points: BTreeMap::new(),
            centres: Vec::new(),
            centre_index: BTreeMap::new(),
            units: Vec::new(),
            origin_point: vec![None; n],
            oriented: problem.objects.iter().map(|o| o.orient.is_some()).collect(),
            own: problem.objects.iter().map(|o| matches!(o.position, Position::Own { .. })).collect(),
            object_points: vec![Vec::new(); n],
            wire_points: Vec::new(),
            route_points: Vec::new(),
        };
        for k in 0..n {
            if m.own[k] {
                m.add_point(PointId::Origin(k), Some(k), None);
                m.origin_point[k] = Some(PointId::Origin(k));
            }
        }
        for k in 0..n {
            if let Position::At(main) = &problem.objects[k].position {
                m.origin_point[k] = m.point(problem, main, 0);
            }
        }
        for (i, fact) in problem.facts.iter().enumerate() {
            let unit = m.encode(problem, i, &fact.kind);
            m.units.push(unit);
        }
        for w in &problem.wires {
            let points = w.anchors.iter().map(|r| m.point(problem, r, 0)).collect();
            m.wire_points.push(points);
        }
        for r in &problem.routes {
            let points = r.pins.iter().map(|p| m.point(problem, p, 0)).collect();
            m.route_points.push(points);
        }
        for (id, info) in &m.points {
            if let Some(k) = info.object {
                m.object_points[k].push(*id);
            }
        }
        m
    }

    /// The variable of a point.
    pub(crate) fn var(&self, p: PointId) -> Var {
        self.points[&p].var
    }

    fn new_var(&mut self, kind: VarKind) -> Var {
        self.vars.push(kind);
        self.vars.len() - 1
    }

    fn add_point(&mut self, id: PointId, object: Option<usize>, local: Option<(Q, Q)>) -> PointId {
        if !self.points.contains_key(&id) {
            let var = self.new_var(VarKind::Point(id));
            self.points.insert(id, PointInfo { var, object, local });
        }
        id
    }

    /// The point a reference designates, its variable made the first time; none for a reference
    /// to nothing the model has, such as an object out of range.
    pub(crate) fn point(&mut self, problem: &LevelProblem, r: &PointRef, depth: usize) -> Option<PointId> {
        if depth > MAX_DEPTH {
            return None;
        }
        match r {
            PointRef::Origin(k) => match &problem.objects.get(*k)?.position {
                Position::Own { .. } => Some(PointId::Origin(*k)),
                Position::At(main) => self.point(problem, main, depth + 1),
                Position::InSymbol { owner, local } => {
                    let local = (from_f64(local.0).unwrap_or_default(), from_f64(local.1).unwrap_or_default());
                    Some(self.add_point(PointId::Text(*k), Some(*owner), Some(local)))
                }
            },
            PointRef::Pin { object, pin, .. } => {
                let o = problem.objects.get(*object)?;
                let pos = o.symbol.and_then(|s| problem.symbols.get(s)).and_then(|s| s.pins.get(*pin)).map(|p| p.pos);
                let (x, y) = pos.unwrap_or((0.0, 0.0));
                let local = (from_f64(x).unwrap_or_default(), from_f64(y).unwrap_or_default());
                Some(self.add_point(PointId::Pin(*object, *pin), Some(*object), Some(local)))
            }
            PointRef::Centre(members) => {
                let mut points = Vec::new();
                self.flatten(problem, members, depth, &mut points);
                points.sort_unstable();
                points.dedup();
                if points.is_empty() {
                    return None;
                }
                Some(PointId::Centre(self.centre(points)))
            }
        }
    }

    /// The points of the members of a set, nested sets followed down to their members (14.1).
    fn flatten(&mut self, problem: &LevelProblem, members: &[PointRef], depth: usize, out: &mut Vec<PointId>) {
        for r in members {
            match r {
                PointRef::Centre(inner) if depth <= MAX_DEPTH => self.flatten(problem, inner, depth + 1, out),
                _ => out.extend(self.point(problem, r, depth + 1)),
            }
        }
    }

    /// The centre of a set of points, sorted and without repeats, made the first time.
    fn centre(&mut self, members: Vec<PointId>) -> usize {
        if let Some(&c) = self.centre_index.get(&members) {
            return c;
        }
        let c = self.centres.len();
        let id = PointId::Centre(c);
        let var = self.new_var(VarKind::Point(id));
        self.points.insert(id, PointInfo { var, object: None, local: None });
        let lo = self.new_var(VarKind::CentreLo(c));
        let hi = self.new_var(VarKind::CentreHi(c));
        self.centre_index.insert(members.clone(), c);
        self.centres.push(Centre { members, var, lo, hi });
        c
    }

    /// The bounds that make a centre's variable the middle of the extent of its set on an axis,
    /// with the literals of its lowest and highest points ([`Centre`]); they always hold.
    pub(crate) fn centre_definition(&self, c: usize, axis: usize) -> Vec<Bound> {
        let centre = &self.centres[c];
        let var = |p: &PointId| self.var(*p);
        match centre.members.as_slice() {
            [p] => vec![Bound::equal(axis, LinExpr::diff(centre.var, var(p)), Q::zero())],
            [p, q] => {
                let e =
                    LinExpr::from_terms([(centre.var, int(2)), (var(p), -Q::one()), (var(q), -Q::one())], Q::zero());
                vec![Bound::equal(axis, e, Q::zero())]
            }
            members => {
                let mut out = Vec::new();
                for p in members {
                    out.push(Bound::at_most(axis, LinExpr::diff(centre.lo, var(p)), Q::zero()));
                    out.push(Bound::at_least(axis, LinExpr::diff(centre.hi, var(p)), Q::zero()));
                }
                let e = LinExpr::from_terms(
                    [(centre.var, int(2)), (centre.lo, -Q::one()), (centre.hi, -Q::one())],
                    Q::zero(),
                );
                out.push(Bound::equal(axis, e, Q::zero()));
                out
            }
        }
    }

    /// The bound of a centre's literal: its `j`th point is the lowest (`high` false) or the highest.
    pub(crate) fn centre_extreme(&self, c: usize, axis: usize, high: bool, j: usize) -> Bound {
        let centre = &self.centres[c];
        let p = self.var(centre.members[j]);
        if high {
            Bound::at_most(axis, LinExpr::diff(centre.hi, p), Q::zero())
        } else {
            Bound::at_least(axis, LinExpr::diff(centre.lo, p), Q::zero())
        }
    }

    /// The points of a list of references, those the model cannot place left out.
    fn points_of(&mut self, problem: &LevelProblem, refs: &[PointRef]) -> Vec<PointId> {
        refs.iter().filter_map(|r| self.point(problem, r, 0)).collect()
    }

    /// The bounds that put each point of `after` at least `min` beyond each point of `before`
    /// (one gap of 14.5): one per pair when a side has a single point, otherwise through a
    /// separator.
    fn lower_gap(
        &mut self,
        unit: usize,
        gap: u8,
        axis: usize,
        before: &[PointId],
        after: &[PointId],
        min: &Q,
    ) -> Vec<Bound> {
        let mut out = Vec::new();
        if before.len() == 1 || after.len() == 1 {
            for a in before {
                for b in after {
                    out.push(Bound::at_least(axis, LinExpr::diff(self.var(*b), self.var(*a)), min.clone()));
                }
            }
            return out;
        }
        let t = self.new_var(VarKind::Separator { unit, gap, before: before.to_vec() });
        for a in before {
            out.push(Bound::at_most(axis, LinExpr::diff(self.var(*a), t), Q::zero()));
        }
        for b in after {
            out.push(Bound::at_least(axis, LinExpr::diff(self.var(*b), t), min.clone()));
        }
        out
    }

    /// The upper bound of one gap of 14.5, `min(after) − max(before) ≤ max`: a bound when each side
    /// has one point, otherwise a choice of the pair that is close enough.
    fn upper_gap(
        &self,
        unit: usize,
        gap: u8,
        axis: usize,
        before: &[PointId],
        after: &[PointId],
        max: &Q,
    ) -> (Vec<Bound>, Option<Choice>) {
        let mut pairs = Vec::new();
        let mut values = Vec::new();
        for a in before {
            for b in after {
                pairs.push(if gap == 0 { (*a, *b) } else { (*b, *a) });
                values.push(vec![Bound::at_most(axis, LinExpr::diff(self.var(*b), self.var(*a)), max.clone())]);
            }
        }
        if values.len() == 1 {
            (values.pop().unwrap_or_default(), None)
        } else {
            (
                Vec::new(),
                Some(Choice { key: LitKey::Pair(unit, gap), kind: ChoiceKind::Pair { axis, gap, pairs }, values }),
            )
        }
    }

    /// The unit of a fact (8.5).
    fn encode(&mut self, problem: &LevelProblem, unit: usize, kind: &FactKind) -> Unit {
        let tol = self.tol.clone();
        let mut fixed = Vec::new();
        let mut choices = Vec::new();
        let mut orient = OrientEffect::None;
        let mut on: Vec<(usize, PointId)> = Vec::new();
        let number = |v: f64| from_f64(v).unwrap_or_default();
        match kind {
            FactKind::Place { at, axis, value } => {
                let a = ax(*axis);
                if let Some(p) = self.point(problem, at, 0) {
                    let v = number(*value);
                    fixed.push(Bound::within(a, LinExpr::var(self.var(p)), &v - &tol, &v + &tol));
                    on.push((a, p));
                }
            }
            FactKind::PointCoordinate { point, axis, value } => {
                let a = ax(*axis);
                if let Some(p) = self.point(problem, &PointRef::Origin(*point), 0) {
                    let v = number(*value);
                    fixed.push(Bound::within(a, LinExpr::var(self.var(p)), &v - &tol, &v + &tol));
                    on.push((a, p));
                }
            }
            FactKind::Offset { text, from, axis, value } => {
                let a = ax(*axis);
                let t = self.point(problem, &PointRef::Origin(*text), 0);
                let f = self.point(problem, from, 0);
                if let (Some(t), Some(f)) = (t, f) {
                    let v = number(*value);
                    fixed.push(Bound::within(a, LinExpr::diff(self.var(t), self.var(f)), &v - &tol, &v + &tol));
                    on.extend([(a, t), (a, f)]);
                }
            }
            FactKind::Orient { object, code } => orient = OrientEffect::Fix { object: *object, code: *code },
            FactKind::OrientRelation { relation, from, to } => {
                orient = OrientEffect::Relate { relation: *relation, from: from.clone(), to: to.clone() }
            }
            FactKind::Align { axis, members } => {
                let a = ax(*axis);
                let ps = self.points_of(problem, members);
                for p in ps.iter().skip(1) {
                    fixed.push(Bound::equal(a, LinExpr::diff(self.var(*p), self.var(ps[0])), Q::zero()));
                }
                on.extend(ps.iter().map(|p| (a, *p)));
            }
            FactKind::Order { axis, sense, members } => {
                let a = ax(*axis);
                let ps = self.points_of(problem, members);
                if ps.len() >= 2 {
                    let step = Q::one() - &tol;
                    let chain = |m: &Model, up: bool| -> Vec<Bound> {
                        ps.windows(2)
                            .map(|w| {
                                let (lo, hi) = if up { (w[0], w[1]) } else { (w[1], w[0]) };
                                Bound::at_least(a, LinExpr::diff(m.var(hi), m.var(lo)), step.clone())
                            })
                            .collect()
                    };
                    match sense {
                        Sense::Increasing => fixed.extend(chain(self, true)),
                        Sense::Decreasing => fixed.extend(chain(self, false)),
                        Sense::Either => choices.push(Choice {
                            key: LitKey::Order(unit),
                            kind: ChoiceKind::Order { axis: a, members: ps.clone() },
                            values: vec![chain(self, true), chain(self, false)],
                        }),
                    }
                }
                on.extend(ps.iter().map(|p| (a, *p)));
            }
            FactKind::Distribute { axis, members } => {
                let a = ax(*axis);
                let ps = self.points_of(problem, members);
                for w in ps.windows(3) {
                    let e = LinExpr::from_terms(
                        [(self.var(w[2]), Q::one()), (self.var(w[1]), int(-2)), (self.var(w[0]), Q::one())],
                        Q::zero(),
                    );
                    fixed.push(Bound::equal(a, e, Q::zero()));
                }
                on.extend(ps.iter().map(|p| (a, *p)));
            }
            FactKind::Distance { axis, sense, min, max, from, to } => {
                let a = ax(*axis);
                let mut from_ps = self.points_of(problem, from);
                let mut to_ps = self.points_of(problem, to);
                // an extent is of a set of points: repeats change nothing
                for ps in [&mut from_ps, &mut to_ps] {
                    let mut seen = std::collections::BTreeSet::new();
                    ps.retain(|p| seen.insert(*p));
                }
                if !from_ps.is_empty() && !to_ps.is_empty() {
                    let min = min.map(|v| number(v) - &tol);
                    let max = max.map(|v| number(v) + &tol);
                    let (af, bt) = (from_ps.clone(), to_ps.clone());
                    match sense {
                        Sense::Increasing | Sense::Decreasing => {
                            let gap = u8::from(*sense == Sense::Decreasing);
                            let (before, after) = if gap == 0 { (&af, &bt) } else { (&bt, &af) };
                            if let Some(m) = &min {
                                fixed.extend(self.lower_gap(unit, gap, a, before, after, m));
                            }
                            if let Some(m) = &max {
                                let (bounds, choice) = self.upper_gap(unit, gap, a, before, after, m);
                                fixed.extend(bounds);
                                choices.extend(choice);
                            }
                        }
                        Sense::Either => {
                            if let Some(m) = &min {
                                // |b − a| ≥ m holds of two points whenever m ≤ 0
                                let trivial = af.len() == 1 && bt.len() == 1 && *m <= Q::zero();
                                if !trivial {
                                    let up = self.lower_gap(unit, 0, a, &af, &bt, m);
                                    let down = self.lower_gap(unit, 1, a, &bt, &af, m);
                                    choices.push(Choice {
                                        key: LitKey::Side(unit),
                                        kind: ChoiceKind::Side { axis: a, from: af.clone(), to: bt.clone() },
                                        values: vec![up, down],
                                    });
                                }
                            }
                            if let Some(m) = &max {
                                for (gap, before, after) in [(0, &af, &bt), (1, &bt, &af)] {
                                    let (bounds, choice) = self.upper_gap(unit, gap, a, before, after, m);
                                    fixed.extend(bounds);
                                    choices.extend(choice);
                                }
                            }
                        }
                    }
                }
                on.extend(from_ps.iter().chain(&to_ps).map(|p| (a, *p)));
            }
        }
        let mut touched: [Vec<PointId>; 2] = [Vec::new(), Vec::new()];
        let mut centres: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for (a, p) in on {
            touched[a].push(p);
            if let PointId::Centre(c) = p {
                touched[a].extend(self.centres[c].members.iter().copied());
                if self.centres[c].members.len() >= 3 {
                    centres[a].push(c);
                }
            }
        }
        for a in 0..2 {
            touched[a].sort_unstable();
            touched[a].dedup();
            centres[a].sort_unstable();
            centres[a].dedup();
        }
        Unit { fixed, choices, orient, touched, centres }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::frac;
    use crate::resolve::tests::{fact, instance, point, problem_of};
    use rnl_runtime::problem::Sense;

    fn tol() -> Q {
        frac(1, 1_000_000)
    }

    #[test]
    fn every_object_with_a_position_has_an_origin_and_pins_have_their_local_position() {
        let p = problem_of(
            vec![instance(), point()],
            vec![fact(FactKind::Align {
                axis: Axis::Y,
                members: vec![PointRef::Pin { object: 0, pin: 0, terminal: "g".into() }, PointRef::Origin(1)],
            })],
        );
        let m = Model::new(&p, tol());
        assert_eq!(m.origin_point, vec![Some(PointId::Origin(0)), Some(PointId::Origin(1))]);
        let pin = &m.points[&PointId::Pin(0, 0)];
        assert_eq!(pin.object, Some(0));
        assert_eq!(pin.local, Some((int(-3), int(0))));
        assert_eq!(m.units[0].touched[1], vec![PointId::Origin(1), PointId::Pin(0, 0)]);
        assert!(m.units[0].touched[0].is_empty());
    }

    #[test]
    fn constant_bounds_are_widened_and_equalities_stay_exact() {
        let p = problem_of(
            vec![point(), point()],
            vec![
                fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 3.0 }),
                fact(FactKind::Align { axis: Axis::X, members: vec![PointRef::Origin(0), PointRef::Origin(1)] }),
                fact(FactKind::Order {
                    axis: Axis::X,
                    sense: Sense::Increasing,
                    members: vec![PointRef::Origin(0), PointRef::Origin(1)],
                }),
            ],
        );
        let m = Model::new(&p, tol());
        let place = &m.units[0].fixed[0];
        assert_eq!((place.lo.clone(), place.hi.clone()), (Some(int(3) - tol()), Some(int(3) + tol())));
        let align = &m.units[1].fixed[0];
        assert_eq!((align.lo.clone(), align.hi.clone()), (Some(int(0)), Some(int(0))));
        assert_eq!(m.units[2].fixed[0].lo, Some(int(1) - tol()));
    }

    #[test]
    fn distances_between_sets_use_separators_pairs_and_sides() {
        let o = |k| PointRef::Origin(k);
        let p = problem_of(
            vec![point(), point(), point(), point()],
            vec![
                fact(FactKind::Distance {
                    axis: Axis::X,
                    sense: Sense::Increasing,
                    min: Some(2.0),
                    max: Some(5.0),
                    from: vec![o(0), o(1)],
                    to: vec![o(2), o(3)],
                }),
                fact(FactKind::Distance {
                    axis: Axis::X,
                    sense: Sense::Either,
                    min: Some(1.0),
                    max: None,
                    from: vec![o(0)],
                    to: vec![o(1)],
                }),
                fact(FactKind::Distance {
                    axis: Axis::X,
                    sense: Sense::Either,
                    min: Some(0.0),
                    max: None,
                    from: vec![o(0)],
                    to: vec![o(1)],
                }),
            ],
        );
        let m = Model::new(&p, tol());
        let u = &m.units[0];
        // a separator: two rows for the points before, two for those after
        assert_eq!(u.fixed.len(), 4);
        assert!(m.vars.iter().any(|v| matches!(v, VarKind::Separator { unit: 0, gap: 0, .. })));
        // the upper bound: one of four pairs
        assert_eq!(u.choices.len(), 1);
        assert_eq!(u.choices[0].key, LitKey::Pair(0, 0));
        assert_eq!(u.choices[0].values.len(), 4);
        // either side, of two points: a side to choose; with min 0 nothing to choose
        assert_eq!(m.units[1].choices[0].key, LitKey::Side(1));
        assert!(m.units[2].choices.is_empty() && m.units[2].fixed.is_empty());
    }

    #[test]
    fn centres_of_three_points_have_literals_and_of_two_an_equation() {
        let o = |k| PointRef::Origin(k);
        let p = problem_of(
            vec![point(), point(), point(), point()],
            vec![
                fact(FactKind::Align { axis: Axis::Y, members: vec![PointRef::Centre(vec![o(0), o(1), o(2)]), o(3)] }),
                fact(FactKind::Align {
                    axis: Axis::X,
                    members: vec![PointRef::Centre(vec![o(1), PointRef::Centre(vec![o(0)])]), o(3)],
                }),
            ],
        );
        let m = Model::new(&p, tol());
        assert_eq!(m.centres.len(), 2);
        assert_eq!(m.centres[1].members, vec![PointId::Origin(0), PointId::Origin(1)]);
        assert_eq!(m.units[0].centres, [vec![], vec![0]]);
        assert!(m.units[1].centres[0].is_empty());
        assert_eq!(m.centre_definition(0, 1).len(), 7);
        assert_eq!(m.centre_definition(1, 0).len(), 1);
        // the members of a centre are touched with it
        assert_eq!(
            m.units[0].touched[1],
            vec![PointId::Origin(0), PointId::Origin(1), PointId::Origin(2), PointId::Origin(3), PointId::Centre(0)]
        );
    }
}
