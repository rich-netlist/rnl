// The commands and the keys that run them. Both come from the command table, commands.json, which
// the desktop viewer also builds its native menu from: the menus, their items, and per item its id,
// its label, its keys and the platforms that offer it. A command runs by its id, whether it comes
// from the platform's menu, the HTML menu bar or a key.

import { inform } from "./dialog.js";
import { fit, zoomCenter } from "./drawing.js";
import { pickAndOpen, reload } from "./open.js";
import { setProblems, setSidebar } from "./panels.js";
import { clearSelection } from "./selection.js";
import { shell } from "./shell.js";
import { selected } from "./state.js";
import { ascend, descend, opens } from "./subcircuits.js";
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
}

/**
 * Run a command by its id. Exit and About are the platform's own: the page has nothing to do for
 * Exit, and shows the shell's About box, or a plain one (about).
 */
export function command(id) {
  switch (id) {
    case "open": pickAndOpen("file"); break;
    case "open-folder": pickAndOpen("folder"); break;
    case "open-url": pickAndOpen("url"); break;
    case "reload": reload(); break;
    case "fit": fit(); break;
    case "zoom-in": zoomCenter(1.25); break;
    case "zoom-out": zoomCenter(0.8); break;
    case "toggle-netlist": setSidebar($("app").classList.contains("sidebar-hidden")); break;
    case "toggle-problems": setProblems($("pane-netlist").classList.contains("problems-folded")); break;
    case "descend": descend(selected); break;
    case "ascend": ascend(); break;
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
    const text = "Draws netlists with Rich Netlist (RNL) records. It only reads: it neither edits nor solves.";
    inform({ title: "About RNL Viewer", paragraphs: [text] });
  }
}

/** The project site's page of the tools, which offers the desktop viewer, in each of its languages. */
const TOOLS = { zh: "https://rich-netlist.github.io/rnl/tools.html", en: "https://rich-netlist.github.io/rnl/en/tools.html" };

/**
 * Show the site's page of the tools through the shell (Help > Desktop Viewer, on the web): in
 * English when the browser's languages put English before Chinese, else in Chinese, the site's
 * first language.
 */
function toolsPage() {
  const first = navigator.languages.find((language) => /^(en|zh)\b/i.test(language)) || "";
  if (shell.browse) shell.browse(/^en/i.test(first) ? TOOLS.en : TOOLS.zh);
}

/** Set the state of a check-mark command, such as "toggle-netlist", in the menus that show it. */
export function setCheck(id, checked) {
  checks.set(id, checked);
  if (shell) shell.setCheck(id, checked);
}

/** Whether a check-mark command is checked. */
export function isChecked(id) {
  return checks.get(id) === true;
}

/** Whether this platform offers a command; one that names no platforms is offered on all. */
function offered(item) {
  return !item.platforms || item.platforms.includes(shell.platform);
}

/** The menus with the items this platform offers, and a separator ("-") only between two items. */
export function menus() {
  return table.menus
    .map((m) => {
      const items = [];
      let gap = false;
      for (const item of m.items) {
        if (item === "-") {
          gap = items.length > 0;
        } else if (offered(item)) {
          if (gap) items.push("-");
          items.push(item);
          gap = false;
        }
      }
      return { label: m.label, items };
    })
    .filter((m) => m.items.length > 0);
}

/** How a menu shows an item's keys: its `shown` text, or else its first key. */
export function shortcut(item) {
  return item.shown || (item.keys ? item.keys[0] : "");
}

/**
 * Whether a key event is the key `spec` names, such as "Ctrl+Shift+M", "F" or "Enter"; Ctrl stands
 * for Cmd too. A key with Ctrl compares Shift only for a letter, since typing "+" may need Shift.
 * A key without Ctrl that types or deletes does not count while Ctrl, Cmd or Alt is held; other
 * keys, such as Enter and F5, count whatever is held with them.
 */
function matches(e, spec) {
  const parts = spec.split(/\+(?=.)/);
  const key = parts.pop().toLowerCase();
  if (e.key.toLowerCase() !== key) return false;
  const ctrl = e.ctrlKey || e.metaKey;
  if (parts.includes("Ctrl")) return ctrl && (!/^[a-z]$/.test(key) || e.shiftKey === parts.includes("Shift"));
  return !(ctrl || e.altKey) || !(key.length === 1 || key === "backspace");
}

/** The command a key runs, or null. In a text box only keys with Ctrl and function keys count. */
function keyCommand(e, typing) {
  if (!table) return null;
  for (const m of table.menus) {
    for (const item of m.items) {
      if (item === "-" || !offered(item)) continue;
      for (const spec of item.keys || []) {
        if (typing && !/^(Ctrl\+|F\d+$)/.test(spec)) continue;
        if (matches(e, spec)) return item.id;
      }
    }
  }
  return null;
}

document.addEventListener("keydown", (e) => {
  const ctrl = e.ctrlKey || e.metaKey;
  const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
  let id = null;
  if (e.key === "F5" || (ctrl && e.key.toLowerCase() === "r")) id = "reload"; // re-read the file, not the page
  else if (e.key === "Escape" && !typing) {
    clearSelection();
    e.preventDefault();
    return;
  } else id = keyCommand(e, typing);
  // Enter opens a selected subcircuit instance, and presses a button that has the focus
  if (id === "descend" && (e.target instanceof HTMLButtonElement || opens(selected) === null)) return;
  if (id) {
    e.preventDefault();
    command(id);
  }
});
