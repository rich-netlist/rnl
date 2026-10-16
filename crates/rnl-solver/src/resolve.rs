//! The resolution of 8.5: the facts in the order of [`LevelProblem::facts`], each kept when it fits
//! with those kept before it, its literals searched depth first, a fact that does not fit
//! discarded with every minimal conflict group it has, the hints of each statement, and a witness
//! that satisfies every fact kept (plan 5.3; the draft 0.3, 8.5).
//!
//! **State.** One incremental simplex per axis ([`crate::simplex`]) holds the bounds of the facts
//! kept that hold whatever is chosen, each tagged with its fact, and the bounds of every literal
//! open at the value the witness gives it, each tagged with the literal and its value; the
//! orientation classes ([`crate::orient`]) hold ORIENT and ORIENT-RELATION; the **witness** is a
//! placement of every object, an origin and an orientation each, that satisfies every fact kept.
//!
//! **Literals** (plan 5.3, five kinds): the direction of an ORDER given only an axis; the side of a
//! DISTANCE given only an axis with a lower bound; the pair that keeps a gap of a DISTANCE between
//! sets within its upper bound; the lowest and the highest point of a set of three or more whose
//! centre a fact constrains; and the orientation of a class whose orientation matters, that is,
//! one with an object two of whose points facts reach on one axis (plan 5.1). Only then are the
//! pins of the object linked to its origin, `x_pin − x_origin = c(orientation)`, and only on that
//! axis.
//!
//! **Taking a fact in.**
//!
//! 1. When the witness satisfies it already, it is kept: its bounds go into the simplex, for later
//!    explanations, with those of the literals it opens at the witness's values, and nothing is
//!    searched.
//! 2. Otherwise its bounds go in with those of the literals it opens, and the simplex checks them
//!    with every literal at the value it holds. A solution keeps the fact, and becomes the witness;
//!    a conflict that names no literal discards it. When the conflict names a literal, or the fact
//!    changes the orientation classes, a depth-first search looks for values of the literals that
//!    have a solution, over the bounds of the facts alone: each node asserts the bounds of every
//!    literal not decided at the value the witness gives it (the new fact's own at the one the
//!    witness comes closest to), and checks. A solution keeps the fact, becomes the witness, and
//!    the literals are held at its values. A conflict is explained by the tags of the bounds behind
//!    it: when no literal still open is among them, the node fails with that explanation;
//!    otherwise the search decides one of them, trying each of its values, the witness's first. A
//!    value whose explanation does not involve the literal fails the node at once
//!    (conflict-directed backjumping); when every value fails, the node's explanation is the union
//!    of theirs without the literal, plus what made its values the only ones: its own fact, or,
//!    for an orientation, the ORIENT-RELATIONs that tie its objects together and the ORIENT that
//!    fixes the class. The explanations of the nodes that fail with a literal decided are kept as
//!    nogoods, combinations of values that cannot hold together, and prune later searches. Values
//!    of an orientation that give every link of its class the same constants are tried once.
//! 3. When the root fails, the fact is discarded. The facts of its explanation conflict with it;
//!    a deletion filter makes the set minimal: from the latest rank on, each fact is left out when
//!    the rest still conflicts with the fact discarded, each test a small resolution of its own.
//!    So this first group leans towards the facts that rank first.
//! 4. Once every fact is decided and each fact discarded has its first group, the other minimal
//!    conflict groups of each are enumerated (the draft 0.3, 8.5: the hint names every set of
//!    facts taken in that has no solution together with the discarded fact and from which no fact
//!    can be left out). MARCO over the deletion filter ([`all_groups`]): a seed, a largest subset
//!    of the facts kept before the fact that holds none of the groups found and is no subset of a
//!    set found to fit, either fits with the fact, and every later seed must hold one fact outside
//!    it, or holds a new group, which the filter makes minimal, and every later seed must leave out
//!    one fact of it. When no seed is left, the groups are all.
//!
//! The search is complete: a fact is discarded only when no values of the literals have a
//! solution, so what is kept is the lexicographically greatest consistent subset in the order of
//! 8.5. Every decision compares exact rationals.
//!
//! **The hints by statement** (the draft 0.3, 8.5; 7.6). A fact is named by the statement that
//! wrote it ([`Statement`]): where it is written, and, for a fact expanded from a usage, the usages
//! of its chain, the first of them the AS or ATTACH statement of the level. Each statement whose
//! facts are in a group gets the facts discarded because of them ([`StatementHint`]).
//!
//! **The time limit** (the draft 0.3, 8.5; [`SolverOptions::time_limit`]). Deciding the facts is
//! NP-hard in general (the side of `"x"` DISTANCEs alone can encode graph colouring), so the
//! resolution of a level may be limited in time, on the clock the caller gives ([`Deadline`]). The
//! limit runs from the start of the resolution and covers the checks of the facts, then the first
//! group of each fact discarded, then the other groups, nothing else. The time is read at the
//! start of each check of a fact, at each node of a search, every few pivots of a simplex
//! ([`crate::simplex::PIVOTS_PER_READ`]), at each seed of the enumeration and every few nodes of
//! its search ([`SEED_NODES_PER_READ`]). When it is out during the check of a fact, the fact is
//! discarded **undecided**, with no group and a warning (7.6), and so is every later fact that
//! needs a check; a later fact that the witness already satisfies is kept, as step 1 needs no
//! check, so that the witness satisfies every fact kept. When it is out after every fact is
//! decided, the groups found by then are given, each minimal, and [`Discarded::complete`] is false
//! for each fact whose groups were not all found: a group that its filter had not made minimal is
//! left out, as the hint names only minimal ones. Without a limit, and whenever no check reaches
//! it, the facts kept are those of 8.5; when the enumeration ends within it too, the groups are
//! all. What is not the resolution of 8.5 is not timed: the witness taken as written below, and
//! the solver's own uses of the resolution, for the relations of a candidate (`crate::search`),
//! are bounded by caps of work ([`Caps`]), and a search or a simplex that reaches its cap leaves
//! its relation undecided.
//!
//! **The witness given back** is taken, when it can be, from the facts kept resolved once more
//! without the tolerance: the tolerance decides conflicts (11.7), but a simplex stops on the bounds
//! it widens, and a PLACE at 4 should put its object at 4. When the facts kept need the tolerance
//! to hold together, each that has a constant (a coordinate, an offset, an ORDER, a DISTANCE with a
//! bound) is held with its constant as written, in the order of 8.5, as long as the facts kept
//! still hold together, the others within the tolerance ([`Model::with_exact`]); the witness is
//! taken from them so. Of two facts that hold together only within the tolerance, the one that
//! ranks first keeps its constant, and the coordinates sought on that model are what it says, so
//! that the record of the solver, which a viewer checks with its own tolerance (11.7), writes the
//! constants of the facts that rank first (17). These resolutions are not timed but bounded by the
//! caps of work ([`CAPS`]), so the witness is the same whenever every fact is decided; when a fact
//! is left undecided they are not made, and the witness is the one of the resolution, which
//! satisfies the facts kept within the tolerance.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use num_traits::Zero;
use rnl_runtime::geom::{fmt_num, Orient};
use rnl_runtime::host::file_name;
use rnl_runtime::problem::{FactKind, LevelProblem, PointRef, Position};

use crate::model::{ax, turned, Bound, ChoiceKind, LitKey, Model, OrientEffect, PointId, VarKind};
use crate::options::{Caps, CAPS};
use crate::orient::{from_index, index, OrientClasses};
use crate::rational::{from_f64, to_f64, Q};
use crate::simplex::{Failure, LinExpr, Rel, Simplex, Var};
use crate::{Clock, Discarded, SolverOptions, Statement, StatementHint, Work};

/// The time limit of the resolution of one level (the draft 0.3, 8.5), on the clock the caller
/// gives: it runs from the reading when the resolution begins, and once it is out it stays out,
/// the clock no longer read.
pub(crate) struct Deadline<'c> {
    clock: &'c dyn Clock,
    /// The reading of the clock after which the time is out; none without a limit.
    at: Option<f64>,
    out: Cell<bool>,
}

impl<'c> Deadline<'c> {
    /// A limit of `limit` seconds from now, or none.
    pub(crate) fn new(clock: &'c dyn Clock, limit: Option<f64>) -> Deadline<'c> {
        let at = limit.map(|l| clock.elapsed() + l);
        Deadline { clock, at, out: Cell::new(false) }
    }

    /// Whether the time is out: the clock is read, until it is.
    pub(crate) fn passed(&self) -> bool {
        if self.out.get() {
            return true;
        }
        let Some(at) = self.at else { return false };
        let out = self.clock.elapsed() > at;
        self.out.set(out);
        out
    }

    /// Whether the time was out when the clock was last read; the clock is not read.
    pub(crate) fn has_passed(&self) -> bool {
        self.out.get()
    }
}

/// What bounds a resolution: the resolution of 8.5 has the time limit and no cap of work; the
/// witness taken as written and the solver's own uses of the resolution have caps of work and no
/// time limit (see the module).
#[derive(Clone, Copy)]
pub(crate) enum Limits<'a> {
    Time(&'a Deadline<'a>),
    Caps(Caps),
}

impl Limits<'_> {
    /// The nodes one search may make.
    fn nodes(&self) -> u64 {
        match self {
            Limits::Time(_) => u64::MAX,
            Limits::Caps(c) => c.nodes,
        }
    }

    /// Whether the time is out, reading the clock.
    fn out_of_time(&self) -> bool {
        matches!(self, Limits::Time(d) if d.passed())
    }

    /// Whether the time was out at the last reading, without reading the clock.
    fn was_out(&self) -> bool {
        matches!(self, Limits::Time(d) if d.has_passed())
    }
}

/// What a bound of the simplex is asserted for: what an explanation names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Tag {
    /// A definition that always holds, such as the bounds that make a variable the centre of a
    /// set: never part of a conflict set.
    Def,
    /// A unit of 8.5, by its index.
    Fact(usize),
    /// A literal at a value.
    Lit(LitKey, u32),
    /// An object at an orientation, by its index in [`Orient::ALL`]: the links of its pins.
    Ori(usize, u8),
}

/// A placement of the objects of a problem, exact: the witness of the resolution.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Geometry {
    /// The origin of each object with a position of its own, x and y; 0 for the others.
    pub origin: Vec<[Q; 2]>,
    /// The orientation of each object; R0 for those without one.
    pub orient: Vec<Orient>,
}

/// How deep the main objects of objects drawn at a point are followed.
const MAX_DEPTH: usize = 16;

impl Geometry {
    /// Every object at the origin with R0.
    pub(crate) fn new(n: usize) -> Geometry {
        Geometry { origin: vec![[Q::zero(), Q::zero()]; n], orient: vec![Orient::R0; n] }
    }

    /// The value of a point on an axis (14.1): an origin as placed, a pin or a text as its object
    /// turns it, a text on the axis that the scene gives it as placed, a centre the middle of the
    /// extent of its set.
    pub(crate) fn value(&self, model: &Model, p: PointId, axis: usize) -> Q {
        self.value_at(model, p, axis, 0)
    }

    fn value_at(&self, model: &Model, p: PointId, axis: usize, depth: usize) -> Q {
        match p {
            PointId::Origin(k) => self.origin[k][axis].clone(),
            PointId::Text(t) if model.points[&p].free == Some(axis) => self.origin[t][axis].clone(),
            PointId::Pin(..) | PointId::Text(_) => {
                let info = &model.points[&p];
                let Some(k) = info.object else { return Q::zero() };
                let base = match model.origin_point[k] {
                    Some(b) if depth < MAX_DEPTH => self.value_at(model, b, axis, depth + 1),
                    _ => Q::zero(),
                };
                match &info.local {
                    Some(local) => base + turned(self.orient[k], local, axis),
                    None => base,
                }
            }
            PointId::Centre(c) => {
                let (lo, hi) = self.extent(model, &model.centres[c].members, axis);
                (lo + hi) / Q::from_integer(2.into())
            }
        }
    }

    /// The lowest and the highest value of some points; 0 for none.
    fn extent(&self, model: &Model, points: &[PointId], axis: usize) -> (Q, Q) {
        let mut values = points.iter().map(|p| self.value(model, *p, axis));
        let Some(first) = values.next() else { return (Q::zero(), Q::zero()) };
        values.fold((first.clone(), first), |(lo, hi), v| {
            let lo = if v < lo { v.clone() } else { lo };
            let hi = if v > hi { v } else { hi };
            (lo, hi)
        })
    }

    /// The value of a variable of the model: a point as above, the lowest and highest point of a
    /// centre's set, a separator at the highest point before it, the least it can be.
    pub(crate) fn var_value(&self, model: &Model, v: Var, axis: usize) -> Q {
        match &model.vars[v] {
            VarKind::Point(p) => self.value(model, *p, axis),
            VarKind::CentreLo(c) => self.extent(model, &model.centres[*c].members, axis).0,
            VarKind::CentreHi(c) => self.extent(model, &model.centres[*c].members, axis).1,
            VarKind::Separator { before, .. } => self.extent(model, before, axis).1,
        }
    }
}

/// What became of a unit taken in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Kept,
    /// It conflicts with these units kept before it (not yet minimal).
    Conflict(Vec<usize>),
    /// Its check did not finish: the time limit of the resolution was out, or a cap of the
    /// solver's own use of it ran out ([`Limits`]).
    Undecided,
}

/// A literal open in one search.
#[derive(Clone, Debug)]
struct Lit {
    key: LitKey,
    /// The value the witness gives it, which a node takes when the literal is not decided.
    default: u32,
    /// The bounds of the default value, with their tags, made once for the search.
    defaults: Vec<(Bound, Tag)>,
    /// How many values it has: for an orientation, the values its class allows, before those
    /// that give the links the same constants are taken as one ([`Resolver::values`]).
    count: usize,
    body: LitBody,
}

#[derive(Clone, Debug)]
enum LitBody {
    /// A choice of a unit, by unit and index.
    Choice { unit: usize, choice: usize },
    /// The lowest (`high` false) or highest point of a centre's set on an axis.
    Centre { centre: usize, axis: usize, high: bool },
    /// The orientation of the class rooted at `root`, with the links of its pins that matter.
    Class { root: usize, links: Vec<ClassLink> },
}

/// `point − base = turned(ori(object), local)` on an axis, with `ori(object) = ori(root).then(label)`.
#[derive(Clone, Debug)]
struct ClassLink {
    object: usize,
    label: Orient,
    axis: usize,
    id: PointId,
    point: Var,
    base: Var,
    local: (Q, Q),
}

/// The result of a node of the search.
enum Node {
    Sat,
    /// No solution under the literals decided: the explanation, which names only literals decided
    /// and units.
    Conflict(Vec<Tag>),
    /// The time was out, or the cap of nodes or pivots ran out ([`Limits`]).
    Stopped,
}

/// The state of one search.
struct Search {
    nodes: u64,
    limit: u64,
    /// The values of the literals at the solution found.
    solution: Option<Vec<u32>>,
}

/// Values of literals that cannot hold together while the units of `facts` are in.
#[derive(Clone, Debug)]
struct Nogood {
    lits: Vec<Tag>,
    facts: Vec<Tag>,
}

/// How many literals a nogood may have, and how many are kept: beyond these, learning helps less
/// than matching costs.
const NOGOOD_LITS: usize = 8;
const NOGOODS: usize = 4096;

/// A literal bound held in the simplex between searches, at the value the witness gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Held {
    /// A choice of a unit, by unit and index.
    Choice(usize, usize),
    /// The lowest (false) or highest point of a centre's set on an axis.
    Centre(usize, usize, bool),
    /// The link of a point of an object to its origin on an axis, the value the object's
    /// orientation.
    Link(usize, usize, PointId),
}

/// What a tentative unit changes, to undo.
struct Saved {
    classes: OrientClasses,
    touched: [BTreeSet<PointId>; 2],
    centres_on: [BTreeSet<usize>; 2],
    choices: usize,
    nogoods: usize,
    held: BTreeMap<Held, u32>,
    kept: usize,
    checkpoint: [usize; 2],
}

/// The resolution of 8.5 in progress: units are taken in one by one ([`Resolver::add`]).
pub(crate) struct Resolver<'a> {
    model: &'a Model,
    limits: Limits<'a>,
    sx: [Simplex<Tag>; 2],
    classes: OrientClasses,
    geometry: Geometry,
    /// The points that the units kept constrain, by axis (plan 5.1).
    touched: [BTreeSet<PointId>; 2],
    /// The centres of three or more points that the units kept constrain, by axis.
    centres_on: [BTreeSet<usize>; 2],
    /// The choices of the units kept, by unit and index.
    choices: Vec<(usize, usize)>,
    nogoods: Vec<Nogood>,
    /// The units kept, in order.
    kept: Vec<usize>,
    /// The literals whose bounds the simplex holds, at their values: every literal open, at the
    /// value the witness gives it. A search that must change one asserts the literals node by
    /// node instead, from the bounds of the units kept, and holds them again after.
    held: BTreeMap<Held, u32>,
    /// The checkpoint after the definitions of the centres, from which the bounds are rebuilt.
    base: [usize; 2],
    /// The nodes searched.
    pub(crate) nodes: u64,
}

impl<'a> Resolver<'a> {
    /// A resolution with nothing taken in yet, every object at the origin with R0.
    pub(crate) fn new(model: &'a Model, n_objects: usize, limits: Limits<'a>) -> Resolver<'a> {
        let mut sx = [Simplex::new(), Simplex::new()];
        for s in &mut sx {
            for _ in 0..model.vars.len() {
                s.new_var();
            }
            if let Limits::Caps(caps) = limits {
                s.set_pivot_limit(Some(caps.pivots));
            }
        }
        // the centres are defined from the start: the definitions restrict no point
        for c in 0..model.centres.len() {
            for (axis, s) in sx.iter_mut().enumerate() {
                for b in model.centre_definition(c, axis) {
                    // a definition alone always has a solution
                    let _ = assert_bound(s, &b, Tag::Def);
                }
            }
        }
        let base = [sx[0].checkpoint(), sx[1].checkpoint()];
        Resolver {
            model,
            limits,
            sx,
            classes: OrientClasses::new(n_objects),
            geometry: Geometry::new(n_objects),
            touched: [BTreeSet::new(), BTreeSet::new()],
            centres_on: [BTreeSet::new(), BTreeSet::new()],
            choices: Vec::new(),
            nogoods: Vec::new(),
            kept: Vec::new(),
            held: BTreeMap::new(),
            base,
            nodes: 0,
        }
    }

    /// Starts from a placement other than every object at the origin with R0: the witness of an
    /// earlier resolution, so that the facts it kept are taken in again without a check (step 1
    /// of the module). Only before the first unit is taken in.
    pub(crate) fn seed(&mut self, geometry: Geometry) {
        debug_assert!(self.kept.is_empty() && geometry.orient.len() == self.geometry.orient.len());
        self.geometry = geometry;
    }

    /// The simplex of an axis, with the bounds of the units kept and of the literals held: the
    /// coordinates of a candidate are optimised over it (`crate::coords`).
    pub(crate) fn simplex_mut(&mut self, axis: usize) -> &mut Simplex<Tag> {
        &mut self.sx[axis]
    }

    /// The pivots of both simplexes.
    pub(crate) fn pivots(&self) -> u64 {
        self.sx[0].pivots() + self.sx[1].pivots()
    }

    /// The witness: it satisfies every unit kept.
    pub(crate) fn geometry(&self) -> &Geometry {
        &self.geometry
    }

    pub(crate) fn classes(&self) -> &OrientClasses {
        &self.classes
    }

    /// Takes a unit in (8.5).
    pub(crate) fn add(&mut self, u: usize) -> Outcome {
        if self.holds(u) {
            let saved = self.save();
            if self.commit(u).is_ok() && self.hold_new(u).is_ok() {
                self.kept.push(u);
                return Outcome::Kept;
            }
            self.restore(saved);
        }
        self.decide(u)
    }

    /// Whether the witness satisfies a unit: its orientations, its bounds, and for each choice
    /// the bounds of one of its values.
    pub(crate) fn holds(&self, u: usize) -> bool {
        let unit = &self.model.units[u];
        let ori = &self.geometry.orient;
        let oriented = match &unit.orient {
            OrientEffect::None => true,
            OrientEffect::Fix { object, code } => ori.get(*object) == Some(code),
            OrientEffect::Relate { relation, from, to } => relation_holds(ori, *relation, from, to),
        };
        let holds = |b: &Bound| b.holds(|v| self.geometry.var_value(self.model, v, b.axis));
        oriented
            && unit.fixed.iter().all(holds)
            && unit.choices.iter().all(|c| c.values.iter().any(|bounds| bounds.iter().all(holds)))
    }

    fn save(&self) -> Saved {
        Saved {
            classes: self.classes.clone(),
            touched: self.touched.clone(),
            centres_on: self.centres_on.clone(),
            choices: self.choices.len(),
            nogoods: self.nogoods.len(),
            held: self.held.clone(),
            kept: self.kept.len(),
            checkpoint: self.checkpoint(),
        }
    }

    /// Undoes a tentative unit. When a search has rebuilt the bounds since, they are to be rebuilt
    /// again ([`Self::rebuild`]).
    fn restore(&mut self, saved: Saved) {
        self.backtrack(saved.checkpoint);
        self.classes = saved.classes;
        self.touched = saved.touched;
        self.centres_on = saved.centres_on;
        self.choices.truncate(saved.choices);
        self.nogoods.truncate(saved.nogoods);
        self.held = saved.held;
        self.kept.truncate(saved.kept);
    }

    /// The bounds in the simplex made again from the units kept, then those of `extra`, and, with
    /// `held`, those of the literals held. The bounds of `extra` may contradict those of the units
    /// kept at once: that conflict is given.
    fn rebuild(&mut self, extra: Option<usize>, held: bool) -> Result<(), Vec<Tag>> {
        self.backtrack(self.base);
        let model = self.model;
        for i in 0..self.kept.len() {
            let u = self.kept[i];
            for b in &model.units[u].fixed {
                let r = assert_bound(&mut self.sx[b.axis], b, Tag::Fact(u));
                debug_assert!(r.is_ok(), "the units kept hold together");
            }
        }
        if let Some(u) = extra {
            for b in &model.units[u].fixed {
                assert_bound(&mut self.sx[b.axis], b, Tag::Fact(u))?;
            }
        }
        if held {
            let entries: Vec<(Held, u32)> = self.held.iter().map(|(h, v)| (*h, *v)).collect();
            for (h, v) in entries {
                for (b, tag) in self.held_bounds(h, v) {
                    let r = assert_bound(&mut self.sx[b.axis], &b, tag);
                    debug_assert!(r.is_ok(), "the literals held hold with the units kept");
                }
            }
        }
        Ok(())
    }

    /// The bounds of a literal held at a value, with their tags.
    fn held_bounds(&self, h: Held, v: u32) -> Vec<(Bound, Tag)> {
        let model = self.model;
        match h {
            Held::Choice(unit, idx) => {
                let c = &model.units[unit].choices[idx];
                c.values[v as usize].iter().map(|b| (b.clone(), Tag::Lit(c.key, v))).collect()
            }
            Held::Centre(c, axis, high) => {
                let key = if high { LitKey::CentreHi(c, axis as u8) } else { LitKey::CentreLo(c, axis as u8) };
                vec![(model.centre_extreme(c, axis, high, v as usize), Tag::Lit(key, v))]
            }
            Held::Link(k, axis, p) => {
                let (Some(origin), Some(local)) = (model.origin_point[k], &model.points[&p].local) else {
                    return Vec::new();
                };
                let c = turned(from_index(v as u8), local, axis);
                let expr = LinExpr::diff(model.var(p), model.var(origin));
                vec![(Bound { axis, expr, lo: Some(c.clone()), hi: Some(c) }, Tag::Ori(k, v as u8))]
            }
        }
    }

    /// Holds the literals that a unit just put in opens: its choices, the extremes of the centres
    /// it constrains, the links of the objects whose orientation now matters; each at the value
    /// the witness gives it.
    fn hold_new(&mut self, u: usize) -> Result<(), Vec<Tag>> {
        let model = self.model;
        let unit = &model.units[u];
        let mut new: Vec<(Held, u32)> = Vec::new();
        for (idx, c) in unit.choices.iter().enumerate() {
            new.push((Held::Choice(u, idx), self.choice_default(&c.kind) as u32));
        }
        for axis in 0..2 {
            for &c in &unit.centres[axis] {
                let values: Vec<Q> =
                    model.centres[c].members.iter().map(|p| self.geometry.value(model, *p, axis)).collect();
                for high in [false, true] {
                    new.push((Held::Centre(c, axis, high), extreme(&values, high) as u32));
                }
            }
            // the objects whose points it reaches, and the main objects of those drawn at a point
            let mut objects = BTreeSet::new();
            for p in &unit.touched[axis] {
                let mut point = Some(*p);
                for _ in 0..MAX_DEPTH {
                    let Some(k) = point.and_then(|p| model.points[&p].object_on(axis)) else { break };
                    objects.insert(k);
                    point = if model.own[k] { None } else { model.origin_point[k] };
                }
            }
            for k in objects {
                for link in self.object_links(k, axis) {
                    new.push((Held::Link(k, axis, link), u32::from(index(self.geometry.orient[k]))));
                }
            }
        }
        for (h, v) in new {
            if self.held.contains_key(&h) {
                continue;
            }
            for (b, tag) in self.held_bounds(h, v) {
                assert_bound(&mut self.sx[b.axis], &b, tag)?;
            }
            self.held.insert(h, v);
        }
        Ok(())
    }

    /// The points of an object linked to its origin on an axis: none unless the object has an
    /// orientation and the units reach two of its points on that axis, its origin counted as one
    /// (plan 5.1); then each point of it reached but its origin.
    fn object_links(&self, k: usize, axis: usize) -> Vec<PointId> {
        let model = self.model;
        let Some(origin) = model.origin_point[k] else { return Vec::new() };
        if !model.oriented[k] {
            return Vec::new();
        }
        let points: Vec<PointId> = model.object_points[k]
            .iter()
            .copied()
            .filter(|p| *p != origin && self.touched[axis].contains(p) && model.points[p].linked(axis))
            .collect();
        if points.len() + usize::from(self.touched[axis].contains(&origin)) < 2 {
            return Vec::new();
        }
        points
    }

    fn checkpoint(&self) -> [usize; 2] {
        [self.sx[0].checkpoint(), self.sx[1].checkpoint()]
    }

    fn backtrack(&mut self, cp: [usize; 2]) {
        self.sx[0].backtrack(cp[0]);
        self.sx[1].backtrack(cp[1]);
    }

    /// Puts a unit in: its orientations into the classes, its points among those reached, its
    /// choices among the literals, and last its bounds into the simplex. A conflict found on the
    /// way gives the tags of what it involves: the facts of a cycle of orientations, or the bounds
    /// that the unit's contradict at once, which may be those of a literal held.
    fn commit(&mut self, u: usize) -> Result<(), Vec<Tag>> {
        let model = self.model;
        let unit = &model.units[u];
        self.apply_orient(u).map_err(|facts| facts.into_iter().map(Tag::Fact).collect::<Vec<_>>())?;
        for axis in 0..2 {
            for &p in &unit.touched[axis] {
                self.touch(p, axis);
            }
            self.centres_on[axis].extend(unit.centres[axis].iter().copied());
        }
        self.choices.extend((0..unit.choices.len()).map(|c| (u, c)));
        for b in &unit.fixed {
            assert_bound(&mut self.sx[b.axis], b, Tag::Fact(u))?;
        }
        Ok(())
    }

    /// ORIENT and ORIENT-RELATION into the classes (11.2, 14.9).
    fn apply_orient(&mut self, u: usize) -> Result<(), Vec<usize>> {
        let n = self.geometry.orient.len();
        match &self.model.units[u].orient {
            OrientEffect::None => Ok(()),
            OrientEffect::Fix { object, code } if *object < n => self.classes.fix(*object, *code, u),
            OrientEffect::Fix { .. } => Ok(()),
            OrientEffect::Relate { relation, from, to } => {
                let from: Vec<usize> = from.iter().copied().filter(|&k| k < n).collect();
                let to: Vec<usize> = to.iter().copied().filter(|&k| k < n).collect();
                // a side that is empty: it holds trivially (14.9)
                let (Some(&a), Some(&b)) = (from.first(), to.first()) else { return Ok(()) };
                for &k in &from[1..] {
                    self.classes.relate(a, k, Orient::R0, u)?;
                }
                for &k in &to[1..] {
                    self.classes.relate(b, k, Orient::R0, u)?;
                }
                self.classes.relate(a, b, *relation, u)
            }
        }
    }

    /// Marks a point reached on an axis. A point of an object drawn at its main object's point
    /// (12.1) reaches that point too, so that the object's orientation matters with it.
    fn touch(&mut self, p: PointId, axis: usize) {
        if !self.touched[axis].insert(p) {
            return;
        }
        if let Some(k) = self.model.points[&p].object_on(axis) {
            if let Some(o) = self.model.origin_point[k] {
                if o != p && !self.model.own[k] {
                    self.touch(o, axis);
                }
            }
        }
    }

    /// Takes a unit in that the witness does not satisfy (step 2 of the module). First quickly:
    /// the literals held at the witness's values and the new ones too, and one check; when the
    /// conflict names a literal, a search, which may change any. Undecided when the time is out
    /// before or during the check, or a cap runs out.
    fn decide(&mut self, u: usize) -> Outcome {
        if self.limits.out_of_time() {
            return Outcome::Undecided;
        }
        let saved = self.save();
        // a unit that changes the classes may change orientations the links hold: it searches
        let mut quickly = matches!(self.model.units[u].orient, OrientEffect::None);
        match self.commit(u) {
            Ok(()) => {}
            // its bounds contradict a literal held: the search may change the literal
            Err(tags) if names_literal(&tags) => quickly = false,
            Err(tags) => {
                self.restore(saved);
                return Outcome::Conflict(facts_of(&tags, u));
            }
        }
        if quickly {
            self.nodes += 1;
            let quick = match self.hold_new(u) {
                Ok(()) => self.check(),
                Err(tags) => Err(Failure::Conflict(tags)),
            };
            match quick {
                Ok(()) => {
                    self.update_geometry(&[], &[]);
                    self.kept.push(u);
                    return Outcome::Kept;
                }
                Err(Failure::Conflict(tags)) if !names_literal(&tags) => {
                    self.restore(saved);
                    return Outcome::Conflict(facts_of(&tags, u));
                }
                Err(Failure::Conflict(_)) => {}
                Err(_) => {
                    self.restore(saved);
                    return Outcome::Undecided;
                }
            }
        }
        if let Err(tags) = self.rebuild(Some(u), false) {
            self.restore(saved);
            let _ = self.rebuild(None, true);
            return Outcome::Conflict(facts_of(&tags, u));
        }
        let cp = self.checkpoint();
        let lits = self.literals(u);
        let mut search = Search { nodes: 0, limit: self.limits.nodes(), solution: None };
        let result = self.node(&lits, &mut Vec::new(), &mut search);
        self.nodes += search.nodes;
        match result {
            Node::Sat => {
                let values = search.solution.take().unwrap_or_default();
                self.update_geometry(&lits, &values);
                self.backtrack(cp);
                self.held = held_from(&lits, &values);
                self.kept.push(u);
                let _ = self.rebuild(None, true);
                Outcome::Kept
            }
            Node::Conflict(tags) => {
                self.restore(saved);
                let _ = self.rebuild(None, true);
                Outcome::Conflict(facts_of(&tags, u))
            }
            Node::Stopped => {
                self.restore(saved);
                let _ = self.rebuild(None, true);
                Outcome::Undecided
            }
        }
    }

    /// The literals open while `u` is taken in: the choices of `u`, then those of the units kept,
    /// the extremes of the centres constrained, and the orientations that matter.
    fn literals(&self, u: usize) -> Vec<Lit> {
        let mut out = Vec::new();
        let mut order: Vec<(usize, usize)> = self.choices.iter().copied().filter(|(v, _)| *v == u).collect();
        order.extend(self.choices.iter().copied().filter(|(v, _)| *v != u));
        for (unit, choice) in order {
            let c = &self.model.units[unit].choices[choice];
            let default = match self.held.get(&Held::Choice(unit, choice)) {
                Some(v) => *v,
                None => self.choice_default(&c.kind) as u32,
            };
            out.push(Lit {
                key: c.key,
                default,
                defaults: Vec::new(),
                count: c.values.len(),
                body: LitBody::Choice { unit, choice },
            });
        }
        for axis in 0..2 {
            for &c in &self.centres_on[axis] {
                let members = &self.model.centres[c].members;
                let values: Vec<Q> = members.iter().map(|p| self.geometry.value(self.model, *p, axis)).collect();
                for high in [false, true] {
                    let pick = match self.held.get(&Held::Centre(c, axis, high)) {
                        Some(v) => *v,
                        None => extreme(&values, high) as u32,
                    };
                    let key = if high { LitKey::CentreHi(c, axis as u8) } else { LitKey::CentreLo(c, axis as u8) };
                    out.push(Lit {
                        key,
                        default: pick,
                        defaults: Vec::new(),
                        count: members.len(),
                        body: LitBody::Centre { centre: c, axis, high },
                    });
                }
            }
        }
        for (root, links) in self.links() {
            // the witness gives a class a value it allows
            let (default, count) = match self.classes.fixed(root) {
                Some(f) => (u32::from(index(f.value)), 1),
                None => (u32::from(index(self.geometry.orient[root])), 8),
            };
            out.push(Lit {
                key: LitKey::Class(root),
                default,
                defaults: Vec::new(),
                count,
                body: LitBody::Class { root, links },
            });
        }
        for lit in &mut out {
            lit.defaults = self.lit_bounds(lit, lit.default);
        }
        out
    }

    /// The values of a literal to try, its default first. For an orientation, the values its class
    /// allows, one of each group that gives every link of the class the same constants: those
    /// make the same bounds.
    fn values(&self, lit: &Lit) -> Vec<u32> {
        let LitBody::Class { root, links } = &lit.body else {
            return first(lit.default as usize, lit.count);
        };
        let domain: Vec<u32> = match self.classes.fixed(*root) {
            Some(f) => vec![u32::from(index(f.value))],
            None => (0..8).collect(),
        };
        let mut ordered: Vec<u32> = domain.iter().copied().filter(|&v| v == lit.default).collect();
        ordered.extend(domain.iter().copied().filter(|&v| v != lit.default));
        let mut values = Vec::new();
        let mut seen: Vec<Vec<Q>> = Vec::new();
        for v in ordered {
            let o = from_index(v as u8);
            let signature: Vec<Q> = links.iter().map(|l| turned(o.then(l.label), &l.local, l.axis)).collect();
            if !seen.contains(&signature) {
                seen.push(signature);
                values.push(v);
            }
        }
        values
    }

    /// The value of a choice that the witness comes closest to.
    fn choice_default(&self, kind: &ChoiceKind) -> usize {
        let value = |p: &PointId, axis: usize| self.geometry.value(self.model, *p, axis);
        match kind {
            ChoiceKind::Order { axis, members } => {
                let first = members.first().map(|p| value(p, *axis));
                let last = members.last().map(|p| value(p, *axis));
                usize::from(last < first)
            }
            ChoiceKind::Side { axis, from, to } => {
                let (a, b) =
                    (self.geometry.extent(self.model, from, *axis), self.geometry.extent(self.model, to, *axis));
                // gap 0 = min(to) − max(from), gap 1 = min(from) − max(to)
                usize::from(&a.0 - &b.1 > &b.0 - &a.1)
            }
            // the solver's own alternatives: the one it prefers
            ChoiceKind::Ranked => 0,
            ChoiceKind::Pair { axis, gap, pairs } => {
                // the pair with the smallest gap
                let gap_of = |(a, b): &(PointId, PointId)| {
                    if *gap == 0 {
                        value(b, *axis) - value(a, *axis)
                    } else {
                        value(a, *axis) - value(b, *axis)
                    }
                };
                let gaps: Vec<Q> = pairs.iter().map(gap_of).collect();
                extreme(&gaps, false)
            }
        }
    }

    /// The links of the pins whose orientation matters ([`Self::object_links`]), by the root of
    /// their class.
    fn links(&self) -> BTreeMap<usize, Vec<ClassLink>> {
        let model = self.model;
        let mut out: BTreeMap<usize, Vec<ClassLink>> = BTreeMap::new();
        for k in 0..model.oriented.len() {
            for axis in 0..2 {
                let points = self.object_links(k, axis);
                let (Some(origin), false) = (model.origin_point[k], points.is_empty()) else { continue };
                let base = model.var(origin);
                let (root, label) = self.classes.find(k);
                for p in points {
                    let info = &model.points[&p];
                    out.entry(root).or_default().push(ClassLink {
                        object: k,
                        label,
                        axis,
                        id: p,
                        point: info.var,
                        base,
                        local: info.local.clone().unwrap_or_default(),
                    });
                }
            }
        }
        out
    }

    /// The bounds of a literal at a value, with their tags.
    fn lit_bounds(&self, lit: &Lit, v: u32) -> Vec<(Bound, Tag)> {
        match &lit.body {
            LitBody::Choice { unit, choice } => self.model.units[*unit].choices[*choice].values[v as usize]
                .iter()
                .map(|b| (b.clone(), Tag::Lit(lit.key, v)))
                .collect(),
            LitBody::Centre { centre, axis, high } => {
                vec![(self.model.centre_extreme(*centre, *axis, *high, v as usize), Tag::Lit(lit.key, v))]
            }
            LitBody::Class { links, .. } => links
                .iter()
                .map(|l| {
                    let o = from_index(v as u8).then(l.label);
                    let c = turned(o, &l.local, l.axis);
                    let b =
                        Bound { axis: l.axis, expr: LinExpr::diff(l.point, l.base), lo: Some(c.clone()), hi: Some(c) };
                    (b, Tag::Ori(l.object, index(o)))
                })
                .collect(),
        }
    }

    /// Whether a tag is one of the literal's.
    fn owns(&self, lit: &Lit, tag: &Tag) -> bool {
        match (tag, &lit.body) {
            (Tag::Lit(key, _), _) => *key == lit.key,
            (Tag::Ori(k, _), LitBody::Class { root, .. }) => self.classes.find(*k).0 == *root,
            _ => false,
        }
    }

    /// What makes the values of a literal the only ones: its unit for a choice; nothing for a
    /// centre, whose set has a lowest and a highest point whatever the facts; for an orientation,
    /// the ORIENT-RELATIONs from the objects `involved` to the root of the class, and the ORIENT
    /// that fixes it.
    fn reason(&self, lit: &Lit, involved: &BTreeSet<usize>) -> Vec<Tag> {
        match &lit.body {
            LitBody::Choice { unit, .. } => vec![Tag::Fact(*unit)],
            LitBody::Centre { .. } => Vec::new(),
            LitBody::Class { root, .. } => {
                let mut facts = BTreeSet::new();
                for &k in involved {
                    facts.extend(self.classes.path_to_root(k));
                }
                if let Some(f) = self.classes.fixed(*root) {
                    facts.extend(self.classes.path_to_root(f.node));
                    facts.insert(f.fact);
                }
                facts.into_iter().map(Tag::Fact).collect()
            }
        }
    }

    /// One node of the search: the literals in `decided` hold their values; the others take the
    /// witness's (see the module).
    fn node(&mut self, lits: &[Lit], decided: &mut Vec<(usize, u32)>, search: &mut Search) -> Node {
        if search.nodes >= search.limit || self.limits.out_of_time() {
            return Node::Stopped;
        }
        search.nodes += 1;
        let cp = self.checkpoint();
        let mut conflict = None;
        'assert: for (i, lit) in lits.iter().enumerate() {
            if decided.iter().any(|(j, _)| *j == i) {
                continue;
            }
            for (b, tag) in &lit.defaults {
                if let Err(tags) = assert_bound(&mut self.sx[b.axis], b, *tag) {
                    conflict = Some(tags);
                    break 'assert;
                }
            }
        }
        if conflict.is_none() {
            match self.check() {
                Ok(()) => {
                    let values = (0..lits.len())
                        .map(|i| decided.iter().find(|(j, _)| *j == i).map_or(lits[i].default, |(_, v)| *v))
                        .collect();
                    search.solution = Some(values);
                    self.backtrack(cp);
                    return Node::Sat;
                }
                Err(Failure::Conflict(tags)) => conflict = Some(tags),
                Err(_) => {
                    self.backtrack(cp);
                    return Node::Stopped;
                }
            }
        }
        self.backtrack(cp);
        let explanation: Vec<Tag> = conflict.unwrap_or_default().into_iter().filter(|t| *t != Tag::Def).collect();
        let open = (0..lits.len())
            .filter(|i| !decided.iter().any(|(j, _)| j == i) && explanation.iter().any(|t| self.owns(&lits[*i], t)));
        // the literal with the fewest values first, then the first
        let Some(i) = open.min_by_key(|&i| (lits[i].count, i)) else {
            return Node::Conflict(explanation);
        };
        let lit = &lits[i];
        let mut union: BTreeSet<Tag> = BTreeSet::new();
        let mut involved: BTreeSet<usize> = BTreeSet::new();
        for v in self.values(lit) {
            let result = if let Some(n) = self.pruned(lits, decided, (i, v)) {
                Node::Conflict(self.nogoods[n].lits.iter().chain(&self.nogoods[n].facts).copied().collect())
            } else {
                let cp = self.checkpoint();
                let mut result = None;
                for (b, tag) in self.lit_bounds(lit, v) {
                    if let Err(tags) = assert_bound(&mut self.sx[b.axis], &b, tag) {
                        result = Some(Node::Conflict(tags.into_iter().filter(|t| *t != Tag::Def).collect()));
                        break;
                    }
                }
                let result = result.unwrap_or_else(|| {
                    decided.push((i, v));
                    let r = self.node(lits, decided, search);
                    decided.pop();
                    r
                });
                self.backtrack(cp);
                result
            };
            match result {
                Node::Conflict(tags) => {
                    if !tags.iter().any(|t| self.owns(lit, t)) {
                        // the literal does not matter to this conflict: neither do its other values
                        return Node::Conflict(tags);
                    }
                    for t in tags {
                        if self.owns(lit, &t) {
                            if let Tag::Ori(k, _) = t {
                                involved.insert(k);
                            }
                        } else {
                            union.insert(t);
                        }
                    }
                }
                other => return other,
            }
        }
        union.extend(self.reason(lit, &involved));
        let union: Vec<Tag> = union.into_iter().collect();
        self.learn(&union);
        Node::Conflict(union)
    }

    /// Checks both axes; when the resolution has a time limit, a check stops once it is out.
    fn check(&mut self) -> Result<(), Failure<Tag>> {
        let limits = self.limits;
        let out_of_time = move || limits.out_of_time();
        self.sx[0].check_until(&out_of_time)?;
        self.sx[1].check_until(&out_of_time)
    }

    /// A nogood that the literals decided and `extra` match, by index.
    fn pruned(&self, lits: &[Lit], decided: &[(usize, u32)], extra: (usize, u32)) -> Option<usize> {
        self.nogoods.iter().position(|ng| ng.lits.iter().all(|t| self.assigned(lits, decided, extra, t)))
    }

    /// Whether the literals decided and `extra` give a literal tag its value.
    fn assigned(&self, lits: &[Lit], decided: &[(usize, u32)], extra: (usize, u32), tag: &Tag) -> bool {
        let mut values = decided.iter().copied().chain(std::iter::once(extra));
        match tag {
            Tag::Lit(key, value) => values.any(|(j, w)| lits[j].key == *key && w == *value),
            Tag::Ori(k, o) => {
                let (r, g) = self.classes.find(*k);
                values.any(|(j, w)| {
                    matches!(lits[j].body, LitBody::Class { root, .. } if root == r)
                        && index(from_index(w as u8).then(g)) == *o
                })
            }
            Tag::Def | Tag::Fact(_) => true,
        }
    }

    /// Keeps an explanation that names literals as a nogood.
    fn learn(&mut self, explanation: &[Tag]) {
        let (lits, facts): (Vec<Tag>, Vec<Tag>) =
            explanation.iter().partition(|t| matches!(t, Tag::Lit(..) | Tag::Ori(..)));
        if !lits.is_empty() && lits.len() <= NOGOOD_LITS && self.nogoods.len() < NOGOODS {
            self.nogoods.push(Nogood { lits, facts });
        }
    }

    /// The witness from the solution of a search: the orientations of the classes as the literals
    /// say, of a class whose orientation does not matter its fixed value or the one it had; each
    /// coordinate reached by the simplex's value, or from the one pin or text reached on that axis
    /// turned back to the origin; the others as they were.
    fn update_geometry(&mut self, lits: &[Lit], values: &[u32]) {
        let model = self.model;
        let mut root_value: BTreeMap<usize, Orient> = BTreeMap::new();
        for (lit, &v) in lits.iter().zip(values) {
            if let LitBody::Class { root, .. } = lit.body {
                root_value.insert(root, from_index(v as u8));
            }
        }
        let old = self.geometry.orient.clone();
        for (k, ori) in self.geometry.orient.iter_mut().enumerate() {
            if model.oriented[k] {
                let (r, g) = self.classes.find(k);
                let value = root_value.get(&r).copied().or_else(|| self.classes.fixed(r).map(|f| f.value));
                *ori = value.unwrap_or(old[r]).then(g);
            }
        }
        let mut of: BTreeMap<(usize, usize), Vec<PointId>> = BTreeMap::new();
        for axis in 0..2 {
            for &p in &self.touched[axis] {
                match (p, model.points[&p].object_on(axis)) {
                    // a text on the axis that the scene gives it: its own coordinate (13.3)
                    (PointId::Text(t), None) => {
                        self.geometry.origin[t][axis] = self.sx[axis].value(model.var(p)).clone()
                    }
                    (_, Some(k)) => of.entry((k, axis)).or_default().push(p),
                    _ => {}
                }
            }
        }
        for ((k, axis), points) in of {
            if !model.own[k] {
                continue;
            }
            let origin = PointId::Origin(k);
            let value = if points.contains(&origin) || points.len() >= 2 {
                self.sx[axis].value(model.var(origin)).clone()
            } else {
                let p = points[0];
                let local = model.points[&p].local.clone().unwrap_or_default();
                self.sx[axis].value(model.var(p)) - turned(self.geometry.orient[k], &local, axis)
            };
            self.geometry.origin[k][axis] = value;
        }
    }
}

/// The literals held at the values a search found: each choice and extreme at its value, each
/// link at the orientation its class's value gives its object.
fn held_from(lits: &[Lit], values: &[u32]) -> BTreeMap<Held, u32> {
    let mut out = BTreeMap::new();
    for (lit, &v) in lits.iter().zip(values) {
        match &lit.body {
            LitBody::Choice { unit, choice } => {
                out.insert(Held::Choice(*unit, *choice), v);
            }
            LitBody::Centre { centre, axis, high } => {
                out.insert(Held::Centre(*centre, *axis, *high), v);
            }
            LitBody::Class { links, .. } => {
                for l in links {
                    let o = from_index(v as u8).then(l.label);
                    out.insert(Held::Link(l.object, l.axis, l.id), u32::from(index(o)));
                }
            }
        }
    }
    out
}

/// The literal values `0..n`, `d` first.
fn first(d: usize, n: usize) -> Vec<u32> {
    let d = d.min(n.saturating_sub(1));
    std::iter::once(d).chain((0..n).filter(|&v| v != d)).map(|v| v as u32).collect()
}

/// The index of the lowest value (`high` false) or the highest, the first of equal ones.
fn extreme(values: &[Q], high: bool) -> usize {
    let mut best = 0;
    for (i, v) in values.iter().enumerate().skip(1) {
        if (high && *v > values[best]) || (!high && *v < values[best]) {
            best = i;
        }
    }
    best
}

/// Whether orientations satisfy an ORIENT-RELATION (14.9).
fn relation_holds(ori: &[Orient], relation: Orient, from: &[usize], to: &[usize]) -> bool {
    let get = |k: &usize| ori.get(*k).copied().unwrap_or_default();
    let (Some(a), Some(b)) = (from.first(), to.first()) else { return true };
    from.iter().all(|k| get(k) == get(a)) && to.iter().all(|k| get(k) == get(b)) && get(b) == get(a).then(relation)
}

/// Whether an explanation names a literal.
fn names_literal(tags: &[Tag]) -> bool {
    tags.iter().any(|t| matches!(t, Tag::Lit(..) | Tag::Ori(..)))
}

/// The unit behind a tag, if any: a fact, or the fact that owns a choice.
fn owner(tag: &Tag) -> Option<usize> {
    match tag {
        Tag::Fact(f) => Some(*f),
        Tag::Lit(LitKey::Order(f) | LitKey::Side(f) | LitKey::Pair(f, _), _) => Some(*f),
        _ => None,
    }
}

/// The units an explanation names, `u` left out, sorted.
fn facts_of(tags: &[Tag], u: usize) -> Vec<usize> {
    let set: BTreeSet<usize> = tags.iter().filter_map(owner).filter(|&f| f != u).collect();
    set.into_iter().collect()
}

/// Asserts a bound with a tag; a conflict found at once gives its tags.
fn assert_bound(s: &mut Simplex<Tag>, b: &Bound, tag: Tag) -> Result<(), Vec<Tag>> {
    let result = match (&b.lo, &b.hi) {
        (Some(lo), Some(hi)) if lo == hi => s.assert(&b.expr, Rel::Eq, lo, tag),
        (lo, hi) => {
            let r = match lo {
                Some(lo) => s.assert(&b.expr, Rel::Ge, lo, tag),
                None => Ok(()),
            };
            match (r, hi) {
                (Ok(()), Some(hi)) => s.assert(&b.expr, Rel::Le, hi, tag),
                (r, _) => r,
            }
        }
    };
    result.map_err(|f| match f {
        Failure::Conflict(tags) => tags,
        Failure::Unbounded | Failure::Budget | Failure::Time => vec![tag],
    })
}

/// The witness of the resolution: a value for every variable of the model on each axis, and an
/// orientation for every object, so for every class.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Witness {
    pub geometry: Geometry,
    /// Every variable of the model on x and on y.
    pub values: [Vec<Q>; 2],
    pub classes: OrientClasses,
}

/// The resolution of 8.5 of a level: what became of each fact, and a placement that satisfies
/// every fact kept, the witness from which the coordinates are sought.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    /// The facts kept, by their index in [`LevelProblem::facts`], in that order (8.5).
    pub kept: Vec<usize>,
    /// The facts discarded, in that order, each with its minimal conflict groups, all of them
    /// unless the time limit ran out before they were found, or undecided ([`Discarded`]).
    pub discarded: Vec<Discarded>,
    /// The hints of 8.5 by statement (the draft 0.3, 8.5): each statement whose facts are in a
    /// group of `discarded`, with those facts and the facts discarded because of them.
    pub statements: BTreeMap<Statement, StatementHint>,
    pub work: Work,
    /// The warnings (7.6): one for each fact discarded undecided because the time limit was out
    /// (the draft 0.3, 8.5), in the order of the facts.
    pub warnings: Vec<String>,
    /// Anything else worth saying of the resolution, one sentence each. What the time limit left
    /// undone is not here but in `warnings` and [`Discarded::minimal`].
    pub notes: Vec<String>,
    pub(crate) model: Model,
    pub(crate) witness: Witness,
    /// Whether the facts kept hold together with their constants as written, the tolerance not
    /// needed: the witness is then taken from them so, and the coordinates are sought so.
    pub(crate) as_written: bool,
    /// The model the witness is taken from and the coordinates are sought on: without the
    /// tolerance when the facts kept hold so (`as_written`), otherwise with the facts kept that
    /// hold with their constants as written held so, in the order of 8.5 (see the module).
    pub(crate) placing: Model,
}

impl Resolved {
    /// Where the witness puts a point, such as the origin of an object or the pin of a terminal:
    /// exact, then rounded to the nearest `f64`. None for a reference to nothing in the problem.
    pub fn point(&self, problem: &LevelProblem, r: &PointRef) -> Option<(f64, f64)> {
        let x = point_value(problem, &self.witness.geometry, r, 0, 0)?;
        let y = point_value(problem, &self.witness.geometry, r, 1, 0)?;
        Some((to_f64(&x), to_f64(&y)))
    }

    /// The position the witness gives an object with a position of its own (11.1).
    pub fn position(&self, object: usize) -> Option<(f64, f64)> {
        if !*self.model.own.get(object)? {
            return None;
        }
        let [x, y] = &self.witness.geometry.origin[object];
        Some((to_f64(x), to_f64(y)))
    }

    /// The orientation the witness gives an object that has one (11.2).
    pub fn orientation(&self, object: usize) -> Option<Orient> {
        self.model.oriented.get(object)?.then(|| self.witness.geometry.orient[object])
    }
}

/// The value of a reference on an axis under a placement, from the problem itself (14.1): the
/// origin, a pin turned with its object, a text in its symbol, the middle of the extent of a set.
pub(crate) fn point_value(problem: &LevelProblem, g: &Geometry, r: &PointRef, axis: usize, depth: usize) -> Option<Q> {
    if depth > MAX_DEPTH {
        return None;
    }
    let origin = |k: usize| -> Option<Q> {
        match &problem.objects.get(k)?.position {
            Position::Own { .. } => Some(g.origin[k][axis].clone()),
            Position::At(main) => point_value(problem, g, main, axis, depth + 1),
            // on the axis that the scene gives it, a text is where it is placed (13.3)
            Position::InSymbol { replaced: Some((a, _)), .. } if ax(*a) == axis => Some(g.origin[k][axis].clone()),
            Position::InSymbol { owner, local, .. } => {
                let base = point_value(problem, g, &PointRef::Origin(*owner), axis, depth + 1)?;
                let local = (from_f64(local.0)?, from_f64(local.1)?);
                Some(base + turned(g.orient[*owner], &local, axis))
            }
        }
    };
    match r {
        PointRef::Origin(k) => origin(*k),
        PointRef::Pin { object, pin, .. } => {
            let o = problem.objects.get(*object)?;
            let pos = problem.symbols.get(o.symbol?)?.pins.get(*pin)?.pos;
            let local = (from_f64(pos.0)?, from_f64(pos.1)?);
            Some(origin(*object)? + turned(g.orient[*object], &local, axis))
        }
        PointRef::Centre(members) => {
            let values: Vec<Q> =
                members.iter().map(|m| point_value(problem, g, m, axis, depth + 1)).collect::<Option<_>>()?;
            let lo = values.iter().min()?;
            let hi = values.iter().max()?;
            Some((lo + hi) / Q::from_integer(2.into()))
        }
    }
}

/// Resolves the facts of a level (8.5), with the time limit of the options on `clock` (the draft
/// 0.3, 8.5): the facts first, then a first minimal conflict group of each fact discarded, then
/// the other groups of each, within the one limit; then, counted in work and not timed, the
/// witness taken as written (see the module).
pub(crate) fn resolve(problem: &LevelProblem, options: &SolverOptions, clock: &dyn Clock) -> Resolved {
    let tol = from_f64(options.tolerance)
        .filter(|t| *t >= Q::zero())
        .unwrap_or_else(|| from_f64(rnl_runtime::geom::TOL).unwrap_or_default());
    let model = Model::new(problem, tol);
    let n = problem.objects.len();
    let deadline = Deadline::new(clock, options.time_limit);
    let limits = Limits::Time(&deadline);
    let seconds = fmt_num(options.time_limit.unwrap_or_default());
    let mut work = Work::default();
    let mut kept = Vec::new();
    let mut conflicts: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut undecided = Vec::new();
    let mut warnings = Vec::new();
    let mut r = Resolver::new(&model, n, limits);
    for f in 0..problem.facts.len() {
        match r.add(f) {
            Outcome::Kept => kept.push(f),
            Outcome::Conflict(set) => conflicts.push((f, set)),
            Outcome::Undecided => {
                warnings.push(format!(
                    "the fact {} was not decided within the time limit of {seconds} s and is discarded (8.5)",
                    fact_name(problem, f)
                ));
                undecided.push(f);
            }
        }
    }
    work.resolve_nodes += r.nodes;
    work.pivots += r.pivots();
    // The minimal conflict groups, still within the limit (the draft 0.3, 8.5): first one group
    // of each fact discarded, the deletion filter of its explanation, so that every fact has one
    // before the time goes to the others; then the others of each, in the order of the facts.
    let mut discarded: Vec<Discarded> = undecided
        .iter()
        .map(|&fact| Discarded { fact, groups: Vec::new(), complete: false, undecided: true })
        .collect();
    let mut found = Vec::new();
    for (f, set) in conflicts {
        let (group, minimal) = minimize(&model, n, limits, f, set, &mut work);
        let groups = if minimal { vec![group] } else { Vec::new() };
        found.push(Discarded { fact: f, groups, complete: false, undecided: false });
    }
    for d in &mut found {
        let Some(first) = d.groups.first().cloned() else { continue };
        let before: Vec<usize> = kept.iter().copied().take_while(|&k| k < d.fact).collect();
        (d.groups, d.complete) = all_groups(&model, n, limits, d.fact, &before, first, &mut work);
    }
    discarded.extend(found);
    discarded.sort_by_key(|d| d.fact);
    let statements = statement_hints(problem, &discarded);
    // The tolerance decides conflicts (11.7); it need not move what the facts do not move. When
    // the facts kept hold with their constants as written, the witness is taken from them so: a
    // PLACE at 4 puts the object at 4, not at 4 less the tolerance, where a simplex stops.
    // Otherwise as many of them as can, in the order of 8.5 (see the module). Not timed, so that
    // it is the same whenever every fact is decided; when one is not, the witness of the
    // resolution, which satisfies the facts kept within the tolerance.
    let placed = if undecided.is_empty() { taken_as_written(problem, &model, &kept, &mut work) } else { None };
    let (witness, as_written, placing) = placed.unwrap_or_else(|| (witness_of(&r, &model), false, model.clone()));
    drop(r);
    Resolved { kept, discarded, statements, work, warnings, notes: Vec::new(), witness, model, as_written, placing }
}

/// The hints of 8.5 by statement (the draft 0.3, 8.5): for each statement whose facts are in a
/// minimal conflict group of a fact discarded, those facts and the facts discarded because of them.
pub(crate) fn statement_hints(problem: &LevelProblem, discarded: &[Discarded]) -> BTreeMap<Statement, StatementHint> {
    let mut by: BTreeMap<Statement, (BTreeSet<usize>, BTreeSet<usize>)> = BTreeMap::new();
    for d in discarded {
        for k in d.in_groups() {
            let (facts, lost) = by.entry(Statement::of(problem, k)).or_default();
            facts.insert(k);
            lost.insert(d.fact);
        }
    }
    by.into_iter()
        .map(|(s, (facts, lost))| {
            (s, StatementHint { facts: facts.into_iter().collect(), discarded: lost.into_iter().collect() })
        })
        .collect()
}

/// A fact as a warning names it: its index, its label, where its statement is written and the
/// definitions whose usages expanded to it, outermost first (8.5, 7.6).
fn fact_name(problem: &LevelProblem, f: usize) -> String {
    let fact = &problem.facts[f];
    let mut place = Vec::new();
    if let Some(l) = &fact.source {
        place.push(format!("{}:{}", file_name(&l.file), l.line));
    }
    let defs: Vec<&str> =
        fact.chain.iter().filter_map(|u| problem.usages.get(*u)).map(|u| u.definition.as_str()).collect();
    if !defs.is_empty() {
        place.push(format!("from {}", defs.join(" in ")));
    }
    if place.is_empty() {
        format!("[{f}] {}", fact.label)
    } else {
        format!("[{f}] {} ({})", fact.label, place.join(", "))
    }
}

/// The placement a resolution has reached, with every variable of `model` on each axis.
fn witness_of(r: &Resolver, model: &Model) -> Witness {
    let geometry = r.geometry().clone();
    let classes = r.classes().clone();
    let values = [0, 1].map(|axis| (0..model.vars.len()).map(|v| geometry.var_value(model, v, axis)).collect());
    Witness { geometry, values, classes }
}

/// The witness taken from the facts kept with their constants as written, as far as they hold so
/// (see the module), whether all of them do, and the model it is taken from; none when the facts
/// kept do not hold again on that model, as when a cap runs out. Bounded by the caps of work
/// ([`CAPS`]), not timed.
fn taken_as_written(
    problem: &LevelProblem,
    model: &Model,
    kept: &[usize],
    work: &mut Work,
) -> Option<(Witness, bool, Model)> {
    let n = problem.objects.len();
    let limits = Limits::Caps(CAPS);
    let exact = Model::new(problem, Q::zero());
    let mut e = Resolver::new(&exact, n, limits);
    let all = kept.iter().all(|&f| e.add(f) == Outcome::Kept);
    work.resolve_nodes += e.nodes;
    work.pivots += e.pivots();
    let witness = all.then(|| witness_of(&e, model));
    drop(e);
    if let Some(witness) = witness {
        return Some((witness, true, exact));
    }
    let placing = Model::with_exact(problem, model.tol.clone(), held_as_written(problem, model, kept, work));
    let mut w = Resolver::new(&placing, n, limits);
    let holds = kept.iter().all(|&f| w.add(f) == Outcome::Kept);
    work.resolve_nodes += w.nodes;
    work.pivots += w.pivots();
    let witness = holds.then(|| witness_of(&w, model));
    drop(w);
    Some((witness?, false, placing))
}

/// The facts kept whose constants can hold as written, in the order of 8.5: each in turn is held
/// so when the facts kept still hold together with it and those before it held so, the others
/// within the tolerance of `model`. A fact without a constant to widen is the same either way.
/// Bounded by the caps of work ([`CAPS`]): a trial that reaches one does not hold its fact so.
fn held_as_written(problem: &LevelProblem, model: &Model, kept: &[usize], work: &mut Work) -> BTreeSet<usize> {
    let n = problem.objects.len();
    let mut exact = BTreeSet::new();
    for &f in kept {
        let widened = matches!(
            problem.facts[f].kind,
            FactKind::Place { .. }
                | FactKind::PointCoordinate { .. }
                | FactKind::Offset { .. }
                | FactKind::Order { .. }
                | FactKind::Distance { .. }
        );
        if !widened {
            continue;
        }
        exact.insert(f);
        let trial = Model::with_exact(problem, model.tol.clone(), exact.clone());
        let mut t = Resolver::new(&trial, n, Limits::Caps(CAPS));
        let holds = kept.iter().all(|&g| t.add(g) == Outcome::Kept);
        work.resolve_nodes += t.nodes;
        work.pivots += t.pivots();
        if !holds {
            exact.remove(&f);
        }
    }
    exact
}

/// The deletion filter (steps 3 and 4 of the module): a minimal subset of `set` that `f`
/// conflicts with, the facts of the latest rank left out first, and whether it is minimal. When
/// `f` still conflicts without a fact, the set shrinks to what that test's own explanation names,
/// which leaves out at once the facts the conflict does not need. When the time is out, before
/// the filter or during it, the filter stops, and the set it has reached, which still conflicts
/// with `f`, is not known to be minimal: a test that did not finish keeps its fact. The caller
/// then leaves it out of the groups. An empty set is minimal as it is.
fn minimize(model: &Model, n: usize, limits: Limits, f: usize, set: Vec<usize>, work: &mut Work) -> (Vec<usize>, bool) {
    let mut set = set;
    let mut tried: BTreeSet<usize> = BTreeSet::new();
    // the latest fact of the set not tried yet
    while let Some(c) = set.iter().rev().copied().find(|c| !tried.contains(c)) {
        if limits.was_out() {
            return (set, false);
        }
        tried.insert(c);
        let rest: Vec<usize> = set.iter().copied().filter(|&g| g != c).collect();
        let mut r = Resolver::new(model, n, limits);
        let fits = rest.iter().all(|&g| r.add(g) == Outcome::Kept);
        if fits {
            if let Outcome::Conflict(smaller) = r.add(f) {
                // what the conflict without `c` needs: a subset of `rest`
                set = if smaller.iter().all(|g| rest.contains(g)) { smaller } else { rest };
            }
        }
        work.resolve_nodes += r.nodes;
        work.pivots += r.pivots();
        if limits.was_out() {
            return (set, false);
        }
    }
    (set, true)
}

/// Every minimal conflict group of `f` (step 4 of the module): the minimal subsets of `before`,
/// the facts kept before `f` in order, that have no solution together with `f`, `first` among
/// them; sorted by rank inside and between groups, and whether they are all, which they are
/// unless the time is out before the enumeration ends.
///
/// MARCO (Liffiton, Previti, Malik and Marques-Silva, 2016) over the deletion filter: a [`Map`]
/// of the subsets not yet explored gives a seed, a largest subset that holds no group found whole
/// and is no subset of a set found to fit. A seed that fits with `f` is a largest set that does,
/// as each fact more would complete a group: every later seed must hold one fact outside it. A
/// seed that does not fit holds a group not found yet; [`minimize`] makes its explanation
/// minimal, and every later seed must leave out one fact of that group. When the map has no seed
/// left, every group is found: a group not found would be a subset of no set that fits and hold
/// none of the groups found.
fn all_groups(
    model: &Model,
    n: usize,
    limits: Limits,
    f: usize,
    before: &[usize],
    first: Vec<usize>,
    work: &mut Work,
) -> (Vec<Vec<usize>>, bool) {
    let at = |facts: &[usize]| -> Vec<usize> { facts.iter().filter_map(|g| before.binary_search(g).ok()).collect() };
    let mut map = Map { size: before.len(), groups: vec![at(&first)], fits: Vec::new() };
    let mut groups = vec![first];
    let complete = loop {
        if limits.out_of_time() {
            break false;
        }
        let seed = match map.seed(limits) {
            Seed::Found(seed) => seed,
            Seed::None => break true,
            Seed::Stopped => break false,
        };
        let facts: Vec<usize> = seed.iter().map(|&i| before[i]).collect();
        let mut r = Resolver::new(model, n, limits);
        let fits = facts.iter().all(|&g| r.add(g) == Outcome::Kept);
        // the facts kept before `f` hold together, so every seed does, unless the time is out
        let outcome = if fits { r.add(f) } else { Outcome::Undecided };
        work.resolve_nodes += r.nodes;
        work.pivots += r.pivots();
        match outcome {
            Outcome::Kept => map.fits.push((0..before.len()).filter(|i| seed.binary_search(i).is_err()).collect()),
            Outcome::Conflict(explanation) => {
                let (group, minimal) = minimize(model, n, limits, f, explanation, work);
                if !minimal {
                    break false;
                }
                map.groups.push(at(&group));
                groups.push(group);
            }
            Outcome::Undecided => break false,
        }
    };
    groups.sort();
    (groups, complete)
}

/// What [`Map::seed`] gives.
enum Seed {
    /// A seed, by position, sorted.
    Found(Vec<usize>),
    /// No subset is left to explore.
    None,
    /// The time was out.
    Stopped,
}

/// The subsets of the facts that a fact discarded may conflict with, by position among them, that
/// MARCO has not explored yet ([`all_groups`]): those that hold no group found whole and are no
/// subset of a set found to fit.
struct Map {
    /// How many facts there are.
    size: usize,
    /// The groups found: a seed leaves out one position of each.
    groups: Vec<Vec<usize>>,
    /// The positions that each set found to fit leaves out: a seed holds one of each.
    fits: Vec<Vec<usize>>,
}

/// How many nodes the search of a seed makes between two readings of the clock.
const SEED_NODES_PER_READ: u64 = 4096;

impl Map {
    /// A seed: a subset not explored, as large as can be, so that no position can be added without
    /// completing a group found. The positions are taken in, in order, before they are left out,
    /// so that the seed leans towards the facts that rank first; a search with backtracking, in
    /// which a position is in or out, finds a subset not explored, and the positions left out that
    /// complete no group are then taken in. None when every subset is explored.
    fn seed(&self, limits: Limits) -> Seed {
        if self.groups.iter().chain(&self.fits).any(Vec::is_empty) {
            return Seed::None;
        }
        // the clauses of each position
        let mut groups_of: Vec<Vec<usize>> = vec![Vec::new(); self.size];
        for (c, g) in self.groups.iter().enumerate() {
            for &i in g {
                groups_of[i].push(c);
            }
        }
        let mut fits_of: Vec<Vec<usize>> = vec![Vec::new(); self.size];
        for (c, s) in self.fits.iter().enumerate() {
            for &i in s {
                fits_of[i].push(c);
            }
        }
        let mut value: Vec<Option<bool>> = vec![None; self.size];
        let mut i = 0;
        let mut nodes: u64 = 0;
        while i < self.size {
            let next = match value[i] {
                None => Some(true),
                Some(true) => Some(false),
                Some(false) => None,
            };
            let Some(v) = next else {
                // both values of `i` fail: back to the position before
                value[i] = None;
                if i == 0 {
                    return Seed::None;
                }
                i -= 1;
                continue;
            };
            value[i] = Some(v);
            nodes += 1;
            if nodes % SEED_NODES_PER_READ == 0 && limits.out_of_time() {
                return Seed::Stopped;
            }
            // only the clauses of `i` can have changed: a group all in, or a set to meet all out
            let whole = |g: &Vec<usize>, want: bool| g.iter().all(|&j| value[j] == Some(want));
            let fails = if v {
                groups_of[i].iter().any(|&c| whole(&self.groups[c], true))
            } else {
                fits_of[i].iter().any(|&c| whole(&self.fits[c], false))
            };
            if !fails {
                i += 1;
            }
        }
        let mut seed: Vec<bool> = value.iter().map(|v| *v == Some(true)).collect();
        for i in 0..self.size {
            if !seed[i] {
                seed[i] = true;
                if groups_of[i].iter().any(|&c| self.groups[c].iter().all(|&j| seed[j])) {
                    seed[i] = false;
                }
            }
        }
        Seed::Found((0..self.size).filter(|&i| seed[i]).collect())
    }
}

#[cfg(test)]
pub(crate) mod tests;
