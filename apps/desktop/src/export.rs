//! File > Export Image (apps/viewer-ui/export-image.js): the core draws a level of the document shown
//! with the overlays the page shows ([`rnl_view::overlays::svg`]), and the app saves the image the
//! page makes of it where the user says. Nothing is written anywhere else.

use std::path::PathBuf;

use rnl_cli::FsProvider;
use tauri::ipc::{InvokeBody, Request};
use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::{files, formats, library, menu, settings};

/// The SVG of `levels[level]` of the document at `path`, or of `text` held as the file `name`, built
/// as `open_document` and `open_text` build it, with `overlays`. `title` must name that level, as
/// the page's document does: a document that is not the one shown is refused.
#[tauri::command]
// Tauri injects the managed state alongside the separate IPC arguments.
#[allow(clippy::too_many_arguments)]
pub async fn export_svg(
    settings: State<'_, settings::Settings>,
    library: State<'_, library::Library>,
    auto_include: Option<bool>,
    pool: Option<Vec<String>>,
    path: Option<String>,
    name: Option<String>,
    text: Option<String>,
    edits: Option<files::Edits>,
    formats: Option<formats::Formats>,
    level: usize,
    title: String,
    overlays: rnl_view::OverlayOptions,
) -> Result<String, String> {
    let paths = settings.paths()?;
    let library = library.snapshot()?;
    let pool = pool.unwrap_or_default();
    let options = rnl_view::BuildOptions { formats: formats.unwrap_or_default(), ..Default::default() };
    tauri::async_runtime::spawn_blocking(move || {
        let edits = edits.unwrap_or_default();
        let (entry, held) = match (path, name, text) {
            (Some(path), _, _) => {
                let path = if path.starts_with(rnl_view::BUNDLED_DIR) {
                    PathBuf::from(&path)
                } else {
                    std::path::absolute(&path).map_err(|e| format!("{path}: {e}"))?
                };
                let entry = path.to_string_lossy().into_owned();
                let held = files::held(&entry, &edits);
                (entry, held)
            }
            (None, Some(name), Some(text)) => {
                let entry = format!("/{name}");
                let held = files::held_text(&entry, &text, &edits);
                (entry, held)
            }
            _ => return Err("export_svg needs a path, or a name and a text".to_string()),
        };
        let disk = FsProvider::new(&paths).with_search_paths(&paths);
        let disk = rnl_runtime::source::FilePool { files: &disk, paths: &pool };
        let layered = files::layered(&held, &disk);
        let index = library.with_edits(&held, &layered);
        let provider =
            rnl_view::LibraryProvider { files: &layered, index: &index, enabled: auto_include.unwrap_or(false) };
        let doc = rnl_view::build(&entry, &provider, &options)?;
        let shown = doc.levels.get(level).filter(|l| l.title == title).ok_or_else(|| {
            format!("the level {title} is not where the document shown has it: open the file again and export")
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
    tauri::async_runtime::spawn_blocking(move || std::fs::write(&to, bytes).map(|_| to))
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
