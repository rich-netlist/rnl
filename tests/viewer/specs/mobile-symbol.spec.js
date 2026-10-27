// Touch on the symbol preview of the Symbol view, on a phone (the projects mobile-chromium and
// mobile-webkit): the preview takes the gestures of the drawing, by the same code (touch-gestures.js):
// one finger drags it, two pinch it around their middle and drag it as they move, a double tap fits
// it; its buttons are tapped as before; and none of it scrolls or zooms the page itself. The mouse and
// the wheel on it are tested in inspector.spec.js. How the fingers are made: tests/viewer/README.md#mobile.

import { expect, pageMoved, test, UNMOVED } from "../mobile.js";

/** The preview's view box, `x`, `y`, `w` and `h` in its units, `k` the page pixels of a unit, and its box `r`. */
function preview(page) {
  return page.evaluate(() => {
    const el = document.querySelector(".symbol-preview");
    const [x, y, w, h] = el.querySelector(":scope > svg").getAttribute("viewBox").split(/\s+/).map(Number);
    const b = el.getBoundingClientRect();
    return { x, y, w, h, k: el.clientWidth / w, r: { x: b.left, y: b.top, width: b.width, height: b.height } };
  });
}
const under = async (page, p) => {
  const v = await preview(page);
  return { u: v.x + (p.x - v.r.x) / v.k, v: v.y + (p.y - v.r.y) / v.k };
};
const over = async (page, { u, v: w }) => {
  const v = await preview(page);
  return { x: v.r.x + (u - v.x) * v.k, y: v.r.y + (w - v.y) * v.k };
};
const middle = async (page) => {
  const { r } = await preview(page);
  return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
};
function near(a, b, px) {
  expect(Math.abs(a.x - b.x), `x ${a.x} near ${b.x}`).toBeLessThanOrEqual(px);
  expect(Math.abs(a.y - b.y), `y ${a.y} near ${b.y}`).toBeLessThanOrEqual(px);
}

test.describe("on a phone", () => {
  test.beforeEach(async ({ page, viewer }) => {
    await viewer.open("symbol-inspector");
    // an instance is opened, and the side panel's page, on its Symbol view, shows its symbol
    await viewer.click("Xparent");
    await page.keyboard.press("Enter");
    await page.locator("#page-panel").tap();
    await page.locator("#tab-symbol").tap();
    await viewer.symbolFitted();
    await page.locator(".symbol-preview").scrollIntoViewIfNeeded();
  });

  test("two fingers spread and pinched zoom the preview around their middle, and the page stays as it is", async ({ page, fingers }) => {
    const at = await middle(page);
    for (const [from, to] of [[40, 120], [120, 60]]) {
      const before = await preview(page);
      const point = await under(page, at);
      await fingers.pinch(at, from, to);
      const after = await preview(page);
      // the zoom follows the fingers' distance, and the point between them stays between them
      expect(after.k / before.k).toBeGreaterThan((to / from) * 0.95);
      expect(after.k / before.k).toBeLessThan((to / from) * 1.05);
      near(await over(page, point), at, 2);
    }
    expect(await pageMoved(page)).toEqual(UNMOVED);
  });

  test("two fingers that move together drag the preview while they pinch", async ({ page, fingers }) => {
    const at = await middle(page);
    const point = await under(page, at);
    const shift = { x: 30, y: -20 };
    await fingers.pinch(at, 50, 80, { shift });
    near(await over(page, point), { x: at.x + shift.x, y: at.y + shift.y }, 2);
  });

  test("one finger drags the preview with it, at the same zoom", async ({ page, fingers }) => {
    const before = await preview(page);
    const from = await middle(page);
    const to = { x: from.x + 40, y: from.y + 25 };
    const point = await under(page, from);
    await fingers.drag(from, to);
    // the point of the symbol under the finger stays under it
    near(await over(page, point), to, 1);
    expect((await preview(page)).k).toBeCloseTo(before.k, 6);
    expect(await pageMoved(page)).toEqual(UNMOVED);
  });

  test("a double tap fits the preview again", async ({ page, fingers }) => {
    const fitted = await page.locator(".symbol-preview > svg").getAttribute("viewBox");
    await fingers.pinch(await middle(page), 40, 120);
    await expect(page.locator(".symbol-preview > svg")).not.toHaveAttribute("viewBox", fitted);
    await fingers.doubleTap(await middle(page));
    await expect(page.locator(".symbol-preview > svg")).toHaveAttribute("viewBox", fitted);
  });

  test("a tap on one of the preview's buttons zooms it, and is no gesture", async ({ page, fingers }) => {
    const before = await preview(page);
    await page.locator('.symbol-preview [data-action="zoom-in"]').tap();
    const after = await preview(page);
    expect(after.k / before.k).toBeCloseTo(1.25, 2);
  });

  test("the gestures neither scroll nor zoom the page", async ({ page, fingers, browserName }) => {
    test.skip(browserName !== "chromium", "only Chromium's touch goes through the browser; WebKit's is made by the test");
    const at = await middle(page);
    await fingers.drag(at, { x: at.x, y: at.y + 60 });
    await fingers.drag(at, { x: at.x - 50, y: at.y });
    await fingers.pinch(at, 40, 140);
    await fingers.pinch(at, 140, 40);
    await expect.poll(() => pageMoved(page)).toEqual(UNMOVED);
  });
});
