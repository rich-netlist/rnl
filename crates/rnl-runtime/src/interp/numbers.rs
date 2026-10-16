//! The numbers of the objects without a name (the draft 0.3, 5.9).
//!
//! The record of the solver refers by number to the objects that no name reaches, so that it can
//! write back every decision: the points, the wires, the texts without a key, the bundles, the
//! graphics in the scene and the usages. Each is noted when it is made in a drawing level
//! ([`Birth`]): the statement and the token that make it, the member of a set it is made for, and
//! the usage whose body it is in. The objects made by the level's own records are then numbered:
//!
//! 1. those of the authored records, the level's own text first and then the records written into
//!    it with `scope=` (2.6), with the bodies of the usages that they make. They are counted as the
//!    level reads without its records of the solver, as a solver reads it, and keep their numbers
//!    when it is read with them: a reading with a record of the solver of a file of the draft first
//!    reads the document without those records ([`NumberRun::authored`]), and its objects take the
//!    numbers of the objects made there in the same way ([`Interp::identity`]);
//! 2. those of one record of the solver (`type="generated"`, in the level's own text), with the
//!    bodies of the usages that it makes, counted on the record as written, every number in it
//!    read. A number may be used before the member that makes its object: when that member is
//!    still being read, the document is read again with the objects counted the first time
//!    ([`NumberRun::own`]).
//!
//! In a record, the objects go by the token that makes them, in the order of the text (the `(` of
//! a point, the keyword of the others), not in the order of evaluation; one token that makes an
//! object for each member of a set gives each its number, in the order of the members; the
//! objects of the body of a usage come right after it, in the order of the body, depth first.
//! Not numbered: an object that the member of a record or of a definition body making it binds to
//! a name (`%x =`, `=> %x`, a local name in a body; not a name bound inside a `{…}`, which holds
//! only there, 6.7), and what rules make (10.5). What `::` reaches from another object, such as a
//! local object or a text with a key, is never noted. Numbers start at 1.
//!
//! The numbers are computed for every version, for the solver's view of a level; they are read,
//! as `%#3`, only in a file of the draft. The check of a record of the solver is made of one line
//! for each number ([`Interp::number_lines`]).

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::host::LevelId;
use crate::numbering::{check, LevelNumbering, RecordNumbering};
use crate::problem::written_name;
use crate::syntax::{statement_tokens, uses_number, Ref};

use super::ctx::{Ctx, StmtSpan};
use super::eval::{Stop, R};
use super::obj::*;
use super::unit::{UnitKind, UnitMember};
use super::Interp;

/// The hint of a record of the solver whose numbering is out of date (5.9).
pub(crate) const STALE_HINT: &str = "this record of the solver is out of date for the objects without a name: its numbering is not the check of the level, so its statements that use a number take no effect; solve the level again (5.9)";

/// The error of a number written in a definition body (5.9).
pub(crate) fn in_body_message(r: &Ref) -> String {
    format!(
        "{}: a number is written only in the statements of the record of the solver, not in a definition body (5.9)",
        r.text()
    )
}

/// An object of a kind that may be numbered, as it was made.
#[derive(Clone, Debug)]
pub(crate) struct Birth {
    pub obj: ObjId,
    pub scene: SceneId,
    /// The member of a record being evaluated when it was made: in a body, the statement that made
    /// the outermost usage.
    pub root: MemberKey,
    /// The usage whose body made it.
    pub parent: Option<ObjId>,
    /// The statement that made it: of the record, or of the body.
    pub stmt: Option<StmtSpan>,
    /// The token that made it, in the record of `stmt`.
    pub off: usize,
    /// The member of a set it was made for, with its place in the set, when the token made one
    /// object for each member (5.2.1).
    pub each: Option<(usize, ObjId)>,
    /// How many times the same token made an object for the same member before, a set holding a
    /// member more than once (5.2).
    pub repeat: usize,
    /// The evaluation of a member that made it (`Interp::evals`).
    pub eval: u64,
}

/// A numbered object as a reading counted it: the member of a record whose evaluation made it,
/// and how it was made ([`Interp::identity`]), which another reading of the document compares.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Counted {
    pub root: MemberKey,
    pub identity: Rc<str>,
}

/// The objects of the authored records of a level in the order of their numbers, as the level
/// reads without its records of the solver (5.9), with the lines of the check.
#[derive(Clone, Debug, Default)]
pub(crate) struct AuthoredCount {
    pub objects: Vec<Counted>,
    pub lines: Vec<String>,
}

/// What a reading of the interpreter takes from the readings before it (5.9).
#[derive(Clone, Debug, Default)]
pub(crate) struct NumberRun {
    /// Records of the solver without `numbering` that use numbers: their numbers are errors (2.8).
    pub refuse: HashSet<RecId>,
    /// Records of the solver that use numbers and whose `numbering` is not the check of the level:
    /// their statements that use a number are left out, and the record gives one hint.
    pub stale: HashSet<RecId>,
    /// This reading, without the records of the solver, counts the objects of the authored
    /// records for another (`Interpretation::counted`).
    pub count: bool,
    /// The objects of the authored records of each level as the reading without the records of
    /// the solver counted them: this reading gives their numbers to the objects made as they were.
    pub authored: Option<Rc<HashMap<LevelId, AuthoredCount>>>,
    /// The record's own objects of each record of the solver, as a reading before this one counted
    /// them: a number of one of them resolves to the object made as it was.
    pub own: Option<Rc<HashMap<RecId, Vec<Counted>>>>,
}

impl NumberRun {
    /// No record of the solver to read differently.
    pub fn is_empty(&self) -> bool {
        self.refuse.is_empty() && self.stale.is_empty()
    }
}

/// The numbered objects of a level in a reading (5.9).
#[derive(Debug)]
pub(crate) struct LevelNumbers {
    /// The objects of the authored records by number from 1, None for a number whose object this
    /// reading does not make.
    pub authored: Vec<Option<ObjId>>,
    /// Their lines, as the reading that counted them gives them.
    pub authored_lines: Vec<String>,
    /// Each record of the solver read, with its own objects in order, numbered after those.
    pub own: Vec<(RecId, Vec<ObjId>)>,
}

/// The records whose statements make the numbered objects of a level.
struct LevelRecords {
    /// The authored records, the level's own and those written into it with `scope=`, by their
    /// place in the order of 2.6.
    authored: HashMap<RecId, usize>,
    /// The records of the solver in the level's own text, in order.
    solver: Vec<RecId>,
}

impl Interp<'_> {
    /// Note an object that may be numbered as it is made in scene `s`, by the token at `off` of
    /// the statement being evaluated; `each` is the member of a set it is made for (5.9). A text
    /// with a key is not noted: `::` reaches it.
    pub(crate) fn note_birth(&mut self, s: SceneId, obj: ObjId, ctx: &Ctx, off: usize, each: Option<(usize, ObjId)>) {
        if matches!(self.o(obj), ObjData::KeyText { .. }) || self.birth_of.contains_key(&obj) {
            return;
        }
        let parent = ctx.frame_usage();
        let repeat = match each {
            Some((_, m)) => {
                let seen = self.each_seen.entry((s, parent, ctx.stmt.and_then(|st| st.rec), off, m)).or_default();
                *seen += 1;
                *seen - 1
            }
            None => 0,
        };
        self.birth_of.insert(obj, self.births.len());
        self.births_by_root.entry((s, ctx.member_key.clone())).or_default().push(self.births.len());
        self.births.push(Birth {
            obj,
            scene: s,
            root: ctx.member_key.clone(),
            parent,
            stmt: ctx.stmt,
            off,
            each,
            repeat,
            eval: self.evals.last().copied().unwrap_or(0),
        });
    }

    /// A name of a member of a record or of a definition body is bound to `val`: when that member
    /// made it, it is the object's own name, and the object is not numbered (5.9).
    pub(crate) fn note_named(&mut self, val: ObjId) {
        if let (Some(&i), Some(&e)) = (self.birth_of.get(&val), self.evals.last()) {
            if self.births[i].eval == e {
                self.named.insert(val);
            }
        }
    }

    fn level_records(&self, s: SceneId) -> LevelRecords {
        let u = self.level_unit[self.scenes[s].level];
        let mut authored = HashMap::new();
        let mut solver = Vec::new();
        for &rec in &self.unit_recs[u] {
            let Some(h) = self.rec_header.get(&rec) else { continue };
            if h.scope.is_some() {
                continue; // its statements belong to the subcircuit it names (2.6)
            }
            match h.type_.as_str() {
                "authored" => {
                    authored.insert(rec, authored.len());
                }
                "generated" => solver.push(rec),
                _ => {}
            }
        }
        // then the records written into it with scope=, in the order they take effect (2.6)
        for m in &self.units[u].scope_members {
            if let Some(rec) = m.rec.filter(|_| m.header.type_ == "authored") {
                let n = authored.len();
                authored.entry(rec).or_insert(n);
            }
        }
        LevelRecords { authored, solver }
    }

    /// Does this reading count the objects of the authored records as a reading without the
    /// records of the solver does (5.9)? When it reads a record of the solver of a file of 0.3;
    /// then that reading comes first. A file of 0.2 never needs it.
    pub(crate) fn needs_count(&self) -> bool {
        self.read_generated
            && self.rec_header.iter().any(|(&rec, h)| {
                let u = self.rec_unit[rec];
                h.type_ == "generated"
                    && h.scope.is_none()
                    && self.units[u].kind == UnitKind::Level
                    && self.follows_0_3(u)
            })
    }

    /// Where an object goes among the objects of its record: the token that made it and its member
    /// of a set, after those of the usages whose bodies it is in, outermost first.
    fn birth_key(&self, i: usize) -> Vec<u64> {
        let b = &self.births[i];
        let mut key = match b.parent.and_then(|u| self.birth_of.get(&u)) {
            Some(&p) => self.birth_key(p),
            None => Vec::new(),
        };
        key.push(b.off as u64);
        key.push(b.each.map_or(0, |(k, _)| k as u64));
        key
    }

    /// Is a birth one of the numbered objects of the members of the level's records?
    fn numbered_birth(&self, s: SceneId, b: &Birth) -> bool {
        b.scene == s && !self.named.contains(&b.obj) && !self.made_by_rule(s, b.obj)
    }

    /// The objects numbered in scene `s`, in the order of their numbers, those of one part (5.9):
    /// with no record, those that the authored records make in this reading; with a record of the
    /// solver, its own, and with `upto` only those of its members before that one.
    fn in_order(&mut self, s: SceneId, solver: Option<RecId>, upto: Option<usize>) -> Rc<Vec<ObjId>> {
        let version = (self.births.len(), self.named.len());
        let key = (s, solver, upto);
        if let Some((v, list)) = self.numbered_memo.get(&key) {
            if *v == version {
                return list.clone();
            }
        }
        let u = self.level_unit[self.scenes[s].level];
        let recs = self.level_records(s);
        let mut keyed: Vec<(usize, Vec<u64>, ObjId)> = Vec::new();
        for i in 0..self.births.len() {
            let b = &self.births[i];
            let MemberKey::M { unit, rec, k, .. } = b.root else { continue };
            if unit != u || !self.numbered_birth(s, b) {
                continue;
            }
            let place = match solver {
                None => match recs.authored.get(&rec) {
                    Some(&n) => n,
                    None => continue,
                },
                Some(g) if g == rec && upto.is_none_or(|kk| k < kk) => 0,
                Some(_) => continue,
            };
            let obj = b.obj;
            keyed.push((place, self.birth_key(i), obj));
        }
        keyed.sort();
        let list = Rc::new(keyed.into_iter().map(|(_, _, o)| o).collect::<Vec<_>>());
        self.numbered_memo.insert(key, (version, list.clone()));
        list
    }

    /// The numbered objects of scene `s` in this reading (5.9): those of the authored records, as
    /// the reading without the records of the solver counted them when there was one, and those of
    /// each record of the solver read.
    pub(crate) fn level_numbers(&mut self, s: SceneId) -> Rc<LevelNumbers> {
        let version = (self.births.len(), self.named.len());
        if let Some((v, n)) = self.level_numbers_memo.get(&s) {
            if *v == version {
                return n.clone();
            }
        }
        let lv = self.scenes[s].level;
        let all = self.numbers.authored.clone();
        let (authored, authored_lines) = match all.as_ref().and_then(|a| a.get(&lv)) {
            Some(count) => {
                // the objects made here as the counted ones were made
                let recs = self.level_records(s);
                let u = self.level_unit[lv];
                let mut made: HashMap<(MemberKey, Rc<str>), ObjId> = HashMap::new();
                for i in 0..self.births.len() {
                    let b = &self.births[i];
                    let in_authored = matches!(b.root, MemberKey::M { unit, rec, .. } if unit == u && recs.authored.contains_key(&rec));
                    if !in_authored || !self.numbered_birth(s, b) {
                        continue;
                    }
                    let (root, obj) = (b.root.clone(), b.obj);
                    let identity = self.identity(obj);
                    made.entry((root, identity)).or_insert(obj);
                }
                let objects =
                    count.objects.iter().map(|c| made.get(&(c.root.clone(), c.identity.clone())).copied()).collect();
                (objects, count.lines.clone())
            }
            None => {
                let list = self.in_order(s, None, None);
                let numbers: HashMap<ObjId, usize> = list.iter().enumerate().map(|(k, o)| (*o, k + 1)).collect();
                let lines = self.number_lines(s, &numbers, &list);
                (list.iter().map(|o| Some(*o)).collect(), lines)
            }
        };
        let mut own = Vec::new();
        for g in self.level_records(s).solver {
            own.push((g, self.in_order(s, Some(g), None).to_vec()));
        }
        let n = Rc::new(LevelNumbers { authored, authored_lines, own });
        self.level_numbers_memo.insert(s, (version, n.clone()));
        n
    }

    /// The objects without a name in the bodies of the usages that rules make in scene `s`, by
    /// usage (5.9, 10.5). Such objects are not numbered; once the record of the solver writes their
    /// usage out as an explicit statement, they are made again as the record's own objects,
    /// numbered right after the usage, in the order of the body, depth first: each list is in that
    /// order. Reached through a name (a local name, or one of a usage in the body), an object is
    /// not in it.
    pub(crate) fn generated_bodies(&self, s: SceneId) -> HashMap<ObjId, Vec<ObjId>> {
        let mut keyed: HashMap<ObjId, Vec<(Vec<u64>, ObjId)>> = HashMap::new();
        for (i, b) in self.births.iter().enumerate() {
            if b.scene != s || self.named.contains(&b.obj) {
                continue;
            }
            // the usage a rule made that the body it is in belongs to: the outermost generated one
            let mut at = b.parent;
            while let Some(u) = at {
                if self.rule_made(u) {
                    keyed.entry(u).or_default().push((self.birth_key(i), b.obj));
                    break;
                }
                at = self.usage(u).parent;
            }
        }
        keyed
            .into_iter()
            .map(|(u, mut list)| {
                list.sort();
                (u, list.into_iter().map(|(_, o)| o).collect())
            })
            .collect()
    }

    /// The records of the solver in the level's own text of scene `s`, in order (5.9).
    pub(crate) fn solver_records(&self, s: SceneId) -> Vec<RecId> {
        self.level_records(s).solver
    }

    /// The objects without a name in the bodies of the usages that the statements of the record of
    /// the solver `g` make in scene `s`, by usage, each list in the order of the body, depth first,
    /// as [`Interp::generated_bodies`] gives those of the usages that rules make: once the record
    /// writes such a usage out, the objects of its body are these (5.9, 10.5).
    pub(crate) fn written_bodies(&self, s: SceneId, g: RecId) -> HashMap<ObjId, Vec<ObjId>> {
        let mut keyed: HashMap<ObjId, Vec<(Vec<u64>, ObjId)>> = HashMap::new();
        for (i, b) in self.births.iter().enumerate() {
            if !matches!(b.root, MemberKey::M { rec, .. } if rec == g) || !self.numbered_birth(s, b) {
                continue;
            }
            // the outermost usage whose body it is in
            let mut top = None;
            let mut at = b.parent;
            while let Some(u) = at {
                top = Some(u);
                at = self.usage(u).parent;
            }
            if let Some(u) = top {
                keyed.entry(u).or_default().push((self.birth_key(i), b.obj));
            }
        }
        keyed
            .into_iter()
            .map(|(u, mut list)| {
                list.sort();
                (u, list.into_iter().map(|(_, o)| o).collect())
            })
            .collect()
    }

    /// The numbers of the objects of scene `s` for its problem (5.9): those of the authored records,
    /// and after them, for each record of the solver read, the record's own objects.
    pub(crate) fn problem_numbers(&mut self, s: SceneId) -> HashMap<ObjId, usize> {
        let n = self.level_numbers(s);
        let mut out: HashMap<ObjId, usize> =
            n.authored.iter().enumerate().filter_map(|(k, o)| o.map(|o| (o, k + 1))).collect();
        for (_, own) in &n.own {
            for (k, o) in own.iter().enumerate() {
                out.entry(*o).or_insert(n.authored.len() + k + 1);
            }
        }
        out
    }

    /// `%#n` in a statement (5.9). Only the record of the solver, with `type="generated"` in the
    /// level's own text, of a file of the draft, writes numbers; its `numbering` decides afterwards
    /// whether they take effect (`NumberRun`). The numbers of the authored records are those of the
    /// reading without the records of the solver; the record's own follow, counted on the record
    /// in the order of its members, so that a number may be used before the member that makes its
    /// object, as a name may (6.7).
    pub(crate) fn resolve_number(&mut self, n: u64, e: &Ref, ctx: &Ctx) -> R<ObjId> {
        if ctx.frame.is_some() {
            let d = match self.body_errors.get(&super::eval::ptr(e)).copied() {
                Some(d) => d,
                None => self.err(ctx, in_body_message(e), e.start),
            };
            return Err(Stop::Fail(Some(d)));
        }
        let solver = ctx.rec.filter(|&r| {
            !ctx.scope_in
                && ctx.header.type_ == "generated"
                && self.rec_unit[r] == ctx.unit
                && self.units[ctx.unit].kind == UnitKind::Level
                && !self.numbers.refuse.contains(&r)
        });
        let (Some(g), Some(s)) = (solver, ctx.scene) else {
            let msg = format!("{}: a number is written only in the record of the solver, with type=\"generated\" and the field numbering, in the level's own text (2.6, 2.8, 5.9)", e.text());
            return self.fail(ctx, msg, e.start);
        };
        let u = ctx.unit;
        let beyond = |it: &mut Self, count: usize| -> R<ObjId> {
            let msg =
                format!("{}: no object without a name has this number; the level numbers {count} (5.9)", e.text());
            it.fail(ctx, msg, e.start)
        };
        // the objects of the authored records, as the level reads without its records of the solver
        let lv = self.scenes[s].level;
        let authored = self.numbers.authored.clone();
        let counted = authored.as_ref().and_then(|a| a.get(&lv));
        let n1 = match counted {
            Some(count) => count.objects.len(),
            None => {
                // no such reading: this one's, every authored member read first
                let recs = self.level_records(s);
                self.read_members(s, u, |rec, _| recs.authored.contains_key(&rec));
                self.in_order(s, None, None).len()
            }
        };
        let n = match usize::try_from(n) {
            Ok(n) if n >= 1 => n,
            _ => return beyond(self, n1),
        };
        if n <= n1 {
            return match counted {
                Some(count) => self.counted_object(s, &count.objects[n - 1], e, ctx),
                None => Ok(self.in_order(s, None, None)[n - 1]),
            };
        }
        // the record's own objects: as the reading before this one counted them
        let j = n - n1;
        if let Some(own) = self.numbers.own.clone() {
            let list = own.get(&g).map_or(&[][..], |l| &l[..]);
            return match list.get(j - 1) {
                Some(c) => self.counted_object(s, c, e, ctx),
                None => beyond(self, n1 + list.len()),
            };
        }
        // or counted here, member by member in order, until one makes the object
        let members: Vec<UnitMember> = self.units[u].members.iter().filter(|m| m.rec == Some(g)).cloned().collect();
        for m in members {
            let key = (m.key(u), Some(s));
            if self.in_progress.contains(&key) {
                // still being read, so not all its objects are made, and those of the members after
                // it are numbered after them: read again with the objects this reading counts
                self.forward = true;
                return Ok(self.new_obj(ObjData::Point { x: None, y: None, local: false }));
            }
            if !self.memo.contains_key(&key) {
                let _ = self.eval_member_at(u, m.rec, m.k, m.embedded, &m.m, Some(s), m.header.clone(), None, "");
            }
            let own = self.in_order(s, Some(g), Some(m.k + 1));
            if own.len() >= j {
                return Ok(own[j - 1]);
            }
        }
        let total = self.in_order(s, Some(g), None).len();
        beyond(self, n1 + total)
    }

    /// The object of this reading made as a reading counted it: the member that made it is read,
    /// and of its objects the one made the same way.
    fn counted_object(&mut self, s: SceneId, c: &Counted, e: &Ref, ctx: &Ctx) -> R<ObjId> {
        let key = (c.root.clone(), Some(s));
        if self.in_progress.contains(&key) {
            let msg = format!("{}: the object with this number is made by a member that is still being read, through this one: a cycle (5.9, 6.6)", e.text());
            return self.fail(ctx, msg, e.start);
        }
        if let (false, MemberKey::M { unit, rec, k, embedded }) = (self.memo.contains_key(&key), c.root.clone()) {
            let m = self.units[unit]
                .members
                .iter()
                .chain(&self.units[unit].scope_members)
                .find(|m| m.rec == Some(rec) && m.k == k && m.embedded == embedded)
                .cloned();
            if let Some(m) = m {
                let _ = self.eval_member_at(unit, m.rec, m.k, m.embedded, &m.m, Some(s), m.header.clone(), None, "");
            }
        }
        let births = self.births_by_root.get(&(s, c.root.clone())).cloned().unwrap_or_default();
        for i in births {
            let (numbered, obj) = (self.numbered_birth(s, &self.births[i]), self.births[i].obj);
            if numbered && self.identity(obj) == c.identity {
                return Ok(obj);
            }
        }
        let msg = format!(
            "{}: the object with this number is not made when the level is read with its records of the solver (5.9)",
            e.text()
        );
        self.fail(ctx, msg, e.start)
    }

    /// Read the members of the level of scene `s` that `which` takes, by record and place, each
    /// once, as the level reads it; one being read now is left.
    fn read_members(&mut self, s: SceneId, u: UnitId, which: impl Fn(RecId, usize) -> bool) {
        let members: Vec<UnitMember> = self.units[u]
            .members
            .iter()
            .chain(&self.units[u].scope_members)
            .filter(|m| m.rec.is_some_and(|r| which(r, m.k)))
            .cloned()
            .collect();
        for m in members {
            if self.restart.is_some() {
                return;
            }
            let key = (m.key(u), Some(s));
            if self.memo.contains_key(&key) || self.in_progress.contains(&key) {
                continue;
            }
            let _ = self.eval_member_at(u, m.rec, m.k, m.embedded, &m.m, Some(s), m.header.clone(), None, "");
        }
    }

    /// How an object was made, which another reading of the same document compares (5.9): the
    /// record and the token that made it, the member of a set it was made for, and the usage whose
    /// body it is in, each by how it was made in turn; a host object by its name, a terminal by
    /// its owner and its number, a position or a text with a key by what it belongs to.
    pub(crate) fn identity(&mut self, o: ObjId) -> Rc<str> {
        if let Some(v) = self.identities.get(&o) {
            return v.clone();
        }
        let text = if let Some(&i) = self.birth_of.get(&o) {
            let b = self.births[i].clone();
            let at = match b.stmt.and_then(|st| st.rec) {
                Some(r) => {
                    let l = self.recs[r].loc(0);
                    format!("{}:{}:{}", l.file, l.line, l.col)
                }
                None => String::new(),
            };
            let member = match b.each {
                Some((_, m)) => format!("{}~{}", self.identity(m), b.repeat),
                None => String::new(),
            };
            let parent = match b.parent {
                Some(p) => self.identity(p).to_string(),
                None => String::new(),
            };
            format!("{at}+{}[{member}]({parent})", b.off)
        } else if o == LEVEL {
            "#".to_string()
        } else {
            match self.o(o).clone() {
                ObjData::LevelObj { .. } => "#".to_string(),
                ObjData::Inst { .. } => format!("#{}", self.inst_of(o).name),
                ObjData::Net { key, .. } => format!("&{key}"),
                ObjData::Term { owner, index, .. } => format!("{}:{index}", self.identity(owner)),
                ObjData::Pos { parent } => format!("{}::position", self.identity(parent)),
                ObjData::KeyText { attach, key, .. } => format!("{}::{key}", self.identity(attach)),
                _ => self.label(o),
            }
        };
        let text: Rc<str> = Rc::from(text);
        self.identities.insert(o, text.clone());
        text
    }

    /// The objects of the authored records of a level as this reading, without the records of the
    /// solver, counts them for another (5.9).
    pub(crate) fn counted_authored(&mut self, s: SceneId) -> AuthoredCount {
        let list = self.in_order(s, None, None);
        let numbers: HashMap<ObjId, usize> = list.iter().enumerate().map(|(k, o)| (*o, k + 1)).collect();
        let lines = self.number_lines(s, &numbers, &list);
        let objects = list
            .iter()
            .map(|o| Counted { root: self.births[self.birth_of[o]].root.clone(), identity: self.identity(*o) })
            .collect();
        AuthoredCount { objects, lines }
    }

    /// The records' own objects of every record of the solver read, as this reading counts them
    /// for a reading after it (5.9).
    pub(crate) fn counted_own(&mut self) -> HashMap<RecId, Vec<Counted>> {
        let mut out = HashMap::new();
        for s in 0..self.scenes.len() {
            for g in self.level_records(s).solver {
                let list = self.in_order(s, Some(g), None);
                let counted = list
                    .iter()
                    .map(|o| Counted { root: self.births[self.birth_of[o]].root.clone(), identity: self.identity(*o) })
                    .collect();
                out.insert(g, counted);
            }
        }
        out
    }

    /// The numbers of a drawn level and their checks (5.9), for `Interpretation::numbering`.
    pub(crate) fn level_numbering(&mut self, s: SceneId) -> LevelNumbering {
        let numbers = self.level_numbers(s);
        let n1 = numbers.authored.len();
        let mut records = Vec::new();
        for (g, own) in &numbers.own {
            let mut by_object: HashMap<ObjId, usize> =
                numbers.authored.iter().enumerate().filter_map(|(k, o)| o.map(|o| (o, k + 1))).collect();
            for (k, o) in own.iter().enumerate() {
                by_object.insert(*o, n1 + k + 1);
            }
            let mut lines = numbers.authored_lines.clone();
            lines.extend(self.number_lines(s, &by_object, own));
            let at = self.recs[*g].loc(0);
            let written = self.rec_header.get(g).and_then(|h| h.numbering.clone());
            let ast = self.parse(*g);
            let uses_numbers = ast.body.as_ref().is_some_and(|b| b.members.iter().any(|m| uses_number(m)));
            records.push(RecordNumbering {
                file: at.file,
                line: at.line,
                check: check(&lines),
                lines,
                written,
                uses_numbers,
            });
        }
        let lines = numbers.authored_lines.clone();
        LevelNumbering { level: self.scenes[s].level, check: check(&lines), authored: lines, records }
    }

    /// The records of the solver that a second run reads differently (5.9): in a file of the draft,
    /// one that uses numbers without `numbering`, and one that uses numbers and whose `numbering`
    /// is not its check. A record that uses no number need not have `numbering`, and one that does
    /// not match changes nothing. The check is compared as hexadecimal digits, in either case.
    pub(crate) fn number_recheck(&mut self, numbering: &[LevelNumbering]) -> NumberRun {
        let mut out = NumberRun::default();
        for lvn in numbering {
            let Some(s) = self.level_scene[lvn.level] else { continue };
            let solver = self.level_records(s).solver;
            for (g, r) in solver.into_iter().zip(&lvn.records) {
                if !self.follows_0_3(self.rec_unit[g]) || !r.uses_numbers {
                    continue;
                }
                match &r.written {
                    None => {
                        out.refuse.insert(g);
                    }
                    Some(w) if !w.eq_ignore_ascii_case(&r.check) => {
                        out.stale.insert(g);
                    }
                    _ => {}
                }
            }
        }
        out
    }

    /// One line for each object of `list` (5.9): the tokens of the statement that makes it,
    /// without free text and comments, separated by one space; in the body of a usage, the line of
    /// the usage first, then a tab and the statement of the body as the definition writes it; when
    /// the statement makes one object for each member of a set, a tab and the member as a
    /// reference, an object without a name by its number in `numbers`.
    pub(crate) fn number_lines(&mut self, s: SceneId, numbers: &HashMap<ObjId, usize>, list: &[ObjId]) -> Vec<String> {
        let refs = self.line_refs(s, numbers);
        let mut texts: HashMap<StmtSpan, String> = HashMap::new();
        list.iter().map(|o| self.birth_line(self.birth_of[o], &refs, &mut texts)).collect()
    }

    fn birth_line(&mut self, i: usize, refs: &HashMap<ObjId, String>, texts: &mut HashMap<StmtSpan, String>) -> String {
        let b = self.births[i].clone();
        let mut line = match b.parent.and_then(|u| self.birth_of.get(&u).copied()) {
            Some(p) => self.birth_line(p, refs, texts) + "\t",
            None => String::new(),
        };
        if let Some(st) = b.stmt {
            let text = texts.entry(st).or_insert_with(|| match st.rec {
                Some(rec) => statement_tokens(&self.record_chars(rec), st.start, st.end).join(" "),
                None => String::new(),
            });
            line.push_str(text);
        }
        if let Some((_, m)) = b.each {
            line.push('\t');
            line.push_str(&self.member_handle(m, refs));
        }
        line
    }

    /// What the objects of a scene are referred to by in the lines: the level's `%` names, the
    /// numbers, and through `::` the local objects of the usages that have either.
    fn line_refs(&mut self, s: SceneId, numbers: &HashMap<ObjId, usize>) -> HashMap<ObjId, String> {
        let mut refs: HashMap<ObjId, String> = HashMap::new();
        for (name, o) in self.level_names(s) {
            refs.entry(o).or_insert_with(|| format!("%{}", written_name(&name)));
        }
        for (o, n) in numbers {
            refs.entry(*o).or_insert_with(|| format!("%#{n}"));
        }
        let mut usages: Vec<ObjId> =
            self.scenes[s].usages.iter().copied().filter(|a| !self.usage(*a).in_symbol).collect();
        usages.sort();
        usages.dedup();
        for a in usages {
            let Some(r) = refs.get(&a).cloned() else { continue };
            for (name, o) in self.usage(a).locals.clone() {
                refs.entry(o).or_insert_with(|| format!("{r}::{}", written_name(&name)));
            }
        }
        refs
    }

    /// A member of a set as a reference (5.9): a host instance or net by its name, a terminal by
    /// its owner and its number, such as `#M1:1`, an RNL object by its name or its number; the
    /// position and the keyed texts of an object through `::`. An object that has neither, such as
    /// one that only the reading with the records of the solver makes, by the name it was bound
    /// to, else as diagnostics show it.
    fn member_handle(&mut self, o: ObjId, refs: &HashMap<ObjId, String>) -> String {
        if o == LEVEL {
            return "#".into();
        }
        match self.o(o).clone() {
            ObjData::LevelObj { .. } => "#".into(),
            ObjData::Inst { .. } => format!("#{}", written_name(&self.inst_of(o).name)),
            ObjData::Net { display, .. } => format!("&{}", written_name(&display)),
            ObjData::Term { owner, index, .. } => {
                let base = self.member_handle(owner, refs);
                if base == "#" {
                    format!("#:{index}")
                } else {
                    format!("{base}:{index}")
                }
            }
            ObjData::Pos { parent } => format!("{}::position", self.member_handle(parent, refs)),
            ObjData::KeyText { attach, key, .. } => {
                format!("{}::{}", self.member_handle(attach, refs), written_name(&key))
            }
            _ => match refs.get(&o) {
                Some(r) => r.clone(),
                None => {
                    let name = &self.objs[o.0 as usize].name;
                    if name.is_empty() {
                        self.label(o)
                    } else if name.starts_with('$') {
                        name.clone()
                    } else {
                        format!("%{}", written_name(name))
                    }
                }
            },
        }
    }
}
