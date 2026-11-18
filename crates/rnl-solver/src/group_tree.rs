//! Build the solver's containment tree from all active GROUP declarations (11.5), by the rules
//! of [`rnl_runtime::group_tree`], which a viewer applies too: direct declarations precede
//! expanded ones, later declarations first within each class; repeated declarations on one set
//! use its first GROUP; a crossing set is left out, an equal member set reuses the group already
//! selected. Here the sets taken are then projected onto the elements laid out. Names affect only
//! the diagnostic text.

use crate::structure::{GroupNode, GroupTree, NodeKind};
use rnl_runtime::group_tree::{nest, select, Declaration, Fate, Selection, Step};
use rnl_runtime::problem::LevelProblem;
use std::collections::{BTreeMap, BTreeSet};

fn root(elements: &[usize]) -> GroupTree {
    let members: Vec<_> = elements.iter().copied().collect::<BTreeSet<_>>().into_iter().collect();
    GroupTree {
        nodes: vec![GroupNode {
            kind: NodeKind::Root,
            own: members.clone(),
            members,
            children: Vec::new(),
            fixed_order: false,
        }],
    }
}

#[cfg(test)]
fn from_problem(problem: &LevelProblem, elements: &[usize]) -> (GroupTree, Vec<String>) {
    let (tree, notes, _) = analyse(problem, elements, &BTreeMap::new());
    (tree, notes)
}

/// `attached`: of a level of 0.4, each element of the netlist with the symbols of usages of one role
/// attached to it ([`crate::structure::attached_element`]). Each such element and its symbols are
/// one group of its own ([`NodeKind::Attached`]), and each group the element is in takes its
/// symbols in too, so that the group of the element nests in it; a group that would then cross
/// another keeps the members it was given.
pub(crate) fn analyse(
    problem: &LevelProblem,
    elements: &[usize],
    attached: &BTreeMap<usize, Vec<usize>>,
) -> (GroupTree, Vec<String>, Vec<usize>) {
    let fallback = || {
        (root(elements), vec!["group search stopped at the shared budget; using one layout block".into()], Vec::new())
    };
    let all: BTreeSet<_> = elements.iter().copied().collect();
    let declarations: Vec<Declaration> = problem
        .groups
        .iter()
        .map(|g| Declaration {
            identity: g.identity,
            first: g.first,
            direct: g.direct,
            members: g.members.iter().copied().collect(),
        })
        .collect();
    let mut permit = |step: Step| {
        let size = match step {
            Step::Declaration(g) => problem.groups[g].members.len() as u64,
            Step::Compare(n) | Step::Place(n) => crate::budget::product(elements.len(), n),
        };
        crate::budget::permit_size(crate::budget::Step::Structure, size)
    };
    let Some(Selection { fates, order, taken }) = select(&declarations, &mut permit) else {
        return fallback();
    };
    let notes: Vec<String> = order
        .iter()
        .filter_map(|&g| match fates[g] {
            Fate::Crosses(t) => Some(format!(
                "the group {} crosses {}; it is not searched as a block",
                problem.groups[g].label, problem.groups[t].label
            )),
            _ => None,
        })
        .collect();
    // Crossings must be decided before projecting onto elements: a shared port can make
    // otherwise disjoint element groups cross. Projection can then make nested sets equal.
    let mut selected = taken.clone();
    selected.sort_unstable();
    let mut sets: Vec<(NodeKind, BTreeSet<usize>)> = Vec::new();
    for g in taken {
        let set: BTreeSet<_> = declarations[g].members.intersection(&all).copied().collect();
        if set.len() >= 2 && !sets.iter().any(|(_, t)| *t == set) {
            sets.push((NodeKind::Group(g), set));
        }
    }
    // the symbols attached to an element go into each group the element is in, where that
    // crosses no other group: the larger groups first, and again until none changes, so that an
    // inner group is widened after the group round it; a group made equal to another goes
    let crosses = |a: &BTreeSet<usize>, b: &BTreeSet<usize>| !a.is_disjoint(b) && !a.is_subset(b) && !b.is_subset(a);
    let mut changed = true;
    while changed {
        changed = false;
        let mut order: Vec<usize> = (0..sets.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(sets[i].1.len()));
        for i in order {
            let mut wider = sets[i].1.clone();
            for (e, symbols) in attached {
                if wider.contains(e) {
                    wider.extend(symbols.iter().copied().filter(|k| all.contains(k)));
                }
            }
            if wider != sets[i].1 && !sets.iter().enumerate().any(|(j, (_, t))| j != i && crosses(&wider, t)) {
                sets[i].1 = wider;
                changed = true;
            }
        }
    }
    let mut unique: Vec<(NodeKind, BTreeSet<usize>)> = Vec::new();
    for (kind, set) in sets {
        if !unique.iter().any(|(_, t)| *t == set) {
            unique.push((kind, set));
        }
    }
    let mut sets = unique;
    for (e, symbols) in attached {
        let set: BTreeSet<usize> =
            std::iter::once(*e).chain(symbols.iter().copied()).filter(|k| all.contains(k)).collect();
        if set.len() >= 2 && !sets.iter().any(|(_, t)| *t == set || crosses(&set, t)) {
            sets.push((NodeKind::Attached(*e), set));
        }
    }
    let rank = |kind: &NodeKind| match kind {
        NodeKind::Group(g) => (0, *g),
        NodeKind::Attached(e) => (1, *e),
        _ => (2, 0),
    };
    sets.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| rank(&a.0).cmp(&rank(&b.0))));
    let placed: Vec<&BTreeSet<usize>> = sets.iter().map(|(_, s)| s).collect();
    let Some(parents) = nest(&placed, &mut permit) else {
        return fallback();
    };
    let mut tree = root(elements);
    for (i, (kind, set)) in sets.iter().enumerate() {
        tree.nodes.push(GroupNode {
            kind: kind.clone(),
            members: set.iter().copied().collect(),
            children: Vec::new(),
            own: Vec::new(),
            fixed_order: false,
        });
        let parent = parents[i].map_or(0, |p| p + 1);
        tree.nodes[parent].children.push(i + 1);
    }
    for n in 0..tree.nodes.len() {
        let mut children = std::mem::take(&mut tree.nodes[n].children);
        children.sort_by_key(|&c| tree.nodes[c].members[0]);
        let inner: BTreeSet<_> = children.iter().flat_map(|&c| tree.nodes[c].members.iter().copied()).collect();
        tree.nodes[n].own = tree.nodes[n].members.iter().copied().filter(|k| !inner.contains(k)).collect();
        tree.nodes[n].children = children;
    }
    (tree, notes, selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{instance, problem_of};
    use rnl_runtime::problem::ProblemGroup;
    fn group(identity: usize, direct: bool, label: &str, members: &[usize]) -> ProblemGroup {
        ProblemGroup {
            identity,
            first: true,
            direct,
            label: label.into(),
            members: members.to_vec(),
            spacing_x: None,
            spacing_y: None,
        }
    }
    #[test]
    fn provenance_and_order_choose_crossing_groups_without_reading_names() {
        let mut p = problem_of((0..5).map(|_| instance()).collect(), vec![]);
        p.groups = vec![
            group(0, true, "looks-expanded", &[0, 1]),
            group(1, false, "%looks-direct", &[1, 2]),
            group(2, true, "other", &[3, 4]),
        ];
        let (tree, notes) = from_problem(&p, &[0, 1, 2, 3, 4]);
        assert_eq!(notes.len(), 1);
        assert!(tree.nodes.iter().any(|n| n.kind == NodeKind::Group(0)));
        assert!(!tree.nodes.iter().any(|n| n.kind == NodeKind::Group(1)));
        p.groups[0].label = "%changed".into();
        p.groups[1].label = "changed".into();
        assert_eq!(from_problem(&p, &[0, 1, 2, 3, 4]).0, tree);
        p.groups[1].direct = true;
        let tree = from_problem(&p, &[0, 1, 2, 3, 4]).0;
        assert!(tree.nodes.iter().any(|n| n.kind == NodeKind::Group(1)));
        assert!(!tree.nodes.iter().any(|n| n.kind == NodeKind::Group(0)));
    }
    #[test]
    fn first_group_per_handle_and_latest_equal_member_set_are_distinct_rules() {
        let mut p = problem_of((0..3).map(|_| instance()).collect(), vec![]);
        p.groups =
            vec![group(0, false, "body", &[0, 1]), group(1, true, "cross", &[1, 2]), group(0, true, "again", &[0, 1])];
        let tree = from_problem(&p, &[0, 1, 2]).0;
        assert!(tree.nodes.iter().any(|n| n.kind == NodeKind::Group(1)));
        assert_eq!(tree.nodes.len(), 2);
        p.groups[2].identity = 2;
        let tree = from_problem(&p, &[0, 1, 2]).0;
        assert!(tree.nodes.iter().any(|n| n.kind == NodeKind::Group(2)));
        assert_eq!(tree.nodes.len(), 2);
    }
    #[test]
    fn shared_non_element_member_still_makes_groups_cross() {
        let mut p = problem_of((0..5).map(|_| instance()).collect(), vec![]);
        p.groups = vec![group(0, true, "left", &[0, 1, 4]), group(1, true, "right", &[2, 3, 4])];
        let (tree, notes) = from_problem(&p, &[0, 1, 2, 3]);
        assert_eq!(notes.len(), 1);
        assert_eq!(tree.nodes.len(), 2);
        assert_eq!(tree.nodes[1].kind, NodeKind::Group(1));
        assert_eq!(tree.nodes[0].own, vec![0, 1]);
        // A one-element group participates in selection before it is omitted as a block.
        p.groups[1].members = vec![2, 4];
        let (tree, notes) = from_problem(&p, &[0, 1, 2, 3]);
        assert_eq!(notes.len(), 1);
        assert_eq!(tree, root(&[0, 1, 2, 3]));
    }

    /// The symbols attached to an element make one group with it, and each group the element is
    /// in takes them in: B and C attached to A, with {A D} declared, give {A B C} within {A B C D}.
    #[test]
    fn symbols_attached_to_an_element_group_with_it_within_its_groups() {
        let mut p = problem_of((0..5).map(|_| instance()).collect(), vec![]);
        p.groups = vec![group(0, true, "g", &[0, 3])];
        let attached = BTreeMap::from([(0, vec![1, 2])]);
        let (tree, _, _) = analyse(&p, &[0, 1, 2, 3, 4], &attached);
        let node = |kind: NodeKind| tree.nodes.iter().position(|n| n.kind == kind).unwrap();
        let (outer, inner) = (node(NodeKind::Group(0)), node(NodeKind::Attached(0)));
        assert_eq!(tree.nodes[outer].members, vec![0, 1, 2, 3]);
        assert_eq!(tree.nodes[inner].members, vec![0, 1, 2]);
        assert_eq!(tree.nodes[outer].children, vec![inner]);
        assert_eq!(tree.nodes[outer].own, vec![3]);
        assert_eq!(tree.nodes[0].own, vec![4]);
    }

    /// An inner group takes the attached symbols after the group round it has, whatever the order
    /// of the declarations; and two groups made equal by them are one node.
    #[test]
    fn attached_symbols_widen_nested_groups_from_the_outside_and_equal_groups_merge() {
        let mut p = problem_of((0..5).map(|_| instance()).collect(), vec![]);
        let attached = BTreeMap::from([(0, vec![1])]);
        p.groups = vec![group(0, true, "outer", &[0, 3, 4]), group(1, true, "inner", &[0, 3])];
        let (tree, _, _) = analyse(&p, &[0, 1, 2, 3, 4], &attached);
        let members = |kind: NodeKind| tree.nodes.iter().find(|n| n.kind == kind).map(|n| n.members.clone());
        assert_eq!(members(NodeKind::Group(0)), Some(vec![0, 1, 3, 4]));
        assert_eq!(members(NodeKind::Group(1)), Some(vec![0, 1, 3]));
        assert_eq!(members(NodeKind::Attached(0)), Some(vec![0, 1]));
        p.groups = vec![group(0, true, "a", &[0, 2]), group(1, true, "b", &[0, 1, 2])];
        let (tree, _, _) = analyse(&p, &[0, 1, 2, 3, 4], &attached);
        let sets: Vec<&Vec<usize>> = tree.nodes[1..].iter().map(|n| &n.members).collect();
        assert_eq!(sets.iter().filter(|m| ***m == vec![0, 1, 2]).count(), 1, "{sets:?}");
        assert_eq!(tree.nodes[0].own, vec![3, 4]);
    }

    #[test]
    fn shared_budget_stop_returns_a_valid_complete_root() {
        let mut p = problem_of((0..3).map(|_| instance()).collect(), vec![]);
        p.groups = vec![group(0, true, "g", &[0, 1])];
        let ((tree, notes), _) =
            crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(0), None, || from_problem(&p, &[0, 1, 2]));
        assert_eq!(tree, root(&[0, 1, 2]));
        assert_eq!(notes.len(), 1);
    }
}
