"use strict";

// The page of the RNL Viewer. The app reads the file and draws it (src/document.rs); this page
// shows the drawing, the netlist text and the diagnostics, and follows a selected instance or net
// between the drawing and the text.

const tauri = window.__TAURI__;
const invoke = (cmd, args) => tauri.core.invoke(cmd, args);
const $ = (id) => document.getElementById(id);

const app = $("app");
const canvas = $("canvas");
const source = $("source");
const sidebar = $("sidebar");

const MIN_ZOOM = 0.02;
const MAX_ZOOM = 50;
const FIT_MARGIN = 24; // pixels around the drawing when it is fitted
const CLICK_SLOP = 4; // pixels a press may move and still be a click
const HIT_SLOP = 6; // pixels around a pin or a wire that still hit it
const SEVERITY_ORDER = { error: 0, warning: 1, hint: 2 };
const SIDEBAR_MIN = 220;
const DRAWING_MIN = 280;
const PROBLEMS_MIN = 60;
const SOURCE_MIN = 120;

let doc = null; // what open_document returned
let lastPath = null; // the file last opened, also when it failed to open, for reloading
let loading = false;

let current = 0; // the level drawn
let svg = null; // its <svg> element
let overlay = null; // the selection marks drawn over it
let groups = new Map(); // data-id -> the SVG groups that draw it
let extent = null; // the drawing's own viewBox: {x, y, w, h}
let view = null; // the view: top-left corner in drawing units, and pixels per unit (k)
let size = { w: 0, h: 0 }; // the canvas size the view was last applied to
let press = null; // a mouse press on the drawing: where, and whether it became a drag

let selected = null; // the selected object, an index into doc.objects
let fileShown = 0; // the file in the text pane
let fileViews = []; // per file: { el, lines: [line elements], scroll }
let lineOwners = []; // per file: line number -> the instance whose statement it is

// ------------------------------------------------------------------ opening

async function openPath(path, keep) {
  if (!path || loading) return;
  loading = true;
  document.body.classList.add("busy");
  try {
    lastPath = path;
    const d = await invoke("open_document", { path });
    const before = keep && doc ? snapshot() : null;
    doc = d;
    showDocument(before);
  } catch (e) {
    doc = null;
    clearDrawing(`Could not open ${path}`);
    clearSource("The file could not be read.");
    renderDiagnostics([{ severity: "error", message: String(e), file: "", line: 0, col: 0, origin: "" }]);
    setProblems(true);
  } finally {
    loading = false;
    document.body.classList.remove("busy");
  }
}

async function pickAndOpen() {
  if (loading) return;
  const path = await invoke("pick_file");
  if (path) await openPath(path, false);
}

function reload() {
  if (lastPath) openPath(lastPath, true);
}

/** What to keep when the same file is read again. */
function snapshot() {
  const sel = selected === null ? null : doc.objects[selected];
  if (fileViews[fileShown]) fileViews[fileShown].scroll = source.scrollTop;
  return {
    level: doc.levels[current] ? doc.levels[current].title : null,
    view,
    files: doc.files.map((f, k) => ({ path: f.path, scroll: fileViews[k] ? fileViews[k].scroll : 0 })),
    file: doc.files[fileShown] ? doc.files[fileShown].path : null,
    selected: sel && { kind: sel.kind, name: sel.name, level: sel.level },
  };
}

// ------------------------------------------------------------------ the document

function showDocument(before) {
  selected = null;
  renderFiles();
  const diags = doc.diagnostics.slice();
  if (doc.failed) diags.unshift({ severity: "error", message: doc.failed, file: "", line: 0, col: 0, origin: "" });
  renderDiagnostics(diags);
  markDiagnostics(diags);

  const nav = $("levels");
  nav.replaceChildren();
  doc.levels.forEach((lv, i) => {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = lv.title;
    b.title = lv.kind === "subckt" ? `Subcircuit ${lv.title}` : lv.title;
    b.addEventListener("click", () => showLevel(i, null));
    nav.append(b);
  });

  if (before) {
    for (const f of before.files) {
      const k = doc.files.findIndex((x) => x.path === f.path);
      if (k >= 0) fileViews[k].scroll = f.scroll;
    }
  }
  const file = before ? doc.files.findIndex((f) => f.path === before.file) : -1;
  showFile(file >= 0 ? file : 0);

  if (doc.levels.length === 0) {
    clearDrawing("Nothing to draw.");
    return;
  }
  const level = before ? doc.levels.findIndex((l) => l.title === before.level) : -1;
  showLevel(level >= 0 ? level : 0, level >= 0 ? before.view : null);

  if (before && before.selected) {
    const s = before.selected;
    const o = doc.objects.findIndex((x) => x.kind === s.kind && x.name === s.name && x.level === s.level);
    if (o >= 0) select(o, "reload");
  }
}

// ------------------------------------------------------------------ the drawing

function showLevel(i, keptView) {
  current = i;
  const lv = doc.levels[i];
  [...$("levels").children].forEach((b, k) => b.setAttribute("aria-selected", String(k === i)));
  $("status").textContent = lv.status ? `Not drawn without a solver: ${lv.status}` : "";
  $("status").title = $("status").textContent;
  if (svg) svg.remove();
  svg = null;
  overlay = null;
  groups = new Map();
  if (lv.empty) {
    view = null;
    $("zoom").textContent = "";
    $("empty").textContent = "Nothing on this level has a position yet, so nothing is drawn.";
    $("empty").hidden = false;
    return;
  }
  const parsed = new DOMParser().parseFromString(lv.svg, "image/svg+xml");
  const el = document.importNode(parsed.documentElement, true);
  const box = (el.getAttribute("viewBox") || "").split(/[\s,]+/).map(Number);
  extent = box.length === 4 && box.every(Number.isFinite)
    ? { x: box[0], y: box[1], w: box[2], h: box[3] }
    : { x: 0, y: 0, w: Number(el.getAttribute("width")) || 200, h: Number(el.getAttribute("height")) || 60 };
  el.removeAttribute("width");
  el.removeAttribute("height");
  el.setAttribute("preserveAspectRatio", "xMinYMin meet");
  for (const g of el.querySelectorAll("[data-id]")) {
    const id = g.getAttribute("data-id");
    if (!groups.has(id)) groups.set(id, []);
    groups.get(id).push(g);
  }
  overlay = document.createElementNS("http://www.w3.org/2000/svg", "g");
  overlay.setAttribute("class", "overlay");
  overlay.setAttribute("pointer-events", "none");
  el.append(overlay);
  $("empty").hidden = true;
  svg = el;
  canvas.append(svg);
  if (keptView) {
    view = keptView;
    apply();
  } else {
    fit();
  }
  paintDrawingSelection();
}

function clearDrawing(message) {
  if (svg) svg.remove();
  svg = null;
  overlay = null;
  view = null;
  $("empty").textContent = message;
  $("empty").hidden = false;
  $("levels").replaceChildren();
  $("status").textContent = "";
  $("zoom").textContent = "";
  $("selection").textContent = "";
}

function canvasSize() {
  return { w: canvas.clientWidth || 1, h: canvas.clientHeight || 1 };
}

function apply() {
  if (!svg || !view) return;
  size = canvasSize();
  svg.setAttribute("viewBox", `${view.x} ${view.y} ${size.w / view.k} ${size.h / view.k}`);
  $("zoom").textContent = `${Math.round(view.k * 100)}%`;
}

function fit() {
  if (!svg) return;
  const { w, h } = canvasSize();
  const k = clamp(Math.min((w - 2 * FIT_MARGIN) / extent.w, (h - 2 * FIT_MARGIN) / extent.h), MIN_ZOOM, MAX_ZOOM);
  view = { k, x: extent.x - (w / k - extent.w) / 2, y: extent.y - (h / k - extent.h) / 2 };
  apply();
}

/** Zoom by `factor` keeping the drawing point under (px, py), in canvas pixels, in place. */
function zoomAt(px, py, factor) {
  if (!view) return;
  const k = clamp(view.k * factor, MIN_ZOOM, MAX_ZOOM);
  const ux = view.x + px / view.k;
  const uy = view.y + py / view.k;
  view = { k, x: ux - px / k, y: uy - py / k };
  apply();
}

function zoomCenter(factor) {
  const { w, h } = canvasSize();
  zoomAt(w / 2, h / 2, factor);
}

function clamp(v, lo, hi) {
  return Math.min(hi, Math.max(lo, v));
}

function canvasPoint(e) {
  const r = canvas.getBoundingClientRect();
  return { x: e.clientX - r.left, y: e.clientY - r.top };
}

canvas.addEventListener(
  "wheel",
  (e) => {
    e.preventDefault();
    if (!view) return;
    const p = canvasPoint(e);
    const r = canvas.getBoundingClientRect();
    const dy = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaMode === 2 ? e.deltaY * r.height : e.deltaY;
    zoomAt(p.x, p.y, Math.exp(-dy * 0.0015));
  },
  { passive: false },
);

canvas.addEventListener("pointerdown", (e) => {
  if (!view || (e.button !== 0 && e.button !== 1)) return;
  e.preventDefault();
  canvas.setPointerCapture(e.pointerId);
  press = { id: e.pointerId, button: e.button, x0: e.clientX, y0: e.clientY, x: e.clientX, y: e.clientY, drag: false };
});

canvas.addEventListener("pointermove", (e) => {
  if (!press || e.pointerId !== press.id) {
    if (!press && view) canvas.classList.toggle("over-object", hitAt(canvasPoint(e)) !== null);
    return;
  }
  if (!press.drag && Math.hypot(e.clientX - press.x0, e.clientY - press.y0) > CLICK_SLOP) {
    press.drag = true;
    canvas.classList.add("dragging");
  }
  if (!press.drag) return;
  view = { ...view, x: view.x - (e.clientX - press.x) / view.k, y: view.y - (e.clientY - press.y) / view.k };
  press.x = e.clientX;
  press.y = e.clientY;
  apply();
});

function endPress(e, cancelled) {
  if (!press || e.pointerId !== press.id) return;
  const click = !press.drag && press.button === 0 && !cancelled;
  press = null;
  canvas.classList.remove("dragging");
  if (!click) return;
  const hit = hitAt(canvasPoint(e));
  if (hit) select(hit.objects[0], "drawing");
  else clearSelection();
}
canvas.addEventListener("pointerup", (e) => endPress(e, false));
canvas.addEventListener("pointercancel", (e) => endPress(e, true));
// no autoscroll on the middle button, no page menu on the drawing
canvas.addEventListener("mousedown", (e) => { if (e.button === 1) e.preventDefault(); });
canvas.addEventListener("contextmenu", (e) => e.preventDefault());
canvas.addEventListener("dblclick", fit);

// keep the middle of the view in place when the window or a panel changes size
new ResizeObserver(() => {
  if (!view) return;
  const now = canvasSize();
  view = { ...view, x: view.x + (size.w - now.w) / (2 * view.k), y: view.y + (size.h - now.h) / (2 * view.k) };
  apply();
}).observe(canvas);

// ------------------------------------------------------------------ what a click on the drawing hits

/**
 * The hit under a canvas point that stands for some object: a pin first, then a wire, then the
 * smallest symbol box around the point. Null when there is none.
 */
function hitAt(p) {
  if (!view || !doc || !doc.levels[current]) return null;
  const u = view.x + p.x / view.k;
  const v = view.y + p.y / view.k;
  const slop = HIT_SLOP / view.k;
  const hits = doc.levels[current].hits.filter((h) => h.objects.length > 0);
  let best = null;
  let bestD = Infinity;
  for (const h of hits) {
    if (h.kind !== "pin") continue;
    const d = Math.hypot(u - h.points[0][0], v - h.points[0][1]);
    if (d <= slop && d < bestD) [best, bestD] = [h, d];
  }
  if (best) return best;
  for (const h of hits) {
    if (h.kind !== "wire") continue;
    for (let k = 1; k < h.points.length; k++) {
      const d = segmentDistance(u, v, h.points[k - 1], h.points[k]);
      if (d <= slop && d < bestD) [best, bestD] = [h, d];
    }
  }
  if (best) return best;
  let bestArea = Infinity;
  for (const h of hits) {
    if (h.kind !== "box") continue;
    const [[x0, y0], [x1, y1]] = h.points;
    if (u < x0 - slop || u > x1 + slop || v < y0 - slop || v > y1 + slop) continue;
    const area = (x1 - x0) * (y1 - y0);
    if (area < bestArea) [best, bestArea] = [h, area];
  }
  return best;
}

function segmentDistance(u, v, a, b) {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const len2 = dx * dx + dy * dy;
  const t = len2 === 0 ? 0 : clamp(((u - a[0]) * dx + (v - a[1]) * dy) / len2, 0, 1);
  return Math.hypot(u - (a[0] + t * dx), v - (a[1] + t * dy));
}

// ------------------------------------------------------------------ selection

/** Select an object. `from` is "drawing", "source" or "reload". */
function select(o, from) {
  if (!doc || o === undefined || o === null) return;
  selected = o;
  const obj = doc.objects[o];
  // a name picked in the text is shown on the level that has it
  if (from === "source" && (!doc.levels[current] || doc.levels[current].host !== obj.level)) {
    const k = doc.levels.findIndex((l) => l.host === obj.level);
    if (k >= 0) showLevel(k, null);
  }
  paintDrawingSelection();
  paintSourceSelection(from === "drawing");
  const what = obj.kind === "instance" ? "instance" : "net";
  const sel = $("selection");
  sel.replaceChildren();
  const b = document.createElement("b");
  b.textContent = obj.name;
  sel.append(b, ` ${what} in ${obj.level_title}`);
}

function clearSelection() {
  if (selected === null) return;
  selected = null;
  paintDrawingSelection();
  paintSourceSelection(false);
  $("selection").textContent = "";
}

function paintDrawingSelection() {
  if (!svg) return;
  for (const g of svg.querySelectorAll(".sel")) g.classList.remove("sel");
  overlay.replaceChildren();
  if (selected === null) return;
  const isInstance = doc.objects[selected].kind === "instance";
  const pad = 3;
  for (const h of doc.levels[current].hits) {
    if (!h.objects.includes(selected)) continue;
    if (h.id) for (const g of groups.get(h.id) || []) g.classList.add("sel");
    if (h.kind === "box" && isInstance) {
      const [[x0, y0], [x1, y1]] = h.points;
      const r = svgElement("rect", { x: x0 - pad, y: y0 - pad, width: x1 - x0 + 2 * pad, height: y1 - y0 + 2 * pad });
      r.setAttribute("vector-effect", "non-scaling-stroke");
      overlay.append(r);
    } else if (h.kind === "pin" && !isInstance) {
      const c = svgElement("circle", { cx: h.points[0][0], cy: h.points[0][1], r: 3.2 });
      c.setAttribute("vector-effect", "non-scaling-stroke");
      overlay.append(c);
    }
  }
}

function svgElement(name, attrs) {
  const el = document.createElementNS("http://www.w3.org/2000/svg", name);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
  return el;
}

/** Mark the selected object's places in the text; with `reveal`, scroll to where it is named. */
function paintSourceSelection(reveal) {
  for (const el of source.querySelectorAll(".hl")) el.classList.remove("hl");
  for (const el of source.querySelectorAll(".hl-line")) el.classList.remove("hl-line");
  if (selected === null || !doc) return;
  const obj = doc.objects[selected];
  if (obj.lines) {
    const [f, a, b] = obj.lines;
    for (let n = a; n <= b; n++) fileViews[f].lines[n - 1]?.classList.add("hl-line");
  }
  for (const el of source.querySelectorAll(`[data-obj="${selected}"]`)) el.classList.add("hl");
  if (!reveal) return;
  const spans = doc.spans.filter((s) => s.object === selected);
  const target = spans.find((s) => s.kind === "name") || spans.find((s) => s.file === fileShown) || spans[0];
  if (target) revealLine(target.file, target.line, false);
}

// ------------------------------------------------------------------ the netlist text

function clearSource(message) {
  fileViews = [];
  lineOwners = [];
  $("file-tabs").replaceChildren();
  source.replaceChildren();
  const p = document.createElement("p");
  p.className = "placeholder";
  p.textContent = message;
  source.append(p);
}

function renderFiles() {
  source.replaceChildren();
  fileViews = [];
  lineOwners = doc.files.map(() => new Map());
  doc.objects.forEach((o, k) => {
    if (o.kind !== "instance" || !o.lines) return;
    const [f, a, b] = o.lines;
    for (let n = a; n <= b; n++) lineOwners[f].set(n, k);
  });
  const tabs = $("file-tabs");
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

function showFile(f) {
  if (!fileViews[f]) return;
  if (fileViews[fileShown] && fileShown !== f) fileViews[fileShown].scroll = source.scrollTop;
  fileViews.forEach((v, k) => { v.el.hidden = k !== f; });
  [...$("file-tabs").children].forEach((b, k) => b.setAttribute("aria-selected", String(k === f)));
  fileShown = f;
  source.scrollTop = fileViews[f].scroll;
}

/** Show a line of a file in the middle of the text pane; with `flash`, blink it. */
function revealLine(f, line, flash) {
  if (f === null || f === undefined || !fileViews[f] || !line) return;
  if (app.classList.contains("sidebar-hidden")) setSidebar(true);
  showFile(f);
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

source.addEventListener("click", (e) => {
  if (!doc || String(window.getSelection())) return; // selecting text is not a click
  const s = e.target.closest("[data-obj]");
  if (s) {
    select(Number(s.dataset.obj), "source");
    return;
  }
  const ln = e.target.closest(".ln");
  const owner = ln ? lineOwners[fileShown].get(Number(ln.dataset.line)) : undefined;
  if (owner !== undefined) select(owner, "source");
  else clearSelection();
});

// ---- syntax

const KEYWORD_CHAR = /[\p{L}\p{N}_-]/u;
const NAME_CHAR = /[\p{L}\p{N}_.\-/]/u;
const REF_CLASS = { "#": "host", "&": "host", "$": "def", "%": "local", "?": "local" };
const SPECTRE_WORDS = new Set([
  "simulator", "subckt", "ends", "inline", "model", "include", "ahdl_include", "parameters", "global",
  "section", "endsection", "library", "endlibrary", "statistics", "real",
]);

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

  const out = [];
  const state = { comment: false };
  let joined = false; // the previous host line ends with `\`
  lines.forEach((text, i) => {
    const cls = new Array(text.length).fill("");
    const obj = new Array(text.length).fill(-1);
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
    for (const s of spans[i]) {
      for (let c = s.col - 1; c < s.col - 1 + s.len && c < text.length; c++) obj[c] = s.object;
    }
    out.push(lineElement(text, i + 1, cls, obj));
  });
  return out;
}

function lineElement(text, n, cls, obj) {
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
    while (j < text.length && cls[j] === cls[i] && obj[j] === obj[i]) j++;
    const piece = text.slice(i, j);
    if (!cls[i] && obj[i] < 0) {
      code.append(piece);
    } else {
      const s = document.createElement("span");
      s.textContent = piece;
      if (cls[i]) s.className = `s-${cls[i]}`;
      if (obj[i] >= 0) {
        s.classList.add("obj");
        s.dataset.obj = String(obj[i]);
      }
      code.append(s);
    }
    i = j;
  }
  div.append(no, code);
  return div;
}

function mark(cls, from, to, name) {
  for (let k = from; k < to && k < cls.length; k++) cls[k] = name;
}

/**
 * Highlight a host line outside records: comments, the first word of a statement, parameter
 * names. Returns whether the line continues on the next (a Spectre `\`).
 */
function hostLine(text, cls, cont, records) {
  const trimmed = text.trimStart();
  const lead = text.length - trimmed.length;
  if (trimmed.startsWith("*") || trimmed.startsWith("//")) {
    // a comment line; a carrier's records are highlighted over it
    mark(cls, 0, text.length, records ? "marker" : "comment");
    return false;
  }
  let end = text.length;
  for (let k = 0; k < text.length; k++) {
    const c = text[k];
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
  return text.slice(0, end).trimEnd().endsWith("\\");
}

/** Highlight the record characters [from, to) of a line; `state` carries an open comment. */
function rnlRange(text, from, to, first, cls, state) {
  let i = from;
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
      let j = i + 1;
      while (j < to && KEYWORD_CHAR.test(text[j])) j++;
      mark(cls, i, j, "keyword");
      i = j;
      continue;
    }
    if (c in REF_CLASS) {
      let j = i + 1;
      if (text[j] === '"') {
        j++;
        while (j < to && text[j] !== '"') j++;
        j++;
      } else {
        while (j < to && NAME_CHAR.test(text[j])) j++;
      }
      mark(cls, i, j, REF_CLASS[c]);
      i = j;
      continue;
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
    if (KEYWORD_CHAR.test(c)) {
      let j = i;
      while (j < to && KEYWORD_CHAR.test(text[j])) j++;
      let k = j;
      while (k < to && text[k] === " ") k++;
      if (text[k] === "=" && text[k + 1] !== ">") mark(cls, i, j, "param");
      i = j;
      continue;
    }
    if ("{}()[];,=:<>".includes(c)) mark(cls, i, i + 1, "punct");
    i++;
  }
}

// ------------------------------------------------------------------ diagnostics

function renderDiagnostics(list) {
  const sorted = list
    .map((d, i) => ({ d, i }))
    .sort((a, b) => SEVERITY_ORDER[a.d.severity] - SEVERITY_ORDER[b.d.severity] || a.i - b.i)
    .map((x) => x.d);
  const ul = $("diags");
  ul.replaceChildren();
  for (const d of sorted) {
    const li = document.createElement("li");
    li.className = `diag ${d.severity}`;
    li.append(span("sev", d.severity), span("msg", d.message));
    if (d.file) {
      const loc = span("loc", `${d.file}:${d.line}${d.col ? `:${d.col}` : ""}`);
      if (d.full_path) loc.title = d.full_path;
      li.append(loc);
    }
    if (d.origin) li.append(span("more", d.origin));
    if (d.cause) li.append(span("more", `upstream cause: ${d.cause}`));
    if (d.file_index !== null && d.file_index !== undefined && d.line) {
      li.classList.add("located");
      li.title = "Show in the netlist";
      li.addEventListener("click", () => revealLine(d.file_index, d.line, true));
    }
    ul.append(li);
  }
  if (sorted.length === 0) {
    const li = document.createElement("li");
    li.className = "nothing";
    li.textContent = "No errors, warnings or hints.";
    ul.append(li);
  }
  const counts = $("counts");
  counts.replaceChildren();
  const n = (s) => list.filter((d) => d.severity === s).length;
  for (const [s, one, many] of [["error", "error", "errors"], ["warning", "warning", "warnings"], ["hint", "hint", "hints"]]) {
    const k = n(s);
    if (k) counts.append(span(`n-${s}`, `${k} ${k === 1 ? one : many}`));
  }
  if (list.length === 0) counts.append(span("none", "none"));
}

/** Colour the line numbers of lines with diagnostics, by the most serious one. */
function markDiagnostics(list) {
  const worst = new Map();
  for (const d of list) {
    if (d.file_index === null || d.file_index === undefined || !d.line) continue;
    const key = `${d.file_index}:${d.line}`;
    const was = worst.get(key);
    if (!was || SEVERITY_ORDER[d.severity] < SEVERITY_ORDER[was]) worst.set(key, d.severity);
  }
  for (const [key, sev] of worst) {
    const [f, line] = key.split(":").map(Number);
    const el = fileViews[f] && fileViews[f].lines[line - 1];
    if (el) el.classList.add(`d-${sev}`);
  }
}

function span(cls, text) {
  const s = document.createElement("span");
  s.className = cls;
  s.textContent = text;
  return s;
}

// ------------------------------------------------------------------ panels and their sizes

function load(key, fallback) {
  try {
    const v = localStorage.getItem(key);
    return v === null ? fallback : JSON.parse(v);
  } catch {
    return fallback;
  }
}

function save(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // the layout just starts from its defaults next time
  }
}

function setSidebar(shown) {
  app.classList.toggle("sidebar-hidden", !shown);
  save("sidebar-shown", shown);
  invoke("set_view_check", { panel: "netlist", shown }).catch(() => {});
}

function setProblems(shown) {
  $("problems-head").setAttribute("aria-expanded", String(shown));
  sidebar.classList.toggle("problems-folded", !shown);
  save("problems-shown", shown);
  invoke("set_view_check", { panel: "problems", shown }).catch(() => {});
}

function sidebarWidthLimits() {
  const w = window.innerWidth;
  return [SIDEBAR_MIN, Math.max(SIDEBAR_MIN, Math.min(w * 0.75, w - DRAWING_MIN))];
}

function problemsHeightLimits() {
  const head = $("problems-head").offsetHeight + document.querySelector(".pane-head").offsetHeight;
  return [PROBLEMS_MIN, Math.max(PROBLEMS_MIN, sidebar.clientHeight - head - SOURCE_MIN)];
}

function setSidebarWidth(px) {
  const [lo, hi] = sidebarWidthLimits();
  const w = Math.round(clamp(px, lo, hi));
  document.documentElement.style.setProperty("--sidebar-w", `${w}px`);
  return w;
}

function setProblemsHeight(px) {
  const [lo, hi] = problemsHeightLimits();
  const h = Math.round(clamp(px, lo, hi));
  document.documentElement.style.setProperty("--problems-h", `${h}px`);
  return h;
}

/** Drag a splitter: `axis` "x" or "y"; `get` the size when the drag starts; `set` the new size. */
function splitter(el, axis, sign, get, set, key) {
  el.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    el.setPointerCapture(e.pointerId);
    el.classList.add("active");
    document.body.classList.add(axis === "x" ? "resizing-x" : "resizing-y");
    const start = axis === "x" ? e.clientX : e.clientY;
    const base = get();
    let last = base;
    const move = (ev) => {
      last = set(base + sign * ((axis === "x" ? ev.clientX : ev.clientY) - start));
    };
    const up = () => {
      el.releasePointerCapture(e.pointerId);
      el.classList.remove("active");
      document.body.classList.remove("resizing-x", "resizing-y");
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
      save(key, last);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
  });
}

splitter($("vsplit"), "x", 1, () => sidebar.getBoundingClientRect().width, setSidebarWidth, "sidebar-width");
splitter($("hsplit"), "y", -1, () => $("problems-body").getBoundingClientRect().height, setProblemsHeight, "problems-height");

window.addEventListener("resize", () => {
  setSidebarWidth(sidebar.getBoundingClientRect().width || load("sidebar-width", 420));
  if (!sidebar.classList.contains("problems-folded")) setProblemsHeight($("problems-body").getBoundingClientRect().height);
});

$("hide-sidebar").addEventListener("click", () => setSidebar(false));
$("show-sidebar").addEventListener("click", () => setSidebar(true));
$("problems-head").addEventListener("click", () => setProblems(sidebar.classList.contains("problems-folded")));

// ------------------------------------------------------------------ commands: menu and keys

function command(id) {
  switch (id) {
    case "open": pickAndOpen(); break;
    case "reload": reload(); break;
    case "fit": fit(); break;
    case "zoom-in": zoomCenter(1.25); break;
    case "zoom-out": zoomCenter(0.8); break;
    case "toggle-netlist": setSidebar(app.classList.contains("sidebar-hidden")); break;
    case "toggle-problems": setProblems(sidebar.classList.contains("problems-folded")); break;
    default: break;
  }
}

tauri.event.listen("menu", (e) => command(e.payload));

document.addEventListener("keydown", (e) => {
  const ctrl = e.ctrlKey || e.metaKey;
  const key = e.key.toLowerCase();
  let id = null;
  if (ctrl && !e.shiftKey && key === "o") id = "open";
  else if (e.key === "F5" || (ctrl && key === "r")) id = "reload"; // re-read the file, not the page
  else if (ctrl && !e.shiftKey && key === "b") id = "toggle-netlist";
  else if (ctrl && e.shiftKey && key === "m") id = "toggle-problems";
  else if (ctrl && (e.key === "=" || e.key === "+")) id = "zoom-in";
  else if (ctrl && e.key === "-") id = "zoom-out";
  else if (!ctrl && !e.altKey && (key === "f" || key === "0")) id = "fit";
  else if (e.key === "Escape") {
    clearSelection();
    e.preventDefault();
    return;
  }
  if (id) {
    e.preventDefault();
    command(id);
  }
});

// ------------------------------------------------------------------ start

setSidebarWidth(load("sidebar-width", 420));
setProblemsHeight(load("problems-height", 180));
setSidebar(load("sidebar-shown", true));
setProblems(load("problems-shown", true));
{
  const li = document.createElement("li");
  li.className = "nothing";
  li.textContent = "No file is open.";
  $("diags").append(li);
}
invoke("initial_file").then((path) => openPath(path, false));
