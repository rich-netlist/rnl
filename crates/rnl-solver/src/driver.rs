//! The flow of a solve, its budget and its report (plan 5.9).
//!
//! ```text
//! resolve (8.5) → structure → search (with coordinate LPs) → for the best `beam`: coords → route
//! → score → the best by the layered tuple, ties by the canonical code
//! ```
//!
//! Stage S1: none of these steps exists yet. [`solve`] checks the options and gives an empty
//! solution, with a note that says so; the stages that follow fill the steps in.

use rnl_runtime::problem::LevelProblem;
use rnl_runtime::solution::Solution;

use crate::{SolveReport, Solved, SolverOptions};

/// The note of a solve that resolves and places nothing.
pub(crate) const UNSOLVED_NOTE: &str =
    "rnl-solver is at stage S1 of solver v1: nothing is resolved, placed or routed yet, and the solution is empty";

pub(crate) fn solve(problem: &LevelProblem, options: &SolverOptions) -> Solved {
    let mut notes = vec![UNSOLVED_NOTE.to_string()];
    if let Err(e) = options.validate() {
        notes.push(format!("the options are not valid: {e}"));
    }
    // the stages that follow read the problem; this one only counts it
    notes.push(format!(
        "the level `{}` has {} objects, {} facts and {} ROUTEs",
        problem.title,
        problem.objects.len(),
        problem.facts.len(),
        problem.routes.len()
    ));
    Solved { solution: Solution::default(), report: SolveReport { notes, ..SolveReport::default() } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Objectives, Work};

    fn problem() -> LevelProblem {
        LevelProblem {
            level: 0,
            title: "t.sp".to_string(),
            kind: "file",
            name: String::new(),
            version: "0.2".to_string(),
            authored_numbers: 0,
            file: "t.sp".to_string(),
            line: 1,
            end_line: 1,
            packages: Vec::new(),
            names: Vec::new(),
            objects: Vec::new(),
            symbols: Vec::new(),
            facts: Vec::new(),
            wires: Vec::new(),
            routes: Vec::new(),
            groups: Vec::new(),
            usages: Vec::new(),
        }
    }

    #[test]
    fn the_stub_gives_an_empty_solution_and_says_so() {
        let solved = crate::solve(&problem(), &SolverOptions::default());
        assert_eq!(solved.solution, Solution::default());
        assert_eq!(solved.report.notes[0], UNSOLVED_NOTE);
        assert!(solved.report.kept.is_empty() && solved.report.discarded.is_empty());
        assert_eq!(solved.report.work, Work::default());
        assert_eq!(solved, crate::solve(&problem(), &SolverOptions::default()));
    }

    #[test]
    fn options_that_make_no_sense_are_noted() {
        let mut options = SolverOptions { beam: 0, ..SolverOptions::default() };
        options.objectives = Objectives::none();
        let solved = crate::solve(&problem(), &options);
        assert!(solved.report.notes.iter().any(|n| n.contains("beam")));
    }
}
