// The prefix of the names the page keeps in the browser's storage (apps/viewer-ui/storage-prefix.js, which the web build writes
// from RNL_STORAGE_PREFIX): a build with one keeps and reads its settings under it, and leaves another build's alone. The
// page is given the prefixed file in the place of its own.

import { expect, test } from "../viewer.js";

const PREFIX = "frozen-";

test("with a prefix, the settings are kept and read under it, and another build's are neither read nor touched", async ({ page, viewer }) => {
  await page.route("**/storage-prefix.js", (route) =>
    route.fulfill({ contentType: "text/javascript", body: `export const STORAGE_PREFIX = ${JSON.stringify(PREFIX)};` }));
  // the other build kept the grid on; this one has kept the usages on
  await page.addInitScript((prefix) => {
    if (localStorage.getItem("seeded") !== null) return;
    localStorage.setItem("seeded", "1");
    localStorage.setItem("grid-shown", "true");
    localStorage.setItem(`${prefix}usages-shown`, "true");
  }, PREFIX);
  await viewer.open("04-hierarchy");
  expect(await page.evaluate(() => window.viewer.grid.shown)).toBe(false);
  expect(await page.evaluate(() => window.viewer.checked("toggle-usages"))).toBe(true);
  // a change is kept under the prefix
  await page.evaluate(() => window.viewer.command("toggle-currents"));
  expect(await page.evaluate(() => localStorage.getItem("frozen-currents-shown"))).toBe("true");
  expect(await page.evaluate(() => localStorage.getItem("currents-shown"))).toBeNull();
  // and nothing the page wrote is without it, but what the test put there
  const keys = await page.evaluate(() => Object.keys(localStorage).filter((k) => !k.startsWith("frozen-")).sort());
  expect(keys).toEqual(["grid-shown", "seeded"]);
  // read again after a reload
  await viewer.open("04-hierarchy");
  expect(await page.evaluate(() => window.viewer.checked("toggle-currents"))).toBe(true);
});

test("with no prefix, the names are the plain ones", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await page.evaluate(() => window.viewer.command("toggle-currents"));
  expect(await viewer.stored("currents-shown")).toBe(true);
  expect(await page.evaluate(() => Object.keys(localStorage).some((k) => k.startsWith("frozen-")))).toBe(false);
});
