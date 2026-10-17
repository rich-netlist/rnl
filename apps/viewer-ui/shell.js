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
 * @property {(url: string) => void | Promise<void>} [browse] Show a web page outside the viewer: on the
 *   web in a new tab, on the desktop in the default browser. Help > Desktop Viewer shows the project
 *   site's page of the tools with it, and Help > Check for Updates a release's page; without it,
 *   those commands do nothing.
 * @property {() => Promise<string>} [version] The version of the app that runs, such as "0.1.0", for
 *   Help > Check for Updates (updates.js), which only the desktop offers; without it, that command
 *   does nothing.
 * @property {(source: Source, settings: SolverSettings) => Promise<Solved>} [solve] Solve a
 *   source as `rnl solve` solves it, without its records of the solver, and give it drawn as
 *   solved, nothing written (solve.js, decision 0010), with the solver's options set to
 *   `settings`. Rejects with a message when it cannot, and with CANCELLED when `cancelSolve`
 *   stopped it. Only the desktop offers it; without it, the Solve menu is not shown and its
 *   commands do nothing.
 * @property {() => Promise<SolverDefaults>} [solverDefaults] The solver's options as it has them
 *   without any setting, and the names of its objectives, for Solve > Settings to show
 *   (solve-settings.js); the page keeps none of them itself. Offered with `solve`.
 * @property {(options: string) => Promise<string | null>} [checkSolverOptions] Whether the solver takes
 *   `options`, written as `rnl solve --solver-options` takes them: null when it does, else what is
 *   wrong, in the solver's words. Offered with `solve`.
 * @property {() => void} [cancelSolve] Stop the solve under way.
 * @property {() => Promise<string | null>} [saveSolved] Save a copy of the last solve shown where the
 *   user says, never over a file that was read: what was saved, in a sentence, or null when the
 *   user cancelled. Rejects with a message when it cannot.
 * @property {(handler: (step: SolveStep) => void) => void} [onSolveProgress] Call `handler` with each
 *   level as the solve under way begins it.
 */

/**
 * A solve drawn: the Document of the files as solved, and the report of what became of each fact
 * (crates/rnl-cli/src/preview.rs).
 *
 * @typedef {object} Solved
 * @property {import("../../crates/rnl-view/types/document.js").Document} document
 * @property {object} report
 */

/**
 * The settings of a solve: `options` are the options that differ from the solver's defaults,
 * written as `rnl solve --solver-options` takes them, such as "beam=8;time_limit=none", empty for
 * the defaults; `hintsForLow` asks for every conflict group of the facts of priority low, which
 * that text cannot say (`rnl solve --report` does).
 *
 * @typedef {{ options: string, hintsForLow: boolean }} SolverSettings
 */

/**
 * The solver's options as it has them by default: the `tolerance`, the `grid` (null for none), the
 * `beam`, the `budget` of the search (`candidates`) and the router (`astar_expansions`), the
 * `time_limit` in seconds (null for none), `hints_for_low`; its `objectives`, the layers in order,
 * each with its `items`, objectives with their weights, and its `tolerance`; and the `names` of
 * every objective it knows, on or off.
 *
 * @typedef {object} SolverDefaults
 * @property {number} tolerance
 * @property {number | null} grid
 * @property {number} beam
 * @property {{ candidates: number, astar_expansions: number }} budget
 * @property {number | null} time_limit
 * @property {boolean} hints_for_low
 * @property {{ items: { name: string, weight: number }[], tolerance: number }[]} objectives
 * @property {string[]} names
 */

/**
 * Where a solve is: the level it begins, counted from 0 among the `levels` it solves, its title,
 * and whether it is the second pass, after files were raised to 0.3.
 *
 * @typedef {{ level: number, levels: number, title: string, again: boolean }} SolveStep
 */

/**
 * Something a shell can open. The page keeps it to read it again (File > Reload) and shows its
 * name; anything else in it is the shell's own: a path, files held in memory, a URL. A source
 * with `text`, as File > Load from Clipboard makes it, holds its text itself: every shell builds
 * that text as the file `name`, such as "clipboard.sp", whose extension says how it is read, with
 * no other file beside it.
 *
 * @typedef {object} Source
 * @property {string} name How the page names it, such as a file's path.
 * @property {string} [text] The text itself, for a source that holds it.
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
