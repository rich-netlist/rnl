# 0014: Shared inspection overlay geometry

Status: proposed; implementation under review.

The Viewer previously computed usage frames from hits and role extents. CLI and npm SVG
exports could not reproduce these frames or inspect effective terminal currents.

Keep effective current annotations in the runtime's display model, using the interpreter's
existing precedence. This adds no facts and changes no solver input. `rnl-view` computes
style-free usage frames, role anchors/widths and current arrow points alongside hits. All
frontends receive the same geometry, including native and WebAssembly builds. This fits
the shared-core boundary of decision 0008 without adding a dependency from the renderer
back to the view.

Use one rectangle per usage, including dispersed members and nested usages. Virtual symbols
whose roles are local connections frame only their own symbol. Padding is 12 SVG pixels;
title/role anchors and collision offsets are fixed in drawing coordinates. This deliberately
changes the old screen-space padding on zoom, so exported and interactive geometry agree.
Text advances use the shipped font metrics; SVG textLength keeps badge widths reproducible.
Viewer retains screen-size labels and selection strokes, which are interaction styling.

Device arrows run along oriented pin exits on the exterior segment 6–20 SVG pixels from
the pin: `in` points toward the device, `out` away. Unspecified, directionless or unplaced
terminals have no arrow. Declaration ports have no scene position: draw their arrows at
explicit `$IO_PORT` usages on the same net, with `in` pointing from the tag into the circuit.
Do not invent coordinates or propagate declaration annotations to instance terminals.

Optional SVG export lives in `rnl-view::overlays`, above the standard renderer. Disabled
options return exactly the existing SVG. Enabled overlays use dedicated underlay/overlay
groups and expand the export canvas only. They never add hits or affect scene geometry.

The CLI/npm renderer uses a fixed palette by usage order, dotted frames, translucent fill,
and solid current arrows. A separate style object/file overrides palette, stroke width,
frame dashes and current color. Viewer controls its own colors, visibility and selection;
these preferences and RNL language styles remain independent. No legend is added: usage
titles, optional role labels and terminal metadata identify the overlays.

No standard, standard package, solver behavior or release/tag is changed. CLI/npm ship in
an approved `core-v*` release and Viewer in an approved `viewer-v*` release.
