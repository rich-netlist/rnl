# Rich Netlist (RNL)

RNL is a proposed way to attach circuit meaning and presentation information to native netlists. This independent repository is intended to grow into a portable RNL runtime, solver, viewer, editor, and standard symbol library.

**Status:** This repository contains a directory skeleton and the RNL documents migrated from Analog Canvas. It has no parser, solver, renderer, application, build configuration, or released package. The language documents are drafts, not a published or frozen standard.

## Repository map

| Path | Intended responsibility |
| --- | --- |
| [`spec/`](spec/README.md) | RNL principles and syntax drafts; the only maintained location for language work. |
| [`crates/`](crates/README.md) | Future Rust runtime, host adapters, shared renderer, solver, and CLI. |
| [`bindings/`](bindings/README.md) | Separate runtime and solver WASM entry points. |
| [`packages/`](packages/README.md) | TypeScript SDKs and framework independent viewer/editor cores. |
| [`apps/`](apps/README.md) | Web application and desktop shell. |
| [`libraries/`](libraries/README.md) | External data libraries, starting with standard symbols. |
| [`tests/`](tests/README.md) | Future conformance, roundtrip, and integration corpora. |
| [`examples/`](examples/README.md) | Future example documents. |
| [`scripts/`](scripts/README.md) | Future project maintenance scripts. |
| [`.github/workflows/`](.github/workflows/README.md) | Future CI workflow location. |
| [`docs/`](docs/README.md) | Toolchain architecture proposal, roadmap, and decision records. |
| [`site/`](site/README.md) | Static introduction page and language tour for circuit designers, in Chinese and English, built from one bilingual source by `site/build.py`. |

Start with the [documentation map](docs/README.md), the [toolchain architecture proposal](docs/rnl-toolchain-architecture.md) (in Chinese), and the [roadmap](docs/roadmap.md). Each component directory contains a short README describing its planned role.

## Next step

Define the first precise-profile language draft and establish real Cargo and pnpm workspaces as implementation begins. The [roadmap](docs/roadmap.md) tracks this work without presenting it as complete.

## License

| Content | Paths | License |
| --- | --- | --- |
| Everything else, including code, configuration, and top-level files | everything not listed below | [MIT](LICENSE) |
| Specification text and project documentation | `spec/`, `docs/` | [CC-BY-4.0](LICENSES/CC-BY-4.0.txt) |
| Data: standard libraries, test corpora, and examples | `libraries/`, `tests/`, `examples/` | [CC0-1.0](LICENSES/CC0-1.0.txt) |

[`REUSE.toml`](REUSE.toml) records this mapping, and `reuse lint` checks it. The documents migrated from Analog Canvas are published here under CC-BY-4.0 by their copyright holder; the frozen copies in Analog Canvas remain under that repository's AGPL-3.0 license. See [decision 0002](docs/decisions/0002-licenses.md).

## Contributing

Please discuss language or module boundary changes in an issue or pull request and keep documentation status explicit. A contribution is licensed under the license of the path it changes; no sign-off or contributor agreement is required. See [CONTRIBUTING.md](CONTRIBUTING.md).
