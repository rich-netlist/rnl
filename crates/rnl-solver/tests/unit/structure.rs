use super::*;
use crate::resolve::tests::{fact, instance, point, problem_of};
use rnl_runtime::problem::ProblemGroup;

fn structure_of(p: &LevelProblem) -> Structure {
    let options = SolverOptions::without_time_limit();
    let r = crate::resolve(p, &options);
    analyse(p, &r, &options, crate::options::CAPS, &mut Work::default())
}

#[test]
fn groups_nest_and_overlapping_groups_split_in_three() {
    let mut p = problem_of((0..6).map(|_| instance()).collect(), Vec::new());
    let group = |label: &str, members: Vec<usize>| ProblemGroup { label: label.to_string(), members };
    p.groups =
        vec![group("%a", vec![0, 1, 2]), group("%b", vec![2, 3]), group("%c", vec![0, 1]), group("%d", vec![4, 5])];
    let s = structure_of(&p);
    let nodes = &s.tree.nodes;
    assert_eq!(nodes[0].kind, NodeKind::Root);
    assert_eq!(nodes[0].members, vec![0, 1, 2, 3, 4, 5]);
    // %a and %b overlap: their union, with %a alone, the shared #2, %b alone; %c nests in the
    // first part, %d stands apart
    let union = nodes.iter().position(|n| n.kind == NodeKind::Union(0, 1)).unwrap();
    assert_eq!(nodes[union].members, vec![0, 1, 2, 3]);
    assert!(nodes[union].fixed_order);
    let parts: Vec<&Vec<usize>> = nodes[union].children.iter().map(|&c| &nodes[c].members).collect();
    assert_eq!(parts, vec![&vec![0, 1], &vec![2], &vec![3]]);
    // %c is the same set as the first part: it is not made twice
    assert_eq!(nodes.len(), 6, "{nodes:?}");
    let d = nodes.iter().position(|n| n.kind == NodeKind::Group(3)).unwrap();
    assert_eq!(nodes[0].children, vec![union, d]);
    assert!(nodes[0].own.is_empty());
    let order = s.tree.post_order();
    assert_eq!(*order.last().unwrap(), 0);
    assert!(
        order.iter().position(|&n| n == union)
            > nodes[union].children.iter().map(|&c| order.iter().position(|&n| n == c)).max().unwrap()
    );
    assert_eq!(s.tree.lowest_common(&[0, 1]), nodes[union].children[0]);
    assert_eq!(s.tree.lowest_common(&[0, 4]), 0);
    // the scopes: each group, nothing outside
    assert_eq!(s.scopes.len(), 4);
}

#[test]
fn a_group_that_overlaps_two_others_is_not_nested_and_said_so() {
    let mut p = problem_of((0..5).map(|_| instance()).collect(), Vec::new());
    let group = |label: &str, members: Vec<usize>| ProblemGroup { label: label.to_string(), members };
    p.groups = vec![group("%a", vec![0, 1]), group("%b", vec![2, 3]), group("%c", vec![1, 2])];
    let s = structure_of(&p);
    assert_eq!(s.tree.nodes.len(), 3);
    assert!(s.notes[0].contains("%c"), "{:?}", s.notes);
}

/// The tree of groups over `n` free instances with these groups, `%g0`, `%g1`, …, and the notes.
fn tree_of(n: usize, groups: &[Vec<usize>]) -> (GroupTree, Vec<String>) {
    let mut p = problem_of((0..n).map(|_| instance()).collect(), Vec::new());
    p.groups =
        groups.iter().enumerate().map(|(i, m)| ProblemGroup { label: format!("%g{i}"), members: m.clone() }).collect();
    let s = structure_of(&p);
    (s.tree, s.notes)
}

#[test]
fn a_part_or_a_union_with_the_members_of_a_group_is_that_group() {
    // the first part of the union of %g1 and %g2 is %g0
    let (t, notes) = tree_of(4, &[vec![0, 1], vec![0, 1, 2], vec![2, 3]]);
    let nodes = &t.nodes;
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(nodes.len(), 5, "{nodes:?}");
    let union = nodes.iter().position(|n| n.kind == NodeKind::Union(1, 2)).unwrap();
    assert_eq!(nodes[0].children, vec![union]);
    let parts: Vec<(&NodeKind, &Vec<usize>)> =
        nodes[union].children.iter().map(|&c| (&nodes[c].kind, &nodes[c].members)).collect();
    assert_eq!(
        parts,
        vec![
            (&NodeKind::Group(0), &vec![0, 1]),
            (&NodeKind::Part(1, 2, 1), &vec![2]),
            (&NodeKind::Part(1, 2, 2), &vec![3])
        ]
    );
    // the union of %g2 and %g3 is %g1, inside %g0: %g1 is the union, its parts what only %g2
    // has, what both have, what only %g3 has
    let (t, notes) = tree_of(5, &[vec![0, 1, 2, 3, 4], vec![0, 1, 2, 3], vec![2, 3], vec![0, 1, 2]]);
    let nodes = &t.nodes;
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(nodes.len(), 6, "{nodes:?}");
    let top = nodes.iter().position(|n| n.kind == NodeKind::Group(0)).unwrap();
    let union = nodes.iter().position(|n| n.kind == NodeKind::Union(2, 3)).unwrap();
    assert_eq!((nodes[0].children.clone(), nodes[top].children.clone()), (vec![top], vec![union]));
    assert_eq!(nodes[union].members, vec![0, 1, 2, 3]);
    assert!(nodes[union].fixed_order);
    let parts: Vec<&Vec<usize>> = nodes[union].children.iter().map(|&c| &nodes[c].members).collect();
    assert_eq!(parts, vec![&vec![3], &vec![2], &vec![0, 1]]);
    // a group written after the union that is one of its two groups is in it already; one with
    // the two outer parts and not the shared one cannot be in one piece with them
    let (t, notes) = tree_of(4, &[vec![0, 1, 2], vec![2, 3], vec![0, 1, 2], vec![0, 1, 3]]);
    assert_eq!(t.nodes.len(), 5, "{:?}", t.nodes);
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("%g3"), "{notes:?}");
}

/// A fixed sequence of pseudo-random numbers below `n`, for the tests.
pub(crate) struct Lcg(pub u64);

impl Lcg {
    pub(crate) fn below(&mut self, n: u64) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % n
    }

    /// Sets of groups over `n` elements: two to five, each element in each by a coin.
    pub(crate) fn groups(&mut self, n: usize) -> Vec<Vec<usize>> {
        let count = 2 + self.below(4) as usize;
        (0..count).map(|_| (0..n).filter(|_| self.below(2) == 0).collect()).collect()
    }
}

/// Groups drawn at random: the nodes nest, each set of elements is one node, the children of a
/// node and its own elements are its members, each once, a union's three parts are what only
/// its first group has, what both have and what only the second has, and every group is a
/// node, one of the two groups of a union, or noted.
#[test]
fn every_tree_of_groups_holds_each_set_once_and_splits_a_union_in_three() {
    let mut rng = Lcg(0x2545_f491_4f6c_dd1d);
    let mut unions = 0;
    for _ in 0..500 {
        let groups = rng.groups(6);
        let (t, notes) = tree_of(6, &groups);
        let nodes = &t.nodes;
        let what = format!("{groups:?}: {nodes:?}");
        let set = |n: usize| nodes[n].members.iter().copied().collect::<BTreeSet<usize>>();
        for a in 0..nodes.len() {
            for b in a + 1..nodes.len() {
                let (x, y) = (set(a), set(b));
                assert!(x.is_disjoint(&y) || x.is_subset(&y) || y.is_subset(&x), "{what}");
                assert!(a == 0 || x != y, "{what}");
            }
            let mut m: Vec<usize> = nodes[a].children.iter().flat_map(|&c| nodes[c].members.clone()).collect();
            m.extend(nodes[a].own.iter().copied());
            m.sort_unstable();
            assert_eq!(m, nodes[a].members, "{what}");
            assert!(nodes[a].children.iter().all(|&c| a == 0 || nodes[c].members.len() < nodes[a].members.len()));
            if let NodeKind::Union(h, g) = nodes[a].kind {
                unions += 1;
                let (h, g): (BTreeSet<usize>, BTreeSet<usize>) =
                    (groups[h].iter().copied().collect(), groups[g].iter().copied().collect());
                let parts: Vec<BTreeSet<usize>> = nodes[a].children.iter().map(|&c| set(c)).collect();
                let expected: Vec<BTreeSet<usize>> = vec![
                    h.difference(&g).copied().collect(),
                    h.intersection(&g).copied().collect(),
                    g.difference(&h).copied().collect(),
                ];
                assert_eq!(parts, expected, "{what}");
                assert!(nodes[a].fixed_order && nodes[a].own.is_empty(), "{what}");
            }
        }
        for (i, g) in groups.iter().enumerate() {
            let g: BTreeSet<usize> = g.iter().copied().collect();
            let in_union = nodes.iter().any(|n| match n.kind {
                NodeKind::Union(a, b) => {
                    [a, b].iter().any(|&j| groups[j].iter().copied().collect::<BTreeSet<usize>>() == g)
                }
                _ => false,
            });
            let noted = notes.iter().any(|n| n.contains(&format!("%g{i} ")));
            assert!(g.len() < 2 || (0..nodes.len()).any(|n| set(n) == g) || in_union || noted, "%g{i}: {what}");
        }
    }
    assert!(unions > 50, "{unions}");
}

#[test]
fn rows_come_from_the_facts_on_heights_without_a_supply() {
    let o = |k| PointRef::Origin(k);
    let p = problem_of(
        vec![instance(), instance(), instance(), instance()],
        vec![
            fact(FactKind::Distance {
                axis: Axis::Y,
                sense: rnl_runtime::problem::Sense::Increasing,
                min: Some(5.0),
                max: None,
                from: vec![o(0)],
                to: vec![o(1)],
            }),
            fact(FactKind::Order {
                axis: Axis::X,
                sense: rnl_runtime::problem::Sense::Increasing,
                members: vec![o(2), o(0)],
            }),
            fact(FactKind::Place { at: o(3), axis: Axis::Y, value: -20.0 }),
        ],
    );
    let s = structure_of(&p);
    assert!(s.depth.is_none());
    // 1 above 0, 3 placed below; 2, which the ORDER joins to 0 and nothing on y keeps from
    // it, in the row of 0
    assert_eq!(s.rows, vec![vec![1], vec![0, 2], vec![3]]);
    assert!(s.precedence.contains(&(2, 0)) && !s.precedence.contains(&(0, 2)));
    // every element is its own free class
    assert_eq!(s.open.classes.len(), 4);
    // no wires, no supply: current_direction and the routing objectives are inert
    assert!(s.active.inert.iter().any(|i| i.starts_with("current_direction")));
    assert!(!s.has(Source::DepthRows) && s.has(Source::Barycentre) && s.has(Source::FacingPins));
}

#[test]
fn an_order_is_a_row_where_the_facts_let_it_be_and_an_align_on_x_a_column() {
    let o = |k| PointRef::Origin(k);
    let order = |members: Vec<PointRef>| {
        fact(FactKind::Order { axis: Axis::X, sense: rnl_runtime::problem::Sense::Increasing, members })
    };
    let p = problem_of(
        (0..6).map(|_| instance()).collect(),
        vec![
            // 1 is kept above 0 by 5, so the ORDER cannot hold them in one row; 2 joins 1
            fact(FactKind::Distance {
                axis: Axis::Y,
                sense: rnl_runtime::problem::Sense::Increasing,
                min: Some(5.0),
                max: None,
                from: vec![o(0)],
                to: vec![o(1)],
            }),
            order(vec![o(0), o(1), o(2)]),
            // 3 and 4 in one column: rows of their own, though nothing else tells them apart
            fact(FactKind::Align { axis: Axis::X, members: vec![o(3), o(4)] }),
        ],
    );
    let s = structure_of(&p);
    let row_of = |k: usize| s.rows.iter().position(|r| r.contains(&k)).unwrap();
    assert_eq!(row_of(1), row_of(2), "{:?}", s.rows);
    assert_ne!(row_of(0), row_of(1), "{:?}", s.rows);
    assert_ne!(row_of(3), row_of(4), "{:?}", s.rows);
    // 5, connected to nothing, goes with the most
    assert_eq!(row_of(5), row_of(3).min(row_of(4)), "{:?}", s.rows);
}

#[test]
fn only_the_facts_with_a_side_give_precedences_and_those_left_open_can_be_reversed() {
    let o = |k| PointRef::Origin(k);
    let distance = |sense, min: Option<f64>, max: Option<f64>, a: usize, b: usize| {
        fact(FactKind::Distance { axis: Axis::X, sense, min, max, from: vec![o(a)], to: vec![o(b)] })
    };
    use rnl_runtime::problem::Sense::{Either, Increasing};
    let p = problem_of(
        (0..8).map(|_| instance()).collect(),
        vec![
            // a side: 0 before 1
            distance(Increasing, Some(2.0), None, 0, 1),
            // no side: within 5 of each other, either way; and a DISTRIBUTE, an ALIGN
            distance(Either, None, Some(5.0), 2, 3),
            fact(FactKind::Distribute { axis: Axis::X, members: vec![o(2), o(3), o(4)] }),
            fact(FactKind::Align { axis: Axis::X, members: vec![o(5), o(6)] }),
            // a side left open: reversible
            fact(FactKind::Order { axis: Axis::X, sense: Either, members: vec![o(6), o(7)] }),
        ],
    );
    let s = structure_of(&p);
    assert_eq!(s.precedence.iter().copied().collect::<Vec<_>>(), vec![(0, 1)]);
    assert_eq!(s.open.orders, vec![4]);
    assert_eq!(s.reversible.len(), 2, "{:?}", s.reversible);
    // the DISTRIBUTE fixes no side, but its members come in its order, one way or the other
    assert_eq!(s.reversible[0].facts, vec![2]);
    assert!(monotone(&s.reversible[0].pairs, &[2, 3, 4]), "{:?}", s.reversible);
    assert_eq!(s.reversible[1].facts, vec![4]);
    let [(a, b)] = s.reversible[1].pairs[..] else { panic!("{:?}", s.reversible) };
    assert_eq!((a.min(b), a.max(b)), (6, 7));
}

/// Whether `pairs` puts every element of `chain` before each later one, or every one after
/// each later one, and nothing else.
fn monotone(pairs: &[(usize, usize)], chain: &[usize]) -> bool {
    let way = |forward: bool| -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (i, &a) in chain.iter().enumerate() {
            out.extend(chain[i + 1..].iter().map(|&b| if forward { (a, b) } else { (b, a) }));
        }
        out.sort_unstable();
        out
    };
    let mut pairs = pairs.to_vec();
    pairs.sort_unstable();
    pairs == way(true) || pairs == way(false)
}

/// A DISTRIBUTE on x orders its elements as an ORDER given only an axis does (review of phase
/// 2 of the quality review): those of a `$MIRROR_PAIR`'s, its axis a point between them, one
/// before the other; with an ORDER given only an axis on two of its elements, both turn
/// together; with an ORDER with a direction on two of them, it runs that way, not reversible.
#[test]
fn a_distribute_orders_its_elements_either_way() {
    let o = |k| PointRef::Origin(k);
    let distribute = |members: Vec<PointRef>| fact(FactKind::Distribute { axis: Axis::X, members });
    let order = |sense, members: Vec<PointRef>| fact(FactKind::Order { axis: Axis::X, sense, members });
    use rnl_runtime::problem::Sense::{Either, Increasing};
    let mut objects: Vec<_> = (0..9).map(|_| instance()).collect();
    objects.push(point());
    let p = problem_of(
        objects,
        vec![
            distribute(vec![o(0), o(1), o(2)]),
            order(Either, vec![o(2), o(0)]),
            distribute(vec![o(3), o(4), o(5)]),
            order(Increasing, vec![o(5), o(3)]),
            distribute(vec![o(6), o(9), o(7)]),
        ],
    );
    let s = structure_of(&p);
    assert_eq!(s.reversible.len(), 2, "{:?}", s.reversible);
    assert_eq!(s.reversible[0].facts, vec![0, 1]);
    assert!(monotone(&s.reversible[0].pairs, &[0, 1, 2]), "{:?}", s.reversible);
    // the ORDER puts 5 first, so the DISTRIBUTE runs from 5 to 3
    for pair in [(5, 4), (4, 3), (5, 3)] {
        assert!(s.precedence.contains(&pair), "{:?}", s.precedence);
    }
    assert_eq!(s.precedence.len(), 3, "{:?}", s.precedence);
    // the point orders nothing, and keeps its place between 6 and 7
    assert_eq!(s.reversible[1].facts, vec![4]);
    assert!(monotone(&s.reversible[1].pairs, &[6, 7]), "{:?}", s.reversible);
}

/// A DISTRIBUTE has no direction (14.4): where the witness puts all its members at one x, it
/// runs the way the netlist has them, however it is written, and the same way written the other
/// way round (review of the fix of phase 2: it ran as written). A DISTRIBUTE of two members
/// holds wherever they are, and orders nothing.
#[test]
fn a_distribute_at_one_x_runs_the_way_of_the_netlist() {
    let distribute = |members: &[usize]| {
        let members = members.iter().map(|&k| PointRef::Origin(k)).collect();
        fact(FactKind::Distribute { axis: Axis::X, members })
    };
    // the pairs of the reversible facts, and the precedences, of some DISTRIBUTEs on 0..6 and a
    // point 6, all at x 0 at the witness
    type Orders = (Vec<Vec<(usize, usize)>>, BTreeSet<(usize, usize)>);
    let pairs = |written: &[&[usize]]| -> Orders {
        let mut objects: Vec<_> = (0..6).map(|_| instance()).collect();
        objects.push(point());
        let p = problem_of(objects, written.iter().map(|m| distribute(m)).collect());
        let s = structure_of(&p);
        assert!(s.witness.origin.iter().all(|o| o[0].is_zero()), "{:?}", s.witness.origin);
        (s.reversible.into_iter().map(|r| r.pairs).collect(), s.precedence)
    };
    // the netlist puts the first before the last: written so, or the other way round
    for written in [[0, 1, 2], [2, 1, 0], [0, 2, 1], [1, 2, 0], [1, 0, 2], [2, 0, 1]] {
        let [a, b, c] = written;
        let mut way = vec![(a, b), (a, c), (b, c)];
        if a > c {
            way = way.into_iter().map(|(x, y)| (y, x)).collect();
        }
        way.sort_unstable();
        assert_eq!(pairs(&[&written]), (vec![way], BTreeSet::new()), "{written:?}");
    }
    // written the other way round, the same pairs
    for (written, reversed) in
        [(&[3, 6, 4][..], &[4, 6, 3][..]), (&[0, 1, 2, 0], &[0, 2, 1, 0]), (&[5, 1, 3, 2], &[2, 3, 1, 5])]
    {
        assert_eq!(pairs(&[written]), pairs(&[reversed]), "{written:?} and {reversed:?}");
    }
    // the point between orders nothing
    assert_eq!(pairs(&[&[4, 6, 3]]).0, vec![vec![(3, 4)]]);
    // two that run against each other the way of the netlist turn together, one the other's
    // way: 2, 1, 0 and 1, 0, 3 both hold in 2, 1, 0, 3
    assert_eq!(pairs(&[&[0, 1, 2], &[1, 0, 3]]), (vec![vec![(0, 3), (1, 0), (1, 3), (2, 0), (2, 1)]], BTreeSet::new()));
    // two members, nothing
    assert_eq!(pairs(&[&[4, 3], &[1, 5]]), (Vec::new(), BTreeSet::new()));
}

#[test]
fn a_net_routed_twice_is_drawn_once_with_the_pins_of_both() {
    let mut p = problem_of(vec![instance(), instance(), instance()], Vec::new());
    let pin = |object: usize, pin: usize| PointRef::Pin { object, pin, terminal: ["g", "d", "s"][pin].to_string() };
    let route = |pins: Vec<PointRef>| rnl_runtime::problem::RouteIntent {
        net: "n".to_string(),
        mode: "auto".to_string(),
        style: Vec::new(),
        pins,
        wires: Vec::new(),
    };
    p.routes = vec![route(vec![pin(0, 1), pin(1, 1)]), route(vec![pin(1, 1), pin(2, 0)])];
    let s = structure_of(&p);
    let drawn = &s.connections.drawn;
    assert_eq!(drawn.len(), 1, "{drawn:?}");
    assert_eq!(drawn[0].points, vec![PointId::Pin(0, 1), PointId::Pin(1, 1), PointId::Pin(2, 0)]);
    // every pin of the net drawn to the others
    assert_eq!(s.connections.targets[&(0, 1)], vec![(1, 1), (2, 0)]);
    assert_eq!(s.connections.targets[&(2, 0)], vec![(0, 1), (1, 1)]);
}

#[test]
fn the_members_of_a_set_are_ordered_by_its_centre() {
    let o = |k| PointRef::Origin(k);
    let p = problem_of(
        vec![instance(), instance(), instance()],
        vec![fact(FactKind::Order {
            axis: Axis::X,
            sense: rnl_runtime::problem::Sense::Increasing,
            members: vec![o(0), PointRef::Centre(vec![o(1), o(2)])],
        })],
    );
    let s = structure_of(&p);
    assert!(s.precedence.contains(&(0, 1)) && s.precedence.contains(&(0, 2)));
    assert!(!s.precedence.contains(&(1, 2)) && !s.precedence.contains(&(2, 1)));
}
