// The box a constraint of the Solve view has on the drawing once its row is clicked (apps/viewer-ui/constraints.js,
// usages.js, constraint-box.js), with the web viewer and the core it is built with: around the objects the constraint
// acts on, its title and the order of the objects at its upper left and beside each, with no solve and with a solve's
// preview. It is a hint of the page: it is not in an exported image, and a level other than the one shown has none.
import { test, expect } from '@playwright/test';

test.skip(!process.env.RNL_WEB_URL, 'the web site is not built');

// a subcircuit and a top level of three resistors in a row, placed, which three constraints name in different orders
const NETLIST = `* A row of resistors with constraints
* @RNL(type="header") { @VERSION("0.3") }
.subckt cell input output
Rinside input output 1k
* @RNL { #Rinside @PLACE(0, 0) }
.ends cell
R1 a b 1k
R2 b c 2k
R3 c d 3k
X1 d e cell
* @RNL {
*   #R1 @PLACE(0, 0); #R2 @PLACE(9, 0); #R3 @PLACE(18, 0); #X1 @PLACE(27, 0);
*   {#R1 #R2 #R3} @ALIGN("y");
*   {#R3 #R1 #R2} @ORDER("left-to-right");
*   #R1 #R2 @DISTANCE("x", min=2, max=4);
*   {#R2 #R3 #R2} @ALIGN("y")
* }
.end
`;

async function open(page) {
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await expect(page.locator('#menubar')).toBeVisible();
  await page.locator('#file-input').setInputFiles({ name: 'row.sp', mimeType: 'text/plain', buffer: Buffer.from(NETLIST) });
  await page.waitForFunction(() => window.viewer.doc && !document.body.classList.contains('busy'));
  await page.keyboard.press('7');
  await expect.poll(() => page.locator('#constraints-list li.constraint').count()).toBeGreaterThan(0);
}
const row = (page, text) => page.locator('#constraints-list li.constraint').filter({ hasText: text }).first();
const texts = (page) => page.evaluate(() => [...document.querySelectorAll('.constraint-box text')].map((t) => t.textContent));
const frame = (page) => page.locator('.constraint-box .constraint-frame');

test('a box with a title and the numbers of the objects, in the order the constraint lists them', async ({ page }) => {
  await open(page);
  await expect(frame(page)).toHaveCount(0);
  await row(page, /ALIGN y \{#R1 #R2 #R3\}/).click();
  await expect(frame(page)).toHaveCount(1);
  expect(await texts(page)).toEqual(['ALIGN y', '1', '2', '3']);
  // the order of the list, not the order of the drawing
  await row(page, /ORDER x increasing \{#R3 #R1 #R2\}/).click();
  await expect(frame(page)).toHaveCount(1);
  expect(await texts(page)).toEqual(['ORDER x increasing', '1', '2', '3']);
  const order = await page.evaluate(() => {
    const marks = [...document.querySelectorAll('.constraint-box .constraint-mark')].map((g) => [g.querySelector('text').textContent, g.getBoundingClientRect().left]);
    return marks.sort((a, b) => a[1] - b[1]).map((m) => m[0]);
  });
  expect(order).toEqual(['2', '3', '1']);
  // a distance numbers its two lists apart, and writes its limits
  await row(page, /DISTANCE x/).click();
  expect(await texts(page)).toEqual(['DISTANCE x ≥ 2 ≤ 4', 'from1', 'to1']);
  // every frame of the usages but the box is dimmed
  await expect(page.locator('.usage-overlays [data-muted]')).toHaveCount(await page.locator('.usage-overlays .usage-frame').count());
  // a second click on the row takes the box away
  await row(page, /DISTANCE x/).click();
  await expect(frame(page)).toHaveCount(0);
});

test("a number is at the centre of its object's box, and two numbers at one place are side by side", async ({ page }) => {
  await open(page);
  await row(page, /ALIGN y \{#R1 #R2 #R3\}/).click();
  await expect(frame(page)).toHaveCount(1);
  const marks = () => page.evaluate(() => {
    const svg = document.querySelector('.constraint-box').ownerSVGElement;
    const vb = svg.viewBox.baseVal;
    const k = svg.getBoundingClientRect().width / vb.width;
    const level = window.viewer.doc.levels[0];
    const centre = (name) => {
      const idx = window.viewer.doc.objects.findIndex((o) => o.kind === 'instance' && o.name === name);
      const [[x0, y0], [x1, y1]] = level.hits.find((h) => h.kind === 'box' && h.objects[0] === idx).points;
      return [(x0 + x1) / 2, (y0 + y1) / 2];
    };
    const at = (g) => {
      const m = /translate\(([-\d.e]+)[ ,]([-\d.e]+)\)/.exec(g.getAttribute('transform'));
      const b = g.querySelector('rect').getBBox();
      return [Number(m[1]) + b.x + b.width / 2, Number(m[2])];
    };
    const layer = document.querySelector('.constraint-box');
    const scale = layer.getAttribute('transform').match(/scale\(([-\d.e]+)\)/)[1];
    return {
      k, view: [vb.x, vb.y], scale: Number(scale),
      marks: [...layer.querySelectorAll('.constraint-mark')].map((g) => ({ text: g.querySelector('text').textContent, at: at(g), box: g.getBoundingClientRect().toJSON() })),
      centres: ['R1', 'R2', 'R3'].map(centre),
    };
  });
  const one = await marks();
  expect(one.marks.map((m) => m.text)).toEqual(['1', '2', '3']);
  // the badge's centre, in screen pixels of the layer, is the centre of the object's box
  one.marks.forEach((m, i) => {
    const [cx, cy] = one.centres[i];
    expect(m.at[0] * one.scale + one.view[0]).toBeCloseTo(cx, 0);
    expect(m.at[1] * one.scale + one.view[1]).toBeCloseTo(cy, 0);
  });
  // an object listed twice: both numbers, in a row, not on each other
  await row(page, /ALIGN y \{#R2 #R3 #R2\}/).click();
  const two = (await marks()).marks;
  expect(two.map((m) => m.text)).toEqual(['1', '2', '3']);
  const same = two.filter((m) => m.text !== '2');
  expect(same[0].box.right).toBeLessThanOrEqual(same[1].box.left + 0.5);
  expect(Math.abs(same[0].box.top - same[1].box.top)).toBeLessThan(1);
});

test('the box is not in an exported image', async ({ page }) => {
  await open(page);
  await row(page, /ALIGN y/).click();
  await expect(frame(page)).toHaveCount(1);
  const download = page.waitForEvent('download');
  await page.evaluate(() => { window.viewer.command('export-svg'); });
  const svg = (await (await download).createReadStream().then(async (s) => { const parts = []; for await (const c of s) parts.push(c); return Buffer.concat(parts); })).toString();
  expect(svg.startsWith('<?xml')).toBe(true);
  expect(svg).not.toContain('constraint-box');
  expect(svg).not.toContain('ALIGN y');
  // and the page's box is there still
  await expect(frame(page)).toHaveCount(1);
});

test('another level ends the box, and a selection in the drawing does too', async ({ page }) => {
  await open(page);
  await row(page, /ALIGN y/).click();
  await expect(frame(page)).toHaveCount(1);
  const cell = await page.evaluate(() => window.viewer.doc.levels.findIndex((l) => l.title === 'cell'));
  expect(cell).toBeGreaterThan(0);
  await page.evaluate(async (level) => (await import('./subcircuits.js')).enterRoot(level, { here: true }), cell);
  await expect(frame(page)).toHaveCount(0);
  await expect(page.locator('#constraints-list li.constraint.selected')).toHaveCount(0);
  await page.evaluate(async () => (await import('./subcircuits.js')).enterRoot(0, { here: true }));
  await expect(frame(page)).toHaveCount(0);
  await row(page, /ALIGN y/).click();
  await expect(frame(page)).toHaveCount(1);
  await page.evaluate(async () => {
    const o = window.viewer.doc.objects.findIndex((x) => x.kind === 'instance');
    (await import('./selection.js')).select(o, 'list', { usage: false });
  });
  await expect(frame(page)).toHaveCount(0);
});

test('a box does not take the pointer: a click on the drawing under it selects what is there', async ({ page }) => {
  await open(page);
  await row(page, /ALIGN y/).click();
  await expect(frame(page)).toHaveCount(1);
  const point = await page.evaluate(() => {
    const svg = document.querySelector('.constraint-box').ownerSVGElement;
    const box = svg.getBoundingClientRect();
    const vb = svg.viewBox.baseVal;
    const k = box.width / vb.width;
    const level = window.viewer.doc.levels[0];
    const idx = window.viewer.doc.objects.findIndex((o) => o.kind === 'instance' && o.name === 'R2');
    const hit = level.hits.find((h) => h.kind === 'box' && h.objects[0] === idx);
    const [[x0, y0], [x1, y1]] = hit.points;
    return [box.left + ((x0 + x1) / 2 - vb.x) * k, box.top + ((y0 + y1) / 2 - vb.y) * k];
  });
  await page.mouse.click(point[0], point[1]);
  await expect.poll(() => page.evaluate(() => window.viewer.doc.objects[window.viewer.selected]?.name)).toBe('R2');
  // the selection ended the box
  await expect(frame(page)).toHaveCount(0);
});
