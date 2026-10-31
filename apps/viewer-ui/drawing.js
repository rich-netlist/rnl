import { append, m, setAttr, setText } from "./i18n.js";
// The drawing: one level at a time, as the SVG the core drew. The view moves by changing the SVG's
// viewBox; a click selects what it hits, found from the boxes, pins, wires and labels of the level
// (hits.js), or lists what it hits for the user to choose (picker.js); the pointer resting on an
// instance or a net shows its name (hover.js). The selection is marked over the drawing, with the
// labels attached to it, and a selected subcircuit instance gets the open badge. The grid under it
// follows the view, and the status bar shows where the pointer is in the scene.

import { current, doc, selected, selectedAsUsage, setCurrent, setPath } from "./state.js";
import { colourDrawing } from "./background.js";
import { paintGrid } from "./grid.js";
import { updateHierarchy } from "./hierarchy.js";
import { boxAt, hitsAt, nameOf, objectsOf } from "./hits.js";
import { hideTip, hoverNamesOn, hoverRolesOn, showTip } from "./hover.js";
import { roleNote } from "./usage-table.js";
import { closePicker, openPicker } from "./picker.js";
import { clearSelection, select } from "./selection.js";
import { canOpen, descend, updatePath } from "./subcircuits.js";
import { hideParams, updateParams } from "./params.js";
import { markUnplaced, scopeSource } from "./text.js";
import { renderUnplaced } from "./unplaced.js";
import { $, clamp, svgElement } from "./util.js";
import { paintUsages, usageAt, usagesAt } from "./usages.js";
import { TOUCH_CLICK_MS, touchGestures } from "./touch-gestures.js";

const canvas = $("canvas");

const MIN_ZOOM = 0.02;
const MAX_ZOOM = 50;
const FIT_MARGIN = 24; // pixels around the drawing when it is fitted
const CLICK_SLOP = 4; // pixels a press may move and still be a click
const BADGE_GAP = 4; // CSS pixels from the box's top-right corner to the badge's bottom-left
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
const zoomBox = document.createElement("div");
zoomBox.className = "zoom-box";
zoomBox.hidden = true;
append(canvas, zoomBox);

/**
 * Draw level `i`, with a view kept for it or else fitted. The hierarchy path that leads to it is set
 * already (subcircuits.js); the bar above the drawing shows it.
 */
export function showLevel(i, keptView) {
  cancelGesture();
  setCurrent(i);
  notice("");
  const lv = doc.levels[i];
  setText($("level-kind"), i === 0 ? m("top level") : lv.kind === "subckt" ? m("subcircuit") : m("file level"));
  setText($("status"), lv.status ? m`Not drawn without a solver: ${lv.status}` : "");
  setAttr($("status"), "title", lv.status ? m`Not drawn without a solver: ${lv.status}` : "");
  if (svg) svg.remove();
  svg = null;
  overlay = null;
  groups = new Map();
  closePicker();
  hideTip();
  updateHierarchy();
  updatePath();
  updateParams();
  scopeSource(i);
  markUnplaced(i);
  renderUnplaced();
  badgeAt = null;
  positionBadge();
  scale = lv.scale;
  if (lv.empty) {
    paintUsages(null, null, groups);
    view = null;
    paintGrid(null);
    showPointer();
    setText($("zoom"), "");
    setText($("empty"), emptyMessage(lv));
    $("empty").hidden = false;
    window.dispatchEvent(new Event("viewer-level"));
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
  colourDrawing(el);
  for (const g of el.querySelectorAll("[data-id]")) {
    const id = g.getAttribute("data-id");
    if (!groups.has(id)) groups.set(id, []);
    groups.get(id).push(g);
  }
  overlay = document.createElementNS("http://www.w3.org/2000/svg", "g");
  overlay.setAttribute("class", "overlay");
  overlay.setAttribute("pointer-events", "none");
  append(el, overlay);
  $("empty").hidden = true;
  svg = el;
  append(canvas, svg);
  if (keptView) {
    view = keptView;
    apply();
  } else {
    fit();
  }
  paintDrawingSelection();
  window.dispatchEvent(new Event("viewer-level"));
}

/**
 * Why a level shows nothing: its objects have no position yet, or it has no objects at all, when
 * the netlist's other levels are in the Hierarchy.
 */
function emptyMessage(lv) {
  // the file opened is no netlist, which the core does not read
  if (doc.dialect === "text") return m("This file is not a netlist. It is shown as text.");
  if (lv.unplaced.length > 0) return m("Nothing on this level has a position yet, so nothing is drawn.");
  const none = m("This level has no objects, so nothing is drawn.");
  return doc.levels.length > 1 ? m`${none} The subcircuits of this netlist are in the Hierarchy.` : none;
}

/** The line in the status bar that says why this level is shown; showing another level takes it away. */
export function notice(text) {
  const el = $("notice");
  setText(el, text);
  el.hidden = text === "";
}

export function clearDrawing(message) {
  paintUsages(null, null, groups);
  cancelGesture();
  notice("");
  closePicker();
  hideTip();
  if (svg) svg.remove();
  svg = null;
  overlay = null;
  view = null;
  setText($("empty"), message);
  $("empty").hidden = false;
  setPath([]);
  updatePath();
  hideParams();
  setText($("level-kind"), "");
  badgeAt = null;
  positionBadge();
  paintGrid(null);
  showPointer();
  setText($("status"), "");
  setText($("zoom"), "");
  setText($("selection"), "");
  renderUnplaced();
  window.dispatchEvent(new Event("viewer-level"));
}

function canvasSize() {
  return { w: canvas.clientWidth || 1, h: canvas.clientHeight || 1 };
}

function apply() {
  if (!svg || !view) return;
  size = canvasSize();
  svg.setAttribute("viewBox", `${view.x} ${view.y} ${size.w / view.k} ${size.h / view.k}`);
  setText($("zoom"), `${Math.round(view.k * 100)}%`);
  positionBadge();
  paintGrid(view, scale);
  paintUsages(svg, view, groups);
  showPointer();
}

/**
 * Show where the pointer is in the scene, x and y with y up, to the digits the zoom tells apart: the
 * last is worth 1 to 10 pixels. Nothing while the pointer is off the drawing.
 */
function showPointer() {
  const el = $("pointer");
  if (!pointer || !view) {
    setText(el, "");
    return;
  }
  const digits = clamp(Math.ceil(Math.log10(view.k * scale)) - 1, 0, 3);
  // no "-0.0" for a point a little below 0
  const fixed = (v) => (Number(v.toFixed(digits)) || 0).toFixed(digits);
  const x = (view.x + pointer.x / view.k) / scale;
  const y = -(view.y + pointer.y / view.k) / scale;
  setText(el, `x ${fixed(x)}, y ${fixed(y)}`);
}

// Compact embeds frame painted ink instead of the renderer's paper margin and font line boxes.
// Text's getBBox includes the font's ascender/descender space, even for three capital letters.
let inkMeasure = null;
function inkExtent() {
  const inverse = svg.getCTM()?.inverse();
  if (!inverse) return extent;
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (const el of svg.querySelectorAll("path, circle, ellipse, line, polyline, polygon, rect, text")) {
    if (el.classList.contains("backdrop") || el.closest(".overlay, .usage-overlays, .usage-underlays")) continue;
    const style = getComputedStyle(el);
    if (style.display === "none" || style.visibility === "hidden" || (style.fill === "none" && style.stroke === "none")) continue;
    let { x, y, width, height } = el.getBBox();
    if (el.localName === "text") {
      inkMeasure ||= document.createElement("canvas").getContext("2d");
      inkMeasure.font = `${style.fontStyle} ${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
      inkMeasure.fontKerning = "none";
      const metrics = inkMeasure.measureText(el.textContent);
      const advance = el.textLength.baseVal.value || metrics.width;
      const stretch = metrics.width ? advance / metrics.width : 1;
      const anchor = style.textAnchor === "middle" ? advance / 2 : style.textAnchor === "end" ? advance : 0;
      x = (el.x.baseVal[0]?.value || 0) - anchor - metrics.actualBoundingBoxLeft * stretch;
      y = (el.y.baseVal[0]?.value || 0) - metrics.actualBoundingBoxAscent;
      width = (metrics.actualBoundingBoxLeft + metrics.actualBoundingBoxRight) * stretch;
      height = metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent;
    }
    const half = style.stroke === "none" ? 0 : parseFloat(style.strokeWidth) / 2;
    if (!width && !height && !half) continue;
    const matrix = inverse.multiply(el.getCTM());
    for (const point of [[x - half, y - half], [x + width + half, y - half],
      [x - half, y + height + half], [x + width + half, y + height + half]]) {
      const p = new DOMPoint(...point).matrixTransform(matrix);
      x0 = Math.min(x0, p.x); y0 = Math.min(y0, p.y);
      x1 = Math.max(x1, p.x); y1 = Math.max(y1, p.y);
    }
  }
  return x1 > x0 && y1 > y0 ? { x: x0, y: y0, w: x1 - x0, h: y1 - y0 } : extent;
}

export function fit() {
  cancelGesture();
  if (!svg) return;
  const { w, h } = canvasSize();
  const style = getComputedStyle(canvas);
  const padding = parseFloat(style.getPropertyValue("--fit-margin"));
  const margin = Number.isFinite(padding) ? Math.max(0, padding) : FIT_MARGIN;
  const box = style.getPropertyValue("--fit-content").trim() === "1" ? inkExtent() : extent;
  const k = clamp(Math.min((w - 2 * margin) / box.w, (h - 2 * margin) / box.h), MIN_ZOOM, MAX_ZOOM);
  view = { k, x: box.x - (w / k - box.w) / 2, y: box.y - (h / k - box.h) / 2 };
  apply();
}

/**
 * The view as a manifest records it (export of a project's display settings): the point at the
 * centre of the canvas, in the drawing's units, and the pixels per unit; null with no drawing.
 */
export function viewCentre() {
  if (!view) return null;
  const { w, h } = canvasSize();
  return { center: [view.x + w / (2 * view.k), view.y + h / (2 * view.k)], scale: view.k };
}

/** Show the drawing centred on `center`, in its units, at `scale` pixels per unit, as a manifest names a view. */
export function showView({ center, scale }) {
  cancelGesture();
  if (!svg) return;
  const { w, h } = canvasSize();
  const k = clamp(scale, MIN_ZOOM, MAX_ZOOM);
  view = { k, x: center[0] - w / (2 * k), y: center[1] - h / (2 * k) };
  apply();
}

/** Bring the selection into the middle of the view, zoomed out as far as it needs to be all in it. */
export function showSelection() {
  cancelGesture();
  if (!svg || !view || selected === null) return;
  const points = doc.levels[current].hits.filter((h) => h.objects.includes(selected)).flatMap((h) => h.points);
  if (!points.length) return;
  const xs = points.map((p) => p[0]), ys = points.map((p) => p[1]);
  const x0 = Math.min(...xs), y0 = Math.min(...ys), x1 = Math.max(...xs), y1 = Math.max(...ys);
  const { w, h } = canvasSize();
  const k = clamp(Math.min(view.k, (w - 2 * FIT_MARGIN) / (x1 - x0 || 1), (h - 2 * FIT_MARGIN) / (y1 - y0 || 1)), MIN_ZOOM, MAX_ZOOM);
  view = { k, x: (x0 + x1) / 2 - w / (2 * k), y: (y0 + y1) / 2 - h / (2 * k) };
  apply();
}

/** Zoom by `factor` keeping the drawing point under (px, py), in canvas pixels, in place. */
function zoomAt(px, py, factor) {
  cancelGesture();
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

/** Move the viewing window by a fraction of its size; arrows point toward the scene to reveal. */
export function pan(dx, dy, fraction) {
  if (!view || press) return false;
  const { w, h } = canvasSize();
  view = { ...view, x: view.x + dx * w * fraction / view.k, y: view.y + dy * h * fraction / view.k };
  hideTip();
  closePicker();
  apply();
  return true;
}

/** Abandon a drag without selecting or changing a right-button zoom rectangle's original view. */
export function cancelGesture() {
  if (!press) return false;
  const id = press.id;
  press = null;
  zoomBox.hidden = true;
  canvas.classList.remove("dragging");
  if (canvas.hasPointerCapture(id)) canvas.releasePointerCapture(id);
  return true;
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
  if (e.pointerType === "touch") { gestures.down(e); return; }
  if (!view || press || ![0, 1, 2].includes(e.button)) return;
  e.preventDefault();
  hideTip();
  canvas.focus({ preventScroll: true }); // so that Enter and Backspace act on the drawing
  canvas.setPointerCapture(e.pointerId);
  press = { id: e.pointerId, button: e.button, x0: e.clientX, y0: e.clientY, x: e.clientX, y: e.clientY, drag: false };
});

canvas.addEventListener("pointermove", (e) => {
  if (e.pointerType === "touch") { gestures.move(e); return; }
  pointer = canvasPoint(e);
  showPointer();
  if (!press || e.pointerId !== press.id) {
    if (!press && view) pointOut(e, pointer);
    return;
  }
  if (!press.drag && Math.hypot(e.clientX - press.x0, e.clientY - press.y0) > CLICK_SLOP) {
    press.drag = true;
    if (press.button !== 2) canvas.classList.add("dragging");
  }
  if (!press.drag) return;
  if (press.button === 2) {
    const r = canvas.getBoundingClientRect();
    const x = clamp(e.clientX, r.left, r.right), y = clamp(e.clientY, r.top, r.bottom);
    zoomBox.hidden = false;
    Object.assign(zoomBox.style, {
      left: `${Math.min(press.x0, x) - r.left}px`, top: `${Math.min(press.y0, y) - r.top}px`,
      width: `${Math.abs(x - press.x0)}px`, height: `${Math.abs(y - press.y0)}px`,
    });
    return;
  }
  view = { ...view, x: view.x - (e.clientX - press.x) / view.k, y: view.y - (e.clientY - press.y) / view.k };
  press.x = e.clientX;
  press.y = e.clientY;
  apply();
});

function endPress(e, cancelled) {
  if (!press || e.pointerId !== press.id) return;
  const click = !press.drag && press.button === 0 && !cancelled;
  if (press.button === 2 && press.drag && !cancelled) {
    const r = canvas.getBoundingClientRect();
    const x = clamp(e.clientX, r.left, r.right), y = clamp(e.clientY, r.top, r.bottom);
    const w = Math.abs(x - press.x0), h = Math.abs(y - press.y0);
    if (w > CLICK_SLOP && h > CLICK_SLOP) {
      const { w: cw, h: ch } = canvasSize();
      const cx = view.x + ((x + press.x0) / 2 - r.left) / view.k;
      const cy = view.y + ((y + press.y0) / 2 - r.top) / view.k;
      const k = clamp(view.k * Math.min(cw / w, ch / h), MIN_ZOOM, MAX_ZOOM);
      view = { k, x: cx - cw / (2 * k), y: cy - ch / (2 * k) };
      apply();
    }
  }
  cancelGesture();
  if (click) clickAt(e);
}

/** A click, or a tap, at the event's point: select what is there, or ask which, or clear the selection. */
function clickAt(e) {
  const { hits, blocked } = hitsAt(doc && doc.levels[current], view, canvasPoint(e));
  const objects = hits.length ? objectsOf(hits) : !blocked ? usagesAt(canvasPoint(e)) : [];
  if (e.shiftKey && objects.some((o) => canOpen(o))) return; // Shift-double-click leaves this viewer unchanged
  if (objects.length > 1) {
    hideTip();
    openPicker(objects, e.clientX, e.clientY, (o) => select(o, "drawing"));
  } else if (objects.length === 1) select(objects[0], "drawing");
  else if (!blocked) clearSelection();
}

/** A double click, or a double tap: open the subcircuit instance there, or else fit the drawing. */
function doubleAt(e) {
  const box = boxAt(doc && doc.levels[current], view, canvasPoint(e));
  const o = box ? box.objects.find((k) => canOpen(k)) : undefined;
  if (o !== undefined) descend(o, { newWindow: e.shiftKey });
  else fit();
}

/**
 * Touch (pointerType "touch"), as a phone or a tablet gives it, is the gestures of touch-gestures.js,
 * shared with the preview of a symbol: a tap does what a click does here, a double tap what a double
 * click does, once.
 */
const gestures = touchGestures({
  element: canvas,
  view: () => view,
  setView: (v) => { view = v; apply(); },
  min: MIN_ZOOM,
  max: MAX_ZOOM,
  onDown: hideTip,
  onTap: clickAt,
  onDoubleTap: doubleAt,
});

/**
 * The pointer moved over the drawing with no button down: the cursor says whether a click selects
 * something, and, if names are shown on hover, the box beside the pointer names what it rests on.
 */
function pointOut(e, point) {
  // a move the browser makes of a tap names nothing: a finger does not hover
  if (performance.now() - gestures.touchedAt() < TOUCH_CLICK_MS) { hideTip(); return; }
  const { hits } = hitsAt(doc && doc.levels[current], view, point);
  canvas.classList.toggle("over-object", hits.length > 0 || usageAt(point) !== null);
  const name = hoverNamesOn() ? nameOf(doc, hits[0]) : null;
  if (name === null) { hideTip(); return; }
  // with View > Show Roles on Hover, the role of the object in its first usage follows its name
  const note = hoverRolesOn() ? roleNote(doc, hits[0].objects[0]) : null;
  showTip(note === null ? name : `${name} · ${note}`, e.clientX, e.clientY);
}
// a finger that lifts names nothing: a finger does not hover
const touchUp = (e, cancelled) => { gestures.up(e, cancelled); hideTip(); };
canvas.addEventListener("pointerup", (e) => (e.pointerType === "touch" ? touchUp(e, false) : endPress(e, false)));
canvas.addEventListener("pointercancel", (e) => (e.pointerType === "touch" ? touchUp(e, true) : endPress(e, true)));
canvas.addEventListener("lostpointercapture", (e) => (e.pointerType === "touch" ? touchUp(e, true) : endPress(e, true)));
window.addEventListener("blur", cancelGesture);
canvas.addEventListener("pointerleave", () => {
  pointer = null;
  showPointer();
  hideTip();
});
// no autoscroll on the middle button, no page menu on the drawing
canvas.addEventListener("mousedown", (e) => { if (e.button === 1) e.preventDefault(); });
canvas.addEventListener("contextmenu", (e) => e.preventDefault());
// a double click opens a subcircuit instance, and fits the drawing anywhere else; one the browser
// makes of two taps is left out, as the page told the double tap itself
canvas.addEventListener("dblclick", (e) => {
  if (e.target.closest("#enter-badge") || performance.now() - gestures.touchedAt() < TOUCH_CLICK_MS) return;
  doubleAt(e);
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
  lower();
  if (!svg) return;
  for (const g of svg.querySelectorAll(".sel")) g.classList.remove("sel");
  overlay.replaceChildren();
  paintUsages(svg, view, groups);
  if (selected === null) return;
  const kind = doc.objects[selected].kind;
  const isInstance = kind === "instance";
  for (const h of doc.levels[current].hits) {
    if (!h.objects.includes(selected)) continue;
    if (h.id) for (const g of groups.get(h.id) || []) g.classList.add("sel");
    if (h.kind === "label") {
      // the labels of the selected object are marked too, also those whose group is not theirs alone
      const [[x0, y0], [x1, y1]] = h.points;
      const r = svgElement("rect", { x: x0 - 1.5, y: y0 - 1.5, width: x1 - x0 + 3, height: y1 - y0 + 3, class: "label" });
      append(overlay, r);
    } else if (h.kind === "box" && h.objects[0] === selected) {
      const [[x0, y0], [x1, y1]] = h.points;
      if (isInstance && canOpen(selected) && !badgeAt) badgeAt = [x1, y0];
      const r = svgElement("rect", { x: x0, y: y0, width: x1 - x0, height: y1 - y0 });
      r.setAttribute("vector-effect", "non-scaling-stroke");
      append(overlay, r);
    } else if (h.kind === "pin" && kind === "net") {
      const c = svgElement("circle", { cx: h.points[0][0], cy: h.points[0][1], r: 3.2 });
      c.setAttribute("vector-effect", "non-scaling-stroke");
      append(overlay, c);
    }
  }
  // the object and its texts are drawn over the usages' frames and titles, not under them; a selected usage's own frame
  // is already over the drawing
  if (!selectedAsUsage) lift();
  stack();
  positionBadge();
}

/**
 * The selected object's groups of the drawing, with a comment in the place of each in the drawing, so that they go back
 * to it. They are taken out of the drawing and put over the layer of the usages (usages.js).
 */
let lifted = [];
function lower() {
  for (const [g, mark] of lifted) mark.replaceWith(g);
  lifted = [];
}
function lift() {
  for (const g of svg.querySelectorAll(":scope > g.sel")) {
    const mark = document.createComment("");
    g.replaceWith(mark);
    lifted.push([g, mark]);
  }
}
/** The order of the top of the drawing: the usages' layer, over it what is lifted and the marks of the selection, the currents last. */
function stack() {
  if (!svg) return;
  const top = svg.querySelector(":scope > .current-overlays");
  for (const [g] of lifted) svg.insertBefore(g, top);
  svg.insertBefore(overlay, top);
}
// the layer of the usages is made again when a usage is shown or hidden, over what was lifted
window.addEventListener("usage-layer", stack);

/** Put the open badge on its corner of the drawing, at the same size whatever the zoom. */
function positionBadge() {
  const badge = $("enter-badge");
  if (!badgeAt || !view || !svg) {
    badge.hidden = true;
    return;
  }
  badge.hidden = false;
  badge.style.left = `${(badgeAt[0] - view.x) * view.k + BADGE_GAP}px`;
  badge.style.top = `${(badgeAt[1] - view.y) * view.k - BADGE_GAP}px`;
}

$("enter-badge").addEventListener("pointerdown", (e) => e.stopPropagation());
$("enter-badge").addEventListener("click", (e) => descend(selected, { newWindow: e.shiftKey }));
