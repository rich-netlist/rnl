//! Give local pins names while retaining earlier point-valued bindings (9.2.4–9.2.5).
use super::edit::{record_edit, Edit};
use rnl_runtime::host::Record;
use rnl_runtime::source::normalize;
use rnl_runtime::syntax::{parse_record_features, Member, ObjExpr, Value};
use rnl_runtime::Interpretation;
use std::collections::{BTreeMap, BTreeSet};

struct Insertion {
    at: usize,
    text: String,
}
struct Plan {
    edits: Vec<Insertion>,
    points: bool,
}
#[derive(Default)]
pub(super) struct LocalSymbols {
    plans: BTreeMap<(String, u32, u32, usize), Plan>,
    pub pins: BTreeMap<(usize, String, String), String>,
}

fn plan(member: &Member, reserved: &BTreeSet<String>) -> Option<(Plan, Vec<(String, String)>)> {
    if member.segments.len() != 1 {
        return None;
    }
    let segment = &member.segments[0];
    if segment.call.as_ref()?.kw != "DEFINE-SYMBOL" {
        return None;
    }
    let [arg] = segment.args.as_slice() else { return None };
    let ObjExpr::Set(body) = &arg.obj else { return None };
    let mut names = BTreeSet::new();
    for statement in &body.members {
        for binding in &statement.binds {
            if !names.insert(binding.name.clone()) {
                return None;
            }
        }
        // A bare symbol inclusion can contribute names and displaced bindings.
        if statement
            .segments
            .iter()
            .any(|s| s.call.is_none() && !s.args.iter().all(|a| matches!(a.obj, ObjExpr::Point(_))))
        {
            return None;
        }
    }
    let mut terminals = BTreeSet::new();
    let mut edits = Vec::new();
    let mut pins = Vec::new();
    let mut positional = false;
    let mut points = false;
    let mut next = 1;
    for statement in &body.members {
        let calls: Vec<_> = statement.segments.iter().filter_map(|s| s.call.as_ref()).collect();
        if !calls.iter().any(|c| matches!(c.kw.as_str(), "PIN" | "TEXT")) {
            continue;
        }
        let [segment] = statement.segments.as_slice() else { return None };
        let call = segment.call.as_ref()?;
        let [arg] = segment.args.as_slice() else { return None };
        let point = match &arg.obj {
            ObjExpr::Point(_) => true,
            ObjExpr::Ref(r) => r.prefix == '%' && r.name.is_some() && r.path.is_empty() && r.sub.is_none(),
            _ => false,
        };
        // Attached labels are not point-valued fixed texts.
        if call.kw == "TEXT" && statement.binds.is_empty() {
            continue;
        }
        if !point || segment.piped || statement.binds.len() > 1 || statement.binds.iter().any(|b| b.prefix != '%') {
            return None;
        }
        let fresh = loop {
            let candidate = format!("rnl_upgrade_local_{next}");
            next += 1;
            if !reserved.contains(&candidate) && names.insert(candidate.clone()) {
                break candidate;
            }
        };
        if call.kw == "PIN" {
            if let Some(terminal) = call.params.as_ref().and_then(|p| p.get("terminal")) {
                positional |= matches!(terminal, Value::Num { .. })
                    || matches!(terminal, Value::Str(n) if n.chars().all(|c| c.is_ascii_digit()));
                let key = match terminal {
                    Value::Num { value, .. } => value.to_string(),
                    _ => super::rewrite::value(terminal),
                };
                if matches!(terminal, Value::Arr(_) | Value::Hole(_)) || !terminals.insert(key) {
                    return None;
                }
                if let Value::Str(name) = terminal {
                    if !name.chars().all(|c| c.is_ascii_digit()) {
                        pins.push((name.clone(), fresh.clone()));
                    }
                }
            }
        }
        if let Some(old) = statement.binds.first() {
            points = true;
            edits.push(Insertion {
                at: call.start,
                text: format!("; %{fresh} = %{} ", rnl_runtime::problem::written_name(&old.name)),
            });
        } else {
            edits.push(Insertion { at: statement.text_start(), text: format!("%{fresh} = ") });
        }
    }
    // A numeric binding may resolve to the same role as a named binding, and
    // the old assignment may select the later pin. Names alone cannot prove it.
    if positional {
        pins.clear();
    }
    (!edits.is_empty()).then_some((Plan { edits, points }, pins))
}

impl LocalSymbols {
    pub(super) fn new(input: &Interpretation) -> Self {
        let mut out = Self::default();
        if super::standard::shadows(input).1 {
            return out;
        }
        // Owner roles and text keys share the assigned pin's namespace (9.2.4).
        // Reserve every existing spelling of our generated names, even outside
        // this symbol, so a fresh pin does not capture one of those names.
        let reserved = input
            .sources
            .iter()
            .flat_map(|(_, source)| {
                source
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .filter(|name| name.starts_with("rnl_upgrade_local_"))
                    .map(str::to_owned)
            })
            .collect();
        for (index, level) in input.doc.levels.iter().enumerate() {
            if !matches!(super::effective_version(&input.doc, index).as_str(), "0.2" | "0.3") {
                continue;
            }
            let mut counts = BTreeMap::new();
            for record in &level.records {
                if let Some(body) = parse_record_features(&record.text, true, false).body {
                    for member in &body.members {
                        for binding in member.binds.iter().filter(|b| b.prefix == '$') {
                            *counts.entry(binding.name.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
            for record in &level.records {
                let Some(body) = parse_record_features(&record.text, true, false).body else { continue };
                for member in &body.members {
                    let Some(binding) = member.binds.iter().find(|b| b.prefix == '$') else { continue };
                    if counts.get(&binding.name) != Some(&1) {
                        continue;
                    }
                    let Some((plan, pins)) = plan(member, &reserved) else { continue };
                    for (terminal, pin) in pins {
                        out.pins.insert((index, format!("${}", binding.name), terminal), pin);
                    }
                    out.plans.insert((normalize(&level.path), record.lines.0, record.lines.1, member.start), plan);
                }
            }
        }
        out
    }
    pub(super) fn preserves_points(&self, path: &str, record: &Record, member: &Member) -> bool {
        self.plans.get(&(normalize(path), record.lines.0, record.lines.1, member.start)).is_some_and(|p| p.points)
    }
    pub(super) fn edits(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        member: &Member,
    ) -> Result<Vec<Edit>, String> {
        let Some(plan) = self.plans.get(&(normalize(path), record.lines.0, record.lines.1, member.start)) else {
            return Ok(Vec::new());
        };
        plan.edits
            .iter()
            .map(|e| record_edit(source, record, e.at..e.at, e.text.clone(), "symbol-local-point"))
            .collect()
    }
}
