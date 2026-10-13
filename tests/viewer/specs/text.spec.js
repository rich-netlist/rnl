// The text pane: the statements that do not take effect and the box that marks them, the problems
// that show their lines, the file tabs, and a line that waits for the text pane to be shown.

import { expect, test } from "../viewer.js";

const TOP = "04-hierarchy.sp";

test("failed and overridden statements are marked, and the checkbox turns the marks off", async ({ page, viewer }) => {
  await viewer.open("12-diagnostics");
  const failed = await page.evaluate(() => window.viewer.doc.files[0].failed);
  expect(failed.some((f) => f[4]) && failed.some((f) => !f[4])).toBe(true);
  // each mark covers its statement, on one line here
  for (const [l0, c0, l1, c1, overridden] of failed) {
    expect(l1).toBe(l0);
    const want = (await viewer.line(l0).locator(".code").textContent()).slice(c0 - 1, c1);
    const marked = await viewer
      .line(l0)
      .locator(overridden ? ".x-overridden" : ".x-failed")
      .evaluateAll((els) => els.map((e) => e.textContent).join(""));
    expect(marked).toContain(want);
  }
  // on line 44 an override, then a failure
  await expect(viewer.line(44).locator(".x-overridden").first()).toHaveAttribute("title", /^Overridden/);
  await expect(viewer.line(44).locator(".x-failed").first()).toHaveAttribute("title", /^Failed/);

  const background = (cls) => viewer.line(44).locator(cls).first().evaluate((e) => getComputedStyle(e).backgroundColor);
  const none = "rgba(0, 0, 0, 0)";
  const [failedBg, overriddenBg] = [await background(".x-failed"), await background(".x-overridden")];
  expect(failedBg).not.toBe(none);
  expect(overriddenBg).not.toBe(none);
  expect(failedBg).not.toBe(overriddenBg);

  const box = page.locator("#show-failed");
  await expect(box).toBeChecked();
  await box.uncheck();
  await expect(page.locator("#source")).toHaveClass(/\bhide-failed\b/);
  expect([await background(".x-failed"), await background(".x-overridden")]).toEqual([none, none]);
  expect(await viewer.stored("show-failed")).toBe(false);
  // the setting is kept for the next start
  await viewer.open("12-diagnostics");
  await expect(box).not.toBeChecked();
  await expect(page.locator("#source")).toHaveClass(/\bhide-failed\b/);
  // the label's text turns it back on
  await page.locator(".failed-toggle span", { hasText: "Failed statements" }).click();
  await expect(box).toBeChecked();
  expect([await background(".x-failed"), await background(".x-overridden")]).toEqual([failedBg, overriddenBg]);
  expect(await viewer.stored("show-failed")).toBe(true);
});

test("clicking a problem shows its line in the middle of the text and flashes it", async ({ page, viewer }) => {
  await viewer.open("12-diagnostics");
  const problem = (text) => page.locator("#diags li.diag", { hasText: text });
  await expect(problem("PRIORITY takes constraints")).toHaveClass(/\blocated\b/);
  expect(await viewer.within(viewer.line(44))).toBe(false);

  await problem("PRIORITY takes constraints").click();
  await expect.poll(() => viewer.within(viewer.line(44))).toBe(true);
  await expect(viewer.line(44)).toHaveClass(/\bflash\b/);
  // in the middle of the text
  const [line, source] = [await viewer.line(44).boundingBox(), await page.locator("#source").boundingBox()];
  expect(Math.abs(line.y + line.height / 2 - (source.y + source.height / 2))).toBeLessThan(line.height);

  // one at the end, as far as the text scrolls
  await problem("formal tokens after the record").click();
  await expect.poll(() => viewer.within(viewer.line(72))).toBe(true);
  await expect(viewer.line(72)).toHaveClass(/\bflash\b/);

  // one near the top: the text scrolls back
  await problem("package no-such-package").click();
  await expect.poll(() => viewer.within(viewer.line(16))).toBe(true);
  await expect(viewer.line(16)).toHaveClass(/\bflash\b/);
  expect(await viewer.within(viewer.line(72))).toBe(false);
});

test("selecting on the drawing shows the file and the line of the statement", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  expect(await viewer.within(viewer.line(49))).toBe(false);
  await viewer.click("X2");
  await expect.poll(() => viewer.within(viewer.line(49))).toBe(true);
  const main = await page.locator("#source").evaluate((s) => s.scrollTop);
  expect(main).toBeGreaterThan(0);

  // a statement in the included file shows that file
  await page.locator("#tab-hierarchy").click();
  await page.locator("#index .index-row", { hasText: "rdiv" }).click();
  await page.locator("#tab-netlist").click();
  await viewer.click("R2");
  await viewer.selected("instance R2 in rdiv");
  await expect(page.locator("#file-tabs button[aria-selected=true]")).toHaveText("04-hierarchy-lib.inc");
  expect(await viewer.markedLines()).toEqual([4]);

  // each file keeps its own scroll
  await page.locator("#file-tabs button", { hasText: TOP }).click();
  expect(await page.locator("#source").evaluate((s) => s.scrollTop)).toBe(main);
});

test("a line to show waits until the text pane is shown", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  // the side panel hidden
  await page.keyboard.press("Control+B");
  await expect(page.locator("#sidebar")).toBeHidden();
  await viewer.click("X2");
  await viewer.selected(`instance X2 in ${TOP}`);
  await page.keyboard.press("Control+B");
  await expect(page.locator("#sidebar")).toBeVisible();
  await expect.poll(() => viewer.within(viewer.line(49))).toBe(true);

  // the hierarchy view shown instead of the text
  await page.locator("#tab-hierarchy").click();
  await page.locator("#source").evaluate((s) => { s.scrollTop = 0; });
  await viewer.click("Vin");
  await viewer.selected(`instance Vin in ${TOP}`);
  await page.locator("#tab-netlist").click();
  await expect.poll(() => viewer.within(viewer.line(52))).toBe(true);
});
