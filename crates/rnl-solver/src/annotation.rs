//! Solver-facing exact placement of a materialized, non-electrical annotation.
use crate::rational::{from_f64, to_f64, Q};
use crate::resolve::{point_value, Geometry};
use rnl_runtime::problem::{LevelProblem, PointRef};

/// Error indices are in `LevelProblem.annotations`, never in objects or wires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnnotationError {
    pub index: usize,
    pub reason: AnnotationFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnnotationFailure {
    Unmapped,
    UnresolvedAnchor,
    NonFiniteAnchor,
    NonFiniteGraphics,
    InvalidGeometry,
}

impl AnnotationFailure {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::Unmapped => "unmapped annotation object",
            Self::UnresolvedAnchor => "unresolved annotation anchor",
            Self::NonFiniteAnchor => "non-finite annotation anchor",
            Self::NonFiniteGraphics => "non-finite annotation graphics",
            Self::InvalidGeometry => "invalid annotation geometry",
        }
    }
}

/// Evaluate visible bounds at a candidate position. `None` from the renderer is genuinely empty;
/// an unrepresentable anchor or geometry is an error and may never mean "no obstacle".
pub(crate) fn bounds(problem: &LevelProblem, geometry: &Geometry) -> Result<Vec<(usize, [Q; 4])>, AnnotationError> {
    let mut result = Vec::new();
    for (index, annotation) in problem.annotations.iter().enumerate() {
        let object = annotation.object.ok_or(AnnotationError { index, reason: AnnotationFailure::Unmapped })?;
        let point = PointRef::Origin(object);
        let x = point_value(problem, geometry, &point, 0, 0)
            .ok_or(AnnotationError { index, reason: AnnotationFailure::UnresolvedAnchor })?;
        let y = point_value(problem, geometry, &point, 1, 0)
            .ok_or(AnnotationError { index, reason: AnnotationFailure::UnresolvedAnchor })?;
        if let Some(bounds) = bounds_at(&annotation.view, &x, &y, index)? {
            result.push((object, bounds));
        }
    }
    Ok(result)
}

/// Bounds of a view at exact origin, returning coordinates in the rational solver domain.
pub(crate) fn bounds_at(
    view: &rnl_runtime::model::AnnotationView,
    x: &Q,
    y: &Q,
    index: usize,
) -> Result<Option<[Q; 4]>, AnnotationError> {
    let (fx, fy) = (to_f64(x), to_f64(y));
    if !fx.is_finite() || !fy.is_finite() {
        return Err(AnnotationError { index, reason: AnnotationFailure::NonFiniteAnchor });
    }
    let mut view = view.clone();
    view.anchor = (fx, fy);
    let bounds = rnl_render::annotation_bounds(&view, rnl_render::RenderOptions::default().scale).map_err(|error| {
        AnnotationError {
            index,
            reason: match error {
                rnl_render::AnnotationBoundsError::NonFinite => AnnotationFailure::NonFiniteGraphics,
                rnl_render::AnnotationBoundsError::InvalidGeometry => AnnotationFailure::InvalidGeometry,
            },
        }
    })?;
    let Some([xmin, ymin, xmax, ymax]) = bounds else { return Ok(None) };
    let convert = |v| from_f64(v).ok_or(AnnotationError { index, reason: AnnotationFailure::NonFiniteGraphics });
    // Placement is rational while the renderer and emitted record use finite f64. Enclose both
    // the exact origin and its emitted rounding so neither drawing can extend outside the box.
    let emitted_x = convert(fx)?;
    let emitted_y = convert(fy)?;
    let lo_x = x.clone().min(emitted_x.clone());
    let hi_x = x.clone().max(emitted_x);
    let lo_y = y.clone().min(emitted_y.clone());
    let hi_y = y.clone().max(emitted_y);
    let bounds = [lo_x + convert(xmin)?, lo_y + convert(ymin)?, hi_x + convert(xmax)?, hi_y + convert(ymax)?];
    if bounds[0] > bounds[2] || bounds[1] > bounds[3] {
        return Err(AnnotationError { index, reason: AnnotationFailure::InvalidGeometry });
    }
    Ok(Some(bounds))
}

/// Electrical owner when an annotation follows an object or one of its pin positions.
pub(crate) fn attachment_owner(problem: &LevelProblem, object: usize) -> Option<usize> {
    match &problem.objects[object].position {
        rnl_runtime::problem::Position::At(PointRef::Origin(k))
            if matches!(
                problem.objects.get(*k).map(|o| o.kind),
                Some(
                    rnl_runtime::problem::ObjectKind::Instance
                        | rnl_runtime::problem::ObjectKind::Bundle
                        | rnl_runtime::problem::ObjectKind::Usage
                )
            ) =>
        {
            Some(*k)
        }
        rnl_runtime::problem::Position::At(
            PointRef::Pin { object, .. } | PointRef::SymbolPin { object, .. } | PointRef::PinPosition { object, .. },
        ) => Some(*object),
        _ => None,
    }
}

/// Candidate collisions, with exact world boxes. A follower may overlap its own owner.
pub(crate) type Collision = (usize, usize, [Q; 4], [Q; 4]);

pub(crate) fn collisions(
    problem: &LevelProblem,
    structure: &crate::structure::Structure,
    geometry: &Geometry,
    origins: &[[Q; 2]],
) -> Result<Vec<Collision>, AnnotationError> {
    let boxes = bounds(problem, geometry)?;
    let intersects = |a: &[Q; 4], b: &[Q; 4]| a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3];
    let mut result = Vec::new();
    for (i, (object, b)) in boxes.iter().enumerate() {
        for k in structure.components() {
            if attachment_owner(problem, *object) == Some(k) {
                continue;
            }
            if let Some(v) = structure.box_at(origins, &geometry.orient, k) {
                if intersects(b, &v) {
                    result.push((*object, k, b.clone(), v));
                }
            }
        }
        for (other, v) in boxes.iter().skip(i + 1) {
            if attachment_owner(problem, *object).is_some()
                && attachment_owner(problem, *object) == attachment_owner(problem, *other)
            {
                continue;
            }
            if intersects(b, v) {
                result.push((*object, *other, b.clone(), v.clone()));
            }
        }
    }
    Ok(result)
}

pub(crate) fn overlaps(
    problem: &LevelProblem,
    structure: &crate::structure::Structure,
    geometry: &Geometry,
    origins: &[[Q; 2]],
) -> Result<usize, AnnotationError> {
    Ok(collisions(problem, structure, geometry, origins)?.len())
}

/// Deterministic admission estimate for materialization/geometry work. This is not an A* count.
pub(crate) fn estimated_work(problem: &LevelProblem) -> usize {
    let source = problem.annotations.iter().fold(0usize, |total, annotation| {
        let graphics = annotation.view.graphics.iter().fold(0usize, |count, graphic| {
            let primitive = &graphic.primitive;
            count
                .saturating_add(1)
                .saturating_add(primitive.points.len())
                .saturating_add(primitive.text.as_ref().map_or(0, |s| s.len()))
                .saturating_add(graphic.text_refs.len())
                .saturating_add(graphic.lines.iter().fold(0usize, |n, line| n.saturating_add(line.len())))
        });
        total.saturating_add(graphics.saturating_add(1).saturating_mul(16))
    });
    source.saturating_add(collision_estimate(problem.annotations.len(), problem.objects.len()).saturating_mul(16))
}

// One comparison per annotation/component and unordered annotation pair. The object count is a
// conservative upper bound on components. Halve before multiplication to preserve saturation.
fn collision_estimate(n: usize, objects: usize) -> usize {
    let pair = if n % 2 == 0 {
        (n / 2).saturating_mul(n.saturating_sub(1))
    } else {
        n.saturating_mul(n.saturating_sub(1) / 2)
    };
    n.saturating_mul(objects).saturating_add(pair)
}

#[cfg(test)]
mod estimate_tests {
    use super::collision_estimate;
    #[test]
    fn comparisons_include_pairs_and_saturate() {
        assert_eq!(collision_estimate(0, usize::MAX), 0);
        assert_eq!(collision_estimate(2, 5), 11);
        assert_eq!(collision_estimate(usize::MAX, usize::MAX), usize::MAX);
    }
}
