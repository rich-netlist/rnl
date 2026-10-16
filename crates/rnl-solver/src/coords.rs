//! The coordinates of a candidate: its relations taken in by the resolution after the facts kept
//! (8.5), a lexicographic LP on each axis, then rounding to the grid where the facts allow it
//! (plan 5.6; standard 11.7).
//!
//! **Taking the candidate in** ([`place`]). The orientations of its free classes are taken in
//! first, after the facts kept, and the orientations that result are read back: a value the
//! facts do not allow is dropped, never a fact. The other relations are made for those
//! orientations (`crate::search::relations`) and taken in after them, each dropped when it does
//! not fit; the simplex of each axis then holds the facts kept, the literals at their values and
//! the relations kept.
//!
//! **The LP** (each axis on its own, as the axes are independent once the orientations are
//! decided, algorithm synthesis 3.2). Every pin and text the model has is first linked to its
//! object's origin by the orientation, `x_pin − x_origin = c(orientation)`, so that all of it is
//! one rigid body. Then, one layer after the other, each optimum fixed as a bound before the
//! next:
//!
//! 1. the wire length: for each net drawn as wires, the extent of its points, `hi − lo` with
//!    `lo ≤ xₚ ≤ hi`, summed: half the perimeter of its box, the stand-in for its length;
//! 2. compactness: the extent of the boxes of the components;
//! 3. the L1 distance of each origin to where the resolution of the candidate put it: the
//!    tie-break that makes the result stable. A symbol's target is that place rounded to the grid;
//!    a point's is the place itself, as it mostly follows a symbol. A text attached to a point or
//!    to an object with a coordinate point has its target there instead, `x_text − x_anchor = c`,
//!    at the point or beside the object ([`crate::structure::Anchor`], 13.3), so that a text no
//!    fact places goes beside what it is attached to.
//!
//! **Rounding.** The grid is the solver's own rule, ranked after every description (11.7), and
//! also after the layers above: with the layers' bounds taken away, the origins are visited, the
//! symbols first in the order of the objects, then the points and texts, which mostly follow a
//! symbol. Each goes to the nearer grid point beside it when the facts and relations allow, or
//! else to the other, and each that ends up on the grid is held there, so that a later one cannot
//! move it off again. A symbol that fits on neither is tried once more before the others held in
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
//! apart before them leave none, the rounds ran out, or the budget did.

use std::collections::{BTreeMap, BTreeSet};

use num_traits::{One, Zero};
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::LevelProblem;

use crate::model::{turned, Model, PointId};
use crate::rational::{from_f64, Q};
use crate::resolve::{Geometry, Resolver, Tag};
use crate::search::{orientation_relations, relations, up, with_relations, Candidate, RelKind, Relation, Tagged};
use crate::simplex::{LinExpr, Rel, Simplex, Var};
use crate::structure::{Role, Structure};
use crate::{SolverOptions, Work};

/// How many rounds of putting overlapping symbols apart.
const REPAIR_ROUNDS: usize = 6;

/// Why two symbols still overlap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    /// The facts kept, with the orientations taken, leave no side to put them apart.
    Facts,
    /// A side fits with the facts kept, but not with the symbols put apart before them.
    Others,
    /// The rounds of putting symbols apart ran out before they had their turn.
    Rounds,
    /// The budget of pivots ran out while the candidate was taken in or placed, so a relation
    /// may have been dropped without a conflict.
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
    /// Whether the LP ran: when its budget ran out, the placement is where the resolution of the
    /// candidate put it.
    pub optimised: bool,
    /// Whether the budget ran out deciding a relation of the candidate, which was then dropped
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
    let budget = options.budget;
    let grid = options.grid.and_then(from_f64).filter(|g| *g > Q::zero());
    let orient = orientation_relations(s, c);
    let actual: Vec<Orient> = with_relations(s, &orient, budget, work, |r, _, _| r.geometry().orient.clone());
    let mut separations: Vec<Tagged> = Vec::new();
    let mut separated: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut round = 0;
    loop {
        let mut rels = orient.clone();
        rels.extend(separations.iter().cloned());
        rels.extend(relations(s, c, &actual, options));
        let (geometry, kept, undecided, optimised) = with_relations(s, &rels, budget, work, |r, model, t| {
            let (g, optimised) = optimise(r, model, s, options);
            (g, t.kept.clone(), t.undecided, optimised)
        });
        let origins = s.origins(problem, &geometry);
        let overlaps = overlapping(s, &origins, &geometry.orient);
        let new: Vec<(usize, usize)> = overlaps.iter().copied().filter(|p| !separated.contains(p)).collect();
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
                        apart_cause(s, &origins, &geometry.orient, a, b, &grid, budget, work)
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
    }
}

/// Why two symbols that were given a relation to put them apart still overlap: whether the facts
/// kept, with the orientations as placed, leave a side for them at all.
#[allow(clippy::too_many_arguments)]
fn apart_cause(
    s: &Structure,
    origins: &[[Q; 2]],
    ori: &[Orient],
    a: usize,
    b: usize,
    grid: &Option<Q>,
    budget: crate::Budget,
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
    let (fits, undecided) = with_relations(s, &rels, budget, work, |_, _, t| (t.kept[t.kept.len() - 1], t.undecided));
    match (fits, undecided) {
        (true, _) => Cause::Others,
        (false, true) => Cause::Budget,
        (false, false) => Cause::Facts,
    }
}

/// The symbols placed by their own position: elements, supply, ground and port symbols, and other
/// virtual parts with a position of their own.
pub(crate) fn solid(s: &Structure, k: usize) -> bool {
    let o = &s.objects[k];
    o.own && o.symbol.is_some() && !matches!(o.role, Role::Point | Role::Text)
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
fn link(sx: &mut Simplex<Tag>, model: &Model, ori: &[Orient], axis: usize) -> bool {
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
fn minimise_and_keep(sx: &mut Simplex<Tag>, objective: &LinExpr) -> Result<(), ()> {
    if objective.is_constant() {
        return Ok(());
    }
    sx.minimize(objective).map_err(|_| ())?;
    sx.keep_optimum(objective, Tag::Def).map_err(|_| ())
}

/// Where layer 3 of the LP pulls an origin (see the module).
enum Target {
    /// To a place.
    At(Q),
    /// To a place relative to another variable: `v − base = c`.
    From(Var, Q),
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
                let base = model.points.get(&model.origin_point.get(a.object).copied().flatten()?)?.var;
                let pin = match a.pin {
                    Some(pin) => s.pin_offset(a.object, pin, ori[a.object])?[axis].clone(),
                    None => Q::zero(),
                };
                Some(Target::From(base, pin + &a.offset[axis]))
            });
            let at = &fallback.origin[*k][axis];
            anchored.unwrap_or_else(|| Target::At(if solid(s, *k) { round(at, grid) } else { at.clone() }))
        })
        .collect()
}

/// The three layers of the LP on one axis (see the module).
fn layers(
    sx: &mut Simplex<Tag>,
    model: &Model,
    s: &Structure,
    ori: &[Orient],
    own: &[(usize, Var)],
    targets: &[Target],
    axis: usize,
) -> Result<(), ()> {
    let zero = Q::zero();
    let fail = |_| ();
    // 1. the wire length
    let mut objective = LinExpr::default();
    for d in &s.connections.drawn {
        let vars: BTreeSet<Var> = d.points.iter().filter_map(|p| model.points.get(p).map(|i| i.var)).collect();
        if vars.len() < 2 {
            continue;
        }
        let (hi, lo) = (sx.new_var(), sx.new_var());
        for &v in &vars {
            sx.assert(&LinExpr::diff(hi, v), Rel::Ge, &zero, Tag::Def).map_err(fail)?;
            sx.assert(&LinExpr::diff(v, lo), Rel::Ge, &zero, Tag::Def).map_err(fail)?;
        }
        objective.add_term(hi, &Q::one());
        objective.add_term(lo, &-Q::one());
    }
    minimise_and_keep(sx, &objective)?;
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
    // 3. the distance to where the resolution put each origin, or to a text's anchor:
    // d ≥ e − c and d ≥ c − e, for e = v or e = v − base
    let mut objective = LinExpr::default();
    for ((_, v), t) in own.iter().zip(targets) {
        let d = sx.new_var();
        let (base, c) = match t {
            Target::At(c) => (None, c),
            Target::From(base, c) => (Some(*base), c),
        };
        let mut below = LinExpr::from_terms([(d, Q::one()), (*v, -Q::one())], Q::zero());
        let mut above = LinExpr::from_terms([(d, Q::one()), (*v, Q::one())], Q::zero());
        if let Some(b) = base {
            below.add_term(b, &Q::one());
            above.add_term(b, &-Q::one());
        }
        sx.assert(&below, Rel::Ge, &-c.clone(), Tag::Def).map_err(fail)?;
        sx.assert(&above, Rel::Ge, c, Tag::Def).map_err(fail)?;
        objective.add_term(d, &Q::one());
    }
    minimise_and_keep(sx, &objective)
}

/// Holds `v` at `r` when the bounds allow it, and says whether they did.
fn try_pin(sx: &mut Simplex<Tag>, v: Var, r: &Q) -> bool {
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
fn round_to_grid(sx: &mut Simplex<Tag>, order: &[Visit], grid: &Q, kept: &BTreeSet<usize>) -> Vec<(usize, Q)> {
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
fn optimise(r: &mut Resolver, model: &Model, s: &Structure, options: &SolverOptions) -> (Geometry, bool) {
    let fallback = r.geometry().clone();
    let ori = fallback.orient.clone();
    let grid = options.grid.and_then(from_f64).filter(|g| *g > Q::zero());
    let own: Vec<(usize, Var)> = (0..s.objects.len())
        .filter(|&k| s.objects[k].own)
        .filter_map(|k| Some((k, model.points.get(&PointId::Origin(k))?.var)))
        .collect();
    // the symbols go on the grid first, then the points and texts, which mostly follow a symbol
    let mut order: Vec<Visit> = own.iter().filter(|(k, _)| solid(s, *k)).map(|&(k, v)| (k, v, true)).collect();
    order.extend(own.iter().filter(|(k, _)| !solid(s, *k)).map(|&(k, v)| (k, v, false)));
    let mut out = fallback.clone();
    for axis in 0..2 {
        let sx = r.simplex_mut(axis);
        let linked = link(sx, model, &ori, axis);
        if !linked {
            return (fallback, false);
        }
        // where the resolution of the candidate put each origin, a symbol's on the grid; a text or a
        // point follows what it is placed from, and a target rounded on its own would pull that
        // off the grid; a text goes to its anchor
        let targets = targets(model, s, &own, &fallback, &grid, axis);
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
                    return (fallback, false);
                }
            }
            if sx.check().is_err() {
                return (fallback, false);
            }
            let cp = sx.checkpoint();
            let solved = layers(sx, model, s, &ori, &own, &targets, axis).is_ok();
            sx.backtrack(cp);
            if !solved && sx.check().is_err() {
                return (fallback, false);
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
            return (fallback, false);
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
    (out, true)
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
