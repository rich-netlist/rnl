// The desktop shell: the page inside the RNL Viewer app (apps/desktop), which reads files and builds
// documents natively and shows the menu bar itself. The only module that uses window.__TAURI__.

import { flushSettings, load } from "../util.js";
import { choose, inform, modal } from "../dialog.js";

let pool = [];
let poolReady = Promise.resolve();
const tauri = window.__TAURI__;
const invoke = (cmd, args) => tauri.core.invoke(cmd, args);

/** A file on disk, named by its path; null for no path. */
const file = (path) => (path ? { name: path, path } : null);
let openingDrop = false;

/** @type {import("../shell.js").Shell} */
export default {
  platform: "desktop",
  loadSettings: (legacy) => invoke("load_settings", { legacy }),
  saveSetting: (key, value) => invoke("save_setting", { key, value }),
  bundledFiles: () => invoke("bundled_files"),
  include: async (kind) => (await invoke("pick_pool", { kind })).map((path) => ({ path, name: path, source: file(path) })),
  pickPath: () => invoke("pick_pool", { kind: "path" }).then((paths) => paths[0] || null),
  pathPackages: (dir) => invoke("path_packages", { dir }),
  setPool: (items, reload = false, refresh = []) => {
    const paths = items.filter((item) => !item.builtin).map((item) => item.path);
    // PATH packages retain the provider's per-directory precedence. Netlists may also resolve
    // bare includes; both kinds enter the subcircuit index, independently of package lookup.
    pool = items.filter((item) => !item.builtin && (!item.searchPath || !/\.rnl$/i.test(item.path))).map((item) => item.path);
    poolReady = poolReady.catch(() => {}).then(() => invoke("update_library", { pool: paths, reload, refresh }));
    return poolReady;
  },
  nativeMenu: true,
  initialSession: () => invoke("initial_session"),
  newWindow: async (session) => {
    await flushSettings();
    return invoke("new_window", { session: { ...session,
      entry: session.entry ? { path: session.entry.path, name: session.entry.name,
        text: session.entry.path ? undefined : session.entry.text, readonly: session.entry.readonly,
        temporary: session.entry.temporary } : null,
      pool: session.pool.map((item) => ({ path: item.path, name: item.name, source: { path: item.path, name: item.source.name } })),
    } });
  },
  setEnabled: (id, enabled) => invoke("set_command_enabled", { id, enabled }).catch(() => {}),
  initial: () => invoke("initial_file").then(file),
  fileAssociations: () => invoke("open_default_apps"),
  async onOpen(handler) {
    await tauri.webview.getCurrentWebview().onDragDropEvent(async ({ payload }) => {
      const blocked = openingDrop || modal() || document.body.classList.contains("busy");
      document.body.classList.toggle("dropping", !blocked && ["enter", "over"].includes(payload.type));
      if (payload.type !== "drop" || blocked) return;
      openingDrop = true;
      try {
        const paths = await invoke("dropped_files", { paths: payload.paths });
        if (!paths.length) {
          await inform({ title: "Open", paragraphs: ["Drop a netlist or RNL package file (.sp, .spi, .spice, .cir, .ckt, .net, .netlist, .inc, .lib, .scs or .rnl)."] });
          return;
        }
        const path = paths.length === 1 ? paths[0] : await choose({
          title: "Open Netlist", text: "Choose the dropped file to open.", filter: true,
          items: paths.map((path) => ({ label: path, value: path })),
        });
        if (path) await handler(file(path));
      } catch (error) {
        await inform({ title: "Open", paragraphs: [`The dropped files could not be opened: ${error.message || error}`] });
      } finally {
        openingDrop = false;
      }
    });
  },
  pick: (kind) => (kind === "file" ? invoke("pick_file").then(file) : Promise.resolve(null)),
  // a file by its path, or text held by the source, with edits to it and its includes
  build: async (source, edits, progress) => {
    await flushSettings();
    await poolReady;
    const autoInclude = load("auto-include", false) === true;
    const requestId = progress ? crypto.randomUUID() : null;
    const unlisten = progress ? await tauri.event.listen("load-stage", ({ payload }) => {
      if (payload.id === requestId) progress(payload.stage);
    }) : null;
    try {
      progress?.("reading");
      return await (source.text === undefined
        ? invoke("open_document", { path: source.path, edits: edits || null, pool, requestId, autoInclude })
        : invoke("open_text", { name: source.name, text: source.text, edits: edits || null, pool, requestId, autoInclude }));
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
    await flushSettings();
    await poolReady;
    const autoInclude = load("auto-include", false) === true;
    return invoke(
      "solve_preview",
      source.text === undefined
        ? { path: source.path, settings, edits: edits || null, pool, autoInclude }
        : { name: source.name, text: source.text, settings, edits: edits || null, pool, autoInclude },
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
  saveSolved: () => invoke("save_solved_copy"),
  onSolveProgress(handler) {
    tauri.event.listen("solve-progress", (e) => handler(e.payload));
  },
  // decision 0011: the files changed, written where they were read, and the netlist saved as another file
  // a file that is not what was read, because another program changed it, is not written: its name comes back
  save: (source, files, overwrite) =>
    invoke("save_files", { entry: source.path, files, overwrite: overwrite === true }).then((saved) =>
      saved.changed.length > 0 ? { changed: saved.changed } : saved.said,
    ),
  saveAs: (source, saved) =>
    invoke("save_file_as", { entry: source.path || null, name: saved.name, text: saved.text }).then(file),
  setDirty(dirty) {
    invoke("set_dirty", { dirty }).catch(() => {});
  },
  onCloseRequested(handler) {
    tauri.event.listen("close-requested", () => handler());
  },
  quit() {
    invoke("quit").catch(() => {});
  },
};
