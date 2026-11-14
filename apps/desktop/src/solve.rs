//! The Solve menu (decision 0010): the netlist solved as `rnl solve` solves it, without its records
//! of the solver, with the options the user set in Solve > Settings, in memory on a thread of its
//! own, its progress sent to the page, and cancelled at the user's word; the result drawn as a
//! preview; and a copy of the files solved saved where the user says. The core does the work
//! (`rnl_cli::preview`); nothing here writes a file the viewer opened.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rnl_cli::preview::{one_file, preview_progress, respace, save_copy_with_encoding, Copy, Report, StoppableClock};
use rnl_cli::progress::Event;
use rnl_cli::solve::{check_settings, SolveOptions, ToolSettings};
use rnl_runtime::source::MemoryProvider;
use rnl_solver::SolverOptions;
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

/// The solver's settings as the page gives them (Solve > Settings): `config` are the settings that
/// differ from the defaults, written as RNL named parameters as `rnl solve --solver-config` takes
/// them, such as `layout="rows",beam=8`. `timeall` is the seconds that all levels of a time limit share
/// (`rnl solve --timeall`), none for each level's own. `level` is the title of the one level to solve, the circuit
/// shown; none solves every level (Solve > Solve All Circuits). `respace` draws each level where the record of the
/// solver in the file last put its objects, spaced again as `config` says, nothing searched (Solve > Respace). None
/// given, or an empty text, are the defaults of `rnl solve`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    config: String,
    #[serde(default)]
    timeall: Option<f64>,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    respace: bool,
}

impl Settings {
    /// The whole file's time: a positive number of seconds, or none.
    fn total_seconds(&self) -> Result<Option<f64>, String> {
        match self.timeall {
            Some(s) if !s.is_finite() || s <= 0.0 => Err("timeall must be a positive number of seconds".into()),
            other => Ok(other),
        }
    }

    /// The defaults with these settings over them, as `rnl solve` reads `--solver-config`: a key the
    /// solver does not have, or a value that does not do, is an error that says why.
    fn solver(&self) -> Result<SolverOptions, String> {
        SolverOptions::default().with_config(&self.config)
    }
}

/// The table of the solver's settings, which the settings dialog is drawn from: the page keeps no
/// key, bound or default of its own (docs/solver-settings-table.md).
#[tauri::command]
pub fn solver_table() -> serde_json::Value {
    rnl_cli::solver_table::settings_table_json()
}

/// What `rnl solve --check --json` says of settings written as RNL named parameters, with no file:
/// one level, with each setting in effect and the problems of each, by key. Err when the text does
/// not read at all.
#[tauri::command]
pub fn check_solver_settings(config: String) -> Result<serde_json::Value, String> {
    let checked =
        check_settings(None, &MemoryProvider::new(), &SolveOptions::default(), &[ToolSettings::Config(config)])?;
    serde_json::to_value(checked).map_err(|e| e.to_string())
}

/// Solve the netlist the page shows, a file by its path or a text held in memory as the file
/// `name`, and give it drawn as solved, nothing written. The page is told of every step of the solve
/// as it goes, as the events of docs/solver-progress.md (`solve-progress`); a solve started while
/// another runs stops that one first.
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
    let settings = settings.unwrap_or_default();
    let spaced = settings.respace;
    let options = SolveOptions {
        level: settings.level.clone().filter(|title| !title.is_empty()),
        solver: settings.solver()?,
        total_seconds: settings.total_seconds()?,
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
        let watch = |event: &Event| {
            let _ = window.emit("solve-progress", event);
            !running.load(Ordering::Relaxed)
        };
        let clock = StoppableClock::new(running.clone());
        let disk = crate::public_packages::provider(&paths);
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
                if spaced {
                    respace(&entry, &files, &options, &clock, &watch)
                } else {
                    preview_progress(&entry, &files, &options, &clock, &watch)
                }
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
                if spaced {
                    respace(&entry, &files, &options, &clock, &watch)
                } else {
                    preview_progress(&entry, &files, &options, &clock, &watch)
                }
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
            s.push_str(&format!(
                ", along with {} copied alongside them",
                n(w.copied.len(), "other file", "other files")
            ));
        }
        s.push('.');
        s
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use rnl_solver::SolverOptions;
    use serde_json::json;

    use super::{check_solver_settings, copy_name, solver_table, Settings};

    /// The settings the page gives: none, or an empty text, are the defaults of `rnl solve`; a text
    /// is read over them as `--solver-config` reads it.
    #[test]
    fn the_settings_the_page_gives() {
        let given = |v: serde_json::Value| serde_json::from_value::<Settings>(v).unwrap().solver();
        assert_eq!(Settings::default().solver(), Ok(SolverOptions::default()));
        assert_eq!(given(json!({})), Ok(SolverOptions::default()));
        assert_eq!(given(json!({ "config": "" })), Ok(SolverOptions::default()));
        let set = given(json!({ "config": "beam=8,grid=false,candidates=7,report_hints=true" })).unwrap();
        // the settings given are the tool's overrides, which the solve applies over the file's own
        let keys: Vec<_> = set.settings.overrides.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, ["beam", "candidates", "grid", "report_hints"]);
        let set = given(json!({ "config": "objectives=\"area+wire_length*2~0.5,turns\"" })).unwrap();
        assert_eq!(set.settings.overrides.len(), 1);
        assert_eq!(set.settings.overrides[0].key, "objectives");
        // what does not read says why, in the solver's words, and so does the check, by the key
        for (text, key, why) in [
            ("beam=0", "beam", "the beam must be at least 1"),
            ("tolerance=\"x\"", "tolerance", "must be a finite number"),
            ("objectives=\"beauty\"", "objectives", "beauty"),
        ] {
            let e = given(json!({ "config": text })).unwrap_err();
            assert!(e.contains(why), "{text}: {e}");
            let checked = check_solver_settings(text.to_string()).unwrap();
            assert_eq!(checked["ok"], json!(false), "{text}");
            let errors = checked["levels"][0]["errors"].as_array().unwrap();
            assert!(errors.iter().any(|p| p["key"] == json!(key)), "{text}: {errors:?}");
        }
        // a key the solver does not have is a warning of the check: nothing is refused, and the key says which
        let checked = check_solver_settings("speed=3".to_string()).unwrap();
        assert_eq!(checked["ok"], json!(true));
        assert_eq!(checked["levels"][0]["warnings"][0]["key"], json!("speed"));
        let checked = check_solver_settings(String::new()).unwrap();
        assert_eq!(checked["ok"], json!(true));
        let checked = check_solver_settings("beam=2,layout=\"rows\"".to_string()).unwrap();
        assert_eq!(checked["ok"], json!(true));
        assert_eq!(checked["levels"][0]["settings"]["layout"]["from"], json!("command-line"));
    }

    /// The dialog is drawn from the table the solver gives; the page keeps nothing of it itself.
    #[test]
    fn the_table_the_dialog_is_drawn_from_is_the_solvers() {
        let t = solver_table();
        assert_eq!(t["version"], json!(1));
        let keys: Vec<&str> = t["settings"].as_array().unwrap().iter().map(|s| s["key"].as_str().unwrap()).collect();
        for key in ["layout", "spacing_x", "beam", "objectives", "grid", "limit", "seconds", "report_hints"] {
            assert!(keys.contains(&key), "{key}");
        }
        let o = SolverOptions::default();
        let beam = t["settings"].as_array().unwrap().iter().find(|s| s["key"] == "beam").unwrap();
        assert_eq!(beam["default"], json!(o.beam));
        assert!(!t["objectives"].as_array().unwrap().is_empty());
        assert!(!t["groups"].as_array().unwrap().is_empty());
    }

    /// The whole file's time is a positive number of seconds, or none.
    #[test]
    fn the_whole_files_time_is_a_positive_number_or_none() {
        let time = |v: serde_json::Value| serde_json::from_value::<Settings>(v).unwrap().total_seconds();
        assert_eq!(time(json!({})), Ok(None));
        assert_eq!(time(json!({ "timeall": null })), Ok(None));
        assert_eq!(time(json!({ "timeall": 90 })), Ok(Some(90.0)));
        assert!(time(json!({ "timeall": 0 })).is_err());
        assert!(time(json!({ "timeall": -1 })).is_err());
    }

    /// A respace is asked for by the page, and a solve is the default.
    #[test]
    fn a_respace_is_asked_for_by_the_page() {
        let respace = |v: serde_json::Value| serde_json::from_value::<Settings>(v).unwrap().respace;
        assert!(!respace(json!({})));
        assert!(!respace(json!({ "respace": false })));
        assert!(respace(json!({ "config": "spacing_x=2", "respace": true })));
    }

    /// The circuit shown is solved alone when the page names it; none, or an empty title, solve every one.
    #[test]
    fn a_level_is_given_by_its_title() {
        let level = |v: serde_json::Value| serde_json::from_value::<Settings>(v).unwrap().level;
        assert_eq!(level(json!({})), None);
        assert_eq!(level(json!({ "config": "", "level": null })), None);
        assert_eq!(level(json!({ "level": "amp" })).as_deref(), Some("amp"));
    }

    #[test]
    fn a_copy_is_offered_beside_its_file_with_solved_in_its_name() {
        assert_eq!(copy_name(Path::new("D:/w/amp.sp")), "amp-solved.sp");
        assert_eq!(copy_name(Path::new("/w/top")), "top-solved");
    }
}
