# 0001. This repository is the canonical home of the RNL documents

- Date: 2026-09-24
- Status: superseded by [0006](0006-language-versions-from-research-repository.md)

## Context

The RNL principles draft, syntax draft 0.1-draft.2, and toolchain architecture proposal were written in the Analog Canvas repository under `docs/standards/`. The proposal asks for the drafts to move into the independent repository so that only one copy is maintained.

## Decision

- The three documents live here as `spec/rich-netlist.md`, `spec/RNL-Syntax-0.1-draft.2.md`, and `docs/rnl-toolchain-architecture.md`. File names are unchanged so existing references stay recognizable.
- They were copied without Git history. Each file names its source commit at the top. Migration edits are limited to that provenance note and to wording and links that depend on where the file lives; content changes are made here afterwards as ordinary changes.
- The English architecture baseline written before the migration is removed. It restated the proposal and would have drifted from it.

## Consequences

- Language and architecture changes are made only in this repository.
- Analog Canvas keeps its copies, each marked at the top as migrated and frozen. They are to be replaced with pointers once this repository is public; until then, a link from that public repository to this private one would not open for its readers.
- Licensing is settled separately in [0002](0002-licenses.md).

## Update, 2026-09-29

The consequences above describe the copies imprecisely. The copies marked as migrated and frozen are in the maintainer's private research repository, which was split out of the Analog Canvas branch that held the drafts. In Analog Canvas itself, the drafts exist only on the unmerged `rich-netlist` branch; `main` never carried them.

Now that this repository is public, both point here: the research repository's copies name `rich-netlist/rnl` at the top, and Analog Canvas commit `24a915d7` on the `rich-netlist` branch adds the same note to its two drafts. The decision itself is unchanged.
