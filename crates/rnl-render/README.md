# rnl-render

The shared renderer. It turns the effective model of one drawing level into basic primitives (paths, rectangles, circles, ellipses, texts, and the dashed lines of `ROUTE`), each tagged with the object it draws, its layer and its bounding box. `svg_document` writes them as an SVG file; other back ends can consume the same primitives.

It draws what the model says and nothing more: objects without coordinates are not drawn, and wires are drawn only where their vertices are known. Colours, line width, font metrics and pin labels are this implementation's defaults (principle 2.7, 9.3).

So is how a line style looks (9.3). The renderer resolves each into strokes that any back end draws with a colour, a width, dashes, a cap and a join, and `svg_document` writes them as they are. The hairline is 1.2 pixels wide at every zoom; the dashes of "dashed", "dotted" and "dash-dot" are multiples of the width, but at least a pixel at the SVG's scale; "bold" is twice as wide; "double", on every shape, is a stroke three times as wide under one in the fill colour as wide as the gap; a flyline is a grey dashed stroke 1 pixel wide.

See the [roadmap](../../docs/roadmap.md) for planned work.
