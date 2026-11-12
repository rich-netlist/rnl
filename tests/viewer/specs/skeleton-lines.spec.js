// Solve > Show Component and Pin Skeleton Lines (`toggle-skeleton-symbols`), Solve > Show Wire
// Skeleton Lines (`toggle-skeleton-wires`) and their Solve panel checkboxes
// (apps/viewer-ui/skeleton-lines.js): thin, dashed, faint reference lines computed directly from
// drawing geometry, deduplicated when coincident, named on hover, placed under Solve (never View),
// and excluded from File > Export Image.

import fs from "node:fs";
import { expect, test } from "../viewer.js";

const lines = (page) => page.locator("#canvas .skeleton-lines line");

const NETLIST_04 = `* skeleton lines 0.4
* @RNL(type="header") { @VERSION("0.4") }
* @RNL {
*   $BOX2 = @DEFINE-SYMBOL(box=[-2, -1, 2, 1]) {
*     %p = (-2, 0) @PIN(terminal="1", direction="left");
*     %n = (2, 0) @PIN(terminal="2", direction="right");
*     @RECTANGLE {(-2, -1) (2, 1)}
*   };
*   $MARK_SYM = @DEFINE-SYMBOL(box=[-0.4, -0.4, 0.4, 0.4]) {
*     (0, 0) @CIRCLE(radius=0.3, fill="linecolor", linestyle="none")
*   };
*   $MARK = @DEFINE(roles=[["at", "point"]]) { {? ?at} @ALIGN("x"); {? ?at} @ALIGN("y") };
*   # @SELECT-USAGE(recursive=true) $MARK => @SET-SYMBOL $MARK_SYM;
*   {#R1 #R2} @SET-SYMBOL $BOX2;
*   %m1 = $MARK @ATTACH (0, 0);
*   #R1 @PLACE(-6, 0); #R2 @PLACE(6, 0);
*   %m1 @PLACE(0, 0);
*   &b @WIRE { #R1::n (0, 0) #R2::p }
* }
R1 a b 1k
R2 b c 1k
.end
`;

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform}`, () => {
    test("both switches start off, live under Solve and the Solve panel (not View), sync controls, and persist", async ({ page, viewer }) => {
      await viewer.open("04-hierarchy", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-skeleton-symbols"))).toBe(false);
      expect(await page.evaluate(() => window.viewer.checked("toggle-skeleton-wires"))).toBe(false);
      await expect(lines(page)).toHaveCount(0);
      await expect(page.locator('#menubar .menu:has(.menu-title:text-is("View")) .menu-item[data-id="toggle-skeleton-symbols"]')).toHaveCount(0);
      await expect(page.locator('#menubar .menu:has(.menu-title:text-is("View")) .menu-item[data-id="toggle-skeleton-wires"]')).toHaveCount(0);

      if (platform === "web") {
        await viewer.menuTitle("Solve").click();
        const symItem = page.locator('#menubar .menu-item[data-id="toggle-skeleton-symbols"]');
        await expect(symItem.locator(".label")).toHaveText("Show Component and Pin Skeleton Lines");
        await symItem.click();
      } else {
        await page.evaluate(() => window.fakeShell.menu("toggle-skeleton-symbols"));
      }

      expect(await page.evaluate(() => window.viewer.checked("toggle-skeleton-symbols"))).toBe(true);
      const symCount = await lines(page).count();
      expect(symCount).toBeGreaterThan(0);
      await expect(page.locator("#canvas .skeleton-lines .skeleton-symbol")).toHaveCount(symCount);
      expect(await page.evaluate(() => document.querySelector(".skeleton-lines").getAttribute("pointer-events"))).toBe("none");
      const style = await page.evaluate(() => {
        const s = getComputedStyle(document.querySelector(".skeleton-lines line"));
        return { vectorEffect: s.vectorEffect, strokeDasharray: s.strokeDasharray, strokeOpacity: Number(s.strokeOpacity) };
      });
      expect(style.vectorEffect).toBe("non-scaling-stroke");
      expect(style.strokeDasharray).not.toBe("none");
      expect(style.strokeOpacity).toBeLessThan(0.7);

      // Solve panel checkboxes reflect menu state and toggle both switches independently.
      await page.keyboard.press("7");
      const symCheck = page.locator("#solve-skeleton-symbols");
      const wireCheck = page.locator("#solve-skeleton-wires");
      await expect(symCheck).toBeVisible();
      await expect(wireCheck).toBeVisible();
      await expect(symCheck).toBeChecked();
      await expect(wireCheck).not.toBeChecked();

      await wireCheck.check();
      expect(await page.evaluate(() => window.viewer.checked("toggle-skeleton-wires"))).toBe(true);
      const bothCount = await lines(page).count();
      expect(bothCount).toBeGreaterThanOrEqual(symCount);

      await symCheck.uncheck();
      expect(await page.evaluate(() => window.viewer.checked("toggle-skeleton-symbols"))).toBe(false);
      const wireCount = await lines(page).count();
      expect(wireCount).toBeGreaterThan(0);
      await expect(page.locator("#canvas .skeleton-lines .skeleton-wire")).toHaveCount(wireCount);
      // Coincident lines between component/pin and wire skeletons are drawn once when both are on.
      expect(bothCount).toBeLessThan(symCount + wireCount);

      // Persisted across reloads.
      await viewer.open("04-hierarchy", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-skeleton-symbols"))).toBe(false);
      expect(await page.evaluate(() => window.viewer.checked("toggle-skeleton-wires"))).toBe(true);
      await expect(lines(page)).toHaveCount(wireCount);
    });

    test("updates across subcircuit levels and keeps non-scaling stroke when zooming", async ({ page, viewer }) => {
      await viewer.open("usage-current", { platform });
      await page.evaluate(() => {
        window.viewer.command("toggle-skeleton-symbols");
        window.viewer.command("toggle-skeleton-wires");
      });
      const topCount = await lines(page).count();
      expect(topCount).toBeGreaterThan(2);

      const widthBefore = await page.evaluate(() => getComputedStyle(document.querySelector(".skeleton-lines line")).strokeWidth);
      await page.evaluate(async () => (await import("./drawing.js")).zoomCenter(2.5));
      const widthAfter = await page.evaluate(() => getComputedStyle(document.querySelector(".skeleton-lines line")).strokeWidth);
      expect(widthAfter).toBe(widthBefore);

      await page.evaluate(async () => (await import("./subcircuits.js")).goTo(1));
      expect(await lines(page).count()).toBeGreaterThan(2);
      await page.evaluate(async () => (await import("./subcircuits.js")).goTo(0));
      await expect(lines(page)).toHaveCount(topCount);
    });
  });
}

test("deduplicates RNL 0.4 component/pin/wire lines, skips point marks, names owners on hover, and stays out of Export SVG", async ({ page }) => {
  test.skip(!process.env.RNL_WEB_URL, "the web site is not built");
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await expect(page.locator("#menubar")).toBeVisible();
  await page.locator("#file-input").setInputFiles({
    name: "skeleton-04.sp",
    mimeType: "text/plain",
    buffer: Buffer.from(NETLIST_04),
  });
  await page.waitForFunction(() => window.viewer?.doc && !document.body.classList.contains("busy"));
  await page.evaluate(() => window.viewer.command("fit"));

  await page.keyboard.press("7");
  await page.locator("#solve-skeleton-symbols").check();
  // #R1 at (-6,0) and #R2 at (6,0) with horizontal pins at y=0, %m1 point mark at (0,0) skipped:
  // x=-6, x=6, and y=0 -> 3 unique symbol/pin lines (no x=0 line from %m1).
  await expect(lines(page)).toHaveCount(3);

  await page.locator("#solve-skeleton-wires").check();
  // Wire &b has two collinear horizontal segments on y=0: deduplicated into the shared y=0 line -> still 3 lines total.
  await expect(lines(page)).toHaveCount(3);
  await expect(page.locator("#canvas .skeleton-lines .skeleton-shared")).toHaveCount(1);

  // Hover near the left end of the shared y=0 line (outside #R1's box) and check the owner tooltip.
  const hoverPt = await page.evaluate(() => {
    const shared = document.querySelector("#canvas .skeleton-lines .skeleton-shared");
    const r = shared.getBoundingClientRect();
    return { x: r.left + 8, y: r.top + r.height / 2 };
  });
  await page.mouse.move(hoverPt.x, hoverPt.y);
  const tip = page.locator("#hover-tip");
  await expect(tip).toBeVisible();
  await expect(tip).toContainText("y of #R1");
  await expect(tip).toContainText("pin #R1::p");
  await expect(tip).toContainText("wire &b (2 segments)");

  const download = page.waitForEvent("download");
  await page.evaluate(() => window.viewer.command("export-svg"));
  const result = await download;
  const svgText = fs.readFileSync(await result.path(), "utf8");
  expect(svgText).not.toContain("skeleton-lines");
});
