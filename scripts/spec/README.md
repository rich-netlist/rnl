# scripts/spec

Builds every published form of the RNL standard from the chapters in [`spec/`](../../spec/README.md), and checks that they agree ([decision 0007](../../docs/decisions/0007-publishing-the-standard.md)).

```bash
python scripts/spec/build.py                 # check, then build every version into scripts/spec/dist/
python scripts/spec/build.py --check         # the checks only
python scripts/spec/build.py --version 0.2   # only this version, as the release of a tag spec-0.2-rN does
python scripts/spec/build.py --site site/dist   # also add the web pages and their downloads to the built site
python scripts/spec/build.py --no-pdf        # skip the PDFs
python scripts/spec/build.py --stamp         # after bringing the Chinese in line with the English
```

Every version in `spec/` is built: the folders named `major.minor`, such as `0.2/`, that hold `en/` and `zh/`. Decision 0007 keeps the earlier versions published when a new one is added.

Needs Python 3.10 or later (standard library only), [pandoc](https://pandoc.org/) 3.x and [Typst](https://typst.app/) 0.13 or later on PATH, and git. The CI workflows install pinned versions.

## Input

| Path in `spec/` | Content |
| --- | --- |
| `0.2/en/`, `0.2/zh/` | `front.md`, the chapters `ch01.md`, `ch02.md`, … and the annexes `annex-a.md`, `annex-b.md`, …: the source, the only text that is edited. The parts of a version are the files there, chapters by number, then annexes by letter; both languages have the same parts |
| `rnl-0.2.md`, `rnl-0.2.zh.md` | the combined texts, assembled from the chapters and committed |
| `rnl-words-0.2.json` | the word list of the version |
| `rnl-0.3-guide.md`, `rnl-0.3-guide.zh.md` | the short guide to the language of a version that has one, as the web page shows it ([The short guide](#the-short-guide)) |
| `rnl-words.schema.json` | the schema of the word lists, shared by all versions; a new version may add optional properties to it, so that it still describes the lists of the earlier ones |

The chapters come from the research repository where the language is worked out ([decision 0006](../../docs/decisions/0006-language-versions-from-research-repository.md)).

## Output

For each version, in English and Chinese (`rnl-0.2…` and `rnl-0.2.zh…`), all from the same chapters and with the same revision label:

| File | Form |
| --- | --- |
| `rnl-0.2.md` | the combined Markdown |
| `rnl-0.2-chapters.zip` | the chapters as separate files, with a README that lists them, the word list and its schema, and a MANIFEST of SHA-256 hashes; reproducible, with the commit time as timestamps |
| `rnl-0.2.html` | one self-contained web page: contents beside the text, section anchors such as `#9.4`, links for section references, RNL code coloured at build time, light and dark, a language switch that keeps the section |
| `rnl-0.2.pdf` | the PDF: title page, contents with page numbers, the same anchors, links and colours |
| `SHA256SUMS` | hashes of all of the above |

With `--site DIR`, the Chinese page of each version goes to `DIR/zh/spec/0.2/index.html` and the English one to `DIR/en/spec/0.2/index.html`, following the site's layout, each with its downloads and `SHA256SUMS`. `DIR/zh/spec/index.html` and `DIR/en/spec/index.html` lead to the newest version and keep the section of the address, so `en/spec/#9.4` opens 9.4 of the newest version; the top bar of the site links there. The Chinese pages of 0.2 were at `DIR/spec/0.2/` until the Chinese pages of the site moved to `zh/` (`MOVED_ZH`): there, each page now leads to the same page under `zh/` with the section of the address, so `spec/0.2/#1.3` opens `zh/spec/0.2/#1.3`, the downloads are copies, since no page can stand in for a PDF, and `DIR/spec/index.html` leads to the newest Chinese page. When there are several versions, each page lists them under its revision label, with links.

Next to the page of each version that has earlier ones, `--site` also writes a page that compares it with each of them, such as `DIR/en/spec/0.3/compare-0.2.html`, linked from the line of versions. It is the text of 0.3 in which what only 0.2 has is struck through on red and what only 0.3 has is on green, built by [`specdiff.py`](specdiff.py) from both versions as pandoc reads them:

- sections are matched by their numbers; a section that only one version has is marked as a whole;
- within a section, paragraphs, list items and table rows are matched in order, the most similar first; words are compared in English and characters in Chinese, and inline code counts as one word; text of which less than half is kept is shown removed and added as a whole;
- code is compared line by line, and each line is coloured as it reads in its own version.

The page has the contents, anchors and colours of the version's own page. Its head links the sections that changed, were added and were removed; the contents mark them; a bar at the bottom moves from one change to the next and can hide the sections that did not change. Before the page is written, the build takes both versions back out of the comparison and stops when either does not come back whole.

Each version has its own revision label, from git: "revision 2" at the tag `spec-0.2-r2`, "revision 2 + later changes" once the chapters or the word list of 0.2 have changed since that tag, "not released" before its first tag; with the date and short hash of the tagged commit, or of the last commit that changed those files, and a note when they have uncommitted changes. Commits that change other files, such as the toolkit or another version, leave the label as it is.

## The short guide

A version may have a short guide to its language, `rnl-0.3-guide.md` (English) and `rnl-0.3-guide.zh.md` (Chinese) next to its combined texts: what it takes to read and write RNL files, in about four thousand words, with examples that read with `rnl check`. It is a section of the web page of that version, the first one, after the front matter and before chapter 1, with a note that it is not normative and where the standard differs the standard prevails, and a link to the page for agents. It is in no other form: not in the PDF, the combined text or the chapters zip, and since it is not in the chapters, it leaves the revision label as it is. The same files are copied into the skill package ([`skills/`](../../skills/README.md)).

The files start with one title, `# …`, which becomes the heading of the section, with the id `guide`; their other headings are not numbered, and go one level down on the page. Code blocks are marked `spice` or `spectre` for a whole netlist, which `scripts/skill/build.py --check` reads with `rnl check`, `rnl` for lines out of context, and `error` after the language for an error example. The build checks that the two languages have the same headings, list items, table rows, inline code and code blocks, and that every section the guide cites exists.

## How the forms are kept in step

- **One source.** Only the chapters are edited. Everything else is built from them in one run; of the built files, only the combined texts are committed, and the build checks them.
- **One parse.** pandoc reads the chapters once per language. One filter, `rnl.lua`, gives the web page and the PDF their anchors, reference links and code colours; only the layout differs: `page.html` for the web, `print.typ` for the PDF (pandoc writes Typst, Typst makes the PDF).
- **Checks before building**, any of which stops the build:
  1. the Chinese chapters have the sections, list items, table rows and code of the English ones;
  2. each Chinese part starts with `<!-- translated from en/ch09.md sha256:… -->`, the hash of the English part it was translated from; when the English changes, the build stops until the Chinese is brought in line and `--stamp` is run;
  3. Annex A in both languages, and the JSON excerpt in 5.3, are what the word list gives;
  4. every section number cited in the text exists;
  5. the combined texts are what the chapters give;
  6. the short guide, when there is one: both languages, one title, the same structure and code in both, and every section it cites exists.
- **No warnings.** pandoc runs with `--fail-if-warnings`, and any output of Typst fails the build. Fonts are passed only when Typst finds them, so the same build runs on Windows, macOS and CI.

## Writing for the build

- Cite sections as the standard does: `(9.4)`, `(5.3, Annex A)`, `chapter 7`, `9.4 item 3`, `（9.4）`, `第 7 章`, `附录 A`. Numbers after `Principles` or `原则`, including the rest of a list such as `Principles 2.4, 2.9`, refer to the principles and stay text.
- Write version numbers as code (`"1.2"`); a bare `1.2` would be linked as section 1.2.
- Code blocks are marked `rnl`, `spice` or `rnl-file` to be coloured; `error` after the language marks an error example.

## Files

| File | Content |
| --- | --- |
| `build.py` | the checks and the build |
| `spectools.py` | assembling the combined texts, comparing the languages, Annex A from the word list |
| `specdiff.py` | comparing two versions, for the pages that show what changed |
| `rnl.lua` | the pandoc filter: anchors, reference links, RNL code colours, the Typst table of contents |
| `page.html` | the web page template |
| `print.typ` | the PDF template |

`dist/` is ignored by git; `dist/work/` keeps the pandoc input, the Typst source, the comparisons as pandoc reads them (`0.3-0.2-en.json` and so on), and `links-0.2-en.tsv`, `links-0.2-zh.tsv` and so on: every reference that was linked, for review.
