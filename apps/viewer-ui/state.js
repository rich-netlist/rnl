// What the page shows: the document, the level drawn, the selection, and the way through the
// hierarchy that led to the level. The modules read these bindings directly; an imported binding
// cannot be assigned, so the modules that change them call the setters.

export let doc = null; // the Document the shell built
export let current = 0; // the level drawn
export let selected = null; // the selected object, an index into doc.objects

export let expanded = new Set(["0"]); // the open branches of the hierarchy tree, by path
export let treeActive = null; // the path of the tree row that opened the level shown, if one did
export let viewCache = new Map(); // tree path -> the view a level had when it was left

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

export function setTreeActive(path) {
  treeActive = path;
}

export function setViewCache(views) {
  viewCache = views;
}
