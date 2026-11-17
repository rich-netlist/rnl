// Hide duplicate usages, a checkbox of the Usage panel (usages.js): of the usages that frame the same objects (instances,
// pins and points; nets left out) one frame is drawn, that of the last one written, or of the outermost usage of the set
// that it is made inside; usages of other objects are drawn as before. And Group frames, a checkbox of the Solve panel: the frames of the groups, which the Usage panel does
// not list.
// The fixture duplicate-usages-04: %pair makes a %match of its two devices and a %one of the first; %quad makes a %pair of
// its two. %dp is a %pair and %m, written after it, a %match of the same devices (a duplicate written later); %m2, a
// %match, is written before %dp2, a %pair of the same devices (the later one makes a %match of them in turn); %q is a
// %quad, which makes a %pair, which makes a %match, all of the same devices (nested in definitions only); %pa and %pb frame
// R7 and a pin of it each, the same device and not the same objects.

import { expect, test } from "../viewer.js";

test.use({ keptSettings: { "usages-shown": true } });

/** The name of a usage: its id, or for one made inside another, its definition in that one's name. */
const NAME = `const by = new Map(window.viewer.doc.levels[0].usages.map((u) => [u.object, u]));
  const name = (u) => (u.parent == null ? u.id : \`\${u.definition} in \${name(by.get(u.parent))}\`);`;
/** The usages whose frames are drawn, by name, sorted. */
const frames = (page) => page.evaluate(`(() => { ${NAME}
  return [...document.querySelectorAll("#canvas .usage-frame")].map((f) => name(by.get(Number(f.dataset.usage)))).sort(); })()`);
/** The rows that say which usage is drawn in their place: their usage's name, and what they say. */
const notes = (page) => page.evaluate(`(() => { ${NAME}
  return Object.fromEntries([...document.querySelectorAll("#usage-list .usage-row.duplicate")].map((row) =>
    [name(by.get(Number(row.dataset.object))), row.querySelector(".usage-duplicate-note")?.textContent])); })()`);
const ALL = ["%dp", "%dp2", "%m", "%m2", "%match in %dp", "%match in %dp2", "%match in %pair in %q", "%one in %dp", "%one in %dp2",
  "%one in %pair in %q", "%pa", "%pair in %q", "%pb", "%q"];
// of other objects than the rest: the %one of each %pair, of one device, and %pa and %pb, which frame R7 and another pin each
const ONES = ["%one in %dp", "%one in %dp2", "%one in %pair in %q", "%pa", "%pb"];
const kept = (page, key) => page.evaluate(async (key) => (await import("./util.js")).load(key, null), key);
/** The check of the row of the usage `id` among those of `type` (its name alone: another row's note may name it). */
const check = (page, type, id) => page.locator(`#usage-list [data-type="${type}"] .usage-row`)
  .filter({ has: page.locator(".usage-name", { hasText: new RegExp(`^${id}$`) }) }).locator('input[type="checkbox"]');

test.beforeEach(async ({ page, viewer }) => {
  await viewer.open("duplicate-usages-04");
  await page.locator("#tab-usages").click();
});

test("Hide duplicate usages draws one frame of the usages of the same objects: the last written, or the outermost it is made inside", async ({ page }) => {
  const box = page.locator("#usage-hide-duplicates");
  await expect(box).not.toBeChecked();
  await expect(page.locator(".usage-duplicates-toggle")).toHaveText("Hide duplicate usages");
  await expect.poll(() => frames(page)).toEqual(ALL);
  await box.check();
  expect(await kept(page, "usages-duplicates-hidden")).toBe(true);
  // a duplicate written later: %m, after %dp and the %match %dp makes, is drawn, they are not;
  // a duplicate written before: %dp2 makes the last %match of R5 and R6, and is the outermost it is made inside: %dp2;
  // nested in definitions only: the outermost, %q;
  // the %one of each, of one device, and %pa and %pb, of R7 and a pin each, are of other objects than all the rest and are drawn
  await expect.poll(() => frames(page)).toEqual(["%dp2", "%m", "%q", ...ONES].sort());
  // the rows stay, each saying which usage is drawn in its place
  await page.locator("#usage-expand-all").click();
  expect(await notes(page)).toEqual({
    "%dp": "duplicate of %match: %m",
    "%match in %dp": "duplicate of %match: %m",
    "%m2": "duplicate of %pair: %dp2",
    "%match in %dp2": "duplicate of %pair: %dp2",
    "%pair in %q": "duplicate of %quad: %q",
    "%match in %pair in %q": "duplicate of %quad: %q",
  });
  // one turned off by its check is not in the set: the next is drawn in its place, the last of those left, or the
  // outermost it is made inside
  await check(page, "%match", "%m").uncheck();
  await expect.poll(() => frames(page)).toEqual(["%dp", "%dp2", "%q", ...ONES].sort());
  await check(page, "%quad", "%q").uncheck();
  await expect.poll(() => frames(page)).toEqual(["%dp", "%dp2", "%pair in %q", ...ONES].sort());
  // a usage not drawn is drawn while it is selected
  await page.locator("#usage-list .usage-row.duplicate").first().locator(".usage-name").click();
  await expect(page.locator("#canvas .usage-frame.selected")).toHaveCount(1);
  // off, every frame shown by its check is drawn again
  await page.locator("#canvas").click({ position: { x: 2, y: 2 } });
  await check(page, "%match", "%m").check();
  await check(page, "%quad", "%q").check();
  await box.uncheck();
  await expect.poll(() => frames(page)).toEqual(ALL);
  await expect(page.locator("#usage-list .usage-row.duplicate")).toHaveCount(0);
  expect(await kept(page, "usages-duplicates-hidden")).toBe(false);
});

test.describe("kept on", () => {
  test.use({ keptSettings: { "usages-shown": true, "usages-duplicates-hidden": true } });
  test("Hide duplicate usages is on at the start when it was kept on", async ({ page }) => {
    await expect(page.locator("#usage-hide-duplicates")).toBeChecked();
    await expect.poll(() => frames(page)).toEqual(["%dp2", "%m", "%q", ...ONES].sort());
  });
});

test.describe("kept on as Hide nested usages", () => {
  test.use({ keptSettings: { "usages-shown": true, "usages-nested-hidden": true } });
  test("the setting of the Hide nested usages it replaces is read", async ({ page }) => {
    await expect(page.locator("#usage-hide-duplicates")).toBeChecked();
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
