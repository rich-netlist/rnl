# runtime-wasm

The shared core as WebAssembly ([decision 0008](../../docs/decisions/0008-shared-core-per-platform-frontends.md)): [`rnl_view::build`](../../crates/rnl-view/README.md), a workspace in and a Document out, and [`rnl_edit::write`](../../crates/rnl-edit/README.md), an edit transaction's ops written into a netlist, behind a small C-style interface over linear memory, with a JSON request in and a JSON response out. The module imports nothing and needs no generated glue, so building it takes only Cargo. JavaScript modules load it, run it in a Worker and collect the files a document needs; the [web viewer](../../apps/web/README.md) uses them.

## Building

```bash
node bindings/runtime-wasm/build.mjs
```

builds the module with the workspace's `wasm` profile, as

```bash
cargo build --profile wasm --target wasm32-unknown-unknown -p rnl-runtime-wasm
```

does, and puts it with the JavaScript modules into `dist/`, which git ignores and the web viewer copies: `rnl_runtime_wasm.wasm`, `core.js`, `worker.js`, `client.js`, `packages.js`, their TypeScript declarations with those of the Document and the snapshot (`document.d.ts`, `snapshot.d.ts`, from `rnl-view`), and a `package.json` that makes Node read the modules as ES modules. It needs Rust with the `wasm32-unknown-unknown` target and Node 18 or later. `--out DIR` writes into another directory, and `--features test-panic` builds the module the tests use.

A frontend that links the result as a package, such as Analog Canvas with pnpm's `link:`, builds it with the packages it needs and a name for it:

```bash
node bindings/runtime-wasm/build.mjs --out packages/runtime/dist --packages DIR --name @rich-netlist/runtime --version 0.1.0-preview.1
```

`--packages DIR` brings the `.rnl` files of DIR with the build: `packages.js` holds them under `/rnl-bundled/packages`, and the Worker gives them to every request, after the request's own files and package directories. A newer `rnl-std` among them is the standard package at the root (16.3). `--name` and `--version` go into the `package.json`, whose `exports` name `client.js` as the package and `core.js` as `./core`. A named package is not private, and holds the README of [`packages/runtime`](../../packages/runtime/README.md) and the licenses of what it holds: MIT for the code, CC0-1.0 for the standard packages and `analog-canvas-extensions` that the module compiles in, and OFL-1.1 for the metrics of Noto Sans SC.

A frontend built with Vite needs three settings for it, as Analog Canvas found:

- `worker: { format: "es" }`: the Worker is a module with a top-level `await`, which Vite's default format for Workers, `"iife"`, cannot bundle.
- `optimizeDeps: { exclude: [the package's name] }`: the dev server's pre-bundling moves the modules, and `new URL("./worker.js", import.meta.url)` then names a file that is not there.
- `server.fs.allow` with the package's real directory, when it is linked from outside the frontend's workspace, as `link:` does: the dev server otherwise refuses to serve `worker.js` ("outside of Vite serving allow list"). A package installed into `node_modules` does not need it.

Vite may say that `node:worker_threads` and `node:fs/promises` were left out for the browser: the modules import them only when they run in Node.

The `wasm` profile is the release profile with `opt-level = "s"`, LTO, one codegen unit and no symbols; the CLI keeps the release profile. A document takes milliseconds either way, so the profile favours size; `opt-level = "z"` would save another 63 KB compressed, but builds take a third longer. Measured with Rust 1.97 and Node 24 on Windows 11, building each of the 24 files of the conformance corpus once from memory:

| Settings | Size (bytes) | gzip, level 9 | The 24 files |
| --- | --- | --- | --- |
| `--release` (`opt-level = 3`) | 1,817,885 | 548,195 | 63 ms |
| `opt-level = 3`, LTO, one unit, no symbols | 1,433,823 | 502,843 | 61 ms |
| `opt-level = "s"`, LTO, one unit, no symbols: the `wasm` profile | 1,198,139 | 406,793 | 72 ms |
| `opt-level = "z"`, LTO, one unit, no symbols | 1,016,022 | 343,232 | 98 ms |

The module uses the WebAssembly features the target enables by default: bulk memory, sign extension, non-trapping float-to-int conversion, multiple values, reference types and mutable globals. Chromium 109, Safari 16 and Node 18 have them all.

## The interface

The exports, in [`src/lib.rs`](src/lib.rs). Addresses and lengths are `i32`; read them as unsigned (`>>> 0`).

| Export | |
| --- | --- |
| `memory` | The module's memory. |
| `rnl_request(len)` | Make room for a request of `len` bytes, and return where to write it. |
| `rnl_build()` | Answer the request written there, and return the length of the answer. |
| `rnl_response()` | Where the answer is. It stays there until the next call. |
| `rnl_panic_len()`, `rnl_panic_message()` | The length of the message of the panic that stopped the last call, 0 when there was none, and where it is. |
| `rnl_alloc_failed()` | The size in bytes of an allocation that failed in the last call, 0 when none did. |

A call writes the request as UTF-8 JSON, calls `rnl_build`, and reads the answer as UTF-8 JSON. The request:

```json
{
  "entry": "/w/top.sp",
  "files": { "/w/top.sp": "* top\n.include \"lib/parts.inc\"\n...", "/w/lib/parts.inc": "..." },
  "package_dirs": ["/w/packages"],
  "dialect": "spice",
  "std": true
}
```

| Field | |
| --- | --- |
| `entry` | The path of the file to open, as the frontend names it. Required. [`rnl-view`](../../crates/rnl-view/README.md) says how to name the files of a workspace: by rooted paths, such as `/w/top.sp`, or by absolute ones. |
| `files` | The texts of the workspace by path, the entry's among them. Paths are read as the runtime reads them: `\` as `/`, and without `.` and `..`. |
| `package_dirs` | The directories, among the paths of `files`, whose `.rnl` files are packages found by name (16.2). |
| `dialect` | `"spice"` or `"spectre"`, instead of guessing from the entry. |
| `std` | Whether the standard package is used at the root (16.3); true when not given. |
| `std_version` | The version of the standard package at the root, whatever the files' language version, such as `"0.2"` to read a file of 0.3 with the series 0.2; a version pinned in a header record still comes first (16.3). By the language version when not given. |
| `svg` | Whether each level's SVG is written; true when not given. |
| `primitives` | Whether each level's primitives are given; false when not given. |
| `status` | Whether each level gets its solve status, `solve`, what a frontend decides from whether to solve it ([`rnl-view`](../../crates/rnl-view/README.md)); false when not given. |
| `parts` | `{"drawing", "host", "model"}`: the parts of the editor interface's snapshot to give ([`rnl-view`](../../crates/rnl-view/README.md#snapshots)), the Document unless `drawing` is false, each level's host circuit and model when they are true. With it, the answer is a snapshot. |

Fields the core does not know are ignored, so that a frontend can ask for an option that an older core does not have. So is `draft`, which earlier cores accepted with no effect: the files that declare 0.3, and a main file without a header record, are read in 0.3 without any switch (16.1). The answer is `{"document": ...}`, the Document of `rnl-view` as `rnl view` prints it, or with `parts` `{"snapshot": ...}`, as `rnl snapshot` prints it; or `{"error": "..."}` when the request is not JSON of this shape or the entry is not among the files. `Document.missing` lists the files the core asked for and did not get; give them and build again.

A panic aborts, which traps: the call throws in JavaScript, with a `WebAssembly.RuntimeError`. The binding's panic hook keeps the message first, as `panicked at FILE:LINE:COL:` and the message on the next line, where `rnl_panic_len` and `rnl_panic_message` find it. Running out of memory aborts too, without a panic: the binding's allocator, which passes every allocation on to the system's, notes the size of the one that failed for `rnl_alloc_failed`. The instance must not answer another request; make a new one from the same module. Built with the Cargo feature `test-panic`, the core panics with the message in a request's `panic` field, and allocates until no memory is left when its `out_of_memory` field is true, for the tests.

The request, the answer and the panic message are buffers owned by the module, in statics, so the exports need no unsafe code: the caller writes only into the room `rnl_request` made. The only unsafe code is the allocator's.

### Writing

A request with a `write` field writes an edit transaction's ops into a netlist ([`rnl-edit`](../../crates/rnl-edit/README.md)), and builds what was written:

```json
{
  "write": { "dialect": "spice", "netlist": "\n.subckt rc VSS\n...", "author": "analog-canvas", "ops": [] },
  "svg": false,
  "primitives": false
}
```

The netlist is the entry, under `/rnl-edit/netlist.sp`, or `.scs` for Spectre; a `file` in `write`, the name the netlist is saved under, such as `"file": "top.net"`, puts it under that name instead, so that a Spectre netlist that is not a `.scs` file is read in the SPICE mode up to its `simulator lang=spectre`, and its header and records go where that reading keeps them (2.2). A request without it writes as before; `files`, `package_dirs`, `std`, `std_version`, `svg` and `primitives` are a build's. The answer is `{"written": {"text": "...", "document": ...}}`: the netlist with the records written, and its Document, whose `diagnostics` say how the records read. An op that cannot be written is `{"error": "op 3 (set): ..."}`.

## JavaScript modules

ES modules in [`js/`](js/), with no dependencies. They keep to the browser baseline of decision 0008, Chromium 109 and Safari 16, and run in module Workers and in Node 18 or later.

| Module | |
| --- | --- |
| `core.js` | `loadCore(wasm)` loads the module, given by URL, as a `Response`, as bytes or as a `WebAssembly.Module`, and returns `{ build(request), snapshot(request), write(request), respond(request) }`. `build` calls the core synchronously and returns the Document, or throws an `Error` with the core's message; `snapshot` returns the snapshot of a request with `parts`, and `write` `{ text, document }` for a write request, in the same way; `respond` returns the core's answer as it gives it. After a trap they throw `The core stopped:` with the panic message, or with how much memory the core had when it ran out, and make a new instance for the next call. A build blocks while it runs, so a page runs the core in a Worker. `requestText(request)` is the JSON the core reads: what `JSON.stringify` makes of the request, with a lone surrogate, which the core's JSON reader refuses, made U+FFFD, as `TextEncoder` makes it. The calls take a request as an object or as this text. |
| `worker.js` | A module Worker that loads `core.js` and the module next to it, and answers each message `{ id, request }`, the request an object or its `requestText`, with `{ id, document }`, `{ id, snapshot }`, `{ id, written }` or `{ id, error }`. Every request gets the packages of `packages.js`. |
| `packages.js` | The packages the build brings, as `build.mjs --packages` fills it in: none in the web viewer's. |
| `client.js` | `startCore(options)` starts `worker.js` in a module Worker, or a worker thread in Node, and returns `{ build(request), snapshot(request), write(request), terminate() }`; `build` sends the request's `requestText`, so it takes what `loadCore`'s `build` takes, such as a `Proxy` of a framework's state, and returns a promise of the Document, `snapshot` a promise of the snapshot, and `write` a promise of `{ text, document }`. The Worker's URL is written out in full, `new Worker(new URL("./worker.js", import.meta.url), { type: "module" })`, so that a bundler such as Vite finds the Worker and bundles it; `options.worker` names another. `collect(core, request, read)` builds a document with the files it needs: given the files known so far in the request and `read(path)`, which returns a file's text or null, it reads what `Document.missing` lists that it did not try yet, and builds again, until nothing new can be read. It returns the last Document and the files it used. The core is either `startCore`'s or `loadCore`'s. `decodeText(bytes)` makes a file's text from its bytes as `rnl` does: UTF-8 without a byte order mark, or Latin-1 when the bytes are not UTF-8. |
| `core.d.ts`, `client.d.ts` | TypeScript declarations of `core.js` and `client.js`, written by hand, with the requests and the ops a write takes. |

A frontend gives `collect` its own reading: files the user dropped, a folder, or files under a URL. It decodes the bytes it reads with `decodeText`, not with `Response.text()` or a `TextDecoder`, so that a Latin-1 netlist gives the Document the CLI and the desktop viewer give. Packages found by name are never missing, so it gives the files of its package directories in the request.

```js
import { collect, decodeText, startCore } from "./client.js";

const core = startCore();
const base = "https://example.org/netlists/";
const read = (path) => fetch(new URL(`.${path}`, base)).then((r) => (r.ok ? r.arrayBuffer().then(decodeText) : null));
const { document, files } = await collect(core, { entry: "/top.sp" }, read);
```

## Publishing

[`publish-runtime.yml`](../../.github/workflows/publish-runtime.yml) publishes the package to npm as [`@rich-netlist/runtime`](https://www.npmjs.com/package/@rich-netlist/runtime). A pushed tag `runtime-v<version>`, such as `runtime-v0.1.0-preview.7`, builds it as above, its version the tag's and without `--packages`, as the core bundles the standard packages. The workflow then tests its modules with `tests/core.mjs`, checks with [`tests/package.mjs`](tests/package.mjs) what npm gets, and publishes it under the dist-tag `next`, with provenance:

```bash
git tag runtime-v0.1.0-preview.7
git push origin runtime-v0.1.0-preview.7
```

Only versions with a pre-release suffix are published for now: a tag without one fails. A pull request that changes the package builds and checks it the same way, and so does the workflow run by hand, which publishes only with `dry_run` set to false on a `runtime-v` tag. The build runs in a job of its own, which cannot publish; the publishing job publishes the tarball the build packed.

npm takes the publish by trusted publishing: the package's settings on npmjs.com name the repository `rich-netlist/rnl` and the workflow `publish-runtime.yml`, so the workflow uses and stores no token ([decision 0003](../../docs/decisions/0003-npm-scope.md)). The first two previews, 0.1.0-preview.8 and 0.1.0-preview.9, were published with a one-off token, revoked since.

A frontend installs the previews with

```bash
npm i @rich-netlist/runtime@next
```

and sets up Vite as above; installed into `node_modules`, the package needs no `server.fs.allow`.

## Tests

```bash
cargo test -p rnl-runtime-wasm
node bindings/runtime-wasm/build.mjs --features test-panic --out target/wasm-test
node bindings/runtime-wasm/tests/core.mjs target/wasm-test
node bindings/runtime-wasm/build.mjs
cargo build -p rnl-cli
node bindings/runtime-wasm/tests/equality.mjs
node bindings/runtime-wasm/tests/equality.mjs --snapshot
```

- The Rust tests answer requests natively, builds and writes, the same request with the same answer every time, and call the exports as the module's caller does.
- `tests/core.mjs` tests the modules in Node: an untitled entry built from memory, a missing include listed in `Document.missing`, an error, lone surrogates, the same Document from the same request every time, a panic and running out of memory each followed by a new instance, `collect`, the core in a worker thread, a write through the core and through the Worker, the packages a build brings given to every request (in a copy of the build with a `packages.js` of its own), and `decodeText`. Without the `test-panic` feature the tests that stop the core are skipped.
- [`packages/runtime/smoke`](../../packages/runtime/smoke/README.md) installs the package into a minimal frontend built with Vite, the tarball before a publish and the version from npm after, and runs it in Chromium from the build and from the dev server.
- `tests/package.mjs DIR VERSION` checks a named build as npm gets it, before [`publish-runtime.yml`](../../.github/workflows/publish-runtime.yml) publishes it: the files `npm pack` puts into the tarball, and nothing else; the name, the version, not private, public, the license, the repository that npm's provenance checks, and exports whose files are there; and no packages of the build's own.
- `tests/equality.mjs` requires that native and WebAssembly builds give identical Documents. For every file of [`tests/conformance`](../../tests/conformance/README.md)'s `cases`, `equivalence` and `cases-0.3`, the standard packages and a Latin-1 file it writes, the Document `rnl view` prints for the absolute path and the one `core.js` builds for the same entry, with the files `collect` reads from the disk as `rnl` reads them, must be deep-equal, numbers compared exactly (`Object.is`, so even 0 and -0 differ). `--rnl PATH` names the `rnl` command, `target/debug/rnl` by default, and `--dist DIR` the directory `build.mjs` wrote. `--view ARG` passes an argument on to `rnl view`, and `--request JSON` adds fields to every request, so that both sides get the same option: `--view --no-std --request '{"std": false}'`, or `--view --status --request '{"status": true}'`. With `--snapshot` it compares the editor interface's snapshots instead, with the host circuit and the model: the one `rnl snapshot` prints and the one the core gives for the files `collect` read, the revision included, so that the fingerprint is the same in both builds.

The core takes its sines, cosines and arc tangents from `libm`, a math library in Rust, so that both builds compute the same bits; the rest of its arithmetic (`+`, `-`, `*`, `/`, square roots, rounding) is correctly rounded by IEEE 754 and gives the same bits everywhere. Nothing in a Document may follow the order of a hash map, whose keys change from one build to the next, even within one instance: the corpus case `16-ties` has a hint with names equally close and several overridden statements, whose order once did, and the Rust tests and `tests/core.mjs` build it again and again. CI runs these tests in the `wasm` job of [`rust.yml`](../../.github/workflows/rust.yml).

See the [roadmap](../../docs/roadmap.md) for planned work.
