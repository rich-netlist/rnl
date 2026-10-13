// The Document of rnl-view (crates/rnl-view/README.md) as TypeScript declarations, generated from
// its Rust types by ts-rs. Do not edit: after changing the types, run
//   RNL_BLESS=1 cargo test -p rnl-view types
// The Document is plain JSON; these are its shapes, with nothing to load at run time.

/**
 * What a viewer shows of a document, as [`crate::build`] gives it.
 */
export type Document = {
  /**
   * The entry as the frontend named it, for building again.
   */
  path: string,
  /**
   * Its file name, for the title bar.
   */
  name: string,
  /**
   * "spice" or "spectre".
   */
  dialect: string,
  levels: Array<Level>,
  diagnostics: Array<Diagnostic>,
  /**
   * Why the document could not be interpreted at all, such as an unsupported language version.
   */
  failed: string | null,
  /**
   * The host files: the opened file first, then the files it includes.
   */
  files: Array<SourceFile>,
  /**
   * The instances and nets of every level.
   */
  objects: Array<Object>,
  /**
   * The places in the files that name an object.
   */
  spans: Array<Span>,
  /**
   * The files the core asked for by path and did not get, such as an include or a package
   * file named by `path=` that is not in the workspace: each once, as the provider was asked
   * for it, in the order first asked. A frontend that can read them adds them to the
   * workspace and builds again. The bundled packages are never missing, and neither is a
   * package found by name: that comes only from the provider's package directories (16.2).
   */
  missing: Array<string>,
};

/**
 * A drawing level. Its SVG and its hits are in pixels with y down; its primitives are in scene
 * units with y up, `scale` pixels each.
 */
export type Level = {
  /**
   * The host level drawn, as `Object::level` names it.
   */
  host: number,
  title: string,
  /**
   * "file" or "subckt".
   */
  kind: string,
  /**
   * The drawing as an SVG document, written from the primitives; absent unless
   * `BuildOptions::svg` asks for it, as it does by default.
   */
  svg?: string,
  /**
   * The drawing as primitives in drawing order (decision 0006); absent unless
   * `BuildOptions::primitives` asks for them.
   */
  primitives?: Array<Primitive>,
  /**
   * Pixels per scene unit in the SVG and the hits, given with the primitives: a point (x, y) of
   * a primitive is at (x·scale, −y·scale) in them.
   */
  scale?: number,
  /**
   * Nothing on the level can be drawn: no object has a position yet.
   */
  empty: boolean,
  /**
   * What the level lacks without a solver; see [`crate::level_status`].
   */
  status: string | null,
  /**
   * What can be clicked on the drawing, in SVG pixels.
   */
  hits: Array<Hit>,
  /**
   * The subcircuit instances of the level, in netlist order: the hierarchy below it.
   */
  children: Array<Child>,
};

/**
 * Something on a drawing that stands for objects.
 */
export type Hit = {
  /**
   * "box": a symbol, given by its top-left and bottom-right corners; "pin": a pin, one point;
   * "wire": a wire, its vertices.
   */
  kind: string,
  /**
   * In SVG pixels, y down.
   */
  points: Array<[number, number]>,
  /**
   * The objects it stands for: an instance (several for a bundle), or nets.
   */
  objects: Array<number>,
  /**
   * The `data-id` of the SVG groups that draw it, and the owner of its primitives.
   */
  id: string | null,
};

/**
 * A subcircuit instance, as a branch of the hierarchy.
 */
export type Child = {
  instance: string,
  /**
   * The subcircuit it instantiates, as written.
   */
  master: string,
  /**
   * The level that draws the subcircuit, an index into `Document::levels`; None when the
   * netlist does not define it.
   */
  level: number | null,
};

export type SourceFile = {
  /**
   * The file as the runtime knows it: relative to the opened file's directory when it is in it
   * or below, otherwise as the provider names it. With a rooted or absolute entry, the entry's
   * directory joined with this path (`rnl_runtime::source::join`) is the file in the
   * workspace. With a relative entry in a directory, a file outside that directory keeps a
   * relative name, which can be that of a file inside it; the Document then holds only one of
   * the two.
   */
  path: string,
  /**
   * The file as shown: relative to the opened file's directory when it is inside it.
   */
  name: string,
  text: string,
  /**
   * Where each record is: first line, first column, last line, last column (1-based, inclusive).
   */
  records: Array<[number, number, number, number]>,
  /**
   * The statements, or parts of them, that do not take effect: first line, first column, last
   * line, last column, and 0 when they failed or 1 when a conflict overrode them (7.5).
   */
  failed: Array<[number, number, number, number, number]>,
};

export type Object = {
  /**
   * "instance" or "net".
   */
  kind: string,
  name: string,
  /**
   * The host level it belongs to, and that level's title.
   */
  level: number,
  level_title: string,
  /**
   * An instance's statement: file index, first and last line.
   */
  lines: [number, number, number] | null,
  /**
   * For a subcircuit instance, the level that draws its subcircuit, an index into
   * `Document::levels`.
   */
  opens: number | null,
};

export type Span = {
  object: number,
  file: number,
  line: number,
  /**
   * 1-based, in characters.
   */
  col: number,
  len: number,
  /**
   * "name": an instance's name in its statement; "node": a net in an instance's statement;
   * "port": a net in a subcircuit's header; "ref": a reference in a record.
   */
  kind: string,
};

export type Diagnostic = {
  /**
   * "error", "warning" or "hint".
   */
  severity: string,
  /**
   * The file as shown: relative to the opened file's directory when it is inside it.
   */
  file: string,
  /**
   * The file as the runtime knows it.
   */
  full_path: string,
  /**
   * The index of the file in `Document::files`, when it is one of them.
   */
  file_index: number | null,
  line: number,
  /**
   * 0 when the diagnostic is about the whole line.
   */
  col: number,
  message: string,
  /**
   * Expansion, rule or package history, such as "applying $GND_STUB (a.sp:12:3)".
   */
  origin: string,
  /**
   * The upstream cause, as "file:line:col: message".
   */
  cause: string | null,
};

/**
 * One thing drawn on a level: a shape or a text, with the object it draws and its place in the
 * drawing order (11.6). A level lists its primitives in drawing order already.
 */
export type Primitive = {
  /**
   * The `data-id` of the SVG group that draws it; empty for a shape written in the scene and
   * for a flyline. A symbol's drawing, its pin names included, has its item, such as "item-3",
   * and a wire's drawing its wire, such as "wire-0", as `Hit::id` names them. Any other text
   * has the name the diagnostics give it, such as "#R1::name" for an instance's name or
   * "a text" for a text written in the scene: several texts can share it, and no hit has it.
   */
  owner: string,
  layer: Layer,
  /**
   * Its place within the layer, compared element by element (11.6).
   */
  order: Array<number>,
  /**
   * x0, y0, x1, y1: what it covers, in scene units; for a text, each run from its face's
   * ascender to its descender, leaning with it when italic, and the bars.
   */
  bbox: [number, number, number, number],
} & Shape;

/**
 * The layers, drawn bottom first (11.6).
 */
export type Layer = "bottom" | "middle" | "top";

/**
 * What a primitive draws, by its `kind`, with points as x and y in scene units, y up: "path",
 * lines, arcs and curves (9.2.5), and arrow heads, which are filled triangles; "rect", with its
 * sides along the axes, by its lowest and highest corners; "circle"; "ellipse", with its axes
 * along x and y; "text"; "flyline", a line a viewer may show for a connection that is not drawn
 * yet (13.7), not a wire, by its two ends.
 *
 * A shape is filled first and then stroked along its outline with each of its `strokes`, first to
 * last; with no strokes it has no outline. The outline, and so its dashes, starts where a path or
 * a flyline starts; at a rectangle's corner of lowest x and highest y, along the side of highest
 * y first; and at the point of a circle or an ellipse of highest x. It goes round a rectangle,
 * circle or ellipse clockwise, as SVG and Canvas draw them on a screen with y down.
 *
 * A shape of no size is drawn as SVG draws it, which Canvas 2D does not always do: a rectangle
 * with no width or no height, and a circle or an ellipse with a radius of 0, draw nothing, filled
 * or stroked; a part of a path that stays at one point, such as a wire from a point to itself,
 * draws with each stroke only its caps, a dot as wide as the stroke with round caps, a square with
 * square caps and nothing with butt caps.
 */
export type Shape =
  | { "kind": "path", segments: Array<Segment>, strokes: Array<Stroke>, fill: Color | null }
  | { "kind": "rect", min: [number, number], max: [number, number], strokes: Array<Stroke>, fill: Color | null }
  | { "kind": "circle", center: [number, number], r: number, strokes: Array<Stroke>, fill: Color | null }
  | { "kind": "ellipse", center: [number, number], rx: number, ry: number, strokes: Array<Stroke>, fill: Color | null }
  | { "kind": "text" } & Text
  | { "kind": "flyline", points: [[number, number], [number, number]], strokes: Array<Stroke> };

/**
 * A piece of a path, by its `kind`: "move" starts a new part of the path at `to`, as every path
 * starts; "line" goes straight to `to`; "arc" is a circular arc of radius `r` to `to`, the longer
 * way round when `large`, counter-clockwise (y up) when `ccw`; "cubic" is a cubic Bézier curve
 * to `to` with the control points `c1` and `c2`; "close" goes back to where the part started.
 */
export type Segment =
  | { "kind": "move", to: [number, number] }
  | { "kind": "line", to: [number, number] }
  | { "kind": "arc", r: number, large: boolean, ccw: boolean, to: [number, number] }
  | { "kind": "cubic", c1: [number, number], c2: [number, number], to: [number, number] }
  | { "kind": "close" };

/**
 * A colour as the language writes it (9.3), which is as CSS does: `#` and 3, 4, 6 or 8
 * hexadecimal digits of either case, for red, green and blue and then, with 4 or 8, the opacity.
 * So "#1f4fd81a" is a faint blue; a reader that takes the opacity first, as Qt's and Android's
 * do, has to move it.
 */
export type Color = string;

/**
 * One stroke along a shape's outline, a line style (9.3) as the renderer resolved it: a frontend
 * draws it with these and nothing else. A "solid", "dashed", "dotted" or "dash-dot" line is one
 * stroke, the last three with their dashes; a "bold" line, one stroke twice as wide; a "double"
 * line, two strokes with butt caps and miter joins, one in the line's colour three times as wide
 * as each of its two lines and then one in the colour between them as wide as the gap; "none",
 * no stroke.
 */
export type Stroke = {
  color: Color,
  /**
   * The width in the stroke's `unit`, centred on the outline.
   */
  width: number,
  unit: Unit,
  /**
   * The lengths of the dashes and of the gaps between them, in turn, in the stroke's `unit`,
   * repeated along the outline; empty for an unbroken line.
   */
  dashes: Array<number>,
  /**
   * How far into the dashes the outline starts, in the stroke's `unit`: with 0, the first dash
   * starts where the outline starts.
   */
  dash_offset: number,
  cap: Cap,
  join: Join,
};

/**
 * The unit of a stroke's width and dashes: "scene", scene units, which zoom with the drawing;
 * "pixel", pixels of the screen, the same at every zoom. The hairline, a line 0 wide in the style
 * (9.3), is 1.2 pixels wide and a flyline 1 pixel. The SVG is the drawing at the level's `scale`,
 * where a pixel is one of the SVG's own; no dash or gap is shorter than a pixel there.
 */
export type Unit = "scene" | "pixel";

/**
 * The ends of an open line and of its dashes (9.3).
 */
export type Cap = "butt" | "round" | "square";

/**
 * The corners of a line (9.3). A miter that would be more than 4 times as long as the line is wide
 * is drawn as a bevel, as SVG does; Canvas's limit is 10 unless it is set.
 */
export type Join = "miter" | "round" | "bevel";

/**
 * A text (9.2.5, 13.6) as the core laid it out, with the metrics of the fonts shipped with it,
 * Noto Sans SC Regular and Bold (decision 0007). A frontend draws its `runs` and its `bars` and
 * nothing else, so it needs no alignment, baseline or measuring of its own; the other fields say
 * what the text is and how it was placed. Its glyphs read upright whatever turns it, or from
 * bottom to top when it is vertical.
 */
export type Text = {
  anchor: [number, number],
  /**
   * The lines, first to last, each a list of runs of the markup.
   */
  lines: Array<Array<Run>>,
  /**
   * The text height, the em of the font, in scene units.
   */
  size: number,
  color: Color,
  /**
   * The font family the style names, a hint (9.3): texts are laid out and drawn in the shipped
   * family.
   */
  font: string,
  bold: boolean,
  italic: boolean,
  /**
   * Where the anchor is along the lines, after turning (9.2.5).
   */
  halign: HAlign,
  /**
   * Where the anchor is across the lines, after turning.
   */
  valign: VAlign,
  /**
   * Turned a quarter counter-clockwise: the lines read from bottom to top, and each run is drawn
   * turned a quarter counter-clockwise about its `at`.
   */
  vertical: boolean,
  /**
   * The runs as laid out, drawn in the text's colour.
   */
  runs: Array<TextRun>,
  /**
   * The bars of its fractions and the lines over its overlined runs.
   */
  bars: Array<Bar>,
};

/**
 * A run of a laid-out text: `text` in one face and size, drawn from `at`, the start of its
 * baseline. Its glyphs follow each other by their advances in the face, with no kerning,
 * ligatures or other shaping, so that the run is `width` long; in CSS, `font-kerning: none` and
 * `font-variant-ligatures: none`. Its spaces are drawn as they are.
 */
export type TextRun = {
  text: string,
  /**
   * The start of its baseline, in scene units.
   */
  at: [number, number],
  /**
   * The font size, the em, in scene units.
   */
  size: number,
  /**
   * The sum of its characters' advances in its face, in scene units. A character the fonts do
   * not map is 1 em wide, as the box they draw for it.
   */
  width: number,
  /**
   * In Noto Sans SC Bold, else Regular.
   */
  bold: boolean,
  /**
   * Leaning, since the fonts have no italic face: a point of a glyph moves forward along the
   * baseline by 0.25 times its height above it, about 14°. Its advances are the upright face's.
   */
  italic: boolean,
};

/**
 * A straight line of a text, the bar of a fraction or the line over an overlined run (13.6),
 * between its two `points`, drawn with its stroke: in the text's colour, in scene units, with butt
 * caps.
 */
export type Bar = { points: [[number, number], [number, number]], stroke: Stroke };

/**
 * Where a text's anchor is along its lines: at their start, middle or end.
 */
export type HAlign = "start" | "middle" | "end";

/**
 * Where a text's anchor is across its lines: on the first line's baseline; at the top, the first
 * line's ascender, 1.16 times the size above its baseline; at the bottom, the last line's
 * descender, 0.288 times the size below its baseline; or in the middle, halfway between the two.
 * The lines are the fonts' line height apart, 1.448 times the size.
 */
export type VAlign = "baseline" | "top" | "middle" | "bottom";

/**
 * A piece of a line of text in one style (13.6), by its `kind`: "text", or "frac" for
 * `\frac{…}{…}`, the numerator over the denominator. Laid out, the numerator and the denominator
 * are at 0.8 of the fraction's size and centred on it, with their baselines 0.72 of the size above
 * its baseline and 0.42 below it; the fraction is as wide as the wider of them and 0.15 of the
 * size, and its bar is 0.3 of the size above the baseline and 0.06 of it short of each end. A
 * fraction's bar and an overline are 0.06 of their size wide, but at least 0.8 pixels at the
 * level's `scale`; an overline is 0.88 of its run's size above the run's baseline, at the top of
 * the em box.
 */
export type Run =
  | { "kind": "text", text: string, style: RunStyle }
  | { "kind": "frac", num: Array<Run>, den: Array<Run>, style: RunStyle };

/**
 * The style the markup gives a run (13.6).
 */
export type RunStyle = {
  /**
   * How far the run is raised: each `^` it is in adds 1 and each `_` takes 1 away, so the z of
   * `x^{y^{z}}` is at 2. Laid out, each level is 0.7 of the size of the level next to it towards
   * 0, with its baseline 0.35 of that size higher, or 0.15 of it lower; in a fraction, the levels
   * count from the fraction's.
   */
  level: number,
  bold: boolean,
  italic: boolean,
  overline: boolean,
};
