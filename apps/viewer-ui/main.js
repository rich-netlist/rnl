// The page of the RNL Viewer, shared by the desktop viewer and the web viewer. Its shell reads a
// netlist and builds the Document (shell.js); the page shows the drawing, the netlist text, the
// diagnostics and the subcircuit hierarchy, and follows a selected instance or net between the
// drawing and the text. Each part is a module of its own (README.md); this one starts the page.

import { command, startCommands } from "./commands.js";
import { showMenuBar } from "./menubar.js";
import { openSource } from "./open.js";
import {
  load, setIndexHeight, setPane, setProblems, setProblemsHeight, setShowFailed, setSidebar, setSidebarWidth,
} from "./panels.js";
import { loadShell } from "./shell.js";
import { current, doc, selected } from "./state.js";
import { $ } from "./util.js";

setSidebarWidth(load("sidebar-width", 420));
setProblemsHeight(load("problems-height", 180));
setSidebar(load("sidebar-shown", true));
setProblems(load("problems-shown", true));
setPane(load("side-pane", "netlist"));
setShowFailed(load("show-failed", true));
setIndexHeight(load("index-height", 260));
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
  openSource(await shell.initial(), false);
}

start().catch((e) => {
  $("empty").textContent = e.message;
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
  /** The selected object, an index into doc.objects, or null. */
  get selected() {
    return selected;
  },
  /** Run a command by its id, as its menu item does. */
  command,
  /** Show or hide the page's own menu bar, whatever the shell says. */
  menuBar: showMenuBar,
};
