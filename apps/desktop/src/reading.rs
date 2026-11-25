//! What the page reads a netlist with, file by file (apps/viewer-ui/file-reading.js, the desktop shell's `reader`): the
//! outline of the whole document, the reading of one file alone in the context of its entry, the hashes and texts of
//! its files as the core reads them, and the symbols of a file's subcircuits. The page keeps the readings, in memory,
//! by the rules every shell keeps them by; the app keeps nothing of them but the drawings of what the reading shown
//! has with no position, which the page asks for as it scrolls (`unplaced_drawings`). Only a reading that keeps them,
//! and the outline of the document it is of, run on the core thread, through its reading cache (`Drawn::cache`); the
//! others, the page's preloading and Check All Files among them, run beside it, so that they never hold up the reading
//! the page is to show. Its answer may be the smaller one, without the texts, which the page asks for apart, the spans
//! and the geometry hints, which the page asks for right after it (`read_rest`), from what the cache kept of that
//! reading: the page draws before they come. The texts it left out are kept in memory until the page asks for them
//! (`KeptTexts`), so that they are those the reading was made from, also when a file is saved in between.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use rnl_runtime::numbering::{hex, sha256};
use rnl_runtime::source::Provider;
use rnl_view::{Bind, BuildOptions, FileResult};
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

/// The texts of the files that the answer of the reading to show left out (`read_file` with `keep` and `later`), by path
/// as the document names them, kept for the open that made it, by its id, until the page asks for them (`file_texts`):
/// the netlist pane then shows the texts that the objects and the spans were read from, not those of a file saved in
/// between. Only those of the last reading to show are kept, in memory; they are given once, and dropped when the next
/// reading to show starts, so that an open taken over keeps none.
#[derive(Clone, Default)]
pub struct KeptTexts(Arc<Mutex<Option<Kept>>>);

/// The id of an open and the texts kept for it, by path.
type Kept = (String, HashMap<String, String>);

impl KeptTexts {
    fn lock(&self) -> MutexGuard<'_, Option<Kept>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Drop the texts kept, as another reading to show starts.
    fn clear(&self) {
        *self.lock() = None;
    }

    /// Keep `texts` for the open `id`, in place of any kept before.
    fn keep(&self, id: String, texts: HashMap<String, String>) {
        *self.lock() = Some((id, texts));
    }

    /// The texts kept for the open `id`, which are kept no longer; None when none are kept for it, and those of another
    /// open stay.
    fn take(&self, id: &str) -> Option<HashMap<String, String>> {
        let mut kept = self.lock();
        if kept.as_ref().is_some_and(|(of, _)| of == id) {
            kept.take().map(|(_, texts)| texts)
        } else {
            None
        }
    }
}

/// Leave the texts of `result`'s files out, as `rnl_view::leave_out_texts` does, and give those left out, by path: those
/// it has, as the reading of one file has none of the files it does not read.
fn take_texts(result: &mut FileResult) -> HashMap<String, String> {
    let texts = result
        .document
        .files
        .iter_mut()
        .filter(|f| !f.path.starts_with(rnl_view::BUNDLED_DIR) && !f.text.is_empty())
        .map(|f| (f.path.clone(), std::mem::take(&mut f.text)))
        .collect();
    rnl_view::leave_out_texts(result);
    texts
}

/// The text of each of `paths`: the one `kept`, else as `read` reads it.
fn texts_of(
    paths: &[String],
    mut kept: HashMap<String, String>,
    read: impl Fn(&str) -> Option<String>,
) -> Vec<Option<String>> {
    paths.iter().map(|p| kept.remove(p).or_else(|| read(p))).collect()
}

/// The settings of a reading of `source` besides the texts of its files, which the reading cache is used for: the entry,
/// the file pool, Auto Include and the formats.
fn cache_key(source: &Source) -> String {
    serde_json::json!([source.path, source.name, source.pool, source.auto_include, source.formats]).to_string()
}

/// The options of a reading: those the page's readings are made with, flylines and the layout hints asked for, not
/// left to the core's default (View > Show Flylines hides the flylines, and the problems list has the hints).
fn options(formats: formats::Formats, bind: Option<Bind>, constraints: bool) -> BuildOptions {
    BuildOptions { formats, bind, constraints, flylines: true, hints: true, ..BuildOptions::default() }
}

/// Run `job` with the drawings of the reading shown, the entry as the core names it, and the files of `source` as a
/// reading reads them: its edits over the disk, the file pool, the public packages, PATH and the Library, Auto Include
/// when it is on. With `drawn`, it runs on the core thread, which holds the drawings; otherwise on a thread of its own,
/// with none. With `progress`, the window is told when the first file is read (`load-stage`).
async fn with_files<T: Send + 'static>(
    core: &Core,
    settings: &settings::Settings,
    library: &library::Library,
    source: Source,
    (drawn, progress): (bool, Option<(WebviewWindow, String)>),
    job: impl FnOnce(&mut Drawn, &str, &dyn Provider, &formats::Formats) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let core = core.clone();
    let paths = settings.paths()?;
    let library = library.snapshot()?;
    tauri::async_runtime::spawn_blocking(move || {
        let read = move |drawn: &mut Drawn| {
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
        };
        if drawn {
            core.run(read)?
        } else {
            read(&mut Drawn::default())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The version of the core and of the shape of its readings, which the key of a reading kept has.
#[tauri::command]
pub fn reading_core() -> serde_json::Value {
    serde_json::json!({ "version": rnl_view::core_version(), "format": rnl_view::FILE_RESULT_FORMAT })
}

/// The outline of the document of `source` (`rnl_view::Outline`). With `cache`, the outline of the document the page is
/// to show: it runs on the core thread, through the reading cache, for the reading that keeps the drawings to take.
#[tauri::command]
pub async fn read_outline(
    window: WebviewWindow,
    core: State<'_, Core>,
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    source: Source,
    cache: Option<bool>,
    request_id: Option<String>,
) -> Result<tauri::ipc::Response, String> {
    let progress = request_id.map(|id| (window, id));
    let cache = cache.unwrap_or(false);
    let key = cache_key(&source);
    let json =
        with_files(&core, &settings, &library, source, (cache, progress), move |drawn, entry, files, formats| {
            let options = options(formats.clone(), None, false);
            let outline = if cache {
                rnl_view::outline_cached(entry, files, &options, drawn.cache(&key))?
            } else {
                rnl_view::outline(entry, files, &options)?
            };
            serde_json::to_string(&outline).map_err(|e| e.to_string())
        })
        .await?;
    Ok(tauri::ipc::Response::new(json))
}

/// The reading of `file` of the document of `source`, alone in the context of its entry (`rnl_view::FileResult`). With
/// `keep`, the drawings of the objects it lists without a position are kept for `unplaced_drawings`, in place of those
/// of the last reading kept: the reading is the one the page shows, made through the reading cache; with `later` too,
/// its answer is without the texts of the files, the spans and the geometry hints, which `read_rest` gives next (the
/// spans when the cache could keep them, else they stay in the answer). With `bind`, the parameters of a subcircuit
/// are built with those values, to show; with `constraints`, the levels have their constraints, which the Solve view
/// lists. The texts the answer leaves out are kept for the open of `request_id` (`KeptTexts`); a reading with `keep`
/// drops those kept before.
// Tauri injects the window and managed state alongside the separate IPC arguments.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn read_file(
    window: WebviewWindow,
    core: State<'_, Core>,
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    kept: State<'_, KeptTexts>,
    source: Source,
    file: String,
    keep: Option<bool>,
    later: Option<bool>,
    bind: Option<Bind>,
    constraints: Option<bool>,
    request_id: Option<String>,
) -> Result<tauri::ipc::Response, String> {
    let open = request_id.clone();
    let progress = request_id.map(|id| (window, id));
    let drawn = keep.unwrap_or(false);
    let key = cache_key(&source);
    let kept = kept.inner().clone();
    let json =
        with_files(&core, &settings, &library, source, (drawn, progress), move |drawn, entry, files, formats| {
            let mut options = options(formats.clone(), bind, constraints.unwrap_or(false));
            let result = if keep.unwrap_or(false) {
                // (on the core thread, as the readings to show are made in order)
                kept.clear();
                let later = later.unwrap_or(false);
                options.hints = !later;
                let (mut result, drawings) =
                    rnl_view::build_file_drawn_cached(entry, &file, files, &options, drawn.cache(&key))?;
                drawn.keep(drawings);
                if later {
                    drawn.cache(&key).keep_spans(&mut result);
                    let texts = take_texts(&mut result);
                    if let Some(open) = open {
                        kept.keep(open, texts);
                    }
                }
                result
            } else {
                rnl_view::build_file(entry, &file, files, &options)?
            };
            serde_json::to_string(&result).map_err(|e| e.to_string())
        })
        .await?;
    Ok(tauri::ipc::Response::new(json))
}

/// What the answer of `file` that `read_file` with `keep` and `later` just made through the reading cache was left
/// without (`rnl_view::FileRest`): its spans and the geometry hints of its levels, from what the cache kept of that
/// reading, with the same `source`, `bind` and `constraints`; the error "not kept" when the cache keeps no such
/// reading over the same files, and the page reads the file again with them. It gives them once.
#[tauri::command]
pub async fn read_rest(
    core: State<'_, Core>,
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    source: Source,
    file: String,
    bind: Option<Bind>,
    constraints: Option<bool>,
) -> Result<tauri::ipc::Response, String> {
    let key = cache_key(&source);
    let json = with_files(&core, &settings, &library, source, (true, None), move |drawn, entry, files, formats| {
        let options = BuildOptions { hints: false, ..options(formats.clone(), bind, constraints.unwrap_or(false)) };
        let rest = rnl_view::file_rest_cached(entry, &file, files, &options, drawn.cache(&key))?;
        serde_json::to_string(&rest).map_err(|e| e.to_string())
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
    with_files(&core, &settings, &library, source, (false, None), move |_, entry, files, _| {
        Ok(paths
            .iter()
            .map(|p| read_named(entry, p, files).map(|text| hex(&sha256(text.as_bytes()))[..16].to_string()))
            .collect())
    })
    .await
}

/// The text of each of `paths`, files of the document of `source` as it names them, as the core reads them; None for
/// one that is not there. For the netlist pane: a reading of one file leaves out the texts of the others. Those that
/// the reading to show of the open `request_id` left out are the texts it was made from (`KeptTexts`), given once.
#[tauri::command]
pub async fn file_texts(
    core: State<'_, Core>,
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    kept: State<'_, KeptTexts>,
    source: Source,
    paths: Vec<String>,
    request_id: Option<String>,
) -> Result<Vec<Option<String>>, String> {
    let kept = request_id.and_then(|id| kept.take(&id)).unwrap_or_default();
    with_files(&core, &settings, &library, source, (false, None), move |_, entry, files, _| {
        Ok(texts_of(&paths, kept, |p| read_named(entry, p, files)))
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
    let json = with_files(&core, &settings, &library, source, (false, None), move |_, entry, files, formats| {
        let symbols = rnl_view::symbols(entry, &file, files, &options(formats.clone(), None, false))?;
        serde_json::to_string(&symbols).map_err(|e| e.to_string())
    })
    .await?;
    Ok(tauri::ipc::Response::new(json))
}

#[cfg(test)]
mod tests {
    use rnl_view::MemoryProvider;

    use super::*;

    const FIRST: &str = "* first\n.include 'cell.inc'\nR1 a 0 1k\nX1 a 0 cell\n.end\n";
    const CELL: &str = "* cell\n.subckt cell p q\nR1 p q 1k\n.ends\n";

    /// The answer of the reading to show, with the texts it leaves out kept for the open `id`, as `read_file` makes it.
    fn read(kept: &KeptTexts, id: &str, files: &MemoryProvider) -> FileResult {
        kept.clear();
        let mut result = rnl_view::build_file("/w/top.sp", "top.sp", files, &Default::default()).unwrap();
        let texts = take_texts(&mut result);
        kept.keep(id.to_string(), texts);
        result
    }

    #[test]
    fn the_texts_left_out_are_those_the_reading_was_made_from_though_a_file_is_saved_after() {
        let kept = KeptTexts::default();
        let disk = MemoryProvider::new().with("/w/top.sp", FIRST).with("/w/cell.inc", CELL);
        let result = read(&kept, "open-1", &disk);
        // the answer has none of the texts, as rnl_view::leave_out_texts leaves them out
        let paths: Vec<String> = result.document.files.iter().map(|f| f.path.clone()).collect();
        assert_eq!(paths, ["top.sp", "cell.inc"]);
        assert!(result.document.files.iter().all(|f| f.text.is_empty()));
        // the file is saved before the page asks: a line before R1
        let after = format!("* saved\n{FIRST}");
        let saved = MemoryProvider::new().with("/w/top.sp", &after).with("/w/cell.inc", CELL);
        let from_disk = |p: &str| saved.read(&format!("/w/{p}"));
        let texts = texts_of(&paths, kept.take("open-1").unwrap(), from_disk);
        // (the file included is not read by the reading of the entry: its text is read from the disk)
        assert_eq!(texts, [Some(FIRST.to_string()), Some(CELL.to_string())]);
        // given once: asked again, they are read from the disk
        assert!(kept.take("open-1").is_none());
        let again = texts_of(&paths, kept.take("open-1").unwrap_or_default(), from_disk);
        assert_eq!(again[0].as_deref(), Some(after.as_str()));
        // a path not left out is read from the disk too
        let disk_only = texts_of(&["other.inc".to_string()], HashMap::new(), |_| Some("x".to_string()));
        assert_eq!(disk_only, [Some("x".to_string())]);
    }

    #[test]
    fn only_the_texts_of_the_last_reading_to_show_are_kept_and_only_for_its_open() {
        let kept = KeptTexts::default();
        let disk = MemoryProvider::new().with("/w/top.sp", FIRST).with("/w/cell.inc", CELL);
        read(&kept, "open-1", &disk);
        // another open does not take them, and they stay for their own
        assert!(kept.take("open-2").is_none());
        // the next reading to show starts: those of the open taken over are dropped
        kept.clear();
        assert!(kept.take("open-1").is_none());
        // the next reading's own, kept for its open alone
        read(&kept, "open-2", &disk);
        assert!(kept.take("open-1").is_none());
        let texts = kept.take("open-2").unwrap();
        assert_eq!(texts.get("top.sp").map(String::as_str), Some(FIRST));
        assert_eq!(texts.len(), 1);
        assert!(kept.take("open-2").is_none());
    }
}
