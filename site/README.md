# site

Six static pages, in Chinese and English, for circuit designers who have not worked on EDA tooling:

- `index.html`, the introduction: what RNL is, the problem it addresses, the goal, before/after scenarios, where things stand (the released milestones and what comes next, with an FAQ), and the ways on to the other pages. It is short on purpose; what it once held has moved ([Sections that moved](#sections-that-moved)).
- `language.html`, the language tour: the language at a glance (the anatomy of a statement, the keyword families, how a level takes effect), nine hands-on sections on the parts of the language the introduction does not unpack (standard sentences and Chinese, definitions, the standard package's defaults, sets and selections, wires, text, orientation, priority, the header record), and a few examples.
- `tools.html`, the page of the tools: what the viewers of the first toolkit, 0.1, do; the web viewer, with a few samples to open in it; the desktop viewer for Windows and Linux, with the files of its newest release; the toolchain, its modules and how data flows through them; and the licences.
- `gallery.html`, the gallery of circuits: a list of designs in public GitHub repositories, each shown by a thumbnail and opened in the web viewer. The page holds no data: it reads `index.json` and the thumbnails from the GitHub Pages of [rnl-gallery](https://github.com/rich-netlist/rnl-gallery), where its checks write them ([The circuit gallery](#the-circuit-gallery)).
- `agents.html`, the entry for AI agents: how to get the RNL skill package and install it, what is in it, and how to check and draw the files an agent writes ([For agents](#for-agents)).
- `packages.html`, the gallery of the standard packages: every package in [`libraries/`](../libraries/README.md), with what it is for, what it references and its text, every symbol as the core draws it, with its pins and box, and the rules that give elements their symbols ([The gallery](#the-gallery-of-the-standard-packages)).

The site also publishes the skill package for agents, at `skill/`, and `llms.txt`, at `llms.txt` and `zh/llms.txt`; the web viewer itself, at `viewer/` ([`apps/web`](../apps/web/README.md)), one copy that the pages of both languages link to, with the conformance corpus as its samples; the pages of the language standard, at `zh/spec/<version>/` and `en/spec/<version>/` for each version, such as `zh/spec/0.3/`, with the short guide to the language as their first section and the principles of RNL as their last; and the gallery's data, at `gallery/`. [`pages.yml`](../.github/workflows/pages.yml) adds them to what `build.py` writes.

**Status:** an explainer, not a specification. The RNL snippets follow the language standard 0.3 and the standard series 0.3 ([`spec/`](../spec/README.md), [`libraries/standard/0.3`](../libraries/standard/README.md)); the pages say so and link to the standard. Every file the pages show reads with `rnl check` in 0.3 without errors, but for the ones that show errors on purpose, and every statement they show reads within a netlist that has its objects. Layouts in the drawings are placed by hand, not produced by a solver, with the pins where the series 0.3 puts them; the symbols of the gallery are drawn by the core.

## One source, extensible locales

Pages are built from one source in `src/`, using the locale registry in
[`languages.json`](languages.json) and [`scripts/site_i18n.py`](../scripts/site_i18n.py).
Only Chinese and English are enabled. The registry owns language codes, native names, URL
directories, browser-language matching, fonts, navigation breakpoints and shared interface
messages, including those of the language-standard pages. English is the fallback source.
Text remains next to its translations:

```html
<h2><zh>同一件事，两种做法。</zh><en>Same task, two ways.</en></h2>
<button aria-label="<zh>切换配色</zh><en>Switch color theme</en>">…</button>
```

```js
cap: '<zh><b>只有网表</b>：网表里只有器件和连接。</zh><en><b>Netlist only</b>: the netlist holds only devices and connections.</en>'
```

The same markup works in HTML text, attribute values, and JavaScript strings. Structure, drawings, and behavior exist once, so a change to the layout or to a demo applies to both languages at the same time.

When you edit text:

- Every group ends with its required `<en>` block. Optional translations precede it, in any order, with only whitespace between blocks. Missing translations use the English text; English itself must be present. Duplicate, nested and unclosed language blocks fail the build.
- An empty block is allowed where one language needs no text, for example `<zh>Host adapter</zh><en></en>` for a gloss the English page does not need.
- For a phrase with a number or name in it, translate the whole phrase and use a placeholder, as `fill('<zh>交叉 {crossings} 处</zh><en>{crossings} crossings</en>', m)` does in `intro.js`, rather than splitting it into fragments.
- Links to another page are relative to that language's directory. Shared destinations such as the viewer use `../viewer/`. Language menus, fonts, HTML language attributes and primary navigation labels are generated from the registry rather than translated in each template.
- Code comments are in English and are not translated.

The English source still checks for accidentally leaked Chinese. Intentionally quoted text
uses an HTML `lang` attribute; Chinese examples in scripts use `/*zh-CN*/` and `/*/zh-CN*/`.
This check applies to English, so it does not reject another locale's CJK translations.

### Adding a language

Add one entry to `languages` in `languages.json`, for example:

```json
{"code": "ja", "name": "日本語", "short": "JA"}
```

This creates `ja/` pages, adds Japanese to every menu and browser-language negotiation,
and inherits English interface messages, fonts and content. Add translations progressively,
before the English block, such as `<ja>紹介</ja><en>Introduction</en>`. There is no need to
add an empty Japanese block to every existing group. An explicit empty block stays empty;
it means intentional omission, rather than missing translation. Translations of shared UI
messages go in the entry's `messages`; font settings and `nav_breakpoint` can be overridden.
Timeline dictionaries follow the same rule: `en` is required, other registered codes are optional.

The `standard` field selects an existing standard-document language; it defaults to `en`.
A new site locale therefore links to the English standard until a translated standard is
published. Existing Chinese and English standard sources, translation stamps and formal
publication checks remain separate from website prose fallback.

The native language menu works without JavaScript; with scripts it also closes on Escape
or an outside click and remembers the explicit choice. Switching preserves the page,
query string and current section, including package-gallery filters. The root entry uses
the saved choice, then supported browser preferences, then English. Old root URLs still
redirect to their original Chinese pages.

`RNL_SITE_LANGUAGES=/absolute/path/languages.json` selects a temporary registry for local
checks. Browser tests generate a third-locale fixture in an OS temporary directory and
remove it afterwards; it is never included in the deployed site. Set `RNL_I18N_SCREENSHOTS`
to capture the open menu in both languages and themes at 1440px and 390px.

## The desktop viewer's release

The page of the tools links to the files of the desktop viewer's newest release, which [`viewer-release.yml`](../.github/workflows/viewer-release.yml) publishes on a tag `viewer-vX.Y.Z` as the GitHub release of that tag. The version is not written in the page: `build.py` takes it from the newest such tag in the repository, by version, and fills it in. A tag with anything after the version, such as a release candidate's `viewer-v0.2.0-rc.1`, does not count. The source marks what depends on it:

```html
<!--release--><a href="https://github.com/rich-netlist/rnl/releases/download/{{viewer-tag}}/RNL-Viewer-{{viewer-version}}-windows-x64.msi">…</a><!--/release-->
<!--no-release--><p>…the first release is coming…</p><!--/no-release-->
```

With a release, the build keeps the `<!--release-->` blocks and fills in `{{viewer-version}}`, such as `0.1.0`, and `{{viewer-tag}}`, `viewer-v0.1.0`; without one, it keeps the `<!--no-release-->` blocks, which say that the first release is coming and link the [GitHub releases](https://github.com/rich-netlist/rnl/releases). It refuses to build when a block does not close or holds another, and when a field is used outside a `<!--release-->` or `<!--linux-release-->` block or is not one of the two. Both kinds of block are checked for their languages, whichever the build keeps.

`pages.yml` checks out the whole history, tags included, and runs again after each release of the desktop viewer, so that the published page links to the new files. `--viewer-version` sets the version for a local build or a test, whatever the tags say, and `--viewer-version none` builds the page as before the first release.

## The homepage timeline

`build.py` fills the homepage's `{{timeline}}` marker from `src/timeline.json` and the
repository's `viewer-v*`, `spec-*-r*` and `core-v*` tags. Shanghai dates come from tag creation
times (commit times for lightweight tags), and only the latest published milestone gets
`class="now"`. The existing timeline elements, styles and Next entries are retained.

Release notes carry a one-sentence `timeline-summary` JSON block with English and optional translations; the
[release checklist and template](../release-notes/README.md) define the paths and format.
Manual milestones can claim tags to override or combine them, and hold non-release events
with their actual PR merge timestamps and source links. Old release summaries can be
backfilled in the checkout without moving old tags. Core generation is ready for its first
real tag; an unreleased core is never added just because its workflow exists.

Pages fetches full history and tags, rebuilds after Viewer, standard and runtime/core
publication, and checks notes changes. The Stage text stays unchanged in this update.

## The gallery of the standard packages

`packages.html` shows what [`gallery/build.mjs`](gallery/build.mjs) writes into `site/dist/gallery/` from the built-in standard series and the latest [public package catalog](https://rich-netlist.github.io/rnl-packages/catalog.json). The page is built by `build.py` like the others; its data is not, and is the same for both languages.

- **Groups.** Each standard-series directory and each public package selected from the latest catalog is a group in the left navigation. The build writes its `kind`: `libraries/standard/*` is `standard`, `libraries/pdk/*` is `pdk`, and other directories are `extension`. The navigation lists standard series (newest first, with 0.2 marked read-only), extensions, then PDKs. A group expands to its packages and an All entry. Search filters group and package names without changing how symbols are searched. The `series`, `package` and `q` query parameters keep shared and earlier links working; group and package selections also support browser Back and Forward. A package is known by the name and version its header record declares; what it references and the order in which using it brings packages in come from its `USE-PACKAGE`s, and the packages `rnl-std` brings in are the ones every netlist has by default. Adding a directory or a file needs no change to the page code.
- **Statements.** [`gallery/package-file.mjs`](gallery/package-file.mjs) splits each record of a package file into its statements and sorts them by their first tokens: symbols, styles, definitions, sets, rules (symbol, style, text and set rules) and definitions attached to sets. It is not an interpreter; what a statement means and how a symbol looks come from the core.
- **Symbols.** Every symbol a package defines with a `$` name is drawn by the core as WebAssembly, the one the web viewer runs: the build writes a netlist with a subcircuit for each symbol, in which one instance is drawn with the symbol at the origin, and asks the core for the drawing and the snapshot of every level, with the package files of `libraries/` in the request, so that they are drawn as they are there whatever the core bundles. The labels a symbol fills with `${…}` show their keys, such as `name` and `value`. The pins, their directions and the box come from the core; the labels of the pins and the texts of the labels, from the text. Each pin is marked on the picture with its terminal number and the arrow of the direction it points, where its wire leaves, such as `1↑`. The names of the terminals, which the picture does not show, are listed under it when the comment right before a symbol starts with them, as `d g s b; from Analog Canvas nmos`. All symbols are drawn at the same scale, and the page shows or hides the pins, the box and the labels on demand. A thumbnail, as in the table of symbol rules, is framed on the symbol alone and scaled to fit.
- **What each package is for**: a sentence in both languages in `SUMMARY` in [`src/assets/gallery.js`](src/assets/gallery.js), for every version of a package, or for one version where they differ, as for `rnl-std-fill 0.3`; a package without one shows the comment its file starts with.
- **Problems.** A package file the core reports errors or warnings for, a symbol it reports them for while drawing, or one whose pins it gives differently from the text, is listed by the build, as a warning annotation in GitHub Actions, and marked on the page. The build fails only when the core cannot read a gallery's netlist or draws no level for a symbol.

The data is scripts that the page loads with `<script>`, `index.js` and one file per group, so that the page also works opened from the disk.

The Origin toggle adds a red registration mark at each symbol's `@PLACE(0, 0)` point on
cards, rule thumbnails and enlarged views. It starts off and is saved with the other
display settings. The build includes that point in both SVG frames and writes its SVG
coordinates as `origin`; the page keeps the cross at 10 screen pixels, its stroke at
1.5 pixels and its centre dot at a 1.5-pixel radius as previews resize. It reserves space
for the mark at the frame's edge when needed. Compared with the previous frames, only
`rnl-std-virtual 0.2`'s `$CURRENT_ARROW_SYM` expands to include an outside origin.

## The circuit gallery

`gallery.html` is a page of cards, one per entry of [rnl-gallery](https://github.com/rich-netlist/rnl-gallery): a thumbnail of the design, its title, description and author, its licence and the repository and commit it is read from. Unlike the gallery of the standard packages, nothing of it is built with the site: [`circuits.js`](src/assets/circuits.js) fetches `https://rich-netlist.github.io/rnl-gallery/index.json` (with `cache: "no-cache"`, so that the browser asks again every time and a new entry shows up when rnl-gallery has published it, a few minutes after its merge) and the thumbnails it names. It reads nothing else: no GitHub API, no netlists.

- **What is trusted.** The index is the work of reviewed pull requests, but the page treats it as data: titles, descriptions and authors are set as text, a thumbnail must be `thumbs/….png` of that site, a manifest `manifests/….json`, a source link a github.com address, a viewer address a raw.githubusercontent.com one.
- **Opening an entry.** A design of one file opens in the web viewer with `../viewer/?url=<raw address>`, and its `level` when it has one. A design of several files is opened by the viewer from its manifest, `?manifest=`, which the viewer does not do yet: `MANIFEST` in `circuits.js` is `false` until it does, and such a card shows its thumbnail, its source and a note.
- **When it cannot be read.** An index that does not come within eight seconds or answers with an error leaves the message and a link to the repository; if the browser holds one it has read before, that is shown, with a note. An index of a newer `format` than the page knows asks for a reload.
- **Tests.** [`tests/circuits.spec.mjs`](tests/circuits.spec.mjs) answers the requests to rnl-gallery from [`tests/fixtures/circuits/`](tests/fixtures/circuits) and checks the cards, the addresses, that nothing from the index is shown as markup, the search with `?q=` and the language switch, the failures, and the layout in both themes at 1440px and 390px. `RNL_CIRCUITS_SCREENSHOTS` saves those eight screenshots.

## For agents

`agents.html` and the files it links make the entry for AI agents. The page is built by `build.py` like the others, from `src/agents.html`, with its own `assets/agents.css` and `assets/agents.js` (the copy buttons of the sentence to give an agent and of the commands); like the gallery it has no single-file version, and it has no old address at the root of the site (`NEW_PAGES`). What it links is not written by `build.py` but by [`scripts/skill/build.py --site`](../skills/README.md), which `pages.yml` runs:

- `skill/rnl-skill.zip`, the RNL skill package of [`skills/rnl/`](../skills/rnl/SKILL.md) in the Agent Skills format, and each of its files as plain text under `skill/rnl/`, with `skill/manifest.json` and `skill/SHA256SUMS.txt`;
- `llms.txt` and `zh/llms.txt`, the plain-text entry that agents look for: what RNL is, how to download and unpack the skill, the files it holds, how to check and draw files, and where the standard is. They are [`src/llms.txt`](src/llms.txt) and [`src/llms.zh.txt`](src/llms.zh.txt) with `{{site}}` and `{{github}}` filled in, and every address in them is absolute.

The page and both `llms` files name every file of the package, so that they cannot fall behind it: `scripts/skill/build.py --check` fails when one does not. The short guide to the language, which the package holds as `references/language.md`, is also the first section of the page of the standard 0.3 ([`scripts/spec`](../scripts/spec/README.md#the-short-guide)); the top bar of every page, of the standard's too, links "Agent".

## Building and viewing

All six pages share the standard page's seven primary links and the project's GitHub link, after Agent.
The introduction, language tour, tools and agents pages put their first-level section links beside the content, below the full-width
hero. Below 1000px, the top bar's Contents button opens that navigation; choosing an entry closes
it. `test_build.py` compares both languages' primary links and current-page markers with
`scripts/spec/page.html` and the publisher's labels, and checks the copied CSS values.

After building the site and gallery, run the browser checks with Playwright's Chromium and WebKit
(two workers, also run by Pages CI):

```bash
npm ci --prefix tests/viewer
node tests/viewer/node_modules/@playwright/test/cli.js install chromium webkit
node tests/viewer/node_modules/@playwright/test/cli.js test --config site/playwright.config.mjs
```

Set `RNL_SITE_SCREENSHOTS` to an output directory to capture the five Chinese light pages at
1440px, the introduction and gallery in English dark at 1440px and with the menu open at 390px,
and a gallery fixture with 15 groups and long names. The fixture is intercepted in the test browser;
it does not change the generated gallery or the published libraries.

Set `RNL_GALLERY_ORIGIN_SCREENSHOTS` to capture the standard 0.3 gallery with Origin off,
on and in the dark theme, an outside origin, and an enlarged symbol. The origin tests also
check saved settings, coordinates, fixed screen sizes and framing in both browsers.

The tools page offers Windows downloads and, for tags that include `apps/desktop/tauri.linux.conf.json` with both Linux bundle targets, Linux AppImage and deb downloads. `build.py` checks the configuration at the selected `viewer-v*` tag, so old Windows-only tags do not acquire broken Linux links. `<!--linux-release-->` and `<!--no-linux-release-->` are top-level conditional blocks alongside the existing release blocks. Run `python -m unittest discover -s site -p 'test_*.py'` to check old releases, Linux releases, and the bilingual and standalone pages; `pages.yml` runs these checks too.

From the repository root, with Python 3 and nothing else (and git, for the tags):

```bash
python site/build.py            # write site/dist/ (ignored by Git)
python site/build.py --check    # check the sources only
python site/build.py --zip rnl-intro.zip   # also pack the built site
python site/build.py --viewer-version 0.1.0   # link the files of the desktop viewer 0.1.0
```

`agents.html` links the package's downloads, which `build.py` does not write: `python scripts/skill/build.py --site site/dist` adds them after it (`build.py` empties `site/dist/`).

Then open `site/dist/zh/index.html` (Chinese) or `site/dist/en/index.html` (English) in a browser, or serve `site/dist/` with any static server, where the root leads to the pages in the browser's language; `language.html` and `tools.html` sit next to each `index.html`. Fonts load from Google Fonts and fall back to system fonts offline.

The web viewer is not written by `build.py`. To look at the pages with it, build it as `pages.yml` does and copy it in, then serve `site/dist/` with a server that sends `.wasm` as `application/wasm` and `.js` as JavaScript, such as Python's; the viewer only runs when served, not from the disk:

```bash
python scripts/fonts.py web
node apps/web/build.mjs --samples
cp -r apps/web/dist site/dist/viewer
python -m http.server --directory site/dist
```

The gallery's data needs the core as WebAssembly too, as `node apps/web/build.mjs` or `node bindings/runtime-wasm/build.mjs` builds it into `bindings/runtime-wasm/dist/`, and Node 18 or later with nothing to install. After `build.py`, which empties `site/dist/`:

```bash
node site/gallery/build.mjs                   # write site/dist/gallery/
node --test site/gallery/package-file.test.mjs   # the tests of reading package files
```

`--core DIR` takes the core from another directory, `--out DIR` writes into another built site, and `--libraries DIR` reads the package files of another checkout. Opened from the disk or served, `packages.html` then shows the gallery; without its data, it says how to build it.

| Output in `site/dist/` | Content |
| --- | --- |
| `zh/index.html`, `zh/language.html`, `zh/tools.html`, `zh/packages.html`, `zh/agents.html`, `zh/assets/` | Chinese pages |
| `en/index.html`, `en/language.html`, `en/tools.html`, `en/packages.html`, `en/agents.html`, `en/assets/` | English pages |
| `index.html` | The root of the site: it leads to `zh/` when the browser's first language is Chinese, and to `en/` otherwise or without scripts ([Addresses](#addresses)). |
| `language.html`, `tools.html`, `packages.html`, `rnl-*-standalone.html` | The old addresses of the Chinese pages, which were at the root until 2026-10-18 ([Addresses](#addresses)): each leads to the same page under `zh/`. |
| `gallery/` | The gallery's data, for both languages: `index.js`, the groups, and one script per group, such as `standard-0.2.js`. Not written by `build.py`: `pages.yml` writes it with `node site/gallery/build.mjs`, after building the web viewer and so the core. |
| `zh/spec/<version>/`, `en/spec/<version>/`, and `zh/spec/`, `en/spec/` | The language standard: for each version in `spec/`, one page per language with its PDF and Markdown downloads, and next to it the pages that compare it with each earlier version, such as `compare-0.2.html`; `zh/spec/` and `en/spec/` lead to the newest version, which the top bar links to. The old addresses of the Chinese pages of 0.2, `spec/0.2/` and `spec/`, lead to them, and the downloads are copied there. Not written by `build.py`: [`pages.yml`](../.github/workflows/pages.yml) adds them with `python scripts/spec/build.py --site site/dist` ([decision 0007](../docs/decisions/0007-publishing-the-standard.md)). |
| `skill/`, `llms.txt`, `zh/llms.txt` | The skill package for agents as a zip and as plain-text files, and the plain-text entry for agents, in English and in Chinese ([For agents](#for-agents)). Not written by `build.py`: `pages.yml` adds them with `python3 scripts/skill/build.py --site site/dist`. |
| `viewer/` | The web viewer with its samples, one copy for both languages, its interface in English. Not written by `build.py`: `pages.yml` builds it with `node apps/web/build.mjs --samples` and copies `apps/web/dist/` here. |
| `zh/rnl-intro-standalone.html`, `zh/rnl-language-standalone.html`, `zh/rnl-tools-standalone.html`, and the same under `en/` | Single-file pages with the styles and scripts inlined; their links to the other pages, the languages and the web viewer point to the published site |

[`pages.yml`](../.github/workflows/pages.yml) publishes `site/dist/` to GitHub Pages, at <https://rich-netlist.github.io/rnl/>, which leads to the Chinese pages at <https://rich-netlist.github.io/rnl/zh/> or the English ones at <https://rich-netlist.github.io/rnl/en/> by the browser's language, whenever `site/`, `spec/` or `scripts/spec/` changes on `main`, or what the web viewer is built from, the package files in `libraries/` among it; after a revision of the standard is released; and after a release of the desktop viewer. The web viewer is at <https://rich-netlist.github.io/rnl/viewer/>. The language switch keeps the reader on the same section.

### Sections that moved

The introduction once held more sections. They are now where the pages they belong to are, and their old addresses, the sections of the introduction, lead there ([`index.html`](src/index.html) has a small script in its head for it):

| Old address | New address |
| --- | --- |
| `/rnl/zh/#tools` (the toolchain), also `#toolchain` | `/rnl/zh/tools.html#toolchain` |
| `/rnl/zh/#principles` (the design principles) | `/rnl/zh/spec/#principles`, the section of the principles on the standard's page, last of the page |
| `/rnl/zh/#language` (the language) | `/rnl/zh/language.html#glance` |
| `/rnl/zh/#examples` (the examples) | `/rnl/zh/language.html#examples` |

The same under `/rnl/en/`. The section of the principles is made by [`scripts/spec`](../scripts/spec/README.md#the-principles) from the files that are the principles, [`spec/rich-netlist.md`](../spec/rich-netlist.md) and its Chinese version, and is not written on the site. The old addresses of the Chinese pages, and the root, are in [Addresses](#addresses) below.

### Addresses

Both languages sit one level below the root, `zh/` and `en/`, alike. The root, `index.html`, leads to `zh/` when the browser's first language (`navigator.languages[0]`, else `navigator.language`) is Chinese, `zh` or `zh-…` such as `zh-CN`, `zh-TW` or `zh-Hans`, and to `en/` otherwise, keeping the query and the section of the address; without scripts, a refresh in `<noscript>` leads to `en/`, and the page links both. It names `zh/` and `en/` as its alternates (`hreflang`), `en/` as the default (`x-default`). Until 2026-10-18 the Chinese pages were at the root; their old addresses still work, since GitHub Pages has no redirects of its own: each is a page that leads to the new address with the query and the section of the old one, at once by a script, or by a refresh where scripts do not run. The pages of the standard keep theirs the same way ([`scripts/spec`](../scripts/spec/README.md)), and the downloads of 0.2 are copies at their old addresses, since no page can stand in for a PDF.

| Old address | New address |
| --- | --- |
| `/rnl/` | `/rnl/zh/` when the browser's first language is Chinese, else `/rnl/en/` |
| `/rnl/language.html`, `tools.html`, `packages.html` | `/rnl/zh/language.html`, `tools.html`, `packages.html` |
| `/rnl/rnl-intro-standalone.html` and the other two | `/rnl/zh/rnl-intro-standalone.html` and the other two |
| `/rnl/spec/` | `/rnl/zh/spec/0.2/`, the newest version |
| `/rnl/spec/0.2/`, such as `/rnl/spec/0.2/#1.3` | `/rnl/zh/spec/0.2/`, such as `/rnl/zh/spec/0.2/#1.3` |
| `/rnl/spec/0.2/rnl-0.2.zh.pdf`, `.zh.md`, `.zh-chapters.zip`, `SHA256SUMS` | copies of `/rnl/zh/spec/0.2/…` |

## Files

| Path | Content |
| --- | --- |
| `src/index.html` | Introduction: structure and text |
| `src/language.html` | Language tour: structure and text |
| `src/tools.html` | Page of the tools: structure and text, with the blocks that depend on the desktop viewer's release |
| `src/packages.html` | Gallery of the standard packages: structure and text; the gallery itself is drawn by `gallery.js` from the data |
| `src/agents.html` | The entry for AI agents: structure and text |
| `src/llms.txt`, `src/llms.zh.txt` | The plain-text entry for agents, English and Chinese, which `scripts/skill/build.py --site` writes to `llms.txt` and `zh/llms.txt` |
| `src/assets/agents.css`, `src/assets/agents.js` | Styles and copy buttons of the page for agents, on top of `intro.css` and `common.js` |
| `src/gallery.html` | The circuit gallery: structure and text |
| `src/assets/circuits.js`, `src/assets/circuits.css` | The circuit gallery: reading rnl-gallery's index, the cards, the search; their styles on top of `gallery.css` |
| `src/assets/gallery.js` | The gallery: loading the data, the groups, search, the packages and their statements, the symbol cards and the details of a symbol, what each package is for |
| `src/assets/gallery.css` | Styles of the gallery, on top of `intro.css`, with the colours of the core's drawings mapped to the theme |
| `gallery/build.mjs` | Builds the gallery's data from standard files and verified public catalog downloads, with the symbols drawn by the core as WebAssembly |
| `gallery/package-file.mjs` | Reads a package file: its header record, its comments, and its statements by kind, with the pins and labels of the symbols |
| `gallery/package-file.test.mjs` | Tests of `package-file.mjs`, and that every package file in `libraries/` reads as a package |
| `src/assets/intro.css` | Styles of all pages: light and dark palettes, narrow-screen layout, no animation under `prefers-reduced-motion`, fonts per language |
| `src/assets/schem.js` | Small schematic renderer in RNL coordinates (x right, y up): self-drawn textbook symbols with the standard package's pin positions, the eight orientation codes, pin-based wiring, animated layout transitions, annotation layer |
| `src/assets/rnl-code.js` | Display-only highlighting of SPICE lines, `@RNL` records (keywords, aliases, references, free text) and whole package files, and code-line/drawing cross-highlighting |
| `src/assets/scenes.js` | Circuits, layouts, wiring, annotations, and example files for the introduction's demos |
| `src/assets/common.js` | Shared by all pages: theme, navigation (a top bar whose links do not fit draws them closer, then writes the short forms, such as T1 to T9 in the tour, and then folds them into a menu), tabs, segmented controls, highlighting of static snippets |
| `src/assets/intro.js` | Introduction interactions: the hero, the scenarios and the journey |
| `tests/logo-embed.spec.mjs` | The homepage logo's lazy iframe, close/focus handling, theme synchronization and 390px layout, tested in Chromium and WebKit. `RNL_EMBED_REVIEW_DIR` saves screenshots and a before/after click request log. |
| `serve.py` | Local static server for browser tests, with enough queued connections for both workers' viewer module requests. |
| `src/assets/toolchain.js` | The toolchain diagram of the page of the tools |
| `src/assets/glance.js` | The language at a glance, with the 46-keyword reference, and the examples, of the language tour |
| `src/assets/tour.js` | The interactions of the nine sections of the language tour |
| `build.py` | Builds every page in the registered locales, the single-file pages (the gallery has none), and the optional zip; checks translation groups and release blocks, and finds the desktop viewer's newest release |

The symbols are drawn for this page and contain no Analog Canvas material. Like other paths outside `spec/`, `docs/`, and the data directories, this directory is licensed under [MIT](../LICENSE).

Public package names, versions, kinds and bilingual descriptions come from the catalog; source links point to rnl-packages. Every download is verified against SHA-256. `gallery/catalog-provenance.json` records the catalog URL and raw SHA-256 used for the build. `RNL_PACKAGE_CATALOG` can select another catalog URL or a local file. Browser tests use the locked local catalog generated by `python scripts/packages.py fixture`, then the Pages job restores the production gallery before deployment. The Pages workflow rebuilds main daily at 02:17 UTC and supports workflow_dispatch; see CONTRIBUTING.md for GitHub's inactivity limit.

## CLI downloads and release versions

The bilingual tools page has a command-line download section with fixed
`releases/latest/download/rnl-TARGET.zip` or `.tar.gz` links and `SHA256SUMS.txt`.
Latest belongs to the core (`core-vX.Y.Z`); these links become available at its first release.
Viewer links continue to use the latest stable `viewer-v*` tag selected by `build.py`,
independently of the core and `spec-*` standard releases. The web viewer reads the two
Viewer version declarations and writes `about.json` with `version`, `coreVersion` and
`commit`. See [release procedures](../docs/releases.md).
