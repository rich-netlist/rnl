// The keys of a text box of Edit Text (editor.js) that do more than a plain text box does: Enter in a
// comment begins the next line as a comment, with the same marker and the same space after it, and
// Tab and Shift+Tab indent and outdent instead of moving the focus. Esc, then Tab, leaves the text box,
// so the keyboard still has a way out of it. These are functions of the text, the selection and the
// key alone: what they give is the edit to make, which editor.js makes as typing would, so that it can
// be undone.

/** The spaces Tab puts in, and Shift+Tab takes out. */
export const INDENT = "  ";

/** The markers that begin a comment line in a file of `format`: `*` in SPICE, `//` and `*` in Spectre; none in a package or plain text. */
export function commentMarkers(format) {
  if (format === "spectre") return ["//", "*"];
  if (format === "rnl" || format === "text") return [];
  return ["*"];
}

/**
 * The comment that the line `line` is, in a file of `format`: `marker`, the index just after its marker,
 * and `prefix`, what begins the line up to its text (the space before the marker, the marker and the
 * space after it); null for a line that is no comment.
 */
export function commentOf(line, format) {
  const before = /^[ \t]*/.exec(line)[0];
  for (const marker of commentMarkers(format)) {
    if (!line.startsWith(marker, before.length)) continue;
    const end = before.length + marker.length;
    const after = /^[ \t]*/.exec(line.slice(end))[0];
    return { marker: end, prefix: line.slice(0, end) + after };
  }
  return null;
}

const lineStart = (text, at) => text.lastIndexOf("\n", at - 1) + 1;
const lineEnd = (text, at) => {
  const k = text.indexOf("\n", at);
  return k < 0 ? text.length : k;
};

/**
 * What a key does in a text box holding `text`, with the selection `start`..`end`, in a file of `format`:
 * `{ from, to, insert, start, end }`, the text from `from` to `to` replaced by `insert` and the selection
 * that follows; `{ none: true }` when the key is taken but changes nothing, as Shift+Tab with no indent
 * to take out, which keeps the focus; null when the key does what it does in any text box. `key` is the KeyboardEvent's, and
 * `shift` and `other` (Ctrl, Alt or Meta held) its modifiers; `escaped` is true when the key before was
 * Esc, after which Tab moves the focus as usual.
 */
export function editKey({ text, start, end, key, shift = false, other = false, escaped = false, format }) {
  if (other) return null;
  if (key === "Enter") return shift ? null : continueComment(text, start, end, format);
  if (key === "Tab" && !escaped) return shift || text.slice(start, end).includes("\n") ? indentLines(text, start, end, shift, format) : insertIndent(start, end);
  return null;
}

/** Enter in a comment, after its marker: the new line begins as the comment did. Before the marker, or in no comment, a plain Enter. */
function continueComment(text, start, end, format) {
  const from = lineStart(text, start);
  const comment = commentOf(text.slice(from, lineEnd(text, from)), format);
  if (!comment || start - from < comment.marker) return null;
  const insert = `\n${comment.prefix}`;
  return { from: start, to: end, insert, start: start + insert.length, end: start + insert.length };
}

/** Tab with nothing selected, or a selection within a line: the spaces of an indent in its place. */
function insertIndent(start, end) {
  return { from: start, to: end, insert: INDENT, start: start + INDENT.length, end: start + INDENT.length };
}

/**
 * Tab over several lines, and Shift+Tab: each line of the selection indented, or outdented, by the
 * spaces of an indent at most. A comment's indent is the space after its marker, of which an outdent
 * leaves one; any other line's is the space it begins with. An empty line is left as it is. With
 * nothing to indent or outdent, `{ none: true }`: the key changes nothing, and the focus stays.
 */
function indentLines(text, start, end, outdent, format) {
  const from = lineStart(text, start);
  // a selection that ends at the beginning of a line leaves that line out
  const last = end > start && lineStart(text, end) === end ? end - 1 : end;
  const to = lineEnd(text, last);
  const edits = []; // { at, remove, insert }, in the text, in order
  let at = from;
  for (const line of text.slice(from, to).split("\n")) {
    const comment = commentOf(line, format);
    const indent = comment ? comment.marker : 0;
    const spaces = /^ */.exec(line.slice(indent))[0].length;
    if (outdent) {
      const remove = Math.min(INDENT.length, comment ? Math.max(0, spaces - 1) : spaces);
      if (remove) edits.push({ at: at + indent + (comment ? 1 : 0), remove, insert: "" });
    } else if (line.trim()) {
      edits.push({ at: at + indent, remove: 0, insert: INDENT });
    }
    at += line.length + 1;
  }
  if (!edits.length) return { none: true };
  let insert = "";
  let k = from;
  for (const e of edits) {
    insert += text.slice(k, e.at) + e.insert;
    k = e.at + e.remove;
  }
  insert += text.slice(k, to);
  // a place in the text moves by what was put in or taken out before it; one in what was taken out goes to where it was
  const move = (pos) => {
    let delta = 0;
    for (const e of edits) {
      if (pos >= e.at + e.remove) delta += e.insert.length - e.remove;
      else if (pos > e.at) return e.at + delta;
    }
    return pos + delta;
  };
  return { from, to, insert, start: move(start), end: move(end) };
}
