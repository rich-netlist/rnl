use crate::{
    decode_solution, encode_solution,
    json::{Error, Node, Result},
    report, VERSION,
};
use rnl_runtime::{
    problem::{input_digest, LevelProblem},
    solution::Solution,
};
use rnl_solver::SolveReport;
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolverIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LevelResult {
    /// Ordinal in the request, disambiguating identical levels in different files.
    pub request: usize,
    pub level: usize,
    /// The original problem's input digest, independent of run settings.
    pub input: String,
    /// Effective settings used for this result and for its persisted hash.
    pub options: String,
    pub solution: Solution,
    /// Absence means no fact report was supplied, never that every fact was satisfied.
    pub report: Option<SolveReport>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResultDocument {
    pub solver: SolverIdentity,
    pub levels: Vec<LevelResult>,
}

pub fn encode_result(result: &ResultDocument) -> Result<Value> {
    let levels = result
        .levels
        .iter()
        .map(|r| {
            Ok(json!({
                "request":r.request,"level":r.level,"input":r.input,"options":r.options,"solution":encode_solution(&r.solution)?,
                "report":r.report.as_ref().map(report::encode).transpose()?,
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"format":"rnl-solver-result","version":VERSION,"solver":{"name":result.solver.name,"version":result.solver.version},"levels":levels}),
    )
}

fn report_indices(r: &SolveReport, p: &LevelProblem, at: &Node<'_>) -> Result<()> {
    let valid = |i: usize, size: usize, what: &str| {
        if i < size {
            Ok(())
        } else {
            Err(at.error(format!("{what} index {i} is out of range")))
        }
    };
    if let Some(wire) = r.fatal_written_wire {
        valid(wire, p.wires.len(), "fatal written wire")?;
        if r.errors.is_empty() {
            return Err(at.error("fatal written wire requires a diagnostic"));
        }
    }
    let fact = |i| valid(i, p.facts.len(), "fact");
    let mut classified = BTreeSet::new();
    for f in r.kept.iter().chain(&r.failed).copied().chain(r.discarded.iter().map(|d| d.fact)) {
        fact(f)?;
        if !classified.insert(f) {
            return Err(at.error(format!("fact {f} has duplicate or contradictory classifications")));
        }
    }
    for f in r.discarded.iter().flat_map(|d| d.groups.iter().flatten()) {
        fact(*f)?;
    }
    for (statement, hint) in &r.statements {
        for u in &statement.chain {
            valid(*u, p.usages.len(), "usage")?;
        }
        if let Some(k) = statement.point {
            valid(k, p.objects.len(), "point object")?;
        }
        for f in hint.facts.iter().chain(&hint.discarded) {
            fact(*f)?;
        }
    }
    for u in &r.unrouted {
        valid(u.route, p.routes.len(), "route")?;
    }
    Ok(())
}

/// Decode a complete response to the supplied levels. Missing, duplicate, extra, or stale
/// levels are errors before any output file is written.
pub fn decode_result(value: &Value, problems: &[LevelProblem]) -> Result<ResultDocument> {
    let n = Node::root(value);
    if n.field("format")?.string()? != "rnl-solver-result" {
        return Err(n.field("format")?.error("expected rnl-solver-result"));
    }
    if n.field("version")?.uint()? != VERSION {
        return Err(n.field("version")?.error("unsupported solver result protocol version"));
    }
    let solver = n.field("solver")?;
    let solver = SolverIdentity {
        name: solver.field("name")?.string()?.to_owned(),
        version: solver.field("version")?.string()?.to_owned(),
    };
    let mut seen = BTreeSet::new();
    let mut levels = n.field("levels")?.list(|l| {
        let request_node = l.field("request")?;
        let request = request_node.index()?;
        let level = l.field("level")?.index()?;
        let input = l.field("input")?.string()?.to_owned();
        let p = problems.get(request).ok_or_else(|| request_node.error("request index is out of range"))?;
        if p.level != level || input_digest(p) != input {
            return Err(l.error("result level or input digest does not match its request"));
        }
        if !seen.insert(request) {
            return Err(l.error("duplicate result for one requested level"));
        }
        let solution_node = l.field("solution")?;
        let solution = decode_solution(solution_node.value, p).map_err(|e| Error {
            path: format!("{}{}", solution_node.path, e.path.trim_start_matches('$')),
            message: e.message,
        })?;
        let report = l.field("report")?.nullable(report::decode)?;
        let options = l.field("options")?.string()?.to_owned();
        if let Some(report) = &report {
            report_indices(report, p, &l.field("report")?)?;
            if report.canonical_options.as_ref().is_some_and(|canonical| *canonical != options) {
                return Err(l.field("options")?.error("result settings differ from the report's canonical settings"));
            }
        }
        Ok(LevelResult { request, level, input, options, solution, report })
    })?;
    if seen.len() != problems.len() {
        return Err(n.field("levels")?.error("response is missing requested levels"));
    }
    levels.sort_by_key(|l| l.request);
    Ok(ResultDocument { solver, levels })
}
