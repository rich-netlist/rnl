//! The numbers of the objects without a name (the draft 0.3, 5.9).
//!
//! The record of the solver refers by number to the objects that no name reaches, so that it can
//! write back every decision: the points, the wires, the texts without a key, the bundles, the
//! graphics in the scene and the usages. Each is noted when it is made in a drawing level
//! ([`Birth`]): the statement and the token that make it, the member of a set it is made for, and
//! the usage whose body it is in. The objects made by the level's own records are then put in order:
//!
//! 1. those of the authored records, the level's own text first and then the records written into
//!    it with `scope=` (2.6), with the bodies of the usages that they make;
//! 2. those of one record of the solver (`type="generated"`, in the level's own text), with the
//!    bodies of the usages that it makes.
//!
//! In a record, the objects go by the token that makes them, in the order of the text (the `(` of
//! a point, the keyword of the others), not in the order of evaluation; one token that makes an
//! object for each member of a set gives each its number, in the order of the members; the
//! objects of the body of a usage come right after it, in the order of the body, depth first.
//! Not numbered: an object that the statement making it binds to a name (`%x =`, `=> %x`, a
//! local name in a body), and what rules make (10.5). What `::` reaches from another object, such
//! as a local object or a text with a key, is never noted. Numbers start at 1.
//!
//! The numbers are computed for every version, for the solver's view of a level; they are read,
//! as `%#3`, only in a file of the draft. The check of a record of the solver is made of one line
//! for each number ([`Interp::number_lines`]).

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

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
    /// The evaluation of a member that made it (`Interp::evals`).
    pub eval: u64,
}

/// The records of the solver that a run of the interpreter reads differently (5.9).
#[derive(Clone, Debug, Default)]
pub(crate) struct NumberRun {
    /// Without `numbering`: their numbers are errors (2.8).
    pub refuse: HashSet<RecId>,
    /// Their `numbering` is not the check of the level: their statements that use a number are
    /// left out, and the record gives one hint.
    pub stale: HashSet<RecId>,
}

impl NumberRun {
    pub fn is_empty(&self) -> bool {
        self.refuse.is_empty() && self.stale.is_empty()
    }
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
        self.birth_of.insert(obj, self.births.len());
        self.births.push(Birth {
            obj,
            scene: s,
            root: ctx.member_key.clone(),
            parent: ctx.frame_usage(),
            stmt: ctx.stmt,
            off,
            each,
            eval: self.evals.last().copied().unwrap_or(0),
        });
    }

    /// A name is bound to `val`: when the member that binds it made it, it is the object's own name,
    /// and the object is not numbered (5.9).
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

    /// The objects numbered in scene `s` for the record of the solver `solver`, in the order of
    /// their numbers, the first being number 1 (5.9): those of the authored records, then those of
    /// the record itself. With no record, the first part alone; with `before`, only the statements
    /// of the record before its member `before`.
    pub(crate) fn numbered(&mut self, s: SceneId, solver: Option<RecId>, before: Option<usize>) -> Rc<Vec<ObjId>> {
        let version = (self.births.len(), self.named.len());
        let key = (s, solver, before);
        if let Some((v, list)) = self.numbered_memo.get(&key) {
            if *v == version {
                return list.clone();
            }
        }
        let u = self.level_unit[self.scenes[s].level];
        let recs = self.level_records(s);
        let mut keyed: Vec<((u8, usize), Vec<u64>, ObjId)> = Vec::new();
        for i in 0..self.births.len() {
            let b = &self.births[i];
            if b.scene != s || self.named.contains(&b.obj) {
                continue;
            }
            let MemberKey::M { unit, rec, k, .. } = b.root else { continue };
            if unit != u {
                continue;
            }
            let place = match recs.authored.get(&rec) {
                Some(&n) => (0, n),
                None if Some(rec) == solver && before.is_none_or(|kk| k < kk) => (1, 0),
                None => continue,
            };
            let obj = b.obj;
            if self.made_by_rule(s, obj) {
                continue;
            }
            keyed.push((place, self.birth_key(i), obj));
        }
        keyed.sort();
        let list = Rc::new(keyed.into_iter().map(|(_, _, o)| o).collect::<Vec<_>>());
        self.numbered_memo.insert(key, (version, list.clone()));
        list
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

    /// The numbers of the objects of scene `s` for its problem (5.9): those of the authored records,
    /// and after them, for each record of the solver read, the record's own objects.
    pub(crate) fn problem_numbers(&mut self, s: SceneId) -> HashMap<ObjId, usize> {
        let authored = self.numbered(s, None, None);
        let mut out: HashMap<ObjId, usize> = authored.iter().enumerate().map(|(k, o)| (*o, k + 1)).collect();
        for g in self.level_records(s).solver {
            let list = self.numbered(s, Some(g), None);
            for (k, o) in list.iter().enumerate().skip(authored.len()) {
                out.entry(*o).or_insert(k + 1);
            }
        }
        out
    }

    /// `%#n` in a statement (5.9). Only the record of the solver, with `type="generated"` in the
    /// level's own text, of a file of the draft, writes numbers; its `numbering` decides afterwards
    /// whether they take effect (`NumberRun`). A number is the object of that place among the
    /// objects of the authored records and those that the statements of the record before this one
    /// make: those are read first, as the level reads them.
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
        let (Some(g), Some(s), MemberKey::M { k: current, .. }) = (solver, ctx.scene, ctx.member_key.clone()) else {
            let msg = format!("{}: a number is written only in the record of the solver, with type=\"generated\" and the field numbering, in the level's own text (2.6, 2.8, 5.9)", e.text());
            return self.fail(ctx, msg, e.start);
        };
        self.read_before_number(s, ctx.unit, g, current);
        let list = self.numbered(s, Some(g), Some(current));
        match usize::try_from(n).ok().filter(|&k| k >= 1 && k <= list.len()) {
            Some(k) => Ok(list[k - 1]),
            None => {
                let msg = format!("{}: no object without a name has this number; the authored records of the level and the statements of this record before this one number {} (5.9)", e.text(), list.len());
                self.fail(ctx, msg, e.start)
            }
        }
    }

    /// Read what a number in the member `current` of the record of the solver `g` counts: the
    /// members of the authored records of the level, and those of the record before it. Each is
    /// read once, as the level reads it; one being read now is left.
    fn read_before_number(&mut self, s: SceneId, u: UnitId, g: RecId, current: usize) {
        let recs = self.level_records(s);
        let members: Vec<UnitMember> = self.units[u]
            .members
            .iter()
            .chain(&self.units[u].scope_members)
            .filter(|m| m.rec.is_some_and(|r| recs.authored.contains_key(&r) || (r == g && m.k < current)))
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

    /// The numbers of a drawn level and their checks (5.9), for `Interpretation::numbering`.
    pub(crate) fn level_numbering(&mut self, s: SceneId) -> LevelNumbering {
        let authored = self.numbered(s, None, None);
        let lines = self.number_lines(s, &authored);
        let mut records = Vec::new();
        for g in self.level_records(s).solver {
            let list = self.numbered(s, Some(g), None);
            let lines = self.number_lines(s, &list);
            let at = self.recs[g].loc(0);
            let written = self.rec_header.get(&g).and_then(|h| h.numbering.clone());
            let ast = self.parse(g);
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
        LevelNumbering { level: self.scenes[s].level, check: check(&lines), authored: lines, records }
    }

    /// The records of the solver that a second run reads differently (5.9): in a file of the draft,
    /// one that uses numbers without `numbering`, and one whose `numbering` is not its check. The
    /// check is compared as hexadecimal digits, in either case.
    pub(crate) fn number_recheck(&mut self, numbering: &[LevelNumbering]) -> NumberRun {
        let mut out = NumberRun::default();
        for lvn in numbering {
            let Some(s) = self.level_scene[lvn.level] else { continue };
            let solver = self.level_records(s).solver;
            for (g, r) in solver.into_iter().zip(&lvn.records) {
                if !self.follows_0_3(self.rec_unit[g]) {
                    continue;
                }
                match &r.written {
                    None if r.uses_numbers => {
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

    /// One line for each object of a list in the order of their numbers (5.9): the tokens of the
    /// statement that makes it, without free text and comments, separated by one space; in the
    /// body of a usage, the line of the usage first, then a tab and the statement of the body as the
    /// definition writes it; when the statement makes one object for each member of a set, a tab
    /// and the member as a reference, an object without a name by its number in the list.
    pub(crate) fn number_lines(&mut self, s: SceneId, list: &[ObjId]) -> Vec<String> {
        let numbers: HashMap<ObjId, usize> = list.iter().enumerate().map(|(k, o)| (*o, k + 1)).collect();
        let refs = self.line_refs(s, &numbers);
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
    /// one named only inside a `{…}`, by the name it was bound to, else as diagnostics show it.
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
