//! Diagnostics (spec-draft 7.6): three levels, a location, and an optional upstream cause.
//!
//! Checking and interpreting are one process, so the same list serves a linter and a viewer.
//! An error means an item failed; a warning flags something legal that is probably not what the
//! author meant; a hint reports a common override, shadowing or trade-off.

use std::collections::HashSet;
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

/// Index of a diagnostic in its [`Sink`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DiagId(pub u32);

#[derive(Clone, Debug, PartialEq)]
pub struct Diag {
    pub severity: Severity,
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
#[derive(Default, Debug)]
pub struct Sink {
    items: Vec<Diag>,
    seen: HashSet<(Severity, String, Option<Loc>, String)>,
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

    pub fn add(&mut self, severity: Severity, message: impl Into<String>, loc: Option<Loc>, extra: Extra) -> DiagId {
        let message = message.into();
        let key = (severity, message.clone(), loc.clone(), extra.origin.clone());
        if self.seen.contains(&key) {
            let pos = self
                .items
                .iter()
                .position(|d| d.severity == severity && d.message == message && d.loc == loc && d.origin == extra.origin)
                .unwrap_or(0);
            return DiagId(pos as u32);
        }
        self.seen.insert(key);
        self.items.push(Diag {
            severity,
            message,
            loc,
            cause: extra.cause,
            origin: extra.origin,
            in_package: extra.in_package,
        });
        DiagId(self.items.len() as u32 - 1)
    }

    pub fn error(&mut self, message: impl Into<String>, loc: Option<Loc>) -> DiagId {
        self.add(Severity::Error, message, loc, Extra::default())
    }

    pub fn warning(&mut self, message: impl Into<String>, loc: Option<Loc>) -> DiagId {
        self.add(Severity::Warning, message, loc, Extra::default())
    }

    pub fn hint(&mut self, message: impl Into<String>, loc: Option<Loc>) -> DiagId {
        self.add(Severity::Hint, message, loc, Extra::default())
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

    pub fn render(&self, d: &Diag) -> String {
        let mut text = String::from(d.severity.as_str());
        if let Some(loc) = &d.loc {
            text.push_str(&format!(" {loc}"));
        }
        text.push_str(": ");
        text.push_str(&d.message);
        if !d.origin.is_empty() {
            text.push_str(&format!(" [{}]", d.origin));
        }
        if let Some(c) = d.cause {
            let cause = self.get(c);
            match &cause.loc {
                Some(loc) => text.push_str(&format!("\n    upstream cause: {loc}: {}", cause.message)),
                None => text.push_str(&format!("\n    upstream cause: {}", cause.message)),
            }
        }
        text
    }
}
