//! The definition used by tools that add junction dots (13.2).
use crate::diag::{codes, Diag, Severity};
use crate::problem::SolverSetting;

/// Resolved in the target level, without applying a solver policy while reading.
/// Invalid settings are reported by the tool that requests dots, not by an ordinary viewer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JunctionConfig {
    /// A validated, writable definition reference, or no dots.
    pub definition: Option<String>,
    /// A value that failed resolution, kept for a consumer's located warning: the last one, when
    /// `definition` is that of an earlier value, or none (14.10). In practice a tool's value: the
    /// file's own values are judged in reading, where one that names no definition is an error and
    /// is left out of the settings. A tool's value that names none leaves the file's definition in
    /// effect, which its `definition` then repeats.
    pub invalid: Option<SolverSetting>,
    /// An explicit tool override, resolved in the same level. Operational metadata: the
    /// file's definition above remains the input identity used by sig2.
    pub tool_override: Option<Box<JunctionConfig>>,
}
impl JunctionConfig {
    pub fn effective(&self) -> &JunctionConfig {
        self.tool_override.as_deref().unwrap_or(self)
    }

    pub fn warning(&self) -> Option<Diag> {
        let effective = self.effective();
        let setting = effective.invalid.as_ref()?;
        let outcome = match &effective.definition {
            Some(definition) => format!("the file's {definition} applies"),
            None => "no dots are written".to_string(),
        };
        // a value that is not taken is an error for its key, and the value before it applies (14.10)
        Some(Diag {
            severity: Severity::Error,
            code: codes::JUNCTION_DEFINITION_INVALID,
            message: format!("junction={} given to the tool must name a definition whose first role accepts a point in the level; the value is not taken there, and {outcome} (13.2, 14.10)", setting.value.show()),
            loc: setting.source.clone(), cause: None, origin: String::new(), in_package: setting.package.clone(),
        })
    }
}
