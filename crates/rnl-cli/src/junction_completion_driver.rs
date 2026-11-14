//! Isolated interpretation of candidate generated records. This module never calls a solver.
use crate::junction_completion::{self, Axis, Point, Suggestion};
use crate::junction_completion_problem::{
    added_fact_indices, rebased_reference, suggestions, validate_interpreted_trial, TrialValidation,
};
use rnl_runtime::{
    problem::{LevelProblem, PointRef, Position, Reference, Slot},
    solution::{Solution, Written},
    source::{Layered, MemoryProvider, Provider},
    Interpretation, Options,
};
use std::collections::BTreeMap;

pub struct Context<'a> {
    pub entry: &'a str,
    pub dialect: Option<&'a str>,
    pub formats: &'a BTreeMap<String, String>,
    pub files: &'a dyn Provider,
    pub runtime: &'a Options,
    pub original: &'a Interpretation,
    pub problem: &'a LevelProblem,
    pub original_text: &'a str,
    pub host_path: &'a str,
    pub kept: &'a [usize],
    pub solution: &'a Solution,
    /// Must satisfy both normal interpretation and the effective solver tolerance.
    pub tolerance: f64,
    pub on_interpret: Option<&'a dyn Fn()>,
}

fn read(ctx: &Context<'_>, record: &str) -> Result<Interpretation, String> {
    // Numeric references are inactive until their record has a valid check. Prepare each
    // complete candidate, including its appended PLACE statements, in the same package
    // environment as the normal reading. Keep the caller's raw record and byte spans intact.
    let numbered = rnl_view::finish_record_with_formats(
        &ctx.original.doc,
        ctx.files,
        ctx.formats,
        ctx.runtime,
        ctx.problem,
        record,
    );
    if let Some(observe) = ctx.on_interpret {
        observe();
    }
    let text = rnl_runtime::writeback::replace_generated_with_header(
        &ctx.original.doc,
        ctx.problem.level,
        ctx.original_text,
        &numbered,
        &ctx.problem.version,
    );
    let mut overlay = MemoryProvider::new();
    overlay.insert(ctx.host_path, &text);
    let layered = Layered { layers: vec![&overlay, ctx.files] };
    let opts = Options { generated: true, problems: true, ..ctx.runtime.clone() };
    let read = rnl_view::interpret_with(ctx.entry, ctx.dialect, ctx.formats, &layered, &opts)?;
    let file = &read.doc.levels.get(ctx.problem.level).ok_or("trial level missing")?.file;
    let numbering =
        read.numbering.iter().find(|level| level.level == ctx.problem.level).ok_or("trial numbering unavailable")?;
    if !numbering.records.iter().any(|record| record.file == *file)
        || numbering.records.iter().any(|record| record.file == *file && !record.holds())
    {
        return Err("trial generated record has missing or stale numbering".into());
    }
    Ok(read)
}

/// Why a record does not read back when objects of the level are lost in it: an object the solve
/// placed is not made when the document is read with its record. Nothing the dots do can mend it.
pub const LOST: &str = "not made when the result is read again: a rule selects by what the solve writes back";

/// The objects of `p`, the problem the solve read, that `read`, the document read with the record
/// written, does not make: those a written name or a rule refers to, none of which a record of the
/// solver takes away. A rule that selects by what the record writes, such as the pins a stub of
/// the fill was to go on before it is written, makes them no more. An object with a name is lost
/// when nothing in `read` is known to be it ([`rnl_runtime::previous::document_ids`]) and the level
/// of `read` has no object of its id; the usages a rule made and the record does not write, when
/// the rule makes fewer of them in `read`. Named in [`LOST`], at most three, a usage by its rule's
/// place.
fn lost(p: &LevelProblem, read: &Interpretation) -> Option<String> {
    use std::collections::{BTreeMap, HashSet};
    // the place of the rule that made a usage, from its id "@file:line:column", with "#n" after it
    // when the rule made more than one
    let site = |id: &str| id.split('#').next().unwrap_or(id).trim_start_matches('@').to_string();
    let ids = rnl_runtime::previous::document_ids(p, read);
    let again = read.problems.iter().find(|q| q.level == p.level)?;
    let objects: HashSet<&String> = again.ids.objects.iter().collect();
    let mut gone = Vec::new();
    // by the place of its rule: the usages not written, and the label of one
    let mut unwritten: BTreeMap<String, (usize, &str)> = BTreeMap::new();
    for ((o, known), id) in p.objects.iter().zip(&ids.objects).zip(&p.ids.objects) {
        if known.is_some() {
            continue;
        }
        match &o.reference {
            Reference::Name(name) if !objects.contains(id) => gone.push(name.clone()),
            Reference::Generated { usage, path } if path.is_empty() => {
                let Some(usage) = p.ids.usages.get(*usage) else { continue };
                let entry = unwritten.entry(site(usage)).or_insert((0, &o.label));
                entry.0 += 1;
            }
            _ => {}
        }
    }
    for (at, (count, label)) in unwritten {
        let made = again.ids.usages.iter().filter(|id| site(id) == at).count();
        match count.saturating_sub(made) {
            0 => {}
            1 => gone.push(format!("{label}, made by the rule at {at}")),
            n => gone.push(format!("{n} of {label}, made by the rule at {at}")),
        }
    }
    let (first, rest) = gone.split_first()?;
    let mut said = first.clone();
    for name in rest.iter().take(2) {
        said.push_str("; ");
        said.push_str(name);
    }
    if rest.len() > 2 {
        said.push_str(&format!(" and {} more", rest.len() - 2));
    }
    let verb = if gone.len() == 1 { "is" } else { "are" };
    Some(format!("{said} {verb} {LOST}"))
}

fn line_mapping(ctx: &Context<'_>, new_read: &Interpretation, record: &str) -> Result<Vec<Option<u32>>, String> {
    let old_level = ctx.original.doc.levels.get(ctx.problem.level).ok_or("old level missing")?;
    let new_level = new_read.doc.levels.get(ctx.problem.level).ok_or("new level missing")?;
    let generated_ranges = |level: &rnl_runtime::host::Level| -> Vec<(u32, u32)> {
        level
            .records
            .iter()
            .filter(|r| r.file == old_level.file && rnl_runtime::writeback::record_type(r) == "generated")
            .map(|r| r.lines)
            .collect()
    };
    let headers = |doc: &rnl_runtime::host::HostDocument| -> Vec<rnl_runtime::host::Record> {
        doc.levels
            .iter()
            .flat_map(|level| &level.records)
            .filter(|r| r.file == old_level.file && rnl_runtime::writeback::record_type(r) == "header")
            .cloned()
            .collect()
    };
    let old_headers = headers(&ctx.original.doc);
    let new_headers = headers(&new_read.doc);
    let mut old_ranges = generated_ranges(old_level);
    let mut new_ranges = generated_ranges(new_level);
    if !old_headers.iter().map(|r| &r.text).eq(new_headers.iter().map(|r| &r.text)) {
        // A changed header may alter columns on a shared physical line. Do not guess
        // positions of an adjacent record; unchanged headers retain ordinary line mapping.
        let shared = |doc: &rnl_runtime::host::HostDocument, headers: &[rnl_runtime::host::Record]| {
            doc.levels.iter().flat_map(|level| &level.records).any(|r| {
                r.file == old_level.file
                    && rnl_runtime::writeback::record_type(r) != "header"
                    && headers.iter().any(|h| r.lines.0 <= h.lines.1 && h.lines.0 <= r.lines.1)
            })
        };
        if shared(&ctx.original.doc, &old_headers) || shared(&new_read.doc, &new_headers) {
            return Err("changed header shares a physical line with another record".into());
        }
        old_ranges.extend(old_headers.iter().map(|r| r.lines));
        new_ranges.extend(new_headers.iter().map(|r| r.lines));
    }
    let new_text = rnl_runtime::writeback::replace_generated_with_header(
        &ctx.original.doc,
        ctx.problem.level,
        ctx.original_text,
        record,
        &ctx.problem.version,
    );
    let kept = |line: u32, ranges: &[(u32, u32)]| !ranges.iter().any(|&(start, end)| start <= line && line <= end);
    let old: Vec<_> = ctx.original_text.lines().enumerate().filter(|(i, _)| kept(*i as u32 + 1, &old_ranges)).collect();
    let new: Vec<_> = new_text.lines().enumerate().filter(|(i, _)| kept(*i as u32 + 1, &new_ranges)).collect();
    if old.len() != new.len() {
        return Err("cannot map lines outside generated record edits".into());
    }
    let mut map = vec![None; ctx.original_text.lines().count() + 1];
    for ((old_index, old_line), (new_index, new_line)) in old.into_iter().zip(new) {
        if old_line != new_line {
            return Err("non-generated line moved or changed during writeback".into());
        }
        map[old_index + 1] = Some(new_index as u32 + 1);
    }
    Ok(map)
}

fn named_point(problem: &LevelProblem, name: &str) -> Option<Point> {
    let mut matching = problem.objects.iter().filter(|o| o.reference == Reference::Name(name.to_owned()));
    let o = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let Position::Own { x: Slot::Fixed { value: x, .. }, y: Slot::Fixed { value: y, .. } } = &o.position else {
        return None;
    };
    Some(Point { x: *x, y: *y })
}

fn fixed_point(read: &Interpretation, p: &LevelProblem, point: &PointRef) -> Option<Point> {
    fixed_point_inner(read, p, point, 0)
}
fn fixed_point_inner(read: &Interpretation, p: &LevelProblem, point: &PointRef, depth: usize) -> Option<Point> {
    if depth > p.objects.len() {
        return None;
    }
    let level = read.model.levels.iter().find(|v| v.level == p.level)?;
    match point {
        PointRef::Origin(index) => {
            let object = p.objects.get(*index)?;
            if let Position::At(point) = &object.position {
                if point == &PointRef::Origin(*index) {
                    return None;
                }
                return fixed_point_inner(read, p, point, depth + 1);
            }
            if let Reference::Name(name) = &object.reference {
                if let Some(point) = named_point(p, name) {
                    return Some(point);
                }
            }
            let id = p.ids.objects.get(*index)?;
            let mut found = level.objects.iter().filter(|o| &o.id == id);
            let at = found.next()?.at?;
            if found.next().is_some() {
                return None;
            }
            Some(Point { x: at.0, y: at.1 })
        }
        PointRef::Pin { object, pin, .. }
        | PointRef::SymbolPin { object, pin, .. }
        | PointRef::PinPosition { object, pin, .. } => {
            let id = p.ids.objects.get(*object)?;
            let mut owner = level.objects.iter().filter(|o| &o.id == id);
            let owner = owner.next()?;
            let mut found = owner.pins.iter().filter(|q| q.identity.as_ref().is_some_and(|id| id.index == *pin));
            let at = found.next()?.at?;
            if found.next().is_some() {
                return None;
            }
            Some(Point { x: at.0, y: at.1 })
        }
        PointRef::Centre(points) => {
            let coords: Vec<_> =
                points.iter().map(|q| fixed_point_inner(read, p, q, depth + 1)).collect::<Option<_>>()?;
            let first = coords.first()?;
            let (mut xmin, mut xmax, mut ymin, mut ymax) = (first.x, first.x, first.y, first.y);
            for q in coords {
                xmin = xmin.min(q.x);
                xmax = xmax.max(q.x);
                ymin = ymin.min(q.y);
                ymax = ymax.max(q.y);
            }
            Some(Point { x: (xmin + xmax) / 2., y: (ymin + ymax) / 2. })
        }
    }
}
fn original_point_in_trial(
    point: &PointRef,
    original: &LevelProblem,
    trial: &LevelProblem,
    ids: &[Option<String>],
) -> Option<PointRef> {
    let owner = |index: usize| -> Option<usize> {
        let old = original.objects.get(index)?;
        let id = ids.get(index)?.as_ref()?;
        let mut found = trial.ids.objects.iter().enumerate().filter(|(_, candidate)| *candidate == id);
        let (index, _) = found.next()?;
        if found.next().is_some() || trial.objects.get(index)?.kind != old.kind {
            return None;
        }
        Some(index)
    };
    let pin_owner = |object: usize, pin: usize| -> Option<usize> {
        let mapped = owner(object)?;
        let old_symbol = original.symbols.get(original.objects.get(object)?.symbol?)?;
        let new_symbol = trial.symbols.get(trial.objects.get(mapped)?.symbol?)?;
        if old_symbol.name != new_symbol.name || old_symbol.pins.get(pin)? != new_symbol.pins.get(pin)? {
            return None;
        }
        Some(mapped)
    };
    match point {
        PointRef::Origin(i) => Some(PointRef::Origin(owner(*i)?)),
        PointRef::Pin { object, pin, terminal } => {
            Some(PointRef::Pin { object: pin_owner(*object, *pin)?, pin: *pin, terminal: terminal.clone() })
        }
        PointRef::SymbolPin { object, pin, reference } => {
            let mapped = pin_owner(*object, *pin)?;
            Some(PointRef::SymbolPin {
                object: mapped,
                pin: *pin,
                reference: rebased_reference(
                    reference,
                    &original.objects[*object].reference,
                    &trial.objects[mapped].reference,
                )?,
            })
        }
        PointRef::PinPosition { object, pin, reference } => {
            let mapped = pin_owner(*object, *pin)?;
            Some(PointRef::PinPosition {
                object: mapped,
                pin: *pin,
                reference: rebased_reference(
                    reference,
                    &original.objects[*object].reference,
                    &trial.objects[mapped].reference,
                )?,
            })
        }
        PointRef::Centre(_) => None,
    }
}

/// Returns a replacement record only after normal interpretation and conservative validation.
/// Unmappable/ambiguous usages and non-positioned role targets fail closed.
pub struct Completion {
    pub record: String,
    pub trials: usize,
    pub elapsed: std::time::Duration,
}
pub fn complete_record(ctx: &Context<'_>, written: &Written) -> Result<Completion, String> {
    complete_within(ctx, written, MAX_TRIALS, &mut 0, None)
}

/// A single global bound on the candidate trials of a level, independent of junction count.
const MAX_TRIALS: usize = 64;

/// Why a dot is left out when the level's time ran out before it was tried, apart from a dot that
/// has no place: the time, not the dot.
pub const RAN_OUT: &str = "the time of the level ran out before the dot was tried";

/// When the time of a level runs out for the completion: `after` from the moment it is made. It is
/// read before each trial, so that no trial begins once it has passed; a trial begun before may end
/// after it. Where there is no clock (WebAssembly), only a deadline of no time has passed.
pub struct Deadline {
    since: crate::solve::Stopwatch,
    after: std::time::Duration,
}

impl Deadline {
    /// The deadline `after` from now.
    pub fn after(after: std::time::Duration) -> Deadline {
        Deadline { since: crate::solve::Stopwatch::start(), after }
    }

    fn passed(&self) -> bool {
        self.after.is_zero() || self.since.elapsed() >= self.after
    }
}

/// [`complete_record`] with at most `max_trials` candidate trials, counted into `used`, and none
/// begun after `deadline`: then it fails with [`RAN_OUT`].
fn complete_within(
    ctx: &Context<'_>,
    written: &Written,
    max_trials: usize,
    used: &mut usize,
    deadline: Option<&Deadline>,
) -> Result<Completion, String> {
    let start = crate::solve::Stopwatch::start();
    let late = || deadline.is_some_and(Deadline::passed);
    if late() {
        return Err(RAN_OUT.into());
    }
    if !written.unsolved.is_empty() {
        return Err("first-pass decisions could not all be written".into());
    }
    let record = &written.record;
    let before = read(ctx, record)?;
    if let Some(why) = lost(ctx.problem, &before) {
        return Err(why);
    }
    let line_map = line_mapping(ctx, &before, record)?;
    let problem = before.problems.iter().find(|p| p.level == ctx.problem.level).ok_or("trial problem missing")?;
    // The writer omits coordinates already supplied by following text or a symbol.
    // Compare their interpreted positions, using the same generated-reference mapping
    // as previous results, rather than requiring redundant fixed slots in the record.
    let drawn = rnl_runtime::previous::drawn(ctx.problem, &before);
    for placement in &ctx.solution.placements {
        let new = drawn.get(placement.object).ok_or("first-pass placement index invalid")?;
        for (axis, (wanted, actual)) in [placement.x, placement.y].into_iter().zip(new.at).enumerate() {
            if let Some(wanted) = wanted {
                if !actual.is_some_and(|actual| (actual - wanted).abs() <= ctx.tolerance) {
                    return Err(format!(
                        "first-pass placement did not survive interpretation: object {}, axis {axis}, expected {wanted}, drawn {actual:?}",
                        placement.object
                    ));
                }
            }
        }
        if let Some(orient) = placement.orient {
            if new.orient != Some(orient) {
                return Err("first-pass orientation did not survive interpretation".into());
            }
        }
    }

    let host = before.doc.levels.get(ctx.problem.level).ok_or("trial level missing")?;
    let generated: Vec<_> =
        host.records.iter().filter(|r| rnl_runtime::writeback::record_type(r) == "generated").collect();
    if generated.len() != 1 {
        return Err("generated record missing or ambiguous".into());
    }
    struct Plan {
        end: usize,
        usage_id: String,
        fixed: [Option<f64>; 2],
        candidates: Vec<Point>,
    }
    let mut plans = Vec::new();
    let original_ids = rnl_runtime::previous::document_ids(ctx.problem, &before);
    for junction in &written.junctions {
        let prefix = record.get(..junction.span.start).ok_or("invalid junction span")?;
        let line = generated[0].lines.0 + prefix.bytes().filter(|byte| *byte == b'\n').count() as u32;
        let file = &generated[0].file;
        let mut found = problem
            .usages
            .iter()
            .enumerate()
            .filter(|(_, u)| u.source.as_ref().is_some_and(|s| s.file == *file && s.line == line));
        let (usage, u) = found.next().ok_or("junction usage not interpreted")?;
        if found.next().is_some() {
            return Err("ambiguous junction source".into());
        }
        let own = u.object.ok_or("junction has no own point")?;
        let usage_id = problem.ids.objects.get(own).ok_or("junction object id missing")?.clone();
        let role = if let Some(original) = &junction.original {
            let mapped = original_point_in_trial(original, ctx.problem, problem, &original_ids.objects)
                .ok_or("original junction identity is ambiguous")?;
            fixed_point(&before, problem, &mapped).ok_or("original junction identity has no fixed location")?
        } else {
            named_point(problem, &junction.target).ok_or("branch point has no fixed location")?
        };
        let view =
            before.model.levels.iter().find(|level| level.level == ctx.problem.level).ok_or("trial level missing")?;
        let object = view.objects.iter().find(|object| object.id == usage_id).ok_or("junction view missing")?;
        let mut fixed = object.at.map(|(x, y)| [Some(x), Some(y)]).unwrap_or([None, None]);
        if let Position::Own { x, y } = &problem.objects[own].position {
            for (axis, slot) in [x, y].into_iter().enumerate() {
                if fixed[axis].is_none() {
                    if let Slot::Fixed { value, .. } = slot {
                        fixed[axis] = Some(*value);
                    }
                }
            }
        }
        let mut facts = suggestions(problem, usage, own, |q| fixed_point(&before, problem, q));
        if let Some(value) = fixed[0] {
            facts.insert(0, Suggestion { axis: Axis::X, value });
        }
        if let Some(value) = fixed[1] {
            facts.insert(0, Suggestion { axis: Axis::Y, value });
        }
        let mut candidates = Vec::new();
        let result = junction_completion::complete(role, &facts, 64, |p| {
            if fixed[0].is_none_or(|x| x == p.x) && fixed[1].is_none_or(|y| y == p.y) {
                candidates.push(p);
            }
            Err("collect".into())
        });
        if !matches!(result, Err(junction_completion::CompletionError::Unresolved(_))) || candidates.is_empty() {
            return Err("junction candidate limit or invalid coordinate".into());
        }
        plans.push(Plan { end: junction.span.end, usage_id, fixed, candidates });
    }
    let mut indexes = vec![0usize; plans.len()];
    let mut last_error = String::from("no candidates");
    for attempt in 0..max_trials {
        if late() {
            return Err(RAN_OUT.into());
        }
        *used += 1;
        let mut trial = record.clone();
        let targets: Vec<_> = plans.iter().zip(&indexes).map(|(p, &i)| (p.usage_id.clone(), p.candidates[i])).collect();
        for (plan, &index) in plans.iter().zip(&indexes).rev() {
            let q = plan.candidates[index];
            let place = match plan.fixed {
                [Some(_), Some(_)] => String::new(),
                [Some(_), None] => format!(" => @PLACE(y={})", q.y),
                [None, Some(_)] => format!(" => @PLACE(x={})", q.x),
                [None, None] => format!(" => @PLACE({}, {})", q.x, q.y),
            };
            trial.insert_str(plan.end, &place);
        }
        let after = read(ctx, &trial)?;
        let after_problem =
            after.problems.iter().find(|p| p.level == ctx.problem.level).ok_or("trial problem missing")?;
        let added = added_fact_indices(
            ctx.problem,
            after_problem,
            &ctx.original.doc.levels[ctx.problem.level].file,
            &line_map,
            &after,
        )
        .map_err(str::to_owned)?;
        match validate_interpreted_trial(TrialValidation {
            original: ctx.original,
            original_problem: ctx.problem,
            host_file: &ctx.original.doc.levels[ctx.problem.level].file,
            line_map: &line_map,
            kept: ctx.kept,
            before: &before,
            after: &after,
            level: ctx.problem.level,
            targets: &targets,
            added_facts: &added,
            tolerance: ctx.tolerance,
        }) {
            Ok(()) => return Ok(Completion { record: trial, trials: attempt + 1, elapsed: start.elapsed() }),
            Err(e) => {
                last_error = e;
                if let Some(d) = after.diagnostics().iter().find(|d| {
                    matches!(d.severity, rnl_runtime::diag::Severity::Error | rnl_runtime::diag::Severity::Violation)
                }) {
                    last_error.push_str(&format!(": {}: {}", d.code, d.message));
                }
            }
        }
        let mut advanced = false;
        for i in (0..indexes.len()).rev() {
            indexes[i] += 1;
            if indexes[i] < plans[i].candidates.len() {
                advanced = true;
                break;
            }
            indexes[i] = 0;
        }
        if !advanced {
            break;
        }
    }
    Err(format!("junction placement unresolved after globally bounded trials: {last_error}"))
}

/// The record with the dots that can be placed: what [`Partial::record`] holds, and the dots left
/// out, by their index in [`Written::junctions`], with why.
pub struct Partial {
    pub record: String,
    pub trials: usize,
    pub elapsed: std::time::Duration,
    pub dropped: Vec<(usize, String)>,
}

/// Places every dot when [`complete_record`] can; otherwise keeps, in order, each dot that can
/// be placed beside those kept before it, and leaves out the rest. A dot that no place keeps the
/// facts of the level for costs only itself: the rest of the record is written. The record without
/// any dot must pass the same checks, or nothing is written: what fails then is not the dots'.
pub fn complete_what_can_be(ctx: &Context<'_>, written: &Written) -> Result<Partial, String> {
    complete_what_can_be_by(ctx, written, None)
}

/// [`complete_what_can_be`] with no trial begun after `deadline`: the dots not tried by then are
/// left out, each with [`RAN_OUT`]. The record without any dot is checked all the same, however late,
/// since nothing is written that does not read back as it was solved.
pub fn complete_what_can_be_by(
    ctx: &Context<'_>,
    written: &Written,
    deadline: Option<&Deadline>,
) -> Result<Partial, String> {
    let start = crate::solve::Stopwatch::start();
    // one bound on the candidate trials of the level covers every stage, so that the time a level
    // takes stays bounded however many dots it has; the record without any dot is checked beyond it
    let mut trials = 0;
    let why = match complete_within(ctx, written, MAX_TRIALS, &mut trials, deadline) {
        Ok(done) => return Ok(Partial { record: done.record, trials, elapsed: start.elapsed(), dropped: Vec::new() }),
        Err(why) => why,
    };
    let bare = only(written, &[]);
    if let Err(without) = complete_within(ctx, &bare, 1, &mut 0, None) {
        // objects lost on reading the record back are lost with the dots or without them
        if without.ends_with(LOST) {
            return Err(without);
        }
        return Err(format!("{why}; without any dot: {without}"));
    }
    // the dots one by one, in order, each beside those kept before it: a dot kept may leave no
    // place for a later one that could have been placed alone
    let mut kept: Vec<usize> = Vec::new();
    let mut record = None;
    let mut dropped = Vec::new();
    for index in 0..written.junctions.len() {
        if deadline.is_some_and(Deadline::passed) {
            dropped.push((index, RAN_OUT.to_string()));
            continue;
        }
        if trials >= MAX_TRIALS {
            dropped.push((index, "the bound on candidate trials was reached".to_string()));
            continue;
        }
        let mut with = kept.clone();
        with.push(index);
        match complete_within(ctx, &only(written, &with), MAX_TRIALS - trials, &mut trials, deadline) {
            Ok(done) => {
                kept = with;
                record = Some(done.record);
            }
            Err(why) => dropped.push((index, why)),
        }
    }
    let record = record.unwrap_or(bare.record);
    Ok(Partial { record, trials, elapsed: start.elapsed(), dropped })
}

/// The written record with only the junctions `keep`, by their index in [`Written::junctions`]:
/// the ATTACH statements of the others are taken out, with their separators.
pub fn only(written: &Written, keep: &[usize]) -> Written {
    const SEPARATOR: &str = ";\n  ";
    let mut out = written.clone();
    let mut cuts: Vec<std::ops::Range<usize>> = Vec::new();
    for (index, junction) in written.junctions.iter().enumerate() {
        if keep.contains(&index) {
            continue;
        }
        let span = junction.span.clone();
        let cut = if written.record.get(..span.start).is_some_and(|before| before.ends_with(SEPARATOR)) {
            span.start - SEPARATOR.len()..span.end
        } else if written.record.get(span.end..).is_some_and(|after| after.starts_with(SEPARATOR)) {
            span.start..span.end + SEPARATOR.len()
        } else {
            span
        };
        cuts.push(cut);
    }
    cuts.sort_by_key(|cut| std::cmp::Reverse(cut.start));
    for cut in &cuts {
        out.record.replace_range(cut.clone(), "");
    }
    // the only statement taken out leaves an empty body
    const EMPTY: &str = " {\n  \n}";
    if let Some(body) = out.record.find(EMPTY) {
        out.record.replace_range(body..body + EMPTY.len(), " {}");
    }
    out.junctions = Vec::new();
    for (index, junction) in written.junctions.iter().enumerate() {
        if !keep.contains(&index) {
            continue;
        }
        let shift: usize = cuts.iter().filter(|cut| cut.end <= junction.span.start).map(|cut| cut.len()).sum();
        let mut moved = junction.clone();
        moved.span = junction.span.start - shift..junction.span.end - shift;
        out.junctions.push(moved);
    }
    out
}
