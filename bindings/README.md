# bindings

Entry points to the shared core for platforms that do not call Rust natively ([decision 0006](../docs/decisions/0006-shared-core-per-platform-frontends.md)). The runtime and the solver stay separate.

| Binding | |
| --- | --- |
| [`runtime-wasm`](runtime-wasm/README.md) | The core as WebAssembly: a JSON request in, a Document out, through a small C-style interface with no generated glue; and the JavaScript modules that load it, run it in a Worker and collect the files a document needs. CI requires that its Documents are those of the native build. |
| [`solver-wasm`](solver-wasm/README.md) | Future WebAssembly binding of the optional solver. |

See the [roadmap](../docs/roadmap.md) for planned work.
