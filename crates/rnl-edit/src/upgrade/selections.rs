//! Keep electrical filtering before a drawing selection (10.1, 10.2).
use super::edit::{record_edit, Edit};
use rnl_runtime::host::Record;
use rnl_runtime::syntax::{Member, Value};

/// A terminal selection consumed only by TEXT can become its pins. A bound
/// selection or an electrical consumer must retain terminal identity instead.
pub(super) fn drawing_selection(member: &Member, index: usize) -> bool {
    let Some(call) = member.segments.get(index).and_then(|s| s.call.as_ref()) else { return false };
    let Some(params) = &call.params else { return false };
    call.kw == "SELECT-TERMINAL"
        && matches!(params.get("wired"), Some(Value::Bool(_)))
        && member.binds.is_empty()
        && member.segments.len() == index + 2
        && member.segments[index + 1].piped
        && member.segments[index + 1].call.as_ref().is_some_and(|c| c.kw == "TEXT")
        && !super::mechanical::mixed_recursion(member)
}

pub(super) fn drawing_pins(
    source: &str,
    record: &Record,
    member: &Member,
    index: usize,
) -> Result<Option<Edit>, String> {
    if !drawing_selection(member, index) {
        return Ok(None);
    }
    let call = member.segments[index].call.as_ref().unwrap();
    let params = call.params.as_ref().unwrap();
    let recursive = params.get("recursive").cloned().unwrap_or(Value::Bool(true));
    let mut first = params.positional.iter().map(super::rewrite::value).collect::<Vec<_>>();
    for named in &params.named {
        if named.name == "wired" {
            continue;
        }
        let name = if named.quoted { rnl_runtime::problem::written_string(&named.name) } else { named.name.clone() };
        first.push(format!("{name}={}", super::rewrite::value(&named.value)));
    }
    if params.get("recursive").is_none() {
        first.push("recursive=true".into());
    }
    let replacement = format!(
        "@SELECT-TERMINAL({}) => @SELECT-PIN(wired={}, recursive={})",
        first.join(", "),
        super::rewrite::value(params.get("wired").unwrap()),
        super::rewrite::value(&recursive)
    );
    record_edit(source, record, call.start..call.end, replacement, "drawing-pin-selection").map(Some)
}
