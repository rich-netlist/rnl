//! Diagnostics (standard 7.6): three levels, a kind, a location, and an optional upstream cause.
//!
//! Checking and interpreting are one process, so the same list serves a linter and a viewer.
//! An error means an item failed; a warning flags something legal that is probably not what the
//! author meant; a hint reports a common override, shadowing or trade-off. Each diagnostic is of a
//! kind of the toolkit's list (`data/diagnostics.json`), [`codes`], by which a tool or a test tells
//! it apart from the others, whatever its message says.

use std::collections::HashMap;
use std::fmt;

/// How serious a diagnostic is (7.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Hint => "hint",
        }
    }
}

/// A place in a source file. Lines and columns count from 1; column 0 means the whole line.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Loc {
    pub file: String,
    pub line: u32,
    pub col: u32,
}

impl Loc {
    pub fn new(file: impl Into<String>, line: u32, col: u32) -> Loc {
        Loc { file: file.into(), line, col }
    }

    pub fn line(file: impl Into<String>, line: u32) -> Loc {
        Loc { file: file.into(), line, col: 0 }
    }
}

impl fmt::Display for Loc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.col > 0 {
            write!(f, "{}:{}:{}", self.file, self.line, self.col)
        } else {
            write!(f, "{}:{}", self.file, self.line)
        }
    }
}

/// The kind of a diagnostic: its number in the toolkit's list, RNL and four digits, such as
/// RNL3120, and its short name, such as `param-wrong-type`. A kind of the versions 0.2 and 0.3 alone
/// has a name and no number. Numbers and names are kept from one version to the next and never given
/// to another kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Code {
    number: Option<u16>,
    name: &'static str,
}

impl Code {
    pub const fn new(number: u16, name: &'static str) -> Code {
        Code { number: Some(number), name }
    }

    pub const fn unnumbered(name: &'static str) -> Code {
        Code { number: None, name }
    }

    /// The number, as "RNL3120"; None for a kind without one.
    pub fn number(&self) -> Option<String> {
        self.number.map(|n| format!("RNL{n:04}"))
    }

    /// The short name, such as "param-wrong-type".
    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl fmt::Display for Code {
    /// "RNL3120 param-wrong-type", or the name alone for a kind without a number.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.number() {
            Some(n) => write!(f, "{n} {}", self.name),
            None => f.write_str(self.name),
        }
    }
}

/// The kinds of the toolkit's list, `data/diagnostics.json`, one constant for each, named by its
/// short name: `param-wrong-type` is [`codes::PARAM_WRONG_TYPE`].
pub mod codes {
    use super::Code;
    include!(concat!(env!("OUT_DIR"), "/codes.rs"));
}

/// Index of a diagnostic in its [`Sink`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DiagId(pub u32);

#[derive(Clone, Debug, PartialEq)]
pub struct Diag {
    pub severity: Severity,
    /// Its kind ([`codes`]).
    pub code: Code,
    pub message: String,
    pub loc: Option<Loc>,
    /// The upstream cause of an item that failed because something it depends on failed (7.2).
    pub cause: Option<DiagId>,
    /// Expansion, rule or package history (7.6), such as "applying $GND_STUB (a.sp:12:3)".
    pub origin: String,
    /// The package the location is in, when it is inside a package file or body.
    pub in_package: String,
}

/// Collects diagnostics; the same message at the same place is kept once.
#[derive(Clone, Default, Debug)]
pub struct Sink {
    items: Vec<Diag>,
    /// Each diagnostic kept, by what makes it the same, with its index in `items`.
    seen: HashMap<(Severity, String, Option<Loc>, String), u32>,
}

/// Optional parts of a diagnostic.
#[derive(Default, Clone)]
pub struct Extra {
    pub cause: Option<DiagId>,
    pub origin: String,
    pub in_package: String,
}

impl Sink {
    pub fn new() -> Sink {
        Sink::default()
    }

    pub fn add(
        &mut self,
        severity: Severity,
        code: Code,
        message: impl Into<String>,
        loc: Option<Loc>,
        extra: Extra,
    ) -> DiagId {
        let message = message.into();
        let key = (severity, message.clone(), loc.clone(), extra.origin.clone());
        if let Some(&k) = self.seen.get(&key) {
            return DiagId(k);
        }
        self.seen.insert(key, self.items.len() as u32);
        self.items.push(Diag {
            severity,
            code,
            message,
            loc,
            cause: extra.cause,
            origin: extra.origin,
            in_package: extra.in_package,
        });
        DiagId(self.items.len() as u32 - 1)
    }

    pub fn error(&mut self, code: Code, message: impl Into<String>, loc: Option<Loc>) -> DiagId {
        self.add(Severity::Error, code, message, loc, Extra::default())
    }

    pub fn warning(&mut self, code: Code, message: impl Into<String>, loc: Option<Loc>) -> DiagId {
        self.add(Severity::Warning, code, message, loc, Extra::default())
    }

    pub fn hint(&mut self, code: Code, message: impl Into<String>, loc: Option<Loc>) -> DiagId {
        self.add(Severity::Hint, code, message, loc, Extra::default())
    }

    pub fn get(&self, id: DiagId) -> &Diag {
        &self.items[id.0 as usize]
    }

    pub fn all(&self) -> &[Diag] {
        &self.items
    }

    /// What a document shows (7.6): inside a package the document uses, only errors; warnings and
    /// hints about a package are given when that package itself is checked.
    pub fn shown(&self, checking_package: &str) -> Vec<&Diag> {
        self.items
            .iter()
            .filter(|d| d.in_package.is_empty() || d.in_package == checking_package || d.severity == Severity::Error)
            .collect()
    }

    /// A diagnostic as a line of text: "error t.sp:3:5: RNL3120 param-wrong-type: …", with its
    /// origin and its upstream cause.
    pub fn render(&self, d: &Diag) -> String {
        let mut text = String::from(d.severity.as_str());
        if let Some(loc) = &d.loc {
            text.push_str(&format!(" {loc}"));
        }
        text.push_str(&format!(": {}: ", d.code));
        text.push_str(&d.message);
        if !d.origin.is_empty() {
            text.push_str(&format!(" [{}]", d.origin));
        }
        if let Some(c) = d.cause {
            let cause = self.get(c);
            match &cause.loc {
                Some(loc) => text.push_str(&format!("\n    upstream cause: {loc}: {}: {}", cause.code, cause.message)),
                None => text.push_str(&format!("\n    upstream cause: {}: {}", cause.code, cause.message)),
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_kind_has_a_number_and_a_name_of_its_own() {
        let mut numbers = HashSet::new();
        let mut names = HashSet::new();
        for c in codes::ALL {
            if let Some(n) = c.number() {
                assert!(n.len() == 7 && n.starts_with("RNL"), "{n}");
                assert!(numbers.insert(n.clone()), "{n} is given twice");
            }
            assert!(names.insert(c.name()), "{} is given twice", c.name());
        }
        assert!(numbers.len() > 400);
        assert_eq!(codes::PARAM_WRONG_TYPE.to_string(), "RNL3120 param-wrong-type");
        // a kind of 0.2 and 0.3 alone has a name and no number
        assert_eq!(codes::UNSOLVED_NO_NUMBERS.number(), None);
        assert_eq!(codes::UNSOLVED_NO_NUMBERS.to_string(), "unsolved-no-numbers");
    }

    #[test]
    fn a_diagnostic_is_written_with_its_kind() {
        let mut sink = Sink::new();
        let cause = sink.error(codes::SYNTAX_TWO_KEYWORDS, "at most one keyword in a segment (4.2 rule 2)", None);
        let extra = Extra { cause: Some(cause), ..Extra::default() };
        let d = sink.add(
            Severity::Error,
            codes::PARAM_WRONG_TYPE,
            "the x of @PLACE has the wrong type (5.3)",
            Some(Loc::new("t.sp", 3, 5)),
            extra,
        );
        assert_eq!(sink.get(d).code, codes::PARAM_WRONG_TYPE);
        assert_eq!(
            sink.render(sink.get(d)),
            "error t.sp:3:5: RNL3120 param-wrong-type: the x of @PLACE has the wrong type (5.3)\n    upstream cause: RNL1146 syntax-two-keywords: at most one keyword in a segment (4.2 rule 2)"
        );
    }
}
