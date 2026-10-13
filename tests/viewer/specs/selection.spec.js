// Selecting an instance or a net, on the drawing or in the text, and following it to the other.

import { expect, test } from "../viewer.js";

const TOP = "04-hierarchy.sp";

/** The data-id of the groups that draw an object on the level shown, sorted. */
function groupsOf(page, o) {
  return page.evaluate((o) => {
    const v = window.viewer;
    const ids = v.doc.levels[v.level].hits.filter((h) => h.id && h.objects.includes(o)).map((h) => h.id);
    return [...new Set(ids)].sort();
  }, o);
}

test.beforeEach(async ({ viewer }) => {
  await viewer.open("04-hierarchy");
});

test("clicking a symbol selects its instance and marks its statement and the places that name it", async ({ page, viewer }) => {
  const x2 = await viewer.object("instance", "X2", TOP);
  await viewer.click("X2");
  await viewer.selected(`instance X2 in ${TOP}`);
  expect(await viewer.markedLines()).toEqual([49]);
  // its name in the statement and the references to it in the records
  const lines = await viewer.spanLines(x2);
  expect(lines.length).toBeGreaterThan(2);
  expect(await viewer.markedPlaces()).toEqual(lines);
  await expect(viewer.places(x2).first()).toHaveClass(/\bhl\b/);
  // its symbol, its box and the badge that opens it
  expect(await viewer.selectedGroups()).toEqual(await groupsOf(page, x2));
  await expect(page.locator("#canvas > svg .overlay rect")).toHaveCount(1);
  await expect(page.locator("#enter-badge")).toBeVisible();
  await expect(page.locator("#canvas")).toHaveClass(/\bsel-sub\b/);
  await expect(page.locator("#selection")).toHaveText(`X2 instance of amp in ${TOP} · Enter opens it`);
  // the statement is scrolled into view
  const [line, source] = [await viewer.line(49).boundingBox(), await page.locator("#source").boundingBox()];
  expect(line.y).toBeGreaterThanOrEqual(source.y);
  expect(line.y + line.height).toBeLessThanOrEqual(source.y + source.height);

  // an instance that opens nothing
  await viewer.click("Vin");
  await viewer.selected(`instance Vin in ${TOP}`);
  expect(await viewer.markedLines()).toEqual([52]);
  await expect(page.locator("#enter-badge")).toBeHidden();
  await expect(page.locator("#canvas")).not.toHaveClass(/\bsel-sub\b/);
  await expect(page.locator("#selection")).toHaveText(`Vin instance in ${TOP}`);
});

test("clicking a wire, a pin or a ground selects the net", async ({ page, viewer }) => {
  for (const [name, kind] of [["b", "wire"], ["a", "pin"], ["0", "box"]]) {
    const net = await viewer.object("net", name, TOP);
    await viewer.click(name, kind);
    await viewer.selected(`net ${name} in ${TOP}`);
    await expect(page.locator("#selection")).toHaveText(`${name} net in ${TOP}`);
    expect(await viewer.selectedGroups()).toEqual(await groupsOf(page, net));
    // a net has no statement of its own, only the places that name it
    expect(await viewer.markedLines()).toEqual([]);
    expect(await viewer.markedPlaces()).toEqual(await viewer.spanLines(net));
    // its pins are marked, and no instance box
    const pins = await page.evaluate((o) => window.viewer.doc.levels[0].hits.filter((h) => h.kind === "pin" && h.objects.includes(o)).length, net);
    await expect(page.locator("#canvas > svg .overlay circle")).toHaveCount(pins);
    await expect(page.locator("#canvas > svg .overlay rect")).toHaveCount(0);
    await expect(page.locator("#enter-badge")).toBeHidden();
  }
});

test("clicking a statement or a node in the text selects it on the drawing, on its level", async ({ page, viewer }) => {
  // M1's statement, in the subcircuit amp
  await viewer.line(8).locator(".no").click();
  await viewer.selected("instance M1 in amp");
  expect(await viewer.state()).toMatchObject({ title: "amp" });
  await expect(page.locator("#level-name")).toHaveText("amp");
  const m1 = await viewer.object("instance", "M1", "amp");
  expect(await viewer.selectedGroups()).toEqual(await groupsOf(page, m1));
  expect(await viewer.markedLines()).toEqual([8]);

  // the node b of X1's statement, on the top level
  const b = await viewer.object("net", "b", TOP);
  await viewer.line(48).locator(`[data-obj~="${b}"]`).click();
  await viewer.selected(`net b in ${TOP}`);
  await expect(page.locator("#level-name")).toHaveText(TOP);
  expect(await viewer.selectedGroups()).toEqual(await groupsOf(page, b));

  // a reference in a record, such as #X3 in #X3:top, which names its instance first
  const x3 = await viewer.object("instance", "X3", TOP);
  const ref = (await viewer.spanLines(x3)).find((l) => l !== 50);
  await viewer.line(ref).locator(`[data-obj~="${x3}"]`).first().click();
  await viewer.selected(`instance X3 in ${TOP}`);
  expect(await viewer.markedLines()).toEqual([50]);

  // a node in the included file, in the subcircuit rdiv
  await page.locator("#file-tabs button", { hasText: "04-hierarchy-lib.inc" }).click();
  const mid = await viewer.object("net", "mid", "rdiv");
  await viewer.line(3).locator(`[data-obj~="${mid}"]`).click();
  await viewer.selected("net mid in rdiv");
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  expect(await viewer.selectedGroups()).toEqual(await groupsOf(page, mid));
});

test("Escape and a click on an empty place clear the selection", async ({ page, viewer }) => {
  const cleared = async () => {
    await viewer.selected(null);
    await expect(page.locator("#source .hl, #source .hl-line")).toHaveCount(0);
    await expect(page.locator("#canvas > svg g.sel")).toHaveCount(0);
    await expect(page.locator("#canvas > svg .overlay > *")).toHaveCount(0);
    await expect(page.locator("#selection")).toHaveText("");
    await expect(page.locator("#enter-badge")).toBeHidden();
  };
  await viewer.click("X2");
  await viewer.selected(`instance X2 in ${TOP}`);
  await page.keyboard.press("Escape");
  await cleared();

  await viewer.click("b", "wire");
  await viewer.selected(`net b in ${TOP}`);
  const empty = await viewer.empty();
  await page.mouse.click(empty.x, empty.y);
  await cleared();

  // in the text: a line that is no statement and names nothing
  await viewer.line(49).locator(".no").click();
  await viewer.selected(`instance X2 in ${TOP}`);
  await viewer.line(1).click();
  await cleared();
});

test("dragging moves the view and selects nothing", async ({ page, viewer }) => {
  await viewer.click("X2");
  await viewer.selected(`instance X2 in ${TOP}`);
  const before = (await viewer.viewBox()).split(" ").map(Number);
  const from = await viewer.where("X1");
  await viewer.drag(from, { x: from.x + 120, y: from.y + 40 });
  const after = (await viewer.viewBox()).split(" ").map(Number);
  const k = (await page.locator("#canvas").evaluate((c) => c.clientWidth)) / before[2];
  expect(after[0]).toBeCloseTo(before[0] - 120 / k, 6);
  expect(after[1]).toBeCloseTo(before[1] - 40 / k, 6);
  expect(after.slice(2)).toEqual(before.slice(2));
  await viewer.selected(`instance X2 in ${TOP}`);
});
