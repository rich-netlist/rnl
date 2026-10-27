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
import { showManifestBar } from "./manifest-bar.js";
import { displayFollows } from "./display.js";
import { chooseFormat, chosenFormat, formatInfo, formatList, formatTitle, setFormatList } from "./formats.js";
import { sourcePath } from "./source-path.js";

let items = [];
let bundledPackages = []; // the desktop's bundled public packages: { name, version, kind, path }
let temporary = null;
let currentPath = null;
let changing = false;
export const isLibraryChanging = () => changing;
let scanVersion = 0;
const expanded = new Map([["files", true]]); // session only; never written to settings
let scanned = new Map();
let libraryErrors = [];
const refreshPaths = new Set();
// the files of the external manifest opened last (?manifest=): they stay listed, read only, while the page is open
let external = null;
// the host files of the document shown, by Library path: the format each is read in and the one it
// would be without the user's choice (SourceFile.format, default_format); its includes, after the first
let shownFormats = new Map();
let shownIncludes = [];
// the default formats of Library files that the document shown does not read, as the shell detects them
const detected = new Map();

async function syncPool(reload = false) {
  const all = [...items];
  const seen = new Set(all.map((i) => i.path));
  for (const { files } of scanned.values()) for (const path of files) {
    if (!seen.has(path)) { all.push({ path, searchPath: true, source: { path, name: path } }); seen.add(path); }
  }
  // the files of an external manifest are searched as the Library's are: for bare includes, packages
  // by name and Auto Include, by any file opened; its pool first, in its order
  if (external && shell.externalFile) {
    for (const path of [...external.pool, ...Object.keys(external.files)]) {
      if (seen.has(path) || external.files[path] === undefined) continue;
      const text = external.files[path];
      all.push({ path, external: true, read: () => new File([text], path.split("/").pop()), source: shell.externalFile(external, path) });
      seen.add(path);
    }
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

/** Keep the formats of the host files of `doc`, by their Library paths, for the format chips. */
function documentFormats(doc) {
  if (!doc) return;
  setFormatList(doc.formats);
  shownFormats = new Map();
  shownIncludes = [];
  doc.files.forEach((file, index) => {
    if (!file.format) return;
    const path = filePath(sourcePath(doc, file).replace(/^\/?<pool>(?=\/)/, ""));
    const entry = { path, name: file.name, format: file.format, auto: file.default_format };
    shownFormats.set(path, entry);
    if (index > 0) shownIncludes.push(entry);
  });
}

/** Each independently opened entry, including cross-file navigation, joins Files here. */
export async function sourceOpened(source, doc, add, applied = false) {
  if (source?.external && source.external !== external) {
    external = source.external;
    expanded.set("external", true);
    // its files join the Library's search; the build that opened it searched them already (shells/web.js)
    syncPool().catch((e) => console.error(e));
  }
  // the bar stays while the file shown is one of the manifest's, whichever opened it, and goes with another file
  const shown = source?.path || doc?.path;
  showManifestBar(source?.external || (external && shown && (shown in external.files || external.pool.includes(shown)) ? external : null));
  // and a manifest's display settings, the same way: those of the manifest a file is one of
  displayFollows(shown && filePath(shown));
  documentFormats(doc);
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

/** The PATH directories added with their sub-folders, which lookup searches too (desktop settings). */
function deepPaths() {
  const value = load("search-paths-subfolders", []);
  return new Set(Array.isArray(value) ? value.filter((p) => typeof p === "string") : []);
}

/** Keep `search-paths-subfolders` to the PATH directories `next` still has. */
async function saveDeepPaths(next, add = null) {
  const deep = [...deepPaths(), ...(add ? [add] : [])].filter((dir, i, all) => next.includes(dir) && all.indexOf(dir) === i);
  await save("search-paths-subfolders", deep);
}

/** Add a PATH directory, with its sub-folders when `subfolders`; one already there gains them. */
const addPath = (subfolders) => update(async () => {
  const path = await shell.pickPath();
  if (!path) return false;
  const known = paths().includes(path);
  if (known && (!subfolders || deepPaths().has(path))) return false;
  const next = known ? paths() : [...paths(), path];
  if (!known) await save("search-paths", next);
  await saveDeepPaths(next, subfolders ? path : null);
}, true);

async function scanPaths(reload = false) {
  if (!shell.pathPackages) { await syncPool(reload); render(); return; }
  const version = ++scanVersion;
  const deep = deepPaths();
  const results = await Promise.all(paths().map(async (dir) => {
    try {
      // an array from a shell that lists a directory alone, or { files, truncated }
      const found = await shell.pathPackages(dir, deep.has(dir));
      return [dir, Array.isArray(found) ? { files: found } : { files: found.files, truncated: found.truncated }];
    }
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

/**
 * The format chip of the file at `path`: the format it is read in, muted while it reads by default and
 * solid when the user chose it, which opens the menu of the core's formats. A file that the document
 * shown does not read shows its detected default, once the shell has detected it.
 */
function formatChip(path, name) {
  const shown = shownFormats.get(path);
  const chosen = chosenFormat(path);
  const auto = shown ? shown.auto : detected.get(path);
  const current = chosen && formatInfo(chosen) ? chosen : shown ? shown.format : auto;
  const chip = document.createElement("button");
  chip.type = "button";
  chip.className = "format-chip";
  chip.dataset.format = current || "";
  chip.classList.toggle("chosen", !!chosen);
  setText(chip, formatTitle(current) || "…");
  setAttr(chip, "aria-haspopup", "menu");
  setAttr(chip, "title", chosen ? m("Parse format: chosen by you") : m("Parse format: detected automatically"));
  setAttr(chip, "aria-label", m`Parse format of ${name}: ${formatTitle(current) || ""}`);
  chip.disabled = !formatList().length || (!shown && !auto);
  chip.addEventListener("click", () => formatMenu(chip, path, chosen, auto));
  if (!shown && !detected.has(path) && shell.detectFormat) {
    detected.set(path, undefined);
    shell.detectFormat(path).then((format) => { detected.set(path, format); render(); }, () => detected.delete(path));
  }
  return chip;
}

/** The menu of a format chip: Automatic, with the format it detects, then each format of the core. */
function formatMenu(chip, path, chosen, auto) {
  document.querySelector(".format-menu")?.remove();
  const menu = document.createElement("div");
  menu.className = "format-menu";
  menu.setAttribute("role", "menu");
  const close = () => { menu.remove(); document.removeEventListener("pointerdown", outside, true); };
  const outside = (e) => { if (!menu.contains(e.target) && e.target !== chip) close(); };
  const choose = (name) => {
    close();
    chip.focus();
    if ((name || undefined) === chosen) return;
    update(async () => { await chooseFormat(path, name); });
  };
  const entries = [[null, m`Automatic (${formatTitle(auto) || "?"})`], ...formatList().map((f) => [f.name, formatTitle(f.name)])];
  for (const [name, label] of entries) {
    const item = document.createElement("button");
    item.type = "button";
    item.className = "format-item";
    item.setAttribute("role", "menuitemradio");
    item.setAttribute("aria-checked", String((name || undefined) === chosen));
    item.dataset.format = name || "";
    // the check mark is drawn apart from the name, which screen readers read with aria-checked
    const check = span("format-check", (name || undefined) === chosen ? "✓" : "");
    check.setAttribute("aria-hidden", "true");
    append(item, check, span("format-label", label));
    item.addEventListener("click", () => choose(name));
    append(menu, item);
  }
  menu.addEventListener("keydown", (e) => {
    const items = [...menu.querySelectorAll(".format-item")];
    const at = items.indexOf(document.activeElement);
    if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(); chip.focus(); }
    else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault(); e.stopPropagation();
      items[(at + (e.key === "ArrowDown" ? 1 : items.length - 1)) % items.length].focus();
    }
  });
  chip.after(menu);
  const rect = chip.getBoundingClientRect();
  menu.style.top = `${rect.bottom + 2}px`;
  menu.style.left = `${Math.max(4, rect.right - menu.offsetWidth)}px`;
  document.addEventListener("pointerdown", outside, true);
  (menu.querySelector('[aria-checked="true"]') || menu.firstElementChild).focus();
}

/** A file that the netlist shown includes, under the row of the file opened, with its format chip. */
function includeRow(entry) {
  const row = document.createElement("li");
  row.className = "pool-include"; // not a .pool-row: it opens nothing and counts as no Library file
  row.dataset.path = entry.path;
  const name = span("pool-include-name", entry.name);
  setAttr(name, "title", entry.path);
  append(row, name, formatChip(entry.path, entry.name));
  return row;
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
  if (kind === "files" || kind === "path") append(row, formatChip(item.path, item.name));
  if (kind === "builtin" || kind === "temporary" || kind === "external" || kind === "public") {
    append(row, span("pool-badge", kind === "temporary" ? m("temporary · not searched") : m("read only")));
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
    if (pathInfo.deep) append(toggle, span("lib-deep", m("with sub-folders")));
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
        if (!delta) await saveDeepPaths(next);
      }, true), tooltip);
      act.className = "lib-act";
      act.disabled = delta !== 0 && (index + delta < 0 || index + delta >= all.length);
      append(head, act);
    }
  }
  list.hidden = !expanded.get(key);
  if (pathInfo && scanned.get(title)?.truncated) {
    const note = document.createElement("li");
    note.className = "lib-warning";
    setText(note, m`Only the first ${500} sub-folders are searched; add deeper folders as PATH entries of their own.`);
    append(list, note);
  }
  for (const item of files) {
    append(list, fileRow(item, kind));
    if (kind === "files" && item.path === currentPath) for (const entry of shownIncludes) append(list, includeRow(entry));
  }
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
  const deep = deepPaths();
  if (shell.pathPackages) paths().forEach((dir, index, all) => {
    const prefix = dir.replace(/\\/g, "/").replace(/\/$/, "") + "/";
    const files = (scanned.get(dir)?.files || []).map((path) => ({
      path, name: path.startsWith(prefix) ? path.slice(prefix.length) : path,
      source: { path, name: path, library: true },
    }));
    append(groups, group(`path:${dir}`, dir, files, "path", { index, all, deep: deep.has(dir) }));
  });
  // the public packages this build bundles, read only, searched after the PATH directories above
  if (bundledPackages.length) {
    const files = bundledPackages.map((p) => ({ path: p.path, name: `${p.name} ${p.version}`,
      source: { path: p.path, name: p.path, library: true, readonly: true } }));
    append(groups, group("public", m("Public packages (bundled)"), files, "public"));
  }
  if (external && shell.externalFile) {
    const root = `/${external.id}/`;
    const files = Object.keys(external.files).sort().map((path) => ({ path, name: path.slice(root.length), source: shell.externalFile(external, path) }));
    append(groups, group("external", m`From external manifest · ${external.id} · read only`, files, "external"));
  }
  append(groups, group("builtin", m("Built-in"), items.filter((i) => i.builtin), "builtin"));
  setText($("library-status"), libraryErrors.join("\n"));
}

export async function startPool() {
  showAutoInclude();
  // The core's formats, which a build is given only the choices of. The desktop lists them at once;
  // the web's core may take long to start, so the web does not wait: until a list comes (or the
  // first Document brings one), its choices go as they are, and the core checks them itself.
  const listing = shell.formats?.().then(setFormatList, () => {});
  if (shell.pathPackages) await listing;
  $("auto-include").addEventListener("change", (e) => toggleAutoInclude(e.target.checked));
  // the desktop's bundled public packages, in a group of their own; none in the build without them
  if (shell.bundledPackages) bundledPackages = await shell.bundledPackages().then((info) => info.packages || [], () => []);
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
    : m("Browsers cannot search local directories. Include files or folders above; Library files remain until this tab closes."));
  $("add-path-subfolders").hidden = !shell.pathPackages;
  $("add-path").addEventListener("click", () => addPath(false));
  $("add-path-subfolders").addEventListener("click", () => addPath(true));
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
  // The desktop's PATH must be scanned before the first build. The web has no PATH, and its index
  // waits for the core, as every build waits for the index itself (shell.setPool): startup need not.
  const scanning = scanPaths();
  if (shell.pathPackages) await scanning;
  else scanning.catch((e) => console.error(e));
}

export const poolSnapshot = () => items.filter((i) => !i.builtin).slice();
/** The desktop's PATH directories in their order: each with whether its sub-folders are searched, and its files. */
export const pathGroups = () => shell.pathPackages
  ? paths().map((dir) => ({ dir, subfolders: deepPaths().has(dir), files: (scanned.get(dir)?.files || []).slice() }))
  : [];

/**
 * Put a project's files in the Library in place of its Files (project.js), with the formats chosen
 * for some of them and its settings, before its entry is opened: nothing is built here. False when
 * the Library is busy.
 */
export async function setProjectFiles(added, formats = {}, settings = {}) {
  if (changing) return false;
  changing = true;
  try {
    items = items.filter((i) => i.builtin);
    temporary = null;
    external = null;
    addItems(added);
    // the project's own formats win over choices kept for these files, for this page: the settings keep the user's
    for (const [path, name] of Object.entries(formats)) await chooseFormat(filePath(path), name, { session: true });
    if (typeof settings.autoInclude === "boolean" && settings.autoInclude !== (load("auto-include", true) === true)) {
      await save("auto-include", settings.autoInclude);
      showAutoInclude();
    }
    await syncPool();
    render();
    return true;
  } finally {
    changing = false;
  }
}
/** Export includes every built-in, even unused packages, and the clipboard's separate slot. */
export const librarySnapshot = () => ({ items: items.slice(), temporary: temporary?.source || null });
export async function restorePool(pool) {
  addItems(pool);
  await syncPool();
  render();
}
