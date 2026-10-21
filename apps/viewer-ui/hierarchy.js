import { append, m, setAttr, setText } from "./i18n.js";
// The hierarchy view: the subcircuit instances as a tree under the top level, below it the
// subcircuits that the top level does not call as trees of their own, and every subcircuit by name
// under its initial, with a search box. Picking a row opens its level: a row of the tree by the way
// to it from its root, a subcircuit of the index as the root of the path.
//
// The paths of the rows: "0" is the top level and "0/1/0" the first instance of the second instance
// of the top level; "r3/0" is the first instance of the subcircuit 3 that the top level does not
// call (roots.js).

import { current, doc, expanded } from "./state.js";
import { uncalledRoots } from "./roots.js";
import { enterRoot, enterRow, pathRow } from "./subcircuits.js";
import { $, span } from "./util.js";

let active = null; // the tree row of the level shown: its path's, or else the first that reaches it

export function renderHierarchy() {
  renderTree();
  renderIndex();
}

const ROOT = { root: true }; // the `child` of a row that starts a tree of an uncalled subcircuit

/** The path of the row of an uncalled subcircuit's level. */
export function rootPath(level) {
  return `r${level}`;
}

/**
 * The tree of subcircuit instances under the top level, and below it, in a group of their own, those
 * under each subcircuit that the top level does not call; only open branches are listed.
 */
function renderTree() {
  const tree = $("tree");
  tree.replaceChildren();
  if (!doc || doc.levels.length === 0) return;
  const walk = (level, path, depth, child, above) => {
    const lv = doc.levels[level];
    const cycle = above.includes(level);
    const branches = cycle ? [] : lv.children;
    const open = branches.length > 0 && expanded.has(path);
    append(tree, treeRow({ level, path, depth, child, branches: branches.length, open, cycle }));
    if (!open) return;
    branches.forEach((c, k) => {
      const p = `${path}/${k}`;
      if (c.level === null) append(tree, treeRow({ level: null, path: p, depth: depth + 1, child: c, branches: 0 }));
      else walk(c.level, p, depth + 1, c, above.concat(level));
    });
  };
  walk(0, "0", 0, null, []);
  const roots = uncalledRoots();
  if (roots.length === 0) return;
  const heading = document.createElement("div");
  heading.className = "tree-group";
  setText(heading, m("Not called from the top level"));
  append(tree, heading);
  for (const r of roots) walk(r, rootPath(r), 0, ROOT, []);
}

function treeRow({ level, path, depth, child, branches, open, cycle }) {
  const row = document.createElement("div");
  row.className = "tree-row";
  row.setAttribute("role", "treeitem");
  row.dataset.path = path;
  row.style.paddingLeft = `${4 + depth * 16}px`;
  const twisty = document.createElement("button");
  twisty.type = "button";
  twisty.className = branches ? "twisty" : "twisty leaf";
  twisty.tabIndex = -1;
  if (branches) {
    twisty.setAttribute("aria-expanded", String(open));
    setAttr(twisty, "aria-label", open ? m("Collapse") : m("Expand"));
    twisty.addEventListener("click", (e) => {
      e.stopPropagation();
      toggleBranch(path);
    });
  }
  append(row, twisty);
  if (child === null) {
    append(row, span("inst", doc.levels[0].title), span("badge", m("top")));
  } else if (child === ROOT) {
    append(row, span("inst", doc.levels[level].title), span("badge", m("not called")));
    setAttr(row, "title", m`No instance calls ${doc.levels[level].title}`);
  } else {
    append(row, span("inst", child.instance), span("master", child.master));
    if (level === null) append(row, span("badge", m("not defined")));
    if (cycle) append(row, span("badge", m("recursive")));
  }
  if (level === null) {
    row.classList.add("undefined");
    setAttr(row, "title", m`${child.master} is not defined in the netlist`);
    return row;
  }
  row.dataset.level = String(level);
  if (level === current) row.classList.add("current");
  if (path === active) row.classList.add("active");
  row.addEventListener("click", (e) => openLevel(level, path, { newWindow: e.shiftKey }));
  row.addEventListener("dblclick", () => { if (branches) toggleBranch(path); });
  return row;
}

function toggleBranch(path) {
  if (expanded.has(path)) expanded.delete(path);
  else expanded.add(path);
  renderTree();
}

/**
 * The first path in the tree that reaches `level`, from the top level and else from the roots of the
 * subcircuits it does not call; or null.
 */
export function pathTo(level) {
  const find = (l, path, above) => {
    if (l === level) return path;
    if (above.includes(l)) return null;
    const kids = doc.levels[l].children;
    for (let k = 0; k < kids.length; k++) {
      if (kids[k].level === null) continue;
      const p = find(kids[k].level, `${path}/${k}`, above.concat(l));
      if (p) return p;
    }
    return null;
  };
  const top = find(0, "0", []);
  if (top) return top;
  for (const r of uncalledRoots()) {
    const p = find(r, rootPath(r), []);
    if (p) return p;
  }
  return null;
}

/** The subcircuits by name, under their initials; the top level always first. */
function renderIndex() {
  const list = $("index");
  const letters = $("letters");
  list.replaceChildren();
  letters.replaceChildren();
  setText($("index-count"), "");
  if (!doc || doc.levels.length === 0) return;
  const q = $("search").value.trim().toLowerCase();
  const uses = doc.levels.map(() => 0);
  for (const lv of doc.levels) for (const c of lv.children) if (c.level !== null) uses[c.level]++;
  append(list, indexRow(0, "", null));
  const all = doc.levels.map((l, k) => ({ title: l.title, k })).filter((x) => x.k !== 0);
  const shown = all
    .filter((x) => !q || x.title.toLowerCase().includes(q))
    .sort((a, b) => a.title.localeCompare(b.title, undefined, { sensitivity: "base", numeric: true }));
  setText($("index-count"), q ? m`${shown.length} of ${all.length}` : String(all.length));
  let group = null;
  for (const x of shown) {
    const initial = initialOf(x.title);
    if (initial !== group) {
      group = initial;
      const h = document.createElement("div");
      h.className = "index-group";
      h.id = `initial-${initial}`;
      setText(h, initial);
      append(list, h);
      const b = document.createElement("button");
      b.type = "button";
      setText(b, initial);
      setAttr(b, "title", m`Subcircuits starting with ${initial}`);
      b.addEventListener("click", () => scrollToGroup(list, h));
      append(letters, b);
    }
    append(list, indexRow(x.k, q, uses[x.k]));
  }
  if (all.length > 0 && shown.length === 0) {
    const p = document.createElement("div");
    p.className = "nothing";
    setText(p, m("No subcircuit matches."));
    append(list, p);
  }
}

/**
 * Scroll the index so that the group under the heading `h` starts at its top. The headings stick to
 * the top of the list, so a heading's own place may be where it sticks; the group's first row gives
 * where the group starts, and it shows below its heading.
 */
function scrollToGroup(list, h) {
  const row = h.nextElementSibling;
  const top = list.getBoundingClientRect().top + list.clientTop + h.offsetHeight;
  list.scrollTop += row.getBoundingClientRect().top - top;
}

function initialOf(name) {
  const c = name.charAt(0).toUpperCase();
  return /\p{L}/u.test(c) ? c : "#";
}

function indexRow(level, q, count) {
  const row = document.createElement("div");
  row.className = level === 0 ? "index-row top" : "index-row";
  row.setAttribute("role", "option");
  row.dataset.level = String(level);
  const name = document.createElement("span");
  name.className = "name";
  const title = doc.levels[level].title;
  const at = q ? title.toLowerCase().indexOf(q) : -1;
  if (at >= 0) {
    const m = document.createElement("mark");
    setText(m, title.slice(at, at + q.length));
    append(name, title.slice(0, at), m, title.slice(at + q.length));
  } else {
    setText(name, title);
  }
  append(row, name);
  if (level === 0) append(row, span("uses", m("top level")));
  else append(row, span("uses", count === 1 ? m("1 use") : m`${count} uses`));
  if (level === current) {
    row.classList.add("current");
    row.setAttribute("aria-selected", "true");
  }
  row.addEventListener("click", (e) => openLevel(level, null, { newWindow: e.shiftKey }));
  return row;
}

/**
 * Open a level from the hierarchy: from a tree row (`path`), by the way to it from the top level, or
 * from the index (null), as the root of the path (subcircuits.js). A pick of the path shown already
 * changes nothing, and the tree is not drawn again, so that the second click of a double click
 * presses the row the first did: Firefox sends no dblclick to a row that is out of the page.
 */
export function openLevel(level, path, options) {
  if (!doc) return;
  if (path !== null) enterRow(path, options);
  else enterRoot(level, options);
}

/**
 * Mark the level shown in the tree and the index, and bring it into view in both. The tree marks the
 * row of the path when the path starts at the top level, and else the first row that reaches it.
 */
export function updateHierarchy() {
  if (!doc) return;
  active = pathRow() || pathTo(current);
  if (active) {
    const parts = active.split("/");
    for (let k = 1; k < parts.length; k++) expanded.add(parts.slice(0, k).join("/"));
  }
  renderHierarchy();
  for (const id of ["tree", "index"]) {
    const box = $(id);
    const row = box.querySelector(id === "tree" ? ".tree-row.active" : ".index-row.current:not(.top)");
    if (!row) continue;
    const top = row.offsetTop - box.offsetTop;
    if (top < box.scrollTop || top + row.offsetHeight > box.scrollTop + box.clientHeight) {
      box.scrollTop = Math.max(0, top - box.clientHeight / 2);
    }
  }
}

$("search").addEventListener("input", renderIndex);
$("search").addEventListener("keydown", (e) => {
  if (e.key === "Escape" && $("search").value) {
    $("search").value = "";
    renderIndex();
    e.stopPropagation();
  } else if (e.key === "Enter") {
    const first = $("index").querySelector(".index-row:not(.top)");
    if (first) openLevel(Number(first.dataset.level), null, { newWindow: e.shiftKey });
  }
});
