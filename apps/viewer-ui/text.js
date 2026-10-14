// The text pane: the netlist and the files it includes, highlighted, with the statements that do
// not take effect marked, and the places of the selected object. Clicking a name or a statement in
// the text selects its object; double-clicking a subcircuit instance opens it.

import { doc, selected } from "./state.js";
import { clearSelection, select } from "./selection.js";
import { descend, opens } from "./subcircuits.js";
import { hostLine, mark, rnlRange } from "./syntax.js";
import { $ } from "./util.js";

const app = $("app");
const source = $("source");
const netlistPane = $("pane-netlist");
const tabs = $("file-tabs");

export let fileShown = 0; // the file in the text pane
export let fileViews = []; // per file: { el, lines: [line elements], scroll }
let lineOwners = []; // per file: line number -> the instance whose statement it is
let pendingReveal = null; // a line to bring into view once the text pane is shown

/** Mark the selected object's places in the text; with `reveal`, scroll to where it is named. */
export function paintSourceSelection(reveal) {
  for (const el of source.querySelectorAll(".hl")) el.classList.remove("hl");
  for (const el of source.querySelectorAll(".hl-line")) el.classList.remove("hl-line");
  if (selected === null || !doc) return;
  const obj = doc.objects[selected];
  if (obj.lines) {
    const [f, a, b] = obj.lines;
    for (let n = a; n <= b; n++) fileViews[f].lines[n - 1]?.classList.add("hl-line");
  }
  for (const el of source.querySelectorAll(`[data-obj~="${selected}"]`)) el.classList.add("hl");
  if (!reveal) return;
  const spans = doc.spans.filter((s) => s.object === selected);
  const target = spans.find((s) => s.kind === "name") || spans.find((s) => s.file === fileShown) || spans[0];
  if (target) revealLine(target.file, target.line, false);
}

export function clearSource(message) {
  fileViews = [];
  lineOwners = [];
  tabs.replaceChildren();
  source.replaceChildren();
  const p = document.createElement("p");
  p.className = "placeholder";
  p.textContent = message;
  source.append(p);
}

export function renderFiles() {
  source.replaceChildren();
  fileViews = [];
  lineOwners = doc.files.map(() => new Map());
  doc.objects.forEach((o, k) => {
    if (o.kind !== "instance" || !o.lines) return;
    const [f, a, b] = o.lines;
    for (let n = a; n <= b; n++) lineOwners[f].set(n, k);
  });
  tabs.replaceChildren();
  doc.files.forEach((file, f) => {
    const el = document.createElement("div");
    el.className = "file";
    el.hidden = true;
    const lines = renderLines(file, f);
    el.append(...lines);
    source.append(el);
    fileViews.push({ el, lines, scroll: 0 });
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = file.name.split(/[\\/]/).pop();
    b.title = file.path;
    b.addEventListener("click", () => showFile(f));
    tabs.append(b);
  });
  if (doc.files.length === 0) clearSource("This document has no host file to show.");
}

export function showFile(f) {
  if (!fileViews[f]) return;
  if (fileViews[fileShown] && fileShown !== f) fileViews[fileShown].scroll = source.scrollTop;
  fileViews.forEach((v, k) => { v.el.hidden = k !== f; });
  [...tabs.children].forEach((b, k) => b.setAttribute("aria-selected", String(k === f)));
  fileShown = f;
  source.scrollTop = fileViews[f].scroll;
  tabInView(tabs.children[f]);
}

// The file tabs scroll sideways with no scrollbar: the tab of the file shown is kept in view, and a
// wheel turned over them scrolls them, as a sideways swipe does.

/** Scroll the file tabs, when they must, so that `tab` is in view, or as much of it as fits. */
function tabInView(tab) {
  const [s, t] = [tabs.getBoundingClientRect(), tab.getBoundingClientRect()];
  if (t.left < s.left) tabs.scrollLeft -= s.left - t.left;
  else if (t.right > s.right) tabs.scrollLeft += Math.min(t.right - s.right, t.left - s.left);
}

tabs.addEventListener(
  "wheel",
  (e) => {
    if (Math.abs(e.deltaY) <= Math.abs(e.deltaX) || tabs.scrollWidth <= tabs.clientWidth) return;
    e.preventDefault();
    tabs.scrollLeft += e.deltaMode === 1 ? e.deltaY * 16 : e.deltaMode === 2 ? e.deltaY * tabs.clientWidth : e.deltaY;
  },
  { passive: false },
);

/** Whether the text pane is on screen, so that lines can be scrolled to. */
function sourceShown() {
  return !app.classList.contains("sidebar-hidden") && !netlistPane.hidden;
}

/**
 * Show a line of a file in the middle of the text pane; with `flash`, blink it. While the text
 * pane is hidden, the line waits until it is shown again.
 */
export function revealLine(f, line, flash) {
  if (f === null || f === undefined || !fileViews[f] || !line) return;
  showFile(f);
  if (!sourceShown()) {
    pendingReveal = { f, line, flash };
    return;
  }
  pendingReveal = null;
  const el = fileViews[f].lines[line - 1];
  if (!el) return;
  const top = el.offsetTop - source.clientHeight / 2 + el.offsetHeight / 2;
  source.scrollTop = Math.max(0, top);
  if (flash) {
    el.classList.remove("flash");
    void el.offsetWidth; // restart the animation
    el.classList.add("flash");
  }
}

export function revealPending() {
  if (pendingReveal && sourceShown()) {
    const { f, line, flash } = pendingReveal;
    revealLine(f, line, flash);
  }
}

source.addEventListener("click", (e) => {
  if (!doc || String(window.getSelection())) return; // selecting text is not a click
  const s = e.target.closest("[data-obj]");
  if (s) {
    // the first object of a place is the one written most narrowly: `#M1` in `#M1:d` is M1
    select(Number(s.dataset.obj.split(" ")[0]), "source");
    return;
  }
  const ln = e.target.closest(".ln");
  const owner = ln ? lineOwners[fileShown].get(Number(ln.dataset.line)) : undefined;
  if (owner !== undefined) select(owner, "source");
  else clearSelection();
});

source.addEventListener("dblclick", (e) => {
  if (!doc) return;
  const s = e.target.closest("[data-obj]");
  const ln = e.target.closest(".ln");
  const o = s
    ? s.dataset.obj.split(" ").map(Number).find((k) => opens(k) !== null)
    : ln ? lineOwners[fileShown].get(Number(ln.dataset.line)) : undefined;
  if (opens(o) === null) return;
  window.getSelection().removeAllRanges(); // the word the double click selected
  descend(o);
});

/** The line elements of a file: numbers, highlighted text, and the names of objects. */
function renderLines(file, f) {
  const lines = file.text.split(/\r?\n/);
  if (lines.length > 1 && lines[lines.length - 1] === "") lines.pop();
  // which characters belong to a record, per line: [from, to) in 0-based characters
  const rnl = lines.map(() => []);
  for (const [l0, c0, l1, c1] of file.records) {
    for (let n = l0; n <= l1 && n <= lines.length; n++) {
      const from = n === l0 ? c0 - 1 : 0;
      const to = n === l1 ? c1 : lines[n - 1].length;
      rnl[n - 1].push({ from, to, first: n === l0 });
    }
  }
  const spans = lines.map(() => []);
  for (const s of doc.spans) if (s.file === f && spans[s.line - 1]) spans[s.line - 1].push(s);
  // the statements that do not take effect, per line: [from, to) and why
  const failedOn = lines.map(() => []);
  for (const [l0, c0, l1, c1, overridden] of file.failed) {
    for (let n = l0; n <= l1 && n <= lines.length; n++) {
      const from = n === l0 ? c0 - 1 : leadingCarrier(lines[n - 1]);
      const to = n === l1 ? c1 : lines[n - 1].length;
      failedOn[n - 1].push({ from, to, kind: overridden ? "overridden" : "failed" });
    }
  }

  const out = [];
  const state = { comment: false };
  let joined = false; // the previous host line ends with `\`
  lines.forEach((text, i) => {
    const cls = new Array(text.length).fill("");
    if (i === 0 && f === 0 && doc.dialect === "spice") {
      mark(cls, 0, text.length, "comment"); // the title line
    } else if (rnl[i].length) {
      hostLine(text, cls, false, rnl[i]);
      for (const r of rnl[i]) {
        if (r.first) state.comment = false;
        rnlRange(text, r.from, Math.min(r.to, text.length), r.first, cls, state);
      }
      joined = false;
    } else {
      const cont = joined || /^\s*\+/.test(text);
      joined = hostLine(text, cls, cont, null);
    }
    // what does not take effect at each character: a failure over an override
    const fail = new Array(text.length).fill("");
    for (const r of failedOn[i]) {
      for (let c = r.from; c < r.to && c < text.length; c++) {
        if (fail[c] !== "failed") fail[c] = r.kind;
      }
    }
    // the objects named at each character, the most narrowly named first
    const objs = new Array(text.length).fill(null);
    for (const s of spans[i].slice().sort((a, b) => a.len - b.len)) {
      for (let c = s.col - 1; c < s.col - 1 + s.len && c < text.length; c++) {
        if (!objs[c]) objs[c] = [];
        if (!objs[c].includes(s.object)) objs[c].push(s.object);
      }
    }
    out.push(lineElement(text, i + 1, cls, objs.map((o) => (o ? o.join(" ") : "")), fail));
  });
  return out;
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
