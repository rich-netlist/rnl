# scripts

Maintenance scripts, added only when a concrete workflow needs them.

| Path | Purpose |
| --- | --- |
| [`spec/`](spec/README.md) | Builds the published forms of the language standard from its chapters and checks that they agree ([decision 0007](../docs/decisions/0007-publishing-the-standard.md)). |
| [`fonts.py`](fonts.py) | The fonts shipped with the core, Noto Sans SC Regular and Bold ([decision 0009](../docs/decisions/0009-fonts-shipped-with-the-core.md)). `fetch` downloads them from the noto-cjk repository at a pinned tag into `target/fonts/` and checks their SHA-256 sums; `metrics` writes from them what the renderer lays out text with, [`crates/rnl-render/fonts/noto-sans-sc.metrics`](../crates/rnl-render/README.md#fonts), and `metrics --check` fails when that file is not what the pinned fonts give. The script documents the file's layout. `web` writes the web fonts of the viewers' page into `apps/viewer-ui/fonts/`, which git ignores: WOFF2 files split by Unicode ranges, `fonts.css` and the licence ([Fonts](../apps/viewer-ui/README.md#fonts)). |

`fonts.py` needs Python 3.8 or later; `fetch` needs nothing else, `metrics` needs [fontTools](https://github.com/fonttools/fonttools) and `web` brotli too (`pip install fonttools brotli`). The commands fetch the fonts first when they are not in `target/fonts/` yet.

See the [roadmap](../docs/roadmap.md) for planned work.
