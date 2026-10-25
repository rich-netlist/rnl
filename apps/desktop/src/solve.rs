//! The Solve menu (decision 0010): the netlist solved as `rnl solve` solves it, without its records
//! of the solver, with the options the user set in Solve > Settings, in memory on a thread of its
//! own, its progress sent to the page, and cancelled at the user's word; the result drawn as a
//! preview; and a copy of the files solved saved where the user says. The core does the work
//! (`rnl_cli::preview`); nothing here writes a file the viewer opened.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::json;

use rnl_cli::preview::{one_file, preview, save_copy_with_encoding, Copy, Report, StoppableClock};
use rnl_cli::solve::{Progress, SolveOptions};
use rnl_cli::FsProvider;
use rnl_solver::{SolverOptions, OBJECTIVES};
use rnl_view::Document;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::files::{self, Edits};

/// The solve under way, and the last one shown, whose files a copy saves.
#[derive(Clone, Default)]
pub struct Solves {
    /// The stop of the solve under way, if any.
    running: Arc<Mutex<Option<Arc<AtomicBool>>>>,
    /// What a copy of the last solve shown saves.
    last: Arc<Mutex<Option<SavedCopy>>>,
}

/// A copy and its source provenance: a memory source has UTF-8 text and no original disk bytes.
#[derive(Clone)]
struct SavedCopy {
    copy: Copy,
    in_memory: bool,
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
}

impl Settings {
    /// The defaults with these settings over them, as `rnl solve` reads `--solver-options`: an
    /// unknown option, or a value that does not do, is an error that says why.
    fn solver(&self) -> Result<SolverOptions, String> {
        let options = SolverOptions::default().with_overrides(&self.options)?;
        Ok(SolverOptions { hints_for_low: self.hints_for_low, ..options })
    }
}

/// The solver's defaults and the objectives it knows, for the settings dialog to show: the options
/// as the page edits them, `grid` and `time_limit` null for none, and the layers of the objectives,
/// the first first, each with its objectives and their weights and its tolerance.
#[tauri::command]
pub fn solver_defaults() -> serde_json::Value {
    let d = SolverOptions::default();
    let layers: Vec<_> = d
        .objectives
        .layers
        .iter()
        .map(|layer| {
            let items: Vec<_> =
                layer.items.iter().map(|(name, weight)| json!({ "name": name, "weight": weight })).collect();
            json!({ "items": items, "tolerance": layer.tolerance })
        })
        .collect();
    json!({
        "tolerance": d.tolerance,
        "grid": d.grid,
        "beam": d.beam,
        "budget": { "candidates": d.budget.candidates, "astar_expansions": d.budget.astar_expansions },
        "time_limit": d.time_limit,
        "hints_for_low": d.hints_for_low,
        "objectives": layers,
        "names": OBJECTIVES,
    })
}

/// Whether the solver takes settings written as `--solver-options` takes them: nothing when it does,
/// else what is wrong, in its own words.
#[tauri::command]
pub fn check_solver_options(options: String) -> Result<(), String> {
    SolverOptions::default().with_overrides(&options).map(|_| ())
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
    library: State<'_, crate::library::Library>,
    auto_include: Option<bool>,
    pool: Option<Vec<String>>,
    path: Option<String>,
    name: Option<String>,
    text: Option<String>,
    settings: Option<Settings>,
    edits: Option<Edits>,
    formats: Option<crate::formats::Formats>,
) -> Result<Solved, String> {
    let paths = preferences.paths()?;
    let library = library.snapshot()?;
    let pool = pool.unwrap_or_default();
    let in_memory = path.is_none();
    let options = SolveOptions {
        solver: settings.unwrap_or_default().solver()?,
        formats: formats.unwrap_or_default(),
        ..SolveOptions::default()
    };
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
                let layered = files::layered(&held, &disk);
                let index = library.with_edits(&held, &layered);
                let files = rnl_view::LibraryProvider {
                    files: &layered,
                    index: &index,
                    enabled: auto_include.unwrap_or(false),
                };
                preview(&entry, &files, &options, &clock, &watch)
            }
            (None, Some(name), Some(text)) => {
                let entry = format!("/{name}");
                let held = files::held_text(&entry, &text, &edits.unwrap_or_default());
                let layered = files::layered(&held, &disk);
                let index = library.with_edits(&held, &layered);
                let files = rnl_view::LibraryProvider {
                    files: &layered,
                    index: &index,
                    enabled: auto_include.unwrap_or(false),
                };
                preview(&entry, &files, &options, &clock, &watch)
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
    *solves.last.lock().map_err(|e| e.to_string())? = Some(SavedCopy { copy: shown.copy, in_memory });
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
pub async fn save_solved_copy(
    window: WebviewWindow,
    solves: State<'_, Solves>,
    encoding: Option<rnl_cli::codec::OutputEncoding>,
) -> Result<Option<String>, String> {
    let run = solves.last.lock().map_err(|e| e.to_string())?.clone();
    let Some(SavedCopy { copy: run, in_memory }) = run else {
        return Err("solve the netlist first: there is no solve to save".to_string());
    };
    // A clipboard source has no original bytes. Preserve means its native UTF-8 text; a named
    // disk source must remain distinguishable from one that has disappeared since the preview.
    let encoding = if in_memory { rnl_cli::codec::OutputEncoding::Utf8 } else { encoding.unwrap_or_default() };
    let entry = PathBuf::from(&run.entry);
    let dialog = window.dialog().file().set_parent(&window);
    let dialog = match entry.parent().filter(|p| p.is_dir()) {
        Some(dir) => dialog.set_directory(dir),
        None => dialog,
    };
    let one = one_file(&run);
    let title = if one {
        crate::menu::text(window.app_handle(), "Save Solved Copy As")
    } else {
        crate::menu::text(window.app_handle(), "Save the Solved Files Into a Folder")
    };
    let dialog = dialog.set_title(title);
    let chosen = tauri::async_runtime::spawn_blocking(move || {
        if one {
            dialog.set_file_name(copy_name(&entry)).blocking_save_file()
        } else {
            dialog.blocking_pick_folder()
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(chosen) = chosen else { return Ok(None) };
    let to = chosen.into_path().map_err(|e| e.to_string())?;
    let saved =
        tauri::async_runtime::spawn_blocking(move || save_copy_with_encoding(&run, &to, encoding).map(|w| (w, to)))
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

    use rnl_solver::{SolverOptions, DEFAULT_LAYERS, OBJECTIVES};
    use serde_json::json;

    use super::{check_solver_options, copy_name, solver_defaults, Settings};

    /// The settings the page gives: none, or an empty text, are the defaults of `rnl solve`; a text
    /// is read over them as `--solver-options` reads it; `hintsForLow` is not part of that text.
    #[test]
    fn the_settings_the_page_gives() {
        let given = |v: serde_json::Value| serde_json::from_value::<Settings>(v).unwrap().solver();
        assert_eq!(Settings::default().solver(), Ok(SolverOptions::default()));
        assert_eq!(given(json!({})), Ok(SolverOptions::default()));
        assert_eq!(given(json!({ "options": "", "hintsForLow": false })), Ok(SolverOptions::default()));
        let text = "beam=8;time_limit=none;grid=none;budget=candidates:7";
        let set = given(json!({ "options": text, "hintsForLow": true })).unwrap();
        let default = SolverOptions::default();
        assert_eq!((set.beam, set.time_limit, set.grid), (8, None, None));
        assert_eq!((set.budget.candidates, set.budget.astar_expansions), (7, default.budget.astar_expansions));
        assert!(set.hints_for_low);
        assert_eq!(set.objectives, default.objectives);
        // the objectives replace the whole list, as `--solver-options` has them
        let set = given(json!({ "options": "objectives=area+wire_length*2~0.5,turns" })).unwrap();
        assert_eq!(set.canonical().split(';').nth(2), Some("objectives=area+wire_length*2~0.5,turns"));
        // what does not read says why, in the solver's words, and so does the check
        for (text, why) in [
            ("beam=0", "the beam must be at least 1"),
            ("time_limit=-1", "the time limit must be a number of seconds not below 0"),
            ("tolerance=x", "the tolerance is not a number"),
            ("objectives=beauty", "no objective is called `beauty`"),
            ("budget=pivots:1", "the budget has no `pivots`"),
            ("speed=3", "the solver has no option `speed`"),
        ] {
            let e = given(json!({ "options": text })).unwrap_err();
            assert!(e.contains(why), "{text}: {e}");
            assert_eq!(check_solver_options(text.to_string()), Err(e));
        }
        assert_eq!(check_solver_options(String::new()), Ok(()));
        assert_eq!(check_solver_options("beam=2;time_limit=none".to_string()), Ok(()));
    }

    /// The dialog shows what the solver's defaults are, and which objectives there are, as the
    /// solver says; the page keeps nothing of them itself.
    #[test]
    fn the_defaults_the_dialog_shows_are_the_solvers() {
        let d = solver_defaults();
        let o = SolverOptions::default();
        assert_eq!(d["tolerance"], json!(o.tolerance));
        assert_eq!(d["grid"], json!(o.grid));
        assert_eq!(d["beam"], json!(o.beam));
        assert_eq!(d["time_limit"], json!(o.time_limit));
        assert_eq!(d["hints_for_low"], json!(false));
        let budget = json!({ "candidates": o.budget.candidates, "astar_expansions": o.budget.astar_expansions });
        assert_eq!(d["budget"], budget);
        assert_eq!(d["names"], json!(OBJECTIVES));
        let layers = d["objectives"].as_array().unwrap();
        let first: Vec<&str> = layers.iter().map(|l| l["items"][0]["name"].as_str().unwrap()).collect();
        assert_eq!(first, DEFAULT_LAYERS);
        assert!(layers.iter().all(|l| l["items"].as_array().unwrap().len() == 1 && l["tolerance"] == 0.0));
    }

    #[test]
    fn a_copy_is_offered_beside_its_file_with_solved_in_its_name() {
        assert_eq!(copy_name(Path::new("D:/w/amp.sp")), "amp-solved.sp");
        assert_eq!(copy_name(Path::new("/w/top")), "top-solved");
    }
}
