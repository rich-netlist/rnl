//! Durable, throttled JSON snapshots of solver observations.
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use rnl_runtime::diag::{codes, Code};
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
    /// of a stage, the time of the stages ended within it, which are not its own
    #[serde(skip)]
    within: f64,
}

/// The stage a span is in: itself or the nearest stage around it.
fn stage_of(spans: &[Span], mut at: Option<usize>) -> Option<usize> {
    while let Some(i) = at {
        if spans[i].category == "stage" {
            return Some(i);
        }
        at = spans[i].parent;
    }
    None
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
}

/// A wall clock with profiling. Deadline readings and profiling readings use the same source,
/// through independent `Clock` calls; profiling never calls the deadline clock on its behalf.
pub struct ProfileClock(Rc<Writer>);

impl ProfileClock {
    /// The clock writing its profile to `path`, a new file; or why it cannot, with its kind.
    pub fn new(path: &Path) -> Result<Self, (Code, String)> {
        // Reserve only a new path: a profile must never replace an input/include/package file.
        std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e| {
            (codes::CLI_PROFILE_CREATE_FAILED, format!("cannot create new profile {}: {e}", path.display()))
        })?;
        let writer = Rc::new(Writer {
            clock: WallClock::new(),
            path: path.to_path_buf(),
            state: RefCell::new(State { status: "running".into(), ..State::default() }),
        });
        writer.flush(true);
        if let Some(error) = writer.state.borrow().error.clone() {
            return Err((codes::CLI_PROFILE_WRITE_FAILED, error));
        }
        Ok(Self(writer))
    }

    /// Finishes even an unsuccessful run, retaining all completed and partial observations.
    pub fn finish(&self, success: bool) -> Result<(), (Code, String)> {
        self.0.state.borrow_mut().status = if success { "complete" } else { "error" }.into();
        self.0.flush(true);
        match &self.0.state.borrow().error {
            Some(e) => Err((codes::CLI_PROFILE_WRITE_FAILED, e.clone())),
            None => Ok(()),
        }
    }
}

impl Clock for ProfileClock {
    fn check_every(&self) -> u64 {
        self.0.clock.check_every()
    }
    fn elapsed(&self) -> f64 {
        self.0.clock.elapsed()
    }
    fn profiler(&self) -> Option<ProfileHandle> {
        Some(self.0.clone())
    }
}

impl Clock for Writer {
    fn check_every(&self) -> u64 {
        self.clock.check_every()
    }
    fn elapsed(&self) -> f64 {
        self.clock.elapsed()
    }
}

fn metric(metrics: &mut BTreeMap<String, Value>, name: &str, value: u64, mode: u8) {
    let old = metrics.get(name).and_then(Value::as_u64).unwrap_or(0);
    let value = match mode {
        0 => old.saturating_add(value),
        1 => old.max(value),
        _ => value,
    };
    metrics.insert(name.into(), json!(value));
}

impl ProfileObserver for Writer {
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
                    stage_seconds: [
                        "resolve",
                        "structure",
                        "search",
                        "coords",
                        "route",
                        "score",
                        "tree",
                        "place",
                        "lines",
                    ]
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
                let mut outer = true;
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
                        // a stage within another, such as a step of the lines layout that a level
                        // takes many times, waits for the throttled checkpoint
                        outer = stage_of(&level.spans, parent).is_none();
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
                        within: 0.0,
                    });
                    state.stack.push(id);
                }
                force = (category == "stage" && outer) || category == "resolve_phase";
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
                            // its own time, without that of the stages within it; the stage around
                            // it, if any, is current again and leaves this one's time out of its own
                            let (elapsed, own) = (span.elapsed_seconds, span.elapsed_seconds - span.within);
                            *level.stage_seconds.entry(span.name.clone()).or_default() += own;
                            let parent = span.parent;
                            level.current_stage = match stage_of(&level.spans, parent) {
                                Some(o) => {
                                    level.spans[o].within += elapsed;
                                    Some(level.spans[o].name.clone())
                                }
                                None => None,
                            };
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
            ProfileEvent::Checkpoint => checkpoint = true,
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
            }
            // each open stage's own time so far: without the stages ended within it and the open
            // one within it
            let open: Vec<usize> =
                state.stack.iter().copied().filter(|&i| level.spans[i].category == "stage").collect();
            for (j, &i) in open.iter().enumerate() {
                let inner = open.get(j + 1).map_or(0.0, |&k| level.spans[k].elapsed_seconds);
                let own = level.spans[i].elapsed_seconds - level.spans[i].within - inner;
                *level.stage_seconds.entry(level.spans[i].name.clone()).or_default() += own;
            }
        }
        state.checkpoints += 1;
        let current = levels.last();
        let data = json!({
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
                "stage_seconds are mutually exclusive solver stages: a stage within another counts as its own, not as the other's; other nested span times are inclusive",
                "missing metrics are unobserved, not zero; partial counts include work through the last checkpoint",
                "checkpoints are throttled to 0.5 seconds at work boundaries; one long rational operation may delay a checkpoint",
                "rational bit maxima cover stored tableau coefficients, values and bounds at observations, not temporary arithmetic intermediates",
                "profiling leaves deadline read sites and all options unchanged; wall-clock observation overhead may move a reached deadline",
                "exact output equivalence is guaranteed only when no resolution deadline is reached, or with a controlled deadline clock",
                "peak memory is not available from this portable writer; collect it externally"
            ],
            "levels": levels,
        });
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

    /// A stage within another, such as a step of the lines layout, counts as its own: the stage
    /// around it leaves its time out, it is current while it runs and the other after it, and the
    /// stages add up to no more than the level.
    #[test]
    fn a_stage_within_another_counts_as_its_own() {
        let dir = std::env::temp_dir().join(format!("rnl-profile-nested-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("nested.json");
        let _ = std::fs::remove_file(&path);
        let clock = ProfileClock::new(&path).unwrap();
        let observer = clock.profiler().unwrap();
        observer.record(ProfileEvent::Level {
            level: 0,
            title: "nested".into(),
            objects: 2,
            facts: 1,
            routes: 1,
            options: "unchanged".into(),
            hash: "sig2-test".into(),
        });
        let wait = || {
            let start = clock.elapsed();
            while clock.elapsed() - start < 0.02 {}
        };
        observer.record(ProfileEvent::Begin { category: "stage", name: "lines".into() });
        wait();
        observer.record(ProfileEvent::Begin { category: "lines", name: "settle".into() });
        observer.record(ProfileEvent::Begin { category: "stage", name: "place".into() });
        wait();
        observer.record(ProfileEvent::Begin { category: "stage", name: "route".into() });
        wait();
        clock.0.flush(true);
        let partial: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(partial["current_stage"], "route");
        let stages = &partial["levels"][0]["stage_seconds"];
        let total = partial["levels"][0]["total_seconds"].as_f64().unwrap();
        let sum: f64 = ["lines", "place", "route"].iter().map(|k| stages[k].as_f64().unwrap()).sum();
        assert!(sum <= total + 1e-9, "{stages} of {total}");
        for k in ["lines", "place", "route"] {
            assert!(stages[k].as_f64().unwrap() >= 0.015, "{k}: {stages}");
        }
        observer.record(ProfileEvent::End);
        clock.0.flush(true);
        let after: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(after["current_stage"], "place");
        observer.record(ProfileEvent::End);
        observer.record(ProfileEvent::End);
        clock.0.flush(true);
        let after: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(after["current_stage"], "lines");
        observer.record(ProfileEvent::End);
        observer.record(ProfileEvent::LevelEnd);
        clock.finish(true).unwrap();
        let done: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let level = &done["levels"][0];
        let stages = &level["stage_seconds"];
        let sum: f64 = stages.as_object().unwrap().values().map(|v| v.as_f64().unwrap()).sum();
        assert!(sum <= level["total_seconds"].as_f64().unwrap() + 1e-9, "{stages}");
        let span = |name: &str| {
            level["spans"].as_array().unwrap().iter().find(|s| s["name"] == name).unwrap()["elapsed_seconds"]
                .as_f64()
                .unwrap()
        };
        // the stage around the others: its own time, less than the span's, which takes them in
        assert!(stages["lines"].as_f64().unwrap() < span("lines") - 0.03, "{stages}");
        assert!((stages["place"].as_f64().unwrap() - (span("place") - span("route"))).abs() < 1e-9);
        assert_eq!(done["current_stage"], Value::Null);
        std::fs::remove_file(path).unwrap();
    }
}
