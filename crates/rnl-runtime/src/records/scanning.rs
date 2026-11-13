//! Choose record boundaries only after each file's language version is known (2.4, 16.1).
use std::collections::{HashMap, HashSet};

use crate::diag::{Diag, DiagId, Extra, Sink};
use crate::host::format::Reading;
use crate::host::{LevelId, LevelKind, Record};
use crate::interp::unit::{declared_record_version, keeps_0_4, DEFAULT_VERSION};

use super::{scan_run, scan_run_with, CarrierLine};

pub(crate) struct PendingRun {
    level: LevelId,
    start: usize,
    count: usize,
    candidate: Vec<Record>,
    diagnostics: Vec<Diag>,
    original_diagnostics: Vec<DiagId>,
}

fn add_diagnostic(sink: &mut Sink, diagnostic: &Diag, cause: Option<DiagId>) -> DiagId {
    sink.add(
        diagnostic.severity,
        diagnostic.code,
        diagnostic.message.clone(),
        diagnostic.loc.clone(),
        Extra { cause, origin: diagnostic.origin.clone(), in_package: diagnostic.in_package.clone() },
    )
}

impl Reading<'_> {
    /// Read one carrier run. Adapters call this in source order instead of choosing a
    /// language themselves. Released diagnostics are emitted at their original position.
    pub fn scan_records(&mut self, run: &[CarrierLine], file: &str, level: LevelId) {
        let mut old_sink = Sink::new();
        let records = scan_run(run, file, &mut old_sink);
        let original_diagnostics = old_sink.all().iter().map(|d| add_diagnostic(self.sink, d, None)).collect();
        let mut new_sink = Sink::new();
        let candidate = scan_run_with(run, file, &mut new_sink, true);
        self.pending_records.push(PendingRun {
            level,
            start: self.doc.levels[level].records.len(),
            count: records.len(),
            candidate,
            diagnostics: new_sink.all().to_vec(),
            original_diagnostics,
        });
        self.doc.levels[level].records.extend(records);
    }

    /// Choose the boundaries of the records of each file by its language version; and say, for
    /// each level, whether its file is of version 0.4 or later.
    pub(crate) fn finish_record_scans(&mut self) -> Vec<bool> {
        let runs = std::mem::take(&mut self.pending_records);
        if runs.is_empty() {
            // no carrier, so no header record: every file is of the default version
            return vec![keeps_0_4(DEFAULT_VERSION); self.doc.levels.len()];
        }
        // Every range refers to the original level vector. Reverse order also puts
        // consecutive runs that originally yielded no records in their source order.
        let mut order: Vec<_> = (0..runs.len()).collect();
        order.sort_by_key(|&index| std::cmp::Reverse((runs[index].level, runs[index].start, index)));
        let mut candidates: Vec<_> = self.doc.levels.iter().map(|level| level.records.clone()).collect();
        for &index in &order {
            let run = &runs[index];
            candidates[run.level].splice(run.start..run.start + run.count, run.candidate.clone());
        }
        let mut declarations = HashMap::new();
        for (level, records) in candidates.iter().enumerate() {
            if self.doc.levels[level].kind != LevelKind::File {
                continue;
            }
            let Some(first) = records.first() else { continue };
            let at = first.loc(0);
            let earlier = candidates.iter().enumerate().any(|(other, records)| {
                self.doc.file_level(other) == level
                    && records.iter().any(|record| {
                        let loc = record.loc(0);
                        (loc.line, loc.col) < (at.line, at.col)
                    })
            });
            if !earlier {
                if let Some((version, _)) = declared_record_version(&first.text) {
                    declarations.insert(level, version);
                }
            }
        }
        let modern: Vec<_> = (0..self.doc.levels.len())
            .map(|level| {
                let mut file = self.doc.file_level(level);
                loop {
                    if let Some(version) = declarations.get(&file) {
                        return keeps_0_4(version);
                    }
                    match self.doc.levels[file].parent {
                        Some(parent) => file = self.doc.file_level(parent),
                        None => return keeps_0_4(DEFAULT_VERSION),
                    }
                }
            })
            .collect();
        if !modern.iter().any(|&modern| modern) {
            return modern;
        }
        for &index in &order {
            let run = &runs[index];
            if modern[run.level] {
                self.doc.levels[run.level].records.splice(run.start..run.start + run.count, run.candidate.clone());
            }
        }
        // A diagnostic can be shared by two readings of the same physical file.
        // Keep it if a released-language reading still needs it.
        let kept: HashSet<_> = runs
            .iter()
            .filter(|run| !modern[run.level])
            .flat_map(|run| run.original_diagnostics.iter().map(|id| id.0))
            .collect();
        let removed: HashSet<_> = runs
            .iter()
            .filter(|run| modern[run.level])
            .flat_map(|run| run.original_diagnostics.iter().map(|id| id.0))
            .filter(|id| !kept.contains(id))
            .collect();
        // Host data stores no diagnostic IDs. Rebuild here, before the interpreter
        // retains any IDs, remapping upstream causes of the retained host diagnostics.
        let original = std::mem::take(self.sink);
        let mut ids = Vec::new();
        for (index, diagnostic) in original.all().iter().enumerate() {
            let next = if removed.contains(&(index as u32)) {
                None
            } else {
                let cause = diagnostic.cause.and_then(|id| ids.get(id.0 as usize).copied().flatten());
                Some(add_diagnostic(self.sink, diagnostic, cause))
            };
            ids.push(next);
        }
        for run in &runs {
            if modern[run.level] {
                for diagnostic in &run.diagnostics {
                    add_diagnostic(self.sink, diagnostic, None);
                }
            }
        }
        modern
    }
}
