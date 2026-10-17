# viewer-ui

The page of the RNL viewers: the drawing, the netlist text, the problems and the subcircuit hierarchy, with a selected instance or net followed between the drawing and the text. The desktop viewer ([`apps/desktop`](../desktop/README.md)) and the web viewer ([`apps/web`](../web/README.md)) show it ([decision 0008](../../docs/decisions/0008-shared-core-per-platform-frontends.md)). The page shows a Document that its shell builds; it never reads files itself.

It is plain HTML, CSS and ES modules, with no dependencies and no build step. It keeps to the browser baseline of decision 0008, Chromium 109 and Safari 16, so newer features such as JSON import attributes, CSS nesting or `Array.prototype.toSorted` are not used. `package.json` only makes Node read the scripts as modules, so that they can be checked:

```bash
find apps/viewer-ui -name '*.js' -print0 | xargs -0 -n 1 node --check
```

[`tests/viewer`](../../tests/viewer/README.md) tests the page in Chromium, Firefox and WebKit against a fake shell, and smoke-tests the web viewer with its own shell and the core.

## Modules

| Path | Content |
| --- | --- |
| `index.html`, `style.css` | The page and its styles, light and dark. |
| `main.js` | The start: the stored layout, the shell, the command table, the first document, and the debug handle. |
| `shell.js` | The shell interface, and choosing the shell. |
| `shells/desktop.js` | The desktop shell, the only module that uses `window.__TAURI__`. |
| `shells/web.js` | The web shell: files, a folder, a URL or what is dropped, read as the core asks for them, and built by the core in a Worker. |
| `state.js` | What the page shows: the Document, the level drawn, the selection, the hierarchy path to the level and the views kept by path. Modules read these bindings and change them through setters. |
| `open.js` | Opening a source through the shell; reading it again keeps the level, the hierarchy path while its instances are there, the view and the selection. File › Load from Clipboard: the clipboard's text, shown with what it is guessed to be, opened as a source that holds it. |
| `drawing.js` | The drawing of a level: the view, moved by changing the SVG's `viewBox`; what a click hits, from the boxes, pins and wires the Document gives; the selection marks and the open badge; where the pointer is in the scene, in the status bar. |
| `grid.js` | The grid under the drawing, a view aid of the page on a canvas of its own (see [The grid](#the-grid)), and its settings dialog. |
| `selection.js` | Selecting an instance or a net. |
| `subcircuits.js` | The hierarchy path of the level shown and the bar above the drawing that shows it (see [The hierarchy path](#the-hierarchy-path)): going down into a subcircuit instance and back along the path, each level with the view it had on it. |
| `text.js` | The text pane: the tabs and the lines of each file, the statements that do not take effect, the places of the selection, revealing a line, the text outside the definition of the subcircuit drawn dimmed (`Level.lines`), and clicks in the text. |
| `syntax.js` | Highlighting the host statements and the RNL records. |
| `problems.js` | The problems list and the coloured line numbers. |
| `hierarchy.js` | The hierarchy view: the instance tree, and the subcircuit index with its search and initials. |
| `panels.js` | The side panel's views, what is folded, and the splitters. The layout is kept in `localStorage` under `sidebar-shown`, `sidebar-width`, `side-pane`, `problems-shown`, `problems-height`, `index-height` and `show-failed`. |
| `commands.js` | Running a command by its id, and the keys. |
| `solve.js` | The desktop's Solve menu (decision 0010): a solve through the shell, its progress and its Cancel button in the status bar, drawn as a preview over the file, its report in the Solve view, each line shown in the netlist when clicked, Close Preview and Save Solved Copy As. Only a shell with `solve` offers it. |
| `solve-settings.js` | Solve > Settings, the solver's options in a dialog (see [The solver's settings](#the-solvers-settings)): kept under `solve-options` as what differs from the solver's defaults, and given to a solve as the text `rnl solve --solver-options` takes. |
| `menubar.js` | The page's own menu bar. |
| `dialog.js` | Dialogs in the page's style for what a shell or the page asks: one of a list, with a filter; a line of text; several values in a form, such as the grid's settings; a message; a message with a button that acts. They are modal: the page's keys and its menu bar wait while one is shown. |
| `updates.js` | Help > Check for Updates: the newest release of the viewer on GitHub against the version that runs, compared by Semantic Versioning, in a dialog. |
| `util.js` | Small helpers for building the page, and for keeping values in `localStorage`, read and written so that a page without it starts from its defaults. |
| `commands.json` | The command table. |
| `fonts/` | The drawing's web fonts, which [`scripts/fonts.py`](../../scripts/README.md) writes and git ignores (see [Fonts](#fonts)). |

The modules import each other in cycles. At its top level a module only looks up its elements, keeps its own state and adds listeners; it calls nothing of another module there, so no imported binding is read before its module has run.

## The hierarchy path

The bar above the drawing shows the path of the level shown, from its root down to it, a segment per level: the root's names its level, the top level's title or a subcircuit's name, and every other names its subcircuit with, smaller and dimmer beside it, the instance it was reached through. The path is a list of levels, each with the name of its instance in the level before it (`state.js`):

- Going down into a subcircuit instance, by Enter, a double click, the open badge or View › Open Subcircuit, adds the instance to the path.
- A subcircuit picked in the index is the root of a path of its own, with no way up from it; a row of the tree gives the path from the top level down to its instance; a level reached otherwise, such as by an object of it selected in the text, cuts the path back to it when it is on it, and is the root of a new path when not.
- A click on a segment but the last goes back to its level, cutting the path there; the up button and Backspace go back one level, and at the root there is no up button and Backspace does nothing. The up button that hides at the root gives the focus to the drawing, so that Enter goes back down at once, also in a browser that moves the focus off a hidden element only later, as WebKit does; Enter is left to a focused button only while the button is shown. The level comes back with the view it had when it was left and the instance it was left through selected.
- Reading the document again keeps the path when its levels and instances are still there by their names; otherwise the level shown is the root.
- The views kept are by path, the root's title and the instances on it, so a subcircuit reached two ways keeps a view for each.

The tree marks the row of the path when the path starts at the top level, and else the first row that reaches the level; the index marks the level shown. Where the bar is narrow, the level's kind gives way first, then the levels after the root, from the root's side, behind an ellipsis whose list goes back to them, then the root; the level shown always stays, its name cut short only when nothing else is left to give way.

## The solver's settings

Solve > Settings (desktop only, where it replaces Solve > Time Limit) is a dialog of the options of `rnl solve` ([`options.rs`](../../crates/rnl-solver/src/options.rs)), which the Solve command then uses:

- **Time**: the time limit of the resolution of each level, in seconds, or no limit (8.5).
- **Placement**: the tolerance (11.7), the grid free coordinates are rounded to, or no rounding, and the beam of the search.
- **Work**: the candidates the search makes over a level, the cells the router expands for one net, and whether to seek every conflict group of the facts of priority low, which only changes the report and can take far longer (`hints_for_low`). The resolution of 8.5 has no count of work of its own: only the time limit bounds it.
- **Objectives**: the 14 objectives of the solver in an ordered list, each with buttons that move it up and down, a check box that turns it on (an objective that is off is not computed), one that puts it in the level of the objective above (the objectives of a level add up with their weights; levels rank in order), its weight, and, on the objective that begins a level, how far from the best a value counts as equal to it, so that the next level decides among those (the tolerance of a level). A badge says each one's level, `1`, `2`, ..., `+` for one added to the level above, `off`.
- **Restore Defaults** sets every field to the solver's default; OK then keeps no change at all.

The dialog says that the priority of a constraint, low, normal or high, is written in the netlist (7.6, 8.5) and not set here: these settings are what the solver does after the constraints are satisfied, the order in which it ranks the placements that satisfy them, and how much work it may do. Under the fields the dialog writes the settings as the command line takes them, `rnl solve --solver-options "beam=8;time_limit=none"`, with `--report` when every conflict group of the low facts is sought.

The page keeps no default of the solver's: the shell gives them (`solverDefaults`, the objectives' names too), so a default that the solver changes is the one shown, and one that was never changed follows it. What the user changed is kept in `localStorage` under `solve-options`, as only the settings that differ, in the form `{ time_limit, tolerance, grid, beam, budget: { candidates, astar_expansions }, objectives: [{ items: [{ name, weight }], tolerance }], hints_for_low }` with `"none"` for no limit or no rounding; a time limit kept under `solve-time-limit` by the earlier menu item counts until the settings are kept. A solve gets them from `solverSettings()` as `{ options, hintsForLow }`: `options` is the text of the settings that differ, such as `beam=8;time_limit=none`, empty for the defaults. OK first asks the shell whether the solver takes the text (`checkSolverOptions`), and the dialog shows its refusal in the solver's words, so the ranges are the solver's alone; the page only checks that a field holds a number.

## The grid

The drawing can show a light grey grid under it, at the integer points of the scene, the usual places of pins: the language's coordinates have x to the right and y up, a unit has no size of its own, and the origin no meaning of its own. View › Grid shows it as dots or lines, or not at all (G shows and hides it), and the axes through the origin, x = 0 and y = 0, a little darker; its dialog, Grid Settings, also sets the spacing, Auto (1 unit) or a number of units, and how often a line is a major one. Where the points would come closer than 10 pixels, the grid takes 2, 5, 10, 20, 50 ... times the spacing instead; with lines, every fifth line is a little darker, or, where that would not be a step of the sequence, the line at the next step of it (every 50 units with a step of 5). The dots and lines are the same size on the screen at every zoom.

The grid is drawn on a canvas under the drawing's SVG, from the view and the level's scale, `Level.scale`, the pixels of the SVG per unit. One row of dots is drawn on a canvas of its own and copied to every row: WebKit copies a canvas drawn onto itself whole first, each time, which made zooming slow and could crash the page. It is not part of the drawing: it takes no clicks, and changes neither fitting, nor the selection, nor the SVG, of which the page only leaves out the white backdrop that the SVG writer puts first, as the canvas is white already. The settings are kept in `localStorage` under `grid-shown`, `grid-style` (`"dots"` or `"lines"`), `grid-spacing` (`"auto"` or a number of units), `grid-major` and `grid-axes`; a value that will not do leaves the default: dots at every unit, and no axes.

The status bar shows where the pointer is on the drawing, in units with y up, to the digit that is worth 1 to 10 pixels at the zoom.

## Fonts

The core lays out the drawing's texts with the metrics of Noto Sans SC Regular and Bold ([decision 0009](../../docs/decisions/0009-fonts-shipped-with-the-core.md)), and the SVG names that family first. `fonts/` holds the two faces as web fonts for the page: WOFF2 files split by Unicode ranges, so that a browser downloads only the slices a drawing uses (Latin with the symbols of the markup, the other symbols, the CJK punctuation, the 3755 common hanzi of GB 2312 in slices of 50, its 3008 others in slices of 250, and the rest in slices of 1000 code points); `fonts.css`, which `index.html` links, with their `@font-face` rules; and `OFL.txt`, their licence. Write them with Python 3, [fontTools](https://github.com/fonttools/fonttools) and brotli:

```bash
pip install fonttools brotli
python scripts/fonts.py web
```

It downloads the pinned fonts into `target/fonts/` first. The page works without `fonts/`: the drawing then takes the family under the names it is installed by (Noto Sans CJK SC, Source Han Sans SC), or else sans-serif, each run still where the core placed it and as wide. What the fonts do not map, such as hangul or emoji, is drawn in a font of the system's either way, centred on the width the core estimated for it. The desktop app embeds `fonts/` with the page, and the web viewer's build copies it into the site.

The slices of both faces are 17.3 MB in all; the Latin slice of the regular face is 45 KB, and `fonts.css` is 229 KB, 69 KB gzip-compressed. A drawing with a few Chinese labels downloads a few hundred KB: corpus case 15, with 9 hanzi on its drawing, 205 KB in 11 slices (Latin, CJK punctuation and 9 of hanzi); case 14, with 23, 379 KB in 23 slices.

## The shell

Everything platform-specific goes through the shell, chosen at start: `shells/desktop.js` when the page runs in the desktop viewer (`window.__TAURI__` is there), otherwise the shell that `?shell=NAME` names, `web` by default. `shell.js` documents the interface:

| Member | |
| --- | --- |
| `platform` | `"desktop"` or `"web"`: which commands of the table it offers. |
| `nativeMenu` | Whether the platform shows the menu bar itself; if not, the page shows its own. |
| `initial()` | What to open at start, or null; on the desktop, the file on the command line. |
| `pick(kind)` | Ask for something to open: `"file"`, and on the web also `"folder"` or `"url"`; null when nothing is chosen. |
| `build(source)` | Read a source and return its Document; rejects with a message when it cannot. The Document's TypeScript declarations are generated from the core's Rust types ([`crates/rnl-view/types`](../../crates/rnl-view/README.md#type-definitions)); `shell.js` refers to them in its JSDoc, which loads nothing at run time. |
| `setCheck(id, checked)` | Show a check-mark command's state in the platform's menu. |
| `onCommand(handler)` | Call `handler(id)` for each command chosen in the platform's menu. |
| `onOpen(handler)` | Optional. Call `handler(source)` for each source opened without `pick`, such as files dropped on the page. |
| `about()` | Optional. Show the platform's About box, for Help > About in the page's own menu bar; without it the page shows a plain one. A platform whose menu shows its own About box has none. |
| `browse(url)` | Optional. Show a web page outside the viewer: the web shell opens it in a new tab, with no hold on the page (`noopener`); the desktop shell opens it in the default browser through Tauri's opener plugin, which the app's capability lets open the repository's pages on GitHub only. Help > Desktop Viewer (web) shows the project site's page of the tools with it, and Help > Check for Updates (desktop) a release's page; without it, those commands do nothing. |
| `solve(source, settings)` | Optional. Solve a source as `rnl solve` solves it, without its records of the solver, and give `{document, report}`: the Document of the files as solved, nothing written, and the report of [`rnl_cli::preview`](../../crates/rnl-cli/src/preview.rs), every line of it located in that Document's files. `settings` is `{ options, hintsForLow }`: the options that differ from the solver's defaults as `rnl solve --solver-options` takes them (empty for the defaults), and whether to seek every conflict group of the facts of priority low. Rejects with a message when it cannot, and with `the solve was cancelled` when `cancelSolve` stopped it. The desktop shell solves natively on a thread of the app's; the web shell has none, so the Solve menu, whose commands the table offers on the desktop only, and the Solve view are not shown there. |
| `cancelSolve()` | Optional. Stop the solve under way. |
| `solverDefaults()` | Optional, with `solve`. The solver's options as it has them without any setting, and the names of its objectives, for Solve > Settings. |
| `checkSolverOptions(options)` | Optional, with `solve`. Whether the solver takes `options`, written as `--solver-options` takes them: null when it does, else what is wrong, in the solver's words. |
| `saveSolved()` | Optional. Save a copy of the last solve shown where the user says, never over a file that was read: what was saved, in a sentence, or null when the user cancelled. |
| `onSolveProgress(handler)` | Optional. Call `handler({level, levels, title, again})` as the solve under way begins each level. |
| `version()` | Optional. The version of the app that runs, which the desktop shell has from Tauri, for Help > Check for Updates, the command `check-updates` that the table offers on the desktop only. The page asks GitHub's API for the releases itself (`updates.js`), as the desktop app's content security policy allows, and only when the command runs; without `version`, the command does nothing. |

A Source is the shell's own object: the page keeps it to read it again and shows its `name`, and knows nothing else of it. On the desktop it is a path; on the web, files chosen or dropped, a folder or a URL, each with a way to read its entry, which gives the entry's path in the workspace, and its other files. The one source that the page makes itself holds its `text`, for File › Load from Clipboard: each shell builds that text alone as the file `name`, such as `clipboard.sp`, whose extension says how it is read: the desktop with the app's `open_text` command, the web in the core's Worker.

The web shell builds with the core as WebAssembly in a module Worker, one for the page's lifetime, from `core/client.js` next to the page ([`bindings/runtime-wasm`](../../bindings/runtime-wasm/README.md), which [`apps/web`](../web/README.md) puts there), and asks for the SVG only. `collect` reads the files a document needs, decoded with `decodeText`, each named in the workspace by a rooted path: files chosen with Open or dropped at `/NAME`, a folder's at `/FOLDER/SUB/NAME`, and a URL's by their paths on the server that served the netlist, after any redirect, fetched relative to that address; a server that sends nothing for 30 seconds is given up. When more than one file could be the netlist, it asks which with `dialog.js`. It names the browser tab after the document, as the desktop app names its window. Its file inputs have the ids `file-input` and `folder-input`, for tests: files set on one while no chooser is shown open as if chosen with Open or Open Folder.

## The command table

`commands.json` lists the menus, each with its `label` and `items`. An item is `"-"`, a separator, shown only between two items; a submenu, `{ "label", "items" }` as a menu has, at any depth, shown only with an item the platform offers; or a command:

| Field | |
| --- | --- |
| `id` | What `command(id)` runs. `exit` and `about` are the platform's own: the desktop app quits, or shows its About box; the page runs `about` through the shell's `about()`, which the web shell has, or shows a plain About box when the shell has none and the page shows the menu bar. `desktop-viewer`, which only the web offers, shows the project site's page of the tools, where the desktop viewer is downloaded, with the shell's `browse(url)`: [the English page](https://rich-netlist.github.io/rnl/en/tools.html) when the browser's languages put English before Chinese, else [the Chinese one](https://rich-netlist.github.io/rnl/tools.html). |
| `label` | The text, with `&` before the underlined letter. |
| `keys` | The keys that run it, written like `Ctrl+Shift+M`, `F5` or `Enter`; Ctrl stands for Cmd on macOS. The first is the native menu's accelerator. |
| `shown` | How menus show the keys, when not as the first of them. |
| `check` | Whether it has a check mark. |
| `radio` | Whether, with `check`, it is one of a group of which the page checks one: the page's menu bar shows a dot, a native menu a check mark. |
| `page` | Whether the page always handles its key, as for Enter, Backspace and G: no menu accelerator would take the key from a text box. |
| `platforms` | The platforms that offer it; all when not given. |

The desktop app builds its native menu from the table ([`src/menu.rs`](../desktop/src/menu.rs)), its submenus too: on Windows the page handles the keys and the menu shows them after a tab; elsewhere the menu's accelerators handle them. A native menu toggles a check item when it is chosen, so the page sets the marks of a command's group again after running it, whether or not they changed. In the page's own menu bar, pointing at a submenu's entry opens the submenu beside it; from the keyboard, ArrowRight, Enter or the entry's letter opens it, and ArrowLeft or Escape closes it. The page runs keys by the same table. A key with Ctrl compares Shift only for a letter, since typing `+` may need Shift; a key without Ctrl that types or deletes does not count while Ctrl or Alt is held. In a text box only Ctrl keys and function keys count. Enter only opens a selected subcircuit instance, Escape clears the selection, and F5 and Ctrl+R, whatever else is held, read the document again instead of reloading the page.

## Debug handle

Module scope hides the page's state from tests and the console, so `window.viewer` shows a little of it:

| Member | |
| --- | --- |
| `doc` | The Document shown, or null. |
| `level` | The level drawn, an index into `doc.levels`. |
| `path` | The hierarchy path to it from its root: per level, `{ level, via }`, its index and the name of the instance it was reached through, null for the root. |
| `selected` | The selected object, an index into `doc.objects`, or null. |
| `command(id)` | Run a command, as its menu item does. |
| `checked(id)` | Whether a check-mark command, such as `toggle-netlist`, is checked. |
| `grid` | The grid's settings, `shown`, `style`, `spacing`, `major` and `axes`, and the steps it is drawn at in units, `step` and, for lines, `majorStep`; null when it is not drawn. |
| `menuBar(shown)` | Show or hide the page's own menu bar, whatever the shell says. |
