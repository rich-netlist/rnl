# desktop

RNL Viewer, a small desktop application that opens a netlist and draws it. It only reads: it neither edits nor solves, so objects without a position are not drawn, as with `rnl render`.

- **File › Open** (Ctrl+O) opens a SPICE netlist, a Spectre netlist (`.scs`) or a package file (`.rnl`); the file can also be given on the command line. **File › Reload** (F5) reads it again, keeping the level, the view and the selection.
- The drawing shows one level at a time; its name is above it. It comes from the same runtime and renderer as `rnl render`, with the bundled standard packages. Drag with the left or middle button to move the view; scroll to zoom around the pointer. **View › Fit to Window** (F, or double-click) shows the whole level.
- The side panel has two views. **Netlist** shows the netlist and the files it includes, with RNL and the host statements highlighted. Below it, the **Problems** list shows the errors, warnings and hints; clicking one shows its line. Line numbers are coloured where a line has a diagnostic. **Hierarchy** shows the subcircuit instances as a tree under the top level, and below it every subcircuit by name, grouped by initial, with a search box; the top level is always first. Picking a subcircuit in either opens its drawing, and both mark the one open. The side panel folds (View › Netlist Panel, Ctrl+B), so does the problems list (View › Problems, Ctrl+Shift+M), and the borders between the parts can be dragged within limits.
- **Selection follows an instance or a net between the drawing and the text.** Clicking a symbol (anywhere in its box) selects its instance; clicking a wire or a pin selects the net, and clicking a ground or port symbol selects the net it stands on. The text then marks the instance's statement and every place that names it: its name, the nodes of a net in the statements and subcircuit headers, and the `#name` and `&net` references in the records. A terminal reference such as `#M1:d` belongs to both: its `#M1` to the instance, all of it to the net at the terminal. The other way round, clicking an instance's statement or a node in the text selects it on the drawing, switching to its level if needed. Clicking an empty place or pressing Esc clears the selection.
- **A selected subcircuit instance can be opened.** It is marked in its own colour, with an open badge on the corner of its box. Clicking the badge, double-clicking the instance (on the drawing or in its statement), pressing Enter, or View › Open Subcircuit shows the subcircuit's drawing. While a subcircuit is shown, the button on the right of the bar above the drawing, Backspace, or View › Up to Parent Level goes back up along the hierarchy path that led there, with the instance selected and the view as it was left.
- **Statements that do not take effect have a soft background** in the text: rose for those that failed with an error, cream for those overridden in a conflict (7.5): dropped in the order of 8.5, or a symbol that loses to later statements on every object. Only the failing part of a pipe is marked. The *Failed statements* box above the text turns the marking off and on. Statements in styles that later ones override property by property are not marked.
- The status bar shows the selection, what the shown level lacks without a solver (objects without a position, and constraints that need a solver or could not be checked), and the zoom.
- **Help › Check for Updates** looks on GitHub for a newer release of the viewer when it is chosen, never by itself ([Checking for updates](#checking-for-updates)).

## Running

From the repository root:

```bash
cargo run --manifest-path apps/desktop/Cargo.toml -- tests/conformance/cases/03-five-transistor-ota.sp
```

The menus and their keys come from the page's command table ([`commands.json`](../viewer-ui/README.md#the-command-table)). On Windows the page handles the shortcut keys and the menu only shows them, because the keys reach the WebView first; on macOS and Linux the menu handles them.

The app embeds the page in [`apps/viewer-ui`](../viewer-ui/README.md) when it is built, with the drawing's web fonts in its `fonts/` when `python scripts/fonts.py web` has written them ([Fonts](../viewer-ui/README.md#fonts)); they add 17.3 MB. Without them the drawing falls back on the fonts installed on the computer. The About box gives the fonts' copyright and licence.

On Linux the build needs the WebKitGTK development packages that Tauri lists, for example `libwebkit2gtk-4.1-dev`, `libgtk-3-dev` and `librsvg2-dev` on Debian and Ubuntu. Windows needs the WebView2 runtime, which Windows 10 and 11 normally have.

## Building

The app is a single executable with the page and its fonts embedded, `rnl-viewer.exe` on Windows; write the fonts first. From the repository root:

```bash
python scripts/fonts.py web
cargo build --release --manifest-path apps/desktop/Cargo.toml
```

`apps/desktop/target/release/rnl-viewer.exe` needs no other file of the build but the WebView2 loader. Rust's MSVC toolchain, which GitHub's Windows runners have, links the loader into the executable, so that it runs as a single file. The GNU toolchain (`x86_64-pc-windows-gnu`), on a computer without Visual Studio, makes it import `WebView2Loader.dll`, which the build puts next to it and which must stay there. Either way the app needs the WebView2 runtime.

The installers come from the Tauri command line, which [`package.json`](package.json) pins at 2.12.1, with the sums of its files in `package-lock.json`. In this directory:

```bash
npm ci
npx tauri build
```

It builds the executable as above, with Tauri's `custom-protocol` feature as for production, and then the installers that the bundle settings of `tauri.conf.json` ask for, in `target/release/bundle/`; the first time, it downloads NSIS and WiX into `%LOCALAPPDATA%\tauri`. `npx tauri build --no-bundle` builds only the executable, and `--bundles nsis` or `--bundles msi` one of the installers.

- `nsis/RNL Viewer_VERSION_x64-setup.exe`, about 20 MB, installs for the current user without administrator rights, into `%LOCALAPPDATA%\RNL Viewer`, in English or Simplified Chinese as Windows is set. It adds shortcuts in the Start menu and on the desktop, and an entry among the installed apps, which uninstalls it. Tauri's uninstaller would keep in the registry where the app was installed, unless its *Delete the application data* box is checked, which a silent uninstall cannot do; [`nsis-hooks.nsh`](nsis-hooks.nsh) removes that too, except when an update runs the uninstaller.
- `msi/RNL Viewer_VERSION_x64_en-US.msi`, about 22 MB, installs for all users of the computer, into Program Files, and needs administrator rights. It is in English only, as one MSI is built per language. Its upgrade code is fixed in `tauri.conf.json`, so that a later version replaces this one even if the product is renamed.

Both show the licence in [`LICENSE.txt`](LICENSE.txt) and install it next to the app: the MIT License, and the copyright notice and the SIL Open Font License 1.1 of the fonts the app embeds ([Fonts](../viewer-ui/README.md#fonts)). It is written in ASCII, a line per paragraph for the installers' windows to wrap, since the MSI's licence dialog reads it as Windows-1252. When the WebView2 runtime is missing, both download Microsoft's bootstrapper, which installs it: Windows 10 and 11 normally have the runtime, and embedding the bootstrapper instead would add 1.8 MB to every installer to save a download, while the bootstrapper downloads the runtime anyway. The installers and the executable are not signed, so Windows SmartScreen may warn before they run. Built with the GNU toolchain, both installers carry `WebView2Loader.dll` next to the app.

## Installing

The installers also install and uninstall silently, for scripts and for deployment. The NSIS installer takes the directory last, after `/D=`, unquoted even with spaces:

```bat
RNL-Viewer-0.1.0-windows-x64-setup.exe /S /D=C:\Tools\RNL Viewer
"%LOCALAPPDATA%\RNL Viewer\uninstall.exe" /S
msiexec /i RNL-Viewer-0.1.0-windows-x64.msi /qn
msiexec /x RNL-Viewer-0.1.0-windows-x64.msi /qn
```

## Releasing

[`viewer-release.yml`](../../.github/workflows/viewer-release.yml) builds and releases the viewer for Windows on GitHub's runner. To release version X.Y.Z:

1. Set the version in `Cargo.toml` and `tauri.conf.json` here and in `[workspace.package]` of the root `Cargo.toml`, which the web viewer's About box shows, and update both lock files (`cargo update --workspace`, here with `--manifest-path apps/desktop/Cargo.toml`). The tests here fail while the three differ.
2. Write the release's notes in `release-notes/X.Y.Z.md`.
3. Push the tag `viewer-vX.Y.Z` of the commit to GitHub.

The workflow stops when the tag is not the app's version or the notes are missing. It builds with the MSVC toolchain, checks with `dumpbin` that the executable imports no `WebView2Loader.dll`, smoke-tests the executable alone in a directory, installs each installer silently, smoke-tests the installed app and uninstalls it. Then it makes the GitHub release *RNL Viewer X.Y.Z* with the notes and four files: `RNL-Viewer-X.Y.Z-windows-x64-setup.exe`, `RNL-Viewer-X.Y.Z-windows-x64.msi`, `RNL-Viewer-X.Y.Z-windows-x64.exe`, the single executable, and `SHA256SUMS.txt`. The release is not marked as the repository's latest, which stays the language standard's release, so the viewer's releases are told apart by their tags. A version with a prerelease part, such as `0.2.0-rc.1`, makes a prerelease, which Check for Updates passes over. Run by hand, from the Actions tab, the workflow only builds and checks, and keeps the four files as the run's artifact.

## Checking for updates

**Help › Check for Updates**, which only the desktop offers, looks for a newer release when it is chosen, never by itself. The page ([`updates.js`](../viewer-ui/updates.js)) asks GitHub's API for the repository's 30 newest releases, `https://api.github.com/repos/rich-netlist/rnl/releases?per_page=30`. Of those tagged `viewer-v` and a version, neither drafts nor prereleases, it takes the one with the highest version and compares it with the app's by Semantic Versioning. A newer one is offered with a button that opens its release page in the default browser; otherwise the dialog says that this is the latest version, or why the latest could not be looked up, with a link to the releases. GitHub answers 60 requests an hour from one address without an account.

The app's content security policy (`tauri.conf.json`) lets the page reach `https://api.github.com`, and no other address outside the app; its capability (`capabilities/default.json`) lets the page open only the repository's pages, `https://github.com/rich-netlist/rnl/*`, through Tauri's opener plugin; the app's version comes from Tauri, which has it from `tauri.conf.json`. The desktop shell gives the page the version and opens the pages ([The shell](../viewer-ui/README.md#the-shell)).

## Testing

`cargo test --manifest-path apps/desktop/Cargo.toml` tests the menu built from the command table, and the configuration: that the app has one version, that `LICENSE.txt` has both licences word for word, and that the page may reach GitHub's API but open only the repository's pages. On Windows a smoke test builds the app, starts it on a netlist of the corpus and drives its page through WebView2's DevTools port: the drawing is shown, the window is named after the file, a click selects an instance, Enter opens its subcircuit and Backspace comes back, and the native menu has the table's items, runs them and follows the page's check marks. Help › Check for Updates, chosen in the native menu, offers a newer release, which the test makes GitHub's API list by answering the page's request itself. It needs Node 22 or later and nothing else:

```bash
node apps/desktop/tests/smoke.mjs [--port PORT] [--no-build | --exe PATH]
```

The app runs with a WebView2 profile of its own in a temporary directory, so the layout kept in your profile is left as it was, and is stopped by its process ID. The test fails at once when the app ends early, and a step fails when the page stops answering. `--exe` starts the app at PATH instead of building one, such as `target/release/rnl-viewer.exe` or an installed app. The page itself is tested in three browsers against a fake shell in [`tests/viewer`](../../tests/viewer/README.md).

The installers' test installs an installer silently, checks its files, its shortcuts and its entry among the installed apps, smoke-tests the installed app, uninstalls it silently and checks that nothing of it is left:

```bash
node apps/desktop/tests/installers.mjs --nsis SETUP [--dir DIR] [--no-smoke]
node apps/desktop/tests/installers.mjs --msi PACKAGE [--no-smoke]
```

The NSIS installer installs for you, into DIR or else a temporary directory; the test stops when RNL Viewer is installed for you already, to leave it be. The MSI installs for the computer and needs administrator rights, which GitHub's runners have; the release workflow tests both.

## Structure

| Path | Content |
| --- | --- |
| `src/main.rs` | The Tauri application, and the commands to open a file, ask for a file and read the file named on the command line. Opening a file builds its Document with [`rnl-view`](../../crates/rnl-view/README.md) from the files on disk: one SVG per level with what on it can be clicked, the source files with the places of their records, the instances and nets of every level with the places that name them, and the diagnostics. |
| `src/menu.rs` | The menu bar, built from the page's command table, and the command that sets its check marks. |
| [`../viewer-ui/`](../viewer-ui/README.md) | The page, which the web viewer shares: plain HTML, CSS and ES modules with no build step. It reaches the app only through its desktop shell, `shells/desktop.js`. |
| `tests/smoke.mjs` | The smoke test, for Windows. |
| `tests/installers.mjs` | The installers' test, for Windows. |
| `tauri.conf.json`, `capabilities/` | The window, where the page is, the content security policy, what the page may call, and the installers' settings. |
| `package.json`, `package-lock.json` | The Tauri command line, pinned, which builds the installers. |
| `LICENSE.txt` | The licence that the installers show and install. |
| `nsis-hooks.nsh` | What the NSIS installer does besides Tauri's: its uninstaller leaves nothing in the registry. |
| `release-notes/` | The notes of each release, by version, which the release workflow publishes. |
| `icons/` | The application icon. |

This directory is a Cargo workspace of its own, so the WebView dependencies stay out of the library crates' builds; [`desktop.yml`](../../.github/workflows/desktop.yml) builds it on Linux, Windows and macOS, and [`viewer-release.yml`](../../.github/workflows/viewer-release.yml) releases it for Windows.

The viewer calls the shared Rust core natively, through `rnl-view`, as [decision 0006](../../docs/decisions/0006-shared-core-per-platform-frontends.md) decides; the [web viewer](../web/README.md) calls the same core through WebAssembly and shares the page in [`apps/viewer-ui`](../viewer-ui/README.md).
