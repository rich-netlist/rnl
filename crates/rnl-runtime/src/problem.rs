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
//! [`sig1`] is the hash of chapter 17 over a problem: `sig1:` then two digests of 16 hexadecimal
//! digits joined by `.`, the first of the solver's input, the second of the solver's options. A
//! viewer without a solver computes the first and compares only it ([`same_input`]).

use crate::diag::Loc;
use crate::geom::{fmt_num, Orient, Pt};
use crate::interp::obj::{Lit, Priority};
use crate::markup::{Line, Run};
use crate::model::PinView;

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

/// The sense of ORDER and DISTANCE along their axis (14.3, 14.5): `"left-to-right"` and
/// `"bottom-to-top"` increase, `"right-to-left"` and `"top-to-bottom"` decrease, `"x"` and `"y"`
/// take either.
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
    /// The centre of a set, the middle of the extent of its points, nested sets followed down
    /// to the innermost objects (6.5, 14.1).
    Centre(Vec<PointRef>),
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
    /// A usage with a symbol: a virtual part (12.1).
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
    /// ALIGN (14.2): the members, a set by its centre, are equal on the axis.
    Align { axis: Axis, members: Vec<PointRef> },
    /// ORDER (14.3): strictly monotonic, neighbours at least one unit apart.
    Order { axis: Axis, sense: Sense, members: Vec<PointRef> },
    /// DISTRIBUTE (14.4): equal steps between neighbours.
    Distribute { axis: Axis, members: Vec<PointRef> },
    /// DISTANCE (14.5) between the extents of the points of `from` and of `to`.
    Distance { axis: Axis, sense: Sense, min: Option<f64>, max: Option<f64>, from: Vec<PointRef>, to: Vec<PointRef> },
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
    /// Its own `priority=`, or the default of its record (14.8).
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
    /// out of the input digest ([`sig1`]).
    pub source: Option<Loc>,
}

/// What a solver needs of one drawing level.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelProblem {
    /// The host level: its index in the host document's levels.
    pub level: usize,
    pub title: String,
    /// "file" or "subckt".
    pub kind: &'static str,
    pub name: String,
    /// The language version the level's file is read in (16.1): what it declares; for an included
    /// file that declares none, the version of the file that includes it; else the toolkit's
    /// default, "0.3". Only the record of the solver of a file of the draft 0.3 writes numbers
    /// ([`Reference::Number`]).
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
    pub groups: Vec<ProblemGroup>,
    /// Every usage made in the level, in the order they were made, so a usage comes after the one
    /// whose expansion made it.
    pub usages: Vec<UsageEntry>,
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

/// The digest of the solver's input: 16 hexadecimal digits over [`canonical`]. A viewer without
/// a solver computes it from a problem built without the generated records ([`crate::Options::solver`]).
pub fn input_digest(p: &LevelProblem) -> String {
    format!("{:016x}", fnv1a(canonical(p).as_bytes()))
}

/// The digest of the solver's options: 16 hexadecimal digits over the options as the solver
/// writes them, one string with every setting that changes its result, the tolerance (11.7)
/// included, such as `tolerance=0.000001;grid=1`.
pub fn options_digest(options: &str) -> String {
    format!("{:016x}", fnv1a(options.as_bytes()))
}

/// The `hash` a solver writes in the head of its record for a level (2.8, 17):
/// `sig1:<input digest>.<options digest>`.
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
pub fn sig1(p: &LevelProblem, solver_options: &str) -> String {
    format!("{SIG1}:{}.{}", input_digest(p), options_digest(solver_options))
}

/// Does a `hash` written in a record head match the input of the level now? Only the input
/// digest is compared: a viewer does not know the solver's options. None when the value is not
/// a `sig1` hash, which a tool does not compare (17).
pub fn same_input(hash: &str, p: &LevelProblem) -> Option<bool> {
    let rest = hash.strip_prefix(SIG1)?.strip_prefix(':')?;
    let input = rest.split('.').next()?;
    if input.len() != 16 || !input.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(input.eq_ignore_ascii_case(&input_digest(p)))
}

/// The canonical text of a problem that [`input_digest`] hashes: every field but those that say
/// where things are written, in a fixed order, numbers by [`fmt_num`] and strings quoted.
pub fn canonical(p: &LevelProblem) -> String {
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
            FactKind::Align { axis, members } | FactKind::Distribute { axis, members } => {
                c.word(axis.as_str());
                c.points(members);
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
        c.str(&r.net);
        c.str(&r.mode);
        for (k, v) in &r.style {
            c.str(k);
            c.word(&written_lit(v));
        }
        c.points(&r.pins);
        c.indexes(&r.wires);
    }
    c.word("groups");
    for g in &p.groups {
        c.str(&g.label);
        c.indexes(&g.members);
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
                Run::Frac { num, den, style } => {
                    self.word("frac");
                    self.runs(num);
                    self.runs(den);
                    style
                }
            };
            self.word(&format!("{}{}{}{}", st.level, st.bold as u8, st.italic as u8, st.overline as u8));
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
            // numbers are left out of the hash (`sig1`)
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
        };
        let h = sig1(&p, "tolerance=0.000001");
        assert!(h.starts_with("sig1:"));
        assert_eq!(h.len(), "sig1:".len() + 16 + 1 + 16);
        assert_ne!(h, sig1(&p, "tolerance=0.001"));
        assert_eq!(same_input(&h, &p), Some(true));
        assert_eq!(same_input(&sig1(&p, "other"), &p), Some(true));
        let moved = LevelProblem { line: 7, end_line: 9, file: "u.sp".into(), level: 3, ..p.clone() };
        assert_eq!(input_digest(&moved), input_digest(&p));
        let renamed = LevelProblem { title: "u.sp".into(), ..p.clone() };
        assert_eq!(same_input(&h, &renamed), Some(false));
        assert_eq!(same_input("sig2:00000000", &p), None);
        assert_eq!(same_input("sig1:5be1c09a", &p), None);
    }
}
