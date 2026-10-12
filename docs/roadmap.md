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
- [x] Add the CLI: check, render, dump, and strip generated records.
- [ ] Add the runtime WASM binding, TypeScript runtime package, and framework independent viewer core.
- [ ] Add the React/Vite web application and Tauri desktop shell; measure startup and document loading.

## Solving and editing

- [ ] Define the precise RNL output contract, source metadata, and input fingerprint in the language draft.
- [ ] Implement the solver, separate solver WASM binding/package, and native/WASM consistency checks.
- [ ] Add viewer integration that requests solving only when needed or requested.
- [ ] Implement the framework independent editor core, transactions, and safe host writeback.

## Release

- [ ] Define package compatibility and publication policy after the first working end-to-end path.
- [ ] Claim crate names with the first real crate release.
- [ ] Establish platform builds, performance baselines, documentation, and release checks.
