//! Parser for one record (spec-draft chapters 3 and 4).
//!
//! The parser is scannerless: free text (3.3) and comments (3.4) are skipped between formal
//! tokens. A syntax error makes the current member fail (7.4); the parser resumes at the next `;`
//! or at the end of the enclosing set. Offsets are character indices into the record text.

use std::collections::HashMap;

pub type Off = usize;

/// A node id, unique inside one record. Together with the record it identifies a member, segment
/// or reference for memoisation.
pub type NodeId = u32;

const NOT_FREE: &str = "@#&%$?\";={}()[]<>";
/// Glued to a reference these are probably part of a host name that needs quotes, such as `vdd!` (3.3).
const GLUED_NOTE: &str = "-!'+/*\\^~|";

pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_ascii_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// A name in role parentheses or at a parameter-name position: ASCII letters, digits, `_` and any
/// non-ASCII character (3.6, 16.5).
pub fn is_name_char(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric() || !c.is_ascii()
}

// ---------------------------------------------------------------------------- AST

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Num { value: f64, text: String },
    Str(String),
    Bool(bool),
    Arr(Vec<Value>),
    /// `?name` in a literal position (6.10).
    Hole(String),
    /// An unquoted word: an alias of `true` or `false`, or an error (enumerations are strings).
    Word(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Coord {
    Num(f64),
    /// `_`: the axis is free (3.8).
    Free,
    Hole(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SubItem {
    One(u64),
    Range(u64, u64, u64),
}

/// A subscript `<…>` or `[…]` after a host name (6.11).
#[derive(Clone, Debug, PartialEq)]
pub struct Sub {
    pub bracket: char,
    pub items: Vec<SubItem>,
}

impl Sub {
    /// The one number of a subscript that names a single host name, such as `<3>`.
    pub fn single(&self) -> Option<u64> {
        match self.items.as_slice() {
            [SubItem::One(k)] => Some(*k),
            _ => None,
        }
    }

    /// The numbers a subscript expands to, in the written order (6.11).
    pub fn expand(&self) -> Vec<u64> {
        let mut out = Vec::new();
        for it in &self.items {
            match *it {
                SubItem::One(k) => out.push(k),
                SubItem::Range(a, b, s) => {
                    if b >= a {
                        let mut k = a;
                        while k <= b {
                            out.push(k);
                            k += s;
                        }
                    } else {
                        let mut k = a as i64;
                        while k >= b as i64 {
                            out.push(k as u64);
                            k -= s as i64;
                        }
                    }
                }
            }
        }
        out
    }

    pub fn text(&self, k: u64) -> String {
        if self.bracket == '<' {
            format!("<{k}>")
        } else {
            format!("[{k}]")
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sep {
    /// `:` a terminal (6.4).
    Term,
    /// `::` a sub-object (6.5).
    Sub,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathItem {
    pub sep: Sep,
    pub name: String,
    pub quoted: bool,
    pub sub: Option<Sub>,
    pub number: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ref {
    pub id: NodeId,
    /// `#`, `&`, `%`, `$` or `?`.
    pub prefix: char,
    /// None: a bare `#` (this level) or a bare `?` (the symbol's owner).
    pub name: Option<String>,
    pub quoted: bool,
    pub sub: Option<Sub>,
    pub path: Vec<PathItem>,
    pub start: Off,
    pub end: Off,
    /// The spelling as written when a `$` alias was read as its original (16.5).
    pub written: Option<String>,
}

impl Ref {
    pub fn text(&self) -> String {
        let mut s = String::new();
        s.push(self.prefix);
        if let Some(n) = &self.name {
            s.push_str(n);
        }
        if self.sub.is_some() {
            s.push_str("<…>");
        }
        for p in &self.path {
            s.push_str(if p.sep == Sep::Term { ":" } else { "::" });
            s.push_str(&p.name);
        }
        s
    }

    /// The bare `#`: this level (6.3).
    pub fn is_bare_level(&self) -> bool {
        self.prefix == '#' && self.name.is_none() && self.path.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub x: Coord,
    pub y: Coord,
    pub start: Off,
    pub end: Off,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetExpr {
    pub id: NodeId,
    pub members: Vec<Member>,
    pub start: Off,
    pub end: Off,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObjExpr {
    Ref(Ref),
    Point(Point),
    Set(SetExpr),
}

impl ObjExpr {
    pub fn start(&self) -> Off {
        match self {
            ObjExpr::Ref(r) => r.start,
            ObjExpr::Point(p) => p.start,
            ObjExpr::Set(s) => s.start,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    pub obj: ObjExpr,
    pub role: Option<String>,
    /// A role written in quotes is always the name itself, never an alias (16.5).
    pub role_quoted: bool,
    pub start: Off,
    pub end: Off,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Named {
    pub name: String,
    pub value: Value,
    pub quoted: bool,
    pub off: Off,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Params {
    pub positional: Vec<Value>,
    pub named: Vec<Named>,
    pub start: Off,
    pub end: Off,
}

impl Params {
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.named.iter().find(|n| n.name == name).map(|n| &n.value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    pub kw: String,
    pub params: Option<Params>,
    pub start: Off,
    pub end: Off,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub id: NodeId,
    pub args: Vec<Arg>,
    pub call: Option<Call>,
    pub start: Off,
    pub end: Off,
    /// Not the first segment of its member.
    pub piped: bool,
}

/// A name bound by `%x =` (first token of a member) or `=> %x` (after a segment), 4.2 rules 3 and 8.
#[derive(Clone, Debug, PartialEq)]
pub struct Bind {
    pub prefix: char,
    pub name: String,
    pub off: Off,
    /// The index of the segment whose result the name binds.
    pub after: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub id: NodeId,
    pub binds: Vec<Bind>,
    pub segments: Vec<Segment>,
    pub start: Off,
    pub end: Off,
    pub error: Option<String>,
    pub error_pos: Off,
    /// Free-text words with their offsets, for the recommended warnings of 3.3.
    pub free_words: Vec<(String, Off)>,
    /// The spelling of `$` names bound here when a `$` alias was read as its original (16.5).
    pub written: HashMap<String, String>,
}

impl Member {
    /// The last name of the member.
    pub fn bind(&self) -> Option<&Bind> {
        self.binds.last()
    }

    pub fn binds_after(&self, k: usize) -> impl Iterator<Item = &Bind> {
        self.binds.iter().filter(move |b| b.after == k)
    }

    /// Where the member's text begins: its first name or segment, not the free text before it.
    pub fn text_start(&self) -> Off {
        let first = self.segments.first().map(|s| s.start).unwrap_or(self.start);
        self.binds.iter().map(|b| b.off).chain(std::iter::once(first)).min().unwrap_or(first)
    }
}

/// A warning found while reading, with the keyword it concerns when there is one.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub off: Off,
    pub message: String,
    pub kw: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordAst {
    pub header: Option<Params>,
    pub body: Option<SetExpr>,
    pub error: Option<(String, Off)>,
    pub notes: Vec<Note>,
}

#[derive(Debug)]
struct SyntaxErr {
    msg: String,
    pos: Off,
    /// "Expected a …": a point replaces it with its own hint.
    expected: bool,
}

type PResult<T> = Result<T, SyntaxErr>;

// ---------------------------------------------------------------------------- parser

struct Parser {
    t: Vec<char>,
    i: usize,
    notes: Vec<Note>,
    next_id: NodeId,
}

impl Parser {
    fn new(text: &str) -> Parser {
        Parser { t: text.chars().collect(), i: 0, notes: Vec::new(), next_id: 0 }
    }

    fn id(&mut self) -> NodeId {
        self.next_id += 1;
        self.next_id
    }

    fn n(&self) -> usize {
        self.t.len()
    }

    fn peek(&self) -> Option<char> {
        self.t.get(self.i).copied()
    }

    fn peek_at(&self, k: usize) -> Option<char> {
        self.t.get(self.i + k).copied()
    }

    fn starts_with(&self, s: &str) -> bool {
        self.starts_with_at(self.i, s)
    }

    fn starts_with_at(&self, at: usize, s: &str) -> bool {
        let mut j = at;
        for c in s.chars() {
            if self.t.get(j) != Some(&c) {
                return false;
            }
            j += 1;
        }
        true
    }

    fn peek_in(&self, set: &str) -> bool {
        matches!(self.peek(), Some(c) if set.contains(c))
    }

    fn at_end_of_member(&self) -> bool {
        matches!(self.peek(), None | Some(';') | Some('}'))
    }

    fn err<T>(&self, msg: impl Into<String>) -> PResult<T> {
        Err(SyntaxErr { msg: msg.into(), pos: self.i, expected: false })
    }

    fn err_at<T>(&self, msg: impl Into<String>, pos: Off) -> PResult<T> {
        Err(SyntaxErr { msg: msg.into(), pos, expected: false })
    }

    fn expected<T>(&self, msg: impl Into<String>) -> PResult<T> {
        Err(SyntaxErr { msg: msg.into(), pos: self.i, expected: true })
    }

    // ---------------- low level

    /// Whitespace and comments only (3.4, 3.5).
    fn skip_ws(&mut self) -> PResult<()> {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                self.i += 1;
            } else if self.starts_with("/*") {
                match self.find("*/", self.i + 2) {
                    Some(j) => self.i = j + 2,
                    None => return self.err("comment not closed"),
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    fn find(&self, s: &str, from: usize) -> Option<usize> {
        (from..self.n()).find(|&j| self.starts_with_at(j, s))
    }

    /// Free text (3.3), whitespace and comments, outside brackets.
    fn skip_free(&mut self, mut words: Option<&mut Vec<(String, Off)>>) -> PResult<()> {
        while self.i < self.n() {
            self.skip_ws()?;
            let Some(c) = self.peek() else { return Ok(()) };
            if is_word_char(c) {
                let j0 = self.i;
                let mut j = self.i;
                while j < self.n() && is_word_char(self.t[j]) {
                    j += 1;
                }
                if let Some(w) = words.as_deref_mut() {
                    w.push((self.t[j0..j].iter().collect(), j0));
                }
                self.i = j;
            } else if NOT_FREE.contains(c) {
                return Ok(());
            } else {
                if c == '|' && words.is_some() {
                    self.notes.push(Note {
                        off: self.i,
                        message: "`|` in free text: a pipe is written `=>` (4.2 rule 3; it was `|` before round D)".into(),
                        kw: None,
                    });
                }
                self.i += 1;
            }
        }
        Ok(())
    }

    // ---------------- record

    fn record(&mut self) -> RecordAst {
        debug_assert!(self.starts_with("@RNL"));
        self.i = 4;
        let mut header = None;
        let r = (|| -> PResult<SetExpr> {
            self.skip_ws()?;
            if self.peek() == Some('(') {
                header = Some(self.params()?);
            }
            self.skip_ws()?;
            if self.peek() != Some('{') {
                return self.err("the record has no braces: write @RNL {…} (2.4)");
            }
            self.set_expr()
        })();
        let notes = std::mem::take(&mut self.notes);
        match r {
            Ok(body) => RecordAst { header, body: Some(body), error: None, notes },
            Err(e) => RecordAst { header, body: None, error: Some((e.msg, e.pos)), notes },
        }
    }

    // ---------------- sets and members

    fn set_expr(&mut self) -> PResult<SetExpr> {
        let start = self.i;
        let id = self.id();
        self.i += 1; // `{`
        let mut members = Vec::new();
        loop {
            if let Some(m) = self.member() {
                members.push(m);
            }
            self.skip_ws()?;
            match self.peek() {
                Some(';') => {
                    self.i += 1;
                }
                Some('}') => {
                    self.i += 1;
                    return Ok(SetExpr { id, members, start, end: self.i });
                }
                None => return self.err("the set is not closed (missing `}`)"),
                // a member stopped on something unexpected: recover
                Some(_) => self.i += 1,
            }
        }
    }

    fn member(&mut self) -> Option<Member> {
        let start = self.i;
        let id = self.id();
        let mut words = Vec::new();
        let r = self.member_body(&mut words);
        match r {
            Ok(None) => None,
            Ok(Some((binds, segments))) => {
                let m = Member {
                    id,
                    binds,
                    segments,
                    start,
                    end: self.i,
                    error: None,
                    error_pos: 0,
                    free_words: words,
                    written: HashMap::new(),
                };
                match check_member(&m) {
                    Ok(()) => Some(m),
                    Err(e) => {
                        self.recover();
                        Some(failed_member(id, start, self.i, e, m.free_words))
                    }
                }
            }
            Err(e) => {
                self.recover();
                Some(failed_member(id, start, self.i, e, words))
            }
        }
    }

    #[allow(clippy::type_complexity)]
    fn member_body(&mut self, words: &mut Vec<(String, Off)>) -> PResult<Option<(Vec<Bind>, Vec<Segment>)>> {
        self.skip_free(Some(words))?;
        if self.at_end_of_member() {
            return Ok(None);
        }
        let mut binds = Vec::new();
        let mut front = None;
        if self.peek_in("%$") && self.is_binding() {
            // `%x = …` binds the last handle of the member (4.2 rule 8, 5.5)
            let prefix = self.peek().unwrap();
            let off = self.i;
            self.i += 1;
            let (name, _q) = self.rnl_name()?;
            self.skip_ws()?;
            self.i += 1; // `=`
            front = Some((prefix, name, off));
        }
        let mut segments: Vec<Segment> = Vec::new();
        let mut piped = false;
        loop {
            let seg = self.segment(piped, words)?;
            segments.push(seg);
            self.skip_free(Some(words))?;
            let mut next_segment = false;
            while self.starts_with("=>") {
                self.i += 2;
                self.skip_free(Some(words))?;
                if self.peek_in("%$") && self.naming_ahead() {
                    let prefix = self.peek().unwrap();
                    let off = self.i;
                    self.i += 1;
                    let (name, _q) = self.rnl_name()?;
                    binds.push(Bind { prefix, name, off, after: segments.len() - 1 });
                    self.skip_free(Some(words))?;
                    continue;
                }
                next_segment = true;
                break;
            }
            if next_segment {
                piped = true;
                continue;
            }
            if self.at_end_of_member() {
                break;
            }
            let c = self.peek().unwrap();
            return self.err(format!("`{c}` cannot appear here"));
        }
        if let Some((prefix, name, off)) = front {
            binds.insert(0, Bind { prefix, name, off, after: segments.len() - 1 });
        }
        Ok(Some((binds, segments)))
    }

    /// At `%` or `$` after `=>`: a single name followed by `=>`, `;`, `}` or the end of the member
    /// is a naming; anything else is the next segment (4.2 rule 3, round D W1).
    fn naming_ahead(&mut self) -> bool {
        let save = self.i;
        let r = (|| -> PResult<bool> {
            self.i += 1;
            match self.peek() {
                Some('"') => {}
                Some(c) if is_ascii_word(c) => {}
                _ => return Ok(false),
            }
            self.rnl_name()?;
            if self.peek_in(":<[(") {
                return Ok(false);
            }
            self.skip_free(None)?;
            Ok(self.at_end_of_member() || self.starts_with("=>"))
        })();
        self.i = save;
        r.unwrap_or(false)
    }

    fn is_binding(&self) -> bool {
        let n = self.n();
        let mut j = self.i + 1;
        if j < n && self.t[j] == '"' {
            j += 1;
            while j < n {
                if self.t[j] == '"' {
                    if j + 1 < n && self.t[j + 1] == '"' {
                        j += 2;
                        continue;
                    }
                    break;
                }
                j += 1;
            }
            j += 1;
        } else {
            let k = j;
            while j < n && is_ascii_word(self.t[j]) {
                j += 1;
            }
            if j == k {
                return false;
            }
        }
        while j < n && matches!(self.t[j], ' ' | '\t' | '\n') {
            j += 1;
        }
        j < n && self.t[j] == '=' && !(j + 1 < n && matches!(self.t[j + 1], '=' | '>'))
    }

    fn recover(&mut self) {
        let mut depth = 0usize;
        while self.i < self.n() {
            let c = self.t[self.i];
            if c == '"' {
                let mut j = self.i + 1;
                loop {
                    match self.t.get(j) {
                        None => {
                            j = self.n();
                            break;
                        }
                        Some('"') if self.t.get(j + 1) == Some(&'"') => j += 2,
                        Some('"') => {
                            j += 1;
                            break;
                        }
                        _ => j += 1,
                    }
                }
                self.i = j;
                continue;
            }
            if self.starts_with("/*") {
                self.i = self.find("*/", self.i + 2).map(|j| j + 2).unwrap_or(self.n());
                continue;
            }
            if c == '{' || c == '(' {
                depth += 1;
            } else if c == ')' && depth == 0 {
                // a stray `)` belongs to the failed member
            } else if c == '}' || c == ')' {
                if depth == 0 {
                    return;
                }
                depth -= 1;
            } else if c == ';' && depth == 0 {
                return;
            }
            self.i += 1;
        }
    }

    fn segment(&mut self, piped: bool, words: &mut Vec<(String, Off)>) -> PResult<Segment> {
        let start = self.i;
        let id = self.id();
        let mut args = Vec::new();
        let mut call: Option<Call> = None;
        let mut last_end: Option<usize> = None;
        loop {
            let gap_start = self.i;
            self.skip_free(Some(words))?;
            let Some(c) = self.peek() else { break };
            if c == ';' || c == '}' || self.starts_with("=>") {
                break;
            }
            if c == '@' {
                if self.i == 0 || self.t[self.i - 1] != ' ' {
                    return self.err("the `@` of a keyword must follow a space (2.3)");
                }
                if call.is_some() {
                    return self.err("at most one keyword in a segment (4.2 rule 2)");
                }
                let cl = self.call()?;
                last_end = Some(cl.end);
                call = Some(cl);
                continue;
            }
            if "#&%$?({".contains(c) {
                if c == '(' {
                    if let Some(cl) = &call {
                        let gap: String = self.t[gap_start..self.i].iter().collect();
                        if cl.params.is_none() && last_end == Some(cl.end) && gap.trim().is_empty() {
                            // carries the keyword: after AS, ATTACH a point is an ordinary object (`$GND @ATTACH (0, 0)`)
                            self.notes.push(Note {
                                off: self.i,
                                message: format!(
                                    "@{} and the parenthesis after it are separated by whitespace: these parentheses are a point, not parameters; parameter parentheses touch the keyword (3.5)",
                                    cl.kw
                                ),
                                kw: Some(cl.kw.clone()),
                            });
                        }
                    }
                }
                let a = self.arg()?;
                last_end = Some(a.end);
                args.push(a);
                continue;
            }
            if c == '<' {
                return self.err("a point is written `(x, y)` (3.8; it was `<x, y>` before round D)");
            }
            if c == '=' {
                return self.err(
                    "`=` is only used in a naming `%name =` at the start of a member and in parameters; name in a pipe with `=> %name` (4.2 rule 8)",
                );
            }
            return self.err(format!("`{c}` is neither free text nor a formal token (3.3)"));
        }
        if args.is_empty() && call.is_none() {
            return self.err_at("a segment cannot be empty (4.2 rule 1)", start);
        }
        Ok(Segment { id, args, call, start, end: self.i, piped })
    }

    // ---------------- objects

    fn arg(&mut self) -> PResult<Arg> {
        let start = self.i;
        let obj = self.obj()?;
        let end = self.i;
        // role parentheses touch the object; parentheses after a space start a point (3.5)
        if self.peek() == Some('(') {
            self.i += 1;
            self.skip_ws()?;
            let (role, quoted) = if self.peek() == Some('"') {
                (self.string()?, true)
            } else {
                if matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                    return self.err("an unquoted role name cannot start with a digit (3.6)");
                }
                let w = self.name_word();
                if w.is_empty() {
                    return self.err("write a role name in the role parentheses");
                }
                (w, false)
            };
            self.skip_ws()?;
            if self.peek() != Some(')') {
                return self.err("the role parentheses are not closed");
            }
            self.i += 1;
            return Ok(Arg { obj, role: Some(role), role_quoted: quoted, start, end: self.i });
        }
        if let ObjExpr::Ref(_) = obj {
            // recommended warning: `#M1.d` (3.3)
            if self.peek() == Some('.') && matches!(self.peek_at(1), Some(c) if is_word_char(c) && c != '_') {
                self.notes.push(Note {
                    off: self.i,
                    message: "a reference followed by `.` and a name: terminals are written with `:`; this `.…` is free text (3.3)".into(),
                    kw: None,
                });
            }
            // a character glued to a reference, such as `&vdd!`: part of a name that needs quotes (3.3, TR4)
            if let Some(c) = self.peek() {
                if GLUED_NOTE.contains(c) {
                    self.notes.push(Note {
                        off: self.i,
                        message: format!(
                            "a reference followed by `{c}`: a name with this character is written in quotes, such as `&\"vdd!\"`; here the `{c}` is free text (3.3)"
                        ),
                        kw: None,
                    });
                }
            }
        }
        Ok(Arg { obj, role: None, role_quoted: false, start, end })
    }

    fn obj(&mut self) -> PResult<ObjExpr> {
        match self.peek() {
            Some('{') => Ok(ObjExpr::Set(self.set_expr()?)),
            Some('(') => Ok(ObjExpr::Point(self.point()?)),
            Some(c) if "#&%$?".contains(c) => Ok(ObjExpr::Ref(self.reference()?)),
            _ => self.expected("expected an object here"),
        }
    }

    fn ascii_name(&mut self) -> Option<String> {
        let j0 = self.i;
        while self.i < self.n() && is_ascii_word(self.t[self.i]) {
            self.i += 1;
        }
        if self.i == j0 {
            None
        } else {
            Some(self.t[j0..self.i].iter().collect())
        }
    }

    fn reference(&mut self) -> PResult<Ref> {
        let start = self.i;
        let id = self.id();
        let prefix = self.peek().unwrap();
        self.i += 1;
        let mut name = None;
        let mut quoted = false;
        let mut sub = None;
        match self.peek() {
            Some('"') => {
                name = Some(self.string()?);
                quoted = true;
            }
            Some(c) if is_ascii_word(c) => name = self.ascii_name(),
            _ if prefix == '#' || prefix == '?' => {}
            _ => return self.err(format!("write a name after `{prefix}`")),
        }
        if name.is_some() && self.peek_in("<[") {
            if prefix == '#' || prefix == '&' {
                sub = Some(self.subscript()?);
            } else {
                return self.err("RNL names take no subscript (6.11); a point is written `(x, y)`");
            }
        }
        let mut path = Vec::new();
        loop {
            if self.starts_with("::") {
                self.i += 2;
                let (pn, pq) = if self.peek() == Some('"') {
                    (self.string()?, true)
                } else {
                    match self.ascii_name() {
                        Some(n) => (n, false),
                        None => return self.err("write a name after `::` (non-ASCII names and aliases in quotes, 16.5)"),
                    }
                };
                if self.peek_in("<[") {
                    return self.err("sub-object names take no subscript (6.11)");
                }
                path.push(PathItem { sep: Sep::Sub, name: pn, quoted: pq, sub: None, number: None });
                continue;
            }
            if self.peek() == Some(':') && self.peek_at(1) != Some(':') {
                match self.peek_at(1) {
                    Some('"') => {}
                    Some(c) if is_ascii_word(c) => {}
                    // a `:` without a terminal name after it is free text (3.3, TR4)
                    _ => break,
                }
                self.i += 1;
                let (pn, pq, num) = if self.peek() == Some('"') {
                    (self.string()?, true, None)
                } else {
                    let n = self.ascii_name().unwrap_or_default();
                    let num = if n.chars().all(|c| c.is_ascii_digit()) { n.parse::<u32>().ok() } else { None };
                    (n, false, num)
                };
                let mut psub = None;
                if self.peek_in("<[") {
                    if prefix == '#' {
                        psub = Some(self.subscript()?);
                    } else {
                        return self.err("usage terminal names take no subscript (6.11)");
                    }
                }
                path.push(PathItem { sep: Sep::Term, name: pn, quoted: pq, sub: psub, number: num });
                continue;
            }
            break;
        }
        let expansions = usize::from(sub.as_ref().is_some_and(|s| s.single().is_none()))
            + path.iter().filter(|p| p.sub.as_ref().is_some_and(|s| s.single().is_none())).count();
        if expansions > 1 {
            return self.err_at("only one subscript in a reference may expand to a group (6.11)", start);
        }
        Ok(Ref { id, prefix, name, quoted, sub, path, start, end: self.i, written: None })
    }

    fn number_u64(&mut self) -> Option<u64> {
        let j0 = self.i;
        while self.i < self.n() && self.t[self.i].is_ascii_digit() {
            self.i += 1;
        }
        if self.i == j0 {
            return None;
        }
        self.t[j0..self.i].iter().collect::<String>().parse().ok()
    }

    fn subscript(&mut self) -> PResult<Sub> {
        let bracket = self.peek().unwrap();
        let close = if bracket == '<' { '>' } else { ']' };
        self.i += 1;
        let mut items = Vec::new();
        loop {
            self.skip_ws()?;
            let mut nums = Vec::new();
            loop {
                match self.number_u64() {
                    Some(k) => nums.push(k),
                    None => return self.err("a subscript holds unsigned integers, `,` and `:` only (6.11)"),
                }
                self.skip_ws()?;
                if self.peek() == Some(':') {
                    self.i += 1;
                    self.skip_ws()?;
                    continue;
                }
                break;
            }
            if nums.len() > 3 || (nums.len() == 3 && nums[2] < 1) {
                return self.err("a subscript item is i, a:b or a:b:s with s at least 1 (6.11)");
            }
            items.push(match nums.as_slice() {
                [k] => SubItem::One(*k),
                [a, b] => SubItem::Range(*a, *b, 1),
                [a, b, s] => SubItem::Range(*a, *b, *s),
                _ => unreachable!(),
            });
            if self.peek() == Some(',') {
                self.i += 1;
                continue;
            }
            if self.peek() == Some(close) {
                self.i += 1;
                break;
            }
            return self.err("the subscript is not closed");
        }
        if self.peek_in("<[") {
            return self.err("a name takes one subscript only (6.11)");
        }
        Ok(Sub { bracket, items })
    }

    fn point(&mut self) -> PResult<Point> {
        let start = self.i;
        self.i += 1;
        let hint = "a point is written `(x, y)` (3.8); parameter parentheses touch the keyword and role parentheses touch the object, parentheses after whitespace are a point (3.5)";
        let r = (|| -> PResult<(Coord, Coord)> {
            let x = self.coord()?;
            self.skip_ws()?;
            if self.peek() != Some(',') {
                return self.err(hint);
            }
            self.i += 1;
            let y = self.coord()?;
            self.skip_ws()?;
            if self.peek() != Some(')') {
                return self.err(hint);
            }
            Ok((x, y))
        })();
        let (x, y) = match r {
            Ok(v) => v,
            Err(e) if e.expected => return Err(SyntaxErr { msg: hint.into(), pos: e.pos, expected: false }),
            Err(e) => return Err(e),
        };
        self.i += 1;
        Ok(Point { x, y, start, end: self.i })
    }

    fn coord(&mut self) -> PResult<Coord> {
        self.skip_ws()?;
        match self.peek() {
            Some('_') if !matches!(self.peek_at(1), Some(c) if is_word_char(c)) => {
                self.i += 1;
                Ok(Coord::Free)
            }
            Some('?') => {
                self.i += 1;
                let (name, _q) = self.rnl_name()?;
                Ok(Coord::Hole(name))
            }
            _ => {
                let (v, _t) = self.number()?;
                Ok(Coord::Num(v))
            }
        }
    }

    // ---------------- calls and parameters

    fn call(&mut self) -> PResult<Call> {
        let start = self.i;
        self.i += 1;
        let mut j = self.i;
        let n = self.n();
        while j < n && (is_word_char(self.t[j]) || (self.t[j] == '-' && j > self.i && j + 1 < n && is_word_char(self.t[j + 1]))) {
            j += 1;
        }
        if j == self.i {
            return self.err("write a keyword after `@`");
        }
        let kw: String = self.t[self.i..j].iter().collect();
        self.i = j;
        let params = if self.peek() == Some('(') { Some(self.params()?) } else { None };
        Ok(Call { kw, params, start, end: self.i })
    }

    fn params(&mut self) -> PResult<Params> {
        let start = self.i;
        self.i += 1;
        let mut p = Params { start, ..Params::default() };
        self.skip_ws()?;
        if self.peek() == Some(')') {
            self.i += 1;
            p.end = self.i;
            return Ok(p);
        }
        loop {
            self.skip_ws()?;
            let off = self.i;
            let mut name = None;
            let mut quoted = false;
            if self.peek() == Some('"') {
                let save = self.i;
                let s = self.string()?;
                self.skip_ws()?;
                if self.peek() == Some('=') {
                    name = Some(s);
                    quoted = true;
                    self.i += 1;
                } else {
                    self.i = save;
                }
            } else if matches!(self.peek(), Some(c) if is_name_char(c) && !c.is_ascii_digit()) {
                let save = self.i;
                let w = self.name_word();
                self.skip_ws()?;
                if self.peek() == Some('=') {
                    name = Some(w);
                    self.i += 1;
                } else {
                    self.i = save;
                }
            }
            let val = self.value()?;
            match name {
                None => {
                    // positional parameters come first; after the first `name=value` all are named (4.2 rule 9)
                    if !p.named.is_empty() {
                        return self.err_at(
                            "parameters written by position come before the named ones: after one name, every parameter is named (4.2 rule 9)",
                            off,
                        );
                    }
                    p.positional.push(val);
                }
                Some(name) => {
                    if p.named.iter().any(|x| x.name == name) {
                        return self.err_at(format!("parameter {name} is written twice (4.2 rule 9)"), off);
                    }
                    p.named.push(Named { name, value: val, quoted, off });
                }
            }
            self.skip_ws()?;
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                    continue;
                }
                Some(')') => {
                    self.i += 1;
                    break;
                }
                _ => return self.err("parameters are separated by `,`; there is no free text inside parentheses (3.3)"),
            }
        }
        p.end = self.i;
        Ok(p)
    }

    fn value(&mut self) -> PResult<Value> {
        self.skip_ws()?;
        match self.peek() {
            Some('"') => Ok(Value::Str(self.string()?)),
            Some('[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.skip_ws()?;
                if self.peek() == Some(']') {
                    self.i += 1;
                    return Ok(Value::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.skip_ws()?;
                    match self.peek() {
                        Some(',') => self.i += 1,
                        Some(']') => {
                            self.i += 1;
                            return Ok(Value::Arr(items));
                        }
                        _ => return self.err("the array is not closed"),
                    }
                }
            }
            Some('?') => {
                self.i += 1;
                let (name, _q) = self.rnl_name()?;
                Ok(Value::Hole(name))
            }
            Some(c) if c.is_ascii_digit() || c == '+' || c == '-' || c == '.' => {
                let (value, text) = self.number()?;
                Ok(Value::Num { value, text })
            }
            Some(c) if is_word_char(c) => {
                let w = self.word();
                Ok(match w.as_str() {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    _ => Value::Word(w),
                })
            }
            Some(c) if "#&%$({<".contains(c) => self.err("parameters hold literals only; objects are written outside the parentheses (4.4)"),
            _ => self.expected("expected a value here"),
        }
    }

    /// A decimal number: sign, digits, an optional fraction and exponent (3.7).
    fn number(&mut self) -> PResult<(f64, String)> {
        let n = self.n();
        let s = self.i;
        let mut j = self.i;
        if j < n && (self.t[j] == '+' || self.t[j] == '-') {
            j += 1;
        }
        let digits = |j: &mut usize, t: &Vec<char>| {
            let k = *j;
            while *j < n && t[*j].is_ascii_digit() {
                *j += 1;
            }
            *j > k
        };
        let int = digits(&mut j, &self.t);
        let mut ok = int;
        if j < n && self.t[j] == '.' {
            let save = j;
            j += 1;
            let frac = digits(&mut j, &self.t);
            if !int && !frac {
                j = save;
            }
            ok = int || frac;
        }
        if !ok {
            return self.expected("expected a number here");
        }
        if j < n && (self.t[j] == 'e' || self.t[j] == 'E') {
            let save = j;
            j += 1;
            if j < n && (self.t[j] == '+' || self.t[j] == '-') {
                j += 1;
            }
            if !digits(&mut j, &self.t) {
                j = save;
            }
        }
        if j < n && is_word_char(self.t[j]) {
            return self.err_at("a number takes no unit or suffix (3.7)", s);
        }
        let text: String = self.t[s..j].iter().collect();
        self.i = j;
        match text.parse::<f64>() {
            Ok(v) => Ok((v, text)),
            Err(_) => self.err_at("expected a number here", s),
        }
    }

    fn word(&mut self) -> String {
        let j0 = self.i;
        while self.i < self.n() && is_word_char(self.t[self.i]) {
            self.i += 1;
        }
        self.t[j0..self.i].iter().collect()
    }

    /// An unquoted role or parameter name; any non-ASCII character counts (3.6, 16.5).
    fn name_word(&mut self) -> String {
        let j0 = self.i;
        while self.i < self.n() && is_name_char(self.t[self.i]) {
            self.i += 1;
        }
        self.t[j0..self.i].iter().collect()
    }

    fn rnl_name(&mut self) -> PResult<(String, bool)> {
        if self.peek() == Some('"') {
            return Ok((self.string()?, true));
        }
        match self.ascii_name() {
            Some(n) => Ok((n, false)),
            None => self.expected("expected a name here (non-ASCII names in quotes, 3.6)"),
        }
    }

    fn string(&mut self) -> PResult<String> {
        let mut j = self.i + 1;
        let mut out = String::new();
        loop {
            match self.t.get(j) {
                None | Some('\n') => return self.err("the string is not closed (strings cannot span lines, 3.7)"),
                Some('"') => {
                    if self.t.get(j + 1) == Some(&'"') {
                        out.push('"');
                        j += 2;
                        continue;
                    }
                    break;
                }
                Some(&c) => {
                    out.push(c);
                    j += 1;
                }
            }
        }
        self.i = j + 1;
        Ok(out)
    }
}

fn failed_member(id: NodeId, start: Off, end: Off, e: SyntaxErr, words: Vec<(String, Off)>) -> Member {
    Member {
        id,
        binds: Vec::new(),
        segments: Vec::new(),
        start,
        end,
        error: Some(e.msg),
        error_pos: e.pos,
        free_words: words,
        written: HashMap::new(),
    }
}

fn check_member(m: &Member) -> PResult<()> {
    let err = |msg: &str, pos| Err(SyntaxErr { msg: msg.into(), pos, expected: false });
    for (k, seg) in m.segments.iter().enumerate() {
        if k > 0 && seg.call.is_none() {
            return err("a segment after a pipe has no keyword (4.2 rule 3)", seg.start);
        }
        if seg.call.is_none() && k == 0 && m.segments.len() > 1 && seg.args.len() != 1 {
            return err("the first segment of a pipe without a keyword holds one object (4.2 rule 3)", seg.start);
        }
        if seg.call.is_none() && m.binds_after(k).next().is_some() && seg.args.len() != 1 {
            return err(
                "a named segment without a keyword holds one object; write several objects as `{…}` (4.2 rule 4)",
                seg.start,
            );
        }
        for a in &seg.args {
            if a.role.is_some() && seg.call.is_none() {
                return err("roles are written only on the objects of a segment with a keyword (4.2 rule 5)", a.start);
            }
            if a.role.is_some() && k > 0 {
                return err("a segment after a pipe cannot label roles (4.2 rule 5)", a.start);
            }
        }
        let mut roles: Vec<&String> = seg.args.iter().filter_map(|a| a.role.as_ref()).collect();
        let n = roles.len();
        roles.sort();
        roles.dedup();
        if roles.len() != n {
            return err("a role is labelled only once (4.2 rule 6)", seg.start);
        }
    }
    Ok(())
}

/// Parse one record; `text` starts with `@RNL` and ends with the matching `}`.
pub fn parse_record(text: &str) -> RecordAst {
    Parser::new(text).record()
}

/// Every reference in a member, its segments and nested sets, except inside definition bodies.
pub fn walk_refs<'a>(m: &'a Member, out: &mut Vec<RefOrHole<'a>>) {
    for seg in &m.segments {
        for a in &seg.args {
            walk_obj(&a.obj, out);
        }
        if let Some(p) = seg.call.as_ref().and_then(|c| c.params.as_ref()) {
            for v in &p.positional {
                walk_val(v, out);
            }
            for n in &p.named {
                walk_val(&n.value, out);
            }
        }
    }
}

pub enum RefOrHole<'a> {
    Ref(&'a Ref),
    Hole(&'a str),
}

fn walk_obj<'a>(o: &'a ObjExpr, out: &mut Vec<RefOrHole<'a>>) {
    match o {
        ObjExpr::Ref(r) => out.push(RefOrHole::Ref(r)),
        ObjExpr::Point(p) => {
            for c in [&p.x, &p.y] {
                if let Coord::Hole(n) = c {
                    out.push(RefOrHole::Hole(n));
                }
            }
        }
        ObjExpr::Set(s) => {
            for m in &s.members {
                let defines = m.segments.iter().any(|sg| {
                    sg.call.as_ref().is_some_and(|c| matches!(c.kw.as_str(), "DEFINE-SYMBOL" | "DEFINE" | "DEFINE-STYLE"))
                });
                if !defines {
                    walk_refs(m, out);
                }
            }
        }
    }
}

fn walk_val<'a>(v: &'a Value, out: &mut Vec<RefOrHole<'a>>) {
    match v {
        Value::Hole(n) => out.push(RefOrHole::Hole(n)),
        Value::Arr(items) => {
            for x in items {
                walk_val(x, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(text: &str) -> SetExpr {
        let r = parse_record(text);
        assert!(r.error.is_none(), "{:?}", r.error);
        r.body.unwrap()
    }

    #[test]
    fn statements_pipes_and_names() {
        let b = body("@RNL {%g0 = $GND @ATTACH &vss => @PLACE(24, 38); #M1(in_p) #M2(in_n) @AS $DIFFPAIR => %dp}");
        assert_eq!(b.members.len(), 2);
        let m = &b.members[0];
        assert_eq!(m.binds.len(), 1);
        assert_eq!(m.binds[0].name, "g0");
        assert_eq!(m.binds[0].after, 1);
        assert_eq!(m.segments.len(), 2);
        assert_eq!(m.segments[1].call.as_ref().unwrap().kw, "PLACE");
        let m = &b.members[1];
        assert_eq!(m.segments[0].args[0].role.as_deref(), Some("in_p"));
        assert_eq!(m.binds[0].name, "dp");
    }

    #[test]
    fn free_text_and_points() {
        let b = body("@RNL { the pair: {#M1 #M2} @ALIGN(\"y\"), i.e., same y; #M5 is at @PLACE(20, 6) => it @ORIENT(\"R0\")}");
        assert_eq!(b.members.len(), 2);
        assert!(b.members[0].error.is_none());
        let b = body("@RNL {%p = (1.5, _); (0, -2.5E1) @TEXT(\"x\")}");
        let ObjExpr::Point(p) = &b.members[0].segments[0].args[0].obj else { panic!() };
        assert_eq!(p.x, Coord::Num(1.5));
        assert_eq!(p.y, Coord::Free);
    }

    #[test]
    fn errors_fail_one_member() {
        let b = body("@RNL {#M1 @PLACE (10, 20); #M2 @PLACE(1, 2); #M3 @PLACE(x=1, 2)}");
        assert!(b.members[0].error.is_none()); // a point after PLACE: the keyword fails later
        assert!(b.members[1].error.is_none());
        assert!(b.members[2].error.as_ref().unwrap().contains("by position"));
        let b = body("@RNL {#M1 => #M2; #M3}");
        assert!(b.members[0].error.as_ref().unwrap().contains("no keyword"));
        assert!(b.members[1].error.is_none());
    }

    #[test]
    fn subscripts() {
        let b = body("@RNL {&D<7:0> @WIRE {(0, 0) (1, 0)}; #XI0<0:6:2, 9>}");
        let ObjExpr::Ref(r) = &b.members[0].segments[0].args[0].obj else { panic!() };
        assert_eq!(r.sub.as_ref().unwrap().expand(), vec![7, 6, 5, 4, 3, 2, 1, 0]);
        let ObjExpr::Ref(r) = &b.members[1].segments[0].args[0].obj else { panic!() };
        assert_eq!(r.sub.as_ref().unwrap().expand(), vec![0, 2, 4, 6, 9]);
        let b = body("@RNL {#XI0<1:0>:D<1:0>}");
        assert!(b.members[0].error.is_some());
    }

    #[test]
    fn strings_and_names() {
        let b = body("@RNL {#M1 @TEXT(\"say \"\"hi\"\"\", key=\"k\"); %\"差分对\" = {#M1}}");
        let p = b.members[0].segments[0].call.as_ref().unwrap().params.as_ref().unwrap();
        assert_eq!(p.positional[0], Value::Str("say \"hi\"".into()));
        assert_eq!(b.members[1].binds[0].name, "差分对");
    }

    #[test]
    fn unclosed_record_body() {
        let r = parse_record("@RNL {#M1 @PLACE(1, 2)");
        assert!(r.error.is_some());
    }
}
