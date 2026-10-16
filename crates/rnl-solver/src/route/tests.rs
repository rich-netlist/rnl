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
