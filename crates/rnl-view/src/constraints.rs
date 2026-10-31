//! The constraints of each level after expansion, as a viewer lists them without a solver
//! ([`crate::BuildOptions::constraints`]): the facts of the [`LevelProblem`], the solver's input,
//! with each object it refers to tied to the Document built with them. Experimental: these shapes
//! may change in any release.
//!
//! The facts are read as a solver reads the level, without its records of the solver (2.8), and
//! the Document is drawn with them, so an object is found in the Document by its reference, as
//! [`rnl_runtime::previous::previous`] finds it.

use std::collections::HashMap;

use rnl_runtime::model::{quoted, LevelView};
use rnl_runtime::previous::document_ids;
use rnl_runtime::problem::{
    fact_statement, input_digest, FactKind, LevelProblem, PointRef, ProblemFact, Reference, RouteIntent,
};
use rnl_runtime::Interpretation;
use serde::Serialize;

use crate::{Level, Object};

/// The constraints of a level after expansion, what a solver takes in. **Experimental: not promised
/// to stay as it is.** Facts with priority `"disabled"` and those that failed as they were read are
/// not here: a usage turned off is in `Level.usages[]` with its `state`, and what failed is in
/// `Document.diagnostics` and `SourceFile.failed`.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct LevelConstraints {
    /// The digest of the solver's input, the first part of the `sig2` hash (17). A solve's
    /// `report.levels[].input` being the same says that `report.levels[].facts[k]` is `facts[k]`.
    pub input: String,
    /// The objects that `facts`, `routes` and `groups` refer to by index.
    pub objects: Vec<ConstraintObject>,
    /// The facts in the order of 8.5: the index of each is its rank.
    pub facts: Vec<Constraint>,
    /// The usages made in the level, which `Constraint.chain` and `ConstraintUsage.parent` refer to
    /// by index.
    pub usages: Vec<ConstraintUsage>,
    /// The nets to draw as wires (13.7).
    pub routes: Vec<ConstraintRoute>,
    /// The sets given GROUP that are in effect (11.5).
    pub groups: Vec<ConstraintGroup>,
}

/// An object as the constraints refer to it, and where the Document has it.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ConstraintObject {
    /// "instance", "bundle", "usage", "point" or "text".
    pub kind: &'static str,
    /// As diagnostics show it.
    pub label: String,
    /// How the level's own text refers to it.
    pub reference: ConstraintReference,
    /// Made by a rule (10.5), alone or as part of a generated usage.
    pub generated: bool,
    /// Its id in the Document's reading, as `Usage.id`: `#M1`, `%g0`, or for one without a name
    /// where it was made, `@file:line:col`. None when nothing there is known to be this object.
    pub id: Option<String>,
    /// What it is in `Document.objects`: an instance, the instances of a bundle, or a usage; empty
    /// for a point, a text, and an object without an id.
    pub objects: Vec<usize>,
}

/// How the level's own text refers to something: what it writes, a usage made by a rule and the
/// path through it, a number in a file of the draft 0.3 (5.9), or nothing.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(untagged)]
pub enum ConstraintReference {
    Written(String),
    Generated { generated_by: usize, path: String },
    Number { number: usize, path: String },
    GeneratedNumber { generated_by: usize, number: usize, path: String },
    None,
}

/// A point that a fact relates: the origin of an object, the pin of one of its terminals, or the
/// centre of a set. `origin` and `object` are indexes in `LevelConstraints.objects`.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(untagged)]
pub enum ConstraintPoint {
    Origin { origin: usize },
    Pin { object: usize, pin: usize, terminal: String },
    Centre { centre: Vec<ConstraintPoint> },
}

/// What a fact says, by its kind. Objects are indexes in `LevelConstraints.objects`.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind")]
pub enum ConstraintKind {
    #[serde(rename = "PLACE-x")]
    PlaceX { at: ConstraintPoint, value: f64 },
    #[serde(rename = "PLACE-y")]
    PlaceY { at: ConstraintPoint, value: f64 },
    #[serde(rename = "POINT-x")]
    PointX { point: usize, value: f64 },
    #[serde(rename = "POINT-y")]
    PointY { point: usize, value: f64 },
    #[serde(rename = "TEXT-dx")]
    TextDx { text: usize, from: ConstraintPoint, value: f64 },
    #[serde(rename = "TEXT-dy")]
    TextDy { text: usize, from: ConstraintPoint, value: f64 },
    #[serde(rename = "ORIENT")]
    Orient { object: usize, code: &'static str },
    #[serde(rename = "ALIGN")]
    Align { axis: &'static str, members: Vec<ConstraintPoint> },
    #[serde(rename = "ORDER")]
    Order { axis: &'static str, sense: &'static str, members: Vec<ConstraintPoint> },
    #[serde(rename = "DISTRIBUTE")]
    Distribute { axis: &'static str, members: Vec<ConstraintPoint> },
    #[serde(rename = "DISTANCE")]
    Distance {
        axis: &'static str,
        sense: &'static str,
        min: Option<f64>,
        max: Option<f64>,
        from: Vec<ConstraintPoint>,
        to: Vec<ConstraintPoint>,
    },
    #[serde(rename = "ORIENT-RELATION")]
    OrientRelation { relation: &'static str, from: Vec<usize>, to: Vec<usize> },
}

/// A constraint after expansion, as `rnl problem` prints it in `facts`, with `what`.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Constraint {
    #[serde(flatten)]
    pub fact: ConstraintKind,
    /// The effective priority (14.8).
    pub priority: &'static str,
    /// As diagnostics show it, such as `%c1` or `the x of #M1 = 3`.
    pub label: String,
    /// What it says, in one line, such as `ALIGN x {#M1 #M2}`.
    pub what: String,
    /// For a constraint with a handle: how to refer to it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub reference: Option<ConstraintReference>,
    /// Where the statement is written; none for the coordinates of a point.
    pub source: Option<ConstraintSource>,
    /// The usages whose expansion made it, outermost first, by index in `LevelConstraints.usages`.
    pub chain: Vec<usize>,
}

/// A place in a file, as `Document.files[].path` names the file.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ConstraintSource {
    pub file: String,
    pub line: u32,
    pub col: u32,
}

/// A usage that a constraint may come from.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ConstraintUsage {
    /// The definition, such as `$GND_STUB`.
    pub definition: String,
    pub label: String,
    pub reference: ConstraintReference,
    /// Made by a rule (10.5), alone or in the expansion of a generated usage.
    pub generated: bool,
    /// The usage whose expansion made it, by index in `LevelConstraints.usages`.
    pub parent: Option<usize>,
    /// It in `Document.objects`, as `Level.usages[].object`, with or without a symbol. None when
    /// nothing in the Document's reading is known to be this usage.
    pub object: Option<usize>,
}

/// A net to draw as wires (13.7).
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ConstraintRoute {
    pub net: String,
    /// A hint for the solver, with the values of WIRE.
    pub mode: String,
    /// The pins to join.
    pub pins: Vec<ConstraintPoint>,
}

/// A set given GROUP (11.5).
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ConstraintGroup {
    pub label: String,
    /// Indexes in `LevelConstraints.objects`, nested sets followed down.
    pub members: Vec<usize>,
}

fn reference(r: &Reference, numbers: bool) -> ConstraintReference {
    match r {
        Reference::Name(n) => ConstraintReference::Written(n.clone()),
        Reference::Generated { usage, path } => {
            ConstraintReference::Generated { generated_by: *usage, path: path.concat() }
        }
        Reference::Number { number, path } if numbers => {
            ConstraintReference::Number { number: *number, path: path.concat() }
        }
        Reference::GeneratedNumber { usage, index, path } if numbers => {
            ConstraintReference::GeneratedNumber { generated_by: *usage, number: *index, path: path.concat() }
        }
        Reference::Number { .. } | Reference::GeneratedNumber { .. } | Reference::Anonymous => {
            ConstraintReference::None
        }
    }
}

fn point(r: &PointRef) -> ConstraintPoint {
    match r {
        PointRef::Origin(k) => ConstraintPoint::Origin { origin: *k },
        PointRef::Pin { object, pin, terminal } => {
            ConstraintPoint::Pin { object: *object, pin: *pin, terminal: terminal.clone() }
        }
        PointRef::Centre(points) => ConstraintPoint::Centre { centre: points_of(points) },
    }
}

fn points_of(rs: &[PointRef]) -> Vec<ConstraintPoint> {
    rs.iter().map(point).collect()
}

fn kind(f: &ProblemFact) -> ConstraintKind {
    use rnl_runtime::problem::Axis::{X, Y};
    match &f.kind {
        FactKind::Place { at, axis: X, value } => ConstraintKind::PlaceX { at: point(at), value: *value },
        FactKind::Place { at, axis: Y, value } => ConstraintKind::PlaceY { at: point(at), value: *value },
        FactKind::PointCoordinate { point, axis: X, value } => ConstraintKind::PointX { point: *point, value: *value },
        FactKind::PointCoordinate { point, axis: Y, value } => ConstraintKind::PointY { point: *point, value: *value },
        FactKind::Offset { text, from, axis: X, value } => {
            ConstraintKind::TextDx { text: *text, from: point(from), value: *value }
        }
        FactKind::Offset { text, from, axis: Y, value } => {
            ConstraintKind::TextDy { text: *text, from: point(from), value: *value }
        }
        FactKind::Orient { object, code } => ConstraintKind::Orient { object: *object, code: code.as_str() },
        FactKind::Align { axis, members } => ConstraintKind::Align { axis: axis.as_str(), members: points_of(members) },
        FactKind::Order { axis, sense, members } => {
            ConstraintKind::Order { axis: axis.as_str(), sense: sense.as_str(), members: points_of(members) }
        }
        FactKind::Distribute { axis, members } => {
            ConstraintKind::Distribute { axis: axis.as_str(), members: points_of(members) }
        }
        FactKind::Distance { axis, sense, min, max, from, to } => ConstraintKind::Distance {
            axis: axis.as_str(),
            sense: sense.as_str(),
            min: *min,
            max: *max,
            from: points_of(from),
            to: points_of(to),
        },
        FactKind::OrientRelation { relation, from, to } => {
            ConstraintKind::OrientRelation { relation: relation.as_str(), from: from.clone(), to: to.clone() }
        }
    }
}

/// The fact of rank `k` of a problem. `rnl problem` prints these, without `what`.
pub fn constraint(p: &LevelProblem, k: usize) -> Constraint {
    let f = &p.facts[k];
    let numbers = p.follows_0_3();
    Constraint {
        fact: kind(f),
        priority: f.priority.as_str(),
        label: f.label.clone(),
        what: fact_statement(p, k),
        reference: match &f.reference {
            Some(Reference::Number { .. } | Reference::GeneratedNumber { .. }) if !numbers => None,
            Some(r) => Some(reference(r, numbers)),
            None => None,
        },
        source: f.source.as_ref().map(|l| ConstraintSource { file: l.file.clone(), line: l.line, col: l.col }),
        chain: f.chain.clone(),
    }
}

fn route(r: &RouteIntent) -> ConstraintRoute {
    ConstraintRoute { net: r.net.clone(), mode: r.mode.clone(), pins: points_of(&r.pins) }
}

/// The constraints of the level of `p`, the problem a solver reads, tied to `level`, its level in
/// the Document built from `shown`, whose objects are `objects`; `lv` is that level in `shown`.
pub(crate) fn level_constraints(
    p: &LevelProblem,
    shown: &Interpretation,
    lv: &LevelView,
    level: &Level,
    objects: &[Object],
) -> LevelConstraints {
    let numbers = p.follows_0_3();
    let ids = document_ids(p, shown);
    let usages: HashMap<&str, usize> = level.usages.iter().map(|u| (u.id.as_str(), u.object)).collect();
    let instances: HashMap<String, usize> = objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.kind == "instance" && o.level == lv.level)
        .map(|(k, o)| (format!("#{}", quoted(&o.name)), k))
        .collect();
    let infos: HashMap<&str, &Vec<String>> = lv.objects.iter().map(|o| (o.id.as_str(), &o.host)).collect();
    let document = |id: &str| -> Vec<usize> {
        if let Some(&u) = usages.get(id) {
            return vec![u];
        }
        infos.get(id).map_or_else(Vec::new, |host| host.iter().filter_map(|h| instances.get(h).copied()).collect())
    };
    LevelConstraints {
        input: input_digest(p),
        objects: p
            .objects
            .iter()
            .zip(&ids.objects)
            .map(|(o, id)| ConstraintObject {
                kind: o.kind.as_str(),
                label: o.label.clone(),
                reference: reference(&o.reference, numbers),
                generated: o.generated,
                objects: id.as_deref().map(document).unwrap_or_default(),
                id: id.clone(),
            })
            .collect(),
        facts: (0..p.facts.len()).map(|k| constraint(p, k)).collect(),
        usages: p
            .usages
            .iter()
            .zip(&ids.usages)
            .map(|(u, id)| ConstraintUsage {
                definition: u.definition.clone(),
                label: u.label.clone(),
                reference: reference(&u.reference, numbers),
                generated: u.generated,
                parent: u.parent,
                object: id.as_deref().and_then(|id| usages.get(id).copied()),
            })
            .collect(),
        routes: p.routes.iter().map(route).collect(),
        groups: p
            .groups
            .iter()
            .map(|g| ConstraintGroup { label: g.label.clone(), members: g.members.clone() })
            .collect(),
    }
}
