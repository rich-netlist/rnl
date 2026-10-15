//! The flow of a solve, its budget and its report (plan 5.9).
//!
//! ```text
//! resolve (8.5) → structure → search (with coordinate LPs) → for the best `beam`: coords → route
//! → score → the best by the layered tuple, ties by the canonical code
//! ```
//!
//! Stage S2: only the resolution exists. [`solve`] resolves the facts and places every object
//! where the witness of the resolution puts it (the placeholder of plan 5.5, "P0"), with a note
//! that says so; the stages that follow fill the other steps in.

use rnl_runtime::problem::{LevelProblem, Position};
use rnl_runtime::solution::{Placement, Solution};

use crate::{SolveReport, Solved, SolverOptions};

/// The note of a solve that only resolves.
pub(crate) const STAGE_NOTE: &str = "rnl-solver is at stage S2 of solver v1: the facts are resolved (8.5) and every \
     object is placed where the witness of the resolution puts it; nothing is searched, rounded to the grid or routed yet";

pub(crate) fn solve(problem: &LevelProblem, options: &SolverOptions) -> Solved {
    let mut notes = vec![STAGE_NOTE.to_string()];
    if let Err(e) = options.validate() {
        notes.push(format!("the options are not valid: {e}"));
    }
    notes.push(format!(
        "the level `{}` has {} objects, {} facts and {} ROUTEs",
        problem.title,
        problem.objects.len(),
        problem.facts.len(),
        problem.routes.len()
    ));
    let resolved = crate::resolve::resolve(problem, options);
    notes.extend(resolved.notes.iter().cloned());
    let mut placements = Vec::new();
    for (k, o) in problem.objects.iter().enumerate() {
        let orient = resolved.orientation(k);
        match o.position {
            Position::Own { .. } => placements.push(Placement { object: k, at: resolved.position(k), orient }),
            // drawn at the point of its main object: only its orientation is the solver's (12.1)
            Position::At(_) if orient.is_some() => placements.push(Placement { object: k, at: None, orient }),
            // a text placed by its symbol goes with its owner (13.3)
            Position::At(_) | Position::InSymbol { .. } => {}
        }
    }
    let report = SolveReport {
        kept: resolved.kept.clone(),
        discarded: resolved.discarded.clone(),
        inert_objectives: Vec::new(),
        work: resolved.work,
        notes,
    };
    Solved { solution: Solution { placements, ..Solution::default() }, report }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{fact, instance, point, problem_of};
    use crate::Objectives;
    use rnl_runtime::geom::Orient;
    use rnl_runtime::problem::{Axis, FactKind, PointRef};

    #[test]
    fn an_empty_level_gives_an_empty_solution_and_says_what_is_done() {
        let p = problem_of(Vec::new(), Vec::new());
        let solved = crate::solve(&p, &SolverOptions::default());
        assert_eq!(solved.solution, Solution::default());
        assert_eq!(solved.report.notes[0], STAGE_NOTE);
        assert!(solved.report.kept.is_empty() && solved.report.discarded.is_empty());
        assert_eq!(solved, crate::solve(&p, &SolverOptions::default()));
    }

    #[test]
    fn options_that_make_no_sense_are_noted() {
        let mut options = SolverOptions { beam: 0, ..SolverOptions::default() };
        options.objectives = Objectives::none();
        let solved = crate::solve(&problem_of(Vec::new(), Vec::new()), &options);
        assert!(solved.report.notes.iter().any(|n| n.contains("beam")));
    }

    #[test]
    fn every_object_is_placed_at_the_witness() {
        let p = problem_of(
            vec![instance(), point()],
            vec![
                fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 4.0 }),
                fact(FactKind::Orient { object: 0, code: Orient::MY }),
                fact(FactKind::Align { axis: Axis::Y, members: vec![PointRef::Origin(1), PointRef::Origin(0)] }),
                fact(FactKind::Place { at: PointRef::Origin(1), axis: Axis::Y, value: 2.5 }),
            ],
        );
        let solved = crate::solve(&p, &SolverOptions::default());
        assert_eq!(solved.report.kept, vec![0, 1, 2, 3]);
        assert_eq!(
            solved.solution.placements,
            vec![
                Placement { object: 0, at: Some((4.0, 2.5)), orient: Some(Orient::MY) },
                Placement { object: 1, at: Some((0.0, 2.5)), orient: None },
            ]
        );
        assert!(solved.report.work.resolve_nodes > 0);
    }
}
