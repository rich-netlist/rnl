//! Conservative certificates for scoring a recorded layout without inventing free values.
use crate::resolve::Resolved;
use crate::structure::Structure;
use rnl_runtime::previous::Previous;
use rnl_runtime::problem::{Axis, FactKind, LevelProblem, ObjectKind, PointRef, Position, Slot};
use serde_json::{json, Value};
type Axes = Vec<[Option<String>; 2]>;
fn known(r: &PointRef, axis: usize, axes: &Axes, orient: &[bool]) -> bool {
    match r {
        PointRef::Origin(k) => axes[*k][axis].is_some(),
        PointRef::Pin { object, .. } => axes[*object][axis].is_some() && orient[*object],
        PointRef::Centre(rs) => !rs.is_empty() && rs.iter().all(|r| known(r, axis, axes, orient)),
    }
}
fn points(r: &PointRef, used: &mut std::collections::BTreeSet<usize>) {
    match r {
        PointRef::Origin(k) => {
            used.insert(*k);
        }
        PointRef::Pin { object, .. } => {
            used.insert(*object);
        }
        PointRef::Centre(rs) => {
            for r in rs {
                points(r, used);
            }
        }
    }
}
pub(super) fn coverage(p: &LevelProblem, resolved: &Resolved, s: &Structure, previous: &Previous) -> Value {
    let mut axes: Axes = vec![[None, None]; p.objects.len()];
    let mut orient: Vec<bool> = (0..p.objects.len())
        .map(|k| !s.objects[k].oriented || resolved.witness.classes.value_of(k).is_some())
        .collect();
    let constant = |slot: &Slot, object: usize, axis: usize| match slot {
        Slot::Fixed { fact, .. } if resolved.as_written && resolved.kept.contains(fact) => match &p.facts[*fact].kind {
            FactKind::Place { at: PointRef::Origin(k), axis: a, .. } => {
                *k == object && usize::from(*a == Axis::Y) == axis
            }
            FactKind::PointCoordinate { point, axis: a, .. } => *point == object && usize::from(*a == Axis::Y) == axis,
            _ => false,
        },
        _ => false,
    };
    for (k, o) in p.objects.iter().enumerate() {
        if let Position::Own { x, y } = &o.position {
            for (axis, slot) in [x, y].into_iter().enumerate() {
                if constant(slot, k, axis) {
                    axes[k][axis] = Some("retained authored constant".into());
                }
            }
        }
    }
    for given in &previous.placements {
        for (axis, value) in [given.x, given.y].into_iter().enumerate() {
            if value.is_some() {
                axes[given.object][axis] = Some("solver record".into());
            }
        }
        if given.orient.is_some() {
            orient[given.object] = true;
        }
    }
    // Only runtime position relationships are inherited: point_value evaluates these from the
    // recorded owner geometry. Do not infer missing free coordinates from an arbitrary witness
    // or from a relation that merely happened to hold in the previous solve.
    loop {
        let before = axes.clone();
        for (k, o) in p.objects.iter().enumerate() {
            match &o.position {
                Position::At(r) => {
                    for axis in [0, 1] {
                        if known(r, axis, &axes, &orient) {
                            axes[k][axis] = Some("authored point relationship".into());
                        }
                    }
                }
                Position::InSymbol { owner, replaced, .. } if orient[*owner] => {
                    for axis in [0, 1] {
                        if let Some((a, slot)) = replaced.as_ref().filter(|(a, _)| usize::from(*a == Axis::Y) == axis) {
                            let _ = a;
                            if constant(slot, k, axis) {
                                axes[k][axis] = Some("retained authored replaced-axis constant".into());
                            }
                        } else if axes[*owner][axis].is_some() {
                            axes[k][axis] = Some("authored symbol relationship".into());
                        }
                    }
                }
                _ => {}
            }
        }
        if before == axes {
            break;
        }
    }
    let mut used = std::collections::BTreeSet::new();
    for wire in &p.wires {
        for r in &wire.anchors {
            points(r, &mut used);
        }
    }
    for o in &p.objects {
        if let Position::At(r) = &o.position {
            points(r, &mut used);
        }
    }
    let mut missing = Vec::new();
    let mut excluded = Vec::new();
    let mut certified = Vec::new();
    for (k, o) in p.objects.iter().enumerate() {
        if s.objects[k].left_free {
            excluded.push(json!({"object":k,"label":o.label,"reason":"left to viewer, not scored"}));
            continue;
        }
        if o.symbol.is_none() && o.text.is_none() && !used.contains(&k) {
            excluded.push(json!({"object":k,"label":o.label,"reason":"no Step1 scoring geometry"}));
            continue;
        }
        for axis in [0, 1] {
            if axes[k][axis].is_none() {
                missing.push(json!({"object":k,"label":o.label,"axis":if axis==0 {"x"} else {"y"},"reason":"not recorded or certified as an authored constant/position relationship"}));
            }
        }
        if o.kind != ObjectKind::Text && o.symbol.is_some() && !orient[k] {
            missing.push(
                json!({"object":k,"label":o.label,"axis":"orientation","reason":"free orientation absent from record"}),
            );
        }
        certified.push(
            json!({"object":k,"label":o.label,"x":axes[k][0],"y":axes[k][1],"orientation_fixed_or_recorded":orient[k]}),
        );
    }
    json!({"complete":missing.is_empty(),"missing":missing,"excluded":excluded,"objects":certified})
}
