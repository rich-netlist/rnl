# Roadmap

This is the central TODO list. A checked box means work present in this repository, not a claim that a product feature works.

## Bootstrap

- [x] Create the independent directory map and documentation entry points.
- [x] Migrate the principles draft, syntax draft 0.1-draft.2, and toolchain architecture proposal from Analog Canvas with provenance ([decision 0001](decisions/0001-canonical-rnl-documents.md)).
- [x] Start the decision record log.
- [x] Confirm that the migrated documents belong to the maintainer's personal project and can be relicensed here.
- [x] Decide the licenses for code, specification text, and data ([decision 0002](decisions/0002-licenses.md)).
- [x] Reserve the npm organization scope `@rich-netlist` ([decision 0003](decisions/0003-npm-scope.md)).
- [x] Move the repository into the `rich-netlist` GitHub organization ([decision 0005](decisions/0005-github-organization.md)).
- [x] Decide the contribution terms ([decision 0004](decisions/0004-contribution-terms.md), [CONTRIBUTING.md](../CONTRIBUTING.md)).
- [x] Add a Chinese introduction page for circuit designers ([`site/`](../site/README.md)). Its RNL snippets follow the ongoing 1.16 language discussion, which is not yet in `spec/`.
- [x] Add the English version of the introduction page, built with the Chinese one from a single source that pairs every text in both languages; CI fails on an unpaired text.
- [x] Point the old copies of the drafts at this repository now that it is public: the frozen copies in the research repository, and the drafts on Analog Canvas's unmerged `rich-netlist` branch (Analog Canvas `main` never carried them). See the update in [decision 0001](decisions/0001-canonical-rnl-documents.md).

## Toolchain foundation

- [x] Establish the Cargo workspace ([`crates/`](../crates/README.md)) with a CI workflow: formatting, lints, tests on Linux, Windows and macOS, and a WebAssembly build of the core crates.
- [ ] Establish the pnpm workspace when the first TypeScript package is ready.
- [ ] Define module APIs, package names, versioning, and build targets.
- [x] Assemble a conformance corpus for language draft 0.2 ([`tests/conformance/`](../tests/conformance/README.md)): 15 cases with their expected effective models and diagnostic places, and 3 circuits written in both SPICE and Spectre.
- [ ] Bring language draft 0.2 into [`spec/`](../spec/README.md), with a decision record that supersedes [decision 0001](decisions/0001-canonical-rnl-documents.md) on where language work happens.
- [ ] Add dedicated roundtrip cases; the writeback checks run over the conformance cases for now.

## Reading and rendering

- [x] Implement the Rust runtime and the SPICE and Spectre host adapters for language draft 0.2, without a solver: constraints are checked against the written coordinates and unplaced objects are reported.
- [x] Implement the shared renderer (primitives and SVG) and the draft standard package with its symbols ([`libraries/standard/`](../libraries/standard/README.md)); every symbol writes its box.
- [ ] Add standard symbols for the elements that still lack one (J, F, H, B and others).
- [x] Make reading SPICE and Spectre netlists linear in their size. Each reader keeps the names of a level's instances to find a name defined twice, the SPICE reader looks through a run of comment lines once, the Spectre reader joins `\` continuations in place, and records that leave a comment open no longer each look to the end of their run of carrier lines. Reading 32,000 resistors takes 0.05 to 0.06 s instead of 1.3 to 1.5 s, and the rich synthetic of 16,000 cells (32,000 elements, 142,000 comment lines) 0.11 s instead of 9.9 s in SPICE; doubling the netlist about doubles the time.
- [ ] Make checking with the standard package linear in the number of subcircuits. Its fill sets are shared by all levels: the `@ADD-TO` and `@REMOVE-FROM` of every level go into a set's list of changes, which each later level copies and goes through. Checking 4,000 subcircuits of one resistor takes about 9 s (2,000: about 2 s), against 0.1 s without the standard package and 0.01 s to read them.
- [x] Add the CLI: check, render, dump, and strip generated records.
- [x] Add a desktop viewer that opens a netlist, draws its levels, shows the netlist text and its diagnostics, follows a selected instance or net between the drawing and the text, and moves and zooms the view ([`apps/desktop/`](../apps/desktop/README.md)). It calls the Rust runtime directly through Tauri; it does not edit or solve.
- [x] Measure the desktop viewer's startup: on Windows 11 the first drawing appears after 0.6 to 1.0 s, most of it the WebView engine starting.

## Shared core and web viewer

One shared Rust core decides the drawing; the frontends are per platform ([decision 0006](decisions/0006-shared-core-per-platform-frontends.md)). In order:

- [x] Make style resolution linear in the number of objects. A chain of 2,000 resistors takes 0.17 s to check instead of 36 s, and doubling the chain about doubles the time (0.78 s for 8,000).
- [x] Add `crates/rnl-view`, the core interface (a workspace in, a Document out). It takes over the desktop viewer's document builder and the parts of `rnl-cli` that read no files: dialect detection, the bundled standard packages, level status. The desktop viewer and the CLI use it.
- [x] Give the Document each level's primitives in serializable form, with SVG written from them; generate type definitions for frontends. The frontend chooses the outputs; by default the Document is as before. The TypeScript declarations are in [`crates/rnl-view/types`](../crates/rnl-view/README.md#type-definitions).
- [ ] Draw line styles in the renderer rather than the SVG writer (the hairline's width, dashes, double lines on every shape, flylines), and give them in the primitives, so that every frontend draws them as the SVG does.
- [ ] Ship fonts with the core and lay out text with their metrics; choose the font set, including Chinese, and its licence.
- [x] Use a pure-Rust math library in the core, and test in CI that native and WebAssembly builds give identical Documents on the conformance corpus. The core takes its sines, cosines and arc tangents from `libm`, which changed no output; the 24 files of the corpus, the two standard packages and a Latin-1 file give identical Documents. Two things in the Document followed a hash map's order, which changes from build to build: which of several equally close names a hint offers, and the order of the overridden statements. Both are fixed, and corpus case 16 keeps them so.
- [x] Add the WebAssembly binding in [`bindings/`](../bindings/README.md), run in a Worker ([`bindings/runtime-wasm`](../bindings/runtime-wasm/README.md)): a JSON request in and a Document out, with no generated glue, and JavaScript modules that load it, run it in a Worker and collect the files a document needs. The module is 1.20 MB, 407 KB gzip-compressed.
- [x] Split the desktop viewer's page into modules behind a shell interface and a command table, so that the web viewer can reuse it.
- [x] Add the web viewer in [`apps/web/`](../apps/web/README.md): open files, a folder or a URL, collect the included files, an HTML menu bar, and the panels of the desktop viewer. It is a static site of 1.36 MB with the core, and for the same files its Documents are `rnl view`'s but for the paths that name the files.
- [ ] Test the shared page in Chromium, WebKit and Firefox against a fake shell, and smoke-test both viewers.

## Solving and editing

- [ ] Edit RNL text in the viewers and redraw on request (a refresh button and a shortcut), including a netlist pasted without a file ([decision 0006](decisions/0006-shared-core-per-platform-frontends.md)).

- [ ] Define the precise RNL output contract, source metadata, and input fingerprint in the language draft.
- [ ] Implement the solver, separate solver WASM binding/package, and native/WASM consistency checks.
- [ ] Add viewer integration that requests solving only when needed or requested.
- [ ] Implement the framework independent editor core, transactions, and safe host writeback.

## Release

- [ ] Define package compatibility and publication policy after the first working end-to-end path.
- [ ] Claim crate names with the first real crate release.
- [ ] Establish platform builds, performance baselines, documentation, and release checks.
