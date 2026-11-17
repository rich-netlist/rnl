// Hide nested usages, a checkbox of the Usage panel (usages.js): a usage made inside another one's definition with the same
// members is not drawn while that one's frame is shown, only the outermost; one made inside with other members is drawn.
// And Group frames, a checkbox of the Solve panel: the frames of the groups, which the Usage panel no longer lists.
// The fixture nested-usages-04: %pair makes a %match of its two members and a %one of the first; %quad makes a %pair of
// its two; %dp is a %pair, %q a %quad, and %m a %match of its own, with the members of %dp, made inside nothing.

import { expect, test } from "../viewer.js";

test.use({ keptSettings: { "usages-shown": true } });

/** The frames drawn, each as its definition and the definition of the usage it is made in, `-` for none, sorted. */
const frames = (page) => page.evaluate(() => {
  const usages = window.viewer.doc.levels[0].usages;
  const by = new Map(usages.map((u) => [u.object, u]));
  return [...document.querySelectorAll("#canvas .usage-frame")].map((f) => by.get(Number(f.dataset.usage)))
    .map((u) => `${u.definition} in ${u.parent == null ? "-" : by.get(u.parent).definition}`).sort();
});
const ALL = ["%match in %pair", "%match in %pair", "%match in -", "%one in %pair", "%one in %pair", "%pair in %quad", "%pair in -", "%quad in -"];
const kept = (page, key) => page.evaluate(async (key) => (await import("./util.js")).load(key, null), key);

test.beforeEach(async ({ page, viewer }) => {
  await viewer.open("nested-usages-04");
  await page.locator("#tab-usages").click();
});

test("Hide nested usages draws only the outermost of the usages with the same members, and the others as before", async ({ page }) => {
  const box = page.locator("#usage-hide-nested");
  await expect(box).not.toBeChecked();
  await expect(page.locator(".usage-nested-toggle")).toHaveText("Hide nested usages");
  await expect.poll(() => frames(page)).toEqual(ALL);
  await box.check();
  // the %match of each %pair and the %pair of %quad go; the %one of each %pair, of one member only, and %m, made inside
  // nothing, stay
  await expect.poll(() => frames(page)).toEqual(["%match in -", "%one in %pair", "%one in %pair", "%pair in -", "%quad in -"]);
  expect(await kept(page, "usages-nested-hidden")).toBe(true);
  // their rows stay, each saying inside which usage it is
  await page.locator("#usage-expand-all").click();
  const notes = page.locator("#usage-list .usage-row.nested .usage-nested-note");
  await expect(notes).toHaveCount(3);
  await expect(page.locator('#usage-list [data-type="%match"] .usage-row.nested .usage-nested-note')).toHaveText(["inside %pair: %dp", "inside %quad: %q"]);
  await expect(page.locator('#usage-list [data-type="%one"] .usage-row.nested')).toHaveCount(0);
  await expect(page.locator('#usage-list [data-type="%match"] .usage-row', { hasText: "%m" }).first()).not.toHaveClass(/nested/);
  // with %q not drawn, its %pair is the outermost shown: it is drawn, and the %match inside it is not
  await page.locator('#usage-list [data-type="%quad"] .usage-row', { hasText: "%q" }).locator('input[type="checkbox"]').uncheck();
  await expect.poll(() => frames(page)).toEqual(["%match in -", "%one in %pair", "%one in %pair", "%pair in %quad", "%pair in -"]);
  await expect(page.locator('#usage-list [data-type="%match"] .usage-row.nested .usage-nested-note')).toHaveText(["inside %pair: %dp", /^inside %pair: /]);
  // a nested usage that is selected is drawn
  await page.locator('#usage-list [data-type="%match"] .usage-row.nested').first().locator(".usage-name").click();
  await expect(page.locator("#canvas .usage-frame.selected")).toHaveCount(1);
  // off, every frame is drawn again
  await page.locator("#canvas").click({ position: { x: 2, y: 2 } });
  await page.locator('#usage-list [data-type="%quad"] .usage-row', { hasText: "%q" }).locator('input[type="checkbox"]').check();
  await box.uncheck();
  await expect.poll(() => frames(page)).toEqual(ALL);
  await expect(page.locator("#usage-list .usage-row.nested")).toHaveCount(0);
  expect(await kept(page, "usages-nested-hidden")).toBe(false);
});

test.describe("kept on", () => {
  test.use({ keptSettings: { "usages-shown": true, "usages-nested-hidden": true } });
  test("Hide nested usages is on at the start when it was kept on", async ({ page }) => {
    await expect(page.locator("#usage-hide-nested")).toBeChecked();
    await expect.poll(() => frames(page)).toEqual(["%match in -", "%one in %pair", "%one in %pair", "%pair in -", "%quad in -"]);
  });
});

test.describe("the groups", () => {
  test("the Usage panel does not list the groups; Group frames in the Solve panel draws their frames, apart from Show Usages", async ({ page, viewer }) => {
    await viewer.open("ota-groups");
    await page.locator("#tab-usages").click();
    await expect(page.locator("#usage-list .usage-group").first()).toBeAttached();
    await expect(page.locator('#usage-list [data-type="@GROUP"]')).toHaveCount(0);
    const groupFrames = page.locator("#canvas .usage-frame .usage-title text", { hasText: "@GROUP:" });
    await expect(groupFrames).toHaveCount(0);
    await page.locator("#tab-solve").click();
    const box = page.locator("#solve-group-frames");
    await expect(box).not.toBeChecked();
    await expect(page.locator(".solve-overlays label", { has: box })).toHaveText("Group frames");
    await box.check();
    await expect(groupFrames).toHaveCount(4);
    expect(await kept(page, "groups-shown")).toBe(true);
    // the usages' frames stay as View > Show Usages has them: off, the groups' frames alone are drawn
    await page.evaluate(() => window.viewer.command("toggle-usages"));
    await expect(groupFrames).toHaveCount(4);
    await expect(page.locator("#canvas .usage-frame")).toHaveCount(4);
    await box.uncheck();
    await expect(page.locator("#canvas .usage-frame")).toHaveCount(0);
    expect(await kept(page, "groups-shown")).toBe(false);
  });
});
