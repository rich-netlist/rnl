// Hide duplicate usages, a checkbox of the Usage panel (usages.js): of the usages that frame the same leaves (instances,
// and usages with a symbol of their own, as ports; a pin for what it is the pin of; nets and points left out) one frame is
// drawn, that of the last one written, or of the outermost usage of the set that it is made inside; usages of other leaves
// are drawn as before. And Group frames, a checkbox of the Solve panel: the frames of the groups, which the Usage panel does
// not list.
// The fixture duplicate-usages-04: %pair makes a %match of its two devices and a %one of the first; %quad makes a %pair of
// its two. %dp is a %pair and %m, written after it, a %match of the same devices (a duplicate written later); %m2, a
// %match, is written before %dp2, a %pair of the same devices (the later one makes a %match of them in turn); %q is a
// %quad, which makes a %pair, which makes a %match, all of the same devices (nested in definitions only); %pa and %pb frame
// R7 and a pin of it each, the same leaf; %pp and %pr frame R8 and the point %px, %pq R8 and %py, the same leaf too.

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
  "%one in %pair in %q", "%pa", "%pair in %q", "%pb", "%pp", "%pq", "%pr", "%q"];
// of other leaves than the rest: the %one of each %pair, of one device; %pb, the later of %pa and %pb, which frame R7 and a
// pin of it each; %pr, the last of the three of R8, whatever their points
const ONES = ["%one in %dp", "%one in %dp2", "%one in %pair in %q", "%pb", "%pr"];
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
  // the %one of each, of one device, is of other leaves than all the rest and is drawn; of %pa and %pb, of R7 and a pin of
  // it each, the later, %pb; of %pp, %pq and %pr, of R8 at a point each, the last, %pr
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
    "%pa": "duplicate of %onpin: %pb",
    "%pp": "duplicate of %axis: %pr",
    "%pq": "duplicate of %axis: %pr",
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

test.describe("the same pair in other shapes", () => {
  // duplicate-usages-mirror-04: %cm1 has R1 and R2 in a role each, %m12 in one role, %x12 as left and right about the point
  // %ax, and %m34 and %x34 the same of R3 and R4, as a charge pump's cell writes its mirror loads and its input pair;
  // %x345 is about R5, an instance, and %cp has R1 and R2 in a role beside R5
  test("usages of the same instances are one set whatever their roles, and a point beside them changes nothing", async ({ page, viewer }) => {
    await viewer.open("duplicate-usages-mirror-04");
    await page.locator("#tab-usages").click();
    await expect.poll(() => frames(page)).toEqual(["%cm1", "%cp", "%m12", "%m34", "%x12", "%x34", "%x345"]);
    await page.locator("#usage-hide-duplicates").check();
    // of R1 and R2, the last written, %x12; of R3 and R4, %x34; %x345, about an instance, and %cp, with R5, are drawn
    await expect.poll(() => frames(page)).toEqual(["%cp", "%x12", "%x34", "%x345"]);
    await page.locator("#usage-expand-all").click();
    expect(await notes(page)).toEqual({
      "%cm1": "duplicate of %mirrored: %x12",
      "%m12": "duplicate of %mirrored: %x12",
      "%m34": "duplicate of %mirrored: %x34",
    });
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
