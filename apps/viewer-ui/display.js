import { append, m, setText, tr } from "./i18n.js";
// A manifest's display settings (manifest.js `display`): the view of its level, the background, the
// colour scheme, whether usages and current directions are shown, and the grid. A manifest only
// proposes them: the page lists those it has and applies the ones the user ticks. They hold while a
// file of the manifest is shown, as its bar does (pool.js): a file of another goes back to the user's
// own settings, and a file of the manifest again brings them back, unasked. They are never written
// into the settings; a setting the user changes meanwhile is the user's. And the page writes them, as
// they are now, into a project or a manifest of links when the user asks.
import { isDark, setBackground } from "./background.js";
import { load } from "./util.js";
import { form, modal } from "./dialog.js";
import { fit, notice, showView, viewCentre } from "./drawing.js";
import { flylinesShown, setFlylines } from "./flylines.js";
import { gridSettings, setGrid } from "./grid.js";
import { hoverNamesOn, hoverRolesOn, setHoverNames, setHoverRoles } from "./hover.js";
import { layoutShown, toggleLayout } from "./layout-frame.js";
import { netLabelsShown, setNetLabels } from "./net-labels.js";
import { COLOR_FIELDS, DEFAULT_COLORS, currentColors, showColors, showOwnColors } from "./netlist-colors.js";
import { originsShown, setOrigins } from "./origins.js";
import { skeletonSymbolsShown, skeletonWiresShown, toggleSkeletonSymbols, toggleSkeletonWires } from "./skeleton-lines.js";
import { overlayChoices, showOverlays } from "./usages.js";

// each manifest offered in this page, by its address or name: its settings, those the user kept, and
// whether a file is one of its; and the one whose settings are shown, or null for the user's own
const scopes = new Map();
let active = null;

const colourNames = new Set(COLOR_FIELDS.map(([key]) => key));
const shownOrHidden = (on) => (on ? m("shown") : m("hidden"));

/** What the settings of `display` are, one line each, in the order they apply. */
function items(display) {
  const out = [];
  if (display.background) out.push({ key: "background", label: m`Background: ${display.background === "dark" ? m("dark") : m("light")}` });
  const colors = Object.keys(display.colors || {}).filter((name) => colourNames.has(name));
  if (colors.length) out.push({ key: "colors", label: m`Color scheme: ${colors.length} of its colors` });
  if (typeof display.names === "boolean") out.push({ key: "names", label: m`Names on hover: ${shownOrHidden(display.names)}` });
  if (typeof display.roles === "boolean") out.push({ key: "roles", label: m`Roles on hover: ${shownOrHidden(display.roles)}` });
  if (typeof display.usages === "boolean") out.push({ key: "usages", label: m`Usages: ${shownOrHidden(display.usages)}` });
  if (typeof display.currents === "boolean") out.push({ key: "currents", label: m`Current directions: ${shownOrHidden(display.currents)}` });
  if (typeof display.origins === "boolean") out.push({ key: "origins", label: m`Registration points: ${shownOrHidden(display.origins)}` });
  if (typeof display.flylines === "boolean") out.push({ key: "flylines", label: m`Flylines: ${shownOrHidden(display.flylines)}` });
  if (typeof display["net-labels"] === "boolean") out.push({ key: "net-labels", label: m`Net names: ${shownOrHidden(display["net-labels"])}` });
  if (typeof display.layout === "boolean") out.push({ key: "layout", label: m`Layout table: ${shownOrHidden(display.layout)}` });
  if (typeof display["skeleton-parts"] === "boolean") out.push({ key: "skeleton-parts", label: m`Component and pin skeleton lines: ${shownOrHidden(display["skeleton-parts"])}` });
  if (typeof display["skeleton-wires"] === "boolean") out.push({ key: "skeleton-wires", label: m`Wire skeleton lines: ${shownOrHidden(display["skeleton-wires"])}` });
  if (display.grid) out.push({ key: "grid", label: display.grid === "none" ? m("Grid: none") : display.grid === "dots" ? m("Grid: dots") : m("Grid: lines") });
  if (display.view) out.push({ key: "view", label: display.view.fit ? m("View: the whole level, fitted to the window") : m("View: the part of the level it shows") });
  return out;
}

/** Apply the settings of `display` named in `keys`, never into the settings. */
function apply(display, keys) {
  if (keys.has("background")) setBackground(display.background, { persist: false });
  if (keys.has("colors")) showColors(Object.fromEntries(Object.entries(display.colors).filter(([name]) => colourNames.has(name))));
  if (keys.has("names")) setHoverNames(display.names, { persist: false });
  if (keys.has("roles")) setHoverRoles(display.roles, { persist: false });
  const overlays = {};
  if (keys.has("usages")) overlays.usages = display.usages;
  if (keys.has("currents")) overlays.currents = display.currents;
  if (Object.keys(overlays).length) showOverlays(overlays);
  if (keys.has("origins")) setOrigins(display.origins, { persist: false });
  if (keys.has("flylines")) setFlylines(display.flylines, { persist: false });
  if (keys.has("net-labels")) setNetLabels(display["net-labels"], { persist: false });
  if (keys.has("layout")) toggleLayout(display.layout, { persist: false });
  if (keys.has("skeleton-parts")) toggleSkeletonSymbols(display["skeleton-parts"], { persist: false });
  if (keys.has("skeleton-wires")) toggleSkeletonWires(display["skeleton-wires"], { persist: false });
  if (keys.has("grid")) setGrid(display.grid === "none" ? { shown: false } : { shown: true, style: display.grid }, { persist: false });
  if (keys.has("view")) {
    if (display.view.fit) fit();
    else showView(display.view);
  }
}

/** The user's own display settings again, as the settings keep them. */
function restoreOwn() {
  setBackground(load("background", "light"), { persist: false });
  showOwnColors();
  setHoverNames(load("hover-names", true) !== false, { persist: false });
  setHoverRoles(load("hover-roles", false) === true, { persist: false });
  showOverlays({ usages: load("usages-shown", false) === true, currents: load("currents-shown", false) === true });
  setOrigins(load("origins-shown", false) === true, { persist: false });
  setFlylines(load("flylines-shown", false) === true, { persist: false });
  setNetLabels(load("net-labels-shown", false) === true, { persist: false });
  toggleLayout(load("layout-shown", false) === true, { persist: false });
  toggleSkeletonSymbols(load("skeleton-symbols-shown", false) === true, { persist: false });
  toggleSkeletonWires(load("skeleton-wires-shown", false) === true, { persist: false });
  setGrid({ shown: load("grid-shown", false), style: load("grid-style", "dots") }, { persist: false });
}

/** Show the settings the user kept of `scope`; its view only when it was just opened. */
function enter(scope, opened) {
  active = scope;
  const keys = new Set(scope.keys);
  if (!opened) keys.delete("view");
  apply(scope.display, keys);
}

/**
 * The file at `path` is shown (pool.js): keep a manifest's settings while it is one of its files,
 * go back to the user's own for a file of none, and bring them back for a file of one again.
 */
export function displayFollows(path) {
  if (!path || active?.has(path)) return;
  const scope = [...scopes.values()].find((s) => s.keys.size && s.has(path)) || null;
  if (active && !scope) restoreOwn();
  active = null;
  if (scope) enter(scope, false);
}

/**
 * Offer the display settings `display` of the manifest named `name` (its address, or a file's name),
 * whose files are those `has` says: list them, each with a box ticked, and apply those the user
 * keeps while a file of it is shown (displayFollows). Those of another shape that the manifest had
 * (`ignored`) are told. Asked once per manifest in a page.
 */
export async function offerDisplay(display, name, ignored = [], has = () => false) {
  if (ignored.length) notice(m`This viewer leaves out display settings it does not know or cannot read: ${ignored.join(", ")}.`);
  const list = items(display || {});
  if (!list.length) return;
  const known = scopes.get(name);
  if (known) {
    Object.assign(known, { display, has });
    if (known.keys.size) enter(known, true);
    return;
  }
  if (modal()) return;
  const boxes = list.map(({ key, label }) => {
    const row = document.createElement("label");
    row.className = "display-setting";
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = true;
    box.dataset.setting = key;
    const text = document.createElement("span");
    setText(text, label);
    append(row, box, text);
    return { key, row, box };
  });
  const keys = await form({
    title: m("Display Settings"),
    text: m("The manifest includes these display settings. Selected settings apply while viewing this manifest without changing your saved preferences."),
    parts: boxes.map((b) => b.row),
    action: tr("Apply Selected"),
    cancel: tr("Don't Apply"),
    answer: () => new Set(boxes.filter((b) => b.box.checked).map((b) => b.key)),
  });
  const scope = { display, keys: keys || new Set(), has };
  scopes.set(name, scope);
  if (scope.keys.size) enter(scope, true);
}

/** The display settings as the page shows them now, as a manifest records them. */
export function currentDisplay() {
  const display = { background: isDark() ? "dark" : "light" };
  const colors = Object.fromEntries(Object.entries(currentColors()).filter(([name, c]) => c !== DEFAULT_COLORS[name]));
  if (Object.keys(colors).length) display.colors = colors;
  if (!hoverNamesOn()) display.names = false;
  if (hoverRolesOn()) display.roles = true;
  const overlays = overlayChoices();
  display.usages = overlays.usages;
  display.currents = overlays.currents;
  if (originsShown()) display.origins = true;
  if (flylinesShown()) display.flylines = true;
  if (netLabelsShown()) display["net-labels"] = true;
  if (layoutShown()) display.layout = true;
  if (skeletonSymbolsShown()) display["skeleton-parts"] = true;
  if (skeletonWiresShown()) display["skeleton-wires"] = true;
  const grid = gridSettings();
  display.grid = grid.shown ? grid.style : "none";
  const view = viewCentre();
  if (view) display.view = { center: view.center.map((n) => Math.round(n * 100) / 100), scale: Math.round(view.scale * 1000) / 1000 };
  return display;
}

/** A box for a save dialog: whether to write the display settings shown into the manifest. */
export function displayBox() {
  const row = document.createElement("label");
  row.className = "display-setting";
  const box = document.createElement("input");
  box.type = "checkbox";
  box.id = "write-display";
  const text = document.createElement("span");
  setText(text, m("Also write the current view, background, color scheme, usages, current directions and grid"));
  append(row, box, text);
  return { row, box };
}
