//! Exact output geometry together with the identities that make wires connected (13.1).
//!
//! Equal coordinates never make two vertices the same point. In particular, distinct physical
//! pins at one coordinate still need their zero length joining wire. Authored wires retain their
//! order and their own point identities; generated branch identities are shared between outputs.

use std::collections::BTreeSet;

use num_traits::{Signed, Zero};
use rnl_runtime::problem::{LevelProblem, PointRef};

use crate::rational::{int, Q};
use crate::resolve::{point_value, Geometry};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum OutputWireId {
    Authored(usize),
    Generated(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GeometrySource {
    AuthoredFixed,
    AuthoredAuto,
    Generated,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum GeometryPointId {
    Origin(usize),
    Pin {
        object: usize,
        pin: usize,
    },
    Branch(usize),
    Bend {
        wire: OutputWireId,
        index: usize,
    },
    /// Retain less usual references, such as centres, without inventing coordinate identities.
    Reference(PointRef),
}

impl GeometryPointId {
    pub(crate) fn reference(reference: &PointRef) -> Self {
        match reference {
            PointRef::Origin(object) => Self::Origin(*object),
            PointRef::Pin { object, pin, .. } => Self::Pin { object: *object, pin: *pin },
            _ => Self::Reference(reference.clone()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GeometryVertex {
    pub at: [Q; 2],
    pub point: GeometryPointId,
    pub physical_pin: Option<(usize, usize)>,
}

impl GeometryVertex {
    pub(crate) fn anchor(at: [Q; 2], reference: &PointRef) -> Self {
        let physical_pin = match reference {
            PointRef::Pin { object, pin, .. } => Some((*object, *pin)),
            _ => None,
        };
        Self { at, point: GeometryPointId::reference(reference), physical_pin }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GeometryWire {
    pub id: OutputWireId,
    pub vertices: Vec<GeometryVertex>,
    pub nets: BTreeSet<String>,
    pub source: GeometrySource,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RoutingGeometry {
    pub wires: Vec<GeometryWire>,
}

/// The author's points and the bends inserted by the fixed mode. Step1 treats auto as straight,
/// as the existing renderer does. This function is also the sole source of routing obstacles:
/// scoring and routing therefore cannot disagree about the shape of an authored wire.
pub(crate) fn authored_wire(problem: &LevelProblem, g: &Geometry, wire: usize) -> Option<GeometryWire> {
    let authored = problem.wires.get(wire)?;
    let id = OutputWireId::Authored(wire);
    let anchors: Vec<GeometryVertex> = authored
        .anchors
        .iter()
        .map(|reference| {
            Some(GeometryVertex::anchor(
                [point_value(problem, g, reference, 0, 0)?, point_value(problem, g, reference, 1, 0)?],
                reference,
            ))
        })
        .collect::<Option<_>>()?;
    let mut vertices: Vec<GeometryVertex> = anchors.first().cloned().into_iter().collect();
    let two = int(2);
    for pair in anchors.windows(2) {
        let (a, b) = (&pair[0].at, &pair[1].at);
        let (dx, dy) = (&b[0] - &a[0], &b[1] - &a[1]);
        let mut bends = Vec::new();
        if !(dx.is_zero() || dy.is_zero()) {
            match authored.mode.as_str() {
                "horizontal-vertical" => bends.push([b[0].clone(), a[1].clone()]),
                "vertical-horizontal" => bends.push([a[0].clone(), b[1].clone()]),
                "manhattan" if dx.abs() >= dy.abs() => {
                    let x = &a[0] + &dx / &two;
                    bends.extend([[x.clone(), a[1].clone()], [x, b[1].clone()]]);
                }
                "manhattan" => {
                    let y = &a[1] + &dy / &two;
                    bends.extend([[a[0].clone(), y.clone()], [b[0].clone(), y]]);
                }
                "manhattan45" if dx.abs() >= dy.abs() => {
                    let run = (dx.abs() - dy.abs()) / &two * dx.signum();
                    bends.extend([[&a[0] + &run, a[1].clone()], [&b[0] - &run, b[1].clone()]]);
                }
                "manhattan45" => {
                    let run = (dy.abs() - dx.abs()) / &two * dy.signum();
                    bends.extend([[a[0].clone(), &a[1] + &run], [b[0].clone(), &b[1] - &run]]);
                }
                _ => {}
            }
        }
        for at in bends {
            vertices.push(GeometryVertex {
                at,
                point: GeometryPointId::Bend { wire: id, index: vertices.len() },
                physical_pin: None,
            });
        }
        vertices.push(pair[1].clone());
    }
    Some(GeometryWire {
        id,
        vertices,
        nets: authored.nets.iter().cloned().collect(),
        source: if authored.mode == "auto" { GeometrySource::AuthoredAuto } else { GeometrySource::AuthoredFixed },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::frac;
    use crate::resolve::tests::{point, problem_of};
    use rnl_runtime::problem::{ProblemWire, Reference};

    #[test]
    fn coincident_objects_still_have_distinct_identities() {
        let at = [int(0), int(0)];
        let a = GeometryVertex::anchor(at.clone(), &PointRef::Origin(0));
        let b = GeometryVertex::anchor(at.clone(), &PointRef::Origin(1));
        assert_eq!(a.at, b.at);
        assert_ne!(a.point, b.point);
        let pin = |terminal: &str| PointRef::Pin { object: 2, pin: 0, terminal: terminal.into() };
        let bus_a = GeometryVertex::anchor(at.clone(), &pin("a"));
        let bus_b = GeometryVertex::anchor(at, &pin("b"));
        assert_eq!(bus_a.point, bus_b.point);
        assert_eq!(bus_a.physical_pin, Some((2, 0)));
    }

    #[test]
    fn authored_points_keep_order_and_exact_half_grid_bends() {
        let mut problem = problem_of(vec![point(), point(), point()], Vec::new());
        problem.wires.push(ProblemWire {
            label: "%wire".into(),
            reference: Reference::Name("%wire".into()),
            nets: vec!["a".into(), "b".into()],
            bus: true,
            anchors: vec![PointRef::Origin(0), PointRef::Origin(1), PointRef::Origin(2), PointRef::Origin(0)],
            mode: "manhattan".into(),
            generated: false,
        });
        let mut geometry = Geometry::new(3);
        geometry.origin = vec![[int(0), int(0)], [int(3), int(1)], [int(3), int(1)]];
        let wire = authored_wire(&problem, &geometry, 0).unwrap();
        assert_eq!(wire.id, OutputWireId::Authored(0));
        assert_eq!(wire.source, GeometrySource::AuthoredFixed);
        assert_eq!(wire.nets, ["a".into(), "b".into()].into_iter().collect());
        assert_eq!(wire.vertices[1].at, [frac(3, 2), int(0)]);
        let anchors: Vec<_> = wire
            .vertices
            .iter()
            .filter_map(|vertex| match vertex.point {
                GeometryPointId::Origin(object) => Some(object),
                _ => None,
            })
            .collect();
        assert_eq!(anchors, vec![0, 1, 2, 0]);
        assert_eq!(wire.vertices[3].at, wire.vertices[4].at);
        assert_ne!(wire.vertices[3].point, wire.vertices[4].point);
        problem.wires[0].mode = "auto".into();
        let auto = authored_wire(&problem, &geometry, 0).unwrap();
        assert_eq!(auto.source, GeometrySource::AuthoredAuto);
        assert_eq!(auto.vertices.len(), 4, "Step1 auto retains the author's straight polyline");
    }
}
