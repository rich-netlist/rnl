// The view of the drawing: fitting it, zooming around the middle by the keys and around the pointer
// by the wheel, within the limits of the zoom.

import { expect, test } from "../viewer.js";

const TOP = "04-hierarchy.sp";

/** The view box as numbers: x, y, width and height. */
async function box(viewer) {
  return (await viewer.viewBox()).split(" ").map(Number);
}

/** The middle of a view box. */
const middle = (b) => [b[0] + b[2] / 2, b[1] + b[3] / 2];

test.beforeEach(async ({ viewer }) => {
  await viewer.open("04-hierarchy");
});

test("Ctrl+=, Ctrl++ and Ctrl+- zoom around the middle, and F and 0 fit the drawing", async ({ page, viewer }) => {
  const fitted = await viewer.viewBox();
  const width = await page.locator("#canvas").evaluate((c) => c.clientWidth);
  for (const [key, factor] of [["Control+=", 1.25], ["Control++", 1.25], ["Control+-", 0.8], ["Control+-", 0.8]]) {
    const before = await box(viewer);
    await page.keyboard.press(key);
    await expect.poll(async () => (await box(viewer))[2]).toBeCloseTo(before[2] / factor, 6);
    const after = await box(viewer);
    expect(after[3]).toBeCloseTo(before[3] / factor, 6);
    expect(middle(after)[0]).toBeCloseTo(middle(before)[0], 6);
    expect(middle(after)[1]).toBeCloseTo(middle(before)[1], 6);
    await expect(page.locator("#zoom")).toHaveText(`${Math.round((width / after[2]) * 100)}%`);
  }
  await page.keyboard.press("f");
  await expect.poll(() => viewer.viewBox()).toBe(fitted);
  await page.keyboard.press("Control+=");
  await expect.poll(() => viewer.viewBox()).not.toBe(fitted);
  await page.keyboard.press("0");
  await expect.poll(() => viewer.viewBox()).toBe(fitted);
});

test("the fitted drawing lies within the canvas", async ({ page, viewer }) => {
  const extent = await page.evaluate(() => {
    const v = window.viewer;
    const svg = new DOMParser().parseFromString(v.doc.levels[v.level].svg, "image/svg+xml").documentElement;
    return svg.getAttribute("viewBox").split(/[\s,]+/).map(Number);
  });
  const [x, y, w, h] = await box(viewer);
  expect(x).toBeLessThanOrEqual(extent[0]);
  expect(y).toBeLessThanOrEqual(extent[1]);
  expect(x + w).toBeGreaterThanOrEqual(extent[0] + extent[2]);
  expect(y + h).toBeGreaterThanOrEqual(extent[1] + extent[3]);
  // and fills it one way
  expect(Math.min(w - extent[2], h - extent[3]) / Math.max(w, h)).toBeLessThan(0.1);
});

test("the wheel zooms around the pointer, and a double click on an empty place fits", async ({ page, viewer }) => {
  const fitted = await viewer.viewBox();
  const at = await viewer.where("X2");
  const p = { x: Math.round(at.x), y: Math.round(at.y) };
  // the point of the drawing under the pointer
  const [b, r] = [await box(viewer), await page.locator("#canvas").boundingBox()];
  const k = r.width / b[2];
  const [u, v] = [b[0] + (p.x - r.x) / k, b[1] + (p.y - r.y) / k];
  await page.mouse.move(p.x, p.y);
  await page.mouse.wheel(0, -300);
  await expect.poll(() => viewer.viewBox()).not.toBe(fitted);
  const zoomedIn = await box(viewer);
  expect(zoomedIn[2]).toBeLessThan(b[2]);
  // stays under it, within a pixel
  const after = await viewer.point(u, v);
  expect(Math.abs(after.x - p.x)).toBeLessThan(1);
  expect(Math.abs(after.y - p.y)).toBeLessThan(1);
  // so what was under the pointer still is
  await page.mouse.click(p.x, p.y);
  await viewer.selected(`instance X2 in ${TOP}`);

  await page.mouse.wheel(0, 600);
  await expect.poll(async () => (await box(viewer))[2]).toBeGreaterThan(zoomedIn[2]);
  const empty = await viewer.empty();
  await page.mouse.dblclick(empty.x, empty.y);
  await expect.poll(() => viewer.viewBox()).toBe(fitted);
  await viewer.selected(null);
});

test("the zoom stops at its limits", async ({ page }) => {
  for (let i = 0; i < 40; i++) await page.keyboard.press("Control+=");
  await expect(page.locator("#zoom")).toHaveText("5000%");
  for (let i = 0; i < 80; i++) await page.keyboard.press("Control+-");
  await expect(page.locator("#zoom")).toHaveText("2%");
});
