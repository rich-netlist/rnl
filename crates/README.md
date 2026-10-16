# crates

The Rust workspace ([`../Cargo.toml`](../Cargo.toml)). The crates implement the RNL language standard 0.2 ([`spec/`](../spec/README.md)) without a solver: they read a netlist and its records, report diagnostics, build the effective model, and draw what has coordinates. They were written against the draft the standard was consolidated from; section numbers in code comments and diagnostics are the standard's.

| Crate | Responsibility |
| --- | --- |
| [`rnl-runtime`](rnl-runtime/README.md) | Language core: records, names and references, effect rules, symbols, diagnostics, the effective model, and writeback. Reads no files. |
| [`rnl-host-spice`](rnl-host-spice/README.md) | Reads SPICE netlists into the runtime's host document. |
| [`rnl-host-spectre`](rnl-host-spectre/README.md) | Reads Spectre netlists, including `simulator lang=spice` parts. |
| [`rnl-render`](rnl-render/README.md) | Draws an effective model as basic primitives and SVG. |
| [`rnl-view`](rnl-view/README.md) | The core interface for frontends: a workspace in, a Document out, with the standard packages bundled; and the editor interface's snapshot. Reads no files. |
| [`rnl-edit`](rnl-edit/README.md) | Writing RNL into host netlists: the ops of an edit transaction into a netlist without RNL, in the standard form. Reads no files. |
| [`rnl-cli`](rnl-cli/README.md) | The `rnl` command: check, render, dump, view, snapshot, strip generated records, and write a transaction's ops. Its `FsProvider` reads files for the desktop viewer too. |
| [`rnl-solver`](rnl-solver/README.md) | Future solver; no crate yet. |

```bash
cargo build --release -p rnl-cli
cargo test --workspace
```

The runtime, the renderer, both host adapters and `rnl-view` build for `wasm32-unknown-unknown`, and take their sines, cosines and arc tangents from `libm`, so that native and WebAssembly builds compute the same bits. The binding in [`bindings/runtime-wasm`](../bindings/runtime-wasm/README.md) builds them into one module; CI requires that its Documents are those of the native build.

The crates are not published. See the [roadmap](../docs/roadmap.md) for planned work.
