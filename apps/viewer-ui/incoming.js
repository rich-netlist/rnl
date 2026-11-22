// Files that come in: chosen, dropped, opened by the system or in a folder included. Which of them the viewer lists, and
// which it opens, the same in every shell: the suffixes are the core's (formats: `listed` and `entries`), the netlists
// opened on their own first, then the files mostly included or used by a netlist; a file of no listed suffix comes
// last and is not in a folder's listing.
import { m } from "./i18n.js";
import { choose } from "./dialog.js";
import { formatList, setFormatList } from "./formats.js";
import { shell } from "./shell.js";

let known = null; // { entries, parts }, once the core's formats are known

/** The suffixes the core lists, `{ entries, parts }`: the netlists opened on their own, then the others. */
export async function listedSuffixes() {
  if (known) return known;
  let formats = formatList();
  if (!formats.some((f) => Array.isArray(f.listed))) {
    formats = (await shell.formats?.().catch(() => null)) ?? [];
    setFormatList(formats);
  }
  const entries = formats.flatMap((f) => f.entries ?? []);
  const parts = formats.flatMap((f) => f.listed ?? []).filter((s) => !entries.includes(s));
  if (entries.length) known = { entries, parts };
  return { entries, parts };
}

/** The suffix of a path's file name, in lower case; "" when it has none. */
export function suffix(path) {
  const name = path.slice(Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")) + 1);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
}


/** The suffixes the core lists, written for a sentence: ".sp, .spi, … or .rnl". */
export async function listedText() {
  const { entries, parts } = await listedSuffixes();
  const all = [...entries, ...parts].map((s) => `.${s}`);
  return all.length > 1 ? m("{0} or {1}", all.slice(0, -1).join(", "), all.at(-1)) : all.join("");
}

const byPath = (a, b) => a.localeCompare(b, undefined, { numeric: true });

/**
 * The entry among `paths`, the file to open. Any file can be the netlist, whatever its name: the netlists are offered
 * first, then the files mostly included, then the others. Of files chosen or dropped, the entry is the only one not
 * mostly included, or the only one; when more than one could be, the user picks. In a folder, named by `folder` (""
 * for several things dropped, a folder among them), the user always picks, with a filter. `label` names a path in
 * the list. Null when the user cancels.
 */
export async function chooseEntry(paths, { folder = null, label = (p) => p } = {}) {
  const { entries, parts } = await listedSuffixes();
  const rank = (p) => (entries.includes(suffix(p)) ? 0 : parts.includes(suffix(p)) ? 1 : 2);
  const offered = [...paths].sort((a, b) => rank(a) - rank(b) || byPath(a, b));
  if (folder === null) {
    const own = offered.filter((p) => rank(p) !== 1);
    if (own.length === 1) return own[0];
    if (offered.length === 1) return offered[0];
  }
  let text = m("Select the top-level netlist to open:");
  if (folder !== null) text = folder ? m`The netlist to open in ${folder}:` : m("The netlist to open:");
  return choose({
    title: folder === null ? m("Open") : m("Open Folder"),
    text,
    items: offered.map((p) => ({ label: label(p), value: p })),
    filter: folder !== null,
  });
}
