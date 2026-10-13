// The shell: everything the page needs from the platform it runs on, behind one small interface,
// so that the desktop viewer and the web viewer share the page. Only the shell's own module knows
// how the platform reads files or shows a menu.

/**
 * What the page asks of its platform.
 *
 * @typedef {object} Shell
 * @property {"desktop" | "web"} platform The platform, which picks the commands of commands.json it offers.
 * @property {boolean} nativeMenu Whether the platform shows the menu bar itself; if not, the page shows
 *   its own (menubar.js).
 * @property {() => Promise<Source | null>} initial What to open at start, if anything: on the desktop,
 *   the file named on the command line.
 * @property {(kind: string) => Promise<Source | null>} pick Ask for something to open: kind "file", and on
 *   the web also "folder" or "url". Null when nothing is chosen.
 * @property {(source: Source) => Promise<import("../../crates/rnl-view/types/document.js").Document>} build
 *   Read a source and build its Document; rejects with a message when it cannot. The Document's
 *   types are generated from the core's Rust types into crates/rnl-view/types/document.d.ts, for
 *   checking the page; nothing is loaded from there at run time.
 * @property {(id: string, checked: boolean) => void} setCheck Show a check-mark command's state, such as
 *   "toggle-netlist", in the platform's menu.
 * @property {(handler: (id: string) => void) => void} onCommand Call `handler` with the id of each command
 *   chosen in the platform's menu.
 * @property {(handler: (source: Source) => void) => void} [onOpen] Call `handler` with each source opened
 *   without `pick`, such as files dropped on the page. A shell that opens nothing so leaves it out.
 * @property {() => void} [about] Show the platform's About box, for Help > About in the page's own menu
 *   bar; without it the page shows a plain one. A platform whose menu shows its own About box leaves
 *   it out.
 */

/**
 * Something a shell can open. The page keeps it to read it again (File > Reload) and shows its
 * name; anything else in it is the shell's own: a path, files held in memory, a URL, a text.
 *
 * @typedef {object} Source
 * @property {string} name How the page names it, such as a file's path.
 */

/** @type {Shell | null} the shell chosen at start */
export let shell = null;

/**
 * Choose the shell and load it: shells/desktop.js when the page runs in the desktop viewer
 * (window.__TAURI__ is there), otherwise the shell that `?shell=NAME` names, "web" by default.
 */
export async function loadShell() {
  const name = "__TAURI__" in window ? "desktop" : new URLSearchParams(location.search).get("shell") || "web";
  if (!/^[a-z][a-z0-9-]*$/.test(name)) throw new Error(`"${name}" is not the name of a shell.`);
  try {
    shell = (await import(`./shells/${name}.js`)).default;
  } catch (e) {
    throw new Error(`The page could not load its "${name}" shell: ${e.message}`);
  }
  return shell;
}
