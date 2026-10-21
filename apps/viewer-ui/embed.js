// Canvas-only web viewer. The desktop and the normal web page never start this controller.
import { setBackground } from "./background.js";
import { startCommands } from "./commands.js";
import { clearDrawing, fit, zoomCenter } from "./drawing.js";
import { resetBuffers } from "./edits.js";
import { setGrid } from "./grid.js";
import { setHoverNames } from "./hover.js";
import { adoptSource, beginExternalOpen, showBuilt } from "./open.js";
import { shell } from "./shell.js";
import { prepareEmbed, sourceFromUrl } from "./shells/web.js";
import { current, doc, path, setDoc } from "./state.js";
import { backTo, enterRow, enterRoot } from "./subcircuits.js";
import { $, span, svgElement } from "./util.js";
import { startLoading, nextPaint } from "./loading.js";
import { m, setText, setAttr } from "./i18n.js";

const params = new URLSearchParams(location.search);
const targetOrigin = params.has("origin") ? params.get("origin") : "*";
let generation = 0;
let opening = false;
let reportedLevel = null;
let link;
let crumbs;

function send(type, fields = {}) {
  window.parent.postMessage({ rnl: 1, type, ...fields }, targetOrigin);
}

function error(reason) {
  const message = `Could not open the netlist: ${reason instanceof Error ? reason.message : String(reason)}`;
  setDoc(null);
  resetBuffers(null);
  clearDrawing(message);
  crumbs.hidden = true;
  link.hidden = true;
  reportedLevel = null;
  send("error", { message });
}

/** A level named by the API; preserve a real hierarchy path when it is reachable from the top. */
function level(name) {
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
  if (row) return enterRow(row, { newWindow: false });
  return enterRoot(wanted, { newWindow: false });
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
  if (!opening && name !== undefined && name !== reportedLevel) {
    reportedLevel = name;
    send("level", { name });
  }
}

/** Each open has an identity. Late reads, builds and errors never replace a newer request. */
async function open(message, initial = false) {
  const request = ++generation;
  const pending = beginExternalOpen();
  opening = true;
  document.body.classList.add("busy");
  const progress = startLoading();
  link.hidden = true;
  try {
    let source;
    let url = null;
    if (typeof message.text === "string" && typeof message.name === "string" && message.name.trim()) {
      source = { name: message.name, text: message.text, temporary: true };
    } else if (typeof message.url === "string") {
      source = sourceFromUrl(message.url);
      url = message.url;
    } else throw new Error("open needs text and name, or url.");
    await progress.stage("reading");
    const built = await shell.build(source, undefined, progress.setStage);
    if (request !== generation) return;
    await progress.stage("displaying");
    if (request !== generation) return;
    const errors = built.diagnostics.filter((item) => item.severity === "error");
    if (built.failed || errors.length) throw new Error(built.failed || errors[0].message);
    resetBuffers(built);
    adoptSource(source);
    showBuilt(built, false);
    pending.finish(); // the initial level may itself independently open a different file
    document.title = `${built.name} - RNL Viewer`;
    if (initial && params.has("level")) await level(params.get("level"));
    if (request !== generation) return;
    // A short embed fits visible ink, so its first view needs the drawing's actual font metrics.
    if (getComputedStyle($("canvas")).getPropertyValue("--fit-content").trim() === "1") {
      $("canvas").getBoundingClientRect();
      await document.fonts.ready;
      if (request !== generation) return;
    }
    fit();
    await nextPaint();
    if (request !== generation) return;
    if (url !== null) {
      const full = new URL(location.href);
      full.searchParams.delete("embed");
      full.searchParams.set("url", url);
      link.href = full.href;
      link.hidden = false;
    }
    opening = false;
    reportedLevel = null;
    send("loaded", {
      name: built.name,
      levels: doc.levels.map((item) => item.title),
      level: doc.levels[current]?.title ?? "",
      errors: errors.length,
      warnings: built.diagnostics.filter((item) => item.severity === "warning").length,
    });
    updateLevel();
  } catch (reason) {
    if (request === generation) error(reason);
  } finally {
    pending.finish();
    progress.finish();
    if (request === generation) {
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
    const [version] = await Promise.all([prepareEmbed(), startCommands()]);
    setBackground(params.get("background") === "dark" ? "dark" : "light");
    const grid = params.get("grid");
    setGrid({ shown: grid === "dots" || grid === "lines", style: grid === "lines" ? "lines" : "dots" });
    setHoverNames(params.get("names") !== "0");
    window.addEventListener("viewer-level", updateLevel);
    window.addEventListener("message", async (event) => {
      if (params.has("origin") && event.origin !== targetOrigin) return;
      const message = event.data;
      if (!message || typeof message !== "object" || message.rnl !== 1) return;
      try {
        switch (message.type) {
          case "open": void open(message); break;
          case "level":
            if (typeof message.name === "string") await level(message.name);
            break;
          case "fit": fit(); break;
          case "background":
            if (message.value === "light" || message.value === "dark") setBackground(message.value);
            break;
          default: break;
        }
      } catch (reason) {
        send("error", { message: reason instanceof Error ? reason.message : String(reason) });
      }
    });
    send("ready", { version });
    if (params.has("url")) void open({ url: params.get("url") }, true);
  } catch (reason) {
    error(reason);
  }
}
