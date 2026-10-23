//! Opt-in prototype counters, separate from --profile and all solver output.

use std::cell::{Cell, RefCell};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::rc::Rc;
use std::time::Instant;

use rnl_solver::profile::{StateReuseObserver, StateReuseReport};
use rnl_solver::Clock;

pub struct StateReuseStats {
    file: RefCell<File>,
    started: Instant,
    ordinal: Cell<u64>,
    error: RefCell<Option<String>>,
}

impl StateReuseStats {
    pub fn from_env() -> Result<Option<Rc<Self>>, String> {
        let Some(path) = std::env::var_os("RNL_STATE_REUSE_STATS") else { return Ok(None) };
        // Do not truncate a pre-existing file, in particular a source netlist or old receipt.
        let file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| {
            format!("cannot create state reuse statistics {}: {e}", std::path::Path::new(&path).display())
        })?;
        Ok(Some(Rc::new(Self {
            file: RefCell::new(file),
            started: Instant::now(),
            ordinal: Cell::new(0),
            error: RefCell::new(None),
        })))
    }

    pub fn finish(&self) -> Result<(), String> {
        match self.error.borrow().as_ref() {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
}

impl Clock for StateReuseStats {
    fn elapsed(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }
}

impl StateReuseObserver for StateReuseStats {
    fn record(&self, report: StateReuseReport) {
        let ordinal = self.ordinal.get();
        self.ordinal.set(ordinal + 1);
        let value = serde_json::json!({
            "schema": 1, "ordinal": ordinal,
            "hit": report.hit, "miss_reason": report.miss_reason,
            "reused_relations": report.reused_relations, "total_relations": report.total_relations,
            "save_clones": report.save_clones, "save_ns": report.save_ns,
            "restore_ns": report.restore_ns, "model_clone_ns": report.model_clone_ns,
            "logical_prefix_nodes": report.logical_prefix_nodes,
            "logical_prefix_pivots": report.logical_prefix_pivots,
        });
        let result = (|| {
            let mut file = self.file.borrow_mut();
            serde_json::to_writer(&mut *file, &value)?;
            file.write_all(b"\n").map_err(serde_json::Error::io)
        })();
        if let Err(error) = result {
            *self.error.borrow_mut() = Some(format!("cannot write state reuse statistics: {error}"));
        }
    }
}
