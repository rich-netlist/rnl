# scripts/spec

Builds every published form of the RNL standard from the chapters in [`spec/`](../../spec/README.md), and checks that they agree ([decision 0007](../../docs/decisions/0007-publishing-the-standard.md)).

```bash
python scripts/spec/build.py                 # check, then build into scripts/spec/dist/
python scripts/spec/build.py --check         # the checks only
python scripts/spec/build.py --site site/dist   # also add the web pages and their downloads to the built site
python scripts/spec/build.py --no-pdf        # skip the PDFs
python scripts/spec/build.py --stamp         # after bringing the Chinese in line with the English
```

Needs Python 3.10 or later (standard library only), [pandoc](https://pandoc.org/) 3.x and [Typst](https://typst.app/) 0.13 or later on PATH, and git. The CI workflows install pinned versions.

## Input

| Path in `spec/` | Content |
| --- | --- |
| `0.2/en/`, `0.2/zh/` | `front.md`, `ch01.md` to `ch17.md`, `annex-a.md` to `annex-c.md`: the source, the only text that is edited |
| `rnl-0.2.md`, `rnl-0.2.zh.md` | the combined texts, assembled from the chapters and committed |
| `rnl-words-0.2.json`, `rnl-words.schema.json` | the word list and its schema |

The chapters come from the research repository where the language is worked out ([decision 0006](../../docs/decisions/0006-language-versions-from-research-repository.md)).

## Output

For English and Chinese (`rnl-0.2…` and `rnl-0.2.zh…`), all from the same chapters and with the same revision label:

| File | Form |
| --- | --- |
| `rnl-0.2.md` | the combined Markdown |
| `rnl-0.2-chapters.zip` | the chapters as separate files, with a README that lists them, the word list and its schema, and a MANIFEST of SHA-256 hashes; reproducible, with the commit time as timestamps |
| `rnl-0.2.html` | one self-contained web page: contents beside the text, section anchors such as `#9.4`, links for section references, RNL code coloured at build time, light and dark, a language switch that keeps the section |
| `rnl-0.2.pdf` | the PDF: title page, contents with page numbers, the same anchors, links and colours |
| `SHA256SUMS` | hashes of all of the above |

With `--site DIR`, the Chinese page goes to `DIR/spec/0.2/index.html` and the English one to `DIR/en/spec/0.2/index.html`, following the site's layout, each with its downloads and `SHA256SUMS`.

The revision label comes from git: "revision 2" at the tag `spec-0.2-r2`, "revision 2 + later changes" after it, "not released" before the first tag; always with the commit date and short hash, and a note when `spec/` or this folder has uncommitted changes.

## How the forms are kept in step

- **One source.** Only the chapters are edited. Everything else is built from them in one run; of the built files, only the combined texts are committed, and the build checks them.
- **One parse.** pandoc reads the chapters once per language. One filter, `rnl.lua`, gives the web page and the PDF their anchors, reference links and code colours; only the layout differs: `page.html` for the web, `print.typ` for the PDF (pandoc writes Typst, Typst makes the PDF).
- **Checks before building**, any of which stops the build:
  1. the Chinese chapters have the sections, list items, table rows and code of the English ones;
  2. each Chinese part starts with `<!-- translated from en/ch09.md sha256:… -->`, the hash of the English part it was translated from; when the English changes, the build stops until the Chinese is brought in line and `--stamp` is run;
  3. Annex A in both languages, and the JSON excerpt in 5.3, are what the word list gives;
  4. every section number cited in the text exists;
  5. the combined texts are what the chapters give.
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
| `rnl.lua` | the pandoc filter: anchors, reference links, RNL code colours, the Typst table of contents |
| `page.html` | the web page template |
| `print.typ` | the PDF template |

`dist/` is ignored by git; `dist/work/` keeps the pandoc input, the Typst source, and `links-en.tsv`, `links-zh.tsv`: every reference that was linked, for review.
