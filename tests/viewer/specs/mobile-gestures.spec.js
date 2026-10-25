// Touch on the drawing, on a phone (the projects mobile-chromium and mobile-webkit): one finger drags
// the view, two pinch it around their middle, a tap selects what a click selects, a double tap does
// what a double click does, and none of it scrolls or zooms the page itself. A laptop with a touch
// screen keeps its mouse. How the fingers are made: tests/viewer/README.md#mobile.
//
// What the page does not do yet is marked test.fixme, until the Viewer's touch support lands.

import { canvasMiddle, expect, over, pageMoved, test, under, UNMOVED, view } from "../mobile.js";

const TOP = "04-hierarchy.sp";

/** Expect page points within `px` pixels of each other. */
function near(a, b, px) {
  expect(Math.abs(a.x - b.x), `x ${a.x} near ${b.x}`).toBeLessThanOrEqual(px);
  expect(Math.abs(a.y - b.y), `y ${a.y} near ${b.y}`).toBeLessThanOrEqual(px);
}

test.describe("on a phone", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy");
  });

  test("one finger drags the drawing with it, and selects nothing", async ({ page, viewer, fingers }) => {
    const before = await view(page);
    const from = await viewer.where("X1");
    const to = { x: from.x + 60, y: from.y + 40 };
    const point = await under(page, from);
    await fingers.drag(from, to);
    // the point of the drawing under the finger stays under it, at the same zoom
    near(await over(page, point), to, 1);
    expect((await view(page)).k).toBeCloseTo(before.k, 6);
    await viewer.selected(null);
    expect(await pageMoved(page)).toEqual(UNMOVED);
  });

  test("a tap selects what a click selects, and a tap on an empty place clears the selection", async ({ page, viewer, fingers }) => {
    await fingers.tap(await viewer.where("X2"));
    await viewer.selected(`instance X2 in ${TOP}`);
    // the netlist follows, as for a click
    expect(await viewer.markedLines()).toEqual([50]);
    await fingers.tap(await viewer.empty());
    await viewer.selected(null);
  });

  test.fixme("a finger that moves a few pixels while it taps still selects", async ({ page, viewer, fingers }) => {
    // a fingertip is not still: a press that moves less than about 10 pixels is a tap
    const at = await viewer.where("X2");
    await fingers.press(at);
    await fingers.move({ x: at.x + 4, y: at.y + 3 });
    await fingers.move({ x: at.x + 7, y: at.y + 2 });
    await fingers.lift();
    await viewer.selected(`instance X2 in ${TOP}`);
  });

  test.fixme("two fingers spread and pinched zoom around their middle", async ({ page, viewer, fingers }) => {
    const middle = await canvasMiddle(page);
    for (const [from, to] of [[60, 180], [180, 90]]) {
      const before = await view(page);
      const point = await under(page, middle);
      await fingers.pinch(middle, from, to);
      const after = await view(page);
      // the zoom follows the fingers' distance, and the point between them stays between them
      expect(after.k / before.k).toBeGreaterThan((to / from) * 0.95);
      expect(after.k / before.k).toBeLessThan((to / from) * 1.05);
      near(await over(page, point), middle, 2);
    }
    // the zoom shown follows
    await expect(page.locator("#zoom")).not.toHaveText("");
    await viewer.selected(null);
    expect(await pageMoved(page)).toEqual(UNMOVED);
  });

  test.fixme("two fingers that move together drag the drawing while they pinch", async ({ page, viewer, fingers }) => {
    const middle = await canvasMiddle(page);
    const point = await under(page, middle);
    const shift = { x: 50, y: -40 };
    await fingers.pinch(middle, 80, 120, { shift });
    near(await over(page, point), { x: middle.x + shift.x, y: middle.y + shift.y }, 2);
  });

  test.fixme("a finger lifted from a pinch leaves the other dragging, with no jump", async ({ page, viewer, fingers }) => {
    const middle = await canvasMiddle(page);
    const left = { x: middle.x - 40, y: middle.y };
    const right = { x: middle.x + 40, y: middle.y };
    await fingers.press(left, right);
    await fingers.move({ x: left.x - 30, y: left.y }, { x: right.x + 30, y: right.y });
    const pinched = await view(page);
    await fingers.lift(1);
    // lifting the second finger moves nothing
    const lifted = await view(page);
    expect(lifted.x).toBeCloseTo(pinched.x, 6);
    expect(lifted.y).toBeCloseTo(pinched.y, 6);
    expect(lifted.k).toBeCloseTo(pinched.k, 6);
    // the one left drags the drawing under it
    const from = { x: left.x - 30, y: left.y };
    const point = await under(page, from);
    const to = { x: from.x + 50, y: from.y + 60 };
    await fingers.move({ x: from.x + 25, y: from.y + 30 });
    await fingers.move(to);
    near(await over(page, point), to, 1);
    await fingers.lift();
    await viewer.selected(null);
  });

  test.fixme("two fingers that tap select nothing", async ({ page, viewer, fingers }) => {
    const at = await viewer.where("X2");
    await fingers.press(at, { x: at.x + 30, y: at.y });
    await fingers.lift();
    await viewer.selected(null);
  });

  test("a double tap on an empty place fits the drawing", async ({ page, viewer, fingers, browserName }) => {
    test.fixme(browserName === "webkit", "WebKit makes no dblclick of two taps: the page has to tell a double tap itself");
    const fitted = await viewer.viewBox();
    await page.evaluate(() => window.viewer.command("zoom-in"));
    await page.evaluate(() => window.viewer.command("zoom-in"));
    await expect.poll(() => viewer.viewBox()).not.toBe(fitted);
    await fingers.doubleTap(await viewer.empty());
    await expect.poll(() => viewer.viewBox()).toBe(fitted);
  });

  test("a double tap on a subcircuit instance opens it, once", async ({ page, viewer, fingers, browserName }) => {
    test.fixme(browserName === "webkit", "WebKit makes no dblclick of two taps: the page has to tell a double tap itself");
    await fingers.doubleTap(await viewer.where("X3"));
    await expect(page.locator("#level-name")).toHaveText("rdiv");
    await expect(page.locator("#up-name")).toHaveText(TOP);
    // one level down, not two, and the level opened stays fitted
    expect(await page.evaluate(() => window.viewer.path.length)).toBe(2);
    // the up button is the way back
    await fingers.tap(await viewer.middle(page.locator("#up")));
    await expect(page.locator("#level-name")).toHaveText(TOP);
  });

  test("gestures on the drawing neither scroll nor zoom the page", async ({ page, viewer, fingers, browserName }) => {
    test.skip(browserName !== "chromium", "only Chromium's touch goes through the browser; WebKit's is made by the test");
    const middle = await canvasMiddle(page);
    await fingers.drag(middle, { x: middle.x, y: middle.y + 150 });
    await fingers.drag(middle, { x: middle.x - 120, y: middle.y });
    await fingers.pinch(middle, 50, 200);
    await fingers.pinch(middle, 200, 50);
    await fingers.doubleTap(await viewer.empty());
    await expect.poll(() => pageMoved(page)).toEqual(UNMOVED);
  });

  test("a long press on the drawing opens no menu and selects no text", async ({ page, viewer, fingers }) => {
    await page.evaluate(() => {
      window.menus = 0;
      document.addEventListener("contextmenu", (e) => { if (!e.defaultPrevented) window.menus += 1; });
    });
    const at = await viewer.where("X2");
    await fingers.press(at);
    await page.waitForTimeout(900); // a long press is a time
    await fingers.lift();
    expect(await page.evaluate(() => window.menus)).toBe(0);
    expect(await page.evaluate(() => String(window.getSelection()))).toBe("");
  });
});

test.describe("on a laptop with a touch screen", () => {
  test.use({ isMobile: false, hasTouch: true, viewport: { width: 1200, height: 800 }, deviceScaleFactor: 1 });

  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy");
  });

  test("the mouse drags, zooms by the wheel and clicks as on a desktop", async ({ page, viewer }) => {
    const before = await view(page);
    const from = await viewer.where("X1");
    await viewer.drag(from, { x: from.x + 120, y: from.y + 40 });
    const dragged = await view(page);
    expect(dragged.x).toBeCloseTo(before.x - 120 / before.k, 6);
    expect(dragged.y).toBeCloseTo(before.y - 40 / before.k, 6);
    // the wheel zooms around the pointer, so what is under it still is
    const at = await viewer.where("X1");
    await page.mouse.move(at.x, at.y);
    await page.mouse.wheel(0, -300);
    await expect.poll(async () => (await view(page)).k).toBeGreaterThan(dragged.k);
    await viewer.click("X1");
    await viewer.selected(`instance X1 in ${TOP}`);
  });

  test("a finger drags and taps too", async ({ page, viewer, fingers }) => {
    const from = await viewer.where("X1");
    const point = await under(page, from);
    const to = { x: from.x - 80, y: from.y + 30 };
    await fingers.drag(from, to);
    near(await over(page, point), to, 1);
    await fingers.tap(await viewer.where("X2"));
    await viewer.selected(`instance X2 in ${TOP}`);
  });

  test.fixme("two fingers pinch the drawing, and the wheel zooms after them", async ({ page, viewer, fingers }) => {
    const middle = await canvasMiddle(page);
    const before = await view(page);
    await fingers.pinch(middle, 80, 160);
    const pinched = await view(page);
    expect(pinched.k / before.k).toBeGreaterThan(1.9);
    await page.mouse.move(middle.x, middle.y);
    await page.mouse.wheel(0, -300);
    await expect.poll(async () => (await view(page)).k).toBeGreaterThan(pinched.k);
  });
});
