import { test } from "node:test";
import assert from "node:assert/strict";
import { attachments, groupTree, layoutTable, nodeOf } from "../viewer-ui/group-tree.js";

const box = (x0, y0, x1, y1, objects) => ({ kind: "box", points: [[x0, y0], [x1, y1]], objects, id: null });

test("groupTree is the tree the core gives: the group above each, and the groups left out, repeated or turned off", () => {
  // the Document's order: the tree, each group after the group above it, then the groups outside it
  const level = {
    usages: [
      { kind: "group", object: 11, id: "%outer", state: "in-effect", roles: [{ name: "members", objects: [0, 1, 2] }] },
      { kind: "group", object: 13, id: "%cross", state: "in-effect", group_parent: 11, roles: [{ name: "members", objects: [1, 2] }] },
      { kind: "group", object: 12, id: "%inner", state: "left-out", crosses: 13, reason: "it crosses %cross", roles: [{ name: "members", objects: [0, 1] }] },
      { kind: "group", object: 10, id: "pair_1", state: "left-out", crosses: 13, roles: [{ name: "members", objects: [0, 1] }] },
      { kind: "group", object: 14, id: "%off", state: "disabled", roles: [{ name: "members", objects: [0, 2] }] },
      { kind: "group", object: 15, id: "%empty", state: "left-out", roles: [{ name: "members", objects: [] }] },
    ],
  };
  const tree = groupTree(level);
  assert.deepEqual(tree.nodes.slice(1).map((n) => n.group.id), ["%outer", "%cross"]);
  const outer = tree.nodes.find((n) => n.group?.id === "%outer");
  const cross = tree.nodes.find((n) => n.group?.id === "%cross");
  assert.equal(outer.parent, tree.root);
  assert.equal(cross.parent, outer);
  assert.deepEqual([outer.depth, cross.depth], [1, 2]);
  assert.deepEqual(outer.own, [0]);
  assert.equal(nodeOf(tree, 1), cross);
  assert.equal(nodeOf(tree, 0), outer);
  assert.equal(nodeOf(tree, 99), tree.root);
  assert.deepEqual(tree.off.map((u) => u.id), ["%off"]);
  assert.deepEqual(tree.left.map((x) => [x.group.id, x.crosses?.id ?? null]), [["%inner", "%cross"], ["pair_1", "%cross"], ["%empty", null]]);
  assert.equal(tree.left[0].reason, "it crosses %cross");

  // a group with the members of another is that one
  const twinTree = groupTree({
    usages: [
      { kind: "group", object: 21, id: "%named", state: "in-effect", roles: [{ name: "members", objects: [0, 1] }] },
      { kind: "group", object: 20, id: "gen_a", state: "same", same_as: 21, roles: [{ name: "members", objects: [0, 1] }] },
    ],
  });
  assert.deepEqual(twinTree.nodes.slice(1).map((n) => n.group.id), ["%named"]);
  assert.equal(twinTree.same.length, 1);
  assert.equal(twinTree.same[0].group.id, "gen_a");
  assert.equal(twinTree.same[0].as.id, "%named");

  // a level of an earlier version gives no tree: its groups are all at the top
  const flat = groupTree({
    usages: [
      { kind: "group", object: 1, id: "%a", state: "in-effect", roles: [{ name: "members", objects: [0, 1] }] },
      { kind: "group", object: 2, id: "%b", state: "in-effect", roles: [{ name: "members", objects: [0] }] },
    ],
  });
  assert.deepEqual(flat.root.children.map((n) => n.group.id), ["%a", "%b"]);
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
