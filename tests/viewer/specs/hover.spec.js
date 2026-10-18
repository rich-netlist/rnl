// The name beside the pointer while it rests on an instance or a net of the drawing
// (apps/viewer-ui/hover.js), with View > Show Names on Hover to turn it off. The box is beside the
// pointer, never under it, takes no clicks, and goes when the pointer leaves what it names. A
// virtual part, such as a ground or a port, shows its own name only when the netlist names the
// part: the ground symbols of 04-hierarchy have no name of their own, the ports of the OTA have.
// Both platforms, the web viewer's menu bar and the desktop's native menu, have the item.

import { expect, test } from "../viewer.js";

const PLATFORMS = { web: {}, desktop: { platform: "desktop" } };

const tip = (page) => page.locator("#hover-tip");

/** Whether the box is clear of the page point: the pointer is not under it. */
function clearOf(box, p) {
  return p.x < box.x || p.x > box.x + box.width || p.y < box.y || p.y > box.y + box.height;
}

for (const [platform, query] of Object.entries(PLATFORMS)) {
  test.describe(`names on hover, ${platform}`, () => {
    test.beforeEach(async ({ viewer }) => {
      await viewer.open("04-hierarchy", query);
    });

    test("an instance shows its name beside the pointer, not under it", async ({ page, viewer }) => {
      const p = await viewer.where("X2");
      await page.mouse.move(p.x, p.y);
      await expect(tip(page)).toBeVisible();
      await expect(tip(page)).toHaveText("X2");
      const box = await tip(page).boundingBox();
      expect(clearOf(box, p)).toBe(true);
      // beside it, and near
      expect(box.x - p.x).toBeGreaterThan(0);
      expect(box.x - p.x).toBeLessThan(40);
      expect(box.y - p.y).toBeGreaterThan(0);
      expect(box.y - p.y).toBeLessThan(50);
      // it takes no clicks: a click through it is the drawing's
      expect(await tip(page).evaluate((el) => getComputedStyle(el).pointerEvents)).toBe("none");
    });

    test("the box goes with the pointer, and goes when the pointer leaves the instance", async ({ page, viewer }) => {
      const p = await viewer.where("X2");
      await page.mouse.move(p.x, p.y);
      await expect(tip(page)).toHaveText("X2");
      const before = await tip(page).boundingBox();
      await page.mouse.move(p.x + 9, p.y + 5);
      await expect(tip(page)).toHaveText("X2");
      const after = await tip(page).boundingBox();
      expect(Math.abs(after.x - before.x - 9)).toBeLessThan(1.5);
      expect(Math.abs(after.y - before.y - 5)).toBeLessThan(1.5);
      const empty = await viewer.empty();
      await page.mouse.move(empty.x, empty.y);
      await expect(tip(page)).toBeHidden();
    });

    test("a wire and a pin show the name of their net", async ({ page, viewer }) => {
      await page.mouse.move(...Object.values(await viewer.where("b", "wire")));
      await expect(tip(page)).toHaveText("b");
      await page.mouse.move(...Object.values(await viewer.where("vdd", "pin")));
      await expect(tip(page)).toHaveText("vdd");
    });

    test("a ground that the netlist does not name shows nothing, a label shows nothing, the drawing's empty parts show nothing", async ({ page, viewer }) => {
      await page.mouse.move(...Object.values(await viewer.where("0", "box")));
      await expect(tip(page)).toBeHidden();
      await page.mouse.move(...Object.values(await viewer.labelWhere("#X1::name")));
      await expect(tip(page)).toBeHidden();
      const empty = await viewer.empty();
      await page.mouse.move(empty.x, empty.y);
      await expect(tip(page)).toBeHidden();
    });

    test("a press, the wheel and leaving the drawing hide it", async ({ page, viewer }) => {
      const p = await viewer.where("X2");
      await page.mouse.move(p.x, p.y);
      await expect(tip(page)).toBeVisible();
      await page.mouse.down();
      await expect(tip(page)).toBeHidden();
      await page.mouse.up();
      await page.mouse.move(p.x + 2, p.y + 2);
      await expect(tip(page)).toBeVisible();
      await page.mouse.wheel(0, -60);
      await expect(tip(page)).toBeHidden();
      const canvas = await page.locator("#canvas").boundingBox();
      const q = await viewer.where("X2");
      await page.mouse.move(q.x, q.y);
      await expect(tip(page)).toBeVisible();
      await page.mouse.move(canvas.x - 8, canvas.y + 40);
      await expect(tip(page)).toBeHidden();
    });

    test("a virtual part that the netlist names shows its name", async ({ page, viewer }) => {
      await viewer.open("03-five-transistor-ota", query);
      // the ports of the OTA are %pi, %pn and %po, on the nets inp, inn and out
      const named = await page.evaluate(() => {
        const level = window.viewer.doc.levels[0];
        return level.hits.filter((h) => h.kind === "box" && h.name).map((h) => [h.name, window.viewer.doc.objects[h.objects[0]].name]);
      });
      expect(named.sort()).toEqual([["pi", "inp"], ["pn", "inn"], ["po", "out"]]);
      const [u, v] = await page.evaluate(() => {
        const hit = window.viewer.doc.levels[0].hits.find((h) => h.kind === "box" && h.name === "pi");
        const [a, b] = hit.points;
        return [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
      });
      const p = await viewer.point(u, v);
      await page.mouse.move(p.x, p.y);
      await expect(tip(page)).toHaveText("pi");
    });

    test("View > Show Names on Hover is on to begin with, turns it off, and the choice is kept", async ({ page, viewer }) => {
      /** The item's check mark where the platform shows it: the page's menu bar or the native menu. */
      const mark = async () => {
        if (platform === "desktop") return page.evaluate(() => window.fakeShell.checks["toggle-hover-names"]);
        await viewer.menuTitle("View").click();
        const checked = await viewer.menuItem("toggle-hover-names").getAttribute("aria-checked");
        await page.keyboard.press("Escape");
        return checked === "true";
      };
      expect(await mark()).toBe(true);
      expect(await page.evaluate(() => window.viewer.checked("toggle-hover-names"))).toBe(true);
      const turnOff = async () => {
        if (platform === "desktop") await page.evaluate(() => window.fakeShell.menu("toggle-hover-names"));
        else {
          await viewer.menuTitle("View").click();
          await viewer.menuItem("toggle-hover-names").click();
        }
      };
      await turnOff();
      expect(await mark()).toBe(false);
      const p = await viewer.where("X2");
      await page.mouse.move(p.x, p.y);
      await page.mouse.move(p.x + 3, p.y + 3);
      await expect(tip(page)).toBeHidden();
      // the next start has the choice
      await page.reload();
      await viewer.started();
      await viewer.idle();
      expect(await page.evaluate(() => window.viewer.checked("toggle-hover-names"))).toBe(false);
      const q = await viewer.where("X2");
      await page.mouse.move(q.x, q.y);
      await page.mouse.move(q.x + 3, q.y + 3);
      await expect(tip(page)).toBeHidden();
      // and turned on again, the names come back
      await turnOff();
      expect(await mark()).toBe(true);
      await page.mouse.move(q.x, q.y);
      await expect(tip(page)).toHaveText("X2");
    });
  });
}
