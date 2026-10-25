import { append, m, setText, startI18n } from "./i18n.js";
// The page of the RNL Viewer, shared by the desktop viewer and the web viewer. Its shell reads a
// netlist and builds the Document (shell.js); the page shows the drawing, the netlist text, the
// diagnostics and the subcircuit hierarchy, and follows a selected instance or net between the
// drawing and the text. Each part is a module of its own (README.md); this one starts the page.

import { startPool } from "./pool.js";
import { startPreload } from "./preload.js";
import { startUsages } from "./usages.js";
import { startNetlistColors } from "./netlist-colors.js";
import { setBackground } from "./background.js";
import { command, isChecked, runStartupShortcuts, setCheck, startCommands } from "./commands.js";
import { closeRequested } from "./editor.js";
import { dirtyPaths, unappliedPaths } from "./edits.js";
import { gridState, setGrid } from "./grid.js";
import { setHoverNames } from "./hover.js";
import { showMenuBar } from "./menubar.js";
import { openSource } from "./open.js";
import { showManifestLevel } from "./manifest-bar.js";
import { offerDisplay } from "./display.js";
import { restoreSession } from "./navigation.js";
import {
  setIndexHeight, setPane, setProblems, setProblemsHeight, setShowFailed, setSidebar, setSidebarWidth, phone,
} from "./panels.js";
import { loadShell } from "./shell.js";
import { startSolvePanel } from "./solve.js";
import { finishStartup, revealViewer, stylesReady } from "./startup.js";
import { current, doc, path, selected } from "./state.js";
import { renderUnplaced } from "./unplaced.js";
import { setSemanticsOnly, setRnlOnly } from "./text.js";
import { $, load, save, startSettings } from "./util.js";

/** Load the shell and the command table, then open what the shell has for the start. */
async function start() {
  await stylesReady;
  const shell = await loadShell();
  await startSettings(shell);
  startNetlistColors();
  await startI18n(shell, load("language", null), value => save("language", value));
  if (shell.embed) {
    const { startEmbed } = await import("./embed.js");
    await startEmbed();
    return;
  }
  setBackground(load("background", "light"));
  startUsages();
  setSidebarWidth(load("sidebar-width", 420));
  setProblemsHeight(load("problems-height", 180));
  // a phone starts on the drawing's page; the side panel's is a tap away
  setSidebar(phone.matches ? false : load("sidebar-shown", true));
  setProblems(load("problems-shown", true));
  startSolvePanel();
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
    setText(li, m("No file is open."));
    append($("diags"), li);
  }
  renderUnplaced();

  await startCommands();
  // the Library's files, without waiting for the core to index them (each build waits for that)
  await startPool();
  startPreload();
  if (!shell.nativeMenu) showMenuBar(true);
  if (shell.onOpen) await shell.onOpen((source) => openSource(source, false));
  if (shell.onCloseRequested) shell.onCloseRequested(closeRequested);
  if (shell.platform === "web" && new URLSearchParams(location.search).get("api") === "1") {
    const { startFullViewerApi } = await import("./full-viewer-api.js");
    await startFullViewerApi();
    return;
  }
  // The panels, the menus, their keys and the ways to open a file work from here. The core
  // (WebAssembly on the web, slow where a browser runs it without JIT) and the first document
  // follow, the document with its own loading stages, so a panel key never waits for them.
  void revealViewer(runStartupShortcuts);
  const session = await shell.initialSession?.();
  if (session) await restoreSession(session);
  else {
    const initial = await shell.initial();
    await openSource(initial, false);
    // files opened by a manifest open on the level it, or the address, names
    if (initial?.external) {
      await showManifestLevel(initial);
      // and the display settings it proposes, which the user chooses to apply or not
      const external = initial.external;
      await offerDisplay(external.display, initial.manifest || external.id, external.displayIgnored,
        (path) => path in external.files || external.pool.includes(path));
    }
  }
}

// a shell may reject with a plain message, as Tauri's invoke does
start().catch((e) => {
  setText($("empty"), e instanceof Error ? e.message : String(e));
}).finally(() => finishStartup(runStartupShortcuts));

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
   * Visit history: per level, its index (-1 for another file) and the instance it was reached
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
