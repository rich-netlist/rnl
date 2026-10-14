# Rich Netlist (RNL)

English | [中文](README.zh.md)

RNL is written in the comments of a circuit netlist. It describes how the circuit is drawn as a schematic and what its parts mean, while the netlist alone defines the circuit. This repository holds the language standard, and is intended to grow into a portable RNL runtime, solver, viewer, editor, and standard symbol library.

**Website: [rich-netlist.github.io/rnl](https://rich-netlist.github.io/rnl/en/)**, with an [introduction for circuit designers](https://rich-netlist.github.io/rnl/en/), a [tour of the language](https://rich-netlist.github.io/rnl/en/language.html) and the [language standard 0.2](https://rich-netlist.github.io/rnl/en/spec/0.2/). Every page is also in Chinese.

**Status:** This repository holds the RNL language standard, version 0.2, with its principles and the project documents, and a directory skeleton for the toolchain. It has no parser, solver, renderer, application, or released package yet. Versions 0.x of the language make no promise of compatibility with each other.

## At a glance

```spice
* Five-transistor OTA, input stage
* @RNL(type="header") { @VERSION("0.2")}
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   %dp1 = #M1(in_p) and #M2(in_n) treated @AS $DIFFPAIR;
*   {#M1 #M2} @ALIGN("y"), i.e., same y;
*   #M5 is at @PLACE(20, 6) => it @ORIENT("R0");
*   %g0 = $GND is @ATTACH to &vss;
*   net &vss has a @WIRE connecting {#M5:3 %g0:p}
* }
.ends ota5t
.end
```

The netlist is unchanged; the record in its comments says that M1 and M2 are a differential pair (M1 in the role `in_p`, M2 in `in_n`) and share one y, where M5 goes and how it is turned, and that the net `vss` gets a ground symbol wired to M5. Words between the tokens, such as `and`, `treated` and `is at`, are free text: the standard sentences add them so that statements read like sentences, and they do not change the meaning. Section [1.3 of the standard](https://rich-netlist.github.io/rnl/en/spec/0.2/#1.3) explains every token.

## The language standard

| What | Where |
| --- | --- |
| Web page | [English](https://rich-netlist.github.io/rnl/en/spec/0.2/) (normative) and [Chinese](https://rich-netlist.github.io/rnl/spec/0.2/), each with its PDF and Markdown downloads |
| Every published form of a revision | The [GitHub releases](https://github.com/rich-netlist/rnl/releases) tagged `spec-0.2-r1`, `spec-0.2-r2`, …: PDF, single-file web page, Markdown, the chapters as a zip, and `SHA256SUMS` |
| Markdown in this repository | [`spec/rnl-0.2.md`](spec/rnl-0.2.md) (English, normative) and [`spec/rnl-0.2.zh.md`](spec/rnl-0.2.zh.md) (Chinese), assembled from the chapters in [`spec/0.2/`](spec/README.md) |
| Standard word list | [`spec/rnl-words-0.2.json`](spec/rnl-words-0.2.json) and its [schema](spec/rnl-words.schema.json), part of the standard |
| Principles | [`spec/rich-netlist.md`](spec/rich-netlist.md) (English, authoritative) and [`spec/rich-netlist.zh.md`](spec/rich-netlist.zh.md) (Chinese) |

Every form is built from the chapters of one commit and names its revision ([decision 0007](docs/decisions/0007-publishing-the-standard.md)). Language versions are worked out in a research repository and published here ([decision 0006](docs/decisions/0006-language-versions-from-research-repository.md)).

## Repository map

| Path | Intended responsibility |
| --- | --- |
| [`spec/`](spec/README.md) | The RNL language standard and its principles. |
| [`crates/`](crates/README.md) | Future Rust runtime, host adapters, shared renderer, solver, and CLI. |
| [`bindings/`](bindings/README.md) | Separate runtime and solver WASM entry points. |
| [`packages/`](packages/README.md) | TypeScript SDKs and framework independent viewer/editor cores. |
| [`apps/`](apps/README.md) | Web application and desktop shell. |
| [`libraries/`](libraries/README.md) | External data libraries, starting with standard symbols. |
| [`tests/`](tests/README.md) | Future conformance, roundtrip, and integration corpora. |
| [`examples/`](examples/README.md) | Future example documents. |
| [`scripts/`](scripts/README.md) | Project maintenance scripts, such as the build of the standard's published forms. |
| [`.github/workflows/`](.github/workflows/README.md) | CI workflows: the project site and the releases of the standard. |
| [`docs/`](docs/README.md) | Toolchain architecture proposal, roadmap, and decision records. |
| [`site/`](site/README.md) | The [project site](https://rich-netlist.github.io/rnl/en/): introduction page and language tour for circuit designers, in Chinese and English, built from one bilingual source by `site/build.py`. |

Start with the [documentation map](docs/README.md), the [toolchain architecture proposal](docs/rnl-toolchain-architecture.md) (in Chinese), and the [roadmap](docs/roadmap.md). Each component directory contains a short README describing its planned role.

## Next step

Define the profile of standard 0.2 that the first toolchain supports, assemble conformance and roundtrip corpora, and establish real Cargo and pnpm workspaces as implementation begins. The [roadmap](docs/roadmap.md) tracks this work without presenting it as complete.

## License

| Content | Paths | License |
| --- | --- | --- |
| Everything else, including code, configuration, and top-level files | everything not listed below | [MIT](LICENSE) |
| Specification text and project documentation | `spec/`, `docs/` | [CC-BY-4.0](LICENSES/CC-BY-4.0.txt) |
| Data: standard libraries, test corpora, and examples | `libraries/`, `tests/`, `examples/` | [CC0-1.0](LICENSES/CC0-1.0.txt) |

[`REUSE.toml`](REUSE.toml) records this mapping, and `reuse lint` checks it. The documents migrated from Analog Canvas are published here under CC-BY-4.0 by their copyright holder; the frozen copies in Analog Canvas remain under that repository's AGPL-3.0 license. See [decision 0002](docs/decisions/0002-licenses.md).

## Contributing

Please discuss language or module boundary changes in an issue or pull request and keep documentation status explicit. A contribution is licensed under the license of the path it changes; no sign-off or contributor agreement is required. See [CONTRIBUTING.md](CONTRIBUTING.md).
