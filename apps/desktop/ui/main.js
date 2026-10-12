"use strict";

// The page of the RNL Viewer. The app reads and draws the file (src/document.rs); this page shows
// one drawing level at a time, moves and zooms the view, and lists the diagnostics.

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);

const $ = (id) => document.getElementById(id);
const canvas = $("canvas");

const MIN_ZOOM = 0.02;
const MAX_ZOOM = 50;
const FIT_MARGIN = 24; // pixels around the drawing when it is fitted
const SEVERITY_ORDER = { error: 0, warning: 1, hint: 2 };

let doc = null; // what open_document returned
let lastPath = null; // the file last opened, also when it failed to open, for reloading
let current = 0; // the level shown
let svg = null; // its <svg> element
let extent = null; // the drawing's own viewBox: {x, y, w, h}
let view = null; // the view: top-left corner in drawing units, and pixels per unit (k)
let size = { w: 0, h: 0 }; // the canvas size the view was last applied to
let drag = null;
let loading = false;

// ------------------------------------------------------------------ opening

async function openPath(path, keepView) {
  if (!path || loading) return;
  loading = true;
  document.body.classList.add("busy");
  try {
    lastPath = path;
    const d = await invoke("open_document", { path });
    const shown = keepView && doc && doc.levels[current] ? doc.levels[current].title : null;
    const level = shown === null ? -1 : d.levels.findIndex((l) => l.title === shown);
    doc = d;
    showDocument(level >= 0 ? level : 0, level >= 0 ? view : null);
  } catch (e) {
    doc = null;
    clearDrawing(`Could not open ${path}`);
    renderDiagnostics([{ severity: "error", message: String(e), file: "", line: 0, col: 0, origin: "" }]);
    setPanelOpen(true);
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

// ------------------------------------------------------------------ showing

function showDocument(level, keptView) {
  $("file").textContent = doc.name;
  $("file").title = doc.path;
  $("reload").disabled = false;
  const nav = $("levels");
  nav.replaceChildren();
  if (doc.levels.length > 1) {
    doc.levels.forEach((lv, i) => {
      const b = document.createElement("button");
      b.type = "button";
      b.textContent = lv.title;
      b.title = lv.kind === "subckt" ? `Subcircuit ${lv.title}` : lv.title;
      b.addEventListener("click", () => showLevel(i, null));
      nav.append(b);
    });
  }
  const diags = doc.diagnostics.slice();
  if (doc.failed) diags.unshift({ severity: "error", message: doc.failed, file: "", line: 0, col: 0, origin: "" });
  renderDiagnostics(diags);
  if (doc.levels.length === 0) {
    clearDrawing("Nothing to draw.");
    return;
  }
  showLevel(level, keptView);
}

function showLevel(i, keptView) {
  current = i;
  const lv = doc.levels[i];
  [...$("levels").children].forEach((b, k) => b.setAttribute("aria-selected", String(k === i)));
  $("status").textContent = lv.status ? `Not drawn without a solver: ${lv.status}` : "";
  $("status").title = $("status").textContent;
  if (lv.empty) {
    if (svg) svg.remove();
    svg = null;
    view = null;
    $("zoom").textContent = "";
    $("fit").disabled = true;
    $("empty").textContent = "Nothing on this level has a position yet, so nothing is drawn.";
    $("empty").hidden = false;
    return;
  }
  $("fit").disabled = false;
  const parsed = new DOMParser().parseFromString(lv.svg, "image/svg+xml");
  const el = document.importNode(parsed.documentElement, true);
  const box = (el.getAttribute("viewBox") || "").split(/[\s,]+/).map(Number);
  extent = box.length === 4 && box.every(Number.isFinite)
    ? { x: box[0], y: box[1], w: box[2], h: box[3] }
    : { x: 0, y: 0, w: Number(el.getAttribute("width")) || 200, h: Number(el.getAttribute("height")) || 60 };
  el.removeAttribute("width");
  el.removeAttribute("height");
  el.setAttribute("preserveAspectRatio", "xMinYMin meet");
  $("empty").hidden = true;
  if (svg) svg.remove();
  svg = el;
  canvas.append(svg);
  if (keptView) {
    view = keptView;
    apply();
  } else {
    fit();
  }
}

function clearDrawing(message) {
  if (svg) svg.remove();
  svg = null;
  view = null;
  $("empty").textContent = message;
  $("empty").hidden = false;
  $("levels").replaceChildren();
  $("status").textContent = "";
  $("zoom").textContent = "";
  $("fit").disabled = true;
  $("reload").disabled = !lastPath;
  $("file").textContent = lastPath ? lastPath.split(/[\\/]/).pop() : "";
  $("file").title = lastPath || "";
}

// ------------------------------------------------------------------ the view

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

function clamp(v, lo, hi) {
  return Math.min(hi, Math.max(lo, v));
}

canvas.addEventListener(
  "wheel",
  (e) => {
    e.preventDefault();
    if (!view) return;
    const r = canvas.getBoundingClientRect();
    const dy = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaMode === 2 ? e.deltaY * r.height : e.deltaY;
    zoomAt(e.clientX - r.left, e.clientY - r.top, Math.exp(-dy * 0.0015));
  },
  { passive: false },
);

canvas.addEventListener("pointerdown", (e) => {
  if (!view || (e.button !== 0 && e.button !== 1)) return;
  e.preventDefault();
  canvas.setPointerCapture(e.pointerId);
  drag = { id: e.pointerId, x: e.clientX, y: e.clientY };
  canvas.classList.add("dragging");
});

canvas.addEventListener("pointermove", (e) => {
  if (!drag || e.pointerId !== drag.id) return;
  view = { ...view, x: view.x - (e.clientX - drag.x) / view.k, y: view.y - (e.clientY - drag.y) / view.k };
  drag.x = e.clientX;
  drag.y = e.clientY;
  apply();
});

function endDrag(e) {
  if (!drag || e.pointerId !== drag.id) return;
  drag = null;
  canvas.classList.remove("dragging");
}
canvas.addEventListener("pointerup", endDrag);
canvas.addEventListener("pointercancel", endDrag);
// no autoscroll on the middle button, no page menu on the drawing
canvas.addEventListener("mousedown", (e) => { if (e.button === 1) e.preventDefault(); });
canvas.addEventListener("contextmenu", (e) => e.preventDefault());
canvas.addEventListener("dblclick", fit);

// keep the middle of the view in place when the window or the panel changes size
new ResizeObserver(() => {
  if (!view) return;
  const now = canvasSize();
  view = { ...view, x: view.x + (size.w - now.w) / (2 * view.k), y: view.y + (size.h - now.h) / (2 * view.k) };
  apply();
}).observe(canvas);

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
    const sev = span("sev", d.severity);
    const msg = span("msg", d.message);
    const loc = span("loc", d.file ? `${d.file}:${d.line}${d.col ? `:${d.col}` : ""}` : "");
    if (d.full_path) loc.title = d.full_path;
    li.append(sev, msg, loc);
    if (d.origin) li.append(span("more", d.origin));
    if (d.cause) li.append(span("more", `upstream cause: ${d.cause}`));
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

function span(cls, text) {
  const s = document.createElement("span");
  s.className = cls;
  s.textContent = text;
  return s;
}

function setPanelOpen(open) {
  $("panel-head").setAttribute("aria-expanded", String(open));
  try {
    localStorage.setItem("panel-open", String(open));
  } catch {
    // the panel just opens expanded next time
  }
}

$("panel-head").addEventListener("click", () => {
  setPanelOpen($("panel-head").getAttribute("aria-expanded") !== "true");
});

// ------------------------------------------------------------------ commands

$("open").addEventListener("click", pickAndOpen);
$("reload").addEventListener("click", reload);
$("fit").addEventListener("click", fit);

document.addEventListener("keydown", (e) => {
  const ctrl = e.ctrlKey || e.metaKey;
  if (ctrl && e.key.toLowerCase() === "o") {
    e.preventDefault();
    pickAndOpen();
  } else if (e.key === "F5" || (ctrl && e.key.toLowerCase() === "r")) {
    e.preventDefault(); // re-read the file instead of reloading the page
    reload();
  } else if (!ctrl && !e.altKey && (e.key === "f" || e.key === "F" || e.key === "0")) {
    fit();
  }
});

try {
  if (localStorage.getItem("panel-open") === "false") $("panel-head").setAttribute("aria-expanded", "false");
} catch {
  // keep it expanded
}

const noFile = document.createElement("li");
noFile.className = "nothing";
noFile.textContent = "No file is open.";
$("diags").append(noFile);
invoke("initial_file").then((path) => openPath(path, false));
