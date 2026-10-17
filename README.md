# Rich Netlist (RNL)

English | [中文](README.zh.md)

RNL is written in the comments of a circuit netlist. It describes how the circuit is drawn as a schematic and what its parts mean, while the netlist alone defines the circuit. This repository holds the language standard, and is intended to grow into a portable RNL runtime, solver, viewer, editor, and standard symbol library.

**Website: [rich-netlist.github.io/rnl](https://rich-netlist.github.io/rnl/en/)**, with an [introduction for circuit designers](https://rich-netlist.github.io/rnl/en/), a [tour of the language](https://rich-netlist.github.io/rnl/en/language.html), the [language standard 0.2](https://rich-netlist.github.io/rnl/en/spec/0.2/), the [standard packages](https://rich-netlist.github.io/rnl/en/packages.html) with every symbol drawn, and the [viewers](https://rich-netlist.github.io/rnl/en/tools.html): one [in the browser](https://rich-netlist.github.io/rnl/viewer/) and one to download for Windows. Every page is also in Chinese.

**Status:** This repository holds the RNL language standard, version 0.2, with its principles and the project documents, and the RNL toolkit, from its first version, 0.1, with a first version of the solver ([`crates/rnl-solver`](crates/rnl-solver/README.md)). The toolkit reads SPICE and Spectre netlists with RNL records, reports diagnostics, builds the effective model, draws what has coordinates, and writes records back; it ships the `rnl` command, whose `rnl solve` writes the solver's records into a copy of the files or in place, and drafts of the standard package and its symbols. One shared core decides the drawing ([decision 0008](docs/decisions/0008-shared-core-per-platform-frontends.md)): it lays out text with the fonts it ships ([decision 0009](docs/decisions/0009-fonts-shipped-with-the-core.md)) and gives the same Documents natively and as WebAssembly ([`bindings/runtime-wasm`](bindings/runtime-wasm/README.md)). Two viewers show them with one page ([`apps/viewer-ui`](apps/viewer-ui/README.md)): a desktop viewer for Windows ([`apps/desktop`](apps/desktop/README.md)) and a web viewer ([`apps/web`](apps/web/README.md)); they open a netlist, draw its levels, show its text, diagnostics and hierarchy, follow an instance or a net between the drawing and the text, list the objects not drawn, and edit the text and apply it, saving into the files on the desktop and by a download on the web ([decision 0011](docs/decisions/0011-viewers-edit-the-text-and-write-only-what-the-user-saves.md)). The desktop viewer also solves as a preview and saves a solved copy ([decision 0010](docs/decisions/0010-desktop-viewer-solves-as-a-preview.md)); its release 0.1.0 came before editing and solving. There is no editor yet that moves devices by dragging. Versions 0.x of the language make no promise of compatibility with each other.

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
| [`crates/`](crates/README.md) | Rust runtime, SPICE and Spectre host adapters, shared renderer, the core interface for viewers, and CLI; the solver is future work. |
| [`bindings/`](bindings/README.md) | The core as WebAssembly, with JavaScript modules to run it in a Worker; the solver's binding, separate, is future work. |
| [`packages/`](packages/README.md) | TypeScript SDKs and framework independent viewer/editor cores. |
| [`apps/`](apps/README.md) | The desktop and web viewers, and the page they share. |
| [`libraries/`](libraries/README.md) | External data libraries: the standard package with its symbols, and the Chinese alias package. |
| [`tests/`](tests/README.md) | Conformance corpus, and browser tests of the viewers; future roundtrip and integration corpora. |
| [`examples/`](examples/README.md) | Future example documents. |
| [`scripts/`](scripts/README.md) | Maintenance scripts: the build of the standard's published forms, and the fonts shipped with the core. |
| [`.github/workflows/`](.github/workflows/README.md) | CI workflows: the project site, the releases of the standard and of the desktop viewer, the Rust workspace, the viewers and their browser tests. |
| [`docs/`](docs/README.md) | Toolchain architecture proposal, roadmap, and decision records. |
| [`site/`](site/README.md) | The [project site](https://rich-netlist.github.io/rnl/en/): introduction page, language tour and the page of the tools for circuit designers, in Chinese and English, built from one bilingual source by `site/build.py`; [`pages.yml`](.github/workflows/pages.yml) adds the pages of the standard and the web viewer. |

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

Or in the web viewer, which is on the site with the same samples: [rich-netlist.github.io/rnl/viewer/?url=samples/cases/03-five-transistor-ota.sp](https://rich-netlist.github.io/rnl/viewer/?url=samples/cases/03-five-transistor-ota.sp). To run it here, build its site with the samples and serve it, then open `http://127.0.0.1:8000/?url=samples/cases/03-five-transistor-ota.sp`. Both viewers draw texts in the shipped fonts once `python scripts/fonts.py web` has written them ([`scripts`](scripts/README.md)), and fall back on a system font without them.

```bash
node apps/web/build.mjs --samples
```

```bash
node apps/web/serve.mjs
```

## Next step

Check the toolkit against the published text of standard 0.2 and record the profile it supports, add roundtrip corpora, let the viewers edit RNL text and redraw it, and define the solver's input and output. The [roadmap](docs/roadmap.md) tracks this work without presenting it as complete.

## License

| Content | Paths | License |
| --- | --- | --- |
| Everything else, including code, configuration, and top-level files | everything not listed below | [MIT](LICENSE) |
| Specification text and project documentation | `spec/`, `docs/` | [CC-BY-4.0](LICENSES/CC-BY-4.0.txt) |
| Data: standard libraries, test corpora, and examples | `libraries/`, `tests/`, `examples/` | [CC0-1.0](LICENSES/CC0-1.0.txt) |
| The metrics of the fonts shipped with the core, Noto Sans SC, copyright 2014-2021 Adobe ([decision 0009](docs/decisions/0009-fonts-shipped-with-the-core.md)) | `crates/rnl-render/fonts/noto-sans-sc.metrics` | [OFL-1.1](LICENSES/OFL-1.1.txt) |

[`REUSE.toml`](REUSE.toml) records this mapping, and `reuse lint` checks it. The documents migrated from Analog Canvas are published here under CC-BY-4.0 by their copyright holder; the frozen copies in Analog Canvas remain under that repository's AGPL-3.0 license. See [decision 0002](docs/decisions/0002-licenses.md).

## Contributing

Please discuss language or module boundary changes in an issue or pull request and keep documentation status explicit. A contribution is licensed under the license of the path it changes; no sign-off or contributor agreement is required. See [CONTRIBUTING.md](CONTRIBUTING.md).
