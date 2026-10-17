// The desktop shell: the page inside the RNL Viewer app (apps/desktop), which reads files and builds
// documents natively and shows the menu bar itself. The only module that uses window.__TAURI__.

const tauri = window.__TAURI__;
const invoke = (cmd, args) => tauri.core.invoke(cmd, args);

/** A file on disk, named by its path; null for no path. */
const file = (path) => (path ? { name: path, path } : null);

/** @type {import("../shell.js").Shell} */
export default {
  platform: "desktop",
  nativeMenu: true,
  initial: () => invoke("initial_file").then(file),
  pick: (kind) => (kind === "file" ? invoke("pick_file").then(file) : Promise.resolve(null)),
  // a file by its path, or a text held by the source, such as the clipboard's
  build: (source) =>
    source.text === undefined
      ? invoke("open_document", { path: source.path })
      : invoke("open_text", { name: source.name, text: source.text }),
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
  solve: (source, settings) =>
    invoke(
      "solve_preview",
      source.text === undefined ? { path: source.path, settings } : { name: source.name, text: source.text, settings },
    ),
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
};
