// A file that is no netlist, Verilog-A or Verilog, opened as the document, is shown in the text pane as
// plain text: the core does not read it, and its one hint says so; nothing in the text is marked as a
// netlist's (colours, the marks of objects and of what is not drawn). A netlist is
// shown as before; a file the user chose a parse format for in the Library is read as a netlist again.

import { expect, test } from "../viewer.js";
import { repository, translated, uiLanguages, zh } from "../zh.js";

const NOTICE = "This file is not a netlist. It is shown as text.";

/** What the text pane of the file shown marks as a netlist: highlighted pieces, objects' places, undrawn lines. */
function reading(page) {
  return page.evaluate(() => {
    const file = document.querySelector("#source .file:not([hidden])");
    return {
      highlighted: file.querySelectorAll('.code [class*="s-"]').length,
      places: file.querySelectorAll("[data-obj]").length,
      notDrawn: file.querySelectorAll(".ln.not-drawn").length,
      folds: file.querySelectorAll(".fold-toggle").length,
      lines: file.querySelectorAll(".ln").length,
    };
  });
}

for (const [fixture, name, language] of [["verilog-a", "resistor.va", "Verilog-A"], ["verilog", "inverter.v", "Verilog"]]) {
  test(`${name} is shown as plain text, while the core's problems are still listed`, async ({ page, viewer }) => {
    await viewer.open(fixture);
    await expect(page.locator("#source-file")).toHaveText(name);
    const shown = await reading(page);
    expect(shown.lines).toBeGreaterThan(3);
    expect(shown).toMatchObject({ highlighted: 0, places: 0, notDrawn: 0, folds: 0 });
    // the text itself is whole
    await expect(viewer.line(1).locator(".code")).toContainText("in Verilog");
    // the core's reading is told where it is told: in the problems, which the lines still mark
    expect(await page.evaluate(() => window.viewer.doc.diagnostics.length)).toBeGreaterThan(0);
    await expect(page.locator("#problems")).toContainText(`\`${name}\` is not a netlist (${language}); it is not read`);
    await expect(page.locator("#problems")).not.toContainText("SPICE subset");
    // the drawing area says so, instead of that nothing has a position
    await expect(page.locator("#empty")).toHaveText(NOTICE);
  });
}

test("the notice of the drawing area is in every language of the interface, and follows the language", async ({ page, viewer }) => {
  for (const { lang } of uiLanguages(repository)) if (lang !== "en") expect(translated(lang, NOTICE), lang).not.toBe("");
  await viewer.open("verilog-a");
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  // the language is kept for the next start
  await viewer.open("verilog-a");
  await expect(page.locator("#empty")).toHaveText(zh(NOTICE));
});

test("a netlist is highlighted as before", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  const shown = await reading(page);
  expect(shown.highlighted).toBeGreaterThan(10);
  expect(shown.places).toBeGreaterThan(0);
  await expect(page.locator("#empty")).toBeHidden();
});

test("a file with a parse format chosen for it in the Library is read as a netlist", async ({ page, viewer }) => {
  await viewer.open("verilog-a");
  expect(await reading(page)).toMatchObject({ highlighted: 0 });
  await page.evaluate(async () => {
    const { chooseFormat } = await import("./formats.js");
    const { sourcePath } = await import("./source-path.js");
    const { renderFiles } = await import("./text.js");
    const doc = window.viewer.doc;
    await chooseFormat(sourcePath(doc, doc.files[0]), "spice", { session: true });
    renderFiles();
  });
  await expect.poll(async () => (await reading(page)).highlighted).toBeGreaterThan(0);
});
