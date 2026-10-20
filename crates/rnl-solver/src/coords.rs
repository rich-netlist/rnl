//! The coordinates of a candidate: its relations taken in by the resolution after the facts kept
//! (8.5), a lexicographic LP on each axis, then rounding to the grid where the facts allow it
//! (plan 5.6; standard 11.7).
//!
//! **Taking the candidate in** ([`place`]). The orientations of its free classes are taken in
//! first, after the facts kept, and the orientations that result are read back: a value the
//! facts do not allow is dropped, never a fact. The other relations are made for those
//! orientations (`crate::search::relations`) and taken in after them, each dropped when it does
//! not fit; the simplex of each axis then holds the facts kept, the literals at their values and
//! the relations kept. The first LP keeps neighbouring rows apart as wholes, the lowest point of
//! one row a gap above the highest of the next; then the rows are kept in the order of their
//! origins, and only the pairs of elements of two rows whose parts that LP made neighbours on x
//! are kept a gap apart on y, so that a row rises into the gaps of the one above, beside what is
//! there, but over the span of each wire that the router draws between two rows, where they stay
//! apart as wholes (`crate::search::row_pairs`; quality review SE-4 and the review of its phase
//! 2); placed again, at most twice more as new pairs come.
//!
//! **The LP** (each axis on its own, as the axes are independent once the orientations are
//! decided, algorithm synthesis 3.2). Every pin and text the model has is first linked to its
//! object's origin by the orientation, `x_pin − x_origin = c(orientation)`, so that all of it is
//! one rigid body. Then, one layer after the other, each optimum fixed as a bound before the
//! next:
//!
//! 1. the wire length: for each net drawn as wires, the extent of its points, `hi − lo` with
//!    `lo ≤ xₚ ≤ hi`, summed: half the perimeter of its box, the stand-in for its length; a pin
//!    of a net routed counts by the end of its lead, a unit its own way, as the router leaves it
//!    (quality review LP-1);
//! 2. compactness: the extent of the boxes of the components;
//! 3. the nets that only stubs draw, the supply and the ground that no ROUTE draws: for each, the
//!    extent of the pins of the elements on it, summed, so that elements that only stubs join, a
//!    supply source and what it feeds, are not left far apart where the wires and the envelope
//!    leave room (quality review LP-2); after the wires drawn, as it weighs less than they do;
//! 4. the L1 distance of each origin to where the resolution of the candidate put it: the
//!    tie-break that makes the result stable. A symbol's target is that place rounded to the grid;
//!    a point's is the place itself, as it mostly follows a symbol. A text attached to a point or
//!    to an object with a coordinate point has its target there instead, `x_text − x_anchor = c`,
//!    at the point or beside the object ([`crate::structure::Anchor`], 13.3), so that a text no
//!    fact places goes beside what it is attached to; one attached to a wire, on the axis that no
//!    fact reaches, above the midpoint of two of its points, `x_text − (x_a + x_b) / 2 = c`.
//!
//! **Rounding.** The grid is the solver's own rule, ranked after every description (11.7), and
//! also after the layers above: with the layers' bounds taken away, the origins are visited, the
//! symbols first in the order of the objects, then the points and texts, which mostly follow a
//! symbol. A text on an axis that no fact kept reaches is not visited: it is held at its anchor
//! there, where the viewer puts it (13.3), and follows what the grid moves. Each origin visited
//! goes to the nearer grid point beside it when the facts and relations allow, or else to the
//! other, and each that ends up on the grid is held there, so that a later one cannot move it off
//! again. A symbol that fits on neither is tried once more before the others held in
//! the visit, which are then held again on the grid (a centre is tied to its set). Then the layers
//! are solved again with the origins moved kept, and the rounding done once more. A coordinate
//! that the facts force off the grid stays as they force it. The constants of the relations are
//! themselves on the grid ([`crate::search`]), those that put overlapping symbols apart too, so
//! that the LP's vertices mostly are.
//!
//! **Overlaps.** Not overlapping is the solver's own rule too (11.7): two symbols whose boxes
//! still overlap are put apart by a relation with four sides to choose from, the one that moves
//! least first, ranked after the facts and before the structural relations, so that it takes
//! precedence over them; then the candidate is taken in again, for a few rounds. Two symbols that
//! still overlap do so for a reason ([`Cause`]): the facts kept leave no side, the symbols put
//! apart before them leave none, the rounds ran out, or a cap of work did.

use std::collections::{BTreeMap, BTreeSet};

use num_traits::{One, Zero};
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::LevelProblem;

use crate::model::{turned, Model, PointId};
use crate::rational::{from_f64, int, Q};
use crate::resolve::{Geometry, Resolver, Tag};
use crate::route::CLEARANCE;
use crate::search::{
    orientation_relations, relations, relations_by_pairs, row_pairs, up, with_placement_seed, with_relations,
    Candidate, RelKind, Relation, RowPair, Tagged,
};
use crate::simplex::{LinExpr, Rel, Var};
use crate::structure::{Drawn, Role, Structure};
use crate::{SolverOptions, Work};

pub(crate) mod affine;
use affine::Axis;

/// How many rounds of putting overlapping symbols apart.
const REPAIR_ROUNDS: usize = 6;

/// How many times, after the first LP, the rows are put apart pair by pair again with the pairs
/// that the last LP made neighbours on x (quality review SE-4).
const ROW_ROUNDS: usize = 2;

/// Why two symbols still overlap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    /// The facts kept, with the orientations taken, leave no side to put them apart.
    Facts,
    /// A side fits with the facts kept, but not with the symbols put apart before them.
    Others,
    /// The rounds of putting symbols apart ran out before they had their turn.
    Rounds,
    /// A cap of work ran out while the candidate was taken in or placed (`Structure::caps`), so a
    /// relation may have been dropped without a conflict.
    Budget,
}

/// Two symbols that overlap, the earlier first, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Overlap {
    pub a: usize,
    pub b: usize,
    pub cause: Cause,
}

/// A candidate placed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Placed {
    /// The origin of each object with a position of its own, and every orientation: the facts
    /// kept hold there, exactly.
    pub geometry: Geometry,
    /// The origin of every object (`Structure::origins`).
    pub origins: Vec<[Q; 2]>,
    /// The relations the resolution dropped, by kind, with repeats.
    pub dropped: Vec<RelKind>,
    /// The pairs of symbols that still overlap.
    pub overlaps: Vec<Overlap>,
    /// Whether the LP ran: when its cap on pivots ran out, the placement is where the resolution of the
    /// candidate put it.
    pub optimised: bool,
    /// Whether a cap of work ran out deciding a relation of the candidate, which was then dropped
    /// without a conflict.
    pub undecided: bool,
}

/// Places a candidate (see the module).
pub(crate) fn place(
    problem: &LevelProblem,
    s: &Structure,
    c: &Candidate,
    options: &SolverOptions,
    work: &mut Work,
) -> Placed {
    let grid = options.grid.and_then(from_f64).filter(|g| *g > Q::zero());
    let phase = crate::profile::work_span("coords_work", "orientation");
    let orient = orientation_relations(s, c);
    let actual: Vec<Orient> = with_relations(s, &orient, work, "orientation", |r, _, _| r.geometry().orient.clone());
    drop(phase);
    let mut separations: Vec<Tagged> = Vec::new();
    let mut separated: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut round = 0;
    // the rows apart as wholes in the first LP, then pair by pair (quality review SE-4)
    let mut pairs: Option<Vec<RowPair>> = None;
    let mut row_rounds = 0;
    // Local to this candidate. A failed round clears it; the last round never stores one.
    let mut next_seed = None;
    loop {
        let phase = crate::profile::work_span("coords_work", "relations_build");
        crate::profile::work_set("work_repair_round", round as u64);
        crate::profile::work_set("work_row_round", row_rounds as u64);
        let mut rels = orient.clone();
        rels.extend(separations.iter().cloned());
        rels.extend(match &pairs {
            None => relations(s, c, &actual, options),
            Some(pairs) => relations_by_pairs(s, c, &actual, options, pairs),
        });
        drop(phase);
        let phase = crate::profile::work_span("coords_work", "placement");
        crate::profile::work_set("work_repair_round", round as u64);
        crate::profile::work_set("work_row_round", row_rounds as u64);
        let seed = next_seed.take();
        let (geometry, kept, undecided, optimised, seed_complete) =
            with_placement_seed(s, &rels, work, seed.as_ref(), &actual, |r, model, t| {
                let (g, optimised, seed_complete) = optimise(r, model, s, options);
                (g, t.kept.clone(), t.undecided, optimised, seed_complete)
            });
        drop(phase);
        let _post = crate::profile::work_span("coords_work", "post_geometry");
        let origins = s.origins(problem, &geometry);
        // the pairs of the rows that this LP made neighbours on x, added to those before, each
        // with the larger gap it asks; only for a candidate with rows to put apart, and while
        // the LP was solved
        let mut rows_again = false;
        if c.rows.len() > 1 && optimised && row_rounds < ROW_ROUNDS {
            let mut all: BTreeMap<(usize, usize), Q> =
                pairs.iter().flatten().map(|p| ((p.upper, p.lower), p.least.clone())).collect();
            let before = all.clone();
            for p in row_pairs(s, c, &geometry.orient, &origins, options) {
                let least = all.entry((p.upper, p.lower)).or_insert_with(|| p.least.clone());
                if p.least > *least {
                    *least = p.least;
                }
            }
            if pairs.is_none() || all != before {
                pairs = Some(all.into_iter().map(|((upper, lower), least)| RowPair { upper, lower, least }).collect());
                row_rounds += 1;
                rows_again = true;
            }
        }
        let overlaps = overlapping(s, &origins, &geometry.orient);
        let new: Vec<(usize, usize)> = overlaps.iter().copied().filter(|p| !separated.contains(p)).collect();
        if rows_again && round < REPAIR_ROUNDS {
            // placed again with the rows apart pair by pair before any overlap is repaired: the
            // overlaps of this placement may not be those of the next
            next_seed = subsequent_seed(geometry, optimised, undecided, seed_complete);
            continue;
        }
        if new.is_empty() || round >= REPAIR_ROUNDS {
            let dropped = rels.iter().zip(&kept).filter(|(_, k)| !**k).map(|(t, _)| t.kind).collect();
            let overlaps = overlaps
                .into_iter()
                .map(|(a, b)| {
                    let cause = if undecided || !optimised {
                        Cause::Budget
                    } else if !separated.contains(&(a, b)) {
                        Cause::Rounds
                    } else {
                        apart_cause(s, &origins, &geometry.orient, a, b, &grid, work)
                    };
                    Overlap { a, b, cause }
                })
                .collect();
            return Placed { geometry, origins, dropped, overlaps, optimised, undecided };
        }
        for (a, b) in new {
            separated.insert((a, b));
            if let Some(t) = separation(s, &origins, &geometry.orient, a, b, &grid) {
                separations.push(t);
            }
        }
        round += 1;
        next_seed = subsequent_seed(geometry, optimised, undecided, seed_complete);
    }
}

/// Called only when another round will run. Do not retain an older seed after a failed round,
/// and never retry an unsuccessful solve with a second, cold resolution.
fn subsequent_seed(geometry: Geometry, optimised: bool, undecided: bool, seed_complete: bool) -> Option<Geometry> {
    if optimised && !undecided && seed_complete {
        Some(geometry)
    } else {
        if crate::profile::work_enabled() {
            crate::profile::add("coords_seed_skipped_incomplete", 1);
        }
        None
    }
}

/// Why two symbols that were given a relation to put them apart still overlap: whether the facts
/// kept, with the orientations as placed, leave a side for them at all.
fn apart_cause(
    s: &Structure,
    origins: &[[Q; 2]],
    ori: &[Orient],
    a: usize,
    b: usize,
    grid: &Option<Q>,
    work: &mut Work,
) -> Cause {
    let Some(apart) = separation(s, origins, ori, a, b, grid) else { return Cause::Facts };
    let mut rels: Vec<Tagged> = s
        .open
        .classes
        .iter()
        .map(|c| Tagged { kind: RelKind::Orientation, rel: Relation::Orient { root: c.root, value: ori[c.root] } })
        .collect();
    rels.push(apart);
    let (fits, undecided) =
        with_relations(s, &rels, work, "apart_cause", |_, _, t| (t.kept[t.kept.len() - 1], t.undecided));
    match (fits, undecided) {
        (true, _) => Cause::Others,
        (false, true) => Cause::Budget,
        (false, false) => Cause::Facts,
    }
}

/// The symbols placed by their own position: elements, supply, ground and port symbols, and other
/// virtual parts with a position of their own; not those that the solution leaves free
/// (`crate::structure::ObjectInfo::left_free`), which are no part of the drawing.
pub(crate) fn solid(s: &Structure, k: usize) -> bool {
    let o = &s.objects[k];
    o.own && o.symbol.is_some() && !o.left_free && !matches!(o.role, Role::Point | Role::Text)
}

/// The pairs of symbols whose boxes overlap with some area, the earlier first.
pub(crate) fn overlapping(s: &Structure, origins: &[[Q; 2]], ori: &[Orient]) -> Vec<(usize, usize)> {
    let boxes: Vec<(usize, [Q; 4])> =
        (0..s.objects.len()).filter(|&k| solid(s, k)).filter_map(|k| Some((k, s.box_at(origins, ori, k)?))).collect();
    let mut out = Vec::new();
    for i in 0..boxes.len() {
        for j in i + 1..boxes.len() {
            let (a, b) = (&boxes[i].1, &boxes[j].1);
            let x = a[0].clone().max(b[0].clone()) < a[2].clone().min(b[2].clone());
            let y = a[1].clone().max(b[1].clone()) < a[3].clone().min(b[3].clone());
            if x && y {
                out.push((boxes[i].0, boxes[j].0));
            }
        }
    }
    out
}

/// One side two symbols may be put apart on: the axis, the form, its least value touching, and its
/// value now.
type SideBound = (usize, Vec<(PointId, Q)>, Q, Q);

/// The relation that puts two symbols apart: `b` right of, left of, above or below `a`, a unit
/// apart, those that move least first, then the same touching; each distance rounded up to the
/// grid.
pub(crate) fn separation(
    s: &Structure,
    origins: &[[Q; 2]],
    ori: &[Orient],
    a: usize,
    b: usize,
    grid: &Option<Q>,
) -> Option<Tagged> {
    let (pa, pb) = (s.model.origin_point.get(a).copied().flatten()?, s.model.origin_point.get(b).copied().flatten()?);
    let ba = s.box_offset(a, ori[a])?;
    let bb = s.box_offset(b, ori[b])?;
    let one = Q::one;
    let b_minus_a = || vec![(pb, one()), (pa, -one())];
    let a_minus_b = || vec![(pa, one()), (pb, -one())];
    let d = |axis: usize| &origins[b][axis] - &origins[a][axis];
    // (axis, terms, the constant at no gap, the value now)
    let sides: [SideBound; 4] = [
        (0, b_minus_a(), &ba[2] - &bb[0], d(0)),
        (0, a_minus_b(), &bb[2] - &ba[0], -d(0)),
        (1, b_minus_a(), &ba[3] - &bb[1], d(1)),
        (1, a_minus_b(), &bb[3] - &ba[1], -d(1)),
    ];
    let mut order: Vec<usize> = (0..4).collect();
    order.sort_by_key(|&i| (&sides[i].2 - &sides[i].3, i));
    let mut alternatives = Vec::new();
    for gap in [one(), Q::zero()] {
        for &i in &order {
            let (axis, terms, c, _) = &sides[i];
            // on the grid, as the constants of the other relations are
            alternatives.push(vec![(*axis, terms.clone(), up(c + &gap, grid))]);
        }
    }
    Some(Tagged { kind: RelKind::Separation, rel: Relation::OneOf(alternatives) })
}

/// `v` on the nearest point of the grid, halves up.
fn round(v: &Q, grid: &Option<Q>) -> Q {
    match grid {
        Some(g) => (v / g + Q::new(1.into(), 2.into())).floor() * g,
        None => v.clone(),
    }
}

/// Every pin and text of the model linked to its object's origin by its orientation, a text only
/// on the axes its owner's symbol gives (13.3); then a check.
fn link(sx: &mut Axis<Tag>, model: &Model, ori: &[Orient], axis: usize) -> bool {
    for (id, info) in &model.points {
        let (Some(k), Some(local), true) = (info.object, &info.local, info.linked(axis)) else { continue };
        let Some(base) = model.origin_point.get(k).copied().flatten() else { continue };
        if base == *id {
            continue;
        }
        let c = turned(ori[k], local, axis);
        if sx.assert(&LinExpr::diff(info.var, model.var(base)), Rel::Eq, &c, Tag::Def).is_err() {
            return false;
        }
    }
    sx.check().is_ok()
}

/// Minimises a form and keeps its optimum for the layers after it (`Simplex::keep_optimum`).
fn minimise_and_keep(sx: &mut Axis<Tag>, objective: &LinExpr) -> Result<(), ()> {
    if objective.is_constant() {
        return Ok(());
    }
    sx.minimize(objective).map_err(|_| ())?;
    sx.keep_optimum(objective, Tag::Def).map_err(|_| ())
}

/// Where layer 4 of the LP pulls an origin (see the module).
enum Target {
    /// To a place.
    At(Q),
    /// To a place relative to other variables, the mean of them: `v − Σ base / n = c`.
    From(Vec<Var>, Q),
}

/// The target of each origin on an axis: a text's at its anchor, the others where the resolution
/// of the candidate put them, a symbol's on the grid.
fn targets(
    model: &Model,
    s: &Structure,
    own: &[(usize, Var)],
    fallback: &Geometry,
    grid: &Option<Q>,
    axis: usize,
) -> Vec<Target> {
    let ori = &fallback.orient;
    own.iter()
        .map(|(k, _)| {
            let anchored = s.anchors.get(k).and_then(|a| {
                let mut bases = Vec::new();
                let mut c = Q::zero();
                for &(object, pin) in &a.from {
                    bases.push(model.points.get(&model.origin_point.get(object).copied().flatten()?)?.var);
                    if let Some(pin) = pin {
                        c += &s.pin_offset(object, pin, ori[object])?[axis];
                    }
                }
                let n = int(i64::try_from(bases.len()).ok()?);
                (!bases.is_empty()).then(|| Target::From(bases, c / n + &a.offset[axis]))
            });
            let at = &fallback.origin[*k][axis];
            anchored.unwrap_or_else(|| Target::At(if solid(s, *k) { round(at, grid) } else { at.clone() }))
        })
        .collect()
}

/// Where the wire of a net drawn leaves a point of it, from the point, on an axis: for a pin of a
/// net routed, the end of the lead the router gives it, `CLEARANCE` its own way (`crate::route`),
/// so that the wire length sees the L that a pin leaving up and one leaving sideways towards it
/// make, rather than the jog it would take to join them at one height (quality review LP-1); the
/// point itself otherwise. Linear, as the orientations are decided.
fn lead(s: &Structure, ori: &[Orient], d: &Drawn, p: PointId, axis: usize) -> Q {
    match p {
        PointId::Pin(k, pin) if d.routed => match s.exit(k, pin, ori[k]) {
            Some((x, y)) => int(CLEARANCE * if axis == 0 { x } else { y }),
            None => Q::zero(),
        },
        _ => Q::zero(),
    }
}

/// The pins of the elements on each supply or ground net that no ROUTE draws, by the variable of
/// the element's origin and the pin's offset from it on an axis: such a net is drawn by the stubs
/// of its pins alone, and the wire length does not see it. Those with pins of two elements or
/// more, in the order of the nets.
fn stub_nets(model: &Model, s: &Structure, ori: &[Orient], axis: usize) -> Vec<BTreeSet<(Var, Q)>> {
    let routed: BTreeSet<&str> = s.connections.drawn.iter().filter(|d| d.routed).map(|d| d.net.as_str()).collect();
    let mut out = Vec::new();
    for net in &s.connections.nets {
        if !(net.supply || net.ground) || routed.contains(net.name.as_str()) {
            continue;
        }
        let ends: BTreeSet<(Var, Q)> = net
            .pins
            .iter()
            .filter(|(k, _)| s.objects[*k].role == Role::Element)
            .filter_map(|&(k, pin)| {
                let origin = model.origin_point.get(k).copied().flatten()?;
                Some((model.points.get(&origin)?.var, s.pin_offset(k, pin, ori[k])?[axis].clone()))
            })
            .collect();
        if ends.iter().map(|(v, _)| v).collect::<BTreeSet<_>>().len() >= 2 {
            out.push(ends);
        }
    }
    out
}

/// The four layers of the LP on one axis (see the module).
fn layers(
    sx: &mut Axis<Tag>,
    model: &Model,
    s: &Structure,
    ori: &[Orient],
    own: &[(usize, Var)],
    targets: &[Target],
    axis: usize,
) -> Result<(), ()> {
    let fail = |_| ();
    let phase = crate::profile::span("lex_layer", "1_wire_length");
    // 1. the wire length: a pin of a net routed by the end of its lead
    let mut objective = LinExpr::default();
    for d in &s.connections.drawn {
        let ends: BTreeSet<(Var, Q)> =
            d.points.iter().filter_map(|p| Some((model.points.get(p)?.var, lead(s, ori, d, *p, axis)))).collect();
        if ends.iter().map(|(v, _)| v).collect::<BTreeSet<_>>().len() < 2 {
            continue;
        }
        let (hi, lo) = (sx.new_var(), sx.new_var());
        // hi ≥ v + c and lo ≤ v + c
        for (v, c) in &ends {
            sx.assert(&LinExpr::diff(hi, *v), Rel::Ge, c, Tag::Def).map_err(fail)?;
            sx.assert(&LinExpr::diff(*v, lo), Rel::Ge, &-c.clone(), Tag::Def).map_err(fail)?;
        }
        objective.add_term(hi, &Q::one());
        objective.add_term(lo, &-Q::one());
    }
    minimise_and_keep(sx, &objective)?;
    drop(phase);
    let phase = crate::profile::span("lex_layer", "2_envelope");
    // 2. the envelope of the components
    let boxes: Vec<(Var, [Q; 4])> =
        own.iter().filter(|(k, _)| solid(s, *k)).filter_map(|&(k, v)| Some((v, s.box_offset(k, ori[k])?))).collect();
    if !boxes.is_empty() {
        let (hi, lo) = (sx.new_var(), sx.new_var());
        for (v, b) in &boxes {
            sx.assert(&LinExpr::diff(hi, *v), Rel::Ge, &b[axis + 2], Tag::Def).map_err(fail)?;
            sx.assert(&LinExpr::diff(*v, lo), Rel::Ge, &-b[axis].clone(), Tag::Def).map_err(fail)?;
        }
        minimise_and_keep(sx, &LinExpr::diff(hi, lo))?;
    }
    drop(phase);
    let phase = crate::profile::span("lex_layer", "3_stub_extent");
    // 3. the nets that only stubs draw: the extent of the pins of the elements on each
    let mut objective = LinExpr::default();
    for ends in stub_nets(model, s, ori, axis) {
        let (hi, lo) = (sx.new_var(), sx.new_var());
        for (v, c) in &ends {
            sx.assert(&LinExpr::diff(hi, *v), Rel::Ge, c, Tag::Def).map_err(fail)?;
            sx.assert(&LinExpr::diff(*v, lo), Rel::Ge, &-c.clone(), Tag::Def).map_err(fail)?;
        }
        objective.add_term(hi, &Q::one());
        objective.add_term(lo, &-Q::one());
    }
    minimise_and_keep(sx, &objective)?;
    drop(phase);
    let _phase = crate::profile::span("lex_layer", "4_origin_distance");
    // 4. the distance to where the resolution put each origin, or to a text's anchor:
    // d ≥ e − c and d ≥ c − e, for e = v or e = v − base
    let mut objective = LinExpr::default();
    for ((_, v), t) in own.iter().zip(targets) {
        let d = sx.new_var();
        let (bases, c) = match t {
            Target::At(c) => (&[][..], c),
            Target::From(bases, c) => (&bases[..], c),
        };
        let mut below = LinExpr::from_terms([(d, Q::one()), (*v, -Q::one())], Q::zero());
        let mut above = LinExpr::from_terms([(d, Q::one()), (*v, Q::one())], Q::zero());
        let share = Q::one() / int(i64::try_from(bases.len().max(1)).unwrap_or(1));
        for b in bases {
            below.add_term(*b, &share);
            above.add_term(*b, &-share.clone());
        }
        sx.assert(&below, Rel::Ge, &-c.clone(), Tag::Def).map_err(fail)?;
        sx.assert(&above, Rel::Ge, c, Tag::Def).map_err(fail)?;
        objective.add_term(d, &Q::one());
    }
    minimise_and_keep(sx, &objective)
}

/// Holds `v` at `r` when the bounds allow it, and says whether they did.
fn try_pin(sx: &mut Axis<Tag>, v: Var, r: &Q) -> bool {
    let cp = sx.checkpoint();
    if sx.assert(&LinExpr::var(v), Rel::Eq, r, Tag::Def).is_ok() && sx.check().is_ok() {
        return true;
    }
    sx.backtrack(cp);
    let _ = sx.check();
    false
}

/// The grid points to try for a value: itself when it is on the grid; otherwise the nearer grid
/// point beside it, then the other.
fn grid_points(x: &Q, grid: &Q) -> Vec<Q> {
    let near = round(x, &Some(grid.clone()));
    if near == *x {
        return vec![near];
    }
    let far = if near > *x { &near - grid } else { &near + grid };
    vec![near, far]
}

/// An origin to put on the grid: the object, its variable on the axis, and whether it is a symbol.
type Visit = (usize, Var, bool);

/// Puts origins on the grid (see the module), in the order given, and holds each one visited that
/// ends up on it there, so that a later one put on the grid cannot move it off again. An origin
/// off the grid goes to the nearer grid point beside it when the bounds allow it, otherwise to the
/// other. When they allow neither and it is a symbol, the origins held in this call are let go, it
/// is put on the grid first, and they are held again, each where it was or on a grid point beside
/// where it is now: the facts may tie it to them, as a centre is tied to its set. When one of them
/// can no longer be on the grid, all is as before, and the origin stays where it is, free. `kept`
/// are the origins held on the grid already, which are left as they are. Gives the origins moved
/// onto the grid, with where.
fn round_to_grid(sx: &mut Axis<Tag>, order: &[Visit], grid: &Q, kept: &BTreeSet<usize>) -> Vec<(usize, Q)> {
    let start = sx.checkpoint();
    // the origins held in this call, in the order they were held, with where
    let mut held: Vec<(usize, Var, Q)> = Vec::new();
    let mut moved: BTreeMap<usize, Q> = BTreeMap::new();
    for &(k, v, symbol) in order {
        if kept.contains(&k) {
            continue;
        }
        let x = sx.value(v).clone();
        let points = grid_points(&x, grid);
        if let Some(r) = points.iter().find(|r| try_pin(sx, v, r)) {
            if *r != x {
                moved.insert(k, r.clone());
            }
            held.push((k, v, r.clone()));
            continue;
        }
        if !symbol || held.is_empty() {
            continue;
        }
        // this one first, then those held before it again
        sx.backtrack(start);
        let _ = sx.check();
        let mut again: Vec<(usize, Var, Q)> = Vec::new();
        if let Some(r) = points.iter().find(|r| try_pin(sx, v, r)) {
            again.push((k, v, r.clone()));
            for (j, w, at) in &held {
                let now = sx.value(*w).clone();
                let mut tries = vec![at.clone()];
                tries.extend(grid_points(&now, grid).into_iter().filter(|q| q != at));
                match tries.into_iter().find(|q| try_pin(sx, *w, q)) {
                    Some(q) => again.push((*j, *w, q)),
                    None => break,
                }
            }
        }
        if again.len() == held.len() + 1 {
            for (j, _, q) in &again {
                let before = held.iter().find(|(i, _, _)| i == j).map(|(_, _, at)| at);
                if before != Some(q) {
                    moved.insert(*j, q.clone());
                }
            }
            held = again;
        } else {
            // as before
            sx.backtrack(start);
            for (_, w, at) in &held {
                let _ = sx.assert(&LinExpr::var(*w), Rel::Eq, at, Tag::Def);
            }
            let _ = sx.check();
        }
    }
    moved.into_iter().collect()
}

/// The coordinates of what a resolution holds (see the module); the resolution's own witness when
/// the LP cannot finish, with false.
fn optimise(r: &mut Resolver, model: &Model, s: &Structure, options: &SolverOptions) -> (Geometry, bool, bool) {
    let fallback = r.geometry().clone();
    let ori = fallback.orient.clone();
    let grid = options.grid.and_then(from_f64).filter(|g| *g > Q::zero());
    let own: Vec<(usize, Var)> = (0..s.objects.len())
        .filter(|&k| s.objects[k].own)
        .filter_map(|k| Some((k, model.points.get(&PointId::Origin(k))?.var)))
        .collect();
    // the symbols go on the grid first, then the points and texts, which mostly follow a symbol; a
    // text on an axis that no fact kept reaches stays at its anchor, where the viewer puts it (13.3)
    let at_anchor = |k: usize, axis: usize| s.anchors.contains_key(&k) && !s.objects[k].fact_axes[axis];
    let mut out = fallback.clone();
    // The old optimised flag also permits feasible results after a failed layer. Track seed
    // eligibility separately without changing the old flow, result, or failure handling.
    let mut all_layers_completed = true;
    for axis in 0..2 {
        let _lp = crate::profile::span_lazy("lp", || format!("axis_{axis}"));
        crate::profile::add("lps", 1);
        crate::profile::set("axis", axis as u64);
        let mut order: Vec<Visit> = own.iter().filter(|(k, _)| solid(s, *k)).map(|&(k, v)| (k, v, true)).collect();
        order.extend(own.iter().filter(|(k, _)| !solid(s, *k) && !at_anchor(*k, axis)).map(|&(k, v)| (k, v, false)));
        let sx = r.simplex_mut(axis);
        sx.profile_tableau();
        let linked = link(sx, model, &ori, axis);
        if !linked {
            return (fallback, false, false);
        }
        // where the resolution of the candidate put each origin, a symbol's on the grid; a text or a
        // point follows what it is placed from, and a target rounded on its own would pull that
        // off the grid; a text goes to its anchor
        let targets = targets(model, s, &own, &fallback, &grid, axis);
        // a text on an axis that no fact kept reaches is held at its anchor, which nothing else
        // there bounds, so that what the grid moves takes it along
        for ((k, v), t) in own.iter().zip(&targets) {
            let (true, Target::From(bases, c)) = (at_anchor(*k, axis), t) else { continue };
            let share = Q::one() / int(i64::try_from(bases.len()).unwrap_or(1));
            let mut e = LinExpr::var(*v);
            for b in bases {
                e.add_term(*b, &-share.clone());
            }
            let cp = sx.checkpoint();
            if sx.assert(&e, Rel::Eq, c, Tag::Def).is_err() || sx.check().is_err() {
                sx.backtrack(cp);
                let _ = sx.check();
            }
        }
        let base = sx.checkpoint();
        let mut moved: Vec<(usize, Q)> = Vec::new();
        // two passes at most: the layers, the grid, and once more when the grid moved something
        for _ in 0..2 {
            // the origins the grid moved in the pass before stay where it put them; the others are
            // free for the layers again
            sx.backtrack(base);
            for (k, at) in &moved {
                let Some(&(_, v)) = own.iter().find(|(j, _)| j == k) else { continue };
                if sx.assert(&LinExpr::var(v), Rel::Eq, at, Tag::Def).is_err() {
                    return (fallback, false, false);
                }
            }
            if sx.check().is_err() {
                return (fallback, false, false);
            }
            let cp = sx.checkpoint();
            let solved = layers(sx, model, s, &ori, &own, &targets, axis).is_ok();
            all_layers_completed &= solved;
            sx.backtrack(cp);
            if !solved && sx.check().is_err() {
                return (fallback, false, false);
            }
            let Some(g) = &grid else { break };
            let kept: BTreeSet<usize> = moved.iter().map(|(k, _)| *k).collect();
            let now = round_to_grid(sx, &order, g, &kept);
            // the layers again only when the grid moved something
            if now.is_empty() {
                break;
            }
            moved.extend(now);
        }
        if sx.check().is_err() {
            return (fallback, false, false);
        }
        for (k, v) in &own {
            out.origin[*k][axis] = sx.value(*v).clone();
        }
        // a text on the axis that the scene gives it has its own coordinate there (13.3)
        for (id, info) in &model.points {
            if let (PointId::Text(t), Some(free)) = (id, info.free) {
                if free == axis {
                    out.origin[*t][axis] = sx.value(info.var).clone();
                }
            }
        }
    }
    // Grid/try_pin may ignore a failed check. Any Budget there requires the monotonic axis
    // pivot count to have reached the same cap; conservatively exclude equality as well.
    let below_caps = (0..2).all(|axis| r.simplex_mut(axis).pivots() < s.caps.pivots);
    (out, true, all_layers_completed && below_caps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::{frac, int};

    #[test]
    fn halves_go_up_to_the_grid() {
        let g = Some(int(1));
        assert_eq!(round(&frac(9, 2), &g), int(5));
        assert_eq!(round(&frac(-9, 2), &g), int(-4));
        assert_eq!(round(&frac(21, 5), &g), int(4));
        assert_eq!(round(&frac(21, 5), &None), frac(21, 5));
        assert_eq!(round(&frac(13, 10), &Some(frac(1, 2))), frac(3, 2));
    }
}

#[cfg(test)]
#[path = "coords/geometry_seed_tests.rs"]
mod geometry_seed_tests;
