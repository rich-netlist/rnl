// The text the user edits (decision 0011): per file of the document, what the file was when it was
// read, what the editor holds now, and what the Document shown was built from. The editor
// (editor.js) changes the text; a change is not applied to the Document until the user says so, and
// not saved to the file until the user says that. Text is kept with its line ends as `\n`, as a text
// box gives it, and written with the line ends the file had.

/** @type {Map<string, { name: string, original: string, text: string, applied: string, eol: string }>} */
const buffers = new Map(); // by the file's path in the Document

/** A text with its line ends as `\n`. */
const lf = (text) => text.replace(/\r\n?/g, "\n");

/** The line end a text has most of: `\r\n` when it has more of them than bare `\n`, else `\n`. */
function eolOf(text) {
  const crlf = (text.match(/\r\n/g) || []).length;
  const bare = (text.match(/\n/g) || []).length - crlf;
  return crlf > bare ? "\r\n" : "\n";
}

/** A buffer for a file of a Document, whose text is as it was read. */
function add(file) {
  const text = lf(file.text);
  buffers.set(file.path, { name: file.name, original: text, text, applied: text, eol: eolOf(file.text) });
}

/** The text of a buffer as a file has it: with the line ends the file had. */
const written = (b, text) => (b.eol === "\n" ? text : text.replace(/\n/g, b.eol));

/** A Document built from the files as they are on disk: nothing is edited. */
export function resetBuffers(doc) {
  buffers.clear();
  if (doc) for (const file of doc.files) add(file);
}

/**
 * A Document built from the files with the edits applied: its texts are what is applied now. A file
 * that it has and the buffers have not, such as one an edit made the netlist include, is as it was read.
 */
export function adoptBuffers(doc) {
  for (const file of doc.files) {
    const b = buffers.get(file.path);
    if (b) b.applied = lf(file.text);
    else add(file);
  }
}

/** Whether there is a buffer for the file. */
export const hasBuffer = (path) => buffers.has(path);

/** The text the editor holds for a file. */
export const textOf = (path) => (buffers.get(path) ? buffers.get(path).text : "");

/** Set the text of a file, which the editor changed; false when there is no such file. */
export function setText(path, text) {
  const b = buffers.get(path);
  if (b) b.text = lf(text);
  return b !== undefined;
}

/** Put a file's text back as the Document has it. */
export function discard(path) {
  const b = buffers.get(path);
  if (b) b.text = b.applied;
}

/** The files whose text differs from what was read, and so is not saved: `{ path, name, text }`, the text as the file has it. */
export function dirty() {
  const out = [];
  for (const [path, b] of buffers) if (b.text !== b.original) out.push({ path, name: b.name, text: written(b, b.text) });
  return out;
}

/** The paths of the files whose text differs from what was read. */
export function dirtyPaths() {
  return [...buffers].filter(([, b]) => b.text !== b.original).map(([path]) => path);
}

/** The paths of the files whose text is not applied to the Document yet. */
export function unappliedPaths() {
  return [...buffers].filter(([, b]) => b.text !== b.applied).map(([path]) => path);
}

/**
 * The texts that differ from what was read, by path, for a shell to build or solve over the files:
 * those the editor holds now (`applied` false), or those applied to the Document shown.
 */
export function changes(applied) {
  const out = {};
  for (const [path, b] of buffers) {
    const text = applied ? b.applied : b.text;
    if (text !== b.original) out[path] = written(b, text);
  }
  return out;
}

/** Whether a file's text differs from what was read. */
export function isDirty(path) {
  const b = buffers.get(path);
  return b !== undefined && b.text !== b.original;
}

/** What was written to disk, or downloaded, is what was read from now on. */
export function markSaved(paths) {
  for (const path of paths) {
    const b = buffers.get(path);
    if (b) b.original = b.text;
  }
}

/** The text of a file as a file has it, for a download. */
export function fileText(path) {
  const b = buffers.get(path);
  return b ? written(b, b.text) : "";
}
