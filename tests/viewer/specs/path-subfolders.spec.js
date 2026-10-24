// Desktop PATH directories added with their sub-folders: the two Add PATH buttons, the badge of a
// group added with sub-folders, its listing through the shell, the setting that keeps it, and the
// note when a directory has more sub-folders than are searched.
import { test, expect } from "../viewer.js";

const group = (page, key) => page.locator(`.lib-group[data-group=${JSON.stringify(key)}]`);
const stored = (page, key) => page.evaluate((key) => JSON.parse(sessionStorage.getItem("fake-settings"))[key], key);

test("desktop: Add PATH with Sub-folders lists a directory's sub-folders, is kept, and loses the mark when removed", async ({ page, viewer }) => {
  await viewer.open(null, { platform: "desktop", pool: "1", settings: "1" });
  await page.evaluate(() => {
    window.fakeShell.packages = { "/flat": ["/flat/a.rnl"], "/pdk": ["/pdk/top.rnl"] };
    window.fakeShell.deepPackages = { "/pdk": ["/pdk/top.rnl", "/pdk/cells/inv.sp", "/pdk/models/nmos.lib"] };
  });
  await page.locator("#tab-files").click();
  await expect(page.locator("#add-path")).toHaveText("Add PATH…");
  await expect(page.locator("#add-path-subfolders")).toHaveText("Add PATH with Sub-folders…");
  await page.evaluate(() => { window.fakeShell.path = "/flat"; });
  await viewer.build(() => page.locator("#add-path").click());
  await page.evaluate(() => { window.fakeShell.path = "/pdk"; });
  await viewer.build(() => page.locator("#add-path-subfolders").click());
  expect(await stored(page, "search-paths")).toEqual(["/flat", "/pdk"]);
  expect(await stored(page, "search-paths-subfolders")).toEqual(["/pdk"]);
  await expect(group(page, "path:/pdk").locator(".lib-deep")).toHaveText("with sub-folders");
  await expect(group(page, "path:/flat").locator(".lib-deep")).toHaveCount(0);
  await group(page, "path:/pdk").locator(".lib-toggle").click();
  await expect(group(page, "path:/pdk").locator(".pool-open")).toHaveText(["top.rnl", "cells/inv.sp", "models/nmos.lib"]);
  const calls = await page.evaluate(() => window.fakeShell.calls.filter((c) => c[0] === "pathPackages"));
  expect(calls).toContainEqual(["pathPackages", "/pdk", "subfolders"]);
  expect(calls).toContainEqual(["pathPackages", "/flat"]);

  // adding a known directory with its sub-folders marks it; adding it again plainly changes nothing
  await page.evaluate(() => { window.fakeShell.path = "/flat"; });
  await viewer.build(() => page.locator("#add-path-subfolders").click());
  expect(await stored(page, "search-paths")).toEqual(["/flat", "/pdk"]);
  expect((await stored(page, "search-paths-subfolders")).sort()).toEqual(["/flat", "/pdk"]);
  const builds = await viewer.builds();
  await page.locator("#add-path").click();
  expect(await viewer.builds()).toBe(builds);

  // removing a PATH entry forgets its mark
  await viewer.build(() => group(page, "path:/pdk").getByRole("button", { name: "Remove from PATH", exact: true }).click());
  expect(await stored(page, "search-paths")).toEqual(["/flat"]);
  expect(await stored(page, "search-paths-subfolders")).toEqual(["/flat"]);
});

test("desktop: a directory with more sub-folders than are searched says so", async ({ page, viewer }) => {
  await viewer.open(null, { platform: "desktop", pool: "1" });
  await page.evaluate(() => {
    localStorage.setItem("search-paths", JSON.stringify(["/huge"]));
    localStorage.setItem("search-paths-subfolders", JSON.stringify(["/huge"]));
    window.fakeShell.packages = { "/huge": ["/huge/a.rnl"] };
    window.fakeShell.truncated = { "/huge": true };
  });
  await page.locator("#tab-files").click();
  const huge = group(page, "path:/huge");
  await huge.locator(".lib-toggle").click();
  await expect(huge.locator(".lib-warning")).toHaveText("Only the first 500 sub-folders are searched; add deeper folders as PATH entries of their own.");
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect(page.locator("#add-path-subfolders")).toHaveText("添加 PATH（含子目录）…");
  await expect(huge.locator(".lib-deep")).toHaveText("含子目录");
});

test("web: there is no PATH, so neither Add PATH button shows", async ({ page, viewer }) => {
  await viewer.open(null, { pool: "1" });
  await page.locator("#tab-files").click();
  await expect(page.locator("#add-path")).toBeHidden();
  await expect(page.locator("#add-path-subfolders")).toBeHidden();
});
