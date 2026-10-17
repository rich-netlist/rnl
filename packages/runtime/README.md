# @rich-netlist/runtime

The core of [RNL](https://github.com/rich-netlist/rnl), the Rich Netlist, as WebAssembly for the browser and Node. It reads a SPICE or Spectre netlist with its RNL records, the drawing of a schematic written into the netlist, and gives:

- the Document a viewer draws, with an SVG of each level;
- the editor interface's snapshot;
- the netlist with an edit's operations written into it.

It is the same core as the `rnl` command and the desktop viewer, and gives the Documents they give. It bundles the standard packages, the series 0.2 and 0.3, and `analog-canvas-extensions`, so a netlist that names them needs no package files of its own.

## Install

Only previews are published for now, under the dist-tag `next`:

```bash
npm i @rich-netlist/runtime@next
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

The previews are numbered `0.1.0-preview.N`. They follow the toolkit's `main` branch, and their requests, Documents and snapshots can still change between previews.

## License

The JavaScript modules and the code compiled into the module are under the MIT license (`LICENSE`). The module also holds the standard packages and `analog-canvas-extensions`, under CC0-1.0, and the metrics of the font Noto Sans SC, copyright 2014-2021 Adobe, under the SIL Open Font License 1.1 (`LICENSES/`).
