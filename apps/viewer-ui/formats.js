// The parse format of each file (spice, spectre, ...): the one the user chose in the Library, by the
// file's path as the core names it (Document.path joined with SourceFile.path, source-path.js), and
// for the files without a choice the core's own default (SourceFile.default_format). The formats
// themselves are the core's list (Document.formats), which the page never writes out itself.
import { shell } from "./shell.js";
import { m } from "./i18n.js";
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

/**
 * Whether `file`, a file of the Document, is shown as plain text: the core says it is no netlist, its
 * format being text, which it gives a file of a hardware description language, such as Verilog-A, that
 * has none set, and a file the user set to Text. The core keeps it as text and reads none of it; what
 * the user chose, a file of that suffix read as SPICE, is a netlist again.
 */
export const isPlainFile = (file) => file.format === TEXT;

// the names of the formats are no words to translate, but "text" is one
const TEXT = "text";
const TITLES = new Map([[TEXT, m("Text")]]);

/** The title of the format named `name` as the menus show it: the core's, or the translation of the word it is. */
export function formatTitle(name) {
  return TITLES.get(name) ?? formatInfo(name)?.title ?? name;
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
