// Opening a document: the drawing, the text and the problems; a source that cannot be built; the
// page with nothing open; and the colours of the dark mode.

import { expect, test } from "../viewer.js";

test("opening shows the drawing, the text and the problems", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  expect(await viewer.calls("build")).toEqual([["04-hierarchy"]]);
  await expect(page.locator("#canvas > svg")).toHaveCount(1);
  await expect(page.locator("#empty")).toBeHidden();
  await expect(page.locator("#level-name")).toHaveText("04-hierarchy.sp");
  await expect(page.locator("#level-kind")).toHaveText("top level");
  await expect(page.locator("#zoom")).toHaveText(/^\d+%$/);
  // every symbol and wire of the level is drawn
  const ids = await page.evaluate(() => window.viewer.doc.levels[0].hits.filter((h) => h.id).map((h) => h.id));
  for (const id of ids) await expect(page.locator(`#canvas > svg [data-id="${id}"]`).first()).toBeAttached();

  // the files, the first one shown with all of its lines
  await expect(page.locator("#file-tabs button")).toHaveText(["04-hierarchy.sp", "04-hierarchy-lib.inc"]);
  await expect(page.locator("#file-tabs button").first()).toHaveAttribute("aria-selected", "true");
  const lines = await page.evaluate(() => window.viewer.doc.files[0].text.split(/\r?\n/).filter((l, i, a) => i < a.length - 1 || l).length);
  await expect(page.locator("#source .file:not([hidden]) .ln")).toHaveCount(lines);
  await expect(viewer.line(48)).toHaveText(/X1 a b vdd 0 amp/);
  await expect(page.locator("#counts")).toHaveText("none");
  await expect(page.locator("#diags")).toHaveText("No errors, warnings or hints.");

  // the other file
  await page.locator("#file-tabs button", { hasText: "04-hierarchy-lib.inc" }).click();
  await expect(viewer.line(2)).toHaveText(/\.subckt rdiv top mid bot/);
});

test("the problems are listed, the most serious first, and their lines are coloured", async ({ page, viewer }) => {
  await viewer.open("12-diagnostics");
  const diags = await page.evaluate(() => window.viewer.doc.diagnostics);
  const count = (s) => diags.filter((d) => d.severity === s).length;
  const [errors, warnings, hints] = [count("error"), count("warning"), count("hint")];
  expect(errors && warnings && hints).toBeTruthy();
  await expect(page.locator("#counts")).toHaveText(`${errors} errors${warnings} warnings${hints} hints`);
  const items = page.locator("#diags li.diag");
  await expect(items).toHaveCount(diags.length);
  const shown = await items.evaluateAll((els) => els.map((e) => e.classList[1]));
  expect(shown).toEqual([...Array(errors).fill("error"), ...Array(warnings).fill("warning"), ...Array(hints).fill("hint")]);
  // an error's location, and its line number in the error colour
  const first = diags.find((d) => d.severity === "error" && d.line);
  await expect(items.filter({ hasText: first.message }).first()).toContainText(`${first.file}:${first.line}:${first.col}`);
  await expect(viewer.line(first.line)).toHaveClass(/\bd-error\b/);
  // a level without positions is not drawn
  await page.locator("#tab-hierarchy").click();
  await page.locator("#index .index-row", { hasText: "sub" }).click();
  await expect(page.locator("#empty")).toBeVisible();
  await expect(page.locator("#empty")).toHaveText("Nothing on this level has a position yet, so nothing is drawn.");
  await expect(page.locator("#status")).toHaveText("Not drawn without a solver: 3 without a position");
});

test("a Spectre netlist is drawn with its subcircuit", async ({ page, viewer }) => {
  await viewer.open("inverter-chain");
  await expect(page.locator("#level-name")).toHaveText("inverter-chain.scs");
  await expect(viewer.line(5)).toHaveText(/subckt inv \(a y vdd vss\)/);
  await viewer.click("X1");
  await page.keyboard.press("Enter");
  await expect(page.locator("#level-name")).toHaveText("inv");
  await expect(page.locator("#canvas > svg")).toHaveCount(1);
});

test("a build that fails is reported with the source's name", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await page.keyboard.press("Control+Shift+M"); // folded problems unfold to show the failure
  await expect(page.locator("#problems-head")).toHaveAttribute("aria-expanded", "false");
  await page.evaluate(() => {
    window.fakeShell.picks.push({ name: "/work/missing.sp", fixture: "missing" });
    window.fakeShell.fail = "/work/missing.sp: The system cannot find the file specified. (os error 2)";
  });
  await viewer.build(() => page.keyboard.press("Control+O"));
  await expect(page.locator("#empty")).toBeVisible();
  await expect(page.locator("#empty")).toHaveText("Could not open /work/missing.sp");
  await expect(page.locator("#canvas > svg")).toHaveCount(0);
  await expect(page.locator("#source")).toHaveText("The file could not be read.");
  await expect(page.locator("#problems-head")).toHaveAttribute("aria-expanded", "true");
  await expect(page.locator("#diags li.diag.error")).toHaveText(/The system cannot find the file specified/);
  await expect(page.locator("#counts")).toHaveText("1 error");
  await expect(page.locator("#level-name")).toHaveText("");
  expect(await page.evaluate(() => window.viewer.doc)).toBeNull();

  // reading it again tries the same source
  await viewer.build(() => page.keyboard.press("F5"));
  expect((await viewer.calls("build")).at(-1)).toEqual(["/work/missing.sp"]);
  await expect(page.locator("#empty")).toHaveText("Could not open /work/missing.sp");
});

test("a source the shell cannot find at start is reported", async ({ page, viewer }) => {
  await viewer.open(null, { open: "no-such-netlist" });
  await expect(page.locator("#empty")).toHaveText("Could not open no-such-netlist");
  await expect(page.locator("#diags li.diag.error")).toHaveText(/no-such-netlist: there is no such fixture/);
});

test("a start that fails is reported, also with a plain message", async ({ page, viewer }) => {
  await viewer.open(null, { "fail-start": "the command line names no readable file" });
  await expect(page.locator("#empty")).toHaveText("the command line names no readable file");
  expect(await viewer.calls("build")).toEqual([]);

  // a shell that cannot be loaded
  await page.goto(`${process.env.RNL_VIEWER_URL}/index.html?shell=nosuch`);
  await expect(page.locator("#empty")).toHaveText(/^The page could not load its "nosuch" shell: /);
  await page.goto(`${process.env.RNL_VIEWER_URL}/index.html?shell=..%2Ffake`);
  await expect(page.locator("#empty")).toHaveText('"../fake" is not the name of a shell.');
});

test("with nothing open, the page says how to open a netlist", async ({ page, viewer }) => {
  await viewer.open(null);
  await expect(page.locator("#empty")).toContainText("Open a SPICE or Spectre netlist");
  await expect(page.locator("#source")).toHaveText("No file is open.");
  await expect(page.locator("#diags")).toHaveText("No file is open.");
  expect(await viewer.calls("build")).toEqual([]);
  // the keys that act on a document do nothing
  for (const key of ["f", "Control+=", "Enter", "Backspace", "Escape", "F5"]) await page.keyboard.press(key);
  expect(await viewer.calls("build")).toEqual([]);
  await expect(page.locator("#zoom")).toHaveText("");
});

test.describe("in dark mode", () => {
  test.use({ colorScheme: "dark" });

  test("the page takes the dark colours", async ({ page, viewer }) => {
    await viewer.open("12-diagnostics");
    const colours = () =>
      page.evaluate(() => {
        const style = (sel) => getComputedStyle(document.querySelector(sel));
        return {
          scheme: style(":root").colorScheme,
          body: style("body").backgroundColor,
          text: style("body").color,
          panel: style("#sidebar").backgroundColor,
          search: style("#search").backgroundColor,
          error: style("#diags .diag.error .sev").color,
          canvas: style("#canvas").backgroundColor,
        };
      });
    expect(await colours()).toEqual({
      scheme: "dark",
      body: "rgb(31, 31, 34)",
      text: "rgb(232, 232, 234)",
      panel: "rgb(35, 35, 39)",
      search: "rgb(43, 43, 48)",
      error: "rgb(239, 107, 107)",
      // the drawing stays on white
      canvas: "rgb(255, 255, 255)",
    });
    await page.emulateMedia({ colorScheme: "light" });
    expect(await colours()).toEqual({
      scheme: "light",
      body: "rgb(244, 244, 242)",
      text: "rgb(29, 29, 31)",
      panel: "rgb(251, 251, 250)",
      search: "rgb(255, 255, 255)",
      error: "rgb(198, 40, 40)",
      canvas: "rgb(255, 255, 255)",
    });
  });
});
