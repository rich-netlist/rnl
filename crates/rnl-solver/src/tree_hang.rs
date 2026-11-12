//! CURRENT-directed ordering and placement of ports and unmarked elements.

use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::geom::Orient;

use crate::search::{grounded_end_down, stub_on_body};
use crate::structure::{FreeClass, Source, Structure};
use crate::tree::Item;

pub(crate) fn current_order(s: &Structure, class: &FreeClass) -> Vec<(Orient, u8)> {
    let ok = |v: Orient| {
        class.members.iter().all(|&(m, label)| {
            let o = v.then(label);
            if stub_on_body(s, m, o) || (s.has(Source::GroundsDown) && !grounded_end_down(s, m, o)) {
                return false;
            }
            !s.has(Source::GroundsDown) || hangs_upright(s, m, o)
        })
    };
    let rank = |v: Orient| {
        let mut worst = 0u8;
        for &(m, label) in &class.members {
            if s.objects[m].left_free {
                continue;
            }
            let o = v.then(label);
            for ((_, pin), out) in s.currents.range((m, 0)..(m + 1, 0)) {
                let Some(e) = s.exit(m, *pin, o) else { continue };
                let with = if *out { (0, -1) } else { (0, 1) };
                worst = worst.max(if e == with {
                    0
                } else if e == (-with.0, -with.1) {
                    2
                } else {
                    1
                });
            }
        }
        worst
    };
    let mut out: Vec<(Orient, u8)> = Orient::ALL.into_iter().filter(|v| ok(*v)).map(|v| (v, rank(v))).collect();
    if out.is_empty() {
        out = Orient::ALL.into_iter().map(|v| (v, rank(v))).collect();
    }
    out.sort_by_key(|(_, r)| *r);
    out
}

pub(crate) fn current_ranks(s: &Structure) -> Vec<Vec<(Orient, u8)>> {
    s.open.classes.iter().map(|class| current_order(s, class)).collect()
}

pub(crate) fn move_rank(
    ranks: &[Vec<(Orient, u8)>],
    from: &crate::tree::Arrangement,
    to: &crate::tree::Arrangement,
) -> u8 {
    let mut worst = 0;
    for (c, values) in ranks.iter().enumerate() {
        let (Some(a), Some(b)) = (from.values.get(c), to.values.get(c)) else { continue };
        if a != b {
            worst = worst.max(values.iter().find(|(v, _)| v == b).map_or(2, |(_, r)| *r));
        }
    }
    worst
}

pub(crate) fn ordered_ports(
    before: &BTreeSet<(usize, usize)>,
    items: &[Item],
    is_port: &dyn Fn(&Item) -> bool,
    leaves: &dyn Fn(&Item) -> Vec<usize>,
) -> Vec<Item> {
    let others: BTreeSet<usize> = items.iter().filter(|it| !is_port(it)).flat_map(leaves).collect();
    items
        .iter()
        .filter(|it| is_port(it))
        .filter(|it| {
            leaves(it).iter().any(|&k| {
                before.iter().any(|&(a, b)| (a == k && others.contains(&b)) || (b == k && others.contains(&a)))
            })
        })
        .cloned()
        .collect()
}

pub(crate) fn joined_within(s: &Structure, leaves: &BTreeSet<usize>, placed: &BTreeSet<usize>) -> bool {
    let joined: Vec<usize> = leaves.iter().flat_map(|&k| s.attach_pins(k)).map(|p| p.0).collect();
    !joined.is_empty() && joined.iter().all(|k| placed.contains(k))
}

pub(crate) fn stacked(
    row: Vec<Item>,
    port: &dyn Fn(&Item) -> bool,
    apart: &dyn Fn(&Item, &Item) -> bool,
    stack: &dyn Fn(Vec<Item>) -> Item,
) -> Vec<Item> {
    let mut out = Vec::new();
    let mut run: Vec<Item> = Vec::new();
    for it in row {
        if port(&it) && !run.iter().any(|r| apart(r, &it)) {
            run.push(it);
            continue;
        }
        if !run.is_empty() {
            out.push(stack(std::mem::take(&mut run)));
        }
        if port(&it) {
            run.push(it);
        } else {
            out.push(it);
        }
    }
    if !run.is_empty() {
        out.push(stack(run));
    }
    out
}

pub(crate) fn levels(s: &Structure, level: &mut BTreeMap<usize, i64>) {
    if s.depth.is_none() {
        return;
    }
    let c = &s.connections;
    let marked = |k: usize| (0..s.pins(k).len()).any(|p| s.currents.contains_key(&(k, p)));
    let free: Vec<usize> = s.elements.iter().copied().filter(|&k| !marked(k)).collect();
    let mut placed: BTreeSet<usize> = s.elements.iter().copied().filter(|&k| marked(k)).collect();
    if placed.is_empty() {
        return;
    }
    loop {
        let mut found: Vec<(usize, i64)> = Vec::new();
        for &k in free.iter().filter(|k| !placed.contains(k)) {
            if !crate::budget::permit(crate::budget::Step::Structure, s.pins(k).len()) {
                return;
            }
            let mut nets: Vec<usize> = (0..s.pins(k).len())
                .flat_map(|p| c.nets_of.get(&(k, p)).into_iter().flatten().copied())
                .filter(|&i| !c.nets[i].supply && !c.nets[i].ground)
                .collect();
            nets.sort_by_key(|&i| (c.nets[i].pins.len(), i));
            nets.dedup();
            for i in nets {
                let there: BTreeSet<usize> =
                    c.nets[i].pins.iter().map(|q| q.0).filter(|&q| q != k && placed.contains(&q)).collect();
                let mut ls: Vec<i64> = there.iter().filter_map(|q| level.get(q).copied()).collect();
                if ls.is_empty() {
                    continue;
                }
                ls.sort_unstable();
                found.push((k, ls[(ls.len() - 1) / 2]));
                break;
            }
        }
        if found.is_empty() {
            return;
        }
        for (k, l) in found {
            level.insert(k, l);
            placed.insert(k);
        }
    }
}

pub(crate) fn hangs_upright(s: &Structure, m: usize, o: Orient) -> bool {
    let info = &s.objects[m];
    let above = match info.role {
        crate::structure::Role::Ground => true,
        crate::structure::Role::Supply
            if info.stub.is_none() && s.attach_pins(m).iter().any(|p| s.power.supply_up.contains(p)) =>
        {
            false
        }
        _ => return true,
    };
    match (s.pin_offset(m, 0, o), s.box_offset(m, o)) {
        (Some(p), Some(b)) => (p[1].clone() * crate::rational::int(2) > &b[1] + &b[3]) == above,
        _ => true,
    }
}

/// A symbol made by a usage whose parent binds a role to an electrical pin.
pub(crate) fn host(problem: &rnl_runtime::problem::LevelProblem, k: usize) -> Option<crate::structure::PinKey> {
    use rnl_runtime::problem::Filler;
    let usage = problem.usages.get(problem.objects.get(k)?.usage?)?;
    let parent = problem.usages.get(usage.parent?)?;
    parent.roles.iter().flat_map(|r| &r.members).find_map(|m| match m {
        Filler::Terminal { object, pin: Some(pin), .. } | Filler::Pin { object, pin } => Some((*object, *pin)),
        _ => None,
    })
}
