# web

The web viewer: the page of the RNL viewers ([`apps/viewer-ui`](../viewer-ui/README.md)) in a browser, with the shared core built as WebAssembly and run in a module Worker ([`bindings/runtime-wasm`](../../bindings/runtime-wasm/README.md)), as [decision 0006](../../docs/decisions/0006-shared-core-per-platform-frontends.md) decides. It is a static site with no server code: files opened from the computer are read in the page and sent nowhere. Like the desktop viewer ([`apps/desktop`](../desktop/README.md)) it only reads, and it shows the same panels: the drawing, the netlist text, the problems and the hierarchy, with the selection followed between the drawing and the text.

- **File › Open** (Ctrl+O) opens one or several files. With several, the netlist among them is opened; when more than one could be, the page asks which. The files a netlist includes are found among the files chosen with it, by their relative paths.
- **File › Open Folder** opens a folder: the page lists its netlists, then the files they may include, with a filter, and opens the one picked. Includes are found in the folder, and only the files the document needs are read.
- **File › Open URL** opens a netlist on a web server, and fetches the files it includes relative to the address it was served from, after any redirect, `../` included. `?url=ADDRESS` in the page's address opens one at start, resolved against the page, such as `?url=samples/cases/04-hierarchy.sp`. An include that cannot be fetched (not found, no answer, or not allowed by the server's CORS headers) is missing like any other; a netlist that cannot be fetched is an error that names its address.
- Files or a folder dropped on the page open as with Open and Open Folder.
- **File › Reload** (F5, Ctrl+R) reads the document again, not the page, keeping the level, the view and the selection. A URL is fetched again past the browser's cache. Files are read again from what the browser holds of them: a dropped file as it is now, but a file chosen with Open or Open Folder only as it was when it was chosen; once it has changed, Reload says to open it again.
- **Help › About** names the version and the commit the site was built from. The browser tab is named after the document.
- Packages found by name (16.2) are only the bundled standard packages, as with `rnl` without `--package-dir`; a package file named by path is read like an include.

Text is decoded as `rnl` decodes it, UTF-8 or else Latin-1, so a document is the one `rnl view` and the desktop viewer give for the same files, but for the paths that name them. Files opened from the computer are named in the workspace as `/files/NAME`, and a folder's as `/FOLDER/SUB/NAME`; a URL's files by their paths on the server, such as `/netlists/a/top.sp`, and those of a netlist at the root of its server under a directory named after the server, such as `/example.org/top.sp`. At the root of the workspace the core would name a file by its path, `/top.sp`, and lose the file of some of its problems.

## Building

```bash
node apps/web/build.mjs
```

builds the core with [`bindings/runtime-wasm/build.mjs`](../../bindings/runtime-wasm/README.md#building) and puts the site into `apps/web/dist/`, which git ignores: the page from `apps/viewer-ui` without its README, its `package.json` and the desktop shell; the binding's modules and the WebAssembly module in `core/`; and `about.json`, the version of the workspace and the commit for the About box. It needs Rust with the `wasm32-unknown-unknown` target and Node 18 or later, and no npm packages.

| Option | |
| --- | --- |
| `--no-core` | Take the core as `bindings/runtime-wasm/build.mjs` last built it, without building it again. |
| `--samples` | Copy the conformance corpus's [`cases`](../../tests/conformance/README.md) and `equivalence` files into `samples/`, for trying the viewer and for tests. |

The site is 25 files and 1.36 MB, of which the core is 1.22 MB, 412 KB gzip-compressed; the samples add 0.70 MB.

## Serving

```bash
node apps/web/serve.mjs
```

serves `apps/web/dist/` at `http://127.0.0.1:8000/`, on this computer only; `--port N` takes another port, 0 any free one. It sends the MIME types the site needs and tells the browser not to cache, so that a rebuilt site and a changed sample are read anew. With the samples built, `http://127.0.0.1:8000/?url=samples/cases/04-hierarchy.sp` opens a netlist and its include.

## Deploying

Any static host serves the site: copy `dist/` as it is, at the root or under a path. It needs no server code and no special headers, only the usual MIME types:

- JavaScript (`.js`) as `text/javascript`: the page and the core's Worker are ES modules, which a browser runs only with a JavaScript type;
- `style.css` as `text/css` and the page as `text/html`;
- the core (`.wasm`) as `application/wasm`, so that the browser compiles it while it downloads; with another type it is compiled once downloaded.

The core's module Worker must come from the page's own origin, so `core/` stays next to the page. A netlist opened by URL from another origin can only be read when that server allows the site's origin with CORS headers.

## Browsers

The page keeps the browser baseline of decision 0006: Chromium 109, the last Chrome and Edge on Windows 7 and 8.1, and Safari 16; and Firefox from 114, the first with module Workers. It uses no newer feature, such as JSON import attributes, CSS nesting or `Array.prototype.toSorted`; it reads files through `<input type="file">`, a folder through its `webkitdirectory`, and dropped folders through `webkitGetAsEntry`, which all three have, and not through `showOpenFilePicker`, which only Chromium has. It has been tried in headless Chrome 155 and Edge 155.

## How it differs from the desktop viewer

- **Its own menu bar.** The page shows the menu bar of the command table itself ([`menubar.js`](../viewer-ui/README.md#modules)); Alt or F10 moves to it. File has Open Folder and Open URL, and no Exit; F5 and Ctrl+R read the document again rather than reload the page.
- **File access.** A browser lets the page read only what the user chooses or drops, so the files a netlist includes must be chosen with it, be in the folder opened, or be on the server of its URL. The desktop viewer reads any file by its path.
- **Reload.** A file chosen with Open or Open Folder cannot be read again once it has changed; open it again, or drop it, which Reload reads anew.
- **About** is a dialog in the page, and the document names the browser tab instead of the window.

## Structure

| Path | Content |
| --- | --- |
| `build.mjs` | Builds the core and assembles the site in `dist/`. |
| `serve.mjs` | Serves `dist/` on this computer. |
| [`../viewer-ui/shells/web.js`](../viewer-ui/README.md#the-shell) | The web shell: opening files, a folder, a URL or what is dropped; reading only the files a document needs, with `collect`; building in the core's Worker; the About box. |
| [`../viewer-ui/dialog.js`](../viewer-ui/README.md#modules) | The dialogs the web shell asks with: which netlist, which address, About. |
| [`../../bindings/runtime-wasm`](../../bindings/runtime-wasm/README.md) | The core as WebAssembly, and the modules that run it in a Worker and collect the files a document needs. |

[`web.yml`](../../.github/workflows/web.yml) builds the site with the samples on Linux, checks every script with `node --check`, and checks that the site is served with the right types.

See the [roadmap](../../docs/roadmap.md) for planned work.
