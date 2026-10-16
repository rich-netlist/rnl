//! Candidates and the beam search over them (plan 5.5; algorithm synthesis 5.4).
//!
//! A **candidate** ([`Candidate`]) is a set of discrete decisions: the orientation of every free
//! orientation class, the rows of the elements and their order left to right, the groups kept
//! apart as blocks, and the structural relations that follow from them ([`relations`]): the
//! members of a row on one height, neighbours in a row apart by their boxes and a gap, neighbouring
//! rows apart likewise, neighbouring blocks of a group apart, ports beside the pins they connect
//! to, and, for a candidate with even rows, equal steps in each row. Each relation is a unit of
//! the solver's own, ranked after every fact (8.5): the resolution takes it in after the facts
//! kept, and one that does not fit with them is dropped, never a fact.
//!
//! **P0**, the baseline and fallback: rows by current depth (one row when there is no
//! supply and the facts say nothing of heights), the order of the netlist in a row, every free
//! orientation R0, ports beside what they connect to, supply and ground stubs where their own
//! facts put them.
//!
//! **The beam search** ([`search`]) goes up the tree of groups (`crate::structure::GroupTree`),
//! children before their parents. At a node, the items are its children, each a block with the
//! variants found for it, and its own elements:
//!
//! 1. the variants of the children are combined, a beam of `beam` combinations kept;
//! 2. the order of the items is made by the barycentre of what each is connected to, over a few
//!    rounds, keeping the left-to-right precedences the facts give, then varied by swapping
//!    neighbours, and by reversing each ORDER or DISTANCE on x whose side the facts leave open
//!    (`crate::structure::Reversible`), which the resolution then turns;
//! 3. each free class whose members lie in the node takes the orientation that turns most of its
//!    pins towards what they connect to, among those its sources allow (sources towards the
//!    ground, grounds below their pins), and is varied to its second best; mirror pairs take
//!    mirrored orientations;
//! 4. every variant is ranked by the layered estimates of the objectives in force
//!    (`crate::objectives::estimate`) on a sketch of the whole level, ties by the canonical
//!    code; the best are checked by the resolution with the node's relations, those whose
//!    relations all fit first, and `beam` of them are kept.
//!
//! The root's variants are the complete candidates; P0 joins them, and with the even rows in force
//! each has a twin with evenly spaced rows. The coordinates of each are then sought by the
//! lexicographic LP (`crate::coords`). The search makes no more than `Budget::candidates`
//! candidates; past that it keeps what it has.

use std::collections::{BTreeMap, BTreeSet};

use num_traits::{One, Signed, Zero};
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::LevelProblem;

use crate::model::{Bound, Choice, ChoiceKind, LitKey, Model, OrientEffect, PointId, Unit, VarKind};
use crate::objectives::View;
use crate::orient::{index, inverse};
use crate::rational::{from_f64, int, Q};
use crate::resolve::{Geometry, Outcome, Resolver};
use crate::simplex::LinExpr;
use crate::structure::{FreeClass, PinKey, PortKind, Role, Source, Structure};
use crate::{Budget, SolverOptions, Work};

/// The gap between neighbours in a row, and between blocks, in units.
pub(crate) const HGAP: i64 = 2;
/// The gap between neighbouring rows.
pub(crate) const VGAP: i64 = 2;
/// How far a port is from the box of what it connects to, and a supply or ground symbol from its
/// pin.
pub(crate) const ATTACH_GAP: i64 = 2;

/// How many rounds of ordering by barycentres a node takes at most; it stops at an order seen.
const BARYCENTRE_ROUNDS: usize = 8;

/// Where a port or a free supply or ground symbol goes: beside a pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Left,
    Right,
    Below,
    Above,
}

/// A port or a supply or ground symbol that no fact places, put beside the pin it connects to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Attach {
    pub host: PinKey,
    pub side: Side,
}

/// A candidate: the discrete decisions of a layout (see the module).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Candidate {
    /// The orientation of each free class, by its index in `Open::classes`: the orientation of its
    /// root.
    pub values: Vec<Orient>,
    /// The rows, the top first, each left to right.
    pub rows: Vec<Vec<usize>>,
    /// Blocks next to each other in a group: the elements of the left one and of the right one.
    pub apart: Vec<(Vec<usize>, Vec<usize>)>,
    /// Rows of three or more members evenly spaced.
    pub even: bool,
    /// Elements in neighbouring rows connected pin to pin put in one column.
    pub columns: bool,
    /// Where it comes from: "P0" or "beam".
    pub origin: &'static str,
}

impl Candidate {
    /// The orientation of every object: the free classes as the candidate says, the others as the
    /// witness of the resolution has them.
    pub(crate) fn orientations(&self, s: &Structure) -> Vec<Orient> {
        orientations(s, &self.values)
    }

    /// The canonical code: the orientation of each object with one, in the order of the objects,
    /// then the row and the place in it of each element, then the even rows. Ties between
    /// candidates go to the smaller code (plan 5.8).
    pub(crate) fn code(&self, s: &Structure) -> Vec<u32> {
        let ori = self.orientations(s);
        let mut code: Vec<u32> =
            (0..s.objects.len()).filter(|&k| s.objects[k].oriented).map(|k| u32::from(index(ori[k]))).collect();
        let mut place: BTreeMap<usize, (u32, u32)> = BTreeMap::new();
        for (r, row) in self.rows.iter().enumerate() {
            for (i, &k) in row.iter().enumerate() {
                place.insert(k, (r as u32, i as u32));
            }
        }
        for &k in &s.elements {
            let (r, i) = place.get(&k).copied().unwrap_or((u32::MAX, u32::MAX));
            code.extend([r, i]);
        }
        code.push(u32::from(self.even));
        code.push(u32::from(self.columns));
        code
    }
}

/// The orientation of every object given the values of the free classes.
fn orientations(s: &Structure, values: &[Orient]) -> Vec<Orient> {
    let mut ori = s.witness.orient.clone();
    for (class, value) in s.open.classes.iter().zip(values) {
        for (m, label) in &class.members {
            ori[*m] = value.then(*label);
        }
    }
    ori
}

/// What a relation is for, to say which were dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RelKind {
    Orientation,
    Separation,
    Attachment,
    Row,
    Gap,
    RowsApart,
    BlocksApart,
    Column,
    Even,
}

impl RelKind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            RelKind::Orientation => "orientation",
            RelKind::Separation => "separation of overlapping symbols",
            RelKind::Attachment => "port or symbol beside its pin",
            RelKind::Row => "row alignment",
            RelKind::Gap => "gap between neighbours in a row",
            RelKind::RowsApart => "gap between rows",
            RelKind::BlocksApart => "gap between blocks",
            RelKind::Column => "column",
            RelKind::Even => "even row",
        }
    }
}

/// A term of a relation: a point of the model with a coefficient.
pub(crate) type Term = (PointId, Q);

/// A relation of the solver's own, one unit of the resolution (8.5).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Relation {
    /// An orientation class takes a value: the orientation of its root (11.2).
    Orient { root: usize, value: Orient },
    /// `lo ≤ Σ aᵢ pᵢ ≤ hi` on an axis.
    Linear { axis: usize, terms: Vec<Term>, lo: Option<Q>, hi: Option<Q> },
    /// Every point of `low` plus its offset is at most some `t`, and every point of `high` plus
    /// its offset at least `t + gap`: the extents apart on an axis.
    Apart { axis: usize, low: Vec<Term>, high: Vec<Term>, gap: Q },
    /// One of several lists of lower bounds `Σ aᵢ pᵢ ≥ c` on an axis, the first preferred.
    OneOf(Vec<Vec<(usize, Vec<Term>, Q)>>),
    /// Equal steps between neighbours on an axis (14.4).
    Even { axis: usize, points: Vec<PointId> },
}

/// A relation and what it is for.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Tagged {
    pub kind: RelKind,
    pub rel: Relation,
}

impl Relation {
    /// The relation as the unit `index` of a model, its separator made in the model.
    pub(crate) fn unit(&self, model: &mut Model, index: usize) -> Unit {
        let mut fixed = Vec::new();
        let mut choices = Vec::new();
        let mut orient = OrientEffect::None;
        let mut touched: [BTreeSet<PointId>; 2] = [BTreeSet::new(), BTreeSet::new()];
        let expr = |model: &Model, terms: &[Term]| {
            LinExpr::from_terms(terms.iter().map(|(p, a)| (model.var(*p), a.clone())), Q::zero())
        };
        match self {
            Relation::Orient { root, value } => orient = OrientEffect::Fix { object: *root, code: *value },
            Relation::Linear { axis, terms, lo, hi } => {
                touched[*axis].extend(terms.iter().map(|(p, _)| *p));
                fixed.push(Bound { axis: *axis, expr: expr(model, terms), lo: lo.clone(), hi: hi.clone() });
            }
            Relation::Apart { axis, low, high, gap } => {
                let a = *axis;
                touched[a].extend(low.iter().chain(high).map(|(p, _)| *p));
                if let ([(l, ol)], [(h, oh)]) = (low.as_slice(), high.as_slice()) {
                    // h + oh ≥ l + ol + gap
                    let e = LinExpr::diff(model.var(*h), model.var(*l));
                    fixed.push(Bound::at_least(a, e, gap + ol - oh));
                } else {
                    let before: Vec<PointId> = low.iter().map(|(p, _)| *p).collect();
                    let t = model.new_var(VarKind::Separator { unit: index, gap: 0, before });
                    for (p, o) in low {
                        fixed.push(Bound::at_most(a, LinExpr::diff(model.var(*p), t), -o));
                    }
                    for (p, o) in high {
                        fixed.push(Bound::at_least(a, LinExpr::diff(model.var(*p), t), gap - o));
                    }
                }
            }
            Relation::OneOf(alternatives) => {
                let values: Vec<Vec<Bound>> = alternatives
                    .iter()
                    .map(|bounds| {
                        bounds
                            .iter()
                            .map(|(axis, terms, c)| {
                                touched[*axis].extend(terms.iter().map(|(p, _)| *p));
                                Bound::at_least(*axis, expr(model, terms), c.clone())
                            })
                            .collect()
                    })
                    .collect();
                choices.push(Choice { key: LitKey::Side(index), kind: ChoiceKind::Ranked, values });
            }
            Relation::Even { axis, points } => {
                touched[*axis].extend(points.iter().copied());
                for w in points.windows(3) {
                    let e = LinExpr::from_terms(
                        [(model.var(w[2]), Q::one()), (model.var(w[1]), int(-2)), (model.var(w[0]), Q::one())],
                        Q::zero(),
                    );
                    fixed.push(Bound::equal(*axis, e, Q::zero()));
                }
            }
        }
        let [x, y] = touched;
        Unit {
            fixed,
            choices,
            orient,
            touched: [x.into_iter().collect(), y.into_iter().collect()],
            centres: [Vec::new(), Vec::new()],
        }
    }
}

/// What became of the relations that [`with_relations`] takes in.
pub(crate) struct Taken {
    /// Whether each was kept.
    pub kept: Vec<bool>,
    /// Whether the budget ran out deciding any of them (`Outcome::Undecided`): those were
    /// dropped without a conflict.
    pub undecided: bool,
}

/// The facts kept and then the relations, taken in by a resolution that starts from the witness
/// (8.5): `f` is given the resolution, its model, and what became of the relations.
pub(crate) fn with_relations<R>(
    s: &Structure,
    relations: &[Tagged],
    budget: Budget,
    work: &mut Work,
    f: impl FnOnce(&mut Resolver, &Model, &Taken) -> R,
) -> R {
    let mut model = s.model.clone();
    let base = model.units.len();
    let units: Vec<Unit> = relations.iter().enumerate().map(|(i, r)| r.rel.unit(&mut model, base + i)).collect();
    model.units.extend(units);
    let mut r = Resolver::new(&model, s.objects.len(), budget);
    r.seed(s.witness.clone());
    for &f in &s.kept {
        let kept = r.add(f);
        debug_assert_eq!(kept, Outcome::Kept, "the facts kept hold at the witness");
    }
    let outcomes: Vec<Outcome> = (0..relations.len()).map(|i| r.add(base + i)).collect();
    let taken = Taken {
        kept: outcomes.iter().map(|o| *o == Outcome::Kept).collect(),
        undecided: outcomes.iter().any(|o| matches!(o, Outcome::Undecided(_))),
    };
    let out = f(&mut r, &model, &taken);
    work.resolve_nodes += r.nodes;
    work.pivots += r.pivots();
    out
}

/// The point that is an object's origin in the model: its own, or the point it is drawn at.
fn origin(s: &Structure, k: usize) -> Option<PointId> {
    s.model.origin_point.get(k).copied().flatten()
}

/// A box with nothing in it, at the origin.
fn empty_box() -> [Q; 4] {
    [Q::zero(), Q::zero(), Q::zero(), Q::zero()]
}

fn union(a: &mut [Q; 4], b: &[Q; 4]) {
    for i in 0..2 {
        if b[i] < a[i] {
            a[i] = b[i].clone();
        }
        if b[i + 2] > a[i + 2] {
            a[i + 2] = b[i + 2].clone();
        }
    }
}

fn shifted(b: &[Q; 4], d: &[Q; 2]) -> [Q; 4] {
    [&b[0] + &d[0], &b[1] + &d[1], &b[2] + &d[0], &b[3] + &d[1]]
}

/// A constant of a relation rounded up to the grid, so that the coordinates the LP gives stay on
/// it: an inequality between origins on the grid with a constant on it has its optimum on it.
pub(crate) fn up(c: Q, grid: &Option<Q>) -> Q {
    match grid {
        Some(g) if g.is_positive() => (c / g).ceil() * g,
        _ => c,
    }
}

fn grid_of(options: &SolverOptions) -> Option<Q> {
    options.grid.and_then(from_f64).filter(|g| g.is_positive())
}

/// Where each port and free supply or ground symbol goes (see the module): a port beside the pin
/// it connects to, on the side the pin leaves when it leaves sideways, otherwise beside the pin
/// of the topmost element on its net, on the right for an output and on the left otherwise; a
/// supply above and a ground below the first pin on their net. Objects that a fact kept reaches
/// are left where the facts put them.
pub(crate) fn attachments(s: &Structure, rows: &[Vec<usize>], ori: &[Orient]) -> BTreeMap<usize, Attach> {
    let row_of: BTreeMap<usize, usize> =
        rows.iter().enumerate().flat_map(|(r, row)| row.iter().map(move |&k| (k, r))).collect();
    let mut out = BTreeMap::new();
    for (k, info) in s.objects.iter().enumerate() {
        if !info.own || info.fact_touched || info.stub.is_some() || origin(s, k).is_none() || s.pins(k).is_empty() {
            continue;
        }
        let net_pins = |k: usize| -> Vec<PinKey> {
            let mut pins: Vec<PinKey> = s.connections.targets.get(&(k, 0)).cloned().unwrap_or_default();
            if pins.is_empty() {
                for &i in s.connections.nets_of.get(&(k, 0)).map_or(&[][..], |v| v) {
                    pins.extend(s.connections.nets[i].pins.iter().copied().filter(|p| p.0 != k));
                }
            }
            pins.retain(|p| s.objects[p.0].role == Role::Element);
            pins.sort_unstable();
            pins.dedup();
            pins
        };
        match info.role {
            Role::Port => {
                let pins = net_pins(k);
                let sideways = pins.iter().find_map(|&p| match s.exit(p.0, p.1, ori[p.0]) {
                    Some((-1, 0)) => Some((p, Side::Left)),
                    Some((1, 0)) => Some((p, Side::Right)),
                    _ => None,
                });
                let attach = sideways.or_else(|| {
                    let kind = s.ports.iter().find(|p| p.object == k).map_or(PortKind::Unknown, |p| p.kind);
                    let side = if kind == PortKind::Output { Side::Right } else { Side::Left };
                    pins.iter()
                        .min_by_key(|p| (row_of.get(&p.0).copied().unwrap_or(usize::MAX), **p))
                        .map(|&p| (p, side))
                });
                if let Some((host, side)) = attach {
                    out.insert(k, Attach { host, side });
                }
            }
            Role::Supply | Role::Ground => {
                if let Some(&host) = net_pins(k).first() {
                    out.insert(
                        k,
                        Attach { host, side: if info.role == Role::Supply { Side::Above } else { Side::Below } },
                    );
                }
            }
            _ => {}
        }
    }
    out
}

/// Where an attached object's origin is from its host's origin, tight against its gap.
fn attach_offset(s: &Structure, k: usize, a: &Attach, ori: &[Orient]) -> Option<[Q; 2]> {
    let (h, hp) = a.host;
    let hpin = s.pin_offset(h, hp, ori[h])?;
    let ppin = s.pin_offset(k, 0, ori[k])?;
    let gap = int(ATTACH_GAP);
    Some(match a.side {
        Side::Left | Side::Right => {
            // clear of the host's texts too
            let hb = dressed_box(s, h, ori[h]).unwrap_or_else(empty_box);
            let pb = s.box_offset(k, ori[k]).unwrap_or_else(empty_box);
            let x = if a.side == Side::Left { &hb[0] - &gap - &pb[2] } else { &hb[2] + &gap - &pb[0] };
            [x, &hpin[1] - &ppin[1]]
        }
        Side::Below => [&hpin[0] - &ppin[0], &hpin[1] - &gap - &ppin[1]],
        Side::Above => [&hpin[0] - &ppin[0], &hpin[1] + &gap - &ppin[1]],
    })
}

/// The box of a symbol with the texts it places.
fn dressed_box(s: &Structure, k: usize, o: Orient) -> Option<[Q; 4]> {
    let mut b = s.box_offset(k, o)?;
    if let Some(l) = s.label_offset(k, o) {
        union(&mut b, &l);
    }
    Some(b)
}

/// The box of an element with what hangs from it, the texts of each symbol included: its stubs
/// where their facts put them, and what is attached to it.
fn effective_box(s: &Structure, k: usize, ori: &[Orient], attach: &BTreeMap<usize, Attach>) -> [Q; 4] {
    let mut b = dressed_box(s, k, ori[k]).unwrap_or_else(empty_box);
    for &j in s.hanging.get(&k).map_or(&[][..], |v| v) {
        let Some(stub) = &s.objects[j].stub else { continue };
        let (Some(hp), Some(sp), Some(sb)) =
            (s.pin_offset(k, stub.host.1, ori[k]), s.pin_offset(j, 0, ori[j]), dressed_box(s, j, ori[j]))
        else {
            continue;
        };
        let at = [&hp[0] - &sp[0], &hp[1] + &stub.rise - &sp[1]];
        union(&mut b, &shifted(&sb, &at));
    }
    for (&j, a) in attach.iter().filter(|(_, a)| a.host.0 == k) {
        if let (Some(at), Some(sb)) = (attach_offset(s, j, a, ori), dressed_box(s, j, ori[j])) {
            union(&mut b, &shifted(&sb, &at));
        }
    }
    b
}

/// The relations of a candidate (see the module), with the orientations `ori` of the objects:
/// the attachments, the rows, the gaps in the rows, the gaps between rows, the blocks apart, and
/// the even rows. The orientations themselves are [`orientation_relations`].
pub(crate) fn relations(s: &Structure, c: &Candidate, ori: &[Orient], options: &SolverOptions) -> Vec<Tagged> {
    relations_within(s, c, ori, options, None)
}

/// The relations of a candidate among the elements of `within` only, when given.
fn relations_within(
    s: &Structure,
    c: &Candidate,
    ori: &[Orient],
    options: &SolverOptions,
    within: Option<&BTreeSet<usize>>,
) -> Vec<Tagged> {
    let grid = grid_of(options);
    let inside = |k: &usize| within.is_none_or(|w| w.contains(k));
    let attach = attachments(s, &c.rows, ori);
    let eff: BTreeMap<usize, [Q; 4]> = s.elements.iter().map(|&k| (k, effective_box(s, k, ori, &attach))).collect();
    let mut out = Vec::new();
    let o = |k: usize| origin(s, k);
    let one = Q::one;
    if within.is_none() {
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
            // the gap at least as the offset says, on the grid
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
    }
    let rows: Vec<Vec<usize>> = c
        .rows
        .iter()
        .map(|row| row.iter().copied().filter(inside).collect::<Vec<usize>>())
        .filter(|r| !r.is_empty())
        .collect();
    for row in &rows {
        for w in row.windows(2) {
            let (Some(a), Some(b)) = (o(w[0]), o(w[1])) else { continue };
            // neighbours drawn to each other through a pin that leaves sideways: those pins on
            // one horizontal line, so that the wire between them is straight; others by origin
            let c = row_link(s, w[0], w[1], ori).map_or_else(Q::zero, |(p, q)| &p[1] - &q[1]);
            out.push(Tagged {
                kind: RelKind::Row,
                rel: Relation::Linear {
                    axis: 1,
                    terms: vec![(b, one()), (a, -one())],
                    lo: Some(c.clone()),
                    hi: Some(c),
                },
            });
        }
    }
    for row in &rows {
        for w in row.windows(2) {
            let (Some(a), Some(b)) = (o(w[0]), o(w[1])) else { continue };
            let c = up(&eff[&w[0]][2] - &eff[&w[1]][0] + int(HGAP), &grid);
            out.push(Tagged {
                kind: RelKind::Gap,
                rel: Relation::Linear { axis: 0, terms: vec![(b, one()), (a, -one())], lo: Some(c), hi: None },
            });
        }
    }
    for w in rows.windows(2) {
        let low: Vec<Term> = w[1].iter().filter_map(|&k| Some((o(k)?, up(eff[&k][3].clone(), &grid)))).collect();
        let high: Vec<Term> = w[0].iter().filter_map(|&k| Some((o(k)?, -up(-eff[&k][1].clone(), &grid)))).collect();
        if !low.is_empty() && !high.is_empty() {
            out.push(Tagged { kind: RelKind::RowsApart, rel: Relation::Apart { axis: 1, low, high, gap: int(VGAP) } });
        }
    }
    for (left, right) in &c.apart {
        if !left.iter().chain(right).all(inside) {
            continue;
        }
        let low: Vec<Term> = left.iter().filter_map(|&k| Some((o(k)?, up(eff[&k][2].clone(), &grid)))).collect();
        let high: Vec<Term> = right.iter().filter_map(|&k| Some((o(k)?, -up(-eff[&k][0].clone(), &grid)))).collect();
        if !low.is_empty() && !high.is_empty() {
            out.push(Tagged {
                kind: RelKind::BlocksApart,
                rel: Relation::Apart { axis: 0, low, high, gap: int(HGAP) },
            });
        }
    }
    if c.columns {
        out.extend(columns(s, &rows, ori));
    }
    if c.even {
        for row in rows.iter().filter(|r| r.len() >= 3) {
            let points: Vec<PointId> = row.iter().filter_map(|&k| o(k)).collect();
            out.push(Tagged { kind: RelKind::Even, rel: Relation::Even { axis: 0, points } });
        }
    }
    out
}

/// The pins by which two neighbours of a row are drawn to each other, when one of them leaves
/// sideways: their offsets from their origins, `a`'s first; the pair that leave towards each other
/// first, then the first pins.
fn row_link(s: &Structure, a: usize, b: usize, ori: &[Orient]) -> Option<([Q; 2], [Q; 2])> {
    let mut best: Option<((u8, usize, usize), usize, usize)> = None;
    for p in 0..s.pins(a).len() {
        for &t in s.connections.targets.get(&(a, p)).map_or(&[][..], |v| v) {
            if t.0 != b {
                continue;
            }
            let (ea, eb) = (s.exit(a, p, ori[a]), s.exit(b, t.1, ori[b]));
            let sideways = |e: Option<(i64, i64)>| e.is_some_and(|(_, y)| y == 0);
            let rank = if ea == Some((1, 0)) && eb == Some((-1, 0)) {
                0
            } else if sideways(ea) || sideways(eb) {
                1
            } else {
                continue;
            };
            let key = (rank, p, t.1);
            if best.as_ref().is_none_or(|(k, _, _)| key < *k) {
                best = Some((key, p, t.1));
            }
        }
    }
    let (_, p, q) = best?;
    Some((s.pin_offset(a, p, ori[a])?, s.pin_offset(b, q, ori[b])?))
}

/// What orders the columns an element may join: see [`columns`].
type ColumnKey = (bool, usize, usize, usize);

/// The columns of a candidate (`Source::Columns`): for each element of a row, the first element of
/// the row above that one of its pins is drawn to, a pin the current runs through on both sides
/// first, with the two pins on one vertical line; each element of the row above in one column at
/// most.
fn columns(s: &Structure, rows: &[Vec<usize>], ori: &[Orient]) -> Vec<Tagged> {
    let mut out = Vec::new();
    for w in rows.windows(2) {
        let (above, below) = (&w[0], &w[1]);
        let mut taken: BTreeSet<usize> = BTreeSet::new();
        for &b in below {
            // (whether not both pins carry the current, the rank above, the pins), the pins
            let mut best: Option<(ColumnKey, PinKey, PinKey)> = None;
            for q in 0..s.pins(b).len() {
                for &t in s.connections.targets.get(&(b, q)).map_or(&[][..], |v| v) {
                    let Some(rank) = above.iter().position(|&a| a == t.0) else { continue };
                    if taken.contains(&t.0) {
                        continue;
                    }
                    let both = s.conducts((b, q)) && s.conducts(t);
                    let key = (!both, rank, q, t.1);
                    if best.as_ref().is_none_or(|(k, _, _)| key < *k) {
                        best = Some((key, (b, q), t));
                    }
                }
            }
            let Some((_, (b, q), (a, p))) = best else { continue };
            let (Some(pb), Some(pa)) = (origin(s, b), origin(s, a)) else { continue };
            let (Some(qo), Some(po)) = (s.pin_offset(b, q, ori[b]), s.pin_offset(a, p, ori[a])) else { continue };
            taken.insert(a);
            let c = &po[0] - &qo[0];
            out.push(Tagged {
                kind: RelKind::Column,
                rel: Relation::Linear {
                    axis: 0,
                    terms: vec![(pb, Q::one()), (pa, -Q::one())],
                    lo: Some(c.clone()),
                    hi: Some(c),
                },
            });
        }
    }
    out
}

/// The orientations of a candidate's free classes as relations, one each.
pub(crate) fn orientation_relations(s: &Structure, c: &Candidate) -> Vec<Tagged> {
    s.open
        .classes
        .iter()
        .zip(&c.values)
        .map(|(class, value)| Tagged {
            kind: RelKind::Orientation,
            rel: Relation::Orient { root: class.root, value: *value },
        })
        .collect()
}

/// A rough placement of a candidate, for the estimates of the search: the rows from the top
/// down, each packed from the left with its boxes and gaps, stubs where their facts put them,
/// attached objects beside their pins, everything else where the witness has it. Gives the origin
/// of every object ([`Structure::origins`]).
pub(crate) fn sketch(problem: &LevelProblem, s: &Structure, c: &Candidate, ori: &[Orient]) -> Vec<[Q; 2]> {
    let attach = attachments(s, &c.rows, ori);
    let mut g: Geometry = s.witness.clone();
    g.orient = ori.to_vec();
    let mut y = Q::zero();
    let mut previous_bottom: Option<Q> = None;
    for row in &c.rows {
        let boxes: Vec<[Q; 4]> = row.iter().map(|&k| effective_box(s, k, ori, &attach)).collect();
        // each member's height from the first, as the row's relations put it
        let mut dy = vec![Q::zero(); row.len()];
        for i in 1..row.len() {
            let c = row_link(s, row[i - 1], row[i], ori).map_or_else(Q::zero, |(p, q)| &p[1] - &q[1]);
            dy[i] = &dy[i - 1] + c;
        }
        let top = boxes.iter().zip(&dy).map(|(b, d)| &b[3] + d).max().unwrap_or_default();
        let bottom = boxes.iter().zip(&dy).map(|(b, d)| &b[1] + d).min().unwrap_or_default();
        if let Some(pb) = &previous_bottom {
            y = &y + pb - int(VGAP) - &top;
        }
        let mut cursor = Q::zero();
        for ((&k, b), d) in row.iter().zip(&boxes).zip(&dy) {
            let x = &cursor - &b[0];
            cursor = &x + &b[2] + int(HGAP);
            g.origin[k] = [x, &y + d];
        }
        previous_bottom = Some(bottom);
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
    for (&k, a) in &attach {
        if let Some(at) = attach_offset(s, k, a, ori) {
            let base = g.origin[a.host.0].clone();
            g.origin[k] = [&base[0] + &at[0], &base[1] + &at[1]];
        }
    }
    s.origins(problem, &g)
}

/// The orientations a free class may take: those that keep the stubs of its members off their
/// bodies, always; that turn the sources of its transistors towards the ground, hang its
/// two-terminal elements with an end on a supply or the ground towards it and lay those between
/// two signals across, and hang its grounds below their pins, as far as the sources in force ask.
/// All eight when none satisfies them.
fn domain(s: &Structure, class: &FreeClass) -> Vec<Orient> {
    let ok = |v: Orient| {
        class.members.iter().all(|&(m, label)| {
            let o = v.then(label);
            let info = &s.objects[m];
            if stub_on_body(s, m, o) {
                return false;
            }
            if s.has(Source::SourcesDown) && !two_terminal_ok(s, m, o) {
                return false;
            }
            let down = match info.device.n_type() {
                Some(n) if s.has(Source::SourcesDown) => match (s.drain(m), s.source(m)) {
                    (Some(d), Some(src)) => {
                        let (Some(d), Some(src)) = (s.pin_offset(m, d, o), s.pin_offset(m, src, o)) else {
                            return true;
                        };
                        if n {
                            src[1] < d[1]
                        } else {
                            src[1] > d[1]
                        }
                    }
                    _ => true,
                },
                _ => true,
            };
            let ground = if info.role == Role::Ground && s.has(Source::GroundsDown) {
                match (s.pin_offset(m, 0, o), s.box_offset(m, o)) {
                    (Some(p), Some(b)) => p[1].clone() * int(2) > &b[1] + &b[3],
                    _ => true,
                }
            } else {
                true
            };
            down && ground
        })
    };
    let allowed: Vec<Orient> = Orient::ALL.into_iter().filter(|v| ok(*v)).collect();
    if allowed.is_empty() {
        Orient::ALL.to_vec()
    } else {
        allowed
    }
}

/// Whether a stub of an element, hung where its facts put it from a pin of the element turned by
/// `o`, lies on the element's body.
fn stub_on_body(s: &Structure, m: usize, o: Orient) -> bool {
    let Some(body) = s.box_offset(m, o) else { return false };
    s.hanging.get(&m).is_some_and(|stubs| {
        stubs.iter().any(|&j| {
            let Some(stub) = &s.objects[j].stub else { return false };
            let so = s.witness.orient[j];
            let (Some(hp), Some(sp), Some(sb)) =
                (s.pin_offset(m, stub.host.1, o), s.pin_offset(j, 0, so), s.box_offset(j, so))
            else {
                return false;
            };
            let at = [&hp[0] - &sp[0], &hp[1] + &stub.rise - &sp[1]];
            let b = shifted(&sb, &at);
            b[0].clone().max(body[0].clone()) < b[2].clone().min(body[2].clone())
                && b[1].clone().max(body[1].clone()) < b[3].clone().min(body[3].clone())
        })
    })
}

/// Whether a two-terminal element turned by `o` hangs an end on a supply up and an end on the
/// ground down, or, with neither, lies across: its pins leave up or down for an end on a rail, left
/// or right between two signals. Other objects, and pins without a direction, are not judged.
fn two_terminal_ok(s: &Structure, m: usize, o: Orient) -> bool {
    let info = &s.objects[m];
    if info.role != Role::Element || info.device.n_type().is_some() || s.pins(m).len() != 2 {
        return true;
    }
    let rail = |j: usize| {
        if s.power.supply_pins.contains(&(m, j)) {
            Some((0, 1))
        } else if s.power.ground_pins.contains(&(m, j)) {
            Some((0, -1))
        } else {
            None
        }
    };
    let rails: Vec<(usize, (i64, i64))> = (0..2).filter_map(|j| rail(j).map(|d| (j, d))).collect();
    if rails.is_empty() {
        (0..2).all(|j| s.exit(m, j, o).is_none_or(|(_, y)| y == 0))
    } else {
        rails.iter().all(|&(j, d)| s.exit(m, j, o).is_none_or(|e| e == d))
    }
}

/// The order in which equally good orientations are preferred: unturned, then mirrored, then
/// turned upside down, then a quarter turn.
pub(crate) fn preference(o: Orient) -> u8 {
    match o {
        Orient::R0 => 0,
        Orient::MY => 1,
        Orient::MX => 2,
        Orient::R180 => 3,
        Orient::R90 => 4,
        Orient::R270 => 5,
        Orient::MXR90 => 6,
        Orient::MYR90 => 7,
    }
}

/// How many pins of a class's members, at the class value `value`, leave towards what they are
/// drawn to: one for each pin and target on the side the pin leaves (9.2.4). A stub's pin is where
/// its facts put it from its pin; ports are left out, as they go where the pins leave.
fn facing(s: &Structure, at: &[[Q; 2]], ori: &[Orient], class: &FreeClass, value: Orient) -> i64 {
    let mut score = 0;
    for &(m, label) in &class.members {
        let o = value.then(label);
        for pin in 0..s.pins(m).len() {
            let (Some(e), Some(off)) = (s.exit(m, pin, o), s.pin_offset(m, pin, o)) else { continue };
            let p = [&at[m][0] + &off[0], &at[m][1] + &off[1]];
            for &t in s.connections.targets.get(&(m, pin)).map_or(&[][..], |v| v) {
                let info = &s.objects[t.0];
                if info.role == Role::Port {
                    continue;
                }
                let v = match &info.stub {
                    Some(stub) if stub.host == (m, pin) => [Q::zero(), stub.rise.clone()],
                    _ => match s.pin_at(at, ori, t.0, t.1) {
                        Some(q) => [&q[0] - &p[0], &q[1] - &p[1]],
                        None => continue,
                    },
                };
                let dot = int(e.0) * &v[0] + int(e.1) * &v[1];
                score += i64::from(dot.is_positive());
            }
        }
    }
    score
}

/// The values of a class best first: the most pins facing, then by [`preference`].
fn ranked_values(s: &Structure, at: &[[Q; 2]], ori: &[Orient], class: &FreeClass) -> Vec<Orient> {
    let mut values: Vec<(i64, u8, Orient)> =
        domain(s, class).into_iter().map(|v| (-facing(s, at, ori, class, v), preference(v), v)).collect();
    values.sort_by_key(|(f, i, _)| (*f, *i));
    values.into_iter().map(|(_, _, v)| v).collect()
}

/// An item of a group node: a child block, or one of its own elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Item {
    Block(usize),
    Element(usize),
}

/// The decisions taken so far for the whole level.
#[derive(Clone, Debug, PartialEq)]
struct Plan {
    /// Every element, in an order left to right that the rows follow.
    order: Vec<usize>,
    /// The value of each free class.
    values: Vec<Orient>,
    /// The order of the items of each node decided.
    items: BTreeMap<usize, Vec<Item>>,
    /// The facts whose side is open that the plan reverses, by index in `Structure::reversible`.
    flipped: BTreeSet<usize>,
}

/// A plan with its layered estimate and the canonical code of its candidate.
type Scored = (Vec<Q>, Vec<u32>, Plan);

/// A variant of a node: its elements in order, the values of the classes decided in it and below,
/// the orders of its items and of those below, and whether it reverses each fact whose side is open
/// decided in it and below.
#[derive(Clone, Debug, PartialEq)]
struct Variant {
    order: Vec<usize>,
    values: BTreeMap<usize, Orient>,
    items: BTreeMap<usize, Vec<Item>>,
    flips: BTreeMap<usize, bool>,
}

/// The search in progress: the level, the options, the plan of the decisions so far and the
/// count of candidates made.
struct Search<'a> {
    problem: &'a LevelProblem,
    s: &'a Structure,
    options: &'a SolverOptions,
    made: u64,
    work: &'a mut Work,
}

impl Search<'_> {
    fn out_of_budget(&self) -> bool {
        self.made >= self.options.budget.candidates
    }

    /// A plan with a variant's decisions in place.
    fn with(&self, plan: &Plan, v: &Variant) -> Plan {
        let mut p = plan.clone();
        let set: BTreeSet<usize> = v.order.iter().copied().collect();
        let mut next = v.order.iter();
        for slot in p.order.iter_mut() {
            if set.contains(slot) {
                if let Some(&k) = next.next() {
                    *slot = k;
                }
            }
        }
        for (&c, &value) in &v.values {
            p.values[c] = value;
        }
        p.items.extend(v.items.iter().map(|(n, items)| (*n, items.clone())));
        for (&r, &flip) in &v.flips {
            if flip {
                p.flipped.insert(r);
            } else {
                p.flipped.remove(&r);
            }
        }
        p
    }

    /// The node at which a fact whose side is open is decided: the lowest that holds every
    /// element it orders.
    fn reversible_node(&self, r: usize) -> usize {
        let elements: BTreeSet<usize> = self.s.reversible[r].pairs.iter().flat_map(|&(a, b)| [a, b]).collect();
        self.s.tree.lowest_common(&elements.into_iter().collect::<Vec<usize>>())
    }

    /// The candidate of a plan: its rows in its order, the blocks apart in each node, each port's
    /// class turned towards the pin it goes beside.
    fn candidate(&self, plan: &Plan, origin: &'static str) -> Candidate {
        let s = self.s;
        let rank: BTreeMap<usize, usize> = plan.order.iter().enumerate().map(|(i, &k)| (k, i)).collect();
        let rows: Vec<Vec<usize>> = s
            .rows
            .iter()
            .map(|row| {
                let mut r = row.clone();
                r.sort_by_key(|k| rank.get(k).copied().unwrap_or(usize::MAX));
                r
            })
            .collect();
        let mut apart = Vec::new();
        for items in plan.items.values() {
            let blocks: Vec<usize> =
                items.iter().filter_map(|i| if let Item::Block(b) = i { Some(*b) } else { None }).collect();
            for w in blocks.windows(2) {
                apart.push((s.tree.nodes[w[0]].members.clone(), s.tree.nodes[w[1]].members.clone()));
            }
        }
        let mut values = plan.values.clone();
        let ori = orientations(s, &values);
        for (k, a) in attachments(s, &rows, &ori) {
            if s.objects[k].role != Role::Port {
                continue;
            }
            let Some(c) = s.class_of(k) else { continue };
            let class = &s.open.classes[c];
            if class.members.len() != 1 {
                continue;
            }
            let want = match a.side {
                Side::Left => (1, 0),
                Side::Right => (-1, 0),
                _ => continue,
            };
            let label = class.members[0].1;
            let mut all = Orient::ALL;
            all.sort_by_key(|v| preference(*v));
            if let Some(v) = all.into_iter().find(|v| s.exit(k, 0, v.then(label)) == Some(want)) {
                values[c] = v;
            }
        }
        let columns = origin != "P0" && s.has(Source::Columns);
        Candidate { values, rows, apart, even: false, columns, origin }
    }

    /// The layered estimate of a plan, and its candidate.
    fn estimate(&mut self, plan: &Plan) -> (Vec<Q>, Candidate) {
        self.made += 1;
        let c = self.candidate(plan, "beam");
        let ori = c.orientations(self.s);
        let at = sketch(self.problem, self.s, &c, &ori);
        let e = self.s.active.estimate(&View { s: self.s, at: &at, orient: &ori });
        (e, c)
    }

    /// The order of a node's items: a topological order of the precedences between them, the
    /// smallest key first among those free to go. The precedences are those of the facts, `before`
    /// ([`precedences`]), and, with the mirror pairs in force, the first role of a pair (`a`,
    /// `in_p`, `ref`) before the second where the facts do not say otherwise.
    fn order_items(
        &self,
        node: usize,
        items: &[Item],
        key: &dyn Fn(Item) -> (Q, usize),
        before: &BTreeSet<(usize, usize)>,
    ) -> Vec<Item> {
        let s = self.s;
        let members = |i: Item| -> Vec<usize> {
            match i {
                Item::Block(b) => s.tree.nodes[b].members.clone(),
                Item::Element(k) => vec![k],
            }
        };
        let sets: Vec<Vec<usize>> = items.iter().map(|&i| members(i)).collect();
        let precedes = |a: usize, b: usize| {
            before.contains(&(a, b))
                || (s.has(Source::MirrorPairs)
                    && !before.contains(&(b, a))
                    && s.pairs.iter().any(|p| p.a == a && p.b == b))
        };
        let mut first: BTreeSet<(usize, usize)> = BTreeSet::new();
        for i in 0..items.len() {
            for j in 0..items.len() {
                if i != j && sets[i].iter().any(|&a| sets[j].iter().any(|&b| precedes(a, b))) {
                    first.insert((i, j));
                }
            }
        }
        if s.tree.nodes[node].fixed_order {
            for i in 1..items.len() {
                first.insert((i - 1, i));
            }
        }
        let mut done = vec![false; items.len()];
        let mut out = Vec::new();
        while out.len() < items.len() {
            let free = |j: usize| !done[j] && (0..items.len()).all(|i| done[i] || !first.contains(&(i, j)));
            let mut pick = (0..items.len()).filter(|&j| free(j)).min_by(|&a, &b| key(items[a]).cmp(&key(items[b])));
            if pick.is_none() {
                // precedences in a cycle: the smallest key of those left
                pick = (0..items.len()).filter(|&j| !done[j]).min_by(|&a, &b| key(items[a]).cmp(&key(items[b])));
            }
            let Some(j) = pick else { break };
            done[j] = true;
            out.push(items[j]);
        }
        out
    }

    /// The elements of items in order, each block in the order of the plan.
    fn flatten(&self, plan: &Plan, items: &[Item]) -> Vec<usize> {
        let rank: BTreeMap<usize, usize> = plan.order.iter().enumerate().map(|(i, &k)| (k, i)).collect();
        let mut out = Vec::new();
        for &i in items {
            match i {
                Item::Element(k) => out.push(k),
                Item::Block(b) => {
                    let mut m = self.s.tree.nodes[b].members.clone();
                    m.sort_by_key(|k| rank[k]);
                    out.extend(m);
                }
            }
        }
        out
    }

    /// The node at which a class is decided: the lowest that holds all its elements; the root for
    /// a class with none. None for a class of ports, which turn towards their pins.
    fn class_node(&self, c: usize) -> Option<usize> {
        let s = self.s;
        let members = &s.open.classes[c].members;
        let elements: Vec<usize> =
            members.iter().map(|(m, _)| *m).filter(|&m| s.objects[m].role == Role::Element).collect();
        if !elements.is_empty() {
            Some(s.tree.lowest_common(&elements))
        } else if members.iter().any(|(m, _)| s.objects[*m].role == Role::Port) {
            None
        } else {
            Some(0)
        }
    }

    /// The classes decided at a node.
    fn local_classes(&self, node: usize) -> Vec<usize> {
        (0..self.s.open.classes.len()).filter(|&c| self.class_node(c) == Some(node)).collect()
    }

    /// The nodes of the subtree of a node, itself included.
    fn subtree(&self, node: usize) -> BTreeSet<usize> {
        let mut out = BTreeSet::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            out.insert(n);
            stack.extend(self.s.tree.nodes[n].children.iter().copied());
        }
        out
    }

    /// The classes turned, one after the other, to the value that faces most, on the sketch of
    /// the plan; mirror pairs then mirrored (14.9). Gives the plan, and the second best value of
    /// each class.
    fn orient(&self, plan: &Plan, classes: &[usize]) -> (Plan, Vec<(usize, Orient)>) {
        let s = self.s;
        let mut p = plan.clone();
        let mut second = Vec::new();
        // one sketch for all: a class turned moves its own pins, not where the others are
        let cand = self.candidate(&p, "beam");
        let mut ori = cand.orientations(s);
        let at = sketch(self.problem, s, &cand, &ori);
        for &c in classes {
            let class = &s.open.classes[c];
            let ranked = ranked_values(s, &at, &ori, class);
            if let Some(&best) = ranked.first() {
                p.values[c] = best;
                for (m, label) in &class.members {
                    ori[*m] = best.then(*label);
                }
            }
            if let Some(&next) = ranked.get(1) {
                second.push((c, next));
            }
        }
        if s.has(Source::MirrorPairs) {
            for pair in &s.pairs {
                let (Some(ca), Some(cb)) = (s.class_of(pair.a), s.class_of(pair.b)) else { continue };
                if ca == cb || !classes.contains(&cb) {
                    continue;
                }
                let ori = orientations(s, &p.values);
                let label_b =
                    s.open.classes[cb].members.iter().find(|(m, _)| *m == pair.b).map_or(Orient::R0, |(_, l)| *l);
                p.values[cb] = ori[pair.a].then(Orient::MY).then(inverse(label_b));
            }
        }
        (p, second)
    }

    /// The variants of a node (see the module), best first.
    fn node(&mut self, node: usize, plan: &Plan, variants: &BTreeMap<usize, Vec<Variant>>) -> Vec<Variant> {
        let s = self.s;
        let beam = self.options.beam.max(1);
        let n = &s.tree.nodes[node];
        let items: Vec<Item> =
            n.children.iter().map(|&c| Item::Block(c)).chain(n.own.iter().map(|&k| Item::Element(k))).collect();
        // 1. the children's variants combined
        let mut combos: Vec<(Vec<Q>, Plan)> = vec![(Vec::new(), plan.clone())];
        for &c in &n.children {
            let mut next: Vec<Scored> = Vec::new();
            for (_, p) in &combos {
                for v in variants.get(&c).map_or(&[][..], |v| v) {
                    if self.out_of_budget() && !next.is_empty() {
                        break;
                    }
                    let q = self.with(p, v);
                    let (e, cand) = self.estimate(&q);
                    next.push((e, cand.code(s), q));
                }
            }
            if next.is_empty() {
                continue;
            }
            let mut next = ranked(s, next);
            next.truncate(beam);
            combos = next.into_iter().map(|(e, _, p)| (e, p)).collect();
        }
        let classes = self.local_classes(node);
        let mut found: Vec<Scored> = Vec::new();
        for (_, base) in combos.iter().take(2) {
            // 2. the orders of the items: first by the elements' places in the netlist, then by
            // the barycentre of what each is connected to
            let first_key = |i: Item| -> (Q, usize) {
                let k = match i {
                    Item::Element(k) => k,
                    Item::Block(b) => s.tree.nodes[b].members[0],
                };
                (Q::zero(), k)
            };
            let before = precedences(s, &base.flipped);
            let mut orders: Vec<Vec<Item>> = vec![self.order_items(node, &items, &first_key, &before)];
            if s.has(Source::Barycentre) || s.has(Source::InputsLeft) {
                let mut current = orders[0].clone();
                for _ in 0..BARYCENTRE_ROUNDS {
                    let p = Plan { order: self.place(base, &current), ..base.clone() };
                    let (p, _) = self.orient(&p, &classes);
                    let key = self.barycentre(&p, &current);
                    let next = self.order_items(node, &items, &|i| key[&i].clone(), &before);
                    if orders.contains(&next) {
                        break;
                    }
                    orders.push(next.clone());
                    current = next;
                }
            }
            let mut tried: Vec<Scored> = Vec::new();
            let mut seconds: Vec<Vec<(usize, Orient)>> = Vec::new();
            for order in &orders {
                if self.out_of_budget() && !tried.is_empty() {
                    break;
                }
                let mut p = Plan { order: self.place(base, order), ..base.clone() };
                p.items.insert(node, order.clone());
                let (p, second) = self.orient(&p, &classes);
                let (e, cand) = self.estimate(&p);
                let code = cand.code(s);
                found.push((e.clone(), code.clone(), p.clone()));
                tried.push((e, code, p));
                seconds.push(second);
            }
            // 3. the neighbourhood of the best: neighbours swapped, a class at its second best
            let Some(&b) = rank_of(s, &tried).first() else { continue };
            let (bp, second) = (tried.swap_remove(b).2, seconds.swap_remove(b));
            let border = bp.items.get(&node).cloned().unwrap_or_default();
            let before = precedences(s, &bp.flipped);
            for i in 1..border.len() {
                if self.out_of_budget() {
                    break;
                }
                let mut o = border.clone();
                o.swap(i - 1, i);
                let place = |it: Item| (Q::zero(), o.iter().position(|x| *x == it).unwrap_or(0));
                if self.order_items(node, &o, &place, &before) != o {
                    continue;
                }
                let mut p = Plan { order: self.place(&bp, &o), ..bp.clone() };
                p.items.insert(node, o);
                let (e, cand) = self.estimate(&p);
                found.push((e, cand.code(s), p));
            }
            for (c, v) in second {
                if self.out_of_budget() {
                    break;
                }
                let mut p = bp.clone();
                p.values[c] = v;
                let (e, cand) = self.estimate(&p);
                found.push((e, cand.code(s), p));
            }
            // and each fact whose side is open decided here reversed
            let reversed = self.reversed(node, &bp, &border);
            found.extend(reversed);
        }
        // ranked, each candidate once
        let mut codes: BTreeSet<Vec<u32>> = BTreeSet::new();
        let found: Vec<Scored> =
            ranked(s, found).into_iter().filter(|(_, code, _)| codes.insert(code.clone())).collect();
        // diversity: the best of each order of the items first, then the rest by rank
        let order_of = |p: &Plan| p.items.get(&node).cloned().unwrap_or_default();
        let mut picked: Vec<usize> = Vec::new();
        let mut orders_seen: Vec<Vec<Item>> = Vec::new();
        for (i, (_, _, p)) in found.iter().enumerate() {
            let o = order_of(p);
            if !orders_seen.contains(&o) {
                orders_seen.push(o);
                picked.push(i);
            }
        }
        picked.truncate(2 * beam);
        for i in 0..found.len() {
            if picked.len() >= 2 * beam {
                break;
            }
            if !picked.contains(&i) {
                picked.push(i);
            }
        }
        picked.sort_unstable();
        let mut found: Vec<Scored> = picked.into_iter().map(|i| found[i].clone()).collect();
        // 4. the best checked by the resolution with the node's relations
        found.truncate(2 * beam);
        let within: BTreeSet<usize> = n.members.iter().copied().collect();
        let subtree = self.subtree(node);
        let mut checked: Vec<(usize, usize)> = Vec::new();
        for (i, (_, _, p)) in found.iter().enumerate() {
            let c = self.candidate(p, "beam");
            let rejected = self.rejected(&c, &within);
            checked.push((usize::from(rejected > 0), i));
        }
        checked.sort();
        checked
            .into_iter()
            .take(beam)
            .map(|(_, i)| {
                let p = &found[i].2;
                let order = self.flatten(p, p.items.get(&node).map_or(&[][..], |v| v));
                // the decisions of this node and those below, and nothing else
                let values = (0..s.open.classes.len())
                    .filter(|&c| self.class_node(c).is_some_and(|n| subtree.contains(&n)))
                    .map(|c| (c, p.values[c]))
                    .collect();
                let items = p.items.iter().filter(|(n, _)| subtree.contains(n)).map(|(n, v)| (*n, v.clone())).collect();
                let flips = (0..s.reversible.len())
                    .filter(|&r| subtree.contains(&self.reversible_node(r)))
                    .map(|r| (r, p.flipped.contains(&r)))
                    .collect();
                Variant { order, values, items, flips }
            })
            .collect()
    }

    /// The plan `bp` with each fact whose side is open decided at `node` reversed in turn
    /// (`Structure::reversible`), its items in the order nearest to `border` that the reversed
    /// precedences allow, each with its estimate.
    fn reversed(&mut self, node: usize, bp: &Plan, border: &[Item]) -> Vec<Scored> {
        let s = self.s;
        let mut out = Vec::new();
        let local: Vec<usize> = (0..s.reversible.len()).filter(|&r| self.reversible_node(r) == node).collect();
        for r in local {
            if self.out_of_budget() {
                break;
            }
            let mut p = bp.clone();
            if !p.flipped.remove(&r) {
                p.flipped.insert(r);
            }
            let place = |it: Item| (Q::zero(), border.iter().position(|x| *x == it).unwrap_or(0));
            let o = self.order_items(node, border, &place, &precedences(s, &p.flipped));
            p.order = self.place(bp, &o);
            p.items.insert(node, o);
            let (e, cand) = self.estimate(&p);
            out.push((e, cand.code(s), p));
        }
        out
    }

    /// The global order with a node's elements in the order of its items.
    fn place(&self, plan: &Plan, items: &[Item]) -> Vec<usize> {
        let v = Variant {
            order: self.flatten(plan, items),
            values: BTreeMap::new(),
            items: BTreeMap::new(),
            flips: BTreeMap::new(),
        };
        self.with(plan, &v).order
    }

    /// The barycentre of each item: the mean x, on the sketch of the plan, of the pins of other
    /// elements that the pins of its elements are drawn to outside it; for an item drawn to nothing outside, the
    /// mean x of its own elements. With the inputs on the left in force, elements connected to an
    /// input port go first and those connected to an output port last.
    fn barycentre(&self, plan: &Plan, items: &[Item]) -> BTreeMap<Item, (Q, usize)> {
        let s = self.s;
        let cand = self.candidate(plan, "beam");
        let ori = cand.orientations(s);
        let at = sketch(self.problem, s, &cand, &ori);
        let mut out = BTreeMap::new();
        for &item in items {
            let members: Vec<usize> = match item {
                Item::Block(b) => s.tree.nodes[b].members.clone(),
                Item::Element(k) => vec![k],
            };
            let inside: BTreeSet<usize> = members.iter().copied().collect();
            let mut sum = Q::zero();
            let mut count = 0i64;
            let mut flow = 0i64;
            for &m in &members {
                for pin in 0..s.pins(m).len() {
                    for &t in s.connections.targets.get(&(m, pin)).map_or(&[][..], |v| v) {
                        if inside.contains(&t.0) {
                            continue;
                        }
                        if s.objects[t.0].role == Role::Port {
                            if let Some(port) = s.ports.iter().find(|p| p.object == t.0) {
                                flow += match port.kind {
                                    PortKind::Input => -1,
                                    PortKind::Output => 1,
                                    PortKind::Unknown => 0,
                                };
                            }
                            continue;
                        }
                        // stubs and attached symbols go where their element goes
                        if s.objects[t.0].role != Role::Element {
                            continue;
                        }
                        if let Some(q) = s.pin_at(&at, &ori, t.0, t.1) {
                            sum += &q[0];
                            count += 1;
                        }
                    }
                }
            }
            let mean = if count > 0 {
                sum / int(count)
            } else {
                members.iter().map(|&m| at[m][0].clone()).sum::<Q>() / int(members.len().max(1) as i64)
            };
            let key = if s.has(Source::InputsLeft) && flow != 0 {
                // far enough to come first or last whatever the barycentres
                let far: Q = at.iter().map(|p| p[0].abs()).max().unwrap_or_default() * int(4) + int(1000);
                if flow < 0 {
                    mean - far
                } else {
                    mean + far
                }
            } else {
                mean
            };
            out.insert(item, (key, members[0]));
        }
        out
    }

    /// How many of a candidate's relations among the elements of `within`, with its orientations,
    /// the resolution drops.
    fn rejected(&mut self, c: &Candidate, within: &BTreeSet<usize>) -> usize {
        let s = self.s;
        let ori = c.orientations(s);
        let mut rels = orientation_relations(s, c);
        rels.extend(relations_within(s, c, &ori, self.options, Some(within)));
        let budget = self.options.budget;
        with_relations(s, &rels, budget, &mut *self.work, |_, _, t| t.kept.iter().filter(|k| !**k).count())
    }
}

/// The order of scored plans, best first ([`crate::objectives::Active::rank`]).
fn rank_of(s: &Structure, list: &[Scored]) -> Vec<usize> {
    let entries: Vec<(&[Q], &[u32])> = list.iter().map(|(e, code, _)| (e.as_slice(), code.as_slice())).collect();
    s.active.rank(&entries)
}

/// Scored plans put in the order of [`rank_of`].
fn ranked(s: &Structure, list: Vec<Scored>) -> Vec<Scored> {
    let order = rank_of(s, &list);
    let mut slots: Vec<Option<Scored>> = list.into_iter().map(Some).collect();
    order.into_iter().filter_map(|i| slots[i].take()).collect()
}

/// The left-to-right precedences between elements: those of the facts with a side, and those of
/// the facts whose side is open, reversed for the facts of `flipped` (`Structure::reversible`).
fn precedences(s: &Structure, flipped: &BTreeSet<usize>) -> BTreeSet<(usize, usize)> {
    let mut out = s.precedence.clone();
    for (r, rev) in s.reversible.iter().enumerate() {
        let flip = flipped.contains(&r);
        out.extend(rev.pairs.iter().map(|&(a, b)| if flip { (b, a) } else { (a, b) }));
    }
    out
}

/// The plan of P0: the elements in the order of the netlist, as the precedences of the facts
/// allow, those left open as the witness has them; every free class R0.
fn p0_plan(s: &Structure) -> Plan {
    let before = precedences(s, &BTreeSet::new());
    let mut order: Vec<usize> = Vec::new();
    let mut done: BTreeSet<usize> = BTreeSet::new();
    while order.len() < s.elements.len() {
        let free = s.elements.iter().copied().find(|&k| {
            !done.contains(&k) && s.elements.iter().all(|&a| done.contains(&a) || !before.contains(&(a, k)))
        });
        let Some(k) = free.or_else(|| s.elements.iter().copied().find(|k| !done.contains(k))) else { break };
        done.insert(k);
        order.push(k);
    }
    Plan { order, values: vec![Orient::R0; s.open.classes.len()], items: BTreeMap::new(), flipped: BTreeSet::new() }
}

/// The candidates of a level (see the module): the root's variants best first, then P0, and with
/// the even rows in force a twin with even rows after each, none twice.
pub(crate) fn search(
    problem: &LevelProblem,
    s: &Structure,
    options: &SolverOptions,
    work: &mut Work,
) -> Vec<Candidate> {
    let plan = p0_plan(s);
    let mut search = Search { problem, s, options, made: 0, work };
    let p0 = search.candidate(&plan, "P0");
    let mut out: Vec<Candidate> = Vec::new();
    if !s.elements.is_empty() {
        let mut variants: BTreeMap<usize, Vec<Variant>> = BTreeMap::new();
        let mut current = plan.clone();
        for node in s.tree.post_order() {
            let vs = search.node(node, &current, &variants);
            if let Some(best) = vs.first() {
                current = search.with(&current, best);
            }
            variants.insert(node, vs);
        }
        for v in variants.get(&0).map_or(&[][..], |v| v) {
            let p = search.with(&plan, v);
            out.push(search.candidate(&p, "beam"));
        }
    }
    out.push(p0);
    if s.has(Source::EvenRows) && s.rows.iter().any(|r| r.len() >= 3) {
        let twins: Vec<Candidate> = out.iter().map(|c| Candidate { even: true, ..c.clone() }).collect();
        out = out.into_iter().zip(twins).flat_map(|(a, b)| [a, b]).collect();
    }
    let mut seen: Vec<Vec<u32>> = Vec::new();
    out.retain(|c| {
        let code = c.code(s);
        let new = !seen.contains(&code);
        seen.push(code);
        new
    });
    // the candidates estimated, and P0 and the twins, which are not
    let estimated = out.iter().filter(|c| c.origin == "beam" && !c.even).count() as u64;
    search.work.candidates += search.made + out.len() as u64 - estimated;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{instance, problem_of};

    #[test]
    fn a_relation_between_two_points_is_one_bound_and_between_sets_a_separator() {
        let p = problem_of(vec![instance(), instance(), instance()], Vec::new());
        let mut model = Model::new(&p, Q::zero());
        let o = PointId::Origin;
        let one = (Q::one(), Q::one());
        let rel = Relation::Apart {
            axis: 1,
            low: vec![(o(0), one.0.clone())],
            high: vec![(o(1), -one.1.clone())],
            gap: int(2),
        };
        let u = rel.unit(&mut model, 0);
        assert_eq!(u.fixed.len(), 1);
        // y1 − 1 ≥ y0 + 1 + 2
        assert_eq!(u.fixed[0].lo, Some(int(4)));
        assert_eq!(u.touched[1], vec![o(0), o(1)]);
        let vars = model.vars.len();
        let rel = Relation::Apart {
            axis: 0,
            low: vec![(o(0), int(1)), (o(1), int(1))],
            high: vec![(o(2), int(-3))],
            gap: int(2),
        };
        let u = rel.unit(&mut model, 1);
        assert_eq!(model.vars.len(), vars + 1);
        assert_eq!(u.fixed.len(), 3);
        let rel = Relation::OneOf(vec![
            vec![(0, vec![(o(1), int(1)), (o(0), int(-1))], int(3))],
            vec![(1, vec![(o(0), int(1))], int(0))],
        ]);
        let u = rel.unit(&mut model, 2);
        assert_eq!(u.choices.len(), 1);
        assert_eq!(u.choices[0].values.len(), 2);
        assert_eq!(u.choices[0].kind, ChoiceKind::Ranked);
        assert_eq!(u.touched, [vec![o(0), o(1)], vec![o(0)]]);
    }

    /// An ORDER given only an axis fixes no precedence; reversed, its members swap their order,
    /// the others staying where they were.
    #[test]
    fn a_fact_whose_side_is_open_is_reversed() {
        use rnl_runtime::problem::{Axis, FactKind, PointRef, Sense};
        let o = PointRef::Origin;
        let order = FactKind::Order { axis: Axis::X, sense: Sense::Either, members: vec![o(0), o(2)] };
        let p = problem_of(vec![instance(), instance(), instance()], vec![crate::resolve::tests::fact(order)]);
        let options = SolverOptions::default();
        let mut work = Work::default();
        let resolved = crate::resolve::resolve(&p, &options);
        let s = crate::structure::analyse(&p, &resolved, &options, &mut work);
        assert!(s.precedence.is_empty());
        assert_eq!(s.reversible.len(), 1);
        let (a, b) = s.reversible[0].pairs[0];
        let mut search = Search { problem: &p, s: &s, options: &options, made: 0, work: &mut work };
        let border = vec![Item::Element(0), Item::Element(1), Item::Element(2)];
        let mut plan = p0_plan(&s);
        plan.items.insert(0, border.clone());
        let reversed = search.reversed(0, &plan, &border);
        assert_eq!(reversed.len(), 1);
        let q = &reversed[0].2;
        assert_eq!(q.flipped, BTreeSet::from([0]));
        let at = |k: usize| q.order.iter().position(|&j| j == k).unwrap();
        // the witness had a before b: now b goes first
        assert!(at(b) < at(a), "{:?}", q.order);
        // and reversed again, back as the witness had it
        let again = search.reversed(0, q, &q.items[&0].clone());
        assert!(again[0].2.flipped.is_empty());
        let at = |k: usize| again[0].2.order.iter().position(|&j| j == k).unwrap();
        assert!(at(a) < at(b), "{:?}", again[0].2.order);
    }

    #[test]
    fn constants_go_up_to_the_grid() {
        let g = Some(int(1));
        assert_eq!(up(crate::rational::frac(16, 5), &g), int(4));
        assert_eq!(up(int(3), &g), int(3));
        assert_eq!(up(crate::rational::frac(-1, 2), &g), int(0));
        assert_eq!(up(crate::rational::frac(16, 5), &None), crate::rational::frac(16, 5));
        assert_eq!(up(crate::rational::frac(3, 4), &Some(crate::rational::frac(1, 2))), int(1));
    }
}
