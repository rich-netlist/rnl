// The parse format of each file (spice, spectre, ...): the one the user chose in the Library, by the
// file's path as the core names it (Document.path joined with SourceFile.path, source-path.js), and
// for the files without a choice the core's own default (SourceFile.default_format). The formats
// themselves are the core's list (Document.formats), which the page never writes out itself.
import { shell } from "./shell.js";
import { load, save } from "./util.js";

const KEY = "file-formats";
// the formats of the last Document, in the core's order: { name, title, extensions, fallback }
let list = [];
// choices kept only while the page is open: files a web page reads without a stable address
const session = new Map();

/** The key a file's choice is kept by: the shell's (case folded on a desktop that ignores case), or the path. */
const key = (path) => shell.formatKey?.(path) ?? path;

function stored() {
  const value = load(KEY, {});
  return value && typeof value === "object" && !Array.isArray(value) ? value : {};
}

/** Adopt the formats a Document lists, which the menus offer and the requests are filtered by. */
export function setFormatList(formats) {
  if (Array.isArray(formats) && formats.length) list = formats.filter((f) => typeof f?.name === "string");
}

export const formatList = () => list;

/** The format named `name`, from the list; undefined for a name the core does not offer. */
export const formatInfo = (name) => list.find((f) => f.name === name);

// The files of hardware description languages that are no netlists. The core lists only the parse formats
// spice, which reads any file it is not told otherwise, and spectre, so that its format does not say that
// a file is none; its suffix does. A list of these, not of netlists, so that a netlist with any other suffix
// (.txt, .sub, .cdl) is still shown as one.
const NOT_NETLISTS = new Set(["va", "vams", "v", "vh", "sv", "svh", "vhd", "vhdl"]);

/**
 * Whether the file at `path` is shown as plain text: it has the suffix of a file that is no netlist,
 * such as Verilog-A, and the user has chosen no parse format for it in the Library, which says it is
 * one. The core still reads it, as SPICE, and its problems are listed; the text shows none of that
 * reading in its colours, folds and marks.
 */
export function isPlainText(path, url = null) {
  if (chosenFormat(path, url) !== undefined) return false;
  const name = path.split(/[\\/]/).pop();
  const dot = name.lastIndexOf(".");
  return dot > 0 && NOT_NETLISTS.has(name.slice(dot + 1).toLowerCase());
}

/**
 * The user's choice for the file at `path`, or undefined when it reads by default; `url`, the file's
 * address when it has one, finds a choice kept by it while the file is not in the document shown.
 */
export function chosenFormat(path, url = null) {
  for (const k of url ? [key(path), url] : [key(path)]) {
    const name = session.has(k) ? session.get(k) : stored()[k];
    if (typeof name === "string") return name;
  }
  return undefined;
}

/**
 * The choices to give a build, `{ path: format }`. Names that the core's list does not have, such as
 * one kept by a newer viewer, are left out, so that a stale choice reads by default instead of
 * failing the build; before any list is known every choice goes, as the core checks them itself.
 */
export function fileFormats() {
  const all = { ...stored() };
  for (const [k, name] of session) all[k] = name;
  const known = new Set(list.map((f) => f.name));
  return Object.fromEntries(Object.entries(all).filter(([, name]) => typeof name === "string" && (!known.size || known.has(name))));
}

/**
 * Choose the format of the file at `path`, or with null its default again. The choice is kept in the
 * settings when the shell can name the file again later (`shell.keepsFormat`), else for this page only;
 * `{ session: true }` keeps it for this page only, as a project's formats are (project.js).
 */
export async function chooseFormat(path, name, { session: only = false } = {}) {
  const k = key(path);
  session.delete(k);
  if (only) {
    if (name) session.set(k, name);
    return;
  }
  const kept = shell.keepsFormat ? shell.keepsFormat(path) : true;
  const next = stored();
  if (name && kept) next[k] = name;
  else delete next[k];
  if (name && !kept) session.set(k, name);
  await save(KEY, next);
}
