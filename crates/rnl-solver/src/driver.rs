//! The flow of a solve, its budget and its report (plan 5.9).
//!
//! ```text
//! resolve (8.5) → structure → search → for each candidate: coords
//! → the best `beam` by the estimate: route (13.7) → score
//! → the best by the layered tuple, ties by the canonical code
//! ```
//!
//! Every candidate the search makes is placed by the lexicographic LP (`crate::coords`) and
//! ranked by the estimate of the objectives, the routing objectives measuring the straight
//! connections of each net; the best `beam` of them are routed (`crate::route`) and scored on
//! their wires (`crate::score`), and the best of those is the solution. Overlapping symbols rank
//! first, then wires that mislead (wires of different nets that touch, a pin on a wire of another
//! net, a wire through a symbol, 13.2) and then joins left undone, as the solver's own rules rank
//! after every fact and before every objective (8.5, 11.7).
//!
//! **The solution** gives every object with a position of its own its position, a position the
//! file fixes included (plan 3.2), and every object with an orientation its orientation. A text
//! that its owner's symbol places, on the axis that the scene gives it (13.3), has a coordinate of
//! its own there, and the solution gives its position too. The record writes them all (the draft
//! 0.3, 14.8), so that between two solves a statement written after it that conflicts with them
//! is discarded (14.8); `rnl_runtime::solution::write_record` leaves to the file only the axes on
//! which a text follows what it is attached to, where the solution has it there (13.3).
//!
//! **Budget** (`crate::Budget`): the work is counted, never timed, so the result is the same on
//! every machine. The resolution stops deciding a fact after `resolve_nodes` nodes and discards it
//! undecided; a simplex stops at `pivots`, a relation of the solver's own then dropped or an LP
//! left where the resolution put the candidate; the search makes at most `candidates`, P0 always
//! among them; the router expands at most `astar_expansions` cells for one net and leaves the
//! rest of the net to flight lines. Each falls back to the best found so far, at worst P0, and
//! the report says so.

use std::collections::BTreeMap;

use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{LevelProblem, Position};
use rnl_runtime::solution::{Placement, Solution};

use crate::coords::{place, Cause, Placed};
use crate::rational::{to_f64, Q};
use crate::resolve::Resolved;
use crate::route::{route, style_notes, Failure, Routing};
use crate::score::{drawn, layers, misleading, order, Entry};
use crate::search::{search, Candidate, RelKind};
use crate::structure::{analyse, Structure};
use crate::{SolveReport, Solved, SolverOptions, Unrouted};

/// The note of every solve: how this solver solves a level.
pub(crate) const METHOD_NOTE: &str =
    "the facts are resolved (8.5), the candidates of the structure-first search placed \
     by the lexicographic LP and rounded to the grid, the best of them by the estimate routed (13.7), and the best by \
     the layered objectives on their wires taken";

/// A solve with its exact result, for the tests.
#[derive(Clone, Debug)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct Exact {
    pub resolved: Resolved,
    pub structure: Structure,
    pub candidate: Candidate,
    pub placed: Placed,
    pub routing: Routing,
    /// The value of each layer of the objectives in force, on the wires.
    pub score: Vec<Q>,
}

pub(crate) fn solve(problem: &LevelProblem, options: &SolverOptions) -> Solved {
    solve_exact(problem, options).0
}

/// A count and its noun, singular for one: `count(1, "wire")` is "1 wire", `count(2, "wire")` "2
/// wires".
pub(crate) fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// A candidate placed, with how it ranks.
struct Placing {
    candidate: Candidate,
    placed: Placed,
    entry: Entry,
}

/// Solves a level, and gives the exact result with it.
pub(crate) fn solve_exact(problem: &LevelProblem, options: &SolverOptions) -> (Solved, Exact) {
    let mut notes = vec![METHOD_NOTE.to_string()];
    if let Err(e) = options.validate() {
        notes.push(format!("the options are not valid: {e}"));
    }
    notes.push(format!(
        "the level `{}` has {}, {} and {}",
        problem.title,
        count(problem.objects.len(), "object"),
        count(problem.facts.len(), "fact"),
        count(problem.routes.len(), "ROUTE")
    ));
    let resolved = crate::resolve::resolve(problem, options);
    notes.extend(resolved.notes.iter().cloned());
    let mut work = resolved.work;
    let structure = analyse(problem, &resolved, options, &mut work);
    notes.extend(structure.notes.iter().cloned());
    let candidates = search(problem, &structure, options, &mut work);
    let placed_count = candidates.len();
    let mut all: Vec<Placing> = Vec::new();
    for c in candidates {
        let placed = place(problem, &structure, &c, options, &mut work);
        let estimate = layers(&structure, &placed.origins, &placed.geometry, None);
        let written = drawn(problem, &structure, &placed.geometry, &Routing::default());
        let entry = Entry {
            overlaps: placed.overlaps.len(),
            misleading: misleading(problem, &structure, &placed.origins, &placed.geometry, &written),
            joins_left: 0,
            layers: estimate,
            code: c.code(&structure),
        };
        all.push(Placing { candidate: c, placed, entry });
    }
    // the best `beam` by the estimate are routed and scored on their wires (plan 5.8)
    let estimated: Vec<Entry> = all.iter().map(|c| c.entry.clone()).collect();
    let ranked = order(&structure.active, &estimated);
    let beam = options.beam.max(1);
    let mut finals: Vec<(usize, Routing, Entry)> = Vec::new();
    for &i in ranked.iter().take(beam) {
        let p = &all[i];
        let routing = route(problem, &structure, &p.placed.geometry, &p.placed.origins, options, &mut work);
        let wires = drawn(problem, &structure, &p.placed.geometry, &routing);
        let score = layers(&structure, &p.placed.origins, &p.placed.geometry, Some(&wires));
        let entry = Entry {
            misleading: misleading(problem, &structure, &p.placed.origins, &p.placed.geometry, &wires),
            joins_left: routing.joins_left(),
            layers: score,
            ..p.entry.clone()
        };
        finals.push((i, routing, entry));
    }
    let entries: Vec<Entry> = finals.iter().map(|(_, _, e)| e.clone()).collect();
    let best = order(&structure.active, &entries)[0];
    let (taken, routing, entry) = finals.swap_remove(best);
    let Placing { candidate, placed, .. } = all.swap_remove(taken);
    let score = entry.layers;
    notes.push(format!(
        "{} placed and {} routed; the one taken comes from {}",
        count(placed_count, "candidate"),
        entries.len(),
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
        notes.push(if placed.undecided {
            format!(
                "the budget of pivots ran out while the layout taken was placed, and some of the solver's own \
                 relations were left out, not all for a conflict with the facts: {}",
                list.join(", ")
            )
        } else {
            format!(
                "the facts kept leave out some of the solver's own relations of the layout taken: {}",
                list.join(", ")
            )
        });
    }
    for o in &placed.overlaps {
        let why = match o.cause {
            Cause::Facts => "the facts kept, with the orientations taken, leave no side to put them apart",
            Cause::Others => "the symbols put apart before them leave no side for them",
            Cause::Rounds => "the rounds of putting symbols apart ran out before their turn",
            Cause::Budget => "the budget of pivots ran out before they were put apart",
        };
        notes.push(format!("{} and {} overlap: {why}", problem.objects[o.a].label, problem.objects[o.b].label));
    }
    if !problem.routes.is_empty() {
        notes.extend(style_notes(problem));
        let mut nets: Vec<usize> = routing.wires.iter().map(|w| w.route).collect();
        nets.sort_unstable();
        nets.dedup();
        notes.push(format!(
            "{} and {} routed for the ROUTEs of {}, {} expanded",
            count(routing.wires.len(), "wire"),
            count(routing.branches.len(), "branch point"),
            count(nets.len(), "net"),
            count(usize::try_from(routing.expansions).unwrap_or(usize::MAX), "cell")
        ));
    }
    let mut unrouted = Vec::new();
    for l in &routing.left {
        notes.push(format!(
            "the ROUTE of &{} is left in {} parts, which a viewer draws flight lines between: {}",
            l.net,
            l.parts,
            l.failure.reason()
        ));
        unrouted.push(Unrouted {
            route: l.route,
            net: l.net.clone(),
            parts: l.parts,
            budget: l.failure == Failure::Budget,
        });
    }
    let mut placements = Vec::new();
    for (k, o) in problem.objects.iter().enumerate() {
        let orient: Option<Orient> = o.orient.is_some().then(|| placed.geometry.orient[k]);
        match &o.position {
            // every position of its own, those the file fixes too (plan 3.2)
            Position::Own { .. } => {
                let [x, y] = &placed.geometry.origin[k];
                placements.push(Placement { object: k, at: Some((to_f64(x), to_f64(y))), orient })
            }
            // drawn at the point of its main object: only its orientation is the solver's (12.1)
            Position::At(_) if orient.is_some() => placements.push(Placement { object: k, at: None, orient }),
            // a text that its owner's symbol places has a coordinate of its own on the axis that
            // the scene gives it (13.3): where it is, which the record writes on that axis unless
            // its offset still places it there
            Position::InSymbol { replaced: Some(_), .. } => {
                let [x, y] = &placed.origins[k];
                placements.push(Placement { object: k, at: Some((to_f64(x), to_f64(y))), orient: None })
            }
            // a text placed by its symbol alone goes with its owner (13.3)
            Position::At(_) | Position::InSymbol { .. } => {}
        }
    }
    let report = SolveReport {
        kept: resolved.kept.clone(),
        discarded: resolved.discarded.clone(),
        inert_objectives: structure.active.inert.clone(),
        unrouted,
        work,
        notes,
    };
    let solution = Solution {
        placements,
        branches: routing.branches.clone(),
        wires: routing.wires.clone(),
        wire_routes: Vec::new(),
    };
    let solved = Solved { solution, report };
    (solved, Exact { resolved, structure, candidate, placed, routing, score })
}

#[cfg(test)]
mod tests;
