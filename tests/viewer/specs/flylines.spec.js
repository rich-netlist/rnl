// View > Show Flylines (apps/viewer-ui/flylines.js): the flylines of the level shown, which the core draws as groups of class
// "flyline"; off to begin with and kept; hidden by a class of the page, not drawn again, and then not under the pointer.

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

    test("a shown flyline is named on hover and selected as a wire is, and the net's selection lights it up", async ({ page, viewer }) => {
      await viewer.open("flylines", { platform });
      await page.evaluate(() => window.viewer.command("toggle-flylines"));
      const title = await page.evaluate(() => window.viewer.doc.levels[window.viewer.level].title);
      const id = await page.evaluate(() => window.viewer.doc.levels[window.viewer.level].hits.find((h) => h.kind === "flyline").id);
      const p = await viewer.where("b", "flyline");
      await page.mouse.move(0, 0);
      await page.mouse.move(p.x, p.y);
      await expect(page.locator("#hover-tip")).toHaveText("b");
      await expect(page.locator("#canvas")).toHaveClass(/over-object/);
      await page.mouse.click(p.x, p.y);
      await viewer.selected(`net b in ${title}`);
      expect(await viewer.selectedGroups()).toContain(id);
      // the net selected by one of its pins lights the flyline up too
      await page.evaluate(() => window.viewer.command("fit"));
      await viewer.click("a", "pin");
      await viewer.selected(`net a in ${title}`);
      expect(await viewer.selectedGroups()).not.toContain(id);
      await viewer.click("b", "pin");
      await viewer.selected(`net b in ${title}`);
      expect(await viewer.selectedGroups()).toContain(id);
    });

    test("a hidden flyline is not under the pointer, and a click on it selects nothing", async ({ page, viewer }) => {
      await viewer.open("flylines", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-flylines"))).toBe(false);
      const p = await viewer.where("b", "flyline");
      await page.mouse.move(0, 0);
      await page.mouse.move(p.x, p.y);
      await expect(page.locator("#hover-tip")).toBeHidden();
      await expect(page.locator("#canvas")).not.toHaveClass(/over-object/);
      await page.mouse.click(p.x, p.y);
      await viewer.selected(null);
      // shown, the same point hits it; hidden again, it does not
      await page.evaluate(() => window.viewer.command("toggle-flylines"));
      await page.mouse.move(0, 0);
      await page.mouse.move(p.x, p.y);
      await expect(page.locator("#hover-tip")).toHaveText("b");
      await page.evaluate(() => window.viewer.command("toggle-flylines"));
      await page.mouse.move(0, 0);
      await page.mouse.move(p.x, p.y);
      await expect(page.locator("#hover-tip")).toBeHidden();
      await page.mouse.click(p.x, p.y);
      await viewer.selected(null);
    });

    test("the key L shows and hides them, but not while typing", async ({ page, viewer }) => {
      await viewer.open("flylines", { platform });
      await page.locator("#canvas").click({ position: { x: 5, y: 5 } });
      await page.keyboard.press("l");
      expect(await page.evaluate(() => window.viewer.checked("toggle-flylines"))).toBe(true);
      expect(await display(page)).not.toBe("none");
      await page.keyboard.press("Shift+L");
      expect(await page.evaluate(() => window.viewer.checked("toggle-flylines"))).toBe(false);
      // in the editor, L is a letter
      await page.keyboard.press("Control+q");
      await expect(page.locator("#source textarea:visible")).toBeFocused();
      await page.keyboard.type("lL");
      expect(await page.evaluate(() => window.viewer.checked("toggle-flylines"))).toBe(false);
    });
  });
}
