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
//!   `$GND`, the nets they attach to and the terminals they hang from.
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
//!   direction is in force, otherwise by the height the witness of the resolution gives them, with
//!   the elements an ALIGN on y joins in one row; and the left-to-right precedences that the facts
//!   on x give the elements they join.
//! - **The sources of candidates** ([`Source`]), instantiated from the configured objectives in
//!   the order of their layers, each objective with a structural side adding its own, and the
//!   objectives configured but inert recorded with the reason ([`crate::objectives::Active`]).

use std::collections::{BTreeMap, BTreeSet};

use num_traits::Zero;
use rnl_runtime::geom::Orient;
use rnl_runtime::markup::Run;
use rnl_runtime::problem::{FactKind, Filler, LevelProblem, ObjectKind, PointRef, Position, TextInfo};

use crate::model::{turned, Model, PointId};
use crate::objectives::{Active, Inputs, Objective};
use crate::rational::{frac, from_f64, int, Q};
use crate::resolve::{point_value, Geometry, Resolved};
use crate::SolverOptions;

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
}

/// Supply and ground.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Power {
    /// The supply symbols, by object.
    pub supply: Vec<usize>,
    pub ground: Vec<usize>,
    pub supply_nets: BTreeSet<String>,
    pub ground_nets: BTreeSet<String>,
    /// The pins on a supply or ground net: the terminals stubs hang from, and the pins on those
    /// nets.
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
    /// first, the part they share and the part only in the second, in that order.
    Union(usize, usize),
    /// A part of a [`NodeKind::Union`]: 0 only in the first group, 1 shared, 2 only in the second.
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
    /// The order of its children is fixed: a union keeps the shared part between the others.
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
    /// Ground symbols hang below their pins (ground_direction).
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
    /// The facts: an ALIGN on y joins a row, the facts on x order what they join: always.
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
    /// Pairs of elements `(a, b)` that the facts on x join and the witness puts `a` left of `b`.
    pub precedence: BTreeSet<(usize, usize)>,
    pub active: Active,
    pub sources: Vec<Source>,
    /// Anything worth saying, such as groups that cannot be nested.
    pub notes: Vec<String>,
    /// The model the candidates are resolved on: without the tolerance when the facts kept hold so
    /// (`Resolved::as_written`).
    pub model: Model,
    /// The witness of the resolution: the facts kept hold there.
    pub witness: Geometry,
    /// The facts kept, in order.
    pub kept: Vec<usize>,
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

/// The structure of a level, from its resolution, for the options given.
pub(crate) fn analyse(problem: &LevelProblem, resolved: &Resolved, options: &SolverOptions) -> Structure {
    let mut notes = Vec::new();
    let symbols: Vec<SymbolInfo> = problem.symbols.iter().map(symbol_info).collect();
    let model = if resolved.as_written { Model::new(problem, Q::zero()) } else { resolved.model.clone() };
    let witness = resolved.witness.geometry.clone();
    let kept = resolved.kept.clone();
    let mut objects = object_infos(problem, &model, &kept);
    let power = power(problem, &mut objects);
    let connections = connections(problem, &model, &power);
    let power = power_pins(power, &connections);
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
        let (Position::InSymbol { owner, local }, Some(t)) = (&o.position, &o.text) else { continue };
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
        tree,
        scopes,
        open,
        depth: None,
        rows: Vec::new(),
        precedence: BTreeSet::new(),
        active,
        sources,
        notes,
        model,
        witness,
        kept,
    };
    if s.has(Source::DepthRows) {
        s.depth = Some(depths(&s));
    }
    s.rows = rows(problem, &s);
    s.precedence = precedence(problem, &s);
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
                    PointId::Pin(k, _) | PointId::Text(k) => model.points.get(p).and_then(|i| i.object).or(Some(*k)),
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

/// The supply and ground symbols, their nets, and the stubs with the pins they hang from.
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
                if let Filler::Net(name) = m {
                    if supply {
                        p.supply_nets.insert(name.clone());
                    } else {
                        p.ground_nets.insert(name.clone());
                    }
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

/// The pins on the supply and ground nets added to those the stubs hang from, and the nets marked.
fn power_pins(mut p: Power, c: &Connections) -> Power {
    for net in &c.nets {
        if p.supply_nets.contains(&net.name) {
            p.supply_pins.extend(net.pins.iter().copied());
        }
        if p.ground_nets.contains(&net.name) {
            p.ground_pins.extend(net.pins.iter().copied());
        }
    }
    p
}

fn connections(problem: &LevelProblem, model: &Model, power: &Power) -> Connections {
    let mut nets: BTreeMap<String, BTreeSet<PinKey>> = BTreeMap::new();
    let pin_of = |r: &PointRef| match r {
        PointRef::Pin { object, pin, .. } => Some((*object, *pin)),
        _ => None,
    };
    let mut drawn = Vec::new();
    let mut targets: BTreeMap<PinKey, BTreeSet<PinKey>> = BTreeMap::new();
    let mut seen_routes: BTreeSet<&str> = BTreeSet::new();
    for (i, route) in problem.routes.iter().enumerate() {
        let pins: Vec<PinKey> = route.pins.iter().filter_map(pin_of).collect();
        nets.entry(route.net.clone()).or_default().extend(pins.iter().copied());
        // a net asked for twice, as by a ROUTE that flows down and one of the level's own, is drawn once
        if !seen_routes.insert(route.net.as_str()) {
            continue;
        }
        let points: Vec<PointId> =
            model.route_points.get(i).map_or(Vec::new(), |ps| ps.iter().flatten().copied().collect());
        for a in &pins {
            for b in &pins {
                if a.0 != b.0 {
                    targets.entry(*a).or_default().insert(*b);
                }
            }
        }
        drawn.push(Drawn { net: route.net.clone(), points, routed: true });
    }
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
    let mut out = Connections { drawn, ..Connections::default() };
    for (name, pins) in nets {
        let i = out.nets.len();
        for p in &pins {
            out.nets_of.entry(*p).or_default().push(i);
        }
        out.nets.push(Net {
            supply: power.supply_nets.contains(&name),
            ground: power.ground_nets.contains(&name),
            name,
            pins: pins.into_iter().collect(),
        });
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

/// The tree of groups over the elements.
fn group_tree(problem: &LevelProblem, objects: &[ObjectInfo], elements: &[usize]) -> (GroupTree, Vec<String>) {
    let mut notes = Vec::new();
    // blocks: (members, kind); the root last added
    let mut blocks: Vec<(BTreeSet<usize>, NodeKind)> = Vec::new();
    let laminar = |a: &BTreeSet<usize>, b: &BTreeSet<usize>| a.is_disjoint(b) || a.is_subset(b) || b.is_subset(a);
    for (g, group) in problem.groups.iter().enumerate() {
        let set: BTreeSet<usize> = group
            .members
            .iter()
            .copied()
            .filter(|&k| objects.get(k).is_some_and(|o| o.role == Role::Element))
            .collect();
        if set.len() < 2 || blocks.iter().any(|(b, _)| *b == set) {
            continue;
        }
        let crossing: Vec<usize> = (0..blocks.len()).filter(|&i| !laminar(&blocks[i].0, &set)).collect();
        if crossing.is_empty() {
            blocks.push((set, NodeKind::Group(g)));
            continue;
        }
        // one group it overlaps: the union, with three parts, when that nests with the rest
        if let [i] = crossing.as_slice() {
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
                if nests {
                    blocks[*i] = (union, NodeKind::Union(h, g));
                    for (p, which) in parts {
                        if !p.is_empty() {
                            blocks.push((p, NodeKind::Part(h, g, which)));
                        }
                    }
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
    // a union's parts in their order; other children by their first element
    for n in 0..nodes.len() {
        let mut children = std::mem::take(&mut nodes[n].children);
        children.sort_by_key(|&c| match nodes[c].kind {
            NodeKind::Part(_, _, which) => (u64::from(which), 0),
            _ => (0, nodes[c].members[0] as u64),
        });
        let inner: BTreeSet<usize> = children.iter().flat_map(|&c| nodes[c].members.iter().copied()).collect();
        nodes[n].own = nodes[n].members.iter().copied().filter(|k| !inner.contains(k)).collect();
        nodes[n].children = children;
    }
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

/// The rows the elements start in (see the module).
fn rows(problem: &LevelProblem, s: &Structure) -> Vec<Vec<usize>> {
    if s.elements.is_empty() {
        return Vec::new();
    }
    // the key of each element's row: larger keys higher
    let mut key: BTreeMap<usize, Q> = BTreeMap::new();
    match &s.depth {
        Some(depth) => {
            let last = s.elements.iter().filter_map(|&k| depth[k]).max().map_or(0, |d| d + 1);
            for &k in &s.elements {
                key.insert(k, -int(i64::from(depth[k].unwrap_or(last))));
            }
            // an ALIGN on y kept joins its elements in the row of the first of them
            let mut parent: BTreeMap<usize, usize> = s.elements.iter().map(|&k| (k, k)).collect();
            fn find(parent: &BTreeMap<usize, usize>, mut k: usize) -> usize {
                while parent[&k] != k {
                    k = parent[&k];
                }
                k
            }
            for &f in &s.kept {
                if !matches!(problem.facts[f].kind, FactKind::Align { axis: rnl_runtime::problem::Axis::Y, .. }) {
                    continue;
                }
                let joined = touched_elements(s, f, 1);
                for w in joined.windows(2) {
                    let (a, b) = (find(&parent, w[0]), find(&parent, w[1]));
                    if a != b {
                        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
                        parent.insert(hi, lo);
                    }
                }
            }
            let keys = key.clone();
            for &k in &s.elements {
                key.insert(k, keys[&find(&parent, k)].clone());
            }
        }
        None => {
            // the heights of the elements that a fact kept on y relates to another element, or
            // places; other facts that reach one element only, such as those of its stubs, say
            // nothing of rows
            let mut related = BTreeSet::new();
            for &f in &s.kept {
                let joined = touched_elements(s, f, 1);
                let placed = matches!(
                    problem.facts[f].kind,
                    FactKind::Place { axis: rnl_runtime::problem::Axis::Y, .. }
                        | FactKind::PointCoordinate { axis: rnl_runtime::problem::Axis::Y, .. }
                );
                if joined.len() >= 2 || placed {
                    related.extend(joined);
                }
            }
            for &k in &related {
                key.insert(k, s.witness.origin[k][1].clone());
            }
            spread(s, &mut key);
        }
    }
    let mut levels: Vec<Q> = key.values().cloned().collect::<BTreeSet<Q>>().into_iter().collect();
    levels.reverse();
    levels.iter().map(|l| s.elements.iter().copied().filter(|k| key[k] == *l).collect()).collect()
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

/// The row keys of the elements without one: the key of the first element they are connected to
/// that has one, round after round; then, for those connected to none, the key that most elements
/// have, the highest of equals; 0 when no element has one.
fn spread(s: &Structure, key: &mut BTreeMap<usize, Q>) {
    loop {
        let found: Vec<(usize, Q)> = s
            .elements
            .iter()
            .filter(|k| !key.contains_key(k))
            .filter_map(|&k| neighbours(s, k).into_iter().find_map(|j| key.get(&j).cloned()).map(|q| (k, q)))
            .collect();
        if found.is_empty() {
            break;
        }
        key.extend(found);
    }
    let mut counts: BTreeMap<Q, usize> = BTreeMap::new();
    for q in key.values() {
        *counts.entry(q.clone()).or_default() += 1;
    }
    let common = counts.iter().max_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0))).map(|(q, _)| q.clone());
    let common = common.unwrap_or_default();
    for &k in &s.elements {
        key.entry(k).or_insert_with(|| common.clone());
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

/// The left-to-right precedences the facts kept on x give the elements they join, as the witness
/// orders them: for each fact, each member at the witness's x of the point the fact takes it by,
/// the elements of a set (a centre, 6.5) all at the centre's, and an element before another when
/// it is further left.
fn precedence(problem: &LevelProblem, s: &Structure) -> BTreeSet<(usize, usize)> {
    let mut out = BTreeSet::new();
    let element = |k: usize| s.objects.get(k).is_some_and(|o| o.role == Role::Element);
    for &f in &s.kept {
        let members: Vec<&PointRef> = match &problem.facts[f].kind {
            FactKind::Align { axis: rnl_runtime::problem::Axis::X, members }
            | FactKind::Order { axis: rnl_runtime::problem::Axis::X, members, .. }
            | FactKind::Distribute { axis: rnl_runtime::problem::Axis::X, members } => members.iter().collect(),
            FactKind::Distance { axis: rnl_runtime::problem::Axis::X, from, to, .. } => from.iter().chain(to).collect(),
            _ => continue,
        };
        let mut at: Vec<(usize, Q)> = Vec::new();
        for m in members {
            let Some(v) = point_value(problem, &s.witness, m, 0, 0) else { continue };
            let mut ks = Vec::new();
            elements_of(m, &mut ks);
            at.extend(ks.into_iter().filter(|&k| element(k)).map(|k| (k, v.clone())));
        }
        for (a, va) in &at {
            for (b, vb) in &at {
                if a != b && va < vb {
                    out.insert((*a, *b));
                }
            }
        }
    }
    out
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
mod tests {
    use super::*;
    use crate::resolve::tests::{fact, instance, problem_of};
    use rnl_runtime::problem::{Axis, ProblemGroup};

    fn structure_of(p: &LevelProblem) -> Structure {
        let options = SolverOptions::default();
        let r = crate::resolve::resolve(p, &options);
        analyse(p, &r, &options)
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
        // 1 above 0, 3 placed below; 2, related to nothing on y and connected to nothing, goes
        // with the most, the highest of equals
        assert_eq!(s.rows, vec![vec![1, 2], vec![0], vec![3]]);
        assert!(s.precedence.contains(&(2, 0)) && !s.precedence.contains(&(0, 2)));
        // every element is its own free class
        assert_eq!(s.open.classes.len(), 4);
        // no wires, no supply: current_direction and the routing objectives are inert
        assert!(s.active.inert.iter().any(|i| i.starts_with("current_direction")));
        assert!(!s.has(Source::DepthRows) && s.has(Source::Barycentre) && s.has(Source::FacingPins));
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
