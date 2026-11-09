//! Direction changes with an algebraic or literal-coordinate justification (14.3–14.5).
use super::edit::{record_edit, Edit};
use rnl_runtime::host::Record;
use rnl_runtime::syntax::{Call, Coord, Member, ObjExpr, Value};

fn number(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Num { value, .. }) => Some(*value),
        _ => None,
    }
}
fn coordinate(object: &ObjExpr, axis: &str) -> Option<f64> {
    let ObjExpr::Point(point) = object else { return None };
    match if axis == "x" { &point.x } else { &point.y } {
        Coord::Num(n) => Some(*n),
        _ => None,
    }
}
fn literal_coordinates(member: &Member, index: usize, axis: &str) -> Option<Vec<f64>> {
    let segment = &member.segments[index];
    if segment.piped {
        return None;
    }
    let mut objects = segment.args.iter().map(|a| &a.obj).collect::<Vec<_>>();
    if let [ObjExpr::Set(set)] = objects.as_slice() {
        let mut inner = Vec::new();
        for m in &set.members {
            if !m.binds.is_empty() || m.segments.len() != 1 || m.segments[0].call.is_some() {
                return None;
            }
            inner.extend(m.segments[0].args.iter().map(|a| &a.obj));
        }
        objects = inner;
    }
    objects.into_iter().map(|o| coordinate(o, axis)).collect()
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
        let segment = &member.segments[index];
        // Extent gaps of sets do not negate like the signed distance of two points.
        if segment.piped
            || segment.args.len() != 2
            || !segment.args.iter().all(|a| match &a.obj {
                ObjExpr::Point(_) => true,
                ObjExpr::Ref(r) => {
                    r.prefix == '#' && r.name.is_some() && r.sub.is_none() && r.places.is_none() && r.path.is_empty()
                }
                _ => false,
            })
        {
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
        let points = literal_coordinates(member, index, axis)?;
        if points.len() < 2 {
            return None;
        }
        let increasing = points.windows(2).all(|p| p[1] - p[0] >= 1.0);
        let decreasing = points.windows(2).all(|p| p[0] - p[1] >= 1.0);
        let sense = if call.kw == "DISTANCE" && points.len() == 2 {
            points[1] >= points[0]
        } else if increasing {
            true
        } else if decreasing {
            false
        } else {
            return None;
        };
        if call.kw == "DISTANCE"
            && ["min", "max", "exact"]
                .iter()
                .any(|k| matches!(params.get(k),Some(v) if !matches!(v,Value::Num{value,..} if *value>=0.0)))
        {
            return None;
        }
        direction = match (axis.as_str(), sense) {
            ("x", true) => "left-to-right",
            ("x", false) => "right-to-left",
            ("y", true) => "bottom-to-top",
            _ => "top-to-bottom",
        }
        .into();
    }
    if direction == *axis {
        return None;
    }
    Some(render(call, &direction, axis_key, interval))
}
fn render(call: &Call, direction: &str, axis_key: &str, interval: Option<Vec<(String, f64)>>) -> String {
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
