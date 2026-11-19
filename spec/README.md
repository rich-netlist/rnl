# spec

The RNL language standard and principles.

| Document | Status |
| --- | --- |
| [RNL 0.4](rnl-0.4.md) | Language standard, version 0.4, the current version. English, normative. Versions 0.x make no promise of compatibility with each other; Annex D lists what changed from 0.3 and how a file moves to 0.4. |
| [RNL 0.4, Chinese](rnl-0.4.zh.md) | Translation of the standard; the English text prevails. |
| [`0.4/en/`](0.4/en/) and the translations [`0.4/zh/`](0.4/zh/), [`0.4/zh-hant/`](0.4/zh-hant/), [`0.4/ja/`](0.4/ja/), [`0.4/ko/`](0.4/ko/), [`0.4/fr/`](0.4/fr/), [`0.4/es/`](0.4/es/), [`0.4/de/`](0.4/de/) | The chapters of 0.4, one file each: the source of every published form. The combined texts above are assembled from them. |
| [Short guide 0.4](rnl-0.4-guide.md) and its translations, such as the [Chinese one](rnl-0.4-guide.zh.md) | What it takes to read and write RNL files, in a few pages. Not normative, and not a part of the standard: it is a page of its own beside the page of 0.4 and, the English file copied, the guide of the [RNL skill package](../skills/README.md) for AI agents, which is in English only ([`scripts/spec`](../scripts/spec/README.md#the-short-guide)). |
| [Word list 0.4](rnl-words-0.4.json) and its [schema](rnl-words-0.4.schema.json) | Part of the standard: all standard words and keyword signatures of version 0.4, machine-readable. |
| [RNL 0.3](rnl-0.3.md) and its [Chinese translation](rnl-0.3.zh.md), the chapters [`0.3/en/`](0.3/en/) and [`0.3/zh/`](0.3/zh/), the [short guide 0.3](rnl-0.3-guide.md), the [word list 0.3](rnl-words-0.3.json) | The previous version, archived and kept unchanged. The tools of 0.4 do not read files of 0.3: core 0.5.7, the last core that reads them, converts them to 0.4 with `rnl upgrade`. |
| [RNL 0.2](rnl-0.2.md) and its [Chinese translation](rnl-0.2.zh.md), the chapters [`0.2/en/`](0.2/en/) and [`0.2/zh/`](0.2/zh/), the [word list 0.2](rnl-words-0.2.json) | An earlier version, archived and kept unchanged; core 0.5.7 converts its files to 0.4 too. The word lists of 0.2 and 0.3 share the [schema](rnl-words.schema.json). |
| [Principles and boundaries](rich-netlist.md) | Principles; the standard follows them. English, authoritative. |
| [Principles and boundaries, Chinese](rich-netlist.zh.md) | The Chinese version of the principles, from which the English text was translated. |

Language versions are worked out in the maintainer's research repository and published here as consolidated texts ([decision 0006](../docs/decisions/0006-language-versions-from-research-repository.md)).

Each revision of the standard is published by a tag such as `spec-0.4-r1`. Its GitHub release carries, for English and Chinese, the combined Markdown, the chapters as a zip, a single-file web page and a PDF; the project site shows the web pages, in [English](https://rich-netlist.github.io/rnl/en/spec/0.4/) and [Chinese](https://rich-netlist.github.io/rnl/zh/spec/0.4/), and keeps those of 0.3 and 0.2. All of them are built from the chapters of the same commit, and each names its revision ([decision 0007](../docs/decisions/0007-publishing-the-standard.md)). A version is released, for the site, when its folder holds a file `RELEASED` (`0.2/RELEASED`, `0.3/RELEASED`, `0.4/RELEASED`); the file is added in the change that publishes the version, and until then the site shows nothing of it, neither its pages nor the standard package series of its number. A released version is archived when its folder also holds a file `ARCHIVED` (`0.2/ARCHIVED`, `0.3/ARCHIVED`): its pages stay as they are, with a banner that leads to the current version ([`scripts/spec/`](../scripts/spec/README.md)). The earlier syntax draft 0.1-draft.2 is superseded by 0.2; it remains in the Git history.

See the [roadmap](../docs/roadmap.md) for planned work.
