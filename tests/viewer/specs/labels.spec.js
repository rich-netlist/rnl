// Labels on the drawing (apps/viewer-ui/drawing.js, hits.js): a click on a label is a click on the
// object it is attached to, an instance or a net; a label of something that cannot be selected, such
// as a terminal, does nothing; and the labels attached to the selected object are marked with it.
// Both platforms run the same page; the fixture "pick" has R1, R2 and C1 with their name and value
// labels, and a text attached to R1's first terminal.

import { expect, test } from "../viewer.js";

const PLATFORMS = { web: {}, desktop: { platform: "desktop" } };
const TOP = "pick.sp";

/** The ids of the label marks over the drawing: one rectangle for each label of the selected object. */
const marks = (page) => page.locator("#canvas svg .overlay rect.label");

for (const [platform, query] of Object.entries(PLATFORMS)) {
  test.describe(`labels, ${platform}`, () => {
    test.beforeEach(async ({ viewer }) => {
      await viewer.open("pick", query);
    });

    test("a click on a label selects the instance it is attached to, as a click on its symbol does", async ({ page, viewer }) => {
      await viewer.click("C1");
      await viewer.selected(`instance C1 in ${TOP}`);
      const symbol = { lines: await viewer.markedLines(), places: await viewer.markedPlaces() };
      await page.keyboard.press("Escape");
      await viewer.selected(null);
      for (const id of ["#C1::name", "#C1::value"]) {
        const p = await viewer.labelWhere(id);
        await page.mouse.click(p.x, p.y);
        await viewer.selected(`instance C1 in ${TOP}`);
        // the netlist shows what it shows for the symbol
        expect({ lines: await viewer.markedLines(), places: await viewer.markedPlaces() }).toEqual(symbol);
        await page.keyboard.press("Escape");
        await viewer.selected(null);
      }
    });

    test("the cursor says that a label can be clicked, and that one of nothing cannot", async ({ page, viewer }) => {
      const p = await viewer.labelWhere("#C1::value");
      await page.mouse.move(p.x, p.y);
      await expect(page.locator("#canvas")).toHaveClass(/over-object/);
      const q = await viewer.labelWhere("a text");
      await page.mouse.move(q.x, q.y);
      await expect(page.locator("#canvas")).not.toHaveClass(/over-object/);
    });

    test("a label of something that cannot be selected does nothing, the selection stays", async ({ page, viewer }) => {
      await viewer.click("C1");
      await viewer.selected(`instance C1 in ${TOP}`);
      const p = await viewer.labelWhere("a text");
      await page.mouse.click(p.x, p.y);
      await viewer.selected(`instance C1 in ${TOP}`);
      // an empty place of the drawing clears the selection, as before
      const empty = await viewer.empty();
      await page.mouse.click(empty.x, empty.y);
      await viewer.selected(null);
    });

    test("the labels of the selected object are marked with it, and only those", async ({ page, viewer }) => {
      await expect(marks(page)).toHaveCount(0);
      await viewer.click("C1");
      await viewer.selected(`instance C1 in ${TOP}`);
      await expect(marks(page)).toHaveCount(2);
      expect(await viewer.selectedGroups()).toEqual(["#C1::name", "#C1::value", "item-2"]);
      // the marks are over the labels' places, and the labels are in the selection colour
      const label = await viewer.labelWhere("#C1::name");
      const [first] = await marks(page).evaluateAll((els) => els.map((e) => e.getBoundingClientRect()));
      const rects = await marks(page).evaluateAll((els) => els.map((e) => e.getBoundingClientRect()));
      expect(rects.some((r) => label.x >= r.left && label.x <= r.right && label.y >= r.top && label.y <= r.bottom)).toBe(true);
      expect(first.width).toBeGreaterThan(10);
      // another object takes them off, and puts its own
      await viewer.click("R1");
      await viewer.selected(`instance R1 in ${TOP}`);
      await expect(marks(page)).toHaveCount(2);
      expect(await viewer.selectedGroups()).toEqual(["#R1::name", "#R1::value", "item-0"]);
      await page.keyboard.press("Escape");
      await expect(marks(page)).toHaveCount(0);
      await expect(page.locator("#canvas svg g.sel")).toHaveCount(0);
    });
  });

  test.describe(`labels of nets, ${platform}`, () => {
    test.beforeEach(async ({ viewer }) => {
      await viewer.open("04-hierarchy", query);
    });

    test("the label of a supply symbol selects its net, and a net marks the labels of the symbols on it", async ({ page, viewer }) => {
      const vdd = await page.evaluate(() => {
        const v = window.viewer;
        return v.doc.objects.findIndex((o) => o.kind === "net" && o.name === "vdd" && o.level === v.doc.levels[v.level].host);
      });
      const labels = await page.evaluate((o) => {
        const v = window.viewer;
        return v.doc.levels[v.level].hits.filter((h) => h.kind === "label" && h.objects.includes(o)).length;
      }, vdd);
      expect(labels).toBe(3);
      // the first of them, by where it is
      const [u, w] = await page.evaluate((o) => {
        const v = window.viewer;
        const hit = v.doc.levels[v.level].hits.find((h) => h.kind === "label" && h.objects.includes(o));
        const [a, b] = hit.points;
        return [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
      }, vdd);
      const p = await viewer.point(u, w);
      await page.mouse.click(p.x, p.y);
      await viewer.selected("net vdd in 04-hierarchy.sp");
      await expect(marks(page)).toHaveCount(3);
      // the labels of the supply symbols have no group of their own: the marks are what shows them
      expect(await viewer.selectedGroups()).not.toContain("%s::name");
    });
  });
}
