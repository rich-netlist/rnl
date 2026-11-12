import { test } from "node:test";
import assert from "node:assert/strict";
import { skeletonHit, skeletonLines } from "../viewer-ui/skeleton-geometry.js";

const doc = {
  objects: [
    { kind: "instance", name: "R1", level: 0 },
    { kind: "instance", name: "R2", level: 0 },
    { kind: "usage", name: "%flag1", level: 0 },
    { kind: "usage", name: "%dot1", level: 0 },
    { kind: "net", name: "mid", level: 0 },
    {
      kind: "pin",
      name: "pin(#R1,0)",
      level: 0,
      pin: { owner: 0, index: 0, reference: "#R1::p", at: [-2, 0], local_at: [-2, 0], dir: 180, nets: ["in"] },
    },
    {
      kind: "pin",
      name: "pin(#R1,1)",
      level: 0,
      pin: { owner: 0, index: 1, reference: "#R1::n", at: [2, 0], local_at: [2, 0], dir: 0, nets: ["mid"] },
    },
    {
      kind: "pin",
      name: "pin(#R2,0)",
      level: 0,
      pin: { owner: 1, index: 0, reference: "#R2::p", at: [8, 2], local_at: [0, 2], dir: 90, nets: ["mid"] },
    },
  ],
};

const level = {
  host: 0,
  scale: 10,
  usages: [
    { object: 2, kind: "usage", id: "%flag1", definition: "$FLAG", has_symbol: true, roles: [{ name: "p", objects: [4] }] },
    { object: 3, kind: "usage", id: "%dot1", definition: "$MARK", has_symbol: true, roles: [] },
  ],
  hits: [
    { kind: "box", points: [[-20, -10], [20, 10]], origin: [0, 0], objects: [0] },
    { kind: "box", points: [[70, -20], [90, 20]], origin: [80, 0], objects: [1] },
    { kind: "box", points: [[35, 30], [45, 50]], origin: [40, 40], objects: [2] },
    // Point-mark usage at wire T-junction: does not emit a coordinate cross
    { kind: "box", points: [[78, -2], [82, 2]], origin: [80, 0], objects: [3] },
    // Wire from (20, 0) -> (80, 0) -> (80, -20), another collinear y=0 segment (-40, 0) -> (-20, 0), and a diagonal
    { kind: "wire", points: [[20, 0], [80, 0], [80, -20]], objects: [4] },
    { kind: "wire", points: [[-40, 0], [-20, 0]], objects: [4] },
    { kind: "wire", points: [[40, 40], [80, 0]], objects: [4] },
  ],
};

test("skeletonLines builds component crosses, directional pin lines and extended wire lines, skipping point marks", () => {
  const symOnly = skeletonLines(level, doc, { symbols: true, wires: false });
  // Components: #R1 (x=0, y=0), #R2 (x=80, y=0), %flag1 (x=40, y=40); %dot1 skipped.
  // Pins: #R1::p (dir 180 -> y=0), #R1::n (dir 0 -> y=0), #R2::p (dir 90 -> x=80).
  // Distinct infinite lines for symbols+pins: x=0, y=0, x=80, x=40, y=40 -> 5 lines.
  assert.equal(symOnly.lines.length, 5);
  assert.ok(symOnly.lines.every((l) => l.kind === "symbol"));
  assert.ok(!symOnly.lines.some((l) => l.owners.some((o) => o.includes("dot1"))));

  const yZero = symOnly.lines.find((l) => Math.abs(l.a[1]) < 1e-6 && Math.abs(l.b[1]) < 1e-6);
  assert.ok(yZero);
  assert.deepEqual(yZero.owners, ["y of #R1", "y of #R2", "pin #R1::p", "pin #R1::n"]);

  const xEighty = symOnly.lines.find((l) => Math.abs(l.a[0] - 80) < 1e-6 && Math.abs(l.b[0] - 80) < 1e-6);
  assert.ok(xEighty);
  assert.deepEqual(xEighty.owners, ["x of #R2", "pin #R2::p"]);
});

test("skeletonLines deduplicates coincident component/pin and wire lines, counts collinear segments, and clips diagonal wires", () => {
  const wireOnly = skeletonLines(level, doc, { symbols: false, wires: true });
  // Wire segments: y=0 (2 segments), x=80 (1 segment), and diagonal x + y = 80 -> 3 distinct lines
  assert.equal(wireOnly.lines.length, 3);
  assert.ok(wireOnly.lines.every((l) => l.kind === "wire"));

  const both = skeletonLines(level, doc, { symbols: true, wires: true });
  // 5 symbol lines + 3 wire lines - 2 shared (y=0 and x=80) = 6 lines total
  assert.equal(both.lines.length, 6);
  const shared = both.lines.filter((l) => l.kind === "shared");
  assert.equal(shared.length, 2);

  // Diagonal wire line spans the padded extent boundary
  const diag = both.lines.find((l) => l.kind === "wire");
  assert.ok(diag);
  assert.deepEqual(diag.owners, ["wire &mid"]);
  const [x0, y0, x1, y1] = both.extent;
  for (const [px, py] of [diag.a, diag.b]) {
    const onBorder =
      Math.abs(px - x0) < 1e-5 ||
      Math.abs(px - x1) < 1e-5 ||
      Math.abs(py - y0) < 1e-5 ||
      Math.abs(py - y1) < 1e-5;
    assert.ok(onBorder);
  }
});

test("skeletonHit reports all owners on hovered lines and falls back to box-edge pin inference when PinObjects are absent", () => {
  const both = skeletonLines(level, doc, { symbols: true, wires: true });
  const tip = skeletonHit(both.lines, -30, 0, 2);
  assert.equal(tip, ["y of #R1", "y of #R2", "pin #R1::p", "pin #R1::n", "wire &mid (2 segments)"].join("\n"));
  assert.equal(skeletonHit(both.lines, -30, 15, 2), null);

  // Fallback for 0.2/0.3 levels without PinObjects in doc.objects
  const doc03 = { objects: [{ kind: "instance", name: "U1" }, { kind: "net", name: "out" }] };
  const level03 = {
    hits: [
      { kind: "box", points: [[-10, -20], [10, 20]], origin: [0, 0], objects: [0] },
      { kind: "pin", points: [[10, 5]], objects: [1] }, // right vertical edge -> horizontal pin line y=5
    ],
  };
  const fb = skeletonLines(level03, doc03, { symbols: true, wires: false });
  assert.equal(fb.lines.length, 3); // x=0, y=0, and pin y=5
  assert.equal(skeletonHit(fb.lines, -20, 5, 1), "pin of #U1 (&out)");
  assert.equal(skeletonHit(fb.lines, 0, 5, 1), "x of #U1\npin of #U1 (&out)");
});
