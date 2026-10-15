//! The flow of a solve, its budget and its report (plan 5.9).
//!
//! ```text
//! resolve (8.5) → structure → search → for each candidate: coords → score
//! → the best by the layered tuple, ties by the canonical code
//! ```
//!
//! Stage S3: the facts are resolved, the candidates of the search placed by the lexicographic LP
//! and rounded to the grid, and the best by the layered score of the objectives in force taken.
//! Nothing is routed yet (stage S4): the routing objectives measure the straight connections of
//! each net, and a note says so.

use std::collections::BTreeMap;

use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{LevelProblem, Position};
use rnl_runtime::solution::{Placement, Solution};

use crate::coords::{place, Placed};
use crate::objectives::View;
use crate::rational::{to_f64, Q};
use crate::resolve::Resolved;
use crate::search::{search, Candidate, RelKind};
use crate::structure::{analyse, Structure};
use crate::{SolveReport, Solved, SolverOptions};

/// The note of a solve at this stage.
pub(crate) const STAGE_NOTE: &str = "rnl-solver is at stage S3 of solver v1: the facts are resolved (8.5), the \
     candidates of the structure-first search placed by the lexicographic LP and rounded to the grid, and the best by \
     the layered objectives taken; nothing is routed yet, so the routing objectives measure straight connections";

/// A solve with its exact result, for the tests.
#[derive(Clone, Debug)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct Exact {
    pub resolved: Resolved,
    pub structure: Structure,
    pub candidate: Candidate,
    pub placed: Placed,
    /// The value of each layer of the objectives in force.
    pub score: Vec<Q>,
}

pub(crate) fn solve(problem: &LevelProblem, options: &SolverOptions) -> Solved {
    solve_exact(problem, options).0
}

/// Solves a level, and gives the exact result with it.
pub(crate) fn solve_exact(problem: &LevelProblem, options: &SolverOptions) -> (Solved, Exact) {
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
    let mut work = resolved.work;
    let structure = analyse(problem, &resolved, options);
    notes.extend(structure.notes.iter().cloned());
    let candidates = search(problem, &structure, options, &mut work);
    let count = candidates.len();
    let mut best: Option<(Vec<Q>, Vec<u32>, Candidate, Placed)> = None;
    for c in candidates {
        let placed = place(problem, &structure, &c, options, &mut work);
        let view = View { s: &structure, at: &placed.origins, orient: &placed.geometry.orient };
        let score = structure.active.score(&view, None);
        let code = c.code(&structure);
        // not overlapping is the solver's own rule, ranked after the facts and before the
        // objectives (8.5, 11.7)
        let better = best.as_ref().is_none_or(|(s, k, _, p)| {
            placed
                .overlaps
                .len()
                .cmp(&p.overlaps.len())
                .then_with(|| structure.active.compare(&score, s))
                .then_with(|| code.cmp(k))
                .is_lt()
        });
        if better {
            best = Some((score, code, c, placed));
        }
    }
    let (score, _, candidate, placed) = best.expect("the search gives P0 at least");
    notes.push(format!(
        "{count} candidates placed; the one taken comes from {}",
        if candidate.origin == "P0" { "P0, the order of the netlist" } else { "the beam search" }
    ));
    if !placed.optimised {
        notes.push(
            "the coordinate LP ran out of its budget: the objects are where the resolution of the candidate put them"
                .to_string(),
        );
    }
    let mut dropped: BTreeMap<RelKind, usize> = BTreeMap::new();
    for k in &placed.dropped {
        *dropped.entry(*k).or_default() += 1;
    }
    if !dropped.is_empty() {
        let list: Vec<String> = dropped.iter().map(|(k, n)| format!("{n} {}", k.name())).collect();
        notes.push(format!(
            "the facts kept leave out some of the solver's own relations of the layout taken: {}",
            list.join(", ")
        ));
    }
    for &(a, b) in &placed.overlaps {
        notes.push(format!(
            "{} and {} overlap: the facts kept leave no side to put them apart",
            problem.objects[a].label, problem.objects[b].label
        ));
    }
    let mut placements = Vec::new();
    for (k, o) in problem.objects.iter().enumerate() {
        let orient: Option<Orient> = o.orient.is_some().then(|| placed.geometry.orient[k]);
        match o.position {
            Position::Own { .. } => {
                let [x, y] = &placed.geometry.origin[k];
                placements.push(Placement { object: k, at: Some((to_f64(x), to_f64(y))), orient })
            }
            // drawn at the point of its main object: only its orientation is the solver's (12.1)
            Position::At(_) if orient.is_some() => placements.push(Placement { object: k, at: None, orient }),
            // a text placed by its symbol goes with its owner (13.3)
            Position::At(_) | Position::InSymbol { .. } => {}
        }
    }
    let report = SolveReport {
        kept: resolved.kept.clone(),
        discarded: resolved.discarded.clone(),
        inert_objectives: structure.active.inert.clone(),
        work,
        notes,
    };
    let solved = Solved { solution: Solution { placements, ..Solution::default() }, report };
    (solved, Exact { resolved, structure, candidate, placed, score })
}

#[cfg(test)]
mod tests;
