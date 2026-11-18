// Asking to solve a netlist opened with components not drawn (apps/viewer-ui/solve-on-open.js): a
// dialog says how many, as the Not drawn tab counts them, and offers Solve, which solves as Solve does,
// or Not now, after which the same file asks no more while the page is open. Only when a file is
// opened, not read again or rebuilt; only where a solver is; Solve › Ask to Solve on Open turns it
// off. The other tests start with it off (playwright.config.js); these start from an empty storage.

import { expect, test } from "../viewer.js";

test.use({ storageState: { cookies: [], origins: [] } });

const DESKTOP = { platform: "desktop" };
const dialog = (page) => page.locator("dialog.dialog");

/** Open `fixture` and wait until the page has seen it, without waiting for idle (a dialog is up). */
async function open(page, fixture, query = DESKTOP) {
  const params = new URLSearchParams({ shell: "fake", open: fixture, ...query });
  await page.goto(`${process.env.RNL_VIEWER_URL}/index.html?${params}`);
  await page.waitForFunction(() => window.viewer?.doc && !document.body.classList.contains("busy"));
}

/** Open the file shown again, as File > Open of the same file would. */
const openAgain = (page) => page.evaluate(async () => {
  const o = await import("./open.js");
  await o.openSource(o.currentSource(), false);
});

test("a netlist opened with components not drawn asks to solve them, and Solve solves", async ({ page, viewer }) => {
  await open(page, "T4-miller-conflicts");
  await expect(dialog(page)).toBeVisible();
  await expect(dialog(page)).toContainText("Solve the Netlist?");
  // as many as the Not drawn tab counts
  const counted = (await page.locator("#unplaced-tab-count").textContent()).match(/\d+/)[0];
  await expect(dialog(page)).toContainText(`${counted} components have no position yet, so they are not drawn.`);
  await expect(dialog(page)).toContainText("the files are not changed");
  await expect(dialog(page).getByRole("button", { name: "Solve" })).toHaveClass(/default/);
  await dialog(page).getByRole("button", { name: "Solve" }).click();
  await expect(dialog(page)).toHaveCount(0);
  await expect.poll(async () => (await viewer.calls("solve")).length).toBe(1);
  await expect(page.locator("#preview-badge")).toBeVisible();
});

test("Not now solves nothing, and the same file asks no more: opened again, read again, or rebuilt", async ({ page, viewer }) => {
  await open(page, "unplaced");
  await dialog(page).getByRole("button", { name: "Not now" }).click();
  await expect(dialog(page)).toHaveCount(0);
  await openAgain(page);
  await page.evaluate(() => window.viewer.command("reload"));
  await page.waitForFunction(() => !document.body.classList.contains("busy"));
  await expect(dialog(page)).toHaveCount(0);
  expect(await viewer.calls("solve")).toEqual([]);
});

test("Escape is Not now too", async ({ page, viewer }) => {
  await open(page, "unplaced");
  await expect(dialog(page)).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog(page)).toHaveCount(0);
  await openAgain(page);
  await expect(dialog(page)).toHaveCount(0);
  expect(await viewer.calls("solve")).toEqual([]);
});

test("a netlist with every component placed asks nothing, nor a page without a solver", async ({ page }) => {
  await open(page, "edit");
  await page.waitForTimeout(300);
  await expect(dialog(page)).toHaveCount(0);
  // the web page of the fake shell has no solver
  await open(page, "unplaced", {});
  await page.waitForTimeout(300);
  await expect(dialog(page)).toHaveCount(0);
});

test("Solve > Ask to Solve on Open, on to begin with, turns the question off and is kept", async ({ page, viewer }) => {
  await open(page, "edit");
  expect(await page.evaluate(() => window.viewer.checked("toggle-solve-on-open"))).toBe(true);
  await page.evaluate(() => window.viewer.command("toggle-solve-on-open"));
  expect(await page.evaluate(() => window.viewer.checked("toggle-solve-on-open"))).toBe(false);
  expect(await viewer.stored("solve-on-open")).toBe(false);
  await open(page, "unplaced");
  await page.waitForTimeout(300);
  await expect(dialog(page)).toHaveCount(0);
  expect(await page.evaluate(() => window.viewer.checked("toggle-solve-on-open"))).toBe(false);
});
