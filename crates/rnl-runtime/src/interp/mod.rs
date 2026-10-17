//! The interpreter (standard chapters 5–16): reads the records of a host document and its
//! packages, resolves names, evaluates every statement into objects and facts, and lets each
//! drawing level take effect phase by phase (8.2).
//!
//! Each drawing level (the main file level and every subcircuit) gets its own scene; rules
//! written above it are evaluated again in it (10.3).

pub mod attrs;
mod conflicts;
pub mod ctx;
pub mod cycles;
pub mod eval;
mod export;
pub mod kw;
mod numbers;
pub mod obj;
mod output;
mod packages;
pub mod phases;
pub mod scene;
mod selfcheck;
pub mod unit;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use crate::diag::{DiagId, Sink};
use crate::host::{HostDocument, Level, LevelId, LevelKind, Record};
use crate::model::{FailedSpan, HostRef, Model};
use crate::numbering::LevelNumbering;
use crate::previous::RecordResult;
use crate::source::Provider;
use crate::syntax::RecordAst;

use eval::{Bound, Stop, R};
use numbers::{AuthoredCount, Birth, Counted, LevelNumbers, NumberRun};
use obj::*;
use packages::{Eff, EffKey, Expansion};
use scene::Scene;
use unit::{Header, Place, Unit, UnitKind};

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
    /// Build the solver's view of each drawing level, [`Interpretation::problems`].
    pub problems: bool,
    /// Two coordinates closer than this are equal when constraints are checked against the
    /// written coordinates and conflicting coordinates are found (11.7). The standard leaves the
    /// tolerance to the tool; a viewer that does not solve uses [`crate::geom::TOL`].
    pub tolerance: f64,
}

impl Default for Options {
    fn default() -> Options {
        Options { std: true, generated: true, problems: false, tolerance: crate::geom::TOL }
    }
}

impl Options {
    /// What a solver reads (2.8, 14.8, 17): the levels without the records with
    /// `type="generated"`, and the problem of each.
    pub fn solver() -> Options {
        Options { generated: false, problems: true, ..Options::default() }
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
    /// What a solver needs of each drawing level, in the order of `model.levels`; empty unless
    /// [`Options::problems`] is set. Built without the generated records ([`Options::solver`]),
    /// it is the solver's input (2.8, 17).
    pub problems: Vec<crate::problem::LevelProblem>,
    /// The numbers of the objects without a name of each drawing level, with their checks, whatever
    /// its version (5.9); empty for a package file checked on its own. See [`crate::numbering`].
    pub numbering: Vec<LevelNumbering>,
    /// For each level of the host document, by its index in [`HostDocument::levels`]: `None`
    /// when its RNL is interpreted, otherwise the file level on the way up from it, itself
    /// included, that declares a language version this reading does not interpret, with that
    /// version. No RNL of that file is interpreted, nor of the levels in it, and the file is kept
    /// as it is when saved (16.1): a tool writes no record into such a level. Empty when a package
    /// file is checked on its own (2.7).
    pub unsupported: Vec<Option<(LevelId, String)>>,
    /// The records of the solver to read again, differently (5.9).
    pub(crate) recheck: NumberRun,
    /// In a reading without the records of the solver that counts for another, the objects of the
    /// authored records of each level (5.9).
    pub(crate) counted: HashMap<LevelId, AuthoredCount>,
    /// The records' own objects of the records of the solver, when a reading after this one needs
    /// them: a number was used before the member that makes its object, or this reading resolved
    /// such numbers with the counts of the one before (5.9).
    pub(crate) own: HashMap<RecId, Vec<Counted>>,
    /// A number was used before the member that makes its object, while that member was read.
    pub(crate) forward: bool,
    /// What the last record of the solver of each level gives, read with the records of the solver
    /// and with [`Options::problems`] (see [`crate::previous::previous`]).
    pub(crate) results: Vec<RecordResult>,
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
    /// The unit whose text each record is in.
    pub(crate) rec_unit: Vec<UnitId>,
    pub(crate) asts: Vec<Option<Rc<RecordAst>>>,
    /// The characters of each record's text, collected when an offset into it is first read.
    pub(crate) rec_chars: Vec<Option<Rc<[char]>>>,
    pub(crate) pkg_levels: Vec<Level>,
    pub(crate) pkg_sources: Vec<(String, String)>,
    pub(crate) packages: HashMap<(String, String), UnitId>,
    pub(crate) pkg_files: Option<Vec<(String, String, String)>>,
    /// The language version each package file of the package directories is written in (16.1).
    pub(crate) pkg_languages: HashMap<String, String>,
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
    /// Build the problems (`Options::problems`).
    pub(crate) want_problems: bool,
    /// The tolerance of the checks against written coordinates (`Options::tolerance`, 11.7).
    pub(crate) tol: f64,
    /// The standard package used implicitly for the files of each language version (16.3).
    pub(crate) std_units: HashMap<String, Option<UnitId>>,
    /// The packages in effect on each chain of levels, and the number of units they were found
    /// among (16.2).
    pub(crate) effs: HashMap<EffKey, Rc<Eff>>,
    pub(crate) effs_units: usize,
    /// What each package brings in when used (16.2).
    pub(crate) expansions: HashMap<UnitId, Expansion>,
    /// Where the names of a package are looked up after its own, on each chain (16.2).
    pub(crate) envs: HashMap<(EffKey, UnitId), Rc<Vec<UnitId>>>,
    /// The standard package in effect on each chain and what it references (16.2, 16.3).
    #[allow(clippy::type_complexity)]
    pub(crate) std_envs: HashMap<EffKey, Rc<(HashSet<String>, Vec<UnitId>)>>,
    /// The chain each package's statements are read on without a level (16.2).
    pub(crate) home: HashMap<UnitId, EffKey>,
    /// The packages whose references were checked for hiding (16.2).
    pub(crate) refs_checked: HashSet<UnitId>,
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
    /// The header of each record read, other than the header record (2.8).
    pub(crate) rec_header: HashMap<RecId, Rc<Header>>,
    /// The objects that may be numbered, as they were made, and each one's index there (5.9).
    pub(crate) births: Vec<Birth>,
    pub(crate) birth_of: HashMap<ObjId, usize>,
    /// The objects that the member making them binds to a name: not numbered (5.9).
    pub(crate) named: HashSet<ObjId>,
    /// The evaluations of members under way, innermost last, and the last one begun.
    pub(crate) evals: Vec<u64>,
    pub(crate) next_eval: u64,
    /// The numbered objects by (scene, record of the solver, before its member), with the number of
    /// births and of names they were computed with.
    #[allow(clippy::type_complexity)]
    pub(crate) numbered_memo: HashMap<(SceneId, Option<RecId>, Option<usize>), ((usize, usize), Rc<Vec<ObjId>>)>,
    /// The numbered objects of each scene, with the number of births and of names they were
    /// computed with.
    pub(crate) level_numbers_memo: HashMap<SceneId, ((usize, usize), Rc<LevelNumbers>)>,
    /// The births by the member of a record whose evaluation made them, in each scene.
    pub(crate) births_by_root: HashMap<(SceneId, MemberKey), Vec<usize>>,
    /// How many objects one token made for one member of a set so far, by scene, usage whose body
    /// it is in, record, token and member.
    #[allow(clippy::type_complexity)]
    pub(crate) each_seen: HashMap<(SceneId, Option<ObjId>, Option<RecId>, usize, ObjId), usize>,
    /// How each object was made, as another reading compares it (5.9).
    pub(crate) identities: HashMap<ObjId, Rc<str>>,
    /// A number was used before the member that makes its object, while that member was read.
    pub(crate) forward: bool,
    /// What this run takes from the readings before it (5.9).
    pub(crate) numbers: NumberRun,
    /// The conflicts found while the geometry of a level of the draft 0.3 is resolved, whose
    /// hints are given once it is (its 8.5).
    pub(crate) conflicts: conflicts::Conflicts,
}

impl<'p> Interp<'p> {
    fn blank(
        doc: HostDocument,
        provider: &'p dyn Provider,
        options: &Options,
        sink: Sink,
        numbers: NumberRun,
    ) -> Interp<'p> {
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
            rec_unit: Vec::new(),
            asts: Vec::new(),
            rec_chars: Vec::new(),
            pkg_levels: Vec::new(),
            pkg_sources: Vec::new(),
            packages: HashMap::new(),
            pkg_files: None,
            pkg_languages: HashMap::new(),
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
            want_problems: options.problems,
            tol: options.tolerance,
            std_units: HashMap::new(),
            effs: HashMap::new(),
            effs_units: 0,
            expansions: HashMap::new(),
            envs: HashMap::new(),
            std_envs: HashMap::new(),
            home: HashMap::new(),
            refs_checked: HashSet::new(),
            scoped_memo: HashMap::new(),
            body_errors: HashMap::new(),
            made_routes: Vec::new(),
            refs: Vec::new(),
            refs_seen: HashSet::new(),
            failed_spans: Vec::new(),
            failed_seen: HashSet::new(),
            package_main: None,
            checking_path: None,
            rec_header: HashMap::new(),
            births: Vec::new(),
            birth_of: HashMap::new(),
            named: HashSet::new(),
            evals: Vec::new(),
            next_eval: 0,
            numbered_memo: HashMap::new(),
            level_numbers_memo: HashMap::new(),
            births_by_root: HashMap::new(),
            each_seen: HashMap::new(),
            identities: HashMap::new(),
            forward: false,
            numbers,
            conflicts: conflicts::Conflicts::default(),
        };
        for _ in 0..3 {
            it.objs.push(Obj { data: ObjData::Marker, name: String::new() }); // LEVEL, OWNER, NOTHING
        }
        it
    }

    /// Prepare the interpretation of a host document: read every level's records and headers.
    /// `sink` may already hold the host adapter's diagnostics.
    pub fn new(doc: HostDocument, provider: &'p dyn Provider, options: &Options, sink: Sink) -> Interp<'p> {
        Interp::new_run(doc, provider, options, sink, NumberRun::default())
    }

    /// The same, with the records of the solver that this run reads differently (5.9).
    fn new_run(
        doc: HostDocument,
        provider: &'p dyn Provider,
        options: &Options,
        sink: Sink,
        numbers: NumberRun,
    ) -> Interp<'p> {
        let mut it = Interp::blank(doc, provider, options, sink, numbers);
        for lv in 0..it.doc.levels.len() {
            let mut u = Unit::new(UnitKind::Level, &it.doc.levels[lv].title());
            u.level = Some(lv);
            let unit = it.units.len();
            it.level_unit[lv] = unit;
            it.units.push(u);
            let recs = it.doc.levels[lv].records.clone();
            let ids: Vec<RecId> = recs.into_iter().map(|r| it.add_record(r, unit)).collect();
            it.unit_recs.push(ids);
        }
        for lv in 0..it.doc.levels.len() {
            let u = it.level_unit[lv];
            it.read_unit(u);
        }
        it.publish_scope_names();
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
        let mut it = Interp::blank(doc, provider, options, Sink::new(), NumberRun::default());
        let mut u = Unit::new(UnitKind::Level, &name);
        u.level = Some(0);
        u.ok = false;
        it.units.push(u);
        it.unit_recs.push(Vec::new());
        it.checking_path = Some(crate::source::normalize(path));
        let pu = it.load_package_file(path, None);
        it.package_main = Some(pu);
        it
    }

    pub fn run(&mut self) {
        // the standard package of each chain first, by the language version of its file: reading
        // one reads the packages it references (16.3)
        match self.package_main {
            Some(pm) => {
                self.implicit_std(EffKey::Alone(pm));
            }
            None => {
                for lv in 0..self.doc.levels.len() {
                    self.implicit_std(EffKey::Level(lv));
                }
            }
        }
        if let Some(pm) = self.package_main {
            // checked on its own (2.7): the package and what its references bring in (16.2)
            let chain = EffKey::Alone(pm);
            self.set_home(chain);
            let eff = self.eff(chain);
            let name = self.units[pm].pkg_name.clone();
            let brought: Vec<UnitId> =
                self.expansion(&name, Some(pm)).0.iter().filter_map(|n| eff.unit.get(n).copied().flatten()).collect();
            for &u in &brought {
                self.classify(u);
            }
            for &u in &brought {
                self.check_package_refs(u);
            }
            return;
        }
        // a package's statements read without a level take the packages in effect on the first
        // chain it is in effect on (16.2)
        for lv in 0..self.doc.levels.len() {
            if self.file_ok(lv) {
                self.set_home(EffKey::Level(lv));
            }
        }
        let packages: Vec<UnitId> = (0..self.units.len())
            .filter(|&i| self.units[i].kind == UnitKind::Package && !self.units[i].embedded)
            .collect();
        let embedded = (0..self.units.len()).filter(|&i| self.units[i].embedded);
        let todo: Vec<UnitId> = packages.iter().copied().chain(embedded).collect();
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
        for u in packages {
            self.check_package_refs(u);
        }
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
        }
        let mut work: Vec<Work> = Vec::new();
        let eff = self.effective_packages(lv);
        for e in &eff.entries {
            for m in self.rule_members(e.unit).iter() {
                work.push(Work { u: e.unit, m: m.clone() });
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
                work.push(Work { u: au, m: m.clone() });
            }
        }
        // its own text, then the records written above with scope= this subcircuit (2.6), whose
        // members are its own as well, evaluated in its unit
        let own = self.units[unit].members.iter().chain(&self.units[unit].scope_members);
        work.extend(own.map(|m| Work { u: unit, m: m.clone() }));
        for w in work {
            if self.restart.is_some() {
                return;
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

    /// The members of a unit that are rules, in order, those of the records written into it with
    /// scope= after its own. They are taken again only when the unit has new members or new rules,
    /// so that a level does not go through every member above it.
    fn rule_members(&mut self, u: UnitId) -> Rc<Vec<unit::UnitMember>> {
        let counts = (self.units[u].members.len(), self.rules.count(u));
        if let Some((c, ms)) = self.rule_members.get(&u) {
            if *c == counts {
                return ms.clone();
            }
        }
        // a subcircuit's own rules, then those of the records written into it with scope=, which
        // act as if written there: in the levels below it too, after its own text (2.6, 10.4)
        let unit = &self.units[u];
        let all = unit.members.iter().chain(&unit.scope_members);
        let ms: Vec<unit::UnitMember> = all.filter(|m| self.rules.contains(&m.key(u))).cloned().collect();
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
                let Some(hidden) = self.lookup_unit('$', &key.1, u, true, None) else { continue };
                let owner = hidden.owner;
                let wher = if self.units[owner].kind == UnitKind::Package {
                    // one of the packages of the standard package in effect (16.3)
                    let pn = self.units[owner].pkg_name.clone();
                    if !self.std_env(EffKey::Level(lv)).0.contains(&pn) {
                        format!("in the package {pn}")
                    } else if pn == packages::STD_NAME {
                        format!("in the standard package {pn}")
                    } else {
                        format!("in {pn} of the standard package {}", packages::STD_NAME)
                    }
                } else {
                    let l = self.units[owner].level.map(|l| self.doc.levels[l].title()).unwrap_or_default();
                    format!("in the level above, {l}")
                };
                let written = packages::shown(&b);
                let loc = self.rec_loc(b.rec, b.off);
                self.sink.hint(format!("{written} hides the definition of the same name {wher}; this level and below use this one (6.8)"), loc);
            }
        }
    }

    /// Packages used in the same level, or at the root, that define the same `$X`: the later
    /// hides the earlier (16.2). What one USE-PACKAGE brings in is the business of its package.
    fn check_package_shadowing(&mut self) {
        // each USE-PACKAGE, by its unit and place, and name: hinted once
        let mut seen: HashSet<(UnitId, (usize, usize), String)> = HashSet::new();
        for lv in 0..self.doc.levels.len() {
            let u = self.level_unit[lv];
            if !self.units[u].ok || !self.file_ok(lv) {
                continue;
            }
            let eff = self.effective_packages(lv);
            let std = self.std_env(EffKey::Level(lv));
            let mut mine: Vec<&unit::EffPackage> =
                eff.entries.iter().filter(|e| matches!(e.place, Place::Level(l, _, _) if l == lv)).collect();
            mine.sort_by_key(|e| match e.place {
                Place::Level(_, pos, j) => (pos, j),
                Place::Root(..) => ((0, 0), 0),
            });
            let mut roots: Vec<&unit::EffPackage> =
                eff.entries.iter().filter(|e| matches!(e.place, Place::Root(..))).collect();
            roots.sort_by_key(|e| match e.place {
                Place::Root(k, j) => (k, j),
                Place::Level(..) => (0, 0),
            });
            for (at, group) in [("at the root", roots), ("in the same level", mine)] {
                // each name: the package that defines it and the USE-PACKAGE that brings it in
                let mut before: HashMap<String, (UnitId, Option<&unit::UseDecl>)> = HashMap::new();
                for e in group {
                    let p = e.unit;
                    for key in self.units[p].name_order.clone() {
                        if key.0 != '$' {
                            continue;
                        }
                        let prev = before.insert(key.1.clone(), (p, e.decl.as_ref()));
                        let (Some((q, by)), Some(d)) = (prev, &e.decl) else { continue };
                        if by.is_some_and(|by| by.same(d)) {
                            continue;
                        }
                        if !seen.insert((d.unit, d.pos, key.1.clone())) {
                            continue;
                        }
                        let written = packages::shown(&self.units[p].names[&key]);
                        let (pn, qn) = (self.units[p].pkg_name.clone(), self.units[q].pkg_name.clone());
                        let what = match by {
                            _ if std.0.contains(&qn) => format!("the standard package {}", packages::STD_NAME),
                            Some(by) if Some(q) != by.pkg => {
                                format!("the package {qn} that the earlier USE-PACKAGE of {} brings in", by.name)
                            }
                            _ => format!("the earlier USE-PACKAGE of {qn}"),
                        };
                        let who = if Some(p) == d.pkg {
                            format!("package {pn}")
                        } else {
                            format!("package {pn}, which the USE-PACKAGE of {} brings in,", d.name)
                        };
                        let loc = self.rec_loc(d.rec, d.offset);
                        self.sink.hint(format!("{written} in {who} hides the definition of the same name in {what} {at}; this level and below use the one of {pn} (16.2)"), loc);
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
        let (problems, results) =
            if self.want_problems && self.package_main.is_none() { self.build_problems() } else { Default::default() };
        // the numbers and their checks (5.9); a reading that counts the objects of the authored
        // records for another gives those alone
        let counting = self.numbers.count;
        let counted: HashMap<LevelId, AuthoredCount> = if counting {
            (0..self.scenes.len()).map(|s| (self.scenes[s].level, self.counted_authored(s))).collect()
        } else {
            HashMap::new()
        };
        let numbering: Vec<LevelNumbering> = if !counting && self.package_main.is_none() {
            (0..self.scenes.len()).map(|s| self.level_numbering(s)).collect()
        } else {
            Vec::new()
        };
        let recheck = self.number_recheck(&numbering);
        let own = if self.forward || self.numbers.own.is_some() { self.counted_own() } else { HashMap::new() };
        let forward = self.forward;
        let checking = self.package_main.map(|p| self.units[p].pkg_name.clone()).unwrap_or_default();
        let mut sources = self.doc.sources.clone();
        sources.extend(self.pkg_sources.clone());
        let unsupported = if self.package_main.is_some() {
            Vec::new()
        } else {
            (0..self.doc.levels.len()).map(|lv| self.unsupported_file(lv)).collect()
        };
        Interpretation {
            model,
            sink: std::mem::take(&mut self.sink),
            checking_package: checking,
            sources,
            refs: std::mem::take(&mut self.refs),
            failed: std::mem::take(&mut self.failed_spans),
            problems,
            numbering,
            unsupported,
            recheck,
            counted,
            own,
            forward,
            results,
            doc: self.doc,
        }
    }

    /// Does this reading count the objects of the authored records for another (5.9)?
    fn counts_first(&self) -> bool {
        self.numbers.authored.is_none() && self.needs_count()
    }
}

/// Interpret a host document: the effective model and the diagnostics. `sink` holds what the
/// host adapter reported, if anything.
///
/// The numbers of 0.3 take more than one reading of the document (5.9), only when a file of 0.3 is
/// read with its records of the solver:
///
/// - the objects of the authored records are counted as the level reads without its records of
///   the solver, so that reading comes first (`NumberRun::authored`);
/// - a number used before the member that makes its object, while that member is being read, is
///   resolved by reading again with the record's own objects that the reading counted, until they
///   stay the same (`NumberRun::own`);
/// - a record of the solver whose numbers fail or are out of date is known only once every level is
///   read: the document is then read again, with that record's numbers errors (it has no
///   `numbering`) or its statements that use a number left out (its `numbering` does not match).
///   The checks given are those of the reading before, where every number is read: the text as
///   written.
pub fn interpret(doc: HostDocument, provider: &dyn Provider, options: &Options, sink: Sink) -> Interpretation {
    let host = sink.clone();
    let mut run = NumberRun::default();
    let mut out = read(doc, provider, options, sink, &host, &mut run);
    for _ in 0..3 {
        let same = run.own.as_ref().is_some_and(|own| **own == out.own);
        if !out.forward && (run.own.is_none() || same) {
            break;
        }
        run.own = Some(Rc::new(std::mem::take(&mut out.own)));
        out = read(out.doc, provider, options, host.clone(), &host, &mut run);
    }
    if !out.recheck.is_empty() {
        let recheck = std::mem::take(&mut out.recheck);
        let numbering = std::mem::take(&mut out.numbering);
        let mut again = NumberRun { refuse: recheck.refuse, stale: recheck.stale, ..run };
        out = read(out.doc, provider, options, host.clone(), &host, &mut again);
        out.numbering = numbering;
        out.recheck = NumberRun::default();
    }
    out
}

/// One reading of a document; first the reading without the records of the solver that counts the
/// objects of the authored records, when this one needs it and `run` does not have it yet (5.9).
fn read(
    doc: HostDocument,
    provider: &dyn Provider,
    options: &Options,
    sink: Sink,
    host: &Sink,
    run: &mut NumberRun,
) -> Interpretation {
    let mut it = Interp::new_run(doc, provider, options, sink, run.clone());
    if it.counts_first() {
        let without = Options { generated: false, problems: false, ..options.clone() };
        let counting = NumberRun { count: true, ..NumberRun::default() };
        let first = Interp::new_run(it.doc.clone(), provider, &without, host.clone(), counting.clone())
            .run_settled(|x| Interp::new_run(x.doc, provider, &without, host.clone(), counting.clone()));
        run.authored = Some(Rc::new(first.counted));
        it.numbers.authored = run.authored.clone();
    }
    let again = run.clone();
    it.run_settled(|x| Interp::new_run(x.doc, provider, options, host.clone(), again.clone()))
}

/// Check a package file on its own (2.7).
pub fn check_package(path: &str, provider: &dyn Provider, options: &Options) -> Interpretation {
    Interp::new_package(path, provider, options).run_settled(|_| Interp::new_package(path, provider, options))
}
