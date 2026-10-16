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
//! - **What cannot be routed** is reported, never an error: a part with no path to the rest, or a
//!   net whose budget of expansions ran out ([`crate::Budget::astar_expansions`]), is left as it
//!   is, and a viewer draws flight lines for it. A net without ROUTE is not routed at all.
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

use crate::rational::{int, to_f64, Q};
use crate::resolve::{point_value, Geometry};
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

/// A ROUTE whose net the router left in parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Left {
    /// The first ROUTE of the net, by its index in [`LevelProblem::routes`].
    pub route: usize,
    pub net: String,
    /// The parts it is left in, two or more: the pins and the wires written that no wire joins.
    pub parts: usize,
    pub failure: Failure,
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
    /// The nets left in parts.
    pub left: Vec<Left>,
    /// The cells the searches expanded.
    pub expansions: u64,
}

impl Routing {
    /// The joins left undone: for each net left in parts, its parts less one.
    pub(crate) fn joins_left(&self) -> usize {
        self.left.iter().map(|l| l.parts - 1).sum()
    }
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
}

/// No state before: the start of a path.
const ROOT: u32 = u32::MAX;
/// No net keeps the node.
const FREE: u32 = u32::MAX;
/// Pins of two nets leave through the node: neither keeps it.
const SHARED: u32 = u32::MAX - 1;

impl<'a> Grid<'a> {
    fn new(layout: &'a Layout) -> Result<Grid<'a>, Failure> {
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
        let lat = Lattice::new(&extent, CLEARANCE + MARGIN, [&extra[0], &extra[1]], [&near[0], &near[1]])?;
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
            lat,
        };
        let c = int(CLEARANCE);
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
            let (Some(na), Some(nb)) = (self.lat.at(a), self.lat.at(b)) else { continue };
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
pub(crate) fn route_layout(layout: &Layout, budget: u64) -> (Vec<NetOut>, u64) {
    let mut expansions = 0;
    let wanted = |n: &NetIn| n.anchors.len() >= 2;
    if !layout.nets.iter().any(wanted) {
        return (
            layout.nets.iter().map(|n| NetOut { parts: n.anchors.len().min(1), ..NetOut::default() }).collect(),
            0,
        );
    }
    let mut grid = match Grid::new(layout) {
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
    // the smallest nets first: by the half perimeter of their points, then their number
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
    let mut outs: Vec<NetOut> = vec![NetOut::default(); layout.nets.len()];
    for (_, _, i) in order {
        outs[i] = grid.route_net(i, budget, &mut expansions);
        // its wires are other wires now: the leads it kept and did not take are free
        for n in std::mem::take(&mut grid.kept_by[i]) {
            if grid.kept[n] == i as u32 {
                grid.kept[n] = FREE;
            }
        }
    }
    (outs, expansions)
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

/// The vertices a wire written is drawn through, exactly (13.1): its points, and the bends its mode
/// adds between them; an `"auto"` wire straight, as a viewer that does not route draws it. None when
/// one of its points has no place.
pub(crate) fn written_path(problem: &LevelProblem, g: &Geometry, wire: usize) -> Option<Vec<[Q; 2]>> {
    let w = problem.wires.get(wire)?;
    let mut points = Vec::new();
    for r in &w.anchors {
        points.push([point_value(problem, g, r, 0, 0)?, point_value(problem, g, r, 1, 0)?]);
    }
    let mut out: Vec<[Q; 2]> = points.first().cloned().into_iter().collect();
    let two = int(2);
    for pair in points.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let (dx, dy) = (&b[0] - &a[0], &b[1] - &a[1]);
        if !(dx.is_zero() || dy.is_zero()) {
            match w.mode.as_str() {
                "horizontal-vertical" => out.push([b[0].clone(), a[1].clone()]),
                "vertical-horizontal" => out.push([a[0].clone(), b[1].clone()]),
                "manhattan" if dx.abs() >= dy.abs() => {
                    let x = &a[0] + &dx / &two;
                    out.extend([[x.clone(), a[1].clone()], [x, b[1].clone()]]);
                }
                "manhattan" => {
                    let y = &a[1] + &dy / &two;
                    out.extend([[a[0].clone(), y.clone()], [b[0].clone(), y]]);
                }
                "manhattan45" if dx.abs() >= dy.abs() => {
                    let run = (dx.abs() - dy.abs()) / &two * dx.signum();
                    out.extend([[&a[0] + &run, a[1].clone()], [&b[0] - &run, b[1].clone()]]);
                }
                "manhattan45" => {
                    let run = (dy.abs() - dx.abs()) / &two * dy.signum();
                    out.extend([[a[0].clone(), &a[1] + &run], [b[0].clone(), &b[1] - &run]]);
                }
                _ => {}
            }
        }
        out.push(b.clone());
    }
    Some(out)
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
/// the first of them, as [`crate::Unrouted::route`] and `RoutedWire::route` refer to it; the others'
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
    if jobs.is_empty() {
        return Routing::default();
    }
    let mut layout = Layout::default();
    for k in s.components() {
        if let Some(b) = s.box_at(origins, &g.orient, k) {
            layout.boxes.push((k, b));
        }
    }
    for k in 0..problem.objects.len() {
        for pin in 0..s.pins(k).len() {
            layout.pins.extend(s.pin_at(origins, &g.orient, k, pin));
        }
    }
    for w in 0..problem.wires.len() {
        layout.written.extend(written_path(problem, g, w));
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
    let (outs, expansions) = route_layout(&layout, options.budget.astar_expansions);
    work.astar_expansions += expansions;
    let mut routing = Routing { expansions, ..Routing::default() };
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
            routing.wires.push(RoutedWire { route: job.route, points: wire_points });
            routing.paths.push(w.vertices);
            routing.drawn.push(drawn);
        }
        if let Some(failure) = out.failure {
            routing.left.push(Left { route: job.route, net: job.net.clone(), parts: out.parts, failure });
        }
    }
    routing
}

#[cfg(test)]
mod tests;
