# Rich Netlist (RNL)

RNL is written in the comments of a circuit netlist. It describes how the circuit is drawn as a schematic and what its parts mean, while the netlist alone defines the circuit. This repository holds the language standard and its toolkit: a portable runtime, a solver, the `rnl` command, a desktop viewer and a web viewer, the standard symbol library, and a skill package for AI agents. An editor that moves devices by dragging is not built yet.

**Website: [rich-netlist.github.io/rnl](https://rich-netlist.github.io/rnl/en/)**, with an [introduction for circuit designers](https://rich-netlist.github.io/rnl/en/), a [tour of the language](https://rich-netlist.github.io/rnl/en/language.html), the [language standard 0.3](https://rich-netlist.github.io/rnl/en/spec/0.3/), the [standard packages](https://rich-netlist.github.io/rnl/en/packages.html) with every symbol drawn, and the [viewers](https://rich-netlist.github.io/rnl/en/tools.html): one [in the browser](https://rich-netlist.github.io/rnl/viewer/) and one to download for Windows. Every page is also in Chinese.

**Status:** This repository holds the RNL language standard, version 0.3, now in its revision 2, which replaces 0.2 (tools still read files of 0.2), with its principles and the project documents, and the RNL toolkit, released on two lines ([release lines](docs/releases.md)): the core, that is the `rnl` command and the npm package, at 0.5.0, and RNL Viewer at 0.5.0. The toolkit reads SPICE and Spectre netlists with RNL records, reports diagnostics, builds the effective model, draws what has coordinates, and writes records back. It ships the `rnl` command; the standard packages of the series 0.3 and 0.2 with their symbols, and the package of the Analog Canvas symbols that the series 0.3 does not draw; a skill package that teaches an AI agent to read, write, check and draw RNL ([`skills/rnl`](skills/README.md)); and a solver ([`crates/rnl-solver`](crates/rnl-solver/README.md)), whose `rnl solve` places and routes what has no coordinates and writes its records into a copy of the files or in place. The solver has no WebAssembly binding yet, and a second version of its objectives is in progress. One shared core decides the drawing ([decision 0008](docs/decisions/0008-shared-core-per-platform-frontends.md)): it lays out text with the fonts it ships ([decision 0009](docs/decisions/0009-fonts-shipped-with-the-core.md)) and gives the same Documents natively and as WebAssembly ([`bindings/runtime-wasm`](bindings/runtime-wasm/README.md)); the WebAssembly build is published to npm as `@rich-netlist/runtime` ([`packages/runtime`](packages/runtime/README.md)). Two viewers show the Documents with one page ([`apps/viewer-ui`](apps/viewer-ui/README.md)): a desktop viewer for Windows, Linux and macOS ([`apps/desktop`](apps/desktop/README.md)) and a web viewer ([`apps/web`](apps/web/README.md)); they open a netlist, draw its levels, show its text, diagnostics and hierarchy, follow an instance or a net between the drawing and the text, list the objects that are not drawn (those that have no position, and the transistors whose model card is not found), and edit the text and apply it, saving into the files on the desktop and by a download on the web ([decision 0011](docs/decisions/0011-viewers-edit-the-text-and-write-only-what-the-user-saves.md)). The desktop viewer also solves as a preview and saves a solved copy ([decision 0010](docs/decisions/0010-desktop-viewer-solves-as-a-preview.md)). There is no editor yet that moves devices by dragging. Versions 0.x of the language make no promise of compatibility with each other.

## At a glance

```spice
* Five-transistor OTA, input stage
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   %dp1 = #M1(in_p) and #M2(in_n) treated @AS $DIFFERENTIAL_PAIR;
*   {#M1 #M2} @ALIGN("y"), i.e., same y;
*   #M5 is at @PLACE(20, 6) => it @ORIENT("R0");
*   %g0 = $GROUND is @ATTACH to &vss;
*   net &vss has a @WIRE connecting {#M5:3 %g0:p}
* }
.ends ota5t
.end
```

The netlist is unchanged; the record in its comments says that M1 and M2 are a differential pair (M1 in the role `in_p`, M2 in `in_n`) and share one y, where M5 goes and how it is turned, and that the net `vss` gets a ground symbol wired to M5. Words between the tokens, such as `and`, `treated` and `is at`, are free text: the standard sentences add them so that statements read like sentences, and they do not change the meaning. Section [1.3 of the standard](https://rich-netlist.github.io/rnl/en/spec/0.3/#1.3) explains every token.

## The language standard

| What | Where |
| --- | --- |
| Web page | [English](https://rich-netlist.github.io/rnl/en/spec/0.3/) (normative) and [Chinese](https://rich-netlist.github.io/rnl/zh/spec/0.3/), each with its PDF and Markdown downloads; version 0.2 stays at [English](https://rich-netlist.github.io/rnl/en/spec/0.2/) and [Chinese](https://rich-netlist.github.io/rnl/zh/spec/0.2/) |
| Every published form of a revision | The [GitHub releases](https://github.com/rich-netlist/rnl/releases) tagged `spec-0.3-r2`, `spec-0.3-r1`, `spec-0.2-r1`, …: PDF, single-file web page, Markdown, the chapters as a zip, and `SHA256SUMS` |
| Markdown in this repository | [`spec/rnl-0.3.md`](spec/rnl-0.3.md) (English, normative) and [`spec/rnl-0.3.zh.md`](spec/rnl-0.3.zh.md) (Chinese), assembled from the chapters in [`spec/0.3/`](spec/README.md); 0.2 is kept beside them |
| Standard word list | [`spec/rnl-words-0.3.json`](spec/rnl-words-0.3.json) and its [schema](spec/rnl-words.schema.json), part of the standard |
| Principles | [`spec/rich-netlist.md`](spec/rich-netlist.md) (English, authoritative) and [`spec/rich-netlist.zh.md`](spec/rich-netlist.zh.md) (Chinese) |

Every form is built from the chapters of one commit and names its revision ([decision 0007](docs/decisions/0007-publishing-the-standard.md)). Language versions are worked out in a research repository and published here ([decision 0006](docs/decisions/0006-language-versions-from-research-repository.md)).

## Repository map

| Path | Responsibility |
| --- | --- |
| [`spec/`](spec/README.md) | The RNL language standard and its principles. |
| [`crates/`](crates/README.md) | Rust runtime, SPICE and Spectre host adapters, shared renderer, the core interface for viewers, the writer of edits, the solver, and the `rnl` command. |
| [`bindings/`](bindings/README.md) | The core as WebAssembly, with JavaScript modules to run it in a Worker, built into the npm package `@rich-netlist/runtime`; the solver's binding, separate, is future work. |
| [`packages/`](packages/README.md) | TypeScript packages: the README and the smoke test of the npm package `@rich-netlist/runtime` (`runtime/`; the package itself is built from `bindings/runtime-wasm`); the framework independent viewer and editor cores and the solver's interface are future work. |
| [`apps/`](apps/README.md) | The desktop and web viewers, and the page they share. |
| [`libraries/`](libraries/README.md) | External data libraries: the standard packages of the series 0.3 and 0.2 with their symbols, the Chinese alias packages, and `analog-canvas-extensions`. |
| [`tests/`](tests/README.md) | Conformance corpus, cases for the solver, and browser tests of the viewers; dedicated roundtrip cases are future work. |
| [`skills/`](skills/README.md) | Skill packages for AI agents: the RNL skill, which teaches an agent to read, write, check and draw RNL 0.3, published on the site with an entry for agents ([For agents](https://rich-netlist.github.io/rnl/en/agents.html)). |
| [`scripts/`](scripts/README.md) | Maintenance scripts: the build of the standard's published forms, the build and check of the skill package, and the fonts shipped with the core. |
| [`.github/workflows/`](.github/workflows/README.md) | CI workflows: the project site, the releases of the standard, of the desktop viewer and of the npm package, the Rust workspace, the skill package, the viewers and their browser tests. |
| [`docs/`](docs/README.md) | Toolchain architecture proposal, roadmap, and decision records. |
| [`site/`](site/README.md) | The [project site](https://rich-netlist.github.io/rnl/en/), in Chinese and English, built from one bilingual source by `site/build.py`: the introduction, the language tour (with the language at a glance and examples), the gallery of the standard packages, the tools (with the toolchain) and the page for agents; [`pages.yml`](.github/workflows/pages.yml) adds the pages of the standard (with their short guide and the principles), the web viewer and the skill package. |

Start with the [documentation map](docs/README.md), the [toolchain architecture proposal](docs/rnl-toolchain-architecture.md), and the [roadmap](docs/roadmap.md). Each component directory has a README that says what it holds and what is still to do.

## Downloading and release lines

The [tools page](https://rich-netlist.github.io/rnl/en/tools.html#cli) and the latest core release
offer CLI archives for Windows x64, Linux x86_64 and macOS Apple Silicon/Intel.
`rnl --version` prints the core version and build commit; verify downloads against
`SHA256SUMS.txt`. The CLI and npm share `core-vX.Y.Z`; Viewer uses independent `viewer-vX.Y.Z`
versions and the language standard keeps `spec-*`. Only the core becomes GitHub latest.
Historical `runtime-v0.1.0-preview.N` tags were npm-only releases and no longer publish.
npm publishes each core version under the dist-tag `latest`. See [release procedures](docs/releases.md)
for rehearsal, publication and recovery if npm succeeds before GitHub publication fails.

## Trying it

```bash
cargo run --release -p rnl-cli -- check tests/conformance/cases/03-five-transistor-ota.sp
cargo run --release -p rnl-cli -- render tests/conformance/cases/03-five-transistor-ota.sp -o target/svg
```

See [`crates/rnl-cli`](crates/rnl-cli/README.md) for the other commands. The desktop viewer has installers for Windows x64, Linux x86_64 and macOS (Apple Silicon and Intel) on the [releases](https://github.com/rich-netlist/rnl/releases) page (the newest tag `viewer-v…`), and the core is on npm: `npm i @rich-netlist/runtime`. To run the desktop viewer from source, including on macOS, and look at a netlist in it:

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

## Next steps

The [roadmap](docs/roadmap.md) is the list, and says what is done and what is not. What is next: the language 0.4, planned to let a rule choose a symbol by the number of a device's terminals and to let a netlist configure the solver; the second version of the solver's objectives, with the standard package's names moved out of the solver into its configuration; a WebAssembly binding for the solver, so that the web viewer can solve too; symbol packages for the devices of foundries' PDKs; releases of the desktop viewer for macOS, and signed releases for Windows; and a record of the profile of the standard that the toolkit supports, checked against the published text, with dedicated roundtrip cases. An editor that moves devices by dragging is set aside for now, and the roadmap says why.

## License

| Content | Paths | License |
| --- | --- | --- |
| Everything else, including code, configuration, and top-level files | everything not listed below | [MIT](LICENSE) |
| Specification text and project documentation | `spec/`, `docs/` | [CC-BY-4.0](LICENSES/CC-BY-4.0.txt) |
| Data: standard libraries and test corpora | `libraries/`, `tests/` | [CC0-1.0](LICENSES/CC0-1.0.txt) |
| The metrics of the fonts shipped with the core, Noto Sans SC, copyright 2014-2021 Adobe ([decision 0009](docs/decisions/0009-fonts-shipped-with-the-core.md)) | `crates/rnl-render/fonts/noto-sans-sc.metrics` | [OFL-1.1](LICENSES/OFL-1.1.txt) |

[`REUSE.toml`](REUSE.toml) records this mapping, and `reuse lint` checks it. The documents migrated from Analog Canvas are published here under CC-BY-4.0 by their copyright holder; the frozen copies in Analog Canvas remain under that repository's AGPL-3.0 license. See [decision 0002](docs/decisions/0002-licenses.md).

## Contributing

Please discuss language or module boundary changes in an issue or pull request and keep documentation status explicit. A contribution is licensed under the license of the path it changes; no sign-off or contributor agreement is required. See [CONTRIBUTING.md](CONTRIBUTING.md).

Optional extension and PDK packages live in [rnl-packages](https://github.com/rich-netlist/rnl-packages). The web viewer reads its catalog and verifies SHA-256; the standard series stays built in. For development and tests run `python scripts/packages.py download`; `packages.lock.json` pins the versions. See [Contributing](CONTRIBUTING.md).
