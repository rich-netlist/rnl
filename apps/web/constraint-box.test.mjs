import { test } from 'node:test';
import assert from 'node:assert/strict';
import { boxTitle, constraintBox, factMembers, factTextIds } from '../viewer-ui/constraint-box.js';

// a level of three resistors in a row, a ground stub under the last, a free point and a text
const objects = [
  { kind: 'instance', label: '#R1', id: '#R1', objects: [0], pins: [[0, 8], [80, 8]] },
  { kind: 'instance', label: '#R2', id: '#R2', objects: [3], pins: [[100, 8], [180, 8]] },
  { kind: 'instance', label: '#R3', id: '#R3', objects: [5], pins: [[200, 8], [280, 8]] }, // (its first pin has no net: the hits have none for it)
  { kind: 'usage', label: '%g', id: null, objects: [], pins: [[310, 40]] },
  { kind: 'point', label: '%f', id: '%f', objects: [] },
  { kind: 'point', label: '(0, 6)', id: '@5', objects: [] },
  { kind: 'text', label: '#R1::name', id: '#R1::name', objects: [] },
  { kind: 'text', label: 'a text', id: '%t', objects: [] },
  { kind: 'instance', label: '#R9', id: '#R9', objects: [99] }, // not on the level's drawing
  { kind: 'usage', label: '%s', id: '%s', objects: [20] }, // drawn as a frame only
];
const box = (x0, y0, x1, y1) => ({ kind: 'box', points: [[x0, y0], [x1, y1]], objects: [], id: null });
const hits = [
  { ...box(0, 0, 80, 16), objects: [0], id: 'item-0' },
  { ...box(100, 0, 180, 16), objects: [3], id: 'item-1' },
  { ...box(200, 0, 280, 16), objects: [5], id: 'item-2' },
  { ...box(300, 40, 320, 60), objects: [30, 6], id: 'item-3' },
  { kind: 'label', points: [[2, -20], [30, -4]], objects: [0], id: '#R1::name' },
];
const level = {
  scale: 10,
  hits,
  usages: [
    { object: 20, definition: '$ROW', has_symbol: false, frame: { bounds: [-10, -30, 300, 30] } },
    { object: 30, definition: '$GND', has_symbol: true, frame: { bounds: [290, 35, 330, 70] } },
  ],
  constraints: {
    objects,
    usages: [{ definition: '$GND', label: '%g', generated: true, object: null, parent: null }],
    facts: [
      { kind: 'POINT-x', point: 5, value: 0 }, { kind: 'POINT-y', point: 5, value: 6 },
    ],
  },
};
const origin = (o) => ({ origin: o });
const pin = (object, p, terminal = 'x') => ({ object, pin: p, terminal });

test('a title says the kind and what it is given, not the objects', () => {
  assert.equal(boxTitle({ kind: 'ALIGN', axis: 'y' }), 'ALIGN y');
  assert.equal(boxTitle({ kind: 'DISTRIBUTE', axis: 'x' }), 'DISTRIBUTE x');
  assert.equal(boxTitle({ kind: 'ORDER', axis: 'x', sense: 'increasing' }), 'ORDER x increasing');
  assert.equal(boxTitle({ kind: 'DISTANCE', axis: 'x', sense: 'either', min: 2, max: 4 }), 'DISTANCE x ≥ 2 ≤ 4');
  assert.equal(boxTitle({ kind: 'DISTANCE', axis: 'x', sense: 'increasing', min: 8, max: 8 }), 'DISTANCE x increasing = 8');
  assert.equal(boxTitle({ kind: 'DISTANCE', axis: 'y', sense: 'either', min: 1.5, max: null }), 'DISTANCE y ≥ 1.5');
  assert.equal(boxTitle({ kind: 'DISTANCE', axis: 'y', sense: 'either', min: null, max: 3 }), 'DISTANCE y ≤ 3');
  assert.equal(boxTitle({ kind: 'PLACE-x', value: 3 }), 'PLACE-x = 3');
  assert.equal(boxTitle({ kind: 'PLACE-y', value: -2 }), 'PLACE-y = -2');
  assert.equal(boxTitle({ kind: 'POINT-y', value: 0.1 + 0.2 }), 'POINT-y = 0.3');
  assert.equal(boxTitle({ kind: 'TEXT-dx', value: 4 }), 'TEXT-dx = 4');
  assert.equal(boxTitle({ kind: 'ORIENT', code: 'R90' }), 'ORIENT R90');
  assert.equal(boxTitle({ kind: 'ORIENT-RELATION', relation: 'MX' }), 'ORIENT-RELATION MX');
});

test('the members keep the order of the fact, a distance and a relation number each of their two sets', () => {
  const tags = (fact) => factMembers(fact).map((m) => m.tag);
  assert.deepEqual(tags({ kind: 'ORDER', axis: 'x', sense: 'increasing', members: [origin(2), origin(0), origin(1)] }), ['1', '2', '3']);
  assert.deepEqual(factMembers({ kind: 'ALIGN', axis: 'y', members: [origin(2), origin(0)] }).map((m) => m.point.origin), [2, 0]);
  assert.deepEqual(tags({ kind: 'DISTANCE', from: [origin(0), origin(1)], to: [origin(2)] }), ['from1', 'from2', 'to1']);
  assert.deepEqual(tags({ kind: 'ORIENT-RELATION', from: [0], to: [1, 2] }), ['from1', 'to1', 'to2']);
  assert.deepEqual(factMembers({ kind: 'ORIENT-RELATION', from: [0], to: [1, 2] }).map((m) => m.object), [0, 1, 2]);
  // a fact that lists no objects has none numbered
  assert.deepEqual(tags({ kind: 'PLACE-x', at: origin(1), value: 3 }), [null]);
  assert.deepEqual(tags({ kind: 'ORIENT', object: 1, code: 'R0' }), [null]);
  assert.deepEqual(factMembers({ kind: 'TEXT-dx', text: 6, from: origin(0), value: 1 }).map((m) => m.object ?? m.point.origin), [6, 0]);
});

test('the box holds the objects the fact names, each with its number, in the SVG pixels of the hits', () => {
  const b = constraintBox(level, { kind: 'ORDER', axis: 'x', sense: 'increasing', members: [origin(2), origin(0), origin(1)] });
  assert.equal(b.title, 'ORDER x increasing');
  assert.deepEqual(b.items, [
    { rect: [200, 0, 280, 16], tag: '1' }, { rect: [0, 0, 80, 16], tag: '2' }, { rect: [100, 0, 180, 16], tag: '3' },
  ]);
  assert.deepEqual(b.bounds, [0, 0, 280, 16]);
});

test('a pin is where the core says the object has it, and a set is one number for each of its objects', () => {
  const b = constraintBox(level, {
    kind: 'DISTANCE', axis: 'x', sense: 'increasing', min: 20, max: 20, from: [pin(0, 1)], to: [{ centre: [origin(1), origin(2)] }],
  });
  assert.deepEqual(b.items, [
    { rect: [80, 8, 80, 8], tag: 'from1' }, { rect: [100, 0, 180, 16], tag: 'to1' }, { rect: [200, 0, 280, 16], tag: 'to1' },
  ]);
  // a floating pin before it changes nothing: R3's first pin has no net, so the hits have no point for it, and its second pin is its second
  assert.deepEqual(constraintBox(level, { kind: 'ALIGN', axis: 'x', members: [pin(2, 1), pin(2, 0)] }).items, [
    { rect: [280, 8, 280, 8], tag: '1' }, { rect: [200, 8, 200, 8], tag: '2' },
  ]);
  // a pin the object does not have is its box
  assert.deepEqual(constraintBox(level, { kind: 'ALIGN', axis: 'x', members: [pin(2, 3)] }).items, [{ rect: [200, 0, 280, 16], tag: '1' }]);
});

test('the same object twice has a number each time, and an object of the same level that is not on the drawing none', () => {
  const twice = constraintBox(level, { kind: 'DISTANCE', axis: 'x', sense: 'either', min: null, max: null, from: [origin(0)], to: [origin(0)] });
  assert.deepEqual(twice.items.map((i) => i.tag), ['from1', 'to1']);
  const partly = constraintBox(level, { kind: 'ALIGN', axis: 'y', members: [origin(0), origin(8), origin(4)] });
  assert.deepEqual(partly.items, [{ rect: [0, 0, 80, 16], tag: '1' }]);
  // nothing of the fact on the drawing: no box
  assert.equal(constraintBox(level, { kind: 'ALIGN', axis: 'y', members: [origin(8), origin(4)] }), null);
  assert.equal(constraintBox({ ...level, constraints: undefined }, { kind: 'ORIENT', object: 0, code: 'R0' }), null);
});

test('texts, points and usages that have no box of their own are found by their id, their coordinates or their definition', () => {
  const rects = (fact) => constraintBox(level, fact).items.map((i) => i.rect);
  // a label of an object, by the id the hit has
  assert.deepEqual(rects({ kind: 'TEXT-dx', text: 6, from: origin(0), value: 1 }), [[2, -20, 30, -4], [0, 0, 80, 16]]);
  // a point written (x, y), which is at (x·scale, −y·scale)
  assert.deepEqual(rects({ kind: 'POINT-x', point: 5, value: 0 }), [[0, -60, 0, -60]]);
  // a free point is nowhere
  assert.equal(constraintBox(level, { kind: 'POINT-x', point: 4, value: 0 }), null);
  // a usage drawn as a frame
  assert.deepEqual(rects({ kind: 'ALIGN', axis: 'x', members: [origin(9)] }), [[-10, -30, 300, 30]]);
  // the usage a rule made, which the Document does not name, is the level's one usage of its definition with a symbol, as its box is
  assert.deepEqual(rects({ kind: 'ALIGN', axis: 'x', members: [origin(3), pin(3, 0)] }), [[300, 40, 320, 60], [310, 40, 310, 40]]);
  // a text of the scene is found by the drawing, when the one group has its label
  const found = constraintBox(level, { kind: 'TEXT-dy', text: 7, from: origin(0), value: 1 }, (id) => (id === 'a text' ? [5, 5, 9, 9] : null));
  assert.deepEqual(found.items[0].rect, [5, 5, 9, 9]);
  assert.deepEqual(constraintBox(level, { kind: 'TEXT-dy', text: 7, from: origin(0), value: 1 }).items.map((i) => i.rect), [[0, 0, 80, 16]]);
});

test('the texts a fact names are the labels that stay lit', () => {
  assert.deepEqual(factTextIds(level, { kind: 'TEXT-dx', text: 6, from: origin(0), value: 1 }), ['#R1::name']);
  assert.deepEqual(factTextIds(level, { kind: 'ALIGN', axis: 'x', members: [origin(0), origin(1)] }), []);
});
