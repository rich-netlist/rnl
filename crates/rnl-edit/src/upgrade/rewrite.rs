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
    pub owned_packages: BTreeSet<String>,
    pub references: References,
    pub sets: super::sets::Sets,
    pub approximate: bool,
}

fn pending(member: &Member) -> Option<(&'static str, &'static str)> {
    if mechanical::mixed_recursion(member) {
        return Some(("recursive", "mixed local and recursive selections need rule-level migration (10.1)"));
    }
    for (index, segment) in member.segments.iter().enumerate() {
        let Some(call) = &segment.call else { continue };
        if super::directions::replacement(member, index).is_none() {
            if let Some(reason) = mechanical::distance_bound(call) {
                return Some(("distance-interval", reason));
            }
        }
        let parameters = call.params.as_ref();
        let axis = parameters.is_some_and(|p| {
            p.positional
                .iter()
                .chain(p.named.iter().map(|p| &p.value))
                .any(|v| matches!(v, Value::Str(s) if s == "x" || s == "y"))
        });
        match call.kw.as_str() {
            "DISTRIBUTE" if super::directions::replacement(member, index).is_none() => {
                return Some(("direction", "DISTRIBUTE needs an explicit direction (14.4)"))
            }
            "ORDER" | "DISTANCE" if axis && super::directions::replacement(member, index).is_none() => {
                return Some(("direction", "an axis does not determine a direction (14.3, 14.5)"))
            }
            "SELECT-TERMINAL"
                if parameters.is_some_and(|p| p.get("wired").is_some())
                    && !super::selections::drawing_selection(member, index) =>
            {
                return Some(("pin-selection", "wired is a pin-selection condition in 0.4 (10.1)"));
            }
            "SELECT-TERMINAL" if !super::selections::drawing_selection(member, index) => {
                return Some(("terminal-selection", "terminal selection now includes terminals without pins; preserve the original drawn membership for consumers that need electrical terminals (10.1, 10.2)"));
            }
            _ => {}
        }
    }
    None
}

pub(super) fn value(v: &Value) -> String {
    match v {
        Value::Num { text, .. } => text.clone(),
        Value::Str(s) => written_string(s),
        Value::Bool(v) => v.to_string(),
        Value::Arr(a) => format!("[{}]", a.iter().map(value).collect::<Vec<_>>().join(", ")),
        Value::Hole(s) => format!("?{s}"),
        Value::Word(s) => s.clone(),
    }
}

fn renamed_standard_package(kw: &str, name: &str) -> Option<&'static str> {
    match (kw, name) {
        ("USE-PACKAGE", "rnl-std" | "rnl-std-full") => Some("rnl-std-all"),
        ("USE-PACKAGE", "rnl-std-structures") => Some("rnl-std-semantics"),
        ("USE-ALIASES" | "USE-PACKAGE", "rnl-zh") => Some("alias-zh"),
        _ => None,
    }
}

fn package_call(call: &Call, names: &BTreeSet<String>, owned: &BTreeSet<String>, inline: bool) -> Option<String> {
    let p = call.params.as_ref()?;
    let Value::Str(name) = p.get("name")? else { return None };
    if owned.contains(name) {
        return None;
    }
    let target_name =
        renamed_standard_package(&call.kw, name).or_else(|| names.contains(name).then_some(name.as_str()))?;
    let mut parts = p.positional.iter().map(value).collect::<Vec<_>>();
    for named in &p.named {
        if named.name == "version" || (!inline && named.name == "path") {
            continue;
        }
        let key = if named.quoted { written_string(&named.name) } else { named.name.clone() };
        let val = if named.name == "name" { written_string(target_name) } else { value(&named.value) };
        parts.push(format!("{key}={val}"));
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

fn uses_std(segment: &rnl_runtime::syntax::Segment, owned: &BTreeSet<String>) -> bool {
    let Some(call) = &segment.call else { return false };
    if call.kw != "USE-PACKAGE" {
        return false;
    }
    let Some(Value::Str(name)) = call.params.as_ref().and_then(|p| p.get("name")) else { return false };
    if owned.contains(name) {
        return false;
    }
    let inline = segment.args.iter().any(|a| matches!(a.obj, ObjExpr::Set(_)));
    matches!(name.as_str(), "rnl-std" | "rnl-std-all" | "rnl-std-full")
        || (!inline && matches!(name.as_str(), "rnl-std-core" | "rnl-std-symbols"))
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
            has_std |= body.members.iter().flat_map(|m| &m.segments).any(|s| uses_std(s, &context.owned_packages));
        }
        let mut error = None;
        let mut local_selection_spans = Vec::new();
        visit(&body.members, &mut |member| {
            if mechanical::mixed_recursion(member) {
                local_selection_spans.push(member.text_start()..member.end);
            }
        });
        visit(&body.members, &mut |member| {
            let mut reasons = super::manual::reasons(member);
            for reason in context.references.collection_reviews(path, record, member) {
                if !reasons.iter().any(|r| r.0 == reason.0) {
                    reasons.push(reason);
                }
            }
            if context.references.local_symbols.preserves_points(path, record, member) {
                reasons.retain(|(rule, _)| *rule != "symbol-text-result");
            }
            match context.references.local_symbols.edits(path, source, record, member) {
                Ok(found) => edits.extend(found),
                Err(e) => error = Some(e),
            }
            reasons.extend(context.references.usage_reviews(path, record, member));
            match context.references.usage_changes(path, source, record, member, context.approximate, report) {
                Ok((found, preserved)) => {
                    edits.extend(found);
                    if preserved {
                        reasons.retain(|(rule, _)| *rule != "usage-main-point");
                    }
                }
                Err(e) => error = Some(e),
            }
            reasons.extend(context.references.unresolved_pins(path, source, record, member));
            reasons.extend(pending(member));
            let readings =
                context.references.readings_at(path, record, member.start).map(Vec::as_slice).unwrap_or_default();
            match context.sets.unrouting(source, record, member, readings) {
                Ok(Ok(found)) => edits.extend(found),
                Ok(Err(reason)) => reasons.push(reason),
                Err(e) => error = Some(e),
            }
            if member.segments.iter().flat_map(|s| &s.args).any(|a| matches!(&a.obj, ObjExpr::Ref(r) if context.references.removed_standard_name(path, record, r) && !matches!(context.sets.reference(source, record, r), Ok(Some(_)))))
                && !reasons.iter().any(|(rule, _)| *rule == "standard-definition-removed") {
                reasons.push(("standard-name-removed", "the referenced standard name has no direct target in series 0.4; retain its original policy or choose an explicit replacement (16.3)"));
            }
            match context.references.approximate(path, source, record, member, context.approximate, report) {
                Ok(outcome) => {
                    reasons.retain(|(rule, _)| !outcome.resolved.contains(rule));
                    reasons.extend(outcome.review);
                    edits.extend(outcome.edits);
                }
                Err(e) => error = Some(e),
            }
            for (rule, reason) in reasons {
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
            for (index, segment) in member.segments.iter().enumerate() {
                match context.references.role_arguments(path, source, record, segment) {
                    Ok(found) => edits.extend(found),
                    Err(e) => error = Some(e),
                }
                for arg in &segment.args {
                    if let ObjExpr::Ref(reference) = &arg.obj {
                        if context.approximate {
                            match context.references.approximate_role(path, source, record, reference) {
                                Ok(Some(edit)) => {
                                    report.approximations.push(super::Approximation { rule: edit.change.rule.clone(), line: edit.change.line, before: edit.change.before.clone(), after: edit.change.after.clone(), risk: "Role reference follows an opted-in differential-pair replacement; that definition introduces additional facts and assertions.".into() });
                                    edits.push(edit);
                                    continue;
                                }
                                Ok(None) => {}
                                Err(e) => error = Some(e),
                            }
                        }
                        match context.references.host_terminal(path, source, record, reference) {
                            Ok(Some(edit)) => {
                                edits.push(edit);
                                continue;
                            }
                            Ok(None) => {}
                            Err(e) => error = Some(e),
                        }
                        match context.references.legacy_reference(path, source, record, reference) {
                            Ok(Some(edit)) => {
                                if !edits.iter().any(|e| e.start < edit.end && edit.start < e.end) {
                                    edits.push(edit);
                                }
                                continue;
                            }
                            Ok(None) => {}
                            Err(e) => error = Some(e),
                        }
                        match context.sets.reference(source, record, reference) {
                            Ok(Some(edit)) => {
                                edits.push(edit);
                                continue;
                            }
                            Ok(None) => {}
                            Err(e) => error = Some(e),
                        }
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
                let drawing = super::selections::drawing_selection(member, index);
                let mut found = vec![
                    context.references.legacy_package(path, source, record, segment),
                    context.references.terminal_map(path, source, record, member, index),
                    context.references.text_key(path, source, record, member, index),
                    mechanical::route(source, record, call),
                    super::directions::edit(source, record, member, index),
                    super::selections::drawing_pins(source, record, member, index),
                ];
                if !drawing && !local_selection_spans.iter().any(|range| range.contains(&call.start)) {
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
                        package_call(call, &context.packages, &context.owned_packages, inline)
                            .map(|s| ("standard-series", s))
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
        declarations.push("@VERSION(\"0.4\")".to_string());
    }
    if context.std && !has_std {
        declarations.push("@USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\")".to_string());
    }
    for package in context.references.legacy_packages(path, records) {
        declarations.push(format!("@USE-PACKAGE(name={}, version=\"0.4\")", written_string(&package)));
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
    if let (Some((record, _)), Some(seed)) = (header, context.sets.seed(path)) {
        let at = record.text.chars().count();
        let location = record.loc(at);
        let marker = carrier_at(doc, path, source, location.line);
        let eol = if source.contains("\r\n") { "\r\n" } else { "\n" };
        let prefix = if marker.is_empty() { String::new() } else { format!("{marker} ") };
        edits.push(record_edit(source, record, at..at, format!("{eol}{prefix}{seed}"), "explicit-fill-policy")?);
    }
    let mut text = apply(source, &mut edits)?;
    report.changes.extend(edits.into_iter().filter(|e| !e.change.rule.starts_with("approximate-")).map(|e| e.change));
    if header.is_none() {
        let old = text.clone();
        text = add_header(doc, path, &text, |_| {
            let header = format!("@RNL(type=\"header\") {{ {} }}", declarations.join("; "));
            context.sets.seed(path).map_or(header.clone(), |seed| format!("{header}\n{seed}"))
        });
        if text == old {
            return Err(format!("cannot insert a header in {path}"));
        }
        if let Some(seed) = context.sets.seed(path) {
            report.changes.push(Change {
                rule: "explicit-fill-policy".into(),
                line: old.lines().zip(text.lines()).take_while(|(a, b)| a == b).count() as u32 + 1,
                before: String::new(),
                after: seed.into(),
            });
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
