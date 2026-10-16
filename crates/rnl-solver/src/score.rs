//! The layered score of a routed candidate: each layer the weighted sum of its objectives, layers
//! compared lexicographically within their tolerances, ties by the canonical code (plan 5.8).
//!
//! The objectives are measured by the score functions of their table (`crate::objectives`) on
//! the candidate as it will be drawn: the positions and orientations the coordinate LP gave it
//! (`crate::coords`) and its wires, those routed for the ROUTEs (`crate::route`) and those written
//! (13.1), drawn by their mode. Before routing, the routing objectives measure the straight
//! connections of each net; that is the estimate the driver picks the candidates to route by.
//!
//! **The order of candidates** ([`order`]) has three counts ahead of the objectives, as the
//! solver's own rules rank after every fact and before every objective (8.5, 11.7): first the
//! pairs of symbols that overlap; then the places where the wires mislead ([`misleading`]): lines
//! that touch or overlap make no connection (13.2), so wires of different nets that do, a pin on a
//! wire of another net, or a wire through a symbol read as what they are not; then the joins that
//! routing left undone, a ROUTE (13.7) being a request of the file and the objectives only
//! preferences of the solver. The wires routed avoid all of it by themselves; the count is there
//! for the wires written (13.1), which the placement moves and their mode draws. Among candidates
//! equal in the three, the layers decide as `crate::objectives::Active::rank` does: layer by
//! layer, the candidates within the layer's tolerance of its least value kept, the canonical code
//! breaking what is left tied. An objective that is off has no layer, so it neither ranks nor
//! breaks ties.

use std::collections::BTreeSet;

use num_traits::{One, Zero};
use rnl_runtime::problem::LevelProblem;

use crate::coords::solid;
use crate::objectives::{Active, Routed, View};
use crate::rational::Q;
use crate::resolve::Geometry;
use crate::route::{written_path, Routing};
use crate::structure::Structure;

/// A candidate as the order of candidates sees it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    /// The pairs of symbols that overlap.
    pub overlaps: usize,
    /// The places where the wires mislead ([`misleading`]); before routing, those of the wires
    /// written.
    pub misleading: usize,
    /// The joins routing left undone (`Routing::joins_left`); 0 before routing.
    pub joins_left: usize,
    /// The value of each layer of the objectives in force.
    pub layers: Vec<Q>,
    /// The canonical code (`crate::search::Candidate::code`).
    pub code: Vec<u32>,
}

/// The wires a routed candidate is drawn with, as the routing objectives measure them: the wires
/// routed, each with its net, and the wires written, drawn by their mode. A wire written on a
/// net that a ROUTE asks for counts as of that net, so that the routed wires that meet it do not
/// count as crossing another net.
pub(crate) fn drawn(problem: &LevelProblem, s: &Structure, g: &Geometry, routing: &Routing) -> Routed {
    let drawn = &s.connections.drawn;
    // a net by the first entry drawn with its name
    let net = |d: usize| drawn.iter().position(|e| drawn.get(d).is_some_and(|x| x.net == e.net)).unwrap_or(d);
    let mut paths: Vec<(usize, Vec<[Q; 2]>)> =
        routing.drawn.iter().zip(&routing.paths).map(|(d, path)| (net(*d), path.clone())).collect();
    let first_written = drawn.len().saturating_sub(problem.wires.len());
    for w in 0..problem.wires.len() {
        if let Some(path) = written_path(problem, g, w) {
            paths.push((net(first_written + w), path));
        }
    }
    Routed { paths }
}

/// The value of each layer of the objectives in force on a placed candidate: with `routed`, the
/// routing objectives measure its wires; without, the straight connections of each net.
pub(crate) fn layers(s: &Structure, origins: &[[Q; 2]], g: &Geometry, routed: Option<&Routed>) -> Vec<Q> {
    let view = View { s, at: origins, orient: &g.orient };
    s.active.score(&view, routed)
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

/// The order of some candidates, best first (see the module): the fewest overlaps, then the fewest
/// places where the wires mislead, then the fewest joins left undone, then the layers within
/// their tolerances, then the code.
pub(crate) fn order(active: &Active, entries: &[Entry]) -> Vec<usize> {
    let key = |e: &Entry| (e.overlaps, e.misleading, e.joins_left);
    let mut keys: Vec<(usize, usize, usize)> = entries.iter().map(key).collect();
    keys.sort_unstable();
    keys.dedup();
    let mut out = Vec::with_capacity(entries.len());
    for k in keys {
        let members: Vec<usize> = (0..entries.len()).filter(|&i| key(&entries[i]) == k).collect();
        let refs: Vec<(&[Q], &[u32])> =
            members.iter().map(|&i| (entries[i].layers.as_slice(), entries[i].code.as_slice())).collect();
        out.extend(active.rank(&refs).into_iter().map(|j| members[j]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objectives::Inputs;
    use crate::rational::int;
    use crate::{Layer, Objectives};

    fn entry(overlaps: usize, joins_left: usize, layers: &[i64], code: u32) -> Entry {
        Entry {
            overlaps,
            misleading: 0,
            joins_left,
            layers: layers.iter().map(|v| int(*v)).collect(),
            code: vec![code],
        }
    }

    #[test]
    fn overlaps_then_joins_left_then_the_layers_then_the_code() {
        let active = Active::new(
            &Objectives { layers: vec![Layer::single("area"), Layer::single("aspect")] },
            &Inputs::default(),
        );
        let entries = [
            entry(1, 0, &[0, 0], 0),
            entry(0, 2, &[0, 0], 1),
            entry(0, 0, &[5, 0], 2),
            entry(0, 0, &[3, 9], 3),
            entry(0, 0, &[3, 1], 5),
            entry(0, 0, &[3, 1], 4),
            entry(0, 1, &[0, 0], 6),
        ];
        assert_eq!(order(&active, &entries), vec![5, 4, 3, 2, 6, 1, 0]);
    }

    fn pt(x: i64, y: i64) -> [Q; 2] {
        [int(x), int(y)]
    }

    #[test]
    fn what_misleads_ranks_after_the_overlaps_and_before_the_joins_left() {
        let active = Active::new(&Objectives { layers: vec![Layer::single("area")] }, &Inputs::default());
        let mut entries = [entry(0, 0, &[5], 0), entry(0, 0, &[1], 1), entry(1, 0, &[0], 2), entry(0, 1, &[0], 3)];
        entries[1].misleading = 2;
        entries[0].misleading = 1;
        // the overlaps first, then what misleads, then the joins left, then the layers
        assert_eq!(order(&active, &entries), vec![3, 0, 1, 2]);
    }

    #[test]
    fn segments_touch_at_an_end_or_along_a_line_and_not_where_they_cross() {
        // along one line: a stretch shared, an end on the other, ends that meet, apart
        assert!(touch(&pt(0, 0), &pt(4, 0), &pt(2, 0), &pt(6, 0)));
        assert!(touch(&pt(0, 0), &pt(4, 0), &pt(4, 0), &pt(6, 0)));
        assert!(!touch(&pt(0, 0), &pt(4, 0), &pt(5, 0), &pt(6, 0)));
        // a T: the end of one inside the other; a bend on the other
        assert!(touch(&pt(0, 0), &pt(4, 0), &pt(2, 0), &pt(2, 3)));
        assert!(touch(&pt(0, 0), &pt(4, 0), &pt(2, -3), &pt(2, 0)));
        // a crossing, inside both: the objective crossings, not a touch; parallel lines apart
        assert!(!touch(&pt(0, 0), &pt(4, 0), &pt(2, -3), &pt(2, 3)));
        assert!(!touch(&pt(0, 0), &pt(4, 0), &pt(0, 1), &pt(4, 1)));
        // oblique: an end on it, and a crossing
        assert!(touch(&pt(0, 0), &pt(4, 4), &pt(2, 2), &pt(5, 0)));
        assert!(!touch(&pt(0, 0), &pt(4, 4), &pt(0, 4), &pt(4, 0)));
        assert!(on_segment(&pt(2, 2), &pt(0, 0), &pt(4, 4)) && !on_segment(&pt(5, 5), &pt(0, 0), &pt(4, 4)));
    }

    /// The one level of a netlist given as text, as a solver reads it, and its structure.
    fn level(text: &str) -> (LevelProblem, Structure) {
        let files = rnl_runtime::source::MemoryProvider::new().with("t.sp", text);
        let mut ps = rnl_view::interpret("t.sp", None, &files, &rnl_runtime::Options::solver()).unwrap().problems;
        assert_eq!(ps.len(), 1);
        let p = ps.remove(0);
        let options = crate::SolverOptions::without_time_limit();
        let resolved = crate::resolve(&p, &options);
        let mut work = resolved.work;
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut work);
        (p, s)
    }

    /// Every object where the PLACEs and the coordinates of the file put it, turned R0; the others
    /// at the origin.
    fn as_written(p: &LevelProblem) -> Geometry {
        use rnl_runtime::problem::{Position, Slot};
        let mut g = Geometry::new(p.objects.len());
        for (k, o) in p.objects.iter().enumerate() {
            if let Position::Own { x: Slot::Fixed { value: x, .. }, y: Slot::Fixed { value: y, .. } } = &o.position {
                g.origin[k] = [crate::rational::from_f64(*x).unwrap(), crate::rational::from_f64(*y).unwrap()];
            }
        }
        g
    }

    /// The places where wires mislead, counted on a layout drawn by hand, each case apart from the
    /// others so that the count of each is known: the wire written of `a` runs over the pin of R1
    /// on `c`, which the ROUTE draws; the bus `Q<1:0>` meets the wire of `Q<1>`, one of its nets,
    /// which does not count, and the wire of `d`, which does; the wire written of `b`, a net that
    /// a ROUTE asks for, is of the ROUTE's net (`drawn`), so that it meets a wire routed for `b`
    /// without counting, and the wire of `e`, written first so that it has the index of the ROUTE
    /// of `b` among the wires written, with.
    #[test]
    fn wires_written_mislead_over_pins_of_other_nets_and_where_they_meet_one() {
        let (p, s) = level(
            "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n\
             R1 c z1 1k\nR2 c z2 1k\nR3 b z3 1k\nR4 b z4 1k\nR5 Q<1> z5 1k\nR6 Q<0> z6 1k\nR7 d z7 1k\nR8 e z8 1k\n\
             R9 a z9 1k\n* @RNL {\n\
             *   #R1 @PLACE(10, -3); #R2 @PLACE(100, -40); #R3 @PLACE(110, -40); #R4 @PLACE(120, -40);\n\
             *   #R5 @PLACE(130, -40); #R6 @PLACE(140, -40); #R7 @PLACE(150, -40); #R8 @PLACE(160, -40);\n\
             *   #R9 @PLACE(170, -40);\n\
             *   {&b &c} @ROUTE;\n\
             *   &e @WIRE {(55, 20) (55, 15)};\n\
             *   &a @WIRE {(5, 0) (15, 0)};\n\
             *   &Q<1:0> @WIRE {(30, 0) (40, 0)};\n\
             *   &Q<1> @WIRE {(40, 0) (40, 5)};\n\
             *   &d @WIRE {(35, 0) (35, -5)};\n\
             *   &b @WIRE {(50, 20) (60, 20)}\n* }\n.end\n",
        );
        let g = as_written(&p);
        let origins = s.origins(&p, &g);
        let nets: Vec<String> = p.wires.iter().map(|w| w.nets.join(",")).collect();
        assert_eq!(nets, ["e", "a", "Q<1>,Q<0>", "Q<1>", "d", "b"]);
        let route_b = s.connections.drawn.iter().position(|d| d.routed && d.net == "b").unwrap();
        assert_eq!(route_b, 0);
        let written = drawn(&p, &s, &g, &Routing::default());
        assert_eq!(written.paths.len(), p.wires.len());
        // the wire written of b is of the ROUTE's net, the others their own
        assert_eq!(written.paths[5].0, route_b);
        assert_eq!(written.paths[0].0, s.connections.drawn.len() - p.wires.len());
        // the pin of R1 that the wire of a runs over, on c
        let r1 = p.objects.iter().position(|o| o.label == "#R1").unwrap();
        assert_eq!(s.pin_at(&origins, &g.orient, r1, 0), Some(pt(10, 0)));
        let routed_b = (route_b, vec![pt(60, 20), pt(60, 25)]);
        let count = |wires: &[usize], routed: bool| {
            let mut paths: Vec<(usize, Vec<[Q; 2]>)> = wires.iter().map(|&w| written.paths[w].clone()).collect();
            if routed {
                paths.push(routed_b.clone());
            }
            misleading(&p, &s, &origins, &g, &Routed { paths })
        };
        assert_eq!(count(&[1], false), 1, "the pin of R1 on the wire of a");
        assert_eq!(count(&[2, 3], false), 0, "a bus and a wire of one of its nets");
        assert_eq!(count(&[2, 3, 4], false), 1, "and a wire of another net");
        assert_eq!(count(&[5], true), 0, "a wire written and one routed of the ROUTE's net");
        assert_eq!(count(&[5, 0], true), 1, "and a wire of another net");
        assert_eq!(count(&[0, 1, 2, 3, 4, 5], true), 3);
    }

    #[test]
    fn a_segment_runs_through_a_box_only_inside_it() {
        let b = [int(0), int(0), int(4), int(2)];
        // across, and along the middle; in from the edge to the inside
        assert!(through(&b, &pt(-1, 1), &pt(5, 1)));
        assert!(through(&b, &pt(2, -3), &pt(2, 1)));
        // along an edge, up to an edge, past a corner, outside
        assert!(!through(&b, &pt(-1, 2), &pt(5, 2)));
        assert!(!through(&b, &pt(-3, 1), &pt(0, 1)));
        assert!(!through(&b, &pt(-1, 1), &pt(1, -1)));
        assert!(!through(&b, &pt(5, 0), &pt(5, 2)));
        // oblique, through the inside and only touching a corner
        assert!(through(&b, &pt(-1, -1), &pt(5, 3)));
        assert!(!through(&b, &pt(3, 3), &pt(5, 1)));
    }

    /// Within a layer's tolerance the next layer decides, and an objective that is off breaks no
    /// tie: with no objective at all, the code alone orders candidates equal in the counts.
    #[test]
    fn tolerances_and_objectives_off() {
        let active = Active::new(
            &Objectives {
                layers: vec![Layer { items: vec![("area".to_string(), 1)], tolerance: 2.0 }, Layer::single("aspect")],
            },
            &Inputs::default(),
        );
        let entries = [entry(0, 0, &[1, 7], 0), entry(0, 0, &[2, 3], 1), entry(0, 0, &[4, 0], 2)];
        assert_eq!(order(&active, &entries), vec![1, 0, 2]);
        let none = Active::new(&Objectives::none(), &Inputs::default());
        let entries = [entry(0, 0, &[], 3), entry(0, 0, &[], 1), entry(0, 1, &[], 0), entry(0, 0, &[], 2)];
        assert_eq!(order(&none, &entries), vec![1, 3, 0, 2]);
    }
}
