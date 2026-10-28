# scripts

Maintenance scripts, added only when a concrete workflow needs them.

| Path | Purpose |
| --- | --- |
| [`spec/`](spec/README.md) | Builds the published forms of the language standard from its chapters and checks that they agree ([decision 0007](../docs/decisions/0007-publishing-the-standard.md)). |
| [`i18n/`](i18n/status.py) | Reports what there is to translate in the project and how far each language is, from [`i18n/surfaces.json`](../i18n/README.md); `check` fails only on a translation in a malformed format, and `stamp` records the source a translated document was made from ([decision 0015](../docs/decisions/0015-english-is-the-only-core-language.md)). `migrate_site.py` moves translations written beside the English in `site/src/` into the catalogs of `i18n/<lang>/site/`. |
| [`site_i18n.py`](site_i18n.py) | The locale registry, [`i18n/languages.json`](../i18n/languages.json), and the translation groups of the site's sources, shared by the site's build and the standard's pages. |
| [`skill/`](skill/build.py) | Builds the generated files of the RNL skill package in [`skills/`](../skills/README.md), checks that it is whole and that every netlist in it reads with `rnl check`, and writes its downloads and `llms.txt` into the built site. |
| [`fonts.py`](fonts.py) | The fonts shipped with the core, Noto Sans SC Regular and Bold ([decision 0009](../docs/decisions/0009-fonts-shipped-with-the-core.md)). `fetch` downloads them from the noto-cjk repository at a pinned tag into `target/fonts/` and checks their SHA-256 sums; `metrics` writes from them what the renderer lays out text with, [`crates/rnl-render/fonts/noto-sans-sc.metrics`](../crates/rnl-render/README.md#fonts), and `metrics --check` fails when that file is not what the pinned fonts give. The script documents the file's layout. `web` writes the web fonts of the viewers' page into `apps/viewer-ui/fonts/`, which git ignores: WOFF2 files split by Unicode ranges, `fonts.css` and the licence ([Fonts](../apps/viewer-ui/README.md#fonts)). |
| [`packages.py`](packages.py) | Downloads the single `packages.lock.json` into `.rnl-packages/`, verifies SHA-256, checks the cache and generates offline browser catalogs. Works from every worktree. The SKY130 generator now lives in [rnl-packages](https://github.com/rich-netlist/rnl-packages/tree/main/scripts/pdk). |

`fonts.py` needs Python 3.8 or later; `fetch` needs nothing else, `metrics` needs [fontTools](https://github.com/fonttools/fonttools) and `web` brotli too (`pip install fonttools brotli`). The commands fetch the fonts first when they are not in `target/fonts/` yet.

See the [roadmap](../docs/roadmap.md) for planned work.
