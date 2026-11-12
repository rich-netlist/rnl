//! Solver-owned values of SOLVER-CONFIG. The runtime has already applied package, header
//! and level precedence; explicitly supplied tool values come last.

use rnl_runtime::interp::obj::Lit;
use rnl_runtime::problem::{LevelProblem, SolverSetting};

use crate::{RunLimit, SolverOptions};
use std::collections::BTreeMap;

/// The layout engine used for a 0.4 level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    #[default]
    Rows,
    Tree,
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

fn apply_tool(o: &mut SolverOptions, settings: &[SolverSetting]) -> Result<(), String> {
    for s in settings {
        if known(&s.key) {
            apply(o, s)?;
        }
    }
    let find = |key: &str| settings.iter().find(|s| s.key == key);
    let seconds = find("seconds").map(number).transpose()?;
    if seconds.is_some_and(|s| s < 0.0) {
        return Err("seconds must not be negative".into());
    }
    let work = find("work").map(|s| count(s, u64::MAX)).transpose()?;
    let mode = find("limit").map(string).transpose()?;
    o.run_limit = match mode {
        Some("time") => Some(RunLimit::Time(seconds.unwrap_or(10.0))),
        Some("seed") => Some(RunLimit::Seed),
        Some("work") => Some(RunLimit::Work(work.ok_or("limit=\"work\" requires work=COUNT")?)),
        Some(_) => return Err("limit must be \"time\", \"work\" or \"seed\"".into()),
        None if seconds.is_some() && work.is_some() => {
            return Err("give limit when both seconds and work are supplied".into())
        }
        None if seconds.is_some() => Some(RunLimit::Time(seconds.unwrap())),
        None if work.is_some() => Some(RunLimit::Work(work.unwrap())),
        None => o.run_limit,
    };
    o.validate()
}

impl SolverOptions {
    /// Defaults in the syntax accepted by --solver-config. Layout stays rows until the 0.4 switch.
    pub fn defaults_config() -> String {
        let d = Self::default();
        format!("layout=\"rows\", spacing_x=0, spacing_y=0, limit=\"time\", seconds=10, safety_seconds=30, tolerance={}, grid=1, beam={}, candidates={}, astar_expansions={}", rnl_runtime::geom::fmt_num(d.tolerance), d.beam, d.budget.candidates, d.budget.astar_expansions)
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

    /// Materialize effective settings. Bad file values warn and keep the previous/default value.
    pub fn for_problem(&self, p: &LevelProblem) -> (Self, Vec<String>) {
        let mut out = self.clone();
        let mut warnings = Vec::new();
        if p.version != "0.4" {
            return (out, warnings);
        }
        if self.settings.legacy_alias {
            warnings.push("--solver-options is deprecated; use --solver-config with RNL named parameters".into());
        }
        for s in &p.solver_config {
            if is_run_setting(&s.key) {
                warnings.push(format!("{} is a tool-only run setting and is ignored in the file", s.key));
                continue;
            }
            if self.settings.overrides.iter().any(|o| o.key == s.key) {
                continue;
            }
            let mut next = out.clone();
            match apply(&mut next, s) {
                Ok(()) => out = next,
                Err(e) => warnings.push(format!("{e}; ignored")),
            }
        }
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
        if out.settings.spacing != [0, 0] {
            warnings.push(
                "spacing_x and spacing_y are retained for line layout; row and tree layouts keep their fixed gaps"
                    .into(),
            );
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
        assert_eq!(warnings.len(), 5);
        assert!(warnings.iter().filter(|w| w.contains("tool-only")).count() == 2);
        assert_eq!(SolverOptions::parse(&effective.canonical()).unwrap(), effective);
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
