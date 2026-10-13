# runtime-wasm

The shared core as WebAssembly ([decision 0006](../../docs/decisions/0006-shared-core-per-platform-frontends.md)): [`rnl_view::build`](../../crates/rnl-view/README.md), a workspace in and a Document out, behind a small C-style interface over linear memory, with a JSON request in and a JSON response out. The module imports nothing and needs no generated glue, so building it takes only Cargo. JavaScript modules load it, run it in a Worker and collect the files a document needs; the web viewer will use them.

## Building

```bash
node bindings/runtime-wasm/build.mjs
```

builds the module with the workspace's `wasm` profile, as

```bash
cargo build --profile wasm --target wasm32-unknown-unknown -p rnl-runtime-wasm
```

does, and puts it with the JavaScript modules into `dist/`, which git ignores and the web viewer copies: `rnl_runtime_wasm.wasm`, `core.js`, `worker.js`, `client.js`, and a `package.json` that makes Node read the modules as ES modules. It needs Rust with the `wasm32-unknown-unknown` target and Node 18 or later. `--out DIR` writes into another directory, and `--features test-panic` builds the module the tests use.

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
| `svg` | Whether each level's SVG is written; true when not given. |
| `primitives` | Whether each level's primitives are given; false when not given. |

Fields the core does not know are ignored, so that a frontend can ask for an option that an older core does not have. The answer is `{"document": ...}`, the Document of `rnl-view` as `rnl view` prints it, or `{"error": "..."}` when the request is not JSON of this shape or the entry is not among the files. `Document.missing` lists the files the core asked for and did not get; give them and build again.

A panic aborts, which traps: the call throws in JavaScript, with a `WebAssembly.RuntimeError`. The binding's panic hook keeps the message first, as `panicked at FILE:LINE:COL:` and the message on the next line, where `rnl_panic_len` and `rnl_panic_message` find it. Running out of memory aborts too, without a panic: the binding's allocator, which passes every allocation on to the system's, notes the size of the one that failed for `rnl_alloc_failed`. The instance must not answer another request; make a new one from the same module. Built with the Cargo feature `test-panic`, the core panics with the message in a request's `panic` field, and allocates until no memory is left when its `out_of_memory` field is true, for the tests.

The request, the answer and the panic message are buffers owned by the module, in statics, so the exports need no unsafe code: the caller writes only into the room `rnl_request` made. The only unsafe code is the allocator's.

## JavaScript modules

ES modules in [`js/`](js/), with no dependencies. They keep to the browser baseline of decision 0006, Chromium 109 and Safari 16, and run in module Workers and in Node 18 or later.

| Module | |
| --- | --- |
| `core.js` | `loadCore(wasm)` loads the module, given by URL, as a `Response`, as bytes or as a `WebAssembly.Module`, and returns `{ build(request) }`. `build` calls the core synchronously and returns the Document, or throws an `Error` with the core's message. After a trap it throws `The core stopped:` with the panic message, or with how much memory the core had when it ran out, and makes a new instance for the next call. A build blocks while it runs, so a page runs the core in a Worker. `requestText(request)` is the JSON the core reads: what `JSON.stringify` makes of the request, with a lone surrogate, which the core's JSON reader refuses, made U+FFFD, as `TextEncoder` makes it. `build` takes a request as an object or as this text. |
| `worker.js` | A module Worker that loads `core.js` and the module next to it, and answers each message `{ id, request }`, the request an object or its `requestText`, with `{ id, document }` or `{ id, error }`. |
| `client.js` | `startCore(options)` starts `worker.js` in a module Worker, or a worker thread in Node, and returns `{ build(request), terminate() }`; `build` sends the request's `requestText`, so it takes what `loadCore`'s `build` takes, such as a `Proxy` of a framework's state, and returns a promise of the Document. `collect(core, request, read)` builds a document with the files it needs: given the files known so far in the request and `read(path)`, which returns a file's text or null, it reads what `Document.missing` lists that it did not try yet, and builds again, until nothing new can be read. It returns the last Document and the files it used. The core is either `startCore`'s or `loadCore`'s. `decodeText(bytes)` makes a file's text from its bytes as `rnl` does: UTF-8 without a byte order mark, or Latin-1 when the bytes are not UTF-8. |

A frontend gives `collect` its own reading: files the user dropped, a folder, or files under a URL. It decodes the bytes it reads with `decodeText`, not with `Response.text()` or a `TextDecoder`, so that a Latin-1 netlist gives the Document the CLI and the desktop viewer give. Packages found by name are never missing, so it gives the files of its package directories in the request.

```js
import { collect, decodeText, startCore } from "./client.js";

const core = startCore();
const base = "https://example.org/netlists/";
const read = (path) => fetch(new URL(`.${path}`, base)).then((r) => (r.ok ? r.arrayBuffer().then(decodeText) : null));
const { document, files } = await collect(core, { entry: "/top.sp" }, read);
```

## Tests

```bash
cargo test -p rnl-runtime-wasm
node bindings/runtime-wasm/build.mjs --features test-panic --out target/wasm-test
node bindings/runtime-wasm/tests/core.mjs target/wasm-test
node bindings/runtime-wasm/build.mjs
cargo build -p rnl-cli
node bindings/runtime-wasm/tests/equality.mjs
```

- The Rust tests answer requests natively, the same request with the same answer every time, and call the exports as the module's caller does.
- `tests/core.mjs` tests the modules in Node: an untitled entry built from memory, a missing include listed in `Document.missing`, an error, lone surrogates, the same Document from the same request every time, a panic and running out of memory each followed by a new instance, `collect`, the core in a worker thread, and `decodeText`. Without the `test-panic` feature the tests that stop the core are skipped.
- `tests/equality.mjs` requires that native and WebAssembly builds give identical Documents. For every file of [`tests/conformance`](../../tests/conformance/README.md)'s `cases` and `equivalence`, the standard packages and a Latin-1 file it writes, the Document `rnl view` prints for the absolute path and the one `core.js` builds for the same entry, with the files `collect` reads from the disk as `rnl` reads them, must be deep-equal, numbers compared exactly (`Object.is`, so even 0 and -0 differ). `--rnl PATH` names the `rnl` command, `target/debug/rnl` by default, and `--dist DIR` the directory `build.mjs` wrote. `--view ARG` passes an argument on to `rnl view`, and `--request JSON` adds fields to every request, so that both sides get the same option: `--view --no-std --request '{"std": false}'`.

The core takes its sines, cosines and arc tangents from `libm`, a math library in Rust, so that both builds compute the same bits; the rest of its arithmetic (`+`, `-`, `*`, `/`, square roots, rounding) is correctly rounded by IEEE 754 and gives the same bits everywhere. Nothing in a Document may follow the order of a hash map, whose keys change from one build to the next, even within one instance: the corpus case `16-ties` has a hint with names equally close and several overridden statements, whose order once did, and the Rust tests and `tests/core.mjs` build it again and again. CI runs these tests in the `wasm` job of [`rust.yml`](../../.github/workflows/rust.yml).

See the [roadmap](../../docs/roadmap.md) for planned work.
