// The text the user edits (decision 0011): per file of the document, what the file was when it was
// read, the last saved snapshot, what the editor holds now, and what the Document was built from. The editor
// (editor.js) changes the text; a change is not applied to the Document until the user says so, and
// not saved to the file until the user says that. Text is kept with its line ends as `\n`, as a text
// box gives it, and written with the line ends the file had: each line the edit leaves as it was with its own (found by
// a diff of the lines), a line written in the place of one with that one's, and a line added with the one the file has
// most of.

/**
 * @type {Map<string, { name: string, original: string, base: string, text: string, applied: string, eol: string,
 *   ends: string[] }>}
 */
const buffers = new Map(); // by the file's path in the Document

/** A text with its line ends as `\n`. */
const lf = (text) => text.replace(/\r\n?/g, "\n");

/** The line end a text has most of: `\r\n` when it has more of them than bare `\n`, else `\n`. */
function eolOf(text) {
  const crlf = (text.match(/\r\n/g) || []).length;
  const bare = (text.match(/\n/g) || []).length - crlf;
  return crlf > bare ? "\r\n" : "\n";
}

/** The line ends of a text, in their order: `\r\n`, `\n` or `\r`, one for each line but the last. */
const endsOf = (text) => text.match(/\r\n|\r|\n/g) ?? [];

/** A buffer for a file of a Document, whose text is as it was read. */
function add(file) {
  const text = lf(file.text);
  buffers.set(file.path, { name: file.name, original: text, base: text, text, applied: text, eol: eolOf(file.text), ends: endsOf(file.text) });
}

/** The most lines a diff of the lines changes (`common`); past it, the lines between the first and last change pair up in order. */
const MOST_CHANGES = 2000;

/**
 * The lines `a` and `b` have in common, in their order, as pairs of their indexes: a longest such run (Myers' diff),
 * or none when it takes more than MOST_CHANGES lines added or taken out.
 * @param {string[]} a
 * @param {string[]} b
 * @returns {[number, number][]}
 */
function common(a, b) {
  const n = a.length, m = b.length, max = n + m;
  if (!n || !m) return [];
  const v = new Int32Array(2 * max + 3); // by diagonal k = x - y, at k + max + 1: the furthest x reached on it
  const at = (k) => k + max + 1;
  const trace = []; // before each round d, v on the diagonals -d..d, at k + d
  for (let d = 0; d <= Math.min(max, MOST_CHANGES); d++) {
    trace.push(v.slice(at(-d), at(d) + 1));
    for (let k = -d; k <= d; k += 2) {
      let x = k === -d || (k !== d && v[at(k - 1)] < v[at(k + 1)]) ? v[at(k + 1)] : v[at(k - 1)] + 1;
      let y = x - k;
      while (x < n && y < m && a[x] === b[y]) { x++; y++; }
      v[at(k)] = x;
      if (x >= n && y >= m) return path(trace, n, m);
    }
  }
  return [];
}

/** The pairs of lines in common on the path a diff found, going back from its end. */
function path(trace, n, m) {
  const pairs = [];
  let x = n, y = m;
  for (let d = trace.length - 1; d >= 0; d--) {
    if (d === 0) {
      while (x > 0 && y > 0) pairs.push([--x, --y]);
      break;
    }
    const was = trace[d], get = (k) => was[k + d];
    const k = x - y;
    const down = k === -d || (k !== d && get(k - 1) < get(k + 1));
    const pk = down ? k + 1 : k - 1;
    const px = get(pk), py = px - pk;
    const sx = down ? px : px + 1, sy = sx - k;
    while (x > sx && y > sy) pairs.push([--x, --y]);
    x = px;
    y = py;
  }
  return pairs.reverse();
}

/** For each line of `now`, the line of `was` it is when the edit left it as it was, else -1. */
function sameLines(now, was) {
  const same = new Array(now.length).fill(-1);
  let head = 0;
  while (head < now.length && head < was.length && now[head] === was[head]) {
    same[head] = head;
    head++;
  }
  let tail = 0;
  while (tail < now.length - head && tail < was.length - head && now[now.length - 1 - tail] === was[was.length - 1 - tail]) {
    same[now.length - 1 - tail] = was.length - 1 - tail;
    tail++;
  }
  for (const [i, j] of common(now.slice(head, now.length - tail), was.slice(head, was.length - tail))) same[head + i] = head + j;
  return same;
}

/**
 * The text of a buffer as a file has it: with the line ends the file had (`ends`, those of `original`). A line the
 * edit left as it was keeps its own, wherever it is; between two of them, the lines written have the ends of the lines
 * there before, the first the first's and so on, and those past them the end the file has most of (`eol`).
 */
function written(b, text) {
  if (b.ends.every((end) => end === b.eol)) return b.eol === "\n" ? text : text.replace(/\n/g, b.eol);
  const lines = text.split("\n");
  const was = b.original.split("\n");
  const same = sameLines(lines, was);
  // the line of `was` after the next line left as it was, for each line
  const until = new Array(lines.length + 1);
  until[lines.length] = was.length;
  for (let k = lines.length - 1; k >= 0; k--) until[k] = same[k] >= 0 ? same[k] : until[k + 1];
  let out = "";
  let next = 0; // the line of `was` the next line written in the place of one replaces
  for (let k = 0; k < lines.length; k++) {
    out += lines[k];
    let from = same[k];
    if (from >= 0) next = from + 1;
    else from = next < until[k] ? next++ : -1;
    if (k === lines.length - 1) break;
    out += b.ends[from] ?? b.eol;
  }
  return out;
}

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

/** A temporary source now reads applied in-memory text, without changing its saved snapshot. */
export function rebaseSource(path, text) {
  const b = buffers.get(path);
  if (b) b.base = lf(text);
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

/**
 * The files whose text differs from what was read, and so is not saved: `{ path, name, text, read }`,
 * the text as the file has it, and the text as it was read, with `\n` line ends, which a shell
 * that writes the file can look for on disk.
 */
export function dirty() {
  const out = [];
  for (const [path, b] of buffers) {
    if (b.text !== b.original) out.push({ path, name: b.name, text: written(b, b.text), read: b.original });
  }
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
 * The texts that differ from the underlying source, by path, for a shell to build or solve over it:
 * those the editor holds now (`applied` false), or those applied to the Document shown.
 */
export function changes(applied) {
  const out = {};
  for (const [path, b] of buffers) {
    const text = applied ? b.applied : b.text;
    if (text !== b.base) out[path] = written(b, text);
  }
  return out;
}

/** Whether a file's text differs from what was read. */
export function isDirty(path) {
  const b = buffers.get(path);
  return b !== undefined && b.text !== b.original;
}

/**
 * Record the snapshot actually saved, not later typing during an asynchronous save. A browser
 * download clears the unsaved mark but does not replace the File/URL the next build will read.
 */
export function markSaved(files, writtenToSource) {
  for (const file of files) {
    const b = buffers.get(file.path);
    if (b) {
      // (the text as the file has it now: its line ends are those of the file)
      b.original = lf(file.text);
      b.ends = endsOf(file.text);
      if (writtenToSource) b.base = b.original;
    }
  }
}

/** The text of a file as a file has it, for a download. */
export function fileText(path) {
  const b = buffers.get(path);
  return b ? written(b, b.text) : "";
}
