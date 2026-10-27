# viewer-ui

Usage frame and role geometry now comes from the shared core (`Usage.frame`), projected
into the view without recomputing member bounds or label placement. Frame padding is fixed
in SVG coordinates; labels and interaction strokes retain their screen size. Colors,
visibility, selection and saved usage preferences remain frontend choices. View > Show
Current Directions displays `Level.currents` arrows, starts off, and persists independently
of Show Usages. Declaration ports use explicitly drawn `$IO_PORT` usages on the port net.

During startup the shared page shows **RNL Viewer**, with the version below it, until its shell,
settings and initial document are ready. `startup.js` reads the version independently of the main
module graph, from Tauri on desktop or the web build's `about.json`. Startup errors reveal the
normal page with the error message; unavailable version metadata does not block opening it.
The startup styles are inline and the full stylesheets load without blocking that first paint.
Layout initialization waits for the main stylesheet. On Windows, the main window paints native
name/version text before WebView2 starts, then reveals the initialized page in that same window.

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
| `loading.js` | Reading, parsing and displaying stages with an indeterminate bar; operation ownership prevents stale progress/completions from replacing a newer load. |
| `shell.js` | The shell interface, and choosing the shell. |
| `shells/desktop.js` | The desktop shell, the only module that uses `window.__TAURI__`. |
| `shells/web.js` | The web shell: files, a folder, a URL or what is dropped, read as the core asks for them, and built by the core in a Worker. |
| `state.js` | What the page shows: the Document, the level drawn, the selection, the hierarchy path to the level and the views kept by path. Modules read these bindings and change them through setters. |
| `open.js` | Opening a source through the shell; reading it again keeps the level, the visit history while its circuits are there, the view and the selection. File › Load from Clipboard: the clipboard's text, shown with what it is guessed to be, opened as a source that holds it. |
| `drawing.js` | The drawing of a level: the view, moved by changing the SVG's `viewBox`; a click and the pointer on it, with `hits.js`, `picker.js` and `hover.js` (see [Clicking and pointing](#clicking-and-pointing)); the selection marks, those of the labels attached to it too, and the open badge; where the pointer is in the scene, in the status bar. |
| `touch-gestures.js` | The touch of a view that is panned and zoomed by its coordinates, shared by the drawing and the symbol preview of the Symbol view (`svg-viewer.js`), so that a finger does the same on both: one finger drags, two pinch around their middle and drag as it moves, a tap and a double tap are told to the page. The element takes `touch-action: none`, so that a touch that starts on it neither scrolls nor zooms the page; the page is scrolled by touching beside it. |
| `hits.js` | What a point of the drawing is on: the Document's hits (boxes, pins, wires and labels) under it, in the order a click prefers them, the objects they stand for, and the name the pointer shows. |
| `picker.js` | The list beside the pointer that a click on several objects opens, to choose from with the mouse or the keys. |
| `hover.js` | The name beside the pointer while it rests on an instance or a net, and View › Show Names on Hover. |
| `grid.js` | The grid under the drawing, a view aid of the page on a canvas of its own (see [The grid](#the-grid)), and its settings dialog. |
| `background.js` | View > Background, kept as `background` in native settings on desktop and localStorage on web; Light by default. Dark uses a deep grey canvas and reflects HSL lightness strictly below 25%, keeping hue, saturation and alpha. It changes imported SVG elements, including lazy thumbnails, without changing the Document. Highlights retain their normal styles and apply the same colour rule, with no added glow; hover tips follow the drawing background. The menus and side panel keep their system colour scheme. |
| `selection.js` | Selecting an instance or a net. |
| `subcircuits.js` | The hierarchy path of the level shown and the bar above the drawing that shows it (see [The hierarchy path](#the-hierarchy-path)): going down into a subcircuit instance and back along the path, each level with the view it had on it. |
| `text.js` | The text pane: the tabs and the lines of each file, the statements that do not take effect, the places of the selection, revealing a line, the text outside the definition of the subcircuit drawn dimmed (`Level.lines`), the statements of the objects that are not drawn greyed (`Level.unplaced`), and clicks in the text. A file whose format is `text` (`isPlainFile` in `formats.js`), which the core gives a file of a hardware description language, Verilog-A for one, that has none set, and a file the user set to Text in the Library, is shown as plain text: no colours, folds, object places or not-drawn greying, as the core reads none of it (it gives one hint). |
| `text-lines.js`, `line-window.js` | Full text and lexical checkpoints, with a bounded DOM window for large files. |
| `virtual-selection.js`, `text-find.js` | File-coordinate text selection/copy and full-model find, including rows outside the DOM window. |
| `syntax.js` | Highlighting the host statements and the RNL records. |
| `edits.js` | Per-file editor text, applied text, saved snapshot and underlying source baseline; dirty/unapplied marks and the overlays to build or solve. |
| `navigation.js`, `window-session.js` | The common navigation entry point and new-window session handoff (see [Navigation and new windows](#navigation-and-new-windows)). |
| `editor.js` | Editing the text: the netlist pane switched to edit, a text box and line numbers for each file, the state above the text, Apply Changes, Save and Save As, and asking before changes not saved are left (see [Editing the text](#editing-the-text)). |
| `problems.js` | The problems list and the coloured line numbers. |
| `formats.js` | The parse format chosen for each file in the Library, kept by path under `file-formats`, and the core's list of formats, whose titles it translates where they are words (`formatTitle`: Text; SPICE and Spectre are names); the choices that every request to the core carries (see **Parse formats**, after the Library's paragraphs). |
| `package-offers.js` | Background web lookup of explicit missing packages and navigation-safe Library confirmations. |
| `hierarchy.js` | The hierarchy view: the instance tree, with below it the subcircuits that the top level does not call as trees of their own, and the subcircuit index with its search and initials. |
| `usages.js` | Scope-local usages grouped by definition; editable Color, visibility preferences, source and roles; fixed-screen overlays and role labels, selection focus and border picking. |
| `roots.js` | The subcircuits that the top level does not call, which start trees of their own, and the level a document opens on (see [Subcircuits that nothing calls](#subcircuits-that-nothing-calls)). |
| `unplaced.js` | The Not drawn view: the objects of the level shown that have no position, each with its symbol and the nets at its terminals, in a list that loads a part at a time, with their drawings, and is searched by name (see [The objects not drawn](#the-objects-not-drawn)). |
| `panels.js` | The side panel's views, what is folded, and the splitters. The layout is kept in the shell’s settings store (native `settings.json` on desktop, `localStorage` on web) under `sidebar-shown`, `sidebar-width`, `side-pane`, `problems-shown`, `problems-height`, `index-height` and `show-failed`. |
| `commands.js` | Running a command by its id, and the keys. |
| `solve.js` | The desktop's Solve menu (decision 0010): a solve through the shell, its progress and its Cancel button in the status bar, drawn as a preview over the file, its report in the Solve view, each line shown in the netlist when clicked, Close Preview and Save Solved Copy As. Only a shell with `solve` offers it. |
| `solve-settings.js` | Solve > Settings, the solver's options in a dialog (see [The solver's settings](#the-solvers-settings)): kept under `solve-options` as what differs from the solver's defaults, and given to a solve as the text `rnl solve --solver-options` takes. |
| `menubar.js` | The page's own menu bar. |
| `dialog.js` | Dialogs in the page's style for what a shell or the page asks: one of a list, with a filter; a line of text; several values in a form, such as the grid's settings; a message; a message with a button that acts. They are modal: the page's keys and its menu bar wait while one is shown. |
| `updates.js` | Help > Check for Updates: the newest release of the viewer on GitHub against the version that runs, compared by Semantic Versioning, in a dialog. |
| `util.js` | Small helpers for building the page, and for keeping values in the shell’s settings store (native `settings.json` on desktop, `localStorage` on web), read and written so that a page without it starts from its defaults. |
| `embed.js`, `embed.css` | Web-only canvas controls, hierarchy path and postMessage API; started only when the web shell has `embed=1`. See [Embedding](../web/README.md#embedding). |
| `commands.json` | The command table. |
| `fonts/` | The drawing's web fonts, which [`scripts/fonts.py`](../../scripts/README.md) writes and git ignores (see [Fonts](#fonts)). |

The modules import each other in cycles. At its top level a module only looks up its elements, keeps its own state and adds listeners; it calls nothing of another module there, so no imported binding is read before its module has run.

## Loading and large netlists

Opening, reloading and applying text show **Reading files…**, **Parsing netlist…** and
**Displaying netlist…** above an indeterminate bar. No percentage is claimed. The web shell reports
file collection and Worker builds; the desktop shell matches native parsing events to one request.
The indicator appears after 120 ms, so quick opens do not flash or wait for progress-only frames.
Completion and errors clear the indicator. Embedded viewers use the same stages, and a late result
from an older open cannot clear the newer open's indicator. Reduced-motion preferences stop the bar's animation.

The desktop shell keeps a bounded LRU of compiled documents, SVGs and lazy drawings; a hit reports
**Using cached drawing…**. File > Memory Cache configures its per-window budget (256 MiB by default).
`shell.build` takes an optional fourth argument `{ fresh }`; Reload sets it to true so F5 always
rebuilds. The optional `shell.memoryCache(clear)` returns native cache statistics and can clear it.
The web shell continues building in its Worker and does not expose this native cache setting.

Files over 200 lines keep their full text and lexical state but render only the viewport and 40
overscan rows on each side. Diagnostics, selected statements, scope and not-drawn marks are attached
to logical lines and reapplied to recycled rows. File switches, hierarchy/Problems jumps, sidebar
resizing and editing preserve the line position; the gutter grows to fit large line numbers.
Editing continues to use a native textarea containing the complete file.

Problems retains the complete diagnostic list and total counts, but initially creates only 100
rows. Scrolling near the bottom, or activating the button there, appends the next 100. This keeps
large diagnostic sets out of navigation's first layout; every row still links to its source.

**Edit › Find in Netlist** / **Ctrl+F** (Cmd+F on macOS) searches the entire active read-only file, including offscreen
text; Enter and Shift+Enter move between matches, Escape closes find. Mouse selection is retained
in file coordinates while scrolling. **Ctrl+A** in the read view selects the whole file, and copying
uses the full text rather than just rendered rows. Copied line endings are normalized to LF.
Small files retain ordinary DOM and native selection.

## Viewing and output

Drag with the right mouse button to fit a rectangle to the drawing viewport, keeping its aspect ratio
and the existing 2%–5000% zoom limits. A drag must exceed 4 CSS pixels in both dimensions; a shorter
press does nothing (the canvas has no context menu). Escape, pointer cancellation, loss of capture
or window focus cancels the rectangle. Left-button selection and left/middle-button panning are
unchanged. With focus on the drawing or the page body, arrow keys move the viewing window by 10%
of its width or height, or 30% with Shift. Inputs, text editors, lists, menus and other controls
keep their own keyboard navigation.

An instance or virtual symbol's selection rectangle follows its primary `box` hit exactly, including its
orientation and boxes inferred from graphics when no explicit symbol box was given. The 1.5 CSS-pixel
non-scaling stroke is centred on that boundary (0.75 pixels on either side). The open badge sits
above and to the right of the box, with a fixed 4 CSS-pixel gap in each direction, independent of
zoom. Label margins and hit-testing tolerances are separate and unchanged.

Virtual symbols have independent `usage` objects, including junctions without net pins. Their box
hits put the usage first and retain the associated net for network highlighting. Selecting a wire
or pin still selects the net, marks its pins with circles, and draws no symbol frames; selecting
a usage draws its own box without pin circles or an open-subcircuit badge. Usage labels keep the
net first when present, preserving label clicks and network label highlighting.

Zero and omitted line widths use the renderer's 1.2 CSS-pixel hairline, with SVG non-scaling strokes;
double hairlines use two 1.2-pixel lines separated by a 1.2-pixel gap. Nonzero widths keep their
scene units. This also applies to standalone `rnl render` SVG files. Exported SVGs retain the
original colours and white backdrop regardless of the viewing preference. There is no PNG export
or print command in the viewer. When a standalone SVG is rasterized, hairlines use the rasterizer's
output pixels; printed SVG hairlines use the print engine's CSS-pixel mapping (normally 96 CSS px
per inch, so 1.2 px is 0.9 pt). Printing the viewer page through a browser prints its current display,
including its chosen background, subject to the browser's background-printing option.

## The hierarchy path

The bar above the drawing shows the path of the level shown, from its root down to it, a segment per level: the root's names its level, the top level's title or a subcircuit's name, and every other names its subcircuit with, smaller and dimmer beside it, the instance it was reached through. The path is a visit history of file sources and circuit names, with the instance used for each descent (`state.js`), independent of include resolution:

- Going down into a subcircuit instance, by E or Enter, a double click, the open badge or Edit › Open Subcircuit, adds the instance to the path.
- A subcircuit picked in the index is the root of a path of its own, with no way up from it; tree picks and other navigation, including an object selected in another level’s text, also make the target a new history head.
- A click on a segment but the last goes back to its level, cutting the path there; the up button and Ctrl+E or Backspace go back one level, and at the root there is no up button and Backspace does nothing. The up button that hides at the root gives the focus to the drawing, so that Enter goes back down at once, also in a browser that moves the focus off a hidden element only later, as WebKit does; Enter is left to a focused button only while the button is shown. The level comes back with the view it had when it was left and the instance it was left through selected.
- Reading the document again keeps the visits while circuits belonging to that file still exist by name; otherwise the level shown is the new history head. Visits in other files are resolved when returned to.
- The views kept are by source identity, circuit title and descent instances, so a subcircuit reached two ways keeps a view for each.

The tree marks the row of the path when the path starts at the top level, and else the first row that reaches the level; the index marks the level shown. Where the bar is narrow, the level's kind gives way first, then the levels after the root, from the root's side, behind an ellipsis whose list goes back to them, then the root; the level shown always stays, its name cut short only when nothing else is left to give way.

## Subcircuits that nothing calls

A netlist of subcircuits alone, a library or a testbench that is defined and never instantiated, has a top level with nothing on it, and no instance leads from it to its subcircuits. The page does not leave them to the index:

- **The tree** lists, below the top level, a group *Not called from the top level*. Its roots are the subcircuits that no chain of instances from the top level reaches and that no other such subcircuit calls, in the order of the netlist, each with the subcircuits it calls below it (`Testbench` with its instance `XDUT` of `ota_5t`); a subcircuit that a root calls hangs below it and is not a root too, and may be below several. Of subcircuits that call each other around a cycle nothing outside calls, the first starts the tree. The roots start open. Their rows have the paths `r3`, `r3/0`, ..., `r3` being the root's level, and open a level as a new history head, as rows of the first tree do. The core says nothing of this: `roots.js` works it out from `Level.children`.
- **A document opens on** the top level, also when its objects have no position yet. When the top level has no objects at all (`Level.empty` and no `Level.unplaced`) and the entry file itself defines roots, it opens on the first of those roots that has something to draw, or else the first, and the status bar says so, *The top level is empty; showing Testbench*, until another level is shown. A document read again keeps the level it was on.
- **A level with nothing to draw** says why: *This file is not a netlist. It is shown as text.* when the file opened has the format `text` (`Document.dialect`); *Nothing on this level has a position yet, so nothing is drawn.* when it has objects that have no position (they are in the Not drawn view), and *This level has no objects, so nothing is drawn.* when it has none, with *The subcircuits of this netlist are in the Hierarchy.* when the netlist has any.

## The solver's settings

Solve > Settings (desktop only, where it replaces Solve > Time Limit) is a dialog of the options of `rnl solve` ([`options.rs`](../../crates/rnl-solver/src/options.rs)), which the Solve command then uses:

- **Time**: the time limit of the resolution of each level, in seconds, or no limit (8.5).
- **Placement**: the tolerance (11.7), the grid free coordinates are rounded to, or no rounding, and the beam of the search.
- **Work**: the candidates the search makes over a level, the cells the router expands for one net, and whether to seek every conflict group of the facts of priority low, which only changes the report and can take far longer (`hints_for_low`). The resolution of 8.5 has no count of work of its own: only the time limit bounds it.
- **Objectives**: the 14 objectives of the solver in an ordered list, each with buttons that move it up and down, a check box that turns it on (an objective that is off is not computed), one that puts it in the level of the objective above (the objectives of a level add up with their weights; levels rank in order), its weight, and, on the objective that begins a level, how far from the best a value counts as equal to it, so that the next level decides among those (the tolerance of a level). A badge says each one's level, `1`, `2`, ..., `+` for one added to the level above, `off`.
- **Restore Defaults** sets every field to the solver's default; OK then keeps no change at all.

The dialog says that the priority of a constraint, low, normal or high, is written in the netlist (7.6, 8.5) and not set here: these settings are what the solver does after the constraints are satisfied, the order in which it ranks the placements that satisfy them, and how much work it may do. Under the fields the dialog writes the settings as the command line takes them, `rnl solve --solver-options "beam=8;time_limit=none"`, with `--report` when every conflict group of the low facts is sought.

The page keeps no default of the solver's: the shell gives them (`solverDefaults`, the objectives' names too), so a default that the solver changes is the one shown, and one that was never changed follows it. What the user changed is kept in the shell’s settings store (native `settings.json` on desktop, `localStorage` on web) under `solve-options`, as only the settings that differ, in the form `{ time_limit, tolerance, grid, beam, budget: { candidates, astar_expansions }, objectives: [{ items: [{ name, weight }], tolerance }], hints_for_low }` with `"none"` for no limit or no rounding; a time limit kept under `solve-time-limit` by the earlier menu item counts until the settings are kept. A solve gets them from `solverSettings()` as `{ options, hintsForLow }`: `options` is the text of the settings that differ, such as `beam=8;time_limit=none`, empty for the defaults. OK first asks the shell whether the solver takes the text (`checkSolverOptions`), and the dialog shows its refusal in the solver's words, so the ranges are the solver's alone; the page only checks that a field holds a number.

## Editing the text

The netlist pane can be switched to edit (Edit › Edit Text, Ctrl+Q; the command `toggle-edit`, a check item). Each file of the document then has a text box with its line numbers, in place of the highlighted lines, under the same tabs; the text is plain while it is edited. The files' texts live in `edits.js`, per file by the path the Document names it by: the last saved snapshot, the underlying source baseline, the text the editor *holds*, and the text the Document shown was *built from* (*applied*). Saving records the submitted snapshot, preserving typing made while a save is pending. A download leaves the source baseline intact; a native disk write or replacing a temporary source updates it. Build overlays compare against that baseline, independently of the unsaved mark. Text is kept with `\n` line ends, as a text box gives it, and written back with the line ends the file had.

- **Exiting editing applies changes** through the same `shell.build(source, edits)` rebuild as Apply used previously. Ctrl+Q or Edit › Edit Text toggles the mode; Edit › Apply and Exit Editing (Ctrl+Enter, also the text pane button) applies and exits. With no pending changes, exit does not rebuild. Syntax or build errors appear in Problems and editing still closes; a failed transport keeps pending buffers available for another attempt. The level, drawing view and selection survive rebuilding.
- **Discard Changes** in Edit confirms when there are pending changes, restores the last applied text and exits. Applied but unsaved changes remain available to Save. Entering and exiting preserve the first visible line of each text file, converting by each view's line height, and retain the caret.
- A **solve** first exits editing and applies pending text; it then solves the applied text. A rebuild failure exits editing and stops that solve. A preview is not edited; Close Preview rebuilds the applied text.
- A file **has changes not saved** when its text differs from what was read: its tab has a dot (the colour says whether they are also not applied), the platform is told (`shell.setDirty`) for the window's title and its closing, and Save writes it. Save (File › Save, Ctrl+S) gives the shell the files with changes (`shell.save(source, files, overwrite)`), each with the text as it was read, so that the desktop can see that another program changed the file since: it writes nothing then, and the page asks, in a dialog, to Overwrite, to Reload (reading the files again, which drops the changes made here) or to Cancel; on the web, which cannot write a file, it gives the file shown, which the shell downloads. Save As (desktop) gives the first file, the netlist that was opened, to `shell.saveAs`, which asks where and gives the source of the file written, which is then opened. A source that holds its text, such as the clipboard's, has no file to write to, so Save asks where, as Save As does.
- **Leaving** with changes not saved asks first (`confirmLeave`): opening another file (`openSource`), reading the file again (File › Reload) and the window's closing (`shell.onCloseRequested`, then `shell.quit`). On the desktop the choices are Save, Don't Save and Cancel; on the web, Discard Changes and Cancel, and the browser's own question comes when the page is closed (`setDirty` sets `beforeunload`). Leaving edit applies pending changes; leaving a document or closing the window keeps the existing save/discard/cancel question.

## The objects not drawn

An object that has no position, such as an instance that no `PLACE` fixes before a solver has run, is not drawn on its level, so the drawing alone would hide it. The core lists those of each level in `Level.unplaced` (see [`rnl-view`](../../crates/rnl-view/README.md#the-objects-not-drawn)): in the order of the netlist, each with its kind (an instance, a bundle or a usage such as a ground), its name, its symbol's name, the instances it stands for as indexes into `doc.objects`, and its symbol drawn alone with the name of the net at each of its terminals beside the pin, as an SVG that the core's renderer wrote, so that the page lays out nothing of it. The shells' cores leave those drawings out of the Document, a netlist with no positions listing every object, and the page asks for them with `unplacedDrawings` as the list adds rows (below). The page shows them in three places:

- **The Not drawn view** of the side panel, a tab beside Hierarchy that counts them, `Not drawn (14)`. Each row has the symbol drawn on white, and the name with the kind and the symbol's name below it. The list adds rows 20 at a time, the next when it is scrolled to within 60 pixels of its end (and while its rows do not yet fill it, so that a tall panel shows more at once), with a line at its end that says how many are shown. When it adds a part it asks the shell for the drawings of the rows that have none in the Document, all in one request, by their places in the level's `unplaced` and with their names; a thumbnail is empty until its drawing comes, is not asked for again by a search or a scroll that shows the row again, and if the drawing cannot be had says so in its tooltip while the row works as any other. A Document that carries the drawings, such as a solve's, is drawn from itself and asks for none. Its search matches the names, the part of a name it matches marked; Escape clears it, and Enter picks the first object found. A level with all its objects placed says so, and so does a page with no document open.
- **A device that cannot be drawn** at all, such as a transistor whose model card the netlist does not have (its polarity is told by the card alone, so the standard package has no symbol for it), is in the list after the others, with `Level.unplaced[].reason` (`model card not found`) in place of the symbol's name, an empty box in place of the drawing, and a warning in the Problems list. It is listed whatever its position, since no solver can place what has no symbol.
- **A row picked**, by a click, Enter or Space, selects the instance (as a click on the drawing does, which marks its statement and its name in the text; a subcircuit instance can then be opened with Enter) and shows the netlist with its statement in view and blinking (`revealLine`). An object that no statement of the netlist writes, such as a ground that a record makes, has no statement to show, and picking it does nothing. The list keeps its scroll and its search while the netlist is shown, and starts again, with an empty search, when the level or the document changes.
- **The netlist** greys the lines of the statements of these objects on the level shown, as it dims the text outside the subcircuit drawn: the line's text faded on the grey of `--out-of-scope`, with the title *Not drawn: it has no position yet*, or, for a device that cannot be drawn, *Not drawn: model card not found*. Another level greys its own.

The status bar's line about what the level lacks without a solver says how many are without a position; clicking it shows the view. A solve shown as a preview (`solve.js`) is a Document with the positions the solver found, so the objects it placed are not listed.

## Clicking and pointing

The Document gives each level its `hits` (`rnl-view`): a `box` for each symbol, a `pin` for each pin with its nets, a `wire` for each wire, and a `label` for each text attached to an object, with its bounding box. Each stands for objects, an instance or nets, and none for what cannot be selected. `hits.js` finds what a canvas point is on: a pin or a wire within 6 pixels, a label within 2, the inside of a box, and a box within 6 pixels only where nothing else is. A pin is on its symbol's outline and on its net, so a click on it is on the net, and the boxes it lies on are left out. Of everything under the point, in the order pin, label, wire, box (the nearer or the smaller first, equal ones in the order of the netlist), the first of each hit's objects is what a click selects.

- **A click on one object** selects it, as before. A label stands for what its text is attached to: the instance for its name, its value and the rest, the net for the label of a supply symbol or a wire. A symbol and its labels, a pin and its wire, are one object. A label of a terminal stands for the nets of its pin, as a click on the pin does, so it selects the net and the net shows it marked. A label of something that cannot be selected, such as a terminal at a pin with no net, stands for nothing: a click on it, with nothing else under it, changes nothing, not even the selection.
- **A click on several objects** opens a list beside the pointer (`picker.js`), a row for each with its kind and name, the first being what a click would have selected. Up and Down move, Home and End jump, Enter chooses, Escape closes it and leaves the selection as it was; the mouse over a row makes it the one Enter takes, and a click on a row chooses it. A click anywhere else, the wheel, another level or a document read again closes it.
- **The selection marks the labels attached to it**: a rectangle over each label of the selected object, and its text in the selection colour; a net marks the labels of the terminals on it too. Unkeyed wire labels and repeated unnamed supply symbols have separate display groups, so selecting one net recolours only its labels, in both light and dark backgrounds.
- **The pointer at rest on an instance or a net** shows its name in a box beside it, not under it (`hover.js`): the instance's name over a symbol, the net's over a pin or a wire. A virtual part, such as a ground, a supply, a junction or a port, is a usage of the netlist and stands for the net it is on; it shows its own name when the netlist gives it one (`Hit.name`: `gnd1` for `%gnd1`), and nothing when it does not, as for a ground that the standard package adds. Labels show nothing. A press, the wheel and leaving the drawing take the box away. View › Show Names on Hover turns it off; it is on to begin with, and the choice is kept in the shell’s settings store (native `settings.json` on desktop, `localStorage` on web) as the other settings of the view are.

## The grid

The drawing can show a light grey grid under it, at the integer points of the scene, the usual places of pins: the language's coordinates have x to the right and y up, a unit has no size of its own, and the origin no meaning of its own. View › Grid shows it as dots or lines, or not at all (G shows and hides it), and the axes through the origin, x = 0 and y = 0, a little darker; its dialog, Grid Settings, also sets the spacing, Auto (1 unit) or a number of units, and how often a line is a major one. Where the points would come closer than 10 pixels, the grid takes 2, 5, 10, 20, 50 ... times the spacing instead; with lines, every fifth line is a little darker, or, where that would not be a step of the sequence, the line at the next step of it (every 50 units with a step of 5). The dots and lines are the same size on the screen at every zoom.

The grid is drawn on a canvas under the drawing's SVG, from the view and the level's scale, `Level.scale`, the pixels of the SVG per unit. One row of dots is drawn on a canvas of its own and copied to every row: WebKit copies a canvas drawn onto itself whole first, each time, which made zooming slow and could crash the page. It is not part of the drawing: it takes no clicks, and changes neither fitting, nor the selection, nor the SVG, of which the page only leaves out the white backdrop that the SVG writer puts first, as the canvas is white already. The settings are kept in the shell’s settings store (native `settings.json` on desktop, `localStorage` on web) under `grid-shown`, `grid-style` (`"dots"` or `"lines"`), `grid-spacing` (`"auto"` or a number of units), `grid-major` and `grid-axes`; a value that will not do leaves the default: dots at every unit, and no axes.

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
| `build(source, edits, progress, packages, signal, options)` | Read a source and return its Document; rejects with a message when it cannot. Optional `progress(stage)` reports `reading` or `parsing`. Web API imports can supply request-local `packages` and an abort `signal`; desktop `options.fresh` bypasses cached compilation on explicit reload. With `edits`, `{ path: text }` by the path the Document names a file by, the texts of those files are the ones given, not what is read: Edit > Apply Changes. A source that holds its text takes the text of its one file. The Document's TypeScript declarations are generated from the core's Rust types ([`crates/rnl-view/types`](../../crates/rnl-view/README.md#type-definitions)); `shell.js` refers to them in its JSDoc, which loads nothing at run time. |
| `setCheck(id, checked)` | Show a check-mark command's state in the platform's menu. |
| `onCommand(handler)` | Call `handler(id)` for each command chosen in the platform's menu. |
| `onOpen(handler)` | Optional. Call `handler(source)` for each source opened without `pick`, such as files dropped on the page. Startup awaits listener registration before opening the initial source. Desktop drops use Tauri's native WebView drag/drop events. |
| `fileAssociations()` | Optional. Open Windows Default Apps settings; File > File Associations explains installed and standalone builds before invoking it. |
| `about()` | Optional. Show the platform's About box, for Help > About in the page's own menu bar; without it the page shows a plain one. A platform whose menu shows its own About box has none. |
| `browse(url)` | Optional. Show a web page outside the viewer: the web shell opens it in a new tab, with no hold on the page (`noopener`); the desktop shell opens it in the default browser through Tauri's opener plugin, which the app's capability lets open the repository's pages on GitHub only. Help > Desktop Viewer (web) shows the project site's page of the tools with it, and Help > Check for Updates (desktop) a release's page; without it, those commands do nothing. |
| `solve(source, settings, edits)` | Optional. Solve a source as `rnl solve` solves it, without its records of the solver, and give `{document, report}`: the Document of the files as solved, nothing written, and the report of [`rnl_cli::preview`](../../crates/rnl-cli/src/preview.rs), every line of it located in that Document's files. `settings` is `{ options, hintsForLow }`: the options that differ from the solver's defaults as `rnl solve --solver-options` takes them (empty for the defaults), and whether to seek every conflict group of the facts of priority low; `edits` are the texts applied, as `build` takes them. Rejects with a message when it cannot, and with `the solve was cancelled` when `cancelSolve` stopped it. The desktop shell solves natively on a thread of the app's; the web shell has none, so the Solve menu, whose commands the table offers on the desktop only, and the Solve view are not shown there. |
| `cancelSolve()` | Optional. Stop the solve under way. |
| `unplacedDrawings(level, items, names)` | Optional. The drawings, `{ svg }` each, of the objects at `items` of `doc.levels[level].unplaced` of the Document `build` gave last, in order: the core keeps them from that build, and `names` are the objects' names, which it checks, so that a request about a Document that was built since is refused with a message. A shell whose Documents carry the drawings leaves it out. |
| `solverDefaults()` | Optional, with `solve`. The solver's options as it has them without any setting, and the names of its objectives, for Solve > Settings. |
| `checkSolverOptions(options)` | Optional, with `solve`. Whether the solver takes `options`, written as `--solver-options` takes them: null when it does, else what is wrong, in the solver's words. |
| `saveSolved()` | Optional. Save a copy of the last solve shown where the user says, never over a file that was read: what was saved, in a sentence, or null when the user cancelled. |
| `onSolveProgress(handler)` | Optional. Call `handler({level, levels, title, again})` as the solve under way begins each level. |
| `save(source, files, overwrite)` | Optional. Write the files, `{ path, name, text, read }` each (`read` is the text as it was read, with `\n` line ends), where they were read from, because the user chose File > Save, and say in a sentence what was written. Unless `overwrite`, a file on disk that is no longer what was read, because another program changed it or took it away, is not written, nor are the others: `{ changed }`, their names, comes back, and the page asks (Overwrite, Reload or Cancel). The web shell, which cannot write a file, downloads them and checks nothing. Rejects with a message when it cannot. The page gives it only the files with changes not saved, and on the web the file shown. |
| `saveAs(source, file)` | Optional, the desktop's. Ask where to save the netlist that was opened, write its text there and give the source of the file written, or null when the user cancelled. |
| `setDirty(dirty)` | Optional. Whether files have changes not saved, for the window's title and for closing it: the desktop shell tells the app, the web shell sets the browser's `beforeunload` question. |
| `onCloseRequested(handler)` | Optional. Call `handler()` when the user asks to close the window while `setDirty(true)` was the last word: the window stays until the page calls `quit()`. |
| `version()` | Optional. The version of the app that runs, which the desktop shell has from Tauri, for Help > Check for Updates, the command `check-updates` that the table offers on the desktop only. The page asks GitHub's API for the releases itself (`updates.js`), as the desktop app's content security policy allows, and only when the command runs; without `version`, the command does nothing. |

A Source is the shell's own object: the page keeps it to read it again and shows its `name`. Its optional `path` identifies a real file in the Library; native backslashes are normalized to forward slashes for matching and deduplication. On the desktop it is a file path; on the web, files chosen or dropped, a folder or a URL, each with a way to read its entry and its other files. A URL updates its workspace path after redirects. The source that the page makes for File › Load from Clipboard holds its `text` and is marked `temporary`: each shell builds that text, with the file pool available, as the file `name`, such as `clipboard.sp`, whose extension says how it is read: the desktop with the app's `open_text` command, the web in the core's Worker.

The web shell takes each candidate from a prewarmed standby module Worker, adopts its lazy drawing provider only when the document is displayed, and recycles the previous or rejected candidate as the next standby. Superseded builds terminate their worker. Each worker retains its Library index; changed files are synchronized before reuse, and Reload Library invalidates both indexes on their next use. It prepares a standby in the background when none can be reused; `ready` waits for the standby actually used by the first open. The displayed document stays isolated so a rejected build cannot replace its lazy drawings. It loads the core as WebAssembly from `core/client.js` next to the page ([`bindings/runtime-wasm`](../../bindings/runtime-wasm/README.md), which [`apps/web`](../web/README.md) puts there), and asks for the SVG only. `collect` reads the files a document needs, decoded with `decodeText`, each named in the workspace by a rooted path: files chosen with Open or dropped at `/NAME`, a folder's at `/FOLDER/SUB/NAME`, and a URL's by their paths on the server that served the netlist, after any redirect, fetched relative to that address; a server that sends nothing for 30 seconds is given up. When more than one file could be the netlist, it asks which with `dialog.js`. It names the browser tab after the document, as the desktop app names its window. Its file inputs have the ids `file-input` and `folder-input`, for tests: files set on one while no chooser is shown open as if chosen with Open File or Include Folder.

## The command table

`commands.json` lists the menus, each with its `label` and `items`. An item is `"-"`, a separator, shown only between two items; a submenu, `{ "label", "items" }` as a menu has, at any depth, shown only with an item the platform offers; or a command:

| Field | |
| --- | --- |
| `id` | What `command(id)` runs. `exit` and `about` are the platform's own: the desktop app quits, or shows its About box; the page runs `about` through the shell's `about()`, which the web shell has, or shows a plain About box when the shell has none and the page shows the menu bar. `desktop-viewer`, which only the web offers, shows the project site's page of the tools, where the desktop viewer is downloaded, with the shell's `browse(url)`: the page in the first of the browser's languages that `i18n/languages.json` offers, by its `matches`, such as [the Chinese page](https://rich-netlist.github.io/rnl/zh/tools.html), else [the English one](https://rich-netlist.github.io/rnl/en/tools.html), the site's default. |
| `label` | The text, with `&` before the underlined letter. |
| `english`, `icon` | On a menu or a submenu: `english`, an English word that follows its translated label in parentheses when the interface is not in English ("Sprache (Language)"), and `icon`, the name of a picture the page's menu bar shows before it (`globe`). They are on the View menu and its Language submenu, so that a user who switched to a language they cannot read finds the way back. The word is no text of the catalogs, which the scan of `status.py` does not read from this key. The names of the languages in the submenu are not touched. The native menu of the desktop has the word, and no picture, as a submenu there has none. |
| `keys` | The keys that run it, written like `Ctrl+Shift+M`, `F5` or `Enter`; Ctrl stands for Cmd on macOS; `Control` means the physical Control key on every platform. The first is the native menu's accelerator. |
| `shown` | How menus show the keys, when not as the first of them. |
| `check` | Whether it has a check mark. |
| `radio` | Whether, with `check`, it is one of a group of which the page checks one: the page's menu bar shows a dot, a native menu a check mark. |
| `page` | Whether the page always handles its key, as for Enter, Backspace and G: no menu accelerator would take the key from a text box. |
| `platforms` | The platforms that offer it; all when not given. |
| `windowsOnly` | Offer this command only on Windows, in addition to its `platforms` restriction. |

The desktop app builds its native menu from the table ([`src/menu.rs`](../desktop/src/menu.rs)), its submenus too: on Windows the page handles the keys and the menu shows them after a tab; elsewhere the menu's accelerators handle them. A native menu toggles a check item when it is chosen, so the page sets the marks of a command's group again after running it, whether or not they changed. In the page's own menu bar, pointing at a submenu's entry opens the submenu beside it; from the keyboard, ArrowRight, Enter or the entry's letter opens it, and ArrowLeft or Escape closes it. The page runs keys by the same table. A key with Ctrl compares Shift only for a letter, since typing `+` may need Shift; a key without Ctrl that types or deletes does not count while Ctrl or Alt is held. In a text box or list only Ctrl keys and function keys count. E and Enter open a selected subcircuit instance, Escape clears the selection, and F5 reads the document again instead of reloading the page. Ctrl+R toggles usage overlays.

## Debug handle

Module scope hides the page's state from tests and the console, so `window.viewer` shows a little of it:

| Member | |
| --- | --- |
| `doc` | The Document shown, or null. |
| `level` | The level drawn, an index into `doc.levels`. |
| `path` | Visit history: `{ level, via }`; level is its index in the current entry, or -1 for a visit in another file. via names the descent instance, null for the head. |
| `selected` | The selected object, an index into `doc.objects`, or null. |
| `edits` | `{ unsaved, unapplied }`: the paths of the files whose text is not saved, and of those whose text is not applied. |
| `command(id)` | Run a command, as its menu item does. |
| `checked(id)` | Whether a check-mark command, such as `toggle-netlist`, is checked. |
| `grid` | The grid's settings, `shown`, `style`, `spacing`, `major` and `axes`, and the steps it is drawn at in units, `step` and, for lines, `majorStep`; null when it is not drawn. |
| `menuBar(shown)` | Show or hide the page's own menu bar, whatever the shell says. |

## File pool and settings storage

`pool.js` manages the Library tab (the stable IDs remain `tab-files`/`pane-files`): Files, ordered desktop PATH groups, and immutable Built-in packages. Opened files join Files; Include only adds, and Exclude leaves an open root visible. Files starts expanded, PATH/Built-in collapsed; collapse state stays in memory. A separate single temporary row retains clipboard content and applied edits without entering search. Root-file rows highlight across groups, independently of the source file shown in Netlist. Shells supply `bundledFiles`, `include`, `setPool`, a web `poolItem`, and (desktop only) `pickPath`/`pathPackages`; native PATH enumeration lists recognized netlists as well as `.rnl` packages, directly and in numeric version subdirectories. Package lookup keeps its existing directory precedence. PATH rescans when Library is shown and after PATH changes; `onInclude` delivers dropped folders and test file-input selections. Search changes rebuild the applied document without dropping pending edits. Built-in sources carry `readonly`, which editing, saving and solving honor.

Library file names stay on one line, with an ellipsis hiding the beginning of a long path as the sidebar narrows. Hovering a file name shows its complete path.

**Parse formats.** Each netlist row of Files and PATH ends with a chip naming the format the file is read in (SPICE, Spectre, ...): muted while the file reads by default, solid when the user chose it. The files the netlist shown includes are listed under its row, each with its chip. A click opens a menu of Automatic, with the format the core detects, and the formats of the core's list (`Document.formats`, or `shell.formats` before a Document); the page writes no format names of its own. A choice rebuilds the document as other Library changes do, keeping edits. `formats.js` keeps the choices under `file-formats`, `{ path: format }`, by the file's path as the core names it (`Document.path` joined with `SourceFile.path`), folded where the platform ignores case (`shell.formatKey`); a web page keeps a choice only for a file a server served, by its address, and the others while it is open (`shell.keepsFormat`). Every build, preload, solve and Library index request carries the choices whose names the core lists. A row the document shown does not read shows the format `shell.detectFormat` gives; `.rnl` packages and built-ins have no chip.

`util.startSettings(shell)` runs before any initial view state is applied. Desktop shells supply `loadSettings(legacy)` and `saveSetting(key, value)`; the native settings file is authoritative after one successful migration. Writes are serialized and native builds/solves await them. Web and test shells without these methods keep using browser storage. The Library PATH groups and other settings use this one store. See the desktop and web READMEs for lifetime and lookup semantics.

## Navigation and new windows

The unmodified number keys open the sidebar and select its view: **1 Library, 2 Netlist, 3 Hierarchy, 4 Symbol, 5 Usage, 6 Not drawn, 7 Solve** (desktop only). Each tab carries its number at the upper left. View > Side Panels and Help > Keyboard Shortcuts list them too. These page-handled keys work from the drawing, tabs and navigation lists, but leave text inputs, selectors, editors, dialogs and IME composition alone. The desktop Solve tab is available before any solve and shows an empty-state hint; key 7 never starts a solve (F9 does).

**Key diagnostics.** With `?debug=keys` in the page's address, a corner of the page lists the last ten keys and focus changes: each key's `key` and `code`, its target, the focused element, whether the page had focus and whether something prevented it, and what became of it: the command it ran, or why none ran (taken before the page's handler, by the menu bar, kept until startup ends, typing in a text box, in a list, no command for the key, the command table not loaded yet). The listeners sit on `window` in the capture phase, from the top of `index.html`, so a key that an extension takes earlier shows as having reached no handler of the page. Without the parameter the page is unchanged. When a shortcut does nothing, open the page with `?debug=keys` and press it: if the list does not show the key at all, something outside the page took it, so try a private (InPrivate, incognito) window, which runs without extensions, or turn the browser's extensions off one at a time. A user's number keys that did nothing in Edge after a reload were found so: an extension took them, and they worked in a private window.

Every user action that changes the displayed circuit must call `navigate(target, { newWindow })` in `navigation.js`. Do not call `showLevel` or change the hierarchy path directly from a new click or key handler. Rendering/rebuilding and restoring a session are the internal exceptions. A target identifies its source, visit history and, when needed, an object to select. Crossing to a circuit owned by another file reopens that file as an independent entry and adds it to Files Library; only its own includes and the current Auto Include setting supply definitions. The history carries sources and circuit names, never a compiled document or an include closure. Only descent appends to it; tree/index/text/file navigation starts a new head. Ascend and breadcrumb clicks restore the corresponding prefix and view. Reload preserves valid visits, dropping a stale chain if a same-file ancestor was removed. Instance double clicks, the open badge, E/Enter, Ctrl+E/Backspace, path segments and their overflow menu, Hierarchy rows, Subcircuits rows, Library files and cross-level selections in the text all use this entry point. Not drawn rows select objects on their existing level and have no cross-level jump.

Hold Shift during any of those actions to open the target in another viewer and preserve the original circuit, path, selection and buffers. Shift+E and Shift+Enter run Edit › Open Subcircuit in New Window…, enabled only for an openable selected instance. File › New Window copies the current source, path, selection and Library pool. Library file clicks retain the Library tab. Help › Keyboard Shortcuts lists the new keys. Ctrl+Q uses the physical Control key on macOS too, avoiding Cmd+Q's system Quit binding; other Ctrl shortcuts retain the existing Cmd mapping.

Desktop shells start the same executable as an independent process, passing a temporary JSON file with `--session FILE`. The child reads and deletes it, restores its Library and target, and opens 24 logical pixels from the parent. Disk sources are read from disk; applied unsaved changes trigger the specified warning before opening. Clipboard sources retain their applied text. Settings are read once at startup; each changed key is persisted by a locked read/modify/replace of `settings.json`, so independent writers preserve other keys. Startup skips unchanged settings writes.

Web shells synchronously reserve a new tab with `window.open`. A random one-use token, an exact origin check and matching opener/child references bind the ready/session `postMessage` exchange. Local files and the pool are serialized as text; applied document text takes precedence and unapplied editor text stays behind. The child drops its opener after receiving the session. A blocked pop-up or expired handoff is explained in a dialog. Sidebar context menus are suppressed except in editable controls.


### Auto Include and Reload Library

File > Auto Include and the Netlist checkbox share a persisted setting, on by default. Normal explicit includes always resolve first. With Auto Include enabled, undefined subcircuit instances use a session index of the current Library files (including desktop PATH netlists). New library files are indexed once using the host parser, with SPICE's case-insensitive and Spectre's case-sensitive names. Repeated lookups do not rescan unrelated files. Unique definitions are imported using the normal include reader, including relative dependencies and model cards; dependencies can themselves be resolved from the index. Source text and line numbers are preserved, and each automatically resolved subcircuit has a warning naming its source file in Problems. The same configurable provider is used for native Solve.

Multiple definitions, including two in the same file, are reported as conflicts rather than chosen by PATH order. Auto Include also refuses a file that would redefine a subcircuit already loaded or selected in the same pass. Explicit includes choose the intended definition; sectioned libraries still require an explicit section selection. An included definition reached through several Library items counts as one definition.

When a Library item discovers a definition through an include, the index also checks and caches the defining file as an independent import target. Auto Include imports that file and its dependencies, rather than the enclosing testbench, so it does not bring unrelated top-level circuits or a back-reference to the opened file into the document. A definition reached through an explicitly selected library section retains the selecting wrapper. These import targets follow the same refresh, reload and applied-edit invalidation as the Library index.

File > Reload Library rescans PATH, rereads all current Library files (packages included), removes stale or unreadable definitions, rebuilds the index and the applied document, and preserves unapplied editor text. Failed reads appear beneath Library. Applied library edits use a document-local index overlay, without contaminating the saved session index. The web shell caches library text and indexes it in its Worker when included; Reload Library rereads available handles and refetches public packages. Browser-selected File objects remain snapshots: reselect changed local files to supply their new contents.

### Source display and folding

Netlist shows the current source path as plain text, with a shrinking directory portion and the filename kept at the end. It follows circuit navigation; explicit source navigation through Problems or include links shows that file instead. Controls occupy the row below the path and wrap in narrow panes. The filename's unsaved marker covers changes in any source file; its tooltip names them. Resolved explicit include filenames are links, using the host parser's resolved file identity (including selected library sections). Clicking opens the included file independently, including model/parameter-only files; Shift opens that target in a new viewer. Neither action inherits the parent file’s include context.

Selected netlist names change text colour (including their syntax pieces), without a highlight fill. Active RNL records have a light blue background; lines with parser-recognized free text have a deeper blue. **RNL only** shows RNL lines; its subordinate **Semantics only** option narrows them to free-text lines and is disabled when RNL only is off. Quoted parameters, comments and disabled records do not count as free text. **Failed statements** controls visibility of grey failed/overridden rows and undrawn host statements in the current circuit. Filters preserve original line numbers and virtual scrolling; Find and copy use the filtered rows. Entering Edit Text disables filtering so editing shows the complete file. Filter preferences persist across restarts.

The **Symbol** sidebar shows the symbol actually used by the parent instance when descending, or the subcircuit's own block symbol when entering directly from the index. Its embedded SVG viewer supports drag, wheel zoom, reset/zoom buttons, and Box, Origin, Pins and Labels toggles without changing the main drawing. Pin/label lists follow it; the original definition starts with an abbreviated source path and line range. Symbols are identified by runtime identity, so equally named definitions and overrides stay distinct. Nested usages contribute drawing graphics, not extra outer terminals. Generated default symbols say that they have no RNL source. The Document carries one preview per unique symbol (equivalent generated defaults share a preview); inspecting a symbol does not rebuild or solve the circuit. Not drawn's tab count counts undrawn host instances, excluding decorative usages and port markers, and disappears at zero.

Source models, syntax checkpoints and rows are prepared only when a file is viewed. Subcircuit blocks have guide lines and +/- gutter buttons, including definitions in unselected library sections. Folding preserves original line numbers and full-source copy; find and diagnostic navigation expand hidden destinations. Virtual scrolling maps visible rows to logical lines. Unchanged reloads keep folds, and edit mode shows the complete buffer at the corresponding source line.

Selecting an existing Library/PATH row reuses its index. Include of a replacement file and Reload Library invalidate definitions; an auto-include pass validates a library's complete definition list only once even when many missing masters come from it.

### Side Panels menu

View > Side Panels marks only the currently visible pane with a check. Hide collapses the sidebar, clears all pane marks and is disabled while collapsed. Tabs, number keys and restored layout update this same state. Ctrl+B still toggles the sidebar and Ctrl+Shift+M folds Problems; these are shortcut-only commands, separate from the menu entries in `commands.json`.

### UI languages

View > Language switches the language immediately, in both the web page and desktop native menus: English, and each language that [`i18n/languages.json`](../../i18n/languages.json) shows (`published`) and that has a catalog of the viewers, `i18n/<lang>/viewer.json`, by its name in itself (decision 0015). The command table's Language submenu, marked `"languages": true`, holds English; the page and the desktop app add the others, as `language-<code>` of the registry, such as `language-zh`. The first start, with no choice saved, takes the first language in the browser's or the system's list of languages (`navigator.languages`, which the desktop's WebView gets from the system) that is offered, else English, as the site's root does: a browser set to Italian and then German gets German; a user choice is saved with the other viewer settings. Embeds use their isolated settings and system language. Changing language only refreshes bound UI text and attributes: document identities, rendering, unsaved buffers, selection and viewport stay intact.

`i18n.js` uses English source text as a stable catalog key and fallback. It takes the languages and their catalogs from the shell's `uiLanguages()` and `uiCatalog(code)`, which the desktop app builds into itself from `i18n/` ([`build.rs`](../desktop/build.rs)), or else reads them from `locales/languages.json` and `locales/<code>.json`, which the web viewer's build writes and the page tests' server serves ([`scripts/i18n/viewer-locales.mjs`](../../scripts/i18n/viewer-locales.mjs)). A language is known by the code the viewers save in their settings, such as `zh-CN` (the registry's `codes.viewer`); a text the catalog has no translation for, or `null`, shows in English. A language whose catalog cannot be read is left out, and the page starts without it; without the list of languages it starts in English alone; a line in the status bar says so, in English whatever the interface's language, as every notice about translation itself (decision 0015), naming the language by its English name (`english`, the registry's `names.en`). It is no text of the catalogs. The desktop app builds its catalogs in, and never leaves one out. The first start walks the system's or browser's languages in their order and takes the first that any language's `matches` fit (`preferredLanguage`), else English; of the languages whose `matches` fit one entry, the one that fits it longest, so that `zh-TW` takes a Traditional Chinese that names `zh-tw` rather than Simplified Chinese's `zh`, as the site's root does. Mark UI messages with `m("Text")` or a tagged template such as `` m`Open ${filename}` ``; a string's placeholders `{0}`, `{1}`… take the values after it, as in `m("{0} errors", n)`. A text that reaches `m()` through a variable, such as a dialog's button label or a name in a table, is written `tr("Text")` where it is defined: `tr` returns it unchanged and marks it for the scan of `python scripts/i18n/status.py`, which reads every text of the interface from these forms and reports a catalog entry the interface no longer has ([`i18n/`](../../i18n/README.md)). Write a count as whole phrases, one for each form the English has (`m(n === 1 ? "{0} line" : "{0} lines", n)` with `tr` for both), not as a number and a word, since other languages count differently. Use `setText`, `append` and `setAttr` to bind the message to a text node or attribute. Keep the outer template marked when nesting translated fragments. `setAttr(node, name, null)` removes an attribute and its binding. The initial static HTML is bound once, before any user document is loaded. Desktop menu labels and native file chooser titles use the same catalog.

Keep action tokens, menu IDs, shell return types, solver options, netlist/source text, filenames and parser/solver diagnostic payloads raw. Translate their surrounding UI only; dialog buttons render a translated label but return the original action token. Do not construct HTML from translated strings. Add catalog entries for new messages and run `i18n.spec.js`, plus the affected UI tests. Real web Save and the desktop smoke test cover the shell boundaries.


### Idle preloading (desktop)

File > Memory Cache also controls **Preload nearby netlists while idle**, enabled by default.
After 1.5 seconds without pointer, wheel or keyboard input, the scheduler walks the currently
shown subcircuit tree breadth first (direct children first), then the Library/PATH netlists.
It deduplicates cycles/files and limits each plan to 128 candidates. This reads, interprets and
renders each candidate as an independent entry with the current Auto Include setting: visit
history and the parent file's includes never supply its scope. Preparation neither navigates nor
adds hypothetical files to Files. It does not invoke the placement solver or write files.

One separate native compiler runs at a time, using Windows background processing priority.
Between jobs it rests for at least 1.5 seconds and three times the preceding job's duration,
targeting roughly 25% of one core across repeated jobs, not a hard instantaneous CPU limit.
New jobs pause while interacting, hidden, editing, unsaved, solving or displaying a dialog/preview.
Foreground navigation and library changes invalidate in-flight results. A compiler already
interpreting can finish at background priority; foreground commands never wait for it.

Completed JSON, SVG and lazy Not drawn data transfer into the existing bounded cache. WStr's
immutable strings use Arc so the exported drawing model is Send; interpreter contexts remain
thread-local. Speculation uses spare cache space only and is evicted before visited entries.
Cache hits still validate complete observed inputs, including missing files, package resolution
and Auto Include conflicts. Clear/disable, reload and generation checks prevent old work from
repopulating invalidated results. Cached hits are reactivated only by an actual open.

Speculation pauses below 1 GiB available system memory, skips inputs exceeding min(8 MiB,
remaining cache budget / 8), and stops the current plan when cache space is exhausted. These
limits bound retained results and speculative input, not the compiler's transient working set.
Oversized files still open normally. The web shell continues using its existing Worker; it does
not expose native PATH preloading or its memory-cache controls.

## Netlist colour scheme

View > Color Scheme opens the twelve Netlist colour categories and the drawing's second highlight, each with a hex field and a native colour picker. Valid edits preview immediately; Cancel/Escape restores the previous palette, Save stores one `netlist-colors` object, and Restore Defaults resets the draft until saved. The desktop keeps it in its user settings JSON; the web keeps it in local storage. Invalid or missing saved entries fall back individually to defaults. The palette uses explicit colours independently of the UI theme. Its *Selection highlight* is the Netlist's; the drawing keeps its own. Below it, *Subcircuit selection highlight (drawing)* is the colour of a selected subcircuit instance that can be opened, on the drawing only (`--sub`, with its fill), #C2255C by default; the drawing lightens it on a dark background as it does its normal highlight. The rectangle of a right-drag zoom keeps the normal highlight whatever is selected.

Component names and `#` references share one colour; host nodes, subcircuit ports and `&` references share another. Parameters, `?role` references and role bindings use the parameter colour; free RNL words have their own category. Numbers, punctuation, directives, comments and unclassified text use Other text. Semantic RNL rows derive a stronger tint from the RNL background, and inactive/scope-excluded rows retain dimming. The same syntax palette is used for symbol definition source and plain editor foreground/background. Applying colours changes CSS variables only, preserving source buffers, virtual rows, scroll position and compiled drawings.

## Usage inspection

Panel 5 lists only usages of the current drawing scope, grouped by definition with counts and mixed-state visibility checkboxes. Panel 6 is Not drawn; panel 7 is the desktop Solve report. View > Show Usages (Ctrl+R) gates individual visibility without changing the checkboxes. Usages with no drawing objects of their own or inside their bodies default on. Usages with symbols, wires, routes or annotations of their own or in descendants default off, including virtual parts and drawing wrappers such as GROUND_STUB. Selecting a usage in the list or source temporarily forces its overlay on; selecting something else restores the switches. Hidden usages cannot be picked as usages on the drawing: their virtual symbols keep ordinary symbol selection, without a usage frame or focus effect. Reload preserves which selection mode was used. Color and visibility are Viewer display preferences, stored with the source path, scope and runtime identity (up to 2,000 recent entries), not written into RNL.

Only visible role members contribute to a semantic usage's frame. A virtual usage whose roles are local net/terminal/point connections uses its own symbol's box and keeps focus local; a ground or supply's net role must not enclose every symbol on that net. Whole component roles keep their original geometry and role centers, including when the usage has a symbol. The dotted round frame is 1pt with 10% fill; the selected frame is 2pt with 20% fill. Unrelated object groups retain their alpha and shift their RGB halfway toward white on a light background or black on a dark background. The frame title is `type: name`; an unselected title is hidden (and has no hit area) until it fits inside the frame. A selected title always shows. Titles and selected role names stay 13 CSS pixels through pan and zoom. Role names are centered on their role extent or local symbol pins with translucent white or black paper matching the drawing background. Local roles with shared or unavailable pin geometry use the symbol center, with colliding badges separated vertically. Source navigation, reload and new-window sessions retain the usage inspection mode. Clicking a visible border or title selects the usage and marks its source statement; the filled interior retains ordinary component picking.

The usage list is built only when its pane is visible. Types with more than 100 usages initially fold, and expand in batches of 100 with Show more. Screen text widths are measured once per overlay, and selecting ordinary components preserves existing usage overlay nodes.
