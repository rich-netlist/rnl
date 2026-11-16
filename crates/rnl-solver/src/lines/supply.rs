//! Count extra supply and ground heights within accepted group scopes.

use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::problem::{LevelProblem, PointRef};

use crate::coords::Placed;
use crate::rational::{int, Q};
use crate::structure::{Role, Structure};

/// The loose virtual symbols of a level (`virtual_weight`): the supply, ground and port symbols
/// that no stub hangs from an element's pin, no ROUTE or wire reaches, and no group holds with an
/// element.
pub(crate) fn loose(problem: &LevelProblem, s: &Structure) -> BTreeSet<usize> {
    let object = |r: &PointRef| match r {
        PointRef::Origin(k) => Some(*k),
        _ => r.pin_key().map(|(k, _)| k),
    };
    let mut joined: BTreeSet<usize> = BTreeSet::new();
    for route in &problem.routes {
        joined.extend(route.pins.iter().filter_map(object));
    }
    for wire in &problem.wires {
        joined.extend(wire.anchors.iter().filter_map(object));
    }
    for g in &problem.groups {
        if g.members.iter().any(|&m| s.objects.get(m).is_some_and(|o| o.role == Role::Element)) {
            joined.extend(g.members.iter().copied());
        }
    }
    s.components()
        .filter(|&k| {
            let o = &s.objects[k];
            matches!(o.role, Role::Supply | Role::Ground | Role::Port) && o.stub.is_none() && !joined.contains(&k)
        })
        .collect()
}

/// Lines (heights, or rows and columns) beyond one, weighed: a line some firm item takes counts 1,
/// one only loose items take counts `w`, less the weight of the first, the heaviest. Each line by
/// whether a firm item takes it. With `w` 1 it is the count of the lines less one.
pub(crate) fn weighed_beyond_one(lines: &BTreeMap<Q, bool>, w: &Q) -> Q {
    if lines.is_empty() {
        return int(0);
    }
    let weight = |firm: bool| if firm { int(1) } else { w.clone() };
    let total: Q = lines.values().map(|&f| weight(f)).sum();
    total - weight(lines.values().any(|&f| f))
}

/// The lines taken, weighed: those a firm item takes count 1, those only loose items take count
/// `w`.
pub(crate) fn weighed_lines(lines: &BTreeMap<Q, bool>, w: &Q) -> Q {
    lines.values().map(|&f| if f { int(1) } else { w.clone() }).sum()
}

/// [`levels`] with the heights only loose symbols take weighed by `w` ([`weighed_beyond_one`]).
pub(crate) fn levels_weighed(
    s: &Structure,
    placed: &Placed,
    within: Option<&BTreeSet<usize>>,
    loose: &BTreeSet<usize>,
    w: &Q,
) -> Q {
    let g = &placed.geometry;
    [Role::Ground, Role::Supply]
        .into_iter()
        .map(|role| {
            let mut ys: BTreeMap<Q, bool> = BTreeMap::new();
            for k in s.components().filter(|&k| within.is_none_or(|w| w.contains(&k)) && s.objects[k].role == role) {
                if let Some(p) = s.pin_at(&placed.origins, &g.orient, k, 0) {
                    *ys.entry(p[1].clone()).or_insert(false) |= !loose.contains(&k);
                }
            }
            weighed_beyond_one(&ys, w)
        })
        .sum()
}
pub(crate) fn levels(s: &Structure, placed: &Placed, within: Option<&BTreeSet<usize>>) -> usize {
    let g = &placed.geometry;
    [Role::Ground, Role::Supply]
        .into_iter()
        .map(|role| {
            let ys: BTreeSet<Q> = s
                .components()
                .filter(|&k| within.is_none_or(|w| w.contains(&k)) && s.objects[k].role == role)
                .filter_map(|k| s.pin_at(&placed.origins, &g.orient, k, 0).map(|p| p[1].clone()))
                .collect();
            ys.len().saturating_sub(1)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{instance, problem_of};
    use crate::{SolverOptions, Work};
    use rnl_runtime::problem::{Filler, ObjectKind, ProblemGroup, Reference, RoleFill, RouteIntent, UsageEntry};

    /// Three supply symbols on a net and an element: one a ROUTE reaches, one a group holds with
    /// the element, one nothing joins, which alone is loose.
    #[test]
    fn a_loose_symbol_is_one_no_stub_wire_or_group_joins_to_an_element() {
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
        for k in 1..4 {
            let o = &mut p.objects[k];
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
        assert!(
            (1..4).all(|k| s.objects[k].role == Role::Supply),
            "{:?}",
            s.objects.iter().map(|o| o.role).collect::<Vec<_>>()
        );
        assert_eq!(loose(&p, &s), BTreeSet::from([1]));
        // a group of supply symbols alone joins none of them to an element
        p.groups[0].members = vec![1, 3];
        assert_eq!(loose(&p, &s), BTreeSet::from([1, 3]));
    }

    #[test]
    fn heights_and_lines_only_loose_symbols_take_count_by_the_weight() {
        let half = int(1) / int(2);
        let lines = |v: &[(i64, bool)]| v.iter().map(|&(y, f)| (int(y), f)).collect::<BTreeMap<Q, bool>>();
        // two heights firm symbols take and one only a loose one takes
        let m = lines(&[(0, true), (5, true), (9, false)]);
        assert_eq!(weighed_beyond_one(&m, &int(1)), int(2));
        assert_eq!(weighed_beyond_one(&m, &half), int(1) + &half);
        assert_eq!(weighed_lines(&m, &int(1)), int(3));
        assert_eq!(weighed_lines(&m, &half), int(2) + &half);
        // loose ones only: the first is the one
        let m = lines(&[(0, false), (5, false)]);
        assert_eq!(weighed_beyond_one(&m, &half), half);
        assert_eq!(weighed_beyond_one(&m, &int(0)), int(0));
        assert_eq!(weighed_beyond_one(&BTreeMap::new(), &half), int(0));
    }
}
