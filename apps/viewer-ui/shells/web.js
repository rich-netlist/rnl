// The web shell: the page in a browser, served with the core next to it as core/ (apps/web). It opens
// files the user chooses or drops, a folder, or a URL; reads only the files a document needs, as
// collect in the core's client asks for them; and builds the Document with the core as WebAssembly in
// a module Worker, one for the page's lifetime. The page shows its own menu bar.

import { collect, decodeText, startCore } from "../core/client.js";
import { ask, choose, inform, modal } from "../dialog.js";

/** The extensions of netlists that are opened on their own, listed first as the file to open. */
const NETLISTS = ["sp", "spi", "spice", "cir", "ckt", "net", "scs"];
/** The extensions of files mostly included or used by a netlist, listed after the netlists. */
const PARTS = ["inc", "lib", "rnl"];
/** How long a server may send nothing, before or during its answer, until a fetch is given up (ms). */
const STALL = 30000;

const core = startCore();
const built = new WeakSet(); // the sources built before: building one again reads it again
let opened = null; // the page's handler for sources opened without asking, such as dropped files
let lastUrl = ""; // the URL opened last, which Open URL offers again

/**
 * A source of this shell: files chosen or dropped, a folder, or a URL. The page's sources that hold
 * their text (shell.js) need none of this.
 *
 * @typedef {object} WebSource
 * @property {string} name How the page names it: the entry's file name, its path in a folder, or the URL.
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
    name: entry.slice(1),
    async open() {
      return { path: entry, text: await read(entry) };
    },
    read,
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
  let dir = []; // the entry's directory in the workspace, by its names
  return {
    name: url.href,
    async open(again) {
      const got = await fetchBytes(url.href, again);
      if (!got.bytes) throw new Error(`${url.href} could not be fetched: ${got.why}.`);
      served = new URL(got.url || url.href);
      const path = serverPath(served);
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

/** The files of a folder, each at /<folder>/<sub>/<name> in the workspace. */
async function openFolder(files) {
  if (files.length === 0) return null;
  const all = new Map(files.map((f) => [`/${f.webkitRelativePath || f.name}`, () => f]));
  const folder = (files[0].webkitRelativePath || "").split("/")[0];
  const entry = await chooseEntry(all, `/${folder}`, folder);
  return entry && filesSource(all, entry);
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

const fileInput = input("file-input", false);
const folderInput = input("folder-input", true);
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
    else if (opened && !modal()) (folder ? openFolder : openFiles)(files).then((source) => source && opened(source));
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

// Files or a folder dropped on the page open as with Open and Open Folder. A dropped file is read
// anew from its entry each time, so Reload reads it as it is now.

let over = 0; // how deep the files dragged over the page are, in the elements they entered

const carriesFiles = (e) => e.dataTransfer && [...e.dataTransfer.types].includes("Files");

function dragging(on) {
  over = on ? over + 1 : 0;
  document.body.classList.toggle("dropping", over > 0);
}

document.addEventListener("dragenter", (e) => {
  if (carriesFiles(e)) dragging(true);
});
document.addEventListener("dragleave", (e) => {
  if (!carriesFiles(e)) return;
  over = Math.max(0, over - 1);
  document.body.classList.toggle("dropping", over > 0);
});
document.addEventListener("dragover", (e) => {
  if (!carriesFiles(e)) return;
  e.preventDefault(); // else the browser shows the file instead of the page
  e.dataTransfer.dropEffect = modal() ? "none" : "copy";
});
document.addEventListener("drop", (e) => {
  if (!carriesFiles(e)) return;
  e.preventDefault();
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
 * The source of the files and folders dropped, each by its `fullPath`, as Open and Open Folder put
 * them: files at /<name> and folders at /<folder>/<sub>/<name>. With a folder among them the user
 * picks the netlist. Only files, as a browser without entries gives them, open as with Open.
 */
async function dropped(entries, files) {
  if (entries.length === 0 || !entries.every(Boolean)) return openFiles(files);
  const all = new Map();
  for (const entry of entries) await walk(entry, all);
  if (all.size === 0) return null;
  const folders = entries.filter((entry) => entry.isDirectory);
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
    if (kind === "folder") return chooseFiles(folderInput).then(openFolder);
    if (kind === "url") return openUrl();
    return Promise.resolve(null);
  },
  /**
   * Read the files the document needs, as SVG for the page to draw, and name the browser tab. A
   * source that holds its text, such as the clipboard's, is the only file of its workspace. The
   * texts of `edits`, by the paths the Document names the files by, are those of the files, not what
   * is read.
   */
  async build(source, edits) {
    const again = built.has(source);
    built.add(source);
    try {
      const held = source.text !== undefined;
      const entry = held ? { path: `/${source.name}`, text: source.text } : await source.open(again);
      const dir = entry.path.slice(0, entry.path.lastIndexOf("/"));
      const files = { [entry.path]: entry.text };
      for (const [path, text] of Object.entries(edits || {})) files[joined(dir, path)] = text;
      const request = { entry: entry.path, files, svg: true, primitives: false };
      const read = held ? async () => null : (path) => source.read(path, again);
      const { document: doc } = await collect(core, request, read);
      document.title = `${doc.name} - RNL Viewer`;
      return doc;
    } catch (e) {
      throw e instanceof Error ? e.message : String(e);
    }
  },
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
