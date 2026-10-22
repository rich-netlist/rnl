// The shell: everything the page needs from the platform it runs on, behind one small interface,
// so that the desktop viewer and the web viewer share the page. Only the shell's own module knows
// how the platform reads files or shows a menu.

/**
 * What the page asks of its platform.
 *
 * @typedef {object} Shell
 * @property {boolean} [embed] The web shell's canvas-only mode, with no persisted preferences.
 * @property {"desktop" | "web"} platform The platform, which picks the commands of commands.json it offers.
 * @property {(language: "en" | "zh-CN") => Promise<void>} [setLanguage] Update native UI labels, preserving command IDs and state.
 * @property {() => Promise<Record<string, string>>} [bundledFiles] Built-in package texts by path.
 * @property {() => Promise<Array<{name: string, version: string, path: string, description: string, builtin: boolean}>>} [publicPackages] Published package metadata for lookup hints.
 * @property {(pkg: {name: string, version: string}, signal?: AbortSignal) => Promise<object[]>} [includePackage] Include an exact published package.
 * @property {(kind: string) => Promise<object[]>} [include] Include files, a folder or a library package.
 * @property {(items: object[], reload?: boolean, refresh?: string[]) => Promise<string[]> | void} [setPool] Replace and index the search pool; reload rereads all files, refresh replaces named entries. Returns read errors.
 * @property {() => Promise<string | null>} [pickPath] Pick a desktop search directory.
 * @property {(dir: string) => Promise<string[]>} [pathPackages] Package and netlist paths in a desktop PATH directory.
 * @property {(source: Source) => object} [poolItem] A web source's entry as a searchable file.
 * @property {(source: Source, document: object, file: number) => Source} sourceForFile Re-root at a document file, retaining read capabilities but no compiled include scope.
 * @property {() => Source[]} [preloadCandidates] Library/PATH sources eligible for background compilation.
 * @property {(source: Source) => Promise<string>} [preload] Prepare an independent file without activating its document. Returns ready/cached/full/paused/busy/cancelled/skipped/disabled.
 * @property {() => Promise<void>} [cancelPreload] Discard in-flight speculative results; never waits for their compiler.
 * @property {(legacy: object) => Promise<{ path: string, values: object }>} [loadSettings] Native settings and migration.
 * @property {(key: string, value: unknown) => Promise<void>} [saveSetting] Persist a native setting.
 * @property {(session: object, doc?: object) => Promise<unknown>} [newWindow] Open an independent viewer with a session.
 * @property {() => Promise<object | null>} [initialSession] Consume the one-use startup session, before initial().
 * @property {(id: string, enabled: boolean) => void} [setEnabled] Update a native command enabled state.
 * @property {boolean} nativeMenu Whether the platform shows the menu bar itself; if not, the page shows
 *   its own (menubar.js).
 * @property {() => Promise<Source | null>} initial What to open at start, if anything: on the desktop,
 *   the file named on the command line.
 * @property {(kind: string) => Promise<Source | null>} pick Ask for something to open: kind "file", and on
 *   the web also "url". Null when nothing is chosen.
 * @property {(clear?: boolean) => Promise<{ limit: number, used: number, entries: number, hits: number, misses: number, prefetched?: number }>} [memoryCache] Desktop render cache statistics; apply the saved limit and optionally clear retained results.
 * @property {(source: Source, edits?: Edits, progress?: (stage: string) => void, packages?: object[], signal?: AbortSignal, options?: { fresh?: boolean }) => Promise<import("../../crates/rnl-view/types/document.js").Document>} build
 *   Read a source and build its Document; rejects with a message when it cannot. With `edits`, the
 *   texts of some of its files are those, not what is read: Edit > Apply Changes (editor.js). The
 *   Document's types are generated from the core's Rust types into
 *   crates/rnl-view/types/document.d.ts, for checking the page; nothing is loaded from there at
 *   run time.
 *   Web API packages are request-local pool items; signal cancels an obsolete build.
 * @property {(doc: object) => void} [commitBuild] Adopt a web candidate's lazy drawing provider when displayed.
 * @property {(doc: object) => void} [discardBuild] Dispose of an unaccepted web candidate; harmless after commit.
 * @property {(level: number, items: number[], names: string[]) => Promise<{ svg?: string }[]>} [unplacedDrawings]
 *   The drawings of the objects at `items` of `levels[level].unplaced` of the Document `build` gave
 *   and the page accepted, `{ svg }` each, in order: the Document leaves them out (`Level.unplaced[].svg`), a netlist
 *   with no positions listing every object, and the list of them in the side panel asks for the
 *   rows it shows (unplaced.js). `names` are the objects' names as the Document lists them, which the
 *   shell's core checks, so that a request about a Document that was built since is refused. Rejects
 *   with a message when it cannot. A shell whose Documents carry the drawings leaves it out.
 * @property {(id: string, checked: boolean) => void} setCheck Show a check-mark command's state, such as
 *   "toggle-netlist", in the platform's menu.
 * @property {(handler: (id: string) => void) => void} onCommand Call `handler` with the id of each command
 *   chosen in the platform's menu.
 * @property {(handler: (source: Source) => void | Promise<unknown>) => void | Promise<void>} [onOpen] Call `handler` with each source opened
 *   without `pick`, such as files dropped on the page. A shell that opens nothing so leaves it out.
 * @property {() => Promise<void>} [fileAssociations] Open the operating system's default-app settings.
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
 * @property {(source: Source, settings: SolverSettings, edits?: Edits) => Promise<Solved>} [solve] Solve a
 *   source as `rnl solve` solves it, without its records of the solver, and give it drawn as
 *   solved, nothing written (solve.js, decision 0010), with the solver's options set to
 *   `settings`, over the texts of `edits` where it has any, as `build` takes them: the text applied
 *   (decision 0011). Rejects with a message when it cannot, and with CANCELLED when `cancelSolve`
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
 * @property {(source: Source, files: SavedFile[], overwrite?: boolean) => Promise<string | { changed: string[] }>} [save]
 *   Write the text of the files where they were read from, which the user has asked for with File >
 *   Save (decision 0011): a sentence that says what was saved. Unless `overwrite`, a file that is no
 *   longer what it was when it was read (`SavedFile.read`), because another program changed it or
 *   took it away, is not written, nor are the others: `{ changed }`, the names of those files, for
 *   the user to say. On the web, which cannot write files, download them. Rejects with a message when
 *   it cannot. A shell that offers neither does not save.
 * @property {(files: {path: string, read: () => Promise<Blob | string> | Blob | string}[], preparing: () => void) => Promise<boolean>} [downloadLibrary]
 *   Web only: choose a destination, then export the captured library as one ZIP; false on cancel.
 * @property {(source: Source, file: SavedFile) => Promise<Source | null>} [saveAs] Ask where to save the
 *   text of the netlist that was opened, write it there, and give the source of the file written; null
 *   when the user cancels. Rejects with a message when it cannot. Only the desktop offers it.
 * @property {(dirty: boolean) => void} [setDirty] Tell the platform that files have changes that are not
 *   saved, or have none, for the window's title and for closing it.
 * @property {(handler: () => void) => void} [onCloseRequested] Call `handler` when the user asks to
 *   close the window while `setDirty(true)` was the last word: the window stays until `quit`.
 * @property {() => void} [quit] Close the window, as `onCloseRequested` asked.
 */

/**
 * Texts to build over the files of a source, by the path the Document names each file by
 * (`SourceFile.path`): what the user edited and applied. The shell finds the file in its workspace as
 * the core does, relative to the directory of the netlist.
 *
 * @typedef {Record<string, string>} Edits
 */

/**
 * A file to save: its path as the Document names it, its name as shown, its text as the file has it,
 * and its text as it was read, with `\n` line ends, which a shell that can look at the file on disk
 * checks it for.
 *
 * @typedef {{ path: string, name: string, text: string, read?: string }} SavedFile
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
 * name and file path; other details are the shell's own: files held in memory or a URL. A source
 * with `text`, as File > Load from Clipboard makes it, holds its text itself: every shell builds
 * that text as the file `name`, such as "clipboard.sp", whose extension says how it is read, with
 * no other file beside it.
 *
 * @typedef {object} Source
 * @property {string} name How the page names it, such as a file's path.
 * @property {string} [path] A real file's identity, native or in the browser's workspace, for the Library.
 * @property {string} [text] The text itself, for a source that holds it.
 * @property {boolean} [temporary] Content without a file, kept in the Library's single temporary slot.
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
