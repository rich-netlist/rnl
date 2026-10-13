// The commands and the keys that run them. A command runs by its id, whether it comes from the
// platform's menu or a key.

import { fit, zoomCenter } from "./drawing.js";
import { pickAndOpen, reload } from "./open.js";
import { setProblems, setSidebar } from "./panels.js";
import { clearSelection } from "./selection.js";
import { shell } from "./shell.js";
import { selected } from "./state.js";
import { ascend, descend, opens } from "./subcircuits.js";
import { $ } from "./util.js";

const checks = new Map(); // the id of a check-mark command -> whether it is checked

/** Pass the shell the check marks set so far, and the commands of its menu. */
export function startCommands() {
  for (const [id, checked] of checks) shell.setCheck(id, checked);
  shell.onCommand(command);
}

/** Run a command by its id. */
export function command(id) {
  switch (id) {
    case "open": pickAndOpen("file"); break;
    case "reload": reload(); break;
    case "fit": fit(); break;
    case "zoom-in": zoomCenter(1.25); break;
    case "zoom-out": zoomCenter(0.8); break;
    case "toggle-netlist": setSidebar($("app").classList.contains("sidebar-hidden")); break;
    case "toggle-problems": setProblems($("pane-netlist").classList.contains("problems-folded")); break;
    case "descend": descend(selected); break;
    case "ascend": ascend(); break;
    default: break;
  }
}

/** Set the state of a check-mark command, such as "toggle-netlist", in the platform's menu. */
export function setCheck(id, checked) {
  checks.set(id, checked);
  if (shell) shell.setCheck(id, checked);
}

document.addEventListener("keydown", (e) => {
  const ctrl = e.ctrlKey || e.metaKey;
  const key = e.key.toLowerCase();
  const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
  let id = null;
  if (ctrl && !e.shiftKey && key === "o") id = "open";
  else if (e.key === "F5" || (ctrl && key === "r")) id = "reload"; // re-read the file, not the page
  else if (ctrl && !e.shiftKey && key === "b") id = "toggle-netlist";
  else if (ctrl && e.shiftKey && key === "m") id = "toggle-problems";
  else if (ctrl && (e.key === "=" || e.key === "+")) id = "zoom-in";
  else if (ctrl && e.key === "-") id = "zoom-out";
  else if (typing) return;
  else if (!ctrl && !e.altKey && (key === "f" || key === "0")) id = "fit";
  else if (e.key === "Enter" && !(e.target instanceof HTMLButtonElement) && opens(selected) !== null) id = "descend";
  else if (e.key === "Backspace" && !ctrl && !e.altKey) id = "ascend";
  else if (e.key === "Escape") {
    clearSelection();
    e.preventDefault();
    return;
  }
  if (id) {
    e.preventDefault();
    command(id);
  }
});
