// View > Show Registration Points (apps/viewer-ui/origins.js): a small mark at the registration point of each instance and
// each usage with a symbol, from the `origin` of the box hits of the level shown; off to begin with and kept; one size
// whatever the zoom; no pointer events; on the netlist's drawing and on a solve's preview alike.

import { expect, test } from "../viewer.js";

const marks = (page) => page.locator("#canvas .origin-marks .origin-mark");

/** The registration points of the level shown, as the Document gives them. */
const origins = (page) => page.evaluate(() => window.viewer.doc.levels[window.viewer.level].hits.filter((h) => h.kind === "box" && h.origin).map((h) => h.origin));

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform}`, () => {
    test("it starts off, the View menu has it, and the choice is kept", async ({ page, viewer }) => {
      await viewer.open("04-hierarchy", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-origins"))).toBe(false);
      await expect(marks(page)).toHaveCount(0);
      if (platform === "web") {
        await viewer.menuTitle("View").click();
        const item = page.locator('.menu-item[data-id="toggle-origins"]');
        await expect(item.locator(".label")).toHaveText("Show Registration Points");
        await item.click();
      } else {
        await page.evaluate(() => window.fakeShell.menu("toggle-origins"));
      }
      expect(await page.evaluate(() => window.viewer.checked("toggle-origins"))).toBe(true);
      const wanted = await origins(page);
      expect(wanted.length).toBeGreaterThan(0);
      await expect(marks(page)).toHaveCount(wanted.length);
      // kept: a page opened again has them, and the shell was told the check
      await viewer.open("04-hierarchy", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-origins"))).toBe(true);
      await expect(marks(page)).toHaveCount(wanted.length);
      expect(await viewer.calls("setCheck")).toEqual(expect.arrayContaining([["toggle-origins", true]]));
      await page.evaluate(() => window.viewer.command("toggle-origins"));
      await expect(marks(page)).toHaveCount(0);
      expect(await page.evaluate(() => window.viewer.checked("toggle-origins"))).toBe(false);
    });

    test("each mark is at its registration point, at one size whatever the zoom, and takes no pointer events", async ({ page, viewer }) => {
      await viewer.open("04-hierarchy", { platform });
      await page.evaluate(() => window.viewer.command("toggle-origins"));
      const wanted = await origins(page);
      await expect(marks(page)).toHaveCount(wanted.length);
      // the mark's centre on the screen, from the view, is the point's: the drawing's SVG pixels, projected
      const centres = () => page.evaluate(() => {
        const svg = document.querySelector("#canvas > svg");
        const box = svg.getBoundingClientRect();
        const vb = svg.viewBox.baseVal;
        const k = box.width / vb.width;
        return {
          k,
          marks: [...document.querySelectorAll("#canvas .origin-marks .origin-mark")].map((g) => {
            const r = g.getBoundingClientRect();
            return [r.left + r.width / 2 - box.left, r.top + r.height / 2 - box.top, r.width, r.height];
          }),
          at: window.viewer.doc.levels[window.viewer.level].hits.filter((h) => h.kind === "box" && h.origin).map((h) => [(h.origin[0] - vb.x) * k, (h.origin[1] - vb.y) * k]),
        };
      });
      const one = await centres();
      one.marks.forEach(([x, y], i) => {
        expect(x).toBeCloseTo(one.at[i][0], 0);
        expect(y).toBeCloseTo(one.at[i][1], 0);
      });
      const size = one.marks[0].slice(2);
      // another zoom: the marks move with the drawing and keep their size
      await page.evaluate(async () => (await import("./drawing.js")).zoomCenter(3));
      const two = await centres();
      expect(two.k).not.toBeCloseTo(one.k, 3);
      two.marks.forEach(([x, y, w, h], i) => {
        expect(x).toBeCloseTo(two.at[i][0], 0);
        expect(y).toBeCloseTo(two.at[i][1], 0);
        expect(w).toBeCloseTo(size[0], 0);
        expect(h).toBeCloseTo(size[1], 0);
      });
      // no pointer events: the layer says so, and a click on a mark selects what is under it
      expect(await page.evaluate(() => getComputedStyle(document.querySelector(".origin-marks")).pointerEvents)).toBe("none");
      expect(await page.evaluate(() => document.querySelector(".origin-marks").getAttribute("pointer-events"))).toBe("none");
    });

    test("another level has the marks of its own, and the marks stay with the level when it is shown again", async ({ page, viewer }) => {
      await viewer.open("usage-current", { platform });
      await page.evaluate(() => window.viewer.command("toggle-origins"));
      const top = (await origins(page)).length;
      await expect(marks(page)).toHaveCount(top);
      await page.evaluate(async () => (await import("./subcircuits.js")).goTo(1));
      const below = (await origins(page)).length;
      expect(below).not.toBe(top);
      await expect(marks(page)).toHaveCount(below);
      await page.evaluate(async () => (await import("./subcircuits.js")).goTo(0));
      await expect(marks(page)).toHaveCount(top);
    });
  });
}

test("a solve's preview has them too", async ({ page, viewer }) => {
  await viewer.open("T4-miller-conflicts", { platform: "desktop" });
  await page.evaluate(() => window.viewer.command("toggle-origins"));
  await page.evaluate(() => window.viewer.command("solve"));
  await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"));
  const wanted = await origins(page);
  expect(wanted.length).toBeGreaterThan(0);
  await expect(marks(page)).toHaveCount(wanted.length);
  await page.evaluate(() => window.viewer.command("solve-close"));
  await expect(page.locator("#preview-badge")).toBeHidden();
  await expect.poll(async () => (await origins(page)).length).toBeGreaterThan(0);
  await expect(marks(page)).toHaveCount((await origins(page)).length);
});
