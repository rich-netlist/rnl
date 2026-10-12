# desktop

RNL Viewer, a small desktop application that opens a netlist and draws it. It only reads: it neither edits nor solves, so objects without a position are not drawn, as with `rnl render`.

- **Open** (Ctrl+O) a SPICE netlist, a Spectre netlist (`.scs`) or a package file (`.rnl`), or give the file on the command line. **Reload** (F5) reads it again, keeping the level and the view.
- Each drawing level (the file and each subcircuit) has a tab. The drawing comes from the same runtime and renderer as `rnl render`, with the bundled standard packages.
- Drag with the left or middle button to move the view; scroll to zoom around the pointer. **Fit** (F, or double-click) shows the whole level.
- The **Problems** panel lists the errors, warnings and hints; click its header to fold it. The header also says what the shown level lacks without a solver: objects without a position, and constraints that need a solver or could not be checked.

## Running

From the repository root:

```bash
cargo run --manifest-path apps/desktop/Cargo.toml -- tests/conformance/cases/03-five-transistor-ota.sp
```

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
| `src/main.rs` | The Tauri application and its three commands: open a file, ask for a file, and the file named on the command line. |
| `src/document.rs` | Reading a file into what the page shows: one SVG per level, the diagnostics, each level's status. |
| `ui/` | The page: plain HTML, CSS and JavaScript with no build step. It shows one level at a time and moves the view by changing the SVG's `viewBox`. |
| `tauri.conf.json`, `capabilities/` | The window, the content security policy, and what the page may call. |
| `icons/` | The application icon. |

This directory is a Cargo workspace of its own, so the WebView dependencies stay out of the library crates' builds; [`desktop.yml`](../../.github/workflows/desktop.yml) builds it on Linux, Windows and macOS.

The [architecture proposal](../../docs/rnl-toolchain-architecture.md) plans a desktop shell around the shared TypeScript viewer and the runtime compiled to WebAssembly. Neither exists yet, so this viewer calls the Rust runtime directly; its page is small enough to replace when the shared viewer arrives.
