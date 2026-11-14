//! Extract conservative proposals from normally interpreted new-usage facts.
//! Proposals are never proof that a fact is retained; `verified_record` must reread and validate.
use super::junction_completion::{Axis, Point, Suggestion};
use rnl_runtime::problem::{Axis as ProblemAxis, FactKind, Filler, LevelProblem, PointRef, Reference, Sense};

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

fn mapped_index(map: &[usize], i: usize) -> Result<usize, &'static str> {
    map.get(i).copied().filter(|i| *i != usize::MAX).ok_or("referenced original object is not mapped")
}

pub(crate) fn rebased_reference(reference: &Reference, old: &Reference, new: &Reference) -> Option<Reference> {
    if old == new {
        return Some(reference.clone());
    }
    let suffix: Vec<String> = match (reference, old) {
        (Reference::Name(a), Reference::Name(b)) => {
            let tail = a.strip_prefix(b)?;
            if !tail.is_empty() && !tail.starts_with(':') {
                return None;
            }
            vec![tail.to_owned()]
        }
        (Reference::Number { number: a, path: ap }, Reference::Number { number: b, path: bp })
            if a == b && ap.starts_with(bp) =>
        {
            ap[bp.len()..].to_vec()
        }
        (Reference::Generated { usage: a, path: ap }, Reference::Generated { usage: b, path: bp })
            if a == b && ap.starts_with(bp) =>
        {
            ap[bp.len()..].to_vec()
        }
        (
            Reference::GeneratedNumber { usage: a, index: ai, path: ap },
            Reference::GeneratedNumber { usage: b, index: bi, path: bp },
        ) if a == b && ai == bi && ap.starts_with(bp) => ap[bp.len()..].to_vec(),
        _ => return None,
    };
    Some(suffix.iter().fold(new.clone(), |reference, part| reference.then(part)))
}

fn remap_point(
    point: &mut PointRef,
    map: &[usize],
    original: &LevelProblem,
    trial: &LevelProblem,
) -> Result<(), &'static str> {
    match point {
        PointRef::Origin(i) => *i = mapped_index(map, *i)?,
        PointRef::Pin { object, .. } => {
            *object = mapped_index(map, *object)?;
        }
        PointRef::SymbolPin { object, reference, .. } | PointRef::PinPosition { object, reference, .. } => {
            let old = original.objects.get(*object).ok_or("invalid original pin owner")?;
            let mapped = mapped_index(map, *object)?;
            let new = trial.objects.get(mapped).ok_or("invalid mapped pin owner")?;
            *reference = rebased_reference(reference, &old.reference, &new.reference)
                .ok_or("pin reference does not belong to its mapped owner")?;
            *object = mapped;
        }
        PointRef::Centre(points) => {
            for point in points {
                remap_point(point, map, original, trial)?;
            }
        }
    }
    Ok(())
}
fn remap_fact(
    fact: &mut FactKind,
    map: &[usize],
    original: &LevelProblem,
    trial: &LevelProblem,
) -> Result<(), &'static str> {
    match fact {
        FactKind::Place { at, .. } => remap_point(at, map, original, trial)?,
        FactKind::PointCoordinate { point, .. } => *point = mapped_index(map, *point)?,
        FactKind::Offset { text, from, .. } => {
            *text = mapped_index(map, *text)?;
            remap_point(from, map, original, trial)?;
        }
        FactKind::Orient { object, .. } => *object = mapped_index(map, *object)?,
        FactKind::Align { members, .. } | FactKind::Order { members, .. } | FactKind::Distribute { members, .. } => {
            for point in members {
                remap_point(point, map, original, trial)?;
            }
        }
        FactKind::Distance { from, to, .. } => {
            for point in from.iter_mut().chain(to.iter_mut()) {
                remap_point(point, map, original, trial)?;
            }
        }
        FactKind::PinOrient { pin, .. } => {
            pin.object = mapped_index(map, pin.object)?;
        }
        FactKind::PinOrientRelation { from, to, .. } => {
            from.object = mapped_index(map, from.object)?;
            to.object = mapped_index(map, to.object)?;
        }
        FactKind::OrientRelation { from, to, .. } => {
            for object in from.iter_mut().chain(to.iter_mut()) {
                *object = mapped_index(map, *object)?;
            }
        }
        FactKind::Table { rows, .. } => {
            *rows = rows
                .try_map(&mut |point| {
                    let mut point = point.clone();
                    remap_point(&mut point, map, original, trial).ok()?;
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

struct IdentityMapping {
    objects: Vec<usize>,
    usages: Vec<usize>,
    reified: Vec<bool>,
}

fn unique_ids(ids: &[Option<String>], current: &[String]) -> Result<Vec<usize>, &'static str> {
    let mut used = std::collections::BTreeSet::new();
    ids.iter()
        .map(|id| {
            // Unreferenced symbol text can disappear when a generated usage is
            // written out. Any fact or role that needs it must still reject it.
            let Some(id) = id.as_ref() else { return Ok(usize::MAX) };
            let mut found = current.iter().enumerate().filter(|(_, candidate)| *candidate == id);
            let (index, _) = found.next().ok_or("original identity missing in trial")?;
            if found.next().is_some() || !used.insert(index) {
                return Err("original identity is ambiguous in trial");
            }
            Ok(index)
        })
        .collect()
}

fn same_params(old: &rnl_runtime::problem::UsageEntry, new: &rnl_runtime::problem::UsageEntry) -> bool {
    // String unit IDs belong to one interpretation. Compare the literal values
    // carried by the solver protocol for implicit and materialized usages alike.
    old.params.len() == new.params.len()
        && old.params.iter().zip(&new.params).all(|((a, x), (b, y))| {
            a == b && rnl_runtime::problem::written_lit(x) == rnl_runtime::problem::written_lit(y)
        })
}

fn identity_mapping(
    original: &LevelProblem,
    trial: &LevelProblem,
    read: &rnl_runtime::Interpretation,
    host_file: &str,
    line_map: &[Option<u32>],
) -> Result<IdentityMapping, &'static str> {
    // Runtime IDs are used only after the core maps original references into this
    // particular reading. Generated usages are reified under their written names.
    let ids = rnl_runtime::previous::document_ids(original, read);
    let mut objects = unique_ids(&ids.objects, &trial.ids.objects)?;
    let mut usages = unique_ids(&ids.usages, &trial.ids.usages)?;
    // A rule usage not written into the generated record can still exist in both
    // readings. Its source, semantic parent and typed role identities must prove it.
    // In particular, Other is display text, not an identity.
    for _ in 0..original.usages.len().saturating_add(1) {
        let mut changed = false;
        for (i, old) in original.usages.iter().enumerate() {
            if usages[i] != usize::MAX || !old.rule_made || old.source.is_none() {
                continue;
            }
            let role_matches = |a: &Filler, b: &Filler| match (a, b) {
                (Filler::Object(x), Filler::Object(y)) => objects.get(*x) == Some(y),
                (Filler::Usage(x), Filler::Usage(y)) => usages.get(*x) == Some(y),
                (Filler::Pin { object: x, pin: xp }, Filler::Pin { object: y, pin: yp }) => {
                    objects.get(*x) == Some(y) && xp == yp
                }
                (
                    Filler::Terminal { object: x, terminal: xt, pin: xp },
                    Filler::Terminal { object: y, terminal: yt, pin: yp },
                ) => objects.get(*x) == Some(y) && xt == yt && xp == yp,
                (Filler::Net(x), Filler::Net(y)) => x == y,
                _ => false,
            };
            let mut found = trial.usages.iter().enumerate().filter(|(j, new)| {
                !usages.contains(j)
                    && new.rule_made
                    && equivalent_source(old.source.as_ref(), new.source.as_ref(), host_file, line_map)
                    && old.definition == new.definition
                    && same_params(old, new)
                    && old.priority == new.priority
                    && old.parent.map(|p| usages.get(p).copied()) == new.parent.map(Some)
                    && old.roles.len() == new.roles.len()
                    && old.roles.iter().zip(&new.roles).all(|(a, b)| {
                        a.role == b.role
                            && a.members.len() == b.members.len()
                            && a.members.iter().zip(&b.members).all(|(a, b)| role_matches(a, b))
                    })
            });
            let Some((j, new)) = found.next() else { continue };
            if found.next().is_some() {
                continue;
            }
            if let (Some(a), Some(b)) = (old.object, new.object) {
                if original.objects.get(a).zip(trial.objects.get(b)).is_none_or(|(x, y)| x.kind != y.kind)
                    || (objects[a] != usize::MAX && objects[a] != b)
                    || objects.iter().enumerate().any(|(k, x)| k != a && *x == b)
                {
                    continue;
                }
                objects[a] = b;
            } else if old.object.is_some() != new.object.is_some() {
                continue;
            }
            usages[i] = j;
            changed = true;
        }
        if !changed {
            break;
        }
    }
    if usages.contains(&usize::MAX) {
        return Err("original usage identity is not mapped");
    }
    let reified: Vec<_> =
        original.usages.iter().zip(&usages).map(|(old, &i)| old.rule_made && !trial.usages[i].rule_made).collect();
    for (old, &index) in original.objects.iter().zip(&objects) {
        if index == usize::MAX {
            continue;
        }
        if old.kind != trial.objects[index].kind {
            return Err("original object kind changed");
        }
    }
    let mapped_filler = |f: &Filler| -> Result<Filler, &'static str> {
        Ok(match f {
            Filler::Object(i) => Filler::Object(mapped_index(&objects, *i)?),
            Filler::Usage(i) => Filler::Usage(*usages.get(*i).ok_or("invalid role usage")?),
            Filler::Terminal { object, terminal, pin } => {
                Filler::Terminal { object: mapped_index(&objects, *object)?, terminal: terminal.clone(), pin: *pin }
            }
            Filler::Pin { object, pin } => Filler::Pin { object: mapped_index(&objects, *object)?, pin: *pin },
            Filler::Wire(i) => {
                let old = original.wires.get(*i).ok_or("invalid role wire")?;
                let mut matching = trial.wires.iter().enumerate().filter(|(_, w)| w.reference == old.reference);
                let (index, _) = matching.next().ok_or("role wire missing")?;
                if matching.next().is_some() {
                    return Err("role wire ambiguous");
                }
                Filler::Wire(index)
            }
            Filler::Net(_) | Filler::Other(_) => f.clone(),
        })
    };
    for ((old, &index), &reified) in original.usages.iter().zip(&usages).zip(&reified) {
        let new = &trial.usages[index];
        // A rule-made usage is written as a top-level ATTACH. Its body still
        // retains its definition, parameters, roles and nested usage identities.
        let parent = if reified {
            None
        } else {
            old.parent.map(|i| usages.get(i).copied().ok_or("invalid parent usage")).transpose()?
        };
        let object = old.object.map(|i| mapped_index(&objects, i)).transpose()?;
        if old.definition != new.definition
            || !same_params(old, new)
            || parent != new.parent
            || object != new.object
            || old.roles.len() != new.roles.len()
        {
            return Err("original usage identity changed");
        }
        for (a, b) in old.roles.iter().zip(&new.roles) {
            let members = a.members.iter().map(&mapped_filler).collect::<Result<Vec<_>, _>>()?;
            if a.role != b.role || members != b.members {
                return Err("original usage role changed");
            }
        }
    }
    Ok(IdentityMapping { objects, usages, reified })
}

fn mapped_chain(chain: &[usize], map: &IdentityMapping) -> Result<Vec<usize>, &'static str> {
    let start = chain.iter().rposition(|i| map.reified.get(*i) == Some(&true)).unwrap_or(0);
    chain[start..].iter().map(|i| map.usages.get(*i).copied().ok_or("invalid fact usage chain")).collect()
}

/// Prove the first-pass kept facts against the interpreted drawing. Reifying a rule-made
/// usage can empty its former local collection, so presence in the new expanded fact list
/// is not a retention proof. The original authored records are unchanged by this transaction.
pub fn retained_facts_preserved(
    original: &LevelProblem,
    trial: &LevelProblem,
    host_file: &str,
    line_map: &[Option<u32>],
    kept: &[usize],
    read: &rnl_runtime::Interpretation,
    tolerance: f64,
) -> Result<(), &'static str> {
    if kept.is_empty() {
        return Ok(());
    }
    let mapping = identity_mapping(original, trial, read, host_file, line_map)?;
    let mut drawn = rnl_runtime::previous::drawn(original, read);
    let ids = rnl_runtime::previous::document_ids(original, read);
    let trial_drawn = current_drawn(trial, read);
    for (i, &mapped) in mapping.objects.iter().enumerate() {
        // A still-implicit usage has no materialized name for previous::drawn.
        // Use its actual trial drawing only after identity_mapping proved its
        // source, parent and typed role identities; never match coordinates.
        if ids.objects.get(i) == Some(&None) {
            if let Some(at) = trial_drawn.get(mapped) {
                let old_symbol = original.objects[i].symbol.and_then(|s| original.symbols.get(s));
                let new_symbol = trial.objects[mapped].symbol.and_then(|s| trial.symbols.get(s));
                if old_symbol != new_symbol {
                    return Err("implicit object's symbol changed in trial");
                }
                drawn[i] = at.clone();
            }
        }
    }
    for &index in kept {
        let fact = original.facts.get(index).ok_or("invalid kept fact index")?;
        let mut kind = fact.kind.clone();
        remap_fact(&mut kind, &mapping.objects, original, trial)?;
        let chain = mapped_chain(&fact.chain, &mapping)?;
        // Loc + kind + priority is not sufficient if it matches more than once; do not guess.
        let matches = trial
            .facts
            .iter()
            .filter(|candidate| {
                equivalent_source(fact.source.as_ref(), candidate.source.as_ref(), host_file, line_map)
                    && candidate.priority == fact.priority
                    && candidate.kind == kind
                    && candidate.chain == chain
            })
            .count();
        if matches > 1 {
            return Err("retained fact ambiguous in trial");
        }
        if rnl_solver::problem_fact_holds_as_drawn(original, &drawn, index, tolerance) != Some(true) {
            return Err("retained fact is unresolved or not satisfied in the drawing");
        }
    }
    Ok(())
}

/// Every effective trial constraint not corresponding to an original fact must be checked,
/// including constraints created by external selection rules, irrespective of usage chain.
pub fn added_fact_indices(
    original: &LevelProblem,
    trial: &LevelProblem,
    host_file: &str,
    line_map: &[Option<u32>],
    read: &rnl_runtime::Interpretation,
) -> Result<Vec<usize>, &'static str> {
    let map = identity_mapping(original, trial, read, host_file, line_map)?;
    let mut old_kinds = Vec::new();
    for fact in &original.facts {
        let mut kind = fact.kind.clone();
        remap_fact(&mut kind, &map.objects, original, trial)?;
        old_kinds.push((fact, kind, mapped_chain(&fact.chain, &map)?));
    }
    let mut added = Vec::new();
    for (index, candidate) in trial.facts.iter().enumerate() {
        let count = old_kinds
            .iter()
            .filter(|(fact, kind, chain)| {
                equivalent_source(fact.source.as_ref(), candidate.source.as_ref(), host_file, line_map)
                    && fact.priority == candidate.priority
                    && *kind == candidate.kind
                    && *chain == candidate.chain
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
    pub original: &'a rnl_runtime::Interpretation,
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

pub fn validate_interpreted_trial(input: TrialValidation<'_>) -> Result<(), String> {
    let TrialValidation {
        original,
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
    retained_facts_preserved(original_problem, trial_problem, host_file, line_map, kept, after, tolerance)?;
    let drawn = current_drawn(trial_problem, after);
    for &index in added_facts {
        if rnl_solver::problem_fact_holds_as_drawn(trial_problem, &drawn, index, tolerance) != Some(true) {
            return Err(format!(
                "new or changed fact is unresolved or not satisfied: {}",
                rnl_runtime::problem::fact_statement(trial_problem, index)
            ));
        }
    }
    if after.failed.iter().any(|failed| !before.failed.contains(failed)) {
        return Err("trial newly failed or dropped a statement".into());
    }
    // Assertions in the input may already report a violation without failing a
    // statement, and an error may void a single key, such as a SOLVER-CONFIG value
    // that one level does not take (14.10). Preserve those reports, but never excuse
    // a newly introduced one merely because it was already visible in the first
    // generated-record read.
    let original_diagnostics = original.diagnostics();
    let mut matched = vec![false; original_diagnostics.len()];
    if after.diagnostics().iter().any(|d| {
        matches!(d.severity, Severity::Error | Severity::Violation)
            && !original_diagnostics.iter().enumerate().any(|(i, old)| {
                let same = !matched[i]
                    && old.severity == d.severity
                    && old.code == d.code
                    && old.message == d.message
                    && old.origin == d.origin
                    && old.in_package == d.in_package
                    && old.cause == d.cause
                    && equivalent_source(old.loc.as_ref(), d.loc.as_ref(), host_file, line_map);
                if same {
                    matched[i] = true;
                }
                same
            })
    }) {
        return Err("trial interpretation has an error or violation".into());
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
            return Err("junction definition or role identity changed".into());
        }
        if selected.next().is_some() || dot.at != Some((candidate.x, candidate.y)) {
            return Err("trial usage identity or placement incorrect".into());
        }
    }
    for old in left.objects.iter().filter(|v| !targets.iter().any(|(id, _)| *id == v.id)) {
        let mut matches = right.objects.iter().filter(|v| v.id == old.id);
        let new = matches.next().ok_or("old object missing")?;
        if matches.next().is_some() || old.at != new.at || old.orient != new.orient || old.pins.len() != new.pins.len()
        {
            return Err("old object geometry changed".into());
        }
        for (a, b) in old.pins.iter().zip(&new.pins) {
            if a.at != b.at || a.dir != b.dir || a.terminals != b.terminals || a.nets != b.nets {
                return Err("old pin geometry or connection changed".into());
            }
            match (&a.identity, &b.identity) {
                (Some(a), Some(b)) if a.index == b.index && a.reference == b.reference => {}
                (None, None) => {}
                _ => return Err("physical pin identity changed".into()),
            }
        }
    }
    if left.points.len() != right.points.len() {
        return Err("named point count changed".into());
    }
    for old in &left.points {
        let mut matches = right.points.iter().filter(|point| point.id == old.id);
        let new = matches.next().ok_or("old point missing")?;
        if matches.next().is_some() || old.at != new.at || old.junction != new.junction {
            return Err("old point geometry or junction identity changed".into());
        }
    }
    if left.wires.len() != right.wires.len() {
        return Err("wire count changed".into());
    }
    for (a, b) in left.wires.iter().zip(&right.wires) {
        if a.object_id != b.object_id || a.nets != b.nets || a.points != b.points || a.via != b.via || a.mode != b.mode
        {
            return Err("wire geometry or endpoint identity changed".into());
        }
    }
    // The first pass can already have dropped or deferred a constraint. Its
    // unchanged outcome needs no new resolution; newly introduced pending facts
    // are still exported and must pass the actual-drawing check above.
    for (index, constraint) in right.constraints.iter().enumerate() {
        if matches!(constraint.status.as_str(), "ok" | "disabled") {
            continue;
        }
        // Added/changed facts were independently checked against the actual drawing above.
        // A label alone is not a unique identity: different constraints can share it.
        let unchanged_pending = matches!(constraint.status.as_str(), "unsolved" | "dropped" | "needs-solver")
            && left.constraints.get(index).is_some_and(|old| {
                old.status == constraint.status
                    && old.label == constraint.label
                    && old.kind == constraint.kind
                    && old.priority == constraint.priority
            });
        if !unchanged_pending {
            return Err(format!(
                "constraint outcome requires general resolution: index {index}, status {}, previous {:?}",
                constraint.status,
                left.constraints.get(index).map(|old| (&old.status, &old.label, &old.kind, &old.priority))
            ));
        }
    }
    if right.unplaced.iter().any(|id| targets.iter().any(|(target, _)| target == id) || !left.unplaced.contains(id)) {
        return Err("junction or new generated object remains unplaced".into());
    }
    Ok(())
}
