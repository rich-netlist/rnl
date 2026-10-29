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

  test("a name that fits a line is moved whole to the next, not broken", async ({ page, viewer }) => {
    await viewer.open("usage-ota5t");
    await page.evaluate(async () => {
      const o = window.viewer.doc.objects.findIndex((x) => x.kind === "instance" && x.name === "M1");
      (await import("./selection.js")).select(o, "list", { usage: false });
    });
    await page.locator("#page-panel").tap();
    await page.locator("#tab-usages").tap();
    await expect(page.locator("#usage-detail .usage-membership-row")).toHaveCount(2);
    for (const lang of ["en", "zh"]) {
      if (lang === "zh") await page.evaluate(() => window.viewer.command("language-zh"));
      const names = await page.evaluate(() => [...document.querySelectorAll("#usage-detail .membership-definition, #usage-detail .membership-id")].map((d) => {
        const first = d.firstChild;
        const r = document.createRange();
        r.setStart(first, 0); r.setEnd(first, 1);
        const last = d.lastChild.lastChild ?? d.lastChild;
        const q = document.createRange();
        q.selectNodeContents(last);
        return [d.textContent.replace("）", ")"), Math.round(r.getBoundingClientRect().top) === Math.round(q.getBoundingClientRect().top) ? 1 : 2];
      }));
      expect(names).toEqual([["%dp1", 1], ["$DIFFERENTIAL_PAIR)", 1], ["%sym12", 1], ["$MIRRORED)", 1]]);
    }
  });

  test("the button that opens and folds all the groups of usages is on the screen's width", async ({ page, viewer }) => {
    await viewer.open("usage-inspector");
    await page.locator("#page-panel").tap();
    await page.locator("#tab-usages").tap();
    const button = page.locator("#usage-expand-all");
    await expect(button).toBeVisible();
    const [b, list] = await Promise.all([button.boundingBox(), page.locator("#usage-list").boundingBox()]);
    expect(b.x).toBeGreaterThanOrEqual(list.x);
    expect(b.x + b.width).toBeLessThanOrEqual(list.x + list.width + 1);
    await expect(page.locator(".usage-row")).toHaveCount(0);
    await button.tap();
    await expect(page.locator(".usage-row").first()).toBeVisible();
    await expect(button).toHaveText("Collapse All");
  });

  test("a tap on the id of a row selects the usage", async ({ page }) => {
    await page.locator('#usage-detail button.membership-id:text-is("%gnd")').tap();
    await expect(page.locator("#usage-detail h2")).toHaveText(/%gnd/);
    await expect(page.locator("#usage-detail .usage-membership-row")).toHaveCount(0);
  });
});
