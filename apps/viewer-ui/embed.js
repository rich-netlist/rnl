// Canvas-only web viewer. The desktop and the normal web page never start this controller.
import { setBackground } from "./background.js";
import { startCommands } from "./commands.js";
import { clearDrawing, fit, zoomCenter } from "./drawing.js";
import { resetBuffers } from "./edits.js";
import { setGrid } from "./grid.js";
import { setHoverNames } from "./hover.js";
import { adoptSource, beginOpenOperation, showBuilt } from "./open.js";
import { shell } from "./shell.js";
import { prepareViewer } from "./shells/web.js";
import { startViewerApi } from "./viewer-api.js";
import { viewerChannel } from "./window-session.js";
import { current, doc, path, setDoc } from "./state.js";
import { backTo, enterRow, enterRoot } from "./subcircuits.js";
import { $, span, svgElement } from "./util.js";
import { startLoading, nextPaint } from "./loading.js";
import { m, setText, setAttr } from "./i18n.js";

const params = new URLSearchParams(location.search);
let api;
let opening = false;
let reportedLevel = null;
let link;
let crumbs;

function send(type, fields = {}) {
  api?.send(type, fields);
}

function error(reason) {
  const message = `Could not open the netlist: ${reason instanceof Error ? reason.message : String(reason)}`;
  setDoc(null);
  resetBuffers(null);
  clearDrawing(message);
  crumbs.hidden = true;
  link.hidden = true;
  reportedLevel = null;
}

/** A level named by the API; preserve a real hierarchy path when it is reachable from the top. */
function level(name, signal) {
  const wanted = doc?.levels.findIndex((item) => item.title === name) ?? -1;
  if (wanted < 0) throw new Error(`Unknown level: ${name}`);
  function find(at, row, visited) {
    if (at === wanted) return row;
    if (visited.has(at)) return null;
    const next = new Set([...visited, at]);
    for (const [index, child] of doc.levels[at].children.entries()) {
      if (child.level === null) continue;
      const found = find(child.level, `${row}/${index}`, next);
      if (found) return found;
    }
    return null;
  }
  const row = find(0, "0", new Set());
  if (row) return enterRow(row, { newWindow: false, signal });
  return enterRoot(wanted, { newWindow: false, signal });
}

function updateLevel() {
  crumbs.replaceChildren();
  // Uncalled subcircuits can be roots of their own: still show their level name below the top.
  crumbs.hidden = !doc || (current === 0 && path.length < 2);
  if (!doc) return;
  path.forEach((step, index) => {
    if (index) crumbs.append(span("embed-separator", "›"));
    const name = step.title;
    const item = document.createElement(index === path.length - 1 ? "span" : "button");
    item.textContent = name;
    if (item instanceof HTMLButtonElement) {
      item.type = "button";
      setAttr(item, "title", m`Back to ${name}`);
      item.addEventListener("click", (e) => backTo(index, { newWindow: e.shiftKey }));
    } else item.setAttribute("aria-current", "location");
    crumbs.append(item);
  });
  const name = doc.levels[current]?.title;
  if ((!opening || opening.aborted) && name !== undefined && name !== reportedLevel) {
    reportedLevel = name;
    send("level", { name });
  }
}

/** Each open has an identity. Late reads, builds and errors never replace a newer request. */
async function open(source, packages, isCurrent, initial, message, signal) {
  opening = signal;
  document.body.classList.add("busy");
  const progress = startLoading();
  link.hidden = true;
  let built;
  try {
    const url = message.url ?? null, manifest = message.manifest ?? null;
    await progress.stage("reading");
    built = await shell.build(source, undefined, progress.setStage, packages, signal);
    if (!isCurrent()) return;
    await progress.stage("displaying");
    if (!isCurrent()) return;
    const errors = built.diagnostics.filter((item) => item.severity === "error");
    if (built.failed || errors.length) throw new Error(built.failed || errors[0].message);
    resetBuffers(built);
    adoptSource(source);
    showBuilt(built, false);
    document.title = `${built.name} - RNL Viewer`;
    // the address's level first; else a gallery design's, where "(top)" is the level it opens on
    const wanted = (initial && params.get("level")) || source.gallery?.level;
    if (wanted && wanted !== "(top)") await level(wanted, signal);
    if (!isCurrent()) return;
    // A short embed fits visible ink, so its first view needs the drawing's actual font metrics.
    if (getComputedStyle($("canvas")).getPropertyValue("--fit-content").trim() === "1") {
      $("canvas").getBoundingClientRect();
      await document.fonts.ready;
      if (!isCurrent()) return;
    }
    fit();
    await nextPaint();
    if (!isCurrent()) return;
    if (url !== null || manifest !== null) {
      const full = new URL(location.href);
      full.searchParams.delete("embed");
      full.searchParams.delete(url !== null ? "manifest" : "url");
      full.searchParams.set(url !== null ? "url" : "manifest", url ?? manifest);
      link.href = full.href;
      link.hidden = false;
    }
    opening = false;
    reportedLevel = null;
    const result = {
      name: built.name,
      levels: doc.levels.map((item) => item.title),
      level: doc.levels[current]?.title ?? "",
      errors: errors.length,
      warnings: built.diagnostics.filter((item) => item.severity === "warning").length,
    };
    updateLevel();
    return result;
  } finally {
    if (built) shell.discardBuild?.(built);
    progress.finish();
    if (isCurrent()) {
      opening = false;
      document.body.classList.remove("busy");
    }
  }
}

function controls() {
  const toolbar = document.createElement("div");
  toolbar.className = "embed-toolbar";
  toolbar.setAttribute("role", "group");
  setAttr(toolbar, "aria-label", m("Canvas tools"));
  const buttons = [
    ["fit", m("Fit to window"), "M6 2H2v4m8-4h4v4M2 10v4h4m8-4v4h-4", fit],
    ["zoom-in", m("Zoom in"), "M3 8h10M8 3v10", () => zoomCenter(1.25)],
    ["zoom-out", m("Zoom out"), "M3 8h10", () => zoomCenter(0.8)],
  ];
  for (const [id, label, drawing, action] of buttons) {
    const button = document.createElement("button");
    button.type = "button";
    button.id = `embed-${id}`;
    setAttr(button, "title", label);
    setAttr(button, "aria-label", label);
    const icon = svgElement("svg", { viewBox: "0 0 16 16", "aria-hidden": "true" });
    icon.append(svgElement("path", { d: drawing, fill: "none", stroke: "currentColor", "stroke-width": 1.5,
      "stroke-linecap": "round", "stroke-linejoin": "round" }));
    button.append(icon);
    button.addEventListener("click", action);
    toolbar.append(button);
  }
  crumbs = document.createElement("nav");
  crumbs.id = "embed-path";
  crumbs.hidden = true;
  setAttr(crumbs, "aria-label", m("Hierarchy path"));
  link = document.createElement("a");
  link.id = "embed-open";
  setText(link, m("Open in RNL Viewer ↗"));
  link.target = "_blank";
  link.rel = "noopener noreferrer";
  link.hidden = true;
  // The canvas owns its pointer gestures. Overlay controls must not begin a pan or a selection.
  for (const element of [toolbar, crumbs, link]) {
    for (const type of ["pointerdown", "dblclick", "wheel"]) element.addEventListener(type, (event) => event.stopPropagation());
  }
  $("canvas").append(toolbar, crumbs, link);
}

export async function startEmbed() {
  controls();
  clearDrawing(m("Waiting for a netlist."));
  $("canvas").tabIndex = 0;
  try {
    const [version] = await Promise.all([prepareViewer(), startCommands()]);
    setBackground(params.get("background") === "dark" ? "dark" : "light");
    const grid = params.get("grid");
    setGrid({ shown: grid === "dots" || grid === "lines", style: grid === "lines" ? "lines" : "dots" });
    setHoverNames(params.get("names") !== "0");
    window.addEventListener("viewer-level", updateLevel);
    api = startViewerApi({ version, open, onError: message => error(message.replace(/^Could not open the netlist: /, "")), begin(signal) {
      const operation = beginOpenOperation();
      signal.addEventListener("abort", () => operation.abort(), { once: true });
      return operation.signal;
    }, async command(message) {
        switch (message.type) {
          case "level":
            if (typeof message.name === "string") await level(message.name);
            break;
          case "fit": fit(); break;
          case "background":
            if (message.value === "light" || message.value === "dark") setBackground(message.value);
            break;
          default: break;
        }
    } });
    if (params.has("url")) void api.open({ url: params.get("url") }, true);
    else if (params.has("manifest")) void api.open({ manifest: params.get("manifest") }, true);
  } catch (reason) {
    error(reason);
    // Initialization can fail before the shared API has announced readiness.
    viewerChannel().send("error", { message: `Could not open the netlist: ${reason instanceof Error ? reason.message : String(reason)}` });
  }
}
