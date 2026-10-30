// How the frames of usages and groups are set out, for the drawing (usages.js) and for Export Image (export-image.js), so
// that an image is as the page shows it. Frames that lie inside others go out one label height for each frame inside
// them, the outer MAX_STEPS of a nest only, and the titles that would lie on one another go down a row each. Both are
// counted over the frames shown, in the units of the drawing at its scale: CSS pixels on the page, SVG units in the image.

export const FRAME_STEP = 18; // how far a frame goes out beyond the frames within it, for each step: a title's height
export const MAX_STEPS = 3; // the outer frames of a nest that are set out; the frames below them are drawn at their extent, over one another
export const MAX_NESTED = 400; // frames: more than this, they are drawn as the core gives them (the pairs are counted)
export const TITLE_ROW = 18; // the height of a title, by which one that would lie on another goes down
const MAX_ROWS = 12;

/**
 * How many steps outside its extent each frame of `drawn` is drawn, by the usage's object: so that nested frames, which
 * the core gives the same edge when their members share one, and their titles do not lie on one another. Usages and
 * groups alike, the frames shown only. A frame goes out one step for each frame in it, the longest chain, but only the
 * outer MAX_STEPS frames of a nest are set out: those below are drawn at their extent, over one another. Of two frames
 * of one extent the one listed later is the outer. `drawn` has `object` and `frame.bounds` ([x0, y0, x1, y1]) of each;
 * a virtual symbol's own box (`frame.local`) is no frame of the usages and is left out of it.
 */
export function frameSteps(drawn) {
  const steps = new Map();
  if (drawn.length > MAX_NESTED) return steps;
  const inside = (a, b) => a.bounds[0] >= b.bounds[0] - 0.5 && a.bounds[1] >= b.bounds[1] - 0.5 && a.bounds[2] <= b.bounds[2] + 0.5 && a.bounds[3] <= b.bounds[3] + 0.5;
  const equal = (a, b) => inside(a, b) && inside(b, a);
  const holds = (i, k) => i !== k && inside(drawn[k].frame, drawn[i].frame) && (!equal(drawn[k].frame, drawn[i].frame) || k < i); // frame i holds frame k
  const indexes = drawn.map((_, i) => i);
  const memo = (next) => {
    const done = new Map();
    const of = (i) => {
      if (!done.has(i)) done.set(i, Math.max(-1, ...indexes.filter((k) => next(i, k)).map(of)) + 1); // (the order of equal frames is strict: no chain loops)
      return done.get(i);
    };
    return of;
  };
  const below = memo((i, k) => holds(i, k)); // frames in a frame, the longest chain
  const above = memo((i, k) => holds(k, i)); // frames around a frame, the longest chain
  indexes.forEach((i) => steps.set(drawn[i].object, Math.min(below(i), Math.max(0, MAX_STEPS - above(i)))));
  return steps;
}

/**
 * The row, 0 or more, each title is to go down by, so that none lies on another's: titles are `{ key, x, y, width,
 * first }`, the badge of a title being from x - 4 to x + width + 4 and y - 9 to y + 9. The ones with a greater `first`
 * are placed first and stay where they are; the others go down a TITLE_ROW at a time until they are clear.
 */
export function titleRows(titles) {
  const placed = [];
  const rows = new Map();
  for (const t of [...titles].sort((a, b) => b.first - a.first)) {
    const meets = (top) => placed.some((r) => t.x - 4 < r[2] - 0.5 && t.x + t.width + 4 > r[0] + 0.5 && top - 9 < r[3] - 0.5 && top + 9 > r[1] + 0.5);
    let row = 0;
    while (row < MAX_ROWS && meets(t.y + row * TITLE_ROW)) row++;
    rows.set(t.key, row);
    const top = t.y + row * TITLE_ROW;
    placed.push([t.x - 4, top - 9, t.x + t.width + 4, top + 9]);
  }
  return rows;
}
