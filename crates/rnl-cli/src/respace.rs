//! `rnl solve --respace` and `rnl preview --respace`: each level as the record of the solver last
//! drew it, spaced again to the spacing of its settings, nothing searched
//! ([`rnl_solver::respace_with_clock`]). A level whose record cannot be used is left as it is, in
//! [`SolveRun::not_solved`], with the reason: [`NO_RECORD`] when it has no record of the solver, or
//! one that places nothing; [`STALE`] when its record is not of the netlist as it is now, such as
//! after the level was edited, or its numbers no longer hold; [`UNSUPPORTED`] for a level before
//! RNL 0.4. Solving such a level again gives it a record to respace.

use std::collections::HashMap;
use std::path::Path;

use rnl_runtime::previous::Previous;
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::source::Provider;
use rnl_solver::Clock;

use crate::progress::{solve_progress_skipping, Event};
use crate::provider::path_str;
use crate::solve::{BackendResult, SolveOptions, SolveRun};

/// The reason of a level with no record of the solver to respace, or one that places nothing.
pub const NO_RECORD: &str = "no-generated";
/// The reason of a level whose record of the solver is not of the netlist as it is now, or cannot be
/// compared with it.
pub const STALE: &str = "mismatch";
/// The reason of a level of a version before RNL 0.4.
pub const UNSUPPORTED: &str = "unsupported";

/// The records of the solver of the levels of `file`, read onto the problems, by level and title.
fn records(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
) -> Result<HashMap<(usize, String), Previous>, String> {
    let (r, previous) = rnl_view::read_previous_with(
        &path_str(file),
        options.dialect.as_deref(),
        &options.formats,
        files,
        &options.runtime(),
    )?;
    Ok(r.problems.iter().zip(previous).filter_map(|(p, record)| Some(((p.level, p.title.clone()), record?))).collect())
}

/// Why the level of `p` is not respaced, as [`crate::solve::NotSolved::reason`] gives it: the code,
/// a colon, and what to do.
fn why_not(p: &LevelProblem, record: Option<&Previous>) -> Option<String> {
    match record {
        _ if p.version != "0.4" => Some(format!("{UNSUPPORTED}: only a level of RNL 0.4 is respaced")),
        None => Some(format!("{NO_RECORD}: the level has no record of the solver to respace; solve it first")),
        Some(r) if r.same_input.is_none() => Some(format!(
            "{STALE}: the record of the solver has no hash this solver can compare with the netlist; solve the \
             level again"
        )),
        Some(r) if r.same_input == Some(false) || r.numbering == Some(false) => Some(format!(
            "{STALE}: the record of the solver is not of the netlist as it is now, which has changed since; solve \
             the level again"
        )),
        Some(r) if r.placements.is_empty() => {
            Some(format!("{NO_RECORD}: the record of the solver places nothing; solve the level first"))
        }
        Some(_) => None,
    }
}

/// Respace `file`, every step given to `sink` as for [`crate::progress::solve_progress`].
pub fn respace_progress(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
    sink: &dyn Fn(&Event) -> bool,
) -> Result<SolveRun, String> {
    let records = records(file, files, options)?;
    let record = |p: &LevelProblem| records.get(&(p.level, p.title.clone()));
    solve_progress_skipping(file, files, options, clock, sink, &|p| why_not(p, record(p)), &|p, options, clock| {
        let previous = record(p).ok_or_else(|| format!("the level {} has no record of the solver", p.title))?;
        Ok(BackendResult {
            solved: rnl_solver::respace_with_clock(p, previous, options, clock)?,
            author: rnl_solver::AUTHOR.to_string(),
            report_available: true,
        })
    })
}

/// [`respace_progress`] with no one watching.
pub fn respace(
    file: &Path,
    files: &dyn Provider,
    options: &SolveOptions,
    clock: &dyn Clock,
) -> Result<SolveRun, String> {
    respace_progress(file, files, options, clock, &|_| true)
}
