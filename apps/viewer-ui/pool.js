// The Library: session files, one temporary source, ordered desktop PATH groups and built-ins.
import { inform } from "./dialog.js";
import { changes } from "./edits.js";
import { currentSource, isLoading, openSource } from "./open.js";
import { setPane, setSidebar } from "./panels.js";
import { shell } from "./shell.js";
import { navigate } from "./navigation.js";
import { $, load, save, settingsPath } from "./util.js";

let items = [];
let temporary = null;
let currentPath = null;
let changing = false;
let scanVersion = 0;
const expanded = new Map([["files", true]]); // session only; never written to settings
let scanned = new Map();

// Native choosers/CLI arguments use Windows separators; the lookup provider lists forward slashes.
const filePath = (path) => shell.platform === "desktop" ? path.replace(/\\/g, "/") : path;

function button(label, action, title = label) {
  const b = document.createElement("button");
  b.type = "button";
  b.textContent = label;
  b.title = title;
  b.setAttribute("aria-label", title);
  b.addEventListener("click", action);
  return b;
}

function addItems(added) {
  for (const item of added) {
    item.path = filePath(item.path);
    item.source.path = filePath(item.source.path || item.path);
    const at = items.findIndex((old) => old.path === item.path && !old.builtin);
    if (at < 0) { items.push(item); expanded.set("files", true); }
    else items[at] = item;
  }
  shell.setPool?.(items);
}

/** Called only when a root is opened/applied, never when a Netlist include tab is selected. */
export function sourceOpened(source, doc, add, applied = false) {
  const held = source.temporary || (source.text !== undefined && !source.path && !source.readonly);
  if (held) {
    if (add) temporary = { source, name: source.name || "Pasted text" };
    // Store applied text, so discarded, unapplied editor changes cannot leak into this slot.
    if (applied && temporary?.source === source && doc) source.text = doc.files[0]?.text ?? source.text;
    currentPath = null;
  } else {
    const path = filePath(source.path || doc?.path || source.name);
    currentPath = path;
    if (add && !source.readonly && !items.some((i) => i.path === path && i.builtin)) {
      source.path = path;
      const item = shell.poolItem?.(source) || { path, name: source.name, source };
      // Keep Include's display name/read handle when opening its own source again.
      if (!items.some((i) => i.path === item.path && i.source === source)) addItems([item]);
    }
  }
  render();
}

async function update(action, rescan = false) {
  if (changing || isLoading()) return;
  changing = true;
  try {
    if (await action() === false) return false;
    shell.setPool?.(items);
    if (rescan) await scanPaths();
    else render();
    // Rebuild applied text without re-adding an excluded root or dropping pending edits.
    if (currentSource()) await openSource(currentSource(), true, changes(true));
    return true;
  } catch (e) {
    render();
    await inform({ title: "Library", paragraphs: [String(e)] });
  } finally {
    changing = false;
  }
}

export async function includeFiles(kind) {
  if (!shell.include) return;
  const changed = await update(async () => {
    const added = await shell.include(kind);
    if (!added.length) return false;
    addItems(added);
  });
  if (changed) { setSidebar(true); setPane("files"); }
}

/** Include the exact published package offered beneath a missing-package diagnostic. */
export async function includePublicPackage(pkg) {
  if (!shell.includePackage) return;
  return update(async () => {
    const added = await shell.includePackage(pkg);
    if (!added.length) return false;
    addItems(added);
  });
}

function paths() {
  const value = load("search-paths", []);
  return Array.isArray(value) ? value.filter((p) => typeof p === "string") : [];
}

async function scanPaths() {
  if (!shell.pathPackages) return;
  const version = ++scanVersion;
  const results = await Promise.all(paths().map(async (dir) => {
    try { return [dir, { files: await shell.pathPackages(dir) }]; }
    catch (e) { return [dir, { files: [], error: String(e) }]; }
  }));
  if (version !== scanVersion) return;
  scanned = new Map(results);
  render();
}

function span(className, text) {
  const el = document.createElement("span");
  el.className = className;
  el.textContent = text;
  return el;
}

function fileRow(item, kind) {
  const row = document.createElement(kind === "temporary" ? "div" : "li");
  row.className = "pool-row";
  row.dataset.path = item.path || "";
  if (kind === "temporary") row.classList.add("pool-temporary");
  const active = kind === "temporary" ? currentSource() === item.source : currentPath === item.path;
  row.classList.toggle("current", active);
  if (active) row.setAttribute("aria-current", "true");
  const open = button(item.name, (e) => {
    navigate({ source: item.source }, { newWindow: e.shiftKey });
  }, item.path || item.name);
  open.className = "pool-open";
  open.setAttribute("aria-label", `Open ${item.name}`);
  row.append(open);
  if (kind === "builtin" || kind === "temporary") {
    row.append(span("pool-badge", kind === "builtin" ? "read only" : "temporary · not searched"));
  } else if (kind === "files") {
    row.append(button("Exclude", () => update(() => { items = items.filter((i) => i !== item); }), `Exclude ${item.name}`));
  }
  return row;
}

function group(key, title, files, kind, pathInfo) {
  const el = document.createElement("section");
  el.className = "lib-group";
  el.dataset.group = key;
  const head = document.createElement("div");
  head.className = "lib-group-head";
  const list = document.createElement("ul");
  list.className = "pool-list";
  if (kind === "files") list.id = "pool-list";
  const toggle = button("", () => {
    const shown = !expanded.get(key);
    expanded.set(key, shown);
    toggle.setAttribute("aria-expanded", String(shown));
    list.hidden = !shown;
  }, title);
  toggle.className = "lib-toggle";
  toggle.setAttribute("aria-expanded", String(!!expanded.get(key)));
  toggle.append(span("chevron", ""));
  if (pathInfo) {
    const path = span("lib-path", title);
    path.title = title;
    toggle.append(span("lib-ordinal", `PATH ${pathInfo.index + 1}`), path);
  } else toggle.append(span("lib-title", title));
  toggle.append(span("lib-count", String(files.length)));
  head.append(toggle);
  if (pathInfo) {
    const { index, all } = pathInfo;
    for (const [label, tooltip, delta] of [["↑", "Move up in PATH", -1], ["↓", "Move down in PATH", 1], ["×", "Remove from PATH", 0]]) {
      const act = button(label, () => update(async () => {
        const next = paths();
        if (delta) [next[index], next[index + delta]] = [next[index + delta], next[index]];
        else next.splice(index, 1);
        await save("search-paths", next);
      }, true), tooltip);
      act.className = "lib-act";
      act.disabled = delta !== 0 && (index + delta < 0 || index + delta >= all.length);
      head.append(act);
    }
  }
  list.hidden = !expanded.get(key);
  for (const item of files) list.append(fileRow(item, kind));
  if (!files.length) {
    const empty = document.createElement("li");
    const error = pathInfo && scanned.get(title)?.error;
    empty.className = error ? "lib-warning" : "lib-empty";
    empty.textContent = error || (kind === "path" ? "No .rnl packages found." : "No files.");
    list.append(empty);
  }
  el.append(head, list);
  return el;
}

function render() {
  const groups = $("library-groups");
  groups.replaceChildren();
  if (temporary) groups.append(fileRow(temporary, "temporary"));
  groups.append(group("files", "Files", items.filter((i) => !i.builtin), "files"));
  if (shell.pathPackages) paths().forEach((dir, index, all) => {
    const prefix = dir.replace(/\\/g, "/").replace(/\/$/, "") + "/";
    const files = (scanned.get(dir)?.files || []).map((path) => ({
      path, name: path.startsWith(prefix) ? path.slice(prefix.length) : path,
      source: { path, name: path },
    }));
    groups.append(group(`path:${dir}`, dir, files, "path", { index, all }));
  });
  groups.append(group("builtin", "Built-in", items.filter((i) => i.builtin), "builtin"));
}

export async function startPool() {
  if (shell.bundledFiles) {
    const bundled = await shell.bundledFiles();
    items = Object.entries(bundled).map(([path, text]) => ({ path, name: path.replace("<bundled>/", ""), builtin: true,
      source: { path, name: path, text: shell.platform === "web" ? text : undefined, readonly: true } }));
  }
  $("include-file").addEventListener("click", () => includeFiles("file"));
  $("include-folder").addEventListener("click", () => includeFiles("folder"));
  $("include-library").hidden = !shell.includeLibrary;
  $("include-library").addEventListener("click", () => includeFiles("library"));
  $("add-path").hidden = !shell.pathPackages;
  $("path-note").textContent = shell.pathPackages ? `Settings: ${settingsPath}`
    : "Browsers cannot search local directories. Include files or folders above; the file pool lasts until this page closes.";
  $("add-path").addEventListener("click", () => update(async () => {
    const path = await shell.pickPath();
    if (!path || paths().includes(path)) return false;
    await save("search-paths", [...paths(), path]);
  }, true));
  shell.onInclude?.((added) => update(() => {
    if (!added.length) return false;
    addItems(added);
    setSidebar(true);
    setPane("files");
  }));
  window.addEventListener("library-shown", scanPaths);
  window.addEventListener("settings-error", (event) => {
    $("path-note").textContent = `Settings could not be saved: ${event.detail}`;
  });
  render();
  await scanPaths();
}

export const poolSnapshot = () => items.filter((i) => !i.builtin).slice();
export async function restorePool(pool) {
  addItems(pool);
  render();
}
