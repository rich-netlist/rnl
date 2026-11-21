//! What the page reads a netlist with, file by file (apps/viewer-ui/file-reading.js, the desktop shell's `reader`): the
//! outline of the whole document, the reading of one file alone in the context of its entry, the hashes and texts of
//! its files as the core reads them, and the symbols of a file's subcircuits. The page keeps the readings, in memory,
//! by the rules every shell keeps them by; the app keeps nothing of them but the drawings of what the reading shown
//! has with no position, which the page asks for as it scrolls (`unplaced_drawings`).

use rnl_runtime::numbering::{hex, sha256};
use rnl_runtime::source::Provider;
use rnl_view::{Bind, BuildOptions};
use tauri::{State, WebviewWindow};

use crate::core_thread::{Core, Drawn};
use crate::files::{self, Edits};
use crate::{formats, library, load_progress, settings};

/// The source a reading is of, as the page names it: the netlist at `path`, or the text held as the file `name`; the
/// edits over its files; the file pool, Auto Include and the formats set for files.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    path: Option<String>,
    name: Option<String>,
    text: Option<String>,
    edits: Option<Edits>,
    pool: Option<Vec<String>>,
    auto_include: Option<bool>,
    formats: Option<formats::Formats>,
}

/// The options of a reading: those the page's readings are made with, flylines and the layout hints asked for, not
/// left to the core's default (View > Show Flylines hides the flylines, and the problems list has the hints).
fn options(formats: formats::Formats, bind: Option<Bind>, constraints: bool) -> BuildOptions {
    BuildOptions { formats, bind, constraints, flylines: true, hints: true, ..BuildOptions::default() }
}

/// Run `job` on the core thread, with its drawings, the entry as the core names it, and the files of `source` as a
/// reading reads them: its edits over the disk, the file pool, the public packages, PATH and the Library, Auto Include
/// when it is on. With `progress`, the window is told when the first file is read (`load-stage`).
async fn with_files<T: Send + 'static>(
    core: &Core,
    settings: &settings::Settings,
    library: &library::Library,
    source: Source,
    progress: Option<(WebviewWindow, String)>,
    job: impl FnOnce(&mut Drawn, &str, &dyn Provider, &formats::Formats) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let core = core.clone();
    let paths = settings.paths()?;
    let library = library.snapshot()?;
    tauri::async_runtime::spawn_blocking(move || {
        core.run(move |drawn| {
            let Source { path, name, text, edits, pool, auto_include, formats } = source;
            let (entry, held) = files::entry_held(path, name, text, &edits.unwrap_or_default())?;
            let pool = pool.unwrap_or_default();
            let disk = crate::public_packages::provider(&paths);
            let disk = rnl_runtime::source::FilePool { files: &disk, paths: &pool };
            let layered = files::layered(&held, &disk);
            let index = library.with_edits(&held, &layered);
            let files =
                rnl_view::LibraryProvider { files: &layered, index: &index, enabled: auto_include.unwrap_or(false) };
            let formats = formats.unwrap_or_default();
            match &progress {
                Some((window, id)) => {
                    let provider = load_progress::LoadingProvider::new(&files, window, Some(id));
                    job(drawn, &entry, &provider, &formats)
                }
                None => job(drawn, &entry, &files, &formats),
            }
        })
    })
    .await
    .map_err(|e| e.to_string())??
}

/// The version of the core and of the shape of its readings, which the key of a reading kept has.
#[tauri::command]
pub fn reading_core() -> serde_json::Value {
    serde_json::json!({ "version": rnl_view::core_version(), "format": rnl_view::FILE_RESULT_FORMAT })
}

/// The outline of the document of `source` (`rnl_view::Outline`).
#[tauri::command]
pub async fn read_outline(
    window: WebviewWindow,
    core: State<'_, Core>,
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    source: Source,
    request_id: Option<String>,
) -> Result<tauri::ipc::Response, String> {
    let progress = request_id.map(|id| (window, id));
    let json = with_files(&core, &settings, &library, source, progress, |_, entry, files, formats| {
        let outline = rnl_view::outline(entry, files, &options(formats.clone(), None, false))?;
        serde_json::to_string(&outline).map_err(|e| e.to_string())
    })
    .await?;
    Ok(tauri::ipc::Response::new(json))
}

/// The reading of `file` of the document of `source`, alone in the context of its entry (`rnl_view::FileResult`). With
/// `keep`, the drawings of the objects it lists without a position are kept for `unplaced_drawings`, in place of those
/// of the last reading kept: the reading is the one the page shows. With `bind`, the parameters of a subcircuit are
/// built with those values, to show; with `constraints`, the levels have their constraints, which the Solve view lists.
// Tauri injects the window and managed state alongside the separate IPC arguments.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn read_file(
    window: WebviewWindow,
    core: State<'_, Core>,
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    source: Source,
    file: String,
    keep: Option<bool>,
    bind: Option<Bind>,
    constraints: Option<bool>,
    request_id: Option<String>,
) -> Result<tauri::ipc::Response, String> {
    let progress = request_id.map(|id| (window, id));
    let json = with_files(&core, &settings, &library, source, progress, move |drawn, entry, files, formats| {
        let options = options(formats.clone(), bind, constraints.unwrap_or(false));
        let result = if keep.unwrap_or(false) {
            let (result, drawings) = rnl_view::build_file_drawn(entry, &file, files, &options)?;
            drawn.keep(drawings);
            result
        } else {
            rnl_view::build_file(entry, &file, files, &options)?
        };
        serde_json::to_string(&result).map_err(|e| e.to_string())
    })
    .await?;
    Ok(tauri::ipc::Response::new(json))
}

/// The file `path` of the document of `entry`, as the document names it, or a package by its own path, as the core
/// reads it; None for a bundled one, which the page does not ask about, or one that is not there.
fn read_named(entry: &str, path: &str, files: &dyn Provider) -> Option<String> {
    if path.starts_with('<') {
        return None;
    }
    files.read(&files::located(entry, path))
}

/// The hash of each of `paths`, files of the document of `source` as it names them, or packages by their paths, as the
/// core makes it (`OutlineFile::text_hash`): the first 16 hexadecimal digits of the SHA-256 of the text; None for one
/// that is not there.
#[tauri::command]
pub async fn file_hashes(
    core: State<'_, Core>,
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    source: Source,
    paths: Vec<String>,
) -> Result<Vec<Option<String>>, String> {
    with_files(&core, &settings, &library, source, None, move |_, entry, files, _| {
        Ok(paths
            .iter()
            .map(|p| read_named(entry, p, files).map(|text| hex(&sha256(text.as_bytes()))[..16].to_string()))
            .collect())
    })
    .await
}

/// The text of each of `paths`, files of the document of `source` as it names them, as the core reads them; None for
/// one that is not there. For the netlist pane: a reading of one file leaves out the texts of the others.
#[tauri::command]
pub async fn file_texts(
    core: State<'_, Core>,
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    source: Source,
    paths: Vec<String>,
) -> Result<Vec<Option<String>>, String> {
    with_files(&core, &settings, &library, source, None, move |_, entry, files, _| {
        Ok(paths.iter().map(|p| read_named(entry, p, files)).collect())
    })
    .await
}

/// The symbols the subcircuits of `file`, a file of the document of `source`, give themselves (`rnl_view::FileSymbols`),
/// for the entry page.
#[tauri::command]
pub async fn file_symbols(
    core: State<'_, Core>,
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    source: Source,
    file: String,
) -> Result<tauri::ipc::Response, String> {
    let json = with_files(&core, &settings, &library, source, None, move |_, entry, files, formats| {
        let symbols = rnl_view::symbols(entry, &file, files, &options(formats.clone(), None, false))?;
        serde_json::to_string(&symbols).map_err(|e| e.to_string())
    })
    .await?;
    Ok(tauri::ipc::Response::new(json))
}
