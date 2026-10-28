// The usages of the selected object on a phone: the Usage panel is the second page, its lower half is the table of
// the usages the selected instance is a member of (apps/viewer-ui/usage-table.js); a tap on a row selects the usage,
// and the table, with its indented rows and its long ids, stays within the screen's width. The pointer's box and
// the text's context menu are for a pointer; a long press on a reference gives the menu where the browser sends it.
// How the fingers are made: tests/viewer/README.md#mobile.

import { expect, test } from "../mobile.js";

test.describe("on a phone", () => {
  test.beforeEach(async ({ page, viewer }) => {
    await viewer.open("usage-query");
    await viewer.click("R3");
    await page.locator("#page-panel").tap();
    await page.locator("#tab-usages").tap();
  });

  test("the table of the selected instance's usages fits the width of the screen", async ({ page }) => {
    const rows = page.locator("#usage-detail .usage-membership-row");
    await expect(page.locator("#usage-detail h2")).toHaveText("Usages of R3");
    await expect(rows).toHaveCount(7);
    const wide = await page.evaluate(() => {
      const detail = document.querySelector("#usage-detail");
      return { page: document.documentElement.scrollWidth - document.documentElement.clientWidth, detail: detail.scrollWidth - detail.clientWidth };
    });
    expect(wide.page).toBeLessThanOrEqual(0);
    expect(wide.detail).toBeLessThanOrEqual(0);
    // the rows are as wide as the panel, however long a usage's id is
    const inside = await rows.evaluateAll((els) => {
      const panel = document.querySelector("#usage-detail").getBoundingClientRect();
      return els.every((e) => { const b = e.getBoundingClientRect(); return b.left >= panel.left - 1 && b.right <= panel.right + 1; });
    });
    expect(inside).toBe(true);
  });

  test("usages nested deep, with long names, wrap within the width of the screen", async ({ page, viewer }) => {
    await viewer.open("usage-deep");
    await page.evaluate(async () => {
      const o = window.viewer.doc.objects.findIndex((x) => x.kind === "instance" && x.name === "R1");
      (await import("./selection.js")).select(o, "list", { usage: false });
    });
    await page.locator("#page-panel").tap();
    await page.locator("#tab-usages").tap();
    const rows = page.locator("#usage-detail .usage-membership-row");
    await expect(rows).toHaveCount(8);
    const wide = await page.evaluate(() => {
      const detail = document.querySelector("#usage-detail");
      const panel = detail.getBoundingClientRect();
      return {
        page: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        detail: detail.scrollWidth - detail.clientWidth,
        outside: [...detail.querySelectorAll(".usage-membership-row *")].filter((e) => e.getBoundingClientRect().right > panel.right + 1).length,
      };
    });
    expect(wide.page).toBeLessThanOrEqual(0);
    expect(wide.detail).toBeLessThanOrEqual(0);
    expect(wide.outside).toBe(0);
  });

  test("a tap on the id of a row selects the usage", async ({ page }) => {
    await page.locator('#usage-detail button.membership-id:text-is("%gnd")').tap();
    await expect(page.locator("#usage-detail h2")).toHaveText(/%gnd/);
    await expect(page.locator("#usage-detail .usage-membership-row")).toHaveCount(0);
  });
});
