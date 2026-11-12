import { test } from "node:test";
import assert from "node:assert/strict";
import { attachments, groupTree, layoutTable, nodeOf } from "../viewer-ui/group-tree.js";

const box = (x0, y0, x1, y1, objects) => ({ kind: "box", points: [[x0, y0], [x1, y1]], objects, id: null });

test("groupTree orders written groups before generated ones and separates twins, crossings and disabled groups", () => {
  const level = {
    usages: [
      { kind: "group", object: 10, id: "pair_1", state: "in-effect", roles: [{ name: "members", objects: [0, 1] }] },
      { kind: "group", object: 11, id: "%outer", state: "in-effect", roles: [{ name: "members", objects: [0, 1, 2] }] },
      { kind: "group", object: 12, id: "%inner", state: "in-effect", roles: [{ name: "members", objects: [0, 1] }] },
      { kind: "group", object: 13, id: "%cross", state: "in-effect", roles: [{ name: "members", objects: [1, 2] }] },
      { kind: "group", object: 14, id: "%off", state: "disabled", roles: [{ name: "members", objects: [0, 2] }] },
    ],
  };
  // Written groups in reverse order: %cross ([1,2]), then %inner ([0,1], crosses %cross -> left), then %outer ([0,1,2], holds %cross),
  // then generated pair_1 ([0,1], crosses %cross -> left).
  const tree = groupTree(level);
  assert.deepEqual(tree.nodes.slice(1).map((n) => n.group.id), ["%cross", "%outer"]);
  const outer = tree.nodes.find((n) => n.group?.id === "%outer");
  const cross = tree.nodes.find((n) => n.group?.id === "%cross");
  assert.equal(outer.parent, tree.root);
  assert.equal(cross.parent, outer);
  assert.deepEqual(outer.own, [0]);
  assert.equal(nodeOf(tree, 1), cross);
  assert.equal(nodeOf(tree, 0), outer);
  assert.equal(nodeOf(tree, 99), tree.root);
  assert.deepEqual(tree.off.map((u) => u.id), ["%off"]);
  assert.equal(tree.left.length, 2);

  // Identical member sets merge into `same`.
  const twinTree = groupTree({
    usages: [
      { kind: "group", object: 20, id: "gen_a", state: "in-effect", roles: [{ name: "members", objects: [0, 1] }] },
      { kind: "group", object: 21, id: "%named", state: "in-effect", roles: [{ name: "members", objects: [0, 1] }] },
    ],
  });
  assert.deepEqual(twinTree.nodes.slice(1).map((n) => n.group.id), ["%named"]);
  assert.equal(twinTree.same.length, 1);
  assert.equal(twinTree.same[0].group.id, "gen_a");
  assert.equal(twinTree.same[0].as.id, "%named");
});

test("attachments resolves pin_of/parent chains and point-mark usages from structure and solver_config", () => {
  const doc = {
    objects: [
      { kind: "instance", name: "R1" },
      { kind: "net", name: "n1" },
      { kind: "usage", name: "%wrap" },
      { kind: "usage", name: "%cap_sym" },
      { kind: "usage", name: "%dot_struct" },
      { kind: "usage", name: "%dot_cfg" },
      { kind: "usage", name: "%net_tag" },
      { kind: "usage", name: "%two_pin" },
      { kind: "pin", name: "%two_pin:p", pin: { owner: 7, local_at: [-2, 0], nets: [] } },
    ],
  };
  const level = {
    hits: [
      box(0, 0, 40, 20, [0]),
      box(10, 20, 30, 40, [3]),
      box(50, 8, 54, 12, [4]),
      box(60, 8, 64, 12, [5]),
      box(80, 0, 100, 20, [6, 1]),
      box(120, 0, 160, 20, [7]),
    ],
    usages: [
      { kind: "usage", object: 2, definition: "$WRAP", pin_of: 0, has_symbol: false, roles: [] },
      { kind: "usage", object: 3, definition: "$DRAWN_TAG", parent: 2, has_symbol: true, roles: [] },
      { kind: "usage", object: 4, definition: "$MARK_A", has_symbol: true, roles: [] },
      { kind: "usage", object: 5, definition: "$MARK_B", has_symbol: true, roles: [{ name: "r", objects: [1] }] },
      { kind: "usage", object: 6, definition: "$NET_SYM", has_symbol: true, roles: [{ name: "net", objects: [1] }] },
      { kind: "usage", object: 7, definition: "$PART_SYM", has_symbol: true, roles: [] },
    ],
    constraints: {
      solver_config: [{ key: "junction", value: "$MARK_B" }],
    },
  };
  const { hostOf, junction } = attachments(level, doc);
  assert.equal(hostOf(3), 0);
  assert.equal(hostOf(2), 0);
  assert.equal(hostOf(6), 6);
  assert.equal(junction(4), true);
  assert.equal(junction(5), true);
  assert.equal(junction(3), false);
  assert.equal(junction(6), false);
  assert.equal(junction(7), false);
});

test("layoutTable partitions nested groups and objects into cells and marks interlocking regions stuck", () => {
  const level = {
    hits: [
      box(0, 0, 40, 20, [0]),
      box(10, 20, 30, 36, [3]), // hangs on object 0 -> merged into object 0's cell
      box(80, 0, 120, 20, [1]),
      box(0, 60, 120, 80, [2]),
      box(60, 8, 64, 12, [4]), // point mark -> ignored
    ],
    usages: [
      { kind: "usage", object: 3, definition: "$TAG", pin_of: 0, has_symbol: true, roles: [] },
      { kind: "usage", object: 4, definition: "$DOT", has_symbol: true, roles: [] },
      { kind: "group", object: 10, id: "%top_pair", state: "in-effect", roles: [{ name: "members", objects: [0, 1] }] },
    ],
  };
  const table = layoutTable(level);
  assert.equal(table[0].kind, "frame");
  assert.deepEqual(table[0].rect, [-14, -14, 134, 94]);
  assert.ok(table.some((e) => e.kind === "group" && e.key === 10));
  const cells = table.filter((e) => e.kind === "cell");
  assert.deepEqual(cells.map((c) => c.key).sort((a, b) => a - b), [0, 1, 2]);

  // Pinwheel of four boxes that no single straight line across x or y separates -> stuck + loose.
  const pinwheel = {
    hits: [
      box(0, 0, 60, 30, [0]),
      box(70, 0, 100, 60, [1]),
      box(40, 70, 100, 100, [2]),
      box(0, 40, 30, 100, [3]),
    ],
    usages: [],
  };
  const stuck = layoutTable(pinwheel);
  assert.ok(stuck.some((e) => e.kind === "stuck"));
  assert.equal(stuck.filter((e) => e.kind === "loose").length, 4);
});
