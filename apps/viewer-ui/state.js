// What the page shows: the document, the level drawn, the selection, and the hierarchy path to the
// level. The modules read these bindings directly; an imported binding cannot be assigned, so the
// modules that change them call the setters.

export let doc = null; // the Document the shell built
export let current = 0; // the level drawn
export let selected = null; // the selected object, an index into doc.objects

export let expanded = new Set(["0"]); // the open branches of the hierarchy tree, by path
// The hierarchy path of the level drawn (subcircuits.js), from its root down to it: per level of it,
// { level, via }, its index and the name of the instance it was reached through, null for the root.
export let path = [];
export let viewCache = new Map(); // a path, by the names on it -> the view its level had when it was left

export function setDoc(d) {
  doc = d;
}

export function setCurrent(i) {
  current = i;
}

export function setSelected(o) {
  selected = o;
}

export function setExpanded(paths) {
  expanded = paths;
}

export function setPath(p) {
  path = p;
}

export function setViewCache(views) {
  viewCache = views;
}
