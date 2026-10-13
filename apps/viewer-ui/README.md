# viewer-ui

The page of the RNL viewers: the drawing, the netlist text, the problems and the subcircuit hierarchy, with a selected instance or net followed between the drawing and the text. The desktop viewer ([`apps/desktop`](../desktop/README.md)) shows it now, and the web viewer will share it ([decision 0006](../../docs/decisions/0006-shared-core-per-platform-frontends.md)). The page shows a Document that its shell builds; it never reads files itself.

It is plain HTML, CSS and ES modules, with no dependencies and no build step. It keeps to the browser baseline of decision 0006, Chromium 109 and Safari 16, so newer features such as JSON import attributes, CSS nesting or `Array.prototype.toSorted` are not used. `package.json` only makes Node read the scripts as modules, so that they can be checked:

```bash
find apps/viewer-ui -name '*.js' -print0 | xargs -0 -n 1 node --check
```

## Modules

| Path | Content |
| --- | --- |
| `index.html`, `style.css` | The page and its styles, light and dark. |
| `main.js` | The start: the stored layout, the shell, the command table, the first document, and the debug handle. |
| `shell.js` | The shell interface, and choosing the shell. |
| `shells/desktop.js` | The desktop shell, the only module that uses `window.__TAURI__`. |
| `state.js` | What the page shows: the Document, the level drawn, the selection, and the hierarchy path that led to the level. Modules read these bindings and change them through setters. |
| `open.js` | Opening a source through the shell; reading it again keeps the level, the view and the selection. |
| `drawing.js` | The drawing of a level: the view, moved by changing the SVG's `viewBox`; what a click hits, from the boxes, pins and wires the Document gives; the selection marks and the open badge. |
| `selection.js` | Selecting an instance or a net. |
| `subcircuits.js` | Going down into a subcircuit instance and back up, each level with the view it had. |
| `text.js` | The text pane: the lines of each file, the statements that do not take effect, the places of the selection, revealing a line, and clicks in the text. |
| `syntax.js` | Highlighting the host statements and the RNL records. |
| `problems.js` | The problems list and the coloured line numbers. |
| `hierarchy.js` | The hierarchy view: the instance tree, and the subcircuit index with its search and initials. |
| `panels.js` | The side panel's views, what is folded, and the splitters. The layout is kept in `localStorage` under `sidebar-shown`, `sidebar-width`, `side-pane`, `problems-shown`, `problems-height`, `index-height` and `show-failed`. |
| `commands.js` | Running a command by its id, and the keys. |
| `menubar.js` | The page's own menu bar. |
| `util.js` | Small helpers for building the page. |
| `commands.json` | The command table. |

The modules import each other in cycles. At its top level a module only looks up its elements, keeps its own state and adds listeners; it calls nothing of another module there, so no imported binding is read before its module has run.

## The shell

Everything platform-specific goes through the shell, chosen at start: `shells/desktop.js` when the page runs in the desktop viewer (`window.__TAURI__` is there), otherwise the shell that `?shell=NAME` names, `web` by default, which is not written yet. `shell.js` documents the interface:

| Member | |
| --- | --- |
| `platform` | `"desktop"` or `"web"`: which commands of the table it offers. |
| `nativeMenu` | Whether the platform shows the menu bar itself; if not, the page shows its own. |
| `initial()` | What to open at start, or null; on the desktop, the file on the command line. |
| `pick(kind)` | Ask for something to open: `"file"`, and on the web also `"folder"` or `"url"`; null when nothing is chosen. |
| `build(source)` | Read a source and return its Document; rejects with a message when it cannot. |
| `setCheck(id, checked)` | Show a check-mark command's state in the platform's menu. |
| `onCommand(handler)` | Call `handler(id)` for each command chosen in the platform's menu. |

A Source is the shell's own object: the page keeps it to read it again and shows its `name`, and knows nothing else of it. On the desktop it is a path; on the web it will be files held in memory, a folder or a URL, and later a text being edited.

## The command table

`commands.json` lists the menus, each with its `label` and `items`. An item is `"-"`, a separator, shown only between two items, or a command:

| Field | |
| --- | --- |
| `id` | What `command(id)` runs. `exit` and `about` are the platform's own: the desktop app quits, or shows its About box. |
| `label` | The text, with `&` before the underlined letter. |
| `keys` | The keys that run it, written like `Ctrl+Shift+M`, `F5` or `Enter`; Ctrl stands for Cmd on macOS. The first is the native menu's accelerator. |
| `shown` | How menus show the keys, when not as the first of them. |
| `check` | Whether it has a check mark. |
| `page` | Whether the page always handles its key, as for Enter and Backspace: no menu accelerator would take the key from a text box. |
| `platforms` | The platforms that offer it; all when not given. |

The desktop app builds its native menu from the table ([`src/menu.rs`](../desktop/src/menu.rs)): on Windows the page handles the keys and the menu shows them after a tab; elsewhere the menu's accelerators handle them. The page runs keys by the same table. A key with Ctrl compares Shift only for a letter, since typing `+` may need Shift; a key without Ctrl that types or deletes does not count while Ctrl or Alt is held. In a text box only Ctrl keys and function keys count. Enter only opens a selected subcircuit instance, Escape clears the selection, and F5 and Ctrl+R, whatever else is held, read the document again instead of reloading the page.

## Debug handle

Module scope hides the page's state from tests and the console, so `window.viewer` shows a little of it:

| Member | |
| --- | --- |
| `doc` | The Document shown, or null. |
| `level` | The level drawn, an index into `doc.levels`. |
| `selected` | The selected object, an index into `doc.objects`, or null. |
| `command(id)` | Run a command, as its menu item does. |
| `menuBar(shown)` | Show or hide the page's own menu bar, whatever the shell says. |
