# rnl-render

The shared renderer. It turns the effective model of one drawing level into basic primitives (paths, rectangles, circles, ellipses, texts, and the dashed lines of `ROUTE`), each tagged with the object it draws, its layer and its bounding box. `svg_document` writes them as an SVG file; other back ends can consume the same primitives.

It draws what the model says and nothing more: objects without coordinates are not drawn, and wires are drawn only where their vertices are known. Colours, line width, font metrics and pin labels are this implementation's defaults (principle 2.7, 9.3).

See the [roadmap](../../docs/roadmap.md) for planned work.
