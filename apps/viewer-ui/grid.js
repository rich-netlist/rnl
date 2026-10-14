// The grid: a view aid of the page, under the drawing and not a part of it. Dots or lines at the
// integer points of the scene, x to the right and y up, or at a spacing of the user's; where they
// would come closer than MIN_SPACING pixels, at 2, 5, 10, 20, 50 ... times the spacing instead, and
// for lines a little darker every fifth, or at the next step of that sequence; and the axes through
// the origin. They are drawn on a canvas of their own under the drawing's SVG, the same size on the
// screen at every zoom, so that clicks, fitting and the selection go by the drawing alone. The
// settings are the View menu's Grid submenu and its dialog, kept in localStorage for the next start.

import { setCheck } from "./commands.js";
import { form } from "./dialog.js";
import { $, save } from "./util.js";

const holder = $("canvas");
const grid = $("grid");

const MIN_SPACING = 10; // CSS pixels between neighbouring dots or lines, at least
const DOT = 2; // the side of a dot, in CSS pixels
const LINE = 1; // the width of a line and of an axis, in CSS pixels
const MAX_SPACING = 1000; // units, the most a spacing of the user's may be
const MAX_MAJOR = 100; // lines, the most a major line may be apart

/** The settings: whether the grid is shown, as "dots" or "lines", its spacing ("auto" for 1 unit), its major lines and the axes. */
const settings = { shown: true, style: "dots", spacing: "auto", major: 5, axes: false };
let shownView = null; // the view the grid was last drawn for, and the drawing's pixels per unit
let drawn = null; // the step of the grid drawn and of its major lines, in units
let colours = null; // the colours of the dots, the lines, the major lines and the axes, from style.css

/** What each setting may be. */
const VALID = {
  shown: (v) => typeof v === "boolean",
  style: (v) => v === "dots" || v === "lines",
  spacing: (v) => v === "auto" || (typeof v === "number" && v > 0 && v <= MAX_SPACING),
  major: (v) => Number.isInteger(v) && v >= 2 && v <= MAX_MAJOR,
  axes: (v) => typeof v === "boolean",
};

/**
 * Change the settings that `changes` names, keep them for the next start, and draw the grid again.
 * A value that will not do, such as one an older page kept, leaves its setting as it was.
 */
export function setGrid(changes) {
  for (const [key, value] of Object.entries(changes)) {
    if (VALID[key] && VALID[key](value)) settings[key] = value;
  }
  for (const key of Object.keys(VALID)) save(`grid-${key}`, settings[key]);
  // all the marks, also those that did not change: a native menu toggles the item chosen
  setCheck("toggle-grid", settings.shown);
  setCheck("grid-none", !settings.shown);
  setCheck("grid-dots", settings.shown && settings.style === "dots");
  setCheck("grid-lines", settings.shown && settings.style === "lines");
  setCheck("grid-axes", settings.axes);
  paint();
}

/** Run a command of the Grid submenu. */
export function gridCommand(id) {
  switch (id) {
    case "toggle-grid": setGrid({ shown: !settings.shown }); break;
    case "grid-none": setGrid({ shown: false }); break;
    case "grid-dots": setGrid({ shown: true, style: "dots" }); break;
    case "grid-lines": setGrid({ shown: true, style: "lines" }); break;
    case "grid-axes": setGrid({ axes: !settings.axes }); break;
    case "grid-settings": askSettings(); break;
    default: break;
  }
}

/** The settings, and the steps of the grid drawn, in units: `step`, and `majorStep` for lines; null when none is drawn. */
export function gridState() {
  return { ...settings, step: drawn ? drawn.step : null, majorStep: drawn ? drawn.major : null };
}

/**
 * Draw the grid for a view of the drawing (drawing.js), which has `scale` pixels of the SVG per
 * unit; with no view, as when nothing is drawn, clear it.
 */
export function paintGrid(view, scale) {
  shownView = view && scale ? { view, scale } : null;
  paint();
}

/** The multiples of the spacing that the grid may take: 1, 2, 5, 10, 20, 50 and so on. */
function* steps() {
  for (let decade = 1; ; decade *= 10) {
    for (const m of [1, 2, 5]) yield m * decade;
  }
}

function paint() {
  const dpr = window.devicePixelRatio || 1;
  const [w, h] = [holder.clientWidth, holder.clientHeight];
  const [W, H] = [Math.max(1, Math.round(w * dpr)), Math.max(1, Math.round(h * dpr))];
  if (grid.width !== W) grid.width = W;
  if (grid.height !== H) grid.height = H;
  const ctx = grid.getContext("2d");
  ctx.clearRect(0, 0, W, H);
  drawn = null;
  if (!shownView || (!settings.shown && !settings.axes)) return;
  const { view, scale } = shownView;
  if (!colours) {
    const style = getComputedStyle(grid);
    const colour = (name) => style.getPropertyValue(name).trim();
    colours = { dot: colour("--grid-dot"), line: colour("--grid-line"), major: colour("--grid-major"), axis: colour("--grid-axis") };
  }
  // where a point of the scene is on the canvas, in device pixels, and the scene the canvas shows
  const px = (u) => (u * scale - view.x) * view.k * dpr;
  const py = (v) => (-v * scale - view.y) * view.k * dpr;
  const [u0, u1] = [view.x / scale, (view.x + w / view.k) / scale];
  const [v0, v1] = [-(view.y + h / view.k) / scale, -view.y / scale];
  const unit = scale * view.k; // CSS pixels per unit
  const line = Math.max(1, Math.round(LINE * dpr));
  const across = (p) => Math.round(p - line / 2);

  if (settings.shown) {
    const base = settings.spacing === "auto" ? 1 : settings.spacing;
    let q = 1; // the step, in multiples of the spacing
    for (q of steps()) if (base * q * unit >= MIN_SPACING) break;
    const s = base * q;
    const range = (a, b) => Array.from({ length: Math.max(0, Math.ceil(b / s) - Math.floor(a / s) + 1) }, (_, k) => Math.floor(a / s) + k);
    const [is, js] = [range(u0, u1), range(v0, v1)];
    if (settings.style === "dots") {
      const d = Math.max(1, Math.round(DOT * dpr));
      const xs = is.map((i) => Math.round(px(i * s) - d / 2));
      const ys = js.map((j) => Math.round(py(j * s) - d / 2));
      ctx.fillStyle = colours.dot;
      // one row of dots, copied to the others
      const first = ys.find((y) => y >= 0 && y + d <= H);
      for (const y of first === undefined ? ys : [first]) for (const x of xs) ctx.fillRect(x, y, d, d);
      if (first !== undefined) for (const y of ys) if (y !== first) ctx.drawImage(grid, 0, first, W, d, 0, y, W, d);
      drawn = { step: s, major: null };
    } else {
      // the major lines: every `major` lines, or the next step of the sequence that is a multiple of the step
      let Q = q;
      for (Q of steps()) if (Q >= settings.major * q && Q % q === 0) break;
      for (const major of [false, true]) {
        ctx.fillStyle = major ? colours.major : colours.line;
        for (const i of is) if (((i * q) % Q === 0) === major) ctx.fillRect(across(px(i * s)), 0, line, H);
        for (const j of js) if (((j * q) % Q === 0) === major) ctx.fillRect(0, across(py(j * s)), W, line);
      }
      drawn = { step: s, major: base * Q };
    }
  }
  if (settings.axes) {
    ctx.fillStyle = colours.axis;
    ctx.fillRect(across(px(0)), 0, line, H);
    ctx.fillRect(0, across(py(0)), W, line);
  }
}

// the colours follow the colour scheme, and the canvas the pixels of the screen it is on
matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
  colours = null;
  paint();
});

function watchResolution() {
  matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`).addEventListener(
    "change",
    () => {
      paint();
      watchResolution();
    },
    { once: true },
  );
}
watchResolution();

// ------------------------------------------------------------------ the dialog

/** A labelled control of the dialog: an input of a type, and the text around it. */
function field(type, before, after = "") {
  const input = document.createElement("input");
  input.type = type;
  const label = document.createElement("label");
  label.className = `dialog-${type === "number" ? "line" : "choice"}`;
  if (type === "number") label.append(before, input, after);
  else label.append(input, before);
  return [label, input];
}

/** A group of radio buttons under a legend, each [value, text]; the one of `value` is checked. */
function radios(legend, name, choices, value) {
  const set = document.createElement("fieldset");
  const head = document.createElement("legend");
  head.textContent = legend;
  set.append(head);
  const inputs = choices.map(([v, text]) => {
    const [label, input] = field("radio", text);
    input.name = name;
    input.value = v;
    input.checked = v === value;
    set.append(label);
    return input;
  });
  return [set, inputs];
}

/** The Grid Settings dialog: the style, the spacing, the major lines and the axes. */
async function askSettings() {
  const now = settings.shown ? settings.style : "none";
  const [styleSet, styles] = radios("Grid", "grid-style", [["none", "None"], ["dots", "Dots"], ["lines", "Lines"]], now);
  const auto = settings.spacing === "auto";
  const [spacingSet, [autoSpacing, fixedSpacing]] = radios(
    "Spacing",
    "grid-spacing",
    [["auto", "Auto: 1 unit, wider where the grid would be dense"], ["fixed", "Fixed: "]],
    auto ? "auto" : "fixed",
  );
  // the units of a fixed spacing, after its radio button
  const units = document.createElement("input");
  units.type = "number";
  units.min = "0";
  units.step = "any";
  units.value = auto ? "1" : String(settings.spacing);
  units.setAttribute("aria-label", "Fixed spacing in units");
  fixedSpacing.parentElement.append(units, " units");
  units.addEventListener("input", () => {
    fixedSpacing.checked = true;
  });
  const [majorLine, major] = field("number", "A major line every ", " lines");
  major.min = "2";
  major.max = String(MAX_MAJOR);
  major.step = "1";
  major.value = String(settings.major);
  const [axesLine, axes] = field("checkbox", "Axes through the origin");
  axes.checked = settings.axes;
  const error = document.createElement("p");
  error.className = "dialog-error";
  error.hidden = true;
  // major lines are drawn with lines only
  const sync = () => {
    major.disabled = !styles[2].checked;
  };
  for (const r of styles) r.addEventListener("change", sync);
  sync();

  const wrong = (message, input) => {
    error.textContent = message;
    error.hidden = false;
    input.focus();
    return undefined;
  };
  const answer = await form({
    title: "Grid Settings",
    parts: [styleSet, spacingSet, majorLine, axesLine, error],
    focus: styles.find((r) => r.checked),
    answer: () => {
      const spacing = autoSpacing.checked ? "auto" : Number(units.value);
      if (!VALID.spacing(spacing)) return wrong(`The spacing is a number of units above 0 and at most ${MAX_SPACING}.`, units);
      const every = Number(major.value);
      if (!VALID.major(every)) return wrong(`A major line comes every 2 to ${MAX_MAJOR} lines, a whole number.`, major);
      const style = styles.find((r) => r.checked).value;
      return { shown: style !== "none", ...(style !== "none" && { style }), spacing, major: every, axes: axes.checked };
    },
  });
  if (answer) setGrid(answer);
}
