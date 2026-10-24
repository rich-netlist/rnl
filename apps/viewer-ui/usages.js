// Scope-local semantic groups. Display preferences never change the RNL source or its geometry.
import { append, m, setAttr, setText } from "./i18n.js";
import { current, doc, selected, selectedAsUsage } from "./state.js";
import { select } from "./selection.js";
import { setCheck } from "./commands.js";
import { $, load, save, span, svgElement } from "./util.js";
import { setPathText } from "./source-path.js";

export const USAGE_COLORS = ["#e03131", "#e5b500", "#4263eb", "#2f9e44", "#9c36b5", "#009ccc"];
const MAX_PREFERENCES = 2000;
let preferences = new Map();
const activeSettings = new WeakMap();
let shown = true;
let currentsShown = false;
let currentLayer = null;
let revision = 0;
const collapsed = new Set();
const expandedLarge = new Set();
const rowLimits = new Map();
let drawing = null;
let layer = null;
let underlay = null;
let frames = [];
let layerDoc = null;
let layerLevel = -1;
let layerRevision = -1;
let layerSelected = null;
let context = null;

const usages = () => doc?.levels[current]?.usages || [];
const chosen = () => selectedAsUsage ? usages().find(u => u.object === selected) : null;
const visible = u => shown && setting(u).visible;
/** Hidden virtual symbols still select as ordinary parts when picked on the drawing. */
export function usageSelection(object, from) {
  const u = usages().find(u => u.object === object);
  return !!u && (from === "usage" || from === "source" || visible(u));
}
const keyOf = u => JSON.stringify([doc.path, doc.levels[current].kind, doc.levels[current].title, u.id]);
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
  const scroll = list.scrollTop;
  const focused = list.contains(document.activeElement) ? document.activeElement.dataset.focusKey : null;
  list.replaceChildren();
  const all = usages();
  const inspected = chosen()?.object;
  if (!all.length) append(list, span("placeholder", m("No usages in this circuit.")));
  const groups = new Map();
  for (const u of all) {
    if (!groups.has(u.definition)) groups.set(u.definition, []);
    groups.get(u.definition).push(u);
  }
  for (const [type, items] of [...groups].sort(([a], [b]) => a.localeCompare(b, undefined, { numeric: true }))) {
    const group = document.createElement("section");
    group.className = "usage-group";
    group.dataset.type = type;
    const head = document.createElement("div");
    head.className = "usage-group-head";
    const expand = document.createElement("button");
    expand.type = "button";
    expand.className = "usage-expand";
    expand.dataset.focusKey = `type:${type}`;
    const folded = collapsed.has(type) || (items.length > 100 && !expandedLarge.has(type));
    expand.setAttribute("aria-expanded", String(!folded));
    append(expand, span("chevron", ""), type, span("badge", `(${items.length})`));
    expand.addEventListener("click", () => {
      if (folded) { collapsed.delete(type); expandedLarge.add(type); } else collapsed.add(type);
      renderUsages();
    });
    const visible = items.filter(u => setting(u).visible).length;
    const control = checkbox(m`Show all usages of ${type}`, visible === items.length, value => {
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
        append(row, button, check);
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
  renderDetail();
}

function renderDetail() {
  const detail = $("usage-detail");
  detail.replaceChildren();
  const u = chosen();
  if (!u) { append(detail, span("placeholder", m("Select a usage to inspect it."))); return; }
  const title = document.createElement("h2");
  setText(title, `${u.definition}: ${u.id}`);
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

// Net and terminal roles of a virtual symbol denote its local connections, not every
// other symbol on the same electrical net. Whole component roles retain their geometry.
function localSymbol(u) {
  return u.frame?.local ?? false;
}
function badge(text, color, parent, cls, width) {
  const g = svgElement("g", { class: cls });
  const back = svgElement("rect", { rx: 3, fill: "var(--usage-label-bg)", "fill-opacity": .75 });
  const label = svgElement("text", { fill: color, "font-size": 13, "font-family": "system-ui, sans-serif", "dominant-baseline": "central" });
  label.textContent = text;
  if (width > 0) { label.setAttribute("textLength", width); label.setAttribute("lengthAdjust", "spacingAndGlyphs"); }
  append(g, back, label);
  append(parent, g);
  return { g, back, label, width };
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
    const keep = u ? new Set(localSymbol(u) ? [] : u.roles.flatMap(r => r.groups)) : null;
    if (keep && !localSymbol(u)) {
      const members = new Set(u.roles.flatMap(r => r.objects));
      for (const h of doc.levels[current].hits) if (h.id && h.objects.some(o => members.has(o))) keep.add(h.id);
    }
    // A virtual symbol is local to one usage; its attached net may span the entire drawing.
    if (keep) for (const h of doc.levels[current].hits) if (h.objects.includes(u.object) && h.id) keep.add(h.id);
    for (const [id, elements] of groups) for (const g of elements) {
      g.classList.toggle("usage-muted", !!keep && !keep.has(id) && !g.parentElement.closest("[data-id]"));
    }
    frames = [];
    for (const entry of [...usages()].sort((a, b) => Number(a.object === focus) - Number(b.object === focus))) {
      const active = entry.object === focus;
      if (!active && !visible(entry)) continue;
      const local = localSymbol(entry);
      const bounds = entry.frame?.bounds;
      if (!bounds) continue;
      const color = setting(entry).color;
      const group = svgElement("g", { class: `usage-frame${active ? " selected" : ""}`, "data-usage": entry.object });
      const fill = svgElement("rect", { class: "usage-fill", "data-usage": entry.object, fill: color, "fill-opacity": active ? .2 : .1, rx: 10 });
      append(underlay, fill);
      // Overlay coordinates are CSS pixels: 1pt = 4/3px, independent of drawing zoom.
      const rect = svgElement("rect", { fill: "none", stroke: color,
        "stroke-width": active ? 8 / 3 : 4 / 3, "stroke-dasharray": "1 5", "stroke-linecap": "round", rx: 10 });
      append(group, rect);
      append(layer, group);
      const title = badge(`${entry.definition}: ${entry.id}`, color, group, "usage-title", entry.frame.title_width);
      const roles = active ? entry.roles.map((r, i) => ({ ...r, anchor: entry.frame.roles[i], width: entry.frame.role_widths[i] })).filter(r => r.anchor).map(r => ({
        anchor: r.anchor,
        ...badge(r.name, color, group, "usage-role-label", r.width),
      })) : [];
      frames.push({ entry, bounds, rect, fill, title, roles, active, local });
    }
    append(svg, currentLayer);
  }
  layer.setAttribute("transform", `translate(${view.x} ${view.y}) scale(${1 / view.k})`);
  underlay.setAttribute("transform", layer.getAttribute("transform"));
  const project = (x, y) => [(x - view.x) * view.k, (y - view.y) * view.k];
  for (const f of frames) {
    const [x, y] = project(f.bounds[0], f.bounds[1]);
    const [right, bottom] = project(f.bounds[2], f.bounds[3]);
    const b = [x, y, right, bottom];
    f.screen = b;
    for (const [attr, value] of Object.entries({ x: b[0], y: b[1], width: b[2] - b[0], height: b[3] - b[1] })) {
      f.rect.setAttribute(attr, value); f.fill.setAttribute(attr, value);
    }
    placeBadge(f.title, ...project(...f.entry.frame.title), false);
    const titleVisible = f.active || f.title.width + 16 <= b[2] - b[0];
    f.title.g.setAttribute("display", titleVisible ? "inline" : "none");
    if (!titleVisible) f.title.screen = null;
    for (const role of f.roles) {
      placeBadge(role, ...project(...role.anchor), true);
    }
  }
}
function placeBadge(b, x, y, centered) {
  const width = b.width ??= b.label.getComputedTextLength();
  b.g.setAttribute("transform", `translate(${centered ? x - width / 2 : x} ${y})`);
  for (const [attr, value] of Object.entries({ x: -4, y: -9, width: width + 8, height: 18 })) b.back.setAttribute(attr, value);
  b.screen = [centered ? x - width / 2 - 4 : x - 4, y - 9, centered ? x + width / 2 + 4 : x + width + 4, y + 9];
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
window.addEventListener("viewer-selection", renderUsages);
