//! Resolve old terminal names to stable host positions without changing netlist lines (6.4, 6.12).
use rnl_runtime::syntax::{Ref, Sep};
use rnl_runtime::Interpretation;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct HostTerminals {
    positions: BTreeMap<(usize, String, String, bool), usize>,
}
impl HostTerminals {
    pub(super) fn new(input: &Interpretation) -> Self {
        let mut out = Self::default();
        for level in &input.model.levels {
            for (index, instance) in input.doc.levels[level.level].instances.iter().enumerate() {
                let Some(names) = level.terminal_names.get(index) else { continue };
                let defaults_changed = instance
                    .default_terminals
                    .as_ref()
                    .is_some_and(|v| v == &["p_plus", "p_minus", "s_plus", "s_minus"]);
                let cs = instance.case_sensitive;
                let key = |s: &str| if cs { s.to_string() } else { s.to_lowercase() };
                for (position, name) in names.iter().enumerate() {
                    let Some(name) = name else { continue };
                    let numeric = !name.is_empty() && name.chars().all(|c| c.is_ascii_digit());
                    let old_default = defaults_changed
                        && instance.default_terminals.as_ref().and_then(|v| v.get(position)) == Some(name);
                    if (numeric || old_default)
                        && names.iter().filter(|n| n.as_ref().is_some_and(|n| key(n) == key(name))).count() == 1
                    {
                        out.positions.insert((level.level, key(&instance.name), key(name), cs), position + 1);
                    }
                }
            }
        }
        out
    }
    pub(super) fn replacement(&self, readings: &[usize], reference: &Ref) -> Option<String> {
        if reference.prefix != '#' || reference.sub.is_some() || reference.places.is_some() || reference.path.len() != 1
        {
            return None;
        }
        let host = reference.name.as_ref()?;
        let terminal = &reference.path[0];
        if terminal.sep != Sep::Term || terminal.sub.is_some() || terminal.number.is_some() {
            return None;
        }
        let mut resolved = None;
        for level in readings {
            let found = self
                .positions
                .get(&(*level, host.clone(), terminal.name.clone(), true))
                .or_else(|| self.positions.get(&(*level, host.to_lowercase(), terminal.name.to_lowercase(), false)))?;
            if resolved.is_some_and(|old| old != *found) {
                return None;
            }
            resolved = Some(*found);
        }
        Some(format!("#{}:{}", rnl_runtime::problem::written_name(host), resolved?))
    }
}
