//! Header and package migration (16.1–16.3).

use std::collections::BTreeSet;

use rnl_runtime::host::{HostDocument, Record};
use rnl_runtime::problem::written_string;
use rnl_runtime::syntax::{parse_record_features, Call, Member, ObjExpr, Params, Value};
use rnl_runtime::writeback::{add_header, carrier_at, record_type};

use super::edit::{apply, discard_generated, record_edit, Edit};
use super::{mechanical, references::References, Change, FileReport, Unconverted};

pub(super) struct Context {
    pub std: bool,
    pub packages: BTreeSet<String>,
    pub references: References,
}

fn pending(member: &Member) -> Option<(&'static str, &'static str)> {
    if mechanical::mixed_recursion(member) {
        return Some(("recursive", "mixed local and recursive selections need rule-level migration (10.1)"));
    }
    for segment in &member.segments {
        let Some(call) = &segment.call else { continue };
        if let Some(reason) = mechanical::distance_bound(call) {
            return Some(("distance-interval", reason));
        }
        let parameters = call.params.as_ref();
        let axis = parameters.is_some_and(|p| {
            p.positional
                .iter()
                .chain(p.named.iter().map(|p| &p.value))
                .any(|v| matches!(v, Value::Str(s) if s == "x" || s == "y"))
        });
        match call.kw.as_str() {
            "DISTRIBUTE" => return Some(("direction", "DISTRIBUTE needs an explicit direction (14.4)")),
            "ORDER" | "DISTANCE" if axis => {
                return Some(("direction", "an axis does not determine a direction (14.3, 14.5)"))
            }
            "AS" | "ATTACH" => {
                return Some(("usage", "the definition, roles and usage point need migration review (9.4, 12.1)"))
            }
            "SELECT-TERMINAL" if parameters.is_some_and(|p| p.get("wired").is_some()) => {
                return Some(("pin-selection", "wired is a pin-selection condition in 0.4 (10.1)"));
            }
            _ => {}
        }
    }
    None
}

fn value(v: &Value) -> String {
    match v {
        Value::Num { text, .. } => text.clone(),
        Value::Str(s) => written_string(s),
        Value::Bool(v) => v.to_string(),
        Value::Arr(a) => format!("[{}]", a.iter().map(value).collect::<Vec<_>>().join(", ")),
        Value::Hole(s) => format!("?{s}"),
        Value::Word(s) => s.clone(),
    }
}

fn package_call(call: &Call, names: &BTreeSet<String>, inline: bool) -> Option<String> {
    let p = call.params.as_ref()?;
    let Value::Str(name) = p.get("name")? else { return None };
    if !names.contains(name) {
        return None;
    }
    let mut parts = p.positional.iter().map(value).collect::<Vec<_>>();
    for named in &p.named {
        if named.name == "version" || (!inline && named.name == "path") {
            continue;
        }
        let key = if named.quoted { written_string(&named.name) } else { named.name.clone() };
        parts.push(format!("{key}={}", value(&named.value)));
    }
    parts.push("version=\"0.4\"".into());
    Some(format!("@{}({})", call.kw, parts.join(", ")))
}

fn visit(members: &[std::rc::Rc<Member>], f: &mut impl FnMut(&Member)) {
    for member in members {
        f(member);
        for segment in &member.segments {
            for arg in &segment.args {
                if let ObjExpr::Set(set) = &arg.obj {
                    visit(&set.members, f);
                }
            }
        }
    }
}

fn uses_std(call: &Call) -> bool {
    call.kw == "USE-PACKAGE" && call.params.as_ref().and_then(|p| p.get("name")) == Some(&Value::Str("rnl-std".into()))
}

pub(super) fn package_paths(records: &[Record], standard: &BTreeSet<String>) -> Vec<String> {
    let mut paths = Vec::new();
    for record in records {
        let ast = parse_record_features(&record.text, true, false);
        let Some(body) = ast.body else { continue };
        visit(&body.members, &mut |member| {
            for call in member.segments.iter().filter_map(|s| s.call.as_ref()) {
                if !matches!(call.kw.as_str(), "USE-PACKAGE" | "USE-ALIASES") {
                    continue;
                }
                let Some(params) = &call.params else { continue };
                if matches!(params.get("name"), Some(Value::Str(n)) if standard.contains(n)) {
                    continue;
                }
                if let Some(Value::Str(path)) = params.get("path") {
                    paths.push(path.clone());
                }
            }
        });
    }
    paths
}

fn body_start(record: &Record, header: Option<&Params>) -> Option<usize> {
    let from = header.map_or(4, |h| h.end);
    record.text.chars().enumerate().skip(from).find(|(_, c)| *c == '{').map(|(i, _)| i + 1)
}

pub(super) fn file(
    doc: &HostDocument,
    path: &str,
    source: &str,
    records: &[Record],
    context: &Context,
    report: &mut FileReport,
) -> Result<String, String> {
    let mut edits: Vec<Edit> = Vec::new();
    let mut header = None;
    let mut has_std = false;
    let mut has_version = false;
    for record in records {
        if record_type(record) == "generated" {
            edits.push(discard_generated(source, record)?);
            report.needs_solve = true;
            continue;
        }
        let ast = parse_record_features(&record.text, report.from != "0.2", false);
        if record_type(record) == "header" {
            header = Some((record, body_start(record, ast.header.as_ref())));
        }
        let Some(body) = ast.body else { continue };
        if record_type(record) == "header" {
            has_std |= body.members.iter().flat_map(|m| &m.segments).filter_map(|s| s.call.as_ref()).any(uses_std);
        }
        let mut error = None;
        let mut local_selection_spans = Vec::new();
        visit(&body.members, &mut |member| {
            if mechanical::mixed_recursion(member) {
                local_selection_spans.push(member.text_start()..member.end);
            }
        });
        visit(&body.members, &mut |member| {
            if let Some((rule, reason)) = pending(member) {
                let at = member.text_start();
                let location = record.loc(at);
                let marker = carrier_at(doc, path, source, location.line);
                let eol = if source.contains("\r\n") { "\r\n" } else { "\n" };
                let prefix = if marker.is_empty() { String::new() } else { format!("{marker} ") };
                let replacement = format!("/* rnl upgrade: {reason} */{eol}{prefix}");
                match record_edit(source, record, at..at, replacement, "unconverted-note") {
                    Ok(e) => edits.push(e),
                    Err(e) => error = Some(e),
                }
                report.unconverted.push(Unconverted {
                    rule: rule.into(),
                    line: location.line,
                    before: record.text.chars().skip(at).take(member.end.saturating_sub(at)).collect(),
                    reason: reason.into(),
                });
            }
            match mechanical::separate_sets(source, record, member) {
                Ok(found) => edits.extend(found),
                Err(e) => error = Some(e),
            }
            for segment in &member.segments {
                match context.references.role_arguments(path, source, record, segment) {
                    Ok(found) => edits.extend(found),
                    Err(e) => error = Some(e),
                }
                for arg in &segment.args {
                    if let ObjExpr::Ref(reference) = &arg.obj {
                        match context.references.role_reference(path, source, record, reference) {
                            Ok(Some(edit)) => {
                                edits.push(edit);
                                continue;
                            }
                            Ok(None) => {}
                            Err(e) => error = Some(e),
                        }
                        match context.references.usage_pin(path, source, record, reference) {
                            Ok(Some(edit)) => edits.push(edit),
                            Ok(None) => {}
                            Err(e) => error = Some(e),
                        }
                    }
                }
                let Some(call) = &segment.call else { continue };
                let mut found = vec![mechanical::route(source, record, call)];
                if !local_selection_spans.iter().any(|range| range.contains(&call.start)) {
                    found.push(mechanical::recursive(source, record, call));
                }
                for edit in found {
                    match edit {
                        Ok(Some(e)) => edits.push(e),
                        Ok(None) => {}
                        Err(e) => error = Some(e),
                    }
                }
                let change = match call.kw.as_str() {
                    "VERSION" => {
                        has_version = true;
                        Some(("language-version", "@VERSION(\"0.4\")".to_string()))
                    }
                    "USE-PACKAGE" | "USE-ALIASES" => {
                        let inline = segment.args.iter().any(|a| matches!(a.obj, ObjExpr::Set(_)));
                        package_call(call, &context.packages, inline).map(|s| ("standard-series", s))
                    }
                    _ => None,
                };
                if let Some((rule, replacement)) = change {
                    match record_edit(source, record, call.start..call.end, replacement, rule) {
                        Ok(e) if e.change.before != e.change.after => edits.push(e),
                        Ok(_) => {}
                        Err(e) => error = Some(e),
                    }
                }
            }
        });
        if let Some(error) = error {
            return Err(error);
        }
    }
    let mut declarations = Vec::new();
    if !has_version {
        declarations.push("@VERSION(\"0.4\")");
    }
    if context.std && !has_std {
        declarations.push("@USE-PACKAGE(name=\"rnl-std\", version=\"0.4\")");
    }
    if let Some((record, Some(at))) = header {
        if !declarations.is_empty() {
            edits.push(record_edit(
                source,
                record,
                at..at,
                format!(" {};", declarations.join("; ")),
                "explicit-header",
            )?);
        }
    }
    let mut text = apply(source, &mut edits)?;
    report.changes.extend(edits.into_iter().map(|e| e.change));
    if header.is_none() {
        let old = text.clone();
        text = add_header(doc, path, &text, |_| format!("@RNL(type=\"header\") {{ {} }}", declarations.join("; ")));
        if text == old {
            return Err(format!("cannot insert a header in {path}"));
        }
        report.changes.push(Change {
            rule: "explicit-header".into(),
            line: old.lines().zip(text.lines()).take_while(|(a, b)| a == b).count() as u32 + 1,
            before: String::new(),
            after: declarations.join("; "),
        });
    }
    Ok(text)
}
