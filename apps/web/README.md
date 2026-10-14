# web

The web viewer: the page of the RNL viewers ([`apps/viewer-ui`](../viewer-ui/README.md)) in a browser, with the shared core built as WebAssembly and run in a module Worker ([`bindings/runtime-wasm`](../../bindings/runtime-wasm/README.md)), as [decision 0008](../../docs/decisions/0008-shared-core-per-platform-frontends.md) decides. It is a static site with no server code: files opened from the computer are read in the page and sent nowhere. Like the desktop viewer ([`apps/desktop`](../desktop/README.md)) it only reads, and it shows the same panels: the drawing, the netlist text, the problems and the hierarchy, with the selection followed between the drawing and the text.

- **File › Open** (Ctrl+O) opens one or several files. With several, the one that is not mostly included (`.inc`, `.lib`, `.rnl`) is opened, whatever its name; when more than one could be the netlist, the page asks which. The files a netlist includes are found among the files chosen with it, by their relative paths.
- **File › Open Folder** opens a folder: the page lists its files, the netlists first (`.sp`, `.scs` and the like), then the files they may include, then the others, with a filter, and opens the one picked. Includes are found in the folder, and only the files the document needs are read.
- **File › Open URL** opens a netlist on a web server, and fetches the files it includes relative to the address it was served from, after any redirect, `../` included. `?url=ADDRESS` in the page's address opens one at start, resolved against the page, such as `?url=samples/cases/04-hierarchy.sp`. An include that cannot be fetched (not found, no answer, nothing sent for 30 seconds, or not allowed by the server's CORS headers) is missing like any other; a netlist that cannot be fetched is an error that names its address.
- Files or a folder dropped on the page open as with Open and Open Folder.
- **File › Reload** (F5, Ctrl+R) reads the document again, not the page, keeping the level, the view and the selection. A URL is fetched again past the browser's cache. Files are read again from what the browser holds of them: a dropped file as it is now, but a file chosen with Open or Open Folder only as it was when it was chosen; once it has changed, Reload says to open it again.
- **Help › About** names the version and the commit the site was built from. The browser tab is named after the document.
- **Help › Desktop Viewer** opens the project site's page of the tools in a new tab, where the desktop viewer is downloaded: [in English](https://rich-netlist.github.io/rnl/en/tools.html) when the browser's languages put English before Chinese, else [in Chinese](https://rich-netlist.github.io/rnl/tools.html).
- Packages found by name (16.2) are only the bundled standard packages, as with `rnl` without `--package-dir`; a package file named by path is read like an include.

Text is decoded as `rnl` decodes it, UTF-8 or else Latin-1, so a document is the one `rnl view` and the desktop viewer give for the same files, but for the paths that name them. Files opened from the computer are named in the workspace as `/NAME`, and a folder's as `/FOLDER/SUB/NAME`; a URL's files by their paths on the server that served the netlist, after any redirect, such as `/netlists/a/top.sp`, or `/top.sp` at its root, so that a URL that redirects opens as the one it redirects to. A netlist served at a directory's address is named after the directory, or after the server at its root, such as `/example.org`. The workspace has nothing above its root: an include such as `../x.inc` of a file opened from the computer is missing, not a file of that name opened with it.

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

- **Its own menu bar.** The page shows the menu bar of the command table itself ([`menubar.js`](../viewer-ui/README.md#modules)); Alt or F10 moves to it. File has Open Folder and Open URL, and no Exit; Help has Desktop Viewer; F5 and Ctrl+R read the document again rather than reload the page.
- **File access.** A browser lets the page read only what the user chooses or drops, so the files a netlist includes must be chosen with it, be in the folder opened, or be on the server of its URL. The desktop viewer reads any file by its path.
- **Reload.** A file chosen with Open or Open Folder cannot be read again once it has changed; open it again, or drop it, which Reload reads anew.
- **About** is a dialog in the page, and the document names the browser tab instead of the window.

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
