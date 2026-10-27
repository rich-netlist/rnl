// The format "RNL package": a package file is one by its format, not by its suffix. In the Library a package
// file has a format chip like any other, so that a `.pkg` can be set to RNL package and a `.rnl` to another
// format; a package file opened under another suffix is highlighted as RNL; the title is translated.

import { expect, test } from "../viewer.js";
import { repository, translated, uiLanguages, zh } from "../zh.js";

const group = (page, key) => page.locator(`.lib-group[data-group=${JSON.stringify(key)}]`);
const menu = (page) => page.locator(".format-menu");
const TITLE = "RNL package";

/** What the text pane of the file shown marks as RNL, and whether its prose line is read as a netlist statement. */
function reading(page) {
  return page.evaluate(() => {
    const file = document.querySelector("#source .file:not([hidden])");
    return {
      records: file.querySelectorAll(".ln.rnl-line").length,
      keywords: file.querySelectorAll(".code .s-keyword").length,
      // line 5 of acme.pkg is prose between the records
      prose: file.querySelector('.ln[data-line="5"] .code')?.querySelectorAll('[class*="s-"]').length,
    };
  });
}

test("a package file opened under another suffix is a package: the format, the highlighting as RNL, the drawing area", async ({ page, viewer }) => {
  await viewer.open("package-pkg", { pool: "1" });
  const doc = await page.evaluate(() => ({ dialect: window.viewer.doc.dialect, format: window.viewer.doc.files[0].format }));
  expect(doc).toEqual({ dialect: "rnl", format: "rnl" });
  const shown = await reading(page);
  expect(shown.records).toBeGreaterThan(5);
  expect(shown.keywords).toBeGreaterThan(3);
  // its prose is no netlist statement
  expect(shown.prose).toBe(0);
  await expect(page.locator("#problems")).toContainText("No errors, warnings or hints");
});

test("a package file has a format chip in the Library: a .pkg can be set to RNL package, a .rnl to another format", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { pool: "1" });
  await page.evaluate(() => {
    window.fakeShell.detected["/lib/acme.pkg"] = "spice"; // by its suffix, before the core reads into it
    window.fakeShell.includes = ["acme.pkg", "styles.rnl"].map((name) => ({ path: `/lib/${name}`, name, source: { name: "04-hierarchy", fixture: "04-hierarchy" } }));
  });
  await page.locator("#tab-files").click();
  await viewer.build(() => page.locator("#include-file").click());
  const files = group(page, "files");
  const pkg = files.locator('.pool-row[data-path="/lib/acme.pkg"] .format-chip');
  const rnl = files.locator('.pool-row[data-path="/lib/styles.rnl"] .format-chip');
  await expect(pkg).toHaveText("SPICE");
  await expect(rnl).toHaveText(TITLE);
  await expect(rnl).not.toHaveClass(/chosen/);
  // the menu offers the format, between Spectre and Text
  await pkg.click();
  await expect(menu(page).locator(".format-label")).toHaveText(["Automatic (SPICE)", "SPICE", "Spectre", TITLE, "Text"]);
  await viewer.build(() => menu(page).getByRole("menuitemradio", { name: TITLE, exact: true }).click());
  await expect(pkg).toHaveText(TITLE);
  await expect(pkg).toHaveClass(/chosen/);
  expect(await page.evaluate(() => window.fakeShell.buildFormats.at(-1))).toEqual({ "/lib/acme.pkg": "rnl" });
  // a .rnl that is no package for its user
  await rnl.click();
  await viewer.build(() => menu(page).getByRole("menuitemradio", { name: "SPICE", exact: true }).click());
  await expect(rnl).toHaveText("SPICE");
  await expect(rnl).toHaveClass(/chosen/);
  expect(await page.evaluate(() => window.fakeShell.buildFormats.at(-1))).toEqual({ "/lib/acme.pkg": "rnl", "/lib/styles.rnl": "spice" });
});

test("a package is told by its format: the user's choice, else the suffix the core's formats list", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { pool: "1" });
  const told = (paths) => page.evaluate(async (paths) => {
    const { isPackagePath } = await import("./formats.js");
    return paths.map((p) => isPackagePath(p));
  }, paths);
  expect(await told(["/lib/a.rnl", "/lib/A.RNL", "/lib/a.pkg", "/lib/a.sp", "/lib/rnl", "/lib/a.rnl/b"])).toEqual([true, true, false, false, false, false]);
  await page.evaluate(async () => {
    const { chooseFormat } = await import("./formats.js");
    await chooseFormat("/lib/a.pkg", "rnl", { session: true });
    await chooseFormat("/lib/a.rnl", "spice", { session: true });
  });
  expect(await told(["/lib/a.rnl", "/lib/a.pkg"])).toEqual([false, true]);
});

test("the title RNL package is in every language of the interface and follows the language", async ({ page, viewer }) => {
  for (const { lang } of uiLanguages(repository)) if (lang !== "en") expect(translated(lang, TITLE), lang).not.toBe("");
  await viewer.open("package-pkg", { pool: "1" });
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await page.locator("#tab-files").click();
  const chip = group(page, "files").locator(".pool-row.current .format-chip");
  await expect(chip).toHaveText(zh(TITLE));
  await chip.click();
  await expect(menu(page).locator(".format-label")).toHaveText([zh("Automatic ({0})", zh(TITLE)), "SPICE", "Spectre", zh(TITLE), zh("Text")]);
});
