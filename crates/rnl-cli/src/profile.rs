//! Durable, throttled JSON snapshots of solver observations.
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use rnl_solver::profile::{ProfileEvent, ProfileHandle, ProfileObserver};
use rnl_solver::Clock;
use serde::Serialize;
use serde_json::{json, Value};

use crate::solve::WallClock;

#[derive(Clone, Serialize)]
struct Span {
    id: usize,
    parent: Option<usize>,
    category: String,
    name: String,
    candidate_id: Option<u64>,
    started_seconds: f64,
    elapsed_seconds: f64,
    complete: bool,
    metrics: BTreeMap<String, Value>,
}

#[derive(Clone, Serialize)]
struct Level {
    level_id: usize,
    title: String,
    problem_hash: String,
    options_canonical: String,
    objects: usize,
    facts: usize,
    routes: usize,
    status: String,
    current_stage: Option<String>,
    started_seconds: f64,
    total_seconds: f64,
    stage_seconds: BTreeMap<String, f64>,
    metrics: BTreeMap<String, Value>,
    spans: Vec<Span>,
}

#[derive(Default)]
struct State {
    levels: Vec<Level>,
    stack: Vec<usize>,
    status: String,
    last_write: f64,
    checkpoints: u64,
    written_bytes: u64,
    error: Option<String>,
}

struct Writer {
    clock: WallClock,
    path: PathBuf,
    state: RefCell<State>,
    work: bool,
}

/// A wall clock with profiling. Deadline readings and profiling readings use the same source,
/// through independent `Clock` calls; profiling never calls the deadline clock on its behalf.
pub struct ProfileClock(Rc<Writer>);

impl ProfileClock {
    pub fn new(path: &Path) -> Result<Self, String> {
        Self::create(path, false)
    }

    /// Explicit diagnostic mode: coarse batch accounting without stored-tableau bit scans.
    pub fn new_work(path: &Path) -> Result<Self, String> {
        Self::create(path, true)
    }

    fn create(path: &Path, work: bool) -> Result<Self, String> {
        // Reserve only a new path: a profile must never replace an input/include/package file.
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| format!("cannot create new profile {}: {e}", path.display()))?;
        let writer = Rc::new(Writer {
            clock: WallClock::new(),
            path: path.to_path_buf(),
            state: RefCell::new(State { status: "running".into(), ..State::default() }),
            work,
        });
        writer.flush(true);
        if let Some(error) = writer.state.borrow().error.clone() {
            return Err(error);
        }
        Ok(Self(writer))
    }

    /// Finishes even an unsuccessful run, retaining all completed and partial observations.
    pub fn finish(&self, success: bool) -> Result<(), String> {
        self.0.state.borrow_mut().status = if success { "complete" } else { "error" }.into();
        self.0.flush(true);
        match &self.0.state.borrow().error {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }
}

impl Clock for ProfileClock {
    fn elapsed(&self) -> f64 {
        self.0.clock.elapsed()
    }
    fn profiler(&self) -> Option<ProfileHandle> {
        Some(self.0.clone())
    }
}

impl Clock for Writer {
    fn elapsed(&self) -> f64 {
        self.clock.elapsed()
    }
}

fn metric(metrics: &mut BTreeMap<String, Value>, name: &str, value: u64, mode: u8) -> bool {
    let old = metrics.get(name).and_then(Value::as_u64).unwrap_or(0);
    let overflow = mode == 0 && old.checked_add(value).is_none();
    let value = match mode {
        0 => old.saturating_add(value),
        1 => old.max(value),
        _ => value,
    };
    metrics.insert(name.into(), json!(value));
    overflow
}

fn work_metrics(
    metrics: &mut BTreeMap<String, Value>,
    counters: &[(&str, u64)],
    maxima: &[(&str, u64)],
    gauges: &[(&str, u64)],
) {
    let mut overflow = false;
    for (values, mode) in [(counters, 0), (maxima, 1), (gauges, 2)] {
        for &(name, value) in values {
            overflow |= metric(metrics, name, value, mode);
        }
    }
    if overflow {
        metric(metrics, "work_counter_overflow", 1, 1);
    }
}

impl ProfileObserver for Writer {
    fn work_diagnostics(&self) -> bool {
        self.work
    }

    fn record(&self, event: ProfileEvent) {
        let mut force = false;
        let mut checkpoint = false;
        let mut state = self.state.borrow_mut();
        match event {
            ProfileEvent::Level { level, title, objects, facts, routes, options, hash } => {
                let now = self.elapsed();
                state.stack.clear();
                state.levels.push(Level {
                    level_id: level,
                    title,
                    objects,
                    facts,
                    routes,
                    options_canonical: options,
                    problem_hash: hash,
                    status: "running".into(),
                    current_stage: None,
                    started_seconds: now,
                    total_seconds: 0.0,
                    stage_seconds: ["resolve", "structure", "search", "coords", "route", "score"]
                        .into_iter()
                        .map(|k| (k.into(), 0.0))
                        .collect(),
                    metrics: BTreeMap::new(),
                    spans: Vec::new(),
                });
                force = true;
            }
            ProfileEvent::LevelEnd => {
                let now = self.elapsed();
                if let Some(level) = state.levels.last_mut() {
                    level.total_seconds = now - level.started_seconds;
                    level.status = "complete".into();
                    level.current_stage = None;
                }
                force = true;
            }
            ProfileEvent::Begin { category, name } => {
                let now = self.elapsed();
                let parent = state.stack.last().copied();
                if let Some(level) = state.levels.last_mut() {
                    let candidate_id = parent.and_then(|i| {
                        level.spans[i]
                            .metrics
                            .get("candidate_id")
                            .and_then(Value::as_u64)
                            .or(level.spans[i].candidate_id)
                    });
                    let id = level.spans.len();
                    if category == "stage" {
                        level.current_stage = Some(name.clone());
                    }
                    level.spans.push(Span {
                        id,
                        parent,
                        category: category.into(),
                        name,
                        candidate_id,
                        started_seconds: now,
                        elapsed_seconds: 0.0,
                        complete: false,
                        metrics: BTreeMap::new(),
                    });
                    state.stack.push(id);
                }
                force = category == "stage" || category == "resolve_phase";
                checkpoint = true;
            }
            ProfileEvent::End => {
                let now = self.elapsed();
                if let Some(i) = state.stack.pop() {
                    if let Some(level) = state.levels.last_mut() {
                        let span = &mut level.spans[i];
                        span.elapsed_seconds = now - span.started_seconds;
                        span.complete = true;
                        if span.category == "stage" {
                            *level.stage_seconds.entry(span.name.clone()).or_default() += span.elapsed_seconds;
                            level.current_stage = None;
                        }
                    }
                }
                checkpoint = true;
            }
            ProfileEvent::Add { name, value }
            | ProfileEvent::Max { name, value }
            | ProfileEvent::Set { name, value } => {
                let mode = match event {
                    ProfileEvent::Add { .. } => 0,
                    ProfileEvent::Max { .. } => 1,
                    _ => 2,
                };
                let stack = state.stack.clone();
                if let Some(level) = state.levels.last_mut() {
                    metric(&mut level.metrics, name, value, mode);
                    for i in stack {
                        metric(&mut level.spans[i].metrics, name, value, mode);
                    }
                }
            }
            ProfileEvent::Text { name, value } => {
                let stack = state.stack.clone();
                if let Some(level) = state.levels.last_mut() {
                    level.metrics.insert(name.into(), json!(value));
                    for i in stack {
                        level.spans[i].metrics.insert(name.into(), json!(value));
                    }
                }
            }
            ProfileEvent::Work { counters, maxima, gauges } => {
                let stack = state.stack.clone();
                if let Some(level) = state.levels.last_mut() {
                    work_metrics(&mut level.metrics, &counters, &maxima, &gauges);
                    for i in stack {
                        work_metrics(&mut level.spans[i].metrics, &counters, &maxima, &gauges);
                    }
                }
            }
            // Hot simplex/resolver checkpoints do not serialize work diagnostics. Existing
            // coarse Begin/End boundaries still checkpoint; there is no refresh-time guarantee.
            ProfileEvent::Checkpoint => checkpoint = !self.work,
        }
        drop(state);
        if force || checkpoint {
            self.flush(force);
        }
    }
}

impl Writer {
    fn flush(&self, force: bool) {
        let now = self.elapsed();
        let mut state = self.state.borrow_mut();
        if !force && now - state.last_write < 0.5 {
            return;
        }
        let mut levels = state.levels.clone();
        if let Some(level) = levels.last_mut() {
            if level.status == "running" {
                level.total_seconds = now - level.started_seconds;
            }
            for &i in &state.stack {
                let span = &mut level.spans[i];
                span.elapsed_seconds = now - span.started_seconds;
                if span.category == "stage" {
                    *level.stage_seconds.entry(span.name.clone()).or_default() += span.elapsed_seconds;
                }
            }
        }
        state.checkpoints += 1;
        let current = levels.last();
        let mut data = json!({
            "schema_version": 1,
            "status": state.status,
            "elapsed_seconds": now,
            "cli_elapsed_seconds": now,
            "solver_total_seconds": levels.iter().map(|l| l.total_seconds).sum::<f64>(),
            "last_updated_seconds": now,
            "checkpoint_count": state.checkpoints,
            "snapshot_bytes_before_this_checkpoint": state.written_bytes,
            "current_level": current.filter(|l| l.status == "running").map(|l| l.level_id),
            "current_stage": current.and_then(|l| l.current_stage.as_ref()),
            "active_span": state.stack.last().and_then(|i| current.map(|l| l.spans[*i].name.clone())),
            "counts_complete": state.status == "complete",
            "peak_memory_bytes": Value::Null,
            "notes": [
                "stage_seconds are mutually exclusive top-level solver stages; nested span times are inclusive",
                "missing metrics are unobserved, not zero; partial counts include work through the last checkpoint",
                "checkpoints are throttled to 0.5 seconds at work boundaries; one long rational operation may delay a checkpoint",
                "rational bit maxima cover stored tableau coefficients, values and bounds at observations, not temporary arithmetic intermediates",
                "profiling leaves deadline read sites and all options unchanged; wall-clock observation overhead may move a reached deadline",
                "exact output equivalence is guaranteed only when no resolution deadline is reached, or with a controlled deadline clock",
                "peak memory is not available from this portable writer; collect it externally"
            ],
            "levels": levels,
        });
        if self.work {
            data["work_diagnostics_schema_version"] = json!(1);
            data["diagnostic_mode"] = json!("work");
            data["tableau_details"] = json!({
                "status": "not_observed",
                "omitted_metrics": ["tableau_scans", "tableau_rows", "variables", "constraints",
                    "max_numerator_bits", "max_denominator_bits"],
                "reason": "work mode disables the full stored-rational tableau scan"
            });
            data["work_counter_notes"] = json!([
                "work counts are saturated u64 execution totals, not timings; overflow is flagged",
                "visits include coefficient misses; hit/miss counts do not imply time shares",
                "tableau nnz is maintained from executed row changes without a new row scan",
                "work_live_nnz is a current live-account gauge; maxima and work totals are separate",
                "clone starts new execution totals; rollback does not undo work; drop retains pending work",
                "reserve requests and capacity changes are not exact allocator-call counts",
                "work is batched at coarse boundaries; partial data is a lower-bound prefix, not all work at exit",
                "nested work spans include axis/lex children; never add parent and child seconds",
                "work diagnostics have overhead and are not profile-off performance measurements"
            ]);
        }
        let tmp = self.path.with_extension(format!("json.partial-{}-{}", std::process::id(), state.checkpoints));
        let result = serde_json::to_vec(&data).map_err(|e| e.to_string()).and_then(|bytes| {
            state.written_bytes = state.written_bytes.saturating_add(bytes.len() as u64);
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
                .and_then(|mut file| {
                    use std::io::Write;
                    file.write_all(&bytes)
                })
                .and_then(|()| std::fs::rename(&tmp, &self.path))
                .map_err(|e| e.to_string())
        });
        match result {
            Ok(()) => state.last_write = now,
            Err(e) => {
                state.error = Some(format!("cannot write profile {}: {e}", self.path.display()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_batches_add_execution_but_replace_gauges_and_flag_overflow() {
        let mut metrics = BTreeMap::new();
        work_metrics(&mut metrics, &[("work_rows", 5)], &[("work_peak", 7)], &[("work_live", 9)]);
        work_metrics(&mut metrics, &[("work_rows", 2)], &[("work_peak", 4)], &[("work_live", 3)]);
        assert_eq!(metrics["work_rows"], 7);
        assert_eq!(metrics["work_peak"], 7);
        assert_eq!(metrics["work_live"], 3);
        work_metrics(&mut metrics, &[("work_rows", u64::MAX)], &[], &[]);
        assert_eq!(metrics["work_rows"], u64::MAX);
        assert_eq!(metrics["work_counter_overflow"], 1);
    }

    #[test]
    fn checkpoints_keep_active_work_and_replace_valid_json() {
        let dir = std::env::temp_dir().join(format!("rnl-profile-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("partial.json");
        let _ = std::fs::remove_file(&path);
        let clock = ProfileClock::new(&path).unwrap();
        let observer = clock.profiler().unwrap();
        observer.record(ProfileEvent::Level {
            level: 3,
            title: "partial".into(),
            objects: 2,
            facts: 1,
            routes: 1,
            options: "unchanged".into(),
            hash: "sig2-test".into(),
        });
        observer.record(ProfileEvent::Begin { category: "stage", name: "route".into() });
        observer.record(ProfileEvent::Add { name: "astar_expansions", value: 256 });
        clock.0.flush(true);
        let partial: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(partial["status"], "running");
        assert_eq!(partial["counts_complete"], false);
        assert_eq!(partial["current_stage"], "route");
        assert_eq!(partial["levels"][0]["metrics"]["astar_expansions"], 256);
        assert_eq!(partial["levels"][0]["spans"][0]["complete"], false);
        assert!(partial["levels"][0]["stage_seconds"]["route"].as_f64().unwrap() >= 0.0);
        observer.record(ProfileEvent::End);
        observer.record(ProfileEvent::LevelEnd);
        clock.finish(true).unwrap();
        let final_json: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(final_json["status"], "complete");
        assert_eq!(final_json["levels"][0]["spans"][0]["complete"], true);
        assert_eq!(final_json["current_stage"], Value::Null);
        assert!(final_json["checkpoint_count"].as_u64().unwrap() > partial["checkpoint_count"].as_u64().unwrap());
        assert!(std::fs::read_dir(&dir).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".partial-")));
        std::fs::remove_file(path).unwrap();
    }
}
