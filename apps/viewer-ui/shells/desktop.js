import { m, tr } from "../i18n.js";
// The desktop shell: the page inside the RNL Viewer app (apps/desktop), which reads files and builds
// documents natively and shows the menu bar itself. The only module that uses window.__TAURI__.
// The page reads a document file by file with `reader` (file-reading.js), as in every shell: the app reads the files,
// makes the outline and the reading of one file with the core (reading.rs), and keeps nothing of them but the drawings
// of the reading shown. What is kept, and when it holds, is the page's.

import { flushSettings, load } from "../util.js";
import { decide, inform, modal } from "../dialog.js";
import { chooseEntry, listedText } from "../incoming.js";
import { sourcePath } from "../source-path.js";
import { fileFormats } from "../formats.js";
import { libraryStamps } from "../file-results.js";

let pool = [];
let poolReady = Promise.resolve();
const tauri = window.__TAURI__;
// (the options are the third argument: write_project_file and save_image take their file's path, name and kind from its
// headers)
const invoke = (cmd, args, options) => tauri.core.invoke(cmd, args, options);
// the files given to the Library last, with `opened` for those that joined it by being opened, and their stamps
let poolItems = [];
let stamps = null;
// the version of the core and of the shape of its readings, asked for once
let readingCore = null;
// what each Document given was read with, and the Document shown (commitBuild): its drawings are the app's while
// `drawn` is the number of the last reading that kept drawings (`kept`)
const readOf = new WeakMap();
let shownRead = null;
let kept = 0;
// Windows and macOS file systems ignore case, so a file's format choice is kept by its path in lower case
// there; the core compares the paths it reads the same way (Provider::path_key).
const caseless = /^(Win|Mac)/i.test(navigator.platform || "");

/** A file on disk, named by its path; null for no path. */
const file = (path) => (path ? { name: path, path } : null);
// a view of a file of a document (visit-source.js) keeps the file it shows and its entry's own source
const sessionSource = source => source ? { path: source.path, name: source.name,
  text: source.path && !source.readonly ? undefined : source.text, readonly: source.readonly,
  temporary: source.temporary, ...(source.view ? { view: source.view, entrySource: sessionSource(source.entrySource) } : {}) } : null;
/** What the app reads a source from: the file at its path, or the text it holds as the file of its name. */
const reading = (source) => source.text === undefined ? { path: source.path } : { name: source.name, text: source.text };
let openingDrop = false;
// the project archive this window opened or saved last, where Save Project offers to save again
let lastProject = null;
// a project's file, by its name within the folder chosen to open the project in, which the app keeps (project.rs)
const writeProjectFile = (relative, bytes) => invoke("write_project_file", bytes, { headers: { path: encodeURIComponent(relative) } });
// the folder the user chose to open a project in, as projectRoot gives it
let projectFolder = null;

/** @type {import("../shell.js").Shell} */
export default {
  platform: "desktop",
  perFile: true,
  sourceForFile(source, document, index) {
    const f = document.files[index], path = sourcePath(document, f);
    return path.startsWith('<') ? { path, name: f.name, text: f.text, readonly: true } : file(path);
  },
  sourceForPath: (source, path) => file(path),
  // the menu bar as the page plans it, in the language shown (native-menu.js): the app only draws it
  setMenu: (plan) => invoke("set_menu", { plan }),
  // the languages the app was built with, from i18n/ (apps/desktop/build.rs)
  uiLanguages: () => invoke("ui_languages"),
  uiCatalog: language => invoke("ui_catalog", { language }),
  // the core's parse formats, and the one it detects for a file the document shown does not read
  formats: () => invoke("formats"),
  detectFormat: (path) => invoke("detect_format", { path }),
  formatKey: (path) => caseless ? path.toLowerCase() : path,
  // Help > About (commands.js): the app's version, the core's and the commit they were built from
  async buildInfo() {
    const core = await invoke("core_build_info");
    return { viewer: await tauri.app.getVersion(), core: core.version, commit: core.commit, corePage: false };
  },
  loadSettings: (legacy) => invoke("load_settings", { legacy }),
  saveSetting: (key, value) => invoke("save_setting", { key, value }),
  bundledFiles: () => invoke("bundled_files"),
  // the public packages this build bundles, found without a network (public_packages.rs)
  bundledPackages: () => invoke("public_packages"),
  include: async (kind) => (await invoke("pick_pool", { kind })).map((path) => ({ path, name: path, source: file(path) })),
  pickPath: () => invoke("pick_pool", { kind: "path" }).then((paths) => paths[0] || null),
  // a PATH directory's files, with those of its sub-folders when it was added with them
  pathPackages: (dir, subfolders = false) => invoke("path_packages", { dir, subfolders }),
  setPool: (items, reload = false, refresh = []) => {
    poolItems = items;
    const paths = items.filter((item) => !item.builtin).map((item) => item.path);
    // PATH is searched by the app itself, folder by folder (rnl_cli FsProvider), as the CLI searches it: its files are
    // not in the pool, which is searched first, every file alike; they enter the subcircuit index all the same (`paths`)
    pool = items.filter((item) => !item.builtin && !item.searchPath).map((item) => item.path);
    poolReady = poolReady.catch(() => {}).then(() => invoke("update_library", { pool: paths, reload, refresh, formats: fileFormats() }));
    return poolReady;
  },
  nativeMenu: true,
  initialSession: () => invoke("initial_session"),
  newWindow: async (session) => {
    await flushSettings();
    return invoke("new_window", { session: { ...session,
      entry: sessionSource(session.entry),
      path: session.path.map(step => ({ ...step, entry: sessionSource(step.entry) })),
      pool: session.pool.map((item) => ({ path: item.path, name: item.name, source: { path: item.path, name: item.source.name } })),
    } });
  },
  setEnabled: (id, enabled) => invoke("set_command_enabled", { id, enabled }).catch(() => {}),
  initial: () => invoke("initial_file").then(file),
  fileAssociations: () => invoke("open_default_apps"),
  async onOpen(handler) {
    // Finder's initial file is held by Rust until initial_file/finish_startup; later Open
    // With events use the same filter, chooser and unsaved-edit flow as native drops.
    let pending = Promise.resolve();
    await tauri.event.listen('opened-files', ({payload}) => {
      pending = pending.catch(() => {}).then(async () => {
        const paths = await invoke('dropped_files', {paths:payload});
        // the file to open, as the page chooses it in every shell (incoming.js)
        const path = paths.length ? await chooseEntry(paths) : null;
        if (path) await handler(file(path));
      });
    });
    await tauri.webview.getCurrentWebview().onDragDropEvent(async ({ payload }) => {
      const blocked = openingDrop || modal() || document.body.classList.contains("busy");
      document.body.classList.toggle("dropping", !blocked && ["enter", "over"].includes(payload.type));
      if (payload.type !== "drop" || blocked) return;
      openingDrop = true;
      try {
        const paths = await invoke("dropped_files", { paths: payload.paths });
        if (!paths.length) {
          await inform({ title: m("Open"), paragraphs: [m("Drop a netlist or RNL package file ({0}).", await listedText())] });
          return;
        }
        // the file to open, as the page chooses it in every shell (incoming.js)
        const path = await chooseEntry(paths);
        if (path) await handler(file(path));
      } catch (error) {
        await inform({ title: m("Open"), paragraphs: [m`The dropped files could not be opened: ${error.message || error}`] });
      } finally {
        openingDrop = false;
      }
    });
  },
  pick: (kind) => (kind === "file" ? invoke("pick_file").then(file) : Promise.resolve(null)),
  /**
   * What the page reads a document file by file with (file-reading.js, shell.js `Reader`): the files of `source`, with
   * `edits` over them, the file pool, PATH and the Library, as the app reads them.
   */
  async reader(source, edits, { progress, bind = null, constraints = false } = {}) {
    await flushSettings();
    await poolReady;
    const args = { ...reading(source), edits: edits || null, pool, autoInclude: load("auto-include", true) === true, formats: fileFormats() };
    const core = await (readingCore ??= invoke("reading_core"));
    const requestId = progress ? crypto.randomUUID() : null;
    let unlisten = null;
    const listening = async () => {
      if (progress && !unlisten) unlisten = await tauri.event.listen("load-stage", ({ payload }) => { if (payload.id === requestId) progress(payload.stage); });
    };
    const hashes = new Map();
    let drawn = null;
    progress?.("reading");
    return {
      core,
      entry: source.text === undefined ? source.path.replaceAll("\\", "/") : `/${source.name}`,
      options: { autoInclude: args.autoInclude, formats: args.formats },
      async hashes(paths) {
        const wanted = [...new Set(paths.filter((p) => !hashes.has(p)))];
        if (wanted.length) {
          const got = await invoke("file_hashes", { source: args, paths: wanted });
          wanted.forEach((p, k) => hashes.set(p, got[k] ?? null));
        }
        return new Map(paths.map((p) => [p, hashes.get(p)]));
      },
      async outline() {
        await listening();
        return invoke("read_outline", { source: args, requestId });
      },
      async readFile(file, keep) {
        await listening();
        const result = await invoke("read_file", { source: args, file, keep, bind, constraints, requestId });
        if (keep) drawn = ++kept;
        return result;
      },
      // the texts of the other files, which the reading of one leaves out, for the netlist pane
      async texts(doc) {
        const missing = doc.files.filter((f) => !f.text);
        if (!missing.length) return;
        const got = await invoke("file_texts", { source: args, paths: missing.map((f) => f.path) });
        missing.forEach((f, k) => { f.text = got[k] ?? ""; });
      },
      symbols: (file) => invoke("file_symbols", { source: args, file }),
      finish(doc) {
        unlisten?.();
        if (doc) readOf.set(doc, { args, file: doc.file, bind, constraints, drawn });
      },
    };
  },
  /** The Document `doc` is shown: its drawings are asked for (unplacedDrawings). */
  commitBuild(doc) {
    const read = readOf.get(doc);
    if (!read) return;
    shownRead = read;
  },
  /** The window's title, as the page writes it (window-title.js). */
  setTitle(title) {
    invoke("set_title", { title }).catch(() => {});
  },
  /** The stamps of the Library now (file-results.js libraryStamps), once what it is given is read. */
  async libraryStamps() {
    await poolReady.catch(() => {});
    if (stamps?.items !== poolItems) {
      const items = poolItems;
      const hashes = await invoke("library_hashes");
      stamps = { items, stamps: libraryStamps(items.filter((i) => !i.builtin).map((i) => ({ path: i.path, opened: !!i.opened })), async (p) => hashes[p] ?? null) };
    }
    return stamps.stamps;
  },
  // the drawings of the objects listed without a position, which the app keeps from the reading shown: one given from
  // the readings kept, or one another reading has taken the place of, is read again first, with its drawings
  async unplacedDrawings(level, items, names) {
    const read = shownRead;
    if (read && read.drawn !== kept) {
      await invoke("read_file", { source: read.args, file: read.file, keep: true, bind: read.bind, constraints: read.constraints });
      read.drawn = ++kept;
    }
    return invoke("unplaced_drawings", { level, items, names });
  },
  setCheck(id, checked) {
    invoke("set_view_check", { id, checked }).catch(() => {});
  },
  onCommand(handler) {
    tauri.event.listen("menu", (e) => handler(e.payload));
  },
  // the version Tauri has from tauri.conf.json, which the installers carry too
  version: () => tauri.app.getVersion(),
  // the opener plugin, which the app's capability allows repository pages and the exact public package page
  browse: (url) => invoke("plugin:opener|open_url", { url }),
  // the Solve menu (decision 0010): solved natively on a thread of the app's, nothing written
  solve: async (source, settings, edits) => {
    await flushSettings();
    await poolReady;
    const autoInclude = load("auto-include", true) === true;
    return invoke(
      "solve_preview",
      source.text === undefined
        ? { path: source.path, settings, edits: edits || null, pool, autoInclude, formats: fileFormats() }
        : { name: source.name, text: source.text, settings, edits: edits || null, pool, autoInclude, formats: fileFormats() },
    );
  },
  // the solver's table of settings, which Solve > Settings draws its dialog from, and what it says of settings given
  // as named parameters (`rnl solve --check --json`)
  solverTable: () => invoke("solver_table"),
  checkSettings: (config) => invoke("check_solver_settings", { config }),
  cancelSolve() {
    invoke("cancel_solve").catch(() => {});
  },
  saveSolved: (encoding) => invoke("save_solved_copy", { encoding }),
  onSolveProgress(handler) {
    tauri.event.listen("solve-progress", (e) => handler(e.payload));
  },
  // decision 0011: the files changed, written where they were read, and the netlist saved as another file
  // a file that is not what was read, because another program changed it, is not written: its name comes back
  save: (source, files, overwrite, encoding) =>
    invoke("save_files", { entry: source.path, files, overwrite: overwrite === true, encoding }).then((saved) =>
      saved.changed.length > 0 ? { changed: saved.changed } : { written: saved.written },
    ),
  saveAs: (source, saved, encoding) =>
    invoke("save_file_as", { entry: source.path || null, name: saved.name, text: saved.text, encoding }).then(file),
  setDirty(dirty) {
    invoke("set_dirty", { dirty }).catch(() => {});
  },
  // File > Export Image: the core draws the level again, with the overlays, from what the document shown was built from
  // (a solve's preview: from the files the solve wrote)
  exportSvg: async ({ level, title, overlays, source, edits, preview }) => {
    await flushSettings();
    await poolReady;
    const args = { level, title, overlays, preview: preview === true, file: source.view ?? null, edits: edits || null, pool, autoInclude: load("auto-include", true) === true, formats: fileFormats() };
    return invoke("export_svg", source.text === undefined ? { ...args, path: source.path } : { ...args, name: source.name, text: source.text });
  },
  saveImage: async (name, blob, kind) =>
    invoke("save_image", new Uint8Array(await blob.arrayBuffer()), { headers: { name: encodeURIComponent(name), kind } }),
  // projects (project.js): the page makes and reads the archive; the app asks where, and reads and writes the bytes
  readFile: async (path) => new Uint8Array(await invoke("read_bytes", { path })),
  /** `text` as bytes in the encoding of the file at `path`, as Save writes it (project.js, for an edited file). */
  encodeLike: async (path, text) => new Uint8Array(await invoke("encode_like", { path, text })),
  async saveProject(name, build) {
    const path = await invoke("pick_project_save", { name, last: lastProject });
    if (!path) return false;
    // (the app writes the archive where its dialog was answered, and nowhere else)
    await invoke("write_project_archive", new Uint8Array(await (await build()).arrayBuffer()));
    lastProject = path;
    return path;
  },
  async pickProject() {
    const path = await invoke("pick_project");
    if (!path) return null;
    return { name: path.split(/[\\/]/).pop(), path, bytes: new Uint8Array(await invoke("read_bytes", { path })) };
  },
  /** The folder to open a project in, which the user chooses; files of the project already there are replaced only when the user says. */
  async projectRoot(picked, manifest) {
    const folder = await invoke("pick_project_folder", { near: picked.path });
    if (!folder) return null;
    const existing = await invoke("existing_files", { paths: manifest.files.map((f) => f.path) });
    if (existing.length) {
      const shown = existing.slice(0, 5).join(", ") + (existing.length > 5 ? m`, and ${existing.length - 5} more` : "");
      const choice = await decide({ title: m("Open Project"), paragraphs: [m`The folder already has files of this project: ${shown}.`, m("Replace them?")], action: tr("Replace") });
      if (choice !== "Replace") return null;
    }
    projectFolder = `${folder.replace(/\\/g, "/").replace(/\/$/, "")}/`;
    return projectFolder;
  },
  /** Write a project read into its folder: its entry, and its pool as Library files. */
  async projectSource(read, picked) {
    for (const [path, bytes] of Object.entries(read.bytes)) {
      if (!projectFolder || !path.startsWith(projectFolder)) throw new Error(`${path} is not in the folder chosen for the project.`);
      await writeProjectFile(path.slice(projectFolder.length), bytes);
    }
    lastProject = picked.path;
    return { source: file(read.entry), items: read.pool.map((path) => ({ path, name: path, source: file(path) })) };
  },
  onCloseRequested(handler) {
    tauri.event.listen("close-requested", () => handler());
  },
  quit() {
    invoke("quit").catch(() => {});
  },
};
