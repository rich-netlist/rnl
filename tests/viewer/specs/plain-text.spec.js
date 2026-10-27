// A file that is no netlist, Verilog-A or Verilog, has the format text for the core, which does not read it
// (one hint says so); the text pane shows it as plain text and the drawing area says it is no netlist. The
// viewer takes the core's verdict, the file's format: a file the user sets to Text is plain too, and a
// Verilog-A file set to SPICE is a netlist again. A netlist is shown as before.

import { expect, showSource, test } from "../viewer.js";
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
      marked: file.querySelectorAll(".ln.d-error, .ln.d-warning").length,
      folds: file.querySelectorAll(".fold-toggle").length,
      lines: file.querySelectorAll(".ln").length,
    };
  });
}

for (const [fixture, name, kind] of [["verilog-a", "resistor.va", "Verilog-A"], ["verilog", "inverter.v", "Verilog"]]) {
  test(`${name} is text for the core and is shown as plain text, with one hint and nothing not drawn`, async ({ page, viewer }) => {
    await viewer.open(fixture);
    await expect(page.locator("#source-file")).toHaveText(name);
    const shown = await reading(page);
    expect(shown.lines).toBeGreaterThan(3);
    expect(shown).toMatchObject({ highlighted: 0, places: 0, notDrawn: 0, marked: 0, folds: 0 });
    await expect(viewer.line(1).locator(".code")).toContainText("in Verilog");
    const doc = await page.evaluate(() => ({ dialect: window.viewer.doc.dialect, format: window.viewer.doc.files[0].format,
      diagnostics: window.viewer.doc.diagnostics.map((d) => `${d.severity}: ${d.message}`) }));
    expect(doc).toMatchObject({ dialect: "text", format: "text" });
    expect(doc.diagnostics).toHaveLength(1);
    expect(doc.diagnostics[0]).toContain(`is not a netlist (${kind}); it is not read`);
    // only that hint is listed; nothing is called not drawn, in the tab, in the status bar or by the drawing area
    await expect(page.locator("#problems")).toContainText("is not a netlist");
    await expect(page.locator("#problems")).not.toContainText("SPICE subset");
    await expect(page.locator("#unplaced-tab-count")).toHaveText("");
    await expect(page.locator(".statusbar")).not.toContainText("Not drawn");
    await expect(page.locator("#empty")).toHaveText(NOTICE);
  });
}

test("a netlist is highlighted as before", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  const shown = await reading(page);
  expect(shown.highlighted).toBeGreaterThan(10);
  expect(shown.places).toBeGreaterThan(0);
  await expect(page.locator("#empty")).toBeHidden();
});

test("a Verilog-A file set to SPICE in the Library is a netlist again, and Text sets a netlist to plain text", async ({ page, viewer }) => {
  await viewer.open("verilog-a", { pool: "1" });
  await page.locator("#tab-files").click();
  const chip = page.locator('.lib-group[data-group="files"] .pool-row.current .format-chip');
  await expect(chip).toHaveText("Text");
  await chip.click();
  await expect(page.locator(".format-menu .format-label")).toHaveText(["Automatic (Text)", "SPICE", "Spectre", "RNL package", "Text"]);
  await page.evaluate(() => { window.fakeShell.next = "verilog-a-spice"; });
  await viewer.build(() => page.locator(".format-menu").getByRole("menuitemradio", { name: "SPICE", exact: true }).click());
  await page.locator("#tab-netlist").click();
  expect(await page.evaluate(() => window.viewer.doc.files[0].format)).toBe("spice");
  await expect.poll(async () => (await reading(page)).highlighted).toBeGreaterThan(0);
  await expect(page.locator("#empty")).not.toHaveText(NOTICE);
});

test("a Verilog-A file that a netlist includes is plain text in its own tab, and the netlist is not", async ({ page, viewer }) => {
  await viewer.open("includes-verilog-a");
  expect(await page.evaluate(() => window.viewer.doc.files.map((f) => `${f.name}:${f.format}`))).toEqual(["includes-verilog-a.sp:spice", "resistor.va:text"]);
  expect((await reading(page)).highlighted).toBeGreaterThan(0);
  await expect(page.locator("#empty")).toBeHidden();
  await showSource(page, "resistor.va");
  await expect(page.locator("#source-file")).toHaveText("resistor.va");
  expect(await reading(page)).toMatchObject({ highlighted: 0, places: 0, notDrawn: 0, folds: 0 });
  // the hint is at the include line
  const hint = await page.evaluate(() => window.viewer.doc.diagnostics.find((d) => d.message.includes("not a netlist")));
  expect(hint).toMatchObject({ line: 2 });
});

test("the format Text and the notice are in every language of the interface and follow the language", async ({ page, viewer }) => {
  for (const { lang } of uiLanguages(repository)) {
    if (lang === "en") continue;
    expect(translated(lang, NOTICE), lang).not.toBe("");
    expect(translated(lang, "Text"), lang).not.toBe("");
  }
  await viewer.open("verilog-a", { pool: "1" });
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await expect(page.locator("#empty")).toHaveText(zh(NOTICE));
  await page.locator("#tab-files").click();
  const chip = page.locator('.lib-group[data-group="files"] .pool-row.current .format-chip');
  await expect(chip).toHaveText(zh("Text"));
  await chip.click();
  // the core's title of the RNL package format, not translated yet
  await expect(page.locator(".format-menu .format-label")).toHaveText([zh("Automatic ({0})", zh("Text")), "SPICE", "Spectre", "RNL package", zh("Text")]);
});
