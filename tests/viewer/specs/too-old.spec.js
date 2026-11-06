// A browser too old for the page is told so (the ES5 script of index.html), instead of a start-up screen that never goes:
// the scripts use features that came in 2021 to 2023. The tests take features away from the page, as an old browser lacks them.

import { expect, test } from "../viewer.js";

const address = () => `${process.env.RNL_VIEWER_URL}/index.html?shell=fake`;

test("a current browser is told nothing", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await expect(page.locator("#too-old")).toHaveCount(0);
  expect(await page.evaluate(() => window.rnlTooOld)).toBeUndefined();
});

for (const [feature, remove] of [
  ["Object.hasOwn", () => { delete Object.hasOwn; }],
  ["Array.prototype.at", () => { delete Array.prototype.at; }],
  ["WebAssembly", () => { Object.defineProperty(window, "WebAssembly", { value: undefined, configurable: true }); }],
  ["Worker", () => { Object.defineProperty(window, "Worker", { value: undefined, configurable: true }); }],
]) {
  test(`a browser without ${feature} is told it is too old, and what to use`, async ({ page }) => {
    await page.addInitScript(remove);
    await page.goto(address());
    const note = page.locator("#too-old");
    await expect(note).toBeVisible();
    await expect(note).toContainText("This browser is too old for RNL Viewer");
    await expect(note).toContainText(feature);
    await expect(note).toContainText("Firefox 115 ESR");
    await expect(note).toHaveAttribute("role", "alert");
    // it covers the start-up screen, which would otherwise go on saying the viewer is loading
    const covers = await page.evaluate(() => {
      const r = document.querySelector("#too-old").getBoundingClientRect();
      return r.width >= innerWidth - 1 && r.height >= innerHeight - 1;
    });
    expect(covers).toBe(true);
  });
}

test("a browser whose Workers do not read the module option is too old: Firefox before 114", async ({ page }) => {
  await page.addInitScript(() => {
    const Real = window.Worker;
    // a Worker that ignores its options, as the browsers before module Workers did
    window.Worker = function (url) { return new Real(url); };
    window.Worker.prototype = Real.prototype;
  });
  await page.goto(address());
  await expect(page.locator("#too-old")).toContainText("module Workers");
});
