# bindings

Entry points to the shared core for platforms that do not call Rust natively ([decision 0008](../docs/decisions/0008-shared-core-per-platform-frontends.md)). The runtime and the solver stay separate.

| Binding | |
| --- | --- |
| [`runtime-wasm`](runtime-wasm/README.md) | The core as WebAssembly: a JSON request in, a Document out, through a small C-style interface with no generated glue; and the JavaScript modules that load it, run it in a Worker and collect the files a document needs. CI requires that its Documents are those of the native build. |

A WebAssembly binding of the optional solver is future work, and has no directory here yet. See the [roadmap](../docs/roadmap.md) for planned work.
