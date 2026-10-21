import { append, m, setAttr, setText } from "./i18n.js";
// Inspect the exact runtime symbol chosen on the navigation path, without rebuilding the circuit.
import { current, doc, path } from "./state.js";
import { colourDrawing } from "./background.js";
import { lineModel } from "./text-lines.js";
import { $, load, save, svgElement } from "./util.js";
import { svgViewer } from './svg-viewer.js';
import { setPathText, sourcePath } from './source-path.js';

const parts = ["box", "origin", "pins", "labels"];
let lastDoc, lastKey;
let destroyPreview;
const node = (tag, text, cls) => {
  const el = document.createElement(tag);
  if (text !== undefined) setText(el, text);
  if (cls) el.className = cls;
  return el;
};
const number = n => Number(n.toFixed(4));
const point = p => `(${p.map(number).join(", ")})`;
const direction = d => m(({0: "→ right", 90: "↑ up", 180: "← left", 270: "↓ down"})[d] || "—");
// Provenance captions emitted by document/symbols.rs; names and diagnostic origins stay raw.
function originText(origin) {
  if (origin.startsWith("Defined in ")) return m`Defined in ${origin.slice(11)}`;
  if (origin.startsWith("Package ")) return m`Package ${origin.slice(8)}`;
  return ["Viewer default symbol", "Inline symbol"].includes(origin) ? m(origin) : origin;
}

function displayParts() {
  for (const part of parts) {
    const shown = $(`symbol-${part}`).checked;
    $("symbol-detail").classList.toggle(`hide-symbol-${part}`, !shown);
  }
}
for (const part of parts) $(`symbol-${part}`).addEventListener("change", () => {
  save(`symbol-${part}`, $(`symbol-${part}`).checked);
  displayParts();
});

export function renderSymbol() {
  if (lastDoc !== doc) {
    destroyPreview?.(); destroyPreview = null;
    lastDoc = null; lastKey = null;
    $("symbol-detail").replaceChildren();
  }
  if ($("pane-symbol").hidden) return;
  const level = doc?.levels[current], hop = path.at(-1), parent = path.at(-2);
  const child = parent && hop?.via ? doc?.levels[parent.level]?.children.find(c => c.instance === hop.via) : null;
  const symbol = doc?.symbols?.[child ? child.symbol : level?.symbol];
  const key = `${current}:${parent?.level}:${hop?.via}`;
  if (lastDoc === doc && lastKey === key) return;
  lastDoc = doc; lastKey = key;
  for (const part of parts) $(`symbol-${part}`).checked = load(`symbol-${part}`, true);
  displayParts();
  const detail = $("symbol-detail");
  destroyPreview?.(); destroyPreview = null;
  detail.replaceChildren();
  if (!symbol) {
    append(detail, node("p", !level ? m("No circuit is open.") : child ? m("This instance has no symbol.") : m("This circuit does not define its own symbol. Descend from a parent instance to inspect the symbol used there."), "placeholder"));
    return;
  }
  append(detail, node("h2", symbol.name, "symbol-name"));
  append(detail, node("p", child ? m`${hop.via} in ${doc.levels[parent.level].title} → ${level.title}` : m`${level.title} · own symbol`, "symbol-context"));
  append(detail, preview(symbol));
  append(detail, node("p", originText(symbol.origin), "symbol-origin-info"));
  const table = node("table", undefined, "symbol-pins-table");
  const head = node("tr");
  for (const title of [m("Terminal"), m("Name"), m("Direction"), m("Position")]) append(head, node("th", title));
  const thead = node("thead"); append(thead, head); append(table, thead);
  const tbody = node("tbody");
  for (const p of symbol.pins) {
    const names = p.terminals.map(t => /^\d+$/.test(t) ? level.ports?.[Number(t) - 1] || "" : t).filter(Boolean);
    const row = node("tr");
    for (const text of [p.terminals.join(", "), p.name || names.join(", ") || "—", direction(p.direction), point(p.at)]) append(row, node("td", text));
    append(tbody, row);
  }
  append(table, tbody);
  const scroll = node("div", undefined, "symbol-table-scroll"); append(scroll, table); append(detail, scroll);
  append(detail, node("p", m`Box = [${symbol.bbox.map(number).join(", ")}]`, "symbol-bounds"));
  append(detail, node("h3", m("Labels")));
  const labels = node("ul", undefined, "symbol-label-list");
  for (const l of symbol.labels) append(labels, node("li", m`${l.key || m("(unkeyed)")} = ${l.text}${l.at ? ` · ${point(l.at)}` : m(" · automatic position")}`));
  if (!symbol.labels.length) append(labels, node("li", m("No labels.")));
  append(detail, labels, node("h3", m("Definition source")));
  if (symbol.file) {
    const file = doc.files.find(f => f.path === symbol.file);
    const location = node('div', undefined, 'symbol-source-location path-text');
    setPathText(location, file ? sourcePath(doc, file) : symbol.file);
    append(detail, location);
    const last = symbol.first_line + (symbol.source ? symbol.source.split('\n').length - 1 : 0);
    append(detail, node('p', m`Lines ${symbol.first_line}–${last}`, 'symbol-source-lines'));
  }
  if (symbol.source) {
    const lines = symbol.source.split("\n");
    const model = lineModel({ text: symbol.source, records: [[1, 1, lines.length, [...lines.at(-1)].length]], failed: [] }, 0, [], "none");
    const code = node("div", undefined, "symbol-definition");
    for (let i = 0; i < lines.length; i++) {
      const row = model.render(i);
      setText(row.querySelector('.no'), String(symbol.first_line + i));
      append(code, row);
    }
    append(detail, code);
  } else append(detail, node("p", symbol.default ? m("Generated by the viewer; no RNL definition.") : m("Definition source is unavailable."), "placeholder"));
}

function preview(symbol) {
  const wrap = node("div", undefined, "symbol-preview");
  const parsed = new DOMParser().parseFromString(symbol.svg, "image/svg+xml");
  const svg = document.importNode(parsed.documentElement, true);
  svg.removeAttribute("width"); svg.removeAttribute("height");
  svg.setAttribute("role", "img"); setAttr(svg, "aria-label", m`${symbol.name} symbol preview`);
  const scale = symbol.scale;
  const [x0, y0, x1, y1] = symbol.bbox;
  const box = svgElement("rect", {x: x0 * scale, y: -y1 * scale, width: (x1-x0)*scale, height: (y1-y0)*scale,
    fill: "none", stroke: "#c04b08", "stroke-dasharray": "5 4", "stroke-width": 1.5, "vector-effect": "non-scaling-stroke", class: "symbol-box-mark"});
  append(svg, box);
  const origin = svgElement("path", {d: "M-5 0H5M0 -5V5", fill: "none", stroke: "#d32f2f", "stroke-width": 1.5, class: "symbol-origin-mark"});
  append(svg, origin);
  const pins = svgElement("g", {class: "symbol-pin-marks", fill: "#2454d8", "font-size": 11});
  let bounds = [Math.min(0, x0*scale), Math.min(0, -y1*scale), Math.max(0, x1*scale), Math.max(0, -y0*scale)];
  const vb = svg.getAttribute("viewBox").split(/[\s,]+/).map(Number);
  bounds = [Math.min(bounds[0], vb[0]), Math.min(bounds[1], vb[1]), Math.max(bounds[2], vb[0]+vb[2]), Math.max(bounds[3], vb[1]+vb[3])];
  for (const p of symbol.pins) {
    const [x, y] = [p.at[0]*scale, -p.at[1]*scale];
    append(pins, svgElement("circle", {cx: x, cy: y, r: 3, fill: "white", stroke: "#2454d8", "stroke-width": 1.5}));
    const mark = svgElement("text", {x: x+5, y: y-6});
    setText(mark, p.terminals.join(",") + ({0:"→",90:"↑",180:"←",270:"↓"}[p.direction] || ""));
    append(pins, mark);
    bounds = [Math.min(bounds[0],x-5), Math.min(bounds[1],y-20), Math.max(bounds[2],x+10+mark.textContent.length*7), Math.max(bounds[3],y+5)];
  }
  append(svg, pins);
  const pad = 16;
  svg.setAttribute("viewBox", `${bounds[0]-pad} ${bounds[1]-pad} ${Math.max(1,bounds[2]-bounds[0])+pad*2} ${Math.max(1,bounds[3]-bounds[1])+pad*2}`);
  colourDrawing(svg);
  destroyPreview = svgViewer(wrap, svg, scale);
  return wrap;
}

window.addEventListener("viewer-level", renderSymbol);
