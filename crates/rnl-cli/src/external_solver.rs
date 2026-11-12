//! Execute only an explicitly selected program. File-backed standard streams avoid pipe
//! deadlocks and let the host enforce a timeout even when a descendant inherits a stream.

use crate::solve::BackendResult;
use rnl_runtime::problem::LevelProblem;
use rnl_solver::{Solved, SolverOptions};
use rnl_solver_io::{InputDocument, LevelInput};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const MAX_RESULT: u64 = 64 * 1024 * 1024;

struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        // Only the process we launched is ours to stop. Already-reaped children are harmless.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Run one level through an explicitly selected executable, validating the response before return.
pub fn solve(
    program: &Path,
    args: &[String],
    seconds: f64,
    problem: &LevelProblem,
    options: &SolverOptions,
) -> Result<BackendResult, String> {
    run(program, args, seconds, problem, options).map_err(|e| format!("external solver: {e}"))
}

fn run(
    program: &Path,
    args: &[String],
    seconds: f64,
    problem: &LevelProblem,
    options: &SolverOptions,
) -> Result<BackendResult, String> {
    let io = |e: std::io::Error| e.to_string();
    let input = InputDocument { levels: vec![LevelInput { problem: problem.clone(), options: options.clone() }] };
    let value = rnl_solver_io::encode_input(&input).map_err(|e| e.to_string())?;
    let mut stdin = tempfile::tempfile().map_err(io)?;
    serde_json::to_writer(&mut stdin, &value).map_err(|e| e.to_string())?;
    stdin.flush().map_err(io)?;
    stdin.seek(SeekFrom::Start(0)).map_err(io)?;
    let mut stdout = tempfile::tempfile().map_err(io)?;
    let mut command = Command::new(program);
    command.args(args).stdin(Stdio::from(stdin)).stdout(Stdio::from(stdout.try_clone().map_err(io)?));
    // Diagnostics are deliberately outside the JSON result and its persisted hash.
    command.stderr(Stdio::inherit());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = Running(command.spawn().map_err(|e| format!("cannot launch {}: {e}", program.display()))?);
    let start = Instant::now();
    let timeout = Duration::try_from_secs_f64(seconds).map_err(|e| e.to_string())?;
    loop {
        if stdout.metadata().map_err(io)?.len() > MAX_RESULT {
            return Err("result exceeds 64 MiB".into());
        }
        if let Some(status) = child.0.try_wait().map_err(io)? {
            if !status.success() {
                return Err(format!("process exited with {status}"));
            }
            break;
        }
        if start.elapsed() >= timeout {
            return Err(format!("process exceeded {seconds} seconds"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    stdout.seek(SeekFrom::Start(0)).map_err(io)?;
    let mut bytes = Vec::new();
    stdout.take(MAX_RESULT + 1).read_to_end(&mut bytes).map_err(io)?;
    if bytes.len() as u64 > MAX_RESULT {
        return Err("result exceeds 64 MiB".into());
    }
    let json = rnl_solver_io::parse_json(&bytes).map_err(|e| format!("invalid result JSON: {e}"))?;
    let result = rnl_solver_io::decode_result(&json, std::slice::from_ref(problem)).map_err(|e| e.to_string())?;
    let author = format!("{} {}", result.solver.name, result.solver.version);
    let level = result.levels.into_iter().next().ok_or("missing level result")?;
    let report_available = level.report.is_some();
    let mut report = level.report.unwrap_or_default();
    report.canonical_options = Some(level.options);
    Ok(BackendResult { solved: Solved { solution: level.solution, report }, author, report_available })
}
