import { expect, test } from "../viewer.js";

const panes = ["files", "netlist", "hierarchy", "symbol", "usages", "unplaced", "solve"];

for (const platform of ["desktop", "web"]) {
  test(`${platform}: digits select panes and reopen the collapsed sidebar without rebuilding`, async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform });
    const builds = await viewer.builds();
    const offered = platform === "desktop" ? panes : panes.slice(0, 6);
    for (const [i, pane] of offered.entries()) {
      await page.keyboard.press("Control+B");
      await expect(page.locator("#sidebar")).toBeHidden();
      await page.keyboard.press(String(i + 1));
      await expect(page.locator(`#pane-${pane}`)).toBeVisible();
      await expect(page.locator(`#tab-${pane}`)).toBeFocused();
      await expect(page.locator(`#tab-${pane}`)).toHaveAttribute("aria-selected", "true");
      await expect(page.locator(`#tab-${pane} .tab-key`)).toHaveText(String(i + 1));
      expect(await viewer.stored("sidebar-shown")).toBe(true);
      expect(await page.evaluate(() => window.viewer.checked("toggle-netlist"))).toBe(true);
    }
    if (platform === "desktop") {
      await expect(page.locator("#solve-report")).toContainText("Run Solve (F9)");
    } else {
      await page.keyboard.press("7");
      await expect(page.locator("#tab-solve")).toBeHidden();
      await expect(page.locator("#pane-unplaced")).toBeVisible();
    }
    expect(await viewer.calls("solve")).toEqual([]);
    expect(await viewer.builds()).toBe(builds);
  });
}

test("opening Not drawn from a collapsed sidebar fills its first page immediately", async ({ page, viewer }) => {
  await viewer.open("unplaced", { platform: "desktop" });
  await page.keyboard.press("Control+B");
  await page.keyboard.press("6");
  await expect(page.locator("#unplaced-list .unplaced-row")).toHaveCount(20);
  await expect(page.locator("#unplaced-list .unplaced-row").first()).toBeVisible();
});

test("digits preserve text input, modifiers, dialogs and IME composition", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { platform: "desktop" });
  await page.locator("#show-failed").focus();
  await page.keyboard.press("3");
  await expect(page.locator("#show-failed")).toBeChecked();
  const search = page.locator("#search");
  await search.focus();
  await page.keyboard.type("1234567");
  await expect(search).toHaveValue("1234567");
  await expect(page.locator("#pane-hierarchy")).toBeVisible();
  // Hiding the sidebar while its input is focused must not leave focus in a hidden input.
  await page.keyboard.press("Control+B");
  await page.keyboard.press("2");
  await expect(page.locator("#pane-netlist")).toBeVisible();
  await page.keyboard.press("Control+q");
  const editor = page.locator("#source textarea:visible");
  await expect(editor).toBeFocused();
  await page.keyboard.press("Control+a");
  await page.keyboard.type("1234567");
  await expect(editor).toHaveValue("1234567");
  await expect(page.locator("#pane-netlist")).toBeVisible();
  await page.locator("#tab-netlist").focus();
  for (const modifiers of [{ ctrlKey: true }, { metaKey: true }, { altKey: true }, { shiftKey: true }, { isComposing: true }]) {
    for (const key of ["1", "3", "4", "5", "6", "7"]) {
      await page.locator("#tab-netlist").dispatchEvent("keydown", { key, ...modifiers });
      await expect(page.locator("#pane-netlist")).toBeVisible();
    }
  }
  await page.evaluate(() => window.viewer.command("shortcuts"));
  const dialog = page.locator("dialog.dialog");
  await expect(dialog).toBeVisible();
  await page.keyboard.type("134567");
  await expect(page.locator("#pane-netlist")).toBeVisible();
  await dialog.getByRole("button", { name: "Close" }).click();
});

test("digits switch from lists and the symbol viewer, and native menu commands open panes", async ({ page, viewer }) => {
  await viewer.open("symbol-inspector", { platform: "desktop" });
  await viewer.click("Xparent");
  await page.keyboard.press("Enter");
  await page.keyboard.press("3");
  await page.locator("#tree .twisty").first().focus();
  await expect(page.locator("#tree .twisty").first()).toBeFocused();
  await page.keyboard.press("4");
  await expect(page.locator("#pane-symbol")).toBeVisible();
  await page.locator(".symbol-preview").focus();
  await page.keyboard.press("6");
  await expect(page.locator("#pane-unplaced")).toBeVisible();
  await page.keyboard.press("Control+B");
  await page.evaluate(() => window.fakeShell.menu("panel-solve"));
  await expect(page.locator("#pane-solve")).toBeVisible();
  expect(await viewer.calls("solve")).toEqual([]);
});

test("panel shortcuts work with no open netlist and the badges fit a narrow sidebar", async ({ page, viewer }) => {
  await viewer.open(null, { platform: "desktop" });
  await page.evaluate(async () => (await import("./panels.js")).setSidebarWidth(220));
  for (const [i, pane] of panes.entries()) {
    await page.keyboard.press(String(i + 1));
    await expect(page.locator(`#pane-${pane}`)).toBeVisible();
    const tab = page.locator(`#tab-${pane}`);
    const [box, key, side] = await Promise.all([tab.boundingBox(), tab.locator(".tab-key").boundingBox(), page.locator("#sidebar").boundingBox()]);
    expect(box.x + box.width).toBeLessThanOrEqual(side.x + side.width);
    expect(key.x).toBeGreaterThanOrEqual(box.x);
    expect(key.x + key.width).toBeLessThan(box.x + 14);
    expect(key.y).toBeGreaterThanOrEqual(box.y);
    expect(key.y + key.height).toBeLessThan(box.y + box.height);
  }
});
