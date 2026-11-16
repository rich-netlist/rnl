//! Count extra supply and ground heights within accepted group scopes.

use std::collections::{BTreeMap, BTreeSet};

use crate::coords::Placed;
use crate::rational::{int, Q};
use crate::structure::{Role, Structure};

/// The virtual symbols of a level (`virtual_weight`): the supply, ground and port symbols, the
/// usages drawn with a symbol of one pin that hangs on a net or a pin, whatever wire, stub, ROUTE
/// or group joins them. An instance of the netlist is never one, a source neither.
pub(crate) fn virtual_symbols(s: &Structure) -> BTreeSet<usize> {
    s.components().filter(|&k| matches!(s.objects[k].role, Role::Supply | Role::Ground | Role::Port)).collect()
}

/// Lines (heights, or rows and columns) beyond one, weighed: a line some firm item takes counts 1,
/// one only virtual symbols take counts `w`, less the weight of the first, the heaviest. Each line
/// by whether a firm item takes it. With `w` 1 it is the count of the lines less one.
pub(crate) fn weighed_beyond_one(lines: &BTreeMap<Q, bool>, w: &Q) -> Q {
    if lines.is_empty() {
        return int(0);
    }
    let weight = |firm: bool| if firm { int(1) } else { w.clone() };
    let total: Q = lines.values().map(|&f| weight(f)).sum();
    total - weight(lines.values().any(|&f| f))
}

/// The lines taken, weighed: those a firm item takes count 1, those only virtual symbols take count
/// `w`.
pub(crate) fn weighed_lines(lines: &BTreeMap<Q, bool>, w: &Q) -> Q {
    lines.values().map(|&f| if f { int(1) } else { w.clone() }).sum()
}

/// [`levels`] with the heights only virtual symbols take weighed by `w` ([`weighed_beyond_one`]).
pub(crate) fn levels_weighed(
    s: &Structure,
    placed: &Placed,
    within: Option<&BTreeSet<usize>>,
    virtuals: &BTreeSet<usize>,
    w: &Q,
) -> Q {
    let g = &placed.geometry;
    [Role::Ground, Role::Supply]
        .into_iter()
        .map(|role| {
            let mut ys: BTreeMap<Q, bool> = BTreeMap::new();
            for k in s.components().filter(|&k| within.is_none_or(|w| w.contains(&k)) && s.objects[k].role == role) {
                if let Some(p) = s.pin_at(&placed.origins, &g.orient, k, 0) {
                    *ys.entry(p[1].clone()).or_insert(false) |= !virtuals.contains(&k);
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
    use rnl_runtime::problem::{
        Filler, ObjectKind, PointRef, ProblemGroup, Reference, RoleFill, RouteIntent, UsageEntry,
    };

    /// Three supply symbols on a net and an element: one a ROUTE joins to the element, one a group
    /// holds with it, one hung from its pin. All three are virtual, the element is not.
    #[test]
    fn every_supply_ground_and_port_symbol_is_virtual_whatever_joins_it() {
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
        let mut s = crate::structure::analyse(&p, &resolved, &o, crate::options::CAPS, &mut Work::default());
        assert!(
            (1..4).all(|k| s.objects[k].role == Role::Supply),
            "{:?}",
            s.objects.iter().map(|o| o.role).collect::<Vec<_>>()
        );
        s.objects[1].hung_from = Some((0, 0));
        assert_eq!(virtual_symbols(&s), BTreeSet::from([1, 2, 3]));
    }

    #[test]
    fn heights_and_lines_only_virtual_symbols_take_count_by_the_weight() {
        let half = int(1) / int(2);
        let lines = |v: &[(i64, bool)]| v.iter().map(|&(y, f)| (int(y), f)).collect::<BTreeMap<Q, bool>>();
        // two heights firm symbols take and one only a virtual one takes
        let m = lines(&[(0, true), (5, true), (9, false)]);
        assert_eq!(weighed_beyond_one(&m, &int(1)), int(2));
        assert_eq!(weighed_beyond_one(&m, &half), int(1) + &half);
        assert_eq!(weighed_lines(&m, &int(1)), int(3));
        assert_eq!(weighed_lines(&m, &half), int(2) + &half);
        // virtual ones only: the first is the one
        let m = lines(&[(0, false), (5, false)]);
        assert_eq!(weighed_beyond_one(&m, &half), half);
        assert_eq!(weighed_beyond_one(&m, &int(0)), int(0));
        assert_eq!(weighed_beyond_one(&BTreeMap::new(), &half), int(0));
    }
}
