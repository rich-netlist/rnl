// The page of the RNL Viewer, shared by the desktop viewer and the web viewer. Its shell reads a
// netlist and builds the Document (shell.js); the page shows the drawing, the netlist text, the
// diagnostics and the subcircuit hierarchy, and follows a selected instance or net between the
// drawing and the text. Each part is a module of its own (README.md); this one starts the page.

import { startPool } from "./pool.js";
import { setBackground } from "./background.js";
import { command, isChecked, setCheck, startCommands } from "./commands.js";
import { closeRequested } from "./editor.js";
import { dirtyPaths, unappliedPaths } from "./edits.js";
import { gridState, setGrid } from "./grid.js";
import { setHoverNames } from "./hover.js";
import { showMenuBar } from "./menubar.js";
import { openSource } from "./open.js";
import { restoreSession } from "./navigation.js";
import {
  setIndexHeight, setPane, setProblems, setProblemsHeight, setShowFailed, setSidebar, setSidebarWidth,
} from "./panels.js";
import { loadShell } from "./shell.js";
import { finishStartup, stylesReady } from "./startup.js";
import { current, doc, path, selected } from "./state.js";
import { renderUnplaced } from "./unplaced.js";
import { setSemanticsOnly, setRnlOnly } from "./text.js";
import { $, load, startSettings } from "./util.js";

/** Load the shell and the command table, then open what the shell has for the start. */
async function start() {
  await stylesReady;
  const shell = await loadShell();
  await startSettings(shell);
  if (shell.embed) {
    const { startEmbed } = await import("./embed.js");
    await startEmbed();
    return;
  }
  setBackground(load("background", "light"));
  setSidebarWidth(load("sidebar-width", 420));
  setProblemsHeight(load("problems-height", 180));
  setSidebar(load("sidebar-shown", true));
  setProblems(load("problems-shown", true));
  setPane(load("side-pane", "netlist"));
  setShowFailed(load("show-failed", true));
  setSemanticsOnly(load("semantics-only", false));
  setRnlOnly(load("rnl-only", false));
  // the text is read, not edited, until the user says (editor.js); the native menu starts checked
  setCheck("toggle-edit", false);
  setIndexHeight(load("index-height", 260));
  // the grid: dots at the integer points, and no axes, since the origin means nothing in particular
  setGrid({
    shown: load("grid-shown", true),
    style: load("grid-style", "dots"),
    spacing: load("grid-spacing", "auto"),
    major: load("grid-major", 5),
    axes: load("grid-axes", false),
  });
  setHoverNames(load("hover-names", true));
  {
    const li = document.createElement("li");
    li.className = "nothing";
    li.textContent = "No file is open.";
    $("diags").append(li);
  }
  renderUnplaced();

  await startCommands();
  await startPool();
  if (!shell.nativeMenu) showMenuBar(true);
  if (shell.onOpen) await shell.onOpen((source) => openSource(source, false));
  if (shell.onCloseRequested) shell.onCloseRequested(closeRequested);
  const session = await shell.initialSession?.();
  if (session) await restoreSession(session);
  else await openSource(await shell.initial(), false);
}

// a shell may reject with a plain message, as Tauri's invoke does
start().catch((e) => {
  $("empty").textContent = e instanceof Error ? e.message : String(e);
}).finally(finishStartup);

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
  /** The paths of the files whose text is not saved, and of those whose text is not applied. */
  get edits() {
    return { unsaved: dirtyPaths(), unapplied: unappliedPaths() };
  },
};

// Keep native/browser copy-paste menus in editable controls; the rest of the sidebar is a viewer.
$("sidebar").addEventListener("contextmenu", (e) => {
  if (!e.target.closest('input, textarea, select, [contenteditable="true"], [contenteditable=""]')) e.preventDefault();
});
