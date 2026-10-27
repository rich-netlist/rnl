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

test("a .pkg with a PACKAGE header gets the automatic format RNL package: a muted chip, and the menu says Automatic (RNL package)", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { pool: "1" });
  await page.evaluate(() => {
    // acme.pkg is the package of the fixture package-pkg; the shell gives the core its text, which says what it is
    window.fakeShell.includes = [{ path: "/lib/acme.pkg", name: "acme.pkg", source: { name: "acme.pkg", fixture: "package-pkg" } }];
  });
  await page.locator("#tab-files").click();
  await viewer.build(() => page.locator("#include-file").click());
  const chip = group(page, "files").locator('.pool-row[data-path="/lib/acme.pkg"] .format-chip');
  await expect(chip).toHaveText(TITLE);
  await expect(chip).not.toHaveClass(/chosen/);
  await chip.click();
  await expect(menu(page).locator(".format-label")).toHaveText([`Automatic (${TITLE})`, "SPICE", "Spectre", TITLE, "Text"]);
  await expect(menu(page).getByRole("menuitemradio", { name: `Automatic (${TITLE})` })).toHaveAttribute("aria-checked", "true");
  // set by hand to SPICE, the chip is solid; Automatic gives the package back
  await viewer.build(() => menu(page).getByRole("menuitemradio", { name: "SPICE", exact: true }).click());
  await expect(chip).toHaveText("SPICE");
  await expect(chip).toHaveClass(/chosen/);
  expect(await page.evaluate(() => window.fakeShell.buildFormats.at(-1))).toEqual({ "/lib/acme.pkg": "spice" });
  await chip.click();
  await viewer.build(() => menu(page).getByRole("menuitemradio", { name: `Automatic (${TITLE})` }).click());
  await expect(chip).toHaveText(TITLE);
  await expect(chip).not.toHaveClass(/chosen/);
});

test("a package file is told by its format in the Library: a .rnl can be set to another format, a file of any suffix to RNL package", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { pool: "1" });
  await page.evaluate(() => {
    window.fakeShell.includes = ["styles.rnl", "notes.dat"].map((name) => ({ path: `/lib/${name}`, name, source: { name: "04-hierarchy", fixture: "04-hierarchy" } }));
  });
  await page.locator("#tab-files").click();
  await viewer.build(() => page.locator("#include-file").click());
  const files = group(page, "files");
  const rnl = files.locator('.pool-row[data-path="/lib/styles.rnl"] .format-chip');
  const dat = files.locator('.pool-row[data-path="/lib/notes.dat"] .format-chip');
  // a .rnl is a package by its suffix, any other file a netlist unless its header says it is a package
  await expect(rnl).toHaveText(TITLE);
  await expect(rnl).not.toHaveClass(/chosen/);
  await expect(dat).toHaveText("SPICE");
  await dat.click();
  await viewer.build(() => menu(page).getByRole("menuitemradio", { name: TITLE, exact: true }).click());
  await expect(dat).toHaveText(TITLE);
  await expect(dat).toHaveClass(/chosen/);
  expect(await page.evaluate(() => window.fakeShell.buildFormats.at(-1))).toEqual({ "/lib/notes.dat": "rnl" });
  await rnl.click();
  await viewer.build(() => menu(page).getByRole("menuitemradio", { name: "SPICE", exact: true }).click());
  await expect(rnl).toHaveText("SPICE");
  await expect(rnl).toHaveClass(/chosen/);
  expect(await page.evaluate(() => window.fakeShell.buildFormats.at(-1))).toEqual({ "/lib/notes.dat": "rnl", "/lib/styles.rnl": "spice" });
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
