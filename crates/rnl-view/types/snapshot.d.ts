// The snapshot of the editor interface (crates/rnl-view/src/snapshot.rs) as TypeScript
// declarations, generated from its Rust types by ts-rs. Do not edit: after changing the types, run
//   RNL_BLESS=1 cargo test -p rnl-view types
// The snapshot is plain JSON; these are its shapes, with nothing to load at run time.

import type { Diagnostic, Document, Run } from "./document.js";

/**
 * A document as the editor interface gives it: its files, packages and diagnostics, and each
 * drawing level's host circuit and model, as [`Parts`] ask.
 */
export type Snapshot = {
  /**
   * "rnl-snapshot/1".
   */
  schema: string,
  /**
   * The version of the runtime that made it.
   */
  runtime: string,
  /**
   * The language version the main file declares, or the one a file without it is read as (16.1).
   */
  language: string,
  /**
   * The fingerprint of what it is made from (3.7), 32 hexadecimal digits: the texts of the host
   * files and of the packages read, and the build's options. The same for the same texts and
   * options, whatever their line endings; new after any change to a text, a space included.
   */
  revision: string,
  /**
   * "spice" or "spectre".
   */
  dialect: string,
  /**
   * The packages in effect, by name and version, in the order they are brought in: those of
   * the standard package first, the packages it references before it (16.2, 16.3).
   */
  packages: Array<PackageUse>,
  /**
   * The host files, as in the Document: the opened file first.
   */
  files: Array<FileRef>,
  /**
   * The drawing levels, as in the Document.
   */
  levels: Array<LevelSnapshot>,
  diagnostics: Array<Diagnostic>,
  /**
   * Why the document could not be interpreted at all.
   */
  failed: string | null,
  /**
   * The Document, when `parts.drawing` asks for it.
   */
  drawing?: Document,
};

export type PackageUse = { name: string, version: string };

export type FileRef = {
  /**
   * As the Document's `SourceFile::path`.
   */
  path: string,
  /**
   * As shown.
   */
  name: string,
  /**
   * The format the file was read with, as the Document's `SourceFile::format`: only when the
   * netlist's files are of more than one format, or the request set a format for some.
   */
  format?: string,
};

/**
 * A drawing level.
 */
export type LevelSnapshot = {
  /**
   * `subckt:NAME` for a subcircuit, `file:PATH` for the file level, PATH as in `files`.
   */
  id: string,
  /**
   * "file" or "subckt".
   */
  kind: string,
  /**
   * The subcircuit's name, or the file's.
   */
  name: string,
  /**
   * The file it is defined in, an index into `files`.
   */
  file: number | null,
  /**
   * The line of a subcircuit's header; 0 for a file level.
   */
  line: number,
  /**
   * What the level lacks without a solver, as the Document's `Level::status`.
   */
  status: string | null,
  host?: HostLevel,
  model?: LevelModel,
};

/**
 * The host circuit of a level, read only. Names are written as on the host's lines; a net
 * written several ways goes by its first.
 */
export type HostLevel = {
  /**
   * The subcircuit's ports in order, as net ids.
   */
  ports: Array<string>,
  /**
   * The instances in netlist order.
   */
  instances: Array<HostInstance>,
  /**
   * The nets: the ports first, then as the instances name them.
   */
  nets: Array<HostNet>,
};

export type HostInstance = {
  /**
   * `#M1`.
   */
  id: string,
  /**
   * An instance of a subcircuit, defined in the netlist or not.
   */
  subckt: boolean,
  /**
   * The name in the model position: a model, a subcircuit or a Spectre master.
   */
  master: string | null,
  terminals: Array<HostTerminal>,
};

export type HostTerminal = {
  /**
   * From 1, in the order of the line's nodes.
   */
  n: number,
  /**
   * From DEVICE, the subcircuit's ports or the adapter's defaults (6.4, 6.12).
   */
  name: string | null,
  /**
   * `&vout`.
   */
  net: string,
};

export type HostNet = {
  /**
   * `&vout`, `&"vdd!"`.
   */
  id: string,
  global: boolean,
  port: boolean,
};

/**
 * What the RNL makes of a level. Coordinates are scene units, y up.
 */
export type LevelModel = {
  /**
   * The symbols the objects are drawn with, and the level's own.
   */
  symbols: Array<SymbolRef>,
  /**
   * A subcircuit's own symbol, `#` (9.2.6), by its id in `symbols`: what its instances are drawn
   * with when they are given no other. Null for a file level, and for a subcircuit without one.
   */
  symbol: Slot<string>,
  /**
   * The instances, bundles and usages, drawn or not.
   */
  objects: Array<ObjectView>,
  /**
   * The named points.
   */
  points: Array<PointView>,
  /**
   * The wires that have their vertices.
   */
  wires: Array<WireView>,
  /**
   * The texts with an anchor and something to show.
   */
  texts: Array<TextView>,
  /**
   * The graphics of the scene (13.4) that have their points, in the order written.
   */
  shapes: Array<ShapeView>,
};

/**
 * A symbol, by an id unique in the level's `symbols`: its name, or its name and where it is
 * defined after `@` when two of the level's symbols share the name.
 */
export type SymbolRef = {
  id: string,
  /**
   * `$NMOS`, or `%ota5t` for a symbol defined in a level.
   */
  name: string,
  /**
   * The package that defines it.
   */
  package: string | null,
  /**
   * The level that defines it, by its id.
   */
  level: string | null,
  /**
   * The viewer's own stand-in box (9.2.6).
   */
  default: boolean,
  /**
   * For a symbol the document defines, such as a block's (9.2.6), what it is made of, in its
   * own coordinates; for a package's symbol, null: its name says it.
   */
  definition: SymbolDefinition | null,
};

/**
 * A symbol's box, pins and graphics, in its own coordinates, y up.
 */
export type SymbolDefinition = {
  /**
   * x1, y1, x2, y2: the box the symbol writes, or the extent of its graphics.
   */
  box: [number, number, number, number],
  /**
   * The symbol writes its box.
   */
  box_written: boolean,
  pins: Array<SymbolPin>,
  /**
   * Its graphics, texts left out, in the order written.
   */
  prims: Array<SymbolPrim>,
};

export type SymbolPin = {
  /**
   * The terminals at the pin, by number or name as the symbol writes them.
   */
  terminals: Array<string>,
  at: [number, number],
  /**
   * The exit direction in degrees: 0 right, 90 up, 180 left, 270 down.
   */
  dir: number | null,
  /**
   * The pin's name for display (9.2.4): as written, and as lines of runs.
   */
  label: PinLabel | null,
};

export type PinLabel = { markup: string, lines: Array<Array<Run>> };

/**
 * A graphic of a symbol (9.2.5).
 */
export type SymbolPrim = {
  /**
   * "line", "rectangle", "circle", "ellipse", "arc" or "bezier".
   */
  shape: string,
  /**
   * The centre of a rectangle, circle or ellipse with its size, else its points.
   */
  points: Array<[number, number]>,
  /**
   * Its parameters: width, height, radius, fill, arrow.
   */
  params: Record<string, string | number>,
  /**
   * The style properties it writes.
   */
  style: Record<string, string | number | boolean>,
};

/**
 * A value and the statement it comes from (`source`, a key of `sources`; null for now).
 */
export type Slot<T> = { value: T | null, source: string | null };

export type ObjectView = {
  /**
   * Unique in the level: `#M1`, `%gnd1`, or `@file:line:col` where the statement that made an
   * object without a name of its own is, with `#n` after it when it made several.
   */
  id: string,
  /**
   * "instance", "bundle" or "usage".
   */
  kind: string,
  /**
   * The host instances it draws: one for an instance, the members of a bundle, none for a usage.
   */
  host: Array<string>,
  /**
   * A usage's definition, such as `$GROUND`.
   */
  definition: string | null,
  /**
   * A usage's roles, each with the id of what it is attached to, or a point; a set role (9.4)
   * with the list of its members, each an id or a point, nested sets followed down, empty when the
   * role is left empty.
   */
  attach: Record<string, string | [number, number] | (string | [number, number])[]> | null,
  /**
   * The id of its symbol in `symbols`.
   */
  symbol: Slot<string>,
  /**
   * Its registration point; null when it has none yet.
   */
  at: Slot<[number, number]>,
  /**
   * "R0", "R90", "R180", "R270", "MX", "MY", "MXR90" or "MYR90".
   */
  orient: Slot<string>,
  /**
   * The symbol's pins where they are drawn, when it has a symbol and a position.
   */
  pins: Array<PinView>,
};

export type PinView = {
  /**
   * An instance's terminals by number, `#M1:1`; a usage's roles, `p`.
   */
  terminals: Array<string>,
  at: [number, number],
  /**
   * The exit direction after the orientation, in degrees: 0 right, 90 up, 180 left, 270 down.
   */
  dir: number | null,
  /**
   * The nets at the pin, as ids.
   */
  nets: Array<string>,
};

export type PointView = {
  /**
   * `%j1`.
   */
  id: string,
  at: Slot<[number, number]>,
  /**
   * A junction dot is drawn at it.
   */
  junction: boolean,
};

export type WireView = {
  id: string,
  nets: Array<string>,
  /**
   * The vertices as the statement writes them.
   */
  via: Array<Vertex>,
  /**
   * The vertices after the routing mode.
   */
  points: Array<[number, number]>,
  mode: string,
  /**
   * The style properties the document's records give it; a package's, such as the standard
   * package's defaults, are left out.
   */
  style: Record<string, string | number | boolean>,
};

/**
 * A vertex of a wire.
 */
export type Vertex =
  | { terminal: string }
  | { point: string }
  | { usage: string, role: string }
  | { at: [number, number] };

export type TextView = {
  /**
   * `#M1::name` for a label with a key; else as an object's.
   */
  id: string,
  /**
   * The id of the object it is attached to; null for a text at a point.
   */
  attach: string | null,
  /**
   * The key of a label, such as "name".
   */
  key: string | null,
  /**
   * The text as written, before `${…}` is read (13.6).
   */
  markup: Slot<string>,
  /**
   * The document's records write it, or a part of it; false for a label its symbol alone gives,
   * such as the formula of a block.
   */
  written: boolean,
  /**
   * What it shows, as the Document's texts give it.
   */
  lines: Array<Array<Run>>,
  anchor: [number, number],
  halign: string,
  valign: string,
  rotate: number,
  /**
   * Its size, from the document or a package, when one gives it.
   */
  size: number | null,
  /**
   * The style properties the document's records give it, such as `textcolor`, `bold` and
   * `italic`; a package's are left out.
   */
  style: Record<string, string | number | boolean>,
};

/**
 * A graphic of the scene (13.4), as an `add-shape` op writes one.
 */
export type ShapeView = {
  /**
   * `%box1` when the level names it; else where the statement that made it is, as an object's.
   */
  id: string,
  /**
   * "line", "rectangle", "circle", "ellipse", "arc" or "bezier".
   */
  shape: string,
  /**
   * The centre of a rectangle, circle or ellipse with its size, else its points; a point given
   * by a reference, such as `%j1`, where it is.
   */
  points: Array<[number, number]>,
  /**
   * Its parameters: width, height, radius, fill, arrow.
   */
  params: Record<string, string | number>,
  /**
   * The style properties the document's records give it; a package's are left out.
   */
  style: Record<string, string | number | boolean>,
  /**
   * "bottom", "middle" or "top".
   */
  layer: string,
};
