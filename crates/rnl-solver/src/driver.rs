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
//! first, then what crowds the drawing (gaps between neighbours in a row that the facts kept left
//! out, texts over another symbol), then wires that mislead (wires of different nets that touch, a
//! pin on a wire of another net, a wire through a symbol, 13.2) and then joins left undone, as the
//! solver's own rules rank after every fact and before every objective (8.5, 11.7).
//!
//! **The solution** gives every object with a position of its own its position, a position the
//! file fixes included (plan 3.2), and every object with an orientation its orientation. A text
//! that its owner's symbol places, on the axis that the scene gives it (13.3), has a coordinate of
//! its own there, and the solution gives its position too. The record writes them all (the draft
//! 0.3, 14.8), so that between two solves a statement written after it that conflicts with them
//! is discarded (14.8); `rnl_runtime::solution::write_record` leaves to the file only the axes on
//! which a text follows what it is attached to, where the solution has it there (13.3). But what
//! the solver has nothing to place by is left free ([`left_free`]), and the report says how many:
//! a text attached to a wire or a net, which has no coordinate point, or to anything else the
//! solver has no point for, and that no fact kept reaches, which the viewer places next to what it
//! is attached to (13.3); and a supply, ground or port symbol without a name that no fact kept
//! reaches and that is on no net with a pin that the problem shows, which a viewer leaves
//! unplaced.
//!
//! **Limits.** Only the resolution of 8.5 is timed (the draft 0.3, 8.5): its checks and the
//! search for its minimal conflict groups stop when the time limit of the options is out, on the
//! clock the caller gives (`crate::Clock`); a fact whose check did not finish is discarded
//! undecided, with a warning in the report (7.6), and a fact whose groups were not all found says
//! so (`crate::Discarded::complete`). Everything after it is counted, never timed, so that it is
//! the same on every machine: the search makes at most `candidates`, P0 always among them
//! (`crate::Budget`); the router expands at most `astar_expansions` cells for one net and leaves
//! the rest of the net to flight lines; the witness of the resolution taken as written, the
//! solver's own resolutions of the relations of a candidate and of the rows, and the coordinate LP
//! stop at the caps of the implementation (`crate::options::CAPS`), a relation then dropped or an
//! LP left where the resolution put the candidate. Each falls back to the best found so far, at worst P0, and the report says so.

use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{LevelProblem, Position};
use rnl_runtime::solution::{Placement, Solution};

use crate::coords::{place, Cause, Placed};
use crate::options::{Caps, CAPS};
use crate::rational::{to_f64, Q};
use crate::resolve::Resolved;
use crate::route::{route, style_notes, Failure, Routing};
use crate::score::{crowding, drawn, measured_layers, misleading, order, Entry};
use crate::search::{search, Candidate, RelKind};
use crate::structure::{analyse, Role, Structure};
use crate::{Clock, SolveReport, Solved, SolverOptions, StoppedClock, Unrouted};

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

pub(crate) fn solve(problem: &LevelProblem, options: &SolverOptions, clock: &dyn Clock) -> Solved {
    solve_exact_with(problem, options, clock, CAPS).0
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

/// Solves a level, and gives the exact result with it: the time limit never reached, the caps of
/// the implementation.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn solve_exact(problem: &LevelProblem, options: &SolverOptions) -> (Solved, Exact) {
    solve_exact_with(problem, options, &StoppedClock, CAPS)
}

/// Solves a level with the time limit of the options on `clock`, and the solver's own resolutions,
/// of the rows, of the relations of the candidates and of the coordinate LP, bounded by `caps`
/// (other caps only in the tests; the witness taken as written in the resolution always has
/// `CAPS`), and gives the exact result with it.
pub(crate) fn solve_exact_with(
    problem: &LevelProblem,
    options: &SolverOptions,
    clock: &dyn Clock,
    caps: Caps,
) -> (Solved, Exact) {
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
    let phase = crate::profile::span("stage", "resolve");
    let resolved = crate::resolve::resolve(problem, options, clock);
    drop(phase);
    notes.extend(resolved.notes.iter().cloned());
    let mut work = resolved.work;
    let phase = crate::profile::span("stage", "structure");
    let structure = analyse(problem, &resolved, options, caps, &mut work);
    let use_tree = problem.version == "0.4" && options.settings.layout == crate::Layout::Tree;
    let structure =
        if problem.version == "0.4" { crate::search_choices::for_layout(&structure, &mut work) } else { structure };
    notes.extend(structure.notes.iter().cloned());
    drop(phase);
    let (candidate, placed, routing, entry) = if use_tree {
        let laid = crate::tree::lay_out(problem, &structure, options, &mut work, &crate::tree::Stop::default());
        notes.extend(laid.notes);
        (laid.candidate, laid.placed, laid.routing, laid.entry)
    } else {
        let phase = crate::profile::span("stage", "search");
        let candidates = search(problem, &structure, options, &mut work);
        drop(phase);
        let mut all: Vec<Placing> = Vec::new();
        for (candidate_id, c) in candidates.into_iter().enumerate() {
            if !crate::budget::permit(crate::budget::Step::Placement, problem.objects.len()) && !all.is_empty() {
                break;
            }
            let phase = crate::profile::span("stage", "coords");
            crate::profile::set("candidate_id", candidate_id as u64);
            let placed = place(problem, &structure, &c, options, &mut work);
            drop(phase);
            if crate::budget::stopped() && !all.is_empty() {
                break;
            }
            let phase = crate::profile::span("stage", "score");
            let estimate = measured_layers(problem, &structure, &placed, None);
            let written = drawn(problem, &structure, &placed.geometry, &Routing::default());
            let entry = Entry {
                overlaps: placed.overlaps.len(),
                crowding: crowding(&structure, &placed.origins, &placed.geometry.orient, &placed.dropped),
                misleading: misleading(problem, &structure, &placed.origins, &placed.geometry, &written),
                joins_left: 0,
                layers: estimate,
                code: c.code(&structure),
            };
            all.push(Placing { candidate: c, placed, entry });
            drop(phase);
        }
        let placed_count = all.len();
        // the best `beam` by the estimate are routed and scored on their wires (plan 5.8)
        let phase = crate::profile::span("stage", "score");
        let estimated: Vec<Entry> = all.iter().map(|c| c.entry.clone()).collect();
        let ranked = order(&structure.active, &estimated);
        let beam = options.beam.max(1);
        let mut finals: Vec<(usize, Routing, Entry)> = Vec::new();
        drop(phase);
        for &i in ranked.iter().take(beam) {
            if !crate::budget::permit(crate::budget::Step::Route, problem.routes.len()) && !finals.is_empty() {
                break;
            }
            let p = &all[i];
            let phase = crate::profile::span("stage", "route");
            crate::profile::set("candidate_id", i as u64);
            let routing = route(problem, &structure, &p.placed.geometry, &p.placed.origins, options, &mut work);
            crate::profile::add("wires", routing.wires.len() as u64);
            drop(phase);
            if crate::budget::stopped() && !finals.is_empty() {
                break;
            }
            let phase = crate::profile::span("stage", "score");
            let wires = drawn(problem, &structure, &p.placed.geometry, &routing);
            let score = measured_layers(problem, &structure, &p.placed, Some(&wires));
            let entry = Entry {
                misleading: misleading(problem, &structure, &p.placed.origins, &p.placed.geometry, &wires),
                joins_left: routing.joins_left(),
                layers: score,
                ..p.entry.clone()
            };
            finals.push((i, routing, entry));
            drop(phase);
        }
        let phase = crate::profile::span("stage", "score");
        let entries: Vec<Entry> = finals.iter().map(|(_, _, e)| e.clone()).collect();
        let best = order(&structure.active, &entries)[0];
        let mut seen: BTreeSet<_> = all.iter().map(|p| p.entry.code.clone()).collect();
        let (taken, mut routing, mut entry) = finals.swap_remove(best);
        let Placing { mut candidate, mut placed, .. } = all.swap_remove(taken);
        drop(phase);
        notes.push(format!(
            "{} placed and {} routed; the one taken comes from {}",
            count(placed_count, "candidate"),
            entries.len(),
            if candidate.origin == "P0" { "P0, the order of the netlist" } else { "the beam search" }
        ));
        if problem.version == "0.4" && crate::budget::continues_rows() {
            let mut start = candidate.clone();
            let troubled = |routing: &Routing, placed: &Placed| {
                let wires = drawn(problem, &structure, &placed.geometry, routing);
                crate::score::trouble(&structure, &placed.origins, &placed.geometry.orient, &wires)
            };
            let mut rank = troubled(&routing, &placed);
            let mut radius = 0;
            let mut refined = 0;
            'search: loop {
                radius += 1;
                let mut skip = 0;
                let mut none = true;
                loop {
                    if !crate::budget::permit(crate::budget::Step::Candidate, structure.elements.len()) {
                        break 'search;
                    }
                    let (batch, more, any) = crate::search::neighbours(
                        problem,
                        &structure,
                        options,
                        &mut work,
                        &start,
                        radius,
                        skip,
                        Some(&rank),
                    );
                    none &= !any;
                    for c in batch {
                        if !seen.insert(c.code(&structure)) {
                            continue;
                        }
                        if !crate::budget::permit(crate::budget::Step::Placement, structure.objects.len()) {
                            break 'search;
                        }
                        let p = place(problem, &structure, &c, options, &mut work);
                        let r = route(problem, &structure, &p.geometry, &p.origins, options, &mut work);
                        if crate::budget::stopped() {
                            break 'search;
                        }
                        let wires = drawn(problem, &structure, &p.geometry, &r);
                        let next = Entry {
                            layers: measured_layers(problem, &structure, &p, Some(&wires)),
                            code: c.code(&structure),
                            overlaps: p.overlaps.len(),
                            crowding: crowding(&structure, &p.origins, &p.geometry.orient, &p.dropped),
                            misleading: misleading(problem, &structure, &p.origins, &p.geometry, &wires),
                            joins_left: r.joins_left(),
                        };
                        refined += 1;
                        if order(&structure.active, &[entry.clone(), next.clone()])[0] == 1 {
                            (candidate, placed, routing, entry) = (c, p, r, next);
                            start = candidate.clone();
                            rank = troubled(&routing, &placed);
                            radius = 0;
                            continue 'search;
                        }
                    }
                    if !more {
                        break;
                    }
                    skip += crate::search::BATCH;
                }
                if none {
                    break;
                }
            }
            notes.push(format!("row search: {refined} additional candidates placed and routed"));
        }
        (candidate, placed, routing, entry)
    };
    let score = entry.layers;
    if !placed.optimised {
        notes.push(
            "the coordinate LP reached its cap on pivots: the objects are where the resolution of the candidate put \
             them"
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
                "a cap of work ran out while the layout taken was placed, and some of the solver's own \
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
            Cause::Budget => "a cap of work ran out before they were put apart",
        };
        notes.push(format!("{} and {} overlap: {why}", problem.objects[o.a].label, problem.objects[o.b].label));
    }
    if !problem.routes.is_empty() {
        notes.extend(style_notes(problem));
        let mut nets: Vec<usize> = routing.wires.iter().map(|w| w.route).collect();
        nets.sort_unstable();
        nets.dedup();
        if problem.version == "0.4" || problem.routes.iter().any(|route| route.tree) {
            let mut names: Vec<_> = nets.iter().map(|&index| &problem.routes[index].net).collect();
            names.sort_unstable();
            names.dedup();
            notes.push(format!(
                "{} and {} routed for {} on {}, {} expanded",
                count(routing.wires.len(), "wire"),
                count(routing.branches.len(), "branch point"),
                count(nets.len(), "routing tree"),
                count(names.len(), "net"),
                count(usize::try_from(routing.expansions).unwrap_or(usize::MAX), "cell")
            ));
        } else {
            notes.push(format!(
                "{} and {} routed for the ROUTEs of {}, {} expanded",
                count(routing.wires.len(), "wire"),
                count(routing.branches.len(), "branch point"),
                count(nets.len(), "net"),
                count(usize::try_from(routing.expansions).unwrap_or(usize::MAX), "cell")
            ));
        }
    }
    if !routing.forced.is_empty() {
        notes.push(format!(
            "{} ROUTE trees joined through obstacles after bounded routing: {}",
            routing.forced.len(),
            routing.forced.iter().map(|n| format!("&{n}")).collect::<Vec<_>>().join(", ")
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
    let (mut texts, mut symbols) = (0, 0);
    for (k, o) in problem.objects.iter().enumerate() {
        let orient: Option<Orient> = o.orient.is_some().then(|| placed.geometry.orient[k]);
        match &o.position {
            // a text attached to what the solver has no point for, a wire, a net, a graphic or a
            // terminal without a pin, named or not, and a supply, ground or port symbol without a
            // name on no net that the problem shows, that no fact kept reaches: neither its
            // position nor its orientation is decided (13.3, 11.1)
            Position::Own { .. } if left_free(&structure, k) => {
                if structure.objects[k].role == Role::Text {
                    texts += 1;
                } else {
                    symbols += 1;
                }
            }
            // every position of its own, those the file fixes too (plan 3.2)
            Position::Own { .. } => {
                let [x, y] = &placed.geometry.origin[k];
                placements.push(Placement { object: k, x: Some(to_f64(x)), y: Some(to_f64(y)), orient })
            }
            // drawn at the point of its main object: only its orientation is the solver's (12.1)
            Position::At(_) if orient.is_some() => placements.push(Placement { object: k, x: None, y: None, orient }),
            // a text that its owner's symbol places has a coordinate of its own on the axis that
            // the scene gives it (13.3): where it is, which the record writes on that axis unless
            // its offset still places it there
            Position::InSymbol { replaced: Some(_), .. } => {
                let [x, y] = &placed.origins[k];
                placements.push(Placement { object: k, x: Some(to_f64(x)), y: Some(to_f64(y)), orient: None })
            }
            // a text placed by its symbol alone goes with its owner (13.3)
            Position::At(_) | Position::InSymbol { .. } => {}
        }
    }
    if texts > 0 {
        let is = if texts == 1 { "is" } else { "are" };
        notes.push(format!(
            "{} that no fact kept reaches {is} left free, attached to a wire, a net or anything else the solver has no \
             point for: the viewer places such a text next to what it is attached to (13.3)",
            count(texts, "text")
        ));
    }
    if symbols > 0 {
        let is = if symbols == 1 { "is" } else { "are" };
        notes.push(format!(
            "{} without a name that no fact kept reaches {is} left free, a supply, ground or port symbol on no net \
             with a pin that the problem shows, as no ROUTE or wire written draws one: the solver has no pin to put it \
             beside, and a viewer leaves it unplaced (11.1)",
            count(symbols, "symbol")
        ));
    }
    let report = SolveReport {
        budget: None,
        canonical_options: None,
        kept: resolved.kept.clone(),
        discarded: resolved.discarded.clone(),
        failed: resolved.failed.clone(),
        statements: resolved.statements.clone(),
        inert_objectives: structure.active.inert.clone(),
        unrouted,
        work,
        errors: resolved.errors.clone(),
        warnings: resolved.warnings.clone(),
        notes,
    };
    let solution = Solution {
        placements,
        branches: routing.branches.clone(),
        junctions: Vec::new(),
        wires: routing.wires.clone(),
        wire_routes: Vec::new(),
    };
    let solved = Solved { solution, report };
    (solved, Exact { resolved, structure, candidate, placed, routing, score })
}

/// Whether the solution leaves the position and the orientation of object `k` free, as what the
/// solver has nothing to place by and no part of the drawing it makes
/// (`crate::structure::ObjectInfo::left_free`): a text with a position of its own that no fact kept
/// reaches, attached to what has no coordinate point, a wire or a net ("Wires and nets have no
/// coordinate point", 13.3), or to anything else that the solver has no point for, such as a
/// terminal without a pin or a graphic, named or not, which the viewer places next to what it is
/// attached to (13.3); and a supply, ground or port symbol without a name that no fact kept
/// reaches and that is on no net with a pin of an element that the problem shows, so that the
/// search has no pin to put it beside, which a viewer leaves unplaced, as a record of 0.2 did. A
/// text that a fact kept reaches is the solver's to place, at its anchor on the axis that no fact
/// reaches (`crate::structure::Anchor`); so is a text attached to a point or to an object with a
/// coordinate point, at it or beside it, as the viewer puts it (13.3), and a point `(_, _)` that
/// nothing else places (11.1).
pub(crate) fn left_free(s: &Structure, k: usize) -> bool {
    s.objects.get(k).is_some_and(|o| o.left_free)
}

#[cfg(test)]
mod tests;
