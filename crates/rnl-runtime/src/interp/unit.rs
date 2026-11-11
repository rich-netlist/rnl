//! Units of RNL text: host levels and packages (standard 2.7, 2.8, 6.1, 16.1–16.5).
//!
//! A unit has its own name table. Reading a unit parses its records, reads the header record
//! (version, aliases, terminal names, root packages) and collects the members of the others.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::diag::{codes, Code, Extra, Loc, Severity};
use crate::host::language::Languages;
use crate::host::{file_name, Level, LevelId, LevelKind};
use crate::records::{package_lines, scan_package_run};
use crate::source::{dirname, join, normalize};
use crate::syntax::{self, Member, ObjExpr, Params, RecordAst, SetExpr, Value};
use crate::words::{words, ParamType};

use super::obj::{Lit, MemberKey, Priority, RecId, UnitId};
use super::Interp;

/// The first correctly formed VERSION declaration of a header, without reporting diagnostics.
/// Carrier scanning uses the same declaration rules before interpreting a file's members.
pub(crate) fn declared_record_version(text: &str) -> Option<(String, usize)> {
    let ast = syntax::parse_record(text);
    if ast.error.is_some() || ast.header.as_ref().and_then(|h| h.get("type")) != Some(&Value::Str("header".into())) {
        return None;
    }
    let body = ast.body?;
    let (segment, call) = body.members.iter().find_map(|member| {
        if member.error.is_some() || member.segments.len() != 1 || member.bind().is_some() {
            return None;
        }
        let segment = &member.segments[0];
        segment.call.as_ref().filter(|call| call.kw == "VERSION").map(|call| (segment, call))
    })?;
    let params = params_by_name("VERSION", call.params.as_ref(), true, str::to_string).ok()?;
    if !segment.args.is_empty() {
        return None;
    }
    match params.get("version") {
        Some(Value::Str(version)) => Some((version.clone(), call.start)),
        _ => None,
    }
}

/// The versions of the language this implementation reads (16.1): 0.2, 0.3, and 0.4, whose text
/// is still a draft. Every rule that 0.3 changes applies to the files of 0.3 and of the
/// versions after it ([`keeps_0_3`]), so a file of 0.2 reads as it did before 0.3 was read.
pub const READ_VERSIONS: [&str; 3] = ["0.2", "0.3", "0.4"];
/// The version a main file, or a package file, without a header record is read in: the tool's
/// default (16.1). An included file without one follows the file that includes it.
pub const DEFAULT_VERSION: &str = "0.3";

/// Does a file of the language version `version` keep the rules that 0.3 changes? 0.3 and every
/// version after it that this implementation reads do, by the order of versions (16.1); 0.2 does
/// not, nor a version it does not read, none of whose RNL is interpreted.
pub fn keeps_0_3(version: &str) -> bool {
    READ_VERSIONS.contains(&version) && version_key(version) >= (0, 3)
}

/// Does a file of the language version `version` follow the rules that 0.4 changes? 0.4 and every
/// version after it that this implementation reads do, by the order of versions (16.1).
pub fn keeps_0_4(version: &str) -> bool {
    READ_VERSIONS.contains(&version) && version_key(version) >= (0, 4)
}

/// The versions this implementation reads, as a message lists them: "0.2, 0.3 and 0.4".
fn read_versions() -> String {
    match READ_VERSIONS.split_last() {
        Some((last, [])) => last.to_string(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// The draft 0.4 and earlier languages have no package compatibility. Keep the published
/// 0.2/0.3 compatibility within the older languages (16.1).
pub(super) fn package_languages_match(language: &str, reader: &str) -> bool {
    (language == "0.4") == (reader == "0.4")
}

/// Prefer packages on the reader's side of the 0.4 language boundary (16.1).
fn finds_unnamed(language: &str, reader: &str) -> bool {
    package_languages_match(language, reader)
        && (version_key(language) <= version_key(DEFAULT_VERSION) || version_key(language) <= version_key(reader))
}
const HEADER_KW: [&str; 5] = ["VERSION", "PACKAGE", "ALIAS", "DEVICE", "USE-ALIASES"];
/// A second VERSION in a header record (16.1).
pub const SECOND_VERSION: &str =
    "a file declares its version once (16.1): this VERSION is ignored, and the first one stands";

/// The fields of a record header that affect interpretation (2.8).
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub type_: String,
    pub priority: Option<Priority>,
    pub scope: Option<String>,
    /// The check of the numbers that the record of the solver uses, as written: a field of the
    /// draft 0.3, only with `type="generated"` (5.9).
    pub numbering: Option<String>,
    /// Host sections on which this record acts (2.8); ALL when no field is written.
    pub languages: Languages,
}

impl Default for Header {
    fn default() -> Header {
        Header { type_: "authored".into(), priority: None, scope: None, numbering: None, languages: Languages::ALL }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Level,
    Package,
}

/// A name bound in a unit (6.6).
#[derive(Clone, Debug)]
pub struct Binding {
    pub prefix: char,
    pub name: String,
    pub member: Rc<Member>,
    pub rec: Option<RecId>,
    pub owner: UnitId,
    pub index: usize,
    pub embedded: bool,
    pub off: usize,
    /// As spelled, through a `$` alias (16.5).
    pub written: String,
    pub header: Rc<Header>,
}

impl Binding {
    pub fn key(&self) -> MemberKey {
        MemberKey::M { unit: self.owner, rec: self.rec.unwrap_or(usize::MAX), k: self.index, embedded: self.embedded }
    }
}

/// A member of a unit's records, with the header of its record.
#[derive(Clone, Debug)]
pub struct UnitMember {
    pub rec: Option<RecId>,
    pub k: usize,
    pub embedded: bool,
    pub m: Rc<Member>,
    pub header: Rc<Header>,
}

impl UnitMember {
    pub fn key(&self, unit: UnitId) -> MemberKey {
        MemberKey::M { unit, rec: self.rec.unwrap_or(usize::MAX), k: self.k, embedded: self.embedded }
    }
}

/// A USE-PACKAGE or USE-ALIASES (16.2, 16.5). In the header record of a package file, a
/// USE-PACKAGE is a reference to another package (16.2).
#[derive(Clone, Debug)]
pub struct UseDecl {
    /// At least one unqualified header declares this effective reference (14.10).
    pub solver_config: bool,
    pub name: String,
    pub version: Option<String>,
    pub path: Option<String>,
    pub body: Option<Rc<SetExpr>>,
    pub unit: UnitId,
    pub in_header: bool,
    pub languages: Languages,
    /// (record, member index): its place in the text.
    pub pos: (usize, usize),
    pub rec: Option<RecId>,
    pub offset: usize,
    pub pkg: Option<UnitId>,
}

impl UseDecl {
    /// Is it the same statement as `other`?
    pub fn same(&self, other: &UseDecl) -> bool {
        self.unit == other.unit && self.rec == other.rec && self.pos == other.pos && self.offset == other.offset
    }
}

/// The aliases scoped to one definition: (alias, (original, where it is declared)).
pub type ScopedAliases = Vec<(String, (String, Loc))>;

/// A DEVICE statement (6.12).

#[derive(Clone, Debug)]
pub struct DeviceDecl {
    pub name: String,
    pub model: Option<String>,
    pub terminals: Option<Vec<Lit>>,
    pub mtype: Option<String>,
    pub loc: Loc,
    pub package: String,
    pub languages: Languages,
}

#[derive(Clone, Debug)]
pub struct AliasDecl {
    pub alias: String,
    pub original: String,
    pub definition: Option<String>,
    pub loc: Loc,
}

#[derive(Debug)]
pub struct Unit {
    pub solver_config: Vec<crate::problem::SolverSetting>,
    pub kind: UnitKind,
    pub name: String,
    /// The host level of a level unit.
    pub level: Option<LevelId>,
    /// The package file level of a package read from a file (index into `pkg_levels`).
    pub pkg_level: Option<usize>,
    pub pkg_name: String,
    pub pkg_version: String,
    pub version: String,
    pub names: HashMap<(char, String), Binding>,
    pub name_order: Vec<(char, String)>,
    pub dups: Vec<Binding>,
    pub members: Vec<UnitMember>,
    pub uses: Vec<UseDecl>,
    /// A package file: the packages its header record references, in order (16.2).
    pub refs: Vec<UseDecl>,
    pub header_record: Option<RecId>,
    pub header_records: Vec<RecId>,
    /// alias -> original: keywords with "@", `$` names with "$", other standard words bare.
    pub aliases: HashMap<String, String>,
    pub alias_decls: Vec<AliasDecl>,
    pub alias_pending: Vec<(String, String, Loc)>,
    pub alias_rejected: HashMap<String, String>,
    /// The aliases its header record imports from alias packages with @USE-ALIASES (16.5).
    pub imported_aliases: Vec<super::ImportedAlias>,
    /// Definition-scoped aliases as declared: (definition, alias, original, loc, source package).
    pub scoped_raw: Vec<(String, String, String, Loc, String)>,
    /// "$NAME" -> [(alias, (original, loc))].
    pub scoped: HashMap<String, ScopedAliases>,
    pub header_kws: Vec<String>,
    pub other_records: usize,
    pub devices: Vec<DeviceDecl>,
    pub ok: bool,
    pub scope_records: Vec<(RecId, Rc<RecordAst>, Rc<Header>)>,
    /// The indexes in `scope_records` by the key of the subcircuit named in scope=, in order.
    pub scope_index: HashMap<String, Vec<usize>>,
    /// In a subcircuit's unit, the members of the records written above with scope= it, in the
    /// order they take effect after its own text (2.6). The names they bind are its own (6.7).
    pub scope_members: Vec<UnitMember>,
    /// Where each of those records takes effect: the outer ones later (2.6).
    pub scope_places: HashMap<RecId, i64>,
    pub classified: bool,
    /// An embedded package reads with the aliases of the file it is written in (16.5).
    pub alias_from: Option<UnitId>,
    pub embedded: bool,
}

impl Unit {
    pub fn new(kind: UnitKind, name: &str) -> Unit {
        Unit {
            solver_config: Vec::new(),
            kind,
            name: name.to_string(),
            level: None,
            pkg_level: None,
            pkg_name: String::new(),
            pkg_version: String::new(),
            version: String::new(),
            names: HashMap::new(),
            name_order: Vec::new(),
            dups: Vec::new(),
            members: Vec::new(),
            uses: Vec::new(),
            refs: Vec::new(),
            header_record: None,
            header_records: Vec::new(),
            aliases: HashMap::new(),
            alias_decls: Vec::new(),
            alias_pending: Vec::new(),
            alias_rejected: HashMap::new(),
            imported_aliases: Vec::new(),
            scoped_raw: Vec::new(),
            scoped: HashMap::new(),
            header_kws: Vec::new(),
            other_records: 0,
            devices: Vec::new(),
            ok: true,
            scope_records: Vec::new(),
            scope_index: HashMap::new(),
            scope_members: Vec::new(),
            scope_places: HashMap::new(),
            classified: false,
            alias_from: None,
            embedded: false,
        }
    }

    pub fn pkg(&self) -> &str {
        if self.kind == UnitKind::Package {
            &self.pkg_name
        } else {
            ""
        }
    }
}

/// Where an effective package sits (8.3, 16.2, 16.3). `j` is its place in what one USE-PACKAGE
/// brings in: the packages it references first, then the package itself (16.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// At the root: `k` counts the USE-PACKAGEs at the root that bring in packages not placed
    /// there yet, the standard package's first, then those of the header records (16.3).
    Root(usize, usize),
    /// At a USE-PACKAGE in a level: the level, the (record, member) position, and `j`.
    Level(LevelId, (usize, usize), usize),
}

/// A package in effect on a chain of levels.
#[derive(Clone, Debug)]
pub struct EffPackage {
    /// Reached by an effective use path with no host-language restriction (14.10).
    pub solver_config: bool,
    /// The USE-PACKAGE written in a file that brings it in; none for the standard package used
    /// implicitly (16.3) and for a package checked on its own (2.7).
    pub decl: Option<UseDecl>,
    pub unit: UnitId,
    pub place: Place,
    pub languages: Languages,
}

/// The option most like `word`, ignoring case, when it is like enough; of options equally like it,
/// the first. The caller gives the options in a fixed order, so that the answer is the same every time.
pub fn closest<'a>(word: &str, options: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut best = None;
    let mut score = 0.0;
    let w = word.to_lowercase();
    for o in options {
        let s = similarity(&w, &o.to_lowercase());
        if s > score {
            best = Some(o.to_string());
            score = s;
        }
    }
    if score >= 0.6 {
        best
    } else {
        None
    }
}

/// Ratcliff/Obershelp similarity, as Python's difflib computes it.
pub fn similarity(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    fn matches(a: &[char], b: &[char]) -> usize {
        if a.is_empty() || b.is_empty() {
            return 0;
        }
        let (mut best, mut bi, mut bj) = (0usize, 0usize, 0usize);
        for i in 0..a.len() {
            for j in 0..b.len() {
                let mut k = 0;
                while i + k < a.len() && j + k < b.len() && a[i + k] == b[j + k] {
                    k += 1;
                }
                if k > best {
                    best = k;
                    bi = i;
                    bj = j;
                }
            }
        }
        if best == 0 {
            return 0;
        }
        best + matches(&a[..bi], &b[..bj]) + matches(&a[bi + best..], &b[bj + best..])
    }
    2.0 * matches(&a, &b) as f64 / (a.len() + b.len()) as f64
}

fn str_val(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::Str(s)) => Some(s.clone()),
        _ => None,
    }
}

pub(crate) fn version_key(v: &str) -> (i64, i64) {
    let mut it = v.split('.');
    match (it.next().and_then(|a| a.parse().ok()), it.next().and_then(|b| b.parse().ok()), it.next()) {
        (Some(a), Some(b), None) => (a, b),
        _ => (-1, -1),
    }
}

/// Decimal parts compared by length and digits, without a machine-integer limit (16.1).
type DecimalVersion<'a> = ((usize, &'a str), (usize, &'a str));

fn decimal_version(v: &str) -> Option<DecimalVersion<'_>> {
    let (a, b) = v.split_once('.')?;
    if [a, b].iter().any(|s| s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit())) {
        return None;
    }
    fn part(s: &str) -> (usize, &str) {
        let digits = s.trim_start_matches('0');
        (digits.len(), digits)
    }
    Some((part(a), part(b)))
}

/// The same numeric draft version denotes one package reading, whatever its leading zeroes.
pub(super) fn package_cache_key(name: String, version: String, modern: bool) -> (String, String, bool) {
    let version = if modern {
        decimal_version(&version).map_or_else(
            || version.clone(),
            |(a, b)| format!("{}.{}", if a.1.is_empty() { "0" } else { a.1 }, if b.1.is_empty() { "0" } else { b.1 }),
        )
    } else {
        version
    };
    (name, version, modern)
}

pub(super) fn valid_package_version(v: &str) -> bool {
    decimal_version(v).is_some()
}

pub(super) fn package_version_cmp(a: &str, b: &str, modern: bool) -> std::cmp::Ordering {
    if modern {
        decimal_version(a).cmp(&decimal_version(b))
    } else {
        version_key(a).cmp(&version_key(b))
    }
}

pub(super) fn same_package_major(a: &str, b: &str, modern: bool) -> bool {
    if modern {
        let (Some(ka), Some(kb)) = (decimal_version(a), decimal_version(b)) else { return false };
        if ka.0 .0 == 0 || kb.0 .0 == 0 {
            ka == kb
        } else {
            ka.0 == kb.0
        }
    } else {
        let (ka, kb) = (version_key(a), version_key(b));
        if ka.0 == 0 || kb.0 == 0 {
            ka == kb
        } else {
            ka.0 == kb.0
        }
    }
}

pub(super) fn version_ok(want: Option<&str>, have: &str, modern: bool) -> bool {
    match want {
        None => !modern || valid_package_version(have),
        Some(w) if modern => same_package_major(w, have, true) && !package_version_cmp(have, w, true).is_lt(),
        Some(w) => w == have || (same_package_major(w, have, false) && version_key(have) > version_key(w)),
    }
}

/// What a package file's header record declares (2.7, 2.8): its PACKAGE, by name and version, and
/// its VERSION, the language version it is written in (16.1). The header record is the file's
/// first record, and its statements count as reading the file counts them (`Interp::read_header`):
/// a PACKAGE or a VERSION written in a comment or a string, such as an example in the file's header
/// comment, declares nothing; the first VERSION stands. In draft 0.4 the first PACKAGE
/// stands too; released versions keep their last declaration.
fn header_decls(text: &str) -> (Option<(String, String)>, Option<String>) {
    let recs = scan_package_run(&package_lines(text), "", &mut crate::diag::Sink::new());
    let Some(first) = recs.first() else { return (None, None) };
    let ast = syntax::parse_record(&first.text);
    let header = ast.header.as_ref().and_then(|h| h.get("type"));
    let (None, Some(Value::Str(kind)), Some(body)) = (&ast.error, header, &ast.body) else { return (None, None) };
    if kind.as_str() != "header" {
        return (None, None);
    }
    let text_of = |v: Option<&Value>| match v {
        Some(Value::Str(s)) => Some(s.clone()),
        _ => None,
    };
    let (mut package, mut version, mut versions) = (None, None, 0);
    let (mut first_package, mut packages) = (None, 0);
    for m in &body.members {
        if m.error.is_some() || m.segments.len() != 1 || m.bind().is_some() {
            continue;
        }
        let seg = &m.segments[0];
        let Some(call) = &seg.call else { continue };
        let kw = call.kw.as_str();
        if kw == "VERSION" {
            versions += 1;
        } else if kw == "PACKAGE" {
            packages += 1;
        }
        if !(kw == "PACKAGE" || (kw == "VERSION" && versions == 1)) {
            continue;
        }
        let Ok(p) = params_by_name(kw, call.params.as_ref(), true, str::to_string) else { continue };
        if !seg.args.is_empty() {
            continue;
        }
        if kw == "VERSION" {
            version = text_of(p.get("version"));
        } else {
            package = text_of(p.get("name")).zip(text_of(p.get("version")));
            if packages == 1 {
                first_package = package.clone();
            }
        }
    }
    if version.as_deref().is_some_and(keeps_0_4) {
        package = first_package.filter(|(_, v)| valid_package_version(v));
    }
    (package, version)
}

/// "@PACKAGE(name="…", version="…")" in a package file's header record (2.7).
pub fn scan_package_decl(text: &str) -> Option<(String, String)> {
    header_decls(text).0
}

/// The language version a package file declares, "@VERSION(…)" in its header record, read as the
/// version of the file is (16.1); None when it declares none. The standard series a package is of
/// is written in this version (16.3).
pub fn scan_language_decl(text: &str) -> Option<String> {
    header_decls(text).1
}

/// The parameters of `@kw` by name, those written by position named in its signature's order;
/// nothing twice, none it does not have, and none it requires left out (4.2 rule 9, 5.3); or why
/// they cannot be read, with its kind. A header record is in the standard form (`std_form`);
/// elsewhere `word` gives the standard form of a name written unquoted, which may be an alias (16.5).
pub(crate) fn params_by_name(
    kw: &str,
    p: Option<&Params>,
    std_form: bool,
    word: impl Fn(&str) -> String,
) -> Result<HashMap<String, Value>, (Code, String)> {
    params_by_name_in(kw, p, std_form, word, DEFAULT_VERSION)
}

pub(super) fn params_by_name_in(
    kw: &str,
    p: Option<&Params>,
    std_form: bool,
    word: impl Fn(&str) -> String,
    version: &str,
) -> Result<HashMap<String, Value>, (Code, String)> {
    params_by_name_mode(kw, p, std_form, word, version, false)
}

/// Declaration headers in a body can receive arrays from enclosing parameters.
pub(super) fn definition_header_params(
    p: Option<&Params>,
    word: impl Fn(&str) -> String,
    version: &str,
) -> Result<HashMap<String, Value>, (Code, String)> {
    params_by_name_mode("DEFINE", p, false, word, version, true)
}

fn params_by_name_mode(
    kw: &str,
    p: Option<&Params>,
    std_form: bool,
    word: impl Fn(&str) -> String,
    version: &str,
    deferred: bool,
) -> Result<HashMap<String, Value>, (Code, String)> {
    let spec = words().keyword(kw).unwrap();
    let params = spec.params_in(version);
    let names: Vec<&str> = params.iter().map(|q| q.name.as_str()).collect();
    let order = &spec.positional;
    let modern = keeps_0_4(version);
    let site =
        crate::parameter_errors::Site::Header { standard: std_form, first: names.first().copied().unwrap_or("name") };
    let mut out = HashMap::new();
    if let Some(p) = p {
        if !p.positional.is_empty() {
            if order.is_empty() {
                return Err(crate::parameter_errors::positional(kw, site, modern));
            }
            if p.positional.len() > order.len() {
                return Err(crate::parameter_errors::too_many(kw, order, p.positional.len()));
            }
            for (name, v) in order.iter().zip(&p.positional) {
                out.insert(name.clone(), v.clone());
            }
        }
        for n in &p.named {
            let std = if std_form || n.quoted { n.name.clone() } else { word(&n.name) };
            if !names.contains(&std.as_str()) {
                return Err(crate::parameter_errors::unknown(kw, n, &std, &names, site, modern));
            }
            if out.contains_key(&std) {
                return Err(crate::parameter_errors::twice(kw, &std));
            }
            out.insert(std, n.value.clone());
        }
    }
    for q in &params {
        if q.required && !out.contains_key(&q.name) {
            return Err((codes::PARAM_MISSING_REQUIRED, format!("@{kw} lacks the parameter {} (5.3)", q.name)));
        }
        if let Some(value) = out.get(&q.name) {
            if deferred && matches!(value, Value::Hole(_)) {
                continue;
            }
            let valid = match q.ty {
                ParamType::String => matches!(value, Value::Str(_)),
                ParamType::Array => matches!(value, Value::Arr(_)),
                _ => true,
            };
            if !valid {
                return Err((codes::PARAM_WRONG_TYPE, parameter_type_message(kw, &q.name, &q.ty, keeps_0_4(version))));
            }
        }
    }
    Ok(out)
}

/// Share the expected-type description between headers and evaluated statements (5.3).
pub(super) fn parameter_type_message(kw: &str, name: &str, ty: &ParamType, modern: bool) -> String {
    if !modern {
        return format!("the {name} of @{kw} has the wrong type (5.3)");
    }
    let expected = match ty {
        ParamType::Number => "a number",
        ParamType::String => "a string",
        ParamType::Bool => "true or false",
        ParamType::Array => "an array",
        ParamType::Color => "a colour string, such as \"#RRGGBB\"",
        ParamType::Term if kw == "PIN" => "a positive terminal number, a terminal name or an array of them",
        ParamType::Term => "a positive terminal number or a terminal name",
        ParamType::Enum(_) => "one of the allowed values",
        ParamType::Any => "a literal value",
    };
    format!("the {name} of @{kw} must be {expected} (5.3)")
}

/// A record whose head writes `type="generated"`: the solver's result (2.8).
pub fn is_generated_record(ast: &RecordAst) -> bool {
    ast.header.as_ref().and_then(|h| h.get("type")).is_some_and(|v| *v == Value::Str("generated".into()))
}

/// An alias package: a package file whose header has only VERSION, PACKAGE and ALIAS, and no other
/// record (16.5).
pub fn is_alias_package(u: &Unit) -> bool {
    u.kind == UnitKind::Package
        && u.ok
        && u.pkg_level.is_some()
        && u.header_record.is_some()
        && !u.pkg_name.is_empty()
        && u.other_records == 0
        && u.header_kws.iter().all(|k| matches!(k.as_str(), "VERSION" | "PACKAGE" | "ALIAS"))
}

impl Interp<'_> {
    // ------------------------------------------------------------------ records and diagnostics

    pub(crate) fn rec_loc(&self, rec: Option<RecId>, off: usize) -> Option<Loc> {
        rec.map(|r| self.recs[r].loc(off))
    }

    pub(crate) fn diag_in(
        &mut self,
        sev: Severity,
        code: Code,
        msg: impl Into<String>,
        loc: Option<Loc>,
        pkg: &str,
    ) -> crate::diag::DiagId {
        self.sink.add(sev, code, msg, loc, Extra { in_package: pkg.to_string(), ..Extra::default() })
    }

    /// Parse a record once. In a file of the draft 0.3, `%#3` is a number (5.9); in 0.2 it stays a
    /// lexical error.
    pub(crate) fn parse(&mut self, rec: RecId) -> Rc<RecordAst> {
        if let Some(a) = &self.asts[rec] {
            return a.clone();
        }
        let numbers = self.follows_0_3(self.rec_unit[rec]);
        let places = self.follows_0_4(self.rec_unit[rec]);
        self.places.enabled |= places;
        let a = Rc::new(syntax::parse_record_language(&self.recs[rec].text, numbers, places));
        self.asts[rec] = Some(a.clone());
        a
    }

    /// Add a record written in the text of unit `u`.
    pub(crate) fn add_record(&mut self, r: crate::host::Record, u: UnitId) -> RecId {
        self.recs.push(r);
        self.rec_unit.push(u);
        self.asts.push(None);
        self.rec_chars.push(None);
        self.recs.len() - 1
    }

    /// The characters of a record's text, collected once: offsets into a record count characters.
    pub(crate) fn record_chars(&mut self, rec: RecId) -> Rc<[char]> {
        if let Some(c) = &self.rec_chars[rec] {
            return c.clone();
        }
        let c: Rc<[char]> = self.recs[rec].text.chars().collect();
        self.rec_chars[rec] = Some(c.clone());
        c
    }

    /// The records of a unit's level, in text order.
    fn unit_records(&self, u: UnitId) -> Vec<RecId> {
        self.unit_recs[u].clone()
    }

    // ------------------------------------------------------------------ reading a unit

    /// Discover file versions before interpreting any file's header or body (16.1).
    /// Only the first correctly placed header contributes a version. Includes inherit the
    /// nearest declared version, but an explicit version starts a new boundary.
    pub(crate) fn read_file_versions(&mut self) {
        let files: Vec<_> =
            (0..self.doc.levels.len()).filter(|&lv| self.doc.levels[lv].kind == LevelKind::File).collect();
        let mut declarations = HashMap::new();
        for &lv in &files {
            let u = self.level_unit[lv];
            let Some(&rec) = self.unit_recs[u].first() else { continue };
            if self.after_a_record_of_its_file(u, rec) {
                continue;
            }
            if let Some((version, offset)) = declared_record_version(&self.recs[rec].text) {
                self.units[u].version = version;
                declarations.insert(lv, self.recs[rec].loc(offset));
            }
        }
        for lv in files {
            let u = self.level_unit[lv];
            let version = self.version_of(Some(u));
            if !READ_VERSIONS.contains(&version.as_str()) {
                self.units[u].ok = false;
                let loc = declarations.remove(&lv).unwrap_or_else(|| {
                    Loc::line(
                        &self.doc.levels[lv].file,
                        self.doc.levels[lv].section_lines.map_or(1, |(start, _)| start + 1),
                    )
                });
                let reads = read_versions();
                self.sink.error(codes::VERSION_UNSUPPORTED,
                    format!("unsupported version {version}: no RNL of this file is interpreted (16.1; this implementation reads {reads})"),
                    Some(loc),
                );
            }
        }
    }

    pub(crate) fn read_unit(&mut self, u: UnitId) {
        if self.units[u].kind == UnitKind::Level && self.units[u].level.is_some_and(|lv| !self.file_ok(lv)) {
            // An unread file contributes no names, rules, aliases, DEVICE declarations or
            // packages. Supported ancestors still contribute through the host hierarchy.
            self.units[u].ok = false;
            return;
        }
        let level_kind = self.unit_level_kind(u);
        let is_file_level = matches!(level_kind, Some(LevelKind::File) | Some(LevelKind::Package));
        let pkg = self.units[u].pkg().to_string();
        for (index, rec) in self.unit_records(u).into_iter().enumerate() {
            let ast = self.parse(rec);
            if !self.read_generated && self.units[u].kind == UnitKind::Level && is_generated_record(&ast) {
                // a solver does not read the solver's records, nor what they would report (2.8, 14.8)
                continue;
            }
            let early_header = if self.follows_0_4(u) {
                let header = Rc::new(self.read_header_fields(&ast, rec, u));
                self.rec_header.insert(rec, header.clone());
                if header.languages == Languages::NONE {
                    self.units[u].other_records += 1;
                    continue;
                }
                Some(header)
            } else {
                None
            };
            let deferred =
                self.units[u].kind == UnitKind::Package && self.checking_path.is_none() && early_header.is_some();
            if deferred {
                self.pending_language_records.push(rec);
            }
            let dormant = deferred || early_header.as_ref().is_some_and(|header| !self.record_has_language(u, header));
            for note in ast.notes.iter().filter(|_| !dormant) {
                if let Some(kw) = &note.kw {
                    let k = self.kw(kw, u);
                    if k == "AS" || k == "ATTACH" {
                        continue; // `$GND @ATTACH (0, 0)`: the point is an object of AS, ATTACH (5.6)
                    }
                }
                let loc = self.recs[rec].loc(note.off);
                self.diag_in(Severity::Warning, note.code, note.message.clone(), Some(loc), &pkg);
            }
            if let Some((e, pos)) = &ast.error {
                if dormant {
                    self.units[u].other_records += 1;
                    continue;
                }
                let loc = self.recs[rec].loc(*pos);
                self.diag_in(Severity::Error, e.code, e.message.clone(), Some(loc), &pkg);
                let n = self.recs[rec].text.chars().count();
                self.note_failed(Some(rec), 0, n, false);
                self.units[u].other_records += 1;
                continue;
            }
            let header = early_header.unwrap_or_else(|| Rc::new(self.read_header_fields(&ast, rec, u)));
            self.rec_header.insert(rec, header.clone());
            // the record of the solver whose numbering does not match the level: its statements
            // that use a number are left out, as if they were not written, with one hint (5.9)
            let stale = self.numbers.stale.contains(&rec);
            if stale {
                let off = ast
                    .header
                    .as_ref()
                    .and_then(|h| h.named.iter().find(|n| n.name == "numbering"))
                    .map_or(0, |n| n.off);
                let loc = self.recs[rec].loc(off);
                self.diag_in(
                    Severity::Hint,
                    codes::NUMBERING_RECORD_STALE,
                    super::numbers::STALE_HINT,
                    Some(loc),
                    &pkg,
                );
            }
            if header.type_ == "header" {
                let modern = self.follows_0_4(u);
                let same_header =
                    self.units[u].header_records.iter().any(|r| self.rec_header[r].languages == header.languages);
                let wrong = if !is_file_level {
                    Some((codes::HEADER_RECORD_IN_SUBCIRCUIT,
                        "a header record is written at the file level, not inside a subcircuit (2.8); this one is ignored"))
                } else if (!modern && self.units[u].header_record.is_some()) || (modern && same_header) {
                    Some((
                        codes::HEADER_RECORD_SECOND,
                        if modern {
                            "a file has at most one header record without language and one per language (2.8); this one is ignored"
                        } else {
                            "a file has at most one header record (2.8); this one is ignored"
                        },
                    ))
                } else if (if modern {
                    self.units[u].other_records != 0
                        || (header.languages == Languages::ALL && !self.units[u].header_records.is_empty())
                } else {
                    index != 0
                }) || self.after_a_record_of_its_file(u, rec)
                {
                    Some((
                        codes::HEADER_RECORD_NOT_FIRST,
                        if modern {
                            "header records precede all other records, with the one without language first (2.8); this one is ignored"
                        } else {
                            "a header record is the first record of a file, and a record comes before this one (2.8); this one is ignored"
                        },
                    ))
                } else {
                    None
                };
                if let Some((code, msg)) = wrong {
                    let loc = self.recs[rec].loc(0);
                    self.diag_in(Severity::Error, code, msg, Some(loc), &pkg);
                    self.units[u].other_records += 1;
                    continue;
                }
                if header.languages == Languages::ALL {
                    self.units[u].header_record = Some(rec);
                }
                self.units[u].header_records.push(rec);
                self.read_header(u, rec, &ast, false);
                continue;
            }
            self.units[u].other_records += 1;
            // `$` aliases replace whole name tokens, in references and bindings alike (16.5)
            let mut ast_body = ast.body.clone().unwrap();
            if let Some(fu) = self.file_unit(u) {
                let table: HashMap<String, String> = self.units[fu]
                    .aliases
                    .iter()
                    .filter(|(a, _)| a.starts_with('$'))
                    .map(|(a, o)| (a.clone(), o.clone()))
                    .collect();
                if !table.is_empty() {
                    let mut b = (*ast_body).clone();
                    rename_dollar_set(&mut b, &table);
                    ast_body = Rc::new(b);
                    let mut a2 = (*ast).clone();
                    a2.body = Some(ast_body.clone());
                    self.asts[rec] = Some(Rc::new(a2));
                }
            }
            if let Some(target) = &header.scope {
                // a syntax error fails its member, as in the target's own records, and is reported
                // at its place in this file (7.4); the other members go to the target
                for m in ast_body.members.iter().filter(|m| m.error.is_some() && !dormant) {
                    let loc = self.recs[rec].loc(m.error_pos);
                    let e = m.error.as_ref().unwrap();
                    self.diag_in(Severity::Error, e.code, e.message.clone(), Some(loc), &pkg);
                    self.note_failed(Some(rec), m.start, m.end, false);
                }
                let a = self.asts[rec].clone().unwrap();
                let key = self.doc.key(target);
                let unit = &mut self.units[u];
                unit.scope_index.entry(key).or_default().push(unit.scope_records.len());
                unit.scope_records.push((rec, a, header));
                continue;
            }
            for (k, m) in ast_body.members.iter().enumerate() {
                if let Some(err) = &m.error {
                    if dormant {
                        continue;
                    }
                    let loc = self.recs[rec].loc(m.error_pos);
                    self.diag_in(Severity::Error, err.code, err.message.clone(), Some(loc), &pkg);
                    self.note_failed(Some(rec), m.start, m.end, false);
                    continue;
                }
                if stale && syntax::uses_number(m) {
                    continue;
                }
                self.units[u].members.push(UnitMember {
                    rec: Some(rec),
                    k,
                    embedded: false,
                    m: m.clone(),
                    header: header.clone(),
                });
                self.declare(u, Some(rec), k, false, m, &header);
                // USE-PACKAGE written in the level (16.2)
                if m.segments.len() == 1 {
                    if let Some(call) = &m.segments[0].call {
                        if self.kw(&call.kw, u) == "USE-PACKAGE" {
                            let seg = m.segments[0].clone();
                            self.use_decl(u, Some(rec), k, &seg, false);
                        }
                    }
                }
            }
        }
        if is_file_level
            && self.units[u].header_record.is_none()
            && level_kind == Some(LevelKind::File)
            && self.units[u].level == Some(0)
            && self.doc.dialect.name != crate::host::text::NAME
        {
            // the header record may be left out; tools write it when they save the file (16.1, TR5)
            let file = self.doc.levels[0].file.clone();
            self.sink.hint(codes::HEADER_RECORD_MISSING,
                format!("the main file has no header record: it is read in the tool's default version {DEFAULT_VERSION}, and tools add the header when they write the file (16.1)"),
                Some(Loc::line(file, 2)),
            );
        }
    }

    /// Whether a record comes before this one in the file whose file level is the unit `u`: a
    /// record of that level or of a level below it in the same file, such as a subcircuit written
    /// above it. A section of `.lib` that is read counts as a file, with a file level of its own
    /// (2.8).
    fn after_a_record_of_its_file(&self, u: UnitId, rec: RecId) -> bool {
        let Some(top) = self.units[u].level else { return false };
        let at = self.recs[rec].loc(0);
        let in_file = |mut l: LevelId| loop {
            if self.doc.levels[l].kind == LevelKind::File {
                return l == top;
            }
            match self.doc.levels[l].parent {
                Some(p) => l = p,
                None => return false,
            }
        };
        self.doc.levels.iter().enumerate().any(|(l, lv)| {
            in_file(l)
                && lv.records.iter().any(|o| {
                    let p = o.loc(0);
                    (p.line, p.col) < (at.line, at.col)
                        && !(self.follows_0_4(u)
                            && self.units[u].header_records.iter().any(|r| {
                                let accepted = self.recs[*r].loc(0);
                                accepted.file == p.file && accepted.line == p.line && accepted.col == p.col
                            }))
                })
        })
    }

    pub(crate) fn unit_level_kind(&self, u: UnitId) -> Option<LevelKind> {
        let unit = &self.units[u];
        if let Some(lv) = unit.level {
            return Some(self.doc.levels[lv].kind);
        }
        if unit.pkg_level.is_some() {
            return Some(LevelKind::Package);
        }
        None
    }

    /// Check free text only once all forward names, scope records and effective packages are
    /// known. This consults the same visibility rules as references, without evaluating them.
    pub(crate) fn check_free_words(&mut self) {
        for u in 0..self.units.len() {
            if !self.units[u].ok {
                continue;
            }
            let members: Vec<_> = self.units[u].members.iter().chain(&self.units[u].scope_members).cloned().collect();
            for m in members {
                if !self.record_has_language(u, &m.header) {
                    continue;
                }
                if let Some(rec) = m.rec {
                    self.free_word_warnings(u, rec, &m.m, &HashSet::new(), &HashSet::new());
                }
            }
        }
    }

    /// 3.3 recommended warning: a free word equal to an instance, a visible RNL name or keyword.
    fn free_word_warnings(
        &mut self,
        u: UnitId,
        rec: RecId,
        m: &Member,
        local: &HashSet<String>,
        defining: &HashSet<String>,
    ) {
        if m.error.is_some() {
            return;
        }
        let fu = self.file_unit(u);
        let aliases = fu.map(|f| self.units[f].aliases.clone()).unwrap_or_default();
        let pkg = self.units[u].pkg().to_string();
        for (word, off) in &m.free_words {
            let instance = self.units[u].level.filter(|_| self.units[u].kind == UnitKind::Level).is_some_and(|lv| {
                self.doc.levels[lv].instances.iter().any(|i| {
                    if i.case_sensitive {
                        i.name == *word
                    } else {
                        i.name.eq_ignore_ascii_case(word)
                    }
                })
            });
            let dollar = aliases.get(&format!("${word}")).and_then(|a| a.strip_prefix('$')).unwrap_or(word);
            let what = if instance {
                Some(format!("instance #{word}"))
            } else if words().is_keyword(word) || aliases.contains_key(&format!("@{word}")) {
                Some(format!("keyword @{word}"))
            } else if local.contains(word)
                || self.lookup_unit('%', word, u, defining.contains(word), None).is_some()
                || self.lookup_unit('$', dollar, u, defining.contains(dollar), None).is_some()
            {
                Some("an RNL name".to_string())
            } else {
                None
            };
            if let Some(what) = what {
                let loc = self.recs[rec].loc(*off);
                self.diag_in(
                    Severity::Warning,
                    codes::RECORD_FREE_WORD_LIKE_NAME,
                    format!("the free-text word {word} is spelled like {what}: is a prefix missing? (3.3)"),
                    Some(loc),
                    &pkg,
                );
            }
        }
        for (index, segment) in m.segments.iter().enumerate() {
            let keyword = segment.call.as_ref().map(|c| self.kw(&c.kw, u));
            // Inline package bodies have their own unit and are checked there. Its private
            // names must not leak into this enclosing lexical scope.
            if keyword.as_deref() == Some("USE-PACKAGE") {
                continue;
            }
            let mut nested = local.clone();
            let mut hidden = defining.clone();
            if matches!(keyword.as_deref(), Some("DEFINE" | "DEFINE-SYMBOL" | "DEFINE-STYLE")) {
                // A definition's own binding is not visible inside its body (6.9); an outer
                // binding with that spelling can still be visible through lookup_unit.
                for binding in m.binds.iter().filter(|b| b.after == index) {
                    nested.remove(&binding.name);
                    hidden.insert(binding.name.clone());
                }
            }
            if keyword.as_deref() == Some("DEFINE") {
                if let Some(params) = segment.call.as_ref().and_then(|c| c.params.as_ref()) {
                    for parameter in &params.named {
                        if !matches!(self.word(&parameter.name, u).as_str(), "roles" | "parameters") {
                            continue;
                        }
                        if let Value::Arr(items) = &parameter.value {
                            for item in items {
                                let name = match item {
                                    Value::Str(name) => Some(name),
                                    Value::Arr(pair) => {
                                        pair.first()
                                            .and_then(|v| if let Value::Str(name) = v { Some(name) } else { None })
                                    }
                                    _ => None,
                                };
                                if let Some(name) = name {
                                    nested.insert(name.trim_matches(['{', '}']).to_string());
                                }
                            }
                        }
                    }
                }
            }
            for arg in &segment.args {
                if let ObjExpr::Set(body) = &arg.obj {
                    let mut visible = nested.clone();
                    visible.extend(
                        body.members
                            .iter()
                            .filter(|m| m.error.is_none())
                            .flat_map(|m| m.binds.iter().map(|b| b.name.clone())),
                    );
                    for member in &body.members {
                        self.free_word_warnings(u, rec, member, &visible, &hidden);
                    }
                }
            }
        }
    }

    fn read_header_fields(&mut self, ast: &RecordAst, rec: RecId, u: UnitId) -> Header {
        let mut h = Header::default();
        let pkg = self.units[u].pkg().to_string();
        let Some(params) = &ast.header else { return h };
        if !params.positional.is_empty() {
            let loc = self.recs[rec].loc(params.start);
            self.diag_in(
                Severity::Error,
                codes::RECORD_HEADER_POSITIONAL,
                "record header fields are written by name; positional values are ignored (2.8)",
                Some(loc),
                &pkg,
            );
        }
        // `numbering` is a field of the draft 0.3 only (2.8, 5.9)
        let draft: &[crate::words::HeaderField] = if self.follows_0_4(u) {
            crate::words::draft_0_4_header_fields()
        } else if self.follows_0_3(u) {
            crate::words::draft_header_fields()
        } else {
            &[]
        };
        let (mut numbering_at, mut scope_at, mut language_value) = (None, None, None);
        for n in &params.named {
            let loc = self.recs[rec].loc(n.off);
            let fields = || words().header_fields.iter().chain(draft);
            let Some(spec) = fields().find(|f| f.name == n.name) else {
                let close = closest(&n.name, fields().map(|f| f.name.as_str()));
                let hint = close.map(|c| format!(", did you mean {c}?")).unwrap_or_default();
                self.diag_in(
                    Severity::Warning,
                    codes::RECORD_HEADER_FIELD_UNKNOWN,
                    format!("unknown record header field {} is ignored (2.8){hint}", n.name),
                    Some(loc),
                    &pkg,
                );
                continue;
            };
            let mut occurrences = params.named.iter().filter(|field| field.name == n.name);
            let first = occurrences.next().unwrap();
            if let Some(second) = occurrences.next() {
                if n.off == first.off {
                    self.diag_in(
                        Severity::Error,
                        codes::RECORD_HEADER_FIELD_TWICE,
                        format!(
                            "record header field {} is written more than once; treated as not written (2.8)",
                            n.name
                        ),
                        Some(self.recs[rec].loc(second.off)),
                        &pkg,
                    );
                }
                continue;
            }
            let Value::Str(v) = &n.value else {
                self.diag_in(
                    Severity::Error,
                    codes::RECORD_HEADER_FIELD_NOT_STRING,
                    format!("record header field {} takes a string; treated as not written (2.8)", n.name),
                    Some(loc),
                    &pkg,
                );
                continue;
            };
            if n.name == "language" {
                language_value = Some((v.clone(), loc));
                continue;
            }
            if let Some(vals) = &spec.values {
                if !vals.contains(v) {
                    self.diag_in(
                        Severity::Error,
                        codes::RECORD_HEADER_FIELD_BAD_VALUE,
                        format!("record header field {} cannot be {v:?}; treated as not written (2.8)", n.name),
                        Some(loc),
                        &pkg,
                    );
                    continue;
                }
            }
            match n.name.as_str() {
                "type" => h.type_ = v.clone(),
                "priority" => h.priority = Priority::parse(v),
                "scope" => {
                    h.scope = Some(v.clone());
                    scope_at = Some(loc);
                }
                "numbering" => {
                    if v.chars().count() != 16 || !v.chars().all(|c| c.is_ascii_hexdigit()) {
                        self.diag_in(
                            Severity::Error,
                            codes::RECORD_HEADER_NUMBERING_MALFORMED, format!("record header field numbering holds a check of 16 hexadecimal digits (5.9), not {v:?}; treated as not written (2.8)"),
                            Some(loc),
                            &pkg,
                        );
                        continue;
                    }
                    h.numbering = Some(v.clone());
                    numbering_at = Some(loc);
                }
                _ => {}
            }
        }
        if let Some((value, loc)) = language_value {
            if h.type_ == "generated" {
                self.diag_in(
                    Severity::Error,
                    codes::RECORD_HEADER_LANGUAGE_GENERATED,
                    "record header field language is not written with type=\"generated\"; treated as not written (2.8)",
                    Some(loc),
                    &pkg,
                );
            } else {
                h.languages = Languages::parse(&value).unwrap_or_else(|| {
                    let close = closest(&value, ["spice", "spectre"]).unwrap_or("spice".into());
                    self.diag_in(Severity::Warning, codes::RECORD_HEADER_LANGUAGE_UNKNOWN,
                        format!("unknown record language {value:?}; this record acts on nothing (2.8), did you mean {close}?"),
                        Some(loc), &pkg);
                    Languages::NONE
                });
            }
        }
        // the RNL of a subcircuit covers only itself: scope written inside one is an error, and the
        // record is read as one of the subcircuit's own (2.6, 2.8)
        if let (Some(loc), Some(LevelKind::Subckt)) = (scope_at, self.unit_level_kind(u)) {
            self.diag_in(
                Severity::Error,
                codes::RECORD_HEADER_SCOPE_IN_SUBCIRCUIT, "scope= is not written in a record inside a subcircuit, whose RNL covers only itself; treated as not written, so the record is the subcircuit's own (2.6, 2.8)",
                Some(loc),
                &pkg,
            );
            h.scope = None;
        }
        // numbering is written only in the record of the solver, which is in the level's own text
        let wrong = if h.type_ != "generated" {
            Some((codes::RECORD_HEADER_NUMBERING_NOT_GENERATED, "record header field numbering is written only with type=\"generated\", in the record of the solver; treated as not written (2.8, 5.9)"))
        } else if h.scope.is_some() {
            Some((codes::RECORD_HEADER_NUMBERING_WITH_SCOPE, "record header field numbering is not written in a record with scope=: the record of the solver is in the level's own text; treated as not written (2.6, 2.8, 5.9)"))
        } else {
            None
        };
        if let (Some(loc), Some((code, msg))) = (numbering_at, wrong) {
            self.diag_in(Severity::Error, code, msg, Some(loc), &pkg);
            h.numbering = None;
        }
        if self.units[u].kind == UnitKind::Package && (h.scope.is_some() || h.type_ == "generated") {
            let loc = self.recs[rec].loc(0);
            self.diag_in(
                Severity::Error,
                codes::PACKAGE_RECORD_SCOPE_OR_GENERATED,
                "a package cannot use scope= or type=\"generated\" (16.2)",
                Some(loc),
                &pkg,
            );
            h.scope = None;
            h.type_ = "authored".into();
        }
        h
    }

    /// The header record is always read in the standard form, without aliases (16.5).
    fn read_header(&mut self, u: UnitId, rec: RecId, ast: &RecordAst, complete: bool) {
        let pkg = self.units[u].pkg().to_string();
        let body = ast.body.clone().unwrap();
        let deferred = self.pending_language_records.contains(&rec);
        let dormant = !self.record_has_language(u, &self.rec_header[&rec]);
        for (k, m) in body.members.iter().enumerate() {
            if !complete {
                let kw = if m.error.is_none() && m.segments.len() == 1 && m.bind().is_none() {
                    m.segments[0].call.as_ref().map(|call| call.kw.as_str()).unwrap_or("?")
                } else {
                    "?"
                };
                // Package classification depends on the written header, even when a
                // DEVICE declaration waits for a matching language section.
                self.units[u].header_kws.push(kw.into());
            }
            let early_declaration = m.error.is_none()
                && m.segments.len() == 1
                && m.bind().is_none()
                && m.segments[0].call.as_ref().is_some_and(|call| {
                    call.kw == "USE-PACKAGE"
                        || (self.rec_header[&rec].languages == Languages::ALL
                            && matches!(call.kw.as_str(), "VERSION" | "PACKAGE" | "ALIAS" | "USE-ALIASES"))
                });
            // Package names remain visible across sections. Read those dependencies now;
            // DEVICE and other checks wait for a matching host section, if there is one.
            if (complete && early_declaration) || (!complete && (deferred || dormant) && !early_declaration) {
                continue;
            }
            if let Some(err) = &m.error {
                let loc = self.recs[rec].loc(m.error_pos);
                self.diag_in(Severity::Error, err.code, err.message.clone(), Some(loc), &pkg);
                self.note_failed(Some(rec), m.start, m.end, false);
                continue;
            }
            if m.segments.len() != 1 || m.segments[0].call.is_none() || m.bind().is_some() {
                let loc = self.recs[rec].loc(m.start);
                self.diag_in(
                    Severity::Error,
                    codes::HEADER_MEMBER_NOT_STATEMENT,
                    "a header record holds only VERSION, PACKAGE, USE-PACKAGE, USE-ALIASES, ALIAS and DEVICE (2.8)",
                    Some(loc),
                    &pkg,
                );
                continue;
            }
            let seg = m.segments[0].clone();
            let call = seg.call.clone().unwrap();
            let kw = call.kw.clone();
            let loc = self.recs[rec].loc(call.start);
            if kw == "SOLVER-CONFIG" && self.follows_0_4(u) {
                self.header_solver_config(u, rec, &seg);
                continue;
            }
            if self.rec_header[&rec].languages != Languages::ALL && !matches!(kw.as_str(), "USE-PACKAGE" | "DEVICE") {
                self.diag_in(Severity::Error, codes::HEADER_KEYWORD_NOT_ALLOWED,
                    format!("@{kw} cannot be written in a header record with language; only USE-PACKAGE and DEVICE are allowed (2.8)"),
                    Some(loc), &pkg);
                continue;
            }
            if !HEADER_KW.contains(&kw.as_str()) && kw != "USE-PACKAGE" {
                self.diag_in(
                    Severity::Error,
                    codes::HEADER_KEYWORD_NOT_ALLOWED,
                    format!("@{kw} cannot be written in a header record (2.8)"),
                    Some(loc),
                    &pkg,
                );
                continue;
            }
            if kw == "USE-PACKAGE" {
                self.use_decl(u, Some(rec), k, &seg, true);
                continue;
            }
            let Some(p) = self.header_params(&kw, call.params.as_ref(), u, &loc, true) else { continue };
            if !seg.args.is_empty() {
                self.diag_in(
                    Severity::Error,
                    codes::HEADER_STATEMENT_TAKES_NO_OBJECTS,
                    format!("@{kw} takes no objects (2.8)"),
                    Some(loc),
                    &pkg,
                );
                continue;
            }
            match kw.as_str() {
                // a file declares its version once (16.1): the first VERSION of its header
                // record stands, as the scan of the package directories takes it for a package file
                "VERSION" if self.units[u].header_kws.iter().filter(|k| *k == "VERSION").count() > 1 => {
                    self.diag_in(Severity::Error, codes::VERSION_DECLARED_TWICE, SECOND_VERSION, Some(loc), &pkg);
                }
                "VERSION" => {
                    let Some(Value::Str(v)) = p.get("version") else {
                        self.diag_in(
                            Severity::Error,
                            codes::VERSION_NOT_STRING,
                            "@VERSION takes a string (16.1)",
                            Some(loc),
                            &pkg,
                        );
                        continue;
                    };
                    self.units[u].version = v.clone();
                    if version_key(v).0 < 0 || !READ_VERSIONS.contains(&v.as_str()) {
                        let reads = read_versions();
                        self.diag_in(
                            Severity::Error,
                            codes::VERSION_UNSUPPORTED, format!("unsupported version {v}: no RNL of this file is interpreted (16.1; this implementation reads {reads})"),
                            Some(loc),
                            &pkg,
                        );
                        self.units[u].ok = false;
                    }
                }
                "PACKAGE" => {
                    if self.units[u].kind != UnitKind::Package {
                        let message = "@PACKAGE is written only in the header record of a package file (2.7)";
                        self.sink.error(codes::PACKAGE_DECL_OUTSIDE_PACKAGE_FILE, message, Some(loc));
                        continue;
                    }
                    if self.follows_0_4(u) && self.units[u].header_kws.iter().filter(|k| *k == "PACKAGE").count() > 1 {
                        self.diag_in(
                            Severity::Error,
                            codes::PACKAGE_DECLARED_TWICE,
                            "a package file declares PACKAGE only once; the first declaration stands (2.7, 16.2)",
                            Some(loc),
                            &pkg,
                        );
                        continue;
                    }
                    let name = str_val(p.get("name")).unwrap_or_default();
                    let version = str_val(p.get("version")).unwrap_or_default();
                    if self.follows_0_4(u) && !valid_package_version(&version) {
                        self.diag_in(Severity::Error, codes::PACKAGE_VERSION_MALFORMED,
                            format!("@PACKAGE version={version:?} must be two unsigned decimal integers separated by a dot; this file is not a package (16.1, 16.2)"), Some(loc), &pkg);
                        self.units[u].ok = false;
                        continue;
                    }
                    self.units[u].pkg_name = name.clone();
                    self.units[u].pkg_version = version.clone();
                    // a package checked on its own is the one that its references, and the
                    // standard package's, find by its name and version; one that a USE-PACKAGE
                    // or a reference found is there already (16.2)
                    if !name.is_empty() {
                        let language = self.version_of(Some(u));
                        self.packages.entry(package_cache_key(name, version, language == "0.4")).or_insert(u);
                    }
                }
                "USE-ALIASES" => self.use_aliases(u, &p, &loc),
                "ALIAS" => {
                    let (Some(alias), Some(orig)) = (str_val(p.get("alias")), str_val(p.get("original"))) else {
                        self.diag_in(
                            Severity::Error,
                            codes::ALIAS_TAKES_STRINGS,
                            "@ALIAS takes two strings: the alias and the original; definition= is a string too (16.5)",
                            Some(loc),
                            &pkg,
                        );
                        continue;
                    };
                    let dfn = match p.get("definition") {
                        None => None,
                        Some(Value::Str(s)) => Some(s.clone()),
                        Some(_) => {
                            self.diag_in(Severity::Error, codes::ALIAS_TAKES_STRINGS, "@ALIAS takes two strings: the alias and the original; definition= is a string too (16.5)", Some(loc), &pkg);
                            continue;
                        }
                    };
                    // a package used by another file is read in its own version, but whether an original is a
                    // standard word is a question of the version of the file that uses it (16.5)
                    let defer = self.units[u].kind == UnitKind::Package && !self.checking_package_unit(u);
                    let version = self.version_of(Some(u));
                    if let Some(decl) = self.check_alias(&alias, &orig, dfn.as_deref(), &loc, &pkg, !defer, &version) {
                        let pending = defer && decl.2.is_none() && !decl.1.starts_with('$');
                        self.add_alias(u, decl.clone(), &loc, "");
                        if pending {
                            self.units[u].alias_pending.push((decl.0, decl.1, loc.clone()));
                        }
                    }
                }
                "DEVICE" => {
                    let modern = self.follows_0_4(u);
                    let name = str_val(p.get("name"));
                    let model = str_val(p.get("model"));
                    let mtype = str_val(p.get("type")).map(|s| s.to_lowercase());
                    let terms = match p.get("terminals") {
                        Some(Value::Arr(items)) => Some(items.clone()),
                        _ => None,
                    };
                    if (modern && ((name.is_none() && model.is_none()) || (terms.is_none() && mtype.is_none())))
                        || (!modern && (name.is_none() || terms.is_none()))
                    {
                        let message = if modern {
                            "@DEVICE needs name or model, and terminals or type (6.12)"
                        } else {
                            "@DEVICE takes name and terminals (6.12)"
                        };
                        self.diag_in(Severity::Error, codes::DEVICE_MISSING_PARAMS, message, Some(loc), &pkg);
                        continue;
                    }
                    if terms.as_ref().is_some_and(|a| a.iter().any(|t| !matches!(t, Value::Str(_)))) {
                        self.diag_in(
                            Severity::Error,
                            codes::DEVICE_TERMINALS_NOT_NAMES,
                            "the terminals of @DEVICE must be an array of names (strings) (6.12)",
                            Some(loc),
                            &pkg,
                        );
                        continue;
                    }
                    if modern {
                        let cs = self.units[u].kind != UnitKind::Package && self.doc.rules(&loc.file).case_sensitive;
                        let mut seen = HashSet::new();
                        let invalid = terms.as_ref().is_some_and(|a| {
                            a.iter().any(|v| {
                                let Value::Str(n) = v else { return false };
                                let key = if cs { n.clone() } else { n.to_lowercase() };
                                (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) || !seen.insert(key)
                            })
                        });
                        if invalid {
                            self.diag_in(Severity::Error, codes::DEVICE_TERMINAL_NAMES_INVALID,
                                "the terminal names of @DEVICE must be distinct by the host's case rules and cannot be made only of digits (6.12)", Some(loc), &pkg);
                            continue;
                        }
                    }
                    let terminals = terms.map(|a| {
                        a.iter()
                            .filter_map(|x| match x {
                                Value::Str(s) => Some(Lit::Str(super::obj::WStr::new(s, None))),
                                _ => None,
                            })
                            .collect()
                    });
                    self.units[u].devices.push(DeviceDecl {
                        name: name.unwrap_or_else(|| "*".into()),
                        model,
                        terminals,
                        mtype,
                        loc,
                        package: pkg.clone(),
                        languages: self.rec_header[&rec].languages,
                    });
                }
                _ => {}
            }
        }
        self.finish_aliases(u);
    }

    /// The graph is now known, so qualified package records can be checked in their
    /// actual nominal scope. Inactive records retain declarations but no failed spans.
    pub(crate) fn check_language_records(&mut self) {
        for rec in std::mem::take(&mut self.pending_language_records) {
            let u = self.rec_unit[rec];
            if !self.record_has_language(u, &self.rec_header[&rec]) {
                continue;
            }
            let ast = self.parse(rec);
            let pkg = self.units[u].pkg().to_string();
            for note in &ast.notes {
                if note.kw.as_ref().is_some_and(|kw| matches!(self.kw(kw, u).as_str(), "AS" | "ATTACH")) {
                    continue;
                }
                self.diag_in(
                    Severity::Warning,
                    note.code,
                    note.message.clone(),
                    Some(self.recs[rec].loc(note.off)),
                    &pkg,
                );
            }
            if let Some((error, off)) = &ast.error {
                self.diag_in(Severity::Error, error.code, error.message.clone(), Some(self.recs[rec].loc(*off)), &pkg);
                self.note_failed(Some(rec), 0, self.recs[rec].text.chars().count(), false);
            } else if self.units[u].header_records.contains(&rec) {
                self.read_header(u, rec, &ast, true);
            } else if let Some(body) = &ast.body {
                for member in &body.members {
                    if let Some(error) = &member.error {
                        self.diag_in(
                            Severity::Error,
                            error.code,
                            error.message.clone(),
                            Some(self.recs[rec].loc(member.error_pos)),
                            &pkg,
                        );
                        self.note_failed(Some(rec), member.start, member.end, false);
                    }
                }
            }
        }
    }

    /// Is this package the document being checked on its own (2.7)?
    pub(crate) fn checking_package_unit(&self, u: UnitId) -> bool {
        match (self.units[u].pkg_level, &self.checking_path) {
            (Some(pl), Some(p)) => self.pkg_levels[pl].path == *p,
            _ => false,
        }
    }

    /// Parameters of a statement read when the file is read (header statements, USE-PACKAGE), as
    /// `params_by_name` reads them, each error reported at `loc`.
    pub(crate) fn header_params(
        &mut self,
        kw: &str,
        p: Option<&Params>,
        u: UnitId,
        loc: &Loc,
        std_form: bool,
    ) -> Option<HashMap<String, Value>> {
        match params_by_name_in(kw, p, std_form, |n| self.word(n, u), &self.version_of(Some(u))) {
            Ok(out) => Some(out),
            Err((code, msg)) => {
                let pkg = self.units[u].pkg().to_string();
                self.diag_in(Severity::Error, code, msg, Some(loc.clone()), &pkg);
                None
            }
        }
    }

    // ------------------------------------------------------------------ aliases (16.5)

    /// Check one ALIAS declaration on its own: (alias, original, definition), or None.
    #[allow(clippy::too_many_arguments)]
    fn check_alias(
        &mut self,
        alias: &str,
        orig: &str,
        definition: Option<&str>,
        loc: &Loc,
        pkg: &str,
        check_original: bool,
        version: &str,
    ) -> Option<(String, String, Option<String>)> {
        let bad = |s: &mut Self, code: Code, msg: String| {
            s.diag_in(Severity::Error, code, msg, Some(loc.clone()), pkg);
            None
        };
        if let Some(def) = definition {
            // a name inside a definition: role, parameter or local name, used where that definition is applied
            if !def.starts_with('$') || def.chars().count() < 2 {
                return bad(self, codes::ALIAS_DEFINITION_NOT_DOLLAR_NAME, format!("definition= names an ordinary definition by its `$` name, such as \"$DIFFPAIR\", not {def:?} (16.5)"));
            }
            let first_digit = alias.chars().next().is_some_and(|c| c.is_ascii_digit());
            if alias.is_empty() || !alias.chars().all(syntax::is_name_char) || first_digit {
                return bad(self, codes::ALIAS_SCOPED_MALFORMED, format!("alias {alias:?} is not well formed: an alias of a name in a definition must be writable unquoted in parentheses (16.5)"));
            }
            if orig.is_empty() || "@$%?#&".contains(orig.chars().next().unwrap()) {
                return bad(self, codes::ALIAS_SCOPED_ORIGINAL_PREFIXED, format!("with definition=, the original is a role, parameter or local name without a prefix, not {orig:?} (16.5)"));
            }
            return Some((alias.into(), orig.into(), Some(def.into())));
        }
        if alias.starts_with('$') || orig.starts_with('$') {
            if !(alias.starts_with('$') && orig.starts_with('$'))
                || alias.chars().count() < 2
                || orig.chars().count() < 2
            {
                return bad(self, codes::ALIAS_DOLLAR_MISMATCH, "the alias and the original of a name both carry `$`, such as @ALIAS(\"$DP\", \"$DIFFERENTIAL_PAIR\") (16.5)".into());
            }
            if alias == orig {
                return bad(
                    self,
                    codes::ALIAS_SAME_AS_ORIGINAL,
                    format!("the alias {alias} is the same as the original (16.5)"),
                );
            }
            return Some((alias.into(), orig.into(), None));
        }
        if orig.starts_with('@') != alias.starts_with('@') {
            return bad(
                self,
                codes::ALIAS_KEYWORD_MISMATCH,
                "the original and the alias of a keyword both carry `@`; other words carry neither (16.5)".into(),
            );
        }
        if let Some(base) = orig.strip_prefix('@') {
            if matches!(base, "RNL" | "VERSION" | "ALIAS") {
                return bad(
                    self,
                    codes::ALIAS_OF_OUTER_FORM_KEYWORD,
                    format!("{orig} belongs to the outer form and cannot have an alias (16.5)"),
                );
            }
            if base == "USE-ALIASES" {
                return bad(
                    self,
                    codes::ALIAS_OF_USE_ALIASES,
                    "@USE-ALIASES cannot have an alias: the aliases are not known before it is read (16.5)".into(),
                );
            }
            if check_original && !standard_original_in(orig, version) {
                return bad(
                    self,
                    codes::ALIAS_ORIGINAL_NOT_STANDARD,
                    format!("the original {orig} is not a standard keyword (16.5)"),
                );
            }
            let a = &alias[1..];
            if (words().is_keyword(a) && (a != "TABLE" || keeps_0_4(version)))
                || a.is_empty()
                || !a.chars().all(syntax::is_word_char)
            {
                return bad(
                    self,
                    codes::ALIAS_MALFORMED_OR_KEYWORD,
                    format!("the alias {alias} is not well formed or is the name of a standard keyword (16.5)"),
                );
            }
        } else {
            if check_original && !standard_original_in(orig, version) {
                return bad(
                    self,
                    codes::ALIAS_ORIGINAL_NOT_STANDARD,
                    format!("the original {orig} is not a standard word (16.5)"),
                );
            }
            // a word that 0.3 adds, `numbering`, is a standard word in a file of 0.3 (16.1): a file
            // of 0.2 raised to 0.3 with such an alias, with a warning, has it in error here
            if words().all_words_in(version).contains(alias) || added_word(alias, version) {
                return bad(
                    self,
                    codes::ALIAS_IS_STANDARD_WORD,
                    format!("the alias {alias} is the name of a standard word (16.5)"),
                );
            }
        }
        Some((alias.into(), orig.into(), None))
    }

    /// Add a checked declaration to a file's aliases; together they keep one original per alias in
    /// its scope (16.5). `source` names the alias package it comes from.
    fn add_alias(&mut self, u: UnitId, decl: (String, String, Option<String>), loc: &Loc, source: &str) {
        let (alias, orig, definition) = decl;
        let package = if self.follows_0_4(u) { self.units[u].pkg().to_string() } else { String::new() };
        let wher = if source.is_empty() { String::new() } else { format!("in alias package {source}, ") };
        if let Some(def) = definition {
            self.units[u].scoped_raw.push((def.clone(), alias.clone(), orig.clone(), loc.clone(), source.to_string()));
            self.units[u].alias_decls.push(AliasDecl {
                alias,
                original: orig,
                definition: Some(def),
                loc: loc.clone(),
            });
            return;
        }
        if let Some(old) = self.units[u].aliases.get(&alias).cloned() {
            if old != orig {
                self.diag_in(
                    Severity::Error,
                    codes::ALIAS_CONFLICT,
                    format!("{wher}the alias {alias} is already declared for {old} and cannot also be {orig} (16.5)"),
                    Some(loc.clone()),
                    &package,
                );
            }
            return;
        }
        if alias.starts_with('$') {
            if self.units[u].aliases.contains_key(&orig) {
                self.diag_in(
                    Severity::Error,
                    codes::ALIAS_ORIGINAL_IS_ALIAS,
                    format!("{wher}the alias {orig} cannot also be an original (16.5)"),
                    Some(loc.clone()),
                    &package,
                );
                return;
            }
            if self.units[u].aliases.values().any(|o| *o == alias) {
                self.diag_in(
                    Severity::Error,
                    codes::ALIAS_IS_ORIGINAL,
                    format!("{wher}{alias} is already the original of another alias and cannot be an alias (16.5)"),
                    Some(loc.clone()),
                    &package,
                );
                return;
            }
        }
        self.units[u].aliases.insert(alias.clone(), orig.clone());
        self.units[u].alias_decls.push(AliasDecl { alias, original: orig, definition: None, loc: loc.clone() });
    }

    /// Definition-scoped aliases by the definition's `$` name, read through the `$` aliases (16.5, D17).
    fn finish_aliases(&mut self, u: UnitId) {
        let package = if self.follows_0_4(u) { self.units[u].pkg().to_string() } else { String::new() };
        self.units[u].scoped.clear();
        let mut prio: Vec<String> = vec!["priority".into()];
        prio.extend(self.units[u].aliases.iter().filter(|(_, o)| *o == "priority").map(|(a, _)| a.clone()));
        let raw = self.units[u].scoped_raw.clone();
        for (definition, alias, orig, loc, source) in raw {
            let wher = if source.is_empty() { String::new() } else { format!("in alias package {source}, ") };
            let key = self.units[u].aliases.get(&definition).cloned().unwrap_or(definition);
            if prio.contains(&alias) {
                // the standard parameter of AS, ATTACH (14.8) is there for every definition
                self.diag_in(Severity::Error, codes::ALIAS_SCOPED_LIKE_PRIORITY,
                    format!("{wher}the alias {alias} is spelled like the standard parameter priority of AS, ATTACH or its alias; its meaning is not unique (16.5)"),
                    Some(loc),
                    &package,
                );
                continue;
            }
            let table = self.units[u].scoped.entry(key.clone()).or_default();
            if let Some((_, (o, _))) = table.iter().find(|(a, _)| *a == alias) {
                if *o != orig {
                    let o = o.clone();
                    self.diag_in(Severity::Error, codes::ALIAS_SCOPED_CONFLICT, format!("{wher}the alias {alias} in {key} is already declared for {o} and cannot also be {orig} (16.5)"), Some(loc), &package);
                }
                continue;
            }
            table.push((alias, (orig, loc)));
        }
    }

    /// The definition-scoped aliases for a definition in the file of unit `u` (16.5, D17). An alias
    /// spelled like a role, parameter or local name the definition has is an error and is left out.
    pub(crate) fn scoped_for(&mut self, u: UnitId, dfn: super::obj::ObjId) -> Vec<(String, String)> {
        let Some(fu) = self.file_unit(u) else { return Vec::new() };
        let name = self.objs[dfn.0 as usize].name.clone();
        if !name.starts_with('$') {
            return Vec::new();
        }
        if let Some(v) = self.scoped_memo.get(&(fu, dfn)) {
            return v.clone();
        }
        let out = self.checked_scoped_aliases(fu, dfn, &name);
        self.scoped_memo.insert((fu, dfn), out.clone());
        out
    }

    fn checked_scoped_aliases(&mut self, fu: UnitId, dfn: super::obj::ObjId, name: &str) -> Vec<(String, String)> {
        let Some(table) = self.units[fu].scoped.get(name).cloned() else { return Vec::new() };
        let d = self.def(dfn).clone();
        let mut taken: Vec<String> = d.roles.iter().map(|r| r.name.clone()).collect();
        taken.extend(d.params.iter().map(|(n, _)| n.clone()));
        taken.extend(d.local_names());
        let mut out = Vec::new();
        for (alias, (orig, loc)) in table {
            if taken.contains(&alias) {
                let package = if self.follows_0_4(fu) { self.units[fu].pkg().to_string() } else { String::new() };
                self.diag_in(
                    Severity::Error,
                    codes::ALIAS_SCOPED_LIKE_OWN_NAME,
                    format!("the alias {alias} is spelled like a role, parameter or local name of the definition {name} itself; its meaning is not unique and the alias is not used (16.5)"),
                    Some(loc),
                    &package,
                );
                continue;
            }
            out.push((alias, orig));
        }
        out
    }

    /// A declaration must not hide an own name even when the definition is never applied
    /// (16.5). Inspect values that evaluation already produced; a missing definition or
    /// original is not looked up eagerly, and this check never creates usages.
    pub(crate) fn check_unused_scoped_aliases(&mut self) {
        for u in 0..self.units.len() {
            if !self.units[u].ok || !self.follows_0_4(u) {
                continue;
            }
            let Some(fu) = self.file_unit(u) else { continue };
            let mut names: Vec<_> = self.units[fu].scoped.keys().cloned().collect();
            names.sort();
            let scene = self.home_scene(u);
            for name in names {
                let Some(bare) = name.strip_prefix('$') else { continue };
                let Some(binding) = self.lookup_unit('$', bare, u, false, scene) else { continue };
                let key = binding.key();
                let home = if self.places.rules.contains(&key) { scene } else { self.home_scene(binding.owner) };
                let Some(value) = self.evaluated_binding(&binding, home) else { continue };
                if matches!(self.objs[value.0 as usize].data, super::obj::ObjData::Definition(_)) {
                    self.checked_scoped_aliases(fu, value, &name);
                }
            }
        }
    }

    /// `@USE-ALIASES` in a header: load an alias package and add its aliases to this file's (16.5).
    fn use_aliases(&mut self, u: UnitId, p: &HashMap<String, Value>, loc: &Loc) {
        let pkg = self.units[u].pkg().to_string();
        let Some(name) = str_val(p.get("name")) else {
            self.diag_in(
                Severity::Error,
                codes::USE_ALIASES_MISSING_NAME,
                "@USE-ALIASES takes name (16.5)",
                Some(loc.clone()),
                &pkg,
            );
            return;
        };
        let decl = UseDecl {
            name: name.clone(),
            version: str_val(p.get("version")),
            path: str_val(p.get("path")),
            body: None,
            solver_config: true,
            unit: u,
            in_header: true,
            languages: Languages::ALL,
            pos: (0, 0),
            rec: None,
            offset: 0,
            pkg: None,
        };
        let base = self.file_dir(u);
        let Some(pk) = self.resolve_package(&decl, loc.clone(), Some(base), "USE-ALIASES") else { return };
        if !self.units[pk].ok {
            return;
        }
        if !is_alias_package(&self.units[pk]) {
            self.diag_in(
                Severity::Error,
                codes::USE_ALIASES_NOT_ALIAS_PACKAGE, format!("package {name} is not an alias package: its header has only @VERSION, @PACKAGE and @ALIAS and there is no other record; this USE-ALIASES fails (16.5)"),
                Some(loc.clone()),
                &pkg,
            );
            return;
        }
        let version = self.version_of(Some(u));
        let decls = self.units[pk].alias_decls.clone();
        let level = self.units[u].level;
        for d in decls {
            if d.definition.is_none() && !d.original.starts_with('$') && !standard_original_in(&d.original, &version) {
                let what = if d.original.starts_with('@') { "standard keyword" } else { "standard word" };
                self.diag_in(
                    Severity::Error,
                    codes::USE_ALIASES_ORIGINAL_NOT_IN_VERSION, format!(
                        "in alias package {name}, the original {} of the alias {} is not a {what} in this file's language version {version}; the alias is not used (16.5)",
                        d.original, d.alias
                    ),
                    Some(loc.clone()),
                    &pkg,
                );
                self.units[u].alias_rejected.entry(d.alias.clone()).or_insert(name.clone());
                continue;
            }
            // the package is read in its own version, and the alias is checked in it; but a word
            // that the version of this file adds is a standard word here, which no alias may be, as
            // for an alias declared in this file (16.1, 16.5): a file of 0.2 raised to 0.3 has it
            // in error, as the raise warned
            if d.definition.is_none() && added_word(&d.alias, &version) {
                self.diag_in(
                    Severity::Error,
                    codes::USE_ALIASES_ALIAS_IS_ADDED_WORD, format!(
                        "in alias package {name}, the alias {} is the name of a standard word in this file's language version {version}; the alias is not used (16.5)",
                        d.alias
                    ),
                    Some(loc.clone()),
                    &pkg,
                );
                continue;
            }
            if let Some(level) = level {
                let imported = super::ImportedAlias {
                    level,
                    alias: d.alias.clone(),
                    original: d.original.clone(),
                    package: name.clone(),
                    loc: loc.clone(),
                };
                if !self.units[u].imported_aliases.contains(&imported) {
                    self.units[u].imported_aliases.push(imported);
                }
            }
            self.add_alias(u, (d.alias, d.original, d.definition), loc, &name);
        }
    }

    /// The standard keyword for a written keyword, through the file's aliases (16.5).
    pub(crate) fn kw(&self, written: &str, u: UnitId) -> String {
        if words().is_keyword(written) && (written != "TABLE" || self.follows_0_4(u)) {
            return written.to_string();
        }
        if let Some(orig) = self.alias(u, &format!("@{written}")) {
            return orig[1..].to_string();
        }
        written.to_string()
    }

    /// The standard word for a written word, through the file's aliases (16.5).
    pub(crate) fn word(&self, written: &str, u: UnitId) -> String {
        self.alias(u, written).unwrap_or_else(|| written.to_string())
    }

    fn alias(&self, u: UnitId, written: &str) -> Option<String> {
        let fu = self.file_unit(u)?;
        self.units[fu].aliases.get(written).cloned()
    }

    /// The unit of the file whose text a unit is (aliases are per file, 16.5).
    pub(crate) fn file_unit(&self, u: UnitId) -> Option<UnitId> {
        let unit = &self.units[u];
        if unit.kind == UnitKind::Package {
            return Some(unit.alias_from.unwrap_or(u));
        }
        let mut lv = unit.level;
        while let Some(l) = lv {
            if self.doc.levels[l].kind == LevelKind::File {
                return Some(self.level_unit[l]);
            }
            lv = self.doc.levels[l].parent;
        }
        None
    }

    /// The directory of the file a unit's text is in (path= of USE-PACKAGE and USE-ALIASES, 16.2, 16.5).
    pub(crate) fn file_dir(&self, u: UnitId) -> String {
        let unit = &self.units[u];
        if let Some(pl) = unit.pkg_level {
            return dirname(&self.pkg_levels[pl].path);
        }
        let mut lv = unit.level;
        while let Some(l) = lv {
            let level = &self.doc.levels[l];
            if level.kind == LevelKind::File {
                return dirname(&level.path);
            }
            lv = level.parent;
        }
        dirname(&self.doc.levels[0].path)
    }

    /// The language version a unit's file is read in (16.1): what its header record declares; for
    /// an included file without one, the version of the file that includes it; for the main file
    /// and a package file without one, the tool's default.
    pub(crate) fn version_of(&self, u: Option<UnitId>) -> String {
        let mut fu = u.and_then(|u| self.file_unit(u));
        while let Some(f) = fu {
            if !self.units[f].version.is_empty() {
                return self.units[f].version.clone();
            }
            let above = self.units[f].level.and_then(|l| self.doc.levels[l].parent);
            fu = above.and_then(|p| self.file_unit(self.level_unit[p]));
        }
        DEFAULT_VERSION.to_string()
    }

    /// Does a unit follow 0.3? When the file the unit's text is in is read in 0.3 or a later version
    /// ([`Self::version_of`], [`keeps_0_3`]): every rule that 0.3 changes applies only there, so a
    /// file of 0.2 reads as it did before 0.3 was read.
    pub fn follows_0_3(&self, u: UnitId) -> bool {
        keeps_0_3(&self.version_of(Some(u)))
    }

    /// Does a unit follow 0.4? When the file the unit's text is in is read in 0.4 or a later version
    /// ([`keeps_0_4`]): every rule that 0.4 changes applies only there, so a file of 0.2 or 0.3 reads
    /// as it did before 0.4 was read.
    pub fn follows_0_4(&self, u: UnitId) -> bool {
        keeps_0_4(&self.version_of(Some(u)))
    }

    /// Does the text a statement is written in follow the draft 0.3? That is the file of its
    /// record: for a record written into a subcircuit with scope=, the file that writes it (2.6);
    /// for a statement of a definition body, the file of the definition (6.9).
    pub fn statement_follows_0_3(&self, ctx: &super::ctx::Ctx) -> bool {
        self.follows_0_3(ctx.rec.map_or(ctx.unit, |r| self.rec_unit[r]))
    }

    /// A USE-PACKAGE written in a level or in a file's header record (16.2, 16.3), or, in the
    /// header record of a package file, a reference to another package (16.2).
    fn use_decl(&mut self, u: UnitId, rec: Option<RecId>, k: usize, seg: &syntax::Segment, in_header: bool) {
        let call = seg.call.as_ref().unwrap();
        let loc = self.rec_loc(rec, call.start).unwrap();
        let pkg = self.units[u].pkg().to_string();
        let reference = self.units[u].kind == UnitKind::Package;
        if reference && !in_header {
            self.diag_in(
                Severity::Error,
                codes::USE_PACKAGE_IN_PACKAGE_RECORDS, "USE-PACKAGE is not written in the records of a package other than its header record: a package file references other packages in its header record (16.2)",
                Some(loc),
                &pkg,
            );
            return;
        }
        let Some(p) = self.header_params("USE-PACKAGE", call.params.as_ref(), u, &loc, in_header) else { return };
        let Some(name) = str_val(p.get("name")) else {
            self.diag_in(
                Severity::Error,
                codes::USE_PACKAGE_MISSING_NAME,
                "@USE-PACKAGE takes name (16.2)",
                Some(loc),
                &pkg,
            );
            return;
        };
        let mut body = None;
        if !seg.args.is_empty() {
            if reference {
                // inline content has no version and cannot take part in choosing one (16.2)
                self.diag_in(
                    Severity::Error,
                    codes::PACKAGE_REFERENCE_WITH_BODY, "a reference to another package in the header record of a package file writes only name, version and path, without a body: this reference fails (16.2)",
                    Some(loc),
                    &pkg,
                );
                return;
            }
            match (seg.args.len(), &seg.args[0].obj) {
                (1, ObjExpr::Set(s)) => body = Some(s.clone()),
                _ => {
                    self.sink.error(
                        codes::USE_PACKAGE_BODY_NOT_ONE_SET,
                        "@USE-PACKAGE takes one set as its body (16.2)",
                        Some(loc),
                    );
                    return;
                }
            }
        }
        let mut decl = UseDecl {
            name: name.clone(),
            version: str_val(p.get("version")),
            path: str_val(p.get("path")),
            body: body.clone(),
            solver_config: rec.and_then(|r| self.rec_header.get(&r)).is_none_or(|h| h.languages == Languages::ALL),
            unit: u,
            in_header,
            languages: rec.and_then(|r| self.rec_header.get(&r)).map_or(Languages::ALL, |h| h.languages),
            pos: (rec.unwrap_or(0), k),
            rec,
            offset: call.start,
            pkg: None,
        };
        if self.follows_0_4(self.use_writer(&decl))
            && decl.version.as_deref().is_some_and(|v| !valid_package_version(v))
        {
            decl.pkg =
                self.resolve_package(&decl, loc.clone(), None, if reference { "reference" } else { "USE-PACKAGE" });
            if reference {
                self.units[u].refs.push(decl);
            } else {
                self.units[u].uses.push(decl);
            }
            return;
        }
        if self.follows_0_4(u) && in_header {
            let previous = if reference { &self.units[u].refs } else { &self.units[u].uses };
            let previous = previous.iter().position(|old| old.in_header && old.name == name && old.rec != rec);
            if let Some(index) = previous {
                let old = if reference { self.units[u].refs[index].clone() } else { self.units[u].uses[index].clone() };
                let body_tokens = |decl: &UseDecl| {
                    decl.body.as_ref().and_then(|body| {
                        decl.rec.map(|rec| {
                            let chars: Vec<char> = self.recs[rec].text.chars().collect();
                            syntax::statement_tokens_features(&chars, body.start, body.end, true)
                        })
                    })
                };
                let same_body = body_tokens(&old) == body_tokens(&decl);
                let old_version = old.version.clone().or_else(|| old.pkg.map(|p| self.units[p].pkg_version.clone()));
                let new_version = if decl.version.is_some() || decl.body.is_some() {
                    decl.version.clone()
                } else if old.path == decl.path
                    && (old.path.is_some() || old.version == decl.version)
                    && old.pkg.is_some()
                {
                    old_version.clone()
                } else {
                    decl.pkg = self.resolve_package(
                        &decl,
                        loc.clone(),
                        None,
                        if reference { "reference" } else { "USE-PACKAGE" },
                    );
                    let Some(p) = decl.pkg else { return };
                    Some(self.units[p].pkg_version.clone())
                };
                if (old_version
                    .as_ref()
                    .zip(new_version.as_ref())
                    .is_some_and(|(a, b)| !package_version_cmp(a, b, true).is_eq()))
                    || !same_body
                {
                    self.diag_in(Severity::Error, codes::HEADER_PACKAGE_LANGUAGE_CONFLICT,
                        format!("package {name} has different versions or inline content in header records; this language-qualified reference fails (2.8)"),
                        Some(loc), &pkg);
                    return;
                } else if old.pkg.is_some() {
                    let declarations = if reference { &mut self.units[u].refs } else { &mut self.units[u].uses };
                    declarations[index].languages = old.languages.union(decl.languages);
                    declarations[index].solver_config = old.solver_config || decl.solver_config;
                    return;
                }
            }
        }
        let what = if reference { "reference" } else { "USE-PACKAGE" };
        if decl.pkg.is_none() {
            decl.pkg = self.resolve_package(&decl, loc.clone(), None, what);
        }
        // a package of the series 0.2 that the series 0.3 merged into another: found among the
        // series 0.2's, or written inline, it brings in nothing of the series 0.3 (16.3); one not
        // found says so in its error
        if decl.pkg.is_some() {
            if let Some(merged) = self.merged_in_0_3(&decl) {
                self.diag_in(
                    Severity::Warning,
                    codes::PACKAGE_MERGED_IN_0_3,
                    format!("{merged} (16.3)"),
                    Some(loc.clone()),
                    &pkg,
                );
            }
        }
        if let (Some(pk), None) = (decl.pkg, &body) {
            if is_alias_package(&self.units[pk]) && self.units[pk].header_kws.iter().any(|k| k == "ALIAS") {
                self.diag_in(
                    Severity::Error,
                    codes::USE_PACKAGE_ALIAS_PACKAGE, format!("package {name} is an alias package; load it with @USE-ALIASES(name=\"{name}\") in the header record. This {what} fails (16.5)"),
                    Some(loc),
                    &pkg,
                );
                decl.pkg = None;
            }
        }
        if reference {
            self.units[u].refs.push(decl);
        } else {
            self.units[u].uses.push(decl);
        }
    }

    // ------------------------------------------------------------------ packages (16.2)

    pub(crate) fn scan_packages(&mut self, name: &str) {
        if self.pkg_files.contains_key(name) {
            return;
        }
        let mut out = Vec::new();
        for f in self.provider.package_candidates(name) {
            let header = if let Some(header) = self.pkg_headers.get(&f) {
                header.clone()
            } else {
                let text = self.provider.read(&f).unwrap_or_default();
                let header = scan_package_decl(&text);
                let language = scan_language_decl(&text).unwrap_or_else(|| DEFAULT_VERSION.to_string());
                self.pkg_languages.insert(f.clone(), language);
                self.pkg_headers.insert(f.clone(), header.clone());
                header
            };
            if file_name(&f) == format!("{name}.rnl") && header.as_ref().is_none_or(|(n, _)| n != name) {
                self.sink.warning(codes::PACKAGE_FILE_NAME_MISMATCH, format!("{f}: filename suggests package {name}, but its @PACKAGE header does not match; continuing package search"), Some(Loc::line(&f, 1)));
            }
            if let Some((n, v)) = header {
                out.push((n, v, f));
            }
        }
        self.pkg_files.insert(name.to_string(), out);
    }

    /// Find a package by path= or in the package directories (16.2). path= is relative to the
    /// directory of the file that writes the USE-PACKAGE or USE-ALIASES, or the reference: `what`
    /// is "USE-PACKAGE", "USE-ALIASES" or "reference".
    pub(crate) fn resolve_package(
        &mut self,
        decl: &UseDecl,
        loc: Loc,
        base: Option<String>,
        what: &str,
    ) -> Option<UnitId> {
        let sec = if what == "USE-ALIASES" { "16.5" } else { "16.2" };
        let kind = if what == "USE-ALIASES" { "alias package" } else { "package" };
        // a reference in the header record of a package file is reported in that package (7.6)
        let pkg = if what == "reference" { self.units[decl.unit].pkg_name.clone() } else { String::new() };
        let modern = self.follows_0_4(self.use_writer(decl));
        if modern && decl.version.as_deref().is_some_and(|v| !valid_package_version(v)) {
            let version = decl.version.as_deref().unwrap();
            self.diag_in(Severity::Error, codes::PACKAGE_VERSION_MALFORMED,
                format!("@{what} version={version:?} must be two unsigned decimal integers separated by a dot; this reference fails without lookup (16.1, 16.2)"), Some(loc), &pkg);
            return None;
        }
        if let Some(body) = &decl.body {
            let mut u = Unit::new(UnitKind::Package, &format!("{} (embedded)", decl.name));
            u.pkg_name = decl.name.clone();
            u.version = decl.version.clone().unwrap_or_default();
            u.alias_from = self.file_unit(decl.unit); // the body is the file's text: the file's aliases (16.5)
            u.embedded = true;
            let id = self.units.len();
            self.units.push(u);
            self.unit_recs.push(Vec::new());
            self.read_embedded(id, body.clone(), decl.rec);
            return Some(id);
        }
        if !modern {
            self.scan_packages(&decl.name);
        }
        let mut cand: Option<(String, String, String)> = None;
        if let Some(path) = &decl.path {
            let target = join(&base.unwrap_or_else(|| self.file_dir(decl.unit)), path);
            if let Some(text) = self.provider.read(&target) {
                if let Some((n, v)) = scan_package_decl(&text) {
                    if n == decl.name && version_ok(decl.version.as_deref(), &v, modern) {
                        if let Some(want) = &decl.version {
                            if *want != v {
                                self.diag_in(
                                    Severity::Warning,
                                    codes::PACKAGE_PATH_VERSION_DIFFERS, format!("{path} is {kind} {} {v}, not {want}: a different version may draw differently (16.2)", decl.name),
                                    Some(loc.clone()),
                                    &pkg,
                                );
                            }
                        }
                        cand = Some((n, v, target));
                    }
                } else if modern {
                    self.diag_in(Severity::Error, codes::PACKAGE_PATH_NOT_PACKAGE,
                        format!("{path} has no valid PACKAGE declaration; this {what} fails without looking up another file (2.7, 16.2)"), Some(loc), &pkg);
                    return None;
                }
            }
            if cand.is_none() {
                self.diag_in(
                    Severity::Warning,
                    codes::PACKAGE_PATH_NOT_FOUND, format!("the file named by path= does not exist or its @PACKAGE does not match; looking the package up by name ({sec})"),
                    Some(loc.clone()),
                    &pkg,
                );
            }
        }
        if cand.is_none() {
            if modern {
                self.scan_packages(&decl.name);
            }
            let same: Vec<(String, String, String)> =
                self.pkg_files[&decl.name].iter().filter(|c| c.0 == decl.name).cloned().collect();
            let reader = self.version_of(Some(self.use_writer(decl)));
            let found = |c: &&(String, String, String)| {
                finds_unnamed(self.pkg_languages.get(&c.2).map_or(DEFAULT_VERSION, String::as_str), &reader)
            };
            if let Some(want) = &decl.version {
                if let Some(c) =
                    same.iter()
                        .find(|c| if modern { package_version_cmp(&c.1, want, true).is_eq() } else { c.1 == *want })
                {
                    cand = Some(c.clone());
                } else {
                    let mut newer: Vec<&(String, String, String)> = same
                        .iter()
                        .filter(|c| {
                            same_package_major(&c.1, want, modern) && package_version_cmp(&c.1, want, modern).is_gt()
                        })
                        .collect();
                    newer.sort_by(|a, b| package_version_cmp(&a.1, &b.1, modern));
                    if let Some(c) = newer.first() {
                        self.diag_in(
                            Severity::Warning,
                            codes::PACKAGE_VERSION_SUBSTITUTED, format!("{kind} {} has no version {want}; using {}: a different version may draw differently (16.2)", decl.name, c.1),
                            Some(loc.clone()),
                            &pkg,
                        );
                        cand = Some((*c).clone());
                    }
                }
            } else if let Some(c) =
                same.iter().filter(found).min_by(|a, b| package_version_cmp(&b.1, &a.1, modern)).or_else(|| {
                    same.iter()
                        .filter(|c| {
                            !package_languages_match(
                                self.pkg_languages.get(&c.2).map_or(DEFAULT_VERSION, String::as_str),
                                &reader,
                            )
                        })
                        .min_by(|a, b| package_version_cmp(&b.1, &a.1, modern))
                })
            {
                // the newest found, and of its copies the one of the first package directory, as
                // for a version named (16.2)
                let this = if what == "reference" { "the reference to".to_string() } else { what.to_string() };
                self.diag_in(
                    Severity::Hint,
                    codes::PACKAGE_NO_VERSION_NEWEST,
                    format!("{this} {} names no version; using the newest found, {} ({sec})", decl.name, c.1),
                    Some(loc.clone()),
                    &pkg,
                );
                cand = Some(c.clone());
            }
        }
        let Some((n, v, path)) = cand else {
            let mut then = match what {
                "USE-ALIASES" => "; statements using its aliases fail".to_string(),
                "reference" => format!(", and the statements of package {pkg} that use its names fail with it"),
                _ => String::new(),
            };
            // a package of the series 0.2 that the series 0.3 merged into another (16.3)
            if let Some(merged) = (what != "USE-ALIASES").then(|| self.merged_in_0_3(decl)).flatten() {
                then.push_str(&format!("; {merged}"));
            }
            let ver = decl.version.as_ref().map(|v| format!(" {v}")).unwrap_or_default();
            self.diag_in(
                Severity::Error,
                codes::PACKAGE_NOT_FOUND,
                format!("{kind} {}{ver} cannot be found: this {what} fails{then} ({sec})", decl.name),
                Some(loc),
                &pkg,
            );
            return None;
        };
        let language = self
            .provider
            .read(&path)
            .and_then(|t| scan_language_decl(&t))
            .unwrap_or_else(|| DEFAULT_VERSION.to_string());
        let reader = self.version_of(Some(self.use_writer(decl)));
        if self.reject_package_language(&n, &reader, &language, Some(loc), &pkg, what) {
            return None;
        }
        if let Some(&u) = self.packages.get(&package_cache_key(n.clone(), v.clone(), language == "0.4")) {
            return Some(u);
        }
        Some(self.load_package_file(&path, Some((n, v))))
    }

    pub(super) fn use_writer(&self, decl: &UseDecl) -> UnitId {
        decl.rec.map_or(decl.unit, |r| self.rec_unit[r])
    }

    /// Keep 0.4 package references separate from the published 0.2/0.3 behavior (16.1).
    pub(super) fn reject_package_language(
        &mut self,
        name: &str,
        reader: &str,
        language: &str,
        loc: Option<Loc>,
        pkg: &str,
        what: &str,
    ) -> bool {
        // Unsupported versions keep their own diagnostic when the package is read.
        if package_languages_match(language, reader) || !READ_VERSIONS.contains(&language) {
            return false;
        }
        self.diag_in(Severity::Error, codes::PACKAGE_LANGUAGE_MISMATCH,
            format!("package {name} is written in language {language}, but this file uses {reader}: language 0.4 cannot use packages of earlier languages or be used by them; this {what} fails (16.1, 16.2, 16.3)"),
            loc, pkg);
        true
    }

    /// Read a package file into a unit (2.7). `key`, the name and version it is found by, holds it
    /// before its header record is read, so that a reference back to it finds it (16.2).
    pub(crate) fn load_package_file(&mut self, path: &str, key: Option<(String, String)>) -> UnitId {
        let text = self.provider.read(path).unwrap_or_default();
        let language = scan_language_decl(&text).unwrap_or_else(|| DEFAULT_VERSION.to_string());
        let fname = file_name(path).to_string();
        let mut lv = Level::new(LevelKind::Package, &fname, &fname, normalize(path));
        let recs = scan_package_run(&package_lines(&text), &fname, &mut self.sink);
        lv.records = recs.clone();
        let pl = self.pkg_levels.len();
        self.pkg_levels.push(lv);
        // files of one name in two package directories, such as rnl-std.rnl of two standard series,
        // are two sources: the revision of a snapshot counts the text of every file read
        if !self.pkg_sources.iter().any(|(f, t)| *f == fname && *t == text) {
            self.pkg_sources.push((fname.clone(), text));
        }
        let mut u = Unit::new(UnitKind::Package, &fname);
        // Mark the 0.4 boundary before reading references written ahead of VERSION in the
        // header. Leave the published 0.2/0.3 header handling as it was.
        if language == "0.4" {
            u.version = language.clone();
        }
        u.pkg_level = Some(pl);
        let id = self.units.len();
        self.units.push(u);
        if let Some((name, version)) = key {
            self.packages.insert(package_cache_key(name, version, language == "0.4"), id);
        }
        let rec_ids: Vec<RecId> = recs.into_iter().map(|r| self.add_record(r, id)).collect();
        self.unit_recs.push(rec_ids);
        self.read_unit(id);
        if self.units[id].header_record.is_none() || self.units[id].pkg_name.is_empty() {
            self.sink.error(
                codes::PACKAGE_FILE_NO_HEADER,
                format!("{fname} has no header record with @PACKAGE and is not a package (2.7, 16.2)"),
                Some(Loc::line(fname.clone(), 1)),
            );
            self.units[id].ok = false;
        }
        if !self.units[id].alias_pending.is_empty() && !is_alias_package(&self.units[id]) {
            // a package's own aliases read its own text: their originals are checked in its own version (16.5)
            self.check_pending_originals(id);
        }
        id
    }

    fn check_pending_originals(&mut self, u: UnitId) {
        let pending = std::mem::take(&mut self.units[u].alias_pending);
        let pkg = self.units[u].pkg_name.clone();
        for (alias, orig, loc) in pending {
            if standard_original_in(&orig, &self.version_of(Some(u))) {
                continue;
            }
            let what = if orig.starts_with('@') { "standard keyword" } else { "standard word" };
            self.diag_in(
                Severity::Error,
                codes::ALIAS_ORIGINAL_NOT_STANDARD,
                format!("the original {orig} is not a {what} (16.5)"),
                Some(loc),
                &pkg,
            );
            self.units[u].aliases.remove(&alias);
            self.units[u].alias_decls.retain(|d| !(d.alias == alias && d.definition.is_none()));
        }
    }

    fn read_embedded(&mut self, u: UnitId, body: Rc<SetExpr>, rec: Option<RecId>) {
        let header = Rc::new(Header::default());
        for (k, m) in body.members.iter().enumerate() {
            if let Some(err) = &m.error {
                let loc = self.rec_loc(rec, m.error_pos);
                self.sink.error(err.code, err.message.clone(), loc);
                self.note_failed(rec, m.start, m.end, false);
                continue;
            }
            // an inline package has no header record, so it references no other package (16.2)
            let call = m.segments.first().and_then(|s| s.call.as_ref()).filter(|_| m.segments.len() == 1);
            if let Some(call) = call.filter(|c| self.kw(&c.kw, u) == "USE-PACKAGE") {
                let loc = self.rec_loc(rec, call.start);
                self.sink.error(codes::INLINE_PACKAGE_REFERENCES_PACKAGE,
                    "an inline package has no header record and cannot reference other packages; to use the definitions of another package, write them into its body (16.2)",
                    loc,
                );
                self.note_failed(rec, m.start, m.end, false);
                continue;
            }
            self.units[u].members.push(UnitMember { rec, k, embedded: true, m: m.clone(), header: header.clone() });
            self.declare(u, rec, k, true, m, &header);
        }
    }

    /// Is the effective version of this level's own file supported (16.1)?
    pub(crate) fn file_ok(&self, level: LevelId) -> bool {
        self.unsupported_file(level).is_none()
    }

    /// The declaration supplying an unsupported version to this level's file. An explicit
    /// supported version stops inheritance, even below an unsupported ancestor. A file
    /// without a declaration inherits; subcircuits follow the file containing their text.
    pub(crate) fn unsupported_file(&self, level: LevelId) -> Option<(LevelId, String)> {
        let mut fu = self.file_unit(self.level_unit[level]);
        while let Some(u) = fu {
            let unit = &self.units[u];
            let lv = unit.level?;
            if !unit.version.is_empty() {
                return (!READ_VERSIONS.contains(&unit.version.as_str())).then(|| (lv, unit.version.clone()));
            }
            fu = self.doc.levels[lv].parent.and_then(|p| self.file_unit(self.level_unit[p]));
        }
        None
    }

    // ------------------------------------------------------------------ names (6.6)

    /// The records written above a subcircuit with scope= commit their statements to it, and the
    /// names they bind are published there, in one name space with its own (2.6, 6.7, as the
    /// draft 0.3 makes clear in its 7.4). They come after its own text, the nearest level's
    /// first, as they take effect; a name bound again is the 6.6 error of the subcircuit's level,
    /// and the earlier binding is kept. Called once every level is read.
    pub(crate) fn publish_scope_names(&mut self) {
        for lv in 0..self.doc.levels.len() {
            let level = &self.doc.levels[lv];
            if level.kind != LevelKind::Subckt {
                continue;
            }
            let key = self.doc.key(&level.name);
            let unit = self.level_unit[lv];
            let above = self.doc.ancestors(lv);
            for (depth, a) in above.iter().enumerate() {
                let au = self.level_unit[*a];
                // the records whose scope= names it, by the rules of the file it is written in:
                // in a netlist of one dialect, those of its key
                let ks: Vec<usize> = if self.doc.mixed() {
                    let records = &self.units[au].scope_records;
                    let named = |i: &usize| records[*i].2.scope.as_deref().is_some_and(|t| self.doc.names_level(lv, t));
                    let mut ks: Vec<usize> =
                        self.units[au].scope_index.values().flatten().copied().filter(named).collect();
                    ks.sort_unstable();
                    ks
                } else {
                    match self.units[au].scope_index.get(&key) {
                        Some(ks) => ks.clone(),
                        None => continue,
                    }
                };
                let place = -((above.len() - depth) as i64);
                for i in ks {
                    let (rec, ast, header) = self.units[au].scope_records[i].clone();
                    self.units[unit].scope_places.insert(rec, place);
                    for (k, m) in ast.body.as_ref().unwrap().members.iter().enumerate() {
                        if m.error.is_some() {
                            continue;
                        }
                        let member =
                            UnitMember { rec: Some(rec), k, embedded: false, m: m.clone(), header: header.clone() };
                        self.units[unit].scope_members.push(member);
                        self.declare(unit, Some(rec), k, false, m, &header);
                    }
                }
            }
        }
    }

    pub(crate) fn declare(
        &mut self,
        u: UnitId,
        rec: Option<RecId>,
        k: usize,
        embedded: bool,
        m: &Rc<Member>,
        header: &Rc<Header>,
    ) {
        for b in &m.binds {
            let key = (b.prefix, b.name.clone());
            let written = m
                .written
                .get(&format!("{}{}", b.prefix, b.name))
                .cloned()
                .unwrap_or_else(|| format!("{}{}", b.prefix, b.name));
            let binding = Binding {
                prefix: b.prefix,
                name: b.name.clone(),
                member: m.clone(),
                rec,
                owner: u,
                index: k,
                embedded,
                off: b.off,
                written,
                header: header.clone(),
            };
            if self.units[u].names.contains_key(&key) {
                self.units[u].dups.push(binding);
                continue;
            }
            self.units[u].names.insert(key.clone(), binding);
            self.units[u].name_order.push(key);
        }
    }
}

/// Is a word one that the language version `version` adds, `numbering` in 0.3 (16.1)? It is a
/// standard word in a file of that version or a later one, which no alias may be spelled as (16.5).
pub fn added_word(word: &str, version: &str) -> bool {
    keeps_0_3(version) && crate::words::draft_header_fields().iter().any(|f| f.name == word)
}

/// Is an alias original a standard keyword (written with `@`) or a standard word (16.1, 16.5)?
pub fn standard_original(orig: &str) -> bool {
    match orig.strip_prefix('@') {
        Some(k) => words().is_keyword(k),
        None => words().all_words().contains(orig),
    }
}

/// Keep the released alias vocabulary unchanged when introducing solver settings.
fn standard_original_in(orig: &str, version: &str) -> bool {
    if !keeps_0_4(version)
        && matches!(orig, "@TABLE" | "transpose" | "spacing_x" | "spacing_y" | "@SOLVER-CONFIG" | "junction")
    {
        return false;
    }
    standard_original(orig)
}

/// Read `$` aliases as their originals, whole token, wherever a `$` name is written (16.5).
pub fn rename_dollar_set(set: &mut SetExpr, table: &HashMap<String, String>) {
    for m in &mut set.members {
        let m = Rc::make_mut(m);
        rename_dollar_member(m, table);
    }
}

fn rename_dollar_member(m: &mut Member, table: &HashMap<String, String>) {
    for b in &mut m.binds {
        if b.prefix == '$' {
            if let Some(orig) = table.get(&format!("${}", b.name)) {
                let written = format!("${}", b.name);
                b.name = orig[1..].to_string();
                m.written.insert(format!("${}", b.name), written);
            }
        }
    }
    for seg in &mut m.segments {
        for a in &mut seg.args {
            rename_dollar_obj(&mut a.obj, table);
        }
    }
}

fn rename_dollar_obj(o: &mut ObjExpr, table: &HashMap<String, String>) {
    match o {
        ObjExpr::Set(s) => rename_dollar_set(Rc::make_mut(s), table),
        ObjExpr::Ref(r) => {
            if r.prefix == '$' {
                if let Some(n) = &r.name {
                    if let Some(orig) = table.get(&format!("${n}")) {
                        r.written = Some(format!("${n}"));
                        r.name = Some(orig[1..].to_string());
                    }
                }
            }
        }
        ObjExpr::Point(_) => {}
    }
}

/// Check that a value fits a parameter type from the word list.
pub fn type_name(t: &ParamType) -> &'static str {
    match t {
        ParamType::Number => "a number",
        ParamType::String => "a string",
        ParamType::Bool => "true or false",
        ParamType::Array => "an array",
        ParamType::Term => "a terminal",
        ParamType::Color => "a colour",
        ParamType::Enum(_) => "one of the listed values",
        ParamType::Any => "a value",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similarity_like_difflib() {
        assert!((similarity("alig", "align") - 0.888).abs() < 0.01);
        assert_eq!(closest("priorty", ["type", "scope", "priority", "author"]).as_deref(), Some("priority"));
        assert_eq!(
            scan_package_decl(
                "@RNL(type=\"header\") { @VERSION(\"0.2\"); @PACKAGE(name=\"rnl-std\", version=\"0.2\")}"
            ),
            Some(("rnl-std".into(), "0.2".into()))
        );
    }
}
