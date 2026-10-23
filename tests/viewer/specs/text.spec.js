import { showSource } from "../viewer.js";
// The text pane: the highlighting of the host statements and the records, the statements over
// several lines, the statements that do not take effect and the box that marks them, the problems
// that show their lines, the file tabs, a line that waits for the text pane to be shown, and the
// places after characters outside the BMP, which JavaScript counts as two UTF-16 units.

import { CJK } from "../fixtures.js";
import { expect, test } from "../viewer.js";

const TOP = "04-hierarchy.sp";

/**
 * The highlighting of a line of the file shown: the pieces of its text that syntax.js gives a class,
 * each as [text, class] without the "s-", in order; the text between them is left out.
 */
function highlighted(viewer, n) {
  return viewer.line(n).locator(".code").evaluate((code) => {
    const pieces = [];
    for (const node of code.childNodes) {
      const s = node.nodeType === 1 ? [...node.classList].find((c) => c.startsWith("s-")) : undefined;
      const cls = s ? s.slice(2) : "";
      const last = pieces[pieces.length - 1];
      // a name of an object or a failed statement splits a piece into elements
      if (last && last[1] === cls) last[0] += node.textContent;
      else pieces.push([node.textContent, cls]);
    }
    return pieces.filter(([, cls]) => cls);
  });
}

test("the host statements and the records are highlighted", async ({ viewer }) => {
  await viewer.open("04-hierarchy");
  // the title line, directives, and an instance's name and parameters
  expect(await highlighted(viewer, 1)).toEqual([["* Two-stage buffer with subcircuits", "comment"]]);
  expect(await highlighted(viewer, 7)).toEqual([[".model", "directive"], ["level", "param"]]);
  expect(await highlighted(viewer, 9)).toEqual([["M1", "inst"], ["out", "net"], ["in", "net"], ["vss", "net"], ["vss", "net"], ["w", "param"], ["l", "param"]]);
  // records: the comment marker, keywords, references of each kind, parameters, strings and numbers
  expect(await highlighted(viewer, 11)).toEqual([["* ", "marker"], ["@RNL", "keyword"], ["{", "punct"]]);
  expect(await highlighted(viewer, 17)).toEqual([
    ["*", "marker"], ["?", "param"], ["@TEXT", "keyword"], ["(", "punct"], ['"${name}"', "string"], [",", "punct"],
    ["key", "param"], ["=", "punct"], ['"name"', "string"], [")", "punct"], ["=>", "punct"], ["@PLACE", "keyword"],
    ["(", "punct"], ["1.1", "number"], [",", "punct"], ["0.25", "number"], [");", "punct"],
  ]);
  expect(await highlighted(viewer, 20)).toEqual([
    ["*", "marker"], ["#", "host"], ["@SELECT-INSTANCE", "keyword"], ["(", "punct"], ["name", "param"], ["=", "punct"],
    ['"R*"', "string"], [")", "punct"], ["=>", "punct"], ["@SET-SYMBOL", "keyword"], ["$RES_IEC", "def"],
  ]);
  expect(await highlighted(viewer, 22)).toEqual([
    ["*", "marker"], ["%amp_block", "local"], ["=", "punct"], ["@DEFINE-SYMBOL", "keyword"], ["{", "punct"],
  ]);
  expect(await highlighted(viewer, 60)).toEqual([
    ["*", "marker"], ["&c", "net"], ["@WIRE", "keyword"], ["(", "punct"], ["mode", "param"], ["=", "punct"],
    ['"vertical-horizontal"', "string"], [")", "punct"], ["{", "punct"], ["#X2", "host"], [":", "punct"], ["out", "net"],
    ["#X3", "host"], [":", "punct"], ["top", "net"], ["}", "punct"],
  ]);

  // Spectre: its comments, its own directives, and records after //
  await viewer.open("inverter-chain");
  expect((await highlighted(viewer, 1)).map(([, cls]) => cls)).toEqual(["comment"]);
  expect(await highlighted(viewer, 3)).toEqual([["model", "directive"], ["type", "param"]]);
  expect(await highlighted(viewer, 5)).toEqual([["subckt", "directive"], ["a", "net"], ["y", "net"], ["vdd", "net"], ["vss", "net"]]);
  expect(await highlighted(viewer, 6)).toEqual([["MP", "inst"], ["y", "net"], ["a", "net"], ["vdd", "net"], ["vdd", "net"], ["w", "param"], ["l", "param"]]);
  expect(await highlighted(viewer, 8)).toEqual([["// ", "marker"], ["@RNL", "keyword"], ["{", "punct"]]);
});

test("a statement over several lines is marked on each of them, and highlighted as one", async ({ viewer }) => {
  // SPICE: X1 on lines 5 and 6, R2 on 7 and 8
  await viewer.open("continued");
  await viewer.click("X1");
  await viewer.selected("instance X1 in continued.sp");
  expect(await viewer.markedLines()).toEqual([5, 6]);
  await viewer.line(8).locator(".no").click();
  await viewer.selected("instance R2 in continued.sp");
  expect(await viewer.markedLines()).toEqual([7, 8]);
  // a later line starts with +, and its first word is no statement's; $ starts a comment
  expect(await highlighted(viewer, 6)).toEqual([["+", "punct"], ["b", "net"]]);
  expect(await highlighted(viewer, 8)).toEqual([["+", "punct"], ["r", "param"], ["$ the value", "comment"]]);
  // a comment of a record over two lines, and a string with a quote in it
  expect(await highlighted(viewer, 10)).toEqual([
    ["*", "marker"], ["#X1", "host"], ["@PLACE", "keyword"], ["(", "punct"], ["0", "number"], [",", "punct"],
    ["0", "number"], [");", "punct"], ["/* placed", "comment"],
  ]);
  expect(await highlighted(viewer, 11)).toEqual([
    ["*", "marker"], ["   by hand */", "comment"], ["#R2", "host"], ["@PLACE", "keyword"], ["(", "punct"],
    ["6", "number"], [",", "punct"], ["0", "number"], [")", "punct"], ["=>", "punct"], ["@TEXT", "keyword"],
    ["(", "punct"], ['"a ""quoted"" text"', "string"], [",", "punct"], ["key", "param"], ["=", "punct"],
    ['"note"', "string"], [")", "punct"],
  ]);

  // Spectre: a line that ends with \ goes on on the next, X1 on lines 6 and 7
  await viewer.open("continued-spectre");
  await viewer.line(7).locator(".no").click();
  await viewer.selected("instance X1 in continued.scs");
  expect(await viewer.markedLines()).toEqual([6, 7]);
  expect(await highlighted(viewer, 2)).toEqual([["simulator", "directive"], ["lang", "param"]]);
  expect(await highlighted(viewer, 6)).toEqual([["X1", "inst"], ["a", "net"], ["b", "net"]]);
  expect(await highlighted(viewer, 7)).toEqual([]);
  expect(await highlighted(viewer, 9)).toEqual([["r", "param"]]);
});

test("failed and overridden statements are greyed, and the checkbox hides their rows", async ({ page, viewer }) => {
  await viewer.open("12-diagnostics");
  const failed = await page.evaluate(() => window.viewer.doc.files[0].failed);
  expect(failed.some((f) => f[4]) && failed.some((f) => !f[4])).toBe(true);
  // each mark covers its statement, on one line here
  for (const [l0, c0, l1, c1, overridden] of failed) {
    expect(l1).toBe(l0);
    const want = (await viewer.line(l0).locator(".code").textContent()).slice(c0 - 1, c1);
    const marked = await viewer
      .line(l0)
      .locator(overridden ? ".x-overridden" : ".x-failed")
      .evaluateAll((els) => els.map((e) => e.textContent).join(""));
    expect(marked).toContain(want);
  }
  // on line 44 an override, then a failure
  await expect(viewer.line(44).locator(".x-overridden").first()).toHaveAttribute("title", /^Overridden/);
  await expect(viewer.line(44).locator(".x-failed").first()).toHaveAttribute("title", /^Failed/);

  await expect(viewer.line(44)).toHaveClass(/inactive-line/);
  expect(await viewer.line(44).locator('.code').evaluate(el => Number(getComputedStyle(el).opacity))).toBeLessThan(1);

  const box = page.locator("#show-failed");
  await expect(box).toBeChecked();
  await box.uncheck();
  for (const [line] of failed) await expect(viewer.line(line)).toHaveCount(0);
  expect(await viewer.stored("show-failed")).toBe(false);
  // the setting is kept for the next start
  await viewer.open("12-diagnostics");
  await expect(box).not.toBeChecked();
  await expect(viewer.line(44)).toHaveCount(0);
  // the label's text turns it back on
  await page.locator(".failed-toggle span", { hasText: "Failed statements" }).click();
  await expect(box).toBeChecked();
  await expect(viewer.line(44)).toHaveClass(/inactive-line/);
  expect(await viewer.stored("show-failed")).toBe(true);
});

test("clicking a problem shows its line in the middle of the text and flashes it", async ({ page, viewer }) => {
  await viewer.open("12-diagnostics");
  const problem = (text) => page.locator("#diags li.diag", { hasText: text });
  await expect(problem("PRIORITY takes constraints")).toHaveClass(/\blocated\b/);
  expect(await viewer.within(viewer.line(44))).toBe(false);

  await problem("PRIORITY takes constraints").click();
  await expect.poll(() => viewer.within(viewer.line(44))).toBe(true);
  await expect(viewer.line(44)).toHaveClass(/\bflash\b/);
  // in the middle of the text
  const [line, source] = [await viewer.line(44).boundingBox(), await page.locator("#source").boundingBox()];
  expect(Math.abs(line.y + line.height / 2 - (source.y + source.height / 2))).toBeLessThan(line.height);

  // one at the end, as far as the text scrolls
  await problem("formal tokens after the record").click();
  await expect.poll(() => viewer.within(viewer.line(72))).toBe(true);
  await expect(viewer.line(72)).toHaveClass(/\bflash\b/);

  // one near the top: the text scrolls back
  await problem("package no-such-package").click();
  await expect.poll(() => viewer.within(viewer.line(16))).toBe(true);
  await expect(viewer.line(16)).toHaveClass(/\bflash\b/);
  expect(await viewer.within(viewer.line(72))).toBe(false);
});

test("selecting on the drawing shows the file and the line of the statement", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  expect(await viewer.within(viewer.line(50))).toBe(false);
  await viewer.click("X2");
  await expect.poll(() => viewer.within(viewer.line(50))).toBe(true);
  const main = await page.locator("#source").evaluate((s) => s.scrollTop);
  expect(main).toBeGreaterThan(0);

  // a statement in the included file shows that file
  await page.locator("#tab-hierarchy").click();
  await page.locator("#index .index-row", { hasText: "rdiv" }).click();
  await page.locator("#tab-netlist").click();
  await viewer.click("R2");
  await viewer.selected("instance R2 in rdiv");
  await expect(page.locator("#source-file")).toHaveText("04-hierarchy-lib.inc");
  expect(await viewer.markedLines()).toEqual([4]);

  // each file keeps its own scroll
  await showSource(page, TOP);
  expect(await page.locator("#source").evaluate((s) => s.scrollTop)).toBe(main);
});

test("a line to show waits until the text pane is shown", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  // the side panel hidden
  await page.keyboard.press("Control+B");
  await expect(page.locator("#sidebar")).toBeHidden();
  await viewer.click("X2");
  await viewer.selected(`instance X2 in ${TOP}`);
  await page.keyboard.press("Control+B");
  await expect(page.locator("#sidebar")).toBeVisible();
  await expect.poll(() => viewer.within(viewer.line(50))).toBe(true);

  // the hierarchy view shown instead of the text
  await page.locator("#tab-hierarchy").click();
  await page.locator("#source").evaluate((s) => { s.scrollTop = 0; });
  await viewer.click("Vin");
  await viewer.selected(`instance Vin in ${TOP}`);
  await page.locator("#tab-netlist").click();
  await expect.poll(() => viewer.within(viewer.line(53))).toBe(true);
});

test("characters outside the BMP move no place of an object, a record or a failed statement", async ({ viewer }) => {
  // the Document counts an emoji or a CJK character of the extension planes as one character, and
  // JavaScript as two UTF-16 units; lines 4 and 5 have them before the places
  await viewer.open("unicode");
  const texts = (locator) => locator.evaluateAll((els) => els.map((e) => e.textContent));
  const [r1, r2] = [
    await viewer.object("instance", "R1", "unicode.sp"),
    await viewer.object("instance", `R${CJK}2`, "unicode.sp"),
  ];
  expect(await texts(viewer.line(4).locator(`[data-obj~="${r1}"]`))).toEqual(["#R1"]);
  expect(await texts(viewer.line(5).locator(`[data-obj~="${r2}"]`))).toEqual([`#"R${CJK}2"`]);
  // the statement that fails, from its first character to its last
  expect((await texts(viewer.line(5).locator(".x-failed"))).join("")).toBe("#R9 @PLACE(0, 0)");
  // each record to its last character, and a name with a character outside the BMP highlighted whole
  expect((await highlighted(viewer, 4)).at(-1)).toEqual(["}", "punct"]);
  const line5 = await highlighted(viewer, 5);
  expect(line5.at(-1)).toEqual(["}", "punct"]);
  expect(line5).toContainEqual([`#"R${CJK}2"`, "host"]);
  // a click on a place selects its object
  await viewer.line(4).locator(`[data-obj~="${r1}"]`).click();
  await viewer.selected("instance R1 in unicode.sp");
});
