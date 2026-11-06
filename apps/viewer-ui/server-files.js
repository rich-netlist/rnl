import { m, tr } from "./i18n.js";
// The file chooser of the local shell (shells/local.js): the files of the machine the program runs on, which the browser
// cannot show, listed by that program (`/api/ls`) and chosen one folder at a time with the list dialog of the page, whose
// first entries go up and, for a folder or a place to save, choose the folder shown. The browser's own chooser, which gives
// the files of the computer the browser is on, is the last entry of a list that asks for a file to open.

import { ask, choose } from "./dialog.js";

/** What the program lists of a folder: its real path, its parent and its entries, folders first. */
export async function listFolder(api, dir) {
  const query = new URLSearchParams(dir ? { dir } : {});
  return api(`api/ls?${query}`);
}

const joinPath = (dir, name) => (dir.endsWith("/") ? dir + name : `${dir}/${name}`);

/**
 * Choose a file, a folder, or a place to save a file on the machine of the program, starting in `start`.
 *
 * - `kind` "file": a file; with `browser` (a function), the last entry, "From the computer of the browser…", gives that
 *   function's answer instead, as `{ browser }`.
 * - `kind` "folder": a folder: the entry "Choose this folder" at the top chooses the one shown.
 * - `kind` "save": a file to write, in the folder shown: "Save in this folder" asks for the name, `name` first.
 *
 * Resolves to `{ path }`, or what `browser` gave, or null when the user cancelled. A folder that cannot be listed is said so
 * and the user is taken back to the one before.
 */
export async function chooseOnServer(api, { kind = "file", start = "", title, name = "", browser = null, action = tr("Open"), remember = () => {} }) {
  let dir = start;
  for (;;) {
    let listing;
    try {
      listing = await listFolder(api, dir);
    } catch (e) {
      if (!dir) throw e;
      // the folder that was asked for is gone or may not be read: from the home folder
      dir = "";
      continue;
    }
    dir = listing.dir;
    const items = [];
    if (kind === "folder") items.push({ label: m("Choose this folder"), value: { choose: dir } });
    if (kind === "save") items.push({ label: m("Save in this folder"), value: { save: dir } });
    if (listing.parent) items.push({ label: `../ ${m("Parent folder")}`, value: { go: listing.parent } });
    for (const entry of listing.entries) {
      if (entry.dir) items.push({ label: `${entry.name}/`, value: { go: joinPath(dir, entry.name) } });
      else if (kind === "file") items.push({ label: entry.name, value: { file: joinPath(dir, entry.name) } });
    }
    if (kind === "file" && browser) items.push({ label: m("From the computer of the browser…"), value: { browser: true } });
    const picked = await choose({ title: title ?? m("Open Netlist"), text: m`Folder: ${dir}`, items: items.map((i) => ({ ...i, label: String(i.label) })), filter: true, action });
    if (picked === null) return null;
    if (picked.go !== undefined) {
      dir = picked.go;
      continue;
    }
    remember(dir);
    if (picked.browser) return { browser: await browser() };
    if (picked.file !== undefined) return { path: picked.file };
    if (picked.choose !== undefined) return { path: picked.choose };
    if (picked.save !== undefined) {
      const written = await ask({ title: title ?? m("Save As"), text: m`Folder: ${dir}`, value: name, placeholder: m("Name"), action: tr("Save") });
      // a name, not a path: what follows the last slash
      const file = written?.trim().split(/[\\/]/).pop();
      return file ? { path: joinPath(dir, file) } : null;
    }
  }
}
