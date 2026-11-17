// The Groups view of the Solve pane (apps/viewer-ui/groups-view.js): the groups of the level shown as the tree the core
// gives (`group_parent`), the ones it does not take in after them with their reasons, a click that lights a group and a
// check that shows or hides its frame.

import { expect, test } from "../viewer.js";

const rows = (page) => page.locator("#groups-list .group-row");
const depthOf = (row) => row.evaluate((e) => (parseFloat(e.style.paddingLeft) - 26) / 14);

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform}`, () => {
    test("the groups of a 0.4 level stand in the tree, the others follow with their reasons", async ({ page, viewer }) => {
      await viewer.open("groups-04", { platform });
      await page.keyboard.press("7");
      await expect(page.locator("#solve-view-groups")).toBeVisible();
      await page.locator("#solve-view-groups").click();
      await expect(page.locator("#solve-view-groups")).toHaveAttribute("aria-pressed", "true");
      // the tree: %big holds the rest, the three below it in the order the solver takes them in
      const tree = page.locator("#groups-list > .group-row:not(.greyed)");
      await expect(tree).toHaveCount(4);
      const inTree = await tree.locator(".usage-name").evaluateAll((els) => els.map((e) => e.childNodes[1].textContent));
      expect(inTree[0]).toBe("%big");
      expect(new Set(inTree.slice(1))).toEqual(new Set(["%again", "%cross", "%far"]));
      expect(await depthOf(tree.nth(0))).toBe(0);
      for (const i of [1, 2, 3]) expect(await depthOf(tree.nth(i))).toBe(1);
      // the members, as written, beside the name
      await expect(tree.nth(0).locator(".group-members")).toHaveText("#R1 #R2 #R3 #R4 #R5");
      // the groups the solver does not take in: left out for crossing, the same members as another, turned off
      await expect(page.locator("#groups-list > .groups-rest")).toHaveText("Not in the tree (3)");
      const rest = page.locator("#groups-list > .group-row.greyed");
      await expect(rest).toHaveCount(3);
      const outer = rest.filter({ hasText: "%outer" });
      await expect(outer.locator(".usage-state")).toHaveText("left out");
      await expect(outer.locator(".group-note")).toContainText("%cross");
      const inner = rest.filter({ hasText: "%inner" });
      await expect(inner.locator(".usage-state")).toHaveText("same members");
      await expect(inner.locator(".group-note")).toContainText("%again");
      const off = rest.filter({ hasText: "%off" });
      await expect(off.locator(".usage-state")).toHaveText("disabled");
      await expect(off.locator(".group-note")).toHaveText("turned off by PRIORITY");
    });

    test("a click lights a group and selects it, and the check shows or hides its frame", async ({ page, viewer }) => {
      await viewer.open("groups-04", { platform });
      await page.keyboard.press("7");
      await page.locator("#solve-view-groups").click();
      const far = rows(page).filter({ hasText: "%far" });
      await expect(far).not.toHaveClass(/selected/);
      await far.locator(".usage-name").click();
      await expect(far).toHaveClass(/selected/);
      // the group is selected, and the Usage panel, which does not list the groups, shows its details
      await page.keyboard.press("5");
      await expect(page.locator("#usage-detail h2")).toHaveText("@GROUP: %far");
      await expect(page.locator('#usage-list .usage-row:has-text("%far")')).toHaveCount(0);
      await page.keyboard.press("7");
      // with Group frames on, the check shows or hides the frame (the fixture draws no resistor: what the page would draw)
      const shown = () => page.evaluate(async () => {
        const far = window.viewer.doc.levels[0].usages.find((u) => u.id === "%far");
        return (await import("./usages.js")).overlayChoices().each.get(far.object).visible;
      });
      const check = far.locator('input[type="checkbox"]');
      await check.check();
      expect(await shown()).toBe(false);
      await page.locator("#solve-group-frames").check();
      expect(await shown()).toBe(true);
      await check.uncheck();
      expect(await shown()).toBe(false);
    });

    test("a level of an earlier version has its groups all at the top, and a level without groups has no view", async ({ page, viewer }) => {
      await viewer.open("ota-groups", { platform });
      await page.keyboard.press("7");
      await page.locator("#solve-view-groups").click();
      const top = page.locator("#groups-list > .group-row:not(.greyed)");
      expect(await top.count()).toBeGreaterThan(1);
      for (let i = 0; i < await top.count(); i++) expect(await depthOf(top.nth(i))).toBe(0);
      // %bias is turned off by its PRIORITY
      await expect(page.locator("#groups-list > .group-row.greyed .usage-state")).toHaveText("disabled");

      await viewer.open("01-rc-lowpass", { platform });
      await page.keyboard.press("7");
      await expect(page.locator("#solve-view-groups")).toBeHidden();
      await expect(page.locator("#solve-groups")).toBeHidden();
    });
  });
}
