# Decision records

Each settled project decision gets one numbered file: its context, the decision, and its consequences. A record is never rewritten to reverse a decision; a later record supersedes it and both link to each other.

Status values are `proposed`, `accepted`, and `superseded by NNNN`.

Decisions made before this log started, including those from the first architecture review, are recorded in the revision notes of the [toolchain architecture](../rnl-toolchain-architecture.md).

| No. | Decision | Status |
| --- | --- | --- |
| [0001](0001-canonical-rnl-documents.md) | This repository is the canonical home of the RNL documents | superseded by [0006](0006-language-versions-from-research-repository.md) |
| [0002](0002-licenses.md) | MIT for code, CC-BY-4.0 for specification and documentation, CC0-1.0 for data | accepted |
| [0003](0003-npm-scope.md) | npm packages use the `@rich-netlist` scope | accepted |
| [0004](0004-contribution-terms.md) | Contributions are licensed under the path's license, without sign-off or CLA | accepted |
| [0005](0005-github-organization.md) | The repository lives in the `rich-netlist` GitHub organization | accepted |
| [0006](0006-language-versions-from-research-repository.md) | Language versions are worked out in a research repository and published here | accepted |
| [0007](0007-publishing-the-standard.md) | The standard is published in several forms built from one Markdown source | accepted |
| [0008](0008-shared-core-per-platform-frontends.md) | One shared Rust core decides the drawing; frontends are per platform | accepted |
| [0009](0009-fonts-shipped-with-the-core.md) | The core ships Noto Sans SC and lays out text with its metrics | accepted |
| [0010](0010-desktop-viewer-solves-as-a-preview.md) | The desktop viewer solves as a preview and saves a solved copy; it never changes the files it opens | accepted; its second point is changed by [0011](0011-viewers-edit-the-text-and-write-only-what-the-user-saves.md) |
| [0011](0011-viewers-edit-the-text-and-write-only-what-the-user-saves.md) | The viewers edit the text, and write a file only when the user has edited it and saves it | accepted |
| [0012](0012-public-packages-use-an-immutable-catalog.md) | Optional public packages use a separate repository and an immutable catalog | accepted; implementation under review |
