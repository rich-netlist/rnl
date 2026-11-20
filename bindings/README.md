# bindings

Entry points to the shared core for platforms that do not call Rust natively ([decision 0008](../docs/decisions/0008-shared-core-per-platform-frontends.md)). The runtime and the solver stay separate.

| Binding | |
| --- | --- |
| [`runtime-wasm`](runtime-wasm/README.md) | The core as WebAssembly: a JSON request in, a Document out, through a small C-style interface with no generated glue; and the JavaScript modules that load it, run it in a Worker and collect the files a document needs. CI requires that its Documents are those of the native build. |
| [`solver-wasm`](solver-wasm/README.md) | The optional solver as WebAssembly, in a module of its own that a page loads only when it solves: a solve request in, the Document drawn as solved, the report and the texts the solve wrote out, as `rnl preview` gives them. |

See the [roadmap](../docs/roadmap.md) for planned work.
