use crate::json::{Node, Result};
use rnl_solver::{
    BudgetReport, Discarded, RunLimit, SolveReport, Statement, StatementHint, StopReason, Unrouted, Work,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn limit_value(limit: RunLimit) -> Value {
    match limit {
        RunLimit::Time(seconds) => json!({"kind":"time","seconds":seconds}),
        RunLimit::Work(work) => json!({"kind":"work","work":work}),
        RunLimit::Seed => json!({"kind":"seed"}),
        RunLimit::Unlimited => json!({"kind":"unlimited"}),
    }
}

/// Encode the shared budget shape used by results and experimental previews.
pub fn encode_budget(b: BudgetReport) -> Result<Value> {
    let value = json!({"limit":limit_value(b.limit),"work":b.work,"reason":match b.reason {
            StopReason::Complete=>"complete",StopReason::Time=>"time",StopReason::Work=>"work",StopReason::Safety=>"safety",
    }});
    budget(Node::root(&value))?;
    Ok(value)
}

pub(crate) fn encode(r: &SolveReport) -> Result<Value> {
    let mut value = json!({
        "budget":r.budget.map(encode_budget).transpose()?,
        "canonical_options":r.canonical_options,
        "kept":r.kept,"failed":r.failed,
        "discarded":r.discarded.iter().map(|d|json!({"fact":d.fact,"groups":d.groups,"complete":d.complete,"undecided":d.undecided})).collect::<Vec<_>>(),
        "statements":r.statements.iter().map(|(s,h)|json!({"statement":{"file":s.file,"line":s.line,"col":s.col,"chain":s.chain,"point":s.point},"facts":h.facts,"discarded":h.discarded})).collect::<Vec<_>>(),
        "inert_objectives":r.inert_objectives,
        "unrouted":r.unrouted.iter().map(|u|json!({"route":u.route,"net":u.net,"parts":u.parts,"budget":u.budget})).collect::<Vec<_>>(),
        "work":{"resolve_nodes":r.work.resolve_nodes,"pivots":r.work.pivots,"candidates":r.work.candidates,"astar_expansions":r.work.astar_expansions},
        "errors":r.errors,"warnings":r.warnings,"notes":r.notes,
    });
    if let Some(index) = r.fatal_annotation {
        value["fatal_annotation"] = json!(index);
    }
    if let Some(wire) = r.fatal_written_wire {
        value["fatal_written_wire"] = json!(wire);
    }
    decode(Node::root(&value))?;
    Ok(value)
}

fn limit(n: Node<'_>) -> Result<RunLimit> {
    Ok(match n.field("kind")?.string()? {
        "time" => {
            let value = n.field("seconds")?.number()?;
            if value < 0. {
                return Err(n.field("seconds")?.error("seconds must not be negative"));
            }
            RunLimit::Time(value)
        }
        "work" => RunLimit::Work(n.field("work")?.uint()?),
        "seed" => RunLimit::Seed,
        "unlimited" => RunLimit::Unlimited,
        _ => return Err(n.field("kind")?.error("unknown budget limit")),
    })
}

fn strings(n: Node<'_>) -> Result<Vec<String>> {
    n.list(|n| Ok(n.string()?.to_owned()))
}
fn indices(n: Node<'_>) -> Result<Vec<usize>> {
    n.list(|n| n.index())
}

pub(crate) fn decode(n: Node<'_>) -> Result<SolveReport> {
    let budget = n.field("budget")?.nullable(budget)?;
    let mut statements = BTreeMap::new();
    for (i, (statement, hint)) in n
        .field("statements")?
        .list(|s| {
            let st = s.field("statement")?;
            let line = |key| {
                let at = st.field(key)?;
                u32::try_from(at.uint()?).map_err(|_| at.error("source position exceeds u32"))
            };
            Ok((
                Statement {
                    file: st.field("file")?.string()?.to_owned(),
                    line: line("line")?,
                    col: line("col")?,
                    chain: indices(st.field("chain")?)?,
                    point: st.field("point")?.nullable(|p| p.index())?,
                },
                StatementHint { facts: indices(s.field("facts")?)?, discarded: indices(s.field("discarded")?)? },
            ))
        })?
        .into_iter()
        .enumerate()
    {
        if statements.insert(statement, hint).is_some() {
            return Err(n.field("statements")?.error(format!("duplicate statement at index {i}")));
        }
    }
    let work = n.field("work")?;
    Ok(SolveReport {
        fatal_annotation: if n.value.get("fatal_annotation").is_some() {
            Some(n.field("fatal_annotation")?.index()?)
        } else {
            None
        },
        fatal_written_wire: if n.value.get("fatal_written_wire").is_some() {
            Some(n.field("fatal_written_wire")?.index()?)
        } else {
            None
        },
        budget,
        canonical_options: n.field("canonical_options")?.nullable(|n| Ok(n.string()?.to_owned()))?,
        kept: indices(n.field("kept")?)?,
        failed: indices(n.field("failed")?)?,
        discarded: n.field("discarded")?.list(|d| {
            Ok(Discarded {
                fact: d.field("fact")?.index()?,
                groups: d.field("groups")?.list(indices)?,
                complete: d.field("complete")?.boolean()?,
                undecided: d.field("undecided")?.boolean()?,
            })
        })?,
        statements,
        inert_objectives: strings(n.field("inert_objectives")?)?,
        unrouted: n.field("unrouted")?.list(|u| {
            Ok(Unrouted {
                route: u.field("route")?.index()?,
                net: u.field("net")?.string()?.to_owned(),
                parts: u.field("parts")?.index()?,
                budget: u.field("budget")?.boolean()?,
            })
        })?,
        work: Work {
            resolve_nodes: work.field("resolve_nodes")?.uint()?,
            pivots: work.field("pivots")?.uint()?,
            candidates: work.field("candidates")?.uint()?,
            astar_expansions: work.field("astar_expansions")?.uint()?,
        },
        errors: strings(n.field("errors")?)?,
        warnings: strings(n.field("warnings")?)?,
        notes: strings(n.field("notes")?)?,
    })
}

fn budget(n: Node<'_>) -> Result<BudgetReport> {
    Ok(BudgetReport {
        limit: limit(n.field("limit")?)?,
        work: n.field("work")?.uint()?,
        reason: match n.field("reason")?.string()? {
            "complete" => StopReason::Complete,
            "time" => StopReason::Time,
            "work" => StopReason::Work,
            "safety" => StopReason::Safety,
            _ => return Err(n.field("reason")?.error("unknown stop reason")),
        },
    })
}

#[cfg(test)]
mod fatal_tests {
    use super::*;

    #[test]
    fn fatal_is_explicit_and_legacy_report_has_no_new_field() {
        let legacy = SolveReport::default();
        let value = encode(&legacy).unwrap();
        assert!(value.get("fatal_written_wire").is_none());
        assert_eq!(decode(Node::root(&value)).unwrap(), legacy);
        let mut fatal = legacy;
        fatal.fatal_written_wire = Some(7);
        fatal.errors.push("wire geometry failed".into());
        let mut value = encode(&fatal).unwrap();
        assert_eq!(decode(Node::root(&value)).unwrap(), fatal);
        value["fatal_written_wire"] = json!("7");
        assert!(decode(Node::root(&value)).is_err());
    }
}
