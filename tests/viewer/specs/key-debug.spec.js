// ?debug=keys: the corner list of keys and focus changes, with what became of each key; and the panel
// keys after the ways a user may refresh: F5 (the page reads the file again), Ctrl+R (Show Usages)
// and a reload of the page itself.
import { test, expect } from "../viewer.js";

const panel = (page) => page.locator("#key-debug");

test("?debug=keys lists each key, what it reached and why no command ran; without it nothing shows", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await expect(panel(page)).toHaveCount(0);
  expect(await page.evaluate(() => typeof window.rnlKeyNote)).toBe("undefined");

  await viewer.open("04-hierarchy", { debug: "keys" });
  await page.keyboard.press("3");
  await expect(page.locator("#pane-hierarchy")).toBeVisible();
  await expect(panel(page)).toContainText('key="3" code=Digit3');
  await expect(panel(page)).toContainText("ran panel-hierarchy");
  await page.keyboard.press("q");
  await expect(panel(page)).toContainText("no command for this key");
  await page.keyboard.press("2");
  await page.keyboard.press("Control+f");
  await expect(page.locator("#netlist-query")).toBeFocused();
  await page.keyboard.press("4");
  await expect(panel(page)).toContainText("typing in a text box");
  await expect(panel(page)).toContainText("focus input#netlist-query");
  await expect(panel(page)).toContainText("target=input#netlist-query");
});

for (const platform of ["web", "desktop"]) {
  test(`${platform}: panel keys work after F5, Ctrl+R and a reload of the page`, async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform });
    await viewer.build(() => page.keyboard.press("F5"));
    await page.keyboard.press("3");
    await expect(page.locator("#pane-hierarchy")).toBeVisible();
    await page.keyboard.press("Control+r");
    await page.keyboard.press("1");
    await expect(page.locator("#pane-files")).toBeVisible();
    await page.reload();
    await viewer.started();
    await viewer.idle();
    await page.keyboard.press("4");
    await expect(page.locator("#pane-symbol")).toBeVisible();
  });
}
