// Solve > Show Layout Table and the Solve panel's Layout table checkbox (apps/viewer-ui/layout-frame.js):
// thin non-scaling cells computed from the drawing geometry and group tree; off by default, kept in
// settings, placed in Solve (never View), and excluded from File > Export Image.

import fs from "node:fs";
import { expect, test } from "../viewer.js";

const rects = (page) => page.locator("#canvas .layout-table rect");

const NETLIST_04 = `* layout table 0.4
* @RNL(type="header") { @VERSION("0.4") }
* @RNL {
*   $BOX2 = @DEFINE-SYMBOL(box=[-2, -1, 2, 1]) {
*     %p = (-2, 0) @PIN(terminal="1", direction="left");
*     %n = (2, 0) @PIN(terminal="2", direction="right");
*     @RECTANGLE {(-2, -1) (2, 1)}
*   };
*   $FLAG_SYM = @DEFINE-SYMBOL(box=[-1, -2, 1, 0]) {
*     %pin = (0, 0) @PIN(terminal="p", direction="up");
*     @LINE {(0, 0) (0, -2)}
*   };
*   $FLAG = @DEFINE(roles=[["p", "net"]]) {};
*   # @SELECT-USAGE(recursive=true) $FLAG => @SET-SYMBOL $FLAG_SYM;
*   $PIN_FLAG = @DEFINE(roles=[["t", "pin"]]) {
*     %f = $FLAG @ATTACH ?t::net
*   };
*   $MARK_SYM = @DEFINE-SYMBOL(box=[-0.4, -0.4, 0.4, 0.4]) {
*     (0, 0) @CIRCLE(radius=0.3, fill="linecolor", linestyle="none")
*   };
*   $MARK = @DEFINE(roles=[["at", "point"]]) { {? ?at} @ALIGN("x"); {? ?at} @ALIGN("y") };
*   # @SELECT-USAGE(recursive=true) $MARK => @SET-SYMBOL $MARK_SYM;
*   {#R1 #R2 #R3} @SET-SYMBOL $BOX2;
*   {#R1 #R2} @GROUP;
*   %pf = $PIN_FLAG @ATTACH #R1::p;
*   %m1 = $MARK @ATTACH (0, 4);
*   #R1 @PLACE(-6, 4); #R2 @PLACE(6, 4); #R3 @PLACE(0, -4);
*   %pf::f @PLACE(-8, 2);
*   %m1 @PLACE(0, 4)
* }
R1 a b 1k
R2 b c 1k
R3 a c 1k
.end
`;

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform}`, () => {
    test("starts off, lives under Solve and the Solve panel (not View), syncs both controls, and persists", async ({ page, viewer }) => {
      await viewer.open("04-hierarchy", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-layout"))).toBe(false);
      await expect(rects(page)).toHaveCount(0);
      await expect(page.locator('#menubar .menu:has(.menu-title:text-is("View")) .menu-item[data-id="toggle-layout"]')).toHaveCount(0);

      if (platform === "web") {
        await viewer.menuTitle("Solve").click();
        const item = page.locator('#menubar .menu-item[data-id="toggle-layout"]');
        await expect(item.locator(".label")).toHaveText("Show Layout Table");
        await item.click();
      } else {
        await page.evaluate(() => window.fakeShell.menu("toggle-layout"));
      }

      expect(await page.evaluate(() => window.viewer.checked("toggle-layout"))).toBe(true);
      await expect(page.locator("#canvas .layout-table .layout-frame")).toHaveCount(1);
      expect(await page.locator("#canvas .layout-table .layout-cell").count()).toBeGreaterThan(0);
      expect(await page.evaluate(() => document.querySelector(".layout-table").getAttribute("pointer-events"))).toBe("none");
      expect(await page.evaluate(() => getComputedStyle(document.querySelector(".layout-table rect")).vectorEffect)).toBe("non-scaling-stroke");

      // The Solve panel checkbox reflects the menu state and toggles it back.
      await page.keyboard.press("7");
      const checkbox = page.locator("#solve-layout-table");
      await expect(checkbox).toBeVisible();
      await expect(checkbox).toBeChecked();
      await checkbox.uncheck();
      expect(await page.evaluate(() => window.viewer.checked("toggle-layout"))).toBe(false);
      await expect(rects(page)).toHaveCount(0);

      await checkbox.check();
      expect(await page.evaluate(() => window.viewer.checked("toggle-layout"))).toBe(true);
      const count = await rects(page).count();
      expect(count).toBeGreaterThan(1);

      // Persisted across reloads.
      await viewer.open("04-hierarchy", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-layout"))).toBe(true);
      await expect(rects(page)).toHaveCount(count);
    });

    test("updates across subcircuit levels and keeps non-scaling stroke when zooming", async ({ page, viewer }) => {
      await viewer.open("usage-current", { platform });
      await page.evaluate(() => window.viewer.command("toggle-layout"));
      const topCount = await rects(page).count();
      expect(topCount).toBeGreaterThan(1);

      const widthBefore = await page.evaluate(() => getComputedStyle(document.querySelector(".layout-table .layout-frame")).strokeWidth);
      await page.evaluate(async () => (await import("./drawing.js")).zoomCenter(2.5));
      const widthAfter = await page.evaluate(() => getComputedStyle(document.querySelector(".layout-table .layout-frame")).strokeWidth);
      expect(widthAfter).toBe(widthBefore);

      await page.evaluate(async () => (await import("./subcircuits.js")).goTo(1));
      expect(await rects(page).count()).toBeGreaterThan(1);
      await page.evaluate(async () => (await import("./subcircuits.js")).goTo(0));
      await expect(rects(page)).toHaveCount(topCount);
    });
  });
}

test("draws group and cell partitions on RNL 0.4 netlists (merging pin-attached usages and skipping point marks) and stays out of Export SVG", async ({ page }) => {
  test.skip(!process.env.RNL_WEB_URL, "the web site is not built");
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await expect(page.locator("#menubar")).toBeVisible();
  await page.locator("#file-input").setInputFiles({
    name: "layout-04.sp",
    mimeType: "text/plain",
    buffer: Buffer.from(NETLIST_04),
  });
  await page.waitForFunction(() => window.viewer?.doc && !document.body.classList.contains("busy"));

  await page.keyboard.press("7");
  await page.locator("#solve-layout-table").check();
  await expect(page.locator("#canvas .layout-table .layout-frame")).toHaveCount(1);
  await expect(page.locator("#canvas .layout-table .layout-group")).toHaveCount(1);
  // Three components (#R1 with %tag1 merged in, #R2, #R3); %dot1 is skipped as a point mark.
  await expect(page.locator("#canvas .layout-table .layout-cell")).toHaveCount(3);

  const download = page.waitForEvent("download");
  await page.evaluate(() => window.viewer.command("export-svg"));
  const result = await download;
  const svgText = fs.readFileSync(await result.path(), "utf8");
  expect(svgText).not.toContain("layout-table");
});
