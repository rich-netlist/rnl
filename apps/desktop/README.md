# desktop

On Windows, the main window first paints **RNL Viewer** and the version using a native child
control, before WebView2 is created. After its first paint, the HTML page takes over in that window, with
the same size, position and HWND; there is no separate startup window.

RNL Viewer, a small desktop application that opens a netlist and draws it. It can edit the text and apply the changes, solve the netlist as a preview and save a solved copy; it writes a file only when you have edited it and choose Save ([decision 0011](../../docs/decisions/0011-viewers-edit-the-text-and-write-only-what-the-user-saves.md), [0010](../../docs/decisions/0010-desktop-viewer-solves-as-a-preview.md)). As opened, objects without a position are not drawn, as with `rnl render`.

- **Drop netlist files from Explorer onto the window** to open them, with the same unsaved-edit confirmation as Open File. Several files offer a choice of which to open. Native file drops show the dashed drop border and retain absolute paths, so relative includes still resolve. Supported extensions are `sp`, `spi`, `spice`, `cir`, `ckt`, `net`, `netlist`, `inc`, `lib`, `scs` and `rnl`, without regard to case; directories and unrelated files are ignored.
- **Windows file associations:** the NSIS and MSI installers register those extensions in Open With and Default Apps. **File > File Associations...** explains how to select RNL Viewer and opens Windows Settings. Windows 10/11 requires the user to confirm the default application; installation, updates and removal preserve existing defaults. With the standalone EXE, use Explorer's Open with > Choose another app, browse to the executable and choose Always. Uninstalling removes the installer's registration.

- **File › Open File** (Ctrl+O) opens a SPICE netlist, a Spectre netlist (`.scs`) or a package file (`.rnl`); the file can also be given on the command line, where `--remote-debugging-port=PORT` opens the DevTools port of the app's WebView on 127.0.0.1, as in Chromium, for tests. **File › Reload** (F5) reads it again, keeping the level, the view and the selection.
- **File › Load from Clipboard** (Ctrl+Shift+V) shows the clipboard's text, with what it will be read as: a SPICE netlist, a Spectre netlist or a package, as guessed from the text or as you choose. **Load** opens it as `clipboard.sp`, `.scs` or `.rnl`, with files from the pool and PATH available; Reload builds the same text again. The app builds it with its `open_text` command.
- The drawing shows one level at a time; the hierarchy path to it is above it (see below). It comes from the same runtime and renderer as `rnl render`, with the bundled standard packages. Drag with the left or middle button to move the view; scroll to zoom around the pointer. **View › Fit to Window** (F, or double-click) shows the whole level. Resting the pointer on an instance or a net shows its name beside it (**View › Show Names on Hover**, on to begin with). A click on a label selects the instance or net it is attached to, and the selected object's labels are marked; a click on several objects at once, such as a wire over a symbol, opens a list of them beside the pointer to choose from, with the mouse or Up, Down, Enter and Escape.
- **A light grey grid lies under the drawing**, dots at every unit of the scene by default, where pins usually are. **View › Grid** shows it as dots or lines, or hides it (G shows and hides it), and shows the axes through the origin; **Grid Settings** also sets its spacing, Auto (1 unit) or a number of units, and how often a line is a darker major one. Zoomed out, it keeps its points at least 10 pixels apart by taking 2, 5, 10, 20, 50 ... units instead. It is only a view aid: clicks, fitting and the drawing ignore it. The settings stay for the next start, and the status bar shows where the pointer is on the drawing, in units with y up.
- The side panel starts with Library, followed by the document views. **Netlist** shows the netlist and the files it includes, with RNL and the host statements highlighted. Below it, the **Problems** list shows the errors, warnings and hints; clicking one shows its line. Line numbers are coloured where a line has a diagnostic. **Hierarchy** shows the subcircuit instances as a tree under the top level, below it, in a group *Not called from the top level*, the subcircuits that no chain of instances from the top level reaches, each with the subcircuits it calls, and below that every subcircuit by name, grouped by initial, with a search box; the top level is always first. When the top level has no objects at all and the netlist has such subcircuits, as with a library, or a testbench defined and never instantiated, the document opens on the first of them that has something to draw, and the status bar says so ([the page](../viewer-ui/README.md#subcircuits-that-nothing-calls)). Picking a subcircuit in either opens its drawing, and both mark the one open: picked in the index, the subcircuit is the start of the path above the drawing; picked in the tree, the path comes from the top level down to that instance. **Not drawn** lists the objects of the level shown that have no position, so that nothing draws them yet: each with its symbol, the nets at its terminals and its name, loaded a part at a time as the list is scrolled, and searched by name. Picking one shows its statement in the netlist, where the statements of these objects are greyed ([the page](../viewer-ui/README.md#the-objects-not-drawn)). The side panel folds (View › Netlist Panel, Ctrl+B), so does the problems list (View › Problems, Ctrl+Shift+M), and the borders between the parts can be dragged within limits.
- **Selection follows an instance or a net between the drawing and the text.** Clicking a symbol (anywhere in its box) selects its instance; clicking a wire or a pin selects the net, and clicking a ground or port symbol selects the net it stands on. The text then marks the instance's statement and every place that names it: its name, the nodes of a net in the statements and subcircuit headers, and the `#name` and `&net` references in the records. A terminal reference such as `#M1:d` belongs to both: its `#M1` to the instance, all of it to the net at the terminal. The other way round, clicking an instance's statement or a node in the text selects it on the drawing, switching to its level if needed. Clicking an empty place or pressing Esc clears the selection.
- **A selected subcircuit instance can be opened.** It is marked in its own colour, with an open badge on the corner of its box. Clicking the badge, double-clicking the instance (on the drawing or in its statement), pressing E or Enter, or Edit › Open Subcircuit shows the subcircuit's drawing.
- **While a subcircuit is drawn, the text outside its definition is dimmed**, from its header to its `.ends`, in its file and in the others. Going down into it, or picking it in the hierarchy, brings the start of its definition to the top of the text; going back up shows the statement of the instance it was left through. A name picked in the text leaves the text where it is.
- **The bar above the drawing shows the hierarchy path to the level shown**, such as `04-hierarchy.sp › amp X2`: from its start, the top level or a subcircuit picked in the index, a segment for each instance gone down into, naming its subcircuit with the instance beside it. Clicking a segment goes back to its level; the button on the right of the bar, Ctrl+E or Backspace, or Edit › Up to Parent Level goes back one level, and at the start of the path there is none to go back to. A level comes back with the instance it was left through selected and the view as it was left; each path to a subcircuit keeps a view of its own. Selecting an object of another level in the text cuts the path back to that level, or starts it there. Reload keeps the path while its instances are there. In a narrow window the levels between the start and the level shown go behind an ellipsis, which lists them.
- **Statements that do not take effect have a soft background** in the text: rose for those that failed with an error, cream for those overridden in a conflict (7.5): dropped in the order of 8.5, or a symbol that loses to later statements on every object. Only the failing part of a pipe is marked. The *Failed statements* box above the text turns the marking off and on. Statements in styles that later ones override property by property are not marked.
- The status bar shows the selection, what the shown level lacks without a solver (objects without a position, which a click shows in the Not drawn view, and constraints that need a solver or could not be checked), the solve under way or the last one, where the pointer is on the drawing, and the zoom.
- **Solve › Solve** (F9) solves the netlist shown, with what it includes, as `rnl solve` solves it: read without its records of the solver (`--no-generated`), with the solver's options as Solve › Settings set them, the defaults of `rnl solve` until then. It runs on a thread of its own, so the window stays usable; the status bar shows the level being solved, with a **Cancel** button. The result is drawn as a **preview**: the texts the solve would write are held in memory over the files, the bar above the drawing says *Solved preview: the files are unchanged*, and nothing is written. **File › Reload** (F5), or Solve again, solves again; **Solve › Close Preview**, the button beside that note, or opening a file, shows the file as written. **Solve › Settings** sets the options of the solver in a dialog: the time limit of each level's resolution, the tolerance, the grid, the beam, the work of the search and the router, whether to seek every conflict group of the facts of priority low, and the objectives, which it ranks the placements by: their order, which are on, and which share a level with what weight and tolerance. Moving an objective up or down changes which counts first. The dialog says that the priority of a constraint (low, normal, high) is written in the netlist, not set here; **Restore Defaults** sets the solver's own defaults, and the settings stay for the next start ([the page](../viewer-ui/README.md#the-solvers-settings)).
- **The Solve view of the side panel** has the solve's report while a preview is shown: per level, the facts that failed, those discarded with the groups of facts they conflict with, those left undecided within the time limit and those kept, the statements whose facts took part in discarding, and the solver's errors, warnings and notes. Clicking a line shows it in the netlist, as a problem does.
- **Solve › Save Solved Copy As** (Ctrl+Shift+S) saves the files the solve would write where you choose: one file, offered as `NAME-solved.EXT` beside the netlist, for a netlist that reads no other file; otherwise a folder, which gets the files solved and the other files read in their places relative to the netlist, as `rnl solve -o DIR` writes them. A place that would replace a file that was read, the netlist itself or any file it includes, is refused. A file of 0.2 comes out in 0.3, as `rnl solve` writes it (16.1).
- **Edit › Edit Text** (Ctrl+Q) switches the netlist view to edit: a text box for the source file selected through the hierarchy or Problems, with line numbers and no highlighting. A change is shown as *Changes not applied yet*, and a dot on the top-file button marks unsaved changes in any source. Clicking that filename returns to the root source. **Edit › Apply and Exit Editing** (Ctrl+Enter, or the button above the text) builds the netlist again from the texts in memory, over the files on disk, and draws it and checks it as opening them would: the drawing, the problems and the hierarchy are made again, keeping the level, the view and the selection. Exiting editing also applies pending changes, and applying exits editing. Edit › Discard Changes confirms before dropping pending changes and exiting. Entering and exiting preserve the first visible text line. Nothing solves itself: **Solve** is a separate step, of the text applied, and the Solve view is not edited (**Close Preview** shows the text again).
- **File › Save** (Ctrl+S) writes the files with changes not saved where they were read from, in the encoding and with the line ends each had (a file that is not UTF-8 stays Latin-1; text it cannot hold is refused, and the message asks for explicit UTF-8 conversion); no other file is written. **File › Save As** writes the netlist that was opened, the first tab, under a name you give, and opens that file; included files are saved where they are with Save, and Save As waits until they are. A text from the clipboard has no file: Save and Save As ask where, and the file written is the netlist from then on. The window's title has a dot while changes are not saved, and **opening another file, File › Reload and closing the window** ask first to save them, leave them unsaved or go no further. Before it writes, Save looks at each file on disk: one that is no longer what was read, because another program changed it or took it away (compared as the core reads it, so another program's change of the line ends or the byte order mark does not count), is not written, nor are the others, and a dialog asks: **Overwrite** writes the text in the viewer over it, **Reload** reads the files again and drops the changes made in the viewer, **Cancel** leaves everything as it is.
- **File › Save Changes Using UTF-8**, **File › Save As UTF-8**, and **Solve › Save Solved Copy As UTF-8** explicitly convert the files being written to UTF-8, normally without an encoding BOM. If the text begins with U+FEFF, a separate encoding BOM protects that character from removal when reopened; the same exception applies to newly inserted leading U+FEFF in an ordinary UTF-8 save. Save Changes Using UTF-8 applies to unsaved edits; use Save As UTF-8 to convert an unchanged netlist. Ordinary save commands preserve the source encoding, including Save As even over an existing destination with a different encoding. The CLI and desktop use the [same codec and refusal policy](../../docs/output-encoding.md).
- **Help › Check for Updates** looks on GitHub for a newer release of the viewer when it is chosen, never by itself ([Checking for updates](#checking-for-updates)).

Large netlists show reading, parsing and display stages with an indeterminate bar. The Netlist
read view renders a viewport of large files; Edit > Find in Netlist (Ctrl+F) and copy still use the
complete text. File and edit-mode changes keep the first visible line. See the shared
[large-netlist behavior](../viewer-ui/README.md#loading-and-large-netlists).

## File pool and search paths

The first sidebar tab, **Library**, groups the session's searchable files. Opening a file with Open File, a drop, the command line (desktop), or `?url=` (web) also adds it to **Files**, without duplicates. **Include File** adds selected netlists or `.rnl` packages; **Include Folder** recursively adds files with netlist/package extensions (`sp`, `spi`, `spice`, `cir`, `ckt`, `net`, `netlist`, `inc`, `lib`, `scs`, `rnl`), preserving their directories. Click a pooled file to open it. **Exclude** removes it from the search pool, without deleting the file or closing an already open document. Pool changes rebuild the current document while keeping edited text and the view; edits not yet applied remain unapplied.

Built-in standard packages from series 0.3 and 0.2 are in the initially collapsed **Built-in** group, with **read only** badges. They can be opened but cannot be edited or excluded. The pool itself lasts for this viewer session. Files starts expanded; group collapse state lasts only for this session, and a newly added file expands Files. Matching root-file rows are highlighted across all groups; selecting an included Netlist tab does not change that highlight.

Clipboard text occupies one separate **temporary · not searched** row above the groups. It never enters include/package lookup. Click it to return to its last applied text; unapplied edits follow the usual file-switch confirmation. The next paste replaces this slot, and closing or reloading clears it.

Relative references are tried first. For a bare include filename only, a missing relative file falls back to the pool, then the desktop's PATH directories in their displayed order; a hint names the selected file. A path containing directory components does not use include fallback. Packages referenced by name search the pool, then PATH, then bundled packages. Version selection still follows 16.2: an exact version, or the specified compatible/newest version; copies of the same version follow search order. Thus a user's `rnl-std` can override the built-in copy (16.3).

Within each package directory, `<name>.rnl` is preferred, then `<version>/<name>.rnl`, then other `.rnl` files (including version directories). Every candidate's `PACKAGE` header is checked. A conventional filename with the wrong package name or no valid header produces a warning. A different filename is valid; filenames are a tool search preference, not a language requirement.

Between Files and Built-in, each ordered PATH directory has its own initially collapsed group. **Add PATH Directory…** adds one; its header can move it up/down or remove it. Expand it to see direct `.rnl` files and those in one version-number subdirectory, or a directory error. Opening a row also adds it to Files. Groups rescan whenever Library is shown or PATH changes; these are the same persisted settings used for opening, applying edits, reloading and solving, including clipboard documents. This does not modify the system environment's PATH.

All viewer settings (including Solve › Settings, grid, hover names, sidebar state/sizes and search paths) are stored in `settings.json` under Tauri's `app_config_dir()`: normally `%APPDATA%/io.github.rich-netlist.viewer` on Windows, `~/Library/Application Support/io.github.rich-netlist.viewer` on macOS and `~/.config/io.github.rich-netlist.viewer` on Linux. The exact path is displayed at the bottom of Library. On first startup without this file, known settings are migrated from the WebView's localStorage; successful migration removes those legacy keys. Later starts use the file. A malformed or unreadable settings file is reported and preserved, rather than overwritten. Saves replace the file through a temporary file. File pool entries are not persisted.

## Running

Download the files from a [viewer release](https://github.com/rich-netlist/rnl/releases?q=viewer-v), or the [tools page](https://rich-netlist.github.io/rnl/en/tools.html#desktop). Windows x64 has the NSIS installer, MSI and portable executable described below; run the installer or the portable `.exe`. Windows 10 and 11 normally include WebView2. The files are unsigned: if SmartScreen warns, choose **More info › Run anyway**.

Linux x86_64 packages start with 0.1.2. They are built and tested on Ubuntu 22.04, the minimum supported Ubuntu; newer Ubuntu versions are supported too. Other distributions need compatible glibc and WebKitGTK 4.1. AppImage bundles most dependencies but cannot make an older glibc compatible. Replace `X.Y.Z` below with the downloaded version:

```bash
chmod +x RNL-Viewer-X.Y.Z-linux-x86_64.AppImage
./RNL-Viewer-X.Y.Z-linux-x86_64.AppImage
# Without FUSE, extract and run instead:
./RNL-Viewer-X.Y.Z-linux-x86_64.AppImage --appimage-extract-and-run

# Debian/Ubuntu: apt installs the required system libraries too.
sudo apt install ./RNL-Viewer-X.Y.Z-linux-x86_64.deb
rnl-viewer # or choose RNL Viewer in the application menu
sudo apt remove rnl-viewer

# Download SHA256SUMS.txt beside the package(s) to verify them:
sha256sum --check --ignore-missing SHA256SUMS.txt
```

Both Linux formats accept a netlist path after the executable, or **File › Open File** after launch. The AppImage needs no installation; remove that file to uninstall it. macOS packages remain future work; build from source there.

From the repository root:

```bash
cargo run --manifest-path apps/desktop/Cargo.toml -- tests/conformance/cases/03-five-transistor-ota.sp
```

Local development builds optimize the RNL runtime, host readers, viewer model and renderer at
level 2 while keeping the desktop shell in the development profile. Large hierarchies otherwise
spend seconds interpreting their circuits even when the Library's definition index is already warm.
Release builds retain their usual profile.

Each desktop window keeps a **256 MiB LRU memory cache** of compiled documents, SVG drawings
and lazy drawings for the Not drawn pane. Switching back to an unchanged file reuses them;
the least recently used results are evicted first. **File > Memory Cache…** shows its retained
budget and hit count, changes the limit (0 disables caching), or clears it. An oversized document
still opens but is not retained. The budget conservatively estimates owned cache data, including
source snapshots and nested drawing data; the active view and temporary build/IPC buffers are
additional memory, so this is not a cap on the entire process. Results stay only until the window closes.

A cache hit checks the contents of all observed inputs, including failed reads, and the ordering
of package/include and Auto Include candidates. File edits, newly available dependencies and
lookup changes therefore invalidate affected results. F5 bypasses reuse; Reload Library clears
all results. Explicit Solve operations still run the solver and are not cached.

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

On Linux, Tauri also reads [`tauri.linux.conf.json`](tauri.linux.conf.json), which selects AppImage and deb without changing the Windows settings. `npx tauri build --bundles appimage,deb` writes both to `target/release/bundle/`. The release workflow installs the build libraries, `patchelf` and FUSE support on Ubuntu 22.04, writes the same pinned fonts as Windows, and uses the pinned Tauri CLI.

- `nsis/RNL Viewer_VERSION_x64-setup.exe`, about 20 MB, installs for the current user without administrator rights, into `%LOCALAPPDATA%\RNL Viewer`, in English or Simplified Chinese as Windows is set. It adds shortcuts in the Start menu and on the desktop, and an entry among the installed apps, which uninstalls it. Tauri's uninstaller would keep in the registry where the app was installed, unless its *Delete the application data* box is checked, which a silent uninstall cannot do; [`nsis-hooks.nsh`](nsis-hooks.nsh) removes that too, except when an update runs the uninstaller.
- `msi/RNL Viewer_VERSION_x64_en-US.msi`, about 22 MB, installs for all users of the computer, into Program Files, and needs administrator rights. It is in English only, as one MSI is built per language. Its upgrade code is fixed in `tauri.conf.json`, so that a later version replaces this one even if the product is renamed.

Both show the licence in [`LICENSE.txt`](LICENSE.txt) and install it next to the app: the MIT License, and the copyright notice and the SIL Open Font License 1.1 of the fonts the app embeds ([Fonts](../viewer-ui/README.md#fonts)). It is written in ASCII, a line per paragraph for the installers' windows to wrap, since the MSI's licence dialog reads it as Windows-1252. When the WebView2 runtime is missing, both download Microsoft's bootstrapper, which installs it: Windows 10 and 11 normally have the runtime, and embedding the bootstrapper instead would add 1.8 MB to every installer to save a download, while the bootstrapper downloads the runtime anyway. The installers and the executable are not signed, so Windows SmartScreen may warn before they run. Built with the GNU toolchain, both installers carry `WebView2Loader.dll` next to the app.

## Installing

The installers also install and uninstall silently, for scripts and for deployment. The NSIS installer takes the directory last, after `/D=`, unquoted even with spaces:

```bat
RNL-Viewer-0.1.3-windows-x64-setup.exe /S /D=C:\Tools\RNL Viewer
"%LOCALAPPDATA%\RNL Viewer\uninstall.exe" /S
msiexec /i RNL-Viewer-0.1.3-windows-x64.msi /qn
msiexec /x RNL-Viewer-0.1.3-windows-x64.msi /qn
```

## Releasing

See [the three release lines](../../docs/releases.md) for version ownership and manual `candidate_tag` mismatch rehearsals. First-release versions remain subject to the user's decision.

[`viewer-release.yml`](../../.github/workflows/viewer-release.yml) builds and releases the viewer for Windows x64 and Linux x86_64 on GitHub's runners. To release version X.Y.Z:

1. Set the Viewer version only in `Cargo.toml` and `tauri.conf.json` here, and update the app's own lockfile entry. Leave the root workspace/core version unchanged. The web viewer reads the Viewer version too; both About dialogs also name the core version and build commit. The tests require the two Viewer declarations to match, independently of the core.
2. Write the release's notes in `release-notes/X.Y.Z.md`.
3. Push the tag `viewer-vX.Y.Z` of the commit to GitHub.

The workflow stops when the tag is not the app's version or the notes are missing. On Windows it builds with MSVC, checks with `dumpbin` that the executable imports no `WebView2Loader.dll`, smoke-tests the executable alone in a directory, installs each installer silently, smoke-tests the installed app and uninstalls it. On Linux it builds the AppImage and deb, smoke-tests the AppImage alone in a directory under Xvfb, installs the deb, smoke-tests `/usr/bin/rnl-viewer`, uninstalls it and checks that the binary and desktop entry are gone. The tests use a D-Bus session, software rendering and isolated profiles, and keep screenshots as Actions artifacts.

After both platforms succeed, the workflow verifies each platform's checksums and writes one combined `SHA256SUMS.txt`. The GitHub release *RNL Viewer X.Y.Z* contains six files:

- `RNL-Viewer-X.Y.Z-windows-x64-setup.exe`
- `RNL-Viewer-X.Y.Z-windows-x64.msi`
- `RNL-Viewer-X.Y.Z-windows-x64.exe`
- `RNL-Viewer-X.Y.Z-linux-x86_64.AppImage`
- `RNL-Viewer-X.Y.Z-linux-x86_64.deb`
- `SHA256SUMS.txt`

The release uses `--latest=false`; the repository's latest belongs to the core (`core-vX.Y.Z`), so the viewer's releases are told apart by their tags. A version with a prerelease part, such as `0.2.0-rc.1`, makes a prerelease, which Check for Updates passes over. Run by hand from the Actions tab on a branch, the workflow builds and checks both platforms and keeps the platform artifacts and combined six-file artifact; it never publishes. Before pushing a release tag, run it on the intended commit, check both build jobs and the combined checksums, inspect the Linux screenshots, and confirm the versions and notes above. The site only links Linux files when the selected release tag contains the Linux bundle configuration, so earlier Windows-only releases keep valid download links.

## Checking for updates

**Help › Check for Updates**, which only the desktop offers, looks for a newer release when it is chosen, never by itself. The page ([`updates.js`](../viewer-ui/updates.js)) asks GitHub's API for the repository's 30 newest releases, `https://api.github.com/repos/rich-netlist/rnl/releases?per_page=30`. Of those tagged `viewer-v` and a version, neither drafts nor prereleases, it takes the one with the highest version and compares it with the app's by Semantic Versioning. A newer one is offered with a button that opens its release page in the default browser; otherwise the dialog says that this is the latest version, or why the latest could not be looked up, with a link to the releases. GitHub answers 60 requests an hour from one address without an account.

The app's content security policy (`tauri.conf.json`) lets the page reach `https://api.github.com`, and no other address outside the app; its capability (`capabilities/default.json`) lets the page open the repository's pages, `https://github.com/rich-netlist/rnl/*`, and the exact public package page, `https://rich-netlist.github.io/rnl/en/packages.html`, through Tauri's opener plugin; the app's version comes from Tauri, which has it from `tauri.conf.json`. The desktop shell gives the page the version and opens the pages ([The shell](../viewer-ui/README.md#the-shell)).

## Testing

`cargo test --manifest-path apps/desktop/Cargo.toml` tests the menu built from the command table, with its submenus and the Solve menu, the settings the page gives, read as `--solver-options` reads them, the defaults the dialog shows, taken from the solver, and the name a solved copy is offered under, and the configuration: that the app has one version, that `LICENSE.txt` has both licences word for word, that the page may reach GitHub's API but open only the repository's pages, and that the DevTools switch is not taken for the file. The files module (`src/files.rs`) is tested for the edits built over a directory of files, the include and the netlist itself, and the disk left as it was; for the encoding a file is written in (UTF-8, with its byte order mark if it has one, Latin-1 where it is not UTF-8); and for writing only the files given, where the Document names them. The core thread (`src/core_thread.rs`) is tested for the drawings of a build asked for from another thread, refused when the names are not the build's, and for a job that panics, which is an error while the thread goes on. On Windows a smoke test builds the app, starts it on a netlist of the corpus and drives its page through the DevTools port that `--remote-debugging-port` opens: the drawing is shown, the window is named after the file, a click selects an instance, Enter opens its subcircuit and Backspace comes back, and the native menu has the table's items, those of its submenus too, runs them, also from a submenu, and follows the page's check marks. Help › Check for Updates, chosen in the native menu, offers a newer release, which the test makes GitHub's API list by answering the page's request itself. Solve › Solve draws the netlist solved as a preview, with its report, and the file on disk is byte for byte as it was; Close Preview reads it again. Solve › Settings shows the solver's defaults, which the app gives, and the dialog shows the solver's refusal of a beam of 0. What a solve gives, a copy saved and refused over a file that was read, and a solve cancelled or cut by the time limit are tested in `rnl-cli` ([`tests/preview.rs`](../../crates/rnl-cli/tests/preview.rs)), and the Solve menu of the page with a fake shell ([`tests/viewer/specs/solve.spec.js`](../../tests/viewer/specs/solve.spec.js)). The app gives the port to WebView2 itself, as WebView2 ignores `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` where the app runs elevated, as on GitHub's runners. The test needs Node 22 or later and nothing else:

```bash
node apps/desktop/tests/smoke.mjs [--port PORT] [--no-build | --exe PATH]
```

The app runs with a WebView2 profile of its own in a temporary directory, so the layout kept in your profile is left as it was, and is stopped by its process ID. The test fails at once when the app ends early, and a step fails when the page stops answering; when the WebView's DevTools port does not answer, it says what there is of the WebView: its processes, with their arguments, and its profile. `--exe` starts the app at PATH instead of building one, such as `target/release/rnl-viewer.exe` or an installed app. The page itself is tested in three browsers against a fake shell in [`tests/viewer`](../../tests/viewer/README.md).

On Linux, install `webkit2gtk-driver`, `xvfb`, `xdotool`, `dbus-x11` and `imagemagick`, build the app, and run:

```bash
dbus-run-session -- xvfb-run -a node apps/desktop/tests/linux-smoke.mjs --exe apps/desktop/target/debug/rnl-viewer
```

Both native smoke tests require painted tab labels, source text and schematic pixels, after the
startup/loading overlays clear. The Linux test checks both a WebKit snapshot and the real X11
framebuffer cropped to the WebView, saving the initial and accepted frames plus pixel evidence.
The grid, borders and a flat background cannot satisfy the checks.

For a release investigation, manually run `desktop.yml` with `linux_diagnostics=true`.
This skips the regular desktop test matrix and runs only diagnostics: released 0.1.2/0.1.3/0.1.4
AppImages, plus a CI-built release executable, AppImage and installed deb from the same commit.
Each gets 1/3/10-second X11 and WebKit captures, DOM/computed styles and rAF counts under default,
compositing-disabled, DMABUF-disabled and software-rendering environments. Separate X11-only and
ordinary native launches control for WebDriver/snapshot effects. Capture failures are retained
instead of preventing the other observations. `diagnostic_binary_run_id` reuses the exact binaries
from an earlier diagnostic run, including their source commit, and skips the historical matrix.
The diagnostic jobs only upload artifacts; they cannot publish. They also compare the original
DOM-only smoke test at the investigation's starting commit with the painted-content gate.

This test uses WebKitGTK's native WebDriver directly, with the same capabilities and automation environment as Tauri's driver. It opens a temporary copy of `04-hierarchy.sp` and its include, checks the visible window's title, the rendered SVG, the loaded `amp` subcircuit and the absence of document errors, and checks the netlist was not written. It builds nothing and also accepts a release AppImage or an installed binary. Set `SMOKE_OUTPUT_DIR` to keep screenshots. CI runs it against the debug app in `desktop.yml` and both packaged forms in `viewer-release.yml`.

The installers' test installs an installer silently, checks its files, its shortcuts and its entry among the installed apps, smoke-tests the installed app, uninstalls it silently and checks that nothing of it is left:

```bash
node apps/desktop/tests/installers.mjs --nsis SETUP [--dir DIR] [--no-smoke]
node apps/desktop/tests/installers.mjs --msi PACKAGE [--no-smoke]
```

The NSIS installer installs for you, into DIR or else a temporary directory, which the test gives in its long form: the uninstaller removes a shortcut only when its target, which Windows keeps long, reads as the installation directory was given, so an installation into a short path such as `C:\Users\RUNNER~1\...` leaves its shortcuts behind. The test stops when RNL Viewer is installed for you already, to leave it be. The MSI installs for the computer and needs administrator rights, which GitHub's runners have; the release workflow tests both.

The app runs on the WebView2 Runtime, which Windows 10 and 11 come with and the installers install where it is missing. `node apps/desktop/tests/webview2.mjs` prints its version, and with `--install` installs it first with Microsoft's bootstrapper when there is none, as the workflows do on GitHub's Windows Server runners.

## Structure

| Path | Content |
| --- | --- |
| `src/main.rs` | The Tauri application, and the commands to open a file, ask for a file and read the file named on the command line. Opening a file builds its Document with [`rnl-view`](../../crates/rnl-view/README.md) from the files on disk: one SVG per level with what on it can be clicked, the source files with the places of their records, the instances and nets of every level with the places that name them, and the diagnostics. The Document leaves out the drawings of the objects that have no position, which the Not drawn list asks for (`unplaced_drawings`) as it adds rows. |
| `src/core_thread.rs` | The thread that builds the documents, and keeps the drawings of the objects that the last build listed without a position: they hold what the model does, which is not `Send`, so a build and a request for drawings both run there, one after the other. |
| `src/menu.rs` | The menu bar, built from the page's command table, and the command that sets its check marks. |
| `src/files.rs` | The edits built over the files on disk, and writing the files the user saved: each in its encoding. |
| [`../viewer-ui/`](../viewer-ui/README.md) | The page, which the web viewer shares: plain HTML, CSS and ES modules with no build step. It reaches the app only through its desktop shell, `shells/desktop.js`. |
| `tests/smoke.mjs` | The smoke test, for Windows. |
| `tests/linux-smoke.mjs` | The Linux smoke test, for the debug app, AppImage and installed deb. |
| `tests/installers.mjs` | The installers' test, for Windows. |
| `tests/webview2.mjs` | The WebView2 Runtime's version, and its installation where it is missing. |
| `tauri.conf.json`, `capabilities/` | The window, where the page is, the content security policy, what the page may call, and the installers' settings. |
| `package.json`, `package-lock.json` | The Tauri command line, pinned, which builds the installers. |
| `LICENSE.txt` | The licence that the installers show and install. |
| `nsis-hooks.nsh` | What the NSIS installer does besides Tauri's: its uninstaller leaves nothing in the registry. |
| `release-notes/` | The notes of each release, by version, which the release workflow publishes. |
| `icons/` | The application icon. |

This directory is a Cargo workspace of its own, so the WebView dependencies stay out of the library crates' builds; [`desktop.yml`](../../.github/workflows/desktop.yml) builds it on Linux, Windows and macOS, and [`viewer-release.yml`](../../.github/workflows/viewer-release.yml) releases it for Windows and Linux.

The viewer calls the shared Rust core natively, through `rnl-view`, as [decision 0008](../../docs/decisions/0008-shared-core-per-platform-frontends.md) decides; the [web viewer](../web/README.md) calls the same core through WebAssembly and shares the page in [`apps/viewer-ui`](../viewer-ui/README.md).

A missing named USE-PACKAGE keeps the core error and adds a hint to include its file in Library or a PATH directory, with a Packages-page link that opens in the system browser. Public extensions such as `analog-canvas-extensions` are not built in.

### New windows and navigation keys

E/Enter opens a selected subcircuit; Ctrl+E/Backspace goes up. Shift modifies every circuit navigation to open its target in a new viewer; Shift+E/Shift+Enter explicitly opens the selected subcircuit in a new viewer. File › New Window copies the current level, selection, saved settings and Library. Library file clicks keep the Library visible. Ctrl+Q toggles editing using physical Control on macOS too; exiting applies, Ctrl+Enter applies and exits, and Edit › Discard Changes drops pending text after confirmation. See the [shared navigation/session contract](../viewer-ui/README.md#navigation-and-new-windows).

New windows are independent processes started with `--session FILE`; the one-use JSON is removed after reading. The child is offset 24 logical pixels and reads disk files. Applied unsaved edits require confirmation before a child opens. Settings writes lock, re-read and change only the requested key.


File > Reload Library rereads every session Library file and package, rescans PATH, and rebuilds the subcircuit index. PATH lists recognized netlists (`.sp`, `.spi`, `.spice`, `.cir`, `.ckt`, `.net`, `.netlist`, `.inc`, `.lib`, `.scs`) alongside `.rnl` packages, directly and in numeric version directories. File > Auto Include and the Netlist checkbox enable indexed lookup of undefined subcircuits; unique matches produce warnings in Problems. The switch is saved, defaults on, and also applies to Solve. Conflicting definitions require an explicit include. No include statements are written to source files by this option.
Optional public packages are maintained in [rnl-packages](https://github.com/rich-netlist/rnl-packages) and are not bundled with the desktop viewer. Download and verify them, then include the files in Library or a PATH directory. The missing `analog-canvas-extensions` hint remains unchanged; a desktop catalog picker is outside this change.

## Chinese fonts on Linux

The shared web UI and netlist editor fall back to the bundled, pinned Noto Sans SC web fonts when
system fonts lack Chinese glyphs. Native Latin fonts and monospace metrics remain in use; no new
font files are shipped, and the schematic renderer and its output are unchanged. The browser loads
only the existing Unicode slices needed by the visible text.

GTK native menus, native tooltips (including HTML title tooltips) and system file dialogs cannot use web fonts. On Debian/Ubuntu, install
`sudo apt install fonts-noto-cjk` and restart the application for Chinese native menus and dialogs.
The AppImage uses the host's native fonts too. The deb does not add a mandatory font dependency.

Linux smoke tests use a Latin-only Fontconfig profile. They compare the UI/source glyph raster
with the bundled Noto glyphs, reject the missing-glyph negative control, and retain Chinese
before/after screenshots alongside the native-window and WebKit painted-content checks.
