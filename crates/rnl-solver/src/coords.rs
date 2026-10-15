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
//! 3. the L1 distance of each origin to where the resolution of the candidate put it, rounded to
//!    the grid: the tie-break that makes the result stable.
//!
//! **Rounding.** The grid is the solver's own rule, ranked after every description (11.7), and
//! also after the layers above: with the layers' bounds taken away, each origin off the grid, in
//! the order of the objects, is put on the nearest point of it when the facts and relations
//! allow, and kept there; then the layers are solved again with those origins kept, and the
//! rounding done once more. A coordinate that the facts force off the grid stays as they force
//! it. The constants of the relations are themselves on the grid ([`crate::search`]), so that the
//! LP's vertices mostly are.
//!
//! **Overlaps.** Not overlapping is the solver's own rule too (11.7): two symbols whose boxes
//! still overlap are put apart by a relation with four sides to choose from, the one that moves
//! least first, ranked after the facts and before the structural relations, so that it takes
//! precedence over them; then the candidate is taken in again, for a few rounds. Two symbols that
//! no side puts apart overlap because the facts make them.

use std::collections::BTreeSet;

use num_traits::{One, Zero};
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::LevelProblem;

use crate::model::{turned, Model, PointId};
use crate::rational::{from_f64, Q};
use crate::resolve::{Geometry, Resolver, Tag};
use crate::search::{orientation_relations, relations, with_relations, Candidate, RelKind, Relation, Tagged};
use crate::simplex::{LinExpr, Rel, Simplex, Var};
use crate::structure::{Role, Structure};
use crate::{SolverOptions, Work};

/// How many rounds of putting overlapping symbols apart.
const REPAIR_ROUNDS: usize = 6;

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
    pub overlaps: Vec<(usize, usize)>,
    /// Whether the LP ran: when its budget ran out, the placement is where the resolution of the
    /// candidate put it.
    pub optimised: bool,
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
    let orient = orientation_relations(s, c);
    let actual: Vec<Orient> = with_relations(s, &orient, budget, work, |r, _, _| r.geometry().orient.clone());
    let mut separations: Vec<Tagged> = Vec::new();
    let mut separated: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut round = 0;
    loop {
        let mut rels = orient.clone();
        rels.extend(separations.iter().cloned());
        rels.extend(relations(s, c, &actual, options));
        let (geometry, kept, optimised) = with_relations(s, &rels, budget, work, |r, model, kept| {
            let (g, optimised) = optimise(r, model, s, options);
            (g, kept.to_vec(), optimised)
        });
        let origins = s.origins(problem, &geometry);
        let overlaps = overlapping(s, &origins, &geometry.orient);
        let new: Vec<(usize, usize)> = overlaps.iter().copied().filter(|p| !separated.contains(p)).collect();
        if new.is_empty() || round >= REPAIR_ROUNDS {
            let dropped = rels.iter().zip(&kept).filter(|(_, k)| !**k).map(|(t, _)| t.kind).collect();
            return Placed { geometry, origins, dropped, overlaps, optimised };
        }
        for (a, b) in new {
            separated.insert((a, b));
            if let Some(t) = separation(s, &origins, &geometry.orient, a, b) {
                separations.push(t);
            }
        }
        round += 1;
    }
}

/// The symbols placed by their own position: elements, supply, ground and port symbols, and other
/// virtual parts with a position of their own.
fn solid(s: &Structure, k: usize) -> bool {
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
/// apart, those that move least first, then the same touching.
pub(crate) fn separation(s: &Structure, origins: &[[Q; 2]], ori: &[Orient], a: usize, b: usize) -> Option<Tagged> {
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
            alternatives.push(vec![(*axis, terms.clone(), c + &gap)]);
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

/// Every pin and text of the model linked to its object's origin by its orientation; then a check.
fn link(sx: &mut Simplex<Tag>, model: &Model, ori: &[Orient], axis: usize) -> bool {
    for (id, info) in &model.points {
        let (Some(k), Some(local)) = (info.object, &info.local) else { continue };
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

/// The three layers of the LP on one axis (see the module).
fn layers(
    sx: &mut Simplex<Tag>,
    model: &Model,
    s: &Structure,
    ori: &[Orient],
    own: &[(usize, Var)],
    targets: &[Q],
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
    // 3. the distance to where the resolution put each origin
    let mut objective = LinExpr::default();
    for ((_, v), t) in own.iter().zip(targets) {
        let d = sx.new_var();
        sx.assert(&LinExpr::diff(d, *v), Rel::Ge, &-t.clone(), Tag::Def).map_err(fail)?;
        let sum = LinExpr::from_terms([(d, Q::one()), (*v, Q::one())], Q::zero());
        sx.assert(&sum, Rel::Ge, t, Tag::Def).map_err(fail)?;
        objective.add_term(d, &Q::one());
    }
    minimise_and_keep(sx, &objective)
}

/// The origins off the grid, in the order of the objects, put on it where that is feasible and
/// kept there; those put on it are added to `fixed`.
fn round_to_grid(sx: &mut Simplex<Tag>, own: &[(usize, Var)], grid: &Option<Q>, fixed: &mut BTreeSet<usize>) {
    if grid.is_none() {
        return;
    }
    for &(k, v) in own {
        if fixed.contains(&k) {
            continue;
        }
        let x = sx.value(v).clone();
        let r = round(&x, grid);
        if r == x {
            continue;
        }
        let cp = sx.checkpoint();
        if sx.assert(&LinExpr::var(v), Rel::Eq, &r, Tag::Def).is_ok() && sx.check().is_ok() {
            fixed.insert(k);
        } else {
            sx.backtrack(cp);
            let _ = sx.check();
        }
    }
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
    let mut out = fallback.clone();
    for axis in 0..2 {
        let sx = r.simplex_mut(axis);
        let linked = link(sx, model, &ori, axis);
        if !linked {
            return (fallback, false);
        }
        let targets: Vec<Q> = own.iter().map(|(k, _)| round(&fallback.origin[*k][axis], &grid)).collect();
        let mut fixed = BTreeSet::new();
        for pass in 0..2 {
            let cp = sx.checkpoint();
            let solved = layers(sx, model, s, &ori, &own, &targets, axis).is_ok();
            sx.backtrack(cp);
            if !solved && sx.check().is_err() {
                return (fallback, false);
            }
            let before = fixed.len();
            round_to_grid(sx, &own, &grid, &mut fixed);
            // the layers again only when the grid moved something
            if pass == 0 && fixed.len() == before {
                break;
            }
        }
        if sx.check().is_err() {
            return (fallback, false);
        }
        for (k, v) in &own {
            out.origin[*k][axis] = sx.value(*v).clone();
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
