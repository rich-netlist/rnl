// The window's title, the same in every shell: the name of the document shown, and a dot before it while files have
// changes not saved (`● name - RNL Viewer`, decision 0011). The page writes it; the desktop shell names its window by it
// (`shell.setTitle`), a page in a browser is its tab's title.
import { shell } from "./shell.js";

let name = null;
let dirty = false;

function show() {
  if (name === null) return;
  const title = `${dirty ? "● " : ""}${name} - RNL Viewer`;
  if (shell?.setTitle) shell.setTitle(title);
  else document.title = title;
}

/** The document shown is named `next`. */
export function titleName(next) {
  name = next;
  show();
}

/** Whether files have changes not saved. */
export function titleDirty(next) {
  dirty = next;
  show();
}
