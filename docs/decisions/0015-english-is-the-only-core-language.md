# 0015. English is the only core language; every other language, Chinese included, is a translation

- Date: 2026-10-25
- Status: accepted; point 8 amended on 2026-10-25: the notices are in English

## Context

The project has been written in English and Chinese. The two languages are kept in about two dozen places and in six ways:

- chapters of the standard, one folder per language, each Chinese chapter stamped with the hash of the English one;
- whole files beside their source, such as `README.zh.md`, with no record of the source;
- `<zh>…</zh><en>…</en>` groups in the source files of the site;
- `{"en": …, "zh": …}` dictionaries in data, such as the timeline and the entries of the gallery;
- a catalog keyed by the English text, `apps/viewer-ui/locales/zh-CN.json`, shared by the viewers' page and the desktop app's native menus;
- tables in code, such as the Chinese fixed phrases of Annex A in `scripts/spec/spectools.py`.

Only the chapters of the standard show when a translation has fallen behind its source. Elsewhere nothing does. For example, the catalog of the viewers holds sentences that the interface no longer has, and `site/src/llms.zh.txt` missed the changes that #178 made to `llms.txt`. At least eight places in the code assume that there are exactly two languages.

The user asked that the project now plan for keeping its languages up over time, across the site, the documents and the viewers: one entry from which a later maintainer of another language can follow how much there is to translate, and one way to submit the support of a language. Reviewing the plan for it, they decided that from now on Chinese becomes merely one of the supported languages and the core of the project keeps English as its one language: new content may first be published in English alone, and the other languages, Chinese included, follow.

## Decision

1. **English is the project's only core language.** New content may be published in English alone: a page of the site, a document, the standard, a text of the viewers, release notes or the skill package.
2. **Every other language is a translation, Chinese included.** A translation follows the English, kept in the places below, and wherever it has no text the English is shown. Chinese is treated like any other language. It keeps no place of its own beside the English in source files: the Chinese text of the site's pages and timeline moves out of `site/src/`, and the viewers' catalog moves to `i18n/zh/`, both in steps that leave what users see unchanged.
3. **A language has four places**, listed for every surface in [`i18n/surfaces.json`](../../i18n/surfaces.json); `python scripts/i18n/status.py files LANG` prints them:
   - `i18n/<lang>/`: catalogs keyed by the English text, for the viewers (`viewer.json`), the site (`site/<path under site/src>.json`) and the fixed phrases of the standard (`spec.json`), and the language's glossary;
   - whole documents beside their source, named `<name>.<lang>.<ext>`, such as `README.zh.md`;
   - the standard's chapters, in `spec/<version>/<lang>/`;
   - an alias package, `libraries/standard/<series>/rnl-<lang>.rnl`. Alias packages are part of the language (16.5 of the standard), not translations of the interface; whether a language has one is decided with the standard. `rnl-zh` is not affected by this decision.
4. **[`i18n/languages.json`](../../i18n/languages.json) is the one list of languages.** It was `site/languages.json`. A language is `published`, shown in the language menus of the site and the viewers, or `incubating`, built and tested but not shown.
5. **A stale translation is found from the source it was made from.**
   - A whole document starts with `<!-- translated from <path> sha256:<first 12 hex digits> -->`, the hash of its source with line ends as LF, as the chapters of the standard already do.
   - A catalog is keyed by the English text. When the English changes, the old entry is left unused and the new text has none; the report pairs the two, with the old translation.
6. **Checks.** A translation in a malformed format fails its check: invalid JSON, placeholders that differ from the English, a malformed stamp. A missing, stale or unused translation is reported, in the summary of the check and by `python scripts/i18n/status.py`, and fails no check and no build.
7. **The standard.** The English text remains normative. A translation of the standard is published as a form of a release, the PDF and the combined Markdown, only when every part of it is complete and in line with the English. The site shows the Chinese pages of the standard as they are. A chapter whose English has changed since it was translated carries a notice that it may differ from the English and that the English prevails. Version 0.2 is frozen and gets no new translations.
8. **When a language is shown.**
   - A language is `published` when the site's navigation, introduction and page for agents are fully translated, the other pages at least 80% and the viewers at least 95%, and a native speaker named in the registry has reviewed it. A draft may be made by machine translation or by an AI model; it is shown only after that review. Chinese meets this today and stays shown.
   - A page of a published language that is less than 80% translated is shown in English, with a notice that the page is not yet translated.
   - A page between 80% and 100% is shown translated, with the English where the translation is missing, and a notice that part of it is not yet translated.
   - These notices, as every notice of the system, such as that of point 7, are in English, in every language, Chinese included. The user decided so on reading the first draft, which had them in each language: they are all in English, since by the principle a notice in Chinese was itself a fault; and they asked that the project be checked for Chinese embedded elsewhere. Their texts are therefore no translatable message.
9. **Not translated**:
   - the diagnostics of the core and the command line, whose wording and language the standard leaves open (7.6), and the command line's usage text;
   - the skill package, which is in English only;
   - the developer documents: `docs/`, the READMEs of the crates, apps and packages, and `CONTRIBUTING.md`;
   - the bodies of release notes, whose one-sentence timeline summary may be translated;
   - comments in code and in package files;
   - the tags of the gallery.
10. **Language codes.** `zh` remains the code of Simplified Chinese in addresses and file names, written `zh-CN` in HTML and kept as `zh-CN` in the viewers' settings. Traditional Chinese would be `zh-Hant`. The registry maps the codes of each place, such as `{"viewer": "zh-CN"}`.
11. **Public packages.** A record of the public catalog requires an English summary only; summaries in other languages are kept outside the immutable record, so that they can be added to published versions. This changes the bilingual summary of [decision 0012](0012-public-packages-use-an-immutable-catalog.md) once rich-netlist/rnl-packages implements it; the `zh` summaries of published records remain valid.
12. **No hosted translation platform for now.** The formats, JSON catalogs and Markdown, are ones such platforms read, should one be wanted later.

## Consequences

- [`i18n/README.md`](../../i18n/README.md) is the entry for maintainers of a language: what there is to translate, how far each language is, how to add or update one. `python scripts/i18n/status.py` gives the table, and the workflow `i18n.yml` writes it into the summary of every pull request.
- The work is done in steps. Each step leaves what users see unchanged, but for the notices of point 8.
  1. The registry, the list of surfaces, the report and this record. Stamps on `README.zh.md` and `site/src/llms.zh.txt`.
  2. The viewers: every interface text found by the scan, the catalog in `i18n/zh/viewer.json`, the language menu built from the registry.
  3. The site: the Chinese text out of `site/src/` into `i18n/zh/site/`, the notices of point 8 and a page of the translation status.
  4. The standard: Chinese no longer has to be in line for the build to pass; stale chapters get the notice of point 7; the fixed phrases move to `i18n/zh/spec.json`; stamps on the short guide and the principles.
  5. rich-netlist/rnl-packages and rich-netlist/rnl-gallery.
- Until step 4, `scripts/spec/build.py` still stops when a Chinese chapter is out of line with the English. Until step 3, the site still reads its Chinese from the groups in `site/src/`. Until then, the rules of this record apply to new text: English alone may be merged, and the Chinese can follow.
- A contributor who changes English text need not change any translation. The translations show where they have fallen behind.
