//! `rnl solve --check`: the solver's settings of each level checked, with where each comes from, and
//! nothing solved or written (14.10).
//!
//! The settings of a level are taken in the order of 14.10: the solver's defaults, the packages,
//! the file's header, the level's own records, the command line, a later one winning by key. A
//! value that the solver does not take is an error for its key (14.10): when the file gives it,
//! the solve leaves it out and goes on with the value before it, or the default; when the command
//! line gives it, the solve does not start. A key the solver does not know is a warning. Without
//! a file, the command line's settings alone are checked, over the defaults.

use std::collections::BTreeMap;
use std::path::Path;

use rnl_runtime::diag::{Loc, Severity};
use rnl_runtime::interp::obj::Lit;
use rnl_runtime::problem::{LevelProblem, SolverSetting};
use rnl_runtime::source::Provider;
use rnl_runtime::writeback::record_type;
use rnl_runtime::{Interpretation, Options};
use rnl_solver::{SettingFrom, SettingProblem, SolverOptions};
use serde::Serialize;

use super::SolveOptions;

/// A text of solver settings that a tool gives, as the command line gives them, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolSettings {
    /// `--solver-config`: RNL named parameters, as in `@SOLVER-CONFIG(…)` (14.10).
    Config(String),
    /// `--solver-options`: the old key=value;… text, deprecated. A text in the named syntax is
    /// read as such, as `rnl solve` reads it.
    Options(String),
}

/// What [`check_settings`] finds: `rnl solve --check --json` prints it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Checked {
    /// No level has an error.
    pub ok: bool,
    /// Each level checked, in the order of the document; without a file, one, untitled.
    pub levels: Vec<CheckedLevel>,
}

/// The settings of one level, and their problems.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CheckedLevel {
    /// The level's title: a subcircuit name, or the file name; none without a file.
    pub title: Option<String>,
    /// Each setting in effect, by key.
    pub settings: BTreeMap<String, CheckedSetting>,
    /// What a solve ignores and says so: a run setting written in the file, a key the solver does
    /// not know.
    pub warnings: Vec<CheckedProblem>,
    /// The errors: a value of the file that the solver does not take, which a solve leaves out,
    /// going on with the value before it; a value of the command line that it does not take, with
    /// which a solve does not start; a SOLVER-CONFIG that the reading fails or a value of it that
    /// the reading does not take.
    pub errors: Vec<CheckedProblem>,
}

/// One setting in effect.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CheckedSetting {
    /// As JSON: a number, a string, a boolean or an array of these.
    pub value: serde_json::Value,
    /// Where it comes from: "default", "package:NAME", "header", "level" or "command-line".
    pub from: String,
    /// Where it is written, `file:line`, for a setting of a package, the file's header or a level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

/// A problem of one setting, or of the settings together when `key` is none.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CheckedProblem {
    pub key: Option<String>,
    pub message: String,
    /// For a value of the file that the solver does not take, and a solve leaves out: that value,
    /// where it comes from and where it is written. The message names the value that stays in effect in
    /// its place, the one before it or the default; the key's value in effect is in `settings`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refused: Option<CheckedSetting>,
}

impl CheckedProblem {
    fn new(key: Option<String>, message: String) -> CheckedProblem {
        CheckedProblem { key, message, refused: None }
    }
}

/// Whether an old `--solver-options` text is written in the named syntax of `--solver-config`,
/// which `rnl solve` then reads as such: no `;`, and a key or a value only the named syntax has.
pub fn named_syntax(text: &str) -> bool {
    !text.contains(';')
        && rnl_runtime::solver_settings::parse_overrides(text).is_ok_and(|settings| {
            settings.len() > 1
                || settings.iter().any(|s| {
                    matches!(
                        s.key.as_str(),
                        "limit"
                            | "work"
                            | "seconds"
                            | "safety_seconds"
                            | "candidates"
                            | "astar_expansions"
                            | "junction"
                            | "layout"
                    ) || matches!(s.value, Lit::Str(_) | Lit::Bool(_) | Lit::Arr(_))
                })
        })
}

/// The solver's settings of `file`, level by level, or without a file those of `tool` alone: what
/// `rnl solve --check` reports. `options` gives how the file is read and which level; its
/// `solver` is not read, as `tool` gives the tool's settings, in order, a later one winning by key.
/// A problem of `tool` is an error of every level; one of a file, a warning of the levels it
/// applies to, but a SOLVER-CONFIG that the reading fails is an error. Err when the file cannot be
/// read, or no level has the title `options.level`.
///
/// A viewer that checks its own settings text calls it without a file:
/// `check_settings(None, &provider, &SolveOptions::default(), &[ToolSettings::Options(text)])`.
pub fn check_settings(
    file: Option<&Path>,
    files: &dyn Provider,
    options: &SolveOptions,
    tool: &[ToolSettings],
) -> Result<Checked, String> {
    let (settings, tool_errors) = tool_settings(tool);
    let mut levels = Vec::new();
    match file {
        None => levels.push(level(None, &settings, &tool_errors, None, Vec::new())),
        Some(file) => {
            let runtime = Options {
                std: options.std,
                include_std: options.include_std,
                solver_config: settings
                    .iter()
                    .filter(|(s, _)| !rnl_solver::is_run_setting(&s.key))
                    .map(|(s, _)| s.clone())
                    .collect(),
                ..Options::solver()
            };
            let r = crate::interpret_file_with(file, options.dialect.as_deref(), &options.formats, files, &runtime)?;
            for p in r.problems.iter().filter(|p| options.level.as_ref().is_none_or(|t| *t == p.title)) {
                let read = reading_problems(&r, p);
                levels.push(level(Some((&r, p)), &settings, &tool_errors, Some(p.title.clone()), read));
            }
            if levels.is_empty() {
                if let Some(title) = &options.level {
                    return Err(format!("no drawing level is titled {title}"));
                }
            }
        }
    }
    Ok(Checked { ok: levels.iter().all(|l| l.errors.is_empty()), levels })
}

/// The tool's settings, merged by key, the last winning, each with whether it comes from an old
/// `--solver-options` text; and the problems of the texts, all of them.
fn tool_settings(tool: &[ToolSettings]) -> (Vec<(SolverSetting, bool)>, Vec<CheckedProblem>) {
    let mut merged = BTreeMap::new();
    let mut problems = Vec::new();
    for text in tool {
        let (old, text) = match text {
            ToolSettings::Config(text) => (false, text),
            ToolSettings::Options(text) => (true, text),
        };
        let (settings, found) = if old && !named_syntax(text) {
            SolverOptions::legacy_settings(text)
        } else {
            match rnl_runtime::solver_settings::parse_overrides(text) {
                Ok(settings) => (settings, Vec::new()),
                Err(e) => (Vec::new(), vec![SettingProblem::new(None, e.message)]),
            }
        };
        let option = if old { "--solver-options" } else { "--solver-config" };
        problems.extend(found.into_iter().map(|p| CheckedProblem::new(p.key, format!("{option}: {}", p.message))));
        merged.extend(settings.into_iter().map(|s| (s.key.clone(), (s, old))));
    }
    (merged.into_values().collect(), problems)
}

/// One level checked: `at` is the reading and the level's problem, none without a file.
fn level(
    at: Option<(&Interpretation, &LevelProblem)>,
    tool: &[(SolverSetting, bool)],
    tool_errors: &[CheckedProblem],
    title: Option<String>,
    read: Vec<(Severity, CheckedProblem)>,
) -> CheckedLevel {
    let check = SolverOptions::check_settings(tool, at.map(|(_, p)| p));
    // the value of the file of index k
    let file = |k: usize| {
        at.map(|(r, p)| {
            let s = &p.solver_config[k];
            let written = s.source.as_ref().map(|l| format!("{}:{}", l.file, l.line));
            CheckedSetting { value: json(&s.value), from: origin(r, s), at: written }
        })
    };
    let settings = check
        .values
        .into_iter()
        .map(|(key, (value, from))| {
            let other = |from: &str| CheckedSetting { value: json(&value), from: from.into(), at: None };
            let setting = match from {
                SettingFrom::Default => other("default"),
                SettingFrom::Tool => other("command-line"),
                SettingFrom::File(k) => file(k).unwrap_or_else(|| other("level")),
            };
            (key, setting)
        })
        .collect();
    let problem = |p: SettingProblem| CheckedProblem {
        refused: p.refused.and_then(file),
        ..CheckedProblem::new(p.key, p.message)
    };
    let mut warnings: Vec<CheckedProblem> = check.warnings.into_iter().map(problem).collect();
    let mut errors: Vec<CheckedProblem> = tool_errors.to_vec();
    errors.extend(check.errors.into_iter().map(problem));
    for (severity, problem) in read {
        if severity == Severity::Error {
            errors.push(problem);
        } else {
            warnings.push(problem);
        }
    }
    CheckedLevel { title, settings, warnings, errors }
}

/// Where a setting of the level's own comes from: a package by its name, or the record it is
/// written in, the file's header or the level.
fn origin(r: &Interpretation, s: &SolverSetting) -> String {
    if !s.package.is_empty() {
        return format!("package:{}", s.package);
    }
    let header = s.source.as_ref().is_some_and(|at| record_at(r, at).is_some_and(|rec| record_type(rec) == "header"));
    if header { "header" } else { "level" }.to_string()
}

/// The record of the document whose lines hold `at`.
fn record_at<'a>(r: &'a Interpretation, at: &Loc) -> Option<&'a rnl_runtime::host::Record> {
    r.doc
        .levels
        .iter()
        .flat_map(|l| &l.records)
        .find(|rec| rec.file == at.file && rec.lines.0 <= at.line && at.line <= rec.lines.1)
}

/// What the reading says of the SOLVER-CONFIG that `p` reads: of the level's own records, and of
/// the headers and packages, which apply to every level.
fn reading_problems(r: &Interpretation, p: &LevelProblem) -> Vec<(Severity, CheckedProblem)> {
    let own = &r.doc.levels[p.level].records;
    r.diagnostics()
        .into_iter()
        .filter(|d| d.message.contains("SOLVER-CONFIG"))
        .filter(|d| {
            // a diagnostic in a record of another level is that level's
            let Some(at) = &d.loc else { return true };
            match record_at(r, at) {
                Some(rec) if record_type(rec) != "header" => own.iter().any(|o| std::ptr::eq(o, rec)),
                _ => true,
            }
        })
        .map(|d| {
            let place = d.loc.as_ref().map(|l| format!("{}:{}: ", l.file, l.line)).unwrap_or_default();
            (d.severity, CheckedProblem::new(None, format!("{place}{}", d.message)))
        })
        .collect()
}

/// An RNL literal as JSON: a whole number as an integer.
fn json(value: &Lit) -> serde_json::Value {
    match value {
        Lit::Num(n) if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 => serde_json::json!(*n as i64),
        Lit::Num(n) => serde_json::json!(n),
        Lit::Str(s) => serde_json::json!(s.as_str()),
        Lit::Bool(b) => serde_json::json!(b),
        Lit::Arr(a) => serde_json::Value::Array(a.iter().map(json).collect()),
    }
}

/// What `rnl solve --check` prints without `--json`: each level's settings, one a line with where
/// it comes from, and its warnings and errors.
pub fn check_lines(checked: &Checked) -> Vec<String> {
    let mut out = Vec::new();
    for level in &checked.levels {
        if let Some(title) = &level.title {
            out.push(format!("level {title}:"));
        }
        let indent = if level.title.is_some() { "  " } else { "" };
        for (key, s) in &level.settings {
            let at = s.at.as_ref().map(|at| format!(", {at}")).unwrap_or_default();
            out.push(format!("{indent}{key}={} ({}{at})", s.value, s.from));
        }
        for (kind, problems) in [("warning", &level.warnings), ("error", &level.errors)] {
            for p in problems {
                let key = p.key.as_ref().map(|k| format!("{k}: ")).unwrap_or_default();
                out.push(format!("{indent}{kind}: {key}{}", p.message));
            }
        }
    }
    out
}
