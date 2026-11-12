// View > Show Net Names: the name of each net written on the drawing, one for each connected part of the net, at the place
// the core gives (`Level.net_labels`, in SVG pixels, as the hits' points are). It is a hint of the page, as the usages' frames
// and the registration points are: off to begin with and kept with the other View settings, in grey at one size and in one
// font whatever the zoom, over the drawing and under the current directions, with no pointer events. File > Export Image
// follows the page: the names are in an exported image when they are shown, at that size in the pixels of the image.
// `paintNetLabels` is called with the view by `paintUsages` (usages.js), which paints the other overlays.

import { setCheck } from "./commands.js";
import { current, doc } from "./state.js";
import { load, save, svgElement } from "./util.js";

const FONT_SIZE = 11; // pixels, on the screen and in an exported image
const COLORS = { light: { ink: "#6b6b6b", halo: "#ffffff" }, dark: { ink: "#a8a8a8", halo: "#000000" } };

let shown = false;
let layer = null;
let marks = []; // [text, x, y] in SVG pixels
let drawn = null; // what the layer was made for: [svg, doc, level, shown]

/** The names to write on a level: `{ name, x, y }` for each of its net labels whose net the Document names. */
export function netLabelsOf(document, level) {
  const names = [];
  for (const { net, at } of level?.net_labels ?? []) {
    const name = document?.objects[net]?.name;
    if (name) names.push({ name, x: at[0], y: at[1] });
  }
  return names;
}

/** Whether the net names are shown. */
export const netLabelsShown = () => shown;

export function startNetLabels() {
  shown = load("net-labels-shown", false) === true;
  setCheck("toggle-net-labels", shown);
}

export function setNetLabels(value, { persist = true } = {}) {
  shown = !!value;
  if (persist) save("net-labels-shown", shown);
  setCheck("toggle-net-labels", shown);
  window.dispatchEvent(new Event("net-labels-changed"));
}

export function toggleNetLabels() {
  setNetLabels(!shown);
}

/** One name, centred on its place, with a halo of the paper's colour so that it reads over the lines under it. */
function label({ name }, ink, halo) {
  const text = svgElement("text", {
    "text-anchor": "middle", "dominant-baseline": "central", "font-size": FONT_SIZE, fill: ink, stroke: halo,
    "stroke-width": 3, "stroke-linejoin": "round", "paint-order": "stroke", "pointer-events": "none",
  });
  text.textContent = name;
  return text;
}

/** Make the names of the level shown when something changed, and put them on the view: `svg` null takes them away. */
export function paintNetLabels(svg, view) {
  if (!svg || !view || !shown || !doc) {
    layer?.remove();
    layer = null;
    marks = [];
    drawn = null;
    return;
  }
  if (!drawn || drawn[0] !== svg || drawn[1] !== doc || drawn[2] !== current || drawn[3] !== shown) {
    layer?.remove();
    layer = svgElement("g", { class: "net-labels", "pointer-events": "none" });
    // the font is the drawing's: the one the core names on the <svg> itself
    const font = svg.getAttribute("font-family");
    if (font) layer.setAttribute("font-family", font);
    marks = netLabelsOf(doc, doc.levels[current]).map((l) => {
      const text = label(l, "var(--net-label-color)", "var(--net-label-halo)");
      layer.append(text);
      return [text, l.x, l.y];
    });
    // over the usages' layers, under the current directions
    svg.insertBefore(layer, svg.querySelector(":scope > .current-overlays"));
    drawn = [svg, doc, current, shown];
  } else if (layer.parentNode !== svg) {
    svg.insertBefore(layer, svg.querySelector(":scope > .current-overlays"));
  }
  layer.setAttribute("transform", `translate(${view.x} ${view.y}) scale(${1 / view.k})`);
  for (const [text, x, y] of marks) text.setAttribute("transform", `translate(${(x - view.x) * view.k} ${(y - view.y) * view.k})`);
}

/**
 * Write the names of `level` into an exported image `svg`, in the image's pixels, which are the hits'; `dark` is the page's
 * background. The font is the one the image names on its <svg>, which the image embeds.
 */
export function addNetLabels(svg, document, level, dark) {
  const { ink, halo } = COLORS[dark ? "dark" : "light"];
  const g = svg.ownerDocument.createElementNS("http://www.w3.org/2000/svg", "g");
  g.setAttribute("class", "net-labels");
  for (const l of netLabelsOf(document, level)) {
    const text = label(l, ink, halo);
    text.setAttribute("x", String(l.x));
    text.setAttribute("y", String(l.y));
    g.append(text);
  }
  svg.append(g);
}
