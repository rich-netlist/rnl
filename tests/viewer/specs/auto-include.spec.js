import { test, expect } from "../viewer.js";

for (const platform of ["desktop", "web"]) {
  test(`${platform}: Auto Include checkbox and File menu share a persisted setting and keep pending edits`, async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform, pool: "1", settings: "1" });
    await page.locator("#tab-netlist").click();
    await expect(page.locator("#auto-include")).not.toBeChecked();
    await page.evaluate(() => window.viewer.command("toggle-edit"));
    const text = await page.locator("#source textarea:visible").inputValue();
    await page.locator("#source textarea:visible").fill(text + "\n* pending text\n");
    const pending = await page.evaluate(() => window.viewer.edits);
    await viewer.build(() => page.locator("#auto-include").check());
    expect(await page.evaluate(() => window.viewer.checked("toggle-auto-include"))).toBe(true);
    expect(await page.evaluate(() => window.viewer.edits)).toEqual(pending);
    await expect(page.locator("#source textarea:visible")).toHaveValue(text + "\n* pending text\n");
    await viewer.build(() => page.evaluate(() => window.viewer.command("toggle-auto-include")));
    await expect(page.locator("#auto-include")).not.toBeChecked();
    expect(await page.evaluate(() => JSON.parse(sessionStorage.getItem("fake-settings"))["auto-include"])).toBe(false);
  });

  test(`${platform}: Reload Library refreshes files and reports missing items without dropping edits`, async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform, pool: "1" });
    await page.evaluate(() => { window.fakeShell.libraryErrors = ["Cannot read library file /gone.inc"]; });
    await viewer.build(() => page.evaluate(() => window.viewer.command("reload-library")));
    expect((await viewer.calls("setPool")).some(([, reload]) => reload)).toBe(true);
    await expect(page.locator("#library-status")).toHaveText("Cannot read library file /gone.inc");
    await expect(page.locator("#level-name")).toHaveText("04-hierarchy.sp");
  });
}
