import "./local-key.js"; // (first: on the local program's page, the key goes with every request below; elsewhere it does nothing)
import { append, m, setAttr } from "../i18n.js";
// The web shell: the page in a browser, served with the core next to it as core/ (apps/web). It opens
// files the user chooses or drops, a folder, or a URL; reads only the files a document needs, as
// collect in the core's client asks for them; and builds the Document with the core as WebAssembly in
// module Workers. Each candidate keeps its own lazy drawings until the page accepts it. The page's own
// opens read the file shown alone, in the context of its entry (`reader`, which file-reading.js reads with):
// the outline of the whole document, then the reading of that one file (collectOutline, collectFile). What is
// kept of them, and when, is the page's (file-reading.js); nothing of it is in the browser's storage. Those
// requests go through the reading cache of the Worker's core, which keeps in memory the host document read last,
// so that the reading of the file does not read again what the outline read (`cache`). The first answer of the file
// to show is the smaller one: without the texts, which the page has, the spans and the geometry hints, which the
// Worker gives from what it kept of that reading right after (`restOf`); the page draws before they come. An
// embedded viewer, and the viewer API, read the whole document (`build`).

import { collect, collectFile, collectOutline, coreVersion, decodeText, fileResultFormat, startCore } from "../core/client.js";
import { removeOldStore } from "../result-cache.js";
import { libraryStamps, outlinePaths, requestPath, textHash } from "../file-results.js";
import { loadManifest, readManifest } from "../manifest.js";
import { ask, choose, inform, modal } from "../dialog.js";
import { libraryZip } from "../zip.js";
import { chooseEntry, sayListedInPart } from "../incoming.js";
import { openWebWindow, receiveWebSession } from "../window-session.js";
import { load } from "../util.js";
import { sourcePath } from "../source-path.js";
import { entrySource } from "../visit-source.js";
import { chooseFormat, fileFormats } from "../formats.js";
import { DEFAULT_CATALOG, readCatalog, readPublicPackage, latestPackages } from "../public-packages.js";

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
  append(document.head, style);
}

/**
 * The solver a development server runs for the page (apps/web/solver-service.mjs, `serve.mjs --rnl PATH`): what it
 * says of itself, or null. Asked only on this computer's own address, and only of the server that served the page,
 * so that the published site, which has none, asks nothing; an address in the page's query is never taken.
 */
async function localSolver() {
  if (embed || !["127.0.0.1", "localhost", "[::1]"].includes(location.hostname)) return null;
  try {
    const response = await fetch(new URL("rnl-solver", location.href), { cache: "no-store" });
    const info = response.ok ? await response.json() : null;
    return info && info.service === "rnl-solver" && info.version ? info : null;
  } catch {
    return null;
  }
}
const solver = await localSolver();

/**
 * The solver of the page's own, when this build has it (apps/web/solver-worker.js, bindings/solver-wasm): what
 * `solver.json` says of it, or null. A page that is not served by a development server solves in the browser, in a
 * Worker started by the first solve; an embed does not solve.
 */
async function browserSolver() {
  if (embed || solver) return null;
  try {
    const response = await fetch(new URL("solver.json", location.href));
    const info = response.ok ? await response.json() : null;
    return info && typeof info.worker === "string" ? info : null;
  } catch {
    return null;
  }
}
const inBrowser = await browserSolver();
let lastBuilt = null; // the Document of a build the page accepted last: what a solve is of, not a preview of one
// the last solve, in the browser or by a solver service: the request of the build it solved, with the texts applied over
// its files, and the texts the solve wrote, by the paths of the request (Save Solved Copy in the browser, and Export
// Image of the preview)
let lastSolved = null;
/**
 * A solve's answer, kept as the last solve: the request of the build it solved, with the files it read (`read`, the texts
 * applied over them), and the texts it wrote, by the paths of the request (`answer.files`), which the solver in the browser,
 * the local program's service and the development server's (`rnl preview --request`) give alike.
 */
function solvedBy(request, read, answer) {
  lastSolved = { request: { ...request, files: read.files }, written: answer.files ?? {} };
  return answer;
}
let solving = null; // the AbortController of the solve under way
let solverWorker = null; // the Worker of the solver in the browser, once a call has started it
const solverCalls = new Map(); // the calls the Worker has not answered, by id: { resolve, reject, onEvent }
let solverCallId = 0;
/** Stop the solver's Worker, and with it the calls under way; the next call starts another. */
function stopSolver(why = "the solver was stopped") {
  solverWorker?.terminate();
  solverWorker = null;
  for (const call of solverCalls.values()) call.reject(why);
  solverCalls.clear();
}
/**
 * A call to the solver's Worker, which a first call starts: `message` is `{ request }` (a solve) or `{ table: true }`;
 * the events of a solve go to `onEvent` as they happen; what it gives is the answer of a solve or the table. Each
 * message has the size of the solver's module that `solver.json` says, and how the Worker's first load of it goes is
 * told to the page's handler of the solve's events, as the event `load` (apps/web/solver-worker.js), whichever call
 * made it load.
 */
function callSolver(info, message, onEvent) {
  return new Promise((resolve, reject) => {
    if (!solverWorker) {
      const worker = new Worker(new URL(info.worker, location.href), { type: "module" });
      worker.onmessage = ({ data }) => {
        if (data.load) return solveProgress?.({ event: "load", ...data.load });
        const call = solverCalls.get(data.id);
        if (!call) return;
        if (data.event) return call.onEvent?.(data.event);
        solverCalls.delete(data.id);
        if (data.error !== undefined) call.reject(data.error);
        else call.resolve(data.table ?? data.answer);
      };
      worker.onerror = (e) => stopSolver(`the solver could not be started${e.message ? `: ${e.message}` : ""}`);
      solverWorker = worker;
    }
    const id = ++solverCallId;
    solverCalls.set(id, { resolve, reject, onEvent });
    solverWorker.postMessage({ id, ...message, size: info.bytes });
  });
}
let solveProgress = null; // the page's handler of the events of the solve under way

const WARMUP = { entry: "/ready.sp", files: { "/ready.sp": "* Ready\n.end\n" }, svg: false };
function warmCore() {
  const worker = startCore();
  const ready = worker.build(WARMUP);
  ready.catch(() => {}); // prepareViewer or the next build reports initialization failures.
  return { worker, ready };
}

let core = null; // only the displayed document's lazy drawing provider
let spare = warmCore();
const pendingCores = new Set(); // candidates building or waiting to be accepted
// the request each Document was built from, with the files collected: Export Image builds it again with overlays
const builtRequests = new WeakMap();
const candidates = new WeakMap();
function replenishCore() {
  if (!spare && pendingCores.size === 0) spare = warmCore();
}
const built = new WeakSet(); // the sources built before: building one again reads it again
let opened = null; // the page's handler for sources opened without asking, such as dropped files
let included = null;
let poolFiles = new Map();
let poolSources = new Map();
let poolTexts = new Map();
let poolReady = Promise.resolve();
let librarySnapshot = { texts: poolTexts, epoch: 0 };
const indexedLibraries = new WeakMap();

// Candidate workers retain independent lazy drawings and indexes. Synchronize only changed
// library files when a worker is reused; a reload invalidates both workers on their next use.
async function indexLibrary(worker, snapshot) {
  const previous = indexedLibraries.get(worker);
  if (previous === snapshot) return [];
  const refresh = previous ? [...snapshot.texts].filter(([p, text]) =>
    previous.texts.has(p) && previous.texts.get(p) !== text).map(([p]) => p) : [];
  const errors = await worker.library({ library: { reload: previous?.epoch !== snapshot.epoch, refresh },
    pool: [...snapshot.texts.keys()], files: Object.fromEntries(snapshot.texts), formats: requestFormats(servedFiles.origin) });
  indexedLibraries.set(worker, snapshot);
  return errors;
}
// the store of results earlier versions of the page kept in the browser's storage is deleted; an embed touches none
if (!embed) removeOldStore();
// the Documents given from the readings kept (file-reading.js): the core shown holds no drawings of theirs until it
// reads them again
const kept = new WeakSet();
const primed = new WeakSet();
// the stamps of each Library snapshot (file-results.js libraryStamps), made once
const stamped = new WeakMap();
/** The options of a request that change a reading, as the key of one kept has them (file-reading.js). */
const READING_OPTIONS = ["svg", "primitives", "flylines", "hints", "lazy_unplaced", "auto_include", "library_prefix", "formats", "dialect"];
// The files the core asked for that readers of an entry found, by the entry's path. A reader reads them before its first
// request, so that a reading is not made twice: once without them, which the core answers by naming them, and once with
// them (the package files a header names by `path`, which no outline lists). Only the paths are kept: each reader reads
// the files itself, as they are now.
const askedFor = new Map();
// The reading settings of the document the reading cache of each Worker's core holds, by the Worker: the outline and
// file requests of a reader go through the cache (`"cache": true`), and the first of them on a Worker whose cache holds
// a document of another entry, or read with other settings, drops it first (`"forget": true`). The core takes what it
// holds only when the files answer as they did, and then answers as it does without; this only frees the memory early.
// A test page turns the cache off with `globalThis.rnlReadingCache = false`, to compare.
const cachedFor = new WeakMap();
const readingCache = () => globalThis.rnlReadingCache !== false;
let lastUrl = ""; // the URL opened last, which Open URL offers again
// The files of the document shown that a server served, by their workspace paths, and its origin: the
// format chosen for one of them is kept by its address, as the page can read it again; a file chosen
// from the computer is named by its name alone, so its choice lasts only while the page is open.
let servedFiles = { origin: null, paths: new Set() };

/**
 * The format choices to give a request, by workspace path: those kept by the address of a file of
 * `origin` become its path on that server, and each also goes under the name of the pool, as
 * `poolPath` gives it, since a Library file is read under either.
 */
function requestFormats(origin, poolPath = (path) => path) {
  const out = {};
  for (const [key, name] of Object.entries(fileFormats())) {
    let path = key;
    if (/^https?:/i.test(key)) {
      const url = new URL(key);
      if (url.origin !== origin) continue;
      path = serverPath(url);
    }
    out[path] = name;
    out[poolPath(path)] = name;
  }
  return out;
}

/** The files of a Document of `source` a server served, by their workspace paths, and its origin (`servedFiles`). */
const served = (source, doc) => source.url
  ? { origin: new URL(source.url).origin, paths: new Set(doc.files.filter((f) => f.format).map((f) => sourcePath(doc, f)).filter((p) => !p.startsWith("/<"))) }
  : { origin: null, paths: new Set() };

/**
 * What a reading of `source`, the entry's source of a view, asks the core for, and how the files it asks for are read:
 * the request, with the Library's files under their names in it, a manifest's, `edits` over the texts and the API
 * packages; `read(path)`, which reads a file the core asks for; the Library snapshot the request has. A file at an
 * address is asked for before the Library is ready, which takes longer than it does on a slow network.
 */
async function prepare(source, edits, suppliedPackages, progress, { bind, constraints } = {}) {
  source = entrySource(source);
  const again = built.has(source);
  built.add(source);
  // an error of the file asked for early is told where it is awaited
  const reading = source.prefetch && source.text === undefined ? source.open(again, progress) : null;
  reading?.catch(() => {});
  await poolReady;
  await bundledReady;
  const library = librarySnapshot;
  progress?.("reading");
  const held = source.text !== undefined;
  const entry = held ? { path: source.path || `/${source.name}`, text: source.text } : await (reading ?? source.open(again, progress));
  const dir = entry.path.slice(0, entry.path.lastIndexOf("/"));
  const files = {};
  // An independently opened source owns its relative paths. Put the pool in a separate
  // namespace so preloaded pool text cannot hide a source-relative file collect has yet to read.
  const poolPath = (path) => source.fromPool ? path : `/<pool>${path}`;
  for (const [path, text] of library.texts) {
    files[poolPath(path)] = path === entry.path ? entry.text : text;
  }
  files[entry.path] = entry.text;
  Object.assign(files, source.snapshot || {});
  for (const [path, text] of Object.entries(edits || {})) files[joined(dir, path)] = text;
  // a manifest's pool first, in its order, then the Library's files; PATH's are searched folder by folder after the pool,
  // as the desktop and the CLI search them: given as the folders, their files read but not in the pool
  const pool = [...(source.pool || []), ...[...library.texts.keys()].filter((path) => !library.path?.files.has(path)).map(poolPath)];
  const searched = (library.path?.dirs || []).map(poolPath);
  // the public packages bundled, for packages only: read, but not in the pool, and searched after PATH
  for (const [path, text] of Object.entries(bundled.files)) files[path] ??= text;
  // the files of a manifest are indexed as the Library's are, for Auto Include, from the build
  // that opens it on: the Library has them only after it (pool.js)
  for (const [path, text] of Object.entries(source.external?.files || {})) {
    if (library.texts.has(path) || files[poolPath(path)] !== undefined) continue;
    files[poolPath(path)] = text;
    pool.push(poolPath(path));
  }
  // flylines and the layout hints are asked for, not left to the core's default: View > Show Flylines hides the flylines (flylines.js), and the problems list has the hints (problems.js)
  const request = { entry: entry.path, files, pool, package_dirs: [...searched, ...bundled.dirs], search_paths: searched, svg: true, primitives: false, flylines: true, hints: true, lazy_unplaced: true,
    auto_include: load("auto-include", true) === true, library_prefix: source.fromPool ? "" : "/<pool>",
    formats: requestFormats(source.url ? new URL(source.url).origin : null, poolPath),
    library_edits: Object.fromEntries(Object.entries(edits || {}).map(([path, text]) => {
      const full = joined(dir, path);
      return [full.replace(/^\/<pool>(?=\/)/, ""), text];
    })) };
  // the parameters of a subcircuit to build with, for showing (params-dialog.js)
  if (bind) request.bind = bind;
  // the constraints of each level, for the Solve view (constraints.js)
  if (constraints) request.constraints = true;
  // API packages are request-local until the newest request commits them to Library.
  for (const item of suppliedPackages || []) {
    const path = `/<api>${item.path}`;
    request.files[path] = decodeText(await (await item.read()).arrayBuffer());
    request.pool.push(path);
  }
  const read = held ? async () => null : (path) => {
    progress?.("reading");
    return path.startsWith("/<pool>/") ? Promise.resolve(null) : source.read(path, again);
  };
  return { source, entry, request, read, library };
}

/**
 * What a solve reads, as the last build read it (crates/rnl-view RequestFiles): its files with the texts of `edits`
 * applied over them, its Library (`pool`), the folders it searched, Auto Include and the Library's edits; so that a solve
 * finds what the build found, an include from the Library and a subcircuit that Auto Include added among them.
 */
function solveFiles(request, edits) {
  const dir = request.entry.slice(0, request.entry.lastIndexOf("/"));
  const files = { ...request.files };
  const libraryEdits = { ...(request.library_edits || {}) };
  for (const [path, text] of Object.entries(edits || {})) {
    const full = joined(dir, path);
    files[full] = text;
    // (as the build names them: prepare)
    libraryEdits[full.replace(/^\/<pool>(?=\/)/, "")] = text;
  }
  return {
    entry: request.entry, files, pool: request.pool ?? [], package_dirs: request.package_dirs ?? [], search_paths: request.search_paths ?? [],
    auto_include: request.auto_include === true, library_prefix: request.library_prefix ?? "", library_edits: libraryEdits,
    formats: request.formats ?? {},
  };
}

// The public packages a shell of the page has bundled (the local program's): their files, and their directories, which
// are searched for packages only, after PATH, as the desktop searches its own (apps/desktop/src/public_packages.rs)
let bundled = { files: {}, dirs: [] };
let bundledReady = Promise.resolve();

/** The public packages bundled, once `loading` gives them: `{ files, dirs }`; none when it fails. */
export function useBundledPackages(loading) {
  bundledReady = loading.then((given) => { bundled = given; }, () => {});
}

/** A core to ask what needs no document: the one shown, or the warm spare. */
async function someCore() {
  if (core) return core;
  spare ??= warmCore();
  await spare.ready;
  return spare.worker;
}

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

/**
 * Files by an external manifest (manifest.js), such as the RNL gallery writes, or a manifest of
 * links: its files, fetched and checked against their SHA-256, under /<id>/ in the workspace; its
 * pool, in its order; read only. Each open reads the manifest (at `url`, or `json` as a file gave
 * it) and its files again, so Reload (F5) does too. `external` holds what was read, for the Library,
 * the source bar and the level (manifest-bar.js).
 *
 * @param {URL | null} url
 * @param {object | null} json
 */
function manifestSource(url, json = null, name = null) {
  const source = {
    name: url?.href ?? name,
    path: `/${url ? url.pathname.split("/").pop() || "manifest.json" : name}`,
    readonly: true,
    manifest: url?.href ?? null,
    external: null,
    snapshot: {},
    pool: [],
    async open(_again, progress) {
      const onProgress = (done, total) => progress?.({ stage: "fetching", done, total });
      const read = url ? await readManifest(url.href, { onProgress }) : await loadManifest(json, { onProgress });
      // the formats it gives are chosen as the user chooses them, once: a later choice, or Reload, keeps the user's
      if (!source.external) for (const [path, format] of Object.entries(read.formats)) await chooseFormat(path, format, { session: true });
      Object.assign(source, { external: { ...read, manifest: url?.href ?? null }, snapshot: read.files, pool: read.pool, path: read.entry,
        name: read.entry.slice(read.id.length + 2) });
      return { path: read.entry, text: read.files[read.entry] };
    },
    read: async (path) => source.snapshot[path] ?? null,
  };
  return source;
}

/** Another file of an external manifest, as cross-file navigation opens it: read only, from what was read. */
function externalFile(external, path) {
  return {
    name: path.slice(external.id.length + 2), path, readonly: true, manifest: external.manifest, external,
    snapshot: external.files, pool: external.pool,
    async open() {
      if (external.files[path] === undefined) throw new Error(`${path.slice(1)} is not among the manifest's files.`);
      return { path, text: external.files[path] };
    },
    read: async (file) => external.files[file] ?? null,
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
function urlSource(url, cached = null) {
  let served = url; // where the entry was served from when it was last read
  let dir = serverPath(url).split("/").slice(0, -1); // also supports transferred sources read before open
  return {
    files: cached?.files,
    get url() { return served.href; },
    // read at once, while the Library is indexed and the core loads, not when they are done (build)
    prefetch: true,
    name: url.href,
    path: serverPath(url),
    /** The address of a file of this source by its workspace path: the entry's, or one it includes. */
    address(path) {
      if (!path.startsWith("/")) return null;
      return path === this.path ? served.href : new URL(relative(dir, path), served).href;
    },
    async open(again) {
      const got = await fetchBytes(url.href, again);
      if (!got.bytes) {
        const path = serverPath(url), text = await cached?.read(path);
        if (text != null) {
          served = url;
          this.path = path;
          dir = path.split('/').slice(0, -1);
          return { path, text };
        }
        throw new Error(`${url.href} could not be fetched: ${got.why}.`);
      }
      served = new URL(got.url || url.href);
      const path = serverPath(served);
      this.path = path;
      dir = path.split("/").slice(0, -1);
      return { path, text: decodeText(got.bytes) };
    },
    async read(path, again) {
      if (!path.startsWith("/")) return null; // such as C:/models/x.lib
      const got = await fetchBytes(new URL(relative(dir, path), served).href, again);
      return got.bytes ? decodeText(got.bytes) : await cached?.read(path) ?? null;
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
  const url = new URL(href);
  const library = new URL("libraries/", location.href);
  const legacy = url.origin === library.origin && url.pathname.startsWith(library.pathname)
    ? decode(url.pathname.slice(library.pathname.length)) : null;
  if (legacy && !legacy.startsWith("standard/") && legacy.endsWith(".rnl")) {
    try {
      const pkg = (await publicPackages()).find(p => p.path === legacy);
      if (!pkg) return { why: `No public package matches the old address: ${href}` };
      return { bytes: new Uint8Array(await readPublicPackage(pkg)), url: href };
    } catch (error) {
      return { why: error.message || String(error) };
    }
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
    if (stop.signal.aborted) return { why: `the server did not respond for ${STALL / 1000} seconds` };
    return { why: "the server did not respond, or blocked cross-origin access (CORS)" };
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
  if (!url) throw new Error(m("Give an http or https address, or one relative to this page."));
  return urlSource(url);
}

/** The files of a manifest at an http or https address, or one relative to this page. */
export function sourceFromManifest(text) {
  const url = parseUrl(text);
  if (!url) throw new Error(m("Give an http or https address, or one relative to this page."));
  return manifestSource(url);
}

/** The source of a manifest a file gave (File > Load Manifest), named `name`. */
export const sourceFromManifestJson = (json, name) => manifestSource(null, json, name);

/** Warm the actual Worker/core before announcing that messages can be handled. */
export async function prepareViewer() {
  const prepared = spare ??= warmCore();
  const [response] = await Promise.all([
    fetch("about.json"),
    prepared.ready,
    embedStyle,
  ]);
  if (!response.ok) throw new Error(`The viewer version could not be read (${response.status}).`);
  return (await response.json()).version;
}


/** Files chosen, or dropped without a folder: each at /<name> in the workspace. */
async function openFiles(files) {
  if (files.length === 0) return null;
  const all = new Map(files.map((f) => [`/${f.name}`, () => f]));
  const entry = await chooseEntry(all.keys(), { label: (p) => within(p, "") });
  return entry && filesSource(all, entry);
}

/** Pool files retain their directory structure, but only the current pool supplies siblings; a folder's, those the viewer lists. */
async function poolItems(files, folder = false) {
  return (folder ? await listedFolders(files, (f) => f.webkitRelativePath || f.name) : files).map((f) => {
    const path = `/${f.webkitRelativePath || f.name}`;
    const read = () => f;
    return { path, name: path.slice(1), read, source: { ...filesSource(new Map([[path, read]]), path), fromPool: true } };
  });
}

/**
 * The items of a folder chosen or dropped that join the Library, as every viewer lists a folder included: the core lists
 * them from their paths (`pathOf`, whose first part is the folder included, or a file given alone), as the desktop app
 * and the local program list a folder on disk (rnl_view::folder). A folder with more files than are listed is included
 * in part, which is said, with its name and how many files a folder brings in.
 */
async function listedFolders(items, pathOf) {
  const byPath = new Map(items.map((item) => [pathOf(item), item]));
  const listed = await (await someCore()).folders([...byPath.keys()]);
  sayListedInPart(listed);
  return listed.files.map((path) => byPath.get(path));
}

let libraryIndex = null;
const PUBLIC_TIMEOUT = 10000;
async function publicRequest(read, signal) {
  const stop = new AbortController();
  const abort = () => stop.abort();
  if (signal?.aborted) abort();
  else signal?.addEventListener('abort', abort, { once: true });
  const timer = setTimeout(abort, PUBLIC_TIMEOUT);
  try { return await read(url => fetch(url, { signal: stop.signal })); }
  catch (error) {
    if (stop.signal.aborted && !signal?.aborted) throw new Error(m('Public package request timed out after 10 seconds.'));
    throw error;
  } finally {
    clearTimeout(timer);
    signal?.removeEventListener('abort', abort);
  }
}
function publicPackages() {
  if (!libraryIndex) libraryIndex = publicRequest(async fetcher => {
    const config = await fetcher("public-packages.json");
    const url = new URL(config.ok ? (await config.json()).url : DEFAULT_CATALOG, location.href).href;
    return (await readCatalog(url, fetcher)).packages;
  }).then(packages => {
    window.dispatchEvent(new CustomEvent("public-package-status", { detail: "" }));
    return packages;
  }).catch((error) => {
    libraryIndex = null;
    window.dispatchEvent(new CustomEvent("public-package-status", { detail: m`Public packages unavailable (${error.message}). Built-in standard packages remain available.` }));
    throw error;
  });
  return libraryIndex;
}

async function includeLibrary() {
  const catalog = await publicPackages();
  const packages = latestPackages(catalog);
  const selected = await choose({ title: m("Include Public Package"), text: m("Choose a package to add to the Library."),
    items: packages.map((p) => ({ label: `${p.name} ${p.version} — ${p.description}`, value: p })), filter: true });
  if (!selected) return [];
  return includePackage(selected);
}

async function includePackage(selected, signal) {
  // Re-resolve against the validated catalog rather than trusting an old session descriptor.
  const packages = await publicPackages();
  const pkg = packages.find(p => p.name === selected.name && p.version === selected.version);
  if (!pkg) throw new Error(`Public package ${selected.name} ${selected.version} is unavailable.`);
  const text = decodeText(await publicRequest(fetcher => readPublicPackage(pkg, fetcher), signal));
  const path = `/library/${pkg.path}`;
  const read = () => new File([text], pkg.name + ".rnl");
  return [{ path, name: `${pkg.name} ${pkg.version}`, url: pkg.url, read, source: { ...filesSource(new Map([[path, read]]), path), fromPool: true } }];
}

/** Ask for a URL, resolved against the page. */
async function openUrl() {
  const text = await ask({
    title: m("Open URL"),
    text: m("The address of a netlist. The files it includes are read from the same server."),
    value: lastUrl,
    placeholder: "https://example.org/netlists/top.sp",
    action: "Open",
    check: (t) => (parseUrl(t) ? null : m("Give an http or https address, or one relative to this page.")),
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
// a project's archive, and a manifest of links: one file each, opened only through their commands
const projectInput = embed ? null : input("project-input", false, ".zip,application/zip");
const manifestInput = embed ? null : input("manifest-input", false, ".json,application/json");
let waiting = null; // resolves the chooser shown with the files chosen

/** A hidden file input in the page, for files or, with `folder`, a folder; with `accept`, one file of those types. */
function input(id, folder, accept = null) {
  const el = document.createElement("input");
  el.type = "file";
  el.id = id;
  el.multiple = !accept;
  if (accept) el.accept = accept;
  el.webkitdirectory = folder;
  el.hidden = true;
  el.addEventListener("change", () => {
    const files = [...el.files];
    el.value = "";
    if (waiting) chosen(files);
    else if (!modal() && !accept) {
      if ((folder || id === "include-input") && included) poolItems(files, folder).then(included);
      else if (opened) openFiles(files).then((source) => source && opened(source));
    }
  });
  el.addEventListener("cancel", () => chosen([]));
  append(document.body, el);
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
    (err) => inform({ title: m("Open"), paragraphs: [m`The dropped files could not be read: ${err.message || err}`] }),
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
    included((await listedFolders([...all], ([path]) => path)).map(([path, read]) =>
      ({ path, name: path.slice(1), read, source: { ...filesSource(new Map([[path, read]]), path), fromPool: true } })));
    return null;
  }
  const folder = folders.length === 0 ? null : entries.length === 1 ? folders[0].name : "";
  const dir = entries.length === 1 && folder ? `/${folder}` : "";
  const entry = await chooseEntry(all.keys(), { folder, label: (p) => within(p, dir) });
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
function download(name, content) {
  const blob = content instanceof Blob ? content : new Blob([content], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.hidden = true;
  append(document.body, a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10000);
}

/** The version and commit the site was built from, and the core's version (about.json); none where it cannot be read. */
async function buildInfo() {
  let site = {};
  try {
    const response = await fetch("about.json", { cache: "no-cache" });
    if (response.ok) site = await response.json();
  } catch {
    // the box just names no version
  }
  return { viewer: site.version ?? null, core: site.coreVersion ?? null, commit: site.commit ?? null, corePage: true };
}

/**
 * The source of the file at `full`, as the shell names it, where the file `f` (its name and, for a built-in, its text) is read from:
 * the pool, a server, an external folder or the host. The file is read again, with no compiled include closure.
 */
function sourceAt(source, full, f) {
  const path = full.replace(/^\/<pool>(?=\/)/, '');
  if (full !== path || source.fromPool && poolSources.has(path)) {
    return poolSources.get(path) || { ...filesSource(poolFiles, path), fromPool: true };
  }
  if (full.startsWith('<')) return { path, name: f.name, text: f.text, readonly: true };
  if (source.external) return externalFile(source.external, path);
  // A reader supplies available bytes, never a previously compiled include closure.
  if (source.url) {
    const url = new URL(relative(source.path.split('/').slice(0, -1), path), source.url);
    // Re-read like Open URL, with the session's local bytes available if the server is gone.
    return urlSource(url, source.files ? filesSource(source.files, path) : null);
  }
  const read = source.read || (async () => null);
  return { path, name: path.slice(1), files: source.files, fromPool: source.fromPool,
    read, async open(again) {
      const text = await read(path, again);
      if (text === null) throw new Error(`${path} could not be read. Open it again.`);
      return { path, text };
    } };
}

/** @type {import("../shell.js").Shell} */
export default {
  platform: "web",
  // the core runs in this page, as WebAssembly (the local program's page too): it is given the texts of built-in files
  coreInPage: true,
  // the page's opens read the file shown alone, in the context of its entry (per-file loading); an embed reads its document whole
  perFile: !embed,
  // a file of an earlier version of the language: the site's old web viewer, on the same circuit (old-version.js)
  oldViewer: "page",
  // the core's parse formats, the one it detects for a Library file the document shown does not read,
  // and the key a choice is kept by: the address of a file a server served, else its path
  formats: async () => (await (await someCore()).formats({ formats: {} })).list,
  async detectFormat(path) {
    await poolReady;
    const text = poolTexts.get(path);
    if (text === undefined) throw new Error(`${path} is not in the Library`);
    return (await (await someCore()).formats({ formats: { detect: { [path]: text } } })).detected[path];
  },
  formatKey: (path) => servedFiles.paths.has(path) ? new URL(path, servedFiles.origin).href : path,
  keepsFormat: (path) => servedFiles.paths.has(path),
  sourceForFile(source, document, index) {
    const f = document.files[index];
    // At workspace root the core displays this virtual prefix without its leading slash.
    return sourceAt(source, sourcePath(document, f).replace(/^<pool>\//, '/<pool>/'), f);
  },
  // a Library file the document does not read, such as one of several definitions of a subcircuit
  sourceForPath: (source, full) => sourceAt(source, full, { name: full.split('/').pop() }),
  newWindow(session, doc) {
    return openWebWindow(async () => {
      // Share descriptors within the handoff, not entire Documents or duplicate folder snapshots.
      const sources = [], ids = new Map(), snapshots = new Map();
      const sourceId = async source => {
        if (!source) return null;
        if (ids.has(source)) return ids.get(source);
        const id = sources.length;
        ids.set(source, id);
        let files = source.files && snapshots.get(source.files);
        if (!files) {
          files = {};
          for (const [path, read] of source.files || []) files[path] = decodeText(await (await read()).arrayBuffer());
          if (source.files) snapshots.set(source.files, files);
        }
        const descriptor = { name: source.name, path: source.path || `/${source.name}`, url: source.url,
          temporary: source.temporary, readonly: source.readonly, fromPool: source.fromPool, view: source.view, files };
        sources.push(descriptor);
        if (source.text !== undefined) files[descriptor.path] = source.text;
        return id;
      };
      const entryId = await sourceId(session.entry);
      const route = [];
      for (const step of session.path) route.push({ ...step, entry: await sourceId(step.entry) });
      const files = {};
      if (entryId !== null) Object.assign(files, sources[entryId].files);
      // The document contains applied text, never the pending editor buffer.
      if (doc) for (const f of doc.files) files[f.path.startsWith("/") || f.path.startsWith("<bundled>") ? f.path : joined(doc.path.slice(0, doc.path.lastIndexOf("/")), f.path)] = f.text;
      const pool = await Promise.all(session.pool.map(async (item) => ({ path: item.path, name: item.name, url: item.source.url,
        text: files[`/<pool>${item.path}`] ?? files[item.path] ?? decodeText(await (await item.read()).arrayBuffer()) })));
      if (entryId !== null) sources[entryId].files = files;
      return { ...session, pool, sources, entry: entryId, path: route };
    });
  },
  async initialSession() {
    const session = await receiveWebSession();
    if (!session) return null;
    const heldFiles = texts => new Map(Object.entries(texts || {}).map(([path, text]) => [path, () => new File([text], path)]));
    session.pool = session.pool.map((item) => {
      const read = () => new File([item.text], item.name);
      return { path: item.path, name: item.name, read, source: sessionSource(new Map([[item.path, read]]),
        { path: item.path, name: item.name, url: item.url, fromPool: !item.url }) };
    });
    if (session.sources) {
      // a view of a file of a document (visit-source.js) is the source of its entry with the file to show
      const sources = session.sources.map(({ files, view, ...entry }) => {
        const source = { ...sessionSource(heldFiles(files), entry), snapshot: files };
        return view ? { ...source, entrySource: source, view } : source;
      });
      session.entry = sources[session.entry] || null;
      session.path = session.path.map(step => ({ ...step, entry: sources[step.entry] }));
    } else if (session.entry) session.entry = { ...sessionSource(heldFiles(session.files), session.entry), snapshot: session.files };
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
  setPool(items, reload = false, refresh = [], searched = []) {
    const next = new Map(items.filter((item) => !item.builtin).map((item) => [item.path, item.read]));
    poolSources = new Map(items.filter(item => !item.builtin).map(item => [item.path, item.source]));
    poolReady = poolReady.catch(() => {}).then(async () => {
      const texts = new Map();
      const errors = [];
      for (const [path, read] of next) {
        try {
          const text = !reload && !refresh.includes(path) && poolFiles.get(path) === read && poolTexts.has(path) ? poolTexts.get(path)
            : decodeText(await (await read(reload)).arrayBuffer());
          texts.set(path, text);
        } catch (e) { errors.push(`Cannot read library file ${path}: ${e.message || e}`); }
      }
      poolFiles = next;
      poolTexts = texts;
      // (the files that joined the Library by being opened, which its stamps tell apart)
      librarySnapshot = { texts, epoch: librarySnapshot.epoch + (reload ? 1 : 0), opened: new Set(items.filter((item) => item.opened).map((item) => item.path)),
        // PATH (the local program's page): its files, and the folders it searches in order (prepare)
        path: { files: new Set(items.filter((item) => item.searchPath).map((item) => item.path)), dirs: searched } };
      if (!spare && !core) spare = warmCore();
      const indexed = await indexLibrary(spare?.worker ?? core, librarySnapshot);
      return [...errors, ...indexed];
    });
    return poolReady;
  },
  onInclude(handler) { included = handler; },
  // another file of an external manifest, opened from the Library
  externalFile,
  nativeMenu: false,
  /**
   * The URL that `?url=` names in the page's address, resolved against the page; or the files of
   * the manifest that `?manifest=` names.
   */
  initial() {
    const params = new URLSearchParams(location.search);
    if (params.has("manifest")) {
      if (params.has("url")) return Promise.reject(new Error("The page takes ?url= or ?manifest=, not both."));
      const url = parseUrl(params.get("manifest"));
      if (!url) return Promise.reject(new Error(`The page cannot open ?manifest=${params.get("manifest")}: it is not an http or https address.`));
      return Promise.resolve(manifestSource(url));
    }
    const given = params.get("url");
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
   * Read the whole document the source reads, as SVG for the page to draw, as an embedded viewer and the viewer API
   * read it; the page reads the file shown alone with `reader`. A source that holds its text, such as the clipboard's,
   * can use the file pool for includes. The texts of `edits`, by the paths the Document names the files by, are those
   * of the files, not what is read.
   */
  async build(source, edits, progress, suppliedPackages, signal, { bind, constraints } = {}) {
    let buildingCore;
    const abort = () => buildingCore?.terminate();
    try {
      const prepared = await prepare(source, edits, suppliedPackages, progress, { bind, constraints });
      const { request, read, library } = prepared;
      source = prepared.source;
      // An incoming candidate may be declined or superseded after building. Its lazy
      // drawings must not replace those of the retained document until display commits it.
      if (signal?.aborted) throw new Error("superseded");
      const warm = spare ?? warmCore();
      spare = null;
      buildingCore = warm.worker;
      pendingCores.add(buildingCore);
      signal?.addEventListener("abort", abort, { once: true });
      await warm.ready;
      await indexLibrary(buildingCore, library);
      const builder = { build: (request) => {
        if (signal?.aborted) throw new Error("superseded");
        progress?.("parsing"); return buildingCore.build(request);
      } };
      let { document: doc, files: collected } = await collect(builder, request, read);
      if (embed) {
        // Embeds have no Library controls. Resolve named public dependencies with the same
        // catalog and verification as Include Public Package; keep downloads request-local.
        const added = new Set();
        while (true) {
          const missing = doc.diagnostics.filter(d => d.severity === "error")
            .map(d => /^package (\S+) (\S+) cannot be found:/.exec(d.message)).filter(Boolean);
          if (!missing.length) break;
          const catalog = await publicPackages();
          let changed = false;
          for (const [, name, version] of missing) {
            const key = `${name} ${version}`;
            if (added.has(key)) continue;
            added.add(key);
            const pkg = catalog.find(p => p.name === name && p.version === version);
            if (!pkg) continue;
            const path = `/library/${pkg.path}`;
            request.files[path] = decodeText(await readPublicPackage(pkg));
            request.pool.push(path);
            changed = true;
          }
          if (!changed) break;
          ({ document: doc, files: collected } = await collect(builder, request, read));
        }
      }
      candidates.set(doc, buildingCore);
      builtRequests.set(doc, { ...request, files: collected });
      servedFiles = served(source, doc);
      return doc;
    } catch (e) {
      buildingCore?.terminate();
      pendingCores.delete(buildingCore);
      replenishCore();
      throw e instanceof Error ? e.message : String(e);
    } finally { signal?.removeEventListener("abort", abort); }
  },
  commitBuild(doc) {
    const candidate = candidates.get(doc);
    // a Document given from the readings kept, read with no core: the core shown stays, and reads it when asked to
    if (!candidate) {
      if (builtRequests.has(doc)) lastBuilt = doc;
      return;
    }
    lastBuilt = doc;
    if (core) {
      if (!spare) spare = { worker: core, ready: Promise.resolve() };
      else core.terminate();
    }
    core = candidate;
    candidates.delete(doc);
    pendingCores.delete(candidate);
    replenishCore();
  },
  ...(embed ? {} : {
    /**
     * What the page reads a document file by file with (file-reading.js, shell.js `Reader`): the files the source reads,
     * with the Library and `edits` over them, read and hashed as the core reads them, and a core, which starts only when
     * the outline or a reading is asked for. A reading with `keep` holds the drawings of what has no position: the
     * reader's core does, and becomes the core shown when the page shows the Document. A `background` reader has a core
     * of its own, and leaves the warm spare to the page.
     */
    async reader(source, edits, { progress, signal, bind, constraints, background = false } = {}) {
      const prepared = await prepare(source, edits, [], progress, { bind, constraints });
      const { entry, request, read, library } = prepared;
      // the texts known, by the paths of the request, and those read, once each
      const texts = { ...request.files };
      const reads = new Map();
      const textOf = (path) => {
        if (texts[path] !== undefined) return Promise.resolve(texts[path]);
        if (!reads.has(path)) {
          reads.set(path, Promise.resolve(read(path)).then((t) => (typeof t === "string" ? (texts[path] = t) : null), () => null));
        }
        return reads.get(path);
      };
      // the files earlier readers of the entry were asked for, read before the first request (`askedFor`), and those
      // this one is asked for, kept for the next
      if (!askedFor.has(entry.path)) askedFor.set(entry.path, new Set());
      const asked = askedFor.get(entry.path);
      let before = null;
      const readBefore = () => (before ??= Promise.all([...asked].map(textOf)));
      // the files the request was made with, and those the core asked this reader for: what it read besides them, to
      // hash them or before the first request, is what a reading may no longer name (`finish`)
      const given = new Set(Object.keys(texts));
      const askedHere = new Set();
      // (an error reading a file the core asks for fails the reading, as it did)
      const readAsked = async (path) => {
        const t = texts[path] !== undefined ? texts[path] : await read(path);
        if (typeof t === "string") {
          texts[path] = t;
          asked.add(path);
          askedHere.add(path);
        }
        return t;
      };
      const hashes = new Map();
      let worker = null, starting = null, failed = false;
      const abort = () => worker?.terminate();
      const stopped = () => { if (signal?.aborted) throw new Error("superseded"); };
      const useCore = () => (starting ??= (async () => {
        stopped();
        // (a background reading, preloading, takes a core of its own: the warm spare stays for the page's next open)
        const warm = background ? warmCore() : spare ?? warmCore();
        if (!background) spare = null;
        worker = warm.worker;
        if (!background) pendingCores.add(worker);
        signal?.addEventListener("abort", abort, { once: true });
        await warm.ready;
        await indexLibrary(worker, library);
        return worker;
      })());
      // the settings the host document is read with besides the texts (`cachedFor`)
      const cacheKey = JSON.stringify([entry.path, request.formats, request.dialect ?? null, request.auto_include, request.library_prefix, request.pool, request.package_dirs, request.search_paths]);
      const asking = (name) => async (r) => {
        stopped();
        const w = await useCore();
        stopped();
        progress?.("parsing");
        if (!readingCache()) return w[name](r);
        const forget = cachedFor.has(w) && cachedFor.get(w) !== cacheKey;
        cachedFor.set(w, cacheKey);
        return w[name](forget ? { ...r, cache: true, forget: true } : { ...r, cache: true });
      };
      const failing = async (job) => {
        try { return await job(); }
        catch (e) { failed = true; throw e instanceof Error ? e.message : String(e); }
      };
      // the last reading made with `later`, for its rest to be asked for (`restOf`): its request, with the files it was made from
      let partial = null;
      return {
        core: coreVersion === null ? null : { version: coreVersion, format: fileResultFormat },
        entry: entry.path,
        options: Object.fromEntries(READING_OPTIONS.filter((k) => request[k] !== undefined).map((k) => [k, request[k]])),
        async hashes(paths) {
          const out = new Map();
          await Promise.all(paths.map(async (path) => {
            if (!hashes.has(path)) hashes.set(path, textOf(requestPath(entry.path, path)).then((t) => (t === null ? null : textHash(t))));
            out.set(path, await hashes.get(path));
          }));
          return out;
        },
        outline: () => failing(async () => {
          await readBefore();
          const got = await collectOutline({ outline: asking("outline") }, { ...request, files: texts }, readAsked);
          Object.assign(texts, got.files);
          return got.outline;
        }),
        // with `later`, the reading is made through the core's reading cache and its answer is the smaller one: without the
        // texts of the files (`texts`), the spans and the geometry hints, which `restOf` then gives from what the cache
        // kept of it; the core answers sooner, and the page draws before they come
        restLater: readingCache(),
        readFile: (file, _keep, later = false) => failing(async () => {
          await readBefore();
          const asked = later && readingCache()
            ? { ...request, file, files: texts, hints: false, spans: false, texts: false } : { ...request, file, files: texts };
          const got = await collectFile({ buildFile: asking("buildFile") }, asked, readAsked);
          Object.assign(texts, got.files);
          partial = asked.spans === false ? { ...asked, files: got.files } : null;
          return got.result;
        }),
        // what the answer of `file` just read with `later` was left without (a FileRest: its spans and hints), asked for at
        // once, before the Worker is asked anything else through the cache; null when the core kept no such reading
        async restOf(file) {
          const made = partial?.file === file ? partial : null;
          partial = null;
          if (!made || !worker) return null;
          const { file: _, ...rest } = made;
          try { return await worker.restOf({ ...rest, rest_of: file }); }
          catch (e) { if (e instanceof Error && e.message === "not kept") return null; throw e; }
        },
        // the texts of the other files, which the reading of one leaves out, for the netlist pane
        async texts(doc) {
          for (const f of doc.files) {
            if (f.text) continue;
            const full = sourcePath(doc, f);
            f.text = texts[full] ?? texts[full.replace(/^<pool>\//, "/<pool>/")] ?? (await textOf(requestPath(entry.path, f.path))) ?? "";
          }
        },
        async symbols(file) {
          return (core ?? (await someCore())).symbols({ ...request, files: texts, symbols: file });
        },
        finish(doc, hit) {
          signal?.removeEventListener("abort", abort);
          if (!doc) {
            if (!worker) return;
            pendingCores.delete(worker);
            if (!failed && !spare && !signal?.aborted) spare = { worker, ready: Promise.resolve() };
            else worker.terminate();
            replenishCore();
            return;
          }
          if (worker) candidates.set(doc, worker);
          // a Document given from a reading kept: the core holds no drawings of it until it reads it again
          if (hit) kept.add(doc);
          // what the solve, Export Image and the drawings are read again from: the files read, but of those read to hash
          // them or before the first request, only those the outline or the reading names, not an include since removed
          const named = new Set([...(doc.outline ? outlinePaths(doc.outline) : []), ...(doc.inputs?.files ?? []).map((i) => i.path)]
            .map((path) => requestPath(entry.path, path)));
          const files = Object.fromEntries(Object.entries(texts).filter(([path]) => given.has(path) || askedHere.has(path) || named.has(path)));
          builtRequests.set(doc, { ...request, file: doc.file, files });
          servedFiles = served(prepared.source, doc);
        },
      };
    },
    /** The stamps of the Library now (file-results.js libraryStamps), once what it is given is read. */
    async libraryStamps() {
      await poolReady.catch(() => {});
      const snapshot = librarySnapshot;
      if (!stamped.has(snapshot)) {
        stamped.set(snapshot, libraryStamps([...snapshot.texts.keys()].map((path) => ({ path, opened: !!snapshot.opened?.has(path) })),
          (path) => textHash(snapshot.texts.get(path))));
      }
      return stamped.get(snapshot);
    },
  }),
  discardBuild(doc) {
    const candidate = candidates.get(doc);
    if (!candidate) return;
    if (!spare) spare = { worker: candidate, ready: Promise.resolve() };
    else candidate.terminate();
    candidates.delete(doc);
    pendingCores.delete(candidate);
    replenishCore();
  },
  ...(solver ? {
    /**
     * Solve the netlist shown with the solver service of the server (solve.js): the files the last build read, with
     * the text applied over them, are sent to it, and it gives the Document of the netlist as solved and the report,
     * as the desktop's Solve does, of the one level `settings.level` names or of every level; with `settings.file`, the
     * file that level is written in, only that file read for it, and the Document that file's. The service tells the
     * events of the solve (docs/solver-progress.md) as lines of JSON as they come, each to `onSolveProgress`'s
     * handler, then the result (apps/web/solver-service.mjs); a service that does not stream gives the result alone.
     * Cancelling drops the request, and the service stops the solver.
     */
    async solve(_source, settings, edits) {
      const request = lastBuilt && builtRequests.get(lastBuilt);
      if (!request) throw "Open a netlist first.";
      const read = solveFiles(request, edits);
      const own = new AbortController();
      solving = own;
      try {
        const response = await fetch(new URL("rnl-solver/solve", location.href), { method: "POST", signal: own.signal,
          // (X-RNL: the local program, which serves the page to one browser, asks for it of every request that does anything)
          headers: { "Content-Type": "application/json", "X-RNL": "1" },
          // (the files as the build read them, with the formats set for them when the netlist was opened)
          body: JSON.stringify({ ...read, config: settings?.config || "", timeall: settings?.timeall ?? null, respace: settings?.respace === true, level: settings?.level ?? null,
            file: settings?.file ?? null, stream: true }) });
        if (!/^application\/x-ndjson/.test(response.headers.get("Content-Type") || "")) {
          const answer = await response.json().catch(() => ({ error: `the solver service answered ${response.status}` }));
          if (!response.ok) throw answer.error || `the solver service answered ${response.status}`;
          return solvedBy(request, read, answer);
        }
        // lines of JSON: the events as they come, then the result or the error
        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let text = "";
        for (;;) {
          const { done, value } = await reader.read();
          if (value) text += decoder.decode(value, { stream: true });
          const lines = text.split("\n");
          text = done ? "" : lines.pop();
          for (const line of lines) {
            if (!line.trim()) continue;
            const said = JSON.parse(line);
            if (said.error) throw said.error;
            if (said.result) return solvedBy(request, read, said.result);
            if (said.event) solveProgress?.(said);
          }
          if (done) throw "the solver service ended without a result";
        }
      } catch (e) {
        if (own.signal.aborted) throw "the solve was cancelled";
        throw e instanceof TypeError ? "the solver service could not be reached; is serve.mjs still running?" : e;
      } finally {
        if (solving === own) solving = null;
      }
    },
    /** The table of the solver’s settings, as the server’s `rnl solver-defaults --json` gives it. */
    async solverTable() {
      const response = await fetch(new URL("rnl-solver/table", location.href), { cache: "no-store" });
      if (!response.ok) throw (await response.json().catch(() => ({}))).error || `the solver service answered ${response.status}`;
      return response.json();
    },
    /** What `rnl solve --check --json` says of settings written as named parameters, with no file. */
    async checkSettings(config) {
      const response = await fetch(new URL("rnl-solver/check", location.href), { method: "POST", headers: { "Content-Type": "application/json", "X-RNL": "1" },
        body: JSON.stringify({ config }) });
      const answer = await response.json().catch(() => ({ error: `the solver service answered ${response.status}` }));
      if (!response.ok) throw answer.error || `the solver service answered ${response.status}`;
      return answer;
    },
    cancelSolve() {
      solving?.abort();
    },
    onSolveProgress(handler) {
      solveProgress = handler;
    },
  } : inBrowser ? {
    /**
     * Solve the netlist shown in the browser, with the solver as WebAssembly in a Worker of its own that this
     * first solve starts (apps/web/solver-worker.js): the files the last build read, with the text applied over
     * them, are its request, and it gives the Document of the netlist as solved and the report, as the desktop's
     * Solve does, of the one level `settings.level` names or of every level. The events of the solve
     * (docs/solver-progress.md) go to `onSolveProgress`'s handler as they happen. Cancelling stops the Worker, and
     * the next solve starts another. It reads the files as the build did (`solveFiles`), as the development
     * server's solve does.
     */
    async solve(_source, settings, edits) {
      const request = lastBuilt && builtRequests.get(lastBuilt);
      if (!request) throw "Open a netlist first.";
      const read = solveFiles(request, edits);
      const own = new AbortController();
      solving = own;
      own.signal.addEventListener("abort", () => stopSolver("the solve was cancelled"), { once: true });
      try {
        const answer = await callSolver(inBrowser, { request: { ...read,
          solver_config: settings?.config || "", ...(settings?.timeall ? { timeall: settings.timeall } : {}), ...(settings?.respace ? { respace: true } : {}), level: settings?.level ?? null,
          file: settings?.file ?? null } }, (event) => solveProgress?.(event));
        if (answer.cancelled) throw "the solve was cancelled";
        if (answer.error !== undefined) throw answer.error;
        return solvedBy(request, read, answer);
      } finally {
        if (solving === own) solving = null;
      }
    },
    /** The table of the solver’s settings, from the solver’s module: the Worker that loads it gives it. */
    solverTable: () => callSolver(inBrowser, { table: true }),
    /** What the solver’s module says of settings written as named parameters, with no file: its `check(request)`. */
    checkSettings: (config) => callSolver(inBrowser, { check: { solver_config: config } }),
    cancelSolve() {
      solving?.abort();
    },
    onSolveProgress(handler) {
      solveProgress = handler;
    },
    /**
     * Solve > Save Solved Copy As: the last solve shown, downloaded, as the web cannot write where the files were read
     * (decision 0011): the netlist alone, under its name with `-solved`, when it reads no other file; else a ZIP of
     * every file the solve wrote and read, in their places. What was downloaded, for the page to say.
     */
    async saveSolved() {
      if (!lastSolved) throw "solve the netlist first: there is no solve to save";
      const { request, written } = lastSolved;
      const read = Object.keys(request.files).filter((path) => !request.pool.includes(path) || written[path] !== undefined);
      const name = request.entry.split("/").pop();
      const stem = name.includes(".") ? name.slice(0, name.lastIndexOf(".")) : name;
      if (read.length === 1 && read[0] === request.entry) {
        const copy = name.includes(".") ? `${stem}-solved${name.slice(name.lastIndexOf("."))}` : `${stem}-solved`;
        download(copy, written[request.entry] ?? request.files[request.entry]);
        return { downloaded: copy, written: 1, copied: 0 };
      }
      const paths = [...new Set([...Object.keys(written), ...read])].sort();
      const zip = await libraryZip(paths.map((path) => ({ path, read: () => written[path] ?? request.files[path] })));
      download(`${stem}-solved.zip`, zip);
      const copied = paths.filter((path) => written[path] === undefined).length;
      return { downloaded: `${stem}-solved.zip`, written: paths.length - copied, copied };
    },
  } : {}),
  /** The drawings of the objects the last Document lists without a position, which the core keeps. */
  /**
   * The drawings of the objects the last Document lists without a position, which the core keeps; a Document given
   * from a kept reading is read again first, for the core to hold its drawings.
   */
  async unplacedDrawings(level, items, names) {
    if (lastBuilt && kept.has(lastBuilt) && !primed.has(lastBuilt)) {
      core ??= await someCore();
      if (spare?.worker === core) spare = null;
      await indexLibrary(core, librarySnapshot);
      await core.buildFile(builtRequests.get(lastBuilt));
      primed.add(lastBuilt);
    }
    return core.drawings({ drawings: { level, items, names } });
  },
  /** Download the files: the web cannot write them where they were read from (decision 0011). */
  async save(source, files) {
    for (const f of files) download(f.name.split("/").pop(), f.text);
    return files.length === 1 ? `Downloaded ${files[0].name.split("/").pop()}.` : `Downloaded ${files.length} files.`;
  },
  /** Download Project: the archive `build` gives, where the browser lets the user choose, else as a download. */
  async saveProject(name, build) {
    let handle;
    if (typeof window.showSaveFilePicker === "function") {
      try {
        // Request the destination while the menu click still has user activation.
        handle = await window.showSaveFilePicker({ id: "rnl-project", suggestedName: name,
          types: [{ description: "ZIP", accept: { "application/zip": [".zip"] } }] });
      } catch (error) {
        if (error.name === "AbortError") return false;
        // Sandboxed/embedded pages may expose the API but not permit its use.
        if (error.name !== "SecurityError" && error.name !== "NotAllowedError") throw error;
      }
    }
    const blob = await build();
    if (handle) {
      const writer = await handle.createWritable();
      try { await writer.write(blob); await writer.close(); }
      catch (error) { await writer.abort().catch(() => {}); throw error; }
    } else download(name, blob);
    return name;
  },
  /** A manifest file the user chooses (File > Load Manifest): its name and text, or null. */
  async pickManifest() {
    const [file] = await chooseFiles(manifestInput);
    return file ? { name: file.name, text: await file.text() } : null;
  },
  sourceFromManifestJson,
  /** Download Manifest: the manifest of links, as a download. */
  async saveManifest(name, text) {
    download(name, new Blob([text], { type: "application/json" }));
  },
  /** A project's archive the user chooses: its name and bytes, or null. */
  async pickProject() {
    const [file] = await chooseFiles(projectInput);
    return file ? { name: file.name, bytes: new Uint8Array(await file.arrayBuffer()) } : null;
  },
  // a project's texts go to the WebAssembly core in one request, which holds them several times over:
  // a web page opens smaller projects than the desktop does
  projectLimits: { projectBytes: 256 * 1024 * 1024 },
  /** A project's files go in the workspace under its archive's name, as a folder's do. */
  projectRoot: (picked) => `/${picked.name.replace(/\.zip$/i, "").replace(/[\\/]/g, "_") || "project"}/`,
  /** The entry of a project read (project.js), and its pool as Library files; each sees all its files, as a folder's do. */
  projectSource(read) {
    const all = new Map(Object.entries(read.bytes).map(([path, bytes]) => [path, () => new File([bytes], path.split("/").pop())]));
    const items = read.pool.map((path) => ({ path, name: path.slice(1), read: all.get(path), source: { ...filesSource(all, path), fromPool: true } }));
    return { source: filesSource(all, read.entry), items };
  },
  /**
   * The SVG of `levels[level]` of `doc`, the Document shown, with `overlays` (File > Export Image):
   * the core builds its request again, with the files it was built from, on the core that holds it.
   */
  async exportSvg({ doc: shown, level, title, overlays, preview }) {
    // a solve's preview is drawn again from the files the solve wrote, over those it read, as the desktop does
    const solved = preview && lastSolved && builtRequests.get(lastBuilt);
    const request = solved
      ? { ...lastSolved.request, files: { ...lastSolved.request.files, ...lastSolved.written } }
      : builtRequests.get(shown);
    const worker = solved ? core : candidates.get(shown) ?? core;
    if (!request || !worker) throw new Error("This document was not loaded in this page. Reopen the file and export again.");
    // a reading of one file is read again as one
    const built = request.file === undefined ? await worker.build({ ...request, lazy_unplaced: false, overlays })
      : await worker.buildFile({ ...request, lazy_unplaced: false, overlays });
    const svg = built.levels[level]?.title === title ? built.levels[level].svg : null;
    if (!svg) throw new Error(`Level ${title} has changed since the document was loaded. Reload or reopen the file and export again.`);
    return svg;
  },
  /** Export Image: the image, as a download. */
  async saveImage(name, blob) {
    download(name, blob);
    return name;
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
  buildInfo,
  // Help > Check for Updates (updates.js): the version the site was built from
  version: async () => (await buildInfo()).viewer,
  /** Show a web page in a new tab, with no hold on this one. */
  browse(url) {
    window.open(url, "_blank", "noopener");
  },
};
