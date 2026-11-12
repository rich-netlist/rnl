//! Hierarchical line layout. Candidate ordering and coordinate compaction share one budget.

mod axis;
mod band;
mod cost;
mod current;
mod dots;
pub(crate) mod engine;
mod facing;
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
    allowance: Option<u64>,
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
    let allowance = match limit {
        Some(crate::RunLimit::Seed | crate::RunLimit::Unlimited) => None,
        _ => Some(tree_allowance(problem)),
    };
    let (mut laid, mut skeleton) = engine::lay_out(problem, s, options, work, &crate::tree::Stop::default(), allowance);
    if laid.entry.written_path_error.is_none() && !crate::budget::stopped() {
        let before = laid.placed.geometry.clone();
        (laid.placed, laid.routing) =
            port::to_edge(problem, s, laid.placed, laid.routing, options.tolerance, &mut laid.notes);
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

// A versioned input-size policy, independent of the requested seconds or total work.
// Optimized-build measurements and the replay contract are documented in solver-lines.md.
fn tree_allowance(problem: &rnl_runtime::problem::LevelProblem) -> u64 {
    tree_allowance_for_size(problem.objects.len() as u64)
}

fn tree_allowance_for_size(objects: u64) -> u64 {
    let n = objects.max(1);
    40_000_000u64.saturating_mul(n).saturating_mul(n).saturating_add(2_500_000_000u64.saturating_mul(n))
}

#[cfg(test)]
mod tests {
    #[test]
    fn tree_phase_work_policy_is_fixed_and_saturates_without_wrapping() {
        for (objects, expected) in [
            (0, 2_540_000_000),
            (1, 2_540_000_000),
            (8, 22_560_000_000),
            (16, 50_240_000_000),
            (64, 323_840_000_000),
            (u64::MAX, u64::MAX),
        ] {
            assert_eq!(super::tree_allowance_for_size(objects), expected);
        }
    }
}
