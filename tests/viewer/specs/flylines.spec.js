// View > Show Flylines (apps/viewer-ui/flylines.js): the flylines of the level shown, which the core draws as groups of class
// "flyline"; off to begin with and kept; hidden by a class of the page, not drawn again.

import { expect, test } from "../viewer.js";

const lines = (page) => page.locator("#canvas g.flyline");
/** The display of the first flyline: "none" when the page hides it. */
const display = (page) => page.evaluate(() => getComputedStyle(document.querySelector("#canvas g.flyline")).display);

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform}`, () => {
    test("it starts off, the View menu has it, and the choice is kept", async ({ page, viewer }) => {
      await viewer.open("flylines", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-flylines"))).toBe(false);
      await expect(lines(page).first()).toBeAttached();
      expect(await display(page)).toBe("none");
      if (platform === "web") {
        await viewer.menuTitle("View").click();
        const item = page.locator('.menu-item[data-id="toggle-flylines"]');
        await expect(item.locator(".label")).toHaveText("Show Flylines");
        await item.click();
      } else {
        await page.evaluate(() => window.fakeShell.menu("toggle-flylines"));
      }
      expect(await page.evaluate(() => window.viewer.checked("toggle-flylines"))).toBe(true);
      expect(await display(page)).not.toBe("none");
      // kept: a page opened again has them shown, and the shell was told the check
      await viewer.open("flylines", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-flylines"))).toBe(true);
      expect(await display(page)).not.toBe("none");
      expect(await viewer.calls("setCheck")).toEqual(expect.arrayContaining([["toggle-flylines", true]]));
      await page.evaluate(() => window.viewer.command("toggle-flylines"));
      expect(await page.evaluate(() => window.viewer.checked("toggle-flylines"))).toBe(false);
      expect(await display(page)).toBe("none");
    });

    test("showing and hiding them draws nothing again: the same groups stay in the drawing", async ({ page, viewer }) => {
      await viewer.open("flylines", { platform });
      const before = await lines(page).count();
      expect(before).toBeGreaterThan(0);
      const builds = await viewer.builds();
      await page.evaluate(() => window.viewer.command("toggle-flylines"));
      expect(await lines(page).count()).toBe(before);
      await page.evaluate(() => window.viewer.command("toggle-flylines"));
      expect(await lines(page).count()).toBe(before);
      expect(await viewer.builds()).toBe(builds);
    });
  });
}
