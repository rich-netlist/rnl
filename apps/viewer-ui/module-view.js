import { append, m, setAttr, setText } from "./i18n.js";
// A module of Verilog-A, shown in place of a drawing. The outline of a shell that reads the file shown alone has the
// modules of the document's Verilog-A files, with their ports and the modules their instances name (level-tree.js),
// but no reading draws one (2.9): the reading of a Verilog-A file has no levels. A module entered, from the hierarchy
// or through an instance that names it, opens the view of its file as a subcircuit of another file does
// (subcircuits.js), and is shown there as a symbol of its ports, with a note, and its instances, each with the line it
// is written on: a click shows the line in the text, a double click or Enter opens the module it names. A name that no
// module has is listed as not defined. The parameters an instance gives a module are not shown. In the preview of a solve,
// which keeps the outline of the reading it started from (level-tree.js `previewModules`), a module is shown in place,
// below the levels of the preview, which stays: the way back leads to it.

import { doc, path, setPath } from "./state.js";
import { clearDrawing } from "./drawing.js";
import { updateHierarchy } from "./hierarchy.js";
import { levelTree, previewModules, setModuleShown } from "./level-tree.js";
import { navigate } from "./navigation.js";
import { currentSource } from "./open.js";
import { updatePath, viewOfFile } from "./subcircuits.js";
import { revealLine } from "./text.js";
import { sameSource } from "./visit-source.js";
import { $, span, svgElement } from "./util.js";

const card = $("module-card");

/**
 * The host level of the module of the file the Document shown reads that `title` names, or of its first module when
 * `title` names none of them; -1 when the Document is no reading of a file of modules.
 */
export function moduleOfFile(title) {
  const tree = levelTree();
  // (the preview of a solve: the module of that title, of whichever file)
  if (previewModules()) return tree.nodes.findIndex((n) => n?.module && n.title === title);
  if (!doc?.outline || !tree || typeof doc.file !== "string") return -1;
  const ours = tree.nodes.flatMap((n, k) => (n?.module && n.file === doc.file ? [k] : []));
  return ours.find((k) => tree.nodes[k].title === title) ?? ours[0] ?? -1;
}

/**
 * Show the module of the Document shown that the last step of the path `route` (by the titles of its levels) names, or
 * the first module of its file, with `route` as the path to it. False when the Document has no module to show.
 */
export function showModule(route) {
  const host = moduleOfFile(route.at(-1)?.title);
  if (host < 0) return false;
  const node = levelTree().nodes[host];
  clearDrawing("");
  $("empty").hidden = true;
  setModuleShown(host);
  const steps = route.length ? route : [{ title: node.title, via: null }];
  const last = steps.length - 1;
  // (in a preview, the steps above the module are levels of the preview)
  const preview = previewModules();
  setPath(steps.map((s, k) => ({ ...s, title: k === last ? node.title : s.title, entry: s.entry || currentSource(),
    level: preview && k < last ? s.level ?? -1 : -1 })));
  updatePath();
  setText($("level-kind"), m("Verilog-A module"));
  renderCard(node);
  updateHierarchy();
  const f = fileOf(node);
  if (node.lines) revealLine(f, node.lines[0], false, true);
  return true;
}

/** The index of the file of the module `node` among the files of the Document shown. */
const fileOf = (node) => doc.files.findIndex((x) => x.path === node.file);

/** Open the module the instance `child` of the module shown names, below it on the path. */
function enter(child, options) {
  const target = levelTree().nodes[child.host];
  if (!target || target.file === null) return;
  if (!options?.newWindow) window.dispatchEvent(new Event("viewer-navigate"));
  const step = { title: target.title, via: child.instance };
  if (previewModules()) return navigate({ path: [...path, step], selection: null, reveal: true }, options);
  return navigate({ source: viewOfFile(target.file), path: [...path, step], selection: null, reveal: true }, options);
}

/** Show the module that the path `next` of the Document shown leads to, when it leads to no level of it. */
export function showModuleOf(next) {
  // (in a preview, below levels of the preview)
  if (previewModules()) return next.at(-1)?.level < 0 && showModule(next);
  return next.every((s) => !sameSource(s.entry, currentSource()) || s.level < 0) && showModule(next);
}

function renderCard(node) {
  card.replaceChildren();
  const heading = document.createElement("div");
  heading.className = "module-heading";
  // the name of the language is no word to translate
  append(heading, span("module-name", node.title), span("badge", "Verilog-A"));
  const note = document.createElement("p");
  note.className = "module-note";
  setText(note, m("A module of Verilog-A has no drawing: its ports are shown as a symbol, and the modules its instances name are listed below."));
  append(card, heading, portSymbol(node.ports), note);
  const list = document.createElement("div");
  list.className = "module-instances";
  list.setAttribute("role", "list");
  setAttr(list, "aria-label", m("Instances"));
  if (node.children.length === 0) {
    const none = document.createElement("p");
    none.className = "module-none";
    setText(none, m("No instances of modules."));
    append(list, none);
  }
  for (const child of node.children) append(list, instanceRow(node, child));
  append(card, list);
  card.hidden = false;
}

/** A row of an instance of the module shown: its name, the module it names and the line it is on. */
function instanceRow(node, child) {
  const row = document.createElement("div");
  row.className = "module-instance";
  row.setAttribute("role", "listitem");
  row.tabIndex = 0;
  append(row, span("inst", child.instance ?? `${child.count}×`), span("master", child.master));
  if (child.host === null) {
    row.classList.add("undefined");
    append(row, span("badge", m("not defined")));
    setAttr(row, "title", m`No module is named ${child.master}`);
    return row;
  }
  if (child.line) append(row, span("line", m`line ${child.line}`));
  setAttr(row, "title", m`Click to show its line in the text; double-click or Enter opens ${child.master} (Shift opens a new window)`);
  const reveal = () => {
    for (const r of card.querySelectorAll(".module-instance.selected")) r.classList.remove("selected");
    row.classList.add("selected");
    if (child.line) revealLine(fileOf(node), child.line, true);
  };
  row.addEventListener("click", reveal);
  row.addEventListener("dblclick", (e) => enter(child, { newWindow: e.shiftKey }));
  row.addEventListener("keydown", (e) => {
    if (e.key === "Enter") enter(child, { newWindow: e.shiftKey });
    else if (e.key === " ") reveal();
    else return;
    e.preventDefault();
    e.stopPropagation();
  });
  return row;
}

/** The symbol of a module, or of a subcircuit that gives itself none: a box with a pin for each of its `ports`, in their order. */
export function portSymbol(ports = []) {
  const left = ports.slice(0, Math.ceil(ports.length / 2));
  const right = ports.slice(left.length);
  const pitch = 20;
  const pin = 20;
  // room for the longest names of both sides, in characters of about 7 pixels
  const longest = (names) => Math.max(0, ...names.map((n) => n.length));
  const width = Math.max(120, (longest(left) + longest(right)) * 7 + 24);
  const rows = Math.max(1, left.length, right.length);
  const height = rows * pitch + pitch;
  const svg = svgElement("svg", { class: "module-symbol", viewBox: `0 0 ${width + 2 * pin} ${height}`, width: width + 2 * pin, height, "aria-hidden": "true" });
  svg.append(svgElement("rect", { x: pin, y: 0.5, width, height: height - 1, rx: 2 }));
  const side = (names, x0, x1, anchor, tx) => names.forEach((name, k) => {
    const y = pitch * (k + 1);
    svg.append(svgElement("line", { x1: x0, y1: y, x2: x1, y2: y }));
    const t = svgElement("text", { x: tx, y: y + 4, "text-anchor": anchor });
    t.textContent = name;
    svg.append(t);
  });
  side(left, 0, pin, "start", pin + 4);
  side(right, pin + width, 2 * pin + width, "end", pin + width - 4);
  return svg;
}

// the card is no drawing: the wheel scrolls it, and a press or a double click on it is not one on the drawing
for (const type of ["wheel", "pointerdown", "dblclick", "mousedown", "contextmenu"]) card.addEventListener(type, (e) => e.stopPropagation());
