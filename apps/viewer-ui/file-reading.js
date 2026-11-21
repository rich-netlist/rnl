// Reading a document file by file (per-file loading), the same in every shell: the outline of the whole document, then
// the file shown alone, in the context of its entry, each kept in memory for this page (result-cache.js) by the rules
// of file-results.js, and given again while they hold, with no core started and no outline made. A shell gives only
// what reads the files and starts the core (`shell.reader`, shell.js); what is kept, when it holds, Check All Files,
// what the entry page tells of each file and File › Memory Cache are here.
//
// A reading with the parameters of a subcircuit bound (params.js), or with the constraints of the Solve view, is never
// kept; neither is a solve's preview, which the shells make apart (solve.js). What the page draws itself is not kept.

import { MEMORY_LIMIT, ResultCache } from "./result-cache.js";
import * as rules from "./file-results.js";
import { shell } from "./shell.js";
import { doc as shownDoc } from "./state.js";
import { load } from "./util.js";
import { entrySource } from "./visit-source.js";

const MIB = 1024 * 1024;

/** The limit of the readings kept, in bytes, as File › Memory Cache sets it: 256 MiB by default; 0 keeps none. */
const cacheLimit = () => (shell.embed ? MEMORY_LIMIT : load("file-cache-memory-mib", MEMORY_LIMIT / MIB) * MIB);

let cache = null;
const kept = () => (cache ??= new ResultCache({ memoryLimit: cacheLimit() }));

// the Library's stamps the cache holds readings of (file-results.js libraryStamps), null before the first reading
let stamps = null;
/** The Library's stamps now; the cache is emptied when it really changed since the last reading. */
async function library() {
  const now = (await shell.libraryStamps?.()) ?? { user: "", all: "" };
  if (rules.libraryChanged(stamps, now)) await kept().clear({ counts: false });
  stamps = now;
  return now;
}

// what each Document given was read with: its reader and the outline, for the entry page (fileSummaries)
const readWith = new WeakMap();

/** The outline of the document `reader` reads: the one kept, while it holds, or one the core makes, which is kept. */
async function outlineOf(reader, lib, fresh) {
  const key = reader.core ? await rules.outlineKey(reader.core, reader.entry, reader.options) : null;
  if (key && !fresh) {
    const found = await kept().get(key, async (k) => rules.outlineHolds(k, await reader.hashes(rules.outlinePaths(k.outline)), lib), { counted: false });
    if (found) return found.outline;
  }
  const outline = await reader.outline();
  if (key) {
    const value = { outline, library: lib.all };
    await kept().put(key, value, value, { counted: false });
  }
  return outline;
}

/**
 * The reading of `file` of the document `reader` reads, whose outline is `outline`: the one kept, while it holds, or one
 * the core makes, which is kept when nothing was missing and `unkept` does not say otherwise. With `keep`, the core keeps
 * the drawings of what it has with no position, for the Document shown (`shell.unplacedDrawings`). `hit` says which.
 */
async function readOne(reader, outline, file, lib, { fresh = false, keep = false, unkept = false } = {}) {
  const shape = rules.outlineShape(outline);
  const hash = reader.core && !unkept ? (await reader.hashes([file])).get(file) : null;
  const key = hash ? await rules.readingKey(reader.core, reader.entry, reader.options, file, hash) : null;
  if (key && !fresh) {
    const found = await kept().get(key, async (k) => rules.readingHolds(k, await reader.hashes(rules.readingPaths(k.result)), shape, lib));
    if (found) return { result: structuredClone(found.result), hit: true };
  }
  const started = performance.now();
  const result = await reader.readFile(file, keep);
  // kept when the document has every file it asks for, with how long it took to read (the entry page tells it)
  if (key && result.missing.length === 0) {
    const value = { result: structuredClone(result), shape, library: lib.all, took: performance.now() - started };
    await kept().put(key, value, value);
  }
  return { result, hit: false };
}

/** The file a source shows: its view, or the entry's own file, the outline's first. */
const fileOf = (source, outline, reader) => source.view ?? outline.files[0]?.path ?? reader.entry.slice(reader.entry.lastIndexOf("/") + 1);

/**
 * Read the source to show, with `edits` over the files, as `shell.build` does: in a shell that reads the file shown
 * alone (`shell.reader`), the file the source shows (its `view`, or the entry's own), with the outline of the whole
 * document as `outline`; in another, the whole document, as the shell builds it. `options` are those of `shell.build`:
 * `fresh` reads every file again, and keeps what it reads; `bind` and `constraints` are read and not kept.
 */
export async function readDocument(source, edits, progress, signal, options = {}) {
  if (!shell.reader) return shell.build(source, edits, progress, undefined, signal, options);
  const { fresh = false, bind, constraints } = options;
  const reader = await shell.reader(entrySource(source), edits, { progress, signal, bind, constraints });
  let doc = null;
  let hit = false;
  let done = false;
  try {
    const lib = await library();
    const outline = await outlineOf(reader, lib, fresh);
    ({ result: doc, hit } = await readOne(reader, outline, fileOf(source, outline, reader), lib,
      { fresh, keep: true, unkept: !!bind || !!constraints }));
    doc.outline = outline;
    await reader.texts(doc);
    readWith.set(doc, { reader, outline });
    done = true;
    return doc;
  } finally {
    // a reading that failed on the way gives no Document to show
    reader.finish(done ? doc : null, hit);
  }
}

/**
 * Check All Files (file-checks.js): read each of `files`, files of the document `source` reads as its outline names
 * them, alone in the context of its entry, with `edits` over the texts, and give each to `each(file, result, error)` as
 * it is read: the reading (a FileResult), or null and why it could not be read. The readings are kept as an open keeps
 * them; the drawings the core keeps stay those of the Document shown. `signal` stops before the next.
 */
export async function checkFiles(source, edits, files, each, signal) {
  const reader = await shell.reader(entrySource(source), edits, { signal });
  const stopped = () => { if (signal?.aborted) throw new Error("superseded"); };
  try {
    const lib = await library();
    const outline = await outlineOf(reader, lib, false);
    for (const file of files) {
      stopped();
      let result = null, error = null;
      try { ({ result } = await readOne(reader, outline, file, lib)); }
      catch (e) { stopped(); error = e instanceof Error ? e.message : String(e); }
      stopped();
      each(file, result, error);
    }
  } finally {
    reader.finish(null, false);
  }
}

/**
 * Of each of `files`, files of the document shown as its outline names them, for the entry page (entry-page.js): the
 * symbols its subcircuits give themselves (`FileSymbols.subckts`, or null when they cannot be read), whether a reading
 * of it is kept that still holds, and how long that reading took, in milliseconds (null when not known).
 */
export async function fileSummaries(files) {
  const read = shownDoc && readWith.get(shownDoc);
  if (!read) return {};
  const { reader, outline } = read;
  const lib = stamps ?? { user: "", all: "" };
  const shape = rules.outlineShape(outline);
  const out = {};
  for (const file of files) {
    let subckts = null;
    try { subckts = (await reader.symbols(file)).subckts; } catch { /* no symbols: the page draws a box */ }
    const hash = reader.core ? (await reader.hashes([file])).get(file) : null;
    const key = hash ? await rules.readingKey(reader.core, reader.entry, reader.options, file, hash) : null;
    const found = key ? await kept().peek(key, async (k) => rules.readingHolds(k, await reader.hashes(rules.readingPaths(k.result)), shape, lib)) : null;
    // how long the last reading took, also of one that no longer holds
    const last = found ?? (key ? await kept().peek(key) : null);
    out[file] = { subckts, kept: !!found, took: last?.took ?? null };
  }
  return out;
}

/** What the readings kept hold, as `ResultCache.stats` gives it. */
export const readingStats = () => kept().stats();

/** Whether this page reads a document file by file, and keeps the readings: a shell with `reader`. */
export const readsByFile = () => !!shell.reader;

/**
 * File › Memory Cache (memory-cache.js): apply the limit saved to the readings kept, empty them with `clear`, and tell
 * what they hold, as `ResultCache.stats` gives it.
 */
export async function memoryCache(clear = false) {
  await kept().setLimits({ memoryLimit: cacheLimit() });
  if (clear) await kept().clear();
  return kept().stats();
}
