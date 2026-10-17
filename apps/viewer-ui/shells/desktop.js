// The desktop shell: the page inside the RNL Viewer app (apps/desktop), which reads files and builds
// documents natively and shows the menu bar itself. The only module that uses window.__TAURI__.

const tauri = window.__TAURI__;
const invoke = (cmd, args) => tauri.core.invoke(cmd, args);

/** A file on disk, named by its path; null for no path. */
const file = (path) => (path ? { name: path, path } : null);

/** The text of a source that holds it, with its edit, if any: it has the one file. */
const held = (source, edits) => (edits && Object.values(edits).length > 0 ? Object.values(edits)[0] : source.text);

/** @type {import("../shell.js").Shell} */
export default {
  platform: "desktop",
  nativeMenu: true,
  initial: () => invoke("initial_file").then(file),
  pick: (kind) => (kind === "file" ? invoke("pick_file").then(file) : Promise.resolve(null)),
  // a file by its path, or a text held by the source, such as the clipboard's, which is its one
  // file: an edit of it is the text
  build: (source, edits) =>
    source.text === undefined
      ? invoke("open_document", { path: source.path, edits: edits || null })
      : invoke("open_text", { name: source.name, text: held(source, edits) }),
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
  solve: (source, settings, edits) =>
    invoke(
      "solve_preview",
      source.text === undefined
        ? { path: source.path, settings, edits: edits || null }
        : { name: source.name, text: held(source, edits), settings },
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
