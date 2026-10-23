// Syntax highlighting of the netlist text: the host statements outside records, and the RNL
// records. Each function marks a class per character of a line.

const KEYWORD_CHAR = /[\p{L}\p{N}_-]/u;
const NAME_CHAR = /[\p{L}\p{N}_.\-/]/u;
const REF_CLASS = { "#": "host", "&": "net", "$": "def", "%": "local", "?": "param" };
const SPECTRE_WORDS = new Set([
  "simulator", "subckt", "ends", "inline", "model", "include", "ahdl_include", "parameters", "global",
  "section", "endsection", "library", "endlibrary", "statistics", "real",
]);

export function mark(cls, from, to, name) {
  for (let k = from; k < to && k < cls.length; k++) cls[k] = name;
}

/**
 * The end of the characters from `i`, before `to`, that `re` matches: whole characters, as a
 * character outside the BMP, such as an emoji, takes two of JavaScript's UTF-16 units.
 */
function run(text, i, to, re) {
  while (i < to) {
    const c = String.fromCodePoint(text.codePointAt(i));
    if (!re.test(c)) break;
    i += c.length;
  }
  return i;
}

/**
 * Highlight a host line outside records: comments, the first word of a statement, parameter
 * names. Returns whether the line continues on the next (a Spectre `\`).
 */
export function hostLine(text, cls, cont, records) {
  const trimmed = text.trimStart();
  const lead = text.length - trimmed.length;
  if (trimmed.startsWith("*") || trimmed.startsWith("//")) {
    // a comment line; a carrier's records are highlighted over it
    mark(cls, 0, text.length, records ? "marker" : "comment");
    return false;
  }
  let end = text.length;
  let quoted = null;
  for (let k = 0; k < text.length; k++) {
    const c = text[k];
    if (quoted) {
      if (c === quoted) { if (text[k + 1] === quoted) k++; else quoted = null; }
      continue;
    }
    if (c === '"' || c === "'") { quoted = c; continue; }
    if (c === ";" || (c === "$" && (k === 0 || /\s/.test(text[k - 1]))) || (c === "/" && text[k + 1] === "/")) {
      end = k;
      break;
    }
  }
  mark(cls, end, text.length, "comment");
  let k = lead;
  let first = !cont;
  if (cont && text[k] === "+") {
    mark(cls, k, k + 1, "punct");
    k++;
  }
  while (k < end) {
    if (/[\s(),\\]/.test(text[k])) {
      k++;
      continue;
    }
    const start = k;
    while (k < end && !/[\s(),\\]/.test(text[k])) k++;
    const word = text.slice(start, k);
    const eq = word.indexOf("=");
    if (first) {
      const directive = word.startsWith(".") || SPECTRE_WORDS.has(word.toLowerCase());
      mark(cls, start, k, directive ? "directive" : "inst");
      first = false;
    } else if (eq > 0) {
      mark(cls, start, start + eq, "param");
    }
  }
  // Host quoted expressions and filenames use the same string colour as RNL strings.
  for (let at = lead; at < end; at++) if (text[at] === '"' || text[at] === "'") {
    const quote = text[at];
    let to = at + 1;
    while (to < end) {
      if (text[to++] === quote) { if (text[to] === quote) to++; else break; }
    }
    mark(cls, at, to, "string");
    at = to - 1;
  }
  return text.slice(0, end).trimEnd().endsWith("\\");
}

/** Highlight a record range, retaining comments and delimiter context across physical lines. */
export function rnlRange(text, from, to, first, cls, state) {
  let i = from;
  // Newlines/carrier markers break the adjacency required for role parentheses. An already
  // open role, point or subscript can still continue onto the next physical line.
  let after = "";
  state.stack ||= "";
  mark(cls, from, to, "");
  if (!first) {
    // the carrier's comment marker on a later line of the record
    while (i < to && /\s/.test(text[i])) i++;
    if (text.startsWith("//", i)) {
      mark(cls, i, i + 2, "marker");
      i += 2;
    } else if (text[i] === "*") {
      mark(cls, i, i + 1, "marker");
      i++;
    }
  }
  while (i < to) {
    if (state.comment) {
      const e = text.indexOf("*/", i);
      const end = e < 0 || e + 2 > to ? to : e + 2;
      mark(cls, i, end, "comment");
      if (e >= 0 && e + 2 <= to) state.comment = false;
      i = end;
      continue;
    }
    const c = text[i];
    const previous = after;
    after = "";
    if (text.startsWith("/*", i)) {
      state.comment = true;
      mark(cls, i, i + 2, "comment");
      i += 2;
      continue;
    }
    if (c === '"') {
      let j = i + 1;
      while (j < to) {
        if (text[j] === '"') {
          if (text[j + 1] === '"') {
            j += 2;
            continue;
          }
          j++;
          break;
        }
        j++;
      }
      mark(cls, i, j, "string");
      i = j;
      continue;
    }
    if (c === "@") {
      const j = run(text, i + 1, to, KEYWORD_CHAR);
      mark(cls, i, j, "keyword");
      after = "keyword";
      i = j;
      continue;
    }
    if (c in REF_CLASS) {
      let j = i + 1;
      if (text[j] === '"') {
        j++;
        while (j < to) {
          if (text[j++] === '"') { if (text[j] === '"') j++; else break; }
        }
      } else {
        j = run(text, j, to, NAME_CHAR);
      }
      mark(cls, i, j, REF_CLASS[c]);
      // A terminal name belongs to the reference, not to the surrounding free prose.
      if (text[j] === ":") {
        mark(cls, j, j + 1, "punct");
        const terminal = ++j;
        if (text[j] === '"') {
          j++;
          while (j < to) {
            if (text[j++] === '"') { if (text[j] === '"') j++; else break; }
          }
        } else j = run(text, j, to, NAME_CHAR);
        mark(cls, terminal, j, "net");
      }
      after = "object";
      i = j;
      continue;
    }
    // Closing a reference's subscript resumes that reference; :terminal / ::sub-object
    // can follow it, and role parentheses must touch the final object character.
    if (c === ":" && previous === "object") {
      const start = i + (text[i + 1] === ":" ? 2 : 1);
      let j = start;
      if (text[j] === '"') {
        j++;
        while (j < to) {
          if (text[j++] === '"') { if (text[j] === '"') j++; else break; }
        }
      } else j = run(text, j, to, NAME_CHAR);
      if (j > start) {
        mark(cls, i, start, "punct");
        mark(cls, start, j, "net");
        after = "object";
        i = j;
        continue;
      }
    }
    if (c === "(") state.stack += previous === "object" ? "r" : previous === "keyword" ? "k" : "p";
    else if (c === "{") state.stack += "b";
    else if (c === "[" || c === "<") state.stack += previous === "object" ? "s" : "a";
    else if (")}]>".includes(c) && state.stack) {
      const kind = state.stack.at(-1);
      if ((c === ")" && "rkp".includes(kind)) || (c === "}" && kind === "b") ||
          ((c === "]" || c === ">") && "sa".includes(kind))) {
        state.stack = state.stack.slice(0, -1);
        if ("pbs".includes(kind)) after = "object";
      }
    }
    if (/[0-9]/.test(c) || ((c === "-" || c === ".") && /[0-9]/.test(text[i + 1] || ""))) {
      let j = i + 1;
      while (j < to && /[0-9.a-zA-Z]/.test(text[j])) j++;
      mark(cls, i, j, "number");
      i = j;
      continue;
    }
    if (text.startsWith("=>", i)) {
      mark(cls, i, i + 2, "punct");
      i += 2;
      continue;
    }
    const j = run(text, i, to, KEYWORD_CHAR);
    if (j > i) {
      let k = j;
      while (k < to && text[k] === " ") k++;
      mark(cls, i, j, state.stack.endsWith("r") || (text[k] === "=" && text[k + 1] !== ">") ? "param" : "free");
      i = j;
      continue;
    }
    if ("{}()[];,=:<>".includes(c)) mark(cls, i, i + 1, "punct");
    i++;
  }
}
