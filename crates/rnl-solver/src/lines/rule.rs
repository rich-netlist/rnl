//! Orientation filters from retained local facts, and the least bends between directed pins.

use crate::rational::Q;
use crate::structure::{FreeClass, Structure};
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{Axis, FactKind, LevelProblem, PointRef, Sense};
use std::collections::BTreeMap;

pub(super) fn minbend(a: (i64, i64), b: (i64, i64), v: (f64, f64)) -> u32 {
    let dot = |p: (i64, i64), q: (f64, f64)| p.0 as f64 * q.0 + p.1 as f64 * q.1;
    let cross = |p: (i64, i64), q: (f64, f64)| p.0 as f64 * q.1 - p.1 as f64 * q.0;
    if v.0 == 0.0 && v.1 == 0.0 {
        return 0;
    }
    if a == b {
        if dot(a, v) > 0.0 {
            return if cross(a, v) == 0.0 { 0 } else { 2 };
        }
        return 4;
    }
    if a == (-b.0, -b.1) {
        return if cross(a, v) == 0.0 { 4 } else { 2 };
    }
    if dot(a, v) > 0.0 && dot(b, v) > 0.0 {
        1
    } else {
        3
    }
}

pub(super) fn allowed(problem: &LevelProblem, s: &Structure, class: &FreeClass, v: Orient) -> bool {
    let members: BTreeMap<usize, Orient> = class.members.iter().map(|&(m, label)| (m, v.then(label))).collect();
    let point = |r: &PointRef| -> Option<(usize, [Q; 2])> {
        match r {
            PointRef::Origin(k) if members.contains_key(k) => Some((*k, [Q::default(), Q::default()])),
            PointRef::Pin { object, pin, .. }
            | PointRef::SymbolPin { object, pin, .. }
            | PointRef::PinPosition { object, pin, .. } => {
                let o = *members.get(object)?;
                Some((*object, s.pin_offset(*object, *pin, o)?))
            }
            _ => None,
        }
    };
    let axis_of = |a: &Axis| if *a == Axis::X { 0 } else { 1 };
    for &f in &s.kept {
        match &problem.facts[f].kind {
            FactKind::Align { axis, members: ms } => {
                for pair in ms.windows(2) {
                    if let (Some((ka, pa)), Some((kb, pb))) = (point(&pair[0]), point(&pair[1])) {
                        if ka == kb && pa[axis_of(axis)] != pb[axis_of(axis)] {
                            return false;
                        }
                    }
                }
            }
            FactKind::Order { axis, sense, members: ms } | FactKind::Distribute { axis, sense, members: ms } => {
                let sign = match sense {
                    Sense::Increasing => 1,
                    Sense::Decreasing => -1,
                    Sense::Either => continue,
                };
                for pair in ms.windows(2) {
                    if let (Some((ka, pa)), Some((kb, pb))) = (point(&pair[0]), point(&pair[1])) {
                        let d = (&pb[axis_of(axis)] - &pa[axis_of(axis)]) * crate::rational::int(sign);
                        if ka == kb && d < crate::rational::int(1) {
                            return false;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_table_of_least_bends() {
        use super::minbend;
        assert_eq!(minbend((1, 0), (1, 0), (5.0, 0.0)), 0);
        assert_eq!(minbend((1, 0), (1, 0), (5.0, 2.0)), 2);
        assert_eq!(minbend((1, 0), (1, 0), (-5.0, 2.0)), 4);
        assert_eq!(minbend((1, 0), (-1, 0), (5.0, 0.0)), 4);
        assert_eq!(minbend((1, 0), (-1, 0), (5.0, 3.0)), 2);
        assert_eq!(minbend((1, 0), (0, 1), (5.0, 3.0)), 1);
        assert_eq!(minbend((1, 0), (0, 1), (-5.0, 3.0)), 3);
        assert_eq!(minbend((1, 0), (0, 1), (0.0, 0.0)), 0);
    }
}
