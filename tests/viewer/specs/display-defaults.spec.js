// What the page shows when no setting is kept: every display aid but View > Show Names on Hover is off, as the CLI's SVG
// is; and a setting the user has kept is read as before.

import { expect, test } from "../viewer.js";

const AIDS = ["toggle-hover-roles", "toggle-usages", "toggle-currents", "toggle-origins", "toggle-flylines", "toggle-net-labels", "toggle-grid"];

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform}`, () => {
    test("nothing is shown to begin with but the names on hover", async ({ page, viewer }) => {
      await viewer.open("usage-inspector", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-hover-names"))).toBe(true);
      for (const id of AIDS) expect(await page.evaluate((id) => window.viewer.checked(id), id), id).toBe(false);
      // the grid
      expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({ shown: false });
      for (const id of ["grid-dots", "grid-lines", "grid-axes"]) expect(await page.evaluate((id) => window.viewer.checked(id), id), id).toBe(false);
      expect(await page.evaluate(() => window.viewer.checked("grid-none"))).toBe(true);
      expect(await page.evaluate(() => {
        const grid = document.getElementById("grid");
        const data = grid.getContext("2d").getImageData(0, 0, grid.width, grid.height).data;
        return data.some((v, i) => i % 4 === 3 && v > 0);
      })).toBe(false);
      // the usages' frames, the current directions, the registration points and the net names
      expect(await page.locator("#canvas .usage-frame").count()).toBe(0);
      expect(await page.locator("#canvas .current-arrow").count()).toBe(0);
      expect(await page.locator("#canvas .origin-marks").count()).toBe(0);
      expect(await page.locator("#canvas .net-labels").count()).toBe(0);
      // the flylines are in the drawing, hidden
      expect(await page.evaluate(() => document.documentElement.classList.contains("flylines-hidden"))).toBe(true);
    });

    test("a setting the user has kept is read: the usages and the grid", async ({ page, viewer }) => {
      await viewer.keep({ "usages-shown": true, "grid-shown": true, "grid-style": "lines" });
      await viewer.open("usage-inspector", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-usages"))).toBe(true);
      expect(await page.locator("#canvas .usage-frame").count()).toBeGreaterThan(0);
      expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({ shown: true, style: "lines" });
      expect(await page.evaluate(() => window.viewer.checked("grid-lines"))).toBe(true);
    });
  });
}
