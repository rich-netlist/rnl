//! Structural proposal sources derived from configured terms and exported language facts.
//! CURRENT hosts retain semantic identity through SCC compression; display mappings are applied
//! only when proposing positions. Attachments and grouping use object relationships, never a
//! built-in definition or symbol vocabulary. Candidate relations remain subordinate to every
//! language fact retained by 8.5.

use std::collections::{BTreeMap, BTreeSet};

use num_traits::{One, Signed, Zero};
use rnl_runtime::geom::Orient;
use rnl_runtime::markup::Run;
use rnl_runtime::problem::{
    Axis, FactKind, Filler, LevelProblem, ObjectKind, PointRef, Position, Reference, Sense, TextInfo,
};

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
    /// A drawn virtual symbol attached to a point, terminal or net.
    Port,
    Point,
    Text,
    /// A virtual part drawn at the point of its main object (12.1) that is none of the above.
    Other,
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
    pub symbol: Option<usize>,
    pub oriented: bool,
    /// It has a position of its own (11.1).
    pub own: bool,
    pub stub: Option<Stub>,
    pub attached_host: Option<usize>,
    /// A fact kept reaches one of its points.
    pub fact_touched: bool,
    /// On each axis, a fact kept reaches one of its points there.
    pub fact_axes: [bool; 2],
    /// The solution leaves its position free ([`left_free`]): it is no part of the drawing that
    /// the solver makes.
    pub left_free: bool,
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Port {
    pub object: usize,
    pub net: Option<String>,
}

/// Where a text goes when no fact decides its position (13.3): "a text attached to a point usually
/// goes at that point, and a text attached to an object next to the object". At the point itself,
/// or beside the coordinate point of the object, the origin of an instance, a usage drawn or a
/// bundle, or the pin of a terminal, by the offset that a viewer that does not solve puts it at.
/// A wire or a net has no coordinate point (13.3), and a text attached to one is anchored where
/// the viewer puts such a text only for the axis that no fact kept reaches, when a fact kept
/// reaches the other: with no fact kept reaching it, the solution leaves it free and the viewer
/// places it ([`left_free`]).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Anchor {
    /// The points it is from, each the origin of an object or a pin of it, by the object and the
    /// pin in the object's symbol: the one it is attached to, or whose terminal; for a wire, the
    /// two in the middle of its points, whose midpoint it is from; for a net, one of its points.
    pub from: Vec<(usize, Option<usize>)>,
    /// From the point, or the midpoint, along the axes of the scene.
    pub offset: [Q; 2],
    /// What the text is attached to has a coordinate point (13.3), which the anchor is: a point,
    /// an object or a terminal; not a wire or a net, whose anchor is only near some of its points.
    pub point: bool,
}

/// Where a viewer that does not solve puts a text attached to an object, from the object's
/// coordinate point (13.3); the solver puts it there too, so that a text looks the same solved.
fn beside() -> [Q; 2] {
    [frac(6, 5), frac(6, 5)]
}

/// The origin of an object or a pin, as a point of an anchor.
fn anchor_point(r: &PointRef) -> Option<(usize, Option<usize>)> {
    match r {
        PointRef::Origin(k) => Some((*k, None)),
        PointRef::Pin { object, pin, .. } => Some((*object, Some(*pin))),
        PointRef::Centre(_) => None,
    }
}

/// The first of some points that is the origin of an object or a pin, as a point of an anchor.
fn first_point(points: &[PointRef]) -> Option<(usize, Option<usize>)> {
    points.iter().find_map(anchor_point)
}

/// The anchors of the texts with a position of their own ([`Structure::anchors`]): those attached
/// to a point, an object with a coordinate point, or a terminal with a pin; and, as the viewer
/// puts a text attached to what has no coordinate point (13.3), those attached to a wire 0.6
/// above the midpoint of the two points in the middle of the wire's, the middle of its middle
/// segment when the wire is drawn straight between them, and those attached to a net at the first
/// point of its first wire, else of its first ROUTE, else at its first pin of an object that the
/// solution places. `objects` say which objects the solution leaves free.
fn anchors(problem: &LevelProblem, connections: &Connections, objects: &[ObjectInfo]) -> BTreeMap<usize, Anchor> {
    let mut out = BTreeMap::new();
    let zero = || [Q::zero(), Q::zero()];
    for (k, o) in problem.objects.iter().enumerate() {
        let (ObjectKind::Text, Position::Own { .. }, Some(t)) = (o.kind, &o.position, &o.text) else { continue };
        let anchor = match &t.attach {
            Filler::Object(j) => match problem.objects.get(*j).map(|a| a.kind) {
                Some(ObjectKind::Point) => Anchor { from: vec![(*j, None)], offset: zero(), point: true },
                Some(ObjectKind::Instance | ObjectKind::Usage | ObjectKind::Bundle) => {
                    Anchor { from: vec![(*j, None)], offset: beside(), point: true }
                }
                _ => continue,
            },
            Filler::Terminal { object, pin: Some(pin), .. } => {
                Anchor { from: vec![(*object, Some(*pin))], offset: beside(), point: true }
            }
            Filler::Wire(w) => {
                let Some(w) = problem.wires.get(*w) else { continue };
                let points: Vec<(usize, Option<usize>)> = w.anchors.iter().filter_map(anchor_point).collect();
                if points.len() < 2 {
                    continue;
                }
                let m = points.len() / 2;
                Anchor { from: points[m - 1..=m].to_vec(), offset: [Q::zero(), frac(3, 5)], point: false }
            }
            Filler::Net(name) => {
                let wire = problem.wires.iter().filter(|w| w.nets.contains(name)).find_map(|w| first_point(&w.anchors));
                let route = || problem.routes.iter().filter(|r| r.net == *name).find_map(|r| first_point(&r.pins));
                let pin = || {
                    let net = connections.nets.iter().find(|n| n.name == *name)?;
                    net.pins.iter().find(|p| !objects[p.0].left_free).map(|&(object, pin)| (object, Some(pin)))
                };
                let Some(point) = wire.or_else(route).or_else(pin) else { continue };
                Anchor { from: vec![point], offset: zero(), point: false }
            }
            _ => continue,
        };
        out.insert(k, anchor);
    }
    out
}

/// Whether the solution leaves the position of object `k` free, so that it is no part of the
/// drawing that the solver makes ([`ObjectInfo::left_free`]): an object with a position of its
/// own that no fact kept reaches, and that the solver has nothing to place by. The LP would leave
/// it where the witness of the resolution happens to be, such as the origin, and the record would
/// fix it there.
///
/// - A text attached to what has no coordinate point, a wire or a net ("Wires and nets have no
///   coordinate point", 13.3), or to anything else that the solver has no point for, such as a
///   terminal without a pin or a graphic, named or not: "When no PLACE, dx, dy or constraint fixes
///   it, the solver and the viewer decide" (13.3). Left free, the viewer places it next to what it
///   is attached to (13.3): a label of a wire above the middle of the wire drawn, one of a net at
///   the start of its first wire or on one of its pins, one of a graphic beside its first point;
///   and it follows them when they move, or is not drawn when nothing of that is. A text attached
///   to a point or to an object with a coordinate point is placed at its [`Anchor`].
/// - A supply, ground or port symbol without a name (5.9), no stub, that has no pin or is on no
///   net with a pin of an element that the problem shows, by a ROUTE or a wire written
///   ([`Connections`]), so that the search has no pin to put it beside (`crate::search`): nothing
///   decides where it goes in the drawing, and drawn where the witness is, beside a wire of
///   another net, it would say that it is on that one. Left free, a viewer leaves it unplaced, as
///   a record of 0.2, which cannot refer to it, did; in the draft 0.3, which could refer to it by
///   its number, it is left free all the same, so that a level is solved the same in both
///   versions (16.1), and a file of 0.2 is not raised to 0.3 for it. One with a name, its own or
///   the one that the record gives a usage made by a rule (10.5), is placed where the witness and
///   the repair of overlaps put it, as it was before files were raised: it is still drawn, beside
///   nothing it is on, rather than not at all, as the ports of a level without an element are.
///
/// A point `(_, _)` that nothing else places is the solver's to place all the same (11.1).
fn left_free(
    problem: &LevelProblem,
    objects: &[ObjectInfo],
    symbols: &[SymbolInfo],
    connections: &Connections,
    anchors: &BTreeMap<usize, Anchor>,
    k: usize,
) -> bool {
    let o = &objects[k];
    if !o.own || o.fact_touched {
        return false;
    }
    match o.role {
        Role::Text => anchors.get(&k).is_none_or(|a| !a.point),
        Role::Port => {
            let unnamed = matches!(
                problem.objects[k].reference,
                Reference::Number { .. } | Reference::GeneratedNumber { .. } | Reference::Anonymous
            );
            let pinless = o.symbol.and_then(|s| symbols.get(s)).is_none_or(|s| s.pins.is_empty());
            let nowhere = pinless || attach_pins(objects, connections, k).is_empty();
            o.symbol.is_some() && o.stub.is_none() && unnamed && nowhere
        }
        _ => false,
    }
}

/// The pins of elements that a port or a supply or ground symbol `k` goes beside
/// ([`Structure::attach_pins`]).
fn attach_pins(objects: &[ObjectInfo], connections: &Connections, k: usize) -> Vec<PinKey> {
    let mut pins: Vec<PinKey> = connections.targets.get(&(k, 0)).cloned().unwrap_or_default();
    if pins.is_empty() {
        for &i in connections.nets_of.get(&(k, 0)).map_or(&[][..], |v| v) {
            pins.extend(connections.nets[i].pins.iter().copied().filter(|p| p.0 != k));
        }
    }
    pins.retain(|p| objects[p.0].role == Role::Element);
    pins.sort_unstable();
    pins.dedup();
    pins
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

/// Facts kept on x whose direction or side the facts leave open ([`Open::orders`],
/// [`Open::sides`]), or a DISTRIBUTE, which runs either way (14.4): the pairs of elements `(a, b)`
/// they put `a` left of `b`, as the witness has them (a DISTRIBUTE that it puts all at one x the
/// way of the netlist, `netlist_way`). The search may reverse them all, and the
/// resolution then turns the facts' literals. Facts that order the same two elements are one, as
/// they turn together.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Reversible {
    /// The facts, in order.
    pub facts: Vec<usize>,
    pub pairs: Vec<(usize, usize)>,
}

/// A source of candidates (algorithm synthesis 5.3), what a configured objective's structural
/// side, or the problem itself, contributes to making them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Source {
    /// Rows by current depth (current_direction).
    DepthRows,
    /// Elements connected across rows put in one column, in an order that keeps connections short
    /// and uncrossed: the barycentre of what each is connected to (component_alignment,
    /// crossings, wire_length).
    Barycentre,
    /// The members of a row of one set to space evenly ([`Structure::even_sets`]) evenly spaced
    /// (distribution).
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
    pub problem: LevelProblem,
    pub separate_gap: Q,
    pub currents: Vec<rnl_runtime::problem::ProblemCurrent>,
    pub symbol_names: Vec<String>,
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
    /// The box of the texts each object's symbol places (13.3), about the object's origin, for
    /// each orientation of the object in the order of `Orient::ALL`, by the object ([`text_box`]).
    pub labels: BTreeMap<usize, Vec<[Q; 4]>>,
    /// The boxes of the texts that offsets kept place from an element's origin on both axes, such
    /// as `#R1 @TEXT("R0", dx=-1, dy=3.5)`, about its origin, by the element: they move with it
    /// but do not turn (13.3; review of phase 2 of the quality review).
    pub offset_labels: BTreeMap<usize, Vec<[Q; 4]>>,
    /// Where each text with a position of its own goes when nothing decides it, by the text: at
    /// the point it is attached to, or beside the object; above the middle of a wire, at the start
    /// of a net (13.3).
    pub anchors: BTreeMap<usize, Anchor>,
    pub tree: GroupTree,
    /// The components of each scope of the objectives measured within groups: each group; the
    /// components in no group are no scope ([`scopes`]).
    pub scopes: Vec<Vec<usize>>,
    /// The sets of elements that a candidate with even rows spaces evenly where three or more of
    /// one are in a row ([`even_sets`]).
    pub even_sets: Vec<Vec<usize>>,
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
    /// The facts kept on x whose side is open, and the DISTRIBUTEs on x of three members or more,
    /// with the pairs they order.
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
    pub(crate) fn explicit_pin(&self, object: usize, pin: usize) -> bool {
        self.problem
            .objects
            .get(object)
            .and_then(|o| o.symbol)
            .and_then(|i| self.problem.symbols.get(i))
            .filter(|s| !s.default)
            .and_then(|s| s.pins.get(pin))
            .is_some_and(|p| p.direction_written)
    }

    /// Whether a source is in force.
    pub(crate) fn has(&self, s: Source) -> bool {
        self.sources.contains(&s)
    }

    /// The objects measured as components: elements, and supply, ground and port symbols, with a
    /// symbol and a place, but those that the solution leaves free.
    pub(crate) fn components(&self) -> impl Iterator<Item = usize> + '_ {
        self.objects.iter().enumerate().filter_map(|(k, o)| {
            let component = matches!(o.role, Role::Element | Role::Port);
            (component && o.symbol.is_some() && !o.left_free).then_some(k)
        })
    }

    /// The pins of elements that a port or a supply or ground symbol `k` goes beside: those its
    /// pin is drawn to, or else those on its nets, as far as the problem shows them (the nets of
    /// the ROUTEs and the wires written, [`Connections`]).
    pub(crate) fn attach_pins(&self, k: usize) -> Vec<PinKey> {
        attach_pins(&self.objects, &self.connections, k)
    }

    /// The pins of an object's symbol.
    pub(crate) fn pins(&self, k: usize) -> &[PinInfo] {
        self.objects[k].symbol.and_then(|s| self.symbols.get(s)).map_or(&[], |s| &s.pins)
    }

    pub(crate) fn conducts(&self, (k, pin): PinKey) -> bool {
        self.pins(k).get(pin).is_some()
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

    /// The box of the texts an object's symbol places, about its origin, with the object turned by
    /// `o`: each text's anchor turned with it, the text laid out upright as the renderer keeps it
    /// (9.2.5; [`text_box`]).
    pub(crate) fn label_offset(&self, k: usize, o: Orient) -> Option<[Q; 4]> {
        self.labels.get(&k)?.get(usize::from(crate::orient::index(o))).cloned()
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
        wires: !problem.routes.is_empty() || !problem.wires.is_empty(),
        currents: problem.currents.iter().any(|c| c.display.is_some()),
    };
    let active = Active::new(&options.objectives, &inputs);
    let sources = sources(&active);
    let mut hanging: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (k, o) in objects.iter().enumerate() {
        if let Some(stub) = &o.stub {
            hanging.entry(stub.host.0).or_default().push(k);
        }
    }
    let mut labels: BTreeMap<usize, Vec<[Q; 4]>> = BTreeMap::new();
    for o in &problem.objects {
        // the texts that move with their owner on both axes; one that the scene places on an axis
        // does not (13.3)
        let (Position::InSymbol { owner, local, replaced: None }, Some(t)) = (&o.position, &o.text) else { continue };
        let local = (number(local.0), number(local.1));
        let Some(boxes) = Orient::ALL.into_iter().map(|turn| text_box(t, &local, turn)).collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        add_labels(&mut labels, *owner, boxes);
    }
    // the texts that offsets kept place from an element's origin on both axes: at the offsets,
    // unturned
    let mut offset_labels: BTreeMap<usize, Vec<[Q; 4]>> = BTreeMap::new();
    let mut offsets: BTreeMap<usize, [Option<(usize, Q)>; 2]> = BTreeMap::new();
    for &i in &kept {
        let Some(FactKind::Offset { text, from: PointRef::Origin(owner), axis, value }) =
            problem.facts.get(i).map(|f| &f.kind)
        else {
            continue;
        };
        let (Some(o), Some(value)) = (objects.get(*owner), from_f64(*value)) else { continue };
        if o.role == Role::Element {
            offsets.entry(*text).or_default()[usize::from(*axis == Axis::Y)] = Some((*owner, value));
        }
    }
    for (text, [x, y]) in offsets {
        let (Some((owner, dx)), Some((other, dy)), Some(t)) =
            (x, y, problem.objects.get(text).and_then(|o| o.text.as_ref()))
        else {
            continue;
        };
        if owner != other || !matches!(problem.objects[text].position, Position::Own { .. }) {
            continue;
        }
        if let Some(b) = text_box(t, &(dx, dy), Orient::R0) {
            offset_labels.entry(owner).or_default().push(b);
        }
    }
    // the symbols that the solution leaves free first, as no text is anchored on their pins, then
    // the texts
    let none = BTreeMap::new();
    for k in 0..objects.len() {
        if objects[k].role != Role::Text {
            objects[k].left_free = left_free(problem, &objects, &symbols, &connections, &none, k);
        }
    }
    let anchors = anchors(problem, &connections, &objects);
    for k in 0..objects.len() {
        if objects[k].role == Role::Text {
            objects[k].left_free = left_free(problem, &objects, &symbols, &connections, &anchors, k);
        }
    }
    let mut s = Structure {
        problem: problem.clone(),
        separate_gap: options.separate_gap.clone(),
        currents: problem.currents.clone(),
        symbol_names: problem.symbols.iter().map(|s| s.name.clone()).collect(),
        symbols,
        objects,
        elements,
        connections,
        power,
        ports,
        pairs,
        hanging,
        labels,
        offset_labels,
        anchors,
        tree,
        scopes,
        even_sets: Vec::new(),
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
    s.even_sets = even_sets(problem, &s);
    s
}

/// The boxes of some texts of an object, one for each orientation, added to those of `labels`.
fn add_labels(labels: &mut BTreeMap<usize, Vec<[Q; 4]>>, owner: usize, boxes: Vec<[Q; 4]>) {
    match labels.get_mut(&owner) {
        Some(all) => {
            for (a, b) in all.iter_mut().zip(&boxes) {
                for i in 0..2 {
                    if b[i] < a[i] {
                        a[i] = b[i].clone();
                    }
                    if b[i + 2] > a[i + 2] {
                        a[i + 2] = b[i + 2].clone();
                    }
                }
            }
        }
        None => {
            labels.insert(owner, boxes);
        }
    }
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

/// How far the box of a text reaches past the advances of its runs at each end of its lines, in
/// sizes of the text: a margin for glyphs that reach past their advances, and for the rounding of
/// the advances that a frontend draws.
const TEXT_MARGIN: (i64, i64) = (1, 10);

/// How far above and below its baseline a run's glyphs reach, in sizes of the run: the capitals,
/// digits and ascenders of the Latin letters, about 0.72 high in the shipped fonts, and their
/// descenders, about 0.24 deep, with a margin; the ideographs, which fill the em box from 0.12
/// below the baseline to 0.88 above it, nearly so.
const RUN_ABOVE: (i64, i64) = (4, 5);
const RUN_BELOW: (i64, i64) = (2, 5);

/// The box of a text of a symbol anchored at `at` in the symbol's coordinates (9.2.5), about the
/// owner's origin with the owner turned by `turn`, as the renderer lays it out (quality review
/// ST-7): along its lines, from the start of the first run to the end of the last, each run as
/// wide as the advances of its characters in the shipped fonts (`rnl_render::lay_out`, decision
/// 0009), with the style's size, bold and italic (9.3; the font named is a hint that the layout
/// does not follow), and `TEXT_MARGIN` more at each end; across them, each run from `RUN_BELOW`
/// below its baseline to `RUN_ABOVE` above it, and the bars of fractions and overlines, the lines
/// and the box beside the anchor where the renderer puts them as `halign` and `valign` say; the
/// sides on a hundredth of a unit, outwards. The anchor turns with the owner, but the glyphs read
/// upright, as the renderer draws them: the text runs across or up by `rotate` and `turn`, with
/// `halign` reversed where they would read backwards and a `valign` of top or bottom swapped
/// where they would stand upside down, a baseline staying a baseline (the review of phase 2 of
/// the quality review: a value at the baseline of a symbol turned R270 lay mirrored across its
/// baseline). The solver keeps the symbols' texts clear of their neighbours by it. None for a
/// text without characters.
pub(crate) fn text_box(t: &TextInfo, at: &(Q, Q), turn: Orient) -> Option<[Q; 4]> {
    let style = |name: &str| t.style.iter().find(|(n, _)| n == name).map(|(_, v)| v);
    let size = style("size").and_then(|v| v.num()).filter(|s| s.is_finite() && *s > 0.0).unwrap_or(1.0);
    let flag = |name: &str| style(name).and_then(|v| v.boolean()).unwrap_or(false);
    if t.lines.iter().all(|l| line_chars(l) == 0) {
        return None;
    }
    // 0, 90, 180 or 270 (9.2.5), read exactly
    let r = from_f64(t.rotate).unwrap_or_default();
    let full = int(360);
    let r = &r - (&r / &full).floor() * &full;
    let quarter = [(90, Orient::R90), (180, Orient::R180), (270, Orient::R270)]
        .into_iter()
        .find(|(d, _)| r == int(*d))
        .map_or(Orient::R0, |(_, o)| o);
    // which way the text reads, `u`, and its glyphs stand, `v`, turned by its rotate and then
    // with its owner, as rnl_render's text() has them: a text that would read down reads up, one
    // that would read leftwards reads rightwards, each from the other end of its box
    let (u0, v0, u1, v1) = quarter.then(turn).matrix();
    let (u, v) = ((u0, u1), (v0, v1));
    let vertical = u.0 == 0;
    let backwards = if vertical { u.1 < 0 } else { u.0 < 0 };
    let upside_down = if vertical { v.0 > 0 } else { v.1 < 0 };
    let halign = match (backwards, t.halign.as_str()) {
        (true, "start") => "end".to_string(),
        (true, "end") => "start".to_string(),
        (_, h) => h.to_string(),
    };
    let valign = match (upside_down, t.valign.as_str()) {
        (true, "top") => "bottom".to_string(),
        (true, "bottom") => "top".to_string(),
        (_, h) => h.to_string(),
    };
    let mut laid = rnl_render::TextPrim {
        anchor: (0.0, 0.0),
        lines: t.lines.clone(),
        size,
        color: String::new(),
        font: String::new(),
        bold: flag("bold"),
        italic: flag("italic"),
        halign,
        valign,
        vertical: false,
        runs: Vec::new(),
        bars: Vec::new(),
    };
    // the renderer's own scale, which decides only how thin a bar may be
    let laid_box = rnl_render::lay_out(&mut laid, rnl_render::RenderOptions::default().scale);
    let ratio = |(n, d): (i64, i64)| n as f64 / d as f64;
    // across the lines, the glyphs of each run rather than the ascender and the descender of the
    // face, which the renderer's box reaches and which go well past them
    let (mut y1, mut y2) = (f64::MAX, f64::MIN);
    for r in &laid.runs {
        y1 = y1.min(r.at.1 - ratio(RUN_BELOW) * r.size);
        y2 = y2.max(r.at.1 + ratio(RUN_ABOVE) * r.size);
    }
    for b in &laid.bars {
        let half = b.stroke.width / 2.0;
        y1 = y1.min(b.a.1.min(b.b.1) - half);
        y2 = y2.max(b.a.1.max(b.b.1) + half);
    }
    if y1 > y2 {
        return None;
    }
    let margin = ratio(TEXT_MARGIN) * size;
    let hundredths = |v: f64, up: bool| {
        // the noise of the floating point off first, so that a side on a hundredth stays there
        let c = (v * 100.0 * 1e6).round() / 1e6;
        let c = if up { c.ceil() } else { c.floor() };
        // a box the size of a page, as no text drawn is wider than that
        frac(c.clamp(-1e12, 1e12) as i64, 100)
    };
    let b = [
        hundredths(laid_box[0] - margin, false),
        hundredths(y1, false),
        hundredths(laid_box[2] + margin, true),
        hundredths(y2, true),
    ];
    // a text that reads up has the tops of its glyphs to the left
    let b = if vertical { turned_box(&b, Orient::R90)? } else { b };
    let at = [turned(turn, at, 0), turned(turn, at, 1)];
    Some([&b[0] + &at[0], &b[1] + &at[1], &b[2] + &at[0], &b[3] + &at[1]])
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
    let mut touched = vec![[false; 2]; problem.objects.len()];
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
                    t[axis] = true;
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
            let attached_host = o
                .usage
                .and_then(|u| problem.usages.get(u))
                .and_then(|u| u.parent)
                .and_then(|u| problem.usages.get(u))
                .and_then(|u| {
                    u.roles.iter().flat_map(|r| &r.members).find_map(|m| match m {
                        Filler::Terminal { object, pin: Some(_), .. } => Some(*object),
                        _ => None,
                    })
                });
            let pin_symbol = o.symbol.and_then(|i| problem.symbols.get(i)).is_some_and(|s| !s.pins.is_empty());
            let role = match o.kind {
                ObjectKind::Point => Role::Point,
                ObjectKind::Text => Role::Text,
                ObjectKind::Usage if pin_symbol && (!own || !o.nets.is_empty() || attached_host.is_some()) => {
                    Role::Port
                }
                ObjectKind::Usage if own && o.symbol.is_some() => Role::Element,
                ObjectKind::Usage => Role::Other,
                ObjectKind::Instance | ObjectKind::Bundle if own => Role::Element,
                ObjectKind::Instance | ObjectKind::Bundle => Role::Other,
            };
            ObjectInfo {
                role,
                symbol: o.symbol,
                oriented: o.orient.is_some(),
                own,
                stub: None,
                attached_host,
                fact_touched: touched[k][0] || touched[k][1],
                fact_axes: touched[k],
                left_free: false,
            }
        })
        .collect()
}

/// Structural attachments do not reconstruct geometry that disabled language facts removed.
fn power(_problem: &LevelProblem, _objects: &mut [ObjectInfo]) -> Power {
    Power::default()
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
    // A virtual part's roles are its terminals (12.1): preserve the runtime's physical-pin
    // mapping, including distinct roles and bus pins, rather than assigning every role to pin 0.
    for usage in &problem.usages {
        let Some(k) = usage.object else { continue };
        for net in &problem.nets {
            nets.entry(net.name.clone())
                .or_default()
                .extend(net.pins.iter().filter_map(pin_of).filter(|(object, _)| *object == k));
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

/// Structural net attachments have no inferred electrical input/output kind.
fn ports(problem: &LevelProblem, objects: &[ObjectInfo], _symbols: &[SymbolInfo], _c: &Connections) -> Vec<Port> {
    objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.role == Role::Port)
        .map(|(object, _)| Port {
            object,
            net: problem.objects[object].nets.first().and_then(|&i| problem.nets.get(i)).map(|n| n.name.clone()),
        })
        .collect()
}

/// The mirror pairs of the semantic and layout usages.
fn pairs(_problem: &LevelProblem, _objects: &[ObjectInfo]) -> Vec<Pair> {
    Vec::new()
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
/// stubs that hang from its members. The components in no group are no scope: they are not a
/// group, and no alignment or spacing is judged among them (algorithm synthesis 9.3, decided
/// 2026-10-17; quality review SE-3).
fn scopes(problem: &LevelProblem, objects: &[ObjectInfo]) -> Vec<Vec<usize>> {
    let component = |k: usize| {
        let o = &objects[k];
        matches!(o.role, Role::Element | Role::Port) && o.symbol.is_some()
    };
    let owner = |k: usize| objects[k].attached_host.unwrap_or(k);
    let mut out = Vec::new();
    for g in &problem.groups {
        let members: BTreeSet<usize> = g.members.iter().copied().collect();
        let scope: Vec<usize> = (0..objects.len()).filter(|&k| component(k) && members.contains(&owner(k))).collect();
        if !scope.is_empty() {
            out.push(scope);
        }
    }
    out
}

/// What a net is to the role of an element on it ([`even_sets`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum NetRole {
    Supply,
    Ground,
    /// A signal joining two pins of elements and ports.
    Link,
    /// A signal joining three or more: a node that branches.
    Node,
    /// On no net that the problem shows.
    Unknown,
}

/// The sets of elements whose spacing a candidate with even rows makes equal, in a row where three
/// or more of one set are (`Source::EvenRows`; quality review SE-3): those that share a group,
/// each node of the tree of groups but the whole level; those joined by a DISTRIBUTE on x kept or a
/// mirror pair; and those of one kind and one role in the same group, or in none: the same symbol,
/// and the same roles of the nets of their pins, in any order, such as the legs of a ladder,
/// each from a node of the spine to a net of its own, or loads hung from one net to the ground; an
/// element on no net that the problem shows has no role. A chain of a `$FLOW` is none of these,
/// nor are the elements in no group as such. Each set sorted, none twice, those of fewer than three
/// elements left out.
fn even_sets(problem: &LevelProblem, s: &Structure) -> Vec<Vec<usize>> {
    let mut out: BTreeSet<Vec<usize>> = BTreeSet::new();
    for node in s.tree.nodes.iter().skip(1) {
        out.insert(node.members.clone());
    }
    // the DISTRIBUTEs on x kept and the mirror pairs, joined
    let mut joined = Joined::new(&s.elements);
    let is_element = |k: usize| s.objects.get(k).is_some_and(|o| o.role == Role::Element);
    for &f in &s.kept {
        let FactKind::Distribute { axis: Axis::X, members } = &problem.facts[f].kind else { continue };
        let mut ks = Vec::new();
        for m in members {
            elements_of(m, &mut ks);
        }
        ks.retain(|&k| is_element(k));
        for w in ks.windows(2) {
            joined.join(w[0], w[1]);
        }
    }
    for pair in &s.pairs {
        let ends: Vec<usize> =
            [Some(pair.a), Some(pair.b), pair.axis].into_iter().flatten().filter(|&k| is_element(k)).collect();
        for w in ends.windows(2) {
            joined.join(w[0], w[1]);
        }
    }
    let mut by_root: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &k in &s.elements {
        by_root.entry(joined.find(k)).or_default().push(k);
    }
    out.extend(by_root.into_values());
    // one kind and one role, in the same node of the tree of groups
    let net_role = |i: usize| {
        let net = &s.connections.nets[i];
        if net.supply {
            return NetRole::Supply;
        }
        if net.ground {
            return NetRole::Ground;
        }
        let ends = net
            .pins
            .iter()
            .filter(|(k, _)| s.objects.get(*k).is_some_and(|o| matches!(o.role, Role::Element | Role::Port)))
            .count();
        if ends >= 3 {
            NetRole::Node
        } else {
            NetRole::Link
        }
    };
    let node_of = |k: usize| s.tree.nodes.iter().position(|n| n.own.contains(&k)).unwrap_or(0);
    let mut by_role: BTreeMap<(usize, Option<usize>, Vec<NetRole>), Vec<usize>> = BTreeMap::new();
    for &k in &s.elements {
        let mut roles: Vec<NetRole> = (0..s.pins(k).len())
            .map(|pin| {
                if s.power.supply_pins.contains(&(k, pin)) {
                    NetRole::Supply
                } else if s.power.ground_pins.contains(&(k, pin)) {
                    NetRole::Ground
                } else {
                    s.connections
                        .nets_of
                        .get(&(k, pin))
                        .and_then(|nets| nets.first())
                        .map_or(NetRole::Unknown, |&i| net_role(i))
                }
            })
            .collect();
        // an element on no net that the problem shows has no role to tell
        if roles.iter().all(|r| *r == NetRole::Unknown) {
            continue;
        }
        roles.sort_unstable();
        by_role.entry((node_of(k), s.objects[k].symbol, roles)).or_default().push(k);
    }
    out.extend(by_role.into_values());
    out.into_iter()
        .map(|mut set| {
            set.sort_unstable();
            set
        })
        .filter(|set| set.len() >= 3)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
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
            Objective::CurrentDirection => &[Source::DepthRows],
            Objective::GroundDirection => &[],
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
    use rnl_runtime::problem::{CurrentDirection, CurrentTerminal, PointRef};
    let mut hosts: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for c in &s.currents {
        if let CurrentTerminal::Instance { instance, .. } = &c.terminal {
            let drawn = hosts.entry(*instance).or_default();
            if let Some(PointRef::Pin { object, .. }) = &c.display {
                drawn.insert(*object);
            }
        }
    }
    let ids: Vec<usize> = hosts.keys().copied().collect();
    let index: BTreeMap<_, _> = ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
    let n = ids.len();
    let mut by_net: BTreeMap<&str, (BTreeSet<usize>, BTreeSet<usize>)> = BTreeMap::new();
    for c in &s.currents {
        if let CurrentTerminal::Instance { instance, .. } = &c.terminal {
            let pair = by_net.entry(&c.net).or_default();
            match c.direction {
                CurrentDirection::Out => {
                    pair.0.insert(index[instance]);
                }
                CurrentDirection::In => {
                    pair.1.insert(index[instance]);
                }
            }
        }
    }
    let mut edges = vec![BTreeSet::new(); n];
    let mut reverse = vec![BTreeSet::new(); n];
    for (outs, ins) in by_net.values() {
        for &a in outs {
            for &b in ins {
                edges[a].insert(b);
                reverse[b].insert(a);
            }
        }
    }
    // Iterative Kosaraju avoids recursion and a quadratic reachability matrix.
    let mut visited = vec![false; n];
    let mut order = Vec::new();
    for seed in 0..n {
        if visited[seed] {
            continue;
        }
        let mut stack = vec![(seed, false)];
        while let Some((node, finish)) = stack.pop() {
            if finish {
                order.push(node);
                continue;
            }
            if visited[node] {
                continue;
            }
            visited[node] = true;
            stack.push((node, true));
            for &next in edges[node].iter().rev() {
                if !visited[next] {
                    stack.push((next, false));
                }
            }
        }
    }
    let mut component = vec![usize::MAX; n];
    let mut count = 0;
    for &seed in order.iter().rev() {
        if component[seed] != usize::MAX {
            continue;
        }
        let mut stack = vec![seed];
        component[seed] = count;
        while let Some(node) = stack.pop() {
            for &next in reverse[node].iter().rev() {
                if component[next] == usize::MAX {
                    component[next] = count;
                    stack.push(next);
                }
            }
        }
        count += 1;
    }
    let mut dag = vec![BTreeSet::new(); count];
    let mut incoming = vec![0; count];
    for a in 0..n {
        for &b in &edges[a] {
            let (x, y) = (component[a], component[b]);
            if x != y && dag[x].insert(y) {
                incoming[y] += 1;
            }
        }
    }
    let mut ready: BTreeSet<usize> = (0..count).filter(|&i| incoming[i] == 0).collect();
    let mut depth = vec![0u32; count];
    while let Some(node) = ready.pop_first() {
        for &next in &dag[node] {
            depth[next] = depth[next].max(depth[node].saturating_add(1));
            incoming[next] -= 1;
            if incoming[next] == 0 {
                ready.insert(next);
            }
        }
    }
    let mut out = vec![None; s.objects.len()];
    for (i, host) in ids.iter().enumerate() {
        for &object in &hosts[host] {
            let d = depth[component[i]];
            out[object] = Some(out[object].map_or(d, |old: u32| old.max(d)));
        }
    }
    out
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
/// members of one side unordered; an ALIGN fixes no side (14.2). A DISTRIBUTE fixes no side
/// either, but its members come in its order one way or the other, each a step further than the
/// one before (14.4), so it orders them as an ORDER given only an axis does: the way the witness
/// has them, or, where the witness puts them all at one x, the way of the netlist
/// ([`netlist_way`]), however it is written, and the search may reverse it. With fewer than three
/// members it holds wherever they are (14.4), and orders nothing. Its members that are no
/// elements, such as the axis of a `$MIRROR_PAIR` or the middle point of a `$TAIL_CENTER`, keep
/// their places in the order and order nothing themselves (review of phase 2 of the quality
/// review: a swap of neighbours broke the order of a DISTRIBUTE, the resolution then dropped the
/// gaps between them, and a text lay over the next symbol).
///
/// Two facts whose side is open that order the same two elements turn together, as one, since one
/// turned without the other leaves the search no order of them, the earlier turned the way of the
/// later where they run against each other (two DISTRIBUTEs at one x, each the way of the
/// netlist); and one that orders two elements that a fact with a side orders is not open at all,
/// its precedences fixed the way that fact has them.
fn precedence(problem: &LevelProblem, s: &Structure) -> (BTreeSet<(usize, usize)>, Vec<Reversible>) {
    let element = |k: usize| s.objects.get(k).is_some_and(|o| o.role == Role::Element);
    let elements = |m: &PointRef| -> Vec<usize> {
        let mut ks = Vec::new();
        elements_of(m, &mut ks);
        ks.retain(|&k| element(k));
        ks
    };
    let at = |ms: &[PointRef]| -> Vec<(usize, Q)> {
        let mut out = Vec::new();
        for m in ms {
            let Some(v) = point_value(problem, &s.witness, m, 0, 0) else { continue };
            out.extend(elements(m).into_iter().map(|k| (k, v.clone())));
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
    let mut open: Vec<Reversible> = Vec::new();
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
            FactKind::Distribute { axis: Axis::X, members } => {
                // with fewer than three members it holds wherever they are (14.4)
                if members.len() < 3 {
                    continue;
                }
                // the members with a place at the witness, in order: they run the way its first
                // and last are; all at one x, the way of the netlist, read from either end
                let chain: Vec<(Q, Vec<usize>)> = members
                    .iter()
                    .filter_map(|m| Some((point_value(problem, &s.witness, m, 0, 0)?, elements(m))))
                    .collect();
                let forward = match (chain.first(), chain.last()) {
                    (Some((first, _)), Some((last, _))) if first != last => first < last,
                    _ => netlist_way(chain.iter().map(|(_, ks)| ks.as_slice())),
                };
                let mut pairs = Vec::new();
                for (i, (_, a)) in chain.iter().enumerate() {
                    for (_, b) in &chain[i + 1..] {
                        for &x in a {
                            pairs.extend(b.iter().filter(|&&y| x != y).map(|&y| if forward { (x, y) } else { (y, x) }));
                        }
                    }
                }
                // an element in two members, such as `{#R1 #R2 #R1}`, at one x with all between:
                // no order of them
                let both: BTreeSet<(usize, usize)> = pairs.iter().copied().collect();
                pairs.retain(|&(a, b)| !both.contains(&(b, a)));
                (Sense::Either, pairs)
            }
            _ => continue,
        };
        pairs.sort_unstable();
        pairs.dedup();
        match sense {
            Sense::Either
                if s.open.orders.contains(&f)
                    || s.open.sides.contains(&f)
                    || matches!(problem.facts[f].kind, FactKind::Distribute { .. }) =>
            {
                if !pairs.is_empty() {
                    open.push(Reversible { facts: vec![f], pairs });
                }
            }
            Sense::Either => {}
            Sense::Increasing | Sense::Decreasing => fixed.extend(pairs),
        }
    }
    // the facts whose side is open that order the same two elements, one after the other, as one
    let unordered =
        |r: &Reversible| -> BTreeSet<(usize, usize)> { r.pairs.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect() };
    let turned = |pairs: &[(usize, usize)]| -> Vec<(usize, usize)> { pairs.iter().map(|&(a, b)| (b, a)).collect() };
    let mut merged: Vec<Reversible> = Vec::new();
    for mut r in open {
        loop {
            let shared = unordered(&r);
            let Some(i) = merged.iter().position(|m| !unordered(m).is_disjoint(&shared)) else { break };
            let mut m = merged.remove(i);
            // the way of `r`, where they run against each other
            if m.pairs.iter().any(|&(a, b)| r.pairs.contains(&(b, a))) {
                m.pairs = turned(&m.pairs);
            }
            r.facts.extend(m.facts);
            r.pairs.extend(m.pairs);
        }
        r.facts.sort_unstable();
        r.pairs.sort_unstable();
        r.pairs.dedup();
        merged.push(r);
    }
    merged.sort_by(|a, b| a.facts.cmp(&b.facts));
    // and those that a fact with a side turns already, fixed, the way it turns them
    let mut reversible = Vec::new();
    for r in merged {
        if r.pairs.iter().any(|p| fixed.contains(p)) {
            fixed.extend(r.pairs);
        } else if r.pairs.iter().any(|&(a, b)| fixed.contains(&(b, a))) {
            fixed.extend(turned(&r.pairs));
        } else {
            reversible.push(r);
        }
    }
    (fixed, reversible)
}

/// Whether a DISTRIBUTE whose members the witness puts all at one x runs as written, by the
/// elements of its members in order (`members`): a DISTRIBUTE has no direction (14.4), so the
/// members written the other way round must run the same way, which is the way of the netlist.
/// The members with elements, each by its elements in the order of the netlist, are read from
/// both ends, and the end that comes first in the netlist, compared as words are, goes left; the
/// members without elements, such as the axis of a `$MIRROR_PAIR`, order nothing and do not
/// count. Read the same from both ends, the way does not matter (review of the fix of phase 2:
/// the members ran as written, which drove the first plan, the barycentre rounds and the swaps).
fn netlist_way<'a>(members: impl DoubleEndedIterator<Item = &'a [usize]> + Clone) -> bool {
    let keyed = |ks: &[usize]| -> Option<Vec<usize>> {
        let mut ks = ks.to_vec();
        ks.sort_unstable();
        (!ks.is_empty()).then_some(ks)
    };
    members.clone().filter_map(keyed).le(members.rev().filter_map(keyed))
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
#[path = "../tests/unit/structure.rs"]
pub(crate) mod tests;
