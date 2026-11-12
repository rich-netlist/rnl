//! Count junctions too close to pins using exact routed geometry.

use std::collections::{BTreeMap, BTreeSet};

use num_traits::Zero;
use rnl_runtime::problem::{LevelProblem, PointRef};

use crate::budget::{permit, Step};
use crate::coords::Placed;
use crate::rational::{int, Q};
use crate::route::Routing;
use crate::structure::Structure;
pub(crate) fn near_pins(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    within: Option<&BTreeSet<usize>>,
) -> usize {
    if !permit(Step::Structure, problem.objects.len() + routing.wires.len()) {
        return 0;
    }
    let g = &placed.geometry;
    let reads = |k: usize| within.is_none_or(|w| w.contains(&k));
    type Pin = ([Q; 2], Option<(i64, i64)>);
    type Segment = ([Q; 2], [Q; 2]);
    // Each resolved route is an independent tree, even when two routes name the same net.
    // Written wires belong to each explicit route that includes them, or to their own tree.
    let mut pins: BTreeMap<usize, Vec<Pin>> = BTreeMap::new();
    let mut segs: BTreeMap<usize, Vec<Segment>> = BTreeMap::new();
    let mut add_pins = |tree, references: &[PointRef]| {
        for r in references {
            let (PointRef::Pin { object: k, pin, .. } | PointRef::SymbolPin { object: k, pin, .. }) = r else {
                continue;
            };
            if !reads(*k) {
                continue;
            }
            let Some(at) = s.pin_at(&placed.origins, &g.orient, *k, *pin) else { continue };
            let lead = s.exit(*k, *pin, g.orient[*k]).map(|e| (-e.0, -e.1));
            let own = pins.entry(tree).or_default();
            if !own.contains(&(at.clone(), lead)) {
                own.push((at, lead));
            }
        }
    };
    for (tree, route) in problem.routes.iter().enumerate() {
        add_pins(tree, &route.pins);
    }
    for (wire, path) in routing.wires.iter().zip(&routing.paths) {
        segs.entry(wire.route)
            .or_default()
            .extend(path.windows(2).filter(|p| p[0] != p[1]).map(|p| (p[0].clone(), p[1].clone())));
    }
    for (w, wire) in problem.wires.iter().enumerate() {
        let Ok(Some(path)) = crate::route::written_path(problem, g, w) else { continue };
        let mut trees: Vec<usize> =
            problem.routes.iter().enumerate().filter(|(_, r)| r.wires.contains(&w)).map(|(i, _)| i).collect();
        if trees.is_empty() {
            trees.push(problem.routes.len() + w);
        }
        for tree in trees {
            add_pins(tree, &wire.anchors);
            segs.entry(tree)
                .or_default()
                .extend(path.windows(2).filter(|p| p[0] != p[1]).map(|p| (p[0].clone(), p[1].clone())));
        }
    }
    let sign = |v: Q| -> i64 {
        if v.is_zero() {
            0
        } else if v > Q::zero() {
            1
        } else {
            -1
        }
    };
    let inside = |p: &[Q; 2], a: &[Q; 2], b: &[Q; 2]| {
        let between = |v: &Q, x: &Q, y: &Q| (x < v && v < y) || (y < v && v < x);
        (a[0] == b[0] && p[0] == a[0] && between(&p[1], &a[1], &b[1]))
            || (a[1] == b[1] && p[1] == a[1] && between(&p[0], &a[0], &b[0]))
    };
    let mut near = 0;
    for (tree, ss) in &segs {
        let own: &[Pin] = pins.get(tree).map_or(&[], |v| v.as_slice());
        let mut places: BTreeSet<[Q; 2]> = ss.iter().flat_map(|(a, b)| [a.clone(), b.clone()]).collect();
        places.extend(own.iter().map(|(p, _)| p.clone()));
        for at in &places {
            if !permit(Step::Structure, ss.len() + own.len()) {
                return 0;
            }
            let mut ways: BTreeSet<(i64, i64)> = BTreeSet::new();
            for (a, b) in ss {
                if a == at || inside(at, a, b) {
                    ways.insert((sign(&b[0] - &a[0]), sign(&b[1] - &a[1])));
                }
                if b == at || inside(at, a, b) {
                    ways.insert((sign(&a[0] - &b[0]), sign(&a[1] - &b[1])));
                }
            }
            if ways.is_empty() {
                continue;
            }
            ways.extend(own.iter().filter(|(p, _)| p == at).filter_map(|(_, l)| *l));
            if ways.len() < 3 {
                continue;
            }
            let close = own.iter().any(|(p, _)| {
                let d = (&p[0] - &at[0]).abs() + (&p[1] - &at[1]).abs();
                d > Q::zero() && d < int(2)
            });
            near += usize::from(close);
        }
    }
    near
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{instance, problem_of};
    use crate::{SolverOptions, StoppedClock, Work};
    use rnl_runtime::problem::{Reference, RouteIntent};
    use rnl_runtime::solution::{RoutedWire, WirePoint};

    #[test]
    fn same_net_trees_and_position_only_anchors_do_not_invent_nearby_junctions() {
        let mut p = problem_of(vec![instance()], vec![]);
        p.version = "0.4".into();
        p.symbols[0].pins.truncate(1);
        p.symbols[0].pins[0].pos = (0., 0.);
        let pin = PointRef::SymbolPin { object: 0, pin: 0, reference: Reference::Anonymous };
        let route = |pins| RouteIntent {
            net: "shared".into(),
            tree: true,
            mode: "auto".into(),
            style: vec![],
            pins,
            wires: vec![],
        };
        p.routes = vec![route(vec![pin]), route(vec![])];
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
        let mut geometry = s.witness.clone();
        geometry.origin[0] = [int(-1), int(0)];
        let placed = Placed {
            origins: s.origins(&p, &geometry),
            geometry,
            dropped: vec![],
            overlaps: vec![],
            optimised: true,
            undecided: false,
        };
        let mut routing = Routing {
            wires: vec![
                RoutedWire { route: 0, points: vec![WirePoint::Bend((-3., 0.)), WirePoint::Bend((3., 0.))] },
                RoutedWire { route: 1, points: vec![WirePoint::Bend((0., 0.)), WirePoint::Bend((0., 3.))] },
            ],
            paths: vec![vec![[int(-3), int(0)], [int(3), int(0)]], vec![[int(0), int(0)], [int(0), int(3)]]],
            drawn: vec![0, 1],
            ..Default::default()
        };
        assert_eq!(near_pins(&p, &s, &placed, &routing, None), 0);
        routing.wires[1].route = 0;
        routing.drawn[1] = 0;
        assert_eq!(near_pins(&p, &s, &placed, &routing, None), 1);
        p.routes[0].pins = vec![PointRef::PinPosition { object: 0, pin: 0, reference: Reference::Anonymous }];
        assert_eq!(near_pins(&p, &s, &placed, &routing, None), 0);
    }
}
