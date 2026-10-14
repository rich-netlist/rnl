// The page of the RNL Viewer, shared by the desktop viewer and the web viewer. Its shell reads a
// netlist and builds the Document (shell.js); the page shows the drawing, the netlist text, the
// diagnostics and the subcircuit hierarchy, and follows a selected instance or net between the
// drawing and the text. Each part is a module of its own (README.md); this one starts the page.

import { command, isChecked, startCommands } from "./commands.js";
import { gridState, setGrid } from "./grid.js";
import { showMenuBar } from "./menubar.js";
import { openSource } from "./open.js";
import {
  setIndexHeight, setPane, setProblems, setProblemsHeight, setShowFailed, setSidebar, setSidebarWidth,
} from "./panels.js";
import { loadShell } from "./shell.js";
import { current, doc, path, selected } from "./state.js";
import { $, load } from "./util.js";

setSidebarWidth(load("sidebar-width", 420));
setProblemsHeight(load("problems-height", 180));
setSidebar(load("sidebar-shown", true));
setProblems(load("problems-shown", true));
setPane(load("side-pane", "netlist"));
setShowFailed(load("show-failed", true));
setIndexHeight(load("index-height", 260));
// the grid: dots at the integer points, and no axes, since the origin means nothing in particular
setGrid({
  shown: load("grid-shown", true),
  style: load("grid-style", "dots"),
  spacing: load("grid-spacing", "auto"),
  major: load("grid-major", 5),
  axes: load("grid-axes", false),
});
{
  const li = document.createElement("li");
  li.className = "nothing";
  li.textContent = "No file is open.";
  $("diags").append(li);
}

/** Load the shell and the command table, then open what the shell has for the start. */
async function start() {
  const shell = await loadShell();
  await startCommands();
  if (!shell.nativeMenu) showMenuBar(true);
  if (shell.onOpen) shell.onOpen((source) => openSource(source, false));
  openSource(await shell.initial(), false);
}

// a shell may reject with a plain message, as Tauri's invoke does
start().catch((e) => {
  $("empty").textContent = e instanceof Error ? e.message : String(e);
});

// A handle for tests and the console, which cannot see into the modules.
window.viewer = {
  /** The Document shown, or null. */
  get doc() {
    return doc;
  },
  /** The level drawn, an index into doc.levels. */
  get level() {
    return current;
  },
  /**
   * The hierarchy path to it, from its root: per level, its index and the instance it was reached
   * through, null for the root.
   */
  get path() {
    return path.map((s) => ({ level: s.level, via: s.via }));
  },
  /** The selected object, an index into doc.objects, or null. */
  get selected() {
    return selected;
  },
  /** Run a command by its id, as its menu item does. */
  command,
  /** Whether a check-mark command, such as "toggle-netlist", is checked. */
  checked: isChecked,
  /** The grid's settings, and the steps it is drawn at, in units (grid.js). */
  get grid() {
    return gridState();
  },
  /** Show or hide the page's own menu bar, whatever the shell says. */
  menuBar: showMenuBar,
};
