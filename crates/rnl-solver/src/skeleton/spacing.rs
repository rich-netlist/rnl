//! Resolve each gap's spacing from the innermost accepted GROUP containing both features.

use super::read::{Key, Kind, Skeleton};
use crate::structure::Structure;
use rnl_runtime::problem::{LevelProblem, PointRef, Position};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Policy {
    global: [i64; 2],
    // Innermost first; feature membership uses full GROUP membership, not the element projection.
    groups: Vec<([Option<i64>; 2], BTreeSet<usize>)>,
}

impl Policy {
    pub(super) fn new(problem: &LevelProblem, s: &Structure, sk: &Skeleton, global: [i64; 2]) -> Option<Self> {
        let mut trees: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for path in &sk.paths {
            for &vertex in &path.vertices {
                let v = &sk.vertices[vertex];
                if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                    return None;
                }
                match v.key {
                    Key::Pin(k, _) | Key::PinPosition(k, _) | Key::Point(k) => {
                        trees.entry(path.tree).or_default().insert(k);
                    }
                    Key::Coordinate(w, j) => {
                        let mut pending = vec![&problem.wires[w].anchors[j]];
                        while let Some(reference) = pending.pop() {
                            if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                                return None;
                            }
                            match reference {
                                PointRef::Origin(k)
                                | PointRef::Pin { object: k, .. }
                                | PointRef::SymbolPin { object: k, .. }
                                | PointRef::PinPosition { object: k, .. } => {
                                    trees.entry(path.tree).or_default().insert(*k);
                                }
                                PointRef::Centre(members) => pending.extend(members),
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut order = s.selected_groups.clone();
        order.sort_by_key(|&g| (problem.groups[g].members.len(), std::cmp::Reverse(g)));
        let mut groups = Vec::new();
        for gi in order {
            if !crate::budget::permit(crate::budget::Step::Structure, problem.objects.len() + sk.features.len()) {
                return None;
            }
            let group = &problem.groups[gi];
            let mut members: BTreeSet<_> = group.members.iter().copied().collect();
            // Symbol labels and attached symbols belong with their owners. The closure handles
            // nested ownership without relying on labels or the ordering of objects.
            loop {
                let before = members.len();
                for (k, obj) in problem.objects.iter().enumerate() {
                    if !crate::budget::permit(crate::budget::Step::Structure, 1) {
                        return None;
                    }
                    let owner = match obj.position {
                        Position::InSymbol { owner, .. } => Some(owner),
                        _ => None,
                    }
                    .or_else(|| s.objects[k].stub.as_ref().map(|x| x.host.0))
                    .or_else(|| s.objects[k].hung_from.map(|x| x.0));
                    let anchored = s
                        .anchors
                        .get(&k)
                        .is_some_and(|a| !a.from.is_empty() && a.from.iter().all(|(o, _)| members.contains(o)));
                    if owner.is_some_and(|o| members.contains(&o)) || anchored {
                        members.insert(k);
                    }
                }
                if before == members.len() {
                    break;
                }
            }
            let mut features = BTreeSet::new();
            for (i, f) in sk.features.iter().enumerate() {
                let size = match f.kind {
                    Kind::Segment(g) => trees.get(&sk.segs[g].tree).map_or(1, BTreeSet::len),
                    Kind::Group(g) => sk.groups[g].members.len(),
                    _ => 1,
                };
                if !crate::budget::permit(crate::budget::Step::Structure, size) {
                    return None;
                }
                let inside = match f.kind {
                    Kind::Symbol(k) | Kind::Text(k) | Kind::Pin(k, _) => members.contains(&k),
                    Kind::Segment(g) => {
                        trees.get(&sk.segs[g].tree).is_some_and(|ks| !ks.is_empty() && ks.is_subset(&members))
                    }
                    // A group's own boundary takes its parent's spacing.
                    Kind::Group(g) => sk.groups[g].group != Some(gi) && sk.groups[g].members.is_subset(&members),
                };
                if inside {
                    features.insert(i);
                }
            }
            let value = |v: Option<f64>| {
                v.filter(|v| v.is_finite() && v.fract() == 0. && (-1. ..=20.).contains(v)).map(|v| v as i64)
            };
            groups.push(([value(group.spacing_x), value(group.spacing_y)], features));
        }
        Some(Self { global, groups })
    }

    pub(super) fn active(&self, axis: usize) -> bool {
        self.global[axis] != 0 || self.groups.iter().any(|(values, _)| values[axis].is_some_and(|n| n != 0))
    }

    pub(super) fn description(&self, axis: usize) -> String {
        if self.groups.iter().any(|(values, _)| values[axis].is_some()) {
            format!("level {}, with innermost GROUP overrides", self.global[axis])
        } else {
            self.global[axis].to_string()
        }
    }

    pub(super) fn extra(&self, a: usize, b: usize, axis: usize) -> i64 {
        self.groups
            .iter()
            .find(|(_, members)| members.contains(&a) && members.contains(&b))
            .and_then(|(values, _)| values[axis])
            .unwrap_or(self.global[axis])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{fact, instance, problem_of};
    use crate::route::Routing;
    use crate::{SolverOptions, StoppedClock, Work};
    use rnl_runtime::problem::{Axis, FactKind, ProblemGroup, ProblemWire, Reference};
    use rnl_runtime::solution::{RoutedWire, WirePoint};

    fn problem() -> LevelProblem {
        let mut p = problem_of(
            vec![instance(); 3],
            (0..3)
                .map(|k| fact(FactKind::Place { at: PointRef::Origin(k), axis: Axis::X, value: 20. * k as f64 }))
                .collect(),
        );
        p.version = "0.4".into();
        p.groups.push(ProblemGroup {
            identity: 0,
            first: true,
            direct: true,
            spacing_x: Some(5.),
            spacing_y: None,
            label: "group".into(),
            members: vec![0, 1],
        });
        p
    }

    fn structure(p: &LevelProblem) -> Structure {
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(p, &options, &StoppedClock);
        crate::structure::analyse(p, &resolved, &options, crate::options::CAPS, &mut Work::default())
    }

    fn check(p: &LevelProblem, s: &Structure, routing: &Routing, expected: &[i64]) {
        let sk = super::super::read::read(p, s, &s.witness, routing, &|_| true, false, false, None, false).unwrap();
        let policy = Policy::new(p, s, &sk, [0, 0]).unwrap();
        let mut checked = BTreeSet::new();
        for (i, f) in sk.features.iter().enumerate() {
            if let Kind::Segment(g) = f.kind {
                checked.insert(sk.segs[g].tree);
                assert_eq!(policy.extra(i, i, 0), expected[sk.segs[g].tree], "segment {g}");
            }
        }
        assert_eq!(checked, (0..expected.len()).collect(), "every routed tree must have a nonzero segment feature");
    }

    #[test]
    fn centre_anchors_include_every_member_in_the_spacing_scope() {
        let mut p = problem();
        for anchors in [
            vec![PointRef::Origin(0), PointRef::Centre(vec![PointRef::Origin(1), PointRef::Origin(2)])],
            vec![PointRef::Centre(vec![PointRef::Origin(0)]), PointRef::Centre(vec![PointRef::Origin(1)])],
        ] {
            p.wires.push(ProblemWire {
                label: "wire".into(),
                reference: Reference::Anonymous,
                nets: vec![],
                bus: false,
                anchors,
                mode: "straight".into(),
                default_auto: false,
                generated: false,
            });
        }
        let s = structure(&p);
        check(&p, &s, &Routing::default(), &[0, 5]);
    }

    #[test]
    fn independent_routes_sharing_a_pin_keep_their_scopes_in_either_direction() {
        let mut p = problem();
        for end in [2, 1] {
            p.routes.push(rnl_runtime::problem::RouteIntent {
                tree: true,
                net: "shared".into(),
                mode: "manhattan".into(),
                style: vec![],
                pins: [0, end]
                    .map(|object| PointRef::SymbolPin { object, pin: 0, reference: Reference::Anonymous })
                    .to_vec(),
                wires: vec![],
            });
        }
        let s = structure(&p);
        for reverse in [false, true] {
            let mut routing = Routing::default();
            for (tree, end) in [2, 1].into_iter().enumerate() {
                let mut anchors: Vec<_> = [0, end]
                    .map(|object| PointRef::SymbolPin { object, pin: 0, reference: Reference::Anonymous })
                    .to_vec();
                if reverse {
                    anchors.reverse();
                }
                let mut path: Vec<_> = anchors
                    .iter()
                    .map(|pnt| [0, 1].map(|axis| crate::resolve::point_value(&p, &s.witness, pnt, axis, 0).unwrap()))
                    .collect();
                let middle = [0, 1].map(|axis| (&path[0][axis] + &path[1][axis]) / crate::rational::int(2));
                path.insert(1, middle.clone());
                routing.paths.push(path);
                let mut points: Vec<_> = anchors.into_iter().map(WirePoint::Anchor).collect();
                points.insert(1, WirePoint::Bend(middle.each_ref().map(crate::rational::to_f64).into()));
                routing.wires.push(RoutedWire { route: tree, points });
                routing.drawn.push(tree);
            }
            check(&p, &s, &routing, &[0, 5]);
            let sk =
                super::super::read::read(&p, &s, &s.witness, &routing, &|_| true, true, false, None, false).unwrap();
            let pin = sk.features.iter().find(|f| f.kind == Kind::Pin(0, 0)).unwrap();
            let mut checked = BTreeSet::new();
            for segment in &sk.features {
                if let Kind::Segment(g) = segment.kind {
                    if !sk.at_pin(g, (0, 0)) {
                        checked.insert(sk.segs[g].tree);
                        assert_eq!(super::super::pass::spacing(&sk, segment, pin), None);
                    }
                }
            }
            assert_eq!(checked, [0, 1].into(), "check a distant segment of each tree sharing the pin");
            assert_eq!(sk.tree_objects[&0], [0, 2].into());
            assert_eq!(sk.tree_objects[&1], [0, 1].into());
        }
    }
}
