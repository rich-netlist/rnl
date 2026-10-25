// The web viewer (apps/web) on a phone (the projects mobile-chromium and mobile-webkit): its site, as
// the web viewer's smoke tests use it, with the web shell and the real core as WebAssembly, opens a
// sample by its address, fits on the screen, and a finger drags the drawing and taps it.

import { canvasMiddle, expect, over, pageMoved, test, under, UNMOVED } from "../mobile.js";

test.skip(!process.env.RNL_WEB_URL, "the site is not built: RNL_WEB=0");

test.beforeEach(async ({ page }) => {
  await page.route("https://rich-netlist.github.io/rnl-packages/**", (route) => route.abort());
});

test("a sample opens by its address, fits on the screen, and a finger drags and taps the drawing", async ({ page, fingers }) => {
  await page.goto(`${process.env.RNL_WEB_URL}/?url=samples/cases/04-hierarchy.sp`);
  await page.waitForFunction(() => window.viewer && window.viewer.doc && !document.body.classList.contains("busy"));
  await expect(page).toHaveTitle(/^04-hierarchy\.sp/);
  await expect(page.locator("#menubar")).toBeVisible();
  await expect(page.locator("#canvas > svg")).toBeVisible();
  expect(await pageMoved(page)).toEqual(UNMOVED);

  const from = await canvasMiddle(page);
  const point = await under(page, from);
  const to = { x: from.x + 40, y: from.y - 50 };
  await fingers.drag(from, to);
  const after = await over(page, point);
  expect(Math.abs(after.x - to.x)).toBeLessThanOrEqual(1);
  expect(Math.abs(after.y - to.y)).toBeLessThanOrEqual(1);
  expect(await pageMoved(page)).toEqual(UNMOVED);

  // a tap on an instance selects it, as a click does; whether the status bar's name of it fits is
  // mobile-layout.spec.js's question
  const at = await page.evaluate(() => {
    const doc = window.viewer.doc;
    const hit = doc.levels[window.viewer.level].hits.find((h) => h.kind === "box" && doc.objects[h.objects[0]].name === "X2");
    const [a, b] = hit.points;
    return { u: (a[0] + b[0]) / 2, v: (a[1] + b[1]) / 2 };
  });
  await fingers.tap(await over(page, at));
  await expect.poll(() => page.evaluate(() => window.viewer.selected !== null && window.viewer.doc.objects[window.viewer.selected].name)).toBe("X2");
});
