# Rich Netlist (RNL)

RNL is a proposed way to attach circuit meaning and presentation information to native netlists. This independent repository is intended to grow into a portable RNL runtime, solver, viewer, editor, and standard symbol library.

**Status:** This repository contains the RNL documents migrated from Analog Canvas and a first Rust toolkit without a solver. The toolkit reads SPICE and Spectre netlists with RNL records, reports diagnostics, builds the effective model, draws what has coordinates as SVG, and writes records back; it ships the draft standard package and symbols and the `rnl` command. It implements language draft 0.2, which is not yet in [`spec/`](spec/README.md); the drafts there are older. A small desktop viewer ([`apps/desktop`](apps/desktop/README.md)) opens a netlist, draws it, and lists its diagnostics. The core also builds for WebAssembly ([`bindings/runtime-wasm`](bindings/runtime-wasm/README.md)) and gives the same Documents there. There is no solver, web viewer, shared viewer package, editor, or released package. The language documents are drafts, not a published or frozen standard.

## Repository map

| Path | Intended responsibility |
| --- | --- |
| [`spec/`](spec/README.md) | RNL principles and syntax drafts; the only maintained location for language work. |
| [`crates/`](crates/README.md) | Rust runtime, SPICE and Spectre host adapters, shared renderer, the core interface for viewers, and CLI; the solver is future work. |
| [`bindings/`](bindings/README.md) | The core as WebAssembly, with JavaScript modules to run it in a Worker; the solver's binding, separate, is future work. |
| [`packages/`](packages/README.md) | TypeScript SDKs and framework independent viewer/editor cores. |
| [`apps/`](apps/README.md) | Desktop viewer; future web application. |
| [`libraries/`](libraries/README.md) | External data libraries: the standard package with its symbols, and the Chinese alias package. |
| [`tests/`](tests/README.md) | Conformance corpus; future roundtrip and integration corpora. |
| [`examples/`](examples/README.md) | Future example documents. |
| [`scripts/`](scripts/README.md) | Future project maintenance scripts. |
| [`.github/workflows/`](.github/workflows/README.md) | CI workflows for the introduction page and the Rust workspace. |
| [`docs/`](docs/README.md) | Toolchain architecture proposal, roadmap, and decision records. |
| [`site/`](site/README.md) | Static introduction page for circuit designers, in Chinese and English, built from one bilingual source by `site/build.py`. |

Start with the [documentation map](docs/README.md), the [toolchain architecture proposal](docs/rnl-toolchain-architecture.md) (in Chinese), and the [roadmap](docs/roadmap.md). Each component directory contains a short README describing its planned role.

## Trying it

```bash
cargo run --release -p rnl-cli -- check tests/conformance/cases/03-five-transistor-ota.sp
cargo run --release -p rnl-cli -- render tests/conformance/cases/03-five-transistor-ota.sp -o target/svg
```

See [`crates/rnl-cli`](crates/rnl-cli/README.md) for the other commands. To look at a netlist in the desktop viewer instead:

```bash
cargo run --manifest-path apps/desktop/Cargo.toml -- tests/conformance/cases/03-five-transistor-ota.sp
```

## Next step

Bring language draft 0.2 into `spec/`, so that the toolkit and its corpus refer to a draft in this repository. Then define the solver's input and output, and add the web viewer and the viewer core. The [roadmap](docs/roadmap.md) tracks this work without presenting it as complete.

## License

| Content | Paths | License |
| --- | --- | --- |
| Everything else, including code, configuration, and top-level files | everything not listed below | [MIT](LICENSE) |
| Specification text and project documentation | `spec/`, `docs/` | [CC-BY-4.0](LICENSES/CC-BY-4.0.txt) |
| Data: standard libraries, test corpora, and examples | `libraries/`, `tests/`, `examples/` | [CC0-1.0](LICENSES/CC0-1.0.txt) |
| The metrics of the fonts shipped with the core, Noto Sans SC, copyright 2014-2021 Adobe ([decision 0007](docs/decisions/0007-fonts-shipped-with-the-core.md)) | `crates/rnl-render/fonts/noto-sans-sc.metrics` | [OFL-1.1](LICENSES/OFL-1.1.txt) |

[`REUSE.toml`](REUSE.toml) records this mapping, and `reuse lint` checks it. The documents migrated from Analog Canvas are published here under CC-BY-4.0 by their copyright holder; the frozen copies in Analog Canvas remain under that repository's AGPL-3.0 license. See [decision 0002](docs/decisions/0002-licenses.md).

## Contributing

Please discuss language or module boundary changes in an issue or pull request and keep documentation status explicit. A contribution is licensed under the license of the path it changes; no sign-off or contributor agreement is required. See [CONTRIBUTING.md](CONTRIBUTING.md).
