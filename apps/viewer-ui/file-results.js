// The core's readings of one file (FileResult) kept between reads, for a shell that reads the file shown alone
// (per-file loading): the key a reading is kept under, and whether what it recorded it depends on (`FileInputs`) still
// holds. The cache itself is result-cache.js, which knows nothing of what it keeps.
//
// A reading is found by its key: the core's version and the shape of its results, the options that change it, the
// entry, the file and the hash of its text. It is given again only when every input still holds: the hash of each
// file and package it read (a bundled package changes only with the core, which the key has), where each name its
// levels instantiate is defined and the global nets, as the outline read now has them, the Library, and the host
// levels it names, which the outline numbers: a file that gains or loses a subcircuit before it moves them, and the
// reading is made again.

import { cacheKey } from "./result-cache.js";

/** The hash the core gives a text (`OutlineFile.text_hash`): the first 16 hexadecimal digits of the SHA-256 of its UTF-8 bytes. */
export async function textHash(text) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return [...new Uint8Array(digest).slice(0, 8)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** The options of a request that change a reading, as its key has them. */
const OPTIONS = ["svg", "primitives", "flylines", "hints", "lazy_unplaced", "auto_include", "library_prefix", "formats", "bind", "constraints", "dialect"];

/** The key of the reading of `file` of the document `request.entry` names, whose text is `text`, with the core `core`. */
export async function readingKey(core, request, file, text) {
  const options = Object.fromEntries(OPTIONS.filter((k) => request[k] !== undefined).map((k) => [k, request[k]]));
  return cacheKey({ core: core.version, format: core.format, entry: request.entry, file, text: await textHash(text), options });
}

/** The file `name`, as the document names it (relative to the entry's folder, or a path of its own), as the files of a request name it. */
export function requestPath(entry, name) {
  // (a file of the Library, which the document names under <pool>/, is under /<pool>/ in the files of the request)
  if (name.startsWith("<pool>/")) return `/${name}`;
  if (/^(?:[A-Za-z]:\/|\/|<)/.test(name)) return name;
  return `${entry.slice(0, entry.lastIndexOf("/") + 1)}${name}`;
}

/** Where each name the levels of `file` call is defined, by the outline: master → `{ file, title }`. */
function calls(outline, file) {
  const at = outline.files.findIndex((f) => f.path === file);
  const out = new Map();
  if (at < 0) return out;
  for (const l of outline.files[at].levels) {
    for (const c of outline.levels[l].calls) {
      if (out.has(c.master)) continue;
      const def = c.level === null ? null : outline.levels[c.level];
      out.set(c.master, def ? { file: def.file === null ? null : outline.files[def.file].path, title: def.title, host: c.level } : { file: null, title: null, host: null });
    }
  }
  return out;
}

/**
 * Whether the reading `kept` (a FileResult) still holds for the document as it is read now: `outline` its outline,
 * `textOf(path)` the text of a file or package by the path a request names it by (undefined when there is none), and
 * `library` the stamp of the Library now, against `kept.library`, the one it was read with.
 */
export async function holds({ result, library: stamp }, { outline, textOf, library }) {
  const inputs = result.inputs;
  for (const input of inputs.files) {
    if (input.bundled) continue;
    const text = textOf(input.role === "package" ? input.path : requestPath(inputs.entry, input.path));
    if (text === undefined || await textHash(text) !== input.hash) return false;
  }
  // the Library, always: a reading that found no package, or no definition, by a name depends on it too, though it does
  // not say so (`inputs.library` is set only when it used a file of it)
  if (stamp !== library) return false;
  const now = calls(outline, inputs.file);
  for (const c of inputs.outline.calls) {
    const found = now.get(c.master);
    if (!found || found.file !== c.file || found.title !== c.title) return false;
  }
  if (JSON.stringify(inputs.outline.globals) !== JSON.stringify(outline.globals)) return false;
  // the host levels it names: its own, in the order of the file, and those of the subcircuits it calls
  const at = outline.files.findIndex((f) => f.path === inputs.file);
  const own = at < 0 ? [] : outline.files[at].levels.filter((l) => outline.levels[l].circuit);
  if (JSON.stringify(own) !== JSON.stringify(result.levels.map((l) => l.host))) return false;
  return result.children.every((c) => c.host === null || now.get(c.master)?.host === c.host);
}
