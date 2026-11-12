// Solve > Show Component and Pin Skeleton Lines and Solve > Show Wire Skeleton Lines: thin, dashed,
// faint reference lines computed directly from the drawing geometry (skeleton-geometry.js) for any
// placed circuit. Off by default, persisted in settings, synced with the Solve panel checkboxes,
// named on hover with the components, pins or wire segments they belong to, and omitted from
// File > Export Image. `paintSkeleton` is called by `paintUsages`.

import { setCheck } from "./commands.js";
import { current, doc } from "./state.js";
import { $, load, save, svgElement } from "./util.js";
import { skeletonHit, skeletonLines } from "./skeleton-geometry.js";

export { skeletonHit, skeletonLines };

const NEAR = 4; // CSS pixels from a line within which the pointer names its owners

let symbolsShown = false;
let wiresShown = false;
let layer = null;
let shapes = []; // [line element, line]
let lines = [];
let drawn = null; // [svg, doc, level, symbolsShown, wiresShown]
let hooked = false;

export const skeletonSymbolsShown = () => symbolsShown;
export const skeletonWiresShown = () => wiresShown;

function syncControls() {
  setCheck("toggle-skeleton-symbols", symbolsShown);
  setCheck("toggle-skeleton-wires", wiresShown);
  const symBox = $("solve-skeleton-symbols");
  if (symBox) symBox.checked = symbolsShown;
  const wireBox = $("solve-skeleton-wires");
  if (wireBox) wireBox.checked = wiresShown;
}

export function startSkeleton() {
  symbolsShown = load("skeleton-symbols-shown", false) === true;
  wiresShown = load("skeleton-wires-shown", false) === true;
  syncControls();
  if (!hooked) {
    const symBox = $("solve-skeleton-symbols");
    const wireBox = $("solve-skeleton-wires");
    if (symBox || wireBox) {
      hooked = true;
      symBox?.addEventListener("change", () => toggleSkeletonSymbols(symBox.checked));
      wireBox?.addEventListener("change", () => toggleSkeletonWires(wireBox.checked));
    }
  }
}

/** Show or hide component and pin skeleton lines: `value` true or false, or inverted when omitted. */
export function toggleSkeletonSymbols(value) {
  symbolsShown = typeof value === "boolean" ? value : !symbolsShown;
  save("skeleton-symbols-shown", symbolsShown);
  syncControls();
  window.dispatchEvent(new Event("skeleton-changed"));
}

/** Show or hide wire skeleton lines: `value` true or false, or inverted when omitted. */
export function toggleSkeletonWires(value) {
  wiresShown = typeof value === "boolean" ? value : !wiresShown;
  save("skeleton-wires-shown", wiresShown);
  syncControls();
  window.dispatchEvent(new Event("skeleton-changed"));
}

/** What the pointer at canvas point `p` rests on among the visible skeleton lines, or `null`. */
export function skeletonAt(view, p) {
  if (!layer || !view || !lines.length) return null;
  return skeletonHit(lines, view.x + p.x / view.k, view.y + p.y / view.k, NEAR / view.k);
}

/** Build or project the skeleton lines of the current level onto `svg`; `svg` null clears them. */
export function paintSkeleton(svg, view) {
  if (!svg || !view || !(symbolsShown || wiresShown) || !doc) {
    layer?.remove();
    layer = null;
    shapes = [];
    lines = [];
    drawn = null;
    return;
  }
  const key = [svg, doc, current, symbolsShown, wiresShown];
  if (!drawn || key.some((v, i) => v !== drawn[i])) {
    layer?.remove();
    layer = svgElement("g", { class: "skeleton-lines", "pointer-events": "none", fill: "none" });
    lines = skeletonLines(doc.levels[current], doc, { symbols: symbolsShown, wires: wiresShown }).lines;
    shapes = lines.map((line) => {
      const el = svgElement("line", { class: `skeleton-${line.kind}` });
      layer.append(el);
      return [el, line];
    });
    svg.append(layer);
    drawn = key;
  }
  if (svg.lastElementChild !== layer) svg.append(layer);
  layer.setAttribute("transform", `translate(${view.x} ${view.y}) scale(${1 / view.k})`);
  const cx = (x) => (x - view.x) * view.k;
  const cy = (y) => (y - view.y) * view.k;
  for (const [el, line] of shapes) {
    el.setAttribute("x1", cx(line.a[0]));
    el.setAttribute("y1", cy(line.a[1]));
    el.setAttribute("x2", cx(line.b[0]));
    el.setAttribute("y2", cy(line.b[1]));
  }
}
