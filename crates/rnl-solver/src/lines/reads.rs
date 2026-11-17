//! Which objects a reference reads, and the fixture the tests of the line layout share.

use rnl_runtime::problem::PointRef;

/// Geometric scope includes every member of a centre, independently of connectivity.
pub(super) fn reads_reference(r: &PointRef, reads: &impl Fn(usize) -> bool) -> bool {
    match r {
        PointRef::Origin(k)
        | PointRef::Pin { object: k, .. }
        | PointRef::SymbolPin { object: k, .. }
        | PointRef::PinPosition { object: k, .. } => reads(*k),
        PointRef::Centre(members) => members.iter().all(|r| reads_reference(r, reads)),
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::coords::Placed;
    use crate::rational::int;
    use crate::resolve::tests::{fact, instance, problem_of};
    use crate::route::Routing;
    use crate::structure::Structure;
    use crate::{SolverOptions, StoppedClock, Work};
    use rnl_runtime::geom::Orient;
    use rnl_runtime::problem::{FactKind, LevelProblem, Reference, RouteIntent};
    use rnl_runtime::solution::{RoutedWire, WirePoint};

    fn pin(k: usize) -> PointRef {
        PointRef::SymbolPin { object: k, pin: 0, reference: Reference::Anonymous }
    }

    pub(in crate::lines) fn fixture() -> (LevelProblem, Structure, Placed, Routing) {
        let mut p = problem_of(vec![instance(), instance()], vec![]);
        p.version = "0.4".into();
        p.symbols[0].pins.truncate(1);
        p.symbols[0].pins[0].pos = (0., 0.);
        let route = |net: &str, pins| RouteIntent {
            net: net.into(),
            tree: true,
            mode: "auto".into(),
            style: vec![],
            pins,
            wires: vec![],
        };
        p.routes = vec![route("a", vec![pin(0), pin(1)]), route("b", vec![])];
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
        let mut geometry = s.witness.clone();
        geometry.origin = vec![[int(0), int(0)], [int(10), int(0)]];
        // Pick the orientation whose outward side points left.
        geometry.orient[0] = Orient::ALL.into_iter().find(|&o| s.exit(0, 0, o) == Some((1, 0))).unwrap();
        let placed = Placed {
            origins: s.origins(&p, &geometry),
            geometry,
            dropped: vec![],
            overlaps: vec![],
            optimised: true,
            undecided: false,
        };
        let routing = Routing {
            wires: vec![
                RoutedWire { route: 0, points: vec![WirePoint::Anchor(pin(0)), WirePoint::Anchor(pin(1))] },
                RoutedWire { route: 1, points: vec![WirePoint::Bend((-2., -4.)), WirePoint::Bend((-2., 4.))] },
            ],
            paths: vec![vec![[int(0), int(0)], [int(10), int(0)]], vec![[int(-2), int(-4)], [int(-2), int(4)]]],
            drawn: vec![0, 1],
            ..Default::default()
        };
        (p, s, placed, routing)
    }

    #[test]
    fn centre_scope_checks_all_nested_members_and_exact_facts_keep_all_orientations() {
        let nested = PointRef::Centre(vec![pin(0), PointRef::Centre(vec![PointRef::Origin(1)])]);
        assert!(!reads_reference(&nested, &|k| k == 0));
        let (mut p, _, placed, _) = fixture();
        let chosen = placed.geometry.orient[0];
        let other = Orient::ALL.into_iter().find(|o| *o != chosen).unwrap();
        p.facts = vec![fact(FactKind::Orient { object: 0, code: other })];
        p.orient_lists = vec![(0, vec![other, chosen])];
        let ori = |k| placed.geometry.orient.get(k).copied();
        assert_eq!(crate::holds::check_problem(&p, 0, &|_, _| None, &ori, &int(0), &int(0)), Some(true));
        p.orient_lists[0].1.pop();
        assert_eq!(crate::holds::check_problem(&p, 0, &|_, _| None, &ori, &int(0), &int(0)), Some(false));
    }
}
