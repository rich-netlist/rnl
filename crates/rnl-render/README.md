# rnl-render

The shared renderer. It turns the effective model of one drawing level into basic primitives (paths, rectangles, circles, ellipses, texts, and the dashed lines of `ROUTE`), each tagged with the object it draws, its layer and its bounding box. `svg_document` writes them as an SVG file; other back ends can consume the same primitives.

It draws what the model says and nothing more: objects without coordinates are not drawn, and wires are drawn only where their vertices are known. Colours, line width and pin labels are this implementation's defaults (principle 2.7, 9.3).

So is how a line style looks (9.3). The renderer resolves each into strokes that any back end draws with a colour, a width, dashes, a cap and a join, and `svg_document` writes them as they are. The hairline is 1.2 pixels wide at every zoom; the dashes of "dashed", "dotted" and "dash-dot" are multiples of the width, but at least a pixel at the SVG's scale; "bold" is twice as wide; "double", on every shape, is a stroke three times as wide under one in the fill colour as wide as the gap; a flyline is a grey dashed stroke 1 pixel wide.

## Fonts

Texts are laid out with the metrics of the fonts shipped with the core, Noto Sans SC Regular and Bold ([decision 0007](../../docs/decisions/0007-fonts-shipped-with-the-core.md)), so that every back end draws them alike. `lay_out` turns a text's lines of markup (13.6) into runs, each a piece of text in one face and size from the start of its baseline with its width, and bars, a fraction's bar and the line over an overlined run, as strokes. A back end draws each run at its place in that face with no kerning or ligatures, and needs no alignment, baseline or measuring of its own. `svg_document` writes each run as a `text` element at its place, with its spaces, held to its width with `textLength`, so that a browser without the fonts still keeps the layout.

- A run is as wide as the sum of its characters' advances in its face; a character the fonts do not map is 1 em wide, as their .notdef box. Bold runs use the bold advances; italic has no face of its own, so an italic run leans by a quarter of the height (about 14°) with the upright advances.
- Lines are the fonts' line height apart, ascender less descender and line gap: 1.448 times the size. The vertical alignments (9.2.5) put the anchor at the first line's baseline, its ascender (top), the last line's descender (bottom), or halfway between those (middle).
- Each level of sub- or superscript is 0.7 of the size of the one next to it towards 0, with its baseline 0.35 of that size higher or 0.15 lower. A fraction's numerator and denominator are at 0.8 of its size with their baselines 0.72 above its own and 0.42 below it, centred over a bar 0.3 above it.
- The font a style names (9.3) is a hint: texts are laid out and drawn in the shipped family.
- A text's bounding box covers its runs, each from its face's ascender to its descender, and its bars.

The metrics are in [`fonts/noto-sans-sc.metrics`](fonts/noto-sans-sc.metrics), read with `include_bytes!`: 8.5 KB, the units per em, the ascender, descender and line gap, the top of the em box, and the advances of the 30,890 code points the fonts map in 850 runs. The fonts are not in the repository; [`scripts/fonts.py`](../../scripts/README.md) downloads the pinned fonts and writes the file from them:

```bash
python scripts/fonts.py metrics           # write it again
python scripts/fonts.py metrics --check   # fail when it is not what the pinned fonts give
```

The file is licensed under the fonts' licence, the [SIL Open Font License 1.1](../../LICENSES/OFL-1.1.txt).

See the [roadmap](../../docs/roadmap.md) for planned work.
