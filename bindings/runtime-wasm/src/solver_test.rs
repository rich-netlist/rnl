//! Executed cross-target acceptance seam, compiled only with `test-solver`.
//!
//! Both targets parse the embedded default, interpret the same 0.3 source and perform a real
//! solve. The comparison retains the entire logical solution and structured scoring. No clocks,
//! host config resolver, absolute filesystem paths or debug representations enter this seam.

use rnl_runtime::diag::Severity;
use rnl_runtime::problem::PointRef;
use rnl_runtime::solution::{Solution, WirePoint};
use rnl_runtime::{source::MemoryProvider, Options};
use rnl_solver::{solve, SolverOptions, DEFAULT_CONFIG};
use serde_json::{json, Value};

const ENTRY: &str = "/solver-smoke.sp";
const SOURCE: &str = include_str!("../tests/solver-smoke.sp");

fn point_ref(point: &PointRef) -> Value {
    match point {
        PointRef::Origin(object) => json!({ "origin": object }),
        PointRef::Pin { object, pin, terminal } => {
            json!({ "pin": { "object": object, "pin": pin, "terminal": terminal } })
        }
        PointRef::Centre(members) => json!({ "centre": members.iter().map(point_ref).collect::<Vec<_>>() }),
    }
}

fn wire_point(point: &WirePoint) -> Value {
    match point {
        WirePoint::Anchor(point) => json!({ "anchor": point_ref(point) }),
        WirePoint::Branch(branch) => json!({ "branch": branch }),
        WirePoint::Bend((x, y)) => json!({ "bend": [x, y] }),
    }
}

fn solution(solution: &Solution) -> Value {
    json!({
        "placements": solution.placements.iter().map(|placement| json!({
            "object": placement.object, "x": placement.x, "y": placement.y,
            "orient": placement.orient.map(|orient| orient.as_str()),
        })).collect::<Vec<_>>(),
        "branches": solution.branches.iter().map(|branch| json!({
            "at": [branch.at.0, branch.at.1], "junction": branch.junction,
        })).collect::<Vec<_>>(),
        "wires": solution.wires.iter().map(|wire| json!({
            "route": wire.route, "points": wire.points.iter().map(wire_point).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "wire_routes": solution.wire_routes.iter().map(|wire| json!({
            "wire": wire.wire, "points": wire.points.iter().map(wire_point).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// Run the same acceptance solve as native Rust and as an instantiated WebAssembly module.
pub fn solver_test() -> Result<Value, String> {
    let options = SolverOptions::from_toml(DEFAULT_CONFIG).map_err(|error| error.to_string())?;
    if options != SolverOptions::builtin() {
        return Err("parsing the embedded configuration differs from builtin()".into());
    }
    let mut files = MemoryProvider::default();
    files.insert(ENTRY, SOURCE);
    let interpreted = rnl_view::interpret(ENTRY, Some("spice"), &files, &Options::solver())?;
    let errors: Vec<_> = interpreted
        .sink
        .all()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    if !errors.is_empty() {
        return Err(format!("the solver smoke input has errors: {}", errors.join("; ")));
    }
    if interpreted.problems.len() != 1 || interpreted.problems[0].version != "0.3" {
        return Err("the solver smoke input must export exactly one 0.3 level".into());
    }
    let mut levels = Vec::new();
    for problem in &interpreted.problems {
        let solved = solve(problem, &options);
        let report = &solved.report;
        if !report.errors.is_empty() {
            return Err(format!("the solver smoke solve has errors: {}", report.errors.join("; ")));
        }
        if solved.solution.wires.is_empty() || solved.solution.placements.len() < 2 || report.scoring.levels.is_empty()
        {
            return Err("the solver smoke solve must place symbols, route wires and report configured scores".into());
        }
        levels.push(json!({
            "level": problem.level, "version": problem.version,
            "routes": problem.routes.iter().map(|route| &route.net).collect::<Vec<_>>(),
            "solution": solution(&solved.solution),
            "report": {
                "kept": report.kept, "failed": report.failed,
                "discarded": report.discarded.iter().map(|discarded| json!({
                    "fact": discarded.fact, "groups": discarded.groups,
                    "complete": discarded.complete, "undecided": discarded.undecided,
                })).collect::<Vec<_>>(),
                "statements": report.statements.iter().map(|(statement, hint)| json!({
                    "statement": { "file": statement.file, "line": statement.line, "col": statement.col,
                        "chain": statement.chain, "point": statement.point },
                    "facts": hint.facts, "discarded": hint.discarded,
                })).collect::<Vec<_>>(),
                "scoring": report.scoring, "routing_fallbacks": report.routing_fallbacks,
                "inert_objectives": report.inert_objectives,
                "work": { "resolve_nodes": report.work.resolve_nodes, "pivots": report.work.pivots,
                    "candidates": report.work.candidates, "astar_expansions": report.work.astar_expansions },
                "errors": report.errors, "warnings": report.warnings, "notes": report.notes,
            },
        }));
    }
    Ok(json!({ "schema": "rnl-solver-wasm-smoke-v1", "config": options.canonical(), "levels": levels }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_configuration_and_a_real_solve_execute_without_host_io() {
        let first = solver_test().unwrap();
        assert_eq!(solver_test().unwrap(), first);
        assert_eq!(first["levels"][0]["version"], "0.3");
        assert!(first["levels"][0]["solution"]["wires"].as_array().is_some_and(|wires| !wires.is_empty()));
    }
}
