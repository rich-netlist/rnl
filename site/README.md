# site

Three static pages, in Chinese and English, for circuit designers who have not worked on EDA tooling:

- `index.html`, the introduction: what RNL is, the problem it addresses, the goal, before/after scenarios, the toolchain, the settled principles, the language at a glance, and a few examples.
- `language.html`, the language tour: nine hands-on sections on the parts of the language the introduction does not unpack (standard sentences and Chinese, definitions, the standard package's defaults, sets and selections, wires, text, orientation, priority, the header record).
- `tools.html`, the page of the tools: what the viewers of the first toolkit, 0.1, do; the web viewer, with a few samples to open in it; the desktop viewer for Windows, with the files of its newest release; and their licences.

The site also publishes the web viewer itself, at `viewer/` ([`apps/web`](../apps/web/README.md)), one copy that the pages of both languages link to, with the conformance corpus as its samples; and the pages of the language standard, at `spec/0.2/`. [`pages.yml`](../.github/workflows/pages.yml) adds both to what `build.py` writes.

**Status:** an explainer, not a specification. The RNL snippets follow language draft 0.2 (2026-10-12), which has since been consolidated into the language standard 0.2 in [`spec/`](../spec/README.md); the pages say so and link to it. The snippets have been checked with the RNL interpreter prototype, which lives in a research repository that is not public yet. Layouts in the drawings are placed by hand, not produced by a solver.

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

## Building and viewing

From the repository root, with Python 3 and nothing else (and git, for the tags):

```bash
python site/build.py            # write site/dist/ (ignored by Git)
python site/build.py --check    # check the sources only
python site/build.py --zip rnl-intro.zip   # also pack the built site
python site/build.py --viewer-version 0.1.0   # link the files of the desktop viewer 0.1.0
```

Then open `site/dist/index.html` (Chinese) or `site/dist/en/index.html` (English) in a browser, or serve `site/dist/` with any static server; `language.html` and `tools.html` sit next to each `index.html`. Fonts load from Google Fonts and fall back to system fonts offline.

The web viewer is not written by `build.py`. To look at the pages with it, build it as `pages.yml` does and copy it in, then serve `site/dist/` with a server that sends `.wasm` as `application/wasm` and `.js` as JavaScript, such as Python's; the viewer only runs when served, not from the disk:

```bash
python scripts/fonts.py web
node apps/web/build.mjs --samples
cp -r apps/web/dist site/dist/viewer
python -m http.server --directory site/dist
```

| Output in `site/dist/` | Content |
| --- | --- |
| `index.html`, `language.html`, `tools.html`, `assets/` | Chinese pages, at the site root |
| `en/index.html`, `en/language.html`, `en/tools.html`, `en/assets/` | English pages |
| `spec/0.2/`, `en/spec/0.2/` | The language standard, one page per language with its PDF and Markdown downloads. Not written by `build.py`: [`pages.yml`](../.github/workflows/pages.yml) adds them with `python scripts/spec/build.py --site site/dist` ([decision 0007](../docs/decisions/0007-publishing-the-standard.md)). |
| `viewer/` | The web viewer with its samples, one copy for both languages, its interface in English. Not written by `build.py`: `pages.yml` builds it with `node apps/web/build.mjs --samples` and copies `apps/web/dist/` here. |
| `rnl-intro-standalone.html`, `rnl-language-standalone.html`, `rnl-tools-standalone.html`, and the same under `en/` | Single-file pages with the styles and scripts inlined; their links to the other pages, the languages and the web viewer point to the published site |

[`pages.yml`](../.github/workflows/pages.yml) publishes `site/dist/` to GitHub Pages, at <https://rich-netlist.github.io/rnl/>, whenever `site/`, `spec/` or `scripts/spec/` changes on `main`, or what the web viewer is built from; after a revision of the standard is released; and after a release of the desktop viewer. The web viewer is at <https://rich-netlist.github.io/rnl/viewer/>. The language switch keeps the reader on the same section.

## Files

| Path | Content |
| --- | --- |
| `src/index.html` | Introduction: structure and text |
| `src/language.html` | Language tour: structure and text |
| `src/tools.html` | Page of the tools: structure and text, with the blocks that depend on the desktop viewer's release |
| `src/assets/intro.css` | Styles of all pages: light and dark palettes, narrow-screen layout, no animation under `prefers-reduced-motion`, fonts per language |
| `src/assets/schem.js` | Small schematic renderer in RNL coordinates (x right, y up): self-drawn textbook symbols with the standard package's pin positions, the eight orientation codes, pin-based wiring, animated layout transitions, annotation layer |
| `src/assets/rnl-code.js` | Display-only highlighting of SPICE lines and `@RNL` records (keywords, aliases, references, free text), and code-line/drawing cross-highlighting |
| `src/assets/scenes.js` | Circuits, layouts, wiring, annotations, and example files for the introduction's demos |
| `src/assets/common.js` | Shared by all pages: theme, navigation, tabs, segmented controls, highlighting of static snippets |
| `src/assets/intro.js` | Introduction interactions, including the 46-keyword reference |
| `src/assets/tour.js` | Language tour interactions |
| `build.py` | Builds every page in both languages, the single-file pages, and the optional zip; checks the language pairs and the release blocks, and finds the desktop viewer's newest release |

The symbols are drawn for this page and contain no Analog Canvas material. Like other paths outside `spec/`, `docs/`, and the data directories, this directory is licensed under [MIT](../LICENSE).
