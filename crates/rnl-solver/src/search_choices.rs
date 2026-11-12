//! Search coordinates for authored orientation alternatives. These classes describe a
//! candidate, not the facts: placement always retains the original full predicates.

use crate::model::OrientEffect;
use crate::orient::OrientClasses;
use crate::structure::{FreeClass, Structure};
use rnl_runtime::geom::Orient;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Way {
    pub fact: usize,
    pub count: usize,
    pub taken: usize,
}

/// An absolute pin-direction predicate restricts one object's orientation. Expose its
/// equivalent finite alternatives to layout search without rewriting the retained fact.
fn absolute_pin_codes(effect: &OrientEffect) -> Option<(usize, Vec<Orient>)> {
    let OrientEffect::Pins { from, to: None, .. } = effect else { return None };
    Some((from.object, Orient::ALL.into_iter().filter(|&o| effect.predicate_holds(|_| o)).collect()))
}

/// Choose one authored alternative as a search preference, leaving the original unit intact.
pub(crate) fn effect(effect: &OrientEffect, value: usize) -> OrientEffect {
    if let Some((object, codes)) = absolute_pin_codes(effect) {
        return codes.get(value).map_or(OrientEffect::None, |&code| OrientEffect::Fix { object, code });
    }
    match effect {
        OrientEffect::FixOneOf { object, codes } => {
            codes.get(value).map_or(OrientEffect::None, |code| OrientEffect::Fix { object: *object, code: *code })
        }
        OrientEffect::RelateOneOf { from, to, relations } => {
            relations.get(value).map_or(OrientEffect::None, |relation| OrientEffect::Relate {
                from: from.clone(),
                to: to.clone(),
                relation: *relation,
            })
        }
        _ => OrientEffect::None,
    }
}

fn take(classes: &mut OrientClasses, effect: &OrientEffect, fact: usize) -> Result<(), Vec<usize>> {
    match effect {
        OrientEffect::Fix { object, code } => classes.fix(*object, *code, fact),
        OrientEffect::Relate { from, to, relation } => {
            if from.is_empty() || to.is_empty() {
                return Ok(());
            }
            for side in [from, to] {
                for pair in side.windows(2) {
                    classes.relate(pair[0], pair[1], Orient::R0, fact)?;
                }
            }
            if let (Some(a), Some(b)) = (from.first(), to.first()) {
                classes.relate(*a, *b, *relation, fact)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// The candidate's classes in fact order. Set-to-set lists retain each pair's witnessed
/// relation; their alternatives remain predicates in the placement resolver.
pub(crate) fn classes(s: &Structure, ways: &[Way], values: &[usize]) -> Option<OrientClasses> {
    let mut classes = OrientClasses::new(s.objects.len());
    for &f in &s.kept {
        let original = &s.model.units[f].orient;
        if let Some((i, w)) = ways.iter().enumerate().find(|(_, w)| w.fact == f) {
            take(&mut classes, &effect(original, values.get(i).copied().unwrap_or(w.taken)), f).ok()?;
            continue;
        }
        if let Some((object, _)) = absolute_pin_codes(original) {
            classes.fix(object, s.witness.orient[object], f).ok()?;
            continue;
        }
        match original {
            OrientEffect::FixOneOf { object, .. } => {
                classes.fix(*object, s.witness.orient[*object], f).ok()?;
            }
            OrientEffect::RelateOneOf { from, to, relations } => {
                for &a in from {
                    for &b in to {
                        let relation =
                            relations.iter().find(|r| s.witness.orient[a].then(**r) == s.witness.orient[b])?;
                        classes.relate(a, b, *relation, f).ok()?;
                    }
                }
            }
            _ => take(&mut classes, original, f).ok()?,
        }
    }
    Some(classes)
}

/// A private view for layout search. The model and its verified witness are unchanged.
pub(crate) fn for_layout(s: &Structure, work: &mut crate::Work) -> Structure {
    let mut out = s.clone();
    out.open.search_classes = true;
    let Some(classes) = classes(s, &[], &[]) else { return out };
    let mut by_root: BTreeMap<usize, Vec<(usize, Orient)>> = BTreeMap::new();
    for (k, o) in s.objects.iter().enumerate().filter(|(_, o)| o.oriented) {
        let _ = o;
        let (root, label) = classes.find(k);
        if classes.fixed(root).is_none() {
            by_root.entry(root).or_default().push((k, label));
        }
    }
    out.open.classes = by_root.into_iter().map(|(root, members)| FreeClass { root, members }).collect();
    out.open.ways = s
        .kept
        .iter()
        .filter_map(|&fact| {
            if let Some((object, codes)) = absolute_pin_codes(&s.model.units[fact].orient) {
                return Some(Way {
                    fact,
                    count: codes.len(),
                    taken: codes.iter().position(|v| *v == s.witness.orient[object])?,
                });
            }
            let (count, taken) = match &s.model.units[fact].orient {
                OrientEffect::FixOneOf { object, codes } => {
                    (codes.len(), codes.iter().position(|v| *v == s.witness.orient[*object])?)
                }
                OrientEffect::RelateOneOf { from, to, relations } => {
                    let ([a], [b]) = (from.as_slice(), to.as_slice()) else { return None };
                    let ((ra, ga), (rb, gb)) = (classes.find(*a), classes.find(*b));
                    if ra != rb || classes.fixed(ra).is_some() {
                        return None;
                    }
                    (relations.len(), relations.iter().position(|v| ga.then(*v) == gb)?)
                }
                _ => return None,
            };
            Some(Way { fact, count, taken })
        })
        .collect();
    // Fitting proofs refer to the original hard classes. A merged search class must be
    // checked as a whole by placement; reusing one member's proof would filter valid moves.
    out.fitting.clear();
    let (fitting, stopped) = crate::structure::fitting_values(&out, work);
    out.fitting = fitting;
    if stopped {
        out.notes.push(
            "current-direction checks for layout classes reached their work limit; unchecked values remain available"
                .into(),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{fact, instance, problem_of};
    use crate::search::{orientation_relations, with_relations, Candidate};
    use crate::{SolverOptions, Work};
    use rnl_runtime::problem::FactKind;

    fn structure(p: &rnl_runtime::problem::LevelProblem) -> Structure {
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(p, &options, &crate::StoppedClock);
        crate::structure::analyse(p, &resolved, &options, crate::options::CAPS, &mut Work::default())
    }

    fn candidate(values: Vec<Orient>, ways: Vec<usize>) -> Candidate {
        Candidate { values, ways, rows: vec![], apart: vec![], even: false, columns: false, origin: "tree" }
    }

    #[test]
    fn relation_choices_turn_both_members_without_rewriting_the_fact() {
        let mut p = problem_of(
            vec![instance(), instance()],
            vec![fact(FactKind::OrientRelation { relation: Orient::R0, from: vec![0], to: vec![1] })],
        );
        p.version = "0.4".into();
        p.orient_lists = vec![(0, vec![Orient::R0, Orient::MY])];
        let original = structure(&p);
        assert_eq!(original.open.classes.len(), 2);
        let s = for_layout(&original, &mut Work::default());
        assert_eq!(s.open.classes.len(), 1);
        assert_eq!(s.model.units, original.model.units);
        assert_eq!(s.witness, original.witness);
        for value in Orient::ALL {
            for (way, relation) in [Orient::R0, Orient::MY].into_iter().enumerate() {
                let c = candidate(vec![value], vec![way]);
                let proposed = c.orientations(&s);
                assert_eq!(proposed[1], proposed[0].then(relation));
                with_relations(&s, &orientation_relations(&s, &c), &mut Work::default(), |resolver, _, taken| {
                    assert!(taken.kept.iter().all(|k| *k));
                    assert!(resolver.holds(0));
                    assert_eq!(resolver.geometry().orient, proposed);
                });
            }
        }
    }

    #[test]
    fn an_empty_relation_side_does_not_join_search_classes() {
        let mut p = problem_of(
            vec![instance(), instance()],
            vec![fact(FactKind::OrientRelation { relation: Orient::R0, from: vec![0, 1], to: vec![] })],
        );
        p.version = "0.4".into();
        let original = structure(&p);
        let s = for_layout(&original, &mut Work::default());
        assert_eq!(s.open.classes.len(), 2);
        assert!(classes(&s, &[], &[]).is_some());
    }

    #[test]
    fn incompatible_way_choices_reject_the_whole_batch() {
        let mut p = problem_of(
            vec![instance()],
            vec![
                fact(FactKind::Orient { object: 0, code: Orient::R0 }),
                fact(FactKind::Orient { object: 0, code: Orient::R0 }),
            ],
        );
        p.version = "0.4".into();
        p.orient_lists = vec![(0, vec![Orient::R0, Orient::R90]), (1, vec![Orient::R0, Orient::R90])];
        let original = structure(&p);
        let s = for_layout(&original, &mut Work::default());
        assert_eq!(s.open.ways.len(), 2);
        let c = candidate(vec![], vec![1, 0]);
        with_relations(&s, &orientation_relations(&s, &c), &mut Work::default(), |resolver, _, _| {
            assert!(resolver.holds(0) && resolver.holds(1));
            assert_eq!(resolver.geometry().orient, s.witness.orient);
        });
    }

    #[test]
    fn fixed_list_choices_survive_rejected_preferences_and_budget_exhaustion() {
        let mut p = problem_of(vec![instance()], vec![fact(FactKind::Orient { object: 0, code: Orient::R0 })]);
        p.version = "0.4".into();
        p.orient_lists = vec![(0, vec![Orient::R0, Orient::R90])];
        let original = structure(&p);
        let s = for_layout(&original, &mut Work::default());
        assert!(s.open.classes.is_empty());
        let c = candidate(vec![], vec![1]);
        with_relations(&s, &orientation_relations(&s, &c), &mut Work::default(), |resolver, _, taken| {
            assert!(taken.kept.iter().all(|k| *k));
            assert_eq!(resolver.geometry().orient[0], Orient::R90);
            assert!(resolver.holds(0));
        });
        let mut capped = s.clone();
        capped.caps.nodes = 0;
        with_relations(&capped, &orientation_relations(&capped, &c), &mut Work::default(), |resolver, _, taken| {
            assert!(taken.undecided);
            assert!(resolver.holds(0));
        });
        crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(0), None, || {
            with_relations(&s, &orientation_relations(&s, &c), &mut Work::default(), |resolver, _, _| {
                assert!(resolver.holds(0));
                assert!(p.orient_lists[0].1.contains(&resolver.geometry().orient[0]));
            });
        });
    }

    #[test]
    fn absolute_pin_directions_are_search_ways_without_rewriting_the_predicate() {
        use rnl_runtime::problem::DirectedPin;
        let mut p = problem_of(
            vec![instance(), instance()],
            vec![
                fact(FactKind::PinOrient {
                    pin: DirectedPin { object: 0, pin: 2, direction: 270 },
                    codes: vec![Orient::R0],
                }),
                fact(FactKind::OrientRelation { relation: Orient::R0, from: vec![0], to: vec![1] }),
            ],
        );
        p.version = "0.4".into();
        let original = structure(&p);
        let s = for_layout(&original, &mut Work::default());
        assert_eq!(s.model.units, original.model.units);
        assert_eq!(s.witness, original.witness);
        assert!(s.open.classes.is_empty());
        assert_eq!(s.open.ways.len(), 1);
        assert_eq!(s.open.ways[0].count, 2);
        let mut tried = Vec::new();
        for value in 0..2 {
            let c = candidate(vec![], vec![value]);
            let proposed = c.orientations(&s);
            assert_eq!(proposed[0], proposed[1]);
            with_relations(&s, &orientation_relations(&s, &c), &mut Work::default(), |r, _, taken| {
                assert!(taken.kept.iter().all(|k| *k));
                assert!(r.holds(0) && r.holds(1));
                assert_eq!(r.geometry().orient, proposed);
            });
            tried.push(proposed[0]);
        }
        assert_ne!(tried[0], tried[1]);
        let mut capped = s.clone();
        capped.caps.nodes = 0;
        with_relations(
            &capped,
            &orientation_relations(&capped, &candidate(vec![], vec![1])),
            &mut Work::default(),
            |r, _, _| {
                assert!(r.holds(0) && r.holds(1));
            },
        );
    }
}
