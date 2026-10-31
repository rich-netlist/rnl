// View > Show Registration Points: a small mark at the registration point of each instance and each usage with a symbol
// of the level shown, the point its position is the place of (the `origin` of the Document's box hits, in SVG pixels, as
// the hits' points are). It is a hint of the page, as the usages' frames are: off to begin with and kept with the other
// View settings, drawn at one size whatever the zoom, over the drawing and under the current directions, with no pointer
// events, and not in File > Export Image, which the core draws. It is shown with the netlist's drawing and a solve's
// preview alike. `paintOrigins` is called with the view by `paintUsages` (usages.js), which paints the other overlays.

import { setCheck } from "./commands.js";
import { current, doc } from "./state.js";
import { $, load, save, svgElement } from "./util.js";

let shown = false;
let layer = null;
let marks = []; // [group, x, y] in SVG pixels
let drawn = null; // what the layer was made for: [svg, doc, level, shown]

/** The registration points of the level's boxes: `[x, y]` for each box hit that has its `origin`. */
export function originsOf(level) {
  return (level?.hits ?? []).filter((h) => h.kind === "box" && Array.isArray(h.origin)).map((h) => h.origin);
}

/** Whether the registration points are shown. */
export const originsShown = () => shown;

export function startOrigins() {
  shown = load("origins-shown", false) === true;
  setCheck("toggle-origins", shown);
}

export function toggleOrigins() {
  shown = !shown;
  save("origins-shown", shown);
  setCheck("toggle-origins", shown);
  window.dispatchEvent(new Event("origins-changed"));
}

/** One mark: a cross with a ring, with a light halo, so that it reads over the drawing; its size is in screen pixels. */
function mark() {
  const g = svgElement("g", { class: "origin-mark" });
  const halo = { fill: "none", stroke: "var(--usage-label-bg)", "stroke-width": 4, "stroke-linecap": "round", "stroke-opacity": 0.8 };
  const ink = { fill: "none", stroke: "var(--origin-color)", "stroke-width": 1.5, "stroke-linecap": "round" };
  for (const attrs of [halo, ink]) {
    g.append(svgElement("path", { ...attrs, d: "M-6 0H6M0 -6V6" }));
    g.append(svgElement("circle", { ...attrs, cx: 0, cy: 0, r: 3 }));
  }
  return g;
}

/** Make the marks of the level shown when something changed, and put them on the view: `svg` null takes them away. */
export function paintOrigins(svg, view) {
  if (!svg || !view || !shown || !doc) {
    layer?.remove();
    layer = null;
    marks = [];
    drawn = null;
    return;
  }
  if (!drawn || drawn[0] !== svg || drawn[1] !== doc || drawn[2] !== current || drawn[3] !== shown) {
    layer?.remove();
    layer = svgElement("g", { class: "origin-marks", "pointer-events": "none" });
    marks = originsOf(doc.levels[current]).map(([x, y]) => {
      const g = mark();
      layer.append(g);
      return [g, x, y];
    });
    // over the usages' layers, under the current directions
    svg.insertBefore(layer, svg.querySelector(":scope > .current-overlays"));
    drawn = [svg, doc, current, shown];
  } else if (layer.parentNode !== svg) {
    svg.insertBefore(layer, svg.querySelector(":scope > .current-overlays"));
  }
  layer.setAttribute("transform", `translate(${view.x} ${view.y}) scale(${1 / view.k})`);
  for (const [g, x, y] of marks) g.setAttribute("transform", `translate(${(x - view.x) * view.k} ${(y - view.y) * view.k})`);
}
