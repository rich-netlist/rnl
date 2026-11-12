//! Measure hidden ports and move eligible ports towards a free edge.

use std::collections::BTreeSet;

use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{LevelProblem, PointRef};

use crate::budget::{permit, Step};
use crate::coords::Placed;
use crate::rational::{int, Q};
use crate::route::Routing;
use crate::structure::{Role, Structure};
fn tip() -> Q {
    int(3) / int(5)
}
pub(crate) fn hidden(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    within: Option<&BTreeSet<usize>>,
) -> Vec<usize> {
    if !permit(Step::Structure, problem.objects.len() + routing.wires.len()) {
        return Vec::new();
    }
    let g = &placed.geometry;
    let reads = |k: usize| within.is_none_or(|w| w.contains(&k));
    let ports: Vec<usize> = s
        .components()
        .filter(|&k| reads(k) && s.objects[k].role == Role::Port && s.objects[k].hung_from.is_none())
        .collect();
    if ports.is_empty() {
        return Vec::new();
    }
    let net_names = |k: usize, pin: usize| -> Vec<&str> {
        s.connections
            .nets_of
            .get(&(k, pin))
            .into_iter()
            .flatten()
            .map(|&n| s.connections.nets[n].name.as_str())
            .collect()
    };
    let mut segs: Vec<(&str, [Q; 2], [Q; 2])> = Vec::new();
    for (w, path) in routing.wires.iter().zip(&routing.paths) {
        let Some(route) = problem.routes.get(w.route) else { continue };
        for pair in path.windows(2) {
            segs.push((route.net.as_str(), pair[0].clone(), pair[1].clone()));
        }
    }
    for (w, wire) in problem.wires.iter().enumerate() {
        let read = wire.anchors.iter().all(|r| reads_reference(r, &reads));
        let Some(net) = wire.nets.first().filter(|_| read) else { continue };
        let Ok(Some(path)) = crate::route::written_path(problem, g, w) else { continue };
        for pair in path.windows(2) {
            segs.push((net.as_str(), pair[0].clone(), pair[1].clone()));
        }
    }
    let boxes: Vec<(usize, [Q; 4])> = s
        .components()
        .filter(|&k| reads(k))
        .filter_map(|k| s.box_at(&placed.origins, &g.orient, k).map(|b| (k, b)))
        .collect();
    let mut out = Vec::new();
    for &k in &ports {
        if !permit(Step::Structure, boxes.len() + segs.len()) {
            return Vec::new();
        }
        let (Some(pin), Some(exit)) = (s.pin_at(&placed.origins, &g.orient, k, 0), s.exit(k, 0, g.orient[k])) else {
            continue;
        };
        let Some(&net) = net_names(k, 0).first() else { continue };
        let on_net = |j: usize| (0..s.pins(j).len()).all(|p| net_names(j, p).contains(&net));
        let blocked = |d: (i64, i64)| {
            let t = [&pin[0] + &(tip() * int(d.0)), &pin[1] + &(tip() * int(d.1))];
            let (along, across) = if d.0 != 0 { (0, 1) } else { (1, 0) };
            let sign = int(d.0 + d.1);
            let ahead = |v: &Q| (v - &t[along]) * &sign >= int(0);
            let by_box = boxes.iter().any(|(j, b)| {
                *j != k
                    && !on_net(*j)
                    && b[across] < t[across]
                    && t[across] < b[across + 2]
                    && (if sign > int(0) { b[along + 2] > t[along] } else { b[along] < t[along] })
            });
            by_box
                || segs.iter().any(|(n, a, b)| {
                    if *n == net {
                        return false;
                    }
                    if a[across] == b[across] {
                        return a[across] == t[across] && (ahead(&a[along]) || ahead(&b[along]));
                    }
                    let (lo, hi) = if a[across] < b[across] { (a, b) } else { (b, a) };
                    if t[across] < lo[across] || t[across] > hi[across] {
                        return false;
                    }
                    let u = (&t[across] - &lo[across]) / (&hi[across] - &lo[across]);
                    ahead(&(&lo[along] + &(u * (&hi[along] - &lo[along]))))
                })
        };
        let out_way = (-exit.0, -exit.1);
        let free = [(1, 0), (0, 1), (-1, 0), (0, -1)].into_iter().filter(|&d| !blocked(d)).count();
        if blocked(out_way) || free <= 1 {
            out.push(k);
        }
    }
    out
}
pub(crate) fn segments(problem: &LevelProblem, placed: &Placed, routing: &Routing) -> Vec<(String, [Q; 2], [Q; 2])> {
    segments_but(problem, placed, routing, None)
}
fn segments_but(
    problem: &LevelProblem,
    placed: &Placed,
    routing: &Routing,
    but: Option<usize>,
) -> Vec<(String, [Q; 2], [Q; 2])> {
    let mut segs = Vec::new();
    for (i, (w, path)) in routing.wires.iter().zip(&routing.paths).enumerate() {
        if Some(i) == but {
            continue;
        }
        let Some(route) = problem.routes.get(w.route) else { continue };
        for pair in path.windows(2) {
            segs.push((route.net.clone(), pair[0].clone(), pair[1].clone()));
        }
    }
    for (w, wire) in problem.wires.iter().enumerate() {
        let (Some(n), Ok(Some(path))) = (wire.nets.first(), crate::route::written_path(problem, &placed.geometry, w))
        else {
            continue;
        };
        for pair in path.windows(2) {
            segs.push((n.clone(), pair[0].clone(), pair[1].clone()));
        }
    }
    segs
}
pub(crate) fn to_edge(
    problem: &LevelProblem,
    s: &Structure,
    mut placed: Placed,
    mut routing: Routing,
    tolerance: f64,
    notes: &mut Vec<String>,
) -> (Placed, Routing) {
    use rnl_runtime::problem::Axis;
    if !permit(Step::Candidate, problem.objects.len()) {
        return (placed, routing);
    }
    let tol = crate::rational::from_f64(tolerance).unwrap_or_else(|| int(0));
    let broken = |placed: &Placed| -> BTreeSet<usize> {
        let g = &placed.geometry;
        let at = |m: &PointRef, axis: Axis| crate::resolve::point_value(problem, g, m, usize::from(axis == Axis::Y), 0);
        let ori = |k: usize| g.orient.get(k).copied();
        s.kept
            .iter()
            .copied()
            .filter(|&f| crate::holds::check_problem(problem, f, &at, &ori, &tol, &tol) != Some(true))
            .collect()
    };
    let label = |k: usize| problem.objects[k].label.clone();
    let (mut moved, mut turns, mut stayed): (Vec<String>, Vec<String>, Vec<String>) =
        (Vec::new(), Vec::new(), Vec::new());
    for k in hidden(problem, s, &placed, &routing, None) {
        if !permit(Step::Candidate, problem.objects.len() + routing.wires.len()) {
            break;
        }
        let now = hidden(problem, s, &placed, &routing, None);
        if !now.contains(&k) {
            continue;
        }
        match out_of_hiding(problem, s, &placed, &routing, k, &broken, now.len()) {
            Ok((p, r, to))
                if !crate::budget::stopped()
                    && (0..problem.wires.len())
                        .all(|w| crate::route::written_path(problem, &p.geometry, w).is_ok()) =>
            {
                placed = p;
                routing = r;
                moved.push(format!("{} to {to}", label(k)));
            }
            Ok(_) => break,
            Err(_) if crate::budget::stopped() => break,
            Err(why) => match turned(problem, s, &placed, &routing, k, &broken, now.len()) {
                Ok((p, r, to))
                    if !crate::budget::stopped()
                        && (0..problem.wires.len())
                            .all(|w| crate::route::written_path(problem, &p.geometry, w).is_ok()) =>
                {
                    placed = p;
                    routing = r;
                    turns.push(format!("{} to {to}", label(k)));
                }
                Ok(_) => break,
                Err(_) if crate::budget::stopped() => break,
                Err(not) => stayed.push(format!("{} ({why}; not turned: {not})", label(k))),
            },
        }
    }
    if !moved.is_empty() || !turns.is_empty() || !stayed.is_empty() {
        notes.push(format!(
            "ports hidden moved out along their way past the wires that shut them in: [{}]; turned to face away from \
             what they are wired to: [{}]; left where they are: [{}]",
            moved.join(", "),
            turns.join(", "),
            stayed.join(", ")
        ));
    }
    (placed, routing)
}
const TURNS: [Orient; 7] =
    [Orient::MY, Orient::MX, Orient::R180, Orient::R90, Orient::R270, Orient::MYR90, Orient::MXR90];
fn turned(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    k: usize,
    broken: &dyn Fn(&Placed) -> BTreeSet<usize>,
    hidden_before: usize,
) -> Result<(Placed, Routing, String), &'static str> {
    use rnl_runtime::problem::Position;
    use rnl_runtime::solution::WirePoint;
    if !permit(Step::Candidate, problem.objects.len() + routing.wires.len()) {
        return Err("budget exhausted");
    }
    if !matches!(problem.objects[k].position, Position::Own { .. }) {
        return Err("its position is defined by another object");
    }
    let g = &placed.geometry;
    let pin = s.pin_at(&placed.origins, &g.orient, k, 0).ok_or("no pin")?;
    let was = g.orient[k];
    let exit = s.exit(k, 0, was).ok_or("no way out")?;
    if problem.wires.iter().any(|w| {
        w.anchors
            .iter()
            .any(|a| matches!(a, PointRef::Pin { object, .. } | PointRef::SymbolPin { object, .. } if *object == k))
    }) {
        return Err("a wire written to it");
    }
    let at_port = |w: Option<&WirePoint>| matches!(w, Some(WirePoint::Anchor(PointRef::Pin { object, .. } | PointRef::SymbolPin { object, .. })) if *object == k);
    let ends: Vec<usize> = (0..routing.wires.len())
        .filter(|&i| at_port(routing.wires[i].points.first()) || at_port(routing.wires[i].points.last()))
        .collect();
    let &[w] = ends.as_slice() else { return Err("not one wire routed to it") };
    let (wire, path) = (&routing.wires[w], &routing.paths[w]);
    if path.len() < 2 || path.len() != wire.points.len() {
        return Err("no wire to it");
    }
    let back = !at_port(wire.points.first());
    let mut from: Vec<[Q; 2]> = path.clone();
    if back {
        from.reverse();
    }
    if from[0] != pin || from.windows(2).any(|q| q[0][0] != q[1][0] && q[0][1] != q[1][1]) {
        return Err("a wire that is not level and upright");
    }
    let far = &from[from.len() - 1];
    let (dx, dy) = (far[0].clone() - &pin[0], far[1].clone() - &pin[1]);
    let toward = if dx.clone().abs() >= dy.clone().abs() {
        (if dx > int(0) { 1 } else { -1 }, 0)
    } else {
        (0, if dy > int(0) { 1 } else { -1 })
    };
    if dx == int(0) && dy == int(0) {
        return Err("its wire goes nowhere");
    }
    if exit == toward {
        return Err("it faces away from its wire's far end already");
    }
    let to = TURNS
        .iter()
        .map(|&t| was.then(t))
        .find(|&o| s.exit(k, 0, o) == Some(toward))
        .ok_or("no way of its symbol faces so")?;
    let turn = Orient::ALL.into_iter().find(|&t| was.then(t) == to).ok_or("no way of its symbol faces so")?;
    let (a, b, c, d) = turn.matrix();
    let rotate = |v: &[Q; 2], about: &[Q; 2]| -> [Q; 2] {
        let (x, y) = (&v[0] - &about[0], &v[1] - &about[1]);
        [
            &about[0] + &(int(i64::from(a)) * &x + int(i64::from(b)) * &y),
            &about[1] + &(int(i64::from(c)) * &x + int(i64::from(d)) * &y),
        ]
    };
    let mut p = placed.clone();
    p.geometry.orient[k] = to;
    let offset = s.pin_offset(k, 0, to).ok_or("no pin")?;
    let origin = [&pin[0] - &offset[0], &pin[1] - &offset[1]];
    p.geometry.origin[k].clone_from(&origin);
    p.origins[k] = origin;
    for (j, o) in problem.objects.iter().enumerate() {
        let placed_by_it = matches!(&o.position, Position::InSymbol { owner, replaced: Some(_), .. } if *owner == k);
        let by_it = matches!(o.position, Position::Own { .. })
            && s.anchors.get(&j).is_some_and(|a| !a.from.is_empty() && a.from.iter().all(|(m, _)| *m == k));
        if placed_by_it || by_it {
            let at = rotate(&placed.origins[j], &pin);
            match &o.position {
                Position::InSymbol { replaced: Some((axis, _)), .. } => {
                    let axis = usize::from(*axis == rnl_runtime::problem::Axis::Y);
                    p.geometry.origin[j][axis].clone_from(&at[axis]);
                }
                _ => p.geometry.origin[j] = at,
            }
        }
    }
    p.origins = s.origins(problem, &p.geometry);
    let net = problem.routes.get(wire.route).map(|r| r.net.clone()).ok_or("no net")?;
    let segs = segments_but(problem, placed, routing, Some(w));
    let boxes: Vec<(usize, [Q; 4])> = s
        .components()
        .filter(|&j| j != k)
        .filter_map(|j| s.box_at(&p.origins, &p.geometry.orient, j).map(|bx| (j, bx)))
        .collect();
    let crossings = |path: &[[Q; 2]]| -> usize {
        path.windows(2).map(|q| segs.iter().filter(|(n, a, b)| *n != net && crosses(&q[0], &q[1], a, b)).count()).sum()
    };
    let before = crossings(&from);
    let e = [int(toward.0), int(toward.1)];
    let (along, across) = if toward.0 != 0 { (0, 1) } else { (1, 0) };
    type PathChoice = ((usize, usize, Q), Vec<[Q; 2]>);
    let mut best: Option<PathChoice> = None;
    for i in 1..from.len() - 1 {
        let q = &from[i];
        let ahead = (&q[along] - &pin[along]) * &e[along];
        let mut leads: Vec<Vec<[Q; 2]>> = Vec::new();
        if q[across] == pin[across] && ahead > int(0) {
            leads.push(vec![pin.clone(), q.clone()]);
        }
        let mut corner = pin.clone();
        corner[along].clone_from(&q[along]);
        leads.push(vec![pin.clone(), corner, q.clone()]);
        for out in [1, 2] {
            let mut first = pin.clone();
            first[along] = &pin[along] + &(&e[along] * int(out));
            let mut second = first.clone();
            second[across].clone_from(&q[across]);
            leads.push(vec![pin.clone(), first, second, q.clone()]);
        }
        for lead in leads {
            if !permit(Step::Route, from.len() + segs.len() + boxes.len()) {
                return Err("budget exhausted");
            }
            let mut made = lead;
            made.extend(from[i + 1..].iter().cloned());
            let Some(made) = simplified(made) else { continue };
            let leaves = (&made[1][0] - &made[0][0], &made[1][1] - &made[0][1]);
            if leaves.0.clone() * &e[0] + leaves.1.clone() * &e[1] <= int(0) {
                continue;
            }
            let (n, m) = (made.len(), from.len());
            if !same_way(&made[n - 2], &made[n - 1], &from[m - 2], &from[m - 1]) {
                continue;
            }
            let new: Vec<([Q; 2], [Q; 2])> = made
                .windows(2)
                .map(|q| (q[0].clone(), q[1].clone()))
                .filter(|(a, b)| !from.windows(2).any(|o| (&o[0], &o[1]) == (a, b)))
                .collect();
            let meets = |a: &[Q; 2], b: &[Q; 2], c: &[Q; 2], d: &[Q; 2]| {
                crosses(a, b, c, d)
                    || overlaps(a, b, c, d)
                    || [c, d].into_iter().any(|x| x != far && on(x, a, b))
                    || [a, b].into_iter().any(|x| x != far && on(x, c, d))
            };
            let clear = new.iter().all(|(a, b)| {
                !boxes.iter().any(|(_, bx)| through(a, b, bx))
                    && !segs.iter().any(|(n, c, d)| overlaps(a, b, c, d) || (*n == net && meets(a, b, c, d)))
                    && !s.components().filter(|&j| j != k).any(|j| {
                        (0..s.pins(j).len()).any(|pn| {
                            let on_net = s
                                .connections
                                .nets_of
                                .get(&(j, pn))
                                .into_iter()
                                .flatten()
                                .any(|&x| s.connections.nets[x].name == net);
                            !on_net && s.pin_at(&p.origins, &p.geometry.orient, j, pn).is_some_and(|x| on(&x, a, b))
                        })
                    })
            });
            if !clear {
                continue;
            }
            let length = made
                .windows(2)
                .map(|q| (&q[1][0] - &q[0][0]).abs() + (&q[1][1] - &q[0][1]).abs())
                .fold(int(0), |x, y| x + y);
            let key = (crossings(&made), made.len(), length);
            if best.as_ref().is_none_or(|(b, _)| key < *b) {
                best = Some((key, made));
            }
        }
    }
    let Some(((crossed, _, _), mut made)) = best else { return Err("no way to draw its wire again") };
    if crossed > before {
        return Err("its wire would cross more");
    }
    let mut r = routing.clone();
    if back {
        made.reverse();
    }
    let (first, last) = (wire.points[0].clone(), wire.points[wire.points.len() - 1].clone());
    let mut points = vec![first];
    points.extend(
        made[1..made.len() - 1]
            .iter()
            .map(|v| WirePoint::Bend((crate::rational::to_f64(&v[0]), crate::rational::to_f64(&v[1])))),
    );
    points.push(last);
    r.wires[w].points = points;
    r.paths[w] = made;
    refresh_paths(problem, &p, &mut r)?;
    if s.box_at(&p.origins, &p.geometry.orient, k)
        .is_some_and(|ob| boxes.iter().any(|(_, bx)| ob[0] < bx[2] && bx[0] < ob[2] && ob[1] < bx[3] && bx[1] < ob[3]))
    {
        return Err("it would overlap a symbol");
    }
    let after = hidden(problem, s, &p, &r, None);
    if after.contains(&k) || after.len() >= hidden_before {
        return Err("it would still be hidden");
    }
    if !broken(&p).is_subset(&broken(placed)) {
        return Err("a fact holds its way");
    }
    if crate::budget::stopped() {
        return Err("budget exhausted");
    }
    Ok((p, r, to.as_str().to_string()))
}
fn simplified(path: Vec<[Q; 2]>) -> Option<Vec<[Q; 2]>> {
    let mut out: Vec<[Q; 2]> = Vec::new();
    for v in path {
        if out.last() == Some(&v) {
            continue;
        }
        if out.len() >= 2 {
            let (a, b) = (&out[out.len() - 2], &out[out.len() - 1]);
            let level = a[1] == b[1] && b[1] == v[1];
            let upright = a[0] == b[0] && b[0] == v[0];
            if level || upright {
                let axis = usize::from(upright);
                if (&b[axis] - &a[axis]) * (&v[axis] - &b[axis]) < int(0) {
                    return None;
                }
                out.pop();
            }
        }
        out.push(v);
    }
    (out.len() >= 2).then_some(out)
}
fn same_way(a: &[Q; 2], b: &[Q; 2], c: &[Q; 2], d: &[Q; 2]) -> bool {
    let sign = |x: Q| {
        if x > int(0) {
            1
        } else if x < int(0) {
            -1
        } else {
            0
        }
    };
    (sign(&b[0] - &a[0]), sign(&b[1] - &a[1])) == (sign(&d[0] - &c[0]), sign(&d[1] - &c[1]))
}
fn crosses(a: &[Q; 2], b: &[Q; 2], c: &[Q; 2], d: &[Q; 2]) -> bool {
    let between = |x: &Q, p: &Q, q: &Q| (p < x && x < q) || (q < x && x < p);
    if a[1] == b[1] && c[0] == d[0] {
        return between(&c[0], &a[0], &b[0]) && between(&a[1], &c[1], &d[1]);
    }
    if a[0] == b[0] && c[1] == d[1] {
        return between(&a[0], &c[0], &d[0]) && between(&c[1], &a[1], &b[1]);
    }
    false
}
fn overlaps(a: &[Q; 2], b: &[Q; 2], c: &[Q; 2], d: &[Q; 2]) -> bool {
    for axis in 0..2 {
        let other = 1 - axis;
        if a[other] == b[other] && c[other] == d[other] && a[other] == c[other] {
            let (lo, hi) = (a[axis].clone().min(b[axis].clone()), a[axis].clone().max(b[axis].clone()));
            let (lo2, hi2) = (c[axis].clone().min(d[axis].clone()), c[axis].clone().max(d[axis].clone()));
            if lo.max(lo2) < hi.min(hi2) {
                return true;
            }
        }
    }
    false
}
fn on(x: &[Q; 2], a: &[Q; 2], b: &[Q; 2]) -> bool {
    let within = |v: &Q, p: &Q, q: &Q| p.clone().min(q.clone()) <= *v && *v <= p.clone().max(q.clone());
    ((a[0] == b[0] && x[0] == a[0]) || (a[1] == b[1] && x[1] == a[1]))
        && within(&x[0], &a[0], &b[0])
        && within(&x[1], &a[1], &b[1])
}
fn through(a: &[Q; 2], b: &[Q; 2], bx: &[Q; 4]) -> bool {
    let (x0, x1) = (a[0].clone().min(b[0].clone()), a[0].clone().max(b[0].clone()));
    let (y0, y1) = (a[1].clone().min(b[1].clone()), a[1].clone().max(b[1].clone()));
    if a[1] == b[1] {
        bx[1] < a[1] && a[1] < bx[3] && x0 < bx[2] && bx[0] < x1
    } else {
        bx[0] < a[0] && a[0] < bx[2] && y0 < bx[3] && bx[1] < y1
    }
}
fn out_of_hiding(
    problem: &LevelProblem,
    s: &Structure,
    placed: &Placed,
    routing: &Routing,
    k: usize,
    broken: &dyn Fn(&Placed) -> BTreeSet<usize>,
    hidden_before: usize,
) -> Result<(Placed, Routing, String), &'static str> {
    use rnl_runtime::problem::Position;
    use rnl_runtime::solution::WirePoint;
    if !permit(Step::Candidate, problem.objects.len() + routing.wires.len()) {
        return Err("budget exhausted");
    }
    if !matches!(problem.objects[k].position, Position::Own { .. }) {
        return Err("its position is defined by another object");
    }
    let g = &placed.geometry;
    let pin = s.pin_at(&placed.origins, &g.orient, k, 0).ok_or("no pin")?;
    let exit = s.exit(k, 0, g.orient[k]).ok_or("no way out")?;
    let way = (-exit.0, -exit.1);
    let (along, across) = if way.0 != 0 { (0, 1) } else { (1, 0) };
    let sign = int(way.0 + way.1);
    let positive = sign > int(0);
    let net = s
        .connections
        .nets_of
        .get(&(k, 0))
        .and_then(|ns| ns.first())
        .map(|&n| s.connections.nets[n].name.clone())
        .ok_or("no net")?;
    let at_port = |w: Option<&WirePoint>| matches!(w, Some(WirePoint::Anchor(PointRef::Pin { object, .. } | PointRef::SymbolPin { object, .. })) if *object == k);
    let mut wired = false;
    for (w, path) in routing.wires.iter().zip(&routing.paths) {
        if path.len() < 2 {
            continue;
        }
        for (end, next) in [(w.points.first(), &path[1]), (w.points.last(), &path[path.len() - 2])] {
            if at_port(end) {
                wired = true;
                if next[across] != pin[across] {
                    return Err("a wire that leaves it across its line");
                }
            }
        }
    }
    for (w, wire) in problem.wires.iter().enumerate() {
        if !wire
            .anchors
            .iter()
            .any(|a| matches!(a, PointRef::Pin { object, .. } | PointRef::SymbolPin { object, .. } if *object == k))
        {
            continue;
        }
        wired = true;
        let path = crate::route::written_path(problem, g, w)
            .map_err(|_| "a written wire has unevaluable geometry")?
            .ok_or("a wire written that is not drawn")?;
        for (i, q) in path.iter().enumerate() {
            if *q != pin {
                continue;
            }
            let next = [i.checked_sub(1), Some(i + 1)].into_iter().flatten().filter_map(|j| path.get(j));
            if next.into_iter().any(|n| n[across] != pin[across]) {
                return Err("a wire written that leaves it across its line");
            }
        }
    }
    if !wired {
        return Err("no wire to it");
    }
    let further = |a: &Q, b: &Q| if positive { a > b } else { a < b };
    let boxes: Vec<(usize, [Q; 4])> = s
        .components()
        .filter(|&j| j != k)
        .filter_map(|j| s.box_at(&placed.origins, &g.orient, j).map(|b| (j, b)))
        .collect();
    let segs = segments(problem, placed, routing);
    let t = [&pin[0] + &(tip() * int(way.0)), &pin[1] + &(tip() * int(way.1))];
    for (_, b) in &boxes {
        let beyond = if positive { b[along + 2] > t[along] } else { b[along] < t[along] };
        if b[across] < t[across] && t[across] < b[across + 2] && beyond {
            return Err("a symbol in its way");
        }
    }
    let mut farthest: Option<Q> = None;
    for (n, a, b) in &segs {
        if *n == net {
            continue;
        }
        let hit = if a[across] == b[across] {
            if a[across] == t[across] && (!further(&t[along], &a[along]) || !further(&t[along], &b[along])) {
                return Err("another net's wire along its way");
            }
            continue;
        } else {
            let (lo, hi) = if a[across] < b[across] { (a, b) } else { (b, a) };
            if t[across] < lo[across] || t[across] > hi[across] {
                continue;
            }
            let u = (&t[across] - &lo[across]) / (&hi[across] - &lo[across]);
            &lo[along] + &(u * (&hi[along] - &lo[along]))
        };
        if further(&hit, &t[along]) && farthest.as_ref().is_none_or(|f| further(&hit, f)) {
            farthest = Some(hit);
        }
    }
    let mut edge = pin[along].clone();
    for (_, b) in &boxes {
        let v = if positive { &b[along + 2] } else { &b[along] };
        if further(v, &edge) {
            edge = v.clone();
        }
    }
    for (_, a, b) in &segs {
        for v in [&a[along], &b[along]] {
            if further(v, &edge) {
                edge = v.clone();
            }
        }
    }
    let past = |v: &Q| -> Q {
        if positive {
            v.ceil() + int(2)
        } else {
            v.floor() - int(2)
        }
    };
    let mut targets: Vec<Q> = farthest.iter().map(past).collect();
    targets.push(past(&edge));
    targets.dedup();
    let before = broken(placed);
    let mut why = "nowhere further to go";
    for to in targets {
        if !permit(Step::Placement, problem.objects.len() + routing.wires.len()) {
            return Err("budget exhausted");
        }
        if !further(&to, &pin[along]) {
            continue;
        }
        let delta = &to - &pin[along];
        let mut p = placed.clone();
        p.geometry.origin[k][along] += &delta;
        p.origins[k][along] += &delta;
        for (j, o) in problem.objects.iter().enumerate() {
            let placed_by_it =
                matches!(&o.position, Position::InSymbol { owner, replaced: Some(_), .. } if *owner == k);
            let by_it = matches!(o.position, Position::Own { .. })
                && s.anchors.get(&j).is_some_and(|a| !a.from.is_empty() && a.from.iter().all(|(m, _)| *m == k));
            if (placed_by_it || by_it)
                && !matches!(&o.position, Position::InSymbol { replaced: Some((axis, _)), .. }
                    if usize::from(*axis == rnl_runtime::problem::Axis::Y) != along)
            {
                p.geometry.origin[j][along] += &delta;
            }
        }
        p.origins = s.origins(problem, &p.geometry);
        let new_pin = s.pin_at(&p.origins, &p.geometry.orient, k, 0).ok_or("no pin")?;
        let mut r = routing.clone();
        for (w, path) in r.wires.iter().zip(r.paths.iter_mut()) {
            if path.len() < 2 {
                continue;
            }
            let last = path.len() - 1;
            for (end, i) in [(w.points.first(), 0), (w.points.last(), last)] {
                if at_port(end) {
                    path[i] = new_pin.clone();
                }
            }
        }
        refresh_paths(problem, &p, &mut r)?;
        let (lo, hi) = if pin[along] < new_pin[along] { (&pin, &new_pin) } else { (&new_pin, &pin) };
        let through_box = boxes.iter().any(|(_, b)| {
            b[across] < pin[across] && pin[across] < b[across + 2] && b[along] < hi[along] && lo[along] < b[along + 2]
        });
        let along_wire = segs.iter().any(|(n, a, b)| {
            *n != net
                && a[across] == b[across]
                && a[across] == pin[across]
                && a[along].clone().max(b[along].clone()) > lo[along]
                && a[along].clone().min(b[along].clone()) < hi[along]
        });
        let over_pin = s.components().filter(|&j| j != k).any(|j| {
            (0..s.pins(j).len()).any(|q| {
                let on_net = s
                    .connections
                    .nets_of
                    .get(&(j, q))
                    .into_iter()
                    .flatten()
                    .any(|&n| s.connections.nets[n].name == net);
                !on_net
                    && s.pin_at(&p.origins, &p.geometry.orient, j, q)
                        .is_some_and(|x| x[across] == pin[across] && lo[along] <= x[along] && x[along] <= hi[along])
            })
        });
        let overlaps = s
            .box_at(&p.origins, &p.geometry.orient, k)
            .is_some_and(|ob| boxes.iter().any(|(_, b)| ob[0] < b[2] && b[0] < ob[2] && ob[1] < b[3] && b[1] < ob[3]));
        let slanted = problem.wires.iter().enumerate().any(|(w, wire)| {
            let touches = wire.anchors.iter().any(
                |a| matches!(a, PointRef::Pin { object, .. } | PointRef::SymbolPin { object, .. } if *object == k),
            );
            touches
                && crate::route::written_path(problem, &p.geometry, w).map_or(true, |path| {
                    path.is_some_and(|path| path.windows(2).any(|q| q[0][0] != q[1][0] && q[0][1] != q[1][1]))
                })
        });
        if through_box || slanted {
            why = "its wire would run through a symbol";
            continue;
        }
        if along_wire || over_pin {
            why = "its wire would run along another net's wire or over its pin";
            continue;
        }
        if overlaps {
            why = "it would overlap a symbol";
            continue;
        }
        let after = hidden(problem, s, &p, &r, None);
        if after.contains(&k) || after.len() >= hidden_before {
            why = "it would still be hidden";
            continue;
        }
        if !broken(&p).is_subset(&before) {
            why = "a fact holds it";
            continue;
        }
        if crate::budget::stopped() {
            return Err("budget exhausted");
        }
        let axis = if along == 0 { "x" } else { "y" };
        return Ok((p, r, format!("{axis} = {}", crate::rational::fmt(&to))));
    }
    Err(why)
}

/// Geometric scope includes every member of a centre, independently of connectivity.
pub(super) fn reads_reference(r: &PointRef, reads: &impl Fn(usize) -> bool) -> bool {
    match r {
        PointRef::Origin(k)
        | PointRef::Pin { object: k, .. }
        | PointRef::SymbolPin { object: k, .. }
        | PointRef::PinPosition { object: k, .. } => reads(*k),
        PointRef::Centre(members) => members.iter().all(|r| reads_reference(r, reads)),
    }
}

/// Keep the route cache consistent with every geometric anchor after moving a port.
/// Position-only anchors move geometrically without acquiring electrical identity.
fn refresh_paths(problem: &LevelProblem, placed: &Placed, routing: &mut Routing) -> Result<(), &'static str> {
    use rnl_runtime::solution::WirePoint;
    for (wire, path) in routing.wires.iter().zip(&mut routing.paths) {
        if !permit(Step::Route, wire.points.len()) {
            return Err("budget exhausted");
        }
        if wire.points.len() != path.len() {
            return Err("incomplete routed geometry");
        }
        let old = path.clone();
        for (point, at) in wire.points.iter().zip(path.iter_mut()) {
            if let WirePoint::Anchor(reference) = point {
                let value = |axis| {
                    crate::resolve::point_value(problem, &placed.geometry, reference, axis, 0)
                        .ok_or("an anchor has no position")
                };
                *at = [value(0)?, value(1)?];
            }
        }
        if old.windows(2).zip(path.windows(2)).any(|(old, new)| {
            (old[0][0] == old[1][0] || old[0][1] == old[1][1]) && new[0][0] != new[1][0] && new[0][1] != new[1][1]
        }) {
            return Err("a dependent wire would become slanted");
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::resolve::tests::{fact, instance, problem_of, Ticks};
    use crate::{RunLimit, SolverOptions, StoppedClock, Work};
    use rnl_runtime::problem::{FactKind, Position, Reference, RouteIntent};
    use rnl_runtime::solution::{RoutedWire, WirePoint};

    fn pin(k: usize) -> PointRef {
        PointRef::SymbolPin { object: k, pin: 0, reference: Reference::Anonymous }
    }

    pub(in crate::lines) fn fixture() -> (LevelProblem, Structure, Placed, Routing) {
        let mut p = problem_of(vec![instance(), instance()], vec![]);
        p.version = "0.4".into();
        p.symbols[0].pins.truncate(1);
        p.symbols[0].pins[0].pos = (0., 0.);
        let route = |net: &str, pins| RouteIntent {
            net: net.into(),
            tree: true,
            mode: "auto".into(),
            style: vec![],
            pins,
            wires: vec![],
        };
        p.routes = vec![route("a", vec![pin(0), pin(1)]), route("b", vec![])];
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
        let mut s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
        s.objects[0].role = Role::Port;
        s.objects[0].hung_from = None;
        let mut geometry = s.witness.clone();
        geometry.origin = vec![[int(0), int(0)], [int(10), int(0)]];
        // Pick the orientation whose outward side points left.
        geometry.orient[0] = Orient::ALL.into_iter().find(|&o| s.exit(0, 0, o) == Some((1, 0))).unwrap();
        let placed = Placed {
            origins: s.origins(&p, &geometry),
            geometry,
            dropped: vec![],
            overlaps: vec![],
            optimised: true,
            undecided: false,
        };
        let routing = Routing {
            wires: vec![
                RoutedWire { route: 0, points: vec![WirePoint::Anchor(pin(0)), WirePoint::Anchor(pin(1))] },
                RoutedWire { route: 1, points: vec![WirePoint::Bend((-2., -4.)), WirePoint::Bend((-2., 4.))] },
            ],
            paths: vec![vec![[int(0), int(0)], [int(10), int(0)]], vec![[int(-2), int(-4)], [int(-2), int(4)]]],
            drawn: vec![0, 1],
            ..Default::default()
        };
        (p, s, placed, routing)
    }

    #[test]
    fn port_moves_are_complete_transactions_and_replay_time_cutoffs() {
        let (p, s, placed, routing) = fixture();
        assert_eq!(hidden(&p, &s, &placed, &routing, None), [0]);
        let run = || {
            let mut notes = vec![];
            let (p, r) = to_edge(&p, &s, placed.clone(), routing.clone(), 1e-6, &mut notes);
            (p, r, notes)
        };
        let complete = run();
        assert!(complete.0.origins[0][0] < int(-2));
        assert!(hidden(&p, &s, &complete.0, &complete.1, None).is_empty());
        for seconds in 0..24 {
            let (timed, used) = crate::budget::with(&Ticks::default(), RunLimit::Time(seconds as f64), None, run);
            let (work, _) = crate::budget::with(&StoppedClock, RunLimit::Work(used.work), None, run);
            assert_eq!(timed, work, "cutoff {seconds}");
            assert!(timed.0 == placed || timed.0 == complete.0, "partial candidate at {seconds}");
            assert_eq!(timed.0.origins, s.origins(&p, &timed.0.geometry));
            if timed.0 == placed {
                assert_eq!(timed.1, routing);
            }
        }
    }

    #[test]
    fn route_cache_follows_position_only_and_indirect_anchors_without_retyping_them() {
        let (mut p, s, mut placed, mut routing) = fixture();
        p.objects[1].position = Position::At(PointRef::Origin(0));
        let reference = PointRef::PinPosition { object: 1, pin: 0, reference: Reference::Anonymous };
        routing.wires[0].points[1] = WirePoint::Anchor(reference.clone());
        placed.geometry.origin[0][0] = int(-4);
        placed.origins = s.origins(&p, &placed.geometry);
        refresh_paths(&p, &placed, &mut routing).unwrap();
        assert_eq!(routing.paths[0], vec![[int(-4), int(0)]; 2]);
        assert_eq!(routing.wires[0].points[1], WirePoint::Anchor(reference));
    }

    #[test]
    fn centre_scope_checks_all_nested_members_and_exact_facts_keep_all_orientations() {
        let nested = PointRef::Centre(vec![pin(0), PointRef::Centre(vec![PointRef::Origin(1)])]);
        assert!(!reads_reference(&nested, &|k| k == 0));
        let (mut p, _, placed, _) = fixture();
        let chosen = placed.geometry.orient[0];
        let other = Orient::ALL.into_iter().find(|o| *o != chosen).unwrap();
        p.facts = vec![fact(FactKind::Orient { object: 0, code: other })];
        p.orient_lists = vec![(0, vec![other, chosen])];
        let ori = |k| placed.geometry.orient.get(k).copied();
        assert_eq!(crate::holds::check_problem(&p, 0, &|_, _| None, &ori, &int(0), &int(0)), Some(true));
        p.orient_lists[0].1.pop();
        assert_eq!(crate::holds::check_problem(&p, 0, &|_, _| None, &ori, &int(0), &int(0)), Some(false));
    }
}
