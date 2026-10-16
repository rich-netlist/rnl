//! Tests of the router on layouts given as plain geometry; `driver::tests` routes the cases of the
//! files.

use super::*;

fn p(x: i64, y: i64) -> [Q; 2] {
    [int(x), int(y)]
}

fn anchor(x: i64, y: i64, exit: Option<(i64, i64)>) -> AnchorIn {
    AnchorIn { at: p(x, y), exit, owner: None }
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
    }
}

fn net(anchors: Vec<AnchorIn>) -> NetIn {
    NetIn { anchors, joined: Vec::new() }
}

/// What every routed net must be: wires along the axes, through no box (but the one a pin of the
/// wire is inside), clear of the widened boxes but where they leave a pin, from an anchor or a
/// branch point to another, every branch point shared by three wires or more, and the parts as
/// said; and the same twice.
fn check(l: &Layout, budget: u64) -> Vec<NetOut> {
    let (outs, _) = route_layout(l, budget);
    assert_eq!(route_layout(l, budget).0, outs, "not deterministic");
    for (n, out) in l.nets.iter().zip(&outs) {
        let end_at = |e: End| match e {
            End::Anchor(a) => n.anchors[a].at.clone(),
            End::Branch(b) => out.branches[b].clone(),
        };
        let mut ends: Vec<usize> = vec![0; out.branches.len()];
        for w in &out.wires {
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
        assert!(ends.iter().all(|e| *e >= 3), "a branch point not shared by three wires: {ends:?}");
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
    // of it (13.1)
    assert_eq!(out.wires.len(), 1, "{out:?}");
    assert!(out.branches.is_empty());
    let mut ends = [out.wires[0].start, out.wires[0].end];
    ends.sort_by_key(|e| format!("{e:?}"));
    assert!(matches!(ends, [End::Anchor(0) | End::Anchor(1), End::Anchor(2)]), "{out:?}");
    assert_eq!(out.parts, 1);
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
    let a = AnchorIn { at: [half(1), half(-3)], exit: EAST, owner: None };
    let b = AnchorIn { at: [half(13), half(5)], exit: WEST, owner: None };
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
