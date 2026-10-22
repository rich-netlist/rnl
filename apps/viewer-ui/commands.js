import { m, locale, setLanguage, translate } from "./i18n.js";
// The commands and the keys that run them. Both come from the command table, commands.json, which
// the desktop viewer also builds its native menu from: the menus, their items and submenus, and per
// item its id, its label, its keys and the platforms that offer it. A command runs by its id,
// whether it comes from the platform's menu, the HTML menu bar or a key.

import { includeFiles, reloadLibrary, toggleAutoInclude } from "./pool.js";
import { setBackground } from "./background.js";
import { inform } from "./dialog.js";
import { cancelGesture, fit, pan, zoomCenter } from "./drawing.js";
import { applyChanges, discardChanges, editing, save, saveAs, toggleEdit } from "./editor.js";
import { duplicateWindow } from "./navigation.js";
import { gridCommand } from "./grid.js";
import { hoverNamesOn, setHoverNames } from "./hover.js";
import { loadClipboard, pickAndOpen, reload } from "./open.js";
import { setProblems, setSidebar, showPane } from "./panels.js";
import { clearSelection } from "./selection.js";
import { shell } from "./shell.js";
import { closePreview, saveSolved, solve } from "./solve.js";
import { settings } from "./solve-settings.js";
import { doc, selected } from "./state.js";
import { findInSource } from "./text.js";
import { ascend, descend, opens } from "./subcircuits.js";
import { checkForUpdates } from "./updates.js";
import { fileAssociations } from "./file-associations.js";
import { memoryCache } from "./memory-cache.js";
import { toggleUsages } from "./usages.js";
import { netlistColors } from "./netlist-colors.js";
import { $ } from "./util.js";

let table = null; // commands.json, once loaded
const checks = new Map(); // the id of a check-mark command -> whether it is checked

/** Load the command table, and pass the shell the check marks and the commands of its menu. */
export async function startCommands() {
  const response = await fetch("commands.json");
  if (!response.ok) throw new Error(`The page could not load its command table (${response.status}).`);
  table = await response.json();
  for (const [id, checked] of checks) shell.setCheck(id, checked);
  shell.onCommand(command);
  setEnabled("descend-new-window", false);
  setEnabled("find-text", false);
  languageChecks();
}

function languageChecks() {
  if (!table) return;
  setCheck("language-en", locale() === "en");
  setCheck("language-zh", locale() === "zh-CN");
}
window.addEventListener("language-changed", languageChecks);

/**
 * Run a command by its id. Exit and About are the platform's own: the page has nothing to do for
 * Exit, and shows the shell's About box, or a plain one (about).
 */
export function command(id, options = {}) {
  if (!isEnabled(id)) return;
  if (shell?.embed && !["fit", "zoom-in", "zoom-out", "descend", "descend-new-window", "ascend"].includes(id)) return;
  switch (id) {
    case "language-en":
    case "language-zh": return setLanguage(id === "language-en" ? "en" : "zh-CN").catch(error =>
      inform({ title: m("Language"), paragraphs: [m`Could not change language: ${error.message || error}`] }));
    case "new-window": return duplicateWindow();
    case "open": pickAndOpen("file"); break;
    case "include-file": includeFiles("file"); break;
    case "include-folder": includeFiles("folder"); break;
    case "include-library": includeFiles("library"); break;
    case "open-url": pickAndOpen("url"); break;
    case "load-clipboard": loadClipboard(); break;
    case "reload": reload(); break;
    case "reload-library": return reloadLibrary();
    case "toggle-auto-include": return toggleAutoInclude();
    case "memory-cache": return memoryCache();
    case "save": save(); break;
    case "save-as": saveAs(); break;
    case "file-associations": return fileAssociations();
    case "toggle-edit": toggleEdit(); break;
    case "apply-edits": applyChanges(); break;
    case "discard-edits": discardChanges(); break;
    case "find-text": return findInSource();
    case "fit": fit(); break;
    case "zoom-in": zoomCenter(1.25); break;
    case "zoom-out": zoomCenter(0.8); break;
    case "background-light": setBackground("light"); break;
    case "background-dark": setBackground("dark"); break;
    case "netlist-colors": return netlistColors();
    case "toggle-grid":
    case "grid-none":
    case "grid-dots":
    case "grid-lines":
    case "grid-axes":
    case "grid-settings": gridCommand(id); break;
    case "toggle-hover-names": setHoverNames(!hoverNamesOn()); break;
    case "toggle-usages": return toggleUsages();
    case "toggle-netlist": setSidebar($("app").classList.contains("sidebar-hidden")); break;
    case "toggle-problems": setProblems($("pane-netlist").classList.contains("problems-folded")); break;
    case "panel-hide": return setSidebar(false);
    case "panel-files":
    case "panel-netlist":
    case "panel-hierarchy":
    case "panel-symbol":
    case "panel-usages":
    case "panel-unplaced":
    case "panel-solve": return showPane(id.slice("panel-".length));
    case "descend": return descend(selected, options);
    case "descend-new-window": return descend(selected, { newWindow: true });
    case "ascend": return ascend(options);
    case "shortcuts": inform({ title: m("Keyboard Shortcuts"), paragraphs: [
      m`${m`${m("1: Library. 2: Netlist. 3: Hierarchy. 4: Symbol. 5: Usage. 6: Not drawn.")}${shell.solve ? m(" 7: Solve report.") : ""}`}${m(" These keys also expand a collapsed side panel; they do not run while typing.")}`,
      m("Ctrl+R: Show Usages. F5: Reload the netlist."),
      m("E or Enter: Open Subcircuit. Ctrl+E or Backspace: Up to Parent Level."),
      m("Shift+E or Shift+Enter: Open Subcircuit in New Window. Hold Shift during any circuit navigation to open its target in a new window."),
      m("Ctrl+Q: Edit Text (the physical Control key on macOS too). Ctrl+Enter: Apply and Exit Editing."),
      m("Ctrl+F: Find in Netlist, including offscreen text. Enter / Shift+Enter: next / previous match."),
    ] }); break;
    case "solve": solve(); break;
    case "solve-close": closePreview(); break;
    case "solve-save": saveSolved(); break;
    case "solve-settings": settings(); break;
    case "check-updates": checkForUpdates(); break;
    case "about": about(); break;
    case "desktop-viewer": toolsPage(); break;
    default: break;
  }
}

/**
 * The About box: the shell's, or, when it has none and the page shows the menu bar, a plain one of
 * the page's own. A platform whose menu shows About itself shows its own box.
 */
function about() {
  if (shell.about) shell.about();
  else if (!shell.nativeMenu) {
    const text =
      m`${m("Draws netlists with Rich Netlist (RNL) records. The text can be edited and its changes applied; ")}${m("nothing is written to a file until you save.")}`;
    inform({ title: m("About RNL Viewer"), paragraphs: [text] });
  }
}

/** The project site's page of the tools, which offers the desktop viewer, in each of its languages. */
const TOOLS = { zh: "https://rich-netlist.github.io/rnl/zh/tools.html", en: "https://rich-netlist.github.io/rnl/en/tools.html" };

/**
 * Show the site's page of the tools through the shell (Help > Desktop Viewer, on the web): in
 * Chinese when the browser's languages put Chinese before English, else in English, the site's
 * default.
 */
function toolsPage() {
  const first = navigator.languages.find((language) => /^(en|zh)\b/i.test(language)) || "";
  if (shell.browse) shell.browse(/^zh/i.test(first) ? TOOLS.zh : TOOLS.en);
}

/** Set the state of a check-mark command, such as "toggle-netlist", in the menus that show it. */
export function setCheck(id, checked) {
  checks.set(id, checked);
  if (shell && table) shell.setCheck(id, checked);
}

/** Whether a check-mark command is checked. */
export function isChecked(id) {
  return checks.get(id) === true;
}

export const isEnabled = (id) => id === "panel-hide" ? !$("app").classList.contains("sidebar-hidden") : id === "find-text" ? !!doc && !editing : id !== "descend-new-window" || opens(selected) !== null;
export function setEnabled(id, enabled) {
  shell?.setEnabled?.(id, enabled);
}

/** Whether this platform offers a command; one that names no platforms is offered on all. */
function offered(item) {
  return (!item.platforms || item.platforms.includes(shell.platform)) && (!item.windowsOnly || /Win/.test(navigator.platform));
}

/**
 * The entries of a menu or a submenu that this platform shows: the items it offers, the submenus
 * that have any, each as { label, items }, and a separator ("-") only between two entries.
 */
function entries(items) {
  const shown = [];
  let gap = false;
  for (const item of items) {
    if (item === "-") {
      gap = shown.length > 0;
      continue;
    }
    const entry = item.items ? { label: translate(item.label), items: entries(item.items) } : { ...item, label: translate(item.label) };
    if (entry.items ? entry.items.length === 0 : !offered(item)) continue;
    if (gap) shown.push("-");
    shown.push(entry);
    gap = false;
  }
  return shown;
}

/** The menus with the entries this platform shows, and no menu left without any. */
export function menus() {
  return table.menus.map((m) => ({ label: translate(m.label), items: entries(m.items) })).filter((m) => m.items.length > 0);
}

/** The commands among a menu's items, those of its submenus too. */
function* commandsIn(items) {
  for (const item of items) {
    if (item === "-") continue;
    if (item.items) yield* commandsIn(item.items);
    else yield item;
  }
}

/** How a menu shows an item's keys: its `shown` text, or else its first key. */
export function shortcut(item) {
  return item.shown || (item.keys ? item.keys[0] : "");
}

/**
 * Whether a key event is the key `spec` names, such as "Ctrl+Shift+M", "F" or "Enter"; Ctrl stands
 * for Cmd too. A key with Ctrl compares Shift only for a letter, since typing "+" may need Shift.
 * E and Enter have separate Shift commands. Other page shortcuts keep their existing Shift
 * behavior; typing or deleting keys do not count while Ctrl, Cmd or Alt is held.
 */
function matches(e, spec) {
  const parts = spec.split(/\+(?=.)/);
  const key = parts.pop().toLowerCase();
  if (e.key.toLowerCase() !== key) return false;
  const ctrl = e.ctrlKey || e.metaKey;
  if (parts.includes("Control")) return e.ctrlKey && !e.metaKey && !e.altKey && e.shiftKey === parts.includes("Shift");
  if (parts.includes("Ctrl")) return ctrl && (!/^[a-z]$/.test(key) || e.shiftKey === parts.includes("Shift"));
  if (key === "e" || key === "enter") return !ctrl && !e.altKey && e.shiftKey === parts.includes("Shift");
  if (/^[1-7]$/.test(key)) return !ctrl && !e.altKey && !e.shiftKey;
  return !(ctrl || e.altKey) || !(key.length === 1 || key === "backspace");
}

/** The command a key runs, or null. In a text box only keys with Ctrl and function keys count. */
function keyCommand(e, typing, input) {
  if (!table) return null;
  for (const m of [...table.menus, { items: table.shortcuts || [] }]) {
    for (const item of commandsIn(m.items)) {
      if (!offered(item)) continue;
      for (const spec of item.keys || []) {
        // Panel keys work from navigation lists too, but never from editable controls.
        if ((item.id.startsWith("panel-") ? input : typing) && !/^(Ctrl\+|Control\+|F\d+$)/.test(spec)) continue;
        if (matches(e, spec)) return item.id;
      }
    }
  }
  return null;
}

document.addEventListener("keydown", (e) => {
  if (shell?.embed && !document.hasFocus()) return;
  if (e.defaultPrevented || e.isComposing || e.keyCode === 229) return;
  const ctrl = e.ctrlKey || e.metaKey;
  const input = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target.isContentEditable || !!e.target.closest("select");
  const textInput = input && !e.target.matches('input[type="checkbox"], input[type="radio"]');
  const typing = input || !!e.target.closest('[role="listbox"], [role="tree"], #tree, #index, #unplaced-list');
  if (e.key === "Escape" && cancelGesture()) {
    e.preventDefault();
    return;
  }
  const direction = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
  // Lists, menus, editors and controls retain their own navigation. Body and the drawing pan.
  if (direction && !typing && !ctrl && !e.altKey &&
      (e.target === document.body || e.target === $("canvas"))) {
    if (pan(...direction, e.shiftKey ? 0.3 : 0.1)) e.preventDefault();
    return;
  }
  let id = null;
  if (e.key === "F5") id = "reload"; // re-read the file, not the page
  else if (e.key === "Escape" && !typing) {
    clearSelection();
    e.preventDefault();
    return;
  } else id = keyCommand(e, typing, textInput);
  if (id === "find-text" && (typing || !isEnabled(id))) return;
  // Focused buttons and source links retain native Enter activation.
  const control = e.target.closest('button, a[href]')?.getClientRects().length > 0;
  if ((id === "descend" || id === "descend-new-window") && (control || opens(selected) === null)) return;
  // Shift modifies navigation, including Ctrl+Shift+E (up) and Shift+Backspace.
  if (!typing && e.shiftKey && e.key === "Backspace" && !ctrl && !e.altKey) id = "ascend";
  if (e.shiftKey && ctrl && e.key.toLowerCase() === "e") id = "ascend";
  if (id) {
    e.preventDefault();
    command(id, { newWindow: e.shiftKey });
  }
});
