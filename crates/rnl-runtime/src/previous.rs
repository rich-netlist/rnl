//! The previous result of the solver of a level, as hints for a warm start (the draft 0.3, 2.8,
//! 5.9, 14.8, 17).
//!
//! A solver reads a level without its records of the solver (2.8): they do not constrain the next
//! result, and how else the solver uses them is up to it. [`Previous`] gives it the decisions of
//! the level's record of the solver, mapped onto its [`LevelProblem`]: where the record puts the
//! objects of the problem and how it turns them, the routes it gives the wires written in RNL, and
//! the wires it writes for ROUTE, with whether the record is still up to date. [`previous`] maps
//! them from the interpretation with the records of the solver, which a viewer has already, so
//! nothing is read twice for it; [`read_previous`] makes both readings, for a solver that starts
//! from the files.
//!
//! The mapping goes by the reference that each object and wire of the problem has
//! ([`Reference`]): a name as written; a number of the authored records, which is the same with
//! and without the records of the solver (5.9); through a usage that a rule makes, the name that
//! [`generated_names`] gives it, under which the record writes it out (10.5), and for the objects
//! without a name in its body ([`Reference::GeneratedNumber`]), their place among the objects of
//! that body. What maps to nothing is left out, and so is what the record does not hold: a
//! statement that uses a number when the record's `numbering` is out of date takes no effect.

use std::collections::HashMap;

use crate::diag::Sink;
use crate::geom::{Orient, Pt};
use crate::host::{HostDocument, Record};
use crate::problem::{same_input, written_name, LevelProblem, Reference};
use crate::solution::{generated_names, Placement};
use crate::source::Provider;
use crate::writeback::{field, record_type};
use crate::{Interpretation, Options};

/// The decisions of the record of the solver of a level, mapped onto the problem of the level, the
/// reading without the records of the solver: hints for a warm start. Objects and wires are
/// referred to by their indexes in the problem's lists.
#[derive(Clone, Debug, PartialEq)]
pub struct Previous {
    /// The `hash` in the record head, as written (2.8, 17).
    pub hash: Option<String>,
    /// Whether the input digest of `hash` is the problem's ([`same_input`]); None for a record
    /// without a `sig1` hash, which is not compared (17).
    pub same_input: Option<bool>,
    /// Whether its numbers hold (5.9); None when it uses no number. False when its `numbering` is
    /// missing or not the check of the level: then its statements that use a number take no
    /// effect, and give nothing here.
    pub numbering: Option<bool>,
    /// The objects of the problem that the record places or turns, in the order of the objects:
    /// the position, when the record gives both coordinates, and the orientation it gives.
    pub placements: Vec<Placement>,
    /// The wires of the problem that the record rewrites with WIRE given the wire (13.1), in the
    /// order of the wires, with the routes it gives them.
    pub wire_routes: Vec<PreviousRoute>,
    /// The wires that the record writes itself, as a solver does for ROUTE (13.7), in the order
    /// they are made.
    pub wires: Vec<PreviousWire>,
}

/// The route that a record of the solver gives a wire of the problem.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviousRoute {
    /// The wire, by its index in [`LevelProblem::wires`].
    pub wire: usize,
    /// The route in the scene, x to the right and y up, as the record's WIRE draws it in the level
    /// read with the record: the pins and points it passes, and the bends between them.
    pub points: Vec<Pt>,
}

/// A wire that a record of the solver writes itself.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviousWire {
    /// The nets it is on, named as [`crate::problem::RouteIntent::net`] names them.
    pub nets: Vec<String>,
    /// Its points in the scene, as the record's WIRE draws them in the level read with the record.
    pub points: Vec<Pt>,
}

/// The previous result of the level of `p`, the problem a solver reads, from `with`: the same
/// document read with the records of the solver and with [`crate::Options::problems`], such as the
/// reading a viewer shows with the problems built as well, and with [`crate::Options::draft`] for
/// a file of the draft. None when the level has no record of the solver. Read without `problems`,
/// `with` gives the hash and whether the numbers hold, but no decision.
pub fn previous(p: &LevelProblem, with: &Interpretation) -> Option<Previous> {
    let rec = solver_record(&with.doc, p.level)?;
    let hash = field(rec, "hash");
    let mut out = Previous {
        same_input: hash.as_deref().and_then(|h| same_input(h, p)),
        hash,
        numbering: numbers_hold(with, p.level, rec),
        placements: Vec::new(),
        wire_routes: Vec::new(),
        wires: Vec::new(),
    };
    let Some(result) = with.results.iter().find(|r| r.level == p.level) else { return Some(out) };
    let names = generated_names(p);
    let mut objects: HashMap<Key, usize> = HashMap::new();
    for (k, o) in p.objects.iter().enumerate() {
        if let Some(key) = key(p, &names, &o.reference) {
            objects.entry(key).or_insert(k);
        }
    }
    let mut wires: HashMap<Key, usize> = HashMap::new();
    for (k, w) in p.wires.iter().enumerate() {
        if let Some(key) = key(p, &names, &w.reference) {
            wires.entry(key).or_insert(k);
        }
    }
    for given in &result.placed {
        let Some(&object) = objects.get(&given.key) else { continue };
        let at = given.x.zip(given.y);
        let orient = given.orient.filter(|_| p.objects[object].orient.is_some());
        if at.is_some() || orient.is_some() {
            out.placements.push(Placement { object, at, orient });
        }
    }
    out.placements.sort_by_key(|pl| pl.object);
    for (key, points) in &result.routes {
        if let Some(&wire) = wires.get(key) {
            out.wire_routes.push(PreviousRoute { wire, points: points.clone() });
        }
    }
    out.wire_routes.sort_by_key(|r| r.wire);
    out.wires =
        result.wires.iter().map(|(nets, points)| PreviousWire { nets: nets.clone(), points: points.clone() }).collect();
    Some(out)
}

/// Both readings of a host document that a solver makes (2.8): without the records of the
/// solver, whose [`Interpretation::problems`] it solves, and with them, which gives the previous
/// result of each level ([`previous`]), in the order of the problems. `options` are those of both
/// readings, such as `std` and `draft`; `generated` and `problems` are set here. `sink` holds what
/// the host adapter reported, if anything. A document without records of the solver is read once.
pub fn read_previous(
    doc: HostDocument,
    files: &dyn Provider,
    options: &Options,
    sink: Sink,
) -> (Interpretation, Vec<Option<Previous>>) {
    let records = doc.levels.iter().any(|l| l.records.iter().any(|r| record_type(r) == "generated"));
    let without = Options { generated: false, problems: true, ..options.clone() };
    if !records {
        let r = crate::interpret(doc, files, &without, sink);
        let none = r.problems.iter().map(|_| None).collect();
        return (r, none);
    }
    let r = crate::interpret(doc.clone(), files, &without, sink.clone());
    let with = crate::interpret(doc, files, &Options { generated: true, problems: true, ..options.clone() }, sink);
    let previous = r.problems.iter().map(|p| previous(p, &with)).collect();
    (r, previous)
}

/// The record of the solver of a level: its last record with `type="generated"`, without `scope=`
/// (2.6, 2.8).
fn solver_record(doc: &HostDocument, level: usize) -> Option<&Record> {
    let records = &doc.levels.get(level)?.records;
    records.iter().rev().find(|r| record_type(r) == "generated" && field(r, "scope").is_none())
}

/// Whether the numbers of a record of the solver hold, as `with` read them; None when it uses no
/// number, or when `with` was read without the draft, which reads no number.
fn numbers_hold(with: &Interpretation, level: usize, rec: &Record) -> Option<bool> {
    let at = rec.loc(0);
    let numbering = with.numbering.iter().find(|n| n.level == level)?;
    let r = numbering.records.iter().find(|r| r.file == at.file && r.line == at.line)?;
    r.uses_numbers.then(|| r.holds())
}

/// How the reading with a record of the solver finds an object or a wire of the problem read
/// without it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Key {
    /// A name as written, such as `#M1`, `%g0` or `%gnd_stub_M1_s::g`.
    Name(String),
    /// A number of the authored records, the same in both readings (5.9), and its path as written.
    Number(usize, String),
    /// An object without a name in the body of a usage that the record writes out: the usage's
    /// name as written, the object's place among those of the body from 1, its path as written.
    Body(String, usize, String),
}

/// What the last record of the solver of a level gives, as the reading with it has it, by
/// [`Key`]. The interpreter makes one for each such level when it builds the problems.
#[derive(Clone, Debug, Default)]
pub(crate) struct RecordResult {
    /// The host level.
    pub level: usize,
    /// The objects with the coordinates and the orientation that the record gives them.
    pub placed: Vec<Given>,
    /// The wires that the record rewrites, with their routes as drawn.
    pub routes: Vec<(Key, Vec<Pt>)>,
    /// The wires that the record writes itself: their nets and their points as drawn.
    pub wires: Vec<(Vec<String>, Vec<Pt>)>,
}

/// The coordinates and the orientation that a record of the solver gives an object.
#[derive(Clone, Debug)]
pub(crate) struct Given {
    pub key: Key,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub orient: Option<Orient>,
}

/// The key of a reference of a problem read without the records of the solver.
fn key(p: &LevelProblem, names: &[Option<String>], r: &Reference) -> Option<Key> {
    let written = |usage: usize| names.get(usage).cloned().flatten().map(|n| format!("%{}", written_name(&n)));
    match r {
        Reference::Name(n) => Some(Key::Name(n.clone())),
        Reference::Number { number, path } if *number <= p.authored_numbers => {
            Some(Key::Number(*number, path.concat()))
        }
        Reference::Generated { usage, path } => Some(Key::Name(format!("{}{}", written(*usage)?, path.concat()))),
        Reference::GeneratedNumber { usage, index, path } => Some(Key::Body(written(*usage)?, *index, path.concat())),
        Reference::Number { .. } | Reference::Anonymous => None,
    }
}
