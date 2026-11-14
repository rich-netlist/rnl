//! Solver-owned values of SOLVER-CONFIG. The runtime gives every value of each key, in the order
//! of 14.10 (packages, the file's header, the level), having left out the values it does not take
//! of the keys the standard names; the solver judges the values of its own keys in that order,
//! and a value it does not take fails its key alone, the value before it staying in effect. So of
//! each key the last value the solver accepts is in effect. Explicitly supplied tool values come
//! last and replace the file's values of their keys.

use rnl_runtime::interp::obj::Lit;
use rnl_runtime::problem::{LevelProblem, SolverSetting};

use crate::{RunLimit, SolverOptions};
use std::collections::BTreeMap;

/// The layout engine used for a 0.4 level: the lines layout unless another is asked for. A level
/// of an earlier version is laid out in rows whatever is asked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    Rows,
    Tree,
    #[default]
    Lines,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    pub layout: Layout,
    /// The definition literal, normalized after the runtime resolves it for writing.
    pub junction: Option<String>,
    pub legacy_alias: bool,
    pub spacing: [i32; 2],
    /// Only values explicitly supplied by the caller. Defaults must not mask file values.
    pub overrides: Vec<SolverSetting>,
}

pub fn is_run_setting(key: &str) -> bool {
    matches!(
        key,
        "limit"
            | "work"
            | "seconds"
            | "safety_seconds"
            | "candidates"
            | "astar_expansions"
            | "time_limit"
            | "report_hints"
    )
}

fn known(key: &str) -> bool {
    is_run_setting(key)
        || matches!(
            key,
            "junction" | "layout" | "spacing_x" | "spacing_y" | "tolerance" | "grid" | "beam" | "objectives"
        )
}

fn number(s: &SolverSetting) -> Result<f64, String> {
    match &s.value {
        Lit::Num(n) if n.is_finite() => Ok(*n),
        _ => Err(format!("{} must be a finite number", s.key)),
    }
}

fn count(s: &SolverSetting, max: u64) -> Result<u64, String> {
    let n = number(s)?;
    // RNL numbers are binary64. Exact integer counts above 2^53 cannot be expressed.
    if n < 0.0 || n.fract() != 0.0 || n > max.min(1 << 53) as f64 {
        return Err(format!("{} must be a nonnegative integer no greater than {}", s.key, max.min(1 << 53)));
    }
    Ok(n as u64)
}

fn string(s: &SolverSetting) -> Result<&str, String> {
    match &s.value {
        Lit::Str(v) => Ok(v.as_str()),
        _ => Err(format!("{} must be a string", s.key)),
    }
}

fn apply(o: &mut SolverOptions, s: &SolverSetting) -> Result<(), String> {
    match s.key.as_str() {
        "junction" => {
            o.settings.junction = Some(string(s)?.to_string());
        }
        "layout" => {
            o.settings.layout = match string(s)? {
                "rows" => Layout::Rows,
                "tree" => Layout::Tree,
                "lines" => Layout::Lines,
                _ => return Err("layout must be \"rows\", \"tree\", or \"lines\"".into()),
            }
        }
        "spacing_x" | "spacing_y" => {
            let n = number(s)?;
            if n.fract() != 0.0 || !(-1.0..=20.0).contains(&n) {
                return Err(format!("{} must be an integer from -1 to 20", s.key));
            }
            o.settings.spacing[usize::from(s.key == "spacing_y")] = n as i32;
        }
        "tolerance" => o.tolerance = number(s)?,
        "grid" => o.grid = if s.value == Lit::Bool(false) { None } else { Some(number(s)?) },
        "beam" => o.beam = count(s, 1024)? as usize,
        "objectives" => o.objectives = o.with_overrides(&format!("objectives={}", string(s)?))?.objectives,
        "time_limit" => {
            o.time_limit = if s.value == Lit::Bool(false) { None } else { Some(number(s)?) };
            o.run_limit = Some(o.time_limit.map_or(RunLimit::Unlimited, RunLimit::Time));
        }
        "report_hints" => {
            let Lit::Bool(value) = s.value else { return Err("report_hints must be a boolean".into()) };
            o.hints_for_low = value;
        }
        "candidates" => o.budget.candidates = count(s, u64::MAX)?,
        "astar_expansions" => o.budget.astar_expansions = count(s, u64::MAX)?,
        "safety_seconds" => o.safety_seconds = Some(number(s)?),
        "limit" | "work" | "seconds" => {} // validated together, independent of key order
        _ => return Err(format!("unknown solver setting {}", s.key)),
    }
    o.validate()
}

/// A problem with a setting, or with the settings together when `key` is None.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingProblem {
    pub key: Option<String>,
    pub message: String,
    /// The value of the file that is refused, by its index in the level's settings
    /// ([`LevelProblem::solver_config`]).
    pub refused: Option<usize>,
}

impl SettingProblem {
    pub fn new(key: Option<&str>, message: impl Into<String>) -> SettingProblem {
        SettingProblem { key: key.map(str::to_string), message: message.into(), refused: None }
    }
}

/// A value of the file as a warning names it: `key=value`, and where it is written.
fn written(s: &SolverSetting) -> String {
    let at = s.source.as_ref().map(|l| format!(" at {}:{}", l.file, l.line)).unwrap_or_default();
    format!("{}={}{at}", s.key, s.value.show())
}

/// The file's settings applied key by key ([`file_settings`]).
struct FileSettings {
    /// Each value in effect, by its index in the level's settings.
    taken: Vec<usize>,
    /// Each value not taken, by its index, with the problem, in the order of the settings.
    refused: Vec<SettingProblem>,
    /// The tool-only run settings the file gives, each key once, which are ignored.
    run: Vec<String>,
}

/// The file's settings applied to `o` in their order (14.10), of each key that `tool` does not
/// give: a value the solver accepts replaces the one before it, and a value it does not take
/// fails its key alone, saying so, and the value before it stays in effect, or the default. So of
/// each key the last value the solver accepts is in effect.
fn file_settings(o: &mut SolverOptions, settings: &[SolverSetting], tool: &[SolverSetting]) -> FileSettings {
    let mut out = FileSettings { taken: Vec::new(), refused: Vec::new(), run: Vec::new() };
    let mut taken: BTreeMap<&str, usize> = BTreeMap::new();
    for (k, s) in settings.iter().enumerate() {
        if is_run_setting(&s.key) {
            if !out.run.contains(&s.key) {
                out.run.push(s.key.clone());
            }
            continue;
        }
        if tool.iter().any(|t| t.key == s.key) {
            continue;
        }
        let mut next = o.clone();
        match apply(&mut next, s) {
            Ok(()) => {
                *o = next;
                taken.insert(&s.key, k);
            }
            Err(e) => {
                let before = taken.get(s.key.as_str()).map_or("the default".to_string(), |&j| written(&settings[j]));
                out.refused.push(SettingProblem {
                    key: Some(s.key.clone()),
                    message: format!("{e}; {} is left out, and {before} stays in effect", written(s)),
                    refused: Some(k),
                });
            }
        }
    }
    out.taken = taken.into_values().collect();
    out
}

/// Where an effective setting comes from (14.10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingFrom {
    /// The solver's default ([`SolverOptions::defaults_config`]).
    Default,
    /// The level's own settings, [`LevelProblem::solver_config`], by the index of the value in
    /// effect: from a package, the file's header or the level.
    File(usize),
    /// The tool: a command line, or a viewer.
    Tool,
}

/// The settings a solve of a level takes, and what is wrong with them
/// ([`SolverOptions::check_settings`]).
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsCheck {
    /// Each setting in effect, by key: its value, and where it comes from.
    pub values: BTreeMap<String, (Lit, SettingFrom)>,
    /// What a solve ignores, saying so: a bad value or a run setting in the file, an unknown key.
    pub warnings: Vec<SettingProblem>,
    /// What a solve refuses: the tool's bad values. A solve with them does not start.
    pub errors: Vec<SettingProblem>,
}

/// Every problem of the tool's settings, each with its key, rather than the first: the settings
/// without one are applied to `o` in order, and the run limit is made of limit, work and seconds
/// together when none of them has one.
fn tool_problems(o: &mut SolverOptions, settings: &[SolverSetting]) -> Vec<SettingProblem> {
    let mut out = Vec::new();
    for s in settings.iter().filter(|s| known(&s.key)) {
        let mut next = o.clone();
        match apply(&mut next, s) {
            Ok(()) => *o = next,
            Err(e) => out.push(SettingProblem::new(Some(&s.key), e)),
        }
    }
    let find = |key: &str| settings.iter().find(|s| s.key == key);
    let mut value = |key: &'static str, check: &dyn Fn(&SolverSetting) -> Result<(), String>| {
        let problem = find(key).map(check).and_then(Result::err);
        out.extend(problem.map(|e| SettingProblem::new(Some(key), e)));
    };
    value("seconds", &|s| match number(s)? {
        n if n < 0.0 => Err("seconds must not be negative".into()),
        _ => Ok(()),
    });
    value("work", &|s| count(s, u64::MAX).map(drop));
    value("limit", &|s| match string(s)? {
        "time" | "seed" | "work" => Ok(()),
        _ => Err("limit must be \"time\", \"work\" or \"seed\"".into()),
    });
    if out.iter().any(|p| matches!(p.key.as_deref(), Some("seconds" | "work" | "limit"))) {
        return out;
    }
    // each of them is now a value of its kind, or not given
    let seconds = find("seconds").and_then(|s| number(s).ok());
    let work = find("work").and_then(|s| count(s, u64::MAX).ok());
    let limit = match find("limit").and_then(|s| string(s).ok()) {
        Some("time") => Some(RunLimit::Time(seconds.unwrap_or(10.0))),
        Some("seed") => Some(RunLimit::Seed),
        Some(_) => match work {
            Some(work) => Some(RunLimit::Work(work)),
            None => {
                out.push(SettingProblem::new(Some("limit"), "limit=\"work\" requires work=COUNT"));
                return out;
            }
        },
        None => match (seconds, work) {
            (Some(_), Some(_)) => {
                out.push(SettingProblem::new(None, "give limit when both seconds and work are supplied"));
                return out;
            }
            (Some(seconds), None) => Some(RunLimit::Time(seconds)),
            (None, Some(work)) => Some(RunLimit::Work(work)),
            (None, None) => o.run_limit,
        },
    };
    o.run_limit = limit;
    for (key, message) in o.problems() {
        if !out.iter().any(|p| p.message == message) {
            out.push(SettingProblem::new(Some(key), message));
        }
    }
    out
}

/// The tool's settings applied to `o`, or the problems of all of them, joined.
fn apply_tool(o: &mut SolverOptions, settings: &[SolverSetting]) -> Result<(), String> {
    let problems = tool_problems(o, settings);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.into_iter().map(|p| p.message).collect::<Vec<_>>().join("; "))
    }
}

/// The warning of a spacing given to a solve by rows or by the group tree: only the lines layout
/// spaces by it.
pub(crate) const SPACING_NOT_USED: &str =
    "spacing_x and spacing_y are retained for line layout; row and tree layouts keep their fixed gaps";

impl SolverOptions {
    /// Defaults in the syntax accepted by --solver-config.
    pub fn defaults_config() -> String {
        let d = Self::default();
        format!("layout=\"lines\", spacing_x=0, spacing_y=0, limit=\"time\", seconds=10, safety_seconds=30, tolerance={}, grid=1, beam={}, candidates={}, astar_expansions={}", rnl_runtime::geom::fmt_num(d.tolerance), d.beam, d.budget.candidates, d.budget.astar_expansions)
    }
    /// Parse tool settings using RNL literal syntax; reject invalid values before solving.
    pub fn with_config(&self, text: &str) -> Result<Self, String> {
        let mut values: BTreeMap<_, _> = self.settings.overrides.iter().cloned().map(|s| (s.key.clone(), s)).collect();
        for s in rnl_runtime::solver_settings::parse_overrides(text).map_err(|e| e.message)? {
            values.insert(s.key.clone(), s);
        }
        let overrides: Vec<_> = values.into_values().collect();
        let mut checked = self.clone();
        apply_tool(&mut checked, &overrides)?;
        let mut out = self.clone();
        out.settings.overrides = overrides;
        Ok(out)
    }

    /// Transitional old spelling, retaining which keys were explicitly supplied by the tool.
    pub fn with_legacy_config(&self, text: &str) -> Result<Self, String> {
        let parsed = self.with_overrides(text)?;
        let mut settings = Vec::new();
        for item in text.split(';').filter(|s| !s.trim().is_empty()) {
            let (key, value) = item.split_once('=').ok_or("expected key=value")?;
            let (key, value) = (key.trim(), value.trim());
            match key {
                "budget" => settings.extend(value.split(',').map(|p| p.replace(':', "="))),
                "layout" => settings.push(format!("layout=\"{value}\"")),
                "objectives" => settings
                    .push(format!("objectives={}", Lit::Str(rnl_runtime::interp::obj::WStr::new(value, None)).show())),
                "grid" | "time_limit" if value == "none" => settings.push(format!("{key}=false")),
                "junction_utf8" => settings.push(format!(
                    "junction={}",
                    Lit::Str(rnl_runtime::interp::obj::WStr::new(
                        parsed.settings.junction.as_deref().unwrap_or_default(),
                        None
                    ))
                    .show()
                )),
                "run" | "search" => match parsed.run_limit {
                    Some(RunLimit::Seed) => settings.push("limit=\"seed\"".into()),
                    Some(RunLimit::Time(s)) => settings.push(format!("limit=\"time\",seconds={s}")),
                    Some(RunLimit::Work(n)) => settings.push(format!("limit=\"work\",work={n}")),
                    _ => settings.push("time_limit=false".into()),
                },
                _ => settings.push(format!("{key}={value}")),
            }
        }
        let mut out = parsed.with_config(&settings.join(","))?;
        out.settings.legacy_alias = true;
        Ok(out)
    }

    /// The settings of an old `--solver-options` text, key=value;…, as the named settings it
    /// stands for, with the problem of each item that is wrong, by its key, rather than the first.
    pub fn legacy_settings(text: &str) -> (Vec<SolverSetting>, Vec<SettingProblem>) {
        let mut values = BTreeMap::new();
        let mut problems = Vec::new();
        for item in text.split(';').filter(|s| !s.trim().is_empty()) {
            match Self::default().with_legacy_config(item) {
                Ok(o) => values.extend(o.settings.overrides.into_iter().map(|s| (s.key.clone(), s))),
                Err(message) => {
                    let key = item.split_once('=').map(|(key, _)| key.trim());
                    problems.push(SettingProblem::new(key, message));
                }
            }
        }
        (values.into_values().collect(), problems)
    }

    /// What a solve of `problem` would take, key by key, with where each setting comes from, and
    /// what is wrong (14.10): the solver's defaults, then the level's settings (packages, the
    /// file's header, the level), of each key the last value the solver accepts, then the tool's,
    /// `tool`, merged by key, each with whether it comes from an old `--solver-options` text.
    /// Without a problem, the tool's over the defaults. As in a solve, a value of the file that the
    /// solver does not take is a warning, which names the value that stays in effect instead, and
    /// a bad value of the tool is an error; a level before 0.4 reads no SOLVER-CONFIG, and of the tool only the
    /// old `--solver-options`.
    pub fn check_settings(tool: &[(SolverSetting, bool)], problem: Option<&LevelProblem>) -> SettingsCheck {
        let defaults = rnl_runtime::solver_settings::parse_overrides(&Self::defaults_config()).unwrap_or_default();
        let mut values: BTreeMap<_, _> =
            defaults.into_iter().map(|s| (s.key, (s.value, SettingFrom::Default))).collect();
        let mut warnings = Vec::new();
        if tool.iter().any(|(_, old)| *old) {
            warnings.push(SettingProblem::new(
                None,
                "--solver-options is deprecated; use --solver-config with RNL named parameters",
            ));
        }
        let modern = problem.is_none_or(|p| p.version == "0.4");
        let mut read = Vec::new();
        for (s, old) in tool {
            if modern || *old {
                read.push(s.clone());
            } else {
                let message = format!("a level before 0.4 reads no --solver-config; {} is ignored", s.key);
                warnings.push(SettingProblem::new(Some(&s.key), message));
            }
        }
        let mut o = Self::default();
        if let Some(p) = problem.filter(|_| modern) {
            let file = file_settings(&mut o, &p.solver_config, &read);
            for key in file.run {
                let message = format!("{key} is a tool-only run setting and is ignored in the file");
                warnings.push(SettingProblem::new(Some(&key), message));
            }
            for k in file.taken {
                let s = &p.solver_config[k];
                values.insert(s.key.clone(), (s.value.clone(), SettingFrom::File(k)));
            }
            warnings.extend(file.refused);
        }
        let errors = tool_problems(&mut o, &read);
        for s in &read {
            if !known(&s.key) {
                warnings.push(SettingProblem::new(Some(&s.key), format!("unknown solver setting {}; ignored", s.key)));
            } else if !errors.iter().any(|e| e.key.as_deref() == Some(s.key.as_str())) {
                values.insert(s.key.clone(), (s.value.clone(), SettingFrom::Tool));
            }
        }
        // the run limit as the tool's limit, work and seconds make it, together
        let run = ["limit", "work", "seconds"];
        if errors.iter().all(|e| e.key.as_deref().is_none_or(|k| !run.contains(&k)))
            && read.iter().any(|s| run.contains(&s.key.as_str()))
        {
            let limit = |mode: &str| Lit::Str(rnl_runtime::interp::obj::WStr::new(mode, None));
            let (mode, unused) = match o.run_limit {
                Some(RunLimit::Work(_)) => ("work", "seconds"),
                Some(RunLimit::Seed) => ("seed", "seconds"),
                _ => ("time", "work"),
            };
            values.insert("limit".into(), (limit(mode), SettingFrom::Tool));
            if values.get(unused).is_some_and(|(_, from)| *from == SettingFrom::Default) {
                values.remove(unused);
            }
        }
        if modern && o.settings.spacing != [0, 0] && o.settings.layout != Layout::Lines {
            warnings.push(SettingProblem::new(None, SPACING_NOT_USED));
        }
        if !modern {
            // the layouts are those of 0.4: an earlier level is laid out in rows whatever is asked
            if values.get("layout").is_some_and(|(_, from)| *from != SettingFrom::Default) {
                let message = "a level before 0.4 is laid out in rows; layout is ignored";
                warnings.push(SettingProblem::new(Some("layout"), message));
            }
            let rows = Lit::Str(rnl_runtime::interp::obj::WStr::new("rows", None));
            values.insert("layout".into(), (rows, SettingFrom::Default));
        }
        SettingsCheck { values, warnings, errors }
    }

    /// Materialize effective settings. Of each key of the file, the last value the solver accepts;
    /// a value it does not take warns, and the value before it, or the default, stays in effect.
    pub fn for_problem(&self, p: &LevelProblem) -> (Self, Vec<String>) {
        let mut out = self.clone();
        let mut warnings = Vec::new();
        if p.version != "0.4" {
            // the layouts are those of 0.4: an earlier level is laid out in rows, as it always was,
            // and the options its hash covers name no layout, as they never did
            out.settings.layout = Layout::default();
            return (out, warnings);
        }
        if self.settings.legacy_alias {
            warnings.push("--solver-options is deprecated; use --solver-config with RNL named parameters".into());
        }
        let file = file_settings(&mut out, &p.solver_config, &self.settings.overrides);
        for key in file.run {
            warnings.push(format!("{key} is a tool-only run setting and is ignored in the file"));
        }
        warnings.extend(file.refused.into_iter().map(|p| p.message));
        if let Err(e) = apply_tool(&mut out, &self.settings.overrides) {
            warnings.push(format!("invalid tool settings: {e}"));
        }
        for s in &self.settings.overrides {
            if !known(&s.key) {
                warnings.push(format!("unknown solver setting {}; ignored", s.key));
            }
        }
        if p.junction.tool_override.is_some() || out.settings.junction.is_some() {
            // Core-owned tool metadata can also come from another frontend. Hash the definition
            // actually selected for writeback, including an explicit choice to draw no dots.
            out.settings.junction = Some(p.junction.effective().definition.clone().unwrap_or_default());
        }
        if out.settings.spacing != [0, 0] && out.settings.layout != Layout::Lines {
            warnings.push(SPACING_NOT_USED.into());
        }
        out.settings.overrides.clear();
        (out, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::problem_of;

    #[test]
    fn every_problem_of_the_tool_settings_is_found_not_only_the_first() {
        let d = SolverOptions::default();
        // one problem reads as before; several are joined, in the order of the keys
        assert_eq!(d.with_config("beam=0").unwrap_err(), "the beam must be at least 1");
        assert_eq!(
            d.with_config("spacing_x=21, beam=0").unwrap_err(),
            "the beam must be at least 1; spacing_x must be an integer from -1 to 20"
        );
        let tool: Vec<_> =
            rnl_runtime::solver_settings::parse_overrides("beam=0, grid=0, work=-1, limit=\"time\", tolerance=0.5")
                .unwrap()
                .into_iter()
                .map(|s| (s, false))
                .collect();
        let check = SolverOptions::check_settings(&tool, None);
        let keys: Vec<_> = check.errors.iter().map(|e| e.key.as_deref().unwrap()).collect();
        assert_eq!(keys, ["beam", "grid", "work"]);
        // the good ones are in effect, from the tool; the bad ones are not
        assert_eq!(check.values["tolerance"], (Lit::Num(0.5), SettingFrom::Tool));
        assert_eq!(check.values["beam"].1, SettingFrom::Default);
    }

    #[test]
    fn a_refused_value_of_the_file_leaves_the_earlier_one_in_effect() {
        let settings = |text: &str| rnl_runtime::solver_settings::parse_overrides(text).unwrap();
        let mut p = problem_of(Vec::new(), Vec::new());
        p.version = "0.4".into();
        // as the runtime gives them: by key, the values of a key in the order of 14.10
        p.solver_config = [settings("beam=6"), settings("grid=2"), settings("grid=\"x\"")].concat();
        let (o, warnings) = SolverOptions::default().for_problem(&p);
        assert_eq!((o.grid, o.beam), (Some(2.0), 6));
        assert_eq!(warnings, ["grid must be a finite number; grid=\"x\" is left out, and grid=2 stays in effect"]);
        let check = SolverOptions::check_settings(&[], Some(&p));
        assert_eq!(check.values["grid"], (Lit::Num(2.0), SettingFrom::File(1)));
        assert_eq!(check.values["beam"], (Lit::Num(6.0), SettingFrom::File(0)));
        assert_eq!(check.warnings.len(), 1);
        assert_eq!((check.warnings[0].key.as_deref(), check.warnings[0].refused), (Some("grid"), Some(2)));
        // a bad value before a good one fails its place too, and the later one is in effect
        p.solver_config = [settings("grid=\"x\""), settings("grid=3")].concat();
        let (o, warnings) = SolverOptions::default().for_problem(&p);
        assert_eq!(o.grid, Some(3.0));
        assert_eq!(warnings, ["grid must be a finite number; grid=\"x\" is left out, and the default stays in effect"]);
        // each value not taken warns, in order, and with none taken the default stays
        p.solver_config = [settings("grid=0"), settings("grid=\"x\"")].concat();
        let (o, warnings) = SolverOptions::default().for_problem(&p);
        assert_eq!(o.grid, SolverOptions::default().grid);
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("grid=0 is left out, and the default stays"), "{warnings:?}");
        assert!(warnings[1].contains("grid=\"x\" is left out, and the default stays"), "{warnings:?}");
        // the tool's value replaces every value of the file's
        let (o, warnings) = SolverOptions::default().with_config("grid=5").unwrap().for_problem(&p);
        assert_eq!(o.grid, Some(5.0));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn validate_gives_the_first_of_the_problems() {
        let o = SolverOptions { beam: 0, tolerance: -1.0, ..Default::default() };
        let keys: Vec<_> = o.problems().into_iter().map(|(key, _)| key).collect();
        assert_eq!(keys, ["tolerance", "beam"]);
        assert_eq!(o.validate().unwrap_err(), "the tolerance must be a number not below 0, not -1");
    }

    #[test]
    fn explicit_tools_override_files_without_default_values_masking_them() {
        let mut p = problem_of(Vec::new(), Vec::new());
        p.version = "0.4".into();
        p.solver_config = rnl_runtime::solver_settings::parse_overrides(
            "spacing_x=3,spacing_y=4,limit=\"work\",work=0,beam=0,unknown=true",
        )
        .unwrap();
        let o = SolverOptions::default().with_config("spacing_y=6,seconds=2").unwrap();
        let (effective, warnings) = o.for_problem(&p);
        assert_eq!(effective.settings.spacing, [3, 6]);
        assert_eq!(effective.beam, 4);
        assert_eq!(effective.run_limit, Some(RunLimit::Time(2.0)));
        // the lines layout, the default, spaces by the spacing: no warning of it
        assert_eq!(warnings.len(), 4, "{warnings:?}");
        assert!(warnings.iter().filter(|w| w.contains("tool-only")).count() == 2);
        assert_eq!(SolverOptions::parse(&effective.canonical()).unwrap(), effective);
        // the rows and the group tree keep their gaps, and say so
        for layout in ["rows", "tree"] {
            let (_, warnings) = o.with_config(&format!("layout=\"{layout}\"")).unwrap().for_problem(&p);
            assert_eq!(warnings.len(), 5, "{layout} {warnings:?}");
            assert!(warnings.contains(&SPACING_NOT_USED.to_string()), "{layout} {warnings:?}");
        }
    }

    #[test]
    fn defaults_and_budget_values_are_validated_before_solving() {
        let d = SolverOptions::default();
        assert!(d.with_config(&SolverOptions::defaults_config()).is_ok());
        let mut p = problem_of(Vec::new(), Vec::new());
        p.version = "0.4".into();
        assert_eq!(d.with_config("limit=\"seed\"").unwrap().for_problem(&p).0.run_limit, Some(RunLimit::Seed));
        assert_eq!(d.with_legacy_config("search=seed").unwrap().for_problem(&p).0.run_limit, Some(RunLimit::Seed));
        assert_eq!(SolverOptions::parse("run=seed").unwrap().run_limit, Some(RunLimit::Seed));
        for bad in [
            "work=-1",
            "work=1.5",
            "work=9007199254740994",
            "seconds=-1",
            "limit=\"work\"",
            "spacing_x=21",
            "layout=\"missing\"",
            "beam=0",
        ] {
            assert!(d.with_config(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn repeated_tools_and_the_old_alias_merge_per_key_after_files() {
        let mut p = problem_of(Vec::new(), Vec::new());
        p.version = "0.4".into();
        p.solver_config = rnl_runtime::solver_settings::parse_overrides("beam=3,spacing_x=9").unwrap();
        let options = SolverOptions::default()
            .with_legacy_config("beam=8;time_limit=1")
            .unwrap()
            .with_config("spacing_x=3,seconds=2")
            .unwrap()
            .with_config("spacing_y=4")
            .unwrap();
        let (effective, _) = options.for_problem(&p);
        assert_eq!(effective.beam, 8);
        assert_eq!(effective.settings.spacing, [3, 4]);
        assert_eq!(effective.run_limit, Some(RunLimit::Time(2.0)));
        let (last, _) = options.with_config("beam=6").unwrap().with_legacy_config("beam=7").unwrap().for_problem(&p);
        assert_eq!(last.beam, 7);
    }

    #[test]
    fn junction_identity_round_trips_quoted_unicode_and_separators() {
        let options = SolverOptions {
            settings: Settings { junction: Some("$\"点;名\"".into()), ..Default::default() },
            ..Default::default()
        };
        assert_eq!(SolverOptions::parse(&options.canonical()).unwrap(), options);
        for bad in ["junction_utf8=0", "junction_utf8=ff", "junction_utf8=zz", "junction_utf8=点"] {
            assert!(SolverOptions::parse(bad).is_err(), "{bad}");
        }
    }
}
