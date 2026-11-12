//! The solver's view of one drawing level (standard 8.2 step 7, 8.5, 13.7, 14.8, 17).
//!
//! The effective model ([`crate::model`]) holds what a renderer draws: placed objects only, and
//! constraints by their status. A solver needs more: every object that has or needs a position,
//! with each coordinate fixed or free; the symbols; the geometric facts in the order in which 8.5
//! takes them in; the wires, ROUTEs, groups and usages; and how to write each result back.
//! [`LevelProblem`] is that view. The interpreter builds one per drawing level when
//! [`crate::Options::problems`] is set; [`crate::Options::solver`] also leaves out the records
//! with `type="generated"`, which a solver must not read (2.8).
//!
//! Objects, symbols, facts, wires and usages are referred to by their index in the lists of the
//! problem. The indexes follow the order of the interpretation, so the same input gives the same
//! problem, on every platform.
//!
//! [`sig2`] is the hash of chapter 17 over a problem: `sig2:` then two digests of 16 hexadecimal
//! digits joined by `.`, the first of the solver's input, the second of the solver's options. A
//! viewer without a solver computes the first and compares only it ([`same_input`]), also for a
//! `sig1` hash that tools wrote before, over the problem without its nets.

use crate::diag::Loc;
use crate::geom::{fmt_num, Orient, Pt};
use crate::interp::obj::{Lit, Priority};
use crate::markup::{Line, Run};
use crate::model::PinView;

/// One effective SOLVER-CONFIG key (14.10). The core passes its literal value unchanged;
/// a solver validates the keys and values it supports. Source metadata is not hashed.
#[derive(Clone, Debug, PartialEq)]
pub struct SolverSetting {
    pub key: String,
    pub value: Lit,
    pub source: Option<Loc>,
    pub package: String,
}

/// An axis of the scene: x to the right, y up (11.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Axis {
    X,
    Y,
}

impl Axis {
    pub fn as_str(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
        }
    }

    pub(crate) fn of(c: char) -> Axis {
        if c == 'y' {
            Axis::Y
        } else {
            Axis::X
        }
    }
}

/// The sense of ORDER, DISTANCE and, in a file of 0.4, DISTRIBUTE along their axis (14.3, 14.4,
/// 14.5): `"left-to-right"` and `"bottom-to-top"` increase, `"right-to-left"` and `"top-to-bottom"`
/// decrease, `"x"` and `"y"` of 0.3 take either.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sense {
    Increasing,
    Decreasing,
    Either,
}

impl Sense {
    pub fn as_str(self) -> &'static str {
        match self {
            Sense::Increasing => "increasing",
            Sense::Decreasing => "decreasing",
            Sense::Either => "either",
        }
    }

    pub(crate) fn of(s: Option<&str>) -> Sense {
        match s {
            Some("increasing") => Sense::Increasing,
            Some("decreasing") => Sense::Decreasing,
            _ => Sense::Either,
        }
    }
}

/// One coordinate of an object (11.1).
#[derive(Clone, Debug, PartialEq)]
pub enum Slot {
    /// No coordinate fact gives it: constraints and the solver decide.
    Free,
    /// The first coordinate fact on this axis in the order of 8.5 gives it: `fact` is its index
    /// in [`LevelProblem::facts`]. It is what a viewer that does not solve draws; a solver still
    /// resolves it with the other facts, and may discard it in a conflict.
    Fixed { value: f64, fact: usize },
}

/// The orientation of an object (11.2).
#[derive(Clone, Debug, PartialEq)]
pub enum OrientSlot {
    /// No ORIENT gives it: the solver chooses (8.5). A viewer that does not solve draws `"R0"`.
    Free,
    /// The first ORIENT in the order of 8.5 gives it: `fact` is its index in the facts.
    Fixed { code: Orient, fact: usize },
}

/// A point that facts, wires and ROUTEs refer to (14.1).
#[derive(Clone, Debug, PartialEq)]
pub enum PointRef {
    /// The origin of an object: an instance, a bundle or a virtual part by its origin, a point by
    /// itself, a text by its own position.
    Origin(usize),
    /// The pin of a terminal: the object's origin plus the pin position of its symbol, turned
    /// with the object (9.2.4). `pin` indexes the pins of the object's symbol; `terminal` is the
    /// terminal's name, or its number when it has none.
    Pin { object: usize, pin: usize, terminal: String },
    /// An independent pin that a terminal reference cannot identify (0.4, 9.2.4).
    /// Geometry uses `object` and `pin`; writeback must use this exact reference, and must leave
    /// an anonymous endpoint unsolved rather than connect another pin of the same terminal.
    SymbolPin { object: usize, pin: usize, reference: Reference },
    /// The coordinate point of a pin, with no electrical connection to that pin (5.2.4).
    /// `reference` reaches the point itself, including `::position`, for lossless wire writeback.
    PinPosition { object: usize, pin: usize, reference: Reference },
    /// The centre of a set, the middle of the extent of its points, nested sets followed down
    /// to the innermost objects (6.5, 14.1).
    Centre(Vec<PointRef>),
}

impl PointRef {
    /// The geometric identity of a pin, independent of the path used to refer to it.
    pub fn pin_key(&self) -> Option<(usize, usize)> {
        match self {
            Self::Pin { object, pin, .. } | Self::SymbolPin { object, pin, .. } => Some((*object, *pin)),
            _ => None,
        }
    }
}

/// Where an object is.
#[derive(Clone, Debug, PartialEq)]
pub enum Position {
    /// A position of its own, coordinate by coordinate.
    Own { x: Slot, y: Slot },
    /// A virtual part drawn at the coordinate point of its main object, with no position of its
    /// own: only its orientation is the solver's (12.1 rule 5).
    At(PointRef),
    /// A text placed by the symbol of `owner`: the owner's origin plus `local` turned with the
    /// owner (9.2.5, 13.3). A PLACE or an offset (`dx`, `dy`) in the scene replaces the symbol's
    /// position axis by axis, whichever of the two is written later (13.3): `replaced` is the axis
    /// that the scene gives, when it gives one, with the text's own coordinate there, as in
    /// [`Position::Own`]: fixed by the first PLACE on that axis, or free, and then placed by the
    /// offset, a [`FactKind::Offset`]. The symbol still gives the other axis, turned with the owner.
    /// A text whose two axes the scene gives is [`Position::Own`].
    InSymbol { owner: usize, local: Pt, replaced: Option<(Axis, Slot)> },
}

impl Position {
    /// Whether the symbol of an owner gives the coordinate on `axis`: on the axes of a
    /// [`Position::InSymbol`] that the scene does not replace (13.3).
    pub fn from_symbol(&self, axis: Axis) -> bool {
        match self {
            Position::InSymbol { replaced, .. } => replaced.as_ref().is_none_or(|(a, _)| *a != axis),
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ObjectKind {
    Instance,
    Bundle,
    /// A usage with a symbol, or any usage in 0.4, by its own point (12.1, 14.1).
    Usage,
    Point,
    /// A text: a label attached to an object, or an annotation attached to a point (13.3).
    Text,
}

impl ObjectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ObjectKind::Instance => "instance",
            ObjectKind::Bundle => "bundle",
            ObjectKind::Usage => "usage",
            ObjectKind::Point => "point",
            ObjectKind::Text => "text",
        }
    }
}

/// How the level's own text can refer to an object, which is what writing a result back needs
/// (5.7, 10.5, 13.3).
#[derive(Clone, Debug, PartialEq)]
pub enum Reference {
    /// By what is written: `#M1`, `%g0`, `#M1::name`, `%dp::mid`, `#M5:3::netname`.
    Name(String),
    /// Made by a rule (10.5): no single generated object can be referred to, so the generating
    /// usage, `usage` in [`LevelProblem::usages`], is first written out as an explicit statement
    /// with a name, and the object is that name followed by `path`, such as `["::g"]`. An empty
    /// path is the usage itself.
    Generated { usage: usize, path: Vec<String> },
    /// By its number, `%#3` (the draft 0.3, 5.9): an object without a name that no name reaches
    /// through `::`, such as the free point `(_, _)` written in a wire, a text without a key or a
    /// name, or a usage without a name; then `path`, such as `["::mid"]` for a local object of such
    /// a usage, `[":p"]` for one of its terminals. The numbers are those of the level read without
    /// the records of the solver, followed by the objects of each record of the solver read. They
    /// are computed for every version, but only the record of the solver of a file of the draft
    /// writes them ([`LevelProblem::version`]).
    Number { number: usize, path: Vec<String> },
    /// Without a name in the body of a usage that a rule makes, `usage` in
    /// [`LevelProblem::usages`], and reached through no name of it (5.9, 10.5), such as the wire of
    /// a `$GND_STUB` of the standard package. Once the record of the solver writes that usage out
    /// as an explicit statement ([`Reference::Generated`]), the object is one of the record's own,
    /// and the record refers to it by number: the objects that the statement writing the usage out
    /// makes are numbered right after those of the statements before it, the first of them after
    /// the objects of the authored records ([`LevelProblem::authored_numbers`]), and `index`, from
    /// 1, is this one's place among them ([`UsageEntry::written_numbers`]); then `path`. Computed
    /// for every version, written only by the record of the solver of a file of the draft.
    GeneratedNumber { usage: usize, index: usize, path: Vec<String> },
    /// Not at all: an object reached only through one that has no reference, such as a local
    /// object of a usage named only inside a `{…}`, or an object that no statement of the level
    /// makes, such as a text without a key of a symbol (5.7, 13.3).
    Anonymous,
}

impl Reference {
    /// The reference followed by a sub-object or a terminal, `suffix` being written with its `::`
    /// or `:`.
    pub fn then(&self, suffix: &str) -> Reference {
        match self {
            Reference::Name(n) => Reference::Name(format!("{n}{suffix}")),
            Reference::Generated { usage, path } => {
                let mut path = path.clone();
                path.push(suffix.to_string());
                Reference::Generated { usage: *usage, path }
            }
            Reference::Number { number, path } => {
                let mut path = path.clone();
                path.push(suffix.to_string());
                Reference::Number { number: *number, path }
            }
            Reference::GeneratedNumber { usage, index, path } => {
                let mut path = path.clone();
                path.push(suffix.to_string());
                Reference::GeneratedNumber { usage: *usage, index: *index, path }
            }
            Reference::Anonymous => Reference::Anonymous,
        }
    }
}

/// What fills a role of a usage, or what a text is attached to.
#[derive(Clone, Debug, PartialEq)]
pub enum Filler {
    /// An object of the problem.
    Object(usize),
    /// A terminal of an object of the problem, with the pin that draws it when there is one.
    Terminal { object: usize, terminal: String, pin: Option<usize> },
    /// One exact pin of an object's assigned symbol, including an unbound pin (9.2.4).
    Pin { object: usize, pin: usize },
    /// A usage without a symbol, by its index in [`LevelProblem::usages`].
    Usage(usize),
    /// A net, by its name.
    Net(String),
    /// A wire of the problem.
    Wire(usize),
    /// Anything else, as diagnostics show it.
    Other(String),
}

/// What a text shows and how, which its extent follows from (13.3, 13.6, 9.3).
#[derive(Clone, Debug, PartialEq)]
pub struct TextInfo {
    pub attach: Filler,
    pub key: Option<String>,
    /// The content as drawn: the markup read, `${…}` references replaced (13.6).
    pub lines: Vec<Line>,
    pub halign: String,
    pub valign: String,
    pub rotate: f64,
    /// Keep glyph rotation in scene axes; the anchor and box side follow the owner (13.3).
    pub fix_rotate: bool,
    /// Reflect the baseline text box with its transformed vertical axis (9.2.5).
    pub baseline_mirrors: bool,
    /// Local anchor in the reversed label column, used only by reversing orientations (9.2.5).
    pub column_position: Option<Pt>,
    /// The style properties that decide its extent and that some RNL gives it: `size`, `font`,
    /// `bold`, `italic` (9.3). The renderer's defaults fill in the rest.
    pub style: Vec<(String, Lit)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProblemObject {
    pub kind: ObjectKind,
    /// As diagnostics show it.
    pub label: String,
    pub reference: Reference,
    /// Made by a rule (10.5), alone or as part of a generated usage.
    pub generated: bool,
    /// The symbol it is drawn with, in [`LevelProblem::symbols`]; none for points and texts.
    pub symbol: Option<usize>,
    pub position: Position,
    /// None for points and texts, which have no orientation (11.2).
    pub orient: Option<OrientSlot>,
    /// For a virtual part: its entry in [`LevelProblem::usages`].
    pub usage: Option<usize>,
    pub text: Option<TextInfo>,
    /// The nets it is on, by their indexes in [`LevelProblem::nets`], in that order: for a
    /// virtual part, those of its terminals, a role filled by a net or by a terminal on one, as
    /// for an unnamed `$GND`, `$SUPPLY` or `$IOPORT`; for a text, those of what it is attached
    /// to, a net, a wire or a terminal (13.3). Empty for the others: the nets of an instance or a
    /// bundle are those its pins are on ([`ProblemNet::pins`]).
    pub nets: Vec<usize>,
}

impl ProblemObject {
    /// The local anchor after the symbol's label column ordering (9.2.5).
    pub fn symbol_local(&self, orient: Orient) -> Option<Pt> {
        let Position::InSymbol { local, .. } = self.position else { return None };
        Some(if orient.reverses_text_column() {
            self.text.as_ref().and_then(|t| t.column_position).unwrap_or(local)
        } else {
            local
        })
    }
}

/// A net of the level (6.3), with how a solver reaches it.
#[derive(Clone, Debug, PartialEq)]
pub struct ProblemNet {
    /// Its name as the problem names nets elsewhere ([`Filler::Net`], [`ProblemWire::nets`],
    /// [`RouteIntent::net`]): as the netlist first writes it, `0` for the ground.
    pub name: String,
    /// `&name`, how the level's own text refers to it.
    pub reference: Reference,
    /// Every pin on it: each terminal with a pin of an instance, a bundle or a virtual part, the
    /// terminals of `$GND`, `$SUPPLY` and `$IOPORT` and the like included, as a [`PointRef::Pin`];
    /// the pin of a bus of a bundle on each net of its members. In the order of the objects, then
    /// of the pins, each pin once. A terminal without a pin is not listed.
    pub pins: Vec<PointRef>,
    /// The wires that draw it, by their indexes in [`LevelProblem::wires`].
    pub wires: Vec<usize>,
    /// The ROUTEs that ask for it, by their indexes in [`LevelProblem::routes`].
    pub routes: Vec<usize>,
    /// For a net that is a port of the subcircuit, its place in the port list, from 0.
    pub port: Option<usize>,
}

/// A symbol as a solver sees it: its box and its pins (9.2.3, 9.2.4).
#[derive(Clone, Debug, PartialEq)]
pub struct ProblemSymbol {
    pub name: String,
    /// x1, y1, x2, y2 in local coordinates: `box=`, or the extent of the graphics.
    pub bbox: [f64; 4],
    pub box_written: bool,
    /// The viewer's own stand-in for an instance no symbol is given to (9.2.6).
    pub default: bool,
    pub pins: Vec<PinView>,
}

/// A physical pin with its written local direction; the owner supplies its orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectedPin {
    pub object: usize,
    pub pin: usize,
    /// Degrees from right in the symbol: 0, 90, 180 or 270 (9.2.4).
    pub direction: u16,
}

impl DirectedPin {
    pub fn drawn_direction(self, orientation: Orient) -> Option<u16> {
        orientation.direction(self.direction)
    }

    pub fn faces(self, orientation: Orient, codes: &[Orient]) -> bool {
        self.drawn_direction(orientation)
            .is_some_and(|direction| codes.iter().any(|code| code.direction(180) == Some(direction)))
    }

    pub fn relates(self, orientation: Orient, to: DirectedPin, to_orientation: Orient, codes: &[Orient]) -> bool {
        match (self.drawn_direction(orientation), to.drawn_direction(to_orientation)) {
            (Some(from), Some(to)) => codes.iter().any(|code| code.direction(from) == Some(to)),
            _ => false,
        }
    }
}

/// A cell or nested list of TABLE (14.11). Lists keep their written depth, including
/// single-item lists: each nesting changes the direction of its children.
#[derive(Clone, Debug, PartialEq)]
pub enum TableCell<P> {
    Point(P),
    List(Vec<TableCell<P>>),
}

impl<P> TableCell<P> {
    /// All cells in written order, without changing the nested structure.
    pub fn points(&self) -> Vec<&P> {
        match self {
            Self::Point(point) => vec![point],
            Self::List(items) => items.iter().flat_map(Self::points).collect(),
        }
    }

    /// Map cells while preserving every list. An unrepresentable cell fails the whole table.
    pub fn try_map<Q>(&self, f: &mut impl FnMut(&P) -> Option<Q>) -> Option<TableCell<Q>> {
        Some(match self {
            Self::Point(point) => TableCell::Point(f(point)?),
            Self::List(items) => TableCell::List(items.iter().map(|item| item.try_map(f)).collect::<Option<_>>()?),
        })
    }

    /// Adjacent-item point pairs, each at least one unit apart on its axis. X increases;
    /// Y decreases. These are parts of one fact, never independently ranked facts.
    pub fn orders(&self, axis: Axis) -> Vec<(Axis, &P, &P)> {
        let mut out = Vec::new();
        self.visit_orders(axis, &mut |a, from, to| {
            out.push((a, from, to));
            true
        });
        out
    }

    /// Visit orders without constructing an unbounded Cartesian product. Returning false
    /// stops the visit, allowing readers to enforce their implementation limit first.
    pub fn visit_orders<'a>(&'a self, axis: Axis, f: &mut impl FnMut(Axis, &'a P, &'a P) -> bool) -> bool {
        let Self::List(items) = self else { return true };
        let points: Vec<_> = items.iter().map(Self::points).filter(|points| !points.is_empty()).collect();
        for pair in points.windows(2) {
            for from in &pair[0] {
                for to in &pair[1] {
                    if !f(axis, from, to) {
                        return false;
                    }
                }
            }
        }
        let inner = match axis {
            Axis::X => Axis::Y,
            Axis::Y => Axis::X,
        };
        items.iter().all(|item| item.visit_orders(inner, f))
    }
}

/// A geometric fact (8.5) and what it relates.
#[derive(Clone, Debug, PartialEq)]
pub enum FactKind {
    /// One coordinate of a PLACE (11.1).
    Place { at: PointRef, axis: Axis, value: f64 },
    /// A coordinate written in a point, such as the 10 of `(10, _)`; it ranks like a PLACE
    /// written where the point is (5.1, 13.1).
    PointCoordinate { point: usize, axis: Axis, value: f64 },
    /// The `dx` or `dy` of a text: the text is `value` from `from` along `axis`, an exact
    /// increasing DISTANCE (13.3). Offsets replaced by a later PLACE on the same axis, or that
    /// replace an earlier one, are resolved before 8.5 and are not here.
    Offset { text: usize, from: PointRef, axis: Axis, value: f64 },
    /// ORIENT (11.2).
    Orient { object: usize, code: Orient },
    /// ORIENT on a pin (11.2): the code turns left into the requested direction.
    /// Each listed direction allows two owner orientations; this remains one fact.
    PinOrient { pin: DirectedPin, codes: Vec<Orient> },
    /// ORIENT-RELATION of two pins relates their drawn directions (14.9).
    PinOrientRelation { from: DirectedPin, to: DirectedPin, relations: Vec<Orient> },
    /// ALIGN (14.2): the members, a set by its centre, are equal on the axis.
    Align { axis: Axis, members: Vec<PointRef> },
    /// ORDER (14.3): strictly monotonic, neighbours at least one unit apart.
    Order { axis: Axis, sense: Sense, members: Vec<PointRef> },
    /// DISTRIBUTE (14.4): equal steps between neighbours; with a sense, which a file of 0.4 writes,
    /// the members also follow one another strictly in it, as for ORDER.
    Distribute { axis: Axis, sense: Sense, members: Vec<PointRef> },
    /// DISTANCE (14.5) between the extents of the points of `from` and of `to`.
    /// Exact one-point distances also encode body coordinate offsets from the usage point
    /// (0.4, 9.4), keeping the coordinate statement's label, source and rank.
    Distance { axis: Axis, sense: Sense, min: Option<f64>, max: Option<f64>, from: Vec<PointRef>, to: Vec<PointRef> },
    /// TABLE (14.11): one fact, with the outer axis and all nested lists. X increases,
    /// Y decreases, and each nested list changes axis.
    Table { axis: Axis, rows: TableCell<PointRef> },
    /// ORIENT-RELATION (14.9): each side shares one orientation, and `to` is `from` turned by
    /// `relation` in the frame of the drawing.
    OrientRelation { relation: Orient, from: Vec<usize>, to: Vec<usize> },
}

impl FactKind {
    /// The keyword, with the axis for the facts that are one coordinate.
    pub fn name(&self) -> &'static str {
        match self {
            FactKind::Place { axis: Axis::X, .. } => "PLACE-x",
            FactKind::Place { axis: Axis::Y, .. } => "PLACE-y",
            FactKind::PointCoordinate { axis: Axis::X, .. } => "POINT-x",
            FactKind::PointCoordinate { axis: Axis::Y, .. } => "POINT-y",
            FactKind::Offset { axis: Axis::X, .. } => "TEXT-dx",
            FactKind::Offset { axis: Axis::Y, .. } => "TEXT-dy",
            FactKind::Orient { .. } => "ORIENT",
            FactKind::PinOrient { .. } => "PIN-ORIENT",
            FactKind::PinOrientRelation { .. } => "PIN-ORIENT-RELATION",
            FactKind::Table { .. } => "TABLE",
            FactKind::Align { .. } => "ALIGN",
            FactKind::Order { .. } => "ORDER",
            FactKind::Distribute { .. } => "DISTRIBUTE",
            FactKind::Distance { .. } => "DISTANCE",
            FactKind::OrientRelation { .. } => "ORIENT-RELATION",
        }
    }
}

/// A geometric fact with what ranks it (8.5, 14.8). Its index in [`LevelProblem::facts`] is its
/// rank.
#[derive(Clone, Debug, PartialEq)]
pub struct ProblemFact {
    pub kind: FactKind,
    /// The effective priority (14.8); never `"disabled"`, as those facts are removed.
    pub priority: Priority,
    /// As diagnostics show it, such as `%c1` or `the x of #M1 = 3`.
    pub label: String,
    /// For a constraint with a handle, such as `%c1`: how to refer to it.
    pub reference: Option<Reference>,
    /// Where the statement is written; none for the coordinates of a point.
    pub source: Option<Loc>,
    /// The usages whose expansion made it, outermost first, in [`LevelProblem::usages`].
    pub chain: Vec<usize>,
}

/// A wire written in RNL (13.1).
#[derive(Clone, Debug, PartialEq)]
pub struct ProblemWire {
    pub label: String,
    pub reference: Reference,
    pub nets: Vec<String>,
    pub bus: bool,
    /// The points it passes, in order: pins of terminals and point objects.
    pub anchors: Vec<PointRef>,
    pub mode: String,
    /// Made by a rule (10.5), as opposed to written in a record or in the body of a usage written
    /// in one.
    pub generated: bool,
}

/// A net to draw as wires (13.7).
#[derive(Clone, Debug, PartialEq)]
pub struct RouteIntent {
    pub net: String,
    /// An already resolved tree (0.4, 13.7), which must not be merged with other trees on its net.
    /// False retains the whole-net requests of earlier language versions.
    pub tree: bool,
    /// A hint for the solver, with the values of WIRE.
    pub mode: String,
    /// The style parameters of the ROUTE, which the wires written back take.
    pub style: Vec<(String, Lit)>,
    /// The pins to join into one tree: the terminals on the net that have a pin.
    pub pins: Vec<PointRef>,
    /// The wires already written on the net, part of the tree.
    pub wires: Vec<usize>,
}

/// A set given GROUP: treat it as one block (11.5). Groups that PRIORITY turns off are left out.
#[derive(Clone, Debug, PartialEq)]
pub struct ProblemGroup {
    /// Identity of the grouped set within this problem. Repeated GROUPs on the same set share it.
    pub identity: usize,
    /// The first valid declaration on this set, counting declarations disabled by PRIORITY.
    pub first: bool,
    /// The GROUP is outside a definition expansion. This is provenance, not a property of its name.
    pub direct: bool,
    /// Optional whole grid-unit additions to the solver's default spacing (11.5).
    pub spacing_x: Option<f64>,
    pub spacing_y: Option<f64>,
    pub label: String,
    /// The objects of the problem among its members, nested sets followed down.
    pub members: Vec<usize>,
}

/// What fills one role of a usage (9.4, 12.1).
#[derive(Clone, Debug, PartialEq)]
pub struct RoleFill {
    pub role: String,
    /// The objects, sets followed down to their members.
    pub members: Vec<Filler>,
    /// How the level's text writes it, such as `#M1:3`, `&out` or `{#M1 #M2}`; none when part of
    /// it cannot be referred to.
    pub written: Option<String>,
}

/// A usage of a general definition (9.4): a virtual part when it has a symbol, otherwise a
/// semantic usage, which a solver may take as a hint of its own (14.8).
#[derive(Clone, Debug, PartialEq)]
pub struct UsageEntry {
    /// The definition, such as `$GND_STUB`.
    pub definition: String,
    pub label: String,
    pub reference: Reference,
    /// Made by a rule (10.5), alone or in the expansion of a generated usage.
    pub generated: bool,
    /// Made by a generation rule itself, not in the expansion of another generated usage: what is
    /// written back as an explicit statement (10.5, 17).
    pub rule_made: bool,
    pub roles: Vec<RoleFill>,
    /// The parameters it has a value for, passed or by default (9.4).
    pub params: Vec<(String, Lit)>,
    /// Its own `priority=`, or the default of its record, which in a file of 0.3 is only ever
    /// `"disabled"` (14.8). Writing the usage out keeps it (10.5).
    pub priority: Option<Priority>,
    /// The usage whose expansion made it.
    pub parent: Option<usize>,
    /// The object it is drawn as, for a virtual part.
    pub object: Option<usize>,
    /// For a usage that a rule makes: how many objects without a name the statement that writes
    /// it out makes, numbered among the record's own objects (5.9): those of its body, which are
    /// [`Reference::GeneratedNumber`] with `index` 1 and on, then the points that its roles are
    /// written with as coordinates. 0 for the other usages.
    pub written_numbers: usize,
    /// Where it is written: its AS or ATTACH, or the generation rule that makes it (10.5), such as
    /// a statement in the body of a definition for a usage made there. Like every place, it is left
    /// out of the input digest ([`sig2`]).
    pub source: Option<Loc>,
}

/// The final current direction on a host terminal that has a drawing pin (15.2).
/// Terminals without a pin and the direction "unspecified" are omitted.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalCurrent {
    pub pin: PointRef,
    pub direction: String,
}

/// What a solver needs of one drawing level.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelProblem {
    /// The junction definition resolved in this level for solver/editor writeback (13.2).
    pub junction: crate::junction::JunctionConfig,
    /// Effective settings, by key, after package, file-header and level overrides (14.10).
    pub solver_config: Vec<SolverSetting>,
    /// The host level: its index in the host document's levels.
    pub level: usize,
    pub title: String,
    /// "file" or "subckt".
    pub kind: &'static str,
    pub name: String,
    /// The language version the level's file is read in (16.1): what it declares; for an included
    /// file that declares none, the version of the file that includes it; else the toolkit's
    /// default, "0.3". Only the record of the solver of a file of the draft 0.3, or of a later
    /// version, writes numbers ([`Reference::Number`]).
    pub version: String,
    /// How many objects the authored records of the level number (5.9): the record of the solver
    /// numbers its own objects from the next number on ([`Reference::GeneratedNumber`]).
    pub authored_numbers: usize,
    pub file: String,
    pub line: u32,
    /// The line before which a record added to the end of the level goes (17).
    pub end_line: u32,
    /// The packages in effect, those of the standard package included, with their versions, in
    /// the order they are brought in (16.2, 16.3).
    pub packages: Vec<(String, String)>,
    /// The `%` names bound in the level, so that new ones do not clash (6.6).
    pub names: Vec<String>,
    /// Instances and bundles in netlist order, then virtual parts, points and texts, each in the
    /// order they were made.
    pub objects: Vec<ProblemObject>,
    pub symbols: Vec<ProblemSymbol>,
    /// The geometric facts in the order of resolution of 8.5: higher priority first, then the
    /// later first. Facts with priority `"disabled"` are left out (14.8), as are those that
    /// failed.
    pub facts: Vec<ProblemFact>,
    pub wires: Vec<ProblemWire>,
    pub routes: Vec<RouteIntent>,
    /// Final effective CURRENT declarations in a 0.4 level, mapped to their terminals' pins.
    /// Empty in released language versions, preserving their JSON and input digests.
    pub currents: Vec<TerminalCurrent>,
    pub groups: Vec<ProblemGroup>,
    /// Every usage made in the level, in the order they were made, so a usage comes after the one
    /// whose expansion made it.
    pub usages: Vec<UsageEntry>,
    /// The nets of the level, in the order the netlist first names them: the ports of the
    /// subcircuit, then the nets of the instances, line by line.
    pub nets: Vec<ProblemNet>,
    /// The ORIENT and ORIENT-RELATION facts written with a list of codes in a file of 0.4 (11.2,
    /// 14.9), by their index in `facts`: every code of the list, two or more, of which one is to
    /// hold. The fact itself has the first, which is what a solver that does not read the lists
    /// takes. Empty in a problem without such lists, which leaves the input digest as it was.
    pub orient_lists: Vec<(usize, Vec<Orient>)>,
    /// The ids the model gives the objects and the usages, in this reading. Like every place, they
    /// are left out of the input digest ([`sig2`]).
    pub ids: ProblemIds,
}

/// The ids of the objects and the usages of a problem in the model ([`crate::model::ObjectInfo::id`]),
/// in the order of their lists: `#M1`, `%g0`, or for one without a name where it was made.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProblemIds {
    pub objects: Vec<String>,
    pub usages: Vec<String>,
}

impl LevelProblem {
    /// Does the level's file follow 0.3 ([`Self::version`]): is it of 0.3 or a later version? Only
    /// there does the record of the solver refer to objects by number (5.9) and rewrite wires (13.1).
    pub fn follows_0_3(&self) -> bool {
        crate::interp::unit::keeps_0_3(&self.version)
    }
}

/// A point as the report of a solve names it: an object by its name, or else its label, `:` and
/// the terminal for a pin, a set in braces.
pub fn point_name(p: &LevelProblem, r: &PointRef) -> String {
    let label = |k: usize| {
        p.objects.get(k).map_or_else(
            || format!("object {k}"),
            |o| match &o.reference {
                Reference::Name(n) => n.clone(),
                _ => o.label.clone(),
            },
        )
    };
    match r {
        PointRef::Origin(k) => label(*k),
        PointRef::Pin { object, terminal, .. } => format!("{}:{terminal}", label(*object)),
        PointRef::SymbolPin { object, pin, reference } => match reference {
            Reference::Name(n) => n.clone(),
            _ => format!("pin {} of {}", pin + 1, label(*object)),
        },
        PointRef::PinPosition { object, pin, reference } => match reference {
            Reference::Name(n) => n.clone(),
            _ => format!("position of pin {} of {}", pin + 1, label(*object)),
        },
        PointRef::Centre(points) => {
            format!("{{{}}}", points.iter().map(|q| point_name(p, q)).collect::<Vec<_>>().join(" "))
        }
    }
}

/// What the fact of rank `k` says, in one line, such as `ALIGN y {#M1 #M2}` or `the x of #M1 = 3`.
pub fn fact_statement(p: &LevelProblem, k: usize) -> String {
    let f = &p.facts[k];
    let num = |v: f64| fmt_num(v);
    let list = |ms: &[PointRef]| format!("{{{}}}", ms.iter().map(|m| point_name(p, m)).collect::<Vec<_>>().join(" "));
    let objects = |ks: &[usize]| list(&ks.iter().map(|k| PointRef::Origin(*k)).collect::<Vec<_>>());
    let orientation = |first: Orient| {
        p.orient_lists.iter().find(|(i, _)| *i == k).map_or_else(
            || first.as_str().to_string(),
            |(_, codes)| format!("[{}]", codes.iter().map(|code| code.as_str()).collect::<Vec<_>>().join(", ")),
        )
    };
    match &f.kind {
        FactKind::Place { at, axis, value } => {
            format!("the {} of {} = {}", axis.as_str(), point_name(p, at), num(*value))
        }
        FactKind::PointCoordinate { point: k, axis, value } => {
            format!("the {} of {} = {}", axis.as_str(), point_name(p, &PointRef::Origin(*k)), num(*value))
        }
        // the `dx` or `dy` of a text, from the point it is attached to (13.3)
        FactKind::Offset { text, from, axis, value } => format!(
            "the d{} of {} from {} = {}",
            axis.as_str(),
            point_name(p, &PointRef::Origin(*text)),
            point_name(p, from),
            num(*value)
        ),
        FactKind::Orient { object, code } => {
            format!("the orientation {} of {}", orientation(*code), point_name(p, &PointRef::Origin(*object)))
        }
        FactKind::PinOrient { pin, codes } => format!(
            "ORIENT [{}] pin {} of {}",
            codes.iter().map(|code| code.as_str()).collect::<Vec<_>>().join(", "),
            pin.pin + 1,
            point_name(p, &PointRef::Origin(pin.object))
        ),
        FactKind::PinOrientRelation { from, to, relations } => format!(
            "ORIENT-RELATION [{}] pin {} of {} to pin {} of {}",
            relations.iter().map(|code| code.as_str()).collect::<Vec<_>>().join(", "),
            from.pin + 1,
            point_name(p, &PointRef::Origin(from.object)),
            to.pin + 1,
            point_name(p, &PointRef::Origin(to.object))
        ),
        FactKind::Table { axis, rows } => {
            fn show(p: &LevelProblem, cell: &TableCell<PointRef>) -> String {
                match cell {
                    TableCell::Point(point) => point_name(p, point),
                    TableCell::List(items) => {
                        format!("{{{}}}", items.iter().map(|item| show(p, item)).collect::<Vec<_>>().join(" "))
                    }
                }
            }
            format!("TABLE {} {}", axis.as_str(), show(p, rows))
        }
        FactKind::Align { axis, members } => format!("ALIGN {} {}", axis.as_str(), list(members)),
        FactKind::Distribute { axis, sense: Sense::Either, members } => {
            format!("DISTRIBUTE {} {}", axis.as_str(), list(members))
        }
        FactKind::Distribute { axis, sense, members } => {
            format!("DISTRIBUTE {} {} {}", axis.as_str(), sense.as_str(), list(members))
        }
        FactKind::Order { axis, sense, members } => {
            format!("ORDER {} {} {}", axis.as_str(), sense.as_str(), list(members))
        }
        FactKind::Distance { axis, sense, min, max, from, to } => {
            let mut bounds = String::new();
            if let Some(m) = min {
                bounds.push_str(&format!(" min={}", num(*m)));
            }
            if let Some(m) = max {
                bounds.push_str(&format!(" max={}", num(*m)));
            }
            format!("DISTANCE {} {}{bounds} {} {}", axis.as_str(), sense.as_str(), list(from), list(to))
        }
        FactKind::OrientRelation { relation, from, to } => {
            format!("ORIENT-RELATION {} {} {}", orientation(*relation), objects(from), objects(to))
        }
    }
}

/// A name as RNL writes it after a prefix, `:` or `::`: bare when it is ASCII letters, digits and
/// `_`, otherwise in quotes, a `"` doubled (3.6, 3.7).
pub fn written_name(name: &str) -> String {
    if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        name.to_string()
    } else {
        written_string(name)
    }
}

/// A string literal (3.7): in double quotes, a `"` doubled, nothing else escaped.
pub fn written_string(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// A literal as RNL writes it (3.7), numbers by [`fmt_num`].
pub fn written_lit(v: &Lit) -> String {
    match v {
        Lit::Num(n) => fmt_num(*n),
        Lit::Str(s) => written_string(s.as_str()),
        Lit::Bool(b) => b.to_string(),
        Lit::Arr(items) => format!("[{}]", items.iter().map(written_lit).collect::<Vec<_>>().join(", ")),
    }
}

// ---------------------------------------------------------------------------- the hash (17)

/// The tag of the algorithm (17).
pub const SIG1: &str = "sig1";

/// The 64-bit FNV-1a hash of some bytes.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The tag of the algorithm that a solver writes now (17): that of [`SIG1`], with the nets.
pub const SIG2: &str = "sig2";

/// The digest of the solver's input: 16 hexadecimal digits over [`canonical`]. A viewer without
/// a solver computes it from a problem built without the generated records ([`crate::Options::solver`]).
pub fn input_digest(p: &LevelProblem) -> String {
    format!("{:016x}", fnv1a(canonical(p).as_bytes()))
}

/// The digest of the input that a `sig1` hash gives: over [`canonical_sig1`], without the nets.
/// Tools read it and no longer write it.
pub fn input_digest_sig1(p: &LevelProblem) -> String {
    format!("{:016x}", fnv1a(canonical_sig1(p).as_bytes()))
}

/// The digest of the solver's options: 16 hexadecimal digits over the options as the solver
/// writes them, one string with every setting that changes its result, the tolerance (11.7)
/// included, such as `tolerance=0.000001;grid=1`.
pub fn options_digest(options: &str) -> String {
    format!("{:016x}", fnv1a(options.as_bytes()))
}

/// The `hash` a solver writes in the head of its record for a level (2.8, 17):
/// `sig2:<input digest>.<options digest>`.
///
/// The input digest covers the level after inheritance, as the problem holds it: the rules that
/// flow down and take effect, the symbols chosen, the packages and their versions, the objects,
/// facts, wires, ROUTEs, groups and usages. The problem is built without the records with
/// `type="generated"`, and the metadata fields of record heads never reach it. Where things are
/// written (files, lines and columns, the index of the level) is left out, so white space and
/// comments do not change it; the order of the facts, which is what the positions decide, is in
/// it. Numbers are written by [`fmt_num`], so the digest is the same on every platform. The
/// numbers of the objects without a name ([`Reference::Number`], [`Reference::GeneratedNumber`],
/// [`LevelProblem::authored_numbers`], [`UsageEntry::written_numbers`]) are left out, the object
/// counting as one without a reference: they follow from the text, which the check of the record
/// of the solver covers (the draft 0.3, 5.9), and so the digest is the same in every version.
///
/// `sig2` is `sig1` with the nets of the level (6.3): each net, the pins on it, the wires and the
/// ROUTEs on it, its place among the ports, and the nets of each object, so that connecting a
/// terminal elsewhere changes it. A record with a `sig1` hash, written before, is still compared,
/// over what that hash covers ([`same_input`]).
pub fn sig2(p: &LevelProblem, solver_options: &str) -> String {
    format!("{SIG2}:{}.{}", input_digest(p), options_digest(solver_options))
}

/// Does a `hash` written in a record head match the input of the level now? Only the input
/// digest is compared: a viewer does not know the solver's options. A `sig2` hash is compared
/// with [`input_digest`]; a `sig1` hash, which tools wrote before and no longer write, with
/// [`input_digest_sig1`], what it covers, so that a record written so reads as it did. None when
/// the value is neither, which a tool does not compare (17).
pub fn same_input(hash: &str, p: &LevelProblem) -> Option<bool> {
    let (digest, rest): (fn(&LevelProblem) -> String, &str) = match hash.split_once(':') {
        Some((SIG2, rest)) => (input_digest, rest),
        Some((SIG1, rest)) => (input_digest_sig1, rest),
        _ => return None,
    };
    let input = rest.split('.').next()?;
    if input.len() != 16 || !input.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(input.eq_ignore_ascii_case(&digest(p)))
}

/// The canonical text of a problem that [`input_digest`] hashes: that of [`canonical_sig1`], then
/// the nets, each net's name, reference, pins, wires, ROUTEs and place among the ports, and the
/// nets of each object.
pub fn canonical(p: &LevelProblem) -> String {
    let mut c = Canon(canonical_sig1(p));
    c.word("nets");
    for n in &p.nets {
        c.str(&n.name);
        c.reference(&n.reference);
        c.points(&n.pins);
        c.indexes(&n.wires);
        c.indexes(&n.routes);
        c.opt_index(n.port);
    }
    c.word("object-nets");
    for o in &p.objects {
        c.indexes(&o.nets);
    }
    if !p.currents.is_empty() {
        c.word("currents");
        for current in &p.currents {
            c.point(&current.pin);
            c.str(&current.direction);
        }
    }
    c.0
}

/// The canonical text of a problem that a `sig1` hash covers ([`input_digest_sig1`]): every field
/// but those that say where things are written and the nets, in a fixed order, numbers by
/// [`fmt_num`] and strings quoted.
pub fn canonical_sig1(p: &LevelProblem) -> String {
    let mut c = Canon(String::new());
    c.word("level");
    c.str(&p.title);
    c.word(p.kind);
    c.str(&p.name);
    c.word("packages");
    for (n, v) in &p.packages {
        c.str(n);
        c.str(v);
    }
    c.word("names");
    for n in &p.names {
        c.str(n);
    }
    c.word("symbols");
    for s in &p.symbols {
        c.word("symbol");
        c.str(&s.name);
        for v in s.bbox {
            c.num(v);
        }
        c.bool(s.box_written);
        c.bool(s.default);
        for pin in &s.pins {
            c.word("pin");
            c.num(pin.pos.0);
            c.num(pin.pos.1);
            c.opt_str(pin.label.as_deref());
            if pin.label_name_formats {
                c.word("label-name-formats");
            }
            match pin.dir {
                Some(d) => c.word(&d.to_string()),
                None => c.word("-"),
            }
            for t in &pin.terminals {
                c.str(t);
            }
        }
    }
    c.word("objects");
    for o in &p.objects {
        c.word(o.kind.as_str());
        c.str(&o.label);
        c.reference(&o.reference);
        c.bool(o.generated);
        c.opt_index(o.symbol);
        match &o.position {
            Position::Own { x, y } => {
                c.word("own");
                c.slot(x);
                c.slot(y);
            }
            Position::At(r) => {
                c.word("at");
                c.point(r);
            }
            Position::InSymbol { owner, local, replaced } => {
                c.word("in-symbol");
                c.index(*owner);
                c.num(local.0);
                c.num(local.1);
                // nothing more when the symbol gives both axes, so that the digest stays as it was
                if let Some((axis, slot)) = replaced {
                    c.word("replaced");
                    c.word(axis.as_str());
                    c.slot(slot);
                }
            }
        }
        match &o.orient {
            None => c.word("-"),
            Some(OrientSlot::Free) => c.word("free"),
            Some(OrientSlot::Fixed { code, fact }) => {
                c.word(code.as_str());
                c.index(*fact);
            }
        }
        c.opt_index(o.usage);
        if let Some(t) = &o.text {
            c.word("text");
            c.filler(&t.attach);
            c.opt_str(t.key.as_deref());
            for l in &t.lines {
                c.word("line");
                c.runs(l);
            }
            c.str(&t.halign);
            c.str(&t.valign);
            c.num(t.rotate);
            if t.fix_rotate {
                c.word("fix-rotate");
            }
            if t.baseline_mirrors {
                c.word("baseline-mirrors");
                if t.fix_rotate {
                    c.word("fixed-box-side");
                }
            }
            if let Some((x, y)) = t.column_position {
                c.word("column-position");
                c.num(x);
                c.num(y);
            }
            for (k, v) in &t.style {
                c.str(k);
                c.word(&written_lit(v));
            }
        }
    }
    c.word("facts");
    for f in &p.facts {
        c.word(f.kind.name());
        match &f.kind {
            FactKind::Place { at, value, .. } => {
                c.point(at);
                c.num(*value);
            }
            FactKind::PointCoordinate { point, value, .. } => {
                c.index(*point);
                c.num(*value);
            }
            FactKind::Offset { text, from, value, .. } => {
                c.index(*text);
                c.point(from);
                c.num(*value);
            }
            FactKind::Orient { object, code } => {
                c.index(*object);
                c.word(code.as_str());
            }
            FactKind::PinOrient { pin, codes } => {
                c.index(pin.object);
                c.index(pin.pin);
                c.index(usize::from(pin.direction));
                for code in codes {
                    c.word(code.as_str());
                }
            }
            FactKind::PinOrientRelation { from, to, relations } => {
                for pin in [from, to] {
                    c.index(pin.object);
                    c.index(pin.pin);
                    c.index(usize::from(pin.direction));
                }
                for relation in relations {
                    c.word(relation.as_str());
                }
            }
            // without a sense, as a file of 0.3 writes it, as it was, so that the digest stays
            FactKind::Align { axis, members } | FactKind::Distribute { axis, sense: Sense::Either, members } => {
                c.word(axis.as_str());
                c.points(members);
            }
            FactKind::Distribute { axis, sense, members } => {
                c.word(axis.as_str());
                c.word(sense.as_str());
                c.points(members);
            }
            FactKind::Table { axis, rows } => {
                // Include list boundaries, not just leaves: nesting changes the constraint.
                fn table(c: &mut Canon, cell: &TableCell<PointRef>) {
                    match cell {
                        TableCell::Point(point) => {
                            c.word("point");
                            c.points(std::slice::from_ref(point));
                        }
                        TableCell::List(items) => {
                            c.word("list");
                            c.index(items.len());
                            for item in items {
                                table(c, item);
                            }
                        }
                    }
                }
                c.word(axis.as_str());
                table(&mut c, rows);
            }
            FactKind::Order { axis, sense, members } => {
                c.word(axis.as_str());
                c.word(sense.as_str());
                c.points(members);
            }
            FactKind::Distance { axis, sense, min, max, from, to } => {
                c.word(axis.as_str());
                c.word(sense.as_str());
                c.opt_num(*min);
                c.opt_num(*max);
                c.points(from);
                c.points(to);
            }
            FactKind::OrientRelation { relation, from, to } => {
                c.word(relation.as_str());
                c.indexes(from);
                c.indexes(to);
            }
        }
        c.word(f.priority.as_str());
        c.str(&f.label);
        match &f.reference {
            Some(Reference::Number { .. } | Reference::GeneratedNumber { .. }) | None => c.word("-"),
            Some(r) => c.reference(r),
        }
        c.indexes(&f.chain);
    }
    // the lists of codes of 0.4, only when there are any, so that the digest of a problem without
    // them is as it was (11.2, 14.9)
    if !p.orient_lists.is_empty() {
        c.word("orient-lists");
        for (k, codes) in &p.orient_lists {
            c.index(*k);
            for code in codes {
                c.word(code.as_str());
            }
        }
    }
    c.word("wires");
    for w in &p.wires {
        c.str(&w.label);
        c.reference(&w.reference);
        for n in &w.nets {
            c.str(n);
        }
        c.bool(w.bus);
        c.points(&w.anchors);
        c.str(&w.mode);
        c.bool(w.generated);
    }
    c.word("routes");
    for r in &p.routes {
        if r.tree {
            c.word("tree");
        }
        c.str(&r.net);
        c.str(&r.mode);
        for (k, v) in &r.style {
            c.str(k);
            c.word(&written_lit(v));
        }
        c.points(&r.pins);
        c.indexes(&r.wires);
    }
    if let Some(definition) = &p.junction.definition {
        c.word("junction-definition");
        c.str(definition);
    }
    if !p.solver_config.is_empty() {
        c.word("solver-config");
        for setting in &p.solver_config {
            c.str(&setting.key);
            c.word(&setting.value.show());
        }
    }
    c.word("groups");
    for g in &p.groups {
        if p.version == "0.4" {
            c.word("group-origin");
            c.index(g.identity);
            c.word(if g.first { "first" } else { "repeated" });
            c.word(if g.direct { "direct" } else { "expanded" });
        }
        c.str(&g.label);
        c.indexes(&g.members);
        // Preserve the canonical text for groups written before spacing was available.
        for (key, value) in [("spacing_x", g.spacing_x), ("spacing_y", g.spacing_y)] {
            if let Some(value) = value {
                c.word(key);
                c.num(value);
            }
        }
    }
    c.word("usages");
    for u in &p.usages {
        c.str(&u.definition);
        c.str(&u.label);
        c.reference(&u.reference);
        c.bool(u.generated);
        c.bool(u.rule_made);
        for r in &u.roles {
            c.word("role");
            c.str(&r.role);
            for m in &r.members {
                c.filler(m);
            }
            c.opt_str(r.written.as_deref());
        }
        for (k, v) in &u.params {
            c.str(k);
            c.word(&written_lit(v));
        }
        match u.priority {
            Some(pr) => c.word(pr.as_str()),
            None => c.word("-"),
        }
        c.opt_index(u.parent);
        c.opt_index(u.object);
    }
    c.0
}

/// The writer of [`canonical`]: words separated by spaces, lists in brackets.
struct Canon(String);

impl Canon {
    fn word(&mut self, w: &str) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        self.0.push_str(w);
    }

    fn str(&mut self, s: &str) {
        self.word(&written_string(s));
    }

    fn opt_str(&mut self, s: Option<&str>) {
        match s {
            Some(s) => self.str(s),
            None => self.word("-"),
        }
    }

    fn num(&mut self, v: f64) {
        self.word(&fmt_num(v));
    }

    fn opt_num(&mut self, v: Option<f64>) {
        match v {
            Some(v) => self.num(v),
            None => self.word("-"),
        }
    }

    fn bool(&mut self, b: bool) {
        self.word(if b { "true" } else { "false" });
    }

    fn index(&mut self, k: usize) {
        self.word(&k.to_string());
    }

    fn opt_index(&mut self, k: Option<usize>) {
        match k {
            Some(k) => self.index(k),
            None => self.word("-"),
        }
    }

    fn indexes(&mut self, ks: &[usize]) {
        self.word("[");
        for &k in ks {
            self.index(k);
        }
        self.word("]");
    }

    fn runs(&mut self, runs: &[Run]) {
        self.word("[");
        for r in runs {
            let st = match r {
                Run::Text { text, style } => {
                    self.str(text);
                    style
                }
                Run::Sqrt { radicand, style } => {
                    self.word("sqrt");
                    self.runs(radicand);
                    style
                }
                Run::Frac { num, den, style } => {
                    self.word("frac");
                    self.runs(num);
                    self.runs(den);
                    style
                }
            };
            self.word(&format!("{}{}{}{}", st.level, st.bold as u8, st.italic as u8, st.overline as u8));
            if st.upright {
                self.word("upright");
            }
        }
        self.word("]");
    }

    fn slot(&mut self, s: &Slot) {
        match s {
            Slot::Free => self.word("free"),
            Slot::Fixed { value, fact } => {
                self.num(*value);
                self.index(*fact);
            }
        }
    }

    fn point(&mut self, r: &PointRef) {
        match r {
            PointRef::Origin(k) => {
                self.word("origin");
                self.index(*k);
            }
            PointRef::Pin { object, pin, terminal } => {
                self.word("pin");
                self.index(*object);
                self.index(*pin);
                self.str(terminal);
            }
            PointRef::SymbolPin { object, pin, reference } => {
                self.word("symbol-pin");
                self.index(*object);
                self.index(*pin);
                self.reference(reference);
            }
            PointRef::PinPosition { object, pin, reference } => {
                self.word("pin-position");
                self.index(*object);
                self.index(*pin);
                self.reference(reference);
            }
            PointRef::Centre(points) => {
                self.word("centre");
                self.points(points);
            }
        }
    }

    fn points(&mut self, rs: &[PointRef]) {
        self.word("[");
        for r in rs {
            self.point(r);
        }
        self.word("]");
    }

    fn reference(&mut self, r: &Reference) {
        match r {
            Reference::Name(n) => {
                self.word("name");
                self.str(n);
            }
            Reference::Generated { usage, path } => {
                self.word("generated");
                self.index(*usage);
                for s in path {
                    self.str(s);
                }
            }
            // numbers are left out of the hash (`sig2`, and `sig1` before it)
            Reference::Number { .. } | Reference::GeneratedNumber { .. } | Reference::Anonymous => {
                self.word("anonymous")
            }
        }
    }

    fn filler(&mut self, f: &Filler) {
        match f {
            Filler::Object(k) => {
                self.word("object");
                self.index(*k);
            }
            Filler::Terminal { object, terminal, pin } => {
                self.word("terminal");
                self.index(*object);
                self.str(terminal);
                self.opt_index(*pin);
            }
            Filler::Pin { object, pin } => {
                self.word("pin");
                self.index(*object);
                self.index(*pin);
            }
            Filler::Usage(k) => {
                self.word("usage");
                self.index(*k);
            }
            Filler::Net(n) => {
                self.word("net");
                self.str(n);
            }
            Filler::Wire(k) => {
                self.word("wire");
                self.index(*k);
            }
            Filler::Other(l) => {
                self.word("other");
                self.str(l);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_vectors() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn names_and_strings() {
        assert_eq!(written_name("M1"), "M1");
        assert_eq!(written_name("vdd!"), "\"vdd!\"");
        assert_eq!(written_name("M1<0>"), "\"M1<0>\"");
        assert_eq!(written_string("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(written_lit(&Lit::Arr(vec![Lit::Num(1.5), Lit::Bool(true)])), "[1.5, true]");
    }

    #[test]
    fn the_input_segment_is_compared_alone() {
        let p = LevelProblem {
            junction: Default::default(),
            solver_config: Vec::new(),
            level: 0,
            title: "t.sp".into(),
            kind: "file",
            name: "t.sp".into(),
            version: "0.2".into(),
            authored_numbers: 0,
            file: "t.sp".into(),
            line: 1,
            end_line: 3,
            packages: Vec::new(),
            names: Vec::new(),
            objects: Vec::new(),
            symbols: Vec::new(),
            facts: Vec::new(),
            wires: Vec::new(),
            routes: Vec::new(),
            groups: Vec::new(),
            usages: Vec::new(),
            nets: Vec::new(),
            orient_lists: Vec::new(),
            currents: Vec::new(),
            ids: ProblemIds::default(),
        };
        let h = sig2(&p, "tolerance=0.000001");
        assert!(h.starts_with("sig2:"));
        assert_eq!(h.len(), "sig2:".len() + 16 + 1 + 16);
        assert_ne!(h, sig2(&p, "tolerance=0.001"));
        assert_eq!(same_input(&h, &p), Some(true));
        assert_eq!(same_input(&sig2(&p, "other"), &p), Some(true));
        let moved = LevelProblem { line: 7, end_line: 9, file: "u.sp".into(), level: 3, ..p.clone() };
        assert_eq!(input_digest(&moved), input_digest(&p));
        let renamed = LevelProblem { title: "u.sp".into(), ..p.clone() };
        assert_eq!(same_input(&h, &renamed), Some(false));
        assert_eq!(same_input("sig2:00000000", &p), None);
        assert_eq!(same_input("sig3:0000000000000000.0000000000000000", &p), None);
        assert_eq!(same_input("sig1:5be1c09a", &p), None);
    }

    #[test]
    fn a_hash_of_sig1_is_compared_over_what_it_covers() {
        // a record written before sig2 has a sig1 hash, over the problem without its nets: it is
        // compared over that, so that it reads as it did; a sig2 hash covers the nets too
        let net = |name: &str, port: Option<usize>| ProblemNet {
            name: name.into(),
            reference: Reference::Name(format!("&{name}")),
            pins: Vec::new(),
            wires: Vec::new(),
            routes: Vec::new(),
            port,
        };
        let p = LevelProblem {
            junction: Default::default(),
            solver_config: Vec::new(),
            level: 0,
            title: "t.sp".into(),
            kind: "file",
            name: "t.sp".into(),
            version: "0.3".into(),
            authored_numbers: 0,
            file: "t.sp".into(),
            line: 1,
            end_line: 3,
            packages: Vec::new(),
            names: Vec::new(),
            objects: Vec::new(),
            symbols: Vec::new(),
            facts: Vec::new(),
            wires: Vec::new(),
            routes: Vec::new(),
            groups: Vec::new(),
            usages: Vec::new(),
            nets: vec![net("a", None)],
            orient_lists: Vec::new(),
            currents: Vec::new(),
            ids: ProblemIds::default(),
        };
        let old = format!("{SIG1}:{}.{}", input_digest_sig1(&p), options_digest("o"));
        assert_eq!(same_input(&old, &p), Some(true));
        assert!(canonical(&p).starts_with(&canonical_sig1(&p)) && canonical(&p) != canonical_sig1(&p));
        // the nets changed: the sig2 hash sees it, the sig1 hash does not cover them
        let other = LevelProblem { nets: vec![net("a", Some(0))], ..p.clone() };
        assert_eq!(same_input(&sig2(&p, "o"), &other), Some(false));
        assert_eq!(same_input(&old, &other), Some(true));
        let renamed = LevelProblem { title: "u.sp".into(), ..p.clone() };
        assert_eq!(same_input(&old, &renamed), Some(false));
    }
}
