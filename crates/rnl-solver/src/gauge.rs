//! The translation gauge of a level: where its absolute coordinates put it moves the drawing, and
//! changes nothing else.
//!
//! The facts of a level are relative, but for the coordinates a PLACE or a point gives
//! (`FactKind::Place`, `FactKind::PointCoordinate`). Every other fact holds as well when the whole
//! drawing is moved. The solve reads numbers that do not: the witness of the resolution is a
//! feasible point, not one moved with the absolute coordinates, and the candidates start from it.
//! So a level whose only absolute coordinate changed could be drawn otherwise, not only moved.
//!
//! The solve therefore takes the level in a gauge of its own, axis by axis, and moves the solution
//! back:
//!
//! - **One absolute coordinate on the axis.** It holds wherever the rest is, as every other fact is
//!   relative, so the level is solved without it, as if it were all relative, and the solution is
//!   moved so that it holds exactly. The fact is kept, and nothing else changes: the coordinate
//!   changed, or written at the place the object has without it, gives the same drawing, moved.
//! - **Two or more.** They hold together only as they are written, so they are kept, all moved by
//!   the first one's value rounded down to the grid (its value when there is no grid). Moving
//!   every one of them by a number of steps of the grid gives the same drawing, moved.
//!
//! The moves are exact rationals, so a coordinate that came in as a short decimal goes back as one.
//! The drawing moves with the coordinate: when it is off the grid, so is the rest, by as much.

use num_traits::Zero;
use rnl_runtime::problem::{Axis, FactKind, LevelProblem, PointRef, Position, Slot};
use rnl_runtime::solution::{Solution, WirePoint};

use crate::rational::{from_f64, to_f64, Q};
use crate::{Solved, SolverOptions};

/// How a level was taken: per axis, the amount its absolute coordinates were moved by, and the
/// object of its one absolute coordinate solved without it, with that coordinate.
#[derive(Debug, Default)]
pub(crate) struct Gauge {
    shift: [Option<Q>; 2],
    anchor: [Option<(usize, Q)>; 2],
}

fn on(kind: &FactKind, axis: Axis) -> Option<f64> {
    match kind {
        FactKind::Place { axis: a, value, .. } | FactKind::PointCoordinate { axis: a, value, .. } if *a == axis => {
            Some(*value)
        }
        _ => None,
    }
}

/// The slots of the objects of `p` on `axis`, those of a position of its own and the axis a text
/// replaces of its symbol's.
fn slots_on(p: &mut LevelProblem, axis: Axis, mut each: impl FnMut(usize, &mut Slot)) {
    for (k, o) in p.objects.iter_mut().enumerate() {
        match &mut o.position {
            Position::Own { x, y } => each(k, if axis == Axis::X { x } else { y }),
            Position::InSymbol { replaced: Some((a, slot)), .. } if *a == axis => each(k, slot),
            _ => {}
        }
    }
}

/// The object whose own coordinate on `axis` the fact `f` gives, as a solution places it.
fn anchored(p: &LevelProblem, f: usize, axis: Axis) -> Option<usize> {
    let k = match &p.facts[f].kind {
        FactKind::Place { at: PointRef::Origin(k), .. } => *k,
        FactKind::PointCoordinate { point, .. } => *point,
        _ => return None,
    };
    match &p.objects.get(k)?.position {
        Position::Own { .. } => Some(k),
        Position::InSymbol { replaced: Some((a, _)), .. } if *a == axis => Some(k),
        _ => None,
    }
}

/// The level in its gauge, and how to move its solution back; None when it has no absolute
/// coordinate that moves anything. With `single` false, an axis of one absolute coordinate keeps
/// it, moved as two or more are.
pub(crate) fn of(problem: &LevelProblem, options: &SolverOptions, single: bool) -> Option<(LevelProblem, Gauge)> {
    let grid = options.for_problem(problem).0.grid.and_then(from_f64).filter(|g| *g > Q::zero());
    let mut p: Option<LevelProblem> = None;
    let mut gauge = Gauge::default();
    for (a, axis) in [(0usize, Axis::X), (1, Axis::Y)] {
        let absolute: Vec<usize> =
            (0..problem.facts.len()).filter(|&f| on(&problem.facts[f].kind, axis).is_some()).collect();
        let Some(&first) = absolute.first() else { continue };
        let Some(value) = on(&problem.facts[first].kind, axis).and_then(from_f64) else { continue };
        if single && absolute.len() == 1 {
            if let Some(k) = anchored(problem, first, axis) {
                let p = p.get_or_insert_with(|| problem.clone());
                // A fact of one member holds wherever it is: it stands in the fact's place, so that
                // the facts keep their order and their numbers.
                p.facts[first].kind = FactKind::Align { axis, members: vec![PointRef::Origin(k)] };
                slots_on(p, axis, |_, slot| {
                    if matches!(slot, Slot::Fixed { fact, .. } if *fact == first) {
                        *slot = Slot::Free;
                    }
                });
                gauge.anchor[a] = Some((k, value));
                continue;
            }
        }
        let shift = match &grid {
            Some(g) => (&value / g).floor() * g,
            None => value,
        };
        if shift.is_zero() {
            continue;
        }
        let p = p.get_or_insert_with(|| problem.clone());
        let moved = |v: &mut f64| {
            if let Some(q) = from_f64(*v) {
                *v = to_f64(&(q - &shift));
            }
        };
        for &f in &absolute {
            match &mut p.facts[f].kind {
                FactKind::Place { value, .. } | FactKind::PointCoordinate { value, .. } => moved(value),
                _ => {}
            }
        }
        slots_on(p, axis, |_, slot| {
            if let Slot::Fixed { value, fact } = slot {
                if absolute.contains(fact) {
                    moved(value);
                }
            }
        });
        gauge.shift[a] = Some(shift);
    }
    p.map(|p| (p, gauge))
}

impl Gauge {
    /// Moves the solution, and the coordinates of its report, back from the gauge. False when the
    /// solution does not place the object of a coordinate solved without it on its axis, so that
    /// it cannot be made to hold: the level is then solved again with the coordinate kept.
    pub(crate) fn restore(&self, solved: &mut Solved) -> bool {
        let mut by: [Option<Q>; 2] = self.shift.clone();
        for (a, anchor) in self.anchor.iter().enumerate() {
            let Some((k, value)) = anchor else { continue };
            let at = solved.solution.placements.iter().find(|pl| pl.object == *k).and_then(|pl| [pl.x, pl.y][a]);
            let Some(at) = at.and_then(from_f64) else { return false };
            by[a] = Some(value - &at);
        }
        let by = by.map(|d| d.filter(|d| !d.is_zero()));
        if by.iter().all(Option::is_none) {
            return true;
        }
        let moved = |v: &mut f64, a: usize| {
            if let (Some(d), Some(q)) = (&by[a], from_f64(*v)) {
                *v = to_f64(&(q + d));
            }
        };
        let Solution { placements, branches, wires, wire_routes, .. } = &mut solved.solution;
        for pl in placements {
            if let Some(x) = &mut pl.x {
                moved(x, 0);
            }
            if let Some(y) = &mut pl.y {
                moved(y, 1);
            }
        }
        for b in branches {
            moved(&mut b.at.0, 0);
            moved(&mut b.at.1, 1);
        }
        for points in wires.iter_mut().map(|w| &mut w.points).chain(wire_routes.iter_mut().map(|w| &mut w.points)) {
            for p in points {
                if let WirePoint::Bend(pt) = p {
                    moved(&mut pt.0, 0);
                    moved(&mut pt.1, 1);
                }
            }
        }
        if let Some(skeleton) = &mut solved.report.skeleton {
            for t in &mut skeleton.tables {
                t.xs.iter_mut().for_each(|x| moved(x, 0));
                t.ys.iter_mut().for_each(|y| moved(y, 1));
            }
            for l in &mut skeleton.lines {
                moved(&mut l.at, usize::from(l.axis));
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use rnl_runtime::problem::LevelProblem;
    use rnl_runtime::solution::{Solution, WirePoint};

    use crate::{solve, Solved, SolverOptions};

    /// A level of two resistors and a capacitor in a row and a title that no fact places, which a
    /// solve leaves where the resolution puts it; `extra` goes at the end of its record.
    fn level(extra: &str) -> LevelProblem {
        let text = format!(
            "* t\n* @RNL(type=\"header\") {{ @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\"); @VERSION(\"0.4\") }}\n\
             R1 a b 1k\nR2 b c 1k\nC1 c 0 1p\n\
             * @RNL {{\n*   {{#R1 #R2 #C1}} @ORDER(\"left-to-right\");\n*   (_, _) @TEXT(\"a title\") => %title{extra}\n* }}\n.end\n"
        );
        let files = rnl_runtime::source::MemoryProvider::new().with("t.sp", &text);
        let mut problems = rnl_view::interpret("t.sp", None, &files, &rnl_runtime::Options::solver()).unwrap().problems;
        assert_eq!(problems.len(), 1);
        problems.remove(0)
    }

    fn solved(extra: &str) -> (LevelProblem, Solved) {
        let p = level(extra);
        let solved = solve(&p, &SolverOptions::without_time_limit());
        assert!(solved.report.discarded.is_empty(), "{:?}", solved.report.discarded);
        (p, solved)
    }

    fn object(p: &LevelProblem, label: &str) -> usize {
        p.objects.iter().position(|o| o.label == label).unwrap_or_else(|| panic!("no {label}"))
    }

    fn at(s: &Solved, k: usize) -> (f64, f64) {
        let pl = s.solution.placements.iter().find(|pl| pl.object == k).expect("placed");
        (pl.x.expect("x"), pl.y.expect("y"))
    }

    /// The solution with everything it places moved by `(dx, dy)`.
    fn moved(s: &Solution, (dx, dy): (f64, f64)) -> Solution {
        let mut s = s.clone();
        for pl in &mut s.placements {
            pl.x = pl.x.map(|x| x + dx);
            pl.y = pl.y.map(|y| y + dy);
        }
        for b in &mut s.branches {
            b.at = (b.at.0 + dx, b.at.1 + dy);
        }
        for points in s.wires.iter_mut().map(|w| &mut w.points).chain(s.wire_routes.iter_mut().map(|w| &mut w.points)) {
            for p in points {
                if let WirePoint::Bend(pt) = p {
                    *pt = (pt.0 + dx, pt.1 + dy);
                }
            }
        }
        s
    }

    #[test]
    fn moving_the_one_absolute_coordinate_moves_the_drawing_and_changes_nothing_else() {
        let (p, first) = solved(";\n*   #R1 @PLACE(0, 0)");
        let r1 = object(&p, "#R1");
        assert_eq!(at(&first, r1), (0.0, 0.0));
        // off the grid too: the drawing moves with the coordinate, off the grid by as much
        for (x, y) in [(24.0, 0.0), (30.0, 6.0), (-11.0, 3.0), (0.5, 0.0)] {
            let (_, other) = solved(&format!(";\n*   #R1 @PLACE({x}, {y})"));
            assert_eq!(at(&other, r1), (x, y));
            assert_eq!(other.solution, moved(&first.solution, (x, y)), "R1 at ({x}, {y})");
            assert_eq!(other.report.kept, first.report.kept);
        }
    }

    #[test]
    fn a_coordinate_written_where_the_solve_puts_the_object_changes_nothing() {
        let (p, free) = solved("");
        let (x, y) = at(&free, object(&p, "#R1"));
        let (_, written) = solved(&format!(";\n*   #R1 @PLACE({x}, {y})"));
        assert_eq!(written.solution, free.solution);
    }

    #[test]
    fn moving_every_absolute_coordinate_by_steps_of_the_grid_moves_the_drawing_alone() {
        let (_, first) = solved(";\n*   #R1 @PLACE(3, 1);\n*   #C1 @PLACE(17, 2)");
        let (_, other) = solved(";\n*   #R1 @PLACE(10, -2);\n*   #C1 @PLACE(24, -1)");
        assert_eq!(other.solution, moved(&first.solution, (7.0, -3.0)));
        assert_eq!(other.report.kept, first.report.kept);
    }
}
