# desktop

RNL Viewer, a small desktop application that opens a netlist and draws it. It only reads: it neither edits nor solves, so objects without a position are not drawn, as with `rnl render`.

- **File › Open** (Ctrl+O) opens a SPICE netlist, a Spectre netlist (`.scs`) or a package file (`.rnl`); the file can also be given on the command line. **File › Reload** (F5) reads it again, keeping the level, the view and the selection.
- The drawing shows one level at a time; its name is above it. It comes from the same runtime and renderer as `rnl render`, with the bundled standard packages. Drag with the left or middle button to move the view; scroll to zoom around the pointer. **View › Fit to Window** (F, or double-click) shows the whole level.
- The side panel has two views. **Netlist** shows the netlist and the files it includes, with RNL and the host statements highlighted. Below it, the **Problems** list shows the errors, warnings and hints; clicking one shows its line. Line numbers are coloured where a line has a diagnostic. **Hierarchy** shows the subcircuit instances as a tree under the top level, and below it every subcircuit by name, grouped by initial, with a search box; the top level is always first. Picking a subcircuit in either opens its drawing, and both mark the one open. The side panel folds (View › Netlist Panel, Ctrl+B), so does the problems list (View › Problems, Ctrl+Shift+M), and the borders between the parts can be dragged within limits.
- **Selection follows an instance or a net between the drawing and the text.** Clicking a symbol (anywhere in its box) selects its instance; clicking a wire or a pin selects the net, and clicking a ground or port symbol selects the net it stands on. The text then marks the instance's statement and every place that names it: its name, the nodes of a net in the statements and subcircuit headers, and the `#name` and `&net` references in the records. A terminal reference such as `#M1:d` belongs to both: its `#M1` to the instance, all of it to the net at the terminal. The other way round, clicking an instance's statement or a node in the text selects it on the drawing, switching to its level if needed. Clicking an empty place or pressing Esc clears the selection.
- **A selected subcircuit instance can be opened.** It is marked in its own colour, with an open badge on the corner of its box. Clicking the badge, double-clicking the instance (on the drawing or in its statement), pressing Enter, or View › Open Subcircuit shows the subcircuit's drawing. While a subcircuit is shown, the button on the right of the bar above the drawing, Backspace, or View › Up to Parent Level goes back up along the hierarchy path that led there, with the instance selected and the view as it was left.
- **Statements that do not take effect have a soft background** in the text: rose for those that failed with an error, cream for those overridden in a conflict (7.5): dropped in the order of 8.5, or a symbol that loses to later statements on every object. Only the failing part of a pipe is marked. The *Failed statements* box above the text turns the marking off and on. Statements in styles that later ones override property by property are not marked.
- The status bar shows the selection, what the shown level lacks without a solver (objects without a position, and constraints that need a solver or could not be checked), and the zoom.

## Running

From the repository root:

```bash
cargo run --manifest-path apps/desktop/Cargo.toml -- tests/conformance/cases/03-five-transistor-ota.sp
```

The menus and their keys come from the page's command table ([`commands.json`](../viewer-ui/README.md#the-command-table)). On Windows the page handles the shortcut keys and the menu only shows them, because the keys reach the WebView first; on macOS and Linux the menu handles them.

The app embeds the page in [`apps/viewer-ui`](../viewer-ui/README.md) when it is built, with the drawing's web fonts in its `fonts/` when `python scripts/fonts.py web` has written them ([Fonts](../viewer-ui/README.md#fonts)); they add 17.3 MB. Without them the drawing falls back on the fonts installed on the computer. The About box gives the fonts' copyright and licence.

On Linux the build needs the WebKitGTK development packages that Tauri lists, for example `libwebkit2gtk-4.1-dev`, `libgtk-3-dev` and `librsvg2-dev` on Debian and Ubuntu. Windows needs the WebView2 runtime, which Windows 10 and 11 normally have.

To build an installer, install the Tauri command line once and build in this directory:

```bash
cargo install tauri-cli --version "^2" --locked
```

```bash
cargo tauri build
```

## Testing

`cargo test --manifest-path apps/desktop/Cargo.toml` tests the menu built from the command table. On Windows a smoke test builds the app, starts it on a netlist of the corpus and drives its page through WebView2's DevTools port: the drawing is shown, the window is named after the file, a click selects an instance, Enter opens its subcircuit and Backspace comes back, and the native menu has the table's items, runs them and follows the page's check marks. It needs Node 22 or later and nothing else:

```bash
node apps/desktop/tests/smoke.mjs [--port PORT] [--no-build]
```

The app runs with a WebView2 profile of its own in a temporary directory, so the layout kept in your profile is left as it was, and is stopped by its process ID. The test fails at once when the app ends early, and a step fails when the page stops answering. The page itself is tested in three browsers against a fake shell in [`tests/viewer`](../../tests/viewer/README.md).

## Structure

| Path | Content |
| --- | --- |
| `src/main.rs` | The Tauri application, and the commands to open a file, ask for a file and read the file named on the command line. Opening a file builds its Document with [`rnl-view`](../../crates/rnl-view/README.md) from the files on disk: one SVG per level with what on it can be clicked, the source files with the places of their records, the instances and nets of every level with the places that name them, and the diagnostics. |
| `src/menu.rs` | The menu bar, built from the page's command table, and the command that sets its check marks. |
| [`../viewer-ui/`](../viewer-ui/README.md) | The page, which the web viewer shares: plain HTML, CSS and ES modules with no build step. It reaches the app only through its desktop shell, `shells/desktop.js`. |
| `tests/smoke.mjs` | The smoke test, for Windows. |
| `tauri.conf.json`, `capabilities/` | The window, where the page is, the content security policy, and what the page may call. |
| `icons/` | The application icon. |

This directory is a Cargo workspace of its own, so the WebView dependencies stay out of the library crates' builds; [`desktop.yml`](../../.github/workflows/desktop.yml) builds it on Linux, Windows and macOS.

The viewer calls the shared Rust core natively, through `rnl-view`, as [decision 0006](../../docs/decisions/0006-shared-core-per-platform-frontends.md) decides; the [web viewer](../web/README.md) calls the same core through WebAssembly and shares the page in [`apps/viewer-ui`](../viewer-ui/README.md).
