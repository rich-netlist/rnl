//! Which GROUPs (11.5) make the tree of groups, and how they nest, from their member sets alone.
//!
//! The solver builds its tree with these rules on the objects it places, and a viewer shows the
//! same rules on the members as written, without solving. Direct declarations come before those a
//! definition expansion makes, and within each class later declarations come first. Of repeated
//! declarations on one set only the first counts. A set equal to one already taken is that group
//! again, and a set that crosses one already taken, sharing members without either holding the
//! other, is left out. Names never matter.

use std::collections::BTreeSet;

/// One GROUP declaration, in the order the level makes them.
#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    /// The identity of the grouped set: repeated GROUPs on one set share it.
    pub identity: usize,
    /// The first valid declaration on its set; the others repeat it.
    pub first: bool,
    /// Written outside a definition expansion.
    pub direct: bool,
    pub members: BTreeSet<usize>,
}

/// What became of a declaration ([`select`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fate {
    /// In the tree.
    Taken,
    /// Not the first declaration on its set.
    Repeat,
    /// No members.
    Empty,
    /// Its members are those of this declaration, already taken.
    Same(usize),
    /// It crosses this declaration, already taken.
    Crosses(usize),
}

/// What [`select`] decided.
#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    /// The fate of each declaration.
    pub fates: Vec<Fate>,
    /// The first declarations of the sets, in the order they were weighed.
    pub order: Vec<usize>,
    /// The declarations taken, in the order they were taken.
    pub taken: Vec<usize>,
}

/// Weigh the declarations. `permit` is asked before each step; when it says no, selection stops
/// and gives nothing.
pub fn select(declarations: &[Declaration], permit: &mut dyn FnMut(Step) -> bool) -> Option<Selection> {
    let mut fates = vec![Fate::Repeat; declarations.len()];
    let mut identities = BTreeSet::new();
    let mut order = Vec::new();
    for (g, d) in declarations.iter().enumerate() {
        if !d.first || !identities.insert(d.identity) {
            continue;
        }
        if !permit(Step::Declaration(g)) {
            return None;
        }
        order.push(g);
    }
    order.reverse();
    order.sort_by_key(|&g| !declarations[g].direct);
    let mut taken = Vec::<usize>::new();
    for &g in &order {
        if !permit(Step::Compare(taken.len().max(1))) {
            return None;
        }
        let set = &declarations[g].members;
        if set.is_empty() {
            fates[g] = Fate::Empty;
            continue;
        }
        if let Some(&t) = taken.iter().find(|&&t| declarations[t].members == *set) {
            fates[g] = Fate::Same(t);
            continue;
        }
        let crossing = taken.iter().copied().find(|&t| {
            let other = &declarations[t].members;
            !set.is_disjoint(other) && !set.is_subset(other) && !other.is_subset(set)
        });
        if let Some(t) = crossing {
            fates[g] = Fate::Crosses(t);
        } else {
            fates[g] = Fate::Taken;
            taken.push(g);
        }
    }
    Some(Selection { fates, order, taken })
}

/// A step [`select`] or [`nest`] asks leave for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Reading this declaration, by its index.
    Declaration(usize),
    /// Comparing one set with this many taken before it, at least one.
    Compare(usize),
    /// Placing one set among this many nodes, the root counted.
    Place(usize),
}

/// Nest distinct sets by containment, the larger first: for each set in that order, its parent is
/// the latest set placed before it that holds it, or none for the root. `sets` are given as the
/// caller orders equal sizes; the answer is in that order too, `None` when `permit` says no.
pub fn nest(sets: &[&BTreeSet<usize>], permit: &mut dyn FnMut(Step) -> bool) -> Option<Vec<Option<usize>>> {
    let mut by_size: Vec<usize> = (0..sets.len()).collect();
    by_size.sort_by_key(|&i| std::cmp::Reverse(sets[i].len()));
    let mut parents = vec![None; sets.len()];
    let mut placed: Vec<usize> = Vec::new();
    for i in by_size {
        if !permit(Step::Place(placed.len() + 1)) {
            return None;
        }
        parents[i] = placed.iter().rev().copied().find(|&p| sets[i] != sets[p] && sets[i].is_subset(sets[p]));
        placed.push(i);
    }
    Some(parents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(identity: usize, first: bool, direct: bool, members: &[usize]) -> Declaration {
        Declaration { identity, first, direct, members: members.iter().copied().collect() }
    }

    fn all(_: Step) -> bool {
        true
    }

    #[test]
    fn direct_first_then_later_first_and_crossings_left_out() {
        let ds = [
            d(0, true, false, &[0, 1]), // made in an expansion
            d(1, true, true, &[1, 2]),  // written: comes first, so 0 crosses it
            d(2, true, true, &[3, 4]),
            d(3, true, true, &[3, 4, 5]), // later: taken before 2, which nests in it
            d(1, false, true, &[1, 2]),   // a repeat of 1
            d(4, true, true, &[]),
            d(5, true, false, &[3, 4]), // the members of 2
        ];
        let Selection { fates, order, taken } = select(&ds, &mut all).unwrap();
        assert_eq!(order, [5, 3, 2, 1, 6, 0]);
        assert_eq!(
            fates,
            [Fate::Crosses(1), Fate::Taken, Fate::Taken, Fate::Taken, Fate::Repeat, Fate::Empty, Fate::Same(2)]
        );
        assert_eq!(taken, [3, 2, 1]);
        let sets: Vec<_> = taken.iter().map(|&g| &ds[g].members).collect();
        assert_eq!(nest(&sets, &mut all).unwrap(), [None, Some(0), None]);
    }

    #[test]
    fn a_refusal_stops_with_nothing() {
        let ds = [d(0, true, true, &[0, 1]), d(1, true, true, &[2, 3])];
        let mut left = 2;
        let mut permit = |_| {
            left -= 1;
            left >= 0
        };
        assert!(select(&ds, &mut permit).is_none());
    }
}
