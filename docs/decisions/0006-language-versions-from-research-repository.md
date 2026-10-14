# 0006. Language versions are worked out in a research repository and published here

- Date: 2026-10-13
- Status: accepted
- Supersedes: [0001](0001-canonical-rnl-documents.md)

## Context

[0001](0001-canonical-rnl-documents.md) made this repository the only place where the RNL documents are maintained, and `spec/README.md` called `spec/` the only maintained location for language work.

Since early October 2026 the language has been worked out in the maintainer's private research repository: rounds of discussion, a prototype interpreter, and writing and reading trials with language models. That work produces many intermediate drafts, which would be noise here and would be mistaken for the standard.

## Decision

- Language drafts iterate in the private research repository. This repository receives consolidated versions of the language standard only, each a complete text with its version number.
- The first is RNL 0.2: `spec/rnl-0.2.md` (English, normative), `spec/rnl-0.2.zh.md` (Chinese translation), the standard word list `spec/rnl-words-0.2.json` and its schema `spec/rnl-words.schema.json`. Each consolidated version names the research-repository commit it was consolidated from.
- `spec/RNL-Syntax-0.1-draft.2.md` is superseded by 0.2 and removed; it remains in the Git history.
- The principles are still maintained here, as are the toolchain architecture and the other project documents. From now on they are maintained in English: `spec/rich-netlist.md` becomes the English text, which prevails, and the Chinese text moves to `spec/rich-netlist.zh.md`.
- The two repositories are never merged.

The other parts of 0001 stand: the migrated documents keep their provenance notes, and the toolchain architecture keeps its file name.

## Consequences

- Changes to the language between versions are not visible here until the next version is published.
- Issues and pull requests about the language are still welcome here; they are taken into the research repository and come back with the next version.
- The standard package and alias packages are written in RNL and are not part of the language standard (`spec/rnl-0.2.md`, 16.3); they will be published under `libraries/` when their content is settled.
