//! Keep unwired symbols beside the pins to which their attachments connect.

use std::collections::BTreeSet;

use num_traits::Zero;

use super::read::{Key, Made, Skeleton};
use super::store::Why;
use crate::rational::{int, Q};
use crate::search::{attachments, Side};
use crate::structure::{Role, Structure};

pub(super) fn attached(sk: &mut Skeleton, s: &Structure) {
    let wired: BTreeSet<(usize, usize)> = sk
        .vertices
        .iter()
        .filter_map(|v| match v.key {
            Key::Pin(k, pin) => Some((k, pin)),
            _ => None,
        })
        .collect();
    let reads = |k: usize| sk.within.as_ref().is_none_or(|w| w.contains(&k));
    let at = |var: usize, off: &[Q; 2]| [&sk.value[0][var] + &off[0], &sk.value[1][var] + &off[1]];
    let lead = int(crate::route::CLEARANCE);
    let attach = attachments(s, &[], &sk.orient);
    let mut made: [Vec<Made>; 2] = [Vec::new(), Vec::new()];
    for (k, o) in s.objects.iter().enumerate() {
        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
            return;
        }
        if !matches!(o.role, Role::Port | Role::Supply | Role::Ground)
            || !o.own
            || o.left_free
            || o.fact_touched
            || o.stub.is_some()
            || !reads(k)
            || wired.contains(&(k, 0))
        {
            continue;
        }
        let Some(a) = attach.get(&k) else { continue };
        let (h, hp) = a.host;
        if h == k || !reads(h) {
            continue;
        }
        let (Some(var), Some(own)) = (sk.origin[k], s.pin_offset(k, 0, sk.orient[k])) else { continue };
        let (Some(hv), Some(off)) = (sk.origin[h], s.pin_offset(h, hp, sk.orient[h])) else { continue };
        let (p, q) = (at(var, &own), at(hv, &off));
        let across = if matches!(a.side, Side::Left | Side::Right) { 1 } else { 0 };
        let along = 1 - across;
        let d = &p[across] - &q[across];
        made[across].push(Made::Equal(var, own[across].clone(), hv, &off[across] + d, Why::Wire));
        let far = (&p[along] - &q[along]).abs();
        if far.is_zero() {
            made[along].push(Made::Equal(var, own[along].clone(), hv, off[along].clone(), Why::Wire));
            continue;
        }
        let least = if far < lead { far } else { lead.clone() };
        made[along].push(if p[along] > q[along] {
            Made::AtLeast(var, own[along].clone(), hv, off[along].clone(), least, Why::Wire)
        } else {
            Made::AtLeast(hv, off[along].clone(), var, own[along].clone(), least, Why::Wire)
        });
    }
    for (axis, m) in made.into_iter().enumerate() {
        sk.made[axis].extend(m);
    }
}
