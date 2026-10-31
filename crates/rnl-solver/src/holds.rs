//! Whether a fact holds at given coordinates (14.1–14.5, 14.9), a difference within the tolerance
//! counting as satisfied (11.7). The one test of a fact there is: of a level as it is drawn once
//! its record is read back ([`holds_as_drawn`]), of a solution ([`fact_holds`]), and of the exact
//! placement of the solver, whose equalities between points are exact as its model keeps them
//! ([`check`]). A fact with a point or an orientation that is not known has no answer.

use num_traits::{One, Signed, Zero};
use rnl_runtime::geom::{Orient, TOL};
use rnl_runtime::previous::Drawn;
use rnl_runtime::problem::{Axis, FactKind, LevelProblem, PointRef, Sense};

use crate::model::{ax, turned};
use crate::rational::{from_f64, Q};

/// A coordinate of a point, when it is known.
type At<'a> = &'a dyn Fn(&PointRef, Axis) -> Option<Q>;
/// The orientation of an object, when it is known.
type Ori<'a> = &'a dyn Fn(usize) -> Option<Orient>;

/// Whether a fact holds at these coordinates and orientations; None when one that it needs is not
/// known. The constants are met within `tol`, the equalities between points (ALIGN, DISTRIBUTE)
/// within `equal`.
pub(crate) fn check(kind: &FactKind, at: At, ori: Ori, tol: &Q, equal: &Q) -> Option<bool> {
    let along = |members: &[PointRef], axis: Axis| -> Option<Vec<Q>> { members.iter().map(|m| at(m, axis)).collect() };
    let near = |a: Q, b: Q| (a - b).abs() <= *tol;
    Some(match kind {
        FactKind::Place { at: m, axis, value } => near(at(m, *axis)?, from_f64(*value)?),
        FactKind::PointCoordinate { point, axis, value } => {
            near(at(&PointRef::Origin(*point), *axis)?, from_f64(*value)?)
        }
        FactKind::Offset { text, from, axis, value } => {
            near(at(&PointRef::Origin(*text), *axis)? - at(from, *axis)?, from_f64(*value)?)
        }
        FactKind::Orient { object, code } => ori(*object)? == *code,
        FactKind::OrientRelation { relation, from, to } => {
            let side = |ks: &[usize]| -> Option<Vec<Orient>> { ks.iter().map(|k| ori(*k)).collect() };
            let (a, b) = (side(from)?, side(to)?);
            let (Some(first), Some(second)) = (a.first(), b.first()) else { return Some(true) };
            a.iter().all(|o| o == first) && b.iter().all(|o| o == second) && *second == first.then(*relation)
        }
        FactKind::Align { axis, members } => {
            let v = along(members, *axis)?;
            v.iter().all(|x| (x - &v[0]).abs() <= *equal)
        }
        FactKind::Order { axis, sense, members } => {
            let v = along(members, *axis)?;
            let step = Q::one() - tol;
            let inc = v.windows(2).all(|w| &w[1] - &w[0] >= step);
            let dec = v.windows(2).all(|w| &w[0] - &w[1] >= step);
            match sense {
                Sense::Increasing => inc,
                Sense::Decreasing => dec,
                Sense::Either => inc || dec,
            }
        }
        FactKind::Distribute { axis, members } => {
            let v = along(members, *axis)?;
            v.len() < 3 || v.windows(2).all(|w| ((&w[1] - &w[0]) - (&v[1] - &v[0])).abs() <= *equal)
        }
        FactKind::Distance { axis, sense, min, max, from, to } => {
            if from.is_empty() || to.is_empty() {
                return Some(true);
            }
            let extent = |ms: &[PointRef]| -> Option<(Q, Q)> {
                let v = along(ms, *axis)?;
                Some((v.iter().min()?.clone(), v.iter().max()?.clone()))
            };
            let (a, b) = (extent(from)?, extent(to)?);
            let up = &b.0 - &a.1;
            let down = &a.0 - &b.1;
            let gaps = match sense {
                Sense::Increasing => vec![up],
                Sense::Decreasing => vec![down],
                Sense::Either => vec![up, down],
            };
            // "x" or "y": one side has the lower bound; the upper bound holds of the largest gap
            let largest = gaps.iter().max()?.clone();
            let low = match min {
                Some(m) => {
                    let m = from_f64(*m)? - tol;
                    gaps.iter().any(|g| *g >= m)
                }
                None => true,
            };
            let high = match max {
                Some(m) => largest <= from_f64(*m)? + tol,
                None => true,
            };
            low && high
        }
    })
}

/// The tolerance as a number: a tolerance that is not one is that of the toolkit, as the resolution
/// takes it.
fn tolerance(value: f64) -> Q {
    from_f64(value).filter(|t| *t >= Q::zero()).unwrap_or_else(|| from_f64(TOL).unwrap_or_default())
}

/// Whether a fact holds at these coordinates and orientations, within `tolerance` (11.7); None when
/// one that it needs is not known.
pub fn fact_holds(
    kind: &FactKind,
    at: &dyn Fn(&PointRef, Axis) -> Option<f64>,
    orient: &dyn Fn(usize) -> Option<Orient>,
    tolerance: f64,
) -> Option<bool> {
    let tol = self::tolerance(tolerance);
    check(kind, &|r, axis| from_f64(at(r, axis)?), orient, &tol, &tol)
}

/// A coordinate of a point where a level draws its objects: an origin as drawn, a pin turned with
/// its object as drawn, a centre the middle of the extent of its set (14.1).
fn drawn_value(p: &LevelProblem, drawn: &[Drawn], r: &PointRef, axis: Axis) -> Option<Q> {
    match r {
        PointRef::Origin(k) => from_f64(drawn.get(*k)?.at[ax(axis)]?),
        PointRef::Pin { object, pin, .. } => {
            let pos = p.symbols.get(p.objects.get(*object)?.symbol?)?.pins.get(*pin)?.pos;
            let local = (from_f64(pos.0)?, from_f64(pos.1)?);
            let o = drawn.get(*object)?;
            Some(from_f64(o.at[ax(axis)]?)? + turned(o.orient?, &local, ax(axis)))
        }
        PointRef::Centre(members) => {
            let values: Vec<Q> = members.iter().map(|m| drawn_value(p, drawn, m, axis)).collect::<Option<_>>()?;
            let lo = values.iter().min()?;
            let hi = values.iter().max()?;
            Some((lo + hi) / Q::from_integer(2.into()))
        }
    }
}

/// Whether a fact of the problem `p` holds where its level draws the objects, read back from the
/// text ([`rnl_runtime::previous::drawn`]), within `tolerance` (11.7): the digits a value without a
/// finite decimal form is written with count against the equalities too. None when a point that it
/// needs has no position there, or one that the viewer chooses.
pub fn holds_as_drawn(p: &LevelProblem, drawn: &[Drawn], kind: &FactKind, tolerance: f64) -> Option<bool> {
    let tol = self::tolerance(tolerance);
    check(kind, &|r, axis| drawn_value(p, drawn, r, axis), &|k| drawn.get(k)?.orient, &tol, &tol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{instance, problem_of};

    fn at(x: f64, y: f64) -> Drawn {
        Drawn { at: [Some(x), Some(y)], orient: Some(Orient::R0) }
    }

    fn origins(ks: &[usize]) -> Vec<PointRef> {
        ks.iter().map(|k| PointRef::Origin(*k)).collect()
    }

    fn three() -> LevelProblem {
        problem_of(vec![instance(), instance(), instance()], Vec::new())
    }

    #[test]
    fn a_fact_is_measured_where_the_objects_are_drawn() {
        let p = three();
        let drawn = [at(1.0, 0.0), at(1.0 + 5e-7, 3.0), at(4.0, 6.0)];
        let holds = |kind: &FactKind| holds_as_drawn(&p, &drawn, kind, 1e-6);
        // an equality within the tolerance, and one past it
        assert_eq!(holds(&FactKind::Align { axis: Axis::X, members: origins(&[0, 1]) }), Some(true));
        assert_eq!(holds(&FactKind::Align { axis: Axis::X, members: origins(&[0, 2]) }), Some(false));
        assert_eq!(holds(&FactKind::Place { at: PointRef::Origin(2), axis: Axis::Y, value: 6.0 }), Some(true));
        assert_eq!(holds(&FactKind::Place { at: PointRef::Origin(2), axis: Axis::Y, value: 6.1 }), Some(false));
        assert_eq!(holds(&FactKind::Orient { object: 0, code: Orient::R90 }), Some(false));
        let order = |sense| FactKind::Order { axis: Axis::Y, sense, members: origins(&[0, 1, 2]) };
        assert_eq!(holds(&order(Sense::Increasing)), Some(true));
        assert_eq!(holds(&order(Sense::Decreasing)), Some(false));
        let distance = |min, max| FactKind::Distance {
            axis: Axis::Y,
            sense: Sense::Either,
            min,
            max,
            from: origins(&[0]),
            to: origins(&[1, 2]),
        };
        assert_eq!(holds(&distance(Some(3.0), Some(3.0))), Some(true));
        assert_eq!(holds(&distance(Some(4.0), None)), Some(false));
    }

    /// 1/3 written with the digits of a record: steps equal only within the tolerance (11.7).
    #[test]
    fn digits_written_for_a_value_without_a_finite_form_count_as_equal() {
        let p = three();
        let drawn = [at(0.0, 0.0), at(0.333333333, 0.0), at(0.666666667, 0.0)];
        let kind = FactKind::Distribute { axis: Axis::X, members: origins(&[0, 1, 2]) };
        assert_eq!(holds_as_drawn(&p, &drawn, &kind, 1e-6), Some(true));
        assert_eq!(holds_as_drawn(&p, &drawn, &kind, 1e-12), Some(false));
    }

    #[test]
    fn what_has_no_position_has_no_answer() {
        let p = three();
        // no x for the second, and no orientation
        let drawn = [at(0.0, 0.0), Drawn { at: [None, Some(5.0)], orient: None }];
        let holds = |kind: &FactKind| holds_as_drawn(&p, &drawn, kind, 1e-6);
        assert_eq!(holds(&FactKind::Align { axis: Axis::X, members: origins(&[0, 1]) }), None);
        assert_eq!(holds(&FactKind::Align { axis: Axis::Y, members: origins(&[0, 1]) }), Some(false));
        assert_eq!(holds(&FactKind::Orient { object: 1, code: Orient::R0 }), None);
        // nor does a point of an object that is not in the level as read back
        assert_eq!(holds(&FactKind::Place { at: PointRef::Origin(7), axis: Axis::X, value: 0.0 }), None);
        assert_eq!(holds_as_drawn(&p, &[], &FactKind::Orient { object: 0, code: Orient::R0 }, 1e-6), None);
    }
}
