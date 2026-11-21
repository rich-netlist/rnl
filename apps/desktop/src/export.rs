//! File > Export Image (apps/viewer-ui/export-image.js): the core draws a level of the document shown
//! with the overlays the page shows ([`rnl_view::overlays::svg`]), and the app saves the image the
//! page makes of it where the user says. Nothing is written anywhere else.

use tauri::ipc::{InvokeBody, Request};
use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use rnl_runtime::source::Layered;

use crate::{files, formats, library, menu, settings, solve};

/// The SVG of `levels[level]` of the reading of `file` of the document at `path`, or of `text` held as the file
/// `name`, or of the entry's own file when `file` is None, read as the page reads it (reading.rs), with
/// `overlays`. `title` must name that level, as the page's document does: a document that is not the one shown is
/// refused. With `preview`, the document is the last solve's, drawn as `solve_preview` builds it, whole: the files the
/// solve wrote over the files as they are, so that the image is the preview shown and not the netlist as written.
#[tauri::command]
// Tauri injects the managed state alongside the separate IPC arguments.
#[allow(clippy::too_many_arguments)]
pub async fn export_svg(
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    solves: State<'_, solve::Solves>,
    auto_include: Option<bool>,
    pool: Option<Vec<String>>,
    path: Option<String>,
    name: Option<String>,
    text: Option<String>,
    file: Option<String>,
    edits: Option<files::Edits>,
    formats: Option<formats::Formats>,
    level: usize,
    title: String,
    overlays: rnl_view::OverlayOptions,
    preview: Option<bool>,
) -> Result<String, String> {
    let paths = settings.paths()?;
    let solved = if preview == Some(true) {
        Some(solves.written()?.ok_or("there is no solve shown to export: solve the netlist again")?)
    } else {
        None
    };
    let library = library.snapshot()?;
    let pool = pool.unwrap_or_default();
    // flylines are asked for, so that the export can leave them out as the page shows (export-image.js)
    let options = rnl_view::BuildOptions { formats: formats.unwrap_or_default(), flylines: true, ..Default::default() };
    tauri::async_runtime::spawn_blocking(move || {
        let (entry, held) = files::entry_held(path, name, text, &edits.unwrap_or_default())?;
        let disk = crate::public_packages::provider(&paths);
        let disk = rnl_runtime::source::FilePool { files: &disk, paths: &pool };
        let layered = files::layered(&held, &disk);
        let index = library.with_edits(&held, &layered);
        let provider =
            rnl_view::LibraryProvider { files: &layered, index: &index, enabled: auto_include.unwrap_or(false) };
        let doc = match &solved {
            // the solve's files over the others, as the preview was built (rnl_cli::preview)
            Some(written) => rnl_view::build(&entry, &Layered { layers: vec![written, &provider] }, &options)?,
            // the file shown, read alone in the context of its entry, as it was opened
            None => {
                let file = match file {
                    Some(file) => file,
                    None => rnl_view::outline(&entry, &provider, &options)?
                        .files
                        .into_iter()
                        .next()
                        .map(|f| f.path)
                        .ok_or_else(|| format!("{entry} has no file to read"))?,
                };
                rnl_view::build_file(&entry, &file, &provider, &options)?.document
            }
        };
        let shown = doc.levels.get(level).filter(|l| l.title == title).ok_or_else(|| {
            format!(
                "level {title} has changed since the document was loaded: reload or reopen the file and export again"
            )
        })?;
        rnl_view::overlays::svg(shown, &overlays)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Ask where to save the image in the request's body, named after its `name` header (URI-encoded),
/// an SVG or a PNG as its `kind` header says, and write it there; its path, or None when the dialog
/// is cancelled.
#[tauri::command]
pub async fn save_image(window: WebviewWindow, request: Request<'_>) -> Result<Option<String>, String> {
    let InvokeBody::Raw(bytes) = request.body() else { return Err("save_image takes the image's bytes".into()) };
    let header = |key: &str| request.headers().get(key).and_then(|v| v.to_str().ok()).map(decode);
    let name = header("name").unwrap_or_else(|| "drawing".into());
    let png = header("kind").as_deref() == Some("png");
    let bytes = bytes.clone();
    let (filter, extension) = if png { ("PNG", "png") } else { ("SVG", "svg") };
    let dialog = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title(menu::text(window.app_handle(), "Export Image"))
        .set_file_name(name)
        .add_filter(filter, &[extension]);
    let chosen =
        tauri::async_runtime::spawn_blocking(move || dialog.blocking_save_file()).await.map_err(|e| e.to_string())?;
    let Some(chosen) = chosen else { return Ok(None) };
    let to = chosen.into_path().map_err(|e| e.to_string())?;
    // (as Save As writes a file: an image that cannot be written whole leaves the old one as it was, and a link is not
    // written through; the dialog has asked before a file is replaced)
    tauri::async_runtime::spawn_blocking(move || rnl_viewer_files::save::write_chosen(&to, &bytes, true).map(|_| to))
        .await
        .map_err(|e| e.to_string())?
        .map(|to| Some(rnl_cli::path_str(&to)))
        .map_err(|e| e.to_string())
}

/// Percent-decoding of `encodeURIComponent`, as UTF-8.
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut k = 0;
    while k < bytes.len() {
        if bytes[k] == b'%' && k + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[k + 1..k + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
            if let Some(byte) = hex {
                out.push(byte);
                k += 3;
                continue;
            }
        }
        out.push(bytes[k]);
        k += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn names_are_decoded_as_encode_uri_component_wrote_them() {
        assert_eq!(decode("top%20%E7%94%B5%E8%B7%AF-amp.svg"), "top 电路-amp.svg");
        assert_eq!(decode("a%2"), "a%2");
    }
}
