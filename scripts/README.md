# scripts

Maintenance scripts, added only when a concrete workflow needs them.

| Script | Content |
| --- | --- |
| [`fonts.py`](fonts.py) | The fonts shipped with the core, Noto Sans SC Regular and Bold ([decision 0007](../docs/decisions/0007-fonts-shipped-with-the-core.md)). `fetch` downloads them from the noto-cjk repository at a pinned tag into `target/fonts/` and checks their SHA-256 sums; `metrics` writes from them what the renderer lays out text with, [`crates/rnl-render/fonts/noto-sans-sc.metrics`](../crates/rnl-render/README.md#fonts), and `metrics --check` fails when that file is not what the pinned fonts give. The script documents the file's layout. |

`fonts.py` needs Python 3.8 or later; `fetch` needs nothing else, `metrics` needs [fontTools](https://github.com/fonttools/fonttools) (`pip install fonttools`). The commands fetch the fonts first when they are not in `target/fonts/` yet.

See the [roadmap](../docs/roadmap.md) for planned work.
