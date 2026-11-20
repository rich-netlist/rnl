// The files the netlist shown includes, listed in the Library under the file opened, which are no Library files
// (pool.js `includeRow`): a click on the name opens the file as an include link in the text does, with the Library
// still shown, and Add to Library adds it to Files as Include File would, after which it has a row of its own.
import { test, expect } from "../viewer.js";

const group = (page, key) => page.locator(`.lib-group[data-group=${JSON.stringify(key)}]`);
const LIB = { name: "04-hierarchy-lib.inc", path: "/04-hierarchy-lib.inc", fixture: "04-hierarchy" };

for (const platform of ["desktop", "web"]) {
  test(`${platform}: Add to Library makes an included file a file of the Library`, async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform, pool: "1" });
    await page.evaluate((lib) => { window.fakeShell.fileSources = { [lib.name]: lib }; }, LIB);
    await page.locator("#tab-files").click();
    const files = group(page, "files");
    const include = files.locator(".pool-include");
    await expect(include.locator(".pool-include-name")).toHaveText(LIB.name);
    await expect(include.getByRole("button", { name: `Open ${LIB.name}`, exact: true })).toBeVisible();

    // Add to Library adds the file to Files, and its grey row goes
    await viewer.build(() => include.getByRole("button", { name: `Add ${LIB.name} to the Library`, exact: true }).click());
    await expect(files.locator(".pool-row .pool-open")).toHaveText(["04-hierarchy", LIB.name]);
    await expect(files.locator(".pool-include")).toHaveCount(0);
  });

  test(`${platform}: the name of an included file opens it as the include link does, with the Library still shown`, async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform, pool: "1" });
    await page.evaluate((lib) => { window.fakeShell.fileSources = { [lib.name]: lib }; }, LIB);
    await page.locator("#tab-files").click();
    // (the fake shell reads whole documents: the file opens as an entry of its own, as on the desktop)
    await viewer.build(() => group(page, "files").locator(".pool-include").getByRole("button", { name: `Open ${LIB.name}`, exact: true }).click());
    expect(await page.evaluate(() => window.fakeShell.calls.filter((c) => c[0] === "build").at(-1)[1])).toBe(LIB.name);
    await expect(page.locator("#pane-files")).toBeVisible();
  });
}
