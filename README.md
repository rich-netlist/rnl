# Rich Netlist (RNL)

RNL is written in the comments of a circuit netlist. It describes how the circuit is drawn as a schematic and what its parts mean, while the netlist alone defines the circuit. This repository holds the language standard and its toolkit: a portable runtime, a solver, the `rnl` command, a desktop viewer and a web viewer, the standard symbol library, and a skill package for AI agents. An editor that moves devices by dragging is not built yet.

**Website: [rich-netlist.github.io/rnl](https://rich-netlist.github.io/rnl/en/)**, with an [introduction for circuit designers](https://rich-netlist.github.io/rnl/en/), a [tour of the language](https://rich-netlist.github.io/rnl/en/language.html), the [language standard 0.4](https://rich-netlist.github.io/rnl/en/spec/0.4/) with its [short guide](https://rich-netlist.github.io/rnl/en/spec/0.4/guide/), the [standard packages](https://rich-netlist.github.io/rnl/en/packages.html) with every symbol drawn, and the [viewers](https://rich-netlist.github.io/rnl/en/tools.html): one [in the browser](https://rich-netlist.github.io/rnl/viewer/) and one to download for Windows, Linux and macOS. The pages are also translated into several languages.

**Status:** This repository holds the RNL language standard, version 0.4, in its revision 1 (`spec-0.4-r1`), which replaces 0.3, with its principles and the project documents, and the RNL toolkit, released on two lines ([release lines](docs/releases.md)): the core, that is the `rnl` command and the npm package, at 0.6.1, and RNL Viewer at 0.6.1. Both read the language 0.4 alone; core 0.5.7, the last core that reads files of 0.2 and 0.3, converts them to 0.4 with `rnl upgrade`. The toolkit reads SPICE and Spectre netlists with RNL records, reports diagnostics, builds the effective model, draws what has coordinates, and writes records back. It ships the `rnl` command; the standard packages of the series 0.4 with their symbols; a skill package that teaches an AI agent to read, write, check and draw RNL ([`skills/rnl`](skills/README.md)); and a solver ([`crates/rnl-solver`](crates/rnl-solver/README.md)), whose `rnl solve` places and routes what has no coordinates and writes its records into a copy of the files or in place. The solver is built as WebAssembly too, in a module of its own ([`bindings/solver-wasm`](bindings/solver-wasm/README.md)) that a page loads only when it solves; the npm package exports it as `@rich-netlist/runtime/solver`. One shared core decides the drawing ([decision 0008](docs/decisions/0008-shared-core-per-platform-frontends.md)): it lays out text with the fonts it ships ([decision 0009](docs/decisions/0009-fonts-shipped-with-the-core.md)) and gives the same Documents natively and as WebAssembly ([`bindings/runtime-wasm`](bindings/runtime-wasm/README.md)); the WebAssembly build is published to npm as `@rich-netlist/runtime` ([`packages/runtime`](packages/runtime/README.md)). Two viewers show the Documents with one page ([`apps/viewer-ui`](apps/viewer-ui/README.md)): a desktop viewer for Windows, Linux and macOS ([`apps/desktop`](apps/desktop/README.md)) and a web viewer ([`apps/web`](apps/web/README.md)); they open a netlist, draw its levels, show its text, diagnostics and hierarchy, follow an instance or a net between the drawing and the text, list the objects that are not drawn because they have no position, and edit the text and apply it, saving into the files on the desktop and by a download on the web ([decision 0011](docs/decisions/0011-viewers-edit-the-text-and-write-only-what-the-user-saves.md)). Both solve a netlist as a preview, the desktop viewer with the native solver and the web viewer in the browser; the desktop viewer also saves a solved copy ([decision 0010](docs/decisions/0010-desktop-viewer-solves-as-a-preview.md)). For older enterprise Linux, where the desktop viewer's libraries are not available, a static program serves the web viewer on the loopback address for a browser to show ([`apps/local`](apps/local/README.md)); it is released with the desktop viewer. The web viewer on the site is built from `main`, so it can show work that is not in a release yet. There is no editor yet that moves devices by dragging. Versions 0.x of the language make no promise of compatibility with each other.

## At a glance

```spice
* Five-transistor OTA, input stage
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
.model nch nmos level=1
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   %dp1 = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N;
*   {#M1 #M2} @ALIGN("y"), i.e., same y;
*   #M5 is at @PLACE(20, 6) => it @ORIENT("R0");
*   %g0 = $GROUND is @ATTACH to &vss;
*   net &vss, through {#M5:3 %g0::pin}, has a @WIRE
* }
.ends ota5t
.end
```

The netlist is unchanged. The header record declares the language version and uses the standard package. The record in the subcircuit says that M1, M2 and M5 are an N-type differential pair (M1 in the role `left`, M2 in `right`, M5 the `tail`) and that M1 and M2 share one y, where M5 goes and how it is turned, and that the net `vss` gets a ground symbol wired to M5. Words between the tokens, such as `and`, `treated` and `is at`, are free text: the standard sentences add them so that statements read like sentences, and they do not change the meaning. Section [1.3 of the standard](https://rich-netlist.github.io/rnl/en/spec/0.4/#1.3) explains every token.

## The language standard

| What | Where |
| --- | --- |
| Web page | [English](https://rich-netlist.github.io/rnl/en/spec/0.4/) (normative), with its PDF and Markdown downloads, and translations into the languages of the site, such as [Chinese](https://rich-netlist.github.io/rnl/zh/spec/0.4/); the English text prevails. The archived versions stay: 0.3 at [English](https://rich-netlist.github.io/rnl/en/spec/0.3/) and [Chinese](https://rich-netlist.github.io/rnl/zh/spec/0.3/), 0.2 at [English](https://rich-netlist.github.io/rnl/en/spec/0.2/) and [Chinese](https://rich-netlist.github.io/rnl/zh/spec/0.2/) |
| Every published form of a revision | The [GitHub releases](https://github.com/rich-netlist/rnl/releases) tagged `spec-0.4-r1`, `spec-0.3-r2`, `spec-0.3-r1`, `spec-0.2-r1`, …: PDF, single-file web page, Markdown, the chapters as a zip, and `SHA256SUMS`, in English and Chinese |
| Markdown in this repository | [`spec/rnl-0.4.md`](spec/rnl-0.4.md) (English, normative) and its translations, such as [`spec/rnl-0.4.zh.md`](spec/rnl-0.4.zh.md) (Chinese), assembled from the chapters in [`spec/0.4/`](spec/README.md); the [short guide](spec/rnl-0.4-guide.md), not normative; 0.3 and 0.2 are kept beside them |
| Standard word list | [`spec/rnl-words-0.4.json`](spec/rnl-words-0.4.json) and its [schema](spec/rnl-words-0.4.schema.json), part of the standard |
| Principles | [`spec/rich-netlist.md`](spec/rich-netlist.md) (English, authoritative) and [`spec/rich-netlist.zh.md`](spec/rich-netlist.zh.md) (Chinese) |

Every form is built from the chapters of one commit and names its revision ([decision 0007](docs/decisions/0007-publishing-the-standard.md)). Language versions are worked out in a research repository and published here ([decision 0006](docs/decisions/0006-language-versions-from-research-repository.md)).

## Repository map

| Path | Responsibility |
| --- | --- |
| [`spec/`](spec/README.md) | The RNL language standard and its principles. |
| [`crates/`](crates/README.md) | Rust runtime, SPICE and Spectre host adapters, shared renderer, the core interface for viewers, the writer of edits, the solver, and the `rnl` command. |
| [`bindings/`](bindings/README.md) | The core as WebAssembly, with JavaScript modules to run it in a Worker, and the solver as a WebAssembly module of its own; both are built into the npm package `@rich-netlist/runtime`. |
| [`packages/`](packages/README.md) | TypeScript packages: the README and the smoke test of the npm package `@rich-netlist/runtime` (`runtime/`; the package itself is built from `bindings/`); the framework independent viewer and editor cores are future work. |
| [`apps/`](apps/README.md) | The desktop and web viewers, the page they share, and the local program that serves the web viewer for older Linux. |
| [`libraries/`](libraries/README.md) | External data libraries: the standard packages of the series 0.4 with their symbols and the Chinese alias package. |
| [`tests/`](tests/README.md) | Conformance corpus, roundtrip cases of the solver's records, cases for the solver, and browser tests of the viewers. |
| [`skills/`](skills/README.md) | Skill packages for AI agents: the RNL skill, which teaches an agent to read, write, check and draw RNL 0.4, published on the site with an entry for agents ([For agents](https://rich-netlist.github.io/rnl/en/agents.html)); a companion skill that exports Virtuoso schematics to RNL is kept in its exporter's repository. |
| [`scripts/`](scripts/README.md) | Maintenance scripts: the build of the standard's published forms, the build and check of the skill package, and the fonts shipped with the core. |
| [`.github/workflows/`](.github/workflows/README.md) | CI workflows: the project site, the releases of the standard, of the desktop viewer and of the npm package, the Rust workspace, the skill package, the viewers and their browser tests. |
| [`docs/`](docs/README.md) | Toolchain architecture proposal, roadmap, and decision records. |
| [`site/`](site/README.md) | The [project site](https://rich-netlist.github.io/rnl/en/), in English with translations into several languages, built by `site/build.py` from the English in `site/src/` and the catalogs in `i18n/`: the introduction, the language tour (with the language at a glance and examples), the gallery of the standard packages, the tools (with the toolchain) and the page for agents; [`pages.yml`](.github/workflows/pages.yml) adds the pages of the standard (with their short guide and the principles), the web viewer and the skill package. |

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
cargo run --release -p rnl-cli -- check tests/conformance/cases-0.4/from-0.2/03-five-transistor-ota.sp
cargo run --release -p rnl-cli -- render tests/conformance/cases-0.4/from-0.2/03-five-transistor-ota.sp -o target/svg
```

See [`crates/rnl-cli`](crates/rnl-cli/README.md) for the other commands. The desktop viewer has installers for Windows x64, Linux x86_64 and macOS (Apple Silicon and Intel) on the [releases](https://github.com/rich-netlist/rnl/releases) page (the newest tag `viewer-v…`), and the core is on npm: `npm i @rich-netlist/runtime`. To run the desktop viewer from source, including on macOS, and look at a netlist in it:

```bash
cargo run --manifest-path apps/desktop/Cargo.toml -- tests/conformance/cases-0.4/from-0.2/03-five-transistor-ota.sp
```

Or in the web viewer, which is on the site with the same samples: [rich-netlist.github.io/rnl/viewer/?url=samples/cases/03-five-transistor-ota.sp](https://rich-netlist.github.io/rnl/viewer/?url=samples/cases/03-five-transistor-ota.sp). To run it here, build its site with the samples and serve it, then open `http://127.0.0.1:8000/?url=samples/cases/03-five-transistor-ota.sp`. Both viewers draw texts in the shipped fonts once `python scripts/fonts.py web` has written them ([`scripts`](scripts/README.md)), and fall back on a system font without them.

```bash
node apps/web/build.mjs --samples
```

```bash
node apps/web/serve.mjs
```

## Next steps

The [roadmap](docs/roadmap.md) is the list, and says what is done and what is not. Among what is open: the direction facts of pins, their selections and the editing of pins in the language 0.4 ([decision 0019](docs/decisions/0019-pin-identity-in-language-0-4.md)), and pins named in a drawing resolved once the symbols are chosen; symbol packages for the devices of foundries' PDKs, beyond the one for SKY130 in [rnl-packages](https://github.com/rich-netlist/rnl-packages); a viewer that asks for a solve only when one is needed; checks that the solver's WebAssembly build solves as the native one does; a precise contract for what the solver writes; and signed releases for Windows. An editor that moves devices by dragging is set aside for now, and the roadmap says why.

## License

| Content | Paths | License |
| --- | --- | --- |
| Everything else, including code, configuration, and top-level files | everything not listed below | [MIT](LICENSE) |
| Specification text and project documentation | `spec/`, `docs/` | [CC-BY-4.0](LICENSES/CC-BY-4.0.txt) |
| Data: standard libraries and test corpora | `libraries/`, `tests/` | [CC0-1.0](LICENSES/CC0-1.0.txt) |
| The metrics of the fonts shipped with the core, Noto Sans SC, copyright 2014-2021 Adobe ([decision 0009](docs/decisions/0009-fonts-shipped-with-the-core.md)) | `crates/rnl-render/fonts/noto-sans-sc.metrics` | [OFL-1.1](LICENSES/OFL-1.1.txt) |
| The faces the command line draws PNG images with, subsets of Noto Sans SC, copyright 2014-2021 Adobe ([decision 0009](docs/decisions/0009-fonts-shipped-with-the-core.md)) | `crates/rnl-cli/fonts/` | [OFL-1.1](LICENSES/OFL-1.1.txt) |

[`REUSE.toml`](REUSE.toml) records this mapping, and `reuse lint` checks it. The documents migrated from Analog Canvas are published here under CC-BY-4.0 by their copyright holder; the frozen copies in Analog Canvas remain under that repository's AGPL-3.0 license. See [decision 0002](docs/decisions/0002-licenses.md).

## Contributing

Please discuss language or module boundary changes in an issue or pull request and keep documentation status explicit. A contribution is licensed under the license of the path it changes; no sign-off or contributor agreement is required. See [CONTRIBUTING.md](CONTRIBUTING.md).

Optional extension and PDK packages live in [rnl-packages](https://github.com/rich-netlist/rnl-packages). The web viewer reads its catalog and verifies SHA-256; the standard series stays built in. For development and tests run `python scripts/packages.py download`; `packages.lock.json` pins the versions. See [Contributing](CONTRIBUTING.md).
