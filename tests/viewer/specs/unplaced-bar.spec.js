// Objects with no position yet (apps/viewer-ui/unplaced-bar.js): while the level shown has any and draws something, a
// bar over the drawing says so, with a link that opens the Not drawn view, as its key 6 does, and one that solves the
// level shown, as Solve › Solve (F9) does; a level that draws nothing keeps its page that says why, with the links in it.
// Nothing asks when a file is opened. Only where a solver is.

import { expect, test } from "../viewer.js";
import { zh } from "../zh.js";

const DESKTOP = { platform: "desktop" };
const BAR = "Some objects have no position yet: see them in Not drawn (6), or try Solve (F9).";
const VIEW = "Not drawn (6)";
const bar = (page) => page.locator("#unplaced-bar");
const dialog = (page) => page.locator("dialog.dialog");

test("a netlist opened with objects not drawn asks nothing: the bar over the drawing says so, whenever the level is shown", async ({ page, viewer }) => {
  await viewer.open("unplaced", DESKTOP);
  await page.waitForTimeout(300);
  await expect(dialog(page)).toHaveCount(0);
  await expect(bar(page)).toBeVisible();
  await expect(bar(page)).toHaveText(BAR);
  await expect(bar(page).getByRole("button")).toHaveText([VIEW, "Solve (F9)"]);
  // the drawing keeps its size: the bar lies over it
  expect(await page.evaluate(() => getComputedStyle(document.getElementById("unplaced-bar")).position)).toBe("absolute");
  // opened again: the bar again, and still no question
  await page.evaluate(async () => { const o = await import("./open.js"); await o.openSource(o.currentSource(), false); });
  await viewer.idle();
  await expect(bar(page)).toHaveText(BAR);
  await expect(dialog(page)).toHaveCount(0);
  expect(await viewer.calls("solve")).toEqual([]);
});

test("Not drawn (6) in the bar opens the Not drawn view, as 6 does, and solves nothing", async ({ page, viewer }) => {
  await viewer.open("unplaced", DESKTOP);
  await page.evaluate(() => window.viewer.command("panel-netlist"));
  await expect(page.locator("#pane-unplaced")).toBeHidden();
  await bar(page).getByRole("button", { name: VIEW }).click();
  await expect(page.locator("#pane-unplaced")).toBeVisible();
  await expect(page.locator("#tab-unplaced")).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("#tab-unplaced")).toBeFocused();
  await expect(page.locator("#unplaced-list .unplaced-row").first()).toBeVisible();
  expect(await viewer.calls("solve")).toEqual([]);
});

test("Not drawn (6) in the bar expands a collapsed side panel first", async ({ page, viewer }) => {
  await viewer.open("unplaced", DESKTOP);
  await page.evaluate(() => window.viewer.command("panel-hide"));
  await expect(page.locator("#app")).toHaveClass(/sidebar-hidden/);
  await bar(page).getByRole("button", { name: VIEW }).click();
  await expect(page.locator("#app")).not.toHaveClass(/sidebar-hidden/);
  await expect(page.locator("#pane-unplaced")).toBeVisible();
});

test("the bar in Chinese names the view as its tab does", async ({ page, viewer }) => {
  await viewer.open("unplaced", DESKTOP);
  await page.evaluate(() => window.viewer.command("language-zh"));
  const view = zh("Not drawn (6)");
  await expect(bar(page)).toHaveText(zh("Some objects have no position yet: see them in {0}, or try {1}.", view, zh("Solve (F9)")));
  expect(view).toContain(zh("Not drawn"));
  await bar(page).getByRole("button", { name: view }).click();
  await expect(page.locator("#pane-unplaced")).toBeVisible();
});

test("a level whose objects all have a position has no bar", async ({ page, viewer }) => {
  await viewer.open("edit", DESKTOP);
  expect(await page.evaluate(() => window.viewer.doc.levels[window.viewer.level].unplaced.length)).toBe(0);
  await expect(bar(page)).toBeHidden();
});

test("Solve (F9) in the bar solves the level shown, as Solve › Solve does", async ({ page, viewer }) => {
  await viewer.open("T4-miller-conflicts", DESKTOP);
  await expect(bar(page)).toBeVisible();
  await bar(page).getByRole("button", { name: "Solve (F9)" }).click();
  await expect.poll(async () => (await viewer.calls("solve")).length).toBe(1);
  await expect(page.locator("#preview-badge")).toBeVisible();
});

test("a level that draws nothing keeps its page, and the page has the link that solves", async ({ page, viewer }) => {
  await viewer.open("va-top", DESKTOP);
  const empty = page.locator("#empty");
  await expect(empty).toBeVisible();
  await expect(empty).toContainText("Nothing on this level has a position yet, so nothing is drawn.");
  await expect(empty).toContainText("You can see them in Not drawn (6), or try Solve (F9).");
  await expect(bar(page)).toBeHidden();
  await empty.getByRole("button", { name: VIEW }).click();
  await expect(page.locator("#pane-unplaced")).toBeVisible();
  expect(await viewer.calls("solve")).toEqual([]);
  await empty.getByRole("button", { name: "Solve (F9)" }).click();
  await expect.poll(async () => (await viewer.calls("solve")).length).toBe(1);
  await expect(page.locator("#preview-badge")).toBeVisible();
});

test("without a solver, nothing offers to solve", async ({ page, viewer }) => {
  // the web page of the fake shell has no solver
  await viewer.open("unplaced");
  await expect(bar(page)).toBeHidden();
  await viewer.open("uncalled-unplaced");
  await expect(page.locator("#empty .solve-link")).toHaveCount(0);
  await expect(page.locator(".unplaced-link")).toHaveCount(0);
});

test("the Solve menu no longer has Ask to Solve on Open", async ({ page, viewer }) => {
  await viewer.open("unplaced", DESKTOP);
  const ids = await page.evaluate(async () => (await (await fetch("commands.json")).json()).menus.flatMap((m) => m.items).map((i) => i.id));
  expect(ids).not.toContain("toggle-solve-on-open");
});
