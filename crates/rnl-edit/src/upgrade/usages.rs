//! Preserve explicit usage defaults and the old main-role point relation.
use super::edit::{record_edit, Edit};
use super::{FileReport, Unconverted};
use rnl_runtime::host::Record;
use rnl_runtime::problem::{Filler, Position};
use rnl_runtime::source::normalize;
use rnl_runtime::syntax::{Member, ObjExpr};
use rnl_runtime::Interpretation;
use std::collections::BTreeMap;

#[derive(Clone, PartialEq)]
struct Plan {
    role: String,
    align: bool,
    net_role: bool,
    gap: bool,
    notice: Option<super::manual::Reason>,
    fresh: String,
}
#[derive(Default)]
pub(super) struct Usages {
    plans: BTreeMap<(usize, String, u32, u32), Vec<Plan>>,
}
impl Usages {
    pub(super) fn new(input: &Interpretation, policies: &BTreeMap<(usize, String), super::standard::Policy>) -> Self {
        let mut out = Self::default();
        let mut names = BTreeMap::new();
        let roles = super::standard::role_changes(input);
        for problem in &input.problems {
            for usage in &problem.usages {
                if usage.generated || usage.parent.is_some() {
                    continue;
                }
                let Some(loc) = &usage.source else { continue };
                let policy = policies.get(&(problem.level, usage.definition.clone()));
                let align = usage
                    .object
                    .and_then(|i| problem.objects.get(i))
                    .is_some_and(|o| matches!(o.position, Position::At(_)))
                    && !policy.is_some_and(|p| p.tied);
                let role = usage.roles.first().map(|r| r.role.clone()).unwrap_or_default();
                let net_role = policy.is_some()
                    && matches!(usage.definition.as_str(), "$GROUND" | "$SUPPLY" | "$IO_PORT" | "$GND" | "$IOPORT")
                    && usage.roles.first().is_some_and(|r| matches!(r.members.as_slice(), [Filler::Terminal { .. }]));
                let role = roles
                    .get(&(problem.level, usage.definition.clone()))
                    .and_then(|r| r.get(&role))
                    .cloned()
                    .unwrap_or(role);
                let key = (normalize(&loc.file), loc.line, loc.col);
                if !names.contains_key(&key) {
                    let mut n = names.len();
                    let fresh = loop {
                        let name = format!("rnl_upgrade_usage_{n}");
                        if !input.sources.iter().any(|(_, s)| s.contains(&name)) && !names.values().any(|v| v == &name)
                        {
                            break name;
                        }
                        n += 1;
                    };
                    names.insert(key.clone(), fresh);
                }
                out.plans.entry((problem.level, key.0.clone(), key.1, key.2)).or_default().push(Plan {
                    role,
                    align,
                    net_role,
                    gap: policy.is_some_and(|p| p.gap),
                    notice: policy.and_then(|p| p.notice),
                    fresh: names[&key].clone(),
                });
            }
        }
        out
    }
    pub(super) fn edits(
        &self,
        source: &str,
        record: &Record,
        member: &Member,
        readings: &[usize],
        report: &mut FileReport,
        resolve: impl Fn(&rnl_runtime::syntax::Ref) -> Result<Option<Edit>, String>,
    ) -> Result<(Vec<Edit>, bool), String> {
        let applications = member
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.call.as_ref().is_some_and(|c| matches!(c.kw.as_str(), "AS" | "ATTACH")))
            .collect::<Vec<_>>();
        let [(index, segment)] = applications.as_slice() else { return Ok((Vec::new(), false)) };
        let call = segment.call.as_ref().unwrap();
        let loc = record.loc(call.start);
        let start = record.loc(member.text_start());
        let end = record.loc(member.end);
        let mut found = readings.iter().map(|reading| {
            let plans = self
                .plans
                .iter()
                .filter(|((level, file, line, col), _)| {
                    level == reading
                        && *file == normalize(&loc.file)
                        && (*line, *col) >= (start.line, start.col)
                        && (*line, *col) <= (end.line, end.col)
                })
                .flat_map(|(_, p)| p)
                .collect::<Vec<_>>();
            if plans.len() == 1 || (!plans.is_empty() && !plans[0].align && plans.iter().all(|p| *p == plans[0])) {
                Some(plans[0])
            } else {
                None
            }
        });
        let Some(Some(plan)) = found.next() else { return Ok((Vec::new(), false)) };
        if found.any(|p| p != Some(plan)) {
            return Ok((Vec::new(), false));
        }
        let mut edits = Vec::new();
        if let Some((rule, reason)) = plan.notice {
            report.notices.push(Unconverted {
                rule: rule.into(),
                line: loc.line,
                before: record.text.chars().skip(member.text_start()).take(member.end - member.text_start()).collect(),
                reason: reason.into(),
            });
        }
        if plan.gap && call.params.as_ref().is_none_or(|p| p.get("gap").is_none() && p.positional.is_empty()) {
            let mut params = call
                .params
                .as_ref()
                .map(|p| {
                    p.named
                        .iter()
                        .map(|n| {
                            format!(
                                "{}={}",
                                rnl_runtime::problem::written_name(&n.name),
                                super::rewrite::value(&n.value)
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            params.push("gap=2".into());
            edits.push(record_edit(
                source,
                record,
                call.start..call.end,
                format!("@{}({})", call.kw, params.join(", ")),
                "standard-parameter-default",
            )?);
        }
        if !plan.align {
            return Ok((edits, false));
        }
        // Inserting a binding must name this usage, never a later TEXT or another
        // result of the pipeline. Existing scope-changing pipelines need review.
        if segment.piped
            || member.segments[*index + 1..].iter().any(|s| {
                s.call.as_ref().is_none_or(|c| !matches!(c.kw.as_str(), "SET-SYMBOL" | "ORIENT" | "PLACE" | "PRIORITY"))
            })
        {
            return Ok((edits, false));
        }
        let binding = member.binds.iter().find(|b| b.prefix == '%' && b.after >= *index);
        let name = binding.map(|b| b.name.as_str()).unwrap_or(&plan.fresh);
        let usage = format!("%{}", rnl_runtime::problem::written_name(name));
        let target = if plan.net_role {
            let Some(arg) = segment.args.iter().find(|a| {
                a.role.as_deref() == Some("p")
                    || (a.role.is_none() && !matches!(&a.obj, ObjExpr::Ref(r) if r.prefix == '$'))
            }) else {
                return Ok((edits, false));
            };
            let ObjExpr::Ref(r) = &arg.obj else { return Ok((edits, false)) };
            // Host terminals retain their binding through explicit symbol maps.
            // Usage-terminal paths require an additional named-pin resolution.
            if r.prefix != '#' || r.path.is_empty() {
                return Ok((edits, false));
            }
            let target: String = match resolve(r)? {
                Some(edit) => edit.change.after,
                None => record.text.chars().skip(r.start).take(r.end - r.start).collect(),
            };
            edits.push(record_edit(source, record, r.end..r.end, "::net".into(), "usage-net-role")?);
            target
        } else {
            format!("{usage}::{}", rnl_runtime::problem::written_name(&plan.role))
        };
        if binding.is_none() {
            edits.push(record_edit(
                source,
                record,
                segment.end..segment.end,
                format!(" => {usage} "),
                "usage-point-name",
            )?);
        }
        edits.push(record_edit(
            source,
            record,
            member.end..member.end,
            format!("; {{{usage} {target}}} @ALIGN(\"x\"); {{{usage} {target}}} @ALIGN(\"y\")"),
            "usage-main-point",
        )?);
        Ok((edits, true))
    }
}
