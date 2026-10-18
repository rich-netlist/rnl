// A click on the drawing that is on several objects (apps/viewer-ui/picker.js, hits.js): a list opens
// beside the pointer with a row for each, its kind and its name, to choose from with the mouse or the
// keys (Up and Down, Home and End, Enter, Escape); a click on one object selects it at once. In the
// fixture "pick" the box of R2 overlaps R1's, R1's name label is inside R2's box, and a wire of net
// c runs through C1's box.

import { expect, test } from "../viewer.js";

const PLATFORMS = { web: {}, desktop: { platform: "desktop" } };
const TOP = "pick.sp";

const list = (page) => page.locator("#pick-list");
const rows = (page) => page.locator("#pick-list .pick-row");

/** The rows of the list as "kind name". */
const texts = async (page) => (await rows(page).allTextContents()).map((t) => t.replace(/^(instance|net)/, "$1 "));

/** The row the keys act on. */
const active = (page) => page.locator('#pick-list .pick-row[aria-selected="true"]');

for (const [platform, query] of Object.entries(PLATFORMS)) {
  test.describe(`several objects under a click, ${platform}`, () => {
    test.beforeEach(async ({ viewer }) => {
      await viewer.open("pick", query);
    });

    test("a click on two instances lists them beside the pointer, with their kinds, and selects nothing yet", async ({ page, viewer }) => {
      const p = await viewer.overlap("R1", "R2");
      await page.mouse.click(p.x, p.y);
      await expect(list(page)).toBeVisible();
      expect(await texts(page)).toEqual(["instance R1", "instance R2"]);
      await viewer.selected(null);
      // beside the pointer and in the window, not under the pointer
      const box = await list(page).boundingBox();
      expect(box.x - p.x).toBeGreaterThanOrEqual(0);
      expect(box.x - p.x).toBeLessThan(30);
      expect(box.y - p.y).toBeGreaterThanOrEqual(0);
      expect(box.y - p.y).toBeLessThan(30);
      const size = page.viewportSize();
      expect(box.x + box.width).toBeLessThanOrEqual(size.width);
      expect(box.y + box.height).toBeLessThanOrEqual(size.height);
      // the first row is the one Enter takes, and the list has the focus
      await expect(active(page)).toHaveText("instanceR1");
      await expect(list(page)).toBeFocused();
    });

    test("a click on a row selects its object and closes the list", async ({ page, viewer }) => {
      const p = await viewer.overlap("R1", "R2");
      await page.mouse.click(p.x, p.y);
      await rows(page).nth(1).click();
      await expect(list(page)).toBeHidden();
      await viewer.selected(`instance R2 in ${TOP}`);
      // the netlist follows, as for a click on the symbol
      expect(await viewer.markedLines()).toHaveLength(1);
      await page.mouse.click(p.x, p.y);
      await rows(page).nth(0).click();
      await viewer.selected(`instance R1 in ${TOP}`);
    });

    test("the keys choose: Down and Up move, Home and End jump, Enter selects, and the drawing has the focus again", async ({ page, viewer }) => {
      const p = await viewer.overlap("R1", "R2");
      await page.mouse.click(p.x, p.y);
      await expect(active(page)).toHaveText("instanceR1");
      await page.keyboard.press("ArrowDown");
      await expect(active(page)).toHaveText("instanceR2");
      // past the last row is the first, and before the first the last
      await page.keyboard.press("ArrowDown");
      await expect(active(page)).toHaveText("instanceR1");
      await page.keyboard.press("ArrowUp");
      await expect(active(page)).toHaveText("instanceR2");
      await page.keyboard.press("Home");
      await expect(active(page)).toHaveText("instanceR1");
      await page.keyboard.press("End");
      await expect(active(page)).toHaveText("instanceR2");
      await page.keyboard.press("Enter");
      await expect(list(page)).toBeHidden();
      await viewer.selected(`instance R2 in ${TOP}`);
      await expect(page.locator("#canvas")).toBeFocused();
    });

    test("the pointer over a row makes it the one Enter takes", async ({ page, viewer }) => {
      const p = await viewer.overlap("R1", "R2");
      await page.mouse.click(p.x, p.y);
      await rows(page).nth(1).hover();
      await expect(active(page)).toHaveText("instanceR2");
      await page.keyboard.press("Enter");
      await viewer.selected(`instance R2 in ${TOP}`);
    });

    test("Escape closes the list and keeps the selection; the Escape after it clears the selection", async ({ page, viewer }) => {
      await viewer.click("C1");
      await viewer.selected(`instance C1 in ${TOP}`);
      const p = await viewer.overlap("R1", "R2");
      await page.mouse.click(p.x, p.y);
      await expect(list(page)).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(list(page)).toBeHidden();
      await viewer.selected(`instance C1 in ${TOP}`);
      await expect(page.locator("#canvas")).toBeFocused();
      await page.keyboard.press("Escape");
      await viewer.selected(null);
    });

    test("a click elsewhere closes the list, and does what it does", async ({ page, viewer }) => {
      await viewer.click("C1");
      const p = await viewer.overlap("R1", "R2");
      await page.mouse.click(p.x, p.y);
      await expect(list(page)).toBeVisible();
      const empty = await viewer.empty();
      await page.mouse.click(empty.x, empty.y);
      await expect(list(page)).toBeHidden();
      await viewer.selected(null);
    });

    test("a label and the instance under it are two objects, a label and its own symbol are one", async ({ page, viewer }) => {
      // R1's name is inside R2's box: R1 first, the label being a click's first choice
      const p = await viewer.overlap("#R1::name", "R2");
      await page.mouse.click(p.x, p.y);
      expect(await texts(page)).toEqual(["instance R1", "instance R2"]);
      await page.keyboard.press("Enter");
      await viewer.selected(`instance R1 in ${TOP}`);
      // R1's value is no overlap, and its own box has no label in it
      const q = await viewer.where("R1");
      await page.mouse.click(q.x, q.y);
      await expect(list(page)).toBeHidden();
      await viewer.selected(`instance R1 in ${TOP}`);
    });

    test("one object under the click is selected at once, with no list", async ({ page, viewer }) => {
      for (const name of ["C1", "R1"]) {
        const p = await viewer.where(name);
        await page.mouse.click(p.x, p.y);
        await expect(list(page)).toBeHidden();
        await viewer.selected(`instance ${name} in ${TOP}`);
      }
      const label = await viewer.labelWhere("#C1::value");
      await page.mouse.click(label.x, label.y);
      await expect(list(page)).toBeHidden();
      await viewer.selected(`instance C1 in ${TOP}`);
    });

    test("the list is gone when another level is shown or the document is read again", async ({ page, viewer }) => {
      const p = await viewer.overlap("R1", "R2");
      await page.mouse.click(p.x, p.y);
      await expect(list(page)).toBeVisible();
      await viewer.build(() => page.evaluate(() => window.viewer.command("reload")));
      await expect(list(page)).toBeHidden();
    });
  });

  test.describe(`a net and an instance under a click, ${platform}`, () => {
    test("a wire over a symbol lists the net first and the instance, and the wire off the symbol is the net alone", async ({ page, viewer }) => {
      await viewer.open("pick", query);
      const over = await viewer.where("c", "wire");
      await page.mouse.click(over.x, over.y);
      await expect(list(page)).toBeVisible();
      expect(await texts(page)).toEqual(["net c", "instance C1"]);
      await page.keyboard.press("Enter");
      await viewer.selected(`net c in ${TOP}`);
      // the same wire outside the box has no instance under it
      const off = await page.evaluate(() => {
        const level = window.viewer.doc.levels[window.viewer.level];
        const wire = level.hits.find((h) => h.kind === "wire");
        const box = level.hits.find((h) => h.kind === "box" && h.objects.some((o) => window.viewer.doc.objects[o].name === "C1"));
        return [(wire.points[0][0] + box.points[0][0]) / 2, wire.points[0][1]];
      });
      const p = await viewer.point(...off);
      await page.mouse.click(p.x, p.y);
      await expect(list(page)).toBeHidden();
      await viewer.selected(`net c in ${TOP}`);
    });

    test("a pin is on its net, not on the symbol it is a part of: the net is selected at once", async ({ page, viewer }) => {
      await viewer.open("04-hierarchy", query);
      const pin = await viewer.where("a", "pin");
      await page.mouse.click(pin.x, pin.y);
      await expect(list(page)).toBeHidden();
      await viewer.selected("net a in 04-hierarchy.sp");
    });
  });
}
