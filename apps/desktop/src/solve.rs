//! The Solve menu (decision 0010): the netlist solved as `rnl solve` solves it, without its records
//! of the solver, in memory on a thread of its own, its progress sent to the page, and cancelled
//! at the user's word; the result drawn as a preview; and a copy of the files solved saved where
//! the user says. The core does the work (`rnl_cli::preview`); nothing here writes a file the
//! viewer opened.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rnl_cli::preview::{one_file, preview, save_copy, Copy, Report, StoppableClock};
use rnl_cli::solve::{Progress, SolveOptions};
use rnl_cli::FsProvider;
use rnl_view::{Document, MemoryProvider};
use serde::Serialize;
use tauri::{Emitter, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

/// The solve under way, and the last one shown, whose files a copy saves.
#[derive(Clone, Default)]
pub struct Solves {
    /// The stop of the solve under way, if any.
    running: Arc<Mutex<Option<Arc<AtomicBool>>>>,
    /// What a copy of the last solve shown saves.
    last: Arc<Mutex<Option<Copy>>>,
}

/// What a solve gives the page: the netlist drawn as solved, and the report.
#[derive(Serialize)]
pub struct Solved {
    document: Document,
    report: Report,
}

/// Where a solve is, as the page shows it.
#[derive(Clone, Serialize)]
struct Step {
    level: usize,
    levels: usize,
    title: String,
    again: bool,
}

/// The time limit of the resolution of each level, in seconds, as the page gives it: null for the
/// default of `rnl solve`, "none" for no limit, or a number.
fn time_limit(given: Option<serde_json::Value>) -> Result<Option<f64>, String> {
    let default = rnl_cli::solve::SolveOptions::default().solver.time_limit;
    match given {
        None | Some(serde_json::Value::Null) => Ok(default),
        Some(serde_json::Value::String(s)) if s == "none" => Ok(None),
        Some(serde_json::Value::Number(n)) => match n.as_f64() {
            Some(s) if s >= 0.0 && s.is_finite() => Ok(Some(s)),
            _ => Err(format!("{n} is no time limit: give a number of seconds, 0 or more, or none")),
        },
        Some(v) => Err(format!("{v} is no time limit: give a number of seconds, 0 or more, or none")),
    }
}

/// Solve the netlist the page shows, a file by its path or a text held in memory as the file
/// `name`, and give it drawn as solved, nothing written. The page is told of each level as it is
/// solved (`solve-progress`); a solve started while another runs stops that one first.
#[tauri::command]
pub async fn solve_preview(
    window: WebviewWindow,
    solves: State<'_, Solves>,
    path: Option<String>,
    name: Option<String>,
    text: Option<String>,
    time_limit: Option<serde_json::Value>,
) -> Result<Solved, String> {
    let mut options = SolveOptions::default();
    options.solver.time_limit = self::time_limit(time_limit)?;
    let stop = Arc::new(AtomicBool::new(false));
    if let Some(before) = solves.running.lock().map_err(|e| e.to_string())?.replace(stop.clone()) {
        before.store(true, Ordering::Relaxed);
    }
    let solves = solves.inner().clone();
    let running = stop.clone();
    let shown = tauri::async_runtime::spawn_blocking(move || {
        let watch = |p: &Progress| {
            let step = Step { level: p.level, levels: p.levels, title: p.title.clone(), again: p.again };
            let _ = window.emit("solve-progress", step);
            !running.load(Ordering::Relaxed)
        };
        let clock = StoppableClock::new(running.clone());
        match (path, name, text) {
            (Some(path), _, _) => {
                let path = std::path::absolute(&path).map_err(|e| format!("{path}: {e}"))?;
                preview(&path.to_string_lossy(), &FsProvider::new(&[]), &options, &clock, &watch)
            }
            (None, Some(name), Some(text)) => {
                let entry = format!("/{name}");
                preview(&entry, &MemoryProvider::new().with(&entry, &text), &options, &clock, &watch)
            }
            _ => Err("nothing to solve: open a netlist first".to_string()),
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    // this solve is no longer the one under way, unless another began meanwhile
    if let Ok(mut running) = solves.running.lock() {
        if running.as_ref().is_some_and(|r| Arc::ptr_eq(r, &stop)) {
            *running = None;
        }
    }
    let shown = shown?;
    *solves.last.lock().map_err(|e| e.to_string())? = Some(shown.copy);
    Ok(Solved { document: shown.document, report: shown.report })
}

/// Stop the solve under way, if any: it gives back the word that it was cancelled.
#[tauri::command]
pub fn cancel_solve(solves: State<'_, Solves>) {
    if let Ok(running) = solves.running.lock() {
        if let Some(stop) = running.as_ref() {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

/// Save a copy of the last solve shown where the user says: a file for a netlist that reads no
/// other file, else a folder, which gets every file written and read, in their places. A place
/// that would replace a file that was read is refused. None when the dialog is cancelled; else
/// what was saved, for the page to say.
#[tauri::command]
pub async fn save_solved_copy(window: WebviewWindow, solves: State<'_, Solves>) -> Result<Option<String>, String> {
    let run = solves.last.lock().map_err(|e| e.to_string())?.clone();
    let Some(run) = run else { return Err("solve the netlist first: there is no solve to save".to_string()) };
    let entry = PathBuf::from(&run.entry);
    let dialog = window.dialog().file().set_parent(&window);
    let dialog = match entry.parent().filter(|p| p.is_dir()) {
        Some(dir) => dialog.set_directory(dir),
        None => dialog,
    };
    let one = one_file(&run);
    let chosen = tauri::async_runtime::spawn_blocking(move || {
        if one {
            dialog.set_title("Save Solved Copy As").set_file_name(copy_name(&entry)).blocking_save_file()
        } else {
            dialog.set_title("Save the Solved Files Into a Folder").blocking_pick_folder()
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(chosen) = chosen else { return Ok(None) };
    let to = chosen.into_path().map_err(|e| e.to_string())?;
    let saved = tauri::async_runtime::spawn_blocking(move || save_copy(&run, &to).map(|w| (w, to)))
        .await
        .map_err(|e| e.to_string())??;
    Ok(Some(said(&saved.0, &saved.1)))
}

/// The name a copy of one file is offered under: the file's, with `-solved` before its extension.
fn copy_name(entry: &Path) -> String {
    let stem = entry.file_stem().map_or("netlist".into(), |s| s.to_string_lossy().into_owned());
    match entry.extension() {
        Some(ext) => format!("{stem}-solved.{}", ext.to_string_lossy()),
        None => format!("{stem}-solved"),
    }
}

/// What was saved, in a sentence.
fn said(w: &rnl_cli::solve::Writing, to: &Path) -> String {
    let n = |k: usize, one: &str, many: &str| format!("{k} {}", if k == 1 { one } else { many });
    if w.copied.is_empty() && w.written.len() == 1 && w.written[0] == to {
        format!("Saved the solved copy as {}.", to.display())
    } else {
        let mut s = format!("Saved {} into {}", n(w.written.len(), "solved file", "solved files"), to.display());
        if !w.copied.is_empty() {
            s.push_str(&format!(", with {} read beside them", n(w.copied.len(), "other file", "other files")));
        }
        s.push('.');
        s
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{copy_name, time_limit};

    /// The page gives the time limit as null for the default of `rnl solve`, "none" or seconds.
    #[test]
    fn the_time_limit_the_page_gives() {
        let default = rnl_cli::solve::SolveOptions::default().solver.time_limit;
        assert_eq!(time_limit(None), Ok(default));
        assert_eq!(time_limit(Some(serde_json::Value::Null)), Ok(default));
        assert_eq!(time_limit(Some(serde_json::json!("none"))), Ok(None));
        assert_eq!(time_limit(Some(serde_json::json!(2.5))), Ok(Some(2.5)));
        assert!(time_limit(Some(serde_json::json!(-1))).is_err());
        assert!(time_limit(Some(serde_json::json!("soon"))).is_err());
    }

    #[test]
    fn a_copy_is_offered_beside_its_file_with_solved_in_its_name() {
        assert_eq!(copy_name(Path::new("D:/w/amp.sp")), "amp-solved.sp");
        assert_eq!(copy_name(Path::new("/w/top")), "top-solved");
    }
}
