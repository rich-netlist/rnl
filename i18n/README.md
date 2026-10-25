# i18n

The start for anyone who translates RNL, or maintains one of its languages: what there is to translate, how far each language is, and how a language is added or brought up to date.

English is the project's only core language ([decision 0015](../docs/decisions/0015-english-is-the-only-core-language.md)). New content may be published in English alone. Every other language, Chinese included, is a translation that follows the English; wherever a translation has no text, the English is shown. A missing or stale translation is reported, and never fails a check or a build.

| File | Content |
| --- | --- |
| [`languages.json`](languages.json) | The one list of languages: code, native name, HTML language, how browsers' languages match it, fonts, the site's shared messages, and `status`, `published` or `incubating` |
| [`surfaces.json`](surfaces.json) | Every kind of text there is to translate, a *surface*: where its English is, where each language's translation goes, and the texts that are not translated, with the reason |
| [`../scripts/i18n/status.py`](../scripts/i18n/status.py) | The report: how far each language is in each surface, what is missing, what is stale |
| `<lang>/` | The catalogs of one language (from the later steps below) |

## How far each language is

```bash
python scripts/i18n/status.py
```

prints one row per surface and one column per language, such as `1371/1371` or `1/1, 1 stale`. For one language in detail, with what is missing, what is stale and since when:

```bash
python scripts/i18n/status.py --lang zh
```

For the files of a language, those that exist and those still to be written, also of one not registered yet:

```bash
python scripts/i18n/status.py files ja
```

`report --markdown` and `--json` give the same in Markdown and in JSON. `--online` also reads what rich-netlist/rnl-gallery and rich-netlist/rnl-packages publish. `--base origin/main` also compares with that commit, as a pull request does. The workflow [`i18n.yml`](../.github/workflows/i18n.yml) runs `check` on every pull request and puts the report into the summary of the run.

## Where a language's translations go

Four places, the same for every language:

| What | Where | Example |
| --- | --- | --- |
| Catalogs of short texts and of the site's pages, keyed by the English | `i18n/<lang>/` | `i18n/ja/viewer.json`, `i18n/ja/site/index.html.json` |
| Whole documents | beside their source, `<name>.<lang>.<ext>` | `README.ja.md`, `spec/rnl-0.3-guide.ja.md`, `site/src/llms.ja.txt` |
| The standard's chapters | `spec/<version>/<lang>/` | `spec/0.3/ja/ch01.md` |
| An alias package, so that files can be written with the language's own words | `libraries/standard/<series>/rnl-<lang>.rnl` | `libraries/standard/0.3/rnl-zh.rnl` |

An alias package is part of the language (16.5 of the standard), not a translation of an interface; whether a language has one is decided with the standard.

Chinese is moving to the same places in steps (below). Until then, `surfaces.json` names where it is, and `status.py files zh` lists it.

## Formats

**A catalog** is a JSON object keyed by the English text:

```json
{
  "&Open File...\tCtrl+O": "ファイルを開く(&O)...\tCtrl+O",
  "Open {0}": "{0} を開く",
  "Same task, two ways.": null
}
```

- A text is the translation; `null` is one not made yet, shown in English; `""` is a text left out on purpose, where the language needs none.
- Placeholders such as `{0}` or `{name}` are kept. One may be left out where the language does not need it, such as the `{1}` that gives English its plural's "s", but none may be added.
- The shortcut after a tab, such as `\tCtrl+O`, is kept as it is. A menu label has at most one access key, `&`.
- In the site's catalogs, markup such as `<b>`, `<code>` and `<a href="…">` is kept with its addresses. The translation has no more straight quotes, `'` and `"`, and no more backslashes than the English: use typographic quotes, ’ “ ”, since the site puts the text into scripts' strings.
- A key is the English with its runs of white space as one space. When the English changes, the old entry no longer matches: the report pairs it with the new English it most resembles and shows both, with the old translation.

**A whole document** starts with the source it was translated from and the start of that source's SHA-256, its line ends as LF:

```text
<!-- translated from README.md sha256:6dc587e50141 -->
```

The line is not shown on GitHub or on the site. Once a translation is in line with its source again:

```bash
python scripts/i18n/status.py stamp README.ja.md
```

`--as-of COMMIT` stamps it with the source as it is in that commit instead, such as the one that last changed the translation. The chapters of the standard keep their own stamp, `<!-- translated from en/ch01.md sha256:… -->`, written by `python scripts/spec/build.py --stamp`.

## Checks

`python scripts/i18n/status.py check` fails only on a format error, in any language:

- a catalog that is not a JSON object of texts or `null`;
- a placeholder the English does not have, a changed shortcut, a second access key;
- markup that `scripts/site_i18n.py` does not read;
- a document whose first line looks like a stamp but is not one, or names another source;
- `languages.json` or `surfaces.json` themselves malformed.

It reports, without failing:

- what is missing;
- what is stale, with the commit it was translated from and since when;
- entries of a catalog that the English no longer has;
- documents whose headings, lists, tables or code differ from their source;
- on a pull request, an English text that changed while the translation beside it did not.

## Adding a language, or bringing one up to date

1. Run `python scripts/i18n/status.py files <lang>`, and choose what to start with. The site's navigation and introduction, the page for agents and the viewers are what a reader meets first.
2. Write the translations in the places it lists, in the formats above. Translating with a machine or an AI model for a first draft is fine; a native speaker reviews the language before it is shown.
3. Open a pull request titled `i18n(<lang>): …`. The summary of its checks shows how far the language is.
4. A language is shown in the language menus of the site and the viewers, `published`, when the site's navigation, introduction and page for agents are fully translated, its other pages at least 80% and the viewers at least 95%, and a native speaker named in `languages.json` has reviewed it.

Anyone may update a translation at any time, in a pull request of its own. A pull request that changes English text need not change any translation.

**What works now, and what comes in the later steps of decision 0015:**

| Step | What it gives | State |
| --- | --- | --- |
| 1 | This folder, the report, stamps on `README.zh.md` and `site/src/llms.zh.txt` | done |
| 2a | The viewers: every text of the interface written where the scan finds it, `tr()` marking those that reach `m()` through a variable; counts as whole phrases; the entries the interface no longer has removed | done |
| 2b | The viewers: the catalog in `i18n/<lang>/viewer.json`, the language menu from `languages.json`, a manifest's title in any language. Until then the viewers have only English and Chinese | to come |
| 3 | The site: catalogs in `i18n/<lang>/site/`, Chinese moved there out of `site/src/`, the notices of a page not yet translated, `incubating` respected, a page of this report. Until then the site reads translations only from the `<lang>` blocks beside the English, and builds the pages of every registered language | to come |
| 4 | The standard: a stale Chinese chapter reported rather than stopping the build, with a notice on its page; the fixed phrases of Annex A in `i18n/<lang>/spec.json`; translations of the standard in further languages; stamps on the short guide and the principles | to come |
| 5 | rnl-packages: summaries in further languages outside the immutable records; rnl-gallery: a warning for an entry whose English changed alone | to come |

Until step 3, register a new language in `languages.json` only once it may be shown, as the site builds and publishes the pages of every language registered there.

## Not translated

`surfaces.json` lists these with their reasons:

- the diagnostics of the core and the command line, whose wording and language the standard leaves open (7.6), and the command line's usage text;
- the skill package for agents;
- the documents for developers (`docs/`, the READMEs of the crates, apps and packages, `CONTRIBUTING.md`);
- the bodies of release notes: only their one-sentence summary for the timeline is translated;
- comments in code and in package files, and the tags of the gallery.

Fonts are not a translation, but a language may need them. The core and the viewers draw with Noto Sans SC ([decision 0009](../docs/decisions/0009-fonts-shipped-with-the-core.md)). Han characters are drawn in their Chinese forms; Hangul and other scripts are drawn in a font of the system's. A Japanese or Korean interface would need the matching faces.
