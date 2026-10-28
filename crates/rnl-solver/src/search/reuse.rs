//! Complete feasibility prefixes, private to one candidate's coordinate repair rounds.
//!
//! Resolver::new reads all variable and centre definitions; these must be identical, including
//! numbering. Unit reads during add are restricted to the current unit and those already tried:
//! commit/holds read the current unit, rebuild reads kept, and literals/held_bounds read choices
//! of kept/current units. Rejected and undecided adds can leave rows and tags behind, so their
//! units also belong to the checked prefix. No future unit has been read at these boundaries.
//! After that proof, an owned full state may be rebound to the new model. No backtrack is used.

use crate::options::Caps;
use crate::profile::{StateReuseHandle, StateReuseReport};
use crate::resolve::ResolverSnapshot;

use super::*;

struct Prefix {
    len: usize,
    outcomes: Vec<Outcome>,
    state: ResolverSnapshot,
}

struct Previous {
    model: Model,
    base: usize,
    seed: Geometry,
    kept: Vec<usize>,
    caps: Caps,
    objects: usize,
    relations: Vec<Tagged>,
    prefixes: Vec<Prefix>,
}

/// At most three stored boundaries from the previous round, never another candidate. The old
/// and new lists can coexist while saving, so this is a retained-count bound, not a peak bound.
#[derive(Default)]
pub(crate) struct RelationReuse {
    previous: Option<Previous>,
    observer: Option<StateReuseHandle>,
    #[cfg(test)]
    last: StateReuseReport,
}

impl RelationReuse {
    pub(crate) fn new() -> Self {
        Self { observer: crate::profile::state_reuse_observer(), ..Self::default() }
    }

    pub(crate) fn with_relations<R>(
        &mut self,
        s: &Structure,
        relations: &[Tagged],
        work: &mut Work,
        f: impl FnOnce(&mut Resolver, &Model, &Taken) -> R,
    ) -> R {
        // Compile everything in the original order, even on a hit. A suffix that changes any
        // preallocated variable definition or number invalidates every old prefix.
        let mut model = s.model.clone();
        let base = model.units.len();
        let units: Vec<Unit> = relations.iter().enumerate().map(|(i, t)| t.rel.unit(&mut model, base + i)).collect();
        model.units.extend(units);
        let previous = self.previous.take();
        let mut report = StateReuseReport { total_relations: relations.len(), ..StateReuseReport::default() };
        let selected = previous.as_ref().and_then(|p| {
            let compatible = if p.base != base
                || p.caps != s.caps
                || p.objects != s.objects.len()
                || p.kept != s.kept
                || p.seed != s.witness
            {
                Err("context")
            } else if !p.model.same_definitions(&model) {
                Err("model_definitions")
            } else if p.model.units[..base] != model.units[..base] {
                Err("base_units")
            } else {
                Ok(())
            };
            if let Err(reason) = compatible {
                report.miss_reason = reason;
                return None;
            }
            let common = p
                .relations
                .iter()
                .zip(relations)
                .enumerate()
                .take_while(|(i, (a, b))| a == b && p.model.units[base + i] == model.units[base + i])
                .count();
            let pick = p.prefixes.iter().enumerate().filter(|(_, p)| p.len <= common).max_by_key(|(_, p)| p.len);
            if let Some((index, prefix)) = pick {
                report.hit = true;
                report.reused_relations = prefix.len;
                Some(index)
            } else {
                report.miss_reason = "attempted_prefix";
                None
            }
        });
        if previous.is_none() {
            report.miss_reason = "empty";
        }
        let boundaries = boundaries(relations);
        let (mut r, mut outcomes, mut prefixes) = match (previous, selected) {
            (Some(p), Some(index)) => {
                let prefix = &p.prefixes[index];
                let start = now(&self.observer);
                let r = prefix.state.restore(&model);
                let outcomes = prefix.outcomes.clone();
                report.restore_ns = elapsed_ns(&self.observer, start);
                report.logical_prefix_nodes = r.nodes;
                report.logical_prefix_pivots = r.pivots();
                let len = prefix.len;
                let prefixes = p.prefixes.into_iter().filter(|p| p.len <= len && boundaries.contains(&p.len)).collect();
                (r, outcomes, prefixes)
            }
            _ => {
                let mut r = Resolver::new(&model, s.objects.len(), Limits::Caps(s.caps));
                r.seed(s.witness.clone());
                for &fact in &s.kept {
                    let kept = r.add(fact);
                    debug_assert_eq!(kept, Outcome::Kept, "the facts kept hold at the witness");
                }
                (r, Vec::new(), Vec::new())
            }
        };
        // Snapshot only stable semantic boundaries. The consumer below uses r, independent of
        // these deep copies; its objective and grid rows are never saved for the next round.
        save_if_boundary(&r, &outcomes, &boundaries, &mut prefixes, &self.observer, &mut report);
        for i in outcomes.len()..relations.len() {
            outcomes.push(r.add(base + i));
            save_if_boundary(&r, &outcomes, &boundaries, &mut prefixes, &self.observer, &mut report);
        }
        let start = now(&self.observer);
        self.previous = Some(Previous {
            model: model.clone(),
            base,
            seed: s.witness.clone(),
            kept: s.kept.clone(),
            caps: s.caps,
            objects: s.objects.len(),
            relations: relations.to_vec(),
            prefixes,
        });
        report.model_clone_ns = elapsed_ns(&self.observer, start);
        let taken = Taken {
            kept: outcomes.iter().map(|o| *o == Outcome::Kept).collect(),
            undecided: outcomes.contains(&Outcome::Undecided),
        };
        let out = f(&mut r, &model, &taken);
        // These are the original logical costs, including the reused prefix. New Search
        // instances still start at zero; each simplex retains its original accumulated limit.
        work.resolve_nodes += r.nodes;
        work.pivots += r.pivots();
        if let Some(observer) = &self.observer {
            observer.record(report.clone());
        }
        #[cfg(test)]
        {
            self.last = report;
        }
        out
    }
}

fn boundaries(relations: &[Tagged]) -> Vec<usize> {
    let early = relations.iter().take_while(|r| matches!(r.kind, RelKind::Orientation | RelKind::Separation)).count();
    let rows = relations.iter().position(|r| r.kind == RelKind::RowsApart).unwrap_or(relations.len());
    let mut out = vec![early, rows, relations.len()];
    out.sort_unstable();
    out.dedup();
    out
}

fn save_if_boundary(
    r: &Resolver,
    outcomes: &[Outcome],
    boundaries: &[usize],
    prefixes: &mut Vec<Prefix>,
    observer: &Option<StateReuseHandle>,
    report: &mut StateReuseReport,
) {
    let len = outcomes.len();
    if !boundaries.contains(&len) || prefixes.iter().any(|p| p.len == len) {
        return;
    }
    let start = now(observer);
    let state = r.snapshot().expect("coordinate relations always have fixed caps");
    let outcomes = outcomes.to_vec();
    report.save_ns += elapsed_ns(observer, start);
    report.save_clones += 1;
    prefixes.push(Prefix { len, outcomes, state });
}

fn now(observer: &Option<StateReuseHandle>) -> Option<f64> {
    observer.as_ref().map(|o| o.elapsed())
}

fn elapsed_ns(observer: &Option<StateReuseHandle>, start: Option<f64>) -> u64 {
    match (observer, start) {
        (Some(observer), Some(start)) => ((observer.elapsed() - start).max(0.0) * 1e9) as u64,
        _ => 0,
    }
}

#[cfg(test)]
mod tests;
