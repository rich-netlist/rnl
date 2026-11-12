//! Orient and separate symbols that hang from a leaf.

use std::collections::BTreeMap;

use num_traits::Zero;
use rnl_runtime::geom::Orient;

use crate::rational::{int, Q};
use crate::search::{preference, Attach, Side};
use crate::structure::{Role, Structure};

const MOST_STEPS: i64 = 40;

pub(super) fn now(s: &Structure, k: usize, a: &Attach, ori: &[Orient]) -> Attach {
    let mut a = *a;
    if s.objects[k].hung_from == Some(a.host) {
        a.side = crate::search::leaves(s, a.host, ori);
        return a;
    }
    match s.exit(a.host.0, a.host.1, ori[a.host.0]) {
        Some((-1, 0)) if a.side == Side::Right => a.side = Side::Left,
        Some((1, 0)) if a.side == Side::Left => a.side = Side::Right,
        _ => {}
    }
    a
}

pub(super) fn turn(s: &Structure, k: usize, a: &Attach, ori: &mut [Orient]) {
    if s.objects[k].role != Role::Port {
        return;
    }
    let want = match now(s, k, a, ori).side {
        Side::Left => (1, 0),
        Side::Right => (-1, 0),
        Side::Above => (0, -1),
        Side::Below => (0, 1),
    };
    let Some(c) = s.class_of(k) else { return };
    let [(_, label)] = s.open.classes[c].members.as_slice() else { return };
    let mut all = Orient::ALL;
    all.sort_by_key(|v| preference(*v));
    if let Some(v) = all.into_iter().find(|v| s.exit(k, 0, v.then(*label)) == Some(want)) {
        ori[k] = v.then(*label);
    }
}

pub(super) fn clear(
    s: &Structure,
    k: usize,
    hung: &[(usize, Option<Attach>)],
    offs: &mut BTreeMap<usize, [Q; 2]>,
    ori: &[Orient],
) {
    let boxed = |j: usize, at: &[Q; 2]| -> Vec<[Q; 4]> {
        [s.box_offset(j, ori[j]), s.label_offset(j, ori[j])]
            .into_iter()
            .flatten()
            .map(|b| [&b[0] + &at[0], &b[1] + &at[1], &b[2] + &at[0], &b[3] + &at[1]])
            .collect()
    };
    let zero = [Q::zero(), Q::zero()];
    let stays = |j: usize, a: &Option<Attach>| a.is_none_or(|a| s.objects[j].hung_from == Some(a.host));
    let mut placed: Vec<[Q; 4]> = boxed(k, &zero);
    for (j, a) in hung {
        if stays(*j, a) {
            placed.extend(offs.get(j).map(|at| boxed(*j, at)).unwrap_or_default());
        }
    }
    for (j, a) in hung {
        if stays(*j, a) {
            continue;
        }
        let (Some(a), Some(at)) = (a, offs.get(j).cloned()) else { continue };
        let over = |d: &[Q; 2]| boxed(*j, d).iter().any(|b| placed.iter().any(|p| overlap(b, p)));
        let mut to = at.clone();
        if over(&at) {
            let axis = match a.side {
                Side::Above | Side::Below => 0,
                Side::Left | Side::Right => 1,
            };
            let tries = (1..=MOST_STEPS).flat_map(|n| [n, -n]);
            if let Some(n) = tries.into_iter().find(|&n| {
                let mut d = at.clone();
                d[axis] += int(n);
                !over(&d)
            }) {
                to[axis] += int(n);
            }
        }
        placed.extend(boxed(*j, &to));
        offs.insert(*j, to);
    }
}

fn overlap(a: &[Q; 4], b: &[Q; 4]) -> bool {
    a[0].clone().max(b[0].clone()) < a[2].clone().min(b[2].clone())
        && a[1].clone().max(b[1].clone()) < a[3].clone().min(b[3].clone())
}
