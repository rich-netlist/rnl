//! The interpreter (standard chapters 5–16): reads the records of a host document and its
//! packages, resolves names, evaluates every statement into objects and facts, and lets each
//! drawing level take effect phase by phase (8.2).
//!
//! Each drawing level (the main file level and every subcircuit) gets its own scene; rules
//! written above it are evaluated again in it (10.3).

mod annotation_template;
mod assertions;
pub mod attrs;
mod auto_geometry;
mod bindings;
mod body_rules;
pub mod catalog;
mod conflicts;
mod constraints;
mod conversions;
pub mod ctx;
mod currents;
pub mod cycles;
mod definition_calls;
mod definition_failures;
mod definition_scenes;
mod devices;
pub mod eval;
mod export;
mod groups;
mod host_diagnostics;
mod islands;
mod junction;
pub mod kw;
mod languages;
mod layer_kinds;
mod modifier_handles;
mod numbers;
pub mod obj;
mod output;
mod packages;
mod pin_names;
mod places;
mod position_refs;
mod reach;
mod routes;
mod scene_pins;
mod selections;
mod solver_config;
mod specific;
mod table;
/// The name of the standard package that the setting of 16.3 brings in at the root.
pub use packages::{standard_package_name, STANDARD_PACKAGE};
pub mod phases;
pub mod scene;
mod selfcheck;
mod standard;
mod symbol_pins;
mod terminal_maps;
mod topology;
pub mod unit;
mod usage_points;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use crate::diag::{codes, DiagId, Sink};
use crate::host::{HostDocument, Level, LevelId, LevelKind, Record};
use crate::model::{FailedSpan, HostRef, Model};
use crate::numbering::LevelNumbering;
use crate::previous::RecordResult;
use crate::source::Provider;
use crate::syntax::RecordAst;

use eval::{Bound, CachedMember, Stop};
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
    OwnLevel,
    Language,
}

/// The members of a unit that are rules, in order, with the unit's numbers of members and rules
/// when they were taken.
pub(crate) type RuleMembers = ((usize, usize), Rc<Vec<unit::UnitMember>>);

#[derive(Clone, Debug)]
pub struct Options {
    /// Explicit tool settings for core-owned policies such as junction writeback. They do not
    /// replace the file settings in the solver problem's input identity.
    pub solver_config: Vec<crate::problem::SolverSetting>,
    /// Allow the standard package that `include_std` adds (16.3): false disables `include_std`;
    /// explicit USE-PACKAGE statements always keep their meaning.
    pub std: bool,
    /// Add the standard package to the main file's header when it has no USE-PACKAGE of that
    /// name. A writer must persist that reference (16.3).
    pub include_std: bool,
    /// The version of the standard package that `include_std` adds, instead of the newest of the
    /// series written in the file's language version (16.3). A version pinned in a header record
    /// still comes first. None: by the language version. A package of another language version
    /// is not used.
    pub std_version: Option<String>,
    /// Read the records with `type="generated"`, the solver's results. A solver interprets a level
    /// without them (2.8, 14.8): every file of the document is read as if they were not written.
    pub generated: bool,
    /// Build the solver's view of each drawing level, [`Interpretation::problems`].
    pub problems: bool,
    /// Two coordinates closer than this are equal when constraints are checked against the
    /// written coordinates and conflicting coordinates are found (11.7). The standard leaves the
    /// tolerance to the tool; a viewer that does not solve uses [`crate::geom::TOL`].
    pub tolerance: f64,
    /// Build only the levels written in this file, by its name in the document
    /// ([`crate::host::Level::file`]): the subcircuits it defines and, for the main file, the main
    /// file level. Everything else is read as for the whole document, the files above it, the
    /// packages and the block symbols of the subcircuits it instantiates, so each level it builds
    /// is the one the whole document builds (6.1, 10.3). None, as by default: every level.
    pub only_file: Option<String>,
    /// With [`Options::only_file`], build no level at all, and give the symbols the subcircuits of
    /// the file give themselves (9.2.6), [`Interpretation::block_symbols`]: what the levels that
    /// instantiate them use of them. Off by default.
    pub symbols_only: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            solver_config: Vec::new(),
            std: true,
            include_std: false,
            std_version: None,
            generated: true,
            problems: false,
            tolerance: crate::geom::TOL,
            only_file: None,
            symbols_only: false,
        }
    }
}

impl Options {
    /// What a solver reads (2.8, 14.8, 17): the levels without the records with
    /// `type="generated"`, and the problem of each.
    pub fn solver() -> Options {
        Options { generated: false, problems: true, ..Options::default() }
    }

    /// What of these options a reading takes, [`ReadOptions`]: all the interpreter is made from.
    pub fn read_options(&self) -> ReadOptions {
        ReadOptions {
            std: self.std,
            include_std: self.include_std,
            std_version: self.std_version.clone(),
            generated: self.generated,
            problems: self.problems,
            tolerance: self.tolerance,
            only_file: self.only_file.clone(),
            symbols_only: self.symbols_only,
            junction: self.solver_config.iter().filter(|s| s.key == "junction").cloned().collect(),
        }
    }

    /// Whether a document read with these options and with `other` gives the same
    /// [`Interpretation`]: whether what a reading takes of them is the same ([`ReadOptions`]).
    pub fn reads_as(&self, other: &Options) -> bool {
        self.read_options() == other.read_options()
    }
}

/// What of [`Options`] a reading of a document takes, and the only options the interpreter is made
/// from, so that an option reaches a reading only through it: two options read a document alike
/// when what they give of it is equal ([`Options::reads_as`]). Each field is that of [`Options`],
/// but `junction`.
#[derive(Clone, Debug, PartialEq)]
pub struct ReadOptions {
    pub std: bool,
    pub include_std: bool,
    pub std_version: Option<String>,
    pub generated: bool,
    pub problems: bool,
    pub tolerance: f64,
    pub only_file: Option<String>,
    pub symbols_only: bool,
    /// The tool's settings for the solver ([`Options::solver_config`]) that a reading takes: those of
    /// `junction` (13.2). The others are the solver's alone.
    pub junction: Vec<crate::problem::SolverSetting>,
}

/// An alias that the header record of a file imports from an alias package with @USE-ALIASES
/// (16.5), as [`Interpretation::imported_aliases`] lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedAlias {
    /// The file level whose header record imports it.
    pub level: LevelId,
    /// The alias: a keyword with `@`, a `$` name with `$`, another word bare.
    pub alias: String,
    /// Its original, written the same way.
    pub original: String,
    /// The name of the alias package.
    pub package: String,
    /// Where the @USE-ALIASES is.
    pub loc: crate::diag::Loc,
}

/// The result of interpreting a document.
pub struct Interpretation {
    pub model: Model,
    pub sink: Sink,
    /// The standard-package version added by the `include_std` setting. When saving
    /// the main file, a writer persists this reference in its unqualified header (16.3).
    pub included_standard: Option<String>,
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
    /// How many of the first diagnostics of `sink` come from reading the document: the host
    /// readers', those of the language versions of the files, and those of reading their records,
    /// before any level is built. A reading of one file ([`Options::only_file`]) reads the whole
    /// document as well, and gives them too.
    pub read: usize,
    /// With [`Options::only_file`], the symbols subcircuits give themselves (9.2.6), by level, in
    /// the order of the document, None for one that gives itself none: those of the file with
    /// [`Options::symbols_only`], else those of every file that defines a subcircuit the file's
    /// levels instantiate. Empty otherwise.
    pub block_symbols: Vec<(LevelId, Option<crate::model::BlockSymbol>)>,
    /// The package files read, each once, by their paths as the provider names them (normalized),
    /// with their texts, in the order read.
    pub package_files: Vec<(String, String)>,
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
    /// The aliases that the files import from alias packages and use, in the order the files are
    /// read (16.5).
    pub imported_aliases: Vec<ImportedAlias>,
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
    /// The objects of the authored records that a reading of [`interpret_taking`] gave.
    pub(crate) given: AuthoredCounts,
}

impl Interpretation {
    /// The diagnostics a user sees (7.6).
    pub fn diagnostics(&self) -> Vec<&crate::diag::Diag> {
        self.sink.shown(&self.checking_package)
    }

    /// The objects of the authored records that this reading gave, made by [`interpret_taking`]: for
    /// a reading after it of the same document, such as with the records of the solver, or with
    /// another record of the solver written into a level, to take (5.9). None for a reading of
    /// [`interpret`].
    pub fn authored_counts(&self) -> AuthoredCounts {
        self.given.clone()
    }
}

pub struct Interp<'p> {
    pub doc: HostDocument,
    pub(crate) provider: &'p dyn Provider,
    pub sink: Sink,
    pub(crate) objs: Vec<Obj>,
    /// Late failures retain object data needed by earlier phases (7.2, 8.2).
    pub(crate) inactive_objects: HashSet<(SceneId, ObjId)>,
    pub(crate) late_failures: HashMap<ObjId, DiagId>,
    /// Per-input views for typed roles; original objects and written set handles are unchanged.
    pub(crate) role_inputs: HashMap<ObjId, conversions::RoleInput>,
    /// How many times something that the resolution of a named sub-object reads has changed:
    /// a symbol set or taken back, a text with a key made, a role's input resolved
    /// (`Scene::subobjects_resolved_at`).
    pub(crate) subobject_inputs: u64,
    pub(crate) units: Vec<Unit>,
    pub(crate) level_unit: Vec<UnitId>,
    pub(crate) level_scene: Vec<Option<SceneId>>,
    pub(crate) unit_recs: Vec<Vec<RecId>>,
    pub(crate) recs: Vec<Record>,
    /// The unit whose text each record is in.
    pub(crate) rec_unit: Vec<UnitId>,
    pub(crate) asts: Vec<Option<Rc<RecordAst>>>,
    /// The parses this reading took from those kept on the thread ([`unit::parsed`]): a reading
    /// takes each at most once, since it tells statements apart by where their parse is.
    pub(crate) kept_parses: HashSet<*const RecordAst>,
    /// The characters of each record's text, collected when an offset into it is first read.
    pub(crate) rec_chars: Vec<Option<Rc<[char]>>>,
    pub(crate) pkg_levels: Vec<Level>,
    pub(crate) pkg_sources: Vec<(String, String)>,
    /// The package files read, by their paths as the provider names them, with their texts.
    pub(crate) package_files: Vec<(String, String)>,
    /// The packages read, by name and version, the version in its numeric form
    /// ([`unit::package_cache_key`], 16.2).
    pub(crate) packages: HashMap<(String, String), UnitId>,
    pub(crate) pkg_files: HashMap<String, Vec<(String, String, String)>>,
    pub(crate) pkg_headers: HashMap<String, Option<(String, String)>>,
    /// The package files named by path= (16.2), by the path read: None when it cannot be read,
    /// else the PACKAGE its header declares, by name and version, if any. The provider gives the
    /// same text for a path throughout a reading, which names the same file from many headers.
    pub(crate) path_headers: HashMap<String, Option<Option<(String, String)>>>,
    /// The language version each package file of the package directories is written in (16.1).
    pub(crate) pkg_languages: HashMap<String, String>,
    pub(crate) memo: HashMap<(MemberKey, Option<SceneId>), Memo>,
    bindings: bindings::Bindings,
    pub(crate) bound_memo: HashMap<(MemberKey, Option<SceneId>), Bound>,
    pub(crate) in_progress: HashSet<(MemberKey, Option<SceneId>)>,
    pub(crate) nested_memo: HashMap<(usize, usize, Option<SceneId>), CachedMember>,
    pub(crate) tm_memo: HashMap<(usize, ObjId, Option<SceneId>), ObjId>,
    pub(crate) tm_in_progress: HashSet<(usize, ObjId, Option<SceneId>)>,
    pub(crate) node_memo: HashMap<(&'static str, usize, usize, usize), ObjId>,
    specific: specific::SpecificDefinitions,
    pub(crate) rules: Rules,
    body_rules: body_rules::BodyRules,
    /// The members of each unit that are rules, as `rule_members` took them last.
    pub(crate) rule_members: HashMap<UnitId, RuleMembers>,
    pub(crate) block_symbols: HashMap<String, BTreeMap<MemberKey, scene::BlockSymbol>>,
    pub(crate) scenes: Vec<Scene>,
    pub(crate) symbol_order: HashMap<SceneId, Vec<ObjId>>,
    /// Pin views for case-insensitive hosts, shared by all owners of the definition.
    pub(crate) folded_symbols: HashMap<ObjId, ObjId>,
    /// Effective terminal bindings of an assignment; definitions stay reusable.
    pub(crate) mapped_symbols: HashMap<(ObjId, Vec<Vec<obj::TermKey>>), ObjId>,
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
    /// Dependencies of member-place references on edits, including edits written later (5.2.3).
    places: places::Places,
    pending_constraints: HashMap<ObjId, constraints::Pending>,
    /// A constraint split off another one when a usage among its items stands for its members,
    /// with the one it was split from, whose statement and PRIORITY it shares (14.1).
    pub(crate) split_from: HashMap<ObjId, ObjId>,
    pub(crate) std_enabled: bool,
    pub(crate) include_std: bool,
    standard_hints: standard::StandardHints,
    /// [`Options::std_version`].
    pub(crate) std_version: Option<String>,
    /// Read the records with `type="generated"` (`Options::generated`).
    pub(crate) read_generated: bool,
    /// Build the problems (`Options::problems`).
    pub(crate) want_problems: bool,
    /// How many diagnostics reading the document gave ([`Interpretation::read`]).
    pub(crate) read: usize,
    /// With [`Options::only_file`], the levels built: those of the file and the circuits above them,
    /// whose scenes find the rules that reach below (10.1); and the levels given, the file's.
    pub(crate) only_levels: Option<OnlyLevels>,
    /// [`Options::symbols_only`].
    symbols_only: bool,
    /// The tolerance of the checks against written coordinates (`Options::tolerance`, 11.7).
    pub(crate) tol: f64,
    tool_junction: Vec<crate::problem::SolverSetting>,
    /// The standard package that `include_std` adds for the files of each language version (16.3).
    pub(crate) std_units: HashMap<String, Option<UnitId>>,
    /// The version of the standard package that the reading asks for ([`Options::std_version`]),
    /// when it cannot be found, and whether a chain that would use it has said so (16.3).
    pub(crate) missing_std: Option<(String, bool)>,
    pub(crate) std_alias_reported: HashSet<UnitId>,
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
    definition_scenes: definition_scenes::DefinitionScenes,
    /// Fixed nested bodies already checked at their source site, before any outer application.
    static_nested_definitions: HashMap<(UnitId, usize), (ObjId, bool)>,
    pub(crate) body_errors: HashMap<usize, DiagId>,
    pub(crate) body_member_errors: HashMap<usize, DiagId>,
    pub(crate) body_parameter_errors: HashMap<(ObjId, String), DiagId>,
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
    /// Qualified package records whose checks need the final effective package graph (2.8).
    pub(crate) pending_language_records: Vec<RecId>,
    pub(crate) host_languages: languages::HostLanguages,
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
    /// The conflicts found while the geometry of a level is resolved, whose hints are given once it
    /// is (8.5).
    pub(crate) conflicts: conflicts::Conflicts,
}

/// The levels a reading of one file builds and gives ([`Options::only_file`]).
#[derive(Clone, Debug)]
pub(crate) struct OnlyLevels {
    /// The levels of the file and the circuits above them.
    pub(crate) built: HashSet<LevelId>,
    /// The levels of the file.
    pub(crate) given: HashSet<LevelId>,
    /// The subcircuits whose records are read ([`Interp::reads`]): those built, every subcircuit
    /// of a file that defines a subcircuit they instantiate, and the levels above those.
    pub(crate) read: HashSet<LevelId>,
}

impl OnlyLevels {
    /// The levels of `file`, built with the circuits above them, or none built when `symbols_only`.
    fn new(doc: &HostDocument, file: &str, symbols_only: bool) -> OnlyLevels {
        let given: HashSet<LevelId> = (0..doc.levels.len()).filter(|&lv| doc.levels[lv].file == file).collect();
        let mut built = HashSet::new();
        if !symbols_only {
            built.clone_from(&given);
            for &lv in &given {
                built.extend(doc.ancestors(lv).into_iter().filter(|&a| doc.levels[a].circuit));
            }
        }
        // the records of the levels built, and of the levels above the file's; and, for the
        // symbols the levels built draw their instances with (9.2.6), of every subcircuit of the
        // files that define the subcircuits they instantiate, with the levels above them
        let mut read: HashSet<LevelId> = built.iter().chain(&given).copied().collect();
        for &lv in &given {
            read.extend(doc.ancestors(lv));
        }
        let mut files: HashSet<&str> = HashSet::new();
        for &lv in &built {
            let called = doc.levels[lv].instances.iter().filter(|i| i.is_subckt).filter_map(|i| doc.subckt_of(i));
            files.extend(called.map(|c| doc.levels[c].file.as_str()));
        }
        for lv in 0..doc.levels.len() {
            if doc.levels[lv].kind == LevelKind::Subckt && files.contains(doc.levels[lv].file.as_str()) {
                read.insert(lv);
                read.extend(doc.ancestors(lv));
            }
        }
        OnlyLevels { built, given, read }
    }

    /// The subcircuits whose block symbols a reading of one file gives
    /// ([`Interpretation::block_symbols`]), in the order of the document.
    fn symbol_levels(&self, doc: &HostDocument, symbols_only: bool) -> Vec<LevelId> {
        let files: HashSet<&str> = if symbols_only {
            self.given.iter().map(|&lv| doc.levels[lv].file.as_str()).collect()
        } else {
            let called = self.given.iter().flat_map(|&lv| doc.levels[lv].instances.iter());
            called
                .filter(|i| i.is_subckt)
                .filter_map(|i| doc.subckt_of(i))
                .map(|c| doc.levels[c].file.as_str())
                .collect()
        };
        let subckt = |lv: LevelId| doc.levels[lv].kind == LevelKind::Subckt;
        (0..doc.levels.len()).filter(|&lv| subckt(lv) && files.contains(doc.levels[lv].file.as_str())).collect()
    }
}

impl<'p> Interp<'p> {
    fn blank(
        doc: HostDocument,
        provider: &'p dyn Provider,
        options: &ReadOptions,
        sink: Sink,
        numbers: NumberRun,
    ) -> Interp<'p> {
        let nlevels = doc.levels.len();
        let only_levels = options.only_file.as_ref().map(|f| OnlyLevels::new(&doc, f, options.symbols_only));
        let mut it = Interp {
            doc,
            provider,
            sink,
            objs: Vec::new(),
            late_failures: HashMap::new(),
            inactive_objects: HashSet::new(),
            role_inputs: HashMap::new(),
            subobject_inputs: 0,
            units: Vec::new(),
            level_unit: vec![0; nlevels],
            level_scene: vec![None; nlevels],
            unit_recs: Vec::new(),
            recs: Vec::new(),
            rec_unit: Vec::new(),
            asts: Vec::new(),
            kept_parses: HashSet::new(),
            rec_chars: Vec::new(),
            pkg_levels: Vec::new(),
            pkg_sources: Vec::new(),
            package_files: Vec::new(),
            packages: HashMap::new(),
            pkg_files: HashMap::new(),
            pkg_headers: HashMap::new(),
            path_headers: HashMap::new(),
            pkg_languages: HashMap::new(),
            memo: HashMap::new(),
            bindings: Default::default(),
            bound_memo: HashMap::new(),
            in_progress: HashSet::new(),
            nested_memo: HashMap::new(),
            tm_memo: HashMap::new(),
            tm_in_progress: HashSet::new(),
            node_memo: HashMap::new(),
            specific: Default::default(),
            rules: Rules::default(),
            body_rules: body_rules::BodyRules::default(),
            rule_members: HashMap::new(),
            block_symbols: HashMap::new(),
            scenes: Vec::new(),
            symbol_order: HashMap::new(),
            folded_symbols: HashMap::new(),
            mapped_symbols: HashMap::new(),
            made_at: HashMap::new(),
            later_sources: HashMap::new(),
            closing: HashMap::new(),
            restart: None,
            places: places::Places::default(),
            pending_constraints: HashMap::new(),
            split_from: HashMap::new(),
            std_enabled: options.std,
            include_std: options.include_std,
            standard_hints: standard::StandardHints::default(),
            std_version: options.std_version.clone(),
            read_generated: options.generated,
            want_problems: options.problems,
            read: 0,
            only_levels,
            symbols_only: options.symbols_only,
            tol: options.tolerance,
            tool_junction: options.junction.clone(),
            std_units: HashMap::new(),
            missing_std: None,
            std_alias_reported: HashSet::new(),
            effs: HashMap::new(),
            effs_units: 0,
            expansions: HashMap::new(),
            envs: HashMap::new(),
            std_envs: HashMap::new(),
            home: HashMap::new(),
            refs_checked: HashSet::new(),
            scoped_memo: HashMap::new(),
            definition_scenes: Default::default(),
            static_nested_definitions: HashMap::new(),
            body_errors: HashMap::new(),
            body_member_errors: HashMap::new(),
            body_parameter_errors: HashMap::new(),
            made_routes: Vec::new(),
            refs: Vec::new(),
            refs_seen: HashSet::new(),
            failed_spans: Vec::new(),
            failed_seen: HashSet::new(),
            package_main: None,
            checking_path: None,
            rec_header: HashMap::new(),
            pending_language_records: Vec::new(),
            host_languages: languages::HostLanguages::default(),
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
        Interp::new_run(doc, provider, &options.read_options(), sink, NumberRun::default())
    }

    /// Prepare each host reading for the same file-version lookup.
    fn init_host_units(&mut self) {
        for lv in 0..self.doc.levels.len() {
            let mut u = Unit::new(UnitKind::Level, &self.doc.levels[lv].title());
            u.level = Some(lv);
            let unit = self.units.len();
            self.level_unit[lv] = unit;
            self.units.push(u);
            let recs = self.doc.levels[lv].records.clone();
            let ids: Vec<RecId> = recs.into_iter().map(|r| self.add_record(r, unit)).collect();
            self.unit_recs.push(ids);
        }
    }

    /// The same, with the records of the solver that this run reads differently (5.9).
    fn new_run(
        doc: HostDocument,
        provider: &'p dyn Provider,
        options: &ReadOptions,
        sink: Sink,
        numbers: NumberRun,
    ) -> Interp<'p> {
        let mut it = Interp::blank(doc, provider, options, sink, numbers);
        it.init_host_units();
        // Versions precede every other header declaration, including packages and aliases.
        it.read_file_versions();
        it.check_host_diagnostics();
        it.check_host_input_diagnostics();
        for lv in 0..it.doc.levels.len() {
            if it.reads(lv) {
                let u = it.level_unit[lv];
                it.read_unit(u);
            }
        }
        it.publish_scope_names();
        it
    }

    /// Prepare the check of a package file on its own (2.7): the standard package is the tool's.
    pub fn new_package(path: &str, provider: &'p dyn Provider, options: &Options) -> Interp<'p> {
        let name = crate::host::file_name(path).to_string();
        let main = Level::new(LevelKind::File, &name, &name, path);
        let doc = HostDocument::new(main, crate::host::format::Format::dialect(&crate::host::package::Package));
        let mut it = Interp::blank(doc, provider, &options.read_options(), Sink::new(), NumberRun::default());
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
        self.read = self.sink.all().len();
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
            self.check_language_records();
            self.check_free_words();
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
            self.check_unused_scoped_aliases();
            return;
        }
        // a package's statements read without a level take the packages in effect on the first
        // chain it is in effect on (16.2)
        for lv in 0..self.doc.levels.len() {
            if self.file_ok(lv) {
                self.set_home(EffKey::Level(lv));
            }
        }
        self.check_language_records();
        self.check_free_words();
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
            if self.doc.levels[lv].circuit && self.builds(lv) {
                self.build_scene(lv);
            }
        }
        self.finish_specific_definitions();
        self.check_unused_scoped_aliases();
    }

    /// Run, and run again from the start as long as a run finds a statement that closes a cycle of
    /// sets after a statement placed before it was evaluated (5.2.2): each run has the statements
    /// found so far fail from the start. `again` makes a new interpreter from the one that stopped.
    fn run_settled(self, again: impl Fn(Interp<'p>) -> Interp<'p>) -> Interpretation {
        let it = self.settled(&again);
        #[cfg(debug_assertions)]
        if it.places.idle_changed {
            return it.finish_checked(&again);
        }
        it.finish()
    }

    /// Debug builds only: the output of a last run whose plans changed only in ways that read
    /// nothing differently (`Places::idle_changed`), checked against the run again with those
    /// plans that such a change made before: both must give the same drawing, diagnostics, failed
    /// statements, problems and numbers.
    #[cfg(debug_assertions)]
    fn finish_checked(mut self, again: &impl Fn(Interp<'p>) -> Interp<'p>) -> Interpretation {
        let (doc, provider) = (self.doc.clone(), self.provider);
        let blank = || {
            Interp::blank(doc.clone(), provider, &Options::default().read_options(), Sink::new(), NumberRun::default())
        };
        let mut held = blank();
        Self::hand_over(&mut self, &mut held);
        let first = self.finish();
        // `again` makes the next run from the document of the interpreter it is given
        let mut next = again(blank());
        Self::hand_over(&mut held, &mut next);
        let second = next.settled(again).finish();
        debug_assert_eq!(
            format!("{:?}", first.model),
            format!("{:?}", second.model),
            "a run whose plans changed only in ways that read nothing differently draws otherwise when run again"
        );
        debug_assert_eq!(first.sink.all(), second.sink.all());
        debug_assert_eq!(first.failed, second.failed);
        debug_assert_eq!(first.problems, second.problems);
        debug_assert_eq!(first.numbering, second.numbering);
        first
    }

    /// Give `to` what the run of `from` found that a next run must know: the statements that close
    /// a cycle of sets, and the plans of the places, the bindings and the definition scenes.
    #[cfg(debug_assertions)]
    fn hand_over(from: &mut Interp<'p>, to: &mut Interp<'p>) {
        to.closing = std::mem::take(&mut from.closing);
        to.places.plans = std::mem::take(&mut from.places.plans);
        to.bindings.plans = std::mem::take(&mut from.bindings.plans);
        to.bindings.specific_names = std::mem::take(&mut from.bindings.specific_names);
        to.definition_scenes.plans = std::mem::take(&mut from.definition_scenes.plans);
    }

    /// [`Self::run_settled`] without the output: the interpreter of the last run.
    fn settled(mut self, again: impl Fn(Interp<'p>) -> Interp<'p>) -> Interp<'p> {
        loop {
            self.run();
            let restart = self.restart.take();
            let changed = restart.is_none() && self.settle_places();
            let bindings_changed = restart.is_none() && !changed && self.settle_bindings();
            let body_changed = restart.is_none() && !changed && !bindings_changed && self.settle_definition_scenes();
            if restart.is_none() && !changed && !bindings_changed && !body_changed {
                return self;
            }
            #[cfg(test)]
            places::tests::RUNS_AGAIN.with(|count| count.set(count.get() + 1));
            let mut closing = std::mem::take(&mut self.closing);
            if let Some((key, msg)) = restart {
                closing.insert(key, msg);
                self.places.plans.clear();
            }
            if bindings_changed || body_changed {
                self.places.plans.clear();
            }
            let plans = std::mem::take(&mut self.places.plans);
            let bindings = std::mem::take(&mut self.bindings.plans);
            let specific_names = std::mem::take(&mut self.bindings.specific_names);
            let body_plans = std::mem::take(&mut self.definition_scenes.plans);
            self = again(self);
            self.closing = closing;
            self.places.plans = plans;
            self.bindings.plans = bindings;
            self.bindings.specific_names = specific_names;
            self.definition_scenes.plans = body_plans;
        }
    }

    /// Whether this reading builds the level `lv`: every level, or with [`Options::only_file`]
    /// those of the file and the circuits above them.
    fn builds(&self, lv: LevelId) -> bool {
        self.only_levels.as_ref().is_none_or(|o| o.built.contains(&lv))
    }

    /// Whether this reading reads the records of the level `lv`: every level, or with
    /// [`Options::only_file`] every file level and the subcircuits [`OnlyLevels::read`] lists. A
    /// subcircuit's records reach no other level (2.6, 10.1), and a file's packages are those of
    /// the header records of its file levels (16.2, 16.3), so the levels a reading builds read as in
    /// the whole document; the other subcircuits have no names, rules or diagnostics of records.
    fn reads(&self, lv: LevelId) -> bool {
        self.doc.levels[lv].kind != LevelKind::Subckt || self.only_levels.as_ref().is_none_or(|o| o.read.contains(&lv))
    }

    /// Whether this reading gives the level `lv`: every level, or with [`Options::only_file`] those
    /// of the file.
    fn gives(&self, lv: LevelId) -> bool {
        self.only_levels.as_ref().is_none_or(|o| o.given.contains(&lv))
    }

    /// Evaluate the statements of a unit without a circuit once: definitions, rules, errors.
    fn classify(&mut self, u: UnitId) {
        if self.units[u].classified || !self.units[u].ok {
            return;
        }
        self.units[u].classified = true;
        for m in self.units[u].members.clone() {
            if !self.record_has_language(u, &m.header) {
                continue;
            }
            self.register_place_root(u, &m, None);
        }
        for m in self.units[u].members.clone() {
            if !self.record_has_language(u, &m.header) {
                continue;
            }
            let _ = self.eval_member_at(u, m.rec, m.k, m.embedded, &m.m, None, m.header.clone(), None, "");
        }
        self.check_unit_duplicates(u, None);
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
        self.inherited_body_rules(s);
        // its own text, then the records written above with scope= this subcircuit (2.6), whose
        // members are its own as well, evaluated in its unit
        let own = self.units[unit].members.iter().chain(&self.units[unit].scope_members);
        work.extend(own.map(|m| Work { u: unit, m: m.clone() }));
        let languages = self.doc.level_languages(lv);
        work.retain(|w| {
            let active = w.m.header.languages == crate::host::language::Languages::ALL
                || w.m.header.languages.overlaps(languages);
            if !active {
                // A section absent here can still exist below. Keep the record available
                // for inherited rules without checking or evaluating it in this scene.
                self.rules.insert(w.m.key(w.u));
            }
            active
        });
        for w in &work {
            self.register_place_root(w.u, &w.m, Some(s));
        }
        for w in work {
            if self.restart.is_some() {
                return;
            }
            let r =
                self.eval_member_at(w.u, w.m.rec, w.m.k, w.m.embedded, &w.m.m, Some(s), w.m.header.clone(), None, "");
            // a selection whose scope is this level's host objects is just empty below (10.3);
            // the 10.5 warning is for the other roles of a generating AS or ATTACH, not for any
            // other rule, which is a statement for the objects it selects (10.4)
            if matches!(r, Err(Stop::LocalRef))
                && self.units[w.u].level != Some(lv)
                && self.generation_rule(&w.m.m, w.u)
                && !self.scoped_selection(&w.m.m, w.u)
                && !self.stays_in_its_level(&w.m.m, w.u)
            {
                let at = w.m.m.segments.first().map(|sg| sg.start).unwrap_or(w.m.m.start);
                let loc = self.rec_loc(w.m.rec, at);
                self.sink.warning(
                    codes::RULE_USES_OWN_LEVEL_OBJECTS,
                    "this rule uses objects of the level it is written in, so it takes effect only there (10.5)",
                    loc,
                );
            }
        }
        if self.restart.is_some() {
            return;
        }
        self.check_unit_duplicates(unit, Some(s));
        self.run_scene(s);
    }

    /// Check already evaluated bindings without forcing failed or inactive statements (6.6).
    fn check_unit_duplicates(&mut self, unit: UnitId, scene: Option<SceneId>) {
        self.check_binding_duplicates(unit, scene);
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

    /// Whether a rule is a generation rule: one that applies a definition with AS or ATTACH (10.4).
    fn generation_rule(&self, m: &crate::syntax::Member, u: UnitId) -> bool {
        m.segments
            .iter()
            .filter_map(|sg| sg.call.as_ref())
            .any(|c| matches!(self.kw(&c.kw, u).as_str(), "AS" | "ATTACH"))
    }

    /// Whether a rule has a selection written in it and none of its selections writes
    /// `recursive=true`: it takes effect in its own level alone, and reaches no level below
    /// whatever sets it uses (10.1).
    fn stays_in_its_level(&self, m: &crate::syntax::Member, u: UnitId) -> bool {
        use crate::syntax::{ObjExpr, Value};
        fn visit(it: &Interp<'_>, m: &crate::syntax::Member, u: UnitId, written: &mut bool) -> bool {
            for seg in &m.segments {
                if let Some(call) = &seg.call {
                    let keyword = it.kw(&call.kw, u);
                    if matches!(keyword.as_str(), "DEFINE" | "DEFINE-SYMBOL" | "DEFINE-STYLE" | "USE-PACKAGE") {
                        return true;
                    }
                    if keyword.starts_with("SELECT-") {
                        *written = true;
                        let recursive = call.params.as_ref().is_some_and(|params| {
                            params.named.iter().any(|param| {
                                let name = if param.quoted { param.name.clone() } else { it.word(&param.name, u) };
                                name == "recursive"
                                    && match &param.value {
                                        Value::Bool(value) => *value,
                                        Value::Word(word) => it.word(word, u) == "true",
                                        // a hole may be true: the rule may reach below
                                        Value::Hole(_) => true,
                                        _ => false,
                                    }
                            })
                        });
                        if recursive {
                            return false;
                        }
                    }
                }
                for arg in &seg.args {
                    if let ObjExpr::Set(set) = &arg.obj {
                        if !set.members.iter().all(|member| visit(it, member, u, written)) {
                            return false;
                        }
                    }
                }
            }
            true
        }
        let mut written = false;
        visit(self, m, u, &mut written) && written
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

    /// A record written with scope= a subcircuit that is not below its level fails (2.6). A reading
    /// of one file checks the records of its levels.
    fn check_scopes(&mut self) {
        for u in 0..self.units.len() {
            let Some(lv) = self.units[u].level.filter(|_| self.units[u].kind == UnitKind::Level) else { continue };
            if self.units[u].scope_records.is_empty() || !self.gives(lv) {
                continue;
            }
            let below: Vec<usize> = self
                .doc
                .descendants(lv)
                .into_iter()
                .filter(|d| self.doc.levels[*d].kind == LevelKind::Subckt)
                .collect();
            for (rec, _ast, header) in self.units[u].scope_records.clone() {
                let target = header.scope.clone().unwrap_or_default();
                // each subcircuit by the rules of the file it is written in
                if !below.iter().any(|&d| self.doc.names_level(d, &target)) {
                    let loc = self.rec_loc(Some(rec), 0);
                    self.sink.error(codes::SCOPE_NO_SUBCIRCUIT, format!("scope=\"{target}\" finds no subcircuit below this level: the statements of the record fail (2.6)"), loc);
                    let n = self.recs[rec].text.chars().count();
                    self.note_failed(Some(rec), 0, n, false);
                }
            }
        }
    }

    /// A level that binds a `$` name it inherits from a level above or from a package hides that
    /// definition in this level and below: a hint (6.8, 16.2). A reading of one file checks its levels.
    fn check_shadowing(&mut self) {
        for lv in 0..self.doc.levels.len() {
            let u = self.level_unit[lv];
            if !self.units[u].ok || !self.file_ok(lv) || !self.gives(lv) {
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
                    } else if pn == STANDARD_PACKAGE {
                        format!("in the standard package {pn}")
                    } else {
                        format!("in {pn} of the standard package {STANDARD_PACKAGE}")
                    }
                } else {
                    let l = self.units[owner].level.map(|l| self.doc.levels[l].title()).unwrap_or_default();
                    format!("in the level above, {l}")
                };
                let written = packages::shown(&b);
                let loc = self.rec_loc(b.rec, b.off);
                self.sink.hint(codes::DEFINITION_HIDES_INHERITED, format!("{written} hides the definition of the same name {wher}; this level and below use this one (6.8)"), loc);
            }
        }
    }

    /// Packages used in the same level, or at the root, that define the same `$X`: the later
    /// hides the earlier (16.2). What one USE-PACKAGE brings in is the business of its package. A
    /// reading of one file checks its levels.
    fn check_package_shadowing(&mut self) {
        // each USE-PACKAGE, by its unit and place, and name: hinted once
        let mut seen: HashSet<(UnitId, (usize, usize), String)> = HashSet::new();
        for lv in 0..self.doc.levels.len() {
            let u = self.level_unit[lv];
            if !self.units[u].ok || !self.file_ok(lv) || !self.gives(lv) {
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
                            _ if std.0.contains(&qn) => {
                                format!("the standard package {STANDARD_PACKAGE}")
                            }
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
                        self.sink.hint(codes::PACKAGE_HIDES_PACKAGE, format!("{written} in {who} hides the definition of the same name in {what} {at}; this level and below use the one of {pn} (16.2)"), loc);
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
        let included_standard = if self.package_main.is_none() && self.include_std {
            self.implicit_std(packages::EffKey::Level(0)).map(|unit| self.pkg_ver(unit))
        } else {
            None
        };
        // a reading that counts the objects of the authored records for another gives those alone,
        // and draws nothing, once no constraint waits to be made (5.9, `NumberRun::cut`)
        let mut model = if self.cuts() && self.pending_constraints.is_empty() {
            Model::default()
        } else {
            self.check_auto_wire_geometry();
            let model = self.build_model();
            self.hint_crowded_nodes(&model);
            model
        };
        // the objects of the authored records for another reading (5.9): a reading that counts
        // them gives those alone; one without the records of the solver that gives them counts
        // them as that one does, before its problems are built
        let counting = self.numbers.count;
        let giving = self.numbers.give && !self.read_generated && self.package_main.is_none();
        let counted: HashMap<LevelId, AuthoredCount> = if counting || giving {
            (0..self.scenes.len()).map(|s| (self.scenes[s].level, self.counted_authored(s))).collect()
        } else {
            HashMap::new()
        };
        let (mut problems, results) =
            if self.want_problems && self.package_main.is_none() { self.build_problems() } else { Default::default() };
        // the levels above the file were built for the rules they hold, and are not given
        model.levels.retain(|l| self.gives(l.level));
        problems.retain(|p| self.gives(p.level));
        // the numbers and their checks (5.9)
        let numbering: Vec<LevelNumbering> = if !counting && self.package_main.is_none() {
            let mut all: Vec<LevelNumbering> = (0..self.scenes.len()).map(|s| self.level_numbering(s)).collect();
            all.retain(|n| self.gives(n.level));
            all
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
        let imported_aliases = self.units.iter().flat_map(|u| u.imported_aliases.iter().cloned()).collect();
        let block_symbols = match self.only_levels.clone() {
            Some(only) => {
                let levels = only.symbol_levels(&self.doc, self.symbols_only);
                levels.into_iter().map(|lv| (lv, self.block_symbol_of(lv))).collect()
            }
            None => Vec::new(),
        };
        Interpretation {
            model,
            sink: std::mem::take(&mut self.sink),
            included_standard,
            checking_package: checking,
            sources,
            refs: {
                let mut refs = std::mem::take(&mut self.refs);
                refs.retain(|h| self.gives(h.level));
                refs
            },
            failed: std::mem::take(&mut self.failed_spans),
            problems,
            read: self.read,
            block_symbols,
            package_files: std::mem::take(&mut self.package_files),
            numbering,
            unsupported,
            imported_aliases,
            recheck,
            counted,
            own,
            forward,
            results,
            given: AuthoredCounts::default(),
            doc: self.doc,
        }
    }

    /// Whether this reading counts the objects of the authored records for another and stops
    /// where nothing it gives can change (`NumberRun::cut`).
    fn cuts(&self) -> bool {
        self.numbers.count && self.numbers.cut
    }

    /// Does this reading count the objects of the authored records for another (5.9)?
    fn counts_first(&self) -> bool {
        self.numbers.authored.is_none() && self.needs_count()
    }
}

/// Interpret a host document: the effective model and the diagnostics. `sink` holds what the
/// host adapter reported, if anything.
///
/// The numbers take more than one reading of the document (5.9), only when a file is read with its
/// records of the solver:
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
    interpret_counted(doc, provider, &options.read_options(), sink, &mut AuthoredCounts::default(), Counts::None, true)
}

/// [`interpret`], the objects of the authored records taken from `counts` instead of counted by
/// reading the document without its records of the solver first, when they were counted of what
/// that reading of `doc` would count of (5.9), and `counts` given the objects this reading counts:
///
/// - read without the records of the solver ([`Options::generated`] false), its own, counted
///   before its problems are built, as the reading that counts them would count them;
/// - read with them, those it took, else those that the reading without them that it made first
///   counted.
///
/// So a reading with the records of the solver takes the objects of the reading without them made
/// before it, or of another reading with them, of the same document but for the text of its
/// records of the solver. `counts` hold what they were counted of ([`AuthoredCounts`]); when this
/// reading would count something else, it counts. A debug build counts again whenever they are
/// taken and checks that they are the same.
pub fn interpret_taking(
    doc: HostDocument,
    provider: &dyn Provider,
    options: &Options,
    sink: Sink,
    counts: &mut AuthoredCounts,
) -> Interpretation {
    interpret_counted(doc, provider, &options.read_options(), sink, counts, Counts::Checked, true)
}

/// The objects of the authored records of each level as the reading of a document without its
/// records of the solver counted them (5.9), which a reading of the document with its records of
/// the solver takes, with what they were counted of:
///
/// - the options of the reading but [`Options::generated`] and [`Options::problems`];
/// - each record of the host document but those of the solver, with its level, its place among all
///   its records, its file, its line and column, its lines and its text; the dialect and the
///   formats of the files;
/// - the texts of the host files, the lines of the records of the solver left out: lines that
///   hold such a record and nothing else, but the comment marker before it;
/// - the package files read, with their texts.
///
/// That is all that the count depends on: the reading that counts skips the records of the solver
/// (2.8), and each object counted is known by the record that made it, by its place among the
/// records, and by the file, line and column where that record starts (5.9). A document with a
/// record of the solver on a line that holds something else has no such digest, and its objects are
/// never taken. A reading takes them when its own document, options and packages read give the
/// same ([`interpret_taking`]).
#[derive(Clone, Default)]
pub struct AuthoredCounts(Option<Rc<Given>>);

/// The objects counted, with what they were counted of, when that is known.
struct Given {
    counts: Rc<HashMap<LevelId, AuthoredCount>>,
    of: Option<CountedOf>,
}

/// What a count of the objects of the authored records was made of ([`AuthoredCounts`]).
#[derive(Clone, PartialEq)]
struct CountedOf {
    options: ReadOptions,
    doc: DocKey,
    packages: Vec<(String, String)>,
}

/// A record of a host document as a count of the objects of the authored records depends on it:
/// its level, its place among all the records, its file, the line and column where it starts, its
/// lines and its text.
type RecordKey = (usize, usize, String, u32, u32, (u32, u32), String);

/// What a count of the objects of the authored records depends on in a host document
/// ([`AuthoredCounts`]), kept as it is.
#[derive(Clone, PartialEq)]
struct DocKey {
    /// The dialect and the formats of the files.
    formats: String,
    /// Every record but those of the solver.
    records: Vec<RecordKey>,
    /// The texts of the host files by name, without the lines of the records of the solver; shared
    /// with the key a count was given with when they are the same.
    texts: Vec<(String, Rc<str>)>,
}

/// How a reading takes and gives the objects of the authored records.
#[derive(Clone, Copy, PartialEq)]
enum Counts {
    /// Neither: [`interpret`].
    None,
    /// Takes them when they were counted of what it would count of, and gives them.
    Checked,
    /// Takes them whatever they were counted of, and gives them: [`interpret_numbering`] for the
    /// records of the solver of one level finished one after another in the document of the
    /// problem, as [`crate::solution::LevelFinisher::finish`] reads them.
    Trusted,
}

/// [`interpret`] for the checks of the numbers of the levels (5.9), [`Interpretation::numbering`]:
/// the objects of the authored records taken from `counts`, and `counts` given those this reading
/// counts: when `trusted`, whatever they were counted of, else as [`interpret_taking`] takes them.
/// The reading again that a record of the solver without `numbering`, or whose `numbering` is out
/// of date, takes is not made: the checks are those of the reading before it, which this gives. So
/// the rest of the interpretation is not that of [`interpret`] where such a record is read.
pub(crate) fn interpret_numbering(
    doc: HostDocument,
    provider: &dyn Provider,
    options: &Options,
    counts: &mut AuthoredCounts,
    trusted: bool,
) -> Interpretation {
    let how = if trusted { Counts::Trusted } else { Counts::Checked };
    interpret_counted(doc, provider, &options.read_options(), Sink::new(), counts, how, false)
}

/// The options that a count of the objects of the authored records is made with.
fn counting_options(options: &ReadOptions) -> ReadOptions {
    ReadOptions { generated: false, problems: false, ..options.clone() }
}

/// [`interpret`], the objects of the authored records taken from `counts` and given to it as `how`
/// says, and the reading again for the records of the solver whose numbers do not hold made when
/// `recheck`.
fn interpret_counted(
    doc: HostDocument,
    provider: &dyn Provider,
    options: &ReadOptions,
    sink: Sink,
    counts: &mut AuthoredCounts,
    how: Counts,
    recheck: bool,
) -> Interpretation {
    let host = sink.clone();
    // a reading without the records of the solver takes nothing: it counts as it reads. The key of
    // the document as given, made to check a count, is also that of the count this reading gives
    let give = how != Counts::None;
    let records_of_the_solver = || doc.levels.iter().any(|l| l.records.iter().any(|r| r.text.contains("generated")));
    let like = counts.0.as_ref().and_then(|given| given.of.as_ref()).map(|of| &of.doc);
    let key = (give && (!options.generated || records_of_the_solver())).then(|| doc_key(&doc, like)).flatten();
    let mut taken = match (&counts.0, how) {
        (Some(given), Counts::Trusted) if options.generated => Some(given.clone()),
        (Some(given), Counts::Checked) if options.generated => given
            .of
            .as_ref()
            .filter(|of| of.options == counting_options(options) && key.as_ref() == Some(&of.doc))
            .map(|_| given.clone()),
        _ => None,
    };
    #[cfg(debug_assertions)]
    let as_given = taken.is_some().then(|| doc.clone());
    let mut run = NumberRun { authored: taken.as_ref().map(|t| t.counts.clone()), give, ..NumberRun::default() };
    let mut out = read(doc, provider, options, sink, &host, &mut run);
    // a package file read otherwise than when they were counted, or read now and not then, such as
    // one missing then: counted again
    let packages_hold = |given: &Given, out: &Interpretation| {
        given.of.as_ref().is_none_or(|of| package_files_hold(&of.packages, &out.package_files))
    };
    if taken.as_ref().is_some_and(|t| how == Counts::Checked && !packages_hold(t, &out)) {
        taken = None;
        run = NumberRun { give, ..NumberRun::default() };
        out = read(out.doc, provider, options, host.clone(), &host, &mut run);
    }
    #[cfg(debug_assertions)]
    if let (Some(given), Some(doc)) = (&taken, &as_given) {
        check_taken(doc, provider, options, &host, &given.counts);
    }
    if give {
        let of = |packages: Vec<(String, String)>| {
            key.map(|doc| CountedOf { options: counting_options(options), doc, packages })
        };
        counts.0 = match taken {
            Some(given) => Some(given),
            None if !options.generated => Some(Rc::new(Given {
                of: of(out.package_files.clone()),
                counts: Rc::new(std::mem::take(&mut out.counted)),
            })),
            None => run
                .authored
                .clone()
                .map(|counted| Rc::new(Given { of: of(std::mem::take(&mut run.counted_packages)), counts: counted })),
        };
    }
    for _ in 0..3 {
        let same = run.own.as_ref().is_some_and(|own| **own == out.own);
        if !out.forward && (run.own.is_none() || same) {
            break;
        }
        run.own = Some(Rc::new(std::mem::take(&mut out.own)));
        out = read(out.doc, provider, options, host.clone(), &host, &mut run);
    }
    if recheck && !out.recheck.is_empty() {
        let recheck = std::mem::take(&mut out.recheck);
        let numbering = std::mem::take(&mut out.numbering);
        let mut again = NumberRun { refuse: recheck.refuse, stale: recheck.stale, ..run };
        out = read(out.doc, provider, options, host.clone(), &host, &mut again);
        out.numbering = numbering;
        out.recheck = NumberRun::default();
    }
    if give {
        out.given = counts.clone();
    }
    out
}

/// One reading of a document; first the reading without the records of the solver that counts the
/// objects of the authored records, when this one needs it and `run` does not have it yet (5.9).
fn read(
    doc: HostDocument,
    provider: &dyn Provider,
    options: &ReadOptions,
    sink: Sink,
    host: &Sink,
    run: &mut NumberRun,
) -> Interpretation {
    let mut it = Interp::new_run(doc, provider, options, sink, run.clone());
    if it.counts_first() {
        COUNTINGS.with(|n| n.set(n.get() + 1));
        let without = ReadOptions { generated: false, problems: false, ..options.clone() };
        let first = counting_reading(&it.doc, provider, &without, host);
        if run.give {
            run.counted_packages = first.package_files.clone();
        }
        run.authored = Some(Rc::new(first.counted));
        it.numbers.authored = run.authored.clone();
    }
    let again = run.clone();
    it.run_settled(|x| Interp::new_run(x.doc, provider, options, host.clone(), again.clone()))
}

/// The reading of `doc` without the records of the solver, read with `without`, that counts the
/// objects of the authored records for a reading with them (5.9). It stops where nothing it gives
/// can change: it resolves no style, layer or routed tree, and builds no drawing
/// (`NumberRun::cut`). A debug build reads to the end too, and both have to count the same and read
/// the same package files.
fn counting_reading(doc: &HostDocument, provider: &dyn Provider, without: &ReadOptions, host: &Sink) -> Interpretation {
    let reading = |cut: bool| {
        let counting = NumberRun { count: true, cut, ..NumberRun::default() };
        Interp::new_run(doc.clone(), provider, without, host.clone(), counting.clone())
            .run_settled(|x| Interp::new_run(x.doc, provider, without, host.clone(), counting.clone()))
    };
    let first = reading(true);
    #[cfg(debug_assertions)]
    {
        let whole = reading(false);
        assert!(
            first.counted == whole.counted && first.package_files == whole.package_files,
            "a reading that counts and stops early counts otherwise than one read to the end"
        );
    }
    first
}

/// Whether the package files `read` are those of `counted`, each by its path with the same text:
/// one read now and not when counted, such as one missing then, may change the count.
fn package_files_hold(counted: &[(String, String)], read: &[(String, String)]) -> bool {
    let read: HashMap<&str, &str> = read.iter().map(|(path, text)| (path.as_str(), text.as_str())).collect();
    let counted: HashMap<&str, &str> = counted.iter().map(|(path, text)| (path.as_str(), text.as_str())).collect();
    read == counted
}

/// What the count of the objects of the authored records of `doc` depends on in the host document
/// ([`AuthoredCounts`]), with the texts of `like` where they are the same; None when a record of
/// the solver is on a line that holds anything else but a comment marker before it.
fn doc_key(doc: &HostDocument, like: Option<&DocKey>) -> Option<DocKey> {
    let mut dialects: Vec<String> = doc.file_dialects.iter().map(|(f, x)| format!("{f}={x:?}")).collect();
    dialects.sort();
    let formats = format!("{:?}|{:?}|{}", doc.dialect, doc.file_formats, dialects.join(","));
    let texts: HashMap<&str, &str> = doc.sources.iter().map(|(file, text)| (file.as_str(), text.as_str())).collect();
    let mut left_out: HashMap<&str, Vec<(u32, u32)>> = HashMap::new();
    let mut records = Vec::new();
    let mut place = 0usize;
    for (level, lv) in doc.levels.iter().enumerate() {
        for r in &lv.records {
            if r.text.contains("generated") && unit::is_generated_record(&crate::syntax::parse_record(&r.text)) {
                if !alone_on_its_lines(r, texts.get(r.file.as_str())?) {
                    return None;
                }
                left_out.entry(r.file.as_str()).or_default().push(r.lines);
            } else {
                let at = r.loc(0);
                records.push((level, place, r.file.clone(), at.line, at.col, r.lines, r.text.clone()));
            }
            place += 1;
        }
    }
    let texts = doc
        .sources
        .iter()
        .enumerate()
        .map(|(k, (file, text))| {
            let kept: std::borrow::Cow<str> = match left_out.get(file.as_str()) {
                None => text.into(),
                Some(spans) => text
                    .split_inclusive('\n')
                    .enumerate()
                    .filter(|(k, _)| {
                        let no = *k as u32 + 1;
                        !spans.iter().any(|&(first, last)| first <= no && no <= last)
                    })
                    .map(|(_, line)| line)
                    .collect::<String>()
                    .into(),
            };
            let same = like.and_then(|l| l.texts.get(k)).filter(|(f, t)| f == file && **t == *kept);
            (file.clone(), same.map_or_else(|| Rc::from(kept.as_ref()), |(_, t)| t.clone()))
        })
        .collect();
    Some(DocKey { formats, records, texts })
}

/// Whether record `r` is alone on its lines of `text`, the text of its file: each of them holds a
/// part of it and before that nothing but a comment marker, which no letter or digit is in.
fn alone_on_its_lines(r: &crate::host::Record, text: &str) -> bool {
    let chars: Vec<char> = r.text.chars().collect();
    let (first, last) = r.lines;
    if last < first || r.segs.len() != (last - first + 1) as usize {
        return false;
    }
    let lines: Vec<&str> = text.split('\n').skip(first as usize - 1).take(r.segs.len()).collect();
    lines.len() == r.segs.len()
        && r.segs.iter().zip(&lines).enumerate().all(|(k, (seg, line))| {
            let end = r.segs.get(k + 1).map_or(chars.len(), |next| next.off.saturating_sub(1));
            let Ok(col) = usize::try_from(seg.col) else { return false };
            let line: Vec<char> = line.trim_end_matches('\r').chars().collect();
            seg.line == first + k as u32
                && seg.off <= end
                && end <= chars.len()
                && col <= line.len()
                && !line[..col].iter().any(|c| c.is_alphanumeric())
                && line[col..].starts_with(&chars[seg.off..end])
        })
}

/// A debug build's check of the objects of the authored records taken from a reading of another
/// document, `taken`: when a reading of `doc` counts them, it counts them again and they have to
/// be the same.
#[cfg(debug_assertions)]
fn check_taken(
    doc: &HostDocument,
    provider: &dyn Provider,
    options: &ReadOptions,
    host: &Sink,
    taken: &HashMap<LevelId, AuthoredCount>,
) {
    let it = Interp::new_run(doc.clone(), provider, options, host.clone(), NumberRun::default());
    if !it.needs_count() {
        return;
    }
    let without = ReadOptions { generated: false, problems: false, ..options.clone() };
    let counting = NumberRun { count: true, ..NumberRun::default() };
    let counted = Interp::new_run(doc.clone(), provider, &without, host.clone(), counting.clone())
        .run_settled(|x| Interp::new_run(x.doc, provider, &without, host.clone(), counting.clone()))
        .counted;
    assert!(*taken == counted, "the objects of the authored records taken are not those of the document read");
}

thread_local! {
    /// The readings without the records of the solver made on this thread to count the objects of
    /// the authored records for another, a debug build's counts again to check one taken left out.
    static COUNTINGS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many readings of a document without its records of the solver this thread has made to
/// count the objects of the authored records for a reading with them (5.9), for tests.
#[doc(hidden)]
pub fn countings() -> usize {
    COUNTINGS.with(std::cell::Cell::get)
}

/// Resolve the host reader's diagnostics with each file or section's language version.
/// `default_version` is used for an undeclared main file; includes inherit normally.
/// This reads version declarations without evaluating other RNL or loading packages.
pub fn resolve_host_diagnostics(doc: HostDocument, default_version: &str, sink: Sink) -> (HostDocument, Sink) {
    let provider = crate::source::MemoryProvider::new();
    let options = Options { std: false, ..Options::default() }.read_options();
    let mut it = Interp::blank(doc, &provider, &options, sink, NumberRun::default());
    it.init_host_units();
    let main = it.level_unit[0];
    it.units[main].version = default_version.to_string();
    it.read_file_versions();
    it.check_host_diagnostics();
    it.check_host_input_diagnostics();
    (it.doc, it.sink)
}

/// What reading a host document gives before any RNL is interpreted: the host reader's diagnostics,
/// with those of each file or section resolved by its language version as an interpretation
/// resolves them, and the language version of each file level, None for the other levels. This
/// reads the version declarations only: it evaluates no other RNL and loads no package.
pub fn read_file_versions(doc: HostDocument, sink: Sink) -> (HostDocument, Sink, Vec<Option<String>>) {
    let provider = crate::source::MemoryProvider::new();
    let options = Options { std: false, ..Options::default() }.read_options();
    let mut it = Interp::blank(doc, &provider, &options, sink, NumberRun::default());
    // as Interp::new_run starts an interpretation
    it.init_host_units();
    it.read_file_versions();
    it.check_host_diagnostics();
    it.check_host_input_diagnostics();
    let versions = (0..it.doc.levels.len())
        .map(|lv| (it.doc.levels[lv].kind == LevelKind::File).then(|| it.version_of(Some(it.level_unit[lv]))))
        .collect();
    (it.doc, it.sink, versions)
}

/// Check a package file on its own (2.7).
pub fn check_package(path: &str, provider: &dyn Provider, options: &Options) -> Interpretation {
    Interp::new_package(path, provider, options).run_settled(|_| Interp::new_package(path, provider, options))
}
