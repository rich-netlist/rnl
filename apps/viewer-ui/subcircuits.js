// Going down into a subcircuit instance and back up the path that led there, each level with the
// view it had when it was left.

import { current, doc, treeActive, viewCache } from "./state.js";
import { showLevel, view } from "./drawing.js";
import { pathTo } from "./hierarchy.js";
import { clearSelection, select } from "./selection.js";
import { $ } from "./util.js";

/** The level a subcircuit instance opens, or null. */
export function opens(o) {
  const obj = doc && o !== null && o !== undefined ? doc.objects[o] : null;
  return obj && obj.kind === "instance" && obj.opens !== null && obj.opens !== undefined ? obj.opens : null;
}

/** The tree path of the level shown: the row that opened it, or the first row that reaches it. */
function herePath() {
  return treeActive !== null ? treeActive : pathTo(current);
}

/** The level a tree path reaches. */
function levelAt(path) {
  let l = 0;
  for (const k of path.split("/").slice(1).map(Number)) l = doc.levels[l].children[k].level;
  return l;
}

export function rememberView() {
  const here = herePath();
  if (here !== null && view) viewCache.set(here, view);
}

/** Open the subcircuit of instance `o`, below the level shown. */
export function descend(o) {
  const level = opens(o);
  if (level === null) return;
  const name = doc.objects[o].name;
  const here = herePath();
  const k = doc.levels[current].children.findIndex((c) => c.instance === name);
  const path = here !== null && k >= 0 ? `${here}/${k}` : null;
  rememberView();
  clearSelection();
  showLevel(level, path !== null ? viewCache.get(path) || null : null, path);
}

/** The level above the one shown, along its tree path, and the instance that leads down; or null. */
function parentOf() {
  if (!doc || doc.levels.length === 0 || current === 0) return null;
  const here = herePath();
  // a subcircuit nothing instantiates still goes up to the top level
  if (here === null || !here.includes("/")) return { level: 0, path: "0", via: null };
  const parts = here.split("/");
  const path = parts.slice(0, -1).join("/");
  const level = levelAt(path);
  const child = doc.levels[level].children[Number(parts[parts.length - 1])];
  return { level, path, via: child ? child.instance : null };
}

export function ascend() {
  const p = parentOf();
  if (!p) return;
  rememberView();
  showLevel(p.level, viewCache.get(p.path) || null, p.path);
  if (p.via === null) return;
  // the instance we came out of stays selected, so the way back down is one Enter away
  const host = doc.levels[p.level].host;
  const o = doc.objects.findIndex((x) => x.kind === "instance" && x.level === host && x.name === p.via);
  if (o >= 0) select(o, "ascend");
}

export function updateUp() {
  const p = parentOf();
  $("up").hidden = !p;
  if (!p) return;
  const name = doc.levels[p.level].title;
  $("up-name").textContent = name;
  $("up").title = `Up to ${name} (Backspace)`;
}

$("up").addEventListener("click", ascend);
