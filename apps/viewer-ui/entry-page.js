import { append, m, setAttr, setText } from "./i18n.js";
// The entry page: an entry that only includes other files, read alone (a shell with `reader`), has an empty top level,
// and the subcircuits that nothing calls are in the other files (roots.js). In place of the empty drawing, a card says
// so and lists those subcircuits, each with its symbol, the file that defines it, its instances, and whether its file's
// reading is kept, else how long it took the last time (file-reading.js `fileSummaries`); a click opens it, as the index does.
// With one alone, a file opened opens it at once, and the card shows meanwhile, with no symbols: they are not read for it.

import { current, doc } from "./state.js";
import { fileSummaries, readsByFile } from "./file-reading.js";
import { levelTree, localLevel } from "./level-tree.js";
import { uncalledRoots } from "./roots.js";
import { enterHost } from "./subcircuits.js";
import { portSymbol } from "./module-view.js";
import { colourDrawing } from "./background.js";
import { $, span } from "./util.js";

const card = $("entry-card");
// whether the open under way opens the one root of its entry page at once (open.js `openOnlyRoot`)
let opensOnlyRoot = false;

/** Say whether the open under way opens the one root of the entry page it shows at once (open.js). */
export function setOpensOnlyRoot(on) {
  opensOnlyRoot = on;
}

/**
 * The host levels the entry page lists for the level `level` of the Document shown: the subcircuits nothing calls, of
 * other files, when it is the empty top level of an entry read alone; none otherwise.
 */
function entryRoots(level) {
  const top = doc?.levels[level];
  if (!doc?.outline || level !== 0 || !top || !top.empty || top.unplaced?.length || doc.file !== doc.outline.files[0]?.path) return [];
  const { nodes } = levelTree();
  return uncalledRoots().filter((h) => nodes[h] && !nodes[h].module && localLevel(h) < 0);
}

/** Show the entry page in place of the empty drawing of the level `level`, when it is one; false otherwise. */
export function showEntryPage(level) {
  const roots = entryRoots(level);
  if (roots.length === 0) return false;
  const { nodes } = levelTree();
  card.replaceChildren();
  const heading = document.createElement("div");
  heading.className = "entry-heading";
  setText(heading, m`${doc.outline.files[0].name ?? doc.file} only includes other files`);
  const note = document.createElement("p");
  note.className = "entry-note";
  setText(note, roots.length === 1 ? m`Opening ${nodes[roots[0]].title}, which nothing calls.` : m("Open one of the subcircuits that nothing calls:"));
  const list = document.createElement("div");
  list.className = "entry-roots";
  const items = new Map();
  for (const host of roots) {
    const item = rootItem(host);
    items.set(host, item);
    append(list, item.row);
  }
  append(card, heading, note, list);
  card.hidden = false;
  // (a root opened at once needs no symbol, nor the core to read its file for one)
  if (!(roots.length === 1 && opensOnlyRoot)) void summarize(roots, items);
  return true;
}

/** The row of a root: a button with its symbol, its title, its file, its instances, and where its status goes. */
function rootItem(host) {
  const node = levelTree().nodes[host];
  const level = doc.outline.levels[host];
  const row = document.createElement("button");
  row.type = "button";
  row.className = "entry-root";
  row.dataset.level = String(host);
  const thumb = document.createElement("span");
  thumb.className = "entry-symbol";
  append(thumb, portSymbol(level.ports));
  const text = document.createElement("span");
  text.className = "entry-text";
  const status = span("entry-status", "");
  append(text, span("entry-title", node.title), span("entry-file", node.file ?? ""),
    span("entry-instances", level.instances === 1 ? m("1 instance") : m`${level.instances} instances`), status);
  append(row, thumb, text);
  setAttr(row, "title", m`Open ${node.title}`);
  row.addEventListener("click", (e) => enterHost(host, { newWindow: e.shiftKey }));
  return { row, thumb, status };
}

/** Fill in the symbols the roots give themselves, and the state of their files' readings, when they are known. */
async function summarize(roots, items) {
  if (!readsByFile()) return;
  const shown = doc;
  const { nodes } = levelTree();
  const files = [...new Set(roots.map((h) => nodes[h].file).filter((f) => f !== null))];
  let summaries;
  try { summaries = await fileSummaries(files); } catch { return; }
  if (doc !== shown || card.hidden) return;
  for (const host of roots) {
    const s = summaries[nodes[host].file];
    const item = items.get(host);
    if (!s || !item) continue;
    const symbol = s.subckts?.find((x) => x.host === host)?.symbol;
    if (symbol?.svg) {
      const svg = drawn(symbol.svg);
      if (svg) item.thumb.replaceChildren(svg);
    }
    if (s.kept) setText(item.status, m("Kept: opens at once"));
    else if (s.took !== null) setText(item.status, m`Read in ${(s.took / 1000).toFixed(1)} s the last time`);
  }
}

/** The SVG of a symbol, to show small. */
function drawn(text) {
  const parsed = new DOMParser().parseFromString(text, "image/svg+xml");
  if (parsed.querySelector("parsererror")) return null;
  const svg = document.importNode(parsed.documentElement, true);
  svg.removeAttribute("width");
  svg.removeAttribute("height");
  svg.setAttribute("aria-hidden", "true");
  colourDrawing(svg);
  // on the card's own colour, not on the drawing's backdrop
  svg.querySelector("rect.backdrop")?.remove();
  return svg;
}

/** Take the entry page away, as a level is drawn. */
export function hideEntryPage() {
  card.hidden = true;
}

/** A file opened on the entry page of one root alone: open that root, as the page did before it read files alone. */
export function openOnlyRoot() {
  const roots = card.hidden ? [] : entryRoots(current);
  if (roots.length === 1) enterHost(roots[0]);
}

// the card is no drawing: the wheel scrolls it, and a press or a double click on it is not one on the drawing
for (const type of ["wheel", "pointerdown", "dblclick", "mousedown", "contextmenu"]) card.addEventListener(type, (e) => e.stopPropagation());
