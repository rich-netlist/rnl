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
