# @rich-netlist/runtime

The core of [RNL](https://github.com/rich-netlist/rnl), the Rich Netlist, as WebAssembly for the browser and Node. It reads a SPICE or Spectre netlist with its RNL records, the drawing of a schematic written into the netlist, and gives:

- the Document a viewer draws, with an SVG of each level;
- the editor interface's snapshot;
- the netlist with an edit's operations written into it.

It is the same core as the `rnl` command and the desktop viewer, and gives the Documents they give. It bundles only the standard packages, the series 0.2 and 0.3 (including `rnl-zh`). Other packages must be supplied explicitly. Starting with the next npm version after 0.1.0-preview.14, optional package files are distributed by the [public package catalog](https://rich-netlist.github.io/rnl-packages/catalog.json), rather than the npm tarball. Published preview.14 is unchanged.

## Install

Core releases use the workspace version and `core-vX.Y.Z`, shared with the CLI. Install an exact published version:

```bash
npm i @rich-netlist/runtime@X.Y.Z  # replace X.Y.Z with the chosen published core version
```

## Use

`startCore` runs the core in a module Worker, or in a worker thread in Node:

```js
import { startCore } from "@rich-netlist/runtime";

const core = startCore();
const files = { "/top.sp": netlistText };
const document = await core.build({ entry: "/top.sp", files });
// the snapshot takes the parts to give: the host circuit and the model of each level here
const snapshot = await core.snapshot({ entry: "/top.sp", files, parts: { host: true, model: true } });
core.terminate();
```

`@rich-netlist/runtime/core` loads the module in the calling thread instead, and `@rich-netlist/runtime/worker` is the Worker itself. The requests, the Document, the snapshot and the catalog are declared in the package's TypeScript files. The [binding's README](https://github.com/rich-netlist/rnl/tree/main/bindings/runtime-wasm#readme) describes the requests, `collect` for a frontend's own file access, and `decodeText`.

## Inspection overlays

Every Document gives `levels[].usages[].frame` (bounds, title and role anchors/widths)
and `levels[].currents` (terminal, direction, pin and arrow points), in SVG pixels
with y down. These geometry records contain no colors, strokes or visibility settings.
The SVG has no inspection overlays unless requested:

```js
const document = await core.build({ entry: "/top.sp", files, overlays: {
  usages: "semantic", // "none" (default), "semantic" or "all"
  currents: true,    // false by default; unspecified currents produce no arrow
  roles: true,       // false by default; labels on enabled usage frames
  style: { palette: ["#4263eb", "#2f9e44"], width: 2, dashes: [1, 5], current_color: "#d9480f" }
} });
```

Styles are independent of RNL `@SET-STYLE`; omitted properties use deterministic renderer
defaults. Colors are hex (`#RGB`, `#RRGGBB`, `#RRGGBBAA`), the palette is nonempty,
width is positive, and dash lengths are positive (an empty array means solid).
Palette assignment follows usage order, including hidden usages. Current arrows are solid.
The same options work with lazy builds and a snapshot's drawing. Disabling overlays returns
the original SVG bytes. Viewers may render the geometry with their own styling/preferences.
Declaration currents (`#:port`) use explicitly drawn `$IO_PORT` usages on the port's net;
`in` points from the tag into the circuit. A port with no such symbol has no arrow.

## Optional Canvas package

Fetch the exact package record from the public catalog, verify its raw bytes, and explicitly add the decoded text to `files` and its path to `pool`:

```js
import { startCore } from "@rich-netlist/runtime";
const catalogUrl = "https://rich-netlist.github.io/rnl-packages/catalog.json";
const response = await fetch(catalogUrl);
if (!response.ok) throw new Error(`Catalog HTTP ${response.status}`);
const catalog = await response.json();
if (catalog.schemaVersion !== 1) throw new Error("Unsupported catalog");
const pkg = catalog.packages.find(p => p.name === "analog-canvas-extensions" && p.version === "0.1");
if (!pkg) throw new Error("Missing analog-canvas-extensions 0.1");
const download = await fetch(new URL(pkg.url, catalogUrl));
if (!download.ok) throw new Error(`Package HTTP ${download.status}`);
const bytes = await download.arrayBuffer();
const hash = Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)),
  b => b.toString(16).padStart(2, "0")).join("");
if (hash !== pkg.sha256) throw new Error("Public package SHA-256 mismatch");
const core = startCore();
const packagePath = "/library/analog-canvas-extensions.rnl";
const files = { "/top.sp": netlistText, [packagePath]: new TextDecoder().decode(bytes) };
const pool = [packagePath];
const document = await core.build({ entry: "/top.sp", files, pool });
```

The netlist still writes `@USE-PACKAGE(name="analog-canvas-extensions", version="0.1")`.
Keep the same `files` and `pool` for snapshot requests and subsequent rebuilds. Supplying only
`files` does not make a package searchable. Applications that need repeatable builds should
pin the expected SHA-256 as well as the package name and version. This repository's smoke
fixture uses its single `packages.lock.json` via `python scripts/packages.py download`.

## Catalog

`catalog` lists the symbols, definitions and sets of the packages the core has: the bundled ones and a request's own, those of its `package_dirs` among its `files`. Each package is read as the core reads it, so a symbol's pins and box are those of the drawings:

```js
const catalog = await core.catalog({ catalog: {} });
const resistor = catalog.symbols.find((s) => s.symbol === "$RESISTOR");
// resistor.package is "rnl-std-basic", resistor.version "0.3", resistor.box [-1, -2.4, 1, 2.4], and
// resistor.pins [{ terminal: 1, at: [0, 2], direction: "up" }, { terminal: 2, at: [0, -2], direction: "down" }]
```

Of each name only the newest version is listed; `{ catalog: { packages: ["rnl-std-basic"], all_versions: true } }` keeps the packages of these names, every version of each. A symbol's `definition` names the definition whose usages a rule of the packages draws with it, such as `$GROUND` for `$GROUND_SYMBOL`.

## Objects without a position

A netlist that has no positions yet lists every object in `Level.unplaced`, each with its symbol drawn alone as `svg`, about 1.3 KB each. A frontend that shows a few of them at a time builds with `lazy_unplaced: true`, which leaves those drawings out of the Document and keeps them in the core, and asks for the drawings of the objects it shows, by their places in the level's list and their names, which the core checks against the build it kept:

```js
const document = await core.build({ entry: "/top.sp", files, lazy_unplaced: true });
const level = 0;
const unplaced = document.levels[level].unplaced.slice(0, 20);
const drawings = await core.drawings({
  drawings: { level, items: unplaced.map((_, i) => i), names: unplaced.map((u) => u.name) },
});
// drawings[0].svg draws unplaced[0]
```

The request is about the last build that had `lazy_unplaced`, and is refused, with a message, when no build did, when an object is not there, or when the names are those of another build.

## Vite

A frontend built with Vite needs three settings:

- `worker: { format: "es" }`: the Worker is a module with a top-level `await`, which Vite's default format for Workers, `"iife"`, cannot bundle.
- `optimizeDeps: { exclude: ["@rich-netlist/runtime"] }`: the dev server's pre-bundling moves the modules, and `new URL("./worker.js", import.meta.url)` then names a file that is not there.
- `server.fs.allow` with the package's real directory, but only when the package is linked from outside the frontend's workspace, as `link:` does: the dev server otherwise refuses to serve `worker.js`. A package installed into `node_modules` does not need it.

Vite may say that `node:worker_threads` and `node:fs/promises` were left out for the browser: the modules import them only when they run in Node.

## Versions

The npm package and CLI share the root workspace version, released together by `core-vX.Y.Z`.
Viewer and language-standard versions are independent. Versions 0.x may change requests,
Documents and snapshots. Historical `runtime-v0.1.0-preview.N` tags published npm-only
previews (through preview.15); that trigger is retired and those versions remain unchanged.
The first core version and npm dist-tag (`next` or `latest`) are pending the user's decision.
A bare install follows npm's `latest`, which may still point to an older preview; use an exact
version until the release decision is applied. See [release procedures](../../docs/releases.md)
for CI, trusted publishing and partial-publication recovery.

## License

The JavaScript modules and the code compiled into the module are under the MIT license (`LICENSE`). The package also holds the built-in standard packages under CC0-1.0, and the metrics of the font Noto Sans SC, copyright 2014-2021 Adobe, under the SIL Open Font License 1.1 (`LICENSES/`).
