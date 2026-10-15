//! The interpreter (standard chapters 5–16): reads the records of a host document and its
//! packages, resolves names, evaluates every statement into objects and facts, and lets each
//! drawing level take effect phase by phase (8.2).
//!
//! Each drawing level (the main file level and every subcircuit) gets its own scene; rules
//! written above it are evaluated again in it (10.3).

pub mod attrs;
pub mod ctx;
pub mod cycles;
pub mod eval;
pub mod kw;
pub mod obj;
mod output;
pub mod phases;
pub mod scene;
pub mod unit;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use crate::diag::{DiagId, Sink};
use crate::host::{HostDocument, Level, LevelKind, Record};
use crate::model::{FailedSpan, HostRef, Model};
use crate::source::Provider;
use crate::syntax::RecordAst;

use eval::{Bound, Stop, R};
use obj::*;
use scene::Scene;
use unit::{Unit, UnitKind};

/// The memoised value of a member in a scene.
#[derive(Clone, Debug)]
pub enum Memo {
    Val(ObjId),
    LocalRef,
}

/// The members of a unit that are rules, in order, with the unit's numbers of members and rules
/// when they were taken.
pub(crate) type RuleMembers = ((usize, usize), Rc<Vec<unit::UnitMember>>);

#[derive(Clone, Debug)]
pub struct Options {
    /// Use the standard package implicitly at the root (16.3).
    pub std: bool,
    /// Read the records with `type="generated"`, the solver's results. A solver interprets a level
    /// without them (2.8, 14.8): every file of the document is read as if they were not written.
    pub generated: bool,
    /// Two coordinates closer than this are equal when constraints are checked against the
    /// written coordinates and conflicting coordinates are found (11.7). The standard leaves the
    /// tolerance to the tool; a viewer that does not solve uses [`crate::geom::TOL`].
    pub tolerance: f64,
}

impl Default for Options {
    fn default() -> Options {
        Options { std: true, generated: true, tolerance: crate::geom::TOL }
    }
}

/// The result of interpreting a document.
pub struct Interpretation {
    pub model: Model,
    pub sink: Sink,
    /// The name of the package checked on its own, when the document is a package file (2.7).
    pub checking_package: String,
    /// File texts by name: the host files and the package files read.
    pub sources: Vec<(String, String)>,
    /// The host document interpreted.
    pub doc: HostDocument,
    /// The references to host objects in the document's records, in the order they were resolved.
    pub refs: Vec<HostRef>,
    /// The statements, or parts of them, that do not take effect: failed or dropped in a conflict.
    pub failed: Vec<FailedSpan>,
}

impl Interpretation {
    /// The diagnostics a user sees (7.6).
    pub fn diagnostics(&self) -> Vec<&crate::diag::Diag> {
        self.sink.shown(&self.checking_package)
    }
}

pub struct Interp<'p> {
    pub doc: HostDocument,
    pub(crate) provider: &'p dyn Provider,
    pub sink: Sink,
    pub(crate) objs: Vec<Obj>,
    pub(crate) units: Vec<Unit>,
    pub(crate) level_unit: Vec<UnitId>,
    pub(crate) level_scene: Vec<Option<SceneId>>,
    pub(crate) unit_recs: Vec<Vec<RecId>>,
    pub(crate) recs: Vec<Record>,
    pub(crate) asts: Vec<Option<Rc<RecordAst>>>,
    /// The characters of each record's text, collected when an offset into it is first read.
    pub(crate) rec_chars: Vec<Option<Rc<[char]>>>,
    pub(crate) pkg_levels: Vec<Level>,
    pub(crate) pkg_sources: Vec<(String, String)>,
    pub(crate) packages: HashMap<(String, String), UnitId>,
    pub(crate) pkg_files: Option<Vec<(String, String, String)>>,
    pub(crate) memo: HashMap<(MemberKey, Option<SceneId>), Memo>,
    pub(crate) bound_memo: HashMap<(MemberKey, Option<SceneId>), Bound>,
    pub(crate) in_progress: HashSet<(MemberKey, Option<SceneId>)>,
    pub(crate) nested_memo: HashMap<(usize, usize, Option<SceneId>), R<ObjId>>,
    pub(crate) tm_memo: HashMap<(usize, ObjId), ObjId>,
    pub(crate) tm_in_progress: HashSet<(usize, ObjId)>,
    pub(crate) node_memo: HashMap<(&'static str, usize, usize, usize), ObjId>,
    pub(crate) rules: Rules,
    /// The members of each unit that are rules, as `rule_members` took them last.
    pub(crate) rule_members: HashMap<UnitId, RuleMembers>,
    pub(crate) block_symbols: HashMap<String, BTreeMap<MemberKey, (Pos, ObjId)>>,
    pub(crate) scenes: Vec<Scene>,
    pub(crate) symbol_order: HashMap<SceneId, Vec<ObjId>>,
    /// Where each set made of other sets is made (5.2.2).
    pub(crate) made_at: HashMap<ObjId, cycles::MadeAt>,
    /// Per scene (`None`: before any), the latest made source of the edits there that make a set
    /// depend on one made after it (5.2.2). Objects are numbered in the order they are made.
    pub(crate) later_sources: HashMap<Option<SceneId>, ObjId>,
    /// The statements that an earlier run found to close a cycle of sets, with their diagnostics:
    /// they fail from the start (5.2.2).
    pub(crate) closing: HashMap<cycles::StmtKey, String>,
    /// A statement found to close a cycle of sets after a statement placed before it was
    /// evaluated: the run stops, and the interpretation is run again with it in `closing`.
    pub(crate) restart: Option<(cycles::StmtKey, String)>,
    pub(crate) std_enabled: bool,
    /// Read the records with `type="generated"` (`Options::generated`).
    pub(crate) read_generated: bool,
    /// The tolerance of the checks against written coordinates (`Options::tolerance`, 11.7).
    pub(crate) tol: f64,
    pub(crate) std_cache: Option<Option<UnitId>>,
    pub(crate) scoped_memo: HashMap<(UnitId, ObjId), Vec<(String, String)>>,
    pub(crate) body_errors: HashMap<usize, DiagId>,
    pub(crate) made_routes: Vec<ObjId>,
    /// The package file checked on its own (2.7).
    pub(crate) package_main: Option<UnitId>,
    pub(crate) checking_path: Option<String>,
    /// The host references resolved so far, each once.
    pub(crate) refs: Vec<HostRef>,
    pub(crate) refs_seen: HashSet<HostRef>,
    /// The statements found not to take effect so far, each once.
    pub(crate) failed_spans: Vec<FailedSpan>,
    pub(crate) failed_seen: HashSet<FailedSpan>,
}

impl<'p> Interp<'p> {
    fn blank(doc: HostDocument, provider: &'p dyn Provider, options: &Options, sink: Sink) -> Interp<'p> {
        let nlevels = doc.levels.len();
        let mut it = Interp {
            doc,
            provider,
            sink,
            objs: Vec::new(),
            units: Vec::new(),
            level_unit: vec![0; nlevels],
            level_scene: vec![None; nlevels],
            unit_recs: Vec::new(),
            recs: Vec::new(),
            asts: Vec::new(),
            rec_chars: Vec::new(),
            pkg_levels: Vec::new(),
            pkg_sources: Vec::new(),
            packages: HashMap::new(),
            pkg_files: None,
            memo: HashMap::new(),
            bound_memo: HashMap::new(),
            in_progress: HashSet::new(),
            nested_memo: HashMap::new(),
            tm_memo: HashMap::new(),
            tm_in_progress: HashSet::new(),
            node_memo: HashMap::new(),
            rules: Rules::default(),
            rule_members: HashMap::new(),
            block_symbols: HashMap::new(),
            scenes: Vec::new(),
            symbol_order: HashMap::new(),
            made_at: HashMap::new(),
            later_sources: HashMap::new(),
            closing: HashMap::new(),
            restart: None,
            std_enabled: options.std,
            read_generated: options.generated,
            tol: options.tolerance,
            std_cache: None,
            scoped_memo: HashMap::new(),
            body_errors: HashMap::new(),
            made_routes: Vec::new(),
            refs: Vec::new(),
            refs_seen: HashSet::new(),
            failed_spans: Vec::new(),
            failed_seen: HashSet::new(),
            package_main: None,
            checking_path: None,
        };
        for _ in 0..3 {
            it.objs.push(Obj { data: ObjData::Marker, name: String::new() }); // LEVEL, OWNER, NOTHING
        }
        it
    }

    /// Prepare the interpretation of a host document: read every level's records and headers.
    /// `sink` may already hold the host adapter's diagnostics.
    pub fn new(doc: HostDocument, provider: &'p dyn Provider, options: &Options, sink: Sink) -> Interp<'p> {
        let mut it = Interp::blank(doc, provider, options, sink);
        for lv in 0..it.doc.levels.len() {
            let mut u = Unit::new(UnitKind::Level, &it.doc.levels[lv].title());
            u.level = Some(lv);
            it.level_unit[lv] = it.units.len();
            it.units.push(u);
            let recs = it.doc.levels[lv].records.clone();
            let ids: Vec<RecId> = recs.into_iter().map(|r| it.add_record(r)).collect();
            it.unit_recs.push(ids);
        }
        for lv in 0..it.doc.levels.len() {
            let u = it.level_unit[lv];
            it.read_unit(u);
        }
        it
    }

    /// Prepare the check of a package file on its own (2.7): the standard package is the tool's.
    pub fn new_package(path: &str, provider: &'p dyn Provider, options: &Options) -> Interp<'p> {
        let name = crate::host::file_name(path).to_string();
        let main = Level::new(LevelKind::File, &name, &name, path);
        let doc = HostDocument::new(
            main,
            crate::host::Dialect { name: "none".into(), case_sensitive: false, ground_aliases: Vec::new() },
        );
        let mut it = Interp::blank(doc, provider, options, Sink::new());
        let mut u = Unit::new(UnitKind::Level, &name);
        u.level = Some(0);
        u.ok = false;
        it.units.push(u);
        it.unit_recs.push(Vec::new());
        it.checking_path = Some(crate::source::normalize(path));
        let pu = it.load_package_file(path);
        it.package_main = Some(pu);
        it
    }

    pub fn run(&mut self) {
        if let Some(pm) = self.package_main {
            self.classify(pm);
            return;
        }
        let mut todo: Vec<UnitId> = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.kind == UnitKind::Package && !u.embedded)
            .map(|(i, _)| i)
            .collect();
        if let Some(s) = self.std_package() {
            todo.push(s);
        }
        for (i, u) in self.units.iter().enumerate() {
            if u.embedded {
                todo.push(i);
            }
        }
        for u in todo {
            self.classify(u);
        }
        for lv in 0..self.doc.levels.len() {
            if !self.doc.levels[lv].circuit && self.file_ok(lv) {
                let u = self.level_unit[lv];
                self.classify(u);
            }
        }
        self.check_scopes();
        self.check_shadowing();
        self.check_package_shadowing();
        self.prescan_blocks();
        for lv in 0..self.doc.levels.len() {
            if self.restart.is_some() {
                return;
            }
            if self.doc.levels[lv].circuit {
                self.build_scene(lv);
            }
        }
    }

    /// Run, and run again from the start as long as a run finds a statement that closes a cycle of
    /// sets after a statement placed before it was evaluated (5.2.2): each run has the statements
    /// found so far fail from the start. `again` makes a new interpreter from the one that stopped.
    fn run_settled(mut self, again: impl Fn(Interp<'p>) -> Interp<'p>) -> Interpretation {
        self.run();
        while let Some((key, msg)) = self.restart.take() {
            let mut closing = std::mem::take(&mut self.closing);
            closing.insert(key, msg);
            self = again(self);
            self.closing = closing;
            self.run();
        }
        self.finish()
    }

    /// Evaluate the statements of a unit without a circuit once: definitions, rules, errors.
    fn classify(&mut self, u: UnitId) {
        if self.units[u].classified || !self.units[u].ok {
            return;
        }
        self.units[u].classified = true;
        for m in self.units[u].members.clone() {
            let _ = self.eval_member_at(u, m.rec, m.k, m.embedded, &m.m, None, m.header.clone(), None, "");
        }
    }

    fn build_scene(&mut self, lv: usize) {
        let s = self.create_scene(lv);
        if !self.file_ok(lv) {
            self.run_scene(s);
            return;
        }
        let unit = self.level_unit[lv];
        struct Work {
            u: UnitId,
            m: unit::UnitMember,
            pos: Option<Pos>,
            scope_in: bool,
        }
        let mut work: Vec<Work> = Vec::new();
        for e in self.effective_packages(lv) {
            for m in self.rule_members(e.unit).iter() {
                work.push(Work { u: e.unit, m: m.clone(), pos: None, scope_in: false });
            }
        }
        let mut anc = self.doc.ancestors(lv);
        anc.reverse();
        for a in &anc {
            if !self.file_ok(*a) {
                continue;
            }
            let au = self.level_unit[*a];
            for m in self.rule_members(au).iter() {
                work.push(Work { u: au, m: m.clone(), pos: None, scope_in: false });
            }
        }
        for m in self.units[unit].members.clone() {
            work.push(Work { u: unit, m, pos: None, scope_in: false });
        }
        // records written above with scope= this subcircuit (2.6)
        let anc_up = self.doc.ancestors(lv);
        let lvl = &self.doc.levels[lv];
        let key = if lvl.kind == LevelKind::Subckt { Some(self.doc.key(&lvl.name)) } else { None };
        for (depth, a) in anc_up.iter().enumerate() {
            let au = self.level_unit[*a];
            let Some(ks) = key.as_ref().and_then(|k| self.units[au].scope_index.get(k)) else { continue };
            for i in ks.clone() {
                let (rec, ast, header) = self.units[au].scope_records[i].clone();
                for (k, m) in ast.body.as_ref().unwrap().members.iter().enumerate() {
                    if m.error.is_some() {
                        continue;
                    }
                    let p = pos(&[4, -((anc_up.len() - depth) as i64), rec as i64, k as i64]);
                    let um =
                        unit::UnitMember { rec: Some(rec), k, embedded: false, m: m.clone(), header: header.clone() };
                    work.push(Work { u: unit, m: um, pos: Some(p), scope_in: true });
                }
            }
        }
        for w in work {
            if self.restart.is_some() {
                return;
            }
            if w.scope_in {
                self.eval_scope_in(w.u, &w.m, w.pos.unwrap(), s);
                continue;
            }
            let r =
                self.eval_member_at(w.u, w.m.rec, w.m.k, w.m.embedded, &w.m.m, Some(s), w.m.header.clone(), None, "");
            // a selection whose scope is this level's host objects is just empty below (10.3);
            // the 10.5 warning is for the other roles of a generating AS or ATTACH
            if matches!(r, Err(Stop::LocalRef))
                && self.units[w.u].level != Some(lv)
                && !self.scoped_selection(&w.m.m, w.u)
            {
                let at = w.m.m.segments.first().map(|sg| sg.start).unwrap_or(w.m.m.start);
                let loc = self.rec_loc(w.m.rec, at);
                self.sink.warning(
                    "this rule uses objects of the level it is written in, so it takes effect only there (10.5)",
                    loc,
                );
            }
        }
        if self.restart.is_some() {
            return;
        }
        // a name bound twice in the same level (6.6)
        for b in self.units[unit].dups.clone() {
            let first = self.units[unit].names[&(b.prefix, b.name.clone())].clone();
            let k1 = first.key();
            let k2 = b.key();
            let v1 = match self.memo.get(&(k1.clone(), Some(s))) {
                Some(Memo::Val(v)) => Some(self.bound_value(&k1, Some(s), b.prefix, &b.name, *v)),
                _ => None,
            };
            let v2 = match self.memo.get(&(k2.clone(), Some(s))) {
                Some(Memo::Val(v)) => Some(self.bound_value(&k2, Some(s), b.prefix, &b.name, *v)),
                _ => None,
            };
            if let (Some(a), Some(c)) = (v1, v2) {
                if a != c {
                    let loc = self.rec_loc(b.rec, b.member.start);
                    self.sink.error(
                        format!(
                            "{}{} is already bound to another object in this level; the first binding is kept (6.6)",
                            b.prefix, b.name
                        ),
                        loc,
                    );
                }
            }
        }
        self.run_scene(s);
    }

    /// The members of a unit that are rules, in order. They are taken again only when the unit
    /// has new members or new rules, so that a level does not go through every member above it.
    fn rule_members(&mut self, u: UnitId) -> Rc<Vec<unit::UnitMember>> {
        let counts = (self.units[u].members.len(), self.rules.count(u));
        if let Some((c, ms)) = self.rule_members.get(&u) {
            if *c == counts {
                return ms.clone();
            }
        }
        let ms: Vec<unit::UnitMember> =
            self.units[u].members.iter().filter(|m| self.rules.contains(&m.key(u))).cloned().collect();
        let ms = Rc::new(ms);
        self.rule_members.insert(u, (counts, ms.clone()));
        ms
    }

    fn scoped_selection(&self, m: &crate::syntax::Member, u: UnitId) -> bool {
        let Some(seg) = m.segments.first() else { return false };
        let Some(call) = &seg.call else { return false };
        if !self.kw(&call.kw, u).starts_with("SELECT-") {
            return false;
        }
        let scope: Vec<&crate::syntax::Arg> =
            seg.args.iter().filter(|a| !matches!(&a.obj, crate::syntax::ObjExpr::Ref(r) if r.prefix == '$')).collect();
        !scope.is_empty() && scope.iter().all(|a| matches!(&a.obj, crate::syntax::ObjExpr::Ref(r) if (r.prefix == '#' || r.prefix == '&') && r.name.is_some()))
    }

    fn eval_scope_in(&mut self, u: UnitId, m: &unit::UnitMember, p: Pos, s: SceneId) {
        let key = (MemberKey::Scope { rec: m.rec.unwrap_or(0), k: m.k }, Some(s));
        if self.memo.contains_key(&key) {
            return;
        }
        let c = ctx::Ctx {
            scene: Some(s),
            unit: u,
            rec: m.rec,
            header: m.header.clone(),
            pos: p,
            frame: None,
            origin: Rc::from(""),
            nested: None,
            member_key: key.0.clone(),
            selected: false,
            defining: None,
            def_unit: None,
            scope_in: true,
        };
        if let (Ok(v), _) = self.eval_member(&m.m, &c) {
            self.memo.insert(key, Memo::Val(v));
        }
    }

    fn check_scopes(&mut self) {
        for u in 0..self.units.len() {
            let Some(lv) = self.units[u].level.filter(|_| self.units[u].kind == UnitKind::Level) else { continue };
            if self.units[u].scope_records.is_empty() {
                continue;
            }
            let below: HashSet<String> = self
                .doc
                .descendants(lv)
                .iter()
                .map(|d| &self.doc.levels[*d])
                .filter(|l| l.kind == LevelKind::Subckt)
                .map(|l| self.doc.key(&l.name))
                .collect();
            for (rec, _ast, header) in self.units[u].scope_records.clone() {
                let target = header.scope.clone().unwrap_or_default();
                if !below.contains(&self.doc.key(&target)) {
                    let loc = self.rec_loc(Some(rec), 0);
                    self.sink.error(format!("scope=\"{target}\" finds no subcircuit below this level: the statements of the record fail (2.6)"), loc);
                    let n = self.recs[rec].text.chars().count();
                    self.note_failed(Some(rec), 0, n, false);
                }
            }
        }
    }

    /// A level that binds a `$` name it inherits from a level above or from a package hides that
    /// definition in this level and below: a hint (6.8, 16.2).
    fn check_shadowing(&mut self) {
        for lv in 0..self.doc.levels.len() {
            let u = self.level_unit[lv];
            if !self.units[u].ok || !self.file_ok(lv) {
                continue;
            }
            for key in self.units[u].name_order.clone() {
                if key.0 != '$' {
                    continue;
                }
                let b = self.units[u].names[&key].clone();
                let Some(hidden) = self.lookup_unit('$', &key.1, u, true) else { continue };
                let owner = hidden.owner;
                let wher = if self.units[owner].kind == UnitKind::Package {
                    if Some(owner) == self.std_package() {
                        format!("in the standard package {}", self.units[owner].pkg_name)
                    } else {
                        format!("in the package {}", self.units[owner].pkg_name)
                    }
                } else {
                    let l = self.units[owner].level.map(|l| self.doc.levels[l].title()).unwrap_or_default();
                    format!("in the level above, {l}")
                };
                let name = format!("${}", key.1);
                let written = if b.written == name { b.written.clone() } else { format!("{} ({name})", b.written) };
                let loc = self.rec_loc(b.rec, b.off);
                self.sink.hint(format!("{written} hides the definition of the same name {wher}; this level and below use this one (6.8)"), loc);
            }
        }
    }

    /// Packages used in the same level that define the same `$X`: the later hides the earlier (16.2).
    fn check_package_shadowing(&mut self) {
        let mut seen: HashSet<(usize, String)> = HashSet::new();
        for lv in 0..self.doc.levels.len() {
            let u = self.level_unit[lv];
            if !self.units[u].ok || !self.file_ok(lv) {
                continue;
            }
            let pk = self.effective_packages(lv);
            let mut mine: Vec<&unit::EffPackage> =
                pk.iter().filter(|e| matches!(e.place, unit::Place::Level(l, _) if l == lv)).collect();
            mine.sort_by_key(|e| e.decl.as_ref().map(|d| d.pos).unwrap_or((0, 0)));
            let mut roots: Vec<&unit::EffPackage> =
                pk.iter().filter(|e| matches!(e.place, unit::Place::Root(_))).collect();
            roots.sort_by_key(|e| match e.place {
                unit::Place::Root(k) => k,
                _ => 0,
            });
            for (at, group) in [("at the root", roots), ("in the same level", mine)] {
                let mut before: HashMap<String, UnitId> = HashMap::new();
                for e in group {
                    let p = e.unit;
                    for key in self.units[p].name_order.clone() {
                        if key.0 != '$' {
                            continue;
                        }
                        let prev = before.insert(key.1.clone(), p);
                        let Some(prev) = prev else { continue };
                        let Some(d) = &e.decl else { continue };
                        let tag = (d.rec.unwrap_or(0) * 100_000 + d.pos.1, key.1.clone());
                        if !seen.insert(tag) {
                            continue;
                        }
                        let b = self.units[p].names[&key].clone();
                        let name = format!("${}", key.1);
                        let written =
                            if b.written == name { b.written.clone() } else { format!("{} ({name})", b.written) };
                        let what = if self.units[prev].pkg_name == "rnl-std" {
                            format!("the standard package {}", self.units[prev].pkg_name)
                        } else {
                            format!("the earlier USE-PACKAGE of {}", self.units[prev].pkg_name)
                        };
                        let loc = self.rec_loc(d.rec, d.offset);
                        let pn = self.units[p].pkg_name.clone();
                        self.sink.hint(format!("{written} in package {pn} hides the definition of the same name in {what} {at}; this level and below use the one of {pn} (16.2)"), loc);
                    }
                }
            }
        }
    }

    /// `# @SET-SYMBOL` in a subcircuit acts on the levels that instantiate it (9.2.6), which are built first.
    fn prescan_blocks(&mut self) {
        for lv in 0..self.doc.levels.len() {
            if self.doc.levels[lv].kind != LevelKind::Subckt || !self.doc.levels[lv].circuit || !self.file_ok(lv) {
                continue;
            }
            let u = self.level_unit[lv];
            for m in self.units[u].members.clone() {
                if m.m.error.is_some() || m.m.segments.len() != 1 {
                    continue;
                }
                let seg = &m.m.segments[0];
                let Some(call) = &seg.call else { continue };
                if self.kw(&call.kw, u) != "SET-SYMBOL" {
                    continue;
                }
                let bare = matches!(seg.args.first().map(|a| &a.obj), Some(crate::syntax::ObjExpr::Ref(r)) if r.is_bare_level());
                if !bare {
                    continue;
                }
                let _ = self.eval_member_at(u, m.rec, m.k, m.embedded, &m.m, None, m.header.clone(), None, "");
            }
        }
    }

    /// Build the output.
    pub fn finish(mut self) -> Interpretation {
        let model = self.build_model();
        let checking = self.package_main.map(|p| self.units[p].pkg_name.clone()).unwrap_or_default();
        let mut sources = self.doc.sources.clone();
        sources.extend(self.pkg_sources.clone());
        Interpretation {
            model,
            sink: std::mem::take(&mut self.sink),
            checking_package: checking,
            sources,
            refs: std::mem::take(&mut self.refs),
            failed: std::mem::take(&mut self.failed_spans),
            doc: self.doc,
        }
    }
}

/// Interpret a host document: the effective model and the diagnostics. `sink` holds what the
/// host adapter reported, if anything.
pub fn interpret(doc: HostDocument, provider: &dyn Provider, options: &Options, sink: Sink) -> Interpretation {
    let host = sink.clone();
    Interp::new(doc, provider, options, sink).run_settled(|it| Interp::new(it.doc, provider, options, host.clone()))
}

/// Check a package file on its own (2.7).
pub fn check_package(path: &str, provider: &dyn Provider, options: &Options) -> Interpretation {
    Interp::new_package(path, provider, options).run_settled(|_| Interp::new_package(path, provider, options))
}
