import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import test from "node:test";

const source = readFileSync(new URL("../../apps/viewer-ui/roots.js", import.meta.url), "utf8")
  .replace('import { doc } from "./state.js";', "")
  .replaceAll("export function ", "function ");

function viewer() {
  const context = vm.createContext({ doc: null });
  vm.runInContext(`${source}\nglobalThis.roots = uncalledRoots; globalThis.start = startLevel;`, context);
  return (edges, drawn = [], unplaced = []) => {
    context.doc = {
      levels: edges.map((children, level) => ({
        title: `level ${level}`,
        children: children.map((child) => ({ level: child })),
        empty: !drawn.includes(level),
        unplaced: unplaced.includes(level) ? [{}] : [],
      })),
    };
    return { roots: Array.from(context.roots()), start: context.start().level };
  };
}

test("a callee declared before the cycle that calls it is not a root or the starting level", () => {
  assert.deepEqual(viewer()([[], [], [3, 1], [2]], [1, 2]), { roots: [2], start: 2 });
});

test("a downstream cycle declared first stays below the source cycle", () => {
  assert.deepEqual(viewer()([[], [2], [1], [4, 1], [3]], [1, 3]), { roots: [3], start: 3 });
});

test("independent acyclic, cyclic and self-calling roots retain declaration order", () => {
  assert.deepEqual(viewer()([[], [2], [], [4], [3], [5]], [2, 3, 5]), { roots: [1, 3, 5], start: 3 });
});

test("levels reached from the top are excluded even when they call each other", () => {
  assert.deepEqual(viewer()([[1], [2], [1], []], [1, 3]), { roots: [3], start: 3 });
});

test("drawn or unplaced top levels keep the top as the starting level", () => {
  assert.deepEqual(viewer()([[], []], [0, 1]), { roots: [1], start: 0 });
  assert.deepEqual(viewer()([[], []], [1], [0]), { roots: [1], start: 0 });
  assert.deepEqual(viewer()([[]]), { roots: [], start: 0 });
});

// Independent small-graph oracle: classify mutual reachability, then exclude classes with an
// incoming edge from any other class. It does not use DFS finishing order or the production walk.
function sourceRepresentatives(edges) {
  const reach = edges.map((_, start) => {
    const found = new Set([start]);
    let changed;
    do {
      changed = false;
      for (const from of Array.from(found)) {
        for (const to of edges[from]) {
          if (!found.has(to)) {
            found.add(to);
            changed = true;
          }
        }
      }
    } while (changed);
    return found;
  });
  const remaining = edges.map((_, level) => level).filter((level) => !reach[0].has(level));
  return remaining.filter(
    (level) =>
      !remaining.some((other) => other < level && reach[level].has(other) && reach[other].has(level)) &&
      !remaining.some((other) => reach[other].has(level) && !reach[level].has(other)),
  );
}

test("every directed graph on three uncalled levels selects exactly its source components", () => {
  const compute = viewer();
  for (let mask = 0; mask < 1 << 9; mask += 1) {
    const edges = [[], [], [], []];
    for (let from = 1; from <= 3; from += 1) {
      for (let to = 1; to <= 3; to += 1) {
        if (mask & (1 << ((from - 1) * 3 + to - 1))) edges[from].push(to);
      }
    }
    assert.deepEqual(compute(edges).roots, sourceRepresentatives(edges), `edge mask ${mask}`);
  }
});
