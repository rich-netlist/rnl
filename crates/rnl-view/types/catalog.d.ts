// The catalog of the packages the core has (crates/rnl-view/src/catalog.rs) as TypeScript
// declarations, generated from its Rust types by ts-rs. Do not edit: after changing the types, run
//   RNL_BLESS=1 cargo test -p rnl-view types
// The catalog is plain JSON; these are its shapes, with nothing to load at run time.

/**
 * The catalog of the packages a workspace has, the bundled ones among them: their symbols,
 * ordinary definitions and sets.
 */
export type Catalog = {
  /**
   * The packages listed, by name, then by version.
   */
  packages: Array<CatalogPackage>,
  /**
   * Their symbols, package by package as `packages` lists them, each package's in the order of
   * its file; so are the definitions and the sets.
   */
  symbols: Array<CatalogSymbol>,
  definitions: Array<CatalogDefinition>,
  sets: Array<CatalogSet>,
};

/**
 * A package file (2.7).
 */
export type CatalogPackage = {
  /**
   * As its PACKAGE says.
   */
  name: string,
  version: string,
  /**
   * The language version its file declares, or the one a file without it is read as (16.1).
   */
  language: string,
  /**
   * The name of its file, such as `rnl-std-basic.rnl`.
   */
  file: string,
  /**
   * The packages its header record references with USE-PACKAGE, by name, in order (16.2).
   */
  references: Array<string>,
  /**
   * A package of the standard series: `rnl-std` or `rnl-std-*` (16.3).
   */
  series: boolean,
  /**
   * The standard package, `rnl-std`, used implicitly at the root (16.3).
   */
  implicit: boolean,
};

/**
 * A symbol of a package (9.2), in its own coordinates, y up.
 */
export type CatalogSymbol = {
  /**
   * Such as `$RESISTOR`.
   */
  symbol: string,
  /**
   * The package that defines it, its version and the name of its file.
   */
  package: string,
  version: string,
  file: string,
  /**
   * The definition whose usages a rule of the packages listed draws with it, a rule written
   * `# @SELECT-USAGE $DEF => @SET-SYMBOL $SYM`, such as `$GROUND` for `$GROUND_SYMBOL`; null when
   * none does.
   */
  definition: string | null,
  /**
   * Its pins, one for each terminal, in terminal order: those by number, then those by name in
   * the order written.
   */
  pins: Array<CatalogPin>,
  /**
   * x1, y1, x2, y2: the box the symbol writes, or the extent of its graphics (9.2.3), as the
   * drawings have it.
   */
  box: [number, number, number, number],
  /**
   * Its labels, the texts it writes with a key (13.3), in the order written.
   */
  labels: Array<CatalogLabel>,
};

/**
 * The pin of a terminal (9.2.4).
 */
export type CatalogPin = {
  /**
   * As the symbol writes it: a number, or a name.
   */
  terminal: Terminal,
  at: [number, number],
  /**
   * The exit direction: "right", "up", "left" or "down"; null when the pin writes none.
   */
  direction: "right" | "up" | "left" | "down" | null,
};

/**
 * A terminal as a pin writes it (9.2.4): a number, or a name.
 */
export type Terminal = number | string;

/**
 * A text a symbol writes with a key (13.3).
 */
export type CatalogLabel = {
  /**
   * Such as "name" or "value".
   */
  key: string,
  /**
   * As written, before `${…}` is read (13.6), such as "${name}"; null when the label writes none.
   */
  text: string | null,
  /**
   * Where the symbol puts it; null when it does not.
   */
  at: [number, number] | null,
  /**
   * As written, or as the model draws a label that writes none: "start" (13.3).
   */
  halign: string,
  /**
   * As written, or "baseline".
   */
  valign: string,
};

/**
 * An ordinary definition of a package (9.4).
 */
export type CatalogDefinition = {
  /**
   * Such as `$GROUND`.
   */
  definition: string,
  package: string,
  version: string,
  file: string,
  /**
   * Its roles as `roles=` writes them: a name, `"{name}"` for a set role, or `[name, kinds]`,
   * the kinds a name or a list of names.
   */
  roles: Array<string | [string, string | Array<string>]>,
  /**
   * Its parameters, each `[name, default]`; the default is null when it has none.
   */
  parameters: Array<[string, string | number | boolean | Array<unknown> | null]>,
  /**
   * The symbol a rule of the packages listed draws its usages with, the other side of a
   * symbol's `definition`, such as `$GROUND_SYMBOL` for `$GROUND`; null when none does.
   */
  symbol: string | null,
};

/**
 * A set of a package (5.2), such as one the fill edits (10.5).
 */
export type CatalogSet = {
  /**
   * Such as `$GROUND_NET_SET`.
   */
  set: string,
  package: string,
  version: string,
  file: string,
};
