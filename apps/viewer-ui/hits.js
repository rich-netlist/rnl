// What a point of the drawing is on. The core gives a level its hits (`Level.hits`): the boxes of the
// symbols, their pins, the wires and the labels, each standing for objects. A click selects the first
// of what is under it, in this order: a pin, a label, a wire, a symbol box. When it is on several
// objects, the list beside the pointer (picker.js) lets the user choose. Nothing here knows the DOM:
// the view is the drawing's, { x, y, k }, with the top-left corner in drawing units and the pixels
// per unit, and a point is in canvas pixels.

import { clamp } from "./util.js";

export const HIT_SLOP = 6; // pixels around a pin or a wire that still hit it
const LABEL_SLOP = 2; // pixels around a label's box that still hit it
const EDGE = 2; // pixels inside a box's outline that are the outline: a wire along it is not over the symbol

/**
 * What the canvas point `p` is on in `level`, as { hits, blocked }: the hits that stand for some
 * object, in the order a click prefers them, and whether a label that stands for nothing is there.
 * A pin or a wire is hit within HIT_SLOP pixels, a label within LABEL_SLOP, a symbol box inside it
 * by more than EDGE pixels, but for the boxes a pin is on: a pin is a part of its symbol, on its
 * outline, and a click on it is on the net. Only where nothing else is, the symbol box around the
 * point with HIT_SLOP pixels more, its outline included, is. A label
 * of something that cannot be selected is no hit, but a click on it, with nothing else under it,
 * does nothing, which `blocked` says.
 */
export function hitsAt(level, view, p) {
  const none = { hits: [], blocked: false };
  if (!level || !view) return none;
  const u = view.x + p.x / view.k;
  const v = view.y + p.y / view.k;
  const slop = HIT_SLOP / view.k;
  const labelSlop = LABEL_SLOP / view.k;
  const edge = EDGE / view.k;
  const pins = [];
  const labels = [];
  const wires = [];
  const boxes = [];
  const near = [];
  let blocked = false;
  for (const h of level.hits) {
    if (h.kind === "label") {
      const [[x0, y0], [x1, y1]] = h.points;
      if (u < x0 - labelSlop || u > x1 + labelSlop || v < y0 - labelSlop || v > y1 + labelSlop) continue;
      if (h.objects.length === 0) blocked = true;
      else labels.push([(x1 - x0) * (y1 - y0), h]);
      continue;
    }
    if (h.objects.length === 0) continue;
    if (h.kind === "pin") {
      const d = Math.hypot(u - h.points[0][0], v - h.points[0][1]);
      if (d <= slop) pins.push([d, h]);
    } else if (h.kind === "wire") {
      let d = Infinity;
      for (let k = 1; k < h.points.length; k++) d = Math.min(d, segmentDistance(u, v, h.points[k - 1], h.points[k]));
      if (d <= slop) wires.push([d, h]);
    } else if (h.kind === "box") {
      const [[x0, y0], [x1, y1]] = h.points;
      if (u < x0 - slop || u > x1 + slop || v < y0 - slop || v > y1 + slop) continue;
      const area = (x1 - x0) * (y1 - y0);
      (u < x0 + edge || u > x1 - edge || v < y0 + edge || v > y1 - edge ? near : boxes).push([area, h]);
    }
  }
  // the nearer or the smaller first; equal ones, as the boxes of two symbols alike are, in the order of the netlist
  const byKey = (a, b) => (Math.abs(a[0] - b[0]) < 1e-6 ? 0 : a[0] - b[0]);
  const onPin = ([, b]) => pins.some(([, pin]) => inside(b.points, pin.points[0]));
  const hits = [pins, labels, wires, boxes.filter((box) => !onPin(box))].flatMap((group) =>
    group.sort(byKey).map(([, h]) => h),
  );
  if (hits.length === 0 && near.length > 0) hits.push(near.sort(byKey)[0][1]);
  return { hits, blocked };
}

/** The smallest symbol box around a canvas point, whatever pin or wire is on it; or null. */
export function boxAt(level, view, p) {
  if (!level || !view) return null;
  const u = view.x + p.x / view.k;
  const v = view.y + p.y / view.k;
  let best = null;
  let bestArea = Infinity;
  for (const h of level.hits) {
    if (h.kind !== "box" || h.objects.length === 0) continue;
    const [[x0, y0], [x1, y1]] = h.points;
    if (u < x0 || u > x1 || v < y0 || v > y1) continue;
    const area = (x1 - x0) * (y1 - y0);
    if (area < bestArea) [best, bestArea] = [h, area];
  }
  return best;
}

/**
 * The objects the hits stand for, each once, in the order of the hits: the first object of each hit,
 * which is what a click on it selects. A symbol and its label, or a pin and a wire of one net, are
 * one object, so that a click on them does not ask.
 */
export function objectsOf(hits) {
  const objects = [];
  for (const h of hits) if (!objects.includes(h.objects[0])) objects.push(h.objects[0]);
  return objects;
}

/**
 * The name a pointer on `hit` shows, or null for none: an instance's, or the names of the nets of a
 * pin or a wire. A symbol that stands for a net, such as a ground or a port, is a virtual part: it
 * shows its net's name when the netlist names the part (`hit.name`), and nothing when it does not. A
 * label shows nothing, being a name already.
 */
export function nameOf(doc, hit) {
  if (!hit || hit.kind === "label" || hit.objects.length === 0) return null;
  const objects = hit.objects.map((o) => doc.objects[o]).filter(Boolean);
  if (objects.length === 0) return null;
  if (hit.kind === "box" && objects[0].kind !== "instance" && !hit.name) return null;
  return objects.map((o) => o.name).join(", ");
}

/** Whether the point is in the box given by its top-left and bottom-right corners, on its outline too. */
function inside([[x0, y0], [x1, y1]], [u, v]) {
  const eps = 1e-6;
  return u >= x0 - eps && u <= x1 + eps && v >= y0 - eps && v <= y1 + eps;
}

function segmentDistance(u, v, a, b) {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const len2 = dx * dx + dy * dy;
  const t = len2 === 0 ? 0 : clamp(((u - a[0]) * dx + (v - a[1]) * dy) / len2, 0, 1);
  return Math.hypot(u - (a[0] + t * dx), v - (a[1] + t * dy));
}
