// The side panel and its parts: the view it shows, what is folded, and the sizes the splitters
// set, all kept through the shell's settings storage for the next start.

import { setCheck } from "./commands.js";
import { updateHierarchy } from "./hierarchy.js";
import { revealPending, refreshSource, updateSourceFilters } from "./text.js";
import { showUnplaced } from "./unplaced.js";
import { renderSymbol } from "./symbol.js";
import { $, clamp, load, save } from "./util.js";

const app = $("app");
const source = $("source");
const sidebar = $("sidebar");
const netlistPane = $("pane-netlist");
const hierarchyPane = $("pane-hierarchy");

const SIDEBAR_MIN = 220;
const DRAWING_MIN = 280;
const PROBLEMS_MIN = 60;
const SOURCE_MIN = 120;
const INDEX_MIN = 120;
const TREE_MIN = 100;

export function setSidebar(shown) {
  app.classList.toggle("sidebar-hidden", !shown);
  save("sidebar-shown", shown);
  setCheck("toggle-netlist", shown);
  if (shown) { refreshSource(); revealPending(); }
}

/**
 * Show the "netlist", the "hierarchy", the objects "unplaced" (not drawn) or the "solve" view of the
 * side panel; the solve's only while a solve is shown (solve.js), and the netlist's for any other
 * name.
 */
export function setPane(name) {
  const view =
    name === "files" || name === "hierarchy" || name === "symbol" || name === "unplaced" || (name === "solve" && !$("tab-solve").hidden) ? name : "netlist";
  const panes = { files: $("pane-files"), netlist: netlistPane, hierarchy: hierarchyPane, symbol: $("pane-symbol"), unplaced: $("pane-unplaced"), solve: $("pane-solve") };
  for (const [k, el] of Object.entries(panes)) {
    el.hidden = k !== view;
    $(`tab-${k}`).setAttribute("aria-selected", String(k === view));
  }
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
  }
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
  return [SIDEBAR_MIN, Math.max(SIDEBAR_MIN, Math.min(w * 0.75, w - DRAWING_MIN))];
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

/** Drag a splitter: `axis` "x" or "y"; `get` the size when the drag starts; `set` the new size. */
function splitter(el, axis, sign, get, set, key) {
  el.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
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
  });
}

splitter($("vsplit"), "x", 1, () => sidebar.getBoundingClientRect().width, setSidebarWidth, "sidebar-width");
splitter($("hsplit"), "y", -1, () => $("problems-body").getBoundingClientRect().height, setProblemsHeight, "problems-height");
splitter($("hsplit2"), "y", -1, () => $("index-section").getBoundingClientRect().height, setIndexHeight, "index-height");

window.addEventListener("resize", () => {
  setSidebarWidth(sidebar.getBoundingClientRect().width || load("sidebar-width", 420));
  if (!netlistPane.hidden && !netlistPane.classList.contains("problems-folded")) {
    setProblemsHeight($("problems-body").getBoundingClientRect().height);
  }
  if (!hierarchyPane.hidden) setIndexHeight($("index-section").getBoundingClientRect().height);
});

$("hide-sidebar").addEventListener("click", () => setSidebar(false));
$("show-sidebar").addEventListener("click", () => setSidebar(true));
$("problems-head").addEventListener("click", () => setProblems(netlistPane.classList.contains("problems-folded")));
$("tab-files").addEventListener("click", () => setPane("files"));
$("tab-netlist").addEventListener("click", () => setPane("netlist"));
$("tab-hierarchy").addEventListener("click", () => setPane("hierarchy"));
$("tab-symbol").addEventListener("click", () => setPane("symbol"));
$("tab-unplaced").addEventListener("click", () => setPane("unplaced"));
$("tab-solve").addEventListener("click", () => setPane("solve"));
