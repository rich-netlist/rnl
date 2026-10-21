import { test, expect } from "../viewer.js";

for (const platform of ["desktop", "web"]) {
  test(`${platform}: Include File/Folder, open from pool, Exclude and immutable built-ins`, async ({ page, viewer }) => {
    await viewer.open(null, { platform, pool: "1" });
    await expect(page.locator(".view-tabs [role=tab]").first()).toHaveAccessibleName("Library");
    await page.evaluate(() => {
      window.fakeShell.picks = [{ name: "04-hierarchy", fixture: "04-hierarchy" }];
      window.viewer.command("open");
    });
    await viewer.idle();
    await page.locator("#tab-files").click();
    await expect(page.locator("#pool-list .pool-row")).toHaveCount(1);
    await page.evaluate(() => {
      window.fakeShell.includes = [{ path: "/lib/01.sp", name: "01.sp", source: { name: "01-rc-lowpass", fixture: "01-rc-lowpass" } }];
    });
    await viewer.build(() => page.locator("#include-file").click());
    await expect(page.locator("#pool-list .pool-row")).toHaveCount(2);
    await viewer.build(() => page.getByRole("button", { name: "Open 01.sp", exact: true }).click());
    await expect(page.locator("#level-name")).toHaveText("01-rc-lowpass.sp");
    await page.locator("#tab-files").click();
    await viewer.build(() => page.getByRole("button", { name: "Exclude 01.sp", exact: true }).click());
    expect(await page.evaluate(() => window.fakeShell.pool)).toHaveLength(1);
    await expect(page.locator("#level-name")).toHaveText("01-rc-lowpass.sp");
    await expect(page.getByRole("button", { name: "Exclude 01.sp", exact: true })).toHaveCount(0);
    await page.evaluate(() => {
      window.fakeShell.includes = ["a.sp", "b.rnl"].map((name) => ({ path: "/folder/" + name, name, source: { name: "04-hierarchy", fixture: "04-hierarchy" } }));
    });
    await viewer.build(() => page.locator("#include-folder").click());
    expect(await viewer.calls("include")).toEqual([["file"], ["folder"]]);
    expect((await page.evaluate(() => window.fakeShell.pool)).slice(1)).toEqual(["/folder/a.sp", "/folder/b.rnl"]);
    const builtin = page.locator('[data-group="builtin"]');
    await expect(builtin.locator(".lib-toggle")).toHaveAttribute("aria-expanded", "false");
    await builtin.locator(".lib-toggle").click();
    await expect(builtin.locator(".pool-badge")).toHaveText("read only");
    await expect(builtin.getByRole("button", { name: /Exclude/ })).toHaveCount(0);
  });
}

test("desktop migrates browser settings once and persists ordered PATH with the other settings", async ({ page, viewer }) => {
  await viewer.open(null, { platform: "desktop" });
  await page.evaluate(() => {
    localStorage.setItem("solve-options", JSON.stringify({ grid: 3 }));
    localStorage.setItem("grid-style", JSON.stringify("lines"));
    localStorage.setItem("search-paths", JSON.stringify(["/first", "/second"]));
  });
  await viewer.open(null, { platform: "desktop", settings: "1", pool: "1" });
  expect(await page.evaluate(() => window.fakeShell.legacy["solve-options"])).toEqual({ grid: 3 });
  expect(await page.evaluate(() => localStorage.getItem("solve-options"))).toBeNull();
  await page.locator("#tab-files").click();
  await expect(page.locator("#path-note")).toContainText("/user/config/settings.json");
  await page.locator('[data-group="path:/first"]').getByRole("button", { name: "Move down in PATH", exact: true }).click();
  await expect(page.locator(".lib-path")).toHaveText(["/second", "/first"]);
  await page.evaluate(() => { window.fakeShell.path = "/third"; });
  await page.locator("#add-path").click();
  await expect(page.locator(".lib-path")).toHaveText(["/second", "/first", "/third"]);
  await page.locator('[data-group="path:/first"]').getByRole("button", { name: "Remove from PATH", exact: true }).click();
  await expect(page.locator(".lib-path")).toHaveText(["/second", "/third"]);
  await viewer.open(null, { platform: "desktop", settings: "1", pool: "1" });
  await expect(page.locator(".lib-path")).toHaveText(["/second", "/third"]);
  expect(await page.evaluate(() => window.viewer.grid.style)).toBe("lines");
  expect(await page.evaluate(() => JSON.parse(sessionStorage.getItem("fake-settings"))["solve-options"])).toEqual({ grid: 3 });
});
