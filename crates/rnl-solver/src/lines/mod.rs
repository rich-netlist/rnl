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

// A fixed estimate for tree refinement; the shared budget remains authoritative.
// It is deliberately independent of the requested seconds or work count.
fn tree_allowance(_problem: &rnl_runtime::problem::LevelProblem) -> u64 {
    5_000_000_000
}
