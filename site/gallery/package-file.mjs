// Reads an RNL package file into what the gallery shows of it: the header record, the comments,
// and each statement with its kind and its place in the file. Not an interpreter: the statements are
// split at the `;` between the members of a record (4.1), outside strings, comments and brackets, and
// sorted by their first tokens. What a statement means, and how a symbol is drawn, comes from the core.

/** A position in the text: 1-based line of an offset. */
function lineAt(text, offset) {
  let line = 1;
  for (let i = text.indexOf("\n"); i >= 0 && i < offset; i = text.indexOf("\n", i + 1)) line++;
  return line;
}

/**
 * The top-level structure of a file: the records with their heads and the members of their bodies,
 * and the comments between and inside them. Offsets are into `text`.
 */
export function scan(text) {
  const records = [];
  const comments = [];
  let i = 0;
  const n = text.length;
  // skip a string from its opening quote; strings cannot span lines, and `""` is a quote (3.6)
  function string(j) {
    j++;
    while (j < n) {
      if (text[j] === '"') {
        if (text[j + 1] === '"') j += 2;
        else return j + 1;
      } else if (text[j] === "\n") return j;
      else j++;
    }
    return j;
  }
  // a comment is met once while a record's end is found and again while its body is split
  const seen = new Set();
  function comment(j) {
    const end = text.indexOf("*/", j + 2);
    const stop = end < 0 ? n : end + 2;
    if (!seen.has(j)) {
      seen.add(j);
      comments.push({ start: j, end: stop, text: text.slice(j, stop) });
    }
    return stop;
  }
  // the end of a bracket opened at j, with strings and comments inside it
  function bracket(j) {
    const close = { "(": ")", "[": "]", "{": "}" };
    const stack = [close[text[j]]];
    j++;
    while (j < n && stack.length) {
      const c = text[j];
      if (c === '"') j = string(j);
      else if (c === "/" && text[j + 1] === "*") j = comment(j);
      else if (close[c]) {
        stack.push(close[c]);
        j++;
      } else if (c === stack[stack.length - 1]) {
        stack.pop();
        j++;
      } else j++;
    }
    return j;
  }
  while (i < n) {
    const c = text[i];
    if (c === "/" && text[i + 1] === "*") {
      i = comment(i);
      continue;
    }
    if (c === "@" && text.startsWith("@RNL", i) && !/[\w-]/.test(text[i + 4] || "")) {
      const start = i;
      i += 4;
      let head = "";
      if (text[i] === "(") {
        const e = bracket(i);
        head = text.slice(i + 1, e - 1);
        i = e;
      }
      while (i < n && text[i] !== "{") {
        if (text[i] === "/" && text[i + 1] === "*") i = comment(i);
        else i++;
      }
      const open = i;
      const end = bracket(open);
      const record = { start, end, head, header: /\btype\s*=\s*"header"/.test(head), members: [] };
      members(record, open + 1, end - 1);
      records.push(record);
      i = end;
      continue;
    }
    i++;
  }
  // split a body at the `;` of its own level; each member with the comments right before it
  function members(record, from, to) {
    let j = from;
    let memberStart = null;
    let docs = [];
    const flush = (stop) => {
      if (memberStart !== null) {
        const raw = text.slice(memberStart, stop);
        const trimmed = raw.replace(/\s+$/, "");
        record.members.push({ start: memberStart, end: memberStart + trimmed.length, text: trimmed, docs });
      }
      memberStart = null;
      docs = [];
    };
    while (j < to) {
      const c = text[j];
      if (c === "/" && text[j + 1] === "*") {
        const start = j;
        j = comment(j);
        if (memberStart === null) docs.push(text.slice(start, j));
        continue;
      }
      if (/\s/.test(c)) {
        j++;
        continue;
      }
      if (c === ";") {
        flush(j);
        j++;
        continue;
      }
      if (memberStart === null) memberStart = j;
      if (c === '"') j = string(j);
      else if (c === "(" || c === "[" || c === "{") j = bracket(j);
      else j++;
    }
    flush(to);
  }
  return { records, comments };
}

/** The text of a comment without its markers and the indentation of its continuation lines. */
export function commentText(comment) {
  const body = comment.replace(/^\/\*+/, "").replace(/\*+\/$/, "");
  const lines = body.split("\n");
  const indent = Math.min(
    ...lines.slice(1).filter((l) => l.trim()).map((l) => /^\s*/.exec(l)[0].length),
    Infinity,
  );
  return lines
    .map((l, k) => (k === 0 ? l.trim() : l.slice(Number.isFinite(indent) ? indent : 0).trimEnd()))
    .join("\n")
    .trim();
}

/** The parameters of a keyword, as written: `name="x", version="0.2"` to {name: "x", version: "0.2"}. */
export function params(s) {
  const out = {};
  for (const item of splitTop(s, ",")) {
    const m = /^\s*([\p{L}_][\p{L}\p{N}_-]*)\s*=\s*([\s\S]*?)\s*$/u.exec(item);
    if (!m) continue;
    out[m[1]] = /^"[\s\S]*"$/.test(m[2]) ? unquote(m[2]) : m[2];
  }
  return out;
}

/** A string's text: without its quotes, `""` read as `"` (3.6). */
export function unquote(s) {
  return s.slice(1, -1).replace(/""/g, '"');
}

/** `s` split at each `sep` outside strings and brackets. */
function splitTop(s, sep) {
  const parts = [];
  let depth = 0;
  let from = 0;
  for (let j = 0; j < s.length; j++) {
    const c = s[j];
    if (c === '"') {
      j++;
      while (j < s.length && !(s[j] === '"' && s[j + 1] !== '"')) j += s[j] === '"' ? 2 : 1;
    } else if (c === "(" || c === "[" || c === "{") depth++;
    else if (c === ")" || c === "]" || c === "}") depth--;
    else if (c === sep && depth === 0) {
      parts.push(s.slice(from, j));
      from = j + 1;
    }
  }
  parts.push(s.slice(from));
  return parts;
}

/** The arguments of a keyword at the start of `s`, such as `@PACKAGE(...)`, or null. */
function argsOf(keyword, s) {
  const k = s.indexOf(keyword + "(");
  if (k < 0) return null;
  let depth = 0;
  for (let j = k + keyword.length; j < s.length; j++) {
    const c = s[j];
    if (c === '"') {
      j++;
      while (j < s.length && !(s[j] === '"' && s[j + 1] !== '"')) j += s[j] === '"' ? 2 : 1;
    } else if (c === "(") depth++;
    else if (c === ")" && --depth === 0) return s.slice(k + keyword.length + 1, j);
  }
  return null;
}

/** The body of a definition, `{…}` after its keyword and parameters, without the braces. */
function bodyOf(s) {
  const k = s.indexOf("{");
  if (k < 0) return "";
  const e = s.lastIndexOf("}");
  return e > k ? s.slice(k + 1, e) : "";
}

const NAME = String.raw`[$%][\p{L}\p{N}_]+`;
const PIN = /\(\s*(-?[\d.]+)\s*,\s*(-?[\d.]+)\s*\)\s*(?=@PIN\()/gu;

/** A terminal as written: a number, a name, or an array of them for a bus pin (9.2.4). */
function terminalOf(v) {
  if (v === undefined) return undefined;
  if (/^\[[\s\S]*\]$/.test(v)) return splitTop(v.slice(1, -1), ",").map((t) => terminalOf(t.trim()));
  if (/^\d+$/.test(v)) return Number(v);
  return /^"[\s\S]*"$/.test(v) ? unquote(v) : v;
}

/** The pins written in a symbol's body, without those of the parts it draws (`%MOS_CHANNEL;`). */
function ownPins(body) {
  const pins = [];
  let m;
  PIN.lastIndex = 0;
  while ((m = PIN.exec(body))) {
    const p = params(argsOf("@PIN", body.slice(m.index + m[0].length)) || "");
    pins.push({ at: [Number(m[1]), Number(m[2])], terminal: terminalOf(p.terminal), label: p.label, direction: p.direction });
  }
  return pins;
}

/** The texts a symbol shows with a key: `? @TEXT("${name}", key="name")`. */
function ownLabels(body) {
  const labels = [];
  for (let k = body.indexOf("@TEXT("); k >= 0; k = body.indexOf("@TEXT(", k + 1)) {
    const [text, ...rest] = splitTop(argsOf("@TEXT", body.slice(k)) || "", ",");
    const key = params(rest.join(",")).key;
    if (key && /^\s*"[\s\S]*"\s*$/.test(text)) labels.push({ key, text: unquote(text.trim()) });
  }
  return labels;
}

/** The members of a body that only name another symbol, which the symbol draws as a part (9.2). */
function partsOf(body) {
  const parts = [];
  const { records } = scan(`@RNL {${body}}`);
  for (const member of records[0]?.members || []) {
    const m = new RegExp(`^(${NAME})$`, "u").exec(member.text);
    if (m) parts.push(m[1]);
  }
  return parts;
}

/**
 * Sort a member of a record by its first tokens. The kinds:
 * symbol, style, definition, set (a `$` name given a set), rule (`# …` and `$SET …` pipes that
 * select and act), apply (a definition attached, such as a fill), other.
 */
export function classify(text) {
  let m;
  const flat = text.replace(/\/\*[\s\S]*?\*\//g, " ");
  if ((m = new RegExp(`^(${NAME})\\s*=\\s*@DEFINE-SYMBOL\\b`, "u").exec(flat))) {
    const p = params(argsOf("@DEFINE-SYMBOL", flat) || "");
    return { kind: "symbol", name: m[1], box: p.box ? JSON.parse(p.box) : null, body: bodyOf(flat) };
  }
  if ((m = new RegExp(`^(${NAME})\\s*=\\s*@DEFINE-STYLE\\b`, "u").exec(flat))) {
    const props = params(argsOf("@DEFINE-STYLE", flat) || "");
    const base = (bodyOf(flat).match(new RegExp(NAME, "gu")) || []);
    return { kind: "style", name: m[1], props, base };
  }
  if ((m = new RegExp(`^(${NAME})\\s*=\\s*@DEFINE\\b`, "u").exec(flat))) {
    const p = params(argsOf("@DEFINE", flat) || "");
    return { kind: "definition", name: m[1], roles: p.roles || null, parameters: p.parameters || null, body: bodyOf(flat).trim() };
  }
  if ((m = new RegExp(`^(${NAME})\\s*=\\s*([\\s\\S]*)$`, "u").exec(flat))) {
    return { kind: "set", name: m[1], value: m[2].trim() };
  }
  if (/=>/.test(flat) && /^(#|\$)/.test(flat)) {
    const steps = flat.split("=>").map((s) => s.trim());
    const last = steps[steps.length - 1];
    let action = "other";
    let target = null;
    if ((m = new RegExp(`^@SET-SYMBOL\\s+(${NAME})`, "u").exec(last))) [action, target] = ["symbol", m[1]];
    else if ((m = new RegExp(`^@SET-STYLE\\s+(${NAME})`, "u").exec(last))) [action, target] = ["style", m[1]];
    else if ((m = new RegExp(`^@(REMOVE-FROM|ADD-TO)\\s+(${NAME})`, "u").exec(last))) [action, target] = [m[1].toLowerCase(), m[2]];
    else if (/^@TEXT\b/.test(last)) action = "text";
    return { kind: "rule", select: steps.slice(0, -1).join(" => "), action, target, last };
  }
  if ((m = new RegExp(`^(${NAME})\\s+@ATTACH\\s+(${NAME})`, "u").exec(flat))) {
    return { kind: "apply", name: m[1], target: m[2] };
  }
  if ((m = new RegExp(`^(${NAME})\\s+@(REMOVE-FROM|ADD-TO)\\s+(${NAME})`, "u").exec(flat))) {
    return { kind: "rule", select: m[1], action: m[2].toLowerCase(), target: m[3], last: flat.slice(m[1].length).trim() };
  }
  return { kind: "other" };
}

/**
 * Read a package file. Returns its text with `\n` line ends, its header (language version, package
 * name and version, the packages it references, its aliases), its description (the comment before
 * the header record), and its statements in order, each with its line span, the comments right
 * before it and what `classify` makes of it. Symbols also get the pins, labels and parts their
 * bodies write; `resolve` adds those of their parts.
 */
export function readPackage(text) {
  text = text.replace(/\r\n/g, "\n");
  const { records, comments } = scan(text);
  const header = { language: null, name: null, version: null, references: [], aliases: [], other: [] };
  const statements = [];
  const headerRecord = records.find((r) => r.header);
  for (const member of headerRecord?.members || []) {
    let a;
    if ((a = argsOf("@VERSION", member.text)) !== null) header.language = a.trim().replace(/^"|"$/g, "");
    else if ((a = argsOf("@PACKAGE", member.text)) !== null) Object.assign(header, (({ name, version }) => ({ name, version }))(params(a)));
    else if ((a = argsOf("@USE-PACKAGE", member.text)) !== null) header.references.push(params(a));
    else if ((a = argsOf("@ALIAS", member.text)) !== null) {
      const pair = splitTop(a, ",").map((x) => x.trim());
      if (pair.length === 2 && pair.every((x) => /^"[\s\S]*"$/.test(x))) header.aliases.push(pair.map(unquote));
    } else header.other.push(member.text);
  }
  const first = comments[0];
  const description = first && (!headerRecord || first.end <= headerRecord.start) ? commentText(first.text) : "";
  for (const record of records) {
    if (record.header) continue;
    for (const member of record.members) {
      const info = classify(member.text);
      const s = {
        ...info,
        line: lineAt(text, member.start),
        endLine: lineAt(text, member.end),
        text: member.text,
        docs: member.docs.map(commentText),
      };
      if (s.kind === "symbol") {
        s.pins = ownPins(s.body);
        s.labels = ownLabels(s.body);
        s.parts = partsOf(s.body);
        delete s.body;
      }
      statements.push(s);
    }
  }
  return { text, header, description, statements };
}

/**
 * The pins and labels of each symbol with those of the parts it draws, looked up by name among
 * `symbols` (a Map of every symbol known to the gallery), parts first as they are drawn first.
 */
export function resolve(symbol, symbols, seen = new Set()) {
  if (seen.has(symbol.name)) return { pins: [], labels: [] };
  seen.add(symbol.name);
  const pins = [];
  const labels = [];
  for (const part of symbol.parts || []) {
    const p = symbols.get(part);
    if (!p) continue;
    const r = resolve(p, symbols, seen);
    pins.push(...r.pins);
    labels.push(...r.labels);
  }
  pins.push(...symbol.pins);
  for (const l of symbol.labels) {
    const k = labels.findIndex((x) => x.key === l.key);
    if (k >= 0) labels.splice(k, 1);
    labels.push(l);
  }
  return { pins, labels };
}

/**
 * The names of a symbol's terminals, when the comment right before it starts with them, in terminal
 * order and those without a pin in brackets, as the generated packages write them:
 * `a b y (vp) (vn); from Analog Canvas and-gate`. A symbol that draws the usages of a definition says so first,
 * `draws $SPDT_SWITCH; com a b; from Analog Canvas simple-spdt-switch`: that part is skipped. Null when the comment
 * does not start with the names.
 */
export function terminalNames(docs) {
  const doc = docs[docs.length - 1]?.replace(/^\s*draws\s+\$[\w-]+\s*;/, "");
  if (!doc || !doc.includes(";")) return null;
  const head = doc.slice(0, doc.indexOf(";")).trim();
  if (!/^\(?[A-Za-z][\w+-]*\)?(?:\s+\(?[A-Za-z][\w+-]*\)?)*$/.test(head)) return null;
  return head.split(/\s+/).map((t) => t.replace(/^\(|\)$/g, ""));
}
