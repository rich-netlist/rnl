//! Whether the facts kept fix the placement of a level: the position and the orientation of every
//! component. Then no arrangement can place it otherwise, and the group-tree start searches for
//! none: it takes the first. The line search still runs, as it routes too.

use std::collections::BTreeSet;

use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{LevelProblem, OrientSlot, Position, Slot};

use crate::structure::{turned_box, Structure};

/// Why the placement of the level of `s` is not fixed, naming the first component that the facts
/// kept leave free; `None` when it is fixed: every component, of [`Structure::components`], has a
/// position that a PLACE kept gives on both axes, or the point of its main object, and an
/// orientation that an ORIENT kept gives, or none, or one that changes nothing drawn
/// ([`turns_alike`]), such as that of a dot.
pub(crate) fn free(problem: &LevelProblem, s: &Structure) -> Option<String> {
    let kept: BTreeSet<usize> = s.kept.iter().copied().collect();
    let kept = |fact: usize| kept.contains(&fact);
    let slot = |slot: &Slot| matches!(slot, Slot::Fixed { fact, .. } if kept(*fact));
    for k in s.components() {
        let o = &problem.objects[k];
        let placed = match &o.position {
            Position::Own { x, y } => slot(x) && slot(y),
            Position::At(_) => true,
            Position::InSymbol { .. } => false,
        };
        if !placed {
            return Some(format!("the position of {} is not fixed", o.label));
        }
        let turned = match &o.orient {
            None => true,
            Some(OrientSlot::Fixed { fact, .. }) => kept(*fact),
            Some(OrientSlot::Free) => turns_alike(s, k),
        };
        if !turned {
            return Some(format!("the orientation of {} is not fixed", o.label));
        }
    }
    None
}

/// Whether every orientation of the component `k` draws it alike: its symbol's box, its pins and
/// the texts its symbol places are where they are unturned, so that no orientation is better than
/// another and the one the first arrangement gives stands.
fn turns_alike(s: &Structure, k: usize) -> bool {
    let Some(symbol) = s.objects[k].symbol.and_then(|i| s.symbols.get(i)) else { return false };
    let same = |all: Option<&Vec<[crate::rational::Q; 4]>>| all.is_none_or(|v| v.iter().all(|b| *b == v[0]));
    let texts = s.symbol_texts.get(&k).is_none_or(|v| v.iter().all(|b| *b == v[0]));
    Orient::ALL.iter().all(|&o| {
        turned_box(&symbol.bbox, o) == turned_box(&symbol.bbox, Orient::R0)
            && (0..symbol.pins.len()).all(|p| s.pin_offset(k, p, o) == s.pin_offset(k, p, Orient::R0))
    }) && same(s.labels.get(&k))
        && same(s.offset_labels.get(&k))
        && texts
}
