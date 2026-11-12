//! Tests of the router on layouts given as plain geometry; `driver::tests` routes the cases of the
//! files.

use super::*;

fn p(x: i64, y: i64) -> [Q; 2] {
    [int(x), int(y)]
}

fn anchor(x: i64, y: i64, exit: Option<(i64, i64)>) -> AnchorIn {
    AnchorIn { at: p(x, y), exit, owner: None, connects_pin: true, existing_junction: false }
}

const EAST: Option<(i64, i64)> = Some((1, 0));
const WEST: Option<(i64, i64)> = Some((-1, 0));
const NORTH: Option<(i64, i64)> = Some((0, 1));

fn boxed(b: [i64; 4]) -> [Q; 4] {
    b.map(int)
}

/// A layout of some nets and boxes, every anchor a pin.
fn layout(boxes: &[[i64; 4]], nets: Vec<NetIn>) -> Layout {
    let pins = nets.iter().flat_map(|n| n.anchors.iter().map(|a| a.at.clone())).collect();
    Layout {
        boxes: boxes.iter().enumerate().map(|(k, b)| (100 + k, boxed(*b))).collect(),
        pins,
        written: Vec::new(),
        nets,
        ..Layout::default()
    }
}

fn net(anchors: Vec<AnchorIn>) -> NetIn {
    NetIn { anchors, joined: Vec::new(), preferred: None }
}

#[test]
fn nearest_frontier_preserves_global_distance_anchor_ties_and_components() {
    for n in 2..24 {
        for stride in 1..5 {
            let input = net((0..n).map(|i| anchor((i * 7 % 11) as i64, (i * 3 % 5) as i64, None)).collect());
            let groups: Vec<usize> = (0..n).map(|i| i / stride).collect();
            let mut inside: Vec<bool> = groups.iter().map(|g| *g == groups[0]).collect();
            let mut expected = Vec::new();
            loop {
                let mut candidates = Vec::new();
                for a in (0..n).filter(|&a| !inside[a]) {
                    for b in (0..n).filter(|&b| inside[b]) {
                        let (p, q) = (&input.anchors[a].at, &input.anchors[b].at);
                        candidates.push(((&p[0] - &q[0]).abs() + (&p[1] - &q[1]).abs(), a, b));
                    }
                }
                let Some((_, a, b)) = candidates.into_iter().min() else { break };
                expected.push((a, b));
                for k in 0..n {
                    inside[k] |= groups[k] == groups[a];
                }
            }
            assert_eq!(nearest_joins(&input, &groups), expected);
        }
    }
}

#[test]
fn exhausted_shared_budget_skips_the_grid_and_still_connects_every_anchor() {
    let input = net(vec![anchor(0, 0, None), anchor(100_000_000, 100_000_000, None), anchor(-100_000_000, 0, None)]);
    let mut layout = layout(&[], vec![input]);
    layout.modern = true;
    let ((out, expansions), report) =
        crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(0), None, || route_layout(&layout, 200_000));
    assert_eq!(report.work, 0);
    assert_eq!(expansions, 0);
    assert!(all_anchors_connected(&layout.nets[0], &out[0]));
    assert!(out[0].forced);
}

fn all_anchors_connected(net: &NetIn, out: &NetOut) -> bool {
    let n = net.anchors.len();
    let mut reached = vec![false; n + out.branches.len()];
    if n == 0 {
        return true;
    }
    reached[0] = true;
    let index = |e| match e {
        End::Anchor(a) => a,
        End::Branch(b) => n + b,
    };
    let edges: Vec<_> =
        net.joined.iter().copied().chain(out.wires.iter().map(|w| (index(w.start), index(w.end)))).collect();
    loop {
        let mut changed = false;
        for &(a, b) in &edges {
            if reached[a] != reached[b] {
                reached[a] = true;
                reached[b] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    reached[..n].iter().all(|v| *v)
}

#[test]
fn modern_routing_finishes_and_counts_a_budget_limited_tree() {
    let mut l =
        layout(&[[4, -3, 6, 3]], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST), anchor(10, 8, NORTH)])]);
    l.modern = true;
    let (outs, work) = route_layout(&l, 3);
    assert_eq!(work, 3);
    assert_eq!(outs[0].parts, 1);
    assert_eq!(outs[0].failure, None);
    assert!(outs[0].forced);
    assert!(all_anchors_connected(&l.nets[0], &outs[0]));
    assert_eq!(route_layout(&l, 3), (outs, work));
}

#[test]
fn modern_routing_connects_a_walled_in_pin_as_a_reported_last_resort() {
    let walls = [[-5, 1, 5, 5], [-5, -5, 5, -1], [-5, -5, -1, 5], [1, -5, 5, 5]];
    let mut l = layout(&walls, vec![net(vec![anchor(0, 0, None), anchor(20, 0, WEST), anchor(20, 10, NORTH)])]);
    l.modern = true;
    let (outs, _) = route_layout(&l, 100_000);
    assert!(outs[0].forced);
    assert!(all_anchors_connected(&l.nets[0], &outs[0]));
    assert_eq!(outs[0].parts, 1);
}

#[test]
fn text_boxes_are_soft_obstacles_in_modern_routing() {
    let mut l = layout(&[], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST)])]);
    l.modern = true;
    l.texts = vec![boxed([3, -1, 7, 1])];
    let (outs, _) = route_layout(&l, 100_000);
    assert!(!outs[0].forced);
    assert!(all_anchors_connected(&l.nets[0], &outs[0]));
    assert!(outs[0].wires.iter().flat_map(|w| w.vertices.windows(2)).all(|w| !meets_inside(&l.texts[0], &w[0], &w[1])));
    // A text cannot prevent a connection, even when it covers the entire routing lattice.
    l.texts = vec![boxed([-100, -100, 100, 100])];
    let (outs, _) = route_layout(&l, 100_000);
    assert!(!outs[0].forced);
    assert!(all_anchors_connected(&l.nets[0], &outs[0]));
}

#[test]
fn modern_jobs_keep_separate_trees_of_one_net() {
    use crate::resolve::tests::{instance, problem_of};
    use rnl_runtime::problem::RouteIntent;
    let mut p = problem_of((0..4).map(|_| instance()).collect(), vec![]);
    p.routes = [(0, 1), (2, 3)]
        .into_iter()
        .map(|(a, b)| RouteIntent {
            tree: false,
            net: "shared".into(),
            mode: "manhattan".into(),
            style: vec![],
            wires: vec![],
            pins: [a, b].into_iter().map(|object| PointRef::Pin { object, pin: 0, terminal: "1".into() }).collect(),
        })
        .collect();
    assert_eq!(jobs(&p).len(), 1);
    p.version = "0.4".into();
    let trees = jobs(&p);
    assert_eq!(trees.len(), 2);
    assert_eq!(trees.iter().map(|j| (j.route, j.points.len())).collect::<Vec<_>>(), [(0, 2), (1, 2)]);
    assert_ne!(trees[0].points[0].key, trees[1].points[0].key);
    // A resolved tree from a newer included file retains its identity in an older level.
    p.version = "0.3".into();
    for route in &mut p.routes {
        route.tree = true;
    }
    assert_eq!(jobs(&p).len(), 2);
}

#[test]
fn mixed_resolved_and_legacy_requests_keep_distinct_drawn_indices() {
    use crate::resolve::tests::{instance, problem_of};
    use rnl_runtime::problem::RouteIntent;
    let mut problem = problem_of((0..4).map(|_| instance()).collect(), vec![]);
    problem.version = "0.3".into();
    problem.routes = [(true, 0, 1), (false, 2, 3)]
        .into_iter()
        .map(|(tree, a, b)| RouteIntent {
            tree,
            net: "shared".into(),
            mode: "manhattan".into(),
            style: vec![],
            wires: vec![],
            pins: [a, b].into_iter().map(|object| PointRef::Pin { object, pin: 0, terminal: "g".into() }).collect(),
        })
        .collect();
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve(&problem, &options);
    let mut work = Work::default();
    let structure = crate::structure::analyse(&problem, &resolved, &options, crate::options::CAPS, &mut work);
    let mut geometry = Geometry::new(4);
    geometry.origin = vec![p(0, 0), p(12, 0), p(0, 12), p(12, 12)];
    let origins = structure.origins(&problem, &geometry);
    let routed = route(&problem, &structure, &geometry, &origins, &options, &mut work);
    assert_eq!(routed.wires.len(), 2);
    assert!(routed.wires.iter().zip(&routed.drawn).all(|(wire, drawn)| wire.route == *drawn));
    assert_eq!(routed.drawn, [0, 1]);
}

#[test]
fn a_modern_manhattan45_diagonal_has_no_zero_length_corner_segments() {
    use crate::resolve::tests::{point, problem_of};
    use rnl_runtime::problem::{ProblemWire, Reference};
    let mut problem = problem_of(vec![point(), point()], vec![]);
    problem.wires = vec![ProblemWire {
        label: "%w".into(),
        reference: Reference::Name("%w".into()),
        nets: vec![],
        bus: false,
        anchors: vec![PointRef::Origin(0), PointRef::Origin(1)],
        mode: "manhattan45".into(),
        default_auto: false,
        generated: false,
    }];
    let mut geometry = Geometry::new(2);
    geometry.origin[1] = p(4, 4);
    assert_eq!(written_path(&problem, &geometry, 0).unwrap().unwrap().len(), 4);
    problem.version = "0.4".into();
    assert_eq!(written_path(&problem, &geometry, 0).unwrap().unwrap(), vec![p(0, 0), p(4, 4)]);
}

#[test]
fn ordinary_routing_does_not_count_an_unconnected_pin_at_a_position_point() {
    for electrical in [false, true] {
        let mut n = net(vec![anchor(0, 0, None), anchor(-5, 0, None), anchor(0, 5, None)]);
        n.anchors[0].connects_pin = electrical;
        n.joined = vec![(0, 1)];
        let mut l = layout(&[], vec![n]);
        l.modern = true;
        let grid = Grid::new(&l).unwrap();
        let nodes: Vec<_> = l.nets[0].anchors.iter().map(|a| grid.lat.at(&a.at)).collect();
        let mut made = vec![Made { start: End::Anchor(0), end: End::Anchor(2), nodes: vec![] }];
        let mut branches = vec![];
        grid.junctions_at_anchors(&l.nets[0], &nodes, &mut made, &mut branches);
        assert_eq!(branches.len(), usize::from(electrical));
    }
}

#[test]
fn modern_solve_preserves_tree_identity_and_reports_forced_connections() {
    use crate::resolve::tests::{fact, instance, problem_of};
    use rnl_runtime::problem::{Axis, FactKind, RouteIntent};
    let mut p = problem_of((0..4).map(|_| instance()).collect(), vec![]);
    p.version = "0.4".into();
    for (object, (x, y)) in [(0.0, 0.0), (12.0, 0.0), (0.0, 12.0), (12.0, 12.0)].into_iter().enumerate() {
        p.facts.extend([
            fact(FactKind::Place { at: PointRef::Origin(object), axis: Axis::X, value: x }),
            fact(FactKind::Place { at: PointRef::Origin(object), axis: Axis::Y, value: y }),
            fact(FactKind::Orient { object, code: Orient::R0 }),
        ]);
    }
    p.routes = [(0, 1), (2, 3)]
        .into_iter()
        .map(|(a, b)| RouteIntent {
            tree: false,
            net: "shared".into(),
            mode: "manhattan".into(),
            style: vec![],
            wires: vec![],
            pins: [a, b].into_iter().map(|object| PointRef::Pin { object, pin: 0, terminal: "g".into() }).collect(),
        })
        .collect();
    let options = SolverOptions::without_time_limit().with_overrides("budget=candidates:0,astar_expansions:0").unwrap();
    let solved = crate::solve(&p, &options);
    assert_eq!(solved.solution.wires.len(), 2);
    assert!(solved.report.unrouted.is_empty());
    assert!(
        solved.report.notes.iter().any(|n| n.starts_with("2 ROUTE trees joined through obstacles")),
        "{:?}",
        solved.report.notes
    );
    for wire in &solved.solution.wires {
        let allowed = if wire.route == 0 {
            [0, 1]
        } else {
            assert_eq!(wire.route, 1);
            [2, 3]
        };
        for point in &wire.points {
            if let WirePoint::Anchor(pin) = point {
                assert!(allowed.contains(&pin.pin_key().unwrap().0));
            }
        }
    }
    assert_eq!(solved, crate::solve(&p, &options));
}

#[test]
fn a_pin_position_does_not_become_an_electrical_junction_during_fallback() {
    use crate::resolve::tests::{fact, instance, point, problem_of};
    use rnl_runtime::problem::{Axis, FactKind, ProblemWire, Reference, RouteIntent};
    for electrical in [false, true] {
        let mut p = problem_of(vec![instance(), instance(), point()], vec![]);
        p.version = "0.4".into();
        p.junction.definition = Some("$DOT".into());
        let at = [(0.0, 0.0), (0.0, 10.0), (-8.0, 0.0)];
        for (object, (x, y)) in at.into_iter().enumerate() {
            p.facts.extend([
                fact(FactKind::Place { at: PointRef::Origin(object), axis: Axis::X, value: x }),
                fact(FactKind::Place { at: PointRef::Origin(object), axis: Axis::Y, value: y }),
            ]);
            if object < 2 {
                p.facts.push(fact(FactKind::Orient { object, code: Orient::R0 }));
            }
        }
        let end = if electrical {
            PointRef::Pin { object: 0, pin: 0, terminal: "g".into() }
        } else {
            PointRef::PinPosition { object: 0, pin: 0, reference: Reference::Name("#M:g::position".into()) }
        };
        p.wires = vec![ProblemWire {
            label: "%w".into(),
            reference: Reference::Name("%w".into()),
            nets: vec!["shared".into()],
            bus: false,
            anchors: vec![PointRef::Origin(2), end.clone()],
            mode: "straight".into(),
            default_auto: false,
            generated: false,
        }];
        p.routes = vec![RouteIntent {
            tree: false,
            net: "shared".into(),
            mode: "manhattan".into(),
            style: vec![],
            wires: vec![0],
            pins: vec![PointRef::Pin { object: 1, pin: 0, terminal: "g".into() }],
        }];
        let options =
            SolverOptions::without_time_limit().with_overrides("budget=candidates:0,astar_expansions:0").unwrap();
        let solved = crate::solve(&p, &options);
        assert_eq!(solved.solution.branches.len(), usize::from(electrical));
        let written = rnl_runtime::solution::write_record(&p, &solved.solution, "test", "");
        assert_eq!(written.record.contains("$DOT"), electrical, "{}", written.record);
        if !electrical {
            let anchor = WirePoint::Anchor(end);
            assert!(solved.solution.wires.iter().any(|w| w.points.contains(&anchor)));
        }
        p.junction.definition = None;
        let written = rnl_runtime::solution::write_record(&p, &solved.solution, "test", "");
        assert!(!written.record.contains("$DOT"));
    }
}

/// What every routed net must be: wires along the axes, through no box (but the one a pin of the
/// wire is inside), clear of the widened boxes but where they leave a pin, from an anchor or a
/// branch point to another, every branch point shared by three wires or more, or on an anchor
/// and shared by two, one of them the wire of length 0 to the anchor; and the parts as said; and
/// the same twice.
fn check(l: &Layout, budget: u64) -> Vec<NetOut> {
    let (outs, _) = route_layout(l, budget);
    assert_eq!(route_layout(l, budget).0, outs, "not deterministic");
    for (n, out) in l.nets.iter().zip(&outs) {
        let end_at = |e: End| match e {
            End::Anchor(a) => n.anchors[a].at.clone(),
            End::Branch(b) => out.branches[b].clone(),
        };
        let mut ends: Vec<usize> = vec![0; out.branches.len()];
        let mut on_anchor = vec![false; out.branches.len()];
        for w in &out.wires {
            if let (End::Branch(b), End::Anchor(_)) = (w.start, w.end) {
                on_anchor[b] |= w.vertices.len() == 2 && w.vertices[0] == w.vertices[1];
            }
            assert!(w.vertices.len() >= 2, "{w:?}");
            assert_eq!(w.vertices[0], end_at(w.start), "{w:?}");
            assert_eq!(w.vertices[w.vertices.len() - 1], end_at(w.end), "{w:?}");
            for e in [w.start, w.end] {
                if let End::Branch(b) = e {
                    ends[b] += 1;
                }
            }
            for s in w.vertices.windows(2) {
                assert!(s[0][0] == s[1][0] || s[0][1] == s[1][1], "not along an axis: {w:?}");
                for (_, b) in &l.boxes {
                    assert!(!meets_inside(b, &s[0], &s[1]), "through the box {b:?}: {w:?}");
                }
            }
        }
        for (b, e) in ends.iter().enumerate() {
            assert!(*e >= 3 || (*e == 2 && on_anchor[b]), "a branch point not shared by three wires: {ends:?}");
        }
        assert_eq!(out.failure.is_some(), out.parts > 1, "{out:?}");
    }
    outs
}

#[test]
fn two_pins_that_face_each_other_are_one_straight_wire() {
    let l = layout(&[], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST)])]);
    let outs = check(&l, 10_000);
    assert_eq!(outs[0].wires.len(), 1);
    assert_eq!(outs[0].wires[0].vertices, vec![p(0, 0), p(10, 0)]);
    assert_eq!((outs[0].parts, outs[0].failure), (1, None));
    assert!(outs[0].branches.is_empty());
}

#[test]
fn a_wire_goes_round_a_symbol_clear_of_its_box() {
    let l = layout(&[[4, -3, 6, 3]], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST)])]);
    let outs = check(&l, 10_000);
    let w = &outs[0].wires[0];
    // round the widened box, (3, 7) × (−4, 4): 10 across and 4 up and down again
    let length: Q = w.vertices.windows(2).map(|s| (&s[1][0] - &s[0][0]).abs() + (&s[1][1] - &s[0][1]).abs()).sum();
    assert_eq!(length, int(18), "{w:?}");
    assert_eq!(w.vertices.len(), 6, "four bends: {w:?}");
    let widened = boxed([3, -4, 7, 4]);
    for s in w.vertices.windows(2) {
        assert!(!meets_inside(&widened, &s[0], &s[1]), "{w:?}");
    }
}

#[test]
fn a_third_pin_branches_off_the_middle_of_a_wire_with_a_junction() {
    let l = layout(&[], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST), anchor(5, -6, NORTH)])]);
    let outs = check(&l, 10_000);
    let out = &outs[0];
    assert_eq!(out.branches, vec![p(5, 0)]);
    assert_eq!(out.wires.len(), 3, "{out:?}");
    let mut ends: Vec<(End, End)> = out.wires.iter().map(|w| (w.start, w.end)).collect();
    ends.sort_by_key(|(a, b)| format!("{a:?}{b:?}"));
    assert_eq!(
        ends,
        vec![(End::Anchor(0), End::Branch(0)), (End::Anchor(2), End::Branch(0)), (End::Branch(0), End::Anchor(1)),]
    );
    // a fourth branches off at the same point, which then joins four wires
    let l = layout(
        &[],
        vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST), anchor(5, -6, NORTH), anchor(5, 6, Some((0, -1)))])],
    );
    let out = &check(&l, 10_000)[0];
    assert_eq!(out.branches, vec![p(5, 0)], "{out:?}");
    assert_eq!(out.wires.len(), 4, "{out:?}");
}

#[test]
fn pins_that_touch_are_joined_by_a_wire_of_length_zero() {
    let l = layout(&[], vec![net(vec![anchor(3, 2, NORTH), anchor(3, 2, Some((0, -1)))])]);
    let out = &check(&l, 10_000)[0];
    assert_eq!(out.wires.len(), 1);
    assert_eq!(out.wires[0].vertices, vec![p(3, 2), p(3, 2)]);
    assert_eq!((out.wires[0].start, out.wires[0].end), (End::Anchor(0), End::Anchor(1)));
}

#[test]
fn a_wire_crosses_another_straight_rather_than_go_far_round_it() {
    let mut l = layout(&[], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST)])]);
    l.written.push(vec![p(5, -20), p(5, 20)]);
    let out = &check(&l, 100_000)[0];
    assert_eq!(out.wires[0].vertices, vec![p(0, 0), p(10, 0)]);
    // two nets that cross: the second crosses the first, straight
    let l = layout(
        &[],
        vec![
            net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST)]),
            net(vec![anchor(5, -5, NORTH), anchor(5, 5, Some((0, -1)))]),
        ],
    );
    let outs = check(&l, 100_000);
    assert_eq!(outs[0].wires[0].vertices, vec![p(0, 0), p(10, 0)]);
    assert_eq!(outs[1].wires[0].vertices, vec![p(5, -5), p(5, 5)]);
}

#[test]
fn a_wire_never_runs_along_another_nor_touches_its_end() {
    let mut l = layout(&[], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST)])]);
    l.written.push(vec![p(2, 0), p(8, 0)]);
    let out = &check(&l, 100_000)[0];
    let w = &out.wires[0];
    for s in w.vertices.windows(2) {
        let along = s[0][1] == int(0) && s[1][1] == int(0);
        let (lo, hi) = if s[0][0] < s[1][0] { (&s[0][0], &s[1][0]) } else { (&s[1][0], &s[0][0]) };
        assert!(!(along && *lo < int(8) && *hi > int(2)), "{w:?}");
        assert!(!w.vertices.contains(&p(2, 0)) && !w.vertices.contains(&p(8, 0)), "{w:?}");
    }
}

#[test]
fn a_wire_leaves_its_pin_the_way_the_symbol_says() {
    let l = layout(&[], vec![net(vec![anchor(0, 0, NORTH), anchor(6, 4, WEST)])]);
    let out = &check(&l, 10_000)[0];
    assert_eq!(out.wires[0].vertices, vec![p(0, 0), p(0, 4), p(6, 4)]);
}

#[test]
fn a_wire_written_on_the_net_is_part_of_its_tree() {
    let mut n = net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST), anchor(5, -6, NORTH)]);
    n.joined.push((0, 1));
    let mut l = layout(&[], vec![n]);
    l.written.push(vec![p(0, 0), p(10, 0)]);
    let out = &check(&l, 100_000)[0];
    // one wire, between the third pin and an end of the wire written: its middle is not a point
    // of it (13.1). There the two wires and the lead of the pin meet in a T: the wire ends at a
    // branch point with a junction on the pin, which a wire of length 0 joins to the pin (13.2)
    assert_eq!(out.wires.len(), 2, "{out:?}");
    assert_eq!(out.branches.len(), 1, "{out:?}");
    let mut ends = [out.wires[0].start, out.wires[0].end];
    ends.sort_by_key(|e| format!("{e:?}"));
    assert_eq!(ends, [End::Anchor(2), End::Branch(0)], "{out:?}");
    let pin = match out.wires[1].end {
        End::Anchor(a @ (0 | 1)) => a,
        _ => panic!("{out:?}"),
    };
    assert_eq!(out.wires[1].start, End::Branch(0));
    assert_eq!(out.branches[0], l.nets[0].anchors[pin].at);
    assert_eq!(out.wires[1].vertices, vec![out.branches[0].clone(), out.branches[0].clone()]);
    assert_eq!(out.parts, 1);
}

/// Two wires made that meet at a pin from two sides make a T with its lead: they end at a branch
/// point with a junction on the pin instead, which a wire of length 0 joins to it; where they meet
/// at a point, which has no lead, they do not.
#[test]
fn wires_that_meet_at_a_pin_get_a_junction_there() {
    // the pin at (0, 0) leaves up; the two others are on either side of the way up, and each is
    // nearest to it, so that each joins the tree at the pin
    let l = layout(&[[-1, -4, 1, 0]], vec![net(vec![anchor(0, 0, NORTH), anchor(-6, 5, EAST), anchor(6, 5, WEST)])]);
    let out = &check(&l, 100_000)[0];
    assert_eq!(out.parts, 1, "{out:?}");
    let at_pin: Vec<&WireOut> =
        out.wires.iter().filter(|w| w.end == End::Anchor(0) || w.start == End::Anchor(0)).collect();
    if out.branches.iter().any(|b| *b == p(0, 0)) {
        assert_eq!(at_pin.len(), 1, "{out:?}");
        assert_eq!(at_pin[0].vertices, vec![p(0, 0), p(0, 0)], "{out:?}");
    } else {
        assert_eq!(at_pin.len(), 1, "one wire at the pin, the other branching off it: {out:?}");
    }
    // a pin where two wires made end: built from plain ends to have no choice
    let mut made = vec![
        Made { start: End::Anchor(1), end: End::Anchor(0), nodes: Vec::new() },
        Made { start: End::Anchor(2), end: End::Anchor(0), nodes: Vec::new() },
    ];
    let grid = Grid::new(&l).unwrap();
    let nodes: Vec<Option<usize>> = l.nets[0].anchors.iter().map(|a| grid.lat.at(&a.at)).collect();
    let mut branches = Vec::new();
    grid.junctions_at_anchors(&l.nets[0], &nodes, &mut made, &mut branches);
    assert_eq!(branches, vec![nodes[0].unwrap()]);
    let ends: Vec<(End, End)> = made.iter().map(|m| (m.start, m.end)).collect();
    assert_eq!(
        ends,
        vec![(End::Anchor(1), End::Branch(0)), (End::Anchor(2), End::Branch(0)), (End::Branch(0), End::Anchor(0))]
    );
    // at a point, two wires make no T: it has no lead
    let mut l = l.clone();
    l.pins.retain(|q| *q != p(0, 0));
    let grid = Grid::new(&l).unwrap();
    let nodes: Vec<Option<usize>> = l.nets[0].anchors.iter().map(|a| grid.lat.at(&a.at)).collect();
    let mut made = vec![
        Made { start: End::Anchor(1), end: End::Anchor(0), nodes: Vec::new() },
        Made { start: End::Anchor(2), end: End::Anchor(0), nodes: Vec::new() },
    ];
    let mut branches = Vec::new();
    grid.junctions_at_anchors(&l.nets[0], &nodes, &mut made, &mut branches);
    assert!(branches.is_empty());
    // a pin where a wire made and a wire written end, but not two pins that touch
    let mut n = net(vec![anchor(0, 0, NORTH), anchor(-6, 5, EAST), anchor(0, 0, Some((0, -1)))]);
    n.joined.push((0, 1));
    let l = layout(&[], vec![n]);
    let grid = Grid::new(&l).unwrap();
    let nodes: Vec<Option<usize>> = l.nets[0].anchors.iter().map(|a| grid.lat.at(&a.at)).collect();
    let mut made = vec![Made { start: End::Anchor(2), end: End::Anchor(0), nodes: Vec::new() }];
    let mut branches = Vec::new();
    grid.junctions_at_anchors(&l.nets[0], &nodes, &mut made, &mut branches);
    assert_eq!(branches.len(), 1, "the wire written, the wire made and the lead: {made:?}");
    let n = net(vec![anchor(0, 0, NORTH), anchor(0, 0, Some((0, -1)))]);
    let l = layout(&[], vec![n]);
    let grid = Grid::new(&l).unwrap();
    let nodes: Vec<Option<usize>> = l.nets[0].anchors.iter().map(|a| grid.lat.at(&a.at)).collect();
    let mut made = vec![Made { start: End::Anchor(1), end: End::Anchor(0), nodes: Vec::new() }];
    let mut branches = Vec::new();
    grid.junctions_at_anchors(&l.nets[0], &nodes, &mut made, &mut branches);
    assert!(branches.is_empty(), "{made:?}");
}

#[test]
fn a_pin_walled_in_is_reported_and_the_rest_routed() {
    let walls = [[-5, 1, 5, 5], [-5, -5, 5, -1], [-5, -5, -1, 5], [1, -5, 5, 5]];
    let l = layout(&walls, vec![net(vec![anchor(0, 0, None), anchor(20, 0, WEST), anchor(20, 10, Some((0, -1)))])]);
    let out = &check(&l, 100_000)[0];
    assert_eq!((out.parts, out.failure), (2, Some(Failure::NoPath)), "{out:?}");
    // the two outside are joined
    assert_eq!(out.wires.len(), 1, "{out:?}");
}

#[test]
fn a_net_whose_budget_runs_out_is_left_in_parts() {
    let l = layout(&[[4, -3, 6, 3]], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST)])]);
    let (outs, expansions) = route_layout(&l, 3);
    assert_eq!(expansions, 3);
    assert_eq!((outs[0].parts, outs[0].failure), (2, Some(Failure::Budget)));
    assert!(outs[0].wires.is_empty());
    // the budget is the net's own: the next net has its own
    let l = layout(
        &[],
        vec![net(vec![anchor(0, 0, EAST), anchor(30, 0, WEST)]), net(vec![anchor(0, 5, EAST), anchor(2, 5, WEST)])],
    );
    let (outs, _) = route_layout(&l, 10);
    assert_eq!(outs[0].failure, Some(Failure::Budget));
    assert_eq!(outs[1].failure, None);
}

#[test]
fn coordinates_off_the_integers_are_nodes_of_the_lattice() {
    let half = |v: i64| crate::rational::frac(v, 2);
    let a =
        AnchorIn { at: [half(1), half(-3)], exit: EAST, owner: None, connects_pin: false, existing_junction: false };
    let b =
        AnchorIn { at: [half(13), half(5)], exit: WEST, owner: None, connects_pin: false, existing_junction: false };
    let l = layout(&[], vec![net(vec![a, b])]);
    let out = &check(&l, 100_000)[0];
    let w = &out.wires[0];
    assert_eq!(w.vertices.first(), Some(&[half(1), half(-3)]));
    assert_eq!(w.vertices.last(), Some(&[half(13), half(5)]));
    assert_eq!(w.vertices.len(), 4, "{w:?}");
}

/// A pin on the edge of its own symbol leaves it by its lead; the box of the symbol is not crossed.
#[test]
fn a_pin_on_its_symbol_leaves_by_its_lead() {
    // a symbol (0, -2)–(3, 4) with a pin on its left edge facing left, the other pin beyond it to
    // the right: out to the left first, then round below, where it is shorter
    let mut l = layout(&[[0, -2, 3, 4]], vec![net(vec![anchor(0, 0, WEST), anchor(8, 0, None)])]);
    l.nets[0].anchors[0].owner = Some(100);
    let out = &check(&l, 100_000)[0];
    let w = &out.wires[0];
    assert_eq!(w.vertices, vec![p(0, 0), p(-1, 0), p(-1, -3), p(8, -3), p(8, 0)], "{w:?}");
    assert_eq!(out.parts, 1);
    // a lead other than the pin's own way does not run along the edge of its box
    let lat_box = boxed([0, -2, 3, 4]);
    assert!(runs_along(&lat_box, &p(0, 0), &p(0, 5)));
    assert!(!runs_along(&lat_box, &p(0, 0), &p(-1, 0)));
    assert!(runs_along(&lat_box, &p(0, 0), &p(4, 0)));
    // from a corner, out along x touches the box only at the corner
    assert!(!runs_along(&lat_box, &p(3, 4), &p(5, 4)));
    assert!(runs_along(&lat_box, &p(3, 4), &p(1, 4)));
}

/// A pin inside the box of another symbol, as when symbols overlap, cannot be left without running
/// through that symbol: it is reported. A pin inside its own symbol's box leaves through it.
#[test]
fn a_pin_inside_another_symbol_is_reported_and_inside_its_own_leaves_through_it() {
    let l = layout(&[[-2, -2, 2, 2]], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST)])]);
    let out = &check(&l, 100_000)[0];
    assert_eq!((out.parts, out.failure), (2, Some(Failure::NoPath)), "{out:?}");
    // next to it, a step from the pin into the target does not cross the box either
    let l = layout(&[[-2, -2, 2, 2]], vec![net(vec![anchor(0, 0, EAST), anchor(1, 0, WEST)])]);
    let out = &check(&l, 100_000)[0];
    assert_eq!(out.parts, 2, "{out:?}");
    let mut l = layout(&[[-2, -2, 2, 2]], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST)])]);
    l.nets[0].anchors[0].owner = Some(100);
    let (outs, _) = route_layout(&l, 100_000);
    assert_eq!(outs[0].parts, 1, "{:?}", outs[0]);
    assert_eq!(outs[0].wires[0].vertices, vec![p(0, 0), p(10, 0)]);
}

/// A layout of symbols and nets given in full: each symbol a box, by its object, and each anchor a
/// pin of the symbol it names.
fn symbols(boxes: &[(usize, [i64; 4])], nets: Vec<NetIn>, written: Vec<Vec<[Q; 2]>>) -> Layout {
    let pins = nets.iter().flat_map(|n| n.anchors.iter().map(|a| a.at.clone())).collect();
    Layout { boxes: boxes.iter().map(|(k, b)| (*k, boxed(*b))).collect(), pins, written, nets, ..Layout::default() }
}

fn pin(x: i64, y: i64, exit: Option<(i64, i64)>, owner: usize) -> AnchorIn {
    AnchorIn { at: p(x, y), exit, owner: Some(owner), connects_pin: true, existing_junction: false }
}

/// Whether a segment runs along the edge of a box: on the line of one of its edges, over some of
/// its length.
fn along_edge(b: &[Q; 4], s: &[[Q; 2]]) -> bool {
    let (a, c) = (&s[0], &s[1]);
    if a[1] == c[1] && (a[1] == b[1] || a[1] == b[3]) {
        let (lo, hi) = if a[0] <= c[0] { (&a[0], &c[0]) } else { (&c[0], &a[0]) };
        return lo.max(&b[0]) < hi.min(&b[2]);
    }
    if a[0] == c[0] && (a[0] == b[0] || a[0] == b[2]) {
        let (lo, hi) = if a[1] <= c[1] { (&a[1], &c[1]) } else { (&c[1], &a[1]) };
        return lo.max(&b[1]) < hi.min(&b[3]);
    }
    false
}

/// Two resistors in a row 2 apart, a pin of net 1 facing a pin of net 0 across the gap: both lead
/// to the one node between them, which neither net keeps. Net 0, routed first, takes it; the pin
/// of net 1 leaves along the edge of its symbol instead, and both nets are one tree (review of S4).
#[test]
fn a_pin_whose_own_way_is_taken_leaves_along_the_edge_of_its_symbol() {
    let boxes = [(100, [-11, -1, -5, 1]), (101, [-3, -1, 3, 1])];
    let nets = vec![
        net(vec![pin(-11, 0, WEST, 100), pin(-3, 0, WEST, 101)]),
        net(vec![pin(-5, 0, EAST, 100), pin(3, 0, EAST, 101)]),
    ];
    let l = symbols(&boxes, nets, Vec::new());
    let grid = Grid::new(&l).unwrap();
    assert_eq!(grid.kept[grid.lat.at(&p(-4, 0)).unwrap()], SHARED);
    let outs = check(&l, 100_000);
    for out in &outs {
        assert_eq!((out.parts, out.failure), (1, None), "{outs:?}");
    }
    // net 0 runs through the node between them; net 1 leaves (−5, 0) up or down, along x = −5
    let w = &outs[0].wires[0];
    assert!(w.vertices.contains(&p(-4, 0)), "{w:?}");
    let w = &outs[1].wires[0];
    let first = if w.vertices[0] == p(-5, 0) { &w.vertices[..2] } else { &w.vertices[w.vertices.len() - 2..] };
    assert!(first.iter().all(|v| v[0] == int(-5)), "{w:?}");
    assert!(along_edge(&boxed([-11, -1, -5, 1]), first), "{w:?}");
}

/// A pin with its own way open takes it, though a lead along the edge of its symbol would be
/// shorter: that lead is only a fallback.
#[test]
fn a_pin_with_its_own_way_open_does_not_run_along_its_symbol() {
    let boxes = [(100, [-1, -3, 1, 3]), (101, [5, -3, 7, 3])];
    let l = symbols(&boxes, vec![net(vec![pin(0, 3, NORTH, 100), pin(6, 3, NORTH, 101)])], Vec::new());
    let out = &check(&l, 100_000)[0];
    assert_eq!(out.wires[0].vertices, vec![p(0, 3), p(0, 4), p(6, 4), p(6, 3)], "{out:?}");
}

/// A supply stub written on each of two pins takes the pin's own way (13.7, the standard package's
/// $SUPPLY_STUB): the stub's wire is part of the tree and no new wire runs along it, so the two
/// parts are joined from the pins along the edges of their symbols, or from the stubs' pins along
/// theirs; where the new wire meets the stub's at a pin, a junction (review of S4).
#[test]
fn a_pin_whose_own_way_a_wire_of_its_net_takes_is_joined_along_its_symbol() {
    let boxes = [(100, [-2, -3, 2, 3]), (101, [-1, 5, 1, 6]), (102, [-7, -3, -5, 3]), (103, [-7, 5, -5, 6])];
    let south = Some((0, -1));
    let mut n = net(vec![pin(0, 3, NORTH, 100), pin(0, 5, south, 101), pin(-6, 3, NORTH, 102), pin(-6, 5, south, 103)]);
    n.joined = vec![(0, 1), (2, 3)];
    let l = symbols(&boxes, vec![n], vec![vec![p(0, 3), p(0, 5)], vec![p(-6, 3), p(-6, 5)]]);
    let out = &check(&l, 100_000)[0];
    assert_eq!((out.parts, out.failure), (1, None), "{out:?}");
    let long: Vec<&WireOut> = out.wires.iter().filter(|w| w.vertices[0] != w.vertices[w.vertices.len() - 1]).collect();
    assert_eq!(long.len(), 1, "{out:?}");
    assert_eq!(long[0].vertices.len(), 2, "one straight wire: {out:?}");
    assert_eq!(long[0].vertices[0][1], long[0].vertices[1][1], "{out:?}");
    // it meets a stub's wire at both ends: two junctions, each with its wire of length 0
    assert_eq!(out.branches.len(), 2, "{out:?}");
    assert_eq!(out.wires.len(), 3, "{out:?}");
}

/// A part walled in is found before any search: the search from its neighbour does not flood the
/// lattice looking for it, so the budget of the net is left for the rest (review of S4).
#[test]
fn a_part_walled_in_spends_none_of_the_budget() {
    let walls = [[-5, 1, 5, 5], [-5, -5, 5, -1], [-5, -5, -1, 5], [1, -5, 5, 5]];
    // the walled pin is the nearest to the first pin, so it is in the first pair of parts
    let l = layout(&walls, vec![net(vec![anchor(0, 0, None), anchor(0, -10, NORTH), anchor(40, -10, WEST)])]);
    // a flood would expand every free node at least once, more than the budget
    let free = Grid::new(&l).unwrap().blocked.iter().filter(|b| !**b).count();
    assert!(free > 500, "{free}");
    let (outs, expansions) = route_layout(&l, 500);
    assert_eq!((outs[0].parts, outs[0].failure), (2, Some(Failure::NoPath)), "{:?}", outs[0]);
    assert_eq!(outs[0].wires.len(), 1, "{:?}", outs[0]);
    assert!(expansions < 200, "{expansions}");
}

/// A part whose only way out passes the pin of a part not yet joined finds no path at first; once
/// that part is joined, its pin is a point of the tree, and the part is tried again (review of S4).
#[test]
fn a_part_that_found_no_path_is_tried_again_once_the_tree_grows() {
    // a pocket open to the right: x = (0, 0) inside it, q = (3, 0) at its mouth; the tree starts
    // with the pair to the left, which is nearer to x than to q
    let walls = [[-5, 1, 5, 5], [-5, -5, 5, -1], [-5, -5, -1, 5]];
    let l = layout(
        &walls,
        vec![net(vec![anchor(0, 0, EAST), anchor(3, 0, EAST), anchor(-20, 0, NORTH), anchor(-20, 2, Some((0, -1)))])],
    );
    let out = &check(&l, 100_000)[0];
    assert_eq!((out.parts, out.failure), (1, None), "{out:?}");
    // x joins the tree at q
    let x = out.wires.iter().find(|w| w.start == End::Anchor(0) || w.end == End::Anchor(0)).unwrap();
    assert!(x.vertices.iter().all(|v| v[1] == int(0)), "{x:?}");
}

/// The lattice has the tracks near the symbols and skips the empty stretch between symbols placed
/// far apart: the level is routed, its lattice small (review of S4).
#[test]
fn symbols_placed_far_apart_make_a_small_lattice() {
    let boxes = [(100, [-6, -1, 0, 1]), (101, [25000, -1, 25006, 1])];
    let l = symbols(&boxes, vec![net(vec![pin(0, 0, EAST, 100), pin(25000, 0, WEST, 101)])], Vec::new());
    let lat = Grid::new(&l).unwrap().lat;
    assert!(lat.nx() < 100 && lat.ny() < 40, "{} × {}", lat.nx(), lat.ny());
    let out = &check(&l, 100_000)[0];
    assert_eq!(out.wires[0].vertices, vec![p(0, 0), p(25000, 0)], "{out:?}");
    // near the symbols every integer is a node, out to the margin of the level
    for x in (-10..=TRACKS).chain(25000 - TRACKS..=25010) {
        assert!(Lattice::index(&lat.xs, &int(x)).is_some(), "{x}");
    }
}

#[test]
fn auto_written_path_uses_source_marker_and_shared_default() {
    use crate::resolve::tests::{point, problem_of};
    use rnl_runtime::problem::{ProblemWire, Reference};
    let mut problem = problem_of(vec![point(), point()], vec![]);
    problem.version = "0.3".into(); // source marker, not target version, controls auto
    problem.wires = vec![ProblemWire {
        label: "%w".into(),
        reference: Reference::Name("%w".into()),
        nets: vec![],
        bus: false,
        anchors: vec![PointRef::Origin(0), PointRef::Origin(1)],
        mode: "auto".into(),
        default_auto: true,
        generated: false,
    }];
    let mut geometry = Geometry::new(2);
    geometry.origin[1] = p(4, 3);
    assert_eq!(written_path(&problem, &geometry, 0).unwrap().unwrap(), vec![p(0, 0), p(4, 0), p(4, 3)]);
    problem.version = "0.4".into();
    problem.wires[0].default_auto = false;
    assert_eq!(written_path(&problem, &geometry, 0).unwrap().unwrap(), vec![p(0, 0), p(4, 3)]);
}

#[test]
fn separated_two_anchor_trees_each_keep_the_shared_default_path() {
    let mut a = net(vec![anchor(0, 0, None), anchor(4, 3, None)]);
    let mut b = net(vec![anchor(100, 100, None), anchor(104, 103, None)]);
    a.preferred = shared_auto_path(&[p(0, 0), p(4, 3)], None, None);
    b.preferred = shared_auto_path(&[p(100, 100), p(104, 103)], None, None);
    let mut l = layout(&[], vec![a, b]);
    l.modern = true;
    assert!(clear_static_default(&l, 0, l.nets[0].preferred.as_ref().unwrap()));
    assert!(clear_static_default(&l, 1, l.nets[1].preferred.as_ref().unwrap()));
    let (outs, _) = route_layout(&l, 100_000);
    for (index, expected) in
        [vec![p(0, 0), p(4, 0), p(4, 3)], vec![p(100, 100), p(104, 100), p(104, 103)]].iter().enumerate()
    {
        assert_eq!(outs[index].wires.len(), 1, "{:?}", outs[index]);
        let w = &outs[index].wires[0];
        let mut points = w.vertices.clone();
        if w.start == End::Anchor(1) {
            points.reverse();
        }
        assert_eq!(&points, expected);
    }
}

#[test]
fn blocked_default_candidate_does_not_bypass_the_router() {
    let mut n = net(vec![anchor(0, 0, None), anchor(4, 3, None)]);
    let path = shared_auto_path(&[p(0, 0), p(4, 3)], None, None).unwrap();
    n.preferred = Some(path.clone());
    let mut l = layout(&[[1, -1, 3, 1]], vec![n]);
    l.modern = true;
    assert!(!clear_static_default(&l, 0, &path));
    // A candidate rejected during assembly is not installed in the routing lattice.
    l.nets[0].preferred = None;
    let (out, _) = route_layout(&l, 100_000);
    assert_ne!(out[0].wires[0].vertices, path);
}

#[test]
fn default_wire_rotates_pin_exit_but_not_its_position_point() {
    use crate::resolve::tests::{instance, point, problem_of};
    use rnl_runtime::problem::{ProblemWire, Reference};
    let mut problem = problem_of(vec![instance(), point()], vec![]);
    problem.version = "0.4".into();
    problem.wires = vec![ProblemWire {
        label: "%w".into(),
        reference: Reference::Name("%w".into()),
        nets: vec![],
        bus: false,
        anchors: vec![PointRef::Pin { object: 0, pin: 0, terminal: "g".into() }, PointRef::Origin(1)],
        mode: "auto".into(),
        default_auto: true,
        generated: false,
    }];
    let mut geometry = Geometry::new(2);
    geometry.orient[0] = Orient::R90;
    geometry.origin[1] = p(4, 0);
    let pin_path = written_path(&problem, &geometry, 0).unwrap().unwrap();
    let expected = shared_auto_path(&[p(0, -3), p(4, 0)], Some(Direction::Down), None).unwrap();
    assert_eq!(pin_path, expected);
    assert_eq!(pin_path[0][0], pin_path[1][0]);
    assert!(pin_path[1][1] < pin_path[0][1]);
    problem.wires[0].anchors[0] =
        PointRef::PinPosition { object: 0, pin: 0, reference: Reference::Name("#M::g::position".into()) };
    let position_path = written_path(&problem, &geometry, 0).unwrap().unwrap();
    assert_eq!(position_path, shared_auto_path(&[p(0, -3), p(4, 0)], None, None).unwrap());
}

#[test]
fn default_wire_keeps_intermediate_waypoint_and_legacy_modes() {
    use crate::resolve::tests::{point, problem_of};
    use rnl_runtime::problem::{ProblemWire, Reference};
    let mut problem = problem_of(vec![point(), point(), point()], vec![]);
    problem.version = "0.4".into();
    problem.wires = vec![ProblemWire {
        label: "%w".into(),
        reference: Reference::Name("%w".into()),
        nets: vec![],
        bus: false,
        anchors: vec![PointRef::Origin(0), PointRef::Origin(1), PointRef::Origin(2)],
        mode: "auto".into(),
        default_auto: true,
        generated: false,
    }];
    let mut geometry = Geometry::new(3);
    geometry.origin = vec![p(0, 0), p(4, 4), p(4, 0)];
    assert_eq!(written_path(&problem, &geometry, 0).unwrap().unwrap(), vec![p(0, 0), p(0, 4), p(4, 4), p(4, 0)]);
    problem.wires[0].mode = "straight".into();
    problem.wires[0].default_auto = false;
    assert_eq!(written_path(&problem, &geometry, 0).unwrap().unwrap(), vec![p(0, 0), p(4, 4), p(4, 0)]);
}

#[test]
fn production_routing_uses_default_for_two_distant_auto_trees() {
    use crate::resolve::tests::{fact, instance, problem_of};
    use rnl_runtime::problem::{Axis, FactKind, RouteIntent};
    let coords = [(0.0, 0.0), (4.0, 3.0), (100.0, 100.0), (104.0, 103.0)];
    let mut facts = Vec::new();
    for (object, &(x, y)) in coords.iter().enumerate() {
        facts.push(fact(FactKind::Place { at: PointRef::Origin(object), axis: Axis::X, value: x }));
        facts.push(fact(FactKind::Place { at: PointRef::Origin(object), axis: Axis::Y, value: y }));
    }
    let mut problem = problem_of((0..4).map(|_| instance()).collect(), facts);
    problem.version = "0.4".into();
    problem.symbols[0].bbox = [0.0, 0.0, 0.0, 0.0];
    problem.symbols[0].pins.truncate(1);
    problem.symbols[0].pins[0].pos = (0.0, 0.0);
    problem.symbols[0].pins[0].dir = None;
    problem.routes = [("n", 0, 1), ("m", 2, 3)]
        .into_iter()
        .map(|(net, a, b)| RouteIntent {
            tree: true,
            net: net.into(),
            mode: "auto".into(),
            style: vec![],
            wires: vec![],
            pins: [a, b].into_iter().map(|object| PointRef::Pin { object, pin: 0, terminal: "g".into() }).collect(),
        })
        .collect();
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve(&problem, &options);
    let mut work = Work::default();
    let structure = crate::structure::analyse(&problem, &resolved, &options, crate::options::CAPS, &mut work);
    let mut geometry = Geometry::new(4);
    geometry.origin = vec![p(0, 0), p(4, 3), p(100, 100), p(104, 103)];
    let origins = structure.origins(&problem, &geometry);
    let result = route(&problem, &structure, &geometry, &origins, &options, &mut work);
    assert_eq!(result.paths.len(), 2, "{result:?}");
    for (index, expected) in
        [vec![p(0, 0), p(4, 0), p(4, 3)], vec![p(100, 100), p(104, 100), p(104, 103)]].iter().enumerate()
    {
        let mut path = result.paths[index].clone();
        if path.first() != expected.first() {
            path.reverse();
        }
        assert_eq!(&path, expected, "tree {index}");
    }
}

#[test]
fn auto_written_path_reports_unrepresentable_exact_geometry() {
    use crate::resolve::tests::{point, problem_of};
    use rnl_runtime::problem::{ProblemWire, Reference};
    let mut problem = problem_of(vec![point(), point()], vec![]);
    problem.wires = vec![ProblemWire {
        label: "%w".into(),
        reference: Reference::Name("%w".into()),
        nets: vec![],
        bus: false,
        anchors: vec![PointRef::Origin(0), PointRef::Origin(1)],
        mode: "auto".into(),
        default_auto: true,
        generated: false,
    }];
    let mut geometry = Geometry::new(2);
    geometry.origin[1] = [crate::rational::parse_decimal("1e-400").unwrap(), int(1)];
    assert_eq!(written_path(&problem, &geometry, 0), Err(WrittenPathError { wire: 0 }));
    problem.routes.push(rnl_runtime::problem::RouteIntent {
        tree: true,
        net: "other".into(),
        mode: "auto".into(),
        style: vec![],
        wires: vec![],
        pins: vec![PointRef::Origin(0), PointRef::Origin(1)],
    });
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve(&problem, &options);
    let mut work = Work::default();
    let structure = crate::structure::analyse(&problem, &resolved, &options, crate::options::CAPS, &mut work);
    let origins = structure.origins(&problem, &geometry);
    let routing = route(&problem, &structure, &geometry, &origins, &options, &mut work);
    assert_eq!(routing.written_path_error, Some(0));
    assert!(routing.wires.is_empty());
    assert!(crate::score::drawn(&problem, &structure, &geometry, &routing).is_err());
    // Missing source objects are a distinct, non-error outcome.
    problem.wires[0].anchors.push(PointRef::Origin(5));
    assert_eq!(written_path(&problem, &geometry, 0), Ok(None));
}

#[test]
fn an_existing_dot_reuses_the_point_anchor_without_a_branch_or_zero_wire() {
    let mut net = net(vec![anchor(0, 0, None), anchor(-4, 0, None), anchor(4, 0, None), anchor(0, 4, None)]);
    net.anchors[0].connects_pin = false;
    net.anchors[0].existing_junction = true;
    net.joined = vec![(0, 1), (0, 2)];
    let mut l = layout(&[], vec![net]);
    l.modern = true;
    let grid = Grid::new(&l).unwrap();
    let nodes: Vec<Option<usize>> = l.nets[0].anchors.iter().map(|a| grid.lat.at(&a.at)).collect();
    let mut made = vec![Made { start: End::Anchor(3), end: End::Anchor(0), nodes: vec![] }];
    let mut branches = Vec::new();
    grid.junctions_at_anchors(&l.nets[0], &nodes, &mut made, &mut branches);
    assert!(branches.is_empty());
    assert_eq!(made.len(), 1);
    assert_eq!((made[0].start, made[0].end), (End::Anchor(3), End::Anchor(0)));

    // The same topology without an existing dot retains the generated junction.
    l.nets[0].anchors[0].existing_junction = false;
    let grid = Grid::new(&l).unwrap();
    let mut made = vec![Made { start: End::Anchor(3), end: End::Anchor(0), nodes: vec![] }];
    let mut branches = Vec::new();
    grid.junctions_at_anchors(&l.nets[0], &nodes, &mut made, &mut branches);
    assert_eq!(branches, vec![nodes[0].unwrap()]);
    assert_eq!(made.len(), 2);
    assert_eq!((made[1].start, made[1].end), (End::Branch(0), End::Anchor(0)));

    // The new flag must not change released-version routing.
    l.modern = false;
    l.nets[0].anchors[0].existing_junction = true;
    let grid = Grid::new(&l).unwrap();
    let mut made = vec![Made { start: End::Anchor(3), end: End::Anchor(0), nodes: vec![] }];
    let mut branches = Vec::new();
    grid.junctions_at_anchors(&l.nets[0], &nodes, &mut made, &mut branches);
    assert_eq!(branches, vec![nodes[0].unwrap()]);
}
