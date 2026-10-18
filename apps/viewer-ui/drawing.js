// The drawing: one level at a time, as the SVG the core drew. The view moves by changing the SVG's
// viewBox; a click selects what it hits, found from the boxes, pins, wires and labels of the level
// (hits.js), or lists what it hits for the user to choose (picker.js); the pointer resting on an
// instance or a net shows its name (hover.js). The selection is marked over the drawing, with the
// labels attached to it, and a selected subcircuit instance gets the open badge. The grid under it
// follows the view, and the status bar shows where the pointer is in the scene.

import { current, doc, selected, setCurrent, setPath } from "./state.js";
import { paintGrid } from "./grid.js";
import { updateHierarchy } from "./hierarchy.js";
import { boxAt, hitsAt, nameOf, objectsOf } from "./hits.js";
import { hideTip, hoverNamesOn, showTip } from "./hover.js";
import { closePicker, openPicker } from "./picker.js";
import { clearSelection, select } from "./selection.js";
import { descend, opens, updatePath } from "./subcircuits.js";
import { markUnplaced, scopeSource } from "./text.js";
import { renderUnplaced } from "./unplaced.js";
import { $, clamp, svgElement } from "./util.js";

const canvas = $("canvas");

const MIN_ZOOM = 0.02;
const MAX_ZOOM = 50;
const FIT_MARGIN = 24; // pixels around the drawing when it is fitted
const CLICK_SLOP = 4; // pixels a press may move and still be a click
let svg = null; // the <svg> element of the level drawn
let overlay = null; // the selection marks drawn over it
let groups = new Map(); // data-id -> the SVG groups that draw it
let extent = null; // the drawing's own viewBox: {x, y, w, h}
export let view = null; // the view: top-left corner in drawing units, and pixels per unit (k)
let scale = 1; // drawing units (SVG pixels) per unit of the scene: the level's scale, from the Document
let size = { w: 0, h: 0 }; // the canvas size the view was last applied to
let press = null; // a mouse press on the drawing: where, and whether it became a drag
let pointer = null; // where the pointer is on the canvas, in its pixels, while it is over it
let badgeAt = null; // the drawing point of the open badge: a selected subcircuit's box corner

/**
 * Draw level `i`, with a view kept for it or else fitted. The hierarchy path that leads to it is set
 * already (subcircuits.js); the bar above the drawing shows it.
 */
export function showLevel(i, keptView) {
  setCurrent(i);
  notice("");
  const lv = doc.levels[i];
  $("level-kind").textContent = i === 0 ? "top level" : lv.kind === "subckt" ? "subcircuit" : "file level";
  $("status").textContent = lv.status ? `Not drawn without a solver: ${lv.status}` : "";
  $("status").title = $("status").textContent;
  if (svg) svg.remove();
  svg = null;
  overlay = null;
  groups = new Map();
  closePicker();
  hideTip();
  updateHierarchy();
  updatePath();
  scopeSource(i);
  markUnplaced(i);
  renderUnplaced();
  badgeAt = null;
  positionBadge();
  scale = lv.scale;
  if (lv.empty) {
    view = null;
    paintGrid(null);
    showPointer();
    $("zoom").textContent = "";
    $("empty").textContent = emptyMessage(lv);
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

/**
 * Why a level shows nothing: its objects have no position yet, or it has no objects at all, when
 * the netlist's other levels are in the Hierarchy.
 */
function emptyMessage(lv) {
  if (lv.unplaced.length > 0) return "Nothing on this level has a position yet, so nothing is drawn.";
  const none = "This level has no objects, so nothing is drawn.";
  return doc.levels.length > 1 ? `${none} The subcircuits of this netlist are in the Hierarchy.` : none;
}

/** The line in the status bar that says why this level is shown; showing another level takes it away. */
export function notice(text) {
  const el = $("notice");
  el.textContent = text;
  el.hidden = text === "";
}

export function clearDrawing(message) {
  notice("");
  closePicker();
  hideTip();
  if (svg) svg.remove();
  svg = null;
  overlay = null;
  view = null;
  $("empty").textContent = message;
  $("empty").hidden = false;
  setPath([]);
  updatePath();
  $("level-kind").textContent = "";
  badgeAt = null;
  positionBadge();
  paintGrid(null);
  showPointer();
  $("status").textContent = "";
  $("zoom").textContent = "";
  $("selection").textContent = "";
  renderUnplaced();
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
    hideTip();
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
  hideTip();
  canvas.focus({ preventScroll: true }); // so that Enter and Backspace act on the drawing
  canvas.setPointerCapture(e.pointerId);
  press = { id: e.pointerId, button: e.button, x0: e.clientX, y0: e.clientY, x: e.clientX, y: e.clientY, drag: false };
});

canvas.addEventListener("pointermove", (e) => {
  pointer = canvasPoint(e);
  showPointer();
  if (!press || e.pointerId !== press.id) {
    if (!press && view) pointOut(e);
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
  const { hits, blocked } = hitsAt(doc && doc.levels[current], view, canvasPoint(e));
  const objects = objectsOf(hits);
  if (objects.length > 1) {
    hideTip();
    openPicker(objects, e.clientX, e.clientY, (o) => select(o, "drawing"));
  } else if (objects.length === 1) select(objects[0], "drawing");
  else if (!blocked) clearSelection();
}

/**
 * The pointer moved over the drawing with no button down: the cursor says whether a click selects
 * something, and, if names are shown on hover, the box beside the pointer names what it rests on.
 */
function pointOut(e) {
  const { hits } = hitsAt(doc && doc.levels[current], view, canvasPoint(e));
  canvas.classList.toggle("over-object", hits.length > 0);
  const name = hoverNamesOn() ? nameOf(doc, hits[0]) : null;
  if (name === null) hideTip();
  else showTip(name, e.clientX, e.clientY);
}
canvas.addEventListener("pointerup", (e) => endPress(e, false));
canvas.addEventListener("pointercancel", (e) => endPress(e, true));
canvas.addEventListener("pointerleave", () => {
  pointer = null;
  showPointer();
  hideTip();
});
// no autoscroll on the middle button, no page menu on the drawing
canvas.addEventListener("mousedown", (e) => { if (e.button === 1) e.preventDefault(); });
canvas.addEventListener("contextmenu", (e) => e.preventDefault());
// a double click opens a subcircuit instance, and fits the drawing anywhere else
canvas.addEventListener("dblclick", (e) => {
  if (e.target.closest("#enter-badge")) return;
  const box = boxAt(doc && doc.levels[current], view, canvasPoint(e));
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

// ------------------------------------------------------------------ the selection

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
    if (h.kind === "label") {
      // the labels of the selected object are marked too, also those whose group is not theirs alone
      const [[x0, y0], [x1, y1]] = h.points;
      const r = svgElement("rect", { x: x0 - 1.5, y: y0 - 1.5, width: x1 - x0 + 3, height: y1 - y0 + 3, class: "label" });
      overlay.append(r);
    } else if (h.kind === "box" && isInstance) {
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
