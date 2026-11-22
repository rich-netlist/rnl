// The levels of the document as the hierarchy shows them, each by its host level (`Level.host`, the index of the
// outline's levels): its title, the file it is written in, and the subcircuits its instances call. A shell that reads
// the file shown alone gives the outline of the whole document with it (`doc.outline`), which has every level, those
// of the files not read too; otherwise the Document has them all. From the outline, the instances of a level are
// listed by the subcircuit they call, in the order first called, and a name the netlist does not define is one row
// for all its instances. The outline has the modules of Verilog-A too, which no reading draws (2.9): each is a node
// with `module` set, the modules its instances name its children, with the line of each instance. The preview of a solve
// of one file is the reading of that file, with the outline of the reading it was solved from (solve.js). The preview of
// the whole document has no modules: it has the outline of the reading the solve started from as `outlineBefore`, whose
// modules are nodes of its tree too, each instance that names one a child of it.

import { doc } from "./state.js";

let cached = { doc: null, tree: null };
/** The host level of the module of Verilog-A shown in place of a drawing (module-view.js), or null. */
let moduleShown = null;

/**
 * The tree of the document shown, or null: `nodes[host]` is `{ title, file, children }` for each circuit, `file` the
 * file as the document names it, each child `{ instance, master, host, count }` with `host` null when the netlist does
 * not define the subcircuit, `instance` null for a row of all the instances of such a name, and `count` their number;
 * `top` is the host of the top level. From the outline, a module of Verilog-A is a node too, with `module` true, its
 * `ports` and `lines`, and the `line` of each of its children (the line of the instance in the module's file).
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
      if (l.circuit || l.kind === "module") nodes[host] = outlineNode(outline, l);
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
  const before = d.outlineBefore;
  if (before) {
    // the modules, and the instances of the levels that name one, by the calls of the level in the outline
    before.levels.forEach((l, host) => {
      if (l.kind === "module") nodes[host] = outlineNode(before, l);
    });
    for (const l of d.levels) {
      const calls = before.levels[l.host]?.calls ?? [];
      for (const c of nodes[l.host].children) {
        if (c.host !== null) continue;
        const call = calls.find((x) => x.master === c.master && x.level !== null && before.levels[x.level]?.kind === "module");
        if (call) c.host = call.level;
      }
    }
  }
  return { nodes, top: d.levels[0]?.host ?? 0 };
}

/** The node of the level `l` of `outline`: a circuit, or a module of Verilog-A. */
function outlineNode(outline, l) {
  const module = l.kind === "module";
  return {
    title: l.title,
    file: l.file === null ? null : outline.files[l.file].path,
    children: l.calls.flatMap((c) => c.level === null || !c.instances
      ? [{ instance: null, master: c.master, host: null, count: c.count }]
      : c.instances.map((instance, k) => ({ instance, master: c.master, host: c.level, count: 1,
        ...(module ? { line: c.lines?.[k] ?? null } : {}) }))),
    ...(module ? { module, ports: l.ports, lines: l.lines } : {}),
  };
}

/**
 * Whether the Document shown is the preview of a solve that has the outline of the reading it started from
 * (`outlineBefore`), whose modules of Verilog-A it shows in place, below its own levels.
 */
export const previewModules = () => !!doc && !doc.outline && !!doc.outlineBefore;

/** The level of the Document shown that draws the host level `host`, or -1 when another file's reading would. */
export function localLevel(host) {
  return doc ? doc.levels.findIndex((l) => l.host === host) : -1;
}

/** The host level of the level `level` of the Document shown. */
export function hostOf(level) {
  return doc?.levels[level]?.host ?? null;
}

/** Note the module shown in place of a drawing, by its host level, or null as a level is drawn. */
export function setModuleShown(host) {
  moduleShown = host;
}

/** The host level shown: the module's (module-view.js), or that of the level `level` drawn. */
export function shownHost(level) {
  return moduleShown ?? hostOf(level);
}
