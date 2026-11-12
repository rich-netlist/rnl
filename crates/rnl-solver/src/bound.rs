//! What the language says of objects that the solver reads as a whole: the mirror pairs, and the
//! objects a usage hangs from a pin. Read from the facts and from how the roles of usages are bound,
//! never from the name of a definition, a symbol or a package, so that a package of one's own, or
//! a series of the standard package that names them otherwise, is read the same.

use std::collections::BTreeSet;

use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{Axis, FactKind, LevelProblem, PointRef};

use crate::structure::{ObjectInfo, Pair};

/// The mirror pairs ([`Pair`]): two oriented objects, one an ORIENT-RELATION turns into the other by
/// MY, or by one of several relations with MY among them (14.9), `a` the one it turns from; about
/// `axis` when a DISTRIBUTE along x of three members has the two at its ends and in the middle an
/// object with a position of its own, without consulting definition names. Each pair once,
/// in order.
pub(crate) fn pairs(problem: &LevelProblem, objects: &[ObjectInfo]) -> Vec<Pair> {
    let oriented = |k: usize| objects.get(k).is_some_and(|o| o.oriented);
    let own = |k: &usize| objects.get(*k).is_some_and(|o| o.own);
    let mut found: BTreeSet<(usize, usize)> = BTreeSet::new();
    for (f, fact) in problem.facts.iter().enumerate() {
        let FactKind::OrientRelation { relation, from, to } = &fact.kind else { continue };
        let mirrored = problem
            .orient_lists
            .iter()
            .find(|(i, _)| *i == f)
            .map_or(*relation == Orient::MY, |(_, codes)| codes.contains(&Orient::MY));
        if !mirrored {
            continue;
        }
        if let (&[a], &[b]) = (from.as_slice(), to.as_slice()) {
            if a != b && oriented(a) && oriented(b) {
                found.insert((a, b));
            }
        }
    }
    found
        .into_iter()
        .map(|(a, b)| {
            let axis = problem.facts.iter().find_map(|f| match &f.kind {
                FactKind::Distribute { axis: Axis::X, members, .. } => match members.as_slice() {
                    [p, m, q] => {
                        let ends = (single(p)?, single(q)?);
                        (ends == (a, b) || ends == (b, a)).then(|| single(m)).flatten().filter(own)
                    }
                    _ => None,
                },
                _ => None,
            });
            Pair { a, b, axis }
        })
        .collect()
}

/// The object a member of a fact is the point of: its origin, or the centre of a set of it alone.
fn single(p: &PointRef) -> Option<usize> {
    match p {
        PointRef::Origin(k) => Some(*k),
        PointRef::Centre(inner) => match inner.as_slice() {
            [q] => single(q),
            _ => None,
        },
        PointRef::Pin { .. } | PointRef::SymbolPin { .. } | PointRef::PinPosition { .. } => None,
    }
}
