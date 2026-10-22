import { append, m, setAttr, setText } from "./i18n.js";
// The Library: session files, one temporary source, ordered desktop PATH groups and built-ins.
import { inform } from "./dialog.js";
import { setCheck } from "./commands.js";
import { changes, rebaseSource } from "./edits.js";
import { currentSource, isLoading, openSource } from "./open.js";
import { setPane, setSidebar } from "./panels.js";
import { shell } from "./shell.js";
import { navigate } from "./navigation.js";
import { $, load, save, settingsPath } from "./util.js";

let items = [];
let temporary = null;
let currentPath = null;
let changing = false;
export const isLibraryChanging = () => changing;
let scanVersion = 0;
const expanded = new Map([["files", true]]); // session only; never written to settings
let scanned = new Map();
let libraryErrors = [];
const refreshPaths = new Set();

async function syncPool(reload = false) {
  const all = [...items];
  const seen = new Set(all.map((i) => i.path));
  for (const { files } of scanned.values()) for (const path of files) {
    if (!seen.has(path)) { all.push({ path, searchPath: true, source: { path, name: path } }); seen.add(path); }
  }
  const refresh = [...refreshPaths];
  refreshPaths.clear();
  try { libraryErrors = await shell.setPool?.(all, reload, refresh) || []; }
  catch (e) { for (const path of refresh) refreshPaths.add(path); throw e; }
}

function showAutoInclude() {
  const enabled = load("auto-include", true) === true;
  $("auto-include").checked = enabled;
  setCheck("toggle-auto-include", enabled);
}

export async function toggleAutoInclude(value = !load("auto-include", true)) {
  if (changing || isLoading()) { showAutoInclude(); return; }
  await update(async () => { await save("auto-include", value); showAutoInclude(); });
}

export const reloadLibrary = () => update(async () => {}, true, true);

// Native choosers/CLI arguments use Windows separators; the lookup provider lists forward slashes.
const filePath = (path) => shell.platform === "desktop" ? path.replace(/\\/g, "/") : path;

function button(label, action, title = label) {
  const b = document.createElement("button");
  b.type = "button";
  setText(b, label);
  setAttr(b, "title", title);
  setAttr(b, "aria-label", title);
  b.addEventListener("click", action);
  return b;
}

function addItems(added, refresh = true) {
  for (const item of added) {
    item.path = filePath(item.path);
    item.source.path = filePath(item.source.path || item.path);
    if (refresh) refreshPaths.add(item.path);
    const at = items.findIndex((old) => old.path === item.path && !old.builtin);
    if (at < 0) { items.push(item); expanded.set("files", true); }
    else items[at] = item;
  }
}

/** Each independently opened entry, including cross-file navigation, joins Files here. */
export async function sourceOpened(source, doc, add, applied = false) {
  const held = source.temporary || (source.text !== undefined && !source.path && !source.readonly);
  if (held) {
    if (add) temporary = { source, name: source.name || "Pasted text" };
    // Store applied text, so discarded, unapplied editor changes cannot leak into this slot.
    if (applied && temporary?.source === source && doc?.files[0]) {
      source.text = doc.files[0].text;
      rebaseSource(doc.files[0].path, source.text);
    }
    currentPath = null;
  } else {
    const path = filePath(source.path || doc?.path || source.name);
    currentPath = path;
    if (add && !source.readonly && !items.some((i) => i.path === path && i.builtin)) {
      source.path = path;
      const item = shell.poolItem?.(source) || { path, name: source.name, source };
      // Keep Include's display name/read handle when opening its own source again.
      if (!items.some((i) => i.path === item.path && (i.source === source || source.library))) {
        // Selecting a PATH row promotes it to Files but does not change its contents. Its
        // definitions (and dependent roots) are already indexed. Include/Reload refresh them.
        addItems([item], !source.library);
        await syncPool();
      }
    }
  }
  render();
}

async function update(action, rescan = false, reload = false, valid = () => true) {
  if (changing || isLoading() || !valid()) return false;
  changing = true;
  if (reload) setText($("library-status"), m("Reloading library…"));
  $("library-groups").setAttribute("aria-busy", "true");
  try {
    if (await action() === false) return false;
    if (!valid()) return false;
    if (rescan) await scanPaths(reload);
    else { await syncPool(reload); render(); }
    if (!valid()) return false;
    // Rebuild applied text without re-adding an excluded root or dropping pending edits.
    if (currentSource()) await openSource(currentSource(), true, changes(true));
    return true;
  } catch (e) {
    render();
    await inform({ title: m("Library"), paragraphs: [String(e)] });
  } finally {
    changing = false;
    $("library-groups").setAttribute("aria-busy", "false");
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
  return includePublicPackages([pkg]);
}

/** An automatic offer adds all approved packages once, only while its document is still current. */
export async function includePublicPackages(packages, valid = () => true, signal) {
  if (!shell.includePackage) return;
  return update(async () => {
    if (!valid()) return false;
    const added = [];
    try {
      for (const pkg of packages) {
        if (!valid()) return false;
        added.push(...await shell.includePackage(pkg, signal));
      }
    } catch (error) {
      if (!valid()) return false;
      throw error;
    }
    if (!valid()) return false;
    if (!added.length) return false;
    addItems(added);
  }, false, false, valid);
}

function paths() {
  const value = load("search-paths", []);
  return Array.isArray(value) ? value.filter((p) => typeof p === "string") : [];
}

async function scanPaths(reload = false) {
  if (!shell.pathPackages) { await syncPool(reload); render(); return; }
  const version = ++scanVersion;
  const results = await Promise.all(paths().map(async (dir) => {
    try { return [dir, { files: await shell.pathPackages(dir) }]; }
    catch (e) { return [dir, { files: [], error: String(e) }]; }
  }));
  if (version !== scanVersion) return;
  scanned = new Map(results);
  await syncPool(reload);
  render();
}

function span(className, text) {
  const el = document.createElement("span");
  el.className = className;
  setText(el, text);
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
  setAttr(open, "aria-label", m`Open ${item.name}`);
  append(row, open);
  if (kind === "builtin" || kind === "temporary") {
    append(row, span("pool-badge", kind === "builtin" ? m("read only") : m("temporary · not searched")));
  } else if (kind === "files") {
    append(row, button(m("Exclude"), () => update(() => { items = items.filter((i) => i !== item); }), m`Exclude ${item.name}`));
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
  append(toggle, span("chevron", ""));
  if (pathInfo) {
    const path = span("lib-path", title);
    setAttr(path, "title", title);
    append(toggle, span("lib-ordinal", `PATH ${pathInfo.index + 1}`), path);
  } else append(toggle, span("lib-title", title));
  append(toggle, span("lib-count", String(files.length)));
  append(head, toggle);
  if (pathInfo) {
    const { index, all } = pathInfo;
    for (const [label, tooltip, delta] of [["↑", m("Move up in PATH"), -1], ["↓", m("Move down in PATH"), 1], ["×", m("Remove from PATH"), 0]]) {
      const act = button(label, () => update(async () => {
        const next = paths();
        if (delta) [next[index], next[index + delta]] = [next[index + delta], next[index]];
        else next.splice(index, 1);
        await save("search-paths", next);
      }, true), tooltip);
      act.className = "lib-act";
      act.disabled = delta !== 0 && (index + delta < 0 || index + delta >= all.length);
      append(head, act);
    }
  }
  list.hidden = !expanded.get(key);
  for (const item of files) append(list, fileRow(item, kind));
  if (!files.length) {
    const empty = document.createElement("li");
    const error = pathInfo && scanned.get(title)?.error;
    empty.className = error ? "lib-warning" : "lib-empty";
    setText(empty, error || (kind === "path" ? m("No packages or netlists found.") : m("No files.")));
    append(list, empty);
  }
  append(el, head, list);
  return el;
}

function render() {
  const groups = $("library-groups");
  groups.replaceChildren();
  if (temporary) append(groups, fileRow(temporary, "temporary"));
  append(groups, group("files", m("Files"), items.filter((i) => !i.builtin), "files"));
  if (shell.pathPackages) paths().forEach((dir, index, all) => {
    const prefix = dir.replace(/\\/g, "/").replace(/\/$/, "") + "/";
    const files = (scanned.get(dir)?.files || []).map((path) => ({
      path, name: path.startsWith(prefix) ? path.slice(prefix.length) : path,
      source: { path, name: path, library: true },
    }));
    append(groups, group(`path:${dir}`, dir, files, "path", { index, all }));
  });
  append(groups, group("builtin", m("Built-in"), items.filter((i) => i.builtin), "builtin"));
  setText($("library-status"), libraryErrors.join("\n"));
}

export async function startPool() {
  showAutoInclude();
  $("auto-include").addEventListener("change", (e) => toggleAutoInclude(e.target.checked));
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
  setText($("path-note"), shell.pathPackages ? m`Settings: ${settingsPath}`
    : m("Browsers cannot search local directories. Include files or folders above; the file pool lasts until this page closes."));
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
  window.addEventListener("library-shown", () => { if (!changing && !isLoading()) scanPaths().catch(console.error); });
  window.addEventListener("settings-error", (event) => {
    setText($("path-note"), m`Settings could not be saved: ${event.detail}`);
  });
  window.addEventListener("public-package-status", event => {
    const note = document.getElementById("public-package-status");
    setText(note, event.detail);
    note.hidden = !event.detail;
  });
  if (shell.publicPackages) {
    const note = span("lib-note", "");
    note.id = "public-package-status";
    note.hidden = true;
    $("library-groups").before(note);
    shell.publicPackages().catch(() => {});
  }
  render();
  await scanPaths();
}

export const poolSnapshot = () => items.filter((i) => !i.builtin).slice();
/** Export includes every built-in, even unused packages, and the clipboard's separate slot. */
export const librarySnapshot = () => ({ items: items.slice(), temporary: temporary?.source || null });
export async function restorePool(pool) {
  addItems(pool);
  await syncPool();
  render();
}
