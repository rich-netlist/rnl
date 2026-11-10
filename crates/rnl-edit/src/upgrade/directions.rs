//! Direction changes with an algebraic justification (14.3–14.5).
use super::edit::{record_edit, Edit};
use rnl_runtime::host::Record;
use rnl_runtime::syntax::{Call, Member, ObjExpr, Segment, Value};

/// min(B) - max(A) >= d iff every pair has distance >= d (14.5).
/// A maximum or exact bound does not have this property, even with one singleton.
pub(super) fn equivalent_set_distance(segment: &Segment) -> bool {
    let Some(call) = segment.call.as_ref().filter(|c| c.kw == "DISTANCE") else { return false };
    let Some(p) = &call.params else { return false };
    let Some(Value::Str(direction)) = p.positional.first().or_else(|| p.get("direction")) else { return false };
    if opposite(direction).is_none() {
        return false;
    }
    if p.get("max").is_none() && p.get("exact").is_none() {
        return number(p.get("min")).is_some_and(|n| n >= 0.0);
    }
    // Both slots must denote one point, and the old range must already impose
    // the nonnegative ordering that 0.4 adds to a maximum-only constraint.
    !segment.piped
        && segment.args.len() == 2
        && segment.args.iter().all(|a| singleton(&a.obj))
        && (number(p.get("exact")).is_some_and(|n| n >= 0.0) || number(p.get("min")).is_some_and(|n| n >= 0.0))
}

pub(super) fn singleton(object: &ObjExpr) -> bool {
    match object {
        ObjExpr::Point(_) => true,
        ObjExpr::Ref(r) => {
            r.prefix == '#' && r.name.is_some() && r.sub.is_none() && r.places.is_none() && r.path.is_empty()
        }
        ObjExpr::Set(s) => {
            s.members.len() == 1 && s.members[0].segments.len() == 1 && {
                let s = &s.members[0].segments[0];
                s.call.is_none() && s.args.len() == 1 && singleton(&s.args[0].obj)
            }
        }
    }
}

fn number(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Num { value, .. }) => Some(*value),
        _ => None,
    }
}
fn individual_points(member: &Member, index: usize) -> bool {
    let segment = &member.segments[index];
    !segment.piped
        && segment.args.len() == 2
        && segment.args.iter().all(|a| match &a.obj {
            ObjExpr::Point(_) => true,
            ObjExpr::Ref(r) => {
                r.prefix == '#' && r.name.is_some() && r.sub.is_none() && r.places.is_none() && r.path.is_empty()
            }
            _ => false,
        })
}
fn opposite(direction: &str) -> Option<&'static str> {
    Some(match direction {
        "left-to-right" => "right-to-left",
        "right-to-left" => "left-to-right",
        "top-to-bottom" => "bottom-to-top",
        "bottom-to-top" => "top-to-bottom",
        _ => return None,
    })
}

/// Keep all unrelated parameters, including priority. A crossing interval cannot
/// become a single nonnegative directed interval and is deliberately left alone.
pub(super) fn replacement(member: &Member, index: usize) -> Option<String> {
    let call = member.segments.get(index)?.call.as_ref()?;
    if !matches!(call.kw.as_str(), "ORDER" | "DISTRIBUTE" | "DISTANCE") {
        return None;
    }
    let params = call.params.as_ref()?;
    let axis_key = if call.kw == "DISTRIBUTE" { "axis" } else { "direction" };
    let Value::Str(axis) = params.positional.first().or_else(|| params.get(axis_key))? else { return None };
    let mut direction = axis.clone();
    let mut interval: Option<Vec<(String, f64)>> = None;
    if call.kw == "DISTANCE" && opposite(axis).is_some() {
        // Extent gaps of sets do not negate like the signed distance of two points.
        if !individual_points(member, index) {
            return None;
        }
        if let Some(exact) = number(params.get("exact")).filter(|n| *n < 0.0) {
            direction = opposite(axis)?.into();
            interval = Some(vec![("exact".into(), -exact)]);
        } else if params.get("exact").is_none() {
            let max = number(params.get("max"))?;
            let min = number(params.get("min"));
            if max > 0.0 || min.is_some_and(|n| n > 0.0) || (params.get("min").is_some() && min.is_none()) {
                return None;
            }
            direction = opposite(axis)?.into();
            let mut values = vec![("min".into(), -max)];
            if let Some(min) = min {
                values.push(("max".into(), -min));
            }
            interval = Some(values);
        }
    } else if matches!(axis.as_str(), "x" | "y") {
        // Coordinates are discardable facts. They cannot choose one half of an
        // undirected constraint without changing conflict resolution. Only a
        // zero point distance is independent of which direction is chosen.
        let zero = number(params.get("exact")) == Some(0.0)
            || (params.get("exact").is_none()
                && number(params.get("min")) == Some(0.0)
                && number(params.get("max")) == Some(0.0));
        if call.kw != "DISTANCE" || !zero || !individual_points(member, index) {
            return None;
        }
        direction = if axis == "x" { "left-to-right" } else { "bottom-to-top" }.into();
    }
    if direction == *axis {
        return None;
    }
    Some(render(call, &direction, axis_key, interval))
}
pub(super) fn render(call: &Call, direction: &str, axis_key: &str, interval: Option<Vec<(String, f64)>>) -> String {
    let params = call.params.as_ref().unwrap();
    let mut values = vec![rnl_runtime::problem::written_string(direction)];
    values.extend(params.positional.iter().skip(1).map(super::rewrite::value));
    for named in &params.named {
        if named.name == axis_key || (interval.is_some() && matches!(named.name.as_str(), "min" | "max" | "exact")) {
            continue;
        }
        let key = if named.quoted { rnl_runtime::problem::written_string(&named.name) } else { named.name.clone() };
        values.push(format!("{key}={}", super::rewrite::value(&named.value)));
    }
    if let Some(interval) = interval {
        values.extend(interval.into_iter().map(|(key, n)| format!("{key}={}", if n == 0.0 { 0.0 } else { n })));
    }
    format!("@{}({})", call.kw, values.join(", "))
}
pub(super) fn edit(source: &str, record: &Record, member: &Member, index: usize) -> Result<Option<Edit>, String> {
    let Some(replacement) = replacement(member, index) else { return Ok(None) };
    let call = member.segments[index].call.as_ref().unwrap();
    record_edit(source, record, call.start..call.end, replacement, "explicit-direction").map(Some)
}
