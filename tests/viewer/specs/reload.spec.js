import { showSource } from "../viewer.js";
// Reading the document again, by F5: the level, the view, the selection, the views left on
// other levels, the folded branches of the tree, the file shown and its scroll stay, also when the
// file has changed; a reading that fails is shown, and the next one shows the document again.

import { expect, test } from "../viewer.js";

const TOP = "04-hierarchy.sp";

test.beforeEach(async ({ viewer }) => {
  await viewer.open("04-hierarchy");
});

test("F5 reads the document again and keep the level, the view and the selection", async ({ page, viewer }) => {
  await viewer.click("X2");
  await page.keyboard.press("Enter");
  await expect(page.locator("#level-name")).toHaveText("amp");
  await page.keyboard.press("Control+-");
  await viewer.click("M1");
  await viewer.selected("instance M1 in amp");
  const view = await viewer.viewBox();

  for (const key of ["F5", "Control+F5", "Shift+F5"]) {
    await viewer.build(() => page.keyboard.press(key));
    await expect(page.locator("#level-name")).toHaveText("amp");
    expect(await viewer.viewBox()).toBe(view);
    await viewer.selected("instance M1 in amp");
    expect(await viewer.markedLines()).toEqual([9]);
    await expect(page.locator("#canvas > svg .overlay rect:not(.label)")).toHaveCount(1);
  }
  expect(await viewer.calls("build")).toEqual(Array(4).fill(["04-hierarchy"]));
  // the page was not loaded again
  expect(await viewer.calls("initial")).toEqual([[]]);

  // the way down is kept too: up goes back to X2
  await page.keyboard.press("Backspace");
  await expect(page.locator("#level-name")).toHaveText(TOP);
  await viewer.selected(`instance X2 in ${TOP}`);
});

test("reading again keeps the views left on other levels, and the tree's folded branches", async ({ page, viewer }) => {
  await page.keyboard.press("Control+-"); // a view of the top level that fitting would not give
  const top = await viewer.viewBox();
  await viewer.click("X3");
  await page.keyboard.press("Enter");
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  await viewer.build(() => page.keyboard.press("F5"));
  await page.keyboard.press("Backspace");
  await expect(page.locator("#level-name")).toHaveText(TOP);
  expect(await viewer.viewBox()).toBe(top);

  // the tree folded at the top level stays folded
  await page.locator("#tab-hierarchy").click();
  const rows = page.locator("#tree .tree-row");
  await page.locator('#tree .tree-row[data-path="0"] .twisty').click();
  await expect(rows).toHaveCount(1);
  await viewer.build(() => page.keyboard.press("F5"));
  await expect(rows).toHaveCount(1);
  await expect(page.locator('#tree .tree-row[data-path="0"] .twisty')).toHaveAttribute("aria-expanded", "false");
});

test("a file that changed keeps the selection on its statement, which moved", async ({ page, viewer }) => {
  await viewer.click("X2");
  await viewer.selected(`instance X2 in ${TOP}`);
  expect(await viewer.markedLines()).toEqual([50]);
  await page.keyboard.press("Control+=");
  const view = await viewer.viewBox();
  const scroll = await page.locator("#source").evaluate((s) => s.scrollTop);
  expect(scroll).toBeGreaterThan(0);

  await page.evaluate(() => { window.fakeShell.next = "04-hierarchy-changed"; });
  await viewer.build(() => page.keyboard.press("F5"));
  await expect(viewer.line(54)).toHaveText(/Vin a 0 1\.2/);
  await viewer.selected(`instance X2 in ${TOP}`);
  expect(await viewer.markedLines()).toEqual([51]);
  expect(await viewer.viewBox()).toBe(view);
  expect(await page.locator("#source").evaluate((s) => s.scrollTop)).toBe(scroll);

  // the file shown stays shown
  await showSource(page, "04-hierarchy-lib.inc");
  await viewer.build(() => page.keyboard.press("F5"));
  await expect(page.locator("#source-file")).toHaveText("04-hierarchy-lib.inc");
  await expect(viewer.line(2)).toHaveText(/\.subckt rdiv top mid bot/);
});

test("a reading that fails is shown, and the next one shows the document from its top", async ({ page, viewer }) => {
  await viewer.click("X3");
  await page.keyboard.press("Enter");
  await expect(page.locator("#level-name")).toHaveText("rdiv");

  await page.evaluate(() => { window.fakeShell.fail = "04-hierarchy.sp: the file is locked"; });
  await viewer.build(() => page.keyboard.press("F5"));
  await expect(page.locator("#empty")).toHaveText("Could not open 04-hierarchy");
  await expect(page.locator("#diags li.diag.error")).toHaveText(/the file is locked/);
  await expect(page.locator("#tree .tree-row")).toHaveCount(0);
  await expect(page.locator("#up")).toBeHidden();
  await viewer.selected(null);

  await viewer.build(() => page.keyboard.press("F5"));
  await expect(page.locator("#level-name")).toHaveText(TOP);
  await expect(page.locator("#canvas > svg")).toHaveCount(1);
  // the fixture has no problems: its comments are ASCII
  await expect(page.locator("#counts")).toHaveText("none");
  await expect(page.locator("#diags li.diag")).toHaveCount(0);
  await expect(page.locator("#diags li.nothing")).toHaveText("No errors, warnings or hints.");
  await viewer.selected(null);
});
