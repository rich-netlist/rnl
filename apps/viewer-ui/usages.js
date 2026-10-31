// Scope-local semantic groups. Display preferences never change the RNL source or its geometry.
import { append, m, setAttr, setText } from "./i18n.js";
import { current, doc, selected, selectedAsUsage } from "./state.js";
import { clearSelection, select } from "./selection.js";
import { setCheck } from "./commands.js";
import { $, load, save, span, svgElement } from "./util.js";
import { setPathText } from "./source-path.js";
import { isDark, mutedColour } from "./background.js";
import { objectUsages, relatedUsages, stateName } from "./usage-table.js";
import { FRAME_STEP, frameSteps, TITLE_ROW, titleRows } from "./frame-layout.js";
import { setPane } from "./panels.js";
import { revealLine } from "./text.js";

export const USAGE_COLORS = ["#e03131", "#e5b500", "#4263eb", "#2f9e44", "#9c36b5", "#009ccc"];
const MAX_PREFERENCES = 2000;
let preferences = new Map();
const activeSettings = new WeakMap();
let shown = true;
let currentsShown = false;
let currentLayer = null;
let revision = 0;
const expanded = new Set(); // the definitions whose group is open: none to begin with, for each level of each document
const rowLimits = new Map();
let scopeDoc = null;
let revealPending = false; // a usage was selected: its row is brought into view once the list is shown
let scopeLevel = -1;
let drawing = null;
let layer = null;
let underlay = null;
let frames = [];
let hovered = null; // the usage whose row is pointed at, in the usages of an object
let layerDoc = null;
let layerLevel = -1;
let layerRevision = -1;
let layerSelected = null;
let context = null;

const usages = () => doc?.levels[current]?.usages || [];
/** A group (`@GROUP`, 11.5) is listed with the usages and drawn as one, of a kind of its own. */
const GROUP_TYPE = "@GROUP";
export const isGroup = (u) => u.kind === "group";
/** The heading a usage is listed under: its definition, or `@GROUP` for the groups. */
const typeOf = (u) => (isGroup(u) ? GROUP_TYPE : u.definition);
/** What the drawing and the details call it: `$DIFFERENTIAL_PAIR: %dp1`, `@GROUP: %core`. */
const labelOf = (u) => `${typeOf(u)}: ${u.id}`;
/** A group may be turned off by PRIORITY (14.8), which its entry says in `state`; it is never failed or overridden. */
const stateOf = (u) => u.state ?? "in-effect";
const DISABLED_OPACITY = 0.4; // how strongly the frame of a group that is turned off is drawn
/** A level shown, or another document, starts with every group folded. */
function newScope() {
  if (doc === scopeDoc && current === scopeLevel) return;
  scopeDoc = doc;
  scopeLevel = current;
  expanded.clear();
  rowLimits.clear();
}
const chosen =() => selectedAsUsage ? usages().find(u => u.object === selected) : null;
const visible = u => shown && setting(u).visible;
/** Hidden virtual symbols still select as ordinary parts when picked on the drawing. */
export function usageSelection(object, from) {
  const u = usages().find(u => u.object === object);
  return !!u && (from === "usage" || from === "source" || visible(u));
}
const keyOf = u => JSON.stringify([doc.path, doc.levels[current].kind, doc.levels[current].title, ...(isGroup(u) ? ["group"] : []), u.id]);
function setting(u) {
  if (activeSettings.has(u)) return activeSettings.get(u);
  const key = keyOf(u);
  if (!preferences.has(key)) {
    preferences.set(key, { color: USAGE_COLORS[Math.floor(Math.random() * USAGE_COLORS.length)], visible: !(u.has_drawing ?? u.has_symbol) });
    while (preferences.size > MAX_PREFERENCES) preferences.delete(preferences.keys().next().value);
  }
  const value = preferences.get(key);
  activeSettings.set(u, value);
  return value;
}
function persist() { save("usage-preferences", Object.fromEntries(preferences)); }

/**
 * What the drawing shows of the level shown's overlays, for File > Export Image: whether usages and
 * current directions are shown, and each usage's visibility and colour, by its object.
 */
export function overlayChoices() {
  return {
    usages: shown,
    currents: currentsShown,
    each: new Map(usages().map((u) => [u.object, { visible: visible(u), color: setting(u).color }])),
  };
}
function remember(u) {
  const key = keyOf(u);
  preferences.delete(key);
  preferences.set(key, setting(u));
  while (preferences.size > MAX_PREFERENCES) preferences.delete(preferences.keys().next().value);
}
function change(u, values) {
  Object.assign(setting(u), values);
  remember(u);
  persist();
  revision++;
  renderUsages();
  repaint();
}
function repaint() { if (context) paintUsages(...context); }

export function startUsages() {
  preferences = new Map(Object.entries(load("usage-preferences", {})).filter(([, v]) =>
    v && /^#[0-9a-f]{6}$/i.test(v.color) && typeof v.visible === "boolean").slice(-MAX_PREFERENCES));
  shown = load("usages-shown", true) !== false;
  currentsShown = load("currents-shown", false) === true;
  setCheck("toggle-currents", currentsShown);
  setCheck("toggle-usages", shown);
  const height = Math.max(80, Math.min(800, Number(load("usage-list-height", 240)) || 240));
  document.documentElement.style.setProperty("--usage-list-h", `${height}px`);
  renderUsages();
}
/**
 * Show or hide usages and current directions as a manifest's display settings say (display.js):
 * for this page only, not kept in the settings.
 */
export function showOverlays({ usages, currents }) {
  if (typeof usages === "boolean") { shown = usages; setCheck("toggle-usages", shown); }
  if (typeof currents === "boolean") { currentsShown = currents; setCheck("toggle-currents", currentsShown); }
  revision++;
  repaint();
}
export function toggleUsages() {
  shown = !shown;
  save("usages-shown", shown);
  setCheck("toggle-usages", shown);
  revision++;
  repaint();
}
export function toggleCurrents() {
  currentsShown = !currentsShown;
  save("currents-shown", currentsShown);
  setCheck("toggle-currents", currentsShown);
  revision++;
  repaint();
}

function checkbox(label, checked, action) {
  const input = document.createElement("input");
  input.type = "checkbox";
  input.checked = checked;
  setAttr(input, "aria-label", label);
  input.addEventListener("change", () => action(input.checked));
  return input;
}
export function renderUsages() {
  const list = $("usage-list");
  if (!list || $("pane-usages").hidden) return;
  newScope();
  const scroll = list.scrollTop;
  const focused = list.contains(document.activeElement) ? document.activeElement.dataset.focusKey : null;
  list.replaceChildren();
  const all = usages();
  const inspected = chosen()?.object;
  if (!all.length) append(list, span("placeholder", m("No usages in this circuit.")));
  const groups = new Map();
  for (const u of all) {
    if (!groups.has(typeOf(u))) groups.set(typeOf(u), []);
    groups.get(typeOf(u)).push(u);
  }
  // the groups come last, after the definitions
  const sorted = [...groups].sort(([a], [b]) => (a === GROUP_TYPE) - (b === GROUP_TYPE) || a.localeCompare(b, undefined, { numeric: true }));
  if (sorted.length) {
    // one button for all the groups: Expand All, or Collapse All when every group is open
    const every = sorted.every(([type]) => expanded.has(type));
    const bar = document.createElement("div");
    bar.className = "usage-toolbar";
    const all = document.createElement("button");
    all.type = "button";
    all.id = "usage-expand-all";
    all.className = "usage-expand-all";
    all.dataset.focusKey = "expand-all";
    all.setAttribute("aria-pressed", String(every));
    setText(all, every ? m("Collapse All") : m("Expand All"));
    all.addEventListener("click", () => {
      if (every) expanded.clear();
      else for (const [type] of sorted) expanded.add(type);
      renderUsages();
    });
    append(bar, all);
    append(list, bar);
  }
  for (const [type, items] of sorted) {
    const group = document.createElement("section");
    group.className = "usage-group";
    group.dataset.type = type;
    const head = document.createElement("div");
    head.className = "usage-group-head";
    const expand = document.createElement("button");
    expand.type = "button";
    expand.className = "usage-expand";
    expand.dataset.focusKey = `type:${type}`;
    const folded = !expanded.has(type);
    expand.setAttribute("aria-expanded", String(!folded));
    append(expand, span("chevron", ""), type, span("badge", `(${items.length})`));
    expand.addEventListener("click", () => {
      if (folded) expanded.add(type); else expanded.delete(type);
      renderUsages();
    });
    const visible = items.filter(u => setting(u).visible).length;
    const control = checkbox(type === GROUP_TYPE ? m("Show all groups") : m`Show all usages of ${type}`, visible === items.length, value => {
      for (const u of items) { Object.assign(setting(u), { visible: value }); remember(u); }
      persist(); revision++; renderUsages(); repaint();
    });
    control.indeterminate = visible > 0 && visible < items.length;
    control.dataset.focusKey = `check:${type}`;
    append(head, expand, control);
    append(group, head);
    if (!folded) {
      const limit = rowLimits.get(type) || 100;
      for (const u of items.slice(0, limit)) {
        const row = document.createElement("div");
        row.className = "usage-row";
        row.dataset.object = String(u.object);
        row.classList.toggle("selected", u.object === inspected);
        const state = isGroup(u) ? stateOf(u) : "in-effect"; // (the list greys the groups that are turned off; a usage that is not in effect has its state in the table of its members' usages)
        row.classList.toggle("greyed", state !== "in-effect");
        const button = document.createElement("button");
        button.type = "button";
        button.className = "usage-name";
        button.dataset.focusKey = `usage:${u.id}`;
        button.setAttribute("aria-pressed", String(u.object === inspected));
        const dot = span("usage-color-dot", "");
        dot.style.background = setting(u).color;
        append(button, dot, u.id);
        button.addEventListener("click", () => select(u.object, "usage"));
        const check = checkbox(m`Show ${u.id}`, setting(u).visible, visible => change(u, { visible }));
        check.dataset.focusKey = `check:${u.id}`;
        // a group that is turned off is greyed, the whole row, and says so
        append(row, button);
        if (state !== "in-effect") append(row, span("usage-state", stateName(state)));
        append(row, check);
        append(group, row);
      }
      if (items.length > limit) {
        const more = document.createElement("button");
        more.type = "button"; more.className = "usage-more";
        setText(more, m`Show more (${limit} of ${items.length})`);
        more.addEventListener("click", () => { rowLimits.set(type, limit + 100); renderUsages(); });
        append(group, more);
      }
    }
    append(list, group);
  }
  list.scrollTop = scroll;
  if (focused) [...list.querySelectorAll("[data-focus-key]")].find(e => e.dataset.focusKey === focused)?.focus({ preventScroll: true });
  if (revealPending) {
    revealPending = false;
    list.querySelector(".usage-row.selected")?.scrollIntoView({ block: "nearest" });
  }
  renderDetail();
}

function renderDetail() {
  const detail = $("usage-detail");
  detail.replaceChildren();
  highlightUsage(null);
  const u = chosen();
  if (!u) {
    // an object that is no usage: the usages it is a member of, in the place of a usage's details
    // (of the level shown: an object's usages are those of its own level)
    if (selected !== null && doc?.objects[selected]?.level === doc.levels[current]?.host) {
      append(detail, objectUsages(doc, selected, {
        select: (usage) => select(usage, "usage"),
        hover: highlightUsage,
        reveal: (file, line) => { setPane("netlist"); revealLine(file, line, true); },
      }));
      return;
    }
    append(detail, span("placeholder", m("Select an object to see its usages, or a usage to inspect it.")));
    return;
  }
  const title = document.createElement("h2");
  setText(title, labelOf(u));
  const colorRow = document.createElement("label");
  colorRow.className = "usage-color-control";
  const color = document.createElement("input");
  color.type = "color";
  color.value = setting(u).color;
  setAttr(color, "aria-label", m("Color"));
  color.addEventListener("input", () => {
    setting(u).color = color.value;
    const row = $("usage-list").querySelector(`[data-object="${u.object}"] .usage-color-dot`);
    if (row) row.style.background = color.value;
    revision++; repaint();
  });
  color.addEventListener("change", () => { remember(u); persist(); });
  append(colorRow, m("Color"), color);
  const palette = document.createElement("div");
  palette.className = "usage-palette";
  for (const value of USAGE_COLORS) {
    const button = document.createElement("button");
    button.type = "button";
    button.style.background = value;
    button.dataset.color = value;
    setAttr(button, "aria-label", m`Set color to ${value}`);
    button.addEventListener("click", () => change(u, { color: value }));
    append(palette, button);
  }
  const roleHead = document.createElement("h3");
  setText(roleHead, m("Roles"));
  const table = document.createElement("table");
  table.className = "usage-roles";
  for (const r of u.roles) {
    const row = table.insertRow();
    setText(row.insertCell(), r.name);
    setText(row.insertCell(), r.members.join(", ") || m("Not assigned"));
  }
  const sourceHead = document.createElement("h3");
  setText(sourceHead, m("Definition source"));
  const file = document.createElement("div");
  file.className = "path-text usage-source-file";
  setPathText(file, u.file || "");
  const source = document.createElement("pre");
  source.className = "usage-source";
  setText(source, u.source || m("Generated usage; no source statement is available."));
  append(detail, title, colorRow, palette, roleHead, table, sourceHead, file, source);
}

/** The groups of the drawing that stay at full strength when a usage is in focus: its members and what it draws. */
function focusSet(u) {
  const keep = new Set(localSymbol(u) ? [] : u.roles.flatMap(r => r.groups));
  if (!localSymbol(u)) {
    const members = new Set(u.roles.flatMap(r => r.objects));
    for (const h of doc.levels[current].hits) if (h.id && h.objects.some(o => members.has(o))) keep.add(h.id);
  }
  // A virtual symbol is local to one usage; its attached net may span the entire drawing.
  for (const h of doc.levels[current].hits) if (h.objects.includes(u.object) && h.id) keep.add(h.id);
  return keep;
}
/** Dim every group of the drawing that is not in `keep`; with none, dim none. */
function mute(groups, keep) {
  for (const [id, elements] of groups) for (const g of elements) {
    g.classList.toggle("usage-muted", !!keep && !keep.has(id) && !g.parentElement.closest("[data-id]"));
  }
}
/**
 * The usages whose frames stay at full strength: the one in focus, the one a row of an object's usages is pointed at, or
 * the usages of the selected object, those its table lists; null when nothing is selected, and no frame is muted.
 */
function litFrames() {
  if (hovered !== null) return new Set([hovered]);
  const u = chosen();
  if (u) return new Set([u.object]);
  // an object of the level shown, as the table of its usages is
  if (selected !== null && doc?.objects[selected]?.level === doc.levels[current]?.host) return relatedUsages(doc, selected);
  return null;
}
/**
 * The grey of a muted frame: the ink of the drawing blended toward the paper twice by `mutedColour`, which dims the
 * components once, so the frames are the fainter of the two and do not cover the components that are muted.
 */
const frameGrey = () => mutedColour(mutedColour(isDark() ? "#ffffff" : "#000000"));
/**
 * Dim the frames of every usage but those in `lit`, as `mute` dims the groups of the drawing: grey, and not the colour
 * of the usage, so a selected usage or group, or the usages of a selected object, stand out from the others as a selected
 * component does; the titles of the muted ones are not shown, which `paintUsages` sees in `muted`. With null, dim none.
 */
function muteFrames(lit, force = false) {
  let changed = false;
  const grey = frameGrey();
  for (const f of frames) {
    const muted = lit !== null && !lit.has(f.entry.object);
    if (!force && f.muted === muted) continue;
    const colour = muted ? grey : f.color;
    f.muted = muted;
    f.group.toggleAttribute("data-muted", muted);
    f.rect.setAttribute("stroke", colour);
    f.fill.setAttribute("fill", colour);
    changed = true;
  }
  // the frames that stay lit are over those that are muted; with none muted, the frames are in the order they were made in
  if (changed) {
    for (const f of frames) if (f.muted) layer.append(f.group);
    for (const f of frames) if (!f.muted) layer.append(f.group);
  }
}
// the drawing's background decides how far a muted colour goes
globalThis.addEventListener?.("background-colors", () => { if (frames.length) muteFrames(litFrames(), true); });
/**
 * Light up the members of a usage in the drawing while a row of the object's usages is pointed at, as a
 * selection does, without selecting it; with null, the drawing goes back to what is selected.
 */
export function highlightUsage(object) {
  hovered = object;
  if (!context || !doc) return;
  const [, , groups] = context;
  const lit = object === null ? chosen() : usages().find(u => u.object === object);
  mute(groups, lit ? focusSet(lit) : null);
  repaint(); // the frames that are muted, and the titles that are shown
}

// Net and terminal roles of a virtual symbol denote its local connections, not every
// other symbol on the same electrical net. Whole component roles retain their geometry.
function localSymbol(u) {
  return u.frame?.local ?? false;
}
const HIDE_W = 24; // the width the button that hides a selected usage takes at the start of its title
/**
 * The button at the start of the selected usage's title, an eye with a slash on a box that looks like a button: it hides
 * the usage, as the switch in the Usage panel does (the same `change`), and unselects it, since its frame goes with it;
 * showing it again is the panel's. It takes the pointer events itself, so that a press on it is neither a click on the
 * drawing nor a drag. The keyboard goes on to the drawing, where it was before the usage was selected.
 */
function hideButton(entry, color, parent) {
  const label = m`Hide ${entry.id}`;
  const button = svgElement("g", { class: "usage-hide", role: "button", tabindex: 0, "aria-label": label, "pointer-events": "all", "data-usage": entry.object });
  // (the colours of the box are the style sheet's; these are for a page without it)
  const box = svgElement("rect", { class: "usage-hide-box", x: -2, y: -7.5, width: 19, height: 15, rx: 3, fill: "#f4f4f6", stroke: "#8a8a92", "stroke-width": 1 });
  const icon = svgElement("g", { fill: "none", stroke: color, "stroke-width": 1.4, "stroke-linecap": "round", "stroke-linejoin": "round", transform: "translate(0.5 0)", "pointer-events": "none" });
  append(icon, svgElement("path", { d: "M0 0 Q7 -5 14 0 Q7 5 0 0 Z" }), svgElement("circle", { cx: 7, cy: 0, r: 1.8 }), svgElement("path", { d: "M1.5 5 L12.5 -5" }));
  const tip = svgElement("title", {});
  tip.textContent = label;
  append(button, box, icon, tip);
  const hide = () => {
    change(entry, { visible: false });
    clearSelection();
    $("canvas")?.focus({ preventScroll: true });
  };
  for (const type of ["pointerdown", "pointerup", "pointercancel", "dblclick", "contextmenu"]) button.addEventListener(type, (e) => e.stopPropagation());
  button.addEventListener("click", (e) => { e.stopPropagation(); hide(); });
  button.addEventListener("keydown", (e) => {
    if (e.key !== "Enter" && e.key !== " ") return;
    e.preventDefault();
    e.stopPropagation();
    hide();
  });
  append(parent, button);
}
function badge(text, color, parent, cls, width, lead = 0) {
  const g = svgElement("g", { class: cls });
  const back = svgElement("rect", { rx: 3, fill: "var(--usage-label-bg)", "fill-opacity": .75 });
  const label = svgElement("text", { fill: color, "font-size": 13, "font-family": "system-ui, sans-serif", "dominant-baseline": "central", ...(lead ? { x: lead } : {}) });
  label.textContent = text;
  if (width > 0) { label.setAttribute("textLength", width); label.setAttribute("lengthAdjust", "spacingAndGlyphs"); }
  append(g, back, label);
  append(parent, g);
  return { g, back, label, width, lead };
}

/** Draw in screen pixels inside the scene SVG: the inverse view transform cancels its zoom. */
export function paintUsages(svg, view, groups) {
  context = [svg, view, groups];
  if (!svg || !view) {
    frames = []; layer?.remove(); underlay?.remove(); currentLayer?.remove(); underlay = layer = currentLayer = null;
    drawing = layerDoc = null; layerLevel = -1; layerRevision = -1; layerSelected = null; context = null;
    return;
  }
  const u = chosen();
  const focus = u?.object ?? null;
  const rebuild = drawing !== svg || layerDoc !== doc || layerLevel !== current ||
    layerRevision !== revision || layerSelected !== focus;
  if (rebuild) {
    currentLayer?.remove();
    currentLayer = svgElement("g", { class: "current-overlays", "pointer-events": "none",
      fill: "none", stroke: "#d9480f", "stroke-width": 8 / 3, "stroke-linecap": "round", "stroke-linejoin": "round" });
    if (currentsShown) for (const c of doc.levels[current].currents || []) {
      const [a, b, h, j] = c.points;
      append(currentLayer, svgElement("path", { class: "current-arrow", "data-terminal": c.terminal,
        "data-direction": c.direction, d: `M${a} L${b} M${h} L${b} L${j}` }));
    }
    layer?.remove();
    underlay?.remove();
    underlay = svgElement("g", { class: "usage-underlays", "pointer-events": "none" });
    svg.insertBefore(underlay, svg.firstChild);
    layer = svgElement("g", { class: "usage-overlays", "pointer-events": "none" });
    append(svg, layer);
    drawing = svg; layerDoc = doc; layerLevel = current; layerRevision = revision; layerSelected = focus;
    mute(groups, u ? focusSet(u) : null);
    frames = [];
    const levels = frameSteps(usages().filter((u) => u.frame?.bounds && !localSymbol(u) && (u.object === focus || visible(u))));
    for (const entry of [...usages()].sort((a, b) => Number(a.object === focus) - Number(b.object === focus))) {
      const active = entry.object === focus;
      if (!active && !visible(entry)) continue;
      const local = localSymbol(entry);
      const bounds = entry.frame?.bounds;
      if (!bounds) continue;
      const color = setting(entry).color;
      // a group that PRIORITY has turned off is drawn faint, its frame, fill and label
      const off = isGroup(entry) && stateOf(entry) !== "in-effect";
      const group = svgElement("g", { class: `usage-frame${active ? " selected" : ""}${off ? " disabled" : ""}`, "data-usage": entry.object, ...(off ? { opacity: DISABLED_OPACITY } : {}) });
      const fill = svgElement("rect", { class: "usage-fill", "data-usage": entry.object, fill: color, "fill-opacity": (active ? .2 : .1) * (off ? DISABLED_OPACITY : 1), rx: 10 });
      append(underlay, fill);
      // Overlay coordinates are CSS pixels: 1pt = 4/3px, independent of drawing zoom.
      const rect = svgElement("rect", { fill: "none", stroke: color,
        "stroke-width": active ? 8 / 3 : 4 / 3, "stroke-dasharray": "1 5", "stroke-linecap": "round", rx: 10 });
      append(group, rect);
      append(layer, group);
      // (the selected usage's title has the button that hides it in front of its text)
      // (one that is turned off in the panel and shown only because it is selected has none: the panel turns it on)
      const hideable = active && setting(entry).visible;
      const title = badge(labelOf(entry), color, group, "usage-title", entry.frame.title_width, hideable ? HIDE_W : 0);
      if (hideable) hideButton(entry, color, title.g);
      const roles = active ? entry.roles.map((r, i) => ({ ...r, anchor: entry.frame.roles[i], width: entry.frame.role_widths[i] })).filter(r => r.anchor).map(r => ({
        anchor: r.anchor,
        ...badge(r.name, color, group, "usage-role-label", r.width),
      })) : [];
      frames.push({ entry, color, group, bounds, rect, fill, title, roles, active, local, pad: (levels.get(entry.object) ?? 0) * FRAME_STEP });
    }
    append(svg, currentLayer);
    window.dispatchEvent(new Event("usage-layer"));
  }
  muteFrames(litFrames());
  layer.setAttribute("transform", `translate(${view.x} ${view.y}) scale(${1 / view.k})`);
  underlay.setAttribute("transform", layer.getAttribute("transform"));
  const project = (x, y) => [(x - view.x) * view.k, (y - view.y) * view.k];
  for (const f of frames) {
    const [x, y] = project(f.bounds[0], f.bounds[1]);
    const [right, bottom] = project(f.bounds[2], f.bounds[3]);
    const b = [x - f.pad, y - f.pad, right + f.pad, bottom + f.pad];
    f.screen = b;
    for (const [attr, value] of Object.entries({ x: b[0], y: b[1], width: b[2] - b[0], height: b[3] - b[1] })) {
      f.rect.setAttribute(attr, value); f.fill.setAttribute(attr, value);
    }
    const [titleX, titleY] = project(...f.entry.frame.title);
    f.titleAt = [titleX - f.pad, titleY - f.pad];
    placeBadge(f.title, ...f.titleAt, false);
    const titleVisible = !f.muted && (f.active || f.title.width + 16 <= right - x); // (the frame's own width, as the core decides for the image)
    f.title.g.setAttribute("display", titleVisible ? "inline" : "none");
    if (!titleVisible) f.title.screen = null;
    for (const role of f.roles) {
      placeBadge(role, ...project(...role.anchor), true);
    }
  }
  spreadTitles();
}
/**
 * The titles that would lie on another's go down a row each (titleRows), the outer frames' titles, and the selected
 * one's, staying where they are: frames that cross without one holding the other, and the frames below the outer
 * three of a nest, have titles at one place otherwise.
 */
function spreadTitles() {
  const shown = frames.filter((f) => f.title.screen);
  const rows = titleRows(shown.map((f) => ({ key: f, x: f.titleAt[0], y: f.titleAt[1], width: f.title.width + f.title.lead, first: (f.active ? 1e6 : 0) + f.pad })));
  for (const f of shown) if (rows.get(f)) placeBadge(f.title, f.titleAt[0], f.titleAt[1] + rows.get(f) * TITLE_ROW, false);
}
function placeBadge(b, x, y, centered) {
  const width = b.width ??= b.label.getComputedTextLength();
  b.g.setAttribute("transform", `translate(${centered ? x - width / 2 : x} ${y})`);
  for (const [attr, value] of Object.entries({ x: -4, y: -9, width: width + 8 + b.lead, height: 18 })) b.back.setAttribute(attr, value);
  b.screen = [centered ? x - width / 2 - 4 : x - 4, y - 9, centered ? x + width / 2 + 4 : x + width + 4 + b.lead, y + 9];
}

/** Only the border/title selects a usage; its filled interior keeps normal component picking. */
export function usageAt({ x, y }) {
  return usagesAt({ x, y })[0] ?? null;
}
export function usagesAt({ x, y }) {
  const contains = (b, pad = 0) => b && x >= b[0] - pad && x <= b[2] + pad && y >= b[1] - pad && y <= b[3] + pad;
  const hits = [];
  for (const f of [...frames].reverse()) {
    if (!visible(f.entry)) continue;
    if (contains(f.title.screen) || (contains(f.screen, 4) && !contains([f.screen[0] + 5, f.screen[1] + 5, f.screen[2] - 5, f.screen[3] - 5]))) hits.push(f.entry.object);
  }
  return hits;
}
window.addEventListener("viewer-level", () => { revision++; renderUsages(); persist(); });
/** A usage that is selected, from the drawing, the text or the table of an object's usages, has its group open, and is in view. */
function showSelected() {
  newScope();
  const u = chosen();
  if (u) {
    expanded.add(typeOf(u));
    const items = usages().filter((x) => typeOf(x) === typeOf(u));
    const at = items.findIndex((x) => x.object === u.object);
    if (at >= (rowLimits.get(typeOf(u)) || 100)) rowLimits.set(typeOf(u), Math.ceil((at + 1) / 100) * 100);
  }
  revealPending = !!u;
  renderUsages();
}
window.addEventListener("viewer-selection", showSelected);
window.addEventListener("language-changed", renderUsages);
