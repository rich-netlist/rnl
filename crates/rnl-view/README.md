# rnl-view

The core interface of [decision 0008](../../docs/decisions/0008-shared-core-per-platform-frontends.md): a workspace in, a Document out. The desktop viewer and `rnl view` call it natively; the [web viewer](../../apps/web/README.md) calls it through WebAssembly.

```rust
pub fn build(entry: &str, files: &dyn Provider, options: &BuildOptions) -> Result<Document, String>
```

- `entry` is the path of the file to open as the frontend names it, `/`-separated (a `\` is read as `/`). It need not exist on disk: an untitled text is an entry in an in-memory provider. `build` fails when the entry cannot be read. Name the files of a workspace by rooted paths, such as `/w/top.sp`, as the desktop names them by absolute ones (see below).
- `files` is the frontend's file access: `FsProvider` from [`rnl-cli`](../rnl-cli/README.md) on the desktop and in the CLI, a `MemoryProvider` of the texts a frontend holds elsewhere. The standard packages in [`libraries/standard`](../../libraries/standard/README.md) are bundled here and layered under it, in the directory `<bundled>`.
- `BuildOptions` holds the dialect, instead of guessing it from the entry, the runtime's options, such as leaving out the standard package at the root (16.3), and the outputs: whether to write each level's SVG (`svg`, on by default) and whether to give its primitives (`primitives`, off by default).

The Document holds, for each drawing level, its drawing, what on it can be clicked, its status without a solver and its subcircuit instances; for the document, the source files with the places of their records and of the statements that do not take effect (7.5), the instances and nets of every level with the places that name them, and the diagnostics. The references in the records come from the runtime (`Interpretation::refs`), so aliases, scopes and subscripts are resolved as the language says. The field names are what the viewers read; `rnl view FILE` prints the Document as JSON.

`path` is the entry as given. A file of the document is named relative to the entry's directory when it is in it or below, and otherwise as the provider names it, so the entry's directory joined with a file's path (`rnl_runtime::source::join`) is the file in the workspace. That needs a rooted or absolute entry: with a relative entry in a directory, such as `w/top.sp`, a file outside it keeps a relative name, such as `x.inc`, which can be that of a file inside it (`w/x.inc`), and the Document then holds only one of the two. The host adapters name the files this way. A `..` above the root stays in a path: `../x.inc` beside an entry at the root, `/top.sp`, is `/../x.inc` and not `/x.inc`. A file system reads it as `/x.inc`; a workspace in memory has it only when the frontend gives it.

`missing` lists the files the core asked for by path and did not get, such as an include or a package file named by `path=` that is not in the workspace, each once, as the provider was asked for it, in the order first asked. A frontend that can read them adds them to the workspace and builds again until nothing new is missing. A package found by name comes only from the provider's package directories (`Provider::package_files`, such as `MemoryProvider::package_dirs`), so it is never missing: a frontend gives its package directories before it builds.

The crate also has what the `rnl` command needs that reads no files: dialect detection, reading a host document, interpreting a netlist or a package file checked on its own, and the status lines of `rnl check`.

The crate does no file I/O, so it builds for `wasm32-unknown-unknown`.

## Drawings

A level's drawing comes in the outputs the frontend asks for. A level has an `svg` field only when the SVG was asked for, and `primitives` and `scale` fields only when the primitives were, so by default the Document is the one the desktop viewer has always read.

- The primitives (decision 0008) are what the renderer draws, in drawing order: paths, rectangles, circles, ellipses, texts and flylines, each with its layer and place in the drawing order (11.6), its bounding box, and its `owner`, the `data-id` of the SVG group that draws it. A symbol's drawing has its item, such as `item-3`, and a wire's its wire, such as `wire-0`, as the hits' `id` names them; any other text has the name the diagnostics give it, such as `#R1::name`, which several texts can share and no hit has; a shape written in the scene and a flyline have none. The styles and the renderer's defaults are resolved, so a frontend draws the primitives without reading the model: fills; strokes, each shape's in turn over its fill, with their colour, width, dashes and dash offset, in scene units or in pixels (the hairline is 1.2 pixels wide at every zoom), cap and join, which is all a canvas needs to draw the line styles as the SVG does (a double line is two strokes, the second narrower in the colour between the lines, and a flyline a grey dashed stroke of 1 pixel; a shape of no size is drawn as SVG draws it, which the types describe, since Canvas 2D draws some of them otherwise); texts laid out by the core with the metrics of the fonts shipped with it ([decision 0009](../../docs/decisions/0009-fonts-shipped-with-the-core.md)): runs, each a piece of text in one face and size from the start of its baseline, with its width, and bars, the bars of fractions and the lines over overlined runs, as strokes; what the fonts do not map in runs of its own, `fallback`, whole grapheme clusters drawn in a font of the system's, whose width is an estimate; with the lines of markup (13.6) they come from, the size, colour, font hint, alignment after turning and direction. A colour is written as in the language and in CSS, `#RGB`, `#RGBA`, `#RRGGBB` or `#RRGGBBAA`, with the opacity last. The primitives are this crate's own serializable types, converted from the renderer's, which stay free to change. In the JSON, a primitive, a path segment and a run each name their `kind`, and a point is an `[x, y]` array.
- The renderer decides how a line style looks (the hairline's width, the dashes, double lines, flylines) and the SVG writer writes the strokes as they are, with the dashes to at most 6 decimals, so that a long dashed line keeps in step with them. The renderer also lays out the texts, and the SVG writer writes each run where it is, in the shipped family with no kerning or ligatures, held to its width with `textLength`; a run the fonts do not map, in the font the browser picks after that family, not held to its estimated width but centred on it. A frontend that draws the runs and bars in Noto Sans SC Regular and Bold the same way, and the runs the fonts do not map in a font of its system's, draws the texts as the SVG does, with no alignment, baseline or measuring of its own; the types say how, and a test holds the SVG writer to what they say. The font a style names is a hint (9.3) that the drawing does not follow.
- The primitives are in scene units, x to the right and y up, as in the model, but for the strokes in pixels, which the SVG draws in its own. The SVG and the hits are in pixels, x to the right and y down: a point (x, y) of a primitive is at (x·scale, −y·scale) in them, with the level's `scale`. The SVG is written from the same primitives, and its view box frames them with a margin.

## Type definitions

[`types/document.d.ts`](types/document.d.ts) declares the Document and everything in it in TypeScript, for frontends and their checks; the page in [`apps/viewer-ui`](../../apps/viewer-ui/README.md) refers to it. It is generated from the Rust types by [ts-rs](https://crates.io/crates/ts-rs), which the types derive in the tests only, and laid out by the test to be read. The test fails when the file is not what the types give; after a change to them that is meant, write it with:

```bash
RNL_BLESS=1 cargo test -p rnl-view types
```

The serde field names are the contract, and the declarations keep them as they are.

See the [roadmap](../../docs/roadmap.md) for planned work.
