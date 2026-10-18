//! The layered score of a routed candidate: each layer the weighted sum of its objectives, layers
//! compared lexicographically within their tolerances, ties by the canonical code (plan 5.8).
//!
//! The objectives are measured by the score functions of their table (`crate::objectives`) on
//! the candidate as it will be drawn: the positions and orientations the coordinate LP gave it
//! (`crate::coords`) and its wires, those routed for the ROUTEs (`crate::route`) and those written
//! (13.1), drawn by their mode. Before routing, the routing objectives measure the straight
//! connections of each net; that is the estimate the driver picks the candidates to route by.
//!
//! Candidate ranking uses only configured level totals and the canonical code. At each level,
//! keep the tolerance window anchored at the least remaining value, then continue to the next
//! level. Collision diagnostics and routing fallback reports add no hidden ranking counters.
use std::cell::OnceCell;
use std::collections::BTreeSet;

use num_traits::{One, Zero};
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::LevelProblem;

use crate::coords::solid;
use crate::objectives::{Active, Routed, View};
use crate::rational::Q;
use crate::resolve::Geometry;
use crate::route::Routing;
use crate::structure::Structure;

/// A candidate as the order of candidates sees it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    pub layers: Vec<Q>,
    pub code: Vec<u32>,
}

/// The wires a routed candidate is drawn with, as the routing objectives measure them: the wires
/// routed, each with its net, and the wires written, drawn by their mode. A wire written on a
/// net that a ROUTE asks for counts as of that net, so that the routed wires that meet it do not
/// count as crossing another net.
pub(crate) fn drawn(problem: &LevelProblem, s: &Structure, g: &Geometry, routing: &Routing) -> Routed {
    let geometry = if routing.geometry.wires.is_empty() {
        crate::routing_geometry::RoutingGeometry {
            wires: (0..problem.wires.len())
                .filter_map(|w| crate::routing_geometry::authored_wire(problem, g, w))
                .collect(),
        }
    } else {
        routing.geometry.clone()
    };
    let paths = geometry
        .wires
        .iter()
        .map(|w| {
            let net =
                s.connections.drawn.iter().position(|d| w.nets.contains(&d.net)).unwrap_or(s.connections.drawn.len());
            (net, w.vertices.iter().map(|v| v.at.clone()).collect())
        })
        .collect();
    Routed { paths, geometry }
}

/// The value of each layer of the objectives in force on a placed candidate: with `routed`, the
/// routing objectives measure its wires; without, the straight connections of each net.
pub(crate) fn layers(s: &Structure, origins: &[[Q; 2]], g: &Geometry, routed: Option<&Routed>) -> Vec<Q> {
    let view = View { s, at: origins, orient: &g.orient };
    s.active.score(&view, routed)
}

/// Exact geometry shared only by the terms evaluated on this one immutable view. Eligibility
/// remains with each measure: components, solid bodies, label owners and net pins differ.
pub(crate) struct ViewGeometry<'a> {
    pub view: View<'a>,
    boxes: OnceCell<Vec<Option<[Q; 4]>>>,
    pins: OnceCell<Vec<Vec<Option<[Q; 2]>>>>,
    authored: OnceCell<Routed>,
}
impl<'a> ViewGeometry<'a> {
    pub fn new(view: View<'a>) -> Self {
        Self { view, boxes: OnceCell::new(), pins: OnceCell::new(), authored: OnceCell::new() }
    }
    pub fn bbox(&self, k: usize) -> Option<&[Q; 4]> {
        self.boxes
            .get_or_init(|| {
                (0..self.view.s.objects.len()).map(|k| self.view.s.box_at(self.view.at, self.view.orient, k)).collect()
            })
            .get(k)?
            .as_ref()
    }
    pub fn pins(&self, k: usize) -> &[Option<[Q; 2]>] {
        &self.pins.get_or_init(|| {
            (0..self.view.s.objects.len())
                .map(|k| {
                    (0..self.view.s.pins(k).len())
                        .map(|pin| self.view.s.pin_at(self.view.at, self.view.orient, k, pin))
                        .collect()
                })
                .collect()
        })[k]
    }
    pub fn point(&self, p: crate::model::PointId) -> Option<[Q; 2]> {
        match p {
            crate::model::PointId::Pin(k, pin) => self.pins(k).get(pin)?.clone(),
            _ => self.view.s.point_at(self.view.at, self.view.orient, p),
        }
    }
    fn authored(&self) -> &Routed {
        self.authored.get_or_init(|| {
            let g = Geometry { origin: self.view.at.to_vec(), orient: self.view.orient.to_vec() };
            drawn(&self.view.s.problem, self.view.s, &g, &Routing::default())
        })
    }
    pub fn overlaps(&self) -> usize {
        let boxes: Vec<_> =
            (0..self.view.s.objects.len()).filter(|&k| solid(self.view.s, k)).filter_map(|k| self.bbox(k)).collect();
        (0..boxes.len())
            .map(|i| {
                boxes
                    .iter()
                    .skip(i + 1)
                    .filter(|b| {
                        let a = boxes[i];
                        a[0].clone().max(b[0].clone()) < a[2].clone().min(b[2].clone())
                            && a[1].clone().max(b[1].clone()) < a[3].clone().min(b[3].clone())
                    })
                    .count()
            })
            .sum()
    }
    pub fn texts_over(&self) -> usize {
        let s = self.view.s;
        let bodies: Vec<_> =
            (0..s.objects.len()).filter(|&k| solid(s, k)).filter_map(|k| Some((k, self.bbox(k)?))).collect();
        let host = |k: usize| s.objects[k].stub.as_ref().map(|stub| stub.host.0);
        let owners: BTreeSet<usize> = s.labels.keys().chain(s.offset_labels.keys()).copied().collect();
        let mut count = 0;
        for k in owners.into_iter().filter(|&k| !s.objects[k].left_free) {
            let (Some(at), Some(&o)) = (self.view.at.get(k), self.view.orient.get(k)) else { continue };
            let placed = |b: &[Q; 4]| [&at[0] + &b[0], &at[1] + &b[1], &at[0] + &b[2], &at[1] + &b[3]];
            let texts: Vec<_> = s
                .label_offset(k, o)
                .into_iter()
                .chain(s.offset_labels.get(&k).into_iter().flatten().cloned())
                .map(|b| placed(&b))
                .collect();
            for (j, body) in &bodies {
                if *j == k || host(*j) == Some(k) || host(k) == Some(*j) {
                    continue;
                }
                count += texts.iter().filter(|t| meet(t, body)).count();
            }
        }
        count
    }
    pub fn wire_penalty(&self, routed: Option<&Routed>, shorts: bool) -> usize {
        let s = self.view.s;
        let r = routed.unwrap_or_else(|| self.authored());
        let nets: Vec<BTreeSet<&str>> =
            r.geometry.wires.iter().map(|w| w.nets.iter().map(String::as_str).collect()).collect();
        let segments: Vec<_> = r
            .geometry
            .wires
            .iter()
            .enumerate()
            .filter(|(i, _)| !nets[*i].is_empty())
            .flat_map(|(i, w)| w.vertices.windows(2).map(move |v| (i, &v[0].at, &v[1].at)))
            .filter(|(_, a, b)| a != b)
            .collect();
        let mut count = 0;
        if shorts {
            for (&(k, pin), on) in &s.connections.nets_of {
                if s.objects[k].left_free {
                    continue;
                }
                let names: BTreeSet<_> =
                    on.iter().filter_map(|&i| s.connections.nets.get(i)).map(|n| n.name.as_str()).collect();
                if names.is_empty() {
                    continue;
                }
                let Some(p) = self.pins(k).get(pin).and_then(Option::as_ref) else { continue };
                count +=
                    segments.iter().filter(|(i, a, b)| nets[*i].is_disjoint(&names) && on_segment(p, a, b)).count();
            }
        } else {
            for k in (0..s.objects.len()).filter(|&k| solid(s, k)) {
                let Some(b) = self.bbox(k) else { continue };
                let inner: Vec<_> = self
                    .pins(k)
                    .iter()
                    .flatten()
                    .filter(|p| b[0] < p[0] && p[0] < b[2] && b[1] < p[1] && p[1] < b[3])
                    .collect();
                count += segments
                    .iter()
                    .filter(|(_, a, c)| through(b, a, c) && !inner.iter().any(|p| *p == *a || *p == *c))
                    .count();
            }
        }
        count
    }
}

/// The places where the wires of a candidate, drawn as they will be (`routed`: the wires routed and
/// the wires written, by their mode), read as what they are not. Lines that touch, cross or
/// overlap make no connection (13.2), and a reader takes wires that touch for connected, so each
/// of these counts once:
///
/// - two segments of different nets on one line that share a stretch of it, or an end;
/// - two segments of different nets, not on one line, that meet at an end or a bend of one of
///   them (where they meet inside both, they cross: that is the objective `crossings`);
/// - a pin on a segment of a net it is not on, of the nets the level draws: the net of a pin is
///   known from its ROUTE, a wire written on that one net that ends at it, or a usage that attaches
///   its symbol to the net (12.1, `Connections::nets_of`). The problem holds no connections of the
///   netlist, so a pin of a net that nothing draws, or one that only the wire of a bus reaches, is
///   on no net known, and a wire over it is not counted; nor is a pin of a symbol that the
///   solution leaves free, which is not drawn (`crate::structure::ObjectInfo::left_free`);
/// - a segment through the inside of a symbol's box over some length, but one that ends at a pin
///   of that symbol inside its box, which has no other way to the pin.
///
/// Nets are compared by name, a wire of a bus being on each of its nets: two wires are of
/// different nets when they share none. A wire on no net is left out.
#[cfg(test)]
pub(crate) fn misleading(
    problem: &LevelProblem,
    s: &Structure,
    origins: &[[Q; 2]],
    g: &Geometry,
    routed: &Routed,
) -> usize {
    let drawn = &s.connections.drawn;
    let first_written = drawn.len().saturating_sub(problem.wires.len());
    // the nets of an entry of `drawn`: a ROUTE's, or a wire written's
    let nets_of = |d: usize| -> BTreeSet<&str> {
        match d.checked_sub(first_written).and_then(|w| problem.wires.get(w)) {
            Some(w) => w.nets.iter().map(String::as_str).collect(),
            None => drawn.get(d).map(|e| e.net.as_str()).into_iter().collect(),
        }
    };
    let nets: Vec<BTreeSet<&str>> = routed.paths.iter().map(|(d, _)| nets_of(*d)).collect();
    let segments: Vec<(usize, &[Q; 2], &[Q; 2])> = routed
        .paths
        .iter()
        .enumerate()
        .filter(|(i, _)| !nets[*i].is_empty())
        .flat_map(|(i, (_, path))| path.windows(2).map(move |w| (i, &w[0], &w[1])))
        .filter(|(_, a, b)| a != b)
        .collect();
    let mut count = 0;
    for (n, (i, a, b)) in segments.iter().enumerate() {
        for (j, c, d) in &segments[n + 1..] {
            if nets[*i].is_disjoint(&nets[*j]) && touch(a, b, c, d) {
                count += 1;
            }
        }
    }
    for (&(k, pin), on) in s.connections.nets_of.iter().filter(|((k, _), _)| !s.objects[*k].left_free) {
        let names: BTreeSet<&str> =
            on.iter().filter_map(|&i| s.connections.nets.get(i)).map(|net| net.name.as_str()).collect();
        if names.is_empty() {
            continue;
        }
        let Some(p) = s.pin_at(origins, &g.orient, k, pin) else { continue };
        count += segments.iter().filter(|(i, a, b)| nets[*i].is_disjoint(&names) && on_segment(&p, a, b)).count();
    }
    for k in (0..s.objects.len()).filter(|&k| solid(s, k)) {
        let Some(b) = s.box_at(origins, &g.orient, k) else { continue };
        let inner: Vec<[Q; 2]> = s
            .connections
            .nets_of
            .range((k, 0)..(k + 1, 0))
            .filter_map(|(&(_, pin), _)| s.pin_at(origins, &g.orient, k, pin))
            .filter(|p| b[0] < p[0] && p[0] < b[2] && b[1] < p[1] && p[1] < b[3])
            .collect();
        count +=
            segments.iter().filter(|(_, p, q)| through(&b, p, q) && !inner.iter().any(|x| x == *p || x == *q)).count();
    }
    count
}

/// Step1 splits the former misleading diagnostic into pin/line and line/body measures.
/// Per-segment counting is retained until Step2, without the old line/line penalty.
pub(crate) fn wire_penalty(view: &View, routed: Option<&Routed>, shorts: bool) -> usize {
    let s = view.s;
    let g = Geometry { origin: view.at.to_vec(), orient: view.orient.to_vec() };
    let authored;
    let r = match routed {
        Some(r) => r,
        None => {
            authored = drawn(&s.problem, s, &g, &Routing::default());
            &authored
        }
    };
    let nets: Vec<BTreeSet<&str>> =
        r.geometry.wires.iter().map(|w| w.nets.iter().map(String::as_str).collect()).collect();
    let segments: Vec<_> = r
        .geometry
        .wires
        .iter()
        .enumerate()
        .filter(|(i, _)| !nets[*i].is_empty())
        .flat_map(|(i, w)| w.vertices.windows(2).map(move |v| (i, &v[0].at, &v[1].at)))
        .filter(|(_, a, b)| a != b)
        .collect();
    let mut count = 0;
    if shorts {
        for (&(k, pin), on) in &s.connections.nets_of {
            if s.objects[k].left_free {
                continue;
            }
            let names: BTreeSet<_> =
                on.iter().filter_map(|&i| s.connections.nets.get(i)).map(|n| n.name.as_str()).collect();
            if names.is_empty() {
                continue;
            }
            let Some(p) = s.pin_at(view.at, view.orient, k, pin) else { continue };
            count += segments.iter().filter(|(i, a, b)| nets[*i].is_disjoint(&names) && on_segment(&p, a, b)).count();
        }
    } else {
        for k in (0..s.objects.len()).filter(|&k| solid(s, k)) {
            let Some(b) = s.box_at(view.at, view.orient, k) else { continue };
            let inner: Vec<_> = (0..s.pins(k).len())
                .filter_map(|pin| s.pin_at(view.at, view.orient, k, pin))
                .filter(|p| b[0] < p[0] && p[0] < b[2] && b[1] < p[1] && p[1] < b[3])
                .collect();
            count += segments
                .iter()
                .filter(|(_, a, c)| through(&b, a, c) && !inner.iter().any(|p| p == *a || p == *c))
                .count();
        }
    }
    count
}

/// The texts drawn over the box of another symbol, with some area (13.3): for each object drawn
/// with texts of its symbol, the box of those texts, as the solver keeps them clear of its
/// neighbours (`Structure::labels`), and of each text that offsets place from its origin
/// (`Structure::offset_labels`), against the box of every symbol drawn (`crate::coords::solid`)
/// but its own, a stub that hangs from it and its host when it is a stub, each pair of a box of
/// texts and a symbol counted once. What the solution leaves free is not drawn, and counts
/// neither way. A known gap of the measure: a stub's own texts drawn over its host's body, such
/// as the `vdd` of a `$SUPPLY_STUB` turned MX, are not counted, as the pair of the stub and its
/// host is left out whole (review of the fix of phase 2: 5 of 20 levels of a transistor sweep).
pub(crate) fn texts_over(s: &Structure, origins: &[[Q; 2]], ori: &[Orient]) -> usize {
    let bodies: Vec<(usize, [Q; 4])> =
        (0..s.objects.len()).filter(|&k| solid(s, k)).filter_map(|k| Some((k, s.box_at(origins, ori, k)?))).collect();
    let host = |k: usize| s.objects[k].stub.as_ref().map(|stub| stub.host.0);
    let owners: BTreeSet<usize> = s.labels.keys().chain(s.offset_labels.keys()).copied().collect();
    let mut count = 0;
    for k in owners.into_iter().filter(|&k| !s.objects[k].left_free) {
        let (Some(at), Some(&o)) = (origins.get(k), ori.get(k)) else { continue };
        let placed = |b: &[Q; 4]| [&at[0] + &b[0], &at[1] + &b[1], &at[0] + &b[2], &at[1] + &b[3]];
        let texts: Vec<[Q; 4]> = s
            .label_offset(k, o)
            .into_iter()
            .chain(s.offset_labels.get(&k).into_iter().flatten().cloned())
            .map(|b| placed(&b))
            .collect();
        for (j, body) in &bodies {
            if *j == k || host(*j) == Some(k) || host(k) == Some(*j) {
                continue;
            }
            count += texts.iter().filter(|t| meet(t, body)).count();
        }
    }
    count
}

/// Whether two boxes overlap with some area.
fn meet(a: &[Q; 4], b: &[Q; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// `(p - o) x (q - o)`: 0 when the three points are on one line.
fn cross(o: &[Q; 2], p: &[Q; 2], q: &[Q; 2]) -> Q {
    (&p[0] - &o[0]) * (&q[1] - &o[1]) - (&p[1] - &o[1]) * (&q[0] - &o[0])
}

/// The least and the greatest coordinate of two points on an axis.
fn span<'a>(a: &'a [Q; 2], b: &'a [Q; 2], i: usize) -> (&'a Q, &'a Q) {
    if a[i] <= b[i] {
        (&a[i], &b[i])
    } else {
        (&b[i], &a[i])
    }
}

/// Whether `p` is on the segment from `a` to `b`, its ends included.
fn on_segment(p: &[Q; 2], a: &[Q; 2], b: &[Q; 2]) -> bool {
    let within = |i: usize| {
        let (lo, hi) = span(a, b, i);
        *lo <= p[i] && p[i] <= *hi
    };
    within(0) && within(1) && cross(a, b, p).is_zero()
}

/// Whether two segments touch, as a reader takes for a connection: an end of one is on the other.
/// That covers two segments on one line that share a point or a stretch, and leaves out two that
/// cross, meeting inside both. Segments whose boxes are apart are told at once.
#[cfg(test)]
fn touch(a: &[Q; 2], b: &[Q; 2], c: &[Q; 2], d: &[Q; 2]) -> bool {
    let apart = |i: usize| {
        let ((lo1, hi1), (lo2, hi2)) = (span(a, b, i), span(c, d, i));
        hi1 < lo2 || hi2 < lo1
    };
    if apart(0) || apart(1) {
        return false;
    }
    on_segment(c, a, b) || on_segment(d, a, b) || on_segment(a, c, d) || on_segment(b, c, d)
}

/// Whether the segment from `a` to `b` runs through the inside of a box, its edge left out, over
/// some length: the parameters along it at which it is strictly inside the box's range on each
/// axis make an interval that is not empty.
fn through(bx: &[Q; 4], a: &[Q; 2], b: &[Q; 2]) -> bool {
    for i in 0..2 {
        let (lo, hi) = span(a, b, i);
        if *hi <= bx[i] || bx[i + 2] <= *lo {
            return false;
        }
    }
    let (mut lo, mut hi) = (Q::zero(), Q::one());
    for axis in 0..2 {
        let (min, max) = (&bx[axis], &bx[axis + 2]);
        let d = &b[axis] - &a[axis];
        if d.is_zero() {
            if !(*min < a[axis] && a[axis] < *max) {
                return false;
            }
            continue;
        }
        let (t0, t1) = ((min - &a[axis]) / &d, (max - &a[axis]) / &d);
        let (t0, t1) = if t0 <= t1 { (t0, t1) } else { (t1, t0) };
        lo = lo.max(t0);
        hi = hi.min(t1);
    }
    lo < hi
}

/// The order of some candidates, best first (see the module): the fewest overlaps, then the least
/// crowding, then the fewest places where the wires mislead, then the fewest joins left undone,
/// then the layers within their tolerances, then the code.
pub(crate) fn order(active: &Active, entries: &[Entry]) -> Vec<usize> {
    let refs: Vec<(&[Q], &[u32])> = entries.iter().map(|e| (e.layers.as_slice(), e.code.as_slice())).collect();
    active.rank(&refs)
}

#[cfg(test)]
#[path = "../tests/unit/score.rs"]
mod tests;
