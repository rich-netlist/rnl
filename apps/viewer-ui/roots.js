// The subcircuits that the top level does not call, and the level a document opens on. A netlist of
// subcircuits alone, such as a library, or a testbench defined but never instantiated, has a top
// level with nothing on it; the hierarchy would list nothing below it. The levels that no chain of
// instances from the top level reaches are listed as roots of trees of their own, each with the
// subcircuits it calls, and the page opens on one of them when the top level has no objects.

import { doc } from "./state.js";
import { m } from "./i18n.js";

let cached = { doc: null, roots: [] };

/**
 * The levels that no chain of subcircuit instances from the top level reaches and that start a tree
 * of their own, in the order of the netlist. A level that a root calls, or one that calls it
 * around a cycle that nothing else calls, hangs below that root and is not a root itself; of
 * levels that call each other and nothing outside, the first starts the tree. A level that calls
 * itself is called by nothing else all the same.
 */
export function uncalledRoots() {
  if (!doc) return [];
  if (cached.doc !== doc) cached = { doc, roots: compute(doc) };
  return cached.roots;
}

function compute(d) {
  const levels = d.levels;
  if (levels.length === 0) return [];
  const callees = (l) => levels[l].children.map((c) => c.level).filter((k) => k !== null);
  /** Add `from` and the levels below it to `seen`. */
  const reach = (from, seen) => {
    const stack = [from];
    while (stack.length > 0) {
      const l = stack.pop();
      if (seen.has(l)) continue;
      seen.add(l);
      stack.push(...callees(l));
    }
  };
  const seen = new Set();
  reach(0, seen);
  const rest = levels.map((_, k) => k).filter((k) => !seen.has(k));
  const called = new Set();
  for (const l of rest) for (const k of callees(l)) if (k !== l) called.add(k);
  const roots = rest.filter((l) => !called.has(l));
  for (const r of roots) reach(r, seen);
  // what is left is called only around cycles that nothing outside calls
  for (const l of rest) {
    if (seen.has(l)) continue;
    roots.push(l);
    reach(l, seen);
  }
  return roots.sort((a, b) => a - b);
}

/**
 * The level a document opens on, and a line that says why when it is not the top level. The top
 * level when it has something, objects that have no position yet included; when it has no objects
 * at all and the netlist has subcircuits that it does not call, the first of their roots that has
 * something to draw, or else the first.
 */
export function startLevel() {
  const top = doc.levels[0];
  const roots = uncalledRoots();
  if (!top.empty || top.unplaced.length > 0 || roots.length === 0) return { level: 0, note: "" };
  const level = roots.find((r) => !doc.levels[r].empty) ?? roots[0];
  return { level, note: m`The top level is empty; showing ${doc.levels[level].title}.` };
}
