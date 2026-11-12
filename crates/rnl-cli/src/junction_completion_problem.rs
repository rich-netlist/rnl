//! Extract conservative proposals from normally interpreted new-usage facts.
//! Proposals are never proof that a fact is retained; `verified_record` must reread and validate.
use super::junction_completion::{Axis, Point, Suggestion};
use rnl_runtime::problem::{Axis as ProblemAxis, FactKind, LevelProblem, PointRef, Sense};

fn axis(a: ProblemAxis) -> Axis {
    match a {
        ProblemAxis::X => Axis::X,
        ProblemAxis::Y => Axis::Y,
    }
}
fn value(p: Point, a: ProblemAxis) -> f64 {
    match a {
        ProblemAxis::X => p.x,
        ProblemAxis::Y => p.y,
    }
}

/// `fixed` must return a coordinate only for a point fixed by the first result, never a witness.
/// Unknown, set-centre, ambiguous or unrelated points produce no proposal. Closure checks and
/// actual 8.5 retention remain the verifier's responsibility.
pub fn suggestions<F>(problem: &LevelProblem, usage: usize, own: usize, mut fixed: F) -> Vec<Suggestion>
where
    F: FnMut(&PointRef) -> Option<Point>,
{
    let target = PointRef::Origin(own);
    let mut out = Vec::new();
    for fact in &problem.facts {
        if !fact.chain.contains(&usage) {
            continue;
        }
        match &fact.kind {
            FactKind::Align { axis: a, members } if members.len() == 2 => {
                let other = if members[0] == target {
                    Some(&members[1])
                } else if members[1] == target {
                    Some(&members[0])
                } else {
                    None
                };
                if let Some(p) = other.and_then(&mut fixed) {
                    out.push(Suggestion { axis: axis(*a), value: value(p, *a) });
                }
            }
            FactKind::Distance { axis: a, sense, min: Some(lo), max: Some(hi), from, to }
                if lo == hi && lo.is_finite() && from.len() == 1 && to.len() == 1 && *sense != Sense::Either =>
            {
                let sign = if *sense == Sense::Increasing { 1.0 } else { -1.0 };
                let candidate = if to[0] == target {
                    fixed(&from[0]).map(|p| value(p, *a) + sign * lo)
                } else if from[0] == target {
                    fixed(&to[0]).map(|p| value(p, *a) - sign * lo)
                } else {
                    None
                };
                if let Some(v) = candidate {
                    out.push(Suggestion { axis: axis(*a), value: v });
                }
            }
            _ => {}
        }
    }
    out
}

fn remap_point(point: &mut PointRef, map: &[usize]) -> Result<(), &'static str> {
    match point {
        PointRef::Origin(i) => *i = *map.get(*i).ok_or("invalid point index")?,
        PointRef::Pin { object, .. } | PointRef::SymbolPin { object, .. } | PointRef::PinPosition { object, .. } => {
            *object = *map.get(*object).ok_or("invalid pin owner index")?;
        }
        PointRef::Centre(points) => {
            for point in points {
                remap_point(point, map)?;
            }
        }
    }
    Ok(())
}
fn remap_fact(fact: &mut FactKind, map: &[usize]) -> Result<(), &'static str> {
    match fact {
        FactKind::Place { at, .. } => remap_point(at, map)?,
        FactKind::PointCoordinate { point, .. } => *point = *map.get(*point).ok_or("invalid point index")?,
        FactKind::Offset { text, from, .. } => {
            *text = *map.get(*text).ok_or("invalid text index")?;
            remap_point(from, map)?;
        }
        FactKind::Orient { object, .. } => *object = *map.get(*object).ok_or("invalid orient index")?,
        FactKind::Align { members, .. } | FactKind::Order { members, .. } | FactKind::Distribute { members, .. } => {
            for point in members {
                remap_point(point, map)?;
            }
        }
        FactKind::Distance { from, to, .. } => {
            for point in from.iter_mut().chain(to.iter_mut()) {
                remap_point(point, map)?;
            }
        }
        FactKind::PinOrient { pin, .. } => {
            pin.object = *map.get(pin.object).ok_or("invalid pin owner")?;
        }
        FactKind::PinOrientRelation { from, to, .. } => {
            from.object = *map.get(from.object).ok_or("invalid pin owner")?;
            to.object = *map.get(to.object).ok_or("invalid pin owner")?;
        }
        FactKind::OrientRelation { from, to, .. } => {
            for object in from.iter_mut().chain(to.iter_mut()) {
                *object = *map.get(*object).ok_or("invalid oriented owner")?;
            }
        }
        FactKind::Table { rows, .. } => {
            *rows = rows
                .try_map(&mut |point| {
                    let mut point = point.clone();
                    remap_point(&mut point, map).ok()?;
                    Some(point)
                })
                .ok_or("invalid TABLE point")?;
        }
    }
    Ok(())
}

fn equivalent_source(
    a: Option<&rnl_runtime::diag::Loc>,
    b: Option<&rnl_runtime::diag::Loc>,
    host_file: &str,
    line_map: &[Option<u32>],
) -> bool {
    let (Some(a), Some(b)) = (a, b) else {
        return a.is_none() && b.is_none();
    };
    if a.file != b.file || a.col != b.col {
        return false;
    }
    let mapped = if a.file == host_file { line_map.get(a.line as usize).copied().flatten() } else { Some(a.line) };
    mapped == Some(b.line)
}

/// Match retained first-pass facts after unique reference-based object remapping. This rejects
/// ambiguous objects and unsupported fact shapes rather than
/// guessing by label, coordinates, or runtime IDs, which are not stable across readings. Presence
/// does not prove 8.5 retention; an independent post-interpretation status check is still required.
pub fn retained_facts_preserved(
    original: &LevelProblem,
    trial: &LevelProblem,
    host_file: &str,
    line_map: &[Option<u32>],
    kept: &[usize],
) -> Result<(), &'static str> {
    retained_fact_indices(original, trial, host_file, line_map, kept).map(|_| ())
}

/// The result pairs original fact indices with their unique indices in this interpretation.
fn retained_fact_indices(
    original: &LevelProblem,
    trial: &LevelProblem,
    host_file: &str,
    line_map: &[Option<u32>],
    kept: &[usize],
) -> Result<Vec<(usize, usize)>, &'static str> {
    if kept.is_empty() {
        return Ok(Vec::new());
    }
    let mut mapping = Vec::new();
    for old in &original.objects {
        if original.objects.iter().filter(|other| other.reference == old.reference && other.kind == old.kind).count()
            != 1
        {
            return Err("original reference is ambiguous");
        }
        let mut matches =
            trial.objects.iter().enumerate().filter(|(_, new)| old.kind == new.kind && old.reference == new.reference);
        let (index, _) = matches.next().ok_or("original object missing in trial")?;
        if matches.next().is_some() {
            return Err("trial object reference is ambiguous");
        }
        mapping.push(index);
    }
    if original.usages.len() > trial.usages.len() {
        return Err("original usage disappeared");
    }
    for (old, new) in original.usages.iter().zip(&trial.usages) {
        if old.reference != new.reference
            || old.definition != new.definition
            || old.roles != new.roles
            || old.params != new.params
            || old.parent != new.parent
        {
            return Err("original usage identity mapping needs a richer adapter");
        }
    }
    let mut indices = Vec::with_capacity(kept.len());
    for &index in kept {
        let fact = original.facts.get(index).ok_or("invalid kept fact index")?;
        let mut kind = fact.kind.clone();
        remap_fact(&mut kind, &mapping)?;
        // Loc + kind + priority is not sufficient if it matches more than once; do not guess.
        let mut matches = trial.facts.iter().enumerate().filter(|(_, candidate)| {
            equivalent_source(fact.source.as_ref(), candidate.source.as_ref(), host_file, line_map)
                && candidate.priority == fact.priority
                && candidate.kind == kind
                && candidate.chain == fact.chain
        });
        let (trial_index, _) = matches.next().ok_or("retained fact missing or ambiguous in trial")?;
        if matches.next().is_some() {
            return Err("retained fact missing or ambiguous in trial");
        }
        indices.push((index, trial_index));
    }
    Ok(indices)
}

/// Every effective trial constraint not corresponding to an original fact must be checked,
/// including constraints created by external selection rules, irrespective of usage chain.
pub fn added_fact_indices(
    original: &LevelProblem,
    trial: &LevelProblem,
    host_file: &str,
    line_map: &[Option<u32>],
) -> Result<Vec<usize>, &'static str> {
    let mut map = Vec::new();
    for old in &original.objects {
        if original.objects.iter().filter(|o| o.kind == old.kind && o.reference == old.reference).count() != 1 {
            return Err("original object reference ambiguous");
        }
        let mut found =
            trial.objects.iter().enumerate().filter(|(_, o)| o.kind == old.kind && o.reference == old.reference);
        let (i, _) = found.next().ok_or("original object missing")?;
        if found.next().is_some() {
            return Err("trial object reference ambiguous");
        }
        map.push(i);
    }
    if original.usages.len() > trial.usages.len() {
        return Err("original usage missing");
    }
    for (a, b) in original.usages.iter().zip(&trial.usages) {
        if a.reference != b.reference
            || a.definition != b.definition
            || a.roles != b.roles
            || a.params != b.params
            || a.parent != b.parent
        {
            return Err("original usage identity mapping ambiguous");
        }
    }
    let mut old_kinds = Vec::new();
    for fact in &original.facts {
        let mut kind = fact.kind.clone();
        remap_fact(&mut kind, &map)?;
        old_kinds.push((fact, kind));
    }
    let mut added = Vec::new();
    for (index, candidate) in trial.facts.iter().enumerate() {
        let count = old_kinds
            .iter()
            .filter(|(fact, kind)| {
                equivalent_source(fact.source.as_ref(), candidate.source.as_ref(), host_file, line_map)
                    && fact.priority == candidate.priority
                    && *kind == candidate.kind
                    && fact.chain == candidate.chain
            })
            .count();
        if count > 1 {
            return Err("ambiguous old fact identity");
        }
        if count == 0 {
            added.push(index);
        }
    }
    Ok(added)
}

/// Coordinates from the same interpretation as `problem`, not `previous::drawn`, which
/// deliberately excludes anonymous objects belonging to the record being replaced.
fn current_drawn(p: &LevelProblem, read: &rnl_runtime::Interpretation) -> Vec<rnl_runtime::previous::Drawn> {
    use rnl_runtime::problem::{OrientSlot, Position, Slot};
    let level = read.model.levels.iter().find(|v| v.level == p.level);
    p.objects
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let id = p.ids.objects.get(i);
            let object = level.and_then(|v| id.and_then(|id| v.objects.iter().find(|x| &x.id == id)));
            let point = level.and_then(|v| id.and_then(|id| v.points.iter().find(|x| &x.id == id)));
            let at = object.and_then(|x| x.at).or_else(|| point.and_then(|x| x.at));
            let mut xy = at.map(|(x, y)| [Some(x), Some(y)]).unwrap_or([None, None]);
            if let Position::Own { x, y } = &o.position {
                for (axis, slot) in [x, y].into_iter().enumerate() {
                    if xy[axis].is_none() {
                        if let Slot::Fixed { value, .. } = slot {
                            xy[axis] = Some(*value);
                        }
                    }
                }
            }
            let orient = object.map(|x| x.orient).or(match &o.orient {
                Some(OrientSlot::Fixed { code, .. }) => Some(*code),
                _ => None,
            });
            rnl_runtime::previous::Drawn { at: xy, orient }
        })
        .collect()
}

/// Compare a normal generated=true interpretation before and after a trial PLACE. This is
/// deliberately stricter than the language: anything that needs general 8.5 resolution is
/// refused. The caller must identify the trial usage from writer provenance, not coordinates.
pub struct TrialValidation<'a> {
    pub original_problem: &'a LevelProblem,
    pub host_file: &'a str,
    pub line_map: &'a [Option<u32>],
    pub kept: &'a [usize],
    pub before: &'a rnl_runtime::Interpretation,
    pub after: &'a rnl_runtime::Interpretation,
    pub level: usize,
    pub targets: &'a [(String, Point)],
    pub added_facts: &'a [usize],
    pub tolerance: f64,
}

pub fn validate_interpreted_trial(input: TrialValidation<'_>) -> Result<(), &'static str> {
    let TrialValidation {
        original_problem,
        host_file,
        line_map,
        kept,
        before,
        after,
        level,
        targets,
        added_facts,
        tolerance,
    } = input;
    use rnl_runtime::diag::Severity;
    let left = before.model.levels.iter().find(|v| v.level == level).ok_or("missing initial level")?;
    let right = after.model.levels.iter().find(|v| v.level == level).ok_or("missing trial level")?;
    let trial_problem = after.problems.iter().find(|v| v.level == level).ok_or("missing trial problem")?;
    let before_problem = before.problems.iter().find(|v| v.level == level).ok_or("missing initial problem")?;
    let before_kept = retained_fact_indices(original_problem, before_problem, host_file, line_map, kept)?;
    let after_kept = retained_fact_indices(original_problem, trial_problem, host_file, line_map, kept)?;
    let before_drawn = current_drawn(before_problem, before);
    let drawn = current_drawn(trial_problem, after);
    for ((original_index, before_index), (after_original_index, after_index)) in before_kept.iter().zip(&after_kept) {
        if original_index != after_original_index {
            return Err("retained fact mapping changed");
        }
        let prior = rnl_solver::problem_fact_holds_as_drawn(before_problem, &before_drawn, *before_index, tolerance);
        let current = rnl_solver::problem_fact_holds_as_drawn(trial_problem, &drawn, *after_index, tolerance);
        if prior == Some(false) || current == Some(false) || (prior == Some(true) && current != Some(true)) {
            return Err("retained fact is false or became unresolved");
        }
    }
    for &index in added_facts {
        if rnl_solver::problem_fact_holds_as_drawn(trial_problem, &drawn, index, tolerance) != Some(true) {
            return Err("new or changed fact is unresolved or not satisfied");
        }
    }
    if after.failed.iter().any(|failed| !before.failed.contains(failed)) {
        return Err("trial newly failed or dropped a statement");
    }
    if after.diagnostics().iter().any(|d| d.severity == Severity::Error || d.severity == Severity::Violation) {
        return Err("trial interpretation has an error or violation");
    }
    for (usage_id, candidate) in targets {
        let mut selected = right.objects.iter().filter(|v| &v.id == usage_id);
        let dot = selected.next().ok_or("trial usage missing")?;
        let mut original_dots = left.objects.iter().filter(|v| &v.id == usage_id);
        let original_dot = original_dots.next().ok_or("original junction usage missing")?;
        if original_dots.next().is_some()
            || original_dot.definition != dot.definition
            || original_dot.attach != dot.attach
        {
            return Err("junction definition or role identity changed");
        }
        if selected.next().is_some() || dot.at != Some((candidate.x, candidate.y)) {
            return Err("trial usage identity or placement incorrect");
        }
    }
    for old in left.objects.iter().filter(|v| !targets.iter().any(|(id, _)| *id == v.id)) {
        let mut matches = right.objects.iter().filter(|v| v.id == old.id);
        let new = matches.next().ok_or("old object missing")?;
        if matches.next().is_some() || old.at != new.at || old.orient != new.orient || old.pins.len() != new.pins.len()
        {
            return Err("old object geometry changed");
        }
        for (a, b) in old.pins.iter().zip(&new.pins) {
            if a.at != b.at || a.dir != b.dir || a.terminals != b.terminals || a.nets != b.nets {
                return Err("old pin geometry or connection changed");
            }
            match (&a.identity, &b.identity) {
                (Some(a), Some(b)) if a.index == b.index && a.reference == b.reference => {}
                (None, None) => {}
                _ => return Err("physical pin identity changed"),
            }
        }
    }
    if left.points.len() != right.points.len() {
        return Err("named point count changed");
    }
    for old in &left.points {
        let mut matches = right.points.iter().filter(|point| point.id == old.id);
        let new = matches.next().ok_or("old point missing")?;
        if matches.next().is_some() || old.at != new.at || old.junction != new.junction {
            return Err("old point geometry or junction identity changed");
        }
    }
    if left.wires.len() != right.wires.len() {
        return Err("wire count changed");
    }
    for (a, b) in left.wires.iter().zip(&right.wires) {
        if a.object_id != b.object_id || a.nets != b.nets || a.points != b.points || a.via != b.via || a.mode != b.mode
        {
            return Err("wire geometry or endpoint identity changed");
        }
    }
    // Statuses are an additional outcome guard, never a mapping between model constraints and
    // solver facts. Consume baseline occurrences so new bad outcomes cannot reuse one old entry.
    let mut prior_statuses = std::collections::HashMap::new();
    for constraint in &left.constraints {
        if matches!(constraint.status.as_str(), "dropped" | "needs-solver" | "unsolved") {
            *prior_statuses
                .entry((&constraint.label, &constraint.kind, &constraint.priority, &constraint.status))
                .or_insert(0usize) += 1;
        }
    }
    for constraint in &right.constraints {
        if matches!(constraint.status.as_str(), "ok" | "disabled") {
            continue;
        }
        let key = (&constraint.label, &constraint.kind, &constraint.priority, &constraint.status);
        let Some(remaining) = prior_statuses.get_mut(&key) else {
            return Err("constraint outcome requires general resolution");
        };
        if *remaining == 0 {
            return Err("constraint outcome requires general resolution");
        }
        *remaining -= 1;
    }
    if right.unplaced.iter().any(|id| targets.iter().any(|(target, _)| target == id) || !left.unplaced.contains(id)) {
        return Err("junction or new generated object remains unplaced");
    }
    Ok(())
}
