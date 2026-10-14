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
- [x] Add a Chinese introduction page for circuit designers ([`site/`](../site/README.md)). Its RNL snippets follow language draft 0.2, from which standard 0.2 was consolidated.
- [x] Add the English version of the introduction page, built with the Chinese one from a single source that pairs every text in both languages; CI fails on an unpaired text.
- [x] Point the old copies of the drafts at this repository now that it is public: the frozen copies in the research repository, and the drafts on Analog Canvas's unmerged `rich-netlist` branch (Analog Canvas `main` never carried them). See the update in [decision 0001](decisions/0001-canonical-rnl-documents.md).

## Language standard

- [x] Publish the language standard 0.2: English, normative, with a Chinese translation, the standard word list and its schema, consolidated in the research repository ([decision 0006](decisions/0006-language-versions-from-research-repository.md)). The principles are maintained in English, with a Chinese version.
- [x] Build the standard's web pages, PDFs and Markdown downloads from its chapters, with checks that keep the forms and the two languages in step ([decision 0007](decisions/0007-publishing-the-standard.md)).
- [ ] Publish revision 1 of standard 0.2 with the tag `spec-0.2-r1`, once the release workflow has run on GitHub.
- [ ] Publish the standard package and the Chinese alias package under `libraries/` when their content is settled.

## Toolchain foundation

- [ ] Establish real Cargo and pnpm workspaces when the first implementable packages are ready.
- [ ] Define module APIs, package names, versioning, build targets, and meaningful CI gates.
- [ ] Define the supported language profile of standard 0.2 and assemble conformance and roundtrip corpora.

## Reading and rendering

- [ ] Implement the Rust runtime and SPICE/Spectre host adapters for an agreed structural subset.
- [ ] Implement the shared renderer and external standard symbol data.
- [ ] Add the CLI, runtime WASM binding, TypeScript runtime package, and framework independent viewer core.
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
