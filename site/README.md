# site

Four static pages, in Chinese and English, for circuit designers who have not worked on EDA tooling:

- `index.html`, the introduction: what RNL is, the problem it addresses, the goal, before/after scenarios, the toolchain, the settled principles, the language at a glance, and a few examples.
- `language.html`, the language tour: nine hands-on sections on the parts of the language the introduction does not unpack (standard sentences and Chinese, definitions, the standard package's defaults, sets and selections, wires, text, orientation, priority, the header record).
- `tools.html`, the page of the tools: what the viewers of the first toolkit, 0.1, do; the web viewer, with a few samples to open in it; the desktop viewer for Windows, with the files of its newest release; and their licences.
- `packages.html`, the gallery of the standard packages: every package in [`libraries/`](../libraries/README.md), with what it is for, what it references and its text, every symbol as the core draws it, with its pins and box, and the rules that give elements their symbols ([The gallery](#the-gallery-of-the-standard-packages)).

The site also publishes the web viewer itself, at `viewer/` ([`apps/web`](../apps/web/README.md)), one copy that the pages of both languages link to, with the conformance corpus as its samples; the pages of the language standard, at `zh/spec/<version>/` and `en/spec/<version>/` for each version, such as `zh/spec/0.3/`; and the gallery's data, at `gallery/`. [`pages.yml`](../.github/workflows/pages.yml) adds them to what `build.py` writes.

**Status:** an explainer, not a specification. The RNL snippets follow language draft 0.2 (2026-10-12), which has since been consolidated into the language standard 0.2 in [`spec/`](../spec/README.md); the pages say so and link to it. The snippets have been checked with the RNL interpreter prototype, which lives in a research repository that is not public yet. Layouts in the drawings are placed by hand, not produced by a solver; the symbols of the gallery are drawn by the core.

## One source, two languages

Both languages of each page are built from the single source in `src/`. Every piece of text a reader sees is written once per language, next to each other:

```html
<h2><zh>同一件事，两种做法。</zh><en>Same task, two ways.</en></h2>
<button aria-label="<zh>切换配色</zh><en>Switch color theme</en>">…</button>
```

```js
cap: '<zh><b>只有网表</b>：网表里只有器件和连接。</zh><en><b>Netlist only</b>: the netlist holds only devices and connections.</en>'
```

The same markup works in HTML text, attribute values, and JavaScript strings. Structure, drawings, and behavior exist once, so a change to the layout or to a demo applies to both languages at the same time.

When you edit text:

- Change both blocks of a group together. A group is a `<zh>` block followed by an `<en>` block, with only whitespace between them. A long group can put `<en>` on the next line.
- An empty block is allowed where one language needs no text, for example `<zh>Host adapter</zh><en></en>` for a gloss the English page does not need.
- For a phrase with a number or name in it, translate the whole phrase and use a placeholder, as `fill('<zh>交叉 {crossings} 处</zh><en>{crossings} crossings</en>', m)` does in `intro.js`, rather than splitting it into fragments.
- A link to another page is relative to the page, so it differs between the languages where it leaves them: the web viewer is `viewer/` from a Chinese page and `../viewer/` from an English one, written `href="<zh>viewer/</zh><en>../viewer/</en>"`, as the language switch is.
- Code comments are in English and are not translated.

`build.py` refuses to build when a text is missing a language or has a stray language tag, and when Chinese characters reach an English page. Chinese that belongs on every page, such as the language switch or RNL written with the Chinese alias package `rnl-zh`, is marked: in HTML with `lang="zh-CN"` on the element, and in scripts by putting the data between `/*zh-CN*/` and `/*/zh-CN*/`. [`pages.yml`](../.github/workflows/pages.yml) runs the build on every pull request that touches `site/`, so an unpaired text fails the check before it is merged.

## The desktop viewer's release

The page of the tools links to the files of the desktop viewer's newest release, which [`viewer-release.yml`](../.github/workflows/viewer-release.yml) publishes on a tag `viewer-vX.Y.Z` as the GitHub release of that tag. The version is not written in the page: `build.py` takes it from the newest such tag in the repository, by version, and fills it in. A tag with anything after the version, such as a release candidate's `viewer-v0.2.0-rc.1`, does not count. The source marks what depends on it:

```html
<!--release--><a href="https://github.com/rich-netlist/rnl/releases/download/{{viewer-tag}}/RNL-Viewer-{{viewer-version}}-windows-x64.msi">…</a><!--/release-->
<!--no-release--><p>…the first release is coming…</p><!--/no-release-->
```

With a release, the build keeps the `<!--release-->` blocks and fills in `{{viewer-version}}`, such as `0.1.0`, and `{{viewer-tag}}`, `viewer-v0.1.0`; without one, it keeps the `<!--no-release-->` blocks, which say that the first release is coming and link the [GitHub releases](https://github.com/rich-netlist/rnl/releases). It refuses to build when a block does not close or holds another, and when a field is used outside a `<!--release-->` block or is not one of the two. Both kinds of block are checked for their languages, whichever the build keeps.

`pages.yml` checks out the whole history, tags included, and runs again after each release of the desktop viewer, so that the published page links to the new files. `--viewer-version` sets the version for a local build or a test, whatever the tags say, and `--viewer-version none` builds the page as before the first release.

## The gallery of the standard packages

`packages.html` shows what [`gallery/build.mjs`](gallery/build.mjs) writes into `site/dist/gallery/` from the package files in [`libraries/`](../libraries/README.md). The page is built by `build.py` like the others; its data is not, and is the same for both languages.

- **Groups.** Every directory under `libraries/` that holds `.rnl` files is a group, one tab of the page: `libraries/standard/0.2/` is the standard series 0.2, and any other directory, such as one of packages that are not standard, is a group of its own, marked as not in the standard series. The newest series comes first. A package is known by the name and version its header record declares; what it references and the order in which using it brings packages in come from its `USE-PACKAGE`s, and the packages `rnl-std` brings in are the ones every netlist has by default. Adding a directory or a file needs no change to the code.
- **Statements.** [`gallery/package-file.mjs`](gallery/package-file.mjs) splits each record of a package file into its statements and sorts them by their first tokens: symbols, styles, definitions, sets, rules (symbol, style, text and set rules) and definitions attached to sets. It is not an interpreter; what a statement means and how a symbol looks come from the core.
- **Symbols.** Every symbol a package defines with a `$` name is drawn by the core as WebAssembly, the one the web viewer runs: the build writes a netlist with a subcircuit for each symbol, in which one instance is drawn with the symbol at the origin, and asks the core for the drawing and the snapshot of every level, with the package files of `libraries/` in the request, so that they are drawn as they are there whatever the core bundles. The labels a symbol fills with `${…}` show their keys, such as `name` and `value`. The pins, their directions and the box come from the core; the labels of the pins and the texts of the labels, from the text. Each pin is marked on the picture with its terminal number and the arrow of the direction it points, where its wire leaves, such as `1↑`. The names of the terminals, which the picture does not show, are listed under it when the comment right before a symbol starts with them, as `d g s b; from Analog Canvas nmos`. All symbols are drawn at the same scale, and the page shows or hides the pins, the box and the labels on demand. A thumbnail, as in the table of symbol rules, is framed on the symbol alone and scaled to fit.
- **What each package is for**: a sentence in both languages in `SUMMARY` in [`src/assets/gallery.js`](src/assets/gallery.js), for every version of a package, or for one version where they differ, as for `rnl-std-fill 0.3`; a package without one shows the comment its file starts with.
- **Problems.** A package file the core reports errors or warnings for, a symbol it reports them for while drawing, or one whose pins it gives differently from the text, is listed by the build, as a warning annotation in GitHub Actions, and marked on the page. The build fails only when the core cannot read a gallery's netlist or draws no level for a symbol.

The data is scripts that the page loads with `<script>`, `index.js` and one file per group, so that the page also works opened from the disk.

## Building and viewing

From the repository root, with Python 3 and nothing else (and git, for the tags):

```bash
python site/build.py            # write site/dist/ (ignored by Git)
python site/build.py --check    # check the sources only
python site/build.py --zip rnl-intro.zip   # also pack the built site
python site/build.py --viewer-version 0.1.0   # link the files of the desktop viewer 0.1.0
```

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
| `zh/index.html`, `zh/language.html`, `zh/tools.html`, `zh/packages.html`, `zh/assets/` | Chinese pages |
| `en/index.html`, `en/language.html`, `en/tools.html`, `en/packages.html`, `en/assets/` | English pages |
| `index.html` | The root of the site: it leads to `zh/` when the browser's first language is Chinese, and to `en/` otherwise or without scripts ([Addresses](#addresses)). |
| `language.html`, `tools.html`, `packages.html`, `rnl-*-standalone.html` | The old addresses of the Chinese pages, which were at the root until 2026-10-18 ([Addresses](#addresses)): each leads to the same page under `zh/`. |
| `gallery/` | The gallery's data, for both languages: `index.js`, the groups, and one script per group, such as `standard-0.2.js`. Not written by `build.py`: `pages.yml` writes it with `node site/gallery/build.mjs`, after building the web viewer and so the core. |
| `zh/spec/<version>/`, `en/spec/<version>/`, and `zh/spec/`, `en/spec/` | The language standard: for each version in `spec/`, one page per language with its PDF and Markdown downloads, and next to it the pages that compare it with each earlier version, such as `compare-0.2.html`; `zh/spec/` and `en/spec/` lead to the newest version, which the top bar links to. The old addresses of the Chinese pages of 0.2, `spec/0.2/` and `spec/`, lead to them, and the downloads are copied there. Not written by `build.py`: [`pages.yml`](../.github/workflows/pages.yml) adds them with `python scripts/spec/build.py --site site/dist` ([decision 0007](../docs/decisions/0007-publishing-the-standard.md)). |
| `viewer/` | The web viewer with its samples, one copy for both languages, its interface in English. Not written by `build.py`: `pages.yml` builds it with `node apps/web/build.mjs --samples` and copies `apps/web/dist/` here. |
| `zh/rnl-intro-standalone.html`, `zh/rnl-language-standalone.html`, `zh/rnl-tools-standalone.html`, and the same under `en/` | Single-file pages with the styles and scripts inlined; their links to the other pages, the languages and the web viewer point to the published site |

[`pages.yml`](../.github/workflows/pages.yml) publishes `site/dist/` to GitHub Pages, at <https://rich-netlist.github.io/rnl/>, which leads to the Chinese pages at <https://rich-netlist.github.io/rnl/zh/> or the English ones at <https://rich-netlist.github.io/rnl/en/> by the browser's language, whenever `site/`, `spec/` or `scripts/spec/` changes on `main`, or what the web viewer is built from, the package files in `libraries/` among it; after a revision of the standard is released; and after a release of the desktop viewer. The web viewer is at <https://rich-netlist.github.io/rnl/viewer/>. The language switch keeps the reader on the same section.

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
| `src/assets/gallery.js` | The gallery: loading the data, the groups, search, the packages and their statements, the symbol cards and the details of a symbol, what each package is for |
| `src/assets/gallery.css` | Styles of the gallery, on top of `intro.css`, with the colours of the core's drawings mapped to the theme |
| `gallery/build.mjs` | Builds the gallery's data from `libraries/`, with the symbols drawn by the core as WebAssembly |
| `gallery/package-file.mjs` | Reads a package file: its header record, its comments, and its statements by kind, with the pins and labels of the symbols |
| `gallery/package-file.test.mjs` | Tests of `package-file.mjs`, and that every package file in `libraries/` reads as a package |
| `src/assets/intro.css` | Styles of all pages: light and dark palettes, narrow-screen layout, no animation under `prefers-reduced-motion`, fonts per language |
| `src/assets/schem.js` | Small schematic renderer in RNL coordinates (x right, y up): self-drawn textbook symbols with the standard package's pin positions, the eight orientation codes, pin-based wiring, animated layout transitions, annotation layer |
| `src/assets/rnl-code.js` | Display-only highlighting of SPICE lines, `@RNL` records (keywords, aliases, references, free text) and whole package files, and code-line/drawing cross-highlighting |
| `src/assets/scenes.js` | Circuits, layouts, wiring, annotations, and example files for the introduction's demos |
| `src/assets/common.js` | Shared by all pages: theme, navigation, tabs, segmented controls, highlighting of static snippets |
| `src/assets/intro.js` | Introduction interactions, including the 46-keyword reference |
| `src/assets/tour.js` | Language tour interactions |
| `build.py` | Builds every page in both languages, the single-file pages (the gallery has none), and the optional zip; checks the language pairs and the release blocks, and finds the desktop viewer's newest release |

The symbols are drawn for this page and contain no Analog Canvas material. Like other paths outside `spec/`, `docs/`, and the data directories, this directory is licensed under [MIT](../LICENSE).
