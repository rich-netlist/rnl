// Reading a document file by file (per-file loading), the same in every shell: the outline of the whole document, then
// the file shown alone, in the context of its entry, each kept in memory for this page (result-cache.js) by the rules
// of file-results.js, and given again while they hold, with no core started and no outline made. A shell gives only
// what reads the files and starts the core (`shell.reader`, shell.js); what is kept, when it holds, Check All Files,
// what the entry page tells of each file and File › Memory Cache are here.
//
// A reading with the parameters of a subcircuit bound (params.js) is never kept; neither is a solve's preview, which
// the shells make apart (solve.js). What the page draws itself is not kept. A reading with the constraints of the
// Solve view, which every reading asks for once the Solve view was opened (state.js `constraintsWanted`), is kept
// under a key of its own: the same file read with them and without them are two readings. Check All Files, the
// preloading and the entry page read and look for them as the readings shown are made then, so that all find the same.
//
// The first answer of the file to show is the smaller one, where the shell can (`restLater`): without the texts of the
// files, which the page has (`reader.texts`), the spans and the geometry hints of its levels. The core answers sooner,
// and the page draws it; the spans and the hints come next, from what the core kept of that reading (`restOf`), into
// the reading, and the page shows them then (open.js `restIn`). The reading is kept only then, with them, so that what
// is kept is what a reading with them gives.
//
// While the viewer is idle, the files of the subcircuits below the file shown are read too, one at a time, and kept
// into the room that is free, nothing else going for them (`preloadFile`, preload.js): the same in every shell, the
// desktop reading the files from disk, the web from where they were opened. The Library and PATH are not read so.

import { MEMORY_LIMIT, ResultCache } from "./result-cache.js";
import * as rules from "./file-results.js";
import { onlyRootOf } from "./roots.js";
import { shell } from "./shell.js";
import { constraintsWanted, doc as shownDoc } from "./state.js";
import { load } from "./util.js";
import { entrySource } from "./visit-source.js";

const MIB = 1024 * 1024;

/** Whether the readings are kept, as File › Memory Cache sets it: on by default. */
const cacheOn = () => shell.embed || load("file-cache-enabled", true) === true;
/** The limit of the readings kept, in bytes, as File › Memory Cache sets it: 256 MiB by default; 0, or the cache off, keeps none. */
const cacheLimit = () => (shell.embed ? MEMORY_LIMIT : cacheOn() ? load("file-cache-memory-mib", MEMORY_LIMIT / MIB) * MIB : 0);
/** Whether the files below the file shown are read while the viewer is idle, as File › Memory Cache sets it: on by default. */
const preloadOn = () => !shell.embed && load("file-preload", true) === true;

let cache = null;
const kept = () => (cache ??= new ResultCache({ memoryLimit: cacheLimit() }));

// the Library's stamps the cache holds readings of (file-results.js libraryStamps), null before the first reading
let stamps = null;
/**
 * The Library's stamps now, for the readings of the document `entry` (file-results.js libraryFor); the cache is emptied
 * when it really changed since the last reading.
 */
async function library(entry) {
  const now = (await shell.libraryStamps?.()) ?? { user: "", all: "" };
  if (rules.libraryChanged(stamps, now)) await kept().clear({ counts: false });
  stamps = now;
  return rules.libraryFor(now, entry);
}

// what each Document given was read with: its reader and the outline, for the entry page (fileSummaries)
const readWith = new WeakMap();

// the Documents given whose spans and hints are still to come, each with what brings them, and whether they came
const restToComeOf = new WeakMap();

/** The options a reading is kept by: the reader's, and the constraints when the reading has them (a key of its own). */
const keyOptions = (reader, constraints) => (constraints ? { ...reader.options, constraints: true } : reader.options);

/**
 * A reading kept, or to keep, as the Document the page shows: the reading itself is shared, never copied, and the page
 * writes nothing into it but what it writes here, into its own: the outline (`readDocument`) and the texts of the
 * files the reading leaves out (`reader.texts`). The tests freeze what is kept, so that a write anywhere else fails.
 */
const shown = (result) => ({ ...result, files: result.files.map((f) => ({ ...f })) });

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
 * Put what `result`, the smaller answer of `file` that `reader` read with `later`, was left without into it, before
 * anything else has it: the spans and the geometry hints the core kept of that reading (`reader.restOf`), or, when it
 * kept none, those of a reading of the file again with them. A level with no hints has none, as a reading with them
 * gives it; spans the answer has already are its own. False when neither could be had, such as when the open was
 * superseded and its core stopped: `result` is then left without them.
 */
async function addRest(reader, file, result) {
  let rest = null;
  try { rest = await reader.restOf(file); } catch { /* read again */ }
  if (!rest) {
    try {
      const again = await reader.readFile(file, false);
      rest = { spans: again.spans, hints: { levels: Object.fromEntries(again.levels.map((l) => [l.host, l.hints ?? []])) } };
    } catch {
      return false;
    }
  }
  if (result.spans.length === 0) for (const span of rest.spans) result.spans.push(span);
  for (const level of result.levels) {
    const list = rest.hints.levels[level.host];
    if (list?.length) level.hints = list;
  }
  return true;
}

/**
 * The reading of `file` of the document `reader` reads, whose outline is `outline`: the one kept, while it holds, or one
 * the core makes, which is kept when nothing was missing and `unkept` does not say otherwise. With `keep`, the core keeps
 * the drawings of what it has with no position, for the Document shown (`shell.unplacedDrawings`). `hit` says which,
 * and `stored` whether a new reading was kept. A reading `preload`ed counts as no hit or miss, and is kept only into
 * the room that is free. With `later`, where the shell can, the core's answer is the smaller one, and `rest` brings
 * the spans and the hints into it, after which the reading is kept (`stored` is false); one left without them is not.
 */
async function readOne(reader, outline, file, lib, { fresh = false, keep = false, unkept = false, preload = false, constraints = false, later = false } = {}) {
  const shape = rules.outlineShape(outline);
  const hash = reader.core && !unkept ? (await reader.hashes([file])).get(file) : null;
  const key = hash ? await rules.readingKey(reader.core, reader.entry, keyOptions(reader, constraints), file, hash) : null;
  if (key && !fresh) {
    const holds = async (k) => rules.readingHolds(k, await reader.hashes(rules.readingPaths(k.result)), shape, lib);
    const found = preload ? await kept().peek(key, holds) : await kept().get(key, holds);
    if (found) return { result: preload ? found.result : shown(found.result), hit: true, stored: false };
  }
  const started = performance.now();
  const restLater = later && reader.restLater === true;
  const result = await reader.readFile(file, keep, restLater);
  // the rest asked for at once, before the core is asked anything else
  const rest = restLater ? addRest(reader, file, result) : null;
  // kept when the document has every file it asks for, with how long it took to read (the entry page tells it)
  const keepIt = async () => {
    if (!key || result.missing.length > 0) return false;
    const value = { result, shape, library: lib.all, took: performance.now() - started };
    return kept().put(key, value, value, { evict: !preload });
  };
  if (rest) return { result: shown(result), hit: false, stored: false, rest: rest.then((came) => came && keepIt()).catch(() => false) };
  const stored = await keepIt();
  return { result: preload ? result : shown(result), hit: false, stored };
}

/** Whether the spans and the hints of `doc`, a Document `readDocument` gave, are still to come (`restIn`). */
export const restToCome = (doc) => !!doc && restToComeOf.get(doc)?.came === false;

/**
 * The spans and the hints of `doc`, a Document `readDocument` gave, are in it: at once, unless they come after it.
 * The page waits for this before it is done opening.
 */
export async function restIn(doc) {
  await (doc && restToComeOf.get(doc))?.coming;
}

/** The file a source shows: its view, or the entry's own file, the outline's first. */
const fileOf = (source, outline, reader) => source.view ?? outline.files[0]?.path ?? reader.entry.slice(reader.entry.lastIndexOf("/") + 1);

/**
 * Read the source to show, with `edits` over the files, as `shell.build` does: in a shell that reads the file shown
 * alone (`shell.reader`), the file the source shows (its `view`, or the entry's own), with the outline of the whole
 * document as `outline`; in another, the whole document, as the shell builds it. `options` are those of `shell.build`:
 * `fresh` reads every file again, and keeps what it reads; `bind` and `constraints` are read and not kept. With
 * `onlyRoot`, a source of the entry's own file whose entry page would leave at once for its one root (roots.js
 * `onlyRootOf`) is read as that root's file instead, and `onlyRoot` is given the root before it is read.
 */
export async function readDocument(source, edits, progress, signal, options = {}) {
  if (!shell.reader) return shell.build(source, edits, progress, undefined, signal, options);
  const { fresh = false, bind, constraints, onlyRoot } = options;
  const reader = await shell.reader(entrySource(source), edits, { progress, signal, bind, constraints });
  let doc = null;
  let hit = false;
  let done = false;
  try {
    const lib = await library(reader.entry);
    const outline = await outlineOf(reader, lib, fresh);
    // the entry's file draws nothing and its page would only open the one root: the root's file, read in its place
    const root = onlyRoot && !source.view ? onlyRootOf(outline) : null;
    if (root) onlyRoot(root);
    let rest;
    ({ result: doc, hit, rest } = await readOne(reader, outline, root?.file ?? fileOf(source, outline, reader), lib,
      { fresh, keep: true, unkept: !!bind, constraints: !!constraints, later: true }));
    if (rest) {
      const to = { coming: rest, came: false };
      restToComeOf.set(doc, to);
      rest.then(() => { to.came = true; });
    }
    // the loading indicator says so, in every shell
    if (hit) progress?.("cached");
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
  const constraints = constraintsWanted;
  const reader = await shell.reader(entrySource(source), edits, { signal, constraints });
  const stopped = () => { if (signal?.aborted) throw new Error("superseded"); };
  try {
    const lib = await library(reader.entry);
    const outline = await outlineOf(reader, lib, false);
    for (const file of files) {
      stopped();
      let result = null, error = null;
      try { ({ result } = await readOne(reader, outline, file, lib, { constraints })); }
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
  const lib = stamps ? await rules.libraryFor(stamps, reader.entry) : { user: "", all: "" };
  const shape = rules.outlineShape(outline);
  const out = {};
  for (const file of files) {
    let subckts = null;
    try { subckts = (await reader.symbols(file)).subckts; } catch { /* no symbols: the page draws a box */ }
    const hash = reader.core ? (await reader.hashes([file])).get(file) : null;
    const key = hash ? await rules.readingKey(reader.core, reader.entry, keyOptions(reader, constraintsWanted), file, hash) : null;
    const found = key ? await kept().peek(key, async (k) => rules.readingHolds(k, await reader.hashes(rules.readingPaths(k.result)), shape, lib)) : null;
    // how long the last reading took, also of one that no longer holds
    const last = found ?? (key ? await kept().peek(key) : null);
    out[file] = { subckts, kept: !!found, took: last?.took ?? null };
  }
  return out;
}

// the reading of the files below the Document shown while the viewer is idle: its reader, which the next one takes the
// place of, and the number of the preloading, which `stopPreload` moves on
let preloading = null;
let preloadNumber = 0;

/** Whether the files below the file shown are read while the viewer is idle: preloading and the cache on, and room to keep them. */
export const preloadAllowed = () => !!shell.reader && preloadOn() && cacheLimit() > 0;

/** Stop the preloading under way, as the user does something, or another Document is shown. */
export function stopPreload() {
  preloadNumber++;
  preloading?.controller.abort();
  preloading?.reader.finish(null, false);
  preloading = null;
}

/**
 * Read `file`, a file below the file shown (preload-queue.js `preloadFiles`), of the Document `doc` the source `source`
 * reads, and keep it, into the room that is free: "ready" when it is kept, or was, or cannot be (a file missing),
 * "full" when there is no room, "disabled" when preloading is off, "cancelled" when it was stopped, "skipped" when
 * there is nothing to read it with.
 */
export async function preloadFile(source, doc, file) {
  if (!preloadAllowed()) return "disabled";
  if (!source || !doc?.outline) return "skipped";
  if (kept().full()) return "full";
  const number = preloadNumber;
  if (preloading?.doc !== doc) {
    stopPreload();
    const controller = new AbortController();
    const at = ++preloadNumber;
    const constraints = constraintsWanted;
    const reader = await shell.reader(entrySource(source), undefined, { signal: controller.signal, background: true, constraints });
    if (preloadNumber !== at) { reader.finish(null, false); return "cancelled"; }
    preloading = { doc, reader, controller, lib: null, outline: null, number: at, constraints };
  } else if (preloading.number !== number) return "cancelled";
  const p = preloading;
  try {
    p.lib ??= await library(p.reader.entry);
    p.outline ??= await outlineOf(p.reader, p.lib, false);
    if (preloading !== p) return "cancelled";
    const read = await readOne(p.reader, p.outline, file, p.lib, { preload: true, constraints: p.constraints });
    if (preloading !== p) return "cancelled";
    return read.hit || read.stored || read.result.missing.length ? "ready" : "full";
  } catch (e) {
    if (preloading !== p) return "cancelled";
    throw e;
  }
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
  if (clear || !preloadAllowed()) stopPreload();
  await kept().setLimits({ memoryLimit: cacheLimit() });
  if (clear) await kept().clear();
  return kept().stats();
}
