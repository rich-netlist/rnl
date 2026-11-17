//! Turn a symbol whose pin a fact puts on another's to match it, and the boxes such symbols take.

use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{Axis, FactKind, LevelProblem, PointRef};

use crate::rational::Q;
use crate::search::preference;
use crate::structure::{PinKey, Structure};

pub(crate) fn face(s: &Structure, ori: &mut [Orient]) {
    for (from, to, pairs) in &s.pin_pairs {
        pair(s, *from, *to, pairs, ori);
    }
}

#[allow(dead_code)]
pub(crate) fn follow(s: &Structure, k: usize, ori: &mut [Orient]) {
    for (from, to, pairs) in s.pin_pairs.iter().filter(|p| p.1 == k) {
        pair(s, *from, *to, pairs, ori);
    }
}

fn pair(s: &Structure, from: usize, to: usize, pairs: &[(Orient, Orient)], ori: &mut [Orient]) {
    let alone = s.class_of(to).is_some_and(|c| s.open.classes[c].members.len() == 1);
    let a = ori[from];
    if !alone || pairs.contains(&(a, ori[to])) {
        return;
    }
    let mut allowed: Vec<Orient> = pairs.iter().filter(|(x, _)| *x == a).map(|(_, b)| *b).collect();
    allowed.sort_by_key(|v| preference(*v));
    if let Some(&b) = allowed.first() {
        ori[to] = b;
    }
}

pub(crate) fn on_pin(problem: &LevelProblem, kept: &[usize], k: usize) -> Option<(PinKey, usize)> {
    let pins = |axis: Axis| {
        kept.iter().filter_map(move |&f| {
            let FactKind::Align { axis: a, members } = &problem.facts.get(f)?.kind else { return None };
            let [PointRef::Pin { object: o1, pin: p1, .. }
            | PointRef::SymbolPin { object: o1, pin: p1, .. }
            | PointRef::PinPosition { object: o1, pin: p1, .. }, PointRef::Pin { object: o2, pin: p2, .. }
            | PointRef::SymbolPin { object: o2, pin: p2, .. }
            | PointRef::PinPosition { object: o2, pin: p2, .. }] = members.as_slice()
            else {
                return None;
            };
            if *a != axis || o1 == o2 {
                return None;
            }
            if *o2 == k {
                Some(((*o1, *p1), *p2))
            } else if *o1 == k {
                Some(((*o2, *p2), *p1))
            } else {
                None
            }
        })
    };
    let ys: Vec<(PinKey, usize)> = pins(Axis::Y).collect();
    pins(Axis::X).find(|x| ys.contains(x))
}

pub(crate) fn held(s: &Structure, host: PinKey, ori: &[Orient]) -> Vec<[Q; 4]> {
    let Some(hp) = s.pin_offset(host.0, host.1, ori[host.0]) else { return Vec::new() };
    let mut out = Vec::new();
    for (&j, &(h, own)) in &s.pinned {
        let Some(op) = s.pin_offset(j, own, ori[j]).filter(|_| h == host) else { continue };
        let at = [&hp[0] - &op[0], &hp[1] - &op[1]];
        for b in [s.box_offset(j, ori[j]), s.label_offset(j, ori[j])].into_iter().flatten() {
            out.push([&b[0] + &at[0], &b[1] + &at[1], &b[2] + &at[0], &b[3] + &at[1]]);
        }
    }
    out
}
