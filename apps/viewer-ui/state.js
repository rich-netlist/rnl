// What the page shows: the document, the level drawn, the selection, and the hierarchy path to the
// level. The modules read these bindings directly; an imported binding cannot be assigned, so the
// modules that change them call the setters.

export let doc = null; // the Document the shell built
export let current = 0; // the level drawn
export let selected = null; // the selected object, an index into doc.objects
export let selectedAsUsage = false; // a usage inspection, rather than ordinary virtual-symbol selection

export let expanded = new Set(["0"]); // the open branches of the hierarchy tree, by path
// Visit history (subcircuits.js): { entry, title, level, via }. The level index is valid only
// in this entry's Document; visits in other files have level -1. History never supplies includes.
export let path = [];
export let viewCache = new Map(); // a path, by the names on it -> the view its level had when it was left
export const bindings = new WeakMap(); // a Document built with the parameters of a subcircuit bound -> what the page knows of that (params.js)

export function setDoc(d) {
  doc = d;
}

export function setCurrent(i) {
  current = i;
}

// whether builds ask the core for the constraints of the levels (`Level.constraints`): once the Solve view has been
// opened, as they cost a second reading of a file that has records of the solver (solve.js)
export let constraintsWanted = false;

export function setConstraintsWanted(wanted) {
  constraintsWanted = wanted;
}

export function setSelected(o, asUsage = false) {
  selected = o;
  selectedAsUsage = o !== null && asUsage;
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
