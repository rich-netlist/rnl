# web

The web viewer: the page of the RNL viewers ([`apps/viewer-ui`](../viewer-ui/README.md)) in a browser, with the shared core built as WebAssembly and run in a module Worker ([`bindings/runtime-wasm`](../../bindings/runtime-wasm/README.md)), as [decision 0008](../../docs/decisions/0008-shared-core-per-platform-frontends.md) decides. It is a static site with no server code: files opened from the computer are read in the page and sent nowhere. Like the desktop viewer ([`apps/desktop`](../desktop/README.md)) it can edit the text and apply the changes, but a page cannot write a file where it read it, so Save downloads the file shown ([decision 0011](../../docs/decisions/0011-viewers-edit-the-text-and-write-only-what-the-user-saves.md)); and it shows the same panels: the drawing, the netlist text, the problems and the hierarchy, with the selection followed between the drawing and the text.

- **File › Open File** (Ctrl+O) opens one or several files. With several, the one that is not mostly included (`.inc`, `.lib`, `.rnl`) is opened, whatever its name; when more than one could be the netlist, the page asks which. The files a netlist includes are found among the files chosen with it, by their relative paths.
- **File › Include Folder** adds the folder’s netlists and packages to the Library file pool, with their directory structure. Click a file in the pool to open it.
- **File › Open URL** opens a netlist on a web server, and fetches the files it includes relative to the address it was served from, after any redirect, `../` included. `?url=ADDRESS` in the page's address opens one at start, resolved against the page, such as `?url=samples/cases/04-hierarchy.sp`. An include that cannot be fetched (not found, no answer, nothing sent for 30 seconds, or not allowed by the server's CORS headers) is missing like any other; a netlist that cannot be fetched is an error that names its address.
- Dropped files open as with Open File; dropped folders are added as with Include Folder.
- **File › Load from Clipboard** (Ctrl+Shift+V) opens the clipboard's text once the browser lets the page read the clipboard, which it may ask you first: the page shows the text, with what it will be read as, a SPICE or Spectre netlist or a package, and opens it on **Load**, with file pool entries available to bare includes.
- **File › Reload** (F5, Ctrl+R) reads the document again, not the page, keeping the level, the view and the selection. A URL is fetched again past the browser's cache. Files are read again from what the browser holds of them: a dropped file as it is now, but a file chosen with Open File or Include Folder only as it was when it was chosen; once it has changed, Reload says to open it again.
- **Edit › Edit Text** (Ctrl+E) switches the netlist text to edit, a text box for each file; **Edit › Apply Changes** (Ctrl+Enter) builds the document again from the texts held, in the core, over the files as they were read, and draws it; **File › Save** (Ctrl+S) downloads the file shown, with the line ends it had, as UTF-8; nothing is written where a file was read from. Changes not applied are shown as such, a dot marks the tab of each file with changes not saved, and File › Reload, opening another file and closing the page ask first ([the page](../viewer-ui/README.md#editing-the-text)).
- **Help › About** names the version and the commit the site was built from. The browser tab is named after the document.
- The bar above the drawing shows the hierarchy path to the level shown, a segment per subcircuit instance gone down into, each going back to its level, as in the [desktop viewer](../desktop/README.md).
- Resting the pointer on an instance or a net shows its name beside it (**View › Show Names on Hover**, on to begin with and kept in the browser). A click on a label selects the instance or net it is attached to, the selected object's labels are marked, and a click on several objects at once opens a list of them beside the pointer to choose from, as in the [desktop viewer](../desktop/README.md).
- **View › Grid** shows a light grey grid under the drawing, dots or lines at every unit of the scene, coarser when zoomed out, and the axes through the origin, as in the [desktop viewer](../desktop/README.md); G shows and hides it, and the settings are kept in the browser for the next visit. The status bar shows where the pointer is on the drawing, in units with y up.
- **Help › Desktop Viewer** opens the project site's page of the tools in a new tab, where the desktop viewer is downloaded: [in Chinese](https://rich-netlist.github.io/rnl/zh/tools.html) when the browser's languages put Chinese before English, else [in English](https://rich-netlist.github.io/rnl/en/tools.html), the site's default.
- Packages found by name (16.2) come from the file pool before bundled packages; a package file named by path is tried relative to the file that references it.

Text is decoded as `rnl` decodes it, UTF-8 or else Latin-1, so a document is the one `rnl view` and the desktop viewer give for the same files, but for the paths that name them. Files opened from the computer are named in the workspace as `/NAME`, and a folder's as `/FOLDER/SUB/NAME`; a URL's files by their paths on the server that served the netlist, after any redirect, such as `/netlists/a/top.sp`, or `/top.sp` at its root, so that a URL that redirects opens as the one it redirects to. A netlist served at a directory's address is named after the directory, or after the server at its root, such as `/example.org`. The workspace has nothing above its root: an include such as `../x.inc` of a file opened from the computer is missing, not a file of that name opened with it.

## File pool and search paths

The first sidebar tab, **Library**, groups the session's searchable files. Opening a file with Open File, a drop, the command line (desktop), or `?url=` (web) also adds it to **Files**, without duplicates. **Include File** adds selected netlists or `.rnl` packages; **Include Folder** recursively adds files with netlist/package extensions (`sp`, `spi`, `spice`, `cir`, `ckt`, `net`, `inc`, `lib`, `scs`, `rnl`), preserving their directories. Click a pooled file to open it. **Exclude** removes it from the search pool, without deleting the file or closing an already open document. Pool changes rebuild the current document while keeping edited text and the view; edits not yet applied remain unapplied.

Built-in standard packages from series 0.3 and 0.2 are in the initially collapsed **Built-in** group, with **read only** badges. They can be opened but cannot be edited or excluded. The pool itself lasts for this viewer session. Files starts expanded; group collapse state lasts only for this session, and a newly added file expands Files. Matching root-file rows are highlighted across all groups; selecting an included Netlist tab does not change that highlight.

Clipboard text occupies one separate **temporary · not searched** row above the groups. It never enters include/package lookup. Click it to return to its last applied text; unapplied edits follow the usual file-switch confirmation. The next paste replaces this slot, and closing or reloading clears it.

Relative references are tried first. For a bare include filename only, a missing relative file falls back to the pool, then the desktop's PATH directories in their displayed order; a hint names the selected file. A path containing directory components does not use include fallback. Packages referenced by name search the pool, then PATH, then bundled packages. Version selection still follows 16.2: an exact version, or the specified compatible/newest version; copies of the same version follow search order. Thus a user's `rnl-std` can override the built-in copy (16.3).

Within each package directory, `<name>.rnl` is preferred, then `<version>/<name>.rnl`, then other `.rnl` files (including version directories). Every candidate's `PACKAGE` header is checked. A conventional filename with the wrong package name or no valid header produces a warning. A different filename is valid; filenames are a tool search preference, not a language requirement.

The browser cannot access arbitrary local directories. The note at the bottom of Library explains this limitation; use Include File/Folder instead. The pool is held in page memory, not IndexedDB, and is cleared when the page closes or reloads. Local files are not uploaded. Packages in the pool are passed to the WebAssembly core with the `pool` request field; relative URL includes are still fetched by `collect` before a fallback is final.

**File › Include Public Package…** lists optional packages published with this site, currently including SKY130. Choose one to fetch it from the same site and add it to the pool. Built-ins are omitted from this picker. `apps/web/library.mjs` publishes all package files from `libraries/`, `libraries/index.json` (name, version, path, description and built-in flag) and `libraries/bundled.json`; `build.mjs` includes them in every web build and `pages.yml` deploys them at `viewer/libraries/`. No GitHub API is used. If an optional package depends on another optional package, include that package too.

## Building

```bash
node apps/web/build.mjs
```

builds the core with [`bindings/runtime-wasm/build.mjs`](../../bindings/runtime-wasm/README.md#building) and puts the site into `apps/web/dist/`, which git ignores: the page from `apps/viewer-ui` without its README, its `package.json` and the desktop shell; the binding's modules and the WebAssembly module in `core/`; and `about.json`, the version of the workspace and the commit for the About box. It needs Rust with the `wasm32-unknown-unknown` target and Node 18 or later, and no npm packages.

The page's `fonts/`, the drawing's web fonts ([decision 0009](../../docs/decisions/0009-fonts-shipped-with-the-core.md)), comes with it when it is there. Write it first, with Python 3, fontTools and brotli:

```bash
pip install fonttools brotli
python scripts/fonts.py web
```

Without it the build says so, and the drawing falls back on the fonts installed on the computer ([Fonts](../viewer-ui/README.md#fonts)).

| Option | |
| --- | --- |
| `--no-core` | Take the core as `bindings/runtime-wasm/build.mjs` last built it, without building it again. |
| `--samples` | Copy the conformance corpus's [`cases`](../../tests/conformance/README.md) and `equivalence` files into `samples/`, for trying the viewer and for tests. |

The site is 26 files and 1.36 MB, the licence of the fonts' metrics in `fonts/OFL.txt` among them, of which the core is 1.22 MB, 412 KB gzip-compressed; the web fonts add 232 files and 17.6 MB, of which a browser downloads only the slices a drawing uses, and the samples 0.70 MB.

## Serving

```bash
node apps/web/serve.mjs
```

serves `apps/web/dist/` at `http://127.0.0.1:8000/`, on this computer only; `--port N` takes another port, 0 any free one. It sends the MIME types the site needs and tells the browser not to cache, so that a rebuilt site and a changed sample are read anew. With the samples built, `http://127.0.0.1:8000/?url=samples/cases/04-hierarchy.sp` opens a netlist and its include.

## Deploying

The project site publishes it at <https://rich-netlist.github.io/rnl/viewer/>, with the samples, so that <https://rich-netlist.github.io/rnl/viewer/?url=samples/cases/04-hierarchy.sp> opens one. [`pages.yml`](../../.github/workflows/pages.yml) builds it with `--samples` whenever it, the page, the bindings, the crates, the standard library, the font script or the conformance corpus changes on `main`, and puts `dist/` at the site's `viewer/`, which the site's [page of the tools](https://rich-netlist.github.io/rnl/en/tools.html) links to ([`site`](../../site/README.md)).

Any static host serves the site: copy `dist/` as it is, at the root or under a path. It needs no server code and no special headers, only the usual MIME types:

- JavaScript (`.js`) as `text/javascript`: the page and the core's Worker are ES modules, which a browser runs only with a JavaScript type;
- `style.css` as `text/css` and the page as `text/html`;
- the core (`.wasm`) as `application/wasm`, so that the browser compiles it while it downloads; with another type it is compiled once downloaded;
- the web fonts (`.woff2`) as `font/woff2`.

The core's module Worker must come from the page's own origin, so `core/` stays next to the page. A netlist opened by URL from another origin can only be read when that server allows the site's origin with CORS headers.

## Browsers

The page keeps the browser baseline of decision 0008: Chromium 109, the last Chrome and Edge on Windows 7 and 8.1, and Safari 16; and Firefox from 114, the first with module Workers. It uses no newer feature, such as JSON import attributes, CSS nesting or `Array.prototype.toSorted`; it reads files through `<input type="file">`, a folder through its `webkitdirectory`, and dropped folders through `webkitGetAsEntry`, which all three have, and not through `showOpenFilePicker`, which only Chromium has. Its smoke tests pass in headless Chrome 155, and in Playwright's Firefox 155 and WebKit 26.6 ([Testing](#testing)).

## Testing

[`tests/viewer`](../../tests/viewer/README.md#the-web-viewer) smoke-tests the site in Chromium, Firefox and WebKit with Playwright: it builds the site with the samples, serves it with `serve.mjs`, and opens netlists by `?url=`, also by an address that redirects and at a server's root, files and a folder set on the file inputs, dropped files, a Latin-1 file, a missing include and one above the files opened, reads a served netlist again after it changed, and shows the About box. Each Document must be `rnl view`'s for the same files but for the paths that name them. [`viewer.yml`](../../.github/workflows/viewer.yml) runs them in CI.

## How it differs from the desktop viewer

- **Its own menu bar.** The page shows the menu bar of the command table itself ([`menubar.js`](../viewer-ui/README.md#modules)); Alt or F10 moves to it. File has Include File, Include Folder, Include Public Package and Open URL, and no Exit; Help has Desktop Viewer; F5 and Ctrl+R read the document again rather than reload the page.
- **File access.** A browser lets the page read only what the user chooses or drops, so included files must be chosen with the netlist, added to the pool, or be on the server of its URL. The desktop viewer reads any file by its path.
- **Reload.** A file chosen with Open File or Include Folder cannot be read again once it has changed; open it again, or drop it, which Reload reads anew.
- **About** is a dialog in the page, and the document names the browser tab instead of the window.

## Embedding

Embed the canvas with `viewer/?embed=1`. Pan, zoom, double-click a subcircuit to enter it, and use
Backspace or the floating hierarchy path to return. Keys act only while the iframe has focus.
The canvas fills the iframe and preserves its centre when resized. A floating toolbar fits,
zooms in and zooms out; labels follow the browser's Chinese or English language. Errors appear
on the canvas rather than in a dialog. Embedded preferences neither read nor write localStorage
or native settings. The ordinary web viewer and desktop viewer keep their usual behaviour.

| URL parameter | Values | Default | Meaning |
| --- | --- | --- | --- |
| `embed` | `1` | Normal viewer | Use the canvas-only web viewer. |
| `url` | Netlist URL | None | Open a file, with includes relative to its served URL. |
| `level` | Subcircuit name | Automatic, as when opening normally | Initial level. |
| `background` | `light`, `dark` | `light` | Drawing background, as in View › Background; also colours the floating controls. |
| `grid` | `dots`, `lines`, `off` | `off` | Canvas grid. |
| `names` | `1`, `0` | `1` | Show names on hover. |
| `origin` | An exact origin, e.g. `https://example.com` | Any origin | Accept messages only from this origin and reply to it. |

Send object messages to the iframe's `contentWindow`, each with `rnl: 1`:

| `type` | Other fields | Effect |
| --- | --- | --- |
| `open` | `text` and `name` (e.g. `amp.sp`), or `url` | Replace the netlist without reloading. The filename determines its dialect. |
| `level` | `name` | Display that level. |
| `fit` | None | Fit the drawing to the iframe. |
| `background` | `value`: `light` or `dark` | Change the drawing and floating controls' background. |

The iframe sends these messages to `window.parent`, also with `rnl: 1`:

| `type` | Fields | When |
| --- | --- | --- |
| `ready` | `version` | The page and WebAssembly core are ready for messages, once per page load. |
| `loaded` | `name`, `levels` (names), `level` (current name), `errors`, `warnings` (counts) | Each successful open. |
| `level` | `name` | The current level changes, including mouse and keyboard navigation, and after an open. |
| `error` | `message` | A file cannot be fetched or built, has errors, or a requested level does not exist. |

Wait for `ready` before sending commands. The initial `url`, when supplied, opens after `ready`.
When several `open` requests overlap, only the latest result is displayed or reported; late reads
and errors from older requests are discarded. A URL-backed drawing offers **Open in RNL Viewer ↗**
in a new tab with the same URL minus `embed`; opening text hides this link. `level` in the URL
applies to the initial file. Later opens use the normal automatic choice unless sent a `level` message.

This minimal example switches two netlists without fetching across origins:

```html
<button id="rc" disabled>RC</button>
<button id="divider" disabled>Divider</button>
<iframe id="rnl" title="RNL circuit" style="width:100%;height:360px;border:0"></iframe>
<script>
  const frame = document.getElementById("rnl");
  const viewer = new URL("https://rich-netlist.github.io/rnl/viewer/?embed=1");
  viewer.searchParams.set("origin", location.origin);
  const examples = {
    rc: '* RC\nR1 in out 1k\nC1 out 0 1n\n* @RNL { #R1 @PLACE(0, 0); #C1 @PLACE(6, 0) }\n.end\n',
    divider: '* Divider\nR1 in out 1k\nR2 out 0 1k\n* @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(6, 0) }\n.end\n'
  };
  for (const [name, text] of Object.entries(examples)) {
    document.getElementById(name).onclick = () => frame.contentWindow.postMessage(
      { rnl: 1, type: "open", name: `${name}.sp`, text }, viewer.origin);
  }
  window.addEventListener("message", (event) => {
    if (event.source !== frame.contentWindow || event.origin !== viewer.origin || event.data?.rnl !== 1) return;
    if (event.data.type === "ready") {
      for (const name of Object.keys(examples)) document.getElementById(name).disabled = false;
      document.getElementById("rc").click();
    }
  });
  frame.src = viewer.href;
</script>
```

**Security:** set `origin` to the embedding page's exact origin (scheme, host and port, no path),
especially when embedding on another website. With it, only matching `event.origin` messages
are accepted, and replies use that same `targetOrigin`. Without it, any origin is accepted and
replies use `"*"`. Messages without `rnl: 1` are ignored. Replies contain only the fields above,
never netlist text. The parent should also check `event.source`, `event.origin` and `rnl`.
Use an HTTP(S) server for the parent, rather than a `file:` URL with an opaque origin.

**CORS:** `open` with `url` uses the same fetch/include behaviour as `?url=` and can read only
servers that permit the viewer's cross-origin requests. Relative URLs resolve against the
viewer page, not its parent. For cross-origin examples, prefer `text` plus `name`: the parent
fetches or holds the text and sends it directly. Text opens have no sibling include files.

[`embed-demo.html`](embed-demo.html), published beside the viewer at
[`/rnl/viewer/embed-demo.html`](https://rich-netlist.github.io/rnl/viewer/embed-demo.html),
switches three conformance examples and the site's five-transistor OTA through this interface.
Build with `--samples` to include those files.

The same build also publishes `samples/brand/rnl-logo.sp`, with real RNL `@TEXT` annotations:
a large **RNL**, followed on its lower right by “logo by RNL”.
The canonical brand source used for icons stays text-free. On the project homepage, clicking
the large logo replaces the static logo and wordmark with a short embedded viewer in that same
row, keeping the following content in place. The frame is up to 640px wide and fills the available
width on narrow screens; its initial view keeps the circuit logo close to the original icon's size.
Closing with × or Escape restores the wordmark
and keeps that iframe for the next click; theme changes reach it through the `background`
message. No viewer files are requested until the first click.

For embeds at most 180px tall, fitting uses the painted paths, strokes and text ink after fonts
load, excluding the renderer's paper margin and empty font ascender/descender space. A 2px fit
margin keeps these short drawings large. The floating controls run horizontally, appearing on
hover or keyboard focus.

## Structure

| Path | Content |
| --- | --- |
| `build.mjs` | Builds the core and assembles the site in `dist/`. |
| `serve.mjs` | Serves `dist/` on this computer. |
| [`../viewer-ui/shells/web.js`](../viewer-ui/README.md#the-shell) | The web shell: opening files, a folder, a URL or what is dropped; reading only the files a document needs, with `collect`; building in the core's Worker; the About box; a web page in a new tab. |
| [`../viewer-ui/dialog.js`](../viewer-ui/README.md#modules) | The dialogs the web shell asks with: which netlist, which address, About. |
| [`../../bindings/runtime-wasm`](../../bindings/runtime-wasm/README.md) | The core as WebAssembly, and the modules that run it in a Worker and collect the files a document needs. |

[`web.yml`](../../.github/workflows/web.yml) builds the site with the samples on Linux, checks every script with `node --check`, and checks that the site is served with the right types.

See the [roadmap](../../docs/roadmap.md) for planned work.

A missing named USE-PACKAGE keeps the core error. When the public index has its exact name and version, the hint offers Include, adds that package to Library, and rebuilds the applied document while preserving edits. Otherwise it points to File › Include Public Package…. Public extensions such as `analog-canvas-extensions` are not built in.
