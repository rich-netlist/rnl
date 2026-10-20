// The web shell: the page in a browser, served with the core next to it as core/ (apps/web). It opens
// files the user chooses or drops, a folder, or a URL; reads only the files a document needs, as
// collect in the core's client asks for them; and builds the Document with the core as WebAssembly in
// a module Worker, one for the page's lifetime. The page shows its own menu bar.

import { collect, decodeText, startCore } from "../core/client.js";
import { ask, choose, inform, modal } from "../dialog.js";
import { openWebWindow, receiveWebSession } from "../window-session.js";
import { DEFAULT_CATALOG, readCatalog, readPublicPackage, latestPackages } from "../public-packages.js";

/** The extensions of netlists that are opened on their own, listed first as the file to open. */
const NETLISTS = ["sp", "spi", "spice", "cir", "ckt", "net", "scs"];
/** The extensions of files mostly included or used by a netlist, listed after the netlists. */
const PARTS = ["inc", "lib", "rnl"];
/** How long a server may send nothing, before or during its answer, until a fetch is given up (ms). */
const STALL = 30000;
const embed = new URLSearchParams(location.search).get("embed") === "1";
let embedStyle = Promise.resolve();

if (embed) {
  document.body.classList.add("embed");
  const style = document.createElement("link");
  style.rel = "stylesheet";
  style.href = "embed.css";
  embedStyle = new Promise((resolve, reject) => {
    style.onload = resolve;
    style.onerror = () => reject(new Error("The embed stylesheet could not be loaded."));
  });
  embedStyle.catch(() => {});
  document.head.append(style);
}

const core = startCore();
const built = new WeakSet(); // the sources built before: building one again reads it again
let opened = null; // the page's handler for sources opened without asking, such as dropped files
let included = null;
let poolFiles = new Map();
let lastUrl = ""; // the URL opened last, which Open URL offers again

/**
 * A source of this shell: files chosen or dropped, a folder, or a URL. The page's sources that hold
 * their text (shell.js) need none of this.
 *
 * @typedef {object} WebSource
 * @property {string} name How the page names it: the entry's file name, its path in a folder, or the URL.
 * @property {string} path The entry's rooted workspace path; a URL updates it after redirects.
 * @property {(again: boolean) => Promise<{ path: string, text: string }>} open Read the entry: its
 *   path in the workspace, rooted, such as "/top.sp", and its text; rejects with an Error that says
 *   why it cannot be read. `again` when the source is read again (File > Reload).
 * @property {(path: string, again: boolean) => Promise<string | null>} read Another file's text by
 *   its path in the workspace, or null when there is no such file; rejects with an Error that says
 *   why when the file is there but cannot be read.
 */

/**
 * Files held by the page, each by its rooted path in the workspace and a function that gives its
 * File: chosen, in a folder, or dropped. A file is read only when the core asks for it. The user
 * sees the paths without their leading /, such as top.sp or folder/top.sp.
 *
 * @param {Map<string, () => File | Promise<File>>} files
 * @param {string} entry
 * @returns {WebSource}
 */
function filesSource(files, entry) {
  async function read(path) {
    const file = files.get(path);
    if (!file) return null;
    try {
      return decodeText(await (await file()).arrayBuffer());
    } catch {
      // a browser reads a chosen file only as it was when it was chosen
      const why = "it may have changed or been removed since it was opened. Open it again.";
      throw new Error(`The browser could not read ${path.slice(1)}: ${why}`);
    }
  }
  return {
    files,
    name: entry.slice(1),
    path: entry,
    async open() {
      return { path: entry, text: await read(entry) };
    },
    read,
  };
}

/** Keep transferred text locally, and retain a URL source for includes not in that snapshot. */
function sessionSource(files, entry) {
  const local = filesSource(files, entry.path);
  const remote = entry.url ? urlSource(new URL(entry.url)) : null;
  return {
    ...(files.has(entry.path) ? local : remote || local), ...entry,
    read: async (path, again) => await local.read(path) ?? (remote ? remote.read(path, again) : null),
  };
}

/**
 * The path in the workspace of a file the Document names `path`: relative to the directory `dir` of
 * the netlist (`""` for the root) unless it is rooted, with `.` and `..` resolved as the core does,
 * a `..` above the root kept.
 */
function joined(dir, path) {
  const names = (path.startsWith("/") ? path : `${dir}/${path}`).split("/");
  const out = [];
  for (const name of names) {
    if (name === "" || name === ".") continue;
    if (name === ".." && out.length > 0 && out[out.length - 1] !== "..") out.pop();
    else out.push(name);
  }
  return `/${out.join("/")}`;
}

/** A path relative to the directory `dir` when it is in it ("" for the root), else as it is. */
function within(path, dir) {
  return path.startsWith(`${dir}/`) ? path.slice(dir.length + 1) : path;
}

/**
 * A file on a web server, and the files it includes, fetched relative to the address the file was
 * served from, after any redirect. The workspace names each file by its path on that server, such
 * as /netlists/a/top.sp or /top.sp, so that relative includes, ../ included, resolve as there: a URL
 * that redirects opens as the one it redirects to. A file that cannot be fetched is missing; the
 * entry is an error.
 *
 * @param {URL} url
 * @returns {WebSource}
 */
function urlSource(url) {
  let served = url; // where the entry was served from when it was last read
  let dir = serverPath(url).split("/").slice(0, -1); // also supports transferred sources read before open
  return {
    get url() { return served.href; },
    name: url.href,
    path: serverPath(url),
    async open(again) {
      const got = await fetchBytes(url.href, again);
      if (!got.bytes) throw new Error(`${url.href} could not be fetched: ${got.why}.`);
      served = new URL(got.url || url.href);
      const path = serverPath(served);
      this.path = path;
      dir = path.split("/").slice(0, -1);
      return { path, text: decodeText(got.bytes) };
    },
    async read(path, again) {
      if (!path.startsWith("/")) return null; // such as C:/models/x.lib
      const got = await fetchBytes(new URL(relative(dir, path), served).href, again);
      return got.bytes ? decodeText(got.bytes) : null;
    },
  };
}

/**
 * The path in the workspace of the file at `url`: its path on the server, each name decoded. The
 * address of a directory names the file after the directory, and the server's root after the
 * server: /netlists/ is /netlists/netlists, and / is /example.org.
 */
function serverPath(url) {
  const names = url.pathname.split("/").map(decode);
  if (names[names.length - 1] === "") names[names.length - 1] = names[names.length - 2] || url.hostname;
  return names.join("/");
}

/**
 * The bytes at `href` and the address they came from, or why they could not be fetched. `again`
 * fetches past the browser's cache. A server that sends nothing for STALL ms is given up.
 *
 * @param {string} href
 * @param {boolean} again
 * @returns {Promise<{ bytes: Uint8Array, url: string } | { bytes?: undefined, why: string }>}
 */
async function fetchBytes(href, again) {
  const legacy = /\/libraries\/(?!standard\/)(.+\.rnl)$/.exec(new URL(href).pathname);
  if (legacy) {
    const pkg = (await publicPackages()).find(p => p.path === decodeURIComponent(legacy[1]));
    if (!pkg) throw new Error(`No public package matches the old address: ${href}`);
    return { bytes: new Uint8Array(await readPublicPackage(pkg)), url: href };
  }
  const stop = new AbortController();
  let timer = 0;
  const wait = () => {
    clearTimeout(timer);
    timer = setTimeout(() => stop.abort(), STALL);
  };
  wait();
  try {
    const response = await fetch(href, { cache: again ? "reload" : "no-cache", signal: stop.signal });
    if (!response.ok) return { why: `${response.status} ${response.statusText}`.trim() };
    const chunks = [];
    if (response.body) {
      const reader = response.body.getReader();
      for (;;) {
        wait();
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }
    }
    const bytes = new Uint8Array(chunks.reduce((n, c) => n + c.length, 0));
    let at = 0;
    for (const c of chunks) {
      bytes.set(c, at);
      at += c.length;
    }
    return { bytes, url: response.url };
  } catch {
    if (stop.signal.aborted) return { why: `the server sent nothing for ${STALL / 1000} seconds` };
    return { why: "the server did not answer, or it does not let this page read the file (CORS)" };
  } finally {
    clearTimeout(timer);
  }
}

/** A segment of a URL's path, decoded; as it is when it is not percent-encoded UTF-8. */
function decode(segment) {
  try {
    return decodeURIComponent(segment);
  } catch {
    return segment;
  }
}

/**
 * The relative URL of the workspace path `path` from the directory whose names are `dir`, each name
 * percent-encoded, such as "../b/x.inc". Resolved against the address of the file in `dir`, it keeps
 * the server's own spelling of the names the two share.
 */
function relative(dir, path) {
  const names = path.split("/");
  let k = 0;
  while (k < dir.length && k < names.length - 1 && names[k] === dir[k]) k += 1;
  return "../".repeat(dir.length - k) + names.slice(k).map(encodeURIComponent).join("/");
}

/** The http or https URL a text names, resolved against the page; null when it names none. */
function parseUrl(text) {
  if (!text.trim()) return null;
  let url;
  try {
    url = new URL(text.trim(), location.href);
  } catch {
    return null;
  }
  return url.protocol === "http:" || url.protocol === "https:" ? url : null;
}

/** Resolve embed URLs exactly as File > Open URL and ?url= do. */
export function sourceFromUrl(text) {
  const url = parseUrl(text);
  if (!url) throw new Error("Give an http or https address, or one relative to this page.");
  return urlSource(url);
}

/** Warm the actual Worker/core before announcing that messages can be handled. */
export async function prepareEmbed() {
  const [response] = await Promise.all([
    fetch("about.json"),
    core.build({ entry: "/ready.sp", files: { "/ready.sp": "* Ready\n.end\n" }, svg: false }),
    embedStyle,
  ]);
  if (!response.ok) throw new Error(`The viewer version could not be read (${response.status}).`);
  return (await response.json()).version;
}

/** The extension of a path's file name, in lower case; "" when it has none. */
function extension(path) {
  const name = path.slice(path.lastIndexOf("/") + 1);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
}

const byPath = (a, b) => a.localeCompare(b, undefined, { numeric: true });

/**
 * The entry among `files`, by path, each shown relative to the directory `dir`. Any file can be the
 * netlist, whatever its name: the netlists are listed first, then the files mostly included, then
 * the others. Of files chosen or dropped, the entry is the only one not mostly included, or the only
 * file; when more than one could be, the user picks. In a folder, named by `folder` ("" for several
 * things dropped, a folder among them), the user always picks, with a filter. Null when the user
 * cancels.
 *
 * @param {Map<string, unknown>} files
 * @param {string} dir
 * @param {string | null} folder
 * @returns {Promise<string | null>}
 */
function chooseEntry(files, dir, folder) {
  const rank = (p) => (NETLISTS.includes(extension(p)) ? 0 : PARTS.includes(extension(p)) ? 1 : 2);
  const offered = [...files.keys()].sort((a, b) => rank(a) - rank(b) || byPath(a, b));
  if (folder === null) {
    const entries = offered.filter((p) => rank(p) !== 1);
    if (entries.length === 1) return Promise.resolve(entries[0]);
    if (offered.length === 1) return Promise.resolve(offered[0]);
  }
  let text = "Several of the files could be the netlist to open. Which one?";
  if (folder !== null) text = folder ? `The netlist to open in ${folder}:` : "The netlist to open:";
  return choose({
    title: folder === null ? "Open" : "Open Folder",
    text,
    items: offered.map((p) => ({ label: within(p, dir), value: p })),
    filter: folder !== null,
  });
}

/** Files chosen, or dropped without a folder: each at /<name> in the workspace. */
async function openFiles(files) {
  if (files.length === 0) return null;
  const all = new Map(files.map((f) => [`/${f.name}`, () => f]));
  const entry = await chooseEntry(all, "", null);
  return entry && filesSource(all, entry);
}

/** Pool files retain their directory structure, but only the current pool supplies siblings. */
function poolItems(files, folder = false) {
  return files.filter((f) => !folder || [...NETLISTS, ...PARTS].includes(extension(f.name))).map((f) => {
    const path = `/${f.webkitRelativePath || f.name}`;
    const read = () => f;
    return { path, name: path.slice(1), read, source: { ...filesSource(new Map([[path, read]]), path), fromPool: true } };
  });
}

let libraryIndex = null;
function publicPackages() {
  if (!libraryIndex) libraryIndex = (async () => {
    const config = await fetch("public-packages.json");
    const url = new URL(config.ok ? (await config.json()).url : DEFAULT_CATALOG, location.href).href;
    return (await readCatalog(url)).packages;
  })().then(packages => {
    window.dispatchEvent(new CustomEvent("public-package-status", { detail: "" }));
    return packages;
  }).catch((error) => {
    libraryIndex = null;
    window.dispatchEvent(new CustomEvent("public-package-status", { detail: `Public packages unavailable: ${error.message} Built-in standard packages remain available.` }));
    throw error;
  });
  return libraryIndex;
}

async function includeLibrary() {
  const catalog = await publicPackages();
  const packages = latestPackages(catalog);
  const selected = await choose({ title: "Include Public Package", text: "Choose a package to add to the file pool.",
    items: packages.map((p) => ({ label: `${p.name} ${p.version} — ${p.description}`, value: p })), filter: true });
  if (!selected) return [];
  return includePackage(selected);
}

async function includePackage(selected) {
  // Re-resolve against the validated catalog rather than trusting an old session descriptor.
  const packages = await publicPackages();
  const pkg = packages.find(p => p.name === selected.name && p.version === selected.version);
  if (!pkg) throw new Error(`Public package ${selected.name} ${selected.version} is unavailable.`);
  const text = decodeText(await readPublicPackage(pkg));
  const path = `/library/${selected.path}`;
  const read = () => new File([text], selected.name + ".rnl");
  return [{ path, name: `${selected.name} ${selected.version}`, read, source: { ...filesSource(new Map([[path, read]]), path), fromPool: true } }];
}

/** Ask for a URL, resolved against the page. */
async function openUrl() {
  const text = await ask({
    title: "Open URL",
    text: "The address of a netlist. The files it includes are read from the same server.",
    value: lastUrl,
    placeholder: "https://example.org/netlists/top.sp",
    action: "Open",
    check: (t) => (parseUrl(t) ? null : "Give an http or https address, or one relative to this page."),
  });
  if (text === null) return null;
  const url = parseUrl(text);
  lastUrl = url.href;
  return urlSource(url);
}

// The browser's file chooser, through an input kept in the page: one for files, one for a folder.
// They have ids for the tests: files set on one while no chooser is shown, as a test sets them
// (DOM.setFileInputFiles, or Playwright's setInputFiles), open as if they were chosen.

const fileInput = embed ? null : input("file-input", false);
const folderInput = embed ? null : input("folder-input", true);
const includeInput = embed ? null : input("include-input", false);
let waiting = null; // resolves the chooser shown with the files chosen

/** A hidden file input in the page, for files or, with `folder`, a folder. */
function input(id, folder) {
  const el = document.createElement("input");
  el.type = "file";
  el.id = id;
  el.multiple = true;
  el.webkitdirectory = folder;
  el.hidden = true;
  el.addEventListener("change", () => {
    const files = [...el.files];
    el.value = "";
    if (waiting) chosen(files);
    else if (!modal()) {
      if ((folder || id === "include-input") && included) included(poolItems(files, folder));
      else if (opened) openFiles(files).then((source) => source && opened(source));
    }
  });
  el.addEventListener("cancel", () => chosen([]));
  document.body.append(el);
  return el;
}

/** Show the browser's file chooser; the files chosen, none when it is cancelled. */
function chooseFiles(el) {
  chosen([]); // a browser that does not say when a chooser is cancelled leaves one waiting
  return new Promise((resolve) => {
    waiting = resolve;
    el.value = "";
    el.click();
  });
}

/** End the chooser that is waiting, if any, with the files chosen in it. */
function chosen(files) {
  const resolve = waiting;
  waiting = null;
  if (resolve) resolve(files);
}

// Files dropped on the page open; folders join the pool. A dropped file is read
// anew from its entry each time, so Reload reads it as it is now.

let over = 0; // how deep the files dragged over the page are, in the elements they entered

const carriesFiles = (e) => e.dataTransfer && [...e.dataTransfer.types].includes("Files");

function dragging(on) {
  over = on ? over + 1 : 0;
  document.body.classList.toggle("dropping", over > 0);
}

document.addEventListener("dragenter", (e) => {
  if (embed) return;
  if (carriesFiles(e)) dragging(true);
});
document.addEventListener("dragleave", (e) => {
  if (embed) return;
  if (!carriesFiles(e)) return;
  over = Math.max(0, over - 1);
  document.body.classList.toggle("dropping", over > 0);
});
document.addEventListener("dragover", (e) => {
  if (!carriesFiles(e)) return;
  e.preventDefault(); // else the browser shows the file instead of the page
  e.dataTransfer.dropEffect = embed || modal() ? "none" : "copy";
});
document.addEventListener("drop", (e) => {
  if (!carriesFiles(e)) return;
  e.preventDefault();
  if (embed) return;
  dragging(false);
  if (modal() || !opened) return;
  // the items can be read only while the event is dispatched
  const entries = [...e.dataTransfer.items]
    .filter((item) => item.kind === "file")
    .map((item) => (item.webkitGetAsEntry ? item.webkitGetAsEntry() : null));
  const files = [...e.dataTransfer.files];
  const open = opened;
  dropped(entries, files).then(
    (source) => source && open(source),
    (err) => inform({ title: "Open", paragraphs: [`The dropped files could not be read: ${err.message || err}`] }),
  );
});

/**
 * Files and folders dropped retain their `fullPath`: files at /<name> and folders at
 * /<folder>/<sub>/<name>. A folder adds the collection to the pool. Only files, as a browser
 * without entries gives them, open as with Open File.
 */
async function dropped(entries, files) {
  if (entries.length === 0 || !entries.every(Boolean)) return openFiles(files);
  const all = new Map();
  for (const entry of entries) await walk(entry, all);
  if (all.size === 0) return null;
  const folders = entries.filter((entry) => entry.isDirectory);
  if (folders.length > 0 && included) {
    included([...all].filter(([path]) => [...NETLISTS, ...PARTS].includes(extension(path))).map(([path, read]) =>
      ({ path, name: path.slice(1), read, source: { ...filesSource(new Map([[path, read]]), path), fromPool: true } })));
    return null;
  }
  const folder = folders.length === 0 ? null : entries.length === 1 ? folders[0].name : "";
  const entry = await chooseEntry(all, entries.length === 1 && folder ? `/${folder}` : "", folder);
  return entry && filesSource(all, entry);
}

/** Add the files of a dropped entry, a file or a folder, by their paths (`fullPath`). */
async function walk(entry, files) {
  if (entry.isFile) {
    files.set(entry.fullPath, () => new Promise((resolve, reject) => entry.file(resolve, reject)));
    return;
  }
  const reader = entry.createReader();
  for (;;) {
    const batch = await new Promise((resolve, reject) => reader.readEntries(resolve, reject));
    if (batch.length === 0) return;
    for (const child of batch) await walk(child, files);
  }
}

/** Hand the browser a text as a file to save under `name`, by a link it follows. */
function download(name, text) {
  const url = URL.createObjectURL(new Blob([text], { type: "text/plain;charset=utf-8" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.hidden = true;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10000);
}

/** The About box: what the viewer is, and the version and commit the site was built from. */
async function about() {
  let site = {};
  try {
    const response = await fetch("about.json", { cache: "no-cache" });
    if (response.ok) site = await response.json();
  } catch {
    // the box just names no version
  }
  const version = [site.version, site.commit && `(${site.commit})`].filter(Boolean).join(" ");
  const name = document.createElement("p");
  name.className = "dialog-name";
  name.textContent = version ? `RNL Viewer ${version}` : "RNL Viewer";
  await inform({
    title: "About RNL Viewer",
    paragraphs: [
      name,
      "Draws netlists with Rich Netlist (RNL) records. The text can be edited and its changes applied; " +
        "Save downloads the file shown, as no page can write a file where it was read from.",
      "The RNL core runs in this page as WebAssembly. Files opened from this computer are read here " +
        "and sent nowhere; a URL's files are fetched from its server.",
    ],
  });
}

/** @type {import("../shell.js").Shell} */
export default {
  platform: "web",
  newWindow(session, doc) {
    return openWebWindow(async () => {
      const files = {};
      for (const [path, read] of session.entry?.files || []) files[path] = decodeText(await (await read()).arrayBuffer());
      if (session.entry?.text !== undefined) files[session.entry.path || `/${session.entry.name}`] = session.entry.text;
      // The document contains applied text, never the pending editor buffer.
      if (doc) for (const f of doc.files) files[f.path.startsWith("/") || f.path.startsWith("<bundled>") ? f.path : joined(doc.path.slice(0, doc.path.lastIndexOf("/")), f.path)] = f.text;
      const pool = await Promise.all(session.pool.map(async (item) => ({ path: item.path, name: item.name, url: item.source.url,
        text: files[`/<pool>${item.path}`] ?? files[item.path] ?? decodeText(await (await item.read()).arrayBuffer()) })));
      const source = session.entry;
      return { ...session, files, pool, entry: source ? { name: source.name, path: source.path || `/${source.name}`, url: source.url,
        temporary: source.temporary, readonly: source.readonly, fromPool: source.fromPool } : null };
    });
  },
  async initialSession() {
    const session = await receiveWebSession();
    if (!session) return null;
    const files = new Map(Object.entries(session.files || {}).map(([path, text]) => [path, () => new File([text], path)]));
    session.pool = session.pool.map((item) => {
      const read = () => new File([item.text], item.name);
      return { path: item.path, name: item.name, read, source: sessionSource(new Map([[item.path, read]]),
        { path: item.path, name: item.name, url: item.url, fromPool: !item.url }) };
    });
    if (session.entry) session.entry = { ...sessionSource(files, session.entry), snapshot: session.files };
    return session;
  },
  embed,
  bundledFiles: async () => {
    const response = await fetch("libraries/bundled.json");
    if (!response.ok) throw new Error(`Built-in packages could not be listed (${response.status}).`);
    return response.json();
  },
  includeLibrary,
  publicPackages,
  includePackage,
  poolItem(source) {
    const path = source.path;
    return { path, name: source.name, source, read: async () => {
      const text = await source.read(path, false);
      if (text === null) throw new Error(`${source.name} could not be read. Open it again.`);
      return new File([text], source.name);
    } };
  },
  include(kind) {
    if (kind === "library") return includeLibrary();
    return chooseFiles(kind === "folder" ? folderInput : includeInput).then((files) => poolItems(files, kind === "folder"));
  },
  setPool(items) { poolFiles = new Map(items.filter((item) => !item.builtin).map((item) => [item.path, item.read])); },
  onInclude(handler) { included = handler; },
  nativeMenu: false,
  /** The URL that `?url=` names in the page's address, resolved against the page. */
  initial() {
    const given = new URLSearchParams(location.search).get("url");
    if (!given) return Promise.resolve(null);
    const url = parseUrl(given);
    if (!url) return Promise.reject(new Error(`The page cannot open ?url=${given}: it is not an http or https address.`));
    lastUrl = url.href;
    return Promise.resolve(urlSource(url));
  },
  pick(kind) {
    // the chooser is shown before anything is awaited, while the click or key that asked for it counts
    if (kind === "file") return chooseFiles(fileInput).then(openFiles);
    if (kind === "url") return openUrl();
    return Promise.resolve(null);
  },
  /**
   * Read the files the document needs, as SVG for the page to draw, and name the browser tab. A
   * source that holds its text, such as the clipboard's, can use the file pool for includes. The
   * texts of `edits`, by the paths the Document names the files by, are those of the files, not what
   * is read.
   */
  async build(source, edits, progress) {
    const again = built.has(source);
    built.add(source);
    try {
      progress?.("reading");
      const held = source.text !== undefined;
      const entry = held ? { path: source.path || `/${source.name}`, text: source.text } : await source.open(again);
      const dir = entry.path.slice(0, entry.path.lastIndexOf("/"));
      const files = {};
      // An independently opened source owns its relative paths. Put the pool in a separate
      // namespace so preloaded pool text cannot hide a source-relative file collect has yet to read.
      const poolPath = (path) => source.fromPool ? path : `/<pool>${path}`;
      for (const [path, read] of poolFiles) {
        files[poolPath(path)] = path === entry.path ? entry.text : decodeText(await (await read()).arrayBuffer());
      }
      files[entry.path] = entry.text;
      Object.assign(files, source.snapshot || {});
      for (const [path, text] of Object.entries(edits || {})) files[joined(dir, path)] = text;
      const request = { entry: entry.path, files, pool: [...poolFiles.keys()].map(poolPath), svg: true, primitives: false, lazy_unplaced: true };
      const read = held ? async () => null : (path) => {
        progress?.("reading");
        return path.startsWith("/<pool>/") ? Promise.resolve(null) : source.read(path, again);
      };
      const builder = { build: (request) => { progress?.("parsing"); return core.build(request); } };
      const { document: doc } = await collect(builder, request, read);
      if (!embed) document.title = `${doc.name} - RNL Viewer`;
      return doc;
    } catch (e) {
      throw e instanceof Error ? e.message : String(e);
    }
  },
  /** The drawings of the objects the last Document lists without a position, which the core keeps. */
  unplacedDrawings: (level, items, names) => core.drawings({ drawings: { level, items, names } }),
  /** Download the files: the web cannot write them where they were read from (decision 0011). */
  async save(source, files) {
    for (const f of files) download(f.name.split("/").pop(), f.text);
    return files.length === 1 ? `Downloaded ${files[0].name.split("/").pop()}.` : `Downloaded ${files.length} files.`;
  },
  /** The browser asks before a page with changes not saved is left. */
  setDirty(dirty) {
    window.onbeforeunload = dirty
      ? (e) => {
          e.preventDefault();
          e.returnValue = "";
        }
      : null;
  },
  setCheck() {}, // the page's own menu bar shows the check marks
  onCommand() {}, // and runs its commands itself
  onOpen(handler) {
    opened = handler;
  },
  about,
  /** Show a web page in a new tab, with no hold on this one. */
  browse(url) {
    window.open(url, "_blank", "noopener");
  },
};
