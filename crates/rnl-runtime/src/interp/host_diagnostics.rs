//! Select host diagnostic presentation after resolving each reading's language version.
use super::Interp;
use crate::diag::{Diag, DiagId, Extra, Sink};
use std::collections::HashMap;

impl Interp<'_> {
    pub(super) fn check_host_diagnostics(&mut self) {
        let mut variants: HashMap<DiagId, Vec<Diag>> = HashMap::new();
        let mut changed = false;
        for reading in &self.doc.diagnostic_readings {
            let Some(original) = self.sink.all().get(reading.diagnostic.0 as usize) else { continue };
            if original.code != reading.original.code
                || original.severity != reading.original.severity
                || original.message != reading.original.message
                || original.loc != reading.original.loc
                || original.origin != reading.original.origin
                || original.in_package != reading.original.in_package
            {
                continue;
            }
            let mut diagnostic = original.clone();
            if self.follows_0_4(self.level_unit[reading.level]) {
                let (message, loc) = reading.detail.presentation();
                let severity = reading.detail.severity(diagnostic.severity);
                changed |= diagnostic.severity != severity;
                diagnostic.severity = severity;
                changed |=
                    diagnostic.message != message || loc.as_ref().is_some_and(|l| diagnostic.loc.as_ref() != Some(l));
                diagnostic.message = message;
                if loc.is_some() {
                    diagnostic.loc = loc;
                }
            }
            variants.entry(reading.diagnostic).or_default().push(diagnostic);
        }
        if !changed {
            return;
        }
        // Raw host ids remain in the document, paired with the raw host sink for every restart.
        // Rebuild only this interpretation's sink, remapping causes without changing the readers.
        let mut sink = Sink::new();
        let mut remap = HashMap::new();
        let mut causes = Vec::new();
        for (index, original) in self.sink.all().iter().enumerate() {
            let old = DiagId(index as u32);
            let choices = variants.remove(&old).unwrap_or_else(|| vec![original.clone()]);
            for diagnostic in choices {
                let extra = Extra { origin: diagnostic.origin, in_package: diagnostic.in_package, ..Extra::default() };
                let id = sink.add(diagnostic.severity, diagnostic.code, diagnostic.message, diagnostic.loc, extra);
                remap.entry(old).or_insert(id);
                causes.push((id, diagnostic.cause));
            }
        }
        for (id, cause) in causes {
            sink.set_cause(id, cause.and_then(|c| remap.get(&c).copied()));
        }
        self.sink = sink;
    }
}

impl Interp<'_> {
    pub(super) fn check_host_input_diagnostics(&mut self) {
        for input in &self.doc.input_diagnostics {
            if self.follows_0_4(self.level_unit[input.level]) {
                let (code, severity, message) = input.issue.diagnostic();
                self.sink.add(severity, code, message, Some(input.loc.clone()), Extra::default());
            }
        }
    }
}
