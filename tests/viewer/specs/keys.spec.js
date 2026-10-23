// The rules by which a key runs a command of the table (apps/viewer-ui/README.md): a key without
// Ctrl that types or deletes does not count while Ctrl or Alt is held, a key with Ctrl and a letter
// counts only with the Shift it names, and in a text box only keys with Ctrl count. The page runs
// with a native menu, as on the desktop, so that no menu bar of its own takes Alt.

import { expect, test } from "../viewer.js";

const TOP = "04-hierarchy.sp";

test.beforeEach(async ({ viewer }) => {
  await viewer.open("04-hierarchy", { platform: "desktop" });
});

test("F and Backspace do not count while Ctrl or Alt is held", async ({ page, viewer }) => {
  await page.keyboard.press("Control+=");
  const zoomed = await viewer.viewBox();
  for (const key of ["Control+F", "Alt+F"]) {
    await page.keyboard.press(key);
    expect(await viewer.viewBox()).toBe(zoomed);
    if (key === "Control+F") {
      const find = page.getByRole('searchbox', { name: 'Find in netlist' });
      await expect(find).toBeFocused();
      await find.press('Escape');
    }
  }
  await page.keyboard.press("f");
  await expect.poll(() => viewer.viewBox()).not.toBe(zoomed);

  await viewer.click("X2");
  await page.keyboard.press("Enter");
  await expect(page.locator("#level-name")).toHaveText("amp");
  for (const key of ["Control+Backspace", "Alt+Backspace"]) {
    await page.keyboard.press(key);
    await expect(page.locator("#level-name")).toHaveText("amp");
  }
  await page.keyboard.press("Backspace");
  await expect(page.locator("#level-name")).toHaveText(TOP);
});

test("a key with Ctrl and a letter counts only with the Shift it names", async ({ page, viewer }) => {
  // the marks the shell was told at start
  const start = (await viewer.calls("setCheck")).length;
  await page.keyboard.press("Control+Shift+B");
  await page.keyboard.press("Control+M");
  await page.keyboard.press("Control+B");
  await expect(page.locator("#sidebar")).toBeHidden();
  await page.keyboard.press("Control+Shift+M");
  await expect(page.locator("#pane-netlist")).toHaveClass(/\bproblems-folded\b/);
  // The two shortcuts counted; hiding also clears all pane check marks.
  const changes = (await viewer.calls("setCheck")).slice(start);
  expect(changes.filter(([id]) => !id.startsWith("panel-"))).toEqual([["toggle-netlist", false], ["toggle-problems", false]]);
  expect(changes.filter(([id]) => id.startsWith("panel-"))).toEqual(
    ["files", "netlist", "hierarchy", "symbol", "usages", "unplaced", "solve"].map(name => [`panel-${name}`, false]),
  );
});

test("G shows and hides the grid, but not while Ctrl or Alt is held, nor in a text box", async ({ page, viewer }) => {
  const shown = () => page.evaluate(() => window.viewer.grid.shown);
  expect(await shown()).toBe(true);
  for (const key of ["Control+G", "Alt+G"]) {
    await page.keyboard.press(key);
    expect(await shown()).toBe(true);
  }
  await page.keyboard.press("g");
  expect(await shown()).toBe(false);
  await page.keyboard.press("g");
  expect(await shown()).toBe(true);
  await page.locator("#tab-hierarchy").click();
  await page.locator("#search").click();
  await page.keyboard.type("gg");
  await expect(page.locator("#search")).toHaveValue("gg");
  expect(await shown()).toBe(true);
});

test("in a text box only keys with Ctrl count", async ({ page, viewer }) => {
  await page.keyboard.press("Control+=");
  const zoomed = await viewer.viewBox();
  await page.locator("#tab-hierarchy").click();
  const search = page.locator("#search");
  await search.click();
  await page.keyboard.type("f0");
  await expect(search).toHaveValue("f0");
  expect(await viewer.viewBox()).toBe(zoomed);
  await page.keyboard.press("Control+Shift+M");
  await expect(page.locator("#pane-netlist")).toHaveClass(/\bproblems-folded\b/);
  await page.keyboard.press("Control+B");
  await expect(page.locator("#sidebar")).toBeHidden();
});
