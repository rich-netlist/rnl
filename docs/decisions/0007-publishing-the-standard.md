# 0007. The standard is published in several forms built from one Markdown source

- Date: 2026-10-14
- Status: accepted

## Context

The standard is read in several ways: on GitHub, downloaded to read offline or print, and on the project site. Copies kept by hand drift apart, and a reader holding two of them cannot tell whether they say the same thing. [0006](0006-language-versions-from-research-repository.md) publishes each consolidated version of the language here; this record says in which forms, and how they are kept in step.

## Decision

- **One source.** The chapters of each language are the only text that is edited: `spec/<version>/en/` and `spec/<version>/zh/`, each holding `front.md`, `ch01.md` to `ch17.md` and `annex-a.md` to `annex-c.md`. The English text is normative (0006). The combined texts `spec/rnl-<version>.md` and `spec/rnl-<version>.zh.md` are assembled from the chapters and committed, so that they have a stable link; CI checks that they match.
- **Forms.** From the same commit, for each language:
  - the combined Markdown;
  - the chapters as a zip, with a README that lists them, the word list and its schema, and a manifest of SHA-256 hashes;
  - one web page, on the project site under `spec/<version>/` (Chinese) and `en/spec/<version>/` (English), following the site's convention, with the downloads of the same build beside it;
  - a PDF.

  Apart from the combined Markdown, none of these is committed.
- **One build.** pandoc reads the chapters once per language. One filter gives the web page and the PDF their section anchors, the links of section references and the colours of RNL code; Typst turns pandoc's output into the PDF. Section numbers are the anchors in every form and in both languages, such as `#9.4`. CI uses pinned versions of pandoc and Typst. The build lives in `scripts/spec/` (MIT), not in `spec/` (CC-BY-4.0).
- **Checks.** Before anything is built, CI checks that:
  1. the Chinese chapters have the sections, list items, table rows and code of the English ones;
  2. each Chinese part is stamped with the hash of the English part it was translated from, and that hash is current;
  3. Annex A in both languages is what the word list gives;
  4. every section number cited in the text exists;
  5. the combined Markdown is what the chapters give.

  pandoc and Typst must finish without warnings. Any failure stops the build and the publication.
- **Revisions.** The version number (0.2) changes only when the language changes; a new version gets its own files and pages, and the earlier versions stay published. A change that leaves the language as it is (wording, references, examples, errata) makes a new revision of the same version. A revision is published by the tag `spec-<version>-r<n>`, numbered from 1, such as `spec-0.2-r1`. CI then attaches all the forms and their `SHA256SUMS` to a GitHub release named "RNL 0.2, revision 1", whose notes list the changes.
- **Revision label.** Every form carries a label taken from Git: "revision n" when built at a tag, "revision n + later changes" after it, and "not released" before the first tag, each with the commit date and the short hash. Pushing to `main` updates the web pages with that label.

## Consequences

- Two copies agree when their labels agree; the release's `SHA256SUMS` and the zip's manifest check the downloaded files.
- The text is edited in the research repository (0006). This repository receives the chapters, and CI regenerates and checks the rest.
- When the English changes, the Chinese must be brought in line and re-stamped before anything can be published.
- Version numbers in the text are written as code (`"1.2"`), so that they are not taken for section numbers and linked.
- Building the site now needs pandoc and Typst, not only Python; CI installs them, and local builds need them installed.
- The first publication of 0.2 is revision 1, tag `spec-0.2-r1`.

## Note, 2026-10-18

The Chinese pages of the site moved from its root to `zh/`, beside `en/`, and the root leads to the pages in the browser's language, Chinese or else English ([`site`](../../site/README.md#addresses)). Following the site, the Chinese web page of a version is now under `zh/spec/<version>/`. The old addresses of 0.2, `spec/0.2/` and `spec/`, lead there with the section of the address, and its downloads are copied there.
