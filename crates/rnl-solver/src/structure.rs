//! The structure of the problem, the same for every configuration (plan 5.4; algorithm synthesis
//! 5.3), and the sources of candidates that the configured objectives call for.
//!
//! - **What each object is** ([`ObjectInfo`]): an element of the rows (an instance, a bundle
//!   (11.8) or a virtual part with a position of its own, 12.1), a supply, ground or port symbol,
//!   a point or a text; the device it is, read off its symbol (NMOS, PMOS, NPN, PNP: the pins
//!   by the positional order of SPICE, drain or collector 1, gate or base 2, source or emitter 3);
//!   for a stub of the standard package, the pin it hangs from.
//! - **The connection graph** ([`Connections`]): the nets as far as the problem shows them, from
//!   the ROUTEs (13.7), the wires (13.1) and the usages that attach a symbol to a net (12.1); and
//!   what is drawn as wires, the nets of the ROUTEs and the wires written, which the routing
//!   objectives measure and the wire length of the coordinate LP shortens. The problem does not
//!   carry the netlist itself, so a net that is neither routed nor wired is known only through
//!   the usages on it.
//! - **Supply and ground** ([`Power`]): the usages of `$SUPPLY_STUB`, `$GND_STUB`, `$SUPPLY` and
//!   `$GND`, the nets they attach to, the terminals they hang from or are drawn on (12.1), and the
//!   nets of those terminals as far as the problem shows them.
//! - **Current depth**: the distance from a supply terminal through the pins the current runs
//!   through (drain and source, collector and emitter, both ends of other devices) to each element,
//!   by breadth-first search, no operating point; an element reached only through a gate takes the
//!   depth of the element it is connected to.
//! - **The tree of groups** ([`GroupTree`]): the groups (11.5) nested by containment, two groups
//!   that overlap split in three, the part they share between the others (algorithm synthesis
//!   3.5), the whole level the root. A group is where symmetry and even spacing are judged, and a
//!   block of the search; it constrains nothing.
//! - **What is left to decide** ([`Open`]): the orientation classes no ORIENT fixes, and the
//!   ORDERs and DISTANCEs whose direction or side the facts leave open.
//! - **Rows and precedences**: the rows the elements start in, by current depth when the current
//!   direction is in force, otherwise by the height the witness of the resolution gives them. The
//!   facts are rows and columns too (plan 5.4): an ALIGN on y joins its elements in one row, and so,
//!   without the current depth, does an ORDER or a DISTRIBUTE on x, as far as the facts kept let
//!   its elements share a height; an ALIGN on x is a column, its elements in rows of their own.
//!   The left-to-right precedences are those that the facts on x with a side give the elements
//!   they join; an ORDER or a DISTANCE whose side is left open gives its own as the witness has
//!   them, and the search may reverse them ([`Reversible`]).
//! - **The sources of candidates** ([`Source`]), instantiated from the configured objectives in
//!   the order of their layers, each objective with a structural side adding its own, and the
//!   objectives configured but inert recorded with the reason ([`crate::objectives::Active`]).

use std::collections::{BTreeMap, BTreeSet};

use num_traits::{One, Signed, Zero};
use rnl_runtime::geom::Orient;
use rnl_runtime::markup::Run;
use rnl_runtime::problem::{Axis, FactKind, Filler, LevelProblem, ObjectKind, PointRef, Position, Sense, TextInfo};

use crate::model::{turned, Model, PointId};
use crate::objectives::{Active, Inputs, Objective};
use crate::options::Caps;
use crate::rational::{frac, from_f64, int, Q};
use crate::resolve::{point_value, Geometry, Resolved};
use crate::search::{with_relations, RelKind, Relation, Tagged};
use crate::{SolverOptions, Work};

/// A pin, by object and by the index of the pin in its symbol (9.2.4).
pub(crate) type PinKey = (usize, usize);

/// What an object is to the search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    /// An instance, a bundle, or a virtual part with a symbol and a position of its own: a member
    /// of the rows.
    Element,
    /// The symbol of `$SUPPLY`.
    Supply,
    /// The symbol of `$GND`.
    Ground,
    /// The symbol of `$IOPORT`.
    Port,
    Point,
    Text,
    /// A virtual part drawn at the point of its main object (12.1) that is none of the above.
    Other,
}

/// The device an element is, by its symbol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Device {
    NMos,
    PMos,
    Npn,
    Pnp,
    Other,
}

impl Device {
    /// The device a symbol draws, by its name: one that has `NMOS`, `PMOS`, `NPN` or `PNP` in it.
    fn of(name: &str) -> Device {
        let n = name.to_ascii_uppercase();
        if n.contains("NMOS") {
            Device::NMos
        } else if n.contains("PMOS") {
            Device::PMos
        } else if n.contains("NPN") {
            Device::Npn
        } else if n.contains("PNP") {
            Device::Pnp
        } else {
            Device::Other
        }
    }

    /// For a transistor, whether its source (emitter) sits below its drain (collector) when the
    /// current runs down: an n-type. None for other devices.
    pub(crate) fn n_type(self) -> Option<bool> {
        match self {
            Device::NMos | Device::Npn => Some(true),
            Device::PMos | Device::Pnp => Some(false),
            Device::Other => None,
        }
    }

    /// Whether the static current runs through a pin with these terminals: drain and source,
    /// collector and emitter of a transistor, every pin of another device.
    fn conducts(self, terminals: &[String]) -> bool {
        match self {
            Device::Other => true,
            _ => terminals.iter().any(|t| t == "1" || t == "3"),
        }
    }
}

/// A pin of a symbol, exactly.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PinInfo {
    pub local: (Q, Q),
    /// The direction a wire leaves it (9.2.4), as a unit step; none when it has none or one not
    /// along an axis.
    pub dir: Option<(i64, i64)>,
    pub terminals: Vec<String>,
}

/// A symbol, exactly: its box and its pins (9.2.3, 9.2.4).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SymbolInfo {
    /// x1, y1, x2, y2, the smaller first.
    pub bbox: [Q; 4],
    pub pins: Vec<PinInfo>,
}

/// A stub of the standard package (`$GND_STUB`, `$SUPPLY_STUB`): its symbol hangs `rise` above
/// the pin `host` (below for a negative `rise`), as its facts say.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Stub {
    pub host: PinKey,
    pub rise: Q,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ObjectInfo {
    pub role: Role,
    pub device: Device,
    pub symbol: Option<usize>,
    pub oriented: bool,
    /// It has a position of its own (11.1).
    pub own: bool,
    pub stub: Option<Stub>,
    /// A fact kept reaches one of its points.
    pub fact_touched: bool,
}

/// A net as far as the problem shows it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Net {
    pub name: String,
    pub pins: Vec<PinKey>,
    pub supply: bool,
    pub ground: bool,
}

/// What is drawn as wires: a net that a ROUTE asks for, or a wire written.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Drawn {
    pub net: String,
    /// The points it joins, pins and point objects, in order.
    pub points: Vec<PointId>,
    /// Whether a ROUTE asks for it (13.7), as opposed to a wire written (13.1).
    pub routed: bool,
}

/// The connection graph.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Connections {
    /// The nets, by name.
    pub nets: Vec<Net>,
    /// The nets of each pin.
    pub nets_of: BTreeMap<PinKey, Vec<usize>>,
    /// The nets of the ROUTEs, one per net, then the wires written.
    pub drawn: Vec<Drawn>,
    /// For each pin, the pins of other objects it is drawn to: those of its ROUTE's net, and its
    /// neighbours along the wires written.
    pub targets: BTreeMap<PinKey, Vec<PinKey>>,
    /// The net a ROUTE asks to draw that each pin is on, by its index in `drawn`: two pins with
    /// one are joined by the router, which leaves each by its lead (13.7).
    pub route_of: BTreeMap<PinKey, usize>,
    /// The modes of the wires written that run from a pin straight to the next one, by the pair
    /// in the order written (13.1).
    pub written: BTreeMap<(PinKey, PinKey), Vec<String>>,
}

impl Connections {
    /// Whether the router joins two pins: a ROUTE asks for the net they are both on.
    pub(crate) fn routed(&self, a: PinKey, b: PinKey) -> bool {
        self.route_of.get(&a).is_some_and(|n| self.route_of.get(&b) == Some(n))
    }
}

/// Supply and ground.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Power {
    /// The supply symbols, by object.
    pub supply: Vec<usize>,
    pub ground: Vec<usize>,
    pub supply_nets: BTreeSet<String>,
    pub ground_nets: BTreeSet<String>,
    /// The pins on a supply or ground net: the terminals stubs hang from, the terminals a supply
    /// or ground symbol is drawn on (12.1), and the pins on those nets.
    pub supply_pins: BTreeSet<PinKey>,
    pub ground_pins: BTreeSet<PinKey>,
}

/// What a port is, from what its net reaches: an input reaches the gate or base of a transistor
/// and no drain, source, collector or emitter; an output reaches a drain or a collector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PortKind {
    Input,
    Output,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Port {
    pub object: usize,
    pub net: Option<String>,
    pub kind: PortKind,
}

/// Where a text goes when no fact decides its position (13.3): "a text attached to a point usually
/// goes at that point, and a text attached to an object next to the object". At the point itself,
/// or beside the coordinate point of the object, the origin of an instance, a usage drawn or a
/// bundle, or the pin of a terminal, by the offset that a viewer that does not solve puts it at.
/// A wire or a net has no coordinate point (13.3), and a text attached to one is anchored beside
/// one of its points only for the axis that no fact kept reaches, when a fact kept reaches the
/// other: with no fact kept reaching it, the solution leaves it free and the viewer places it
/// (`crate::driver`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Anchor {
    /// The object it is attached to, or whose terminal; for a wire or a net, the object of the
    /// point it is beside.
    pub object: usize,
    /// The pin of the terminal, in the object's symbol.
    pub pin: Option<usize>,
    /// From the point, along the axes of the scene.
    pub offset: [Q; 2],
    /// What the text is attached to has a coordinate point (13.3), which the anchor is: a point,
    /// an object or a terminal; not a wire or a net, whose anchor is only one of its points.
    pub point: bool,
}

/// Where a viewer that does not solve puts a text attached to an object, from the object's
/// coordinate point (13.3); the solver puts it there too, so that a text looks the same solved.
fn beside() -> [Q; 2] {
    [Q::new(6.into(), 5.into()), Q::new(6.into(), 5.into())]
}

/// The first of some points that is the origin of an object or a pin, as an anchor's object and
/// pin.
fn first_point(points: &[PointRef]) -> Option<(usize, Option<usize>)> {
    points.iter().find_map(|r| match r {
        PointRef::Origin(k) => Some((*k, None)),
        PointRef::Pin { object, pin, .. } => Some((*object, Some(*pin))),
        PointRef::Centre(_) => None,
    })
}

/// The anchors of the texts with a position of their own ([`Structure::anchors`]): those attached
/// to a point, an object with a coordinate point, or a terminal with a pin; and those attached to
/// a wire, beside its first point, or to a net, beside the first point of its first wire, else of
/// its first ROUTE, else beside its first pin, as the viewer puts a net's label at the start of
/// its first wire, else on one of its pins (13.3).
fn anchors(problem: &LevelProblem, connections: &Connections) -> BTreeMap<usize, Anchor> {
    let mut out = BTreeMap::new();
    for (k, o) in problem.objects.iter().enumerate() {
        let (ObjectKind::Text, Position::Own { .. }, Some(t)) = (o.kind, &o.position, &o.text) else { continue };
        let anchor = match &t.attach {
            Filler::Object(j) => match problem.objects.get(*j).map(|a| a.kind) {
                Some(ObjectKind::Point) => {
                    Anchor { object: *j, pin: None, offset: [Q::zero(), Q::zero()], point: true }
                }
                Some(ObjectKind::Instance | ObjectKind::Usage | ObjectKind::Bundle) => {
                    Anchor { object: *j, pin: None, offset: beside(), point: true }
                }
                _ => continue,
            },
            Filler::Terminal { object, pin: Some(pin), .. } => {
                Anchor { object: *object, pin: Some(*pin), offset: beside(), point: true }
            }
            Filler::Wire(w) => {
                let Some((object, pin)) = problem.wires.get(*w).and_then(|w| first_point(&w.anchors)) else { continue };
                Anchor { object, pin, offset: beside(), point: false }
            }
            Filler::Net(name) => {
                let wire = problem.wires.iter().filter(|w| w.nets.contains(name)).find_map(|w| first_point(&w.anchors));
                let route = || problem.routes.iter().filter(|r| r.net == *name).find_map(|r| first_point(&r.pins));
                let pin = || {
                    let net = connections.nets.iter().find(|n| n.name == *name)?;
                    net.pins.first().map(|&(object, pin)| (object, Some(pin)))
                };
                let Some((object, pin)) = wire.or_else(route).or_else(pin) else { continue };
                Anchor { object, pin, offset: beside(), point: false }
            }
            _ => continue,
        };
        out.insert(k, anchor);
    }
    out
}

/// A pair drawn as mirror images: `b` is `a` mirrored (14.9), about `axis` when it has one. From
/// `$MIRROR_PAIR`, `$DIFFPAIR`, `$DIFFPAIR_TAIL` and `$CURRENT_MIRROR`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Pair {
    pub a: usize,
    pub b: usize,
    pub axis: Option<usize>,
}

/// Where a node of the tree of groups comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
    /// The whole level.
    Root,
    /// A group, by its index in [`LevelProblem::groups`].
    Group(usize),
    /// Two groups that overlap, by index: their union, whose children are the part only in the
    /// first, the part they share and the part only in the second, in that order. A group with
    /// the same members is this node.
    Union(usize, usize),
    /// A part of a [`NodeKind::Union`]: 0 only in the first group, 1 shared, 2 only in the second.
    /// A part with the members of a node made before is that node, of its own kind.
    Part(usize, usize, u8),
}

/// A node of the tree of groups.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GroupNode {
    pub kind: NodeKind,
    /// Every element in it, sorted.
    pub members: Vec<usize>,
    /// Its children, in order: for a union, the three parts in their order.
    pub children: Vec<usize>,
    /// The elements in it and in no child, sorted.
    pub own: Vec<usize>,
    /// The order of its children is fixed: a union keeps the shared part between the others, its
    /// children in their order or the reverse.
    pub fixed_order: bool,
}

/// The groups nested as a tree; node 0 is the root.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GroupTree {
    pub nodes: Vec<GroupNode>,
}

impl GroupTree {
    /// The nodes, children before their parents, the root last.
    pub(crate) fn post_order(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![(0usize, false)];
        while let Some((n, done)) = stack.pop() {
            if done {
                out.push(n);
            } else {
                stack.push((n, true));
                for &c in self.nodes[n].children.iter().rev() {
                    stack.push((c, false));
                }
            }
        }
        out
    }

    /// The deepest node that holds every one of `objects`.
    pub(crate) fn lowest_common(&self, objects: &[usize]) -> usize {
        let mut node = 0;
        loop {
            let next = self.nodes[node]
                .children
                .iter()
                .copied()
                .find(|&c| objects.iter().all(|k| self.nodes[c].members.binary_search(k).is_ok()));
            match next {
                Some(c) => node = c,
                None => return node,
            }
        }
    }
}

/// An orientation class no ORIENT fixes: its root, and each member with its label,
/// `ori(member) = ori(root).then(label)` (14.9).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FreeClass {
    pub root: usize,
    pub members: Vec<(usize, Orient)>,
}

/// What the facts kept leave to decide.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Open {
    pub classes: Vec<FreeClass>,
    /// The ORDERs given only an axis, by fact: which way they run.
    pub orders: Vec<usize>,
    /// The DISTANCEs given only an axis, by fact: which side.
    pub sides: Vec<usize>,
}

/// A fact kept on x whose direction or side the facts leave open ([`Open::orders`],
/// [`Open::sides`]): the pairs of elements `(a, b)` it puts `a` left of `b`, as the witness has
/// them. The search may reverse them all, and the resolution then turns the fact's literal.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Reversible {
    pub fact: usize,
    pub pairs: Vec<(usize, usize)>,
}

/// A source of candidates (algorithm synthesis 5.3), what a configured objective's structural
/// side, or the problem itself, contributes to making them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Source {
    /// Rows by current depth (current_direction).
    DepthRows,
    /// Transistors oriented with their sources towards the ground, two-terminal elements with an
    /// end on a supply or the ground hung towards it, those between two signals laid across
    /// (current_direction).
    SourcesDown,
    /// Ground symbols hang below their pins, two-terminal elements with an end on the ground hang
    /// that end down (ground_direction).
    GroundsDown,
    /// Inputs on the left, outputs on the right (signal_flow).
    InputsLeft,
    /// Elements connected across rows put in one column, in an order that keeps connections short
    /// and uncrossed: the barycentre of what each is connected to (component_alignment,
    /// crossings, wire_length).
    Barycentre,
    /// Rows evenly spaced (distribution).
    EvenRows,
    /// Mirror pairs given mirrored orientations (symmetry).
    MirrorPairs,
    /// Elements in neighbouring rows connected pin to pin put in one column, the pins on one
    /// vertical line (component_alignment).
    Columns,
    /// Pins turned to face what they are connected to: always.
    FacingPins,
    /// The facts: an ALIGN on y joins a row, and so, without the current depth, does an ORDER or
    /// a DISTRIBUTE on x; an ALIGN on x is a column; the facts on x with a side order what they
    /// join: always.
    Facts,
    /// The order of the netlist, the last tie-break: always.
    NetlistOrder,
}

/// The structure of a level.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Structure {
    pub symbols: Vec<SymbolInfo>,
    pub objects: Vec<ObjectInfo>,
    /// The elements, in the order of the objects.
    pub elements: Vec<usize>,
    pub connections: Connections,
    pub power: Power,
    pub ports: Vec<Port>,
    pub pairs: Vec<Pair>,
    /// The stubs that hang from each object, by the object.
    pub hanging: BTreeMap<usize, Vec<usize>>,
    /// The box of the texts each object's symbol places (13.3), estimated, in the symbol's own
    /// coordinates, by the object ([`text_box`]).
    pub labels: BTreeMap<usize, [Q; 4]>,
    /// Where each text with a position of its own goes when nothing decides it, by the text: at
    /// the point it is attached to, or beside the object (13.3).
    pub anchors: BTreeMap<usize, Anchor>,
    pub tree: GroupTree,
    /// The components of each scope of the objectives measured within groups: each group, then
    /// those in no group (when any).
    pub scopes: Vec<Vec<usize>>,
    pub open: Open,
    /// The current depth of each object, when the current direction is in force: none for those
    /// not elements and those not reached.
    pub depth: Option<Vec<Option<u32>>>,
    /// The rows the elements start in, the top first, each in the order of the objects.
    pub rows: Vec<Vec<usize>>,
    /// Pairs of elements `(a, b)` that a fact kept on x with a side puts `a` left of `b`, as the
    /// witness has them: an ORDER with a direction, a DISTANCE with a direction and a gap not
    /// below 0.
    pub precedence: BTreeSet<(usize, usize)>,
    /// The facts kept on x whose side is open, with the pairs they order.
    pub reversible: Vec<Reversible>,
    pub active: Active,
    pub sources: Vec<Source>,
    /// Anything worth saying, such as groups that cannot be nested.
    pub notes: Vec<String>,
    /// The model the candidates are resolved on: without the tolerance when the facts kept hold so
    /// (`Resolved::as_written`), otherwise with the constants of the facts kept that rank first held
    /// as written as far as they can be (`Resolved::placing`).
    pub model: Model,
    /// The witness of the resolution: the facts kept hold there.
    pub witness: Geometry,
    /// The facts kept, in order.
    pub kept: Vec<usize>,
    /// The caps of work of the solver's own resolutions on this structure: the relations of the
    /// candidates, the analysis of the rows, the coordinate LP ([`crate::options::CAPS`]).
    pub caps: Caps,
}

impl Structure {
    /// Whether a source is in force.
    pub(crate) fn has(&self, s: Source) -> bool {
        self.sources.contains(&s)
    }

    /// The objects measured as components: elements, and supply, ground and port symbols, with a
    /// symbol and a place.
    pub(crate) fn components(&self) -> impl Iterator<Item = usize> + '_ {
        self.objects.iter().enumerate().filter_map(|(k, o)| {
            let component = matches!(o.role, Role::Element | Role::Supply | Role::Ground | Role::Port);
            (component && o.symbol.is_some()).then_some(k)
        })
    }

    /// The pins of an object's symbol.
    pub(crate) fn pins(&self, k: usize) -> &[PinInfo] {
        self.objects[k].symbol.and_then(|s| self.symbols.get(s)).map_or(&[], |s| &s.pins)
    }

    fn pin_with(&self, k: usize, terminal: &str) -> Option<usize> {
        self.pins(k).iter().position(|p| p.terminals.iter().any(|t| t == terminal))
    }

    /// Whether the static current runs through a pin (see [`Device`]).
    pub(crate) fn conducts(&self, (k, pin): PinKey) -> bool {
        self.pins(k).get(pin).is_some_and(|p| self.objects[k].device.conducts(&p.terminals))
    }

    /// The drain (collector) of a transistor: the pin of terminal 1.
    pub(crate) fn drain(&self, k: usize) -> Option<usize> {
        self.pin_with(k, "1")
    }

    /// The source (emitter) of a transistor: the pin of terminal 3.
    pub(crate) fn source(&self, k: usize) -> Option<usize> {
        self.pin_with(k, "3")
    }

    /// The position of a pin in its symbol turned by `o` (11.2).
    pub(crate) fn pin_offset(&self, k: usize, pin: usize, o: Orient) -> Option<[Q; 2]> {
        let local = &self.pins(k).get(pin)?.local;
        Some([turned(o, local, 0), turned(o, local, 1)])
    }

    /// The direction a wire leaves a pin, turned by `o`.
    pub(crate) fn exit(&self, k: usize, pin: usize, o: Orient) -> Option<(i64, i64)> {
        let (x, y) = self.pins(k).get(pin)?.dir?;
        let (a, b, c, d) = o.matrix();
        Some((i64::from(a) * x + i64::from(b) * y, i64::from(c) * x + i64::from(d) * y))
    }

    /// The box of an object's symbol turned by `o`, about its origin.
    pub(crate) fn box_offset(&self, k: usize, o: Orient) -> Option<[Q; 4]> {
        turned_box(&self.symbols.get(self.objects[k].symbol?)?.bbox, o)
    }

    /// The box of the texts an object's symbol places, turned by `o` with the object, as the
    /// text boxes of a symbol turn and mirror with it (9.2.5).
    pub(crate) fn label_offset(&self, k: usize, o: Orient) -> Option<[Q; 4]> {
        turned_box(self.labels.get(&k)?, o)
    }

    /// Where a pin is, with every object's origin at `at` and orientation `orient`.
    pub(crate) fn pin_at(&self, at: &[[Q; 2]], orient: &[Orient], k: usize, pin: usize) -> Option<[Q; 2]> {
        let o = self.pin_offset(k, pin, orient[k])?;
        Some([&at[k][0] + &o[0], &at[k][1] + &o[1]])
    }

    /// Where an object's box is.
    pub(crate) fn box_at(&self, at: &[[Q; 2]], orient: &[Orient], k: usize) -> Option<[Q; 4]> {
        let b = self.box_offset(k, orient[k])?;
        Some([&at[k][0] + &b[0], &at[k][1] + &b[1], &at[k][0] + &b[2], &at[k][1] + &b[3]])
    }

    /// Where a point of the model is; none for a centre.
    pub(crate) fn point_at(&self, at: &[[Q; 2]], orient: &[Orient], p: PointId) -> Option<[Q; 2]> {
        match p {
            PointId::Origin(k) | PointId::Text(k) => at.get(k).cloned(),
            PointId::Pin(k, pin) => self.pin_at(at, orient, k, pin),
            PointId::Centre(_) => None,
        }
    }

    /// The free class an object is in, by the index in [`Open::classes`].
    pub(crate) fn class_of(&self, k: usize) -> Option<usize> {
        self.open.classes.iter().position(|c| c.members.iter().any(|(m, _)| *m == k))
    }

    /// The origin of every object under a placement: its own position, the point it is drawn at
    /// (12.1), or its place in its owner's symbol (13.3).
    pub(crate) fn origins(&self, problem: &LevelProblem, g: &Geometry) -> Vec<[Q; 2]> {
        (0..problem.objects.len())
            .map(|k| {
                let at = |axis| point_value(problem, g, &PointRef::Origin(k), axis, 0).unwrap_or_default();
                [at(0), at(1)]
            })
            .collect()
    }
}

/// The structure of a level, from its resolution, for the options given, its own resolutions,
/// those of the rows here and of the relations of the candidates later, bounded by `caps`. The work
/// of the resolutions it makes is added to `work`.
pub(crate) fn analyse(
    problem: &LevelProblem,
    resolved: &Resolved,
    options: &SolverOptions,
    caps: Caps,
    work: &mut Work,
) -> Structure {
    let mut notes = Vec::new();
    let symbols: Vec<SymbolInfo> = problem.symbols.iter().map(symbol_info).collect();
    let model = resolved.placing.clone();
    let witness = resolved.witness.geometry.clone();
    let kept = resolved.kept.clone();
    let mut objects = object_infos(problem, &model, &kept);
    let power = power(problem, &mut objects);
    let mut connections = connections(problem, &model);
    let power = power_pins(power, &mut connections);
    let elements: Vec<usize> = (0..objects.len()).filter(|&k| objects[k].role == Role::Element).collect();
    let ports = ports(problem, &objects, &symbols, &connections);
    let pairs = pairs(problem, &objects);
    let open = open(problem, resolved);
    let (tree, tree_notes) = group_tree(problem, &objects, &elements);
    notes.extend(tree_notes);
    let scopes = scopes(problem, &objects);
    let inputs = Inputs {
        supply: !power.supply.is_empty(),
        ground: !power.ground.is_empty(),
        port_kinds: ports.iter().any(|p| p.kind != PortKind::Unknown),
        wires: !problem.routes.is_empty() || !problem.wires.is_empty(),
    };
    let active = Active::new(&options.objectives, &inputs);
    let sources = sources(&active);
    let mut hanging: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (k, o) in objects.iter().enumerate() {
        if let Some(stub) = &o.stub {
            hanging.entry(stub.host.0).or_default().push(k);
        }
    }
    let mut labels: BTreeMap<usize, [Q; 4]> = BTreeMap::new();
    for o in &problem.objects {
        // the texts that move with their owner on both axes; one that the scene places on an axis
        // does not (13.3)
        let (Position::InSymbol { owner, local, replaced: None }, Some(t)) = (&o.position, &o.text) else { continue };
        let Some(b) = text_box(t, (number(local.0), number(local.1))) else { continue };
        labels
            .entry(*owner)
            .and_modify(|a| {
                for i in 0..2 {
                    if b[i] < a[i] {
                        a[i] = b[i].clone();
                    }
                    if b[i + 2] > a[i + 2] {
                        a[i + 2] = b[i + 2].clone();
                    }
                }
            })
            .or_insert(b);
    }
    let anchors = anchors(problem, &connections);
    let mut s = Structure {
        symbols,
        objects,
        elements,
        connections,
        power,
        ports,
        pairs,
        hanging,
        labels,
        anchors,
        tree,
        scopes,
        open,
        depth: None,
        rows: Vec::new(),
        precedence: BTreeSet::new(),
        reversible: Vec::new(),
        active,
        sources,
        notes,
        model,
        witness,
        kept,
        caps,
    };
    if s.has(Source::DepthRows) {
        s.depth = Some(depths(&s));
    }
    s.rows = rows(problem, &s, work);
    (s.precedence, s.reversible) = precedence(problem, &s);
    s
}

/// A box turned by an orientation about the origin (11.2).
fn turned_box(b: &[Q; 4], o: Orient) -> Option<[Q; 4]> {
    let corners = [(b[0].clone(), b[1].clone()), (b[2].clone(), b[3].clone())];
    let xs = [turned(o, &corners[0], 0), turned(o, &corners[1], 0)];
    let ys = [turned(o, &corners[0], 1), turned(o, &corners[1], 1)];
    let (x1, x2) = if xs[0] <= xs[1] { (xs[0].clone(), xs[1].clone()) } else { (xs[1].clone(), xs[0].clone()) };
    let (y1, y2) = if ys[0] <= ys[1] { (ys[0].clone(), ys[1].clone()) } else { (ys[1].clone(), ys[0].clone()) };
    Some([x1, y1, x2, y2])
}

/// The characters of a line of markup, as wide as they are drawn side by side: a fraction as wide
/// as the wider of its parts (13.6).
fn line_chars(runs: &[Run]) -> usize {
    runs.iter()
        .map(|r| match r {
            Run::Text { text, .. } => text.chars().filter(|c| *c != rnl_runtime::markup::LITERAL).count(),
            Run::Frac { num, den, .. } => line_chars(num).max(line_chars(den)),
        })
        .sum()
}

/// An estimate of the box of a text anchored at `at` (9.2.5): each character 0.6 of the size wide,
/// each line 1.2 of it high, the box beside the anchor as `halign` and `valign` say (the baseline
/// 0.8 of the size below the top of the first line), turned by `rotate`. The solver keeps the
/// symbols' texts clear of their neighbours by it; the viewer lays the text out itself. None for a
/// text without characters.
pub(crate) fn text_box(t: &TextInfo, at: (Q, Q)) -> Option<[Q; 4]> {
    let size = t
        .style
        .iter()
        .find(|(n, _)| n == "size")
        .and_then(|(_, v)| v.num())
        .and_then(from_f64)
        .unwrap_or_else(|| int(1));
    let chars = t.lines.iter().map(|l| line_chars(l)).max().unwrap_or(0);
    if chars == 0 {
        return None;
    }
    let w = int(chars as i64) * frac(3, 5) * &size;
    let h = int(t.lines.len() as i64) * frac(6, 5) * &size;
    let (x1, x2) = match t.halign.as_str() {
        "middle" => (-(&w / int(2)), &w / int(2)),
        "end" => (-w.clone(), Q::zero()),
        _ => (Q::zero(), w.clone()),
    };
    let (y1, y2) = match t.valign.as_str() {
        "top" => (-h.clone(), Q::zero()),
        "middle" => (-(&h / int(2)), &h / int(2)),
        "bottom" => (Q::zero(), h.clone()),
        // the first line's baseline: the box from 0.8 of the size above it down
        _ => (frac(4, 5) * &size - &h, frac(4, 5) * &size),
    };
    // 0, 90, 180 or 270 (9.2.5), read exactly
    let r = from_f64(t.rotate).unwrap_or_default();
    let turn = int(360);
    let r = &r - (&r / &turn).floor() * &turn;
    let quarter = [(90, Orient::R90), (180, Orient::R180), (270, Orient::R270)]
        .into_iter()
        .find(|(d, _)| r == int(*d))
        .map_or(Orient::R0, |(_, o)| o);
    let b = turned_box(&[x1, y1, x2, y2], quarter)?;
    Some([&b[0] + &at.0, &b[1] + &at.1, &b[2] + &at.0, &b[3] + &at.1])
}

fn number(v: f64) -> Q {
    from_f64(v).unwrap_or_default()
}

fn symbol_info(s: &rnl_runtime::problem::ProblemSymbol) -> SymbolInfo {
    // the corners compared as rationals, as every decision is
    let b = s.bbox.map(number);
    let order = |a: &Q, c: &Q| if a <= c { (a.clone(), c.clone()) } else { (c.clone(), a.clone()) };
    let ((x1, x2), (y1, y2)) = (order(&b[0], &b[2]), order(&b[1], &b[3]));
    let pins = s
        .pins
        .iter()
        .map(|p| PinInfo {
            local: (number(p.pos.0), number(p.pos.1)),
            dir: p.dir.and_then(|d| match d % 360 {
                0 => Some((1, 0)),
                90 => Some((0, 1)),
                180 => Some((-1, 0)),
                270 => Some((0, -1)),
                _ => None,
            }),
            terminals: p.terminals.clone(),
        })
        .collect();
    SymbolInfo { bbox: [x1, y1, x2, y2], pins }
}

fn object_infos(problem: &LevelProblem, model: &Model, kept: &[usize]) -> Vec<ObjectInfo> {
    let mut touched = vec![false; problem.objects.len()];
    for &f in kept {
        for axis in 0..2 {
            for p in &model.units[f].touched[axis] {
                let k = match p {
                    PointId::Origin(k) => Some(*k),
                    PointId::Pin(k, _) => model.points.get(p).and_then(|i| i.object).or(Some(*k)),
                    // the owner of a text on the axes its symbol gives; on the one the scene gives,
                    // the fact reaches the text alone (13.3)
                    PointId::Text(k) => model.points.get(p).and_then(|i| i.object_on(axis)).or(Some(*k)),
                    PointId::Centre(_) => None,
                };
                if let Some(t) = k.and_then(|k| touched.get_mut(k)) {
                    *t = true;
                }
            }
        }
    }
    problem
        .objects
        .iter()
        .enumerate()
        .map(|(k, o)| {
            let own = matches!(o.position, Position::Own { .. });
            let definition = o.usage.and_then(|u| problem.usages.get(u)).map(|u| u.definition.as_str());
            let role = match (o.kind, definition) {
                (ObjectKind::Point, _) => Role::Point,
                (ObjectKind::Text, _) => Role::Text,
                (ObjectKind::Usage, Some("$GND")) => Role::Ground,
                (ObjectKind::Usage, Some("$SUPPLY")) => Role::Supply,
                (ObjectKind::Usage, Some("$IOPORT")) => Role::Port,
                (ObjectKind::Usage, _) if own && o.symbol.is_some() => Role::Element,
                (ObjectKind::Usage, _) => Role::Other,
                (ObjectKind::Instance | ObjectKind::Bundle, _) if own => Role::Element,
                (ObjectKind::Instance | ObjectKind::Bundle, _) => Role::Other,
            };
            let device = o.symbol.and_then(|s| problem.symbols.get(s)).map_or(Device::Other, |s| Device::of(&s.name));
            ObjectInfo {
                role,
                device: if role == Role::Element { device } else { Device::Other },
                symbol: o.symbol,
                oriented: o.orient.is_some(),
                own,
                stub: None,
                fact_touched: touched[k],
            }
        })
        .collect()
}

/// A number of a usage's parameters.
fn param(problem: &LevelProblem, u: usize, name: &str) -> Option<Q> {
    problem.usages[u].params.iter().find(|(n, _)| n == name).and_then(|(_, v)| v.num()).and_then(from_f64)
}

/// The supply and ground symbols, the nets they attach to, the terminals they are drawn on, and
/// the stubs with the pins they hang from.
fn power(problem: &LevelProblem, objects: &mut [ObjectInfo]) -> Power {
    let mut p = Power::default();
    for usage in &problem.usages {
        let supply = match usage.definition.as_str() {
            "$SUPPLY" => true,
            "$GND" => false,
            _ => continue,
        };
        if let Some(k) = usage.object {
            if supply {
                p.supply.push(k);
            } else {
                p.ground.push(k);
            }
        }
        for role in usage.roles.iter().filter(|r| r.role == "p") {
            for m in &role.members {
                match m {
                    Filler::Net(name) if supply => {
                        p.supply_nets.insert(name.clone());
                    }
                    Filler::Net(name) => {
                        p.ground_nets.insert(name.clone());
                    }
                    // drawn right on a terminal (12.1): the terminal is on the supply or the
                    // ground, and so is its net when the problem shows it ([`power_pins`])
                    Filler::Terminal { object, pin: Some(pin), .. } if supply => {
                        p.supply_pins.insert((*object, *pin));
                    }
                    Filler::Terminal { object, pin: Some(pin), .. } => {
                        p.ground_pins.insert((*object, *pin));
                    }
                    _ => {}
                }
            }
        }
        // a stub: the usage this one is made in hangs it from a terminal
        let Some(parent) = usage.parent else { continue };
        let stub = &problem.usages[parent];
        let rises = match stub.definition.as_str() {
            "$SUPPLY_STUB" if supply => true,
            "$GND_STUB" if !supply => false,
            _ => continue,
        };
        let host = stub.roles.iter().filter(|r| r.role == "t").flat_map(|r| &r.members).find_map(|m| match m {
            Filler::Terminal { object, pin: Some(pin), .. } => Some((*object, *pin)),
            _ => None,
        });
        let (Some(k), Some(host)) = (usage.object, host) else { continue };
        let gap = param(problem, parent, "gap").unwrap_or_else(|| int(2));
        if let Some(o) = objects.get_mut(k) {
            o.stub = Some(Stub { host, rise: if rises { gap } else { -gap } });
        }
        if rises {
            p.supply_pins.insert(host);
        } else {
            p.ground_pins.insert(host);
        }
    }
    p.supply.sort_unstable();
    p.ground.sort_unstable();
    p
}

/// The supply and ground nets marked: those the usages attach a supply or ground symbol to, and
/// those of the terminals one is drawn on or a stub hangs from; then the pins on those nets added
/// to the supply and ground pins.
fn power_pins(mut p: Power, c: &mut Connections) -> Power {
    for net in &c.nets {
        if net.pins.iter().any(|q| p.supply_pins.contains(q)) {
            p.supply_nets.insert(net.name.clone());
        }
        if net.pins.iter().any(|q| p.ground_pins.contains(q)) {
            p.ground_nets.insert(net.name.clone());
        }
    }
    for net in &mut c.nets {
        net.supply = p.supply_nets.contains(&net.name);
        net.ground = p.ground_nets.contains(&net.name);
        if net.supply {
            p.supply_pins.extend(net.pins.iter().copied());
        }
        if net.ground {
            p.ground_pins.extend(net.pins.iter().copied());
        }
    }
    p
}

/// The connection graph; the nets are marked supply or ground by [`power_pins`].
fn connections(problem: &LevelProblem, model: &Model) -> Connections {
    let mut nets: BTreeMap<String, BTreeSet<PinKey>> = BTreeMap::new();
    let pin_of = |r: &PointRef| match r {
        PointRef::Pin { object, pin, .. } => Some((*object, *pin)),
        _ => None,
    };
    let mut drawn: Vec<Drawn> = Vec::new();
    let mut targets: BTreeMap<PinKey, BTreeSet<PinKey>> = BTreeMap::new();
    // a net asked for twice, as by a ROUTE that flows down and one of the level's own, is drawn
    // once, with the pins and points of each, in order, none twice
    let mut routed: Vec<(String, Vec<PinKey>)> = Vec::new();
    for (i, route) in problem.routes.iter().enumerate() {
        let pins: Vec<PinKey> = route.pins.iter().filter_map(pin_of).collect();
        nets.entry(route.net.clone()).or_default().extend(pins.iter().copied());
        let points: Vec<PointId> =
            model.route_points.get(i).map_or(Vec::new(), |ps| ps.iter().flatten().copied().collect());
        let at = match routed.iter().position(|(net, _)| *net == route.net) {
            Some(at) => at,
            None => {
                routed.push((route.net.clone(), Vec::new()));
                drawn.push(Drawn { net: route.net.clone(), points: Vec::new(), routed: true });
                routed.len() - 1
            }
        };
        for p in pins {
            if !routed[at].1.contains(&p) {
                routed[at].1.push(p);
            }
        }
        for p in points {
            if !drawn[at].points.contains(&p) {
                drawn[at].points.push(p);
            }
        }
    }
    let mut route_of: BTreeMap<PinKey, usize> = BTreeMap::new();
    for (i, (_, pins)) in routed.iter().enumerate() {
        for a in pins {
            route_of.insert(*a, i);
            for b in pins {
                if a.0 != b.0 {
                    targets.entry(*a).or_default().insert(*b);
                }
            }
        }
    }
    let mut written: BTreeMap<(PinKey, PinKey), Vec<String>> = BTreeMap::new();
    for (i, wire) in problem.wires.iter().enumerate() {
        let pins: Vec<PinKey> = wire.anchors.iter().filter_map(pin_of).collect();
        if let [net] = wire.nets.as_slice() {
            nets.entry(net.clone()).or_default().extend(pins.iter().copied());
        }
        for w in wire.anchors.windows(2) {
            if let (Some(a), Some(b)) = (pin_of(&w[0]), pin_of(&w[1])) {
                if a.0 != b.0 {
                    targets.entry(a).or_default().insert(b);
                    targets.entry(b).or_default().insert(a);
                    written.entry((a, b)).or_default().push(wire.mode.clone());
                }
            }
        }
        let points: Vec<PointId> =
            model.wire_points.get(i).map_or(Vec::new(), |ps| ps.iter().flatten().copied().collect());
        drawn.push(Drawn { net: wire.nets.join(","), points, routed: false });
    }
    // the symbols that usages attach to a net are on it (12.1)
    for usage in &problem.usages {
        let Some(k) = usage.object else { continue };
        for role in usage.roles.iter().filter(|r| r.role == "p") {
            for m in &role.members {
                if let Filler::Net(name) = m {
                    let has_pin = problem.objects.get(k).and_then(|o| o.symbol).and_then(|s| problem.symbols.get(s));
                    if has_pin.is_some_and(|s| !s.pins.is_empty()) {
                        nets.entry(name.clone()).or_default().insert((k, 0));
                    }
                }
            }
        }
    }
    // the routes are the first of `drawn`, one per net, in the order of `routed`
    let mut out = Connections { drawn, route_of, written, ..Connections::default() };
    for (name, pins) in nets {
        let i = out.nets.len();
        for p in &pins {
            out.nets_of.entry(*p).or_default().push(i);
        }
        out.nets.push(Net { supply: false, ground: false, name, pins: pins.into_iter().collect() });
    }
    out.targets = targets.into_iter().map(|(k, v)| (k, v.into_iter().collect())).collect();
    out
}

/// The ports and their kinds.
fn ports(problem: &LevelProblem, objects: &[ObjectInfo], symbols: &[SymbolInfo], c: &Connections) -> Vec<Port> {
    let mut out = Vec::new();
    for (k, info) in objects.iter().enumerate() {
        if info.role != Role::Port {
            continue;
        }
        let usage = problem.objects[k].usage.and_then(|u| problem.usages.get(u));
        let net = usage.and_then(|u| {
            u.roles.iter().filter(|r| r.role == "p").flat_map(|r| &r.members).find_map(|m| match m {
                Filler::Net(n) => Some(n.clone()),
                _ => None,
            })
        });
        let mut control = false;
        let mut conducting = false;
        let mut drain = false;
        if let Some(n) = net.as_ref().and_then(|n| c.nets.iter().find(|x| x.name == *n)) {
            for &(o, pin) in &n.pins {
                let oi = &objects[o];
                if oi.role != Role::Element || oi.device == Device::Other {
                    continue;
                }
                let terminals =
                    oi.symbol.and_then(|s| symbols.get(s)).and_then(|s| s.pins.get(pin)).map(|p| &p.terminals);
                let has = |t: &str| terminals.is_some_and(|ts| ts.iter().any(|x| x == t));
                control |= has("2");
                conducting |= has("1") || has("3");
                drain |= has("1");
            }
        }
        let kind = if drain {
            PortKind::Output
        } else if control && !conducting {
            PortKind::Input
        } else {
            PortKind::Unknown
        };
        out.push(Port { object: k, net, kind });
    }
    out
}

/// The mirror pairs of the semantic and layout usages.
fn pairs(problem: &LevelProblem, objects: &[ObjectInfo]) -> Vec<Pair> {
    let mut out = BTreeSet::new();
    for usage in &problem.usages {
        let roles: (&str, &str, Option<&str>) = match usage.definition.as_str() {
            "$MIRROR_PAIR" => ("a", "b", Some("axis")),
            "$DIFFPAIR" | "$DIFFPAIR_TAIL" => ("in_p", "in_n", None),
            "$CURRENT_MIRROR" => ("ref", "out", None),
            _ => continue,
        };
        let one = |role: &str| -> Option<usize> {
            let fill = usage.roles.iter().find(|r| r.role == role)?;
            match fill.members.as_slice() {
                [Filler::Object(k)] => Some(*k),
                _ => None,
            }
        };
        let (Some(a), Some(b)) = (one(roles.0), one(roles.1)) else { continue };
        if a == b || objects.get(a).is_none_or(|o| !o.oriented) || objects.get(b).is_none_or(|o| !o.oriented) {
            continue;
        }
        let axis = roles.2.and_then(one).filter(|&k| objects.get(k).is_some_and(|o| o.own));
        out.insert(Pair { a, b, axis });
    }
    // a pair given twice, once with its axis and once without, counts once, with its axis
    let all: Vec<Pair> = out.into_iter().collect();
    all.iter()
        .filter(|p| p.axis.is_some() || !all.iter().any(|q| q.a == p.a && q.b == p.b && q.axis.is_some()))
        .cloned()
        .collect()
}

/// The free orientation classes and the open literals of the facts kept.
fn open(problem: &LevelProblem, resolved: &Resolved) -> Open {
    let classes = &resolved.witness.classes;
    let mut by_root: BTreeMap<usize, Vec<(usize, Orient)>> = BTreeMap::new();
    for (k, o) in problem.objects.iter().enumerate() {
        if o.orient.is_none() {
            continue;
        }
        let (root, label) = classes.find(k);
        if classes.fixed(root).is_none() {
            by_root.entry(root).or_default().push((k, label));
        }
    }
    let mut out = Open {
        classes: by_root.into_iter().map(|(root, members)| FreeClass { root, members }).collect(),
        ..Open::default()
    };
    for &f in &resolved.kept {
        for c in &resolved.model.units[f].choices {
            match c.key {
                crate::model::LitKey::Order(_) => out.orders.push(f),
                crate::model::LitKey::Side(_) => out.sides.push(f),
                _ => {}
            }
        }
    }
    out.orders.dedup();
    out.sides.dedup();
    out
}

/// The tree of groups over the elements (algorithm synthesis 3.5). Every set of elements is one
/// block, made once: a group written again, or the same set as a part or the union of two groups
/// that overlap, is that block.
fn group_tree(problem: &LevelProblem, objects: &[ObjectInfo], elements: &[usize]) -> (GroupTree, Vec<String>) {
    let mut notes = Vec::new();
    let elements_of = |g: usize| -> BTreeSet<usize> {
        problem.groups[g]
            .members
            .iter()
            .copied()
            .filter(|&k| objects.get(k).is_some_and(|o| o.role == Role::Element))
            .collect()
    };
    // blocks: (members, kind), no two with the same members
    let mut blocks: Vec<(BTreeSet<usize>, NodeKind)> = Vec::new();
    // the two groups of each union, which are no blocks themselves: a later group must nest with
    // them as with the blocks, so that both stay in one piece
    let mut absorbed: Vec<BTreeSet<usize>> = Vec::new();
    let laminar = |a: &BTreeSet<usize>, b: &BTreeSet<usize>| a.is_disjoint(b) || a.is_subset(b) || b.is_subset(a);
    for g in 0..problem.groups.len() {
        let set = elements_of(g);
        if set.len() < 2 || blocks.iter().any(|(b, _)| *b == set) || absorbed.contains(&set) {
            continue;
        }
        let crossing: Vec<usize> = (0..blocks.len()).filter(|&i| !laminar(&blocks[i].0, &set)).collect();
        let nests_absorbed = absorbed.iter().all(|a| laminar(a, &set));
        if crossing.is_empty() && nests_absorbed {
            blocks.push((set, NodeKind::Group(g)));
            continue;
        }
        // one group it overlaps: the union, with three parts, when that nests with the rest
        if let ([i], true) = (crossing.as_slice(), nests_absorbed) {
            if let NodeKind::Group(h) = blocks[*i].1 {
                let other = blocks[*i].0.clone();
                let parts = [
                    (other.difference(&set).copied().collect::<BTreeSet<usize>>(), 0u8),
                    (other.intersection(&set).copied().collect(), 1),
                    (set.difference(&other).copied().collect(), 2),
                ];
                let union: BTreeSet<usize> = other.union(&set).copied().collect();
                let rest: Vec<&BTreeSet<usize>> =
                    blocks.iter().enumerate().filter(|(j, _)| j != i).map(|(_, b)| &b.0).collect();
                let nests = std::iter::once(&union)
                    .chain(parts.iter().map(|(p, _)| p))
                    .all(|x| rest.iter().all(|b| laminar(x, b)));
                // a block with the members of the union is the union: a group, or a part of
                // another union, not itself a union, whose order it would have to keep too
                let same = blocks.iter().position(|(b, _)| *b == union);
                let reusable = same.is_none_or(|j| !matches!(blocks[j].1, NodeKind::Union(..)));
                if nests && reusable {
                    match same {
                        Some(j) => {
                            blocks[j].1 = NodeKind::Union(h, g);
                            blocks.remove(*i);
                        }
                        None => blocks[*i] = (union, NodeKind::Union(h, g)),
                    }
                    // a part with the members of a block is that block
                    for (p, which) in parts {
                        if !p.is_empty() && !blocks.iter().any(|(b, _)| *b == p) {
                            blocks.push((p, NodeKind::Part(h, g, which)));
                        }
                    }
                    absorbed.push(other);
                    absorbed.push(set);
                    continue;
                }
            }
        }
        notes.push(format!(
            "the group {} overlaps other groups in a way that does not nest: it is measured as a group but not searched as a block",
            problem.groups[g].label
        ));
    }
    let all: BTreeSet<usize> = elements.iter().copied().collect();
    // the root, then the blocks from the largest down, so that each finds its parent before it
    let mut order: Vec<usize> = (0..blocks.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(blocks[i].0.len()), i));
    let mut nodes = vec![GroupNode {
        kind: NodeKind::Root,
        members: all.iter().copied().collect(),
        children: Vec::new(),
        own: Vec::new(),
        fixed_order: false,
    }];
    let mut sets: Vec<BTreeSet<usize>> = vec![all];
    for i in order {
        let (set, kind) = &blocks[i];
        // the smallest node made so far that holds it: the last, since they come largest first
        let parent = (0..sets.len()).rev().find(|&n| set.is_subset(&sets[n]) && *set != sets[n]).unwrap_or(0);
        let n = nodes.len();
        nodes.push(GroupNode {
            kind: kind.clone(),
            members: set.iter().copied().collect(),
            children: Vec::new(),
            own: Vec::new(),
            fixed_order: matches!(kind, NodeKind::Union(..)),
        });
        sets.push(set.clone());
        nodes[parent].children.push(n);
    }
    // a union's parts in their order, by where their members are, as a part may be a block made
    // before; other children by their first element
    for n in 0..nodes.len() {
        let mut children = std::mem::take(&mut nodes[n].children);
        let groups = match nodes[n].kind {
            NodeKind::Union(h, g) => Some((elements_of(h), elements_of(g))),
            _ => None,
        };
        children.sort_by_key(|&c| {
            let first = nodes[c].members[0];
            match &groups {
                Some((h, g)) => (u8::from(g.contains(&first)) + u8::from(!h.contains(&first)), first),
                None => (0, first),
            }
        });
        let inner: BTreeSet<usize> = children.iter().flat_map(|&c| nodes[c].members.iter().copied()).collect();
        nodes[n].own = nodes[n].members.iter().copied().filter(|k| !inner.contains(k)).collect();
        nodes[n].children = children;
    }
    debug_assert!(
        nodes.iter().all(|n| {
            let mut m: Vec<usize> = n.children.iter().flat_map(|&c| nodes[c].members.iter().copied()).collect();
            m.extend(n.own.iter().copied());
            m.sort_unstable();
            m == n.members
        }),
        "the children of a node and its own elements are its members, each once: {nodes:?}"
    );
    (GroupTree { nodes }, notes)
}

/// The components of each scope of the objectives measured within groups: each group with the
/// stubs that hang from its members, then the components in no group.
fn scopes(problem: &LevelProblem, objects: &[ObjectInfo]) -> Vec<Vec<usize>> {
    let component = |k: usize| {
        let o = &objects[k];
        matches!(o.role, Role::Element | Role::Supply | Role::Ground | Role::Port) && o.symbol.is_some()
    };
    let owner = |k: usize| objects[k].stub.as_ref().map_or(k, |s| s.host.0);
    let mut out = Vec::new();
    let mut grouped = BTreeSet::new();
    for g in &problem.groups {
        let members: BTreeSet<usize> = g.members.iter().copied().collect();
        let scope: Vec<usize> = (0..objects.len()).filter(|&k| component(k) && members.contains(&owner(k))).collect();
        grouped.extend(scope.iter().copied());
        if !scope.is_empty() {
            out.push(scope);
        }
    }
    let rest: Vec<usize> = (0..objects.len()).filter(|&k| component(k) && !grouped.contains(&k)).collect();
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

/// The sources the objectives in force call for, in the order of their layers, then those always
/// in force.
fn sources(active: &Active) -> Vec<Source> {
    let mut out = Vec::new();
    for o in active.in_order() {
        if !o.entry().structural {
            continue;
        }
        let add: &[Source] = match o {
            Objective::CurrentDirection => &[Source::DepthRows, Source::SourcesDown],
            Objective::GroundDirection => &[Source::GroundsDown],
            Objective::SignalFlow => &[Source::InputsLeft],
            Objective::ComponentAlignment => &[Source::Barycentre, Source::Columns],
            Objective::Crossings | Objective::WireLength => &[Source::Barycentre],
            Objective::Distribution => &[Source::EvenRows],
            Objective::Symmetry => &[Source::MirrorPairs],
            _ => &[],
        };
        for s in add {
            if !out.contains(s) {
                out.push(*s);
            }
        }
    }
    out.extend([Source::FacingPins, Source::Facts, Source::NetlistOrder]);
    out
}

/// The current depth of each element (see the module); none for the others and those not
/// reached at all.
fn depths(s: &Structure) -> Vec<Option<u32>> {
    let n = s.objects.len();
    let mut depth: Vec<Option<u32>> = vec![None; n];
    let c = &s.connections;
    let is_element = |k: usize| s.objects[k].role == Role::Element;
    let conducts = |(k, pin): PinKey| s.pins(k).get(pin).is_some_and(|p| s.objects[k].device.conducts(&p.terminals));
    let power_net = |i: usize| c.nets[i].supply || c.nets[i].ground;
    let on_supply = |p: PinKey| s.power.supply_pins.contains(&p);
    let pins_of = |k: usize| (0..s.pins(k).len()).map(move |j| (k, j));
    // through the pins the current runs through, from the elements on a supply
    let mut level: Vec<usize> =
        s.elements.iter().copied().filter(|&k| pins_of(k).any(|p| conducts(p) && on_supply(p))).collect();
    for &k in &level {
        depth[k] = Some(0);
    }
    let mut d = 0;
    while !level.is_empty() {
        let mut next = BTreeSet::new();
        for &k in &level {
            for p in pins_of(k).filter(|p| conducts(*p)) {
                for &i in c.nets_of.get(&p).map_or(&[][..], |v| v) {
                    if power_net(i) {
                        continue;
                    }
                    for &q in &c.nets[i].pins {
                        if q.0 != k && is_element(q.0) && depth[q.0].is_none() && conducts(q) {
                            next.insert(q.0);
                        }
                    }
                }
            }
        }
        d += 1;
        for &k in &next {
            depth[k] = Some(d);
        }
        level = next.into_iter().collect();
    }
    // the others through any pin: the depth of what they are connected to
    loop {
        let mut found = Vec::new();
        for &k in &s.elements {
            if depth[k].is_some() {
                continue;
            }
            let mut best: Option<(u32, usize)> = None;
            for p in pins_of(k) {
                for &i in c.nets_of.get(&p).map_or(&[][..], |v| v) {
                    if power_net(i) {
                        continue;
                    }
                    for &q in &c.nets[i].pins {
                        if let Some(dq) = depth[q.0].filter(|_| is_element(q.0) && q.0 != k) {
                            if best.is_none_or(|b| (dq, q.0) < b) {
                                best = Some((dq, q.0));
                            }
                        }
                    }
                }
            }
            if let Some((dq, _)) = best {
                found.push((k, dq));
            }
        }
        if found.is_empty() {
            break;
        }
        for (k, dq) in found {
            depth[k] = Some(dq);
        }
    }
    depth
}

/// Classes of elements that share a row, a union-find: each class by its smallest element.
struct Joined {
    parent: BTreeMap<usize, usize>,
}

impl Joined {
    fn new(elements: &[usize]) -> Joined {
        Joined { parent: elements.iter().map(|&k| (k, k)).collect() }
    }

    fn find(&self, mut k: usize) -> usize {
        while self.parent[&k] != k {
            k = self.parent[&k];
        }
        k
    }

    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            let (lo, hi) = if a < b { (a, b) } else { (b, a) };
            self.parent.insert(hi, lo);
        }
    }
}

/// The rows the elements start in (see the module).
fn rows(problem: &LevelProblem, s: &Structure, work: &mut Work) -> Vec<Vec<usize>> {
    if s.elements.is_empty() {
        return Vec::new();
    }
    let kind = |f: usize| &problem.facts[f].kind;
    // an ALIGN on y kept joins its elements in one row
    let mut joined = Joined::new(&s.elements);
    for &f in &s.kept {
        if matches!(kind(f), FactKind::Align { axis: Axis::Y, .. }) {
            for w in touched_elements(s, f, 1).windows(2) {
                joined.join(w[0], w[1]);
            }
        }
    }
    if s.depth.is_none() {
        // without the current depth, an ORDER or a DISTRIBUTE on x is a row too, as far as the
        // facts kept let its elements share a height: each pair of neighbours at one height is a
        // relation of the solver's own taken in after the facts (8.5), and the pairs that fit join
        let origin = |k: usize| s.model.origin_point.get(k).copied().flatten();
        let mut pairs: Vec<(usize, usize, PointId, PointId)> = Vec::new();
        for &f in &s.kept {
            if matches!(kind(f), FactKind::Order { axis: Axis::X, .. } | FactKind::Distribute { axis: Axis::X, .. }) {
                let elements = touched_elements(s, f, 0);
                pairs.extend(elements.windows(2).filter_map(|w| Some((w[0], w[1], origin(w[0])?, origin(w[1])?))));
            }
        }
        if !pairs.is_empty() {
            let rels: Vec<Tagged> = pairs
                .iter()
                .map(|&(_, _, a, b)| Tagged {
                    kind: RelKind::Row,
                    rel: Relation::Linear {
                        axis: 1,
                        terms: vec![(b, Q::one()), (a, -Q::one())],
                        lo: Some(Q::zero()),
                        hi: Some(Q::zero()),
                    },
                })
                .collect();
            let fits = with_relations(s, &rels, work, |_, _, t| t.kept.clone());
            for (&(a, b, _, _), fit) in pairs.iter().zip(fits) {
                if fit {
                    joined.join(a, b);
                }
            }
        }
    }
    // the height of each class's row, by its smallest element: larger keys higher
    let mut key: BTreeMap<usize, Q> = BTreeMap::new();
    match &s.depth {
        Some(depth) => {
            let last = s.elements.iter().filter_map(|&k| depth[k]).max().map_or(0, |d| d + 1);
            for &k in &s.elements {
                if joined.find(k) == k {
                    key.insert(k, -int(i64::from(depth[k].unwrap_or(last))));
                }
            }
        }
        None => {
            // the heights of the elements that a fact kept on y relates to another element, or
            // places, the first of a class giving the class its height; other facts that reach
            // one element only, such as those of its stubs, say nothing of rows
            let mut related = BTreeSet::new();
            for &f in &s.kept {
                let joins = touched_elements(s, f, 1);
                let placed = matches!(
                    kind(f),
                    FactKind::Place { axis: Axis::Y, .. } | FactKind::PointCoordinate { axis: Axis::Y, .. }
                );
                if joins.len() >= 2 || placed {
                    related.extend(joins);
                }
            }
            for &k in &related {
                key.entry(joined.find(k)).or_insert_with(|| s.witness.origin[k][1].clone());
            }
            spread(s, &joined, &mut key);
        }
    }
    // an ALIGN on x kept is a column: the classes it reaches go in rows of their own, one under
    // the other, the highest at the witness on top
    let mut apart: BTreeSet<(usize, usize)> = BTreeSet::new();
    for &f in &s.kept {
        if matches!(kind(f), FactKind::Align { axis: Axis::X, .. }) {
            let roots: BTreeSet<usize> = touched_elements(s, f, 0).into_iter().map(|k| joined.find(k)).collect();
            for &a in &roots {
                apart.extend(roots.iter().filter(|&&b| a < b).map(|&b| (a, b)));
            }
        }
    }
    let mut levels: Vec<Q> = key.values().cloned().collect::<BTreeSet<Q>>().into_iter().collect();
    levels.reverse();
    let mut rows = Vec::new();
    for level in levels {
        let mut classes: Vec<usize> = key.iter().filter(|(_, q)| **q == level).map(|(r, _)| *r).collect();
        classes.sort_by(|a, b| s.witness.origin[*b][1].cmp(&s.witness.origin[*a][1]).then(a.cmp(b)));
        // each class in the first row of its level that holds no class of a column with it
        let mut sub: BTreeMap<usize, usize> = BTreeMap::new();
        for &r in &classes {
            let taken: BTreeSet<usize> =
                sub.iter().filter(|(q, _)| apart.contains(&((**q).min(r), (**q).max(r)))).map(|(_, c)| *c).collect();
            let c = (0..classes.len()).find(|c| !taken.contains(c)).unwrap_or(0);
            sub.insert(r, c);
        }
        let count = sub.values().max().map_or(0, |c| c + 1);
        for c in 0..count {
            let row: Vec<usize> =
                s.elements.iter().copied().filter(|&k| sub.get(&joined.find(k)) == Some(&c)).collect();
            if !row.is_empty() {
                rows.push(row);
            }
        }
    }
    rows
}

/// The elements that share a net with an element, supply and ground nets left out, sorted.
fn neighbours(s: &Structure, k: usize) -> Vec<usize> {
    let c = &s.connections;
    let mut out = BTreeSet::new();
    for pin in 0..s.pins(k).len() {
        for &i in c.nets_of.get(&(k, pin)).map_or(&[][..], |v| v) {
            if c.nets[i].supply || c.nets[i].ground {
                continue;
            }
            out.extend(c.nets[i].pins.iter().map(|q| q.0).filter(|&j| j != k && s.objects[j].role == Role::Element));
        }
    }
    out.into_iter().collect()
}

/// The row keys of the classes without one: the key of the first element their elements are
/// connected to whose class has one, round after round; then, for those connected to none, the key
/// that most elements have, the highest of equals; 0 when no element has one.
fn spread(s: &Structure, joined: &Joined, key: &mut BTreeMap<usize, Q>) {
    let roots: Vec<usize> = s.elements.iter().copied().filter(|&k| joined.find(k) == k).collect();
    loop {
        let found: Vec<(usize, Q)> = roots
            .iter()
            .filter(|r| !key.contains_key(r))
            .filter_map(|&r| {
                s.elements
                    .iter()
                    .filter(|&&k| joined.find(k) == r)
                    .flat_map(|&k| neighbours(s, k))
                    .find_map(|j| key.get(&joined.find(j)).cloned())
                    .map(|q| (r, q))
            })
            .collect();
        if found.is_empty() {
            break;
        }
        key.extend(found);
    }
    let mut counts: BTreeMap<Q, usize> = BTreeMap::new();
    for &k in &s.elements {
        if let Some(q) = key.get(&joined.find(k)) {
            *counts.entry(q.clone()).or_default() += 1;
        }
    }
    let common = counts.iter().max_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0))).map(|(q, _)| q.clone());
    let common = common.unwrap_or_default();
    for r in roots {
        key.entry(r).or_insert_with(|| common.clone());
    }
}

/// The elements whose points a fact kept reaches on an axis, sorted.
fn touched_elements(s: &Structure, f: usize, axis: usize) -> Vec<usize> {
    let mut out = BTreeSet::new();
    for p in &s.model.units[f].touched[axis] {
        let k = match p {
            PointId::Origin(k) => Some(*k),
            PointId::Pin(k, _) => Some(*k),
            _ => None,
        };
        if let Some(k) = k.filter(|&k| s.objects.get(k).is_some_and(|o| o.role == Role::Element)) {
            out.insert(k);
        }
    }
    out.into_iter().collect()
}

/// The left-to-right precedences of the facts kept on x ([`Structure::precedence`],
/// [`Reversible`]), as the witness orders their elements: each member at the witness's x of the
/// point the fact takes it by, the elements of a set (a centre, 6.5) all at the centre's, and an
/// element before another when it is further left. An ORDER puts every member before the next; a
/// DISTANCE puts one side before the other only when its gap is not below 0, and leaves the
/// members of one side unordered; an ALIGN and a DISTRIBUTE fix no side (14.2, 14.4).
fn precedence(problem: &LevelProblem, s: &Structure) -> (BTreeSet<(usize, usize)>, Vec<Reversible>) {
    let element = |k: usize| s.objects.get(k).is_some_and(|o| o.role == Role::Element);
    let at = |ms: &[PointRef]| -> Vec<(usize, Q)> {
        let mut out = Vec::new();
        for m in ms {
            let Some(v) = point_value(problem, &s.witness, m, 0, 0) else { continue };
            let mut ks = Vec::new();
            elements_of(m, &mut ks);
            out.extend(ks.into_iter().filter(|&k| element(k)).map(|k| (k, v.clone())));
        }
        out
    };
    let before = |a: &[(usize, Q)], b: &[(usize, Q)]| -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (x, vx) in a {
            out.extend(b.iter().filter(|(y, vy)| x != y && vx < vy).map(|(y, _)| (*x, *y)));
        }
        out
    };
    let mut fixed = BTreeSet::new();
    let mut open = Vec::new();
    for &f in &s.kept {
        let (sense, mut pairs) = match &problem.facts[f].kind {
            FactKind::Order { axis: Axis::X, sense, members } => {
                let at = at(members);
                (*sense, before(&at, &at))
            }
            FactKind::Distance { axis: Axis::X, sense, min, from, to, .. } => {
                if !min.and_then(from_f64).is_some_and(|m| !m.is_negative()) {
                    continue;
                }
                let (a, b) = (at(from), at(to));
                let mut pairs = before(&a, &b);
                pairs.extend(before(&b, &a));
                (*sense, pairs)
            }
            _ => continue,
        };
        pairs.sort_unstable();
        pairs.dedup();
        match sense {
            Sense::Either if s.open.orders.contains(&f) || s.open.sides.contains(&f) => {
                if !pairs.is_empty() {
                    open.push(Reversible { fact: f, pairs });
                }
            }
            Sense::Either => {}
            Sense::Increasing | Sense::Decreasing => fixed.extend(pairs),
        }
    }
    (fixed, open)
}

/// The objects a point refers to: its object, or the members of a set followed down.
fn elements_of(r: &PointRef, out: &mut Vec<usize>) {
    match r {
        PointRef::Origin(k) => out.push(*k),
        PointRef::Pin { object, .. } => out.push(*object),
        PointRef::Centre(members) => {
            for m in members {
                elements_of(m, out);
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::resolve::tests::{fact, instance, problem_of};
    use rnl_runtime::problem::ProblemGroup;

    fn structure_of(p: &LevelProblem) -> Structure {
        let options = SolverOptions::without_time_limit();
        let r = crate::resolve(p, &options);
        analyse(p, &r, &options, crate::options::CAPS, &mut Work::default())
    }

    #[test]
    fn groups_nest_and_overlapping_groups_split_in_three() {
        let mut p = problem_of((0..6).map(|_| instance()).collect(), Vec::new());
        let group = |label: &str, members: Vec<usize>| ProblemGroup { label: label.to_string(), members };
        p.groups =
            vec![group("%a", vec![0, 1, 2]), group("%b", vec![2, 3]), group("%c", vec![0, 1]), group("%d", vec![4, 5])];
        let s = structure_of(&p);
        let nodes = &s.tree.nodes;
        assert_eq!(nodes[0].kind, NodeKind::Root);
        assert_eq!(nodes[0].members, vec![0, 1, 2, 3, 4, 5]);
        // %a and %b overlap: their union, with %a alone, the shared #2, %b alone; %c nests in the
        // first part, %d stands apart
        let union = nodes.iter().position(|n| n.kind == NodeKind::Union(0, 1)).unwrap();
        assert_eq!(nodes[union].members, vec![0, 1, 2, 3]);
        assert!(nodes[union].fixed_order);
        let parts: Vec<&Vec<usize>> = nodes[union].children.iter().map(|&c| &nodes[c].members).collect();
        assert_eq!(parts, vec![&vec![0, 1], &vec![2], &vec![3]]);
        // %c is the same set as the first part: it is not made twice
        assert_eq!(nodes.len(), 6, "{nodes:?}");
        let d = nodes.iter().position(|n| n.kind == NodeKind::Group(3)).unwrap();
        assert_eq!(nodes[0].children, vec![union, d]);
        assert!(nodes[0].own.is_empty());
        let order = s.tree.post_order();
        assert_eq!(*order.last().unwrap(), 0);
        assert!(
            order.iter().position(|&n| n == union)
                > nodes[union].children.iter().map(|&c| order.iter().position(|&n| n == c)).max().unwrap()
        );
        assert_eq!(s.tree.lowest_common(&[0, 1]), nodes[union].children[0]);
        assert_eq!(s.tree.lowest_common(&[0, 4]), 0);
        // the scopes: each group, nothing outside
        assert_eq!(s.scopes.len(), 4);
    }

    #[test]
    fn a_group_that_overlaps_two_others_is_not_nested_and_said_so() {
        let mut p = problem_of((0..5).map(|_| instance()).collect(), Vec::new());
        let group = |label: &str, members: Vec<usize>| ProblemGroup { label: label.to_string(), members };
        p.groups = vec![group("%a", vec![0, 1]), group("%b", vec![2, 3]), group("%c", vec![1, 2])];
        let s = structure_of(&p);
        assert_eq!(s.tree.nodes.len(), 3);
        assert!(s.notes[0].contains("%c"), "{:?}", s.notes);
    }

    /// The tree of groups over `n` free instances with these groups, `%g0`, `%g1`, …, and the notes.
    fn tree_of(n: usize, groups: &[Vec<usize>]) -> (GroupTree, Vec<String>) {
        let mut p = problem_of((0..n).map(|_| instance()).collect(), Vec::new());
        p.groups = groups
            .iter()
            .enumerate()
            .map(|(i, m)| ProblemGroup { label: format!("%g{i}"), members: m.clone() })
            .collect();
        let s = structure_of(&p);
        (s.tree, s.notes)
    }

    #[test]
    fn a_part_or_a_union_with_the_members_of_a_group_is_that_group() {
        // the first part of the union of %g1 and %g2 is %g0
        let (t, notes) = tree_of(4, &[vec![0, 1], vec![0, 1, 2], vec![2, 3]]);
        let nodes = &t.nodes;
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(nodes.len(), 5, "{nodes:?}");
        let union = nodes.iter().position(|n| n.kind == NodeKind::Union(1, 2)).unwrap();
        assert_eq!(nodes[0].children, vec![union]);
        let parts: Vec<(&NodeKind, &Vec<usize>)> =
            nodes[union].children.iter().map(|&c| (&nodes[c].kind, &nodes[c].members)).collect();
        assert_eq!(
            parts,
            vec![
                (&NodeKind::Group(0), &vec![0, 1]),
                (&NodeKind::Part(1, 2, 1), &vec![2]),
                (&NodeKind::Part(1, 2, 2), &vec![3])
            ]
        );
        // the union of %g2 and %g3 is %g1, inside %g0: %g1 is the union, its parts what only %g2
        // has, what both have, what only %g3 has
        let (t, notes) = tree_of(5, &[vec![0, 1, 2, 3, 4], vec![0, 1, 2, 3], vec![2, 3], vec![0, 1, 2]]);
        let nodes = &t.nodes;
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(nodes.len(), 6, "{nodes:?}");
        let top = nodes.iter().position(|n| n.kind == NodeKind::Group(0)).unwrap();
        let union = nodes.iter().position(|n| n.kind == NodeKind::Union(2, 3)).unwrap();
        assert_eq!((nodes[0].children.clone(), nodes[top].children.clone()), (vec![top], vec![union]));
        assert_eq!(nodes[union].members, vec![0, 1, 2, 3]);
        assert!(nodes[union].fixed_order);
        let parts: Vec<&Vec<usize>> = nodes[union].children.iter().map(|&c| &nodes[c].members).collect();
        assert_eq!(parts, vec![&vec![3], &vec![2], &vec![0, 1]]);
        // a group written after the union that is one of its two groups is in it already; one with
        // the two outer parts and not the shared one cannot be in one piece with them
        let (t, notes) = tree_of(4, &[vec![0, 1, 2], vec![2, 3], vec![0, 1, 2], vec![0, 1, 3]]);
        assert_eq!(t.nodes.len(), 5, "{:?}", t.nodes);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("%g3"), "{notes:?}");
    }

    /// A fixed sequence of pseudo-random numbers below `n`, for the tests.
    pub(crate) struct Lcg(pub u64);

    impl Lcg {
        pub(crate) fn below(&mut self, n: u64) -> u64 {
            self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 33) % n
        }

        /// Sets of groups over `n` elements: two to five, each element in each by a coin.
        pub(crate) fn groups(&mut self, n: usize) -> Vec<Vec<usize>> {
            let count = 2 + self.below(4) as usize;
            (0..count).map(|_| (0..n).filter(|_| self.below(2) == 0).collect()).collect()
        }
    }

    /// Groups drawn at random: the nodes nest, each set of elements is one node, the children of a
    /// node and its own elements are its members, each once, a union's three parts are what only
    /// its first group has, what both have and what only the second has, and every group is a
    /// node, one of the two groups of a union, or noted.
    #[test]
    fn every_tree_of_groups_holds_each_set_once_and_splits_a_union_in_three() {
        let mut rng = Lcg(0x2545_f491_4f6c_dd1d);
        let mut unions = 0;
        for _ in 0..500 {
            let groups = rng.groups(6);
            let (t, notes) = tree_of(6, &groups);
            let nodes = &t.nodes;
            let what = format!("{groups:?}: {nodes:?}");
            let set = |n: usize| nodes[n].members.iter().copied().collect::<BTreeSet<usize>>();
            for a in 0..nodes.len() {
                for b in a + 1..nodes.len() {
                    let (x, y) = (set(a), set(b));
                    assert!(x.is_disjoint(&y) || x.is_subset(&y) || y.is_subset(&x), "{what}");
                    assert!(a == 0 || x != y, "{what}");
                }
                let mut m: Vec<usize> = nodes[a].children.iter().flat_map(|&c| nodes[c].members.clone()).collect();
                m.extend(nodes[a].own.iter().copied());
                m.sort_unstable();
                assert_eq!(m, nodes[a].members, "{what}");
                assert!(nodes[a].children.iter().all(|&c| a == 0 || nodes[c].members.len() < nodes[a].members.len()));
                if let NodeKind::Union(h, g) = nodes[a].kind {
                    unions += 1;
                    let (h, g): (BTreeSet<usize>, BTreeSet<usize>) =
                        (groups[h].iter().copied().collect(), groups[g].iter().copied().collect());
                    let parts: Vec<BTreeSet<usize>> = nodes[a].children.iter().map(|&c| set(c)).collect();
                    let expected: Vec<BTreeSet<usize>> = vec![
                        h.difference(&g).copied().collect(),
                        h.intersection(&g).copied().collect(),
                        g.difference(&h).copied().collect(),
                    ];
                    assert_eq!(parts, expected, "{what}");
                    assert!(nodes[a].fixed_order && nodes[a].own.is_empty(), "{what}");
                }
            }
            for (i, g) in groups.iter().enumerate() {
                let g: BTreeSet<usize> = g.iter().copied().collect();
                let in_union = nodes.iter().any(|n| match n.kind {
                    NodeKind::Union(a, b) => {
                        [a, b].iter().any(|&j| groups[j].iter().copied().collect::<BTreeSet<usize>>() == g)
                    }
                    _ => false,
                });
                let noted = notes.iter().any(|n| n.contains(&format!("%g{i} ")));
                assert!(g.len() < 2 || (0..nodes.len()).any(|n| set(n) == g) || in_union || noted, "%g{i}: {what}");
            }
        }
        assert!(unions > 50, "{unions}");
    }

    #[test]
    fn rows_come_from_the_facts_on_heights_without_a_supply() {
        let o = |k| PointRef::Origin(k);
        let p = problem_of(
            vec![instance(), instance(), instance(), instance()],
            vec![
                fact(FactKind::Distance {
                    axis: Axis::Y,
                    sense: rnl_runtime::problem::Sense::Increasing,
                    min: Some(5.0),
                    max: None,
                    from: vec![o(0)],
                    to: vec![o(1)],
                }),
                fact(FactKind::Order {
                    axis: Axis::X,
                    sense: rnl_runtime::problem::Sense::Increasing,
                    members: vec![o(2), o(0)],
                }),
                fact(FactKind::Place { at: o(3), axis: Axis::Y, value: -20.0 }),
            ],
        );
        let s = structure_of(&p);
        assert!(s.depth.is_none());
        // 1 above 0, 3 placed below; 2, which the ORDER joins to 0 and nothing on y keeps from
        // it, in the row of 0
        assert_eq!(s.rows, vec![vec![1], vec![0, 2], vec![3]]);
        assert!(s.precedence.contains(&(2, 0)) && !s.precedence.contains(&(0, 2)));
        // every element is its own free class
        assert_eq!(s.open.classes.len(), 4);
        // no wires, no supply: current_direction and the routing objectives are inert
        assert!(s.active.inert.iter().any(|i| i.starts_with("current_direction")));
        assert!(!s.has(Source::DepthRows) && s.has(Source::Barycentre) && s.has(Source::FacingPins));
    }

    #[test]
    fn an_order_is_a_row_where_the_facts_let_it_be_and_an_align_on_x_a_column() {
        let o = |k| PointRef::Origin(k);
        let order = |members: Vec<PointRef>| {
            fact(FactKind::Order { axis: Axis::X, sense: rnl_runtime::problem::Sense::Increasing, members })
        };
        let p = problem_of(
            (0..6).map(|_| instance()).collect(),
            vec![
                // 1 is kept above 0 by 5, so the ORDER cannot hold them in one row; 2 joins 1
                fact(FactKind::Distance {
                    axis: Axis::Y,
                    sense: rnl_runtime::problem::Sense::Increasing,
                    min: Some(5.0),
                    max: None,
                    from: vec![o(0)],
                    to: vec![o(1)],
                }),
                order(vec![o(0), o(1), o(2)]),
                // 3 and 4 in one column: rows of their own, though nothing else tells them apart
                fact(FactKind::Align { axis: Axis::X, members: vec![o(3), o(4)] }),
            ],
        );
        let s = structure_of(&p);
        let row_of = |k: usize| s.rows.iter().position(|r| r.contains(&k)).unwrap();
        assert_eq!(row_of(1), row_of(2), "{:?}", s.rows);
        assert_ne!(row_of(0), row_of(1), "{:?}", s.rows);
        assert_ne!(row_of(3), row_of(4), "{:?}", s.rows);
        // 5, connected to nothing, goes with the most
        assert_eq!(row_of(5), row_of(3).min(row_of(4)), "{:?}", s.rows);
    }

    #[test]
    fn only_the_facts_with_a_side_give_precedences_and_those_left_open_can_be_reversed() {
        let o = |k| PointRef::Origin(k);
        let distance = |sense, min: Option<f64>, max: Option<f64>, a: usize, b: usize| {
            fact(FactKind::Distance { axis: Axis::X, sense, min, max, from: vec![o(a)], to: vec![o(b)] })
        };
        use rnl_runtime::problem::Sense::{Either, Increasing};
        let p = problem_of(
            (0..8).map(|_| instance()).collect(),
            vec![
                // a side: 0 before 1
                distance(Increasing, Some(2.0), None, 0, 1),
                // no side: within 5 of each other, either way; and a DISTRIBUTE, an ALIGN
                distance(Either, None, Some(5.0), 2, 3),
                fact(FactKind::Distribute { axis: Axis::X, members: vec![o(2), o(3), o(4)] }),
                fact(FactKind::Align { axis: Axis::X, members: vec![o(5), o(6)] }),
                // a side left open: reversible
                fact(FactKind::Order { axis: Axis::X, sense: Either, members: vec![o(6), o(7)] }),
            ],
        );
        let s = structure_of(&p);
        assert_eq!(s.precedence.iter().copied().collect::<Vec<_>>(), vec![(0, 1)]);
        assert_eq!(s.open.orders, vec![4]);
        assert_eq!(s.reversible.len(), 1, "{:?}", s.reversible);
        assert_eq!(s.reversible[0].fact, 4);
        let [(a, b)] = s.reversible[0].pairs[..] else { panic!("{:?}", s.reversible) };
        assert_eq!((a.min(b), a.max(b)), (6, 7));
    }

    #[test]
    fn a_net_routed_twice_is_drawn_once_with_the_pins_of_both() {
        let mut p = problem_of(vec![instance(), instance(), instance()], Vec::new());
        let pin = |object: usize, pin: usize| PointRef::Pin { object, pin, terminal: ["g", "d", "s"][pin].to_string() };
        let route = |pins: Vec<PointRef>| rnl_runtime::problem::RouteIntent {
            net: "n".to_string(),
            mode: "auto".to_string(),
            style: Vec::new(),
            pins,
            wires: Vec::new(),
        };
        p.routes = vec![route(vec![pin(0, 1), pin(1, 1)]), route(vec![pin(1, 1), pin(2, 0)])];
        let s = structure_of(&p);
        let drawn = &s.connections.drawn;
        assert_eq!(drawn.len(), 1, "{drawn:?}");
        assert_eq!(drawn[0].points, vec![PointId::Pin(0, 1), PointId::Pin(1, 1), PointId::Pin(2, 0)]);
        // every pin of the net drawn to the others
        assert_eq!(s.connections.targets[&(0, 1)], vec![(1, 1), (2, 0)]);
        assert_eq!(s.connections.targets[&(2, 0)], vec![(0, 1), (1, 1)]);
    }

    #[test]
    fn the_members_of_a_set_are_ordered_by_its_centre() {
        let o = |k| PointRef::Origin(k);
        let p = problem_of(
            vec![instance(), instance(), instance()],
            vec![fact(FactKind::Order {
                axis: Axis::X,
                sense: rnl_runtime::problem::Sense::Increasing,
                members: vec![o(0), PointRef::Centre(vec![o(1), o(2)])],
            })],
        );
        let s = structure_of(&p);
        assert!(s.precedence.contains(&(0, 1)) && s.precedence.contains(&(0, 2)));
        assert!(!s.precedence.contains(&(1, 2)) && !s.precedence.contains(&(2, 1)));
    }
}
