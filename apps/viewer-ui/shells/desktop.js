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
  build: (source) => invoke("open_document", { path: source.path }),
  setCheck(id, checked) {
    // the app's View menu names its check marks after the panels: "netlist", "problems"
    invoke("set_view_check", { panel: id.replace(/^toggle-/, ""), shown: checked }).catch(() => {});
  },
  onCommand(handler) {
    tauri.event.listen("menu", (e) => handler(e.payload));
  },
};
