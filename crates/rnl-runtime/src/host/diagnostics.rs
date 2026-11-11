//! Source context retained by host readers for versioned diagnostic presentation.
use super::LevelId;
use crate::diag::{DiagId, Loc};

/// One reading that produced a host diagnostic. Several readings can share a deduplicated id.
#[derive(Clone, Debug)]
pub struct Reading {
    pub diagnostic: DiagId,
    /// A document may later be interpreted with a different or empty host diagnostic sink.
    pub original: crate::diag::Diag,
    pub level: LevelId,
    pub detail: Detail,
}

/// Information a host reader knows before the runtime reads each file's language version.
#[derive(Clone, Debug)]
pub enum Detail {
    FirstLine,
    UnselectedSection,
    OutsideSection,
    UnclosedSubcircuits(Vec<(String, Loc)>),
    DuplicateInstance { name: String, first: Loc },
    MissingSection { name: String, file: String, include: Option<Loc> },
}

impl Detail {
    pub(crate) fn presentation(&self) -> (String, Option<Loc>) {
        let message = match self {
            Self::FirstLine => "the first line of a file is not read as RNL (2.2)".into(),
            Self::UnselectedSection => "RNL in a library section that is not selected is not read (2.2)".into(),
            Self::OutsideSection => "RNL outside the selected library section is not read (2.2)".into(),
            Self::UnclosedSubcircuits(open) => {
                let names = open.iter().map(|(name, loc)| format!("{name} ({loc})")).collect::<Vec<_>>().join(", ");
                let (subject, verb) = if open.len() == 1 { ("subcircuit", "has") } else { ("subcircuits", "have") };
                format!("{subject} {names} {verb} no closing ends declaration (2.6)")
            }
            Self::DuplicateInstance { name, first } => {
                format!("instance {name} is defined twice; first defined at {first} (2.6)")
            }
            Self::MissingSection { name, file, .. } => format!(
                "the selected library section {name} was not found in {file}; no records can be written there (2.2)"
            ),
        };
        let loc = match self {
            Self::UnclosedSubcircuits(open) => open.first().map(|(_, loc)| loc.clone()),
            Self::MissingSection { include, .. } => include.clone(),
            _ => None,
        };
        (message, loc)
    }
}

/// A host input issue whose presence depends on the reading's RNL version.
#[derive(Clone, Debug)]
pub struct Input {
    pub level: LevelId,
    pub issue: InputIssue,
    pub loc: Loc,
}

#[derive(Clone, Debug)]
pub enum InputIssue {
    MissingOperand { directive: String, operand: &'static str },
    UnmatchedEnd(String),
    UnclosedProtection(String),
    UnsupportedCoupling(String),
}

impl InputIssue {
    pub(crate) fn diagnostic(&self) -> (crate::diag::Code, crate::diag::Severity, String) {
        use crate::diag::{codes, Severity};
        match self {
            Self::MissingOperand { directive, operand } => (codes::HOST_DIRECTIVE_MISSING_OPERAND, Severity::Error, format!("{directive} needs {operand}; the directive is ignored (2.2)")),
            Self::UnmatchedEnd(directive) => (codes::HOST_UNMATCHED_DIRECTIVE, Severity::Warning, format!("{directive} has no open subcircuit to close; the unmatched directive is ignored (2.2)")),
            Self::UnclosedProtection(directive) => (codes::HOST_UNTERMINATED_REGION, Severity::Warning, format!("the protection region opened by {directive} is not closed; it is not writable through end of file (2.2)")),
            Self::UnsupportedCoupling(name) => (codes::HOST_ELEMENT_UNSUPPORTED, Severity::Warning, format!("SPICE coupling {name} is not represented in the drawing; its referenced inductors are still read (2.1)")),
        }
    }
}
