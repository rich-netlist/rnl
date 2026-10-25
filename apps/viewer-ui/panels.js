// The side panel and its parts: the view it shows, what is folded, and the sizes the splitters
// set, all kept through the shell's settings storage for the next start.

import { setCheck, setEnabled } from "./commands.js";
import { updateHierarchy } from "./hierarchy.js";
import { revealPending, refreshSource, updateSourceFilters } from "./text.js";
import { showUnplaced } from "./unplaced.js";
import { renderSymbol } from "./symbol.js";
import { renderUsages } from "./usages.js";
import { $, clamp, load, save } from "./util.js";

const app = $("app");
const source = $("source");
const sidebar = $("sidebar");
const netlistPane = $("pane-netlist");
const hierarchyPane = $("pane-hierarchy");

const SIDEBAR_MIN = 220;
const DRAWING_MIN = 280;
// upright and narrow, as style.css says: the side panel is under the drawing, its handle sets its height
export const upright = matchMedia("(max-width: 1023px) and (max-aspect-ratio: 4/5)");
// a touch screen lying down and low, a phone's: the side panel beside the drawing, narrow, folded at start
export const lowTouch = matchMedia("(pointer: coarse) and (max-height: 500px)");
const CANVAS_MIN = 80; // the drawing keeps a strip of this many pixels above the side panel
const HANDLE = 24; // the handle between them
const PROBLEMS_MIN = 60;
const SOURCE_MIN = 120;
const INDEX_MIN = 120;
const TREE_MIN = 100;

// All entry points (tabs, shortcuts, restore and menu) publish the same visible-pane state.
function syncPanelCommands() {
  const shown = !app.classList.contains("sidebar-hidden");
  for (const name of ["files", "netlist", "hierarchy", "symbol", "usages", "unplaced", "solve"]) {
    const tab = $(`tab-${name}`);
    setCheck(`panel-${name}`, shown && !tab.hidden && tab.getAttribute("aria-selected") === "true");
  }
  setEnabled("panel-hide", shown);
}

export function setSidebar(shown) {
  app.classList.toggle("sidebar-hidden", !shown);
  save("sidebar-shown", shown);
  setCheck("toggle-netlist", shown);
  syncPanelCommands();
  if (shown) { refreshSource(); revealPending(); }
  else if (sidebar.contains(document.activeElement)) $("show-sidebar").focus({ preventScroll: true });
}

/** Select a side-panel view; Solve is available in shells with a solver. */
export function setPane(name) {
  const view =
    name === "files" || name === "hierarchy" || name === "symbol" || name === "usages" || name === "unplaced" || (name === "solve" && !$("tab-solve").hidden) ? name : "netlist";
  const panes = { files: $("pane-files"), netlist: netlistPane, hierarchy: hierarchyPane, symbol: $("pane-symbol"), usages: $("pane-usages"), unplaced: $("pane-unplaced"), solve: $("pane-solve") };
  for (const [k, el] of Object.entries(panes)) {
    el.hidden = k !== view;
    $(`tab-${k}`).setAttribute("aria-selected", String(k === view));
  }
  syncPanelCommands();
  // a solve shown at the next start is not: the netlist's view then
  save("side-pane", view === "solve" ? "netlist" : view);
  if (view === "netlist") {
    setProblemsHeight(load("problems-height", 180));
    refreshSource();
    revealPending();
  } else if (view === "hierarchy") {
    setIndexHeight(load("index-height", 260));
    updateHierarchy();
  } else if (view === "unplaced") {
    showUnplaced();
  } else if (view === "files") {
    window.dispatchEvent(new Event("library-shown"));
  } else if (view === "symbol") {
    renderSymbol();
  } else if (view === "usages") {
    renderUsages();
  }
}

/** A menu or shortcut opens the requested pane, including when the sidebar was collapsed. */
export function showPane(name) {
  const tab = $(`tab-${name}`);
  if (!tab || tab.hidden) return;
  if (app.classList.contains("sidebar-hidden")) setSidebar(true);
  setPane(name);
  tab.focus({ preventScroll: true });
}

export function setShowFailed(shown) {
  $("show-failed").checked = shown;
  save("show-failed", shown);
  updateSourceFilters();
}

$("show-failed").addEventListener("change", () => setShowFailed($("show-failed").checked));

export function setProblems(shown) {
  $("problems-head").setAttribute("aria-expanded", String(shown));
  netlistPane.classList.toggle("problems-folded", !shown);
  save("problems-shown", shown);
  setCheck("toggle-problems", shown);
}

function sidebarWidthLimits() {
  const w = window.innerWidth;
  // a phone lying down keeps most of its width for the drawing
  if (lowTouch.matches) return [SIDEBAR_MIN, Math.max(SIDEBAR_MIN, w * 0.3)];
  return [SIDEBAR_MIN, Math.max(SIDEBAR_MIN, Math.min(w * 0.75, w - DRAWING_MIN))];
}

// the side panel under the drawing: at least its tabs, and at most what leaves the drawing its strip
function panelHeightLimits() {
  // no layout yet: the height is taken as it is, and limited once there is one
  if (app.clientHeight === 0) return [0, Infinity];
  const tabs = sidebar.querySelector(".side-head").offsetHeight || 44;
  const drawing = document.querySelector(".drawing"), canvas = $("canvas");
  const around = drawing && canvas ? drawing.clientHeight - canvas.clientHeight : 0;
  return [tabs, Math.max(tabs, app.clientHeight - HANDLE - around - CANVAS_MIN)];
}

// a hidden pane has no height yet: its size is taken as it is, and limited once it is shown
function problemsHeightLimits() {
  if (netlistPane.clientHeight === 0) return [PROBLEMS_MIN, Infinity];
  const fixed = $("problems-head").offsetHeight + netlistPane.querySelector('.netlist-head').offsetHeight;
  return [PROBLEMS_MIN, Math.max(PROBLEMS_MIN, netlistPane.clientHeight - fixed - SOURCE_MIN)];
}

function indexHeightLimits() {
  if (hierarchyPane.clientHeight === 0) return [INDEX_MIN, Infinity];
  const fixed = hierarchyPane.querySelector(".sub-head").offsetHeight;
  return [INDEX_MIN, Math.max(INDEX_MIN, hierarchyPane.clientHeight - fixed - TREE_MIN)];
}

function setSize(name, px, limits) {
  const [lo, hi] = limits();
  const v = Math.round(clamp(px, lo, hi));
  document.documentElement.style.setProperty(name, `${v}px`);
  return v;
}

export const setSidebarWidth = (px) => setSize("--sidebar-w", px, sidebarWidthLimits);
export const setProblemsHeight = (px) => setSize("--problems-h", px, problemsHeightLimits);
export const setIndexHeight = (px) => setSize("--index-h", px, indexHeightLimits);
export const setPanelHeight = (px) => setSize("--panel-h", px, panelHeightLimits);

/** Lay the side panel out for the screen: under the drawing when upright, its handle then horizontal. */
function layOut() {
  $("vsplit").setAttribute("aria-orientation", upright.matches ? "horizontal" : "vertical");
  if (upright.matches) setPanelHeight(load("panel-height", Math.round(window.innerHeight / 3)));
}
upright.addEventListener("change", layOut);
layOut();
requestAnimationFrame(layOut);

/**
 * Drag a splitter: `axis` "x" or "y"; `get` the size when the drag starts; `set` the new size; or a
 * function `axis` that gives all of them, as `[axis, sign, get, set, key]`, when the drag starts.
 */
function splitter(el, axis, sign, get, set, key) {
  el.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    if (typeof axis === "function") return splitWith(el, e, ...axis());
    splitWith(el, e, axis, sign, get, set, key);
  });
}

function splitWith(el, e, axis, sign, get, set, key) {
  {
    e.preventDefault();
    el.setPointerCapture(e.pointerId);
    el.classList.add("active");
    document.body.classList.add(axis === "x" ? "resizing-x" : "resizing-y");
    const start = axis === "x" ? e.clientX : e.clientY;
    const base = get();
    let last = base;
    const move = (ev) => {
      last = set(base + sign * ((axis === "x" ? ev.clientX : ev.clientY) - start));
    };
    const up = () => {
      el.releasePointerCapture(e.pointerId);
      el.classList.remove("active");
      document.body.classList.remove("resizing-x", "resizing-y");
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
      save(key, last);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
  }
}

// beside the drawing the handle sets the panel's width; under it, its height
splitter($("vsplit"), () => (upright.matches
  ? ["y", -1, () => sidebar.getBoundingClientRect().height, setPanelHeight, "panel-height"]
  : ["x", 1, () => sidebar.getBoundingClientRect().width, setSidebarWidth, "sidebar-width"]));
splitter($("hsplit"), "y", -1, () => $("problems-body").getBoundingClientRect().height, setProblemsHeight, "problems-height");
splitter($("hsplit2"), "y", -1, () => $("index-section").getBoundingClientRect().height, setIndexHeight, "index-height");
splitter($("usage-split"), "y", 1, () => $("usage-list").getBoundingClientRect().height,
  px => setSize("--usage-list-h", px, () => [80, Math.max(80, $("pane-usages").clientHeight - 125)]), "usage-list-height");

window.addEventListener("resize", () => {
  if (upright.matches && !app.classList.contains("sidebar-hidden")) setPanelHeight(sidebar.getBoundingClientRect().height);
  else if (!upright.matches) setSidebarWidth(sidebar.getBoundingClientRect().width || load("sidebar-width", 420));
  if (!netlistPane.hidden && !netlistPane.classList.contains("problems-folded")) {
    setProblemsHeight($("problems-body").getBoundingClientRect().height);
  }
  if (!hierarchyPane.hidden) setIndexHeight($("index-section").getBoundingClientRect().height);
});

$("hide-sidebar").addEventListener("click", () => setSidebar(false));
$("show-sidebar").addEventListener("click", () => setSidebar(true));
$("problems-head").addEventListener("click", () => setProblems(netlistPane.classList.contains("problems-folded")));
// a tab of the side panel folded to its tabs unfolds it
sidebar.querySelector(".view-tabs").addEventListener("click", () => {
  if (app.classList.contains("sidebar-hidden")) setSidebar(true);
}, true);
$("tab-files").addEventListener("click", () => setPane("files"));
$("tab-netlist").addEventListener("click", () => setPane("netlist"));
$("tab-hierarchy").addEventListener("click", () => setPane("hierarchy"));
$("tab-symbol").addEventListener("click", () => setPane("symbol"));
$("tab-usages").addEventListener("click", () => setPane("usages"));
$("tab-unplaced").addEventListener("click", () => setPane("unplaced"));
$("tab-solve").addEventListener("click", () => setPane("solve"));
