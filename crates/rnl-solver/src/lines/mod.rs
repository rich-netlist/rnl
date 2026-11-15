//! Hierarchical line layout. Candidate ordering and coordinate compaction share one budget.

mod axis;
mod band;
mod cost;
mod current;
mod distribution;
mod dots;
pub(crate) mod engine;
mod facing;
mod fingerprint;
mod open;
mod port;
mod quality;
mod rank;
pub(crate) use rank::Rank;
mod rule;
mod settle;
mod supply;
mod track;
pub(crate) mod written;

fn tree_start(
    problem: &rnl_runtime::problem::LevelProblem,
    s: &crate::structure::Structure,
    options: &crate::SolverOptions,
    work: &mut crate::Work,
    stop: &crate::tree::Stop,
    allowance: Option<crate::tree::Allowance>,
) -> crate::tree::Laid {
    crate::tree::lay_out_from(crate::tree::Tree::new(problem, s, options).with_line_rank(), work, stop, allowance)
}

pub(crate) fn lay_out(
    problem: &rnl_runtime::problem::LevelProblem,
    s: &crate::structure::Structure,
    options: &crate::SolverOptions,
    work: &mut crate::Work,
) -> (crate::tree::Laid, engine::LayoutSkeleton) {
    let limit = crate::budget::current_limit().or(options.run_limit);
    let allowance = tree_allowance(limit, options.tree_work);
    let (mut laid, mut skeleton) = engine::lay_out(problem, s, options, work, &crate::tree::Stop::default(), allowance);
    if laid.entry.written_path_error.is_none() && laid.entry.annotation_error.is_none() && !crate::budget::stopped() {
        let before = laid.placed.geometry.clone();
        let saved = (!problem.annotations.is_empty()).then(|| (laid.placed.clone(), laid.routing.clone()));
        (laid.placed, laid.routing) =
            port::to_edge(problem, s, laid.placed, laid.routing, options.tolerance, &mut laid.notes);
        if let Some((placed, routing)) = saved {
            if !quality::annotations_preserved(problem, s, (&placed, &routing), (&laid.placed, &laid.routing)) {
                (laid.placed, laid.routing) = (placed, routing);
                laid.notes
                    .push("lines: port cleanup would worsen annotation clearance; previous drawing retained".into());
            }
        }
        if laid.placed.geometry != before {
            laid.entry = engine::entry(problem, s, &laid.placed, &laid.routing, &laid.entry.code);
            engine::refresh(problem, s, &laid, &mut skeleton);
        }
    }
    if let Some(note) = current::unmet(problem, s, &laid.placed) {
        laid.notes.push(note);
    }
    if written::defaults(problem, s, &laid.placed, &mut laid.routing, None, true) {
        laid.entry = engine::entry(problem, s, &laid.placed, &laid.routing, &laid.entry.code);
        engine::refresh(problem, s, &laid, &mut skeleton);
    }
    (laid, skeleton)
}

/// The most the group-tree start refines, in the solver's estimate of nanoseconds, whatever the
/// limit: the shared budget still bounds it.
const TREE_NS: u64 = 5_000_000_000;

/// How far the group-tree start refines its arrangement before the line search starts
/// (`crate::tree::Allowance`), fixed before it starts: the work `tree_work` gives, or none with
/// `tree_work=false`; otherwise three tenths of the level's limit, in the estimate of nanoseconds
/// for a limit of time and in work for a limit of work, and never past [`TREE_NS`], so that the
/// line search has the rest of the budget whatever the limit is. A seed or no limit refines without
/// an allowance. A solve reports where it stopped (`crate::budget::tree_stop`), so that a run of
/// time replays alike by its work.
fn tree_allowance(limit: Option<crate::RunLimit>, given: Option<crate::TreeWork>) -> Option<crate::tree::Allowance> {
    use crate::tree::Allowance;
    use crate::{RunLimit, TreeWork};
    match (limit, given) {
        (Some(RunLimit::Seed | RunLimit::Unlimited), _) | (_, Some(TreeWork::Off)) => None,
        (_, Some(TreeWork::At(w))) => Some(Allowance { ns: u64::MAX, work: Some(w) }),
        (Some(RunLimit::Time(s)), None) => {
            let share = (s.max(0.) * 0.3e9).min(TREE_NS as f64) as u64;
            Some(Allowance { ns: share, work: None })
        }
        (Some(RunLimit::Work(n)), None) => Some(Allowance { ns: TREE_NS, work: Some((u128::from(n) * 3 / 10) as u64) }),
        (None, None) => Some(Allowance { ns: TREE_NS, work: None }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Allowance;
    use crate::{RunLimit, TreeWork};

    /// Three tenths of the level's limit, never past the fixed estimate; the work `tree_work`
    /// gives over anything the limit would; none with a seed, no limit or `tree_work=false`.
    #[test]
    fn the_tree_allowance_is_a_share_of_the_limit_or_the_work_given() {
        let ns = |ns| Some(Allowance { ns, work: None });
        assert_eq!(tree_allowance(Some(RunLimit::Time(10.)), None), ns(3_000_000_000));
        assert_eq!(tree_allowance(Some(RunLimit::Time(2.5)), None), ns(750_000_000));
        assert_eq!(tree_allowance(Some(RunLimit::Time(60.)), None), ns(TREE_NS));
        assert_eq!(
            tree_allowance(Some(RunLimit::Work(340_000_000_000)), None),
            Some(Allowance { ns: TREE_NS, work: Some(102_000_000_000) })
        );
        assert_eq!(tree_allowance(None, None), ns(TREE_NS));
        let at = Some(Allowance { ns: u64::MAX, work: Some(7) });
        assert_eq!(tree_allowance(Some(RunLimit::Time(10.)), Some(TreeWork::At(7))), at);
        assert_eq!(tree_allowance(Some(RunLimit::Work(9)), Some(TreeWork::At(7))), at);
        assert_eq!(tree_allowance(Some(RunLimit::Work(9)), Some(TreeWork::Off)), None);
        assert_eq!(tree_allowance(Some(RunLimit::Seed), None), None);
        assert_eq!(tree_allowance(Some(RunLimit::Unlimited), Some(TreeWork::At(7))), None);
    }
}
