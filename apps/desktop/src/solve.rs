//! The Solve menu (decision 0010): the netlist solved as `rnl solve` solves it, without its records
//! of the solver, with the options the user set in Solve > Settings, in memory on a thread of its
//! own, its progress sent to the page, and cancelled at the user's word; the result drawn as a
//! preview; and a copy of the files solved saved where the user says. The core does the work
//! (`rnl_cli::preview`); nothing here writes a file the viewer opened.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::json;

use rnl_cli::preview::{one_file, preview, save_copy, Copy, Report, StoppableClock};
use rnl_cli::solve::{Progress, SolveOptions};
use rnl_cli::FsProvider;
use rnl_solver::{exact_text, SolverOptions};
use rnl_view::Document;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::files::{self, Edits};

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

/// The solver's settings as the page gives them (Solve > Settings): `options` are the settings that
/// differ from the defaults, written as `rnl solve --solver-options` takes them, such as
/// `beam=8;time_limit=none`, and `hints_for_low` asks for every conflict group of the facts of
/// priority low, which no such text sets. None given, or an empty text, are the defaults of
/// `rnl solve`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    options: String,
    #[serde(default)]
    hints_for_low: bool,
    #[serde(default)]
    solver_config: Option<String>,
}

impl Settings {
    fn loaded(&self, resource_directory: Option<&Path>) -> Result<rnl_cli::solver_config::LoadedConfig, String> {
        let mut loaded = rnl_cli::solver_config::resolve(
            self.solver_config.as_deref().filter(|s| !s.is_empty()).map(Path::new),
            resource_directory,
            &self.options,
        )?;
        loaded.options.hints_for_low = self.hints_for_low;
        Ok(loaded)
    }
}

fn resource_directory(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().resource_dir().map_err(|e| format!("cannot locate solver resources: {e}"))
}

fn defaults_json(d: &SolverOptions, source: &str) -> serde_json::Value {
    json!({
        "source": source,
        "tolerance": exact_text(&d.tolerance),
        "grid": d.grid.as_ref().map(exact_text),
        "beam": d.beam.to_string(),
        "route_beam": d.route_beam.to_string(),
        "separate_symbols": d.separate_symbols,
        "separate_gap": exact_text(&d.separate_gap),
        "route_clearance": exact_text(&d.route_clearance),
        "texts_at_anchor": d.texts_at_anchor,
        "budget": { "candidates": d.budget.candidates.to_string(), "astar_expansions": d.budget.astar_expansions.to_string(), "soft_expansions": d.budget.soft_expansions.to_string(), "symmetry_steps": d.budget.symmetry_steps.to_string() },
        "time_limit": d.time_limit.as_ref().map(exact_text),
        "hints_for_low": d.hints_for_low,
    })
}

#[tauri::command]
pub fn solver_defaults(app: AppHandle, solver_config: Option<String>) -> Result<serde_json::Value, String> {
    let directory = resource_directory(&app)?;
    let settings = Settings { solver_config, ..Settings::default() };
    match settings.loaded(Some(&directory)) {
        Ok(loaded) => Ok(defaults_json(&loaded.options, &loaded.source())),
        Err(error) => {
            // Keep the dialog usable so an invalid selected/default file can be replaced. This
            // does not select a fallback for solving: checking or solving still returns the error.
            let mut value = defaults_json(&SolverOptions::builtin(), "configuration error");
            value["config_error"] = json!(error);
            Ok(value)
        }
    }
}

#[tauri::command]
pub fn check_solver_options(app: AppHandle, options: String, solver_config: Option<String>) -> Result<(), String> {
    Settings { options, solver_config, ..Settings::default() }.loaded(Some(&resource_directory(&app)?)).map(|_| ())
}

#[tauri::command]
pub async fn pick_solver_config(window: WebviewWindow) -> Result<Option<String>, String> {
    let chosen = tauri::async_runtime::spawn_blocking(move || {
        window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Choose Solver Configuration")
            .add_filter("Solver TOML", &["toml"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    chosen.map(|p| p.into_path().map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string())).transpose()
}

/// Solve the netlist the page shows, a file by its path or a text held in memory as the file
/// `name`, and give it drawn as solved, nothing written. The page is told of each level as it is
/// solved (`solve-progress`); a solve started while another runs stops that one first.
#[tauri::command]
// Tauri injects the window and managed state alongside the separate IPC arguments.
#[allow(clippy::too_many_arguments)]
pub async fn solve_preview(
    window: WebviewWindow,
    solves: State<'_, Solves>,
    preferences: State<'_, crate::settings::Settings>,
    pool: Option<Vec<String>>,
    path: Option<String>,
    name: Option<String>,
    text: Option<String>,
    settings: Option<Settings>,
    edits: Option<Edits>,
) -> Result<Solved, String> {
    let paths = preferences.paths()?;
    let pool = pool.unwrap_or_default();
    let loaded = settings.unwrap_or_default().loaded(Some(&resource_directory(window.app_handle())?))?;
    let config_source = loaded.source();
    let options = SolveOptions { solver: loaded.options, ..SolveOptions::default() };
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
        let disk = FsProvider::new(&paths).with_search_paths(&paths);
        let disk = rnl_runtime::source::FilePool { files: &disk, paths: &pool };
        match (path, name, text) {
            (Some(path), _, _) => {
                let path = std::path::absolute(&path).map_err(|e| format!("{path}: {e}"))?;
                let entry = path.to_string_lossy().into_owned();
                // the text applied is what is solved, over the files on disk
                let held = files::held(&entry, &edits.unwrap_or_default());
                preview(&entry, &files::layered(&held, &disk), &options, &clock, &watch)
            }
            (None, Some(name), Some(text)) => {
                let entry = format!("/{name}");
                preview(
                    &entry,
                    &files::layered(&files::held_text(&entry, &text, &edits.unwrap_or_default()), &disk),
                    &options,
                    &clock,
                    &watch,
                )
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
    let mut shown = shown?;
    shown.report.text.insert(0, format!("solver config: {config_source}"));
    shown.report.config_source = Some(config_source);
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
    use super::{copy_name, defaults_json, Settings};
    use rnl_solver::{exact_text, SolverOptions};
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn settings_keep_exact_text_and_selected_config_path() {
        let settings: Settings = serde_json::from_value(
            json!({"options":"tolerance=0.10000000000000001", "solverConfig":"custom.toml", "hintsForLow":true}),
        )
        .unwrap();
        assert_eq!(settings.solver_config.as_deref(), Some("custom.toml"));
        assert!(settings.hints_for_low);
        let options = SolverOptions::builtin().with_overrides(&settings.options).unwrap();
        assert_eq!(exact_text(&options.tolerance), "10000000000000001/100000000000000000");
        assert!(SolverOptions::builtin().with_overrides("objectives=none").is_err());
    }

    #[test]
    fn defaults_transport_rationals_as_strings() {
        let options = SolverOptions::builtin();
        let value = defaults_json(&options, "builtin");
        assert_eq!(value["tolerance"], json!(exact_text(&options.tolerance)));
        assert!(value.get("objectives").is_none());
        assert_eq!(value["source"], "builtin");
    }

    #[test]
    fn a_copy_is_offered_beside_its_file_with_solved_in_its_name() {
        assert_eq!(copy_name(Path::new("D:/w/amp.sp")), "amp-solved.sp");
        assert_eq!(copy_name(Path::new("/w/top")), "top-solved");
    }
}
