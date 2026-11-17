//! The virtual symbols of a level and the rows and columns only they take (`virtual_weight`).

use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::problem::{LevelProblem, ObjectKind};

use crate::rational::{int, Q};
use crate::structure::Structure;

/// The virtual symbols of a level (`virtual_weight`): the components that are not of the
/// netlist, the objects of usages (12.1), such as a symbol a usage of one role makes or a usage
/// with a symbol and a position of its own, whatever wire, stub, ROUTE or group joins them. An
/// instance of the netlist or a bundle is never one, a source neither.
pub(crate) fn virtual_symbols(problem: &LevelProblem, s: &Structure) -> BTreeSet<usize> {
    s.components().filter(|&k| problem.objects.get(k).is_some_and(|o| o.kind == ObjectKind::Usage)).collect()
}

/// The lines taken, weighed: those a symbol other than a virtual one takes count 1, those only
/// virtual symbols take count `w`.
pub(crate) fn weighed_lines(lines: &BTreeMap<Q, bool>, w: &Q) -> Q {
    lines.values().map(|&f| if f { int(1) } else { w.clone() }).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{instance, problem_of};
    use crate::{SolverOptions, Work};
    use rnl_runtime::problem::{
        Filler, ObjectKind, PointRef, ProblemGroup, Reference, RoleFill, RouteIntent, UsageEntry,
    };

    /// Three supply symbols on a net and an element: one a ROUTE joins to the element, one a group
    /// holds with it, one on its own. All three are virtual, the element is not; a usage
    /// that is an element of the layout, with a symbol and a position of its own, is virtual too.
    #[test]
    fn every_usage_is_virtual_whatever_joins_it_and_an_instance_is_not() {
        let mut p = problem_of((0..4).map(|_| instance()).collect(), vec![]);
        p.version = "0.4".into();
        let mut supply = p.symbols[0].clone();
        supply.name = "$SUPPLY_SYMBOL".into();
        supply.bbox = [-1.2, 0.0, 1.2, 2.2];
        supply.pins.truncate(1);
        supply.pins[0].pos = (0.0, 0.0);
        supply.pins[0].dir = Some(270);
        p.symbols.push(supply);
        p.usages = (1..4)
            .map(|k| UsageEntry {
                definition: "$SUPPLY".into(),
                label: format!("%v{k}"),
                reference: Reference::Anonymous,
                generated: false,
                rule_made: false,
                roles: vec![RoleFill { role: "p".into(), members: vec![Filler::Net("vdd".into())], written: None }],
                params: vec![],
                priority: None,
                parent: None,
                object: Some(k),
                written_numbers: 0,
                source: None,
            })
            .collect();
        for (k, o) in p.objects.iter_mut().enumerate().take(4).skip(1) {
            (o.kind, o.symbol, o.usage) = (ObjectKind::Usage, Some(1), Some(k - 1));
        }
        let pin = |object| PointRef::SymbolPin { object, pin: 0, reference: Reference::Anonymous };
        p.routes = vec![RouteIntent {
            net: "vdd".into(),
            tree: true,
            mode: "auto".into(),
            style: vec![],
            pins: vec![pin(0), pin(2)],
            wires: vec![],
        }];
        p.groups = vec![ProblemGroup {
            identity: 0,
            first: true,
            direct: true,
            label: "%g".into(),
            members: vec![0, 3],
            spacing_x: None,
            spacing_y: None,
        }];
        let o = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(&p, &o, &crate::StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &o, crate::options::CAPS, &mut Work::default());
        // the solver tells no supply: each is an element of the layout as any other
        assert!(
            (1..4).all(|k| s.objects[k].role == crate::structure::Role::Element),
            "{:?}",
            s.objects.iter().map(|o| o.role).collect::<Vec<_>>()
        );
        assert_eq!(virtual_symbols(&p, &s), BTreeSet::from([1, 2, 3]));
        // the instance made a usage of its own: an element of the layout, and virtual
        p.objects[0].kind = ObjectKind::Usage;
        let resolved = crate::resolve::resolve(&p, &o, &crate::StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &o, crate::options::CAPS, &mut Work::default());
        assert_eq!(s.objects[0].role, crate::structure::Role::Element);
        assert_eq!(virtual_symbols(&p, &s), BTreeSet::from([0, 1, 2, 3]));
        p.objects[0].kind = ObjectKind::Bundle;
        assert_eq!(virtual_symbols(&p, &s), BTreeSet::from([1, 2, 3]));
    }

    #[test]
    fn rows_and_columns_only_virtual_symbols_take_count_by_the_weight() {
        let half = int(1) / int(2);
        let lines = |v: &[(i64, bool)]| v.iter().map(|&(y, f)| (int(y), f)).collect::<BTreeMap<Q, bool>>();
        // two lines a symbol other than a virtual one takes and one only a virtual one takes
        let m = lines(&[(0, true), (5, true), (9, false)]);
        assert_eq!(weighed_lines(&m, &int(1)), int(3));
        assert_eq!(weighed_lines(&m, &half), int(2) + &half);
        assert_eq!(weighed_lines(&m, &int(0)), int(2));
        assert_eq!(weighed_lines(&BTreeMap::new(), &half), int(0));
    }
}
