# desktop

RNL Viewer, a small desktop application that opens a netlist and draws it. It only reads: it neither edits nor solves, so objects without a position are not drawn, as with `rnl render`.

- **File › Open** (Ctrl+O) opens a SPICE netlist, a Spectre netlist (`.scs`) or a package file (`.rnl`); the file can also be given on the command line. **File › Reload** (F5) reads it again, keeping the level, the view and the selection.
- The drawing shows one level at a time, with a tab for the file and each subcircuit. It comes from the same runtime and renderer as `rnl render`, with the bundled standard packages. Drag with the left or middle button to move the view; scroll to zoom around the pointer. **View › Fit to Window** (F, or double-click) shows the whole level.
- The **Netlist** panel on the left shows the netlist and the files it includes, with RNL and the host statements highlighted. Below it, the **Problems** list shows the errors, warnings and hints; clicking one shows its line. Line numbers are coloured where a line has a diagnostic. Both parts fold (View › Netlist Panel, Ctrl+B; View › Problems, Ctrl+Shift+M), and the borders between the panels can be dragged within limits.
- **Selection follows an instance or a net between the drawing and the text.** Clicking a symbol (anywhere in its box) selects its instance; clicking a wire or a pin selects the net, and clicking a ground or port symbol selects the net it stands on. The text then marks the instance's statement and every place that names it: its name, the nodes of a net in the statements and subcircuit headers, and the `#name` and `&net` references in the records. The other way round, clicking an instance's statement or a node in the text selects it on the drawing, switching to its level if needed. Clicking an empty place or pressing Esc clears the selection.
- The status bar shows the selection, what the shown level lacks without a solver (objects without a position, and constraints that need a solver or could not be checked), and the zoom.

## Running

From the repository root:

```bash
cargo run --manifest-path apps/desktop/Cargo.toml -- tests/conformance/cases/03-five-transistor-ota.sp
```

On Windows the page handles the shortcut keys and the menu only shows them, because the keys reach the WebView first; on macOS and Linux the menu handles them.

On Linux the build needs the WebKitGTK development packages that Tauri lists, for example `libwebkit2gtk-4.1-dev`, `libgtk-3-dev` and `librsvg2-dev` on Debian and Ubuntu. Windows needs the WebView2 runtime, which Windows 10 and 11 normally have.

To build an installer, install the Tauri command line once and build in this directory:

```bash
cargo install tauri-cli --version "^2" --locked
```

```bash
cargo tauri build
```

## Structure

| Path | Content |
| --- | --- |
| `src/main.rs` | The Tauri application: the menu bar, and the commands to open a file, ask for a file, read the file named on the command line, and set the View menu's check marks. |
| `src/document.rs` | Reading a file into what the page shows: one SVG per level with what on it can be clicked, the source files with the places of their records, the instances and nets of every level with the places that name them, and the diagnostics. The references in the records come from the runtime (`Interpretation::refs`), so aliases, scopes and subscripts are resolved as the language says. |
| `ui/` | The page: plain HTML, CSS and JavaScript with no build step. It moves the view by changing the SVG's `viewBox` and finds what a click hits from the boxes, pins and wires `document.rs` gives. |
| `tauri.conf.json`, `capabilities/` | The window, the content security policy, and what the page may call. |
| `icons/` | The application icon. |

This directory is a Cargo workspace of its own, so the WebView dependencies stay out of the library crates' builds; [`desktop.yml`](../../.github/workflows/desktop.yml) builds it on Linux, Windows and macOS.

The [architecture proposal](../../docs/rnl-toolchain-architecture.md) plans a desktop shell around the shared TypeScript viewer and the runtime compiled to WebAssembly. Neither exists yet, so this viewer calls the Rust runtime directly; its page is small enough to replace when the shared viewer arrives.
