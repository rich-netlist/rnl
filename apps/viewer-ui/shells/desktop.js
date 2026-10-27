import { m, tr } from "../i18n.js";
// The desktop shell: the page inside the RNL Viewer app (apps/desktop), which reads files and builds
// documents natively and shows the menu bar itself. The only module that uses window.__TAURI__.

import { flushSettings, load } from "../util.js";
import { choose, decide, inform, modal } from "../dialog.js";
import { sourcePath } from "../source-path.js";
import { fileFormats, isPackagePath } from "../formats.js";

let pool = [];
let poolReady = Promise.resolve();
const tauri = window.__TAURI__;
const invoke = (cmd, args) => tauri.core.invoke(cmd, args);
let preloadGeneration = 0;
const cancelPreload = () => { preloadGeneration++; return invoke("cancel_preload"); };
// Windows and macOS file systems ignore case, so a file's format choice is kept by its path in lower case
// there; the core compares the paths it reads the same way (Provider::path_key).
const caseless = /^(Win|Mac)/i.test(navigator.platform || "");

/** A file on disk, named by its path; null for no path. */
const file = (path) => (path ? { name: path, path } : null);
const sessionSource = source => source ? { path: source.path, name: source.name,
  text: source.path && !source.readonly ? undefined : source.text, readonly: source.readonly,
  temporary: source.temporary } : null;
let openingDrop = false;
// the project archive this window opened or saved last, where Save Project offers to save again
let lastProject = null;
const writeBytes = (path, bytes) => invoke("write_bytes", bytes, { headers: { path: encodeURIComponent(path) } });

/** @type {import("../shell.js").Shell} */
export default {
  platform: "desktop",
  sourceForFile(source, document, index) {
    const f = document.files[index], path = sourcePath(document, f);
    return path.startsWith('<') ? { path, name: f.name, text: f.text, readonly: true } : file(path);
  },
  setLanguage: language => invoke("set_ui_language", { language }),
  // the languages the app was built with, from i18n/ (apps/desktop/build.rs)
  uiLanguages: () => invoke("ui_languages"),
  uiCatalog: language => invoke("ui_catalog", { language }),
  // the core's parse formats, and the one it detects for a file the document shown does not read
  formats: () => invoke("formats"),
  detectFormat: (path) => invoke("detect_format", { path }),
  formatKey: (path) => caseless ? path.toLowerCase() : path,
  async about() {
    const core = await invoke("core_build_info");
    await inform({ title: m("About RNL Viewer"), paragraphs: [
      `RNL Viewer ${await tauri.app.getVersion()}`,
      `RNL core ${core.version} (${core.commit})`,
      m`${m("Draws netlists with Rich Netlist (RNL) records. The text can be edited and its changes applied; ")}${m("nothing is written to a file until you save.")}`,
      m("The drawing uses Noto Sans SC, copyright 2014–2021 Adobe, under the SIL Open Font License 1.1."),
    ] });
  },
  loadSettings: (legacy) => invoke("load_settings", { legacy }),
  saveSetting: (key, value) => invoke("save_setting", { key, value }),
  bundledFiles: () => invoke("bundled_files"),
  include: async (kind) => (await invoke("pick_pool", { kind })).map((path) => ({ path, name: path, source: file(path) })),
  pickPath: () => invoke("pick_pool", { kind: "path" }).then((paths) => paths[0] || null),
  // a PATH directory's files, with those of its sub-folders when it was added with them
  pathPackages: (dir, subfolders = false) => invoke("path_packages", { dir, subfolders }),
  setPool: (items, reload = false, refresh = []) => {
    cancelPreload().catch(() => {});
    const paths = items.filter((item) => !item.builtin).map((item) => item.path);
    // PATH packages retain the provider's per-directory precedence. Netlists may also resolve
    // bare includes; both kinds enter the subcircuit index, independently of package lookup.
    pool = items.filter((item) => !item.builtin && (!item.searchPath || !isPackagePath(item.path))).map((item) => item.path);
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
        const path = paths.length === 1 ? paths[0] : paths.length ? await choose({
          title:m('Open Netlist'), text:m('Choose the dropped file to open.'), filter:true,
          items:paths.map(path=>({label:path,value:path})),
        }) : null;
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
          await inform({ title: m("Open"), paragraphs: [m("Drop a netlist or RNL package file (.sp, .spi, .spice, .cir, .ckt, .net, .netlist, .inc, .lib, .scs or .rnl).")] });
          return;
        }
        const path = paths.length === 1 ? paths[0] : await choose({
          title: m("Open Netlist"), text: m("Choose the dropped file to open."), filter: true,
          items: paths.map((path) => ({ label: path, value: path })),
        });
        if (path) await handler(file(path));
      } catch (error) {
        await inform({ title: m("Open"), paragraphs: [m`The dropped files could not be opened: ${error.message || error}`] });
      } finally {
        openingDrop = false;
      }
    });
  },
  pick: (kind) => (kind === "file" ? invoke("pick_file").then(file) : Promise.resolve(null)),
  // a file by its path, or text held by the source, with edits to it and its includes
  memoryCache: async (clear = false) => {
    if (clear) await cancelPreload();
    await flushSettings(); return invoke("memory_cache", { clear });
  },
  preloadCandidates: () => pool.map(file),
  cancelPreload,
  preload: async source => {
    const generation = preloadGeneration;
    await poolReady;
    await flushSettings();
    if (generation !== preloadGeneration) return "cancelled";
    const ticket = await invoke("preload_ticket");
    if (generation !== preloadGeneration) return "cancelled";
    return invoke("preload_document", { path: source.path, pool, ticket, autoInclude: load("auto-include", true) === true, formats: fileFormats() });
  },
  build: async (source, edits, progress, _packages, _signal, { fresh = false } = {}) => {
    cancelPreload().catch(() => {});
    await flushSettings();
    await poolReady;
    const autoInclude = load("auto-include", true) === true, formats = fileFormats();
    const requestId = progress ? crypto.randomUUID() : null;
    const unlisten = progress ? await tauri.event.listen("load-stage", ({ payload }) => {
      if (payload.id === requestId) progress(payload.stage);
    }) : null;
    try {
      progress?.("reading");
      return await (source.text === undefined
        ? invoke("open_document", { path: source.path, edits: edits || null, pool, requestId, autoInclude, fresh, formats })
        : invoke("open_text", { name: source.name, text: source.text, edits: edits || null, pool, requestId, autoInclude, fresh, formats }));
    } finally { unlisten?.(); }
  },
  // the drawings of the objects listed without a position, which the app keeps from the last build
  unplacedDrawings: (level, items, names) => invoke("unplaced_drawings", { level, items, names }),
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
    cancelPreload().catch(() => {});
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
  // the solver's own defaults and options, which Solve > Settings shows and checks
  solverDefaults: () => invoke("solver_defaults"),
  checkSolverOptions: (options) =>
    invoke("check_solver_options", { options }).then(
      () => null,
      (e) => (e instanceof Error ? e.message : String(e)),
    ),
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
      saved.changed.length > 0 ? { changed: saved.changed } : saved.said,
    ),
  saveAs: (source, saved, encoding) =>
    invoke("save_file_as", { entry: source.path || null, name: saved.name, text: saved.text, encoding }).then(file),
  setDirty(dirty) {
    invoke("set_dirty", { dirty }).catch(() => {});
  },
  // File > Export Image: the core draws the level again, with the overlays, from what the document shown was built from
  exportSvg: async ({ level, title, overlays, source, edits }) => {
    await flushSettings();
    await poolReady;
    const args = { level, title, overlays, edits: edits || null, pool, autoInclude: load("auto-include", true) === true, formats: fileFormats() };
    return invoke("export_svg", source.text === undefined ? { ...args, path: source.path } : { ...args, name: source.name, text: source.text });
  },
  saveImage: async (name, blob, kind) =>
    invoke("save_image", new Uint8Array(await blob.arrayBuffer()), { headers: { name: encodeURIComponent(name), kind } }),
  // projects (project.js): the page makes and reads the archive; the app asks where, and reads and writes the bytes
  readFile: async (path) => new Uint8Array(await invoke("read_bytes", { path })),
  async saveProject(name, build) {
    const path = await invoke("pick_project_save", { name, last: lastProject });
    if (!path) return false;
    await writeBytes(path, new Uint8Array(await (await build()).arrayBuffer()));
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
    const existing = await invoke("existing_files", { root: folder, paths: manifest.files.map((f) => f.path) });
    if (existing.length) {
      const shown = existing.slice(0, 5).join(", ") + (existing.length > 5 ? m`, and ${existing.length - 5} more` : "");
      const choice = await decide({ title: m("Open Project"), paragraphs: [m`The folder already has files of this project: ${shown}.`, m("Replace them?")], action: tr("Replace") });
      if (choice !== "Replace") return null;
    }
    return `${folder.replace(/\\/g, "/").replace(/\/$/, "")}/`;
  },
  /** Write a project read into its folder: its entry, and its pool as Library files. */
  async projectSource(read, picked) {
    for (const [path, bytes] of Object.entries(read.bytes)) await writeBytes(path, bytes);
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
