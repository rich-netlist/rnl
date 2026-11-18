use crate::{
    json::{Error, Node, Result},
    point::{orientation, point, point_value},
};
use rnl_runtime::{
    problem::{LevelProblem, PointRef, Reference},
    solution::{Branch, Placement, RoutedWire, Solution, WirePoint, WireRoute},
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

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
    validate_nets(&s, problem)?;
    Ok(s)
}

/// The nets a point of a wire is known to be on: a pin, on the nets [`LevelProblem::nets`] lists
/// it on; a point object, on the nets of the wires written through it. None for a point the
/// problem does not tie to a net, which any wire may take.
struct NetsOf<'a> {
    pins: BTreeMap<(usize, usize), BTreeSet<&'a str>>,
    points: BTreeMap<usize, BTreeSet<&'a str>>,
}

impl<'a> NetsOf<'a> {
    fn new(p: &'a LevelProblem) -> Self {
        let mut pins: BTreeMap<(usize, usize), BTreeSet<&str>> = BTreeMap::new();
        for net in &p.nets {
            for r in &net.pins {
                if let Some(key) = r.pin_key() {
                    pins.entry(key).or_default().insert(net.name.as_str());
                }
            }
        }
        let mut points: BTreeMap<usize, BTreeSet<&str>> = BTreeMap::new();
        for wire in &p.wires {
            if wire.nets.is_empty() {
                continue;
            }
            for r in &wire.anchors {
                if let PointRef::Origin(k) = r {
                    points.entry(*k).or_default().extend(wire.nets.iter().map(String::as_str));
                }
            }
        }
        NetsOf { pins, points }
    }

    fn of(&self, r: &PointRef) -> Option<&BTreeSet<&'a str>> {
        match r {
            PointRef::Origin(k) => self.points.get(k),
            PointRef::Centre(_) => None,
            _ => r.pin_key().and_then(|key| self.pins.get(&key)),
        }
    }
}

/// A point at which the wires that pass it are joined, by the identity it keeps however a wire
/// refers to it: a pin, an object by its origin (a named point, 13.1), a branch point of the
/// result. The coordinate point of a pin and the centre of a set join nothing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Joint {
    Pin(usize, usize),
    Origin(usize),
    Branch(usize),
}

impl Joint {
    fn of(point: &WirePoint) -> Option<Joint> {
        match point {
            WirePoint::Anchor(PointRef::Origin(k)) => Some(Joint::Origin(*k)),
            WirePoint::Anchor(r) => r.pin_key().map(|(object, pin)| Joint::Pin(object, pin)),
            WirePoint::Branch(b) => Some(Joint::Branch(*b)),
            WirePoint::Bend(_) => None,
        }
    }
}

/// Each wire stays on its net: a wire routed for a ROUTE, on the ROUTE's net; a wire written in
/// RNL that the result routes, on its own nets. Every pin or point a wire passes that the problem
/// ties to nets is on one of them, but for a point that a wire written of the ROUTE's tree, or the
/// wire written itself, passes as its author wrote it (a wire written may pass a pin of another
/// net, which the reading warns of). A point the problem ties to several nets, such as the pin of a
/// bundle's bus, which is on each net of its members, takes a wire of any of them. The wires of
/// the whole result that pass one point the problem ties to no net, or one branch point
/// ([`Joint`]), share a net: a point no wire was written through, which two wires of the result
/// pass, joins them. A result
/// that joined two nets would be written as one, under the name of one of them. A wire may pass
/// pins of another tree of its own net: the solver joins the trees of a net (`join_trees`), and
/// gives their wires the first tree's ROUTE.
fn validate_nets(s: &Solution, p: &LevelProblem) -> Result<()> {
    let nets_of = NetsOf::new(p);
    // the nets each joint the problem ties to no net can be on, narrowed by each wire of the
    // result that passes it
    let mut joints: BTreeMap<Joint, BTreeSet<String>> = BTreeMap::new();
    let mut check =
        |points: &[WirePoint], nets: BTreeSet<&str>, written: &[&PointRef], path: &str, what: &str| -> Result<()> {
            for (i, point) in points.iter().enumerate() {
                let Some(joint) = Joint::of(point) else { continue };
                let anchor = match point {
                    WirePoint::Anchor(r) => Some(r),
                    _ => None,
                };
                if anchor.is_some_and(|r| written.contains(&r)) {
                    // as its author wrote it: neither checked nor narrowed
                    continue;
                }
                // a point the problem ties to nets: on one of them, whatever other wires pass it
                let tied = anchor.and_then(|r| nets_of.of(r));
                let before: Option<BTreeSet<String>> = match tied {
                    Some(on) => Some(on.iter().map(|n| n.to_string()).collect()),
                    None => joints.get(&joint).cloned(),
                };
                let shared: BTreeSet<String> = match &before {
                    Some(before) => before.iter().filter(|n| nets.contains(n.as_str())).cloned().collect(),
                    None => nets.iter().map(|n| n.to_string()).collect(),
                };
                if shared.is_empty() {
                    let on = before.unwrap_or_default().iter().map(|n| format!("&{n}")).collect::<Vec<_>>().join(", ");
                    return Err(match point {
                        WirePoint::Branch(b) => Error {
                            path: format!("{path}[{i}].branch"),
                            message: format!("branch {b} joins {what} to a wire of another net, on {on}"),
                        },
                        _ => Error {
                            path: format!("{path}[{i}].point"),
                            message: format!("this point is on {on}, not on {what}"),
                        },
                    });
                }
                if tied.is_none() {
                    joints.insert(joint, shared);
                }
            }
            Ok(())
        };
    // the points of the wires written that a wire may pass as their author wrote them
    let anchors = |wires: &mut dyn Iterator<Item = usize>| -> Vec<&PointRef> {
        wires.filter_map(|w| p.wires.get(w)).flat_map(|w| &w.anchors).collect()
    };
    for (i, w) in s.wires.iter().enumerate() {
        let route = &p.routes[w.route];
        let net = route.net.as_str();
        let written = anchors(&mut route.wires.iter().copied());
        let what = format!("&{net} of its route");
        check(&w.points, BTreeSet::from([net]), &written, &format!("$.wires[{i}].points"), &what)?;
    }
    for (i, w) in s.wire_routes.iter().enumerate() {
        let wire = &p.wires[w.wire];
        let nets: BTreeSet<&str> = wire.nets.iter().map(String::as_str).collect();
        if nets.is_empty() {
            continue;
        }
        let what = format!("the nets of {}", wire.label);
        let written = anchors(&mut std::iter::once(w.wire));
        check(&w.points, nets, &written, &format!("$.wire_routes[{i}].points"), &what)?;
    }
    Ok(())
}
