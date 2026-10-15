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
//! The numbers are computed for every version, for the solver's view of a level, and with the
//! draft for the check of the record of the solver: one line for each number
//! ([`Interp::number_lines`]).

use std::collections::HashMap;
use std::rc::Rc;

use crate::numbering::{check, LevelNumbering, RecordNumbering};
use crate::problem::written_name;
use crate::syntax::statement_tokens;

use super::ctx::{Ctx, StmtSpan};
use super::obj::*;
use super::Interp;

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

    /// The numbers of a drawn level and their checks (5.9), for `Interpretation::numbering`.
    pub(crate) fn level_numbering(&mut self, s: SceneId) -> LevelNumbering {
        let authored = self.numbered(s, None, None);
        let lines = self.number_lines(s, &authored);
        let mut records = Vec::new();
        for g in self.level_records(s).solver {
            let list = self.numbered(s, Some(g), None);
            let lines = self.number_lines(s, &list);
            let at = self.recs[g].loc(0);
            records.push(RecordNumbering { file: at.file, line: at.line, check: check(&lines), lines });
        }
        LevelNumbering { level: self.scenes[s].level, check: check(&lines), authored: lines, records }
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
