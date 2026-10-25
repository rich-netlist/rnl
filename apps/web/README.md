# web

The web viewer: the page of the RNL viewers ([`apps/viewer-ui`](../viewer-ui/README.md)) in a browser, with the shared core built as WebAssembly and run in a module Worker ([`bindings/runtime-wasm`](../../bindings/runtime-wasm/README.md)), as [decision 0008](../../docs/decisions/0008-shared-core-per-platform-frontends.md) decides. It is a static site with no server code: files opened from the computer are read in the page and sent nowhere. Like the desktop viewer ([`apps/desktop`](../desktop/README.md)) it can edit the text and apply the changes; Download File downloads the file shown ([decision 0011](../../docs/decisions/0011-viewers-edit-the-text-and-write-only-what-the-user-saves.md)) without writing back to its source. It shows the same panels: the drawing, the netlist text, the problems and the hierarchy, with the selection followed between the drawing and the text.

- **File › Open File** (Ctrl+O) opens one or several files. With several, the one that is not mostly included (`.inc`, `.lib`, `.rnl`) is opened, whatever its name; when more than one could be the netlist, the page asks which. The files a netlist includes are found among the files chosen with it, by their relative paths.
- **File › Include Folder** adds the folder’s netlists and packages to the Library file pool, with their directory structure. Click a file in the pool to open it.
- **File › Open URL** opens a netlist on a web server, and fetches the files it includes relative to the address it was served from, after any redirect, `../` included. `?url=ADDRESS` in the page's address opens one at start, resolved against the page, such as `?url=samples/cases/04-hierarchy.sp`. An include that cannot be fetched (not found, no answer, nothing sent for 30 seconds, or not allowed by the server's CORS headers) is missing like any other; a netlist that cannot be fetched is an error that names its address.
- Dropped files open as with Open File; dropped folders are added as with Include Folder.
- **File › Load from Clipboard** (Ctrl+Shift+V) opens the clipboard's text once the browser lets the page read the clipboard, which it may ask you first: the page shows the text, with what it will be read as, a SPICE or Spectre netlist or a package, and opens it on **Load**, with file pool entries available to bare includes.
- **File › Reload** (F5) reads the document again, not the page, keeping the level, the view and the selection. A URL is fetched again past the browser's cache. Files are read again from what the browser holds of them: a dropped file as it is now, but a file chosen with Open File or Include Folder only as it was when it was chosen; once it has changed, Reload says to open it again.
- **Edit › Edit Text** (Ctrl+Q) switches the netlist text to edit, a text box for each file; **Edit › Apply and Exit Editing** (Ctrl+Enter) builds the document again from the texts held, in the core, over the files as they were read, and draws it; **File › Download File** (Ctrl+S) downloads the file shown, with the line ends it had, as UTF-8; nothing is written where a file was read from. Changes not applied are shown as such, a dot marks the tab of each file with changes not saved, and File › Reload, opening another file and closing the page ask first ([the page](../viewer-ui/README.md#editing-the-text)).
- **File › Download Project** downloads the Library as a project, one ZIP archive with its manifest, `rnl-project.json`: the files not referenced by the document, the original bytes of local files and the current edits, and how they are set up, the entry, the pool's order, the formats chosen, the level shown and Auto Include. **File › Open Project** reads one back, from the web or the desktop viewer ([Projects and manifests](#projects-and-manifests)).
- **File › Download Manifest** downloads the manifest of the Library's files by their addresses alone, when every one has one and is as it was fetched; **File › Load Manifest** opens such a manifest file as `?manifest=` would.
- **File › Export Image › Export SVG / Export PNG** saves the whole of the level shown, not the part in view, as the core draws it with its overlays (`rnl_view::overlays::svg`, as `rnl render --usages --currents` does): usages and current directions as the drawing shows them, on or off, each usage in its colour and hidden ones left out; no selection, hover or grid; an opaque paper, light or dark as the page's. The page's fonts that its texts use go into the SVG, so it shows the texts as the page does elsewhere too. The PNG is that SVG drawn by the browser at twice its size (at most 16384 pixels a side). It is named after the file and the level. The web downloads it.
- **View › Show Current Directions** draws an arrow at each terminal with `@CURRENT("in")` or `@CURRENT("out")`, off to begin with and kept in the browser; the Usage tab frames usages. Both are inspection overlays whose geometry comes from the core, the same as `rnl render --currents --usages` ([decision 0014](../../docs/decisions/0014-inspection-overlay-geometry.md)).
- **Help › About** names the version and the commit the site was built from. The browser tab is named after the document.
- The bar above the drawing shows the hierarchy path to the level shown, a segment per subcircuit instance gone down into, each going back to its level, as in the [desktop viewer](../desktop/README.md).
- Resting the pointer on an instance or a net shows its name beside it (**View › Show Names on Hover**, on to begin with and kept in the browser). A click on a label selects the instance or net it is attached to, the selected object's labels are marked, and a click on several objects at once opens a list of them beside the pointer to choose from, as in the [desktop viewer](../desktop/README.md).
- **View › Grid** shows a light grey grid under the drawing, dots or lines at every unit of the scene, coarser when zoomed out, and the axes through the origin, as in the [desktop viewer](../desktop/README.md); G shows and hides it, and the settings are kept in the browser for the next visit. The status bar shows where the pointer is on the drawing, in units with y up.
- **Help › Desktop Viewer** opens the project site's page of the tools in a new tab, where the desktop viewer is downloaded: [in Chinese](https://rich-netlist.github.io/rnl/zh/tools.html) when the browser's languages put Chinese before English, else [in English](https://rich-netlist.github.io/rnl/en/tools.html), the site's default.
- Packages found by name (16.2) come from the file pool before bundled packages; a package file named by path is tried relative to the file that references it.

Text is decoded as `rnl` decodes it, UTF-8 or else Latin-1, so a document is the one `rnl view` and the desktop viewer give for the same files, but for the paths that name them. Files opened from the computer are named in the workspace as `/NAME`, and a folder's as `/FOLDER/SUB/NAME`; a URL's files by their paths on the server that served the netlist, after any redirect, such as `/netlists/a/top.sp`, or `/top.sp` at its root, so that a URL that redirects opens as the one it redirects to. A netlist served at a directory's address is named after the directory, or after the server at its root, such as `/example.org`. The workspace has nothing above its root: an include such as `../x.inc` of a file opened from the computer is missing, not a file of that name opened with it.

## External manifests

`?manifest=ADDRESS` opens the files an external manifest lists, at a commit of a public GitHub repository, such as the manifests the [RNL gallery](https://rich-netlist.github.io/rnl/en/gallery.html) writes; `?url=` is unchanged, and the page takes one or the other. The reading is `core/manifest.js` (`bindings/runtime-wasm/js/manifest.js`, also `@rich-netlist/runtime/manifest`), which tools in Node can use too:

- The manifest is format 1; a newer format asks for a newer viewer. Its `base` and each file's `url` are https addresses (http only on this computer, as a page in development has), such as `https://raw.githubusercontent.com/` at one commit for the RNL gallery's; every file has its SHA-256 and size; its paths relative without `..`, and its files at most 300, 2 MB each and 20 MB in all.
- Every file is fetched from `base`, six at a time, with the count shown while it loads, and its bytes checked against its SHA-256 before they are decoded; a file that does not match, is missing or cannot be fetched stops the opening, and the error names it. A file with a `url` of its own, such as a public package under `.packages/`, comes from that address instead, checked the same way, without asking.
- The formats a manifest gives, `formats`, are chosen for those files as if the user had chosen them in the Library, once: a later choice, or Reload, keeps the user's.
- The files go under `/<id>/` in the workspace, read only, and the Library lists them in a group of their own, *From external manifest · id · read only*, with their directories. They are searched as the Library's files are, for bare includes, packages by name and Auto Include, by any file opened, theirs or not. The manifest's `pool` is the build's pool, in its order: a bare include falls back to it by file name, and a package by name is found in it first; relative includes resolve as usual.
- The document opens on the manifest's `level`, a subcircuit's name or "(top)" for the netlist's own level, or on `?level=` when the address gives one; a subcircuit the netlist has not is told, and the default level is shown. The level is shown in the entry's document, with all it includes, also when an included file defines the subcircuit; the embed's `level` and `?level=` do the same.
- A slim bar above the drawing shows what the manifest gives of its title (in the page's language), its author and its source, `repo@commit` linked to the manifest's `source.url`; a manifest that gives none of them has no bar.
- Reload (F5) fetches the manifest and the files again.

A manifest of links, which Download Manifest writes and Load Manifest reads, has no title or source: it opens without the bar.

In an embedded canvas, `?embed=1&manifest=` and the `open` message's `manifest` field (instead of `url`) do the same, without the bar and the Library.

## Projects and manifests

A project, a gallery design and a manifest of links have one manifest format, format 1, which `manifest.js` reads and writes (a copy of `bindings/runtime-wasm/js/manifest.js`, `@rich-netlist/runtime/manifest`, which a test keeps the same). Each file is listed by its relative path with the SHA-256 and size of its bytes, and is taken from its `url`, or from the archive the manifest is in, or from `base` joined with its path; `pool` is the build's pool in its order, `formats` the parse format chosen for some files, `level` a subcircuit's name or "(top)", `settings` the Library's (`autoInclude`), and a desktop project's `paths` record its PATH directories, whose files it copied under `PATH/<n>-<name>/` and put in `pool`.

- **Download Project** (web) and **Save Project** (desktop) write the archive. The desktop's files lose the directory all of them are in, so that includes between them resolve where the project is opened.
- **Open Project** reads the archive, checks every file against the manifest, and makes the project's pool the Library's files, with the formats and settings it gives; its entry opens at its level. The web keeps the files under `/<name>/` in the page; the desktop asks for a folder to write them in, starting where the archive is, and asks before replacing files there; its Save Project then offers to save to the archive it opened. Opening a project does not change the desktop's PATH.
- **Display settings.** A manifest may propose display settings, `display`: the view of its `level` (`{ "fit": true }`, or `{ "center": [x, y], "scale": k }` in the drawing's units and pixels per unit), the `background` (`light` or `dark`), `colors` of the colour scheme by their names, whether `usages` and `currents` are shown, and the `grid` (`none`, `dots` or `lines`). `?manifest=`, Load Manifest and Open Project list those it has, each with a box ticked, and apply the ones kept: they hold while a file of the manifest (or of the project) is shown, go back to your own settings for another file and come back, unasked, with a file of it again; they are never written into your settings, and a setting you change meanwhile is yours. The view is set when the manifest is opened. A setting this viewer does not know is told and kept. Download Project, Save Project and Download Manifest ask whether to write the display settings shown into it. Embedded viewers do not apply them.
- A project's archive holds at most 5000 files and 1 GiB, and a file may be larger than a fetched manifest allows. The web opens projects of at most 256 MB, as its WebAssembly core holds a project's texts several times over; a larger one is to be opened in the desktop viewer.

## Touch and small screens

- **On the drawing**, one finger drags the view, two zoom it around their middle as they spread or pinch, and drag it as their middle moves; a tap does what a click does (a finger may move about 10 pixels and still tap) and a double tap what a double click does, once: the page tells taps itself, as WebKit makes no double click of them. Gestures on the drawing neither scroll nor zoom the page; elsewhere the browser may zoom the page. A finger shows no hover names. The mouse, the wheel and the keys are as on a desktop, on a laptop with a touch screen too.
- **Upright and narrow** (`(max-width: 1023px) and (max-aspect-ratio: 4/5)`: a phone, a tablet held upright, a narrow window), the side panel is under the drawing, as wide as the screen, about a third of its height at first. Its handle drags it taller or lower, from its tabs to what leaves the drawing 80 pixels, and the height is kept (`panel-height`). Hide the Side Panel folds it to its tabs; a tap on a tab unfolds it.
- **A phone lying down** starts with the drawing alone; Show the Side Panel puts it beside the drawing, at most a third of the width.
- **On a touch screen** (`pointer: coarse`) the menu titles, the tabs, the side panel's buttons, the up button and the Problems heading are at least 44 pixels, the menus' rows 40, the open badge of a subcircuit instance 32. A menu taller or wider than the room it has moves left or scrolls, its submenus then under their entries. A tap on a menu item chooses it as the finger lifts.
- The status bar cuts a long name of the selection short, so the zoom stays on the screen. Embedded viewers keep their one-canvas layout.

## File pool and search paths

The first sidebar tab, **Library**, groups the session's searchable files. Opening a file with Open File, a drop, the command line (desktop), or `?url=` (web) also adds it to **Files**, without duplicates. **Include File** adds selected netlists or `.rnl` packages; **Include Folder** recursively adds files with netlist/package extensions (`sp`, `spi`, `spice`, `cir`, `ckt`, `net`, `netlist`, `inc`, `lib`, `scs`, `rnl`), preserving their directories. Click a pooled file to open it. **Exclude** removes it from the search pool, without deleting the file or closing an already open document. Pool changes rebuild the current document while keeping edited text and the view; edits not yet applied remain unapplied.

Built-in standard packages from series 0.3 and 0.2 are in the initially collapsed **Built-in** group, with **read only** badges. They can be opened but cannot be edited or excluded. The pool itself lasts for this viewer session. Files starts expanded; group collapse state lasts only for this session, and a newly added file expands Files. Matching root-file rows are highlighted across all groups; selecting an included Netlist tab does not change that highlight.

Clipboard text occupies one separate **temporary · not searched** row above the groups. It never enters include/package lookup. Click it to return to its last applied text; unapplied edits follow the usual file-switch confirmation. The next paste replaces this slot, and closing or reloading clears it.

Relative references are tried first. For a bare include filename only, a missing relative file falls back to the pool, then the desktop's PATH directories in their displayed order; a hint names the selected file. A path containing directory components does not use include fallback. Packages referenced by name search the pool, then PATH, then bundled packages. Version selection still follows 16.2: an exact version, or the specified compatible/newest version; copies of the same version follow search order. Thus a user's `rnl-std` can override the built-in copy (16.3).

Within each package directory, `<name>.rnl` is preferred, then `<version>/<name>.rnl`, then other `.rnl` files (including version directories). Every candidate's `PACKAGE` header is checked. A conventional filename with the wrong package name or no valid header produces a warning. A different filename is valid; filenames are a tool search preference, not a language requirement.

The browser cannot access arbitrary local directories. The note at the bottom of Library explains this limitation; use Include File/Folder instead. The pool is held in page memory, not IndexedDB, and is cleared when the page closes or reloads. Local files are not uploaded. Packages in the pool are passed to the WebAssembly core with the `pool` request field; relative URL includes are still fetched by `collect` before a fallback is final.

**File › Include Public Package…** reads the latest [rnl-packages catalog](https://rich-netlist.github.io/rnl-packages/catalog.json) at runtime and verifies each downloaded file with SHA-256 before adding it to the pool. It works in embedded viewers too. If the catalog is unavailable, Library shows an explanation while built-in packages remain available. Only the standard series is copied to `dist/libraries/`. `public-packages.json` configures the catalog URL; builds accept `RNL_PACKAGE_CATALOG`, or `RNL_PACKAGE_FIXTURE` for an offline fixture directory generated by `python scripts/packages.py fixture`. Tests use the locked fixture and block the online catalog. Legacy public library URL paths resolve through the catalog; transferred sessions retain package text and pool paths. If a package references another optional package, include that package too.

## Building

```bash
node apps/web/build.mjs
```

builds the core with [`bindings/runtime-wasm/build.mjs`](../../bindings/runtime-wasm/README.md#building) and puts the site into `apps/web/dist/`, which git ignores: the page from `apps/viewer-ui` without its README, its `package.json` and the desktop shell; the binding's modules and the WebAssembly module in `core/`; and `about.json`, with the independent Viewer `version` from the desktop manifests, the workspace `coreVersion` and the build `commit` for the About box. It needs Rust with the `wasm32-unknown-unknown` target and Node 18 or later, and no npm packages.

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

- **Its own menu bar.** The page shows the menu bar of the command table itself ([`menubar.js`](../viewer-ui/README.md#modules)); Alt or F10 moves to it. File has Include File, Include Folder, Include Public Package and Open URL, and no Exit; Help has Desktop Viewer; F5 reads the document again rather than reloading the page; Ctrl+R toggles usage overlays.
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
| `manifest` | Manifest URL | None | Open the files of an external manifest ([External manifests](#external-manifests)); not with `url`. |
| `level` | Subcircuit name | Automatic, as when opening normally | Initial level. |
| `background` | `light`, `dark` | `light` | Drawing background, as in View › Background; also colours the floating controls. |
| `grid` | `dots`, `lines`, `off` | `off` | Canvas grid. |
| `names` | `1`, `0` | `1` | Show names on hover. |
| `origin` | An exact origin, e.g. `https://example.com` | Any origin | Accept messages only from this origin and reply to it. |

Send object messages to the iframe's `contentWindow`, each with `rnl: 1`:

| `type` | Other fields | Effect |
| --- | --- | --- |
| `open` | `text` and `name` (e.g. `amp.sp`), `url`, or `manifest`; optional `requestId`, `packages` | Replace the netlist without reloading. The filename determines its dialect. |
| `level` | `name` | Display that level. |
| `fit` | None | Fit the drawing to the iframe. |
| `background` | `value`: `light` or `dark` | Change this page's drawing and floating controls' background without saving a user preference. |

The iframe sends these messages to `window.parent`, also with `rnl: 1`:

| `type` | Fields | When |
| --- | --- | --- |
| `ready` | `version`, `features`, `maxOpenBytes` | The page and WebAssembly core are ready for messages, once per page load. |
| `loaded` | `name`, `levels` (names), `level` (current name), `errors`, `warnings` (counts), optional `requestId` | Each successful open. |
| `level` | `name` | The current level changes, including mouse and keyboard navigation, and after an open. |
| `error` | `message`, optional `requestId`, `reason` | A file cannot be fetched or built, has errors, or a requested level does not exist. |

Wait for `ready` before sending commands. The initial `url`, when supplied, opens after `ready`.
When several `open` requests overlap, only the latest result is displayed or reported; late reads
and errors from older requests are discarded. A replaced request with `requestId` receives one
`error` with its own ID and `reason: "superseded"`; legacy requests without IDs stay silent.
A URL-backed drawing offers **Open in RNL Viewer ↗**
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

### Unified API and complete Viewer tabs

`?api=1&origin=EXACT_HTTP_OR_HTTPS_ORIGIN&token=RANDOM_TOKEN` enables the same protocol in the
complete Viewer, communicating with `window.opener` instead of the iframe's `window.parent`.
The token is a random 16–128 character alphanumeric/hyphen launch identifier, sent in every
message in both directions. Window identity and exact origin checks are shared with the existing
same-origin new-window mechanism. API tabs keep their connection for subsequent opens;
internal File › New Window still uses its existing one-use session handoff.

Both modes accept optional `requestId` (a nonempty string of at most 128 characters) and
`packages: [{ name, version }]` (at most 64 exact `major.minor` versions). Packages are resolved
by exact name/version through the public catalog and verified with SHA-256 **before** building.
They enter the build's file pool, and successful complete-Viewer imports retain them in Library.
Package failures identify the name/version in an `error` carrying the request ID. Embedded
automatic resolution of missing `@USE-PACKAGE` dependencies remains available for legacy callers.
An `open` message may occupy at most **64 MiB (67,108,864 bytes) of UTF-8 JSON**, including its
text and metadata. Larger messages return an error naming the actual and maximum byte counts;
they are never truncated. Existing calls without the new fields, including the site's Logo
easter egg, keep their existing behavior.

`ready.features` advertises `requestId` and `packages`, plus `opener` in complete API mode.
`viewer-api.json` publishes the supported complete-Viewer features and size limit, using the
same constants as `ready`. Callers can check it to report an old complete Viewer promptly:
versions predating this API emit no `ready` at all. In a complete Viewer, an incoming `open`
asks before replacing unsaved or unapplied edits; refusal returns `reason: "declined"` with
the request ID. Replaced requests return `reason: "superseded"` once; stale results never commit.

The caller must synchronously call `window.open(url, "rnl-viewer")` in the click event, with no
`noopener` and no preceding `await`. A plain `<a target="_blank">` normally severs the opener.
Its page must omit COOP or use `Cross-Origin-Opener-Policy: same-origin-allow-popups`, rather
than `same-origin`; check for dependencies on cross-origin isolation before changing that header.
Reuse an already connected window without navigating it again, so its edit confirmation can run.
Netlist text is sent only by `postMessage`, never in the URL or to an upload service.

```js
let viewer, token;
const viewerUrl = new URL("https://rich-netlist.github.io/rnl/viewer/");
const sendOpen = () => viewer.postMessage({
  rnl: 1, type: "open", token, requestId: crypto.randomUUID(),
  name: "circuit.sp", text: currentCompletedExport,
  packages: [{ name: "analog-canvas-extensions", version: "0.1" }]
}, viewerUrl.origin);
window.addEventListener("message", event => {
  const m = event.data;
  if (event.source !== viewer || event.origin !== viewerUrl.origin || m?.rnl !== 1 || m.token !== token) return;
  if (m.type === "ready") {
    if (!["requestId", "packages", "opener"].every(f => m.features?.includes(f))) {
      showError("Viewer version too old"); return;
    }
    sendOpen();
  }
  // Match loaded/error by requestId; clear that request's timer and show errors.
});
openButton.onclick = () => {
  if (viewer && !viewer.closed) { viewer.focus(); sendOpen(); return; }
  token = crypto.randomUUID();
  viewerUrl.search = new URLSearchParams({ api: "1", origin: location.origin, token });
  viewer = window.open(viewerUrl, "rnl-viewer");
  if (!viewer) showError("Allow pop-ups and retry");
};
```

Enable the button only when the current export is complete. Production callers also check
`viewer-api.json`, track readiness and request IDs, and report timeout/closed-tab failures.

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

A missing named USE-PACKAGE keeps the core error and starts a background public-catalog lookup, independently of Auto Include. Only explicit package references qualify; missing symbols and subcircuit definitions never trigger package guesses. One confirmation lists the available packages and adds them to Library only after approval, rebuilding applied text while preserving pending edits. Catalog requests (including configuration/body download) abort after ten seconds; offline failures leave normal viewing/editing and the manual Include hint available. Offers wait until editing and other dialogs finish, are cancelled on navigation, and remember a dismissal for that opened source. Version selection follows the core: exact first, the nearest compatible newer stable version, or the newest when unpinned. Series 0.x require an exact version. Public extensions such as `analog-canvas-extensions` are not built in.

File > Download File (Ctrl+S) downloads the editor's current snapshot, which clears its unsaved mark without changing the browser's original File/URL. Build overlays use a separate source baseline, so Download before Apply, later Library rebuilds and repeated editing keep downloaded edits. Native saves update that source baseline because they write the file. Typing while a save is pending remains unsaved after the earlier snapshot finishes.

File > Download Project exports one `<entry>.zip` with `rnl-project.json` and the Library's files by their paths: unused files, included public packages, loaded include files and the separate `temporary/` clipboard source, but not the built-in standard packages, which every viewer has. Current editor text is captured before choosing a destination, including pending changes, without applying it or marking it saved. Other files retain their original bytes. Browsers that support [showSaveFilePicker](https://wicg.github.io/file-system-access/#api-showsavefilepicker) ask for the ZIP destination; others use the browser's ordinary download. An unreadable file stops the export, and the error names it.

### New windows and navigation keys

E/Enter opens a selected subcircuit; Ctrl+E/Backspace goes up. Shift modifies every circuit navigation to open its target in a new viewer; Shift+E/Shift+Enter explicitly opens the selected subcircuit in a new viewer. File › New Window copies the current level, selection, saved settings and Library. Library file clicks keep the Library visible. Ctrl+Q toggles editing using physical Control on macOS too; exiting applies, Ctrl+Enter applies and exits, and Edit › Discard Changes drops pending text after confirmation. See the [shared navigation/session contract](../viewer-ui/README.md#navigation-and-new-windows).

New windows are same-origin browser tabs. The ready/session postMessage exchange transfers local files, the pool and applied text; pop-up blocking is reported. Pending editor text stays in the original tab.
