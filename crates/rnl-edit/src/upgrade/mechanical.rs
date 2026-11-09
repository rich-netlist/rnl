//! Local rewrites that do not need object resolution.
use rnl_runtime::host::Record;
use rnl_runtime::syntax::{Call, Member, ObjExpr, Value};

use super::edit::{record_edit, Edit};

pub(super) fn route(source: &str, record: &Record, call: &Call) -> Result<Option<Edit>, String> {
    if call.kw != "ROUTE" {
        return Ok(None);
    }
    record_edit(source, record, call.start..call.start + "@ROUTE".len(), "@ROUTE-NET".into(), "route-net").map(Some)
}

fn selections<'a>(member: &'a Member, calls: &mut Vec<&'a Call>) {
    for segment in &member.segments {
        if let Some(call) = &segment.call {
            if call.kw.starts_with("SELECT-") {
                calls.push(call);
            }
            if matches!(call.kw.as_str(), "DEFINE" | "DEFINE-SYMBOL") {
                continue;
            }
        }
        for arg in &segment.args {
            if let ObjExpr::Set(set) = &arg.obj {
                for inner in &set.members {
                    selections(inner, calls);
                }
            }
        }
    }
}

/// An old false selection becomes empty below its writing level. In 0.4 the
/// rule reaches below as a whole, so mixing it with true needs contextual work.
pub(super) fn mixed_recursion(member: &Member) -> bool {
    let mut calls = Vec::new();
    selections(member, &mut calls);
    let local = |c: &&Call| c.params.as_ref().and_then(|p| p.get("recursive")) == Some(&Value::Bool(false));
    calls.iter().any(local) && calls.iter().any(|c| !local(c))
}

pub(super) fn recursive(source: &str, record: &Record, call: &Call) -> Result<Option<Edit>, String> {
    if !call.kw.starts_with("SELECT-") || call.params.as_ref().is_some_and(|p| p.get("recursive").is_some()) {
        return Ok(None);
    }
    let (at, text) = match &call.params {
        Some(p) => (
            p.end - 1,
            format!("{}recursive=true", if p.named.is_empty() && p.positional.is_empty() { "" } else { ", " }),
        ),
        None => (call.end, "(recursive=true)".into()),
    };
    record_edit(source, record, at..at, text, "recursive-selection").map(Some)
}

/// The old grammar reads an adjacent brace as another object; 0.4 can read it
/// as member places of the reference. Keep the original object boundary.
pub(super) fn separate_sets(source: &str, record: &Record, member: &Member) -> Result<Vec<Edit>, String> {
    let mut edits = Vec::new();
    for segment in &member.segments {
        for pair in segment.args.windows(2) {
            if let (ObjExpr::Ref(reference), ObjExpr::Set(set)) = (&pair[0].obj, &pair[1].obj) {
                if reference.end == set.start {
                    edits.push(record_edit(source, record, set.start..set.start, " ".into(), "separate-set")?);
                }
            }
        }
    }
    Ok(edits)
}

pub(super) fn distance_bound(call: &Call) -> Option<&'static str> {
    if call.kw != "DISTANCE" {
        return None;
    }
    let params = call.params.as_ref()?;
    if ["min", "max", "exact"]
        .iter()
        .any(|key| matches!(params.get(key), Some(Value::Num { value, .. }) if *value < 0.0))
    {
        return Some("negative distance bounds need an equivalent interval and direction (14.5)");
    }
    if params.get("min").is_none() && params.get("exact").is_none() {
        return Some(
            "the omitted lower bound becomes zero; the original interval may include negative distances (14.5)",
        );
    }
    if ["min", "max", "exact"].iter().any(|key| matches!(params.get(key), Some(Value::Hole(_)))) {
        return Some("a parameterized distance bound must be nonnegative for every application (14.5)");
    }
    None
}
