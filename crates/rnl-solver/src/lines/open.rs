//! Describe orders left open by the retained facts.

use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::problem::{Axis, FactKind, LevelProblem, PointRef};

use crate::coords::Placed;
use crate::rational::{int, to_f64};
use crate::structure::{Role, Structure};

use super::settle::GroupSettled;

struct AxisOrder {
    class: BTreeMap<usize, usize>,
    after: BTreeMap<usize, BTreeSet<usize>>,
    fixed: BTreeSet<usize>,
}

impl AxisOrder {
    fn find(&self, k: usize) -> usize {
        self.class.get(&k).copied().unwrap_or(k)
    }

    fn orders(&self, u: usize, v: usize) -> bool {
        let (a, b) = (self.find(u), self.find(v));
        a == b
            || (self.fixed.contains(&u) && self.fixed.contains(&v))
            || self.after.get(&a).is_some_and(|s| s.contains(&b))
            || self.after.get(&b).is_some_and(|s| s.contains(&a))
    }
}

fn object_of(r: &PointRef) -> Option<usize> {
    match r {
        PointRef::Origin(k)
        | PointRef::Pin { object: k, .. }
        | PointRef::SymbolPin { object: k, .. }
        | PointRef::PinPosition { object: k, .. } => Some(*k),
        PointRef::Centre(ms) => {
            let first = ms.first().and_then(object_of)?;
            ms.iter().all(|m| object_of(m) == Some(first)).then_some(first)
        }
    }
}

pub(crate) fn open_orders(
    problem: &LevelProblem,
    s: &Structure,
    groups: &[GroupSettled],
    placed: &Placed,
    aligned: &[(usize, (usize, usize))],
) -> Vec<String> {
    if !crate::budget::permit(crate::budget::Step::Structure, problem.facts.len()) {
        return Vec::new();
    }
    let axis_of = |a: &Axis| usize::from(*a == Axis::Y);
    let mut touched: [BTreeSet<usize>; 2] = [BTreeSet::new(), BTreeSet::new()];
    let mut orders: Vec<AxisOrder> = Vec::new();
    for (a, touched_axis) in touched.iter_mut().enumerate() {
        let mut parent: BTreeMap<usize, usize> = BTreeMap::new();
        fn root(p: &mut BTreeMap<usize, usize>, k: usize) -> usize {
            let mut r = k;
            while let Some(&q) = p.get(&r) {
                if q == r {
                    break;
                }
                r = q;
            }
            r
        }
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        let mut fixed: BTreeSet<usize> = BTreeSet::new();
        for &f in &s.kept {
            match &problem.facts[f].kind {
                FactKind::Place { at, axis, .. } if axis_of(axis) == a => {
                    mark_objects(at, touched_axis);
                    if let Some(k) = object_of(at) {
                        fixed.insert(k);
                    }
                }
                FactKind::PointCoordinate { point, axis, .. } if axis_of(axis) == a => {
                    fixed.insert(*point);
                }
                _ => {}
            }
        }
        touched_axis.extend(fixed.iter().copied());
        for &f in &s.kept {
            match &problem.facts[f].kind {
                FactKind::Align { axis, members } if axis_of(axis) == a => {
                    for r in members {
                        mark_objects(r, touched_axis);
                    }
                    let ks: Vec<usize> = members.iter().filter_map(object_of).collect();
                    touched_axis.extend(ks.iter().copied());
                    for w in ks.windows(2) {
                        let (x, y) = (root(&mut parent, w[0]), root(&mut parent, w[1]));
                        if x != y {
                            parent.insert(y.max(x), y.min(x));
                        }
                    }
                }

                FactKind::Order { axis, members, .. } | FactKind::Distribute { axis, members, .. }
                    if axis_of(axis) == a =>
                {
                    for r in members {
                        mark_objects(r, touched_axis);
                    }
                    let ks: Vec<usize> = members.iter().filter_map(object_of).collect();
                    touched_axis.extend(ks.iter().copied());
                    pairs.extend(ks.windows(2).map(|w| (w[0], w[1])));
                }
                FactKind::Table { axis, rows } => {
                    for (on, from, to) in rows.orders(*axis).into_iter().filter(|(on, _, _)| axis_of(on) == a) {
                        let _ = on;
                        for r in [from, to] {
                            mark_objects(r, touched_axis);
                        }
                        if let (Some(x), Some(y)) = (object_of(from), object_of(to)) {
                            pairs.push((x, y));
                        }
                    }
                }
                FactKind::Distance { axis, from, to, .. } if axis_of(axis) == a => {
                    for r in from.iter().chain(to) {
                        mark_objects(r, touched_axis);
                    }
                    let fs: Vec<usize> = from.iter().filter_map(object_of).collect();
                    let ts: Vec<usize> = to.iter().filter_map(object_of).collect();
                    touched_axis.extend(fs.iter().chain(&ts).copied());
                    for &x in &fs {
                        pairs.extend(ts.iter().map(|&y| (x, y)));
                    }
                }
                _ => {}
            }
        }
        let keys: Vec<usize> = parent.keys().copied().collect();
        let class: BTreeMap<usize, usize> = keys.into_iter().map(|k| (k, root(&mut parent, k))).collect();
        let find = |k: usize| class.get(&k).copied().unwrap_or(k);
        let mut after: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for (x, y) in pairs {
            let (x, y) = (find(x), find(y));
            if x != y {
                after.entry(x).or_default().insert(y);
            }
        }
        loop {
            if !crate::budget::permit(crate::budget::Step::Structure, after.len().saturating_mul(after.len())) {
                return Vec::new();
            }
            let mut grew = false;
            let ks: Vec<usize> = after.keys().copied().collect();
            for k in ks {
                let next: Vec<usize> = after[&k].iter().copied().collect();
                let mut add: BTreeSet<usize> = BTreeSet::new();
                for j in next {
                    if let Some(more) = after.get(&j) {
                        add.extend(more.iter().copied());
                    }
                }
                add.remove(&k);
                let set = after.get_mut(&k).expect("key");
                let before = set.len();
                set.extend(add);
                grew |= set.len() > before;
            }
            if !grew {
                break;
            }
        }
        orders.push(AxisOrder { class, after, fixed });
    }
    let name = |k: usize| -> String {
        if s.objects[k].role == Role::Port {
            if let Some(net) = s.ports.iter().find(|p| p.object == k).and_then(|p| p.net.clone()) {
                return net;
            }
        }
        problem.objects[k].label.trim_start_matches('#').to_string()
    };
    let pin_name = |(k, p): (usize, usize)| -> String {
        let terminal = s.pins(k).get(p).and_then(|x| x.terminals.first().cloned()).unwrap_or_else(|| p.to_string());
        format!("{}:{terminal}", name(k))
    };
    let mid = |k: usize| -> Option<(f64, f64)> {
        let b = s.box_at(&placed.origins, &placed.geometry.orient, k)?;
        let m = |i: usize, j: usize| to_f64(&((&b[i] + &b[j]) / int(2)));
        Some((m(0, 2), m(1, 3)))
    };
    let axes_of = |k: usize| -> Vec<usize> { (0..2).filter(|&a| touched[a].contains(&k)).collect() };
    let aligned_to = |k: usize| aligned.iter().find(|(p, _)| *p == k).map(|(_, pin)| pin_name(*pin));
    let top_down = |ks: &[usize]| -> String {
        let mut v = ks.to_vec();
        v.sort_by(|&a, &b| {
            let (pa, pb) = (mid(a).unwrap_or_default(), mid(b).unwrap_or_default());
            pb.1.total_cmp(&pa.1).then(a.cmp(&b))
        });
        v.iter().map(|&k| name(k)).collect::<Vec<_>>().join(", ")
    };
    let left_right = |ks: &[usize]| -> String {
        let mut v = ks.to_vec();
        v.sort_by(|&a, &b| {
            let (pa, pb) = (mid(a).unwrap_or_default(), mid(b).unwrap_or_default());
            pa.0.total_cmp(&pb.0).then(a.cmp(&b))
        });
        v.iter().map(|&k| name(k)).collect::<Vec<_>>().join(", ")
    };
    let mut out = Vec::new();
    for g in groups {
        if !crate::budget::permit(crate::budget::Step::Structure, g.own.len().saturating_mul(g.all.len())) {
            return Vec::new();
        }
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for &u in &g.own {
            for &v in &g.all {
                if u == v || (g.own.contains(&v) && v < u) {
                    continue;
                }
                if !orders[0].orders(u, v) && !orders[1].orders(u, v) {
                    pairs.push((u, v));
                }
            }
        }
        if pairs.is_empty() {
            continue;
        }
        let mut in_pairs: Vec<usize> = pairs.iter().flat_map(|&(u, v)| [u, v]).collect();
        in_pairs.sort_unstable();
        in_pairs.dedup();
        let head = format!("lines: an order the facts leave open, {}:", g.name);
        let none: Vec<usize> = in_pairs.iter().copied().filter(|&k| axes_of(k).is_empty()).collect();
        if !none.is_empty() {
            let set: Vec<String> =
                none.iter().filter_map(|&k| aligned_to(k).map(|p| format!("{} on the line of {p}", name(k)))).collect();
            out.push(format!(
                "{head} {} {} no fact on where {}: the solver put {} left to right {}, top to bottom {}{}; to fix them, \
                 say which side of what each is and what it is level with",
                none.iter().map(|&k| name(k)).collect::<Vec<_>>().join(", "),
                if none.len() == 1 { "has" } else { "have" },
                if none.len() == 1 { "it is" } else { "they are" },
                if none.len() == 1 { "it" } else { "them" },
                left_right(&none),
                top_down(&none),
                if set.is_empty() { String::new() } else { format!(" ({})", set.join(", ")) }
            ));
        }
        for a in 0..2 {
            let ks: Vec<usize> = in_pairs.iter().copied().filter(|&k| axes_of(k) == [a]).collect();
            if ks.is_empty() {
                continue;
            }
            let (given, open) =
                if a == 0 { ("left and right", "up and down") } else { ("up and down", "left and right") };
            let other = if a == 0 { "y" } else { "x" };
            let set: Vec<(usize, String)> = ks.iter().filter_map(|&k| aligned_to(k).map(|p| (k, p))).collect();
            let loose: Vec<usize> = ks.iter().copied().filter(|k| !set.iter().any(|(j, _)| j == k)).collect();
            let mut chose: Vec<String> = set.iter().map(|(k, p)| format!("{} on the line of {p}", name(*k))).collect();
            if loose.len() > 1 {
                chose.push(if a == 0 {
                    format!("top to bottom {}", top_down(&loose))
                } else {
                    format!("left to right {}", left_right(&loose))
                });
            } else if let Some(&k) = loose.first() {
                chose.push(format!("{} where the table put it", name(k)));
            }
            let example =
                set.first().map_or_else(String::new, |(k, p)| format!("ALIGN({other}) {} {p}, or ", name(*k)));
            out.push(format!(
                "{head} {} {} facts {given} only, none {open}: the solver chose {}; to fix them, add {example}an ORDER on {other}",
                ks.iter().map(|&k| name(k)).collect::<Vec<_>>().join(", "),
                if ks.len() == 1 { "has" } else { "have" },
                chose.join(", ")
            ));
        }
        let placed = |k: usize| orders[0].fixed.contains(&k) && orders[1].fixed.contains(&k);
        let rest: Vec<(usize, usize)> = pairs
            .iter()
            .copied()
            .filter(|&(u, v)| axes_of(u).len() == 2 && axes_of(v).len() == 2 && !placed(u) && !placed(v))
            .collect();
        let mut cluster: BTreeMap<usize, usize> = BTreeMap::new();
        fn top(c: &BTreeMap<usize, usize>, k: usize) -> usize {
            let mut r = k;
            while let Some(&q) = c.get(&r) {
                if q == r {
                    break;
                }
                r = q;
            }
            r
        }
        for &(u, v) in &rest {
            let (a, b) = (top(&cluster, u), top(&cluster, v));
            cluster.entry(a).or_insert(a);
            cluster.entry(b).or_insert(b);
            if a != b {
                cluster.insert(a.max(b), a.min(b));
            }
        }
        let mut by: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
        for &(u, v) in &rest {
            by.entry(top(&cluster, u)).or_default().push((u, v));
        }
        for (_, ps) in by {
            let mut ks: Vec<usize> = ps.iter().flat_map(|&(u, v)| [u, v]).collect();
            ks.sort_unstable();
            ks.dedup();
            let said: Vec<String> = ps
                .iter()
                .map(|&(u, v)| {
                    let (pu, pv) = (mid(u).unwrap_or_default(), mid(v).unwrap_or_default());
                    if (pu.1 - pv.1).abs() >= (pu.0 - pv.0).abs() {
                        let (hi, lo) = if pu.1 >= pv.1 { (u, v) } else { (v, u) };
                        format!("{} above {}", name(hi), name(lo))
                    } else {
                        let (l, r) = if pu.0 <= pv.0 { (u, v) } else { (v, u) };
                        format!("{} left of {}", name(l), name(r))
                    }
                })
                .collect();
            out.push(format!(
                "{head} {} are not ordered by the facts on either axis: the solver took {}; to fix it, add an ORDER on x or y",
                ks.iter().map(|&k| name(k)).collect::<Vec<_>>().join(", "),
                said.join(", ")
            ));
        }
    }
    out
}

fn mark_objects(r: &PointRef, out: &mut BTreeSet<usize>) {
    match r {
        PointRef::Origin(k)
        | PointRef::Pin { object: k, .. }
        | PointRef::SymbolPin { object: k, .. }
        | PointRef::PinPosition { object: k, .. } => {
            out.insert(*k);
        }
        PointRef::Centre(members) => {
            for r in members {
                mark_objects(r, out);
            }
        }
    }
}
