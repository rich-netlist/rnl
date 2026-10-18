//! The wires of the ROUTEs (13.7): a deterministic rectilinear A* on a lattice of the level, around
//! the boxes of the placed symbols, with branch points and junctions (13.2) (plan 5.7).
//!
//! ROUTE asks for a net to be drawn as wires: every pin on it joined into one tree, the wires
//! already written on it part of the tree (13.7). The router makes the rest of the tree for a
//! placed candidate (`crate::coords`), net by net:
//!
//! - **The lattice.** The coordinates of every pin, of every point of a wire written and of every
//!   bend of one, so that every pin is a node; and the integers over the extent of the level and a
//!   margin around it that are within [`TRACKS`] of one of those or of an edge of a box, the
//!   tracks the wires run on near the symbols. Where two such coordinates are farther apart, the
//!   lattice goes from the last track of one window to the first of the next in one edge, so that
//!   a symbol placed far away makes the lattice no larger than one placed near. Lengths are exact:
//!   the coordinates are rationals, scaled to integers by the least common multiple of their
//!   denominators.
//! - **Obstacles.** The box of every placed symbol, turned with it (9.2.3, 11.2), widened by one
//!   unit on every side: no wire runs closer to a symbol than that. A point the net joins reaches
//!   the free lattice by a *lead*, a straight run from it in one of the four directions to the
//!   first node clear of every widened box, through no symbol's box and past no other pin. A lead
//!   that does not leave a pin the way its symbol says (9.2.4) costs a violated exit. The lead a
//!   pin's own way is kept for its net: no wire of another net runs through it, so that no wire
//!   walls a pin in; where the own ways of pins of two nets end at one node, neither keeps it,
//!   and the net routed first may take it.
//! - **Leads along the edge.** A lead other than the pin's own way that runs along the edge of the
//!   pin's own symbol (the box is not drawn, the glyph inside it is) is a fallback for a pin whose
//!   own way is taken or barred, such as by the wire of a supply stub written on it: it costs
//!   [`ALONG_COST`] more, so that a pin with its own way open takes that, and a path runs through
//!   it only straight, from or to its pin.
//! - **Other wires.** A wire may cross another, straight through the crossing, which makes no
//!   connection (13.1); it never runs along another or touches one at a bend or an end, which
//!   would look like a connection that is not there (Principles 2.2). The wires written, drawn by
//!   their mode (13.1, `"auto"` straight as a viewer that does not route draws it), and those
//!   routed for the nets before are the other wires. A wire written on the net being routed is
//!   one too: it is part of the tree by its points, which are where the net's new wires join it
//!   (it has no point on a segment, 13.1), and a new wire neither runs along it nor touches it in
//!   between.
//! - **A net.** Its points, pins sorted by x and then y, and the points of its wires written,
//!   which those wires join into parts. The two nearest parts are joined first, then each part
//!   left, the nearest first, to the nearest point of the tree; any point of a wire routed for the
//!   net may be where a new wire branches off. A part none of whose points has an open lead is
//!   walled in: it is reported without a search, which would only flood the lattice. A part that
//!   found no path is tried again once the tree has grown. The cost of a path is its length, 2 for
//!   each bend, 4 for each crossing of another wire, 2 for each end that violates the exit of its
//!   pin and [`ALONG_COST`] for each end that takes a lead along the edge.
//! - **What is written back** (13.7): each wire from a pin or a branch point to a pin or a branch
//!   point, its bends between, `RoutedWire` with `Anchor`, `Bend` and `Branch` points; a wire that
//!   meets the tree in the middle of a wire routed before splits that wire there, and the branch
//!   point, where three wires meet, gets a junction (13.2). A pin at which a wire routed ends and
//!   another too, routed or written, is a T with the pin's lead: the wires routed end at a branch
//!   point on the pin instead, with a junction, and a wire of length 0 joins it to the pin (13.1);
//!   so too a point where three wires end.
//! - **Later phases.** The hard-only `route_layout` seam keeps the preceding algorithm and
//!   costs for compatibility tests. Configured routing uses exact configured local costs and
//!   the requested clearance. Each net completes hard, soft and fallback phases in the existing
//!   half-perimeter order before its geometry becomes an obstacle for the next net. If hard
//!   routing leaves components, a soft search has its own per-net budget and exact, locally
//!   approximated configured costs. If that budget or the lattice is unavailable, deterministic
//!   orthogonal connections complete the remaining components. Only generated wires can gain
//!   interior branches; authored wires keep their original listed points and any authored cycles.
//!   Every ROUTE therefore gets connected geometry; later phases are reported as fallbacks.
//!   A net without ROUTE is not routed at all.
//! - **Several ROUTEs of one net** are one net to route; its wires take the style of the first of
//!   them, and the report notes the net when the others give another ([`style_notes`]).
//!
//! The nets are routed the smallest first, by the half perimeter of their points, so that short
//! connections take the direct paths; the wires come out in the order of the ROUTEs. The search
//! takes its nodes in a fixed order and breaks ties by the index of the state, so the same layout
//! gives the same wires on every platform.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use rnl_runtime::problem::{LevelProblem, ObjectKind, PointRef};
use rnl_runtime::solution::{Branch, RoutedWire, WirePoint};

use crate::options::RoutingWeights;
use crate::rational::{int, to_f64, Q};
use crate::resolve::{point_value, Geometry};
use crate::routing_geometry::{
    authored_wire, GeometryPointId, GeometrySource, GeometryVertex, GeometryWire, OutputWireId, RoutingGeometry,
};
use crate::structure::Structure;
use crate::{SolverOptions, Work};

/// How far a wire keeps from the box of a symbol, in units, but where it leaves a pin.
pub(crate) const CLEARANCE: i64 = 1;
/// The lattice reaches this far beyond the widened boxes, so that wires can go round the outside.
const MARGIN: i64 = 3;
/// The cost of a bend, in units of length.
pub(crate) const BEND_COST: i64 = 2;
/// The cost of crossing another wire.
pub(crate) const CROSSING_COST: i64 = 4;
/// The cost of an end that does not leave its pin the way the symbol says (9.2.4).
pub(crate) const EXIT_COST: i64 = 2;
/// The cost of a lead that runs along the edge of its own symbol, on top of the violated exit:
/// high, so that only a pin walled in its own way takes one (see the module).
pub(crate) const ALONG_COST: i64 = 8;
/// How far from a coordinate that matters (an edge of a box, a pin, a point of a wire written) the
/// lattice keeps every integer, the tracks wires run on near the symbols; between two such
/// coordinates farther apart it goes straight from the last track of one to the first of the next.
const TRACKS: i64 = 8;
/// The most nodes a lattice may have, and the most along one axis.
const MAX_NODES: usize = 1_000_000;
const MAX_SIDE: i64 = 20_000;
/// The largest common denominator of the lattice's coordinates the router scales by.
const MAX_UNIT: i64 = 1_000_000_000;

/// The four directions: +x, +y, −x, −y.
const STEP: [(i64, i64); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
/// No direction: the first state of a search.
const NONE: usize = 4;

fn opposite(d: usize) -> usize {
    (d + 2) % 4
}

fn horizontal(d: usize) -> bool {
    d % 2 == 0
}

fn dir_of(v: (i64, i64)) -> Option<usize> {
    STEP.iter().position(|s| *s == v)
}

// -------------------------------------------------------------------------------------- results

/// Why a net is left in parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    /// No path around the symbols and the other wires joins a part to the rest.
    NoPath,
    /// The budget of expansions for the net ran out ([`crate::Budget::astar_expansions`]).
    Budget,
    /// The level is too large, or its coordinates too fine, for the lattice of the router.
    Lattice,
}

impl Failure {
    /// Why, in a few words.
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Failure::NoPath => "no path around the symbols and the other wires joins the rest",
            Failure::Budget => "the budget of A* expansions for the net ran out",
            Failure::Lattice => "the level is too large, or its coordinates too fine, for the lattice of the router",
        }
    }
}

/// A ROUTE for which the hard router needed a later phase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoutingFallback {
    /// The first ROUTE of the net, by its index in [`LevelProblem::routes`].
    pub route: usize,
    pub net: String,
    pub phase: String,
    pub reason: String,
}

/// The wires routed for the ROUTEs of a placed candidate.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Routing {
    /// The wires, in the order of the ROUTEs, each net's in the order they were made.
    pub wires: Vec<RoutedWire>,
    pub branches: Vec<Branch>,
    /// The exact path of each wire of `wires`: its two ends and the bends between.
    pub paths: Vec<Vec<[Q; 2]>>,
    /// The net of each wire of `wires`, by its index in `crate::structure::Connections::drawn`.
    pub drawn: Vec<usize>,
    /// The phases used after hard routing, with their causes; every ROUTE is still connected.
    pub fallbacks: Vec<RoutingFallback>,
    pub geometry: RoutingGeometry,
    /// The cells the searches expanded.
    pub expansions: u64,
}

// ------------------------------------------------------------------------------------ the input

/// A point a net joins: a pin, or a point of a wire written on it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AnchorIn {
    pub at: [Q; 2],
    /// The direction a wire leaves it, for a pin with one (9.2.4), turned with its object.
    pub exit: Option<(i64, i64)>,
    /// The object whose pin it is.
    pub owner: Option<usize>,
}

/// A net to route.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NetIn {
    pub anchors: Vec<AnchorIn>,
    /// Pairs of anchors that a wire written joins already.
    pub joined: Vec<(usize, usize)>,
}

/// What the router sees of a placed candidate: plain geometry, exact.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Layout {
    /// The box of each symbol, by its object: x1, y1, x2, y2, the smaller first.
    pub boxes: Vec<(usize, [Q; 4])>,
    /// Where every pin of every symbol is.
    pub pins: Vec<[Q; 2]>,
    /// The wires written, each by the vertices it is drawn through.
    pub written: Vec<Vec<[Q; 2]>>,
    pub nets: Vec<NetIn>,
}

/// One end of a wire the router made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum End {
    /// An anchor of the net, by its index in [`NetIn::anchors`].
    Anchor(usize),
    /// A branch point of the net, by its index in [`NetOut::branches`].
    Branch(usize),
}

/// A wire the router made for a net.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WireOut {
    pub start: End,
    pub end: End,
    /// Its two ends and the bends between, exactly.
    pub vertices: Vec<[Q; 2]>,
}

/// What the router made of a net.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NetOut {
    pub wires: Vec<WireOut>,
    pub branches: Vec<[Q; 2]>,
    /// The parts the net is in once routed: 1 when it is one tree, 0 when it has no point.
    pub parts: usize,
    pub failure: Option<Failure>,
}

// ---------------------------------------------------------------------------------- the lattice

/// The nodes of the router: the products of the coordinates on each axis.
#[derive(Clone, Debug)]
struct Lattice {
    xs: Vec<Q>,
    ys: Vec<Q>,
    /// The coordinates scaled by `unit` to integers.
    sx: Vec<i64>,
    sy: Vec<i64>,
    /// The common denominator of the coordinates: one unit of length.
    unit: i64,
}

impl Lattice {
    /// The coordinates `extra` on each axis, and the integers over `extent` widened by `pad` that are
    /// within [`TRACKS`] of one of them or of one of `near`.
    fn new(extent: &[Q; 4], pad: i64, extra: [&BTreeSet<Q>; 2], near: [&BTreeSet<Q>; 2]) -> Result<Lattice, Failure> {
        let mut axes: [Vec<Q>; 2] = [Vec::new(), Vec::new()];
        for axis in 0..2 {
            let lo = extent[axis].floor().to_integer() - BigInt::from(pad);
            let hi = extent[axis + 2].ceil().to_integer() + BigInt::from(pad);
            // the windows of tracks round the coordinates that matter, merged
            let tracks = BigInt::from(TRACKS);
            let mut windows: Vec<(BigInt, BigInt)> = extra[axis]
                .iter()
                .chain(near[axis])
                .map(|v| {
                    (
                        (v.floor().to_integer() - &tracks).max(lo.clone()),
                        (v.ceil().to_integer() + &tracks).min(hi.clone()),
                    )
                })
                .filter(|(a, b)| a <= b)
                .collect();
            windows.sort();
            let mut merged: Vec<(BigInt, BigInt)> = Vec::new();
            for (a, b) in windows {
                match merged.last_mut() {
                    Some(last) if a <= &last.1 + 1 => last.1 = last.1.clone().max(b),
                    _ => merged.push((a, b)),
                }
            }
            let mut count = 0i64;
            for (a, b) in &merged {
                count += (b - a).to_i64().unwrap_or(MAX_SIDE) + 1;
                if count >= MAX_SIDE {
                    return Err(Failure::Lattice);
                }
            }
            let mut set: BTreeSet<Q> = BTreeSet::new();
            for (a, b) in &merged {
                let span = (b - a).to_i64().unwrap_or(0);
                set.extend((0..=span).map(|i| Q::from_integer(a + BigInt::from(i))));
            }
            set.extend(extra[axis].iter().cloned());
            axes[axis] = set.into_iter().collect();
        }
        if axes[0].len().checked_mul(axes[1].len()).is_none_or(|n| n > MAX_NODES) {
            return Err(Failure::Lattice);
        }
        let mut unit = BigInt::one();
        for v in axes.iter().flatten() {
            unit = unit.lcm(v.denom());
            if unit > BigInt::from(MAX_UNIT) {
                return Err(Failure::Lattice);
            }
        }
        let scale = |vs: &[Q]| -> Result<Vec<i64>, Failure> {
            vs.iter()
                .map(|v| {
                    let s = (v * Q::from_integer(unit.clone())).to_integer();
                    s.to_i64().filter(|s| s.abs() < 1 << 50).ok_or(Failure::Lattice)
                })
                .collect()
        };
        let (sx, sy) = (scale(&axes[0])?, scale(&axes[1])?);
        let [xs, ys] = axes;
        Ok(Lattice { xs, ys, sx, sy, unit: unit.to_i64().ok_or(Failure::Lattice)? })
    }

    fn nx(&self) -> usize {
        self.xs.len()
    }

    fn ny(&self) -> usize {
        self.ys.len()
    }

    fn len(&self) -> usize {
        self.nx() * self.ny()
    }

    fn node(&self, i: usize, j: usize) -> usize {
        i * self.ny() + j
    }

    fn ij(&self, n: usize) -> (usize, usize) {
        (n / self.ny(), n % self.ny())
    }

    fn index(vs: &[Q], v: &Q) -> Option<usize> {
        vs.binary_search(v).ok()
    }

    /// The node at a point, when the point is one.
    fn at(&self, p: &[Q; 2]) -> Option<usize> {
        Some(self.node(Lattice::index(&self.xs, &p[0])?, Lattice::index(&self.ys, &p[1])?))
    }

    fn point(&self, n: usize) -> [Q; 2] {
        let (i, j) = self.ij(n);
        [self.xs[i].clone(), self.ys[j].clone()]
    }

    fn scaled(&self, n: usize) -> (i64, i64) {
        let (i, j) = self.ij(n);
        (self.sx[i], self.sy[j])
    }

    /// The neighbour of a node in a direction.
    fn step(&self, n: usize, d: usize) -> Option<usize> {
        let (i, j) = self.ij(n);
        match d {
            0 if i + 1 < self.nx() => Some(self.node(i + 1, j)),
            1 if j + 1 < self.ny() => Some(self.node(i, j + 1)),
            2 if i > 0 => Some(self.node(i - 1, j)),
            3 if j > 0 => Some(self.node(i, j - 1)),
            _ => None,
        }
    }

    /// The length of the edge between neighbours, scaled.
    fn edge_len(&self, a: usize, b: usize) -> i64 {
        let (pa, pb) = (self.scaled(a), self.scaled(b));
        (pa.0 - pb.0).abs() + (pa.1 - pb.1).abs()
    }

    /// The rectilinear distance between two nodes, scaled.
    fn distance(&self, a: usize, b: usize) -> i64 {
        self.edge_len(a, b)
    }
}

// ------------------------------------------------------------------------------------- the grid

/// A wire passes straight through the node along x.
const H: u8 = 1;
/// … along y.
const V: u8 = 2;
/// A wire bends or ends at the node: no other wire may touch it.
const STOP: u8 = 4;
/// A wire written runs obliquely through the node.
const DIAG: u8 = 8;

/// An edge a wire runs along.
const TAKEN: u8 = 1;
/// An edge a wire written crosses obliquely.
const CROSSED: u8 = 2;

/// What a node is to the current search.
const SOURCE: u8 = 1;
const TARGET: u8 = 2;
const FORBID: u8 = 4;

/// A node of the tree of the net being routed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TreeNode {
    Anchor(usize),
    Branch(usize),
    /// Inside a wire made for the net, by its index.
    Wire(usize),
}

/// A wire made for the net, by its nodes.
#[derive(Clone, Debug)]
struct Made {
    start: End,
    end: End,
    nodes: Vec<usize>,
}

/// A lead of an anchor (see the module).
#[derive(Clone, Debug)]
struct Lead {
    /// Its direction from the anchor.
    dir: usize,
    /// Its nodes, from the one next to the anchor out.
    nodes: Vec<usize>,
    /// It runs along the edge of the anchor's own symbol, not the pin's own way: only a pin walled
    /// in takes it, at [`ALONG_COST`].
    along: bool,
}

/// The bit of the axis of a direction: 1 along x, 2 along y.
fn axis_bit(d: usize) -> u8 {
    if horizontal(d) {
        1
    } else {
        2
    }
}

/// The router of a layout: the lattice, what blocks it, and the scratch of the searches.
/// Hard-search costs share one positive denominator throughout a search. Clearing it once
/// preserves every exact comparison, including arbitrarily small nonzero weights, while avoiding
/// rational normalization for each edge and heap comparison. Length and legacy ties are scaled
/// by the lattice unit, also a common positive factor.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct HardCost {
    weighted: BigInt,
    length: BigInt,
    legacy: BigInt,
}
struct HardScale {
    length: BigInt,
    turns: BigInt,
    crossings: BigInt,
    bend: BigInt,
    crossing_legacy: BigInt,
    exit: BigInt,
    along: BigInt,
}
impl HardScale {
    fn new(weights: &RoutingWeights, unit: i64) -> Self {
        let length_denominator = weights.wire_length.denom() * BigInt::from(unit);
        let denominator = length_denominator.lcm(weights.wire_turns.denom()).lcm(weights.wire_crossings.denom());
        Self {
            length: weights.wire_length.numer() * (&denominator / length_denominator),
            turns: weights.wire_turns.numer() * (&denominator / weights.wire_turns.denom()),
            crossings: weights.wire_crossings.numer() * (&denominator / weights.wire_crossings.denom()),
            bend: BigInt::from(unit) * BEND_COST,
            crossing_legacy: BigInt::from(unit) * CROSSING_COST,
            exit: BigInt::from(unit) * EXIT_COST,
            along: BigInt::from(unit) * ALONG_COST,
        }
    }
}

struct Grid<'a> {
    layout: &'a Layout,
    lat: Lattice,
    /// Inside a widened box.
    blocked: Vec<bool>,
    /// A pin is there.
    pin: Vec<bool>,
    occ: Vec<u8>,
    /// The edge from each node along +x, and along +y.
    hedge: Vec<u8>,
    vedge: Vec<u8>,
    /// The symbol whose box the inside of each edge meets, along +x and along +y ([`FREE`] for
    /// none, [`SHARED`] for more than one): no wire runs through a symbol.
    hbox: Vec<u32>,
    vbox: Vec<u32>,
    /// The symbols of the net being routed that a pin of it is inside: a wire leaves such a pin
    /// through its own symbol.
    exempt: Vec<usize>,
    /// Nodes on a lead of the net being routed, by `net_stamp`.
    lead: Vec<u32>,
    net_stamp: u32,
    /// The nodes inside a widened box on a lead along the edge of a symbol of the net being routed,
    /// and on no other lead: a path runs through them only straight along the lead, by the axes
    /// given ([`axis_bit`]), from or to its anchor.
    corridors: BTreeMap<usize, u8>,
    /// For each anchor node of the net being routed, the directions ([`STEP`], as bits) in which
    /// it has a lead only along the edge of its symbol: a path that leaves or enters it that way
    /// costs [`ALONG_COST`].
    along_only: BTreeMap<usize, u8>,
    /// The net whose pin leaves through each node its own way ([`FREE`] for none, [`SHARED`] when
    /// pins of two nets would).
    kept: Vec<u32>,
    /// The nodes each net keeps, released once it is routed.
    kept_by: Vec<Vec<usize>>,
    /// The net being routed, by its index in [`Layout::nets`].
    net: u32,
    /// What each node is to the current search, by `search_stamp`.
    role: Vec<(u32, u8, u8)>,
    search_stamp: u32,
    /// The least cost of each state (node × direction it was entered by), by `search_stamp`.
    best: Vec<(u32, i64, u32)>,
    /// None keeps the historical integer-cost seam. Production uses exact configured costs.
    hard_weights: Option<RoutingWeights>,
    #[cfg(test)]
    rational_hard: bool,
}

/// No state before: the start of a path.
const ROOT: u32 = u32::MAX;
/// No net keeps the node.
const FREE: u32 = u32::MAX;
/// Pins of two nets leave through the node: neither keeps it.
const SHARED: u32 = u32::MAX - 1;

impl<'a> Grid<'a> {
    #[cfg(test)]
    fn new(layout: &'a Layout) -> Result<Grid<'a>, Failure> {
        Self::with_clearance(layout, &int(CLEARANCE))
    }

    fn with_clearance(layout: &'a Layout, clearance: &Q) -> Result<Grid<'a>, Failure> {
        let mut extent: Option<[Q; 4]> = None;
        let mut grow = |p: &[Q; 2]| {
            extent = Some(match extent.take() {
                None => [p[0].clone(), p[1].clone(), p[0].clone(), p[1].clone()],
                Some(e) => [
                    e[0].clone().min(p[0].clone()),
                    e[1].clone().min(p[1].clone()),
                    e[2].clone().max(p[0].clone()),
                    e[3].clone().max(p[1].clone()),
                ],
            })
        };
        let mut extra: [BTreeSet<Q>; 2] = [BTreeSet::new(), BTreeSet::new()];
        let mut near: [BTreeSet<Q>; 2] = [BTreeSet::new(), BTreeSet::new()];
        let add = |p: &[Q; 2], extra: &mut [BTreeSet<Q>; 2]| {
            extra[0].insert(p[0].clone());
            extra[1].insert(p[1].clone());
        };
        for (_, b) in &layout.boxes {
            grow(&[b[0].clone(), b[1].clone()]);
            grow(&[b[2].clone(), b[3].clone()]);
            add(&[b[0].clone(), b[1].clone()], &mut near);
            add(&[b[2].clone(), b[3].clone()], &mut near);
        }
        for p in layout.pins.iter().chain(layout.written.iter().flatten()) {
            grow(p);
            add(p, &mut extra);
        }
        for a in layout.nets.iter().flat_map(|n| &n.anchors) {
            grow(&a.at);
            add(&a.at, &mut extra);
        }
        let Some(extent) = extent else { return Err(Failure::Lattice) };
        let pad = clearance.ceil().to_integer().to_i64().and_then(|c| c.checked_add(MARGIN)).ok_or(Failure::Lattice)?;
        let lat = Lattice::new(&extent, pad, [&extra[0], &extra[1]], [&near[0], &near[1]])?;
        let n = lat.len();
        let mut g = Grid {
            layout,
            blocked: vec![false; n],
            pin: vec![false; n],
            occ: vec![0; n],
            hedge: vec![0; n],
            vedge: vec![0; n],
            hbox: vec![FREE; n],
            vbox: vec![FREE; n],
            exempt: Vec::new(),
            lead: vec![0; n],
            net_stamp: 0,
            corridors: BTreeMap::new(),
            along_only: BTreeMap::new(),
            kept: vec![FREE; n],
            kept_by: vec![Vec::new(); layout.nets.len()],
            net: FREE,
            role: vec![(0, 0, 0); n],
            search_stamp: 0,
            best: vec![(0, 0, 0); n * 5],
            hard_weights: None,
            #[cfg(test)]
            rational_hard: false,
            lat,
        };
        let c = clearance.clone();
        for (_, b) in &layout.boxes {
            let (lo_x, hi_x) = (&b[0] - &c, &b[2] + &c);
            let (lo_y, hi_y) = (&b[1] - &c, &b[3] + &c);
            let i0 = g.lat.xs.partition_point(|x| *x <= lo_x);
            let i1 = g.lat.xs.partition_point(|x| *x < hi_x);
            let j0 = g.lat.ys.partition_point(|y| *y <= lo_y);
            let j1 = g.lat.ys.partition_point(|y| *y < hi_y);
            for i in i0..i1 {
                for j in j0..j1 {
                    let k = g.lat.node(i, j);
                    g.blocked[k] = true;
                }
            }
        }
        // the edges through the inside of a box: along x at a height strictly inside it, between
        // two coordinates that overlap its width; and the same along y
        for (k, b) in &layout.boxes {
            let k = *k as u32;
            let put = |slot: &mut u32| *slot = if *slot == FREE || *slot == k { k } else { SHARED };
            let rows = g.lat.ys.partition_point(|y| *y <= b[1])..g.lat.ys.partition_point(|y| *y < b[3]);
            let cols = g.lat.xs.partition_point(|x| *x <= b[0])..g.lat.xs.partition_point(|x| *x < b[2]);
            // an edge along x from column i meets the width when x_i < x2 and x_{i+1} > x1
            let from_i = g.lat.xs.partition_point(|x| *x <= b[0]).saturating_sub(1);
            let to_i = g.lat.xs.partition_point(|x| *x < b[2]);
            for j in rows {
                for i in from_i..to_i.min(g.lat.nx() - 1) {
                    if g.lat.xs[i] < b[2] && g.lat.xs[i + 1] > b[0] {
                        let n = g.lat.node(i, j);
                        put(&mut g.hbox[n]);
                    }
                }
            }
            let from_j = g.lat.ys.partition_point(|y| *y <= b[1]).saturating_sub(1);
            let to_j = g.lat.ys.partition_point(|y| *y < b[3]);
            for i in cols {
                for j in from_j..to_j.min(g.lat.ny() - 1) {
                    if g.lat.ys[j] < b[3] && g.lat.ys[j + 1] > b[1] {
                        let n = g.lat.node(i, j);
                        put(&mut g.vbox[n]);
                    }
                }
            }
        }
        for p in &layout.pins {
            if let Some(k) = g.lat.at(p) {
                g.pin[k] = true;
            }
        }
        for w in &layout.written {
            g.mark(w);
        }
        // the lead of each pin its own way, kept for its net
        for (i, net) in layout.nets.iter().enumerate() {
            let anchors: BTreeSet<usize> = net.anchors.iter().filter_map(|a| g.lat.at(&a.at)).collect();
            for a in &net.anchors {
                let Some(own) = a.exit.and_then(dir_of) else { continue };
                for lead in g.leads(a, &anchors) {
                    if lead.dir != own {
                        continue;
                    }
                    for n in lead.nodes {
                        let k = &mut g.kept[n];
                        *k = if *k == FREE || *k == i as u32 { i as u32 } else { SHARED };
                        g.kept_by[i].push(n);
                    }
                }
            }
        }
        Ok(g)
    }

    /// The edge from a node in a direction: its flags, by reference.
    fn edge(&mut self, n: usize, d: usize) -> Option<&mut u8> {
        let m = self.lat.step(n, d)?;
        Some(match d {
            0 => &mut self.hedge[n],
            1 => &mut self.vedge[n],
            2 => &mut self.hedge[m],
            _ => &mut self.vedge[m],
        })
    }

    /// Whether the edge from a node in a direction runs through a symbol whose box no pin of the
    /// net being routed is inside.
    fn through_symbol(&self, n: usize, d: usize) -> bool {
        let k = match (d, self.lat.step(n, d)) {
            (0, Some(_)) => self.hbox[n],
            (1, Some(_)) => self.vbox[n],
            (2, Some(m)) => self.hbox[m],
            (3, Some(m)) => self.vbox[m],
            _ => FREE,
        };
        k != FREE && (k == SHARED || !self.exempt.contains(&(k as usize)))
    }

    fn edge_flags(&self, n: usize, d: usize) -> u8 {
        match (d, self.lat.step(n, d)) {
            (0, Some(_)) => self.hedge[n],
            (1, Some(_)) => self.vedge[n],
            (2, Some(m)) => self.hedge[m],
            (3, Some(m)) => self.vedge[m],
            _ => 0,
        }
    }

    /// A wire drawn through `vertices` taken as another wire: its edges taken, the nodes it passes
    /// straight marked as such, its vertices stops; an oblique run marks the nodes and edges it
    /// crosses.
    fn mark(&mut self, vertices: &[[Q; 2]]) {
        for p in vertices {
            if let Some(k) = self.lat.at(p) {
                self.occ[k] |= STOP;
            }
        }
        for w in vertices.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            if a == b {
                continue;
            }
            let (Some(na), Some(nb)) = (self.lat.at(a), self.lat.at(b)) else {
                if a[1] == b[1] || a[0] == b[0] {
                    self.mark_fractional_axis(a, b);
                } else {
                    self.mark_oblique(a, b);
                }
                continue;
            };
            if a[1] == b[1] || a[0] == b[0] {
                let d = if a[1] == b[1] {
                    if a[0] < b[0] {
                        0
                    } else {
                        2
                    }
                } else if a[1] < b[1] {
                    1
                } else {
                    3
                };
                let flag = if horizontal(d) { H } else { V };
                let mut n = na;
                while n != nb {
                    if let Some(e) = self.edge(n, d) {
                        *e |= TAKEN;
                    }
                    let Some(m) = self.lat.step(n, d) else { break };
                    if m != nb {
                        self.occ[m] |= flag;
                    }
                    n = m;
                }
            } else {
                self.mark_oblique(a, b);
            }
        }
    }

    /// A later-phase unit lead may introduce a fractional coordinate absent from the initial
    /// lattice. Mark its exact crossings and occupied stretches without rebuilding that lattice
    /// (which would change the subsequent hard search order). A crossing at an off-grid bend or
    /// end blocks the edge; a proper interior crossing remains passable at the configured cost.
    fn mark_fractional_axis(&mut self, a: &[Q; 2], b: &[Q; 2]) {
        let axis = usize::from(a[0] == b[0]);
        let fixed = 1 - axis;
        let values = if axis == 0 { &self.lat.xs } else { &self.lat.ys };
        let fixed_values = if fixed == 0 { &self.lat.xs } else { &self.lat.ys };
        let lo = a[axis].clone().min(b[axis].clone());
        let hi = a[axis].clone().max(b[axis].clone());
        let node = |along, across| if axis == 0 { self.lat.node(along, across) } else { self.lat.node(across, along) };
        if let Some(across) = Lattice::index(fixed_values, &a[fixed]) {
            for along in 0..values.len() {
                let index = node(along, across);
                if lo < values[along] && values[along] < hi {
                    self.occ[index] |= if axis == 0 { H } else { V };
                }
                if along + 1 < values.len() && values[along] < hi && values[along + 1] > lo {
                    if axis == 0 {
                        self.hedge[index] |= TAKEN;
                    } else {
                        self.vedge[index] |= TAKEN;
                    }
                }
            }
        } else {
            let across = fixed_values.partition_point(|value| *value < a[fixed]);
            if across == 0 || across == fixed_values.len() {
                return;
            }
            for (along, value) in values.iter().enumerate() {
                if *value < lo || *value > hi {
                    continue;
                }
                let index = node(along, across - 1);
                let flag = if *value == lo || *value == hi { TAKEN } else { CROSSED };
                if axis == 0 {
                    self.vedge[index] |= flag;
                } else {
                    self.hedge[index] |= flag;
                }
            }
        }
    }

    /// An oblique segment of a wire written: the nodes on it and the edges it crosses.
    fn mark_oblique(&mut self, a: &[Q; 2], b: &[Q; 2]) {
        let (x0, x1) = if a[0] < b[0] { (&a[0], &b[0]) } else { (&b[0], &a[0]) };
        let (y0, y1) = if a[1] < b[1] { (&a[1], &b[1]) } else { (&b[1], &a[1]) };
        let i0 = self.lat.xs.partition_point(|x| x < x0);
        let i1 = self.lat.xs.partition_point(|x| x <= x1);
        let j0 = self.lat.ys.partition_point(|y| y < y0);
        let j1 = self.lat.ys.partition_point(|y| y <= y1);
        let (dx, dy) = (&b[0] - &a[0], &b[1] - &a[1]);
        for i in i0..i1 {
            for j in j0..j1 {
                let (x, y) = (&self.lat.xs[i], &self.lat.ys[j]);
                let k = self.lat.node(i, j);
                if (x - &a[0]) * &dy == (y - &a[1]) * &dx {
                    self.occ[k] |= DIAG;
                }
                // the edge along +x crosses the segment strictly between its nodes
                if i + 1 < i1 {
                    let xc = &a[0] + (y - &a[1]) * &dx / &dy;
                    if *x < xc && xc < self.lat.xs[i + 1] {
                        self.hedge[k] |= CROSSED;
                    }
                }
                if j + 1 < j1 {
                    let yc = &a[1] + (x - &a[0]) * &dy / &dx;
                    if *y < yc && yc < self.lat.ys[j + 1] {
                        self.vedge[k] |= CROSSED;
                    }
                }
            }
        }
    }

    fn has(&self, n: usize, what: u8) -> bool {
        let (stamp, flags, _) = self.role[n];
        stamp == self.search_stamp && flags & what != 0
    }

    /// The exit of a source or target node, as a direction.
    fn exit(&self, n: usize) -> Option<usize> {
        let (stamp, _, exit) = self.role[n];
        (stamp == self.search_stamp && usize::from(exit) < NONE).then_some(usize::from(exit))
    }

    fn set_role(&mut self, n: usize, what: u8, exit: Option<usize>) {
        let stamp = self.search_stamp;
        let (s, flags, e) = self.role[n];
        let (flags, e) = if s == stamp { (flags, e) } else { (0, NONE as u8) };
        let e = if usize::from(e) < NONE { e } else { exit.map_or(NONE as u8, |d| d as u8) };
        self.role[n] = (stamp, flags | what, e);
    }

    /// Whether a search may pass through a node that is not one of its targets.
    fn passable(&self, n: usize) -> bool {
        !self.has(n, FORBID) && self.clear(n, self.has(n, SOURCE))
    }

    /// Whether a path of the net being routed may pass through a node, whatever the search; a
    /// `source` may be a pin and inside a widened box.
    fn clear(&self, n: usize, source: bool) -> bool {
        if !source && (self.pin[n] || (self.blocked[n] && self.lead[n] != self.net_stamp && self.corridor(n).is_none()))
        {
            return false;
        }
        let kept = self.kept[n];
        if kept != FREE && kept != SHARED && kept != self.net {
            return false;
        }
        let o = self.occ[n];
        o & STOP == 0 && (o & (H | V)) != (H | V)
    }

    /// The axes a path may run through a node by, when it is a corridor ([`Grid::corridors`]).
    fn corridor(&self, n: usize) -> Option<u8> {
        if !self.blocked[n] || self.lead[n] == self.net_stamp {
            return None;
        }
        self.corridors.get(&n).copied()
    }

    /// Whether leaving or entering an anchor node in a direction (from it) takes a lead only along
    /// the edge of its symbol.
    fn along(&self, n: usize, d: usize) -> bool {
        self.along_only.get(&n).is_some_and(|m| m & (1 << d) != 0)
    }

    /// Whether a lead from the node `start` is open to the net being routed: no wire runs along
    /// it, and every node on it may be passed, out to the free lattice or to another anchor of the
    /// net in `anchors`.
    fn lead_open(&self, start: usize, lead: &Lead, anchors: &BTreeSet<usize>) -> bool {
        let mut n = start;
        for &m in &lead.nodes {
            if self.edge_flags(n, lead.dir) & TAKEN != 0 || self.through_symbol(n, lead.dir) {
                return false;
            }
            if anchors.contains(&m) {
                return true;
            }
            if !self.clear(m, false) {
                return false;
            }
            n = m;
        }
        true
    }

    /// The leads of an anchor (see the module): for each direction, the nodes from the anchor out
    /// to the first node clear of every widened box, or to another anchor of the net; none where
    /// the lattice ends first, a symbol's box is in the way or another pin. A lead other than the
    /// pin's own direction that runs along the edge of its own symbol is marked [`Lead::along`].
    fn leads(&self, a: &AnchorIn, anchors: &BTreeSet<usize>) -> Vec<Lead> {
        let Some(start) = self.lat.at(&a.at) else { return Vec::new() };
        let mut out = Vec::new();
        for d in 0..4 {
            let mut nodes = Vec::new();
            let mut n = start;
            let ok = loop {
                let Some(m) = self.lat.step(n, d) else { break false };
                nodes.push(m);
                if anchors.contains(&m) {
                    break true;
                }
                if self.pin[m] {
                    break false;
                }
                if !self.blocked[m] {
                    break true;
                }
                n = m;
            };
            let Some(&last) = nodes.last() else { continue };
            if !ok {
                continue;
            }
            let end = self.lat.point(last);
            let own_way = a.exit.and_then(dir_of) == Some(d);
            let mut along = false;
            let through = self.layout.boxes.iter().any(|(k, b)| {
                let own = a.owner == Some(*k);
                if own && inside(b, &a.at) {
                    return false;
                }
                along |= own && !own_way && runs_along(b, &a.at, &end);
                meets_inside(b, &a.at, &end)
            });
            if !through {
                out.push(Lead { dir: d, nodes, along });
            }
        }
        out
    }

    /// The least-cost path from a source to a target (see the module), its nodes from the source;
    /// `budget` is what the net may still expand.
    fn search(
        &mut self,
        sources: &[usize],
        targets: &[usize],
        budget: &mut u64,
        expansions: &mut u64,
    ) -> Result<Vec<usize>, Failure> {
        if let Some(weights) = self.hard_weights.clone() {
            #[cfg(test)]
            if self.rational_hard {
                return self.search_configured_rational(sources, targets, budget, expansions, &weights);
            }
            return self.search_configured(sources, targets, budget, expansions, &weights);
        }
        let unit = self.lat.unit;
        if targets.is_empty() || sources.is_empty() {
            return Err(Failure::NoPath);
        }
        // the box of the targets, for a heuristic that never overestimates
        let mut bx = [i64::MAX, i64::MAX, i64::MIN, i64::MIN];
        for &t in targets {
            let (x, y) = self.lat.scaled(t);
            bx = [bx[0].min(x), bx[1].min(y), bx[2].max(x), bx[3].max(y)];
        }
        let h = |lat: &Lattice, n: usize| -> i64 {
            let (x, y) = lat.scaled(n);
            (bx[0] - x).max(0).max(x - bx[2]) + (bx[1] - y).max(0).max(y - bx[3])
        };
        let stamp = self.search_stamp;
        let mut heap: BinaryHeap<Reverse<(i64, u32)>> = BinaryHeap::new();
        for &s in sources {
            let st = (s * 5 + NONE) as u32;
            self.best[st as usize] = (stamp, 0, ROOT);
            heap.push(Reverse((h(&self.lat, s), st)));
        }
        while let Some(Reverse((f, st))) = heap.pop() {
            let (_, g, _) = self.best[st as usize];
            let node = st as usize / 5;
            if f > g + h(&self.lat, node) {
                continue;
            }
            if *budget == 0 {
                return Err(Failure::Budget);
            }
            *budget -= 1;
            *expansions += 1;
            if self.has(node, TARGET) {
                let mut nodes = vec![node];
                let mut at = st;
                while self.best[at as usize].2 != ROOT {
                    at = self.best[at as usize].2;
                    nodes.push(at as usize / 5);
                }
                nodes.reverse();
                return Ok(nodes);
            }
            let din = st as usize % 5;
            let crossing = self.occ[node] & (H | V | DIAG) != 0;
            for d in 0..4 {
                if din != NONE && (d == opposite(din) || (crossing && d != din)) {
                    continue;
                }
                let Some(v) = self.lat.step(node, d) else { continue };
                let edge = self.edge_flags(node, d);
                if edge & TAKEN != 0 || self.through_symbol(node, d) {
                    continue;
                }
                // through a corridor only straight along it
                let bit = axis_bit(d);
                if self.corridor(node).is_some_and(|a| a & bit == 0) || self.corridor(v).is_some_and(|a| a & bit == 0) {
                    continue;
                }
                let mut cost = g + self.lat.edge_len(node, v);
                if self.has(v, TARGET) {
                    if self.exit(v).is_some_and(|e| d != opposite(e)) {
                        cost += EXIT_COST * unit;
                    }
                    if self.along(v, opposite(d)) {
                        cost += ALONG_COST * unit;
                    }
                } else {
                    if !self.passable(v) {
                        continue;
                    }
                    let o = self.occ[v];
                    let across = (o & H != 0 && !horizontal(d)) || (o & V != 0 && horizontal(d)) || o & DIAG != 0;
                    if across {
                        cost += CROSSING_COST * unit;
                    }
                }
                if edge & CROSSED != 0 {
                    cost += CROSSING_COST * unit;
                }
                if din != NONE && d != din {
                    cost += BEND_COST * unit;
                }
                if din == NONE && self.exit(node).is_some_and(|e| d != e) {
                    cost += EXIT_COST * unit;
                }
                if din == NONE && self.along(node, d) {
                    cost += ALONG_COST * unit;
                }
                let sv = v * 5 + d;
                let (s, old, _) = self.best[sv];
                if s != stamp || cost < old {
                    self.best[sv] = (stamp, cost, st);
                    heap.push(Reverse((cost + h(&self.lat, v), sv as u32)));
                }
            }
        }
        Err(Failure::NoPath)
    }

    /// The same hard admissibility and deterministic state order as `search`, with the finite
    /// configured local weights. Disabled objectives contribute zero to the primary weighted
    /// value; raw length and the established mechanism costs only resolve ties after it.
    fn search_configured(
        &self,
        sources: &[usize],
        targets: &[usize],
        budget: &mut u64,
        expansions: &mut u64,
        weights: &RoutingWeights,
    ) -> Result<Vec<usize>, Failure> {
        if targets.is_empty() || sources.is_empty() {
            return Err(Failure::NoPath);
        }
        let mut bounds = [i64::MAX, i64::MAX, i64::MIN, i64::MIN];
        for &target in targets {
            let (x, y) = self.lat.scaled(target);
            bounds = [bounds[0].min(x), bounds[1].min(y), bounds[2].max(x), bounds[3].max(y)];
        }
        let scale = HardScale::new(weights, self.lat.unit);
        let heuristic = |node| {
            let (x, y) = self.lat.scaled(node);
            let length =
                BigInt::from((bounds[0] - x).max(0).max(x - bounds[2]) + (bounds[1] - y).max(0).max(y - bounds[3]));
            HardCost { weighted: &length * &scale.length, length: length.clone(), legacy: length }
        };
        let estimate = |cost: &HardCost, node| {
            let h = heuristic(node);
            HardCost {
                weighted: &cost.weighted + h.weighted,
                length: &cost.length + h.length,
                legacy: &cost.legacy + h.legacy,
            }
        };
        let mut best: BTreeMap<usize, (HardCost, Option<usize>)> = BTreeMap::new();
        let mut heap = BinaryHeap::new();
        for &source in sources {
            let state = source * 5 + NONE;
            best.insert(state, (HardCost::default(), None));
            heap.push(Reverse((heuristic(source), state)));
        }
        while let Some(Reverse((priority, state))) = heap.pop() {
            let current = best[&state].0.clone();
            let (node, entered) = (state / 5, state % 5);
            if priority > estimate(&current, node) {
                continue;
            }
            if *budget == 0 {
                return Err(Failure::Budget);
            }
            *budget -= 1;
            *expansions += 1;
            if self.has(node, TARGET) {
                let mut path = Vec::new();
                let mut previous = Some(state);
                while let Some(state) = previous {
                    path.push(state / 5);
                    previous = best[&state].1;
                }
                path.reverse();
                return Ok(path);
            }
            let crossing = self.occ[node] & (H | V | DIAG) != 0;
            for direction in 0..4 {
                if entered != NONE && (direction == opposite(entered) || (crossing && direction != entered)) {
                    continue;
                }
                let Some(next_node) = self.lat.step(node, direction) else { continue };
                let edge = self.edge_flags(node, direction);
                if edge & TAKEN != 0 || self.through_symbol(node, direction) {
                    continue;
                }
                let axis = axis_bit(direction);
                if self.corridor(node).is_some_and(|axes| axes & axis == 0)
                    || self.corridor(next_node).is_some_and(|axes| axes & axis == 0)
                {
                    continue;
                }
                let mut next = current.clone();
                let length = BigInt::from(self.lat.edge_len(node, next_node));
                next.weighted += &length * &scale.length;
                next.length += &length;
                next.legacy += length;
                let mut crossings = usize::from(edge & CROSSED != 0);
                if self.has(next_node, TARGET) {
                    if self.exit(next_node).is_some_and(|exit| direction != opposite(exit)) {
                        next.legacy += &scale.exit;
                    }
                    if self.along(next_node, opposite(direction)) {
                        next.legacy += &scale.along;
                    }
                } else {
                    if !self.passable(next_node) {
                        continue;
                    }
                    let occupied = self.occ[next_node];
                    if (occupied & H != 0 && !horizontal(direction))
                        || (occupied & V != 0 && horizontal(direction))
                        || occupied & DIAG != 0
                    {
                        crossings += 1;
                    }
                }
                next.weighted += &scale.crossings * crossings;
                next.legacy += &scale.crossing_legacy * crossings;
                if entered != NONE && direction != entered {
                    next.weighted += &scale.turns;
                    next.legacy += &scale.bend;
                }
                if entered == NONE && self.exit(node).is_some_and(|exit| direction != exit) {
                    next.legacy += &scale.exit;
                }
                if entered == NONE && self.along(node, direction) {
                    next.legacy += &scale.along;
                }
                let next_state = next_node * 5 + direction;
                if best.get(&next_state).is_none_or(|old| next < old.0) {
                    heap.push(Reverse((estimate(&next, next_node), next_state)));
                    best.insert(next_state, (next, Some(state)));
                }
            }
        }
        Err(Failure::NoPath)
    }

    #[cfg(test)]
    fn search_configured_rational(
        &self,
        sources: &[usize],
        targets: &[usize],
        budget: &mut u64,
        expansions: &mut u64,
        weights: &RoutingWeights,
    ) -> Result<Vec<usize>, Failure> {
        if targets.is_empty() || sources.is_empty() {
            return Err(Failure::NoPath);
        }
        let mut bounds = [i64::MAX, i64::MAX, i64::MIN, i64::MIN];
        for &target in targets {
            let (x, y) = self.lat.scaled(target);
            bounds = [bounds[0].min(x), bounds[1].min(y), bounds[2].max(x), bounds[3].max(y)];
        }
        let heuristic = |node| {
            let (x, y) = self.lat.scaled(node);
            let length = int((bounds[0] - x).max(0).max(x - bounds[2]) + (bounds[1] - y).max(0).max(y - bounds[3]))
                / int(self.lat.unit);
            SoftCost { weighted: &length * &weights.wire_length, length: length.clone(), legacy: length, clearance: 0 }
        };
        let estimate = |cost: &SoftCost, node| {
            let h = heuristic(node);
            SoftCost {
                weighted: &cost.weighted + h.weighted,
                length: &cost.length + h.length,
                legacy: &cost.legacy + h.legacy,
                clearance: 0,
            }
        };
        let mut best: BTreeMap<usize, (SoftCost, Option<usize>)> = BTreeMap::new();
        let mut heap = BinaryHeap::new();
        for &source in sources {
            let state = source * 5 + NONE;
            best.insert(state, (SoftCost::default(), None));
            heap.push(Reverse((heuristic(source), state)));
        }
        while let Some(Reverse((priority, state))) = heap.pop() {
            let current = best[&state].0.clone();
            let (node, entered) = (state / 5, state % 5);
            if priority > estimate(&current, node) {
                continue;
            }
            if *budget == 0 {
                return Err(Failure::Budget);
            }
            *budget -= 1;
            *expansions += 1;
            if self.has(node, TARGET) {
                let mut path = Vec::new();
                let mut previous = Some(state);
                while let Some(state) = previous {
                    path.push(state / 5);
                    previous = best[&state].1;
                }
                path.reverse();
                return Ok(path);
            }
            let crossing = self.occ[node] & (H | V | DIAG) != 0;
            for direction in 0..4 {
                if entered != NONE && (direction == opposite(entered) || (crossing && direction != entered)) {
                    continue;
                }
                let Some(next_node) = self.lat.step(node, direction) else { continue };
                let edge = self.edge_flags(node, direction);
                if edge & TAKEN != 0 || self.through_symbol(node, direction) {
                    continue;
                }
                let axis = axis_bit(direction);
                if self.corridor(node).is_some_and(|axes| axes & axis == 0)
                    || self.corridor(next_node).is_some_and(|axes| axes & axis == 0)
                {
                    continue;
                }
                let mut next = current.clone();
                let length = int(self.lat.edge_len(node, next_node)) / int(self.lat.unit);
                next.weighted += &length * &weights.wire_length;
                next.length += &length;
                next.legacy += length;
                let mut crossings = usize::from(edge & CROSSED != 0);
                if self.has(next_node, TARGET) {
                    if self.exit(next_node).is_some_and(|exit| direction != opposite(exit)) {
                        next.legacy += int(EXIT_COST);
                    }
                    if self.along(next_node, opposite(direction)) {
                        next.legacy += int(ALONG_COST);
                    }
                } else {
                    if !self.passable(next_node) {
                        continue;
                    }
                    let occupied = self.occ[next_node];
                    if (occupied & H != 0 && !horizontal(direction))
                        || (occupied & V != 0 && horizontal(direction))
                        || occupied & DIAG != 0
                    {
                        crossings += 1;
                    }
                }
                next.weighted += &weights.wire_crossings * int(crossings as i64);
                next.legacy += int(CROSSING_COST) * int(crossings as i64);
                if entered != NONE && direction != entered {
                    next.weighted += &weights.wire_turns;
                    next.legacy += int(BEND_COST);
                }
                if entered == NONE && self.exit(node).is_some_and(|exit| direction != exit) {
                    next.legacy += int(EXIT_COST);
                }
                if entered == NONE && self.along(node, direction) {
                    next.legacy += int(ALONG_COST);
                }
                let next_state = next_node * 5 + direction;
                if best.get(&next_state).is_none_or(|old| next < old.0) {
                    heap.push(Reverse((estimate(&next, next_node), next_state)));
                    best.insert(next_state, (next, Some(state)));
                }
            }
        }
        Err(Failure::NoPath)
    }

    /// Routes one net, the `index`th of the layout (see the module); `budget` is the expansions it
    /// may make.
    fn route_net(&mut self, index: usize, mut budget: u64, expansions: &mut u64) -> NetOut {
        let net = &self.layout.nets[index];
        self.net = index as u32;
        self.exempt = net
            .anchors
            .iter()
            .filter_map(|a| {
                let k = a.owner?;
                self.layout.boxes.iter().any(|(o, b)| *o == k && inside(b, &a.at)).then_some(k)
            })
            .collect();
        let nodes: Vec<Option<usize>> = net.anchors.iter().map(|a| self.lat.at(&a.at)).collect();
        // the anchors sorted by x, then y (plan 5.7)
        let mut order: Vec<usize> = (0..net.anchors.len()).filter(|&a| nodes[a].is_some()).collect();
        order.sort_by(|&a, &b| {
            let (p, q) = (&net.anchors[a].at, &net.anchors[b].at);
            p[0].cmp(&q[0]).then_with(|| p[1].cmp(&q[1])).then(a.cmp(&b))
        });
        // the parts the wires written join
        let mut parent: Vec<usize> = (0..net.anchors.len()).collect();
        fn find(parent: &mut [usize], mut i: usize) -> usize {
            while parent[i] != i {
                parent[i] = parent[parent[i]];
                i = parent[i];
            }
            i
        }
        for &(a, b) in &net.joined {
            if a < parent.len() && b < parent.len() {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                parent[ra.max(rb)] = ra.min(rb);
            }
        }
        let mut parts: Vec<Vec<usize>> = Vec::new();
        let mut part_of_root: BTreeMap<usize, usize> = BTreeMap::new();
        for &a in &order {
            let r = find(&mut parent, a);
            let p = *part_of_root.entry(r).or_insert_with(|| {
                parts.push(Vec::new());
                parts.len() - 1
            });
            parts[p].push(a);
        }
        let mut out = NetOut { parts: parts.len(), ..NetOut::default() };
        if parts.len() < 2 {
            return out;
        }
        // the leads of every anchor of the net: those clear of its symbol open the widened boxes
        // they run through, those along its edge are corridors
        self.net_stamp += 1;
        self.corridors.clear();
        self.along_only.clear();
        let anchor_nodes: BTreeSet<usize> = order.iter().filter_map(|&a| nodes[a]).collect();
        let mut leads: Vec<(usize, usize, Lead)> = Vec::new();
        let mut clear_ways: BTreeMap<usize, u8> = BTreeMap::new();
        for &a in &order {
            let Some(n) = nodes[a] else { continue };
            for lead in self.leads(&net.anchors[a], &anchor_nodes) {
                if !lead.along {
                    for &m in &lead.nodes {
                        self.lead[m] = self.net_stamp;
                    }
                    *clear_ways.entry(n).or_default() |= 1 << lead.dir;
                }
                leads.push((a, n, lead));
            }
        }
        for (_, n, lead) in leads.iter().filter(|(_, _, l)| l.along) {
            for &m in lead.nodes.iter().filter(|m| !anchor_nodes.contains(m)) {
                *self.corridors.entry(m).or_default() |= axis_bit(lead.dir);
            }
            if clear_ways.get(n).is_none_or(|w| w & (1 << lead.dir) == 0) {
                *self.along_only.entry(*n).or_default() |= 1 << lead.dir;
            }
        }
        // a part none of whose anchors has a lead open, nor is where an anchor of another part is,
        // is walled in: no path leaves or reaches it, so no search is made for it
        let open: Vec<bool> = (0..net.anchors.len())
            .map(|a| leads.iter().any(|(b, n, lead)| *b == a && self.lead_open(*n, lead, &anchor_nodes)))
            .collect();
        let walled: Vec<bool> = (0..parts.len())
            .map(|p| {
                !parts[p].iter().any(|&a| {
                    open[a]
                        || parts
                            .iter()
                            .enumerate()
                            .any(|(q, other)| q != p && other.iter().any(|&b| nodes[b] == nodes[a]))
                })
            })
            .collect();
        let exit = |a: usize| net.anchors[a].exit.and_then(dir_of);
        let mut tree: BTreeMap<usize, TreeNode> = BTreeMap::new();
        let mut made: Vec<Made> = Vec::new();
        let mut branches: Vec<usize> = Vec::new();
        let mut joined = vec![false; parts.len()];
        let mut failed = walled.clone();
        let mut failure = None;

        // the two nearest parts first
        let dist = |lat: &Lattice, a: &[usize], b: &[usize]| -> i64 {
            a.iter()
                .flat_map(|x| b.iter().map(move |y| (x, y)))
                .map(|(&x, &y)| lat.distance(nodes[x].unwrap_or(0), nodes[y].unwrap_or(0)))
                .min()
                .unwrap_or(i64::MAX)
        };
        let mut pairs: Vec<(i64, usize, usize)> = Vec::new();
        for a in (0..parts.len()).filter(|&a| !walled[a]) {
            for b in (a + 1..parts.len()).filter(|&b| !walled[b]) {
                pairs.push((dist(&self.lat, &parts[a], &parts[b]), a, b));
            }
        }
        pairs.sort_unstable();
        for (_, a, b) in pairs {
            for &x in &parts[b] {
                tree.entry(nodes[x].unwrap_or(0)).or_insert(TreeNode::Anchor(x));
            }
            match self.join(&parts, a, &tree, &nodes, &exit, &mut budget, expansions) {
                Ok(path) => {
                    self.grow(&mut tree, &mut made, &mut branches, &parts[a], &nodes, path);
                    joined[a] = true;
                    joined[b] = true;
                    failure = None;
                    break;
                }
                Err(f) => {
                    tree.clear();
                    failure = Some(f);
                    if f == Failure::Budget {
                        break;
                    }
                }
            }
        }
        // then each part left, the nearest to the tree first
        while !tree.is_empty() {
            let tree_nodes: Vec<usize> = tree.keys().copied().collect();
            let next = (0..parts.len())
                .filter(|&p| !joined[p] && !failed[p])
                .map(|p| {
                    let d = parts[p]
                        .iter()
                        .flat_map(|&x| tree_nodes.iter().map(move |&t| (x, t)))
                        .map(|(x, t)| self.lat.distance(nodes[x].unwrap_or(0), t))
                        .min()
                        .unwrap_or(i64::MAX);
                    (d, p)
                })
                .min();
            let Some((_, p)) = next else { break };
            match self.join(&parts, p, &tree, &nodes, &exit, &mut budget, expansions) {
                Ok(path) => {
                    self.grow(&mut tree, &mut made, &mut branches, &parts[p], &nodes, path);
                    joined[p] = true;
                    // the tree has grown: a part that found no path may find one to it now, its
                    // way perhaps barred only by an anchor of the part just joined
                    failed.clone_from(&walled);
                }
                Err(Failure::NoPath) => {
                    failed[p] = true;
                    failure = Some(Failure::NoPath);
                }
                Err(f) => {
                    failure = Some(f);
                    break;
                }
            }
        }
        let tree_parts = joined.iter().filter(|j| **j).count();
        out.parts = if tree_parts > 0 { parts.len() - tree_parts + 1 } else { parts.len() };
        out.failure = (out.parts > 1).then(|| failure.unwrap_or(Failure::NoPath));
        self.junctions_at_anchors(net, &nodes, &mut made, &mut branches);
        // the wires made, as vertices; then other wires to the nets after
        for m in &made {
            let vertices = self.vertices(&m.nodes);
            self.mark(&vertices);
            out.wires.push(WireOut { start: m.start, end: m.end, vertices });
        }
        out.branches = branches.iter().map(|&n| self.lat.point(n)).collect();
        out
    }

    /// Where a wire made ends at a pin at which another wire ends too, made or written, the wires
    /// meet in a T with the lead of the pin: the wires made end at a branch point on the pin
    /// instead, joined to it by a wire of length 0, and the branch point gets a junction (13.1,
    /// 13.2). So too at a point where three wires end.
    fn junctions_at_anchors(
        &self,
        net: &NetIn,
        nodes: &[Option<usize>],
        made: &mut Vec<Made>,
        branches: &mut Vec<usize>,
    ) {
        let mut written = vec![0usize; net.anchors.len()];
        for &(a, b) in &net.joined {
            for x in [a, b] {
                if let Some(e) = written.get_mut(x) {
                    *e += 1;
                }
            }
        }
        let mut routed = vec![0usize; net.anchors.len()];
        for m in made.iter() {
            for e in [m.start, m.end] {
                if let End::Anchor(a) = e {
                    routed[a] += 1;
                }
            }
        }
        for (a, node) in nodes.iter().enumerate() {
            let Some(n) = *node else { continue };
            if routed[a] == 0 || usize::from(self.pin[n]) + written[a] + routed[a] < 3 {
                continue;
            }
            let b = branches.len();
            branches.push(n);
            for m in made.iter_mut() {
                if m.start == End::Anchor(a) {
                    m.start = End::Branch(b);
                }
                if m.end == End::Anchor(a) {
                    m.end = End::Branch(b);
                }
            }
            made.push(Made { start: End::Branch(b), end: End::Anchor(a), nodes: vec![n] });
        }
    }

    /// Searches from part `p` to the tree.
    #[allow(clippy::too_many_arguments)]
    fn join(
        &mut self,
        parts: &[Vec<usize>],
        p: usize,
        tree: &BTreeMap<usize, TreeNode>,
        nodes: &[Option<usize>],
        exit: &dyn Fn(usize) -> Option<usize>,
        budget: &mut u64,
        expansions: &mut u64,
    ) -> Result<Vec<usize>, Failure> {
        self.search_stamp += 1;
        let mut sources = Vec::new();
        for (q, part) in parts.iter().enumerate() {
            for &a in part {
                let Some(n) = nodes[a] else { continue };
                if q == p {
                    self.set_role(n, SOURCE, exit(a));
                    sources.push(n);
                } else if !tree.contains_key(&n) {
                    self.set_role(n, FORBID, None);
                }
            }
        }
        let mut targets = Vec::with_capacity(tree.len());
        for (&n, t) in tree {
            let e = match t {
                TreeNode::Anchor(a) => exit(*a),
                _ => None,
            };
            self.set_role(n, TARGET, e);
            targets.push(n);
        }
        self.search(&sources, &targets, budget, expansions)
    }

    /// Adds a path from part `part` to the tree: the wire, a branch point where it meets a wire
    /// made before in its middle, and the part's anchors.
    fn grow(
        &self,
        tree: &mut BTreeMap<usize, TreeNode>,
        made: &mut Vec<Made>,
        branches: &mut Vec<usize>,
        part: &[usize],
        nodes: &[Option<usize>],
        path: Vec<usize>,
    ) {
        let (s, t) = (path[0], path[path.len() - 1]);
        let source = part.iter().copied().find(|&a| nodes[a] == Some(s)).unwrap_or(part[0]);
        let end = match tree.get(&t).copied() {
            Some(TreeNode::Anchor(a)) => End::Anchor(a),
            Some(TreeNode::Branch(b)) => End::Branch(b),
            Some(TreeNode::Wire(w)) => {
                // split the wire at the node: a branch point where three wires meet
                let b = branches.len();
                branches.push(t);
                let at = made[w].nodes.iter().position(|&n| n == t).unwrap_or(0);
                let tail: Vec<usize> = made[w].nodes.split_off(at);
                made[w].nodes.push(t);
                let rest = Made { start: End::Branch(b), end: made[w].end, nodes: tail };
                made[w].end = End::Branch(b);
                let index = made.len();
                for &n in &rest.nodes[1..rest.nodes.len() - 1] {
                    tree.insert(n, TreeNode::Wire(index));
                }
                made.push(rest);
                tree.insert(t, TreeNode::Branch(b));
                End::Branch(b)
            }
            None => End::Anchor(source),
        };
        let index = made.len();
        if path.len() > 2 {
            for &n in &path[1..path.len() - 1] {
                tree.insert(n, TreeNode::Wire(index));
            }
        }
        made.push(Made { start: End::Anchor(source), end, nodes: path });
        for &a in part {
            if let Some(n) = nodes[a] {
                tree.entry(n).or_insert(TreeNode::Anchor(a));
            }
        }
    }

    /// The vertices of a path: its ends and the nodes where it turns.
    fn vertices(&self, path: &[usize]) -> Vec<[Q; 2]> {
        let mut out = vec![self.lat.point(path[0])];
        for w in path.windows(3) {
            let (a, b, c) = (self.lat.ij(w[0]), self.lat.ij(w[1]), self.lat.ij(w[2]));
            let straight = (a.0 == b.0 && b.0 == c.0) || (a.1 == b.1 && b.1 == c.1);
            if !straight {
                out.push(self.lat.point(w[1]));
            }
        }
        // a path of one node is a wire of length 0 between two points that touch (13.1)
        out.push(self.lat.point(path[path.len() - 1]));
        out
    }
}

/// Whether a point is inside a box, not on its edge.
fn inside(b: &[Q; 4], p: &[Q; 2]) -> bool {
    b[0] < p[0] && p[0] < b[2] && b[1] < p[1] && p[1] < b[3]
}

/// Whether the segment from `a` to `c`, which runs along an axis, has a point of a box, its edge
/// included, other than `a`.
fn runs_along(b: &[Q; 4], a: &[Q; 2], c: &[Q; 2]) -> bool {
    let axis = usize::from(a[0] == c[0]);
    let fixed = 1 - axis;
    if a[fixed] < b[fixed] || a[fixed] > b[fixed + 2] {
        return false;
    }
    let (lo, hi) = if a[axis] <= c[axis] { (&a[axis], &c[axis]) } else { (&c[axis], &a[axis]) };
    let from = lo.max(&b[axis]);
    let to = hi.min(&b[axis + 2]);
    from < to || (from == to && *from != a[axis])
}

/// Whether the segment from `a` to `c`, which runs along an axis, meets the inside of a box.
pub(crate) fn meets_inside(b: &[Q; 4], a: &[Q; 2], c: &[Q; 2]) -> bool {
    let (x0, x1) = if a[0] <= c[0] { (&a[0], &c[0]) } else { (&c[0], &a[0]) };
    let (y0, y1) = if a[1] <= c[1] { (&a[1], &c[1]) } else { (&c[1], &a[1]) };
    *x0 < b[2] && *x1 > b[0] && *y0 < b[3] && *y1 > b[1]
}

/// Routes the nets of a layout, each with `budget` expansions, the smallest first; what each
/// net became, in the order of `layout.nets`, and the expansions made.
#[cfg(test)]
pub(crate) fn route_layout(layout: &Layout, budget: u64) -> (Vec<NetOut>, u64) {
    hard_layout(layout, budget, &int(CLEARANCE))
}

#[cfg(test)]
fn hard_layout(layout: &Layout, budget: u64, clearance: &Q) -> (Vec<NetOut>, u64) {
    let mut expansions = 0;
    let wanted = |n: &NetIn| n.anchors.len() >= 2;
    if !layout.nets.iter().any(wanted) {
        return (
            layout.nets.iter().map(|n| NetOut { parts: n.anchors.len().min(1), ..NetOut::default() }).collect(),
            0,
        );
    }
    let mut grid =
        match if *clearance == int(CLEARANCE) { Grid::new(layout) } else { Grid::with_clearance(layout, clearance) } {
            Ok(g) => g,
            Err(f) => {
                let outs = layout
                    .nets
                    .iter()
                    .map(|n| {
                        let parts = parts_of(n);
                        NetOut { parts, failure: (parts > 1).then_some(f), ..NetOut::default() }
                    })
                    .collect();
                return (outs, 0);
            }
        };
    let mut outs: Vec<NetOut> = vec![NetOut::default(); layout.nets.len()];
    for i in net_order(layout) {
        outs[i] = grid.route_net(i, budget, &mut expansions);
        release_kept(&mut grid, i);
    }
    (outs, expansions)
}

/// The historical hard order, shared by the legacy seam and the production phase scheduler:
/// half perimeter, anchor count, then input identity. Result serialization remains in input order.
fn net_order(layout: &Layout) -> Vec<usize> {
    let mut order: Vec<(Q, usize, usize)> = layout
        .nets
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let xs = n.anchors.iter().map(|a| &a.at[0]);
            let ys = n.anchors.iter().map(|a| &a.at[1]);
            let span = |v: Vec<&Q>| match (v.iter().min(), v.iter().max()) {
                (Some(lo), Some(hi)) => *hi - *lo,
                _ => Q::zero(),
            };
            (span(xs.collect()) + span(ys.collect()), n.anchors.len(), i)
        })
        .collect();
    order.sort();
    order.into_iter().map(|(_, _, index)| index).collect()
}

fn release_kept(grid: &mut Grid<'_>, index: usize) {
    // Its wires are other wires now: the leads it kept and did not take are free.
    for node in std::mem::take(&mut grid.kept_by[index]) {
        if grid.kept[node] == index as u32 {
            grid.kept[node] = FREE;
        }
    }
}

/// The parts the wires written join a net's anchors into.
fn parts_of(n: &NetIn) -> usize {
    let mut parent: Vec<usize> = (0..n.anchors.len()).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            i = parent[i];
        }
        i
    }
    for &(a, b) in &n.joined {
        if a < parent.len() && b < parent.len() {
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            parent[ra.max(rb)] = ra.min(rb);
        }
    }
    (0..parent.len()).filter(|&i| find(&mut parent, i) == i).count()
}

// ------------------------------------------------------- configured soft routing and completion

/// Memberships parallel the input's physical pins and authored wires. Their order, never their
/// coordinates, supplies identity. The plain Layout remains unchanged for the legacy hard seam.
#[derive(Clone, Debug, Default)]
struct SoftContext {
    nets: Vec<String>,
    pins: Vec<BTreeSet<String>>,
    written: Vec<BTreeSet<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PhaseUse {
    net: usize,
    phase: &'static str,
    failure: Failure,
}

/// A union-find of explicit anchors and generated branches. Passing over a line, or coinciding
/// with a different pin, is deliberately absent from this graph.
struct Components {
    roots: Vec<usize>,
    anchors: usize,
}

impl Components {
    fn new(net: &NetIn, out: &NetOut) -> Self {
        let mut result =
            Self { roots: (0..net.anchors.len() + out.branches.len()).collect(), anchors: net.anchors.len() };
        for &(a, b) in &net.joined {
            if a < result.anchors && b < result.anchors {
                result.join(a, b);
            }
        }
        for wire in &out.wires {
            result.join(result.index(wire.start), result.index(wire.end));
        }
        result
    }

    fn index(&self, end: End) -> usize {
        match end {
            End::Anchor(anchor) => anchor,
            End::Branch(branch) => self.anchors + branch,
        }
    }

    fn root(&self, mut index: usize) -> usize {
        while self.roots[index] != index {
            index = self.roots[index];
        }
        index
    }

    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.root(a), self.root(b));
        self.roots[a.max(b)] = a.min(b);
    }

    fn count(&self) -> usize {
        (0..self.anchors).map(|anchor| self.root(anchor)).collect::<BTreeSet<_>>().len()
    }
}

/// An intentionally selected place to connect. Only generated wires have segment attachments;
/// an author's wire can be reached solely through its listed anchor identities.
#[derive(Clone, Debug)]
struct Attachment {
    at: [Q; 2],
    end: End,
    segment: Option<(usize, usize)>,
}

fn end_at(net: &NetIn, out: &NetOut, end: End) -> [Q; 2] {
    match end {
        End::Anchor(anchor) => net.anchors[anchor].at.clone(),
        End::Branch(branch) => out.branches[branch].clone(),
    }
}

fn attachments(net: &NetIn, out: &NetOut) -> Vec<Attachment> {
    let anchors = (0..net.anchors.len()).map(End::Anchor);
    let branches = (0..out.branches.len()).map(End::Branch);
    anchors.chain(branches).map(|end| Attachment { at: end_at(net, out, end), end, segment: None }).collect()
}

fn distance(a: &[Q; 2], b: &[Q; 2]) -> Q {
    (&a[0] - &b[0]).abs() + (&a[1] - &b[1]).abs()
}

fn on_segment(point: &[Q; 2], a: &[Q; 2], b: &[Q; 2]) -> bool {
    (&point[0] - &a[0]) * (&b[1] - &a[1]) == (&point[1] - &a[1]) * (&b[0] - &a[0])
        && (0..2).all(|axis| {
            a[axis].clone().min(b[axis].clone()) <= point[axis] && point[axis] <= a[axis].clone().max(b[axis].clone())
        })
}

fn project(point: &[Q; 2], a: &[Q; 2], b: &[Q; 2]) -> [Q; 2] {
    std::array::from_fn(|axis| {
        point[axis].clone().max(a[axis].clone().min(b[axis].clone())).min(a[axis].clone().max(b[axis].clone()))
    })
}

/// Start with the closest edge (Kruskal's deterministic first MST edge), then grow its component
/// by the closest available attachment (Prim). A partial hard tree is used as the initial tree.
fn nearest_parts(net: &NetIn, out: &NetOut, tree: Option<usize>) -> Option<(Attachment, Attachment)> {
    let components = Components::new(net, out);
    let tree = tree.map(|anchor| components.root(anchor));
    let mut points = attachments(net, out);
    let segments: Vec<_> = out
        .wires
        .iter()
        .enumerate()
        .flat_map(|(wire_index, wire)| {
            wire.vertices.windows(2).enumerate().map(move |(segment, pair)| (wire_index, segment, wire.start, pair))
        })
        .collect();
    for &(wire, segment, end, pair) in &segments {
        points.push(Attachment { at: pair[0].clone(), end, segment: Some((wire, segment)) });
    }
    let mut best: Option<(Q, Attachment, Attachment)> = None;
    let mut consider = |source: &Attachment, target: &Attachment| {
        let (a, b) = (components.root(components.index(source.end)), components.root(components.index(target.end)));
        if a == b || tree.is_some_and(|root| root != a && root != b) {
            return;
        }
        let distance = distance(&source.at, &target.at);
        if best.as_ref().is_none_or(|old| distance < old.0) {
            best = Some((distance, source.clone(), target.clone()));
        }
    };
    for (a, source) in points.iter().enumerate() {
        for target in &points[a + 1..] {
            consider(source, target);
        }
        // Project endpoints and bends onto the opposite component's generated segments. Keep
        // this quadratic in output size rather than comparing a quadratic projection set.
        for &(wire, segment, end, pair) in &segments {
            consider(
                source,
                &Attachment { at: project(&source.at, &pair[0], &pair[1]), end, segment: Some((wire, segment)) },
            );
        }
    }
    for (a, &(wire_a, segment_a, end_a, pair_a)) in segments.iter().enumerate() {
        for &(wire_b, segment_b, end_b, pair_b) in &segments[a + 1..] {
            if let Some(at) = orthogonal_intersection(&pair_a[0], &pair_a[1], &pair_b[0], &pair_b[1]) {
                consider(
                    &Attachment { at: at.clone(), end: end_a, segment: Some((wire_a, segment_a)) },
                    &Attachment { at, end: end_b, segment: Some((wire_b, segment_b)) },
                );
            }
        }
    }
    best.map(
        |(_, a, b)| {
            if tree.is_some_and(|root| components.root(components.index(a.end)) == root) {
                (b, a)
            } else {
                (a, b)
            }
        },
    )
}

fn orthogonal_intersection(a: &[Q; 2], b: &[Q; 2], c: &[Q; 2], d: &[Q; 2]) -> Option<[Q; 2]> {
    let point = if a[0] == b[0] && c[1] == d[1] {
        [a[0].clone(), c[1].clone()]
    } else if a[1] == b[1] && c[0] == d[0] {
        [c[0].clone(), a[1].clone()]
    } else {
        return None;
    };
    (on_segment(&point, a, b) && on_segment(&point, c, d)).then_some(point)
}

fn intersections(a: &[Q; 2], b: &[Q; 2], c: &[Q; 2], d: &[Q; 2]) -> Vec<[Q; 2]> {
    let mut points: Vec<_> =
        [a, b, c, d].into_iter().filter(|point| on_segment(point, a, b) && on_segment(point, c, d)).cloned().collect();
    points.extend(orthogonal_intersection(a, b, c, d));
    points.sort();
    points.dedup();
    points
}

/// Remove a closed excursion from a newly proposed shape, including a lead which doubles back
/// over itself. This never edits existing generated wires or the author's geometry.
fn erase_loops(mut path: Vec<[Q; 2]>) -> Vec<[Q; 2]> {
    'again: loop {
        for a in 0..path.len().saturating_sub(1) {
            for b in a + 1..path.len().saturating_sub(1) {
                let at = intersections(&path[a], &path[a + 1], &path[b], &path[b + 1])
                    .into_iter()
                    .filter(|point| b != a + 1 || *point != path[a + 1])
                    .min_by_key(|point| distance(&path[a], point));
                if let Some(at) = at {
                    let mut shorter = path[..=a].to_vec();
                    shorter.push(at);
                    shorter.extend_from_slice(&path[b + 1..]);
                    path = simplify(shorter);
                    continue 'again;
                }
            }
        }
        return path;
    }
}

/// Find the first or last intentional contact with a component. Generated segments may be
/// split; authored segments are represented only by their original explicit anchors.
fn component_hit(
    net: &NetIn,
    out: &NetOut,
    path: &[[Q; 2]],
    root: usize,
    last: bool,
) -> Option<(Q, usize, Attachment)> {
    let components = Components::new(net, out);
    let points = attachments(net, out);
    let mut best: Option<(Q, usize, Attachment)> = None;
    let mut walked = Q::zero();
    for (segment, pair) in path.windows(2).enumerate() {
        let mut consider = |attachment: Attachment| {
            if components.root(components.index(attachment.end)) != root {
                return;
            }
            let at = &walked + distance(&pair[0], &attachment.at);
            if best.as_ref().is_none_or(|old| if last { at > old.0 } else { at < old.0 }) {
                best = Some((at, segment, attachment));
            }
        };
        for point in &points {
            if on_segment(&point.at, &pair[0], &pair[1]) {
                consider(point.clone());
            }
        }
        for (wire_index, wire) in out.wires.iter().enumerate() {
            if components.root(components.index(wire.start)) != root {
                continue;
            }
            for (wire_segment, other) in wire.vertices.windows(2).enumerate() {
                for at in intersections(&pair[0], &pair[1], &other[0], &other[1]) {
                    consider(Attachment { at, end: wire.start, segment: Some((wire_index, wire_segment)) });
                }
            }
        }
        walked += distance(&pair[0], &pair[1]);
    }
    best
}

fn trim_connection(
    net: &NetIn,
    out: &NetOut,
    source: &Attachment,
    target: &Attachment,
    path: Vec<[Q; 2]>,
) -> (Attachment, Attachment, Vec<[Q; 2]>) {
    let components = Components::new(net, out);
    let path = erase_loops(path);
    let target_root = components.root(components.index(target.end));
    let (_, target_segment, target) = component_hit(net, out, &path, target_root, false)
        .unwrap_or_else(|| (Q::zero(), path.len() - 2, target.clone()));
    let mut trimmed = path[..=target_segment].to_vec();
    trimmed.push(target.at.clone());
    let trimmed = simplify(trimmed);
    let source_root = components.root(components.index(source.end));
    let (_, source_segment, source) =
        component_hit(net, out, &trimmed, source_root, true).unwrap_or_else(|| (Q::zero(), 0, source.clone()));
    let mut path = vec![source.at.clone()];
    path.extend_from_slice(&trimmed[source_segment + 1..]);
    (source, target, simplify(path))
}

fn materialize(out: &mut NetOut, attachment: &Attachment) -> End {
    let Some((index, segment)) = attachment.segment else { return attachment.end };
    let wire = &out.wires[index];
    if attachment.at == wire.vertices[0] {
        return wire.start;
    }
    if Some(&attachment.at) == wire.vertices.last() {
        return wire.end;
    }
    let branch = End::Branch(out.branches.len());
    out.branches.push(attachment.at.clone());
    let mut head = wire.vertices[..=segment].to_vec();
    if head.last() != Some(&attachment.at) {
        head.push(attachment.at.clone());
    }
    let mut tail = vec![attachment.at.clone()];
    tail.extend(wire.vertices[segment + 1..].iter().skip_while(|point| **point == attachment.at).cloned());
    let rest = WireOut { start: branch, end: wire.end, vertices: tail };
    out.wires[index].vertices = head;
    out.wires[index].end = branch;
    out.wires.push(rest);
    branch
}

fn add_connection(out: &mut NetOut, source: &Attachment, target: &Attachment, vertices: Vec<[Q; 2]>) {
    let start = materialize(out, source);
    let end = materialize(out, target);
    out.wires.push(WireOut { start, end, vertices });
}

/// Keep reversals (an exit lead can require one), remove only unneeded collinear vertices.
fn simplify(vertices: Vec<[Q; 2]>) -> Vec<[Q; 2]> {
    let mut result: Vec<[Q; 2]> = Vec::new();
    for point in vertices {
        if result.last() == Some(&point) {
            continue;
        }
        while result.len() >= 2 {
            let last = result.len() - 1;
            if !on_segment(&result[last], &result[last - 1], &point) {
                break;
            }
            result.pop();
        }
        result.push(point);
    }
    if result.len() == 1 {
        result.push(result[0].clone());
    }
    result
}

fn attachment_exit(net: &NetIn, attachment: &Attachment) -> Option<usize> {
    if attachment.segment.is_some() {
        return None;
    }
    match attachment.end {
        End::Anchor(anchor) => net.anchors[anchor].exit.and_then(dir_of),
        End::Branch(_) => None,
    }
}

/// An exact, bounded-size shape, independent of lattice availability. Both pin leads precede
/// the middle orthogonal connection. An aligned direct connection already provides those leads.
fn fallback_path(net: &NetIn, source: &Attachment, target: &Attachment) -> Vec<[Q; 2]> {
    let (a, b) = (&source.at, &target.at);
    if a == b {
        return vec![a.clone(), b.clone()];
    }
    let direction = if a[1] == b[1] {
        Some(if a[0] < b[0] { 0 } else { 2 })
    } else if a[0] == b[0] {
        Some(if a[1] < b[1] { 1 } else { 3 })
    } else {
        None
    };
    if direction.is_some_and(|direction| {
        attachment_exit(net, source).is_none_or(|exit| exit == direction)
            && attachment_exit(net, target).is_none_or(|exit| exit == opposite(direction))
    }) {
        return vec![a.clone(), b.clone()];
    }
    let lead = |attachment: &Attachment| match attachment_exit(net, attachment) {
        Some(direction) => [&attachment.at[0] + int(STEP[direction].0), &attachment.at[1] + int(STEP[direction].1)],
        None => attachment.at.clone(),
    };
    let (from, to) = (lead(source), lead(target));
    let lo = std::array::from_fn::<_, 2, _>(|axis| {
        a[axis].clone().min(b[axis].clone()).min(from[axis].clone()).min(to[axis].clone()) - int(1)
    });
    let hi = std::array::from_fn::<_, 2, _>(|axis| {
        a[axis].clone().max(b[axis].clone()).max(from[axis].clone()).max(to[axis].clone()) + int(1)
    });
    let mut middles = vec![vec![[to[0].clone(), from[1].clone()]], vec![[from[0].clone(), to[1].clone()]]];
    for y in [lo[1].clone(), hi[1].clone()] {
        middles.push(vec![[from[0].clone(), y.clone()], [to[0].clone(), y]]);
    }
    for x in [lo[0].clone(), hi[0].clone()] {
        middles.push(vec![[x.clone(), from[1].clone()], [x, to[1].clone()]]);
    }
    let mut candidates = Vec::new();
    for middle in middles {
        let mut path = vec![a.clone(), from.clone()];
        path.extend(middle);
        path.extend([to.clone(), b.clone()]);
        let path = simplify(path);
        if erase_loops(path.clone()) == path {
            candidates.push(path);
        }
    }
    candidates
        .into_iter()
        .min_by_key(|path| {
            path.windows(2).map(|pair| distance(&pair[0], &pair[1])).sum::<Q>()
                + int(BEND_COST) * int(path.len().saturating_sub(2) as i64)
        })
        .unwrap_or_else(|| erase_loops(vec![a.clone(), from.clone(), [to[0].clone(), from[1].clone()], to, b.clone()]))
}

/// Complete junctions after later phases without disturbing a hard stage that already succeeded.
/// Reuse a pre-existing zero length pin link if the hard stage had already created its junction.
fn complete_junctions(net: &NetIn, out: &mut NetOut) {
    for anchor in 0..net.anchors.len() {
        let end = End::Anchor(anchor);
        let attached: Vec<_> = out
            .wires
            .iter()
            .enumerate()
            .filter(|(_, wire)| wire.start == end || wire.end == end)
            .map(|(index, _)| index)
            .collect();
        if attached.is_empty() {
            continue;
        }
        let written = net.joined.iter().filter(|&&(a, b)| a == anchor || b == anchor).count();
        let degree = attached.len() + written + usize::from(net.anchors[anchor].owner.is_some());
        if degree < 3 {
            continue;
        }
        let existing = attached.iter().find_map(|&index| {
            let wire = &out.wires[index];
            if wire.vertices.iter().any(|point| *point != net.anchors[anchor].at) {
                return None;
            }
            match (wire.start, wire.end) {
                (End::Branch(branch), End::Anchor(_)) | (End::Anchor(_), End::Branch(branch)) => {
                    Some((index, End::Branch(branch)))
                }
                _ => None,
            }
        });
        let branch = existing.map(|(_, branch)| branch).unwrap_or_else(|| {
            let branch = End::Branch(out.branches.len());
            out.branches.push(net.anchors[anchor].at.clone());
            branch
        });
        for index in attached {
            if existing.is_some_and(|(link, _)| link == index) {
                continue;
            }
            let wire = &mut out.wires[index];
            if wire.start == end {
                wire.start = branch;
            }
            if wire.end == end {
                wire.end = branch;
            }
        }
        if existing.is_none() {
            out.wires.push(WireOut {
                start: branch,
                end,
                vertices: vec![net.anchors[anchor].at.clone(), net.anchors[anchor].at.clone()],
            });
        }
    }
}

/// A soft state's exact accumulated local approximation. Visited penalties are retained with
/// the best label of each (node, direction); merging labels is intentionally an approximation,
/// not a claim that the complete output objective is an additive shortest-path metric.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct SoftCost {
    weighted: Q,
    clearance: usize,
    length: Q,
    legacy: Q,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Penalty {
    Box(usize),
    Clearance(usize),
    Pin(usize),
    Wire(usize),
}

#[derive(Clone)]
struct SoftBest {
    cost: SoftCost,
    seen: BTreeSet<Penalty>,
    previous: Option<usize>,
    source: usize,
}

struct SoftWire<'a> {
    path: &'a [[Q; 2]],
    nets: &'a BTreeSet<String>,
}

struct SoftSearch<'a> {
    layout: &'a Layout,
    context: &'a SoftContext,
    net: usize,
    clearance: &'a Q,
    weights: &'a RoutingWeights,
    wires: Vec<SoftWire<'a>>,
}

/// A proper crossing counted on a half-open search edge (a,b], so an intersection on a lattice
/// node is not counted once on entering it and again on leaving it.
fn crosses(a: &[Q; 2], b: &[Q; 2], c: &[Q; 2], d: &[Q; 2]) -> bool {
    let cross = |u: &[Q; 2], v: &[Q; 2]| &u[0] * &v[1] - &u[1] * &v[0];
    let ab = [&b[0] - &a[0], &b[1] - &a[1]];
    let cd = [&d[0] - &c[0], &d[1] - &c[1]];
    let ac = [&c[0] - &a[0], &c[1] - &a[1]];
    let denominator = cross(&ab, &cd);
    if denominator.is_zero() {
        return false;
    }
    let t = cross(&ac, &cd) / &denominator;
    let u = cross(&ac, &ab) / denominator;
    Q::zero() < t && t <= Q::one() && Q::zero() < u && u < Q::one()
}

fn overlaps_line(a: &[Q; 2], b: &[Q; 2], c: &[Q; 2], d: &[Q; 2]) -> bool {
    if a == b || c == d {
        return false;
    }
    let axis = usize::from(a[0] == b[0]);
    let fixed = 1 - axis;
    a[fixed] == b[fixed]
        && a[fixed] == c[fixed]
        && c[fixed] == d[fixed]
        && a[axis].clone().min(b[axis].clone()).max(c[axis].clone().min(d[axis].clone()))
            < a[axis].clone().max(b[axis].clone()).min(c[axis].clone().max(d[axis].clone()))
}

impl SoftSearch<'_> {
    fn edge(&self, a: &[Q; 2], b: &[Q; 2], best: &SoftBest) -> SoftBest {
        let mut next = best.clone();
        let length = distance(a, b);
        next.cost.weighted += &length * &self.weights.wire_length;
        next.cost.length += &length;
        next.cost.legacy += length;
        let name = &self.context.nets[self.net];
        for (index, (owner, box_)) in self.layout.boxes.iter().enumerate() {
            if self.layout.nets[self.net].anchors.iter().any(|anchor| anchor.owner == Some(*owner)) {
                continue;
            }
            if !self.weights.wire_overlaps.is_zero()
                && meets_inside(box_, a, b)
                && next.seen.insert(Penalty::Box(index))
            {
                next.cost.weighted += &self.weights.wire_overlaps;
            }
            if !self.clearance.is_zero() {
                let widened = [
                    &box_[0] - self.clearance,
                    &box_[1] - self.clearance,
                    &box_[2] + self.clearance,
                    &box_[3] + self.clearance,
                ];
                if meets_inside(&widened, a, b) && next.seen.insert(Penalty::Clearance(index)) {
                    next.cost.clearance += 1;
                }
            }
        }
        if !self.weights.wire_shorts.is_zero() {
            for (index, (pin, nets)) in self.layout.pins.iter().zip(&self.context.pins).enumerate() {
                if !nets.is_empty()
                    && !nets.contains(name)
                    && on_segment(pin, a, b)
                    && next.seen.insert(Penalty::Pin(index))
                {
                    next.cost.weighted += &self.weights.wire_shorts;
                }
            }
        }
        for (index, wire) in self.wires.iter().enumerate() {
            if wire.nets.contains(name) || wire.nets.is_empty() {
                continue;
            }
            if !self.weights.wire_crossings.is_zero()
                && wire.path.windows(2).any(|pair| overlaps_line(a, b, &pair[0], &pair[1]))
                && next.seen.insert(Penalty::Wire(index))
            {
                next.cost.weighted += &self.weights.wire_crossings;
            }
            let count = wire.path.windows(2).filter(|pair| crosses(a, b, &pair[0], &pair[1])).count();
            next.cost.weighted += &self.weights.wire_crossings * int(count as i64);
            next.cost.legacy += int(CROSSING_COST) * int(count as i64);
        }
        next
    }
}

/// All lattice points of generated segments are potential branches. Authored segments are
/// absent: their only legal connections are the original explicit anchors in `attachments`.
fn lattice_attachments(
    lat: &Lattice,
    net: &NetIn,
    out: &NetOut,
    component: usize,
    components: &Components,
) -> BTreeMap<usize, Attachment> {
    let mut result = BTreeMap::new();
    for attachment in attachments(net, out) {
        if components.root(components.index(attachment.end)) == component {
            if let Some(node) = lat.at(&attachment.at) {
                result.entry(node).or_insert(attachment);
            }
        }
    }
    for (index, wire) in out.wires.iter().enumerate() {
        if components.root(components.index(wire.start)) != component {
            continue;
        }
        for (segment, pair) in wire.vertices.windows(2).enumerate() {
            let (Some(mut node), Some(end)) = (lat.at(&pair[0]), lat.at(&pair[1])) else { continue };
            let direction = if pair[0][1] == pair[1][1] {
                if pair[0][0] < pair[1][0] {
                    0
                } else {
                    2
                }
            } else if pair[0][1] < pair[1][1] {
                1
            } else {
                3
            };
            loop {
                result.entry(node).or_insert_with(|| Attachment {
                    at: lat.point(node),
                    end: wire.start,
                    segment: Some((index, segment)),
                });
                if node == end {
                    break;
                }
                let Some(next) = lat.step(node, direction) else { break };
                node = next;
            }
        }
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn soft_join(
    search: &SoftSearch<'_>,
    lat: &Lattice,
    net: &NetIn,
    out: &NetOut,
    source: &Attachment,
    target: &Attachment,
    budget: &mut u64,
    expansions: &mut u64,
) -> Result<(Attachment, Attachment, Vec<[Q; 2]>), Failure> {
    let components = Components::new(net, out);
    let sources = lattice_attachments(lat, net, out, components.root(components.index(source.end)), &components);
    let targets = lattice_attachments(lat, net, out, components.root(components.index(target.end)), &components);
    if sources.is_empty() || targets.is_empty() {
        return Err(Failure::Lattice);
    }
    let heuristic = |node| {
        let distance = targets.keys().map(|&target| lat.distance(node, target)).min().unwrap_or(0);
        let length = int(distance) / int(lat.unit);
        SoftCost {
            weighted: &length * &search.weights.wire_length,
            length: length.clone(),
            legacy: length,
            clearance: 0,
        }
    };
    let estimate = |cost: &SoftCost, node| {
        let h = heuristic(node);
        SoftCost {
            weighted: &cost.weighted + h.weighted,
            clearance: cost.clearance,
            length: &cost.length + h.length,
            legacy: &cost.legacy + h.legacy,
        }
    };
    let mut best: BTreeMap<usize, usize> = BTreeMap::new();
    let mut labels: Vec<(usize, SoftBest)> = Vec::new();
    let mut heap = BinaryHeap::new();
    for &node in sources.keys() {
        let state = node * 5 + NONE;
        let label = labels.len();
        labels
            .push((state, SoftBest { cost: SoftCost::default(), seen: BTreeSet::new(), previous: None, source: node }));
        best.insert(state, label);
        heap.push(Reverse((estimate(&SoftCost::default(), node), state, label)));
    }
    while let Some(Reverse((_, state, label))) = heap.pop() {
        if best[&state] != label {
            continue;
        }
        let current = labels[label].1.clone();
        let (node, entered) = (state / 5, state % 5);
        if let Some(target) = targets.get(&node) {
            let source = sources[&current.source].clone();
            let mut path = Vec::new();
            let mut previous = Some(label);
            while let Some(label) = previous {
                let (state, best) = &labels[label];
                path.push(lat.point(state / 5));
                previous = best.previous;
            }
            path.reverse();
            return Ok((source, target.clone(), simplify(path)));
        }
        if *budget == 0 {
            return Err(Failure::Budget);
        }
        *budget -= 1;
        *expansions += 1;
        for direction in 0..4 {
            let Some(next_node) = lat.step(node, direction) else { continue };
            let mut next = search.edge(&lat.point(node), &lat.point(next_node), &current);
            if entered != NONE && direction != entered {
                next.cost.weighted += &search.weights.wire_turns;
                next.cost.legacy += int(BEND_COST);
            }
            if entered == NONE && attachment_exit(net, &sources[&node]).is_some_and(|exit| exit != direction) {
                next.cost.legacy += int(EXIT_COST);
            }
            if targets
                .get(&next_node)
                .is_some_and(|target| attachment_exit(net, target).is_some_and(|exit| exit != opposite(direction)))
            {
                next.cost.legacy += int(EXIT_COST);
            }
            let next_state = next_node * 5 + direction;
            if best.get(&next_state).is_none_or(|&old| next.cost < labels[old].1.cost) {
                next.previous = Some(label);
                let next_label = labels.len();
                heap.push(Reverse((estimate(&next.cost, next_node), next_state, next_label)));
                labels.push((next_state, next));
                best.insert(next_state, next_label);
            }
        }
    }
    Err(Failure::NoPath)
}

/// The hard stage has its own unchanged seam. Only nets it leaves disconnected enter the soft
/// stage, with a fresh per-net budget; exhausting either budget never suppresses the final tree.
fn configured_layout(
    layout: &Layout,
    hard_budget: u64,
    soft_budget: u64,
    clearance: &Q,
    weights: &RoutingWeights,
    context: &SoftContext,
) -> (Vec<NetOut>, u64, Vec<PhaseUse>) {
    phased_layout(layout, hard_budget, soft_budget, clearance, weights, context, Some(weights))
}

/// The shared production scheduler and compatibility seam. `None` selects the exact historical
/// hard costs for A02; every production call passes the effective configured weights instead.
fn phased_layout(
    layout: &Layout,
    hard_budget: u64,
    soft_budget: u64,
    clearance: &Q,
    weights: &RoutingWeights,
    context: &SoftContext,
    hard_weights: Option<&RoutingWeights>,
) -> (Vec<NetOut>, u64, Vec<PhaseUse>) {
    let mut grid = Grid::with_clearance(layout, clearance).ok();
    if let Some(grid) = &mut grid {
        grid.hard_weights = hard_weights.cloned();
    }
    let mut outs = vec![NetOut::default(); layout.nets.len()];
    let mut expansions = 0;
    let generated_nets: Vec<BTreeSet<String>> =
        context.nets.iter().map(|net| [net.clone()].into_iter().collect()).collect();
    let mut phases = Vec::new();
    for index in net_order(layout) {
        outs[index] = match &mut grid {
            Some(grid) => grid.route_net(index, hard_budget, &mut expansions),
            None => {
                let parts = parts_of(&layout.nets[index]);
                NetOut { parts, failure: (parts > 1).then_some(Failure::Lattice), ..NetOut::default() }
            }
        };
        let Some(hard_failure) = outs[index].failure else {
            if let Some(grid) = &mut grid {
                release_kept(grid, index);
            }
            continue;
        };
        let lattice = grid.as_ref().map(|grid| &grid.lat);
        let net = &layout.nets[index];
        let mut remaining = soft_budget;
        let mut failure = hard_failure;
        let mut soft_noted = false;
        let mut fallback_noted = false;
        let components = Components::new(net, &outs[index]);
        let mut tree = outs[index].wires.first().map(|wire| components.root(components.index(wire.start)));
        while let Some((source, target)) = nearest_parts(net, &outs[index], tree) {
            let components = Components::new(net, &outs[index]);
            let next_tree = components.root(components.index(source.end));
            let result = match lattice.filter(|_| remaining > 0 && !fallback_noted) {
                Some(lattice) => {
                    if !soft_noted {
                        phases.push(PhaseUse { net: index, phase: "soft", failure: hard_failure });
                        soft_noted = true;
                    }
                    let mut wires: Vec<SoftWire<'_>> = layout
                        .written
                        .iter()
                        .zip(&context.written)
                        .map(|(path, nets)| SoftWire { path, nets })
                        .collect();
                    for (other, out) in outs.iter().enumerate() {
                        if other != index {
                            wires.extend(
                                out.wires
                                    .iter()
                                    .map(|wire| SoftWire { path: &wire.vertices, nets: &generated_nets[other] }),
                            );
                        }
                    }
                    let search = SoftSearch { layout, context, net: index, clearance, weights, wires };
                    soft_join(&search, lattice, net, &outs[index], &source, &target, &mut remaining, &mut expansions)
                }
                None => Err(if lattice.is_none() {
                    Failure::Lattice
                } else if remaining == 0 {
                    Failure::Budget
                } else {
                    failure
                }),
            };
            match result {
                Ok((source, target, path)) => add_connection(&mut outs[index], &source, &target, path),
                Err(reason) => {
                    failure = reason;
                    if !fallback_noted {
                        phases.push(PhaseUse { net: index, phase: "fallback", failure });
                        fallback_noted = true;
                    }
                    let path = fallback_path(net, &source, &target);
                    let (source, target, path) = trim_connection(net, &outs[index], &source, &target, path);
                    add_connection(&mut outs[index], &source, &target, path);
                }
            }
            tree = Some(next_tree);
        }
        complete_junctions(net, &mut outs[index]);
        outs[index].parts = Components::new(net, &outs[index]).count();
        outs[index].failure = None;
        if let Some(grid) = &mut grid {
            // Finish this net before routing the next one. Its soft/fallback output, including
            // new junction stops, is now part of the hard obstacles of every subsequent net.
            for wire in &outs[index].wires {
                grid.mark(&wire.vertices);
            }
            release_kept(grid, index);
        }
    }
    (outs, expansions, phases)
}

// ----------------------------------------------------------------------------- from the problem

/// A point of a net, as the problem refers to it.
#[derive(Clone, Debug, PartialEq)]
struct Point {
    reference: PointRef,
    /// A pin by object and index, or a point object.
    key: (usize, Option<usize>),
}

/// A net of the ROUTEs: the first ROUTE of it, its name, its points and the wires written on it.
struct Job {
    route: usize,
    net: String,
    points: Vec<Point>,
    wires: Vec<usize>,
}

/// The nets of the ROUTEs of a level, merged by name in the order of their first ROUTE, as
/// `crate::structure::Connections::drawn` has them.
fn jobs(problem: &LevelProblem) -> Vec<Job> {
    let mut jobs: Vec<Job> = Vec::new();
    for (i, r) in problem.routes.iter().enumerate() {
        let j = match jobs.iter().position(|j| j.net == r.net) {
            Some(j) => j,
            None => {
                jobs.push(Job { route: i, net: r.net.clone(), points: Vec::new(), wires: Vec::new() });
                jobs.len() - 1
            }
        };
        for p in &r.pins {
            if let PointRef::Pin { object, pin, .. } = p {
                let key = (*object, Some(*pin));
                if !jobs[j].points.iter().any(|q| q.key == key) {
                    jobs[j].points.push(Point { reference: p.clone(), key });
                }
            }
        }
        for &w in &r.wires {
            if !jobs[j].wires.contains(&w) {
                jobs[j].wires.push(w);
            }
        }
    }
    jobs
}

/// A note for each net whose ROUTEs give different styles (13.7, 13.1): its wires take the style of
/// the first of them, as `RoutedWire::route` refers to it; the others'
/// are not used, which the report says rather than leave it unsaid.
pub(crate) fn style_notes(problem: &LevelProblem) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut noted: Vec<&str> = Vec::new();
    let sorted = |i: usize| {
        let mut style: Vec<_> = problem.routes[i].style.iter().collect();
        style.sort_by(|a, b| a.0.cmp(&b.0));
        style
    };
    for (i, r) in problem.routes.iter().enumerate() {
        let Some(first) = problem.routes.iter().position(|x| x.net == r.net) else { continue };
        if first == i || noted.contains(&r.net.as_str()) || sorted(first) == sorted(i) {
            continue;
        }
        noted.push(&r.net);
        out.push(format!(
            "the ROUTEs of &{} give different styles: its wires take the style of the first of them",
            r.net
        ));
    }
    out
}

/// The point of a wire written that a routed wire may end at: a pin, or a point object (13.1).
fn wire_point(problem: &LevelProblem, r: &PointRef) -> Option<Point> {
    match r {
        PointRef::Pin { object, pin, .. } => Some(Point { reference: r.clone(), key: (*object, Some(*pin)) }),
        PointRef::Origin(k) if problem.objects.get(*k).is_some_and(|o| o.kind == ObjectKind::Point) => {
            Some(Point { reference: r.clone(), key: (*k, None) })
        }
        _ => None,
    }
}

/// Routes the ROUTEs of a placed candidate (see the module): `g` is its placement, `origins` the
/// origin of every object (`Structure::origins`). The expansions are added to `work`.
pub(crate) fn route(
    problem: &LevelProblem,
    s: &Structure,
    g: &Geometry,
    origins: &[[Q; 2]],
    options: &SolverOptions,
    work: &mut Work,
) -> Routing {
    let jobs = jobs(problem);
    let geometry = RoutingGeometry {
        wires: (0..problem.wires.len()).filter_map(|wire| authored_wire(problem, g, wire)).collect(),
    };
    if jobs.is_empty() {
        return Routing { geometry, ..Routing::default() };
    }
    let mut layout = Layout::default();
    let mut context = SoftContext { nets: jobs.iter().map(|job| job.net.clone()).collect(), ..SoftContext::default() };
    for k in s.components() {
        if let Some(b) = s.box_at(origins, &g.orient, k) {
            layout.boxes.push((k, b));
        }
    }
    // the pins of what is drawn: a symbol that the solution leaves free is not
    for k in (0..problem.objects.len()).filter(|&k| !s.objects[k].left_free) {
        for pin in 0..s.pins(k).len() {
            if let Some(at) = s.pin_at(origins, &g.orient, k, pin) {
                layout.pins.push(at);
                context.pins.push(
                    s.connections
                        .nets_of
                        .get(&(k, pin))
                        .into_iter()
                        .flatten()
                        .filter_map(|&net| s.connections.nets.get(net))
                        .map(|net| net.name.clone())
                        .collect(),
                );
            }
        }
    }
    for wire in &geometry.wires {
        layout.written.push(wire.vertices.iter().map(|vertex| vertex.at.clone()).collect());
        context.written.push(wire.nets.clone());
    }
    let mut points: Vec<Vec<Point>> = Vec::new();
    for job in &jobs {
        let mut net = NetIn::default();
        let mut pts: Vec<Point> = Vec::new();
        let mut add = |p: Point, net: &mut NetIn| -> Option<usize> {
            if let Some(i) = pts.iter().position(|q| q.key == p.key) {
                return Some(i);
            }
            let at = [point_value(problem, g, &p.reference, 0, 0)?, point_value(problem, g, &p.reference, 1, 0)?];
            let exit = match p.key {
                (k, Some(pin)) => s.exit(k, pin, g.orient[k]),
                _ => None,
            };
            let owner = p.key.1.map(|_| p.key.0);
            net.anchors.push(AnchorIn { at, exit, owner });
            pts.push(p);
            Some(pts.len() - 1)
        };
        for p in &job.points {
            add(p.clone(), &mut net);
        }
        for &w in &job.wires {
            let Some(wire) = problem.wires.get(w) else { continue };
            let ends: Vec<usize> =
                wire.anchors.iter().filter_map(|r| wire_point(problem, r)).filter_map(|p| add(p, &mut net)).collect();
            for pair in ends.windows(2) {
                net.joined.push((pair[0], pair[1]));
            }
        }
        layout.nets.push(net);
        points.push(pts);
    }
    let (outs, expansions, phases) = configured_layout(
        &layout,
        options.budget.astar_expansions,
        options.budget.soft_expansions,
        &options.route_clearance,
        &options.routing_weights(),
        &context,
    );
    work.astar_expansions += expansions;
    let mut routing = Routing { expansions, geometry, ..Routing::default() };
    for phase in phases {
        routing.fallbacks.push(RoutingFallback {
            route: jobs[phase.net].route,
            net: jobs[phase.net].net.clone(),
            phase: phase.phase.to_string(),
            reason: phase.failure.reason().to_string(),
        });
    }
    for (j, (job, out)) in jobs.iter().zip(outs).enumerate() {
        let drawn = s.connections.drawn.iter().position(|d| d.routed && d.net == job.net).unwrap_or(j);
        let offset = routing.branches.len();
        routing
            .branches
            .extend(out.branches.iter().map(|b| Branch { at: (to_f64(&b[0]), to_f64(&b[1])), junction: true }));
        for w in out.wires {
            let end = |e: End| match e {
                End::Anchor(a) => WirePoint::Anchor(points[j][a].reference.clone()),
                End::Branch(b) => WirePoint::Branch(offset + b),
            };
            let mut wire_points = vec![end(w.start)];
            let n = w.vertices.len();
            for v in &w.vertices[1..n - 1] {
                wire_points.push(WirePoint::Bend((to_f64(&v[0]), to_f64(&v[1]))));
            }
            wire_points.push(end(w.end));
            let id = OutputWireId::Generated(routing.wires.len());
            let vertices = wire_points
                .iter()
                .zip(&w.vertices)
                .enumerate()
                .map(|(index, (point, at))| match point {
                    WirePoint::Anchor(reference) => GeometryVertex::anchor(at.clone(), reference),
                    WirePoint::Branch(branch) => {
                        GeometryVertex { at: at.clone(), point: GeometryPointId::Branch(*branch), physical_pin: None }
                    }
                    WirePoint::Bend(_) => GeometryVertex {
                        at: at.clone(),
                        point: GeometryPointId::Bend { wire: id, index },
                        physical_pin: None,
                    },
                })
                .collect();
            routing.geometry.wires.push(GeometryWire {
                id,
                vertices,
                nets: [job.net.clone()].into_iter().collect(),
                source: GeometrySource::Generated,
            });
            routing.wires.push(RoutedWire { route: job.route, points: wire_points });
            routing.paths.push(w.vertices);
            routing.drawn.push(drawn);
        }
    }
    routing
}

#[cfg(test)]
mod tests;
