// A file of the document shown that changed on disk since it was read: the user is told once, with the key that reads
// it again (File › Reload, F5). The desktop and the local program look at the files (rnl-viewer-files watch.rs); the page
// gives them those of the document shown (`shell.watchFiles`) and asks once a second what changed (`shell.fileChanges`).
// The web reads no file of a disk, and has neither.
import { m } from "./i18n.js";
import { note } from "./editor.js";
import { shell } from "./shell.js";
import { doc } from "./state.js";
import { sourcePath } from "./source-path.js";

let version = 0;
let started = false;

/** The files of the Document `d`, now shown, are those the shell is to look at. */
export function watchShown(d) {
  if (!shell.watchFiles || !d) return;
  shell.watchFiles(d.files.map((f) => sourcePath(d, f)));
}

async function ask() {
  try {
    const { version: now, changed } = await shell.fileChanges(version);
    version = now;
    if (doc && changed.length) {
      const shown = new Set(doc.files.map((f) => sourcePath(doc, f)));
      const mine = changed.filter((p) => shown.has(p));
      if (mine.length) note(m`${mine[0].split(/[\\/]/).pop()} changed on disk. F5 reloads the netlist.`);
    }
  } catch {
    // the shell has not answered: the next time
  }
  setTimeout(ask, 1000);
}

/** Ask the shell once a second what changed, where it watches files. */
export function watchDisk() {
  if (started || !shell.fileChanges) return;
  started = true;
  void ask();
}
