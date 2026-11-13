//! Materialize routed geometry and explicit junctions for writeback, preserving tree identity.

use std::collections::{BTreeMap, BTreeSet};

use num_traits::Zero;
use rnl_runtime::problem::{LevelProblem, PointRef};
use rnl_runtime::solution::{RoutedWire, WirePoint, WireRoute};

use crate::rational::{to_f64, Q};
use crate::resolve::{point_value, Geometry};
use crate::route::{written_path, Routing};

const NEAR: f64 = 1e-9;
type AnchoredPath = Vec<(WirePoint, [Q; 2])>;

/// Coordinate compaction and port cleanup can change which core default applies.
/// Refresh only complete, unmerged two-anchor trees whose default is still clear
/// of every other wire. Authored and branched topology is never replaced here.
pub(super) fn defaults(
    problem: &LevelProblem,
    s: &crate::structure::Structure,
    placed: &crate::coords::Placed,
    routing: &mut Routing,
    within: Option<&BTreeSet<usize>>,
    final_output: bool,
) -> bool {
    let _span = crate::profile::span(if final_output { "output" } else { "lines" }, "auto-defaults");
    if problem.version != "0.4"
        || routing.written_path_error.is_some()
        || routing.annotation_error.is_some()
        || !final_output && crate::budget::stopped()
    {
        return false;
    }
    let all = (0..problem.objects.len()).collect();
    let objects = within.unwrap_or(&all);
    let (mut layout, _, _, refs) =
        match super::settle::layout_of(problem, s, &placed.geometry, &placed.origins, objects, within.is_none()) {
            Ok(layout) => layout,
            Err(super::settle::LayoutError::Written(error)) => {
                routing.written_path_error = Some(error.wire);
                return false;
            }
            Err(super::settle::LayoutError::Annotation(error)) => {
                routing.annotation_error = Some(error);
                return false;
            }
        };
    let written = layout.written.len();
    let mut changed = false;
    for (i, route) in problem.routes.iter().enumerate() {
        if !final_output && crate::budget::stopped() {
            break;
        }
        if !super::settle::complete_auto_route(route, &refs[i]) {
            continue;
        }
        let mut wires = routing.wires.iter().enumerate().filter(|(_, w)| w.route == i);
        let Some((w, wire)) = wires.next() else { continue };
        if wires.next().is_some() || routing.left.iter().any(|left| left.route == i) {
            continue;
        }
        let (Some(WirePoint::Anchor(first)), Some(WirePoint::Anchor(last))) = (wire.points.first(), wire.points.last())
        else {
            continue;
        };
        let reverse = if first == &refs[i][0] && last == &refs[i][1] {
            false
        } else if first == &refs[i][1] && last == &refs[i][0] {
            true
        } else {
            continue;
        };
        layout.written.truncate(written);
        layout.written.extend(routing.paths.iter().enumerate().filter(|(j, _)| *j != w).map(|(_, p)| p.clone()));
        let path = if final_output {
            crate::route::default_geometry(&layout, i)
        } else {
            crate::route::default_candidate(&layout, i)
        };
        let Some(mut path) = path else { continue };
        if reverse {
            path.reverse();
        }
        if routing.paths.get(w) == Some(&path) {
            continue;
        }
        let mut points = vec![WirePoint::Anchor(first.clone())];
        points.extend(path[1..path.len() - 1].iter().map(|p| WirePoint::Bend((to_f64(&p[0]), to_f64(&p[1])))));
        points.push(WirePoint::Anchor(last.clone()));
        routing.wires[w].points = points;
        routing.paths[w] = path;
        changed = true;
    }
    changed
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum AnchorKey {
    Pin(usize, usize),
    Position(usize, usize),
    Object(usize),
}

fn anchor_key(point: &WirePoint) -> Option<AnchorKey> {
    match point {
        WirePoint::Anchor(PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. }) => {
            Some(AnchorKey::Pin(*object, *pin))
        }
        WirePoint::Anchor(PointRef::PinPosition { object, pin, .. }) => Some(AnchorKey::Position(*object, *pin)),
        WirePoint::Anchor(PointRef::Origin(k)) => Some(AnchorKey::Object(*k)),
        _ => None,
    }
}

pub(crate) fn branch_at_points(problem: &LevelProblem, g: &Geometry, routing: Routing) -> Routing {
    let _span = crate::profile::span("output", "branch-at-points");
    let at = |k: usize| {
        Some((
            to_f64(&point_value(problem, g, &PointRef::Origin(k), 0, 0)?),
            to_f64(&point_value(problem, g, &PointRef::Origin(k), 1, 0)?),
        ))
    };
    let mut same: BTreeMap<usize, usize> = BTreeMap::new();
    for w in &routing.wires {
        let Some(route) = problem.routes.get(w.route) else { continue };
        for p in &w.points {
            let WirePoint::Branch(i) = *p else { continue };
            let Some(b) = routing.branches.get(i).filter(|_| !same.contains_key(&i)) else { continue };
            let point =
                route.wires.iter().filter_map(|&x| problem.wires.get(x)).flat_map(|x| &x.anchors).find_map(|r| {
                    let PointRef::Origin(k) = *r else { return None };
                    at(k).filter(|q| (q.0 - b.at.0).abs() < NEAR && (q.1 - b.at.1).abs() < NEAR).map(|_| k)
                });
            if let Some(k) = point {
                same.insert(i, k);
            }
        }
    }
    if same.is_empty() {
        return routing;
    }
    let Routing { wires, branches, paths, drawn, .. } = routing.clone();
    let mut index = vec![None; branches.len()];
    let mut kept = Vec::new();
    for (i, b) in branches.into_iter().enumerate() {
        if !same.contains_key(&i) {
            index[i] = Some(kept.len());
            kept.push(b);
        }
    }
    let (mut out_wires, mut out_paths, mut out_drawn) = (Vec::new(), Vec::new(), Vec::new());
    for (i, (mut w, mut path)) in wires.into_iter().zip(paths).enumerate() {
        for p in &mut w.points {
            if let WirePoint::Branch(b) = *p {
                *p = match same.get(&b) {
                    Some(&k) => WirePoint::Anchor(PointRef::Origin(k)),
                    None => WirePoint::Branch(index[b].expect("a branch point kept")),
                };
            }
        }
        let mut j = 1;
        while j < w.points.len() {
            if w.points[j] == w.points[j - 1] {
                w.points.remove(j);
                if j < path.len() {
                    path.remove(j);
                }
            } else {
                j += 1;
            }
        }
        if w.points.len() >= 2 {
            out_wires.push(w);
            out_paths.push(path);
            out_drawn.extend(drawn.get(i).copied());
        }
    }
    Routing { wires: out_wires, branches: kept, paths: out_paths, drawn: out_drawn, ..routing }
}

/// Attach the original references to the core's whole-wire default path.
/// Segment slots preserve even coincident authored waypoints; no route is chosen here.
pub(crate) fn auto_route(
    problem: &LevelProblem,
    g: &Geometry,
    w: usize,
) -> Result<Option<AnchoredPath>, crate::route::WrittenPathError> {
    let Some(wire) = problem.wires.get(w) else { return Ok(None) };
    let Some(segments) = crate::route::written_auto_segments(problem, g, w)? else { return Ok(None) };
    let mut out = Vec::new();
    for (i, segment) in segments.iter().enumerate() {
        if i == 0 {
            out.push((WirePoint::Anchor(wire.anchors[0].clone()), segment[0].clone()));
        }
        for point in segment.iter().skip(1).take(segment.len().saturating_sub(2)) {
            out.push((WirePoint::Bend((to_f64(&point[0]), to_f64(&point[1]))), point.clone()));
        }
        out.push((WirePoint::Anchor(wire.anchors[i + 1].clone()), segment.last().unwrap().clone()));
    }
    Ok(Some(out))
}

fn written_points(
    problem: &LevelProblem,
    g: &Geometry,
    w: usize,
) -> Result<Option<AnchoredPath>, crate::route::WrittenPathError> {
    let Some(wire) = problem.wires.get(w) else { return Ok(None) };
    let Some(path) = written_path(problem, g, w)? else { return Ok(None) };
    let mut anchors = wire.anchors.iter().peekable();
    let mut out = Vec::new();
    for v in path {
        let next =
            anchors.peek().and_then(|r| Some([point_value(problem, g, r, 0, 0)?, point_value(problem, g, r, 1, 0)?]));
        if next.as_ref() == Some(&v) {
            out.push((WirePoint::Anchor(anchors.next().expect("a point").clone()), v));
        } else {
            out.push((WirePoint::Bend((to_f64(&v[0]), to_f64(&v[1]))), v));
        }
    }
    Ok(Some(out))
}

fn inside(p: &[Q; 2], a: &[Q; 2], b: &[Q; 2]) -> bool {
    let between = |v: &Q, x: &Q, y: &Q| (x < v && v < y) || (y < v && v < x);
    (a[0] == b[0] && p[0] == a[0] && between(&p[1], &a[1], &b[1]))
        || (a[1] == b[1] && p[1] == a[1] && between(&p[0], &a[0], &b[0]))
}

fn within(p: &[Q; 2], a: &[Q; 2], b: &[Q; 2]) -> bool {
    let (u, v) = ([&b[0] - &a[0], &b[1] - &a[1]], [&p[0] - &a[0], &p[1] - &a[1]]);
    let cross = &u[0] * &v[1] - &u[1] * &v[0];
    let dot = &u[0] * &v[0] + &u[1] * &v[1];
    let length = &u[0] * &u[0] + &u[1] * &u[1];
    cross.is_zero() && dot > Q::zero() && dot < length
}

pub(crate) fn finish(
    problem: &LevelProblem,
    g: &Geometry,
    mut routing: Routing,
) -> (Routing, Vec<WireRoute>, Vec<PointRef>) {
    let _span = crate::profile::span("output", "materialize");
    let tree_of = |w: &RoutedWire| w.route;
    loop {
        let mut cut: Option<(usize, usize, WirePoint, [Q; 2])> = None;
        'find: for (i, w) in routing.wires.iter().enumerate() {
            let path = &routing.paths[i];
            for (end, at) in [(w.points.first(), path.first()), (w.points.last(), path.last())] {
                let (Some(end), Some(at)) = (end, at) else { continue };
                for (j, v) in routing.wires.iter().enumerate() {
                    if j == i || tree_of(v) != tree_of(w) {
                        continue;
                    }
                    if let Some(s) = routing.paths[j].windows(2).position(|s| inside(at, &s[0], &s[1])) {
                        cut = Some((j, s + 1, end.clone(), at.clone()));
                        break 'find;
                    }
                }
            }
        }
        let Some((j, s, end, at)) = cut else { break };
        let (w, path) = (routing.wires[j].clone(), routing.paths[j].clone());
        let first = RoutedWire { route: w.route, points: [&w.points[..s], std::slice::from_ref(&end)].concat() };
        let second = RoutedWire { route: w.route, points: [&[end][..], &w.points[s..]].concat() };
        routing.wires[j] = first;
        routing.paths[j] = [&path[..s], std::slice::from_ref(&at)].concat();
        routing.wires.insert(j + 1, second);
        routing.paths.insert(j + 1, [&[at][..], &path[s..]].concat());
        if let Some(&d) = routing.drawn.get(j) {
            routing.drawn.insert(j + 1, d);
        }
    }
    type WrittenPath = (usize, Vec<(WirePoint, [Q; 2])>, bool);
    let mut written: Vec<WrittenPath> = Vec::new();
    for (w, wire) in problem.wires.iter().enumerate() {
        let auto = wire.mode == "auto" && wire.default_auto;
        let points = if auto { auto_route(problem, g, w) } else { written_points(problem, g, w) };
        let mut points = match points {
            Ok(Some(points)) => points,
            Ok(None) => continue,
            Err(error) => {
                routing.written_path_error = Some(error.wire);
                return (routing, Vec::new(), Vec::new());
            }
        };
        let mut rewrite = auto && points.iter().any(|p| matches!(p.0, WirePoint::Bend(_)));
        if !wire.bus && problem.routes.iter().any(|r| r.wires.contains(&w)) {
            for (v, path) in routing.wires.iter().zip(&routing.paths) {
                if problem.routes.get(v.route).is_none_or(|r| !r.wires.contains(&w)) {
                    continue;
                }
                for (end, at) in [(v.points.first(), path.first()), (v.points.last(), path.last())] {
                    let (Some(end @ WirePoint::Branch(_)), Some(at)) = (end, at) else { continue };
                    if let Some(s) = points.windows(2).position(|s| inside(at, &s[0].1, &s[1].1)) {
                        points.insert(s + 1, (end.clone(), at.clone()));
                        rewrite = true;
                    } else if let Some(bend) =
                        points.iter_mut().find(|p| matches!(p.0, WirePoint::Bend(_)) && p.1 == *at)
                    {
                        bend.0 = end.clone();
                        rewrite = true;
                    }
                }
            }
        }
        written.push((w, points, rewrite));
    }
    let routes: Vec<WireRoute> = written
        .iter()
        .filter(|(_, _, rewrite)| *rewrite)
        .map(|(w, points, _)| WireRoute { wire: *w, points: points.iter().map(|p| p.0.clone()).collect() })
        .collect();
    type Segment = ([Q; 2], [Q; 2]);
    type Tree = (usize, usize);
    let mut segs: BTreeMap<Tree, Vec<Segment>> = BTreeMap::new();
    let mut at_place: BTreeMap<(Tree, [Q; 2]), Vec<WirePoint>> = BTreeMap::new();
    let mut count = |tree: Tree, points: &[(WirePoint, [Q; 2])], places: bool| {
        for s in points.windows(2).filter(|s| s[0].1 != s[1].1) {
            segs.entry(tree).or_default().push((s[0].1.clone(), s[1].1.clone()));
        }
        for p in points.iter().filter(|_| places) {
            at_place.entry((tree, p.1.clone())).or_default().push(p.0.clone());
        }
    };
    for (v, path) in routing.wires.iter().zip(&routing.paths) {
        let points: Vec<(WirePoint, [Q; 2])> = if path.len() == v.points.len() {
            v.points.iter().cloned().zip(path.iter().cloned()).collect()
        } else {
            path.iter().map(|p| (WirePoint::Bend((to_f64(&p[0]), to_f64(&p[1]))), p.clone())).collect()
        };
        count((tree_of(v), 0), &points, true);
    }
    for (w, points, _) in &written {
        let wire = &problem.wires[*w];
        let trees: Vec<usize> =
            problem.routes.iter().enumerate().filter(|(_, r)| r.wires.contains(w)).map(|(r, _)| r).collect();
        if trees.is_empty() {
            for n in 0..wire.nets.len().max(1) {
                count((problem.routes.len() + w, n), points, n == 0 || !wire.bus);
            }
        } else {
            for tree in trees {
                let places = !wire.bus || wire.nets.first().is_some_and(|net| problem.routes[tree].net == *net);
                count((tree, 0), points, places);
            }
        }
    }
    for (tree, route) in problem.routes.iter().enumerate() {
        if !segs.contains_key(&(tree, 0)) {
            continue;
        }
        for r in &route.pins {
            let (Some(x), Some(y)) = (point_value(problem, g, r, 0, 0), point_value(problem, g, r, 1, 0)) else {
                continue;
            };
            at_place.entry(((tree, 0), [x, y])).or_default().push(WirePoint::Anchor(r.clone()));
        }
    }
    let lead = |q: &WirePoint| -> Option<(i64, i64)> {
        let WirePoint::Anchor(PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. }) = q else {
            return None;
        };
        let sym = problem.objects.get(*object)?.symbol?;
        let d = problem.symbols.get(sym)?.pins.get(*pin)?.dir?;
        let (x, y) = match d % 360 {
            0 => (1, 0),
            90 => (0, 1),
            180 => (-1, 0),
            270 => (0, -1),
            _ => return None,
        };
        let (a, b, c, e) = g.orient.get(*object).copied().unwrap_or(rnl_runtime::geom::Orient::R0).matrix();
        Some((-(i64::from(a) * x + i64::from(b) * y), -(i64::from(c) * x + i64::from(e) * y)))
    };
    let sign = |v: Q| -> i64 {
        if v.is_zero() {
            0
        } else if v > Q::zero() {
            1
        } else {
            -1
        }
    };
    let mut junctions = Vec::new();
    type SharedAnchor = (WirePoint, BTreeSet<(i64, i64)>, BTreeSet<Tree>);
    let mut shared: BTreeMap<AnchorKey, SharedAnchor> = BTreeMap::new();
    let mut covered = BTreeSet::new();
    for ((net, at), there) in &at_place {
        let mut ways: BTreeSet<(i64, i64)> = BTreeSet::new();
        for (a, b) in segs.get(net).into_iter().flatten() {
            if a == at || within(at, a, b) {
                ways.insert((sign(&b[0] - &a[0]), sign(&b[1] - &a[1])));
            }
            if b == at || within(at, a, b) {
                ways.insert((sign(&a[0] - &b[0]), sign(&a[1] - &b[1])));
            }
        }
        if ways.is_empty() {
            continue;
        }
        ways.extend(there.iter().filter_map(lead));
        // Independent trees may share a real anchor. Combine directions only at that
        // identity; coordinate equality elsewhere never joins their drawings.
        for point in there {
            if let Some(key) = anchor_key(point) {
                let entry = shared.entry(key).or_insert_with(|| (point.clone(), BTreeSet::new(), BTreeSet::new()));
                entry.1.extend(ways.iter().copied());
                entry.2.insert(*net);
            }
        }
        if ways.len() < 3 {
            continue;
        }
        let branches: Vec<usize> =
            there.iter().filter_map(|q| if let WirePoint::Branch(b) = q { Some(*b) } else { None }).collect();
        if branches.iter().any(|&b| routing.branches.get(b).is_some_and(|x| x.junction)) {
            covered.extend(there.iter().filter_map(anchor_key));
            continue;
        }
        let point = there.iter().find(|q| matches!(q, WirePoint::Anchor(PointRef::Origin(_))));
        let pin =
            there.iter().find(|q| matches!(q, WirePoint::Anchor(PointRef::Pin { .. } | PointRef::SymbolPin { .. })));
        junctions.push(match (point, pin, branches.first()) {
            (Some(q), _, _) | (None, Some(q), _) => q.clone(),
            (None, None, Some(&b)) => WirePoint::Branch(b),
            (None, None, None) => WirePoint::Bend((to_f64(&at[0]), to_f64(&at[1]))),
        });
        covered.extend(there.iter().filter_map(anchor_key));
    }
    junctions.extend(
        shared
            .into_iter()
            .filter(|(key, (_, ways, trees))| trees.len() > 1 && ways.len() >= 3 && !covered.contains(key))
            .map(|(_, (point, _, _))| point),
    );
    let mut anchors = Vec::new();
    for junction in junctions {
        match junction {
            WirePoint::Anchor(reference) => {
                let identity = rnl_runtime::problem::junction_point_identity(&reference);
                if identity.is_some()
                    && problem
                        .existing_junctions
                        .iter()
                        .any(|p| rnl_runtime::problem::junction_point_identity(p) == identity)
                {
                    continue;
                }
                if !anchors.contains(&reference) {
                    anchors.push(reference);
                }
            }
            WirePoint::Branch(i) => {
                if let Some(branch) = routing.branches.get_mut(i) {
                    branch.junction = true;
                }
            }
            WirePoint::Bend(at) => routing.branches.push(rnl_runtime::solution::Branch { at, junction: true }),
        }
    }
    (routing, routes, anchors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::int;
    use crate::resolve::tests::{instance, point, problem_of};
    use rnl_runtime::geom::Orient;
    use rnl_runtime::problem::{ProblemWire, Reference, RouteIntent};
    use rnl_runtime::solution::Branch;

    fn route(wires: Vec<usize>) -> RouteIntent {
        RouteIntent { net: "shared".into(), tree: true, mode: "auto".into(), style: vec![], pins: vec![], wires }
    }
    fn geometry(points: &[(i64, i64)]) -> Geometry {
        Geometry {
            origin: points.iter().map(|&(x, y)| [int(x), int(y)]).collect(),
            orient: vec![Orient::R0; points.len()],
        }
    }
    fn wire(anchors: Vec<PointRef>, mode: &str) -> ProblemWire {
        ProblemWire {
            label: "%wire".into(),
            reference: Reference::Anonymous,
            nets: vec!["shared".into()],
            bus: false,
            anchors,
            mode: mode.into(),
            default_auto: mode == "auto",
            generated: false,
        }
    }

    #[test]
    fn a_partial_multi_anchor_tree_keeps_its_path_until_the_tree_is_complete() {
        let mut p = problem_of(vec![instance(); 3], vec![]);
        p.version = "0.4".into();
        p.symbols[0].bbox = [0.0; 4];
        p.symbols[0].pins.truncate(1);
        p.symbols[0].pins[0].pos = (0.0, 0.0);
        p.symbols[0].pins[0].dir = None;
        let pin = |object| PointRef::Pin { object, pin: 0, terminal: "g".into() };
        p.routes = vec![RouteIntent { pins: (0..3).map(pin).collect(), ..route(vec![]) }];
        let options = crate::SolverOptions::without_time_limit();
        let resolved = crate::resolve(&p, &options);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut crate::Work::default());
        let g = geometry(&[(0, 0), (4, 3), (100, 100)]);
        let placed = crate::coords::Placed {
            origins: s.origins(&p, &g),
            geometry: g,
            dropped: vec![],
            overlaps: vec![],
            optimised: true,
            undecided: false,
        };
        let mut routing = Routing {
            wires: vec![RoutedWire {
                route: 0,
                points: vec![WirePoint::Anchor(pin(0)), WirePoint::Bend((0.0, 3.0)), WirePoint::Anchor(pin(1))],
            }],
            paths: vec![vec![[int(0), int(0)], [int(0), int(3)], [int(4), int(3)]]],
            drawn: vec![0],
            ..Default::default()
        };
        let before = routing.clone();
        let inside = BTreeSet::from([0, 1]);
        assert!(!defaults(&p, &s, &placed, &mut routing, Some(&inside), false));
        assert_eq!(routing, before);

        // A complete two-anchor tree inside the same group still follows the core rule.
        p.routes[0].pins.pop();
        p.routes[0].pins.push(PointRef::SymbolPin { object: 0, pin: 0, reference: Reference::Anonymous });
        assert!(defaults(&p, &s, &placed, &mut routing, Some(&inside), false));
        assert_eq!(routing.paths[0], vec![[int(0), int(0)], [int(4), int(0)], [int(4), int(3)]]);
    }

    #[test]
    fn independent_routes_on_one_net_are_not_split_or_dotted_at_coincident_places() {
        let mut p = problem_of(vec![point(); 4], vec![]);
        p.version = "0.4".into();
        p.routes = vec![route(vec![]), route(vec![])];
        let g = geometry(&[(-2, 0), (2, 0), (0, 0), (0, 2)]);
        let anchor = |i| WirePoint::Anchor(PointRef::Origin(i));
        let routing = Routing {
            wires: vec![
                RoutedWire { route: 0, points: vec![anchor(0), anchor(1)] },
                RoutedWire { route: 1, points: vec![anchor(2), anchor(3)] },
            ],
            paths: vec![vec![g.origin[0].clone(), g.origin[1].clone()], vec![g.origin[2].clone(), g.origin[3].clone()]],
            drawn: vec![0, 1],
            ..Default::default()
        };
        let (out, written, dots) = finish(&p, &g, routing.clone());
        assert_eq!(out, routing);
        assert!(written.is_empty() && dots.is_empty());
        let mut joined = routing;
        joined.wires[1].route = 0;
        joined.drawn[1] = 0;
        let (out, _, dots) = finish(&p, &g, joined);
        assert_eq!(out.wires.len(), 3);
        assert_eq!(dots, vec![PointRef::Origin(2)]);
    }

    #[test]
    fn final_default_refresh_uses_final_coordinates_and_keeps_obstructed_routes() {
        let (mut p, _, mut placed, mut routing) = super::super::port::tests::fixture();
        p.symbols[0].bbox = [0.; 4];
        p.symbols[0].pins[0].dir = None;
        let options = crate::SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(&p, &options, &crate::StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut crate::Work::default());
        placed.geometry.orient.fill(rnl_runtime::geom::Orient::R0);
        placed.geometry.origin[1] = [int(4), int(3)];
        placed.origins = s.origins(&p, &placed.geometry);
        routing.wires.truncate(1);
        routing.paths = vec![vec![[int(0), int(0)], [int(0), int(5)], [int(4), int(5)], [int(4), int(3)]]];
        routing.drawn.truncate(1);
        let saved = routing.clone();
        let (changed, stop) = crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(0), None, || {
            defaults(&p, &s, &placed, &mut routing, None, true)
        });
        assert!(changed);
        assert_eq!(stop.work, 0, "final materialization does not restart search");
        assert_eq!(routing.paths[0], vec![[int(0), int(0)], [int(4), int(0)], [int(4), int(3)]]);
        assert_eq!(routing.wires[0].points.len(), routing.paths[0].len());
        assert!(!defaults(&p, &s, &placed, &mut routing, None, true), "idempotent");
        routing = saved;
        routing
            .wires
            .push(RoutedWire { route: 1, points: vec![WirePoint::Bend((2., -1.)), WirePoint::Bend((2., 1.))] });
        routing.paths.push(vec![[int(2), int(-1)], [int(2), int(1)]]);
        routing.drawn.push(1);
        let obstructed = routing.clone();
        assert!(!defaults(&p, &s, &placed, &mut routing, None, true));
        assert_eq!(routing, obstructed);
    }

    #[test]
    fn auto_routes_respect_electrical_pin_directions_but_not_position_aliases() {
        let mut p = problem_of(vec![instance(), point()], vec![]);
        p.version = "0.4".into();
        p.symbols[0].pins[0].pos = (0., 0.);
        p.symbols[0].pins[0].dir = Some(90);
        let g = geometry(&[(0, 0), (4, 2)]);
        let pin = PointRef::SymbolPin { object: 0, pin: 0, reference: Reference::Anonymous };
        p.wires = vec![wire(vec![pin, PointRef::Origin(1)], "auto")];
        let path = auto_route(&p, &g, 0).unwrap().unwrap();
        assert_eq!(path[1].1, [int(0), int(2)]);
        p.wires[0].anchors[0] = PointRef::PinPosition { object: 0, pin: 0, reference: Reference::Anonymous };
        let path = auto_route(&p, &g, 0).unwrap().unwrap();
        assert_eq!(path[1].1, [int(4), int(0)]);
    }

    #[test]
    fn splitting_a_written_wire_keeps_its_mode_geometry_and_explicit_tree_membership() {
        let mut p = problem_of(vec![point(); 2], vec![]);
        p.version = "0.4".into();
        p.wires = vec![wire(vec![PointRef::Origin(0), PointRef::Origin(1)], "horizontal-vertical")];
        p.routes = vec![route(vec![0]), route(vec![])];
        let g = geometry(&[(0, 0), (2, 2)]);
        let mut routing = Routing {
            branches: vec![Branch { at: (2., 1.), junction: true }],
            wires: vec![RoutedWire { route: 1, points: vec![WirePoint::Branch(0), WirePoint::Bend((4., 1.))] }],
            paths: vec![vec![[int(2), int(1)], [int(4), int(1)]]],
            drawn: vec![1],
            ..Default::default()
        };
        let (_, written, _) = finish(&p, &g, routing.clone());
        assert!(written.is_empty(), "same net alone does not put the written wire in a route");
        routing.wires[0].route = 0;
        routing.drawn[0] = 0;
        let (_, written, _) = finish(&p, &g, routing);
        assert_eq!(written.len(), 1);
        assert_eq!(
            written[0].points,
            vec![
                WirePoint::Anchor(PointRef::Origin(0)),
                WirePoint::Bend((2., 0.)),
                WirePoint::Branch(0),
                WirePoint::Anchor(PointRef::Origin(1))
            ]
        );
    }

    #[test]
    fn shared_pins_combine_directions_only_at_their_identity() {
        let mut p = problem_of(vec![instance(), point(), point()], vec![]);
        p.version = "0.4".into();
        p.symbols[0].pins[0].pos = (0., 0.);
        p.symbols[0].pins[0].dir = Some(180);
        let pin = PointRef::SymbolPin { object: 0, pin: 0, reference: Reference::Anonymous };
        p.routes = vec![route(vec![]), route(vec![])];
        p.routes[0].pins = vec![pin.clone()];
        p.routes[1].pins = vec![pin.clone()];
        let g = geometry(&[(0, 0), (-2, 0), (0, 2)]);
        let mut routing = Routing {
            wires: (0..2)
                .map(|i| RoutedWire {
                    route: i,
                    points: vec![WirePoint::Anchor(pin.clone()), WirePoint::Anchor(PointRef::Origin(i + 1))],
                })
                .collect(),
            paths: vec![vec![g.origin[0].clone(), g.origin[1].clone()], vec![g.origin[0].clone(), g.origin[2].clone()]],
            drawn: vec![0, 1],
            ..Default::default()
        };
        let (out, _, dots) = finish(&p, &g, routing.clone());
        assert_eq!(out, routing);
        assert_eq!(dots, vec![pin]);
        let position = PointRef::PinPosition { object: 0, pin: 0, reference: Reference::Anonymous };
        p.routes[1].pins.clear();
        routing.wires[1].points[0] = WirePoint::Anchor(position);
        let (_, _, dots) = finish(&p, &g, routing);
        assert!(dots.is_empty());
    }
    #[test]
    fn core_default_matrix_keeps_authored_references_under_every_orientation() {
        use rnl_runtime::geom::{
            auto::{auto_wire, Direction},
            Orient,
        };
        type Case<'a> = (&'a [(f64, f64)], &'a [Option<u16>]);
        let cases: &[Case<'_>] = &[
            (&[(0., 0.), (4., 0.)], &[Some(0), Some(180)]),
            (&[(0., 0.), (4., 3.)], &[Some(0), Some(180)]),
            (&[(0., 0.), (0., 3.)], &[Some(0), Some(180)]),
            (&[(0., 0.), (-4., 3.)], &[Some(0), Some(180)]),
            (&[(0., 0.), (-4., 0.)], &[Some(0), Some(180)]),
            (&[(0., 0.), (-4., -3.)], &[Some(0), Some(180)]),
            (&[(0., 0.), (0., -3.)], &[Some(0), Some(180)]),
            (&[(0., 0.), (4., -3.)], &[Some(0), Some(180)]),
            (&[(0., 0.), (4., 0.)], &[Some(0), Some(0)]),
            (&[(0., 0.), (4., 3.)], &[Some(0), Some(0)]),
            (&[(0., 0.), (0., 3.)], &[Some(0), Some(0)]),
            (&[(0., 0.), (-4., 3.)], &[Some(0), Some(0)]),
            (&[(0., 0.), (-4., 0.)], &[Some(0), Some(0)]),
            (&[(0., 0.), (-4., -3.)], &[Some(0), Some(0)]),
            (&[(0., 0.), (0., -3.)], &[Some(0), Some(0)]),
            (&[(0., 0.), (4., -3.)], &[Some(0), Some(0)]),
            (&[(0., 0.), (4., 0.)], &[Some(0), Some(90)]),
            (&[(0., 0.), (4., 3.)], &[Some(0), Some(90)]),
            (&[(0., 0.), (0., 3.)], &[Some(0), Some(90)]),
            (&[(0., 0.), (-4., 3.)], &[Some(0), Some(90)]),
            (&[(0., 0.), (-4., 0.)], &[Some(0), Some(90)]),
            (&[(0., 0.), (-4., -3.)], &[Some(0), Some(90)]),
            (&[(0., 0.), (0., -3.)], &[Some(0), Some(90)]),
            (&[(0., 0.), (4., -3.)], &[Some(0), Some(90)]),
            (&[(0., 0.), (4., 0.)], &[Some(0), Some(270)]),
            (&[(0., 0.), (4., 3.)], &[Some(0), Some(270)]),
            (&[(0., 0.), (0., 3.)], &[Some(0), Some(270)]),
            (&[(0., 0.), (-4., 3.)], &[Some(0), Some(270)]),
            (&[(0., 0.), (-4., 0.)], &[Some(0), Some(270)]),
            (&[(0., 0.), (-4., -3.)], &[Some(0), Some(270)]),
            (&[(0., 0.), (0., -3.)], &[Some(0), Some(270)]),
            (&[(0., 0.), (4., -3.)], &[Some(0), Some(270)]),
            (&[(0., 0.), (4., 0.)], &[Some(0), None]),
            (&[(0., 0.), (4., 3.)], &[Some(0), None]),
            (&[(0., 0.), (0., 3.)], &[Some(0), None]),
            (&[(0., 0.), (-4., 3.)], &[Some(0), None]),
            (&[(0., 0.), (-4., 0.)], &[Some(0), None]),
            (&[(0., 0.), (-4., -3.)], &[Some(0), None]),
            (&[(0., 0.), (0., -3.)], &[Some(0), None]),
            (&[(0., 0.), (4., -3.)], &[Some(0), None]),
            (&[(0., 0.), (4., 0.)], &[None, None]),
            (&[(0., 0.), (4., 3.)], &[None, None]),
            (&[(0., 0.), (0., 3.)], &[None, None]),
            (&[(0., 0.), (-4., 3.)], &[None, None]),
            (&[(0., 0.), (-4., 0.)], &[None, None]),
            (&[(0., 0.), (-4., -3.)], &[None, None]),
            (&[(0., 0.), (0., -3.)], &[None, None]),
            (&[(0., 0.), (4., -3.)], &[None, None]),
            (&[(0., 0.), (4., 4.), (4., 0.)], &[None, None, None]),
            (&[(4., 0.), (0., 0.), (4., 4.)], &[None, None, None]),
            (&[(0., 0.), (3., 4.), (8., 1.)], &[Some(0), None, Some(180)]),
            (&[(0., 0.), (4., 2.), (7., -2.)], &[Some(90), None, None]),
            (&[(0., 0.), (-4., 0.), (-4., 4.)], &[Some(0), None, None]),
            (&[(0., 0.), (3., 3.), (6., 0.), (9., 3.)], &[Some(0), None, None, Some(180)]),
        ];
        assert_eq!(cases.len(), 54);
        for &(points, directions) in cases {
            for orient in Orient::ALL {
                let mut p = problem_of(vec![instance(); points.len()], vec![]);
                let symbol = p.symbols[0].clone();
                p.symbols = vec![symbol; points.len()];
                for (k, symbol) in p.symbols.iter_mut().enumerate() {
                    p.objects[k].symbol = Some(k);
                    symbol.pins[0].pos = (0., 0.);
                    symbol.pins[0].dir = directions[k];
                }
                let anchors: Vec<_> = (0..points.len())
                    .map(|object| PointRef::SymbolPin { object, pin: 0, reference: Reference::Anonymous })
                    .collect();
                p.wires = vec![wire(anchors.clone(), "auto")];
                let mut g = Geometry::new(points.len());
                g.orient.fill(orient);
                g.origin = points
                    .iter()
                    .map(|&(x, y)| [crate::rational::from_f64(x).unwrap(), crate::rational::from_f64(y).unwrap()])
                    .collect();
                let direction = |d: Option<u16>| d.and_then(|d| orient.direction(d)).and_then(Direction::from_degrees);
                let mut expected: Vec<_> =
                    auto_wire(points, direction(directions[0]), direction(*directions.last().unwrap()))
                        .unwrap()
                        .into_iter()
                        .flatten()
                        .collect();
                expected.dedup();
                let result = auto_route(&p, &g, 0).unwrap().unwrap();
                let mut actual: Vec<_> = result.iter().map(|(_, p)| (to_f64(&p[0]), to_f64(&p[1]))).collect();
                actual.dedup();
                assert_eq!(actual, expected, "{points:?} {directions:?} {orient:?}");
                let refs: Vec<_> = result
                    .iter()
                    .filter_map(|(r, _)| match r {
                        WirePoint::Anchor(r) => Some(r.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(refs, anchors);
            }
        }
    }

    #[test]
    fn whole_wire_keeps_coincident_waypoints_and_effective_source_version() {
        let mut p = problem_of(vec![point(); 4], vec![]);
        let anchors: Vec<_> = (0..4).map(PointRef::Origin).collect();
        p.wires = vec![wire(anchors.clone(), "auto")];
        let g = geometry(&[(0, 0), (4, 4), (4, 4), (4, 0)]);
        let path = auto_route(&p, &g, 0).unwrap().unwrap();
        assert_eq!(
            path.iter()
                .filter_map(|(p, _)| match p {
                    WirePoint::Anchor(r) => Some(r.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            anchors
        );
        assert_eq!(path[1].1, [int(0), int(4)], "whole-wire continuation selects the other first bend");
        p.version = "0.3".into();
        p.wires[0].generated = true;
        assert_eq!(
            finish(&p, &g, Routing::default()).1.len(),
            1,
            "effective 0.4 source keeps the core path even in generated content"
        );
        p.version = "0.4".into();
        p.wires[0].default_auto = false;
        assert!(finish(&p, &g, Routing::default()).1.is_empty(), "effective legacy source stays legacy");
        p.wires[0].default_auto = true;
        let mut invalid = g;
        invalid.origin[1][0] = crate::rational::parse_decimal("1e-400").unwrap();
        let (routing, wires, dots) = finish(&p, &invalid, Routing::default());
        assert_eq!(routing.written_path_error, Some(0));
        assert!(wires.is_empty() && dots.is_empty());
    }
}
