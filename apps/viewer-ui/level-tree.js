// The levels of the document as the hierarchy shows them, each by its host level (`Level.host`, the index of the
// outline's levels): its title, the file it is written in, and the subcircuits its instances call. A shell that reads
// the file shown alone gives the outline of the whole document with it (`doc.outline`), which has every level, those
// of the files not read too; otherwise the Document has them all. From the outline, the instances of a level are
// listed by the subcircuit they call, in the order first called, and a name the netlist does not define is one row
// for all its instances.

import { doc } from "./state.js";

let cached = { doc: null, tree: null };

/**
 * The tree of the document shown, or null: `nodes[host]` is `{ title, file, children }` for each circuit, `file` the
 * file as the document names it, each child `{ instance, master, host, count }` with `host` null when the netlist does
 * not define the subcircuit, `instance` null for a row of all the instances of such a name, and `count` their number;
 * `top` is the host of the top level.
 */
export function levelTree() {
  if (!doc) return null;
  if (cached.doc !== doc) cached = { doc, tree: build(doc) };
  return cached.tree;
}

function build(d) {
  const nodes = [];
  const outline = d.outline;
  if (outline) {
    outline.levels.forEach((l, host) => {
      if (!l.circuit) return;
      nodes[host] = {
        title: l.title,
        file: l.file === null ? null : outline.files[l.file].path,
        children: l.calls.flatMap((c) => c.level === null || !c.instances
          ? [{ instance: null, master: c.master, host: null, count: c.count }]
          : c.instances.map((instance) => ({ instance, master: c.master, host: c.level, count: 1 }))),
      };
    });
    return { nodes, top: 0 };
  }
  for (const l of d.levels) {
    nodes[l.host] = {
      title: l.title,
      file: l.lines ? d.files[l.lines[0]]?.path ?? null : d.files[0]?.path ?? null,
      children: l.children.map((c) => ({ instance: c.instance, master: c.master, host: c.level === null ? null : d.levels[c.level].host, count: 1 })),
    };
  }
  return { nodes, top: d.levels[0]?.host ?? 0 };
}

/** The level of the Document shown that draws the host level `host`, or -1 when another file's reading would. */
export function localLevel(host) {
  return doc ? doc.levels.findIndex((l) => l.host === host) : -1;
}

/** The host level of the level `level` of the Document shown. */
export function hostOf(level) {
  return doc?.levels[level]?.host ?? null;
}
