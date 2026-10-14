# spec

The RNL language standard and principles.

| Document | Status |
| --- | --- |
| [RNL 0.2](rnl-0.2.md) | Language standard, version 0.2. English, normative. Versions 0.x make no promise of compatibility with each other. |
| [RNL 0.2, Chinese](rnl-0.2.zh.md) | Translation of the standard; the English text prevails. |
| [`0.2/en/`](0.2/en/), [`0.2/zh/`](0.2/zh/) | The chapters of 0.2, one file each: the source of every published form. The combined texts above are assembled from them. |
| [Word list 0.2](rnl-words-0.2.json) and its [schema](rnl-words.schema.json) | Part of the standard: all standard words and keyword signatures of version 0.2, machine-readable. |
| [Principles and boundaries](rich-netlist.md) | Principles; the standard follows them. English, authoritative. |
| [Principles and boundaries, Chinese](rich-netlist.zh.md) | The Chinese version of the principles, from which the English text was translated. |

Language versions are worked out in the maintainer's research repository and published here as consolidated texts ([decision 0006](../docs/decisions/0006-language-versions-from-research-repository.md)).

Each revision of the standard is published by a tag such as `spec-0.2-r1`. Its GitHub release carries, for English and Chinese, the combined Markdown, the chapters as a zip, a single-file web page and a PDF; the project site shows the web pages. All of them are built from the chapters of the same commit, and each names its revision ([decision 0007](../docs/decisions/0007-publishing-the-standard.md)). The earlier syntax draft 0.1-draft.2 is superseded by 0.2; it remains in the Git history.

See the [roadmap](../docs/roadmap.md) for planned work.
