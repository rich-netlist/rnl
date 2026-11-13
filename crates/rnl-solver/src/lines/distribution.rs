//! Coordinates at the same fraction of the same distributed span are equal.
//! Read only retained facts; these implications guide groups before all endpoints
//! are present together, without adding facts to the problem or consulting names.

use std::collections::BTreeMap;

use rnl_runtime::problem::{Axis, FactKind, LevelProblem, PointRef};

use crate::budget::{permit, Step};
use crate::model::{Model, PointId};
use crate::structure::Structure;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Alignment {
    pub axis: usize,
    pub from: PointRef,
    pub to: PointRef,
}

type SpanPosition = (usize, PointId, PointId, usize, usize);

/// Resolve already modeled coordinates without allocating new model variables.
/// Explicit nested sets flatten, but an object positioned at a centre remains a
/// point when it occurs inside another set.
fn point(model: &Model, centres: &BTreeMap<&[PointId], usize>, reference: &PointRef) -> Result<Option<PointId>, ()> {
    let direct = |r: &PointRef| match r {
        PointRef::Origin(k) => model.origin_point.get(*k).copied().flatten(),
        PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. } => {
            Some(PointId::Pin(*object, *pin))
        }
        PointRef::PinPosition { object, pin, .. } => Some(PointId::PinPosition(*object, *pin)),
        PointRef::Centre(_) => None,
    };
    if !permit(Step::Structure, 1) {
        return Err(());
    }
    let mut id = if let PointRef::Centre(members) = reference {
        let mut pending: Vec<_> = members.iter().collect();
        let mut ids = Vec::new();
        while let Some(r) = pending.pop() {
            if !permit(Step::Structure, 1) {
                return Err(());
            }
            if let PointRef::Centre(inner) = r {
                pending.extend(inner);
            } else if let Some(id) = direct(r) {
                ids.push(id);
            } else {
                return Ok(None);
            }
        }
        ids.sort_unstable();
        ids.dedup();
        let Some(&c) = centres.get(ids.as_slice()) else { return Ok(None) };
        PointId::Centre(c)
    } else {
        let Some(id) = direct(reference) else { return Ok(None) };
        id
    };
    for _ in 0..=model.centres.len() {
        let PointId::Centre(c) = id else { break };
        let Some(centre) = model.centres.get(c) else { return Ok(None) };
        let [member] = centre.members.as_slice() else { break };
        if !permit(Step::Structure, 1) {
            return Err(());
        }
        id = *member;
    }
    // This is coordinate equality, not an electrical connection.
    if let PointId::PinPosition(k, p) = id {
        id = PointId::Pin(k, p);
    }
    Ok(Some(id))
}

pub(super) fn alignments(problem: &LevelProblem, s: &Structure) -> Option<Vec<Alignment>> {
    let mut centres = BTreeMap::new();
    for (c, centre) in s.model.centres.iter().enumerate() {
        if !permit(Step::Structure, centre.members.len().max(1)) {
            return None;
        }
        centres.insert(centre.members.as_slice(), c);
    }
    // Each span/fraction retains only its latest member. Connecting successive
    // members keeps reference storage linear even when a centre has many points.
    let mut at: BTreeMap<SpanPosition, (PointId, &PointRef)> = BTreeMap::new();
    let mut out = Vec::new();
    for &f in &s.kept {
        if !permit(Step::Structure, 1) {
            return None;
        }
        let FactKind::Distribute { axis, members, .. } = &problem.facts[f].kind else { continue };
        if members.len() < 3 {
            continue;
        }
        let axis = usize::from(*axis == Axis::Y);
        let n = members.len() - 1;
        let first = point(&s.model, &centres, &members[0]).ok()?;
        let last = point(&s.model, &centres, &members[n]).ok()?;
        let (Some(first), Some(last)) = (first, last) else { continue };
        let reverse = last < first;
        for (i, member) in members.iter().enumerate().take(n).skip(1) {
            let Some(id) = point(&s.model, &centres, member).ok()? else { continue };
            let i = if reverse { n - i } else { i };
            let gcd = num_integer::gcd(i, n);
            let key = (axis, first.min(last), first.max(last), i / gcd, n / gcd);
            if let Some((before, previous)) = at.insert(key, (id, member)) {
                if before != id {
                    out.push(Alignment { axis, from: previous.clone(), to: member.clone() });
                }
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{fact, point as object, problem_of, Ticks};
    use crate::{RunLimit, SolverOptions, StoppedClock, Work};
    use rnl_runtime::problem::{Position, Sense};

    fn distribute(axis: Axis, members: Vec<PointRef>) -> rnl_runtime::problem::ProblemFact {
        fact(FactKind::Distribute { axis, sense: Sense::Either, members })
    }

    fn structure(p: &LevelProblem) -> Structure {
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(p, &options, &StoppedClock);
        crate::structure::analyse(p, &resolved, &options, crate::options::CAPS, &mut Work::default())
    }

    #[test]
    fn equal_fractions_use_both_retained_spans_and_respect_axes() {
        use PointRef::Origin as o;
        let mut p = problem_of(
            vec![object(); 8],
            vec![
                distribute(Axis::X, vec![o(0), o(2), o(1)]),
                distribute(Axis::X, vec![o(1), o(3), o(4), o(5), o(0)]),
                distribute(Axis::Y, vec![o(0), o(6), o(1)]),
                distribute(Axis::X, vec![o(0), o(7), o(1)]),
            ],
        );
        p.version = "0.4".into();
        let mut s = structure(&p);
        assert_eq!(s.kept.len(), 4);
        assert_eq!(
            alignments(&p, &s).unwrap(),
            vec![Alignment { axis: 0, from: o(2), to: o(4) }, Alignment { axis: 0, from: o(4), to: o(7) },]
        );
        let run = || alignments(&p, &s);
        for seconds in [0., 1., 5., 10., 30., 100.] {
            let (timed, report) = crate::budget::with(&Ticks::default(), RunLimit::Time(seconds), None, run);
            let (replayed, _) = crate::budget::with(&StoppedClock, RunLimit::Work(report.work), None, run);
            assert_eq!(timed, replayed);
        }
        s.kept = vec![0, 2];
        assert!(alignments(&p, &s).unwrap().is_empty(), "discarded distributions imply nothing");
    }

    #[test]
    fn endpoint_aliases_and_singleton_sets_share_coordinate_identity() {
        use PointRef::Origin as o;
        let mut p = problem_of(
            vec![object(); 5],
            vec![
                distribute(Axis::X, vec![o(0), o(2), o(1)]),
                distribute(Axis::X, vec![PointRef::Centre(vec![o(4)]), o(3), o(1)]),
            ],
        );
        p.version = "0.4".into();
        p.objects[4].position = Position::At(o(0));
        let s = structure(&p);
        assert_eq!(s.kept.len(), 2);
        assert_eq!(alignments(&p, &s).unwrap(), vec![Alignment { axis: 0, from: o(2), to: o(3) }]);
    }

    #[test]
    fn a_point_at_a_centre_stays_a_point_inside_another_set() {
        use PointRef::Origin as o;
        let mut p = problem_of(
            vec![object(); 7],
            vec![
                distribute(Axis::X, vec![PointRef::Centre(vec![o(6), o(2)]), o(3), o(4)]),
                distribute(Axis::X, vec![PointRef::Centre(vec![o(0), o(1), o(2)]), o(5), o(4)]),
            ],
        );
        p.version = "0.4".into();
        p.objects[6].position = Position::At(PointRef::Centre(vec![o(0), o(1)]));
        let s = structure(&p);
        assert_eq!(s.kept.len(), 2);
        assert!(alignments(&p, &s).unwrap().is_empty());
    }
}
