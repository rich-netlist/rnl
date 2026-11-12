//! Deterministic proposed junction coordinates. No solver call, file mutation or name matching.
//!
//! The caller MUST provide a normal-interpretation verifier for the exact candidate record. The
//! verifier is responsible for checking facts, identities, original geometry and diagnostics.
//! This module deliberately treats all unverified candidates as failures.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// A sound suggestion extracted from the actual expanded facts and frozen coordinates.
/// It is only a proposal: a lower-priority fact may be discarded by interpretation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Suggestion {
    pub axis: Axis,
    pub value: f64,
}

#[derive(Debug, PartialEq)]
pub enum CompletionError {
    InvalidCoordinate,
    TooManyCandidates,
    Unresolved(Vec<String>),
}

/// Candidate order is deterministic. Each candidate is accepted ONLY after `verify` succeeds.
/// The role point is a fallback layout choice, never an unconditional placement. The candidate
/// limit is structure/work based, not wall time. No in-place state is mutated by this function.
pub fn complete<F>(role: Point, facts: &[Suggestion], limit: usize, mut verify: F) -> Result<Point, CompletionError>
where
    F: FnMut(Point) -> Result<(), String>,
{
    if !role.x.is_finite() || !role.y.is_finite() || facts.iter().any(|s| !s.value.is_finite()) {
        return Err(CompletionError::InvalidCoordinate);
    }
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for fact in facts {
        let values = match fact.axis {
            Axis::X => &mut xs,
            Axis::Y => &mut ys,
        };
        if !values.contains(&fact.value) {
            values.push(fact.value);
        }
    }
    if !xs.contains(&role.x) {
        xs.push(role.x);
    }
    if !ys.contains(&role.y) {
        ys.push(role.y);
    }
    if xs.len().checked_mul(ys.len()).is_none_or(|n| n > limit) {
        return Err(CompletionError::TooManyCandidates);
    }
    let mut failures = Vec::new();
    for &x in &xs {
        for &y in &ys {
            let p = Point { x, y };
            match verify(p) {
                Ok(()) => return Ok(p),
                Err(reason) => failures.push(reason),
            }
        }
    }
    Err(CompletionError::Unresolved(failures))
}

/// Result of a normal interpretation of one isolated trial record. The adapter must construct
/// these comparisons from stable source/point/pin identities, not from runtime object indices.
#[derive(Debug, PartialEq)]
pub struct TrialEvidence {
    pub trial_usage_found_once: bool,
    pub expected_definition_and_role: bool,
    pub usage_at_candidate: bool,
    pub previous_geometry_and_wire_identities_unchanged: bool,
    pub previous_retained_facts_unchanged: bool,
    pub relevant_facts_decidable_and_compatible: bool,
    pub required_generated_objects_placed: bool,
    pub no_new_errors: bool,
    pub same_record_and_numbering_valid: bool,
}

impl TrialEvidence {
    /// Must fail closed on missing fields: callers cannot use a default successful observation.
    pub fn validate(&self) -> Result<(), String> {
        for (ok, label) in [
            (self.trial_usage_found_once, "missing or duplicate trial usage"),
            (self.expected_definition_and_role, "definition or role identity changed"),
            (self.usage_at_candidate, "candidate not effective"),
            (self.previous_geometry_and_wire_identities_unchanged, "prior geometry changed"),
            (self.previous_retained_facts_unchanged, "prior retained fact changed"),
            (self.relevant_facts_decidable_and_compatible, "facts unresolved or conflicting"),
            (self.required_generated_objects_placed, "required generated object unplaced"),
            (self.no_new_errors, "new interpretation error"),
            (self.same_record_and_numbering_valid, "record identity or numbering invalid"),
        ] {
            if !ok {
                return Err(label.into());
            }
        }
        Ok(())
    }
}

/// Construct and validate in isolation. The result is an owned candidate record; only the caller
/// may later commit it, after final numbering. A failed trial cannot mutate an existing record.
pub fn verified_record<F>(
    role: Point,
    facts: &[Suggestion],
    limit: usize,
    mut trial: F,
) -> Result<(Point, String), CompletionError>
where
    F: FnMut(Point) -> Result<(String, TrialEvidence), String>,
{
    let mut accepted = None;
    let point = complete(role, facts, limit, |p| {
        let (record, evidence) = trial(p)?;
        evidence.validate()?;
        accepted = Some(record);
        Ok(())
    })?;
    Ok((point, accepted.expect("successful validation stores candidate")))
}
