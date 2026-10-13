# crates

The Rust workspace ([`../Cargo.toml`](../Cargo.toml)). The crates implement the RNL language draft 0.2 without a solver: they read a netlist and its records, report diagnostics, build the effective model, and draw what has coordinates. The draft is not yet in [`spec/`](../spec/README.md); section numbers in code comments and diagnostics refer to it.

| Crate | Responsibility |
| --- | --- |
| [`rnl-runtime`](rnl-runtime/README.md) | Language core: records, names and references, effect rules, symbols, diagnostics, the effective model, and writeback. Reads no files. |
| [`rnl-host-spice`](rnl-host-spice/README.md) | Reads SPICE netlists into the runtime's host document. |
| [`rnl-host-spectre`](rnl-host-spectre/README.md) | Reads Spectre netlists, including `simulator lang=spice` parts. |
| [`rnl-render`](rnl-render/README.md) | Draws an effective model as basic primitives and SVG. |
| [`rnl-view`](rnl-view/README.md) | What the frontends share: dialect detection, interpretation with the standard packages bundled, level status. Reads no files. |
| [`rnl-cli`](rnl-cli/README.md) | The `rnl` command: check, render, dump, and strip generated records. |
| [`rnl-solver`](rnl-solver/README.md) | Future solver; no crate yet. |

```bash
cargo build --release -p rnl-cli
cargo test --workspace
```

The runtime, the renderer, both host adapters and `rnl-view` build for `wasm32-unknown-unknown`; CI checks this, although no WASM binding exists yet.

The crates are not published. See the [roadmap](../docs/roadmap.md) for planned work.
