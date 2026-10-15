//! The table of objectives (plan 5.4; algorithm synthesis 5.3, 5.7): for each, its default
//! scope, the inputs it needs, whether it has a structural side, its estimate and its score.
//!
//! An objective is a preference of the solver's own, not a fact (14.8): the standard leaves the
//! kinds of objectives and their order to the solver, which keeps them in its options. They rank
//! after every fact (8.5), so they choose only among layouts that keep every fact kept.
//!
//! **Two faces.** An objective with a structural side is both a rule that makes candidates (the
//! sources of `crate::structure`) and a measure that ranks them; one without is only a measure.
//! An objective that is off has neither: it makes no candidate, ranks none and breaks no tie
//! (decided 2026-10-16).
//!
//! **Inert objectives.** An objective that is on but has nothing to work with, such as the
//! current direction in a level without a supply symbol, is treated as off, and the report says
//! so in one sentence ([`Active::inert`]). So are the three this version does not compute,
//! `terminal_wire_alignment`, `labels` and `bend_alignment`.
//!
//! **Values.** Every value is an exact rational ([`Q`]), so that candidates compare alike on
//! every platform. A layer's value is the weighted sum of its objectives; layers compare
//! lexicographically, the first first, two values of a layer within its tolerance counting as
//! equal ([`Active::compare`]).
//!
//! - [`estimate`] measures a sketch: the rough placement of a partial candidate in the search
//!   (`crate::search`), with the routing objectives measured on straight connections, since
//!   nothing is routed yet.
//! - [`score`] measures a candidate placed by the coordinate LP (`crate::coords`), and, once it
//!   is routed (stage S4), its wires. Without wires the routing objectives are measured as the
//!   estimate measures them.
//!
//! The counting objectives (distinct coordinates, kinds of gaps, crossings, turns) are not linear,
//! so they never enter the coordinate LP, which minimises linear stand-ins (plan 5.6); they rank
//! candidates once placed.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use num_traits::{Signed, Zero};
use rnl_runtime::geom::Orient;

use crate::model::PointId;
use crate::rational::{frac, from_f64, int, Q};
use crate::structure::{Role, Structure};
use crate::{Objectives, OBJECTIVES};

/// An objective, as [`OBJECTIVES`] names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Objective {
    CurrentDirection,
    GroundDirection,
    SignalFlow,
    ComponentAlignment,
    Distribution,
    Symmetry,
    Turns,
    Crossings,
    WireLength,
    Area,
    Aspect,
    TerminalWireAlignment,
    Labels,
    BendAlignment,
}

/// Where an objective is measured by default (algorithm synthesis 3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    /// Within each group (11.5), and once over the components in no group: a group is where
    /// symmetry and even spacing are judged. Whether the components in no group count as one
    /// group is open (algorithm synthesis 9.3); this version counts them so.
    Group,
    /// Over the whole level.
    Level,
}

/// What an objective needs from the level to have anything to measure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Need {
    /// A supply symbol: `$SUPPLY`, which `$SUPPLY_STUB` makes.
    Supply,
    /// A ground symbol: `$GND`, which `$GND_STUB` makes.
    Ground,
    /// A port whose kind, input or output, can be told from what it connects to.
    PortKinds,
    /// Something drawn as wires: a ROUTE (13.7) or a wire (13.1).
    Wires,
    /// Routing templates, which this version does not make.
    Templates,
}

/// One row of the table of objectives.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Entry {
    pub objective: Objective,
    pub scope: Scope,
    pub needs: &'static [Need],
    /// Whether it makes candidates as well as ranking them (`crate::structure::Source`).
    pub structural: bool,
    /// Whether this version computes it; one it does not is inert when configured.
    pub implemented: bool,
}

/// The table, in the order of [`OBJECTIVES`].
pub(crate) const TABLE: [Entry; 14] = [
    // MOS and BJT whose source (emitter) is not on the side of the ground; rows by current depth
    Entry {
        objective: Objective::CurrentDirection,
        scope: Scope::Level,
        needs: &[Need::Supply, Need::Ground],
        structural: true,
        implemented: true,
    },
    // ground symbols whose pin is not above their body; they hang below their pins
    Entry {
        objective: Objective::GroundDirection,
        scope: Scope::Level,
        needs: &[Need::Ground],
        structural: true,
        implemented: true,
    },
    // pairs of an input port right of an output port; inputs left, outputs right
    Entry {
        objective: Objective::SignalFlow,
        scope: Scope::Level,
        needs: &[Need::PortKinds],
        structural: true,
        implemented: true,
    },
    // distinct x plus distinct y of the components' origins; who shares a row or a column
    Entry {
        objective: Objective::ComponentAlignment,
        scope: Scope::Group,
        needs: &[],
        structural: true,
        implemented: true,
    },
    // kinds of gaps between adjacent component coordinates (the prototype's gap kinds); even rows
    Entry { objective: Objective::Distribution, scope: Scope::Group, needs: &[], structural: true, implemented: true },
    // mirror pairs not symmetric; mirrored orientations
    Entry { objective: Objective::Symmetry, scope: Scope::Group, needs: &[], structural: true, implemented: true },
    // bends of the wires; no structural side without routing templates
    Entry {
        objective: Objective::Turns,
        scope: Scope::Level,
        needs: &[Need::Wires],
        structural: false,
        implemented: true,
    },
    // crossings of wires of different nets; the order of the columns
    Entry {
        objective: Objective::Crossings,
        scope: Scope::Level,
        needs: &[Need::Wires],
        structural: true,
        implemented: true,
    },
    // the length of the wires; what is connected is put side by side
    Entry {
        objective: Objective::WireLength,
        scope: Scope::Level,
        needs: &[Need::Wires],
        structural: true,
        implemented: true,
    },
    // the area of the envelope; blocks side by side or stacked
    Entry { objective: Objective::Area, scope: Scope::Level, needs: &[], structural: true, implemented: true },
    // |width / height − 16/9| of the envelope
    Entry { objective: Objective::Aspect, scope: Scope::Level, needs: &[], structural: true, implemented: true },
    Entry {
        objective: Objective::TerminalWireAlignment,
        scope: Scope::Level,
        needs: &[Need::Wires],
        structural: false,
        implemented: false,
    },
    Entry {
        objective: Objective::Labels,
        scope: Scope::Level,
        needs: &[Need::Wires],
        structural: false,
        implemented: false,
    },
    Entry {
        objective: Objective::BendAlignment,
        scope: Scope::Group,
        needs: &[Need::Templates],
        structural: true,
        implemented: false,
    },
];

impl Objective {
    /// Its name in the options ([`OBJECTIVES`]).
    pub(crate) fn name(self) -> &'static str {
        OBJECTIVES[self as usize]
    }

    /// The objective of a name in the options.
    pub(crate) fn from_name(name: &str) -> Option<Objective> {
        let i = OBJECTIVES.iter().position(|n| *n == name)?;
        Some(TABLE[i].objective)
    }

    /// Its row of the table.
    pub(crate) fn entry(self) -> &'static Entry {
        &TABLE[self as usize]
    }
}

/// What the level offers the objectives (`crate::structure`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Inputs {
    pub supply: bool,
    pub ground: bool,
    pub port_kinds: bool,
    pub wires: bool,
}

impl Inputs {
    fn has(&self, need: Need) -> bool {
        match need {
            Need::Supply => self.supply,
            Need::Ground => self.ground,
            Need::PortKinds => self.port_kinds,
            Need::Wires => self.wires,
            Need::Templates => false,
        }
    }
}

/// One layer of the objectives in force.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ActiveLayer {
    pub items: Vec<(Objective, u32)>,
    /// Two values within it count as equal (`Layer::tolerance`).
    pub tolerance: Q,
}

/// The objectives in force in a level: the layers configured, without the objectives that are
/// inert there, and a sentence for each of those.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Active {
    pub layers: Vec<ActiveLayer>,
    pub inert: Vec<String>,
}

impl Active {
    /// The layers of `objectives` that the level can measure ([`Inputs`]). A layer left with no
    /// objective is dropped.
    pub(crate) fn new(objectives: &Objectives, inputs: &Inputs) -> Active {
        let mut layers = Vec::new();
        let mut inert = Vec::new();
        for layer in &objectives.layers {
            let mut items = Vec::new();
            for (name, weight) in &layer.items {
                let Some(o) = Objective::from_name(name) else { continue };
                match inert_reason(o, inputs) {
                    Some(reason) => inert.push(reason),
                    None => items.push((o, *weight)),
                }
            }
            if !items.is_empty() {
                let tolerance = from_f64(layer.tolerance).filter(|t| !t.is_negative()).unwrap_or_default();
                layers.push(ActiveLayer { items, tolerance });
            }
        }
        Active { layers, inert }
    }

    /// Whether an objective is in force.
    #[cfg(test)]
    pub(crate) fn is_on(&self, o: Objective) -> bool {
        self.layers.iter().any(|l| l.items.iter().any(|(x, _)| *x == o))
    }

    /// The objectives in force in the order of their layers.
    pub(crate) fn in_order(&self) -> Vec<Objective> {
        self.layers.iter().flat_map(|l| l.items.iter().map(|(o, _)| *o)).collect()
    }

    /// The value of each layer on a sketch ([`estimate`]).
    pub(crate) fn estimate(&self, view: &View) -> Vec<Q> {
        self.layers.iter().map(|l| l.items.iter().map(|(o, w)| estimate(*o, view) * int(i64::from(*w))).sum()).collect()
    }

    /// The value of each layer on a placed candidate ([`score`]).
    pub(crate) fn score(&self, view: &View, routed: Option<&Routed>) -> Vec<Q> {
        self.layers
            .iter()
            .map(|l| l.items.iter().map(|(o, w)| score(*o, view, routed) * int(i64::from(*w))).sum())
            .collect()
    }

    /// Compares two tuples of layer values: the first layer whose values differ by more than its
    /// tolerance decides, the smaller first. Equal when none does; the caller breaks the tie by
    /// the canonical code of the candidates, never by an objective that is off.
    pub(crate) fn compare(&self, a: &[Q], b: &[Q]) -> Ordering {
        for (i, layer) in self.layers.iter().enumerate() {
            let (Some(x), Some(y)) = (a.get(i), b.get(i)) else { break };
            if (x - y).abs() > layer.tolerance {
                return x.cmp(y);
            }
        }
        Ordering::Equal
    }
}

/// Why an objective is inert in a level, in one sentence; none when it is in force.
fn inert_reason(o: Objective, inputs: &Inputs) -> Option<String> {
    let entry = o.entry();
    let name = o.name();
    if !entry.implemented {
        return Some(format!("{name} is not computed by this version of the solver (solver v1): it is treated as off"));
    }
    let missing: Vec<&str> = entry
        .needs
        .iter()
        .filter(|n| !inputs.has(**n))
        .map(|n| match n {
            Need::Supply => "no supply symbol ($SUPPLY or $SUPPLY_STUB)",
            Need::Ground => "no ground symbol ($GND or $GND_STUB)",
            Need::PortKinds => "no port whose kind, input or output, can be told",
            Need::Wires => "nothing drawn as wires (no ROUTE and no wire)",
            Need::Templates => "no routing templates",
        })
        .collect();
    if missing.is_empty() {
        None
    } else {
        Some(format!(
            "{name} has nothing to work with in this level, which has {}: it is treated as off",
            missing.join(" and ")
        ))
    }
}

/// A placement to measure: an origin and an orientation for every object of the problem. The
/// origin of an object drawn at the point of its main object (12.1) is that point, and of a text
/// placed by its symbol its own position (13.3).
#[derive(Clone, Copy)]
pub(crate) struct View<'a> {
    pub s: &'a Structure,
    pub at: &'a [[Q; 2]],
    pub orient: &'a [Orient],
}

impl View<'_> {
    fn pin(&self, k: usize, pin: usize) -> Option<[Q; 2]> {
        self.s.pin_at(self.at, self.orient, k, pin)
    }

    fn bbox(&self, k: usize) -> Option<[Q; 4]> {
        self.s.box_at(self.at, self.orient, k)
    }

    fn point(&self, p: PointId) -> Option<[Q; 2]> {
        self.s.point_at(self.at, self.orient, p)
    }
}

/// The wires of a routed candidate (stage S4): each a polyline, by the index of the drawn net
/// of `crate::structure::Connections::drawn` it belongs to.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Routed {
    pub paths: Vec<(usize, Vec<[Q; 2]>)>,
}

/// The value of an objective on a sketch, for a partial candidate (plan 5.5): as [`score`]
/// measures a placed candidate that is not routed. The routing objectives measure the straight
/// connections of a minimum spanning tree of each net: their length; an L for each connection not
/// along an axis, crossing what it crosses; a turn for each such L and for each end whose pin
/// does not leave towards the other end.
pub(crate) fn estimate(o: Objective, view: &View) -> Q {
    score(o, view, None)
}

/// The value of an objective on a placed candidate (plan 5.8); smaller is better. With `routed`,
/// the routing objectives measure the wires; without, the connections as [`estimate`] does.
/// An objective this version does not compute is 0.
pub(crate) fn score(o: Objective, view: &View, routed: Option<&Routed>) -> Q {
    match o {
        Objective::CurrentDirection => current_direction(view),
        Objective::GroundDirection => ground_direction(view),
        Objective::SignalFlow => signal_flow(view),
        Objective::ComponentAlignment => scoped(view, o.entry().scope, alignment),
        Objective::Distribution => scoped(view, o.entry().scope, |pts| {
            gap_kinds(pts.iter().map(|p| &p[0])) + gap_kinds(pts.iter().map(|p| &p[1]))
        }),
        Objective::Symmetry => symmetry(view),
        Objective::Turns => match routed {
            Some(r) => r.paths.iter().map(|(_, path)| int(bends(path) as i64)).sum(),
            None => turns(view),
        },
        Objective::Crossings => match routed {
            Some(r) => {
                let segments: Vec<(usize, [Q; 2], [Q; 2])> = r
                    .paths
                    .iter()
                    .flat_map(|(net, path)| path.windows(2).map(move |w| (*net, w[0].clone(), w[1].clone())))
                    .collect();
                crossings(&segments)
            }
            None => {
                // each connection drawn as an L, horizontal from its first end
                let mut segments = Vec::new();
                for (net, a, b) in connections(view) {
                    let corner = [b[0].clone(), a[1].clone()];
                    segments.push((net, a, corner.clone()));
                    segments.push((net, corner, b));
                }
                crossings(&segments)
            }
        },
        Objective::WireLength => match routed {
            Some(r) => r.paths.iter().flat_map(|(_, path)| path.windows(2).map(|w| manhattan(&w[0], &w[1]))).sum(),
            None => connections(view).iter().map(|(_, a, b)| manhattan(a, b)).sum(),
        },
        Objective::Area => match envelope(view) {
            Some(e) => (&e[2] - &e[0]) * (&e[3] - &e[1]),
            None => Q::zero(),
        },
        Objective::Aspect => match envelope(view) {
            Some(e) => aspect(&(&e[2] - &e[0]), &(&e[3] - &e[1])),
            None => Q::zero(),
        },
        Objective::TerminalWireAlignment | Objective::Labels | Objective::BendAlignment => Q::zero(),
    }
}

trait IntoQ {
    fn into_q(self) -> Q;
}

impl IntoQ for usize {
    fn into_q(self) -> Q {
        int(self as i64)
    }
}

/// MOS and BJT whose source (emitter) is not on the side of the ground: an n-type with its source
/// not below its drain, a p-type with its source not above it. Two-terminal elements and other
/// devices are left out.
fn current_direction(view: &View) -> Q {
    let mut count = 0usize;
    for &k in &view.s.elements {
        let Some(n_type) = view.s.objects[k].device.n_type() else { continue };
        let (Some(d), Some(s)) = (view.s.drain(k), view.s.source(k)) else { continue };
        let (Some(d), Some(s)) = (view.pin(k, d), view.pin(k, s)) else { continue };
        let wrong = if n_type { s[1] >= d[1] } else { s[1] <= d[1] };
        count += usize::from(wrong);
    }
    count.into_q()
}

/// Ground symbols whose pin is not above the middle of their body.
fn ground_direction(view: &View) -> Q {
    let mut count = 0usize;
    for (k, info) in view.s.objects.iter().enumerate() {
        if info.role != Role::Ground {
            continue;
        }
        let (Some(pin), Some(b)) = (view.pin(k, 0), view.bbox(k)) else { continue };
        count += usize::from(pin[1].clone() * int(2) <= &b[1] + &b[3]);
    }
    count.into_q()
}

/// Pairs of an input port right of an output port.
fn signal_flow(view: &View) -> Q {
    use crate::structure::PortKind;
    let x = |k: usize| view.at[k][0].clone();
    let inputs: Vec<usize> = view.s.ports.iter().filter(|p| p.kind == PortKind::Input).map(|p| p.object).collect();
    let outputs: Vec<usize> = view.s.ports.iter().filter(|p| p.kind == PortKind::Output).map(|p| p.object).collect();
    let mut count = 0usize;
    for &i in &inputs {
        for &o in &outputs {
            count += usize::from(x(i) > x(o));
        }
    }
    count.into_q()
}

/// A measure of the components' origins: summed over the scopes of the groups (`Structure::scopes`)
/// for an objective measured within groups, over all of them for one measured over the level.
fn scoped(view: &View, scope: Scope, measure: impl Fn(&[[Q; 2]]) -> Q) -> Q {
    let of = |ks: &mut dyn Iterator<Item = usize>| {
        let points: Vec<[Q; 2]> = ks.map(|k| view.at[k].clone()).collect();
        measure(&points)
    };
    match scope {
        Scope::Group => view.s.scopes.iter().map(|ks| of(&mut ks.iter().copied())).sum(),
        Scope::Level => of(&mut view.s.components()),
    }
}

/// Distinct x plus distinct y.
fn alignment(points: &[[Q; 2]]) -> Q {
    let xs: BTreeSet<&Q> = points.iter().map(|p| &p[0]).collect();
    let ys: BTreeSet<&Q> = points.iter().map(|p| &p[1]).collect();
    (xs.len() + ys.len()).into_q()
}

/// The prototype's gap kinds of one projection (its `regularity.gap_kinds`): the distinct
/// coordinates sorted, the gaps between neighbours, and the number of kinds of gaps less one,
/// plus `1 − Σ (nₖ / n)²` over the kinds, which is below 1 and prefers one dominant spacing. No
/// gap, no cost.
pub(crate) fn gap_kinds<'a>(coords: impl Iterator<Item = &'a Q>) -> Q {
    let xs: Vec<&Q> = coords.collect::<BTreeSet<&Q>>().into_iter().collect();
    if xs.len() < 2 {
        return Q::zero();
    }
    let mut counts: std::collections::BTreeMap<Q, i64> = std::collections::BTreeMap::new();
    for w in xs.windows(2) {
        *counts.entry(w[1] - w[0]).or_default() += 1;
    }
    let n = (xs.len() - 1) as i64;
    let shares: Q = counts.values().map(|c| frac(c * c, n * n)).sum();
    int(counts.len() as i64 - 1) + int(1) - shares
}

/// Mirror pairs not symmetric: not on one row, orientations not mirrored (`b` is `a` then MY,
/// 14.9), or not equally far from their axis when they have one.
fn symmetry(view: &View) -> Q {
    let mut count = 0usize;
    for pair in &view.s.pairs {
        let (a, b) = (&view.at[pair.a], &view.at[pair.b]);
        let mut ok = a[1] == b[1] && view.orient[pair.b] == view.orient[pair.a].then(Orient::MY);
        if let Some(axis) = pair.axis {
            ok &= &a[0] + &b[0] == view.at[axis][0].clone() * int(2);
        }
        count += usize::from(!ok);
    }
    count.into_q()
}

fn manhattan(a: &[Q; 2], b: &[Q; 2]) -> Q {
    (&a[0] - &b[0]).abs() + (&a[1] - &b[1]).abs()
}

/// The connections of every net drawn as wires: the edges of a minimum spanning tree of its
/// points by rectilinear distance (Prim's, from its first point, ties to the earlier point), each
/// with the index of its net.
fn connections(view: &View) -> Vec<(usize, [Q; 2], [Q; 2])> {
    let mut out = Vec::new();
    for (i, drawn) in view.s.connections.drawn.iter().enumerate() {
        let points: Vec<[Q; 2]> = drawn.points.iter().filter_map(|p| view.point(*p)).collect();
        for (a, b) in spanning_tree(&points) {
            out.push((i, points[a].clone(), points[b].clone()));
        }
    }
    out
}

/// The edges of a minimum spanning tree of points by rectilinear distance (Prim's): each joins a
/// point not yet in the tree, the first of the nearest, to the first nearest point in it.
pub(crate) fn spanning_tree(points: &[[Q; 2]]) -> Vec<(usize, usize)> {
    let n = points.len();
    if n < 2 {
        return Vec::new();
    }
    let mut inside = vec![false; n];
    inside[0] = true;
    let mut best: Vec<(Q, usize)> = (0..n).map(|j| (manhattan(&points[0], &points[j]), 0)).collect();
    let mut edges = Vec::new();
    for _ in 1..n {
        let mut pick: Option<usize> = None;
        for j in 0..n {
            if !inside[j] && pick.is_none_or(|p| best[j].0 < best[p].0) {
                pick = Some(j);
            }
        }
        let Some(j) = pick else { break };
        inside[j] = true;
        edges.push((best[j].1, j));
        for m in 0..n {
            if !inside[m] {
                let d = manhattan(&points[j], &points[m]);
                if d < best[m].0 {
                    best[m] = (d, j);
                }
            }
        }
    }
    edges
}

/// The turns of the connections drawn straight: one for a connection not along an axis, and one
/// at each end whose pin does not leave towards the other end (9.2.4), as a wire must turn there.
fn turns(view: &View) -> Q {
    let mut count = 0usize;
    for drawn in &view.s.connections.drawn {
        let points: Vec<(PointId, [Q; 2])> = drawn.points.iter().filter_map(|p| Some((*p, view.point(*p)?))).collect();
        let positions: Vec<[Q; 2]> = points.iter().map(|(_, q)| q.clone()).collect();
        for (a, b) in spanning_tree(&positions) {
            let (pa, qa) = &points[a];
            let (pb, qb) = &points[b];
            count += usize::from(qa[0] != qb[0] && qa[1] != qb[1]);
            count += usize::from(!leaves_towards(view, *pa, qa, qb));
            count += usize::from(!leaves_towards(view, *pb, qb, qa));
        }
    }
    count.into_q()
}

/// Whether a pin at `from` leaves towards `to`; a point that is no pin, or a pin without a
/// direction, leaves any way.
fn leaves_towards(view: &View, p: PointId, from: &[Q; 2], to: &[Q; 2]) -> bool {
    let PointId::Pin(k, pin) = p else { return true };
    let Some((x, y)) = view.s.exit(k, pin, view.orient[k]) else { return true };
    let dot = int(x) * (&to[0] - &from[0]) + int(y) * (&to[1] - &from[1]);
    dot.is_positive()
}

/// The bends of a polyline: vertices where it changes direction.
fn bends(path: &[[Q; 2]]) -> usize {
    let dir = |a: &[Q; 2], b: &[Q; 2]| ((&b[0] - &a[0]).signum(), (&b[1] - &a[1]).signum());
    let mut dirs: Vec<(Q, Q)> =
        path.windows(2).map(|w| dir(&w[0], &w[1])).filter(|d| !(d.0.is_zero() && d.1.is_zero())).collect();
    dirs.dedup();
    dirs.len().saturating_sub(1)
}

/// The pairs of segments of different nets that cross: they share a point inside both, and are
/// not on one line. A horizontal and a vertical segment, which is what rectilinear wires are made
/// of, are compared by their coordinates alone.
fn crossings(segments: &[(usize, [Q; 2], [Q; 2])]) -> Q {
    // (net, the fixed coordinate, the lower and the upper end of the other)
    let mut horizontal: Vec<(usize, &Q, &Q, &Q)> = Vec::new();
    let mut vertical: Vec<(usize, &Q, &Q, &Q)> = Vec::new();
    let mut other: Vec<usize> = Vec::new();
    let span = |a: &'_ Q, b: &'_ Q| if a <= b { (a.clone(), b.clone()) } else { (b.clone(), a.clone()) };
    let mut spans: Vec<(Q, Q)> = Vec::with_capacity(segments.len());
    for (_, a, b) in segments {
        spans.push(if a[1] == b[1] { span(&a[0], &b[0]) } else { span(&a[1], &b[1]) });
    }
    for (i, (net, a, b)) in segments.iter().enumerate() {
        if a == b {
            continue;
        }
        let (lo, hi) = (&spans[i].0, &spans[i].1);
        if a[1] == b[1] {
            horizontal.push((*net, &a[1], lo, hi));
        } else if a[0] == b[0] {
            vertical.push((*net, &a[0], lo, hi));
        } else {
            other.push(i);
        }
    }
    let mut count = 0usize;
    for (nh, y, x1, x2) in &horizontal {
        for (nv, x, y1, y2) in &vertical {
            if nh != nv && *x1 < *x && *x < *x2 && *y1 < *y && *y < *y2 {
                count += 1;
            }
        }
    }
    for (n, &i) in other.iter().enumerate() {
        let (ni, a, b) = &segments[i];
        // against every other segment, the oblique ones once
        for (j, (nj, c, d)) in segments.iter().enumerate() {
            let earlier_oblique = other[..n].contains(&j);
            if j != i && !earlier_oblique && ni != nj && c != d && proper_crossing(a, b, c, d) {
                count += 1;
            }
        }
    }
    count.into_q()
}

/// Whether segments `ab` and `cd` cross at a point inside both, exactly.
fn proper_crossing(a: &[Q; 2], b: &[Q; 2], c: &[Q; 2], d: &[Q; 2]) -> bool {
    let cross =
        |o: &[Q; 2], p: &[Q; 2], q: &[Q; 2]| (&p[0] - &o[0]) * (&q[1] - &o[1]) - (&p[1] - &o[1]) * (&q[0] - &o[0]);
    let (d1, d2) = (cross(a, b, c), cross(a, b, d));
    let (d3, d4) = (cross(c, d, a), cross(c, d, b));
    let opposite = |x: &Q, y: &Q| (x.is_positive() && y.is_negative()) || (x.is_negative() && y.is_positive());
    opposite(&d1, &d2) && opposite(&d3, &d4)
}

/// The envelope of the components' boxes: x1, y1, x2, y2; none without a component.
fn envelope(view: &View) -> Option<[Q; 4]> {
    let mut out: Option<[Q; 4]> = None;
    for k in view.s.components() {
        let Some(b) = view.bbox(k) else { continue };
        out = Some(match out {
            None => b,
            Some(e) => [
                e[0].clone().min(b[0].clone()),
                e[1].clone().min(b[1].clone()),
                e[2].clone().max(b[2].clone()),
                e[3].clone().max(b[3].clone()),
            ],
        });
    }
    out
}

/// `|w / h − 16/9|`; for an envelope without height, its width, so that a flat drawing is not
/// taken for a well-proportioned one.
fn aspect(w: &Q, h: &Q) -> Q {
    if h.is_zero() {
        w.abs()
    } else {
        (w / h - frac(16, 9)).abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Layer, SolverOptions};

    #[test]
    fn the_table_follows_the_names_of_the_options() {
        assert_eq!(TABLE.len(), OBJECTIVES.len());
        for (i, entry) in TABLE.iter().enumerate() {
            assert_eq!(entry.objective as usize, i);
            assert_eq!(entry.objective.name(), OBJECTIVES[i]);
            assert_eq!(Objective::from_name(OBJECTIVES[i]), Some(entry.objective));
        }
        assert_eq!(Objective::from_name("beauty"), None);
        let off: Vec<&str> = TABLE.iter().filter(|e| !e.implemented).map(|e| e.objective.name()).collect();
        assert_eq!(off, ["terminal_wire_alignment", "labels", "bend_alignment"]);
        assert!(!Objective::Turns.entry().structural && Objective::CurrentDirection.entry().structural);
        assert_eq!(Objective::Symmetry.entry().scope, Scope::Group);
    }

    #[test]
    fn objectives_without_their_inputs_are_inert_and_said_so() {
        let options = SolverOptions::default();
        let all = Inputs { supply: true, ground: true, port_kinds: true, wires: true };
        let active = Active::new(&options.objectives, &all);
        // the default layers less terminal_wire_alignment, which this version does not compute
        assert_eq!(active.layers.len(), 10);
        assert_eq!(active.inert.len(), 1);
        assert!(active.inert[0].starts_with("terminal_wire_alignment is not computed"));
        assert_eq!(active.in_order()[0], Objective::CurrentDirection);
        let none = Inputs::default();
        let active = Active::new(&options.objectives, &none);
        assert!(!active.is_on(Objective::CurrentDirection) && !active.is_on(Objective::Turns));
        assert!(active.is_on(Objective::ComponentAlignment) && active.is_on(Objective::Area));
        let current = active.inert.iter().find(|s| s.starts_with("current_direction")).unwrap();
        assert!(current.contains("no supply symbol") && current.contains("no ground symbol"), "{current}");
        // one sentence per objective configured but inert: current, turns, ground, terminal wire
        // alignment, crossings, wire length
        assert_eq!(active.inert.len(), 6, "{:?}", active.inert);
        // an objective off is neither in force nor inert
        assert!(!active.is_on(Objective::SignalFlow));
        assert!(!active.inert.iter().any(|s| s.starts_with("signal_flow")));
    }

    #[test]
    fn layers_compare_lexicographically_within_their_tolerances() {
        let objectives = Objectives {
            layers: vec![
                Layer { items: vec![("area".to_string(), 1)], tolerance: 0.5 },
                Layer { items: vec![("aspect".to_string(), 2)], tolerance: 0.0 },
            ],
        };
        let active = Active::new(&objectives, &Inputs::default());
        assert_eq!(active.layers[0].tolerance, frac(1, 2));
        let t = |a: i64, b: i64| vec![int(a), int(b)];
        assert_eq!(active.compare(&t(1, 5), &t(2, 0)), Ordering::Less);
        // within the tolerance of the first layer, the second decides
        assert_eq!(active.compare(&[frac(1, 1), int(5)], &[frac(3, 2), int(0)]), Ordering::Greater);
        assert_eq!(active.compare(&t(1, 5), &t(1, 5)), Ordering::Equal);
    }

    #[test]
    fn gap_kinds_count_kinds_and_prefer_one_dominant_spacing() {
        let q = |v: &[i64]| v.iter().map(|x| int(*x)).collect::<Vec<Q>>();
        // one gap kind: 0
        assert_eq!(gap_kinds(q(&[0, 2, 4, 6]).iter()), int(0));
        // repeats count once, fewer than two coordinates cost nothing
        assert_eq!(gap_kinds(q(&[3, 3, 3]).iter()), int(0));
        // two kinds 2, 2, 5: 1 + 1 − (4/9 + 1/9) = 13/9
        assert_eq!(gap_kinds(q(&[0, 2, 4, 9]).iter()), frac(13, 9));
        // two kinds 2, 5: 1 + 1 − (1/4 + 1/4) = 3/2, worse than one dominant kind
        assert_eq!(gap_kinds(q(&[0, 2, 7]).iter()), frac(3, 2));
    }

    #[test]
    fn spanning_trees_bends_and_crossings() {
        let p = |x: i64, y: i64| [int(x), int(y)];
        let points = [p(0, 0), p(10, 0), p(1, 1), p(10, 5)];
        // (1, 0) is as near to (0, 0) as to (1, 1): it hangs from the earlier
        assert_eq!(spanning_tree(&points), vec![(0, 2), (0, 1), (1, 3)]);
        assert_eq!(bends(&[p(0, 0), p(5, 0), p(5, 5), p(5, 9), p(7, 9)]), 2);
        assert_eq!(bends(&[p(0, 0), p(5, 0)]), 0);
        // a plus crosses, a T and a shared end do not
        assert!(proper_crossing(&p(0, 0), &p(4, 0), &p(2, -2), &p(2, 2)));
        assert!(!proper_crossing(&p(0, 0), &p(4, 0), &p(2, 0), &p(2, 2)));
        assert!(!proper_crossing(&p(0, 0), &p(4, 0), &p(4, 0), &p(4, 2)));
        let segments = vec![(0, p(0, 0), p(4, 0)), (1, p(2, -2), p(2, 2)), (0, p(3, -1), p(3, 1))];
        // the two of net 0 cross each other but count only against net 1
        assert_eq!(crossings(&segments), int(1));
        // oblique segments: against all the others, each pair once
        let segments =
            vec![(0, p(0, 0), p(4, 4)), (1, p(0, 4), p(4, 0)), (2, p(2, -1), p(2, 5)), (3, p(-1, 2), p(5, 2))];
        // the two diagonals cross each other at (2, 2), where the plus also crosses: every pair
        assert_eq!(crossings(&segments), int(6));
        // touching at an end is no crossing
        assert_eq!(crossings(&[(0, p(0, 0), p(4, 0)), (1, p(4, 0), p(4, 3))]), int(0));
        assert_eq!(aspect(&int(16), &int(9)), int(0));
        assert_eq!(aspect(&int(4), &int(0)), int(4));
    }
}
