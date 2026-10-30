// Auto Include found several Library definitions of a subcircuit and chose none (the core lists them as the
// `candidates` of the instance's child): entering the instance opens a dialog, the same one for a key, a double
// click and a menu, which lists the files and lines of the definitions; the one chosen opens as a view of its own
// file, as a subcircuit defined in an included file does, and this netlist is not changed, so entering asks again.
// The fixture's Document comes from `rnl view`, which has no Library, so the candidates are put on it here.

import { expect, test } from "../viewer.js";
import { repository, translated, uiLanguages, zh } from "../zh.js";

const CANDIDATES = [
  { path: "/lib/adc_a.inc", file: "/lib/adc_a.inc", source: "lib/adc_a.inc", line: 3 },
  { path: "/lib/adc_b.inc", file: "/lib/adc_b.inc", source: "lib/adc_b.inc", line: 12 },
];
const NEW = ["Choose a definition of {0}",
  "{0} is defined in {1} Library files. The one you choose opens as a view of its own file, without the parameters of this instance; this netlist is not changed.",
  " instance of a subcircuit with {0} definitions in {1} · a double tap asks which to open",
  " instance of a subcircuit with {0} definitions in {1} · E or Enter asks which to open; Shift opens a new window"];

test.beforeEach(async ({ page, viewer }) => {
  await viewer.open("definitions-choice");
  await page.evaluate((candidates) => {
    for (const child of window.viewer.doc.levels[0].children) if (child.master === "adc_ref") child.candidates = candidates;
  }, CANDIDATES);
});

/** Select an instance by name, as a click on a list does: an instance with no definition is not drawn. */
async function pick(page, name) {
  await page.evaluate(async (name) => {
    const o = window.viewer.doc.objects.findIndex((x) => x.kind === "instance" && x.name === name);
    (await import("./selection.js")).select(o, "list", { usage: false });
  }, name);
}
const dialog = (page) => page.locator("dialog.dialog");
const options = (page) => dialog(page).locator(".dialog-option");

test("entering an instance with several definitions opens a dialog that lists their files and lines; Escape opens nothing", async ({ page, viewer }) => {
  await pick(page, "X1");
  await expect(page.locator("#selection")).toContainText("instance of a subcircuit with 2 definitions in");
  await expect(page.locator("#selection")).toContainText("E or Enter asks which to open");
  await page.keyboard.press("e");
  await expect(dialog(page)).toBeVisible();
  await expect(dialog(page).locator(".dialog-title")).toHaveText("Choose a definition of adc_ref");
  await expect(dialog(page)).toContainText("adc_ref is defined in 2 Library files");
  await expect(options(page)).toHaveText(["lib/adc_a.inc:3", "lib/adc_b.inc:12"]);
  await expect(options(page).first()).toHaveAttribute("aria-selected", "true");
  await page.keyboard.press("Escape");
  await expect(dialog(page)).toHaveCount(0);
  expect(await viewer.calls("sourceForPath")).toEqual([]);
  // nothing was opened, and this document is as it was
  expect(await page.evaluate(() => window.viewer.doc.levels.length)).toBe(1);
});

test("the arrow keys and Enter choose, and the file chosen is opened as a view of its own", async ({ page, viewer }) => {
  await pick(page, "X1");
  await page.keyboard.press("Enter");
  await expect(dialog(page)).toBeVisible();
  await page.keyboard.press("ArrowDown");
  await expect(options(page).nth(1)).toHaveAttribute("aria-selected", "true");
  await viewer.build(() => page.keyboard.press("Enter"));
  expect(await viewer.calls("sourceForPath")).toEqual([["/lib/adc_b.inc"]]);
});

test("a double click and a menu enter ask the same, and Shift sends the chosen file to a new window", async ({ page, viewer }) => {
  await page.locator("#tab-netlist").click();
  await page.locator("#source [data-obj]", { hasText: "X1" }).first().dblclick();
  await expect(dialog(page)).toBeVisible();
  await expect(options(page)).toHaveCount(2);
  await page.keyboard.press("Escape");
  await pick(page, "X2");
  await viewer.menuTitle("Edit").click();
  await expect(viewer.menuItem("descend-new-window")).toBeEnabled();
  await viewer.menuItem("descend-new-window").click();
  await expect(dialog(page)).toBeVisible();
  await options(page).first().dblclick();
  await expect.poll(() => viewer.calls("newWindow")).toHaveLength(1);
  const [[session]] = await viewer.calls("newWindow");
  expect(session.entry.path).toBe("/lib/adc_a.inc");
  expect(session.path.at(-1)).toMatchObject({ title: "adc_ref", via: "X2" });
});

test("an instance with one definition found, or none, is not asked about", async ({ page, viewer }) => {
  await page.evaluate(() => { for (const child of window.viewer.doc.levels[0].children) if (child.master === "adc_ref") child.candidates = child.candidates.slice(0, 1); });
  await pick(page, "X1");
  await expect(page.locator("#selection")).not.toContainText("asks which to open");
  await page.keyboard.press("e");
  await expect(dialog(page)).toHaveCount(0);
  await pick(page, "XO");
  await page.keyboard.press("e");
  await expect(dialog(page)).toHaveCount(0);
});

test("the texts are in every language of the interface, and the dialog follows the language", async ({ page, viewer }) => {
  for (const { lang } of uiLanguages(repository)) {
    if (lang === "en") continue;
    for (const english of NEW) expect(translated(lang, english), `${lang}: ${english}`).not.toBe("");
  }
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await pick(page, "X1");
  await page.keyboard.press("e");
  await expect(dialog(page).locator(".dialog-title")).toHaveText(zh("Choose a definition of {0}", "adc_ref"));
  await expect(dialog(page)).toContainText(zh(NEW[1], "adc_ref", 2));
  await page.keyboard.press("Escape");
  await expect(page.locator("#selection")).toContainText(zh(NEW[3], 2, "definitions-choice.sp"));
});
