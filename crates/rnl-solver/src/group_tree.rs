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
    let (tree, notes, _) = analyse(problem, elements);
    (tree, notes)
}

pub(crate) fn analyse(problem: &LevelProblem, elements: &[usize]) -> (GroupTree, Vec<String>, Vec<usize>) {
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
            Step::Declaration(g) => problem.groups[g].members.len(),
            Step::Compare(n) | Step::Place(n) => elements.len().saturating_mul(n),
        };
        crate::budget::permit(crate::budget::Step::Structure, size)
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
    let mut sets = BTreeMap::new();
    let mut projected = Vec::new();
    for g in taken {
        let set: BTreeSet<_> = declarations[g].members.intersection(&all).copied().collect();
        if set.len() >= 2 && !projected.iter().any(|t| sets[t] == set) {
            sets.insert(g, set);
            projected.push(g);
        }
    }
    projected.sort_by_key(|g| (std::cmp::Reverse(sets[g].len()), *g));
    let placed: Vec<&BTreeSet<usize>> = projected.iter().map(|g| &sets[g]).collect();
    let Some(parents) = nest(&placed, &mut permit) else {
        return fallback();
    };
    let mut tree = root(elements);
    for (i, &g) in projected.iter().enumerate() {
        tree.nodes.push(GroupNode {
            kind: NodeKind::Group(g),
            members: sets[&g].iter().copied().collect(),
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
