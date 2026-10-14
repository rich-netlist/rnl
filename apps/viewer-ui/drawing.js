// The drawing: one level at a time, as the SVG the core drew. The view moves by changing the SVG's
// viewBox; a click selects what it hits, found from the boxes, pins and wires of the level; the
// selection is marked over the drawing, and a selected subcircuit instance gets the open badge. The
// grid under it follows the view, and the status bar shows where the pointer is in the scene.

import { current, doc, selected, setCurrent, setTreeActive, treeActive } from "./state.js";
import { paintGrid } from "./grid.js";
import { updateHierarchy } from "./hierarchy.js";
import { clearSelection, select } from "./selection.js";
import { descend, opens, updateUp } from "./subcircuits.js";
import { $, clamp, svgElement } from "./util.js";

const canvas = $("canvas");

const MIN_ZOOM = 0.02;
const MAX_ZOOM = 50;
const FIT_MARGIN = 24; // pixels around the drawing when it is fitted
const CLICK_SLOP = 4; // pixels a press may move and still be a click
const HIT_SLOP = 6; // pixels around a pin or a wire that still hit it
// Pixels of the SVG and the hits per unit of the scene: the level's scale, which the Document gives
// with the primitives, or else the renderer's, which the core draws the SVG at by default.
const SVG_SCALE = 14;

let svg = null; // the <svg> element of the level drawn
let overlay = null; // the selection marks drawn over it
let groups = new Map(); // data-id -> the SVG groups that draw it
let extent = null; // the drawing's own viewBox: {x, y, w, h}
export let view = null; // the view: top-left corner in drawing units, and pixels per unit (k)
let scale = SVG_SCALE; // drawing units (SVG pixels) per unit of the scene, on the level drawn
let size = { w: 0, h: 0 }; // the canvas size the view was last applied to
let press = null; // a mouse press on the drawing: where, and whether it became a drag
let pointer = null; // where the pointer is on the canvas, in its pixels, while it is over it
let badgeAt = null; // the drawing point of the open badge: a selected subcircuit's box corner

/** Draw level `i`. `treePath` is the hierarchy row that asked for it, if one did. */
export function showLevel(i, keptView, treePath) {
  setCurrent(i);
  const lv = doc.levels[i];
  setTreeActive(treePath !== undefined ? treePath : treeActive);
  $("level-name").textContent = lv.title;
  $("level-kind").textContent = i === 0 ? "top level" : lv.kind === "subckt" ? "subcircuit" : "file level";
  $("status").textContent = lv.status ? `Not drawn without a solver: ${lv.status}` : "";
  $("status").title = $("status").textContent;
  if (svg) svg.remove();
  svg = null;
  overlay = null;
  groups = new Map();
  updateHierarchy();
  updateUp();
  badgeAt = null;
  positionBadge();
  scale = lv.scale || SVG_SCALE;
  if (lv.empty) {
    view = null;
    paintGrid(null);
    showPointer();
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

export function clearDrawing(message) {
  if (svg) svg.remove();
  svg = null;
  overlay = null;
  view = null;
  $("empty").textContent = message;
  $("empty").hidden = false;
  $("level-name").textContent = "";
  $("level-kind").textContent = "";
  $("up").hidden = true;
  badgeAt = null;
  positionBadge();
  paintGrid(null);
  showPointer();
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
  positionBadge();
  paintGrid(view, scale);
  showPointer();
}

/**
 * Show where the pointer is in the scene, x and y with y up, to the digits the zoom tells apart: the
 * last is worth 1 to 10 pixels. Nothing while the pointer is off the drawing.
 */
function showPointer() {
  const el = $("pointer");
  if (!pointer || !view) {
    el.textContent = "";
    return;
  }
  const digits = clamp(Math.ceil(Math.log10(view.k * scale)) - 1, 0, 3);
  // no "-0.0" for a point a little below 0
  const fixed = (v) => (Number(v.toFixed(digits)) || 0).toFixed(digits);
  const x = (view.x + pointer.x / view.k) / scale;
  const y = -(view.y + pointer.y / view.k) / scale;
  el.textContent = `x ${fixed(x)}, y ${fixed(y)}`;
}

export function fit() {
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

export function zoomCenter(factor) {
  const { w, h } = canvasSize();
  zoomAt(w / 2, h / 2, factor);
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
  canvas.focus({ preventScroll: true }); // so that Enter and Backspace act on the drawing
  canvas.setPointerCapture(e.pointerId);
  press = { id: e.pointerId, button: e.button, x0: e.clientX, y0: e.clientY, x: e.clientX, y: e.clientY, drag: false };
});

canvas.addEventListener("pointermove", (e) => {
  pointer = canvasPoint(e);
  showPointer();
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
canvas.addEventListener("pointerleave", () => {
  pointer = null;
  showPointer();
});
// no autoscroll on the middle button, no page menu on the drawing
canvas.addEventListener("mousedown", (e) => { if (e.button === 1) e.preventDefault(); });
canvas.addEventListener("contextmenu", (e) => e.preventDefault());
// a double click opens a subcircuit instance, and fits the drawing anywhere else
canvas.addEventListener("dblclick", (e) => {
  if (e.target.closest("#enter-badge")) return;
  const box = boxAt(canvasPoint(e));
  const o = box ? box.objects.find((k) => opens(k) !== null) : undefined;
  if (o !== undefined) descend(o);
  else fit();
});

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

/** The smallest symbol box around a canvas point, whatever pin or wire is on it; or null. */
function boxAt(p) {
  if (!view || !doc || !doc.levels[current]) return null;
  const u = view.x + p.x / view.k;
  const v = view.y + p.y / view.k;
  let best = null;
  let bestArea = Infinity;
  for (const h of doc.levels[current].hits) {
    if (h.kind !== "box" || h.objects.length === 0) continue;
    const [[x0, y0], [x1, y1]] = h.points;
    if (u < x0 || u > x1 || v < y0 || v > y1) continue;
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

export function paintDrawingSelection() {
  badgeAt = null;
  positionBadge();
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
      if (opens(selected) !== null && !badgeAt) badgeAt = [x1 + pad, y0 - pad];
      const r = svgElement("rect", { x: x0 - pad, y: y0 - pad, width: x1 - x0 + 2 * pad, height: y1 - y0 + 2 * pad });
      r.setAttribute("vector-effect", "non-scaling-stroke");
      overlay.append(r);
    } else if (h.kind === "pin" && !isInstance) {
      const c = svgElement("circle", { cx: h.points[0][0], cy: h.points[0][1], r: 3.2 });
      c.setAttribute("vector-effect", "non-scaling-stroke");
      overlay.append(c);
    }
  }
  positionBadge();
}

/** Put the open badge on its corner of the drawing, at the same size whatever the zoom. */
function positionBadge() {
  const badge = $("enter-badge");
  if (!badgeAt || !view || !svg) {
    badge.hidden = true;
    return;
  }
  badge.hidden = false;
  badge.style.left = `${(badgeAt[0] - view.x) * view.k}px`;
  badge.style.top = `${(badgeAt[1] - view.y) * view.k}px`;
}

$("enter-badge").addEventListener("pointerdown", (e) => e.stopPropagation());
$("enter-badge").addEventListener("click", () => descend(selected));
