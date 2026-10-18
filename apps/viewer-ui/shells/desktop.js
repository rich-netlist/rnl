// The desktop shell: the page inside the RNL Viewer app (apps/desktop), which reads files and builds
// documents natively and shows the menu bar itself. The only module that uses window.__TAURI__.

import { flushSettings } from "../util.js";

let pool = [];
const tauri = window.__TAURI__;
const invoke = (cmd, args) => tauri.core.invoke(cmd, args);

/** A file on disk, named by its path; null for no path. */
const file = (path) => (path ? { name: path, path } : null);

/** @type {import("../shell.js").Shell} */
export default {
  platform: "desktop",
  loadSettings: (legacy) => invoke("load_settings", { legacy }),
  saveSetting: (key, value) => invoke("save_setting", { key, value }),
  bundledFiles: () => invoke("bundled_files"),
  include: async (kind) => (await invoke("pick_pool", { kind })).map((path) => ({ path, name: path, source: file(path) })),
  pickPath: () => invoke("pick_pool", { kind: "path" }).then((paths) => paths[0] || null),
  setPool: (items) => { pool = items.filter((item) => !item.builtin).map((item) => item.path); },
  nativeMenu: true,
  initial: () => invoke("initial_file").then(file),
  pick: (kind) => (kind === "file" ? invoke("pick_file").then(file) : Promise.resolve(null)),
  // a file by its path, or text held by the source, with edits to it and its includes
  build: async (source, edits) => {
    await flushSettings();
    return source.text === undefined
      ? invoke("open_document", { path: source.path, edits: edits || null, pool })
      : invoke("open_text", { name: source.name, text: source.text, edits: edits || null, pool });
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
  // the opener plugin, which the app's capability lets open the repository's pages on GitHub only
  browse: (url) => invoke("plugin:opener|open_url", { url }),
  // the Solve menu (decision 0010): solved natively on a thread of the app's, nothing written
  solve: async (source, settings, edits) => {
    await flushSettings();
    return invoke(
      "solve_preview",
      source.text === undefined
        ? { path: source.path, settings, edits: edits || null, pool }
        : { name: source.name, text: source.text, settings, edits: edits || null, pool },
    );
  },
  // the solver's own defaults and options, which Solve > Settings shows and checks
  solverDefaults: (solverConfig = null) => invoke("solver_defaults", { solverConfig }),
  pickSolverConfig: () => invoke("pick_solver_config"),
  checkSolverOptions: (options, solverConfig = null) =>
    invoke("check_solver_options", { options, solverConfig }).then(
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
