// The parse format of each file in the Library: the chip of each netlist row and of each file the
// netlist shown includes, the menu of the core's formats, the choice that every build is given, and
// where the choice is kept.
import { test, expect } from "../viewer.js";
import { zh } from "../zh.js";

const group = (page, key) => page.locator(`.lib-group[data-group=${JSON.stringify(key)}]`);
const menu = (page) => page.locator(".format-menu");
/** The Library paths of the host files of the Document shown, as the page keys their formats. */
const keys = (page) => page.evaluate(async () => {
  const { sourcePath } = await import("./source-path.js");
  return window.viewer.doc.files.map((f) => sourcePath(window.viewer.doc, f));
});
const lastFormats = (page) => page.evaluate(() => window.fakeShell.buildFormats.at(-1));

for (const platform of ["desktop", "web"]) {
  test(`${platform}: each netlist and its includes show a format chip whose menu chooses and resets the format`, async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform, pool: "1" });
    await page.locator("#tab-files").click();
    const files = group(page, "files");
    const root = files.locator(".pool-row.current .format-chip");
    const include = files.locator(".pool-include .format-chip");
    await expect(root).toHaveText("SPICE");
    await expect(root).not.toHaveClass(/chosen/);
    await expect(files.locator(".pool-include")).toHaveCount(1);
    await expect(files.locator(".pool-include-name")).toHaveText("04-hierarchy-lib.inc");
    await expect(include).toHaveText("SPICE");
    expect(await lastFormats(page)).toEqual({});

    await include.click();
    await expect(menu(page).locator(".format-label")).toHaveText(["Automatic (SPICE)", "SPICE", "Spectre"]);
    await expect(menu(page).getByRole("menuitemradio", { name: "Automatic (SPICE)" })).toHaveAttribute("aria-checked", "true");
    await expect(menu(page).getByRole("menuitemradio", { name: "Automatic (SPICE)" })).toBeFocused();
    await viewer.build(() => menu(page).getByRole("menuitemradio", { name: "Spectre", exact: true }).click());
    await expect(menu(page)).toHaveCount(0);
    const [, inc] = await keys(page);
    expect(await lastFormats(page)).toEqual({ [inc]: "spectre" });
    // the Document reads the file in the format chosen, and the chip says it was chosen
    expect(await page.evaluate(() => window.viewer.doc.files.map((f) => f.format))).toEqual(["spice", "spectre"]);
    await expect(include).toHaveText("Spectre");
    await expect(include).toHaveClass(/chosen/);
    await expect(root).not.toHaveClass(/chosen/);

    // choosing what is already chosen builds nothing; Automatic forgets the choice
    const builds = await viewer.builds();
    await include.click();
    await expect(menu(page).getByRole("menuitemradio", { name: "Spectre", exact: true })).toHaveAttribute("aria-checked", "true");
    await menu(page).getByRole("menuitemradio", { name: "Spectre", exact: true }).click();
    expect(await viewer.builds()).toBe(builds);
    await include.click();
    await viewer.build(() => menu(page).getByRole("menuitemradio", { name: "Automatic (SPICE)" }).click());
    expect(await lastFormats(page)).toEqual({});
    await expect(include).toHaveText("SPICE");
    await expect(include).not.toHaveClass(/chosen/);
  });
}

test("the format menu follows the keys and closes on Escape or a click elsewhere", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { pool: "1" });
  await page.locator("#tab-files").click();
  const chip = group(page, "files").locator(".pool-row.current .format-chip");
  await chip.click();
  await page.keyboard.press("ArrowDown");
  await expect(menu(page).getByRole("menuitemradio", { name: "SPICE", exact: true })).toBeFocused();
  await page.keyboard.press("ArrowUp");
  await page.keyboard.press("ArrowUp");
  await expect(menu(page).getByRole("menuitemradio", { name: "Spectre", exact: true })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(menu(page)).toHaveCount(0);
  await expect(chip).toBeFocused();
  await chip.click();
  await page.locator("#canvas").click({ position: { x: 5, y: 5 } });
  await expect(menu(page)).toHaveCount(0);
  expect(await viewer.builds()).toBe(1);
});

test("desktop: a choice is kept in the settings by its path and given to the next start; unknown formats are left out", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { platform: "desktop", pool: "1", settings: "1" });
  await page.locator("#tab-files").click();
  const [main] = await keys(page);
  await page.locator(".pool-row.current .format-chip").click();
  await viewer.build(() => menu(page).getByRole("menuitemradio", { name: "Spectre", exact: true }).click());
  const stored = await page.evaluate(() => JSON.parse(sessionStorage.getItem("fake-settings"))["file-formats"]);
  expect(stored).toEqual({ [main]: "spectre" });
  // a name the core does not list, such as one a newer viewer kept, reads by default
  await page.evaluate(() => {
    const values = JSON.parse(sessionStorage.getItem("fake-settings"));
    values["file-formats"]["/elsewhere/x.va"] = "verilog-a";
    sessionStorage.setItem("fake-settings", JSON.stringify(values));
  });
  await viewer.open("04-hierarchy", { platform: "desktop", pool: "1", settings: "1" });
  expect(await lastFormats(page)).toEqual({ [main]: "spectre" });
  await page.locator("#tab-files").click();
  await expect(page.locator(".pool-row.current .format-chip")).toHaveText("Spectre");
  await expect(page.locator(".pool-row.current .format-chip")).toHaveClass(/chosen/);
});

test("web: a file the shell cannot name again keeps its choice only while the page is open", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { pool: "1" });
  await page.evaluate(() => { window.fakeShell.keepsFormat = false; });
  await page.locator("#tab-files").click();
  const [main] = await keys(page);
  await page.locator(".pool-row.current .format-chip").click();
  await viewer.build(() => menu(page).getByRole("menuitemradio", { name: "Spectre", exact: true }).click());
  expect(await lastFormats(page)).toEqual({ [main]: "spectre" });
  expect(JSON.parse(await page.evaluate(() => localStorage.getItem("file-formats")) || "{}")).toEqual({});
  await viewer.build(() => page.evaluate(() => window.viewer.command("reload")));
  expect(await lastFormats(page)).toEqual({ [main]: "spectre" });
  await viewer.open("04-hierarchy", { pool: "1" });
  expect(await lastFormats(page)).toEqual({});
});

test("desktop: a PATH netlist shows the format the shell detects; packages have no chip", async ({ page, viewer }) => {
  await viewer.open(null, { platform: "desktop", pool: "1" });
  await page.evaluate(() => {
    localStorage.setItem("search-paths", JSON.stringify(["/pdk"]));
    window.fakeShell.packages["/pdk"] = ["/pdk/models.lib", "/pdk/cells.scs", "/pdk/std.rnl"];
    window.fakeShell.detected["/pdk/models.lib"] = "spectre";
  });
  await page.locator("#tab-files").click();
  const pdk = group(page, "path:/pdk");
  await pdk.locator(".lib-toggle").click();
  await expect(pdk.locator(".format-chip")).toHaveText(["Spectre", "Spectre"]);
  await expect(pdk.locator(".pool-row", { hasText: "std.rnl" }).locator(".format-chip")).toHaveCount(0);
  const calls = await page.evaluate(() => window.fakeShell.calls.filter((c) => c[0] === "detectFormat").map((c) => c[1]));
  expect(calls.sort()).toEqual(["/pdk/cells.scs", "/pdk/models.lib"]);
});

test("format labels follow the language", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { pool: "1" });
  await page.locator("#tab-files").click();
  await page.evaluate(() => window.viewer.command("language-zh"));
  const chip = page.locator(".pool-row.current .format-chip");
  await expect(chip).toHaveAttribute("title", zh("Parse format: detected automatically"));
  await chip.click();
  await expect(menu(page).getByRole("menuitemradio").first()).toHaveAccessibleName(zh("Automatic ({0})", "SPICE"));
});

// RNL_REVIEW_DIR saves Chromium's screenshots of the Library's format chips and menu there, in light
// and dark, for review.
for (const scheme of ["light", "dark"]) {
  test(`screenshots of the format chips, ${scheme}`, async ({ page, viewer }, info) => {
    test.skip(!process.env.RNL_REVIEW_DIR || info.project.name !== "chromium", "only for review, in Chromium");
    await page.emulateMedia({ colorScheme: scheme });
    // with the settings store, as the real desktop has, so that the Library names its file
    await viewer.open("04-hierarchy", { platform: "desktop", pool: "1", settings: "1" });
    await page.locator("#tab-files").click();
    const sidebar = page.locator(".sidebar");
    const dir = process.env.RNL_REVIEW_DIR;
    await sidebar.screenshot({ path: `${dir}/formats-${scheme}-auto.png` });
    await page.locator(".pool-include .format-chip").click();
    await expect(menu(page)).toBeVisible();
    await page.screenshot({ path: `${dir}/formats-${scheme}-menu.png`, clip: await sidebar.boundingBox() });
    await viewer.build(() => menu(page).getByRole("menuitemradio", { name: "Spectre", exact: true }).click());
    await sidebar.screenshot({ path: `${dir}/formats-${scheme}-chosen.png` });
  });
}
