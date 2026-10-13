// The hierarchy view: the subcircuit instances as a tree under the top level, and every
// subcircuit by name under its initial, with a search box. Picking a row opens its level.

import { current, doc, expanded, setTreeActive, treeActive, viewCache } from "./state.js";
import { showLevel } from "./drawing.js";
import { rememberView } from "./subcircuits.js";
import { $, span } from "./util.js";

export function renderHierarchy() {
  renderTree();
  renderIndex();
}

/** The tree of subcircuit instances under the top level; only open branches are listed. */
function renderTree() {
  const tree = $("tree");
  tree.replaceChildren();
  if (!doc || doc.levels.length === 0) return;
  const walk = (level, path, depth, child, above) => {
    const lv = doc.levels[level];
    const cycle = above.includes(level);
    const branches = cycle ? [] : lv.children;
    const open = branches.length > 0 && expanded.has(path);
    tree.append(treeRow({ level, path, depth, child, branches: branches.length, open, cycle }));
    if (!open) return;
    branches.forEach((c, k) => {
      const p = `${path}/${k}`;
      if (c.level === null) tree.append(treeRow({ level: null, path: p, depth: depth + 1, child: c, branches: 0 }));
      else walk(c.level, p, depth + 1, c, above.concat(level));
    });
  };
  walk(0, "0", 0, null, []);
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
    twisty.setAttribute("aria-label", open ? "Collapse" : "Expand");
    twisty.addEventListener("click", (e) => {
      e.stopPropagation();
      toggleBranch(path);
    });
  }
  row.append(twisty);
  if (child === null) {
    row.append(span("inst", doc.levels[0].title), span("badge", "top"));
  } else {
    row.append(span("inst", child.instance), span("master", child.master));
    if (level === null) row.append(span("badge", "not defined"));
    if (cycle) row.append(span("badge", "recursive"));
  }
  if (level === null) {
    row.classList.add("undefined");
    row.title = `${child.master} is not defined in the netlist`;
    return row;
  }
  row.dataset.level = String(level);
  if (level === current) row.classList.add("current");
  if (path === treeActive) row.classList.add("active");
  row.addEventListener("click", () => openLevel(level, path));
  row.addEventListener("dblclick", () => { if (branches) toggleBranch(path); });
  return row;
}

function toggleBranch(path) {
  if (expanded.has(path)) expanded.delete(path);
  else expanded.add(path);
  renderTree();
}

/** The first path in the tree that reaches `level`, opening the branches above it; or null. */
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
  return find(0, "0", []);
}

/** The subcircuits by name, under their initials; the top level always first. */
function renderIndex() {
  const list = $("index");
  const letters = $("letters");
  list.replaceChildren();
  letters.replaceChildren();
  $("index-count").textContent = "";
  if (!doc || doc.levels.length === 0) return;
  const q = $("search").value.trim().toLowerCase();
  const uses = doc.levels.map(() => 0);
  for (const lv of doc.levels) for (const c of lv.children) if (c.level !== null) uses[c.level]++;
  list.append(indexRow(0, "", null));
  const all = doc.levels.map((l, k) => ({ title: l.title, k })).filter((x) => x.k !== 0);
  const shown = all
    .filter((x) => !q || x.title.toLowerCase().includes(q))
    .sort((a, b) => a.title.localeCompare(b.title, undefined, { sensitivity: "base", numeric: true }));
  $("index-count").textContent = q ? `${shown.length} of ${all.length}` : String(all.length);
  let group = null;
  for (const x of shown) {
    const initial = initialOf(x.title);
    if (initial !== group) {
      group = initial;
      const h = document.createElement("div");
      h.className = "index-group";
      h.id = `initial-${initial}`;
      h.textContent = initial;
      list.append(h);
      const b = document.createElement("button");
      b.type = "button";
      b.textContent = initial;
      b.title = `Subcircuits starting with ${initial}`;
      b.addEventListener("click", () => {
        list.scrollTop = h.offsetTop - list.querySelector(".index-row.top").offsetHeight;
      });
      letters.append(b);
    }
    list.append(indexRow(x.k, q, uses[x.k]));
  }
  if (all.length > 0 && shown.length === 0) {
    const p = document.createElement("div");
    p.className = "nothing";
    p.textContent = "No subcircuit matches.";
    list.append(p);
  }
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
    m.textContent = title.slice(at, at + q.length);
    name.append(title.slice(0, at), m, title.slice(at + q.length));
  } else {
    name.textContent = title;
  }
  row.append(name);
  if (level === 0) row.append(span("uses", "top level"));
  else row.append(span("uses", count === 1 ? "1 use" : `${count} uses`));
  if (level === current) {
    row.classList.add("current");
    row.setAttribute("aria-selected", "true");
  }
  row.addEventListener("click", () => openLevel(level, null));
  return row;
}

/** Open a level from the hierarchy: from a tree row (`path`) or from the index (null). */
export function openLevel(level, path) {
  if (!doc) return;
  if (level !== current) {
    rememberView();
    const p = path !== null ? path : pathTo(level);
    showLevel(level, p !== null ? viewCache.get(p) || null : null, path);
  } else {
    setTreeActive(path);
    updateHierarchy();
  }
}

/** Mark the level shown in the tree and the index, and bring it into view in both. */
export function updateHierarchy() {
  if (!doc) return;
  if (treeActive !== null) {
    const parts = treeActive.split("/").map(Number);
    let l = 0;
    for (const k of parts.slice(1)) l = doc.levels[l] && doc.levels[l].children[k] ? doc.levels[l].children[k].level : null;
    if (l !== current) setTreeActive(null);
  }
  if (treeActive === null) setTreeActive(pathTo(current));
  if (treeActive) {
    const parts = treeActive.split("/");
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
    if (first) openLevel(Number(first.dataset.level), null);
  }
});
