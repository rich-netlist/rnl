//! A solver's result written back as RNL (standard 10.5, 13.3, 13.7, 14.8, 17; the draft 0.3,
//! 5.9, 13.1, 14.8).
//!
//! A solver gives its result for one level as a [`Solution`]: the position and orientation of the
//! objects of its [`LevelProblem`], the routes it chose for the `"auto"` wires written in RNL, and
//! the wires it routed for ROUTE. [`write_record`] turns it into the one record the solver writes
//! at the end of the level's own text, `@RNL(type="generated", priority="high", author="…",
//! hash="…") {…}`, in standard form. In a level of a file of the draft 0.3, [`finish_record`] then
//! puts `numbering`, the check of its numbers, in its head when it uses a number (5.9).
//! [`crate::writeback::replace_generated`] puts the record in place of the level's previous one.
//!
//! - **The interface is RNL** (the draft's 14.8): a [`Solution`] holds only what a record of the
//!   solver can hold, positions (PLACE), orientations (ORIENT), paths through the points of the
//!   problem and coordinates (WIRE), branch points (named points) and junctions (`$JUNCTION`), so
//!   any solution that a solver passes to another tool can be saved as one record.
//! - **Every decision is written back** (the draft's 14.8): by name or key for the objects that
//!   have one, by number for the others, with WIRE given the wire for the route of a wire, through
//!   explicit statements for what rules make. What the record of the level cannot hold is listed
//!   in [`Written::unsolved`] with the reason, never left out silently. Such a decision is not a
//!   result: the object stays free, as if the level were not solved, and a viewer that does not
//!   solve shows it so.
//! - **Numbers and rewritten wires** are written only in a level of a file of the draft 0.3
//!   ([`LevelProblem::version`]): a tool of 0.2 would read them as names it cannot find and as a
//!   net slot given the wrong kind. In a level of 0.2, the decisions on objects without a name and
//!   the routes of wires are unsolved ([`Unsolved::needs_0_3`]); `rnl solve` raises such a file to
//!   0.3 when no name in it is spelled like a word that 0.3 adds (16.1).
//! - Objects made by rules have no handle of their own (5.7, 10.5): their generating usage is
//!   written out first as an explicit statement with a name [`generated_names`] gives, and the
//!   objects are reached through it, as in `%gnd_stub_M1_s = $GND_STUB @ATTACH(…) #M1:s;
//!   %gnd_stub_M1_s::g @PLACE(0, -4)`, or, without a name in its body, by the number they take
//!   among the record's own objects ([`Reference::GeneratedNumber`]). The explicit usage then
//!   takes the place of the generated one when the file is read again.
//! - The route of a wire is WIRE given the wire, with `mode="straight"`, through the wire's own
//!   points and the bends between them (the draft's 13.1). A wire routed for a ROUTE is a new WIRE
//!   with `mode="straight"` and the style of its ROUTE; the points where wires branch are named
//!   points, with `$JUNCTION` where the solver wants a dot (13.2, 13.7).
//! - Coordinates are written by [`fmt_num`], which reads back as the same value (11.7).
//! - A text that the file places on an axis by what it is attached to, by an offset (`dx`, `dy`)
//!   or as its owner's symbol does, keeps that where the solution has it there: the record writes
//!   its position on the other axis only, `@PLACE(x=…)` or `@PLACE(y=…)`, or none, since a PLACE
//!   written after would replace it and the text would no longer follow what it is attached to
//!   (13.3). Where the solution has it elsewhere, as when the solver discarded the offset (8.5),
//!   the record writes the PLACE on that axis, which replaces the offset there (13.3).
//! - What the file fixes, where the solution has it, is no decision lost: a position or an
//!   orientation that the record cannot hold, such as that of a point without a name in a file of
//!   0.2, is not unsolved when it is what the facts of the file give the object (11.1, 11.2), since
//!   a viewer that does not solve draws it there all the same. An object that the record reaches
//!   has them written all the same, with the record's priority (14.8).

use std::fmt;

use crate::diag::Sink;
use crate::geom::{fmt_num, Orient, Pt, TOL};
use crate::host::{HostDocument, Record, Seg};
use crate::interp::obj::Priority;
use crate::numbering::level_check;
use crate::problem::*;
use crate::source::Provider;
use crate::syntax::{parse_record_with, uses_number};
use crate::writeback::record_type;
use crate::Options;

/// Where the solver puts one object of the problem.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    /// The object, by its index in [`LevelProblem::objects`].
    pub object: usize,
    /// Its coordinate on each axis that the solver decides: both, one alone, which the record
    /// writes as `@PLACE(x=…)` or `@PLACE(y=…)` (11.1), or none, for a virtual part drawn at the
    /// point of its main object, which has no position of its own (12.1), or when only the
    /// orientation is given. An axis not given is no decision of the solver's: it stays as the
    /// file leaves it, and is not unsolved.
    pub x: Option<f64>,
    pub y: Option<f64>,
    /// Its orientation, for the objects that have one: not points and texts (11.2).
    pub orient: Option<Orient>,
}

impl Placement {
    /// Its position, when both coordinates are given.
    pub fn at(&self) -> Option<Pt> {
        self.x.zip(self.y)
    }

    /// Its coordinate on an axis, when given.
    pub fn on(&self, axis: Axis) -> Option<f64> {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
        }
    }
}

/// A point where wires routed for ROUTE branch (13.7).
#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    pub at: Pt,
    /// Draw a dot there (13.2).
    pub junction: bool,
}

/// A point a wire of the solution passes.
#[derive(Clone, Debug, PartialEq)]
pub enum WirePoint {
    /// A pin, or the origin of an object of the problem: a point itself, any other object by its
    /// `::position` (6.5).
    Anchor(PointRef),
    /// A branch point of the solution, by its index in [`Solution::branches`].
    Branch(usize),
    /// A bend, written as coordinates: a point object of the record's own (5.9).
    Bend(Pt),
}

/// A wire routed for a ROUTE (13.7): a new wire.
#[derive(Clone, Debug, PartialEq)]
pub struct RoutedWire {
    /// The ROUTE, by its index in [`LevelProblem::routes`]: the net and the style.
    pub route: usize,
    pub points: Vec<WirePoint>,
}

/// The route the solver chose for an `"auto"` wire written in RNL (the draft 0.3, 13.1): written
/// back as WIRE given the wire, which rewrites it.
#[derive(Clone, Debug, PartialEq)]
pub struct WireRoute {
    /// The wire, by its index in [`LevelProblem::wires`].
    pub wire: usize,
    /// The path from one end to the other. The solver should keep the points of the wire
    /// ([`ProblemWire::anchors`]) in order and add only bends between them: it decides the
    /// geometry, and the connections stay as their author wrote them. The writer notes a path that
    /// does not ([`Written::notes`]), and writes it all the same.
    pub points: Vec<WirePoint>,
}

/// A solver's result for one level. It holds only what a record of the solver can hold (the draft
/// 0.3, 14.8), so that it can always be saved as one: what the record of a particular level cannot
/// hold, such as an object without a name in a file of 0.2, the writer lists as unsolved.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Solution {
    pub placements: Vec<Placement>,
    pub branches: Vec<Branch>,
    /// The wires routed for ROUTE (13.7).
    pub wires: Vec<RoutedWire>,
    /// The routes of the `"auto"` wires written in RNL (the draft 0.3, 13.1).
    pub wire_routes: Vec<WireRoute>,
}

/// One decision of a [`Solution`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// The position of an object, by its index in [`LevelProblem::objects`], on one axis, or on
    /// both when `axis` is None.
    Position { object: usize, axis: Option<Axis> },
    /// The orientation of an object, by its index in [`LevelProblem::objects`].
    Orientation { object: usize },
    /// The route of a wire, by its index in [`LevelProblem::wires`].
    WireRoute { wire: usize },
    /// A wire routed for a ROUTE, by its index in [`Solution::wires`].
    RoutedWire { index: usize },
    /// A branch point, by its index in [`Solution::branches`].
    Branch { index: usize },
}

/// A decision that the record of the level cannot hold. It is not a result (the draft 0.3, 14.8):
/// the object stays free, as if the level were not solved.
#[derive(Clone, Debug, PartialEq)]
pub struct Unsolved {
    pub decision: Decision,
    /// The object, the wire, the ROUTE or the branch point, as diagnostics show it.
    pub label: String,
    /// Why the record cannot hold it, with the section of the standard.
    pub reason: String,
}

impl Unsolved {
    /// Whether a record of a file of 0.3 holds it, where one of 0.2 cannot: the decision on an
    /// object without a name, which 0.3 refers to by its number (5.9), or the route of a wire, which
    /// 0.3 writes with WIRE given the wire (13.1). A tool raises such a file to 0.3 (16.1).
    pub fn needs_0_3(&self) -> bool {
        self.reason == NO_NUMBERS || self.reason == NO_REWRITE
    }
}

impl fmt::Display for Unsolved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.decision {
            Decision::Position { axis: None, .. } => "the position of ",
            Decision::Position { axis: Some(Axis::X), .. } => "the x of ",
            Decision::Position { axis: Some(Axis::Y), .. } => "the y of ",
            Decision::Orientation { .. } => "the orientation of ",
            Decision::WireRoute { .. } => "the route of ",
            Decision::RoutedWire { .. } | Decision::Branch { .. } => "",
        };
        write!(f, "{what}{}: {}", self.label, self.reason)
    }
}

/// A record written from a solution.
#[derive(Clone, Debug, PartialEq)]
pub struct Written {
    /// The whole record, from `@RNL` to the closing brace, lines joined by `\n` without the
    /// comment marker of the host, and without `numbering`: what [`finish_record`] takes, and then
    /// [`crate::writeback::replace_generated`].
    pub record: String,
    /// Every decision of the solution that the record does not hold, with the reason. Every other
    /// decision is in the record.
    pub unsolved: Vec<Unsolved>,
    /// What is written although the standard says that a solver should not do it, such as a route
    /// that does not keep the points of its wire (13.1). A tool that writes the record into its
    /// file adds why it does not, when it does not, such as `rnl solve` for a level in a region
    /// written to be encrypted (2.2).
    pub notes: Vec<String>,
}

const NOT_IN_PROBLEM: &str = "it is not in the problem";
const AT_POINT: &str = "it is drawn at the point of its main object and has no position of its own (12.1)";
const NO_ORIENTATION: &str = "it has no orientation (11.2)";
const ANONYMOUS: &str = "it cannot be referred to (5.7)";
const RULE_TEXT: &str = "a text without a key that a rule makes cannot be referred to: the solver does not decide its position, the viewer places it (13.3, 14.8)";
const NO_NUMBERS: &str =
    "it has no name, and a file of 0.2 cannot refer to it; the draft 0.3 refers to it by its number (5.9)";
const NOT_WRITTEN_OUT: &str = "the usage made by a rule that it belongs to cannot be written out (10.5)";
const NO_REWRITE: &str =
    "a file of 0.2 cannot rewrite a wire; the draft 0.3 writes its route with WIRE given the wire (13.1)";
const OLD_RECORD: &str =
    "it is an object of a record of the solver, which this record replaces: a solver reads the level without them (2.8)";
const TWO_POINTS: &str = "a wire has at least two points (13.1)";
const NOT_A_NUMBER: &str = "a coordinate is not a number of RNL, which has no NaN and no Infinity (3.7)";

/// A name for RNL made of any text: ASCII letters, digits and `_`, without runs of `_`.
fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let c = if c.is_ascii_alphanumeric() { c } else { '_' };
        if c == '_' && (out.is_empty() || out.ends_with('_')) {
            continue;
        }
        out.push(c);
    }
    out.trim_end_matches('_').to_string()
}

/// The names that the usages made by rules take when they are written out (10.5), by usage
/// index: the definition and what fills the first role, such as `gnd_stub_M1_s` for
/// `$GND_STUB @ATTACH #M1:s`, made unique within the level. The same problem gives the same
/// names, so the names stay as they were from one solve to the next while the netlist stays.
/// None for the other usages, and for those that cannot be written: a role that cannot be
/// referred to, or a definition without a `$` name.
pub fn generated_names(p: &LevelProblem) -> Vec<Option<String>> {
    let mut taken: Vec<String> = p.names.clone();
    let mut out = Vec::new();
    for u in &p.usages {
        let writable = u.rule_made && u.definition.starts_with('$') && u.roles.iter().all(|r| r.written.is_some());
        if !writable {
            out.push(None);
            continue;
        }
        let def = slug(&u.definition[1..].to_ascii_lowercase());
        let first = u.roles.first().and_then(|r| r.written.as_deref()).map(slug).unwrap_or_default();
        let base = if first.is_empty() { def } else { format!("{def}_{first}") };
        let mut name = base.clone();
        let mut n = 2;
        while taken.contains(&name) {
            name = format!("{base}_{n}");
            n += 1;
        }
        taken.push(name.clone());
        out.push(Some(name));
    }
    out
}

/// Coordinates as RNL writes them, or why it cannot.
fn coords((x, y): Pt) -> Result<String, String> {
    if x.is_finite() && y.is_finite() {
        Ok(format!("({}, {})", fmt_num(x), fmt_num(y)))
    } else {
        Err(NOT_A_NUMBER.into())
    }
}

/// A piece of a statement: a reference, written once the usages that the record writes out are
/// known, or text.
enum Piece {
    Ref(Reference),
    Text(String),
}

/// How the record of the solver of a level refers to objects.
struct Refs<'a> {
    p: &'a LevelProblem,
    /// The names of the usages made by rules, written out ([`generated_names`]).
    names: Vec<Option<String>>,
    /// The level's file follows 0.3: numbers and rewrites can be written.
    numbers: bool,
    /// For each usage written out: the number before those of the objects its statement makes.
    offsets: Vec<Option<usize>>,
}

impl Refs<'_> {
    /// Whether the record can write a reference, and the usage made by a rule that it goes
    /// through, which is then written out.
    fn reach(&self, r: &Reference) -> Result<Option<usize>, &'static str> {
        match r {
            Reference::Name(_) => Ok(None),
            Reference::Number { .. } | Reference::GeneratedNumber { .. } if !self.numbers => Err(NO_NUMBERS),
            // in a problem read with the records of the solver, their own objects come after the
            // authored ones: this record replaces them, and its numbers would name others (5.9)
            Reference::Number { number, .. } if *number > self.p.authored_numbers => Err(OLD_RECORD),
            Reference::Number { .. } => Ok(None),
            Reference::Generated { usage, .. } | Reference::GeneratedNumber { usage, .. } => {
                match self.names.get(*usage) {
                    Some(Some(_)) => Ok(Some(*usage)),
                    _ => Err(NOT_WRITTEN_OUT),
                }
            }
            Reference::Anonymous => Err(ANONYMOUS),
        }
    }

    /// How the record writes a reference it can reach, once [`Refs::offsets`] are known.
    fn text(&self, r: &Reference) -> String {
        match r {
            Reference::Name(n) => n.clone(),
            Reference::Generated { usage, path } => {
                format!("%{}{}", written_name(self.names[*usage].as_deref().unwrap_or_default()), path.concat())
            }
            Reference::Number { number, path } => format!("%#{number}{}", path.concat()),
            Reference::GeneratedNumber { usage, index, path } => {
                format!("%#{}{}", self.offsets[*usage].unwrap_or_default() + index, path.concat())
            }
            Reference::Anonymous => String::new(),
        }
    }

    fn piece(&self, q: &Piece) -> String {
        match q {
            Piece::Ref(r) => self.text(r),
            Piece::Text(t) => t.clone(),
        }
    }

    /// The points of a wire of the solution as the record writes them, with the usages made by
    /// rules they go through; or why the wire cannot be written.
    fn wire(&self, points: &[WirePoint], branches: &[Option<String>]) -> Result<(Vec<Piece>, Vec<usize>), String> {
        if points.len() < 2 {
            return Err(TWO_POINTS.into());
        }
        let mut pieces = Vec::new();
        let mut roots = Vec::new();
        for q in points {
            let (piece, label) = match q {
                WirePoint::Anchor(PointRef::Pin { object, terminal, .. }) => {
                    let o = self.p.objects.get(*object).ok_or("a point of it is not in the problem")?;
                    let r = o.reference.then(&format!(":{}", written_name(terminal)));
                    (Piece::Ref(r), format!("{}:{terminal}", o.label))
                }
                WirePoint::Anchor(PointRef::Origin(k)) => {
                    let o = self.p.objects.get(*k).ok_or("a point of it is not in the problem")?;
                    let r = match o.kind {
                        ObjectKind::Point => o.reference.clone(),
                        _ => o.reference.then("::position"),
                    };
                    (Piece::Ref(r), o.label.clone())
                }
                WirePoint::Anchor(PointRef::Centre(_)) => {
                    return Err("a point of it is the centre of a set, which this writer does not write (6.5)".into())
                }
                WirePoint::Branch(i) => match branches.get(*i) {
                    Some(Some(name)) => (Piece::Text(format!("%{name}")), String::new()),
                    Some(None) => return Err(format!("its branch point {i}: {NOT_A_NUMBER}")),
                    None => return Err(format!("its branch point {i} is not in the solution")),
                },
                WirePoint::Bend(at) => {
                    (Piece::Text(coords(*at).map_err(|why| format!("a bend of it: {why}"))?), String::new())
                }
            };
            if let Piece::Ref(r) = &piece {
                roots.extend(self.reach(r).map_err(|why| format!("its point {label}: {why}"))?);
            }
            pieces.push(piece);
        }
        Ok((pieces, roots))
    }
}

/// Does a route keep the points of its wire in order, with bends only between them (13.1)?
fn keeps_points(wire: &ProblemWire, points: &[WirePoint]) -> bool {
    let bend = |q: Option<&WirePoint>| matches!(q, Some(WirePoint::Bend(_)) | None);
    if bend(points.first()) || bend(points.last()) {
        return false;
    }
    let kept: Vec<Option<&PointRef>> = points
        .iter()
        .filter_map(|q| match q {
            WirePoint::Bend(_) => None,
            WirePoint::Anchor(r) => Some(Some(r)),
            WirePoint::Branch(_) => Some(None),
        })
        .collect();
    kept.len() == wire.anchors.len() && kept.iter().zip(&wire.anchors).all(|(a, b)| *a == Some(b))
}

/// The statement that writes out a usage made by a rule (10.5): its definition, its roles in
/// order, its parameters and its priority as it has it (14.8): the one its rule writes on it, or
/// the `"disabled"` of the record its rule is written in. In a file of 0.3 a usage without one is
/// written without: the record's `"high"` gives it none, so it keeps the priorities of the facts of
/// its body, as the one the rule made did. In a file of 0.2 it would take the record's, so there
/// it is written with `"normal"`.
fn usage_statement(u: &UsageEntry, name: &str, follows_0_3: bool) -> String {
    let mut params: Vec<String> = u.params.iter().map(|(k, v)| format!("{}={}", k, written_lit(v))).collect();
    let priority = if follows_0_3 { u.priority } else { Some(u.priority.unwrap_or(Priority::Normal)) };
    if let Some(priority) = priority {
        params.push(format!("priority=\"{}\"", priority.as_str()));
    }
    let roles: Vec<&str> = u.roles.iter().filter_map(|r| r.written.as_deref()).collect();
    let params = if params.is_empty() { String::new() } else { format!("({})", params.join(", ")) };
    format!("%{} = ${} @ATTACH{params} {}", written_name(name), written_name(&u.definition[1..]), roles.join(" "))
}

/// The solver's record for a level (14.8, 17): the usages made by rules that the result reaches,
/// written out; the branch points of routed wires; the positions and orientations; the wires
/// routed for ROUTE; the routes of the wires written in RNL; the junctions. `p` is the problem the
/// solver read, without the records of the solver ([`crate::Options::solver`]). `author` and
/// `hash` go in the record head, the hash being what [`crate::problem::sig2`] gives for the problem and the
/// solver's options.
///
/// Every decision of the solution is in the record, or in [`Written::unsolved`] with the reason
/// the record cannot hold it, but what the file fixes (below):
///
/// - in a level of a file of 0.2: the position or orientation of an object without a name, and
///   the route of a wire (the draft 0.3 writes them, by number and with WIRE given the wire);
/// - in every version: the position of a text without a key that a rule makes, which nothing can
///   refer to, so the solver does not decide it (13.3; the draft 0.3, 14.8); an object reached
///   only through a usage that a rule makes and that cannot be written out (10.5), or through
///   nothing (5.7); in a problem read with the records of the solver, an object that one of them
///   makes, which this record replaces; a position given to an object drawn at the point of its
///   main object (12.1), an orientation given to a point or a text (11.2); a wire with fewer than
///   two points (13.1), or through the centre of a set; a coordinate that is NaN or infinite
///   (3.7); an index that is not in the problem or the solution.
///
/// A text keeps the axes on which the file places it by what it is attached to, where the solution
/// has it there: by an offset (13.3), or by its owner's symbol on an axis that the scene does not
/// replace (9.2.5, 13.3). Its position is written on its other axes only, or not at all. On such an
/// axis where the solution has it elsewhere, as when the solver discarded the offset (8.5), the
/// PLACE is written, and replaces the offset there (13.3).
///
/// A position or an orientation that the record cannot hold is not unsolved when it is what the
/// file fixes, the coordinates that the facts of the file give (11.1) and the orientation that an
/// ORIENT gives (11.2), within [`TOL`]: a viewer that does not solve draws the object so all the
/// same. A record that reaches the object writes them, so that they rank with its priority (14.8).
///
/// The record holds what the solution gives, and a viewer checks it with its own tolerance (11.7),
/// such as [`TOL`]. Facts that hold together only within a larger tolerance of the solver do not
/// all hold so, whatever the record writes: a solver then keeps the constants of the facts that
/// rank first as written, as rnl-solver does, so that a viewer drops the later ones, in the order
/// of 8.5.
///
/// In a level of the draft 0.3 the record is not finished: when it uses a number,
/// [`finish_record`] gives it the check of its numbers.
pub fn write_record(p: &LevelProblem, solution: &Solution, author: &str, hash: &str) -> Written {
    let mut refs = Refs { p, names: generated_names(p), numbers: p.follows_0_3(), offsets: vec![None; p.usages.len()] };
    let mut unsolved = Vec::new();
    let mut notes = Vec::new();
    let mut needed = vec![false; p.usages.len()];
    let mut lost = |decision: Decision, label: &str, reason: &str| {
        unsolved.push(Unsolved { decision, label: label.to_string(), reason: reason.to_string() })
    };

    // the branch points, named so as not to clash with the level's names
    let mut taken: Vec<String> = p.names.iter().chain(refs.names.iter().flatten()).cloned().collect();
    let mut branch_names: Vec<Option<String>> = Vec::new();
    for (index, b) in solution.branches.iter().enumerate() {
        if coords(b.at).is_err() {
            lost(Decision::Branch { index }, &format!("branch point {index}"), NOT_A_NUMBER);
            branch_names.push(None);
            continue;
        }
        let mut n = 1;
        while taken.contains(&format!("rt{n}")) {
            n += 1;
        }
        taken.push(format!("rt{n}"));
        branch_names.push(Some(format!("rt{n}")));
    }

    // positions and orientations, a text keeping the axes that the file places it on by what it is
    // attached to, where the solution has it there (13.3)
    let mut placements: Vec<(Reference, [Option<f64>; 2], Option<Orient>)> = Vec::new();
    let mut sorted: Vec<&Placement> = solution.placements.iter().collect();
    sorted.sort_by_key(|pl| pl.object);
    for pl in sorted {
        let orientation = Decision::Orientation { object: pl.object };
        // the position on the axes that `on` marks: one alone, or both
        let position = |on: [bool; 2]| Decision::Position {
            object: pl.object,
            axis: match on {
                [true, false] => Some(Axis::X),
                [false, true] => Some(Axis::Y),
                _ => None,
            },
        };
        let given = [pl.x, pl.y];
        let Some(o) = p.objects.get(pl.object) else {
            let label = format!("object {}", pl.object);
            if given.iter().any(Option::is_some) {
                lost(position(given.map(|v| v.is_some())), &label, NOT_IN_PROBLEM);
            }
            if pl.orient.is_some() {
                lost(orientation, &label, NOT_IN_PROBLEM);
            }
            continue;
        };
        let mut at = given;
        if matches!(o.position, Position::At(_)) && at.iter().any(Option::is_some) {
            lost(position(at.map(|v| v.is_some())), &o.label, AT_POINT);
            at = [None, None];
        }
        // a coordinate that is not a number, axis by axis
        let bad = at.map(|v| v.is_some_and(|v| !v.is_finite()));
        if bad.iter().any(|b| *b) {
            lost(position(bad), &o.label, NOT_A_NUMBER);
            at = [at[0].filter(|_| !bad[0]), at[1].filter(|_| !bad[1])];
        }
        let mut orient = pl.orient;
        if orient.is_some() && o.orient.is_none() {
            lost(orientation, &o.label, NO_ORIENTATION);
            orient = None;
        }
        let at = [(Axis::X, at[0]), (Axis::Y, at[1])]
            .map(|(axis, v)| v.filter(|&v| !follows(p, solution, o, pl.object, axis, v)));
        let placed = at.iter().any(Option::is_some);
        if !placed && orient.is_none() {
            continue;
        }
        match refs.reach(&o.reference) {
            Ok(root) => {
                if let Some(u) = root {
                    needed[u] = true;
                }
                placements.push((o.reference.clone(), at, orient));
            }
            Err(why) => {
                let keyless = o.text.as_ref().is_some_and(|t| t.key.is_none());
                let why = if why == ANONYMOUS && o.generated && keyless { RULE_TEXT } else { why };
                // what the file fixes, where the solution has it, a viewer draws there all the same:
                // unsolved are the axes given that the file does not fix so
                let unfixed =
                    [(Axis::X, at[0]), (Axis::Y, at[1])].map(|(axis, v)| v.is_some_and(|v| !fixed_at(o, axis, v)));
                let fixed_orient =
                    orient.is_none_or(|c| matches!(o.orient, Some(OrientSlot::Fixed { code, .. }) if code == c));
                if unfixed.iter().any(|u| *u) {
                    lost(position(unfixed), &o.label, why);
                }
                if orient.is_some() && !fixed_orient {
                    lost(orientation, &o.label, why);
                }
            }
        }
    }

    // the wires routed for ROUTE: new wires
    let mut routed: Vec<(&RouteIntent, Vec<Piece>)> = Vec::new();
    for (index, rw) in solution.wires.iter().enumerate() {
        let decision = Decision::RoutedWire { index };
        let Some(route) = p.routes.get(rw.route) else {
            lost(decision, &format!("a wire of ROUTE {}", rw.route), NOT_IN_PROBLEM);
            continue;
        };
        let label = format!("a wire of the ROUTE of &{}", written_name(&route.net));
        match refs.wire(&rw.points, &branch_names) {
            Ok((pieces, roots)) => {
                for u in roots {
                    needed[u] = true;
                }
                routed.push((route, pieces));
            }
            Err(why) => lost(decision, &label, &why),
        }
    }

    // the routes of the wires written in RNL: WIRE given the wire (the draft 0.3, 13.1)
    let mut rewrites: Vec<(Reference, Vec<Piece>)> = Vec::new();
    let mut sorted: Vec<&WireRoute> = solution.wire_routes.iter().collect();
    sorted.sort_by_key(|r| r.wire);
    for wr in sorted {
        let decision = Decision::WireRoute { wire: wr.wire };
        let Some(wire) = p.wires.get(wr.wire) else {
            lost(decision, &format!("wire {}", wr.wire), NOT_IN_PROBLEM);
            continue;
        };
        if !refs.numbers {
            lost(decision, &wire.label, NO_REWRITE);
            continue;
        }
        let root = match refs.reach(&wire.reference) {
            Ok(root) => root,
            Err(why) => {
                lost(decision, &wire.label, why);
                continue;
            }
        };
        let (pieces, roots) = match refs.wire(&wr.points, &branch_names) {
            Ok(found) => found,
            Err(why) => {
                lost(decision, &wire.label, &why);
                continue;
            }
        };
        for u in root.into_iter().chain(roots) {
            needed[u] = true;
        }
        if wire.mode != "auto" {
            notes.push(format!(
                "the route of {}: the wire is not \"auto\" but {}, so its route was not the solver's to choose (13.1); written as given",
                wire.label,
                written_string(&wire.mode)
            ));
        }
        if !keeps_points(wire, &wr.points) {
            notes.push(format!(
                "the route of {} does not keep the points of the wire in order with bends only between them, as a solver should (13.1); written as given",
                wire.label
            ));
        }
        rewrites.push((wire.reference.clone(), pieces));
    }

    // the usages written out first, in order: the objects that each makes are numbered after those
    // of the authored records and of the usages before it (5.9)
    let mut next = p.authored_numbers;
    for (k, u) in p.usages.iter().enumerate() {
        if needed[k] && refs.names[k].is_some() {
            refs.offsets[k] = Some(next);
            next += u.written_numbers;
        }
    }

    let mut statements = Vec::new();
    for (k, u) in p.usages.iter().enumerate() {
        if let (true, Some(name)) = (needed[k], &refs.names[k]) {
            statements.push(usage_statement(u, name, p.follows_0_3()));
        }
    }
    for (b, name) in solution.branches.iter().zip(&branch_names) {
        if let (Some(name), Ok(xy)) = (name, coords(b.at)) {
            statements.push(format!("%{name} = {xy}"));
        }
    }
    for (r, at, orient) in &placements {
        let mut parts = Vec::new();
        match at {
            [Some(x), Some(y)] => parts.push(format!("@PLACE({}, {})", fmt_num(*x), fmt_num(*y))),
            [Some(x), None] => parts.push(format!("@PLACE(x={})", fmt_num(*x))),
            [None, Some(y)] => parts.push(format!("@PLACE(y={})", fmt_num(*y))),
            [None, None] => {}
        }
        if let Some(code) = orient {
            parts.push(format!("@ORIENT(\"{}\")", code.as_str()));
        }
        statements.push(format!("{} {}", refs.text(r), parts.join(" => ")));
    }
    let list = |pieces: &[Piece]| pieces.iter().map(|q| refs.piece(q)).collect::<Vec<_>>().join(" ");
    for (route, pieces) in &routed {
        let mut params = vec!["mode=\"straight\"".to_string()];
        params.extend(route.style.iter().map(|(k, v)| format!("{}={}", k, written_lit(v))));
        statements.push(format!("&{} @WIRE({}) {{{}}}", written_name(&route.net), params.join(", "), list(pieces)));
    }
    for (r, pieces) in &rewrites {
        statements.push(format!("{} @WIRE(mode=\"straight\") {{{}}}", refs.text(r), list(pieces)));
    }
    for (b, name) in solution.branches.iter().zip(&branch_names) {
        if let (true, Some(name)) = (b.junction, name) {
            statements.push(format!("$JUNCTION @ATTACH %{name}"));
        }
    }

    let head = format!(
        "@RNL(type=\"generated\", priority=\"high\", author={}, hash={})",
        written_string(author),
        written_string(hash)
    );
    let record = if statements.is_empty() {
        format!("{head} {{}}")
    } else {
        format!("{head} {{\n  {}\n}}", statements.join(";\n  "))
    };
    Written { record, unsolved, notes }
}

/// How deep the points that points are placed from are followed.
const MAX_DEPTH: usize = 16;

/// A coordinate of a point on an axis.
fn along((x, y): Pt, axis: Axis) -> f64 {
    match axis {
        Axis::X => x,
        Axis::Y => y,
    }
}

/// The coordinate that a slot fixes (11.1).
fn fixed(slot: &Slot) -> Option<f64> {
    match slot {
        Slot::Fixed { value, .. } => Some(*value),
        Slot::Free => None,
    }
}

/// The orientation of an object once the record is read: the solution's, or the one an ORIENT of
/// the file gives, or `"R0"` (11.2).
fn orientation(p: &LevelProblem, s: &Solution, k: usize) -> Orient {
    let solved = s.placements.iter().find(|pl| pl.object == k).and_then(|pl| pl.orient);
    let written = match p.objects.get(k).and_then(|o| o.orient.as_ref()) {
        Some(OrientSlot::Fixed { code, .. }) => Some(*code),
        _ => None,
    };
    solved.or(written).unwrap_or(Orient::R0)
}

/// Where a point of the problem is on an axis once the record is read (14.1): an object where the
/// solution places it, or else where the facts of the file fix it, a text that its owner's symbol
/// places with its owner (13.3), an object drawn at the point of its main object there (12.1); a
/// pin turned with its object (9.2.4); the middle of the extent of a set (6.5). None for a point
/// that neither the solution nor the file places.
fn coordinate(p: &LevelProblem, s: &Solution, r: &PointRef, axis: Axis, depth: usize) -> Option<f64> {
    if depth > MAX_DEPTH {
        return None;
    }
    match r {
        PointRef::Origin(k) => {
            let o = p.objects.get(*k)?;
            if let Position::At(main) = &o.position {
                return coordinate(p, s, main, axis, depth + 1);
            }
            if let Some(v) = s.placements.iter().find(|pl| pl.object == *k).and_then(|pl| pl.on(axis)) {
                return Some(v);
            }
            match &o.position {
                Position::Own { x, y } => fixed(if axis == Axis::X { x } else { y }),
                Position::InSymbol { replaced: Some((a, slot)), .. } if *a == axis => fixed(slot),
                Position::InSymbol { owner, local, .. } => {
                    let base = coordinate(p, s, &PointRef::Origin(*owner), axis, depth + 1)?;
                    Some(base + along(orientation(p, s, *owner).apply(*local), axis))
                }
                Position::At(_) => None,
            }
        }
        PointRef::Pin { object, pin, .. } => {
            let o = p.objects.get(*object)?;
            let pos = p.symbols.get(o.symbol?)?.pins.get(*pin)?.pos;
            let base = coordinate(p, s, &PointRef::Origin(*object), axis, depth + 1)?;
            Some(base + along(orientation(p, s, *object).apply(pos), axis))
        }
        PointRef::Centre(members) => {
            let mut values = members.iter().map(|m| coordinate(p, s, m, axis, depth + 1));
            let first = values.next()??;
            let (lo, hi) = values.try_fold((first, first), |(lo, hi), v| v.map(|v| (lo.min(v), hi.max(v))))?;
            Some((lo + hi) / 2.0)
        }
    }
}

/// Whether the solution has object `k`, `o`, at `v` on an axis on which the file places it by
/// what it is attached to (13.3): a text by its offset on that axis, or by its owner's symbol on an
/// axis that the scene does not replace (9.2.5), within [`TOL`]. The record then writes no PLACE
/// there, which would replace it. A text attached to a point that neither the solution nor the
/// file places follows it all the same.
fn follows(p: &LevelProblem, s: &Solution, o: &ProblemObject, k: usize, axis: Axis, v: f64) -> bool {
    let offset = p.facts.iter().find_map(|f| match &f.kind {
        FactKind::Offset { text, from, axis: a, value } if *text == k && *a == axis => Some((from, *value)),
        _ => None,
    });
    let target = match (&o.position, offset) {
        (_, Some((from, value))) => coordinate(p, s, from, axis, 0).map(|c| c + value),
        (position @ Position::InSymbol { owner, local, .. }, None) if position.from_symbol(axis) => {
            let base = coordinate(p, s, &PointRef::Origin(*owner), axis, 0);
            base.map(|c| c + along(orientation(p, s, *owner).apply(*local), axis))
        }
        _ => return false,
    };
    target.is_none_or(|t| (v - t).abs() <= TOL)
}

/// Whether a fact of the file fixes the coordinate of an object on an axis at `v`, within [`TOL`]
/// (11.1): its own coordinate there, or that of a text on the axis that the scene gives it (13.3).
fn fixed_at(o: &ProblemObject, axis: Axis, v: f64) -> bool {
    let slot = match &o.position {
        Position::Own { x, y } => Some(if axis == Axis::X { x } else { y }),
        Position::InSymbol { replaced: Some((a, slot)), .. } if *a == axis => Some(slot),
        _ => None,
    };
    slot.and_then(fixed).is_some_and(|value| (value - v).abs() <= TOL)
}

/// The record of the solver of a level, finished: `record` is what [`write_record`] gives for the
/// problem `p`. When it uses a number, in a level of a file of the draft 0.3, the record with
/// `numbering` in its head, the check of its numbers (5.9); otherwise the record as it is: a
/// record that uses no number needs no `numbering` (5.9), and a level of 0.2 has no numbers.
///
/// The check covers the objects of the authored records of the level and the record's own: the
/// coordinates in its wires, the bodies of the usages it writes out, its junctions. So the document
/// is read again with the record in place of the level's records of the solver, and the check is
/// [`level_check`] of the level, the check of the text
/// as written. `doc` is the host document that the problem was read from, such as
/// [`crate::Interpretation::doc`]; `files` and `options` are what it was read with, packages
/// included (`rnl_view::finish_record` adds the bundled ones).
pub fn finish_record(
    doc: &HostDocument,
    files: &dyn Provider,
    options: &Options,
    p: &LevelProblem,
    record: &str,
) -> String {
    let ast = parse_record_with(record, true);
    let uses_numbers = ast.body.as_ref().is_some_and(|b| b.members.iter().any(|m| uses_number(m)));
    if !p.follows_0_3() || !uses_numbers {
        return record.to_string();
    }
    let mut doc = doc.clone();
    let Some(level) = doc.levels.get_mut(p.level) else { return record.to_string() };
    let file = level.file.clone();
    level.records.retain(|r| r.file != file || record_type(r) != "generated");
    level.records.push(placed(record, &file, level.end_line.max(1)));
    let options = Options { generated: true, problems: false, ..options.clone() };
    let read = crate::interpret(doc, files, &options, Sink::new());
    match level_check(&read, p.level) {
        Some(check) => with_numbering(record, check),
        None => record.to_string(),
    }
}

/// A record as a host adapter would cut it out of the lines from `line` of `file` on.
fn placed(text: &str, file: &str, line: u32) -> Record {
    let mut segs = Vec::new();
    let mut off = 0;
    for (k, l) in text.split('\n').enumerate() {
        segs.push(Seg { off, file: file.to_string(), line: line + k as u32, col: 0 });
        off += l.chars().count() + 1;
    }
    let last = line + segs.len().saturating_sub(1) as u32;
    Record { text: text.to_string(), segs, lines: (line, last), file: file.to_string() }
}

/// A record with `numbering="…"` added at the end of its head (2.8, 5.9).
fn with_numbering(record: &str, check: &str) -> String {
    let field = format!("numbering={}", written_string(check));
    let chars: Vec<char> = record.chars().collect();
    let head = "@RNL(".chars().count();
    if !record.starts_with("@RNL(") {
        let rest = record.strip_prefix("@RNL").unwrap_or(record);
        return format!("@RNL({field}){rest}");
    }
    // the bracket that closes the head, outside strings, where `""` is a quote (3.7)
    let mut k = head;
    let mut in_string = false;
    while k < chars.len() {
        match chars[k] {
            '"' if in_string && chars.get(k + 1) == Some(&'"') => k += 1,
            '"' => in_string = !in_string,
            ')' if !in_string => break,
            _ => {}
        }
        k += 1;
    }
    let before: String = chars[..k].iter().collect();
    let after: String = chars[k..].iter().collect();
    let sep = if before.trim_end().ends_with('(') { "" } else { ", " };
    format!("{}{sep}{field}{after}", before.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("#M1:s"), "M1_s");
        assert_eq!(slug("&\"D<3>\""), "D_3");
        assert_eq!(slug("gnd_stub"), "gnd_stub");
        assert_eq!(slug("{#M1 #M2}"), "M1_M2");
    }

    #[test]
    fn numbering_goes_at_the_end_of_the_head() {
        let check = "0123456789abcdef";
        assert_eq!(
            with_numbering("@RNL(type=\"generated\", author=\"a (b) \"\"c)\"\"\") {}", check),
            "@RNL(type=\"generated\", author=\"a (b) \"\"c)\"\"\", numbering=\"0123456789abcdef\") {}"
        );
        assert_eq!(with_numbering("@RNL() {\n  #R1\n}", check), "@RNL(numbering=\"0123456789abcdef\") {\n  #R1\n}");
        assert_eq!(with_numbering("@RNL {}", check), "@RNL(numbering=\"0123456789abcdef\") {}");
    }

    #[test]
    fn a_record_placed_maps_its_lines() {
        let r = placed("@RNL {\n  #R1 @PLACE(1, 2)\n}", "t.sp", 7);
        assert_eq!(r.lines, (7, 9));
        assert_eq!((r.loc(9).line, r.loc(9).col), (8, 3));
    }
}
