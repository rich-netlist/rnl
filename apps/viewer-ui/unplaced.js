// The "Not drawn" view of the side panel: the objects of the level shown that have no position, so
// that nothing draws them until a solver places them. Each is listed with its symbol and the nets at
// its terminals as the core drew them (`Level.unplaced`), and its name. The list loads a part at a
// time, the next when it is scrolled to its end, and is searched by name; picking an object selects
// it and shows its statement in the netlist. The lines of those statements are greyed in the
// netlist (text.js, `markUnplaced`).

import { setPane, setSidebar } from "./panels.js";
import { select } from "./selection.js";
import { current, doc } from "./state.js";
import { revealLine } from "./text.js";
import { $, span } from "./util.js";

const BATCH = 20; // the objects the list adds when it is scrolled to its end
const NEAR_END = 60; // pixels from the end of the list at which the next part is added

const list = $("unplaced-list");
const search = $("unplaced-search");
let found = []; // the objects of the level shown that the search finds, in netlist order
let rows = 0; // how many of them have a row in the list
let scroll = 0; // the list's scroll, which a pane that is hidden loses

/** The objects of the level shown that are not drawn. */
function unplaced() {
  const level = doc && doc.levels[current];
  return level ? level.unplaced : [];
}

/**
 * Show the objects of the level shown, from the first, with an empty search; the count in the tab
 * and the status bar's way to the list follow the level.
 */
export function renderUnplaced() {
  search.value = "";
  const n = unplaced().length;
  $("unplaced-tab-count").textContent = n ? ` (${n})` : "";
  const status = $("status");
  status.classList.toggle("opens-unplaced", n > 0);
  if (n > 0) status.title = "Show the objects that are not drawn";
  rebuild();
}

/** Make the list of the objects the search finds, from the first. */
function rebuild() {
  const all = unplaced();
  const q = search.value.trim().toLowerCase();
  found = all.filter((u) => !q || u.name.toLowerCase().includes(q));
  rows = 0;
  scroll = 0;
  list.replaceChildren();
  list.scrollTop = 0;
  $("unplaced-count").textContent = !doc || all.length === 0 ? "" : q ? `${found.length} of ${all.length}` : String(all.length);
  if (!doc || all.length === 0 || found.length === 0) {
    const p = document.createElement("div");
    p.className = "nothing";
    p.textContent = !doc
      ? "No file is open."
      : all.length === 0
        ? "Every object of this level has a position."
        : "No object matches.";
    list.append(p);
    return;
  }
  fill();
}

/**
 * Add rows while the list has none yet, and while it is not scrolled away from its end: the first
 * part, and the next when the end comes near. A list that is hidden has no height to measure, so it
 * is filled when it is shown.
 */
function fill() {
  if (list.clientHeight === 0 || rows >= found.length) return;
  const more = list.querySelector(".more");
  if (more) more.remove();
  while (rows < found.length && (rows === 0 || list.scrollHeight - list.scrollTop - list.clientHeight < NEAR_END)) {
    const q = search.value.trim().toLowerCase();
    for (const u of found.slice(rows, rows + BATCH)) list.append(row(u, q));
    rows = Math.min(rows + BATCH, found.length);
  }
  if (rows < found.length) {
    const rest = document.createElement("div");
    rest.className = "more";
    rest.textContent = `${rows} of ${found.length} shown: scroll for more`;
    list.append(rest);
  }
}

/** A row: the object's symbol, its name, and what it is. */
function row(u, q) {
  const el = document.createElement("div");
  el.className = "unplaced-row";
  el.setAttribute("role", "option");
  el.tabIndex = 0;
  el.dataset.name = u.name;
  const located = u.objects.length > 0 && doc.objects[u.objects[0]].lines !== null;
  el.classList.toggle("located", located);
  el.title = located ? "Show in the netlist" : "This object is written in a record, not in a statement";
  el.append(thumbnail(u));
  const name = span("name", "");
  const at = q ? u.name.toLowerCase().indexOf(q) : -1;
  if (at >= 0) {
    const m = document.createElement("mark");
    m.textContent = u.name.slice(at, at + q.length);
    name.append(u.name.slice(0, at), m, u.name.slice(at + q.length));
  } else {
    name.textContent = u.name;
  }
  const info = span("info", "");
  info.append(name, span("symbol", `${u.kind} · ${u.symbol}`));
  el.append(info);
  el.addEventListener("click", () => pick(u));
  el.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      pick(u);
    }
  });
  return el;
}

/** The symbol drawn alone, as the core wrote it; nothing when the page was given no SVG. */
function thumbnail(u) {
  const box = span("thumb", "");
  if (!u.svg) return box;
  const el = document.importNode(new DOMParser().parseFromString(u.svg, "image/svg+xml").documentElement, true);
  el.removeAttribute("width");
  el.removeAttribute("height");
  el.setAttribute("aria-hidden", "true");
  box.append(el);
  return box;
}

/** Select the object and show its statement in the netlist. */
function pick(u) {
  if (u.objects.length === 0) return;
  const o = u.objects[0];
  select(o, "list");
  const lines = doc.objects[o].lines;
  if (!lines) return;
  setPane("netlist");
  revealLine(lines[0], lines[1], true);
}

/** The list has been shown again: its scroll comes back, and it loads what its height asks for. */
export function showUnplaced() {
  fill();
  list.scrollTop = scroll;
}

list.addEventListener("scroll", () => {
  scroll = list.scrollTop;
  fill();
});
search.addEventListener("input", rebuild);
search.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && search.value) {
    search.value = "";
    rebuild();
    e.stopPropagation();
  } else if (e.key === "Enter") {
    if (found.length > 0) pick(found[0]);
  }
});
// the status bar says how many are not drawn: it opens the list
$("status").addEventListener("click", () => {
  if (!$("status").classList.contains("opens-unplaced")) return;
  setSidebar(true);
  setPane("unplaced");
});
