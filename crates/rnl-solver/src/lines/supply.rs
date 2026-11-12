//! Count extra supply and ground heights within accepted group scopes.

use std::collections::BTreeSet;

use crate::coords::Placed;
use crate::rational::Q;
use crate::structure::{Role, Structure};
pub(crate) fn levels(s: &Structure, placed: &Placed, within: Option<&BTreeSet<usize>>) -> usize {
    let g = &placed.geometry;
    [Role::Ground, Role::Supply]
        .into_iter()
        .map(|role| {
            let ys: BTreeSet<Q> = s
                .components()
                .filter(|&k| within.is_none_or(|w| w.contains(&k)) && s.objects[k].role == role)
                .filter_map(|k| s.pin_at(&placed.origins, &g.orient, k, 0).map(|p| p[1].clone()))
                .collect();
            ys.len().saturating_sub(1)
        })
        .sum()
}
