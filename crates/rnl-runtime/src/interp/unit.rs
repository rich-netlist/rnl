//! Units of RNL text: host levels and packages (standard 2.7, 2.8, 6.1, 16.1–16.5).
//!
//! A unit has its own name table. Reading a unit parses its records, reads the header record
//! (version, aliases, terminal names, root packages) and collects the members of the others.

use std::collections::HashMap;
use std::rc::Rc;

use crate::diag::{Extra, Loc, Severity};
use crate::host::{file_name, Level, LevelId, LevelKind};
use crate::records::{package_lines, scan_run};
use crate::source::{dirname, join, normalize};
use crate::syntax::{self, Member, ObjExpr, Params, RecordAst, SetExpr, Value};
use crate::words::{words, ParamType};

use super::obj::{Lit, MemberKey, Priority, RecId, UnitId};
use super::Interp;

/// The versions of the language this implementation reads (16.1): 0.2, and 0.3, whose text is
/// still a draft. Every rule that 0.3 changes applies only to the files of 0.3, so a file of 0.2
/// reads as it did before 0.3 was read.
pub const READ_VERSIONS: [&str; 2] = ["0.2", "0.3"];
/// The version a main file, or a package file, without a header record is read in: the tool's
/// default (16.1). An included file without one follows the file that includes it.
pub const DEFAULT_VERSION: &str = "0.3";
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
}

impl Default for Header {
    fn default() -> Header {
        Header { type_: "authored".into(), priority: None, scope: None, numbering: None }
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
    pub name: String,
    pub version: Option<String>,
    pub path: Option<String>,
    pub body: Option<Rc<SetExpr>>,
    pub unit: UnitId,
    pub in_header: bool,
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
    pub terminals: Vec<Lit>,
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
    /// alias -> original: keywords with "@", `$` names with "$", other standard words bare.
    pub aliases: HashMap<String, String>,
    pub alias_decls: Vec<AliasDecl>,
    pub alias_pending: Vec<(String, String, Loc)>,
    pub alias_rejected: HashMap<String, String>,
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
            aliases: HashMap::new(),
            alias_decls: Vec::new(),
            alias_pending: Vec::new(),
            alias_rejected: HashMap::new(),
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
    /// The USE-PACKAGE written in a file that brings it in; none for the standard package used
    /// implicitly (16.3) and for a package checked on its own (2.7).
    pub decl: Option<UseDecl>,
    pub unit: UnitId,
    pub place: Place,
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

fn same_major(a: &str, b: &str) -> bool {
    let (ka, kb) = (version_key(a), version_key(b));
    if ka.0 == 0 || kb.0 == 0 {
        ka == kb
    } else {
        ka.0 == kb.0
    }
}

fn version_ok(want: Option<&str>, have: &str) -> bool {
    match want {
        None => true,
        Some(w) => w == have || (same_major(w, have) && version_key(have) > version_key(w)),
    }
}

/// "@PACKAGE(name="…", version="…")" in a package file's text.
pub fn scan_package_decl(text: &str) -> Option<(String, String)> {
    let at = text.find("@PACKAGE(")?;
    let rest = &text[at + "@PACKAGE(".len()..];
    let end = rest.find(')')?;
    let inner = &rest[..end];
    let field = |name: &str| -> Option<String> {
        let k = inner.find(name)?;
        let after = inner[k + name.len()..].trim_start();
        let after = after.strip_prefix('=')?.trim_start();
        let after = after.strip_prefix('"')?;
        let q = after.find('"')?;
        Some(after[..q].to_string())
    };
    Some((field("name")?, field("version")?))
}

/// The language version a package file declares, "@VERSION(…)" in its text, read as the version
/// of the file is (16.1); None when it declares none. The standard series a package is of is
/// written in this version (16.3).
pub fn scan_language_decl(text: &str) -> Option<String> {
    let at = text.find("@VERSION(")?;
    let rest = text[at + "@VERSION(".len()..].trim_start();
    // given by position or by name
    let rest = match rest.strip_prefix("version") {
        Some(named) => named.trim_start().strip_prefix('=')?.trim_start(),
        None => rest,
    };
    let rest = rest.strip_prefix('"')?;
    Some(rest[..rest.find('"')?].to_string())
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
        msg: impl Into<String>,
        loc: Option<Loc>,
        pkg: &str,
    ) -> crate::diag::DiagId {
        self.sink.add(sev, msg, loc, Extra { in_package: pkg.to_string(), ..Extra::default() })
    }

    /// Parse a record once. In a file of the draft 0.3, `%#3` is a number (5.9); in 0.2 it stays a
    /// lexical error.
    pub(crate) fn parse(&mut self, rec: RecId) -> Rc<RecordAst> {
        if let Some(a) = &self.asts[rec] {
            return a.clone();
        }
        let numbers = self.follows_0_3(self.rec_unit[rec]);
        let a = Rc::new(syntax::parse_record_with(&self.recs[rec].text, numbers));
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

    pub(crate) fn read_unit(&mut self, u: UnitId) {
        let level_kind = self.unit_level_kind(u);
        let is_file_level = matches!(level_kind, Some(LevelKind::File) | Some(LevelKind::Package));
        let pkg = self.units[u].pkg().to_string();
        for (index, rec) in self.unit_records(u).into_iter().enumerate() {
            let ast = self.parse(rec);
            if !self.read_generated && self.units[u].kind == UnitKind::Level && is_generated_record(&ast) {
                // a solver does not read the solver's records, nor what they would report (2.8, 14.8)
                continue;
            }
            for note in &ast.notes {
                if let Some(kw) = &note.kw {
                    let k = self.kw(kw, u);
                    if k == "AS" || k == "ATTACH" {
                        continue; // `$GND @ATTACH (0, 0)`: the point is an object of AS, ATTACH (5.6)
                    }
                }
                let loc = self.recs[rec].loc(note.off);
                self.diag_in(Severity::Warning, note.message.clone(), Some(loc), &pkg);
            }
            if let Some((msg, pos)) = &ast.error {
                let loc = self.recs[rec].loc(*pos);
                self.diag_in(Severity::Error, msg.clone(), Some(loc), &pkg);
                let n = self.recs[rec].text.chars().count();
                self.note_failed(Some(rec), 0, n, false);
                self.units[u].other_records += 1;
                continue;
            }
            let header = Rc::new(self.read_header_fields(&ast, rec, u));
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
                self.diag_in(Severity::Hint, super::numbers::STALE_HINT, Some(loc), &pkg);
            }
            if header.type_ == "header" {
                // at the file level, the first record of the whole file, and at most one (2.8)
                let wrong = if !is_file_level {
                    Some("a header record is written at the file level, not inside a subcircuit (2.8); this one is ignored")
                } else if self.units[u].header_record.is_some() {
                    Some("a file has at most one header record (2.8); this one is ignored")
                } else if index != 0 || self.after_a_record_of_its_file(u, rec) {
                    Some("a header record is the first record of a file, and a record comes before this one (2.8); this one is ignored")
                } else {
                    None
                };
                if let Some(msg) = wrong {
                    let loc = self.recs[rec].loc(0);
                    self.diag_in(Severity::Error, msg, Some(loc), &pkg);
                    self.units[u].other_records += 1;
                    continue;
                }
                self.units[u].header_record = Some(rec);
                self.read_header(u, rec, &ast);
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
                for m in ast_body.members.iter().filter(|m| m.error.is_some()) {
                    let loc = self.recs[rec].loc(m.error_pos);
                    self.diag_in(Severity::Error, m.error.clone().unwrap(), Some(loc), &pkg);
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
                    let loc = self.recs[rec].loc(m.error_pos);
                    self.diag_in(Severity::Error, err.clone(), Some(loc), &pkg);
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
                self.free_word_warnings(u, rec, m);
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
        {
            // the header record may be left out; tools write it when they save the file (16.1, TR5)
            let file = self.doc.levels[0].file.clone();
            self.sink.hint(
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

    /// 3.3 recommended warning: a free word equal to an instance, an RNL name or a keyword.
    fn free_word_warnings(&mut self, u: UnitId, rec: RecId, m: &Member) {
        if m.free_words.is_empty() {
            return;
        }
        let insts: Vec<String> = match self.units[u].level {
            Some(lv) if self.units[u].kind == UnitKind::Level => {
                self.doc.levels[lv].instances.iter().map(|i| i.name.to_lowercase()).collect()
            }
            _ => Vec::new(),
        };
        let fu = self.file_unit(u);
        let alias_kws: Vec<String> = fu
            .map(|f| self.units[f].aliases.keys().filter_map(|a| a.strip_prefix('@').map(String::from)).collect())
            .unwrap_or_default();
        let pkg = self.units[u].pkg().to_string();
        for (word, off) in &m.free_words {
            let what = if insts.contains(&word.to_lowercase()) {
                Some(format!("instance #{word}"))
            } else if words().is_keyword(word) || alias_kws.contains(word) {
                Some(format!("keyword @{word}"))
            } else if self.units[u].names.keys().any(|(_p, n)| n == word) {
                Some("an RNL name".to_string())
            } else {
                None
            };
            if let Some(what) = what {
                let loc = self.recs[rec].loc(*off);
                self.diag_in(
                    Severity::Warning,
                    format!("the free-text word {word} is spelled like {what}: is a prefix missing? (3.3)"),
                    Some(loc),
                    &pkg,
                );
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
                "record header fields are written by name; positional values are ignored (2.8)",
                Some(loc),
                &pkg,
            );
        }
        // `numbering` is a field of the draft 0.3 only (2.8, 5.9)
        let draft: &[crate::words::HeaderField] =
            if self.follows_0_3(u) { crate::words::draft_header_fields() } else { &[] };
        let (mut numbering_at, mut scope_at) = (None, None);
        for n in &params.named {
            let loc = self.recs[rec].loc(n.off);
            let fields = || words().header_fields.iter().chain(draft);
            let Some(spec) = fields().find(|f| f.name == n.name) else {
                let close = closest(&n.name, fields().map(|f| f.name.as_str()));
                let hint = close.map(|c| format!(", did you mean {c}?")).unwrap_or_default();
                self.diag_in(
                    Severity::Warning,
                    format!("unknown record header field {} is ignored (2.8){hint}", n.name),
                    Some(loc),
                    &pkg,
                );
                continue;
            };
            let Value::Str(v) = &n.value else {
                self.diag_in(
                    Severity::Error,
                    format!("record header field {} takes a string; treated as not written (2.8)", n.name),
                    Some(loc),
                    &pkg,
                );
                continue;
            };
            if let Some(vals) = &spec.values {
                if !vals.contains(v) {
                    self.diag_in(
                        Severity::Error,
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
                            format!("record header field numbering holds a check of 16 hexadecimal digits (5.9), not {v:?}; treated as not written (2.8)"),
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
        // the RNL of a subcircuit covers only itself: scope written inside one is an error, and the
        // record is read as one of the subcircuit's own (2.6, 2.8)
        if let (Some(loc), Some(LevelKind::Subckt)) = (scope_at, self.unit_level_kind(u)) {
            self.diag_in(
                Severity::Error,
                "scope= is not written in a record inside a subcircuit, whose RNL covers only itself; treated as not written, so the record is the subcircuit's own (2.6, 2.8)",
                Some(loc),
                &pkg,
            );
            h.scope = None;
        }
        // numbering is written only in the record of the solver, which is in the level's own text
        let wrong = if h.type_ != "generated" {
            Some("record header field numbering is written only with type=\"generated\", in the record of the solver; treated as not written (2.8, 5.9)")
        } else if h.scope.is_some() {
            Some("record header field numbering is not written in a record with scope=: the record of the solver is in the level's own text; treated as not written (2.6, 2.8, 5.9)")
        } else {
            None
        };
        if let (Some(loc), Some(msg)) = (numbering_at, wrong) {
            self.diag_in(Severity::Error, msg, Some(loc), &pkg);
            h.numbering = None;
        }
        if self.units[u].kind == UnitKind::Package && (h.scope.is_some() || h.type_ == "generated") {
            let loc = self.recs[rec].loc(0);
            self.diag_in(Severity::Error, "a package cannot use scope= or type=\"generated\" (16.2)", Some(loc), &pkg);
            h.scope = None;
            h.type_ = "authored".into();
        }
        h
    }

    /// The header record is always read in the standard form, without aliases (16.5).
    fn read_header(&mut self, u: UnitId, rec: RecId, ast: &RecordAst) {
        let pkg = self.units[u].pkg().to_string();
        let body = ast.body.clone().unwrap();
        for (k, m) in body.members.iter().enumerate() {
            if let Some(err) = &m.error {
                let loc = self.recs[rec].loc(m.error_pos);
                self.diag_in(Severity::Error, err.clone(), Some(loc), &pkg);
                self.note_failed(Some(rec), m.start, m.end, false);
                self.units[u].header_kws.push("?".into());
                continue;
            }
            if m.segments.len() != 1 || m.segments[0].call.is_none() || m.bind().is_some() {
                let loc = self.recs[rec].loc(m.start);
                self.diag_in(
                    Severity::Error,
                    "a header record holds only VERSION, PACKAGE, USE-PACKAGE, USE-ALIASES, ALIAS and DEVICE (2.8)",
                    Some(loc),
                    &pkg,
                );
                self.units[u].header_kws.push("?".into());
                continue;
            }
            let seg = m.segments[0].clone();
            let call = seg.call.clone().unwrap();
            let kw = call.kw.clone();
            self.units[u].header_kws.push(kw.clone());
            let loc = self.recs[rec].loc(call.start);
            if !HEADER_KW.contains(&kw.as_str()) && kw != "USE-PACKAGE" {
                self.diag_in(
                    Severity::Error,
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
                self.diag_in(Severity::Error, format!("@{kw} takes no objects (2.8)"), Some(loc), &pkg);
                continue;
            }
            match kw.as_str() {
                // a file declares its version once (16.1): the first VERSION of its header
                // record stands, as the scan of the package directories takes it for a package file
                "VERSION" if self.units[u].header_kws.iter().filter(|k| *k == "VERSION").count() > 1 => {
                    self.diag_in(Severity::Error, SECOND_VERSION, Some(loc), &pkg);
                }
                "VERSION" => {
                    let Some(Value::Str(v)) = p.get("version") else {
                        self.diag_in(Severity::Error, "@VERSION takes a string (16.1)", Some(loc), &pkg);
                        continue;
                    };
                    self.units[u].version = v.clone();
                    if version_key(v).0 < 0 || !READ_VERSIONS.contains(&v.as_str()) {
                        let reads = READ_VERSIONS.join(" and ");
                        self.diag_in(
                            Severity::Error,
                            format!("unsupported version {v}: no RNL of this file is interpreted (16.1; this implementation reads {reads})"),
                            Some(loc),
                            &pkg,
                        );
                        self.units[u].ok = false;
                    }
                }
                "PACKAGE" => {
                    if self.units[u].kind != UnitKind::Package {
                        self.sink
                            .error("@PACKAGE is written only in the header record of a package file (2.7)", Some(loc));
                        continue;
                    }
                    let name = str_val(p.get("name")).unwrap_or_default();
                    let version = str_val(p.get("version")).unwrap_or_default();
                    self.units[u].pkg_name = name.clone();
                    self.units[u].pkg_version = version.clone();
                    // a package checked on its own is the one that its references, and the
                    // standard package's, find by its name and version; one that a USE-PACKAGE
                    // or a reference found is there already (16.2)
                    if !name.is_empty() {
                        self.packages.entry((name, version)).or_insert(u);
                    }
                }
                "USE-ALIASES" => self.use_aliases(u, &p, &loc),
                "ALIAS" => {
                    let (Some(alias), Some(orig)) = (str_val(p.get("alias")), str_val(p.get("original"))) else {
                        self.diag_in(
                            Severity::Error,
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
                            self.diag_in(Severity::Error, "@ALIAS takes two strings: the alias and the original; definition= is a string too (16.5)", Some(loc), &pkg);
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
                    let name = str_val(p.get("name"));
                    let terms = match p.get("terminals") {
                        Some(Value::Arr(items)) => Some(items.clone()),
                        _ => None,
                    };
                    let (Some(name), Some(terms)) = (name, terms) else {
                        self.diag_in(Severity::Error, "@DEVICE takes name and terminals (6.12)", Some(loc), &pkg);
                        continue;
                    };
                    let model = str_val(p.get("model"));
                    let terminals = terms
                        .iter()
                        .filter_map(|x| match x {
                            Value::Str(s) => Some(Lit::Str(super::obj::WStr::new(s, None))),
                            Value::Num { value, .. } => Some(Lit::Num(*value)),
                            _ => None,
                        })
                        .collect();
                    self.units[u].devices.push(DeviceDecl { name, model, terminals });
                }
                _ => {}
            }
        }
        self.finish_aliases(u);
    }

    /// Is this package the document being checked on its own (2.7)?
    pub(crate) fn checking_package_unit(&self, u: UnitId) -> bool {
        match (self.units[u].pkg_level, &self.checking_path) {
            (Some(pl), Some(p)) => self.pkg_levels[pl].path == *p,
            _ => false,
        }
    }

    /// Parameters of a statement read when the file is read (header statements, USE-PACKAGE):
    /// positional ones first in the signature's order, then names; nothing twice (4.2 rule 9).
    /// The header is in the standard form; elsewhere an unquoted name may be an alias (16.5).
    pub(crate) fn header_params(
        &mut self,
        kw: &str,
        p: Option<&Params>,
        u: UnitId,
        loc: &Loc,
        std_form: bool,
    ) -> Option<HashMap<String, Value>> {
        let pkg = self.units[u].pkg().to_string();
        let spec = words().keyword(kw).unwrap();
        let names: Vec<&str> = spec.param_list().iter().map(|q| q.name.as_str()).collect();
        let order = &spec.positional;
        let mut out = HashMap::new();
        if let Some(p) = p {
            if !p.positional.is_empty() {
                if order.is_empty() {
                    self.diag_in(
                        Severity::Error,
                        format!("the parameters of @{kw} cannot be written by position; write each by name, such as {}=… (4.2 rule 9)", names[0]),
                        Some(loc.clone()),
                        &pkg,
                    );
                    return None;
                }
                if p.positional.len() > order.len() {
                    self.diag_in(
                        Severity::Error,
                        format!(
                            "@{kw} takes at most {} parameters by position ({}), {} are written (4.2 rule 9)",
                            order.len(),
                            order.join(", "),
                            p.positional.len()
                        ),
                        Some(loc.clone()),
                        &pkg,
                    );
                    return None;
                }
                for (name, v) in order.iter().zip(&p.positional) {
                    out.insert(name.clone(), v.clone());
                }
            }
            for n in &p.named {
                let std = if std_form || n.quoted { n.name.clone() } else { self.word(&n.name, u) };
                if !names.contains(&std.as_str()) {
                    let close = closest(&std, names.iter().copied());
                    let mut msg = format!("@{kw} has no parameter {}", n.name);
                    if let Some(c) = close {
                        msg.push_str(&format!(", did you mean {c}?"));
                    }
                    if !n.name.is_ascii() && !n.quoted {
                        if std_form {
                            msg.push_str("; a header record is read in the standard form, without aliases (16.5)");
                        } else {
                            msg.push_str("; is an alias package not loaded?");
                        }
                    }
                    msg.push_str(" (16.4)");
                    self.diag_in(Severity::Error, msg, Some(loc.clone()), &pkg);
                    return None;
                }
                if out.contains_key(&std) {
                    self.diag_in(
                        Severity::Error,
                        format!("parameter {std} of @{kw} is written twice (4.2 rule 9)"),
                        Some(loc.clone()),
                        &pkg,
                    );
                    return None;
                }
                out.insert(std, n.value.clone());
            }
        }
        for q in spec.param_list() {
            if q.required && !out.contains_key(&q.name) {
                self.diag_in(
                    Severity::Error,
                    format!("@{kw} lacks the parameter {} (5.3)", q.name),
                    Some(loc.clone()),
                    &pkg,
                );
                return None;
            }
        }
        Some(out)
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
        _version: &str,
    ) -> Option<(String, String, Option<String>)> {
        let bad = |s: &mut Self, msg: String| {
            s.diag_in(Severity::Error, msg, Some(loc.clone()), pkg);
            None
        };
        if let Some(def) = definition {
            // a name inside a definition: role, parameter or local name, used where that definition is applied
            if !def.starts_with('$') || def.chars().count() < 2 {
                return bad(self, format!("definition= names an ordinary definition by its `$` name, such as \"$DIFFPAIR\", not {def:?} (16.5)"));
            }
            let first_digit = alias.chars().next().is_some_and(|c| c.is_ascii_digit());
            if alias.is_empty() || !alias.chars().all(syntax::is_name_char) || first_digit {
                return bad(self, format!("alias {alias:?} is not well formed: an alias of a name in a definition must be writable unquoted in parentheses (16.5)"));
            }
            if orig.is_empty() || "@$%?#&".contains(orig.chars().next().unwrap()) {
                return bad(self, format!("with definition=, the original is a role, parameter or local name without a prefix, not {orig:?} (16.5)"));
            }
            return Some((alias.into(), orig.into(), Some(def.into())));
        }
        if alias.starts_with('$') || orig.starts_with('$') {
            if !(alias.starts_with('$') && orig.starts_with('$'))
                || alias.chars().count() < 2
                || orig.chars().count() < 2
            {
                return bad(self, "the alias and the original of a name both carry `$`, such as @ALIAS(\"$差分对\", \"$DIFFPAIR\") (16.5)".into());
            }
            if alias == orig {
                return bad(self, format!("the alias {alias} is the same as the original (16.5)"));
            }
            return Some((alias.into(), orig.into(), None));
        }
        if orig.starts_with('@') != alias.starts_with('@') {
            return bad(
                self,
                "the original and the alias of a keyword both carry `@`; other words carry neither (16.5)".into(),
            );
        }
        if let Some(base) = orig.strip_prefix('@') {
            if matches!(base, "RNL" | "VERSION" | "ALIAS") {
                return bad(self, format!("{orig} belongs to the outer form and cannot have an alias (16.5)"));
            }
            if base == "USE-ALIASES" {
                return bad(
                    self,
                    "@USE-ALIASES cannot have an alias: the aliases are not known before it is read (16.5)".into(),
                );
            }
            if check_original && !standard_original(orig) {
                return bad(self, format!("the original {orig} is not a standard keyword (16.5)"));
            }
            let a = &alias[1..];
            if words().is_keyword(a) || a.is_empty() || !a.chars().all(syntax::is_word_char) {
                return bad(
                    self,
                    format!("the alias {alias} is not well formed or is the name of a standard keyword (16.5)"),
                );
            }
        } else {
            if check_original && !standard_original(orig) {
                return bad(self, format!("the original {orig} is not a standard word (16.5)"));
            }
            if words().all_words().contains(alias) {
                return bad(self, format!("the alias {alias} is the name of a standard word (16.5)"));
            }
        }
        Some((alias.into(), orig.into(), None))
    }

    /// Add a checked declaration to a file's aliases; together they keep one original per alias in
    /// its scope (16.5). `source` names the alias package it comes from.
    fn add_alias(&mut self, u: UnitId, decl: (String, String, Option<String>), loc: &Loc, source: &str) {
        let (alias, orig, definition) = decl;
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
                self.sink.error(
                    format!("{wher}the alias {alias} is already declared for {old} and cannot also be {orig} (16.5)"),
                    Some(loc.clone()),
                );
            }
            return;
        }
        if alias.starts_with('$') {
            if self.units[u].aliases.contains_key(&orig) {
                self.sink.error(format!("{wher}the alias {orig} cannot also be an original (16.5)"), Some(loc.clone()));
                return;
            }
            if self.units[u].aliases.values().any(|o| *o == alias) {
                self.sink.error(
                    format!("{wher}{alias} is already the original of another alias and cannot be an alias (16.5)"),
                    Some(loc.clone()),
                );
                return;
            }
        }
        self.units[u].aliases.insert(alias.clone(), orig.clone());
        self.units[u].alias_decls.push(AliasDecl { alias, original: orig, definition: None, loc: loc.clone() });
    }

    /// Definition-scoped aliases by the definition's `$` name, read through the `$` aliases (16.5, D17).
    fn finish_aliases(&mut self, u: UnitId) {
        self.units[u].scoped.clear();
        let mut prio: Vec<String> = vec!["priority".into()];
        prio.extend(self.units[u].aliases.iter().filter(|(_, o)| *o == "priority").map(|(a, _)| a.clone()));
        let raw = self.units[u].scoped_raw.clone();
        for (definition, alias, orig, loc, source) in raw {
            let wher = if source.is_empty() { String::new() } else { format!("in alias package {source}, ") };
            let key = self.units[u].aliases.get(&definition).cloned().unwrap_or(definition);
            if prio.contains(&alias) {
                // the standard parameter of AS, ATTACH (14.8) is there for every definition
                self.sink.error(
                    format!("{wher}the alias {alias} is spelled like the standard parameter priority of AS, ATTACH or its alias; its meaning is not unique (16.5)"),
                    Some(loc),
                );
                continue;
            }
            let table = self.units[u].scoped.entry(key.clone()).or_default();
            if let Some((_, (o, _))) = table.iter().find(|(a, _)| *a == alias) {
                if *o != orig {
                    let o = o.clone();
                    self.sink.error(format!("{wher}the alias {alias} in {key} is already declared for {o} and cannot also be {orig} (16.5)"), Some(loc));
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
        let Some(table) = self.units[fu].scoped.get(&name).cloned() else { return Vec::new() };
        if let Some(v) = self.scoped_memo.get(&(fu, dfn)) {
            return v.clone();
        }
        let d = self.def(dfn).clone();
        let mut taken: Vec<String> = d.roles.iter().map(|r| r.name.clone()).collect();
        taken.extend(d.params.iter().map(|(n, _)| n.clone()));
        taken.extend(d.local_names());
        let mut out = Vec::new();
        for (alias, (orig, loc)) in table {
            if taken.contains(&alias) {
                self.sink.error(
                    format!("the alias {alias} is spelled like a role, parameter or local name of the definition {name} itself; its meaning is not unique and the alias is not used (16.5)"),
                    Some(loc),
                );
                continue;
            }
            out.push((alias, orig));
        }
        self.scoped_memo.insert((fu, dfn), out.clone());
        out
    }

    /// `@USE-ALIASES` in a header: load an alias package and add its aliases to this file's (16.5).
    fn use_aliases(&mut self, u: UnitId, p: &HashMap<String, Value>, loc: &Loc) {
        let pkg = self.units[u].pkg().to_string();
        let Some(name) = str_val(p.get("name")) else {
            self.diag_in(Severity::Error, "@USE-ALIASES takes name (16.5)", Some(loc.clone()), &pkg);
            return;
        };
        let decl = UseDecl {
            name: name.clone(),
            version: str_val(p.get("version")),
            path: str_val(p.get("path")),
            body: None,
            unit: u,
            in_header: true,
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
                format!("package {name} is not an alias package: its header has only @VERSION, @PACKAGE and @ALIAS and there is no other record; this USE-ALIASES fails (16.5)"),
                Some(loc.clone()),
                &pkg,
            );
            return;
        }
        let version = self.version_of(Some(u));
        let decls = self.units[pk].alias_decls.clone();
        for d in decls {
            if d.definition.is_none() && !d.original.starts_with('$') && !standard_original(&d.original) {
                let what = if d.original.starts_with('@') { "standard keyword" } else { "standard word" };
                self.diag_in(
                    Severity::Error,
                    format!(
                        "in alias package {name}, the original {} of the alias {} is not a {what} in this file's language version {version}; the alias is not used (16.5)",
                        d.original, d.alias
                    ),
                    Some(loc.clone()),
                    &pkg,
                );
                self.units[u].alias_rejected.entry(d.alias.clone()).or_insert(name.clone());
                continue;
            }
            self.add_alias(u, (d.alias, d.original, d.definition), loc, &name);
        }
    }

    /// The standard keyword for a written keyword, through the file's aliases (16.5).
    pub(crate) fn kw(&self, written: &str, u: UnitId) -> String {
        if words().is_keyword(written) {
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

    /// Does a unit follow 0.3? When the file the unit's text is in is read in 0.3
    /// ([`Self::version_of`]): every rule that 0.3 changes applies only there, so a file of 0.2
    /// reads as it did before 0.3 was read.
    pub fn follows_0_3(&self, u: UnitId) -> bool {
        self.version_of(Some(u)) == "0.3"
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
                "USE-PACKAGE is not written in the records of a package other than its header record: a package file references other packages in its header record (16.2)",
                Some(loc),
                &pkg,
            );
            return;
        }
        let Some(p) = self.header_params("USE-PACKAGE", call.params.as_ref(), u, &loc, in_header) else { return };
        let Some(name) = str_val(p.get("name")) else {
            self.diag_in(Severity::Error, "@USE-PACKAGE takes name (16.2)", Some(loc), &pkg);
            return;
        };
        let mut body = None;
        if !seg.args.is_empty() {
            if reference {
                // inline content has no version and cannot take part in choosing one (16.2)
                self.diag_in(
                    Severity::Error,
                    "a reference to another package in the header record of a package file writes only name, version and path, without a body: this reference fails (16.2)",
                    Some(loc),
                    &pkg,
                );
                return;
            }
            match (seg.args.len(), &seg.args[0].obj) {
                (1, ObjExpr::Set(s)) => body = Some(s.clone()),
                _ => {
                    self.sink.error("@USE-PACKAGE takes one set as its body (16.2)", Some(loc));
                    return;
                }
            }
        }
        let mut decl = UseDecl {
            name: name.clone(),
            version: str_val(p.get("version")),
            path: str_val(p.get("path")),
            body: body.clone(),
            unit: u,
            in_header,
            pos: (rec.unwrap_or(0), k),
            rec,
            offset: call.start,
            pkg: None,
        };
        let what = if reference { "reference" } else { "USE-PACKAGE" };
        decl.pkg = self.resolve_package(&decl, loc.clone(), None, what);
        if let (Some(pk), None) = (decl.pkg, &body) {
            if is_alias_package(&self.units[pk]) && self.units[pk].header_kws.iter().any(|k| k == "ALIAS") {
                self.diag_in(
                    Severity::Error,
                    format!("package {name} is an alias package; load it with @USE-ALIASES(name=\"{name}\") in the header record. This {what} fails (16.5)"),
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

    pub(crate) fn scan_packages(&mut self) {
        if self.pkg_files.is_some() {
            return;
        }
        let mut out = Vec::new();
        for f in self.provider.package_files() {
            if let Some(text) = self.provider.read(&f) {
                if let Some((n, v)) = scan_package_decl(&text) {
                    // the language version it is written in, the tool's default when it declares none (16.1)
                    let language = scan_language_decl(&text).unwrap_or_else(|| DEFAULT_VERSION.to_string());
                    self.pkg_languages.insert(f.clone(), language);
                    out.push((n, v, f));
                }
            }
        }
        self.pkg_files = Some(out);
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
        self.scan_packages();
        let mut cand: Option<(String, String, String)> = None;
        if let Some(path) = &decl.path {
            let target = join(&base.unwrap_or_else(|| self.file_dir(decl.unit)), path);
            if let Some(text) = self.provider.read(&target) {
                if let Some((n, v)) = scan_package_decl(&text) {
                    if n == decl.name && version_ok(decl.version.as_deref(), &v) {
                        if let Some(want) = &decl.version {
                            if *want != v {
                                self.diag_in(
                                    Severity::Warning,
                                    format!("{path} is {kind} {} {v}, not {want}: a different version may draw differently (16.2)", decl.name),
                                    Some(loc.clone()),
                                    &pkg,
                                );
                            }
                        }
                        cand = Some((n, v, target));
                    }
                }
            }
            if cand.is_none() {
                self.diag_in(
                    Severity::Warning,
                    format!("the file named by path= does not exist or its @PACKAGE does not match; looking the package up by name ({sec})"),
                    Some(loc.clone()),
                    &pkg,
                );
            }
        }
        if cand.is_none() {
            let same: Vec<(String, String, String)> =
                self.pkg_files.as_ref().unwrap().iter().filter(|c| c.0 == decl.name).cloned().collect();
            if let Some(want) = &decl.version {
                if let Some(c) = same.iter().find(|c| c.1 == *want) {
                    cand = Some(c.clone());
                } else {
                    let mut newer: Vec<&(String, String, String)> = same
                        .iter()
                        .filter(|c| same_major(&c.1, want) && version_key(&c.1) > version_key(want))
                        .collect();
                    newer.sort_by_key(|c| version_key(&c.1));
                    if let Some(c) = newer.first() {
                        self.diag_in(
                            Severity::Warning,
                            format!("{kind} {} has no version {want}; using {}: a different version may draw differently (16.2)", decl.name, c.1),
                            Some(loc.clone()),
                            &pkg,
                        );
                        cand = Some((*c).clone());
                    }
                }
            } else if let Some(c) = same.iter().max_by_key(|c| version_key(&c.1)) {
                let this = if what == "reference" { "the reference to".to_string() } else { what.to_string() };
                self.diag_in(
                    Severity::Hint,
                    format!("{this} {} names no version; using the newest found, {} ({sec})", decl.name, c.1),
                    Some(loc.clone()),
                    &pkg,
                );
                cand = Some(c.clone());
            }
        }
        let Some((n, v, path)) = cand else {
            let then = match what {
                "USE-ALIASES" => "; statements using its aliases fail".to_string(),
                "reference" => format!(", and the statements of package {pkg} that use its names fail with it"),
                _ => String::new(),
            };
            let ver = decl.version.as_ref().map(|v| format!(" {v}")).unwrap_or_default();
            self.diag_in(
                Severity::Error,
                format!("{kind} {}{ver} cannot be found: this {what} fails{then} ({sec})", decl.name),
                Some(loc),
                &pkg,
            );
            return None;
        };
        if let Some(&u) = self.packages.get(&(n.clone(), v.clone())) {
            return Some(u);
        }
        Some(self.load_package_file(&path, Some((n, v))))
    }

    /// Read a package file into a unit (2.7). `key`, the name and version it is found by, holds it
    /// before its header record is read, so that a reference back to it finds it (16.2).
    pub(crate) fn load_package_file(&mut self, path: &str, key: Option<(String, String)>) -> UnitId {
        let text = self.provider.read(path).unwrap_or_default();
        let fname = file_name(path).to_string();
        let mut lv = Level::new(LevelKind::Package, &fname, &fname, normalize(path));
        let recs = scan_run(&package_lines(&text), &fname, &mut self.sink);
        lv.records = recs.clone();
        let pl = self.pkg_levels.len();
        self.pkg_levels.push(lv);
        // files of one name in two package directories, such as rnl-std.rnl of two standard series,
        // are two sources: the revision of a snapshot counts the text of every file read
        if !self.pkg_sources.iter().any(|(f, t)| *f == fname && *t == text) {
            self.pkg_sources.push((fname.clone(), text));
        }
        let mut u = Unit::new(UnitKind::Package, &fname);
        u.pkg_level = Some(pl);
        let id = self.units.len();
        self.units.push(u);
        if let Some(key) = key {
            self.packages.insert(key, id);
        }
        let rec_ids: Vec<RecId> = recs.into_iter().map(|r| self.add_record(r, id)).collect();
        self.unit_recs.push(rec_ids);
        self.read_unit(id);
        if self.units[id].header_record.is_none() || self.units[id].pkg_name.is_empty() {
            self.sink.error(
                format!("{fname} has no @PACKAGE and is not a package (2.7)"),
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
            if standard_original(&orig) {
                continue;
            }
            let what = if orig.starts_with('@') { "standard keyword" } else { "standard word" };
            self.diag_in(Severity::Error, format!("the original {orig} is not a {what} (16.5)"), Some(loc), &pkg);
            self.units[u].aliases.remove(&alias);
            self.units[u].alias_decls.retain(|d| !(d.alias == alias && d.definition.is_none()));
        }
    }

    fn read_embedded(&mut self, u: UnitId, body: Rc<SetExpr>, rec: Option<RecId>) {
        let header = Rc::new(Header::default());
        for (k, m) in body.members.iter().enumerate() {
            if let Some(err) = &m.error {
                let loc = self.rec_loc(rec, m.error_pos);
                self.sink.error(err.clone(), loc);
                self.note_failed(rec, m.start, m.end, false);
                continue;
            }
            // an inline package has no header record, so it references no other package (16.2)
            let call = m.segments.first().and_then(|s| s.call.as_ref()).filter(|_| m.segments.len() == 1);
            if let Some(call) = call.filter(|c| self.kw(&c.kw, u) == "USE-PACKAGE") {
                let loc = self.rec_loc(rec, call.start);
                self.sink.error(
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

    /// Is no file on the way up from this level of an unsupported version (16.1)?
    pub(crate) fn file_ok(&self, level: LevelId) -> bool {
        self.unsupported_file(level).is_none()
    }

    /// The file level on the way up from this level, itself included, that is of an unsupported
    /// version (16.1), with the version it declares: no RNL of it is interpreted, nor of the
    /// levels in it.
    pub(crate) fn unsupported_file(&self, level: LevelId) -> Option<(LevelId, String)> {
        let mut lv = Some(level);
        while let Some(l) = lv {
            let unit = &self.units[self.level_unit[l]];
            if self.doc.levels[l].kind == LevelKind::File && !unit.ok {
                return Some((l, unit.version.clone()));
            }
            lv = self.doc.levels[l].parent;
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
                let Some(ks) = self.units[au].scope_index.get(&key).cloned() else { continue };
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

/// Is an alias original a standard keyword (written with `@`) or a standard word (16.1, 16.5)?
pub fn standard_original(orig: &str) -> bool {
    match orig.strip_prefix('@') {
        Some(k) => words().is_keyword(k),
        None => words().all_words().contains(orig),
    }
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
