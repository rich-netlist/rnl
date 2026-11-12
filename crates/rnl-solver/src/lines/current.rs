//! Rank declared current directions and describe unmet pin directions.

use std::collections::BTreeSet;

use rnl_runtime::problem::LevelProblem;

use crate::coords::Placed;
use crate::structure::Structure;
pub(crate) fn against(s: &Structure, placed: &Placed, within: Option<&BTreeSet<usize>>) -> usize {
    s.currents
        .iter()
        .filter(|(&(k, _), _)| within.is_none_or(|w| w.contains(&k)))
        .filter(|(&(k, _), _)| !s.objects[k].left_free && s.objects[k].symbol.is_some())
        .filter(|(&(k, pin), &out)| {
            let with = if out { (0, -1) } else { (0, 1) };
            s.exit(k, pin, placed.geometry.orient[k]).is_some_and(|exit| exit != with)
        })
        .count()
}
pub(crate) fn unmet(problem: &LevelProblem, s: &Structure, placed: &Placed) -> Option<String> {
    let mut sideways = Vec::new();
    let mut against = Vec::new();
    for (&(k, pin), &out) in &s.currents {
        if s.objects[k].left_free || s.objects[k].symbol.is_none() {
            continue;
        }
        let Some(exit) = s.exit(k, pin, placed.geometry.orient[k]) else { continue };
        let with = if out { (0, -1) } else { (0, 1) };
        if exit == with {
            continue;
        }
        let terminal = s.pins(k).get(pin).and_then(|p| p.terminals.first()).cloned().unwrap_or_default();
        let name = format!("{}:{terminal}", problem.objects[k].label);
        if exit == (-with.0, -with.1) {
            against.push(name);
        } else {
            sideways.push(name);
        }
    }
    if sideways.is_empty() && against.is_empty() {
        return None;
    }
    let list = |v: &[String]| if v.is_empty() { "none".to_string() } else { v.join(", ") };
    Some(format!(
        "CURRENT, read as an order of the pins of low priority (in above, out below), is not met at {} of the \
         pins it marks: sideways {}; against the current {}",
        sideways.len() + against.len(),
        list(&sideways),
        list(&against)
    ))
}
