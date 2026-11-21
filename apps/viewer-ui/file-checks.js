import { m } from "./i18n.js";
// The problems of the files of a document read one by one (per-file loading: a shell with `reader`, file-reading.js).
// The reading of the file shown has the problems of its own file, and of the files above it that it reads too; the
// problems list shows those of its own file (problems.js). For the other files, it keeps the problems of each file of
// the document as its own reading gave them, when one was shown or checked, and gives those of the outline (reading
// the netlist: an include not found, a subcircuit defined twice) for the files not read yet. Check All Files reads the
// files not read yet, one by one, most of them kept from before (file-results.js), so that the list is complete.
//
// What is kept belongs to the document as it is: its entry and the text of each of its files (the outline's hashes),
// with the edits applied. A file changed, edited or included anew, or a package that a reading used read with another
// text, starts it again.

import { doc } from "./state.js";
import { checkFiles, readsByFile } from "./file-reading.js";
import { changes } from "./edits.js";
import { currentSource } from "./open.js";

/** The readings kept: `fingerprint` of the document, `inputs` (path → hash of each package read), `files` (path → problems). */
let kept = null;
/** Check All Files while it reads: { controller, done, total }. */
let running = null;

const fingerprint = (d) => JSON.stringify([d.path, d.outline.files.map((f) => [f.path, f.text_hash])]);

/** Tell the problems list that what it lists of the other files has changed. */
const changed = () => window.dispatchEvent(new Event("file-checks"));

/**
 * The index of the file the Document `d` reads alone, among its files, when it is the reading of one file of a document
 * of several (per-file loading); -1 otherwise.
 */
export function readFileIndex(d) {
  if (!d?.outline || typeof d.file !== "string" || d.outline.files.length < 2) return -1;
  return d.files.findIndex((f) => f.path === d.file);
}

/**
 * Whether the problem `x` of the reading `result` is about the file it reads: in it, in no file, or in a file that is not
 * one of the document's (a package it uses), which no other reading would tell.
 */
function ownProblem(result, x, own, documentFiles) {
  if (x.file_index === null || x.file_index === undefined || x.file_index === own) return true;
  return !documentFiles.has(result.files[x.file_index]?.path);
}

/** The problems of the file shown, of the Document `d`: those of its own file, as `ownProblem` tells them. */
export function shownFileProblems(d, list) {
  const own = readFileIndex(d);
  if (own < 0) return list;
  const files = new Set(d.outline.files.map((f) => f.path));
  return list.filter((x) => ownProblem(d, x, own, files));
}

/** The readings kept for the document of `d`, made anew when it is not the document they were kept for. */
function keptFor(d) {
  const now = fingerprint(d);
  if (kept?.fingerprint !== now) {
    running?.controller.abort();
    kept = { fingerprint: now, inputs: new Map(), files: new Map() };
  }
  return kept;
}

/** Keep the problems of the reading `result` of one file, by the path of the file each is in. */
function keep(into, result, documentFiles) {
  // a package read with another text than a reading kept used: those kept are no longer of the same Library
  const used = (result.inputs?.files ?? []).filter((i) => i.hash && !i.narrow && !documentFiles.has(i.path));
  if (used.some((i) => into.inputs.has(i.path) && into.inputs.get(i.path) !== i.hash)) {
    into.inputs.clear();
    into.files.clear();
  }
  for (const i of used) into.inputs.set(i.path, i.hash);
  const own = result.files.findIndex((f) => f.path === result.file);
  into.files.set(result.file, result.diagnostics.filter((x) => ownProblem(result, x, own, documentFiles))
    .map((x) => ({ ...x, path: x.file_index === null || x.file_index === undefined ? null : result.files[x.file_index]?.path ?? null })));
}

/** Keep the problems of the Document shown, when it is the reading of one file. */
export function noteShown(d) {
  // another document, of one file or read whole: what Check All Files was reading is not needed
  if (readFileIndex(d) < 0) return stopChecking();
  const into = keptFor(d);
  // (the list is made again as the Document is shown)
  keep(into, d, new Set(d.outline.files.map((f) => f.path)));
}

/** A problem kept, as the Document `d` places it: by the index of its file among `d`'s files, and its name there. */
function placed(d, x) {
  if (x.path === null) return { ...x, file_index: null };
  const at = d.files.findIndex((f) => f.path === x.path);
  return at < 0 ? { ...x, file_index: null } : { ...x, file_index: at, file: d.files[at].name ?? x.file };
}

/**
 * The problems of the other files of the document of `d`, the reading of one file: each file's as its reading gave
 * them, or the outline's for a file not read yet; with how many of its files were read (`checked`, the file shown
 * among them) of how many (`total`), and how far Check All Files is (`checking`: `{ done, total }`, or null).
 */
export function otherFiles(d) {
  const own = readFileIndex(d);
  if (own < 0) return null;
  const into = keptFor(d);
  const list = [];
  const seen = new Set();
  const add = (x) => {
    const key = JSON.stringify([x.file_index === null ? x.full_path : x.file_index, x.line, x.col, x.code, x.message]);
    if (seen.has(key)) return;
    seen.add(key);
    list.push(x);
  };
  // those of the file shown are the list's own: they are not repeated
  for (const x of shownFileProblems(d, d.diagnostics)) seen.add(JSON.stringify([x.file_index === null ? x.full_path : x.file_index, x.line, x.col, x.code, x.message]));
  let checked = 0;
  for (const [k, f] of d.outline.files.entries()) {
    const problems = into.files.get(f.path);
    if (problems) checked++;
    if (f.path === d.file) continue;
    if (problems) for (const x of problems) add(placed(d, x));
    else for (const x of d.outline.diagnostics) if (x.file_index === k) add(placed(d, { ...x, path: f.path }));
  }
  return { list, checked, total: d.outline.files.length, checking: running && { done: running.done, total: running.total } };
}

/** Whether Check All Files has files to read for the document shown, and is not reading them already. */
export function canCheckAll() {
  if (!readsByFile() || running || readFileIndex(doc) < 0) return false;
  const into = keptFor(doc);
  return doc.outline.files.some((f) => !into.files.has(f.path));
}

/**
 * Check All Files: read each file of the document shown not read yet, alone in the context of its entry, with the edits
 * applied, and keep its problems, the list told of each as it comes. A file that cannot be read has that as its problem.
 */
export async function checkAllFiles() {
  const source = currentSource();
  if (!canCheckAll() || !source) return;
  const d = doc;
  const into = keptFor(d);
  const documentFiles = new Set(d.outline.files.map((f) => f.path));
  const files = d.outline.files.map((f) => f.path).filter((path) => !into.files.has(path));
  const controller = new AbortController();
  const run = (running = { controller, done: 0, total: files.length });
  changed();
  try {
    await checkFiles(source, changes(true), files, (file, result, error) => {
      if (kept !== into || controller.signal.aborted) return;
      run.done++;
      if (result) keep(into, result, documentFiles);
      else {
        into.files.set(file, [{ severity: "error", code: "", name: "", message: m`The file could not be checked: ${error}`,
          file, full_path: file, file_index: null, line: 0, col: 0, origin: "", path: file }]);
      }
      changed();
    }, controller.signal);
  } catch {
    // another document, or the same one changed, is shown: what was read is kept, the rest is not checked
  } finally {
    if (running === run) running = null;
    changed();
  }
}

/** Stop Check All Files, as another document is opened. */
export function stopChecking() {
  running?.controller.abort();
}
