import { append, m, setAttr, setText } from "./i18n.js";
// The hierarchy view: the subcircuit instances as a tree under the top level, below it the
// subcircuits that the top level does not call as trees of their own, and every subcircuit by name
// under its initial, with a search box. Picking a row opens its level: a row of the tree by the way
// to it from its root, a subcircuit of the index as the root of the path.
//
// The paths of the rows: "0" is the top level and "0/1/0" the first instance of the second instance
// of the top level; "r3/0" is the first instance of the subcircuit 3 that the top level does not
// call (roots.js). Levels are named by their host levels (level-tree.js): the tree lists the levels
// of every file, also of those a shell that reads the file shown alone has not read, and the modules
// of Verilog-A of its outline, marked so, which open as their ports and instances (module-view.js).
//
// A level with more than GROUP_AT instances of one subcircuit lists them under one row, with their number, such as the
// thousands of decoupling cells of a top level: open, the row lists them BATCH at a time, and BATCH more when the line
// below them comes into view or is clicked. Their paths are their own all the same ("0/1234"), and the row of the level
// shown is listed, with its group open. The tree is drawn only while the hierarchy is shown: hidden, it is emptied, and
// drawn again when it is shown.

import { current, doc, expanded } from "./state.js";
import { uncalledRoots } from "./roots.js";
import { levelTree, shownHost } from "./level-tree.js";
import { enterHost, enterRow, pathRow } from "./subcircuits.js";
import { $, span } from "./util.js";

let active = null; // the tree row of the level shown: its path's, or else the first that reaches it

const GROUP_AT = 50; // more instances of one subcircuit under a level than this are listed under one row
const BATCH = 200; // the instances such a row lists at a time
const listed = new Map(); // how many instances each open group lists, by the group's path, for the document `listedOf`
let listedOf = null;
const groupsOf = new WeakMap(); // the groups of a level's instances, by the level's node (`groups`)
const pane = $("pane-hierarchy");

/** Whether the hierarchy is shown; hidden, it is not drawn, and what was drawn is emptied. */
function shown() {
  if (!pane.hidden) return true;
  for (const id of ["tree", "index", "letters"]) if ($(id).firstChild) $(id).replaceChildren();
  return false;
}

export function renderHierarchy() {
  if (!shown()) return;
  renderTree();
  renderIndex();
}

/** Forget how many instances the groups list when the document is another. */
function sameDocument() {
  if (listedOf !== doc) { listed.clear(); listedOf = doc; }
}

/**
 * The groups of the instances of the level `lv`: for each subcircuit, defined or not, that more than GROUP_AT of them
 * call, the indices of those instances among `lv.children`, by the index of the first; `of[k]` is the group of child k.
 */
function groups(lv) {
  let found = groupsOf.get(lv);
  if (found) return found;
  const byName = new Map();
  lv.children.forEach((c, k) => {
    const name = `${c.host}|${c.master}`;
    if (!byName.has(name)) byName.set(name, []);
    byName.get(name).push(k);
  });
  found = { starts: new Map(), of: [] };
  for (const ks of byName.values()) {
    if (ks.length <= GROUP_AT) continue;
    found.starts.set(ks[0], ks);
    for (const k of ks) found.of[k] = ks;
  }
  groupsOf.set(lv, found);
  return found;
}

/** The path of the row of the group of instances `ks` under the row `path`. */
const groupPath = (path, ks) => `${path}/g${ks[0]}`;

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
  more.disconnect();
  tree.replaceChildren();
  const levels = levelTree();
  // (the reading of a file of Verilog-A has no levels, and the outline has the levels of the others)
  if (!doc || !levels.nodes[levels.top]) return;
  sameDocument();
  // the rows of the level `level` and of its open branches, into `out`
  const walk = (level, path, depth, child, above, out) => {
    const lv = levels.nodes[level];
    const cycle = above.includes(level);
    const branches = cycle ? [] : lv.children;
    const open = branches.length > 0 && expanded.has(path);
    append(out, treeRow({ level, path, depth, child, branches: branches.length, open, cycle }));
    if (!open) return;
    const one = (k, into) => {
      const c = branches[k];
      const p = `${path}/${k}`;
      if (c.host === null || !levels.nodes[c.host]) append(into, treeRow({ level: null, path: p, depth: depth + 1, child: c, branches: 0 }));
      else walk(c.host, p, depth + 1, c, above.concat(level), into);
    };
    const { starts, of } = groups(lv);
    for (let k = 0; k < branches.length; k++) {
      const ks = starts.get(k);
      if (ks) {
        const g = groupPath(path, ks);
        const opened = expanded.has(g);
        const c = branches[k];
        append(out, groupRow(g, depth + 1, c, ks.length, opened, c.host === null ? undefined : levels.nodes[c.host]));
        if (!opened) continue;
        // the instances listed so far, and the line below them, which gives the rows of BATCH more in its place
        const list = (from, to, into) => {
          for (let i = from; i < to; i++) one(ks[i], into);
          if (to < ks.length) {
            append(into, moreRow(depth + 2, to, ks.length, () => {
              const next = Math.min(ks.length, to + BATCH);
              listed.set(g, next);
              const rows = document.createDocumentFragment();
              list(to, next, rows);
              return rows;
            }));
          }
        };
        list(0, Math.min(ks.length, listed.get(g) ?? BATCH), out);
      } else if (!of[k]) {
        one(k, out);
      }
    }
  };
  walk(levels.top, "0", 0, null, [], tree);
  const roots = uncalledRoots();
  if (roots.length === 0) return;
  const heading = document.createElement("div");
  heading.className = "tree-group";
  setText(heading, m("Not called from the top level"));
  append(tree, heading);
  for (const r of roots) walk(r, rootPath(r), 0, ROOT, [], tree);
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
  const { nodes } = levelTree();
  if (child === null) {
    append(row, span("inst", nodes[level].title), span("badge", m("top")));
  } else if (child === ROOT) {
    append(row, span("inst", nodes[level].title), span("badge", m("not called")));
    setAttr(row, "title", m`No instance calls ${nodes[level].title}`);
  } else {
    // a name the netlist does not define, from the outline: one row for all its instances
    append(row, span("inst", child.instance ?? `${child.count}×`), span("master", child.master));
    if (level === null) append(row, span("badge", m("not defined")));
    if (cycle) append(row, span("badge", m("recursive")));
  }
  // the name of the language is no word to translate
  if (level !== null && nodes[level].module) append(row, span("badge", "Verilog-A"));
  if (level === null) {
    row.classList.add("undefined");
    setAttr(row, "title", m`${child.master} is not defined in the netlist`);
    return row;
  }
  row.dataset.level = String(level);
  if (level === shownHost(current)) row.classList.add("current");
  if (path === active) row.classList.add("active");
  row.addEventListener("click", (e) => openLevel(level, path, { newWindow: e.shiftKey }));
  row.addEventListener("dblclick", () => { if (branches) toggleBranch(path); });
  return row;
}

/**
 * The row of a group of `count` instances of the subcircuit of `child`, at `path` (`groupPath`): their number and the
 * subcircuit's name; a click opens or closes it. `node` is the subcircuit's level, undefined when the netlist does not
 * define it.
 */
function groupRow(path, depth, child, count, open, node) {
  const row = document.createElement("div");
  row.className = node ? "tree-row many" : "tree-row many undefined";
  row.setAttribute("role", "treeitem");
  row.setAttribute("aria-expanded", String(open));
  row.dataset.path = path;
  row.style.paddingLeft = `${4 + depth * 16}px`;
  const twisty = document.createElement("button");
  twisty.type = "button";
  twisty.className = "twisty";
  twisty.tabIndex = -1;
  twisty.setAttribute("aria-expanded", String(open));
  setAttr(twisty, "aria-label", open ? m("Collapse") : m("Expand"));
  append(row, twisty, span("inst", `${count}×`), span("master", child.master));
  if (!node) append(row, span("badge", m("not defined")));
  else if (node.module) append(row, span("badge", "Verilog-A"));
  row.addEventListener("click", () => toggleBranch(path));
  return row;
}

// the line below the instances a group lists so far: in view, or clicked, it gives way to the rows of BATCH more
const rest = new WeakMap(); // the rows each line gives way to, by the line
const more = typeof IntersectionObserver === "undefined"
  ? { observe() {}, unobserve() {}, disconnect() {} }
  : new IntersectionObserver((entries) => {
    for (const e of entries) if (e.isIntersecting) listMore(e.target);
  }, { root: $("tree") });

function moreRow(depth, n, count, next) {
  const row = document.createElement("div");
  row.className = "tree-more";
  row.style.paddingLeft = `${4 + depth * 16}px`;
  const b = document.createElement("button");
  b.type = "button";
  setText(b, m`${n} of ${count} shown: scroll for more`);
  b.addEventListener("click", () => listMore(row));
  append(row, b);
  rest.set(row, next);
  more.observe(row);
  return row;
}

/** Put the rows of the instances that the line `row` stands for in its place. */
function listMore(row) {
  const next = rest.get(row);
  if (!next || !row.isConnected) return;
  rest.delete(row);
  more.unobserve(row);
  row.replaceWith(next());
}

function toggleBranch(path) {
  if (expanded.has(path)) expanded.delete(path);
  else expanded.add(path);
  renderTree();
}

/**
 * Open the groups that the path of tree rows `route` goes through, each listing as many instances as list the one the
 * path goes through.
 */
function revealGroups(route) {
  const { nodes, top } = levelTree();
  const parts = route.split("/");
  let level = parts[0] === "0" ? top : Number(parts[0].slice(1));
  let path = parts[0];
  for (const part of parts.slice(1)) {
    const lv = nodes[level];
    const k = Number(part);
    const ks = lv && groups(lv).of[k];
    if (ks) {
      const g = groupPath(path, ks);
      expanded.add(g);
      listed.set(g, Math.max(listed.get(g) ?? BATCH, Math.ceil((ks.indexOf(k) + 1) / BATCH) * BATCH));
    }
    level = lv?.children[k]?.host ?? null;
    if (level === null) return;
    path = `${path}/${k}`;
  }
}

/**
 * The first path in the tree that reaches the host level `level`, from the top level and else from the roots of the
 * subcircuits it does not call; or null.
 */
export function pathTo(level) {
  const { nodes, top: start } = levelTree();
  const find = (l, path, above) => {
    if (l === level) return path;
    if (above.includes(l) || !nodes[l]) return null;
    const kids = nodes[l].children;
    for (let k = 0; k < kids.length; k++) {
      if (kids[k].host === null) continue;
      const p = find(kids[k].host, `${path}/${k}`, above.concat(l));
      if (p) return p;
    }
    return null;
  };
  const top = find(start, "0", []);
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
  const { nodes, top } = levelTree() ?? {};
  if (!doc || !nodes?.[top]) return;
  const q = $("search").value.trim().toLowerCase();
  const uses = [];
  for (const lv of nodes) for (const c of lv?.children ?? []) if (c.host !== null) uses[c.host] = (uses[c.host] ?? 0) + 1;
  append(list, indexRow(top, "", null));
  const all = nodes.flatMap((l, k) => (l && k !== top ? [{ title: l.title, k }] : []));
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
    append(list, indexRow(x.k, q, uses[x.k] ?? 0));
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
  const { nodes, top } = levelTree();
  const row = document.createElement("div");
  row.className = level === top ? "index-row top" : "index-row";
  row.setAttribute("role", "option");
  row.dataset.level = String(level);
  const name = document.createElement("span");
  name.className = "name";
  const title = nodes[level].title;
  const at = q ? title.toLowerCase().indexOf(q) : -1;
  if (at >= 0) {
    const m = document.createElement("mark");
    setText(m, title.slice(at, at + q.length));
    append(name, title.slice(0, at), m, title.slice(at + q.length));
  } else {
    setText(name, title);
  }
  append(row, name);
  if (nodes[level].module) append(row, span("badge", "Verilog-A"));
  if (level === top) append(row, span("uses", m("top level")));
  else append(row, span("uses", count === 1 ? m("1 use") : m`${count} uses`));
  if (level === shownHost(current)) {
    row.classList.add("current");
    row.setAttribute("aria-selected", "true");
  }
  row.addEventListener("click", (e) => openLevel(level, null, { newWindow: e.shiftKey }));
  return row;
}

/**
 * Open a level from the hierarchy, by its host level: from a tree row (`path`), by the way to it from the top level, or
 * from the index (null), as the root of the path (subcircuits.js). A pick of the path shown already
 * changes nothing, and the tree is not drawn again, so that the second click of a double click
 * presses the row the first did: Firefox sends no dblclick to a row that is out of the page.
 */
export function openLevel(level, path, options) {
  if (!doc) return;
  if (path !== null) enterRow(path, options);
  else enterHost(level, options);
}

/**
 * Mark the level shown in the tree and the index, and bring it into view in both. The tree marks the
 * row of the path when the path starts at the top level, and else the first row that reaches it.
 */
export function updateHierarchy() {
  if (!doc) return;
  active = pathRow() || pathTo(shownHost(current));
  if (active) {
    const parts = active.split("/");
    for (let k = 1; k < parts.length; k++) expanded.add(parts.slice(0, k).join("/"));
    sameDocument();
    revealGroups(active);
  }
  if (!shown()) return;
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
