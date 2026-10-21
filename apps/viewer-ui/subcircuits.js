import { append, m, setAttr, setText } from "./i18n.js";
// The hierarchy path of the level shown, from its root down to it, and the bar above the drawing that
// shows it. Going down into a subcircuit instance adds the instance to the path; its segments in the
// bar, the up button and Backspace go back along it, each level with the view it had there and the
// instance it was left through selected. Picking a subcircuit in the index makes it the root, picking
// an instance in the tree gives the way to it from the top level, and reaching a level otherwise cuts
// the path back to it, or starts it there. The views kept are by path, so a subcircuit reached two
// ways keeps a view for each. Entering a subcircuit brings the start of its definition to the top of
// the text; going back up shows the statement of the instance it was left through.

import { current, doc, path, setPath, viewCache } from "./state.js";
import { showLevel, view } from "./drawing.js";
import { rootPath } from "./hierarchy.js";
import { uncalledRoots } from "./roots.js";
import { clearSelection, select } from "./selection.js";
import { revealDefinition } from "./text.js";
import { navigate } from "./navigation.js";
import { $, span } from "./util.js";

const bar = $("level-path");
const menu = $("path-menu");

let crumbs = []; // per level of the path, its segment and the separator before it, if any
let more = null; // the ellipsis of the levels left out, and the separator before it
let hidden = []; // the levels of the path left out of the bar, behind the ellipsis

/** The level a subcircuit instance opens, or null. */
export function opens(o) {
  const obj = doc && o !== null && o !== undefined ? doc.objects[o] : null;
  return obj && obj.kind === "instance" && obj.opens !== null && obj.opens !== undefined ? obj.opens : null;
}

/** A path's key among the views kept: its root's title and the instances on it. */
function key(p) {
  return JSON.stringify(p.map((s, k) => (k === 0 ? doc.levels[s.level].title : s.via)));
}

const same = (a, b) => a.length === b.length && a.every((s, k) => s.level === b[k].level && s.via === b[k].via);

/** Keep the view of the level shown, for the path to it. */
function rememberView() {
  if (view && path.length > 0) viewCache.set(key(path), view);
}

/**
 * Show the level a path leads to, with the view kept for the path, or else the view shown when the
 * level stays; with `via`, select the instance of that name on it, the one the path was left through.
 */
export function commitNavigation({ path: next, via, selection, reveal = false }) {
  rememberView();
  const level = next[next.length - 1].level;
  const kept = viewCache.get(key(next)) || (level === current ? view : null);
  setPath(next);
  showLevel(level, kept);
  if (selection === null) clearSelection();
  else if (selection !== undefined) select(selection, "reload");
  if (reveal) revealDefinition(level);
  if (via === undefined) return;
  // the instance we came out of stays selected, so the way back down is one Enter away
  const host = doc.levels[level].host;
  const o = doc.objects.findIndex((x) => x.kind === "instance" && x.level === host && x.name === via);
  if (o >= 0) select(o, "ascend");
}

/** The path cut back to a level on it, or else a path that starts at the level. */
function cutTo(level) {
  const k = path.map((s) => s.level).lastIndexOf(level);
  return k >= 0 ? path.slice(0, k + 1) : [{ level, via: null }];
}

/** Open the subcircuit of instance `o`, below the level that has the instance. */
export function descend(o, options) {
  const level = opens(o);
  if (level === null) return;
  const obj = doc.objects[o];
  // the instance is on the level shown, or on one that the path is cut back to or starts at
  const at = doc.levels[current].host === obj.level ? current : doc.levels.findIndex((l) => l.host === obj.level);
  return navigate({ path: at < 0 ? [{ level, via: null }] : [...cutTo(at), { level, via: obj.name }], selection: null, reveal: true }, options);
}

/** Go back along the path to its level at `depth`. */
export function backTo(depth, options) {
  if (depth < 0 || depth >= path.length - 1) return;
  return navigate({ path: path.slice(0, depth + 1), via: path[depth + 1].via }, options);
}

/** Go one level up the path; at its root, nothing. */
export function ascend(options) {
  return backTo(path.length - 2, options);
}

/** Show a level reached otherwise than through an instance, such as by an object of it in the text. */
export function goTo(level, selection, options) {
  return navigate({ path: cutTo(level), selection }, options);
}

/**
 * Show the level of a row of the hierarchy tree, by the way to it from the root of its tree: the top
 * level, "0/1", or a subcircuit that the top level does not call, "r3/0" (hierarchy.js).
 */
export function enterRow(row, options = {}) {
  const [start, ...rest] = row.split("/");
  const next = [{ level: start.startsWith("r") ? Number(start.slice(1)) : 0, via: null }];
  for (const k of rest.map(Number)) {
    const child = doc.levels[next[next.length - 1].level].children[k];
    if (!child || child.level === null) return;
    next.push({ level: child.level, via: child.instance });
  }
  // the path shown already changes nothing, so that the tree is not drawn again (openLevel)
  if (same(next, path) && !options.newWindow) return;
  return navigate({ path: next, reveal: true }, options);
}

/** Show a level as the root of the path, as the subcircuit index does. */
export function enterRoot(level, options = {}) {
  if (same([{ level, via: null }], path) && !options.newWindow) return;
  return navigate({ path: [{ level, via: null }], reveal: true }, options);
}

/**
 * The tree row of the path shown when it starts at the top level, as "0/1/0", or at a subcircuit
 * that the top level does not call, as "r3/0"; otherwise null.
 */
export function pathRow() {
  if (path.length === 0) return null;
  const first = path[0].level;
  if (first !== 0 && !uncalledRoots().includes(first)) return null;
  const row = [first === 0 ? "0" : rootPath(first)];
  for (let d = 1; d < path.length; d++) {
    const k = doc.levels[path[d - 1].level].children.findIndex((c) => c.instance === path[d].via);
    if (k < 0) return null;
    row.push(String(k));
  }
  return row.join("/");
}

/** The path by the names on it, to find it again in the document read again: its levels' titles and instances. */
export function namedPath() {
  return path.map((s) => ({ title: doc.levels[s.level].title, via: s.via }));
}

/**
 * Find a path by its names (namedPath) in the document shown now, which leads to `level`: when its
 * levels and instances are all there, it is the path; otherwise the path starts at `level`.
 */
export function restorePath(named, level) {
  const next = [];
  for (const s of named) {
    let l = -1;
    if (next.length === 0) {
      l = doc.levels.findIndex((x) => x.title === s.title);
    } else {
      const child = doc.levels[next[next.length - 1].level].children.find((c) => c.instance === s.via);
      if (child && child.level !== null) l = child.level;
    }
    if (l < 0 || doc.levels[l].title !== s.title) break;
    next.push({ level: l, via: s.via });
  }
  const found = next.length > 0 && next.length === named.length && next[next.length - 1].level === level;
  setPath(found ? next : [{ level, via: null }]);
}

// ------------------------------------------------------------------ the bar above the drawing

/** Show the path in the bar: a segment per level, all but the last going back to theirs; and the up button. */
export function updatePath() {
  closeMore();
  const name = $("level-name");
  crumbs = path.map((s, d) => {
    const title = doc.levels[s.level].title;
    let crumb;
    if (d === path.length - 1) {
      crumb = document.createElement("span");
      crumb.className = "crumb current";
      crumb.setAttribute("aria-current", "location");
      setText(name, title);
      append(crumb, name);
    } else {
      crumb = document.createElement("button");
      crumb.type = "button";
      crumb.className = "crumb";
      setAttr(crumb, "title", m`Back to ${title}`);
      append(crumb, span("name", title));
      crumb.addEventListener("click", (e) => backTo(d, { newWindow: e.shiftKey }));
    }
    if (s.via !== null) {
      append(crumb, span("via", s.via));
      setAttr(crumb, "title", d < path.length - 1 ? m`${m`Back to ${title}`} (${s.via})` : m`${title}, through ${s.via}`);
    }
    // the name and the instance read apart, as they are seen apart
    if (d < path.length - 1) setAttr(crumb, "aria-label", s.via !== null ? `${title} ${s.via}` : title);
    crumb.dataset.depth = String(d);
    return { crumb, sep: d > 0 ? separator() : null };
  });
  const ellipsis = document.createElement("button");
  ellipsis.type = "button";
  ellipsis.className = "crumb-more";
  setText(ellipsis, "…");
  setAttr(ellipsis, "title", m("The levels in between"));
  setAttr(ellipsis, "aria-label", m("The levels in between"));
  ellipsis.setAttribute("aria-haspopup", "menu");
  ellipsis.setAttribute("aria-expanded", "false");
  ellipsis.addEventListener("click", (e) => {
    if (menu.hidden) openMore(e.detail === 0);
    else closeMore();
  });
  more = { crumb: ellipsis, sep: separator() };
  if (path.length === 0) {
    setText(name, "");
    bar.replaceChildren(name);
  } else {
    bar.replaceChildren(
      crumbs[0].crumb,
      more.sep,
      more.crumb,
      ...crumbs.slice(1).flatMap((c) => [c.sep, c.crumb]),
    );
  }
  const up = path.length > 1 ? doc.levels[path[path.length - 2].level].title : null;
  // at the root the up button hides: the focus goes from it to the drawing first, so that the keys
  // act on the drawing at once. A browser that moves the focus off a hidden element only later,
  // as WebKit does, would otherwise give the next Enter to the hidden button.
  if (up === null && document.activeElement === $("up")) $("canvas").focus({ preventScroll: true });
  $("up").hidden = up === null;
  if (up !== null) {
    setText($("up-name"), up);
    setAttr($("up"), "title", m`Up to ${up} (Ctrl+E or Backspace; Shift opens a new window)`);
  }
  fitPath();
}

function separator() {
  const el = span("crumb-sep", "›");
  el.setAttribute("aria-hidden", "true");
  return el;
}

/** Leave the levels at `depths` out of the bar, behind the ellipsis. */
function leaveOut(depths) {
  hidden = depths.slice().sort((a, b) => a - b);
  crumbs.forEach((c, d) => {
    c.crumb.hidden = hidden.includes(d);
    if (c.sep) c.sep.hidden = c.crumb.hidden;
  });
  if (!more) return;
  more.crumb.hidden = hidden.length === 0;
  // the root's separator goes with the root
  more.sep.hidden = more.crumb.hidden || hidden.includes(0);
}

/**
 * Fit the path into the bar. As the bar narrows, the level's kind gives way first; then the levels
 * after the root, from the root's side, behind an ellipsis that lists them; then the root. The level
 * shown always stays, and its name is cut short only when nothing else is left to give way.
 */
function fitPath() {
  const kind = $("level-kind");
  bar.classList.add("measuring");
  kind.hidden = false;
  leaveOut([]);
  const over = () => bar.scrollWidth > bar.clientWidth + 0.5;
  if (over()) kind.hidden = true;
  const order = crumbs.map((_, d) => d).slice(1, -1);
  if (crumbs.length > 1) order.push(0);
  for (let n = 1; n <= order.length && over(); n++) leaveOut(order.slice(0, n));
  bar.classList.remove("measuring");
}

/** List the levels left out under the ellipsis; from the keyboard, with the focus on the first. */
function openMore(fromKeys) {
  menu.replaceChildren(
    ...hidden.map((d) => {
      const s = path[d];
      const b = document.createElement("button");
      b.type = "button";
      b.className = "path-menu-item";
      b.setAttribute("role", "menuitem");
      b.dataset.depth = String(d);
      const title = doc.levels[s.level].title;
      append(b, span("name", title));
      if (s.via !== null) append(b, span("via", s.via));
      setAttr(b, "aria-label", s.via !== null ? `${title} ${s.via}` : title);
      b.addEventListener("click", (e) => {
        closeMore();
        backTo(d, { newWindow: e.shiftKey });
      });
      return b;
    }),
  );
  // under the ellipsis, in the bar above the drawing
  const [at, within] = [more.crumb.getBoundingClientRect(), bar.parentElement.getBoundingClientRect()];
  menu.style.left = `${at.left - within.left}px`;
  menu.hidden = false;
  more.crumb.setAttribute("aria-expanded", "true");
  if (fromKeys && menu.firstElementChild) menu.firstElementChild.focus();
}

function closeMore() {
  if (menu.hidden) return;
  menu.hidden = true;
  if (more) more.crumb.setAttribute("aria-expanded", "false");
}

// the arrows move in the list, Escape closes it; a press elsewhere closes it too
menu.addEventListener("keydown", (e) => {
  const items = [...menu.children];
  const k = items.indexOf(document.activeElement);
  if (e.key === "ArrowDown" || e.key === "ArrowUp") {
    const step = e.key === "ArrowDown" ? 1 : items.length - 1;
    items[k < 0 ? 0 : (k + step) % items.length].focus();
  } else if (e.key === "Escape") {
    closeMore();
    more.crumb.focus();
  } else if (e.key === "Tab") {
    closeMore();
    return;
  } else {
    return;
  }
  e.preventDefault();
  e.stopPropagation();
});
document.addEventListener(
  "pointerdown",
  (e) => {
    if (!menu.hidden && !menu.contains(e.target) && !(more && more.crumb.contains(e.target))) closeMore();
  },
  true,
);
// the bar narrows or widens with the window and the side panel
new ResizeObserver(() => {
  closeMore();
  fitPath();
}).observe(bar.parentElement);

$("up").addEventListener("click", (e) => ascend({ newWindow: e.shiftKey }));
