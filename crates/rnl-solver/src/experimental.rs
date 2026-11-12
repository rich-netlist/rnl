//! Opt-in layout experiments for the independent solver program. These are not values of
//! SOLVER-CONFIG: the normal solve path and Viewer settings retain their own contract.

use crate::{Clock, LevelProblem, Solved, SolverOptions};

/// A preview position, with the authored PLACE locations that referred to this origin.
/// Locations are provenance, not instructions to rewrite a source file automatically.
#[derive(Clone, Debug, PartialEq)]
pub struct Moved {
    pub object: usize,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub source_places: Vec<rnl_runtime::diag::Loc>,
}

/// Compaction of an authored drawing. This intentionally has no ordinary Solution or kept
/// report: coordinate facts may have been released. It must not be sent to normal writeback.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompactedDrawing {
    /// False if the budget expired before the original drawing could be resolved and read.
    pub available: bool,
    pub positions: Vec<Moved>,
    pub relaxed_fact_indices: Vec<usize>,
    pub preserved_fact_indices: Vec<usize>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompactedPreview {
    pub drawing: CompactedDrawing,
    pub budget: crate::BudgetReport,
    pub canonical_options: String,
}

/// Resolve the original drawing first, then compact it without its retained PLACE and
/// PointCoordinate facts. Other retained facts and wire topology remain in force. The result
/// is an inspection preview, never a normal solve result. All phases share one budget.
pub fn compact_drawn_with_clock(
    problem: &LevelProblem,
    options: &SolverOptions,
    clock: &dyn Clock,
) -> Result<CompactedPreview, String> {
    if problem.version != "0.4" {
        return Err("the compaction preview requires an RNL 0.4 problem".into());
    }
    let (options, warnings) = options.for_problem(problem);
    options.validate()?;
    let (limit, safety) = crate::run_limits(&options, clock);
    let (mut drawing, budget) = crate::budget::without(|| {
        crate::budget::with(clock, limit, safety, || compact_drawn(problem, &options, clock))
    });
    drawing.warnings.extend(warnings);
    if drawing.positions.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return Err("compaction preview has nonfinite coordinates".into());
    }
    let replay = SolverOptions { run_limit: Some(crate::RunLimit::Work(budget.work)), time_limit: None, ..options };
    Ok(CompactedPreview {
        drawing,
        budget,
        canonical_options: format!("{};experimental=compact-drawn", replay.canonical()),
    })
}

fn compact_drawn(problem: &LevelProblem, options: &SolverOptions, clock: &dyn Clock) -> CompactedDrawing {
    use crate::{
        budget::{permit, Step},
        rational::{int, to_f64},
        route::Routing,
    };
    use rnl_runtime::problem::{FactKind, PointRef, Position};
    let resolved = crate::resolve::resolve(problem, options, clock);
    let mut out = CompactedDrawing {
        errors: resolved.errors.clone(),
        warnings: resolved.warnings.clone(),
        notes: resolved.notes.clone(),
        ..Default::default()
    };
    if crate::budget::stopped() {
        out.notes.push("compaction preview unavailable: the original drawing's resolution did not finish".into());
        return out;
    }
    let mut work = resolved.work;
    let structure = crate::structure::analyse(problem, &resolved, options, crate::options::CAPS, &mut work);
    let mut geometry = structure.witness.clone();
    // Use the same anchor origins, pin offsets and scene offsets as normal placement.
    for (&object, anchor) in &structure.anchors {
        if !permit(Step::Structure, anchor.from.len() + 1) {
            break;
        }
        for axis in 0..2 {
            if structure.objects[object].fact_axes[axis] {
                continue;
            }
            let (mut sum, mut count) = (int(0), 0);
            for &(owner, pin) in &anchor.from {
                let Some(origin) = crate::resolve::point_value(problem, &geometry, &PointRef::Origin(owner), axis, 0)
                else {
                    continue;
                };
                sum += origin;
                count += 1;
                if let Some(offset) = pin.and_then(|pin| structure.pin_offset(owner, pin, geometry.orient[owner])) {
                    sum += &offset[axis];
                }
            }
            if count > 0 {
                geometry.origin[object][axis] = sum / int(count) + &anchor.offset[axis];
            }
        }
    }
    if crate::budget::stopped() {
        out.notes.push("compaction preview unavailable: reading the original drawing did not finish".into());
        return out;
    }
    let placed = crate::coords::Placed {
        origins: structure.origins(problem, &geometry),
        geometry,
        dropped: vec![],
        overlaps: vec![],
        optimised: true,
        undecided: false,
    };
    let keep =
        |fact: usize| !matches!(problem.facts[fact].kind, FactKind::Place { .. } | FactKind::PointCoordinate { .. });
    let (made, _, notes) =
        crate::skeleton::compact(problem, &structure, &placed, &Routing::default(), &keep, options, &mut work);
    out.notes.extend(notes);
    out.relaxed_fact_indices = structure.kept.iter().copied().filter(|&f| !keep(f)).collect();
    out.preserved_fact_indices = structure.kept.iter().copied().filter(|&f| keep(f)).collect();
    // Collect provenance once, rather than rescanning every fact for each output object.
    let mut locations = std::collections::BTreeMap::<usize, Vec<rnl_runtime::diag::Loc>>::new();
    for fact in &problem.facts {
        if let (FactKind::Place { at: PointRef::Origin(object), .. }, Some(source)) = (&fact.kind, &fact.source) {
            locations.entry(*object).or_default().push(source.clone());
        }
    }
    for (object, item) in problem.objects.iter().enumerate() {
        if !matches!(item.position, Position::Own { .. }) || structure.objects[object].left_free {
            continue;
        }
        let mut source_places = locations.remove(&object).unwrap_or_default();
        source_places.sort_by(|a, b| (&a.file, a.line, a.col).cmp(&(&b.file, b.line, b.col)));
        source_places.dedup();
        out.positions.push(Moved {
            object,
            label: item.label.clone(),
            x: to_f64(&made.geometry.origin[object][0]),
            y: to_f64(&made.geometry.origin[object][1]),
            source_places,
        });
    }
    out.available = true;
    out
}

/// Maximum additional reroute-and-compact rounds after the initial compaction.
pub const SKELETON_ROUNDS: u8 = 3;

/// Start from a grouping-tree layout and compact its line identities while preserving kept
/// facts. The first compaction is taken even if its score is worse; further rounds must improve
/// the score with pure component-alignment layers ignored. All phases share one budget.
/// If the budget expires before compaction, the verified tree drawing is returned with a note.
pub fn solve_skeleton_with_clock(
    problem: &LevelProblem,
    options: &SolverOptions,
    rounds: u8,
    clock: &dyn Clock,
) -> Result<Solved, String> {
    if problem.version != "0.4" {
        return Err("the skeleton experiment requires an RNL 0.4 problem".into());
    }
    if rounds > SKELETON_ROUNDS {
        return Err(format!("skeleton rounds must be between 0 and {SKELETON_ROUNDS}"));
    }
    let mut solved = crate::budget::without(|| {
        crate::solve_isolated(problem, options, clock, |p, o, c| crate::driver::solve_skeleton(p, o, c, rounds))
    });
    if let Some(canonical) = &mut solved.report.canonical_options {
        canonical.push_str(&format!(";experimental=skeleton:{rounds}"));
    }
    Ok(solved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        resolve::tests::{fact, instance, problem_of, Ticks},
        RunLimit, StoppedClock,
    };
    use rnl_runtime::{
        previous::Drawn,
        problem::{Axis, FactKind, PointRef, Sense},
    };

    #[test]
    fn skeleton_shares_one_replayable_budget_and_retains_every_kept_fact() {
        let mut problem = problem_of(
            vec![instance(), instance(), instance()],
            vec![
                fact(FactKind::Align { axis: Axis::Y, members: (0..3).map(PointRef::Origin).collect() }),
                fact(FactKind::Distance {
                    axis: Axis::X,
                    sense: Sense::Increasing,
                    from: vec![PointRef::Origin(0)],
                    to: vec![PointRef::Origin(2)],
                    min: Some(10.),
                    max: None,
                }),
            ],
        );
        problem.version = "0.4".into();
        let options = SolverOptions {
            run_limit: Some(RunLimit::Seed),
            safety_seconds: None,
            ..SolverOptions::without_time_limit()
        };
        let complete = solve_skeleton_with_clock(&problem, &options, 1, &StoppedClock).unwrap();
        assert_eq!(complete.report.kept.len(), problem.facts.len());
        assert!(
            complete.report.notes.iter().any(|n| n.starts_with("skeleton, round 0: x pass")),
            "{:?}",
            complete.report.notes
        );
        assert!(complete.report.canonical_options.as_ref().unwrap().ends_with(";experimental=skeleton:1"));
        assert!(SolverOptions::default().with_config("layout=\"skeleton\"").is_err());
        assert!(solve_skeleton_with_clock(&problem, &options, 4, &StoppedClock).is_err());
        for seconds in [0., 1., 5., 20., 50., 100., 300., 1000.] {
            let options = SolverOptions { run_limit: Some(RunLimit::Time(seconds)), ..options.clone() };
            let timed = solve_skeleton_with_clock(&problem, &options, 1, &Ticks::default()).unwrap();
            let work = timed.report.budget.unwrap().work;
            let replay = solve_skeleton_with_clock(
                &problem,
                &SolverOptions { run_limit: Some(RunLimit::Work(work)), ..options },
                1,
                &StoppedClock,
            )
            .unwrap();
            assert_eq!(timed.solution, replay.solution, "cutoff {seconds}, work {work}");
            assert_eq!(timed.report.kept, replay.report.kept);
            assert_eq!(timed.report.canonical_options, replay.report.canonical_options);
            let mut drawn = vec![Drawn::default(); problem.objects.len()];
            for p in &timed.solution.placements {
                drawn[p.object] = Drawn { at: [p.x, p.y], orient: p.orient };
            }
            for &fact in &timed.report.kept {
                assert_eq!(crate::problem_fact_holds_as_drawn(&problem, &drawn, fact, 1e-6), Some(true));
            }
        }
    }

    #[test]
    fn a_cutoff_inside_compaction_replays_routed_writeback() {
        use rnl_runtime::problem::{input_digest, Reference, RouteIntent};
        let mut problem = problem_of(
            vec![instance(), instance()],
            vec![fact(FactKind::Distance {
                axis: Axis::X,
                sense: Sense::Increasing,
                min: Some(12.),
                max: None,
                from: vec![PointRef::Origin(0)],
                to: vec![PointRef::Origin(1)],
            })],
        );
        problem.version = "0.4".into();
        for (k, o) in problem.objects.iter_mut().enumerate() {
            o.reference = Reference::Name(format!("#R{}", k + 1));
        }
        problem.routes.push(RouteIntent {
            tree: true,
            net: "shared".into(),
            mode: "manhattan".into(),
            style: vec![],
            wires: vec![],
            pins: (0..2).map(|object| PointRef::Pin { object, pin: 0, terminal: "g".into() }).collect(),
        });
        let options = SolverOptions {
            run_limit: Some(RunLimit::Time(1_000_000.)),
            safety_seconds: None,
            ..SolverOptions::without_time_limit()
        };
        let clock = Ticks::default();
        let complete = solve_skeleton_with_clock(&problem, &options, 0, &clock).unwrap();
        assert!(!complete.solution.wires.is_empty());
        let end = clock.reads();
        let mut hit = false;
        for cutoff in end.saturating_sub(32)..end {
            let timed = solve_skeleton_with_clock(
                &problem,
                &SolverOptions { run_limit: Some(RunLimit::Time(cutoff as f64)), ..options.clone() },
                0,
                &Ticks::default(),
            )
            .unwrap();
            if timed.report.budget.unwrap().reason != crate::StopReason::Time
                || !timed
                    .report
                    .notes
                    .iter()
                    .any(|n| n.starts_with("skeleton, round 0:") && n.contains("pass") && n.contains("budget"))
            {
                continue;
            }
            let replay = solve_skeleton_with_clock(
                &problem,
                &SolverOptions {
                    run_limit: Some(RunLimit::Work(timed.report.budget.unwrap().work)),
                    ..options.clone()
                },
                0,
                &StoppedClock,
            )
            .unwrap();
            assert_eq!(timed.solution, replay.solution);
            assert_eq!(timed.report.canonical_options, replay.report.canonical_options);
            let written = |s: &Solved| {
                rnl_runtime::solution::write_record(
                    &problem,
                    &s.solution,
                    crate::AUTHOR,
                    &format!("{}:{}", input_digest(&problem), s.report.canonical_options.as_ref().unwrap()),
                )
            };
            let record = written(&timed);
            assert!(record.unsolved.is_empty(), "{:?}", record.unsolved);
            assert!(!record.record.is_empty());
            assert_eq!(record, written(&replay));
            hit = true;
            break;
        }
        assert!(hit, "exercise a budget interruption inside the compacted pass, not just the tree");
    }

    #[test]
    fn drawn_preview_resolves_coordinates_before_releasing_them() {
        let mut problem = problem_of(
            vec![instance(), instance()],
            vec![
                fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 0. }),
                fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::Y, value: 0. }),
                fact(FactKind::Place { at: PointRef::Origin(1), axis: Axis::X, value: 30. }),
                fact(FactKind::Place { at: PointRef::Origin(1), axis: Axis::Y, value: 0. }),
                fact(FactKind::Distance {
                    axis: Axis::X,
                    sense: Sense::Increasing,
                    min: Some(6.),
                    max: None,
                    from: vec![PointRef::Origin(0)],
                    to: vec![PointRef::Origin(1)],
                }),
                fact(FactKind::Align { axis: Axis::Y, members: vec![PointRef::Origin(0), PointRef::Origin(1)] }),
                // This loses to the original coordinates. Removing coordinates before
                // resolution would incorrectly keep it and expand the drawing to 40.
                fact(FactKind::Distance {
                    axis: Axis::X,
                    sense: Sense::Increasing,
                    min: Some(40.),
                    max: None,
                    from: vec![PointRef::Origin(0)],
                    to: vec![PointRef::Origin(1)],
                }),
            ],
        );
        problem.version = "0.4".into();
        problem.facts[0].source = Some(rnl_runtime::diag::Loc::line("drawing.sp", 10));
        let options = SolverOptions {
            run_limit: Some(RunLimit::Seed),
            safety_seconds: None,
            ..SolverOptions::without_time_limit()
        };
        let preview = compact_drawn_with_clock(&problem, &options, &StoppedClock).unwrap();
        assert!(preview.drawing.available);
        assert_eq!(preview.drawing.relaxed_fact_indices, [0, 1, 2, 3]);
        assert_eq!(preview.drawing.preserved_fact_indices, [4, 5]);
        assert_eq!(preview.drawing.positions.len(), 2);
        let a = &preview.drawing.positions[0];
        let b = &preview.drawing.positions[1];
        assert!(b.x - a.x >= 6. && b.x - a.x < 30.);
        assert_eq!(a.y, b.y);
        assert_eq!(a.source_places, [rnl_runtime::diag::Loc::line("drawing.sp", 10)]);
        for cutoff in [0., 1., 10., 50., 200., 1000.] {
            let options = SolverOptions { run_limit: Some(RunLimit::Time(cutoff)), ..options.clone() };
            let timed = compact_drawn_with_clock(&problem, &options, &Ticks::default()).unwrap();
            let replay = compact_drawn_with_clock(
                &problem,
                &SolverOptions { run_limit: Some(RunLimit::Work(timed.budget.work)), ..options },
                &StoppedClock,
            )
            .unwrap();
            assert_eq!(timed.drawing, replay.drawing);
            assert_eq!(timed.canonical_options, replay.canonical_options);
            if cutoff == 0. {
                assert!(!timed.drawing.available && timed.drawing.positions.is_empty());
            }
        }
    }
}
