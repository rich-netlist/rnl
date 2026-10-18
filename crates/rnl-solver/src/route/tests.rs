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

/// A layout of symbols and nets given in full: each symbol a box, by its object, and each anchor a
/// pin of the symbol it names.
fn symbols(boxes: &[(usize, [i64; 4])], nets: Vec<NetIn>, written: Vec<Vec<[Q; 2]>>) -> Layout {
    let pins = nets.iter().flat_map(|n| n.anchors.iter().map(|a| a.at.clone())).collect();
    Layout { boxes: boxes.iter().map(|(k, b)| (*k, boxed(*b))).collect(), pins, written, nets }
}

fn pin(x: i64, y: i64, exit: Option<(i64, i64)>, owner: usize) -> AnchorIn {
    AnchorIn { at: p(x, y), exit, owner: Some(owner) }
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

// Configured routing is deliberately tested separately from `check`: its soft and final phases
// may cross obstacles, but may never leave a ROUTE disconnected or connect by coordinate alone.
fn soft_context(l: &Layout) -> SoftContext {
    let nets: Vec<String> = (0..l.nets.len()).map(|index| format!("net{index}")).collect();
    let pins = l
        .nets
        .iter()
        .enumerate()
        .flat_map(|(index, net)| {
            let name = nets[index].clone();
            net.anchors.iter().map(move |_| [name.clone()].into_iter().collect())
        })
        .collect();
    SoftContext { nets, pins, written: vec![BTreeSet::new(); l.written.len()] }
}

fn completed(l: &Layout, hard: u64, soft: u64) -> (Vec<NetOut>, u64, Vec<PhaseUse>) {
    let context = soft_context(l);
    let result = configured_layout(l, hard, soft, &int(1), &RoutingWeights::default(), &context);
    let again = configured_layout(l, hard, soft, &int(1), &RoutingWeights::default(), &context);
    assert_eq!(result, again, "configured routing must be deterministic");
    for (net, out) in l.nets.iter().zip(&result.0) {
        assert_eq!(out.parts, usize::from(!net.anchors.is_empty()));
        assert_eq!(out.failure, None);
        let mut components =
            Components { roots: (0..net.anchors.len() + out.branches.len()).collect(), anchors: net.anchors.len() };
        for &(a, b) in &net.joined {
            components.join(a, b);
        }
        for wire in &out.wires {
            assert!(wire.vertices.len() >= 2);
            assert_eq!(wire.vertices[0], end_at(net, out, wire.start));
            assert_eq!(wire.vertices.last(), Some(&end_at(net, out, wire.end)));
            let (a, b) = (components.index(wire.start), components.index(wire.end));
            assert_ne!(components.root(a), components.root(b), "generated wires must not introduce a cycle: {out:?}");
            components.join(a, b);
        }
        assert_eq!(components.count(), usize::from(!net.anchors.is_empty()));
    }
    result
}

#[test]
fn later_phases_preserve_every_successful_legacy_hard_output() {
    let cases = [
        layout(&[], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST), anchor(5, -6, NORTH)])]),
        layout(&[[4, -3, 6, 3]], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST)])]),
        layout(&[], vec![net(vec![anchor(2, 3, NORTH), anchor(2, 3, WEST)])]),
        layout(
            &[],
            vec![
                net(vec![anchor(0, 0, EAST), anchor(20, 0, WEST)]),
                net(vec![anchor(10, -4, NORTH), anchor(10, 4, Some((0, -1)))]),
            ],
        ),
    ];
    for l in cases {
        let legacy = route_layout(&l, 100_000);
        assert!(legacy.0.iter().all(|out| out.failure.is_none()));
        let context = soft_context(&l);
        let (outs, expansions, phases) =
            phased_layout(&l, 100_000, 0, &int(1), &RoutingWeights::default(), &context, None);
        assert_eq!((outs, expansions), legacy, "A02 protects NetOut, branch order, paths, and work");
        assert!(phases.is_empty());
    }
}

#[test]
fn configured_hard_search_reverses_short_many_turns_and_long_few_turns() {
    let l = layout(&[[2, -2, 4, 10], [6, -10, 8, 2]], vec![net(vec![anchor(0, 0, None), anchor(10, 0, None)])]);
    let context = soft_context(&l);
    let length_weights = RoutingWeights { wire_length: int(1), ..RoutingWeights::default() };
    // Smaller than an f64 can preserve: the configured hard decision must retain its exact,
    // nonzero priority over the raw-length tie, rather than round this objective away.
    let turns_weights =
        RoutingWeights { wire_turns: Q::new(BigInt::one(), BigInt::one() << 1100usize), ..RoutingWeights::default() };
    let short = configured_layout(&l, 100_000, 0, &int(1), &length_weights, &context);
    let few_turns = configured_layout(&l, 100_000, 0, &int(1), &turns_weights, &context);
    assert!(short.2.is_empty() && few_turns.2.is_empty(), "both decisions must occur in the production hard phase");
    let length = |out: &NetOut| {
        out.wires.iter().flat_map(|wire| wire.vertices.windows(2)).map(|pair| distance(&pair[0], &pair[1])).sum::<Q>()
    };
    let turns = |out: &NetOut| out.wires.iter().map(|wire| wire.vertices.len().saturating_sub(2)).sum::<usize>();
    assert!(length(&short.0[0]) < length(&few_turns.0[0]), "{short:?}\n{few_turns:?}");
    assert!(turns(&short.0[0]) > turns(&few_turns.0[0]), "{short:?}\n{few_turns:?}");
    assert_eq!(configured_layout(&l, 100_000, 0, &int(1), &turns_weights, &context), few_turns);
}

#[test]
fn earlier_soft_and_fallback_output_changes_later_hard_search_in_historical_net_order() {
    let first = net(vec![anchor(0, 0, None), anchor(6, 0, None)]);
    let later = net(vec![anchor(3, -5, None), anchor(3, 5, None)]);
    let l = layout(&[[-1, -1, 1, 1]], vec![first.clone(), later.clone()]);
    let weights = RoutingWeights { wire_length: int(1), wire_crossings: int(100), ..RoutingWeights::default() };
    let alone = layout(&[[-1, -1, 1, 1]], vec![later.clone()]);
    let baseline = configured_layout(&alone, 100_000, 0, &int(1), &weights, &soft_context(&alone));
    assert_eq!(baseline.0[0].wires[0].vertices, vec![p(3, -5), p(3, 5)]);
    for soft_budget in [0, 10_000] {
        let routed = configured_layout(&l, 100_000, soft_budget, &int(1), &weights, &soft_context(&l));
        assert_eq!(routed.0[0].wires[0].vertices, vec![p(0, 0), p(6, 0)]);
        assert!(routed
            .2
            .iter()
            .any(|phase| phase.net == 0 && phase.phase == if soft_budget == 0 { "fallback" } else { "soft" }));
        assert!(!routed.2.iter().any(|phase| phase.net == 1), "the later net should still finish in hard routing");
        assert_ne!(
            routed.0[1].wires, baseline.0[0].wires,
            "earlier soft/fallback geometry must affect later hard routing"
        );
        assert!(routed.0[1].wires.iter().all(|wire| wire.vertices.windows(2).all(|pair| !crosses(
            &pair[0],
            &pair[1],
            &p(0, 0),
            &p(6, 0)
        ))));
        let reversed = layout(&[[-1, -1, 1, 1]], vec![later.clone(), first.clone()]);
        let reordered = configured_layout(&reversed, 100_000, soft_budget, &int(1), &weights, &soft_context(&reversed));
        assert_eq!(routed.0[0], reordered.0[1]);
        assert_eq!(
            routed.0[1], reordered.0[0],
            "half-perimeter order, not problem storage order, schedules all three phases"
        );
        assert_eq!(routed.1, reordered.1);
    }
}

#[test]
fn fractional_fallback_segments_are_visible_to_later_hard_edges() {
    let l = layout(&[], vec![net(vec![anchor(0, 0, None), anchor(4, 4, None)])]);
    let mut grid = Grid::new(&l).unwrap();
    let half = crate::rational::frac(1, 2);
    grid.mark(&[[int(0), half.clone()], [int(4), half.clone()]]);
    let interior = grid.lat.at(&p(2, 0)).unwrap();
    assert_eq!(grid.edge_flags(interior, 1) & CROSSED, CROSSED);
    let endpoint = grid.lat.at(&p(0, 0)).unwrap();
    assert_eq!(grid.edge_flags(endpoint, 1) & TAKEN, TAKEN, "touching an off-grid endpoint is not a proper crossing");
    grid.mark(&[[half.clone(), int(2)], [int(2), int(2)]]);
    let occupied = grid.lat.at(&p(0, 2)).unwrap();
    assert_eq!(grid.edge_flags(occupied, 0) & TAKEN, TAKEN, "partial collinear occupancy blocks the whole hard edge");
}

#[test]
fn a_pin_walled_in_is_connected_by_the_independently_budgeted_soft_search() {
    let walls = [[-5, 1, 5, 5], [-5, -5, 5, -1], [-5, -5, -1, 5], [1, -5, 5, 5]];
    let l = layout(&walls, vec![net(vec![anchor(0, 0, None), anchor(12, 0, WEST)])]);
    let (outs, expansions, phases) = completed(&l, 0, 20_000);
    assert!(!outs[0].wires.is_empty());
    assert!(expansions > 0, "hard budget zero must not consume soft's budget");
    assert_eq!(phases.iter().map(|phase| phase.phase).collect::<Vec<_>>(), vec!["soft"]);
    assert!(outs[0]
        .wires
        .iter()
        .flat_map(|wire| wire.vertices.windows(2))
        .any(|pair| l.boxes.iter().any(|(_, box_)| meets_inside(box_, &pair[0], &pair[1]))));
}

#[test]
fn no_search_budget_still_connects_distinct_coincident_pins_and_preserves_authored_cycles() {
    let mut n =
        net(vec![anchor(0, 0, None), anchor(4, 0, None), anchor(4, 4, None), anchor(0, 0, None), anchor(9, 3, EAST)]);
    n.joined = vec![(0, 1), (1, 2), (2, 0)];
    let mut l = layout(&[], vec![n]);
    l.written = vec![vec![p(0, 0), p(4, 0), p(4, 4), p(0, 0)]];
    let written = l.written.clone();
    let (outs, expansions, phases) = completed(&l, 0, 0);
    assert_eq!(expansions, 0);
    assert_eq!(l.written, written);
    assert_eq!(phases.iter().map(|phase| phase.phase).collect::<Vec<_>>(), vec!["fallback"]);
    assert!(
        outs[0].wires.iter().any(|wire| wire.vertices == vec![p(0, 0), p(0, 0)]),
        "coincident distinct anchors need a real zero length link"
    );
}

#[test]
fn hard_coincident_distinct_pins_inside_a_third_body_keep_their_zero_length_identity_edge() {
    let mut l = layout(&[[1, 1, 5, 5]], vec![net(vec![anchor(3, 3, None), anchor(3, 3, None)])]);
    l.nets[0].anchors[0].owner = Some(0);
    l.nets[0].anchors[1].owner = Some(1);
    assert!(inside(&l.boxes[0].1, &l.nets[0].anchors[0].at));
    assert!(l.nets[0].anchors.iter().all(|a| a.owner != Some(l.boxes[0].0)));
    let (outs, _, phases) = completed(&l, 12, 0);
    assert!(phases.is_empty(), "the coincident identity edge succeeds in hard routing");
    assert_eq!(outs[0].wires.len(), 1);
    let wire = &outs[0].wires[0];
    let (End::Anchor(a), End::Anchor(b)) = (wire.start, wire.end) else {
        panic!("both distinct physical anchors must remain explicit: {wire:?}");
    };
    let mut identities = [a, b];
    identities.sort();
    assert_eq!(identities, [0, 1]);
    assert_eq!(wire.vertices, vec![p(3, 3), p(3, 3)]);
    assert_eq!(outs[0].parts, 1);
    assert_eq!(outs[0].failure, None);
}

#[test]
fn an_unrepresentable_lattice_uses_exact_geometry_fallback() {
    let fine = Q::new(1.into(), (MAX_UNIT + 1).into());
    let mut l = layout(&[], vec![net(vec![anchor(0, 0, NORTH), anchor(8, 4, WEST), anchor(20, 1, None)])]);
    l.nets[0].anchors[0].at[0] = fine.clone();
    l.pins[0][0] = fine;
    let (outs, expansions, phases) = completed(&l, 100, 100);
    assert_eq!(expansions, 0);
    assert_eq!(phases, vec![PhaseUse { net: 0, phase: "fallback", failure: Failure::Lattice }]);
    assert!(!outs[0].wires.is_empty());
}

#[test]
fn fallback_branches_only_on_generated_segments_and_counts_only_real_identity_connections() {
    let l =
        layout(&[], vec![net(vec![anchor(0, 0, None), anchor(10, 0, None), anchor(5, 6, None), anchor(10, 10, None)])]);
    let (outs, _, _) = completed(&l, 0, 0);
    assert!(!outs[0].branches.is_empty(), "nearest tree attachment should split a generated segment");
    let mut authored = l.clone();
    authored.nets[0].joined = vec![(0, 1)];
    authored.written = vec![vec![p(0, 0), p(10, 0)]];
    let (outs, _, _) = completed(&authored, 0, 0);
    assert!(!outs[0].branches.contains(&p(5, 0)), "an author's segment has no implicit interior connection point");
}

#[test]
fn soft_cost_counts_identity_violations_once_and_disabled_items_add_nothing() {
    let l = layout(
        &[[1, -1, 4, 1]],
        vec![net(vec![anchor(0, 0, None), anchor(8, 0, None)]), net(vec![anchor(2, 0, None)])],
    );
    let context = soft_context(&l);
    let weights = RoutingWeights { wire_overlaps: int(3), wire_shorts: int(5), ..RoutingWeights::default() };
    let zero = Q::zero();
    let search =
        SoftSearch { layout: &l, context: &context, net: 0, clearance: &zero, weights: &weights, wires: Vec::new() };
    let initial = SoftBest { cost: SoftCost::default(), seen: BTreeSet::new(), previous: None, source: 0 };
    let entered = search.edge(&p(0, 0), &p(3, 0), &initial);
    assert_eq!(entered.cost.weighted, int(8));
    let left = search.edge(&p(3, 0), &p(6, 0), &entered);
    let reentered = search.edge(&p(6, 0), &p(2, 0), &left);
    assert_eq!(
        reentered.cost.weighted,
        int(8),
        "a second visit to the same box and physical pin is not a second violation"
    );
    let weights = RoutingWeights::default();
    let search = SoftSearch { weights: &weights, ..search };
    assert_eq!(search.edge(&p(0, 0), &p(3, 0), &initial).cost.weighted, Q::zero());
}

#[test]
fn fallback_keeps_pin_exits_when_possible_and_truncates_impossible_close_leads() {
    for (a, b) in [(EAST, EAST), (WEST, EAST), (NORTH, NORTH), (EAST, NORTH)] {
        let n = net(vec![anchor(0, 0, a), anchor(6, 0, b)]);
        let ends = attachments(&n, &NetOut::default());
        let path = fallback_path(&n, &ends[0], &ends[1]);
        let step = |a: &[Q; 2], b: &[Q; 2]| ((&b[0] - &a[0]).signum(), (&b[1] - &a[1]).signum());
        let source_exit = a.unwrap();
        let target_exit = b.unwrap();
        assert_eq!(step(&path[0], &path[1]), (int(source_exit.0), int(source_exit.1)), "{path:?}");
        let last = path.len() - 1;
        assert_eq!(step(&path[last], &path[last - 1]), (int(target_exit.0), int(target_exit.1)), "{path:?}");
        assert_eq!(erase_loops(path.clone()), path);
    }
    // Both eastward one-unit leads overlap for pins half a unit apart. A legal connection
    // terminates on the other pin before completing that impossible pair of unit leads.
    for exits in [(EAST, EAST), (EAST, WEST), (WEST, EAST)] {
        let mut l = layout(&[], vec![net(vec![anchor(0, 0, exits.0), anchor(1, 0, exits.1)])]);
        l.nets[0].anchors[1].at[0] = crate::rational::frac(1, 2);
        l.pins[1] = l.nets[0].anchors[1].at.clone();
        let (outs, _, _) = completed(&l, 0, 0);
        for wire in &outs[0].wires {
            assert_eq!(erase_loops(wire.vertices.clone()), wire.vertices);
        }
    }
}

fn configured_hard_reference(l: &Layout, weights: &RoutingWeights, rational: bool) -> (Vec<NetOut>, u64) {
    let mut grid = Grid::new(l).unwrap();
    grid.hard_weights = Some(weights.clone());
    grid.rational_hard = rational;
    let mut expansions = 0;
    let mut outs = vec![NetOut::default(); l.nets.len()];
    for i in net_order(l) {
        outs[i] = grid.route_net(i, 200_000, &mut expansions);
        release_kept(&mut grid, i);
    }
    (outs, expansions)
}

#[test]
fn integer_scaled_hard_costs_match_exact_rationals_in_paths_and_work() {
    let mut fractional =
        layout(&[[2, -2, 4, 10], [6, -10, 8, 2]], vec![net(vec![anchor(0, 0, EAST), anchor(10, 0, WEST)])]);
    for a in &mut fractional.nets[0].anchors {
        a.at[0] += crate::rational::frac(1, 2);
    }
    fractional.pins = fractional.nets[0].anchors.iter().map(|a| a.at.clone()).collect();
    let crossing = layout(
        &[],
        vec![net(vec![anchor(0, 3, EAST), anchor(12, 3, WEST)]), net(vec![anchor(6, -5, NORTH), anchor(6, 11, None)])],
    );
    let weights = [
        RoutingWeights::default(),
        SolverOptions::without_time_limit().routing_weights(),
        RoutingWeights {
            wire_turns: Q::new(BigInt::one(), BigInt::one() << 1100usize),
            wire_length: crate::rational::frac(5, 11),
            wire_crossings: crate::rational::frac(2, 7),
            ..Default::default()
        },
    ];
    for l in [fractional, crossing] {
        for weights in &weights {
            assert_eq!(
                configured_hard_reference(&l, weights, false),
                configured_hard_reference(&l, weights, true),
                "wire geometry, branch order and exact expansion count must be identical"
            );
        }
    }
}

/// A test-only copy of the production input assembly, shared by fixed-frame diagnostics.
fn fixed_record_layout(file: &str, title: &str) -> (Layout, SoftContext, Vec<Job>, Vec<Vec<Point>>) {
    struct Files;
    impl rnl_runtime::source::Provider for Files {
        fn read(&self, path: &str) -> Option<String> {
            std::fs::read_to_string(path).ok()
        }
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/legacy-layout")
        .join(file)
        .to_string_lossy()
        .replace('\\', "/");
    let without = rnl_view::interpret(&path, None, &Files, &rnl_runtime::Options::solver()).unwrap();
    let with = rnl_view::interpret(&path, None, &Files, &rnl_runtime::Options { problems: true, ..Default::default() })
        .unwrap();
    let problem = without.problems.iter().find(|p| p.title == title).unwrap();
    let previous = rnl_runtime::previous::previous(problem, &with).unwrap();
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve(problem, &options);
    let s = crate::structure::analyse(problem, &resolved, &options, crate::options::CAPS, &mut Work::default());
    let mut g = resolved.witness.geometry.clone();
    for pl in previous.placements {
        if let Some(x) = pl.x {
            g.origin[pl.object][0] = crate::rational::from_f64(x).unwrap();
        }
        if let Some(y) = pl.y {
            g.origin[pl.object][1] = crate::rational::from_f64(y).unwrap();
        }
        if let Some(o) = pl.orient {
            g.orient[pl.object] = o;
        }
    }
    let origins_storage = s.origins(problem, &g);
    let origins = origins_storage.as_slice();
    let jobs = jobs(problem);
    let geometry =
        RoutingGeometry { wires: (0..problem.wires.len()).map(|i| authored_wire(problem, &g, i).unwrap()).collect() };
    let mut layout = Layout::default();
    let mut context = SoftContext { nets: jobs.iter().map(|job| job.net.clone()).collect(), ..SoftContext::default() };
    for k in s.components() {
        if let Some(b) = s.box_at(origins, &g.orient, k) {
            layout.boxes.push((k, b));
        }
    }
    // the pins of what is drawn: a symbol that the solution leaves free is not
    for k in (0..problem.objects.len()).filter(|&k| !s.objects[k].left_free) {
        for pin in 0..s.pins(k).len() {
            if let Some(at) = s.pin_at(origins, &g.orient, k, pin) {
                layout.pins.push(at);
                context.pins.push(
                    s.connections
                        .nets_of
                        .get(&(k, pin))
                        .into_iter()
                        .flatten()
                        .filter_map(|&net| s.connections.nets.get(net))
                        .map(|net| net.name.clone())
                        .collect(),
                );
            }
        }
    }
    for wire in &geometry.wires {
        layout.written.push(wire.vertices.iter().map(|vertex| vertex.at.clone()).collect());
        context.written.push(wire.nets.clone());
    }
    let mut points: Vec<Vec<Point>> = Vec::new();
    for job in &jobs {
        let mut net = NetIn::default();
        let mut pts: Vec<Point> = Vec::new();
        let mut add = |p: Point, net: &mut NetIn| -> Option<usize> {
            if let Some(i) = pts.iter().position(|q| q.key == p.key) {
                return Some(i);
            }
            let at = [point_value(problem, &g, &p.reference, 0, 0)?, point_value(problem, &g, &p.reference, 1, 0)?];
            let exit = match p.key {
                (k, Some(pin)) => s.exit(k, pin, g.orient[k]),
                _ => None,
            };
            let owner = p.key.1.map(|_| p.key.0);
            net.anchors.push(AnchorIn { at, exit, owner });
            pts.push(p);
            Some(pts.len() - 1)
        };
        for p in &job.points {
            add(p.clone(), &mut net);
        }
        for &w in &job.wires {
            let Some(wire) = problem.wires.get(w) else { continue };
            let ends: Vec<usize> =
                wire.anchors.iter().filter_map(|r| wire_point(problem, r)).filter_map(|p| add(p, &mut net)).collect();
            for pair in ends.windows(2) {
                net.joined.push((pair[0], pair[1]));
            }
        }
        layout.nets.push(net);
        points.push(pts);
    }
    (layout, context, jobs, points)
}

#[test]
fn fixed_t3_configured_and_legacy_exits_stay_outside_own_symbol_interior() {
    let (l, _context, jobs, points) = fixed_record_layout("T3-ota5t.sp", "T3-ota5t.sp");
    for (label, (outs, expanded)) in [
        ("legacy", route_layout(&l, 200_000)),
        ("configured", configured_hard_reference(&l, &SolverOptions::without_time_limit().routing_weights(), false)),
    ] {
        let mut violations = Vec::new();
        for (i, out) in outs.iter().enumerate() {
            assert!(out.failure.is_none(), "{label} &{}: {:?}", jobs[i].net, out.failure);
            for wire in &out.wires {
                for (end, reverse) in [(wire.start, false), (wire.end, true)] {
                    let End::Anchor(a) = end else { continue };
                    let anchor = &l.nets[i].anchors[a];
                    let Some(exit) = anchor.exit else { continue };
                    let path: Vec<_> =
                        if reverse { wire.vertices.iter().rev().collect() } else { wire.vertices.iter().collect() };
                    let Some(next) = path.iter().find(|p| ***p != anchor.at) else { continue };
                    let step = ((&next[0] - &anchor.at[0]).signum(), (&next[1] - &anchor.at[1]).signum());
                    if step == (int(exit.0), int(exit.1)) {
                        continue;
                    }
                    violations.push(points[i][a].key);
                    let owner = anchor.owner.unwrap();
                    let body = &l.boxes.iter().find(|(k, _)| *k == owner).unwrap().1;
                    let within = inside(body, &anchor.at);
                    let crosses_body = meets_inside(body, &anchor.at, next);
                    eprintln!("{label} &{} anchor={:?} exit={exit:?} start={:?} next={next:?} body={body:?} inside={within} through={crosses_body}",jobs[i].net,points[i][a].reference,anchor.at);
                    assert!(
                        !within && !crosses_body && runs_along(body, &anchor.at, next),
                        "the declared Step1 exception must run along the body's edge, never through it"
                    );
                }
            }
        }
        violations.sort_unstable();
        let expected = if label == "legacy" { Vec::new() } else { vec![(0, Some(0)), (2, Some(0))] };
        assert_eq!(violations, expected);
        eprintln!("{label}: expanded={expanded} exit_hint_violations={violations:?}");
    }
}

#[test]
#[ignore = "controlled serialized timing; run without other native workload"]
fn fixed_t6_exact_cost_representation_profile() {
    let (l, _, _, _) = fixed_record_layout("T6-r2r-ladder.sp", "T6-r2r-ladder.sp");
    let weights = SolverOptions::without_time_limit().routing_weights();
    let started = std::time::Instant::now();
    let rational = configured_hard_reference(&l, &weights, true);
    let rational_seconds = started.elapsed().as_secs_f64();
    let started = std::time::Instant::now();
    let integer = configured_hard_reference(&l, &weights, false);
    let integer_seconds = started.elapsed().as_secs_f64();
    assert_eq!(integer, rational, "fixed same-layout routing output and work cannot change");
    eprintln!("exact-cost-profile rational_seconds={rational_seconds:.6} integer_seconds={integer_seconds:.6} expansions={} ratio={:.3}",integer.1,rational_seconds/integer_seconds);
}

/// This fixed OTA sublevel leaves two source corners outwards. Keep the physical identity and
/// geometry distinct from the main level's positive-length along-edge approximation.
#[test]
fn fixed_t3_ota_corner_exits_preserve_identities_and_stay_outside_bodies() {
    let (l, _, jobs, points) = fixed_record_layout("T3-ota5t.sp", "ota5t");
    for (label, (outs, expanded)) in [
        ("legacy", route_layout(&l, 200_000)),
        ("configured", configured_hard_reference(&l, &SolverOptions::without_time_limit().routing_weights(), false)),
    ] {
        let mut violations = Vec::new();
        for (i, out) in outs.iter().enumerate() {
            assert!(out.failure.is_none(), "{label} &{}: {:?}", jobs[i].net, out.failure);
            for wire in &out.wires {
                for (end, reverse) in [(wire.start, false), (wire.end, true)] {
                    let End::Anchor(a) = end else { continue };
                    let anchor = &l.nets[i].anchors[a];
                    let path: Vec<_> =
                        if reverse { wire.vertices.iter().rev().collect() } else { wire.vertices.iter().collect() };
                    let next = path.iter().find(|p| ***p != anchor.at);
                    let body = anchor.owner.and_then(|owner| l.boxes.iter().find(|(k, _)| *k == owner).map(|(_, b)| b));
                    let follows = match (next, anchor.exit) {
                        (Some(next), Some(exit)) => {
                            ((&next[0] - &anchor.at[0]).signum(), (&next[1] - &anchor.at[1]).signum())
                                == (int(exit.0), int(exit.1))
                        }
                        _ => true,
                    };
                    let within = body.is_some_and(|body| inside(body, &anchor.at));
                    let through = body.zip(next).is_some_and(|(body, next)| meets_inside(body, &anchor.at, next));
                    let along = body.zip(next).is_some_and(|(body, next)| runs_along(body, &anchor.at, next));
                    eprintln!("ota5t {label} &{} key={:?} reference={:?} exit={:?} start={:?} next={next:?} body={body:?} follows={follows} inside={within} through={through} along={along}",jobs[i].net,points[i][a].key,points[i][a].reference,anchor.exit,anchor.at);
                    if !follows {
                        violations.push(points[i][a].key);
                        assert_eq!(jobs[i].net, "tail");
                        assert!(
                            matches!(&points[i][a].reference,PointRef::Pin{object:0|1,pin:2,terminal} if terminal=="s")
                        );
                        let body = body.expect("physical source has its own symbol box");
                        let next = next.unwrap();
                        let corner = (anchor.at[0] == body[0] || anchor.at[0] == body[2])
                            && (anchor.at[1] == body[1] || anchor.at[1] == body[3]);
                        let outward = anchor.at[1] == next[1]
                            && ((anchor.at[0] == body[0] && next[0] < anchor.at[0])
                                || (anchor.at[0] == body[2] && next[0] > anchor.at[0]));
                        assert!(corner&&outward&&!within&&!through,"source departure must only touch its own corner, then stay strictly outside the closed box");
                        assert!(
                            !along,
                            "a corner-outward segment is distinct from a positive-length along-edge departure"
                        );
                    }
                }
            }
        }
        violations.sort_unstable();
        let expected = if label == "legacy" { Vec::new() } else { vec![(0, Some(2)), (1, Some(2))] };
        assert_eq!(violations, expected, "exact physical source endpoints; duplicates are not discarded");
        eprintln!("ota5t {label}: expanded={expanded} exit_hint_violations={violations:?}");
    }
}
