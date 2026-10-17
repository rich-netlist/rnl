// A file of more lines than one call takes as its arguments in Chrome: it opens whole. It runs in
// the project long, in Chromium only and after the other projects (playwright.config.js). Playwright's
// trace takes a snapshot of the DOM at each step, which with 200 000 lines takes seconds, so this
// file runs without it.

import { LONG } from "../fixtures.js";
import { expect, test } from "../viewer.js";

test.use({ trace: "off" });

test("a file of more lines than a call takes as its arguments opens whole", async ({ page, viewer }) => {
  test.slow();
  await viewer.open("long");
  const last = LONG + 4;
  await expect(page.locator("#source .file:not([hidden]) .ln")).toHaveCount(last);
  await expect(viewer.line(1).locator(".code")).toHaveText("* a long file");
  await expect(viewer.line(last).locator(".code")).toHaveText(".end");
  // R1's statement, near the end, is shown when R1 is selected on the drawing
  await viewer.click("R1");
  await viewer.selected("instance R1 in long.sp");
  await expect.poll(() => viewer.within(viewer.line(LONG + 2))).toBe(true);
});
