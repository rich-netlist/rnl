// Prepare line data and lexical checkpoints without constructing offscreen DOM. Syntax state is
// retained across continued host statements and multi-line RNL comments, so any row can render alone.
import { hostLine, mark, rnlRange } from "./syntax.js";
import { m, setAttr } from "./i18n.js";

export function lineModel(file, f, allSpans, dialect) {
  const texts = file.text.split(/\r?\n/);
  if (texts.length > 1 && texts.at(-1) === "") texts.pop();
  const starts = new Map();
  const unit = (n, c) => {
    if (!starts.has(n)) starts.set(n, characterStarts(texts[n - 1]));
    return starts.get(n)(c);
  };
  const rnl = new Map(), spans = new Map(), failed = new Map();
  const semantics = new Set((file.semantics || []).map(n => n - 1));
  const includes = new Map();
  for (const link of file.includes || []) {
    const location = includeLocation(texts, link.line - 1);
    if (location) includes.set(location.line, {...location, file: link.file});
  }
  const add = (map, i, value) => { if (!map.has(i)) map.set(i, []); map.get(i).push(value); };
  for (const [l0, c0, l1, c1] of file.records) {
    for (let n = l0; n <= l1 && n <= texts.length; n++) {
      add(rnl, n - 1, { from: n === l0 ? unit(n, c0 - 1) : 0,
        to: n === l1 ? unit(n, c1) : texts[n - 1].length, first: n === l0 });
    }
  }
  for (const span of allSpans) if (span.file === f && texts[span.line - 1] !== undefined) add(spans, span.line - 1, span);
  for (const [l0, c0, l1, c1, overridden] of file.failed) {
    for (let n = l0; n <= l1 && n <= texts.length; n++) {
      add(failed, n - 1, { from: n === l0 ? unit(n, c0 - 1) : leadingCarrier(texts[n - 1]),
        to: n === l1 ? unit(n, c1) : texts[n - 1].length, kind: overridden ? "overridden" : "failed" });
    }
  }
  function syntax(i, flags, stack = "") {
    const text = texts[i], cls = new Array(text.length).fill("");
    const state = { comment: !!(flags & 2), stack };
    let joined = false;
    if (i === 0 && f === 0 && dialect === "spice") mark(cls, 0, text.length, "comment");
    else if (rnl.has(i)) {
      hostLine(text, cls, false, rnl.get(i));
      for (const r of rnl.get(i)) {
        if (r.first) { state.comment = false; state.stack = ""; }
        rnlRange(text, r.from, Math.min(r.to, text.length), r.first, cls, state);
      }
    } else joined = hostLine(text, cls, !!(flags & 1) || /^\s*\+/.test(text), null);
    return { cls, flags: (joined ? 1 : 0) | (state.comment ? 2 : 0), stack: state.stack };
  }
  const states = new Uint8Array(texts.length);
  const stacks = new Map();
  const folds = [], blocks = [];
  let flags = 0, stack = "", longest = "", columns = 0;
  for (let i = 0; i < texts.length; i++) {
    states[i] = flags;
    if (stack) stacks.set(i, stack);
    const lex = syntax(i, flags, stack);
    flags = lex.flags;
    stack = lex.stack;
    const text = texts[i];
    // Also mark definitions in library sections that the host did not select. Reuse lexical
    // state so comments, RNL records and continuation lines cannot open/close a block.
    const head = /^\s*(\.?subckt|inline\s+subckt|\.?ends)\b/i.exec(text);
    if (head && lex.cls[text.search(/\S/)] === 'directive') {
      if (/ends$/i.test(head[1])) {
        const from = blocks.pop();
        if (from !== undefined) folds.push([from, i]);
      } else blocks.push(i);
    }
    // A conservative monospace width estimate, refined from rendered rows by LineWindow.
    let width = 0;
    for (const c of text) width = c === "\t" ? width + 8 - width % 8 : width + (c.codePointAt(0) >= 0x1100 ? 2 : 1);
    if (width > columns) { columns = width; longest = text; }
  }
  for (const from of blocks) folds.push([from, texts.length - 1]);
  return {
    texts, columns, longest, folds, semantics, rnl, failed, text: file.text.replace(/\r\n/g, "\n"),
    render(i) {
      const text = texts[i], { cls } = syntax(i, states[i], stacks.get(i));
      const fail = new Array(text.length).fill("");
      for (const r of failed.get(i) || []) {
        for (let c = r.from; c < r.to && c < text.length; c++) if (fail[c] !== "failed") fail[c] = r.kind;
      }
      const objs = new Array(text.length).fill(null);
      for (const s of (spans.get(i) || []).slice().sort((a, b) => a.len - b.len)) {
        const from = unit(i + 1, s.col - 1), to = unit(i + 1, s.col - 1 + s.len);
        if (s.kind === "node" || s.kind === "port") mark(cls, from, to, "net");
        for (let c = from; c < to; c++) {
          if (!objs[c]) objs[c] = [];
          if (!objs[c].includes(s.object)) objs[c].push(s.object);
        }
      }
      const el = lineElement(text, i + 1, cls, objs.map(o => o ? o.join(" ") : ""), fail);
      el.classList.toggle("rnl-line", rnl.has(i));
      el.classList.toggle("semantic-rnl", semantics.has(i));
      el.classList.toggle("inactive-line", failed.has(i));
      if (includes.has(i)) includeLink(el, includes.get(i));
      return el;
    },
  };
}

// Resolution comes from the host parser. Only locate the written filename here, not its target.
function includeLocation(texts, line) {
  const head = /^\s*(?:\.inc\S*|\.lib|include)\b\s*/i.exec(texts[line] || '');
  if (!head) return null;
  const spectre = /^\s*include\b/i.test(head[0]);
  let from = head[0].length;
  while (line < texts.length) {
    const tail = texts[line].slice(from);
    const token = spectre && tail.startsWith('//') ? null : /^("[^"]+"|'[^']+'|[^\s]+)/.exec(tail);
    if (token && token[0] !== '\\') return {line, from, to:from+token[0].length};
    // Spectre backslash or SPICE/Spectre '+' continuation: keep the filename's physical line.
    if (token?.[0] !== '\\' && !/^\s*\+/.test(texts[line+1] || '')) return null;
    line++;
    from = /^\s*\+?\s*/.exec(texts[line] || '')[0].length;
  }
  return null;
}

function includeLink(row, {file, from, to}) {
  const code = row.querySelector('.code');
  const point = offset => {
    const walker = document.createTreeWalker(code, NodeFilter.SHOW_TEXT);
    let node;
    while ((node = walker.nextNode())) {
      if (offset <= node.length) return [node, offset];
      offset -= node.length;
    }
    return [code, code.childNodes.length];
  };
  const range = document.createRange();
  range.setStart(...point(from)); range.setEnd(...point(to));
  const link = document.createElement('a');
  link.className = 'include-link'; link.dataset.include = String(file);
  link.href = `#source-${file}`;
  setAttr(link, 'title', m('Go to included file (Shift: new window)'));
  link.append(range.extractContents()); range.insertNode(link);
}

/**
 * Where each character of a line starts in its text, by 0-based column: JavaScript indexes a string
 * in UTF-16 units, two for a character outside the BMP, such as an emoji. A column past the last
 * character is the text's end.
 */
function characterStarts(text) {
  const at = [];
  for (let u = 0; u < text.length; u += text.codePointAt(u) > 0xffff ? 2 : 1) at.push(u);
  if (at.length === text.length) return (c) => Math.min(c, text.length);
  return (c) => (c < at.length ? at[c] : text.length);
}

/** Where a later line of a record starts: after its comment marker and the blanks around it. */
function leadingCarrier(text) {
  const m = /^\s*(\/\/|\*)?\s*/.exec(text);
  return m ? m[0].length : 0;
}

const FAIL_TITLE = {
  failed: "Failed: this does not take effect (see Problems)",
  overridden: "Overridden: another statement wins this conflict (7.5)",
};

function lineElement(text, n, cls, obj, fail) {
  const div = document.createElement("div");
  div.className = "ln";
  div.dataset.line = String(n);
  const no = document.createElement("span");
  no.className = "no";
  no.textContent = String(n);
  const code = document.createElement("span");
  code.className = "code";
  let i = 0;
  while (i < text.length) {
    let j = i + 1;
    while (j < text.length && cls[j] === cls[i] && obj[j] === obj[i] && fail[j] === fail[i]) j++;
    const piece = text.slice(i, j);
    if (!cls[i] && !obj[i] && !fail[i]) {
      code.append(piece);
    } else {
      const s = document.createElement("span");
      s.textContent = piece;
      if (cls[i]) s.className = `s-${cls[i]}`;
      if (fail[i]) {
        s.classList.add(`x-${fail[i]}`);
        s.title = FAIL_TITLE[fail[i]];
      }
      if (obj[i]) {
        s.classList.add("obj");
        s.dataset.obj = obj[i];
      }
      code.append(s);
    }
    i = j;
  }
  div.append(no, code);
  return div;
}
