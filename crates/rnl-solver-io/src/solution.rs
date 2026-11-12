use crate::{
    json::{Error, Node, Result},
    point::{orientation, point, point_value},
};
use rnl_runtime::{
    problem::{LevelProblem, PointRef, Reference},
    solution::{Branch, Placement, RoutedWire, Solution, WirePoint, WireRoute},
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn wire_point_value(p: &WirePoint) -> Value {
    match p {
        WirePoint::Anchor(p) => json!({"kind":"anchor", "point":point_value(p)}),
        WirePoint::Branch(branch) => json!({"kind":"branch", "branch":branch}),
        WirePoint::Bend((x, y)) => json!({"kind":"bend", "at":[x,y]}),
    }
}

fn wire_point(n: Node<'_>) -> Result<WirePoint> {
    Ok(match n.field("kind")?.string()? {
        "anchor" => WirePoint::Anchor(point(n.field("point")?, 0)?),
        "branch" => WirePoint::Branch(n.field("branch")?.index()?),
        "bend" => WirePoint::Bend(n.field("at")?.point()?),
        _ => return Err(n.field("kind")?.error("unknown wire point kind")),
    })
}

/// Encode every writable decision without replacing exact references by display names.
/// Nonfinite coordinates are rejected rather than silently becoming JSON null.
pub fn encode_solution(s: &Solution) -> Result<Value> {
    for (i, p) in s.placements.iter().enumerate() {
        for (axis, value) in [("x", p.x), ("y", p.y)] {
            if value.is_some_and(|v| !v.is_finite()) {
                return Err(Error {
                    path: format!("$.placements[{i}].{axis}"),
                    message: "expected a finite coordinate".into(),
                });
            }
        }
    }
    let value = json!({
        "placements":s.placements.iter().map(|p| json!({"object":p.object,"x":p.x,"y":p.y,"orient":p.orient.map(|o|o.as_str())})).collect::<Vec<_>>(),
        "branches":s.branches.iter().map(|b|json!({"at":[b.at.0,b.at.1],"junction":b.junction})).collect::<Vec<_>>(),
        "junctions":s.junctions.iter().map(point_value).collect::<Vec<_>>(),
        "wires":s.wires.iter().map(|w|json!({"route":w.route,"points":w.points.iter().map(wire_point_value).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "wire_routes":s.wire_routes.iter().map(|w|json!({"wire":w.wire,"points":w.points.iter().map(wire_point_value).collect::<Vec<_>>()})).collect::<Vec<_>>(),
    });
    // Check the serialized numeric fields too: JSON null cannot stand for a bend or branch.
    decode_raw(Node::root(&value))?;
    Ok(value)
}

fn decode_raw(n: Node<'_>) -> Result<Solution> {
    Ok(Solution {
        placements: n.field("placements")?.list(|n| {
            Ok(Placement {
                object: n.field("object")?.index()?,
                x: n.field("x")?.nullable(|n| n.number())?,
                y: n.field("y")?.nullable(|n| n.number())?,
                orient: n.field("orient")?.nullable(orientation)?,
            })
        })?,
        branches: n
            .field("branches")?
            .list(|n| Ok(Branch { at: n.field("at")?.point()?, junction: n.field("junction")?.boolean()? }))?,
        junctions: n.field("junctions")?.list(|n| point(n, 0))?,
        wires: n.field("wires")?.list(|n| {
            Ok(RoutedWire { route: n.field("route")?.index()?, points: n.field("points")?.list(wire_point)? })
        })?,
        wire_routes: n.field("wire_routes")?.list(|n| {
            Ok(WireRoute { wire: n.field("wire")?.index()?, points: n.field("points")?.list(wire_point)? })
        })?,
    })
}

fn index(value: usize, len: usize, path: String, kind: &str) -> Result<()> {
    if value < len {
        Ok(())
    } else {
        Err(Error { path, message: format!("{kind} index {value} is out of range (length {len})") })
    }
}

pub(crate) fn validate_reference(r: &Reference, p: &LevelProblem, path: &str) -> Result<()> {
    match r {
        Reference::Generated { usage, .. } | Reference::GeneratedNumber { usage, .. } => {
            index(*usage, p.usages.len(), format!("{path}.usage"), "usage")
        }
        _ => Ok(()),
    }
}

pub(crate) fn validate_point(r: &PointRef, p: &LevelProblem, path: &str) -> Result<()> {
    match r {
        PointRef::Origin(k) => index(*k, p.objects.len(), format!("{path}.object"), "object"),
        PointRef::Centre(members) => {
            if members.is_empty() {
                return Err(Error { path: path.into(), message: "a centre requires at least one point".into() });
            }
            for (i, m) in members.iter().enumerate() {
                validate_point(m, p, &format!("{path}.members[{i}]"))?;
            }
            Ok(())
        }
        PointRef::Pin { object, pin, .. }
        | PointRef::SymbolPin { object, pin, .. }
        | PointRef::PinPosition { object, pin, .. } => {
            index(*object, p.objects.len(), format!("{path}.object"), "object")?;
            let pins = p.objects[*object].symbol.and_then(|s| p.symbols.get(s)).map_or(0, |s| s.pins.len());
            index(*pin, pins, format!("{path}.pin"), "pin")?;
            if let PointRef::SymbolPin { reference, .. } | PointRef::PinPosition { reference, .. } = r {
                validate_reference(reference, p, &format!("{path}.reference"))?;
            }
            Ok(())
        }
    }
}

fn validate_wire(
    points: &[WirePoint],
    p: &LevelProblem,
    bindings: &crate::bindings::Bindings,
    branches: usize,
    path: &str,
) -> Result<()> {
    if points.len() < 2 {
        return Err(Error { path: path.into(), message: "a wire requires at least two points".into() });
    }
    for (i, point) in points.iter().enumerate() {
        let at = format!("{path}[{i}]");
        match point {
            WirePoint::Anchor(r) => {
                validate_point(r, p, &format!("{at}.point"))?;
                bindings.validate(r, &format!("{at}.point"))?;
            }
            WirePoint::Branch(b) => index(*b, branches, format!("{at}.branch"), "branch")?,
            WirePoint::Bend(_) => {}
        }
    }
    Ok(())
}

/// Decode and validate a result against the original problem before the writer sees it.
/// Model indices refer to this exact problem, never to a replacement supplied by the solver.
pub fn decode_solution(value: &Value, problem: &LevelProblem) -> Result<Solution> {
    let s = decode_raw(Node::root(value))?;
    let bindings = crate::bindings::Bindings::new(problem);
    let mut placed = BTreeSet::new();
    for (i, p) in s.placements.iter().enumerate() {
        let path = format!("$.placements[{i}].object");
        index(p.object, problem.objects.len(), path.clone(), "object")?;
        if !placed.insert(p.object) {
            return Err(Error { path, message: "duplicate placement for one object".into() });
        }
        if p.orient.is_some() && problem.objects[p.object].orient.is_none() {
            return Err(Error {
                path: format!("$.placements[{i}].orient"),
                message: "this object has no orientation".into(),
            });
        }
    }
    for (i, p) in s.junctions.iter().enumerate() {
        validate_point(p, problem, &format!("$.junctions[{i}]"))?;
        bindings.validate(p, &format!("$.junctions[{i}]"))?;
    }
    for (i, w) in s.wires.iter().enumerate() {
        index(w.route, problem.routes.len(), format!("$.wires[{i}].route"), "route")?;
        validate_wire(&w.points, problem, &bindings, s.branches.len(), &format!("$.wires[{i}].points"))?;
    }
    let mut rewrites = BTreeSet::new();
    for (i, w) in s.wire_routes.iter().enumerate() {
        let path = format!("$.wire_routes[{i}].wire");
        index(w.wire, problem.wires.len(), path.clone(), "wire")?;
        if !rewrites.insert(w.wire) {
            return Err(Error { path, message: "duplicate rewrite for one wire".into() });
        }
        validate_wire(&w.points, problem, &bindings, s.branches.len(), &format!("$.wire_routes[{i}].points"))?;
    }
    Ok(s)
}
