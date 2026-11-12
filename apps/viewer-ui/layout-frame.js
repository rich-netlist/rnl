// Solve > Show Layout Table: a thin-line table over the drawing of the level shown so one sees
// which cell each object occupies. Computed directly from the drawing geometry and group tree
// (group-tree.js), independent of how the circuit was placed. Off by default, persisted in
// settings, synced with the Solve panel checkbox, drawn at non-scaling width with no pointer
// events, and omitted from File > Export Image. `paintLayout` is called by `paintUsages`.

import { setCheck } from "./commands.js";
import { current, doc } from "./state.js";
import { $, load, save, svgElement } from "./util.js";
import { layoutTable } from "./group-tree.js";

export { layoutTable };

let shown = false;
let layer = null;
let shapes = []; // [rect element, [x0, y0, x1, y1] in SVG pixels]
let drawn = null; // what the layer was made for: [svg, doc, level, shown]
let hooked = false;

/** Whether the layout table is shown. */
export const layoutShown = () => shown;

function syncControls() {
  setCheck("toggle-layout", shown);
  const checkbox = $("solve-layout-table");
  if (checkbox) checkbox.checked = shown;
}

export function startLayout() {
  shown = load("layout-shown", false) === true;
  syncControls();
  const checkbox = $("solve-layout-table");
  if (checkbox && !hooked) {
    hooked = true;
    checkbox.addEventListener("change", () => toggleLayout(checkbox.checked));
  }
}

/** Show or hide the table: `value` true or false, or inverted when omitted. */
export function toggleLayout(value, { persist = true } = {}) {
  shown = typeof value === "boolean" ? value : !shown;
  if (persist) save("layout-shown", shown);
  syncControls();
  window.dispatchEvent(new Event("layout-changed"));
}

/** Build or project the layout table of the current level onto `svg`; `svg` null clears it. */
export function paintLayout(svg, view) {
  if (!svg || !view || !shown || !doc) {
    layer?.remove();
    layer = null;
    shapes = [];
    drawn = null;
    return;
  }
  if (!drawn || drawn[0] !== svg || drawn[1] !== doc || drawn[2] !== current || drawn[3] !== shown) {
    layer?.remove();
    layer = svgElement("g", { class: "layout-table", "pointer-events": "none", fill: "none" });
    const table = layoutTable(doc.levels[current], doc);
    shapes = table.map(({ rect, kind }) => {
      const el = svgElement("rect", { class: `layout-${kind}` });
      layer.append(el);
      return [el, rect];
    });
    svg.append(layer);
    drawn = [svg, doc, current, shown];
  }
  if (svg.lastElementChild !== layer) svg.append(layer);
  layer.setAttribute("transform", `translate(${view.x} ${view.y}) scale(${1 / view.k})`);
  for (const [el, [x0, y0, x1, y1]] of shapes) {
    const a = [(x0 - view.x) * view.k, (y0 - view.y) * view.k];
    const b = [(x1 - view.x) * view.k, (y1 - view.y) * view.k];
    el.setAttribute("x", Math.min(a[0], b[0]));
    el.setAttribute("y", Math.min(a[1], b[1]));
    el.setAttribute("width", Math.abs(b[0] - a[0]));
    el.setAttribute("height", Math.abs(b[1] - a[1]));
  }
}
