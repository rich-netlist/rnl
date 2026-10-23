import { inform, modal } from "./dialog.js";
import { changes, fileText, hasBuffer } from "./edits.js";
import { note } from "./editor.js";
import { m } from "./i18n.js";
import { libraryFiles } from "./library-files.js";
import { currentSource } from "./open.js";
import { isLibraryChanging, librarySnapshot } from "./pool.js";
import { shell } from "./shell.js";
import { doc } from "./state.js";

let downloading = false;

export async function downloadLibrary() {
  if (!shell.downloadLibrary || downloading || modal() || isLibraryChanging()) return;
  downloading = true;
  let preparing = false;
  try {
    const files = libraryFiles(librarySnapshot(), currentSource(), doc, path => hasBuffer(path) ? fileText(path) : null, changes(false));
    const result = await shell.downloadLibrary(files, () => { preparing = true; note(m("Preparing library ZIP…"), 0); });
    if (result) note(m`Downloaded library (${files.length} files).`);
  } catch (error) {
    if (preparing) note("");
    await inform({ title: m("Download Library"), paragraphs: [m`Could not download the library: ${error.message || error}`] });
  } finally {
    downloading = false;
  }
}
