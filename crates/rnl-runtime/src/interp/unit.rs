//! Units of RNL text: host levels and packages (spec-draft 2.7, 2.8, 6.1, 16.1–16.5).
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

pub const SUPPORTED_VERSION: &str = "0.2";
const HEADER_KW: [&str; 5] = ["VERSION", "PACKAGE", "ALIAS", "DEVICE", "USE-ALIASES"];

/// The fields of a record header that affect interpretation (2.8).
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub type_: String,
    pub priority: Option<Priority>,
    pub scope: Option<String>,
}

impl Default for Header {
    fn default() -> Header {
        Header { type_: "authored".into(), priority: None, scope: None }
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

/// A USE-PACKAGE or USE-ALIASES (16.2, 16.5).
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
    pub std: Option<UnitId>,
    pub ok: bool,
    pub scope_records: Vec<(RecId, Rc<RecordAst>, Rc<Header>)>,
    /// The indexes in `scope_records` by the key of the subcircuit named in scope=, in order.
    pub scope_index: HashMap<String, Vec<usize>>,
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
            std: None,
            ok: true,
            scope_records: Vec::new(),
            scope_index: HashMap::new(),
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

/// Where an effective package sits (8.3, 16.2, 16.3).
#[derive(Clone, Debug)]
pub enum Place {
    /// At the root: the standard package first, then the header USE-PACKAGEs.
    Root(usize),
    /// At a USE-PACKAGE in a level: the level and the (record, member) position.
    Level(LevelId, (usize, usize)),
}

#[derive(Clone, Debug)]
pub struct EffPackage {
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

fn version_key(v: &str) -> (i64, i64) {
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

    /// Parse a record once.
    pub(crate) fn parse(&mut self, rec: RecId) -> Rc<RecordAst> {
        if let Some(a) = &self.asts[rec] {
            return a.clone();
        }
        let a = Rc::new(syntax::parse_record(&self.recs[rec].text));
        self.asts[rec] = Some(a.clone());
        a
    }

    pub(crate) fn add_record(&mut self, r: crate::host::Record) -> RecId {
        self.recs.push(r);
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
            if header.type_ == "header" {
                if !is_file_level || index != 0 || self.units[u].header_record.is_some() {
                    let loc = self.recs[rec].loc(0);
                    self.diag_in(
                        Severity::Error,
                        "a header record is the first record of a file, at most one (2.8); this one is ignored",
                        Some(loc),
                        &pkg,
                    );
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
                format!("the main file has no header record: it is read in the tool's default version {SUPPORTED_VERSION}, and tools add the header when they write the file (16.1)"),
                Some(Loc::line(file, 2)),
            );
        }
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
        for n in &params.named {
            let loc = self.recs[rec].loc(n.off);
            let Some(spec) = words().header_field(&n.name) else {
                let close = closest(&n.name, words().header_fields.iter().map(|f| f.name.as_str()));
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
                "scope" => h.scope = Some(v.clone()),
                _ => {}
            }
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
                "VERSION" => {
                    let Some(Value::Str(v)) = p.get("version") else {
                        self.diag_in(Severity::Error, "@VERSION takes a string (16.1)", Some(loc), &pkg);
                        continue;
                    };
                    self.units[u].version = v.clone();
                    if version_key(v).0 < 0 || v != SUPPORTED_VERSION {
                        self.diag_in(
                            Severity::Error,
                            format!("unsupported version {v}: no RNL of this file is interpreted (16.1; this implementation reads {SUPPORTED_VERSION})"),
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
                    self.units[u].pkg_name = str_val(p.get("name")).unwrap_or_default();
                    self.units[u].pkg_version = str_val(p.get("version")).unwrap_or_default();
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

    /// The language version a unit's file is read in (16.1).
    pub(crate) fn version_of(&self, u: Option<UnitId>) -> String {
        let fu = u.and_then(|u| self.file_unit(u));
        match fu {
            Some(f) if !self.units[f].version.is_empty() => self.units[f].version.clone(),
            _ => SUPPORTED_VERSION.to_string(),
        }
    }

    fn use_decl(&mut self, u: UnitId, rec: Option<RecId>, k: usize, seg: &syntax::Segment, in_header: bool) {
        let call = seg.call.as_ref().unwrap();
        let loc = self.rec_loc(rec, call.start).unwrap();
        let pkg = self.units[u].pkg().to_string();
        if self.units[u].kind == UnitKind::Package {
            self.diag_in(Severity::Error, "a package cannot USE-PACKAGE in this version (16.2)", Some(loc), &pkg);
            return;
        }
        let Some(p) = self.header_params("USE-PACKAGE", call.params.as_ref(), u, &loc, in_header) else { return };
        let Some(name) = str_val(p.get("name")) else {
            self.sink.error("@USE-PACKAGE takes name (16.2)", Some(loc));
            return;
        };
        let mut body = None;
        if !seg.args.is_empty() {
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
        decl.pkg = self.resolve_package(&decl, loc.clone(), None, "USE-PACKAGE");
        if let (Some(pk), None) = (decl.pkg, &body) {
            if is_alias_package(&self.units[pk]) && self.units[pk].header_kws.iter().any(|k| k == "ALIAS") {
                self.sink.error(
                    format!("package {name} is an alias package; load it with @USE-ALIASES(name=\"{name}\") in the header record. This USE-PACKAGE fails (16.5)"),
                    Some(loc),
                );
                decl.pkg = None;
            }
        }
        self.units[u].uses.push(decl);
    }

    // ------------------------------------------------------------------ packages (16.2)

    fn scan_packages(&mut self) {
        if self.pkg_files.is_some() {
            return;
        }
        let mut out = Vec::new();
        for f in self.provider.package_files() {
            if let Some(text) = self.provider.read(&f) {
                if let Some((n, v)) = scan_package_decl(&text) {
                    out.push((n, v, f));
                }
            }
        }
        self.pkg_files = Some(out);
    }

    /// Find a package by path= or in the package directories (16.2). path= is relative to the
    /// directory of the file that writes the USE-PACKAGE or USE-ALIASES.
    pub(crate) fn resolve_package(
        &mut self,
        decl: &UseDecl,
        loc: Loc,
        base: Option<String>,
        what: &str,
    ) -> Option<UnitId> {
        let sec = if what == "USE-ALIASES" { "16.5" } else { "16.2" };
        let kind = if what == "USE-ALIASES" { "alias package" } else { "package" };
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
                                self.sink.warning(
                                    format!("{path} is {kind} {} {v}, not {want}: a different version may draw differently (16.2)", decl.name),
                                    Some(loc.clone()),
                                );
                            }
                        }
                        cand = Some((n, v, target));
                    }
                }
            }
            if cand.is_none() {
                self.sink.warning(format!("the file named by path= does not exist or its @PACKAGE does not match; looking the package up by name ({sec})"), Some(loc.clone()));
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
                        self.sink.warning(
                            format!("{kind} {} has no version {want}; using {}: a different version may draw differently (16.2)", decl.name, c.1),
                            Some(loc.clone()),
                        );
                        cand = Some((*c).clone());
                    }
                }
            } else if let Some(c) = same.iter().max_by_key(|c| version_key(&c.1)) {
                self.sink.hint(
                    format!("{what} {} names no version; using the newest found, {} ({sec})", decl.name, c.1),
                    Some(loc.clone()),
                );
                cand = Some(c.clone());
            }
        }
        let Some((n, v, path)) = cand else {
            let then = if what == "USE-ALIASES" { "; statements using its aliases fail" } else { "" };
            let ver = decl.version.as_ref().map(|v| format!(" {v}")).unwrap_or_default();
            self.sink.error(
                format!("{kind} {}{ver} cannot be found: this {what} fails{then} ({sec})", decl.name),
                Some(loc),
            );
            return None;
        };
        if let Some(&u) = self.packages.get(&(n.clone(), v.clone())) {
            return Some(u);
        }
        let u = self.load_package_file(&path);
        self.packages.insert((n, v), u);
        Some(u)
    }

    /// Read a package file into a unit (2.7).
    pub(crate) fn load_package_file(&mut self, path: &str) -> UnitId {
        let text = self.provider.read(path).unwrap_or_default();
        let fname = file_name(path).to_string();
        let mut lv = Level::new(LevelKind::Package, &fname, &fname, normalize(path));
        let recs = scan_run(&package_lines(&text), &fname, &mut self.sink);
        lv.records = recs.clone();
        let pl = self.pkg_levels.len();
        self.pkg_levels.push(lv);
        if !self.pkg_sources.iter().any(|(f, _)| *f == fname) {
            self.pkg_sources.push((fname.clone(), text));
        }
        let mut u = Unit::new(UnitKind::Package, &fname);
        u.pkg_level = Some(pl);
        let id = self.units.len();
        self.units.push(u);
        let rec_ids: Vec<RecId> = recs.into_iter().map(|r| self.add_record(r)).collect();
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
            self.units[u].members.push(UnitMember { rec, k, embedded: true, m: m.clone(), header: header.clone() });
            self.declare(u, rec, k, true, m, &header);
        }
    }

    /// The standard package: the newest `rnl-std` in the package directories (16.3).
    pub(crate) fn std_package(&mut self) -> Option<UnitId> {
        if !self.std_enabled {
            return None;
        }
        if let Some(s) = self.std_cache {
            return s;
        }
        self.scan_packages();
        let same: Vec<(String, String, String)> =
            self.pkg_files.as_ref().unwrap().iter().filter(|c| c.0 == "rnl-std").cloned().collect();
        let mut out = None;
        if let Some(c) = same.iter().max_by_key(|c| version_key(&c.1)) {
            let key = (c.0.clone(), c.1.clone());
            let u = match self.packages.get(&key) {
                Some(&u) => u,
                None => {
                    let u = self.load_package_file(&c.2);
                    self.packages.insert(key, u);
                    u
                }
            };
            out = Some(u);
        }
        self.std_cache = Some(out);
        out
    }

    /// The packages in effect at a host level, in queue order (8.3, 16.2, 16.3).
    pub(crate) fn effective_packages(&mut self, level: LevelId) -> Vec<EffPackage> {
        let mut chain = self.doc.ancestors(level);
        chain.reverse();
        chain.push(level);
        let mut eff: Vec<(String, EffPackage)> = Vec::new();
        let mut root_order: Vec<String> = Vec::new();
        let put = |eff: &mut Vec<(String, EffPackage)>, name: &str, e: EffPackage| {
            if let Some(slot) = eff.iter_mut().find(|(n, _)| n == name) {
                slot.1 = e;
            } else {
                eff.push((name.to_string(), e));
            }
        };
        let std = if self.file_ok(level) { self.std_package() } else { None };
        if let Some(s) = std {
            put(&mut eff, "rnl-std", EffPackage { decl: None, unit: s, place: Place::Root(0) });
            root_order.push("rnl-std".into());
        }
        for lv in chain {
            let u = self.level_unit[lv];
            if !self.units[u].ok {
                continue;
            }
            let uses = self.units[u].uses.clone();
            for d in uses.iter().filter(|d| d.in_header) {
                if !root_order.contains(&d.name) {
                    root_order.push(d.name.clone());
                }
                let k = root_order.iter().position(|n| *n == d.name).unwrap();
                if let Some(pk) = d.pkg {
                    put(&mut eff, &d.name, EffPackage { decl: Some(d.clone()), unit: pk, place: Place::Root(k) });
                } else {
                    eff.retain(|(n, _)| *n != d.name);
                }
            }
            for d in uses.iter().filter(|d| !d.in_header) {
                if let Some(pk) = d.pkg {
                    put(
                        &mut eff,
                        &d.name,
                        EffPackage { decl: Some(d.clone()), unit: pk, place: Place::Level(lv, d.pos) },
                    );
                } else {
                    eff.retain(|(n, _)| *n != d.name);
                }
            }
        }
        eff.into_iter().map(|(_, e)| e).filter(|e| self.units[e.unit].ok).collect()
    }

    /// DEVICE entries in effect at a level; later ones win (6.12).
    pub(crate) fn devices_for(&mut self, level: LevelId) -> Vec<DeviceDecl> {
        let mut out = Vec::new();
        for e in self.effective_packages(level) {
            out.extend(self.units[e.unit].devices.clone());
        }
        let mut chain = self.doc.ancestors(level);
        chain.reverse();
        chain.push(level);
        for lv in chain {
            if self.doc.levels[lv].kind == LevelKind::File {
                out.extend(self.units[self.level_unit[lv]].devices.clone());
            }
        }
        out
    }

    /// Is no file on the way up from this level of an unsupported version (16.1)?
    pub(crate) fn file_ok(&self, level: LevelId) -> bool {
        let mut lv = Some(level);
        while let Some(l) = lv {
            if self.doc.levels[l].kind == LevelKind::File && !self.units[self.level_unit[l]].ok {
                return false;
            }
            lv = self.doc.levels[l].parent;
        }
        true
    }

    // ------------------------------------------------------------------ names (6.6)

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
